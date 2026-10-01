# 04 - Sample data

This lecture describes the data you will use, shows you how to read it, and explains where such data
comes from in a real setup. Two datasets are used in this manual: a committed file of bars, and an
in-memory fixture of quote ticks that the engine turns into bars.

## The committed file

`sample_data/usdjpy_1min_bars.csv` holds 167 one-minute USD/JPY bars. It is ASCII, comma separated,
with a header row and a trailing newline. Its columns are:

| Column        | Unit                           | Meaning                                  |
| ------------- | ------------------------------ | ---------------------------------------- |
| `ts_event_ns` | integer nanoseconds since 1970 | The time the bar closed                  |
| `open`        | yen per USD                    | The first mid price of the minute        |
| `high`        | yen per USD                    | The highest mid price of the minute      |
| `low`         | yen per USD                    | The lowest mid price of the minute       |
| `close`       | yen per USD                    | The last mid price of the minute         |
| `volume`      | quote size units               | The accumulated size the engine recorded |

It was produced by running the in-memory quote fixture through the engine's one-minute bar
aggregation and writing the result out. The generator command and provenance are in
`sample_data/README.md`. The header and the first seven of the 167 data rows are shown; the
remaining 160 rows are elided:

```csv
ts_event_ns,open,high,low,close,volume
1546383600000000000,109.505,109.505,109.505,109.505,1000000
1546383660000000000,109.506,109.565,109.506,109.565,60000000
1546383720000000000,109.566,109.624,109.566,109.624,60000000
1546383780000000000,109.625,109.681,109.625,109.681,60000000
1546383840000000000,109.682,109.736,109.682,109.736,60000000
1546383900000000000,109.737,109.787,109.737,109.787,60000000
1546383960000000000,109.788,109.835,109.788,109.835,60000000
1546384020000000000,109.836,109.878,109.836,109.878,60000000
```

Read the first row carefully. It has a single quote, so open, high, low and close are all 109.505
and the volume is 1,000,000. The remaining 166 rows each hold one full minute of quotes and a volume
of 60,000,000. Every row obeys the bar rules: `high` is at least `open`, `low` and `close`, and
`low` is at most `open` and `close` (`docs/concepts/data/bar.md`).

## Inspecting the file

Save this as `/tmp/inspect_csv.py` and run it. It reads the file with pandas, then rebuilds real
`Bar` objects from the rows.

```python
import csv
from pathlib import Path

import pandas as pd

from nautilus_trader.model import Bar
from nautilus_trader.model import BarType
from nautilus_trader.model import Price
from nautilus_trader.model import Quantity

CSV = Path(
    r"D:/Users/vince/PycharmProjects/nautilus_trader/docs/usermanauls/"
    r"intraday-systematic/sample_data/usdjpy_1min_bars.csv",
)
BAR_TYPE = BarType.from_str("USD/JPY.SIM-1-MINUTE-MID-EXTERNAL")

frame = pd.read_csv(CSV)
print(frame.head(3).to_string(index=False))
print("rows:", len(frame))
print("close min/max:", frame["close"].min(), frame["close"].max())
print("first ts:", frame["ts_event_ns"].iloc[0], "last ts:", frame["ts_event_ns"].iloc[-1])

bars = []
with CSV.open(newline="") as f:
    for row in csv.DictReader(f):
        ts = int(row["ts_event_ns"])
        bars.append(
            Bar(
                bar_type=BAR_TYPE,
                open=Price.from_str(row["open"]),
                high=Price.from_str(row["high"]),
                low=Price.from_str(row["low"]),
                close=Price.from_str(row["close"]),
                volume=Quantity.from_str(row["volume"]),
                ts_event=ts,
                ts_init=ts,
            ),
        )

print("bars built:", len(bars))
print(bars[1])
```

Its output:

```text
        ts_event_ns    open    high     low   close   volume
1546383600000000000 109.505 109.505 109.505 109.505  1000000
1546383660000000000 109.506 109.565 109.506 109.565 60000000
1546383720000000000 109.566 109.624 109.566 109.624 60000000
rows: 167
close min/max: 109.005 110.005
first ts: 1546383600000000000 last ts: 1546393560000000000
bars built: 167
USD/JPY.SIM-1-MINUTE-MID-EXTERNAL,109.5060,109.5650,109.5060,109.5650,60000000,1546383660000000000
```

Three things to notice. The close wanders between 109.005 and 110.005, a range of one yen. The
timestamps run from 1546383600000000000 to 1546393560000000000, which is 9960 seconds, just under
two hours forty-six minutes. And the rebuilt `Bar` prints its price with four decimals even though
the file stores three; the engine keeps a precision on the price rather than the digits in the file.

The bar type here is `EXTERNAL`, because these bars arrive already built and are handed to the
engine with `add_data`. The engine's matching engine only uses bars for execution when they are
external and the venue keeps an L1 book; see `docs/concepts/backtesting/bar-execution.md`. The
end-to-end run in lecture 05 instead uses `INTERNAL` bars aggregated from quotes, which is the more
faithful offline path.

## The in-memory quote fixture

`TestDataProvider.usdjpy_quotes(count=10_000)` generates quote ticks without any network access
(`python/nautilus_trader/testkit/providers.py`). Each tick's bid follows a sine wave and the ask is
the bid plus 0.010. The first five ticks, printed by a small program:

```python
from nautilus_trader.testkit.providers import TestDataProvider

quotes = TestDataProvider.usdjpy_quotes(count=5)
for q in quotes:
    print(q.instrument_id, q.bid_price, q.ask_price, q.bid_size, q.ask_size, q.ts_event)
print("spread:", quotes[0].ask_price - quotes[0].bid_price)
```

```text
USD/JPY.SIM 109.500 109.510 1000000 1000000 1546383600000000000
USD/JPY.SIM 109.501 109.511 1000000 1000000 1546383601000000000
USD/JPY.SIM 109.502 109.512 1000000 1000000 1546383602000000000
USD/JPY.SIM 109.503 109.513 1000000 1000000 1546383603000000000
USD/JPY.SIM 109.504 109.514 1000000 1000000 1546383604000000000
spread: 0.010
```

That output is the raw material for the committed bars. The engine aggregates these one-second ticks
into one-minute bars. The spread is 0.010 yen forever; a real market's spread changes with activity,
so this fixture is easier than reality, not harder.

## How a real bar series arrives

In a real setup you do not hand-build each `Bar`. You record data into the Parquet data catalog and
query it back. The API is `ParquetDataCatalog` (`docs/concepts/data/catalog.md`). This program
writes the committed bars to a catalog outside the repository, reads them back, and prints what it
found.

```python
import csv
import shutil
from pathlib import Path

from nautilus_trader.model import Bar
from nautilus_trader.model import BarType
from nautilus_trader.model import Price
from nautilus_trader.model import Quantity
from nautilus_trader.persistence import ParquetDataCatalog

CSV = Path(
    r"D:/Users/vince/PycharmProjects/nautilus_trader/docs/usermanauls/"
    r"intraday-systematic/sample_data/usdjpy_1min_bars.csv",
)
BAR_TYPE = BarType.from_str("USD/JPY.SIM-1-MINUTE-MID-EXTERNAL")
CATALOG_PATH = Path(r"D:/tmp/nautmanual/catalog")

if CATALOG_PATH.exists():
    shutil.rmtree(CATALOG_PATH)
CATALOG_PATH.mkdir(parents=True)

bars = []
with CSV.open(newline="") as f:
    for row in csv.DictReader(f):
        ts = int(row["ts_event_ns"])
        bars.append(
            Bar(
                bar_type=BAR_TYPE,
                open=Price.from_str(row["open"]),
                high=Price.from_str(row["high"]),
                low=Price.from_str(row["low"]),
                close=Price.from_str(row["close"]),
                volume=Quantity.from_str(row["volume"]),
                ts_event=ts,
                ts_init=ts,
            ),
        )

catalog = ParquetDataCatalog(str(CATALOG_PATH))
path = catalog.write_bars(bars, skip_disjoint_check=True)
print("written to subdirectory:", path)

loaded = catalog.query_bars(
    identifiers=["USD/JPY.SIM-1-MINUTE-MID-EXTERNAL"],
    start=1546383600000000000,
    end=1546393560000000000,
)
print("bars read back:", len(loaded))
print("first:", loaded[0])
print("last:", loaded[-1])
```

Its output:

```text
written to subdirectory: data/bars/USDJPY.SIM-1-MINUTE-MID-EXTERNAL\2019-01-01T23-00-00-000000000Z_2019-01-02T01-46-00-000000000Z.parquet
bars read back: 167
first: USD/JPY.SIM-1-MINUTE-MID-EXTERNAL,109.505,109.505,109.505,109.505,1000000,1546383600000000000
last: USD/JPY.SIM-1-MINUTE-MID-EXTERNAL,109.912,109.944,109.912,109.944,60000000,1546393560000000000
```

The catalog stored the bars under a subdirectory named after the bar type and the covered time range,
and the query returned all 167 in order. In a real setup the same query would return bars recorded
from a live or historical feed.

Once the bars are in hand, `engine.add_data(bars)` hands them to the engine exactly as in the
inspection program above, and the strategy subscribes to the matching bar type. The typed query
methods for quotes, trades and bars are documented in `docs/concepts/data/catalog.md`.

For a `BacktestNode` run you do not query by hand; a `BacktestDataConfig` names the catalog data for
the run instead (`docs/concepts/data/catalog.md`). The catalog is the storage layer; the engine never
cares whether a bar was aggregated from ticks or read from disk, because both paths produce the same
`Bar` object.

## Where the data can go wrong

- If `ts_event` and `ts_init` are wrong, execution is wrong. For bars timestamped at the close, set
  `ts_init` equal to `ts_event`; for bars timestamped at the open, add the interval length
  (`docs/concepts/backtesting/bar-execution.md`).
- If the bar's price precision does not match the instrument, the matching engine skips the bar. The
  committed file uses three decimals to match the USD/JPY instrument.
- If the data is internally aggregated, the bar is closed by a timer at the interval boundary, and
  data with the same timestamp can arrive just after the close; `DataEngineConfig(time_bars_build_delay=1)`
  documents the one-microsecond remedy (`docs/concepts/backtesting/bar-execution.md`).
- Session time is exchange local time. A calendar resolves holidays and early closes, and daylight
  saving shifts the UTC instant of the open (`docs/concepts/trading_calendars.md`).

Next: build the rule, in [05-build-the-strategy](05-build-the-strategy.md).
