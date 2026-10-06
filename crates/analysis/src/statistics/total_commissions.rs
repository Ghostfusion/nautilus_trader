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
    metric::{MetricDefinition, MetricDirection, MetricInput, MetricStage, MetricTag, MetricUnits},
    period::PerformancePeriod,
    statistic::PortfolioStatistic,
};

/// Calculates the total commission over a performance-period frame.
///
/// Commission is the sum of every period's commission, which the frame records per currency. The
/// total is defined only when those amounts resolve to a single currency; a frame whose commission
/// spans more than one currency, or that is empty, has no defined total and returns `None`. A
/// frame with no fills has a genuine total of zero.
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
pub struct TotalCommissions {}

impl TotalCommissions {
    /// Creates a new [`TotalCommissions`] instance.
    #[must_use]
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for TotalCommissions {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TotalCommissions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Total Commissions")
    }
}

impl PortfolioStatistic for TotalCommissions {
    type Item = f64;

    fn name(&self) -> String {
        self.to_string()
    }

    fn definition(&self) -> MetricDefinition {
        MetricDefinition::new(
            "total_commissions",
            "Total Commissions",
            MetricUnits::Currency,
            MetricDirection::Minimize,
            [MetricInput::PerformancePeriods],
        )
        .with_tags([MetricTag::Trade])
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

        let mut currency: Option<Currency> = None;
        let mut total = Decimal::ZERO;

        for period in periods {
            for money in period.accounting.commission.values() {
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

    fn period(commission: &[(Currency, f64)]) -> PerformancePeriod {
        let mut totals = CurrencyTotals::new();
        for &(currency, amount) in commission {
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
                commission: totals,
            },
            activity: PeriodActivity {
                volume: Decimal::ZERO,
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
                net_return: None,
                drawdown: CurrencyTotals::new(),
                drawdown_percentage: None,
            },
        }
    }

    #[rstest]
    fn test_empty_frame_is_undefined() {
        let statistic = TotalCommissions::new();
        assert_eq!(statistic.calculate_from_periods(&[]), None);
    }

    #[rstest]
    fn test_no_fills_is_zero() {
        let statistic = TotalCommissions::new();
        let periods = vec![period(&[]), period(&[])];
        assert_eq!(statistic.calculate_from_periods(&periods), Some(0.0));
    }

    #[rstest]
    fn test_single_currency_totals_are_summed() {
        let statistic = TotalCommissions::new();
        let periods = vec![
            period(&[(Currency::USD(), 5.0)]),
            period(&[]),
            period(&[(Currency::USD(), 1.5)]),
        ];
        assert_eq!(statistic.calculate_from_periods(&periods), Some(6.5));
    }

    #[rstest]
    fn test_mixed_currencies_are_undefined() {
        let statistic = TotalCommissions::new();
        let periods = vec![
            period(&[(Currency::USD(), 5.0)]),
            period(&[(Currency::EUR(), 1.5)]),
        ];
        assert_eq!(statistic.calculate_from_periods(&periods), None);
    }
}
