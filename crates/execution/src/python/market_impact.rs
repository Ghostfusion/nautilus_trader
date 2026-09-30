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

//! Python bindings for market impact model types.

use nautilus_core::python::{to_pyruntime_err, to_pytype_err};
use nautilus_model::types::Quantity;
use pyo3::prelude::*;

use crate::models::market_impact::{LinearMarketImpactModel, MarketImpactModelAny};

#[pymethods]
#[pyo3_stub_gen::derive::gen_stub_pymethods]
impl LinearMarketImpactModel {
    /// A linear market impact model.
    ///
    /// The model moves the fill price against the order direction by one price increment for
    /// every `quantity_per_increment` units filled, capped at `max_increments`. A fill smaller
    /// than one increment quantity leaves the price unchanged, and the adjustment grows in whole
    /// increments with the fill size, so a larger order is filled further through the book.
    ///
    /// The model is deterministic and takes no random seed: the adjustment is an exact function
    /// of the fill quantity, computed with decimal arithmetic against the configured reference
    /// quantity.
    #[new]
    #[pyo3(signature = (quantity_per_increment, max_increments))]
    fn py_new(quantity_per_increment: Quantity, max_increments: u64) -> PyResult<Self> {
        Self::new(quantity_per_increment, max_increments).map_err(to_pyruntime_err)
    }

    fn __repr__(&self) -> String {
        format!("{self:?}")
    }
}

/// Extracts a Python market impact model object into a Rust [`MarketImpactModelAny`].
///
/// # Errors
///
/// Returns an error if `obj` is not a supported built-in market impact model binding.
pub fn pyobject_to_market_impact_model_any(
    obj: &Bound<'_, PyAny>,
) -> PyResult<MarketImpactModelAny> {
    if let Ok(m) = obj.extract::<LinearMarketImpactModel>() {
        return Ok(MarketImpactModelAny::Linear(m));
    }

    let type_name = obj.get_type().name()?;
    Err(to_pytype_err(format!(
        "Cannot convert {type_name} to MarketImpactModel"
    )))
}

/// Converts a Rust [`MarketImpactModelAny`] into its Python binding object.
///
/// # Errors
///
/// Returns an error if conversion to a Python object fails.
pub fn market_impact_model_any_to_pyobject(
    py: Python<'_>,
    model: &MarketImpactModelAny,
) -> PyResult<Py<PyAny>> {
    match model {
        MarketImpactModelAny::Linear(model) => Ok(Py::new(py, model.clone())?.into_any()),
    }
}

#[cfg(test)]
mod tests {
    use pyo3::ffi::c_str;
    use rstest::rstest;

    use super::*;

    #[rstest]
    fn test_pyobject_to_market_impact_model_any_accepts_binding() {
        Python::initialize();

        Python::attach(|py| {
            let obj = Py::new(
                py,
                LinearMarketImpactModel::new(Quantity::from("100"), 5).unwrap(),
            )
            .unwrap();
            let model = pyobject_to_market_impact_model_any(obj.bind(py).as_any()).unwrap();

            assert!(matches!(model, MarketImpactModelAny::Linear(_)));
        });
    }

    #[rstest]
    fn test_pyobject_to_market_impact_model_any_rejects_other_types() {
        Python::initialize();

        Python::attach(|py| {
            let obj = py.eval(c_str!("object()"), None, None).unwrap();
            assert!(pyobject_to_market_impact_model_any(&obj).is_err());
        });
    }
}
