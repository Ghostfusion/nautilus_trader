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
    /// The rescaled range `R/S` of a series of length `m` is the range of its mean-adjusted
    /// cumulative deviation divided by its dispersion. For a window size `m` the series is
    /// split into non-overlapping blocks of `m` observations (a trailing partial block is
    /// dropped); within each block the mean-adjusted cumulative deviation is accumulated and
    /// `R = max(cumsum) - min(cumsum)`, while `S` is the population standard deviation of the
    /// block. The block contributes `R/S`, and the mean over the blocks is the value for `m`:
    ///
    /// `(R/S)_m = mean over blocks of ( max(cumsum) - min(cumsum) ) / S`
    ///
    /// The method uses a ladder of window sizes `m` that runs over the powers of two from `4`
    /// up to and including the largest power of two not exceeding `n / 2`. The Hurst estimate
    /// is the least-squares slope of `log((R/S)_m)` against `log(m)` over the ladder. A slope
    /// above `0.5` indicates persistence (the series trends), `0.5` is the memoryless value of
    /// a random walk, and a slope below `0.5` indicates anti-persistence (the series reverts).
    ///
    /// A block whose `S` is zero contributes nothing and is skipped, and a window size with no
    /// contributing block is dropped from the ladder. At least three ladder points with a
    /// finite positive `R/S` are required; otherwise the estimate is undefined and returns
    /// `None`. A series with fewer than `16` observations is likewise undefined.
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
