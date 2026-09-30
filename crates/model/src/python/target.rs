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

//! Python bindings for [`Target`] and its value kind.

use nautilus_core::python::to_pyvalue_err;
use pyo3::{prelude::*, types::PyType};

use crate::{
    identifiers::InstrumentId,
    target::{Target, TargetValue},
    types::{Money, Quantity},
};

/// A desired exposure, as held by a [`Target`].
///
/// This is a read-only view of the target's value: it names the kind and exposes the value of that
/// kind. A target is created from a quantity, a weight, or a notional through [`Target`].
#[derive(Clone, Debug, PartialEq)]
#[pyo3::pyclass(
    name = "TargetValue",
    module = "nautilus_trader.model",
    frozen,
    skip_from_py_object
)]
#[pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.model")]
pub struct PyTargetValue {
    inner: TargetValue,
}

impl PyTargetValue {
    /// Creates a new [`PyTargetValue`] from the given value.
    #[must_use]
    pub const fn new(inner: TargetValue) -> Self {
        Self { inner }
    }

    /// Returns the wrapped value.
    #[must_use]
    pub const fn inner(&self) -> &TargetValue {
        &self.inner
    }
}

impl From<TargetValue> for PyTargetValue {
    fn from(value: TargetValue) -> Self {
        Self::new(value)
    }
}

#[pyo3_stub_gen::derive::gen_stub_pymethods]
#[pymethods]
impl PyTargetValue {
    /// Returns the canonical name of the value kind.
    #[getter]
    #[pyo3(name = "kind")]
    #[must_use]
    pub fn py_kind(&self) -> &'static str {
        self.inner.kind()
    }

    /// Returns the target quantity, if this is a quantity target.
    #[getter]
    #[pyo3(name = "quantity")]
    #[must_use]
    pub const fn py_quantity(&self) -> Option<Quantity> {
        self.inner.quantity()
    }

    /// Returns the target weight, if this is a weight target.
    #[getter]
    #[pyo3(name = "weight")]
    #[must_use]
    pub const fn py_weight(&self) -> Option<f64> {
        self.inner.weight()
    }

    /// Returns the target notional value, if this is a notional target.
    #[getter]
    #[pyo3(name = "notional")]
    #[must_use]
    pub const fn py_notional(&self) -> Option<Money> {
        self.inner.notional()
    }

    fn __repr__(&self) -> String {
        self.inner.to_string()
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }
}

#[pyo3_stub_gen::derive::gen_stub_pymethods]
#[pymethods]
impl Target {
    /// Creates a new `Target` holding an absolute quantity.
    ///
    /// # Errors
    ///
    /// Returns an error if the value is rejected, which cannot happen for a quantity.
    #[classmethod]
    #[pyo3(name = "from_quantity")]
    #[pyo3(signature = (instrument_id, quantity, ts_event=0, ts_init=0))]
    pub fn py_from_quantity(
        _cls: &Bound<'_, PyType>,
        instrument_id: InstrumentId,
        quantity: Quantity,
        ts_event: u64,
        ts_init: u64,
    ) -> PyResult<Self> {
        Self::from_quantity(instrument_id, quantity, ts_event.into(), ts_init.into())
            .map_err(to_pyvalue_err)
    }

    /// Creates a new `Target` holding a fraction of portfolio equity.
    ///
    /// # Errors
    ///
    /// Returns an error if the weight is not finite.
    #[classmethod]
    #[pyo3(name = "from_weight")]
    #[pyo3(signature = (instrument_id, weight, ts_event=0, ts_init=0))]
    pub fn py_from_weight(
        _cls: &Bound<'_, PyType>,
        instrument_id: InstrumentId,
        weight: f64,
        ts_event: u64,
        ts_init: u64,
    ) -> PyResult<Self> {
        Self::from_weight(instrument_id, weight, ts_event.into(), ts_init.into())
            .map_err(to_pyvalue_err)
    }

    /// Creates a new `Target` holding a notional value.
    ///
    /// # Errors
    ///
    /// Returns an error if the value is rejected, which cannot happen for a money amount.
    #[classmethod]
    #[pyo3(name = "from_notional")]
    #[pyo3(signature = (instrument_id, notional, ts_event=0, ts_init=0))]
    pub fn py_from_notional(
        _cls: &Bound<'_, PyType>,
        instrument_id: InstrumentId,
        notional: Money,
        ts_event: u64,
        ts_init: u64,
    ) -> PyResult<Self> {
        Self::from_notional(instrument_id, notional, ts_event.into(), ts_init.into())
            .map_err(to_pyvalue_err)
    }

    /// Returns the instrument the target applies to.
    #[getter]
    #[pyo3(name = "instrument_id")]
    #[must_use]
    pub const fn py_instrument_id(&self) -> InstrumentId {
        Self::instrument_id(self)
    }

    /// Returns the desired exposure.
    #[getter]
    #[pyo3(name = "value")]
    #[must_use]
    pub fn py_value(&self) -> PyTargetValue {
        PyTargetValue::new(Self::value(self).clone())
    }

    /// Returns the canonical name of the value kind.
    #[getter]
    #[pyo3(name = "kind")]
    #[must_use]
    pub const fn py_kind(&self) -> &'static str {
        Self::kind(self)
    }

    /// Returns the instant the target was stated.
    #[getter]
    #[pyo3(name = "ts_event")]
    #[must_use]
    pub const fn py_ts_event(&self) -> u64 {
        Self::ts_event(self).as_u64()
    }

    /// Returns the instant the instance was created.
    #[getter]
    #[pyo3(name = "ts_init")]
    #[must_use]
    pub const fn py_ts_init(&self) -> u64 {
        Self::ts_init(self).as_u64()
    }

    /// Returns whether the target is a zero exposure.
    #[pyo3(name = "is_flat")]
    #[must_use]
    pub fn py_is_flat(&self) -> bool {
        Self::is_flat(self)
    }

    fn __repr__(&self) -> String {
        self.to_string()
    }

    fn __str__(&self) -> String {
        self.to_string()
    }
}
