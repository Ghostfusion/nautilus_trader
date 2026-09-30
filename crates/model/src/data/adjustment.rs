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

//! Price representations and the corporate action adjustment math.
//!
//! Three representations of the same instrument are never interchangeable:
//!
//! - Raw: the prices the venue printed. Immutable, and the default input to a run.
//! - Adjusted: a derived series that back-propagates corporate actions so a historical series is
//!   continuous across a split or a dividend. Produced on demand, never stored as the raw input.
//! - Trading events: the corporate actions themselves, delivered as data.
//!
//! Adjustment is opt-in. [`AdjustmentSeries`] carries the convention so both directions are exact
//! and testable: a raw price is converted to the adjusted series, and an adjusted price is
//! converted back to raw.
//!
//! The convention, applied for every action whose `effective_ns` is after the price instant:
//!
//! - A split with ratio `r` (new shares per old share) scales the price by `1 / r`.
//! - A dividend of `d` per share subtracts `d` from the price.
//!
//! So `adjusted = raw * split_factor - dividends` and `raw = (adjusted + dividends) / split_factor`,
//! where the factor and the dividend sum are cumulative over the actions that are in effect after
//! the instant. All arithmetic is decimal, and no rounding is applied here: the resulting scale is
//! the scale of the decimal operation.

use std::fmt::Display;

use anyhow::{Result, bail, ensure};
use nautilus_core::UnixNanos;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::CorporateActionType;
use crate::{data::CorporateAction, identifiers::InstrumentId, types::Price};

/// The price representation a series is expressed in.
#[derive(
    Clone,
    Copy,
    Debug,
    strum::Display,
    strum::AsRefStr,
    strum::EnumString,
    strum::EnumIter,
    strum::FromRepr,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(
        frozen,
        eq,
        eq_int,
        module = "nautilus_trader.model",
        from_py_object,
        rename_all = "SCREAMING_SNAKE_CASE",
    )
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass_enum(module = "nautilus_trader.model")
)]
pub enum PriceRepresentation {
    /// Prices as the venue printed them, with no adjustment applied.
    Raw = 1,
    /// Prices back-adjusted for the corporate actions of the instrument.
    Adjusted = 2,
}

/// A corporate action series for one instrument, in effect order.
///
/// Only splits and dividends participate in the adjustment math; a symbol change and a delisting
/// are delivered as events but do not scale a price.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.model", from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.model")
)]
pub struct AdjustmentSeries {
    instrument_id: InstrumentId,
    actions: Vec<CorporateAction>,
}

impl AdjustmentSeries {
    /// Creates a new [`AdjustmentSeries`] for one instrument.
    ///
    /// The actions are sorted by effect time, then by event time, so the series is canonical
    /// regardless of the order the data arrived in.
    ///
    /// # Errors
    ///
    /// Returns an error if an action belongs to another instrument, or a split ratio is not
    /// positive.
    pub fn new(instrument_id: InstrumentId, mut actions: Vec<CorporateAction>) -> Result<Self> {
        for action in &actions {
            ensure!(
                action.instrument_id == instrument_id,
                "action for {} does not belong to {instrument_id}",
                action.instrument_id
            );
            if action.action == CorporateActionType::Split {
                ensure!(
                    action.value > Decimal::ZERO,
                    "split on {instrument_id} at {} has a non-positive ratio {}",
                    action.effective_ns,
                    action.value
                );
            }
        }

        actions.sort_by_key(|action| (action.effective_ns, action.ts_event));

        Ok(Self {
            instrument_id,
            actions,
        })
    }

    /// Returns the instrument the series applies to.
    #[must_use]
    pub const fn instrument_id(&self) -> InstrumentId {
        self.instrument_id
    }

    /// Returns the actions, in effective order.
    #[must_use]
    pub fn actions(&self) -> &[CorporateAction] {
        &self.actions
    }

    /// Returns the cumulative split factor in effect after the given instant.
    ///
    /// The factor is the product of `1 / r` over the splits whose effect is after `ts`, so
    /// multiplying a raw price by it moves the price into the adjusted series.
    #[must_use]
    pub fn split_factor(&self, ts: UnixNanos) -> Decimal {
        self.actions
            .iter()
            .filter(|action| {
                action.action == CorporateActionType::Split
                    && action.effective_ns > ts
                    && action.value > Decimal::ZERO
            })
            .fold(Decimal::ONE, |factor, action| factor / action.value)
    }

    /// Returns the cumulative cash dividend per share in effect after the given instant.
    #[must_use]
    pub fn dividends(&self, ts: UnixNanos) -> Decimal {
        self.actions
            .iter()
            .filter(|action| {
                action.action == CorporateActionType::Dividend && action.effective_ns > ts
            })
            .fold(Decimal::ZERO, |sum, action| sum + action.value)
    }

    /// Converts a raw price at the given instant into the adjusted series.
    ///
    /// # Errors
    ///
    /// Returns an error if the adjusted value is not a valid price.
    pub fn adjust(&self, price: Price, ts: UnixNanos) -> Result<Price> {
        let adjusted = price.as_decimal() * self.split_factor(ts) - self.dividends(ts);
        Price::from_decimal(adjusted).map_err(Into::into)
    }

    /// Converts an adjusted price at the given instant back into the raw series.
    ///
    /// # Errors
    ///
    /// Returns an error if the split factor is not positive, or the raw value is not a valid price.
    pub fn unadjust(&self, price: Price, ts: UnixNanos) -> Result<Price> {
        let factor = self.split_factor(ts);
        if factor <= Decimal::ZERO {
            bail!("split factor {factor} is not positive");
        }
        let raw = (price.as_decimal() + self.dividends(ts)) / factor;
        Price::from_decimal(raw).map_err(Into::into)
    }

    /// Converts a price from one representation to the other at the given instant.
    ///
    /// A conversion from a representation to itself returns the price unchanged, so a caller can
    /// apply a target representation without branching on the source.
    ///
    /// # Errors
    ///
    /// Returns an error if the conversion is not exact for the price precision.
    pub fn convert(
        &self,
        price: Price,
        ts: UnixNanos,
        from: PriceRepresentation,
        to: PriceRepresentation,
    ) -> Result<Price> {
        match (from, to) {
            (PriceRepresentation::Raw, PriceRepresentation::Adjusted) => self.adjust(price, ts),
            (PriceRepresentation::Adjusted, PriceRepresentation::Raw) => self.unadjust(price, ts),
            _ => Ok(price),
        }
    }
}

impl Display for AdjustmentSeries {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "AdjustmentSeries({}, {} actions)",
            self.instrument_id,
            self.actions.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    const SPLIT_NS: u64 = 1_000_000_000_000_000_000;
    const DIVIDEND_NS: u64 = 2_000_000_000_000_000_000;

    fn instrument_id() -> InstrumentId {
        InstrumentId::from("AAPL.XNYS")
    }

    fn split(ratio: Decimal, effective_ns: u64) -> CorporateAction {
        CorporateAction::new(
            instrument_id(),
            CorporateActionType::Split,
            ratio,
            None,
            effective_ns.into(),
            effective_ns.into(),
            effective_ns.into(),
        )
    }

    fn dividend(amount: Decimal, effective_ns: u64) -> CorporateAction {
        CorporateAction::new(
            instrument_id(),
            CorporateActionType::Dividend,
            amount,
            None,
            effective_ns.into(),
            effective_ns.into(),
            effective_ns.into(),
        )
    }

    fn series(actions: Vec<CorporateAction>) -> AdjustmentSeries {
        AdjustmentSeries::new(instrument_id(), actions).unwrap()
    }

    fn price(value: &str) -> Price {
        Price::from(value)
    }

    #[rstest]
    fn test_four_to_one_split_adjusts_and_reverses() {
        let series = series(vec![split(Decimal::new(4, 0), SPLIT_NS)]);
        let before = UnixNanos::from(SPLIT_NS - 1);
        let after = UnixNanos::from(SPLIT_NS);

        // A 400.00 price before a 4:1 split is 100.00 in the adjusted series.
        assert_eq!(
            series.adjust(price("400.00"), before).unwrap(),
            price("100.00")
        );
        // A price at or after the split is already in the adjusted series.
        assert_eq!(
            series.adjust(price("100.00"), after).unwrap(),
            price("100.00")
        );
        // The raw price round-trips.
        assert_eq!(
            series.unadjust(price("100.00"), before).unwrap(),
            price("400.00")
        );
        assert_eq!(
            series.unadjust(price("100.00"), after).unwrap(),
            price("100.00")
        );
    }

    #[rstest]
    fn test_dividend_is_subtracted() {
        let series = series(vec![dividend(Decimal::new(250, 2), DIVIDEND_NS)]);
        let before = UnixNanos::from(DIVIDEND_NS - 1);

        assert_eq!(
            series.adjust(price("100.00"), before).unwrap(),
            price("97.50")
        );
        assert_eq!(
            series.unadjust(price("97.50"), before).unwrap(),
            price("100.00")
        );
    }

    #[rstest]
    fn test_split_and_dividend_compose() {
        let series = series(vec![
            split(Decimal::new(4, 0), SPLIT_NS),
            dividend(Decimal::new(250, 2), DIVIDEND_NS),
        ]);
        let before = UnixNanos::from(SPLIT_NS - 1);

        // 400.00 raw, 4:1 split, then a 2.50 dividend: 400/4 - 2.50 = 97.50.
        assert_eq!(
            series.adjust(price("400.00"), before).unwrap(),
            price("97.50")
        );
        assert_eq!(
            series.unadjust(price("97.50"), before).unwrap(),
            price("400.00")
        );
    }

    #[rstest]
    fn test_two_splits_compose() {
        let series = series(vec![
            split(Decimal::new(2, 0), SPLIT_NS),
            split(Decimal::new(4, 0), DIVIDEND_NS),
        ]);
        let before = UnixNanos::from(SPLIT_NS - 1);

        assert_eq!(
            series.split_factor(before),
            Decimal::new(1, 0) / Decimal::new(8, 0)
        );
        assert_eq!(
            series.adjust(price("800.00"), before).unwrap(),
            price("100")
        );
    }

    #[rstest]
    fn test_symbol_change_and_delisting_do_not_scale_a_price() {
        let symbol_change = CorporateAction::new(
            instrument_id(),
            CorporateActionType::SymbolChange,
            Decimal::ZERO,
            Some(crate::identifiers::Symbol::from("AAPL.NEW")),
            SPLIT_NS.into(),
            SPLIT_NS.into(),
            SPLIT_NS.into(),
        );
        let delisting = CorporateAction::new(
            instrument_id(),
            CorporateActionType::Delisting,
            Decimal::ZERO,
            None,
            DIVIDEND_NS.into(),
            DIVIDEND_NS.into(),
            DIVIDEND_NS.into(),
        );

        let series = series(vec![symbol_change, delisting]);
        let before = UnixNanos::from(SPLIT_NS - 1);

        assert_eq!(series.split_factor(before), Decimal::ONE);
        assert_eq!(series.dividends(before), Decimal::ZERO);
        assert_eq!(
            series.adjust(price("100.00"), before).unwrap(),
            price("100.00")
        );
    }

    #[rstest]
    fn test_convert_between_representations() {
        let series = series(vec![split(Decimal::new(4, 0), SPLIT_NS)]);
        let before = UnixNanos::from(SPLIT_NS - 1);

        assert_eq!(
            series
                .convert(
                    price("400.00"),
                    before,
                    PriceRepresentation::Raw,
                    PriceRepresentation::Adjusted
                )
                .unwrap(),
            price("100.00")
        );
        assert_eq!(
            series
                .convert(
                    price("100.00"),
                    before,
                    PriceRepresentation::Adjusted,
                    PriceRepresentation::Raw
                )
                .unwrap(),
            price("400.00")
        );
        assert_eq!(
            series
                .convert(
                    price("100.00"),
                    before,
                    PriceRepresentation::Raw,
                    PriceRepresentation::Raw
                )
                .unwrap(),
            price("100.00")
        );
    }

    #[rstest]
    fn test_actions_are_sorted_by_effect_time() {
        let series = series(vec![
            split(Decimal::new(4, 0), DIVIDEND_NS),
            split(Decimal::new(2, 0), SPLIT_NS),
        ]);

        let effect_times: Vec<UnixNanos> = series
            .actions()
            .iter()
            .map(|action| action.effective_ns)
            .collect();
        assert_eq!(
            effect_times,
            vec![UnixNanos::from(SPLIT_NS), UnixNanos::from(DIVIDEND_NS)]
        );
    }

    #[rstest]
    fn test_rejects_an_action_from_another_instrument() {
        let other = CorporateAction::new(
            InstrumentId::from("MSFT.XNYS"),
            CorporateActionType::Split,
            Decimal::new(2, 0),
            None,
            SPLIT_NS.into(),
            SPLIT_NS.into(),
            SPLIT_NS.into(),
        );

        assert!(AdjustmentSeries::new(instrument_id(), vec![other]).is_err());
    }

    #[rstest]
    fn test_rejects_a_non_positive_split_ratio() {
        assert!(
            AdjustmentSeries::new(instrument_id(), vec![split(Decimal::ZERO, SPLIT_NS)]).is_err()
        );
    }

    #[rstest]
    fn test_price_representation_round_trips_through_strings() {
        assert_eq!(
            "RAW".parse::<PriceRepresentation>().unwrap(),
            PriceRepresentation::Raw
        );
        assert_eq!(PriceRepresentation::Adjusted.to_string(), "ADJUSTED");
    }
}
