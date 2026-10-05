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

/// A concave market impact model with a square-root shape.
///
/// The model moves the fill price against the order direction by the prefactor times the square
/// root of the fill's size relative to the reference quantity, capped at `max_increments`:
///
/// ```text
/// increments = floor(prefactor * sqrt(fill_quantity / reference_quantity))
/// ```
///
/// The exponent is fixed at one half rather than exposed as a parameter, because the corpus finds
/// the exponent robust across markets while the prefactor is not: a tape-derived calibration
/// reports the exponent as 0.489 +/- 0.0015 and 0.50 [0.32, 0.66], against a prefactor running
/// from 0.34 to 1.50 across three markets. The parameter that carries the uncertainty is therefore
/// the prefactor, which the calibration tooling reports as an interval rather than a point.
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
    prefactor: f64,
    reference_quantity: Quantity,
    max_increments: u64,
}

impl SquareRootMarketImpactModel {
    /// Creates a new [`SquareRootMarketImpactModel`] instance.
    ///
    /// # Errors
    ///
    /// Returns an error if `prefactor` is not finite and greater than zero, if
    /// `reference_quantity` is zero, or if `max_increments` is zero.
    pub fn new(
        prefactor: f64,
        reference_quantity: Quantity,
        max_increments: u64,
    ) -> anyhow::Result<Self> {
        if !prefactor.is_finite() || prefactor <= 0.0 {
            anyhow::bail!("prefactor must be finite and greater than zero");
        }
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

    /// Returns the dimensionless prefactor applied to the square root of the relative size.
    #[must_use]
    pub const fn prefactor(&self) -> f64 {
        self.prefactor
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
        write!(f, "SquareRootMarketImpactModel")
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

        let increments = (self.prefactor * relative.sqrt()).floor();
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

    #[rstest]
    fn test_square_root_impact_is_concave_in_the_fill_quantity() {
        let mut model =
            SquareRootMarketImpactModel::new(100.0, Quantity::from("100"), 1_000).expect("valid");

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
            SquareRootMarketImpactModel::new(10.0, Quantity::from("1"), 3).expect("valid");

        assert_eq!(model.impact_increments(Quantity::from("10000")).unwrap(), 3);
    }

    #[rstest]
    fn test_square_root_impact_leaves_a_fill_below_one_increment_unchanged() {
        let mut model =
            SquareRootMarketImpactModel::new(2.0, Quantity::from("100"), 10).expect("valid");

        // floor(2 * sqrt(1 / 100)) = 0.
        assert_eq!(model.impact_increments(Quantity::from("1")).unwrap(), 0);
        assert_eq!(model.impact_increments(Quantity::from("0")).unwrap(), 0);
    }

    #[rstest]
    fn test_square_root_impact_carries_decimal_quantity_exactly() {
        let mut model =
            SquareRootMarketImpactModel::new(1.0, Quantity::from("0.1"), 10).expect("valid");

        // floor(sqrt(2.5)) = 1.
        assert_eq!(model.impact_increments(Quantity::from("0.25")).unwrap(), 1);
    }

    #[rstest]
    fn test_square_root_impact_is_schedule_invariant() {
        let mut model =
            SquareRootMarketImpactModel::new(100.0, Quantity::from("100"), 1_000).expect("valid");
        let mut reference =
            SquareRootMarketImpactModel::new(100.0, Quantity::from("100"), 1_000).expect("valid");

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
            SquareRootMarketImpactModel::new(0.0, Quantity::from("1"), 1).is_err(),
            "a zero prefactor must be rejected",
        );
        assert!(
            SquareRootMarketImpactModel::new(-1.0, Quantity::from("1"), 1).is_err(),
            "a negative prefactor must be rejected",
        );
        assert!(
            SquareRootMarketImpactModel::new(f64::NAN, Quantity::from("1"), 1).is_err(),
            "a NaN prefactor must be rejected",
        );
        assert!(
            SquareRootMarketImpactModel::new(f64::INFINITY, Quantity::from("1"), 1).is_err(),
            "an infinite prefactor must be rejected",
        );
        assert!(
            SquareRootMarketImpactModel::new(1.0, Quantity::from("0"), 1).is_err(),
            "a zero reference quantity must be rejected",
        );
        assert!(
            SquareRootMarketImpactModel::new(1.0, Quantity::from("1"), 0).is_err(),
            "zero max_increments must be rejected",
        );
    }

    #[rstest]
    fn test_square_root_impact_exposes_parameters() {
        let model =
            SquareRootMarketImpactModel::new(0.69, Quantity::from("500"), 12).expect("valid");

        assert!((model.prefactor() - 0.69).abs() < f64::EPSILON);
        assert_eq!(model.reference_quantity(), Quantity::from("500"));
        assert_eq!(model.max_increments(), 12);
    }

    #[rstest]
    fn test_any_dispatches_to_square_root_model() {
        let mut any = MarketImpactModelAny::SquareRoot(
            SquareRootMarketImpactModel::new(30.0, Quantity::from("100"), 50).expect("valid"),
        );

        // floor(30 * sqrt(4)) = 60, capped at 50.
        assert_eq!(any.impact_increments(Quantity::from("400")).unwrap(), 50);
    }
}
