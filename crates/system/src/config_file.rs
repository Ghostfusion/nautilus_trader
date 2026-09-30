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

//! Optional JSON file serialization and loading for typed configurations.
//!
//! Typed configuration constructors remain the canonical API. A configuration file is a view of a
//! typed configuration, not a second configuration model, so the schema is exactly the
//! `Serialize`/`Deserialize` surface of the typed configuration.
//!
//! Loading layers the built-in defaults, then the file, then any explicit overrides. Unknown keys
//! are rejected because the typed configurations use `deny_unknown_fields`. Environment profiles
//! and environment variables are deliberately not a configuration source.

use std::{
    error::Error,
    fmt::{Debug, Display},
    fs,
    path::{Path, PathBuf},
};

use serde::{Serialize, de::DeserializeOwned};

/// An error from serializing or loading a configuration file.
#[derive(Debug)]
pub enum ConfigFileError {
    /// The configuration file could not be read.
    Read {
        /// The path that could not be read.
        path: PathBuf,
        /// The underlying I/O error.
        source: std::io::Error,
    },
    /// The configuration file could not be written.
    Write {
        /// The path that could not be written.
        path: PathBuf,
        /// The underlying I/O error.
        source: std::io::Error,
    },
    /// The configuration file contents could not be decoded into the typed configuration.
    Decode {
        /// The path that failed to decode.
        path: PathBuf,
        /// The underlying decode error.
        source: serde_json::Error,
    },
    /// The typed configuration could not be encoded for writing.
    Encode {
        /// The underlying encode error.
        source: serde_json::Error,
    },
}

impl Display for ConfigFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(
                    f,
                    "failed to read configuration file '{}': {source}",
                    path.display()
                )
            }
            Self::Write { path, source } => {
                write!(
                    f,
                    "failed to write configuration file '{}': {source}",
                    path.display()
                )
            }
            Self::Decode { path, source } => {
                write!(
                    f,
                    "failed to decode configuration file '{}': {source}",
                    path.display()
                )
            }
            Self::Encode { source } => write!(f, "failed to encode configuration: {source}"),
        }
    }
}

impl Error for ConfigFileError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Read { source, .. } | Self::Write { source, .. } => Some(source),
            Self::Decode { source, .. } | Self::Encode { source } => Some(source),
        }
    }
}

/// Serializes a typed configuration to `path` as pretty-printed JSON.
///
/// The typed configuration constructors remain the canonical API; this writes a view of the
/// configuration that can later be loaded with [`load_config`].
///
/// # Errors
///
/// Returns [`ConfigFileError::Encode`] if the configuration cannot be serialized, or
/// [`ConfigFileError::Write`] if the file cannot be written.
pub fn save_config<T, P>(path: P, config: &T) -> Result<(), ConfigFileError>
where
    T: Serialize,
    P: AsRef<Path>,
{
    let path = path.as_ref();
    let json = serde_json::to_string_pretty(config)
        .map_err(|source| ConfigFileError::Encode { source })?;
    fs::write(path, json).map_err(|source| ConfigFileError::Write {
        path: path.to_path_buf(),
        source,
    })
}

/// Loads a typed configuration from the JSON file at `path`.
///
/// The loader layers the built-in defaults, then the file, then `overrides` (which are merged
/// recursively over the file contents and take precedence). Unknown keys in the file or the
/// overrides are rejected.
///
/// # Errors
///
/// Returns an error if the file cannot be read, if the contents cannot be decoded into `T`, or if
/// the contents contain keys not present in `T`.
pub fn load_config<T, P>(
    path: P,
    overrides: Option<&serde_json::Value>,
) -> Result<T, ConfigFileError>
where
    T: DeserializeOwned,
    P: AsRef<Path>,
{
    let path = path.as_ref();
    let contents = fs::read_to_string(path).map_err(|source| ConfigFileError::Read {
        path: path.to_path_buf(),
        source,
    })?;

    let value = match overrides {
        None => {
            return serde_json::from_str(&contents).map_err(|source| ConfigFileError::Decode {
                path: path.to_path_buf(),
                source,
            });
        }
        Some(overrides) => {
            let mut value: serde_json::Value =
                serde_json::from_str(&contents).map_err(|source| ConfigFileError::Decode {
                    path: path.to_path_buf(),
                    source,
                })?;
            merge_json(&mut value, overrides);
            value
        }
    };

    serde_json::from_value(value).map_err(|source| ConfigFileError::Decode {
        path: path.to_path_buf(),
        source,
    })
}

/// Merges `overrides` into `target`, recursing into objects and otherwise replacing values.
fn merge_json(target: &mut serde_json::Value, overrides: &serde_json::Value) {
    match (target, overrides) {
        (serde_json::Value::Object(target), serde_json::Value::Object(overrides)) => {
            for (key, value) in overrides {
                match target.get_mut(key) {
                    Some(existing) => merge_json(existing, value),
                    None => {
                        target.insert(key.clone(), value.clone());
                    }
                }
            }
        }
        (target, overrides) => *target = overrides.clone(),
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    fn test_merge_json_prefers_scalar_override() {
        let mut target = serde_json::json!({ "a": 1, "b": 2 });
        merge_json(&mut target, &serde_json::json!({ "b": 3, "c": 4 }));

        assert_eq!(target, serde_json::json!({ "a": 1, "b": 3, "c": 4 }));
    }

    #[rstest]
    fn test_merge_json_recurses_into_objects() {
        let mut target = serde_json::json!({ "a": { "x": 1, "y": 2 } });
        merge_json(&mut target, &serde_json::json!({ "a": { "y": 3 } }));

        assert_eq!(target, serde_json::json!({ "a": { "x": 1, "y": 3 } }));
    }
}
