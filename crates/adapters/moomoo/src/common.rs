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

//! Constants shared across the adapter, and the markets it serves.

use std::fmt;

use nautilus_model::types::Currency;

/// The client identifier the adapter sends in the handshake.
///
/// This is the adapter's identity as a provider, and it is deliberately not the instrument venue.
/// The venue of an instrument is the market the security trades on, so a moomoo instrument and an
/// EODHD instrument for the same security agree on their identifier, which is what lets a provider
/// failover chain swap one for the other without rewriting its instruments.
pub const CLIENT_ID: &str = "MOOMOO";

/// A market this adapter serves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Market {
    /// United States equities.
    Us,
    /// Hong Kong equities.
    Hk,
}

impl Market {
    /// Every market this adapter serves.
    pub const ALL: [Self; 2] = [Self::Us, Self::Hk];

    /// Returns the gateway's own code for the market.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::Us => "US",
            Self::Hk => "HK",
        }
    }

    /// Parses the gateway's own code for the market.
    #[must_use]
    pub fn from_code(code: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|market| market.code() == code)
    }

    /// Parses the gateway's numeric market code, per `Qot_Common.QotMarket`.
    ///
    /// Only the two equity markets this adapter serves are recognised. The gateway numbers a future
    /// market and a security market separately, and treating an unrecognised code as a default
    /// would silently file an instrument under the wrong venue.
    #[must_use]
    pub fn from_qot_market(code: i32) -> Option<Self> {
        match code {
            1 => Some(Self::Hk),
            11 => Some(Self::Us),
            _ => None,
        }
    }

    /// Returns the gateway's numeric market code for the market.
    #[must_use]
    pub fn qot_market(self) -> i32 {
        match self {
            Self::Us => 11,
            Self::Hk => 1,
        }
    }

    /// Returns the currency this market's equities are quoted in.
    #[must_use]
    pub fn currency(self) -> Currency {
        match self {
            Self::Us => Currency::from("USD"),
            Self::Hk => Currency::from("HKD"),
        }
    }

    /// Returns the price precision to use when no usable spread is available.
    ///
    /// This is a fallback, not a definition. A United States equity quotes in cents, and the price
    /// bands covering most of the Hong Kong board quote to three decimals, but Hong Kong tick sizes
    /// are banded by price, so a snapshot's own spread is authoritative whenever it can be used.
    #[must_use]
    pub fn fallback_price_precision(self) -> u8 {
        match self {
            Self::Us => 2,
            Self::Hk => 3,
        }
    }
}

impl fmt::Display for Market {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::us("US", Some(Market::Us))]
    #[case::hk("HK", Some(Market::Hk))]
    #[case::unknown("CN", None)]
    fn test_market_codes_round_trip(#[case] code: &str, #[case] expected: Option<Market>) {
        assert_eq!(Market::from_code(code), expected);
    }

    /// The numeric codes are the gateway's, and the two equity markets are not adjacent or
    /// contiguous, so a mistaken default would pass unnoticed without this.
    #[rstest]
    #[case::us(11, Some(Market::Us))]
    #[case::hk(1, Some(Market::Hk))]
    #[case::hk_future(2, None)]
    #[case::shares(0, None)]
    fn test_qot_market_codes_map(#[case] code: i32, #[case] expected: Option<Market>) {
        assert_eq!(Market::from_qot_market(code), expected);
    }

    #[rstest]
    fn test_qot_market_round_trips_for_served_markets() {
        for market in Market::ALL {
            assert_eq!(Market::from_qot_market(market.qot_market()), Some(market));
        }
    }
}
