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

//! Configuration for the EODHD data client.

use nautilus_core::string::secret::SecretString;
use nautilus_model::types::Currency;
use serde::{Deserialize, Serialize};

use crate::common::{EODHD_DEFAULT_EXCHANGE, EODHD_DEFAULT_PRICE_PRECISION, EODHD_WS_BASE_URL};

/// The default poll interval, in seconds, for a live bar subscription.
pub const EODHD_DEFAULT_POLL_INTERVAL_SECS: u64 = 60;

/// The default number of days of history requested when a bar subscription starts.
pub const EODHD_DEFAULT_BACKFILL_DAYS: u32 = 5;

/// Configuration for the EODHD data client.
///
/// EODHD is an end-of-day and intraday vendor with no HTTP push channel, so the client streams
/// bars by polling. The poll interval and the backfill window are the two knobs that trade request
/// volume against how quickly a forming bar is observed.
#[derive(Debug, Clone, Serialize, Deserialize, bon::Builder)]
#[serde(default, deny_unknown_fields)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.adapters.eodhd", from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.adapters.eodhd")
)]
pub struct EodhdDataClientConfig {
    /// The EODHD API token.
    ///
    /// Falls back to the `EODHD_API_KEY` environment variable when not set.
    pub api_key: Option<SecretString>,
    /// The EODHD REST API base URL override.
    pub http_base_url: Option<SecretString>,
    /// Optional proxy URL for the HTTP client.
    pub proxy_url: Option<SecretString>,
    /// The EODHD exchange code whose instruments are loaded on connect.
    #[builder(default = EODHD_DEFAULT_EXCHANGE.to_string())]
    pub exchange: String,
    /// The exchanges whose daily bars are served from the bulk last-day endpoint.
    ///
    /// A daily bar for an instrument on one of these exchanges is polled with one request per
    /// exchange rather than one request per symbol, which is what keeps a large daily universe
    /// current. Subscribing still seeds `backfill_days` of history for the symbol itself, so
    /// only the ongoing polls are shared.
    ///
    /// Other intervals, and exchanges not listed here, are polled per symbol. An exchange code
    /// is one of those `/exchanges-list` returns, for example `US` or `LSE`.
    #[builder(default)]
    pub bulk_exchanges: Vec<String>,
    /// How often to poll for new bars, in seconds.
    #[builder(default = EODHD_DEFAULT_POLL_INTERVAL_SECS)]
    pub poll_interval_secs: u64,
    /// How many days of history to emit when a bar subscription starts.
    #[builder(default = EODHD_DEFAULT_BACKFILL_DAYS)]
    pub backfill_days: u32,
    /// The instrument price precision.
    #[builder(default = EODHD_DEFAULT_PRICE_PRECISION)]
    pub price_precision: u8,
    /// The instrument currency code, for example `USD`.
    pub currency: Option<String>,
    /// The EODHD streaming API base URL override.
    pub ws_base_url: Option<SecretString>,
    /// Whether trades and quotes stream from the WebSocket API instead of the REST API.
    ///
    /// The streaming channels carry undelayed data and require a streaming entitlement, which the
    /// public `demo` token carries and some paid plans do not. When enabled, a trade or quote
    /// subscription is served by the channel for the instrument's venue; when disabled, quotes
    /// come from the delayed REST endpoint and trades cannot be subscribed at all.
    #[builder(default = false)]
    pub streaming: bool,
    /// The HTTP request timeout in seconds.
    pub timeout_secs: Option<u64>,
    /// Whether to load the `exchange` instruments on connect.
    #[builder(default = true)]
    pub load_instruments: bool,
    /// Whether to emit the corporate actions that fall in the window of a bar request.
    ///
    /// A corporate action is not carried by a data client command: the engine publishes it on the
    /// instrument's corporate action topic, where a strategy receives it after
    /// `subscribe_corporate_actions`. Enabling this fetches the dividends and splits reported for
    /// an instrument, and emits the ones effective within the bars it was asked for, so an action
    /// arrives alongside the bars it adjusts.
    ///
    /// Each bar request and each bar subscription costs two further requests when this is on.
    #[builder(default = false)]
    pub load_corporate_actions: bool,
}

impl Default for EodhdDataClientConfig {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl EodhdDataClientConfig {
    /// Returns the resolved instrument currency, defaulting to USD.
    #[must_use]
    pub fn resolved_currency(&self) -> Currency {
        self.currency
            .as_deref()
            .map_or_else(Currency::USD, Currency::from)
    }

    /// Returns the resolved streaming API base URL.
    #[must_use]
    pub fn resolved_ws_base_url(&self) -> String {
        self.ws_base_url.as_ref().map_or_else(
            || EODHD_WS_BASE_URL.to_string(),
            |url| url.expose_secret().to_string(),
        )
    }

    /// Returns the resolved poll interval, with a floor of one second.
    #[must_use]
    pub fn resolved_poll_interval_secs(&self) -> u64 {
        self.poll_interval_secs.max(1)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    fn test_default_config_values() {
        let config = EodhdDataClientConfig::default();

        assert_eq!(config.exchange, "US");
        assert_eq!(config.poll_interval_secs, 60);
        assert_eq!(config.backfill_days, 5);
        assert_eq!(config.price_precision, 2);
        assert!(config.load_instruments);
        assert!(config.api_key.is_none());
    }

    #[rstest]
    fn test_resolved_currency_defaults_to_usd() {
        let config = EodhdDataClientConfig::default();

        assert_eq!(config.resolved_currency(), Currency::USD());
    }

    #[rstest]
    fn test_resolved_currency_parses_the_configured_code() {
        let config = EodhdDataClientConfig::builder()
            .currency("GBP".to_string())
            .build();

        assert_eq!(config.resolved_currency(), Currency::GBP());
    }

    #[rstest]
    fn test_resolved_poll_interval_has_a_floor_of_one_second() {
        let config = EodhdDataClientConfig::builder()
            .poll_interval_secs(0)
            .build();

        assert_eq!(config.resolved_poll_interval_secs(), 1);
    }

    #[rstest]
    fn test_config_round_trips_through_json() {
        let config = EodhdDataClientConfig::builder()
            .exchange("LSE".to_string())
            .poll_interval_secs(30)
            .build();

        let encoded = serde_json::to_string(&config).unwrap();
        let decoded: EodhdDataClientConfig = serde_json::from_str(&encoded).unwrap();

        assert_eq!(decoded.exchange, "LSE");
        assert_eq!(decoded.poll_interval_secs, 30);
    }

    #[rstest]
    fn test_config_rejects_an_unknown_field() {
        let result = serde_json::from_str::<EodhdDataClientConfig>(r#"{"nonsense": 1}"#);

        assert!(result.is_err());
    }
}
