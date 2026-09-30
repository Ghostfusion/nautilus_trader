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

//! Python bindings for [`SymbolMap`] and [`SymbolMapEntry`].

use std::str::FromStr;

use jiff::civil::Date;
use nautilus_core::python::to_pyvalue_err;
use pyo3::prelude::*;

use crate::{
    data::CorporateAction,
    identifiers::{InstrumentId, Symbol, SymbolMap, SymbolMapEntry},
};

#[pyo3_stub_gen::derive::gen_stub_pymethods]
#[pymethods]
impl SymbolMap {
    /// Parses and validates a symbol map from JSON text.
    ///
    /// # Errors
    ///
    /// Returns an error if the JSON is malformed, the schema is unsupported, a date or identifier
    /// cannot be parsed, an interval is inverted or overlapping, or a symbol resolves to more than
    /// one instrument identity.
    #[staticmethod]
    #[pyo3(name = "from_json_str")]
    fn py_from_json_str(text: &str) -> PyResult<Self> {
        Self::from_json_str(text).map_err(to_pyvalue_err)
    }

    /// Parses and validates a symbol map from a JSON file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read, or if the contents are not a valid symbol map.
    #[staticmethod]
    #[pyo3(name = "from_json_path")]
    fn py_from_json_path(path: &str) -> PyResult<Self> {
        Self::from_json_path(path).map_err(to_pyvalue_err)
    }

    /// Returns the entries, in file order.
    #[getter]
    #[pyo3(name = "entries")]
    fn py_entries(&self) -> Vec<SymbolMapEntry> {
        self.entries().to_vec()
    }

    /// Returns the instrument identity the symbol names on the given date, if any.
    #[pyo3(name = "resolve")]
    fn py_resolve(&self, symbol: &str, date: &str) -> PyResult<Option<InstrumentId>> {
        let date = parse_date(date)?;
        Ok(self.resolve(&Symbol::from(symbol), date))
    }

    /// Returns the distinct instrument identities, sorted and deduplicated.
    #[pyo3(name = "instrument_ids")]
    fn py_instrument_ids(&self) -> Vec<InstrumentId> {
        self.instrument_ids()
    }

    /// Returns each rename as a symbol change corporate action.
    ///
    /// An action is emitted at the instant the successor symbol takes effect (the successor
    /// entry's `valid_from` at midnight UTC), carries the successor symbol in `new_symbol`, and
    /// names the instrument identity. The result is ordered by effective instant, then by identity
    /// and successor symbol, so it is deterministic.
    #[pyo3(name = "changes")]
    fn py_changes(&self) -> Vec<CorporateAction> {
        self.changes()
    }

    fn __repr__(&self) -> String {
        format!("SymbolMap(entries={})", self.entries().len())
    }
}

#[pyo3_stub_gen::derive::gen_stub_pymethods]
#[pymethods]
impl SymbolMapEntry {
    /// The venue symbol in force over the interval.
    #[getter]
    fn symbol(&self) -> String {
        self.symbol.to_string()
    }

    /// The instrument identity the symbol denoted over the interval.
    #[getter]
    fn instrument_id(&self) -> InstrumentId {
        self.instrument_id
    }

    /// The first date the symbol denoted the identity (inclusive).
    #[getter]
    fn valid_from(&self) -> String {
        self.valid_from.to_string()
    }

    /// The last date the symbol denoted the identity (inclusive), if bounded.
    #[getter]
    fn valid_until(&self) -> Option<String> {
        self.valid_until.map(|date| date.to_string())
    }

    fn __repr__(&self) -> String {
        format!(
            "SymbolMapEntry(symbol='{}', instrument_id='{}', valid_from='{}', valid_until={})",
            self.symbol,
            self.instrument_id,
            self.valid_from,
            self.valid_until
                .map_or_else(|| "None".to_string(), |date| format!("'{date}'")),
        )
    }
}

fn parse_date(value: &str) -> PyResult<Date> {
    Date::from_str(value).map_err(|e| to_pyvalue_err(format!("invalid date '{value}': {e}")))
}
