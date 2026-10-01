# 05 - Building the search

This lecture builds a complete search in numbered steps. It widens the grid from lecture 03, gives it
a seed so it repeats, persists its results so a resumed run skips them, declares a validation scheme,
and shows the search refusing a window it is not allowed to score.

Save the full program below as `search.py` in the repository root and run it the same way as lecture
03. It is the lecture 03 program with a different driver at the bottom, so the strategy, the
configuration factory and the data loader are unchanged.

## Step 1: declare a wider space

Three choices for each of two parameters gives nine experiments. The last parameter varies fastest,
so the expansion is (5, 20), (5, 30), (5, 40), (10, 20), and so on.

```python
space = ParameterSpace(
    base={
        "instrument_id": INSTRUMENT_ID,
        "bar_type": BAR_TYPE,
        "trade_size": TRADE_SIZE,
    },
    parameters=(
        Parameter("fast_ema_period", (5, 10, 15)),
        Parameter("slow_ema_period", (20, 30, 40)),
    ),
)
```

## Step 2: choose a seeded search strategy

A grid would evaluate all nine. A `RandomSearch` draws a seeded subset, so you can spend a smaller
budget and still repeat the run exactly.

```python
search = RandomSearch(seed=7, budget=6)
```

The same seed and the same space always select the same experiments in the same order
(`python/nautilus_trader/optimization/search.py`).

## Step 3: give the sweep a store and a cache

An `ExperimentStore` owns a directory and writes digest-keyed JSON. Passing it to `optimize` makes
the sweep memoized: an experiment whose parameter digest is already recorded is reused rather than
executed (`python/nautilus_trader/optimization/optimizer.py`).

```python
store = ExperimentStore(root / "store")
first = make_optimizer(search).optimize(space, store=store)
```

## Step 4: declare the validation scheme

The scheme says which window the search may score and which window it may not. Here the first 120
days are searched and the second 120 days are held out.

```python
scheme = ValidationScheme.single_split(
    search=(start, start + 120 * day),
    held_out=(start + 120 * day, end),
)
```

When a scheme is given, the optimizer refuses a run whose window does not lie inside one of the
scheme's search windows, so a held-out window can never be scored by the search
(`python/nautilus_trader/optimization/optimizer.py`, `run.py`).

## Step 5: describe the run

A `RunDescription` records the space digest, the validation scheme, the seed, the search strategy and
the cache digest. Its digest identifies the run, so two runs are comparable only when their
descriptions are.

```python
description = RunDescription.of(space, scheme, search=search, cache=cache)
```

## Step 6: the complete program

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
    EvaluationCache,
    ExperimentStore,
    GridSearch,
    Optimizer,
    Parameter,
    ParameterSpace,
    RandomSearch,
    RunDescription,
    ValidationScheme,
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
        root = Path(tmp)
        _CATALOG_PATH = root / "catalog"
        _CATALOG_PATH.mkdir()
        catalog = ParquetDataCatalog(str(_CATALOG_PATH))
        catalog.write_instruments([INSTRUMENT])
        catalog.write_bars(load_bars())

        bars = load_bars()
        start = bars[0].ts_event
        end = bars[-1].ts_event + 86_400_000_000_000
        day = 86_400_000_000_000

        space = ParameterSpace(
            base={
                "instrument_id": INSTRUMENT_ID,
                "bar_type": BAR_TYPE,
                "trade_size": TRADE_SIZE,
            },
            parameters=(
                Parameter("fast_ema_period", (5, 10, 15)),
                Parameter("slow_ema_period", (20, 30, 40)),
            ),
        )

        def make_optimizer(search):
            return Optimizer(
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
                search=search,
            )

        search = RandomSearch(seed=7, budget=6)
        store = ExperimentStore(root / "store")
        first = make_optimizer(search).optimize(space, store=store)
        print("run 1: seed", search.seed, "evaluated", first.evaluated, "of", first.space_size)
        print("run 1: executions", first.executions, "failures", len(first.failures))
        print("run 1: order", [d[-8:] for d in first.digests])

        second = make_optimizer(search).optimize(space, store=ExperimentStore(root / "store2"))
        print("run 2: order", [d[-8:] for d in second.digests])
        print("run 2: same order as run 1:", first.digests == second.digests)

        resumed = make_optimizer(search).optimize(space, store=store)
        print("resumed: evaluated", resumed.evaluated, "executions", resumed.executions)
        cache = EvaluationCache(store)
        print("resumed: cache holds", len(cache), "evaluations")

        other = make_optimizer(RandomSearch(seed=99, budget=6)).optimize(
            space,
            store=ExperimentStore(root / "store3"),
        )
        print("seed 99: order", [d[-8:] for d in other.digests])

        scheme = ValidationScheme.single_split(
            search=(start, start + 120 * day),
            held_out=(start + 120 * day, end),
        )
        description = RunDescription.of(space, scheme, search=search, cache=cache)
        print("description digest stable:", description.digest == RunDescription.of(space, scheme, search=search, cache=cache).digest)
        print("description digest", description.digest[:20])

        held_out_optimizer = make_optimizer(search).with_window(start + 120 * day, end)
        try:
            held_out_optimizer.optimize(space, scheme=scheme)
        except ValueError as exc:
            print("refused:", exc)

        inside = make_optimizer(search).with_window(start, start + 120 * day)
        accepted = inside.optimize(space, cache=EvaluationCache(), scheme=scheme)
        print("accepted: evaluated", accepted.evaluated, "scheme digest", accepted.scheme.digest[:20])
```

Command:

```bash
uv run --project python --no-sync python search.py
```

## The output

```text
run 1: seed 7 evaluated 6 of 9
run 1: executions 6 failures 0
run 1: order ['462d738d', '6ea5e04b', 'b6146026', 'c666b92d', '55d700f6', 'fd9da49e']
run 2: order ['462d738d', '6ea5e04b', 'b6146026', 'c666b92d', '55d700f6', 'fd9da49e']
run 2: same order as run 1: True
resumed: evaluated 6 executions 0
resumed: cache holds 6 evaluations
seed 99: order ['f60e860e', 'b6146026', '8cecaeb5', '6de5edfe', '55d700f6', 'fd9da49e']
description digest stable: True
description digest sha256:0cc511ea830b1
refused: run window (1651276800000000000, 1661644800000000000) falls inside the scheme's held-out window (1651276800000000000, 1661644800000000000), so it would be scored by the search
accepted: evaluated 6 scheme digest sha256:1e04e723c589f
```

## Reading the output

- **The seed repeats the run.** Run 1 and run 2 use seed 7 and a fresh store each. Their digest orders
  are identical, so the same seed selected the same experiments in the same order. The suffixes are
  the last eight characters of each canonical result digest.
- **The resume skips finished work.** The resumed run reports `executions 0`: all six evaluations
  were already in the store, so no backtest ran. `cache holds 6 evaluations` is the cache count. In
  lecture 03 the first run had `executions 4` for four experiments; here the count is the evidence.
- **A different seed selects a different subset.** Seed 99 visits six experiments but two of them
  differ from seed 7's selection. The overlap is expected: two samples of six from nine share
  entries.
- **The description is stable.** The same declaration gives the same digest twice.
- **The search refuses a held-out window.** The refused window is exactly the scheme's held-out
  window, so the search would have been scoring a window the scheme reserved. The message names the
  two bounds, so the fix is to window the runner inside the search window, which the last line does:
  the accepted run reports the scheme digest it ran under.

## The configuration-file route

The same workflow can be declared as data and run through the package's own entry point, which is
exactly what the `nautilus optimize` command invokes. This script writes a JSON document and runs it
with `load_config` and `run_config`; it is the same entry point the
`examples/backtest/notebooks/optimization_sweep.py` helper uses. Add these imports to the lecture 05
program, and drop the unused `nautilus_trader.optimization` import block:

```python
import json

from nautilus_trader.optimization.config import CONFIG_SCHEMA, load_config, run_config
```

```python
def config_document() -> dict:
    return {
        "schema": CONFIG_SCHEMA,
        "strategy": {
            "strategy_path": "__main__:EmaCrossStrategy",
            "config_path": "__main__:EmaCrossConfig",
            "config_factory": "__main__:config_factory",
        },
        "space": {
            "base": {
                "instrument_id": INSTRUMENT_ID,
                "bar_type": BAR_TYPE,
                "trade_size": TRADE_SIZE,
            },
            "parameters": [
                {"name": "fast_ema_period", "choices": [5, 10]},
                {"name": "slow_ema_period", "choices": [20, 30]},
            ],
        },
        "window": {"start": None, "end": None},
        "objective": {
            "terms": [
                {"metric": SHARPE, "weight": 1.0, "direction": "maximize"},
                {"metric": DRAWDOWN, "weight": 1.0, "direction": "maximize"},
            ],
        },
        "constraints": [{"metric": DRAWDOWN, "comparison": "at_least", "bound": -0.05}],
        "stage": {"kind": "optimize"},
        "concurrency": {"max_workers": 1},
    }


if __name__ == "__main__":
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        _CATALOG_PATH = root / "catalog"
        _CATALOG_PATH.mkdir()
        catalog = ParquetDataCatalog(str(_CATALOG_PATH))
        catalog.write_instruments([INSTRUMENT])
        catalog.write_bars(load_bars())

        config_path = root / "optimization.json"
        config_path.write_text(json.dumps(config_document(), indent=2, sort_keys=True))

        document = run_config(load_config(config_path))
        best = document["best"]
        print("status:", document["status"], "stage:", document["stage"])
        print("result_count:", document["result_count"], "failure_count:", document["failure_count"])
        print("best parameters:", json.dumps(best["parameters"], sort_keys=True))
        print("best score:", best["score"])
        print("best canonical digest:", best["canonical_digest"])
```

The observed output:

```text
status: ok stage: optimize
result_count: 4 failure_count: 0
best parameters: {"bar_type": "BTCUSDT.BINANCE-1-DAY-LAST-EXTERNAL", "fast_ema_period": 5, "instrument_id": "BTCUSDT.BINANCE", "slow_ema_period": 30, "trade_size": "1.000000"}
best score: 9.075712961025893
best canonical digest: blake3:8737ccc8c632650188280d837a0dcfa5bfac248a38d8fadee53932f9f60e860e
```

The command-line equivalent resolves a Python interpreter and invokes
`nautilus_trader.optimization.config` over the file, and the child's JSON document goes straight to
standard output (`docs/concepts/optimization.md`, "The `optimize` command"). Run directly against a
config whose strategy and factory live in an importable module, the entry point prints one document
that also carries the resolved execution assumptions:

```text
status ok stage optimize schema nautilus.optimization.cli/v1
assumptions: {"bar_execution": true, "gap_handling": "market_price_beyond_trigger", "intrabar_path": "ohlc_sequence", "policy_id": "bar_ohlc", "trigger_fill": "trigger_price_inside_bar", "trigger_precedence": "ohlc_sequence", "version": 1}
result_count 4 failure_count 0
best parameters: {"bar_type": "BTCUSDT.BINANCE-1-DAY-LAST-EXTERNAL", "fast_ema_period": 5, "instrument_id": "BTCUSDT.BINANCE", "slow_ema_period": 30, "trade_size": "1.000000"}
best score: 9.075712961025893
best metric_values: {"Max Drawdown": -0.006376486015939637, "Sharpe Ratio (252 days)": 9.082089447041833}
best canonical_digest: blake3:8737ccc8c632650188280d837a0dcfa5bfac248a38d8fadee53932f9f60e860e
best experiment_digest: sha256:8634e236b1aad0449848ff73209a26ad834c873aabc3165fb62bd9e3c9f24cbb
```

The `assumptions` block is the declared default: bar execution with the fixed Open, High, Low, Close
sequence, `policy_id` `bar_ohlc`, version 1
(`python/nautilus_trader/optimization/assumptions.py`). The emitted result states the rule set it was
produced under, so two results produced under different policies are distinguishable by their policy
digest.
