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
use crate::{
    period::PerformancePeriod, statistic::PortfolioStatistic,
    statistics::exponentially_weighted_sharpe::ExponentiallyWeightedSharpe,
};

#[pymethods]
#[pyo3_stub_gen::derive::gen_stub_pymethods]
impl ExponentiallyWeightedSharpe {
    /// Calculates an exponentially weighted Sharpe ratio for portfolio returns.
    ///
    /// The ratio is the exponentially weighted mean return divided by the exponentially weighted
    /// standard deviation, annualised by the square root of the annualisation period:
    /// `ew_mean / ew_std * sqrt(annualisation)`.
    ///
    /// The weights decay by a factor of `0.5` every `halflife` observations, with the most recent
    /// observation carrying weight one. The weighted standard deviation uses the weighted population
    /// divisor (the sum of the weights), and the returns are first compounded to daily bins exactly
    /// as the arithmetic Sharpe ratio does.
    ///
    /// A series with no dispersion, or an empty series, has no defined ratio and returns `None`.
    /// Non-finite inputs propagate to a non-finite result rather than being dropped.
    #[new]
    #[pyo3(signature = (annualisation=None, halflife=None))]
    fn py_new(annualisation: Option<usize>, halflife: Option<usize>) -> Self {
        Self::new(annualisation, halflife)
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
    fn py_calculate_from_returns(&mut self, raw_returns: BTreeMap<u64, f64>) -> Option<f64> {
        self.calculate_from_returns(&transform_returns(&raw_returns))
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
    fn py_calculate_from_periods(&mut self, _periods: Vec<PerformancePeriod>) -> Option<f64> {
        None
    }
}
