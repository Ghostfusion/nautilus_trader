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
A run over a trade tape whose reported aggressor sides agree with the tick rule four times in five.

The data-quality gate compares each trade's reported aggressor side against the side the tick rule
infers, and reports the observed agreement rate in the run summary whenever a trade was compared.
The tape is fixed, so the rate is a declared number rather than a sampled one; the declared floor
is above the observed rate, so the run also records the disagreement the gate is read against.

The summary entry `aggressor_agreement` is the run figure a declared floor is compared against. No
strategy is needed: the data engine performs the comparison as it receives the tape.
"""

from __future__ import annotations

from decimal import Decimal
from typing import Any

from nautilus_trader.backtest import BacktestEngine
from nautilus_trader.backtest import BacktestEngineConfig
from nautilus_trader.data import DataEngineConfig
from nautilus_trader.data import DataQualityAction
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.model import AccountType
from nautilus_trader.model import AggressorSide
from nautilus_trader.model import Currency
from nautilus_trader.model import Money
from nautilus_trader.model import OmsType
from nautilus_trader.model import Price
from nautilus_trader.model import Quantity
from nautilus_trader.model import TradeId
from nautilus_trader.model import TradeTick
from nautilus_trader.model import Venue
from tests.providers import TestInstrumentProvider
from tests.regression.scenario import DEFAULT_STATISTICS
from tests.regression.scenario import Scenario


NAME = "data_quality_aggressor"

INSTRUMENT = TestInstrumentProvider.ethusdt_binance()

# The declared floor sits above the observed rate, so the run is one a gate would flag.
AGGRESSOR_AGREEMENT_FLOOR = 0.9

# A tape whose reported sides agree with the tick rule four times in five: the price alternates,
# so the inferred side alternates, while every trade reports a buy.
TRADE_PRICES = ("1000.00", "1000.10", "1000.00", "1000.10", "1000.00", "1000.10")

# 2020-09-13T12:26:40Z, the instrument's base timestamp.
BASE_NS = 1_600_000_000_000_000_000

# The selector that addresses the run-summary entry the case pins.
AGGRESSOR_AGREEMENT_SELECTOR = "summary.aggressor_agreement"


def _tape() -> list[TradeTick]:
    """
    Build the fixed trade tape the agreement rate is measured over.
    """
    return [
        TradeTick(
            instrument_id=INSTRUMENT.id,
            price=Price.from_str(price),
            size=Quantity.from_str("1.00000"),
            aggressor_side=AggressorSide.BUY,
            trade_id=TradeId(str(index)),
            ts_event=BASE_NS + index * 1_000_000_000,
            ts_init=BASE_NS + index * 1_000_000_000,
        )
        for index, price in enumerate(TRADE_PRICES)
    ]


def execute() -> Any:
    """
    Run the tape through an engine that declares the aggressor-agreement floor.
    """
    config = BacktestEngineConfig(
        bypass_logging=True,
        run_analysis=False,
        data_engine=DataEngineConfig(
            data_quality_action=DataQualityAction.FLAG,
            aggressor_agreement_floor=AGGRESSOR_AGREEMENT_FLOOR,
        ),
    )
    engine = BacktestEngine(config)
    engine.add_venue(
        venue=Venue("BINANCE"),
        oms_type=OmsType.NETTING,
        account_type=AccountType.MARGIN,
        base_currency=Currency.from_str("USDT"),
        starting_balances=[Money(1_000_000.0, Currency.from_str("USDT"))],
        fee_model=MakerTakerFeeModel(maker_rate=Decimal(0), taker_rate=Decimal(0)),
    )
    engine.add_instrument(INSTRUMENT)
    engine.add_data(_tape())
    try:
        engine.run()
        return engine.get_canonical_result()
    finally:
        engine.dispose()


SCENARIO = Scenario(
    name=NAME,
    execute=execute,
    statistics=(*DEFAULT_STATISTICS, AGGRESSOR_AGREEMENT_SELECTOR),
)
