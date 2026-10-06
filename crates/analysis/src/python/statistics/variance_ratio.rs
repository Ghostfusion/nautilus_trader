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
use crate::{statistic::PortfolioStatistic, statistics::variance_ratio::VarianceRatio};

#[pymethods]
#[pyo3_stub_gen::derive::gen_stub_pymethods]
impl VarianceRatio {
    /// Calculates the variance ratio of portfolio returns over an aggregation scale `q`.
    ///
    /// The variance ratio compares the variance of `q`-period aggregated returns to `q`
    /// times the variance of one-period returns. With `Var_1 = (1/n) * sum (x_t - mu)^2`,
    /// `mu` the full-series mean, and `S_t = sum_{i=t..t+q-1} x_i` the overlapping `q`-period
    /// sums for `t = 1..=n-q+1`:
    ///
    /// `VR(q) = [ (1/(n-q+1)) * sum_t (S_t - q*mu)^2 ] / (q * Var_1)`
    ///
    /// The aggregation sums `S_t` overlap rather than tile the series, which keeps every
    /// observation in the estimate. Both variances divide by the number of observations
    /// (the population divisor), so this is the finite-sample (biased) estimator rather than
    /// an unbiased correction.
    ///
    /// The diffusive value of the tool is `1.0`. A value above `1.0` indicates persistence on
    /// the aggregation scale `q` (aggregated variance grows faster than linearly in `q`, as a
    /// trend or momentum would produce), and a value below `1.0` indicates anti-persistence
    /// (mean reversion, as an alternating series would produce).
    ///
    /// Returns `None` when `n < 2q` (too few observations to aggregate) or when `Var_1` is
    /// zero (the ratio is undefined).
    #[new]
    #[pyo3(signature = (period=None))]
    fn py_new(period: Option<usize>) -> PyResult<Self> {
        Self::new(period).map_err(to_pyvalue_err)
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
