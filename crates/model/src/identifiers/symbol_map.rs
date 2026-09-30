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

//! A symbol map as immutable data.
//!
//! An instrument's venue symbol can change over its life, and a historical dataset may address a
//! period by the symbol that was in force then. An instrument identity is stable; the symbol is a
//! label that moves between identities.
//!
//! A [`SymbolMap`] records, for each venue symbol, the instrument identity it denoted over a date
//! range. It answers which identity a symbol names on a given date, and it delivers each rename as
//! a [`CorporateAction`] of kind [`CorporateActionType::SymbolChange`] at the instant the successor
//! symbol takes effect.
//!
//! A symbol map is an immutable input to a run. It is loaded once, validated on load, and never
//! mutated while the run is in progress.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
    path::Path,
    str::FromStr,
};

use anyhow::{Context, Result, anyhow, ensure};
use jiff::{Timestamp, civil::Date, tz::TimeZone};
use nautilus_core::UnixNanos;
use rust_decimal::Decimal;
use serde::Deserialize;

use crate::{
    data::{CorporateAction, CorporateActionType},
    identifiers::{InstrumentId, Symbol},
};

/// The schema identifier of the symbol map JSON format.
pub const SYMBOL_MAP_SCHEMA: &str = "nautilus-symbol-map/v1";

/// One validity interval mapping a venue symbol to an instrument identity.
///
/// The interval is inclusive of both bounds: the symbol named `instrument_id` from `valid_from`
/// through `valid_until`, or indefinitely when `valid_until` is `None`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.model", from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.model")
)]
pub struct SymbolMapEntry {
    /// The venue symbol in force over the interval.
    pub symbol: Symbol,
    /// The instrument identity the symbol denoted over the interval.
    pub instrument_id: InstrumentId,
    /// The first date the symbol denoted the identity (inclusive).
    pub valid_from: Date,
    /// The last date the symbol denoted the identity (inclusive), if bounded.
    pub valid_until: Option<Date>,
}

impl SymbolMapEntry {
    /// Creates a new [`SymbolMapEntry`].
    #[must_use]
    pub const fn new(
        symbol: Symbol,
        instrument_id: InstrumentId,
        valid_from: Date,
        valid_until: Option<Date>,
    ) -> Self {
        Self {
            symbol,
            instrument_id,
            valid_from,
            valid_until,
        }
    }

    /// Returns whether the given date falls inside the validity interval.
    #[must_use]
    pub fn contains(&self, date: Date) -> bool {
        self.valid_from <= date && self.valid_until.is_none_or(|until| date <= until)
    }
}

/// An immutable symbol map.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.model", from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.model")
)]
pub struct SymbolMap {
    entries: Vec<SymbolMapEntry>,
}

impl SymbolMap {
    /// Parses and validates a symbol map from JSON text.
    ///
    /// # Errors
    ///
    /// Returns an error if the JSON is malformed, the schema is unsupported, a date or identifier
    /// cannot be parsed, an interval is inverted or overlapping, or a symbol resolves to more than
    /// one instrument identity.
    pub fn from_json_str(text: &str) -> Result<Self> {
        let file: SymbolMapFile = serde_json::from_str(text).context("invalid symbol map JSON")?;

        ensure!(
            file.schema == SYMBOL_MAP_SCHEMA,
            "unsupported symbol map schema: {}",
            file.schema
        );
        ensure!(!file.entries.is_empty(), "symbol map declares no entries");

        let mut entries = Vec::with_capacity(file.entries.len());
        for raw in &file.entries {
            let symbol = Symbol::new(&raw.symbol);
            let instrument_id = InstrumentId::from_as_ref(&raw.instrument_id)
                .map_err(|e| anyhow!("invalid symbol map instrument_id: {e}"))?;
            let valid_from = Date::from_str(&raw.valid_from)
                .with_context(|| format!("invalid symbol map valid_from: {}", raw.valid_from))?;
            let valid_until = raw
                .valid_until
                .as_deref()
                .map(|value| {
                    Date::from_str(value)
                        .with_context(|| format!("invalid symbol map valid_until: {value}"))
                })
                .transpose()?;

            if let Some(valid_until) = valid_until {
                ensure!(
                    valid_from <= valid_until,
                    "symbol map entry for {symbol} valid_from {valid_from} must not follow \
                     valid_until {valid_until}"
                );
            }

            entries.push(SymbolMapEntry::new(
                symbol,
                instrument_id,
                valid_from,
                valid_until,
            ));
        }

        validate_entries(&entries)?;

        Ok(Self { entries })
    }

    /// Parses and validates a symbol map from a JSON file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read, or if the contents are not a valid symbol map.
    pub fn from_json_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("cannot read symbol map: {}", path.display()))?;
        Self::from_json_str(&text)
    }

    /// Returns the entries, in file order.
    #[must_use]
    pub fn entries(&self) -> &[SymbolMapEntry] {
        &self.entries
    }

    /// Returns the instrument identity the symbol names on the given date, if any.
    #[must_use]
    pub fn resolve(&self, symbol: &Symbol, date: Date) -> Option<InstrumentId> {
        self.entries
            .iter()
            .find(|entry| entry.symbol == *symbol && entry.contains(date))
            .map(|entry| entry.instrument_id)
    }

    /// Returns the instrument identity the symbol names at the given instant.
    #[must_use]
    pub fn resolve_at(&self, symbol: &Symbol, ts: UnixNanos) -> Option<InstrumentId> {
        let date = ts.to_datetime_utc().to_zoned(TimeZone::UTC).date();
        self.resolve(symbol, date)
    }

    /// Returns the single instrument identity the symbol names, ignoring the date.
    ///
    /// Load-time validation rejects a symbol that resolves to more than one identity, so this is
    /// the canonical identity for the symbol over the map's whole coverage.
    #[must_use]
    pub fn identity(&self, symbol: &Symbol) -> Option<InstrumentId> {
        self.entries
            .iter()
            .find(|entry| entry.symbol == *symbol)
            .map(|entry| entry.instrument_id)
    }

    /// Returns the distinct instrument identities, sorted and deduplicated.
    #[must_use]
    pub fn instrument_ids(&self) -> Vec<InstrumentId> {
        let mut ids: Vec<InstrumentId> = self.entries.iter().map(|e| e.instrument_id).collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    }

    /// Returns each rename as a symbol change corporate action.
    ///
    /// An action is emitted at the instant the successor symbol takes effect (the successor
    /// entry's `valid_from` at midnight UTC), carries the successor symbol in `new_symbol`, and
    /// names the instrument identity. The result is ordered by effective instant, then by identity
    /// and successor symbol, so it is deterministic.
    #[must_use]
    pub fn changes(&self) -> Vec<CorporateAction> {
        let mut by_instrument: BTreeMap<InstrumentId, Vec<&SymbolMapEntry>> = BTreeMap::new();
        for entry in &self.entries {
            by_instrument
                .entry(entry.instrument_id)
                .or_default()
                .push(entry);
        }

        let mut changes = Vec::new();
        for (instrument_id, mut group) in by_instrument {
            group.sort_by_key(|entry| (entry.valid_from, entry.symbol));
            for pair in group.windows(2) {
                let [previous, successor] = pair else {
                    continue;
                };
                if previous.symbol == successor.symbol {
                    continue;
                }
                let effective_ns = date_to_nanos(successor.valid_from);
                changes.push(CorporateAction::new(
                    instrument_id,
                    CorporateActionType::SymbolChange,
                    Decimal::ZERO,
                    Some(successor.symbol),
                    effective_ns,
                    effective_ns,
                    effective_ns,
                ));
            }
        }

        changes
            .sort_by_key(|action| (action.effective_ns, action.instrument_id, action.new_symbol));
        changes
    }
}

impl FromStr for SymbolMap {
    type Err = anyhow::Error;

    fn from_str(text: &str) -> Result<Self> {
        Self::from_json_str(text)
    }
}

impl Display for SymbolMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SymbolMap(entries={})", self.entries.len())
    }
}

/// Validates the entry set as a whole.
///
/// Each symbol's intervals must not overlap and must resolve to exactly one identity, and each
/// identity's intervals (the rename chain) must not overlap, so no date is ambiguous.
fn validate_entries(entries: &[SymbolMapEntry]) -> Result<()> {
    let mut by_symbol: BTreeMap<&Symbol, Vec<&SymbolMapEntry>> = BTreeMap::new();
    for entry in entries {
        by_symbol.entry(&entry.symbol).or_default().push(entry);
    }

    for (symbol, group) in &by_symbol {
        let mut sorted = group.clone();
        sorted.sort_by_key(|entry| entry.valid_from);
        for pair in sorted.windows(2) {
            let [previous, next] = pair else {
                continue;
            };
            ensure!(
                previous
                    .valid_until
                    .is_some_and(|until| until < next.valid_from),
                "symbol map entries for {symbol} overlap: {} and {}",
                interval(previous),
                interval(next)
            );
        }

        let identities: BTreeSet<InstrumentId> =
            group.iter().map(|entry| entry.instrument_id).collect();
        ensure!(
            identities.len() == 1,
            "symbol map symbol {symbol} resolves to more than one instrument identity: {identities:?}"
        );
    }

    let mut by_instrument: BTreeMap<InstrumentId, Vec<&SymbolMapEntry>> = BTreeMap::new();
    for entry in entries {
        by_instrument
            .entry(entry.instrument_id)
            .or_default()
            .push(entry);
    }

    for (instrument_id, group) in &by_instrument {
        let mut sorted = group.clone();
        sorted.sort_by_key(|entry| entry.valid_from);
        for pair in sorted.windows(2) {
            let [previous, next] = pair else {
                continue;
            };
            ensure!(
                previous
                    .valid_until
                    .is_some_and(|until| until < next.valid_from),
                "rename chain for {instrument_id} overlaps: {} and {}",
                interval(previous),
                interval(next)
            );
        }
    }

    Ok(())
}

fn interval(entry: &SymbolMapEntry) -> String {
    match entry.valid_until {
        Some(until) => format!("{}..={until}", entry.valid_from),
        None => format!("{}..", entry.valid_from),
    }
}

fn date_to_nanos(date: Date) -> UnixNanos {
    let timestamp: Timestamp = date
        .at(0, 0, 0, 0)
        .to_zoned(TimeZone::UTC)
        .expect("a UTC date is unambiguous")
        .timestamp();
    UnixNanos::from(
        u64::try_from(timestamp.as_nanosecond()).expect("a civil date is within UnixNanos range"),
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SymbolMapFile {
    schema: String,
    entries: Vec<SymbolMapEntryFile>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SymbolMapEntryFile {
    symbol: String,
    instrument_id: String,
    valid_from: String,
    #[serde(default)]
    valid_until: Option<String>,
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;
    use rstest::rstest;

    use super::*;

    const RENAMED: &str = r#"
    {
      "schema": "nautilus-symbol-map/v1",
      "entries": [
        {
          "symbol": "OLD",
          "instrument_id": "NEW.XNYS",
          "valid_from": "2020-01-01",
          "valid_until": "2024-05-31"
        },
        {
          "symbol": "NEW",
          "instrument_id": "NEW.XNYS",
          "valid_from": "2024-06-01"
        }
      ]
    }"#;

    fn rename() -> SymbolMap {
        SymbolMap::from_json_str(RENAMED).expect("valid test symbol map")
    }

    #[rstest]
    fn test_mid_series_rename_resolves_to_one_identity() {
        let map = rename();
        let old = Symbol::from("OLD");
        let new = Symbol::from("NEW");
        let canonical = InstrumentId::from("NEW.XNYS");

        // Before the rename the old symbol names the identity.
        assert_eq!(map.resolve(&old, date(2023, 1, 1)), Some(canonical));
        // After the rename the new symbol names the same identity.
        assert_eq!(map.resolve(&new, date(2025, 1, 1)), Some(canonical));
        // Each symbol is absent outside its own interval.
        assert_eq!(map.resolve(&old, date(2025, 1, 1)), None);
        assert_eq!(map.resolve(&new, date(2023, 1, 1)), None);

        assert_eq!(map.identity(&old), Some(canonical));
        assert_eq!(map.identity(&new), Some(canonical));
        assert_eq!(map.instrument_ids(), vec![canonical]);
    }

    #[rstest]
    fn test_rename_is_delivered_as_a_symbol_change_at_its_effective_instant() {
        let map = rename();
        let changes = map.changes();

        assert_eq!(changes.len(), 1);
        let change = changes[0];
        assert_eq!(change.action, CorporateActionType::SymbolChange);
        assert_eq!(change.instrument_id, InstrumentId::from("NEW.XNYS"));
        assert_eq!(change.new_symbol, Some(Symbol::from("NEW")));
        assert_eq!(change.value, Decimal::ZERO);
        assert_eq!(change.effective_ns, date_to_nanos(date(2024, 6, 1)));
        assert_eq!(change.ts_event, change.effective_ns);
        assert_eq!(change.ts_init, change.effective_ns);
    }

    #[rstest]
    fn test_rejects_overlapping_entries_for_the_same_symbol() {
        let invalid = RENAMED.replace("\"2024-05-31\"", "\"2024-06-15\"");

        let error = SymbolMap::from_json_str(&invalid).expect_err("overlapping entries must fail");

        assert!(error.to_string().contains("overlap"), "{error}");
    }

    #[rstest]
    fn test_rejects_inverted_validity() {
        let invalid = RENAMED.replace("\"2020-01-01\"", "\"2025-01-01\"");

        let error = SymbolMap::from_json_str(&invalid).expect_err("inverted entries must fail");

        assert!(error.to_string().contains("must not follow"), "{error}");
    }

    #[rstest]
    fn test_rejects_a_symbol_that_resolves_to_two_identities() {
        let invalid = r#"
        {
          "schema": "nautilus-symbol-map/v1",
          "entries": [
            {
              "symbol": "OLD",
              "instrument_id": "OLD.XNYS",
              "valid_from": "2020-01-01",
              "valid_until": "2021-12-31"
            },
            {
              "symbol": "OLD",
              "instrument_id": "NEW.XNYS",
              "valid_from": "2022-01-01"
            }
          ]
        }"#;

        let error =
            SymbolMap::from_json_str(invalid).expect_err("a symbol with two identities must fail");

        assert!(
            error
                .to_string()
                .contains("more than one instrument identity"),
            "{error}"
        );
    }

    #[rstest]
    fn test_rejects_unsupported_schema() {
        let invalid = RENAMED.replace(SYMBOL_MAP_SCHEMA, "nautilus-symbol-map/v9");

        let error = SymbolMap::from_json_str(&invalid).expect_err("schema must be checked");

        assert!(
            error.to_string().contains("unsupported symbol map schema"),
            "{error}"
        );
    }

    #[rstest]
    fn test_resolve_at_uses_the_utc_date() {
        let map = rename();
        let new = Symbol::from("NEW");
        let canonical = InstrumentId::from("NEW.XNYS");

        assert_eq!(
            map.resolve_at(&new, date_to_nanos(date(2024, 6, 1))),
            Some(canonical)
        );
        assert_eq!(
            map.resolve_at(&Symbol::from("OLD"), date_to_nanos(date(2024, 6, 1))),
            None
        );
    }
}
