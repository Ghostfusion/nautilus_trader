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

use std::{
    cell::RefCell,
    fmt::{Debug, Display},
    rc::Rc,
};

use crate::models::fill::ProbabilisticFillState;

/// Trait for slippage models used in backtesting.
///
/// A slippage model owns one concern of execution realism: whether a fill moves against the
/// order direction once the fill model has decided that the order is eligible, how much fills,
/// and at what base price. The adjustment is one price increment on an L1 book, and the adjusted
/// price is the final fill price the fee model is charged on.
///
/// The concern composes after the fill model: fill eligibility, then fill quantity, then base
/// fill price, then the slippage adjustment, then fees.
pub trait SlippageModel {
    /// Returns `true` if the fill price should slip by one tick against the order direction.
    ///
    /// # Errors
    ///
    /// Returns an error if the model cannot determine whether the fill should slip.
    fn is_slipped(&mut self) -> anyhow::Result<bool>;
}

/// Shared runtime handle for a slippage model.
#[derive(Clone)]
pub struct SlippageModelHandle(Rc<RefCell<dyn SlippageModel>>);

impl SlippageModelHandle {
    /// Creates a new [`SlippageModelHandle`] from a slippage model.
    #[must_use]
    pub fn new<T>(model: T) -> Self
    where
        T: SlippageModel + 'static,
    {
        Self(Rc::new(RefCell::new(model)))
    }

    /// Creates a new [`SlippageModelHandle`] from an existing reference-counted model.
    #[must_use]
    pub fn from_rc(model: Rc<RefCell<dyn SlippageModel>>) -> Self {
        Self(model)
    }
}

impl Debug for SlippageModelHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(stringify!(SlippageModelHandle))
            .field(&"<dyn SlippageModel>")
            .finish()
    }
}

impl SlippageModel for SlippageModelHandle {
    fn is_slipped(&mut self) -> anyhow::Result<bool> {
        self.0.borrow_mut().is_slipped()
    }
}

/// A probabilistic one-tick slippage model.
///
/// The model draws a one-tick adverse adjustment with probability `prob_slippage` on each L1
/// fill. It draws from the same seeded probabilistic state the fill models use, so a seeded
/// model reproduces its draws across runs and the same probability and seed as a composite fill
/// model's folded-in slippage produce the same draws.
#[derive(Debug)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.execution", unsendable, from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.execution")
)]
pub struct ProbabilisticSlippageModel {
    state: ProbabilisticFillState,
}

impl ProbabilisticSlippageModel {
    /// Creates a new [`ProbabilisticSlippageModel`] instance.
    ///
    /// # Errors
    ///
    /// Returns an error if `prob_slippage` is not in range [0, 1].
    pub fn new(prob_slippage: f64, random_seed: Option<u64>) -> anyhow::Result<Self> {
        // The limit-fill probability is deterministic: a slippage model only draws slippage.
        Ok(Self {
            state: ProbabilisticFillState::new(1.0, prob_slippage, random_seed)?,
        })
    }
}

impl Clone for ProbabilisticSlippageModel {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
        }
    }
}

impl Default for ProbabilisticSlippageModel {
    fn default() -> Self {
        Self::new(0.0, None).unwrap()
    }
}

impl Display for ProbabilisticSlippageModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ProbabilisticSlippageModel")
    }
}

impl SlippageModel for ProbabilisticSlippageModel {
    fn is_slipped(&mut self) -> anyhow::Result<bool> {
        Ok(self.state.is_slipped())
    }
}

/// The built-in slippage models selectable by configuration.
///
/// The variants correspond one-for-one with the built-in slippage model implementations.
#[derive(Clone, Debug)]
pub enum SlippageModelAny {
    Probabilistic(ProbabilisticSlippageModel),
}

impl SlippageModel for SlippageModelAny {
    fn is_slipped(&mut self) -> anyhow::Result<bool> {
        match self {
            Self::Probabilistic(model) => model.is_slipped(),
        }
    }
}

impl Display for SlippageModelAny {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Probabilistic(model) => write!(f, "{model}"),
        }
    }
}

impl From<SlippageModelAny> for SlippageModelHandle {
    fn from(model: SlippageModelAny) -> Self {
        Self::new(model)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    fn test_probabilistic_slippage_model_is_deterministic_when_disabled() {
        let mut model = ProbabilisticSlippageModel::new(0.0, None).unwrap();

        for _ in 0..10 {
            assert!(!model.is_slipped().unwrap());
        }
    }

    #[rstest]
    fn test_probabilistic_slippage_model_rejects_probability_out_of_range() {
        assert!(ProbabilisticSlippageModel::new(1.5, None).is_err());
        assert!(ProbabilisticSlippageModel::new(-0.1, None).is_err());
    }

    #[rstest]
    fn test_seeded_probabilistic_slippage_model_reproduces_draws() {
        let mut first = ProbabilisticSlippageModel::new(0.5, Some(42)).unwrap();
        let mut second = ProbabilisticSlippageModel::new(0.5, Some(42)).unwrap();

        let first_draws: Vec<bool> = (0..32).map(|_| first.is_slipped().unwrap()).collect();
        let second_draws: Vec<bool> = (0..32).map(|_| second.is_slipped().unwrap()).collect();

        assert_eq!(first_draws, second_draws);
        assert!(first_draws.iter().any(|slipped| *slipped));
    }

    #[rstest]
    fn test_seeded_slippage_model_matches_fill_model_slippage_draws() {
        // A composite fill model with a deterministic limit-fill decision draws its slippage
        // from the same seeded stream, so the separated model reproduces the composite draws.
        use crate::models::fill::{DefaultFillModel, FillModel};

        let mut fill_model = DefaultFillModel::new(1.0, 0.5, Some(7)).unwrap();
        let mut slippage_model = ProbabilisticSlippageModel::new(0.5, Some(7)).unwrap();

        let fill_draws: Vec<bool> = (0..32).map(|_| fill_model.is_slipped().unwrap()).collect();
        let slippage_draws: Vec<bool> = (0..32)
            .map(|_| slippage_model.is_slipped().unwrap())
            .collect();

        assert_eq!(fill_draws, slippage_draws);
    }

    #[rstest]
    fn test_slippage_model_any_dispatches_and_converts_to_handle() {
        let model =
            SlippageModelAny::Probabilistic(ProbabilisticSlippageModel::new(1.0, None).unwrap());
        let mut handle: SlippageModelHandle = model.into();

        assert!(handle.is_slipped().unwrap());
    }
}
