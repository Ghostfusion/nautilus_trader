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

//! Python bindings for square-root impact prefactor calibration.

use nautilus_core::python::to_pyvalue_err;
use nautilus_model::types::Quantity;
use pyo3::prelude::*;

use crate::models::market_impact::PrefactorInterval;
use crate::models::market_impact_calibration::{
    ImpactObservation, fit_prefactor_from_fills, fit_prefactor_from_tape,
};

#[pymethods]
#[pyo3_stub_gen::derive::gen_stub_pymethods]
impl ImpactObservation {
    /// One observed price impact, with the size that caused it and the volume it is measured
    /// against.
    ///
    /// The quantity is the aggressive metaorder's own consumption rather than the clip that
    /// happened to print, and `increments` is the price movement it left behind in whole price
    /// increments.
    #[new]
    #[pyo3(signature = (quantity, reference_quantity, increments))]
    fn py_new(quantity: Quantity, reference_quantity: Quantity, increments: u64) -> PyResult<Self> {
        Self::new(quantity, reference_quantity, increments).map_err(to_pyvalue_err)
    }

    /// The aggressive quantity whose consumption caused the impact.
    #[getter]
    #[pyo3(name = "quantity")]
    fn py_quantity(&self) -> Quantity {
        self.quantity()
    }

    /// The volume the quantity is measured against.
    #[getter]
    #[pyo3(name = "reference_quantity")]
    fn py_reference_quantity(&self) -> Quantity {
        self.reference_quantity()
    }

    /// The observed price movement in whole price increments.
    #[getter]
    #[pyo3(name = "increments")]
    fn py_increments(&self) -> u64 {
        self.increments()
    }

    /// The prefactor this observation implies, or `None` when it has no usable estimate.
    #[getter]
    #[pyo3(name = "prefactor")]
    fn py_prefactor(&self) -> Option<f64> {
        self.prefactor()
    }

    fn __repr__(&self) -> String {
        format!("{self:?}")
    }
}

/// Fits a prefactor from observations taken where the aggressor and the metaorder are observable.
///
/// The fitted interval spans the per-observation estimates and its source records that the
/// prefactor was measured, so the result needs no de-bias.
///
/// # Errors
///
/// Returns a `ValueError` if the observations are empty or if the fitted span is not a usable
/// interval.
#[pyfunction]
#[pyo3_stub_gen::derive::gen_stub_pyfunction(module = "nautilus_trader.execution")]
#[pyo3(name = "fit_prefactor_from_fills")]
#[expect(clippy::needless_pass_by_value)]
pub fn py_fit_prefactor_from_fills(
    observations: Vec<ImpactObservation>,
) -> PyResult<PrefactorInterval> {
    fit_prefactor_from_fills(&observations).map_err(to_pyvalue_err)
}

/// Fits a prefactor from observations reconstructed from an anonymous tape.
///
/// The fitted interval spans the per-observation estimates and its source records that the
/// prefactor was inferred from reconstructed metaorders, so the result carries the reconstruction's
/// inflation until `debiased` is applied to it.
///
/// # Errors
///
/// Returns a `ValueError` if the observations are empty or if the fitted span is not a usable
/// interval.
#[pyfunction]
#[pyo3_stub_gen::derive::gen_stub_pyfunction(module = "nautilus_trader.execution")]
#[pyo3(name = "fit_prefactor_from_tape")]
#[expect(clippy::needless_pass_by_value)]
pub fn py_fit_prefactor_from_tape(
    observations: Vec<ImpactObservation>,
) -> PyResult<PrefactorInterval> {
    fit_prefactor_from_tape(&observations).map_err(to_pyvalue_err)
}
