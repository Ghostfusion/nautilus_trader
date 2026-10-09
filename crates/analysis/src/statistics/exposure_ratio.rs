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

use std::fmt::Display;

use nautilus_model::position::Position;

use crate::{
    Returns,
    metric::{MetricDefinition, MetricDirection, MetricInput, MetricStage, MetricTag, MetricUnits},
    period::PerformancePeriod,
    statistic::PortfolioStatistic,
};

/// Calculates the share of performance periods that ended with at least one open position.
///
/// The value is a sampled proxy for time in market, not a true time-in-market figure: a period
/// counts as exposed when at least one position was open at its end, as reported by the period's
/// exposure, and an intraday position that opened and closed inside a period is invisible to the
/// count. A true time-in-market figure would need an exposure accumulator in the period reducer,
/// summing the held duration within each period; this statistic reports what the frame's
/// end-of-period exposure supports. `None` is returned when there are no periods.
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
pub struct ExposureRatio {}

impl Display for ExposureRatio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Exposure Ratio (share of periods held)")
    }
}

impl PortfolioStatistic for ExposureRatio {
    type Item = f64;

    fn name(&self) -> String {
        self.to_string()
    }

    fn definition(&self) -> MetricDefinition {
        MetricDefinition::new(
            "exposure_ratio",
            "Exposure Ratio (share of periods held)",
            MetricUnits::Fraction,
            MetricDirection::Maximize,
            [MetricInput::PerformancePeriods],
        )
        .with_tags([MetricTag::Exposure])
        .with_stage(MetricStage::Account)
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

    fn calculate_from_periods(&self, periods: &[PerformancePeriod]) -> Option<Self::Item> {
        if periods.is_empty() {
            return None;
        }

        let exposed = periods
            .iter()
            .filter(|period| period.exposure.open_positions > 0)
            .count();

        Some(exposed as f64 / periods.len() as f64)
    }
}

#[cfg(test)]
mod tests {
    use nautilus_core::{UnixNanos, approx_eq};
    use rstest::rstest;

    use super::*;
    use crate::period::{
        CurrencyTotals, PeriodAccounting, PeriodActivity, PeriodExposure, PeriodPerformance,
    };

    fn period(open_positions: usize) -> PerformancePeriod {
        PerformancePeriod {
            accounting: PeriodAccounting {
                start: UnixNanos::default(),
                end: UnixNanos::default(),
                starting_equity: CurrencyTotals::new(),
                ending_equity: CurrencyTotals::new(),
                realized_pnl: CurrencyTotals::new(),
                unrealized_pnl: CurrencyTotals::new(),
                commission: CurrencyTotals::new(),
            },
            activity: PeriodActivity {
                volume: rust_decimal::Decimal::ZERO,
                turnover: CurrencyTotals::new(),
                trade_count: 0,
                winning_trades: 0,
                losing_trades: 0,
            },
            exposure: PeriodExposure {
                open_positions,
                gross_exposure: CurrencyTotals::new(),
                net_exposure: CurrencyTotals::new(),
            },
            performance: PeriodPerformance {
                net_pnl: CurrencyTotals::new(),
                net_return: None,
                drawdown: CurrencyTotals::new(),
                drawdown_percentage: None,
            },
        }
    }

    #[rstest]
    fn test_empty_frame_is_undefined() {
        let statistic = ExposureRatio {};
        assert_eq!(statistic.calculate_from_periods(&[]), None);
    }

    #[rstest]
    fn test_hand_computed_share() {
        let statistic = ExposureRatio {};
        let periods = vec![period(1), period(0), period(2)];

        let result = statistic.calculate_from_periods(&periods);
        assert!(result.is_some());
        assert!(approx_eq!(f64, result.unwrap(), 2.0 / 3.0, epsilon = 1e-9));
    }

    #[rstest]
    fn test_never_held_is_zero() {
        let statistic = ExposureRatio {};
        let periods = vec![period(0), period(0)];

        let result = statistic.calculate_from_periods(&periods);
        assert!(result.is_some());
        assert!(approx_eq!(f64, result.unwrap(), 0.0, epsilon = 1e-9));
    }

    #[rstest]
    fn test_always_held_is_one() {
        let statistic = ExposureRatio {};
        let periods = vec![period(3), period(1)];

        let result = statistic.calculate_from_periods(&periods);
        assert!(result.is_some());
        assert!(approx_eq!(f64, result.unwrap(), 1.0, epsilon = 1e-9));
    }

    #[rstest]
    fn test_other_inputs_are_unsupported() {
        let statistic = ExposureRatio {};
        assert_eq!(statistic.calculate_from_returns(&Returns::default()), None);
        assert_eq!(statistic.calculate_from_realized_pnls(&[1.0]), None);
        assert_eq!(statistic.calculate_from_positions(&[]), None);
    }

    #[rstest]
    fn test_name_and_definition() {
        let statistic = ExposureRatio {};
        assert_eq!(statistic.name(), "Exposure Ratio (share of periods held)");

        let definition = statistic.definition();
        assert_eq!(definition.id(), "exposure_ratio");
        assert_eq!(definition.title(), statistic.name());
    }
}
