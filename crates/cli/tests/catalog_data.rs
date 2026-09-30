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

use std::{
    path::Path,
    process::{Command, Output},
};

use nautilus_core::UnixNanos;
use nautilus_model::{
    data::QuoteTick,
    identifiers::InstrumentId,
    types::{Price, Quantity},
};
use nautilus_persistence::backend::parquet::catalog::ParquetDataCatalog;
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

// Writes a current-format catalog with one quote so the data subcommands have a catalog built from
// the repository's own test data without depending on the platform-specific migration path.
fn write_quote_catalog(path: &Path) {
    std::fs::create_dir_all(path).unwrap();

    let catalog = ParquetDataCatalog::from_uri(path.to_str().unwrap(), None, None, None, None)
        .expect("create catalog");
    let quote = QuoteTick::new(
        InstrumentId::from("EUR/USD.SIM"),
        Price::from("1.00001"),
        Price::from("1.00002"),
        Quantity::from("100000"),
        Quantity::from("100000"),
        UnixNanos::from(1_700_000_000_000_000_000u64),
        UnixNanos::from(1_700_000_000_000_000_001u64),
    );

    catalog
        .write_to_parquet(&[quote], None, None, None)
        .expect("write quote");
}

// A data file that is not valid Parquet makes the catalog malformed, so validate reports the
// reader error, an `invalid` status, and a non-zero exit status rather than an empty result.
#[test]
fn validate_reports_a_malformed_catalog() {
    let directory = TempDir::new().unwrap();
    let file = directory.path().join(
        "data/quotes/EURUSD.SIM/2024-01-01T00-00-00-000000000Z_2024-01-01T23-59-59-999999999Z.parquet",
    );
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, b"not a parquet file").unwrap();

    let output = nautilus(&["catalog", "validate", directory.path().to_str().unwrap()]);
    assert!(!output.status.success());

    let json: Value = serde_json::from_str(&stdout(&output)).unwrap();
    assert_eq!(json["status"], "invalid");
    assert!(!json["problems"].as_array().unwrap().is_empty());
}

// Convert a catalog written from the repository's own test data into a separate destination, and
// prove the destination is a readable catalog through inspect.
#[test]
fn convert_produces_a_catalog_that_reads() {
    let directory = TempDir::new().unwrap();
    let source = directory.path().join("source");
    let converted = directory.path().join("converted");
    write_quote_catalog(&source);

    let validate = nautilus(&["catalog", "validate", source.to_str().unwrap()]);
    assert!(validate.status.success());
    let validated: Value = serde_json::from_str(&stdout(&validate)).unwrap();
    assert_eq!(validated["status"], "ok");

    let convert = nautilus(&[
        "catalog",
        "convert",
        source.to_str().unwrap(),
        converted.to_str().unwrap(),
    ]);
    assert!(
        convert.status.success(),
        "{}",
        String::from_utf8_lossy(&convert.stderr)
    );

    let summary: Value = serde_json::from_str(&stdout(&convert)).unwrap();
    assert_eq!(summary["status"], "ok");
    let quote_rows = summary["converted"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["directory"] == "quotes")
        .expect("quotes were not converted")["rows"]
        .as_u64()
        .unwrap();
    assert_eq!(quote_rows, 1);

    let inspect = nautilus(&["catalog", "inspect", converted.to_str().unwrap()]);
    assert!(
        inspect.status.success(),
        "{}",
        String::from_utf8_lossy(&inspect.stderr)
    );

    let json: Value = serde_json::from_str(&stdout(&inspect)).unwrap();
    assert_eq!(json["status"], "ok");

    let names: Vec<&str> = json["data_types"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["name"].as_str().unwrap())
        .collect();
    assert!(names.contains(&"quotes"), "data types were {names:?}");
}

// Neither download nor generate has a loader or generator surface to delegate to, so each reports
// the missing prerequisite and exits non-zero rather than pretending to work.
#[test]
fn download_and_generate_report_the_missing_prerequisite() {
    for command in ["download", "generate"] {
        let directory = TempDir::new().unwrap();
        let output = nautilus(&[
            "catalog",
            command,
            directory.path().join("catalog").to_str().unwrap(),
        ]);

        assert!(!output.status.success(), "{command} should exit non-zero");

        let json: Value = serde_json::from_str(&stdout(&output)).unwrap();
        assert_eq!(json["status"], "unsupported");
        assert!(
            json["prerequisite"].as_str().unwrap().contains("requires"),
            "{command} did not name its prerequisite"
        );
    }
}

#[test]
fn catalog_help_lists_the_data_subcommands() {
    let output = nautilus(&["catalog", "--help"]);
    assert!(output.status.success());

    let text = stdout(&output);
    for name in [
        "inspect",
        "validate",
        "convert",
        "download",
        "generate",
        "migrate-parquet",
    ] {
        assert!(text.contains(name), "help did not list {name}: {text}");
    }
}
