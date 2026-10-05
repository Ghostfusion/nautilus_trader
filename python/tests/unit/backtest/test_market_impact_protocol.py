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
Test the Python market impact model protocol.

The scenario is the synthetic book and bar strategy the regression scenarios share, exercised
through the engine's own `add_venue` rather than through a run config, because a caller's own model
reaches a venue there.
"""

from __future__ import annotations

from decimal import Decimal

from nautilus_trader.backtest import BacktestEngine
from nautilus_trader.backtest import BacktestEngineConfig
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.model import AccountType
from nautilus_trader.model import BookType
from nautilus_trader.model import Money
from nautilus_trader.model import OmsType
from nautilus_trader.model import Venue
from tests.regression.execution_realism import BAR_TYPE
from tests.regression.execution_realism import INSTRUMENT
from tests.regression.execution_realism import TRADE_SIZE
from tests.regression.execution_realism import VENUE
from tests.regression.execution_realism import TradeOnBarConfig
from tests.regression.execution_realism import TradeOnBarStrategy
from tests.regression.execution_realism import bars


class PythonMarketImpactModel:
    """
    A market impact model written in Python, which is the whole protocol.

    The model records the fill quantities it is asked about and returns the number of price
    increments the fill moves the price by.
    """

    def __init__(self, increments: int = 0) -> None:
        """
        Initialize the instance.
        """
        self.increments = increments
        self.quantities: list[str] = []

    def impact_increments(self, fill_quantity: object) -> int:
        """
        Return the number of price increments the fill moves the price by.
        """
        self.quantities.append(str(fill_quantity))
        return self.increments


class RaisingMarketImpactModel:
    """
    A Python impact model whose method raises.
    """

    def impact_increments(self, _fill_quantity: object) -> int:
        """
        Raise, so that the run must surface the failure.
        """
        msg = "model boom"
        raise ValueError(msg)


def run_venue(model: object) -> float | None:
    """
    Run the shared scenario against a venue configured with the given impact model.

    Returns the price the filled order averaged, or `None` when the run filled nothing.
    """
    engine = BacktestEngine(BacktestEngineConfig(bypass_logging=True, run_analysis=False))
    engine.add_venue(
        venue=Venue(VENUE),
        oms_type=OmsType.NETTING,
        account_type=AccountType.CASH,
        starting_balances=[Money.from_str("1_000_000 USD")],
        book_type=BookType.L1_MBP,
        market_impact_model=model,
        fee_model=MakerTakerFeeModel(maker_rate=Decimal(0), taker_rate=Decimal(0)),
    )
    engine.add_instrument(INSTRUMENT)
    engine.add_data(bars())
    engine.add_strategy(
        TradeOnBarStrategy(TradeOnBarConfig(bar_type=str(BAR_TYPE), trade_size=TRADE_SIZE)),
    )

    try:
        engine.run()
        orders = [order for order in engine.cache.orders() if order.is_closed]
        return float(orders[0].avg_px) if orders else None
    finally:
        engine.dispose()


def test_add_venue_dispatches_a_python_market_impact_model() -> None:
    """
    Test that a venue runs a Python impact model and its adjustment reaches the fill.
    """
    model = PythonMarketImpactModel(increments=3)
    impacted = run_venue(model)
    unimpacted = run_venue(PythonMarketImpactModel(increments=0))

    assert impacted is not None, "the scenario must fill an order"
    assert unimpacted is not None, "the scenario must fill an order"

    # The engine consulted the caller's model with the fill quantities it decided.
    assert model.quantities, "the model must be asked about each fill"
    assert all(quantity != "0" for quantity in model.quantities)

    # Three increments on a buy fill the order higher than none, on the same book and data.
    assert impacted > unimpacted


def test_a_python_market_impact_model_exception_aborts_the_fill() -> None:
    """
    Test that a Python impact model exception stops the fill rather than being ignored.

    The engine logs the error and fills nothing, which is the handling every fill-path error gets,
    so a model that cannot answer cannot produce a fill price.
    """
    assert run_venue(RaisingMarketImpactModel()) is None
