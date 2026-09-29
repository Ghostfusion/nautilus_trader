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

//! Scheduling of calendar session events for actors and strategies.
//!
//! Session events are expanded from a [`TradingCalendar`] into absolute UTC instants, then
//! delivered through the existing timer machinery. A session event is therefore ordered and fired
//! exactly like any other timer event, and no wall clock is read to decide when a phase occurs.
//!
//! A clock timer is an interval. A session event is anchored to the market calendar. The two are
//! separate callbacks: [`crate::actor::DataActor::on_time_event`] receives timer events, and
//! `on_session_event` receives calendar events.

use anyhow::Result;
use jiff::Timestamp;
use log::error;
use nautilus_model::calendars::{SessionEvent, SessionScheduleConfig, TradingCalendar};
use ustr::Ustr;

use super::{Actor, DataActor};
use crate::{actor::registry::try_get_actor_unchecked, clock::ClockApi, timer::TimeEventCallback};

/// Schedules the session events of `calendar` in `[now, to)` on `clock`.
///
/// Each event is registered as a named time alert whose callback dispatches
/// [`DataActor::on_session_event`] on the actor registered under `actor_id` with the concrete type
/// `T`. Because the instant is resolved here, from calendar data, the schedule carries no wall
/// clock dependence into the run.
///
/// An event whose timer is already pending is not rescheduled: the event name identifies the
/// event, so scheduling the same window twice is idempotent.
///
/// # Errors
///
/// Returns an error if the clock rejects an alert.
pub fn schedule_session_events<T>(
    clock: &ClockApi<'_>,
    actor_id: Ustr,
    calendar: &TradingCalendar,
    config: &SessionScheduleConfig,
    to: Timestamp,
) -> Result<usize>
where
    T: Actor + DataActor + Sized + 'static,
{
    let from = clock.timestamp_ns().to_datetime_utc();
    let mut scheduled = 0;

    for event in calendar.session_events(from, to, config) {
        let name = event.name();

        if clock.timer_exists(&name) {
            continue;
        }

        let payload: SessionEvent = event.clone();
        let callback = TimeEventCallback::from(move |_event: crate::timer::TimeEvent| {
            if let Some(mut actor) = try_get_actor_unchecked::<T>(&actor_id) {
                if let Err(e) = DataActor::on_session_event(&mut *actor, &payload) {
                    error!("{e}");
                }
            } else {
                error!("Actor {actor_id} not found for session event handling");
            }
        });

        clock.set_time_alert(&name, event.ts_event, Some(callback), None)?;
        scheduled += 1;
    }

    Ok(scheduled)
}
