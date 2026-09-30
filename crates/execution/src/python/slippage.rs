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

//! Python bindings for slippage model types.

use nautilus_core::python::{to_pyruntime_err, to_pytype_err};
use pyo3::prelude::*;

use crate::models::slippage::{ProbabilisticSlippageModel, SlippageModelAny};

#[pymethods]
#[pyo3_stub_gen::derive::gen_stub_pymethods]
impl ProbabilisticSlippageModel {
    /// A probabilistic one-tick slippage model.
    ///
    /// The model draws a one-tick adverse adjustment with probability `prob_slippage` on each L1
    /// fill. It draws from the same seeded probabilistic state the fill models use, so a seeded
    /// model reproduces its draws across runs and the same probability and seed as a composite fill
    /// model's folded-in slippage produce the same draws.
    #[new]
    #[pyo3(signature = (prob_slippage=0.0, random_seed=None))]
    fn py_new(prob_slippage: f64, random_seed: Option<u64>) -> PyResult<Self> {
        Self::new(prob_slippage, random_seed).map_err(to_pyruntime_err)
    }

    fn __repr__(&self) -> String {
        format!("{self:?}")
    }
}

/// Extracts a Python slippage model object into a Rust [`SlippageModelAny`].
///
/// # Errors
///
/// Returns an error if `obj` is not a supported built-in slippage model binding.
pub fn pyobject_to_slippage_model_any(obj: &Bound<'_, PyAny>) -> PyResult<SlippageModelAny> {
    if let Ok(m) = obj.extract::<ProbabilisticSlippageModel>() {
        return Ok(SlippageModelAny::Probabilistic(m));
    }

    let type_name = obj.get_type().name()?;
    Err(to_pytype_err(format!(
        "Cannot convert {type_name} to SlippageModel"
    )))
}

/// Converts a Rust [`SlippageModelAny`] into its Python binding object.
///
/// # Errors
///
/// Returns an error if conversion to a Python object fails.
pub fn slippage_model_any_to_pyobject(
    py: Python<'_>,
    model: &SlippageModelAny,
) -> PyResult<Py<PyAny>> {
    match model {
        SlippageModelAny::Probabilistic(model) => Ok(Py::new(py, model.clone())?.into_any()),
    }
}

#[cfg(test)]
mod tests {
    use pyo3::ffi::c_str;
    use rstest::rstest;

    use super::*;

    #[rstest]
    fn test_pyobject_to_slippage_model_any_accepts_binding() {
        Python::initialize();

        Python::attach(|py| {
            let obj = Py::new(py, ProbabilisticSlippageModel::new(0.5, Some(3)).unwrap()).unwrap();
            let model = pyobject_to_slippage_model_any(obj.bind(py).as_any()).unwrap();

            assert!(matches!(model, SlippageModelAny::Probabilistic(_)));
        });
    }

    #[rstest]
    fn test_pyobject_to_slippage_model_any_rejects_other_types() {
        Python::initialize();

        Python::attach(|py| {
            let obj = py.eval(c_str!("object()"), None, None).unwrap();
            assert!(pyobject_to_slippage_model_any(&obj).is_err());
        });
    }
}
