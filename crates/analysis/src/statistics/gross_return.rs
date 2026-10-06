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
    metric::{MetricDefinition, MetricDirection, MetricInput, MetricStage, MetricTag, MetricUnits},
    period::{PerformancePeriod, PeriodFrameTotals, single_currency_share},
    statistic::PortfolioStatistic,
};

/// Calculates the gross return over a performance-period frame.
///
/// The return is the frame's net PnL with the commission added back, over the starting equity of
/// its first period. Equity carries the commission drag, so adding the recorded commission back is
/// what makes the figure gross, and the difference between this return and the `NetReturn` of the
/// same frame is exactly the cost the frame paid.
///
/// The return is defined only when the starting equity and the summed PnL resolve to the same
/// single currency and the starting equity is not zero. An empty frame, a frame whose starting
/// equity is zero, and a frame whose PnL spans more than one currency all have no defined return;
/// a frame that recorded no PnL and paid no commission has a genuine return of zero.
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
pub struct GrossReturn {}

impl GrossReturn {
    /// Creates a new [`GrossReturn`] instance.
    #[must_use]
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for GrossReturn {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for GrossReturn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Gross Return")
    }
}

impl PortfolioStatistic for GrossReturn {
    type Item = f64;

    fn name(&self) -> String {
        self.to_string()
    }

    fn definition(&self) -> MetricDefinition {
        MetricDefinition::new(
            "gross_return",
            "Gross Return",
            MetricUnits::Fraction,
            MetricDirection::Maximize,
            [MetricInput::PerformancePeriods],
        )
        .with_tags([MetricTag::Returns, MetricTag::Trade])
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
        let totals = PeriodFrameTotals::from_periods(periods)?;
        single_currency_share(&totals.starting_equity, &totals.gross_pnl)
    }
}

#[cfg(test)]
mod tests {
    use nautilus_core::{UnixNanos, approx_eq};
    use rstest::rstest;

    use super::*;
    use crate::{
        period::{
            CurrencyTotals, PeriodAccounting, PeriodActivity, PeriodExposure, PeriodPerformance,
        },
        statistics::net_return::NetReturn,
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

    /// Returns a frame over a 1,000 USD equity that lost 50 USD and paid 10 USD commission.
    fn frame() -> Vec<PerformancePeriod> {
        vec![
            period(
                &[(Currency::USD(), 1000.0)],
                &[(Currency::USD(), -30.0)],
                &[(Currency::USD(), 4.0)],
                &[(Currency::USD(), 5000.0)],
            ),
            period(
                &[],
                &[(Currency::USD(), -20.0)],
                &[(Currency::USD(), 6.0)],
                &[(Currency::USD(), 5000.0)],
            ),
        ]
    }

    #[rstest]
    fn test_empty_frame_is_undefined() {
        let statistic = GrossReturn::new();
        assert_eq!(statistic.calculate_from_periods(&[]), None);
    }

    #[rstest]
    fn test_gross_return_adds_the_commission_back_to_the_net_pnl() {
        let statistic = GrossReturn::new();
        let value = statistic.calculate_from_periods(&frame()).unwrap();

        // (-30 - 20 + 4 + 6) / 1000, i.e. the net 50 USD loss with the 10 USD commission added back.
        assert!(approx_eq!(f64, value, -0.04, epsilon = 1e-12));
    }

    #[rstest]
    fn test_gross_return_moves_onto_net_return_when_no_commission_is_paid() {
        let periods = vec![period(
            &[(Currency::USD(), 1000.0)],
            &[(Currency::USD(), -30.0)],
            &[],
            &[(Currency::USD(), 5000.0)],
        )];

        let gross = GrossReturn::new().calculate_from_periods(&periods).unwrap();
        let net = NetReturn::new().calculate_from_periods(&periods).unwrap();

        assert!(approx_eq!(f64, gross, -0.03, epsilon = 1e-12));
        assert!(approx_eq!(f64, gross, net, epsilon = 1e-12));
    }

    #[rstest]
    fn test_a_frame_whose_equity_is_not_single_currency_is_undefined() {
        let statistic = GrossReturn::new();
        let periods = vec![period(
            &[(Currency::USD(), 1000.0), (Currency::EUR(), 500.0)],
            &[(Currency::USD(), -30.0)],
            &[(Currency::USD(), 4.0)],
            &[],
        )];

        assert_eq!(statistic.calculate_from_periods(&periods), None);
    }

    #[rstest]
    fn test_a_frame_whose_pnl_is_not_in_the_equity_currency_is_undefined() {
        let statistic = GrossReturn::new();
        let periods = vec![period(
            &[(Currency::USD(), 1000.0)],
            &[(Currency::EUR(), -30.0)],
            &[],
            &[],
        )];

        assert_eq!(statistic.calculate_from_periods(&periods), None);
    }

    #[rstest]
    fn test_a_zero_starting_equity_is_undefined() {
        let statistic = GrossReturn::new();
        let periods = vec![period(
            &[(Currency::USD(), 0.0)],
            &[(Currency::USD(), -30.0)],
            &[(Currency::USD(), 4.0)],
            &[],
        )];

        assert_eq!(statistic.calculate_from_periods(&periods), None);
    }

    #[rstest]
    fn test_scaling_every_amount_leaves_the_return_unchanged() {
        let statistic = GrossReturn::new();
        let base = vec![period(
            &[(Currency::USD(), 1_000.0)],
            &[(Currency::USD(), -30.0)],
            &[(Currency::USD(), 10.0)],
            &[],
        )];
        let scaled = vec![period(
            &[(Currency::USD(), 50_000.0)],
            &[(Currency::USD(), -1_500.0)],
            &[(Currency::USD(), 500.0)],
            &[],
        )];

        let base_value = statistic.calculate_from_periods(&base).unwrap();
        let scaled_value = statistic.calculate_from_periods(&scaled).unwrap();

        // A 2% gross loss stays a 2% gross loss when the whole frame is scaled by fifty.
        assert!(approx_eq!(f64, base_value, -0.02, epsilon = 1e-12));
        assert!(approx_eq!(f64, scaled_value, base_value, epsilon = 1e-12));
    }
}
