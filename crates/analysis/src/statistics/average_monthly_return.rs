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

/// The subset of monthly returns an average is taken over.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(
        eq,
        eq_int,
        frozen,
        hash,
        module = "nautilus_trader.analysis",
        from_py_object,
        rename_all = "SCREAMING_SNAKE_CASE",
    )
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass_enum(module = "nautilus_trader.analysis")
)]
pub enum MonthOutcome {
    /// Every month with a return.
    All,
    /// Only months that closed with a positive return.
    Winning,
    /// Only months that closed with a negative return.
    Losing,
}

impl MonthOutcome {
    /// All variants of the closed vocabulary.
    pub const ALL: &'static [Self] = &[Self::All, Self::Winning, Self::Losing];

    /// Returns the stable string for this outcome.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Winning => "winning",
            Self::Losing => "losing",
        }
    }
}

impl Display for MonthOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Calculates the arithmetic mean of a frame's monthly returns.
///
/// The statistic is defined over a month-period frame: `None` is returned unless every period is
/// a whole calendar month, because a monthly average cannot be read from a frame of days or
/// weeks. Within such a frame the mean is taken over the selected monthly `net_return` values,
/// where winning months are those greater than zero and losing months those less than zero. A
/// month whose return could not be resolved is excluded from every selection, and `None` is
/// returned when the selection is empty.
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
pub struct AverageMonthlyReturn {
    outcome: MonthOutcome,
}

impl AverageMonthlyReturn {
    /// Creates a new [`AverageMonthlyReturn`] instance.
    ///
    /// `None` selects every month with a return.
    #[must_use]
    pub fn new(outcome: Option<MonthOutcome>) -> Self {
        Self {
            outcome: outcome.unwrap_or(MonthOutcome::All),
        }
    }
}

impl Display for AverageMonthlyReturn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Average Monthly Return ({})", self.outcome)
    }
}

impl PortfolioStatistic for AverageMonthlyReturn {
    type Item = f64;

    fn name(&self) -> String {
        self.to_string()
    }

    fn definition(&self) -> MetricDefinition {
        MetricDefinition::new(
            "average_monthly_return",
            "Average Monthly Return ({outcome})",
            MetricUnits::Fraction,
            MetricDirection::Maximize,
            [MetricInput::PerformancePeriods],
        )
        .with_parameter("outcome", self.outcome.as_str())
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

        let mut total = 0.0;
        let mut count = 0_u64;

        for period in periods {
            let Some(net_return) = period.performance.net_return else {
                continue;
            };

            let selected = match self.outcome {
                MonthOutcome::All => true,
                MonthOutcome::Winning => net_return > 0.0,
                MonthOutcome::Losing => net_return < 0.0,
            };

            if !selected {
                continue;
            }

            total += net_return;
            count += 1;
        }

        if count == 0 {
            return None;
        }

        Some(total / count as f64)
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

    fn monthly_frame() -> Vec<PerformancePeriod> {
        vec![
            period(JAN_2019, FEB_2019, Some(0.10)),
            period(FEB_2019, MAR_2019, Some(-0.05)),
            period(MAR_2019, APR_2019, Some(0.02)),
        ]
    }

    #[rstest]
    fn test_empty_frame_is_undefined() {
        let statistic = AverageMonthlyReturn::new(None);
        assert_eq!(statistic.calculate_from_periods(&[]), None);
    }

    #[rstest]
    fn test_all_outcome_averages_every_month() {
        let statistic = AverageMonthlyReturn::new(None);
        let result = statistic.calculate_from_periods(&monthly_frame());

        assert!(result.is_some());
        assert!(approx_eq!(f64, result.unwrap(), 0.07 / 3.0, epsilon = 1e-9));
    }

    #[rstest]
    fn test_winning_outcome_hand_computed() {
        let statistic = AverageMonthlyReturn::new(Some(MonthOutcome::Winning));
        let result = statistic.calculate_from_periods(&monthly_frame());

        assert!(result.is_some());
        assert!(approx_eq!(f64, result.unwrap(), 0.06, epsilon = 1e-9));
    }

    #[rstest]
    fn test_losing_outcome_hand_computed() {
        let statistic = AverageMonthlyReturn::new(Some(MonthOutcome::Losing));
        let result = statistic.calculate_from_periods(&monthly_frame());

        assert!(result.is_some());
        assert!(approx_eq!(f64, result.unwrap(), -0.05, epsilon = 1e-9));
    }

    #[rstest]
    fn test_months_without_a_return_are_excluded() {
        let statistic = AverageMonthlyReturn::new(None);
        let periods = vec![
            period(JAN_2019, FEB_2019, Some(0.10)),
            period(FEB_2019, MAR_2019, None),
            period(MAR_2019, APR_2019, Some(0.02)),
        ];

        let result = statistic.calculate_from_periods(&periods);
        assert!(result.is_some());
        assert!(approx_eq!(f64, result.unwrap(), 0.06, epsilon = 1e-9));
    }

    #[rstest]
    fn test_empty_selection_is_undefined() {
        let statistic = AverageMonthlyReturn::new(Some(MonthOutcome::Winning));
        let periods = vec![period(JAN_2019, FEB_2019, Some(-0.05))];

        assert_eq!(statistic.calculate_from_periods(&periods), None);
    }

    #[rstest]
    fn test_a_non_month_frame_is_refused() {
        let statistic = AverageMonthlyReturn::new(None);
        let periods = vec![period(JAN_2019, JAN_2019 + DAY_NS, Some(0.01))];

        assert_eq!(statistic.calculate_from_periods(&periods), None);
    }

    #[rstest]
    fn test_other_inputs_are_unsupported() {
        let statistic = AverageMonthlyReturn::new(None);
        assert_eq!(statistic.calculate_from_returns(&Returns::default()), None);
        assert_eq!(statistic.calculate_from_realized_pnls(&[1.0]), None);
        assert_eq!(statistic.calculate_from_positions(&[]), None);
    }

    #[rstest]
    fn test_name_and_definition() {
        let statistic = AverageMonthlyReturn::new(Some(MonthOutcome::Winning));
        assert_eq!(statistic.name(), "Average Monthly Return (winning)");

        let definition = statistic.definition();
        assert_eq!(definition.id(), "average_monthly_return");
        assert_eq!(definition.title(), statistic.name());
    }
}
