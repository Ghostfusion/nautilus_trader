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
Declared regression scenario: two venues, two account types, tick-scheduled orders.

A margin account and a cash account each open and close one position from deterministic quote
ticks, which exercises order, fill, position, fee, and PnL projection on both account types.
"""

from __future__ import annotations

from decimal import Decimal
from typing import Any

from nautilus_trader.backtest import BacktestEngineConfig
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.model import AccountType
from nautilus_trader.model import Currency
from nautilus_trader.model import Money
from nautilus_trader.model import OmsType
from nautilus_trader.model import Price
from nautilus_trader.model import Quantity
from nautilus_trader.model import QuoteTick
from nautilus_trader.model import Venue
from nautilus_trader.risk import RiskEngineConfig
from nautilus_trader.trading import ImportableStrategyConfig
from tests.providers import TestInstrumentProvider
from tests.regression.scenario import Checkpoint
from tests.regression.scenario import Scenario
from tests.regression.scenario import backtest_engine


NAME = "multi_venue_parity"

STRATEGY = "strategies.acceptance:MultiInstrumentTickScheduled"
STRATEGY_CONFIG = "strategies.acceptance:MultiInstrumentTickScheduledConfig"

AUDUSD = TestInstrumentProvider.audusd_sim()
ETHUSDT = TestInstrumentProvider.ethusdt_binance()
AUDUSD_ID = str(AUDUSD.id)
ETHUSDT_ID = str(ETHUSDT.id)

TS_START = 1_577_836_800_000_000_000
INTERVAL_NS = 60_000_000_000
AUDUSD_BID_PRICES = ("0.70000", "0.70000", "0.70010", "0.70020", "0.70020")
ETHUSDT_BID_PRICES = ("2000.00", "2000.00", "2000.50", "2001.00", "2001.00")

CHECKPOINTS = (
    Checkpoint("order", instrument_id=AUDUSD_ID, ordinal=0),
    Checkpoint("order", instrument_id=AUDUSD_ID, ordinal=1),
    Checkpoint("fill", instrument_id=AUDUSD_ID, ordinal=0),
    Checkpoint("fill", instrument_id=AUDUSD_ID, ordinal=1),
    Checkpoint("position", instrument_id=AUDUSD_ID, ordinal=0),
    Checkpoint("order", instrument_id=ETHUSDT_ID, ordinal=0),
    Checkpoint("fill", instrument_id=ETHUSDT_ID, ordinal=0),
    Checkpoint("position", instrument_id=ETHUSDT_ID, ordinal=0),
)


def _engine_config() -> BacktestEngineConfig:
    return BacktestEngineConfig(
        bypass_logging=True,
        run_analysis=False,
        risk_engine=RiskEngineConfig(bypass=True),
    )


def _quotes(instrument: Any, bid_prices: tuple[str, ...]) -> list[QuoteTick]:
    quotes: list[QuoteTick] = []
    for index, bid_price in enumerate(bid_prices):
        ts = TS_START + index * INTERVAL_NS
        quotes.append(
            QuoteTick(
                instrument_id=instrument.id,
                bid_price=Price.from_str(bid_price),
                ask_price=Price.from_str(bid_price),
                bid_size=Quantity(1_000_000, precision=instrument.size_precision),
                ask_size=Quantity(1_000_000, precision=instrument.size_precision),
                ts_event=ts,
                ts_init=ts,
            ),
        )
    return quotes


def execute() -> Any:
    """
    Run the scenario and return its canonical result.
    """
    sim = Venue("SIM")
    binance = Venue("BINANCE")
    usd = Currency.from_str("USD")
    eth = Currency.from_str("ETH")
    usdt = Currency.from_str("USDT")

    with backtest_engine(_engine_config()) as engine:
        engine.add_venue(
            venue=sim,
            oms_type=OmsType.NETTING,
            account_type=AccountType.MARGIN,
            base_currency=usd,
            starting_balances=[Money(1_000_000.0, usd)],
            fee_model=MakerTakerFeeModel(
                maker_rate=Decimal("0.00002"),
                taker_rate=Decimal("0.00002"),
            ),
        )
        engine.add_venue(
            venue=binance,
            oms_type=OmsType.NETTING,
            account_type=AccountType.CASH,
            starting_balances=[Money(10.0, eth), Money(100_000.0, usdt)],
            fee_model=MakerTakerFeeModel(
                maker_rate=Decimal("0.0001"),
                taker_rate=Decimal("0.0001"),
            ),
        )
        engine.add_instrument(AUDUSD)
        engine.add_instrument(ETHUSDT)
        engine.add_data(_quotes(AUDUSD, AUDUSD_BID_PRICES))
        engine.add_data(_quotes(ETHUSDT, ETHUSDT_BID_PRICES))
        engine.add_strategy_from_config(
            ImportableStrategyConfig(
                strategy_path=STRATEGY,
                config_path=STRATEGY_CONFIG,
                config={
                    "instrument_actions": {
                        AUDUSD_ID: [(2, "BUY", "100000"), (4, "SELL", "100000")],
                        ETHUSDT_ID: [(2, "BUY", "0.50000"), (4, "SELL", "0.50000")],
                    },
                },
            ),
        )
        engine.run()
        return engine.get_canonical_result()


SCENARIO = Scenario(
    name=NAME,
    execute=execute,
    checkpoints=CHECKPOINTS,
)
