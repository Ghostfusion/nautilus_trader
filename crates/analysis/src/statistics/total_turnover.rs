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

use nautilus_model::{position::Position, types::Currency};
use rust_decimal::{Decimal, prelude::ToPrimitive};

use crate::{
    Returns,
    metric::{MetricDefinition, MetricDirection, MetricInput, MetricTag, MetricUnits},
    period::PerformancePeriod,
    statistic::PortfolioStatistic,
};

/// Calculates the total turnover over a performance-period frame.
///
/// Turnover is the sum of every period's notional turnover, which the frame records per currency.
/// The total is defined only when those amounts resolve to a single currency; a frame whose
/// turnover spans more than one currency, or that is empty, has no defined total and returns
/// `None`. A frame with no trades has a genuine total of zero.
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
pub struct TotalTurnover {}

impl TotalTurnover {
    /// Creates a new [`TotalTurnover`] instance.
    #[must_use]
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for TotalTurnover {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TotalTurnover {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Total Turnover")
    }
}

impl PortfolioStatistic for TotalTurnover {
    type Item = f64;

    fn name(&self) -> String {
        self.to_string()
    }

    fn definition(&self) -> MetricDefinition {
        MetricDefinition::new(
            "total_turnover",
            "Total Turnover",
            MetricUnits::Currency,
            MetricDirection::Minimize,
            [MetricInput::PerformancePeriods],
        )
        .with_tags([MetricTag::Trade])
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

        let mut currency: Option<Currency> = None;
        let mut total = Decimal::ZERO;

        for period in periods {
            for money in period.activity.turnover.values() {
                match currency {
                    None => currency = Some(money.currency),
                    Some(current) if current != money.currency => return None,
                    _ => {}
                }
                total += money.as_decimal();
            }
        }

        total.to_f64()
    }
}

#[cfg(test)]
mod tests {
    use nautilus_core::UnixNanos;
    use rstest::rstest;

    use super::*;
    use crate::period::{
        CurrencyTotals, PeriodAccounting, PeriodActivity, PeriodExposure, PeriodPerformance,
    };
    use nautilus_model::types::Money;

    fn period(turnover: &[(Currency, f64)]) -> PerformancePeriod {
        let mut totals = CurrencyTotals::new();
        for &(currency, amount) in turnover {
            totals.add_money(Money::new(amount, currency));
        }

        PerformancePeriod {
            accounting: PeriodAccounting {
                start: UnixNanos::default(),
                end: UnixNanos::default(),
                starting_equity: CurrencyTotals::new(),
                ending_equity: CurrencyTotals::new(),
                realized_pnl: CurrencyTotals::new(),
                unrealized_pnl: CurrencyTotals::new(),
                commission: CurrencyTotals::new(),
                fees: CurrencyTotals::new(),
                slippage: CurrencyTotals::new(),
            },
            activity: PeriodActivity {
                volume: Decimal::ZERO,
                turnover: totals,
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
                net_return: None,
                drawdown: CurrencyTotals::new(),
                drawdown_percentage: None,
            },
        }
    }

    #[rstest]
    fn test_empty_frame_is_undefined() {
        let statistic = TotalTurnover::new();
        assert_eq!(statistic.calculate_from_periods(&[]), None);
    }

    #[rstest]
    fn test_no_trades_is_zero() {
        let statistic = TotalTurnover::new();
        let periods = vec![period(&[]), period(&[])];
        assert_eq!(statistic.calculate_from_periods(&periods), Some(0.0));
    }

    #[rstest]
    fn test_single_currency_totals_are_summed() {
        let statistic = TotalTurnover::new();
        let periods = vec![
            period(&[(Currency::USD(), 1_000.0)]),
            period(&[]),
            period(&[(Currency::USD(), 250.0)]),
        ];
        assert_eq!(statistic.calculate_from_periods(&periods), Some(1_250.0));
    }

    #[rstest]
    fn test_mixed_currencies_are_undefined() {
        let statistic = TotalTurnover::new();
        let periods = vec![
            period(&[(Currency::USD(), 1_000.0)]),
            period(&[(Currency::EUR(), 250.0)]),
        ];
        assert_eq!(statistic.calculate_from_periods(&periods), None);
    }
}
