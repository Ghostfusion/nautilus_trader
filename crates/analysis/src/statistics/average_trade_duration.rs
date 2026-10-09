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

/// The number of nanoseconds in one UTC day.
const NANOS_PER_DAY: u64 = 86_400_000_000_000;

/// The subset of closed trades an average duration is taken over.
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
pub enum TradeOutcome {
    /// Every closed trade, whatever its outcome.
    All,
    /// Only closed trades that realised a positive PnL.
    Winners,
    /// Only closed trades that realised a negative PnL.
    Losers,
}

impl TradeOutcome {
    /// All variants of the closed vocabulary.
    pub const ALL: &'static [Self] = &[Self::All, Self::Winners, Self::Losers];

    /// Returns the stable string for this outcome.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Winners => "winners",
            Self::Losers => "losers",
        }
    }
}

impl Display for TradeOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Calculates the mean holding time of a set of closed trades, in calendar days.
///
/// The mean is taken over the selected closed positions' `duration_ns`, converted to days. A
/// position is closed when it carries a close timestamp (`ts_closed` is `Some`); an open position
/// has no completed holding time and is excluded. Winners are closed positions with a positive
/// realised PnL and losers those with a negative one; a breakeven or unresolved PnL is neither, so
/// it is excluded from both selections. `None` is returned when the selection is empty.
///
/// A long average holding time is generally a cost, so a smaller value is preferred.
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
pub struct AverageTradeDuration {
    outcome: TradeOutcome,
}

impl AverageTradeDuration {
    /// Creates a new [`AverageTradeDuration`] instance.
    ///
    /// `None` selects every closed trade.
    #[must_use]
    pub fn new(outcome: Option<TradeOutcome>) -> Self {
        Self {
            outcome: outcome.unwrap_or(TradeOutcome::All),
        }
    }
}

impl Display for AverageTradeDuration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Average Trade Duration ({}, days)", self.outcome)
    }
}

impl PortfolioStatistic for AverageTradeDuration {
    type Item = f64;

    fn name(&self) -> String {
        self.to_string()
    }

    fn definition(&self) -> MetricDefinition {
        MetricDefinition::new(
            "average_trade_duration",
            "Average Trade Duration ({outcome}, {unit})",
            MetricUnits::Ratio,
            MetricDirection::Minimize,
            [MetricInput::Positions],
        )
        .with_parameter("outcome", self.outcome.as_str())
        .with_parameter("unit", "days")
        .with_tags([MetricTag::Trade])
        .with_stage(MetricStage::Decision)
    }

    fn calculate_from_returns(&self, _returns: &Returns) -> Option<Self::Item> {
        None
    }

    fn calculate_from_realized_pnls(&self, _realized_pnls: &[f64]) -> Option<Self::Item> {
        None
    }

    fn calculate_from_positions(&self, positions: &[Position]) -> Option<Self::Item> {
        let mut total_days = 0.0;
        let mut count = 0_u64;

        for position in positions {
            if position.ts_closed.is_none() {
                continue;
            }

            let selected = match self.outcome {
                TradeOutcome::All => true,
                TradeOutcome::Winners => {
                    position.realized_pnl.is_some_and(|pnl| pnl.as_f64() > 0.0)
                }
                TradeOutcome::Losers => position.realized_pnl.is_some_and(|pnl| pnl.as_f64() < 0.0),
            };

            if !selected {
                continue;
            }

            total_days += position.duration_ns.as_u64() as f64 / NANOS_PER_DAY as f64;
            count += 1;
        }

        if count == 0 {
            return None;
        }

        Some(total_days / count as f64)
    }
}

#[cfg(test)]
mod tests {
    use ahash::AHashSet;
    use indexmap::IndexMap;
    use nautilus_core::{DurationNanos, UnixNanos, approx_eq};
    use nautilus_model::{
        enums::{InstrumentClass, OrderSide, PositionSide},
        identifiers::{
            AccountId, ClientOrderId, PositionId,
            stubs::{instrument_id_aud_usd_sim, strategy_id_ema_cross, trader_id},
        },
        stubs::TestDefault,
        types::{Currency, Money, Quantity},
    };
    use rstest::rstest;

    use super::*;

    /// Creates a closed position with the given holding time and realised PnL.
    fn closed_position(duration_ns: u64, realized_pnl: Money) -> Position {
        Position {
            events: Vec::new(),
            replay_events: Vec::new(),
            fill_voids: Vec::new(),
            trader_id: trader_id(),
            strategy_id: strategy_id_ema_cross(),
            instrument_id: instrument_id_aud_usd_sim(),
            id: PositionId::new("test-position"),
            account_id: AccountId::new("test-account"),
            opening_order_id: ClientOrderId::test_default(),
            closing_order_id: None,
            entry: OrderSide::Buy,
            side: PositionSide::Flat,
            signed_qty: 0.0,
            quantity: Quantity::default(),
            peak_qty: Quantity::default(),
            price_precision: 2,
            size_precision: 2,
            multiplier: Quantity::default(),
            is_inverse: false,
            base_currency: None,
            quote_currency: Currency::USD(),
            settlement_currency: Currency::USD(),
            ts_init: UnixNanos::default(),
            ts_opened: UnixNanos::default(),
            ts_last: UnixNanos::default(),
            ts_closed: Some(UnixNanos::from(1)),
            duration_ns: DurationNanos::new(duration_ns),
            avg_px_open: 0.0,
            avg_px_close: Some(0.0),
            realized_return: 0.0,
            realized_pnl: Some(realized_pnl),
            trade_ids: AHashSet::new(),
            buy_qty: Quantity::default(),
            sell_qty: Quantity::default(),
            commissions: IndexMap::new(),
            adjustments: Vec::new(),
            instrument_class: InstrumentClass::Spot,
            is_currency_pair: true,
        }
    }

    fn pnl(value: f64) -> Money {
        Money::new(value, Currency::USD())
    }

    #[rstest]
    fn test_empty_positions_is_undefined() {
        let statistic = AverageTradeDuration::new(None);
        assert_eq!(statistic.calculate_from_positions(&[]), None);
    }

    #[rstest]
    fn test_all_outcome_averages_every_closed_trade() {
        let statistic = AverageTradeDuration::new(None);
        let positions = vec![
            closed_position(NANOS_PER_DAY, pnl(10.0)),
            closed_position(3 * NANOS_PER_DAY, pnl(-5.0)),
        ];

        let result = statistic.calculate_from_positions(&positions);
        assert!(result.is_some());
        assert!(approx_eq!(f64, result.unwrap(), 2.0, epsilon = 1e-9));
    }

    #[rstest]
    fn test_winners_selects_positive_pnl_only() {
        let statistic = AverageTradeDuration::new(Some(TradeOutcome::Winners));
        let positions = vec![
            closed_position(NANOS_PER_DAY, pnl(10.0)),
            closed_position(3 * NANOS_PER_DAY, pnl(-5.0)),
            closed_position(5 * NANOS_PER_DAY, pnl(2.0)),
        ];

        let result = statistic.calculate_from_positions(&positions);
        assert!(result.is_some());
        assert!(approx_eq!(f64, result.unwrap(), 3.0, epsilon = 1e-9));
    }

    #[rstest]
    fn test_losers_selects_negative_pnl_only() {
        let statistic = AverageTradeDuration::new(Some(TradeOutcome::Losers));
        let positions = vec![
            closed_position(NANOS_PER_DAY, pnl(10.0)),
            closed_position(3 * NANOS_PER_DAY, pnl(-5.0)),
            closed_position(7 * NANOS_PER_DAY, pnl(-1.0)),
        ];

        let result = statistic.calculate_from_positions(&positions);
        assert!(result.is_some());
        assert!(approx_eq!(f64, result.unwrap(), 5.0, epsilon = 1e-9));
    }

    #[rstest]
    fn test_open_positions_are_excluded() {
        let statistic = AverageTradeDuration::new(None);
        let mut open = closed_position(9 * NANOS_PER_DAY, pnl(10.0));
        open.ts_closed = None;

        let positions = vec![open, closed_position(NANOS_PER_DAY, pnl(1.0))];

        let result = statistic.calculate_from_positions(&positions);
        assert!(result.is_some());
        assert!(approx_eq!(f64, result.unwrap(), 1.0, epsilon = 1e-9));
    }

    #[rstest]
    fn test_winners_without_a_winner_is_undefined() {
        let statistic = AverageTradeDuration::new(Some(TradeOutcome::Winners));
        let positions = vec![closed_position(NANOS_PER_DAY, pnl(-1.0))];

        assert_eq!(statistic.calculate_from_positions(&positions), None);
    }

    #[rstest]
    fn test_returns_and_pnls_are_unsupported() {
        let statistic = AverageTradeDuration::new(None);
        assert_eq!(statistic.calculate_from_returns(&Returns::default()), None);
        assert_eq!(statistic.calculate_from_realized_pnls(&[1.0, -1.0]), None);
    }

    #[rstest]
    fn test_name_and_definition() {
        let statistic = AverageTradeDuration::new(Some(TradeOutcome::Winners));
        assert_eq!(statistic.name(), "Average Trade Duration (winners, days)");

        let definition = statistic.definition();
        assert_eq!(definition.id(), "average_trade_duration");
        assert_eq!(definition.title(), statistic.name());
    }
}
