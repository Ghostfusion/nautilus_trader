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

//! Failures of the machine-readable commands are never silent.
//!
//! The catalog data subcommands, `config`, and `optimize` silence console logging so the JSON
//! document is the only standard output. A failure raised before the document was built would
//! otherwise be reported only through the silenced logging layer, leaving a CI consumer with a
//! non-zero exit and an empty stream. These tests pin that every such failure still reaches the
//! caller, either as an error document on stdout or as a message on stderr.

use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;

fn nautilus(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_nautilus"))
        .args(args)
        .output()
        .expect("failed to spawn nautilus binary")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).to_string()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).to_string()
}

fn missing_path(prefix: &str) -> String {
    std::env::temp_dir()
        .join(format!(
            "nautilus-cli-missing-{prefix}-{}",
            std::process::id()
        ))
        .to_str()
        .unwrap()
        .to_string()
}

// A path that does not exist fails before the catalog envelope is built, so the command must still
// print exactly one error document rather than exiting silently.
#[test]
fn missing_catalog_path_emits_one_error_document() {
    for subcommand in ["inspect", "validate", "convert"] {
        let output = if subcommand == "convert" {
            let destination = TempDir::new().unwrap();
            nautilus(&[
                "catalog",
                subcommand,
                &missing_path(subcommand),
                destination.path().to_str().unwrap(),
            ])
        } else {
            nautilus(&["catalog", subcommand, &missing_path(subcommand)])
        };

        assert_eq!(output.status.code(), Some(1));
        let document: Value =
            serde_json::from_str(&stdout(&output)).expect("exactly one JSON document");
        assert_eq!(document["schema"], "nautilus.catalog.cli/v1");
        assert_eq!(document["command"], subcommand);
        assert_eq!(document["status"], "error");
        assert!(document["error"].is_string(), "document: {document}");
    }
}

#[test]
fn missing_config_file_emits_an_invalid_document() {
    let output = nautilus(&[
        "config",
        "validate",
        &missing_path("config"),
        "--schema",
        "kernel",
    ]);

    assert_eq!(output.status.code(), Some(1));
    let document: Value =
        serde_json::from_str(&stdout(&output)).expect("exactly one JSON document");
    assert_eq!(document["schema"], "nautilus.config.cli/v1");
    assert_eq!(document["status"], "invalid");
    assert!(document["error"].is_string(), "document: {document}");
}

// An unresolvable interpreter fails before the child runs, so the failure is reported on stderr
// because there is no child document to relay.
#[test]
fn optimize_reports_an_unresolvable_interpreter_on_stderr() {
    let output = Command::new(env!("CARGO_BIN_EXE_nautilus"))
        .args(["optimize", "config.json"])
        .env_remove("NAUTILUS_PYTHON")
        .env_remove("VIRTUAL_ENV")
        .env("PATH", "")
        .output()
        .expect("failed to spawn nautilus binary");

    assert_eq!(output.status.code(), Some(1));
    assert!(stdout(&output).is_empty(), "stdout: {}", stdout(&output));
    assert!(
        stderr(&output).contains("no Python interpreter found"),
        "stderr: {}",
        stderr(&output)
    );
}
