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

//! Machine-readable subcommands over typed configuration files and the existing loader.
//!
//! Every subcommand writes one JSON document to standard output and reports failure through both
//! the document and the process exit status, so a CI step can tell a failure from a success.
//! Console logging is disabled by default so the document is the only standard output.
//!
//! There is no second schema and no second loader: the file is a view of an existing typed
//! configuration, `KernelConfig`, `BacktestEngineConfig`, or `LiveNodeConfig` selected by
//! `--schema`, and the commands delegate to [`nautilus_system::config_file::load_config`], whose
//! layering rejects unknown keys.

use nautilus_backtest::config::BacktestEngineConfig;
use nautilus_live::config::LiveNodeConfig;
use nautilus_system::{config::KernelConfig, config_file::load_config};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};

use crate::{
    catalog::{emit, silence_console_logging},
    opt::{ConfigFileOpt, ConfigSchema},
};

/// Schema identifier carried by every emitted JSON document.
const OUTPUT_SCHEMA: &str = "nautilus.config.cli/v1";

/// Validates a configuration file against the selected typed configuration.
///
/// The file is read by the existing loader, so the validation is the loader's own decoding and
/// unknown-key rejection rather than a separate schema check.
///
/// # Errors
///
/// Returns an error when the file cannot be read, when its contents cannot be decoded into the
/// selected typed configuration, or when it contains keys that configuration does not define.
pub(crate) fn run_validate(args: &ConfigFileOpt) -> anyhow::Result<()> {
    silence_console_logging();
    let (document, error) = envelope(args, "validate", false);
    emit(&document);
    finish(error)
}

/// Prints the resolved configuration with the built-in defaults and the file applied.
///
/// The printed document is the selected typed configuration after loading, so an operator sees
/// exactly what the loader produced rather than a re-reading of the file.
///
/// # Errors
///
/// Returns an error when the file cannot be read, when its contents cannot be decoded into the
/// selected typed configuration, or when it contains keys that configuration does not define.
pub(crate) fn run_resolve(args: &ConfigFileOpt) -> anyhow::Result<()> {
    silence_console_logging();
    let (document, error) = envelope(args, "resolve", true);
    emit(&document);
    finish(error)
}

/// Reports a loading failure after the document has been emitted.
fn finish(error: Option<String>) -> anyhow::Result<()> {
    match error {
        Some(message) => anyhow::bail!("configuration command failed: {message}"),
        None => Ok(()),
    }
}

/// Builds the emitted document and the failure message for one invocation.
///
/// The returned message is `None` on success and the loader's own message on failure, so the
/// caller can both emit the document and report the failure through the exit status.
fn envelope(args: &ConfigFileOpt, command: &str, resolved: bool) -> (Value, Option<String>) {
    match args.schema {
        ConfigSchema::Kernel => build::<KernelConfig>(args, command, resolved),
        ConfigSchema::Backtest => build::<BacktestEngineConfig>(args, command, resolved),
        ConfigSchema::Live => build::<LiveNodeConfig>(args, command, resolved),
    }
}

/// Loads the file as `T` and builds the document, optionally including the resolved configuration.
fn build<T>(args: &ConfigFileOpt, command: &str, resolved: bool) -> (Value, Option<String>)
where
    T: DeserializeOwned + Serialize,
{
    let mut document = json!({
        "schema": OUTPUT_SCHEMA,
        "command": command,
        "config": args.config.display().to_string(),
        "config_schema": args.schema.as_str(),
    });

    let config = match load_config::<T, _>(&args.config, None) {
        Ok(config) => config,
        Err(error) => return invalid(document, &error.to_string()),
    };

    document["status"] = json!("ok");

    if resolved {
        match serde_json::to_value(&config) {
            Ok(value) => document["config_resolved"] = value,
            Err(error) => return invalid(document, &error.to_string()),
        }
    }

    (document, None)
}

/// Marks a document invalid with the loader's own message and returns that message.
fn invalid(mut document: Value, message: &str) -> (Value, Option<String>) {
    document["status"] = json!("invalid");
    document["error"] = json!(message);
    (document, Some(message.to_string()))
}

#[cfg(test)]
mod tests {
    use nautilus_system::config_file::save_config;
    use tempfile::TempDir;

    use super::*;

    fn default_file<T: Default + Serialize>(directory: &TempDir, name: &str) -> std::path::PathBuf {
        let path = directory.path().join(name);
        save_config(&path, &T::default()).unwrap();
        path
    }

    fn rewrite(path: &std::path::Path, mutate: impl FnOnce(&mut Value)) {
        let mut value: Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        mutate(&mut value);
        std::fs::write(path, serde_json::to_string_pretty(&value).unwrap()).unwrap();
    }

    fn args(path: &std::path::Path, schema: ConfigSchema) -> ConfigFileOpt {
        ConfigFileOpt {
            config: path.to_path_buf(),
            schema,
        }
    }

    #[test]
    fn validate_accepts_a_file_written_by_save_config() {
        let directory = TempDir::new().unwrap();
        let path = default_file::<KernelConfig>(&directory, "kernel.json");

        let (document, error) = envelope(&args(&path, ConfigSchema::Kernel), "validate", false);

        assert!(error.is_none());
        assert_eq!(document["schema"], OUTPUT_SCHEMA);
        assert_eq!(document["command"], "validate");
        assert_eq!(document["status"], "ok");
        assert_eq!(document["config_schema"], "KernelConfig");
        assert!(document.get("config_resolved").is_none());
    }

    #[test]
    fn validate_rejects_an_unknown_key() {
        let directory = TempDir::new().unwrap();
        let path = default_file::<KernelConfig>(&directory, "kernel.json");
        rewrite(&path, |value| {
            value["unexpected_key"] = json!(1);
        });

        let (document, error) = envelope(&args(&path, ConfigSchema::Kernel), "validate", false);

        let message = error.expect("an unknown key must fail validation");
        assert_eq!(document["status"], "invalid");
        assert!(
            message.contains("unknown field `unexpected_key`"),
            "unexpected message: {message}"
        );
        assert_eq!(document["error"], json!(message));
    }

    #[test]
    fn validate_rejects_malformed_json() {
        let directory = TempDir::new().unwrap();
        let path = directory.path().join("broken.json");
        std::fs::write(&path, "{").unwrap();

        let (document, error) = envelope(&args(&path, ConfigSchema::Kernel), "validate", false);

        let message = error.expect("malformed JSON must fail validation");
        assert_eq!(document["status"], "invalid");
        assert!(
            message.contains("failed to decode configuration file"),
            "unexpected message: {message}"
        );
    }

    #[test]
    fn resolve_materializes_defaults_and_file_overrides() {
        let directory = TempDir::new().unwrap();
        let path = default_file::<KernelConfig>(&directory, "kernel.json");
        rewrite(&path, |value| {
            value["trader_id"] = json!("TRADER-042");
        });

        let (document, error) = envelope(&args(&path, ConfigSchema::Kernel), "resolve", true);

        assert!(error.is_none());
        assert_eq!(document["status"], "ok");
        let resolved = &document["config_resolved"];
        // The file value is applied and the unset fields are materialized from the defaults.
        assert_eq!(resolved["trader_id"], json!("TRADER-042"));
        assert_eq!(resolved["timeout_connection"]["secs"], json!(60));
        assert!(resolved["environment"].is_string());
        assert!(resolved["logging"].is_object());
    }

    #[test]
    fn resolve_loads_each_schema_from_its_own_file() {
        let directory = TempDir::new().unwrap();

        let cases = [
            (
                ConfigSchema::Kernel,
                "KernelConfig",
                default_file::<KernelConfig>(&directory, "kernel.json"),
            ),
            (
                ConfigSchema::Backtest,
                "BacktestEngineConfig",
                default_file::<BacktestEngineConfig>(&directory, "backtest.json"),
            ),
            (
                ConfigSchema::Live,
                "LiveNodeConfig",
                default_file::<LiveNodeConfig>(&directory, "live.json"),
            ),
        ];

        for (schema, name, path) in cases {
            let (document, error) = envelope(&args(&path, schema), "resolve", true);

            assert!(
                error.is_none(),
                "{name} should resolve its own default file"
            );
            assert_eq!(document["config_schema"], name);
            assert!(document["config_resolved"].is_object());
        }
    }
}
