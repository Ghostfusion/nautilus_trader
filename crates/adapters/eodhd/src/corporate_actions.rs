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

//! Conversion of EODHD dividend and split rows into Nautilus corporate actions.
//!
//! A corporate action is auxiliary data rather than a price series: it reports a change to the
//! economic meaning or identity of an instrument. The engine publishes an
//! [`nautilus_model::data::CorporateAction`] on the instrument's corporate action topic, which a
//! strategy receives through `subscribe_corporate_actions` and `on_corporate_action`, so no data
//! client command carries it.

use nautilus_core::UnixNanos;
use nautilus_model::{
    data::{CorporateAction, CorporateActionType},
    identifiers::InstrumentId,
};
use rust_decimal::Decimal;

use crate::{
    bars::date_to_millis,
    http::{EodhdDividend, EodhdSplit},
};

/// Returns the corporate action for an EODHD dividend `row`.
///
/// The as-reported `unadjustedValue` is preferred over the split adjusted `value`, because this
/// adapter serves the raw `close` price: an action describes the same price series the bars
/// describe, and substituting one series for the other is the failure this preference avoids.
///
/// The ex-date is the effective time. The record itself is timestamped with `ts_event`, which
/// [`nautilus_model::data::CorporateAction`] keeps separate from `effective_ns` for exactly this
/// reason: an announced action takes effect at a date that is not when the engine observes it.
///
/// # Errors
///
/// Returns an error if the row date is unparseable, or if the row carries no usable amount.
pub fn action_from_dividend(
    instrument_id: InstrumentId,
    row: &EodhdDividend,
    ts_event: UnixNanos,
) -> anyhow::Result<CorporateAction> {
    let amount = row
        .unadjusted_value
        .as_ref()
        .or(row.value.as_ref())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Dividend for {instrument_id} on {} carries no amount",
                row.date
            )
        })?;

    Ok(CorporateAction::new(
        instrument_id,
        CorporateActionType::Dividend,
        decimal_from_json(amount)?,
        None,
        effective_ns(&row.date)?,
        ts_event,
        ts_event,
    ))
}

/// Returns the corporate action for an EODHD split `row`.
///
/// # Errors
///
/// Returns an error if the row date is unparseable, or if the ratio cannot be interpreted.
pub fn action_from_split(
    instrument_id: InstrumentId,
    row: &EodhdSplit,
    ts_event: UnixNanos,
) -> anyhow::Result<CorporateAction> {
    Ok(CorporateAction::new(
        instrument_id,
        CorporateActionType::Split,
        split_ratio(&row.split)?,
        None,
        effective_ns(&row.date)?,
        ts_event,
        ts_event,
    ))
}

/// Returns the epoch nanoseconds when an EODHD `YYYY-MM-DD` date takes effect, at midnight UTC.
fn effective_ns(date: &str) -> anyhow::Result<UnixNanos> {
    Ok(UnixNanos::from_millis(date_to_millis(date)?.unsigned_abs()))
}

/// Parses the new shares per old share from an EODHD ratio, formatted `<new>/<old>`.
fn split_ratio(value: &str) -> anyhow::Result<Decimal> {
    let invalid = || anyhow::anyhow!("Invalid EODHD split ratio '{value}': expected '<new>/<old>'");

    let (new, old) = value.split_once('/').ok_or_else(invalid)?;
    let new: Decimal = new.trim().parse().map_err(|_| invalid())?;
    let old: Decimal = old.trim().parse().map_err(|_| invalid())?;

    new.checked_div(old).ok_or_else(invalid)
}

/// Parses a JSON amount as an exact decimal.
///
/// EODHD publishes the amount as a JSON number. Its written form is parsed rather than its floating
/// point value, because a dividend of `0.00054` is not representable in binary and the difference
/// compounds once an adjustment stage consumes the action.
fn decimal_from_json(value: &serde_json::Value) -> anyhow::Result<Decimal> {
    let text = match value {
        serde_json::Value::String(text) => text.clone(),
        other => other.to_string(),
    };

    text.trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid EODHD decimal amount '{text}'"))
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;
    use crate::http::EodhdSplit;

    fn instrument_id() -> InstrumentId {
        InstrumentId::from("AAPL.US")
    }

    fn dividend(
        unadjusted: Option<serde_json::Value>,
        value: Option<serde_json::Value>,
    ) -> EodhdDividend {
        EodhdDividend {
            date: "2024-02-09".to_string(),
            declaration_date: Some("2024-02-01".to_string()),
            record_date: Some("2024-02-12".to_string()),
            payment_date: Some("2024-02-15".to_string()),
            period: None,
            value,
            unadjusted_value: unadjusted,
            currency: Some("USD".to_string()),
        }
    }

    #[rstest]
    fn test_dividend_prefers_the_as_reported_amount() {
        let row = dividend(Some(serde_json::json!(0.24)), Some(serde_json::json!(0.06)));

        let action = action_from_dividend(instrument_id(), &row, UnixNanos::from(7)).unwrap();

        assert_eq!(action.action, CorporateActionType::Dividend);
        assert_eq!(action.value.to_string(), "0.24");
        assert_eq!(action.instrument_id, instrument_id());
        assert_eq!(action.new_symbol, None);
        assert_eq!(action.ts_event, UnixNanos::from(7));
        assert_eq!(action.ts_init, UnixNanos::from(7));
    }

    #[rstest]
    fn test_dividend_falls_back_to_the_adjusted_amount() {
        let row = dividend(None, Some(serde_json::json!(0.25)));

        let action = action_from_dividend(instrument_id(), &row, UnixNanos::from(7)).unwrap();

        assert_eq!(action.value.to_string(), "0.25");
    }

    #[rstest]
    fn test_dividend_keeps_an_exact_amount() {
        let row = dividend(Some(serde_json::json!(0.00054)), None);

        let action = action_from_dividend(instrument_id(), &row, UnixNanos::from(7)).unwrap();

        assert_eq!(action.value.to_string(), "0.00054");
    }

    #[rstest]
    fn test_dividend_parses_an_amount_supplied_as_a_string() {
        let row = dividend(Some(serde_json::json!("1.23456789")), None);

        let action = action_from_dividend(instrument_id(), &row, UnixNanos::from(7)).unwrap();

        assert_eq!(action.value.to_string(), "1.23456789");
    }

    #[rstest]
    fn test_dividend_uses_the_ex_date_as_the_effective_time() {
        let row = dividend(Some(serde_json::json!(0.24)), None);

        let action = action_from_dividend(instrument_id(), &row, UnixNanos::from(7)).unwrap();

        assert_eq!(
            action.effective_ns,
            UnixNanos::from(1_707_436_800_000_000_000)
        );
    }

    #[rstest]
    fn test_dividend_rejects_a_row_without_an_amount() {
        let row = dividend(None, None);

        assert!(action_from_dividend(instrument_id(), &row, UnixNanos::from(7)).is_err());
    }

    #[rstest]
    fn test_dividend_rejects_an_unparseable_date() {
        let mut row = dividend(Some(serde_json::json!(0.24)), None);
        row.date = "not-a-date".to_string();

        assert!(action_from_dividend(instrument_id(), &row, UnixNanos::from(7)).is_err());
    }

    fn split(value: &str) -> EodhdSplit {
        EodhdSplit {
            date: "2020-08-31".to_string(),
            split: value.to_string(),
        }
    }

    #[rstest]
    #[case("4.000000/1.000000", "4")]
    #[case("2/1", "2")]
    #[case("1.000000/10.000000", "0.1")]
    fn test_split_ratio_is_the_new_shares_per_old_share(
        #[case] value: &str,
        #[case] expected: &str,
    ) {
        let action = action_from_split(instrument_id(), &split(value), UnixNanos::from(7)).unwrap();

        assert_eq!(action.action, CorporateActionType::Split);
        // Compared as decimals: a division is free to carry trailing zeros.
        assert_eq!(action.value, expected.parse::<Decimal>().unwrap());
        assert_eq!(action.new_symbol, None);
    }

    #[rstest]
    #[case("4.000000")]
    #[case("4/a")]
    #[case("4/0")]
    #[case("")]
    fn test_split_rejects_a_ratio_it_cannot_interpret(#[case] value: &str) {
        assert!(action_from_split(instrument_id(), &split(value), UnixNanos::from(7)).is_err());
    }

    #[rstest]
    fn test_split_uses_the_split_date_as_the_effective_time() {
        let action = action_from_split(instrument_id(), &split("4/1"), UnixNanos::from(7)).unwrap();

        assert_eq!(
            action.effective_ns,
            UnixNanos::from(1_598_832_000_000_000_000)
        );
    }
}
