# Trading Calendars

A trading calendar describes when a market trades: weekly sessions expressed in exchange
local time, holidays, early closes, and the date range the data covers. Calendars answer
whether a given instant is tradeable, and when the next or previous session boundary is.

## Two separate responsibilities

Instrument lifetime and tradeability are different questions, and NautilusTrader keeps them
apart:

- **Instrument lifetime** belongs to the instrument itself, through `activation_ns` and
  `expiration_ns`. These say when the instrument exists.
- **A trading calendar** answers only whether an instant is tradeable, and when the sessions
  are. It never extends or shortens an instrument's lifetime.

A calendar is an immutable input to a run. It is loaded once, validated on load, and never
mutated while the run is in progress, so a backtest and a live run that share the same
calendar data resolve the same sessions.

## The data contract

Calendars are JSON documents with a versioned schema identifier:

```text
nautilus-trading-calendar/v1
```

| Field          | Meaning                                                            |
| -------------- | ------------------------------------------------------------------ |
| `schema`       | The schema identifier, checked on load.                            |
| `venue`        | The venue the calendar applies to.                                 |
| `asset_class`  | The instrument class the calendar applies to.                      |
| `symbol`       | An optional symbol, for a calendar that applies to one listing.    |
| `time_zone`    | The exchange time zone, as an IANA name.                           |
| `sessions`     | Sessions per weekday, each with a local `start` and `end` time.    |
| `holidays`     | Dates with no sessions.                                            |
| `early_closes` | Dates with an earlier session close than the weekly schedule.      |
| `valid_from`   | The first date the data covers.                                    |
| `valid_until`  | The last date the data covers, or null when the data is unbounded. |
| `source`       | Optional provenance for the data.                                  |

The calendar key is the venue plus the instrument class plus the optional symbol, so an
equity calendar resolves per listing while a foreign exchange calendar resolves per session.

Sessions are declared per weekday, are sorted and non-overlapping, and do not cross midnight:
a market that trades overnight is described as two sessions. An early close only shortens a
session that already exists; a date with no session stays closed.

## Bundled calendars

The repository bundles two datasets, embedded at compile time and parsed once on first use:

- **Foreign exchange sessions** under the synthetic venue `FX`, keyed by session symbol
  (`SYDNEY`, `TOKYO`, `LONDON`, `NEW_YORK`). These are the schedules the
  `nautilus_trader.trading` FX session helpers use.
- **`XNYS` equity sessions** for 2024 and 2025, with New York Stock Exchange holidays and
  early closes.

Loading is fallible and reported to the caller:

```python
from nautilus_trader.model import TradingCalendar

calendar = TradingCalendar.bundled("XNYS", "EQUITY")

assert calendar.key == "XNYS.EQUITY"
assert calendar.time_zone == "America/New_York"
assert calendar.sessions_on("2024-11-29") == [("09:30:00", "13:00:00")]
assert calendar.is_holiday("2024-11-28")
```

A venue or instrument class with no bundled calendar resolves `None`, and the calendar is
supplied by path instead:

```python
calendar = TradingCalendar.from_json_path("/data/calendars/xnys.json")
```

## Tradeability and boundaries

Tradeability is evaluated in exchange local time, so a session is open at the same local time
regardless of the daylight saving offset in force:

```python
assert calendar.is_tradeable(1732890600000000000)  # 2024-11-29T14:30:00Z, 09:30 New York
assert not calendar.is_tradeable(1732903200000000000)  # 2024-11-29T18:00:00Z, after the early close
```

Boundaries walk the schedule, skipping holidays and weekdays without sessions:

```python
next_open = calendar.next_open(1732890600000000000)
previous_close = calendar.prev_close(1732890600000000000)
```

Coverage is explicit rather than implied: `valid_from` and `valid_until` bound the data, and
a run that extends past the coverage end logs a warning instead of silently assuming the
schedule of the last covered date.

## Determinism

Calendar resolution is a pure function of the timestamp and the calendar data:

- No wall-clock reads: the current time always comes from the `Clock`.
- No ambient time zone: the exchange time zone comes from the calendar data, resolved through
  the bundled time zone database.
- No hidden mutation: loading produces a new value, and a run cannot change the calendar it
  was given.

## Session events

A session event is a phase of a trading session, resolved to an absolute UTC instant on the
calendar. The kinds are `Premarket`, `Open`, `OpeningRangeComplete`, `Midday`, `PreClose`,
`Close`, and `EarlyClose`. Each kind is a derivation of a session:

| Kind                   | Derivation                                   |
| ---------------------- | -------------------------------------------- |
| `Premarket`            | The session open minus the premarket offset  |
| `Open`                 | The session open                             |
| `OpeningRangeComplete` | The session open plus the opening range      |
| `Midday`               | The midpoint of the session open and close   |
| `PreClose`             | The session close minus the pre-close offset |
| `Close`                | The session close on a full trading day      |
| `EarlyClose`           | The session close on an early-close day      |

Offsets are absolute elapsed time, not civil clock time: a premarket offset of one hour is one
hour of real time before the open. `SessionScheduleConfig` carries the three offsets
(`premarket_offset`, `opening_range`, `pre_close_offset`) and the set of kinds to derive, and
rejects an offset longer than one day. A derived instant that cannot be represented is omitted
rather than saturated, and `OpeningRangeComplete` is omitted when it would land at or after the
session close.

`calendar.session_events(from_ns, to_ns, config)` expands the calendar into the events that
occur in the half-open window `[from, to)`. A phase exactly at `to` belongs to the next
expansion, so consecutive windows neither duplicate nor drop a phase. Events are ordered by
instant, then by kind.

A date with no session, whether a holiday or a weekend, derives no events. On a date the
calendar declares an early close, the session close is reported as `EarlyClose` instead of
`Close`, and every phase derived from the close (the pre-close and the midday) moves with it.

`ts_event` and `ts_init` are equal: an event is a pure derivation of calendar data, so expanding
a schedule never reads the current time. That is what keeps a backtest deterministic.

Every event carries a deterministic name that identifies the phase, for example
`SESSION-OPEN:XNYS.EQUITY:2024-06-03:0`. Because the name identifies the event and nothing else,
scheduling the same window twice is idempotent: an event whose timer is already pending is not
rescheduled.

### Delivery

Session events are delivered through the existing timer machinery, not a parallel path. An actor
or strategy schedules them with `schedule_session_events(calendar, config, to_ns)`, which
registers each event as a named time alert on the component clock. Ordering and firing are
inherited from the timer machinery, and no wall clock is read to decide when a phase occurs.

The callback is `on_session_event`, not `on_time_event`. The two are separate by design, and the
distinction is the point: a clock timer is an interval, while a session event is anchored to a
market calendar at a phase of a session.

```python
from nautilus_trader.model import SessionEventKind, SessionScheduleConfig, TradingCalendar

calendar = TradingCalendar.bundled("XNYS", "EQUITY")
config = SessionScheduleConfig(
    premarket_offset_ns=60 * 60 * 1_000_000_000,  # One hour before the open
    opening_range_ns=30 * 60 * 1_000_000_000,  # Thirty minutes after the open
    pre_close_offset_ns=30 * 60 * 1_000_000_000,  # Thirty minutes before the close
    kinds=[SessionEventKind.OPEN, SessionEventKind.PRE_CLOSE],
)

events = calendar.session_events(from_ns, to_ns, config)
for event in events:
    print(event.name(), event.kind, event.ts_event)
```

where the window is half open, for example `from_ns = 1_717_372_800_000_000_000`
(2024-06-03T00:00:00Z) and `to_ns = 1_717_459_200_000_000_000` (2024-06-04T00:00:00Z).

Scheduling from a component registers the events on its own clock and dispatches to
`on_session_event`:

```python
class OpeningRangeStrategy(Strategy):
    def on_start(self):
        calendar = TradingCalendar.bundled("XNYS", "EQUITY")
        config = SessionScheduleConfig(
            premarket_offset_ns=0,
            opening_range_ns=30 * 60 * 1_000_000_000,
            pre_close_offset_ns=30 * 60 * 1_000_000_000,
        )
        now_ns = self.clock.timestamp_ns()
        to_ns = now_ns + 30 * 24 * 60 * 60 * 1_000_000_000  # Thirty days ahead
        self.schedule_session_events(calendar, config, to_ns=to_ns)

    def on_session_event(self, event):
        if event.kind == SessionEventKind.OPENING_RANGE_COMPLETE:
            self.log.info(f"Opening range complete at {event.ts_event}")
```

### What is not a session event

Session-aware scheduling duplicates nothing that already exists:

- Instrument expiration is an engine timer named `INSTRUMENT-EXPIRATION`, and keeps that path.
- A user timestamp, such as an economic release, is already a `Clock` time alert, so it is
  scheduled through the existing timer API.