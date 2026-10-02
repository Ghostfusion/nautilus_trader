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

//! Conversion of the gateway's dividend and stock split records to the domain model.
//!
//! A corporate action is auxiliary data rather than a price series: it reports a change to the
//! economic meaning of an instrument, and an adjustment stage consumes it to derive an adjusted
//! series. An action describes the same series the bars describe, so the date it takes effect is
//! placed on the same instant a daily bar of that date carries.
//!
//! Three things about these feeds decide the mapping, and none of them is a choice this adapter
//! makes.
//!
//! The dividend feed has no amount field. The only field describing the payment is a sentence the
//! venue writes in English with the amount inside it, so the amount is read out of that sentence. A
//! sentence that does not state a cash amount is refused rather than valued: the same field carries
//! distributions in specie, and an adjustment stage handed a share count where it expects cash
//! would subtract the wrong quantity from a price.
//!
//! The date fields are month first, day second, and year last, which the protocol comment states
//! the other way round. The shipped client passes the field through unmodified, so the recorded
//! values are the wire values, and they include dates whose middle component cannot be a month.
//!
//! The split ratio is two share counts separated by a right arrow, the count held before and the
//! count held after. The domain type wants the new count per old one, which is the second divided
//! by the first.

use anyhow::{Context, bail};
use nautilus_core::UnixNanos;
use nautilus_model::{
    data::{CorporateAction, CorporateActionType},
    identifiers::InstrumentId,
};
use rust_decimal::Decimal;

use crate::{
    common::Market,
    generated::{
        qot_get_corporate_actions_dividends::DividendItem,
        qot_get_corporate_actions_stock_splits::StockSplitItem,
    },
    mappers::bars,
};

/// The opening words of the sentence the venue states a cash dividend in.
const CASH_DIVIDEND_PREFIX: &str = "Cash Dividend: ";

/// The closing words of the sentence the venue states a cash dividend in.
const PER_SHARE_SUFFIX: &str = " Per Share";

/// Returns the corporate action for a gateway dividend record.
///
/// The effective time is the ex-date, which is the date the price series changes and therefore the
/// date an adjustment has to apply from. The record itself is timestamped with `ts_event`, which the
/// domain type keeps separate from `effective_ns` for exactly this reason: an action takes effect at
/// a date that is not when the engine observes it.
///
/// # Errors
///
/// Returns an error if the record carries no statement or no ex-date, if the date is unreadable, if
/// the statement is not a cash amount per share, or if the amount is stated in a currency the market
/// does not quote in.
pub fn action_from_dividend(
    instrument_id: InstrumentId,
    row: &DividendItem,
    market: Market,
    ts_event: UnixNanos,
) -> anyhow::Result<CorporateAction> {
    let statement = row
        .statement
        .as_deref()
        .with_context(|| format!("the dividend for {instrument_id} carries no statement"))?;
    let effective = row.ex_date.as_deref().with_context(|| {
        format!("the dividend '{statement}' for {instrument_id} carries no ex-date")
    })?;

    Ok(CorporateAction::new(
        instrument_id,
        CorporateActionType::Dividend,
        cash_amount(statement, market)?,
        None,
        effective_ns(effective)?,
        ts_event,
        ts_event,
    ))
}

/// Returns the corporate action for a gateway stock split record.
///
/// The effective time is the ex-date when the venue states one and the announcement date when it
/// does not. That is not a fallback for missing data but the venue's own shape: its ex-date is a
/// Hong Kong field, and the United States records carry only the announcement date, which for the
/// splits recorded here is the date the split took effect.
///
/// The calendar date is read from the record's text field rather than from the epoch seconds beside
/// it. The seconds are the market's own midnight, which is four or five hours from UTC depending on
/// the time of year, and the daily bars carry UTC midnight; placing an action on the venue's instant
/// would put it hours away from the bar it adjusts, and across a market offset it would put the two
/// on different dates. The text field states the same calendar date the seconds stand for, so it is
/// the one that lands where the bars are.
///
/// # Errors
///
/// Returns an error if the record carries neither date, if the ratio is not two share counts, or if
/// the date cannot be placed.
pub fn action_from_split(
    instrument_id: InstrumentId,
    row: &StockSplitItem,
    ts_event: UnixNanos,
) -> anyhow::Result<CorporateAction> {
    let date = row
        .ex_date_str
        .as_deref()
        .or(row.dir_deci_pub_date_str.as_deref())
        .with_context(|| {
            format!("the stock split for {instrument_id} carries no date to take effect from")
        })?;
    let rate = row
        .rate
        .as_deref()
        .with_context(|| format!("the stock split for {instrument_id} carries no rate"))?;
    let effective = bars::date_ts_event(date).with_context(|| {
        format!("cannot place the stock split date '{date}' for {instrument_id}")
    })?;

    Ok(CorporateAction::new(
        instrument_id,
        CorporateActionType::Split,
        split_ratio(rate)?,
        None,
        effective,
        ts_event,
        ts_event,
    ))
}

/// Reads the cash amount per share out of the sentence the venue states it in.
///
/// The recorded sentence is `Cash Dividend: 0.27 USD Per Share`, so both the amount and the currency
/// are in the text. The amount is parsed from the written form rather than from a float, because it
/// is an exact decimal that a price series is adjusted by.
///
/// The currency is checked against the market's because the domain type carries no currency of its
/// own: a bare amount means nothing on its own, and a record whose amount is stated in another
/// currency is not the record this instrument's series needs.
///
/// # Errors
///
/// Returns an error if the sentence is not a cash amount per share, if the amount is unreadable, or
/// if the currency is not the market's.
fn cash_amount(statement: &str, market: Market) -> anyhow::Result<Decimal> {
    let invalid = || {
        anyhow::anyhow!(
            "the dividend statement '{statement}' is not a cash amount per share, so it states no value to report"
        )
    };

    let body = statement
        .strip_prefix(CASH_DIVIDEND_PREFIX)
        .and_then(|body| body.strip_suffix(PER_SHARE_SUFFIX))
        .ok_or_else(invalid)?;

    let (amount, stated) = body.split_once(' ').ok_or_else(invalid)?;
    let amount: Decimal = amount.trim().parse().map_err(|_| invalid())?;
    let stated = stated.trim();
    let expected = market.currency().code.as_str();

    if stated != expected {
        bail!(
            "the dividend statement '{statement}' states its amount in {stated}, which is not what {market} quotes in"
        );
    }

    Ok(amount)
}

/// Reads the new shares per old share out of the venue's ratio.
///
/// The recorded ratio is `1` then a right arrow then `4` for the four-for-one split, so the two
/// counts are the shares held before and the shares held after. The domain type wants the new count
/// per old one, which is the second divided by the first, and a consolidation is the same quantity
/// below one.
///
/// The counts are found by splitting on anything that is not a digit, rather than on the arrow
/// itself, so a separator the venue spells differently does not turn a ratio into a refusal. A
/// string that does not carry exactly two counts is refused: one count states no ratio, and three
/// state something this mapping does not understand.
///
/// # Errors
///
/// Returns an error if the value is not two share counts, or if the counts cannot be divided.
fn split_ratio(value: &str) -> anyhow::Result<Decimal> {
    let invalid = || {
        anyhow::anyhow!(
            "the stock split ratio '{value}' is not two share counts, so it states no ratio"
        )
    };

    let mut counts = value
        .split(|c: char| !c.is_ascii_digit() && c != '.')
        .filter(|count| !count.is_empty());

    let (Some(before), Some(after)) = (counts.next(), counts.next()) else {
        return Err(invalid());
    };

    if counts.next().is_some() {
        return Err(invalid());
    }

    let before: Decimal = before.parse().map_err(|_| invalid())?;
    let after: Decimal = after.parse().map_err(|_| invalid())?;

    after.checked_div(before).ok_or_else(invalid)
}

/// Converts the venue's `MM/DD/YYYY` calendar date into the instant the action takes effect.
///
/// The instant is UTC midnight of that date, placed by the helper the daily bars use, because an
/// action describes the same price series the bars describe and the two have to land on the same
/// instant. The components are read as numbers before they are placed, so a value in the wrong order
/// is refused by the calendar rather than accepted as a different date.
///
/// # Errors
///
/// Returns an error if the value is not three numbers, or if they are not a calendar date.
fn effective_ns(date: &str) -> anyhow::Result<UnixNanos> {
    let invalid = || anyhow::anyhow!("cannot read the corporate action date '{date}'");

    let mut parts = date.split('/');

    let (Some(month), Some(day), Some(year)) = (parts.next(), parts.next(), parts.next()) else {
        return Err(invalid());
    };

    if parts.next().is_some() {
        return Err(invalid());
    }

    let month: u8 = month.trim().parse().map_err(|_| invalid())?;
    let day: u8 = day.trim().parse().map_err(|_| invalid())?;
    let year: i16 = year.trim().parse().map_err(|_| invalid())?;

    bars::date_ts_event(&format!("{year:04}-{month:02}-{day:02}"))
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    fn instrument_id() -> InstrumentId {
        InstrumentId::from("AAPL.US")
    }

    /// The arrow the venue separates the two share counts with, written as an escape so that this
    /// source file stays in the characters this repository allows in its own code.
    const ARROW: char = '\u{2192}';

    /// One dividend record as the venue states it, taken from a recorded response.
    fn dividend(statement: &str, ex_date: &str) -> DividendItem {
        DividendItem {
            statement: Some(statement.to_string()),
            ex_date: Some(ex_date.to_string()),
            pub_date: Some("07/31/2026".to_string()),
            record_date: Some("08/10/2026".to_string()),
            dividend_payable_date: Some("08/13/2026".to_string()),
            ..Default::default()
        }
    }

    /// One split record as the venue states it, taken from a recorded response.
    fn split(rate: &str) -> StockSplitItem {
        StockSplitItem {
            dir_deci_pub_date: Some(1_598_846_400),
            dir_deci_pub_date_str: Some("2020-08-31".to_string()),
            reform_type: Some("Split".to_string()),
            rate: Some(rate.to_string()),
            ..Default::default()
        }
    }

    const TS: UnixNanos = UnixNanos::new(1_790_000_000_000_000_000);

    /// The amount is read out of the sentence, not out of a number field, because the feed has none.
    #[rstest]
    fn test_a_cash_dividend_states_its_amount_in_words() {
        let action = action_from_dividend(
            instrument_id(),
            &dividend("Cash Dividend: 0.27 USD Per Share", "08/10/2026"),
            Market::Us,
            TS,
        )
        .unwrap();

        assert_eq!(action.action, CorporateActionType::Dividend);
        assert_eq!(action.value, Decimal::new(27, 2));
        assert_eq!(action.new_symbol, None);
        assert_eq!(
            action.effective_ns,
            UnixNanos::new(1_786_320_000_000_000_000),
            "UTC midnight of the recorded ex-date, 08/10/2026",
        );
        assert_eq!(action.ts_event, TS);
        assert_eq!(action.ts_init, TS);
    }

    /// The Hong Kong statements carry five decimal places, and the written form keeps them.
    #[rstest]
    fn test_the_amount_keeps_the_scale_the_venue_stated() {
        let amount = cash_amount("Cash Dividend: 5.30000 HKD Per Share", Market::Hk).unwrap();

        assert_eq!(amount, Decimal::new(530_000, 5));
        assert_eq!(amount.scale(), 5);
    }

    /// The same field carries distributions in specie, which are not cash, and an adjustment stage
    /// handed a share count where it expects cash would subtract the wrong quantity from a price.
    #[rstest]
    #[case::distribution_in_specie(
        "Distribution in Specie: 1.00000 MEITUAN-W Share for Every 10.00000 Shares Held"
    )]
    #[case::empty("")]
    #[case::prose("A final dividend will be proposed at the annual meeting")]
    #[case::no_basis("Cash Dividend: 0.27 USD")]
    #[case::no_prefix("Dividend: 0.27 USD Per Share")]
    #[case::amount_missing("Cash Dividend: USD Per Share")]
    fn test_a_statement_that_is_not_cash_is_refused(#[case] statement: &str) {
        assert!(cash_amount(statement, Market::Us).is_err());
    }

    /// The domain type carries no currency, so a bare amount is only meaningful beside the market
    /// that quotes it, and a record stating another currency is not this instrument's record.
    #[rstest]
    #[case::us_states_hkd("Cash Dividend: 0.27 HKD Per Share", Market::Us)]
    #[case::hk_states_usd("Cash Dividend: 5.30000 USD Per Share", Market::Hk)]
    fn test_an_amount_in_another_currency_is_refused(
        #[case] statement: &str,
        #[case] market: Market,
    ) {
        assert!(cash_amount(statement, market).is_err());
    }

    /// The ratio is the count held after per the count held before, which for the four-for-one split
    /// is four, and for a consolidation is below one.
    #[rstest]
    #[case::four_for_one("1", "4")]
    #[case::seven_for_one("1", "7")]
    #[case::two_for_one_held_differently("2", "4")]
    #[case::consolidation("2", "1")]
    fn test_the_split_ratio_is_new_shares_per_old(#[case] before: &str, #[case] after: &str) {
        let rate = format!("{before}{ARROW}{after}");
        let expected = after.parse::<Decimal>().unwrap() / before.parse::<Decimal>().unwrap();

        assert_eq!(split_ratio(&rate).unwrap(), expected);
    }

    /// The recorded ratio is the arrow, so the fixture uses it, and the parse does not depend on
    /// which character the venue chose.
    #[rstest]
    #[case::arrow("1\u{2192}4")]
    #[case::ascii_arrow("1->4")]
    #[case::colon("1:4")]
    #[case::spaced("1 / 4")]
    fn test_a_ratio_is_read_whatever_separates_the_counts(#[case] rate: &str) {
        assert_eq!(split_ratio(rate).unwrap(), Decimal::new(4, 0));
    }

    #[rstest]
    #[case::empty("")]
    #[case::one_count("4")]
    #[case::three_counts("1\u{2192}4\u{2192}7")]
    #[case::no_counts("Split")]
    #[case::zero_denominator("0\u{2192}4")]
    fn test_a_value_that_is_not_two_counts_is_refused(#[case] rate: &str) {
        assert!(split_ratio(rate).is_err());
    }

    /// The recorded split states 2020-08-31 twice, as a calendar date and as its own epoch seconds,
    /// and the two are not the same instant: the seconds are the market's midnight, four hours into
    /// the UTC day. The action lands on UTC midnight, where the daily bar of the same date is.
    #[rstest]
    fn test_a_split_date_lands_where_the_bars_do() {
        let mut record = split("1\u{2192}4");
        record.dir_deci_pub_date = Some(1_598_846_400);

        let action = action_from_split(instrument_id(), &record, TS).unwrap();

        assert_eq!(action.action, CorporateActionType::Split);
        assert_eq!(action.value, Decimal::new(4, 0));
        assert_eq!(
            action.effective_ns,
            UnixNanos::new(1_598_832_000_000_000_000),
            "UTC midnight of 2020-08-31, where the daily bar of that date is",
        );
        assert_ne!(
            action.effective_ns,
            UnixNanos::new(1_598_846_400_000_000_000),
            "the venue's own seconds for the same date, which are the market's midnight",
        );
    }

    /// The ex-date is what the price series changes on, and it is the date an adjustment applies
    /// from; the announcement date only stands in where the venue states no ex-date.
    #[rstest]
    fn test_the_ex_date_is_preferred_over_the_announcement_date() {
        let mut record = split("1\u{2192}5");
        record.dir_deci_pub_date_str = Some("2014-03-19".to_string());
        record.ex_date_str = Some("2014-05-15".to_string());

        let action = action_from_split(InstrumentId::from("0700.HK"), &record, TS).unwrap();

        assert_eq!(
            action.effective_ns,
            UnixNanos::new(1_400_112_000_000_000_000),
            "UTC midnight of the recorded ex-date, not of the announcement date",
        );
    }

    #[rstest]
    fn test_a_split_with_no_date_is_refused() {
        let mut record = split("1\u{2192}4");
        record.dir_deci_pub_date = None;
        record.dir_deci_pub_date_str = None;
        record.ex_date = None;
        record.ex_date_str = None;

        assert!(action_from_split(instrument_id(), &record, TS).is_err());
    }

    /// The recorded dates are month first, and reading them the way the protocol comment describes
    /// would put the year where the month is. The instant they are placed on is UTC midnight, so
    /// `08/31/2020` lands on the same instant the split feed's calendar date for that day does.
    #[rstest]
    fn test_the_date_is_month_first() {
        assert_eq!(
            effective_ns("08/31/2020").unwrap(),
            UnixNanos::new(1_598_832_000_000_000_000),
        );
        assert_eq!(
            effective_ns("11/16/2022").unwrap(),
            UnixNanos::new(1_668_556_800_000_000_000),
        );
    }

    /// A value in the other order is refused rather than read as a different date, which is what
    /// makes the format above load-bearing rather than cosmetic.
    #[rstest]
    #[case::year_first("2020/08/31")]
    #[case::day_first("31/08/2020")]
    #[case::two_parts("08/2020")]
    #[case::four_parts("08/31/2020/01")]
    #[case::not_a_date("last quarter")]
    #[case::empty("")]
    fn test_a_date_in_another_shape_is_refused(#[case] date: &str) {
        assert!(effective_ns(date).is_err());
    }

    #[rstest]
    fn test_a_dividend_with_no_statement_or_no_ex_date_is_refused() {
        let mut no_statement = dividend("Cash Dividend: 0.27 USD Per Share", "08/10/2026");
        no_statement.statement = None;
        assert!(action_from_dividend(instrument_id(), &no_statement, Market::Us, TS).is_err());

        let mut no_ex_date = dividend("Cash Dividend: 0.27 USD Per Share", "08/10/2026");
        no_ex_date.ex_date = None;
        assert!(action_from_dividend(instrument_id(), &no_ex_date, Market::Us, TS).is_err());
    }
}
