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

//! Python bindings for [`UniverseChange`] and its enumerations.

use std::str::FromStr;

use nautilus_core::python::to_pyvalue_err;
use pyo3::{PyTypeInfo, prelude::*, types::PyType};

use crate::{
    identifiers::InstrumentId,
    python::common::EnumIterator,
    universe::{UniverseChange, UniverseChangeReason, UniverseMembershipState},
};

#[pyo3_stub_gen::derive::gen_stub_pymethods]
#[pymethods]
impl UniverseMembershipState {
    /// The membership state of an instrument in a universe.
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

    /// Returns the membership state for the given string.
    ///
    /// # Errors
    ///
    /// Returns a `ValueError` if the value is not a membership state.
    #[classmethod]
    #[pyo3(name = "from_str")]
    fn py_from_str(_: &Bound<'_, PyType>, data: &Bound<'_, PyAny>) -> PyResult<Self> {
        let value: &str = data.extract()?;
        Self::from_str(&value.to_uppercase()).map_err(to_pyvalue_err)
    }
}

#[pyo3_stub_gen::derive::gen_stub_pymethods]
#[pymethods]
impl UniverseChangeReason {
    /// Why a membership changed.
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

    /// Returns the change reason for the given string.
    ///
    /// # Errors
    ///
    /// Returns a `ValueError` if the value is not a change reason.
    #[classmethod]
    #[pyo3(name = "from_str")]
    fn py_from_str(_: &Bound<'_, PyType>, data: &Bound<'_, PyAny>) -> PyResult<Self> {
        let value: &str = data.extract()?;
        Self::from_str(&value.to_uppercase()).map_err(to_pyvalue_err)
    }
}

#[pyo3_stub_gen::derive::gen_stub_pymethods]
#[pymethods]
impl UniverseChange {
    /// The universe the change belongs to.
    #[getter]
    #[must_use]
    pub fn universe(&self) -> String {
        self.universe.to_string()
    }

    /// The instrument whose membership changed.
    #[getter]
    #[must_use]
    pub const fn instrument_id(&self) -> InstrumentId {
        self.instrument_id
    }

    /// The state the instrument entered.
    #[getter]
    #[must_use]
    pub const fn state(&self) -> UniverseMembershipState {
        self.state
    }

    /// Why the membership changed.
    #[getter]
    #[must_use]
    pub const fn reason(&self) -> UniverseChangeReason {
        self.reason
    }

    /// The instant the change occurred.
    #[getter]
    #[must_use]
    pub const fn ts_event(&self) -> u64 {
        self.ts_event.as_u64()
    }

    /// The instant the change was initialized.
    #[getter]
    #[must_use]
    pub const fn ts_init(&self) -> u64 {
        self.ts_init.as_u64()
    }

    fn __repr__(&self) -> String {
        format!(
            "UniverseChange(universe='{}', instrument_id={}, state={}, reason={})",
            self.universe, self.instrument_id, self.state, self.reason,
        )
    }
}
