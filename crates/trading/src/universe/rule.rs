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

//! Universe rules.
//!
//! A rule answers one question: which instruments are eligible at an instant. It is consulted only
//! from the universe's selection step, so it must be a function of the instant it is given and of
//! state it already holds. A rule never reads a wall clock and never issues a data request: the
//! universe owns the subscriptions and the metadata requests, and a rule that needs research or
//! venue data holds what it was given out of band.
//!
//! The universe canonicalises what a rule returns (sorted by instrument ID, without duplicates), so
//! two rules that describe the same set produce the same membership order.

use std::{cell::RefCell, fmt::Debug, rc::Rc};

use anyhow::{Result, ensure};
use nautilus_core::{UnixNanos, correctness::check_valid_string_utf8};
use nautilus_model::identifiers::InstrumentId;
use ustr::Ustr;

/// Decides which instruments are eligible at an instant.
pub trait UniverseRule: Debug {
    /// Returns the instruments eligible at `timestamp_ns`.
    ///
    /// # Errors
    ///
    /// Returns an error if the rule cannot evaluate eligibility.
    fn select(&mut self, timestamp_ns: UnixNanos) -> Result<Vec<InstrumentId>>;

    /// Returns the rule name, used for logging.
    fn name(&self) -> &str;
}

/// Wraps a rule for shared mutable ownership by a universe.
pub type SharedUniverseRule = Rc<RefCell<dyn UniverseRule>>;

/// A rule with the same membership set at every instant.
#[derive(Clone, Debug)]
pub struct StaticUniverseRule {
    name: Ustr,
    instruments: Vec<InstrumentId>,
}

impl StaticUniverseRule {
    /// Creates a new [`StaticUniverseRule`].
    ///
    /// The instruments are sorted and de-duplicated, so the rule is canonical for a given set.
    ///
    /// # Errors
    ///
    /// Returns an error if the name is empty.
    pub fn new(name: &str, instruments: Vec<InstrumentId>) -> Result<Self> {
        check_valid_string_utf8(name, stringify!(name))?;
        ensure!(!name.is_empty(), "Universe rule name must not be empty");

        Ok(Self {
            name: Ustr::from(name),
            instruments: canonicalise(instruments),
        })
    }

    /// Returns the instruments.
    #[must_use]
    pub fn instruments(&self) -> &[InstrumentId] {
        &self.instruments
    }
}

impl UniverseRule for StaticUniverseRule {
    fn select(&mut self, _timestamp_ns: UnixNanos) -> Result<Vec<InstrumentId>> {
        Ok(self.instruments.clone())
    }

    fn name(&self) -> &str {
        self.name.as_str()
    }
}

/// One scheduled membership set of a [`ScheduledUniverseRule`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduledUniverseSet {
    /// The instant the set takes effect.
    pub effective_ns: UnixNanos,
    /// The instruments eligible from `effective_ns` until the next set.
    pub instruments: Vec<InstrumentId>,
}

impl ScheduledUniverseSet {
    /// Creates a new [`ScheduledUniverseSet`].
    #[must_use]
    pub fn new(effective_ns: UnixNanos, instruments: Vec<InstrumentId>) -> Self {
        Self {
            effective_ns,
            instruments: canonicalise(instruments),
        }
    }
}

/// A rule whose membership is a declared schedule of sets.
///
/// Selection returns the set with the greatest effective instant at or before the instant it is
/// asked about, and nothing before the first set. A schedule is input data to a run, so the same
/// schedule always produces the same membership at the same instant.
#[derive(Clone, Debug)]
pub struct ScheduledUniverseRule {
    name: Ustr,
    sets: Vec<ScheduledUniverseSet>,
}

impl ScheduledUniverseRule {
    /// Creates a new [`ScheduledUniverseRule`], ordering the sets by effective instant.
    ///
    /// # Errors
    ///
    /// Returns an error if the name is empty or two sets share an effective instant.
    pub fn new(name: &str, mut sets: Vec<ScheduledUniverseSet>) -> Result<Self> {
        check_valid_string_utf8(name, stringify!(name))?;
        ensure!(!name.is_empty(), "Universe rule name must not be empty");

        sets.sort_by_key(|set| set.effective_ns);
        for pair in sets.windows(2) {
            ensure!(
                pair[0].effective_ns != pair[1].effective_ns,
                "Universe rule '{name}' has two sets effective at {}",
                pair[0].effective_ns,
            );
        }

        Ok(Self {
            name: Ustr::from(name),
            sets,
        })
    }

    /// Creates a new [`ScheduledUniverseRule`] from `(effective_ns, instruments)` pairs.
    ///
    /// # Errors
    ///
    /// Returns as [`Self::new`].
    pub fn from_schedule<I>(name: &str, sets: I) -> Result<Self>
    where
        I: IntoIterator<Item = (UnixNanos, Vec<InstrumentId>)>,
    {
        let sets = sets
            .into_iter()
            .map(|(effective_ns, instruments)| ScheduledUniverseSet::new(effective_ns, instruments))
            .collect();

        Self::new(name, sets)
    }

    /// Returns the scheduled sets, ordered by effective instant.
    #[must_use]
    pub fn sets(&self) -> &[ScheduledUniverseSet] {
        &self.sets
    }
}

impl UniverseRule for ScheduledUniverseRule {
    fn select(&mut self, timestamp_ns: UnixNanos) -> Result<Vec<InstrumentId>> {
        Ok(self
            .sets
            .iter()
            .rev()
            .find(|set| set.effective_ns <= timestamp_ns)
            .map(|set| set.instruments.clone())
            .unwrap_or_default())
    }

    fn name(&self) -> &str {
        self.name.as_str()
    }
}

/// Returns the instruments sorted by instrument ID without duplicates.
fn canonicalise(mut instruments: Vec<InstrumentId>) -> Vec<InstrumentId> {
    instruments.sort_unstable();
    instruments.dedup();
    instruments
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    fn instrument_id(symbol: &str) -> InstrumentId {
        InstrumentId::from(symbol)
    }

    fn ids(symbols: &[&str]) -> Vec<InstrumentId> {
        symbols.iter().map(|symbol| instrument_id(symbol)).collect()
    }

    #[rstest]
    fn test_static_rule_returns_its_set_at_every_instant() {
        let mut rule =
            StaticUniverseRule::new("equities", ids(&["MSFT.XNYS", "AAPL.XNYS"])).unwrap();

        assert_eq!(rule.name(), "equities");
        assert_eq!(
            rule.instruments(),
            ids(&["AAPL.XNYS", "MSFT.XNYS"]).as_slice()
        );

        let selected = rule.select(UnixNanos::from(1)).unwrap();
        assert_eq!(selected, ids(&["AAPL.XNYS", "MSFT.XNYS"]));
        assert_eq!(rule.select(UnixNanos::from(1_000)).unwrap(), selected);
    }

    #[rstest]
    fn test_static_rule_canonicalises_and_deduplicates() {
        let rule = StaticUniverseRule::new(
            "equities",
            ids(&["MSFT.XNYS", "AAPL.XNYS", "MSFT.XNYS", "AAPL.XNYS"]),
        )
        .unwrap();

        assert_eq!(
            rule.instruments(),
            ids(&["AAPL.XNYS", "MSFT.XNYS"]).as_slice()
        );
    }

    #[rstest]
    fn test_static_rule_rejects_empty_name() {
        assert!(StaticUniverseRule::new("", Vec::new()).is_err());
    }

    #[rstest]
    #[case(0, &[])]
    #[case(1, &[])]
    #[case(99, &[])]
    #[case(100, &["AAPL.XNYS"])]
    #[case(150, &["AAPL.XNYS"])]
    #[case(200, &["AAPL.XNYS", "MSFT.XNYS"])]
    #[case(1_000, &["AAPL.XNYS", "MSFT.XNYS"])]
    fn test_scheduled_rule_selects_the_effective_set(
        #[case] timestamp_ns: u64,
        #[case] expected: &[&str],
    ) {
        let mut rule = ScheduledUniverseRule::from_schedule(
            "equities",
            [
                (UnixNanos::from(100), ids(&["AAPL.XNYS"])),
                (UnixNanos::from(200), ids(&["AAPL.XNYS", "MSFT.XNYS"])),
            ],
        )
        .unwrap();

        assert_eq!(rule.name(), "equities");
        assert_eq!(
            rule.select(UnixNanos::from(timestamp_ns)).unwrap(),
            ids(expected)
        );
    }

    #[rstest]
    fn test_scheduled_rule_orders_sets_by_effective_instant() {
        let rule = ScheduledUniverseRule::from_schedule(
            "equities",
            [
                (UnixNanos::from(200), ids(&["MSFT.XNYS"])),
                (UnixNanos::from(100), ids(&["AAPL.XNYS"])),
            ],
        )
        .unwrap();

        assert_eq!(rule.sets()[0].effective_ns, UnixNanos::from(100));
        assert_eq!(rule.sets()[1].effective_ns, UnixNanos::from(200));
    }

    #[rstest]
    fn test_scheduled_rule_rejects_duplicate_effective_instants() {
        let error = ScheduledUniverseRule::from_schedule(
            "equities",
            [
                (UnixNanos::from(100), ids(&["AAPL.XNYS"])),
                (UnixNanos::from(100), ids(&["MSFT.XNYS"])),
            ],
        )
        .unwrap_err();

        assert!(error.to_string().contains("two sets effective at"));
    }

    #[rstest]
    fn test_scheduled_rule_with_empty_set_removes_everything() {
        let mut rule = ScheduledUniverseRule::from_schedule(
            "equities",
            [
                (UnixNanos::from(100), ids(&["AAPL.XNYS"])),
                (UnixNanos::from(200), Vec::new()),
            ],
        )
        .unwrap();

        assert_eq!(
            rule.select(UnixNanos::from(100)).unwrap(),
            ids(&["AAPL.XNYS"])
        );
        assert!(rule.select(UnixNanos::from(200)).unwrap().is_empty());
    }

    #[rstest]
    fn test_rules_are_shared_mutable() {
        let rule: SharedUniverseRule = Rc::new(RefCell::new(
            StaticUniverseRule::new("equities", Vec::new()).unwrap(),
        ));

        assert!(
            rule.borrow_mut()
                .select(UnixNanos::from(1))
                .unwrap()
                .is_empty()
        );
        assert_eq!(rule.borrow().name(), "equities");
    }
}
