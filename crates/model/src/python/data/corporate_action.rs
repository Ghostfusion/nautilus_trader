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

use std::{
    collections::{HashMap, hash_map::DefaultHasher},
    hash::{Hash, Hasher},
    str::FromStr,
};

use nautilus_core::{
    python::{
        IntoPyObjectNautilusExt,
        serialization::{from_dict_pyo3, to_dict_pyo3},
        to_pyvalue_err,
    },
    serialization::{
        Serializable,
        msgpack::{FromMsgPack, ToMsgPack},
    },
};
use pyo3::{
    IntoPyObjectExt, PyTypeInfo,
    basic::CompareOp,
    prelude::*,
    types::{PyDict, PyType},
};
use rust_decimal::Decimal;

use crate::{
    data::corporate_action::{CorporateAction, CorporateActionType},
    identifiers::{InstrumentId, Symbol},
    python::common::{EnumIterator, PY_MODULE_MODEL},
};

////////////////////////////////////////
// Type methods
////////////////////////////////////////
#[pymethods]
#[pyo3_stub_gen::derive::gen_stub_pymethods]
impl CorporateAction {
    /// Represents a corporate action on an instrument.
    ///
    /// `effective_ns` is when the action takes effect at the venue, for example the ex-date of a
    /// dividend. `ts_event` and `ts_init` describe the action record itself.
    #[new]
    #[pyo3(signature = (instrument_id, action, value, new_symbol, effective_ns, ts_event, ts_init))]
    fn py_new(
        instrument_id: InstrumentId,
        action: CorporateActionType,
        value: Decimal,
        new_symbol: Option<Symbol>,
        effective_ns: u64,
        ts_event: u64,
        ts_init: u64,
    ) -> Self {
        Self::new(
            instrument_id,
            action,
            value,
            new_symbol,
            effective_ns.into(),
            ts_event.into(),
            ts_init.into(),
        )
    }

    fn __richcmp__(&self, other: &Self, op: CompareOp, py: Python<'_>) -> Py<PyAny> {
        match op {
            CompareOp::Eq => self.eq(other).into_py_any_unwrap(py),
            CompareOp::Ne => self.ne(other).into_py_any_unwrap(py),
            _ => py.NotImplemented(),
        }
    }

    fn __hash__(&self) -> isize {
        let mut h = DefaultHasher::new();
        self.hash(&mut h);
        h.finish() as isize
    }

    fn __repr__(&self) -> String {
        format!("{}({})", stringify!(CorporateAction), self)
    }

    fn __str__(&self) -> String {
        self.to_string()
    }

    #[getter]
    #[pyo3(name = "instrument_id")]
    fn py_instrument_id(&self) -> InstrumentId {
        self.instrument_id
    }

    #[getter]
    #[pyo3(name = "action")]
    #[must_use]
    pub fn py_action(&self) -> CorporateActionType {
        self.action
    }

    #[getter]
    #[pyo3(name = "value")]
    #[must_use]
    pub fn py_value(&self) -> Decimal {
        self.value
    }

    #[getter]
    #[pyo3(name = "new_symbol")]
    #[must_use]
    pub fn py_new_symbol(&self) -> Option<Symbol> {
        self.new_symbol
    }

    #[getter]
    #[pyo3(name = "effective_ns")]
    #[must_use]
    pub fn py_effective_ns(&self) -> u64 {
        self.effective_ns.as_u64()
    }

    #[getter]
    #[pyo3(name = "ts_event")]
    #[must_use]
    pub fn py_ts_event(&self) -> u64 {
        self.ts_event.as_u64()
    }

    #[getter]
    #[pyo3(name = "ts_init")]
    #[must_use]
    pub fn py_ts_init(&self) -> u64 {
        self.ts_init.as_u64()
    }

    #[staticmethod]
    #[pyo3(name = "fully_qualified_name")]
    fn py_fully_qualified_name() -> String {
        format!("{}:{}", PY_MODULE_MODEL, stringify!(CorporateAction))
    }

    /// Returns the metadata for the type, for use with serialization formats.
    #[staticmethod]
    #[pyo3(name = "get_metadata")]
    fn py_get_metadata(instrument_id: &InstrumentId) -> HashMap<String, String> {
        Self::get_metadata(instrument_id)
    }

    /// Returns a new object from the given dictionary representation.
    #[staticmethod]
    #[pyo3(name = "from_dict")]
    fn py_from_dict(py: Python<'_>, values: Py<PyDict>) -> PyResult<Self> {
        from_dict_pyo3(py, values)
    }

    /// Return a dictionary representation of the object.
    #[pyo3(name = "to_dict")]
    fn py_to_dict(&self, py: Python<'_>) -> PyResult<Py<PyDict>> {
        to_dict_pyo3(py, self)
    }

    /// Return JSON encoded bytes representation of the object.
    #[pyo3(name = "to_json_bytes")]
    fn py_to_json_bytes(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        self.to_json_bytes()
            .map_err(to_pyvalue_err)?
            .into_py_any(py)
    }

    /// Return `MsgPack` encoded bytes representation of the object.
    #[pyo3(name = "to_msgpack_bytes")]
    fn py_to_msgpack_bytes(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        self.to_msgpack_bytes()
            .map_err(to_pyvalue_err)?
            .into_py_any(py)
    }
}

#[pymethods]
impl CorporateAction {
    #[staticmethod]
    #[pyo3(name = "from_json")]
    fn py_from_json(data: &[u8]) -> PyResult<Self> {
        Self::from_json_bytes(data).map_err(to_pyvalue_err)
    }

    #[staticmethod]
    #[pyo3(name = "from_msgpack")]
    fn py_from_msgpack(data: &[u8]) -> PyResult<Self> {
        Self::from_msgpack_bytes(data).map_err(to_pyvalue_err)
    }
}

#[pymethods]
#[pyo3_stub_gen::derive::gen_stub_pymethods]
impl CorporateActionType {
    /// The kind of corporate action.
    #[new]
    fn py_new(py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<Self> {
        let t = Self::type_object(py);
        Self::py_from_str(&t, value)
    }

    const fn __hash__(&self) -> isize {
        *self as isize
    }

    fn __str__(&self) -> String {
        self.to_string()
    }

    #[getter]
    #[must_use]
    pub fn name(&self) -> String {
        self.to_string()
    }

    #[getter]
    #[must_use]
    pub fn value(&self) -> u8 {
        *self as u8
    }

    #[classmethod]
    fn variants(_: &Bound<'_, PyType>, py: Python<'_>) -> EnumIterator {
        EnumIterator::new::<Self>(py)
    }

    #[classmethod]
    #[pyo3(name = "from_str")]
    fn py_from_str(_: &Bound<'_, PyType>, data: &Bound<'_, PyAny>) -> PyResult<Self> {
        let data_str: &str = data.extract()?;
        let tokenized = data_str.to_uppercase();
        Self::from_str(&tokenized).map_err(to_pyvalue_err)
    }
}
