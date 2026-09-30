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

//! Instrument universe membership values.
//!
//! A universe decides, at instants the clock drives, which instruments are in play. Membership is
//! an explicit per-instrument state rather than a flag: an instrument is added when selection
//! first includes it, becomes active once the run knows its definition, and leaves through a
//! removal process that can be held while the instrument still has open orders or a position.
//!
//! These are values, not component state. A membership change crosses component boundaries: it is
//! published on the message bus and delivered to subscribers, so it lives here rather than with the
//! component that produces it. The state machine is a pure function of the current and next state,
//! so it holds no clock and no runtime handle.

use std::fmt::Display;

use nautilus_core::UnixNanos;
use ustr::Ustr;

use crate::identifiers::InstrumentId;

/// The membership state of an instrument in a universe.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    strum::Display,
    strum::AsRefStr,
    strum::EnumString,
    strum::FromRepr,
    strum::EnumIter,
)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(
        frozen,
        eq,
        eq_int,
        module = "nautilus_trader.model",
        from_py_object,
        rename_all = "SCREAMING_SNAKE_CASE",
    )
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass_enum(module = "nautilus_trader.model")
)]
pub enum UniverseMembershipState {
    /// Selected: subscriptions are requested and the instrument definition is requested.
    Added = 1,
    /// Active: the instrument definition is known to the run and the member receives data.
    Active = 2,
    /// Removal has started. The subscriptions are still held until the removal completes.
    Removing = 3,
    /// Removed: the member's subscriptions are released.
    Removed = 4,
}

impl UniverseMembershipState {
    /// Returns the canonical string representation.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.as_ref()
    }

    /// Returns whether the state holds membership, i.e. the universe has claims on the instrument.
    #[must_use]
    pub const fn is_member(&self) -> bool {
        matches!(self, Self::Added | Self::Active | Self::Removing)
    }

    /// Returns whether the member is active.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(self, Self::Active)
    }

    /// Returns whether a transition to `next` is valid.
    ///
    /// `ADDED` becomes `ACTIVE` when the definition is known, or `REMOVING` when selection drops
    /// the instrument before its definition arrived. `ACTIVE` becomes `REMOVING`. A removal in
    /// progress is either completed (`REMOVED`) or cancelled by selection re-including the
    /// instrument (`ACTIVE`). A removed instrument can be added again later, which is a new
    /// membership. Self-transitions are not valid: a caller must check the current state first.
    #[must_use]
    pub const fn can_transition_to(&self, next: Self) -> bool {
        matches!(
            (self, next),
            // A definition arriving, or selection re-including an instrument whose removal began.
            (Self::Added | Self::Removing, Self::Active)
                // Selection excluding an instrument, whether its definition arrived or not.
                | (Self::Added | Self::Active, Self::Removing)
                // A removal completing, or an instrument rejoining after being removed.
                | (Self::Removing, Self::Removed)
                | (Self::Removed, Self::Added)
        )
    }
}

/// Why a membership changed.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    strum::Display,
    strum::AsRefStr,
    strum::EnumString,
    strum::FromRepr,
    strum::EnumIter,
)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(
        frozen,
        eq,
        eq_int,
        module = "nautilus_trader.model",
        from_py_object,
        rename_all = "SCREAMING_SNAKE_CASE",
    )
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass_enum(module = "nautilus_trader.model")
)]
pub enum UniverseChangeReason {
    /// Selection included the instrument.
    Selected = 1,
    /// Selection no longer includes the instrument.
    Deselected = 2,
    /// The universe API or a component changed membership explicitly.
    Explicit = 3,
}

impl UniverseChangeReason {
    /// Returns the canonical string representation.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.as_ref()
    }
}

/// A membership change of a universe.
///
/// The record is the payload of the universe's membership topic and the argument of the
/// `on_universe_changed` callback, so a subscriber decides from one value which instrument entered
/// which state, when, and why.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.model", from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.model")
)]
pub struct UniverseChange {
    /// The universe the change belongs to.
    pub universe: Ustr,
    /// The instrument whose membership changed.
    pub instrument_id: InstrumentId,
    /// The state the instrument entered.
    pub state: UniverseMembershipState,
    /// Why the membership changed.
    pub reason: UniverseChangeReason,
    /// The instant the change occurred.
    pub ts_event: UnixNanos,
    /// The instant the change was initialized.
    pub ts_init: UnixNanos,
}

impl UniverseChange {
    /// Creates a new [`UniverseChange`].
    #[must_use]
    pub const fn new(
        universe: Ustr,
        instrument_id: InstrumentId,
        state: UniverseMembershipState,
        reason: UniverseChangeReason,
        ts_event: UnixNanos,
        ts_init: UnixNanos,
    ) -> Self {
        Self {
            universe,
            instrument_id,
            state,
            reason,
            ts_event,
            ts_init,
        }
    }
}

impl Display for UniverseChange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "UNIVERSE {}: {} {} ({})",
            self.universe, self.state, self.instrument_id, self.reason
        )
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(UniverseMembershipState::Added, UniverseMembershipState::Active)]
    #[case(UniverseMembershipState::Added, UniverseMembershipState::Removing)]
    #[case(UniverseMembershipState::Active, UniverseMembershipState::Removing)]
    #[case(UniverseMembershipState::Removing, UniverseMembershipState::Active)]
    #[case(UniverseMembershipState::Removing, UniverseMembershipState::Removed)]
    #[case(UniverseMembershipState::Removed, UniverseMembershipState::Added)]
    fn test_valid_transitions(
        #[case] current: UniverseMembershipState,
        #[case] next: UniverseMembershipState,
    ) {
        assert!(current.can_transition_to(next));
    }

    #[rstest]
    #[case(UniverseMembershipState::Added, UniverseMembershipState::Added)]
    #[case(UniverseMembershipState::Added, UniverseMembershipState::Removed)]
    #[case(UniverseMembershipState::Active, UniverseMembershipState::Added)]
    #[case(UniverseMembershipState::Active, UniverseMembershipState::Active)]
    #[case(UniverseMembershipState::Active, UniverseMembershipState::Removed)]
    #[case(UniverseMembershipState::Removing, UniverseMembershipState::Added)]
    #[case(UniverseMembershipState::Removing, UniverseMembershipState::Removing)]
    #[case(UniverseMembershipState::Removed, UniverseMembershipState::Active)]
    #[case(UniverseMembershipState::Removed, UniverseMembershipState::Removing)]
    #[case(UniverseMembershipState::Removed, UniverseMembershipState::Removed)]
    fn test_invalid_transitions(
        #[case] current: UniverseMembershipState,
        #[case] next: UniverseMembershipState,
    ) {
        assert!(!current.can_transition_to(next));
    }

    #[rstest]
    #[case(UniverseMembershipState::Added, true)]
    #[case(UniverseMembershipState::Active, true)]
    #[case(UniverseMembershipState::Removing, true)]
    #[case(UniverseMembershipState::Removed, false)]
    fn test_is_member(#[case] state: UniverseMembershipState, #[case] expected: bool) {
        assert_eq!(state.is_member(), expected);
    }

    #[rstest]
    #[case(UniverseMembershipState::Added, false)]
    #[case(UniverseMembershipState::Active, true)]
    #[case(UniverseMembershipState::Removing, false)]
    #[case(UniverseMembershipState::Removed, false)]
    fn test_is_active(#[case] state: UniverseMembershipState, #[case] expected: bool) {
        assert_eq!(state.is_active(), expected);
    }

    #[rstest]
    #[case(UniverseMembershipState::Added, "ADDED")]
    #[case(UniverseMembershipState::Active, "ACTIVE")]
    #[case(UniverseMembershipState::Removing, "REMOVING")]
    #[case(UniverseMembershipState::Removed, "REMOVED")]
    fn test_state_round_trips_through_canonical_string(
        #[case] state: UniverseMembershipState,
        #[case] expected: &str,
    ) {
        assert_eq!(state.as_str(), expected);
        assert_eq!(state.to_string(), expected);
        assert_eq!(expected.parse::<UniverseMembershipState>().unwrap(), state);
    }

    #[rstest]
    #[case(UniverseChangeReason::Selected, "SELECTED")]
    #[case(UniverseChangeReason::Deselected, "DESELECTED")]
    #[case(UniverseChangeReason::Explicit, "EXPLICIT")]
    fn test_reason_round_trips_through_canonical_string(
        #[case] reason: UniverseChangeReason,
        #[case] expected: &str,
    ) {
        assert_eq!(reason.as_str(), expected);
        assert_eq!(reason.to_string(), expected);
        assert_eq!(expected.parse::<UniverseChangeReason>().unwrap(), reason);
    }

    #[rstest]
    fn test_change_display() {
        let change = UniverseChange::new(
            Ustr::from("equities"),
            InstrumentId::from("AAPL.XNYS"),
            UniverseMembershipState::Added,
            UniverseChangeReason::Selected,
            UnixNanos::from(1),
            UnixNanos::from(1),
        );

        assert_eq!(
            change.to_string(),
            "UNIVERSE equities: ADDED AAPL.XNYS (SELECTED)"
        );
    }
}
