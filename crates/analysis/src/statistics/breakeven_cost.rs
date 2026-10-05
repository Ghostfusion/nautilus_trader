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

use nautilus_model::position::Position;

use crate::{
    Returns,
    metric::{
        BASIS_POINTS_PER_UNIT, MetricDefinition, MetricDirection, MetricInput, MetricTag,
        MetricUnits,
    },
    period::{PerformancePeriod, PeriodFrameTotals, single_currency_share},
    statistic::PortfolioStatistic,
};

/// Calculates the breakeven cost over a performance-period frame in basis points of turnover.
///
/// The figure is the frame's gross PnL over its notional turnover: the cost rate the strategy could
/// have paid and still broken even, which is the edge the frame actually earned per unit traded,
/// measured on the same basis as the `CostBasisPoints` it is read against. A strategy whose
/// breakeven cost sits below the cost it pays is losing money to its costs, whatever its gross
/// return says, and a result above the cost it pays is not evidence of an edge until the estimate
/// carries its own uncertainty.
///
/// `Maximize` is the direction because the value states how much cost the frame's edge can absorb:
/// a higher breakeven is a more robust result, and a negative one means the frame traded at a loss
/// before costs.
///
/// The rate is defined only when the turnover and the PnL resolve to the same single currency and
/// the turnover is not zero. An empty frame and a frame that traded nothing have no defined rate.
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
pub struct BreakevenCost {}

impl BreakevenCost {
    /// Creates a new [`BreakevenCost`] instance.
    #[must_use]
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for BreakevenCost {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for BreakevenCost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Breakeven Cost (basis points of turnover)")
    }
}

impl PortfolioStatistic for BreakevenCost {
    type Item = f64;

    fn name(&self) -> String {
        self.to_string()
    }

    fn definition(&self) -> MetricDefinition {
        MetricDefinition::new(
            "breakeven_cost",
            "Breakeven Cost (basis points of turnover)",
            MetricUnits::BasisPoints,
            MetricDirection::Maximize,
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
        let totals = PeriodFrameTotals::from_periods(periods)?;
        let rate = single_currency_share(&totals.turnover, &totals.gross_pnl)?;
        Some(rate * BASIS_POINTS_PER_UNIT)
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
    use nautilus_model::types::{Currency, Money};
    use rust_decimal::Decimal;

    /// Returns a period with the given starting equity, net PnL, commission and turnover.
    fn period(
        starting_equity: &[(Currency, f64)],
        net_pnl: &[(Currency, f64)],
        commission: &[(Currency, f64)],
        turnover: &[(Currency, f64)],
    ) -> PerformancePeriod {
        fn totals(amounts: &[(Currency, f64)]) -> CurrencyTotals {
            let mut totals = CurrencyTotals::new();
            for &(currency, amount) in amounts {
                totals.add_money(Money::new(amount, currency));
            }
            totals
        }

        PerformancePeriod {
            accounting: PeriodAccounting {
                start: UnixNanos::default(),
                end: UnixNanos::default(),
                starting_equity: totals(starting_equity),
                ending_equity: CurrencyTotals::new(),
                realized_pnl: CurrencyTotals::new(),
                unrealized_pnl: CurrencyTotals::new(),
                commission: totals(commission),
            },
            activity: PeriodActivity {
                volume: Decimal::ZERO,
                turnover: totals(turnover),
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
                net_pnl: totals(net_pnl),
                net_return: None,
                drawdown: CurrencyTotals::new(),
                drawdown_percentage: None,
            },
        }
    }

    #[rstest]
    fn test_empty_frame_is_undefined() {
        let statistic = BreakevenCost::new();
        assert_eq!(statistic.calculate_from_periods(&[]), None);
    }

    #[rstest]
    fn test_the_edge_per_unit_traded_is_the_breakeven_cost() {
        let statistic = BreakevenCost::new();

        // Net PnL 80 USD over 10,000 USD of turnover, having paid 20 USD of commission: the gross
        // edge is 100 USD, i.e. 100 basis points of turnover.
        let periods = vec![period(
            &[(Currency::USD(), 1000.0)],
            &[(Currency::USD(), 80.0)],
            &[(Currency::USD(), 20.0)],
            &[(Currency::USD(), 10_000.0)],
        )];

        assert!(approx_eq!(
            f64,
            statistic.calculate_from_periods(&periods).unwrap(),
            100.0,
            epsilon = 1e-12
        ));
    }

    #[rstest]
    fn test_a_losing_frame_has_a_negative_breakeven() {
        let statistic = BreakevenCost::new();
        let periods = vec![period(
            &[(Currency::USD(), 1000.0)],
            &[(Currency::USD(), -120.0)],
            &[(Currency::USD(), 20.0)],
            &[(Currency::USD(), 10_000.0)],
        )];

        // A gross loss of 100 USD over 10,000 USD of turnover is -100 basis points.
        assert!(approx_eq!(
            f64,
            statistic.calculate_from_periods(&periods).unwrap(),
            -100.0,
            epsilon = 1e-12
        ));
    }

    #[rstest]
    fn test_no_turnover_is_undefined() {
        let statistic = BreakevenCost::new();
        let periods = vec![period(
            &[(Currency::USD(), 1000.0)],
            &[(Currency::USD(), 80.0)],
            &[(Currency::USD(), 20.0)],
            &[],
        )];

        assert_eq!(statistic.calculate_from_periods(&periods), None);
    }

    #[rstest]
    fn test_a_pnl_in_another_currency_than_the_turnover_is_undefined() {
        let statistic = BreakevenCost::new();
        let periods = vec![period(
            &[(Currency::USD(), 1000.0)],
            &[(Currency::EUR(), 80.0)],
            &[],
            &[(Currency::USD(), 10_000.0)],
        )];

        assert_eq!(statistic.calculate_from_periods(&periods), None);
    }

    #[rstest]
    fn test_scaling_every_amount_leaves_the_rate_unchanged() {
        let statistic = BreakevenCost::new();
        let base = vec![period(
            &[],
            &[(Currency::USD(), 80.0)],
            &[(Currency::USD(), 20.0)],
            &[(Currency::USD(), 10_000.0)],
        )];
        let scaled = vec![period(
            &[],
            &[(Currency::USD(), 400.0)],
            &[(Currency::USD(), 100.0)],
            &[(Currency::USD(), 50_000.0)],
        )];

        let base_value = statistic.calculate_from_periods(&base).unwrap();
        let scaled_value = statistic.calculate_from_periods(&scaled).unwrap();

        assert!(approx_eq!(f64, base_value, 100.0, epsilon = 1e-12));
        assert!(approx_eq!(f64, scaled_value, base_value, epsilon = 1e-12));
    }
}
