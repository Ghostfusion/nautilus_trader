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

//! Model-agnostic option pricing.
//!
//! This module defines the shared parameter set and the [`OptionPricingModel`] interface used to
//! price a vanilla option, and [`price_option`] which selects the pricing model from the *declared*
//! [`ExerciseStyle`]. Style selection is deliberately explicit: a chain must not be assumed to use
//! a single exercise style, so the caller supplies the style per contract.
//!
//! The European price is the repository's own closed form (see [`black_scholes_greeks_exact`]);
//! the American price uses the Cox-Ross-Rubinstein binomial tree in
//! [`super::binomial::CoxRossRubinstein`].

use super::{binomial::CoxRossRubinstein, greeks::black_scholes_greeks_exact};
use crate::enums::{ExerciseStyle, OptionKind};

/// The parameters for pricing a single vanilla option.
///
/// The cost of carry `b` follows the generalized Black-Scholes convention: `b = r - q` for an
/// option on an asset with continuous dividend yield `q`, and `b = 0` for an option on a futures
/// contract (Black-76).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OptionPricingParams {
    /// The underlying spot price.
    pub spot: f64,
    /// The option strike price.
    pub strike: f64,
    /// The continuously compounded risk-free interest rate.
    pub risk_free_rate: f64,
    /// The cost of carry (`b`), equal to `risk_free_rate` minus the dividend yield.
    pub cost_of_carry: f64,
    /// The volatility of the underlying (annualized, as a decimal).
    pub volatility: f64,
    /// The time to expiry in years.
    pub time_to_expiry: f64,
    /// The kind of option (call or put).
    pub option_kind: OptionKind,
    /// The declared exercise style of the option.
    pub exercise_style: ExerciseStyle,
}

/// An error returned while pricing an option.
#[derive(Clone, Copy, Debug, PartialEq, thiserror::Error)]
pub enum PricingError {
    /// A pricing parameter was not finite, or was outside its valid domain.
    #[error("invalid option pricing parameter `{name}`: {value}")]
    InvalidParameter {
        /// The name of the invalid parameter.
        name: &'static str,
        /// The invalid value.
        value: f64,
    },
    /// The risk-neutral up/down probability fell outside the interval `[0, 1]`.
    #[error("risk-neutral probability {probability} is outside [0, 1] for the given parameters")]
    InvalidProbability {
        /// The invalid risk-neutral probability.
        probability: f64,
    },
}

/// A model which can price a vanilla option from a shared [`OptionPricingParams`].
pub trait OptionPricingModel {
    /// Returns the name of the pricing model.
    fn name(&self) -> &'static str;

    /// Returns the price of the option described by `params`.
    ///
    /// # Errors
    ///
    /// Returns a [`PricingError`] if the parameters are invalid or the model cannot be applied.
    fn price(&self, params: &OptionPricingParams) -> Result<f64, PricingError>;
}

/// Prices a vanilla option, selecting the model from the declared [`ExerciseStyle`].
///
/// European exercise uses the repository's closed-form Black-Scholes price; American exercise uses
/// a [`CoxRossRubinstein`] binomial tree with the default step count.
///
/// Expired options (`time_to_expiry <= 0`) return the intrinsic value, and zero-volatility options
/// return the discounted forward intrinsic, for either exercise style.
///
/// # Errors
///
/// Returns a [`PricingError`] if the parameters are invalid.
pub fn price_option(params: &OptionPricingParams) -> Result<f64, PricingError> {
    validate_params(params)?;

    if params.time_to_expiry <= 0.0 {
        return Ok(intrinsic(params));
    }

    if params.volatility == 0.0 {
        return Ok(discounted_forward_intrinsic(params));
    }

    match params.exercise_style {
        ExerciseStyle::European => Ok(european_price(params)),
        ExerciseStyle::American => CoxRossRubinstein::default().price(params),
    }
}

/// Estimates the forward price implied by put-call parity.
///
/// With continuously compounded interest at `risk_free_rate` and time to expiry `time_to_expiry`,
/// put-call parity for European options gives `C - P = exp(-r * T) * (F - K)`, so
/// `F = K + (C - P) * exp(r * T)`.
///
/// The estimate assumes European exercise, a single expiry, no dividends or other carry costs
/// beyond those embedded in the observed prices, no transaction costs, and a single
/// risk-free rate for borrowing and lending. It is therefore unsuitable for American options,
/// where early exercise can break parity.
#[must_use]
pub fn implied_forward_from_parity(
    call_price: f64,
    put_price: f64,
    strike: f64,
    risk_free_rate: f64,
    time_to_expiry: f64,
) -> f64 {
    strike + (call_price - put_price) * (risk_free_rate * time_to_expiry).exp()
}

/// Returns the intrinsic value of the option at the current spot.
pub(crate) fn intrinsic(params: &OptionPricingParams) -> f64 {
    payoff(params.spot, params.strike, params.option_kind)
}

/// Returns the discounted intrinsic value of the option at the forward.
pub(crate) fn discounted_forward_intrinsic(params: &OptionPricingParams) -> f64 {
    let forward = params.spot * (params.cost_of_carry * params.time_to_expiry).exp();
    payoff(forward, params.strike, params.option_kind)
        * (-params.risk_free_rate * params.time_to_expiry).exp()
}

/// Validates the common domain of the pricing parameters.
pub(crate) fn validate_params(params: &OptionPricingParams) -> Result<(), PricingError> {
    for (name, value) in [
        ("spot", params.spot),
        ("strike", params.strike),
        ("risk_free_rate", params.risk_free_rate),
        ("cost_of_carry", params.cost_of_carry),
        ("volatility", params.volatility),
        ("time_to_expiry", params.time_to_expiry),
    ] {
        if !value.is_finite() {
            return Err(PricingError::InvalidParameter { name, value });
        }
    }

    if params.spot <= 0.0 {
        return Err(PricingError::InvalidParameter {
            name: "spot",
            value: params.spot,
        });
    }

    if params.strike <= 0.0 {
        return Err(PricingError::InvalidParameter {
            name: "strike",
            value: params.strike,
        });
    }

    if params.volatility < 0.0 {
        return Err(PricingError::InvalidParameter {
            name: "volatility",
            value: params.volatility,
        });
    }

    if params.time_to_expiry < 0.0 {
        return Err(PricingError::InvalidParameter {
            name: "time_to_expiry",
            value: params.time_to_expiry,
        });
    }

    Ok(())
}

/// Returns the option payoff at expiry for the given underlying price.
pub(crate) fn payoff(spot: f64, strike: f64, option_kind: OptionKind) -> f64 {
    match option_kind {
        OptionKind::Call => (spot - strike).max(0.0),
        OptionKind::Put => (strike - spot).max(0.0),
    }
}

fn european_price(params: &OptionPricingParams) -> f64 {
    let is_call = matches!(params.option_kind, OptionKind::Call);

    black_scholes_greeks_exact(
        params.spot,
        params.risk_free_rate,
        params.cost_of_carry,
        params.volatility,
        is_call,
        params.strike,
        params.time_to_expiry,
    )
    .price
}
