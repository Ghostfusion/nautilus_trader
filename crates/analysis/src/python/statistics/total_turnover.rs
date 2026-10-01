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
    period::PerformancePeriod, statistic::PortfolioStatistic,
    statistics::total_turnover::TotalTurnover,
};

#[pymethods]
#[pyo3_stub_gen::derive::gen_stub_pymethods]
impl TotalTurnover {
    /// Calculates the total turnover over a performance-period frame.
    ///
    /// Turnover is the sum of every period's notional turnover, which the frame records per currency.
    /// The total is defined only when those amounts resolve to a single currency; a frame whose
    /// turnover spans more than one currency, or that is empty, has no defined total and returns
    /// `None`. A frame with no trades has a genuine total of zero.
    #[new]
    fn py_new() -> Self {
        Self::new()
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
    fn py_calculate_from_positions(&mut self, _positions: Vec<Position>) -> Option<f64> {
        None
    }

    #[pyo3(name = "calculate_from_periods")]
    #[expect(clippy::needless_pass_by_value)]
    fn py_calculate_from_periods(&mut self, periods: Vec<PerformancePeriod>) -> Option<f64> {
        self.calculate_from_periods(&periods)
    }
}
