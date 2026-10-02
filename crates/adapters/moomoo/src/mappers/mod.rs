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

//! Conversions from the gateway's Protobuf records to the domain model.
//!
//! Each module here is pure: it takes a decoded message and returns a domain value, owning no
//! socket and reading no clock of its own. That is what lets a mapping be tested against a recorded
//! payload with no gateway and no network, which is the only way most of these paths can be tested
//! at all.

pub mod bars;
pub mod instrument;
