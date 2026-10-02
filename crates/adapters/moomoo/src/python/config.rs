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

//! Python bindings for the moomoo data client configuration.
//!
//! The market, adjustment and session arguments are named by the same words the configuration uses
//! in JSON, so that an adapter configured from a file and one configured from Python are configured
//! the same way. A word that names nothing is refused with the words that do, rather than being
//! silently taken as a default.

use nautilus_core::python::to_pyvalue_err;
use pyo3::prelude::*;

use crate::{
    common::Market,
    config::MoomooDataClientConfig,
    mappers::bars::{Adjustment, BarSession},
};

/// Returns the word the configuration uses for an adjustment.
fn adjustment_name(adjustment: Adjustment) -> &'static str {
    match adjustment {
        Adjustment::None => "none",
        Adjustment::Forward => "forward",
        Adjustment::Backward => "backward",
    }
}

/// Returns the adjustment a word names.
fn adjustment_from_name(name: &str) -> PyResult<Adjustment> {
    match name {
        "none" => Ok(Adjustment::None),
        "forward" => Ok(Adjustment::Forward),
        "backward" => Ok(Adjustment::Backward),
        other => Err(to_pyvalue_err(format!(
            "unknown adjustment {other:?}, expected one of \"none\", \"forward\" or \"backward\""
        ))),
    }
}

/// Returns the word the configuration uses for a session.
fn session_name(session: BarSession) -> &'static str {
    match session {
        BarSession::Regular => "regular",
        BarSession::Extended => "extended",
        BarSession::All => "all",
        BarSession::Overnight => "overnight",
    }
}

/// Returns the session a word names.
fn session_from_name(name: &str) -> PyResult<BarSession> {
    match name {
        "regular" => Ok(BarSession::Regular),
        "extended" => Ok(BarSession::Extended),
        "all" => Ok(BarSession::All),
        "overnight" => Ok(BarSession::Overnight),
        other => Err(to_pyvalue_err(format!(
            "unknown session {other:?}, expected one of \"regular\", \"extended\", \"all\" or \
             \"overnight\""
        ))),
    }
}

/// Returns the markets a list of codes names.
fn markets_from_codes(codes: &[String]) -> PyResult<Vec<Market>> {
    codes
        .iter()
        .map(|code| {
            Market::from_code(code).ok_or_else(|| {
                to_pyvalue_err(format!(
                    "unknown market {code:?}, expected one of \"US\" or \"HK\""
                ))
            })
        })
        .collect()
}

#[pymethods]
#[pyo3_stub_gen::derive::gen_stub_pymethods]
impl MoomooDataClientConfig {
    /// Configuration for the moomoo data client.
    ///
    /// Unset arguments take their documented default. The gateway address and port fall back to the
    /// `MOOMOO_HOST` and `MOOMOO_PORT` environment variables when not supplied, and then to the
    /// loopback address and the gateway's standard port. There is no API key: the gateway holds the
    /// login.
    #[new]
    #[pyo3(signature = (
        host = None,
        port = None,
        markets = None,
        adjustment = None,
        session = None,
        book_depth = None,
        subscribe_trades = None,
        subscribe_quotes = None,
        snapshot_universe = None,
        load_instruments = None,
        timeout_secs = None,
    ))]
    #[expect(clippy::too_many_arguments)]
    fn py_new(
        host: Option<String>,
        port: Option<u16>,
        markets: Option<Vec<String>>,
        adjustment: Option<String>,
        session: Option<String>,
        book_depth: Option<usize>,
        subscribe_trades: Option<bool>,
        subscribe_quotes: Option<bool>,
        snapshot_universe: Option<bool>,
        load_instruments: Option<bool>,
        timeout_secs: Option<u64>,
    ) -> PyResult<Self> {
        let defaults = Self::default();

        Ok(Self {
            host,
            port,
            markets: match markets {
                Some(codes) => markets_from_codes(&codes)?,
                None => defaults.markets,
            },
            adjustment: match adjustment {
                Some(name) => adjustment_from_name(&name)?,
                None => defaults.adjustment,
            },
            session: match session {
                Some(name) => session_from_name(&name)?,
                None => defaults.session,
            },
            book_depth: book_depth.unwrap_or(defaults.book_depth),
            subscribe_trades: subscribe_trades.unwrap_or(defaults.subscribe_trades),
            subscribe_quotes: subscribe_quotes.unwrap_or(defaults.subscribe_quotes),
            snapshot_universe: snapshot_universe.unwrap_or(defaults.snapshot_universe),
            load_instruments: load_instruments.unwrap_or(defaults.load_instruments),
            timeout_secs,
        })
    }

    /// Returns the configured gateway address, if one was set.
    #[getter]
    #[must_use]
    fn host(&self) -> Option<String> {
        self.host.clone()
    }

    /// Returns the configured gateway port, if one was set.
    #[getter]
    #[must_use]
    const fn port(&self) -> Option<u16> {
        self.port
    }

    /// Returns the gateway address actually used, with the environment fallback resolved.
    #[getter(resolved_host)]
    #[must_use]
    fn py_resolved_host(&self) -> String {
        Self::resolved_host(self)
    }

    /// Returns the gateway port actually used, with the environment fallback resolved.
    #[getter(resolved_port)]
    #[must_use]
    fn py_resolved_port(&self) -> u16 {
        Self::resolved_port(self)
    }

    /// Returns the market codes whose instruments are loaded on connect.
    #[getter]
    #[must_use]
    fn markets(&self) -> Vec<String> {
        self.markets
            .iter()
            .map(|market| market.code().to_string())
            .collect()
    }

    /// Returns the bar adjustment, as the word the configuration uses for it.
    #[getter]
    #[must_use]
    fn adjustment(&self) -> &'static str {
        adjustment_name(self.adjustment)
    }

    /// Returns the session the bars cover, as the word the configuration uses for it.
    #[getter]
    #[must_use]
    fn session(&self) -> &'static str {
        session_name(self.session)
    }

    /// Returns the number of book levels served.
    #[getter]
    #[must_use]
    const fn book_depth(&self) -> usize {
        self.book_depth
    }

    /// Returns whether trade subscriptions are honoured.
    #[getter]
    #[must_use]
    const fn subscribe_trades(&self) -> bool {
        self.subscribe_trades
    }

    /// Returns whether quote and book subscriptions are honoured.
    #[getter]
    #[must_use]
    const fn subscribe_quotes(&self) -> bool {
        self.subscribe_quotes
    }

    /// Returns whether a market's instruments are priced from a snapshot when it is loaded.
    #[getter]
    #[must_use]
    const fn snapshot_universe(&self) -> bool {
        self.snapshot_universe
    }

    /// Returns whether the configured markets are loaded on connect.
    #[getter]
    #[must_use]
    const fn load_instruments(&self) -> bool {
        self.load_instruments
    }

    /// Returns the request timeout in seconds, if one was set.
    #[getter]
    #[must_use]
    const fn timeout_secs(&self) -> Option<u64> {
        self.timeout_secs
    }

    #[pyo3(name = "__repr__")]
    fn py_repr(&self) -> String {
        format!("{self:?}")
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    /// The words Python accepts are the words the configuration serializes to, so that a
    /// configuration written in JSON and one written in Python cannot mean different things.
    #[rstest]
    fn test_the_python_words_are_the_serialized_words() {
        for adjustment in [Adjustment::None, Adjustment::Forward, Adjustment::Backward] {
            let serialized = serde_json::to_value(adjustment).unwrap();

            assert_eq!(serialized, adjustment_name(adjustment));
            assert_eq!(
                adjustment_from_name(adjustment_name(adjustment)).unwrap(),
                adjustment
            );
        }

        for session in [
            BarSession::Regular,
            BarSession::Extended,
            BarSession::All,
            BarSession::Overnight,
        ] {
            let serialized = serde_json::to_value(session).unwrap();

            assert_eq!(serialized, session_name(session));
            assert_eq!(session_from_name(session_name(session)).unwrap(), session);
        }

        for market in Market::ALL {
            let serialized = serde_json::to_value(market).unwrap();

            assert_eq!(serialized, market.code());
        }
    }

    #[rstest]
    fn test_an_unknown_word_is_refused() {
        assert!(adjustment_from_name("split").is_err());
        assert!(session_from_name("overnight-plus").is_err());
        assert!(markets_from_codes(&["JP".to_string()]).is_err());
    }
}
