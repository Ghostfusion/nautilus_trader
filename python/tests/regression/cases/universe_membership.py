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
A universe whose scheduled membership decides which instruments trade.

The universe selects on a fixed clock interval, and the rule declares three membership sets: AAPL
alone, then AAPL and MSFT, then AAPL alone again. The strategy submits one market order for each
instrument as it becomes ACTIVE and nothing else, so the orders, fills, and positions in the
canonical result are a direct consequence of the membership schedule.

MSFT is deselected while its position is open, so its removal is held: the state stays REMOVING
and its subscriptions are retained until the removal policy is satisfied. The removal completes on
the selection step after the position closes, which is covered by the membership assertions in the
integration test rather than here.
"""

from __future__ import annotations

from datetime import UTC
from datetime import datetime
from decimal import Decimal
from typing import Any

from nautilus_trader.backtest import BacktestEngineConfig
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.model import AccountType
from nautilus_trader.model import Currency
from nautilus_trader.model import Equity
from nautilus_trader.model import InstrumentId
from nautilus_trader.model import Money
from nautilus_trader.model import OmsType
from nautilus_trader.model import Price
from nautilus_trader.model import Quantity
from nautilus_trader.model import QuoteTick
from nautilus_trader.model import Symbol
from nautilus_trader.model import Venue
from nautilus_trader.risk import RiskEngineConfig
from nautilus_trader.trading import ImportableStrategyConfig
from nautilus_trader.trading import ScheduledUniverseRule
from nautilus_trader.trading import Universe
from nautilus_trader.trading import UniverseDefinition
from tests.regression.scenario import Checkpoint
from tests.regression.scenario import Scenario
from tests.regression.scenario import backtest_engine


NAME = "universe_membership"
UNIVERSE = "equities"

STRATEGY = "strategies.acceptance:UniverseMembership"
STRATEGY_CONFIG = "strategies.acceptance:UniverseMembershipConfig"


def _equity(symbol: str, isin: str) -> Equity:
    return Equity(
        instrument_id=InstrumentId.from_str(f"{symbol}.XNYS"),
        raw_symbol=Symbol(symbol),
        isin=isin,
        currency=Currency.from_str("USD"),
        price_precision=2,
        price_increment=Price.from_str("0.01"),
        lot_size=Quantity.from_int(100),
        ts_event=0,
        ts_init=0,
    )


AAPL = _equity("AAPL", "US0378331005")
MSFT = _equity("MSFT", "US5949181045")
AAPL_ID = str(AAPL.id)
MSFT_ID = str(MSFT.id)

TS_START = int(datetime(2024, 12, 2, tzinfo=UTC).timestamp() * 1_000_000_000)
INTERVAL_NS = 60 * 60 * 1_000_000_000
QUOTE_COUNT = 8

# One order per instrument, each placed on the selection step that made it active.
CHECKPOINTS = (
    Checkpoint("order", instrument_id=AAPL_ID, ordinal=0),
    Checkpoint("fill", instrument_id=AAPL_ID, ordinal=0),
    Checkpoint("position", instrument_id=AAPL_ID, ordinal=0),
    Checkpoint("order", instrument_id=MSFT_ID, ordinal=0),
    Checkpoint("fill", instrument_id=MSFT_ID, ordinal=0),
    Checkpoint("position", instrument_id=MSFT_ID, ordinal=0),
)


def _engine_config() -> BacktestEngineConfig:
    return BacktestEngineConfig(
        bypass_logging=True,
        run_analysis=False,
        risk_engine=RiskEngineConfig(bypass=True),
    )


def _quotes() -> list[QuoteTick]:
    quotes: list[QuoteTick] = []
    for instrument in (AAPL, MSFT):
        for index in range(QUOTE_COUNT):
            ts = TS_START + index * INTERVAL_NS
            quotes.append(
                QuoteTick(
                    instrument_id=instrument.id,
                    bid_price=Price.from_str("200.00"),
                    ask_price=Price.from_str("200.01"),
                    bid_size=Quantity(10_000, precision=0),
                    ask_size=Quantity(10_000, precision=0),
                    ts_event=ts,
                    ts_init=ts,
                ),
            )
    return quotes


def _universe() -> Universe:
    definition = UniverseDefinition(
        UNIVERSE,
        Venue("XNYS"),
        ScheduledUniverseRule(
            UNIVERSE,
            [
                (TS_START + INTERVAL_NS, [AAPL.id]),
                (TS_START + 3 * INTERVAL_NS, [AAPL.id, MSFT.id]),
                (TS_START + 6 * INTERVAL_NS, [AAPL.id]),
            ],
        ),
    )
    definition.set_selection_interval_ns(INTERVAL_NS)
    return Universe(definition)


def execute() -> Any:
    """
    Run the scenario and return its canonical result.
    """
    xnys = Venue("XNYS")
    usd = Currency.from_str("USD")

    with backtest_engine(_engine_config()) as engine:
        engine.add_venue(
            venue=xnys,
            oms_type=OmsType.NETTING,
            account_type=AccountType.CASH,
            base_currency=usd,
            starting_balances=[Money(1_000_000.0, usd)],
            fee_model=MakerTakerFeeModel(
                maker_rate=Decimal("0"),
                taker_rate=Decimal("0"),
            ),
        )
        engine.add_instrument(AAPL)
        engine.add_instrument(MSFT)
        engine.add_data(_quotes())
        engine.add_universe(_universe())
        engine.add_strategy_from_config(
            ImportableStrategyConfig(
                strategy_path=STRATEGY,
                config_path=STRATEGY_CONFIG,
                config={
                    "universe": UNIVERSE,
                    "trade_size": "100",
                },
            ),
        )
        engine.run(start=TS_START, end=TS_START + QUOTE_COUNT * INTERVAL_NS)
        return engine.get_canonical_result()


SCENARIO = Scenario(
    name=NAME,
    execute=execute,
    checkpoints=CHECKPOINTS,
)
