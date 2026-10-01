# Sample data

This folder holds the only data the microstructure-signals manual needs. Both files are committed
copies of fixtures that already exist in this repository, so you can follow every lecture with no
network access and no market-data subscription.

## Files

| File                            | Data rows | Source fixture                                                  |
| ------------------------------- | --------- | --------------------------------------------------------------- |
| `order_book_deltas_binance.csv` | 9         | `crates/adapters/tardis/test_data/csv/deltas_with_snapshot.csv` |
| `tardis_deltas_1.csv`           | 2         | `crates/adapters/tardis/test_data/csv/deltas_1.csv`             |

Both are ASCII, comma separated, with LF line endings and one trailing newline. Neither was edited:
each is a byte-for-byte copy of its source fixture.

## The schema

Both files use the same Tardis CSV schema, a flattened recorder format for order book messages.

| Column            | Type    | Unit                          | Meaning                                                    |
| ----------------- | ------- | ----------------------------- | ---------------------------------------------------------- |
| `exchange`        | string  | none                          | The venue or feed name, for example `binance-futures`.     |
| `symbol`          | string  | none                          | The instrument's raw symbol, for example `BTCUSDT`.        |
| `timestamp`       | integer | microseconds since Unix epoch | When the venue produced the event.                         |
| `local_timestamp` | integer | microseconds since Unix epoch | When the recorder received the event.                      |
| `is_snapshot`     | boolean | none                          | `true` for snapshot rows, `false` for incremental changes. |
| `side`            | string  | none                          | `bid` or `ask`.                                            |
| `price`           | decimal | quote currency per unit       | The price of the level.                                    |
| `amount`          | decimal | base units                    | The size at that level; zero means the level was removed.  |

### Unit warning

`timestamp` and `local_timestamp` are **microseconds** in these fixtures, not nanoseconds. The
engine stores nanoseconds, so a loader multiplies by 1000. For example
`1640995200000000` microseconds is 2022-01-01T00:00:00Z; forgetting the conversion places the events
in 1970. Lecture 04 covers this in full.

## `order_book_deltas_binance.csv`

The primary teaching file: a snapshot at one instant, five incremental changes, then a second
snapshot and two more changes, for `BTCUSDT` on `binance-futures`.

- Rows: 9 data rows plus one header row.
- Instrument: `BTCUSDT` on `binance-futures`, loaded as `BTCUSDT-PERP.BINANCE` in the lectures.
- Snapshot rows: 4. Rows with `amount == 0`: 1.
- Timestamps: from `1640995200000000` (2022-01-01T00:00:00Z) to `1640995302000000`
  (2022-01-01T00:01:42Z).

Full contents:

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

## `tardis_deltas_1.csv`

The minimal schema fixture: one ask and one bid, both flagged as snapshot. It is included because it
is the fixture the manual's brief names as the style's sample data.

- Rows: 2 data rows plus one header row.
- Instrument: `BTC-PERPETUAL` on `deribit`.
- Timestamp: `1585699200245000000` (2020-04-01T00:00:00Z) for both rows.

Full contents:

```csv
exchange,symbol,timestamp,local_timestamp,is_snapshot,side,price,amount
deribit,BTC-PERPETUAL,1585699200245000000,1585699200355684000,true,ask,6421.5,18640
deribit,BTC-PERPETUAL,1585699200245000000,1585699200355684000,true,bid,6421.0,10000
```

## Provenance

Both files are derived from named repository test fixtures. They are not synthetic, and they are not
taken from any external source. Because they are repository fixtures, their licence is the
repository's own; see `LICENSE` at the repository root.

They were produced by copying the source files byte for byte, with no edits:

```bash
cp crates/adapters/tardis/test_data/csv/deltas_with_snapshot.csv \
   docs/usermanauls/microstructure-signals/sample_data/order_book_deltas_binance.csv
cp crates/adapters/tardis/test_data/csv/deltas_1.csv \
   docs/usermanauls/microstructure-signals/sample_data/tardis_deltas_1.csv
```

To reproduce the exact bytes, run those two commands from the repository root. To reproduce the
manual's loader behaviour, see the `load_deltas` function in lecture 05.

## Verifying the committed bytes

From the repository root:

```bash
wc -l docs/usermanauls/microstructure-signals/sample_data/*.csv
file docs/usermanauls/microstructure-signals/sample_data/*.csv
```

The first file reports 10 lines (9 data rows plus the header), the second reports 3 lines, and both
report `CSV ASCII text`. No non-ASCII character appears in either file.
