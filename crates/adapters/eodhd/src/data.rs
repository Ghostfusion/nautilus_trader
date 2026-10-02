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

//! Live data client for EODHD.
//!
//! EODHD offers no HTTP push channel for the entitlements this adapter targets, so the client
//! streams by polling. Each bar or quote subscription owns a task that requests a bounded window,
//! emits what is new or revised, and then waits for its next tick. A shortened window is derived
//! from the last emitted timestamp so a long-running poll does not re-transfer its history.

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::Duration,
};

use async_trait::async_trait;
use indexmap::IndexMap;
use jiff::Timestamp;
use nautilus_common::{
    clients::DataClient,
    live::{runner::get_data_event_sender, sender::EventSender},
    messages::{
        DataEvent,
        data::{
            BarsResponse, DataResponse, InstrumentResponse, InstrumentsResponse, RequestBars,
            RequestInstrument, RequestInstruments, SubscribeBars, SubscribeInstrument,
            SubscribeInstruments, SubscribeQuotes, SubscribeTrades, UnsubscribeBars,
            UnsubscribeQuotes, UnsubscribeTrades,
        },
    },
};
use nautilus_core::{UnixNanos, string::secret::SecretString, time::get_atomic_clock_realtime};
use nautilus_live::task::TaskGroup;
use nautilus_model::{
    data::{Bar, BarType, CorporateAction, Data, QuoteTick, TradeTick},
    enums::{AggregationSource, AggressorSide},
    identifiers::{ClientId, InstrumentId, Symbol, TradeId, Venue},
    instruments::{Instrument, InstrumentAny},
    types::{Price, Quantity},
};
use tokio_util::sync::CancellationToken;

use crate::{
    bars::{EodhdInterval, build_bulk_bar, build_eod_bars, build_intraday_bars, resolve_interval},
    common::{
        EODHD_DEFAULT_EXCHANGE, EODHD_WS_QUOTES_CHANNEL, EODHD_WS_TRADES_CHANNEL, EODHD_WS_VENUE,
    },
    config::EodhdDataClientConfig,
    corporate_actions::{action_from_dividend, action_from_split},
    http::{EodhdDelayedQuote, EodhdHttpClient},
    providers::EodhdInstrumentProvider,
    websocket::{
        EodhdQuoteMessage, EodhdTradeMessage, EodhdWebSocketClient, EodhdWsMessage, StreamCommand,
    },
};

/// Seconds in a day.
const SECONDS_PER_DAY: i64 = 86_400;

/// The size precision applied to EODHD share quantities.
const SIZE_PRECISION: u8 = 0;

/// The longest shutdown wait for a task generation to finish, in seconds.
const SHUTDOWN_GRACE_SECS: u64 = 2;

/// The shortest request window, in seconds, that still catches a bar revision.
const MIN_WINDOW_SECS: i64 = 600;

/// A live data client for EODHD.
///
/// The client is multi-venue: instruments carry their EODHD exchange code as the Nautilus venue,
/// so [`DataClient::venue`] returns `None`. Register the client under a client ID and address
/// subscriptions to that ID when a node runs more than one data client.
#[derive(Debug)]
pub struct EodhdDataClient {
    client_id: ClientId,
    config: EodhdDataClientConfig,
    http_client: EodhdHttpClient,
    provider: EodhdInstrumentProvider,
    is_connected: Arc<AtomicBool>,
    cancellation_token: CancellationToken,
    tasks: TaskGroup,
    data_sender: EventSender<DataEvent>,
    bar_subscriptions: Rc<RefCell<HashMap<BarType, CancellationToken>>>,
    quote_subscriptions: Rc<RefCell<HashMap<InstrumentId, CancellationToken>>>,
    bulk_polls: Rc<RefCell<HashMap<String, tokio::sync::mpsc::UnboundedSender<BulkWatch>>>>,
    trades_stream: Option<EodhdWebSocketClient>,
    quotes_stream: Option<EodhdWebSocketClient>,
    trade_subscriptions: Rc<RefCell<HashSet<InstrumentId>>>,
    stream_sequence: Arc<AtomicU64>,
    instruments: Rc<RefCell<IndexMap<InstrumentId, InstrumentAny>>>,
}

impl EodhdDataClient {
    /// Creates a new [`EodhdDataClient`] instance.
    ///
    /// # Errors
    ///
    /// Returns an error if no API token is available, the HTTP client cannot be built, or the
    /// data event sender is not initialized.
    pub fn new(client_id: ClientId, config: EodhdDataClientConfig) -> anyhow::Result<Self> {
        let http_client = EodhdHttpClient::new(
            config.api_key.as_ref().map(SecretString::expose_secret),
            config
                .http_base_url
                .as_ref()
                .map(SecretString::expose_secret),
            config.timeout_secs,
            config
                .proxy_url
                .as_ref()
                .map(|url| url.expose_secret().to_string()),
        )?;

        let provider = EodhdInstrumentProvider::new(
            http_client.clone(),
            config.resolved_currency(),
            config.price_precision,
        );
        let data_sender = get_data_event_sender();
        let tasks = TaskGroup::new();
        let cancellation_token = tasks.cancellation_token();

        Ok(Self {
            client_id,
            config,
            http_client,
            provider,
            is_connected: Arc::new(AtomicBool::new(false)),
            cancellation_token,
            tasks,
            data_sender,
            bar_subscriptions: Rc::new(RefCell::new(HashMap::new())),
            quote_subscriptions: Rc::new(RefCell::new(HashMap::new())),
            bulk_polls: Rc::new(RefCell::new(HashMap::new())),
            trades_stream: None,
            quotes_stream: None,
            trade_subscriptions: Rc::new(RefCell::new(HashSet::new())),
            stream_sequence: Arc::new(AtomicU64::new(0)),
            instruments: Rc::new(RefCell::new(IndexMap::new())),
        })
    }

    /// Returns the HTTP client used by this data client.
    #[must_use]
    pub fn http_client(&self) -> &EodhdHttpClient {
        &self.http_client
    }

    /// Returns the instruments currently held by the client, in discovery order.
    #[must_use]
    pub fn instruments(&self) -> Vec<InstrumentAny> {
        self.instruments.borrow().values().cloned().collect()
    }

    fn store_instruments(&self, instruments: Vec<InstrumentAny>) {
        for instrument in instruments {
            self.instruments
                .borrow_mut()
                .insert(instrument.id(), instrument);
        }
    }

    fn emit_instruments(&self, instruments: &[InstrumentAny]) {
        for instrument in instruments {
            if let Err(e) = self
                .data_sender
                .send(DataEvent::Instrument(instrument.clone()))
            {
                log::error!("Failed to send instrument event: {e}");
            }
        }
    }

    fn spawn_bar_poll(
        &self,
        bar_type: BarType,
        interval: EodhdInterval,
        token: CancellationToken,
    ) -> anyhow::Result<()> {
        let http_client = self.http_client.clone();
        let sender = self.data_sender.clone();
        let price_precision = self.config.price_precision;
        let poll_interval_secs = self.config.resolved_poll_interval_secs();
        let backfill_days = i64::from(self.config.backfill_days);
        let window_secs = (backfill_days * SECONDS_PER_DAY).max(MIN_WINDOW_SECS);
        let load_corporate_actions = self.config.load_corporate_actions;

        self.tasks.spawn(async move {
            let mut emitter = BarEmitter::new();
            let mut seeded = false;
            let mut poll_ticks = tokio::time::interval(Duration::from_secs(poll_interval_secs));
            let mut from = now_seconds() - window_secs;

            loop {
                tokio::select! {
                    biased;
                    () = token.cancelled() => {
                        log::debug!("Bar polling cancelled for {bar_type}");
                        break;
                    }
                    _ = poll_ticks.tick() => {}
                }

                let now = now_seconds();

                match fetch_bars(&http_client, bar_type, interval, from, now, price_precision).await
                {
                    Ok(bars) => {
                        for bar in bars {
                            if let Some(bar) = emitter.consider(bar) {
                                send_bar(&sender, bar);
                            }
                        }
                    }
                    Err(e) => log::error!("Failed to poll bars for {bar_type}: {e}"),
                }

                // The window of the first poll is the subscription's window, so its actions are
                // emitted once rather than on every tick.
                if !seeded {
                    seeded = true;

                    if load_corporate_actions {
                        emit_corporate_actions(
                            &http_client,
                            &sender,
                            bar_type.instrument_id(),
                            from,
                            now,
                        )
                        .await;
                    }
                }

                // Re-request from just before the newest bar so a revision is still observed.
                from = (emitter.last_ts_seconds() - MIN_WINDOW_SECS).max(from);
            }
        })?;

        Ok(())
    }

    fn spawn_quote_poll(
        &self,
        instrument_id: InstrumentId,
        token: CancellationToken,
    ) -> anyhow::Result<()> {
        let http_client = self.http_client.clone();
        let sender = self.data_sender.clone();
        let price_precision = self.config.price_precision;
        let poll_interval_secs = self.config.resolved_poll_interval_secs();

        self.tasks.spawn(async move {
            let mut emitted: Option<EodhdDelayedQuote> = None;
            let mut poll_ticks = tokio::time::interval(Duration::from_secs(poll_interval_secs));
            let ticker = instrument_id.to_string();

            loop {
                tokio::select! {
                    biased;
                    () = token.cancelled() => {
                        log::debug!("Quote polling cancelled for {instrument_id}");
                        break;
                    }
                    _ = poll_ticks.tick() => {}
                }

                match http_client.delayed_quote(&ticker).await {
                    Ok(quote) => {
                        if emitted.as_ref() == Some(&quote) {
                            continue;
                        }

                        let ts_event = quote.ts_event();

                        match build_quote(
                            instrument_id,
                            &quote,
                            price_precision,
                            ts_event,
                            ts_event,
                        ) {
                            Ok(tick) => {
                                send_quote(&sender, tick);
                                emitted = Some(quote);
                            }
                            Err(e) => {
                                log::error!("Failed to build quote for {instrument_id}: {e}");
                            }
                        }
                    }
                    Err(e) => log::error!("Failed to poll quote for {instrument_id}: {e}"),
                }
            }
        })?;

        Ok(())
    }

    /// Connects the streaming channels and starts emitting their frames.
    ///
    /// # Errors
    ///
    /// Returns an error if a channel cannot be connected or a consumer cannot be started.
    async fn connect_streams(&mut self) -> anyhow::Result<()> {
        let base_url = self.config.resolved_ws_base_url();
        let api_key = self.http_client.api_key().to_string();
        let proxy_url = self
            .config
            .proxy_url
            .as_ref()
            .map(|url| url.expose_secret().to_string());

        let (trades, trades_rx) = EodhdWebSocketClient::connect(
            &base_url,
            EODHD_WS_TRADES_CHANNEL,
            &api_key,
            proxy_url.clone(),
        )
        .await?;
        let (quotes, quotes_rx) =
            EodhdWebSocketClient::connect(&base_url, EODHD_WS_QUOTES_CHANNEL, &api_key, proxy_url)
                .await?;

        let price_precision = self.config.price_precision;
        let token = self.cancellation_token.child_token();

        self.spawn_stream_consumer("trades", trades_rx, price_precision, token.clone())?;
        self.spawn_stream_consumer("quotes", quotes_rx, price_precision, token)?;

        self.trades_stream = Some(trades);
        self.quotes_stream = Some(quotes);

        log::info!("Subscribed to the EODHD trades and quotes channels");

        Ok(())
    }

    /// Spawns the task that emits the frames of a streaming channel.
    ///
    /// # Errors
    ///
    /// Returns an error if the task generation is no longer accepting tasks.
    fn spawn_stream_consumer(
        &self,
        channel: &'static str,
        mut receiver: tokio::sync::mpsc::UnboundedReceiver<EodhdWsMessage>,
        price_precision: u8,
        token: CancellationToken,
    ) -> anyhow::Result<()> {
        let sender = self.data_sender.clone();
        let sequence = Arc::clone(&self.stream_sequence);

        self.tasks.spawn(async move {
            loop {
                tokio::select! {
                    biased;
                    () = token.cancelled() => break,
                    Some(message) = receiver.recv() => {
                        emit_stream_message(&sender, channel, message, price_precision, &sequence);
                    }
                }
            }

            log::debug!("The EODHD {channel} consumer has stopped");
        })?;

        Ok(())
    }

    /// Returns the command sender for `exchange`, starting its poll when it is not running.
    ///
    /// # Errors
    ///
    /// Returns an error if the task generation is no longer accepting tasks.
    fn ensure_bulk_poll(
        &self,
        exchange: &str,
    ) -> anyhow::Result<tokio::sync::mpsc::UnboundedSender<BulkWatch>> {
        if let Some(sender) = self.bulk_polls.borrow().get(exchange) {
            return Ok(sender.clone());
        }

        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel();
        let token = self.cancellation_token.child_token();

        self.spawn_bulk_poll(exchange.to_string(), receiver, token)?;
        self.bulk_polls
            .borrow_mut()
            .insert(exchange.to_string(), sender.clone());

        log::debug!("Started the EODHD bulk last-day poll for {exchange}");

        Ok(sender)
    }

    /// Spawns the poll that serves every daily bar subscription on `exchange`.
    ///
    /// # Errors
    ///
    /// Returns an error if the task generation is no longer accepting tasks.
    fn spawn_bulk_poll(
        &self,
        exchange: String,
        mut receiver: tokio::sync::mpsc::UnboundedReceiver<BulkWatch>,
        token: CancellationToken,
    ) -> anyhow::Result<()> {
        let http_client = self.http_client.clone();
        let sender = self.data_sender.clone();
        let price_precision = self.config.price_precision;
        let poll_interval_secs = self.config.resolved_poll_interval_secs();
        let backfill_days = i64::from(self.config.backfill_days);
        let window_secs = (backfill_days * SECONDS_PER_DAY).max(MIN_WINDOW_SECS);
        let load_corporate_actions = self.config.load_corporate_actions;

        self.tasks.spawn(async move {
            let mut watches: HashMap<String, BulkWatchState> = HashMap::new();
            let mut poll_ticks = tokio::time::interval(Duration::from_secs(poll_interval_secs));

            loop {
                tokio::select! {
                    biased;
                    () = token.cancelled() => {
                        log::debug!("Bulk polling cancelled for {exchange}");
                        break;
                    }
                    Some(watch) = receiver.recv() => {
                        let ticker = watch.bar_type.instrument_id().to_string();
                        let state = register_bulk_watch(
                            &http_client,
                            &sender,
                            watch,
                            backfill_days,
                            window_secs,
                            price_precision,
                            load_corporate_actions,
                        )
                        .await;

                        watches.insert(ticker, state);
                    }
                    _ = poll_ticks.tick() => {}
                }

                // A cancelled watch is an unsubscribed bar type, and an exchange nobody watches
                // costs no request at all.
                watches.retain(|_, state| !state.token.is_cancelled());

                if watches.is_empty() {
                    continue;
                }

                poll_bulk(
                    &http_client,
                    &sender,
                    &exchange,
                    &mut watches,
                    price_precision,
                )
                .await;
            }
        })?;

        Ok(())
    }
}

/// Returns whether `exchange` is among the configured `bulk_exchanges` codes.
///
/// The comparison ignores case because an EODHD exchange code reaches the client as the venue of
/// an instrument ID, which is upper-cased in practice but not by contract.
fn matches_bulk_exchange(bulk_exchanges: &[String], exchange: &str) -> bool {
    bulk_exchanges
        .iter()
        .any(|candidate| candidate.eq_ignore_ascii_case(exchange))
}

/// A daily bar subscription served by a bulk poll task.
#[derive(Debug)]
struct BulkWatch {
    bar_type: BarType,
    token: CancellationToken,
}

/// The state a bulk poll task keeps for one watched instrument.
#[derive(Debug)]
struct BulkWatchState {
    bar_type: BarType,
    token: CancellationToken,
    emitter: BarEmitter,
}

/// Registers a bulk watch, seeding its history before the first bulk poll.
///
/// The symbol's own window is fetched once from the end-of-day endpoint, so a bulk subscription
/// starts with the history a per-symbol subscription would have and only its ongoing polls are
/// shared with the rest of the exchange.
async fn register_bulk_watch(
    http_client: &EodhdHttpClient,
    sender: &EventSender<DataEvent>,
    watch: BulkWatch,
    backfill_days: i64,
    window_secs: i64,
    price_precision: u8,
    load_corporate_actions: bool,
) -> BulkWatchState {
    let mut emitter = BarEmitter::new();
    let now = now_seconds();

    if load_corporate_actions {
        emit_corporate_actions(
            http_client,
            sender,
            watch.bar_type.instrument_id(),
            now - window_secs,
            now,
        )
        .await;
    }

    if backfill_days > 0 {
        match fetch_bars(
            http_client,
            watch.bar_type,
            EodhdInterval::Day,
            now - window_secs,
            now,
            price_precision,
        )
        .await
        {
            Ok(bars) => {
                for bar in bars {
                    if let Some(bar) = emitter.consider(bar) {
                        send_bar(sender, bar);
                    }
                }
            }
            Err(e) => log::error!("Failed to seed history for {}: {e}", watch.bar_type),
        }
    }

    BulkWatchState {
        bar_type: watch.bar_type,
        token: watch.token,
        emitter,
    }
}

/// Requests the last day for `exchange` and emits a bar for every watched instrument.
///
/// The endpoint offers no filter for a subset of symbols, so the whole exchange is fetched and
/// the rows a subscription asked for are selected from it.
async fn poll_bulk(
    http_client: &EodhdHttpClient,
    sender: &EventSender<DataEvent>,
    exchange: &str,
    watches: &mut HashMap<String, BulkWatchState>,
    price_precision: u8,
) {
    let codes: HashSet<String> = watches
        .keys()
        .filter_map(|ticker| ticker.rsplit_once('.').map(|(code, _)| code.to_string()))
        .collect();

    let rows = match http_client.bulk_last_day(exchange, None).await {
        Ok(rows) => rows,
        Err(e) => {
            log::error!("Failed to poll the {exchange} bulk last day: {e}");

            return;
        }
    };

    for row in &rows {
        if !codes.contains(row.code.as_str()) {
            continue;
        }

        let ticker = row.ticker(exchange);
        let Some(state) = watches.get_mut(&ticker) else {
            continue;
        };

        match build_bulk_bar(row, state.bar_type, price_precision) {
            Ok(bar) => {
                if let Some(bar) = state.emitter.consider(bar) {
                    send_bar(sender, bar);
                }
            }
            Err(e) => log::error!("Failed to build the bulk bar for {ticker}: {e}"),
        }
    }
}

/// Sends a bar to the data engine, logging a failure instead of aborting a poll.
fn send_bar(sender: &EventSender<DataEvent>, bar: Bar) {
    if let Err(e) = sender.send(DataEvent::Data(Data::Bar(bar))) {
        log::error!("Failed to send bar event: {e}");
    }
}

/// Sends a quote to the data engine, logging a failure instead of aborting a poll.
fn send_quote(sender: &EventSender<DataEvent>, quote: QuoteTick) {
    if let Err(e) = sender.send(DataEvent::Data(Data::Quote(quote))) {
        log::error!("Failed to send quote event: {e}");
    }
}

/// Sends a trade to the data engine, logging a failure instead of aborting a stream.
fn send_trade(sender: &EventSender<DataEvent>, trade: TradeTick) {
    if let Err(e) = sender.send(DataEvent::Data(Data::Trade(trade))) {
        log::error!("Failed to send trade event: {e}");
    }
}

/// Returns the symbol a streaming channel addresses `instrument_id` with.
///
/// A streaming frame carries the symbol without an exchange suffix, and the United States channels
/// serve one venue, so the venue is checked rather than dropped silently.
///
/// # Errors
///
/// Returns an error if the instrument is not on the venue the channels serve.
fn streaming_symbol(instrument_id: InstrumentId) -> anyhow::Result<String> {
    if instrument_id.venue.as_str() != EODHD_WS_VENUE {
        anyhow::bail!(
            "The EODHD streaming channels serve the {EODHD_WS_VENUE} venue, so {instrument_id} cannot be streamed"
        );
    }

    Ok(instrument_id.symbol.to_string())
}

/// Returns the instrument ID a streaming frame refers to.
///
/// # Errors
///
/// Returns an error if the channel's venue is not a valid venue.
fn stream_instrument_id(symbol: &str) -> anyhow::Result<InstrumentId> {
    Ok(InstrumentId::new(
        Symbol::from(symbol),
        Venue::new_checked(EODHD_WS_VENUE)?,
    ))
}

/// Builds a [`TradeTick`] from a streaming trade print.
///
/// EODHD publishes neither a trade identifier nor an aggressor, so the tick carries
/// [`AggressorSide::NoAggressor`] and an identifier built from the symbol and a monotonic sequence,
/// rather than quoting an identifier the vendor never sent.
///
/// # Errors
///
/// Returns an error if the price or size cannot be represented at the configured precisions.
fn build_trade_tick(
    trade: &EodhdTradeMessage,
    price_precision: u8,
    sequence: &AtomicU64,
) -> anyhow::Result<TradeTick> {
    let instrument_id = stream_instrument_id(&trade.symbol)?;
    let price = Price::new_checked(trade.price, price_precision)?;
    let size = Quantity::new_checked(trade.size, SIZE_PRECISION)?;
    let ts_event = UnixNanos::from_millis(trade.timestamp.unsigned_abs());
    let sequence = sequence.fetch_add(1, Ordering::Relaxed);
    let trade_id = TradeId::new(format!("{}-{sequence}", trade.symbol));

    TradeTick::new_checked(
        instrument_id,
        price,
        size,
        AggressorSide::NoAggressor,
        trade_id,
        ts_event,
        ts_event,
    )
}

/// Builds a [`QuoteTick`] from a streaming best bid and offer update.
///
/// # Errors
///
/// Returns an error if a price or size cannot be represented at the configured precisions.
fn build_stream_quote(quote: &EodhdQuoteMessage, price_precision: u8) -> anyhow::Result<QuoteTick> {
    let instrument_id = stream_instrument_id(&quote.symbol)?;
    let bid_price = Price::new_checked(quote.bid_price, price_precision)?;
    let ask_price = Price::new_checked(quote.ask_price, price_precision)?;
    let bid_size = Quantity::new_checked(quote.bid_size, SIZE_PRECISION)?;
    let ask_size = Quantity::new_checked(quote.ask_size, SIZE_PRECISION)?;
    let ts_event = UnixNanos::from_millis(quote.timestamp.unsigned_abs());

    QuoteTick::new_checked(
        instrument_id,
        bid_price,
        ask_price,
        bid_size,
        ask_size,
        ts_event,
        ts_event,
    )
}

/// Emits the data a streaming frame carries, and reports what it does not carry.
fn emit_stream_message(
    sender: &EventSender<DataEvent>,
    channel: &str,
    message: EodhdWsMessage,
    price_precision: u8,
    sequence: &AtomicU64,
) {
    match message {
        EodhdWsMessage::Trade(trade) => match build_trade_tick(&trade, price_precision, sequence) {
            Ok(tick) => send_trade(sender, tick),
            Err(e) => log::error!("Failed to build a trade from the EODHD {channel} stream: {e}"),
        },
        EodhdWsMessage::Quote(quote) => match build_stream_quote(&quote, price_precision) {
            Ok(tick) => send_quote(sender, tick),
            Err(e) => log::error!("Failed to build a quote from the EODHD {channel} stream: {e}"),
        },
        EodhdWsMessage::Authorized => log::debug!("The EODHD {channel} stream is authorized"),
        EodhdWsMessage::Status { code, message } => {
            // A refused subscription arrives here rather than as a transport error.
            if code == 200 {
                log::debug!("The EODHD {channel} stream reported {message}");
            } else {
                log::warn!("The EODHD {channel} stream reported {code}: {message}");
            }
        }
        EodhdWsMessage::Unknown(value) => {
            log::debug!("Ignoring an EODHD {channel} frame: {value}");
        }
    }
}

/// Sends a corporate action to the data engine, logging a failure instead of aborting a request.
fn send_corporate_action(sender: &EventSender<DataEvent>, action: CorporateAction) {
    if let Err(e) = sender.send(DataEvent::Data(Data::CorporateAction(action))) {
        log::error!("Failed to send corporate action event: {e}");
    }
}

/// Emits the corporate actions effective in `[from, to]` for an instrument.
///
/// No data client command carries a corporate action: the engine publishes it on the instrument's
/// corporate action topic, where a strategy that called `subscribe_corporate_actions` receives it.
/// The client therefore publishes the actions that fall in the window of bars it was asked for,
/// which puts an action next to the bars it adjusts.
async fn emit_corporate_actions(
    http_client: &EodhdHttpClient,
    sender: &EventSender<DataEvent>,
    instrument_id: InstrumentId,
    from: i64,
    to: i64,
) {
    let ticker = instrument_id.to_string();
    let start = timestamp_to_date(from);
    let end = timestamp_to_date(to);
    let ts_event = get_atomic_clock_realtime().get_time_ns();

    match http_client
        .dividends(&ticker, Some(&start), Some(&end))
        .await
    {
        Ok(rows) => {
            log::debug!("Loaded {} dividends for {ticker}", rows.len());

            for row in &rows {
                match action_from_dividend(instrument_id, row, ts_event) {
                    Ok(action) => send_corporate_action(sender, action),
                    Err(e) => log::error!("Failed to convert a dividend for {ticker}: {e}"),
                }
            }
        }
        Err(e) => log::error!("Failed to load dividends for {ticker}: {e}"),
    }

    match http_client.splits(&ticker, Some(&start), Some(&end)).await {
        Ok(rows) => {
            log::debug!("Loaded {} splits for {ticker}", rows.len());

            for row in &rows {
                match action_from_split(instrument_id, row, ts_event) {
                    Ok(action) => send_corporate_action(sender, action),
                    Err(e) => log::error!("Failed to convert a split for {ticker}: {e}"),
                }
            }
        }
        Err(e) => log::error!("Failed to load splits for {ticker}: {e}"),
    }
}

/// Returns the current platform time as epoch seconds.
fn now_seconds() -> i64 {
    let nanos = get_atomic_clock_realtime().get_time_ns();

    i64::try_from(nanos.as_seconds()).unwrap_or(i64::MAX)
}

/// Writes a response, as a client does when it has answered a request.
fn respond(sender: &EventSender<DataEvent>, response: DataResponse) {
    if let Err(e) = sender.send(DataEvent::Response(response)) {
        log::error!("Failed to send a response: {e}");
    }
}

/// Returns one end of a request window as the nanoseconds a response carries it in.
fn bound_nanos(bound: Option<Timestamp>) -> Option<UnixNanos> {
    bound.map(UnixNanos::from)
}

/// Fetches bars for `bar_type` between `from` and `to` inclusive, as epoch seconds.
async fn fetch_bars(
    http_client: &EodhdHttpClient,
    bar_type: BarType,
    interval: EodhdInterval,
    from: i64,
    to: i64,
    price_precision: u8,
) -> anyhow::Result<Vec<Bar>> {
    if interval.is_intraday() {
        let rows = http_client
            .intraday_bars(
                &bar_type.instrument_id().to_string(),
                from,
                to,
                interval.code(),
            )
            .await?;

        return build_intraday_bars(&rows, bar_type, price_precision);
    }

    let start = timestamp_to_date(from);
    let end = timestamp_to_date(to);
    let rows = http_client
        .eod_bars(
            &bar_type.instrument_id().to_string(),
            Some(&start),
            Some(&end),
            interval.code(),
        )
        .await?;

    build_eod_bars(&rows, bar_type, price_precision)
}

/// Formats an epoch second as a UTC `YYYY-MM-DD` date.
fn timestamp_to_date(seconds: i64) -> String {
    let timestamp = Timestamp::from_second(seconds).unwrap_or(Timestamp::UNIX_EPOCH);
    let date = timestamp.to_zoned(jiff::tz::TimeZone::UTC).date();

    date.strftime("%Y-%m-%d").to_string()
}

/// Builds a [`QuoteTick`] from a delayed quote snapshot.
///
/// # Errors
///
/// Returns an error if a price or size cannot be represented at the given precisions.
fn build_quote(
    instrument_id: InstrumentId,
    quote: &EodhdDelayedQuote,
    price_precision: u8,
    ts_event: UnixNanos,
    ts_init: UnixNanos,
) -> anyhow::Result<QuoteTick> {
    let bid_price = Price::new_checked(quote.bid_price, price_precision)?;
    let ask_price = Price::new_checked(quote.ask_price, price_precision)?;
    let bid_size = Quantity::new_checked(quote.bid_size, SIZE_PRECISION)?;
    let ask_size = Quantity::new_checked(quote.ask_size, SIZE_PRECISION)?;

    QuoteTick::new_checked(
        instrument_id,
        bid_price,
        ask_price,
        bid_size,
        ask_size,
        ts_event,
        ts_init,
    )
}

/// Tracks the newest bar emitted for one bar type.
///
/// A streaming vendor publishes a forming bar repeatedly, so the same timestamp arrives with
/// changing values and then stops changing when the bar closes. This holds the last emitted bar
/// and reports only a newer timestamp, or a revision of the newest timestamp with different
/// values.
#[derive(Debug, Default)]
struct BarEmitter {
    last: Option<Bar>,
}

impl BarEmitter {
    /// Creates a new [`BarEmitter`] instance.
    const fn new() -> Self {
        Self { last: None }
    }

    /// Returns the bar to emit for `bar`, if any.
    fn consider(&mut self, bar: Bar) -> Option<Bar> {
        let should_emit = match &self.last {
            None => true,
            Some(last) if bar.ts_event > last.ts_event => true,
            Some(last) if bar.ts_event == last.ts_event => !same_values(last, &bar),
            Some(_) => false,
        };

        if should_emit {
            self.last = Some(bar);

            return Some(bar);
        }

        None
    }

    /// Returns the epoch second of the newest emitted bar.
    fn last_ts_seconds(&self) -> i64 {
        self.last.as_ref().map_or(0, |bar| {
            i64::try_from(bar.ts_event.as_seconds()).unwrap_or(i64::MAX)
        })
    }
}

fn same_values(left: &Bar, right: &Bar) -> bool {
    left.open == right.open
        && left.high == right.high
        && left.low == right.low
        && left.close == right.close
        && left.volume == right.volume
}

#[async_trait(?Send)]
impl DataClient for EodhdDataClient {
    fn client_id(&self) -> ClientId {
        self.client_id
    }

    fn venue(&self) -> Option<Venue> {
        None // Multi-venue: each instrument carries its own EODHD exchange code
    }

    fn start(&mut self) -> anyhow::Result<()> {
        log::info!("Starting {}", self.client_id);

        Ok(())
    }

    fn stop(&mut self) -> anyhow::Result<()> {
        log::info!("Stopping {}", self.client_id);

        self.tasks.begin_shutdown();
        self.bar_subscriptions.borrow_mut().clear();
        self.quote_subscriptions.borrow_mut().clear();
        self.bulk_polls.borrow_mut().clear();
        self.trades_stream = None;
        self.quotes_stream = None;
        self.trade_subscriptions.borrow_mut().clear();
        self.is_connected.store(false, Ordering::Release);

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
        if self.is_connected() && self.tasks.is_open() {
            return Ok(());
        }

        if !self.tasks.is_open() {
            self.tasks
                .finish_shutdown(
                    Duration::from_secs(1),
                    Duration::from_secs(SHUTDOWN_GRACE_SECS),
                )
                .await
                .map_err(|e| anyhow::anyhow!("Failed to terminate EODHD tasks: {e}"))?;
            self.tasks
                .start_generation()
                .map_err(|e| anyhow::anyhow!("Failed to start EODHD task generation: {e}"))?;
            self.cancellation_token = self.tasks.cancellation_token();
        }

        if self.config.load_instruments {
            let exchange = self.config.exchange.clone();
            let instruments = self.provider.fetch_instruments(&exchange).await?;

            log::debug!("Loaded {} instruments for {exchange}", instruments.len());

            self.store_instruments(instruments.clone());
            self.emit_instruments(&instruments);
        }

        if self.config.streaming {
            self.connect_streams().await?;
        }

        self.is_connected.store(true, Ordering::Release);
        log::info!("Connected: {}", self.client_id);

        Ok(())
    }

    async fn disconnect(&mut self) -> anyhow::Result<()> {
        let had_tasks = !self.tasks.is_empty();

        self.tasks.begin_shutdown();
        self.bar_subscriptions.borrow_mut().clear();
        self.quote_subscriptions.borrow_mut().clear();
        self.bulk_polls.borrow_mut().clear();
        self.trades_stream = None;
        self.quotes_stream = None;
        self.trade_subscriptions.borrow_mut().clear();

        let result = self
            .tasks
            .finish_shutdown(
                Duration::from_secs(1),
                Duration::from_secs(SHUTDOWN_GRACE_SECS),
            )
            .await
            .map_err(|e| anyhow::anyhow!("Failed to terminate EODHD tasks: {e}"));

        if had_tasks {
            log::info!("Disconnected: {}", self.client_id);
        }

        self.is_connected.store(false, Ordering::Release);

        result
    }

    fn subscribe_instruments(&mut self, _cmd: SubscribeInstruments) -> anyhow::Result<()> {
        let instruments = self.instruments();

        if instruments.is_empty() {
            log::warn!("No instruments loaded to publish for {}", self.client_id);
        } else {
            self.emit_instruments(&instruments);
        }

        Ok(())
    }

    fn subscribe_instrument(&mut self, cmd: SubscribeInstrument) -> anyhow::Result<()> {
        let instrument_id = cmd.instrument_id;
        let instrument = self.instruments.borrow().get(&instrument_id).cloned();

        match instrument {
            Some(instrument) => {
                if let Err(e) = self.data_sender.send(DataEvent::Instrument(instrument)) {
                    log::error!("Failed to send instrument event: {e}");
                }
            }
            None => log::warn!("Instrument {instrument_id} is not loaded"),
        }

        Ok(())
    }

    fn request_instruments(&self, request: RequestInstruments) -> anyhow::Result<()> {
        let instruments = self.instruments();

        if instruments.is_empty() {
            log::warn!(
                "No instruments loaded to respond with for {}",
                self.client_id
            );
        }

        // The response carries the instruments, and the engine emits them from it, so writing them as
        // data as well would emit each one twice.
        respond(
            &self.data_sender,
            DataResponse::Instruments(InstrumentsResponse::new(
                request.request_id,
                request.client_id.unwrap_or(self.client_id),
                request
                    .venue
                    .unwrap_or_else(|| Venue::from(EODHD_DEFAULT_EXCHANGE)),
                instruments,
                bound_nanos(request.start),
                bound_nanos(request.end),
                get_atomic_clock_realtime().get_time_ns(),
                request.params,
            )),
        );

        Ok(())
    }

    fn request_instrument(&self, request: RequestInstrument) -> anyhow::Result<()> {
        let instrument_id = request.instrument_id;
        let instrument = self.instruments.borrow().get(&instrument_id).cloned();

        let Some(instrument) = instrument else {
            log::warn!("Instrument {instrument_id} is not loaded");

            return Ok(());
        };

        respond(
            &self.data_sender,
            DataResponse::Instrument(Box::new(InstrumentResponse::new(
                request.request_id,
                request.client_id.unwrap_or(self.client_id),
                instrument_id,
                instrument,
                bound_nanos(request.start),
                bound_nanos(request.end),
                get_atomic_clock_realtime().get_time_ns(),
                request.params,
            ))),
        );

        Ok(())
    }

    fn subscribe_bars(&mut self, cmd: SubscribeBars) -> anyhow::Result<()> {
        let bar_type = cmd.bar_type;

        if bar_type.aggregation_source() != AggregationSource::External {
            anyhow::bail!(
                "EODHD serves externally aggregated bars only, so {bar_type} cannot be subscribed"
            );
        }

        if bar_type.is_composite() {
            anyhow::bail!(
                "EODHD serves standard bars only, so composite bar type {bar_type} cannot be subscribed"
            );
        }

        let interval = resolve_interval(&bar_type.spec())?;

        if self.bar_subscriptions.borrow().contains_key(&bar_type) {
            log::debug!("Already subscribed to {bar_type}");

            return Ok(());
        }

        let token = self.cancellation_token.child_token();
        let venue = bar_type.instrument_id().venue;

        // A daily bar on a bulk exchange is served by the exchange's shared poll rather than by
        // a request of its own, so a universe of any size costs one request per exchange.
        if interval == EodhdInterval::Day
            && matches_bulk_exchange(&self.config.bulk_exchanges, venue.as_str())
        {
            let sender = self.ensure_bulk_poll(venue.as_str())?;

            if sender
                .send(BulkWatch {
                    bar_type,
                    token: token.clone(),
                })
                .is_err()
            {
                anyhow::bail!("The EODHD bulk poll for {venue} is no longer running");
            }

            self.bar_subscriptions.borrow_mut().insert(bar_type, token);

            log::debug!("Subscribed to {bar_type} from the EODHD bulk last-day endpoint");

            return Ok(());
        }

        self.spawn_bar_poll(bar_type, interval, token.clone())?;
        self.bar_subscriptions.borrow_mut().insert(bar_type, token);

        log::debug!(
            "Subscribed to {bar_type} from the EODHD {} endpoint",
            if interval.is_intraday() {
                "intraday"
            } else {
                "eod"
            }
        );

        Ok(())
    }

    fn unsubscribe_bars(&mut self, cmd: &UnsubscribeBars) -> anyhow::Result<()> {
        if let Some(token) = self.bar_subscriptions.borrow_mut().remove(&cmd.bar_type) {
            token.cancel();
            log::debug!("Unsubscribed from {}", cmd.bar_type);
        }

        Ok(())
    }

    fn request_bars(&self, request: RequestBars) -> anyhow::Result<()> {
        let bar_type = request.bar_type;

        if bar_type.aggregation_source() != AggregationSource::External {
            anyhow::bail!(
                "EODHD serves externally aggregated bars only, so {bar_type} cannot be requested"
            );
        }

        if bar_type.is_composite() {
            anyhow::bail!(
                "EODHD serves standard bars only, so composite bar type {bar_type} cannot be requested"
            );
        }

        let interval = resolve_interval(&bar_type.spec())?;
        let token = self.cancellation_token.child_token();
        let http_client = self.http_client.clone();
        let sender = self.data_sender.clone();
        let correlation_id = request.request_id;
        let client_id = request.client_id.unwrap_or(self.client_id);
        let start = request.start;
        let end = request.end;
        let params = request.params;
        let price_precision = self.config.price_precision;
        let load_corporate_actions = self.config.load_corporate_actions;

        self.tasks.spawn(async move {
            let now = now_seconds();
            let from = start.map_or(now - SECONDS_PER_DAY, Timestamp::as_second);
            let to = end.map_or(now, Timestamp::as_second);

            // The select arm is for prompt shutdown if the client disconnects mid-request.
            let bars = tokio::select! {
                biased;
                () = token.cancelled() => return,
                bars = fetch_bars(&http_client, bar_type, interval, from, to, price_precision) => bars,
            };

            match bars {
                Ok(bars) => {
                    log::debug!("Requested {} bars for {bar_type}", bars.len());

                    // The response carries the bars, and the engine emits them from it, so sending
                    // them as data as well would emit each one twice.
                    respond(
                        &sender,
                        DataResponse::Bars(BarsResponse::new(
                            correlation_id,
                            client_id,
                            bar_type,
                            bars,
                            bound_nanos(start),
                            bound_nanos(end),
                            get_atomic_clock_realtime().get_time_ns(),
                            params,
                        )),
                    );

                    if load_corporate_actions {
                        emit_corporate_actions(
                            &http_client,
                            &sender,
                            bar_type.instrument_id(),
                            from,
                            to,
                        )
                        .await;
                    }
                }
                Err(e) => log::error!("Failed to request bars for {bar_type}: {e}"),
            }
        })?;

        Ok(())
    }

    fn subscribe_trades(&mut self, cmd: SubscribeTrades) -> anyhow::Result<()> {
        let instrument_id = cmd.instrument_id;

        if self.trade_subscriptions.borrow().contains(&instrument_id) {
            log::debug!("Already subscribed to trades for {instrument_id}");

            return Ok(());
        }

        // The delayed endpoint publishes no trades, so a trade subscription has one source.
        let stream = self.trades_stream.as_ref().ok_or_else(|| {
            anyhow::anyhow!(
                "EODHD trades require 'streaming' to be enabled, because the delayed quote endpoint publishes no trades"
            )
        })?;

        let symbol = streaming_symbol(instrument_id)?;
        stream.send(StreamCommand::Subscribe(vec![symbol]))?;
        self.trade_subscriptions.borrow_mut().insert(instrument_id);

        log::debug!(
            "Subscribed to trades for {instrument_id} from the EODHD {} channel",
            stream.channel()
        );

        Ok(())
    }

    fn unsubscribe_trades(&mut self, cmd: &UnsubscribeTrades) -> anyhow::Result<()> {
        let instrument_id = cmd.instrument_id;

        if !self.trade_subscriptions.borrow_mut().remove(&instrument_id) {
            return Ok(());
        }

        if let Some(stream) = self.trades_stream.as_ref()
            && let Ok(symbol) = streaming_symbol(instrument_id)
        {
            stream.send(StreamCommand::Unsubscribe(vec![symbol]))?;
        }

        log::debug!("Unsubscribed from trades for {instrument_id}");

        Ok(())
    }

    fn subscribe_quotes(&mut self, cmd: SubscribeQuotes) -> anyhow::Result<()> {
        let instrument_id = cmd.instrument_id;

        if self
            .quote_subscriptions
            .borrow()
            .contains_key(&instrument_id)
        {
            log::debug!("Already subscribed to quotes for {instrument_id}");

            return Ok(());
        }

        let token = self.cancellation_token.child_token();

        // With streaming enabled the undelayed channel serves every quote subscription, so the
        // delayed polling path is not used at all.
        if let Some(stream) = self.quotes_stream.as_ref() {
            let symbol = streaming_symbol(instrument_id)?;
            stream.send(StreamCommand::Subscribe(vec![symbol]))?;
            self.quote_subscriptions
                .borrow_mut()
                .insert(instrument_id, token);

            log::debug!(
                "Subscribed to quotes for {instrument_id} from the EODHD {} channel",
                stream.channel()
            );

            return Ok(());
        }

        self.spawn_quote_poll(instrument_id, token.clone())?;
        self.quote_subscriptions
            .borrow_mut()
            .insert(instrument_id, token);

        log::debug!("Subscribed to quotes for {instrument_id} from the delayed quote endpoint");

        Ok(())
    }

    fn unsubscribe_quotes(&mut self, cmd: &UnsubscribeQuotes) -> anyhow::Result<()> {
        let instrument_id = cmd.instrument_id;

        let Some(token) = self.quote_subscriptions.borrow_mut().remove(&instrument_id) else {
            return Ok(());
        };

        token.cancel();

        if let Some(stream) = self.quotes_stream.as_ref()
            && let Ok(symbol) = streaming_symbol(instrument_id)
        {
            stream.send(StreamCommand::Unsubscribe(vec![symbol]))?;
        }

        log::debug!("Unsubscribed from quotes for {instrument_id}");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use nautilus_common::live::runner::replace_data_event_sender;
    use nautilus_core::UUID4;
    use nautilus_model::{
        data::BarSpecification,
        enums::{BarAggregation, PriceType},
        identifiers::InstrumentId,
    };
    use rstest::rstest;
    use tokio::io::AsyncWriteExt;

    use super::*;
    use crate::bars::{bar_type_for, date_end_secs, date_start_secs};

    /// One end-of-day row, as the endpoint returns it.
    const EOD_BODY: &str = r#"[{"date":"2024-01-02","open":100.0,"high":102.0,"low":99.0,"close":101.0,"adjusted_close":101.0,"volume":1000.0}]"#;

    /// Answers one request with `body`, so that a request path can be exercised without the venue.
    async fn serve_once(body: &'static str) -> (String, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();

        let handle = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = vec![0_u8; 4096];
            let _ = socket.readable().await;
            let _ = socket.try_read(&mut request);

            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );

            let _ = socket.write_all(response.as_bytes()).await;
            let _ = socket.shutdown().await;
        });

        (format!("http://{address}"), handle)
    }

    /// Waits for the client to answer, or fails the test rather than hanging the suite.
    async fn answer(events: &mut tokio::sync::mpsc::UnboundedReceiver<DataEvent>) -> DataEvent {
        tokio::time::timeout(Duration::from_secs(5), events.recv())
            .await
            .expect("the client should answer within the deadline")
            .expect("the client should have written an event")
    }

    /// A request is answered with a response under the identity the caller asked for. A chain cannot
    /// see a provider serve, or fail to serve, from anything else.
    #[tokio::test]
    async fn test_a_bar_request_is_answered_with_a_response_carrying_the_callers_identifier() {
        let (base_url, server) = serve_once(EOD_BODY).await;
        let (sender, mut events) = tokio::sync::mpsc::unbounded_channel();
        replace_data_event_sender(EventSender::from(sender));

        let config = EodhdDataClientConfig::builder()
            .api_key(SecretString::from("test"))
            .http_base_url(SecretString::from(base_url))
            .build();
        let client = EodhdDataClient::new(ClientId::from("EODHD"), config).unwrap();

        let request = RequestBars::new(
            bar_type_for(InstrumentId::from("AAPL.US"), EodhdInterval::Day),
            None,
            None,
            None,
            Some(ClientId::from("CHAIN")),
            UUID4::new(),
            UnixNanos::default(),
            None,
        );
        let correlation_id = request.request_id;

        client.request_bars(request).unwrap();

        let DataEvent::Response(DataResponse::Bars(response)) = answer(&mut events).await else {
            panic!("expected a bar response");
        };

        assert_eq!(response.correlation_id, correlation_id);
        assert_eq!(response.client_id, ClientId::from("CHAIN"));
        assert_eq!(response.data.len(), 1);
        assert_eq!(response.data[0].close.as_f64(), 101.0);

        server.await.unwrap();
    }

    fn bar(ts_seconds: i64, close: f64) -> Bar {
        let instrument_id = InstrumentId::from("AAPL.US");
        let bar_type = bar_type_for(instrument_id, EodhdInterval::Minute1);
        let precision = 2;

        build_single_bar(bar_type, close, ts_seconds, precision)
    }

    fn build_single_bar(bar_type: BarType, close: f64, ts_seconds: i64, precision: u8) -> Bar {
        let price = |value: f64| Price::new(value, precision);

        Bar::new(
            bar_type,
            price(close - 1.0),
            price(close + 1.0),
            price(close - 2.0),
            price(close),
            Quantity::new(10.0, 0),
            UnixNanos::from_millis((ts_seconds as u64) * 1_000),
            UnixNanos::from_millis((ts_seconds as u64) * 1_000),
        )
    }

    #[rstest]
    fn test_bar_emitter_emits_the_first_bar() {
        let mut emitter = BarEmitter::new();

        let emitted = emitter.consider(bar(1_700_000_000, 100.0));

        assert!(emitted.is_some());
        assert_eq!(emitter.last_ts_seconds(), 1_700_000_000);
    }

    #[rstest]
    fn test_bar_emitter_suppresses_an_unchanged_repeat() {
        let mut emitter = BarEmitter::new();
        emitter.consider(bar(1_700_000_000, 100.0));

        let emitted = emitter.consider(bar(1_700_000_000, 100.0));

        assert!(emitted.is_none());
    }

    #[rstest]
    fn test_bar_emitter_emits_a_revision_of_the_newest_bar() {
        let mut emitter = BarEmitter::new();
        emitter.consider(bar(1_700_000_000, 100.0));

        let emitted = emitter.consider(bar(1_700_000_000, 101.5));

        assert!(emitted.is_some());
        assert_eq!(emitted.unwrap().close, Price::new(101.5, 2));
    }

    #[rstest]
    fn test_bar_emitter_emits_a_newer_bar_and_not_an_older_one() {
        let mut emitter = BarEmitter::new();
        emitter.consider(bar(1_700_000_000, 100.0));
        emitter.consider(bar(1_700_000_060, 101.0));

        assert!(emitter.consider(bar(1_699_999_940, 99.0)).is_none());
        assert_eq!(emitter.last_ts_seconds(), 1_700_000_060);
        assert!(emitter.consider(bar(1_700_000_120, 102.0)).is_some());
    }

    fn delayed_quote() -> EodhdDelayedQuote {
        EodhdDelayedQuote {
            symbol: "AAPL.US".to_string(),
            bid_price: 330.46,
            ask_price: 330.57,
            bid_size: 12.0,
            ask_size: 2.0,
            bid_time: Some(1_790_886_551_000),
            ask_time: Some(1_790_886_551_000),
            timestamp: Some(1_790_900_940),
        }
    }

    #[rstest]
    fn test_build_quote_maps_the_snapshot_onto_a_quote_tick() {
        let quote = delayed_quote();
        let ts_event = quote.ts_event();
        let instrument_id = InstrumentId::from("AAPL.US");

        let tick = build_quote(instrument_id, &quote, 2, ts_event, ts_event).unwrap();

        assert_eq!(tick.instrument_id, instrument_id);
        assert_eq!(tick.bid_price, Price::new(330.46, 2));
        assert_eq!(tick.ask_price, Price::new(330.57, 2));
        assert_eq!(tick.bid_size, Quantity::new(12.0, 0));
        assert_eq!(tick.ask_size, Quantity::new(2.0, 0));
        assert_eq!(tick.ts_event.as_u64(), 1_790_886_551_000_000_000);
    }

    #[rstest]
    fn test_ts_event_prefers_the_quote_time_over_the_snapshot_time() {
        let mut quote = delayed_quote();
        quote.bid_time = Some(1_790_886_560_000);
        quote.ask_time = Some(1_790_886_570_000);

        assert_eq!(quote.ts_event().as_u64(), 1_790_886_570_000_000_000);
    }

    fn trade_message() -> EodhdTradeMessage {
        EodhdTradeMessage {
            symbol: "AAPL".to_string(),
            price: 330.78,
            size: 5.0,
            timestamp: 1_790_899_194_028,
            dark_pool: Some(false),
            session: Some("extended-hours".to_string()),
        }
    }

    fn quote_message() -> EodhdQuoteMessage {
        EodhdQuoteMessage {
            symbol: "AAPL".to_string(),
            ask_price: 330.8,
            ask_size: 15.0,
            bid_price: 330.0,
            bid_size: 67.0,
            timestamp: 1_790_899_198_000,
        }
    }

    #[rstest]
    fn test_build_trade_tick_maps_a_streaming_print() {
        let sequence = AtomicU64::new(0);

        let tick = build_trade_tick(&trade_message(), 2, &sequence).unwrap();

        assert_eq!(tick.instrument_id, InstrumentId::from("AAPL.US"));
        assert_eq!(tick.price, Price::new(330.78, 2));
        assert_eq!(tick.size, Quantity::new(5.0, 0));
        // The feed carries neither an aggressor nor a trade identifier.
        assert_eq!(tick.aggressor_side, AggressorSide::NoAggressor);
        assert_eq!(tick.ts_event, UnixNanos::from_millis(1_790_899_194_028));
        assert_eq!(tick.trade_id.to_string(), "AAPL-0");
    }

    #[rstest]
    fn test_build_trade_tick_advances_the_identifier() {
        let sequence = AtomicU64::new(0);

        let first = build_trade_tick(&trade_message(), 2, &sequence).unwrap();
        let second = build_trade_tick(&trade_message(), 2, &sequence).unwrap();

        assert_ne!(first.trade_id, second.trade_id);
    }

    #[rstest]
    fn test_build_trade_tick_rejects_a_price_the_precision_cannot_hold() {
        let sequence = AtomicU64::new(0);
        let mut trade = trade_message();
        trade.price = f64::NAN;

        assert!(build_trade_tick(&trade, 2, &sequence).is_err());
    }

    #[rstest]
    fn test_build_stream_quote_maps_both_sides() {
        let tick = build_stream_quote(&quote_message(), 2).unwrap();

        assert_eq!(tick.instrument_id, InstrumentId::from("AAPL.US"));
        assert_eq!(tick.bid_price, Price::new(330.0, 2));
        assert_eq!(tick.ask_price, Price::new(330.8, 2));
        assert_eq!(tick.bid_size, Quantity::new(67.0, 0));
        assert_eq!(tick.ask_size, Quantity::new(15.0, 0));
        assert_eq!(tick.ts_event, UnixNanos::from_millis(1_790_899_198_000));
    }

    #[rstest]
    fn test_streaming_symbol_checks_the_venue_the_channels_serve() {
        assert_eq!(
            streaming_symbol(InstrumentId::from("AAPL.US")).unwrap(),
            "AAPL"
        );
        // The channels carry a bare symbol, so a venue they do not serve has no symbol to send.
        assert!(streaming_symbol(InstrumentId::from("VOD.LSE")).is_err());
    }

    #[rstest]
    fn test_stream_instrument_id_restores_the_channel_venue() {
        assert_eq!(
            stream_instrument_id("AAPL").unwrap(),
            InstrumentId::from("AAPL.US")
        );
    }

    #[rstest]
    fn test_matches_bulk_exchange_ignores_case_and_absent_codes() {
        let codes = vec!["US".to_string(), "LSE".to_string()];

        assert!(matches_bulk_exchange(&codes, "US"));
        assert!(matches_bulk_exchange(&codes, "us"));
        assert!(matches_bulk_exchange(&codes, "LSE"));
        assert!(!matches_bulk_exchange(&codes, "TO"));
        assert!(!matches_bulk_exchange(&[], "US"));
    }

    #[rstest]
    fn test_fetch_bars_rejects_an_unsupported_spec_before_any_request() {
        let spec = BarSpecification::new(15, BarAggregation::Minute, PriceType::Last);

        assert!(resolve_interval(&spec).is_err());
    }

    #[rstest]
    fn test_timestamp_to_date_formats_utc() {
        assert_eq!(timestamp_to_date(0), "1970-01-01");
        assert_eq!(timestamp_to_date(1_704_153_600), "2024-01-02");
    }

    #[rstest]
    fn test_date_bounds_round_trip_through_the_request_window() {
        let start = date_start_secs("2024-01-02").unwrap();
        let end = date_end_secs("2024-01-02").unwrap();

        assert_eq!(timestamp_to_date(start), "2024-01-02");
        assert_eq!(timestamp_to_date(end), "2024-01-02");
    }
}
