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
A session-aware strategy over a holiday and an early close.

The strategy schedules the bundled `XNYS` equity calendar for the run window and submits one market
order per session event. The window covers 2024-11-28, a full holiday with no session events, and
2024-11-29, an early close whose phases all move with the shortened session. The expected canonical
result therefore pins the event set the calendar produced, including the `EarlyClose` phase.

The instrument is a US equity listed on `XNYS` so the calendar key matches the venue the orders are
routed to; the calendar is supplied to the strategy explicitly, never inferred from the venue.
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
from tests.regression.scenario import Checkpoint
from tests.regression.scenario import Scenario
from tests.regression.scenario import backtest_engine


NAME = "equity_session_events"

STRATEGY = "strategies.acceptance:SessionEventStrategy"
STRATEGY_CONFIG = "strategies.acceptance:SessionEventConfig"

AAPL = Equity(
    instrument_id=InstrumentId.from_str("AAPL.XNYS"),
    raw_symbol=Symbol("AAPL"),
    isin="US0378331005",
    currency=Currency.from_str("USD"),
    price_precision=2,
    price_increment=Price.from_str("0.01"),
    lot_size=Quantity.from_int(100),
    ts_event=0,
    ts_init=0,
)
AAPL_ID = str(AAPL.id)

TS_START = int(datetime(2024, 11, 27, tzinfo=UTC).timestamp() * 1_000_000_000)
TS_END = int(datetime(2024, 12, 2, tzinfo=UTC).timestamp() * 1_000_000_000)
INTERVAL_NS = 6 * 60 * 60 * 1_000_000_000
QUOTE_COUNT = 21

# The calendar derives twelve session events in the window: six on 2024-11-27 and six on the
# 2024-11-29 early close, with nothing on the holiday of 2024-11-28 or the weekend.
CHECKPOINTS = (
    Checkpoint("order", instrument_id=AAPL_ID, ordinal=0),
    Checkpoint("order", instrument_id=AAPL_ID, ordinal=1),
    Checkpoint("fill", instrument_id=AAPL_ID, ordinal=0),
    Checkpoint("position", instrument_id=AAPL_ID, ordinal=0),
    # The last event of the window: the early close of 2024-11-29.
    Checkpoint("order", instrument_id=AAPL_ID, ordinal=11),
)


def _engine_config() -> BacktestEngineConfig:
    return BacktestEngineConfig(
        bypass_logging=True,
        run_analysis=False,
        risk_engine=RiskEngineConfig(bypass=True),
    )


def _quotes() -> list[QuoteTick]:
    quotes: list[QuoteTick] = []
    for index in range(QUOTE_COUNT):
        ts = TS_START + index * INTERVAL_NS
        quotes.append(
            QuoteTick(
                instrument_id=AAPL.id,
                bid_price=Price.from_str("200.00"),
                ask_price=Price.from_str("200.01"),
                bid_size=Quantity(10_000, precision=0),
                ask_size=Quantity(10_000, precision=0),
                ts_event=ts,
                ts_init=ts,
            ),
        )
    return quotes


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
        engine.add_data(_quotes())
        engine.add_strategy_from_config(
            ImportableStrategyConfig(
                strategy_path=STRATEGY,
                config_path=STRATEGY_CONFIG,
                config={
                    "instrument_id": AAPL_ID,
                    "venue": "XNYS",
                    "asset_class": "EQUITY",
                    "trade_size": "100",
                    "schedule_to_ns": TS_END,
                },
            ),
        )
        engine.run(start=TS_START, end=TS_END)
        return engine.get_canonical_result()


SCENARIO = Scenario(
    name=NAME,
    execute=execute,
    checkpoints=CHECKPOINTS,
)
