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

//! What the gateway says this login may do.
//!
//! The entitlement read answers with the quote right of every market, the subscription and
//! historical allowances, and whether the risk disclaimer must still be accepted. Resolving it once
//! at startup, rather than discovering it when a request is refused, is what makes a refusal
//! survivable: the operator learns which markets are usable before a strategy depends on one, and a
//! request for a market that is not entitled fails immediately with a message that names the market
//! and the gateway's own word for what it reported.
//!
//! A quote right is not simply present or absent. `Bmp` is a snapshot-only right that the gateway
//! refuses to subscribe, so it does not run a live feed, and a missing value is unknown rather than
//! granted. Only the level rights are treated as usable, which is why the record keeps the gateway's
//! vocabulary rather than collapsing it to a boolean.

use prost::Message as _;
use thiserror::Error;

use crate::{
    common::Market,
    connection::{Connection, ConnectionError, RET_OK},
    generated::get_user_info,
};

/// The protocol identifier of the entitlement read.
pub const PROTO_ID_GET_USER_INFO: u32 = 1005;

/// A market's quote entitlement, in the gateway's own vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuoteRight {
    /// The gateway did not say, which is not the same as a refusal.
    Unknown,
    /// Snapshot only; the market cannot be subscribed.
    Bmp,
    /// Level one.
    Level1,
    /// Level two.
    Level2,
    /// The advanced real-time feed.
    Sf,
    /// No entitlement.
    No,
    /// Level three.
    Level3,
}

impl QuoteRight {
    /// Converts the gateway's numeric code, per `Qot_Common.QotRight`.
    #[must_use]
    pub fn from_code(code: i32) -> Self {
        match code {
            1 => Self::Bmp,
            2 => Self::Level1,
            3 => Self::Level2,
            4 => Self::Sf,
            5 => Self::No,
            6 => Self::Level3,
            _ => Self::Unknown,
        }
    }

    /// Returns the gateway's own name for the right.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "QotRight_Unknow",
            Self::Bmp => "QotRight_Bmp",
            Self::Level1 => "QotRight_Level1",
            Self::Level2 => "QotRight_Level2",
            Self::Sf => "QotRight_SF",
            Self::No => "QotRight_No",
            Self::Level3 => "QotRight_Level3",
        }
    }

    /// Returns whether the right serves subscribable quote data.
    #[must_use]
    pub fn is_subscribable(self) -> bool {
        matches!(self, Self::Level1 | Self::Level2 | Self::Sf | Self::Level3)
    }
}

/// What the gateway says this login may do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capabilities {
    /// The US equity quote entitlement.
    pub us_equity: QuoteRight,
    /// The Hong Kong equity quote entitlement.
    pub hk_equity: QuoteRight,
    /// The US option quote entitlement.
    pub us_option: QuoteRight,
    /// The US future quote entitlement.
    pub us_future: QuoteRight,
    /// The Hong Kong option quote entitlement.
    pub hk_option: QuoteRight,
    /// The Hong Kong future quote entitlement.
    pub hk_future: QuoteRight,
    /// The subscription allowance the gateway reports, in symbols.
    pub subscription_quota: Option<i32>,
    /// The historical request allowance the gateway reports.
    pub history_quota: Option<i32>,
    /// Whether the operator must still accept the risk disclaimer.
    pub disclaimer_required: bool,
    /// The gateway's attribution code for the account.
    pub user_attribution: Option<i32>,
}

impl Capabilities {
    /// Builds the record from an entitlement response.
    ///
    /// # Errors
    ///
    /// Returns [`EntitlementError::Refused`] if the gateway refused the read, or
    /// [`EntitlementError::MissingPayload`] if it answered without one.
    pub fn from_response(response: &get_user_info::Response) -> Result<Self, EntitlementError> {
        if response.ret_type != RET_OK {
            return Err(EntitlementError::Refused {
                ret_type: response.ret_type,
                message: response.ret_msg.clone().unwrap_or_default(),
            });
        }

        let s2c = response
            .s2c
            .as_ref()
            .ok_or(EntitlementError::MissingPayload)?;

        Ok(Self {
            us_equity: QuoteRight::from_code(s2c.us_qot_right.unwrap_or_default()),
            hk_equity: QuoteRight::from_code(s2c.hk_qot_right.unwrap_or_default()),
            us_option: QuoteRight::from_code(s2c.us_option_qot_right.unwrap_or_default()),
            us_future: QuoteRight::from_code(s2c.us_future_qot_right.unwrap_or_default()),
            hk_option: QuoteRight::from_code(s2c.hk_option_qot_right.unwrap_or_default()),
            hk_future: QuoteRight::from_code(s2c.hk_future_qot_right.unwrap_or_default()),
            subscription_quota: s2c.sub_quota,
            history_quota: s2c.history_kl_quota,
            disclaimer_required: s2c.is_need_agree_disclaimer.unwrap_or_default(),
            user_attribution: s2c.user_attribution,
        })
    }

    /// Returns the equity quote entitlement for a market.
    #[must_use]
    pub fn equity_quote_right(&self, market: Market) -> QuoteRight {
        match market {
            Market::Us => self.us_equity,
            Market::Hk => self.hk_equity,
        }
    }

    /// Fails unless the login may subscribe to a market's equity quotes.
    ///
    /// This is the check a request makes before it is sent, so that a market without an entitlement
    /// is reported once, in the gateway's terms, rather than as a failed request later.
    ///
    /// # Errors
    ///
    /// Returns [`EntitlementError::MissingEquityQuotes`], naming the market and what the gateway
    /// reported for it.
    pub fn require_equity_quotes(&self, market: Market) -> Result<QuoteRight, EntitlementError> {
        let right = self.equity_quote_right(market);

        if right.is_subscribable() {
            return Ok(right);
        }

        Err(EntitlementError::MissingEquityQuotes {
            market: market.code(),
            right: right.as_str(),
        })
    }

    /// Reads the record from the gateway.
    ///
    /// # Errors
    ///
    /// Returns [`EntitlementError::Connection`] if the read does not complete,
    /// [`EntitlementError::Decode`] if the answer cannot be decoded, or the errors of
    /// [`Self::from_response`].
    pub async fn read(connection: &Connection) -> Result<Self, EntitlementError> {
        // Leaving `flag` unset asks for every field, which is what this record needs.
        let request = get_user_info::Request {
            c2s: get_user_info::C2s { flag: None },
        };

        let message = connection
            .request(PROTO_ID_GET_USER_INFO, &request.encode_to_vec())
            .await?;

        let response = get_user_info::Response::decode(message.body.as_slice())
            .map_err(|e| EntitlementError::Decode(e.to_string()))?;

        Self::from_response(&response)
    }
}

/// The ways reading the entitlement can fail.
#[derive(Debug, Error)]
pub enum EntitlementError {
    /// The read did not complete.
    #[error(transparent)]
    Connection(#[from] ConnectionError),
    /// The answer could not be decoded.
    #[error("cannot decode the entitlement response: {0}")]
    Decode(String),
    /// The gateway refused the read.
    #[error("the gateway refused the entitlement read with retType {ret_type}: {message:?}")]
    Refused {
        /// The gateway's result code.
        ret_type: i32,
        /// The gateway's message, which is the actionable part.
        message: String,
    },
    /// The answer carried no payload.
    #[error("the entitlement response carried no payload")]
    MissingPayload,
    /// The login cannot subscribe to a market's equity quotes.
    #[error(
        "no {market} equity quote entitlement: the gateway reports {right}, and subscribing to \
         {market} quotes needs a {market} quote card"
    )]
    MissingEquityQuotes {
        /// The market, by the gateway's own code.
        market: &'static str,
        /// What the gateway reported for that market.
        right: &'static str,
    },
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    /// The entitlement this login reports, as read from the gateway through the adapter's own
    /// transport: US equities at level two, Hong Kong equities, options, and futures at level one,
    /// US options refused, and US futures not reported at all.
    fn observed_response() -> get_user_info::Response {
        get_user_info::Response {
            ret_type: RET_OK,
            ret_msg: None,
            err_code: None,
            s2c: Some(get_user_info::S2c {
                us_qot_right: Some(3),
                hk_qot_right: Some(2),
                us_option_qot_right: Some(5),
                has_us_option_qot_right: Some(false),
                us_future_qot_right: None,
                hk_option_qot_right: Some(2),
                hk_future_qot_right: Some(2),
                sub_quota: Some(100),
                history_kl_quota: Some(100),
                is_need_agree_disclaimer: Some(false),
                user_attribution: Some(2),
                ..Default::default()
            }),
        }
    }

    #[rstest]
    fn test_capabilities_are_read_from_the_response() {
        let capabilities = Capabilities::from_response(&observed_response()).unwrap();

        assert_eq!(capabilities.us_equity, QuoteRight::Level2);
        assert_eq!(capabilities.hk_equity, QuoteRight::Level1);
        assert_eq!(capabilities.us_option, QuoteRight::No);
        assert_eq!(capabilities.us_future, QuoteRight::Unknown);
        assert_eq!(capabilities.hk_option, QuoteRight::Level1);
        assert_eq!(capabilities.hk_future, QuoteRight::Level1);
        assert_eq!(capabilities.subscription_quota, Some(100));
        assert_eq!(capabilities.history_quota, Some(100));
        assert!(!capabilities.disclaimer_required);
        assert_eq!(capabilities.user_attribution, Some(2));
    }

    #[rstest]
    fn test_the_gate_passes_where_the_right_is_subscribable() {
        let capabilities = Capabilities::from_response(&observed_response()).unwrap();

        assert_eq!(
            capabilities.require_equity_quotes(Market::Us).unwrap(),
            QuoteRight::Level2
        );
        assert_eq!(
            capabilities.require_equity_quotes(Market::Hk).unwrap(),
            QuoteRight::Level1
        );
    }

    /// A snapshot-only right and an absent right both fail the gate, and the message says which,
    /// because one means "buy a quote card" and the other means "the gateway did not answer".
    #[rstest]
    #[case::snapshot_only(1, "QotRight_Bmp", Market::Us)]
    #[case::no_entitlement(5, "QotRight_No", Market::Us)]
    #[case::not_reported(0, "QotRight_Unknow", Market::Us)]
    fn test_the_gate_refuses_with_the_gateways_own_terms(
        #[case] code: i32,
        #[case] right: &str,
        #[case] market: Market,
    ) {
        let mut response = observed_response();
        response.s2c.as_mut().unwrap().us_qot_right = Some(code);

        let capabilities = Capabilities::from_response(&response).unwrap();
        let error = capabilities.require_equity_quotes(market).unwrap_err();

        assert!(
            matches!(
                error,
                EntitlementError::MissingEquityQuotes {
                    market: "US",
                    right: reported,
                } if reported == right
            ),
            "unexpected error: {error}"
        );
        assert!(error.to_string().contains(right));
    }

    #[rstest]
    fn test_a_refused_read_reports_the_gateway_message() {
        let response = get_user_info::Response {
            ret_type: -1,
            ret_msg: Some("please log in".to_string()),
            ..Default::default()
        };

        let error = Capabilities::from_response(&response).unwrap_err();

        assert!(matches!(
            error,
            EntitlementError::Refused { ret_type: -1, .. }
        ));
        assert!(error.to_string().contains("please log in"));
    }

    #[rstest]
    fn test_a_response_without_a_payload_is_an_error() {
        let response = get_user_info::Response {
            ret_type: RET_OK,
            ..Default::default()
        };

        assert!(matches!(
            Capabilities::from_response(&response).unwrap_err(),
            EntitlementError::MissingPayload
        ));
    }
}
