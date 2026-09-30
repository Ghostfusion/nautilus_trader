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

//! A `CorporateAction` data type representing a corporate action on an instrument.
//!
//! A corporate action is auxiliary data, not a price series. It is the raw input an adjustment
//! stage consumes to derive an adjusted series, and it is delivered as data so a strategy can
//! react to it. One type carries every action kind, discriminated by [`CorporateActionType`], so
//! the catalog, the bus, and the Python surface have one path rather than four.
//!
//! | Action | `value` | `new_symbol` |
//! | ------------------ | ------------------------------------ | ------------------ |
//! | `Split` | The number of new shares per old share | Absent |
//! | `Dividend` | The cash amount per share | Absent |
//! | `SymbolChange` | Zero | The new venue symbol |
//! | `Delisting` | Zero | Absent |
//!
//! `value` is a decimal, so a split ratio and a dividend amount keep exact arithmetic.

use std::{collections::HashMap, fmt::Display};

use nautilus_core::{UnixNanos, serialization::Serializable};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::HasTsInit;
use crate::identifiers::{InstrumentId, Symbol};

/// The kind of corporate action.
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
pub enum CorporateActionType {
    /// A share split or consolidation.
    Split = 1,
    /// A cash dividend.
    Dividend = 2,
    /// A change of the venue symbol.
    SymbolChange = 3,
    /// The instrument stops trading.
    Delisting = 4,
}

/// Represents a corporate action on an instrument.
///
/// `effective_ns` is when the action takes effect at the venue, for example the ex-date of a
/// dividend. `ts_event` and `ts_init` describe the action record itself.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type")]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.model", from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.model")
)]
pub struct CorporateAction {
    /// The instrument ID the action applies to.
    pub instrument_id: InstrumentId,
    /// The kind of corporate action.
    pub action: CorporateActionType,
    /// The action value: the new shares per old share for a split, the cash amount per share for a
    /// dividend, and zero for a symbol change or a delisting.
    pub value: Decimal,
    /// The new venue symbol, for a symbol change.
    pub new_symbol: Option<Symbol>,
    /// UNIX timestamp (nanoseconds) when the action takes effect at the venue.
    pub effective_ns: UnixNanos,
    /// UNIX timestamp (nanoseconds) when the action event occurred.
    pub ts_event: UnixNanos,
    /// UNIX timestamp (nanoseconds) when the instance was created.
    pub ts_init: UnixNanos,
}

impl CorporateAction {
    /// Creates a new [`CorporateAction`] instance.
    #[must_use]
    pub fn new(
        instrument_id: InstrumentId,
        action: CorporateActionType,
        value: Decimal,
        new_symbol: Option<Symbol>,
        effective_ns: UnixNanos,
        ts_event: UnixNanos,
        ts_init: UnixNanos,
    ) -> Self {
        Self {
            instrument_id,
            action,
            value,
            new_symbol,
            effective_ns,
            ts_event,
            ts_init,
        }
    }

    /// Returns the metadata for the type, for use with serialization formats.
    #[must_use]
    pub fn get_metadata(instrument_id: &InstrumentId) -> HashMap<String, String> {
        let mut metadata = HashMap::new();
        metadata.insert("instrument_id".to_string(), instrument_id.to_string());
        metadata
    }
}

impl Display for CorporateAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{},{},{},{}",
            self.instrument_id, self.action, self.value, self.effective_ns
        )
    }
}

impl Serializable for CorporateAction {}

impl HasTsInit for CorporateAction {
    fn ts_init(&self) -> UnixNanos {
        self.ts_init
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
    fn test_split_carries_the_ratio() {
        let action = CorporateAction::new(
            instrument_id(),
            CorporateActionType::Split,
            Decimal::new(4, 0),
            None,
            1.into(),
            2.into(),
            3.into(),
        );

        assert_eq!(action.action, CorporateActionType::Split);
        assert_eq!(action.value, Decimal::new(4, 0));
        assert_eq!(action.new_symbol, None);
        assert_eq!(action.effective_ns, UnixNanos::from(1));
    }

    #[rstest]
    fn test_dividend_keeps_an_exact_amount() {
        let action = CorporateAction::new(
            instrument_id(),
            CorporateActionType::Dividend,
            "0.12345678".parse().unwrap(),
            None,
            1.into(),
            2.into(),
            3.into(),
        );

        assert_eq!(action.value.to_string(), "0.12345678");
    }

    #[rstest]
    fn test_symbol_change_carries_the_new_symbol() {
        let action = CorporateAction::new(
            instrument_id(),
            CorporateActionType::SymbolChange,
            Decimal::ZERO,
            Some(Symbol::from("AAPL.NEW")),
            1.into(),
            2.into(),
            3.into(),
        );

        assert_eq!(action.new_symbol, Some(Symbol::from("AAPL.NEW")));
    }

    #[rstest]
    fn test_has_ts_init() {
        let action = CorporateAction::new(
            instrument_id(),
            CorporateActionType::Delisting,
            Decimal::ZERO,
            None,
            1.into(),
            2.into(),
            3.into(),
        );

        assert_eq!(action.ts_init(), UnixNanos::from(3));
    }

    #[rstest]
    fn test_display_names_the_action() {
        let action = CorporateAction::new(
            instrument_id(),
            CorporateActionType::Delisting,
            Decimal::ZERO,
            None,
            1.into(),
            2.into(),
            3.into(),
        );

        assert_eq!(action.to_string(), "AAPL.XNYS,DELISTING,0,1");
    }

    #[rstest]
    fn test_metadata_names_the_instrument() {
        let metadata = CorporateAction::get_metadata(&instrument_id());

        assert_eq!(metadata.get("instrument_id").unwrap(), "AAPL.XNYS");
    }

    #[rstest]
    fn test_serde_round_trip() {
        let action = CorporateAction::new(
            instrument_id(),
            CorporateActionType::Split,
            Decimal::new(4, 0),
            None,
            1.into(),
            2.into(),
            3.into(),
        );

        let encoded = serde_json::to_string(&action).unwrap();
        let decoded: CorporateAction = serde_json::from_str(&encoded).unwrap();

        assert_eq!(decoded, action);
    }
}
