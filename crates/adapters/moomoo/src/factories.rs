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

//! Factory functions for creating moomoo clients.

use std::{any::Any, cell::RefCell, rc::Rc};

use nautilus_common::{
    cache::CacheView,
    clients::DataClient,
    clock::Clock,
    factories::{ClientConfig, DataClientFactory},
};
use nautilus_model::identifiers::ClientId;

use crate::{common::CLIENT_ID, config::MoomooDataClientConfig, data::MoomooDataClient};

impl ClientConfig for MoomooDataClientConfig {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Factory for creating moomoo data clients.
#[derive(Debug, Clone)]
pub struct MoomooDataClientFactory;

impl MoomooDataClientFactory {
    /// Creates a new [`MoomooDataClientFactory`] instance.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl Default for MoomooDataClientFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl DataClientFactory for MoomooDataClientFactory {
    fn create(
        &self,
        name: &str,
        config: &dyn ClientConfig,
        _cache: CacheView,
        _clock: Rc<RefCell<dyn Clock>>,
    ) -> anyhow::Result<Box<dyn DataClient>> {
        let moomoo_config = config
            .as_any()
            .downcast_ref::<MoomooDataClientConfig>()
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Invalid config type for MoomooDataClientFactory. \
                     Expected MoomooDataClientConfig, was {config:?}",
                )
            })?
            .clone();

        let client_id = ClientId::from(name);
        let client = MoomooDataClient::new(client_id, moomoo_config)?;

        Ok(Box::new(client))
    }

    fn name(&self) -> &'static str {
        CLIENT_ID
    }

    fn config_type(&self) -> &'static str {
        "MoomooDataClientConfig"
    }
}

#[cfg(test)]
mod tests {
    use nautilus_common::factories::DataClientFactory;
    use rstest::rstest;

    use super::*;

    #[rstest]
    fn test_factory_name_and_config_type() {
        let factory = MoomooDataClientFactory::new();

        // The name is the provider, not the venue: this client serves two markets.
        assert_eq!(factory.name(), "MOOMOO");
        assert_eq!(factory.config_type(), "MoomooDataClientConfig");
    }

    #[rstest]
    fn test_config_downcasts_from_the_client_config_trait() {
        let config = MoomooDataClientConfig::default();
        let erased: &dyn ClientConfig = &config;

        assert!(
            erased
                .as_any()
                .downcast_ref::<MoomooDataClientConfig>()
                .is_some()
        );
    }
}
