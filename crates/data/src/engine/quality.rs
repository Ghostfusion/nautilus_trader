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
//! The same vocabulary carries the feed-identity checks an adapter runs over its own feed, where
//! the engine sees nothing to check: whether a venue's reported change in open interest is
//! explicable by the volume traded over the same interval, and whether a reported settlement total
//! reconciles with the sum of its components.
//!
//! [`DataEngineConfig::data_quality_action`]: crate::engine::config::DataEngineConfig::data_quality_action

use std::{
    cmp::Ordering,
    collections::BTreeMap,
    fmt::{Display, Formatter},
};

use nautilus_model::{
    data::{QuoteTick, TradeTick},
    enums::AggressorSide,
    types::{Money, Price, Quantity},
};
use rust_decimal::Decimal;
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
    /// The reported change in open interest exceeded the volume traded over the same interval.
    OpenInterestChangeExceedsVolume,
    /// A reported settlement total did not reconcile with the sum of its components.
    SettlementTotalMismatch,
    /// The reported aggressor side disagreed with the tick-rule inference from the previous price.
    AggressorSignDisagreement,
}

impl DataQualityViolation {
    /// Every violation kind, used to render a complete and stable count line.
    pub const ALL: [Self; 6] = [
        Self::CrossedQuote,
        Self::NonPositiveValue,
        Self::OutOfOrderTimestamp,
        Self::OpenInterestChangeExceedsVolume,
        Self::SettlementTotalMismatch,
        Self::AggressorSignDisagreement,
    ];
}

impl Display for DataQualityViolation {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CrossedQuote => f.write_str("crossed_quote"),
            Self::NonPositiveValue => f.write_str("non_positive_value"),
            Self::OutOfOrderTimestamp => f.write_str("out_of_order_timestamp"),
            Self::OpenInterestChangeExceedsVolume => {
                f.write_str("open_interest_change_exceeds_volume")
            }
            Self::SettlementTotalMismatch => f.write_str("settlement_total_mismatch"),
            Self::AggressorSignDisagreement => f.write_str("aggressor_sign_disagreement"),
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
    aggressor_comparisons: usize,
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

    /// Returns the number of trades whose reported aggressor side was compared with the tick-rule
    /// inference.
    ///
    /// A trade reported as `NoAggressor`, or one whose previous trade price is unknown or equal to
    /// its own, is not compared: it never enters this denominator, so an agreement rate computed
    /// from it is not diluted by trades that carry no aggressor signal.
    #[must_use]
    pub fn aggressor_comparisons(&self) -> usize {
        self.aggressor_comparisons
    }

    /// Returns the observed aggressor-agreement rate, or `None` when nothing was compared.
    ///
    /// The rate is the fraction of compared trades whose reported aggressor side matched the
    /// tick-rule inference, `(comparisons - disagreements) / comparisons`. It is `None`, never
    /// zero, when `comparisons` is zero: a rate that was never measured is not a measured zero,
    /// and a direction-dependent metric must not be refused against an absent measurement.
    #[must_use]
    pub fn aggressor_agreement_rate(&self) -> Option<f64> {
        if self.aggressor_comparisons == 0 {
            return None;
        }

        let disagreements = self.count(DataQualityViolation::AggressorSignDisagreement);
        let agreements = self.aggressor_comparisons.saturating_sub(disagreements);

        Some(agreements as f64 / self.aggressor_comparisons as f64)
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

    /// Records one violation of `violation`.
    ///
    /// The engine records what its gate detects; an adapter records what its own feed-identity
    /// checks detect, so both reach the same totals.
    pub fn record(&mut self, violation: DataQualityViolation) {
        *self.counts.entry(violation).or_insert(0) += 1;
    }

    /// Records one comparison of a reported aggressor side against the tick-rule inference.
    ///
    /// Recorded only when a trade reports a buy or sell aggressor and a previous trade price is
    /// available, so the agreement rate is `comparisons - disagreements` over this count.
    pub fn record_aggressor_comparison(&mut self) {
        self.aggressor_comparisons += 1;
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

/// Infers a trade's aggressor side from the tick rule.
///
/// A trade priced above the previous trade price for the same instrument is inferred a buy, one
/// priced below a sell, and one at the same price (or the first trade, with no previous price)
/// yields no inference. The inference is `None` rather than a guess, so a caller cannot mistake a
/// missing signal for a reported one.
#[must_use]
pub fn infer_aggressor_side(price: Price, previous_price: Option<Price>) -> Option<AggressorSide> {
    let previous_price = previous_price?;

    match price.cmp(&previous_price) {
        Ordering::Greater => Some(AggressorSide::Buy),
        Ordering::Less => Some(AggressorSide::Sell),
        Ordering::Equal => None,
    }
}

/// Validates a venue's reported change in open interest against the volume traded over the same
/// interval.
///
/// Open interest is a stock that moves only through trades, and one traded unit opens or closes at
/// most one unit, so the absolute change in open interest across an interval cannot exceed the
/// volume traded in it, when both are in the same units and cover the same interval. A larger
/// change means the two numbers do not describe the same instrument and the same interval.
///
/// The engine never sees open interest, so this is a pure check for an adapter to run over its own
/// feed. The verdict is drawn from the gate's vocabulary, so the adapter records it with
/// [`DataQualityCounts::record`] and it reaches the same totals.
#[must_use]
pub fn validate_open_interest_change(
    open_interest_prev: Quantity,
    open_interest_now: Quantity,
    volume: Quantity,
) -> Option<DataQualityViolation> {
    let change = (open_interest_now.as_decimal() - open_interest_prev.as_decimal()).abs();

    if change > volume.as_decimal() {
        return Some(DataQualityViolation::OpenInterestChangeExceedsVolume);
    }

    None
}

/// Validates that a reported settlement total reconciles with the sum of its components.
///
/// Every component and the tolerance must be in the reported total's currency, and the absolute
/// difference between the sum of the components and the reported total must not exceed `tolerance`.
/// A component in another currency is itself a mismatch: the two numbers are not the same quantity.
#[must_use]
pub fn validate_settlement_total(
    reported: Money,
    components: &[Money],
    tolerance: Money,
) -> Option<DataQualityViolation> {
    if components
        .iter()
        .any(|component| component.currency != reported.currency)
        || tolerance.currency != reported.currency
    {
        return Some(DataQualityViolation::SettlementTotalMismatch);
    }

    let sum: Decimal = components.iter().map(Money::as_decimal).sum();
    let difference = (sum - reported.as_decimal()).abs();

    if difference > tolerance.as_decimal() {
        return Some(DataQualityViolation::SettlementTotalMismatch);
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
        types::{Money, Price, Quantity},
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
    fn test_infer_aggressor_side_from_a_rise_and_a_fall() {
        assert_eq!(
            infer_aggressor_side(Price::from("101.00"), Some(Price::from("100.00"))),
            Some(AggressorSide::Buy)
        );
        assert_eq!(
            infer_aggressor_side(Price::from("99.00"), Some(Price::from("100.00"))),
            Some(AggressorSide::Sell)
        );
    }

    #[rstest::rstest]
    fn test_infer_aggressor_side_yields_no_inference() {
        assert_eq!(
            infer_aggressor_side(Price::from("100.00"), Some(Price::from("100.00"))),
            None
        );
        assert_eq!(infer_aggressor_side(Price::from("100.00"), None), None);
    }

    #[rstest::rstest]
    fn test_aggressor_comparisons_are_counted_beside_the_counts() {
        let mut counts = DataQualityCounts::default();
        counts.record_aggressor_comparison();
        counts.record_aggressor_comparison();
        counts.record(DataQualityViolation::AggressorSignDisagreement);

        assert_eq!(counts.aggressor_comparisons(), 2);
        assert_eq!(
            counts.count(DataQualityViolation::AggressorSignDisagreement),
            1
        );
        // The comparison count is not part of the accept/reject totals.
        assert_eq!(counts.rejected(), 1);
        assert_eq!(counts.total(), 1);
    }

    #[rstest::rstest]
    fn test_aggressor_agreement_rate_is_absent_without_comparisons() {
        let counts = DataQualityCounts::default();

        // Nothing was compared, so there is no rate: absent rather than a zero that would read
        // as a measured value and breach every floor.
        assert_eq!(counts.aggressor_agreement_rate(), None);
    }

    #[rstest::rstest]
    fn test_aggressor_agreement_rate_is_the_agreeing_fraction() {
        let mut counts = DataQualityCounts::default();
        for _ in 0..5 {
            counts.record_aggressor_comparison();
        }
        counts.record(DataQualityViolation::AggressorSignDisagreement);
        counts.record(DataQualityViolation::AggressorSignDisagreement);

        assert_eq!(counts.aggressor_agreement_rate(), Some(3.0 / 5.0));
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
        let expected = concat!(
            "data quality: total=4 accepted=1 rejected=3 crossed_quote=1 non_positive_value=2 ",
            "out_of_order_timestamp=0 open_interest_change_exceeds_volume=0 ",
            "settlement_total_mismatch=0 aggressor_sign_disagreement=0"
        );
        assert_eq!(line, expected);
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
        assert_eq!(
            DataQualityViolation::OpenInterestChangeExceedsVolume.to_string(),
            "open_interest_change_exceeds_volume"
        );
        assert_eq!(
            DataQualityViolation::SettlementTotalMismatch.to_string(),
            "settlement_total_mismatch"
        );
        assert_eq!(
            DataQualityViolation::AggressorSignDisagreement.to_string(),
            "aggressor_sign_disagreement"
        );
    }

    #[rstest::rstest]
    #[case("100", "101", "5")]
    #[case("100", "95", "5")]
    #[case("100", "105", "5")]
    fn test_open_interest_change_is_accepted_when_explicable_by_volume(
        #[case] prev: &str,
        #[case] now: &str,
        #[case] volume: &str,
    ) {
        assert_eq!(
            validate_open_interest_change(
                Quantity::from(prev),
                Quantity::from(now),
                Quantity::from(volume)
            ),
            None
        );
    }

    #[rstest::rstest]
    fn test_open_interest_change_is_detected_when_it_exceeds_volume() {
        assert_eq!(
            validate_open_interest_change(
                Quantity::from("100"),
                Quantity::from("110"),
                Quantity::from("5")
            ),
            Some(DataQualityViolation::OpenInterestChangeExceedsVolume)
        );
    }

    #[rstest::rstest]
    fn test_settlement_total_is_accepted_when_the_components_reconcile() {
        assert_eq!(
            validate_settlement_total(
                Money::from("100.00 USD"),
                &[Money::from("60.00 USD"), Money::from("40.00 USD")],
                Money::from("0.01 USD")
            ),
            None
        );
    }

    #[rstest::rstest]
    fn test_settlement_total_is_accepted_at_the_tolerance_boundary() {
        assert_eq!(
            validate_settlement_total(
                Money::from("100.00 USD"),
                &[Money::from("60.00 USD"), Money::from("39.99 USD")],
                Money::from("0.01 USD")
            ),
            None
        );
    }

    #[rstest::rstest]
    fn test_settlement_total_is_detected_when_it_does_not_reconcile() {
        assert_eq!(
            validate_settlement_total(
                Money::from("100.00 USD"),
                &[Money::from("60.00 USD"), Money::from("39.00 USD")],
                Money::from("0.01 USD")
            ),
            Some(DataQualityViolation::SettlementTotalMismatch)
        );
    }

    #[rstest::rstest]
    fn test_settlement_total_is_detected_when_a_component_is_in_another_currency() {
        assert_eq!(
            validate_settlement_total(
                Money::from("100.00 USD"),
                &[Money::from("60.00 USD"), Money::from("40.00 EUR")],
                Money::from("0.01 USD")
            ),
            Some(DataQualityViolation::SettlementTotalMismatch)
        );
    }

    #[rstest::rstest]
    fn test_feed_identity_violations_are_counted_through_the_public_entry() {
        let mut counts = DataQualityCounts::default();
        counts.record(DataQualityViolation::OpenInterestChangeExceedsVolume);
        counts.record(DataQualityViolation::SettlementTotalMismatch);

        assert_eq!(
            counts.count(DataQualityViolation::OpenInterestChangeExceedsVolume),
            1
        );
        assert_eq!(
            counts.count(DataQualityViolation::SettlementTotalMismatch),
            1
        );
        assert_eq!(counts.rejected(), 2);
        assert_eq!(counts.total(), 2);
    }
}
