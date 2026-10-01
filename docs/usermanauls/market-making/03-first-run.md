# 03 - First run

This lecture gives you one complete program and explains every line. By the end you will have
printed three reports for a market making backtest on USD/JPY.

## Before you start

You need the environment this repository ships. From the repository root:

```bash
cd python
uv run --no-sync python -c "import nautilus_trader; print(nautilus_trader.__version__)"
```

If that prints a version, you are ready. Every command in this course runs from the `python`
directory, because that is where `uv` finds the project environment.

Write your scripts **outside** this repository, for example in a scratch directory. The manual is
markdown; it must not add `.py` files under `docs/`.

## The complete program

Save this as `mm_first_run.py` in your scratch directory.

```python
from decimal import Decimal

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
from nautilus_trader.model import Quantity
from nautilus_trader.model import TraderId
from nautilus_trader.model import Venue
from nautilus_trader.testkit.providers import TestDataProvider
from nautilus_trader.testkit.providers import TestInstrumentProvider
from nautilus_trader.trading import GridMarketMakerConfig

# 1. Create the engine.
engine = BacktestEngine(
    BacktestEngineConfig(
        trader_id=TraderId.from_str("BACKTESTER-001"),
        logging=LoggerConfig(stdout_level=LogLevel.OFF, print_config=False),
    ),
)

# 2. Add a simulated venue with a fill model and a fee model.
SIM = Venue("SIM")
USD = Currency.from_str("USD")
engine.add_venue(
    venue=SIM,
    oms_type=OmsType.NETTING,
    account_type=AccountType.MARGIN,
    base_currency=USD,
    starting_balances=[Money(10_000_000, USD)],
    fill_model=ProbabilisticFillModel(
        prob_fill_on_limit=1.0,
        prob_slippage=0.0,
        random_seed=42,
    ),
    fee_model=MakerTakerFeeModel(
        maker_rate=Decimal("0.00002"),
        taker_rate=Decimal("0.00002"),
    ),
)

# 3. Add the instrument and the quote data.
USDJPY_SIM = TestInstrumentProvider.usdjpy_sim()
engine.add_instrument(USDJPY_SIM)
engine.add_data(TestDataProvider.usdjpy_quotes(count=1_000))

# 4. Add the shipped grid market maker strategy.
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

# 5. Run the backtest.
engine.run()

# 6. Print the three reports.
with pd.option_context("display.max_columns", None, "display.width", 300):
    print(engine.generate_account_report(SIM).tail(3))
    print()
    print(
        engine.generate_order_fills_report()[
            ["side", "type", "quantity", "price", "filled_qty", "liquidity_side", "commissions"]
        ],
    )
    print()
    print(
        engine.generate_positions_report()[
            ["entry", "side", "quantity", "avg_px_open", "avg_px_close", "realized_pnl", "commissions"]
        ],
    )

engine.reset()
engine.dispose()
```

Run it:

```bash
cd python
uv run --no-sync python /path/to/your/scratch/mm_first_run.py
```

## The exact output

```text
                                total   locked        free currency account_id account_type                                            margins  reported info base_currency
2019-01-01 23:16:39+00:00  9999970.00  5997.07  9993972.93      USD    SIM-001       MARGIN  [{'type': 'MarginBalance', 'initial': '1498.38...     False   {}           USD
2019-01-01 23:16:39+00:00  9999970.00  4498.69  9995471.31      USD    SIM-001       MARGIN  [{'type': 'MarginBalance', 'initial': '0.00', ...     False   {}           USD
2019-01-01 23:16:39+00:00  9994569.62     0.00  9994569.62      USD    SIM-001       MARGIN                                                 []     False   {}           USD

                              side    type quantity    price filled_qty liquidity_side commissions
client_order_id
O-20190101-230000-001-001-2   SELL   LIMIT   500000  109.538     500000          MAKER  [1095 JPY]
O-20190101-230000-001-001-4   SELL   LIMIT   500000  109.571     500000          MAKER  [1096 JPY]
O-20190101-230000-001-001-6   SELL   LIMIT   500000  109.604     500000          MAKER  [1096 JPY]
O-20190101-231639-001-001-13   BUY  MARKET  1500000      NaN    1500000          TAKER  [3299 JPY]

                        entry  side quantity  avg_px_open  avg_px_close realized_pnl commissions
position_id
USD/JPY.SIM-GRID_MM-001  SELL  FLAT        0   109.571333       109.965  -597086 JPY  [6586 JPY]
```

This is deterministic. The same program with the same data produces the same bytes every time,
because both the quote generator (`python/nautilus_trader/testkit/providers.py:653`) and the fill
model's random draws are seeded.

## Line by line

### Step 1: the engine

`BacktestEngine(BacktestEngineConfig(trader_id=...))` builds the whole simulated system: a cache, a
message bus, an execution engine, a risk engine, a portfolio, and a data engine. `trader_id` is the
name your run is logged under.

`logging=LoggerConfig(stdout_level=LogLevel.OFF, print_config=False)` turns off the startup banner
and the per-order logs. The engine logs every command and event at `INFO` level by default, which is
useful when debugging but buries the numbers you want to see. Turning it off is not required; it
only makes the output readable. Leave it on when something misbehaves.

### Step 2: the venue

`add_venue(...)` creates the simulated exchange named `SIM`.

- `oms_type=OmsType.NETTING` creates one net position per instrument. Buying and selling the same
  instrument offset each other into a single inventory.
- `account_type=AccountType.MARGIN` allows short positions and leverage.
- `base_currency=USD` and `starting_balances=[Money(10_000_000, USD)]` start you with ten million
  dollars.
- `fill_model=ProbabilisticFillModel(prob_fill_on_limit=1.0, prob_slippage=0.0, random_seed=42)`
  makes every touched limit order fill with no slippage. This is the least noisy setting for a first
  run.
- `fee_model=MakerTakerFeeModel(maker_rate=Decimal("0.00002"), taker_rate=Decimal("0.00002"))`
  charges `0.002%` of value on each fill, maker or taker.

`Money(10_000_000, USD)` is ten million US dollars. `Decimal("0.00002")` is exact decimal
arithmetic, which matters for money; never use a binary float for a fee rate.

### Step 3: the instrument and the data

`TestInstrumentProvider.usdjpy_sim()` returns a complete USD/JPY instrument on the `SIM` venue,
including its tick size and lot size. `engine.add_instrument(...)` registers it.

`TestDataProvider.usdjpy_quotes(count=1_000)` returns 1,000 `QuoteTick` objects, one per second,
whose mid follows a sine wave between about `109.0` and `110.0`. `engine.add_data(...)` loads them
into the engine, sorted by timestamp. The first tick is at `2019-01-01T23:00:00Z`
(`python/nautilus_trader/testkit/providers.py:653`).

A warning you will meet again in lecture `04`: the other `TestDataProvider` methods that read CSVs,
such as `quotes_from_fxcm_bars`, download the file from GitHub on first use
(`python/nautilus_trader/testkit/providers.py:111`). Those need a network. The in-memory generators
do not.

### Step 4: the strategy

`engine.add_builtin_strategy("GridMarketMaker", GridMarketMakerConfig(...))` registers the shipped
strategy. Every field here is used by the strategy exactly as described in lecture `02`:

- `instrument_id` is what to quote.
- `max_position=Quantity.from_int(1_500_000)` is the hard cap on net inventory, long or short.
- `trade_size=Quantity.from_int(500_000)` is the size of each quoted level. With `num_levels=3`,
  the worst-case long is three buys, so the cap of 1,500,000 exactly fits three levels.
- `num_levels=3` places three buy levels below the mid and three sell levels above it.
- `grid_step_bps=3` spaces the levels `0.03%` apart.
- `skew_factor=0.0` does not shift the grid with inventory. Lecture `07` shows why you might set it.
- `requote_threshold_bps=20` re-quotes only when the mid has moved `0.20%` since the last grid. A
  larger number means fewer cancel-and-replace waves and more resting orders.

### Step 5: run

`engine.run()` replays all 1,000 quotes through the engine. Each quote reaches the strategy's
`on_quote` handler, which may cancel and replace the grid.

### Step 6: the reports

- `engine.generate_account_report(SIM)` returns one row per account-state change. It is long, so the
  program prints only the last three rows. `total` is the account value, `locked` is cash reserved
  against open orders and margin, and `free = total - locked`.
- `engine.generate_order_fills_report()` returns one row per order that filled. `liquidity_side`
  tells you whether each fill was a `MAKER` or a `TAKER`, and `commissions` shows the fee charged.
- `engine.generate_positions_report()` returns one row per position cycle. `FLAT` means the position
  was closed. `realized_pnl` is the profit or loss booked in the cost currency, JPY here.

`engine.reset()` and `engine.dispose()` free the engine. Call them when you are done; a long session
that creates many engines without disposing them accumulates memory.

## What the output means

Three of the four fills are your resting sells: `MAKER`, no slippage, at `109.538`, `109.571`, and
`109.604`. A single market buy of `1,500,000` at the end is the strategy's `on_stop` logic closing
the inventory it had accumulated. That closing order is a `TAKER` and pays `3299 JPY` of commission,
roughly three times a maker fee because it trades three times the size.

The position is `FLAT` with `realized_pnl = -597086 JPY`. The strategy bought high and sold low: the
average entry of the three sells was `109.571333` and the exit was `109.965`. This is normal and it
is the lesson. A grid that sells on the way up and buys on the way down loses money when the market
trends in one direction for long enough. Lecture `06` takes this position apart number by number,
and lecture `07` is about controlling it.

## Common first-run problems

- `ModuleNotFoundError: No module named 'nautilus_trader'`. You ran Python outside the project
  environment. Run `uv run --no-sync python ...` from the `python` directory.
- The run prints thousands of `INFO` lines. Remove the `logging=...` line is not the fix; keep it,
  or set a higher level. The reports are at the end either way.
- Nothing filled. A market maker only fills when its prices are touched. With `prob_fill_on_limit`
  below `1.0` some touches do not fill; with a large `requote_threshold_bps` and a large
  `grid_step_bps` the grid can sit far from the market and never be touched.

Continue to [04-sample-data.md](04-sample-data.md).

Previous: [02-the-engine-view.md](02-the-engine-view.md) | Next: [04-sample-data.md](04-sample-data.md)
