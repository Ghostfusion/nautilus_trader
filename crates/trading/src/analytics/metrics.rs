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

//! Execution metric values and their arithmetic.
//!
//! A value is only ever produced together with the [`MetricDeclaration`] that defines it, and an
//! undefined value is reported as [`MetricValue::NotAvailable`] rather than as `0.0`. A genuine
//! zero (for example a cancel ratio for an execution with no cancellations) stays a real
//! `Available(0.0)`, so an absence can never be mistaken for a measured zero.
//!
//! All arithmetic here is performed on `f64`, which is correct for prices and ratios inside a
//! metric. Quantities and money are converted explicitly with `as_f64()` where a plain float is
//! required, and any notional-weighted average is accumulated in `Decimal` before the single
//! conversion, so money is never summed in floating point.

use std::fmt::Display;

use nautilus_core::{DurationNanos, UnixNanos};
use nautilus_model::enums::OrderSide;

use super::reference::{
    DenominatorSource, MetricDeclaration, MetricDirection, MetricUnits, ReferencePriceSource,
    ReferenceTimestamp,
};

/// Metric identifier for the implementation shortfall against the decision price.
pub const METRIC_IMPLEMENTATION_SHORTFALL_BPS: &str = "implementation_shortfall_bps";
/// Metric identifier for the arrival slippage against the arrival price.
pub const METRIC_ARRIVAL_SLIPPAGE_BPS: &str = "arrival_slippage_bps";
/// Metric identifier for the decision price delay slippage.
pub const METRIC_DECISION_PRICE_SLIPPAGE_BPS: &str = "decision_price_slippage_bps";
/// Metric identifier for the benchmark interval VWAP slippage.
pub const METRIC_VWAP_SLIPPAGE_BPS: &str = "vwap_slippage_bps";
/// Metric identifier for the benchmark interval TWAP slippage.
pub const METRIC_TWAP_SLIPPAGE_BPS: &str = "twap_slippage_bps";
/// Metric identifier for the midpoint slippage at the stated timestamp.
pub const METRIC_MIDPOINT_SLIPPAGE_BPS: &str = "midpoint_slippage_bps";
/// Metric identifier for the spread capture against the quoted half-spread.
pub const METRIC_SPREAD_CAPTURE: &str = "spread_capture";
/// Metric identifier for the adverse selection over the declared horizon.
pub const METRIC_ADVERSE_SELECTION: &str = "adverse_selection";
/// Metric identifier for the fill ratio.
pub const METRIC_FILL_RATIO: &str = "fill_ratio";
/// Metric identifier for the cancel ratio.
pub const METRIC_CANCEL_RATIO: &str = "cancel_ratio";
/// Metric identifier for the completion time against the horizon.
pub const METRIC_COMPLETION_TIME_S: &str = "completion_time_s";
/// Metric identifier for the number of submitted children.
pub const METRIC_CHILD_COUNT: &str = "child_count";
/// Metric identifier for the child churn.
pub const METRIC_CHILD_CHURN: &str = "child_churn";
/// Metric identifier for the mean child lifetime.
pub const METRIC_MEAN_CHILD_LIFETIME_S: &str = "mean_child_lifetime_s";
/// Metric identifier for the partial fill ratio.
pub const METRIC_PARTIAL_FILL_RATIO: &str = "partial_fill_ratio";
/// Metric identifier for the price improvement against the order's own limit price.
pub const METRIC_PRICE_IMPROVEMENT: &str = "price_improvement";

/// Why a metric has no value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UnavailableReason {
    /// A reference price the metric is defined against was not declared.
    NoReferencePrice,
    /// A timestamp the metric is measured from was not observed.
    NoTimestamp,
    /// The observation window contained no data the metric could use.
    NoObservations,
    /// The metric depends on a horizon that was not declared.
    NoHorizon,
    /// A denominator was zero, so the ratio is undefined.
    ZeroDenominator,
}

impl Display for UnavailableReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::NoReferencePrice => "no_reference_price",
            Self::NoTimestamp => "no_timestamp",
            Self::NoObservations => "no_observations",
            Self::NoHorizon => "no_horizon",
            Self::ZeroDenominator => "zero_denominator",
        })
    }
}

/// The value of a metric: either an available number or an explicit absence.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MetricValue {
    /// The metric is defined and has the given value.
    Available(f64),
    /// The metric is undefined and has no value.
    NotAvailable(UnavailableReason),
}

impl MetricValue {
    /// Returns the value when it is available.
    #[must_use]
    pub fn value(self) -> Option<f64> {
        match self {
            Self::Available(value) => Some(value),
            Self::NotAvailable(_) => None,
        }
    }

    /// Returns whether the metric is defined.
    #[must_use]
    pub fn is_available(self) -> bool {
        matches!(self, Self::Available(_))
    }

    /// Returns the reason the metric is undefined, when it is.
    #[must_use]
    pub fn unavailable_reason(self) -> Option<UnavailableReason> {
        match self {
            Self::Available(_) => None,
            Self::NotAvailable(reason) => Some(reason),
        }
    }
}

/// A metric value together with the declaration that defines it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Metric {
    /// The declaration that makes the value meaningful.
    pub declaration: MetricDeclaration,
    /// The value, or the reason it is unavailable.
    pub value: MetricValue,
}

impl Metric {
    /// Creates a new [`Metric`].
    #[must_use]
    pub fn new(declaration: MetricDeclaration, value: MetricValue) -> Self {
        Self { declaration, value }
    }

    /// Returns the metric value when it is available.
    #[must_use]
    pub fn value(&self) -> Option<f64> {
        self.value.value()
    }
}

/// The complete set of execution metrics for one parent order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExecutionMetrics {
    /// Implementation shortfall against the decision price, in basis points.
    pub implementation_shortfall_bps: Metric,
    /// Arrival slippage against the arrival price, in basis points.
    pub arrival_slippage_bps: Metric,
    /// Decision price slippage (the delay component), in basis points.
    pub decision_price_slippage_bps: Metric,
    /// VWAP slippage against the benchmark interval, in basis points.
    pub vwap_slippage_bps: Metric,
    /// TWAP slippage against the benchmark interval, in basis points.
    pub twap_slippage_bps: Metric,
    /// Midpoint slippage at the stated timestamp, in basis points.
    pub midpoint_slippage_bps: Metric,
    /// Spread capture against the quoted half-spread.
    pub spread_capture: Metric,
    /// Adverse selection over the declared horizon.
    pub adverse_selection: Metric,
    /// Fill ratio against the parent target quantity.
    pub fill_ratio: Metric,
    /// Cancel ratio against the submitted child quantity.
    pub cancel_ratio: Metric,
    /// Completion time from the parent submission, in seconds.
    pub completion_time_s: Metric,
    /// The number of submitted child orders.
    pub child_count: Metric,
    /// Child churn (cancelled over submitted).
    pub child_churn: Metric,
    /// Mean child lifetime, in seconds.
    pub mean_child_lifetime_s: Metric,
    /// Partial fill ratio of cancelled children.
    pub partial_fill_ratio: Metric,
    /// Price improvement against the order's own limit price.
    pub price_improvement: Metric,
}

/// Aggregated observations the metric arithmetic is computed from.
///
/// This is `pub(crate)` because it is an internal handoff between the observer and the metric
/// arithmetic, not part of the public surface. Every `Option` field is `None` when the underlying
/// observation was absent, which the arithmetic reports as [`MetricValue::NotAvailable`].
#[derive(Clone, Copy, Debug)]
pub(crate) struct MetricInputs {
    /// The side of the parent order.
    pub side: OrderSide,
    /// The parent order's target quantity as `f64`.
    pub parent_target_qty: f64,
    /// The parent order's own limit price as `f64`, when it has one.
    pub limit_price: Option<f64>,
    /// The declared horizon.
    pub horizon: Option<DurationNanos>,
    /// The declared decision price as `f64`.
    pub decision_price: Option<f64>,
    /// The declared arrival price as `f64`.
    pub arrival_price: Option<f64>,
    /// The parent order's submission timestamp.
    pub parent_submitted: Option<UnixNanos>,
    /// The last observed terminal timestamp (final fill or cancellation).
    pub last_terminal: Option<UnixNanos>,
    /// The quantity-weighted average fill price as `f64`.
    pub fill_avg_px: Option<f64>,
    /// The total filled quantity as `f64`.
    pub filled_qty: f64,
    /// The volume-weighted average price over the benchmark interval as `f64`.
    pub interval_vwap: Option<f64>,
    /// The time-weighted average price over the benchmark interval as `f64`.
    pub interval_twap: Option<f64>,
    /// The midpoint at the arrival timestamp as `f64`.
    pub arrival_mid: Option<f64>,
    /// The quoted half-spread at the arrival timestamp as `f64`.
    pub arrival_half_spread: Option<f64>,
    /// The mean adverse mid move over the declared horizon.
    pub adverse_selection: Option<f64>,
    /// The number of submitted children.
    pub submitted_child_count: u32,
    /// The number of cancelled children.
    pub cancelled_child_count: u32,
    /// The sum of the submitted child quantities as `f64`.
    pub submitted_child_qty: f64,
    /// The sum of the cancelled child quantities as `f64`.
    pub cancelled_child_qty: f64,
    /// The mean child lifetime in seconds.
    pub mean_child_lifetime_s: Option<f64>,
    /// The mean partial fill ratio over cancelled children.
    pub partial_fill_ratio: Option<f64>,
    /// The mean price improvement per fill, signed so that positive is favourable.
    pub price_improvement: Option<f64>,
}

/// Returns the signed cost in basis points.
///
/// The sign convention is cost-positive: a buy that pays more than the reference, and a sell that
/// receives less, both yield a positive value. Returns `None` when the reference is zero.
fn cost_bps(side: OrderSide, actual: f64, reference: f64) -> Option<f64> {
    if reference == 0.0 {
        return None;
    }
    let move_px = match side {
        OrderSide::Buy => actual - reference,
        OrderSide::Sell => reference - actual,
    };
    Some(move_px / reference * 10_000.0)
}

/// Builds a metric from an optional value with a fallback unavailable reason.
fn metric(declaration: MetricDeclaration, value: Option<f64>, reason: UnavailableReason) -> Metric {
    match value {
        Some(value) => Metric::new(declaration, MetricValue::Available(value)),
        None => Metric::new(declaration, MetricValue::NotAvailable(reason)),
    }
}

impl ExecutionMetrics {
    /// Computes the full metric set from the aggregated observations.
    #[must_use]
    pub(crate) fn compute(input: &MetricInputs) -> Self {
        let side = input.side;
        let fill_avg = input.fill_avg_px;

        let implementation_shortfall_bps = metric(
            MetricDeclaration::new(
                METRIC_IMPLEMENTATION_SHORTFALL_BPS,
                MetricUnits::BasisPoints,
                MetricDirection::LowerIsBetter,
            )
            .with_reference_price(ReferencePriceSource::Decision)
            .with_denominator(DenominatorSource::ReferencePrice)
            .with_reference_timestamp(ReferenceTimestamp::Decision),
            input
                .decision_price
                .and_then(|decision| fill_avg.and_then(|avg| cost_bps(side, avg, decision))),
            unavailable_reason(input.decision_price.is_none()),
        );

        let arrival_slippage_bps = metric(
            MetricDeclaration::new(
                METRIC_ARRIVAL_SLIPPAGE_BPS,
                MetricUnits::BasisPoints,
                MetricDirection::LowerIsBetter,
            )
            .with_reference_price(ReferencePriceSource::Arrival)
            .with_denominator(DenominatorSource::ReferencePrice)
            .with_reference_timestamp(ReferenceTimestamp::Arrival),
            input
                .arrival_price
                .and_then(|arrival| fill_avg.and_then(|avg| cost_bps(side, avg, arrival))),
            unavailable_reason(input.arrival_price.is_none()),
        );

        let decision_price_slippage_bps = metric(
            MetricDeclaration::new(
                METRIC_DECISION_PRICE_SLIPPAGE_BPS,
                MetricUnits::BasisPoints,
                MetricDirection::LowerIsBetter,
            )
            .with_reference_price(ReferencePriceSource::Decision)
            .with_denominator(DenominatorSource::ReferencePrice)
            .with_reference_timestamp(ReferenceTimestamp::Decision),
            input.decision_price.and_then(|decision| {
                input
                    .arrival_price
                    .and_then(|arrival| cost_bps(side, arrival, decision))
            }),
            unavailable_reason(input.decision_price.is_none() || input.arrival_price.is_none()),
        );

        let vwap_slippage_bps = metric(
            MetricDeclaration::new(
                METRIC_VWAP_SLIPPAGE_BPS,
                MetricUnits::BasisPoints,
                MetricDirection::LowerIsBetter,
            )
            .with_reference_price(ReferencePriceSource::IntervalVwap)
            .with_denominator(DenominatorSource::ReferencePrice)
            .with_reference_timestamp(ReferenceTimestamp::BenchmarkIntervalStart),
            input
                .interval_vwap
                .and_then(|vwap| fill_avg.and_then(|avg| cost_bps(side, avg, vwap))),
            UnavailableReason::NoObservations,
        );

        let twap_slippage_bps = metric(
            MetricDeclaration::new(
                METRIC_TWAP_SLIPPAGE_BPS,
                MetricUnits::BasisPoints,
                MetricDirection::LowerIsBetter,
            )
            .with_reference_price(ReferencePriceSource::IntervalTwap)
            .with_denominator(DenominatorSource::ReferencePrice)
            .with_reference_timestamp(ReferenceTimestamp::BenchmarkIntervalStart),
            input
                .interval_twap
                .and_then(|twap| fill_avg.and_then(|avg| cost_bps(side, avg, twap))),
            UnavailableReason::NoObservations,
        );

        let midpoint_slippage_bps = metric(
            MetricDeclaration::new(
                METRIC_MIDPOINT_SLIPPAGE_BPS,
                MetricUnits::BasisPoints,
                MetricDirection::LowerIsBetter,
            )
            .with_reference_price(ReferencePriceSource::Midpoint)
            .with_denominator(DenominatorSource::ReferencePrice)
            .with_reference_timestamp(ReferenceTimestamp::Arrival),
            input
                .arrival_mid
                .and_then(|mid| fill_avg.and_then(|avg| cost_bps(side, avg, mid))),
            UnavailableReason::NoObservations,
        );

        let spread_capture = {
            let declaration = MetricDeclaration::new(
                METRIC_SPREAD_CAPTURE,
                MetricUnits::Ratio,
                MetricDirection::HigherIsBetter,
            )
            .with_reference_price(ReferencePriceSource::Midpoint)
            .with_denominator(DenominatorSource::QuotedHalfSpread)
            .with_reference_timestamp(ReferenceTimestamp::Arrival);

            let value = match (input.arrival_mid, input.arrival_half_spread, fill_avg) {
                (Some(mid), Some(half_spread), Some(avg)) if half_spread > 0.0 => {
                    let capture = match side {
                        OrderSide::Buy => mid - avg,
                        OrderSide::Sell => avg - mid,
                    };
                    Some(capture / half_spread)
                }
                _ => None,
            };
            metric(declaration, value, spread_capture_reason(input))
        };

        let adverse_selection = {
            let declaration = MetricDeclaration::new(
                METRIC_ADVERSE_SELECTION,
                MetricUnits::Price,
                MetricDirection::LowerIsBetter,
            )
            .with_reference_price(ReferencePriceSource::Midpoint)
            .with_denominator(DenominatorSource::NotApplicable)
            .with_reference_timestamp(ReferenceTimestamp::FillPlusHorizon);
            let declaration = match input.horizon {
                Some(horizon) => declaration.with_horizon(horizon),
                None => declaration,
            };

            let (value, reason) = match input.horizon {
                None => (None, UnavailableReason::NoHorizon),
                Some(_) => (input.adverse_selection, UnavailableReason::NoObservations),
            };
            metric(declaration, value, reason)
        };

        let fill_ratio = metric(
            MetricDeclaration::new(
                METRIC_FILL_RATIO,
                MetricUnits::Ratio,
                MetricDirection::HigherIsBetter,
            )
            .with_denominator(DenominatorSource::ParentTargetQuantity),
            (input.parent_target_qty > 0.0).then(|| input.filled_qty / input.parent_target_qty),
            UnavailableReason::ZeroDenominator,
        );

        let cancel_ratio = metric(
            MetricDeclaration::new(
                METRIC_CANCEL_RATIO,
                MetricUnits::Ratio,
                MetricDirection::LowerIsBetter,
            )
            .with_denominator(DenominatorSource::SubmittedChildQuantity),
            (input.submitted_child_qty > 0.0)
                .then(|| input.cancelled_child_qty / input.submitted_child_qty),
            UnavailableReason::ZeroDenominator,
        );

        let completion_time_s = {
            let declaration = MetricDeclaration::new(
                METRIC_COMPLETION_TIME_S,
                MetricUnits::Seconds,
                MetricDirection::LowerIsBetter,
            )
            .with_denominator(DenominatorSource::NotApplicable)
            .with_reference_timestamp(ReferenceTimestamp::ParentSubmission);
            let declaration = match input.horizon {
                Some(horizon) => declaration.with_horizon(horizon),
                None => declaration,
            };

            let value = input.parent_submitted.and_then(|start| {
                input
                    .last_terminal
                    .map(|end| end.saturating_duration_since(start).as_secs_f64())
            });
            let reason = if input.horizon.is_none() {
                UnavailableReason::NoHorizon
            } else {
                UnavailableReason::NoTimestamp
            };
            metric(declaration, value, reason)
        };

        let child_count = Metric::new(
            MetricDeclaration::new(
                METRIC_CHILD_COUNT,
                MetricUnits::Count,
                MetricDirection::Neutral,
            )
            .with_denominator(DenominatorSource::NotApplicable),
            MetricValue::Available(f64::from(input.submitted_child_count)),
        );

        let child_churn = metric(
            MetricDeclaration::new(
                METRIC_CHILD_CHURN,
                MetricUnits::Ratio,
                MetricDirection::LowerIsBetter,
            )
            .with_denominator(DenominatorSource::SubmittedChildCount),
            (input.submitted_child_count > 0).then(|| {
                f64::from(input.cancelled_child_count) / f64::from(input.submitted_child_count)
            }),
            UnavailableReason::ZeroDenominator,
        );

        let mean_child_lifetime_s = metric(
            MetricDeclaration::new(
                METRIC_MEAN_CHILD_LIFETIME_S,
                MetricUnits::Seconds,
                MetricDirection::Neutral,
            )
            .with_denominator(DenominatorSource::NotApplicable),
            input.mean_child_lifetime_s,
            UnavailableReason::NoObservations,
        );

        let partial_fill_ratio = metric(
            MetricDeclaration::new(
                METRIC_PARTIAL_FILL_RATIO,
                MetricUnits::Ratio,
                MetricDirection::HigherIsBetter,
            )
            .with_denominator(DenominatorSource::SubmittedChildQuantity),
            input.partial_fill_ratio,
            UnavailableReason::NoObservations,
        );

        let price_improvement = metric(
            MetricDeclaration::new(
                METRIC_PRICE_IMPROVEMENT,
                MetricUnits::Price,
                MetricDirection::HigherIsBetter,
            )
            .with_reference_price(ReferencePriceSource::LimitPrice)
            .with_denominator(DenominatorSource::NotApplicable),
            input.price_improvement,
            unavailable_reason(input.limit_price.is_none()),
        );

        Self {
            implementation_shortfall_bps,
            arrival_slippage_bps,
            decision_price_slippage_bps,
            vwap_slippage_bps,
            twap_slippage_bps,
            midpoint_slippage_bps,
            spread_capture,
            adverse_selection,
            fill_ratio,
            cancel_ratio,
            completion_time_s,
            child_count,
            child_churn,
            mean_child_lifetime_s,
            partial_fill_ratio,
            price_improvement,
        }
    }
}

/// Returns the reason a metric is unavailable, preferring a missing reference price.
fn unavailable_reason(missing_reference: bool) -> UnavailableReason {
    if missing_reference {
        UnavailableReason::NoReferencePrice
    } else {
        UnavailableReason::NoObservations
    }
}

/// Returns the reason spread capture is unavailable.
fn spread_capture_reason(input: &MetricInputs) -> UnavailableReason {
    if input.arrival_mid.is_none() {
        UnavailableReason::NoReferencePrice
    } else if input
        .arrival_half_spread
        .is_some_and(|half_spread| half_spread <= 0.0)
    {
        UnavailableReason::ZeroDenominator
    } else {
        UnavailableReason::NoObservations
    }
}
