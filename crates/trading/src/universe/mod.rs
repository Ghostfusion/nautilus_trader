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

//! Instrument universes: definition, clock-driven selection, and membership.
//!
//! An instrument is static in the kernel: it is added to a run up front and its data subscriptions
//! are declared by the component that consumes the data. A universe makes membership a runtime
//! property instead, in three separate parts with distinct ownership:
//!
//! - **Definition** ([`UniverseDefinition`]): the rule and settings describing what is eligible,
//!   plus the subscriptions a member holds.
//! - **Selection** ([`Universe::select`]): the scheduled evaluation that decides membership. It
//!   takes the instant it is evaluated at, so it is driven by the clock and never by a wall clock.
//! - **Membership** ([`UniverseMembershipState`]): explicit per-instrument state, with removal as a
//!   process rather than an immediate unsubscribe.
//!
//! The component is an actor: it owns its own data subscriptions through the existing data command
//! path, so a departing instrument releases only the claims the universe holds. It registers with a
//! trader like any other actor and is inert until it is constructed and started.
//!
//! # Removal
//!
//! Membership leaves through `REMOVING`. While in that state the universe still holds the member's
//! subscriptions, reports open orders and positions, and completes the removal only when
//! [`UniverseRemovalPolicy`] is satisfied. The universe never submits or cancels orders: it reports
//! the condition and the component that owns the orders decides what to do about them.

mod component;
mod definition;
mod membership;
mod rule;

#[cfg(test)]
mod tests;

pub use component::Universe;
pub use definition::{
    UniverseBarSpec, UniverseDefinition, UniverseRemovalPolicy, UniverseSubscription,
};
pub use membership::UniverseMember;
pub use nautilus_model::universe::{UniverseChange, UniverseChangeReason, UniverseMembershipState};
pub use rule::{
    ScheduledUniverseRule, ScheduledUniverseSet, SharedUniverseRule, StaticUniverseRule,
    UniverseRule,
};
