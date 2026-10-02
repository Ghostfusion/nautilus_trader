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

//! Python bindings for the moomoo data client factory.

use pyo3::prelude::*;

use crate::{common::CLIENT_ID, factories::MoomooDataClientFactory};

#[pymethods]
#[pyo3_stub_gen::derive::gen_stub_pymethods]
impl MoomooDataClientFactory {
    /// Factory for creating moomoo data clients.
    #[new]
    fn py_new() -> Self {
        Self
    }

    /// Returns the adapter identifier this factory registers under.
    ///
    /// The identifier is the provider rather than a venue, because one moomoo client serves the
    /// United States and Hong Kong markets.
    #[pyo3(name = "name")]
    fn py_name(&self) -> &str {
        CLIENT_ID
    }
}
