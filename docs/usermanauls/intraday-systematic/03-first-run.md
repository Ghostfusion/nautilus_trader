# 03 - First run

This lecture gets one complete program running and explains every line of it. Take it slowly; the
program is short and everything in it is reused later.

## Setup

The environment for this repository is Git Bash on Windows with the virtual environment under
`python/.venv`. The commands below add the tools to the path and run a script with the project's
Python. Adjust the paths if your tools live elsewhere.

```bash
export PATH="C:/Users/vince/.cargo/bin;C:/Users/vince/.local/uv012;C:/Users/vince/AppData/Local/Programs/Python/Python312/cpython-3.14-windows-x86_64-none;$PATH"
cd python
uv run --no-sync python /tmp/first_run.py
```

The rule for this manual: write each program to a file outside the repository, run it, and read the
output. Never add `.py` files under `docs/`; the manual is markdown plus sample data.

## The smallest complete program

Save this as `/tmp/first_run.py` and run it with the command above.

```python
from decimal import Decimal

import pandas as pd

from nautilus_trader.backtest import BacktestEngine
from nautilus_trader.common import LoggerConfig
from nautilus_trader.config import BacktestEngineConfig
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.model import AccountType
from nautilus_trader.model import Currency
from nautilus_trader.model import Money
from nautilus_trader.model import OmsType
from nautilus_trader.model import Quantity
from nautilus_trader.model import TraderId
from nautilus_trader.model import Venue
from nautilus_trader.testkit.providers import TestDataProvider
from nautilus_trader.testkit.providers import TestInstrumentProvider
from nautilus_trader.trading import EmaCrossConfig

engine = BacktestEngine(
    BacktestEngineConfig(
        trader_id=TraderId.from_str("BACKTESTER-001"),
        logging=LoggerConfig(bypass_logging=True),
    ),
)

SIM = Venue("SIM")
USD = Currency.from_str("USD")
engine.add_venue(
    venue=SIM,
    oms_type=OmsType.NETTING,
    account_type=AccountType.MARGIN,
    base_currency=USD,
    starting_balances=[Money(1_000_000, USD)],
    fee_model=MakerTakerFeeModel(maker_rate=Decimal("0.00002"), taker_rate=Decimal("0.00002")),
)

USDJPY_SIM = TestInstrumentProvider.usdjpy_sim()
engine.add_instrument(USDJPY_SIM)
engine.add_data(TestDataProvider.usdjpy_quotes(count=10_000))
engine.add_builtin_strategy(
    "EmaCross",
    EmaCrossConfig(
        instrument_id=USDJPY_SIM.id,
        trade_size=Quantity.from_int(100_000),
        fast_period=10,
        slow_period=50,
    ),
)
engine.run()

with pd.option_context("display.max_rows", 100, "display.width", 220):
    print(engine.generate_order_fills_report()[["instrument_id", "side", "filled_qty", "avg_px", "commissions"]])

result = engine.get_result()
print("PnL (total):", result.stats_pnls["USD"]["PnL (total)"])
print("Win Rate:", result.stats_pnls["USD"]["Win Rate"])

engine.reset()
engine.dispose()
```

Its output, printed exactly as it appeared:

```text
                            instrument_id  side filled_qty   avg_px commissions
client_order_id
O-20190101-231332-001-001-1   USD/JPY.SIM  SELL     100000  109.999   [220 JPY]
O-20190101-233943-001-001-2   USD/JPY.SIM   BUY     100000  109.011   [218 JPY]
O-20190102-000554-001-001-3   USD/JPY.SIM  SELL     100000  109.999   [220 JPY]
O-20190102-003205-001-001-4   USD/JPY.SIM   BUY     100000  109.011   [218 JPY]
O-20190102-005815-001-001-5   USD/JPY.SIM  SELL     100000  109.999   [220 JPY]
O-20190102-012426-001-001-6   USD/JPY.SIM   BUY     100000  109.011   [218 JPY]
PnL (total): 2707.229999999865
Win Rate: 1.0
```

Stop and read that result before continuing. Six fills, three round trips, every one a winner. Do
not be impressed; this is a warning, not a success. The data is a smooth invented wave (lecture 04
explains it), and three round trips is far too few to mean anything. Lecture 06 shows how easily
this gets misread.

## Line by line

1. `from decimal import Decimal` imports a number type that stores decimals exactly. Prices and
   rates in this repository are decimals, not binary floating point, so `Decimal("0.00002")` is
   exact.
2. `import pandas as pd` imports the table library. Reports are returned as pandas tables.
3. The `nautilus_trader` imports bring in the engine, the config objects, the fee model, the model
   types, the offline data providers, and the builtin strategy's config.
4. `BacktestEngine(...)` creates the engine. `TraderId.from_str("BACKTESTER-001")` gives it an
   identity. `LoggerConfig(bypass_logging=True)` keeps the run quiet so the output is only the
   report; leave it out when you want the engine's log.
5. `SIM = Venue("SIM")` names the simulated venue. `Currency.from_str("USD")` is the account
   currency.
6. `engine.add_venue(...)` registers the venue with `NETTING` order management, a margin account,
   1,000,000 USD of starting balance, and a fee model charging 0.00002 of notional on both maker and
   taker fills.
7. `TestInstrumentProvider.usdjpy_sim()` returns a complete USD/JPY instrument definition for the
   simulated venue. `engine.add_instrument(...)` registers it.
8. `TestDataProvider.usdjpy_quotes(count=10_000)` builds 10,000 quote ticks in memory. It needs no
   network. Lecture 04 describes the series. `engine.add_data(...)` gives the ticks to the engine.
9. `engine.add_builtin_strategy("EmaCross", EmaCrossConfig(...))` adds a strategy that already
   exists in the repository. It is matched by name in `crates/backtest/src/python/engine.rs`, and
   the name `EmaCross` is one of the registered names in that function.
10. `engine.run()` replays the data. The engine delivers ticks, the strategy reacts, and the
    simulated venue fills orders.
11. `engine.generate_order_fills_report()` returns one row per filled order. The `[...]` selects the
    columns to print. `pd.option_context(...)` widens the display so no column is hidden.
12. `engine.get_result()` returns the run statistics. `stats_pnls["USD"]` is the profit and loss
    statistics in USD.
13. `engine.reset()` and `engine.dispose()` clean up.

## The builtin config fields

The four fields used above are exactly the fields of the builtin config, confirmed in
`crates/trading/src/examples/strategies/ema_cross/config.rs`:

| Field           | Meaning                             |
| --------------- | ----------------------------------- |
| `instrument_id` | The instrument to trade             |
| `trade_size`    | Order quantity, as a `Quantity`     |
| `fast_period`   | The fast average period, default 10 |
| `slow_period`   | The slow average period, default 50 |

The builtin strategy subscribes to quotes, not bars, and enters only on an actual crossover; the
code is in `crates/trading/src/examples/strategies/ema_cross/strategy.rs`. It never exits except by
an opposite crossover. That makes it a poor fit for the bar-driven rule this manual teaches, which
is why lecture 05 builds the rule from scratch. It is, however, the shortest complete program, and
that is its purpose here.

## A warning about the other data providers

Every `TestDataProvider.*_from_*_csv(...)` method downloads its CSV from GitHub on first use
(`python/nautilus_trader/testkit/providers.py`), so it needs the network. Examples that use them are
`examples/backtest/crypto_ema_cross_ethusdt_trade_ticks.py` and
`examples/backtest/fx_ema_cross_audusd_bars_from_ticks.py`; read them once you are comfortable, but
prefer the in-memory generators for offline work, or a committed CSV as in lecture 04.

Next: the data, in [04-sample-data](04-sample-data.md).
