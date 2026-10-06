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

//! Risk cap vocabulary shared by the risk engine configuration and its order denials.
//!
//! A risk cap is a predicate over a scope, a metric and (for a count) a window, rather than a set
//! of hard coded counters. These two enums name the dimensions, so the configuration, the counter
//! state and the denial that reports a refusal speak one vocabulary.

use serde::{Deserialize, Serialize};
use strum::{AsRefStr, Display, EnumIter, EnumString};

use crate::{
    enums::{OrderSide, OrderType},
    identifiers::InstrumentId,
    orders::{Order, OrderAny},
    types::{Price, Quantity},
};

/// The scope a risk cap counts over.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Display,
    AsRefStr,
    EnumIter,
    EnumString,
    Serialize,
    Deserialize,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum RiskCapScope {
    /// Every order the engine sees.
    Global,
    /// The orders of one strategy.
    Strategy,
    /// The orders of one account.
    Account,
    /// The orders of one instrument.
    Instrument,
    /// The orders of one venue.
    Venue,
    /// The orders of one strategy in one instrument.
    StrategyInstrument,
}

/// The metric a risk cap counts.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Display,
    AsRefStr,
    EnumIter,
    EnumString,
    Serialize,
    Deserialize,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum RiskCapMetric {
    /// Orders currently open.
    ///
    /// This is the open order set as it stands rather than an event count, so it carries no
    /// window.
    Active,
    /// Orders admitted for submission.
    Submit,
    /// Orders admitted for modification.
    Modify,
    /// Cancellations observed.
    Cancel,
    /// Fills observed.
    Fill,
    /// Requests repeated with the same canonical identity as the one being evaluated.
    ///
    /// The identity is [`RiskRequestKey`]: the instrument, side, order type, quantity and price of
    /// the request, excluding the client order id, which a repeated request necessarily changes.
    RepeatedRequest,
    /// The quantity a scope has filled over the window, in the instrument's units.
    ///
    /// This is the participation limit as far as it is enforceable here: a budget of traded volume
    /// rather than a share of the market, because the engine observes this trader's fills and not
    /// the venue's total traded volume, so a denominator would have to be invented. Where the
    /// venue's volume is roughly stable the budget is the same discipline; where it is not, read
    /// the cap as a budget. It is measured in quantity, so it carries `RiskCap::quantity_limit`
    /// and must be scoped to an instrument.
    Participation,
    /// The absolute position size a scope holds in the instrument, in the instrument's units.
    ///
    /// Read from the cache's open position at the moment an action is gated, so this is standing
    /// inventory rather than occurrences over a window. It is measured in quantity, so it carries
    /// `RiskCap::quantity_limit`, must be scoped to an instrument, and takes no window.
    Inventory,
}

/// The canonical identity of an order request, excluding the client order id.
///
/// Two requests share an identity when they ask for the same instrument, side, order type, quantity
/// and price. A repeated request guard keys on this identity rather than on the client order id,
/// because a strategy repeating a request necessarily issues a new one, so a counter keyed on the
/// client order id would never observe the repeat.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RiskRequestKey {
    /// The instrument the request trades.
    pub instrument_id: InstrumentId,
    /// The order type requested.
    pub order_type: OrderType,
    /// The side requested.
    pub order_side: OrderSide,
    /// The quantity requested.
    pub quantity: Quantity,
    /// The price requested, `None` for an order type that carries no price.
    pub price: Option<Price>,
}

impl RiskRequestKey {
    /// Creates a new [`RiskRequestKey`] instance.
    #[must_use]
    pub const fn new(
        instrument_id: InstrumentId,
        order_type: OrderType,
        order_side: OrderSide,
        quantity: Quantity,
        price: Option<Price>,
    ) -> Self {
        Self {
            instrument_id,
            order_type,
            order_side,
            quantity,
            price,
        }
    }
}

impl From<&OrderAny> for RiskRequestKey {
    fn from(order: &OrderAny) -> Self {
        Self::new(
            order.instrument_id(),
            order.order_type(),
            order.order_side(),
            order.quantity(),
            order.price(),
        )
    }
}

impl std::fmt::Display for RiskRequestKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let price = self
            .price
            .map_or_else(|| "NONE".to_string(), |price| price.to_string());

        write!(
            f,
            "{}/{}/{}/{}@{price}",
            self.instrument_id, self.order_type, self.order_side, self.quantity
        )
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use rstest::rstest;
    use strum::IntoEnumIterator;

    use super::*;

    #[rstest]
    fn test_scope_tokens_are_screaming_snake() {
        for scope in RiskCapScope::iter() {
            let token = scope.to_string();

            assert_eq!(token, token.to_ascii_uppercase());
            assert!(token.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'));
            assert!(token.chars().any(|c| c.is_ascii_alphabetic()));
            assert_eq!(RiskCapScope::from_str(&token).unwrap(), scope);
            assert_eq!(scope.as_ref(), token);
        }
    }

    #[rstest]
    fn test_metric_tokens_are_screaming_snake() {
        for metric in RiskCapMetric::iter() {
            let token = metric.to_string();

            assert_eq!(token, token.to_ascii_uppercase());
            assert!(token.chars().all(|c| c.is_ascii_alphanumeric() || c == '_'));
            assert!(token.chars().any(|c| c.is_ascii_alphabetic()));
            assert_eq!(RiskCapMetric::from_str(&token).unwrap(), metric);
            assert_eq!(metric.as_ref(), token);
        }
    }

    #[rstest]
    fn test_scopes_serialize_as_their_tokens() {
        let json = serde_json::to_string(&RiskCapScope::StrategyInstrument).unwrap();

        assert_eq!(json, "\"STRATEGY_INSTRUMENT\"");
        assert_eq!(
            serde_json::from_str::<RiskCapScope>(&json).unwrap(),
            RiskCapScope::StrategyInstrument
        );
    }

    #[rstest]
    #[case(RiskCapMetric::Active)]
    #[case(RiskCapMetric::Submit)]
    #[case(RiskCapMetric::Modify)]
    #[case(RiskCapMetric::Cancel)]
    #[case(RiskCapMetric::Fill)]
    #[case(RiskCapMetric::RepeatedRequest)]
    fn test_metrics_serialize_as_their_tokens(#[case] metric: RiskCapMetric) {
        let json = serde_json::to_string(&metric).unwrap();

        assert_eq!(json, format!("\"{metric}\""));
        assert_eq!(
            serde_json::from_str::<RiskCapMetric>(&json).unwrap(),
            metric
        );
    }

    fn request_key(
        instrument_id: &str,
        order_type: OrderType,
        order_side: OrderSide,
        quantity: &str,
        price: Option<&str>,
    ) -> RiskRequestKey {
        RiskRequestKey::new(
            InstrumentId::from(instrument_id),
            order_type,
            order_side,
            Quantity::from(quantity),
            price.map(Price::from),
        )
    }

    #[rstest]
    fn test_request_key_is_canonical_over_its_fields() {
        let base = request_key(
            "AUD/USD.SIM",
            OrderType::Limit,
            OrderSide::Buy,
            "100000",
            Some("1.00000"),
        );

        assert_eq!(
            base,
            request_key(
                "AUD/USD.SIM",
                OrderType::Limit,
                OrderSide::Buy,
                "100000",
                Some("1.00000"),
            )
        );
        assert_ne!(
            base,
            request_key(
                "AUD/USD.SIM",
                OrderType::Limit,
                OrderSide::Sell,
                "100000",
                Some("1.00000"),
            )
        );
        assert_ne!(
            base,
            request_key(
                "AUD/USD.SIM",
                OrderType::Market,
                OrderSide::Buy,
                "100000",
                Some("1.00000"),
            )
        );
        assert_ne!(
            base,
            request_key(
                "AUD/USD.SIM",
                OrderType::Limit,
                OrderSide::Buy,
                "200000",
                Some("1.00000"),
            )
        );
        assert_ne!(
            base,
            request_key(
                "AUD/USD.SIM",
                OrderType::Limit,
                OrderSide::Buy,
                "100000",
                Some("1.00001"),
            )
        );
        assert_ne!(
            base,
            request_key(
                "AUD/USD.SIM",
                OrderType::Limit,
                OrderSide::Buy,
                "100000",
                None
            )
        );
        assert_ne!(
            base,
            request_key(
                "EUR/USD.SIM",
                OrderType::Limit,
                OrderSide::Buy,
                "100000",
                Some("1.00000"),
            )
        );
    }

    #[rstest]
    fn test_request_key_display_distinguishes_its_fields() {
        let base = request_key(
            "AUD/USD.SIM",
            OrderType::Limit,
            OrderSide::Buy,
            "100000",
            Some("1.00000"),
        );
        let same = request_key(
            "AUD/USD.SIM",
            OrderType::Limit,
            OrderSide::Buy,
            "100000",
            Some("1.00000"),
        );
        let other_side = request_key(
            "AUD/USD.SIM",
            OrderType::Limit,
            OrderSide::Sell,
            "100000",
            Some("1.00000"),
        );
        let no_price = request_key(
            "AUD/USD.SIM",
            OrderType::Market,
            OrderSide::Buy,
            "100000",
            None,
        );

        assert_eq!(base.to_string(), same.to_string());
        assert_ne!(base.to_string(), other_side.to_string());
        assert_ne!(base.to_string(), no_price.to_string());
        assert!(no_price.to_string().ends_with("@NONE"));
    }

    #[rstest]
    fn test_request_key_round_trips_through_json() {
        let key = request_key(
            "AUD/USD.SIM",
            OrderType::Limit,
            OrderSide::Buy,
            "100000",
            Some("1.00000"),
        );
        let json = serde_json::to_string(&key).unwrap();

        assert_eq!(serde_json::from_str::<RiskRequestKey>(&json).unwrap(), key);
    }
}
