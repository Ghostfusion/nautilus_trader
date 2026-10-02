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

//! Configuration for the moomoo data client.

use serde::{Deserialize, Serialize};

use crate::{
    common::Market,
    mappers::bars::{Adjustment, BarSession},
    providers::MAX_BOOK_DEPTH,
};

/// The gateway address used when neither the configuration nor the environment names one.
pub const DEFAULT_HOST: &str = "127.0.0.1";

/// The gateway port used when neither the configuration nor the environment names one.
///
/// This is the port the gateway's own launcher listens on, and the one the adapter's documentation
/// tells an operator to expect.
pub const DEFAULT_PORT: u16 = 11111;

/// The number of book levels served when the configuration does not name one.
///
/// Ten is what a consumer reading the top of book needs, and the venue's ceiling is sixty. A
/// subscription is not charged per level, so this is a choice about what a consumer is given rather
/// than about what it costs.
pub const DEFAULT_BOOK_DEPTH: usize = 10;

/// The environment variable naming the gateway address.
pub const HOST_ENV: &str = "MOOMOO_HOST";

/// The environment variable naming the gateway port.
pub const PORT_ENV: &str = "MOOMOO_PORT";

/// Configuration for the moomoo data client.
///
/// There is no API key: the gateway holds the login, and the adapter's only credential-like concern
/// is not leaking a remote gateway address into logs.
///
/// The client is multi-venue, so the markets it serves are configured rather than the venue being,
/// and the identifiers it produces are the market codes the other providers use for the same
/// securities.
#[derive(Debug, Clone, Serialize, Deserialize, bon::Builder)]
#[serde(default, deny_unknown_fields)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.adapters.moomoo", from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.adapters.moomoo")
)]
pub struct MoomooDataClientConfig {
    /// The gateway address, which falls back to `MOOMOO_HOST` and then to the loopback address.
    ///
    /// The gateway is normally on the same host as the adapter, and a remote one is reached through
    /// a tunnel. It is left as an option so that an unset configuration and an explicitly loopback
    /// one are not confused for each other.
    pub host: Option<String>,
    /// The gateway port, which falls back to `MOOMOO_PORT` and then to the standard port.
    pub port: Option<u16>,
    /// The markets whose instruments are loaded on connect.
    #[builder(default = Market::ALL.to_vec())]
    pub markets: Vec<Market>,
    /// The price adjustment applied to the bars this client serves.
    ///
    /// Raw is the default and not the venue's: the gateway's own default for a K-line push is a
    /// forward adjustment, and the corporate actions this adapter serves describe the raw series.
    #[builder(default)]
    pub adjustment: Adjustment,
    /// The session the bars this client serves cover.
    ///
    /// The venue never returns extended hours unless they are asked for, so this is a choice and
    /// not something the adapter can infer.
    #[builder(default)]
    pub session: BarSession,
    /// The number of book levels served to a book consumer.
    ///
    /// The venue's push carries its whole ladder whatever was asked for, so the depth is applied by
    /// the adapter. It is capped at the venue's own ceiling.
    #[builder(default = DEFAULT_BOOK_DEPTH)]
    pub book_depth: usize,
    /// Whether a trade subscription is honoured.
    ///
    /// Leaving this off makes a trade subscription fail with a message that names this setting,
    /// which is what an operator wants when the client is attached to a node for another purpose
    /// and a stray subscription would spend an allowance that is scarce.
    #[builder(default = true)]
    pub subscribe_trades: bool,
    /// Whether a quote subscription is honoured, and whether books are served.
    #[builder(default = true)]
    pub subscribe_quotes: bool,
    /// Whether a market's instruments are priced from a snapshot when its universe is loaded.
    ///
    /// A snapshot is the only record carrying the spread the price precision is derived from, so
    /// with this on the instruments of a market the venue can price arrive with an authoritative
    /// tick. It costs one request per four hundred securities, and the venue refuses a batch as a
    /// whole when one of its members cannot be priced: measured across the whole United States list,
    /// every batch was refused because the list carries over-the-counter codes, while every Hong
    /// Kong batch was served. Turning it off makes a large load one request per market and gives
    /// every instrument the market's fallback precision.
    #[builder(default = true)]
    pub snapshot_universe: bool,
    /// Whether the configured markets' instruments are loaded on connect.
    #[builder(default = true)]
    pub load_instruments: bool,
    /// The request timeout in seconds, which falls back to the transport's own default.
    pub timeout_secs: Option<u64>,
}

impl Default for MoomooDataClientConfig {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl MoomooDataClientConfig {
    /// Returns the gateway address, resolving the environment fallback.
    #[must_use]
    pub fn resolved_host(&self) -> String {
        self.host
            .clone()
            .or_else(|| std::env::var(HOST_ENV).ok())
            .filter(|host| !host.is_empty())
            .unwrap_or_else(|| DEFAULT_HOST.to_string())
    }

    /// Returns the gateway port, resolving the environment fallback.
    ///
    /// A port that is set but unreadable is ignored rather than refused, because refusing it would
    /// make a typo in the environment a client that cannot be constructed at all, when the gateway
    /// is in practice on the standard port.
    #[must_use]
    pub fn resolved_port(&self) -> u16 {
        self.port
            .or_else(|| {
                std::env::var(PORT_ENV)
                    .ok()
                    .and_then(|port| port.parse().ok())
            })
            .unwrap_or(DEFAULT_PORT)
    }

    /// Returns the book depth to serve, capped at the venue's own ceiling.
    #[must_use]
    pub fn resolved_book_depth(&self) -> usize {
        self.book_depth
            .clamp(1, usize::try_from(MAX_BOOK_DEPTH).unwrap_or(60))
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::common::Market;

    #[rstest]
    fn test_default_config_values() {
        let config = MoomooDataClientConfig::default();

        assert_eq!(config.markets, Market::ALL.to_vec());
        assert_eq!(config.adjustment, Adjustment::None);
        assert_eq!(config.session, BarSession::Regular);
        assert_eq!(config.book_depth, 10);
        assert!(config.subscribe_trades);
        assert!(config.subscribe_quotes);
        assert!(config.snapshot_universe);
        assert!(config.load_instruments);
        assert!(config.timeout_secs.is_none());
    }

    #[rstest]
    fn test_resolved_gateway_falls_back_to_the_loopback_defaults() {
        let config = MoomooDataClientConfig::default();

        assert_eq!(config.resolved_host(), DEFAULT_HOST);
        assert_eq!(config.resolved_port(), DEFAULT_PORT);
    }

    #[rstest]
    fn test_resolved_gateway_prefers_the_configuration() {
        let config = MoomooDataClientConfig::builder()
            .host("10.0.0.5".to_string())
            .port(22222)
            .build();

        assert_eq!(config.resolved_host(), "10.0.0.5");
        assert_eq!(config.resolved_port(), 22222);
    }

    #[rstest]
    fn test_an_empty_host_is_not_a_host() {
        let config = MoomooDataClientConfig::builder()
            .host(String::new())
            .build();

        assert_eq!(config.resolved_host(), DEFAULT_HOST);
    }

    #[rstest]
    #[case(0, 1)]
    #[case(10, 10)]
    #[case(60, 60)]
    #[case(500, 60)]
    fn test_book_depth_is_capped_at_the_venue_ceiling(
        #[case] depth: usize,
        #[case] expected: usize,
    ) {
        let config = MoomooDataClientConfig::builder().book_depth(depth).build();

        assert_eq!(config.resolved_book_depth(), expected);
    }

    #[rstest]
    fn test_config_round_trips_through_json() {
        let config = MoomooDataClientConfig::builder()
            .markets(vec![Market::Hk])
            .adjustment(Adjustment::Backward)
            .session(BarSession::Extended)
            .book_depth(20)
            .subscribe_trades(false)
            .build();

        let encoded = serde_json::to_string(&config).unwrap();
        let decoded: MoomooDataClientConfig = serde_json::from_str(&encoded).unwrap();

        assert_eq!(decoded.markets, vec![Market::Hk]);
        assert_eq!(decoded.adjustment, Adjustment::Backward);
        assert_eq!(decoded.session, BarSession::Extended);
        assert_eq!(decoded.book_depth, 20);
        assert!(!decoded.subscribe_trades);
    }

    #[rstest]
    fn test_config_reads_the_market_codes_it_writes() {
        let config: MoomooDataClientConfig =
            serde_json::from_str(r#"{"markets": ["HK"], "adjustment": "forward"}"#).unwrap();

        assert_eq!(config.markets, vec![Market::Hk]);
        assert_eq!(config.adjustment, Adjustment::Forward);
    }

    #[rstest]
    fn test_config_rejects_an_unknown_field() {
        let result = serde_json::from_str::<MoomooDataClientConfig>(r#"{"nonsense": 1}"#);

        assert!(result.is_err());
    }

    #[rstest]
    fn test_config_rejects_an_unknown_market() {
        let result = serde_json::from_str::<MoomooDataClientConfig>(r#"{"markets": ["ZZ"]}"#);

        assert!(result.is_err());
    }
}
