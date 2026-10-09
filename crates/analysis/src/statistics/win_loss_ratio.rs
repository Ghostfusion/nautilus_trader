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
    statistic::PortfolioStatistic,
};

/// Calculates the ratio of the average winning trade to the average losing trade.
///
/// The numerator is the mean of the positive realized PnLs and the denominator the absolute mean
/// of the negative ones. A ratio below one means the average loss is larger than the average win,
/// so a win rate at or below a half cannot be profitable. The ratio is `None` when either side is
/// empty: a ratio with no losing trade has no denominator, and one with no winning trade has no
/// numerator. A breakeven or missing PnL is neither a win nor a loss and is excluded from both.
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
pub struct WinLossRatio {}

impl Display for WinLossRatio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Win/Loss Ratio")
    }
}

impl PortfolioStatistic for WinLossRatio {
    type Item = f64;

    fn name(&self) -> String {
        self.to_string()
    }

    fn definition(&self) -> MetricDefinition {
        MetricDefinition::new(
            "win_loss_ratio",
            "Win/Loss Ratio",
            MetricUnits::Ratio,
            MetricDirection::Maximize,
            [MetricInput::RealizedPnls],
        )
        .with_tags([MetricTag::Trade])
        .with_stage(MetricStage::Decision)
    }

    fn calculate_from_returns(&self, _returns: &Returns) -> Option<Self::Item> {
        None
    }

    fn calculate_from_realized_pnls(&self, realized_pnls: &[f64]) -> Option<Self::Item> {
        let mut win_sum = 0.0;
        let mut win_count = 0_u64;
        let mut loss_sum = 0.0;
        let mut loss_count = 0_u64;

        for &pnl in realized_pnls {
            if pnl > 0.0 {
                win_sum += pnl;
                win_count += 1;
            } else if pnl < 0.0 {
                loss_sum += pnl;
                loss_count += 1;
            }
        }

        if win_count == 0 || loss_count == 0 {
            return None;
        }

        let average_win = win_sum / win_count as f64;
        let average_loss = loss_sum / loss_count as f64;

        Some(average_win / average_loss.abs())
    }

    fn calculate_from_positions(&self, _positions: &[Position]) -> Option<Self::Item> {
        None
    }
}

#[cfg(test)]
mod tests {
    use nautilus_core::approx_eq;
    use rstest::rstest;

    use super::*;

    #[rstest]
    fn test_empty_pnls_is_undefined() {
        let statistic = WinLossRatio {};
        assert_eq!(statistic.calculate_from_realized_pnls(&[]), None);
    }

    #[rstest]
    fn test_hand_computed_ratio() {
        let statistic = WinLossRatio {};
        // Winners average 150, losers average -75: 150 / 75 = 2.
        let realized_pnls = vec![100.0, 200.0, -50.0, -100.0];

        let result = statistic.calculate_from_realized_pnls(&realized_pnls);
        assert!(result.is_some());
        assert!(approx_eq!(f64, result.unwrap(), 2.0, epsilon = 1e-9));
    }

    #[rstest]
    fn test_without_a_losing_trade_is_undefined() {
        let statistic = WinLossRatio {};
        assert_eq!(
            statistic.calculate_from_realized_pnls(&[100.0, 50.0, 200.0]),
            None
        );
    }

    #[rstest]
    fn test_without_a_winning_trade_is_undefined() {
        let statistic = WinLossRatio {};
        assert_eq!(
            statistic.calculate_from_realized_pnls(&[-100.0, -50.0, -200.0]),
            None
        );
    }

    #[rstest]
    fn test_breakeven_trades_are_excluded() {
        let statistic = WinLossRatio {};
        // The zero PnL is neither a win nor a loss, so the two sides are unchanged.
        let realized_pnls = vec![100.0, 0.0, -50.0];

        let result = statistic.calculate_from_realized_pnls(&realized_pnls);
        assert!(result.is_some());
        assert!(approx_eq!(f64, result.unwrap(), 2.0, epsilon = 1e-9));
    }

    #[rstest]
    fn test_returns_and_positions_are_unsupported() {
        let statistic = WinLossRatio {};
        assert_eq!(statistic.calculate_from_returns(&Returns::default()), None);
        assert_eq!(statistic.calculate_from_positions(&[]), None);
    }

    #[rstest]
    fn test_name_and_definition() {
        let statistic = WinLossRatio {};
        assert_eq!(statistic.name(), "Win/Loss Ratio");

        let definition = statistic.definition();
        assert_eq!(definition.id(), "win_loss_ratio");
        assert_eq!(definition.title(), statistic.name());
    }
}
