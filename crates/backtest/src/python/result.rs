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

//! Python bindings for the [`BacktestResult`] type.

use std::collections::{BTreeMap, HashMap};

use nautilus_core::{UUID4, python::to_pyruntime_err};
use pyo3::{Py, PyResult, Python, pybacked::PyBackedBytes, types::PyBytes};

use crate::result::{BacktestResult, CanonicalBacktestResult};

#[pyo3_stub_gen::derive::gen_stub_pymethods]
#[pyo3::pymethods]
impl BacktestResult {
    #[getter]
    #[pyo3(name = "trader_id")]
    fn py_trader_id(&self) -> &str {
        &self.trader_id
    }

    #[getter]
    #[pyo3(name = "machine_id")]
    fn py_machine_id(&self) -> &str {
        &self.machine_id
    }

    #[getter]
    #[pyo3(name = "instance_id")]
    const fn py_instance_id(&self) -> UUID4 {
        self.instance_id
    }

    #[getter]
    #[pyo3(name = "run_config_id")]
    fn py_run_config_id(&self) -> Option<&str> {
        self.run_config_id.as_deref()
    }

    #[getter]
    #[pyo3(name = "run_id")]
    const fn py_run_id(&self) -> Option<UUID4> {
        self.run_id
    }

    #[getter]
    #[pyo3(name = "run_started")]
    fn py_run_started(&self) -> Option<u64> {
        self.run_started.map(|timestamp| timestamp.as_u64())
    }

    #[getter]
    #[pyo3(name = "run_finished")]
    fn py_run_finished(&self) -> Option<u64> {
        self.run_finished.map(|timestamp| timestamp.as_u64())
    }

    #[getter]
    #[pyo3(name = "backtest_start")]
    fn py_backtest_start(&self) -> Option<u64> {
        self.backtest_start.map(|timestamp| timestamp.as_u64())
    }

    #[getter]
    #[pyo3(name = "backtest_end")]
    fn py_backtest_end(&self) -> Option<u64> {
        self.backtest_end.map(|timestamp| timestamp.as_u64())
    }

    #[getter]
    #[pyo3(name = "elapsed_time_secs")]
    const fn py_elapsed_time_secs(&self) -> f64 {
        self.elapsed_time_secs
    }

    #[getter]
    #[pyo3(name = "iterations")]
    const fn py_iterations(&self) -> usize {
        self.iterations
    }

    #[getter]
    #[pyo3(name = "total_events")]
    const fn py_total_events(&self) -> usize {
        self.total_events
    }

    #[getter]
    #[pyo3(name = "total_orders")]
    const fn py_total_orders(&self) -> usize {
        self.total_orders
    }

    #[getter]
    #[pyo3(name = "total_positions")]
    const fn py_total_positions(&self) -> usize {
        self.total_positions
    }

    #[getter]
    #[pyo3(name = "summary")]
    fn py_summary(&self) -> HashMap<String, String> {
        self.summary
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect()
    }

    #[getter]
    #[pyo3(name = "stats_pnls")]
    fn py_stats_pnls(&self) -> HashMap<String, HashMap<String, f64>> {
        self.stats_pnls
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    v.iter().map(|(k2, v2)| (k2.clone(), *v2)).collect(),
                )
            })
            .collect()
    }

    #[getter]
    #[pyo3(name = "stats_returns")]
    fn py_stats_returns(&self) -> HashMap<String, f64> {
        self.stats_returns
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect()
    }

    #[getter]
    #[pyo3(name = "stats_general")]
    fn py_stats_general(&self) -> HashMap<String, f64> {
        self.stats_general
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect()
    }

    #[getter]
    #[pyo3(name = "returns_series")]
    fn py_returns_series(&self) -> BTreeMap<u64, f64> {
        self.returns_series
            .iter()
            .map(|(timestamp, value)| (timestamp.as_u64(), *value))
            .collect()
    }

    fn __repr__(&self) -> String {
        format!(
            "BacktestResult(trader_id='{}', elapsed={:.2}s, iterations={}, orders={}, positions={})",
            self.trader_id,
            self.elapsed_time_secs,
            self.iterations,
            self.total_orders,
            self.total_positions,
        )
    }
}

#[pyo3_stub_gen::derive::gen_stub_pymethods]
#[pyo3::pymethods]
impl CanonicalBacktestResult {
    /// Returns the canonical compact UTF-8 JSON bytes without trailing data.
    ///
    /// # Errors
    ///
    /// Returns an error if the in-memory document cannot be serialized.
    #[pyo3(name = "to_bytes")]
    fn py_to_bytes(&self, py: Python<'_>) -> PyResult<Py<PyBytes>> {
        let bytes = self.to_bytes().map_err(to_pyruntime_err)?;
        Ok(PyBytes::new(py, &bytes).into())
    }

    /// Returns `blake3:` followed by the 32-byte BLAKE3 digest as 64 lowercase hex digits.
    ///
    /// # Errors
    ///
    /// Returns an error if the in-memory document cannot be serialized.
    #[pyo3(name = "digest")]
    fn py_digest(&self) -> PyResult<String> {
        self.digest().map_err(to_pyruntime_err)
    }

    /// Returns the first field-level or record-level difference.
    #[pyo3(name = "first_divergence")]
    #[expect(
        clippy::needless_pass_by_value,
        reason = "PyBackedBytes is required for generated Python bytes stubs"
    )]
    #[expect(clippy::type_complexity)]
    fn py_first_divergence(
        &self,
        expected: PyBackedBytes,
    ) -> PyResult<Option<(String, Option<String>, Option<String>)>> {
        let expected = Self::from_slice(expected.as_ref()).map_err(to_pyruntime_err)?;
        Ok(expected.first_divergence(self).map(|divergence| {
            (
                divergence.path,
                divergence.expected.map(|value| value.to_string()),
                divergence.actual.map(|value| value.to_string()),
            )
        }))
    }
}
