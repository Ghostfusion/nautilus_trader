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
Declared regression scenario: EMA cross strategy over a bounded slice of minute bars.

A bar-driven strategy over shipped test data exercises bar aggregation, indicators, and repeated
order and position cycles, which the quote-tick scenario does not cover.
"""

from __future__ import annotations

from decimal import Decimal
from typing import Any

from nautilus_trader.backtest import BacktestEngineConfig
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.model import AccountType
from nautilus_trader.model import BarType
from nautilus_trader.model import Currency
from nautilus_trader.model import Money
from nautilus_trader.model import OmsType
from nautilus_trader.model import Venue
from nautilus_trader.risk import RiskEngineConfig
from nautilus_trader.trading import ImportableStrategyConfig
from tests.providers import TestDataProvider
from tests.providers import TestInstrumentProvider
from tests.regression.scenario import Checkpoint
from tests.regression.scenario import Scenario
from tests.regression.scenario import backtest_engine


NAME = "btcusdt_ema_cross"

STRATEGY = "strategies.ema_cross:EMACross"
STRATEGY_CONFIG = "strategies.ema_cross:EMACrossConfig"

INSTRUMENT = TestInstrumentProvider.btcusdt_binance()
INSTRUMENT_ID = str(INSTRUMENT.id)
BAR_TYPE = "BTCUSDT.BINANCE-1-MINUTE-LAST-EXTERNAL"
CSV_NAME = "btc-perp-20211231-20220201_1m.csv"
MAX_ROWS = 250
TRADE_SIZE = "0.010000"
FAST_EMA_PERIOD = 10
SLOW_EMA_PERIOD = 20

CHECKPOINTS = (
    Checkpoint("order", instrument_id=INSTRUMENT_ID, ordinal=0),
    Checkpoint("order", instrument_id=INSTRUMENT_ID, ordinal=1),
    Checkpoint("fill", instrument_id=INSTRUMENT_ID, ordinal=0),
    Checkpoint("fill", instrument_id=INSTRUMENT_ID, ordinal=1),
    Checkpoint("position", instrument_id=INSTRUMENT_ID, ordinal=0),
)


def _engine_config() -> BacktestEngineConfig:
    return BacktestEngineConfig(
        bypass_logging=True,
        run_analysis=False,
        risk_engine=RiskEngineConfig(bypass=True),
    )


def execute() -> Any:
    """
    Run the scenario and return its canonical result.
    """
    venue = Venue("BINANCE")
    btc = Currency.from_str("BTC")
    usdt = Currency.from_str("USDT")
    bars = TestDataProvider.bars_from_binance_csv(
        INSTRUMENT,
        bar_type=BarType.from_str(BAR_TYPE),
        csv_name=CSV_NAME,
        max_rows=MAX_ROWS,
    )

    with backtest_engine(_engine_config()) as engine:
        engine.add_venue(
            venue=venue,
            oms_type=OmsType.NETTING,
            account_type=AccountType.CASH,
            starting_balances=[Money(10.0, btc), Money(10_000_000.0, usdt)],
            fee_model=MakerTakerFeeModel(
                maker_rate=Decimal("0.001"),
                taker_rate=Decimal("0.001"),
            ),
        )
        engine.add_instrument(INSTRUMENT)
        engine.add_data(bars)
        engine.add_strategy_from_config(
            ImportableStrategyConfig(
                strategy_path=STRATEGY,
                config_path=STRATEGY_CONFIG,
                config={
                    "instrument_id": INSTRUMENT_ID,
                    "bar_type": BAR_TYPE,
                    "trade_size": TRADE_SIZE,
                    "fast_ema_period": FAST_EMA_PERIOD,
                    "slow_ema_period": SLOW_EMA_PERIOD,
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
