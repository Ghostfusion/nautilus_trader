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
Test the Python latency model protocol.

A Python-defined latency model is injected through `BacktestEngine.add_venue`, the path a caller
uses, and the run must show the delay it reports: an order whose insertion latency exceeds the whole
bar window never reaches the venue and never fills, while the same order with a zero-latency model
fills. The test proves the applied latency by its outcome rather than by the model being called.
"""

from __future__ import annotations

from decimal import Decimal
from typing import Self

from nautilus_trader.backtest import BacktestEngine
from nautilus_trader.backtest import BacktestEngineConfig
from nautilus_trader.execution import LatencyModel
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.model import AccountType
from nautilus_trader.model import Bar
from nautilus_trader.model import BarType
from nautilus_trader.model import Currency
from nautilus_trader.model import Money
from nautilus_trader.model import OmsType
from nautilus_trader.model import OrderSide
from nautilus_trader.model import Price
from nautilus_trader.model import Quantity
from nautilus_trader.model import Venue
from nautilus_trader.trading import Strategy
from nautilus_trader.trading import StrategyConfig
from tests.providers import TestInstrumentProvider


USD = Currency.from_str("USD")
VENUE = Venue("SIM")
INSTRUMENT = TestInstrumentProvider.audusd_sim()
INSTRUMENT_ID = INSTRUMENT.id
BAR_TYPE = BarType.from_str(f"{INSTRUMENT_ID}-1-MINUTE-LAST-EXTERNAL")
MINUTE_NS = 60_000_000_000
TS_START = 1_600_000_000_000_000_000
TRADE_SIZE = "100000"
LIMIT_PRICE = "0.70060"
# The limit crosses the second bar; by the third the market has walked above it, so an order that
# arrives after the window rests below the market and never fills.
BAR_CLOSES = ("0.70000", "0.70050", "0.70100")
# The whole bar window is a few minutes; an hour is far beyond it.
STALLED_INSERT_NANOS = 3_600_000_000_000


class PythonLatencyModel(LatencyModel):
    """
    A latency model written in Python, subclassing the exposed base class.

    Only the insert leg is overridden; the other legs keep the base class's zero defaults.
    """

    def __new__(cls, *_args: object, **_kwargs: object) -> Self:
        """
        Allocate the instance without forwarding arguments to the base constructor.
        """
        return super().__new__(cls)

    def __init__(self, insert_nanos: int = 0) -> None:
        """
        Initialize the instance.
        """
        self._insert_nanos = insert_nanos

    def get_insert_latency(self) -> int:
        """
        Return the insertion latency in nanoseconds.
        """
        return self._insert_nanos


class SubmitLimitOnBarConfig(StrategyConfig):
    """
    Configure the limit-order strategy.
    """

    def __init__(self, *, bar_type: str, trade_size: str, price: str) -> None:
        """
        Initialize the instance.
        """
        super().__init__()
        self.bar_type = bar_type
        self.trade_size = trade_size
        self.price = price


class SubmitLimitOnBar(Strategy):
    """
    Submit one buy limit order on the first bar.
    """

    def __init__(self, config: SubmitLimitOnBarConfig) -> None:
        """
        Initialize the instance.
        """
        super().__init__(config)
        self._bar_type = BarType.from_str(config.bar_type)
        self._quantity = Quantity.from_str(config.trade_size)
        self._price = Price.from_str(config.price)
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
            self.order_factory.limit(
                instrument_id=INSTRUMENT_ID,
                order_side=OrderSide.BUY,
                quantity=self._quantity,
                price=self._price,
            ),
        )


def bars() -> list[Bar]:
    """
    Build one-minute bars whose market walks up, so an order's arrival time decides its fill.
    """
    result: list[Bar] = []
    for index, close in enumerate(BAR_CLOSES):
        ts = TS_START + index * MINUTE_NS
        price = Price.from_str(close)
        result.append(
            Bar(
                BAR_TYPE,
                price,
                price,
                price,
                price,
                Quantity.from_int(1_000_000),
                ts,
                ts,
            ),
        )
    return result


def run_venue(latency_model: object) -> int:
    """
    Run the scenario against a venue configured with the given latency model.

    Returns the number of closed orders.
    """
    engine = BacktestEngine(BacktestEngineConfig(bypass_logging=True, run_analysis=False))
    engine.add_venue(
        venue=VENUE,
        oms_type=OmsType.NETTING,
        account_type=AccountType.MARGIN,
        starting_balances=[Money(1_000_000.0, USD)],
        base_currency=USD,
        latency_model=latency_model,
        fee_model=MakerTakerFeeModel(maker_rate=Decimal(0), taker_rate=Decimal(0)),
    )
    engine.add_instrument(INSTRUMENT)
    engine.add_data(bars())
    engine.add_strategy(
        SubmitLimitOnBar(
            SubmitLimitOnBarConfig(
                bar_type=str(BAR_TYPE),
                trade_size=TRADE_SIZE,
                price=LIMIT_PRICE,
            ),
        ),
    )

    try:
        engine.run()
        return sum(1 for order in engine.cache.orders() if order.is_closed)
    finally:
        engine.dispose()


def test_add_venue_applies_a_python_latency_model() -> None:
    """
    Test that a venue runs a Python latency model and its delay reaches the order arrival.
    """
    zero = PythonLatencyModel(insert_nanos=0)
    assert run_venue(zero) > 0, "a zero-latency model must let the order fill"

    stalled = PythonLatencyModel(insert_nanos=STALLED_INSERT_NANOS)
    assert run_venue(stalled) == 0, "an order delayed past the window must not fill"
