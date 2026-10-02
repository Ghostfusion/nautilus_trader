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

pub mod config;
pub mod factories;

use nautilus_common::factories::{ClientConfig, DataClientFactory};
use nautilus_core::python::{to_pyruntime_err, to_pyvalue_err};
use nautilus_system::get_global_pyo3_registry;
use pyo3::prelude::*;

use crate::{
    common::CLIENT_ID, config::MoomooDataClientConfig, factories::MoomooDataClientFactory,
};

#[expect(clippy::needless_pass_by_value)]
fn extract_moomoo_data_factory(
    py: Python<'_>,
    factory: Py<PyAny>,
) -> PyResult<Box<dyn DataClientFactory>> {
    match factory.extract::<MoomooDataClientFactory>(py) {
        Ok(f) => Ok(Box::new(f)),
        Err(e) => Err(to_pyvalue_err(format!(
            "Failed to extract MoomooDataClientFactory: {e}"
        ))),
    }
}

#[expect(clippy::needless_pass_by_value)]
fn extract_moomoo_data_config(
    py: Python<'_>,
    config: Py<PyAny>,
) -> PyResult<Box<dyn ClientConfig>> {
    match config.extract::<MoomooDataClientConfig>(py) {
        Ok(c) => Ok(Box::new(c)),
        Err(e) => Err(to_pyvalue_err(format!(
            "Failed to extract MoomooDataClientConfig: {e}"
        ))),
    }
}

/// Moomoo Python module.
///
/// The module is exposed as `nautilus_trader._libnautilus.moomoo`.
///
/// # Errors
///
/// Returns a `PyErr` if registering any module components fails.
#[pymodule]
pub fn moomoo(_: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<MoomooDataClientConfig>()?;
    m.add_class::<MoomooDataClientFactory>()?;

    let registry = get_global_pyo3_registry();

    if let Err(e) =
        registry.register_factory_extractor(CLIENT_ID.to_string(), extract_moomoo_data_factory)
    {
        return Err(to_pyruntime_err(format!(
            "Failed to register moomoo data factory extractor: {e}"
        )));
    }

    if let Err(e) = registry.register_config_extractor(
        "MoomooDataClientConfig".to_string(),
        extract_moomoo_data_config,
    ) {
        return Err(to_pyruntime_err(format!(
            "Failed to register moomoo data config extractor: {e}"
        )));
    }

    Ok(())
}
