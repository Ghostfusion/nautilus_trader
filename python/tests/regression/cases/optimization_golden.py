# -------------------------------------------------------------------------------------------------
#  Copyright (C) 2015-2026 Nautech Systems Pty Ltd. All rights reserved.
#  https://nautechsystems.io
#
#  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
#  You may not use this file except in compliance with the License.
#  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
#
#  Unless required by applicable law or agreed to in writing, software
#  distributed under the License is distributed on an "AS IS" BASIS,
#  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#  See the License for the specific language governing permissions and
#  limitations under the License.
# -------------------------------------------------------------------------------------------------
"""
Golden scenario for a declared optimization sweep over backtest runs.

The scenario drives the optimization through its configuration-file entry point, so it exercises
the same `load_config`/`run_config` path the `nautilus optimize` command and the notebook helper
use. It then reproduces the selected experiment by hand through `BacktestNode` and checks that the
hand run's canonical digest is the one the optimizer reported, which is the digest the scenario's
regression expectations pin.

The sweep is deterministic: the runner uses the experiment digest as the run config id, so the
same experiment always produces the same canonical result digest, and the scenario reproduces its
own digest without pinning a random id.

The catalog is written to a fresh temporary directory. The catalog path is not part of the
canonical result, so the digest does not depend on where the data lives.
"""

from __future__ import annotations

import json
import tempfile
from decimal import Decimal
from functools import lru_cache
from pathlib import Path
from typing import Any

from nautilus_trader.backtest import BacktestDataConfig
from nautilus_trader.backtest import BacktestEngineConfig
from nautilus_trader.backtest import BacktestNode
from nautilus_trader.backtest import BacktestRunConfig
from nautilus_trader.backtest import BacktestVenueConfig
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.model import BarType
from nautilus_trader.model import InstrumentId
from nautilus_trader.model import NautilusDataType
from nautilus_trader.optimization.config import CONFIG_SCHEMA
from nautilus_trader.optimization.config import parse_config
from nautilus_trader.optimization.config import run_config
from nautilus_trader.optimization.space import Experiment
from nautilus_trader.persistence import ParquetDataCatalog
from nautilus_trader.trading import ImportableStrategyConfig
from tests.providers import TestDataProvider
from tests.providers import TestInstrumentProvider
from tests.regression.scenario import Scenario


NAME = "optimization_golden"

STRATEGY = "strategies.ema_cross:EMACross"
STRATEGY_CONFIG = "strategies.ema_cross:EMACrossConfig"

INSTRUMENT = TestInstrumentProvider.btcusdt_binance()
INSTRUMENT_ID = str(INSTRUMENT.id)
BAR_TYPE = "BTCUSDT.BINANCE-1-MINUTE-LAST-EXTERNAL"
CSV_NAME = "btc-perp-20211231-20220201_1m.csv"
MAX_ROWS = 120
TRADE_SIZE = "0.010000"

SHARPE = "Sharpe Ratio (252 days)"
DRAWDOWN = "Max Drawdown"


def config_factory(start: int | None, end: int | None) -> tuple[list, list, BacktestEngineConfig]:
    """
    Build the venue, data, and engine configurations for one run window.

    This is the importable configuration factory the optimization configuration references, so the
    sweep performs no data or venue setup of its own.
    """
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
        catalog_path=str(_catalog_path()),
        instrument_id=InstrumentId.from_str(INSTRUMENT_ID),
        bar_types=[BAR_TYPE],
        start_time=start,
        end_time=end,
    )
    engine = BacktestEngineConfig(bypass_logging=True, run_analysis=False)
    return [venue], [data], engine


def config_document(*, store_directory: str | None = None) -> dict[str, Any]:
    """
    Build the optimization configuration document the scenario and the CLI both use.
    """
    document: dict[str, Any] = {
        "schema": CONFIG_SCHEMA,
        "strategy": {
            "strategy_path": STRATEGY,
            "config_path": STRATEGY_CONFIG,
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
    if store_directory is not None:
        document["store"] = {"directory": store_directory}
    return document


def write_config(path: Path, *, store_directory: str | None = None) -> Path:
    """
    Write the optimization configuration document to `path`.
    """
    path.write_text(
        json.dumps(config_document(store_directory=store_directory), indent=2, sort_keys=True),
        encoding="utf-8",
    )
    return path


def execute() -> Any:
    """
    Run the sweep through the config entry point and return the selected run's canonical result.

    The optimizer runs over four experiments and selects one. The selected experiment is then
    reproduced by hand through `BacktestNode`, and the hand run's canonical digest must equal the
    digest the optimizer reported, so the returned canonical result is exactly the one the sweep
    selected.
    """
    store_directory = tempfile.mkdtemp(prefix="nautilus-optimization-golden-store-")
    config = parse_config(config_document(store_directory=store_directory))
    document = run_config(config)

    assert document["status"] == "ok"
    assert document["failures"] == []
    assert document["result_count"] == 4

    best = document["best"]
    assert best is not None
    canonical = _hand_run(best["parameters"])
    assert canonical.digest() == best["canonical_digest"]
    return canonical


def _hand_run(parameters: dict[str, Any]) -> Any:
    """
    Reproduce one experiment by hand through `BacktestNode`, matching the optimizer's run config.
    """
    venues, data, engine = config_factory(None, None)
    experiment = Experiment(parameters)
    config = BacktestRunConfig(
        venues=list(venues),
        data=list(data),
        engine=engine,
        id=experiment.digest,
        raise_exception=True,
        dispose_on_completion=False,
    )
    node = BacktestNode([config])
    node.build()
    node.add_strategy_from_config(
        config.id,
        ImportableStrategyConfig(
            strategy_path=STRATEGY,
            config_path=STRATEGY_CONFIG,
            config=dict(parameters),
        ),
    )
    try:
        node.run()
        return node.get_engine_canonical_result(config.id)
    finally:
        node.dispose()


@lru_cache(maxsize=1)
def _catalog_path() -> Path:
    """
    Write a fresh instrument and minute-bar slice to a temporary Parquet catalog.
    """
    directory = Path(tempfile.mkdtemp(prefix="nautilus-optimization-golden-"))
    catalog = ParquetDataCatalog(str(directory))
    catalog.write_instruments([INSTRUMENT])
    catalog.write_bars(_bars())
    return directory


def _bars() -> list:
    return TestDataProvider.bars_from_binance_csv(
        INSTRUMENT,
        bar_type=BarType.from_str(BAR_TYPE),
        csv_name=CSV_NAME,
        max_rows=MAX_ROWS,
    )


SCENARIO = Scenario(name=NAME, execute=execute)
