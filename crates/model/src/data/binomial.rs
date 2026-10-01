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

//! Binomial-tree option pricing.
//!
//! Implements the Cox-Ross-Rubinstein (CRR) binomial tree, which prices both American and European
//! vanilla options under the generalized Black-Scholes cost-of-carry framework.

use super::pricing::{
    OptionPricingModel, OptionPricingParams, PricingError, discounted_forward_intrinsic, intrinsic,
    payoff, validate_params,
};
use crate::enums::ExerciseStyle;

/// The default number of steps used by [`CoxRossRubinstein::default`].
pub const DEFAULT_STEPS: usize = 512;

/// The Cox-Ross-Rubinstein binomial tree pricing model.
///
/// The tree uses `dt = T / N`, `u = exp(sigma * sqrt(dt))`, `d = 1 / u`, the risk-neutral
/// probability `p = (exp(b * dt) - d) / (u - d)` and per-step discounting `exp(-r * dt)`, where
/// `b` is the cost of carry. With `b = 0` the tree reproduces Black-76 (an option on a future),
/// and with `b = r - q` it reproduces Black-Scholes with a continuous dividend yield `q`.
///
/// Backward induction takes the maximum of the intrinsic value and the continuation value at every
/// node for [`ExerciseStyle::American`], and the continuation value only for
/// [`ExerciseStyle::European`], so a European tree is priced the same way as the closed form.
///
/// # Convergence
///
/// For a European option the CRR price converges to the Black-Scholes price as `O(1 / N)` with an
/// oscillating error term (the parity of `N` shifts the error sign), so the error is not
/// monotonic in `N` in general. The integration tests bound the absolute error against the
/// repository's closed form at named step counts and assert it decreases across them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CoxRossRubinstein {
    /// The number of time steps in the tree.
    pub steps: usize,
}

impl Default for CoxRossRubinstein {
    fn default() -> Self {
        Self {
            steps: DEFAULT_STEPS,
        }
    }
}

impl OptionPricingModel for CoxRossRubinstein {
    fn name(&self) -> &'static str {
        "CoxRossRubinstein"
    }

    fn price(&self, params: &OptionPricingParams) -> Result<f64, PricingError> {
        validate_params(params)?;

        if params.time_to_expiry <= 0.0 {
            return Ok(intrinsic(params));
        }

        if params.volatility == 0.0 || self.steps == 0 {
            return Ok(discounted_forward_intrinsic(params));
        }

        let OptionPricingParams {
            spot,
            strike,
            risk_free_rate: r,
            cost_of_carry: b,
            volatility,
            time_to_expiry: t,
            option_kind,
            exercise_style,
        } = *params;

        let steps = self.steps;
        let dt = t / steps as f64;
        let sqrt_dt = dt.sqrt();
        let u = (volatility * sqrt_dt).exp();
        let d = 1.0 / u;
        let growth = (b * dt).exp();
        let p = (growth - d) / (u - d);

        if !p.is_finite() || !(0.0..=1.0).contains(&p) {
            return Err(PricingError::InvalidProbability { probability: p });
        }

        let discount = (-r * dt).exp();
        let ratio = u * u; // u / d
        let american = matches!(exercise_style, ExerciseStyle::American);

        // Terminal payoffs: node `j` has `j` up-moves, so `S_j = S * d^N * (u / d)^j`.
        let mut values = vec![0.0; steps + 1];
        let mut node_spot = spot * d.powi(steps as i32);

        for value in &mut values {
            *value = payoff(node_spot, strike, option_kind);
            node_spot *= ratio;
        }

        // Backward induction over the tree.
        for step in (0..steps).rev() {
            let mut node_spot = spot * d.powi(step as i32);

            for j in 0..=step {
                let continuation = discount * (p * values[j + 1] + (1.0 - p) * values[j]);

                values[j] = if american {
                    continuation.max(payoff(node_spot, strike, option_kind))
                } else {
                    continuation
                };

                node_spot *= ratio;
            }
        }

        Ok(values[0])
    }
}
