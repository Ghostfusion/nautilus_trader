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

//! Python bindings for [`TradingSignal`] and [`SignalDirection`].

use std::str::FromStr;

use nautilus_core::{from_pydict, python::to_pyvalue_err};
use pyo3::{
    PyTypeInfo,
    prelude::*,
    types::{PyDict, PyType},
};
use ustr::Ustr;

use crate::{
    identifiers::InstrumentId,
    python::common::EnumIterator,
    signal::{SignalDirection, TradingSignal},
};

#[pyo3_stub_gen::derive::gen_stub_pymethods]
#[pymethods]
impl SignalDirection {
    /// The direction of a trading signal.
    #[new]
    fn py_new(py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<Self> {
        let type_object = Self::type_object(py);
        Self::py_from_str(&type_object, value)
    }

    const fn __hash__(&self) -> isize {
        *self as isize
    }

    fn __str__(&self) -> String {
        self.to_string()
    }

    /// Returns the canonical string representation.
    #[getter]
    #[must_use]
    pub fn name(&self) -> String {
        self.to_string()
    }

    #[classmethod]
    fn variants(_: &Bound<'_, PyType>, py: Python<'_>) -> EnumIterator {
        EnumIterator::new::<Self>(py)
    }

    /// Returns the signal direction for the given string.
    ///
    /// # Errors
    ///
    /// Returns a `ValueError` if the value is not a signal direction.
    #[classmethod]
    #[pyo3(name = "from_str")]
    fn py_from_str(_: &Bound<'_, PyType>, data: &Bound<'_, PyAny>) -> PyResult<Self> {
        let value: &str = data.extract()?;
        Self::from_str(&value.to_uppercase()).map_err(to_pyvalue_err)
    }
}

#[pyo3_stub_gen::derive::gen_stub_pymethods]
#[pymethods]
impl TradingSignal {
    /// A statement of view about one instrument.
    ///
    /// The direction and horizon describe the view, `strength` is its magnitude, and `source` names
    /// the component or model that emitted it. `expiry_ns` bounds how long the view is valid for, and
    /// `provenance` carries free-form origin detail. A signal is not a trading command and carries no
    /// order quantity: it never belongs to the order or position layers.
    #[new]
    #[pyo3(signature = (instrument_id, direction, horizon_ns=None, strength=None, source=None, expiry_ns=None, provenance=None, ts_event=0, ts_init=0))]
    #[expect(
        clippy::too_many_arguments,
        reason = "a signal carries its full statement of view as one value"
    )]
    fn py_new(
        py: Python<'_>,
        instrument_id: InstrumentId,
        direction: SignalDirection,
        horizon_ns: Option<u64>,
        strength: Option<f64>,
        source: Option<String>,
        expiry_ns: Option<u64>,
        provenance: Option<Py<PyDict>>,
        ts_event: u64,
        ts_init: u64,
    ) -> PyResult<Self> {
        let provenance = match provenance {
            Some(dict) => from_pydict(py, &dict)?,
            None => None,
        };

        Self::new(
            instrument_id,
            direction,
            horizon_ns,
            strength,
            source.map(|source| Ustr::from(source.as_str())),
            expiry_ns.map(Into::into),
            provenance,
            ts_event.into(),
            ts_init.into(),
        )
        .map_err(to_pyvalue_err)
    }

    /// Returns the instrument the view is about.
    #[getter]
    #[pyo3(name = "instrument_id")]
    #[must_use]
    pub const fn py_instrument_id(&self) -> InstrumentId {
        Self::instrument_id(self)
    }

    /// Returns the direction of the view.
    #[getter]
    #[pyo3(name = "direction")]
    #[must_use]
    pub const fn py_direction(&self) -> SignalDirection {
        Self::direction(self)
    }

    /// Returns the expected holding horizon (nanoseconds), if any.
    #[getter]
    #[pyo3(name = "horizon_ns")]
    #[must_use]
    pub const fn py_horizon_ns(&self) -> Option<u64> {
        Self::horizon_ns(self)
    }

    /// Returns the magnitude of the view, if any.
    ///
    /// This is a magnitude, not a probability: it is not bounded by one and does not sum to one
    /// across the signals of a portfolio.
    #[getter]
    #[pyo3(name = "strength")]
    #[must_use]
    pub const fn py_strength(&self) -> Option<f64> {
        Self::strength(self)
    }

    /// Returns the emitting component or model, if any.
    #[getter]
    #[pyo3(name = "source")]
    #[must_use]
    pub fn py_source(&self) -> Option<String> {
        Self::source(self).map(|source| source.to_string())
    }

    /// Returns when the view lapses, if it does.
    #[getter]
    #[pyo3(name = "expiry_ns")]
    #[must_use]
    pub fn py_expiry_ns(&self) -> Option<u64> {
        Self::expiry_ns(self).map(|expiry_ns| expiry_ns.as_u64())
    }

    /// Returns the free-form origin detail, if any.
    #[getter]
    #[pyo3(name = "provenance")]
    pub fn py_provenance(&self, py: Python<'_>) -> PyResult<Option<Py<PyDict>>> {
        Self::provenance(self)
            .map(|params| params.to_pydict(py))
            .transpose()
    }

    /// Returns the instant the view was stated.
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

    /// Returns whether the view has lapsed at the given instant.
    ///
    /// A signal with no expiry never lapses.
    #[pyo3(name = "is_expired")]
    #[must_use]
    pub fn py_is_expired(&self, at_ns: u64) -> bool {
        Self::is_expired(self, at_ns.into())
    }

    fn __repr__(&self) -> String {
        self.to_string()
    }

    fn __str__(&self) -> String {
        self.to_string()
    }
}
