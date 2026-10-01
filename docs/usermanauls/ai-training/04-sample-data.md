# 04 - The sample data

## What the file represents

`sample_data/btcusdt-1d-bars.csv` is one instrument, BTC/USDT on Binance, as a series of one-day
price bars. A **bar** summarises one interval: its opening price, its highest price, its lowest
price, its closing price, and the volume traded. The engine's `Bar` type carries exactly these five
numbers plus a timestamp.

This is a hand-made fixture. It is not real market data and it is not a promise of profit. It exists
so that every program in this manual runs with no network access, from a file committed beside the
lectures.

## The columns and their units

| Column        | Type    | Unit                             | Meaning                              |
| ------------- | ------- | -------------------------------- | ------------------------------------ |
| `ts_event_ns` | integer | nanoseconds since the Unix epoch | The bar's timestamp.                 |
| `open_price`  | decimal | USDT per BTC, two decimal places | The first traded price in the day.   |
| `high_price`  | decimal | USDT per BTC, two decimal places | The highest traded price in the day. |
| `low_price`   | decimal | USDT per BTC, two decimal places | The lowest traded price in the day.  |
| `close_price` | decimal | USDT per BTC, two decimal places | The last traded price in the day.    |
| `volume_size` | decimal | BTC, six decimal places          | The quantity traded in the day.      |

The timestamp is integer nanoseconds because that is the engine's own time type, `UnixNanos`. One
second is 1,000,000,000 nanoseconds, so a day is 86,400,000,000,000.

## How to inspect it

From the repository root, a few shell commands are enough:

```bash
file docs/usermanauls/ai-training/sample_data/btcusdt-1d-bars.csv
wc -l docs/usermanauls/ai-training/sample_data/btcusdt-1d-bars.csv
head -6 docs/usermanauls/ai-training/sample_data/btcusdt-1d-bars.csv
tail -3 docs/usermanauls/ai-training/sample_data/btcusdt-1d-bars.csv
```

The observed output:

```text
btcusdt-1d-bars.csv: CSV ASCII text
241 docs/usermanauls/ai-training/sample_data/btcusdt-1d-bars.csv
ts_event_ns,open_price,high_price,low_price,close_price,volume_size
1640908800000000000,47000.00,47050.00,46750.00,47000.00,1.500000
1640995200000000000,47542.43,47654.21,47296.41,47564.48,1.545392
1641081600000000000,48078.49,48249.43,47844.28,48121.55,1.590409
1641168000000000000,48601.94,48826.90,48386.88,48663.97,1.634679
1641254400000000000,49106.79,49378.28,48917.45,49184.81,1.677838
1661385600000000000,61625.72,61789.28,61383.30,61684.75,1.715542
1661472000000000000,61420.94,61597.65,61193.18,61455.98,1.673694
1661558400000000000,61184.98,61366.49,60978.97,61191.34,1.630412
```

The file has 241 lines: one header and 240 data rows. The first six data rows and the last three are
shown above; the other 231 rows are elided. The first row is a hand-made seed row; the rest are
generated.

A Python reader can summarise it without the engine:

```python
import csv
from datetime import datetime, timezone
from pathlib import Path

path = Path("docs/usermanauls/ai-training/sample_data/btcusdt-1d-bars.csv")
rows = list(csv.DictReader(path.open()))

def iso(ns: str) -> str:
    return datetime.fromtimestamp(int(ns) / 1e9, tz=timezone.utc).isoformat()

print("columns:", list(rows[0].keys()))
print("rows:", len(rows))
print("first ts:", iso(rows[0]["ts_event_ns"]))
print("last ts :", iso(rows[-1]["ts_event_ns"]))
closes = [float(row["close_price"]) for row in rows]
print("close min:", min(closes), "max:", max(closes))
```

The observed output:

```text
columns: ['ts_event_ns', 'open_price', 'high_price', 'low_price', 'close_price', 'volume_size']
rows: 240
first ts: 2021-12-31T00:00:00+00:00
last ts : 2022-08-27T00:00:00+00:00
close min: 44164.43 max: 62150.27
```

So the fixture spans 240 days and roughly 44,000 to 62,000 USDT per BTC.

## How the engine reads it

A backtest never reads a CSV directly. It reads a `ParquetDataCatalog`, and a `BacktestDataConfig`
points at that catalog. Lecture 03 shows the bridge: `load_bars()` parses the CSV into `Bar` objects,
`catalog.write_instruments(...)` and `catalog.write_bars(...)` write them, and `config_factory`
returns a `BacktestDataConfig` whose `catalog_path` is the catalog directory
(`python/nautilus_trader/optimization/runner.py` shows the factory contract).

## How this would come from a real venue

In a real study you would not write the CSV yourself. You would run a venue adapter, or a
`ParquetDataCatalog` writer, to download the bars from the venue's history and write them to the same
catalog format. The rest of the manual is unchanged: the runner, the parameter space, the objective
and the report do not care whether the bars came from a CSV you shaped or from a live venue feed.

The repository's own test data loaders mostly fetch their files from GitHub on first use. For
example `TestDataProvider.bars_from_binance_csv` checks a local `test_data/` directory first and
downloads from `raw.githubusercontent.com` only if the file is absent
(`python/nautilus_trader/testkit/providers.py`). If you use one of those loaders on a machine without
the `test_data/` directory and without network access, it fails. Prefer the in-memory generators such
as `TestDataProvider.usdjpy_quotes`, or a committed CSV like the one in this folder.

## The generator

The file is deterministic: the same generator produces the same bytes. The generator command and its
exact source are recorded in `sample_data/README.md`, which also states the row count and the
provenance. Re-run it and compare the bytes if you want to check.
