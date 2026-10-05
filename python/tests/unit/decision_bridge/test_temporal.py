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
Tests for the actionability resolution.
"""

from __future__ import annotations

from dataclasses import dataclass
from datetime import UTC
from datetime import date
from datetime import datetime

import pytest

from nautilus_trader.decision_bridge.contract import Diagnostic
from nautilus_trader.decision_bridge.contract import Refusal
from nautilus_trader.decision_bridge.contract import RefusalCode
from nautilus_trader.decision_bridge.temporal import Actionability
from nautilus_trader.decision_bridge.temporal import resolve_actionability
from nautilus_trader.model import TradingCalendar


NS = 1_000_000_000


def unix_ns(year: int, month: int, day: int, hour: int = 0, minute: int = 0) -> int:
    """
    Convert a UTC calendar date and time into a Unix nanosecond timestamp.
    """
    return int(datetime(year, month, day, hour, minute, tzinfo=UTC).timestamp()) * NS


@dataclass(frozen=True)
class StubCalendar:
    """
    A calendar exposing only the three answers the resolver reads, so each branch is reachable.
    """

    coverage_from: int
    coverage_to: int
    session_from: int
    session_to: int
    next_open_at: int | None

    def covers(self, _ts_ns: int) -> bool:
        """
        Return whether the instant falls inside the inclusive coverage window.
        """
        return self.coverage_from <= _ts_ns <= self.coverage_to

    def is_tradeable(self, ts_ns: int) -> bool:
        """
        Return whether the instant falls inside the half-open session window.
        """
        return self.session_from <= ts_ns < self.session_to

    def next_open(self, _ts_ns: int) -> int | None:
        """
        Return the configured next open instant, or None when the stub has none.
        """
        return self.next_open_at


MAX_NS = 4_000_000_000 * NS


def stub(*, session: bool, next_open_at: int | None = None, coverage_from: int = 0, coverage_to: int = MAX_NS) -> StubCalendar:
    """
    Build a stub calendar whose session spans the coverage window or sits just past it.
    """
    if session:
        return StubCalendar(
            coverage_from=coverage_from,
            coverage_to=coverage_to,
            session_from=coverage_from,
            session_to=coverage_to,
            next_open_at=next_open_at,
        )

    return StubCalendar(
        coverage_from=coverage_from,
        coverage_to=coverage_to,
        session_from=coverage_to + 1,
        session_to=coverage_to + 2,
        next_open_at=next_open_at,
    )


def bundled_nasdaq() -> TradingCalendar:
    """
    Load the bundled XNYS equity calendar, skipping the test when it is unavailable.
    """
    calendar = TradingCalendar.bundled("XNYS", "EQUITY")
    if calendar is None:
        pytest.skip("the bundled XNYS.EQUITY calendar is not present in this build")

    return calendar


def test_an_absent_production_instant_refuses_by_name() -> None:
    """
    Test that a missing production instant is refused with the PRODUCED_AT_ABSENT code.
    """
    result = resolve_actionability(
        produced_at=None,
        expires_at=unix_ns(2026, 1, 2, 20),
        received_at=0,
        calendar=stub(session=True),
    )

    assert isinstance(result, Refusal)
    assert result.code is RefusalCode.PRODUCED_AT_ABSENT


def test_an_absent_calendar_is_not_an_uncovered_instant() -> None:
    """
    Test that a missing calendar is refused as CALENDAR_MISSING rather than CALENDAR_UNCOVERED.
    """
    result = resolve_actionability(
        produced_at=unix_ns(2026, 1, 2, 17),
        expires_at=unix_ns(2026, 1, 2, 20),
        received_at=0,
        calendar=None,
    )

    assert isinstance(result, Refusal)
    assert result.code is RefusalCode.CALENDAR_MISSING


def test_an_instant_the_calendar_does_not_cover_refuses() -> None:
    """
    Test that an instant outside the calendar's coverage is refused as CALENDAR_UNCOVERED.
    """
    produced = unix_ns(2023, 12, 31, 17)
    result = resolve_actionability(
        produced_at=produced,
        expires_at=unix_ns(2026, 1, 2, 20),
        received_at=0,
        calendar=stub(session=True, coverage_from=unix_ns(2024, 1, 1)),
    )

    assert isinstance(result, Refusal)
    assert result.code is RefusalCode.CALENDAR_UNCOVERED


def test_a_missing_expiry_refuses_rather_than_being_derived() -> None:
    """
    Test that a missing expiry is refused with MISSING_EXPIRY rather than being derived.
    """
    result = resolve_actionability(
        produced_at=unix_ns(2026, 1, 2, 17),
        expires_at=None,
        received_at=0,
        calendar=stub(session=True),
    )

    assert isinstance(result, Refusal)
    assert result.code is RefusalCode.MISSING_EXPIRY


def test_production_at_or_after_the_expiry_refuses() -> None:
    """
    Test that production at or after the expiry is refused as ARTIFACT_EXPIRED.
    """
    produced = unix_ns(2026, 1, 2, 17)
    result = resolve_actionability(
        produced_at=produced,
        expires_at=produced,
        received_at=0,
        calendar=stub(session=True),
    )

    assert isinstance(result, Refusal)
    assert result.code is RefusalCode.ARTIFACT_EXPIRED


def test_an_untradeable_instant_with_no_next_open_refuses() -> None:
    """
    Test that an untradeable instant with no next open is refused as ACTIONABILITY_INVALID.
    """
    result = resolve_actionability(
        produced_at=unix_ns(2026, 1, 2, 17),
        expires_at=unix_ns(2026, 1, 3, 20),
        received_at=0,
        calendar=stub(session=False, next_open_at=None),
    )

    assert isinstance(result, Refusal)
    assert result.code is RefusalCode.ACTIONABILITY_INVALID


def test_an_instant_earlier_than_receipt_is_clamped_and_diagnosed() -> None:
    """
    Test that an instant before receipt is clamped to the receipt and diagnosed as past.
    """
    produced = unix_ns(2026, 1, 2, 17)
    received = unix_ns(2026, 1, 2, 18)
    result = resolve_actionability(
        produced_at=produced,
        expires_at=unix_ns(2026, 1, 3, 20),
        received_at=received,
        calendar=stub(session=True),
    )

    assert isinstance(result, Actionability)
    assert result.actionable_at == received
    assert result.diagnostics == (Diagnostic.ACTIONABILITY_PAST,)


def test_receipt_at_or_after_the_expiry_refuses() -> None:
    """
    Test that a receipt at or after the expiry is refused as ARTIFACT_EXPIRED.
    """
    produced = unix_ns(2026, 1, 2, 17)
    expires = unix_ns(2026, 1, 2, 18)
    result = resolve_actionability(
        produced_at=produced,
        expires_at=expires,
        received_at=expires,
        calendar=stub(session=True),
    )

    assert isinstance(result, Refusal)
    assert result.code is RefusalCode.ARTIFACT_EXPIRED


def test_an_instant_equal_to_the_expiry_refuses_because_expiry_is_exclusive() -> None:
    """
    Test that an instant equal to the expiry is refused because the expiry bound is exclusive.
    """
    produced = unix_ns(2026, 1, 2, 17)
    expires = unix_ns(2026, 1, 2, 17) + 3600 * NS
    result = resolve_actionability(
        produced_at=produced,
        expires_at=expires,
        received_at=expires,
        calendar=stub(session=True),
    )

    assert isinstance(result, Refusal)
    assert result.code is RefusalCode.ARTIFACT_EXPIRED


def test_production_after_a_session_close_resolves_to_the_next_open() -> None:
    """
    Test that production after a session close resolves to the next session's open.
    """
    calendar = bundled_nasdaq()

    # 2025-06-02 21:00 UTC is 17:00 ET, after the regular session close.
    produced = unix_ns(2025, 6, 2, 21)
    result = resolve_actionability(
        produced_at=produced,
        expires_at=unix_ns(2025, 6, 6, 20),
        received_at=produced,
        calendar=calendar,
    )

    assert isinstance(result, Actionability)
    # 2025-06-03 13:30 UTC is the next session's open, 09:30 ET.
    assert result.actionable_at == unix_ns(2025, 6, 3, 13, 30)


def test_production_during_a_session_resolves_to_its_own_instant() -> None:
    """
    Test that production during a session resolves to that same instant with no diagnostics.
    """
    calendar = bundled_nasdaq()

    produced = unix_ns(2025, 6, 2, 17)
    result = resolve_actionability(
        produced_at=produced,
        expires_at=unix_ns(2025, 6, 6, 20),
        received_at=produced,
        calendar=calendar,
    )

    assert isinstance(result, Actionability)
    assert result.actionable_at == produced
    assert result.diagnostics == ()


def test_a_reference_date_timestamp_would_grant_the_hours_before_production() -> None:
    """
    The worked example: resolving availability at the reference date is not a convention.

    It is hours of foresight, and the bridge refuses to grant them.
    """
    calendar = bundled_nasdaq()

    produced = unix_ns(2025, 6, 2, 17)
    midnight = int(
        datetime(2025, 6, 2, tzinfo=UTC).timestamp(),
    ) * NS
    foresight_hours = (produced - midnight) / (3600 * NS)

    assert foresight_hours == 17.0

    result = resolve_actionability(
        produced_at=produced,
        expires_at=unix_ns(2025, 6, 6, 20),
        received_at=produced,
        calendar=calendar,
    )

    assert isinstance(result, Actionability)
    assert result.actionable_at == produced
    assert result.actionable_at - midnight == int(foresight_hours * 3600 * NS)
    assert date(2025, 6, 2) == date.fromisoformat("2025-06-02")
