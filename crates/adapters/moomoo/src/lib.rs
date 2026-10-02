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

//! [NautilusTrader](https://nautilustrader.io) adapter for the
//! [moomoo](https://www.moomoo.com) OpenD gateway.
//!
//! The `nautilus-moomoo` crate speaks the gateway's own frame protocol directly, over a local
//! OpenD instance. The vendor's Python client is not used: this repository's adapter namespace is
//! Rust-backed, so the crate implements the wire protocol against the schema the gateway client
//! ships.
//!
//! The gateway is push-driven. A subscription is a resource on the venue, held against a shared
//! allowance and not released within a minute of being taken, so subscription lifetime is a
//! subsystem of this adapter rather than a call site.
//!
//! # NautilusTrader
//!
//! [NautilusTrader](https://nautilustrader.io) is an open-source, production-grade, Rust-native
//! engine for multi-asset, multi-venue trading systems.
//!
//! # Feature Flags
//!
//! This crate provides feature flags to control source code inclusion during compilation,
//! depending on the intended use case:
//!
//! - `extension-module`: Builds as a Python extension module.
//! - `high-precision` (default): Enables
//!   [high-precision mode](https://nautilustrader.io/docs/nightly/getting_started/installation/#precision-mode)
//!   to use 128-bit value types.
//! - `python`: Enables Python bindings from [PyO3](https://pyo3.rs).
//!
//! # Scope
//!
//! This crate provides market data for the equities of the markets it serves: instruments, bars,
//! trades, quotes, order books, and corporate actions. Execution is out of scope.
//!
//! The crate is layered rather than monolithic. The frame codec and the vendored schema's generated
//! types depend on nothing local; the connection is built on those; the subscription manager, the
//! mappers, and the data client are built on the connection. Only the last of those needs a gateway,
//! and the codec and the mappers can be exercised without one.

#![warn(rustc::all)]
#![deny(unsafe_code)]
#![deny(nonstandard_style)]
#![deny(missing_debug_implementations)]
#![deny(clippy::missing_errors_doc)]
#![deny(clippy::missing_panics_doc)]
#![deny(rustdoc::broken_intra_doc_links)]

pub mod codec;
pub mod common;
pub mod config;
pub mod connection;
pub mod data;
pub mod entitlement;
pub mod factories;
pub mod loader;
pub mod mappers;
pub mod providers;
pub mod recovery;
pub mod subscription;

#[cfg(feature = "python")]
pub mod python;

/// The generated types for the vendored gateway schema.
///
/// prost lowercases each proto package name, so the module names are idiomatic Rust, and each
/// package becomes its own module because the request and response messages repeat across them.
pub mod generated;
