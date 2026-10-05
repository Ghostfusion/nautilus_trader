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

use std::collections::BTreeMap;

use nautilus_core::{UnixNanos, python::to_pyvalue_err};
use nautilus_model::identifiers::InstrumentId;
use pyo3::prelude::*;

use crate::{
    membership::MembershipSeries,
    panel::{FeatureValue, Panel, PanelRow},
};

#[pymethods]
#[pyo3_stub_gen::derive::gen_stub_pymethods]
impl FeatureValue {
    /// Creates a new feature value with the instant of the latest input it reads.
    #[new]
    #[pyo3(signature = (value, as_of))]
    fn py_new(value: f64, as_of: u64) -> Self {
        Self::new(value, UnixNanos::from(as_of))
    }

    #[getter]
    fn value(&self) -> f64 {
        self.value
    }

    #[getter]
    fn as_of(&self) -> u64 {
        self.as_of.as_u64()
    }

    fn __repr__(&self) -> String {
        format!("FeatureValue(value={}, as_of={})", self.value, self.as_of)
    }
}

#[pymethods]
#[pyo3_stub_gen::derive::gen_stub_pymethods]
impl PanelRow {
    /// Creates an empty panel row for an instrument at a timestamp.
    #[new]
    #[pyo3(signature = (instrument_id, ts_event))]
    fn py_new(instrument_id: InstrumentId, ts_event: u64) -> Self {
        Self::new(instrument_id, UnixNanos::from(ts_event))
    }

    /// Returns a copy of the row with a feature column set.
    #[pyo3(name = "with_feature")]
    #[pyo3(signature = (name, value, as_of))]
    fn py_with_feature(&self, name: String, value: f64, as_of: u64) -> Self {
        self.clone()
            .with_feature(name, value, UnixNanos::from(as_of))
    }

    /// Returns a copy of the row with the label set.
    #[pyo3(name = "with_label")]
    fn py_with_label(&self, label: f64) -> Self {
        self.clone().with_label(label)
    }

    /// Returns a copy of the row with the membership set.
    #[pyo3(name = "with_member")]
    fn py_with_member(&self, member: bool) -> Self {
        self.clone().with_member(member)
    }

    #[getter]
    fn instrument_id(&self) -> InstrumentId {
        self.instrument_id
    }

    #[getter]
    fn ts_event(&self) -> u64 {
        self.ts_event.as_u64()
    }

    /// Returns the feature columns keyed by canonical feature name.
    #[getter]
    fn features(&self) -> BTreeMap<String, FeatureValue> {
        self.features.clone()
    }

    #[getter]
    fn label(&self) -> Option<f64> {
        self.label
    }

    #[getter]
    fn member(&self) -> bool {
        self.member
    }

    fn __repr__(&self) -> String {
        format!(
            "PanelRow(instrument_id={}, ts_event={}, features={}, label={:?}, member={})",
            self.instrument_id,
            self.ts_event,
            self.features.len(),
            self.label,
            self.member,
        )
    }
}

#[pymethods]
#[pyo3_stub_gen::derive::gen_stub_pymethods]
impl Panel {
    /// Creates a panel from rows, resolving every row's membership from `membership`.
    #[new]
    #[pyo3(signature = (membership, rows))]
    fn py_new(membership: &MembershipSeries, rows: Vec<PanelRow>) -> PyResult<Self> {
        Self::new(membership, rows).map_err(to_pyvalue_err)
    }

    /// Checks both structural rules: point-in-time membership and no lookahead.
    #[pyo3(name = "check")]
    fn py_check(&self) -> PyResult<()> {
        self.check().map_err(to_pyvalue_err)
    }

    /// Returns the panel rows.
    #[pyo3(name = "rows")]
    fn py_rows(&self) -> Vec<PanelRow> {
        self.rows().to_vec()
    }

    /// Returns the universe identity of the panel's membership.
    #[pyo3(name = "universe")]
    fn py_universe(&self) -> String {
        self.universe().to_string()
    }

    /// Returns the number of rows.
    #[pyo3(name = "len")]
    fn py_len(&self) -> usize {
        self.len()
    }

    /// Returns whether the panel has no rows.
    #[pyo3(name = "is_empty")]
    fn py_is_empty(&self) -> bool {
        self.is_empty()
    }

    fn __repr__(&self) -> String {
        format!("Panel(universe={}, rows={})", self.universe(), self.len())
    }
}
