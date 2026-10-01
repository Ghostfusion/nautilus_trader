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

//! EODHD interval mapping and conversion of EODHD rows into Nautilus bars.
//!
//! EODHD serves bars from two endpoints: `/eod` for daily, weekly, and monthly periods, and
//! `/intraday` for one minute, five minute, and one hour intervals. Both endpoints publish
//! `LAST` prices, so a bar specification that asks for anything else is rejected rather than
//! silently served the wrong series.

use jiff::{civil::Date, tz::Offset};
use nautilus_core::UnixNanos;
use nautilus_model::{
    data::{Bar, BarSpecification, BarType},
    enums::{AggregationSource, BarAggregation, PriceType},
    identifiers::InstrumentId,
    types::{Price, Quantity},
};

use crate::http::{EodhdBar, EodhdIntradayBar};

/// The size precision applied to EODHD share volumes.
const VOLUME_PRECISION: u8 = 0;

/// Seconds in a day, used to bound an inclusive end date.
const SECONDS_PER_DAY: i64 = 86_400;

/// The EODHD endpoint and query interval that serves a bar specification.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EodhdInterval {
    /// The `/eod` endpoint at a daily period.
    Day,
    /// The `/eod` endpoint at a weekly period.
    Week,
    /// The `/eod` endpoint at a monthly period.
    Month,
    /// The `/intraday` endpoint at one minute.
    Minute1,
    /// The `/intraday` endpoint at five minutes.
    Minute5,
    /// The `/intraday` endpoint at one hour.
    Hour1,
}

impl EodhdInterval {
    /// Returns the `period` or `interval` query value for this interval.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Day => "d",
            Self::Week => "w",
            Self::Month => "m",
            Self::Minute1 => "1m",
            Self::Minute5 => "5m",
            Self::Hour1 => "1h",
        }
    }

    /// Returns whether this interval is served by the intraday endpoint.
    #[must_use]
    pub const fn is_intraday(self) -> bool {
        matches!(self, Self::Minute1 | Self::Minute5 | Self::Hour1)
    }

    /// Returns the nominal duration of this interval in seconds.
    #[must_use]
    pub const fn seconds(self) -> i64 {
        match self {
            Self::Minute1 => 60,
            Self::Minute5 => 300,
            Self::Hour1 => 3_600,
            Self::Day => SECONDS_PER_DAY,
            Self::Week => 7 * SECONDS_PER_DAY,
            Self::Month => 30 * SECONDS_PER_DAY,
        }
    }
}

/// Parses an EODHD period or interval code into an [`EodhdInterval`].
///
/// Accepts the query values themselves (`d`, `w`, `m`, `1m`, `5m`, `1h`) as well as their spelled
/// forms.
///
/// # Errors
///
/// Returns an error if the code is not one of the supported values.
pub fn parse_interval(period: &str) -> anyhow::Result<EodhdInterval> {
    match period.to_ascii_lowercase().as_str() {
        "" | "d" | "day" | "daily" => Ok(EodhdInterval::Day),
        "w" | "week" | "weekly" => Ok(EodhdInterval::Week),
        "m" | "month" | "monthly" => Ok(EodhdInterval::Month),
        "1m" | "minute" | "1min" => Ok(EodhdInterval::Minute1),
        "5m" | "5min" => Ok(EodhdInterval::Minute5),
        "1h" | "hour" | "hourly" => Ok(EodhdInterval::Hour1),
        other => anyhow::bail!(
            "Unsupported EODHD period '{other}': expected one of 'd', 'w', 'm', '1m', '5m', or '1h'"
        ),
    }
}

/// Resolves the [`EodhdInterval`] that serves `spec`.
///
/// # Errors
///
/// Returns an error if the aggregation, step, or price type is not served by EODHD.
pub fn resolve_interval(spec: &BarSpecification) -> anyhow::Result<EodhdInterval> {
    if spec.price_type != PriceType::Last {
        anyhow::bail!(
            "EODHD publishes last prices only, so a {:?} price bar cannot be served",
            spec.price_type
        );
    }

    match (spec.aggregation, spec.step.get()) {
        (BarAggregation::Day, 1) => Ok(EodhdInterval::Day),
        (BarAggregation::Week, 1) => Ok(EodhdInterval::Week),
        (BarAggregation::Month, 1) => Ok(EodhdInterval::Month),
        (BarAggregation::Minute, 1) => Ok(EodhdInterval::Minute1),
        (BarAggregation::Minute, 5) => Ok(EodhdInterval::Minute5),
        (BarAggregation::Hour, 1) => Ok(EodhdInterval::Hour1),
        (aggregation, step) => anyhow::bail!(
            "EODHD does not serve a {step}-{aggregation:?} bar: supported bars are \
             1-MINUTE, 5-MINUTE, 1-HOUR, 1-DAY, 1-WEEK, and 1-MONTH"
        ),
    }
}

/// Returns the externally aggregated Nautilus bar type for an EODHD `interval`.
///
/// EODHD aggregates the bars, so the aggregation source is always
/// [`AggregationSource::External`].
#[must_use]
pub fn bar_type_for(instrument_id: InstrumentId, interval: EodhdInterval) -> BarType {
    let spec = match interval {
        EodhdInterval::Day => BarSpecification::new(1, BarAggregation::Day, PriceType::Last),
        EodhdInterval::Week => BarSpecification::new(1, BarAggregation::Week, PriceType::Last),
        EodhdInterval::Month => BarSpecification::new(1, BarAggregation::Month, PriceType::Last),
        EodhdInterval::Minute1 => BarSpecification::new(1, BarAggregation::Minute, PriceType::Last),
        EodhdInterval::Minute5 => BarSpecification::new(5, BarAggregation::Minute, PriceType::Last),
        EodhdInterval::Hour1 => BarSpecification::new(1, BarAggregation::Hour, PriceType::Last),
    };

    BarType::new(instrument_id, spec, AggregationSource::External)
}

/// Returns the UTC millisecond timestamp for an EODHD `YYYY-MM-DD` date.
///
/// # Errors
///
/// Returns an error if the date cannot be parsed.
pub fn date_to_millis(value: &str) -> anyhow::Result<i64> {
    let date = Date::strptime("%Y-%m-%d", value).map_err(|e| anyhow::anyhow!("{e}"))?;
    let timestamp = Offset::UTC
        .to_timestamp(date.at(0, 0, 0, 0))
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    Ok(timestamp.as_millisecond())
}

/// Returns the epoch second for the start of an EODHD `YYYY-MM-DD` date.
///
/// # Errors
///
/// Returns an error if the date cannot be parsed.
pub fn date_start_secs(value: &str) -> anyhow::Result<i64> {
    Ok(date_to_millis(value)? / 1_000)
}

/// Returns the epoch second for the last second of an EODHD `YYYY-MM-DD` date.
///
/// The returned bound is inclusive, so a request ending on a date covers that whole trading day.
///
/// # Errors
///
/// Returns an error if the date cannot be parsed.
pub fn date_end_secs(value: &str) -> anyhow::Result<i64> {
    Ok(date_start_secs(value)? + SECONDS_PER_DAY - 1)
}

/// Converts EODHD end-of-day rows into Nautilus bars.
///
/// Rows are converted in the order supplied, which for the `/eod` endpoint is ascending by date
/// unless the request asked for descending order.
///
/// # Errors
///
/// Returns an error if a row date is unparseable, or if a row violates an OHLC relationship, in
/// which case the offending field is reported rather than a bar being filled with values the
/// provider never published.
pub fn build_eod_bars(
    rows: &[EodhdBar],
    bar_type: BarType,
    price_precision: u8,
) -> anyhow::Result<Vec<Bar>> {
    let mut bars = Vec::with_capacity(rows.len());

    for row in rows {
        let millis = date_to_millis(&row.date)?;
        let ts_event = UnixNanos::from_millis(millis.unsigned_abs());

        bars.push(build_bar(
            bar_type,
            row.open,
            row.high,
            row.low,
            row.close,
            row.volume,
            ts_event,
            price_precision,
        )?);
    }

    Ok(bars)
}

/// Converts EODHD intraday rows into Nautilus bars.
///
/// The row `timestamp` field is used as the bar timestamp: it is the authoritative epoch second,
/// whereas the `datetime` field is rendered in the exchange offset carried by `gmtoffset`.
///
/// # Errors
///
/// Returns an error if a row timestamp is negative, or if a row violates an OHLC relationship.
pub fn build_intraday_bars(
    rows: &[EodhdIntradayBar],
    bar_type: BarType,
    price_precision: u8,
) -> anyhow::Result<Vec<Bar>> {
    let mut bars = Vec::with_capacity(rows.len());

    for row in rows {
        if row.timestamp < 0 {
            anyhow::bail!("Invalid EODHD intraday timestamp '{}'", row.timestamp);
        }

        let millis = row.timestamp * 1_000;

        bars.push(build_bar(
            bar_type,
            row.open,
            row.high,
            row.low,
            row.close,
            row.volume,
            UnixNanos::from_millis(millis.unsigned_abs()),
            price_precision,
        )?);
    }

    Ok(bars)
}

#[expect(clippy::too_many_arguments)]
fn build_bar(
    bar_type: BarType,
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: Option<f64>,
    ts_event: UnixNanos,
    price_precision: u8,
) -> anyhow::Result<Bar> {
    let open = Price::new_checked(open, price_precision)?;
    let high = Price::new_checked(high, price_precision)?;
    let low = Price::new_checked(low, price_precision)?;
    let close = Price::new_checked(close, price_precision)?;
    let volume = Quantity::new_checked(volume.unwrap_or(0.0), VOLUME_PRECISION)?;

    Bar::new_checked(bar_type, open, high, low, close, volume, ts_event, ts_event)
}

#[cfg(test)]
mod tests {
    use nautilus_model::enums::{AggregationSource, BarAggregation, PriceType};
    use rstest::rstest;

    use super::*;

    fn instrument_id() -> InstrumentId {
        InstrumentId::from("AAPL.US")
    }

    fn eod_row(date: &str, close: f64) -> EodhdBar {
        EodhdBar {
            date: date.to_string(),
            open: close - 1.0,
            high: close + 1.0,
            low: close - 2.0,
            close,
            adjusted_close: None,
            volume: Some(1_000.0),
        }
    }

    fn intraday_row(timestamp: i64, close: f64) -> EodhdIntradayBar {
        EodhdIntradayBar {
            timestamp,
            gmtoffset: Some(0),
            datetime: None,
            open: close - 1.0,
            high: close + 1.0,
            low: close - 2.0,
            close,
            volume: Some(100.0),
        }
    }

    #[rstest]
    #[case("d", EodhdInterval::Day)]
    #[case("", EodhdInterval::Day)]
    #[case("w", EodhdInterval::Week)]
    #[case("m", EodhdInterval::Month)]
    #[case("1m", EodhdInterval::Minute1)]
    #[case("5m", EodhdInterval::Minute5)]
    #[case("1h", EodhdInterval::Hour1)]
    fn test_parse_interval_accepts_the_documented_codes(
        #[case] code: &str,
        #[case] expected: EodhdInterval,
    ) {
        assert_eq!(parse_interval(code).unwrap(), expected);
    }

    #[rstest]
    fn test_parse_interval_rejects_an_unknown_code() {
        let result = parse_interval("x");

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unsupported EODHD period")
        );
    }

    #[rstest]
    fn test_interval_codes_and_intraday_flags() {
        assert_eq!(EodhdInterval::Day.code(), "d");
        assert_eq!(EodhdInterval::Minute1.code(), "1m");
        assert_eq!(EodhdInterval::Hour1.code(), "1h");
        assert!(EodhdInterval::Minute1.is_intraday());
        assert!(EodhdInterval::Hour1.is_intraday());
        assert!(!EodhdInterval::Day.is_intraday());
    }

    #[rstest]
    #[case(BarAggregation::Minute, 1, EodhdInterval::Minute1)]
    #[case(BarAggregation::Minute, 5, EodhdInterval::Minute5)]
    #[case(BarAggregation::Hour, 1, EodhdInterval::Hour1)]
    #[case(BarAggregation::Day, 1, EodhdInterval::Day)]
    #[case(BarAggregation::Week, 1, EodhdInterval::Week)]
    #[case(BarAggregation::Month, 1, EodhdInterval::Month)]
    fn test_resolve_interval_maps_supported_specs(
        #[case] aggregation: BarAggregation,
        #[case] step: usize,
        #[case] expected: EodhdInterval,
    ) {
        let spec = BarSpecification::new(step, aggregation, PriceType::Last);

        assert_eq!(resolve_interval(&spec).unwrap(), expected);
    }

    #[rstest]
    #[case(BarAggregation::Minute, 15)]
    #[case(BarAggregation::Second, 30)]
    #[case(BarAggregation::Tick, 100)]
    #[case(BarAggregation::Day, 2)]
    fn test_resolve_interval_rejects_unsupported_specs(
        #[case] aggregation: BarAggregation,
        #[case] step: usize,
    ) {
        let spec = BarSpecification::new(step, aggregation, PriceType::Last);

        let error = resolve_interval(&spec).unwrap_err().to_string();

        assert!(error.contains("does not serve"), "{error}");
    }

    #[rstest]
    fn test_resolve_interval_rejects_a_non_last_price_type() {
        let spec = BarSpecification::new(1, BarAggregation::Minute, PriceType::Mid);

        let error = resolve_interval(&spec).unwrap_err().to_string();

        assert!(error.contains("last prices only"), "{error}");
    }

    #[rstest]
    fn test_bar_type_for_uses_external_aggregation() {
        let bar_type = bar_type_for(instrument_id(), EodhdInterval::Minute5);

        assert_eq!(bar_type.to_string(), "AAPL.US-5-MINUTE-LAST-EXTERNAL");
        assert_eq!(bar_type.aggregation_source(), AggregationSource::External);
    }

    #[rstest]
    fn test_date_bounds_cover_the_whole_end_day() {
        assert_eq!(date_start_secs("2024-01-02").unwrap(), 1_704_153_600);
        assert_eq!(date_end_secs("2024-01-02").unwrap(), 1_704_239_999);
    }

    #[rstest]
    fn test_date_to_millis_is_utc_midnight() {
        assert_eq!(date_to_millis("1970-01-01").unwrap(), 0);
        assert_eq!(date_to_millis("1970-01-02").unwrap(), 86_400_000);
        assert_eq!(date_to_millis("2024-01-02").unwrap(), 1_704_153_600_000);
    }

    #[rstest]
    fn test_date_to_millis_rejects_a_malformed_date() {
        assert!(date_to_millis("02/01/2024").is_err());
    }

    #[rstest]
    fn test_build_eod_bars_preserves_the_supplied_order() {
        let rows = vec![eod_row("2024-01-02", 185.64), eod_row("2024-01-03", 184.25)];

        let bars =
            build_eod_bars(&rows, bar_type_for(instrument_id(), EodhdInterval::Day), 2).unwrap();

        assert_eq!(bars.len(), 2);
        assert_eq!(bars[0].bar_type.to_string(), "AAPL.US-1-DAY-LAST-EXTERNAL");
        assert!(bars[0].ts_event < bars[1].ts_event);
        assert_eq!(bars[0].close, Price::new(185.64, 2));
        assert_eq!(bars[0].volume, Quantity::new(1_000.0, 0));
    }

    #[rstest]
    fn test_build_eod_bars_rejects_a_row_with_an_impossible_high() {
        let mut invalid = eod_row("2024-01-02", 185.64);
        invalid.high = invalid.close - 5.0;

        let result = build_eod_bars(
            &[invalid],
            bar_type_for(instrument_id(), EodhdInterval::Day),
            2,
        );

        assert!(result.is_err());
    }

    #[rstest]
    fn test_build_eod_bars_treats_a_missing_volume_as_zero() {
        let mut no_volume = eod_row("2024-01-02", 185.64);
        no_volume.volume = None;

        let bars = build_eod_bars(
            &[no_volume],
            bar_type_for(instrument_id(), EodhdInterval::Day),
            2,
        )
        .unwrap();

        assert_eq!(bars[0].volume, Quantity::new(0.0, 0));
    }

    #[rstest]
    fn test_build_intraday_bars_uses_the_epoch_timestamp() {
        let rows = vec![intraday_row(1_780_061_400, 314.35)];

        let bars = build_intraday_bars(
            &rows,
            bar_type_for(instrument_id(), EodhdInterval::Minute5),
            2,
        )
        .unwrap();

        assert_eq!(bars.len(), 1);
        assert_eq!(
            bars[0].bar_type.to_string(),
            "AAPL.US-5-MINUTE-LAST-EXTERNAL"
        );
        assert_eq!(bars[0].ts_event, UnixNanos::from_millis(1_780_061_400_000));
        assert_eq!(bars[0].close, Price::new(314.35, 2));
        assert_eq!(bars[0].volume, Quantity::new(100.0, 0));
    }

    #[rstest]
    fn test_build_intraday_bars_rejects_a_negative_timestamp() {
        let rows = vec![intraday_row(-1, 314.35)];

        let result = build_intraday_bars(
            &rows,
            bar_type_for(instrument_id(), EodhdInterval::Minute1),
            2,
        );

        assert!(result.is_err());
    }
}
