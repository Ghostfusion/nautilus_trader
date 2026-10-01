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
