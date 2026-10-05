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

//! Synthetic order-flow generator calibrated to a target Hurst exponent and impact exponent.
//!
//! The generator draws a persistent per-period signed flow from a truncated fractional
//! moving-average kernel and induces a price path from that flow under the declared impact
//! exponent. It exists so that a backtest can be run against flow whose long memory is known by
//! construction, rather than against historical data whose persistence is only asserted.
//!
//! The persistence comes from the fractional differencing parameter `d = H - 0.5`; the induced
//! price path is kept diffusive by refusing an impact exponent above one half, because a concave
//! impact response to a persistent flow is what stops the flow's memory from becoming a
//! predictable return (W3.3). The draws are deterministic under the configured seed: the same
//! configuration always yields the same flow.

use anyhow::{Result, bail};
use rand::{RngExt, SeedableRng, rngs::StdRng};

/// The maximum number of moving-average coefficients retained by the generator.
///
/// The fractional kernel is summable, so truncating the tail at this length leaves the normalized
/// flow effectively unchanged while bounding the per-period work.
const MAX_TRUNCATION: usize = 256;

/// Configuration for a [`SyntheticFlow`] calibrated to a target Hurst exponent and impact
/// exponent.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.backtest", skip_from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.backtest")
)]
pub struct SyntheticFlowConfig {
    /// The target Hurst exponent of the flow, strictly between 0.5 and 1.0.
    target_hurst: f64,
    /// The impact exponent applied to the flow, within `(0.0, 0.5]`.
    impact_exponent: f64,
    /// The number of periods generated.
    count: usize,
    /// The seed for the deterministic white-noise draws.
    seed: u64,
}

impl SyntheticFlowConfig {
    /// Creates a new [`SyntheticFlowConfig`], validating the calibration parameters.
    ///
    /// # Errors
    ///
    /// Returns an error if `target_hurst` is not finite and strictly between 0.5 and 1.0
    /// exclusive, if `impact_exponent` is not finite and within the half-open range `(0.0, 0.5]`,
    /// or if `count` is below 2.
    pub fn new(target_hurst: f64, impact_exponent: f64, count: usize, seed: u64) -> Result<Self> {
        if !target_hurst.is_finite() || target_hurst <= 0.5 || target_hurst >= 1.0 {
            bail!(
                "target_hurst must be finite and strictly between 0.5 and 1.0 exclusive, because 0.5 is the memoryless boundary and the generator exists to produce a persistent flow, was {target_hurst}"
            );
        }
        if !impact_exponent.is_finite() || impact_exponent <= 0.0 || impact_exponent > 0.5 {
            bail!(
                "impact_exponent must be finite and in the half-open range (0.0, 0.5]; a concave impact response keeps the induced prices diffusive, so an exponent above one half is refused per the diffusiveness condition, was {impact_exponent}"
            );
        }
        if count < 2 {
            bail!(
                "count must be at least 2, because a flow of fewer periods carries no memory to generate, was {count}"
            );
        }
        Ok(Self {
            target_hurst,
            impact_exponent,
            count,
            seed,
        })
    }

    /// Returns the target Hurst exponent of the flow.
    #[must_use]
    pub const fn target_hurst(&self) -> f64 {
        self.target_hurst
    }

    /// Returns the impact exponent applied to the flow.
    #[must_use]
    pub const fn impact_exponent(&self) -> f64 {
        self.impact_exponent
    }

    /// Returns the number of periods generated.
    #[must_use]
    pub const fn count(&self) -> usize {
        self.count
    }

    /// Returns the seed for the deterministic white-noise draws.
    #[must_use]
    pub const fn seed(&self) -> u64 {
        self.seed
    }

    /// Generates a [`SyntheticFlow`] from this configuration.
    ///
    /// The white noise is drawn from `rand::rngs::StdRng` seeded with
    /// [`StdRng::seed_from_u64`], the same generator the fill models use, as a standard uniform
    /// draw over `[-1.0, 1.0)`. A uniform draw is a deliberate, documented choice: `rand_distr`
    /// is not a dependency of `nautilus-backtest`, and the moving-average kernel, not the
    /// marginal distribution of the innovations, is what carries the persistence.
    ///
    /// The induced price path uses a unit impact coefficient of 1.0, so the price change over a
    /// period is exactly `sign(flow) * |flow|^impact_exponent`.
    ///
    /// # Errors
    ///
    /// Returns an error only if the flow cannot be generated; the configuration was validated at
    /// construction, so this is currently infallible.
    #[expect(
        clippy::cast_precision_loss,
        reason = "the coefficient index is bounded by the truncation length and counts are small"
    )]
    pub fn generate(&self) -> Result<SyntheticFlow> {
        let d = self.target_hurst - 0.5;
        let truncation = self.count.min(MAX_TRUNCATION);

        // Truncated fractional moving-average coefficients: psi[0] = 1 and
        // psi[k] = psi[k-1] * (k - 1 + d) / k.
        let mut psi = Vec::with_capacity(truncation + 1);
        let mut coefficient = 1.0;
        psi.push(coefficient);
        for k in 1..=truncation {
            coefficient *= (k as f64 - 1.0 + d) / k as f64;
            psi.push(coefficient);
        }

        // Normalize by the kernel energy so the flow has unit marginal variance regardless of
        // the target Hurst exponent.
        let normalization = psi.iter().map(|psi| psi * psi).sum::<f64>().sqrt();

        let mut rng = StdRng::seed_from_u64(self.seed);
        let noise = (0..self.count)
            .map(|_| rng.random_range(-1.0..1.0))
            .collect::<Vec<f64>>();

        let mut quantities = Vec::with_capacity(self.count);
        for t in 0..self.count {
            let upper = t.min(truncation);
            let mut raw = 0.0;
            for k in 0..=upper {
                raw += psi[k] * noise[t - k];
            }
            quantities.push(raw / normalization);
        }

        let mut prices = Vec::with_capacity(self.count);
        prices.push(0.0);
        for t in 1..self.count {
            let flow = quantities[t];
            let impact = flow.signum() * flow.abs().powf(self.impact_exponent);
            prices.push(prices[t - 1] + impact);
        }

        Ok(SyntheticFlow {
            quantities,
            prices,
            target_hurst: self.target_hurst,
            impact_exponent: self.impact_exponent,
            seed: self.seed,
        })
    }
}

/// A generated synthetic flow: the per-period signed flow and the price path it induces.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.backtest", skip_from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.backtest")
)]
pub struct SyntheticFlow {
    /// The per-period signed flow.
    quantities: Vec<f64>,
    /// The price path induced from the flow.
    prices: Vec<f64>,
    /// The target Hurst exponent of the flow.
    target_hurst: f64,
    /// The impact exponent applied to the flow.
    impact_exponent: f64,
    /// The seed used for the white-noise draws.
    seed: u64,
}

impl SyntheticFlow {
    /// Returns the per-period signed flow.
    #[must_use]
    pub fn quantities(&self) -> &[f64] {
        &self.quantities
    }

    /// Returns the induced price path, beginning at `0.0`.
    #[must_use]
    pub fn prices(&self) -> &[f64] {
        &self.prices
    }

    /// Returns the target Hurst exponent of the flow.
    #[must_use]
    pub const fn target_hurst(&self) -> f64 {
        self.target_hurst
    }

    /// Returns the impact exponent applied to the flow.
    #[must_use]
    pub const fn impact_exponent(&self) -> f64 {
        self.impact_exponent
    }

    /// Returns the seed used for the white-noise draws.
    #[must_use]
    pub const fn seed(&self) -> u64 {
        self.seed
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case(0.5)]
    #[case(0.4)]
    #[case(1.0)]
    #[case(f64::NAN)]
    fn test_the_config_refuses_a_hurst_at_or_below_the_memoryless_boundary(
        #[case] target_hurst: f64,
    ) {
        let error = SyntheticFlowConfig::new(target_hurst, 0.5, 16, 1)
            .expect_err("a hurst at or beyond the memoryless boundary must be refused");
        assert!(
            error.to_string().contains("boundary"),
            "error did not name the memoryless boundary: {error}"
        );
    }

    #[rstest]
    #[case(0.6, false)]
    #[case(1.0, false)]
    #[case(-0.1, false)]
    #[case(0.0, false)]
    #[case(0.5, true)]
    fn test_the_config_refuses_an_impact_exponent_above_one_half(
        #[case] impact_exponent: f64,
        #[case] accepted: bool,
    ) {
        let result = SyntheticFlowConfig::new(0.7, impact_exponent, 16, 1);
        if accepted {
            assert!(
                result.is_ok(),
                "an impact exponent of 0.5 is the boundary and must be accepted"
            );
        } else {
            let error = result.expect_err("an exponent above one half must be refused");
            assert!(
                error.to_string().contains("diffusive"),
                "error did not name the diffusiveness condition: {error}"
            );
        }
    }

    #[rstest]
    #[case(0)]
    #[case(1)]
    fn test_the_config_refuses_a_flow_shorter_than_two_periods(#[case] count: usize) {
        let error = SyntheticFlowConfig::new(0.7, 0.5, count, 1)
            .expect_err("a flow shorter than two periods must be refused");
        assert!(
            error.to_string().contains("at least 2"),
            "error did not name the two-period minimum: {error}"
        );
    }

    #[rstest]
    fn test_the_same_configuration_generates_the_same_flow() {
        let config = SyntheticFlowConfig::new(0.7, 0.5, 256, 42)
            .expect("the calibration parameters are valid");

        let first = config.generate().expect("generation succeeds");
        let second = config.generate().expect("generation succeeds");
        assert_eq!(first.quantities(), second.quantities());
        assert_eq!(first.prices(), second.prices());

        let other = SyntheticFlowConfig::new(0.7, 0.5, 256, 43)
            .expect("the calibration parameters are valid")
            .generate()
            .expect("generation succeeds");
        assert_ne!(
            first.quantities(),
            other.quantities(),
            "a different seed must give a different flow"
        );
    }

    #[rstest]
    #[expect(
        clippy::cast_precision_loss,
        reason = "the sample count is small enough to be represented exactly as f64"
    )]
    fn test_a_persistent_flow_has_a_variance_ratio_above_one() {
        let config = SyntheticFlowConfig::new(0.7, 0.5, 4096, 7)
            .expect("the calibration parameters are valid");
        let flow = config.generate().expect("generation succeeds");
        let quantities = flow.quantities();

        // Population variance throughout: divide by the number of observations, not n - 1.
        let mean = quantities.iter().sum::<f64>() / quantities.len() as f64;
        let single_variance = quantities
            .iter()
            .map(|quantity| (quantity - mean).powi(2))
            .sum::<f64>()
            / quantities.len() as f64;

        // Population variance of the overlapping q-period sums, divided by q.
        let q = 8;
        let sums = quantities
            .windows(q)
            .map(|window| window.iter().sum::<f64>())
            .collect::<Vec<f64>>();
        let sums_mean = sums.iter().sum::<f64>() / sums.len() as f64;
        let aggregated_variance = sums
            .iter()
            .map(|sum| (sum - sums_mean).powi(2))
            .sum::<f64>()
            / sums.len() as f64;

        let variance_ratio = (aggregated_variance / q as f64) / single_variance;
        assert!(
            variance_ratio > 1.0,
            "a persistent flow must have a variance ratio above one, was {variance_ratio}"
        );
    }

    #[rstest]
    fn test_the_induced_price_change_is_the_declared_power_of_the_flow() {
        let config = SyntheticFlowConfig::new(0.7, 0.5, 64, 11)
            .expect("the calibration parameters are valid");
        let flow = config.generate().expect("generation succeeds");
        let quantities = flow.quantities();
        let prices = flow.prices();

        for t in 1..8 {
            let flow_value = quantities[t];
            let expected = flow_value.signum() * flow_value.abs().powf(flow.impact_exponent());
            assert!(
                (prices[t] - prices[t - 1] - expected).abs() < 1e-12,
                "period {t}: price change {} did not equal the declared power {expected}",
                prices[t] - prices[t - 1]
            );
        }
    }
}
