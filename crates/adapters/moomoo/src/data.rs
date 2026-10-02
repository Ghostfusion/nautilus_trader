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

//! Live data client for the moomoo OpenD gateway.
//!
//! The gateway is push-driven, so the client's shape is a connection, a subscription manager, and a
//! task that turns the gateway's pushes into domain events. Everything a consumer asks for has a
//! venue-side counterpart, and the manager is what keeps the two in step.
//!
//! # What a subscription is served by
//!
//! The venue holds a subscription per security and data type, and the three types this client uses
//! are not the three a consumer names. A consumer asks for quotes, trades, or a book; the venue is
//! asked for a book for either of the first two, because the venue's quote snapshot carries no bid
//! or ask, and a quote is built from the book's level one. A bar subscription is the venue's K-line
//! push for one interval, which is why the interval is part of the subscription type.
//!
//! # What the push path knows
//!
//! The push path is a task reading the gateway rather than a call the engine makes, so it cannot
//! read the client's own state. It is given what it needs: the loaded instruments, for the
//! precisions an event is stated at, and the set of things a consumer is waiting for, because the
//! engine treats a tick nobody asked for as a data error and the venue will push whatever it holds a
//! subscription for rather than what this client last cared about.
//!
//! # The engine's subscribe path is synchronous
//!
//! The venue is called with an await, and the trait's subscribe methods are not async, so each
//! subscription is taken by a task and a failure is logged rather than returned. The alternative,
//! blocking a thread that belongs to the engine's runtime, is worse than a log line an operator can
//! find.

use std::{
    collections::{HashMap, HashSet},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use anyhow::{Context, bail};
use async_trait::async_trait;
use indexmap::IndexMap;
use jiff::Timestamp;
use nautilus_common::{
    clients::DataClient,
    live::{get_runtime, runner::try_get_data_event_sender, sender::EventSender},
    messages::{
        DataEvent,
        data::{
            RequestBars, RequestInstrument, RequestInstruments, SubscribeBars, SubscribeBookDeltas,
            SubscribeInstrument, SubscribeInstruments, SubscribeQuotes, SubscribeTrades,
            UnsubscribeBars, UnsubscribeBookDeltas, UnsubscribeQuotes, UnsubscribeTrades,
        },
    },
};
use nautilus_core::time::get_atomic_clock_realtime;
use nautilus_model::{
    data::{BarType, Data},
    enums::AggregationSource,
    identifiers::{ClientId, InstrumentId, Symbol, Venue},
    instruments::{Instrument, InstrumentAny},
};
use parking_lot::Mutex;
use prost::Message as _;
use tokio::{sync::mpsc, task::JoinHandle};

use crate::{
    common::{CLIENT_ID, Market},
    config::MoomooDataClientConfig,
    connection::{ConnectOptions, Connection, Message, connect},
    entitlement::Capabilities,
    generated::{qot_update_order_book, qot_update_ticker},
    loader::{MarketLoad, load_market},
    mappers::{
        bars::{Interval, build_bars},
        book::{BookContext, book_deltas_from, quote_tick_from, side_time},
        trades::{has_size, is_reported_trade, trade_tick_from},
    },
    providers::{self, HistoryRequest},
    subscription::{RELEASE_DELAY, Subscription, SubscriptionManager, SubscriptionType},
};

/// The protocol identifier of the K-line push.
const PROTO_ID_UPDATE_KL: u32 = 3007;

/// The protocol identifier of the ticker push.
const PROTO_ID_UPDATE_TICKER: u32 = 3011;

/// The protocol identifier of the order book push.
const PROTO_ID_UPDATE_ORDER_BOOK: u32 = 3013;

/// The protocol identifier of the gateway's own notifications.
const PROTO_ID_NOTIFY: u32 = 1003;

/// The interval a bar request is given when it names no start, for a bar finer than a day.
const DEFAULT_INTRADAY_WINDOW_DAYS: i64 = 5;

/// The interval a bar request is given when it names no start, for a day or coarser.
const DEFAULT_DAILY_WINDOW_DAYS: i64 = 365;

/// The seconds in a day.
const SECONDS_PER_DAY: i64 = 86_400;

/// What the consumers of one client are waiting for.
///
/// The push path reads this rather than the client's own subscription state, which it cannot see.
/// The three kinds are kept apart even where the venue serves them from one subscription, because
/// what a consumer asked for is what it may be given: a book subscription taken to serve quotes
/// must not deliver deltas to an engine that never asked for a book.
#[derive(Debug, Default)]
struct Watches {
    /// The instruments whose trades a consumer wants.
    trades: HashSet<InstrumentId>,
    /// The instruments whose quotes a consumer wants.
    quotes: HashSet<InstrumentId>,
    /// The instruments whose book deltas a consumer wants.
    books: HashSet<InstrumentId>,
    /// The bar types a consumer wants, keyed by what a K-line push identifies it by.
    bars: HashMap<(InstrumentId, Interval), BarType>,
}

/// A live data client for the moomoo OpenD gateway.
///
/// The client is multi-venue: an instrument's venue is the market it trades on, so
/// [`DataClient::venue`] returns `None`. Register the client under a client ID and address
/// subscriptions to that ID when a node runs more than one data client.
#[derive(Debug)]
pub struct MoomooDataClient {
    client_id: ClientId,
    config: MoomooDataClientConfig,
    connection: Option<Arc<Connection>>,
    subscriptions: Option<Arc<SubscriptionManager>>,
    capabilities: Option<Capabilities>,
    instruments: Arc<Mutex<IndexMap<InstrumentId, InstrumentAny>>>,
    watches: Arc<Mutex<Watches>>,
    data_sender: EventSender<DataEvent>,
    is_connected: Arc<AtomicBool>,
    pushes: Option<JoinHandle<()>>,
}

impl MoomooDataClient {
    /// Creates a new [`MoomooDataClient`] instance.
    ///
    /// The gateway is not contacted here: a client is constructed and connected separately, so that
    /// a node can build one before its runtime is running.
    ///
    /// # Errors
    ///
    /// Returns an error if no data event sender has been installed on this thread, which is a
    /// client built outside the node that was going to consume what it produces.
    pub fn new(client_id: ClientId, config: MoomooDataClientConfig) -> anyhow::Result<Self> {
        let data_sender = try_get_data_event_sender().context(
            "no data event sender is installed on this thread, so a client built here would have \
             nowhere to send what it receives",
        )?;

        Ok(Self {
            client_id,
            config,
            connection: None,
            subscriptions: None,
            capabilities: None,
            instruments: Arc::new(Mutex::new(IndexMap::new())),
            watches: Arc::new(Mutex::new(Watches::default())),
            data_sender,
            is_connected: Arc::new(AtomicBool::new(false)),
            pushes: None,
        })
    }

    /// Returns the instruments the client currently holds, in load order.
    #[must_use]
    pub fn instruments(&self) -> Vec<InstrumentAny> {
        self.instruments.lock().values().cloned().collect()
    }

    /// Returns what the gateway said this login may do, once it has been read.
    #[must_use]
    pub fn capabilities(&self) -> Option<&Capabilities> {
        self.capabilities.as_ref()
    }

    /// Returns the subscriptions the venue currently holds.
    #[must_use]
    pub fn held_subscriptions(&self) -> Vec<Subscription> {
        self.subscriptions
            .as_ref()
            .map_or_else(Vec::new, |manager| manager.held())
    }

    fn instrument(&self, instrument_id: &InstrumentId) -> Option<InstrumentAny> {
        self.instruments.lock().get(instrument_id).cloned()
    }

    fn store_instruments(&self, instruments: Vec<InstrumentAny>) {
        let mut held = self.instruments.lock();

        for instrument in instruments {
            held.insert(instrument.id(), instrument);
        }
    }

    fn emit_instruments(&self, instruments: &[InstrumentAny]) {
        for instrument in instruments {
            if let Err(e) = self
                .data_sender
                .send(DataEvent::Instrument(instrument.clone()))
            {
                log::error!("failed to send an instrument event: {e}");
            }
        }
    }

    /// Returns the price precision an instrument's events are stated at.
    fn price_precision(&self, instrument_id: &InstrumentId, market: Market) -> u8 {
        self.instrument(instrument_id).map_or_else(
            || market.fallback_price_precision(),
            |i| i.price_precision(),
        )
    }

    /// Returns the connection, or an error naming what has not happened.
    fn connection(&self) -> anyhow::Result<Arc<Connection>> {
        self.connection
            .clone()
            .context("the client is not connected to the gateway")
    }

    /// Returns the subscription manager, or an error naming what has not happened.
    fn subscriptions(&self) -> anyhow::Result<Arc<SubscriptionManager>> {
        self.subscriptions
            .clone()
            .context("the client is not connected to the gateway")
    }

    /// Takes a venue-side subscription for a consumer's interest.
    ///
    /// The manager records the interest and then asks the venue, and only the second half is an
    /// await, so what a caller can be told is a failure to start the work rather than a failure of
    /// it. A refusal from the venue, such as an exhausted allowance, is logged with the allowance
    /// named in it.
    fn take_subscription(&self, subscription: Subscription) -> anyhow::Result<()> {
        let manager = self.subscriptions()?;

        get_runtime().spawn(async move {
            if let Err(e) = manager.subscribe(subscription.clone()).await {
                log::error!("cannot subscribe to {subscription:?}: {e}");
            }
        });

        Ok(())
    }

    /// Gives up a consumer's interest in a venue-side subscription.
    fn release_subscription(&self, subscription: &Subscription) -> anyhow::Result<()> {
        let manager = self.subscriptions()?;

        // The manager stops the held subscription after the venue's own minimum, so this returns as
        // soon as the interest is recorded rather than when the venue has let go.
        manager.unsubscribe(subscription)
    }

    /// Adds an instrument a consumer asked for by name, if it is not already held.
    ///
    /// A consumer that names one instrument should not have to load a market to get it, so the
    /// instrument is loaded by the same two requests the universe load uses for one security, and
    /// the answer is emitted when it arrives.
    fn load_instrument(&self, instrument_id: InstrumentId) -> anyhow::Result<()> {
        let connection = self.connection()?;
        let sender = self.data_sender.clone();
        let instruments = Arc::clone(&self.instruments);

        get_runtime().spawn(async move {
            let Ok((market, code)) = market_of(instrument_id) else {
                log::warn!("{instrument_id} is not on a market this adapter serves");

                return;
            };

            match providers::load_instrument(&connection, market, &code).await {
                Ok(instrument) => {
                    instruments.lock().insert(instrument_id, instrument.clone());

                    if let Err(e) = sender.send(DataEvent::Instrument(instrument)) {
                        log::error!("failed to send an instrument event: {e}");
                    }
                }
                Err(e) => log::error!("cannot load {instrument_id}: {e}"),
            }
        });

        Ok(())
    }

    /// Loads every configured market's instruments and emits what it found.
    async fn load_markets(&self, connection: &Connection) -> anyhow::Result<()> {
        for market in self.config.markets.clone() {
            let load: MarketLoad =
                load_market(connection, market, self.config.snapshot_universe).await?;

            log::info!(
                "loaded {} {} instruments, {} of them from the venue's own spread, leaving out {}",
                load.instruments.len(),
                market,
                load.priced,
                load.skipped.len()
            );

            self.store_instruments(load.instruments.clone());
            self.emit_instruments(&load.instruments);
        }

        Ok(())
    }

    /// Stops the things a connection owns, without touching what has been loaded.
    fn release_connection(&mut self) {
        if let Some(manager) = self.subscriptions.take() {
            manager.shutdown();
        }

        if let Some(connection) = self.connection.take() {
            connection.shutdown();
        }

        if let Some(pushes) = self.pushes.take() {
            pushes.abort();
        }

        self.watches.lock().clear();
        self.is_connected.store(false, Ordering::Release);
    }
}

impl Watches {
    fn clear(&mut self) {
        self.trades.clear();
        self.quotes.clear();
        self.books.clear();
        self.bars.clear();
    }
}

/// Returns the market and the gateway's own code for an instrument.
///
/// The venue is the market code, so the symbol is the gateway's code without its market prefix and
/// the two round-trip back to a request with no lookup table.
///
/// # Errors
///
/// Returns an error if the instrument's venue is not a market this adapter serves.
fn market_of(instrument_id: InstrumentId) -> anyhow::Result<(Market, String)> {
    let market = Market::from_code(instrument_id.venue.as_str())
        .with_context(|| format!("{instrument_id} is not on a market this adapter serves"))?;

    Ok((market, instrument_id.symbol.to_string()))
}

/// Returns the gateway's own form of an instrument identifier.
///
/// # Errors
///
/// Returns an error if the market is not one this adapter serves.
fn instrument_of(
    security: &crate::generated::qot_common::Security,
) -> anyhow::Result<(Market, InstrumentId)> {
    let market = Market::from_qot_market(security.market).with_context(|| {
        format!(
            "the gateway named an unsupported market {}",
            security.market
        )
    })?;
    let venue = Venue::new_checked(market.code())?;
    let symbol = Symbol::new_checked(&security.code)?;

    Ok((market, InstrumentId::new(symbol, venue)))
}

/// Returns the date and time a history request is stated with.
///
/// The venue reads a wall clock reading rather than an instant, and it is given one in UTC: the
/// window is compared against session dates the venue states in the market's own zone, so a bound
/// that is off by the market's offset would drop the first or last day of a range.
fn gateway_datetime(timestamp: Timestamp) -> String {
    timestamp.strftime("%Y-%m-%d %H:%M:%S").to_string()
}

/// Returns the window a bar request covers when it names no bounds.
///
/// The venue requires both bounds, so a request that names neither is given one rather than refused.
/// The window is short for an intraday bar and a year for a daily one, because a day of minute bars
/// and a year of daily bars are the same order of data.
fn default_window(interval: Interval, now: Timestamp) -> (Timestamp, Timestamp) {
    let days = if interval.is_intraday() {
        DEFAULT_INTRADAY_WINDOW_DAYS
    } else {
        DEFAULT_DAILY_WINDOW_DAYS
    };

    (
        now - jiff::SignedDuration::from_secs(days * SECONDS_PER_DAY),
        now,
    )
}

/// Sends data to the engine, logging a failure instead of abandoning the path that produced it.
fn send(sender: &EventSender<DataEvent>, data: Data) {
    if let Err(e) = sender.send(DataEvent::Data(data)) {
        log::error!("failed to send a data event: {e}");
    }
}

/// Turns one push into the domain events it carries, for the consumers that asked for them.
///
/// This is the whole decision the push path makes, and it is a function rather than a task so that
/// what a push means can be read without a gateway and a runtime.
fn push_events(
    message: &Message,
    instruments: &IndexMap<InstrumentId, InstrumentAny>,
    watches: &Watches,
    depth: usize,
) -> Vec<Data> {
    match message.proto_id {
        PROTO_ID_UPDATE_TICKER => ticker_events(message, instruments, watches),
        PROTO_ID_UPDATE_ORDER_BOOK => book_events(message, instruments, watches, depth),
        PROTO_ID_UPDATE_KL => kline_events(message, instruments, watches),
        PROTO_ID_NOTIFY => {
            log::debug!(
                "the gateway sent a notification of {} bytes",
                message.body.len()
            );

            Vec::new()
        }
        other => {
            log::debug!("the gateway pushed unsupported protocol {other}");

            Vec::new()
        }
    }
}

/// Turns a ticker push into trade ticks.
fn ticker_events(
    message: &Message,
    instruments: &IndexMap<InstrumentId, InstrumentAny>,
    watches: &Watches,
) -> Vec<Data> {
    let Ok(response) = qot_update_ticker::Response::decode(message.body.as_slice()) else {
        log::warn!("cannot decode a ticker push");

        return Vec::new();
    };

    let Some(s2c) = response.s2c else {
        return Vec::new();
    };

    let Ok((market, instrument_id)) = instrument_of(&s2c.security) else {
        log::debug!("a ticker push named a security this adapter cannot place");

        return Vec::new();
    };

    if !watches.trades.contains(&instrument_id) {
        return Vec::new();
    }

    let Some(instrument) = instruments.get(&instrument_id) else {
        log::debug!("a ticker push arrived for {instrument_id}, which is not loaded");

        return Vec::new();
    };

    let precision = instrument.price_precision();
    let mut events = Vec::with_capacity(s2c.ticker_list.len());

    for ticker in &s2c.ticker_list {
        if !is_reported_trade(ticker) {
            // The venue replays its last known value as the first push of a fresh subscription, and
            // that replay is not a trade.
            log::debug!("ignoring the venue's cached replay for {instrument_id}");

            continue;
        }

        if !has_size(ticker) {
            // The venue pushes a ticker record for changes that are not trades, and one carrying no
            // size is the common case. It is ordinary traffic, and reporting it as a failure would
            // teach an operator to ignore the failures that are not.
            log::debug!("ignoring a ticker record for {instrument_id} that states no size");

            continue;
        }

        match trade_tick_from(ticker, instrument_id, market, precision) {
            Ok(tick) => events.push(Data::Trade(tick)),
            Err(e) => log::error!("cannot map a ticker record for {instrument_id}: {e}"),
        }
    }

    events
}

/// Turns an order book push into a quote tick and, for a book consumer, the book itself.
///
/// The push carries the whole ladder rather than the changes to it, so the same record serves both
/// consumers: a quote is its level one, and the book is the record treated as a snapshot.
fn book_events(
    message: &Message,
    instruments: &IndexMap<InstrumentId, InstrumentAny>,
    watches: &Watches,
    depth: usize,
) -> Vec<Data> {
    let Ok(response) = qot_update_order_book::Response::decode(message.body.as_slice()) else {
        log::warn!("cannot decode an order book push");

        return Vec::new();
    };

    let Some(s2c) = response.s2c else {
        return Vec::new();
    };

    let Ok((_, instrument_id)) = instrument_of(&s2c.security) else {
        log::debug!("an order book push named a security this adapter cannot place");

        return Vec::new();
    };

    let wants_quote = watches.quotes.contains(&instrument_id);
    let wants_book = watches.books.contains(&instrument_id);

    if !wants_quote && !wants_book {
        return Vec::new();
    }

    let Some(instrument) = instruments.get(&instrument_id) else {
        log::debug!("an order book push arrived for {instrument_id}, which is not loaded");

        return Vec::new();
    };

    let ts_init = get_atomic_clock_realtime().get_time_ns();
    let context = BookContext {
        instrument_id,
        price_precision: instrument.price_precision(),
        size_precision: instrument.size_precision(),
        depth,
        ts_init,
    };

    // The venue states a receive time per side and tracks the two separately, so each is resolved
    // on its own and the state is only true once both have arrived.
    let ts_bid = side_time(s2c.svr_recv_time_bid_timestamp, ts_init);
    let ts_ask = side_time(s2c.svr_recv_time_ask_timestamp, ts_init);
    let ts_event = ts_bid.max(ts_ask);

    let mut events = Vec::new();

    if wants_quote {
        match quote_tick_from(
            &s2c.order_book_bid_list,
            &s2c.order_book_ask_list,
            &context,
            ts_event,
        ) {
            Ok(Some(quote)) => events.push(Data::Quote(quote)),
            Ok(None) => log::debug!("the book for {instrument_id} has no two-sided top"),
            Err(e) => log::error!("cannot map the top of book for {instrument_id}: {e}"),
        }
    }

    if wants_book {
        match book_deltas_from(
            &s2c.order_book_bid_list,
            &s2c.order_book_ask_list,
            &context,
            s2c.order_book_type,
            ts_bid,
            ts_ask,
        ) {
            Ok(deltas) => events.push(Data::BookDeltas(Box::new(deltas))),
            Err(e) => log::error!("cannot map the book for {instrument_id}: {e}"),
        }
    }

    events
}

/// Turns a K-line push into bars for the bar type a consumer asked for.
///
/// A push names the security and the interval, so the bar type is looked up rather than rebuilt: a
/// consumer's bar type may name a price other than the last, and rebuilding it would emit a series
/// nobody asked for under the name of one they did.
fn kline_events(
    message: &Message,
    instruments: &IndexMap<InstrumentId, InstrumentAny>,
    watches: &Watches,
) -> Vec<Data> {
    use crate::generated::qot_update_kl;

    let Ok(response) = qot_update_kl::Response::decode(message.body.as_slice()) else {
        log::warn!("cannot decode a K-line push");

        return Vec::new();
    };

    let Some(s2c) = response.s2c else {
        return Vec::new();
    };

    let Some(interval) = Interval::from_kl_type(s2c.kl_type) else {
        log::debug!(
            "the gateway pushed a K-line of unsupported type {}",
            s2c.kl_type
        );

        return Vec::new();
    };

    let Ok((market, instrument_id)) = instrument_of(&s2c.security) else {
        log::debug!("a K-line push named a security this adapter cannot place");

        return Vec::new();
    };

    let Some(bar_type) = watches.bars.get(&(instrument_id, interval)).copied() else {
        return Vec::new();
    };

    let precision = instruments.get(&instrument_id).map_or_else(
        || market.fallback_price_precision(),
        |i| i.price_precision(),
    );

    match build_bars(&s2c.kl_list, bar_type, interval, market, precision) {
        Ok(bars) => bars.into_iter().map(Data::Bar).collect(),
        Err(e) => {
            log::error!("cannot map a K-line push for {instrument_id}: {e}");

            Vec::new()
        }
    }
}

/// Reads the gateway's pushes until the connection that carried them ends.
///
/// The task ends when every sender is gone, which happens when the connection's supervisor stops, so
/// a disconnected client leaves nothing behind.
async fn drain_pushes(
    mut pushes: mpsc::UnboundedReceiver<Message>,
    instruments: Arc<Mutex<IndexMap<InstrumentId, InstrumentAny>>>,
    watches: Arc<Mutex<Watches>>,
    sender: EventSender<DataEvent>,
    depth: usize,
) {
    while let Some(message) = pushes.recv().await {
        let events = {
            let instruments = instruments.lock();
            let watches = watches.lock();

            push_events(&message, &instruments, &watches, depth)
        };

        for data in events {
            send(&sender, data);
        }
    }

    log::debug!("the gateway's push channel closed");
}

#[async_trait(?Send)]
impl DataClient for MoomooDataClient {
    fn client_id(&self) -> ClientId {
        self.client_id
    }

    fn venue(&self) -> Option<Venue> {
        // Multi-venue: an instrument's venue is the market it trades on
        None
    }

    fn start(&mut self) -> anyhow::Result<()> {
        log::info!("Starting {}", self.client_id);

        Ok(())
    }

    fn stop(&mut self) -> anyhow::Result<()> {
        log::info!("Stopping {}", self.client_id);

        self.release_connection();

        Ok(())
    }

    fn reset(&mut self) -> anyhow::Result<()> {
        self.stop()
    }

    fn dispose(&mut self) -> anyhow::Result<()> {
        self.stop()
    }

    fn is_connected(&self) -> bool {
        self.is_connected.load(Ordering::Acquire)
    }

    fn is_disconnected(&self) -> bool {
        !self.is_connected()
    }

    async fn connect(&mut self) -> anyhow::Result<()> {
        if self.is_connected() {
            return Ok(());
        }

        let mut options =
            ConnectOptions::new(self.config.resolved_host(), self.config.resolved_port())
                .with_client_id(CLIENT_ID);

        if let Some(timeout_secs) = self.config.timeout_secs {
            options = options.with_request_timeout(std::time::Duration::from_secs(timeout_secs));
        }

        let (pushes, push_rx) = mpsc::unbounded_channel();
        let connection = Arc::new(connect(&options, pushes).await?);

        // The entitlement is read before anything is asked for, so that a market this login cannot
        // use is known before a strategy depends on one.
        let capabilities = match Capabilities::read(&connection).await {
            Ok(capabilities) => capabilities,
            Err(e) => {
                connection.shutdown();

                return Err(e.into());
            }
        };

        log::info!(
            "the gateway reports level {} for United States equities and level {} for Hong Kong \
             ones, with {} subscriptions available",
            capabilities.us_equity.as_str(),
            capabilities.hk_equity.as_str(),
            capabilities
                .subscription_quota
                .map_or_else(|| "an unreported number of".to_string(), |q| q.to_string()),
        );

        let subscriptions = SubscriptionManager::new(
            Arc::clone(&connection),
            RELEASE_DELAY,
            self.config.session,
            self.config.adjustment,
        );

        if let Err(e) = subscriptions.refresh_allowance().await {
            log::warn!("cannot read the subscription allowance: {e}");
        }

        let depth = self.config.resolved_book_depth();
        self.pushes = Some(get_runtime().spawn(drain_pushes(
            push_rx,
            Arc::clone(&self.instruments),
            Arc::clone(&self.watches),
            self.data_sender.clone(),
            depth,
        )));

        self.connection = Some(Arc::clone(&connection));
        self.subscriptions = Some(subscriptions);
        self.capabilities = Some(capabilities);
        self.is_connected.store(true, Ordering::Release);

        if self.config.load_instruments
            && let Err(e) = self.load_markets(&connection).await
        {
            self.release_connection();

            return Err(e);
        }

        log::info!("Connected: {}", self.client_id);

        Ok(())
    }

    async fn disconnect(&mut self) -> anyhow::Result<()> {
        log::info!("Disconnecting: {}", self.client_id);

        if let Some(connection) = self.connection.take() {
            connection.shutdown();
        }

        if let Some(manager) = self.subscriptions.take() {
            manager.shutdown();
        }

        if let Some(pushes) = self.pushes.take() {
            pushes.abort();
        }

        self.watches.lock().clear();
        self.is_connected.store(false, Ordering::Release);

        Ok(())
    }

    fn subscribe_instruments(&mut self, _cmd: SubscribeInstruments) -> anyhow::Result<()> {
        let instruments = self.instruments();

        if instruments.is_empty() {
            log::warn!(
                "no instruments are loaded to publish for {}",
                self.client_id
            );
        } else {
            self.emit_instruments(&instruments);
        }

        Ok(())
    }

    fn subscribe_instrument(&mut self, cmd: SubscribeInstrument) -> anyhow::Result<()> {
        let instrument_id = cmd.instrument_id;

        match self.instrument(&instrument_id) {
            Some(instrument) => {
                if let Err(e) = self.data_sender.send(DataEvent::Instrument(instrument)) {
                    log::error!("failed to send an instrument event: {e}");
                }

                Ok(())
            }
            // A consumer that named one instrument is answered by loading it rather than by being
            // told to load a market first.
            None if self.is_connected() => self.load_instrument(instrument_id),
            None => bail!("{instrument_id} is not loaded and the client is not connected"),
        }
    }

    fn request_instruments(&self, _request: RequestInstruments) -> anyhow::Result<()> {
        let instruments = self.instruments();

        if instruments.is_empty() {
            log::warn!(
                "no instruments are loaded to respond with for {}",
                self.client_id
            );
        } else {
            self.emit_instruments(&instruments);
        }

        Ok(())
    }

    fn request_instrument(&self, request: RequestInstrument) -> anyhow::Result<()> {
        let instrument_id = request.instrument_id;

        match self.instrument(&instrument_id) {
            Some(instrument) => {
                if let Err(e) = self.data_sender.send(DataEvent::Instrument(instrument)) {
                    log::error!("failed to send an instrument event: {e}");
                }

                Ok(())
            }
            None if self.is_connected() => self.load_instrument(instrument_id),
            None => bail!("{instrument_id} is not loaded and the client is not connected"),
        }
    }

    fn subscribe_bars(&mut self, cmd: SubscribeBars) -> anyhow::Result<()> {
        let bar_type = cmd.bar_type;

        if bar_type.aggregation_source() != AggregationSource::External {
            bail!(
                "the gateway serves externally aggregated bars only, so {bar_type} cannot be subscribed"
            );
        }

        if bar_type.is_composite() {
            bail!(
                "the gateway serves standard bars only, so composite bar type {bar_type} cannot be subscribed"
            );
        }

        let interval = Interval::from_bar_specification(&bar_type.spec())?;
        let instrument_id = bar_type.instrument_id();
        let (market, code) = market_of(instrument_id)?;

        self.watches
            .lock()
            .bars
            .insert((instrument_id, interval), bar_type);

        let subscription = Subscription::new(market, code, SubscriptionType::KLine(interval));

        log::debug!("subscribed to {bar_type} from the gateway's K-line push");

        self.take_subscription(subscription)
    }

    fn unsubscribe_bars(&mut self, cmd: &UnsubscribeBars) -> anyhow::Result<()> {
        let bar_type = cmd.bar_type;
        let interval = Interval::from_bar_specification(&bar_type.spec())?;
        let instrument_id = bar_type.instrument_id();
        let (market, code) = market_of(instrument_id)?;

        if self
            .watches
            .lock()
            .bars
            .remove(&(instrument_id, interval))
            .is_none()
        {
            return Ok(());
        }

        log::debug!("unsubscribed from {bar_type}");

        self.release_subscription(&Subscription::new(
            market,
            code,
            SubscriptionType::KLine(interval),
        ))
    }

    fn request_bars(&self, request: RequestBars) -> anyhow::Result<()> {
        let bar_type = request.bar_type;

        if bar_type.aggregation_source() != AggregationSource::External {
            bail!(
                "the gateway serves externally aggregated bars only, so {bar_type} cannot be requested"
            );
        }

        if bar_type.is_composite() {
            bail!(
                "the gateway serves standard bars only, so composite bar type {bar_type} cannot be requested"
            );
        }

        let interval = Interval::from_bar_specification(&bar_type.spec())?;
        let instrument_id = bar_type.instrument_id();
        let (market, code) = market_of(instrument_id)?;
        let price_precision = self.price_precision(&instrument_id, market);
        let connection = self.connection()?;
        let sender = self.data_sender.clone();
        let config = self.config.clone();

        // The venue requires both bounds and reads them as wall clock readings, so a request that
        // names neither is given a window rather than refused.
        let now = Timestamp::now();
        let (default_start, default_end) = default_window(interval, now);
        let begin = gateway_datetime(request.start.unwrap_or(default_start));
        let end = gateway_datetime(request.end.unwrap_or(default_end));

        get_runtime().spawn(async move {
            let history = HistoryRequest {
                market,
                code,
                interval,
                adjustment: config.adjustment,
                begin,
                end,
                session: config.session,
                page_size: None,
            };

            match providers::request_history_bars(&connection, &history, bar_type, price_precision)
                .await
            {
                Ok(bars) => {
                    log::debug!("requested {} bars for {bar_type}", bars.len());

                    for bar in bars {
                        send(&sender, Data::Bar(bar));
                    }
                }
                Err(e) => log::error!("cannot request bars for {bar_type}: {e}"),
            }
        });

        Ok(())
    }

    fn subscribe_trades(&mut self, cmd: SubscribeTrades) -> anyhow::Result<()> {
        let instrument_id = cmd.instrument_id;

        if !self.config.subscribe_trades {
            bail!(
                "trade subscriptions are disabled by the client's configuration, so \
                 {instrument_id} cannot be subscribed"
            );
        }

        let (market, code) = market_of(instrument_id)?;

        self.watches.lock().trades.insert(instrument_id);

        log::debug!("subscribed to trades for {instrument_id}");

        self.take_subscription(Subscription::new(market, code, SubscriptionType::Ticker))
    }

    fn unsubscribe_trades(&mut self, cmd: &UnsubscribeTrades) -> anyhow::Result<()> {
        let instrument_id = cmd.instrument_id;

        if !self.watches.lock().trades.remove(&instrument_id) {
            return Ok(());
        }

        let (market, code) = market_of(instrument_id)?;

        log::debug!("unsubscribed from trades for {instrument_id}");

        self.release_subscription(&Subscription::new(market, code, SubscriptionType::Ticker))
    }

    fn subscribe_quotes(&mut self, cmd: SubscribeQuotes) -> anyhow::Result<()> {
        self.subscribe_book(cmd.instrument_id, true)
    }

    fn unsubscribe_quotes(&mut self, cmd: &UnsubscribeQuotes) -> anyhow::Result<()> {
        self.unsubscribe_book(cmd.instrument_id, true)
    }

    fn subscribe_book_deltas(&mut self, cmd: SubscribeBookDeltas) -> anyhow::Result<()> {
        self.subscribe_book(cmd.instrument_id, false)
    }

    fn unsubscribe_book_deltas(&mut self, cmd: &UnsubscribeBookDeltas) -> anyhow::Result<()> {
        self.unsubscribe_book(cmd.instrument_id, false)
    }
}

impl MoomooDataClient {
    /// Takes the book subscription that serves quotes, book deltas, or both.
    ///
    /// The venue has one subscription for the two, so they share it: whichever a consumer asks for
    /// first takes it, and the second joins the same subscription rather than taking another, which
    /// is what the manager's reference counting is for. It is released only when both have gone.
    fn subscribe_book(&self, instrument_id: InstrumentId, quotes: bool) -> anyhow::Result<()> {
        if !self.config.subscribe_quotes {
            bail!(
                "quote and book subscriptions are disabled by the client's configuration, so \
                 {instrument_id} cannot be subscribed"
            );
        }

        let (market, code) = market_of(instrument_id)?;

        {
            let mut watches = self.watches.lock();

            if quotes {
                watches.quotes.insert(instrument_id);
            } else {
                watches.books.insert(instrument_id);
            }
        }

        log::debug!(
            "subscribed to {} for {instrument_id}",
            if quotes { "quotes" } else { "book deltas" }
        );

        self.take_subscription(Subscription::new(market, code, SubscriptionType::OrderBook))
    }

    /// Gives up a consumer's interest in the book subscription.
    fn unsubscribe_book(&self, instrument_id: InstrumentId, quotes: bool) -> anyhow::Result<()> {
        let had_interest = {
            let mut watches = self.watches.lock();

            if quotes {
                watches.quotes.remove(&instrument_id)
            } else {
                watches.books.remove(&instrument_id)
            }
        };

        if !had_interest {
            return Ok(());
        }

        let (market, code) = market_of(instrument_id)?;

        log::debug!(
            "unsubscribed from {} for {instrument_id}",
            if quotes { "quotes" } else { "book deltas" }
        );

        self.release_subscription(&Subscription::new(
            market,
            code,
            SubscriptionType::OrderBook,
        ))
    }
}

#[cfg(test)]
mod tests {
    use nautilus_core::UnixNanos;
    use nautilus_model::{
        data::BarSpecification,
        enums::{BarAggregation, PriceType},
        types::Price,
    };
    use rstest::rstest;

    use super::*;
    use crate::{
        generated::qot_common::{Security, SecurityStaticBasic, SecurityStaticInfo},
        mappers::instrument::{SECURITY_TYPE_EQUITY, instrument_from},
    };

    fn instrument_id() -> InstrumentId {
        InstrumentId::from("AAPL.US")
    }

    fn security(code: &str) -> Security {
        Security {
            market: Market::Us.qot_market(),
            code: code.to_string(),
        }
    }

    fn static_info() -> SecurityStaticInfo {
        SecurityStaticInfo {
            basic: SecurityStaticBasic {
                security: security("AAPL"),
                id: 1,
                lot_size: 1,
                sec_type: SECURITY_TYPE_EQUITY,
                name: "APPLE INC".to_string(),
                list_time: "1980-12-12".to_string(),
                delisting: None,
                list_timestamp: None,
                exch_type: Some(1),
            },
            ..SecurityStaticInfo::default()
        }
    }

    /// The instrument the pushes are read against.
    ///
    /// It is built by the adapter's own mapper rather than by hand, so that what the push path is
    /// tested against is what the adapter actually serves, precisions included.
    fn equity() -> InstrumentAny {
        instrument_from(&static_info(), None).unwrap()
    }

    fn instruments() -> IndexMap<InstrumentId, InstrumentAny> {
        let mut map = IndexMap::new();
        map.insert(instrument_id(), equity());

        map
    }

    /// A book push, with or without the venue's own receive time.
    fn book_push(venue_time: bool) -> Message {
        use crate::generated::{
            qot_common::OrderBook,
            qot_update_order_book::{Response, S2c},
        };

        // The venue's own field name is misspelled in its schema, and prost keeps the schema's
        // spelling rather than correcting it behind a caller's back.
        let level = |price: f64, volume: i64| OrderBook {
            price,
            volume,
            oreder_count: 1,
            detail_list: Vec::new(),
            hp_volume: None,
        };

        let body = Response {
            ret_type: 0,
            ret_msg: None,
            err_code: None,
            s2c: Some(S2c {
                security: security("AAPL"),
                name: None,
                order_book_ask_list: vec![level(100.02, 5)],
                order_book_bid_list: vec![level(100.01, 7)],
                svr_recv_time_bid: None,
                svr_recv_time_bid_timestamp: venue_time.then_some(1_700_000_000.5),
                svr_recv_time_ask: None,
                svr_recv_time_ask_timestamp: venue_time.then_some(1_700_000_000.5),
                order_book_type: Some(crate::mappers::book::BOARD_NORMAL),
            }),
        };

        Message {
            proto_id: PROTO_ID_UPDATE_ORDER_BOOK,
            serial_no: 0,
            body: body.encode_to_vec(),
        }
    }

    #[rstest]
    fn test_market_of_reads_the_market_from_the_venue() {
        let (market, code) = market_of(instrument_id()).unwrap();

        assert_eq!(market, Market::Us);
        assert_eq!(code, "AAPL");
    }

    #[rstest]
    fn test_market_of_refuses_a_venue_the_adapter_does_not_serve() {
        let result = market_of(InstrumentId::from("BTCUSDT.BINANCE"));

        assert!(result.is_err());
    }

    #[rstest]
    fn test_gateway_datetime_writes_the_wall_clock_the_venue_reads() {
        let timestamp: Timestamp = "2024-01-02T03:04:05Z".parse().unwrap();

        assert_eq!(gateway_datetime(timestamp), "2024-01-02 03:04:05");
    }

    #[rstest]
    fn test_default_window_is_shorter_for_an_intraday_bar() {
        let now: Timestamp = "2024-01-10T00:00:00Z".parse().unwrap();

        let (begin, end) = default_window(Interval::Minute1, now);
        assert_eq!(end, now);
        assert_eq!(begin, "2024-01-05T00:00:00Z".parse::<Timestamp>().unwrap());

        let (begin, end) = default_window(Interval::Day, now);
        assert_eq!(end, now);
        assert_eq!(begin, "2023-01-10T00:00:00Z".parse::<Timestamp>().unwrap());
    }

    #[rstest]
    fn test_a_book_push_produces_a_quote_for_a_quote_consumer() {
        let watches = Watches {
            quotes: HashSet::from([instrument_id()]),
            ..Watches::default()
        };

        let events = push_events(&book_push(true), &instruments(), &watches, 10);

        assert_eq!(events.len(), 1);

        let Data::Quote(quote) = &events[0] else {
            panic!("expected a quote, was {:?}", events[0]);
        };

        assert_eq!(quote.instrument_id, instrument_id());
        assert_eq!(quote.bid_price, Price::new(100.01, 2));
        assert_eq!(quote.ask_price, Price::new(100.02, 2));
    }

    #[rstest]
    fn test_a_book_push_produces_deltas_for_a_book_consumer_only() {
        let watches = Watches {
            books: HashSet::from([instrument_id()]),
            ..Watches::default()
        };

        let events = push_events(&book_push(true), &instruments(), &watches, 10);

        assert_eq!(events.len(), 1);

        let Data::BookDeltas(deltas) = &events[0] else {
            panic!("expected book deltas, was {:?}", events[0]);
        };

        assert_eq!(deltas.instrument_id, instrument_id());
    }

    #[rstest]
    fn test_a_book_push_produces_both_for_a_consumer_that_asked_for_both() {
        let watches = Watches {
            quotes: HashSet::from([instrument_id()]),
            books: HashSet::from([instrument_id()]),
            ..Watches::default()
        };

        let events = push_events(&book_push(true), &instruments(), &watches, 10);

        assert_eq!(events.len(), 2);
        assert!(matches!(events[0], Data::Quote(_)));
        assert!(matches!(events[1], Data::BookDeltas(_)));
    }

    #[rstest]
    fn test_a_book_push_nobody_asked_for_produces_nothing() {
        let events = push_events(&book_push(true), &instruments(), &Watches::default(), 10);

        assert!(events.is_empty());
    }

    #[rstest]
    fn test_a_book_push_for_an_unloaded_instrument_produces_nothing() {
        let watches = Watches {
            quotes: HashSet::from([instrument_id()]),
            ..Watches::default()
        };

        let events = push_events(&book_push(true), &IndexMap::new(), &watches, 10);

        assert!(events.is_empty());
    }

    #[rstest]
    fn test_a_book_push_without_a_venue_time_is_stamped_locally() {
        // The first push of a subscription carries a receive time of zero for both sides, which the
        // venue's protocol says means it has no instant to report. Zero is therefore not the epoch,
        // and a tick stamped at the epoch would be an event in 1970.
        let watches = Watches {
            quotes: HashSet::from([instrument_id()]),
            ..Watches::default()
        };

        let events = push_events(&book_push(false), &instruments(), &watches, 10);

        assert_eq!(events.len(), 1);

        let Data::Quote(quote) = &events[0] else {
            panic!("expected a quote, was {:?}", events[0]);
        };

        assert!(quote.ts_event > UnixNanos::default());
    }

    #[rstest]
    fn test_a_book_push_carries_the_venue_time_when_it_has_one() {
        let watches = Watches {
            quotes: HashSet::from([instrument_id()]),
            ..Watches::default()
        };

        let events = push_events(&book_push(true), &instruments(), &watches, 10);

        let Data::Quote(quote) = &events[0] else {
            panic!("expected a quote, was {:?}", events[0]);
        };

        // 1_700_000_000.5 seconds, with the fraction placed rather than scaled.
        assert_eq!(
            quote.ts_event,
            UnixNanos::from(1_700_000_000_500_000_000_u64)
        );
    }

    #[rstest]
    fn test_the_book_depth_is_applied_to_the_push() {
        let watches = Watches {
            books: HashSet::from([instrument_id()]),
            ..Watches::default()
        };

        let events = push_events(&book_push(true), &instruments(), &watches, 1);

        let Data::BookDeltas(deltas) = &events[0] else {
            panic!("expected book deltas, was {:?}", events[0]);
        };

        // A clear and one level a side: the depth is applied to a push that carried the venue's
        // whole ladder, and the last flag rides on the final level rather than on a record of its
        // own.
        assert_eq!(deltas.deltas.len(), 3, "a clear and one level a side");
        assert_eq!(
            deltas.deltas[0].action,
            nautilus_model::enums::BookAction::Clear
        );
        assert_ne!(
            deltas.deltas.last().unwrap().flags & nautilus_model::enums::RecordFlag::F_LAST as u8,
            0
        );
    }

    #[rstest]
    fn test_a_ticker_push_produces_a_trade_for_a_trade_consumer() {
        use crate::generated::{
            qot_common::{Security, Ticker},
            qot_update_ticker::{Response, S2c},
        };

        let watches = Watches {
            trades: HashSet::from([instrument_id()]),
            ..Watches::default()
        };

        let body = Response {
            ret_type: 0,
            ret_msg: None,
            err_code: None,
            s2c: Some(S2c {
                security: Security {
                    market: Market::Us.qot_market(),
                    code: "AAPL".to_string(),
                },
                name: None,
                ticker_list: vec![Ticker {
                    time: "2024-01-02 03:04:05.678".to_string(),
                    sequence: 7,
                    dir: crate::mappers::trades::DIRECTION_ASK,
                    price: 100.5,
                    volume: 100,
                    turnover: 10_050.0,
                    // Anything but the cached marker: a cached record is a replay and not a trade.
                    push_data_type: Some(1),
                    ..Ticker::default()
                }],
            }),
        };

        let message = Message {
            proto_id: PROTO_ID_UPDATE_TICKER,
            serial_no: 0,
            body: body.encode_to_vec(),
        };

        let events = push_events(&message, &instruments(), &watches, 10);

        assert_eq!(events.len(), 1);

        let Data::Trade(trade) = &events[0] else {
            panic!("expected a trade, was {:?}", events[0]);
        };

        assert_eq!(trade.instrument_id, instrument_id());
        assert_eq!(trade.price, Price::new(100.5, 2));
    }

    #[rstest]
    fn test_a_ticker_push_nobody_asked_for_produces_nothing() {
        use crate::generated::{
            qot_common::{Security, Ticker},
            qot_update_ticker::{Response, S2c},
        };

        let body = Response {
            ret_type: 0,
            ret_msg: None,
            err_code: None,
            s2c: Some(S2c {
                security: Security {
                    market: Market::Us.qot_market(),
                    code: "AAPL".to_string(),
                },
                name: None,
                ticker_list: vec![Ticker::default()],
            }),
        };

        let message = Message {
            proto_id: PROTO_ID_UPDATE_TICKER,
            serial_no: 0,
            body: body.encode_to_vec(),
        };

        assert!(push_events(&message, &instruments(), &Watches::default(), 10).is_empty());
    }

    #[rstest]
    fn test_a_ticker_push_with_no_size_produces_nothing() {
        // The venue pushes a ticker record for changes that are not trades, and a record stating no
        // size is the common one. Observed live, twice in one session, and it is ordinary traffic:
        // the trade path must ignore it rather than report a failure for it.
        use crate::generated::{
            qot_common::Ticker,
            qot_update_ticker::{Response, S2c},
        };

        let watches = Watches {
            trades: HashSet::from([instrument_id()]),
            ..Watches::default()
        };

        let body = Response {
            ret_type: 0,
            ret_msg: None,
            err_code: None,
            s2c: Some(S2c {
                security: security("AAPL"),
                name: None,
                ticker_list: vec![Ticker {
                    time: "2024-01-02 03:04:05.678".to_string(),
                    sequence: 8,
                    dir: crate::mappers::trades::DIRECTION_ASK,
                    price: 100.5,
                    volume: 0,
                    turnover: 0.0,
                    push_data_type: Some(1),
                    ..Ticker::default()
                }],
            }),
        };

        let message = Message {
            proto_id: PROTO_ID_UPDATE_TICKER,
            serial_no: 0,
            body: body.encode_to_vec(),
        };

        assert!(push_events(&message, &instruments(), &watches, 10).is_empty());
    }

    #[rstest]
    fn test_a_kline_push_produces_bars_for_the_consumers_bar_type() {
        use crate::generated::{
            qot_common::{KLine, Security},
            qot_update_kl::{Response, S2c},
        };

        let row = KLine {
            time: "2024-01-02".to_string(),
            is_blank: false,
            open_price: Some(100.0),
            high_price: Some(102.0),
            low_price: Some(99.0),
            close_price: Some(101.0),
            volume: Some(1_000),
            ..KLine::default()
        };

        let body = Response {
            ret_type: 0,
            ret_msg: None,
            err_code: None,
            s2c: Some(S2c {
                rehab_type: 0,
                kl_type: Interval::Day.kl_type(),
                security: Security {
                    market: Market::Us.qot_market(),
                    code: "AAPL".to_string(),
                },
                name: None,
                kl_list: vec![row],
            }),
        };

        let message = Message {
            proto_id: PROTO_ID_UPDATE_KL,
            serial_no: 0,
            body: body.encode_to_vec(),
        };

        let bar_type = BarType::new(
            instrument_id(),
            BarSpecification::new(1, BarAggregation::Day, PriceType::Last),
            AggregationSource::External,
        );

        let watches = Watches {
            bars: HashMap::from([((instrument_id(), Interval::Day), bar_type)]),
            ..Watches::default()
        };

        let events = push_events(&message, &instruments(), &watches, 10);

        assert_eq!(events.len(), 1);

        let Data::Bar(bar) = &events[0] else {
            panic!("expected a bar, was {:?}", events[0]);
        };

        assert_eq!(bar.bar_type, bar_type);
        assert_eq!(bar.close, Price::new(101.0, 2));
    }

    #[rstest]
    fn test_a_kline_push_nobody_asked_for_produces_nothing() {
        use crate::generated::{
            qot_common::Security,
            qot_update_kl::{Response, S2c},
        };

        let body = Response {
            ret_type: 0,
            ret_msg: None,
            err_code: None,
            s2c: Some(S2c {
                rehab_type: 0,
                kl_type: Interval::Day.kl_type(),
                security: Security {
                    market: Market::Us.qot_market(),
                    code: "AAPL".to_string(),
                },
                name: None,
                kl_list: Vec::new(),
            }),
        };

        let message = Message {
            proto_id: PROTO_ID_UPDATE_KL,
            serial_no: 0,
            body: body.encode_to_vec(),
        };

        assert!(push_events(&message, &instruments(), &Watches::default(), 10).is_empty());
    }

    #[rstest]
    fn test_an_unreadable_push_does_not_produce_events() {
        let message = Message {
            proto_id: PROTO_ID_UPDATE_TICKER,
            serial_no: 0,
            body: vec![0xff, 0xff, 0xff],
        };

        assert!(push_events(&message, &instruments(), &Watches::default(), 10).is_empty());
    }

    #[rstest]
    fn test_a_notification_is_not_data() {
        let message = Message {
            proto_id: PROTO_ID_NOTIFY,
            serial_no: 0,
            body: Vec::new(),
        };

        assert!(push_events(&message, &instruments(), &Watches::default(), 10).is_empty());
    }

    #[rstest]
    fn test_an_unsupported_push_is_not_data() {
        let message = Message {
            proto_id: 3005,
            serial_no: 0,
            body: Vec::new(),
        };

        assert!(push_events(&message, &instruments(), &Watches::default(), 10).is_empty());
    }
}
