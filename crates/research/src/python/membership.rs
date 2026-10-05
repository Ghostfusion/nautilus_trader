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

use nautilus_core::UnixNanos;
use nautilus_model::identifiers::InstrumentId;
use pyo3::prelude::*;

use crate::membership::{MembershipInterval, MembershipSeries};

#[pymethods]
#[pyo3_stub_gen::derive::gen_stub_pymethods]
impl MembershipInterval {
    /// Creates a membership interval from its entry instant.
    ///
    /// The entry instant is mirrored on both `ts_event` and `ts_init`.
    #[new]
    #[pyo3(signature = (universe, source, instrument_id, entered_at, exited_at=None))]
    fn py_new(
        universe: String,
        source: String,
        instrument_id: InstrumentId,
        entered_at: u64,
        exited_at: Option<u64>,
    ) -> Self {
        Self::entry(
            universe,
            source,
            instrument_id,
            UnixNanos::from(entered_at),
            exited_at.map(UnixNanos::from),
        )
    }

    #[getter]
    fn universe(&self) -> String {
        self.universe.clone()
    }

    #[getter]
    fn source(&self) -> String {
        self.source.clone()
    }

    #[getter]
    fn instrument_id(&self) -> InstrumentId {
        self.instrument_id
    }

    #[getter]
    fn ts_event(&self) -> u64 {
        self.ts_event.as_u64()
    }

    #[getter]
    fn ts_init(&self) -> u64 {
        self.ts_init.as_u64()
    }

    #[getter]
    fn exited_at(&self) -> Option<u64> {
        self.exited_at.map(|exit| exit.as_u64())
    }

    /// Returns whether the instrument is a member at `ts`.
    #[pyo3(name = "covers")]
    fn py_covers(&self, ts: u64) -> bool {
        self.covers(UnixNanos::from(ts))
    }

    fn __repr__(&self) -> String {
        format!(
            "MembershipInterval(universe={}, source={}, instrument_id={}, ts_event={}, \
             ts_init={}, exited_at={:?})",
            self.universe,
            self.source,
            self.instrument_id,
            self.ts_event,
            self.ts_init,
            self.exited_at,
        )
    }
}

#[pymethods]
#[pyo3_stub_gen::derive::gen_stub_pymethods]
impl MembershipSeries {
    /// Creates an empty membership series for the given universe and source identities.
    #[new]
    #[pyo3(signature = (universe, source))]
    fn py_new(universe: String, source: String) -> Self {
        Self::new(universe, source)
    }

    /// Returns the universe identity of the series.
    #[pyo3(name = "universe")]
    fn py_universe(&self) -> String {
        self.universe().to_string()
    }

    /// Returns the source identity of the series.
    #[pyo3(name = "source")]
    fn py_source(&self) -> String {
        self.source().to_string()
    }

    /// Returns the stored membership intervals.
    #[pyo3(name = "intervals")]
    fn py_intervals(&self) -> Vec<MembershipInterval> {
        self.intervals().to_vec()
    }

    /// Appends a membership interval.
    #[pyo3(name = "push")]
    fn py_push(&mut self, interval: MembershipInterval) {
        self.push(interval);
    }

    /// Resolves the members that apply at `ts`.
    #[pyo3(name = "members_at")]
    fn py_members_at(&self, ts: u64) -> Vec<InstrumentId> {
        self.members_at(UnixNanos::from(ts))
    }

    /// Returns whether `instrument_id` is a member at `ts`.
    #[pyo3(name = "is_member_at")]
    fn py_is_member_at(&self, instrument_id: InstrumentId, ts: u64) -> bool {
        self.is_member_at(instrument_id, UnixNanos::from(ts))
    }

    /// Returns every instrument that appears in the series.
    #[pyo3(name = "instruments")]
    fn py_instruments(&self) -> Vec<InstrumentId> {
        self.instruments()
    }

    fn __repr__(&self) -> String {
        format!(
            "MembershipSeries(universe={}, source={}, intervals={})",
            self.universe(),
            self.source(),
            self.intervals().len(),
        )
    }
}
