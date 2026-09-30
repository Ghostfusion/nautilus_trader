"""
Example of parameter optimization over backtest runs, driven from a notebook.
"""

# ---
# jupyter:
#   jupytext:
#     formats: py:percent
#     text_representation:
#       extension: .py
#       format_name: percent
#       format_version: '1.3'
#       jupytext_version: 1.19.0
#   kernelspec:
#     display_name: Python 3 (ipykernel)
#     language: python
#     name: python3
# ---


# %% [markdown]
# # Parameter optimization over backtest runs
#
# Declare an optimization in a JSON configuration file, load it with the same
# `nautilus_trader.optimization.config` entry point the `nautilus optimize` command invokes, and
# print the best result. There is one optimization implementation: the notebook and the CLI drive
# the same parameter space, runner, objective, and stage.
#
# The configuration references an importable configuration factory that returns the venue, data,
# and engine configurations for a run window. Here the factory and the strategy live in this
# module, so the notebook is self-contained.
#
# The repository ships no ready-made Parquet catalog of bars, so the example first writes one to a
# temporary directory from the tracked minute-bar CSV, exactly as a backtest would read it.

# %%
from __future__ import annotations

import json
from decimal import Decimal
from pathlib import Path
from tempfile import TemporaryDirectory

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
    Quantity,
)
from nautilus_trader.optimization.config import CONFIG_SCHEMA, load_config, run_config
from nautilus_trader.persistence import ParquetDataCatalog
from nautilus_trader.testkit.providers import TestDataProvider, TestInstrumentProvider
from nautilus_trader.trading import Strategy, StrategyConfig

# %%
INSTRUMENT = TestInstrumentProvider.btcusdt_binance()
INSTRUMENT_ID = str(INSTRUMENT.id)
BAR_TYPE = "BTCUSDT.BINANCE-1-MINUTE-LAST-EXTERNAL"
CSV_NAME = "btc-perp-20211231-20220201_1m.csv"
MAX_ROWS = 120
TRADE_SIZE = "0.010000"
SHARPE = "Sharpe Ratio (252 days)"
DRAWDOWN = "Max Drawdown"

_CATALOG_PATH: Path | None = None


class SweepConfig(StrategyConfig):
    """
    Configure the EMA cross strategy under test.
    """

    def __init__(
        self,
        *,
        instrument_id: str,
        bar_type: str,
        trade_size: str,
        fast_ema_period: int,
        slow_ema_period: int,
    ) -> None:
        """
        Initialize the instance.
        """
        super().__init__()
        self.instrument_id = instrument_id
        self.bar_type = bar_type
        self.trade_size = trade_size
        self.fast_ema_period = fast_ema_period
        self.slow_ema_period = slow_ema_period


class SweepStrategy(Strategy):
    """
    Trade a long position while the fast EMA is above the slow EMA.
    """

    def __init__(self, config: SweepConfig) -> None:
        """
        Initialize the instance.
        """
        super().__init__(config)
        self._bar_type = BarType.from_str(config.bar_type)
        self._instrument_id = InstrumentId.from_str(config.instrument_id)
        self._quantity = Quantity.from_str(config.trade_size)
        self._fast = ExponentialMovingAverage(config.fast_ema_period)
        self._slow = ExponentialMovingAverage(config.slow_ema_period)
        self._long = False

    def on_start(self) -> None:
        """
        On start.
        """
        self.subscribe_bars(self._bar_type)

    def on_bar(self, bar: Bar) -> None:
        """
        On bar.
        """
        self._fast.handle_bar(bar)
        self._slow.handle_bar(bar)
        if not (self._fast.initialized and self._slow.initialized):
            return

        if self._fast.value > self._slow.value and not self._long:
            self._long = True
            self._submit(OrderSide.BUY)
        elif self._fast.value < self._slow.value and self._long:
            self._long = False
            self._submit(OrderSide.SELL)

    def _submit(self, side: OrderSide) -> None:
        """
        Submit one market order on the given side.
        """
        self.submit_order(
            self.order_factory.market(
                instrument_id=self._instrument_id,
                order_side=side,
                quantity=self._quantity,
            ),
        )


def config_factory(start: int | None, end: int | None) -> tuple[list, list, BacktestEngineConfig]:
    """
    Build the venue, data, and engine configurations for one run window.
    """
    assert _CATALOG_PATH is not None
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


def config_document() -> dict:
    """
    Build the optimization configuration document.
    """
    return {
        "schema": CONFIG_SCHEMA,
        "strategy": {
            "strategy_path": f"{__name__}:SweepStrategy",
            "config_path": f"{__name__}:SweepConfig",
            "config_factory": f"{__name__}:config_factory",
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
        "constraints": [{"metric": DRAWDOWN, "comparison": "at_least", "bound": -0.013}],
        "stage": {"kind": "optimize"},
        "concurrency": {"max_workers": 1},
    }


# %%
if __name__ == "__main__":
    with TemporaryDirectory() as tmp_dir:
        directory = Path(tmp_dir)
        _CATALOG_PATH = directory
        catalog = ParquetDataCatalog(str(directory))
        catalog.write_instruments([INSTRUMENT])
        catalog.write_bars(
            TestDataProvider.bars_from_binance_csv(
                INSTRUMENT,
                bar_type=BarType.from_str(BAR_TYPE),
                csv_name=CSV_NAME,
                max_rows=MAX_ROWS,
            ),
        )

        config_path = directory / "optimization.json"
        config_path.write_text(json.dumps(config_document(), indent=2, sort_keys=True))

        document = run_config(load_config(config_path))
        best = document["best"]

        print(f"experiments: {document['result_count']}, failures: {document['failure_count']}")
        print(f"best parameters: {json.dumps(best['parameters'], sort_keys=True)}")
        print(f"best score: {best['score']}")
        print(f"best canonical digest: {best['canonical_digest']}")
