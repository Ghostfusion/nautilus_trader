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
        atomic::{AtomicBool, Ordering},
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
            RequestBars, RequestInstrument, RequestInstruments, SubscribeBars, SubscribeInstrument,
            SubscribeInstruments, SubscribeQuotes, UnsubscribeBars, UnsubscribeQuotes,
        },
    },
};
use nautilus_core::{UnixNanos, string::secret::SecretString, time::get_atomic_clock_realtime};
use nautilus_live::task::TaskGroup;
use nautilus_model::{
    data::{Bar, BarType, Data, QuoteTick},
    enums::AggregationSource,
    identifiers::{ClientId, InstrumentId, Venue},
    instruments::{Instrument, InstrumentAny},
    types::{Price, Quantity},
};
use tokio_util::sync::CancellationToken;

use crate::{
    bars::{EodhdInterval, build_bulk_bar, build_eod_bars, build_intraday_bars, resolve_interval},
    config::EodhdDataClientConfig,
    http::{EodhdDelayedQuote, EodhdHttpClient},
    providers::EodhdInstrumentProvider,
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

        self.tasks.spawn(async move {
            let mut emitter = BarEmitter::new();
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
) -> BulkWatchState {
    let mut emitter = BarEmitter::new();

    if backfill_days > 0 {
        let now = now_seconds();

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

/// Returns the current platform time as epoch seconds.
fn now_seconds() -> i64 {
    let nanos = get_atomic_clock_realtime().get_time_ns();

    i64::try_from(nanos.as_seconds()).unwrap_or(i64::MAX)
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

    fn request_instruments(&self, _request: RequestInstruments) -> anyhow::Result<()> {
        let instruments = self.instruments();

        if instruments.is_empty() {
            log::warn!(
                "No instruments loaded to respond with for {}",
                self.client_id
            );
        } else {
            self.emit_instruments(&instruments);
        }

        Ok(())
    }

    fn request_instrument(&self, request: RequestInstrument) -> anyhow::Result<()> {
        let instrument_id = request.instrument_id;
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
        let start = request.start;
        let end = request.end;
        let price_precision = self.config.price_precision;

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

                    for bar in bars {
                        send_bar(&sender, bar);
                    }
                }
                Err(e) => log::error!("Failed to request bars for {bar_type}: {e}"),
            }
        })?;

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

        self.spawn_quote_poll(instrument_id, token.clone())?;
        self.quote_subscriptions
            .borrow_mut()
            .insert(instrument_id, token);

        log::debug!("Subscribed to quotes for {instrument_id} from the delayed quote endpoint");

        Ok(())
    }

    fn unsubscribe_quotes(&mut self, cmd: &UnsubscribeQuotes) -> anyhow::Result<()> {
        if let Some(token) = self
            .quote_subscriptions
            .borrow_mut()
            .remove(&cmd.instrument_id)
        {
            token.cancel();
            log::debug!("Unsubscribed from quotes for {}", cmd.instrument_id);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use nautilus_model::{
        data::BarSpecification,
        enums::{BarAggregation, PriceType},
        identifiers::InstrumentId,
    };
    use rstest::rstest;

    use super::*;
    use crate::bars::{bar_type_for, date_end_secs, date_start_secs};

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
