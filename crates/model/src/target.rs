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

//! Portfolio target values.
//!
//! A target states a desired exposure for one instrument: an absolute quantity, a fraction of
//! portfolio equity, or a notional value. A target is not an order. It carries no order type, no
//! price, and no execution instruction; a reconciler compares it with cache and portfolio state
//! and emits the minimal order set. The cache and the portfolio remain authoritative for position
//! state, and a target never becomes a second position store.
//!
//! A target is also not a signal: a signal is a statement of view, a target is the exposure that
//! view resolves to, and an order is how the exposure is reached.

use std::fmt::Display;

use anyhow::{Result, ensure};
use nautilus_core::UnixNanos;

use crate::{
    identifiers::InstrumentId,
    types::{Money, Quantity},
};

/// The desired exposure of a [`Target`].
#[derive(Clone, Debug, PartialEq)]
pub enum TargetValue {
    /// An absolute quantity to hold.
    Quantity(Quantity),
    /// A fraction of portfolio equity to hold, where `1.0` is the full equity of the portfolio and
    /// values above one imply leverage.
    Weight(f64),
    /// A notional value to hold, in the currency of the money amount.
    Notional(Money),
}

impl TargetValue {
    /// Returns the canonical name of the value kind.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        match self {
            Self::Quantity(_) => "QUANTITY",
            Self::Weight(_) => "WEIGHT",
            Self::Notional(_) => "NOTIONAL",
        }
    }

    /// Returns the target quantity, if this is a quantity target.
    #[must_use]
    pub const fn quantity(&self) -> Option<Quantity> {
        match self {
            Self::Quantity(quantity) => Some(*quantity),
            Self::Weight(_) | Self::Notional(_) => None,
        }
    }

    /// Returns the target weight, if this is a weight target.
    #[must_use]
    pub const fn weight(&self) -> Option<f64> {
        match self {
            Self::Weight(weight) => Some(*weight),
            Self::Quantity(_) | Self::Notional(_) => None,
        }
    }

    /// Returns the target notional value, if this is a notional target.
    #[must_use]
    pub const fn notional(&self) -> Option<Money> {
        match self {
            Self::Notional(notional) => Some(*notional),
            Self::Quantity(_) | Self::Weight(_) => None,
        }
    }

    /// Returns whether the target is a zero exposure.
    #[must_use]
    pub fn is_flat(&self) -> bool {
        match self {
            Self::Quantity(quantity) => quantity.is_zero(),
            Self::Weight(weight) => *weight == 0.0,
            Self::Notional(notional) => notional.is_zero(),
        }
    }
}

impl Display for TargetValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Quantity(quantity) => write!(f, "QUANTITY({quantity})"),
            Self::Weight(weight) => write!(f, "WEIGHT({weight})"),
            Self::Notional(notional) => write!(f, "NOTIONAL({notional})"),
        }
    }
}

/// A desired exposure for one instrument.
///
/// A target is not an order and is not a trading command: it expresses the exposure to reach, and
/// the order layer decides how to reach it. The cache and the portfolio remain authoritative for
/// position state.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.model", from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.model")
)]
pub struct Target {
    instrument_id: InstrumentId,
    value: TargetValue,
    ts_event: UnixNanos,
    ts_init: UnixNanos,
}

impl Target {
    /// Creates a new [`Target`].
    ///
    /// # Errors
    ///
    /// Returns an error if the value is a weight that is not finite.
    pub fn new(
        instrument_id: InstrumentId,
        value: TargetValue,
        ts_event: UnixNanos,
        ts_init: UnixNanos,
    ) -> Result<Self> {
        if let TargetValue::Weight(weight) = &value {
            ensure!(weight.is_finite(), "weight {weight} is not finite");
        }

        Ok(Self {
            instrument_id,
            value,
            ts_event,
            ts_init,
        })
    }

    /// Creates a new [`Target`] holding an absolute quantity.
    ///
    /// # Errors
    ///
    /// Returns an error if the value is rejected, which cannot happen for a quantity.
    pub fn from_quantity(
        instrument_id: InstrumentId,
        quantity: Quantity,
        ts_event: UnixNanos,
        ts_init: UnixNanos,
    ) -> Result<Self> {
        Self::new(
            instrument_id,
            TargetValue::Quantity(quantity),
            ts_event,
            ts_init,
        )
    }

    /// Creates a new [`Target`] holding a fraction of portfolio equity.
    ///
    /// # Errors
    ///
    /// Returns an error if the weight is not finite.
    pub fn from_weight(
        instrument_id: InstrumentId,
        weight: f64,
        ts_event: UnixNanos,
        ts_init: UnixNanos,
    ) -> Result<Self> {
        Self::new(
            instrument_id,
            TargetValue::Weight(weight),
            ts_event,
            ts_init,
        )
    }

    /// Creates a new [`Target`] holding a notional value.
    ///
    /// # Errors
    ///
    /// Returns an error if the value is rejected, which cannot happen for a money amount.
    pub fn from_notional(
        instrument_id: InstrumentId,
        notional: Money,
        ts_event: UnixNanos,
        ts_init: UnixNanos,
    ) -> Result<Self> {
        Self::new(
            instrument_id,
            TargetValue::Notional(notional),
            ts_event,
            ts_init,
        )
    }

    /// Returns the instrument the target applies to.
    #[must_use]
    pub const fn instrument_id(&self) -> InstrumentId {
        self.instrument_id
    }

    /// Returns the desired exposure.
    #[must_use]
    pub const fn value(&self) -> &TargetValue {
        &self.value
    }

    /// Returns the canonical name of the value kind.
    #[must_use]
    pub const fn kind(&self) -> &'static str {
        self.value.kind()
    }

    /// Returns the instant the target was stated.
    #[must_use]
    pub const fn ts_event(&self) -> UnixNanos {
        self.ts_event
    }

    /// Returns the instant the instance was created.
    #[must_use]
    pub const fn ts_init(&self) -> UnixNanos {
        self.ts_init
    }

    /// Returns whether the target is a zero exposure.
    #[must_use]
    pub fn is_flat(&self) -> bool {
        self.value.is_flat()
    }
}

impl Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Target(instrument_id={}, value={}, ts_event={}, ts_init={})",
            self.instrument_id, self.value, self.ts_event, self.ts_init
        )
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    fn instrument_id() -> InstrumentId {
        InstrumentId::from("AAPL.XNYS")
    }

    #[rstest]
    fn test_from_quantity_exposes_the_exposure() {
        let target = Target::from_quantity(
            instrument_id(),
            Quantity::from("1.5"),
            1_000.into(),
            2_000.into(),
        )
        .unwrap();

        assert_eq!(target.instrument_id(), instrument_id());
        assert_eq!(target.kind(), "QUANTITY");
        assert_eq!(target.value().quantity(), Some(Quantity::from("1.5")));
        assert_eq!(target.value().weight(), None);
        assert_eq!(target.value().notional(), None);
        assert_eq!(target.ts_event(), UnixNanos::from(1_000));
        assert_eq!(target.ts_init(), UnixNanos::from(2_000));
    }

    #[rstest]
    fn test_from_weight_exposes_the_exposure() {
        let target = Target::from_weight(instrument_id(), 0.25, 1.into(), 2.into()).unwrap();

        assert_eq!(target.kind(), "WEIGHT");
        assert_eq!(target.value().weight(), Some(0.25));
        assert_eq!(target.value().quantity(), None);
        assert_eq!(target.value().notional(), None);
    }

    #[rstest]
    fn test_from_notional_exposes_the_exposure() {
        let notional = Money::from("1000 USD");
        let target = Target::from_notional(instrument_id(), notional, 1.into(), 2.into()).unwrap();

        assert_eq!(target.kind(), "NOTIONAL");
        assert_eq!(target.value().notional(), Some(notional));
        assert_eq!(target.value().quantity(), None);
        assert_eq!(target.value().weight(), None);
    }

    #[rstest]
    fn test_new_accepts_each_value_kind() {
        let quantity = Target::new(
            instrument_id(),
            TargetValue::Quantity(Quantity::from("2")),
            1.into(),
            2.into(),
        )
        .unwrap();
        let weight = Target::new(
            instrument_id(),
            TargetValue::Weight(-0.5),
            1.into(),
            2.into(),
        )
        .unwrap();

        assert_eq!(quantity.kind(), "QUANTITY");
        assert_eq!(weight.kind(), "WEIGHT");
    }

    #[rstest]
    #[case(f64::NAN)]
    #[case(f64::INFINITY)]
    #[case(f64::NEG_INFINITY)]
    fn test_rejects_a_non_finite_weight(#[case] weight: f64) {
        assert!(Target::from_weight(instrument_id(), weight, 1.into(), 2.into()).is_err());
        assert!(
            Target::new(
                instrument_id(),
                TargetValue::Weight(weight),
                1.into(),
                2.into(),
            )
            .is_err()
        );
    }

    #[rstest]
    fn test_is_flat_for_a_zero_quantity() {
        let flat =
            Target::from_quantity(instrument_id(), Quantity::zero(2), 1.into(), 2.into()).unwrap();
        let open =
            Target::from_quantity(instrument_id(), Quantity::from("0.01"), 1.into(), 2.into())
                .unwrap();

        assert!(flat.is_flat());
        assert!(!open.is_flat());
    }

    #[rstest]
    fn test_is_flat_for_a_zero_weight() {
        let flat = Target::from_weight(instrument_id(), 0.0, 1.into(), 2.into()).unwrap();
        let open = Target::from_weight(instrument_id(), -0.25, 1.into(), 2.into()).unwrap();

        assert!(flat.is_flat());
        assert!(!open.is_flat());
    }

    #[rstest]
    fn test_is_flat_for_a_zero_notional() {
        let flat = Target::from_notional(instrument_id(), Money::from("0 USD"), 1.into(), 2.into())
            .unwrap();
        let open =
            Target::from_notional(instrument_id(), Money::from("-100 USD"), 1.into(), 2.into())
                .unwrap();

        assert!(flat.is_flat());
        assert!(!open.is_flat());
    }

    #[rstest]
    fn test_value_kind_names_are_canonical() {
        assert_eq!(
            TargetValue::Quantity(Quantity::from("1")).kind(),
            "QUANTITY"
        );
        assert_eq!(TargetValue::Weight(0.5).kind(), "WEIGHT");
        assert_eq!(
            TargetValue::Notional(Money::from("1 USD")).kind(),
            "NOTIONAL"
        );
    }

    #[rstest]
    fn test_display_names_the_target() {
        let target = Target::from_weight(instrument_id(), 0.5, 1.into(), 2.into()).unwrap();

        assert_eq!(
            target.to_string(),
            "Target(instrument_id=AAPL.XNYS, value=WEIGHT(0.5), ts_event=1, ts_init=2)"
        );
    }

    #[rstest]
    fn test_value_display_names_the_kind() {
        let notional = Money::from("1000 USD");

        assert_eq!(
            TargetValue::Notional(notional).to_string(),
            format!("NOTIONAL({notional})")
        );
    }
}
