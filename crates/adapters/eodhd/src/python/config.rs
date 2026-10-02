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

//! Python bindings for the EODHD data client configuration.

use nautilus_core::string::secret::SecretString;
use pyo3::prelude::*;

use crate::config::EodhdDataClientConfig;

#[pymethods]
#[pyo3_stub_gen::derive::gen_stub_pymethods]
impl EodhdDataClientConfig {
    /// Configuration for the EODHD data client.
    ///
    /// Unset arguments take their documented default. The API token falls back to the
    /// `EODHD_API_KEY` environment variable when not supplied.
    #[new]
    #[pyo3(signature = (
        api_key = None,
        http_base_url = None,
        proxy_url = None,
        exchange = None,
        poll_interval_secs = None,
        backfill_days = None,
        price_precision = None,
        currency = None,
        timeout_secs = None,
        load_instruments = None,
        bulk_exchanges = None,
        load_corporate_actions = None,
        ws_base_url = None,
        streaming = None,
    ))]
    #[expect(clippy::too_many_arguments)]
    fn py_new(
        api_key: Option<String>,
        http_base_url: Option<String>,
        proxy_url: Option<String>,
        exchange: Option<String>,
        poll_interval_secs: Option<u64>,
        backfill_days: Option<u32>,
        price_precision: Option<u8>,
        currency: Option<String>,
        timeout_secs: Option<u64>,
        load_instruments: Option<bool>,
        bulk_exchanges: Option<Vec<String>>,
        load_corporate_actions: Option<bool>,
        ws_base_url: Option<String>,
        streaming: Option<bool>,
    ) -> Self {
        let defaults = Self::default();

        Self {
            api_key: api_key.map(SecretString::from),
            http_base_url: http_base_url.map(SecretString::from),
            proxy_url: proxy_url.map(SecretString::from),
            exchange: exchange.unwrap_or(defaults.exchange),
            poll_interval_secs: poll_interval_secs.unwrap_or(defaults.poll_interval_secs),
            backfill_days: backfill_days.unwrap_or(defaults.backfill_days),
            price_precision: price_precision.unwrap_or(defaults.price_precision),
            currency,
            ws_base_url: ws_base_url.map(SecretString::from),
            streaming: streaming.unwrap_or(defaults.streaming),
            timeout_secs,
            load_instruments: load_instruments.unwrap_or(defaults.load_instruments),
            bulk_exchanges: bulk_exchanges.unwrap_or(defaults.bulk_exchanges),
            load_corporate_actions: load_corporate_actions
                .unwrap_or(defaults.load_corporate_actions),
        }
    }

    /// Returns whether an API token is set on this configuration.
    #[getter]
    #[must_use]
    const fn has_api_key(&self) -> bool {
        self.api_key.is_some()
    }

    /// Returns the EODHD exchange code whose instruments are loaded on connect.
    #[getter]
    #[must_use]
    fn exchange(&self) -> &str {
        &self.exchange
    }

    /// Returns how often live subscriptions poll, in seconds.
    #[getter]
    #[must_use]
    const fn poll_interval_secs(&self) -> u64 {
        self.poll_interval_secs
    }

    /// Returns how many days of history a bar subscription emits when it starts.
    #[getter]
    #[must_use]
    const fn backfill_days(&self) -> u32 {
        self.backfill_days
    }

    /// Returns the instrument price precision.
    #[getter]
    #[must_use]
    const fn price_precision(&self) -> u8 {
        self.price_precision
    }

    /// Returns whether the `exchange` instruments are loaded on connect.
    #[getter]
    #[must_use]
    const fn load_instruments(&self) -> bool {
        self.load_instruments
    }

    /// Returns the exchanges whose daily bars are served from the bulk last-day endpoint.
    #[getter]
    #[must_use]
    fn bulk_exchanges(&self) -> Vec<String> {
        self.bulk_exchanges.clone()
    }

    /// Returns whether the corporate actions in a bar window are emitted.
    #[getter]
    #[must_use]
    const fn load_corporate_actions(&self) -> bool {
        self.load_corporate_actions
    }

    /// Returns whether trades and quotes stream from the WebSocket API.
    #[getter]
    #[must_use]
    const fn streaming(&self) -> bool {
        self.streaming
    }

    #[pyo3(name = "__repr__")]
    fn py_repr(&self) -> String {
        format!("{self:?}")
    }
}
