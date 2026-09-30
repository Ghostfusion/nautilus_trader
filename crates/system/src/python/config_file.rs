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

//! Python helpers for the optional configuration file surface.
//!
//! These bind [`crate::config_file`] for the typed configuration pyclasses exposed by other
//! crates. The typed constructors remain the canonical API; a configuration file is a view of a
//! typed configuration that can be saved and loaded. Overrides are explicit caller input and are
//! never sourced from the environment.

use nautilus_core::python::{to_pytype_err, to_pyvalue_err};
use pyo3::{
    Bound, PyResult,
    types::{
        PyAny, PyAnyMethods, PyDict, PyDictMethods, PyList, PyListMethods, PyTuple, PyTupleMethods,
        PyTypeMethods,
    },
};
use serde::{Serialize, de::DeserializeOwned};

use crate::config_file::{load_config, save_config};

/// Serializes `config` to the JSON file at `path` as pretty-printed JSON.
///
/// The typed constructor remains the canonical API; this writes a view of the configuration that
/// can later be reloaded with [`load_config_file`].
///
/// # Errors
///
/// Returns a `ValueError` if the configuration cannot be serialized or the file cannot be written.
pub fn save_config_file<T>(path: &str, config: &T) -> PyResult<()>
where
    T: Serialize,
{
    save_config(path, config).map_err(to_pyvalue_err)
}

/// Loads a configuration of type `T` from the JSON file at `path`.
///
/// When `overrides` is given it is merged recursively over the file contents and takes precedence.
///
/// # Errors
///
/// Returns a `ValueError` if the file cannot be read or decoded, if it contains an unknown key, or
/// if `overrides` contains a value that cannot be represented as JSON. Returns a `TypeError` if an
/// override mapping key is not a string.
pub fn load_config_file<T>(path: &str, overrides: Option<Bound<'_, PyDict>>) -> PyResult<T>
where
    T: DeserializeOwned,
{
    let overrides = overrides
        .map(|dict| py_to_json_value(dict.as_any()))
        .transpose()?;

    load_config(path, overrides.as_ref()).map_err(to_pyvalue_err)
}

/// Converts a Python value into a JSON value for configuration overrides.
///
/// # Errors
///
/// Returns a `TypeError` if an override mapping key is not a string or a value is not
/// representable as JSON.
fn py_to_json_value(value: &Bound<'_, PyAny>) -> PyResult<serde_json::Value> {
    if value.is_none() {
        return Ok(serde_json::Value::Null);
    }

    // Check `bool` before `int`, as Python `bool` is a subclass of `int`.
    if let Ok(bool_value) = value.extract::<bool>() {
        return Ok(serde_json::Value::Bool(bool_value));
    }

    if let Ok(int_value) = value.extract::<i64>() {
        return Ok(serde_json::Value::Number(int_value.into()));
    }

    if let Ok(uint_value) = value.extract::<u64>() {
        return Ok(serde_json::Value::Number(uint_value.into()));
    }

    if let Ok(float_value) = value.extract::<f64>() {
        let number = serde_json::Number::from_f64(float_value).ok_or_else(|| {
            to_pyvalue_err(format!(
                "override value {float_value} is not a finite JSON number"
            ))
        })?;
        return Ok(serde_json::Value::Number(number));
    }

    if let Ok(string_value) = value.extract::<String>() {
        return Ok(serde_json::Value::String(string_value));
    }

    if let Ok(dict) = value.cast::<PyDict>() {
        let mut map = serde_json::Map::with_capacity(dict.len());
        for (key, item) in dict.iter() {
            let key = key
                .extract::<String>()
                .map_err(|_| to_pytype_err("override mapping keys must be strings"))?;
            map.insert(key, py_to_json_value(&item)?);
        }
        return Ok(serde_json::Value::Object(map));
    }

    if let Ok(list) = value.cast::<PyList>() {
        let items = list
            .iter()
            .map(|item| py_to_json_value(&item))
            .collect::<PyResult<Vec<_>>>()?;
        return Ok(serde_json::Value::Array(items));
    }

    if let Ok(tuple) = value.cast::<PyTuple>() {
        let items = tuple
            .iter()
            .map(|item| py_to_json_value(&item))
            .collect::<PyResult<Vec<_>>>()?;
        return Ok(serde_json::Value::Array(items));
    }

    Err(to_pytype_err(format!(
        "override value of type `{}` cannot be represented as JSON",
        value.get_type().name()?
    )))
}
