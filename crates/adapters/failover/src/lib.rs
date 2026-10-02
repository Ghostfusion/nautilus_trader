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

//! A priority chain across market data providers.
//!
//! Providers of the same data are chained rather than mixed: a demand is served by the first
//! provider that can answer it, and the next provider is asked only when the first cannot. This
//! crate is the part of that arrangement that is not a provider: what a failure means, whether it
//! is worth another attempt at the same provider, and when the demand moves to the next one.
//!
//! # Two decisions, kept apart
//!
//! A **retry** is for a transient condition on a provider that is otherwise the right one, and a
//! **hop** is a deliberate move to a different dataset. Collapsing them into one "try the next
//! thing" is what silently substitutes one provider's data for another's, so they are decided
//! separately and both are recorded in a [`chain::Trace`].
//!
//! # Bounded by the providers, not by an attempt count
//!
//! Each provider is retried at most once per demand and entered at most once, so a demand on a
//! chain of `n` providers costs at most `2n - 1` attempts and the chain stops when they are
//! exhausted. The budget is per demand, which is what keeps a failing provider from being asked
//! again on every tick.
//!
//! # What this crate is not
//!
//! It is not a health tracker and not a router. Whether a demand should *start* at the primary, and
//! whether it should stay where it last succeeded, are policies above it, and the pieces here are
//! deliberately the parts a policy needs rather than a policy.
//!
//! # NautilusTrader
//!
//! [NautilusTrader](https://nautilustrader.io) is an open-source, production-grade, Rust-native
//! engine for multi-asset, multi-venue trading systems.

#![warn(rustc::all)]
#![deny(unsafe_code)]
#![deny(nonstandard_style)]
#![deny(missing_debug_implementations)]
#![deny(clippy::missing_errors_doc)]
#![deny(clippy::missing_panics_doc)]
#![deny(rustdoc::broken_intra_doc_links)]

pub mod chain;
pub mod composite;
pub mod failure;
pub mod health;
pub mod pump;
pub mod streaming;
pub mod tap;
pub mod testing;
