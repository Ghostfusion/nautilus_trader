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
End-to-end tests for a `BacktestNode` run over catalog bars that trades.

The node loads bars from a catalog and runs them through the same engine and matching engine the
Python `BacktestEngine` uses. These tests prove that a correctly sized market order submitted from
`on_bar` against a bar driven market enters the execution engine, fills, and appears in the run's
orders, fills, positions, and canonical document, which a scenario for a fill model depends on.

They also prove the configuration layer introduced for the built-in fill models is accepted by the
node's venue configuration.
"""

from __future__ import annotations

from decimal import Decimal
from pathlib import Path

from nautilus_trader.backtest import BacktestDataConfig
from nautilus_trader.backtest import BacktestEngineConfig
from nautilus_trader.backtest import BacktestNode
from nautilus_trader.backtest import BacktestRunConfig
from nautilus_trader.backtest import BacktestVenueConfig
from nautilus_trader.execution import FillModelConfig
from nautilus_trader.execution import FillModelKind
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
BAR_TYPE = BarType.from_str(f"{INSTRUMENT_ID}-1-MINUTE-LAST-EXTERNAL")

MINUTE_NS = 60_000_000_000
TS_START = 1_704_067_200_000_000_000  # 2024-01-01T00:00:00Z, a minute boundary.
CLOSES = ("100.00", "101.00", "102.00")
# A multiple of the instrument lot size, so the venue accepts the order.
TRADE_SIZE = "100"


class TradeOnBarConfig(StrategyConfig):
    """
    Configure the bar-trading strategy.
    """

    def __init__(self, *, bar_type: str, trade_size: str, submit: bool) -> None:
        """
        Initialize the instance.
        """
        super().__init__()
        self.bar_type = bar_type
        self.trade_size = trade_size
        self.submit = submit


class TradeOnBarStrategy(Strategy):
    """
    Submit one market order on the first bar, optionally.
    """

    def __init__(self, config: TradeOnBarConfig) -> None:
        """
        Initialize the instance.
        """
        super().__init__(config)
        self._bar_type = BarType.from_str(config.bar_type)
        self._quantity = Quantity.from_str(config.trade_size)
        self._should_submit = config.submit
        self.submitted = False
        self.fills: list[str] = []
        self.filled_qty = Quantity.from_int(0)

    def on_start(self) -> None:
        """
        On start.
        """
        self.subscribe_bars(self._bar_type)

    def on_bar(self, bar: Bar) -> None:
        """
        On bar.
        """
        if not self._should_submit or self.submitted:
            return

        self.submitted = True
        self.submit_order(
            self.order_factory.market(
                instrument_id=INSTRUMENT_ID,
                order_side=OrderSide.BUY,
                quantity=self._quantity,
            ),
        )

    def on_order_filled(self, event: object) -> None:
        """
        On order filled.
        """
        self.fills.append(str(event))
        self.filled_qty += event.last_qty


def _bars() -> list[Bar]:
    bars: list[Bar] = []
    for index, value in enumerate(CLOSES):
        ts = TS_START + index * MINUTE_NS
        price = Price.from_str(value)
        bars.append(
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
    return bars


def _run_node(
    catalog_path: Path,
    *,
    submit: bool,
    fill_model: FillModelConfig,
) -> tuple[TradeOnBarStrategy, dict[str, object], bytes]:
    catalog_path.mkdir(parents=True, exist_ok=True)
    catalog = ParquetDataCatalog(str(catalog_path))
    catalog.write_instruments([INSTRUMENT])
    catalog.write_bars(_bars())

    venue = BacktestVenueConfig(
        name="XNAS",
        oms_type="NETTING",
        account_type="CASH",
        starting_balances=["1_000_000 USD"],
        book_type="L1_MBP",
        fill_model=fill_model,
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
    strategy = TradeOnBarStrategy(
        TradeOnBarConfig(bar_type=str(BAR_TYPE), trade_size=TRADE_SIZE, submit=submit),
    )
    node.add_strategy(config.id, strategy)
    try:
        node.run()
        reports = {
            "orders": node.generate_orders_report(config.id),
            "fills": node.generate_fills_report(config.id),
            "positions": node.generate_positions_report(config.id),
        }
        canonical = node.get_engine_canonical_result(config.id).to_bytes()
    finally:
        node.dispose()

    return strategy, reports, canonical


def test_node_run_over_bars_fills_a_market_order(tmp_path: Path) -> None:
    """
    Test a market order from `on_bar` fills in a node run over catalog bars.
    """
    strategy, reports, canonical = _run_node(
        tmp_path,
        submit=True,
        fill_model=FillModelConfig(),
    )

    assert strategy.submitted
    assert strategy.filled_qty == Quantity.from_str(TRADE_SIZE)
    assert len(reports["orders"]) == 1
    assert len(reports["fills"]) >= 1
    assert len(reports["positions"]) == 1

    _, _, without_order = _run_node(tmp_path / "no-order", submit=False, fill_model=FillModelConfig())
    assert canonical != without_order


def test_node_run_over_bars_accepts_a_fill_model_configuration(tmp_path: Path) -> None:
    """
    Test the node venue configuration accepts the fill model configuration layer.

    A configuration naming a model other than the default must reach the matching engine, which is
    observable as a different canonical document for the same data and strategy.
    """
    _, _, default = _run_node(
        tmp_path / "default",
        submit=True,
        fill_model=FillModelConfig(),
    )
    _, _, slippage = _run_node(
        tmp_path / "slippage",
        submit=True,
        fill_model=FillModelConfig(
            kind=FillModelKind.ONE_TICK_SLIPPAGE,
            prob_fill_on_limit=1.0,
            prob_slippage=1.0,
        ),
    )

    assert default != slippage
