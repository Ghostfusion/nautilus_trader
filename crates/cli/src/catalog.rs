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

//! Machine-readable data subcommands over the Parquet catalog and its existing loaders.
//!
//! Every subcommand writes one JSON document to standard output and reports failure through both
//! the document and the process exit status, so CI can distinguish a failure from an empty result.
//! The commands only read catalog data and, for `convert`, write a separate destination catalog.
//! No command reads or writes provider credentials.

use std::str::FromStr;

use ahash::AHashMap;
use nautilus_model::{
    data::NautilusDataType,
    instruments::{Instrument, NautilusInstrumentType},
};
use nautilus_persistence::{
    backend::{migration::build_catalog_migration_plan, parquet::catalog::ParquetDataCatalog},
    catalog::traits::{CatalogInstrumentQuery, CatalogQuery, CatalogReader, CatalogWriter},
};
use serde_json::{Value, json};

use crate::opt::{CatalogConvertOpt, CatalogDataOpt};

/// Schema identifier carried by every emitted JSON document.
const OUTPUT_SCHEMA: &str = "nautilus.catalog.cli/v1";

/// Explains why `download` cannot run with the surfaces available to this crate.
const DOWNLOAD_PREREQUISITE: &str = "requires a provider data client reachable from nautilus-cli; the crate depends on no adapter crate, so no provider loader or network client is available. Add an adapter data-loader dependency (for example nautilus-databento) and read its provider API key from the environment. No data was downloaded and no credentials were read or written.";

/// Explains why `generate` cannot run with the surfaces available to this crate.
const GENERATE_PREREQUISITE: &str = "requires a synthetic market-data generator to delegate to; the workspace has no Rust market-data generator (the generate_* helpers in crates/backtest/benches are benchmark-local, and TestOrdersGenerator emits orders rather than market data). Add a generator to an engine or testkit crate. No data was generated.";

/// Inspects a catalog's data types, identifiers, coverage, and instruments.
///
/// # Errors
///
/// Returns an error when the catalog cannot be listed or read.
pub(crate) fn run_inspect(args: &CatalogDataOpt) -> anyhow::Result<()> {
    silence_console_logging();
    let mut catalog = open_catalog(&args.catalog, &args.storage_options)?;

    let mut data_types = Vec::new();
    for name in catalog.list_data_types()? {
        let directory = format!("data/{name}");
        let intervals = catalog.get_directory_intervals(&directory)?;
        let coverage: Vec<Value> = intervals
            .iter()
            .map(|(start, end)| json!({"start_ns": start, "end_ns": end}))
            .collect();
        data_types.push(json!({
            "name": name,
            "identifiers": catalog.list_directory_stems(&directory)?,
            "files": intervals.len(),
            "coverage": coverage,
        }));
    }

    let instruments = CatalogReader::instruments(&mut catalog, &CatalogInstrumentQuery::new())?;
    let instruments: Vec<Value> = instruments
        .iter()
        .map(|instrument| {
            json!({
                "id": instrument.id().to_string(),
                "class": instrument.instrument_class().to_string(),
            })
        })
        .collect();

    emit(&json!({
        "schema": OUTPUT_SCHEMA,
        "command": "inspect",
        "status": "ok",
        "catalog": args.catalog,
        "data_types": data_types,
        "instruments": instruments,
    }));

    Ok(())
}

/// Validates a catalog: reports schema and layout problems, then decodes each data family.
///
/// The schema and layout check reuses the migration preflight planner, and the decode sweep reads
/// through the catalog reader, so both the structural and row-level problems surface as the
/// reader's own errors. A malformed catalog is reported and exits non-zero.
///
/// # Errors
///
/// Returns an error when the catalog is malformed or cannot be opened.
pub(crate) fn run_validate(args: &CatalogDataOpt) -> anyhow::Result<()> {
    silence_console_logging();
    let mut catalog = open_catalog(&args.catalog, &args.storage_options)?;
    let mut problems: Vec<String> = Vec::new();

    match build_catalog_migration_plan(&catalog) {
        Ok(plan) => {
            for conflict in &plan.conflicts {
                let examples = conflict
                    .examples
                    .iter()
                    .map(|example| format!("{} ({})", example.path, example.fingerprint))
                    .collect::<Vec<_>>()
                    .join(", ");
                problems.push(format!(
                    "conflicting schemas for {}: {examples}",
                    conflict.target_table
                ));
            }
            for unresolved in &plan.unresolved_schemas {
                problems.push(unresolved.message.clone());
            }
        }
        Err(error) => problems.push(format!("catalog schema preflight failed: {error}")),
    }

    for name in catalog.list_data_types()? {
        let query = match resolve_family(&name) {
            Some(FamilySelector::Data(data_type)) => {
                if matches!(data_type, NautilusDataType::Custom { .. }) {
                    continue;
                }
                CatalogQuery::new(data_type)
            }
            Some(FamilySelector::InstrumentClass(instrument_type)) => {
                CatalogQuery::new(NautilusDataType::Instrument)
                    .with_instrument_type(Some(instrument_type))
            }
            None => continue,
        };

        if let Err(error) = CatalogReader::query_batch(&mut catalog, &query) {
            problems.push(format!("failed to read data type '{name}': {error}"));
        }
    }

    if problems.is_empty() {
        emit(&json!({
            "schema": OUTPUT_SCHEMA,
            "command": "validate",
            "status": "ok",
            "catalog": args.catalog,
            "problems": [],
        }));
        return Ok(());
    }

    emit(&json!({
        "schema": OUTPUT_SCHEMA,
        "command": "validate",
        "status": "invalid",
        "catalog": args.catalog,
        "problems": problems,
    }));

    anyhow::bail!(
        "catalog validation failed with {} problem(s)",
        problems.len()
    );
}

/// Converts supported data families from a source catalog into an empty destination catalog.
///
/// Each family is read through `CatalogReader` and written through `CatalogWriter`. Families the
/// reader cannot round-trip, such as custom types that need a registered decoder or record types,
/// are reported as skipped rather than converted.
///
/// # Errors
///
/// Returns an error when a location is invalid, the destination is not empty, or a read or write
/// fails.
pub(crate) fn run_convert(args: &CatalogConvertOpt) -> anyhow::Result<()> {
    silence_console_logging();
    anyhow::ensure!(
        !same_location(&args.source, &args.destination),
        "source and destination catalogs must differ"
    );

    create_local_directory(&args.destination)?;

    let mut source = open_catalog(&args.source, &args.source_options)?;
    let mut target = open_catalog(&args.destination, &args.target_options)?;
    ensure_target_empty(&target)?;

    let present = source.list_data_types()?;
    let selected: Vec<String> = if args.data_types.is_empty() {
        present.clone()
    } else {
        args.data_types.clone()
    };

    let mut converted = Vec::new();
    let mut skipped = Vec::new();

    for name in selected {
        if !present.contains(&name) {
            skipped.push(json!({
                "data_type": name,
                "reason": "not present in the source catalog",
            }));
            continue;
        }

        let query = match resolve_family(&name) {
            Some(FamilySelector::Data(data_type)) => {
                if matches!(data_type, NautilusDataType::Custom { .. })
                    && args.data_types.is_empty()
                {
                    skipped.push(json!({
                        "data_type": name,
                        "reason": "custom data needs a registered Arrow decoder; select it explicitly to attempt it",
                    }));
                    continue;
                }
                CatalogQuery::new(data_type)
            }
            Some(FamilySelector::InstrumentClass(instrument_type)) => {
                CatalogQuery::new(NautilusDataType::Instrument)
                    .with_instrument_type(Some(instrument_type))
            }
            None => {
                skipped.push(json!({
                    "data_type": name,
                    "reason": "not a supported data family",
                }));
                continue;
            }
        };

        let identifiers = if args.identifiers.is_empty() {
            None
        } else {
            Some(args.identifiers.clone())
        };
        let query = query.with_identifiers(identifiers);
        let batch = CatalogReader::query_batch(&mut source, &query)?;
        let rows = batch.len();

        if rows > 0 {
            CatalogWriter::write_data_batch(&mut target, &batch, None, None, None)?;
        }

        converted.push(json!({
            "data_type": query.data_type.to_string(),
            "directory": name,
            "rows": rows,
        }));
    }

    emit(&json!({
        "schema": OUTPUT_SCHEMA,
        "command": "convert",
        "status": "ok",
        "source": args.source,
        "destination": args.destination,
        "converted": converted,
        "skipped": skipped,
    }));

    Ok(())
}

/// Reports that `download` is unavailable and exits non-zero.
///
/// # Errors
///
/// Always returns an error naming the missing prerequisite.
pub(crate) fn run_download(args: &CatalogDataOpt) -> anyhow::Result<()> {
    silence_console_logging();
    unsupported("download", &args.catalog, DOWNLOAD_PREREQUISITE)
}

/// Reports that `generate` is unavailable and exits non-zero.
///
/// # Errors
///
/// Always returns an error naming the missing prerequisite.
pub(crate) fn run_generate(args: &CatalogDataOpt) -> anyhow::Result<()> {
    silence_console_logging();
    unsupported("generate", &args.catalog, GENERATE_PREREQUISITE)
}

/// Suppresses console logging so the JSON document is the only standard output.
///
/// The data subcommands emit one machine-readable document on standard output, so console logging
/// is disabled unless `NAUTILUS_LOG` is set to configure it explicitly. Failures stay visible
/// through the document and the process exit status.
pub(crate) fn silence_console_logging() {
    if std::env::var_os("NAUTILUS_LOG").is_none() {
        log::set_max_level(log::LevelFilter::Off);
    }
}

fn open_catalog(uri: &str, options: &[(String, String)]) -> anyhow::Result<ParquetDataCatalog> {
    let options: AHashMap<String, String> = options.iter().cloned().collect();
    let options = if options.is_empty() {
        None
    } else {
        Some(options)
    };
    ParquetDataCatalog::from_uri(uri, options, None, None, None)
}

fn ensure_target_empty(catalog: &ParquetDataCatalog) -> anyhow::Result<()> {
    anyhow::ensure!(
        catalog.list_data_types()?.is_empty()
            && catalog.list_backtest_runs()?.is_empty()
            && catalog.list_live_runs()?.is_empty(),
        "destination catalog must be empty"
    );
    Ok(())
}

fn create_local_directory(uri: &str) -> anyhow::Result<()> {
    if !uri.contains("://") {
        std::fs::create_dir_all(uri)?;
    }
    Ok(())
}

/// Returns whether two locations name the same catalog.
///
/// Local paths are compared after canonicalization when both resolve, so `.` and an absolute path
/// to the same directory are recognized; remote URIs and unresolved paths are compared verbatim.
fn same_location(source: &str, destination: &str) -> bool {
    if source == destination {
        return true;
    }

    match (
        std::fs::canonicalize(source),
        std::fs::canonicalize(destination),
    ) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

/// A catalog directory that maps to a readable family.
#[derive(Debug, Clone)]
enum FamilySelector {
    Data(NautilusDataType),
    InstrumentClass(NautilusInstrumentType),
}

/// Resolves a `data/` directory name to a readable family.
///
/// A data family name such as `quotes` maps to its data type; an instrument class directory such
/// as `currency_pair` maps to the aggregate instrument family restricted to that class.
fn resolve_family(name: &str) -> Option<FamilySelector> {
    if let Ok(data_type) = NautilusDataType::from_str(name) {
        return Some(FamilySelector::Data(data_type));
    }
    NautilusInstrumentType::from_str(name)
        .ok()
        .map(FamilySelector::InstrumentClass)
}

pub(crate) fn emit(value: &Value) {
    println!("{value}");
}

fn unsupported(command: &str, catalog: &str, prerequisite: &str) -> anyhow::Result<()> {
    emit(&json!({
        "schema": OUTPUT_SCHEMA,
        "command": command,
        "status": "unsupported",
        "catalog": catalog,
        "prerequisite": prerequisite,
    }));

    anyhow::bail!("{command} is not available: {prerequisite}")
}
