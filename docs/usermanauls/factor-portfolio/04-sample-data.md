# 04 - The sample data

Every lecture in this manual reads one file:
`docs/usermanauls/factor-portfolio/sample_data/price_panel.csv`. This lecture explains what it
represents, what each column means, how to read it, where such data would come from in a real
system, and how to inspect it.

## What the file represents

The file is a **price panel**. "Panel" is the standard word for a table indexed by two things at
once: one row per instrument per instant. Here the instruments are four, all on the simulated venue
`SIM`, and the instants are 36 consecutive days at 00:00:00 UTC.

- `AAA.SIM` is a steady riser.
- `BBB.SIM` drifts up slowly.
- `CCC.SIM` declines.
- `DDD.SIM` rises fastest in percentage terms.

The prices are made up. They are a deterministic fixture, not a slice of any real market. That is
deliberate: a fixture that never changes is the only kind of data a beginner can check by hand, and
it keeps the manual offline. The full provenance is in
[`sample_data/README.md`](sample_data/README.md).

## Columns and units

| Column          | Type    | Unit                         | Meaning                                    |
| --------------- | ------- | ---------------------------- | ------------------------------------------ |
| `ts_event_ns`   | integer | nanoseconds since Unix epoch | The instant of the close, at 00:00:00 UTC. |
| `instrument_id` | string  | none                         | The instrument as `SYMBOL.VENUE`.          |
| `close`         | decimal | money units                  | The closing price at that instant.         |

The engine stores every timestamp as an integer number of nanoseconds since 1 January 1970. A day is
`86_400_000_000_000` nanoseconds. The first row's timestamp, `1735689600000000000`, is
2025-01-01T00:00:00 UTC.

The column names carry their units on purpose. A bare `close` would leave you guessing whether it is
money or a fraction; `close` in money units, paired with `ts_event_ns` in nanoseconds, is
unambiguous.

## The first rows

Eight rows, verbatim. The file has 144 data rows plus the header; the other 136 data rows are
elided.

```csv
ts_event_ns,instrument_id,close
1735689600000000000,AAA.SIM,100.00
1735689600000000000,BBB.SIM,50.00
1735689600000000000,CCC.SIM,80.00
1735689600000000000,DDD.SIM,20.00
1735776000000000000,AAA.SIM,101.90
1735776000000000000,BBB.SIM,52.05
1735776000000000000,CCC.SIM,80.70
1735776000000000000,DDD.SIM,20.75
```

Notice the layout: all four instruments share one timestamp, then the next timestamp repeats with
all four again. That is what makes it a panel rather than four separate series. To get one
instrument's history you filter on the identifier; to get one day's cross section you filter on the
timestamp. The cross section is what a factor ranks.

## How to read it

Read it as a mapping from instrument to a list of `(timestamp, price)` pairs, in increasing time.
That is exactly what the loader in [03](03-first-run.md) builds, and every later program reuses it.

The two operations you need are:

- **Time-series**: look back within one instrument. "The return over the last five days" is one
  instrument's latest price divided by its price five rows ago, minus one.
- **Cross-sectional**: compare instruments at one instant. "Rank these four by their five-day
  return" is done at a single timestamp across all four.

A factor is almost always a time-series quantity that is then compared cross-sectionally. That is
why the momentum factor is computed per instrument and then sorted across instruments.

## Where such data would come from in a real system

A real daily close arrives as a **bar**: a summary of one trading period with an open, a high, a
low, a close and a volume. In NautilusTrader a `Bar` carries its bar type (instrument, period,
aggregation source) and its timestamp, and the engine persists bars in a `ParquetDataCatalog`. The
research crate reads those catalog intervals: a `SourceInterval` in a `DatasetDeclaration` names a
catalog data type, an optional identifier (such as an instrument), and a time interval
(`crates/research/src/dataset.rs`).

Three differences from the fixture matter when you move to real data:

1. **Calendars.** Real markets close for weekends and holidays. A daily series has gaps, and the
   engine's performance periods use plain UTC civil boundaries with no exchange calendar
   (`docs/concepts/performance_periods.md`). A "five-day" lookback over real bars may span seven
   calendar days.
2. **Adjustments.** A stock split or a dividend changes the quoted price without changing the
   company. Raw prices must be adjusted consistently, and the adjustment policy is part of the
   dataset's identity (`DatasetIdentity.adjustment_policy`,
   `python/nautilus_trader/optimization/identity.py`).
3. **Membership.** A real universe changes over time as instruments list and delist. The fixture's
   four instruments are simply assumed present for all 36 days; lecture 05 makes membership an
   explicit, point-in-time idea.

## Inspecting the file

The commands below are run from the repository root. First the cheap shell checks:

```bash
cd docs/usermanauls/factor-portfolio/sample_data
wc -l price_panel.csv
file price_panel.csv
head -5 price_panel.csv
```

```text
145 price_panel.csv
price_panel.csv: CSV ASCII text
ts_event_ns,instrument_id,close
1735689600000000000,AAA.SIM,100.00
1735689600000000000,BBB.SIM,50.00
1735689600000000000,CCC.SIM,80.00
1735689600000000000,DDD.SIM,20.00
```

Then a small program that summarises the file. Save it as `lecture04.py` and run it from the
repository root:

```python
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path

PANEL = Path("docs/usermanauls/factor-portfolio/sample_data/price_panel.csv")
lines = PANEL.read_text().splitlines()
header, rows = lines[0], lines[1:]
stamps, counts, closes = [], Counter(), {}
for row in rows:
    ts_ns, instrument_id, close = row.split(",")
    stamps.append(int(ts_ns))
    counts[instrument_id] += 1
    closes.setdefault(instrument_id, []).append(float(close))

print("header:", header)
print("data rows:", len(rows))
print("instruments:", len(counts), dict(sorted(counts.items())))
print("timestamps:", len(set(stamps)))
first, last = min(stamps), max(stamps)
print("first:", first, datetime.fromtimestamp(first / 1e9, tz=timezone.utc).isoformat())
print("last: ", last, datetime.fromtimestamp(last / 1e9, tz=timezone.utc).isoformat())
for instrument_id in sorted(closes):
    values = closes[instrument_id]
    print(f"{instrument_id}: min {min(values):.2f} max {max(values):.2f} "
          f"first {values[0]:.2f} last {values[-1]:.2f}")
```

```text
header: ts_event_ns,instrument_id,close
data rows: 144
instruments: 4 {'AAA.SIM': 36, 'BBB.SIM': 36, 'CCC.SIM': 36, 'DDD.SIM': 36}
timestamps: 36
first: 1735689600000000000 2025-01-01T00:00:00+00:00
last:  1738713600000000000 2025-02-05T00:00:00+00:00
AAA.SIM: min 99.70 max 114.00 first 100.00 last 114.00
BBB.SIM: min 48.15 max 53.55 first 50.00 last 51.75
CCC.SIM: min 69.10 max 80.70 first 80.00 last 69.50
DDD.SIM: min 20.00 max 28.75 first 20.00 last 28.75
```

Four checks worth doing every time you load a new panel:

- **Row count.** 144 equals 4 instruments times 36 days. A missing day shows up here.
- **Balanced counts.** Every instrument has 36 rows. An unbalanced panel has a gap, and a gap will
  silently shorten your lookbacks.
- **Distinct timestamps.** 36 distinct instants, not 144. If the timestamps were unique per row, the
  rows would not line up into a cross section.
- **Ranges.** `CCC.SIM` falls from 80.00 to 69.50 and `DDD.SIM` rises from 20.00 to 28.75. Those are
  the shapes the momentum factor will detect.

A real file would also have to be checked for adjusted prices, for missing days, and for instruments
that were not yet listed. The fixture removes all three so that the machinery is what you learn, not
the data cleaning.

Next: [05 - Build the strategy](05-build-the-strategy.md).
