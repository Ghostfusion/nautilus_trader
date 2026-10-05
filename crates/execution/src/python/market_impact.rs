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

use nautilus_core::python::{to_pyruntime_err, to_pytype_err, to_pyvalue_err};
use nautilus_model::types::Quantity;
use pyo3::prelude::*;

use crate::models::market_impact::{
    ImpactCalibrationSource, LinearMarketImpactModel, MarketImpactModel, MarketImpactModelAny,
    MarketImpactModelHandle, PrefactorInterval, SquareRootMarketImpactModel,
};

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

#[pymethods]
#[pyo3_stub_gen::derive::gen_stub_pymethods]
impl PrefactorInterval {
    /// A calibrated prefactor with the interval it was measured over and its provenance.
    ///
    /// An interval whose bounds are equal is a point calibration, which is honest only when the
    /// prefactor really was measured; `source` records which of the two a caller is looking at. A
    /// model applies the upper bound, so an uncertain prefactor cannot flatter a simulated result.
    #[new]
    #[pyo3(signature = (lower, upper, source))]
    fn py_new(lower: f64, upper: f64, source: ImpactCalibrationSource) -> PyResult<Self> {
        Self::new(lower, upper, source).map_err(to_pyvalue_err)
    }

    /// Creates a point calibration from a single prefactor.
    #[staticmethod]
    #[pyo3(name = "point")]
    fn py_point(prefactor: f64, source: ImpactCalibrationSource) -> PyResult<Self> {
        Self::point(prefactor, source).map_err(to_pyvalue_err)
    }

    /// The lower bound of the calibrated prefactor.
    #[getter]
    #[pyo3(name = "lower")]
    fn py_lower(&self) -> f64 {
        self.lower()
    }

    /// The upper bound of the calibrated prefactor.
    #[getter]
    #[pyo3(name = "upper")]
    fn py_upper(&self) -> f64 {
        self.upper()
    }

    /// The bound a model applies, which is the upper one.
    #[getter]
    #[pyo3(name = "applied")]
    fn py_applied(&self) -> f64 {
        self.applied()
    }

    /// Where the interval came from.
    #[getter]
    #[pyo3(name = "source")]
    fn py_source(&self) -> ImpactCalibrationSource {
        self.source()
    }

    /// Whether the interval is a point.
    #[getter]
    #[pyo3(name = "is_point")]
    fn py_is_point(&self) -> bool {
        self.is_point()
    }

    /// Returns this calibration with the anonymous-tape de-bias applied.
    ///
    /// Both bounds are divided by the inflation the reconstruction introduced, and the result
    /// records that it was de-biased rather than measured. A prefactor fitted from the venue's own
    /// fills is refused, because a measured prefactor needs no correction.
    #[pyo3(name = "debiased")]
    fn py_debiased(&self) -> PyResult<Self> {
        self.debiased().map_err(to_pyvalue_err)
    }

    fn __repr__(&self) -> String {
        self.to_string()
    }
}

#[pymethods]
#[pyo3_stub_gen::derive::gen_stub_pymethods]
impl SquareRootMarketImpactModel {
    /// A concave market impact model with a square-root shape.
    ///
    /// The model moves the fill price against the order direction by the interval's upper bound
    /// times the square root of the fill's size relative to `reference_quantity`, capped at
    /// `max_increments`: a fill equal to the reference quantity moves by the applied prefactor's
    /// worth of increments, and doubling the size less than doubles the adjustment. The exponent is
    /// fixed at one half because the corpus finds it robust while the prefactor is not, which is
    /// why the prefactor arrives as an interval with its calibration source rather than as a number.
    ///
    /// The model is a pure function of the fill quantity and takes no random seed.
    #[new]
    #[pyo3(signature = (prefactor, reference_quantity, max_increments))]
    fn py_new(
        prefactor: PrefactorInterval,
        reference_quantity: Quantity,
        max_increments: u64,
    ) -> PyResult<Self> {
        Self::new(prefactor, reference_quantity, max_increments).map_err(to_pyruntime_err)
    }

    /// The calibrated prefactor, with its bounds and its source.
    #[getter]
    #[pyo3(name = "prefactor")]
    fn py_prefactor(&self) -> PrefactorInterval {
        self.prefactor()
    }

    /// The fill quantity that moves the fill price by the prefactor's worth of increments.
    #[getter]
    #[pyo3(name = "reference_quantity")]
    fn py_reference_quantity(&self) -> Quantity {
        self.reference_quantity()
    }

    /// The maximum number of price increments a fill price may move.
    #[getter]
    #[pyo3(name = "max_increments")]
    fn py_max_increments(&self) -> u64 {
        self.max_increments()
    }

    fn __repr__(&self) -> String {
        self.to_string()
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
    if let Ok(m) = obj.extract::<SquareRootMarketImpactModel>() {
        return Ok(MarketImpactModelAny::SquareRoot(m));
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
        MarketImpactModelAny::SquareRoot(model) => Ok(Py::new(py, model.clone())?.into_any()),
    }
}

/// A market impact model implemented in Python.
///
/// The adapter calls `impact_increments` on the Python object it holds with the fill's quantity and
/// returns the whole number of price increments the object reports, so a caller can supply a model
/// fitted from their own data without rebuilding the extension. The object is the caller's, so a
/// raised exception, a missing method and a return that is not a whole number all surface as the
/// error that reaches the caller rather than as a silently unchanged fill price.
#[derive(Debug)]
pub struct PythonMarketImpactModel {
    obj: Py<PyAny>,
}

impl PythonMarketImpactModel {
    /// Creates a new [`PythonMarketImpactModel`] from a Python object.
    #[must_use]
    pub fn new(obj: Py<PyAny>) -> Self {
        Self { obj }
    }
}

impl MarketImpactModel for PythonMarketImpactModel {
    fn impact_increments(&mut self, fill_quantity: Quantity) -> anyhow::Result<u64> {
        Python::attach(|py| -> anyhow::Result<u64> {
            let quantity = Py::new(py, fill_quantity)?;
            self.obj
                .bind(py)
                .call_method1("impact_increments", (quantity,))?
                .extract()
                .map_err(|e| anyhow::anyhow!("{e}"))
        })
        .map_err(|e| anyhow::anyhow!("Python MarketImpactModel.impact_increments failed: {e}"))
    }
}

/// Extracts a Python market impact model object into a runtime [`MarketImpactModelHandle`].
///
/// A built-in model binding is converted as before. Any other object is accepted when it carries
/// an `impact_increments` method, which is the whole protocol: the method takes the fill quantity
/// and returns the number of price increments the fill price moves against the order direction.
///
/// # Errors
///
/// Returns an error if `obj` is neither a supported built-in model nor a Python object with an
/// `impact_increments` method.
pub fn pyobject_to_market_impact_model_handle(
    obj: &Bound<'_, PyAny>,
) -> PyResult<MarketImpactModelHandle> {
    if let Ok(model) = pyobject_to_market_impact_model_any(obj) {
        return Ok(model.into());
    }

    if !obj.hasattr("impact_increments")? {
        let type_name = obj.get_type().name()?;
        return Err(to_pytype_err(format!(
            "Cannot convert {type_name} to MarketImpactModel"
        )));
    }

    Ok(MarketImpactModelHandle::new(PythonMarketImpactModel::new(
        obj.clone().unbind(),
    )))
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

    #[rstest]
    fn test_pyobject_to_market_impact_model_handle_accepts_a_builtin() {
        Python::initialize();

        Python::attach(|py| {
            let obj = Py::new(
                py,
                SquareRootMarketImpactModel::new(
                    PrefactorInterval::point(4.0, ImpactCalibrationSource::Fills).unwrap(),
                    Quantity::from("25"),
                    25,
                )
                .unwrap(),
            )
            .unwrap();

            let mut handle = pyobject_to_market_impact_model_handle(obj.bind(py).as_any()).unwrap();
            assert_eq!(handle.impact_increments(Quantity::from("25")).unwrap(), 4);
        });
    }

    #[rstest]
    fn test_pyobject_to_market_impact_model_handle_accepts_a_duck_typed_model() {
        Python::initialize();

        Python::attach(|py| {
            let obj = py
                .eval(
                    c_str!("type('Model', (), {'impact_increments': lambda self, quantity: 3})()"),
                    None,
                    None,
                )
                .unwrap();

            let mut handle = pyobject_to_market_impact_model_handle(&obj).unwrap();
            assert_eq!(handle.impact_increments(Quantity::from("100")).unwrap(), 3);

            // The protocol is the method alone: the same object answers for any fill size.
            assert_eq!(handle.impact_increments(Quantity::from("1")).unwrap(), 3);
        });
    }

    #[rstest]
    fn test_pyobject_to_market_impact_model_handle_rejects_an_object_without_the_method() {
        Python::initialize();

        Python::attach(|py| {
            let obj = py.eval(c_str!("object()"), None, None).unwrap();
            assert!(pyobject_to_market_impact_model_handle(&obj).is_err());
        });
    }

    #[rstest]
    fn test_a_python_model_that_raises_surfaces_the_error() {
        Python::initialize();

        Python::attach(|py| {
            let obj = py
                .eval(
                    c_str!(
                        "type('Model', (), {'impact_increments': lambda self, quantity: (_ for _ in ()).throw(ValueError('no'))})()"
                    ),
                    None,
                    None,
                )
                .unwrap();

            let mut handle = pyobject_to_market_impact_model_handle(&obj).unwrap();
            let error = handle.impact_increments(Quantity::from("100")).unwrap_err();

            assert!(
                error.to_string().contains("impact_increments failed"),
                "the error must name the call, was {error}",
            );
            assert!(
                error.to_string().contains("ValueError"),
                "the error must carry the Python exception, was {error}",
            );
        });
    }

    #[rstest]
    fn test_a_python_model_that_returns_a_non_integer_surfaces_the_error() {
        Python::initialize();

        Python::attach(|py| {
            let obj = py
                .eval(
                    c_str!(
                        "type('Model', (), {'impact_increments': lambda self, quantity: 'three'})()"
                    ),
                    None,
                    None,
                )
                .unwrap();

            let mut handle = pyobject_to_market_impact_model_handle(&obj).unwrap();
            assert!(handle.impact_increments(Quantity::from("100")).is_err());
        });
    }
}
