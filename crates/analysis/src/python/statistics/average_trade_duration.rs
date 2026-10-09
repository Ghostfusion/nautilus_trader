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

use std::collections::BTreeMap;

use nautilus_model::position::Position;
use pyo3::prelude::*;

use crate::{
    statistic::PortfolioStatistic,
    statistics::average_trade_duration::{AverageTradeDuration, TradeOutcome},
};

#[pymethods]
#[pyo3_stub_gen::derive::gen_stub_pymethods]
impl AverageTradeDuration {
    /// Calculates the mean holding time of a set of closed trades, in calendar days.
    ///
    /// The mean is taken over the selected closed positions' `duration_ns`, converted to days. A
    /// position is closed when it carries a close timestamp (`ts_closed` is `Some`); an open position
    /// has no completed holding time and is excluded. Winners are closed positions with a positive
    /// realised PnL and losers those with a negative one; a breakeven or unresolved PnL is neither, so
    /// it is excluded from both selections. `None` is returned when the selection is empty.
    ///
    /// A long average holding time is generally a cost, so a smaller value is preferred.
    #[new]
    #[pyo3(signature = (outcome=None))]
    fn py_new(outcome: Option<TradeOutcome>) -> Self {
        Self::new(outcome)
    }

    fn __repr__(&self) -> String {
        self.to_string()
    }

    #[getter]
    #[pyo3(name = "name")]
    fn py_name(&self) -> String {
        self.name()
    }

    #[pyo3(name = "calculate_from_returns")]
    fn py_calculate_from_returns(&mut self, _returns: BTreeMap<u64, f64>) -> Option<f64> {
        None
    }

    #[pyo3(name = "calculate_from_realized_pnls")]
    fn py_calculate_from_realized_pnls(&mut self, _realized_pnls: Vec<f64>) -> Option<f64> {
        None
    }

    #[pyo3(name = "calculate_from_positions")]
    #[expect(clippy::needless_pass_by_value)]
    fn py_calculate_from_positions(&mut self, positions: Vec<Position>) -> Option<f64> {
        self.calculate_from_positions(&positions)
    }
}
