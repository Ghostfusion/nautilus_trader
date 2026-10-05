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

use super::transform_returns;
use crate::{statistic::PortfolioStatistic, statistics::rescaled_range::RescaledRange};

#[pymethods]
#[pyo3_stub_gen::derive::gen_stub_pymethods]
impl RescaledRange {
    /// Calculates the Hurst exponent of portfolio returns by the rescaled-range method.
    ///
    /// The estimate is the least-squares slope of `log(mean R/S)` against `log(m)` over a
    /// ladder of power-of-two window sizes `m` from `4` up to the largest power of two not
    /// exceeding `n / 2`, where `R/S` is the rescaled range averaged over the
    /// non-overlapping blocks of each window. A slope above `0.5` indicates persistence,
    /// `0.5` is the memoryless value, and below `0.5` indicates anti-persistence. Returns
    /// `None` when fewer than three usable ladder points exist or the series has fewer than
    /// `16` observations.
    #[new]
    fn py_new() -> Self {
        Self::new()
    }

    fn __repr__(&self) -> String {
        self.name()
    }

    #[getter]
    #[pyo3(name = "name")]
    fn py_name(&self) -> String {
        self.name()
    }

    #[pyo3(name = "calculate_from_returns")]
    #[expect(clippy::needless_pass_by_value)]
    fn py_calculate_from_returns(&self, raw_returns: BTreeMap<u64, f64>) -> Option<f64> {
        self.calculate_from_returns(&transform_returns(&raw_returns))
    }

    #[pyo3(name = "calculate_from_realized_pnls")]
    fn py_calculate_from_realized_pnls(&self, _realized_pnls: Vec<f64>) -> Option<f64> {
        None
    }

    #[pyo3(name = "calculate_from_positions")]
    fn py_calculate_from_positions(&self, _positions: Vec<Position>) -> Option<f64> {
        None
    }
}
