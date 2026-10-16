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
use crate::{statistic::PortfolioStatistic, statistics::correction_impact::CorrectionImpactReport};

#[pymethods]
#[pyo3_stub_gen::derive::gen_stub_pymethods]
impl CorrectionImpactReport {
    /// Reports a correction's measured effect on a declared outcome metric.
    ///
    /// The report is the pair of measurements a correction is read through: `uncorrected` is the
    /// metric on the stream as the gate received it and `corrected` is the metric on the stream after
    /// the correction. The delta is their difference, so a correction that lowered a Sharpe ratio from
    /// 1.518 to 0.589 reports a delta of -0.929 beside the count of records it removed.
    #[new]
    #[pyo3(signature = (outcome_metric=None, uncorrected=0.0, corrected=0.0))]
    #[expect(clippy::needless_pass_by_value)]
    fn py_new(outcome_metric: Option<String>, uncorrected: f64, corrected: f64) -> PyResult<Self> {
        Self::new(outcome_metric.as_deref(), uncorrected, corrected)
            .map_err(|e| to_pyvalue_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        self.to_string()
    }

    #[getter]
    #[pyo3(name = "name")]
    fn py_name(&self) -> String {
        self.name()
    }

    /// Returns the declared outcome metric the correction was measured against.
    #[getter]
    #[pyo3(name = "metric")]
    fn py_metric(&self) -> String {
        self.metric().to_string()
    }

    /// Returns the metric's value on the stream as the gate received it.
    #[getter]
    #[pyo3(name = "uncorrected")]
    fn py_uncorrected(&self) -> f64 {
        self.uncorrected()
    }

    /// Returns the metric's value on the stream after the correction.
    #[getter]
    #[pyo3(name = "corrected")]
    fn py_corrected(&self) -> f64 {
        self.corrected()
    }

    /// Returns the change the correction made to the declared metric.
    ///
    /// The delta is `corrected - uncorrected`. It is exactly zero when the correction changed
    /// nothing, and it is reported rather than omitted.
    #[getter]
    #[pyo3(name = "delta")]
    fn py_delta(&self) -> f64 {
        self.delta()
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
}
