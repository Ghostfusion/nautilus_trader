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

//! Trading calendars as immutable data.
//!
//! A [`TradingCalendar`] describes when a market trades: weekly sessions expressed in exchange
//! local time, holidays, early closes, and the date range the data covers. It answers whether a
//! given instant is tradeable, and what the next or previous session boundary is.
//!
//! Two responsibilities stay separate:
//!
//! - Instrument lifetime belongs to the instrument (`activation_ns` and `expiration_ns`).
//! - A calendar only answers whether an instant is tradeable, and when the sessions are.
//!
//! A calendar is an immutable input to a run. It is loaded once, validated on load, and never
//! mutated while the run is in progress.

use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    fmt::Display,
    path::Path,
    str::FromStr,
    sync::LazyLock,
};

use anyhow::{Context, Result, anyhow, bail, ensure};
use jiff::{
    Span, Timestamp,
    civil::{Date, Time, Weekday},
    tz::TimeZone,
};
use log::warn;
use serde::Deserialize;

use crate::{
    enums::AssetClass,
    identifiers::{Symbol, Venue},
};
use nautilus_core::datetime::get_timezone;

/// The schema identifier of the calendar JSON format.
pub const CALENDAR_SCHEMA: &str = "nautilus-trading-calendar/v1";

/// The maximum number of days a boundary search walks before giving up.
///
/// A search only walks this far when a calendar declares no session for the required direction,
/// which load-time validation already rejects for a well-formed calendar.
const MAX_BOUNDARY_SEARCH_DAYS: i64 = 730;

/// Which edge of a session a boundary request refers to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Boundary {
    Open,
    Close,
}

/// Identifies the market a calendar describes.
///
/// The key is the venue plus the instrument class plus an optional symbol, which is specific enough
/// to resolve an equity calendar per listing and a foreign exchange calendar per session.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CalendarKey {
    /// The venue the calendar applies to.
    pub venue: Venue,
    /// The instrument class the calendar applies to.
    pub asset_class: AssetClass,
    /// The optional symbol the calendar applies to.
    pub symbol: Option<Symbol>,
}

impl CalendarKey {
    /// Creates a new [`CalendarKey`].
    #[must_use]
    pub const fn new(venue: Venue, asset_class: AssetClass, symbol: Option<Symbol>) -> Self {
        Self {
            venue,
            asset_class,
            symbol,
        }
    }
}

impl Display for CalendarKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.symbol {
            Some(symbol) => write!(f, "{}.{}.{symbol}", self.venue, self.asset_class),
            None => write!(f, "{}.{}", self.venue, self.asset_class),
        }
    }
}

/// A trading session expressed in exchange local time.
///
/// Sessions do not cross midnight: a market that trades overnight is described as two sessions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TradingSession {
    /// The session open.
    pub start: Time,
    /// The session close.
    pub end: Time,
}

impl TradingSession {
    /// Creates a new [`TradingSession`].
    #[must_use]
    pub const fn new(start: Time, end: Time) -> Self {
        Self { start, end }
    }

    /// Returns whether the given local time is inside this session.
    #[must_use]
    pub fn contains(&self, time: Time) -> bool {
        self.start <= time && time < self.end
    }
}

/// An immutable trading calendar.
#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.model", from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.model")
)]
pub struct TradingCalendar {
    key: CalendarKey,
    time_zone: TimeZone,
    time_zone_name: String,
    sessions: HashMap<Weekday, Vec<TradingSession>>,
    holidays: BTreeSet<Date>,
    early_closes: BTreeMap<Date, Time>,
    valid_from: Date,
    valid_until: Option<Date>,
    source: Option<String>,
}

impl TradingCalendar {
    /// Parses and validates a calendar from JSON text.
    ///
    /// # Errors
    ///
    /// Returns an error if the JSON is malformed, the schema is unsupported, a session is invalid
    /// or overlapping, a date or time cannot be parsed, or the coverage range is inverted.
    pub fn from_json_str(text: &str) -> Result<Self> {
        let file: CalendarFile =
            serde_json::from_str(text).context("invalid trading calendar JSON")?;

        ensure!(
            file.schema == CALENDAR_SCHEMA,
            "unsupported trading calendar schema: {}",
            file.schema
        );

        let asset_class = AssetClass::from_str(&file.asset_class)
            .map_err(|_| anyhow!("unsupported calendar asset class: {}", file.asset_class))?;
        let time_zone = get_timezone(&file.time_zone)
            .with_context(|| format!("unknown calendar time zone: {}", file.time_zone))?;

        let sessions = parse_sessions(&file.sessions)?;
        ensure!(
            !sessions.is_empty(),
            "trading calendar declares no sessions"
        );

        let holidays = file
            .holidays
            .iter()
            .map(|value| {
                Date::from_str(value).with_context(|| format!("invalid calendar holiday: {value}"))
            })
            .collect::<Result<BTreeSet<_>>>()?;

        let early_closes = file
            .early_closes
            .iter()
            .map(|(date, time)| {
                let date = Date::from_str(date)
                    .with_context(|| format!("invalid early close date: {date}"))?;
                let time = Time::from_str(time)
                    .with_context(|| format!("invalid early close time: {time}"))?;
                Ok((date, time))
            })
            .collect::<Result<BTreeMap<_, _>>>()?;

        let valid_from =
            Date::from_str(&file.valid_from).context("invalid calendar valid_from date")?;
        let valid_until = file
            .valid_until
            .as_deref()
            .map(|value| Date::from_str(value).context("invalid calendar valid_until date"))
            .transpose()?;

        if let Some(valid_until) = valid_until {
            ensure!(
                valid_from <= valid_until,
                "calendar valid_from {valid_from} must not follow valid_until {valid_until}"
            );
        }

        for date in early_closes.keys() {
            ensure!(
                valid_until.is_none_or(|until| *date <= until),
                "calendar early close {date} falls outside the declared coverage"
            );
        }

        Ok(Self {
            key: CalendarKey::new(
                Venue::from(file.venue.as_str()),
                asset_class,
                file.symbol.map(Symbol::from),
            ),
            time_zone,
            time_zone_name: file.time_zone.clone(),
            sessions,
            holidays,
            early_closes,
            valid_from,
            valid_until,
            source: file.source,
        })
    }

    /// Parses and validates a calendar from a JSON file.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read, or if the contents are not a valid calendar.
    pub fn from_json_path(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("cannot read trading calendar: {}", path.display()))?;
        Self::from_json_str(&text)
    }

    /// Returns the calendar key.
    #[must_use]
    pub const fn key(&self) -> &CalendarKey {
        &self.key
    }

    /// Returns the calendar time zone.
    #[must_use]
    pub const fn time_zone(&self) -> &TimeZone {
        &self.time_zone
    }

    /// Returns the declared name of the calendar time zone.
    #[must_use]
    pub fn time_zone_name(&self) -> &str {
        &self.time_zone_name
    }

    /// Returns the first date the calendar data covers.
    #[must_use]
    pub const fn valid_from(&self) -> Date {
        self.valid_from
    }

    /// Returns the last date the calendar data covers, if the data is bounded.
    #[must_use]
    pub const fn valid_until(&self) -> Option<Date> {
        self.valid_until
    }

    /// Returns the optional source of the calendar data, for provenance.
    #[must_use]
    pub fn source(&self) -> Option<&str> {
        self.source.as_deref()
    }

    /// Returns whether the given local date is a declared holiday.
    #[must_use]
    pub fn is_holiday(&self, date: Date) -> bool {
        self.holidays.contains(&date)
    }

    /// Returns the declared early close for the given local date, if any.
    #[must_use]
    pub fn early_close(&self, date: Date) -> Option<Time> {
        self.early_closes.get(&date).copied()
    }

    /// Returns the sessions for the given local date, honouring holidays and early closes.
    #[must_use]
    pub fn sessions_on(&self, date: Date) -> Vec<TradingSession> {
        if self.is_holiday(date) {
            return Vec::new();
        }
        let Some(day_sessions) = self.sessions.get(&date.weekday()) else {
            return Vec::new();
        };
        let early_close = self.early_close(date);

        day_sessions
            .iter()
            .filter_map(|session| {
                let end = early_close.map_or(session.end, |close| close.min(session.end));
                (end > session.start).then_some(TradingSession::new(session.start, end))
            })
            .collect()
    }

    /// Returns whether the given local date has at least one session.
    #[must_use]
    pub fn is_trading_day(&self, date: Date) -> bool {
        !self.sessions_on(date).is_empty()
    }

    /// Returns whether the calendar data covers the given instant.
    #[must_use]
    pub fn covers(&self, ts: Timestamp) -> bool {
        let date = ts.to_zoned(self.time_zone.clone()).date();
        self.valid_from <= date && self.valid_until.is_none_or(|until| date <= until)
    }

    /// Returns whether the given instant falls inside a session.
    #[must_use]
    pub fn is_tradeable(&self, ts: Timestamp) -> bool {
        let zoned = ts.to_zoned(self.time_zone.clone());
        let local_time = zoned.time();
        self.sessions_on(zoned.date())
            .iter()
            .any(|session| session.contains(local_time))
    }

    /// Returns the next session open at or after the given instant.
    #[must_use]
    pub fn next_open(&self, ts: Timestamp) -> Option<Timestamp> {
        self.resolve_boundary(ts, Boundary::Open, true)
    }

    /// Returns the previous session open at or before the given instant.
    #[must_use]
    pub fn prev_open(&self, ts: Timestamp) -> Option<Timestamp> {
        self.resolve_boundary(ts, Boundary::Open, false)
    }

    /// Returns the next session close at or after the given instant.
    #[must_use]
    pub fn next_close(&self, ts: Timestamp) -> Option<Timestamp> {
        self.resolve_boundary(ts, Boundary::Close, true)
    }

    /// Returns the previous session close at or before the given instant.
    #[must_use]
    pub fn prev_close(&self, ts: Timestamp) -> Option<Timestamp> {
        self.resolve_boundary(ts, Boundary::Close, false)
    }

    /// Warns when the calendar coverage ends before the given instant.
    ///
    /// The run is not blocked: the schedule is rule based, so the remaining sessions still resolve,
    /// but the holiday and early close data past the coverage end is absent.
    pub fn warn_if_coverage_ends_before(&self, run_end: Timestamp) {
        let Some(valid_until) = self.valid_until else {
            return;
        };
        let end_date = run_end.to_zoned(self.time_zone.clone()).date();
        if end_date > valid_until {
            warn!(
                "Trading calendar '{}' coverage ends {valid_until}, before the run end {end_date}",
                self.key
            );
        }
    }

    fn resolve_boundary(
        &self,
        ts: Timestamp,
        boundary: Boundary,
        forward: bool,
    ) -> Option<Timestamp> {
        let mut date = ts.to_zoned(self.time_zone.clone()).date();

        for _ in 0..MAX_BOUNDARY_SEARCH_DAYS {
            let mut candidates: Vec<Time> = self
                .sessions_on(date)
                .iter()
                .map(|session| match boundary {
                    Boundary::Open => session.start,
                    Boundary::Close => session.end,
                })
                .collect();

            if !forward {
                candidates.reverse();
            }

            for time in candidates {
                let Some(candidate) = self.local_timestamp(date, time) else {
                    continue;
                };
                if forward && candidate >= ts {
                    return Some(candidate);
                }
                if !forward && candidate <= ts {
                    return Some(candidate);
                }
            }

            let span = Span::new().days(1);
            date = if forward {
                date.checked_add(span).ok()?
            } else {
                date.checked_sub(span).ok()?
            };
        }

        None
    }

    fn local_timestamp(&self, date: Date, time: Time) -> Option<Timestamp> {
        self.time_zone
            .to_ambiguous_timestamp(date.to_datetime(time))
            .compatible()
            .ok()
    }
}

impl FromStr for TradingCalendar {
    type Err = anyhow::Error;

    fn from_str(text: &str) -> Result<Self> {
        Self::from_json_str(text)
    }
}

/// The calendars bundled with the crate, parsed once on first use.
///
/// The four major foreign exchange sessions are bundled under the synthetic venue `FX`, keyed by
/// session name. A venue or instrument class with no bundled calendar resolves no calendar, and the
/// caller supplies one by path instead.
static BUNDLED_CALENDARS: LazyLock<BTreeMap<CalendarKey, TradingCalendar>> = LazyLock::new(|| {
    [
        include_str!("../../resources/calendars/fx-sydney.json"),
        include_str!("../../resources/calendars/fx-tokyo.json"),
        include_str!("../../resources/calendars/fx-london.json"),
        include_str!("../../resources/calendars/fx-new-york.json"),
        include_str!("../../resources/calendars/xnys-equity.json"),
    ]
    .into_iter()
    .map(|text| {
        let calendar =
            TradingCalendar::from_json_str(text).expect("bundled trading calendars must validate");
        (calendar.key().clone(), calendar)
    })
    .collect()
});

/// Returns the bundled calendar for the given key, if the crate bundles one.
#[must_use]
pub fn bundled(key: &CalendarKey) -> Option<&'static TradingCalendar> {
    BUNDLED_CALENDARS.get(key)
}

/// Returns the keys of every bundled calendar.
#[must_use]
pub fn bundled_keys() -> Vec<CalendarKey> {
    BUNDLED_CALENDARS.keys().cloned().collect()
}

fn parse_sessions(
    sessions: &BTreeMap<String, Vec<SessionFile>>,
) -> Result<HashMap<Weekday, Vec<TradingSession>>> {
    let mut parsed: HashMap<Weekday, Vec<TradingSession>> = HashMap::new();

    for (day, entries) in sessions {
        let weekday =
            parse_weekday(day).with_context(|| format!("invalid calendar session day: {day}"))?;
        let mut day_sessions = Vec::with_capacity(entries.len());

        for entry in entries {
            let start = Time::from_str(&entry.start)
                .with_context(|| format!("invalid session start on {day}: {}", entry.start))?;
            let end = Time::from_str(&entry.end)
                .with_context(|| format!("invalid session end on {day}: {}", entry.end))?;
            ensure!(
                start < end,
                "calendar session on {day} must start before it ends: {start}-{end}"
            );
            day_sessions.push(TradingSession::new(start, end));
        }

        day_sessions.sort_by_key(|session| session.start);

        for pair in day_sessions.windows(2) {
            ensure!(
                pair[0].end <= pair[1].start,
                "calendar sessions on {day} overlap: {}-{} and {}-{}",
                pair[0].start,
                pair[0].end,
                pair[1].start,
                pair[1].end
            );
        }

        parsed.insert(weekday, day_sessions);
    }

    Ok(parsed)
}

fn parse_weekday(name: &str) -> Result<Weekday> {
    match name.to_ascii_lowercase().as_str() {
        "monday" => Ok(Weekday::Monday),
        "tuesday" => Ok(Weekday::Tuesday),
        "wednesday" => Ok(Weekday::Wednesday),
        "thursday" => Ok(Weekday::Thursday),
        "friday" => Ok(Weekday::Friday),
        "saturday" => Ok(Weekday::Saturday),
        "sunday" => Ok(Weekday::Sunday),
        other => bail!("unsupported weekday: {other}"),
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CalendarFile {
    schema: String,
    venue: String,
    asset_class: String,
    #[serde(default)]
    symbol: Option<String>,
    time_zone: String,
    #[serde(default)]
    sessions: BTreeMap<String, Vec<SessionFile>>,
    #[serde(default)]
    holidays: Vec<String>,
    #[serde(default)]
    early_closes: BTreeMap<String, String>,
    valid_from: String,
    #[serde(default)]
    valid_until: Option<String>,
    #[serde(default)]
    source: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SessionFile {
    start: String,
    end: String,
}

#[cfg(test)]
mod tests {
    use jiff::civil::date;
    use rstest::rstest;

    use super::*;

    const EQUITY_CALENDAR: &str = r#"
    {
      "schema": "nautilus-trading-calendar/v1",
      "venue": "XNYS",
      "asset_class": "EQUITY",
      "time_zone": "America/New_York",
      "sessions": {
        "monday": [{"start": "09:30:00", "end": "16:00:00"}],
        "tuesday": [{"start": "09:30:00", "end": "16:00:00"}],
        "wednesday": [{"start": "09:30:00", "end": "16:00:00"}],
        "thursday": [{"start": "09:30:00", "end": "16:00:00"}],
        "friday": [{"start": "09:30:00", "end": "16:00:00"}]
      },
      "holidays": ["2024-11-28", "2024-12-25"],
      "early_closes": {"2024-11-29": "13:00:00"},
      "valid_from": "2024-01-01",
      "valid_until": "2024-12-31",
      "source": "test fixture"
    }"#;

    fn calendar() -> TradingCalendar {
        TradingCalendar::from_json_str(EQUITY_CALENDAR).expect("valid test calendar")
    }

    fn utc(year: i16, month: i8, day: i8, hour: i8, minute: i8) -> Timestamp {
        jiff::civil::date(year, month, day)
            .at(hour, minute, 0, 0)
            .to_zoned(TimeZone::UTC)
            .expect("unique UTC timestamp")
            .timestamp()
    }

    #[rstest]
    fn test_calendar_key_and_coverage() {
        let calendar = calendar();

        assert_eq!(
            calendar.key().to_string(),
            "XNYS.EQUITY",
            "unexpected calendar key"
        );
        assert_eq!(calendar.valid_from(), date(2024, 1, 1));
        assert_eq!(calendar.valid_until(), Some(date(2024, 12, 31)));
        assert_eq!(calendar.time_zone_name(), "America/New_York");
        assert_eq!(calendar.source(), Some("test fixture"));
        assert!(calendar.covers(utc(2024, 6, 3, 12, 0)));
        assert!(!calendar.covers(utc(2025, 6, 3, 12, 0)));
    }

    #[rstest]
    fn test_holiday_has_no_sessions() {
        let calendar = calendar();
        let holiday = date(2024, 11, 28);

        assert!(calendar.is_holiday(holiday));
        assert!(!calendar.is_trading_day(holiday));
        assert!(calendar.sessions_on(holiday).is_empty());
        assert!(!calendar.is_tradeable(utc(2024, 11, 28, 15, 0)));
    }

    #[rstest]
    fn test_early_close_shortens_the_session() {
        let calendar = calendar();
        let half_day = date(2024, 11, 29);

        assert_eq!(
            calendar.early_close(half_day),
            Some(Time::constant(13, 0, 0, 0))
        );
        assert_eq!(
            calendar.sessions_on(half_day),
            vec![TradingSession::new(
                Time::constant(9, 30, 0, 0),
                Time::constant(13, 0, 0, 0)
            )]
        );
        // 15:00 UTC is 10:00 in New York, inside the shortened session.
        assert!(calendar.is_tradeable(utc(2024, 11, 29, 15, 0)));
        // 19:00 UTC is 14:00 in New York, after the early close.
        assert!(!calendar.is_tradeable(utc(2024, 11, 29, 19, 0)));
    }

    #[rstest]
    fn test_session_local_hours_are_used() {
        let calendar = calendar();

        // 14:30 UTC is 09:30 in New York on 2024-11-27 (the day before Thanksgiving).
        assert!(calendar.is_tradeable(utc(2024, 11, 27, 14, 30)));
        // One minute before the open.
        assert!(!calendar.is_tradeable(utc(2024, 11, 27, 14, 29)));
        // The close is exclusive: 21:00 UTC is 16:00 in New York.
        assert!(!calendar.is_tradeable(utc(2024, 11, 27, 21, 0)));
    }

    #[rstest]
    fn test_boundaries_skip_weekends_and_holidays() {
        let calendar = calendar();

        // Saturday 2024-11-30 12:00 UTC; the next open is Monday 2024-12-02 09:30 EST.
        let saturday = utc(2024, 11, 30, 12, 0);
        assert_eq!(calendar.next_open(saturday), Some(utc(2024, 12, 2, 14, 30)));
        // The previous close is the shortened Friday session close, 2024-11-29 13:00 EST.
        assert_eq!(
            calendar.prev_close(saturday),
            Some(utc(2024, 11, 29, 18, 0))
        );

        // Thursday 2024-11-28 is Thanksgiving; the next open is Friday 2024-11-29 09:30 EST.
        let thanksgiving = utc(2024, 11, 28, 15, 0);
        assert_eq!(
            calendar.next_open(thanksgiving),
            Some(utc(2024, 11, 29, 14, 30))
        );
        // The previous open is Wednesday 2024-11-27 09:30 EST.
        assert_eq!(
            calendar.prev_open(thanksgiving),
            Some(utc(2024, 11, 27, 14, 30))
        );
    }

    #[rstest]
    fn test_boundary_at_exact_boundary_is_inclusive() {
        let calendar = calendar();
        let open = utc(2024, 11, 27, 14, 30);

        assert_eq!(calendar.next_open(open), Some(open));
        assert_eq!(calendar.prev_open(open), Some(open));
    }

    #[rstest]
    fn test_requires_start_before_end() {
        let invalid = EQUITY_CALENDAR.replace(
            "\"09:30:00\", \"end\": \"16:00:00\"",
            "\"16:00:00\", \"end\": \"09:30:00\"",
        );

        let error =
            TradingCalendar::from_json_str(&invalid).expect_err("invalid session must fail");

        assert!(
            error.to_string().contains("must start before it ends"),
            "{error}"
        );
    }

    #[rstest]
    fn test_rejects_overlapping_sessions() {
        let invalid = EQUITY_CALENDAR.replace(
            "\"monday\": [{\"start\": \"09:30:00\", \"end\": \"16:00:00\"}]",
            "\"monday\": [{\"start\": \"09:30:00\", \"end\": \"16:00:00\"}, {\"start\": \"12:00:00\", \"end\": \"17:00:00\"}]",
        );

        let error = TradingCalendar::from_json_str(&invalid).expect_err("overlap must fail");

        assert!(error.to_string().contains("overlap"), "{error}");
    }

    #[rstest]
    fn test_rejects_unsupported_schema() {
        let invalid = EQUITY_CALENDAR.replace(CALENDAR_SCHEMA, "nautilus-trading-calendar/v9");

        let error = TradingCalendar::from_json_str(&invalid).expect_err("schema must be checked");

        assert!(
            error
                .to_string()
                .contains("unsupported trading calendar schema"),
            "{error}"
        );
    }

    #[rstest]
    fn test_rejects_unknown_weekday() {
        let invalid = EQUITY_CALENDAR.replace("\"friday\":", "\"fryday\":");

        let error = TradingCalendar::from_json_str(&invalid).expect_err("weekday must be checked");

        assert!(
            format!("{error:#}").contains("unsupported weekday"),
            "{error:#}"
        );
    }

    #[rstest]
    fn test_warns_when_coverage_ends_before_the_run() {
        let calendar = calendar();

        // No panic and no early return; the call is observable through the logger only.
        calendar.warn_if_coverage_ends_before(utc(2025, 3, 3, 12, 0));
        calendar.warn_if_coverage_ends_before(utc(2024, 6, 3, 12, 0));
    }

    #[rstest]
    fn test_bundled_fx_calendars_resolve_the_documented_sessions() {
        let london = bundled(&CalendarKey::new(
            Venue::from("FX"),
            AssetClass::FX,
            Some(Symbol::from("LONDON")),
        ))
        .expect("bundled London calendar");

        assert_eq!(london.key().to_string(), "FX.FX.LONDON");
        assert_eq!(
            london.sessions_on(date(2024, 6, 3)),
            vec![TradingSession::new(
                Time::constant(8, 0, 0, 0),
                Time::constant(16, 0, 0, 0)
            )]
        );
        // 2024-06-03 07:00 UTC is 08:00 in London during British Summer Time.
        assert!(london.is_tradeable(utc(2024, 6, 3, 7, 0)));

        // Saturday 2024-06-08 resolves forward to Monday and back to Friday.
        let saturday = utc(2024, 6, 8, 12, 0);
        assert_eq!(london.next_open(saturday), Some(utc(2024, 6, 10, 7, 0)));
        assert_eq!(london.prev_close(saturday), Some(utc(2024, 6, 7, 15, 0)));

        for session in ["SYDNEY", "TOKYO", "LONDON", "NEW_YORK"] {
            let key = CalendarKey::new(
                Venue::from("FX"),
                AssetClass::FX,
                Some(Symbol::from(session)),
            );
            assert!(
                bundled(&key).is_some(),
                "missing bundled calendar for {session}"
            );
        }

        let fx_calendars = bundled_keys()
            .iter()
            .filter(|key| key.venue == Venue::from("FX"))
            .count();
        assert_eq!(fx_calendars, 4, "unexpected number of bundled FX calendars");
    }

    #[rstest]
    fn test_bundled_equity_calendar_resolves_a_holiday_and_a_half_day() {
        let calendar = bundled(&CalendarKey::new(
            Venue::from("XNYS"),
            AssetClass::Equity,
            None,
        ))
        .expect("bundled XNYS calendar");

        assert_eq!(calendar.key().to_string(), "XNYS.EQUITY");
        assert_eq!(calendar.valid_from(), date(2024, 1, 1));
        assert_eq!(calendar.valid_until(), Some(date(2025, 12, 31)));

        // Thanksgiving 2024 and Christmas Day 2025 are closures.
        assert!(calendar.is_holiday(date(2024, 11, 28)));
        assert!(calendar.is_holiday(date(2025, 12, 25)));
        // 2025-01-09 was a one-off national day of mourning, not a recurring holiday rule.
        assert!(calendar.is_holiday(date(2025, 1, 9)));

        // 2024-11-29 closed early at 13:00 local time.
        assert_eq!(
            calendar.sessions_on(date(2024, 11, 29)),
            vec![TradingSession::new(
                Time::constant(9, 30, 0, 0),
                Time::constant(13, 0, 0, 0)
            )]
        );
        assert_eq!(calendar.sessions_on(date(2024, 11, 27)).len(), 1);
        assert_eq!(calendar.time_zone_name(), "America/New_York");
    }
}
