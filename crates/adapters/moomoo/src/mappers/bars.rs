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

//! Conversion of the gateway's K-line records to the domain model.
//!
//! # Timestamps
//!
//! A bar coarser than a day carries a date, and that date is placed at UTC midnight. This is not
//! the only defensible choice, but it is the one the EODHD adapter makes for the same daily bar, and
//! the two have to agree: placing a daily bar at local midnight instead would put the two providers
//! five hours apart on the same bar, and a different number of hours either side of a daylight
//! saving change, so a series assembled across a provider failover would not line up.
//!
//! A bar finer than a day carries a wall clock reading in the exchange's own zone. It is resolved
//! through that market's time zone rather than by a fixed offset, because a fixed offset is wrong
//! for half the year: `09:30` in New York is `14:30` UTC in January and `13:30` UTC in July.
//!
//! # Ordering
//!
//! Rows arrive oldest first within a page and pages arrive in order, but the series is assembled
//! explicitly rather than trusting either. It is sorted by event time, and a repeated event time
//! keeps the last row for it, because a later row for the same bar is a correction and a consumer
//! that saw both could not tell which was which.

use anyhow::{Context, bail};
use jiff::{
    Timestamp, civil,
    tz::{Offset, TimeZone},
};
use nautilus_core::UnixNanos;
use nautilus_model::{
    data::{Bar, BarSpecification, BarType},
    enums::{AggregationSource, BarAggregation, PriceType},
    identifiers::InstrumentId,
    types::{Price, Quantity},
};

use crate::{common::Market, generated::qot_common::KLine};

/// The precision of a share count, which is a whole number.
const VOLUME_PRECISION: u8 = 0;

/// The adjustment applied to the prices of a K-line series.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Adjustment {
    /// Prices exactly as the venue published them.
    ///
    /// This is the default because it is the series the venue's own corporate actions describe, and
    /// because the EODHD adapter serves the same raw series, so a chain over the two agrees.
    #[default]
    None,
    /// Prices restated so that later events do not move them.
    Forward,
    /// Prices restated so that earlier events do not move them.
    Backward,
}

impl Adjustment {
    /// Returns the gateway's own code, per `Qot_Common.RehabType`.
    #[must_use]
    pub fn rehab_type(self) -> i32 {
        match self {
            Self::None => 0,
            Self::Forward => 1,
            Self::Backward => 2,
        }
    }
}

/// The session whose bars a request covers.
///
/// The venue never returns extended hours unless they are asked for, so this is a choice the caller
/// makes rather than something the adapter can infer from the interval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BarSession {
    /// The regular trading session only.
    #[default]
    Regular,
    /// The regular session plus extended hours.
    Extended,
    /// Every session the venue trades.
    All,
    /// The overnight session only.
    Overnight,
}

impl BarSession {
    /// Returns the gateway's own code, per `Common.Session`.
    #[must_use]
    pub fn session(self) -> i32 {
        match self {
            Self::Regular => 1,
            Self::Extended => 2,
            Self::All => 3,
            Self::Overnight => 4,
        }
    }

    /// Returns whether the request asks for bars outside the regular session.
    #[must_use]
    pub fn extended_time(self) -> bool {
        !matches!(self, Self::Regular)
    }
}

/// A K-line interval the gateway offers.
///
/// The vocabulary is the venue's and it is not dense: there is no two-minute bar, the hourly bars
/// are named by their minute count, and a quarter is a first-class interval. An interval the venue
/// does not offer is refused rather than rounded to the nearest one it does, because a strategy
/// asking for a bar it will not receive has a defect that rounding would hide.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interval {
    /// One minute.
    Minute1,
    /// Three minutes.
    Minute3,
    /// Five minutes.
    Minute5,
    /// Ten minutes.
    Minute10,
    /// Fifteen minutes.
    Minute15,
    /// Thirty minutes.
    Minute30,
    /// One hour, which the venue names as sixty minutes.
    Minute60,
    /// Two hours.
    Minute120,
    /// Three hours.
    Minute180,
    /// Four hours.
    Minute240,
    /// One day.
    Day,
    /// One week.
    Week,
    /// One month.
    Month,
    /// One quarter.
    Quarter,
    /// One year.
    Year,
}

impl Interval {
    /// Returns the gateway's own code, per `Qot_Common.KLType`.
    #[must_use]
    pub fn kl_type(self) -> i32 {
        match self {
            Self::Minute1 => 1,
            Self::Day => 2,
            Self::Week => 3,
            Self::Month => 4,
            Self::Year => 5,
            Self::Minute5 => 6,
            Self::Minute15 => 7,
            Self::Minute30 => 8,
            Self::Minute60 => 9,
            Self::Minute3 => 10,
            Self::Quarter => 11,
            Self::Minute10 => 12,
            Self::Minute120 => 13,
            Self::Minute180 => 14,
            Self::Minute240 => 15,
        }
    }

    /// Returns whether the interval is finer than a day.
    #[must_use]
    pub fn is_intraday(self) -> bool {
        !matches!(
            self,
            Self::Day | Self::Week | Self::Month | Self::Quarter | Self::Year
        )
    }

    /// Returns the interval a bar specification describes.
    ///
    /// # Errors
    ///
    /// Returns an error if the venue offers no bar of that aggregation and step.
    pub fn from_bar_specification(spec: &BarSpecification) -> anyhow::Result<Self> {
        let step = spec.step.get();

        let interval = match (spec.aggregation, step) {
            (BarAggregation::Minute, 1) => Self::Minute1,
            (BarAggregation::Minute, 3) => Self::Minute3,
            (BarAggregation::Minute, 5) => Self::Minute5,
            (BarAggregation::Minute, 10) => Self::Minute10,
            (BarAggregation::Minute, 15) => Self::Minute15,
            (BarAggregation::Minute, 30) => Self::Minute30,
            (BarAggregation::Hour, 1) => Self::Minute60,
            (BarAggregation::Hour, 2) => Self::Minute120,
            (BarAggregation::Hour, 3) => Self::Minute180,
            (BarAggregation::Hour, 4) => Self::Minute240,
            (BarAggregation::Day, 1) => Self::Day,
            (BarAggregation::Week, 1) => Self::Week,
            (BarAggregation::Month, 1) => Self::Month,
            (BarAggregation::Month, 3) => Self::Quarter,
            (BarAggregation::Year, 1) => Self::Year,
            (aggregation, step) => {
                bail!("the gateway offers no {step}-step {aggregation:?} bar");
            }
        };

        Ok(interval)
    }

    /// Returns the bar specification this interval describes.
    #[must_use]
    pub fn bar_specification(self) -> BarSpecification {
        let (step, aggregation) = match self {
            Self::Minute1 => (1, BarAggregation::Minute),
            Self::Minute3 => (3, BarAggregation::Minute),
            Self::Minute5 => (5, BarAggregation::Minute),
            Self::Minute10 => (10, BarAggregation::Minute),
            Self::Minute15 => (15, BarAggregation::Minute),
            Self::Minute30 => (30, BarAggregation::Minute),
            Self::Minute60 => (1, BarAggregation::Hour),
            Self::Minute120 => (2, BarAggregation::Hour),
            Self::Minute180 => (3, BarAggregation::Hour),
            Self::Minute240 => (4, BarAggregation::Hour),
            Self::Day => (1, BarAggregation::Day),
            Self::Week => (1, BarAggregation::Week),
            Self::Month => (1, BarAggregation::Month),
            Self::Quarter => (3, BarAggregation::Month),
            Self::Year => (1, BarAggregation::Year),
        };

        BarSpecification::new(step, aggregation, PriceType::Last)
    }
}

/// Builds the bar type for an instrument and interval.
#[must_use]
pub fn bar_type(instrument_id: InstrumentId, interval: Interval) -> BarType {
    BarType::new(
        instrument_id,
        interval.bar_specification(),
        AggregationSource::External,
    )
}

/// Converts a bar timestamp before the Unix epoch into nanoseconds.
fn nanos(timestamp: Timestamp) -> anyhow::Result<u64> {
    u64::try_from(timestamp.as_nanosecond()).context("the bar timestamp is before the Unix epoch")
}

/// Converts a K-line's own timestamp into an event time.
///
/// # Errors
///
/// Returns an error if the value cannot be read as a date or a wall clock time, if it carries a
/// sub-second part this adapter does not place, if the market's time zone cannot be resolved, or if
/// the wall clock reading does not exist in that zone.
pub fn bar_ts_event(value: &str, interval: Interval, market: Market) -> anyhow::Result<UnixNanos> {
    let (seconds, fraction) = value
        .split_once('.')
        .map_or((value, ""), |(seconds, fraction)| (seconds, fraction));

    // A bar is aligned to its own interval, so a fractional second would mean the venue changed the
    // format. Failing is better than placing the bar a fraction early and calling it a duplicate.
    if fraction.bytes().any(|byte| byte != b'0') {
        bail!("the bar timestamp {value} carries a sub-second part that cannot be placed");
    }

    if interval.is_intraday() {
        let datetime = civil::DateTime::strptime("%Y-%m-%d %H:%M:%S", seconds)
            .with_context(|| format!("cannot read the bar timestamp {value}"))?;
        let zone = TimeZone::get(market.time_zone())
            .with_context(|| format!("cannot resolve the {market} time zone"))?;
        let timestamp = zone
            .to_timestamp(datetime)
            .with_context(|| format!("{value} is not a valid local time in {market}"))?;

        return Ok(UnixNanos::from(nanos(timestamp)?));
    }

    let date = &seconds[..seconds.len().min(10)];
    let date = civil::Date::strptime("%Y-%m-%d", date)
        .with_context(|| format!("cannot read the bar date {value}"))?;
    let timestamp = Offset::UTC
        .to_timestamp(date.at(0, 0, 0, 0))
        .with_context(|| format!("cannot place the bar date {value}"))?;

    Ok(UnixNanos::from(nanos(timestamp)?))
}

/// Builds one ordered, deduplicated bar series from the rows of one or more pages.
///
/// A blank row marks an interval in which nothing traded and carries only its time, so it is
/// skipped: emitting it would invent a bar whose open, high, low and close are all the same number.
///
/// # Errors
///
/// Returns an error if a row's timestamp cannot be placed, if a row is missing a price it needs, or
/// if a row violates an OHLC relationship.
pub fn build_bars(
    rows: &[KLine],
    bar_type: BarType,
    interval: Interval,
    market: Market,
    price_precision: u8,
) -> anyhow::Result<Vec<Bar>> {
    let mut ordered: Vec<Bar> = Vec::with_capacity(rows.len());

    for row in rows {
        if row.is_blank {
            continue;
        }

        ordered.push(build_bar(row, bar_type, interval, market, price_precision)?);
    }

    ordered.sort_by_key(|bar| bar.ts_event);

    let mut series: Vec<Bar> = Vec::with_capacity(ordered.len());

    for bar in ordered {
        match series.last_mut() {
            Some(last) if last.ts_event == bar.ts_event => *last = bar,
            _ => series.push(bar),
        }
    }

    Ok(series)
}

fn build_bar(
    row: &KLine,
    bar_type: BarType,
    interval: Interval,
    market: Market,
    price_precision: u8,
) -> anyhow::Result<Bar> {
    let ts_event = bar_ts_event(&row.time, interval, market)?;
    let missing = |field: &'static str| format!("the bar at {} carries no {field}", row.time);

    let open = Price::new_checked(
        row.open_price.with_context(|| missing("open"))?,
        price_precision,
    )?;
    let high = Price::new_checked(
        row.high_price.with_context(|| missing("high"))?,
        price_precision,
    )?;
    let low = Price::new_checked(
        row.low_price.with_context(|| missing("low"))?,
        price_precision,
    )?;
    let close = Price::new_checked(
        row.close_price.with_context(|| missing("close"))?,
        price_precision,
    )?;
    let volume = Quantity::new_checked(row.volume.unwrap_or_default() as f64, VOLUME_PRECISION)?;

    Bar::new_checked(bar_type, open, high, low, close, volume, ts_event, ts_event)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    /// `09:30` in New York on a winter day is `14:30` UTC, and the same wall clock on a summer day
    /// is `13:30` UTC. A fixed offset gets one of the two wrong.
    const WINTER_OPEN: UnixNanos = UnixNanos::new(1_768_487_400_000_000_000);
    const SUMMER_OPEN: UnixNanos = UnixNanos::new(1_773_149_400_000_000_000);
    const MIDSUMMER_OPEN: UnixNanos = UnixNanos::new(1_782_912_600_000_000_000);
    const WINTER_MIDNIGHT: UnixNanos = UnixNanos::new(1_768_435_200_000_000_000);

    fn row(time: &str) -> KLine {
        KLine {
            time: time.to_string(),
            is_blank: false,
            open_price: Some(1.0),
            high_price: Some(2.0),
            low_price: Some(0.5),
            close_price: Some(1.5),
            volume: Some(100),
            ..Default::default()
        }
    }

    #[rstest]
    #[case::winter("2026-01-15 09:30:00", WINTER_OPEN)]
    #[case::summer("2026-03-10 09:30:00", SUMMER_OPEN)]
    #[case::midsummer("2026-07-01 09:30:00", MIDSUMMER_OPEN)]
    fn test_intraday_timestamps_follow_the_market_zone(
        #[case] value: &str,
        #[case] expected: UnixNanos,
    ) {
        assert_eq!(
            bar_ts_event(value, Interval::Minute1, Market::Us).unwrap(),
            expected
        );
    }

    /// A daily bar is a date, and the date is placed at UTC midnight so that the two providers
    /// agree on which instant the same daily bar is.
    #[rstest]
    #[case::daily("2026-01-15 00:00:00", Interval::Day)]
    #[case::daily_ignoring_the_time("2026-01-15 09:30:00", Interval::Day)]
    #[case::weekly("2026-01-15 00:00:00", Interval::Week)]
    #[case::monthly("2026-01-15 00:00:00", Interval::Month)]
    fn test_coarse_timestamps_are_placed_at_utc_midnight(
        #[case] value: &str,
        #[case] interval: Interval,
    ) {
        assert_eq!(
            bar_ts_event(value, interval, Market::Us).unwrap(),
            WINTER_MIDNIGHT
        );
    }

    /// Hong Kong has no daylight saving, so its wall clock is its offset.
    #[rstest]
    #[case::hong_kong("2026-01-15 09:30:00", UnixNanos::new(1_768_440_600_000_000_000))]
    fn test_another_market_uses_its_own_zone(#[case] value: &str, #[case] expected: UnixNanos) {
        assert_eq!(
            bar_ts_event(value, Interval::Minute1, Market::Hk).unwrap(),
            expected
        );
    }

    #[rstest]
    #[case::fraction("2026-01-15 09:30:00.500")]
    #[case::nonsense("not a time")]
    fn test_unplaceable_timestamps_are_refused(#[case] value: &str) {
        assert!(bar_ts_event(value, Interval::Minute1, Market::Us).is_err());
    }

    /// The venue's vocabulary is irregular, and an interval it does not offer must be refused
    /// rather than rounded to one it does.
    #[rstest]
    #[case::one_minute(BarAggregation::Minute, 1, Some(Interval::Minute1))]
    #[case::two_minutes(BarAggregation::Minute, 2, None)]
    #[case::four_minutes(BarAggregation::Minute, 4, None)]
    #[case::one_hour(BarAggregation::Hour, 1, Some(Interval::Minute60))]
    #[case::six_hours(BarAggregation::Hour, 6, None)]
    #[case::day(BarAggregation::Day, 1, Some(Interval::Day))]
    #[case::quarter(BarAggregation::Month, 3, Some(Interval::Quarter))]
    #[case::second(BarAggregation::Second, 1, None)]
    fn test_interval_mapping(
        #[case] aggregation: BarAggregation,
        #[case] step: usize,
        #[case] expected: Option<Interval>,
    ) {
        let spec = BarSpecification::new(step, aggregation, PriceType::Last);

        assert_eq!(
            Interval::from_bar_specification(&spec).ok(),
            expected,
            "for {step}-step {aggregation:?}"
        );
    }

    #[rstest]
    fn test_interval_codes_round_trip() {
        for interval in [
            Interval::Minute1,
            Interval::Minute3,
            Interval::Minute5,
            Interval::Minute10,
            Interval::Minute15,
            Interval::Minute30,
            Interval::Minute60,
            Interval::Minute120,
            Interval::Minute180,
            Interval::Minute240,
            Interval::Day,
            Interval::Week,
            Interval::Month,
            Interval::Quarter,
            Interval::Year,
        ] {
            let spec = interval.bar_specification();

            assert_eq!(
                Interval::from_bar_specification(&spec).unwrap(),
                interval,
                "for {interval:?}"
            );
        }
    }

    #[rstest]
    fn test_a_series_is_ordered_and_deduplicated() {
        let rows = vec![
            row("2026-01-15 09:32:00"),
            row("2026-01-15 09:30:00"),
            // A second row for a time already seen, as a later page can supply.
            KLine {
                close_price: Some(9.0),
                high_price: Some(10.0),
                ..row("2026-01-15 09:31:00")
            },
            KLine {
                close_price: Some(9.5),
                high_price: Some(10.0),
                ..row("2026-01-15 09:31:00")
            },
        ];

        let bars = build_bars(
            &rows,
            bar_type(InstrumentId::from("AAPL.US"), Interval::Minute1),
            Interval::Minute1,
            Market::Us,
            2,
        )
        .unwrap();

        assert_eq!(bars.len(), 3, "one bar per distinct time");
        assert_eq!(
            bars[0].ts_event,
            bar_ts_event("2026-01-15 09:30:00", Interval::Minute1, Market::Us).unwrap()
        );
        assert_eq!(
            bars[1].ts_event,
            bar_ts_event("2026-01-15 09:31:00", Interval::Minute1, Market::Us).unwrap()
        );
        assert_eq!(
            bars[2].ts_event,
            bar_ts_event("2026-01-15 09:32:00", Interval::Minute1, Market::Us).unwrap()
        );
        assert_eq!(
            bars[1].close,
            Price::new(9.5, 2),
            "the later row for a time is the one kept"
        );
    }

    /// A blank row carries only a time, so it must not become a bar of identical prices.
    #[rstest]
    fn test_blank_rows_are_skipped() {
        let rows = vec![
            row("2026-01-15 09:30:00"),
            KLine {
                time: "2026-01-15 09:31:00".to_string(),
                is_blank: true,
                ..Default::default()
            },
        ];

        let bars = build_bars(
            &rows,
            bar_type(InstrumentId::from("AAPL.US"), Interval::Minute1),
            Interval::Minute1,
            Market::Us,
            2,
        )
        .unwrap();

        assert_eq!(bars.len(), 1);
    }

    /// A row missing a price is a row the venue did not publish fully, and filling it with zero
    /// would make a bar that never traded.
    #[rstest]
    fn test_a_row_without_prices_is_refused() {
        let rows = vec![KLine {
            time: "2026-01-15 09:30:00".to_string(),
            is_blank: false,
            ..Default::default()
        }];

        assert!(
            build_bars(
                &rows,
                bar_type(InstrumentId::from("AAPL.US"), Interval::Minute1),
                Interval::Minute1,
                Market::Us,
                2,
            )
            .is_err()
        );
    }

    #[rstest]
    fn test_bar_type_is_external() {
        let bar_type = bar_type(InstrumentId::from("AAPL.US"), Interval::Minute5);

        assert_eq!(bar_type.aggregation_source(), AggregationSource::External);
        assert_eq!(bar_type.to_string(), "AAPL.US-5-MINUTE-LAST-EXTERNAL");
    }

    #[rstest]
    #[case::regular(BarSession::Regular, 1, false)]
    #[case::extended(BarSession::Extended, 2, true)]
    #[case::all(BarSession::All, 3, true)]
    #[case::overnight(BarSession::Overnight, 4, true)]
    fn test_session_codes(#[case] session: BarSession, #[case] code: i32, #[case] extended: bool) {
        assert_eq!(session.session(), code);
        assert_eq!(session.extended_time(), extended);
    }

    #[rstest]
    #[case::raw(Adjustment::None, 0)]
    #[case::forward(Adjustment::Forward, 1)]
    #[case::backward(Adjustment::Backward, 2)]
    fn test_adjustment_codes(#[case] adjustment: Adjustment, #[case] code: i32) {
        assert_eq!(adjustment.rehab_type(), code);
    }
}
