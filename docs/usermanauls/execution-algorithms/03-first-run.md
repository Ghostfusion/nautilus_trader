# 03 - First run

This lecture is the smallest complete program that slices an order with the native TWAP algorithm and
prints the spawned orders. Run it once as given, compare your output with the output below, then read
the line-by-line explanation.

## Step 1: set up the environment

Open a terminal. Every command in this manual is run from the repository root unless stated. First
set the tool path. On Windows with Git Bash, the ground-truth setup is:

```bash
export PATH="$HOME/.cargo/bin;$HOME/.local/uv012;$HOME/AppData/Local/Programs/Python/Python312/cpython-3.14-windows-x86_64-none;$PATH"
```

On Linux or macOS the equivalent is:

```bash
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"
```

Check that the package imports before writing any code:

```bash
cd python
uv run --no-sync python -c "import nautilus_trader; print(nautilus_trader.__version__)"
```

The package prints its version. If this fails, fix the environment before continuing; see the
[developer guide](../../developer_guide/environment_setup.md).

## Step 2: save the program

Save the program below as `twap_run.py` inside this manual folder
(`docs/usermanauls/execution-algorithms/`), next to the committed `sample_data/` directory. That
placement matters: the program reads the sample file relative to its own location. Do not commit
the scratch file; this manual ships only markdown and CSV.

## Step 3: the program

```python
from __future__ import annotations

import csv
from decimal import Decimal
from pathlib import Path

from nautilus_trader.backtest import BacktestEngine
from nautilus_trader.common import LogLevel
from nautilus_trader.config import BacktestEngineConfig
from nautilus_trader.config import ExecutionAlgorithmConfig
from nautilus_trader.config import LoggerConfig
from nautilus_trader.config import StrategyConfig
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.execution import ProbabilisticFillModel
from nautilus_trader.model import AccountType
from nautilus_trader.model import Currency
from nautilus_trader.model import ExecAlgorithmId
from nautilus_trader.model import InstrumentId
from nautilus_trader.model import Money
from nautilus_trader.model import OmsType
from nautilus_trader.model import OrderSide
from nautilus_trader.model import Price
from nautilus_trader.model import Quantity
from nautilus_trader.model import QuoteTick
from nautilus_trader.model import TraderId
from nautilus_trader.model import Venue
from nautilus_trader.testkit.providers import TestInstrumentProvider
from nautilus_trader.trading import Strategy

SAMPLE = Path(__file__).with_name("sample_data") / "usdjpy_quotes_sample.csv"


def load_quotes(path: Path, instrument_id: InstrumentId) -> list[QuoteTick]:
    quotes = []
    with path.open(newline="") as f:
        reader = csv.reader(f)
        next(reader)
        for ts_event_ns, bid, ask, bid_size, ask_size in reader:
            ts = int(ts_event_ns)
            quotes.append(
                QuoteTick(
                    instrument_id=instrument_id,
                    bid_price=Price(float(bid), precision=3),
                    ask_price=Price(float(ask), precision=3),
                    bid_size=Quantity.from_int(int(bid_size)),
                    ask_size=Quantity.from_int(int(ask_size)),
                    ts_event=ts,
                    ts_init=ts,
                ),
            )
    return quotes


class TwapDemoConfig(StrategyConfig):
    def __init__(
        self,
        *,
        instrument_id: InstrumentId,
        order_quantity: str,
        **_kwargs: object,
    ) -> None:
        super().__init__()
        self.instrument_id = instrument_id
        self.order_quantity = order_quantity


class TwapDemo(Strategy):
    def __init__(self, config: TwapDemoConfig) -> None:
        super().__init__(config)
        self._submitted = False

    def on_start(self) -> None:
        self.subscribe_quotes(self.config.instrument_id)

    def on_quote(self, _quote) -> None:
        if self._submitted:
            return
        self._submitted = True
        order = self.order_factory.market(
            instrument_id=self.config.instrument_id,
            order_side=OrderSide.BUY,
            quantity=Quantity.from_str(self.config.order_quantity),
            exec_algorithm_id=ExecAlgorithmId("TWAP"),
            exec_algorithm_params={"horizon_secs": "60", "interval_secs": "10"},
        )
        self.submit_order(order)

    def on_order_filled(self, event) -> None:
        print(
            f"FILL child={event.client_order_id} qty={event.last_qty} "
            f"px={event.last_px} ts={event.ts_event}",
        )


engine = BacktestEngine(
    BacktestEngineConfig(
        trader_id=TraderId.from_str("BACKTESTER-001"),
        logging=LoggerConfig(stdout_level=LogLevel.ERROR),
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
    fill_model=ProbabilisticFillModel(prob_fill_on_limit=0.2, prob_slippage=0.5, random_seed=42),
    fee_model=MakerTakerFeeModel(maker_rate=Decimal("0.00002"), taker_rate=Decimal("0.00002")),
)

engine.add_native_exec_algorithm(
    "TwapAlgorithm",
    ExecutionAlgorithmConfig(exec_algorithm_id=ExecAlgorithmId("TWAP")),
)

USDJPY_SIM = TestInstrumentProvider.default_fx_ccy("USD/JPY", SIM)
engine.add_instrument(USDJPY_SIM)

instrument_id = InstrumentId.from_str("USD/JPY.SIM")
engine.add_data(load_quotes(SAMPLE, instrument_id))
engine.add_strategy(TwapDemo(TwapDemoConfig(instrument_id=instrument_id, order_quantity="6000")))

engine.run()

print("--- orders in the cache ---")
for order in engine.cache.orders():
    print(
        order.client_order_id,
        order.order_type,
        order.side,
        order.quantity,
        order.status,
        "spawn:", order.exec_spawn_id,
    )

engine.dispose()
```

## Step 4: run it

From the repository root:

```bash
cd python
uv run --no-sync python ../docs/usermanauls/execution-algorithms/twap_run.py
```

## The output you should see

```text
FILL child=O-20190101-230000-001-000-1-E1 qty=1000 px=109.511 ts=1546383600000000000
FILL child=O-20190101-230000-001-000-1-E2 qty=1000 px=109.522 ts=1546383610000000000
FILL child=O-20190101-230000-001-000-1-E3 qty=1000 px=109.531 ts=1546383620000000000
FILL child=O-20190101-230000-001-000-1-E4 qty=1000 px=109.541 ts=1546383630000000000
FILL child=O-20190101-230000-001-000-1-E5 qty=1000 px=109.551 ts=1546383640000000000
FILL child=O-20190101-230000-001-000-1 qty=1000 px=109.561 ts=1546383650000000000
--- orders in the cache ---
O-20190101-230000-001-000-1 MARKET BUY 1000 FILLED spawn: O-20190101-230000-001-000-1
O-20190101-230000-001-000-1-E1 MARKET BUY 1000 FILLED spawn: O-20190101-230000-001-000-1
O-20190101-230000-001-000-1-E2 MARKET BUY 1000 FILLED spawn: O-20190101-230000-001-000-1
O-20190101-230000-001-000-1-E3 MARKET BUY 1000 FILLED spawn: O-20190101-230000-001-000-1
O-20190101-230000-001-000-1-E4 MARKET BUY 1000 FILLED spawn: O-20190101-230000-001-000-1
O-20190101-230000-001-000-1-E5 MARKET BUY 1000 FILLED spawn: O-20190101-230000-001-000-1
```

Six fills of 1,000 each. Five are children with client order ids ending `-E1` to `-E5`; the sixth is
the parent order itself, filled with its final slice. The parent ends `FILLED` with quantity `1000`,
which is its last remaining slice, not its original 6,000. This is the quantity conservation rule in
action: 5 children of 1,000 plus the parent's final 1,000 equals the original 6,000, and no quantity
is created or lost.

## Line by line

| Lines                                                                  | What they do                                                                                                      |
| ---------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| `from nautilus_trader.backtest import BacktestEngine`                  | The simulator that replays data and matches orders.                                                               |
| `from nautilus_trader.common import LogLevel` and `LoggerConfig`       | Set console logging to `ERROR` so the engine's informational logs do not bury the output.                         |
| `from nautilus_trader.execution import ...`                            | The fill model (how likely a resting order fills) and the fee model (maker and taker rates).                      |
| `from nautilus_trader.model import ...`                                | The value types: `Price`, `Quantity`, `QuoteTick`, ids, and enums.                                                |
| `from nautilus_trader.testkit.providers import TestInstrumentProvider` | Builds a standard USD/JPY instrument for the `SIM` venue.                                                         |
| `SAMPLE = Path(__file__).with_name("sample_data") / ...`               | The committed quote file next to this program.                                                                    |
| `def load_quotes(...)`                                                 | Reads the CSV and builds one `QuoteTick` per row. Column 1 is nanoseconds, 2 and 3 are prices, 4 and 5 are sizes. |
| `class TwapDemoConfig(StrategyConfig)`                                 | A config object that carries the instrument id and the order quantity into the strategy.                          |
| `class TwapDemo(Strategy)`                                             | The strategy. Its `on_start` subscribes to quotes; its `on_quote` submits the parent order once.                  |
| `exec_algorithm_id=ExecAlgorithmId("TWAP")`                            | Routes this order to the execution algorithm whose id is `TWAP`.                                                  |
| `exec_algorithm_params={"horizon_secs": "60", "interval_secs": "10"}`  | The schedule: 60 seconds of execution, a slice every 10 seconds. Both values are strings.                         |
| `on_order_filled`                                                      | Prints every fill, including the children the algorithm creates.                                                  |
| `engine.add_venue(...)`                                                | Adds the simulated venue with an explicit fill model and an explicit fee model.                                   |
| `engine.add_native_exec_algorithm(...)`                                | Registers the compiled-in TWAP algorithm under the id `TWAP` (`crates/backtest/src/python/engine.rs:510`).        |
| `engine.add_instrument(USDJPY_SIM)`                                    | Adds the instrument the orders reference.                                                                         |
| `engine.add_data(load_quotes(...))`                                    | Adds the quote ticks as the market data to replay.                                                                |
| `engine.add_strategy(TwapDemo(...))`                                   | Adds the strategy that will submit the parent order.                                                              |
| `engine.run()`                                                         | Runs the simulation to the end of the data.                                                                       |
| `engine.cache.orders()`                                                | Returns every order the engine has seen, parents and children.                                                    |
| `engine.dispose()`                                                     | Releases the engine, including threads. Always call this before the process exits.                                |

## Why `horizon_secs` is not always 60

TWAP computes the number of slices as `floor(horizon_secs / interval_secs)`, so the two values must be
consistent (`crates/trading/src/algorithm/twap.rs:274`). The concept page lists both keys at
[Execution algorithms](../../concepts/execution/algorithms.md), and the algorithm refuses a
`horizon_secs` smaller than `interval_secs` (`twap.rs:265`). Both values arrive as strings because
`exec_algorithm_params` is a mapping from string to string (`python/nautilus_trader/common/__init__.pyi:1336`).

## If your output differs

- A `ValueError: Backtest venue requires an explicit fee_model` means you left out
  `fee_model=MakerTakerFeeModel(...)` in `add_venue`.
- An `AttributeError` mentioning `on_start` usually means you put the submission in `on_start`
  instead of `on_quote`. The algorithm must be running before it can receive the order, and the
  earliest safe moment is the first quote.
- If the order sits `INITIALIZED`, the message bus never routed it. Check the `ExecAlgorithmId` in
  the order matches the one passed to `add_native_exec_algorithm`.

Next, read [04-sample-data.md](04-sample-data.md) to understand the quote file this program just
used.
