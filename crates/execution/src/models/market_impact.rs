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

//! Market impact models.
//!
//! A market impact model owns one concern of execution realism: how far the fill price moves
//! against the order direction because of the size a liquidity-taking order consumes. The
//! adjustment is expressed in whole price increments, so it is exactly representable, and it
//! composes after the slippage adjustment, on the L1 fills a slippage model adjusts.
//!
//! A built-in model is deterministic: its adjustment is a pure function of the fill quantity and
//! the parameters it was configured with, and it takes no random seed. The linear model moves the
//! price by one increment per unit of size, while the square-root model moves it by the prefactor
//! times the square root of the size relative to its reference, which is concave in the size.

use std::{
    cell::RefCell,
    fmt::{Debug, Display},
    rc::Rc,
};

use nautilus_model::types::Quantity;
use rust_decimal::{Decimal, prelude::ToPrimitive};

/// Trait for market impact models used in backtesting.
///
/// A market impact model owns one concern of execution realism: whether the fill price moves
/// against the order direction, and by how many price increments, once the fill model has
/// decided that the order is eligible, how much fills, and at what base price, and the
/// slippage model has applied its own adjustment.
///
/// The concern composes after the slippage adjustment: fill eligibility, then fill quantity,
/// then base fill price, then the slippage adjustment, then the market impact adjustment, then
/// the final fill price, then fees.
pub trait MarketImpactModel {
    /// Returns the number of price increments the fill price moves against the order direction.
    ///
    /// A return of zero leaves the fill price unchanged.
    ///
    /// # Errors
    ///
    /// Returns an error if the model cannot determine the impact.
    fn impact_increments(&mut self, fill_quantity: Quantity) -> anyhow::Result<u64>;
}

/// Shared runtime handle for a market impact model.
#[derive(Clone)]
pub struct MarketImpactModelHandle(Rc<RefCell<dyn MarketImpactModel>>);

impl MarketImpactModelHandle {
    /// Creates a new [`MarketImpactModelHandle`] from a market impact model.
    #[must_use]
    pub fn new<T>(model: T) -> Self
    where
        T: MarketImpactModel + 'static,
    {
        Self(Rc::new(RefCell::new(model)))
    }

    /// Creates a new [`MarketImpactModelHandle`] from an existing reference-counted model.
    #[must_use]
    pub fn from_rc(model: Rc<RefCell<dyn MarketImpactModel>>) -> Self {
        Self(model)
    }
}

impl Debug for MarketImpactModelHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(stringify!(MarketImpactModelHandle))
            .field(&"<dyn MarketImpactModel>")
            .finish()
    }
}

impl MarketImpactModel for MarketImpactModelHandle {
    fn impact_increments(&mut self, fill_quantity: Quantity) -> anyhow::Result<u64> {
        self.0.borrow_mut().impact_increments(fill_quantity)
    }
}

/// A linear market impact model.
///
/// The model moves the fill price against the order direction by one price increment for
/// every `quantity_per_increment` units filled, capped at `max_increments`. A fill smaller
/// than one increment quantity leaves the price unchanged, and the adjustment grows in whole
/// increments with the fill size, so a larger order is filled further through the book.
///
/// The model is deterministic and takes no random seed: the adjustment is an exact function
/// of the fill quantity, computed with decimal arithmetic against the configured reference
/// quantity.
#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.execution", unsendable, from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.execution")
)]
pub struct LinearMarketImpactModel {
    quantity_per_increment: Quantity,
    max_increments: u64,
}

impl LinearMarketImpactModel {
    /// Creates a new [`LinearMarketImpactModel`] instance.
    ///
    /// # Errors
    ///
    /// Returns an error if `quantity_per_increment` is zero or `max_increments` is zero.
    pub fn new(quantity_per_increment: Quantity, max_increments: u64) -> anyhow::Result<Self> {
        if quantity_per_increment.is_zero() {
            anyhow::bail!("quantity_per_increment must be greater than zero");
        }
        if max_increments == 0 {
            anyhow::bail!("max_increments must be greater than zero");
        }
        Ok(Self {
            quantity_per_increment,
            max_increments,
        })
    }

    /// Returns the fill quantity that moves the fill price by one price increment.
    #[must_use]
    pub const fn quantity_per_increment(&self) -> Quantity {
        self.quantity_per_increment
    }

    /// Returns the maximum number of price increments a fill price may move.
    #[must_use]
    pub const fn max_increments(&self) -> u64 {
        self.max_increments
    }
}

impl Display for LinearMarketImpactModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LinearMarketImpactModel")
    }
}

impl MarketImpactModel for LinearMarketImpactModel {
    fn impact_increments(&mut self, fill_quantity: Quantity) -> anyhow::Result<u64> {
        let unit = self.quantity_per_increment.as_decimal();
        if unit <= Decimal::ZERO {
            return Ok(0);
        }

        let increments = (fill_quantity.as_decimal() / unit).floor();
        let increments = increments.to_u64().unwrap_or(self.max_increments);

        Ok(increments.min(self.max_increments))
    }
}

/// Where a calibrated prefactor came from.
///
/// The provenance travels with the value because the same number means different things depending
/// on how it was obtained: a prefactor fitted from the venue's own fills is measured, because the
/// aggressor and the metaorder are observable, while one reconstructed from an anonymous tape is
/// inferred, and reconstructing metaorders from an anonymous tape inflates it about twofold unless
/// the de-bias is applied as an explicit step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(
        frozen,
        eq,
        eq_int,
        module = "nautilus_trader.execution",
        from_py_object,
        rename_all = "SCREAMING_SNAKE_CASE",
    )
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass_enum(module = "nautilus_trader.execution")
)]
pub enum ImpactCalibrationSource {
    /// Fitted from the venue's own fills, where the aggressor and the metaorder are observable.
    Fills,
    /// Reconstructed from an anonymous tape, with the de-bias applied as an explicit step.
    AnonymousTapeDebiased,
    /// Reconstructed from an anonymous tape without the de-bias, so the value is inflated.
    AnonymousTape,
    /// Declared by the caller rather than fitted, e.g. a range read from the literature.
    Assumed,
}

impl Display for ImpactCalibrationSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Fills => "fitted from fills",
            Self::AnonymousTapeDebiased => "anonymous tape, de-biased",
            Self::AnonymousTape => "anonymous tape, not de-biased",
            Self::Assumed => "assumed",
        })
    }
}

/// A calibrated prefactor with the interval it was measured over and its provenance.
///
/// The prefactor is uncertain by nature: the corpus finds it running from 0.34 to 1.50 across
/// markets, and reconstructing metaorders from an anonymous tape inflates it about twofold, so the
/// honest output is a bounded range rather than a point. An interval whose bounds are equal is a
/// point calibration and is allowed, because a prefactor fitted from observable fills really is a
/// point; the source is what records which of the two a caller is looking at.
///
/// A model applies [`Self::applied`], which is the upper bound, so an uncertain prefactor cannot
/// flatter a simulated result.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.execution", from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.execution")
)]
pub struct PrefactorInterval {
    lower: f64,
    upper: f64,
    source: ImpactCalibrationSource,
}

impl PrefactorInterval {
    /// Creates a new [`PrefactorInterval`] instance.
    ///
    /// # Errors
    ///
    /// Returns an error if either bound is not finite and greater than zero, or if `lower` is
    /// greater than `upper`.
    pub fn new(lower: f64, upper: f64, source: ImpactCalibrationSource) -> anyhow::Result<Self> {
        if !lower.is_finite() || lower <= 0.0 {
            anyhow::bail!("prefactor lower bound must be finite and greater than zero");
        }
        if !upper.is_finite() || upper <= 0.0 {
            anyhow::bail!("prefactor upper bound must be finite and greater than zero");
        }
        if lower > upper {
            anyhow::bail!("prefactor lower bound must not exceed the upper bound");
        }
        Ok(Self {
            lower,
            upper,
            source,
        })
    }

    /// Creates a point calibration from a single prefactor.
    ///
    /// # Errors
    ///
    /// Returns an error if `prefactor` is not finite and greater than zero.
    pub fn point(prefactor: f64, source: ImpactCalibrationSource) -> anyhow::Result<Self> {
        Self::new(prefactor, prefactor, source)
    }

    /// Returns the lower bound of the calibrated prefactor.
    #[must_use]
    pub const fn lower(&self) -> f64 {
        self.lower
    }

    /// Returns the upper bound of the calibrated prefactor.
    #[must_use]
    pub const fn upper(&self) -> f64 {
        self.upper
    }

    /// Returns the bound a model applies, which is the upper one.
    #[must_use]
    pub const fn applied(&self) -> f64 {
        self.upper
    }

    /// Returns where the interval came from.
    #[must_use]
    pub const fn source(&self) -> ImpactCalibrationSource {
        self.source
    }

    /// Returns whether the interval is a point.
    #[must_use]
    #[expect(
        clippy::float_cmp,
        reason = "a point calibration is constructed with equal bounds, so this comparison is exact by construction"
    )]
    pub fn is_point(&self) -> bool {
        self.lower == self.upper
    }
}

impl Display for PrefactorInterval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_point() {
            write!(f, "prefactor {} ({})", self.upper, self.source)
        } else {
            write!(
                f,
                "prefactor {} to {} ({})",
                self.lower, self.upper, self.source
            )
        }
    }
}

/// A concave market impact model with a square-root shape.
///
/// The model moves the fill price against the order direction by the prefactor times the square
/// root of the fill's size relative to the reference quantity, capped at `max_increments`:
///
/// ```text
/// increments = floor(prefactor * sqrt(fill_quantity / reference_quantity))
/// ```
///
/// The prefactor is a [`PrefactorInterval`] rather than a number, and the model applies its upper
/// bound, so a prefactor inferred from an anonymous tape cannot be read as measured and cannot
/// flatter a result. The interval's bounds and source are part of the model's display, so a report
/// that prints the model prints what the calibration is worth.
///
/// The exponent is fixed at one half rather than exposed as a parameter, because the corpus finds
/// the exponent robust across markets while the prefactor is not: a tape-derived calibration
/// reports the exponent as 0.489 +/- 0.0015 and 0.50 [0.32, 0.66], against a prefactor running
/// from 0.34 to 1.50 across three markets.
///
/// The shape is what makes the model concave: doubling the filled quantity multiplies the impact
/// by the square root of two rather than by two, so a larger fill is filled less far through the
/// book per unit than a smaller one. The model keeps no state between calls, so the adjustment a
/// fill receives does not depend on the fills observed around it.
///
/// The square root is evaluated in binary floating point and floored to whole increments, so the
/// count that leaves the model is exact while the ratio under it is not rational in general. A
/// fill whose continuous impact is below one increment leaves the price unchanged.
#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.execution", unsendable, from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.execution")
)]
pub struct SquareRootMarketImpactModel {
    prefactor: PrefactorInterval,
    reference_quantity: Quantity,
    max_increments: u64,
}

impl SquareRootMarketImpactModel {
    /// Creates a new [`SquareRootMarketImpactModel`] instance.
    ///
    /// # Errors
    ///
    /// Returns an error if `reference_quantity` or `max_increments` is zero. The interval
    /// validates its own bounds when it is constructed.
    pub fn new(
        prefactor: PrefactorInterval,
        reference_quantity: Quantity,
        max_increments: u64,
    ) -> anyhow::Result<Self> {
        if reference_quantity.is_zero() {
            anyhow::bail!("reference_quantity must be greater than zero");
        }
        if max_increments == 0 {
            anyhow::bail!("max_increments must be greater than zero");
        }
        Ok(Self {
            prefactor,
            reference_quantity,
            max_increments,
        })
    }

    /// Returns the calibrated prefactor, with its bounds and its source.
    #[must_use]
    pub const fn prefactor(&self) -> PrefactorInterval {
        self.prefactor
    }

    /// Returns the prefactor bound the model applies, which is the interval's upper bound.
    #[must_use]
    pub fn applied_prefactor(&self) -> f64 {
        self.prefactor.applied()
    }

    /// Returns the fill quantity that moves the fill price by the prefactor's worth of increments.
    #[must_use]
    pub const fn reference_quantity(&self) -> Quantity {
        self.reference_quantity
    }

    /// Returns the maximum number of price increments a fill price may move.
    #[must_use]
    pub const fn max_increments(&self) -> u64 {
        self.max_increments
    }
}

impl Display for SquareRootMarketImpactModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SquareRootMarketImpactModel, {}", self.prefactor)
    }
}

impl MarketImpactModel for SquareRootMarketImpactModel {
    fn impact_increments(&mut self, fill_quantity: Quantity) -> anyhow::Result<u64> {
        let reference = self.reference_quantity.as_decimal();
        if reference <= Decimal::ZERO {
            return Ok(0);
        }

        // A negative or zero fill cannot move the price in this model's own terms; the order
        // direction, not the model, decides which way a positive adjustment is applied.
        let relative = (fill_quantity.as_decimal() / reference)
            .to_f64()
            .unwrap_or(0.0);
        if relative <= 0.0 {
            return Ok(0);
        }

        let increments = (self.prefactor.applied() * relative.sqrt()).floor();
        if !increments.is_finite() {
            return Ok(self.max_increments);
        }

        // The floor leaves a whole count, so the conversion cannot lose precision; the fallback
        // keeps it total without an `as` cast, which the lint set forbids.
        let count = Decimal::try_from(increments)
            .ok()
            .and_then(|value| value.to_u64())
            .unwrap_or(self.max_increments);

        Ok(count.min(self.max_increments))
    }
}

/// The built-in market impact models selectable by configuration.
///
/// The variants correspond one-for-one with the built-in market impact model implementations.
#[derive(Clone, Debug)]
pub enum MarketImpactModelAny {
    Linear(LinearMarketImpactModel),
    SquareRoot(SquareRootMarketImpactModel),
}

impl MarketImpactModel for MarketImpactModelAny {
    fn impact_increments(&mut self, fill_quantity: Quantity) -> anyhow::Result<u64> {
        match self {
            Self::Linear(model) => model.impact_increments(fill_quantity),
            Self::SquareRoot(model) => model.impact_increments(fill_quantity),
        }
    }
}

impl Display for MarketImpactModelAny {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Linear(model) => write!(f, "{model}"),
            Self::SquareRoot(model) => write!(f, "{model}"),
        }
    }
}

impl From<MarketImpactModelAny> for MarketImpactModelHandle {
    fn from(model: MarketImpactModelAny) -> Self {
        Self::new(model)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    fn test_linear_impact_scales_with_fill_quantity() {
        let mut model =
            LinearMarketImpactModel::new(Quantity::from("100"), 10).expect("valid model");

        assert_eq!(model.impact_increments(Quantity::from("0")).unwrap(), 0);
        assert_eq!(model.impact_increments(Quantity::from("50")).unwrap(), 0);
        assert_eq!(model.impact_increments(Quantity::from("99")).unwrap(), 0);
        assert_eq!(model.impact_increments(Quantity::from("100")).unwrap(), 1);
        assert_eq!(model.impact_increments(Quantity::from("199")).unwrap(), 1);
        assert_eq!(model.impact_increments(Quantity::from("200")).unwrap(), 2);
    }

    #[rstest]
    fn test_linear_impact_is_capped() {
        let mut model = LinearMarketImpactModel::new(Quantity::from("1"), 3).expect("valid model");

        assert_eq!(model.impact_increments(Quantity::from("100")).unwrap(), 3);
    }

    #[rstest]
    fn test_linear_impact_carries_decimal_quantity_exactly() {
        let mut model =
            LinearMarketImpactModel::new(Quantity::from("0.1"), 5).expect("valid model");

        assert_eq!(model.impact_increments(Quantity::from("0.25")).unwrap(), 2);
    }

    #[rstest]
    fn test_linear_impact_is_repeatable() {
        let mut first = LinearMarketImpactModel::new(Quantity::from("10"), 5).expect("valid model");
        let mut second =
            LinearMarketImpactModel::new(Quantity::from("10"), 5).expect("valid model");

        for _ in 0..8 {
            assert_eq!(
                first.impact_increments(Quantity::from("25")).unwrap(),
                second.impact_increments(Quantity::from("25")).unwrap(),
            );
        }
    }

    #[rstest]
    fn test_linear_impact_rejects_invalid_parameters() {
        assert!(
            LinearMarketImpactModel::new(Quantity::from("0"), 1).is_err(),
            "zero quantity_per_increment must be rejected",
        );
        assert!(
            LinearMarketImpactModel::new(Quantity::from("1"), 0).is_err(),
            "zero max_increments must be rejected",
        );
    }

    #[rstest]
    fn test_linear_impact_exposes_parameters() {
        let model = LinearMarketImpactModel::new(Quantity::from("50"), 4).expect("valid model");

        assert_eq!(model.quantity_per_increment(), Quantity::from("50"));
        assert_eq!(model.max_increments(), 4);
    }

    #[rstest]
    fn test_any_dispatches_to_linear_model() {
        let mut any = MarketImpactModelAny::Linear(
            LinearMarketImpactModel::new(Quantity::from("10"), 5).expect("valid model"),
        );

        assert_eq!(any.impact_increments(Quantity::from("30")).unwrap(), 3);
    }

    /// Returns a point prefactor interval for the tests that only exercise the model's shape.
    fn point(prefactor: f64) -> PrefactorInterval {
        PrefactorInterval::point(prefactor, ImpactCalibrationSource::Assumed).expect("valid point")
    }

    #[rstest]
    fn test_prefactor_interval_rejects_invalid_bounds() {
        assert!(
            PrefactorInterval::new(0.0, 1.0, ImpactCalibrationSource::Assumed).is_err(),
            "a zero lower bound must be rejected",
        );
        assert!(
            PrefactorInterval::new(1.0, -1.0, ImpactCalibrationSource::Assumed).is_err(),
            "a negative upper bound must be rejected",
        );
        assert!(
            PrefactorInterval::new(f64::NAN, 1.0, ImpactCalibrationSource::Assumed).is_err(),
            "a NaN bound must be rejected",
        );
        assert!(
            PrefactorInterval::new(1.0, f64::INFINITY, ImpactCalibrationSource::Assumed).is_err(),
            "an infinite bound must be rejected",
        );
        assert!(
            PrefactorInterval::new(0.69, 0.34, ImpactCalibrationSource::Assumed).is_err(),
            "a lower bound above the upper bound must be rejected",
        );
    }

    #[rstest]
    fn test_a_point_calibration_applies_its_value() {
        let interval =
            PrefactorInterval::point(4.0, ImpactCalibrationSource::Fills).expect("valid point");

        assert!(interval.is_point());
        assert_eq!(interval.lower(), 4.0);
        assert_eq!(interval.upper(), 4.0);
        assert_eq!(interval.source(), ImpactCalibrationSource::Fills);

        let mut model = SquareRootMarketImpactModel::new(interval, Quantity::from("100"), 10)
            .expect("valid model");

        assert_eq!(model.impact_increments(Quantity::from("100")).unwrap(), 4);
    }

    #[rstest]
    fn test_the_model_applies_the_upper_bound_of_the_interval() {
        let interval =
            PrefactorInterval::new(1.0, 4.0, ImpactCalibrationSource::AnonymousTapeDebiased)
                .expect("valid interval");
        let mut model = SquareRootMarketImpactModel::new(interval, Quantity::from("100"), 10)
            .expect("valid model");

        assert!((model.applied_prefactor() - 4.0).abs() < f64::EPSILON);
        assert_eq!(model.prefactor(), interval);

        // The upper bound is what moves the price: at the reference quantity the model returns
        // four increments, not the one the lower bound would have moved it by, so an inferred
        // prefactor cannot flatter a result.
        assert_eq!(model.impact_increments(Quantity::from("100")).unwrap(), 4);
    }

    #[rstest]
    fn test_the_interval_prints_both_bounds_with_its_source() {
        let interval =
            PrefactorInterval::new(0.34, 0.69, ImpactCalibrationSource::AnonymousTapeDebiased)
                .expect("valid interval");

        assert_eq!(
            interval.to_string(),
            "prefactor 0.34 to 0.69 (anonymous tape, de-biased)",
        );

        let point = PrefactorInterval::point(0.5, ImpactCalibrationSource::Fills).expect("valid");
        assert_eq!(point.to_string(), "prefactor 0.5 (fitted from fills)");
    }

    #[rstest]
    fn test_the_model_prints_the_interval_it_applies() {
        let interval = PrefactorInterval::new(0.34, 0.69, ImpactCalibrationSource::AnonymousTape)
            .expect("valid interval");
        let model = SquareRootMarketImpactModel::new(interval, Quantity::from("100"), 10)
            .expect("valid model");

        assert_eq!(
            model.to_string(),
            "SquareRootMarketImpactModel, prefactor 0.34 to 0.69 (anonymous tape, not de-biased)",
        );
    }

    #[rstest]
    fn test_square_root_impact_is_concave_in_the_fill_quantity() {
        let mut model =
            SquareRootMarketImpactModel::new(point(100.0), Quantity::from("100"), 1_000)
                .expect("valid model");

        let base = model.impact_increments(Quantity::from("100")).unwrap();
        let doubled = model.impact_increments(Quantity::from("200")).unwrap();
        let quadrupled = model.impact_increments(Quantity::from("400")).unwrap();

        // floor(100 * sqrt(1)) = 100 at the reference, floor(100 * sqrt(2)) = 141 when the fill
        // doubles, and floor(100 * sqrt(4)) = 200 when it quadruples.
        assert_eq!(base, 100);
        assert_eq!(doubled, 141);
        assert_eq!(quadrupled, 200);
        assert!(
            doubled < 2 * base,
            "doubling the fill must less than double the impact",
        );
        assert!(
            quadrupled < 4 * base,
            "quadrupling the fill must less than quadruple the impact",
        );
    }

    #[rstest]
    fn test_square_root_impact_is_capped() {
        let mut model =
            SquareRootMarketImpactModel::new(point(10.0), Quantity::from("1"), 3).expect("valid");

        assert_eq!(model.impact_increments(Quantity::from("10000")).unwrap(), 3);
    }

    #[rstest]
    fn test_square_root_impact_leaves_a_fill_below_one_increment_unchanged() {
        let mut model = SquareRootMarketImpactModel::new(point(2.0), Quantity::from("100"), 10)
            .expect("valid model");

        // floor(2 * sqrt(1 / 100)) = 0.
        assert_eq!(model.impact_increments(Quantity::from("1")).unwrap(), 0);
        assert_eq!(model.impact_increments(Quantity::from("0")).unwrap(), 0);
    }

    #[rstest]
    fn test_square_root_impact_carries_decimal_quantity_exactly() {
        let mut model = SquareRootMarketImpactModel::new(point(1.0), Quantity::from("0.1"), 10)
            .expect("valid model");

        // floor(sqrt(2.5)) = 1.
        assert_eq!(model.impact_increments(Quantity::from("0.25")).unwrap(), 1);
    }

    #[rstest]
    fn test_square_root_impact_is_schedule_invariant() {
        let mut model =
            SquareRootMarketImpactModel::new(point(100.0), Quantity::from("100"), 1_000)
                .expect("valid model");
        let mut reference =
            SquareRootMarketImpactModel::new(point(100.0), Quantity::from("100"), 1_000)
                .expect("valid model");

        // The adjustment a fill receives does not depend on the fills observed around it, so the
        // interleaved order returns the same values as the isolated one.
        let isolated = ["100", "400"].map(|quantity| {
            reference
                .impact_increments(Quantity::from(quantity))
                .unwrap()
        });

        let interleaved = [
            model.impact_increments(Quantity::from("400")).unwrap(),
            model.impact_increments(Quantity::from("900")).unwrap(),
            model.impact_increments(Quantity::from("100")).unwrap(),
        ];

        // floor(100 * sqrt(9)) = 300 for the fill neither of the isolated calls observes.
        assert_eq!(interleaved, [isolated[1], 300, isolated[0]]);
    }

    #[rstest]
    fn test_square_root_impact_rejects_invalid_parameters() {
        assert!(
            SquareRootMarketImpactModel::new(point(1.0), Quantity::from("0"), 1).is_err(),
            "a zero reference quantity must be rejected",
        );
        assert!(
            SquareRootMarketImpactModel::new(point(1.0), Quantity::from("1"), 0).is_err(),
            "zero max_increments must be rejected",
        );
    }

    #[rstest]
    fn test_square_root_impact_exposes_parameters() {
        let interval = PrefactorInterval::new(0.34, 0.69, ImpactCalibrationSource::Assumed)
            .expect("valid interval");
        let model = SquareRootMarketImpactModel::new(interval, Quantity::from("500"), 12)
            .expect("valid model");

        assert_eq!(model.prefactor(), interval);
        assert_eq!(model.reference_quantity(), Quantity::from("500"));
        assert_eq!(model.max_increments(), 12);
    }

    #[rstest]
    fn test_any_dispatches_to_square_root_model() {
        let mut any = MarketImpactModelAny::SquareRoot(
            SquareRootMarketImpactModel::new(point(30.0), Quantity::from("100"), 50)
                .expect("valid model"),
        );

        // floor(30 * sqrt(4)) = 60, capped at 50.
        assert_eq!(any.impact_increments(Quantity::from("400")).unwrap(), 50);
    }
}
