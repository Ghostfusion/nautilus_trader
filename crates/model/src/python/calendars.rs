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

//! Python bindings for [`TradingCalendar`].

use std::str::FromStr;

use jiff::{Timestamp, civil::Date};
use nautilus_core::{UnixNanos, python::to_pyvalue_err};
use pyo3::{PyResult, pymethods};

use crate::{
    calendars::{CalendarKey, TradingCalendar, bundled},
    enums::AssetClass,
    identifiers::{Symbol, Venue},
};

#[pyo3_stub_gen::derive::gen_stub_pymethods]
#[pymethods]
impl TradingCalendar {
    /// Parses and validates a calendar from JSON text.
    ///
    /// # Errors
    ///
    /// Returns an error if the JSON is malformed, the schema is unsupported, a session is invalid
    /// or overlapping, a date or time cannot be parsed, or the coverage range is inverted.
    #[staticmethod]
    #[pyo3(name = "from_json_str")]
    fn py_from_json_str(text: &str) -> PyResult<Self> {
        Self::from_json_str(text).map_err(to_pyvalue_err)
    }

    /// Parses and validates a calendar from a JSON file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read, or if the contents are not a valid calendar.
    #[staticmethod]
    #[pyo3(name = "from_json_path")]
    fn py_from_json_path(path: &str) -> PyResult<Self> {
        Self::from_json_path(path).map_err(to_pyvalue_err)
    }

    /// Returns the bundled calendar for the given key, or `None` when none is bundled.
    ///
    /// # Errors
    ///
    /// Returns a `ValueError` if the asset class is not supported.
    #[staticmethod]
    #[pyo3(name = "bundled", signature = (venue, asset_class, symbol=None))]
    fn py_bundled(venue: &str, asset_class: &str, symbol: Option<&str>) -> PyResult<Option<Self>> {
        let asset_class = AssetClass::from_str(asset_class)
            .map_err(|_| to_pyvalue_err(format!("unsupported asset class: {asset_class}")))?;
        let key = CalendarKey::new(Venue::from(venue), asset_class, symbol.map(Symbol::from));

        Ok(bundled(&key).cloned())
    }

    /// Returns the calendar key.
    #[getter]
    #[pyo3(name = "key")]
    fn py_key(&self) -> String {
        self.key().to_string()
    }

    /// Returns the calendar time zone.
    #[getter]
    #[pyo3(name = "time_zone")]
    fn py_time_zone(&self) -> String {
        self.time_zone_name().to_string()
    }

    /// Returns the first date the calendar data covers.
    #[getter]
    #[pyo3(name = "valid_from")]
    fn py_valid_from(&self) -> String {
        self.valid_from().to_string()
    }

    /// Returns the last date the calendar data covers, if the data is bounded.
    #[getter]
    #[pyo3(name = "valid_until")]
    fn py_valid_until(&self) -> Option<String> {
        self.valid_until().map(|date| date.to_string())
    }

    /// Returns the optional source of the calendar data, for provenance.
    #[getter]
    #[pyo3(name = "source")]
    fn py_source(&self) -> Option<String> {
        self.source().map(str::to_string)
    }

    /// Returns whether the given local date is a declared holiday.
    #[pyo3(name = "is_holiday")]
    fn py_is_holiday(&self, date: &str) -> PyResult<bool> {
        Ok(self.is_holiday(parse_date(date)?))
    }

    /// Returns the declared early close for the given local date, if any.
    #[pyo3(name = "early_close")]
    fn py_early_close(&self, date: &str) -> PyResult<Option<String>> {
        Ok(self
            .early_close(parse_date(date)?)
            .map(|time| time.to_string()))
    }

    /// Returns the sessions for the given local date, honouring holidays and early closes.
    #[pyo3(name = "sessions_on")]
    fn py_sessions_on(&self, date: &str) -> PyResult<Vec<(String, String)>> {
        Ok(self
            .sessions_on(parse_date(date)?)
            .into_iter()
            .map(|session| (session.start.to_string(), session.end.to_string()))
            .collect())
    }

    /// Returns whether the given local date has at least one session.
    #[pyo3(name = "is_trading_day")]
    fn py_is_trading_day(&self, date: &str) -> PyResult<bool> {
        Ok(self.is_trading_day(parse_date(date)?))
    }

    /// Returns whether the given instant falls inside a session.
    #[pyo3(name = "is_tradeable")]
    fn py_is_tradeable(&self, ts_ns: u64) -> bool {
        self.is_tradeable(to_timestamp(ts_ns))
    }

    /// Returns whether the calendar data covers the given instant.
    #[pyo3(name = "covers")]
    fn py_covers(&self, ts_ns: u64) -> bool {
        self.covers(to_timestamp(ts_ns))
    }

    /// Returns the next session open at or after the given instant.
    #[pyo3(name = "next_open")]
    fn py_next_open(&self, ts_ns: u64) -> PyResult<Option<u64>> {
        self.next_open(to_timestamp(ts_ns))
            .map(from_timestamp)
            .transpose()
    }

    /// Returns the previous session open at or before the given instant.
    #[pyo3(name = "prev_open")]
    fn py_prev_open(&self, ts_ns: u64) -> PyResult<Option<u64>> {
        self.prev_open(to_timestamp(ts_ns))
            .map(from_timestamp)
            .transpose()
    }

    /// Returns the next session close at or after the given instant.
    #[pyo3(name = "next_close")]
    fn py_next_close(&self, ts_ns: u64) -> PyResult<Option<u64>> {
        self.next_close(to_timestamp(ts_ns))
            .map(from_timestamp)
            .transpose()
    }

    /// Returns the previous session close at or before the given instant.
    #[pyo3(name = "prev_close")]
    fn py_prev_close(&self, ts_ns: u64) -> PyResult<Option<u64>> {
        self.prev_close(to_timestamp(ts_ns))
            .map(from_timestamp)
            .transpose()
    }

    /// Warns when the calendar coverage ends before the given instant.
    ///
    /// The run is not blocked: the schedule is rule based, so the remaining sessions still resolve,
    /// but the holiday and early close data past the coverage end is absent.
    #[pyo3(name = "warn_if_coverage_ends_before")]
    fn py_warn_if_coverage_ends_before(&self, ts_ns: u64) {
        self.warn_if_coverage_ends_before(to_timestamp(ts_ns));
    }

    fn __repr__(&self) -> String {
        format!(
            "TradingCalendar(key='{}', time_zone='{}', valid_from='{}', valid_until={})",
            self.key(),
            self.time_zone_name(),
            self.valid_from(),
            self.valid_until()
                .map_or_else(|| "None".to_string(), |date| format!("'{date}'")),
        )
    }
}

fn parse_date(value: &str) -> PyResult<Date> {
    Date::from_str(value).map_err(|e| to_pyvalue_err(format!("invalid date '{value}': {e}")))
}

fn to_timestamp(ts_ns: u64) -> Timestamp {
    UnixNanos::from(ts_ns).to_datetime_utc()
}

fn from_timestamp(ts: Timestamp) -> PyResult<u64> {
    u64::try_from(ts.as_nanosecond())
        .map_err(|_| to_pyvalue_err("timestamp is outside the UnixNanos range"))
}
