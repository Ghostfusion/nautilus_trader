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