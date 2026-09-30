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
//! The model is deterministic: a linear impact model is a pure function of the fill quantity
//! and takes no random seed.

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

/// The built-in market impact models selectable by configuration.
///
/// The variants correspond one-for-one with the built-in market impact model implementations.
#[derive(Clone, Debug)]
pub enum MarketImpactModelAny {
    Linear(LinearMarketImpactModel),
}

impl MarketImpactModel for MarketImpactModelAny {
    fn impact_increments(&mut self, fill_quantity: Quantity) -> anyhow::Result<u64> {
        match self {
            Self::Linear(model) => model.impact_increments(fill_quantity),
        }
    }
}

impl Display for MarketImpactModelAny {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Linear(model) => write!(f, "{model}"),
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
}
