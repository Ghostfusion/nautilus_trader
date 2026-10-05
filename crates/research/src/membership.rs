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

//! Universe membership history stored as data.
//!
//! A membership rule evaluated today can only see the universe as it is now, so it cannot
//! represent an instrument that has since been delisted. The stored series is therefore the
//! authority once written: the rule seeds it, and every later reader resolves membership from the
//! series rather than re-evaluating the rule.
//!
//! The series is persisted through the catalog's custom-data path, beside the market data it
//! describes. Each interval is one continuous spell of membership for one instrument, with an
//! entry instant and an optional exit instant.

use std::sync::Arc;

use nautilus_core::UnixNanos;
use nautilus_model::{
    custom_data,
    data::{CustomData, CustomDataTrait, Data, DataType},
    identifiers::InstrumentId,
};
use nautilus_persistence::backend::parquet::catalog::ParquetDataCatalog;
use nautilus_serialization::{arrow::custom::ensure_custom_data_registered, arrow_custom_data};

/// A stored membership interval for one instrument in one universe.
///
/// The interval is half-open: the instrument is a member from `ts_event` (its entry instant) up
/// to but not including `exited_at`. An open interval (`exited_at` is `None`) is still a member.
///
/// The entry instant is carried on `ts_event`, which the catalog indexes for range queries;
/// `ts_init` mirrors it so the record can be written and read back.
#[arrow_custom_data]
#[custom_data]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.research", from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.research")
)]
pub struct MembershipInterval {
    /// The universe identity this interval belongs to.
    pub universe: String,
    /// The source identity of the membership series (for example a rule name and version).
    pub source: String,
    /// The instrument that is a member over the interval.
    pub instrument_id: InstrumentId,
    /// The instant the instrument entered the universe.
    pub ts_event: UnixNanos,
    /// The instant the interval was initialized (equal to the entry instant).
    pub ts_init: UnixNanos,
    /// The instant the instrument left the universe, if it has left.
    pub exited_at: Option<UnixNanos>,
}

impl MembershipInterval {
    /// Creates a [`MembershipInterval`] from its entry instant, mirroring the entry on both
    /// `ts_event` and `ts_init`.
    #[must_use]
    pub const fn entry(
        universe: String,
        source: String,
        instrument_id: InstrumentId,
        entered_at: UnixNanos,
        exited_at: Option<UnixNanos>,
    ) -> Self {
        Self {
            universe,
            source,
            instrument_id,
            ts_event: entered_at,
            ts_init: entered_at,
            exited_at,
        }
    }

    /// Returns the entry instant of the interval.
    #[must_use]
    pub const fn entered_at(&self) -> UnixNanos {
        self.ts_event
    }

    /// Returns whether the instrument is a member at `ts`.
    ///
    /// The interval is half-open, so the instrument is a member at its entry instant and is no
    /// longer a member at its exit instant.
    #[must_use]
    pub fn covers(&self, ts: UnixNanos) -> bool {
        ts >= self.ts_event && self.exited_at.is_none_or(|exit| ts < exit)
    }
}

/// A rule-produced membership spell for one instrument, before it is stamped with the series'
/// universe and source identities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MembershipSpell {
    /// The instrument that is a member over the spell.
    pub instrument_id: InstrumentId,
    /// The instant the instrument entered the universe.
    pub entered_at: UnixNanos,
    /// The instant the instrument left the universe, if it has left.
    pub exited_at: Option<UnixNanos>,
}

/// A rule that produces membership spells for one universe.
///
/// A rule is only the seed of a stored [`MembershipSeries`]. Callers must prefer the persisted
/// series for point-in-time work, because a rule cannot see instruments that have since left the
/// universe.
pub trait MembershipRule {
    /// Returns the universe identity this rule computes membership for.
    fn universe(&self) -> &str;

    /// Returns the source identity of the series this rule produces.
    fn source(&self) -> &str;

    /// Evaluates the rule into membership spells.
    fn evaluate(&self) -> Vec<MembershipSpell>;
}

/// A stored membership history for one universe and source.
///
/// The series is the authority for point-in-time membership once it has been written; it is
/// persisted and read through the catalog's custom-data path.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.research", from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.research")
)]
pub struct MembershipSeries {
    universe: String,
    source: String,
    intervals: Vec<MembershipInterval>,
}

impl MembershipSeries {
    /// Creates an empty membership series for the given universe and source identities.
    #[must_use]
    pub fn new(universe: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            universe: universe.into(),
            source: source.into(),
            intervals: Vec::new(),
        }
    }

    /// Builds a series by stamping a rule's spells with the series' universe and source.
    #[must_use]
    pub fn from_rule<R>(rule: &R) -> Self
    where
        R: MembershipRule,
    {
        let universe = rule.universe().to_string();
        let source = rule.source().to_string();
        let intervals = rule
            .evaluate()
            .into_iter()
            .map(|spell| MembershipInterval {
                universe: universe.clone(),
                source: source.clone(),
                instrument_id: spell.instrument_id,
                ts_event: spell.entered_at,
                ts_init: spell.entered_at,
                exited_at: spell.exited_at,
            })
            .collect();

        Self {
            universe,
            source,
            intervals,
        }
    }

    /// Returns the universe identity of the series.
    #[must_use]
    pub fn universe(&self) -> &str {
        &self.universe
    }

    /// Returns the source identity of the series.
    #[must_use]
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Returns the stored membership intervals.
    #[must_use]
    pub fn intervals(&self) -> &[MembershipInterval] {
        &self.intervals
    }

    /// Appends a membership interval.
    ///
    /// The interval is assumed to carry this series' universe and source identities; use
    /// [`MembershipSeries::from_rule`] to stamp them from a rule.
    pub fn push(&mut self, interval: MembershipInterval) {
        self.intervals.push(interval);
    }

    /// Resolves the members that apply at `ts`.
    ///
    /// Returns the identifiers of every instrument whose stored interval covers `ts`, sorted and
    /// deduplicated for a deterministic order.
    #[must_use]
    pub fn members_at(&self, ts: UnixNanos) -> Vec<InstrumentId> {
        let mut ids: Vec<InstrumentId> = self
            .intervals
            .iter()
            .filter(|interval| interval.covers(ts))
            .map(|interval| interval.instrument_id)
            .collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    }

    /// Returns whether `instrument_id` is a member at `ts`.
    #[must_use]
    pub fn is_member_at(&self, instrument_id: InstrumentId, ts: UnixNanos) -> bool {
        self.intervals
            .iter()
            .any(|interval| interval.instrument_id == instrument_id && interval.covers(ts))
    }

    /// Returns every instrument that appears in the series, sorted and deduplicated.
    #[must_use]
    pub fn instruments(&self) -> Vec<InstrumentId> {
        let mut ids: Vec<InstrumentId> = self
            .intervals
            .iter()
            .map(|interval| interval.instrument_id)
            .collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    }

    /// Writes the series to the catalog under its universe identifier.
    ///
    /// # Errors
    ///
    /// Returns an error if the custom data type is not registered or the catalog write fails.
    pub fn write_to_catalog(&self, catalog: &ParquetDataCatalog) -> anyhow::Result<()> {
        if self.intervals.is_empty() {
            return Ok(());
        }

        ensure_custom_data_registered::<MembershipInterval>();

        let data_type = DataType::new(
            <MembershipInterval as CustomDataTrait>::type_name_static(),
            None,
            Some(self.universe.clone()),
        );

        let mut intervals = self.intervals.clone();
        intervals.sort_by_key(|interval| interval.ts_init);

        let data: Vec<CustomData> = intervals
            .into_iter()
            .map(|interval| CustomData::new(Arc::new(interval), data_type.clone()))
            .collect();

        catalog.write_custom_data_batch(data, None, None, Some(true))?;

        Ok(())
    }

    /// Reads a series for `universe` and `source` from the catalog.
    ///
    /// # Errors
    ///
    /// Returns an error if the custom data type is not registered or the catalog query fails.
    pub fn read_from_catalog(
        catalog: &mut ParquetDataCatalog,
        universe: &str,
        source: &str,
    ) -> anyhow::Result<Self> {
        ensure_custom_data_registered::<MembershipInterval>();

        let identifiers = vec![universe.to_string()];
        let rows = catalog.query_custom_data_dynamic(
            <MembershipInterval as CustomDataTrait>::type_name_static(),
            Some(&identifiers),
            None,
            None,
            None,
            None,
            true,
        )?;

        let mut intervals: Vec<MembershipInterval> = rows
            .into_iter()
            .filter_map(|data| match data {
                Data::Custom(custom) => custom
                    .data
                    .as_any()
                    .downcast_ref::<MembershipInterval>()
                    .filter(|interval| interval.source == source)
                    .cloned(),
                _ => None,
            })
            .collect();

        intervals.sort_by(|a, b| {
            a.instrument_id
                .cmp(&b.instrument_id)
                .then(a.ts_event.cmp(&b.ts_event))
        });

        Ok(Self {
            universe: universe.to_string(),
            source: source.to_string(),
            intervals,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instrument(value: &str) -> InstrumentId {
        InstrumentId::from(value)
    }

    #[test]
    fn interval_is_half_open() {
        let interval = MembershipInterval::entry(
            "u".to_string(),
            "s".to_string(),
            instrument("A.X"),
            UnixNanos::from(100),
            Some(UnixNanos::from(200)),
        );

        assert!(!interval.covers(UnixNanos::from(99)));
        assert!(interval.covers(UnixNanos::from(100)));
        assert!(interval.covers(UnixNanos::from(199)));
        assert!(!interval.covers(UnixNanos::from(200)));
    }

    #[test]
    fn members_at_resolves_point_in_time() {
        let mut series = MembershipSeries::new("u", "s");
        series.push(MembershipInterval::entry(
            "u".to_string(),
            "s".to_string(),
            instrument("A.X"),
            UnixNanos::from(100),
            None,
        ));
        series.push(MembershipInterval::entry(
            "u".to_string(),
            "s".to_string(),
            instrument("B.X"),
            UnixNanos::from(300),
            None,
        ));

        assert_eq!(
            series.members_at(UnixNanos::from(150)),
            vec![instrument("A.X")]
        );
        assert_eq!(
            series.members_at(UnixNanos::from(350)),
            vec![instrument("A.X"), instrument("B.X")],
        );
        assert!(series.is_member_at(instrument("A.X"), UnixNanos::from(150)));
        assert!(!series.is_member_at(instrument("B.X"), UnixNanos::from(150)));
    }
}
