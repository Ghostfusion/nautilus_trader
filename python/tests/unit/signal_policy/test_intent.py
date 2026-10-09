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
Tests for signal intents and their sides.

The mapping back onto a `TradingSignal` is checked in both directions: what an open and a close become,
and that a scaling side is refused rather than flattened into a target that would replace the position
instead of adding to or trimming it.
"""

import math
from typing import Any

import pytest

from nautilus_trader.model import InstrumentId
from nautilus_trader.model import SignalDirection
from nautilus_trader.signal_policy import PositionRequirement
from nautilus_trader.signal_policy import PositionSide
from nautilus_trader.signal_policy import SignalIntent
from nautilus_trader.signal_policy import SignalSide


def _intent(side: SignalSide, **kwargs: Any) -> SignalIntent:
    """
    Return an intention on one test instrument.
    """
    return SignalIntent(instrument_id=InstrumentId.from_str("BTCUSDT.BINANCE"), side=side, **kwargs)


def test_the_sides_are_classified_by_what_they_do() -> None:
    """
    Test the transitions are grouped into opens, closes, scales, entries and exits.
    """
    assert SignalSide.OPEN_LONG.is_open
    assert SignalSide.OPEN_SHORT.is_open
    assert not SignalSide.CLOSE_LONG.is_open

    assert SignalSide.CLOSE_LONG.is_close
    assert SignalSide.CLOSE_SHORT.is_close
    assert not SignalSide.SCALE_OUT.is_close

    assert SignalSide.SCALE_IN.is_scale
    assert SignalSide.SCALE_OUT.is_scale
    assert not SignalSide.OPEN_LONG.is_scale

    assert SignalSide.OPEN_LONG.is_long
    assert SignalSide.CLOSE_LONG.is_long
    assert not SignalSide.OPEN_SHORT.is_long

    assert SignalSide.OPEN_SHORT.is_short
    assert SignalSide.CLOSE_SHORT.is_short
    assert not SignalSide.OPEN_LONG.is_short

    # A scaling in commits more of the account, so it is an entry.
    assert SignalSide.OPEN_LONG.is_entry
    assert SignalSide.SCALE_IN.is_entry
    assert not SignalSide.CLOSE_LONG.is_entry

    assert SignalSide.CLOSE_LONG.is_exit
    assert SignalSide.SCALE_OUT.is_exit
    assert not SignalSide.OPEN_SHORT.is_exit


def test_each_side_declares_the_position_state_it_requires() -> None:
    """
    Test an open requires no position, a close requires its own side, and a scale requires any.
    """
    assert SignalSide.OPEN_LONG.requirement is PositionRequirement.FLAT
    assert SignalSide.OPEN_SHORT.requirement is PositionRequirement.FLAT
    assert SignalSide.CLOSE_LONG.requirement is PositionRequirement.LONG
    assert SignalSide.CLOSE_SHORT.requirement is PositionRequirement.SHORT
    assert SignalSide.SCALE_IN.requirement is PositionRequirement.ANY
    assert SignalSide.SCALE_OUT.requirement is PositionRequirement.ANY


def test_the_requirement_truth_table() -> None:
    """
    Test each requirement against each position state.
    """
    flat = PositionRequirement.FLAT
    assert flat.satisfied_by(PositionSide.NONE)
    assert not flat.satisfied_by(PositionSide.LONG)
    assert not flat.satisfied_by(PositionSide.SHORT)

    long = PositionRequirement.LONG
    assert long.satisfied_by(PositionSide.LONG)
    assert not long.satisfied_by(PositionSide.NONE)
    assert not long.satisfied_by(PositionSide.SHORT)

    short = PositionRequirement.SHORT
    assert short.satisfied_by(PositionSide.SHORT)
    assert not short.satisfied_by(PositionSide.NONE)
    assert not short.satisfied_by(PositionSide.LONG)

    any_position = PositionRequirement.ANY
    assert any_position.satisfied_by(PositionSide.LONG)
    assert any_position.satisfied_by(PositionSide.SHORT)
    assert not any_position.satisfied_by(PositionSide.NONE)


def test_an_open_maps_to_a_view_and_a_close_maps_to_flat() -> None:
    """
    Test the direction a side maps onto, flat being the engine's own statement of no exposure.
    """
    assert SignalSide.OPEN_LONG.to_direction() is SignalDirection.LONG
    assert SignalSide.OPEN_SHORT.to_direction() is SignalDirection.SHORT
    assert SignalSide.CLOSE_LONG.to_direction() is SignalDirection.FLAT
    assert SignalSide.CLOSE_SHORT.to_direction() is SignalDirection.FLAT


def test_a_scaling_side_refuses_to_map_onto_a_direction() -> None:
    """
    Test a tranche is refused rather than flattened into a target that would replace the position.
    """
    with pytest.raises(ValueError, match="has no signal direction"):
        SignalSide.SCALE_IN.to_direction()

    with pytest.raises(ValueError, match="has no signal direction"):
        SignalSide.SCALE_OUT.to_direction()


def test_a_strength_outside_the_unit_interval_is_refused() -> None:
    """
    Test a magnitude that is not a magnitude is refused rather than clamped.
    """
    with pytest.raises(ValueError, match="not finite"):
        _intent(SignalSide.OPEN_LONG, strength=math.nan)

    with pytest.raises(ValueError, match="negative"):
        _intent(SignalSide.OPEN_LONG, strength=-0.1)

    with pytest.raises(ValueError, match="exceeds one"):
        _intent(SignalSide.OPEN_LONG, strength=1.5)


def test_a_horizon_and_an_expiry_are_validated() -> None:
    """
    Test a horizon is a duration and an expiry is an instant.
    """
    with pytest.raises(ValueError, match="not positive"):
        _intent(SignalSide.OPEN_LONG, horizon_ns=0)

    with pytest.raises(ValueError, match="negative"):
        _intent(SignalSide.OPEN_LONG, expiry_ns=-1)

    intent = _intent(SignalSide.OPEN_LONG, horizon_ns=60_000_000_000, expiry_ns=1_000)
    assert intent.horizon_ns == 60_000_000_000
    assert intent.expiry_ns == 1_000


def test_strength_is_replaced_without_mutating_the_original() -> None:
    """
    Test an intention is immutable and a changed strength is a new one.
    """
    original = _intent(SignalSide.OPEN_LONG, strength=0.5)
    stronger = original.with_strength(0.9)

    assert stronger.strength == 0.9
    assert original.strength == 0.5
    assert stronger.side is original.side
    assert stronger.instrument_id == original.instrument_id


def test_expiry_is_read_against_a_declared_instant() -> None:
    """
    Test an intention with no expiry never expires and one with an expiry does at it.
    """
    assert not _intent(SignalSide.OPEN_LONG).is_expired(10**20)

    intent = _intent(SignalSide.OPEN_LONG, expiry_ns=1_000)
    assert not intent.is_expired(999)
    assert intent.is_expired(1_000)
    assert intent.is_expired(1_001)


def test_an_open_becomes_a_signal_with_its_own_fields() -> None:
    """
    Test the mapping carries the instrument, the strength, the source and the timestamps.
    """
    intent = _intent(SignalSide.OPEN_SHORT, strength=0.25, source="trend", horizon_ns=1_000)

    signal = intent.to_signal(ts_event=5, ts_init=6)

    assert signal.instrument_id == intent.instrument_id
    assert signal.direction is SignalDirection.SHORT
    assert signal.strength == 0.25
    assert signal.source == "trend"
    assert signal.horizon_ns == 1_000
    assert signal.ts_event == 5
    assert signal.ts_init == 6


def test_a_close_becomes_a_flat_signal() -> None:
    """
    Test a close maps onto no exposure rather than onto the direction it closes.
    """
    signal = _intent(SignalSide.CLOSE_LONG).to_signal(ts_event=1, ts_init=2)

    assert signal.direction is SignalDirection.FLAT


def test_a_scaling_side_cannot_become_a_signal() -> None:
    """
    Test the partial mapping is refused at the boundary rather than approximated.
    """
    with pytest.raises(ValueError, match="has no signal direction"):
        _intent(SignalSide.SCALE_IN).to_signal(ts_event=1, ts_init=2)


def test_the_rendering_names_the_side_and_the_source() -> None:
    """
    Test an intention is readable in a log line.
    """
    rendered = str(_intent(SignalSide.SCALE_OUT, strength=0.5, source="mean-reversion"))

    assert "SCALE_OUT".lower() in rendered.lower()
    assert "mean-reversion" in rendered
    assert "BTCUSDT.BINANCE" in rendered
