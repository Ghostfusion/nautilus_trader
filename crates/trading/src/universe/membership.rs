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

//! Component-side universe membership records.
//!
//! The membership values themselves ([`UniverseMembershipState`], [`UniverseChangeReason`], and
//! the change record) live in `nautilus_model`, because a membership change crosses component
//! boundaries. This module holds what only the universe component needs: the record it keeps for
//! each instrument, and validation of the universe name that identifies the component.

use nautilus_core::{UnixNanos, correctness::check_valid_string_utf8};
use nautilus_model::{
    identifiers::InstrumentId,
    universe::{UniverseChangeReason, UniverseMembershipState},
};
use ustr::Ustr;

/// A member of a universe.
///
/// `added_ns` is when the current membership began, `updated_ns` when the state last changed, and
/// `removal_blocked` whether the last removal evaluation reported a held removal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UniverseMember {
    /// The member instrument.
    pub instrument_id: InstrumentId,
    /// The membership state.
    pub state: UniverseMembershipState,
    /// When the current membership began.
    pub added_ns: UnixNanos,
    /// When the state last changed.
    pub updated_ns: UnixNanos,
    /// Whether a removal is currently held by the removal policy.
    pub removal_blocked: bool,
    /// Why the current removal began, while one is in progress.
    ///
    /// Set by the universe when membership enters `REMOVING`, and carried into the `REMOVED`
    /// change. `None` while no removal is in progress.
    pub removal_reason: Option<UniverseChangeReason>,
}

impl UniverseMember {
    /// Creates a new [`UniverseMember`].
    #[must_use]
    pub const fn new(
        instrument_id: InstrumentId,
        state: UniverseMembershipState,
        added_ns: UnixNanos,
        updated_ns: UnixNanos,
    ) -> Self {
        Self {
            instrument_id,
            state,
            added_ns,
            updated_ns,
            removal_blocked: false,
            removal_reason: None,
        }
    }

    /// Returns whether this member holds membership.
    #[must_use]
    pub const fn is_member(&self) -> bool {
        self.state.is_member()
    }

    /// Transitions the member to `next` at `ts_ns`.
    ///
    /// # Errors
    ///
    /// Returns an error if the transition is not valid from the current state.
    pub fn transition(
        &mut self,
        next: UniverseMembershipState,
        ts_ns: UnixNanos,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.state.can_transition_to(next),
            "Invalid universe membership transition for {}: {} -> {}",
            self.instrument_id,
            self.state,
            next,
        );

        if next == UniverseMembershipState::Added {
            self.added_ns = ts_ns;
        }

        if next == UniverseMembershipState::Removed {
            self.removal_blocked = false;
            self.removal_reason = None;
        }

        self.state = next;
        self.updated_ns = ts_ns;
        Ok(())
    }
}

/// Validates a universe name and returns it interned.
///
/// The name identifies the component, its membership topic, and its selection timer, so it is
/// restricted to characters that need no escaping in any of them.
///
/// # Errors
///
/// Returns an error if the name is empty or contains a character outside `[A-Za-z0-9_-]`.
pub(crate) fn validate_universe_name(name: &str) -> anyhow::Result<Ustr> {
    check_valid_string_utf8(name, stringify!(name))?;
    anyhow::ensure!(!name.is_empty(), "Universe name must not be empty");
    anyhow::ensure!(
        name.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
        "Universe name '{name}' must contain only ASCII alphanumeric characters, '-' or '_'",
    );
    Ok(Ustr::from(name))
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    fn instrument_id(symbol: &str) -> InstrumentId {
        InstrumentId::from(symbol)
    }

    #[rstest]
    fn test_member_transition_sets_timestamps() {
        let instrument_id = instrument_id("AAPL.XNYS");
        let mut member = UniverseMember::new(
            instrument_id,
            UniverseMembershipState::Added,
            UnixNanos::from(1),
            UnixNanos::from(1),
        );

        member
            .transition(UniverseMembershipState::Active, UnixNanos::from(2))
            .unwrap();
        assert_eq!(member.state, UniverseMembershipState::Active);
        assert_eq!(member.added_ns, UnixNanos::from(1));
        assert_eq!(member.updated_ns, UnixNanos::from(2));

        member
            .transition(UniverseMembershipState::Removing, UnixNanos::from(3))
            .unwrap();
        member.removal_blocked = true;
        member.removal_reason = Some(UniverseChangeReason::Deselected);

        member
            .transition(UniverseMembershipState::Removed, UnixNanos::from(4))
            .unwrap();
        assert_eq!(member.state, UniverseMembershipState::Removed);
        assert!(!member.removal_blocked);
        assert_eq!(member.removal_reason, None);
        assert_eq!(member.added_ns, UnixNanos::from(1));
        assert_eq!(member.updated_ns, UnixNanos::from(4));
    }

    #[rstest]
    fn test_member_rejoining_resets_added_ns() {
        let mut member = UniverseMember::new(
            instrument_id("AAPL.XNYS"),
            UniverseMembershipState::Removed,
            UnixNanos::from(1),
            UnixNanos::from(4),
        );

        member
            .transition(UniverseMembershipState::Added, UnixNanos::from(10))
            .unwrap();
        assert_eq!(member.added_ns, UnixNanos::from(10));
        assert_eq!(member.updated_ns, UnixNanos::from(10));
        assert!(member.is_member());
    }

    #[rstest]
    fn test_member_rejects_invalid_transition() {
        let mut member = UniverseMember::new(
            instrument_id("AAPL.XNYS"),
            UniverseMembershipState::Active,
            UnixNanos::from(1),
            UnixNanos::from(1),
        );

        let error = member
            .transition(UniverseMembershipState::Removed, UnixNanos::from(2))
            .unwrap_err();
        assert!(error.to_string().contains("ACTIVE -> REMOVED"));
        assert_eq!(member.state, UniverseMembershipState::Active);
        assert_eq!(member.updated_ns, UnixNanos::from(1));
    }

    #[rstest]
    #[case("equities", true)]
    #[case("XNYS-EQUITY_2026", true)]
    #[case("", false)]
    #[case("equities.xnys", false)]
    #[case("universe*", false)]
    #[case("equities xnys", false)]
    fn test_validate_universe_name(#[case] name: &str, #[case] valid: bool) {
        assert_eq!(validate_universe_name(name).is_ok(), valid);
    }
}
