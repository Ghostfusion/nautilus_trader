# The sample data

This lecture describes the two committed delta files and shows how to read them. Both are copies of
fixtures that already live in this repository, so they are not made up for the manual and their
provenance is exact. They are ASCII, comma separated, with LF line endings and one trailing
newline. You need no network access and no market data subscription.

## The files

| File                                        | Rows        | Source fixture                                                  |
| ------------------------------------------- | ----------- | --------------------------------------------------------------- |
| `sample_data/order_book_deltas_binance.csv` | 9 data rows | `crates/adapters/tardis/test_data/csv/deltas_with_snapshot.csv` |
| `sample_data/tardis_deltas_1.csv`           | 2 data rows | `crates/adapters/tardis/test_data/csv/deltas_1.csv`             |

The first file is the one the manual uses for reconstruction and the imbalance signal: it contains
a snapshot at one instant, some incremental updates, then a second snapshot. The second file is the
minimal fixture - the few rows needed to show the schema - and is included verbatim so you can see
the raw two-row form of the same format.

## Every column

Both files use the same Tardis CSV schema. Every column is a string until you parse it.

| Column            | Type    | Unit                          | Meaning                                                                     |
| ----------------- | ------- | ----------------------------- | --------------------------------------------------------------------------- |
| `exchange`        | string  | none                          | The venue or feed name, for example `binance-futures` or `deribit`.         |
| `symbol`          | string  | none                          | The instrument's raw symbol, for example `BTCUSDT`.                         |
| `timestamp`       | integer | microseconds since Unix epoch | When the venue produced the event.                                          |
| `local_timestamp` | integer | microseconds since Unix epoch | When the recorder received the event.                                       |
| `is_snapshot`     | boolean | none                          | `true` for rows that belong to a snapshot, `false` for incremental changes. |
| `side`            | string  | none                          | `bid` or `ask`.                                                             |
| `price`           | decimal | quote currency per unit       | The price of the level.                                                     |
| `amount`          | decimal | base units                    | The size resting at that level. Zero means the level was removed.           |

Two column facts decide how you load the file.

- **`timestamp` is in microseconds in this fixture, not nanoseconds.** The value
  `1640995200000000` means 1,640,995,200,000,000 microseconds, which is 2022-01-01T00:00:00Z. The
  engine stores nanoseconds, so a loader multiplies by 1000. Lecture 05 does exactly that. If you
  forgot, the events sort thousands of years into the future and your book is empty.
- **`amount == 0` means the level is gone.** Tardis signals a removal by sending size zero rather
  than a separate delete flag. A loader must translate that into `BookAction.DELETE`; otherwise a
  zero-size level stays in the book and corrupts the signal.

## Inspecting the file

Save this as `inspect.py` outside the repository and run it with the same command as before.

```python
import csv
from datetime import datetime, timezone

path = "docs/usermanauls/microstructure-signals/sample_data/order_book_deltas_binance.csv"

with open(path, newline="") as f:
    reader = csv.DictReader(f)
    columns = reader.fieldnames
    rows = list(reader)

print("columns:", columns)
print("data rows:", len(rows))
first = rows[0]
for name in columns:
    print(f"  {name}: {first[name]}")
ts_us = int(first["timestamp"])
ts_ns = ts_us * 1000
print("first timestamp in UTC:", datetime.fromtimestamp(ts_ns / 1e9, tz=timezone.utc).isoformat())
print("rows where is_snapshot is true:", sum(r["is_snapshot"] == "true" for r in rows))
print("rows where amount is zero:", sum(r["amount"] == "0.0" for r in rows))
```

Run it from the repository root:

```bash
uv run --project python --no-sync python <temp-dir>/scratch/inspect.py
```

Real output:

```text
columns: ['exchange', 'symbol', 'timestamp', 'local_timestamp', 'is_snapshot', 'side', 'price', 'amount']
data rows: 9
  exchange: binance-futures
  symbol: BTCUSDT
  timestamp: 1640995200000000
  local_timestamp: 1640995200100000
  is_snapshot: true
  side: bid
  price: 50000.0
  amount: 1.0
first timestamp in UTC: 2022-01-01T00:00:00+00:00
rows where is_snapshot is true: 4
rows where amount is zero: 1
```

The file has 9 data rows. Four of them are snapshot rows and one carries `amount` zero, which is
the removed level.

## The rows, verbatim

Here is the whole file. It is short enough to show in full, so nothing is elided.

```csv
exchange,symbol,timestamp,local_timestamp,is_snapshot,side,price,amount
binance-futures,BTCUSDT,1640995200000000,1640995200100000,true,bid,50000.0,1.0
binance-futures,BTCUSDT,1640995200000000,1640995200100000,true,ask,50001.0,2.0
binance-futures,BTCUSDT,1640995201000000,1640995201100000,false,bid,49999.0,0.5
binance-futures,BTCUSDT,1640995202000000,1640995202100000,false,ask,50002.0,1.5
binance-futures,BTCUSDT,1640995203000000,1640995203100000,false,bid,49998.0,0.0
binance-futures,BTCUSDT,1640995300000000,1640995300100000,true,bid,50100.0,3.0
binance-futures,BTCUSDT,1640995300000000,1640995300100000,true,ask,50101.0,4.0
binance-futures,BTCUSDT,1640995301000000,1640995301100000,false,bid,50099.0,1.0
binance-futures,BTCUSDT,1640995302000000,1640995302100000,false,ask,50102.0,2.0
```

Read it as a story:

- Rows 1 and 2 are a snapshot at 00:00:00: a bid at 50000.0 for 1.0 and an ask at 50001.0 for 2.0.
- Rows 3 to 5 are incremental changes over the next three hundredths of a second: a new bid at
  49999.0 for 0.5, a new ask at 50002.0 for 1.5, and the removal of the level at 49998.0 (size 0).
- Rows 6 to 9 are a second snapshot at 00:01:40 followed by two more changes. The second snapshot
  starts a new book at a higher price, which is why a loader must emit a `CLEAR` before it.

The second file, verbatim (2 data rows):

```csv
exchange,symbol,timestamp,local_timestamp,is_snapshot,side,price,amount
deribit,BTC-PERPETUAL,1585699200245000000,1585699200355684000,true,ask,6421.5,18640
deribit,BTC-PERPETUAL,1585699200245000000,1585699200355684000,true,bid,6421.0,10000
```

It is the smallest possible demonstration of the schema: one ask and one bid, both flagged as
snapshot. Its `timestamp` is also microseconds (`1585699200245000000` is 2020-04-01T00:00:00Z).

## How this would come from a real venue

A real venue publishes one message per book change, and an adapter turns each message into an
`OrderBookDelta`. The columns here are the recorder's flattened form of exactly those messages. In
production you do not read a CSV; you subscribe:

```text
self.subscribe_book_deltas(instrument_id, BookType.L2_MBP, managed=True)
```

and the engine delivers `OrderBookDeltas` batches to `on_book_deltas`. The tutorial
`docs/tutorials/backtest_orderbook_binance.py` replays one trading day of Binance `T_DEPTH`
messages through exactly this path. The manual uses the small CSV because it is deterministic and
offline.

## Provenance and the copy command

Both files were produced by copying the repository fixtures byte for byte:

```bash
cp crates/adapters/tardis/test_data/csv/deltas_with_snapshot.csv \
   docs/usermanauls/microstructure-signals/sample_data/order_book_deltas_binance.csv
cp crates/adapters/tardis/test_data/csv/deltas_1.csv \
   docs/usermanauls/microstructure-signals/sample_data/tardis_deltas_1.csv
```

No rows were added, removed or edited. Because they are repository fixtures, their licence is the
repository's own; see `LICENSE`. They are not synthetic and not derived from any external source.
[`sample_data/README.md`](sample_data/README.md) repeats this per file.

## Verify the committed bytes

From the repository root:

```bash
wc -l docs/usermanauls/microstructure-signals/sample_data/*.csv
file docs/usermanauls/microstructure-signals/sample_data/*.csv
```

You should see 10 lines for the first file (9 data rows plus the header), 3 lines for the second,
and `CSV ASCII text` for both.

## Next

[05](05-build-the-strategy.md) turns these rows into engine objects and rebuilds the book.
