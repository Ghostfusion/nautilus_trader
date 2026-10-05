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
Tests for positive, three-valued, per-leg tradability.
"""

from __future__ import annotations

from nautilus_trader.decision_bridge.contract import Refusal
from nautilus_trader.decision_bridge.contract import RefusalCode
from nautilus_trader.decision_bridge.tradability import Tradability
from nautilus_trader.decision_bridge.tradability import TradabilityTracker
from nautilus_trader.decision_bridge.tradability import combine
from nautilus_trader.decision_bridge.tradability import from_status
from nautilus_trader.decision_bridge.tradability import require
from nautilus_trader.model import InstrumentId
from nautilus_trader.model import InstrumentStatus
from nautilus_trader.model import MarketStatusAction
from tests.unit.decision_bridge.fixtures import AAPL
from tests.unit.decision_bridge.fixtures import NS
from tests.unit.decision_bridge.fixtures import unix_ns


MSFT = InstrumentId.from_str("MSFT.XNAS")


def status(
    action: MarketStatusAction,
    *,
    instrument_id: InstrumentId = AAPL,
    is_trading: bool | None = None,
    ts: int = 0,
) -> InstrumentStatus:
    """
    Build an instrument status from the given action, instrument, trading flag and timestamp.
    """
    return InstrumentStatus(
        instrument_id=instrument_id,
        action=action,
        ts_event=ts,
        ts_init=ts,
        is_trading=is_trading,
    )


def test_the_venues_own_trading_flag_is_read_first() -> None:
    """
    Test that the venue's own trading flag takes precedence when establishing tradability.
    """
    assert from_status(status(MarketStatusAction.TRADING, is_trading=True)) is Tradability.TRADABLE
    assert from_status(status(MarketStatusAction.TRADING, is_trading=False)) is Tradability.NOT_TRADABLE


def test_only_a_trading_action_positively_establishes_tradability() -> None:
    """
    Test that only a TRADING action positively establishes tradability, others being unknown.
    """
    assert from_status(status(MarketStatusAction.TRADING)) is Tradability.TRADABLE

    for action in (
        MarketStatusAction.NONE,
        MarketStatusAction.PRE_OPEN,
        MarketStatusAction.QUOTING,
        MarketStatusAction.PRE_CROSS,
        MarketStatusAction.CROSS,
        MarketStatusAction.ROTATION,
        MarketStatusAction.NEW_PRICE_INDICATION,
    ):
        assert from_status(status(action)) is Tradability.UNKNOWN, action


def test_a_refusing_action_is_not_tradable_and_never_merely_unmapped() -> None:
    """
    Test that refusing market actions map to not tradable rather than unmapped.
    """
    for action in (
        MarketStatusAction.CLOSE,
        MarketStatusAction.PRE_CLOSE,
        MarketStatusAction.POST_CLOSE,
        MarketStatusAction.HALT,
        MarketStatusAction.PAUSE,
        MarketStatusAction.SUSPEND,
        MarketStatusAction.NOT_AVAILABLE_FOR_TRADING,
    ):
        assert from_status(status(action)) is Tradability.NOT_TRADABLE, action


def test_an_unrecognised_status_establishes_nothing() -> None:
    """
    Test that an unrecognised status with no trading flag establishes nothing.
    """
    assert from_status(status(MarketStatusAction.NOT_AVAILABLE_FOR_TRADING, is_trading=None)) is (
        Tradability.NOT_TRADABLE
    )
    assert from_status(status(MarketStatusAction.NONE, is_trading=None)) is Tradability.UNKNOWN


def test_unknown_is_not_permission() -> None:
    """
    Test that only tradable status permits while unknown and not tradable refuse.
    """
    assert require(Tradability.TRADABLE, AAPL) is None

    unknown = require(Tradability.UNKNOWN, AAPL)
    assert isinstance(unknown, Refusal)
    assert unknown.code is RefusalCode.TRADABILITY_UNKNOWN

    rejected = require(Tradability.NOT_TRADABLE, AAPL)
    assert isinstance(rejected, Refusal)
    assert rejected.code is RefusalCode.TRADABILITY_REJECTED


def test_a_leg_that_cannot_trade_makes_the_instrument_untradable() -> None:
    """
    Test that a single non-tradable leg makes the combined instrument untradable.
    """
    assert combine([Tradability.TRADABLE, Tradability.TRADABLE]) is Tradability.TRADABLE
    assert combine([Tradability.TRADABLE, Tradability.NOT_TRADABLE]) is Tradability.NOT_TRADABLE
    assert combine([Tradability.TRADABLE, Tradability.UNKNOWN]) is Tradability.UNKNOWN
    assert combine([Tradability.UNKNOWN, Tradability.NOT_TRADABLE]) is Tradability.NOT_TRADABLE
    assert combine([]) is Tradability.UNKNOWN


def test_a_two_leg_instrument_with_one_unknown_leg_is_not_tradable() -> None:
    """
    Test that a two-leg instrument with one unknown leg is not tradable.
    """
    tracker = TradabilityTracker()
    tracker.observe(status(MarketStatusAction.TRADING, instrument_id=AAPL, ts=unix_ns(2025, 6, 2, 17)))

    assert tracker.state_of_legs([AAPL, MSFT]) is Tradability.UNKNOWN
    assert tracker.state_of_legs([AAPL]) is Tradability.TRADABLE


def test_an_instrument_that_never_reports_a_status_is_unknown() -> None:
    """
    Test that an instrument that never reports a status remains unknown.
    """
    tracker = TradabilityTracker()

    assert tracker.state_of(AAPL) is Tradability.UNKNOWN
    assert tracker.state_of_legs([AAPL, MSFT]) is Tradability.UNKNOWN


def test_a_status_change_between_admission_and_submission_is_honoured() -> None:
    """
    Test that a status change between admission and submission is honoured.
    """
    tracker = TradabilityTracker()
    tracker.observe(status(MarketStatusAction.TRADING, ts=unix_ns(2025, 6, 2, 17)))
    assert tracker.state_of(AAPL) is Tradability.TRADABLE

    tracker.observe(
        status(MarketStatusAction.NOT_AVAILABLE_FOR_TRADING, ts=unix_ns(2025, 6, 2, 18)),
    )
    assert tracker.state_of(AAPL) is Tradability.NOT_TRADABLE

    assert require(tracker.state_of(AAPL), AAPL) is not None


def test_a_stale_status_never_replaces_a_newer_one() -> None:
    """
    Test that a stale status does not replace a newer one.
    """
    tracker = TradabilityTracker()
    newest = unix_ns(2025, 6, 2, 18)
    tracker.observe(status(MarketStatusAction.NOT_AVAILABLE_FOR_TRADING, ts=newest))
    tracker.observe(status(MarketStatusAction.TRADING, ts=newest - 60 * NS))

    assert tracker.state_of(AAPL) is Tradability.NOT_TRADABLE
