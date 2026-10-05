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

use nautilus_core::python::to_pyvalue_err;
use nautilus_model::position::Position;
use pyo3::prelude::*;

use super::transform_returns;
use crate::{statistic::PortfolioStatistic, statistics::autocorrelation::Autocorrelation};

#[pymethods]
#[pyo3_stub_gen::derive::gen_stub_pymethods]
impl Autocorrelation {
    /// Calculates the lag-`k` autocorrelation of portfolio returns.
    ///
    /// The autocorrelation at lag `k` measures how strongly each return is related to the
    /// return `k` observations later. It is the Pearson correlation of the series with its
    /// own `k`-lagged copy:
    ///
    /// `rho_k = sum_{t=1..n-k} (x_t - mu)(x_{t+k} - mu) / sum_{t=1..n} (x_t - mu)^2`
    ///
    /// where `mu` is the mean of the whole series and `n` is the number of returns. The
    /// mean and the denominator use the full series, while the numerator uses the `n - k`
    /// overlapping pairs. A positive value indicates persistence, a negative value indicates
    /// mean reversion, and zero indicates no linear dependence at that lag. Returns `None`
    /// for a series too short for the lag or with zero dispersion.
    #[new]
    #[pyo3(signature = (lag=None))]
    fn py_new(lag: Option<usize>) -> PyResult<Self> {
        Self::new(lag).map_err(to_pyvalue_err)
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
