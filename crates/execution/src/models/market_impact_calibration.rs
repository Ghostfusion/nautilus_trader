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

#[cfg(test)]
mod tests {
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
}
