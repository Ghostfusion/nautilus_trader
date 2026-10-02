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

//! Response models for the EODHD REST API.

use serde::Deserialize;

/// A single end-of-day OHLCV row from the `/eod/{ticker}` endpoint.
///
/// The endpoint returns rows in ascending date order. `volume` is absent or null for some
/// instruments (for example certain indices), and `adjusted_close` is absent on older plans.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct EodhdBar {
    /// The row date, formatted `YYYY-MM-DD`.
    pub date: String,
    /// The open price.
    pub open: f64,
    /// The high price.
    pub high: f64,
    /// The low price.
    pub low: f64,
    /// The close price.
    pub close: f64,
    /// The split and dividend adjusted close price, when supplied.
    #[serde(default)]
    pub adjusted_close: Option<f64>,
    /// The traded volume, when supplied.
    #[serde(default)]
    pub volume: Option<f64>,
}

/// A single row from the `/eod-bulk-last-day/{exchange}` endpoint.
///
/// One request returns the last day for every symbol on an exchange. The row carries the
/// exchange-local code rather than a full ticker, so the ticker is built from the code and the
/// exchange the row reports.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct EodhdBulkBar {
    /// The exchange-local ticker code, without the exchange suffix.
    pub code: String,
    /// The exchange code the endpoint addresses rows of this list with, when supplied.
    #[serde(rename = "exchange_short_name", default)]
    pub exchange: Option<String>,
    /// The row date, formatted `YYYY-MM-DD`.
    pub date: String,
    /// The open price.
    pub open: f64,
    /// The high price.
    pub high: f64,
    /// The low price.
    pub low: f64,
    /// The close price.
    pub close: f64,
    /// The split and dividend adjusted close price, when supplied.
    #[serde(default)]
    pub adjusted_close: Option<f64>,
    /// The traded volume, when supplied.
    #[serde(default)]
    pub volume: Option<f64>,
}

impl EodhdBulkBar {
    /// Returns the EODHD ticker for this row, falling back to the requested `exchange`.
    ///
    /// The row's own exchange code wins because the endpoint reports the code it addresses the
    /// symbol with: a sub-exchange query returns the parent code rather than the sub-exchange.
    #[must_use]
    pub fn ticker(&self, exchange: &str) -> String {
        format!(
            "{}.{}",
            self.code,
            self.exchange.as_deref().unwrap_or(exchange)
        )
    }
}

/// A single symbol row from the `/exchange-symbol-list/{exchange}` endpoint.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct EodhdSymbol {
    /// The exchange-local ticker code, without the exchange suffix.
    #[serde(rename = "Code")]
    pub code: String,
    /// The instrument name.
    #[serde(rename = "Name", default)]
    pub name: Option<String>,
    /// The country of listing.
    #[serde(rename = "Country", default)]
    pub country: Option<String>,
    /// The exchange code, which is also the EODHD ticker suffix.
    #[serde(rename = "Exchange", default)]
    pub exchange: Option<String>,
    /// The trading currency.
    #[serde(rename = "Currency", default)]
    pub currency: Option<String>,
    /// The instrument type, for example `Common Stock`.
    #[serde(rename = "Type", default)]
    pub symbol_type: Option<String>,
    /// The International Securities Identification Number, when supplied.
    #[serde(rename = "Isin", default)]
    pub isin: Option<String>,
}

/// A single intraday bar from the `/intraday/{ticker}` endpoint.
///
/// The `timestamp` is the authoritative epoch second. The `datetime` field is rendered in the
/// exchange offset carried by `gmtoffset`, and is not used for bar timestamps.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct EodhdIntradayBar {
    /// The bar timestamp as an epoch second.
    pub timestamp: i64,
    /// The exchange UTC offset in seconds, when supplied.
    #[serde(default)]
    pub gmtoffset: Option<i64>,
    /// The bar timestamp rendered in the exchange offset.
    #[serde(default)]
    pub datetime: Option<String>,
    /// The open price.
    pub open: f64,
    /// The high price.
    pub high: f64,
    /// The low price.
    pub low: f64,
    /// The close price.
    pub close: f64,
    /// The traded volume, when supplied.
    #[serde(default)]
    pub volume: Option<f64>,
}

/// A delayed quote snapshot from the `/us-quote-delayed` endpoint.
///
/// The endpoint describes the instrument as well as its quote. Only the quote fields are mapped
/// onto Nautilus types; the descriptive fields are available for inspection but carry no engine
/// semantics.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct EodhdDelayedQuote {
    /// The EODHD ticker, for example `AAPL.US`.
    pub symbol: String,
    /// The best bid price.
    #[serde(rename = "bidPrice")]
    pub bid_price: f64,
    /// The best ask price.
    #[serde(rename = "askPrice")]
    pub ask_price: f64,
    /// The size resting at the best bid.
    #[serde(rename = "bidSize", default)]
    pub bid_size: f64,
    /// The size resting at the best ask.
    #[serde(rename = "askSize", default)]
    pub ask_size: f64,
    /// The best bid time as an epoch millisecond, when supplied.
    #[serde(rename = "bidTime", default)]
    pub bid_time: Option<i64>,
    /// The best ask time as an epoch millisecond, when supplied.
    #[serde(rename = "askTime", default)]
    pub ask_time: Option<i64>,
    /// The snapshot time as an epoch second.
    #[serde(default)]
    pub timestamp: Option<i64>,
}

impl EodhdDelayedQuote {
    /// Returns the quote timestamp as epoch nanoseconds.
    ///
    /// The newer of the bid and ask times is used, falling back to the snapshot time.
    #[must_use]
    pub fn ts_event(&self) -> nautilus_core::UnixNanos {
        let millis = match (self.bid_time, self.ask_time) {
            (Some(bid), Some(ask)) => Some(bid.max(ask)),
            (Some(bid), None) => Some(bid),
            (None, Some(ask)) => Some(ask),
            (None, None) => self.timestamp.map(|seconds| seconds * 1_000),
        };

        nautilus_core::UnixNanos::from_millis(millis.unwrap_or(0).unsigned_abs())
    }
}

/// The `/us-quote-delayed` response envelope.
#[derive(Clone, Debug, Deserialize)]
pub struct EodhdDelayedQuoteResponse {
    /// The quotes keyed by EODHD ticker.
    #[serde(default)]
    pub data: std::collections::HashMap<String, EodhdDelayedQuote>,
}

/// The JSON error envelope returned by EODHD.
///
/// A JSON object carrying both `code` and `message` is an error. A JSON object carrying only
/// `code` is a normal payload, because some endpoints use `code` for a ticker symbol.
#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
pub struct EodhdErrorResponse {
    /// The provider error code.
    #[serde(default)]
    pub code: Option<serde_json::Value>,
    /// The provider error message.
    #[serde(default)]
    pub message: Option<String>,
}
