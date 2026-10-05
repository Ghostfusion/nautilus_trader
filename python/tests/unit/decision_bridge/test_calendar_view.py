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
Tests for the path-backed calendar view.
"""

from __future__ import annotations

import json
from datetime import UTC
from datetime import datetime
from pathlib import Path

import pytest

from nautilus_trader.decision_bridge.contract import Refusal
from nautilus_trader.decision_bridge.contract import RefusalCode
from nautilus_trader.decision_bridge.temporal import Actionability
from nautilus_trader.decision_bridge.temporal import JsonCalendarView
from nautilus_trader.decision_bridge.temporal import resolve_actionability
from nautilus_trader.model import TradingCalendar


NS = 1_000_000_000

WEEKDAYS = ("monday", "tuesday", "wednesday", "thursday", "friday")
# A regular 09:30-16:00 exchange session, the shape the bundled XNYS.EQUITY document declares.
SESSIONS = {day: [{"start": "09:30:00", "end": "16:00:00"}] for day in WEEKDAYS}


def unix_ns(year: int, month: int, day: int, hour: int = 0, minute: int = 0) -> int:
    """
    Return the UNIX nanoseconds of a UTC instant.
    """
    return int(datetime(year, month, day, hour, minute, tzinfo=UTC).timestamp()) * NS


def write_document(
    directory: Path,
    *,
    filename: str,
    venue: str,
    asset_class: str,
    valid_from: str,
    valid_until: str,
    symbol: str | None = None,
) -> Path:
    """
    Write a minimal `nautilus-trading-calendar/v1` document into a directory.

    The literal is modelled on `crates/model/resources/calendars/xnys-equity.json`: a regular weekday
    session, no holidays and no early closes, so coverage is the only dimension the tests vary.
    """
    document: dict[str, object] = {
        "schema": "nautilus-trading-calendar/v1",
        "venue": venue,
        "asset_class": asset_class,
        "time_zone": "America/New_York",
        "sessions": SESSIONS,
        "holidays": [],
        "early_closes": {},
        "valid_from": valid_from,
        "valid_until": valid_until,
        "source": "a locally supplied calendar document, for the path-backed view test",
    }
    if symbol is not None:
        document["symbol"] = symbol

    path = directory / filename
    path.write_text(json.dumps(document), encoding="utf-8")

    return path


def test_a_key_outside_the_bundled_set_resolves_through_the_view(tmp_path: Path) -> None:
    """
    Test that a document for a key the bundled data does not carry resolves and is actionable.
    """
    write_document(
        tmp_path,
        filename="xnas-aapl-equity.json",
        venue="XNAS",
        asset_class="EQUITY",
        symbol="AAPL",
        valid_from="2026-01-01",
        valid_until="2026-12-31",
    )

    view = JsonCalendarView(tmp_path)
    calendar = view.calendar_for("XNAS", "EQUITY", "AAPL")

    assert calendar is not None
    assert calendar.key == "XNAS.EQUITY.AAPL"
    # The bundled set carries no such key, so the document is the only source for it.
    assert TradingCalendar.bundled("XNAS", "EQUITY", "AAPL") is None
    assert view.keys == ("XNAS.EQUITY.AAPL",)

    produced = unix_ns(2026, 6, 2, 17)
    result = resolve_actionability(
        produced_at=produced,
        expires_at=unix_ns(2026, 6, 6, 20),
        received_at=produced,
        calendar=calendar,
    )

    assert isinstance(result, Actionability)
    assert result.actionable_at == produced


def test_an_instant_beyond_the_document_is_refused_as_uncovered(tmp_path: Path) -> None:
    """
    Test that an instant past the document's valid_until is refused by coverage, not tradeability.
    """
    write_document(
        tmp_path,
        filename="xnas-aapl-equity.json",
        venue="XNAS",
        asset_class="EQUITY",
        symbol="AAPL",
        valid_from="2026-01-01",
        valid_until="2026-06-30",
    )

    calendar = JsonCalendarView(tmp_path).calendar_for("XNAS", "EQUITY", "AAPL")
    assert calendar is not None

    # 2026-07-02 is a Thursday, so the session shape is tradeable; only coverage is absent.
    produced = unix_ns(2026, 7, 2, 17)
    assert calendar.covers(produced) is False

    result = resolve_actionability(
        produced_at=produced,
        expires_at=unix_ns(2026, 7, 6, 20),
        received_at=produced,
        calendar=calendar,
    )

    assert isinstance(result, Refusal)
    assert result.code is RefusalCode.CALENDAR_UNCOVERED


def test_a_malformed_document_leaves_its_key_missing_without_raising(tmp_path: Path) -> None:
    """
    Test that an unparseable document becomes CALENDAR_MISSING, not an exception from the view.
    """
    write_document(
        tmp_path,
        filename="xnas-aapl-equity.json",
        venue="XNAS",
        asset_class="EQUITY",
        symbol="AAPL",
        valid_from="2026-01-01",
        valid_until="2026-12-31",
    )
    # A document that names a key but declares no sessions fails the calendar loader.
    broken = {
        "schema": "nautilus-trading-calendar/v1",
        "venue": "XNAS",
        "asset_class": "EQUITY",
        "symbol": "MSFT",
        "time_zone": "America/New_York",
        "sessions": {},
        "holidays": [],
        "early_closes": {},
        "valid_from": "2026-01-01",
        "valid_until": "2026-12-31",
    }
    (tmp_path / "xnas-msft-equity.json").write_text(json.dumps(broken), encoding="utf-8")

    # Construction must not raise, and the well-formed document beside it still resolves.
    view = JsonCalendarView(tmp_path)
    assert view.calendar_for("XNAS", "EQUITY", "AAPL") is not None
    assert view.calendar_for("XNAS", "EQUITY", "MSFT") is None

    result = resolve_actionability(
        produced_at=unix_ns(2026, 6, 2, 17),
        expires_at=unix_ns(2026, 6, 6, 20),
        received_at=unix_ns(2026, 6, 2, 17),
        calendar=view.calendar_for("XNAS", "EQUITY", "MSFT"),
    )

    assert isinstance(result, Refusal)
    assert result.code is RefusalCode.CALENDAR_MISSING


def test_two_documents_for_the_same_key_raise_at_construction(tmp_path: Path) -> None:
    """
    Test that a duplicate key is a configuration error rather than a silent preference.
    """
    for filename in ("first.json", "second.json"):
        write_document(
            tmp_path,
            filename=filename,
            venue="XNAS",
            asset_class="EQUITY",
            symbol="AAPL",
            valid_from="2026-01-01",
            valid_until="2026-12-31",
        )

    with pytest.raises(ValueError, match="duplicate calendar key"):
        JsonCalendarView(tmp_path)


def test_a_directory_entry_takes_precedence_over_the_bundled_calendar(tmp_path: Path) -> None:
    """
    Test that a document sharing a bundled key wins, so coverage can extend past the bundle.
    """
    bundled = TradingCalendar.bundled("XNYS", "EQUITY")
    if bundled is None:
        pytest.skip("the bundled XNYS.EQUITY calendar is not present in this build")

    write_document(
        tmp_path,
        filename="xnys-equity-beyond.json",
        venue="XNYS",
        asset_class="EQUITY",
        valid_from="2026-01-01",
        valid_until="2027-12-31",
    )

    calendar = JsonCalendarView(tmp_path).calendar_for("XNYS", "EQUITY")

    assert calendar is not None
    # The same key, two sources: the caller composes the directory entry first, by choice.
    assert calendar.key == bundled.key == "XNYS.EQUITY"
    assert calendar.valid_until == "2027-12-31"

    # An instant beyond whatever the bundle covers can only be served by the document, which is the
    # limitation the directory entry removes. The bundle's own window is asked about rather than
    # pinned, so a later data refresh does not invalidate this test.
    produced = unix_ns(2027, 3, 15, 17)
    assert bundled.covers(produced) is False

    result = resolve_actionability(
        produced_at=produced,
        expires_at=unix_ns(2027, 3, 19, 20),
        received_at=produced,
        calendar=calendar,
    )

    assert isinstance(result, Actionability)
    assert result.actionable_at == produced
