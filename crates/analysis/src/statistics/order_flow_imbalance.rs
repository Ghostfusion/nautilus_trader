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

//! The signed order-flow imbalance accumulated from the trade tape.
//!
//! The imbalance is the sum over the run's trades of the traded size, signed positive for a
//! reported buy aggressor and negative for a reported sell aggressor. Its sign is therefore a
//! property of the venue's reported aggressor side, not of an independently reconstructed one,
//! which is why the definition carries [`MetricTag::DirectionDependent`]: when the reported side
//! disagrees with a reconstruction of it often enough, the magnitude may still be measured while
//! the sign is not, and a signed number built on a wrong sign is worse than no number.
//!
//! The quantity is accumulated by the data engine from its own tape, exactly in the instrument's
//! size units, and handed to this statistic already signed. The statistic is a view of that
//! quantity rather than a second calculation of it: it converts the exact sum to `f64` only at
//! the reporting boundary, where the platform's statistic interface is `f64`.
//!
//! A report refuses this metric below a declared aggressor-agreement floor, and the refusal is
//! the analyzer's, not this statistic's: the statistic has no way to know the observed rate, so
//! it computes the value it was handed and the report path decides whether to surface it.

use std::fmt::Display;

use nautilus_model::position::Position;
use rust_decimal::{Decimal, prelude::ToPrimitive};

use crate::{
    Returns,
    metric::{MetricDefinition, MetricDirection, MetricInput, MetricStage, MetricTag, MetricUnits},
    period::PerformancePeriod,
    statistic::PortfolioStatistic,
};

/// Reports the signed order-flow imbalance accumulated from the run's trade tape.
///
/// A positive value is net buy-aggressed size, a negative value net sell-aggressed size, and a
/// value of zero is a run whose signed flow cancelled out. The value is in the instrument's size
/// units, summed across every instrument the tape carried.
#[repr(C)]
#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.analysis", from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.analysis")
)]
pub struct OrderFlowImbalance {}

impl OrderFlowImbalance {
    /// Creates a new [`OrderFlowImbalance`] instance.
    #[must_use]
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for OrderFlowImbalance {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for OrderFlowImbalance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Order Flow Imbalance (reported aggressor)")
    }
}

impl PortfolioStatistic for OrderFlowImbalance {
    type Item = f64;

    fn name(&self) -> String {
        self.to_string()
    }

    fn definition(&self) -> MetricDefinition {
        MetricDefinition::new(
            "order_flow_imbalance",
            "Order Flow Imbalance (reported aggressor)",
            MetricUnits::Ratio,
            MetricDirection::Informational,
            [MetricInput::Trades],
        )
        .with_tags([MetricTag::DirectionDependent, MetricTag::Exposure])
        .with_stage(MetricStage::Decision)
    }

    fn calculate_from_returns(&self, _returns: &Returns) -> Option<Self::Item> {
        None
    }

    fn calculate_from_realized_pnls(&self, _realized_pnls: &[f64]) -> Option<Self::Item> {
        None
    }

    fn calculate_from_positions(&self, _positions: &[Position]) -> Option<Self::Item> {
        None
    }

    fn calculate_from_periods(&self, _periods: &[PerformancePeriod]) -> Option<Self::Item> {
        None
    }

    fn calculate_from_tape(&self, signed_order_flow_imbalance: Decimal) -> Option<Self::Item> {
        signed_order_flow_imbalance.to_f64()
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    fn test_display_agrees_with_the_declared_title() {
        let statistic = OrderFlowImbalance::new();

        assert_eq!(statistic.name(), statistic.definition().title());
        assert_eq!(
            statistic.to_string(),
            "Order Flow Imbalance (reported aggressor)"
        );
    }

    #[rstest]
    fn test_definition_declares_direction_dependence_and_the_tape_input() {
        let definition = OrderFlowImbalance::new().definition();

        assert_eq!(definition.id(), "order_flow_imbalance");
        assert!(definition.tags().contains(&MetricTag::DirectionDependent));
        assert!(definition.is_defined_over(MetricInput::Trades));
        assert!(!definition.is_defined_over(MetricInput::Returns));
    }

    #[rstest]
    fn test_the_signed_imbalance_is_reported_as_a_signed_value() {
        let statistic = OrderFlowImbalance::new();

        assert_eq!(
            statistic.calculate_from_tape(Decimal::new(1234, 2)),
            Some(12.34)
        );
        assert_eq!(
            statistic.calculate_from_tape(Decimal::new(-5, 0)),
            Some(-5.0)
        );
        assert_eq!(statistic.calculate_from_tape(Decimal::ZERO), Some(0.0));
    }

    #[rstest]
    fn test_a_tape_with_no_signed_flow_declines_every_other_input() {
        let statistic = OrderFlowImbalance::new();

        assert_eq!(statistic.calculate_from_returns(&Returns::default()), None);
        assert_eq!(statistic.calculate_from_realized_pnls(&[1.0]), None);
        assert_eq!(statistic.calculate_from_positions(&[]), None);
        assert_eq!(statistic.calculate_from_periods(&[]), None);
    }
}
