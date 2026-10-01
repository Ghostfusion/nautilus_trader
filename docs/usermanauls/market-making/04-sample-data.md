# 04 - Sample data

Every runnable example so far used data generated in memory. This lecture introduces the committed
files in `sample_data/`, explains every column, and shows you how to load one of them into the
engine yourself.

## Why committed sample data

The in-memory generators are convenient but invisible: you cannot open them in a spreadsheet or diff
them against a real feed. The files in `sample_data/` are small, deterministic, and committed, so a
reader who runs the documented generator command gets byte-identical files. They also let this
course run with no network access.

A warning repeated from lecture `03`: the `TestDataProvider` methods whose names end in `_from_...`
download their CSV from GitHub on first use (`python/nautilus_trader/testkit/providers.py:111`).
They need the network. Prefer the in-memory generators or a committed file like these.

## The four files

Read `sample_data/README.md` for the full per-file description. In brief:

| File                       | Rows | What it is                                           |
| -------------------------- | ---- | ---------------------------------------------------- |
| `usdjpy_quotes_sample.csv` | 300  | USD/JPY quote updates from the sine generator.       |
| `usdjpy_trades_sample.csv` | 300  | USD/JPY trade prints at those quotes.                |
| `gbpusd_quotes_sample.csv` | 40   | GBP/USD quotes around the lecture `01` example.      |
| `book_snapshot_deltas.csv` | 2    | An order book snapshot borrowed from a test fixture. |

## Columns and units

### `usdjpy_quotes_sample.csv`

- `ts_event_ns`: the quote timestamp, integer nanoseconds since the Unix epoch.
- `bid_price`: the best buy price, 3 decimal places.
- `ask_price`: the best sell price, 3 decimal places.
- `bid_size`: the size available at the bid, in base units of USD/JPY.
- `ask_size`: the size available at the ask.

### `usdjpy_trades_sample.csv`

- `ts_event_ns`: the trade timestamp, integer nanoseconds.
- `price`: the price the trade printed at.
- `size`: the traded size.
- `aggressor_side`: `BUY` if the buyer crossed the spread, `SELL` if the seller did.

### `gbpusd_quotes_sample.csv`

Same columns as the USD/JPY quote file, with 5 decimal places and 40 rows.

### `book_snapshot_deltas.csv`

This file uses the Tardis delta shape, copied from the fixture
`crates/adapters/tardis/test_data/csv/deltas_1.csv`:

- `exchange`, `symbol`: where and what.
- `timestamp`, `local_timestamp`: venue and local capture time, nanoseconds.
- `is_snapshot`: `True` means this row is part of a full book image.
- `side`: `bid` or `ask`.
- `price`, `amount`: the level price and size.

## Inspecting the files

The following program reads all four files and prints the first six rows of each. It also proves the
first file is exactly the prefix of the in-memory generator.

```python
from pathlib import Path

import pandas as pd

DATA = Path("docs/usermanauls/market-making/sample_data")

pd.set_option("display.max_columns", None)
pd.set_option("display.width", 200)

for name in (
    "usdjpy_quotes_sample.csv",
    "usdjpy_trades_sample.csv",
    "gbpusd_quotes_sample.csv",
    "book_snapshot_deltas.csv",
):
    frame = pd.read_csv(DATA / name)
    print(f"=== {name}: {len(frame)} rows, columns {list(frame.columns)}")
    print(frame.head(6).to_string(index=False))
    print()

# Verify the first file is the deterministic prefix of the in-memory generator.
from nautilus_trader.testkit.providers import TestDataProvider

quotes = TestDataProvider.usdjpy_quotes(count=300)
frame = pd.read_csv(DATA / "usdjpy_quotes_sample.csv")
matches = all(
    int(row.ts_event_ns) == int(quotes[i].ts_event)
    and float(row.bid_price) == float(quotes[i].bid_price)
    for i, row in frame.iterrows()
)
print("usdjpy_quotes_sample.csv equals the first 300 generated quotes:", matches)
```

Run it from the repository root, where `DATA` resolves:

```bash
uv run --no-sync --project python python /path/to/your/scratch/inspect_sample_data.py
```

## The observed output

```text
=== usdjpy_quotes_sample.csv: 300 rows, columns ['ts_event_ns', 'bid_price', 'ask_price', 'bid_size', 'ask_size']
        ts_event_ns  bid_price  ask_price  bid_size  ask_size
1546383600000000000    109.500    109.510   1000000   1000000
1546383601000000000    109.501    109.511   1000000   1000000
1546383602000000000    109.502    109.512   1000000   1000000
1546383603000000000    109.503    109.513   1000000   1000000
1546383604000000000    109.504    109.514   1000000   1000000
1546383605000000000    109.505    109.515   1000000   1000000

=== usdjpy_trades_sample.csv: 300 rows, columns ['ts_event_ns', 'price', 'size', 'aggressor_side']
        ts_event_ns   price   size aggressor_side
1546383600000000000 109.510 250000            BUY
1546383601000000000 109.501 260000           SELL
1546383602000000000 109.512 270000            BUY
1546383603000000000 109.503 250000           SELL
1546383604000000000 109.514 260000            BUY
1546383605000000000 109.505 270000           SELL

=== gbpusd_quotes_sample.csv: 40 rows, columns ['ts_event_ns', 'bid_price', 'ask_price', 'bid_size', 'ask_size']
        ts_event_ns  bid_price  ask_price  bid_size  ask_size
1546383600000000000    1.20000    1.20020   1000000   1000000
1546383601000000000    1.20001    1.20021   1000000   1000000
1546383602000000000    1.20002    1.20022   1000000   1000000
1546383603000000000    1.20003    1.20023   1000000   1000000
1546383604000000000    1.20004    1.20024   1000000   1000000
1546383605000000000    1.20005    1.20025   1000000   1000000

=== book_snapshot_deltas.csv: 2 rows, columns ['exchange', 'symbol', 'timestamp', 'local_timestamp', 'is_snapshot', 'side', 'price', 'amount']
exchange        symbol           timestamp     local_timestamp  is_snapshot side  price  amount
 deribit BTC-PERPETUAL 1585699200245000000 1585699200355684000         True  ask 6421.5   18640
 deribit BTC-PERPETUAL 1585699200245000000 1585699200355684000         True  bid 6421.0   10000

usdjpy_quotes_sample.csv equals the first 300 generated quotes: True
```

The last line is the important one: the committed file and the generator agree, so the file's
provenance is provable rather than asserted.

## Reading the USD/JPY quotes

Two facts to notice in the numbers:

1. The bid rises by exactly `0.001` per second at the start. That is the sine wave's slope near
   zero, not a real market. Real FX quotes move by irregular increments.
2. The spread is constant at `0.010` (about `0.9` bps at `109.5`). Real spreads widen around news
   and rollover. A constant spread flatters a market maker's fill assumptions.

That is fine for learning. Do not calibrate a live strategy on this file.

## How real data would arrive

In production, quote and trade data arrive from an adapter, not from a CSV you read yourself:

- The FXCM adapter can produce quote ticks from bid and ask bar CSVs; the interaction is what
  `TestDataProvider.quotes_from_fxcm_bars` wraps (`python/nautilus_trader/testkit/providers.py:400`).
- The Tardis adapter replays order book deltas and trades, which is where the shape of
  `book_snapshot_deltas.csv` comes from (`crates/adapters/tardis/test_data/csv/deltas_1.csv`).
- Live, a data client subscribes with `subscribe_quotes(instrument_id)` and the engine calls
  `on_quote` for each tick.

## Loading a committed CSV into the engine

Here is a complete program that reads `usdjpy_quotes_sample.csv`, converts each row into a
`QuoteTick`, and runs the same grid market maker as lecture `03`.

```python
import csv
from decimal import Decimal
from pathlib import Path

import pandas as pd

from nautilus_trader.backtest import BacktestEngine
from nautilus_trader.common import LogLevel
from nautilus_trader.common import LoggerConfig
from nautilus_trader.config import BacktestEngineConfig
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.execution import ProbabilisticFillModel
from nautilus_trader.model import AccountType
from nautilus_trader.model import Currency
from nautilus_trader.model import Money
from nautilus_trader.model import OmsType
from nautilus_trader.model import Price
from nautilus_trader.model import Quantity
from nautilus_trader.model import QuoteTick
from nautilus_trader.model import TraderId
from nautilus_trader.model import Venue
from nautilus_trader.testkit.providers import TestInstrumentProvider
from nautilus_trader.trading import GridMarketMakerConfig

DATA = Path("docs/usermanauls/market-making/sample_data")


def load_quotes(path, instrument_id):
    quotes = []
    with path.open(newline="", encoding="ascii") as f:
        for row in csv.DictReader(f):
            ts = int(row["ts_event_ns"])
            quotes.append(
                QuoteTick(
                    instrument_id=instrument_id,
                    bid_price=Price.from_str(row["bid_price"]),
                    ask_price=Price.from_str(row["ask_price"]),
                    bid_size=Quantity.from_int(int(row["bid_size"])),
                    ask_size=Quantity.from_int(int(row["ask_size"])),
                    ts_event=ts,
                    ts_init=ts,
                ),
            )
    return quotes


engine = BacktestEngine(
    BacktestEngineConfig(
        trader_id=TraderId.from_str("BACKTESTER-001"),
        logging=LoggerConfig(stdout_level=LogLevel.OFF, print_config=False),
    ),
)
SIM = Venue("SIM")
USD = Currency.from_str("USD")
engine.add_venue(
    venue=SIM,
    oms_type=OmsType.NETTING,
    account_type=AccountType.MARGIN,
    base_currency=USD,
    starting_balances=[Money(10_000_000, USD)],
    fill_model=ProbabilisticFillModel(prob_fill_on_limit=1.0, prob_slippage=0.0, random_seed=42),
    fee_model=MakerTakerFeeModel(maker_rate=Decimal("0.00002"), taker_rate=Decimal("0.00002")),
)
USDJPY_SIM = TestInstrumentProvider.usdjpy_sim()
engine.add_instrument(USDJPY_SIM)
engine.add_data(load_quotes(DATA / "usdjpy_quotes_sample.csv", USDJPY_SIM.id))
engine.add_builtin_strategy(
    "GridMarketMaker",
    GridMarketMakerConfig(
        instrument_id=USDJPY_SIM.id,
        max_position=Quantity.from_int(1_500_000),
        trade_size=Quantity.from_int(500_000),
        num_levels=3,
        grid_step_bps=3,
        skew_factor=0.0,
        requote_threshold_bps=20,
    ),
)
engine.run()

pd.set_option("display.max_columns", None)
pd.set_option("display.width", 200)
print(engine.generate_order_fills_report()[["side", "type", "price", "filled_qty", "avg_px", "liquidity_side"]].to_string())

engine.reset()
engine.dispose()
```

Observed output (run from the repository root):

```text
                              side    type    price filled_qty   avg_px liquidity_side
client_order_id
O-20190101-230000-001-001-2   SELL   LIMIT  109.538     500000  109.538          MAKER
O-20190101-230000-001-001-4   SELL   LIMIT  109.571     500000  109.572          MAKER
O-20190101-230000-001-001-6   SELL   LIMIT  109.604     500000  109.604          MAKER
O-20190101-230459-001-001-10   BUY  MARKET      NaN    1500000  109.791          TAKER
```

The three maker sells and the final taker close are the same shape as lecture `03`, on a shorter
sample. The loader is deliberately boring: it builds one `QuoteTick` per row with the timestamp
placed in both `ts_event` and `ts_init`. `ts_event` is when the venue produced the tick; `ts_init`
is when the engine received it. For historical CSV replay they are the same.

## Rule of thumb for your own sample data

Quote rows are enough to place quotes and reason about the spread. They are not enough to measure
queue position or slippage, because the engine cannot see how much size traded ahead of you. For
that you need trade ticks or book deltas, which is why the second file exists and why lecture `05`
turns on `queue_position` only when trades are present.

Continue to [05-build-the-strategy.md](05-build-the-strategy.md).

Previous: [03-first-run.md](03-first-run.md) | Next: [05-build-the-strategy.md](05-build-the-strategy.md)
