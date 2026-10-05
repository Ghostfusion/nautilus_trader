# 03 - Your first sweep

This lecture runs the smallest complete parameter search: one strategy, two parameters, two choices
each, four experiments. At the end you will have a ranked report and you will know exactly what each
number in it means.

## Setup

Open a terminal in the repository root and put the environment on the path (Git Bash on Windows):

```bash
export PATH="$HOME/.cargo/bin;$HOME/.local/uv012;$HOME/AppData/Local/Programs/Python/Python312/cpython-3.14-windows-x86_64-none;$PATH"
```

Save the program below as `first_run.py` in the repository root, then run it from the repository
root, because the program reads the committed sample data by its path relative to the root:

```bash
uv run --project python --no-sync python first_run.py
```

`uv run` uses the environment in `python/` where `nautilus_trader` is installed. The
`--no-sync` flag reuses that environment instead of resolving it again.

## The program

```python
from __future__ import annotations

import csv
import tempfile
from decimal import Decimal
from pathlib import Path

from nautilus_trader.analysis import (
    Constraint,
    ConstraintComparison,
    Objective,
    ObjectiveDirection,
    ObjectiveTerm,
)
from nautilus_trader.backtest import (
    BacktestDataConfig,
    BacktestEngineConfig,
    BacktestVenueConfig,
)
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.indicators import ExponentialMovingAverage
from nautilus_trader.model import (
    Bar,
    BarType,
    InstrumentId,
    NautilusDataType,
    OrderSide,
    Price,
    Quantity,
)
from nautilus_trader.optimization import (
    BacktestRunner,
    ConcurrencyPolicy,
    GridSearch,
    Optimizer,
    Parameter,
    ParameterSpace,
)
from nautilus_trader.persistence import ParquetDataCatalog
from nautilus_trader.testkit.providers import TestInstrumentProvider
from nautilus_trader.trading import Strategy, StrategyConfig

SAMPLE_CSV = Path("docs/usermanauls/ai-training/sample_data/btcusdt-1d-bars.csv")
INSTRUMENT = TestInstrumentProvider.btcusdt_binance()
INSTRUMENT_ID = str(INSTRUMENT.id)
BAR_TYPE = "BTCUSDT.BINANCE-1-DAY-LAST-EXTERNAL"
TRADE_SIZE = "1.000000"
SHARPE = "Sharpe Ratio (252 days)"
DRAWDOWN = "Max Drawdown"

_CATALOG_PATH: Path | None = None


class EmaCrossConfig(StrategyConfig):
    def __init__(
        self,
        *,
        instrument_id: str,
        bar_type: str,
        trade_size: str,
        fast_ema_period: int,
        slow_ema_period: int,
    ) -> None:
        super().__init__()
        self.instrument_id = instrument_id
        self.bar_type = bar_type
        self.trade_size = trade_size
        self.fast_ema_period = fast_ema_period
        self.slow_ema_period = slow_ema_period


class EmaCrossStrategy(Strategy):
    def __init__(self, config: EmaCrossConfig) -> None:
        super().__init__(config)
        self._bar_type = BarType.from_str(config.bar_type)
        self._instrument_id = InstrumentId.from_str(config.instrument_id)
        self._quantity = Quantity.from_str(config.trade_size)
        self._fast = ExponentialMovingAverage(config.fast_ema_period)
        self._slow = ExponentialMovingAverage(config.slow_ema_period)
        self._long = False

    def on_start(self) -> None:
        self.subscribe_bars(self._bar_type)

    def on_bar(self, bar: Bar) -> None:
        self._fast.handle_bar(bar)
        self._slow.handle_bar(bar)
        if not (self._fast.initialized and self._slow.initialized):
            return
        if self._fast.value > self._slow.value and not self._long:
            self._long = True
            self.submit_order(
                self.order_factory.market(
                    instrument_id=self._instrument_id,
                    order_side=OrderSide.BUY,
                    quantity=self._quantity,
                ),
            )
        elif self._fast.value < self._slow.value and self._long:
            self._long = False
            self.submit_order(
                self.order_factory.market(
                    instrument_id=self._instrument_id,
                    order_side=OrderSide.SELL,
                    quantity=self._quantity,
                ),
            )


def config_factory(start, end):
    venue = BacktestVenueConfig(
        name="BINANCE",
        oms_type="NETTING",
        account_type="CASH",
        starting_balances=["10 BTC", "10_000_000 USDT"],
        book_type="L1_MBP",
        fee_model=MakerTakerFeeModel(maker_rate=Decimal("0.001"), taker_rate=Decimal("0.001")),
    )
    data = BacktestDataConfig(
        data_type=NautilusDataType.Bar,
        catalog_path=str(_CATALOG_PATH),
        instrument_id=InstrumentId.from_str(INSTRUMENT_ID),
        bar_types=[BAR_TYPE],
        start_time=start,
        end_time=end,
    )
    engine = BacktestEngineConfig(bypass_logging=True, run_analysis=False)
    return [venue], [data], engine


def load_bars() -> list[Bar]:
    bar_type = BarType.from_str(BAR_TYPE)
    bars = []
    with SAMPLE_CSV.open() as handle:
        for row in csv.DictReader(handle):
            ts = int(row["ts_event_ns"])
            bars.append(
                Bar(
                    bar_type=bar_type,
                    open=Price(float(row["open_price"]), precision=2),
                    high=Price(float(row["high_price"]), precision=2),
                    low=Price(float(row["low_price"]), precision=2),
                    close=Price(float(row["close_price"]), precision=2),
                    volume=Quantity(float(row["volume_size"]), precision=6),
                    ts_event=ts,
                    ts_init=ts,
                ),
            )
    return bars


if __name__ == "__main__":
    with tempfile.TemporaryDirectory() as tmp:
        _CATALOG_PATH = Path(tmp)
        catalog = ParquetDataCatalog(tmp)
        catalog.write_instruments([INSTRUMENT])
        catalog.write_bars(load_bars())

        space = ParameterSpace(
            base={
                "instrument_id": INSTRUMENT_ID,
                "bar_type": BAR_TYPE,
                "trade_size": TRADE_SIZE,
            },
            parameters=(
                Parameter("fast_ema_period", (5, 10)),
                Parameter("slow_ema_period", (20, 30)),
            ),
        )
        optimizer = Optimizer(
            runner=BacktestRunner(
                config_factory=config_factory,
                strategy_path="__main__:EmaCrossStrategy",
                config_path="__main__:EmaCrossConfig",
            ),
            objective=Objective(
                [
                    ObjectiveTerm(SHARPE, 1.0, ObjectiveDirection.MAXIMIZE),
                    ObjectiveTerm(DRAWDOWN, 1.0, ObjectiveDirection.MAXIMIZE),
                ],
            ),
            constraints=(Constraint(DRAWDOWN, ConstraintComparison.AT_LEAST, -0.05),),
            concurrency=ConcurrencyPolicy(max_workers=1),
            search=GridSearch(),
        )
        report = optimizer.optimize(space)
        best = report.best()
        print(f"evaluated={report.evaluated} space_size={report.space_size} executions={report.executions}")
        print(f"evaluated_fraction={report.evaluated_fraction}")
        print(f"failures={len(report.failures)}")
        print(f"best_parameters={dict(best.experiment.parameters)}")
        print(f"best_score={best.score:.6f}")
        print(f"best_feasible={report.best_feasible() is not None}")
```

## Expected output

```text
evaluated=4 space_size=4 executions=4
evaluated_fraction=1.0
failures=0
best_parameters={'instrument_id': 'BTCUSDT.BINANCE', 'bar_type': 'BTCUSDT.BINANCE-1-DAY-LAST-EXTERNAL', 'trade_size': '1.000000', 'fast_ema_period': 5, 'slow_ema_period': 30}
best_score=9.075713
best_feasible=True
```

`evaluated` is the number of experiments the sweep scored. `space_size` is the size of the declared
space. `executions` is the number actually run. On a first run without a cache, `evaluated` and
`executions` are equal. `evaluated_fraction` is `evaluated / space_size`. `failures` counts
experiments that raised or could not be scored. `best_parameters` is the winning parameter set,
`best_score` its objective value, and `best_feasible` reports whether the winning entry satisfies
every constraint.

## Line by line

- `SAMPLE_CSV` is the committed sample data, read relative to the repository root. Lecture 04
  describes it.
- `TestInstrumentProvider.btcusdt_binance()` builds the instrument the bars belong to. Its
  `price_precision` is 2 and its `size_precision` is 6, which is why the prices below are written to
  two decimals and the quantities to six.
- `BAR_TYPE` names the bar series. The `-1-DAY-LAST-EXTERNAL` suffix means one-day bars, priced by
  the last trade, aggregated outside the engine.
- `_CATALOG_PATH` is filled in when the program starts. The configuration factory reads it, because
  a `BacktestDataConfig` reads data from a Parquet catalog, not from a list in memory.
- `EmaCrossConfig` carries the four strategy settings. Two of them, the EMA lengths, are the
  parameters the sweep varies; the other two are carried unchanged in the space's `base`.
- `EmaCrossStrategy` subscribes to the bars, feeds each bar to both exponential moving averages, and
  submits one market order when the fast average crosses the slow one. It holds at most one position
  and never shorts, so the position is either flat or long.
- `config_factory(start, end)` returns the venue configuration, the data configuration and the engine
  configuration for one run window. It is a module-level function because a worker process must be
  able to import it, which is why the runner takes it as a callable rather than running it inline.
- `load_bars()` reads the CSV and builds engine `Bar` objects. Prices and quantities are decimal
  values with an explicit precision; the timestamp is integer nanoseconds.
- `ParameterSpace(base=..., parameters=...)` declares the sweep. The `base` keys must not collide
  with parameter names, and expansion visits the parameters in declaration order with the last one
  varying fastest. Four experiments follow: (5, 20), (5, 30), (10, 20), (10, 30).
- `Objective` and `Constraint` come from `nautilus_trader.analysis`. The objective maximises the
  annualised Sharpe ratio and the (negative) maximum drawdown; the constraint requires the drawdown
  to be at least -0.05, that is, no worse than a five percent fall.
- `BacktestRunner(...)` holds the factory and the strategy's import paths. `GridSearch()` enumerates
  the whole space.
- `ConcurrencyPolicy(max_workers=1)` runs the sweep in-process, sequentially. This matters here: the
  strategy is defined in the `__main__` module, which a separate worker process cannot import. A
  module-level strategy in an importable module can use more workers; lecture 07 covers the memory
  cost of that.
- `optimizer.optimize(space)` runs the sweep and returns a `SearchReport`. `report.best()` returns
  the highest-scoring survivor.

## What to notice

The best parameter set is `fast=5, slow=30`. With only four experiments this is not surprising and
it is not evidence of anything: a better pair is very likely to exist in the values you did not try,
and lecture 01 showed that the best of a small search already looks better than it is. The next two
lectures widen the search and then measure the search honestly.

The run also shows the engine composes a complete backtest per experiment. Nothing in this program
touches the matching engine, the fill model or the fee model directly; it supplies configurations
and the runner does the rest. That is the boundary described in `docs/concepts/optimization.md`.
