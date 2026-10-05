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

//! Python bindings for latency model types.

use nautilus_core::{DurationNanos, python::to_pytype_err};
use pyo3::{IntoPyObjectExt, prelude::*};

use crate::models::latency::{
    LatencyModel, LatencyModelAny, LatencyModelHandle, StaticLatencyModel,
};

/// A latency model implemented in Python.
///
/// Subclass this to return the nanosecond latency each order leg takes: `get_insert_latency` for
/// an order reaching the venue, `get_update_latency` for a modification, and `get_delete_latency`
/// for a cancellation. The backtest exchange reads these values when it computes an order's
/// arrival, so the returned latency decides when the order is seen rather than when it was sent.
/// Every method defaults to zero, so a model overrides only the legs it cares about.
#[pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.execution")]
#[pyclass(
    module = "nautilus_trader.execution",
    name = "LatencyModel",
    subclass,
    unsendable
)]
#[derive(Debug)]
pub struct PyLatencyModel;

#[pymethods]
#[pyo3_stub_gen::derive::gen_stub_pymethods]
impl PyLatencyModel {
    #[new]
    fn py_new() -> Self {
        Self
    }

    fn get_insert_latency(&mut self) -> u64 {
        0
    }

    fn get_update_latency(&mut self) -> u64 {
        0
    }

    fn get_delete_latency(&mut self) -> u64 {
        0
    }

    fn get_base_latency(&mut self) -> u64 {
        0
    }
}

#[pymethods]
#[pyo3_stub_gen::derive::gen_stub_pymethods]
impl StaticLatencyModel {
    /// Static latency model with fixed latency values.
    ///
    /// Models the latency for different order operations including base network latency
    /// and specific operation latencies for insert, update, and delete operations.
    ///
    /// The base latency is automatically added to each operation latency, matching
    /// Python's behavior. For example, if `base_latency_nanos = 100ms` and
    /// `insert_latency_nanos = 200ms`, the effective insert latency will be 300ms.
    #[new]
    #[pyo3(signature = (
        base_latency_nanos = 0,
        insert_latency_nanos = 0,
        update_latency_nanos = 0,
        cancel_latency_nanos = 0,
    ))]
    fn py_new(
        base_latency_nanos: u64,
        insert_latency_nanos: u64,
        update_latency_nanos: u64,
        cancel_latency_nanos: u64,
    ) -> Self {
        Self::new(
            DurationNanos::new(base_latency_nanos),
            DurationNanos::new(insert_latency_nanos),
            DurationNanos::new(update_latency_nanos),
            DurationNanos::new(cancel_latency_nanos),
        )
    }

    fn __repr__(&self) -> String {
        format!("{self:?}")
    }
}

/// A latency model implemented in Python.
///
/// The adapter calls the `get_*_latency` methods on the Python object it holds and returns the
/// nanoseconds the object reports as the arrival delay for each order leg, so a caller can supply a
/// model fitted from their own network measurements without rebuilding the extension. The
/// `LatencyModel` trait is infallible and returns a bare [`DurationNanos`], so a raised exception,
/// a missing method and a return that is not a whole number all have to panic rather than be
/// substituted: a default value here would silently mis-time every arrival in the run, which is
/// worse than failing loudly.
#[derive(Debug)]
pub struct PythonLatencyModel {
    obj: Py<PyAny>,
}

impl PythonLatencyModel {
    /// Creates a new [`PythonLatencyModel`] from a Python object.
    #[must_use]
    pub fn new(obj: Py<PyAny>) -> Self {
        Self { obj }
    }
}

impl LatencyModel for PythonLatencyModel {
    fn get_insert_latency(&self) -> DurationNanos {
        call_latency_method(&self.obj, "get_insert_latency")
    }

    fn get_update_latency(&self) -> DurationNanos {
        call_latency_method(&self.obj, "get_update_latency")
    }

    fn get_delete_latency(&self) -> DurationNanos {
        call_latency_method(&self.obj, "get_delete_latency")
    }

    fn get_base_latency(&self) -> DurationNanos {
        call_latency_method(&self.obj, "get_base_latency")
    }
}

/// Calls a latency method on the Python object, panicking if it fails.
///
/// The trait is infallible, so there is nowhere to return the error. A latency that cannot be read
/// is not a latency of zero: substituting one would silently mis-time every arrival in the run, so
/// the failure is carried out as a panic that names the call.
fn call_latency_method(obj: &Py<PyAny>, method_name: &str) -> DurationNanos {
    match Python::attach(|py| -> PyResult<u64> {
        obj.bind(py).call_method0(method_name)?.extract()
    }) {
        Ok(nanos) => DurationNanos::new(nanos),
        Err(e) => panic!("Python LatencyModel.{method_name} failed: {e}"),
    }
}

/// Extracts a Python latency model object into a Rust [`LatencyModelAny`].
///
/// The built-in model bindings themselves are converted here; a Python-defined model is reached
/// through [`pyobject_to_latency_model_handle`].
///
/// # Errors
///
/// Returns an error if `obj` is not a supported latency model binding.
pub fn pyobject_to_latency_model_any(obj: &Bound<'_, PyAny>) -> PyResult<LatencyModelAny> {
    if let Ok(m) = obj.extract::<StaticLatencyModel>() {
        return Ok(LatencyModelAny::Static(m));
    }

    let type_name = obj.get_type().name()?;
    Err(to_pytype_err(format!(
        "Cannot convert {type_name} to LatencyModel"
    )))
}

/// Extracts a Python latency model object into a runtime [`LatencyModelHandle`].
///
/// A built-in model binding is converted as before. Any other object is accepted when it carries
/// all four of `get_insert_latency`, `get_update_latency`, `get_delete_latency` and
/// `get_base_latency`, which is the whole protocol: each method takes no arguments and returns the
/// nanoseconds that order leg takes. A Python model reaches a backtest through this handle path
/// exactly as a fill model does.
///
/// # Errors
///
/// Returns an error if `obj` is neither a supported built-in model nor a Python object with all
/// four latency methods.
pub fn pyobject_to_latency_model_handle(obj: &Bound<'_, PyAny>) -> PyResult<LatencyModelHandle> {
    if let Ok(model) = pyobject_to_latency_model_any(obj) {
        return Ok(model.into());
    }

    let has_required_methods = obj.hasattr("get_insert_latency")?
        && obj.hasattr("get_update_latency")?
        && obj.hasattr("get_delete_latency")?
        && obj.hasattr("get_base_latency")?;
    if !has_required_methods {
        let type_name = obj.get_type().name()?;
        return Err(to_pytype_err(format!(
            "Cannot convert {type_name} to LatencyModel"
        )));
    }

    Ok(LatencyModelHandle::new(PythonLatencyModel::new(
        obj.clone().unbind(),
    )))
}

/// Converts a Rust [`LatencyModelAny`] into its Python binding object.
///
/// # Errors
///
/// Returns an error if conversion to a Python object fails.
pub fn latency_model_any_to_pyobject(
    py: Python<'_>,
    model: &LatencyModelAny,
) -> PyResult<Py<PyAny>> {
    match model {
        LatencyModelAny::Static(model) => model.clone().into_py_any(py),
    }
}

#[cfg(test)]
mod tests {
    use pyo3::ffi::c_str;
    use rstest::rstest;

    use super::*;

    #[rstest]
    fn test_python_latency_model_handle_calls_python_methods() {
        Python::initialize();

        Python::attach(|py| {
            let model = py
                .eval(
                    c_str!(
                        "type('CustomLatencyModel', (), {\
                            'get_insert_latency': lambda self: 11, \
                            'get_update_latency': lambda self: 22, \
                            'get_delete_latency': lambda self: 33, \
                            'get_base_latency': lambda self: 44\
                        })()"
                    ),
                    None,
                    None,
                )
                .unwrap();
            let handle = pyobject_to_latency_model_handle(&model).unwrap();

            assert_eq!(handle.get_insert_latency(), DurationNanos::new(11));
            assert_eq!(handle.get_update_latency(), DurationNanos::new(22));
            assert_eq!(handle.get_delete_latency(), DurationNanos::new(33));
            assert_eq!(handle.get_base_latency(), DurationNanos::new(44));
        });
    }

    #[rstest]
    fn test_python_latency_model_handle_rejects_missing_method() {
        Python::initialize();

        Python::attach(|py| {
            let model = py
                .eval(
                    c_str!(
                        "type('IncompleteLatencyModel', (), {\
                            'get_insert_latency': lambda self: 11, \
                            'get_update_latency': lambda self: 22, \
                            'get_delete_latency': lambda self: 33\
                        })()"
                    ),
                    None,
                    None,
                )
                .unwrap();
            let error = pyobject_to_latency_model_handle(&model)
                .unwrap_err()
                .to_string();

            assert!(error.contains("Cannot convert IncompleteLatencyModel to LatencyModel"));
        });
    }

    #[rstest]
    fn test_python_latency_model_handle_accepts_builtin() {
        Python::initialize();

        Python::attach(|py| {
            let model = StaticLatencyModel::new(
                DurationNanos::new(1),
                DurationNanos::new(2),
                DurationNanos::new(3),
                DurationNanos::new(4),
            );
            let bound = Py::new(py, model).unwrap().into_bound(py);
            let handle = pyobject_to_latency_model_handle(&bound).unwrap();

            assert_eq!(handle.get_insert_latency(), DurationNanos::new(3));
            assert_eq!(handle.get_update_latency(), DurationNanos::new(4));
            assert_eq!(handle.get_delete_latency(), DurationNanos::new(5));
            assert_eq!(handle.get_base_latency(), DurationNanos::new(1));
        });
    }

    #[rstest]
    fn test_python_latency_model_panics_on_python_error() {
        Python::initialize();

        let handle = Python::attach(|py| {
            let model = py
                .eval(
                    c_str!(
                        "type('RaisingLatencyModel', (), {\
                            'get_insert_latency': lambda self: \
                                (_ for _ in ()).throw(RuntimeError('boom')), \
                            'get_update_latency': lambda self: 0, \
                            'get_delete_latency': lambda self: 0, \
                            'get_base_latency': lambda self: 0\
                        })()"
                    ),
                    None,
                    None,
                )
                .unwrap();
            pyobject_to_latency_model_handle(&model).unwrap()
        });

        let payload =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| handle.get_insert_latency()))
                .unwrap_err();

        let message = payload
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| payload.downcast_ref::<&str>().map(|s| (*s).to_string()))
            .expect("panic payload must be a string");

        assert!(message.contains("Python LatencyModel.get_insert_latency failed"));
        assert!(message.contains("RuntimeError"));
        assert!(message.contains("boom"));
    }
}
