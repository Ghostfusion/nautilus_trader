# 04 - Sample data

The `sample_data/` folder holds three CSV files. Two of them describe the outside world (market
prices, and what a venue says happened). One describes the inside world (what your strategy
remembered). This lecture says what each file is and shows you reading them.

Read [sample_data/README.md](sample_data/README.md) alongside this lecture. It gives the full column
list, the generator recipe, and the SHA-256 of every file.

## Why these three files

Production operations is about the seam between your process and the venue. Each file sits on one
side of that seam:

| File                     | Side of the seam       | Used by    |
| ------------------------ | ---------------------- | ---------- |
| `quotes_audusd.csv`      | The market, inbound    | Lecture 04 |
| `venue_fills_report.csv` | The venue, inbound     | Lecture 07 |
| `state_snapshots.csv`    | Your process, internal | Lecture 07 |

The market file is what your strategy reacts to. The venue report is what reconciliation compares
your records against. The state file is what survives a restart, or fails to.

## The unit rule

Every timestamp is an integer count of nanoseconds since 1970-01-01 00:00:00 UTC. NautilusTrader
stores times this way internally, so an integer nanosecond timestamp can be handed to the engine
without conversion and without losing precision.

The first timestamp in `quotes_audusd.csv` is `1546383600000000000`, which is 2019-01-02 00:00:00
UTC. One second later is `1546383601000000000`, an increase of `1000000000`. A one-second step is
easy to read once you know that a nanosecond is one thousand-millionth of a second.

## Inspect the files from the shell

```bash
cd <repo-root>/docs/usermanauls/production-operations/sample_data
wc -l *.csv
head -4 quotes_audusd.csv
cat venue_fills_report.csv
cat state_snapshots.csv
```

Observed output:

```text
 121 quotes_audusd.csv
   4 state_snapshots.csv
   4 venue_fills_report.csv
 129 total
ts_event_ns,instrument_id,bid_price,ask_price,bid_size,ask_size
1546383600000000000,AUD/USD.SIM,0.71000,0.71010,1000000,1000000
1546383601000000000,AUD/USD.SIM,0.71002,0.71012,1000000,1000000
1546383602000000000,AUD/USD.SIM,0.71003,0.71013,1000000,1000000
ts_event_ns,venue_order_id,client_order_id,instrument_id,side,last_qty,last_px,commission,commission_currency
1546383730000000000,SIM-77-1,O-20190102-000000-001-000-1,AUD/USD.SIM,BUY,100000,0.71005,0.00,USD
1546383731000000000,SIM-77-2,O-20190102-000000-001-000-2,AUD/USD.SIM,BUY,150000,0.71011,0.00,USD
1546383733000000000,SIM-77-3,O-20190102-000000-001-000-3,AUD/USD.SIM,SELL,50000,0.71022,0.00,USD
ts_event_ns,run_id,strategy_id,event,quote_count,orders_submitted,orders_denied,net_position_qty
1546383600000000000,RUN-001,RUNWAY-001,on_save,120,4,2,200000
1546383719000000000,RUN-001,RUNWAY-001,on_save,120,6,4,200000
1546383720000000000,RUN-002,RUNWAY-001,on_load,120,6,4,200000
```

The line counts include the header, so `quotes_audusd.csv` holds 120 data rows. Every file is ASCII
with LF line endings, so `file` reports plain CSV text and a byte-level search for non-ASCII bytes
finds nothing.

## Read the market file with Python

Save this script outside the repository, for example at
`<temp-dir>/po_manual/inspect_sample_data.py`, and run it. It prints a
summary, rebuilds the rows as engine `QuoteTick` objects, and hands them to a backtest engine to
prove the file is engine-usable.

```python
"""Inspect the committed sample data and feed it to the backtest engine."""

from __future__ import annotations

import csv
from decimal import Decimal
from pathlib import Path

import pandas as pd

from nautilus_trader.backtest import BacktestEngine
from nautilus_trader.common import LogLevel
from nautilus_trader.common import LoggerConfig
from nautilus_trader.config import BacktestEngineConfig
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.model import AccountType
from nautilus_trader.model import Currency
from nautilus_trader.model import InstrumentId
from nautilus_trader.model import Money
from nautilus_trader.model import OmsType
from nautilus_trader.model import Price
from nautilus_trader.model import Quantity
from nautilus_trader.model import QuoteTick
from nautilus_trader.model import TraderId
from nautilus_trader.model import Venue
from nautilus_trader.testkit.providers import TestInstrumentProvider

DATA = Path("<repo-root>/docs/usermanauls/production-operations/sample_data")
QUOTES = DATA / "quotes_audusd.csv"

df = pd.read_csv(QUOTES)
print("rows              :", len(df))
print("columns           :", list(df.columns))
print("instrument_id     :", df["instrument_id"].unique().tolist())
print("first ts_event_ns :", int(df["ts_event_ns"].iloc[0]))
print("last  ts_event_ns :", int(df["ts_event_ns"].iloc[-1]))
print(
    "mid min/max       :",
    round(((df.bid_price + df.ask_price) / 2).min(), 5),
    round(((df.bid_price + df.ask_price) / 2).max(), 5),
)
print()
print(df.head(3).to_string(index=False))
print()
print("elided rows       :", len(df) - 3)

instrument_id = InstrumentId.from_str(df["instrument_id"].iloc[0])
ticks = [
    QuoteTick(
        instrument_id=instrument_id,
        bid_price=Price(float(row["bid_price"]), precision=5),
        ask_price=Price(float(row["ask_price"]), precision=5),
        bid_size=Quantity.from_int(int(row["bid_size"])),
        ask_size=Quantity.from_int(int(row["ask_size"])),
        ts_event=int(row["ts_event_ns"]),
        ts_init=int(row["ts_event_ns"]),
    )
    for row in csv.DictReader(QUOTES.open(encoding="ascii"))
]

SIM = Venue("SIM")
USD = Currency.from_str("USD")
AUDUSD_SIM = TestInstrumentProvider.audusd_sim()

engine = BacktestEngine(
    BacktestEngineConfig(
        trader_id=TraderId.from_str("OPS-001"),
        logging=LoggerConfig(stdout_level=LogLevel.ERROR, print_config=False),
    ),
)
engine.add_venue(
    venue=SIM,
    oms_type=OmsType.NETTING,
    account_type=AccountType.MARGIN,
    base_currency=USD,
    starting_balances=[Money(1_000_000, USD)],
    fee_model=MakerTakerFeeModel(maker_rate=Decimal("0"), taker_rate=Decimal("0")),
)
engine.add_instrument(AUDUSD_SIM)
engine.add_data(ticks)
engine.run()
print()
print("engine data ingested :", engine.cache.quote_count(instrument_id))
engine.reset()
engine.dispose()
```

Run it:

```bash
cd <repo-root>/python
uv run --no-sync python <temp-dir>/po_manual/inspect_sample_data.py
```

Observed output:

```text
rows              : 120
columns           : ['ts_event_ns', 'instrument_id', 'bid_price', 'ask_price', 'bid_size', 'ask_size']
instrument_id     : ['AUD/USD.SIM']
first ts_event_ns : 1546383600000000000
last  ts_event_ns : 1546383719000000000
mid min/max       : 0.71005 0.71198

        ts_event_ns instrument_id  bid_price  ask_price  bid_size  ask_size
1546383600000000000   AUD/USD.SIM    0.71000    0.71010   1000000   1000000
1546383601000000000   AUD/USD.SIM    0.71002    0.71012   1000000   1000000
1546383602000000000   AUD/USD.SIM    0.71003    0.71013   1000000   1000000

elided rows       : 117

engine data ingested : 120
```

Three rows are shown and 117 are elided. The last line is the proof that matters: the engine counted
120 quotes after ingesting the file, so the format round-trips through the engine's own types.

## How these files would come from a real venue

| File                     | Real equivalent                                                                     |
| ------------------------ | ----------------------------------------------------------------------------------- |
| `quotes_audusd.csv`      | A recorded WebSocket market data stream, converted by an adapter into `QuoteTick`s. |
| `venue_fills_report.csv` | The venue's REST execution history, returned during startup reconciliation.         |
| `state_snapshots.csv`    | The bytes a database-backed cache wrote when the live node called `on_save`.        |

A realistic version of the market file is much larger and much messier. A real feed has gaps,
out-of-order updates, duplicate timestamps, and a mix of quote, trade, and book messages. The
committed file is deliberately small and clean so that every row can be checked by hand.

If you want a realistic order book shape rather than a quote series, the repository ships one at
`crates/adapters/tardis/test_data/csv/deltas_1.csv`. This manual does not use it, because production
operations is about control paths rather than about book microstructure.

## What to notice

Three details in these files matter for the rest of the manual.

1. The bid and ask in the market file are 0.00010 apart. That gap, widened by however much the price
   moves before your order arrives, is what a market order pays.
2. The venue report in `venue_fills_report.csv` nets to 200,000 units long: two buys of 100,000 and
   150,000, then a sell of 50,000. If your own records said 150,000, reconciliation would have to
   produce the missing 50,000 units. Lecture 07 covers that.
3. The state file shows `orders_denied` rising from 2 to 4 while `net_position_qty` stays at 200,000.
   Denied orders changed nothing about the position. That is the expected shape of a working limit.

Continue to [05 - Build the operational harness](05-build-the-strategy.md).
