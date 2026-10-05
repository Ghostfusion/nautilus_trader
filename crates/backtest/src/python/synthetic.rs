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

//! Python bindings for the synthetic order-flow generator.

use nautilus_core::python::to_pyvalue_err;
use pyo3::prelude::*;

use crate::synthetic::{SyntheticFlow, SyntheticFlowConfig};

#[pyo3_stub_gen::derive::gen_stub_pymethods]
#[pyo3::pymethods]
impl SyntheticFlowConfig {
    /// Synthetic order-flow configuration calibrated to a target Hurst exponent and impact
    /// exponent.
    ///
    /// # Errors
    ///
    /// Returns a ``ValueError`` if ``target_hurst`` is not finite and strictly between 0.5 and
    /// 1.0 exclusive, if ``impact_exponent`` is not finite and within the half-open range
    /// ``(0.0, 0.5]``, or if ``count`` is below 2.
    #[new]
    fn py_new(target_hurst: f64, impact_exponent: f64, count: usize, seed: u64) -> PyResult<Self> {
        Self::new(target_hurst, impact_exponent, count, seed).map_err(to_pyvalue_err)
    }

    /// Returns the target Hurst exponent of the flow.
    #[getter]
    #[pyo3(name = "target_hurst")]
    const fn py_target_hurst(&self) -> f64 {
        self.target_hurst()
    }

    /// Returns the impact exponent applied to the flow.
    #[getter]
    #[pyo3(name = "impact_exponent")]
    const fn py_impact_exponent(&self) -> f64 {
        self.impact_exponent()
    }

    /// Returns the number of periods generated.
    #[getter]
    #[pyo3(name = "count")]
    const fn py_count(&self) -> usize {
        self.count()
    }

    /// Returns the seed for the deterministic white-noise draws.
    #[getter]
    #[pyo3(name = "seed")]
    const fn py_seed(&self) -> u64 {
        self.seed()
    }

    /// Generates a ``SyntheticFlow`` from this configuration.
    ///
    /// The draws are deterministic under the configured seed.
    ///
    /// # Errors
    ///
    /// Returns a ``ValueError`` if generation fails.
    #[pyo3(name = "generate")]
    fn py_generate(&self) -> PyResult<SyntheticFlow> {
        self.generate().map_err(to_pyvalue_err)
    }
}

#[pyo3_stub_gen::derive::gen_stub_pymethods]
#[pyo3::pymethods]
impl SyntheticFlow {
    /// Returns the per-period signed flow.
    #[getter]
    #[pyo3(name = "quantities")]
    fn py_quantities(&self) -> Vec<f64> {
        self.quantities().to_vec()
    }

    /// Returns the induced price path, beginning at ``0.0``.
    #[getter]
    #[pyo3(name = "prices")]
    fn py_prices(&self) -> Vec<f64> {
        self.prices().to_vec()
    }

    /// Returns the target Hurst exponent of the flow.
    #[getter]
    #[pyo3(name = "target_hurst")]
    const fn py_target_hurst(&self) -> f64 {
        self.target_hurst()
    }

    /// Returns the impact exponent applied to the flow.
    #[getter]
    #[pyo3(name = "impact_exponent")]
    const fn py_impact_exponent(&self) -> f64 {
        self.impact_exponent()
    }

    /// Returns the seed used for the white-noise draws.
    #[getter]
    #[pyo3(name = "seed")]
    const fn py_seed(&self) -> u64 {
        self.seed()
    }
}
