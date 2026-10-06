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

//! Read-only execution analytics.
//!
//! This module observes an execution and reports what happened; it never changes it. It is
//! read-only by construction: it does not submit, modify or cancel orders, it keeps no parallel
//! ledger and it holds no state beyond the observation window. The window is whatever a caller
//! feeds the [`ExecutionObserver`]; once the parent is complete the observer can be discarded.
//!
//! # Declared references
//!
//! A metric value has no meaning without the convention that defines it, so every value carries a
//! [`MetricDeclaration`] naming its units, direction and the reference price, denominator and
//! reference timestamp it was measured against. An undefined metric is reported as
//! [`MetricValue::NotAvailable`] with an [`UnavailableReason`], never as `0.0`; a genuine zero
//! stays a real `Available(0.0)`.
//!
//! # Composition
//!
//! - [`reference`] holds the declared reference metadata.
//! - [`metrics`] holds the metric values and their arithmetic.
//! - [`observation`] holds the pure accumulator and the thin [`DataActor`](nautilus_common::actor::DataActor)
//!   collector that feeds it from the real event stream.

pub mod metrics;
pub mod observation;
pub mod reference;

pub use metrics::{
    ExecutionMetrics, METRIC_ADVERSE_SELECTION, METRIC_ARRIVAL_SLIPPAGE_BPS, METRIC_CANCEL_RATIO,
    METRIC_CHILD_CHURN, METRIC_CHILD_COUNT, METRIC_COMPLETION_TIME_S,
    METRIC_DECISION_PRICE_SLIPPAGE_BPS, METRIC_DECISION_TO_EXECUTION_DELAY_S, METRIC_FILL_RATIO,
    METRIC_IMPLEMENTATION_SHORTFALL_BPS, METRIC_MEAN_CHILD_LIFETIME_S,
    METRIC_MIDPOINT_SLIPPAGE_BPS, METRIC_PARTIAL_FILL_RATIO, METRIC_PRICE_IMPROVEMENT,
    METRIC_SPREAD_CAPTURE, METRIC_TWAP_SLIPPAGE_BPS, METRIC_VWAP_SLIPPAGE_BPS, Metric, MetricValue,
    UnavailableReason,
};
pub use observation::{
    BenchmarkInterval, ChildObservation, ExecutionAnalyticsCollector, ExecutionObserver,
    ExecutionTerms, FillObservation, QuoteObservation, ReferencePoint, TradeObservation,
};
pub use reference::{
    DenominatorSource, MetricDeclaration, MetricDirection, MetricUnits, ReferencePriceSource,
    ReferenceTimestamp,
};

#[cfg(test)]
mod tests {
    use nautilus_core::{DurationNanos, UnixNanos};
    use nautilus_model::{
        enums::OrderSide,
        identifiers::{ClientOrderId, InstrumentId},
        types::{Price, Quantity},
    };

    use super::{
        BenchmarkInterval, DenominatorSource, ExecutionObserver, ExecutionTerms,
        METRIC_DECISION_TO_EXECUTION_DELAY_S, MetricDirection, MetricUnits, MetricValue,
        QuoteObservation, ReferenceTimestamp, TradeObservation, UnavailableReason,
    };

    const HORIZON: DurationNanos = DurationNanos::from_millis(60);

    fn instrument_id() -> InstrumentId {
        InstrumentId::from("EUR/USD.PARITY")
    }

    /// Asserts two metric values are equal to within a small tolerance.
    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1e-9,
            "expected {expected}, was {actual}"
        );
    }

    /// Builds the fully pinned scenario used by the main test.
    ///
    /// Terms: BUY 100 at no limit, horizon 60 milliseconds.
    /// Decision 100.00, arrival 101.00, parent submitted at 1.8s.
    /// Benchmark interval [2.0s, 2.3s]: trades 100.00 and 102.00 of equal size give VWAP 101.00,
    /// quotes with midpoints 100.50 and 101.50 give TWAP 101.00.
    /// Fills: 50 at 102.00 and 50 at 102.02 give VWAP 102.01.
    fn pinned_observer() -> ExecutionObserver {
        let terms = ExecutionTerms {
            instrument_id: instrument_id(),
            order_side: OrderSide::Buy,
            quantity: Quantity::from(100u64),
            limit_price: None,
            horizon: Some(HORIZON),
        };
        let mut observer = ExecutionObserver::new(terms).with_benchmark(BenchmarkInterval {
            start: UnixNanos::from(2_000_000_000u64),
            end: UnixNanos::from(2_300_000_000u64),
        });

        observer.set_parent_submitted(UnixNanos::from(1_800_000_000u64));
        observer.set_decision(UnixNanos::from(1_850_000_000u64), Price::new(100.00, 2));
        observer.set_arrival(UnixNanos::from(1_900_000_000u64), Price::new(101.00, 2));

        // Arrival midpoint: (100.50 + 101.50) / 2 = 101.00, half-spread (101.50 - 100.50) / 2 = 0.50.
        observer.observe_quote(QuoteObservation {
            timestamp: UnixNanos::from(1_900_000_000u64),
            bid_price: Price::new(100.50, 2),
            ask_price: Price::new(101.50, 2),
        });
        // Benchmark quotes: midpoints 100.50 and 101.50, TWAP = (100.50 + 101.50) / 2 = 101.00.
        observer.observe_quote(QuoteObservation {
            timestamp: UnixNanos::from(2_000_000_000u64),
            bid_price: Price::new(100.00, 2),
            ask_price: Price::new(101.00, 2),
        });
        observer.observe_quote(QuoteObservation {
            timestamp: UnixNanos::from(2_300_000_000u64),
            bid_price: Price::new(101.00, 2),
            ask_price: Price::new(102.00, 2),
        });
        // Benchmark trades: (100.00 * 1 + 102.00 * 1) / 2 = 101.00.
        observer.observe_trade(TradeObservation {
            timestamp: UnixNanos::from(2_000_000_000u64),
            price: Price::new(100.00, 2),
            size: Quantity::from(1u64),
        });
        observer.observe_trade(TradeObservation {
            timestamp: UnixNanos::from(2_300_000_000u64),
            price: Price::new(102.00, 2),
            size: Quantity::from(1u64),
        });

        let c1 = ClientOrderId::from("C1");
        let c2 = ClientOrderId::from("C2");
        observer.observe_child_submitted(
            c1,
            UnixNanos::from(2_350_000_000u64),
            Quantity::from(50u64),
        );
        observer.observe_fill(
            c1,
            UnixNanos::from(2_400_000_000u64),
            Quantity::from(50u64),
            Price::new(102.00, 2),
        );
        observer.observe_child_submitted(
            c2,
            UnixNanos::from(2_550_000_000u64),
            Quantity::from(50u64),
        );
        observer.observe_fill(
            c2,
            UnixNanos::from(2_600_000_000u64),
            Quantity::from(50u64),
            Price::new(102.02, 2),
        );

        // Quotes for adverse selection: fill 1 at 2.4s sees mid 100.50, at 2.46s sees 100.00;
        // fill 2 at 2.6s sees 101.50, at 2.66s sees 101.00.
        observer.observe_quote(QuoteObservation {
            timestamp: UnixNanos::from(2_400_000_000u64),
            bid_price: Price::new(100.00, 2),
            ask_price: Price::new(101.00, 2),
        });
        observer.observe_quote(QuoteObservation {
            timestamp: UnixNanos::from(2_460_000_000u64),
            bid_price: Price::new(99.50, 2),
            ask_price: Price::new(100.50, 2),
        });
        observer.observe_quote(QuoteObservation {
            timestamp: UnixNanos::from(2_600_000_000u64),
            bid_price: Price::new(101.00, 2),
            ask_price: Price::new(102.00, 2),
        });
        observer.observe_quote(QuoteObservation {
            timestamp: UnixNanos::from(2_660_000_000u64),
            bid_price: Price::new(100.50, 2),
            ask_price: Price::new(101.50, 2),
        });

        observer
    }

    #[test]
    fn pinned_metrics_match_hand_computed_values() {
        let metrics = pinned_observer().metrics();

        // fill VWAP = (102.00 * 50 + 102.02 * 50) / 100 = 102.01.
        // implementation shortfall vs decision 100.00:
        //   ((102.01 - 100.00) / 100.00) * 10_000 = 201 bps.
        assert_close(metrics.implementation_shortfall_bps.value().unwrap(), 201.0);

        // arrival slippage vs arrival 101.00:
        //   ((102.01 - 101.00) / 101.00) * 10_000 = 100 bps.
        assert_close(metrics.arrival_slippage_bps.value().unwrap(), 100.0);

        // decision price slippage over the delay from 100.00 to 101.00:
        //   ((101.00 - 100.00) / 100.00) * 10_000 = 100 bps.
        assert_close(metrics.decision_price_slippage_bps.value().unwrap(), 100.0);

        // decision-to-execution delay measures to the *first* fill: 2.4s - 1.85s = 0.55 seconds,
        // not the last fill (2.6s, which would be 0.75).
        assert_close(metrics.decision_to_execution_delay_s.value().unwrap(), 0.55);

        // interval VWAP = (100.00 * 1 + 102.00 * 1) / 2 = 101.00:
        //   ((102.01 - 101.00) / 101.00) * 10_000 = 100 bps.
        assert_close(metrics.vwap_slippage_bps.value().unwrap(), 100.0);

        // interval TWAP = (100.50 + 101.50) / 2 = 101.00:
        //   ((102.01 - 101.00) / 101.00) * 10_000 = 100 bps.
        assert_close(metrics.twap_slippage_bps.value().unwrap(), 100.0);

        // midpoint slippage vs the arrival midpoint 101.00:
        //   ((102.01 - 101.00) / 101.00) * 10_000 = 100 bps.
        assert_close(metrics.midpoint_slippage_bps.value().unwrap(), 100.0);

        // spread capture vs arrival mid 101.00 and half-spread 0.50:
        //   (101.00 - 102.01) / 0.50 = -2.02 (bought above the mid, so capture is negative).
        assert_close(metrics.spread_capture.value().unwrap(), -2.02);

        // fill ratio = filled 100 / parent target 100 = 1.0.
        assert_close(metrics.fill_ratio.value().unwrap(), 1.0);

        // cancel ratio = cancelled 0 / submitted 100 = 0.0 (a genuine zero, not an absence).
        assert_eq!(metrics.cancel_ratio.value(), Some(0.0));

        // completion time = (2.6s - 1.8s) = 0.8 seconds.
        assert_close(metrics.completion_time_s.value().unwrap(), 0.8);

        // child count = two children submitted.
        assert_close(metrics.child_count.value().unwrap(), 2.0);

        // child churn = cancelled 0 / submitted 2 = 0.0.
        assert_eq!(metrics.child_churn.value(), Some(0.0));

        // no orders were cancelled, so the lifetime and partial fill metrics are undefined.
        assert_eq!(
            metrics.mean_child_lifetime_s.value,
            MetricValue::NotAvailable(UnavailableReason::NoObservations)
        );
        assert_eq!(
            metrics.partial_fill_ratio.value,
            MetricValue::NotAvailable(UnavailableReason::NoObservations)
        );
    }

    #[test]
    fn decision_to_execution_delay_declares_what_it_measures() {
        let metric = pinned_observer().metrics().decision_to_execution_delay_s;

        assert_eq!(
            metric.declaration.metric_id,
            METRIC_DECISION_TO_EXECUTION_DELAY_S
        );
        assert_eq!(metric.declaration.units, MetricUnits::Seconds);
        assert_eq!(metric.declaration.direction, MetricDirection::LowerIsBetter);
        assert_eq!(
            metric.declaration.reference_timestamp,
            Some(ReferenceTimestamp::Decision)
        );
        assert_eq!(
            metric.declaration.denominator,
            DenominatorSource::NotApplicable
        );
    }

    #[test]
    fn decision_to_execution_delay_reports_the_missing_side() {
        let terms = ExecutionTerms {
            instrument_id: instrument_id(),
            order_side: OrderSide::Buy,
            quantity: Quantity::from(10u64),
            limit_price: None,
            horizon: None,
        };

        // A fill with no declared decision has no instant to measure the delay from.
        let mut executed_without_decision = ExecutionObserver::new(terms.clone());
        executed_without_decision.observe_fill(
            ClientOrderId::from("F1"),
            UnixNanos::from(2_000_000_000u64),
            Quantity::from(10u64),
            Price::new(100.00, 2),
        );
        assert_eq!(
            executed_without_decision
                .metrics()
                .decision_to_execution_delay_s
                .value,
            MetricValue::NotAvailable(UnavailableReason::NoTimestamp)
        );

        // A declared decision with nothing executed yet has no delay to report.
        let mut decision_without_fill = ExecutionObserver::new(terms.clone());
        decision_without_fill
            .set_decision(UnixNanos::from(1_900_000_000u64), Price::new(100.00, 2));
        assert_eq!(
            decision_without_fill
                .metrics()
                .decision_to_execution_delay_s
                .value,
            MetricValue::NotAvailable(UnavailableReason::NoObservations)
        );

        // A fill recorded before the decision cannot make the delay negative; it is a real zero.
        let mut fill_before_decision = ExecutionObserver::new(terms);
        fill_before_decision.set_decision(UnixNanos::from(2_100_000_000u64), Price::new(100.00, 2));
        fill_before_decision.observe_fill(
            ClientOrderId::from("F1"),
            UnixNanos::from(2_000_000_000u64),
            Quantity::from(10u64),
            Price::new(100.00, 2),
        );
        assert_eq!(
            fill_before_decision
                .metrics()
                .decision_to_execution_delay_s
                .value(),
            Some(0.0)
        );
    }

    #[test]
    fn adverse_selection_uses_fill_plus_horizon_reference() {
        let metrics = pinned_observer().metrics();
        let metric = metrics.adverse_selection;

        // fill 1: mid 100.50 at 2.4s, mid 100.00 at 2.46s -> 100.50 - 100.00 = +0.50 adverse.
        // fill 2: mid 101.50 at 2.6s, mid 101.00 at 2.66s -> 101.50 - 101.00 = +0.50 adverse.
        // mean over the two fills = (0.50 + 0.50) / 2 = 0.50.
        assert_close(metric.value().unwrap(), 0.50);
        assert_eq!(
            metric.declaration.reference_timestamp,
            Some(ReferenceTimestamp::FillPlusHorizon)
        );
        assert_eq!(metric.declaration.horizon, Some(HORIZON));
    }

    #[test]
    fn undefined_metric_is_not_available_never_zero() {
        let metrics = pinned_observer().metrics();

        // No parent limit price was declared, so price improvement is undefined.
        assert_eq!(
            metrics.price_improvement.value,
            MetricValue::NotAvailable(UnavailableReason::NoReferencePrice)
        );
        assert_eq!(metrics.price_improvement.value(), None);
        assert!(!metrics.price_improvement.value.is_available());
        assert_eq!(
            metrics.price_improvement.value.unavailable_reason(),
            Some(UnavailableReason::NoReferencePrice)
        );
    }

    #[test]
    fn passive_fill_spread_capture_is_positive() {
        let terms = ExecutionTerms {
            instrument_id: instrument_id(),
            order_side: OrderSide::Buy,
            quantity: Quantity::from(10u64),
            limit_price: None,
            horizon: None,
        };
        let mut observer = ExecutionObserver::new(terms);

        // Arrival midpoint 101.00 with half-spread 0.50.
        observer.observe_quote(QuoteObservation {
            timestamp: UnixNanos::from(1_000_000_000u64),
            bid_price: Price::new(100.50, 2),
            ask_price: Price::new(101.50, 2),
        });
        observer.set_arrival(UnixNanos::from(1_000_000_000u64), Price::new(101.00, 2));

        // A passive BUY resting at the bid fills at 100.50: (101.00 - 100.50) / 0.50 = +1.0.
        observer.observe_fill(
            ClientOrderId::from("P1"),
            UnixNanos::from(1_100_000_000u64),
            Quantity::from(10u64),
            Price::new(100.50, 2),
        );

        let metrics = observer.metrics();
        assert_close(metrics.spread_capture.value().unwrap(), 1.0);
        assert!(metrics.spread_capture.value().unwrap() > 0.0);
    }

    #[test]
    fn adverse_selection_requires_a_declared_horizon() {
        let terms = ExecutionTerms {
            instrument_id: instrument_id(),
            order_side: OrderSide::Buy,
            quantity: Quantity::from(10u64),
            limit_price: None,
            horizon: None,
        };
        let observer = ExecutionObserver::new(terms);
        assert_eq!(
            observer.metrics().adverse_selection.value,
            MetricValue::NotAvailable(UnavailableReason::NoHorizon)
        );
    }

    #[test]
    fn algorithm_child_counts_override_observed_counts() {
        let mut observer = pinned_observer();
        observer.set_algorithm_child_counts(7, 3);
        let metrics = observer.metrics();

        // The algorithm reported 7 submitted and 3 cancelled, so the count metrics use those.
        assert_close(metrics.child_count.value().unwrap(), 7.0);
        assert_close(metrics.child_churn.value().unwrap(), 3.0 / 7.0);

        // The quantity-based cancel ratio still works from the observed child events.
        assert_eq!(metrics.cancel_ratio.value(), Some(0.0));
    }

    #[test]
    fn observer_is_send_and_sync_friendly() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ExecutionObserver>();
    }
}
