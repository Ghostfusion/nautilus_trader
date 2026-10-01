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

//! Reference metadata for execution metrics.
//!
//! A metric value has no meaning in isolation: the same number is an implementation shortfall
//! or a benchmark slippage depending on which price it was measured against, over which interval
//! and from which timestamp. The types here make that convention explicit, so a value always
//! travels with the declaration that defines it.

use std::fmt::Display;

use nautilus_core::DurationNanos;

/// Where the reference price a metric is measured against comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReferencePriceSource {
    /// The price at the moment the caller decided to trade.
    Decision,
    /// The price at which the order arrived at the venue.
    Arrival,
    /// The open of the declared benchmark interval.
    BenchmarkIntervalOpen,
    /// The volume-weighted average price over the declared benchmark interval.
    IntervalVwap,
    /// The time-weighted average price over the declared benchmark interval.
    IntervalTwap,
    /// The midpoint of the book at a stated timestamp.
    Midpoint,
    /// The order's own limit price.
    LimitPrice,
}

impl Display for ReferencePriceSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Decision => "decision",
            Self::Arrival => "arrival",
            Self::BenchmarkIntervalOpen => "benchmark_interval_open",
            Self::IntervalVwap => "interval_vwap",
            Self::IntervalTwap => "interval_twap",
            Self::Midpoint => "midpoint",
            Self::LimitPrice => "limit_price",
        })
    }
}

/// Which timestamp a reference price or horizon is measured against.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ReferenceTimestamp {
    /// The decision timestamp the caller declared.
    Decision,
    /// The arrival timestamp the caller declared.
    Arrival,
    /// The parent order's submission timestamp.
    ParentSubmission,
    /// The first child order's submission timestamp.
    FirstChild,
    /// The start of the declared benchmark interval.
    BenchmarkIntervalStart,
    /// The fill timestamp plus the metric's declared horizon.
    FillPlusHorizon,
}

impl Display for ReferenceTimestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Decision => "decision",
            Self::Arrival => "arrival",
            Self::ParentSubmission => "parent_submission",
            Self::FirstChild => "first_child",
            Self::BenchmarkIntervalStart => "benchmark_interval_start",
            Self::FillPlusHorizon => "fill_plus_horizon",
        })
    }
}

/// What a metric value is normalised against.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DenominatorSource {
    /// The metric's own declared reference price.
    ReferencePrice,
    /// The parent order's target quantity.
    ParentTargetQuantity,
    /// The sum of the submitted child quantities.
    SubmittedChildQuantity,
    /// The number of submitted children.
    SubmittedChildCount,
    /// The quoted half-spread at the stated timestamp.
    QuotedHalfSpread,
    /// The declared horizon duration.
    Horizon,
    /// The metric is not a ratio and has no denominator.
    NotApplicable,
}

impl Display for DenominatorSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::ReferencePrice => "reference_price",
            Self::ParentTargetQuantity => "parent_target_quantity",
            Self::SubmittedChildQuantity => "submitted_child_quantity",
            Self::SubmittedChildCount => "submitted_child_count",
            Self::QuotedHalfSpread => "quoted_half_spread",
            Self::Horizon => "horizon",
            Self::NotApplicable => "not_applicable",
        })
    }
}

/// The direction a metric is better in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MetricDirection {
    /// A smaller value is better (a cost).
    LowerIsBetter,
    /// A larger value is better (a benefit).
    HigherIsBetter,
    /// Neither direction is preferred; the value is descriptive.
    Neutral,
}

impl Display for MetricDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::LowerIsBetter => "lower_is_better",
            Self::HigherIsBetter => "higher_is_better",
            Self::Neutral => "neutral",
        })
    }
}

/// The units a metric value is expressed in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MetricUnits {
    /// Basis points (one hundredth of a percent).
    BasisPoints,
    /// A dimensionless ratio or fraction.
    Ratio,
    /// A price difference in the instrument's quote units.
    Price,
    /// A duration in seconds.
    Seconds,
    /// A whole count.
    Count,
}

impl Display for MetricUnits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::BasisPoints => "basis_points",
            Self::Ratio => "ratio",
            Self::Price => "price",
            Self::Seconds => "seconds",
            Self::Count => "count",
        })
    }
}

/// The declaration that makes a metric value defined.
///
/// Every value produced by the analytics layer carries one of these, so a caller cannot read the
/// number without also reading the price, denominator and timestamp it was measured against.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MetricDeclaration {
    /// The stable metric identifier.
    pub metric_id: &'static str,
    /// The units the value is expressed in.
    pub units: MetricUnits,
    /// The direction the value is better in.
    pub direction: MetricDirection,
    /// The reference price the value is measured against, when it has one.
    pub reference_price: Option<ReferencePriceSource>,
    /// What the value is normalised against.
    pub denominator: DenominatorSource,
    /// The timestamp the value is measured from, when it has one.
    pub reference_timestamp: Option<ReferenceTimestamp>,
    /// The declared horizon, when the metric depends on one.
    pub horizon: Option<DurationNanos>,
}

impl MetricDeclaration {
    /// Creates a new declaration with no reference metadata beyond its identity.
    #[must_use]
    pub fn new(metric_id: &'static str, units: MetricUnits, direction: MetricDirection) -> Self {
        Self {
            metric_id,
            units,
            direction,
            reference_price: None,
            denominator: DenominatorSource::NotApplicable,
            reference_timestamp: None,
            horizon: None,
        }
    }

    /// Sets the reference price source.
    #[must_use]
    pub fn with_reference_price(mut self, source: ReferencePriceSource) -> Self {
        self.reference_price = Some(source);
        self
    }

    /// Sets the denominator source.
    #[must_use]
    pub fn with_denominator(mut self, denominator: DenominatorSource) -> Self {
        self.denominator = denominator;
        self
    }

    /// Sets the reference timestamp.
    #[must_use]
    pub fn with_reference_timestamp(mut self, timestamp: ReferenceTimestamp) -> Self {
        self.reference_timestamp = Some(timestamp);
        self
    }

    /// Sets the declared horizon.
    #[must_use]
    pub fn with_horizon(mut self, horizon: DurationNanos) -> Self {
        self.horizon = Some(horizon);
        self
    }
}
