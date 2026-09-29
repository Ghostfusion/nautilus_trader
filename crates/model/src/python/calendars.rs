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

use std::{str::FromStr, time::Duration};

use jiff::{Timestamp, civil::Date};
use nautilus_core::{UnixNanos, python::to_pyvalue_err};
use pyo3::{PyTypeInfo, prelude::*, types::PyType};

use crate::{
    calendars::{
        CalendarKey, SessionEvent, SessionEventKind, SessionScheduleConfig, TradingCalendar,
        bundled,
    },
    enums::AssetClass,
    identifiers::{Symbol, Venue},
    python::common::EnumIterator,
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

    /// Expands the calendar into the session events that occur in `[from, to)`.
    ///
    /// The window is half open, so a phase exactly at `to` belongs to the next expansion and
    /// consecutive windows neither duplicate nor drop a phase. Events are ordered by instant, then
    /// by kind.
    ///
    /// A date with no session, including a holiday and a weekend, derives no events. On a date the
    /// calendar declares an early close, the shortened session reports `EarlyClose` instead of
    /// `Close`, and every phase derived from the close moves with it.
    ///
    /// A derived instant that cannot be represented is omitted rather than saturating: a phase is a
    /// derivation of calendar data, so an impossible instant is absent, not approximated.
    #[pyo3(name = "session_events")]
    fn py_session_events(
        &self,
        from_ns: u64,
        to_ns: u64,
        config: &SessionScheduleConfig,
    ) -> Vec<SessionEvent> {
        self.session_events(to_timestamp(from_ns), to_timestamp(to_ns), config)
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

#[pyo3_stub_gen::derive::gen_stub_pymethods]
#[pymethods]
impl SessionEventKind {
    /// A phase of a trading session, or a calendar condition that replaces one.
    ///
    /// The kinds are the session phases an intraday strategy distinguishes. A clock timer is an
    /// interval; a session event is a market-anchored instant.
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

    /// Returns the session event kind for the given string.
    ///
    /// # Errors
    ///
    /// Returns a `ValueError` if the value is not a session event kind.
    #[classmethod]
    #[pyo3(name = "from_str")]
    fn py_from_str(_: &Bound<'_, PyType>, data: &Bound<'_, PyAny>) -> PyResult<Self> {
        let value: &str = data.extract()?;
        Self::from_str(&value.to_uppercase()).map_err(to_pyvalue_err)
    }
}

#[pyo3_stub_gen::derive::gen_stub_pymethods]
#[pymethods]
impl SessionEvent {
    /// The session phase.
    #[getter]
    #[must_use]
    pub fn kind(&self) -> SessionEventKind {
        self.kind
    }

    /// The calendar the event was derived from.
    #[getter]
    #[must_use]
    pub fn key(&self) -> String {
        self.key.to_string()
    }

    /// The exchange-local session date.
    #[getter]
    #[must_use]
    pub fn session_date(&self) -> String {
        self.session_date.to_string()
    }

    /// The index of the session on that date.
    #[getter]
    #[must_use]
    pub fn session_index(&self) -> usize {
        self.session_index
    }

    /// The instant the phase occurs (UTC).
    #[getter]
    #[must_use]
    pub fn ts_event(&self) -> u64 {
        u64::try_from(self.ts_event.as_nanosecond()).unwrap_or_default()
    }

    /// The instant the event was initialized (UTC).
    #[getter]
    #[must_use]
    pub fn ts_init(&self) -> u64 {
        u64::try_from(self.ts_init.as_nanosecond()).unwrap_or_default()
    }

    /// Returns the deterministic timer name for this event.
    ///
    /// The name identifies the event and nothing else, so scheduling the same event twice replaces
    /// the existing timer instead of duplicating it.
    #[pyo3(name = "name")]
    #[must_use]
    fn py_name(&self) -> String {
        Self::name(self)
    }

    fn __repr__(&self) -> String {
        format!(
            "SessionEvent(kind={}, key='{}', session_date='{}', session_index={}, ts_event={})",
            self.kind, self.key, self.session_date, self.session_index, self.ts_event,
        )
    }
}

#[pyo3_stub_gen::derive::gen_stub_pymethods]
#[pymethods]
impl SessionScheduleConfig {
    /// Configures which session phases are derived, and how far from a session boundary they fall.
    ///
    /// Offsets are absolute elapsed time, not civil clock time: a pre-close offset of 30 minutes is 30
    /// minutes of real time before the close, which is what an intraday strategy needs when reacting to
    /// the close. Nothing here reads a clock.
    #[new]
    #[pyo3(signature = (premarket_offset_ns, opening_range_ns, pre_close_offset_ns, kinds=None))]
    fn py_new(
        premarket_offset_ns: u64,
        opening_range_ns: u64,
        pre_close_offset_ns: u64,
        kinds: Option<Vec<SessionEventKind>>,
    ) -> PyResult<Self> {
        let config = Self::new(
            Duration::from_nanos(premarket_offset_ns),
            Duration::from_nanos(opening_range_ns),
            Duration::from_nanos(pre_close_offset_ns),
        )
        .map_err(to_pyvalue_err)?;

        Ok(match kinds {
            Some(kinds) => config.with_kinds(kinds),
            None => config,
        })
    }

    /// The interval before the session open reported as `Premarket`.
    #[getter]
    #[must_use]
    pub fn premarket_offset_ns(&self) -> u64 {
        self.premarket_offset.as_nanos() as u64
    }

    /// The interval after the session open reported as `OpeningRangeComplete`.
    #[getter]
    #[must_use]
    pub fn opening_range_ns(&self) -> u64 {
        self.opening_range.as_nanos() as u64
    }

    /// The interval before the session close reported as `PreClose`.
    #[getter]
    #[must_use]
    pub fn pre_close_offset_ns(&self) -> u64 {
        self.pre_close_offset.as_nanos() as u64
    }

    /// The phases to derive.
    #[getter]
    #[must_use]
    pub fn kinds(&self) -> Vec<SessionEventKind> {
        self.kinds.clone()
    }

    /// Returns a copy of this configuration deriving only the given phases.
    #[pyo3(name = "with_kinds")]
    #[must_use]
    pub fn py_with_kinds(&self, kinds: Vec<SessionEventKind>) -> Self {
        Self::with_kinds(self.clone(), kinds)
    }

    /// Returns whether the given phase is derived.
    #[pyo3(name = "includes")]
    #[must_use]
    pub fn py_includes(&self, kind: SessionEventKind) -> bool {
        Self::includes(self, kind)
    }

    fn __repr__(&self) -> String {
        format!(
            "SessionScheduleConfig(premarket_offset_ns={}, opening_range_ns={}, pre_close_offset_ns={}, kinds={:?})",
            self.premarket_offset.as_nanos(),
            self.opening_range.as_nanos(),
            self.pre_close_offset.as_nanos(),
            self.kinds,
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
