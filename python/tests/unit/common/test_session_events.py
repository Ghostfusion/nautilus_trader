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
Tests for calendar session events on a Python data actor.
"""

from __future__ import annotations

from datetime import UTC
from datetime import datetime
from decimal import Decimal
from typing import ClassVar

from nautilus_trader.backtest import BacktestEngine
from nautilus_trader.backtest import BacktestEngineConfig
from nautilus_trader.common import DataActor
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
from nautilus_trader.model import SessionEvent
from nautilus_trader.model import SessionEventKind
from nautilus_trader.model import SessionScheduleConfig
from nautilus_trader.model import Symbol
from nautilus_trader.model import TradingCalendar
from nautilus_trader.model import Venue
from nautilus_trader.risk import RiskEngineConfig


def _nanos(value: str) -> int:
    """
    Return the Unix nanosecond timestamp for an ISO 8601 UTC value.
    """
    return int(datetime.fromisoformat(value).replace(tzinfo=UTC).timestamp() * 1_000_000_000)


SESSION_CONFIG = SessionScheduleConfig(
    premarket_offset_ns=3_600_000_000_000,
    opening_range_ns=1_800_000_000_000,
    pre_close_offset_ns=1_800_000_000_000,
)

TWO_HOURS_NS = 7_200_000_000_000
TS_START = _nanos("2024-06-03T00:00:00")
TS_END = _nanos("2024-06-04T00:00:00")
TS_EARLY_CLOSE_START = _nanos("2024-11-29T00:00:00")
TS_EARLY_CLOSE_END = _nanos("2024-11-30T00:00:00")

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


class SessionEventProbeActor(DataActor):
    """
    Record the session events scheduled and dispatched on a Python data actor.
    """

    scheduled: ClassVar[int] = 0
    received: ClassVar[list[SessionEvent]] = []

    def on_start(self) -> None:
        """
        Schedule the bundled `XNYS` equity calendar for the test window.
        """
        type(self).received = []
        type(self).scheduled = self.schedule_session_events(
            TradingCalendar.bundled("XNYS", "EQUITY"),
            SESSION_CONFIG,
            TS_END,
        )

    def on_session_event(self, event: SessionEvent) -> None:
        """
        Record a dispatched session event.
        """
        type(self).received.append(event)


def _quotes() -> list[QuoteTick]:
    quotes: list[QuoteTick] = []
    for index in range(12):
        ts = TS_START + index * TWO_HOURS_NS
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


def test_bundled_equity_calendar_reports_the_documented_full_session() -> None:
    """
    Test the bundled `XNYS` equity calendar derives the documented phases for a full day.
    """
    calendar = TradingCalendar.bundled("XNYS", "EQUITY")

    events = calendar.session_events(TS_START, TS_END, SESSION_CONFIG)

    assert [event.kind for event in events] == [
        SessionEventKind.PREMARKET,
        SessionEventKind.OPEN,
        SessionEventKind.OPENING_RANGE_COMPLETE,
        SessionEventKind.MIDDAY,
        SessionEventKind.PRE_CLOSE,
        SessionEventKind.CLOSE,
    ]
    # 2024-06-03 is daylight time: 09:30-16:00 local is 13:30-20:00 UTC.
    assert [event.ts_event for event in events] == [
        _nanos("2024-06-03T12:30:00"),
        _nanos("2024-06-03T13:30:00"),
        _nanos("2024-06-03T14:00:00"),
        _nanos("2024-06-03T16:45:00"),
        _nanos("2024-06-03T19:30:00"),
        _nanos("2024-06-03T20:00:00"),
    ]


def test_bundled_equity_calendar_reports_the_documented_early_close() -> None:
    """
    Test the bundled `XNYS` equity calendar derives the documented 2024-11-29 early close.
    """
    calendar = TradingCalendar.bundled("XNYS", "EQUITY")
    config = SessionScheduleConfig(
        premarket_offset_ns=3_600_000_000_000,
        opening_range_ns=1_800_000_000_000,
        pre_close_offset_ns=1_800_000_000_000,
        kinds=[
            SessionEventKind.OPEN,
            SessionEventKind.MIDDAY,
            SessionEventKind.PRE_CLOSE,
            SessionEventKind.EARLY_CLOSE,
        ],
    )

    events = calendar.session_events(TS_EARLY_CLOSE_START, TS_EARLY_CLOSE_END, config)

    assert [event.kind for event in events] == [
        SessionEventKind.OPEN,
        SessionEventKind.MIDDAY,
        SessionEventKind.PRE_CLOSE,
        SessionEventKind.EARLY_CLOSE,
    ]
    # Thanksgiving Friday 2024 closes at 13:00 local (18:00 UTC).
    assert [event.ts_event for event in events] == [
        _nanos("2024-11-29T14:30:00"),
        _nanos("2024-11-29T16:15:00"),
        _nanos("2024-11-29T17:30:00"),
        _nanos("2024-11-29T18:00:00"),
    ]


def test_data_actor_receives_scheduled_session_events_as_the_clock_advances() -> None:
    """
    Test a Python data actor receives scheduled session events as the virtual clock advances.
    """
    engine = BacktestEngine(
        BacktestEngineConfig(
            bypass_logging=True,
            run_analysis=False,
            risk_engine=RiskEngineConfig(bypass=True),
        ),
    )
    xnys = Venue("XNYS")
    usd = Currency.from_str("USD")
    actor = SessionEventProbeActor()

    try:
        engine.add_venue(
            venue=xnys,
            oms_type=OmsType.NETTING,
            account_type=AccountType.CASH,
            base_currency=usd,
            starting_balances=[Money(1_000_000.0, usd)],
            fee_model=MakerTakerFeeModel(maker_rate=Decimal(0), taker_rate=Decimal(0)),
        )
        engine.add_instrument(AAPL)
        engine.add_data(_quotes())
        engine.add_actor(actor)
        engine.run(start=TS_START, end=TS_END)
    finally:
        engine.dispose()

    assert SessionEventProbeActor.scheduled == 6
    assert [event.kind for event in SessionEventProbeActor.received] == [
        SessionEventKind.PREMARKET,
        SessionEventKind.OPEN,
        SessionEventKind.OPENING_RANGE_COMPLETE,
        SessionEventKind.MIDDAY,
        SessionEventKind.PRE_CLOSE,
        SessionEventKind.CLOSE,
    ]
    assert [event.ts_event for event in SessionEventProbeActor.received] == [
        _nanos("2024-06-03T12:30:00"),
        _nanos("2024-06-03T13:30:00"),
        _nanos("2024-06-03T14:00:00"),
        _nanos("2024-06-03T16:45:00"),
        _nanos("2024-06-03T19:30:00"),
        _nanos("2024-06-03T20:00:00"),
    ]
    assert SessionEventProbeActor.received[0].name() == (
        "SESSION-PREMARKET:XNYS.EQUITY:2024-06-03:0"
    )
