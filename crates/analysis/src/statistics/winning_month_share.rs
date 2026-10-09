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
    period::{PerformancePeriod, PeriodKind},
    statistic::PortfolioStatistic,
};

/// Calculates the share of months that closed with a positive return.
///
/// The statistic is defined over a month-period frame: `None` is returned unless every period is
/// a whole calendar month, because a share of months cannot be read from a frame of days or
/// weeks. Within such a frame, the share is the fraction of months with a `net_return` that is
/// present and greater than zero, out of the months that have a return at all; `None` when no
/// month has a return. A month whose return could not be resolved is neither a winning nor a
/// losing month and is excluded from the denominator rather than counted as a loss.
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
pub struct WinningMonthShare {}

impl Display for WinningMonthShare {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Winning Month Share")
    }
}

impl PortfolioStatistic for WinningMonthShare {
    type Item = f64;

    fn name(&self) -> String {
        self.to_string()
    }

    fn definition(&self) -> MetricDefinition {
        MetricDefinition::new(
            "winning_month_share",
            "Winning Month Share",
            MetricUnits::Fraction,
            MetricDirection::Maximize,
            [MetricInput::PerformancePeriods],
        )
        .with_tags([MetricTag::Returns])
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
        let is_monthly = periods.iter().all(|period| {
            PeriodKind::Month.is_whole_window(period.accounting.start, period.accounting.end)
        });
        if !is_monthly {
            return None;
        }

        let mut winning = 0_u64;
        let mut with_return = 0_u64;

        for period in periods {
            if let Some(net_return) = period.performance.net_return {
                with_return += 1;
                if net_return > 0.0 {
                    winning += 1;
                }
            }
        }

        if with_return == 0 {
            return None;
        }

        Some(winning as f64 / with_return as f64)
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

    const DAY_NS: u64 = 86_400_000_000_000;
    const JAN_2019: u64 = 1_546_300_800_000_000_000;
    const FEB_2019: u64 = 1_548_979_200_000_000_000;
    const MAR_2019: u64 = 1_551_398_400_000_000_000;
    const APR_2019: u64 = 1_554_076_800_000_000_000;

    fn period(start: u64, end: u64, net_return: Option<f64>) -> PerformancePeriod {
        PerformancePeriod {
            accounting: PeriodAccounting {
                start: UnixNanos::from(start),
                end: UnixNanos::from(end),
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
                open_positions: 0,
                gross_exposure: CurrencyTotals::new(),
                net_exposure: CurrencyTotals::new(),
            },
            performance: PeriodPerformance {
                net_pnl: CurrencyTotals::new(),
                net_return,
                drawdown: CurrencyTotals::new(),
                drawdown_percentage: None,
            },
        }
    }

    #[rstest]
    fn test_empty_frame_is_undefined() {
        let statistic = WinningMonthShare {};
        assert_eq!(statistic.calculate_from_periods(&[]), None);
    }

    #[rstest]
    fn test_hand_computed_share() {
        let statistic = WinningMonthShare {};
        let periods = vec![
            period(JAN_2019, FEB_2019, Some(0.10)),
            period(FEB_2019, MAR_2019, Some(-0.05)),
            period(MAR_2019, APR_2019, Some(0.02)),
        ];

        let result = statistic.calculate_from_periods(&periods);
        assert!(result.is_some());
        assert!(approx_eq!(f64, result.unwrap(), 2.0 / 3.0, epsilon = 1e-9));
    }

    #[rstest]
    fn test_months_without_a_return_are_excluded() {
        let statistic = WinningMonthShare {};
        let periods = vec![
            period(JAN_2019, FEB_2019, Some(0.10)),
            period(FEB_2019, MAR_2019, None),
            period(MAR_2019, APR_2019, Some(-0.05)),
        ];

        let result = statistic.calculate_from_periods(&periods);
        assert!(result.is_some());
        assert!(approx_eq!(f64, result.unwrap(), 0.5, epsilon = 1e-9));
    }

    #[rstest]
    fn test_no_month_with_a_return_is_undefined() {
        let statistic = WinningMonthShare {};
        let periods = vec![
            period(JAN_2019, FEB_2019, None),
            period(FEB_2019, MAR_2019, None),
        ];

        assert_eq!(statistic.calculate_from_periods(&periods), None);
    }

    #[rstest]
    fn test_a_non_month_frame_is_refused() {
        let statistic = WinningMonthShare {};
        let periods = vec![
            period(JAN_2019, JAN_2019 + DAY_NS, Some(0.01)),
            period(JAN_2019 + DAY_NS, JAN_2019 + 2 * DAY_NS, Some(-0.01)),
        ];

        assert_eq!(statistic.calculate_from_periods(&periods), None);
    }

    #[rstest]
    fn test_other_inputs_are_unsupported() {
        let statistic = WinningMonthShare {};
        assert_eq!(statistic.calculate_from_returns(&Returns::default()), None);
        assert_eq!(statistic.calculate_from_realized_pnls(&[1.0]), None);
        assert_eq!(statistic.calculate_from_positions(&[]), None);
    }

    #[rstest]
    fn test_name_and_definition() {
        let statistic = WinningMonthShare {};
        assert_eq!(statistic.name(), "Winning Month Share");

        let definition = statistic.definition();
        assert_eq!(definition.id(), "winning_month_share");
        assert_eq!(definition.title(), statistic.name());
    }
}
