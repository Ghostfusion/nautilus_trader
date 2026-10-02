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

//! Wire messages for the EODHD streaming API.
//!
//! A frame is identified by its shape rather than by a message type, because the channels carry no
//! discriminator field: a frame with `p` and `v` is a trade print, a frame with `bp` or `ap` is a
//! quote, and a frame with `status_code` or `status` is a control message. Fields the adapter does
//! not use are ignored, so a vendor addition does not stop a frame from being read.

use serde::Deserialize;

/// A trade print from the United States trades channel.
///
/// The observed frame is `{"s","p","c","v","dp","ms","t"}`, where `t` is an epoch millisecond.
/// The exchange conditions are not read: their element type is not documented, and an ignored
/// field cannot fail a parse.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct EodhdTradeMessage {
    /// The symbol without an exchange suffix, for example `AAPL`.
    #[serde(rename = "s")]
    pub symbol: String,
    /// The traded price.
    #[serde(rename = "p")]
    pub price: f64,
    /// The traded size.
    #[serde(rename = "v")]
    pub size: f64,
    /// The trade time as an epoch millisecond.
    #[serde(rename = "t")]
    pub timestamp: i64,
    /// Whether the print is a dark pool print, when supplied.
    #[serde(rename = "dp", default)]
    pub dark_pool: Option<bool>,
    /// The market session label, for example `extended-hours`, when supplied.
    #[serde(rename = "ms", default)]
    pub session: Option<String>,
}

/// A best bid and offer update from the United States quotes channel.
///
/// The observed frame is `{"s","ap","as","bp","bs","t"}`, where `t` is an epoch millisecond.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct EodhdQuoteMessage {
    /// The symbol without an exchange suffix, for example `AAPL`.
    #[serde(rename = "s")]
    pub symbol: String,
    /// The ask price.
    #[serde(rename = "ap")]
    pub ask_price: f64,
    /// The ask size.
    #[serde(rename = "as")]
    pub ask_size: f64,
    /// The bid price.
    #[serde(rename = "bp")]
    pub bid_price: f64,
    /// The bid size.
    #[serde(rename = "bs")]
    pub bid_size: f64,
    /// The update time as an epoch millisecond.
    #[serde(rename = "t")]
    pub timestamp: i64,
}

/// A control frame from any channel.
///
/// The server reports authorization as `{"status_code":200,"message":"Authorized"}`, a refused
/// subscription as `{"status_code":422,"message":"..."}`, and a refused connection as
/// `{"status":403,"message":"Server error"}`. Both `status_code` and `status` are accepted.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct EodhdWsStatus {
    /// The status code, under either name the server uses.
    #[serde(default, alias = "status")]
    pub status_code: Option<i64>,
    /// The status message, when supplied.
    #[serde(default)]
    pub message: Option<String>,
}

/// A parsed message from an EODHD streaming channel.
#[derive(Clone, Debug, PartialEq)]
pub enum EodhdWsMessage {
    /// The connection is authorized, which the server sends before it accepts subscriptions.
    Authorized,
    /// A trade print.
    Trade(EodhdTradeMessage),
    /// A best bid and offer update.
    Quote(EodhdQuoteMessage),
    /// The server reports that a subscription or the connection was refused.
    Status {
        /// The reported status code.
        code: i64,
        /// The reported message.
        message: String,
    },
    /// A frame carrying none of the shapes above.
    Unknown(serde_json::Value),
}

/// Parses a streaming frame into an [`EodhdWsMessage`].
///
/// # Errors
///
/// Returns an error if the frame is not valid JSON.
pub fn parse_message(text: &str) -> anyhow::Result<EodhdWsMessage> {
    let value: serde_json::Value = serde_json::from_str(text)?;

    if value.get("status_code").is_some() || value.get("status").is_some() {
        let status: EodhdWsStatus = serde_json::from_value(value)?;
        let code = status.status_code.unwrap_or_default();
        let message = status.message.unwrap_or_default();

        if code == 200 && message.eq_ignore_ascii_case("authorized") {
            return Ok(EodhdWsMessage::Authorized);
        }

        return Ok(EodhdWsMessage::Status { code, message });
    }

    if value.get("p").is_some() && value.get("v").is_some() {
        return Ok(EodhdWsMessage::Trade(serde_json::from_value(value)?));
    }

    if value.get("bp").is_some() || value.get("ap").is_some() {
        return Ok(EodhdWsMessage::Quote(serde_json::from_value(value)?));
    }

    Ok(EodhdWsMessage::Unknown(value))
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    // Frames captured from the live channels.
    const TRADE: &str = r#"{"s":"AAPL","p":330.78,"c":[],"v":5,"dp":false,"ms":"extended-hours","t":1790899194028}"#;
    const QUOTE: &str = r#"{"s":"AAPL","ap":330.8,"as":15,"bp":330,"bs":67,"t":1790899198000}"#;
    const AUTHORIZED: &str = r#"{"status_code":200,"message":"Authorized"}"#;
    const REFUSED_SUBSCRIPTION: &str =
        r#"{"status_code":422,"message":"Only limited symbols allowed for demo"}"#;
    const REFUSED_CONNECTION: &str = r#"{"status":403,"message":"Server error"}"#;

    #[rstest]
    fn test_parse_trade_reads_the_price_size_and_time() {
        let message = parse_message(TRADE).unwrap();

        let EodhdWsMessage::Trade(trade) = message else {
            panic!("expected a trade, got {message:?}");
        };

        assert_eq!(trade.symbol, "AAPL");
        assert_eq!(trade.price, 330.78);
        assert_eq!(trade.size, 5.0);
        assert_eq!(trade.timestamp, 1_790_899_194_028);
        assert_eq!(trade.dark_pool, Some(false));
        assert_eq!(trade.session.as_deref(), Some("extended-hours"));
    }

    #[rstest]
    fn test_parse_quote_reads_both_sides() {
        let message = parse_message(QUOTE).unwrap();

        let EodhdWsMessage::Quote(quote) = message else {
            panic!("expected a quote, got {message:?}");
        };

        assert_eq!(quote.symbol, "AAPL");
        assert_eq!(quote.bid_price, 330.0);
        assert_eq!(quote.bid_size, 67.0);
        assert_eq!(quote.ask_price, 330.8);
        assert_eq!(quote.ask_size, 15.0);
        assert_eq!(quote.timestamp, 1_790_899_198_000);
    }

    #[rstest]
    fn test_parse_authorization() {
        assert_eq!(
            parse_message(AUTHORIZED).unwrap(),
            EodhdWsMessage::Authorized
        );
    }

    #[rstest]
    fn test_parse_refused_subscription() {
        assert_eq!(
            parse_message(REFUSED_SUBSCRIPTION).unwrap(),
            EodhdWsMessage::Status {
                code: 422,
                message: "Only limited symbols allowed for demo".to_string(),
            }
        );
    }

    #[rstest]
    fn test_parse_refused_connection_accepts_the_status_field() {
        assert_eq!(
            parse_message(REFUSED_CONNECTION).unwrap(),
            EodhdWsMessage::Status {
                code: 403,
                message: "Server error".to_string(),
            }
        );
    }

    #[rstest]
    fn test_parse_trade_ignores_an_unknown_field() {
        let frame = r#"{"s":"AAPL","p":1.0,"v":1.0,"t":1,"adding":"later"}"#;

        assert!(matches!(
            parse_message(frame).unwrap(),
            EodhdWsMessage::Trade(_)
        ));
    }

    #[rstest]
    fn test_parse_trade_survives_a_trade_without_the_session_field() {
        let frame = r#"{"s":"AAPL","p":1.0,"v":1.0,"t":1}"#;

        let EodhdWsMessage::Trade(trade) = parse_message(frame).unwrap() else {
            panic!("expected a trade");
        };

        assert_eq!(trade.session, None);
        assert_eq!(trade.dark_pool, None);
    }

    #[rstest]
    fn test_parse_quote_accepts_a_fractional_size() {
        let frame = r#"{"s":"AAPL","ap":1.0,"as":1.5,"bp":1.0,"bs":2.5,"t":1}"#;

        assert!(matches!(
            parse_message(frame).unwrap(),
            EodhdWsMessage::Quote(_)
        ));
    }

    #[rstest]
    fn test_parse_unrecognized_frame_is_reported_not_dropped() {
        let frame = r#"{"s":"AAPL","h":"S","r":"0","t":1}"#;

        assert!(matches!(
            parse_message(frame).unwrap(),
            EodhdWsMessage::Unknown(_)
        ));
    }

    #[rstest]
    fn test_parse_rejects_invalid_json() {
        assert!(parse_message("not json").is_err());
    }
}
