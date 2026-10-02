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

//! Conversion of the gateway's ticker records to the domain model.
//!
//! Two things make this feed better than a synthesised one, and both are the venue's rather than
//! this adapter's: the direction is the venue's own aggressor, and the sequence number is the
//! venue's own identifier for the trade. The sequence is what makes a replay after a reconnect
//! deduplicate for free, because a trade already seen carries an identifier already seen.
//!
//! The one trap is that the venue replays its last known value when a subscription starts. That
//! replay is not a trade, and a consumer that emits it opens every session with a trade that never
//! happened at a price from the previous session.

use anyhow::Context;
use nautilus_model::{
    data::TradeTick,
    enums::AggressorSide,
    identifiers::{InstrumentId, TradeId},
    types::{Price, Quantity},
};

use crate::{common::Market, generated::qot_common::Ticker, mappers::bars::intraday_ts_event};

/// The precision of a share count, which is a whole number.
const SIZE_PRECISION: u8 = 0;

/// The gateway's `PushDataType_Cache`, which marks a replayed last known value.
pub const PUSH_DATA_TYPE_CACHE: i32 = 3;

/// The gateway's `TickerDirection_Bid`, the direction of a trade that hit the bid.
pub const DIRECTION_BID: i32 = 1;

/// The gateway's `TickerDirection_Ask`, the direction of a trade that lifted the offer.
pub const DIRECTION_ASK: i32 = 2;

/// Returns whether a ticker record is a trade the venue just reported.
///
/// A cached record is the venue replaying the last value it holds, and it arrives as the first
/// record of a fresh subscription. It carries the previous session's price and a sequence number
/// from that session, so emitting it would both invent a trade and seed the deduplication with a
/// stale identifier.
#[must_use]
pub fn is_reported_trade(ticker: &Ticker) -> bool {
    ticker.push_data_type != Some(PUSH_DATA_TYPE_CACHE)
}

/// Maps the gateway's direction onto an aggressor side.
///
/// A trade that hit the bid was caused by a seller, and a trade that lifted the offer was caused by
/// a buyer. The venue also reports a neutral direction, which is a real value and not a missing one,
/// so it becomes no aggressor rather than being guessed at.
#[must_use]
pub fn aggressor_side(direction: i32) -> AggressorSide {
    match direction {
        DIRECTION_BID => AggressorSide::Sell,
        DIRECTION_ASK => AggressorSide::Buy,
        _ => AggressorSide::NoAggressor,
    }
}

/// Builds a trade tick from a ticker record.
///
/// The venue reports a volume twice, as a whole number and as a higher precision value, and the
/// higher precision one is preferred when the venue sends it because it states the same quantity
/// without losing a fractional share.
///
/// # Errors
///
/// Returns an error if the timestamp cannot be placed, if the price is not representable at the
/// instrument's precision, or if the size is not a positive quantity.
pub fn trade_tick_from(
    ticker: &Ticker,
    instrument_id: InstrumentId,
    market: Market,
    price_precision: u8,
) -> anyhow::Result<TradeTick> {
    let ts_event = intraday_ts_event(&ticker.time, market)
        .with_context(|| format!("cannot place the trade at {}", ticker.time))?;
    let price = Price::new_checked(ticker.price, price_precision)?;
    let size = Quantity::new_checked(trade_size(ticker), SIZE_PRECISION)?;
    let trade_id = TradeId::new(ticker.sequence.to_string());

    TradeTick::new_checked(
        instrument_id,
        price,
        size,
        aggressor_side(ticker.dir),
        trade_id,
        ts_event,
        ts_event,
    )
}

/// Returns whether a ticker record states a size the domain type can hold as a trade.
///
/// The venue pushes a ticker record for changes that are not trades, and the common one carries no
/// size at all: a price with nothing having traded at it. It also states quantities with a fraction
/// this adapter's size precision cannot hold, observed live as records whose size was under half a
/// share, and those are not trades either.
///
/// The test is made at the precision the size is built at rather than against zero, because a
/// quantity that rounds to zero is refused by the mapper for the same reason a missing one is:
/// there is no trade to report. A caller that sees this traffic routinely recognises it here, and a
/// caller that does not still gets a refusal rather than a phantom tick.
#[must_use]
pub fn has_size(ticker: &Ticker) -> bool {
    let scaled = trade_size(ticker) * 10f64.powi(i32::from(SIZE_PRECISION));

    scaled.round() >= 1.0
}

/// Returns the trade size the venue reported, preferring its higher precision value.
///
/// The venue states the volume twice, as a whole number and as a value that can carry a fraction,
/// and the second is preferred when it is sent because it loses nothing the first one has.
fn trade_size(ticker: &Ticker) -> f64 {
    ticker.hp_volume.unwrap_or(ticker.volume as f64)
}

#[cfg(test)]
mod tests {
    use nautilus_core::UnixNanos;
    use rstest::rstest;

    use super::*;

    fn ticker(direction: i32) -> Ticker {
        Ticker {
            time: "2026-10-01 16:00:01.554".to_string(),
            sequence: 7_691_791_651_198_470_520,
            dir: direction,
            price: 330.32,
            volume: 180,
            turnover: 59_457.6,
            push_data_type: Some(1),
            ..Default::default()
        }
    }

    /// The direction is the venue's aggressor, and a trade that hit the bid was caused by a seller.
    #[rstest]
    #[case::bid(DIRECTION_BID, AggressorSide::Sell)]
    #[case::ask(DIRECTION_ASK, AggressorSide::Buy)]
    #[case::neutral(3, AggressorSide::NoAggressor)]
    #[case::unknown(0, AggressorSide::NoAggressor)]
    fn test_direction_maps_to_the_causing_side(
        #[case] direction: i32,
        #[case] expected: AggressorSide,
    ) {
        assert_eq!(aggressor_side(direction), expected);
    }

    #[rstest]
    fn test_a_trade_maps_to_a_tick() {
        let tick = trade_tick_from(
            &ticker(DIRECTION_BID),
            InstrumentId::from("AAPL.US"),
            Market::Us,
            2,
        )
        .unwrap();

        assert_eq!(tick.instrument_id, InstrumentId::from("AAPL.US"));
        assert_eq!(tick.price, Price::new(330.32, 2));
        assert_eq!(tick.size, Quantity::new(180.0, 0));
        assert_eq!(tick.aggressor_side, AggressorSide::Sell);
        assert_eq!(tick.trade_id.to_string(), "7691791651198470520");
        // 16:00:01.554 in New York on 2026-10-01, which is after the daylight saving change.
        assert_eq!(tick.ts_event, UnixNanos::new(1_790_884_801_554_000_000));
    }

    /// The first record of a fresh subscription is a replay, and emitting it would invent a trade
    /// at the previous session's price.
    #[rstest]
    #[case::realtime(Some(1), true)]
    #[case::cache(Some(PUSH_DATA_TYPE_CACHE), false)]
    #[case::absent(None, true)]
    fn test_only_reported_trades_are_emitted(#[case] kind: Option<i32>, #[case] expected: bool) {
        let mut ticker = ticker(DIRECTION_BID);
        ticker.push_data_type = kind;

        assert_eq!(is_reported_trade(&ticker), expected);
    }

    #[rstest]
    fn test_the_higher_precision_size_is_preferred() {
        let mut ticker = ticker(DIRECTION_ASK);
        ticker.hp_volume = Some(180.5);

        assert_eq!(trade_size(&ticker), 180.5);
    }

    /// A record whose size the domain type cannot hold as a trade is a change that is not a trade,
    /// which the venue pushes routinely. The fraction below half a share is the one seen live: it
    /// rounds to zero once it is built at a whole-share precision.
    #[rstest]
    #[case::a_whole_number(180, None, true)]
    #[case::a_half_share(0, Some(0.5), true)]
    #[case::below_half_a_share(0, Some(0.4), false)]
    #[case::a_fraction_of_a_whole_share(0, Some(1.5), true)]
    #[case::nothing(0, None, false)]
    #[case::nothing_with_a_fraction(0, Some(0.0), false)]
    fn test_only_a_record_with_a_size_could_have_traded(
        #[case] volume: i64,
        #[case] hp_volume: Option<f64>,
        #[case] expected: bool,
    ) {
        let mut ticker = ticker(1);
        ticker.volume = volume;
        ticker.hp_volume = hp_volume;

        assert_eq!(has_size(&ticker), expected);
    }

    #[rstest]
    fn test_an_unplaceable_time_is_refused() {
        let mut ticker = ticker(DIRECTION_BID);
        ticker.time = "not a time".to_string();

        assert!(trade_tick_from(&ticker, InstrumentId::from("AAPL.US"), Market::Us, 2).is_err());
    }
}
