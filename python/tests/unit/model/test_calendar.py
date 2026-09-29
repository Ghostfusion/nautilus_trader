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
Tests for trading calendars.
"""

from __future__ import annotations

import json
from datetime import UTC
from datetime import datetime
from pathlib import Path

import pytest

from nautilus_trader.model import TradingCalendar


def _nanos(value: str) -> int:
    return int(datetime.fromisoformat(value).replace(tzinfo=UTC).timestamp() * 1_000_000_000)


USER_CALENDAR = {
    "schema": "nautilus-trading-calendar/v1",
    "venue": "XNYS",
    "asset_class": "EQUITY",
    "time_zone": "America/New_York",
    "sessions": {
        "monday": [{"start": "09:00:00", "end": "15:00:00"}],
        "tuesday": [{"start": "09:00:00", "end": "15:00:00"}],
        "wednesday": [{"start": "09:00:00", "end": "15:00:00"}],
        "thursday": [{"start": "09:00:00", "end": "15:00:00"}],
        "friday": [{"start": "09:00:00", "end": "15:00:00"}],
    },
    "holidays": ["2024-11-28"],
    "early_closes": {"2024-11-29": "13:00:00"},
    "valid_from": "2024-01-01",
    "valid_until": "2024-12-31",
}


def test_bundled_calendar_exposes_documented_fx_sessions() -> None:
    """
    Test a bundled FX calendar resolves its documented local session.
    """
    calendar = TradingCalendar.bundled("FX", "FX", "LONDON")

    assert calendar is not None
    assert calendar.key == "FX.FX.LONDON"
    assert calendar.time_zone == "Europe/London"
    assert calendar.sessions_on("2024-06-03") == [("08:00:00", "16:00:00")]
    assert calendar.is_tradeable(_nanos("2024-06-03T07:00:00"))
    assert not calendar.is_tradeable(_nanos("2024-06-03T06:00:00"))


def test_bundled_calendar_for_an_unknown_key_is_none() -> None:
    """
    Test an unbundled venue and class resolves no calendar.
    """
    assert TradingCalendar.bundled("XNAS", "EQUITY") is None


def test_user_calendar_overrides_the_bundled_one(tmp_path: Path) -> None:
    """
    Test a user calendar for a bundled key resolves its own sessions.
    """
    path = tmp_path / "user-calendar.json"
    path.write_text(json.dumps(USER_CALENDAR), encoding="utf-8")

    calendar = TradingCalendar.from_json_path(str(path))
    bundled = TradingCalendar.bundled("XNYS", "EQUITY")

    assert bundled is not None
    # The bundled calendar trades the standard session, the user calendar a custom one.
    assert bundled.sessions_on("2024-06-03") == [("09:30:00", "16:00:00")]
    assert calendar.sessions_on("2024-06-03") == [("09:00:00", "15:00:00")]

    assert calendar.key == "XNYS.EQUITY"
    assert calendar.valid_from == "2024-01-01"
    assert calendar.valid_until == "2024-12-31"
    assert calendar.is_holiday("2024-11-28")
    assert not calendar.is_trading_day("2024-11-28")
    assert calendar.early_close("2024-11-29") == "13:00:00"
    assert calendar.sessions_on("2024-11-29") == [("09:00:00", "13:00:00")]
    assert calendar.next_open(_nanos("2024-11-28T15:00:00")) == _nanos("2024-11-29T14:00:00")
    assert calendar.prev_close(_nanos("2024-11-30T12:00:00")) == _nanos("2024-11-29T18:00:00")
    assert calendar.covers(_nanos("2024-06-03T12:00:00"))
    assert not calendar.covers(_nanos("2025-06-03T12:00:00"))


def test_bundled_equity_calendar_resolves_a_holiday_and_a_half_day() -> None:
    """
    Test the bundled equity calendar resolves holidays and an early close.
    """
    calendar = TradingCalendar.bundled("XNYS", "EQUITY")

    assert calendar is not None
    assert calendar.is_holiday("2024-11-28")
    assert calendar.is_holiday("2025-01-09")
    assert calendar.sessions_on("2024-11-29") == [("09:30:00", "13:00:00")]


def test_invalid_calendar_json_raises_value_error() -> None:
    """
    Test an unsupported schema raises a value error.
    """
    unsupported = dict(USER_CALENDAR, schema="nautilus-trading-calendar/v9")

    with pytest.raises(ValueError, match="unsupported trading calendar schema"):
        TradingCalendar.from_json_str(json.dumps(unsupported))


def test_unsupported_asset_class_raises_value_error() -> None:
    """
    Test an unsupported asset class raises a value error.
    """
    with pytest.raises(ValueError, match="unsupported asset class"):
        TradingCalendar.bundled("FX", "NOT_A_CLASS")
