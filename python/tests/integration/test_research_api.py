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
Prove the research path computes the same indicator values as a backtest of the same data.

A temporary Parquet catalog holds one instrument and a slice of minute bars. A `BacktestNode` runs
a strategy that feeds the bars to an `ExponentialMovingAverage` from the existing indicator API and
records its value after every bar. Independently, the research module loads the same catalog bars
and runs the same indicator class over the replayed data. The two value sequences must be
identical, which is the equality the design's W9 risk mitigation calls for.
"""

from __future__ import annotations

from decimal import Decimal
from pathlib import Path

from nautilus_trader.analysis.research import ResearchData
from nautilus_trader.analysis.research import compute_indicator
from nautilus_trader.backtest import BacktestDataConfig
from nautilus_trader.backtest import BacktestEngineConfig
from nautilus_trader.backtest import BacktestNode
from nautilus_trader.backtest import BacktestRunConfig
from nautilus_trader.backtest import BacktestVenueConfig
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.indicators import ExponentialMovingAverage
from nautilus_trader.model import Bar
from nautilus_trader.model import BarType
from nautilus_trader.model import InstrumentId
from nautilus_trader.model import NautilusDataType
from nautilus_trader.persistence import ParquetDataCatalog
from nautilus_trader.trading import Strategy
from nautilus_trader.trading import StrategyConfig
from tests.providers import TestDataProvider
from tests.providers import TestInstrumentProvider


INSTRUMENT = TestInstrumentProvider.btcusdt_binance()
INSTRUMENT_ID: InstrumentId = INSTRUMENT.id
BAR_TYPE = BarType.from_str(f"{INSTRUMENT_ID}-1-MINUTE-LAST-EXTERNAL")
CSV_NAME = "btc-perp-20211231-20220201_1m.csv"
MAX_ROWS = 120
EMA_PERIOD = 10


class EmaRecordingConfig(StrategyConfig):
    """
    Configure the EMA-recording strategy.
    """

    def __init__(self, *, bar_type: str, period: int) -> None:
        """
        Initialize the instance.
        """
        super().__init__()
        self.bar_type = bar_type
        self.period = period


class EmaRecordingStrategy(Strategy):
    """
    Feed every delivered bar to an exponential moving average and record its value.
    """

    def __init__(self, config: EmaRecordingConfig) -> None:
        """
        Initialize the instance.
        """
        super().__init__(config)
        self._bar_type = BarType.from_str(config.bar_type)
        self._ema = ExponentialMovingAverage(config.period)
        self.values: list[float] = []

    def on_start(self) -> None:
        """
        On start.
        """
        self.subscribe_bars(self._bar_type)

    def on_bar(self, bar: Bar) -> None:
        """
        On bar.
        """
        self._ema.handle_bar(bar)
        self.values.append(self._ema.value)


def _bars() -> list[Bar]:
    return TestDataProvider.bars_from_binance_csv(
        INSTRUMENT,
        bar_type=BAR_TYPE,
        csv_name=CSV_NAME,
        max_rows=MAX_ROWS,
    )


def _write_catalog(catalog_path: Path) -> None:
    catalog = ParquetDataCatalog(str(catalog_path))
    catalog.write_instruments([INSTRUMENT])
    catalog.write_bars(_bars())


def _run_backtest(catalog_path: Path) -> list[float]:
    venue = BacktestVenueConfig(
        name="BINANCE",
        oms_type="NETTING",
        account_type="CASH",
        starting_balances=["1_000_000 USDT"],
        book_type="L1_MBP",
        fee_model=MakerTakerFeeModel(maker_rate=Decimal(0), taker_rate=Decimal(0)),
    )
    data = BacktestDataConfig(
        data_type=NautilusDataType.Bar,
        catalog_path=str(catalog_path),
        instrument_id=INSTRUMENT_ID,
        bar_types=[str(BAR_TYPE)],
    )
    config = BacktestRunConfig(
        venues=[venue],
        data=[data],
        engine=BacktestEngineConfig(bypass_logging=True, run_analysis=False),
        dispose_on_completion=False,
    )
    node = BacktestNode([config])
    node.build()
    strategy = EmaRecordingStrategy(
        EmaRecordingConfig(bar_type=str(BAR_TYPE), period=EMA_PERIOD),
    )
    node.add_strategy(config.id, strategy)
    try:
        node.run()
    finally:
        node.dispose()
    return strategy.values


def test_research_indicator_matches_backtest(tmp_path: Path) -> None:
    """
    Test the research path reproduces the backtest indicator values for the same bars.
    """
    _write_catalog(tmp_path)

    backtest_values = _run_backtest(tmp_path)

    research = ResearchData(tmp_path)
    bars = research.bars([str(BAR_TYPE)])
    research_values = compute_indicator(ExponentialMovingAverage(EMA_PERIOD), bars)

    print(f"backtest_values={backtest_values}")  # noqa: T201
    print(f"research_values={research_values}")  # noqa: T201

    assert len(research_values) == len(backtest_values)
    assert research_values == backtest_values


def test_research_replay_orders_by_ts_init(tmp_path: Path) -> None:
    """
    Test the replay helper yields catalog data in `ts_init` order.
    """
    _write_catalog(tmp_path)

    research = ResearchData(tmp_path)
    bars = research.bars([str(BAR_TYPE)])
    shuffled = list(reversed(bars))

    replayed = list(research.replay(shuffled))

    assert [bar.ts_init for bar in replayed] == sorted(bar.ts_init for bar in bars)


def test_research_to_dataframe_delegates_to_query_catalog(tmp_path: Path) -> None:
    """
    Test the DataFrame conversion is the shared catalog query output.
    """
    _write_catalog(tmp_path)

    research = ResearchData(tmp_path)
    bars = research.bars([str(BAR_TYPE)])
    df = research.to_dataframe(NautilusDataType.Bar, identifiers=[str(BAR_TYPE)])

    assert len(df) == len(bars)
    assert df["ts_init"].astype("int64").tolist() == [bar.ts_init for bar in bars]
    assert df["close"].tolist() == [float(bar.close) for bar in bars]
