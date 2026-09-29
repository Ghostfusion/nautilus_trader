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

//! Session events derived from a [`TradingCalendar`].
//!
//! A session event is anchored to the market calendar, not to an interval. It is expanded to an
//! absolute UTC instant at the time it is declared, so a run never reads a wall clock to decide
//! when a session phase occurs.
//!
//! Events are pure derivations of calendar data and a [`SessionScheduleConfig`]. The expansion is
//! deterministic: the same calendar, config, and window always produce the same events in the same
//! order.

use std::{fmt::Display, time::Duration};

use anyhow::{Result, ensure};
use jiff::{Span, Timestamp, civil::Date};

use super::{CalendarKey, TradingCalendar};

/// The longest offset a [`SessionScheduleConfig`] accepts for a derived phase.
///
/// An offset longer than a day cannot describe a phase of a trading session, and rejecting it keeps
/// the derived instants inside the range a timestamp can represent.
const MAX_PHASE_OFFSET: Duration = Duration::from_hours(24);

/// A phase of a trading session, or a calendar condition that replaces one.
///
/// The kinds are the session phases an intraday strategy distinguishes. A clock timer is an
/// interval; a session event is a market-anchored instant.
#[derive(
    Clone,
    Copy,
    Debug,
    strum::Display,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    strum::AsRefStr,
    strum::FromRepr,
    strum::EnumIter,
    strum::EnumString,
)]
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
pub enum SessionEventKind {
    /// The configured interval before the session open.
    Premarket = 1,
    /// The session open.
    Open = 2,
    /// The configured interval after the session open.
    OpeningRangeComplete = 3,
    /// The midpoint between the session open and close.
    Midday = 4,
    /// The configured interval before the session close.
    PreClose = 5,
    /// The session close on a full trading day.
    Close = 6,
    /// The session close on a day the calendar declares an early close.
    EarlyClose = 7,
}

impl SessionEventKind {
    /// Returns the canonical string representation.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.as_ref()
    }
}

/// A market-anchored event on a trading calendar.
///
/// `ts_event` is the instant the phase occurs, in UTC. `ts_init` equals `ts_event`: an event is a
/// pure derivation of calendar data, so expanding a schedule never reads the current time.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.model", from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.model")
)]
pub struct SessionEvent {
    /// The session phase.
    pub kind: SessionEventKind,
    /// The calendar the event was derived from.
    pub key: CalendarKey,
    /// The exchange-local session date.
    pub session_date: Date,
    /// The index of the session on that date.
    pub session_index: usize,
    /// The instant the phase occurs (UTC).
    pub ts_event: Timestamp,
    /// The instant the event was initialized (UTC).
    pub ts_init: Timestamp,
}

impl SessionEvent {
    /// Creates a new [`SessionEvent`].
    #[must_use]
    pub const fn new(
        kind: SessionEventKind,
        key: CalendarKey,
        session_date: Date,
        session_index: usize,
        ts_event: Timestamp,
        ts_init: Timestamp,
    ) -> Self {
        Self {
            kind,
            key,
            session_date,
            session_index,
            ts_event,
            ts_init,
        }
    }

    /// Returns the deterministic timer name for this event.
    ///
    /// The name identifies the event and nothing else, so scheduling the same event twice replaces
    /// the existing timer instead of duplicating it.
    #[must_use]
    pub fn name(&self) -> String {
        format!(
            "SESSION-{}:{}:{}:{}",
            self.kind, self.key, self.session_date, self.session_index
        )
    }
}

impl Display for SessionEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// Configures which session phases are derived, and how far from a session boundary they fall.
///
/// Offsets are absolute elapsed time, not civil clock time: a pre-close offset of 30 minutes is 30
/// minutes of real time before the close, which is what an intraday strategy needs when reacting to
/// the close. Nothing here reads a clock.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.model", from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.model")
)]
pub struct SessionScheduleConfig {
    /// The interval before the session open reported as `Premarket`.
    pub premarket_offset: Duration,
    /// The interval after the session open reported as `OpeningRangeComplete`.
    pub opening_range: Duration,
    /// The interval before the session close reported as `PreClose`.
    pub pre_close_offset: Duration,
    /// The phases to derive. An empty list derives no events.
    pub kinds: Vec<SessionEventKind>,
}

impl SessionScheduleConfig {
    /// Creates a new [`SessionScheduleConfig`] deriving every phase.
    ///
    /// # Errors
    ///
    /// Returns an error if an offset is longer than one day.
    pub fn new(
        premarket_offset: Duration,
        opening_range: Duration,
        pre_close_offset: Duration,
    ) -> Result<Self> {
        ensure!(
            premarket_offset <= MAX_PHASE_OFFSET,
            "premarket offset {premarket_offset:?} must not exceed one day"
        );
        ensure!(
            opening_range <= MAX_PHASE_OFFSET,
            "opening range {opening_range:?} must not exceed one day"
        );
        ensure!(
            pre_close_offset <= MAX_PHASE_OFFSET,
            "pre-close offset {pre_close_offset:?} must not exceed one day"
        );

        Ok(Self {
            premarket_offset,
            opening_range,
            pre_close_offset,
            kinds: SessionEventKind::all().to_vec(),
        })
    }

    /// Returns a copy of this configuration deriving only the given phases.
    #[must_use]
    pub fn with_kinds(mut self, kinds: Vec<SessionEventKind>) -> Self {
        self.kinds = kinds;
        self
    }

    /// Returns whether the given phase is derived.
    #[must_use]
    pub fn includes(&self, kind: SessionEventKind) -> bool {
        self.kinds.contains(&kind)
    }
}

impl SessionEventKind {
    /// Returns every session event kind, in derivation order.
    #[must_use]
    pub const fn all() -> [Self; 7] {
        [
            Self::Premarket,
            Self::Open,
            Self::OpeningRangeComplete,
            Self::Midday,
            Self::PreClose,
            Self::Close,
            Self::EarlyClose,
        ]
    }
}

impl TradingCalendar {
    /// Expands the calendar into the session events that occur in `[from, to)`.
    ///
    /// The window is half open, so a phase exactly at `to` belongs to the next expansion and
    /// consecutive windows neither duplicate nor drop a phase. Events are ordered by instant, then
    /// by kind.
    ///
    /// A date with no session, including a holiday and a weekend, derives no events. On a date the
    /// calendar declares an early close, the shortened session reports `EarlyClose` instead of
    /// `Close`, and every phase derived from the close moves with it.
    ///
    /// A derived instant that cannot be represented is omitted rather than saturating: a phase is a
    /// derivation of calendar data, so an impossible instant is absent, not approximated.
    #[must_use]
    pub fn session_events(
        &self,
        from: Timestamp,
        to: Timestamp,
        config: &SessionScheduleConfig,
    ) -> Vec<SessionEvent> {
        let mut events = Vec::new();

        if from >= to {
            return events;
        }

        let mut date = from.to_zoned(self.time_zone.clone()).date();
        let last_date = to.to_zoned(self.time_zone.clone()).date();

        while date <= last_date {
            let early_close = self.early_close(date);

            for (index, session) in self.sessions_on(date).into_iter().enumerate() {
                let Some(open) = self.local_timestamp(date, session.start) else {
                    continue;
                };
                let Some(close) = self.local_timestamp(date, session.end) else {
                    continue;
                };
                let is_early_close = early_close == Some(session.end);

                let mut derived = Vec::with_capacity(6);

                if config.includes(SessionEventKind::Premarket) {
                    derived.push((
                        SessionEventKind::Premarket,
                        shift_back(open, config.premarket_offset),
                    ));
                }
                if config.includes(SessionEventKind::Open) {
                    derived.push((SessionEventKind::Open, Some(open)));
                }
                if config.includes(SessionEventKind::OpeningRangeComplete) {
                    let instant =
                        shift_forward(open, config.opening_range).filter(|ts| *ts < close);
                    derived.push((SessionEventKind::OpeningRangeComplete, instant));
                }
                if config.includes(SessionEventKind::Midday) {
                    derived.push((SessionEventKind::Midday, midpoint(open, close)));
                }
                if config.includes(SessionEventKind::PreClose) {
                    derived.push((
                        SessionEventKind::PreClose,
                        shift_back(close, config.pre_close_offset),
                    ));
                }
                if is_early_close {
                    if config.includes(SessionEventKind::EarlyClose) {
                        derived.push((SessionEventKind::EarlyClose, Some(close)));
                    }
                } else if config.includes(SessionEventKind::Close) {
                    derived.push((SessionEventKind::Close, Some(close)));
                }

                for (kind, instant) in derived {
                    let Some(ts_event) = instant else {
                        continue;
                    };
                    if ts_event < from || ts_event >= to {
                        continue;
                    }
                    events.push(SessionEvent::new(
                        kind,
                        self.key().clone(),
                        date,
                        index,
                        ts_event,
                        ts_event,
                    ));
                }
            }

            match date.checked_add(Span::new().days(1)) {
                Ok(next) => date = next,
                Err(_) => break,
            }
        }

        events.sort_by(|left, right| {
            left.ts_event
                .cmp(&right.ts_event)
                .then_with(|| left.kind.cmp(&right.kind))
                .then_with(|| left.session_index.cmp(&right.session_index))
        });
        events
    }
}

/// Returns an instant `offset` before `ts`, or `None` when it is not representable.
fn shift_back(ts: Timestamp, offset: Duration) -> Option<Timestamp> {
    let delta = i128::try_from(offset.as_nanos()).ok()?;
    to_timestamp(ts.as_nanosecond().checked_sub(delta)?)
}

/// Returns an instant `offset` after `ts`, or `None` when it is not representable.
fn shift_forward(ts: Timestamp, offset: Duration) -> Option<Timestamp> {
    let delta = i128::try_from(offset.as_nanos()).ok()?;
    to_timestamp(ts.as_nanosecond().checked_add(delta)?)
}

/// Returns the instant halfway between two instants.
fn midpoint(open: Timestamp, close: Timestamp) -> Option<Timestamp> {
    let open_ns = open.as_nanosecond();
    let span = close.as_nanosecond().checked_sub(open_ns)?;
    if span < 0 {
        return None;
    }
    to_timestamp(open_ns.checked_add(span / 2)?)
}

/// Returns a timestamp for a nanosecond value, or `None` when it is not representable.
fn to_timestamp(nanos: i128) -> Option<Timestamp> {
    Timestamp::from_nanosecond(nanos).ok()
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use rstest::{fixture, rstest};

    use super::*;
    use crate::enums::AssetClass;
    use crate::identifiers::Venue;

    #[fixture]
    fn calendar() -> &'static TradingCalendar {
        super::super::bundled(&CalendarKey::new(
            Venue::from("XNYS"),
            AssetClass::Equity,
            None,
        ))
        .expect("XNYS calendar is bundled")
    }

    #[fixture]
    fn config() -> SessionScheduleConfig {
        SessionScheduleConfig::new(
            Duration::from_hours(1),
            Duration::from_mins(30),
            Duration::from_mins(30),
        )
        .unwrap()
    }

    fn ts(value: &str) -> Timestamp {
        value.parse().unwrap()
    }

    #[rstest]
    fn test_full_day_derives_every_phase(
        calendar: &'static TradingCalendar,
        config: SessionScheduleConfig,
    ) {
        let events = calendar.session_events(
            ts("2024-06-03T00:00:00Z"),
            ts("2024-06-04T00:00:00Z"),
            &config,
        );

        let kinds: Vec<SessionEventKind> = events.iter().map(|event| event.kind).collect();
        assert_eq!(
            kinds,
            vec![
                SessionEventKind::Premarket,
                SessionEventKind::Open,
                SessionEventKind::OpeningRangeComplete,
                SessionEventKind::Midday,
                SessionEventKind::PreClose,
                SessionEventKind::Close,
            ]
        );

        // 2024-06-03 is daylight time: 09:30-16:00 local is 13:30-20:00 UTC.
        assert_eq!(events[0].ts_event, ts("2024-06-03T12:30:00Z"));
        assert_eq!(events[1].ts_event, ts("2024-06-03T13:30:00Z"));
        assert_eq!(events[2].ts_event, ts("2024-06-03T14:00:00Z"));
        assert_eq!(events[3].ts_event, ts("2024-06-03T16:45:00Z"));
        assert_eq!(events[4].ts_event, ts("2024-06-03T19:30:00Z"));
        assert_eq!(events[5].ts_event, ts("2024-06-03T20:00:00Z"));
    }

    #[rstest]
    fn test_holiday_derives_no_events(
        calendar: &'static TradingCalendar,
        config: SessionScheduleConfig,
    ) {
        // Thanksgiving 2024.
        let events = calendar.session_events(
            ts("2024-11-28T00:00:00Z"),
            ts("2024-11-29T00:00:00Z"),
            &config,
        );

        assert!(events.is_empty());
    }

    #[rstest]
    fn test_early_close_shifts_dependent_phases(
        calendar: &'static TradingCalendar,
        config: SessionScheduleConfig,
    ) {
        // Thanksgiving Friday 2024 closes at 13:00 local (18:00 UTC).
        let events = calendar.session_events(
            ts("2024-11-29T00:00:00Z"),
            ts("2024-11-30T00:00:00Z"),
            &config,
        );

        let kinds: Vec<SessionEventKind> = events.iter().map(|event| event.kind).collect();
        assert_eq!(
            kinds,
            vec![
                SessionEventKind::Premarket,
                SessionEventKind::Open,
                SessionEventKind::OpeningRangeComplete,
                SessionEventKind::Midday,
                SessionEventKind::PreClose,
                SessionEventKind::EarlyClose,
            ]
        );
        assert_eq!(events[1].ts_event, ts("2024-11-29T14:30:00Z"));
        assert_eq!(events[3].ts_event, ts("2024-11-29T16:15:00Z"));
        assert_eq!(events[4].ts_event, ts("2024-11-29T17:30:00Z"));
        assert_eq!(events[5].ts_event, ts("2024-11-29T18:00:00Z"));
        assert_eq!(events[5].session_date.to_string(), "2024-11-29");
        assert_eq!(events[5].session_index, 0);
        assert_eq!(events[5].key.to_string(), "XNYS.EQUITY");
    }

    #[rstest]
    fn test_window_is_half_open(calendar: &'static TradingCalendar, config: SessionScheduleConfig) {
        let events = calendar.session_events(
            ts("2024-06-03T13:30:00Z"),
            ts("2024-06-03T20:00:00Z"),
            &config,
        );

        let kinds: Vec<SessionEventKind> = events.iter().map(|event| event.kind).collect();
        assert_eq!(
            kinds,
            vec![
                SessionEventKind::Open,
                SessionEventKind::OpeningRangeComplete,
                SessionEventKind::Midday,
                SessionEventKind::PreClose,
            ]
        );
    }

    #[rstest]
    fn test_disabled_kinds_derive_nothing(
        calendar: &'static TradingCalendar,
        config: SessionScheduleConfig,
    ) {
        let config = config.with_kinds(vec![SessionEventKind::Open]);
        let events = calendar.session_events(
            ts("2024-06-03T00:00:00Z"),
            ts("2024-06-04T00:00:00Z"),
            &config,
        );

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].kind, SessionEventKind::Open);
        assert_eq!(events[0].name(), "SESSION-OPEN:XNYS.EQUITY:2024-06-03:0");
    }

    #[rstest]
    fn test_repeated_expansion_is_identical(
        calendar: &'static TradingCalendar,
        config: SessionScheduleConfig,
    ) {
        let first = calendar.session_events(
            ts("2024-06-01T00:00:00Z"),
            ts("2024-06-08T00:00:00Z"),
            &config,
        );
        let second = calendar.session_events(
            ts("2024-06-01T00:00:00Z"),
            ts("2024-06-08T00:00:00Z"),
            &config,
        );

        assert_eq!(first, second);
        assert_eq!(first.len(), 5 * 6);
    }

    #[rstest]
    fn test_event_name_identifies_the_phase() {
        let event = SessionEvent::new(
            SessionEventKind::PreClose,
            CalendarKey::new(Venue::from("XNYS"), AssetClass::Equity, None),
            Date::from_str("2024-11-29").unwrap(),
            0,
            ts("2024-11-29T17:30:00Z"),
            ts("2024-11-29T17:30:00Z"),
        );

        assert_eq!(event.name(), "SESSION-PRE_CLOSE:XNYS.EQUITY:2024-11-29:0");
        assert_eq!(event.to_string(), event.name());
    }

    #[rstest]
    fn test_config_rejects_an_offset_longer_than_a_day() {
        let result = SessionScheduleConfig::new(
            Duration::from_hours(25),
            Duration::from_mins(1),
            Duration::from_mins(1),
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_session_event_kind_round_trips_through_strings() {
        for kind in SessionEventKind::all() {
            assert_eq!(kind.as_str().parse::<SessionEventKind>().unwrap(), kind);
            assert_eq!(kind.to_string(), kind.as_str());
        }
    }
}
