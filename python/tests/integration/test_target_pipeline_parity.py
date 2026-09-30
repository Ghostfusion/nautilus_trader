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
Parity between the direct order path and the opt-in target pipeline.

Two strategies run over the same quote data. The direct strategy submits one market order of a
stated size from `on_quote`; the pipeline strategy enables the target pipeline and submits one long
signal whose constructed target resolves, through fixed-risk sizing, to the same size. Both express
the same intent, so both runs must agree on the orders, the fills, and the positions. This is the
acceptance criterion for the optional pipeline: equivalent intent produces equivalent orders.

The mid of the quotes is 100.000 and the account holds 1,000,000 USD, so with a 1 per cent stop
(100 basis points) and 0.0001 of equity risked the fixed-risk sizing resolves to exactly 100 units,
a multiple of the instrument's 100 unit lot size.
"""

from __future__ import annotations

import json
from decimal import Decimal
from typing import Any

from nautilus_trader.backtest import BacktestEngine
from nautilus_trader.backtest import BacktestEngineConfig
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.model import AccountType
from nautilus_trader.model import Currency
from nautilus_trader.model import InstrumentId
from nautilus_trader.model import Money
from nautilus_trader.model import OmsType
from nautilus_trader.model import OrderSide
from nautilus_trader.model import Price
from nautilus_trader.model import Quantity
from nautilus_trader.model import QuoteTick
from nautilus_trader.model import SignalDirection
from nautilus_trader.model import TradingSignal
from nautilus_trader.model import Venue
from nautilus_trader.risk import RiskEngineConfig
from nautilus_trader.trading import Strategy
from nautilus_trader.trading import StrategyConfig
from nautilus_trader.trading import TargetPipelineConfig
from tests.providers import TestInstrumentProvider


INSTRUMENT = TestInstrumentProvider.aapl_equity()
INSTRUMENT_ID = INSTRUMENT.id
XNAS = Venue("XNAS")
USD = Currency.from_str("USD")

TS_START = 1_704_067_200_000_000_000
INTERVAL_NS = 60_000_000_000
QUOTE_COUNT = 5

# The order size both paths express: a multiple of the instrument's 100 unit lot size.
TRADE_SIZE = Quantity.from_int(100)

# With a 1 per cent stop and 0.0001 of equity risked, the fixed-risk sizing resolves to 100 units:
#   risk money  = 1,000,000 * 0.0001 = 100
#   risk points = |100.000 - 99.000| / 0.01 = 100
#   quantity    = 100 / 100 / 0.01 = 100
# The constructed weight is 100 * 100 / 1,000,000 = 0.01, and reconciliation converts that back to
# 100 units at the same price, so the pipeline order states the same size as the direct order.
PIPELINE_CONFIG = TargetPipelineConfig(
    risk_per_trade=Decimal("0.0001"),
    stop_loss_bps=100,
    max_weight=Decimal("1"),
    commission_rate=Decimal("0"),
    min_order_quantity=Quantity.from_int(0),
)


class DirectStrategyConfig(StrategyConfig):
    """
    Configure the direct strategy.
    """

    def __init__(self, *, instrument_id: str, trade_size: str) -> None:
        """
        Initialize the instance.
        """
        super().__init__()
        self.instrument_id = instrument_id
        self.trade_size = trade_size


class DirectStrategy(Strategy):
    """
    Submit one market order of a stated size on the first quote.
    """

    def __init__(self, config: DirectStrategyConfig) -> None:
        """
        Initialize the instance.
        """
        super().__init__(config)
        self._instrument_id = InstrumentId.from_str(config.instrument_id)
        self._quantity = Quantity.from_str(config.trade_size)
        self.submitted = False

    def on_start(self) -> None:
        """
        On start.
        """
        self.subscribe_quotes(self._instrument_id)

    def on_quote(self, _quote: QuoteTick) -> None:
        """
        On quote.
        """
        if self.submitted:
            return
        self.submitted = True
        self.submit_order(
            self.order_factory.market(
                instrument_id=self._instrument_id,
                order_side=OrderSide.BUY,
                quantity=self._quantity,
            ),
        )


class PipelineStrategyConfig(StrategyConfig):
    """
    Configure the pipeline strategy.
    """

    def __init__(self, *, instrument_id: str) -> None:
        """
        Initialize the instance.
        """
        super().__init__()
        self.instrument_id = instrument_id


class PipelineStrategy(Strategy):
    """
    Submit one long signal through the target pipeline on the first quote.
    """

    def __init__(self, config: PipelineStrategyConfig) -> None:
        """
        Initialize the instance.
        """
        super().__init__(config)
        self._instrument_id = InstrumentId.from_str(config.instrument_id)
        self.submitted = False

    def on_start(self) -> None:
        """
        On start.
        """
        self.enable_target_pipeline(PIPELINE_CONFIG)
        self.subscribe_quotes(self._instrument_id)

    def on_quote(self, _quote: QuoteTick) -> None:
        """
        On quote.
        """
        if self.submitted:
            return
        self.submitted = True
        self.submit_signals(
            [
                TradingSignal(
                    instrument_id=self._instrument_id,
                    direction=SignalDirection.LONG,
                ),
            ],
        )


def _quotes() -> list[QuoteTick]:
    quotes: list[QuoteTick] = []
    for index in range(QUOTE_COUNT):
        ts = TS_START + index * INTERVAL_NS
        quotes.append(
            QuoteTick(
                instrument_id=INSTRUMENT_ID,
                bid_price=Price.from_str("99.99"),
                ask_price=Price.from_str("100.01"),
                bid_size=Quantity(10_000, precision=0),
                ask_size=Quantity(10_000, precision=0),
                ts_event=ts,
                ts_init=ts,
            ),
        )
    return quotes


def _run(strategy: Strategy) -> dict[str, Any]:
    config = BacktestEngineConfig(
        bypass_logging=True,
        run_analysis=False,
        risk_engine=RiskEngineConfig(bypass=True),
    )
    engine = BacktestEngine(config)
    try:
        engine.add_venue(
            venue=XNAS,
            oms_type=OmsType.NETTING,
            account_type=AccountType.CASH,
            base_currency=USD,
            starting_balances=[Money(1_000_000.0, USD)],
            fee_model=MakerTakerFeeModel(
                maker_rate=Decimal("0"),
                taker_rate=Decimal("0"),
            ),
        )
        engine.add_instrument(INSTRUMENT)
        engine.add_data(_quotes())
        engine.add_strategy(strategy)
        engine.run(start=TS_START, end=TS_START + QUOTE_COUNT * INTERVAL_NS)
        return json.loads(engine.get_canonical_result().to_bytes().decode("utf-8"))
    finally:
        engine.dispose()


def _orders(document: dict[str, Any]) -> list[tuple[str, str, str]]:
    result: list[tuple[str, str, str]] = []
    for record in document["orders"]:
        payload = next(iter(record.values()))
        core = payload["core"]
        result.append((core["instrument_id"], core["side"], core["quantity"]))
    return result


def _fills(document: dict[str, Any]) -> list[tuple[str, str, str, str]]:
    result: list[tuple[str, str, str, str]] = []
    for record in document["fills"]:
        payload = next(iter(record["event"].values()))
        result.append(
            (
                payload["instrument_id"],
                payload["order_side"],
                payload["last_px"],
                payload["last_qty"],
            ),
        )
    return result


def _positions(document: dict[str, Any]) -> list[tuple[str, str, str]]:
    result: list[tuple[str, str, str]] = []
    for record in document["positions"]:
        result.append(
            (
                record["instrument_id"],
                record["side"],
                record["quantity"],
            ),
        )
    return result


def test_direct_and_pipeline_paths_produce_equivalent_orders_fills_and_positions() -> None:
    """
    Equivalent intent produces equivalent orders, fills, and positions on both paths.
    """
    direct_document = _run(
        DirectStrategy(
            DirectStrategyConfig(
                instrument_id=str(INSTRUMENT_ID),
                trade_size=str(TRADE_SIZE),
            ),
        ),
    )
    pipeline_document = _run(
        PipelineStrategy(PipelineStrategyConfig(instrument_id=str(INSTRUMENT_ID))),
    )

    direct_orders = _orders(direct_document)
    pipeline_orders = _orders(pipeline_document)
    direct_fills = _fills(direct_document)
    pipeline_fills = _fills(pipeline_document)
    direct_positions = _positions(direct_document)
    pipeline_positions = _positions(pipeline_document)

    # The observed values for both runs, verbatim.
    assert direct_orders == pipeline_orders, (
        f"orders differ: direct={direct_orders!r} pipeline={pipeline_orders!r}"
    )
    assert direct_fills == pipeline_fills, (
        f"fills differ: direct={direct_fills!r} pipeline={pipeline_fills!r}"
    )
    assert direct_positions == pipeline_positions, (
        f"positions differ: direct={direct_positions!r} pipeline={pipeline_positions!r}"
    )

    # Both runs express the same single 100 unit buy, and it filled once for 100 units.
    assert direct_orders == [(str(INSTRUMENT_ID), "BUY", "100")]
    assert len(direct_fills) == 1
    assert direct_positions == [(str(INSTRUMENT_ID), "LONG", "100")]
