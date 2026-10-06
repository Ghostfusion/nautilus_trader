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

//! Calibration of the square-root impact prefactor from observed price impacts.
//!
//! The prefactor is the one input of the impact model that has to come from data, and the data
//! decides how much it can be trusted. An observation is one aggressive metaorder's consumption
//! together with the price movement it left behind, in whole price increments, and the prefactor
//! it implies is `increments / sqrt(quantity / reference_quantity)`. Fitting reduces a series of
//! those per-observation estimates to a bounded range: the span of the estimates, which is the
//! same cross-sectional reading the corpus reports when it places the prefactor between 0.34 and
//! 1.50 across markets.
//!
//! # Fills and tapes
//!
//! A prefactor fitted from the venue's own fills is measured, because the aggressor and the
//! metaorder are observable. A prefactor fitted from an anonymous tape is inferred: the metaorders
//! had to be reconstructed from clips, and that reconstruction inflates the prefactor about
//! twofold, so [`debiasing`](PrefactorInterval::debiased) is an explicit step rather than a
//! default. A calibration that has not been de-biased says so in its source.
//!
//! The reconstruction itself is the caller's step: this module fits the prefactor from the
//! observations it is handed, and records which of the two sources produced them.
//!
//! # Coverage
//!
//! A fitted interval that never contains the value it was fitted for is a decoration, so the
//! module also measures whether an interval is calibrated. [`prefactor_coverage`] draws seeded
//! paths at a known prefactor from a [`PrefactorCoverageModel`], fits the interval on each, and
//! reports the fraction of repetitions whose fitted interval contained the known prefactor,
//! beside the interval, its source, the nominal level it is supposed to hold at and the repetition
//! count. The measurement mirrors the parameter-recovery shape in `nautilus-research` locally
//! rather than depending on it, because `nautilus-execution` sits below that crate and a
//! dependency to reuse two dozen lines would point the graph the wrong way.

use std::fmt;

use nautilus_model::types::Quantity;
use rust_decimal::{Decimal, prelude::ToPrimitive};

use super::market_impact::{ImpactCalibrationSource, PrefactorInterval};

/// One observed price impact, with the size that caused it and the volume it is measured against.
///
/// The quantity is the aggressive metaorder's own consumption, not the clip that happened to
/// print: an anonymous tape's clips are not metaorders, and the caller reconstructs them before
/// fitting. The reference quantity is the volume the impact is expressed relative to, which is
/// what makes the estimates comparable across observations.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.execution", from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.execution")
)]
pub struct ImpactObservation {
    quantity: Quantity,
    reference_quantity: Quantity,
    increments: u64,
}

impl ImpactObservation {
    /// Creates a new [`ImpactObservation`] instance.
    ///
    /// # Errors
    ///
    /// Returns an error if `quantity` or `reference_quantity` is zero, because the prefactor is
    /// expressed relative to the reference volume and an observation of nothing implies none.
    pub fn new(
        quantity: Quantity,
        reference_quantity: Quantity,
        increments: u64,
    ) -> anyhow::Result<Self> {
        if quantity.is_zero() {
            anyhow::bail!("quantity must be greater than zero");
        }
        if reference_quantity.is_zero() {
            anyhow::bail!("reference_quantity must be greater than zero");
        }
        Ok(Self {
            quantity,
            reference_quantity,
            increments,
        })
    }

    /// Returns the aggressive quantity whose consumption caused the impact.
    #[must_use]
    pub const fn quantity(&self) -> Quantity {
        self.quantity
    }

    /// Returns the volume the quantity is measured against.
    #[must_use]
    pub const fn reference_quantity(&self) -> Quantity {
        self.reference_quantity
    }

    /// Returns the observed price movement in whole price increments.
    #[must_use]
    pub const fn increments(&self) -> u64 {
        self.increments
    }

    /// Returns the prefactor this observation implies, or `None` when the observation is not
    /// usable.
    ///
    /// The estimate is `increments / sqrt(quantity / reference_quantity)`: the square-root law
    /// inverted, with the quantity and reference quantity carried as exact decimals so the ratio
    /// is not rounded before the square root is taken.
    #[must_use]
    pub fn prefactor(&self) -> Option<f64> {
        let quantity = self.quantity.as_decimal().to_f64()?;
        let reference = self.reference_quantity.as_decimal().to_f64()?;
        if quantity <= 0.0 || reference <= 0.0 {
            return None;
        }

        let relative = (quantity / reference).sqrt();
        if relative <= 0.0 {
            return None;
        }

        let increments = Decimal::from(self.increments).to_f64()?;
        let estimate = increments / relative;
        estimate.is_finite().then_some(estimate)
    }
}

/// Fits a prefactor from observations taken where the aggressor and the metaorder are observable.
///
/// The fitted interval spans the per-observation estimates, and its source records that the
/// prefactor was measured rather than inferred, so the result needs no de-bias.
///
/// # Errors
///
/// Returns an error if `observations` is empty, if any observation has no usable estimate, or if
/// the span is not a valid interval, which is what a series in which no observation moved the
/// price by an increment produces.
pub fn fit_prefactor_from_fills(
    observations: &[ImpactObservation],
) -> anyhow::Result<PrefactorInterval> {
    fit_prefactor(observations, ImpactCalibrationSource::Fills)
}

/// Fits a prefactor from observations reconstructed from an anonymous tape.
///
/// The fitted interval spans the per-observation estimates and its source records that the
/// prefactor was inferred from reconstructed metaorders, so the result carries the reconstruction's
/// inflation until [`debiased`](PrefactorInterval::debiased) is applied to it.
///
/// # Errors
///
/// Returns an error if `observations` is empty, if any observation has no usable estimate, or if
/// the span is not a valid interval, which is what a series in which no observation moved the
/// price by an increment produces.
pub fn fit_prefactor_from_tape(
    observations: &[ImpactObservation],
) -> anyhow::Result<PrefactorInterval> {
    fit_prefactor(observations, ImpactCalibrationSource::AnonymousTape)
}

/// Fits the prefactor span from observations and attributes it to `source`.
fn fit_prefactor(
    observations: &[ImpactObservation],
    source: ImpactCalibrationSource,
) -> anyhow::Result<PrefactorInterval> {
    if observations.is_empty() {
        anyhow::bail!("at least one observation is required to fit a prefactor");
    }

    let mut lower = f64::INFINITY;
    let mut upper = f64::NEG_INFINITY;

    for observation in observations {
        let estimate = observation.prefactor().ok_or_else(|| {
            anyhow::anyhow!(
                "observation of {} against {} has no usable prefactor estimate",
                observation.quantity(),
                observation.reference_quantity(),
            )
        })?;

        lower = lower.min(estimate);
        upper = upper.max(estimate);
    }

    PrefactorInterval::new(lower, upper, source)
        .map_err(|error| anyhow::anyhow!("fitted prefactor span is not a usable interval: {error}"))
}

/// The default nominal coverage level a prefactor interval is checked against.
///
/// The level is a declared convention rather than a property of the interval, so a caller supplies
/// its own with [`PrefactorCoverageCheck`] and this constant is only the default.
pub const PREFACTOR_COVERAGE_NOMINAL: f64 = 0.9;

/// The seam a prefactor-coverage check draws its paths from.
///
/// A coverage check needs two things from a model: a path of observed impacts generated at a known
/// prefactor under a seed, and the interval the model fits from that path. Both are here, and
/// nothing else is required, so the synthetic-flow generator and a caller's own tape fixture can
/// implement it without knowing about the report.
///
/// A model MUST return the same path for the same `(truth, seed)`, so a coverage report over a seed
/// is reproducible.
pub trait PrefactorCoverageModel {
    /// Draws one path of observed impacts at the known prefactor `truth` under `seed`.
    ///
    /// # Errors
    ///
    /// Returns an error if the path cannot be generated, rather than a plausible substitute.
    fn draw(&self, truth: f64, seed: u64) -> anyhow::Result<Vec<ImpactObservation>>;

    /// Fits the prefactor interval the path declares.
    ///
    /// The interval may be well-calibrated or over-tight; the check measures which. A model that
    /// cannot fit an interval from the path returns an error rather than a fabricated one.
    ///
    /// # Errors
    ///
    /// Returns an error if no interval can be fitted from the observations.
    fn fit(&self, observations: &[ImpactObservation]) -> anyhow::Result<PrefactorInterval>;
}

/// The declared check a coverage measurement is run against.
///
/// The known prefactor, the nominal level and the repetition count are declared here rather than
/// assumed inside the measurement: a caller can disagree with any of them, and the report prints
/// all three with the figure, because a coverage fraction without its nominal level and count is
/// not interpretable.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrefactorCoverageCheck {
    /// The known prefactor the paths are generated at.
    pub truth: f64,
    /// The nominal coverage level the interval is supposed to hold at.
    pub nominal: f64,
    /// The number of seeded repetitions drawn and fitted.
    pub repetitions: usize,
    /// The base seed; repetition `r`'s seed is derived from it deterministically.
    pub seed: u64,
}

impl PrefactorCoverageCheck {
    /// Creates a new [`PrefactorCoverageCheck`], validating its declarations.
    ///
    /// # Errors
    ///
    /// Returns an error if `truth` is not finite and greater than zero, if `nominal` is not finite
    /// and within `(0.0, 1.0]`, or if `repetitions` is zero.
    pub fn new(truth: f64, nominal: f64, repetitions: usize, seed: u64) -> anyhow::Result<Self> {
        if !truth.is_finite() || truth <= 0.0 {
            anyhow::bail!("truth must be finite and greater than zero, was {truth}");
        }
        if !nominal.is_finite() || nominal <= 0.0 || nominal > 1.0 {
            anyhow::bail!("nominal must be finite and within (0.0, 1.0], was {nominal}");
        }
        if repetitions == 0 {
            anyhow::bail!("at least one repetition is required to measure coverage");
        }
        Ok(Self {
            truth,
            nominal,
            repetitions,
            seed,
        })
    }
}

/// The coverage of a prefactor interval over seeded repetitions.
///
/// The report carries the interval it measured, its calibration source, the nominal level it was
/// supposed to hold at, and the repetition count, so the fraction is interpretable on its own. The
/// interval is the one fitted on the reference repetition, and the coverage is the fraction of
/// repetitions whose fitted interval contained the known prefactor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrefactorCoverageReport {
    /// The interval fitted on the reference repetition, with its source.
    pub interval: PrefactorInterval,
    /// The known prefactor the paths were generated at.
    pub truth: f64,
    /// The nominal coverage level the interval is supposed to hold at.
    pub nominal: f64,
    /// The number of seeded repetitions drawn and fitted.
    pub repetitions: usize,
    /// The number of repetitions whose fitted interval contained the known prefactor.
    pub covered: usize,
    /// The observed coverage fraction, `covered / repetitions`.
    pub coverage: f64,
}

impl PrefactorCoverageReport {
    /// Returns whether the observed coverage meets the nominal level.
    #[must_use]
    pub fn holds(&self) -> bool {
        self.coverage >= self.nominal
    }
}

impl fmt::Display for PrefactorCoverageReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "prefactor coverage {:.3} of nominal {:.3} over {} repetitions ({} covered {}), interval {}",
            self.coverage, self.nominal, self.repetitions, self.covered, self.truth, self.interval,
        )
    }
}

/// Measures the coverage of a prefactor interval.
///
/// Draws `check.repetitions` paths from `model` at `check.truth`, each under a seed derived from
/// `check.seed`, fits the prefactor interval on each, and reports the fraction of repetitions whose
/// fitted interval contained the known prefactor, together with the interval fitted on the
/// reference repetition, its source, the nominal level and the repetition count. A deliberately
/// over-tight interval reads below its nominal level rather than being excused: the fit is not
/// widened to meet the level.
///
/// The measurement is a pure function of the model, the check and the seed: it reads no clock and
/// holds no state, so the same seed produces the same report.
///
/// # Errors
///
/// Returns an error if the model cannot draw a path or fit an interval from one.
pub fn prefactor_coverage<M: PrefactorCoverageModel>(
    model: &M,
    check: &PrefactorCoverageCheck,
) -> anyhow::Result<PrefactorCoverageReport> {
    let mut reference: Option<PrefactorInterval> = None;
    let mut covered = 0usize;

    for repetition in 0..check.repetitions {
        let seed = repetition_seed(check.seed, repetition);
        let observations = model.draw(check.truth, seed)?;
        let interval = model.fit(&observations)?;
        if interval.lower() <= check.truth && check.truth <= interval.upper() {
            covered += 1;
        }
        if reference.is_none() {
            reference = Some(interval);
        }
    }

    let interval = reference.ok_or_else(|| {
        anyhow::anyhow!("at least one repetition is required to measure coverage")
    })?;

    Ok(PrefactorCoverageReport {
        interval,
        truth: check.truth,
        nominal: check.nominal,
        repetitions: check.repetitions,
        covered,
        coverage: covered as f64 / check.repetitions as f64,
    })
}

/// Derives the seed of one repetition from the check's base seed.
///
/// The derivation is a `SplitMix64` mix, so consecutive repetitions receive well-separated seeds even
/// when a caller's own generator consumes them linearly. It is arithmetic on the seed alone: no
/// dependency is added and no clock is read.
fn repetition_seed(base: u64, repetition: usize) -> u64 {
    splitmix64(base ^ (repetition as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15))
}

/// Returns the `SplitMix64` mix of a 64-bit value.
fn splitmix64(value: u64) -> u64 {
    let mut mixed = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    mixed ^ (mixed >> 31)
}

#[cfg(test)]
mod tests {
    use rand::{RngExt, SeedableRng, rngs::StdRng};
    use rstest::rstest;

    use super::*;

    /// Returns an observation of `increments` at the given size over a reference of 100 units.
    fn observation(quantity: u64, increments: u64) -> ImpactObservation {
        ImpactObservation::new(
            Quantity::from(quantity.to_string()),
            Quantity::from("100"),
            increments,
        )
        .expect("valid observation")
    }

    #[rstest]
    fn test_an_observation_implies_the_prefactor_that_produced_it() {
        // At the reference quantity the square root is one, so the estimate is the increment count.
        assert!((observation(100, 25).prefactor().unwrap() - 25.0).abs() < 1e-12);

        // Four times the reference quantity doubles the square root, so the estimate halves.
        assert!((observation(400, 25).prefactor().unwrap() - 12.5).abs() < 1e-12);
    }

    #[rstest]
    fn test_an_observation_rejects_a_zero_size_or_reference() {
        assert!(
            ImpactObservation::new(Quantity::from("0"), Quantity::from("100"), 1).is_err(),
            "a zero quantity must be rejected",
        );
        assert!(
            ImpactObservation::new(Quantity::from("10"), Quantity::from("0"), 1).is_err(),
            "a zero reference quantity must be rejected",
        );
    }

    #[rstest]
    fn test_a_fills_fit_spans_the_estimates_and_needs_no_debias() {
        let observations = [
            observation(100, 40),
            observation(100, 50),
            observation(100, 60),
        ];
        let fitted = fit_prefactor_from_fills(&observations).expect("fitted");

        assert_eq!(fitted.lower(), 40.0);
        assert_eq!(fitted.upper(), 60.0);
        assert_eq!(fitted.source(), ImpactCalibrationSource::Fills);

        // A measured prefactor is already the value to use, so the de-bias refuses it.
        assert!(fitted.debiased().is_err());
    }

    #[rstest]
    fn test_a_tape_fit_reports_two_bounds_and_debiases_explicitly() {
        let observations = [observation(100, 40), observation(100, 80)];
        let fitted = fit_prefactor_from_tape(&observations).expect("fitted");

        // The reconstruction inflates the estimate, so the span is what an anonymous tape reports.
        assert_eq!(fitted.lower(), 40.0);
        assert_eq!(fitted.upper(), 80.0);
        assert_eq!(fitted.source(), ImpactCalibrationSource::AnonymousTape);

        let debiased = fitted.debiased().expect("de-biased");
        assert_eq!(debiased.lower(), 20.0);
        assert_eq!(debiased.upper(), 40.0);
        assert_eq!(
            debiased.source(),
            ImpactCalibrationSource::AnonymousTapeDebiased
        );

        // The de-bias is explicit and idempotent in the sense that it refuses a second pass.
        assert!(debiased.debiased().is_err());
    }

    #[rstest]
    fn test_a_single_observation_fits_a_point() {
        let fitted = fit_prefactor_from_fills(&[observation(100, 7)]).expect("fitted");

        assert!(fitted.is_point());
        assert_eq!(fitted.upper(), 7.0);
    }

    #[rstest]
    fn test_an_empty_series_fits_nothing() {
        assert!(fit_prefactor_from_fills(&[]).is_err());
        assert!(fit_prefactor_from_tape(&[]).is_err());
    }

    #[rstest]
    fn test_a_series_that_never_moved_the_price_fits_nothing() {
        // Every estimate is zero, so the span is not a usable interval: the fill sizes were below
        // one price increment and the series says nothing about the prefactor.
        let observations = [observation(1, 0), observation(1, 0)];

        assert!(fit_prefactor_from_fills(&observations).is_err());
    }

    #[rstest]
    fn test_the_fit_carries_decimal_sizes_exactly() {
        // 2.5 at the reference of 100 is a relative size of 0.025, whose square root is
        // 0.15811388, so 3 increments imply a prefactor of 18.97.
        let observation = ImpactObservation::new(Quantity::from("2.5"), Quantity::from("100"), 3)
            .expect("valid observation");

        let estimate = observation.prefactor().unwrap();
        assert!((estimate - 3.0 / 0.025_f64.sqrt()).abs() < 1e-9);

        let fitted = fit_prefactor_from_fills(&[observation]).expect("fitted");
        assert!((fitted.upper() - estimate).abs() < 1e-9);
    }

    #[rstest]
    fn test_a_fitted_interval_configures_a_model() {
        use crate::models::market_impact::{MarketImpactModel, SquareRootMarketImpactModel};

        let observations = [observation(100, 40), observation(100, 80)];
        let fitted = fit_prefactor_from_tape(&observations)
            .expect("fitted")
            .debiased()
            .expect("de-biased");

        let mut model =
            SquareRootMarketImpactModel::new(fitted, Quantity::from("100"), 10).expect("valid");

        // The model applies the de-biased upper bound: floor(40 * sqrt(1)) = 40, capped at 10.
        assert_eq!(model.impact_increments(Quantity::from("100")).unwrap(), 10);
        assert_eq!(model.prefactor(), fitted);
    }

    /// A deterministic coverage model for the tests.
    ///
    /// It draws fill sizes from a seeded uniform range, assigns each the square-root impact at the
    /// known prefactor rounded to the nearest whole increment, which is what a tape records, and
    /// fits the span of the per-observation estimates narrowed by `shrink` about its midpoint. A
    /// `shrink` of one is the calibration's own span; a `shrink` of zero is a point, a deliberately
    /// over-tight interval that the data does not support.
    struct RoundedImpactModel {
        reference: u64,
        min_quantity: u64,
        max_quantity: u64,
        observations: usize,
        shrink: f64,
    }

    impl RoundedImpactModel {
        fn new(observations: usize, shrink: f64) -> Self {
            Self {
                reference: 100,
                min_quantity: 1,
                max_quantity: 10_000,
                observations,
                shrink,
            }
        }
    }

    impl PrefactorCoverageModel for RoundedImpactModel {
        fn draw(&self, truth: f64, seed: u64) -> anyhow::Result<Vec<ImpactObservation>> {
            let mut rng = StdRng::seed_from_u64(seed);
            let mut observations = Vec::with_capacity(self.observations);
            for _ in 0..self.observations {
                let quantity = rng.random_range(self.min_quantity..=self.max_quantity);
                let relative = (quantity as f64 / self.reference as f64).sqrt();
                let increments = (truth * relative).round();
                if increments < 1.0 {
                    // A period that did not move the price by an increment says nothing about the
                    // prefactor, so it is dropped rather than fitted as a zero estimate.
                    continue;
                }
                observations.push(ImpactObservation::new(
                    Quantity::from(quantity.to_string()),
                    Quantity::from(self.reference.to_string()),
                    increments as u64,
                )?);
            }
            Ok(observations)
        }

        fn fit(&self, observations: &[ImpactObservation]) -> anyhow::Result<PrefactorInterval> {
            let fitted = fit_prefactor_from_fills(observations)?;
            let midpoint = f64::midpoint(fitted.lower(), fitted.upper());
            let half = (fitted.upper() - fitted.lower()) * 0.5 * self.shrink;
            PrefactorInterval::new(midpoint - half, midpoint + half, fitted.source())
        }
    }

    #[rstest]
    fn test_a_well_calibrated_interval_reads_at_or_above_its_nominal_level() {
        let model = RoundedImpactModel::new(256, 1.0);
        let check = PrefactorCoverageCheck::new(2.0, PREFACTOR_COVERAGE_NOMINAL, 128, 0x5EED)
            .expect("valid check");
        let report = prefactor_coverage(&model, &check).expect("coverage measured");

        assert!(
            report.holds(),
            "the span fit must hold at its nominal level: {report}"
        );
        assert!(report.coverage >= report.nominal, "{report}");
        assert_eq!(report.repetitions, 128);
        assert_eq!(report.covered as f64 / 128.0, report.coverage);
    }

    #[rstest]
    fn test_a_deliberately_over_tight_interval_reads_below_its_nominal_level() {
        let model = RoundedImpactModel::new(256, 0.0);
        let check = PrefactorCoverageCheck::new(2.0, PREFACTOR_COVERAGE_NOMINAL, 128, 0x5EED)
            .expect("valid check");
        let report = prefactor_coverage(&model, &check).expect("coverage measured");

        assert!(
            !report.holds(),
            "an over-tight interval must read below its nominal level rather than being excused: {report}"
        );
        assert!(report.coverage < report.nominal, "{report}");
    }

    #[rstest]
    fn test_the_same_seed_produces_the_same_coverage() {
        let model = RoundedImpactModel::new(256, 1.0);
        let check = PrefactorCoverageCheck::new(2.0, 0.9, 64, 0x5EED).expect("valid check");

        let first = prefactor_coverage(&model, &check).expect("coverage measured");
        let second = prefactor_coverage(&model, &check).expect("coverage measured");

        assert_eq!(first, second);
        assert_eq!(first.to_string(), second.to_string());
    }

    #[rstest]
    fn test_the_report_carries_the_interval_its_source_the_level_and_the_count() {
        let model = RoundedImpactModel::new(256, 1.0);
        let check = PrefactorCoverageCheck::new(2.0, 0.95, 32, 7).expect("valid check");
        let report = prefactor_coverage(&model, &check).expect("coverage measured");

        assert_eq!(report.interval.source(), ImpactCalibrationSource::Fills);
        assert_eq!(report.truth, 2.0);
        assert_eq!(report.nominal, 0.95);
        assert_eq!(report.repetitions, 32);
        assert!(
            report.interval.lower() > 0.0 && report.interval.upper() >= report.interval.lower()
        );

        let printed = report.to_string();
        assert!(
            printed.contains("0.950"),
            "the nominal level must print: {printed}"
        );
        assert!(
            printed.contains("32"),
            "the repetition count must print: {printed}"
        );
    }

    #[rstest]
    fn test_the_check_refuses_an_invalid_declaration() {
        assert!(
            PrefactorCoverageCheck::new(0.0, 0.9, 8, 1).is_err(),
            "a non-positive truth must be refused",
        );
        assert!(
            PrefactorCoverageCheck::new(1.0, 0.0, 8, 1).is_err(),
            "a zero nominal level must be refused",
        );
        assert!(
            PrefactorCoverageCheck::new(1.0, 1.1, 8, 1).is_err(),
            "a nominal level above one must be refused",
        );
        assert!(
            PrefactorCoverageCheck::new(1.0, 0.9, 0, 1).is_err(),
            "zero repetitions must be refused",
        );
    }
}
