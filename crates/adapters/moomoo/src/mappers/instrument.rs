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

//! Conversion of the gateway's instrument records to the domain model.
//!
//! Two gateway calls describe one instrument and both are needed. The static record carries
//! identity and definition: the code, the name, the lot size, the listing time, the security type,
//! and the exchange type. The snapshot carries the price spread, which is the only field on either
//! record that can be read as a tick size. Neither is sufficient alone, because a static record has
//! no spread and a snapshot has no security type to filter on.
//!
//! # Identifiers
//!
//! A gateway code such as `US.AAPL` becomes an instrument whose symbol is `AAPL` and whose venue is
//! the market code `US`, with the gateway's own form kept as the raw symbol. The exchange type the
//! static record carries is a listing venue such as `NASDAQ`, and it is deliberately not the
//! instrument venue: folding it in would make this adapter's identifiers disagree with every other
//! provider's for the same security.

use anyhow::{Context, bail};
use nautilus_core::time::get_atomic_clock_realtime;
use nautilus_model::{
    identifiers::{InstrumentId, Symbol, Venue},
    instruments::{Equity, InstrumentAny},
    types::{Price, Quantity},
};

use crate::{
    common::Market,
    generated::{qot_common::SecurityStaticInfo, qot_get_security_snapshot::SnapshotBasicData},
};

/// The gateway's `SecurityType_Eqty`, the only type this adapter models.
pub const SECURITY_TYPE_EQUITY: i32 = 3;

/// The largest price precision this adapter will derive from a spread.
///
/// The gateway reports a spread as a double, so a value that is not a clean decimal could otherwise
/// derive an arbitrary precision and a price increment of zero.
const MAX_PRICE_PRECISION: u8 = 9;

/// Returns whether the gateway's security type is one this adapter models.
#[must_use]
pub fn is_modelled(sec_type: i32) -> bool {
    sec_type == SECURITY_TYPE_EQUITY
}

/// Returns the domain venue for a gateway market code.
///
/// # Errors
///
/// Returns an error if the market is not one this adapter serves, or if the venue name is not a
/// valid identifier.
pub fn venue_for_market(code: i32) -> anyhow::Result<(Market, Venue)> {
    let market = Market::from_qot_market(code)
        .with_context(|| format!("unsupported QotMarket code {code}"))?;
    let venue = Venue::new_checked(market.code())?;

    Ok((market, venue))
}

/// Derives the price precision from a snapshot's price spread.
///
/// The spread is the venue's own statement of the smallest price step, so the precision is the
/// number of decimal places it takes to write that step exactly. The search is exact rather than a
/// logarithm, because a logarithm gets realistic steps wrong: `log10(0.05)` rounds to `-1`, which
/// would report one decimal place and a tick of `0.1` for a venue whose tick is `0.05`, and Hong
/// Kong prices use steps of `0.005` and `0.05` in ordinary bands.
///
/// `fallback` is used when the spread cannot be read as a step, which is the case for a missing,
/// zero, or negative value.
#[must_use]
pub fn price_precision_from_spread(spread: f64, fallback: u8) -> u8 {
    if !spread.is_finite() || spread <= 0.0 {
        return fallback;
    }

    for precision in 0..=MAX_PRICE_PRECISION {
        let scaled = spread * 10f64.powi(i32::from(precision));
        let rounded = scaled.round();

        // A step has to be at least one unit at this precision, which also stops a spread far below
        // what any precision can resolve from rounding to zero and looking exact.
        if rounded < 1.0 {
            continue;
        }

        // A relative tolerance, because a step that is exact in decimal is usually not exact in
        // binary: `0.005 * 1000` is `5.000000000000001`.
        if (scaled - rounded).abs() <= 1e-9 * rounded {
            return precision;
        }
    }

    fallback
}

/// Builds a domain instrument from the gateway's two records.
///
/// `snapshot` is optional because an instrument definition does not depend on it. Without one the
/// price precision falls back to the market default, which is a weaker answer than the snapshot's
/// spread but not a wrong instrument.
///
/// # Errors
///
/// Returns an error if the security type is not modelled, the market is not served, the code is
/// empty, the lot size is not a positive integer, or the instrument fails domain validation.
pub fn instrument_from(
    static_info: &SecurityStaticInfo,
    snapshot: Option<&SnapshotBasicData>,
) -> anyhow::Result<InstrumentAny> {
    let basic = &static_info.basic;

    if !is_modelled(basic.sec_type) {
        bail!(
            "security type {} is not modelled as an equity",
            basic.sec_type
        );
    }

    let (market, venue) = venue_for_market(basic.security.market)?;
    let code = basic.security.code.as_str();

    if code.is_empty() {
        bail!("the gateway returned an empty code for market {market}");
    }

    // The gateway's own form is `<market>.<code>`, so the raw symbol round-trips back to a gateway
    // request without a lookup table.
    let raw_symbol = Symbol::from(format!("{}.{code}", market.code()));
    let instrument_id = InstrumentId::new(Symbol::from(code), venue);

    let fallback = market.fallback_price_precision();
    let price_precision = snapshot.map_or(fallback, |snapshot| {
        price_precision_from_spread(snapshot.price_spread, fallback)
    });
    let price_increment = Price::new(10f64.powi(-i32::from(price_precision)), price_precision);

    // The lot size is a count of shares, so it carries no decimal precision, and a non-positive
    // value means the gateway did not report one rather than that the lot is empty.
    let lot_size = u32::try_from(basic.lot_size)
        .ok()
        .filter(|size| *size > 0)
        .map(|size| Quantity::new(f64::from(size), 0));

    let timestamp = get_atomic_clock_realtime().get_time_ns();

    let equity = Equity::builder()
        .instrument_id(instrument_id)
        .raw_symbol(raw_symbol)
        .currency(market.currency())
        .price_precision(price_precision)
        .price_increment(price_increment)
        .maybe_lot_size(lot_size)
        .ts_event(timestamp)
        .ts_init(timestamp)
        .build()?;

    Ok(InstrumentAny::from(equity))
}

#[cfg(test)]
mod tests {
    use prost::Message as _;
    use rstest::rstest;

    use super::*;
    use crate::generated::qot_get_static_info;

    /// A step such as `0.05` is the case a logarithm gets wrong. It is two decimal places, but
    /// `log10(0.05)` rounds to `-1` and would report a tick ten times the real one.
    #[rstest]
    #[case::cents(0.01, 2)]
    #[case::tenth(0.1, 1)]
    #[case::whole(1.0, 0)]
    #[case::mills(0.001, 3)]
    #[case::half_cent(0.005, 3)]
    #[case::nickel(0.05, 2)]
    #[case::quarter(0.25, 2)]
    #[case::half(0.5, 1)]
    fn test_precision_comes_from_the_spread(#[case] spread: f64, #[case] expected: u8) {
        assert_eq!(price_precision_from_spread(spread, 9), expected);
    }

    /// A spread the algorithm cannot resolve is not evidence of a fine tick, so it falls back to
    /// the market default rather than reporting a step the venue cannot have.
    #[rstest]
    #[case::zero(0.0)]
    #[case::negative(-0.01)]
    #[case::nan(f64::NAN)]
    #[case::infinite(f64::INFINITY)]
    #[case::unresolvable(1e-30)]
    fn test_unusable_spreads_fall_back(#[case] spread: f64) {
        assert_eq!(price_precision_from_spread(spread, 2), 2);
    }

    /// A step larger than one has no decimal places, so the precision is zero and not negative.
    #[rstest]
    fn test_a_step_larger_than_one_has_no_decimals() {
        assert_eq!(price_precision_from_spread(100.0, 2), 0);
    }

    #[rstest]
    #[case::us(11, "US")]
    #[case::hk(1, "HK")]
    fn test_market_codes_become_venues(#[case] code: i32, #[case] venue: &str) {
        let (market, resolved) = venue_for_market(code).unwrap();

        assert_eq!(market.code(), venue);
        assert_eq!(resolved.to_string(), venue);
    }

    #[rstest]
    #[case::unserved(0)]
    #[case::future_market(2)]
    fn test_unserved_markets_are_refused(#[case] code: i32) {
        assert!(venue_for_market(code).is_err());
    }

    #[rstest]
    #[case::equity(SECURITY_TYPE_EQUITY, true)]
    #[case::warrant(5, false)]
    #[case::future(10, false)]
    #[case::unknown(0, false)]
    fn test_only_equities_are_modelled(#[case] sec_type: i32, #[case] expected: bool) {
        assert_eq!(is_modelled(sec_type), expected);
    }

    /// The static record for `US.AAPL` exactly as the gateway returned it, so the test covers
    /// decoding the wire form as well as mapping the decoded record. The snapshot response is not
    /// reproduced here because its only field this mapping reads is the spread, taken from the same
    /// snapshot.
    const AAPL_STATIC_INFO: &str = "08001200180022360a340a320a08080b12044141504c1085c30c180120032a054170706c65320a313938302d31322d3132380041000000501397b4414805";

    /// The spread the gateway reported for `US.AAPL` in that session.
    const AAPL_PRICE_SPREAD: f64 = 0.01;

    fn decode_hex(text: &str) -> Vec<u8> {
        assert!(text.len().is_multiple_of(2), "hex input must be even");

        let bytes = text.as_bytes();
        (0..text.len() / 2)
            .map(|i| {
                let pair = std::str::from_utf8(&bytes[i * 2..i * 2 + 2]).unwrap();
                u8::from_str_radix(pair, 16).unwrap()
            })
            .collect()
    }

    fn live_static_info() -> SecurityStaticInfo {
        let response =
            qot_get_static_info::Response::decode(decode_hex(AAPL_STATIC_INFO).as_slice()).unwrap();

        response
            .s2c
            .unwrap()
            .static_info_list
            .into_iter()
            .next()
            .unwrap()
    }

    #[rstest]
    fn test_a_live_record_maps_to_a_complete_instrument() {
        let static_info = live_static_info();
        let snapshot = SnapshotBasicData {
            price_spread: AAPL_PRICE_SPREAD,
            ..Default::default()
        };

        let InstrumentAny::Equity(equity) = instrument_from(&static_info, Some(&snapshot)).unwrap()
        else {
            panic!("an equity record should map to an equity");
        };

        // The identifier is the whole point of the mapping: it has to be the one every other
        // provider produces for this security, or a failover chain cannot swap feeds.
        assert_eq!(equity.id.to_string(), "AAPL.US");
        assert_eq!(equity.id.symbol.to_string(), "AAPL");
        assert_eq!(equity.id.venue.to_string(), "US");

        // The gateway's own form round-trips back to a request.
        assert_eq!(equity.raw_symbol.to_string(), "US.AAPL");

        assert_eq!(equity.currency, Market::Us.currency());
        assert_eq!(equity.price_precision, 2);
        assert_eq!(equity.price_increment, Price::new(0.01, 2));
        assert_eq!(equity.lot_size, Some(Quantity::new(1.0, 0)));
    }

    /// Without a snapshot the spread is unknown, so the instrument falls back to the market default
    /// rather than claiming a tick it was not told.
    #[rstest]
    fn test_a_record_without_a_snapshot_uses_the_market_default() {
        let static_info = live_static_info();

        let InstrumentAny::Equity(equity) = instrument_from(&static_info, None).unwrap() else {
            panic!("an equity record should map to an equity");
        };

        assert_eq!(
            equity.price_precision,
            Market::Us.fallback_price_precision()
        );
    }

    /// A security the adapter does not model has to be refused rather than filed as an equity,
    /// because approximating a warrant into an equity is worse than not loading it.
    #[rstest]
    fn test_an_unmodelled_security_is_refused() {
        let mut static_info = live_static_info();
        static_info.basic.sec_type = 5;

        let error = instrument_from(&static_info, None).unwrap_err();

        assert!(error.to_string().contains("not modelled"));
    }

    /// The exchange type the record carries is a listing venue, and it must not reach the
    /// instrument identifier.
    #[rstest]
    fn test_the_listing_venue_does_not_become_the_instrument_venue() {
        let mut static_info = live_static_info();
        static_info.basic.exch_type = Some(1);

        let InstrumentAny::Equity(equity) = instrument_from(&static_info, None).unwrap() else {
            panic!("an equity record should map to an equity");
        };

        assert_eq!(equity.id.venue.to_string(), "US");
    }
}
