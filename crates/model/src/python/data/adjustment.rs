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

//! Python bindings for corporate action adjustment.

use std::str::FromStr;

use nautilus_core::{UnixNanos, python::to_pyvalue_err};
use pyo3::{PyTypeInfo, prelude::*, types::PyType};
use rust_decimal::Decimal;

use crate::{
    data::{AdjustmentSeries, CorporateAction, PriceRepresentation},
    identifiers::InstrumentId,
    python::common::EnumIterator,
    types::Price,
};

#[pyo3_stub_gen::derive::gen_stub_pymethods]
#[pymethods]
impl PriceRepresentation {
    /// The price representation a series is expressed in.
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

    /// Returns the price representation for the given string.
    ///
    /// # Errors
    ///
    /// Returns a `ValueError` if the value is not a price representation.
    #[classmethod]
    #[pyo3(name = "from_str")]
    fn py_from_str(_: &Bound<'_, PyType>, data: &Bound<'_, PyAny>) -> PyResult<Self> {
        let value: &str = data.extract()?;
        Self::from_str(&value.to_uppercase()).map_err(to_pyvalue_err)
    }
}

#[pyo3_stub_gen::derive::gen_stub_pymethods]
#[pymethods]
impl AdjustmentSeries {
    /// A corporate action series for one instrument, in effect order.
    ///
    /// Only splits and dividends participate in the adjustment math; a symbol change and a delisting
    /// are delivered as events but do not scale a price.
    #[new]
    #[pyo3(signature = (instrument_id, actions))]
    fn py_new(instrument_id: InstrumentId, actions: Vec<CorporateAction>) -> PyResult<Self> {
        Self::new(instrument_id, actions).map_err(to_pyvalue_err)
    }

    /// Returns the instrument the series applies to.
    #[getter]
    #[pyo3(name = "instrument_id")]
    #[must_use]
    pub const fn py_instrument_id(&self) -> InstrumentId {
        Self::instrument_id(self)
    }

    /// Returns the actions, in effective order.
    #[getter]
    #[pyo3(name = "actions")]
    #[must_use]
    pub fn py_actions(&self) -> Vec<CorporateAction> {
        self.actions().to_vec()
    }

    /// Returns the cumulative split factor in effect after the given instant.
    ///
    /// The factor is the product of `1 / r` over the splits whose effect is after `ts`, so
    /// multiplying a raw price by it moves the price into the adjusted series.
    #[pyo3(name = "split_factor")]
    #[must_use]
    pub fn py_split_factor(&self, ts_ns: u64) -> Decimal {
        Self::split_factor(self, UnixNanos::from(ts_ns))
    }

    /// Returns the cumulative cash dividend per share in effect after the given instant.
    #[pyo3(name = "dividends")]
    #[must_use]
    pub fn py_dividends(&self, ts_ns: u64) -> Decimal {
        Self::dividends(self, UnixNanos::from(ts_ns))
    }

    /// Converts a raw price at the given instant into the adjusted series.
    ///
    /// # Errors
    ///
    /// Returns an error if the adjusted value is not a valid price.
    #[pyo3(name = "adjust")]
    pub fn py_adjust(&self, price: Price, ts_ns: u64) -> PyResult<Price> {
        Self::adjust(self, price, UnixNanos::from(ts_ns)).map_err(to_pyvalue_err)
    }

    /// Converts an adjusted price at the given instant back into the raw series.
    ///
    /// # Errors
    ///
    /// Returns an error if the split factor is not positive, or the raw value is not a valid price.
    #[pyo3(name = "unadjust")]
    pub fn py_unadjust(&self, price: Price, ts_ns: u64) -> PyResult<Price> {
        Self::unadjust(self, price, UnixNanos::from(ts_ns)).map_err(to_pyvalue_err)
    }

    /// Converts a price from one representation to the other at the given instant.
    ///
    /// A conversion from a representation to itself returns the price unchanged, so a caller can
    /// apply a target representation without branching on the source.
    ///
    /// # Errors
    ///
    /// Returns an error if the conversion is not exact for the price precision.
    #[pyo3(name = "convert")]
    pub fn py_convert(
        &self,
        price: Price,
        ts_ns: u64,
        from: PriceRepresentation,
        to: PriceRepresentation,
    ) -> PyResult<Price> {
        Self::convert(self, price, UnixNanos::from(ts_ns), from, to).map_err(to_pyvalue_err)
    }

    fn __repr__(&self) -> String {
        self.to_string()
    }
}
