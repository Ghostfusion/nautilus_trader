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

//! Python bindings for [`Capability`].

use pyo3::prelude::*;

use crate::{
    capability::{Capability, is_canonical_code},
    python::to_pyvalue_err,
};

#[pymethods]
#[pyo3_stub_gen::derive::gen_stub_pymethods]
impl Capability {
    /// Returns an available answer, which carries no code and no requirements.
    #[staticmethod]
    #[pyo3(name = "available")]
    fn py_available() -> Self {
        Self::available()
    }

    /// Returns an unavailable answer carrying the given code and detail.
    ///
    /// The code must come from a closed set the domain declares, because a code that is prose
    /// defeats the checkable half of the answer. Canonical form is asserted in debug builds and
    /// every domain asserts the form of its whole set in its own tests, so the closed sets are
    /// what enforce it and this only reports a set that was declared badly.
    #[staticmethod]
    #[pyo3(name = "unavailable")]
    fn py_unavailable(code: &str, detail: &str) -> PyResult<Self> {
        if !is_canonical_code(code) {
            return Err(to_pyvalue_err(format!(
                "capability code '{code}' is not canonical"
            )));
        }

        Ok(Self::unavailable(code, detail))
    }

    /// Returns this answer with a requirement added, in the order the requirements are recorded.
    #[pyo3(name = "requiring")]
    fn py_requiring(&self, requirement: String) -> Self {
        self.clone().requiring(requirement)
    }

    /// Returns whether the request can be served.
    #[getter("is_available")]
    fn py_is_available(&self) -> bool {
        self.is_available()
    }

    /// Returns the canonical code, or `None` when the request can be served.
    #[getter("code")]
    fn py_code(&self) -> Option<String> {
        self.code().map(str::to_owned)
    }

    /// Returns the human-readable detail, which is empty when the request can be served.
    #[getter("detail")]
    fn py_detail(&self) -> String {
        self.detail().to_owned()
    }

    /// Returns the requirements that were not met.
    #[getter("requirements")]
    fn py_requirements(&self) -> Vec<String> {
        self.requirements().to_vec()
    }

    fn __repr__(&self) -> String {
        format!("Capability({self})")
    }

    fn __str__(&self) -> String {
        self.to_string()
    }
}
