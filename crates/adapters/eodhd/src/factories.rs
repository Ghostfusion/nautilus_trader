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

//! Factory functions for creating EODHD clients.

use std::{any::Any, cell::RefCell, rc::Rc};

use nautilus_common::{
    cache::CacheView,
    clients::DataClient,
    clock::Clock,
    factories::{ClientConfig, DataClientFactory},
};
use nautilus_model::identifiers::ClientId;

use crate::{common::EODHD, config::EodhdDataClientConfig, data::EodhdDataClient};

impl ClientConfig for EodhdDataClientConfig {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Factory for creating EODHD data clients.
#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.adapters.eodhd", from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.adapters.eodhd")
)]
pub struct EodhdDataClientFactory;

impl EodhdDataClientFactory {
    /// Creates a new [`EodhdDataClientFactory`] instance.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for EodhdDataClientFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl DataClientFactory for EodhdDataClientFactory {
    fn create(
        &self,
        name: &str,
        config: &dyn ClientConfig,
        _cache: CacheView,
        _clock: Rc<RefCell<dyn Clock>>,
    ) -> anyhow::Result<Box<dyn DataClient>> {
        let eodhd_config = config
            .as_any()
            .downcast_ref::<EodhdDataClientConfig>()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Invalid config type for EodhdDataClientFactory. \
                     Expected EodhdDataClientConfig, was {config:?}",
                )
            })?
            .clone();

        let client_id = ClientId::from(name);
        let client = EodhdDataClient::new(client_id, eodhd_config)?;

        Ok(Box::new(client))
    }

    fn name(&self) -> &'static str {
        EODHD
    }

    fn config_type(&self) -> &'static str {
        "EodhdDataClientConfig"
    }
}

#[cfg(test)]
mod tests {
    use nautilus_common::factories::DataClientFactory;
    use rstest::rstest;

    use super::*;

    #[rstest]
    fn test_factory_name_and_config_type() {
        let factory = EodhdDataClientFactory::new();

        assert_eq!(factory.name(), "EODHD");
        assert_eq!(factory.config_type(), "EodhdDataClientConfig");
    }

    #[rstest]
    fn test_config_downcasts_from_the_client_config_trait() {
        let config = EodhdDataClientConfig::default();
        let erased: &dyn ClientConfig = &config;

        assert!(
            erased
                .as_any()
                .downcast_ref::<EodhdDataClientConfig>()
                .is_some()
        );
    }
}
