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

//! Data-quality gate for quotes and trades.
//!
//! The gate classifies an incoming quote or trade against a closed set of violations and either
//! flags or drops a violating record. Everything it does is counted, so a run reports a number
//! rather than a log line. The gate is off unless [`DataEngineConfig::data_quality_action`]
//! requests it, in which case its effect on the replay path is byte-for-byte the previous
//! behaviour.
//!
//! [`DataEngineConfig::data_quality_action`]: crate::engine::config::DataEngineConfig::data_quality_action

use std::{
    collections::BTreeMap,
    fmt::{Display, Formatter},
};

use nautilus_model::data::{QuoteTick, TradeTick};
use serde::{Deserialize, Serialize};

/// A named reason the data-quality gate rejected or flagged a record.
///
/// The enumeration is closed so an unexpected violation cannot be recorded in an unnamed bucket.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(
        frozen,
        eq,
        module = "nautilus_trader.data",
        from_py_object,
        rename_all = "SCREAMING_SNAKE_CASE",
    )
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass_enum(module = "nautilus_trader.data")
)]
pub enum DataQualityViolation {
    /// The quote's bid price exceeded its ask price.
    CrossedQuote,
    /// A price or size on the record was zero or negative.
    NonPositiveValue,
    /// The record's `ts_event` preceded the last `ts_event` seen for its instrument.
    OutOfOrderTimestamp,
}

impl DataQualityViolation {
    /// Every violation kind, used to render a complete and stable count line.
    pub const ALL: [Self; 3] = [
        Self::CrossedQuote,
        Self::NonPositiveValue,
        Self::OutOfOrderTimestamp,
    ];
}

impl Display for DataQualityViolation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CrossedQuote => f.write_str("crossed_quote"),
            Self::NonPositiveValue => f.write_str("non_positive_value"),
            Self::OutOfOrderTimestamp => f.write_str("out_of_order_timestamp"),
        }
    }
}

/// The action the gate takes on a record that violates a check.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(
        frozen,
        eq,
        module = "nautilus_trader.data",
        from_py_object,
        rename_all = "SCREAMING_SNAKE_CASE",
    )
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass_enum(module = "nautilus_trader.data")
)]
pub enum DataQualityAction {
    /// Forward the record (cache and publish it) and count the violation.
    Flag,
    /// Refuse the record (neither cache nor publish it) and count the violation.
    Drop,
}

impl Display for DataQualityAction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Flag => f.write_str("flag"),
            Self::Drop => f.write_str("drop"),
        }
    }
}

/// Per-kind violation counts for the gate, plus the accepted total.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DataQualityCounts {
    counts: BTreeMap<DataQualityViolation, usize>,
    accepted: usize,
}

impl DataQualityCounts {
    /// Returns the number of records violating `violation`.
    #[must_use]
    pub fn count(&self, violation: DataQualityViolation) -> usize {
        self.counts.get(&violation).copied().unwrap_or(0)
    }

    /// Returns the number of records accepted by the gate.
    #[must_use]
    pub fn accepted(&self) -> usize {
        self.accepted
    }

    /// Returns the total number of records that violated a check.
    #[must_use]
    pub fn rejected(&self) -> usize {
        self.counts.values().sum()
    }

    /// Returns the total number of records seen (accepted and violating).
    #[must_use]
    pub fn total(&self) -> usize {
        self.accepted + self.rejected()
    }

    /// Iterates over the non-zero violation counts by kind.
    pub fn iter(&self) -> impl Iterator<Item = (DataQualityViolation, usize)> + '_ {
        self.counts.iter().map(|(k, v)| (*k, *v))
    }

    pub(crate) fn record(&mut self, violation: DataQualityViolation) {
        *self.counts.entry(violation).or_insert(0) += 1;
    }

    pub(crate) fn accept(&mut self) {
        self.accepted += 1;
    }
}

impl Display for DataQualityCounts {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "data quality: total={} accepted={} rejected={}",
            self.total(),
            self.accepted(),
            self.rejected()
        )?;

        for violation in DataQualityViolation::ALL {
            write!(f, " {violation}={}", self.count(violation))?;
        }

        Ok(())
    }
}

/// Classifies a quote against the value checks (non-positive price or size; a crossed quote).
///
/// The timestamp check is applied by the engine against its own last-seen `ts_event`, because the
/// replay pipeline does not always write the cache and a cache-based check would skip records.
#[must_use]
pub fn classify_quote(quote: &QuoteTick) -> Option<DataQualityViolation> {
    if quote.bid_price.as_f64() <= 0.0
        || quote.ask_price.as_f64() <= 0.0
        || quote.bid_size.as_f64() <= 0.0
        || quote.ask_size.as_f64() <= 0.0
    {
        return Some(DataQualityViolation::NonPositiveValue);
    }

    if quote.bid_price > quote.ask_price {
        return Some(DataQualityViolation::CrossedQuote);
    }

    None
}

/// Classifies a trade against the value checks (a non-positive price or size).
#[must_use]
pub fn classify_trade(trade: &TradeTick) -> Option<DataQualityViolation> {
    if trade.price.as_f64() <= 0.0 || trade.size.as_f64() <= 0.0 {
        return Some(DataQualityViolation::NonPositiveValue);
    }

    None
}

#[cfg(test)]
mod tests {
    use nautilus_core::UnixNanos;
    use nautilus_model::{
        data::{QuoteTick, TradeTick},
        enums::AggressorSide,
        identifiers::{InstrumentId, TradeId},
        types::{Price, Quantity},
    };

    use super::*;

    fn instrument_id() -> InstrumentId {
        InstrumentId::from("ETHUSDT-PERP.BINANCE")
    }

    // Constructed field-wise so the checks can be exercised on values the checked constructors
    // would reject (a non-positive size panics in `TradeTick::new`).
    fn quote(bid: &str, ask: &str, bid_size: &str, ask_size: &str) -> QuoteTick {
        QuoteTick {
            instrument_id: instrument_id(),
            bid_price: Price::from(bid),
            ask_price: Price::from(ask),
            bid_size: Quantity::from(bid_size),
            ask_size: Quantity::from(ask_size),
            ts_event: UnixNanos::from(1),
            ts_init: UnixNanos::from(1),
        }
    }

    fn trade(price: &str, size: &str) -> TradeTick {
        TradeTick {
            instrument_id: instrument_id(),
            price: Price::from(price),
            size: Quantity::from(size),
            aggressor_side: AggressorSide::NoAggressor,
            trade_id: TradeId::from("T-001"),
            ts_event: UnixNanos::from(1),
            ts_init: UnixNanos::from(1),
        }
    }

    #[rstest::rstest]
    fn test_classify_quote_detects_crossed_quote() {
        assert_eq!(
            classify_quote(&quote("101.00", "100.00", "1.00", "1.00")),
            Some(DataQualityViolation::CrossedQuote)
        );
    }

    #[rstest::rstest]
    #[case("0.00", "100.00", "1.00", "1.00")]
    #[case("100.00", "0.00", "1.00", "1.00")]
    #[case("-1.00", "100.00", "1.00", "1.00")]
    #[case("100.00", "101.00", "0.00", "1.00")]
    fn test_classify_quote_detects_non_positive_value(
        #[case] bid: &str,
        #[case] ask: &str,
        #[case] bid_size: &str,
        #[case] ask_size: &str,
    ) {
        assert_eq!(
            classify_quote(&quote(bid, ask, bid_size, ask_size)),
            Some(DataQualityViolation::NonPositiveValue)
        );
    }

    #[rstest::rstest]
    fn test_classify_quote_accepts_clean_quote() {
        assert_eq!(
            classify_quote(&quote("99.00", "101.00", "1.00", "1.00")),
            None
        );
    }

    #[rstest::rstest]
    #[case("0.00", "1.00")]
    #[case("-1.00", "1.00")]
    #[case("100.00", "0.00")]
    fn test_classify_trade_detects_non_positive_value(#[case] price: &str, #[case] size: &str) {
        assert_eq!(
            classify_trade(&trade(price, size)),
            Some(DataQualityViolation::NonPositiveValue)
        );
    }

    #[rstest::rstest]
    fn test_classify_trade_accepts_clean_trade() {
        assert_eq!(classify_trade(&trade("100.00", "1.00")), None);
    }

    #[rstest::rstest]
    fn test_counts_accumulate_per_kind() {
        let mut counts = DataQualityCounts::default();
        counts.record(DataQualityViolation::CrossedQuote);
        counts.record(DataQualityViolation::CrossedQuote);
        counts.record(DataQualityViolation::OutOfOrderTimestamp);
        counts.accept();
        counts.accept();
        counts.accept();

        assert_eq!(counts.count(DataQualityViolation::CrossedQuote), 2);
        assert_eq!(counts.count(DataQualityViolation::NonPositiveValue), 0);
        assert_eq!(counts.count(DataQualityViolation::OutOfOrderTimestamp), 1);
        assert_eq!(counts.rejected(), 3);
        assert_eq!(counts.accepted(), 3);
        assert_eq!(counts.total(), 6);

        let iterated: Vec<_> = counts.iter().collect();
        assert_eq!(
            iterated,
            vec![
                (DataQualityViolation::CrossedQuote, 2),
                (DataQualityViolation::OutOfOrderTimestamp, 1),
            ]
        );
    }

    #[rstest::rstest]
    fn test_counts_display_names_total_and_kinds() {
        let mut counts = DataQualityCounts::default();
        counts.record(DataQualityViolation::CrossedQuote);
        counts.record(DataQualityViolation::NonPositiveValue);
        counts.record(DataQualityViolation::NonPositiveValue);
        counts.accept();

        let line = counts.to_string();
        assert_eq!(
            line,
            "data quality: total=4 accepted=1 rejected=3 crossed_quote=1 non_positive_value=2 out_of_order_timestamp=0"
        );
    }

    #[rstest::rstest]
    fn test_violation_display_is_lowercase() {
        assert_eq!(
            DataQualityViolation::CrossedQuote.to_string(),
            "crossed_quote"
        );
        assert_eq!(
            DataQualityViolation::NonPositiveValue.to_string(),
            "non_positive_value"
        );
        assert_eq!(
            DataQualityViolation::OutOfOrderTimestamp.to_string(),
            "out_of_order_timestamp"
        );
    }
}
