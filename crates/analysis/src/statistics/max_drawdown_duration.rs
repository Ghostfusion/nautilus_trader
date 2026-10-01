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
    metric::{MetricDefinition, MetricDirection, MetricInput, MetricTag, MetricUnits},
    period::PerformancePeriod,
    statistic::PortfolioStatistic,
};

/// The number of nanoseconds in one UTC day.
const NANOS_PER_DAY: u64 = 86_400_000_000_000;

/// Calculates the duration of the maximum drawdown over an equity series.
///
/// The duration is the interval, in calendar days, from the equity peak preceding the maximum
/// drawdown's trough to that trough. The equity series is the ending equity of each performance
/// period in the frame, which is read as a single currency; a frame whose equity does not resolve
/// to one currency, or that carries fewer than two periods, has no defined duration and returns
/// `None`. A series that never declines has a genuine duration of zero.
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
pub struct MaxDrawdownDuration {}

impl MaxDrawdownDuration {
    /// Creates a new [`MaxDrawdownDuration`] instance.
    #[must_use]
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for MaxDrawdownDuration {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for MaxDrawdownDuration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Max Drawdown Duration (days)")
    }
}

impl PortfolioStatistic for MaxDrawdownDuration {
    type Item = f64;

    fn name(&self) -> String {
        self.to_string()
    }

    fn definition(&self) -> MetricDefinition {
        MetricDefinition::new(
            "max_drawdown_duration",
            "Max Drawdown Duration ({unit})",
            MetricUnits::Ratio,
            MetricDirection::Minimize,
            [MetricInput::PerformancePeriods],
        )
        .with_parameter("unit", "days")
        .with_tags([MetricTag::Drawdown])
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
        if periods.len() < 2 {
            return None;
        }

        let mut series = Vec::with_capacity(periods.len());

        for period in periods {
            let equity = period.accounting.ending_equity.as_single_currency()?;
            series.push((period.accounting.end.as_u64(), equity));
        }

        let currency = series[0].1.currency;
        if series.iter().any(|(_, equity)| equity.currency != currency) {
            return None;
        }

        let mut peak = series[0].1.as_decimal();
        let mut peak_timestamp = series[0].0;
        let mut max_decline = rust_decimal::Decimal::ZERO;
        let mut duration_nanos = 0_u64;

        for (timestamp, equity) in &series[1..] {
            let value = equity.as_decimal();

            if value > peak {
                peak = value;
                peak_timestamp = *timestamp;
                continue;
            }

            let decline = peak - value;
            if decline > max_decline {
                max_decline = decline;
                duration_nanos = timestamp - peak_timestamp;
            }
        }

        Some(duration_nanos as f64 / NANOS_PER_DAY as f64)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::period::{
        CurrencyTotals, PeriodAccounting, PeriodActivity, PeriodExposure, PeriodPerformance,
    };
    use nautilus_core::UnixNanos;
    use nautilus_model::types::{Currency, Money};

    const BASE_NS: u64 = 1_600_000_000_000_000_000;

    fn period(day: u64, equity: f64) -> PerformancePeriod {
        let mut ending_equity = CurrencyTotals::new();
        ending_equity.add_money(Money::new(equity, Currency::USD()));

        PerformancePeriod {
            accounting: PeriodAccounting {
                start: UnixNanos::from(BASE_NS + day * NANOS_PER_DAY),
                end: UnixNanos::from(BASE_NS + (day + 1) * NANOS_PER_DAY),
                starting_equity: CurrencyTotals::new(),
                ending_equity,
                realized_pnl: CurrencyTotals::new(),
                unrealized_pnl: CurrencyTotals::new(),
                commission: CurrencyTotals::new(),
                fees: CurrencyTotals::new(),
                slippage: CurrencyTotals::new(),
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
                net_return: None,
                drawdown: CurrencyTotals::new(),
                drawdown_percentage: None,
            },
        }
    }

    #[rstest]
    fn test_empty_frame_is_undefined() {
        let statistic = MaxDrawdownDuration::new();
        assert_eq!(statistic.calculate_from_periods(&[]), None);
    }

    #[rstest]
    fn test_monotonic_equity_has_zero_duration() {
        let statistic = MaxDrawdownDuration::new();
        let periods = vec![period(0, 100.0), period(1, 110.0), period(2, 120.0)];
        assert_eq!(statistic.calculate_from_periods(&periods), Some(0.0));
    }

    #[rstest]
    fn test_duration_is_peak_to_trough() {
        // The peak of 120 is carried by the period ending at the day 3 boundary, and the trough
        // of 90 by the period ending at the day 6 boundary: a duration of three calendar days.
        let statistic = MaxDrawdownDuration::new();
        let periods = vec![
            period(0, 100.0),
            period(1, 110.0),
            period(2, 120.0),
            period(3, 115.0),
            period(4, 100.0),
            period(5, 90.0),
            period(6, 95.0),
        ];
        assert_eq!(statistic.calculate_from_periods(&periods), Some(3.0));
    }
}
