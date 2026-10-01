# 04 - The sample data

This lecture describes the two committed CSV files that every program in this manual uses, shows
their columns and units, and inspects them with a real run. The files live in
[sample_data/](sample_data/); the authoritative per-file description is
[sample_data/README.md](sample_data/README.md).

## What the data represents

The quotes file is a best-bid/best-ask stream for USD/JPY on a simulated venue called `SIM`. A
**quote tick** is one observation of the top of the order book: the best bid price, the best ask
price, and the size quoted at each. It is the cheapest market data to record and it is enough to
simulate fills for small orders, because the matching engine can use the best prices.

The fills file is not market data. It is the output of the lecture 03 TWAP run: one row per spawned
order and for the parent's final slice. It exists so that you can read a real sliced execution
without running the engine, and so that lecture 06 can compute a cost from committed data.

## Column reference

The quotes file [sample_data/usdjpy_quotes_sample.csv](sample_data/usdjpy_quotes_sample.csv):

| Column        | Unit                             | Meaning                                       |
| ------------- | -------------------------------- | --------------------------------------------- |
| `ts_event_ns` | nanoseconds since the Unix epoch | When the quote happened.                      |
| `bid_price`   | JPY per USD, 3 decimals          | The best price at which the market will buy.  |
| `ask_price`   | JPY per USD, 3 decimals          | The best price at which the market will sell. |
| `bid_size`    | USD, integer                     | How much is quoted at the best bid.           |
| `ask_size`    | USD, integer                     | How much is quoted at the best ask.           |

The fills file [sample_data/twap_child_fills_sample.csv](sample_data/twap_child_fills_sample.csv):

| Column            | Unit                             | Meaning                                                        |
| ----------------- | -------------------------------- | -------------------------------------------------------------- |
| `client_order_id` | text                             | The identifier of this order.                                  |
| `exec_spawn_id`   | text                             | The parent's identifier; on the parent row this is its own id. |
| `spawn_sequence`  | integer                          | `0` for the parent's final slice, `1`, `2`, ... for children.  |
| `ts_event_ns`     | nanoseconds since the Unix epoch | When this slice filled.                                        |
| `qty`             | USD, integer                     | The quantity of this slice.                                    |
| `price`           | JPY per USD, 3 decimals          | The average fill price of this slice.                          |

Prices are decimal text, not binary floating point, so the engine's exact values survive the round
trip. Time is integer nanoseconds since 1970-01-01 UTC, which is the unit the engine uses everywhere.

## Where such data comes from in real life

A live venue publishes quotes over a websocket. In NautilusTrader an adapter such as the Binance or
Databento adapter subscribes to the book and emits `QuoteTick` objects, which are the same type this
file produces. A historical file would normally come from a vendor's recording of those messages. For
a real run you would point a `ParquetDataCatalog` at the recorded data instead of a CSV; the data
type handed to the engine is the same. See the [backtesting concepts](../../concepts/backtesting/index.md)
for how recorded data is replayed.

The committed file is a **hand-made synthetic fixture**: it is a deterministic sine wave produced by
`TestDataProvider.usdjpy_quotes` in `python/nautilus_trader/testkit/providers.py:680`, truncated to
120 ticks. It is not a recording of any real market, and it carries no venue licence. That is why it
can be committed to this repository and used offline.

## Warning: some testkit loaders need the network

The in-memory generators used here (the sine wave) need no network. But every
`TestDataProvider.*_from_*_csv(...)` method downloads its CSV from GitHub on first use
(`python/nautilus_trader/testkit/providers.py:111-130`). Prefer the in-memory generators or a
committed file. This manual uses only committed files.

## How to inspect it

Save this as `inspect_sample.py` next to the `sample_data/` folder and run it from the `python`
directory:

```python
import csv
from pathlib import Path

from nautilus_trader.model import InstrumentId
from nautilus_trader.model import Price
from nautilus_trader.model import Quantity
from nautilus_trader.model import QuoteTick

BASE = Path(__file__).with_name("sample_data")


def show(path, n=3):
    with path.open(newline="") as f:
        rows = list(csv.reader(f))
    header, body = rows[0], rows[1:]
    print(f"{path.name}: {len(body)} rows, columns={header}")
    for row in body[:n]:
        print("   ", row)
    print("    (last)", body[-1])


show(BASE / "usdjpy_quotes_sample.csv")
show(BASE / "twap_child_fills_sample.csv")

rows = list(csv.reader((BASE / "usdjpy_quotes_sample.csv").open(newline="")))
body = rows[1:]
bids = [float(r[1]) for r in body]
asks = [float(r[2]) for r in body]
spreads = [ask - bid for bid, ask in zip(bids, asks)]
print(f"bid range: {min(bids):.3f} to {max(bids):.3f}")
print(f"mean spread: {sum(spreads) / len(spreads):.3f}")

ts = int(body[0][0])
tick = QuoteTick(
    instrument_id=InstrumentId.from_str("USD/JPY.SIM"),
    bid_price=Price(float(body[0][1]), precision=3),
    ask_price=Price(float(body[0][2]), precision=3),
    bid_size=Quantity.from_int(int(body[0][3])),
    ask_size=Quantity.from_int(int(body[0][4])),
    ts_event=ts,
    ts_init=ts,
)
print("built QuoteTick:", tick)
```

Run it:

```bash
cd python
uv run --no-sync python ../docs/usermanauls/execution-algorithms/inspect_sample.py
```

Real output from that run:

```text
usdjpy_quotes_sample.csv: 120 rows, columns=['ts_event_ns', 'bid_price', 'ask_price', 'bid_size', 'ask_size']
    ['1546383600000000000', '109.500', '109.510', '1000000', '1000000']
    ['1546383601000000000', '109.501', '109.511', '1000000', '1000000']
    ['1546383602000000000', '109.502', '109.512', '1000000', '1000000']
    (last) ['1546383719000000000', '109.618', '109.628', '1000000', '1000000']
twap_child_fills_sample.csv: 6 rows, columns=['client_order_id', 'exec_spawn_id', 'spawn_sequence', 'ts_event_ns', 'qty', 'price']
    ['O-20190101-230000-001-000-1', 'O-20190101-230000-001-000-1', '0', '1546383650000000000', '1000', '109.561']
    ['O-20190101-230000-001-000-1-E1', 'O-20190101-230000-001-000-1', '1', '1546383600000000000', '1000', '109.511']
    ['O-20190101-230000-001-000-1-E2', 'O-20190101-230000-001-000-1', '2', '1546383610000000000', '1000', '109.522']
    (last) ['O-20190101-230000-001-000-1-E5', 'O-20190101-230000-001-000-1', '5', '1546383640000000000', '1000', '109.551']
bid range: 109.500 to 109.618
mean spread: 0.010
built QuoteTick: USD/JPY.SIM,109.500,109.510,1000000,1000000,1546383600000000000
```

## Reading the output

- The quotes file has exactly 120 data rows and five columns. One row per second, so it covers 119
  seconds of simulated time.
- The bid rises smoothly from 109.500 to 109.618 across the file, and the ask is always 0.010 above
  it. The mean spread is 0.010, which is one price increment for this instrument.
- The fills file has six rows: the parent's final slice (`spawn_sequence` 0) plus five children
  (sequences 1 to 5). The parent row appears first because it is the row with the earliest
  `client_order_id` sort order, not because it filled first; its `ts_event_ns` is the latest of the
  six.
- The last line proves the data is enough to build a real engine object: a `QuoteTick` prints as
  `USD/JPY.SIM,bid,ask,bid_size,ask_size,ts_event`.

## Three common beginner misreadings

- **Nanoseconds are not milliseconds.** `1546383600000000000` is 2019-01-02 00:00:00 UTC, not a year
  far in the future. Divide by one billion for seconds.
- **A wide spread is not a bigger market.** Here the spread is one increment (0.010) at a price near
  109, which is about 0.9 bps. A spread of 0.010 on a price of 1.09 would be far larger in bps.
- **The size column is not depth.** `1000000` is what is quoted at the best price only. It says
  nothing about how much is available behind the touch.

Read [05-build-the-strategy.md](05-build-the-strategy.md) to build on this data.
