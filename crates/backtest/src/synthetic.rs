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
//!
//! An estimator beside the generator recovers the target Hurst exponent and the impact exponent
//! back from a generated series ([`SyntheticFlow::estimate`]) and reports the remaining bias
//! against a declared band, carried with a [`SyntheticRecoveryCheck`]. A series shorter than the
//! check's declared readable length is reported as unreadable rather than estimated, because at
//! that length the estimator's finite-size bias is large enough to invent a result.
//!
//! The generator also serves the impact interval's coverage check: a
//! [`FlowImpactCoverage`] draws paths whose fill sizes are the flow's magnitudes and whose impacts
//! are the square-root law at a known prefactor, so the calibration's fitted interval can be
//! measured against a prefactor that is known by construction rather than asserted.

use std::fmt;

use anyhow::{Result, bail};
use nautilus_execution::models::market_impact::PrefactorInterval;
use nautilus_execution::models::market_impact_calibration::{
    ImpactObservation, PrefactorCoverageModel, fit_prefactor_from_fills,
};
use nautilus_model::types::Quantity;
use rand::{RngExt, SeedableRng, rngs::StdRng};

/// The maximum number of moving-average coefficients retained by the generator.
///
/// The fractional kernel is summable, so truncating the tail at this length leaves the normalized
/// flow effectively unchanged while bounding the per-period work.
const MAX_TRUNCATION: usize = 256;

/// The shortest series at which a synthetic-flow estimate is worth reading.
///
/// The aggregated-variance estimator's finite-size bias grows as the series shortens, and below
/// this length it is large enough to invent a persistence the series does not support. The
/// threshold is declared here rather than hidden inside the estimator so a reader can disagree
/// with the number, and a series shorter than it is reported as unreadable rather than estimated.
pub const SYNTHETIC_READABLE_LENGTH: usize = 8192;

/// The default band, in absolute units, within which a recovered estimate counts as recovered.
///
/// The band is a declared convention, not a property of the estimator: a caller supplies its own
/// with [`SyntheticRecoveryCheck`], and this constant is only the default. An estimate outside
/// the band is still reported with its number; it is simply not counted as recovered.
pub const SYNTHETIC_RECOVERY_BAND: f64 = 0.05;

/// The block sizes the aggregated-variance Hurst estimator regresses over.
const HURST_BLOCK_SIZES: [usize; 6] = [1, 2, 4, 8, 16, 32];

/// The fewest blocks a block size must yield to enter the aggregated-variance regression.
const MIN_HURST_BLOCKS: usize = 16;

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

    /// Generates the observed impacts this flow's fill sizes leave at `prefactor`.
    ///
    /// Each period's fill size is the magnitude of the generated flow, and the impact the
    /// square-root law predicts at `prefactor` is rounded to the nearest whole increment, which is
    /// what a tape records. A period whose flow is too small to move the price by an increment is
    /// dropped, because it says nothing about the prefactor. The result is the path a
    /// [`PrefactorCoverageModel`] fits an interval from, with the prefactor known by construction.
    ///
    /// # Errors
    ///
    /// Returns an error if the flow cannot be generated or an observation cannot be formed; the
    /// configuration was validated at construction, so this is otherwise infallible.
    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the reference quantity is a declared volume and the rounded increment count is a small whole number, so both are represented exactly as f64"
    )]
    pub fn generate_impact_observations(
        &self,
        prefactor: f64,
        reference_quantity: u64,
    ) -> Result<Vec<ImpactObservation>> {
        if reference_quantity == 0 {
            bail!("reference_quantity must be greater than zero");
        }

        let flow = self.generate()?;
        let reference = Quantity::from(reference_quantity.to_string());
        let mut observations = Vec::with_capacity(flow.quantities().len());
        for quantity in flow.quantities() {
            // A quantity carries at most nine decimal places, so the flow's magnitude is rounded
            // to six before it is handed over rather than formatted in full.
            let magnitude = (quantity.abs() * 1_000_000.0).round() / 1_000_000.0;
            let relative = (magnitude / reference_quantity as f64).sqrt();
            let increments = (prefactor * relative).round();
            if increments < 1.0 {
                continue;
            }
            observations.push(ImpactObservation::new(
                Quantity::from(format!("{magnitude:.6}")),
                reference,
                increments as u64,
            )?);
        }
        Ok(observations)
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

    /// Estimates the target Hurst exponent and impact exponent from this generated series.
    ///
    /// The Hurst exponent is recovered from the flow by the aggregated-variance method: for a
    /// long-memory flow the variance of a `q`-period block sum, divided by `q`, scales as
    /// `q^(2H-1)`, so the slope of its logarithm against `ln q` is `2H - 1`. The impact exponent
    /// is recovered from the induced price path by regressing `ln|price change|` on `ln|flow|`
    /// through the origin, which is exact because the generator sets the change to
    /// `sign(flow) * |flow|^impact`.
    ///
    /// A series shorter than the check's declared readable length is not estimated: the report
    /// carries no number and names the reason, because at that length the estimator's bias is
    /// large enough to invent a result. A series at or above the length that the estimator still
    /// cannot resolve is reported as unreadable for the same reason rather than given a number.
    ///
    /// The estimate is a pure function of the series and the check, so the same seed and length
    /// produce the same report.
    #[must_use]
    pub fn estimate(&self, check: &SyntheticRecoveryCheck) -> SyntheticFlowRecoveryReport {
        let length = self.quantities.len();
        let unreadable = |reason| SyntheticFlowRecoveryReport {
            length,
            readable_length: check.readable_length,
            band: check.band,
            hurst: None,
            impact_exponent: None,
            hurst_bias: None,
            impact_bias: None,
            unreadable: Some(reason),
        };

        if length < check.readable_length {
            return unreadable(UnreadableReason::BelowReadableLength);
        }

        match (
            estimate_hurst(&self.quantities),
            estimate_impact_exponent(&self.quantities, &self.prices),
        ) {
            (Some(hurst), Some(impact_exponent)) => SyntheticFlowRecoveryReport {
                length,
                readable_length: check.readable_length,
                band: check.band,
                hurst: Some(hurst),
                impact_exponent: Some(impact_exponent),
                hurst_bias: Some(hurst - self.target_hurst),
                impact_bias: Some(impact_exponent - self.impact_exponent),
                unreadable: None,
            },
            _ => unreadable(UnreadableReason::DegenerateSeries),
        }
    }
}

/// The declared check a [`SyntheticFlow::estimate`] is run against.
///
/// Both the readable length and the recovery band are declared here rather than assumed inside
/// the estimator: a caller can disagree with either number, and the report prints both with the
/// estimate so the disagreement is visible.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SyntheticRecoveryCheck {
    /// The shortest series at which an estimate is worth reading; below it no number is produced.
    pub readable_length: usize,
    /// The absolute band within which a recovered estimate counts as recovered.
    pub band: f64,
}

impl SyntheticRecoveryCheck {
    /// Creates a new check, validating the readable length and the band.
    ///
    /// # Errors
    ///
    /// Returns an error if `readable_length` is below two, because a shorter series carries no
    /// memory to estimate, or if `band` is not finite and positive.
    pub fn new(readable_length: usize, band: f64) -> Result<Self> {
        if readable_length < 2 {
            bail!(
                "readable_length must be at least 2, because a shorter series carries no memory to estimate, was {readable_length}"
            );
        }
        if !band.is_finite() || band <= 0.0 {
            bail!("band must be finite and positive, was {band}");
        }
        Ok(Self {
            readable_length,
            band,
        })
    }
}

impl Default for SyntheticRecoveryCheck {
    fn default() -> Self {
        Self {
            readable_length: SYNTHETIC_READABLE_LENGTH,
            band: SYNTHETIC_RECOVERY_BAND,
        }
    }
}

/// Why a [`SyntheticFlowRecoveryReport`] carries no number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnreadableReason {
    /// The generated series is shorter than the check's declared readable length.
    BelowReadableLength,
    /// The series is long enough but the estimator could not resolve a number from it.
    DegenerateSeries,
}

impl UnreadableReason {
    /// Returns the stable lowercase name of the reason.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::BelowReadableLength => "below_readable_length",
            Self::DegenerateSeries => "degenerate_series",
        }
    }
}

/// The estimate a generated [`SyntheticFlow`] makes of its own calibration.
///
/// The recovered Hurst exponent and impact exponent are `Option`s so that a series the estimator
/// cannot support reports an absence rather than a plausible number, and the reason for the
/// absence is a required field of that case. The band the estimate is judged against is carried
/// with it, so the number and the convention it was read against travel together.
#[derive(Debug, Clone, PartialEq)]
pub struct SyntheticFlowRecoveryReport {
    /// The length of the series the estimate was formed from.
    pub length: usize,
    /// The declared readable length the estimate was formed against.
    pub readable_length: usize,
    /// The declared band within which a recovered estimate counts as recovered.
    pub band: f64,
    /// The recovered Hurst exponent, absent when the series could not support one.
    pub hurst: Option<f64>,
    /// The recovered impact exponent, absent when the series could not support one.
    pub impact_exponent: Option<f64>,
    /// The remaining Hurst bias, `estimate - target`, absent when the estimate is.
    pub hurst_bias: Option<f64>,
    /// The remaining impact bias, `estimate - target`, absent when the estimate is.
    pub impact_bias: Option<f64>,
    /// Why the estimate carries no number, present exactly when it carries none.
    pub unreadable: Option<UnreadableReason>,
}

impl SyntheticFlowRecoveryReport {
    /// Returns whether the series was long enough for the estimate to carry numbers.
    #[must_use]
    pub const fn is_readable(&self) -> bool {
        self.unreadable.is_none()
    }

    /// Returns whether the recovered Hurst lies inside the declared band, absent when unreadable.
    #[must_use]
    pub fn hurst_recovered(&self) -> Option<bool> {
        self.hurst_bias.map(|bias| bias.abs() <= self.band)
    }

    /// Returns whether the recovered impact exponent lies inside the declared band, absent when
    /// unreadable.
    #[must_use]
    pub fn impact_recovered(&self) -> Option<bool> {
        self.impact_bias.map(|bias| bias.abs() <= self.band)
    }
}

impl fmt::Display for SyntheticFlowRecoveryReport {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(reason) = self.unreadable {
            return write!(
                formatter,
                "estimate not readable at length {} against the declared readable length {} ({})",
                self.length,
                self.readable_length,
                reason.name(),
            );
        }
        match (
            self.hurst,
            self.impact_exponent,
            self.hurst_bias,
            self.impact_bias,
        ) {
            (Some(hurst), Some(impact), Some(hurst_bias), Some(impact_bias)) => write!(
                formatter,
                "hurst {hurst:.4} (band +/- {:.4}, bias {hurst_bias:+.4}), impact {impact:.4} (band +/- {:.4}, bias {impact_bias:+.4}), length {}",
                self.band, self.band, self.length,
            ),
            _ => write!(
                formatter,
                "estimate malformed: readable but without numbers"
            ),
        }
    }
}

/// Estimates the Hurst exponent of the flow by the aggregated-variance method.
///
/// For each declared block size the estimator sums non-overlapping blocks of the flow, takes the
/// population variance of those sums divided by the block size, and regresses its logarithm on the
/// logarithm of the block size. The slope is `2H - 1`, so the estimate is `(slope + 1) / 2`. A
/// block size that would leave fewer than [`MIN_HURST_BLOCKS`] blocks is skipped, and a degenerate
/// variance or fewer than two usable block sizes is an explicit absence.
#[expect(
    clippy::cast_precision_loss,
    reason = "block sizes and counts are small enough to be represented exactly as f64"
)]
fn estimate_hurst(quantities: &[f64]) -> Option<f64> {
    let mut log_sizes = Vec::with_capacity(HURST_BLOCK_SIZES.len());
    let mut log_variances = Vec::with_capacity(HURST_BLOCK_SIZES.len());

    for size in HURST_BLOCK_SIZES {
        let blocks = quantities.len() / size;
        if blocks < MIN_HURST_BLOCKS {
            continue;
        }

        let sums: Vec<f64> = quantities
            .chunks_exact(size)
            .map(|block| block.iter().sum::<f64>())
            .collect();
        let mean = sums.iter().sum::<f64>() / blocks as f64;
        let variance = sums.iter().map(|sum| (sum - mean).powi(2)).sum::<f64>() / blocks as f64;
        let aggregated = variance / size as f64;
        if !aggregated.is_finite() || aggregated <= 0.0 {
            return None;
        }

        log_sizes.push((size as f64).ln());
        log_variances.push(aggregated.ln());
    }

    let slope = regression_slope(&log_sizes, &log_variances)?;
    Some(f64::midpoint(slope, 1.0))
}

/// Estimates the impact exponent by regressing `ln|price change|` on `ln|flow|` through the
/// origin.
///
/// The generator sets the price change over a period to `sign(flow) * |flow|^impact`, so the
/// log-log slope of the magnitudes is the exponent exactly. A period whose flow or change is zero
/// is skipped, because its logarithm is undefined, and a regression with no usable period is an
/// explicit absence.
fn estimate_impact_exponent(quantities: &[f64], prices: &[f64]) -> Option<f64> {
    let mut numerator = 0.0;
    let mut denominator = 0.0;
    for period in 1..quantities.len().min(prices.len()) {
        let flow = quantities[period].abs();
        let change = (prices[period] - prices[period - 1]).abs();
        if flow <= 0.0 || change <= 0.0 {
            continue;
        }
        let log_flow = flow.ln();
        numerator += log_flow * change.ln();
        denominator += log_flow * log_flow;
    }

    if !denominator.is_finite() || denominator <= 0.0 {
        return None;
    }
    let impact = numerator / denominator;
    if impact.is_finite() {
        Some(impact)
    } else {
        None
    }
}

/// Returns the ordinary least-squares slope of `y` on `x`, or an explicit absence when the
/// regression is degenerate.
#[expect(
    clippy::cast_precision_loss,
    reason = "the point count is small enough to be represented exactly as f64"
)]
fn regression_slope(x: &[f64], y: &[f64]) -> Option<f64> {
    if x.len() < 2 || x.len() != y.len() {
        return None;
    }

    let count = x.len() as f64;
    let mean_x = x.iter().sum::<f64>() / count;
    let mean_y = y.iter().sum::<f64>() / count;
    let mut covariance = 0.0;
    let mut variance = 0.0;
    for (x, y) in x.iter().zip(y) {
        let dx = x - mean_x;
        covariance += dx * (y - mean_y);
        variance += dx * dx;
    }

    if variance <= 0.0 {
        return None;
    }
    Some(covariance / variance)
}

/// A prefactor-coverage model whose paths are drawn from the synthetic flow.
///
/// Each period's fill size is the magnitude of the flow, and the impact the square-root law
/// predicts at the known prefactor is rounded to the nearest whole increment, which is what a tape
/// records ([`SyntheticFlowConfig::generate_impact_observations`]). The interval it fits is the
/// span of the per-observation estimates, which is the calibration's own fit, so the coverage this
/// model reports is the coverage of the calibration when the true prefactor is known by
/// construction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlowImpactCoverage {
    /// The target Hurst exponent of the flow the paths are drawn from.
    pub target_hurst: f64,
    /// The impact exponent of the flow the paths are drawn from.
    pub impact_exponent: f64,
    /// The number of periods in each generated path.
    pub count: usize,
    /// The reference quantity the impacts are expressed against.
    pub reference_quantity: u64,
}

impl FlowImpactCoverage {
    /// Creates a new [`FlowImpactCoverage`] instance, validating its declarations.
    ///
    /// # Errors
    ///
    /// Returns an error if the flow configuration they describe is invalid or if
    /// `reference_quantity` is zero.
    pub fn new(
        target_hurst: f64,
        impact_exponent: f64,
        count: usize,
        reference_quantity: u64,
    ) -> Result<Self> {
        // Validate the flow declaration here rather than at the first draw, so an invalid model is
        // refused when it is built.
        SyntheticFlowConfig::new(target_hurst, impact_exponent, count, 0)?;
        if reference_quantity == 0 {
            bail!("reference_quantity must be greater than zero");
        }
        Ok(Self {
            target_hurst,
            impact_exponent,
            count,
            reference_quantity,
        })
    }
}

impl PrefactorCoverageModel for FlowImpactCoverage {
    fn draw(&self, truth: f64, seed: u64) -> Result<Vec<ImpactObservation>> {
        SyntheticFlowConfig::new(self.target_hurst, self.impact_exponent, self.count, seed)?
            .generate_impact_observations(truth, self.reference_quantity)
    }

    fn fit(&self, observations: &[ImpactObservation]) -> Result<PrefactorInterval> {
        fit_prefactor_from_fills(observations)
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

    #[rstest]
    fn test_the_check_refuses_a_readable_length_below_two() {
        let error = SyntheticRecoveryCheck::new(1, 0.05)
            .expect_err("a readable length below two must be refused");
        assert!(
            error.to_string().contains("at least 2"),
            "error did not name the two-period minimum: {error}"
        );
    }

    #[rstest]
    #[case(0.0)]
    #[case(-0.05)]
    #[case(f64::NAN)]
    #[case(f64::INFINITY)]
    fn test_the_check_refuses_a_band_that_is_not_finite_and_positive(#[case] band: f64) {
        let error = SyntheticRecoveryCheck::new(64, band)
            .expect_err("a band that is not finite and positive must be refused");
        assert!(
            error
                .to_string()
                .contains("band must be finite and positive"),
            "error did not name the band condition: {error}"
        );
    }

    #[rstest]
    fn test_the_default_check_declares_the_readable_length_and_band() {
        let check = SyntheticRecoveryCheck::default();
        assert_eq!(check.readable_length, SYNTHETIC_READABLE_LENGTH);
        assert!((check.band - SYNTHETIC_RECOVERY_BAND).abs() < f64::EPSILON);
    }

    #[rstest]
    fn test_a_long_series_recovers_the_hurst_and_impact_within_the_band() {
        let flow = SyntheticFlowConfig::new(0.6, 0.5, 16_384, 7)
            .expect("the calibration parameters are valid")
            .generate()
            .expect("generation succeeds");
        let estimate = flow.estimate(&SyntheticRecoveryCheck::default());
        assert!(
            estimate.is_readable(),
            "a 16_384-period series is above the readable length"
        );
        assert_eq!(
            estimate.hurst_recovered(),
            Some(true),
            "the recovered Hurst must lie inside the declared band: {estimate}"
        );
        assert_eq!(
            estimate.impact_recovered(),
            Some(true),
            "the recovered impact exponent must lie inside the declared band: {estimate}"
        );
        assert!(
            estimate.hurst_bias.expect("readable").abs() <= estimate.band,
            "the remaining Hurst bias must be inside the band: {estimate}"
        );
        let printed = estimate.to_string();
        assert!(
            printed.contains("bias"),
            "the report must print the remaining bias: {printed}"
        );
    }

    #[rstest]
    fn test_a_series_below_the_readable_length_reports_no_number() {
        let flow = SyntheticFlowConfig::new(0.6, 0.5, SYNTHETIC_READABLE_LENGTH - 1, 7)
            .expect("the calibration parameters are valid")
            .generate()
            .expect("generation succeeds");
        let estimate = flow.estimate(&SyntheticRecoveryCheck::default());

        assert!(
            !estimate.is_readable(),
            "a series below the readable length must not be estimated"
        );
        assert_eq!(estimate.hurst, None);
        assert_eq!(estimate.impact_exponent, None);
        assert_eq!(estimate.hurst_bias, None);
        assert_eq!(estimate.impact_bias, None);
        assert_eq!(
            estimate.unreadable,
            Some(UnreadableReason::BelowReadableLength)
        );

        let printed = estimate.to_string();
        assert!(
            printed.contains("not readable"),
            "the report must state that the estimate is not readable: {printed}"
        );
        assert!(
            !printed.contains("hurst") && !printed.contains("impact"),
            "the report must print no estimate number: {printed}"
        );
    }

    #[rstest]
    fn test_a_series_at_the_readable_length_is_estimated() {
        let flow = SyntheticFlowConfig::new(0.6, 0.5, SYNTHETIC_READABLE_LENGTH, 7)
            .expect("the calibration parameters are valid")
            .generate()
            .expect("generation succeeds");
        let estimate = flow.estimate(&SyntheticRecoveryCheck::default());

        assert!(
            estimate.is_readable(),
            "a series at the readable length must be estimated: {estimate}"
        );
        assert!(estimate.hurst.is_some());
        assert!(estimate.impact_exponent.is_some());
    }

    #[rstest]
    fn test_the_readable_length_is_the_declared_check_not_a_constant() {
        let flow = SyntheticFlowConfig::new(0.6, 0.5, 1_024, 7)
            .expect("the calibration parameters are valid")
            .generate()
            .expect("generation succeeds");

        assert!(
            !flow
                .estimate(&SyntheticRecoveryCheck::default())
                .is_readable(),
            "the default check must refuse a 1_024-period series"
        );

        let declared =
            SyntheticRecoveryCheck::new(1_024, 0.2).expect("the check parameters are valid");
        let estimate = flow.estimate(&declared);
        assert!(
            estimate.is_readable(),
            "a check declaring a 1_024-period readable length must accept the series"
        );
        assert!((estimate.band - 0.2).abs() < f64::EPSILON);
        assert_eq!(estimate.readable_length, 1_024);
        assert_eq!(estimate.length, 1_024);
    }

    #[rstest]
    fn test_the_same_seed_and_length_produce_the_same_estimate() {
        let config = SyntheticFlowConfig::new(0.6, 0.5, 16_384, 42)
            .expect("the calibration parameters are valid");
        let check = SyntheticRecoveryCheck::default();

        let first = config
            .generate()
            .expect("generation succeeds")
            .estimate(&check);
        let second = config
            .generate()
            .expect("generation succeeds")
            .estimate(&check);
        assert_eq!(first, second);
        assert_eq!(first.to_string(), second.to_string());
    }

    #[rstest]
    fn test_the_impact_path_recovers_the_known_prefactor() {
        let observations = SyntheticFlowConfig::new(0.7, 0.5, 1_024, 7)
            .expect("the calibration parameters are valid")
            .generate_impact_observations(2.0, 1)
            .expect("generation succeeds");

        let fitted = fit_prefactor_from_fills(&observations).expect("the path fits");
        assert!(
            fitted.lower() <= 2.0 && 2.0 <= fitted.upper(),
            "the fitted span must contain the known prefactor: {fitted}"
        );
    }

    #[rstest]
    fn test_the_impact_path_drops_periods_that_did_not_move_the_price() {
        let observations = SyntheticFlowConfig::new(0.7, 0.5, 512, 11)
            .expect("the calibration parameters are valid")
            .generate_impact_observations(2.0, 1)
            .expect("generation succeeds");

        assert!(!observations.is_empty());
        for observation in &observations {
            assert!(
                observation.increments() >= 1,
                "a dropped period must carry no observation"
            );
            assert!(observation.prefactor().is_some());
        }
    }

    #[rstest]
    fn test_the_coverage_model_is_deterministic_at_a_seed() {
        let model = FlowImpactCoverage::new(0.7, 0.5, 512, 1).expect("valid model");
        let first = model.draw(2.0, 0x5EED).expect("draw succeeds");
        let second = model.draw(2.0, 0x5EED).expect("draw succeeds");

        assert_eq!(first, second);
        assert_eq!(
            model.fit(&first).expect("fit succeeds"),
            model.fit(&second).expect("fit succeeds")
        );
    }
}
