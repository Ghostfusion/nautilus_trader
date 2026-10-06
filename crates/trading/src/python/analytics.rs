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

//! Python bindings for the read-only execution analytics surface.
//!
//! A caller declares the parent order's terms, feeds an [`ExecutionObserver`] the same
//! observations the Rust API takes, and reads the metric set back. The observer is a pure
//! accumulator: it holds no clock, cache or message bus, so a backtest can drive it from its own
//! event stream and the engine keeps no state on its behalf. `observe_quote_tick` and
//! `observe_trade_tick` take the model's ticks directly, so an existing data flow needs no
//! conversion.
//!
//! Every metric is exposed as a [`Metric`] carrying both the number and the declaration that
//! defines it, so a value cannot be read without also reading the price, denominator and timestamp
//! it was measured against. An undefined metric reports `None` with its reason on `Metric.reason`,
//! never `0.0`; a genuine zero stays a real `0.0`. `MetricValue` is not bound as a class because
//! `value` and `reason` together carry the same distinction.
//!
//! Timestamps and horizons cross as integer nanoseconds, matching the rest of the package. The
//! metric vocabulary (`MetricUnits`, `MetricDirection`, and the rest) is exposed under its Rust
//! names, as a distinct set of types from the ones `nautilus_trader.analysis` exports for the
//! performance report: the two vocabularies share short names and do not share variants, and
//! passing one for the other is a type error rather than a silent conversion.
//!
//! The bus-integrated [`ExecutionAnalyticsCollector`](crate::analytics::ExecutionAnalyticsCollector)
//! is not bound: registering a Rust actor from Python needs an actor-registration path this crate
//! does not own, and every metric it forwards is reachable through the observer.

use nautilus_core::{DurationNanos, UnixNanos};
use nautilus_model::{
    data::{QuoteTick, TradeTick},
    enums::OrderSide,
    identifiers::{ClientOrderId, InstrumentId},
    types::{Price, Quantity},
};
use pyo3_stub_gen::derive::gen_stub_pymethods;

use crate::analytics::{
    BenchmarkInterval, ChildObservation, DenominatorSource, ExecutionMetrics, ExecutionObserver,
    ExecutionTerms, FillObservation, Metric, MetricDeclaration, MetricDirection, MetricUnits,
    QuoteObservation, ReferencePoint, ReferencePriceSource, ReferenceTimestamp, TradeObservation,
    UnavailableReason,
};

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl ExecutionTerms {
    /// The parent order's terms, as declared by the caller.
    #[new]
    #[pyo3(signature = (instrument_id, order_side, quantity, limit_price=None, horizon_ns=None))]
    fn py_new(
        instrument_id: InstrumentId,
        order_side: OrderSide,
        quantity: Quantity,
        limit_price: Option<Price>,
        horizon_ns: Option<u64>,
    ) -> Self {
        Self {
            instrument_id,
            order_side,
            quantity,
            limit_price,
            horizon: horizon_ns.map(DurationNanos::new),
        }
    }

    /// The instrument the parent executes in.
    #[getter]
    fn instrument_id(&self) -> InstrumentId {
        self.instrument_id
    }

    /// The side of the parent order.
    #[getter]
    fn order_side(&self) -> OrderSide {
        self.order_side
    }

    /// The parent order's target quantity.
    #[getter]
    fn quantity(&self) -> Quantity {
        self.quantity
    }

    /// The parent order's own limit price, when it has one.
    #[getter]
    fn limit_price(&self) -> Option<Price> {
        self.limit_price
    }

    /// The declared horizon in nanoseconds, when the caller declared one.
    #[getter]
    fn horizon_ns(&self) -> Option<u64> {
        self.horizon.map(|horizon| horizon.as_u64())
    }

    fn __repr__(&self) -> String {
        format!("{self:?}")
    }
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl ReferencePoint {
    /// A declared reference price together with the timestamp it belongs to.
    #[new]
    fn py_new(timestamp: u64, price: Price) -> Self {
        Self {
            timestamp: UnixNanos::from(timestamp),
            price,
        }
    }

    /// The timestamp of the reference price, in nanoseconds.
    #[getter]
    fn timestamp(&self) -> u64 {
        self.timestamp.as_u64()
    }

    /// The reference price.
    #[getter]
    fn price(&self) -> Price {
        self.price
    }

    fn __repr__(&self) -> String {
        format!("{self:?}")
    }
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl BenchmarkInterval {
    /// The declared benchmark interval a VWAP or TWAP is formed over.
    #[new]
    fn py_new(start: u64, end: u64) -> Self {
        Self {
            start: UnixNanos::from(start),
            end: UnixNanos::from(end),
        }
    }

    /// The interval start (inclusive), in nanoseconds.
    #[getter]
    fn start(&self) -> u64 {
        self.start.as_u64()
    }

    /// The interval end (inclusive), in nanoseconds.
    #[getter]
    fn end(&self) -> u64 {
        self.end.as_u64()
    }

    /// Returns whether the timestamp in nanoseconds lies within the interval.
    #[pyo3(name = "contains")]
    fn py_contains(&self, timestamp: u64) -> bool {
        self.contains(UnixNanos::from(timestamp))
    }

    fn __repr__(&self) -> String {
        format!("{self:?}")
    }
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl QuoteObservation {
    /// A top-of-book observation used for midpoint and spread metrics.
    #[new]
    fn py_new(timestamp: u64, bid_price: Price, ask_price: Price) -> Self {
        Self {
            timestamp: UnixNanos::from(timestamp),
            bid_price,
            ask_price,
        }
    }

    /// The observation timestamp, in nanoseconds.
    #[getter]
    fn timestamp(&self) -> u64 {
        self.timestamp.as_u64()
    }

    /// The quoted bid price.
    #[getter]
    fn bid_price(&self) -> Price {
        self.bid_price
    }

    /// The quoted ask price.
    #[getter]
    fn ask_price(&self) -> Price {
        self.ask_price
    }

    /// Returns the quoted midpoint.
    #[pyo3(name = "mid")]
    fn py_mid(&self) -> f64 {
        self.mid()
    }

    /// Returns the quoted half-spread.
    #[pyo3(name = "half_spread")]
    fn py_half_spread(&self) -> f64 {
        self.half_spread()
    }

    fn __repr__(&self) -> String {
        format!("{self:?}")
    }
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl TradeObservation {
    /// A trade observation used for benchmark VWAP.
    #[new]
    fn py_new(timestamp: u64, price: Price, size: Quantity) -> Self {
        Self {
            timestamp: UnixNanos::from(timestamp),
            price,
            size,
        }
    }

    /// The observation timestamp, in nanoseconds.
    #[getter]
    fn timestamp(&self) -> u64 {
        self.timestamp.as_u64()
    }

    /// The traded price.
    #[getter]
    fn price(&self) -> Price {
        self.price
    }

    /// The traded size.
    #[getter]
    fn size(&self) -> Quantity {
        self.size
    }

    fn __repr__(&self) -> String {
        format!("{self:?}")
    }
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl FillObservation {
    /// A fill observation attributed to a child order.
    #[new]
    fn py_new(child_id: ClientOrderId, timestamp: u64, quantity: Quantity, price: Price) -> Self {
        Self {
            child_id,
            timestamp: UnixNanos::from(timestamp),
            quantity,
            price,
        }
    }

    /// The client order ID of the child that filled.
    #[getter]
    fn child_id(&self) -> ClientOrderId {
        self.child_id
    }

    /// The fill timestamp, in nanoseconds.
    #[getter]
    fn timestamp(&self) -> u64 {
        self.timestamp.as_u64()
    }

    /// The fill quantity.
    #[getter]
    fn quantity(&self) -> Quantity {
        self.quantity
    }

    /// The fill price.
    #[getter]
    fn price(&self) -> Price {
        self.price
    }

    fn __repr__(&self) -> String {
        format!("{self:?}")
    }
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl ChildObservation {
    /// A child submission or cancellation observation.
    ///
    /// For a submission the quantity is the submitted quantity, and for a cancellation it is the
    /// unfilled quantity the cancellation closes out.
    #[new]
    fn py_new(child_id: ClientOrderId, timestamp: u64, quantity: Quantity) -> Self {
        Self {
            child_id,
            timestamp: UnixNanos::from(timestamp),
            quantity,
        }
    }

    /// The client order ID of the child.
    #[getter]
    fn child_id(&self) -> ClientOrderId {
        self.child_id
    }

    /// The event timestamp, in nanoseconds.
    #[getter]
    fn timestamp(&self) -> u64 {
        self.timestamp.as_u64()
    }

    /// The quantity the observation refers to.
    #[getter]
    fn quantity(&self) -> Quantity {
        self.quantity
    }

    fn __repr__(&self) -> String {
        format!("{self:?}")
    }
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl MetricDeclaration {
    /// The stable metric identifier.
    #[getter]
    fn metric_id(&self) -> String {
        self.metric_id.to_string()
    }

    /// The units the value is expressed in.
    #[getter]
    fn units(&self) -> MetricUnits {
        self.units
    }

    /// The direction the value is better in.
    #[getter]
    fn direction(&self) -> MetricDirection {
        self.direction
    }

    /// The reference price the value is measured against, when it has one.
    #[getter]
    fn reference_price(&self) -> Option<ReferencePriceSource> {
        self.reference_price
    }

    /// What the value is normalised against.
    #[getter]
    fn denominator(&self) -> DenominatorSource {
        self.denominator
    }

    /// The timestamp the value is measured from, when it has one.
    #[getter]
    fn reference_timestamp(&self) -> Option<ReferenceTimestamp> {
        self.reference_timestamp
    }

    /// The declared horizon in nanoseconds, when the metric depends on one.
    #[getter]
    fn horizon_ns(&self) -> Option<u64> {
        self.horizon.map(|horizon| horizon.as_u64())
    }

    fn __repr__(&self) -> String {
        format!("{self:?}")
    }
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl Metric {
    /// The declaration that makes the value meaningful.
    #[getter]
    fn declaration(&self) -> MetricDeclaration {
        self.declaration
    }

    /// The value when the metric is defined, or `None` when it is unavailable.
    #[getter]
    #[pyo3(name = "value")]
    fn py_value(&self) -> Option<f64> {
        self.value()
    }

    /// Why the metric has no value, or `None` when it is available.
    #[getter]
    fn reason(&self) -> Option<UnavailableReason> {
        self.value.unavailable_reason()
    }

    /// Returns whether the metric is defined.
    #[pyo3(name = "is_available")]
    fn py_is_available(&self) -> bool {
        self.value.is_available()
    }

    fn __repr__(&self) -> String {
        format!("{self:?}")
    }
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl ExecutionMetrics {
    /// Implementation shortfall against the decision price, in basis points.
    #[getter]
    fn implementation_shortfall_bps(&self) -> Metric {
        self.implementation_shortfall_bps
    }

    /// Arrival slippage against the arrival price, in basis points.
    #[getter]
    fn arrival_slippage_bps(&self) -> Metric {
        self.arrival_slippage_bps
    }

    /// Decision price slippage (the delay component), in basis points.
    #[getter]
    fn decision_price_slippage_bps(&self) -> Metric {
        self.decision_price_slippage_bps
    }

    /// Delay from the declared decision timestamp to the first fill, in seconds.
    #[getter]
    fn decision_to_execution_delay_s(&self) -> Metric {
        self.decision_to_execution_delay_s
    }

    /// VWAP slippage against the benchmark interval, in basis points.
    #[getter]
    fn vwap_slippage_bps(&self) -> Metric {
        self.vwap_slippage_bps
    }

    /// TWAP slippage against the benchmark interval, in basis points.
    #[getter]
    fn twap_slippage_bps(&self) -> Metric {
        self.twap_slippage_bps
    }

    /// Midpoint slippage at the stated timestamp, in basis points.
    #[getter]
    fn midpoint_slippage_bps(&self) -> Metric {
        self.midpoint_slippage_bps
    }

    /// Spread capture against the quoted half-spread.
    #[getter]
    fn spread_capture(&self) -> Metric {
        self.spread_capture
    }

    /// Adverse selection over the declared horizon.
    #[getter]
    fn adverse_selection(&self) -> Metric {
        self.adverse_selection
    }

    /// Fill ratio against the parent target quantity.
    #[getter]
    fn fill_ratio(&self) -> Metric {
        self.fill_ratio
    }

    /// Cancel ratio against the submitted child quantity.
    #[getter]
    fn cancel_ratio(&self) -> Metric {
        self.cancel_ratio
    }

    /// Completion time from the parent submission, in seconds.
    #[getter]
    fn completion_time_s(&self) -> Metric {
        self.completion_time_s
    }

    /// The number of submitted child orders.
    #[getter]
    fn child_count(&self) -> Metric {
        self.child_count
    }

    /// Child churn (cancelled over submitted).
    #[getter]
    fn child_churn(&self) -> Metric {
        self.child_churn
    }

    /// Mean child lifetime, in seconds.
    #[getter]
    fn mean_child_lifetime_s(&self) -> Metric {
        self.mean_child_lifetime_s
    }

    /// Partial fill ratio of cancelled children.
    #[getter]
    fn partial_fill_ratio(&self) -> Metric {
        self.partial_fill_ratio
    }

    /// Price improvement against the order's own limit price.
    #[getter]
    fn price_improvement(&self) -> Metric {
        self.price_improvement
    }

    fn __repr__(&self) -> String {
        format!("{self:?}")
    }
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl ExecutionObserver {
    /// Creates a new observer for the declared parent terms.
    #[new]
    fn py_new(terms: ExecutionTerms) -> Self {
        Self::new(terms)
    }

    /// The declared parent terms.
    #[getter]
    #[pyo3(name = "terms")]
    fn py_terms(&self) -> ExecutionTerms {
        self.terms().clone()
    }

    /// The declared benchmark interval, or `None` when none was declared.
    #[getter]
    #[pyo3(name = "benchmark")]
    fn py_benchmark(&self) -> Option<BenchmarkInterval> {
        self.benchmark()
    }

    /// Returns a copy of this observer with the benchmark interval set.
    #[pyo3(name = "with_benchmark")]
    fn py_with_benchmark(&self, interval: BenchmarkInterval) -> Self {
        self.clone().with_benchmark(interval)
    }

    /// Sets the decision timestamp in nanoseconds and the decision price.
    #[pyo3(name = "set_decision")]
    fn py_set_decision(&mut self, timestamp: u64, price: Price) {
        self.set_decision(UnixNanos::from(timestamp), price);
    }

    /// Sets the arrival timestamp in nanoseconds and the arrival price.
    #[pyo3(name = "set_arrival")]
    fn py_set_arrival(&mut self, timestamp: u64, price: Price) {
        self.set_arrival(UnixNanos::from(timestamp), price);
    }

    /// Sets the parent order's submission timestamp in nanoseconds.
    #[pyo3(name = "set_parent_submitted")]
    fn py_set_parent_submitted(&mut self, timestamp: u64) {
        self.set_parent_submitted(UnixNanos::from(timestamp));
    }

    /// Records the child counts reported by the execution algorithm that produced this execution.
    #[pyo3(name = "set_algorithm_child_counts")]
    fn py_set_algorithm_child_counts(&mut self, submitted: u32, cancelled: u32) {
        self.set_algorithm_child_counts(submitted, cancelled);
    }

    /// Records a child submission.
    #[pyo3(name = "observe_child_submitted")]
    fn py_observe_child_submitted(
        &mut self,
        child_id: ClientOrderId,
        timestamp: u64,
        quantity: Quantity,
    ) {
        self.observe_child_submitted(child_id, UnixNanos::from(timestamp), quantity);
    }

    /// Records a child cancellation with the unfilled quantity it closes out.
    #[pyo3(name = "observe_child_canceled")]
    fn py_observe_child_canceled(
        &mut self,
        child_id: ClientOrderId,
        timestamp: u64,
        quantity: Quantity,
    ) {
        self.observe_child_canceled(child_id, UnixNanos::from(timestamp), quantity);
    }

    /// Records a fill.
    #[pyo3(name = "observe_fill")]
    fn py_observe_fill(
        &mut self,
        child_id: ClientOrderId,
        timestamp: u64,
        quantity: Quantity,
        price: Price,
    ) {
        self.observe_fill(child_id, UnixNanos::from(timestamp), quantity, price);
    }

    /// Records a quote observation.
    #[pyo3(name = "observe_quote")]
    fn py_observe_quote(&mut self, observation: QuoteObservation) {
        self.observe_quote(observation);
    }

    /// Records a trade observation.
    #[pyo3(name = "observe_trade")]
    fn py_observe_trade(&mut self, observation: TradeObservation) {
        self.observe_trade(observation);
    }

    /// Records a quote tick.
    #[pyo3(name = "observe_quote_tick")]
    fn py_observe_quote_tick(&mut self, quote: QuoteTick) {
        self.observe_quote_tick(&quote);
    }

    /// Records a trade tick.
    #[pyo3(name = "observe_trade_tick")]
    fn py_observe_trade_tick(&mut self, trade: TradeTick) {
        self.observe_trade_tick(&trade);
    }

    /// Returns the full metric set for the observations received so far.
    #[pyo3(name = "metrics")]
    fn py_metrics(&self) -> ExecutionMetrics {
        self.metrics()
    }
}
