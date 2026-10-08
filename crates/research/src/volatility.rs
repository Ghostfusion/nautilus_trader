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

//! A model-free volatility index built from one option-chain snapshot.
//!
//! The index answers one narrow question: how much movement do the option prices in a single
//! snapshot imply for the underlying instrument over a stated horizon? It is model-free in the
//! sense that it never solves for an option model and never reads an implied volatility from a
//! quote. It sums the prices of the out-of-the-money options, each weighted by the spacing of the
//! strikes around it, which is the discrete form of the fair value of a variance swap; the
//! published equity index constructed this way is the VIX.
//!
//! Two terms are needed because one expiry rarely lands on the horizon a reader cares about. Each
//! term yields a variance over its own time to expiry, and the two variances are weighted by how
//! far each expiry is from the target horizon.
//!
//! # Conventions
//!
//! The construction is only reproducible if each step has one stated meaning, so the conventions
//! are fixed here:
//!
//! - A term is one expiry: a set of strike quotes and the time to that expiry in years. Only the
//!   quotes present are read. No price is interpolated, no strike is invented, and no quote is
//!   carried across from another snapshot.
//! - The forward level is taken from the strike whose call and put prices are closest to each
//!   other, as `K + (C - P) * exp(r * T)`, the cost of carrying the difference to expiry. The
//!   lowest strike wins a tie between two equally close pairs, so the level does not depend on the
//!   order the quotes arrive in.
//! - The reference strike is the highest listed strike at or below the forward level.
//! - The out-of-the-money stripe takes the put at every strike below the reference strike and the
//!   call at every strike above it, and the average of the two at the reference strike. A strike
//!   that does not quote the leg the stripe needs is skipped, and a term with an empty stripe is
//!   an error rather than a zero.
//! - The strike spacing of a stripe entry is half the distance to its two neighbouring stripe
//!   entries; the two outermost entries use the one-sided distance to their single neighbour.
//! - The term variance is
//!   `2 / T * sum(dK / K^2 * exp(r * T) * Q(K)) - 1 / T * (F / K0 - 1)^2`, where the second term
//!   removes the cost of carrying the difference between the forward level and the reference
//!   strike.
//! - The two term variances are blended by their distance from the target horizon and divided by
//!   the target horizon in years. The published construction writes the same expression as a ratio
//!   of trading days; this module states it in years so that one unit of time is used throughout.
//! - A variance that is not positive is an error. An undefined statistic is never reported as
//!   zero.
//! - The index is annualised and returned in percentage points, so an annualised variance of
//!   `0.04` is the index level `20.00`.
//!
//! # Example
//!
//! ```text
//! near: 20 days to expiry, next: 40 days to expiry, target: 30 days;
//! the weights are one half each, so a target horizon midway between the terms
//! returns the common term variance.
//! ```

/// One strike of an option chain, with the prices quoted at the snapshot.
///
/// A leg is [`None`] when the snapshot carries no quote for it, which is what makes the
/// out-of-the-money stripe a selection rather than an assumption.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrikeQuote {
    /// The strike price.
    pub strike: f64,
    /// The price of the call at this strike, if it was quoted.
    pub call: Option<f64>,
    /// The price of the put at this strike, if it was quoted.
    pub put: Option<f64>,
}

impl StrikeQuote {
    /// Returns a strike quote.
    #[must_use]
    pub const fn new(strike: f64, call: Option<f64>, put: Option<f64>) -> Self {
        Self { strike, call, put }
    }
}

/// One expiry of an option chain: the quotes and the time to that expiry.
#[derive(Clone, Debug, PartialEq)]
pub struct OptionChainTerm {
    /// The time to expiry in years, which is positive.
    pub time_to_expiry_years: f64,
    /// The quoted strikes, in any order.
    pub quotes: Vec<StrikeQuote>,
}

impl OptionChainTerm {
    /// Returns a term of an option chain.
    #[must_use]
    pub const fn new(time_to_expiry_years: f64, quotes: Vec<StrikeQuote>) -> Self {
        Self {
            time_to_expiry_years,
            quotes,
        }
    }
}

/// The variance one expiry implies, and the two levels it was taken from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TermVariance {
    /// The forward level of the underlying, from the strike whose legs are closest in price.
    pub forward_level: f64,
    /// The reference strike: the highest listed strike at or below the forward level.
    pub reference_strike: f64,
    /// The annualised variance over this term's time to expiry.
    pub variance: f64,
}

/// An error building a volatility index from an option-chain snapshot.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum VolatilityError {
    /// The snapshot carries no strikes at all.
    #[error("the chain snapshot carries no strikes")]
    EmptyChain,
    /// The time to expiry is not positive and finite.
    #[error("the time to expiry must be positive and finite, was {0}")]
    InvalidExpiry(f64),
    /// A strike is not positive and finite.
    #[error("a strike must be positive and finite, was {0}")]
    InvalidStrike(f64),
    /// A quoted price is not finite.
    #[error("a quoted price at strike {strike} must be finite, was {price}")]
    NonFiniteQuote {
        /// The strike carrying the quote.
        strike: f64,
        /// The quoted price.
        price: f64,
    },
    /// One strike was quoted twice, so the snapshot is ambiguous.
    #[error("strike {0} is quoted more than once")]
    DuplicateStrike(f64),
    /// No strike quotes both legs, so no forward level can be taken.
    #[error("no strike quotes both a call and a put, so no forward level can be taken")]
    NoForwardLevel,
    /// No listed strike lies at or below the forward level.
    #[error("no listed strike lies at or below the forward level {0}")]
    NoReferenceStrike(f64),
    /// The out-of-the-money stripe is empty.
    #[error("no out-of-the-money option is quoted around the reference strike {0}")]
    EmptyStripe(f64),
    /// The implied variance is not positive.
    #[error("the implied variance is not positive, it is {0}")]
    NonPositiveVariance(f64),
    /// The risk-free rate is not finite.
    #[error("the risk-free rate must be finite, was {0}")]
    InvalidRate(f64),
    /// The target horizon does not lie between the two expiries.
    #[error("the target horizon {target} must lie between the expiries {near} and {next}")]
    TargetOutsideTerms {
        /// The target horizon in years.
        target: f64,
        /// The near time to expiry in years.
        near: f64,
        /// The next time to expiry in years.
        next: f64,
    },
}

/// Returns the quotes sorted by strike, rejecting a snapshot that cannot be read.
fn sorted_quotes(quotes: &[StrikeQuote]) -> Result<Vec<&StrikeQuote>, VolatilityError> {
    if quotes.is_empty() {
        return Err(VolatilityError::EmptyChain);
    }

    for quote in quotes {
        if !quote.strike.is_finite() || quote.strike <= 0.0 {
            return Err(VolatilityError::InvalidStrike(quote.strike));
        }
        for price in [quote.call, quote.put].into_iter().flatten() {
            if !price.is_finite() {
                return Err(VolatilityError::NonFiniteQuote {
                    strike: quote.strike,
                    price,
                });
            }
        }
    }

    let mut sorted: Vec<&StrikeQuote> = quotes.iter().collect();
    sorted.sort_by(|left, right| left.strike.total_cmp(&right.strike));

    for pair in sorted.windows(2) {
        if pair[0].strike == pair[1].strike {
            return Err(VolatilityError::DuplicateStrike(pair[0].strike));
        }
    }

    Ok(sorted)
}

/// Returns the forward level: the strike whose legs are closest in price, carried to expiry.
fn forward_level(
    quotes: &[&StrikeQuote],
    risk_free_rate: f64,
    time_to_expiry_years: f64,
) -> Result<f64, VolatilityError> {
    let mut closest: Option<(f64, f64)> = None;

    for quote in quotes {
        let (Some(call), Some(put)) = (quote.call, quote.put) else {
            continue;
        };
        let difference = (call - put).abs();
        match closest {
            Some((best, _)) if difference >= best => {}
            _ => {
                let carried = (call - put) * (risk_free_rate * time_to_expiry_years).exp();
                closest = Some((difference, quote.strike + carried));
            }
        }
    }

    match closest {
        Some((_, forward)) => Ok(forward),
        None => Err(VolatilityError::NoForwardLevel),
    }
}

/// Returns the reference strike: the highest listed strike at or below the forward level.
fn reference_strike(quotes: &[&StrikeQuote], forward: f64) -> Result<f64, VolatilityError> {
    quotes
        .iter()
        .rev()
        .find(|quote| quote.strike <= forward)
        .map(|quote| quote.strike)
        .ok_or(VolatilityError::NoReferenceStrike(forward))
}

/// Returns the out-of-the-money stripe as paired strikes and prices.
fn out_of_the_money_stripe(
    quotes: &[&StrikeQuote],
    reference: f64,
) -> Result<Vec<(f64, f64)>, VolatilityError> {
    let mut stripe = Vec::with_capacity(quotes.len());

    for quote in quotes {
        let price = match quote.strike.total_cmp(&reference) {
            std::cmp::Ordering::Less => quote.put,
            std::cmp::Ordering::Greater => quote.call,
            std::cmp::Ordering::Equal => match (quote.call, quote.put) {
                (Some(call), Some(put)) => Some((call + put) / 2.0),
                _ => None,
            },
        };
        if let Some(price) = price {
            stripe.push((quote.strike, price));
        }
    }

    if stripe.is_empty() {
        return Err(VolatilityError::EmptyStripe(reference));
    }

    Ok(stripe)
}

/// Returns the strike spacing of one stripe entry: half the distance to its neighbours.
fn strike_spacing(stripe: &[(f64, f64)], index: usize) -> f64 {
    let (strike, _) = stripe[index];
    match (index.checked_sub(1), stripe.get(index + 1)) {
        (Some(previous), Some(next)) => (next.0 - stripe[previous].0) / 2.0,
        (Some(previous), None) => strike - stripe[previous].0,
        (None, Some(next)) => next.0 - strike,
        (None, None) => 0.0,
    }
}

/// Returns the variance one expiry implies, from one snapshot of its chain.
///
/// # Errors
///
/// Returns an error when the time to expiry is not positive and finite, when a strike is not
/// positive and finite, when a quoted price is not finite, when a strike is quoted twice, when no
/// strike quotes both legs, when no listed strike lies at or below the forward level, when no
/// out-of-the-money option is quoted, or when the implied variance is not positive.
pub fn term_variance(
    term: &OptionChainTerm,
    risk_free_rate: f64,
) -> Result<TermVariance, VolatilityError> {
    let time = term.time_to_expiry_years;
    if !time.is_finite() || time <= 0.0 {
        return Err(VolatilityError::InvalidExpiry(time));
    }
    if !risk_free_rate.is_finite() {
        return Err(VolatilityError::InvalidRate(risk_free_rate));
    }

    let quotes = sorted_quotes(&term.quotes)?;
    let forward = forward_level(&quotes, risk_free_rate, time)?;
    let reference = reference_strike(&quotes, forward)?;
    let stripe = out_of_the_money_stripe(&quotes, reference)?;
    let discount = (risk_free_rate * time).exp();

    let mut weighted = 0.0;
    for (index, (strike, price)) in stripe.iter().enumerate() {
        let spacing = strike_spacing(&stripe, index);
        weighted += spacing / (strike * strike) * discount * price;
    }

    let variance = 2.0 / time * weighted - (forward / reference - 1.0).powi(2) / time;
    if variance <= 0.0 {
        return Err(VolatilityError::NonPositiveVariance(variance));
    }

    Ok(TermVariance {
        forward_level: forward,
        reference_strike: reference,
        variance,
    })
}

/// Returns the annualised volatility index for a target horizon, in percentage points.
///
/// The near term must expire before the target horizon and the next term after it. Each term
/// variance is weighted by how far its expiry is from the target, and the weighted sum is divided
/// by the target horizon in years.
///
/// # Errors
///
/// Returns an error when either term cannot be read, or when the target horizon does not lie
/// strictly between the two times to expiry.
pub fn volatility_index(
    near: &OptionChainTerm,
    next: &OptionChainTerm,
    risk_free_rate: f64,
    target_years: f64,
) -> Result<f64, VolatilityError> {
    let (near_years, next_years) = (near.time_to_expiry_years, next.time_to_expiry_years);
    if !target_years.is_finite()
        || target_years <= near_years
        || target_years >= next_years
        || near_years <= 0.0
    {
        return Err(VolatilityError::TargetOutsideTerms {
            target: target_years,
            near: near_years,
            next: next_years,
        });
    }

    let near_variance = term_variance(near, risk_free_rate)?.variance;
    let next_variance = term_variance(next, risk_free_rate)?.variance;

    let span = next_years - near_years;
    let near_weight = (next_years - target_years) / span;
    let next_weight = (target_years - near_years) / span;
    let blended = (near_years * near_variance * near_weight
        + next_years * next_variance * next_weight)
        / target_years;

    if blended <= 0.0 {
        return Err(VolatilityError::NonPositiveVariance(blended));
    }

    Ok(100.0 * blended.sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A five-strike chain whose forward level lands on the 100 strike, so the correction term
    /// vanishes: every strike quotes the leg the stripe needs, and the spacing is 5 throughout.
    fn centred_chain(time: f64) -> OptionChainTerm {
        OptionChainTerm::new(
            time,
            vec![
                StrikeQuote::new(90.0, None, Some(1.0)),
                StrikeQuote::new(95.0, None, Some(2.0)),
                StrikeQuote::new(100.0, Some(3.0), Some(3.0)),
                StrikeQuote::new(105.0, Some(2.0), None),
                StrikeQuote::new(110.0, Some(1.0), None),
            ],
        )
    }

    #[test]
    fn variance_sums_the_weighted_out_of_the_money_prices() {
        // sum = 5/90^2*1 + 5/95^2*2 + 5/100^2*3 + 5/105^2*2 + 5/110^2*1
        //     = 0.000617283950617 + 0.001108033240997 + 0.0015 + 0.000907029478458
        //       + 0.000413223140496
        //     = 0.004545569810568
        // forward equals the reference strike, so the correction term is zero and the variance is
        // 2 * sum over one year.
        let expected =
            2.0 * (5.0 / 8100.0 + 10.0 / 9025.0 + 15.0 / 10000.0 + 10.0 / 11025.0 + 5.0 / 12100.0);
        let variance = term_variance(&centred_chain(1.0), 0.0).unwrap();

        assert!((variance.variance - expected).abs() < 1e-15);
        assert!((variance.variance - 0.009091139621136).abs() < 1e-12);
        assert_eq!(variance.reference_strike, 100.0);
        assert_eq!(variance.forward_level, 100.0);
    }

    #[test]
    fn the_reference_strike_is_the_listed_strike_below_the_forward_level() {
        // The 100 strike quotes a call 0.5 above its put, so the forward level is 100.5 and the
        // reference strike stays at 100. The correction term is (100.5/100 - 1)^2 / 1 = 0.000025.
        let chain = OptionChainTerm::new(
            1.0,
            vec![
                StrikeQuote::new(95.0, None, Some(2.0)),
                StrikeQuote::new(100.0, Some(3.5), Some(3.0)),
                StrikeQuote::new(105.0, Some(1.0), None),
            ],
        );
        let variance = term_variance(&chain, 0.0).unwrap();

        assert_eq!(variance.forward_level, 100.5);
        assert_eq!(variance.reference_strike, 100.0);
        // sum = 5/95^2*2 + 5/100^2*3.25 + 5/105^2*1 = 0.001108033240997 + 0.001625 +
        // 0.000453514739229 = 0.003186547980226; variance = 2*sum - 0.000025.
        assert!((variance.variance - (2.0 * 0.003186547980226 - 0.000025)).abs() < 1e-12);
    }

    #[test]
    fn the_carry_of_the_leg_difference_scales_the_forward_level() {
        // The same chain at a five percent rate over one year carries the 0.5 difference to
        // 0.5 * exp(0.05) = 0.525634905, so the forward level is 100.525634905.
        let chain = OptionChainTerm::new(
            1.0,
            vec![
                StrikeQuote::new(95.0, None, Some(2.0)),
                StrikeQuote::new(100.0, Some(3.5), Some(3.0)),
                StrikeQuote::new(105.0, Some(1.0), None),
            ],
        );
        let variance = term_variance(&chain, 0.05).unwrap();

        assert!((variance.forward_level - (100.0 + 0.5 * (0.05f64).exp())).abs() < 1e-12);
        assert_eq!(variance.reference_strike, 100.0);
    }

    #[test]
    fn a_target_midway_between_the_terms_returns_the_variance_at_the_target() {
        // With a zero rate the weighted sum of the two term variances collapses to the variance a
        // single term expiring at the target would carry: t1*(2S/t1)*w1 + t2*(2S/t2)*w2 = 2S when
        // the two weights sum to one, and dividing by the target horizon gives 2S/target.
        let near = centred_chain(20.0 / 365.0);
        let next = centred_chain(40.0 / 365.0);
        let at_target = term_variance(&centred_chain(30.0 / 365.0), 0.0)
            .unwrap()
            .variance;
        let index = volatility_index(&near, &next, 0.0, 30.0 / 365.0).unwrap();

        assert!((index - 100.0 * at_target.sqrt()).abs() < 1e-12);
    }

    #[test]
    fn a_horizon_outside_the_two_expiries_is_an_error() {
        let near = centred_chain(20.0 / 365.0);
        let next = centred_chain(40.0 / 365.0);

        for target in [10.0 / 365.0, 40.0 / 365.0, 60.0 / 365.0] {
            assert!(matches!(
                volatility_index(&near, &next, 0.0, target),
                Err(VolatilityError::TargetOutsideTerms { .. })
            ));
        }
    }

    #[test]
    fn a_snapshot_that_cannot_be_read_is_an_error() {
        assert_eq!(
            term_variance(&OptionChainTerm::new(1.0, vec![]), 0.0),
            Err(VolatilityError::EmptyChain)
        );
        assert_eq!(
            term_variance(&centred_chain(0.0), 0.0),
            Err(VolatilityError::InvalidExpiry(0.0))
        );
        assert_eq!(
            term_variance(
                &OptionChainTerm::new(1.0, vec![StrikeQuote::new(-5.0, Some(1.0), Some(1.0))]),
                0.0
            ),
            Err(VolatilityError::InvalidStrike(-5.0))
        );
        assert_eq!(
            term_variance(
                &OptionChainTerm::new(
                    1.0,
                    vec![StrikeQuote::new(100.0, Some(f64::INFINITY), None)]
                ),
                0.0
            ),
            Err(VolatilityError::NonFiniteQuote {
                strike: 100.0,
                price: f64::INFINITY,
            })
        );
        assert_eq!(
            term_variance(
                &OptionChainTerm::new(
                    1.0,
                    vec![
                        StrikeQuote::new(100.0, Some(1.0), None),
                        StrikeQuote::new(100.0, Some(2.0), None),
                    ]
                ),
                0.0
            ),
            Err(VolatilityError::DuplicateStrike(100.0))
        );
        // Every strike quotes one leg only, so no forward level can be taken.
        assert_eq!(
            term_variance(
                &OptionChainTerm::new(
                    1.0,
                    vec![
                        StrikeQuote::new(95.0, None, Some(2.0)),
                        StrikeQuote::new(105.0, Some(1.0), None),
                    ]
                ),
                0.0
            ),
            Err(VolatilityError::NoForwardLevel)
        );
        // The put is worth more than the call at the only two-legged strike, so the forward level
        // is carried below the lowest listed strike and no reference strike exists.
        assert_eq!(
            term_variance(
                &OptionChainTerm::new(
                    1.0,
                    vec![
                        StrikeQuote::new(100.0, Some(1.0), Some(3.0)),
                        StrikeQuote::new(105.0, Some(0.5), None),
                    ]
                ),
                0.0
            ),
            Err(VolatilityError::NoReferenceStrike(98.0))
        );
    }

    #[test]
    fn a_single_quoted_strike_has_no_spacing_and_no_variance() {
        // One strike only, so there is no neighbouring strike to measure a spacing against; the
        // weighted sum stays at zero and the variance cannot be positive.
        let chain = OptionChainTerm::new(1.0, vec![StrikeQuote::new(100.0, Some(3.0), Some(3.0))]);
        assert!(matches!(
            term_variance(&chain, 0.0),
            Err(VolatilityError::NonPositiveVariance(_))
        ));
    }

    #[test]
    fn a_rate_that_is_not_finite_is_an_error() {
        assert!(matches!(
            term_variance(&centred_chain(1.0), f64::INFINITY),
            Err(VolatilityError::InvalidRate(_))
        ));
    }
}
