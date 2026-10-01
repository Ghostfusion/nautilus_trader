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

//! Read-only observation of a parent order's execution.
//!
//! [`ExecutionObserver`] is a pure, clock-free accumulator: a caller feeds it the declared terms,
//! the decision/arrival/submission timestamps, the child submissions and cancellations, the fills
//! and the quote/trade observations of the benchmark interval, and it answers with the metric set.
//! It holds no state beyond the observation window, mutates no order and keeps no parallel ledger;
//! it is safe to construct per execution and discard afterwards. Everything it holds is plain data,
//! so it is `Send + Sync`-friendly and free of any message bus dependency.
//!
//! [`ExecutionAnalyticsCollector`] is a thin [`DataActor`] that subscribes to the real quote, trade
//! and order events and feeds the same accumulator, delegating all arithmetic to it.

use std::{fmt::Debug, fmt::Formatter};

use nautilus_common::{
    actor::{
        DataActor, DataActorConfig, DataActorCore, DataActorNative,
        registry::try_get_actor_unchecked,
    },
    msgbus::{self, TypedHandler},
    nautilus_actor,
};
use nautilus_core::{DurationNanos, UnixNanos};
use nautilus_model::{
    data::{QuoteTick, TradeTick},
    enums::OrderSide,
    events::OrderEventAny,
    identifiers::{ActorId, ClientOrderId, InstrumentId, StrategyId},
    orders::Order,
    types::{Price, Quantity},
};
use rust_decimal::{Decimal, prelude::ToPrimitive};

use super::metrics::{ExecutionMetrics, MetricInputs};

/// The parent order's terms, as declared by the caller.
#[derive(Clone, Debug, PartialEq)]
pub struct ExecutionTerms {
    /// The instrument the parent executes in.
    pub instrument_id: InstrumentId,
    /// The side of the parent order.
    pub order_side: OrderSide,
    /// The parent order's target quantity.
    pub quantity: Quantity,
    /// The parent order's own limit price, when it has one.
    pub limit_price: Option<Price>,
    /// The declared horizon, when the caller declared one.
    pub horizon: Option<DurationNanos>,
}

/// A declared reference price together with the timestamp it belongs to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReferencePoint {
    /// The timestamp of the reference price.
    pub timestamp: UnixNanos,
    /// The reference price.
    pub price: Price,
}

/// The declared benchmark interval a VWAP or TWAP is formed over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BenchmarkInterval {
    /// The interval start (inclusive).
    pub start: UnixNanos,
    /// The interval end (inclusive).
    pub end: UnixNanos,
}

impl BenchmarkInterval {
    /// Returns whether `timestamp` lies within the interval.
    #[must_use]
    pub fn contains(&self, timestamp: UnixNanos) -> bool {
        timestamp >= self.start && timestamp <= self.end
    }
}

/// A top-of-book observation used for midpoint and spread metrics.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QuoteObservation {
    /// The observation timestamp.
    pub timestamp: UnixNanos,
    /// The quoted bid price.
    pub bid_price: Price,
    /// The quoted ask price.
    pub ask_price: Price,
}

impl QuoteObservation {
    /// Returns the quoted midpoint.
    #[must_use]
    pub fn mid(&self) -> f64 {
        self.bid_price.as_f64().midpoint(self.ask_price.as_f64())
    }

    /// Returns the quoted half-spread.
    #[must_use]
    pub fn half_spread(&self) -> f64 {
        (self.ask_price.as_f64() - self.bid_price.as_f64()) / 2.0
    }
}

/// A trade observation used for benchmark VWAP.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TradeObservation {
    /// The observation timestamp.
    pub timestamp: UnixNanos,
    /// The traded price.
    pub price: Price,
    /// The traded size.
    pub size: Quantity,
}

/// A fill observation attributed to a child order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FillObservation {
    /// The client order ID of the child that filled.
    pub child_id: ClientOrderId,
    /// The fill timestamp.
    pub timestamp: UnixNanos,
    /// The fill quantity.
    pub quantity: Quantity,
    /// The fill price.
    pub price: Price,
}

/// A child submission or cancellation observation.
///
/// For a submission the quantity is the submitted quantity, and for a cancellation it is the
/// unfilled quantity the cancellation closes out.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChildObservation {
    /// The client order ID of the child.
    pub child_id: ClientOrderId,
    /// The event timestamp.
    pub timestamp: UnixNanos,
    /// The quantity the observation refers to.
    pub quantity: Quantity,
}

/// A pure, clock-free accumulator for one parent order's execution observations.
///
/// The observer stores only what it is fed and derives every metric without external state. It does
/// not mutate orders, maintain a ledger or depend on a clock or message bus.
#[derive(Clone, Debug, PartialEq)]
pub struct ExecutionObserver {
    terms: ExecutionTerms,
    decision: Option<ReferencePoint>,
    arrival: Option<ReferencePoint>,
    parent_submitted: Option<UnixNanos>,
    benchmark: Option<BenchmarkInterval>,
    child_submissions: Vec<ChildObservation>,
    child_cancellations: Vec<ChildObservation>,
    fills: Vec<FillObservation>,
    quotes: Vec<QuoteObservation>,
    trades: Vec<TradeObservation>,
    algorithm_child_counts: Option<(u32, u32)>,
}

impl ExecutionObserver {
    /// Creates a new observer for the declared parent terms.
    #[must_use]
    pub fn new(terms: ExecutionTerms) -> Self {
        Self {
            terms,
            decision: None,
            arrival: None,
            parent_submitted: None,
            benchmark: None,
            child_submissions: Vec::new(),
            child_cancellations: Vec::new(),
            fills: Vec::new(),
            quotes: Vec::new(),
            trades: Vec::new(),
            algorithm_child_counts: None,
        }
    }

    /// Sets the benchmark interval.
    #[must_use]
    pub fn with_benchmark(mut self, interval: BenchmarkInterval) -> Self {
        self.benchmark = Some(interval);
        self
    }

    /// Returns the declared parent terms.
    #[must_use]
    pub fn terms(&self) -> &ExecutionTerms {
        &self.terms
    }

    /// Returns the declared benchmark interval.
    #[must_use]
    pub fn benchmark(&self) -> Option<BenchmarkInterval> {
        self.benchmark
    }

    /// Sets the decision timestamp and price.
    pub fn set_decision(&mut self, timestamp: UnixNanos, price: Price) {
        self.decision = Some(ReferencePoint { timestamp, price });
    }

    /// Sets the arrival timestamp and price.
    pub fn set_arrival(&mut self, timestamp: UnixNanos, price: Price) {
        self.arrival = Some(ReferencePoint { timestamp, price });
    }

    /// Sets the parent order's submission timestamp.
    pub fn set_parent_submitted(&mut self, timestamp: UnixNanos) {
        self.parent_submitted = Some(timestamp);
    }

    /// Records the child counts reported by the execution algorithm that produced this execution.
    ///
    /// When set, the count metrics (child count and child churn) use these counts rather than
    /// re-deriving them from the observed child events, so a caller holding an algorithm can
    /// reuse its own `counts(primary_id)` evidence. The quantity-based metrics always work from
    /// the observed child events.
    pub fn set_algorithm_child_counts(&mut self, submitted: u32, cancelled: u32) {
        self.algorithm_child_counts = Some((submitted, cancelled));
    }

    /// Records a child submission.
    pub fn observe_child_submitted(
        &mut self,
        child_id: ClientOrderId,
        timestamp: UnixNanos,
        quantity: Quantity,
    ) {
        self.child_submissions.push(ChildObservation {
            child_id,
            timestamp,
            quantity,
        });
    }

    /// Records a child cancellation with the unfilled quantity it closes out.
    pub fn observe_child_canceled(
        &mut self,
        child_id: ClientOrderId,
        timestamp: UnixNanos,
        quantity: Quantity,
    ) {
        self.child_cancellations.push(ChildObservation {
            child_id,
            timestamp,
            quantity,
        });
    }

    /// Records a fill.
    pub fn observe_fill(
        &mut self,
        child_id: ClientOrderId,
        timestamp: UnixNanos,
        quantity: Quantity,
        price: Price,
    ) {
        self.fills.push(FillObservation {
            child_id,
            timestamp,
            quantity,
            price,
        });
    }

    /// Records a quote observation.
    pub fn observe_quote(&mut self, observation: QuoteObservation) {
        self.quotes.push(observation);
    }

    /// Records a trade observation.
    pub fn observe_trade(&mut self, observation: TradeObservation) {
        self.trades.push(observation);
    }

    /// Records a quote tick.
    pub fn observe_quote_tick(&mut self, quote: &QuoteTick) {
        self.observe_quote(QuoteObservation {
            timestamp: quote.ts_event,
            bid_price: quote.bid_price,
            ask_price: quote.ask_price,
        });
    }

    /// Records a trade tick.
    pub fn observe_trade_tick(&mut self, tick: &TradeTick) {
        self.observe_trade(TradeObservation {
            timestamp: tick.ts_event,
            price: tick.price,
            size: tick.size,
        });
    }

    /// Returns the full metric set for the observations received so far.
    #[must_use]
    pub fn metrics(&self) -> ExecutionMetrics {
        let filled_qty: f64 = self.fills.iter().map(|fill| fill.quantity.as_f64()).sum();
        let submitted_child_qty: f64 = self
            .child_submissions
            .iter()
            .map(|child| child.quantity.as_f64())
            .sum();
        let cancelled_child_qty: f64 = self
            .child_cancellations
            .iter()
            .map(|child| child.quantity.as_f64())
            .sum();
        let (submitted_child_count, cancelled_child_count) = self.child_counts();
        let arrival_quote = self.arrival_quote();

        let input = MetricInputs {
            side: self.terms.order_side,
            parent_target_qty: self.terms.quantity.as_f64(),
            limit_price: self.terms.limit_price.map(|price| price.as_f64()),
            horizon: self.terms.horizon,
            decision_price: self.decision.map(|point| point.price.as_f64()),
            arrival_price: self.arrival.map(|point| point.price.as_f64()),
            parent_submitted: self.parent_submitted,
            last_terminal: self.last_terminal(),
            fill_avg_px: self.fill_avg_px(),
            filled_qty,
            interval_vwap: self.interval_vwap(),
            interval_twap: self.interval_twap(),
            arrival_mid: arrival_quote.map(QuoteObservation::mid),
            arrival_half_spread: arrival_quote.map(QuoteObservation::half_spread),
            adverse_selection: self.adverse_selection(),
            submitted_child_count,
            cancelled_child_count,
            submitted_child_qty,
            cancelled_child_qty,
            mean_child_lifetime_s: self.mean_child_lifetime_s(),
            partial_fill_ratio: self.partial_fill_ratio(),
            price_improvement: self.price_improvement(),
        };

        ExecutionMetrics::compute(&input)
    }

    /// Returns the child counts, preferring the algorithm's own counts when recorded.
    fn child_counts(&self) -> (u32, u32) {
        self.algorithm_child_counts.unwrap_or((
            self.child_submissions.len() as u32,
            self.child_cancellations.len() as u32,
        ))
    }

    /// Returns the quantity-weighted average fill price accumulated in `Decimal`.
    fn fill_avg_px(&self) -> Option<f64> {
        let mut notional = Decimal::ZERO;
        let mut qty = Decimal::ZERO;
        for fill in &self.fills {
            notional += fill.price.as_decimal() * fill.quantity.as_decimal();
            qty += fill.quantity.as_decimal();
        }
        if qty.is_zero() {
            None
        } else {
            (notional / qty).to_f64()
        }
    }

    /// Returns the benchmark interval VWAP, formed from the trades inside the interval.
    fn interval_vwap(&self) -> Option<f64> {
        let interval = self.benchmark?;
        let mut notional = Decimal::ZERO;
        let mut size = Decimal::ZERO;
        for trade in &self.trades {
            if interval.contains(trade.timestamp) {
                notional += trade.price.as_decimal() * trade.size.as_decimal();
                size += trade.size.as_decimal();
            }
        }
        if size.is_zero() {
            None
        } else {
            (notional / size).to_f64()
        }
    }

    /// Returns the benchmark interval TWAP, the equal-weighted mean of the quoted midpoints inside
    /// the interval.
    fn interval_twap(&self) -> Option<f64> {
        let interval = self.benchmark?;
        let mut total = 0.0;
        let mut count = 0u32;
        for quote in &self.quotes {
            if interval.contains(quote.timestamp) {
                total += quote.mid();
                count += 1;
            }
        }
        (count > 0).then(|| total / f64::from(count))
    }

    /// Returns the last quote observed at or before `timestamp`.
    fn quote_at(&self, timestamp: UnixNanos) -> Option<&QuoteObservation> {
        self.quotes
            .iter()
            .filter(|quote| quote.timestamp <= timestamp)
            .max_by_key(|quote| quote.timestamp)
    }

    /// Returns the quote at the declared arrival timestamp.
    fn arrival_quote(&self) -> Option<&QuoteObservation> {
        let arrival = self.arrival?;
        self.quote_at(arrival.timestamp)
    }

    /// Returns the mean signed adverse mid move over the declared horizon.
    ///
    /// For each fill the move runs from the midpoint at the fill timestamp to the midpoint at the
    /// fill timestamp plus the horizon, signed so that a positive value is adverse to the parent's
    /// side (the mid moved against the fill). Fills lacking a midpoint at either end are excluded.
    fn adverse_selection(&self) -> Option<f64> {
        let horizon = self.terms.horizon?;
        let mut total = 0.0;
        let mut count = 0u32;
        for fill in &self.fills {
            let Some(end) = fill.timestamp.checked_add(horizon) else {
                continue;
            };
            let (Some(mid_at_fill), Some(mid_at_horizon)) =
                (self.quote_mid_at(fill.timestamp), self.quote_mid_at(end))
            else {
                continue;
            };
            let move_px = match self.terms.order_side {
                OrderSide::Buy => mid_at_fill - mid_at_horizon,
                OrderSide::Sell => mid_at_horizon - mid_at_fill,
            };
            total += move_px;
            count += 1;
        }
        (count > 0).then(|| total / f64::from(count))
    }

    /// Returns the midpoint of the last quote at or before `timestamp`.
    fn quote_mid_at(&self, timestamp: UnixNanos) -> Option<f64> {
        self.quote_at(timestamp).map(QuoteObservation::mid)
    }

    /// Returns the timestamp of the last observed fill or cancellation.
    fn last_terminal(&self) -> Option<UnixNanos> {
        let fill = self.fills.iter().map(|fill| fill.timestamp).max();
        let cancel = self
            .child_cancellations
            .iter()
            .map(|child| child.timestamp)
            .max();
        match (fill, cancel) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        }
    }

    /// Returns the mean lifetime of cancelled children, in seconds.
    fn mean_child_lifetime_s(&self) -> Option<f64> {
        let mut total = 0.0;
        let mut count = 0u32;
        for cancelled in &self.child_cancellations {
            let Some(submitted) = self.submission_of(cancelled.child_id) else {
                continue;
            };
            total += cancelled
                .timestamp
                .saturating_duration_since(submitted.timestamp)
                .as_secs_f64();
            count += 1;
        }
        (count > 0).then(|| total / f64::from(count))
    }

    /// Returns the mean partial fill ratio of cancelled children.
    ///
    /// Each cancelled child contributes its filled quantity over its submitted quantity, so the
    /// denominator is the cancelled child's own submitted quantity.
    fn partial_fill_ratio(&self) -> Option<f64> {
        let mut total = 0.0;
        let mut count = 0u32;
        for cancelled in &self.child_cancellations {
            let Some(submitted) = self.submission_of(cancelled.child_id) else {
                continue;
            };
            let submitted_qty = submitted.quantity.as_f64();
            if submitted_qty <= 0.0 {
                continue;
            }
            let filled: f64 = self
                .fills
                .iter()
                .filter(|fill| fill.child_id == cancelled.child_id)
                .map(|fill| fill.quantity.as_f64())
                .sum();
            total += filled / submitted_qty;
            count += 1;
        }
        (count > 0).then(|| total / f64::from(count))
    }

    /// Returns the mean price improvement per fill against the parent's own limit price.
    fn price_improvement(&self) -> Option<f64> {
        let limit = self.terms.limit_price?.as_f64();
        if self.fills.is_empty() {
            return None;
        }
        let mut total = 0.0;
        for fill in &self.fills {
            let price = fill.price.as_f64();
            total += match self.terms.order_side {
                OrderSide::Buy => limit - price,
                OrderSide::Sell => price - limit,
            };
        }
        Some(total / self.fills.len() as f64)
    }

    /// Returns the first submission observed for `child_id`.
    fn submission_of(&self, child_id: ClientOrderId) -> Option<&ChildObservation> {
        self.child_submissions
            .iter()
            .find(|submitted| submitted.child_id == child_id)
    }
}

/// A thin [`DataActor`] that feeds an [`ExecutionObserver`] from the real event stream.
///
/// On start it subscribes to quotes and trades for the observer's instrument and to the order
/// events for the parent's strategy. Each event is forwarded to the observer; the actor performs
/// no metric arithmetic of its own. It is read-only: it never submits, modifies or cancels an
/// order and keeps no ledger beyond the observer's window.
pub struct ExecutionAnalyticsCollector {
    core: DataActorCore,
    observer: ExecutionObserver,
    strategy_id: StrategyId,
    parent_client_order_id: ClientOrderId,
    order_topic: String,
    order_handler: Option<TypedHandler<OrderEventAny>>,
}

impl ExecutionAnalyticsCollector {
    /// Creates a new collector for the parent order owned by `observer`.
    ///
    /// `actor_id` sets the actor identifier when supplied.
    #[must_use]
    pub fn new(
        observer: ExecutionObserver,
        strategy_id: StrategyId,
        parent_client_order_id: ClientOrderId,
        actor_id: Option<ActorId>,
    ) -> Self {
        let config = DataActorConfig {
            actor_id: Some(actor_id.unwrap_or_else(|| ActorId::from("EXEC_ANALYTICS-001"))),
            ..Default::default()
        };
        let order_topic = format!("events.order.{strategy_id}");
        Self {
            core: DataActorCore::new(config),
            observer,
            strategy_id,
            parent_client_order_id,
            order_topic,
            order_handler: None,
        }
    }

    /// Returns the observer being fed.
    #[must_use]
    pub fn observer(&self) -> &ExecutionObserver {
        &self.observer
    }

    /// Returns the metric set for the observations received so far.
    #[must_use]
    pub fn metrics(&self) -> ExecutionMetrics {
        self.observer.metrics()
    }

    /// Returns the strategy whose order events are tracked.
    #[must_use]
    pub fn strategy_id(&self) -> StrategyId {
        self.strategy_id
    }

    /// Returns the parent client order ID being tracked.
    #[must_use]
    pub fn parent_client_order_id(&self) -> ClientOrderId {
        self.parent_client_order_id
    }

    /// Feeds an order event for the parent or one of its children into the observer.
    pub fn handle_order_event(&mut self, event: &OrderEventAny) {
        let client_order_id = event.client_order_id();

        if client_order_id == self.parent_client_order_id {
            if let OrderEventAny::Submitted(submitted) = event {
                self.observer.set_parent_submitted(submitted.ts_event);
            }
            return;
        }

        let order = self.cache().order(&client_order_id);

        let Some(order) = order else {
            return;
        };

        if order.exec_spawn_id() != Some(self.parent_client_order_id) {
            return;
        }

        match event {
            OrderEventAny::Submitted(submitted) => {
                self.observer.observe_child_submitted(
                    client_order_id,
                    submitted.ts_event,
                    order.quantity(),
                );
            }
            OrderEventAny::Canceled(canceled) => {
                self.observer.observe_child_canceled(
                    client_order_id,
                    canceled.ts_event,
                    order.leaves_qty(),
                );
            }
            OrderEventAny::Filled(filled) => {
                self.observer.observe_fill(
                    client_order_id,
                    filled.ts_event,
                    filled.last_qty,
                    filled.last_px,
                );
            }
            _ => {}
        }
    }
}

nautilus_actor!(ExecutionAnalyticsCollector);

impl Debug for ExecutionAnalyticsCollector {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(ExecutionAnalyticsCollector))
            .field("strategy_id", &self.strategy_id)
            .field("parent_client_order_id", &self.parent_client_order_id)
            .field("order_topic", &self.order_topic)
            .finish()
    }
}

impl DataActor for ExecutionAnalyticsCollector {
    fn on_start(&mut self) -> anyhow::Result<()> {
        let instrument_id = self.observer.terms().instrument_id;
        self.subscribe_quotes(instrument_id, None, None);
        self.subscribe_trades(instrument_id, None, None);

        let actor_id = self.core().actor_id().inner();
        let handler = TypedHandler::from(move |event: &OrderEventAny| {
            if let Some(mut actor) = try_get_actor_unchecked::<Self>(&actor_id) {
                actor.handle_order_event(event);
            } else {
                log::error!(
                    "ExecutionAnalyticsCollector {actor_id} not found for order event handling"
                );
            }
        });
        msgbus::subscribe_order_events(self.order_topic.clone().into(), handler.clone(), None);
        self.order_handler = Some(handler);
        Ok(())
    }

    fn on_stop(&mut self) -> anyhow::Result<()> {
        if let Some(handler) = self.order_handler.take() {
            msgbus::unsubscribe_order_events(self.order_topic.clone().into(), &handler);
        }
        let instrument_id = self.observer.terms().instrument_id;
        DataActor::unsubscribe_quotes(self, instrument_id, None, None);
        DataActor::unsubscribe_trades(self, instrument_id, None, None);
        Ok(())
    }

    fn on_quote(&mut self, quote: &QuoteTick) -> anyhow::Result<()> {
        self.observer.observe_quote_tick(quote);
        Ok(())
    }

    fn on_trade(&mut self, tick: &TradeTick) -> anyhow::Result<()> {
        self.observer.observe_trade_tick(tick);
        Ok(())
    }
}
