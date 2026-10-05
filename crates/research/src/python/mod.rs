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

//! Python bindings from [PyO3](https://pyo3.rs).

pub mod membership;
pub mod panel;

use pyo3::{prelude::*, pymodule};

/// Initializes the Python `research` module.
///
/// Adds the point-in-time panel types and the stored universe membership types.
///
/// # Errors
///
/// Returns a Python exception if adding any class fails.
#[pymodule]
pub fn research(_: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<crate::panel::FeatureValue>()?;
    m.add_class::<crate::panel::PanelRow>()?;
    m.add_class::<crate::panel::Panel>()?;
    m.add_class::<crate::membership::MembershipInterval>()?;
    m.add_class::<crate::membership::MembershipSeries>()?;

    Ok(())
}
