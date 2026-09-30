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
A node over catalog bars, shared by the execution realism regression scenarios.

A scenario for an execution realism model needs orders, fills, and positions in a canonical
result, and the model has to be reached the way a caller reaches it: through the venue
configuration. This harness runs a `BacktestNode` over a synthetic bar catalog with a strategy that
submits one market order from `on_bar`, which fills against the bar driven market, in the shape
`python/tests/integration/test_backtest_node_bar_fills.py` establishes. The bar establishes the
market before the strategy sees it, so a fill needs no quote or trade data.

Each scenario supplies the venue models it declares and verifies the canonical document of the
resulting run. Quote, trade, and bar driven runs share this engine and matching engine, so a
scenario pins the execution semantics rather than a data path.

The synthetic L1 book supplies 25 units of the 100 unit order, so the order exhausts the available
L1 volume and the engine fills the remaining 75 units one price increment deeper than the last
fill (`crates/execution/src/matching_engine/mod.rs:5270-5341`). A scenario therefore observes two
fills: the fill the models decided, and the exhausted-volume remainder, whose price is the decided
fill price plus one increment. The remainder is filled whichever order size is submitted, so the
first fill quantity is stable across scenarios.
"""

from __future__ import annotations

from decimal import Decimal
from pathlib import Path

from nautilus_trader.backtest import BacktestDataConfig
from nautilus_trader.backtest import BacktestEngineConfig
from nautilus_trader.backtest import BacktestNode
from nautilus_trader.backtest import BacktestRunConfig
from nautilus_trader.backtest import BacktestVenueConfig
from nautilus_trader.backtest import CanonicalBacktestResult
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.model import Bar
from nautilus_trader.model import BarType
from nautilus_trader.model import NautilusDataType
from nautilus_trader.model import OrderSide
from nautilus_trader.model import Price
from nautilus_trader.model import Quantity
from nautilus_trader.persistence import ParquetDataCatalog
from nautilus_trader.trading import Strategy
from nautilus_trader.trading import StrategyConfig
from tests.providers import TestInstrumentProvider


INSTRUMENT = TestInstrumentProvider.aapl_equity()
INSTRUMENT_ID = INSTRUMENT.id
INSTRUMENT_ID_STR = str(INSTRUMENT_ID)
BAR_TYPE = BarType.from_str(f"{INSTRUMENT_ID}-1-MINUTE-LAST-EXTERNAL")
VENUE = "XNAS"

MINUTE_NS = 60_000_000_000
TS_START = 1_704_067_200_000_000_000  # 2024-01-01T00:00:00Z, a minute boundary.
CLOSES = ("100.00", "101.00", "102.00")
# A multiple of the instrument lot size, so the venue accepts the order.
TRADE_SIZE = "100"


class TradeOnBarConfig(StrategyConfig):
    """
    Configure the bar-trading strategy.
    """

    def __init__(self, *, bar_type: str, trade_size: str) -> None:
        """
        Initialize the instance.
        """
        super().__init__()
        self.bar_type = bar_type
        self.trade_size = trade_size


class TradeOnBarStrategy(Strategy):
    """
    Submit one market order on the first bar.
    """

    def __init__(self, config: TradeOnBarConfig) -> None:
        """
        Initialize the instance.
        """
        super().__init__(config)
        self._bar_type = BarType.from_str(config.bar_type)
        self._quantity = Quantity.from_str(config.trade_size)
        self.submitted = False

    def on_start(self) -> None:
        """
        On start.
        """
        self.subscribe_bars(self._bar_type)

    def on_bar(self, _bar: Bar) -> None:
        """
        On bar.
        """
        if self.submitted:
            return

        self.submitted = True
        self.submit_order(
            self.order_factory.market(
                instrument_id=INSTRUMENT_ID,
                order_side=OrderSide.BUY,
                quantity=self._quantity,
            ),
        )


def bars() -> list[Bar]:
    """
    Build the synthetic bar catalog data.
    """
    result: list[Bar] = []
    for index, value in enumerate(CLOSES):
        ts = TS_START + index * MINUTE_NS
        price = Price.from_str(value)
        result.append(
            Bar(
                BAR_TYPE,
                price,
                price,
                price,
                price,
                Quantity.from_int(100),
                ts,
                ts,
            ),
        )
    return result


def run_node(
    catalog_path: Path,
    *,
    run_config_id: str,
    slippage_model: object | None = None,
    market_impact_model: object | None = None,
) -> CanonicalBacktestResult:
    """
    Run a node over the bar catalog with the given venue models and return its canonical result.

    The fee model is zero rated, so the fill prices in the result are the execution realism
    decision under test rather than a fee. The canonical result owns its document, so it stays
    valid after the node is disposed.

    The run config ID is supplied by the caller because a generated one is random, and a canonical
    document records it, so a scenario would otherwise not reproduce its own digest.
    """
    catalog_path.mkdir(parents=True, exist_ok=True)
    catalog = ParquetDataCatalog(str(catalog_path))
    catalog.write_instruments([INSTRUMENT])
    catalog.write_bars(bars())

    venue = BacktestVenueConfig(
        name=VENUE,
        oms_type="NETTING",
        account_type="CASH",
        starting_balances=["1_000_000 USD"],
        book_type="L1_MBP",
        slippage_model=slippage_model,
        market_impact_model=market_impact_model,
        fee_model=MakerTakerFeeModel(maker_rate=Decimal(0), taker_rate=Decimal(0)),
    )
    data = BacktestDataConfig(
        data_type=NautilusDataType.Bar,
        catalog_path=str(catalog_path),
        instrument_id=INSTRUMENT_ID,
        bar_types=[str(BAR_TYPE)],
    )
    config = BacktestRunConfig(
        id=run_config_id,
        venues=[venue],
        data=[data],
        engine=BacktestEngineConfig(bypass_logging=True, run_analysis=False),
        dispose_on_completion=False,
    )
    node = BacktestNode([config])
    node.build()
    node.add_strategy(
        config.id,
        TradeOnBarStrategy(TradeOnBarConfig(bar_type=str(BAR_TYPE), trade_size=TRADE_SIZE)),
    )
    try:
        node.run()
        return node.get_engine_canonical_result(config.id)
    finally:
        node.dispose()
