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

//! Provides utilities for determining foreign exchange session times.
//!
//! The session schedules come from the bundled trading calendars in
//! [`nautilus_model::calendars`], so the session times, time zones, and weekday rules have a single
//! definition which can be overridden by supplying a calendar rather than by editing this module.
//!
//! All FX sessions run Monday to Friday local time:
//!
//! - Sydney Session    0700-1600 (Australia / Sydney)
//! - Tokyo Session     0900-1800 (Asia / Tokyo)
//! - London Session    0800-1600 (Europe / London)
//! - New York Session  0800-1700 (America / New York)

use jiff::{Timestamp, Zoned};
use nautilus_model::{
    calendars::{CalendarKey, TradingCalendar, bundled},
    enums::AssetClass,
    identifiers::{Symbol, Venue},
};
use strum::{Display, EnumIter, EnumString, FromRepr};

/// Represents a major Forex market session based on trading hours.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, FromRepr, EnumIter, EnumString, Display)]
#[strum(ascii_case_insensitive)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(
        eq,
        eq_int,
        module = "nautilus_trader.trading",
        from_py_object,
        rename_all = "SCREAMING_SNAKE_CASE"
    )
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass_enum(module = "nautilus_trader.trading")
)]
pub enum ForexSession {
    Sydney,
    Tokyo,
    London,
    NewYork,
}

impl ForexSession {
    /// Returns the bundled calendar for this session.
    fn calendar(self) -> &'static TradingCalendar {
        let key = CalendarKey::new(
            Venue::from("FX"),
            AssetClass::FX,
            Some(Symbol::from(self.calendar_symbol())),
        );

        bundled(&key).expect("bundled FX session calendars must be present")
    }

    /// Returns the session symbol used in the bundled calendar key.
    const fn calendar_symbol(self) -> &'static str {
        match self {
            Self::Sydney => "SYDNEY",
            Self::Tokyo => "TOKYO",
            Self::London => "LONDON",
            Self::NewYork => "NEW_YORK",
        }
    }
}

/// Converts a UTC timestamp to the local time for the given Forex session.
///
/// # Panics
///
/// Panics if the bundled FX session calendars are unavailable.
#[must_use]
pub fn fx_local_from_utc(session: ForexSession, time_now: Timestamp) -> Zoned {
    time_now.to_zoned(session.calendar().time_zone().clone())
}

/// Returns the next session start time in UTC.
///
/// # Panics
///
/// Panics if the bundled FX session calendars are unavailable, or the schedule resolves no
/// boundary. Both indicate missing or invalid bundled calendar data rather than a caller error.
#[must_use]
pub fn fx_next_start(session: ForexSession, time_now: Timestamp) -> Timestamp {
    session
        .calendar()
        .next_open(time_now)
        .expect("FX session must have a next open")
}

/// Returns the previous session start time in UTC.
///
/// # Panics
///
/// Panics if the bundled FX session calendars are unavailable, or the schedule resolves no
/// boundary. Both indicate missing or invalid bundled calendar data rather than a caller error.
#[must_use]
pub fn fx_prev_start(session: ForexSession, time_now: Timestamp) -> Timestamp {
    session
        .calendar()
        .prev_open(time_now)
        .expect("FX session must have a previous open")
}

/// Returns the next session end time in UTC.
///
/// # Panics
///
/// Panics if the bundled FX session calendars are unavailable, or the schedule resolves no
/// boundary. Both indicate missing or invalid bundled calendar data rather than a caller error.
#[must_use]
pub fn fx_next_end(session: ForexSession, time_now: Timestamp) -> Timestamp {
    session
        .calendar()
        .next_close(time_now)
        .expect("FX session must have a next close")
}

/// Returns the previous session end time in UTC.
///
/// # Panics
///
/// Panics if the bundled FX session calendars are unavailable, or the schedule resolves no
/// boundary. Both indicate missing or invalid bundled calendar data rather than a caller error.
#[must_use]
pub fn fx_prev_end(session: ForexSession, time_now: Timestamp) -> Timestamp {
    session
        .calendar()
        .prev_close(time_now)
        .expect("FX session must have a previous close")
}

#[cfg(test)]
mod tests {
    use jiff::{
        Span,
        civil::{Date, Time, Weekday},
        tz::Offset,
    };
    use rstest::rstest;

    use super::*;

    fn local_timestamp(
        session: ForexSession,
        year: i32,
        month: u32,
        day: u32,
        hour: u32,
    ) -> Timestamp {
        let date = Date::new(
            i16::try_from(year).unwrap(),
            i8::try_from(month).unwrap(),
            i8::try_from(day).unwrap(),
        )
        .unwrap();
        let datetime = date.at(i8::try_from(hour).unwrap(), 0, 0, 0);

        session
            .calendar()
            .time_zone()
            .to_ambiguous_timestamp(datetime)
            .unambiguous()
            .unwrap()
    }

    fn utc_timestamp(year: i32, month: i8, day: i8, hour: i8, minute: i8) -> Timestamp {
        Offset::UTC
            .to_timestamp(
                Date::new(i16::try_from(year).unwrap(), month, day)
                    .unwrap()
                    .at(hour, minute, 0, 0),
            )
            .unwrap()
    }

    /// Returns the session local times of the pre-calendar implementation.
    const fn legacy_session_times(session: ForexSession) -> (Time, Time) {
        match session {
            ForexSession::Sydney => (Time::constant(7, 0, 0, 0), Time::constant(16, 0, 0, 0)),
            ForexSession::Tokyo => (Time::constant(9, 0, 0, 0), Time::constant(18, 0, 0, 0)),
            ForexSession::London => (Time::constant(8, 0, 0, 0), Time::constant(16, 0, 0, 0)),
            ForexSession::NewYork => (Time::constant(8, 0, 0, 0), Time::constant(17, 0, 0, 0)),
        }
    }

    /// The forward weekday walk of the pre-calendar implementation.
    ///
    /// Retained as the migration oracle for `fx_next_start` and `fx_next_end`.
    fn legacy_next_boundary(local_now: &Zoned, session_time: Time) -> Timestamp {
        let timezone = local_now.time_zone().clone();
        let mut date = local_now.date();

        if local_now.time() > session_time {
            date = date.checked_add(Span::new().days(1)).unwrap();
        }

        let weekend_days = match date.weekday() {
            Weekday::Saturday => 2,
            Weekday::Sunday => 1,
            _ => 0,
        };
        date = date.checked_add(Span::new().days(weekend_days)).unwrap();

        timezone
            .to_ambiguous_timestamp(date.to_datetime(session_time))
            .unambiguous()
            .unwrap()
    }

    /// The backward weekday walk of the pre-calendar implementation.
    ///
    /// Retained as the migration oracle for `fx_prev_start` and `fx_prev_end`.
    fn legacy_prev_boundary(local_now: &Zoned, session_time: Time) -> Timestamp {
        let timezone = local_now.time_zone().clone();
        let mut date = local_now.date();

        if local_now.time() < session_time {
            date = date.checked_sub(Span::new().days(1)).unwrap();
        }

        let weekend_days = match date.weekday() {
            Weekday::Saturday => 1,
            Weekday::Sunday => 2,
            _ => 0,
        };
        date = date.checked_sub(Span::new().days(weekend_days)).unwrap();

        timezone
            .to_ambiguous_timestamp(date.to_datetime(session_time))
            .unambiguous()
            .unwrap()
    }

    #[rstest]
    #[case(ForexSession::Sydney, "1970-01-01T10:00:00+10:00")]
    #[case(ForexSession::Tokyo, "1970-01-01T09:00:00+09:00")]
    #[case(ForexSession::London, "1970-01-01T01:00:00+01:00")]
    #[case(ForexSession::NewYork, "1969-12-31T19:00:00-05:00")]
    pub fn test_fx_local_from_utc(#[case] session: ForexSession, #[case] expected: &str) {
        let unix_epoch = Timestamp::UNIX_EPOCH;
        let result = fx_local_from_utc(session, unix_epoch);
        assert_eq!(
            result.strftime("%Y-%m-%dT%H:%M:%S%:z").to_string(),
            expected
        );
    }

    #[rstest]
    #[case(ForexSession::Sydney, "1970-01-01T21:00:00+00:00")]
    #[case(ForexSession::Tokyo, "1970-01-01T00:00:00+00:00")]
    #[case(ForexSession::London, "1970-01-01T07:00:00+00:00")]
    #[case(ForexSession::NewYork, "1970-01-01T13:00:00+00:00")]
    pub fn test_fx_next_start(#[case] session: ForexSession, #[case] expected: &str) {
        let unix_epoch = Timestamp::UNIX_EPOCH;
        let result = fx_next_start(session, unix_epoch);
        assert_eq!(result, expected.parse::<Timestamp>().unwrap());
    }

    #[rstest]
    #[case(ForexSession::Sydney, "1969-12-31T21:00:00+00:00")]
    #[case(ForexSession::Tokyo, "1970-01-01T00:00:00+00:00")]
    #[case(ForexSession::London, "1969-12-31T07:00:00+00:00")]
    #[case(ForexSession::NewYork, "1969-12-31T13:00:00+00:00")]
    pub fn test_fx_prev_start(#[case] session: ForexSession, #[case] expected: &str) {
        let unix_epoch = Timestamp::UNIX_EPOCH;
        let result = fx_prev_start(session, unix_epoch);
        assert_eq!(result, expected.parse::<Timestamp>().unwrap());
    }

    #[rstest]
    #[case(ForexSession::Sydney, "1970-01-01T06:00:00+00:00")]
    #[case(ForexSession::Tokyo, "1970-01-01T09:00:00+00:00")]
    #[case(ForexSession::London, "1970-01-01T15:00:00+00:00")]
    #[case(ForexSession::NewYork, "1970-01-01T22:00:00+00:00")]
    pub fn test_fx_next_end(#[case] session: ForexSession, #[case] expected: &str) {
        let unix_epoch = Timestamp::UNIX_EPOCH;
        let result = fx_next_end(session, unix_epoch);
        assert_eq!(result, expected.parse::<Timestamp>().unwrap());
    }

    #[rstest]
    #[case(ForexSession::Sydney, "1969-12-31T06:00:00+00:00")]
    #[case(ForexSession::Tokyo, "1969-12-31T09:00:00+00:00")]
    #[case(ForexSession::London, "1969-12-31T15:00:00+00:00")]
    #[case(ForexSession::NewYork, "1969-12-31T22:00:00+00:00")]
    pub fn test_fx_prev_end(#[case] session: ForexSession, #[case] expected: &str) {
        let unix_epoch = Timestamp::UNIX_EPOCH;
        let result = fx_prev_end(session, unix_epoch);
        assert_eq!(result, expected.parse::<Timestamp>().unwrap());
    }

    #[rstest]
    #[case(ForexSession::Sydney, (2024, 4, 5), (2024, 4, 8), 7)]
    #[case(ForexSession::Sydney, (2024, 10, 4), (2024, 10, 7), 7)]
    #[case(ForexSession::London, (2024, 3, 29), (2024, 4, 1), 8)]
    #[case(ForexSession::London, (2024, 10, 25), (2024, 10, 28), 8)]
    #[case(ForexSession::NewYork, (2024, 3, 8), (2024, 3, 11), 8)]
    #[case(ForexSession::NewYork, (2024, 11, 1), (2024, 11, 4), 8)]
    // Saturday input: advances to Sunday, then takes the Sunday weekend arm.
    #[case(ForexSession::London, (2024, 3, 30), (2024, 4, 1), 8)]
    fn test_fx_next_start_across_dst_weekend(
        #[case] session: ForexSession,
        #[case] input_date: (i32, u32, u32),
        #[case] expected_date: (i32, u32, u32),
        #[case] expected_hour: u32,
    ) {
        let (input_year, input_month, input_day) = input_date;
        let (expected_year, expected_month, expected_day) = expected_date;
        let time_now = local_timestamp(session, input_year, input_month, input_day, 18);
        let expected = local_timestamp(
            session,
            expected_year,
            expected_month,
            expected_day,
            expected_hour,
        );

        assert_eq!(fx_next_start(session, time_now), expected);
    }

    #[rstest]
    #[case(ForexSession::Sydney, (2024, 4, 8), (2024, 4, 5), 7)]
    #[case(ForexSession::Sydney, (2024, 10, 7), (2024, 10, 4), 7)]
    #[case(ForexSession::London, (2024, 4, 1), (2024, 3, 29), 8)]
    #[case(ForexSession::London, (2024, 10, 28), (2024, 10, 25), 8)]
    #[case(ForexSession::NewYork, (2024, 3, 11), (2024, 3, 8), 8)]
    #[case(ForexSession::NewYork, (2024, 11, 4), (2024, 11, 1), 8)]
    // Sunday input: retreats to Saturday, then takes the Saturday weekend arm.
    #[case(ForexSession::London, (2024, 3, 31), (2024, 3, 29), 8)]
    fn test_fx_prev_start_across_dst_weekend(
        #[case] session: ForexSession,
        #[case] input_date: (i32, u32, u32),
        #[case] expected_date: (i32, u32, u32),
        #[case] expected_hour: u32,
    ) {
        let (input_year, input_month, input_day) = input_date;
        let (expected_year, expected_month, expected_day) = expected_date;
        let time_now = local_timestamp(session, input_year, input_month, input_day, 6);
        let expected = local_timestamp(
            session,
            expected_year,
            expected_month,
            expected_day,
            expected_hour,
        );

        assert_eq!(fx_prev_start(session, time_now), expected);
    }

    #[rstest]
    #[case(ForexSession::Sydney, (2024, 4, 5), (2024, 4, 8), 16)]
    #[case(ForexSession::Sydney, (2024, 10, 4), (2024, 10, 7), 16)]
    #[case(ForexSession::London, (2024, 3, 29), (2024, 4, 1), 16)]
    #[case(ForexSession::London, (2024, 10, 25), (2024, 10, 28), 16)]
    #[case(ForexSession::NewYork, (2024, 3, 8), (2024, 3, 11), 17)]
    #[case(ForexSession::NewYork, (2024, 11, 1), (2024, 11, 4), 17)]
    fn test_fx_next_end_across_dst_weekend(
        #[case] session: ForexSession,
        #[case] input_date: (i32, u32, u32),
        #[case] expected_date: (i32, u32, u32),
        #[case] expected_hour: u32,
    ) {
        let (input_year, input_month, input_day) = input_date;
        let (expected_year, expected_month, expected_day) = expected_date;
        let time_now = local_timestamp(session, input_year, input_month, input_day, 18);
        let expected = local_timestamp(
            session,
            expected_year,
            expected_month,
            expected_day,
            expected_hour,
        );

        assert_eq!(fx_next_end(session, time_now), expected);
    }

    #[rstest]
    #[case(ForexSession::Sydney, (2024, 4, 8), (2024, 4, 5), 16)]
    #[case(ForexSession::Sydney, (2024, 10, 7), (2024, 10, 4), 16)]
    #[case(ForexSession::London, (2024, 4, 1), (2024, 3, 29), 16)]
    #[case(ForexSession::London, (2024, 10, 28), (2024, 10, 25), 16)]
    #[case(ForexSession::NewYork, (2024, 3, 11), (2024, 3, 8), 17)]
    #[case(ForexSession::NewYork, (2024, 11, 4), (2024, 11, 1), 17)]
    fn test_fx_prev_end_across_dst_weekend(
        #[case] session: ForexSession,
        #[case] input_date: (i32, u32, u32),
        #[case] expected_date: (i32, u32, u32),
        #[case] expected_hour: u32,
    ) {
        let (input_year, input_month, input_day) = input_date;
        let (expected_year, expected_month, expected_day) = expected_date;
        let time_now = local_timestamp(session, input_year, input_month, input_day, 6);
        let expected = local_timestamp(
            session,
            expected_year,
            expected_month,
            expected_day,
            expected_hour,
        );

        assert_eq!(fx_prev_end(session, time_now), expected);
    }

    #[rstest]
    pub fn test_fx_next_start_on_weekend() {
        let sunday_utc = utc_timestamp(2020, 7, 12, 9, 0); // Sunday
        let result = fx_next_start(ForexSession::Tokyo, sunday_utc);
        let expected = utc_timestamp(2020, 7, 13, 0, 0); // Monday

        assert_eq!(result, expected);
    }

    #[rstest]
    pub fn test_fx_next_start_during_active_session() {
        let during_session = utc_timestamp(2020, 7, 13, 10, 0); // Sydney session is active
        let result = fx_next_start(ForexSession::Sydney, during_session);
        let expected = utc_timestamp(2020, 7, 13, 21, 0); // Next Sydney session start

        assert_eq!(result, expected);
    }

    #[rstest]
    pub fn test_fx_prev_start_before_session() {
        let before_session = utc_timestamp(2020, 7, 13, 6, 0); // Before Tokyo session start
        let result = fx_prev_start(ForexSession::Tokyo, before_session);
        let expected = utc_timestamp(2020, 7, 13, 0, 0); // Current Tokyo session start

        assert_eq!(result, expected);
    }

    #[rstest]
    pub fn test_fx_next_end_crossing_midnight() {
        let late_night = utc_timestamp(2020, 7, 13, 23, 0); // After NY session ended
        let result = fx_next_end(ForexSession::NewYork, late_night);
        let expected = utc_timestamp(2020, 7, 14, 21, 0); // Next NY session end

        assert_eq!(result, expected);
    }

    #[rstest]
    pub fn test_fx_prev_end_after_session() {
        let after_session = utc_timestamp(2020, 7, 13, 17, 30); // Just after NY session ended
        let result = fx_prev_end(ForexSession::NewYork, after_session);
        let expected = utc_timestamp(2020, 7, 10, 21, 0); // Previous NY session end

        assert_eq!(result, expected);
    }

    #[rstest]
    fn test_fx_boundaries_match_the_pre_calendar_implementation() {
        let sessions = [
            ForexSession::Sydney,
            ForexSession::Tokyo,
            ForexSession::London,
            ForexSession::NewYork,
        ];
        let start = utc_timestamp(2023, 1, 1, 0, 0);
        let end = utc_timestamp(2026, 1, 1, 0, 0);
        let mut checked = 0_u32;

        for session in sessions {
            let (start_time, end_time) = legacy_session_times(session);
            let mut ts = start;

            while ts < end {
                let local = fx_local_from_utc(session, ts);

                assert_eq!(
                    fx_next_start(session, ts),
                    legacy_next_boundary(&local, start_time),
                    "next start at {ts} for {session:?}"
                );
                assert_eq!(
                    fx_prev_start(session, ts),
                    legacy_prev_boundary(&local, start_time),
                    "previous start at {ts} for {session:?}"
                );
                assert_eq!(
                    fx_next_end(session, ts),
                    legacy_next_boundary(&local, end_time),
                    "next end at {ts} for {session:?}"
                );
                assert_eq!(
                    fx_prev_end(session, ts),
                    legacy_prev_boundary(&local, end_time),
                    "previous end at {ts} for {session:?}"
                );

                ts = ts.checked_add(Span::new().hours(12)).unwrap();
                checked += 1;
            }
        }

        assert!(
            checked > 8_000,
            "expected a wide spread of instants, checked {checked}"
        );
    }
}
