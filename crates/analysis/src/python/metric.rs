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

use std::collections::HashMap;

use pyo3::prelude::*;

use crate::metric::{
    MetricDefinition, MetricDirection, MetricInput, MetricReason, MetricReport, MetricResult,
    MetricStatus, MetricTag, MetricUnits,
};

#[pymethods]
#[pyo3_stub_gen::derive::gen_stub_pymethods]
impl MetricDefinition {
    /// Returns the stable machine-facing identity of the metric.
    #[getter]
    #[pyo3(name = "id")]
    fn py_id(&self) -> &str {
        self.id()
    }

    /// Returns the title template, with its parameters unrendered.
    #[getter]
    #[pyo3(name = "title_template")]
    fn py_title_template(&self) -> &str {
        self.title_template()
    }

    /// Returns the declared parameters.
    #[getter]
    #[pyo3(name = "parameters")]
    fn py_parameters(&self) -> HashMap<String, String> {
        self.parameters()
            .iter()
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect()
    }

    /// Returns the units the value is expressed in.
    #[getter]
    #[pyo3(name = "units")]
    const fn py_units(&self) -> MetricUnits {
        self.units()
    }

    /// Returns the tags declared for the metric.
    #[getter]
    #[pyo3(name = "tags")]
    fn py_tags(&self) -> Vec<MetricTag> {
        self.tags().to_vec()
    }

    /// Returns the direction a consumer rewards the metric in.
    #[getter]
    #[pyo3(name = "direction")]
    const fn py_direction(&self) -> MetricDirection {
        self.direction()
    }

    /// Returns the target value of a `MetricDirection.Target` definition, if any.
    #[getter]
    #[pyo3(name = "target")]
    const fn py_target(&self) -> Option<f64> {
        self.target()
    }

    /// Returns the inputs the definition requires.
    #[getter]
    #[pyo3(name = "inputs")]
    fn py_inputs(&self) -> Vec<MetricInput> {
        self.inputs().to_vec()
    }

    /// Returns whether the definition is declared or was derived by a wrapper.
    #[getter]
    #[pyo3(name = "is_derived")]
    const fn py_is_derived(&self) -> bool {
        self.is_derived()
    }

    /// Renders the title from the declared parameters.
    #[getter]
    #[pyo3(name = "title")]
    fn py_title(&self) -> String {
        self.title()
    }

    fn __repr__(&self) -> String {
        format!(
            "MetricDefinition(id={:?}, title={:?}, units={}, direction={})",
            self.id(),
            self.title(),
            self.units(),
            self.direction(),
        )
    }
}

#[pymethods]
#[pyo3_stub_gen::derive::gen_stub_pymethods]
impl MetricResult {
    /// Returns the stable metric identity.
    #[getter]
    #[pyo3(name = "id")]
    fn py_id(&self) -> String {
        self.id().to_string()
    }

    /// Returns the rendered title.
    #[getter]
    #[pyo3(name = "title")]
    fn py_title(&self) -> String {
        self.title().to_string()
    }

    /// Returns the status.
    #[getter]
    #[pyo3(name = "status")]
    const fn py_status(&self) -> MetricStatus {
        self.status()
    }

    /// Returns the value, present only when the status is `Computed`.
    #[getter]
    #[pyo3(name = "value")]
    const fn py_value(&self) -> Option<f64> {
        self.value()
    }

    /// Returns the reason, present whenever the status is not `Computed`.
    #[getter]
    #[pyo3(name = "reason")]
    const fn py_reason(&self) -> Option<MetricReason> {
        self.reason()
    }

    /// Returns the human-readable detail of the reason, when one was supplied.
    ///
    /// The detail names the quantities a refusal refers to; it is never canonical, so a consumer
    /// branches on `Self.reason` and reads the detail only for a human.
    #[getter]
    #[pyo3(name = "detail")]
    fn py_detail(&self) -> Option<String> {
        self.detail().map(ToString::to_string)
    }

    fn __repr__(&self) -> String {
        match (self.value(), self.reason()) {
            (Some(value), _) => format!(
                "MetricResult(id={:?}, status={}, value={})",
                self.id(),
                self.status(),
                value,
            ),
            (None, Some(reason)) => match self.detail() {
                Some(detail) => format!(
                    "MetricResult(id={:?}, status={}, reason={}, detail={:?})",
                    self.id(),
                    self.status(),
                    reason,
                    detail,
                ),
                None => format!(
                    "MetricResult(id={:?}, status={}, reason={})",
                    self.id(),
                    self.status(),
                    reason,
                ),
            },
            (None, None) => format!("MetricResult(id={:?}, status={})", self.id(), self.status()),
        }
    }
}

#[pymethods]
#[pyo3_stub_gen::derive::gen_stub_pymethods]
impl MetricReport {
    /// Returns every result, in request order.
    #[getter]
    #[pyo3(name = "results")]
    fn py_results(&self) -> Vec<MetricResult> {
        self.results().to_vec()
    }

    /// Returns the result for a metric identity, if requested.
    #[pyo3(name = "get")]
    fn py_get(&self, id: &str) -> Option<MetricResult> {
        self.get(id).cloned()
    }

    /// Returns every result with the given status.
    #[pyo3(name = "with_status")]
    fn py_with_status(&self, status: MetricStatus) -> Vec<MetricResult> {
        self.with_status(status).into_iter().cloned().collect()
    }

    fn __len__(&self) -> usize {
        self.len()
    }

    fn __repr__(&self) -> String {
        format!(
            "MetricReport(computed={}, unavailable={}, invalid={}, not_registered={})",
            self.with_status(MetricStatus::Computed).len(),
            self.with_status(MetricStatus::Unavailable).len(),
            self.with_status(MetricStatus::Invalid).len(),
            self.with_status(MetricStatus::NotRegistered).len(),
        )
    }
}
