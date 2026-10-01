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

//! Execution intent and policy.
//!
//! An execution algorithm is one way to attempt an order under the constraints the caller set, and
//! those are two declarations rather than one. The [`ExecutionIntent`] is what the caller wants -
//! the order's own terms, taken from the order the algorithm receives - and the [`ExecutionPolicy`]
//! is what the execution must respect while satisfying it.
//!
//! Keeping the two apart is what stops an algorithm accumulating a private parameter vocabulary.
//! An algorithm declares the [`PolicyPart`]s it can honour, and it refuses a declared part it cannot
//! honour instead of silently relaxing it, which is the failure mode that makes per algorithm
//! parameters dangerous: a caller who declares a slippage cap and receives an execution that
//! ignored it has been told something false about their own order.

use std::{fmt::Display, time::Duration};

use indexmap::IndexMap;
use nautilus_core::DurationNanos;
use nautilus_model::{
    enums::{OrderSide, OrderType},
    identifiers::InstrumentId,
    orders::{Order, OrderAny},
    types::{Price, Quantity},
};
use rust_decimal::Decimal;
use ustr::Ustr;

/// The parameter key declaring a policy's horizon in seconds.
pub const KEY_HORIZON_SECS: &str = "horizon_secs";
/// The parameter key declaring a policy's participation rate.
pub const KEY_PARTICIPATION_RATE: &str = "participation_rate";
/// The parameter key declaring a policy's price limit.
pub const KEY_PRICE_LIMIT: &str = "price_limit";
/// The parameter key declaring a policy's slippage cap in basis points.
pub const KEY_MAX_SLIPPAGE_BPS: &str = "max_slippage_bps";
/// The parameter key declaring a policy's passive or aggressive preference.
pub const KEY_PREFERENCE: &str = "preference";
/// The parameter key declaring a policy's urgency.
pub const KEY_URGENCY: &str = "urgency";

/// One part of a policy, which an algorithm either honours or refuses.
///
/// A part is declared by the presence of its parameter key, so an algorithm that cannot honour a
/// part refuses the declaration rather than executing an order that does not respect it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PolicyPart {
    /// The horizon the execution must complete within.
    Horizon,
    /// A participation rate against traded volume.
    ParticipationRate,
    /// A price limit no child may trade outside.
    PriceLimit,
    /// A slippage cap in basis points against a reference price.
    SlippageLimit,
    /// A passive or aggressive execution preference.
    Preference,
    /// An urgency selecting between the algorithm's own speed choices.
    Urgency,
}

impl PolicyPart {
    /// Every policy part, in the order refusals are reported.
    pub const ALL: [Self; 6] = [
        Self::Horizon,
        Self::ParticipationRate,
        Self::PriceLimit,
        Self::SlippageLimit,
        Self::Preference,
        Self::Urgency,
    ];

    /// Returns the parameter key that declares this part.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::Horizon => KEY_HORIZON_SECS,
            Self::ParticipationRate => KEY_PARTICIPATION_RATE,
            Self::PriceLimit => KEY_PRICE_LIMIT,
            Self::SlippageLimit => KEY_MAX_SLIPPAGE_BPS,
            Self::Preference => KEY_PREFERENCE,
            Self::Urgency => KEY_URGENCY,
        }
    }
}

impl Display for PolicyPart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.key())
    }
}

/// A passive or aggressive execution preference.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutionPreference {
    /// Prefer resting on the book over crossing it.
    Passive,
    /// Prefer an immediate execution over a resting one.
    Aggressive,
}

impl Display for ExecutionPreference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Passive => "passive",
            Self::Aggressive => "aggressive",
        })
    }
}

/// How quickly the caller wants the order executed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutionUrgency {
    /// Time is not critical; the execution may wait for better prices.
    Low,
    /// The caller's own pace, without a preference for speed over price.
    Normal,
    /// The execution should be completed as soon as the constraints allow.
    High,
}

impl Display for ExecutionUrgency {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Low => "low",
            Self::Normal => "normal",
            Self::High => "high",
        })
    }
}

/// What the caller wants executed, taken from the order the algorithm is asked to execute.
#[derive(Clone, Debug, PartialEq)]
pub struct ExecutionIntent {
    /// The instrument to execute in.
    pub instrument_id: InstrumentId,
    /// The side of the order.
    pub order_side: OrderSide,
    /// The order type the caller submitted.
    pub order_type: OrderType,
    /// The quantity to execute.
    pub quantity: Quantity,
    /// The order's price, when it has one.
    pub price: Option<Price>,
}

impl From<&OrderAny> for ExecutionIntent {
    fn from(order: &OrderAny) -> Self {
        Self {
            instrument_id: order.instrument_id(),
            order_side: order.order_side(),
            order_type: order.order_type(),
            quantity: order.quantity(),
            price: order.price(),
        }
    }
}

/// The constraints and preferences an execution must respect.
///
/// A policy is declared in the order's execution algorithm parameters, and every field is optional:
/// an absent part is a part the caller did not constrain. Which parameters an algorithm accepts is
/// its own affair, but a part it cannot honour is refused rather than ignored.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExecutionPolicy {
    /// The horizon the execution must complete within.
    pub horizon: Option<DurationNanos>,
    /// The participation rate against traded volume, as a fraction of it.
    pub participation_rate: Option<Decimal>,
    /// A price limit no child may trade outside.
    pub price_limit: Option<Price>,
    /// The maximum slippage against the reference price, in basis points.
    pub max_slippage_bps: Option<Decimal>,
    /// A passive or aggressive execution preference.
    pub preference: Option<ExecutionPreference>,
    /// How quickly the order should be executed.
    pub urgency: Option<ExecutionUrgency>,
}

impl ExecutionPolicy {
    /// Parses the policy declared in `params`.
    ///
    /// A malformed declaration is refused rather than defaulted, because the alternative is an
    /// execution that does not satisfy the constraint the caller believes they declared.
    ///
    /// # Errors
    ///
    /// Returns a [`PolicyError`] naming the field when a declared parameter cannot be parsed or is
    /// outside its domain.
    pub fn from_params(params: &IndexMap<Ustr, Ustr>) -> Result<Self, PolicyError> {
        let mut policy = Self::default();

        if let Some(raw) = params.get(&Ustr::from(KEY_HORIZON_SECS)) {
            let value = raw.as_str().parse::<f64>().map_err(|_| {
                PolicyError::new(
                    KEY_HORIZON_SECS,
                    format!("{KEY_HORIZON_SECS}={raw} is not a valid number"),
                )
            })?;
            policy.horizon = Some(nanos_from_seconds(KEY_HORIZON_SECS, value)?);
        }

        if let Some(raw) = params.get(&Ustr::from(KEY_PARTICIPATION_RATE)) {
            let value = raw.as_str().parse::<Decimal>().map_err(|_| {
                PolicyError::new(
                    KEY_PARTICIPATION_RATE,
                    format!("{KEY_PARTICIPATION_RATE}={raw} is not a valid number"),
                )
            })?;

            if value <= Decimal::ZERO || value > Decimal::ONE {
                return Err(PolicyError::new(
                    KEY_PARTICIPATION_RATE,
                    format!("{KEY_PARTICIPATION_RATE}={raw} must be greater than 0 and at most 1"),
                ));
            }
            policy.participation_rate = Some(value);
        }

        if let Some(raw) = params.get(&Ustr::from(KEY_PRICE_LIMIT)) {
            let value = raw.as_str().parse::<Price>().map_err(|_| {
                PolicyError::new(
                    KEY_PRICE_LIMIT,
                    format!("{KEY_PRICE_LIMIT}={raw} is not a valid price"),
                )
            })?;
            policy.price_limit = Some(value);
        }

        if let Some(raw) = params.get(&Ustr::from(KEY_MAX_SLIPPAGE_BPS)) {
            let value = raw.as_str().parse::<Decimal>().map_err(|_| {
                PolicyError::new(
                    KEY_MAX_SLIPPAGE_BPS,
                    format!("{KEY_MAX_SLIPPAGE_BPS}={raw} is not a valid number"),
                )
            })?;

            if value < Decimal::ZERO {
                return Err(PolicyError::new(
                    KEY_MAX_SLIPPAGE_BPS,
                    format!("{KEY_MAX_SLIPPAGE_BPS}={raw} must not be negative"),
                ));
            }
            policy.max_slippage_bps = Some(value);
        }

        if let Some(raw) = params.get(&Ustr::from(KEY_PREFERENCE)) {
            policy.preference = Some(match raw.as_str() {
                "passive" => ExecutionPreference::Passive,
                "aggressive" => ExecutionPreference::Aggressive,
                other => {
                    return Err(PolicyError::new(
                        KEY_PREFERENCE,
                        format!("{KEY_PREFERENCE}={other} must be one of passive or aggressive"),
                    ));
                }
            });
        }

        if let Some(raw) = params.get(&Ustr::from(KEY_URGENCY)) {
            policy.urgency = Some(match raw.as_str() {
                "low" => ExecutionUrgency::Low,
                "normal" => ExecutionUrgency::Normal,
                "high" => ExecutionUrgency::High,
                other => {
                    return Err(PolicyError::new(
                        KEY_URGENCY,
                        format!("{KEY_URGENCY}={other} must be one of low, normal or high"),
                    ));
                }
            });
        }

        Ok(policy)
    }

    /// Returns the policy parts this policy declares.
    #[must_use]
    pub fn declared_parts(&self) -> Vec<PolicyPart> {
        PolicyPart::ALL
            .into_iter()
            .filter(|part| self.is_declared(*part))
            .collect()
    }

    /// Returns whether `part` is declared by this policy.
    #[must_use]
    pub fn is_declared(&self, part: PolicyPart) -> bool {
        match part {
            PolicyPart::Horizon => self.horizon.is_some(),
            PolicyPart::ParticipationRate => self.participation_rate.is_some(),
            PolicyPart::PriceLimit => self.price_limit.is_some(),
            PolicyPart::SlippageLimit => self.max_slippage_bps.is_some(),
            PolicyPart::Preference => self.preference.is_some(),
            PolicyPart::Urgency => self.urgency.is_some(),
        }
    }

    /// Returns the first declared part of this policy that `supported` does not contain, in
    /// [`PolicyPart::ALL`] order so a refusal is deterministic.
    #[must_use]
    pub fn unsupported(&self, supported: &[PolicyPart]) -> Option<PolicyPart> {
        PolicyPart::ALL
            .into_iter()
            .find(|part| self.is_declared(*part) && !supported.contains(part))
    }

    /// Validates the policy against the intent it is asked to satisfy.
    ///
    /// # Errors
    ///
    /// Returns a [`PolicyError`] naming the field when the declaration cannot be satisfied for this
    /// intent, whatever algorithm attempts it.
    pub fn validate(&self, intent: &ExecutionIntent) -> Result<(), PolicyError> {
        if self.preference == Some(ExecutionPreference::Passive)
            && intent.order_type == OrderType::Market
        {
            return Err(PolicyError::new(
                KEY_PREFERENCE,
                format!(
                    "{KEY_PREFERENCE}=passive is not possible for a market order, which crosses on arrival"
                ),
            ));
        }

        Ok(())
    }

    /// Returns the horizon in seconds, when one is declared.
    #[must_use]
    pub fn horizon_secs(&self) -> Option<f64> {
        self.horizon.map(|horizon| horizon.as_secs_f64())
    }
}

/// A policy field that was refused, with a message naming it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PolicyError {
    /// The policy field that was refused.
    pub field: Ustr,
    /// A message naming the field and why it was refused.
    pub detail: String,
}

impl PolicyError {
    /// Creates a new [`PolicyError`] instance.
    #[must_use]
    pub fn new(field: &str, detail: impl Into<String>) -> Self {
        Self {
            field: Ustr::from(field),
            detail: detail.into(),
        }
    }

    /// Returns the refusal of a policy part an algorithm cannot honour.
    #[must_use]
    pub fn unsupported(part: PolicyPart) -> Self {
        let key = part.key();
        Self::new(
            key,
            format!("{key} is not supported by this execution algorithm"),
        )
    }

    /// Returns the message a denial renders, naming the field.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.detail
    }
}

impl Display for PolicyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message())
    }
}

/// Returns `value` as a duration, refusing a value that is not finite and positive.
fn nanos_from_seconds(field: &str, value: f64) -> Result<DurationNanos, PolicyError> {
    if !value.is_finite() || value <= 0.0 {
        return Err(PolicyError::new(
            field,
            format!("{field}={value} must be finite and positive"),
        ));
    }

    let Ok(duration) = Duration::try_from_secs_f64(value) else {
        return Err(PolicyError::new(
            field,
            format!("{field}={value} exceeds the duration range"),
        ));
    };

    let Ok(nanos) = u64::try_from(duration.as_nanos()) else {
        return Err(PolicyError::new(
            field,
            format!("{field}={value} exceeds the nanosecond range"),
        ));
    };

    Ok(DurationNanos::new(nanos))
}

#[cfg(test)]
mod tests {
    use nautilus_model::{
        enums::{OrderSide, TimeInForce},
        orders::OrderTestBuilder,
    };

    use super::*;

    fn intent(order_type: OrderType) -> ExecutionIntent {
        let mut builder = OrderTestBuilder::new(order_type);
        builder
            .instrument_id(InstrumentId::from("ETHUSDT-PERP.BINANCE"))
            .side(OrderSide::Buy)
            .quantity(Quantity::from("1.0"))
            .time_in_force(TimeInForce::Gtc);

        if order_type == OrderType::Limit {
            builder.price(Price::from("100.00"));
        }

        ExecutionIntent::from(&builder.build())
    }

    fn params(entries: &[(&str, &str)]) -> IndexMap<Ustr, Ustr> {
        entries
            .iter()
            .map(|(key, value)| (Ustr::from(key), Ustr::from(value)))
            .collect()
    }

    #[rstest::rstest]
    fn test_the_intent_is_taken_from_the_order() {
        let intent = intent(OrderType::Market);

        assert_eq!(
            intent.instrument_id,
            InstrumentId::from("ETHUSDT-PERP.BINANCE")
        );
        assert_eq!(intent.order_side, OrderSide::Buy);
        assert_eq!(intent.order_type, OrderType::Market);
        assert_eq!(intent.quantity, Quantity::from("1.0"));
        assert_eq!(intent.price, None);
    }

    #[rstest::rstest]
    fn test_an_absent_policy_declares_nothing_and_refuses_nothing() {
        let policy = ExecutionPolicy::from_params(&params(&[])).unwrap();

        assert_eq!(policy, ExecutionPolicy::default());
        assert!(policy.declared_parts().is_empty());
        assert_eq!(policy.unsupported(&[]), None);
        assert!(policy.validate(&intent(OrderType::Market)).is_ok());
    }

    #[rstest::rstest]
    fn test_a_policy_parses_every_declared_part() {
        let policy = ExecutionPolicy::from_params(&params(&[
            (KEY_HORIZON_SECS, "900"),
            (KEY_PARTICIPATION_RATE, "0.15"),
            (KEY_PRICE_LIMIT, "100.50"),
            (KEY_MAX_SLIPPAGE_BPS, "15"),
            (KEY_PREFERENCE, "passive"),
            (KEY_URGENCY, "high"),
        ]))
        .unwrap();

        assert_eq!(policy.horizon, Some(DurationNanos::from_secs(900)));
        assert_eq!(policy.horizon_secs(), Some(900.0));
        assert_eq!(policy.participation_rate, Some(Decimal::new(15, 2)));
        assert_eq!(policy.price_limit, Some(Price::from("100.50")));
        assert_eq!(policy.max_slippage_bps, Some(Decimal::new(15, 0)));
        assert_eq!(policy.preference, Some(ExecutionPreference::Passive));
        assert_eq!(policy.urgency, Some(ExecutionUrgency::High));
        assert_eq!(policy.declared_parts(), PolicyPart::ALL.to_vec());
    }

    #[rstest::rstest]
    #[case::horizon(KEY_HORIZON_SECS, "soon", "horizon_secs=soon is not a valid number")]
    #[case::horizon_range(KEY_HORIZON_SECS, "-1", "horizon_secs=-1 must be finite and positive")]
    #[case::horizon_nan(
        KEY_HORIZON_SECS,
        "NaN",
        "horizon_secs=NaN must be finite and positive"
    )]
    #[case::rate(
        KEY_PARTICIPATION_RATE,
        "1.5",
        "participation_rate=1.5 must be greater than 0 and at most 1"
    )]
    #[case::rate_zero(
        KEY_PARTICIPATION_RATE,
        "0",
        "participation_rate=0 must be greater than 0 and at most 1"
    )]
    #[case::price(KEY_PRICE_LIMIT, "free", "price_limit=free is not a valid price")]
    #[case::slippage(KEY_MAX_SLIPPAGE_BPS, "-1", "max_slippage_bps=-1 must not be negative")]
    #[case::preference(
        KEY_PREFERENCE,
        "greedy",
        "preference=greedy must be one of passive or aggressive"
    )]
    #[case::urgency(KEY_URGENCY, "now", "urgency=now must be one of low, normal or high")]
    fn test_a_malformed_field_is_refused_and_named(
        #[case] key: &str,
        #[case] value: &str,
        #[case] expected: &str,
    ) {
        let err = ExecutionPolicy::from_params(&params(&[(key, value)])).unwrap_err();

        assert_eq!(err.field, Ustr::from(key));
        assert_eq!(err.message(), expected);
    }

    #[rstest::rstest]
    fn test_an_unsupported_part_is_named_in_declaration_order() {
        let policy = ExecutionPolicy::from_params(&params(&[
            (KEY_HORIZON_SECS, "900"),
            (KEY_URGENCY, "high"),
        ]))
        .unwrap();

        // Horizon is honoured, so the first part it cannot honour is the urgency.
        assert_eq!(
            policy.unsupported(&[PolicyPart::Horizon]),
            Some(PolicyPart::Urgency)
        );
        assert_eq!(
            PolicyError::unsupported(PolicyPart::Urgency).message(),
            "urgency is not supported by this execution algorithm"
        );
        assert_eq!(policy.unsupported(&PolicyPart::ALL), None);
    }

    #[rstest::rstest]
    fn test_a_passive_preference_is_refused_for_a_market_intent() {
        let policy = ExecutionPolicy::from_params(&params(&[(KEY_PREFERENCE, "passive")])).unwrap();

        let err = policy.validate(&intent(OrderType::Market)).unwrap_err();

        assert_eq!(err.field, Ustr::from(KEY_PREFERENCE));

        let aggressive =
            ExecutionPolicy::from_params(&params(&[(KEY_PREFERENCE, "aggressive")])).unwrap();

        assert!(aggressive.validate(&intent(OrderType::Market)).is_ok());
        assert!(policy.validate(&intent(OrderType::Limit)).is_ok());
    }

    #[rstest::rstest]
    fn test_every_part_key_round_trips_through_its_parameter() {
        for part in PolicyPart::ALL {
            assert!(!part.key().is_empty());
            assert_eq!(part.to_string(), part.key());
        }

        assert_eq!(
            ExecutionPolicy::from_params(&params(&[(KEY_PARTICIPATION_RATE, "1")]))
                .unwrap()
                .declared_parts(),
            vec![PolicyPart::ParticipationRate]
        );
        assert_eq!(
            ExecutionPolicy::from_params(&params(&[(KEY_MAX_SLIPPAGE_BPS, "0")]))
                .unwrap()
                .declared_parts(),
            vec![PolicyPart::SlippageLimit]
        );
    }
}
