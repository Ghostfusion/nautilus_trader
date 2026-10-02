// -------------------------------------------------------------------------------------------------
//  Copyright (C) 2015-2026 Nautech Systems Pty Ltd. All rights reserved.
//  https://nautechsystems.io
//
//  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
//  You may not use this file except in compliance with the License.
//  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
// -------------------------------------------------------------------------------------------------

//! Conversion of the gateway's order book records to the domain model.
//!
//! A quote tick needs a bid and an ask, and the venue's quote snapshot carries neither, so the
//! quote path is built on the book instead. The book arrives whole on every push rather than as
//! the changes since the last one, which is what lets each push be treated as a snapshot that
//! replaces the domain book rather than as a delta against it. The push is not depth-limited: the
//! venue sends its full ladder whatever depth was requested, so the requested depth is applied
//! here rather than relied on at the gateway.
//!
//! Three rules here are the venue's and not this adapter's choice. The board type is preserved,
//! because the odd-lot board carries its own prices and merging it into the ordinary book would
//! report a best bid that cannot be traded at that size. A book update is stamped with the receive
//! time the venue reports for the side that changed, not with this host's clock. And a side with no
//! levels is not a side quoted at zero, so half a book produces no tick.

use anyhow::{Context, bail};
use nautilus_core::UnixNanos;
use nautilus_model::{
    data::{BookOrder, OrderBookDelta, OrderBookDeltas, QuoteTick},
    enums::{BookAction, OrderSide, RecordFlag},
    identifiers::InstrumentId,
    types::{Price, Quantity},
};

use crate::generated::qot_common::OrderBook;

/// The gateway's `OrderBookType_Normal`: the board ordinary-sized orders are quoted on.
pub const BOARD_NORMAL: i32 = 0;

/// The gateway's `OrderBookType_Odd`: the odd-lot board, which is a book of its own.
pub const BOARD_ODD: i32 = 1;

/// The sequence a book record is told, because the venue assigns none.
///
/// The domain book skips its ordering check for a zero sequence, which is exactly what a venue
/// without sequence numbers needs: a counter invented here would be compared against the previous
/// record and would eventually fail that comparison, discarding a level the venue did send.
pub const NO_VENUE_SEQUENCE: u64 = 0;

/// Returns whether a book record is from the ordinary board.
///
/// An absent type is the ordinary board, because that is what the request asks for when it does not
/// name one.
#[must_use]
pub fn is_ordinary_board(board: Option<i32>) -> bool {
    board != Some(BOARD_ODD)
}

/// Refuses a book record from a board that is not the ordinary one.
///
/// The odd-lot board is a separate book with its own prices, and a consumer reading it as the
/// ordinary book would see a best bid that no ordinary order is filling. The record's own board
/// field is what is checked rather than the board the request asked for, because the gateway
/// answers a request for the odd-lot board with the ordinary one and reports it as ordinary, so
/// the request carries no evidence about what came back.
fn check_board(board: Option<i32>) -> anyhow::Result<()> {
    if is_ordinary_board(board) {
        return Ok(());
    }

    bail!("the gateway sent the odd-lot board, which is not the book this request asked for")
}

/// Resolves the venue's own receive time for one side of the book into a timestamp.
///
/// The venue reports an instant twice, as a number and as a text field, and only the number is
/// usable: the text is the wall clock of the market, which is neither this host's clock nor UTC,
/// while the number is a fractional count of seconds since the epoch. Observed against the gateway,
/// the two disagree by the market's offset from UTC, so reading the text would place every book
/// update hours away from the trade ticks beside it.
///
/// The venue reports zero when it has no instant to report, which the protocol says happens for the
/// first cached push and after a restart. Zero is therefore no instant and not the epoch, and the
/// caller falls back to the local clock.
///
/// The fraction is placed on its own rather than by scaling the whole value, because a nanosecond
/// count near the epoch is past the range a float states exactly and scaling first would round to
/// the nearest few hundred nanoseconds.
#[must_use]
pub fn venue_recv_time(seconds: Option<f64>) -> Option<UnixNanos> {
    let seconds = seconds?;

    if !seconds.is_finite() || seconds <= 0.0 {
        return None;
    }

    let whole = seconds.trunc();
    let fraction = (seconds.fract() * 1_000_000_000.0).round() as u64;

    Some(UnixNanos::from(whole as u64 * 1_000_000_000 + fraction))
}

/// Returns the timestamp to stamp a book update with.
///
/// The venue's own instant is preferred, because it places the update at the exchange rather than
/// at this host, and the local receive time is the fallback for the markets and the records the
/// venue does not cover.
#[must_use]
pub fn side_time(venue_seconds: Option<f64>, local: UnixNanos) -> UnixNanos {
    venue_recv_time(venue_seconds).unwrap_or(local)
}

/// Returns the size the venue reported for one book level.
///
/// The venue states a level twice, as a whole number and as a value that can carry a fraction, and
/// the second is preferred when it is sent because it loses nothing the first one has.
#[must_use]
pub fn level_size(level: &OrderBook) -> f64 {
    level.hp_volume.unwrap_or(level.volume as f64)
}

/// What a book record is read against.
///
/// These are the facts about the instrument rather than about the record, so a caller serving one
/// instrument builds this once and reuses it for every push, and both the push path and the request
/// path follow the same rules.
#[derive(Debug, Clone, Copy)]
pub struct BookContext {
    /// The instrument the book belongs to.
    pub instrument_id: InstrumentId,
    /// The instrument's price precision, which every level price must be stated at.
    pub price_precision: u8,
    /// The instrument's size precision, which every level size must be stated at.
    pub size_precision: u8,
    /// The number of levels to keep per side, whatever the venue sends.
    pub depth: usize,
    /// When the record reached this host.
    pub ts_init: UnixNanos,
}

/// Builds the top of book quote from the best level of each side.
///
/// Returns `None` when either side has no levels, because a tick carrying half a book would report
/// a price the other side never agreed to. A level priced at or below zero is treated the same way:
/// the venue states an absent side as an empty list, so a zero-priced level is a degraded record
/// rather than a real order, and no price is better than a price nothing can trade at.
///
/// # Errors
///
/// Returns an error if a price or a size falls outside the range the domain type holds. A value
/// carrying more decimals than the instrument states is rounded to the instrument's precision
/// rather than refused, which is what the domain types do with one everywhere.
pub fn quote_tick_from(
    bids: &[OrderBook],
    asks: &[OrderBook],
    context: &BookContext,
    ts_event: UnixNanos,
) -> anyhow::Result<Option<QuoteTick>> {
    let (Some(bid), Some(ask)) = (bids.first(), asks.first()) else {
        return Ok(None);
    };

    if bid.price <= 0.0 || ask.price <= 0.0 {
        return Ok(None);
    }

    let bid_price = Price::new_checked(bid.price, context.price_precision)
        .context("cannot represent the best bid at the instrument's precision")?;
    let ask_price = Price::new_checked(ask.price, context.price_precision)
        .context("cannot represent the best ask at the instrument's precision")?;
    let bid_size = Quantity::new_checked(level_size(bid), context.size_precision)
        .context("cannot represent the best bid size at the instrument's precision")?;
    let ask_size = Quantity::new_checked(level_size(ask), context.size_precision)
        .context("cannot represent the best ask size at the instrument's precision")?;

    QuoteTick::new_checked(
        context.instrument_id,
        bid_price,
        ask_price,
        bid_size,
        ask_size,
        ts_event,
        context.ts_init,
    )
    .map(Some)
}

/// Builds the domain book for one book record, which the venue sends whole.
///
/// The record replaces the book rather than amending it, so the batch opens with a clear carrying
/// the snapshot flag and then adds every level the venue sent, up to the requested depth. The
/// venue's push ignores the depth the subscription named and sends its full ladder, so the depth is
/// applied here: sending sixty levels for a subscription taken for ten would claim a book deeper
/// than the entitlement the gateway granted.
///
/// Levels follow the order the venue sent them in, bids before asks, and the batch ends with the
/// last flag so a consumer knows the snapshot is complete.
///
/// Each level is stamped with the receive time of its own side, because the venue tracks the two
/// sides separately and a level is not on the book until the side carrying it has arrived. The
/// batch as a whole reports the timestamp of its last level, which is the ask side.
///
/// # Errors
///
/// Returns an error if the record is from the odd-lot board, if a level carries no size, or if a
/// price falls outside the range the domain type holds. A level with no size is refused because a
/// snapshot holding one would differ from the venue's own ladder while looking complete, which is
/// worse than a refusal an operator can act on.
pub fn book_deltas_from(
    bids: &[OrderBook],
    asks: &[OrderBook],
    context: &BookContext,
    board: Option<i32>,
    ts_bid: UnixNanos,
    ts_ask: UnixNanos,
) -> anyhow::Result<OrderBookDeltas> {
    check_board(board)?;

    let bids = &bids[..bids.len().min(context.depth)];
    let asks = &asks[..asks.len().min(context.depth)];

    let mut deltas = Vec::with_capacity(bids.len() + asks.len() + 1);

    let last_bid = bids.len().checked_sub(1);
    let last_ask = asks.len().checked_sub(1);

    // The snapshot is only complete once both sides have arrived, so the clear is stamped with the
    // later of the two rather than with whichever side happened to be sent first.
    let cleared_at = if ts_bid > ts_ask { ts_bid } else { ts_ask };

    let mut clear = OrderBookDelta::clear(
        context.instrument_id,
        NO_VENUE_SEQUENCE,
        cleared_at,
        context.ts_init,
    );

    // An empty record still marks the end, so a consumer waiting for the last flag is not left
    // waiting for a snapshot that has already arrived.
    if last_bid.is_none() && last_ask.is_none() {
        clear.flags |= RecordFlag::F_LAST as u8;
    }

    deltas.push(clear);

    let mut push_level =
        |level: &OrderBook, side: OrderSide, ts: UnixNanos, last: bool| -> anyhow::Result<()> {
            let price = Price::new_checked(level.price, context.price_precision)
                .context("cannot represent a book level price at the instrument's precision")?;
            let size = Quantity::new_checked(level_size(level), context.size_precision)
                .context("cannot represent a book level size at the instrument's precision")?;

            // A level is an aggregate over its orders, so it carries no order identifier of its
            // own.
            let order = BookOrder::new(side, price, size, 0);
            let mut flags = RecordFlag::F_MBP as u8;

            if last {
                flags |= RecordFlag::F_LAST as u8;
            }

            let delta = OrderBookDelta::new_checked(
                context.instrument_id,
                BookAction::Add,
                order,
                flags,
                NO_VENUE_SEQUENCE,
                ts,
                context.ts_init,
            )
            .context("cannot build a book level delta")?;

            deltas.push(delta);
            Ok(())
        };

    for (index, level) in bids.iter().enumerate() {
        // The last level of the batch is the last ask when there is one, so a bid only ends the
        // batch when the ask side is empty.
        let last = last_ask.is_none() && Some(index) == last_bid;

        push_level(level, OrderSide::Buy, ts_bid, last)?;
    }

    for (index, level) in asks.iter().enumerate() {
        push_level(level, OrderSide::Sell, ts_ask, Some(index) == last_ask)?;
    }

    OrderBookDeltas::new_checked(context.instrument_id, deltas)
        .context("cannot assemble the order book deltas")
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    /// One rung of the ladder as the venue reports it, taken from a recorded push.
    fn level(price: f64, volume: i64) -> OrderBook {
        OrderBook {
            price,
            volume,
            oreder_count: 0,
            detail_list: Vec::new(),
            hp_volume: None,
        }
    }

    fn ladder() -> (Vec<OrderBook>, Vec<OrderBook>) {
        let bids = vec![level(332.8, 24), level(332.79, 185), level(332.78, 40)];
        let asks = vec![level(332.83, 119), level(332.84, 105), level(332.88, 43)];

        (bids, asks)
    }

    fn instrument() -> InstrumentId {
        InstrumentId::from("AAPL.US")
    }

    fn context() -> BookContext {
        BookContext {
            instrument_id: instrument(),
            price_precision: 2,
            size_precision: 0,
            depth: 10,
            ts_init: TS_INIT,
        }
    }

    // The two sides are stamped apart, as the venue stamped one recorded push apart.
    const TS_BID: UnixNanos = UnixNanos::new(1_790_957_780_500_000_000);
    const TS_ASK: UnixNanos = UnixNanos::new(1_790_957_781_046_000_000);
    const TS_INIT: UnixNanos = UnixNanos::new(1_790_957_790_000_000_000);

    /// The venue states its receive time in seconds, and the recorded value is read as the instant
    /// the venue meant rather than as the double it sent. At this magnitude a double cannot state a
    /// thousandth at all, so the exact double behind `1_790_957_780.436` is
    /// `1_790_957_780.435999989...`, which is ten nanoseconds short of the stated instant. The text
    /// field beside it is the market's wall clock, which disagrees with UTC by the market's offset,
    /// so only the number can be used.
    #[rstest]
    fn test_the_venue_receive_time_is_seconds() {
        let placed = venue_recv_time(Some(1_790_957_780.436)).unwrap();

        assert!(
            placed.as_u64().abs_diff(1_790_957_780_436_000_000) < 1_000,
            "expected the stated instant within a microsecond, got {placed}",
        );
    }

    /// The fraction is placed on its own, because scaling a whole nanosecond count by a float would
    /// round away the part of the instant the venue actually stated.
    #[rstest]
    #[case::half(1_790_957_780.5, 500_000_000)]
    #[case::quarter(1_790_957_780.25, 250_000_000)]
    #[case::eighth(1_790_957_780.125, 125_000_000)]
    #[case::whole(1_790_957_780.0, 0)]
    fn test_the_fraction_is_placed_exactly(#[case] seconds: f64, #[case] expected: u64) {
        let placed = venue_recv_time(Some(seconds)).unwrap();

        assert_eq!(
            placed,
            UnixNanos::new(1_790_957_780 * 1_000_000_000 + expected),
        );
    }

    /// Zero is the venue saying it has no instant, which the protocol says it does for the first
    /// cached push of a subscription. Reading it as the epoch would date the book to 1970.
    #[rstest]
    #[case::zero(Some(0.0))]
    #[case::absent(None)]
    #[case::negative(Some(-1.0))]
    #[case::not_a_number(Some(f64::NAN))]
    #[case::infinite(Some(f64::INFINITY))]
    fn test_an_absent_receive_time_is_not_the_epoch(#[case] value: Option<f64>) {
        assert_eq!(venue_recv_time(value), None);
    }

    /// The local clock is the fallback for the records the venue gives no instant for, and only
    /// the fallback.
    #[rstest]
    fn test_the_venue_time_is_preferred_over_the_local_clock() {
        let local = UnixNanos::new(42);

        assert_eq!(side_time(Some(1_790_957_780.5), local), TS_BID);
        assert_eq!(side_time(Some(0.0), local), local);
        assert_eq!(side_time(None, local), local);
    }

    /// The venue states a level twice, and the value that can carry a fraction is preferred.
    #[rstest]
    fn test_the_higher_precision_level_size_is_preferred() {
        let mut rung = level(332.8, 24);
        assert_eq!(level_size(&rung), 24.0);

        rung.hp_volume = Some(24.5);
        assert_eq!(level_size(&rung), 24.5);
    }

    /// A quote tick carries a bid and an ask that the venue actually has, not a price derived from
    /// anything else.
    #[rstest]
    fn test_a_quote_carries_the_real_best_bid_and_ask() {
        let (bids, asks) = ladder();
        let quote = quote_tick_from(&bids, &asks, &context(), TS_ASK)
            .unwrap()
            .expect("both sides carry levels");

        assert_eq!(quote.instrument_id, instrument());
        assert_eq!(quote.bid_price, Price::new(332.80, 2));
        assert_eq!(quote.ask_price, Price::new(332.83, 2));
        assert_eq!(quote.bid_size, Quantity::new(24.0, 0));
        assert_eq!(quote.ask_size, Quantity::new(119.0, 0));
        assert_eq!(quote.ts_event, TS_ASK);
        assert_eq!(quote.ts_init, TS_INIT);
    }

    /// Half a book is not a quote. The venue states a side it has none of as an empty list, and a
    /// tick carrying half of it would report a price the other side never agreed to.
    #[rstest]
    #[case::no_bids(vec![], vec![level(332.83, 119)])]
    #[case::no_asks(vec![level(332.8, 24)], vec![])]
    #[case::neither(vec![], vec![])]
    fn test_half_a_book_is_not_a_quote(#[case] bids: Vec<OrderBook>, #[case] asks: Vec<OrderBook>) {
        let quote = quote_tick_from(&bids, &asks, &context(), TS_ASK).unwrap();

        assert!(quote.is_none());
    }

    /// A level priced at zero cannot be an order, so it is a degraded record rather than a side
    /// quoted at nothing.
    #[rstest]
    #[case::bid(vec![level(0.0, 24)], vec![level(332.83, 119)])]
    #[case::ask(vec![level(332.8, 24)], vec![level(0.0, 119)])]
    fn test_a_zero_priced_side_is_not_a_quote(
        #[case] bids: Vec<OrderBook>,
        #[case] asks: Vec<OrderBook>,
    ) {
        let quote = quote_tick_from(&bids, &asks, &context(), TS_ASK).unwrap();

        assert!(quote.is_none());
    }

    /// A level with no size is not a level. The domain book refuses an add of nothing, and a
    /// snapshot carrying one is refused rather than admitted with a hole in it.
    #[rstest]
    fn test_a_level_with_no_size_is_refused() {
        let bids = vec![level(332.8, 0)];
        let asks = vec![level(332.83, 119)];

        let refused =
            book_deltas_from(&bids, &asks, &context(), Some(BOARD_NORMAL), TS_BID, TS_ASK);

        assert!(refused.is_err());
    }

    /// The record replaces the book rather than amending it, so it opens with a clear carrying the
    /// snapshot flag and ends with the last flag once every level is in.
    #[rstest]
    fn test_a_book_record_becomes_a_replacing_snapshot() {
        let (bids, asks) = ladder();
        let deltas =
            book_deltas_from(&bids, &asks, &context(), Some(BOARD_NORMAL), TS_BID, TS_ASK).unwrap();

        assert_eq!(deltas.deltas.len(), 7, "a clear and three levels a side");

        let clear = &deltas.deltas[0];
        assert_eq!(clear.action, BookAction::Clear);
        assert_ne!(clear.flags & RecordFlag::F_SNAPSHOT as u8, 0);
        assert_eq!(clear.sequence, NO_VENUE_SEQUENCE);

        for delta in &deltas.deltas[1..] {
            assert_eq!(delta.action, BookAction::Add);
            assert_ne!(delta.flags & RecordFlag::F_MBP as u8, 0);
            assert_eq!(delta.flags & RecordFlag::F_SNAPSHOT as u8, 0);
        }

        assert_eq!(deltas.deltas[1].order.side, Some(OrderSide::Buy));
        assert_eq!(deltas.deltas[3].order.side, Some(OrderSide::Buy));
        assert_eq!(deltas.deltas[4].order.side, Some(OrderSide::Sell));
        assert_ne!(
            deltas.deltas.last().unwrap().flags & RecordFlag::F_LAST as u8,
            0,
        );
        assert_eq!(deltas.deltas[1].order.price, Price::new(332.80, 2));
        assert_eq!(deltas.deltas[1].order.size, Quantity::new(24.0, 0));
    }

    /// Each level is stamped with the receive time of its own side, because the venue tracks the
    /// two sides apart and a level is not on the book until its own side has arrived.
    #[rstest]
    fn test_levels_are_stamped_with_their_own_side_time() {
        let (bids, asks) = ladder();
        let deltas =
            book_deltas_from(&bids, &asks, &context(), Some(BOARD_NORMAL), TS_BID, TS_ASK).unwrap();

        assert_eq!(deltas.deltas[1].ts_event, TS_BID);
        assert_eq!(deltas.deltas[4].ts_event, TS_ASK);
        assert_eq!(
            deltas.deltas[0].ts_event, TS_ASK,
            "the snapshot is complete once the later side has arrived",
        );
        assert_eq!(deltas.ts_event, TS_ASK, "the batch reports its last level");
    }

    /// The push carries the venue's full ladder whatever depth the subscription named, so the
    /// depth has to be applied here or the book would be deeper than the subscription was taken
    /// for.
    #[rstest]
    fn test_the_requested_depth_caps_the_levels() {
        let bids: Vec<OrderBook> = (0..60)
            .map(|i| level((33_280 - i) as f64 / 100.0, 10))
            .collect();
        let asks: Vec<OrderBook> = (0..60)
            .map(|i| level((33_283 + i) as f64 / 100.0, 10))
            .collect();

        let deltas =
            book_deltas_from(&bids, &asks, &context(), Some(BOARD_NORMAL), TS_BID, TS_ASK).unwrap();

        assert_eq!(deltas.deltas.len(), 21, "a clear and ten levels a side");
        assert_eq!(deltas.deltas[10].order.price, Price::new(332.71, 2));
        assert_eq!(deltas.deltas[20].order.price, Price::new(332.92, 2));
    }

    /// The odd-lot board is a book of its own, and reading it as the ordinary book would report a
    /// best bid that no ordinary order is filling.
    #[rstest]
    #[case::normal(Some(BOARD_NORMAL), true)]
    #[case::absent(None, true)]
    #[case::odd(Some(BOARD_ODD), false)]
    fn test_the_odd_lot_board_is_told_apart(#[case] board: Option<i32>, #[case] expected: bool) {
        assert_eq!(is_ordinary_board(board), expected);
    }

    #[rstest]
    fn test_the_odd_lot_board_is_refused() {
        let (bids, asks) = ladder();

        let refused = book_deltas_from(&bids, &asks, &context(), Some(BOARD_ODD), TS_BID, TS_ASK);

        assert!(refused.is_err());
    }

    /// A record the venue sent with no levels still marks the end, so a consumer waiting for the
    /// last flag is not left waiting for a book that has already arrived.
    #[rstest]
    fn test_an_empty_record_still_ends_the_snapshot() {
        let deltas =
            book_deltas_from(&[], &[], &context(), Some(BOARD_NORMAL), TS_BID, TS_ASK).unwrap();

        assert_eq!(deltas.deltas.len(), 1);
        assert_ne!(
            deltas.deltas[0].flags & RecordFlag::F_LAST as u8,
            0,
            "the clear is also the last record when there are no levels",
        );
    }
}
