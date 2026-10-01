# 03 - First run

In this lecture you write the smallest program that shows production operations working: a backtest
where a deliberately broken strategy fires a burst of orders and a risk engine, configured from
Python, refuses most of them. You see the refusal as a real event in the output.

## What you need

- The repository checkout.
- The Python environment under `python/.venv`.
- No network and no venue account.

## Step 1: prepare the shell

Open a Git Bash terminal and set the tool paths. This is the same environment the manual was
verified in.

```bash
export PATH="C:/Users/vince/.cargo/bin;C:/Users/vince/.local/uv012;C:/Users/vince/AppData/Local/Programs/Python/Python312/cpython-3.14-windows-x86_64-none;$PATH"
cd D:/Users/vince/PycharmProjects/nautilus_trader/python
uv --version
```

Expected output:

```text
uv 0.12.20 (2274b80d8 2026-09-28 x86_64-pc-windows-msvc)
```

## Step 2: look at the data you will use

The program uses an in-memory price generator, so it needs no files and no network. The generator is
`TestDataProvider.audusd_quotes(count)` in `python/nautilus_trader/testkit/providers.py`. It returns
QuoteTick objects with a bid price that follows a slow sine wave and an ask price 0.00010 above it.

:::warning
Every `TestDataProvider.*_from_*_csv(...)` method downloads its CSV from GitHub on first use, so it
needs the network. Prefer the in-memory generators, or a committed CSV. This manual never uses a
downloading method.
:::

## Step 3: write the program

Save this file outside the repository, for example at
`C:/Users/vince/AppData/Local/Temp/po_manual/ops_first_run.py`. The manual folder holds markdown and
CSV only, so do not put Python files under `docs/`.

```python
"""Production-operations first run: a runaway loop against Python-configurable risk limits."""

from __future__ import annotations

from decimal import Decimal

from nautilus_trader.backtest import BacktestEngine
from nautilus_trader.common import LogLevel
from nautilus_trader.common import LoggerConfig
from nautilus_trader.config import BacktestEngineConfig
from nautilus_trader.config import StrategyConfig
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.model import AccountType
from nautilus_trader.model import Currency
from nautilus_trader.model import Money
from nautilus_trader.model import OmsType
from nautilus_trader.model import OrderSide
from nautilus_trader.model import QuoteTick
from nautilus_trader.model import TraderId
from nautilus_trader.model import Venue
from nautilus_trader.risk import RiskEngineConfig
from nautilus_trader.testkit.providers import TestDataProvider
from nautilus_trader.testkit.providers import TestInstrumentProvider
from nautilus_trader.trading import Strategy

SIM = Venue("SIM")
USD = Currency.from_str("USD")
AUDUSD_SIM = TestInstrumentProvider.audusd_sim()
INSTRUMENT_ID = AUDUSD_SIM.id


class RunawayConfig(StrategyConfig):
    def __init__(self, instrument_id, burst_size: int, order_qty: str, **_kwargs) -> None:
        super().__init__()
        self.instrument_id = instrument_id
        self.burst_size = burst_size
        self.order_qty = order_qty


class RunawayLoop(Strategy):
    """Fires one burst of orders on the first quote tick, then goes quiet."""

    def __init__(self, config: RunawayConfig) -> None:
        super().__init__(config)
        self.denials: list[str] = []
        self.fired = False

    def on_start(self) -> None:
        self.subscribe_quotes(self.config.instrument_id)

    def on_quote(self, _tick: QuoteTick) -> None:
        if self.fired:
            return
        self.fired = True
        instrument = self.cache.instrument(self.config.instrument_id)
        for _ in range(self.config.burst_size):
            order = self.order_factory.market(
                self.config.instrument_id,
                OrderSide.BUY,
                instrument.make_qty(Decimal(self.config.order_qty)),
            )
            self.submit_order(order)

    def on_order_denied(self, event) -> None:
        self.denials.append(event.reason)


def run_case(title: str, risk: RiskEngineConfig, qty: str, burst: int = 6) -> None:
    print("=" * 68)
    print(title)
    print("=" * 68)
    engine = BacktestEngine(
        BacktestEngineConfig(
            trader_id=TraderId.from_str("OPS-001"),
            logging=LoggerConfig(stdout_level=LogLevel.WARNING, print_config=False),
            risk_engine=risk,
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
    engine.add_data(TestDataProvider.audusd_quotes(count=20))

    strategy = RunawayLoop(
        RunawayConfig(instrument_id=INSTRUMENT_ID, burst_size=burst, order_qty=qty),
    )
    engine.add_strategy(strategy)
    engine.run()

    print(f"orders submitted in burst : {burst}")
    print(f"orders denied             : {len(strategy.denials)}")
    for reason in strategy.denials:
        print(f"  denial reason           : {reason}")
    report = engine.generate_orders_report()
    if report is None:
        print("orders report             : (none)")
    else:
        print("orders report             :")
        print(report.to_string(index=False))
    print()

    engine.reset()
    engine.dispose()


run_case(
    "CASE 1: submit rate limit 2/00:00:01, notional cap 100,000 USD",
    RiskEngineConfig(
        max_order_submit_rate="2/00:00:01",
        max_notional_per_order={str(INSTRUMENT_ID): 100_000},
    ),
    qty="100000",
)

run_case(
    "CASE 2: submit rate limit 100/00:00:01, notional cap 100,000 USD",
    RiskEngineConfig(
        max_order_submit_rate="100/00:00:01",
        max_notional_per_order={str(INSTRUMENT_ID): 100_000},
    ),
    qty="1000000",
    burst=1,
)
```

## Step 4: run it

```bash
cd D:/Users/vince/PycharmProjects/nautilus_trader/python
uv run --no-sync python C:/Users/vince/AppData/Local/Temp/po_manual/ops_first_run.py
```

## Step 5: read the output

The output below is the real output of the command above, observed on 2026-10-01. The wide
31-column order report is elided where marked; lecture 06 reads it column by column.

```text
====================================================================
CASE 1: submit rate limit 2/00:00:01, notional cap 100,000 USD
====================================================================
2019-01-01T23:00:00.000000000Z [WARN] OPS-001.nautilus_risk::engine: SubmitOrder for O-20190101-230000-001-000-3 DENIED: RATE_LIMIT_EXCEEDED
2019-01-01T23:00:00.000000000Z [WARN] OPS-001.nautilus_trading::strategy: RunawayLoop-000 <--[EVT] OrderDenied(instrument_id=AUD/USD.SIM, client_order_id=O-20190101-230000-001-000-3, reason='RATE_LIMIT_EXCEEDED')
2019-01-01T23:00:00.000000000Z [WARN] OPS-001.nautilus_risk::engine: SubmitOrder for O-20190101-230000-001-000-4 DENIED: RATE_LIMIT_EXCEEDED
2019-01-01T23:00:00.000000000Z [WARN] OPS-001.nautilus_trading::strategy: RunawayLoop-000 <--[EVT] OrderDenied(instrument_id=AUD/USD.SIM, client_order_id=O-20190101-230000-001-000-4, reason='RATE_LIMIT_EXCEEDED')
2019-01-01T23:00:00.000000000Z [WARN] OPS-001.nautilus_risk::engine: SubmitOrder for O-20190101-230000-001-000-5 DENIED: RATE_LIMIT_EXCEEDED
2019-01-01T23:00:00.000000000Z [WARN] OPS-001.nautilus_trading::strategy: RunawayLoop-000 <--[EVT] OrderDenied(instrument_id=AUD/USD.SIM, client_order_id=O-20190101-230000-001-000-5, reason='RATE_LIMIT_EXCEEDED')
2019-01-01T23:00:00.000000000Z [WARN] OPS-001.nautilus_risk::engine: SubmitOrder for O-20190101-230000-001-000-6 DENIED: RATE_LIMIT_EXCEEDED
2019-01-01T23:00:00.000000000Z [WARN] OPS-001.nautilus_trading::strategy: RunawayLoop-000 <--[EVT] OrderDenied(instrument_id=AUD/USD.SIM, client_order_id=O-20190101-230000-001-000-6, reason='RATE_LIMIT_EXCEEDED')
orders submitted in burst : 6
orders denied             : 4
  denial reason           : RATE_LIMIT_EXCEEDED
  denial reason           : RATE_LIMIT_EXCEEDED
  denial reason           : RATE_LIMIT_EXCEEDED
  denial reason           : RATE_LIMIT_EXCEEDED
orders report             :
trader_id     strategy_id instrument_id side   type quantity status time_in_force  is_reduce_only  is_quote_quantity filled_qty                              init_id             ts_init             ts_last commissions venue_order_id emulation_trigger contingency_type order_list_id linked_order_ids parent_order_id exec_algorithm_id exec_algorithm_params exec_spawn_id tags account_id slippage                 position_id    liquidity_side          last_trade_id  avg_px
  OPS-001 RunawayLoop-000   AUD/USD.SIM  BUY MARKET   100000 FILLED           GTC           False              False     100000 db01c3c7-79a0-41d2-a2ef-4e9c426b5dbb 1546383600000000000 1546383600000000000  [0.00 USD]        SIM-1-1              None             None          None             None            None              None                  None          None None    SIM-001     None AUD/USD.SIM-RunawayLoop-000             TAKER T-65ff55bb95075e97-001 0.71010
  OPS-001 RunawayLoop-000   AUD/USD.SIM  BUY MARKET   100000 FILLED           GTC           False              False     100000 f1c0c98f-5d25-46b5-9b30-89b7d5e486e8 1546383600000000000 1546383600000000000  [0.00 USD]        SIM-1-2              None             None          None             None            None              None                  None          None None    SIM-001     None AUD/USD.SIM-RunawayLoop-000             TAKER T-65ff55bb95075e97-002 0.71010
  OPS-001 RunawayLoop-000   AUD/USD.SIM  BUY MARKET   100000 DENIED           GTC           False              False          0 b4df5df5-0f4b-4029-8d1c-de1eab5e9c0e 1546383600000000000 1546383600000000000        None            NaN              None             None          None             None            None              None                  None          None None        NaN     None                         NaN NO_LIQUIDITY_SIDE                    NaN     NaN
  ... four more DENIED rows ...
... columns is_reduce_only through avg_px elided for the remaining rows ...

====================================================================
CASE 2: submit rate limit 100/00:00:01, notional cap 100,000 USD
====================================================================
2019-01-01T23:00:00.000000000Z [WARN] OPS-001.nautilus_risk::engine: SubmitOrder for O-20190101-230000-001-000-1 DENIED: NOTIONAL_EXCEEDS_MAX_PER_ORDER: max=100000.00 USD, notional=710100.00 USD
2019-01-01T23:00:00.000000000Z [WARN] OPS-001.nautilus_trading::strategy: RunawayLoop-000 <--[EVT] OrderDenied(instrument_id=AUD/USD.SIM, client_order_id=O-20190101-230000-001-000-1, reason='NOTIONAL_EXCEEDS_MAX_PER_ORDER: max=100000.00 USD, notional=710100.00 USD')
orders submitted in burst : 1
orders denied             : 1
  denial reason           : NOTIONAL_EXCEEDS_MAX_PER_ORDER: max=100000.00 USD, notional=710100.00 USD
orders report             :
... one DENIED row, quantity 1000000, status DENIED ...
```

Two things happened.

In CASE 1 the burst was six orders and the rate limit admitted two. Orders 1 and 2 filled at the ask
price 0.71010, so the report shows `quantity 100000`, `status FILLED`, `filled_qty 100000`. Orders 3
to 6 were refused with `RATE_LIMIT_EXCEEDED` before reaching the simulated venue, so the report shows
`status DENIED` and `filled_qty 0`.

In CASE 2 the rate limit was wide open (100 per second), so the single order reached the notional
check. The engine compared the order's money value against the configured cap and refused it:

```text
max=100000.00 USD, notional=710100.00 USD
```

That is exactly the arithmetic from example 1 in lecture 01: 1,000,000 units times 0.71010 is
710,100 USD.

## Step 6: understand the program line by line

| Line or block                                          | What it does                                                                                     |
| ------------------------------------------------------ | ------------------------------------------------------------------------------------------------ |
| `RiskEngineConfig(...)`                                | Builds the risk engine configuration. This is the Python surface.                                |
| `max_order_submit_rate="2/00:00:01"`                   | Allows two order submissions per one-second interval.                                            |
| `max_notional_per_order={str(INSTRUMENT_ID): 100_000}` | Caps the money value of one order in that instrument at 100,000 of its quote currency, USD here. |
| `BacktestEngineConfig(risk_engine=risk)`               | Attaches that configuration to the engine's risk component.                                      |
| `LogLevel.WARNING`                                     | Prints only warnings and errors, so the refusal lines are visible.                               |
| `engine.add_venue(...)`                                | Defines the simulated venue, its account type, and its starting balance.                         |
| `fee_model=MakerTakerFeeModel(0, 0)`                   | A zero-fee model. A backtest venue requires an explicit fee model.                               |
| `engine.add_instrument(AUDUSD_SIM)`                    | Tells the engine the instrument's price and size precision.                                      |
| `engine.add_data(...)`                                 | Loads the twenty quote ticks the strategy will receive.                                          |
| `on_start` and `subscribe_quotes`                      | Registers the strategy for quote updates in that instrument.                                     |
| `on_quote`, guarded by `self.fired`                    | Runs the burst exactly once, on the first tick.                                                  |
| `instrument.make_qty(Decimal(...))`                    | Converts a decimal string into a `Quantity` at the instrument's precision.                       |
| `self.submit_order(order)`                             | Sends the command to the risk engine, which is the point of the exercise.                        |
| `on_order_denied`                                      | Receives each `OrderDenied` event and records its reason.                                        |
| `engine.generate_orders_report()`                      | Returns a table of every order, including denied ones.                                           |
| `engine.reset()` and `engine.dispose()`                | Releases the engine's resources between the two cases.                                           |

## Step 7: change one number

Change `max_order_submit_rate` in CASE 1 to `"10/00:00:01"` and run again. All six orders are
admitted, because six is below ten. Change it back. This is the whole point: the limit you choose is
a decision, and the engine does what you configured, not what you meant.

In lecture 05 you will meet limits that Python cannot configure at all.

Continue to [04 - Sample data](04-sample-data.md).