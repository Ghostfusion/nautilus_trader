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

//! Guards the configuration file schema against cargo-feature drift.
//!
//! A configuration file is exactly the typed configuration's `Serialize`/`Deserialize` surface,
//! so a field behind a cargo feature must be enabled in the validating build. The Python extension
//! always enables the `streaming` feature on `BacktestEngineConfig` and `LiveNodeConfig`, so the
//! CLI must enable the same features or it cannot read the file the operator's own Python code
//! wrote. These tests fail if the feature-gated keys disappear from the CLI's schema.

use std::process::{Command, Output};

use nautilus_backtest::config::BacktestEngineConfig;
use nautilus_live::config::LiveNodeConfig;
use serde::Serialize;
use serde_json::Value;

/// Serializes a default typed configuration, which is the file schema for that type.
fn default_document<T: Default + Serialize>() -> Value {
    serde_json::to_value(T::default()).unwrap()
}

/// Runs `nautilus config validate` on the document against the given schema selector.
fn validate_with_binary(document: &Value, schema: &str) -> Output {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.json");
    std::fs::write(&path, serde_json::to_string_pretty(document).unwrap()).unwrap();

    Command::new(env!("CARGO_BIN_EXE_nautilus"))
        .args([
            "config",
            "validate",
            path.to_str().unwrap(),
            "--schema",
            schema,
        ])
        .current_dir(std::env::temp_dir())
        .output()
        .expect("failed to spawn nautilus binary")
}

/// Asserts the feature-gated keys are present and the document validates as `ok`.
fn assert_schema_includes_streaming_keys(document: &Value, schema: &str) {
    for key in ["streaming", "catalogs"] {
        assert!(
            document.get(key).is_some(),
            "the {schema} schema must include the feature-gated `{key}` key; \
             the `streaming` cargo feature is not enabled in this build"
        );
    }

    let output = validate_with_binary(document, schema);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "`{schema}` document with streaming keys failed: {stdout}"
    );
    assert!(
        stdout.contains("\"status\":\"ok\""),
        "expected an ok status for `{schema}`, stdout: {stdout}"
    );
}

#[test]
fn backtest_schema_includes_streaming_keys() {
    assert_schema_includes_streaming_keys(&default_document::<BacktestEngineConfig>(), "backtest");
}

#[test]
fn live_schema_includes_streaming_keys() {
    assert_schema_includes_streaming_keys(&default_document::<LiveNodeConfig>(), "live");
}
