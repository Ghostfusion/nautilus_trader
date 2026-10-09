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
Tests for the conflict policy.

Each of the four gates is checked on its own: the disabled and expired gate, the position gate, the
direction mutex in each of its three modes, and the single-slot rule. The refusals are checked as well
as the admissions, because a policy that admits the right intention for the wrong reason is not a
policy anyone can audit.
"""

import json
from typing import TYPE_CHECKING
from typing import Any
from typing import cast

import pytest

from nautilus_trader.model import InstrumentId
from nautilus_trader.signal_policy import DEFAULT_PRECEDENCE
from nautilus_trader.signal_policy import ConflictMode
from nautilus_trader.signal_policy import ConflictPolicy
from nautilus_trader.signal_policy import ConflictReason
from nautilus_trader.signal_policy import PositionSide
from nautilus_trader.signal_policy import SignalIntent
from nautilus_trader.signal_policy import SignalSide


if TYPE_CHECKING:
    from collections.abc import Iterable


BTC = InstrumentId.from_str("BTCUSDT.BINANCE")
ETH = InstrumentId.from_str("ETHUSDT.BINANCE")


def _intent(
    side: SignalSide,
    *,
    instrument: InstrumentId = BTC,
    **kwargs: Any,
) -> SignalIntent:
    """
    Return an intention on one test instrument.
    """
    return SignalIntent(instrument_id=instrument, side=side, **kwargs)


def _reasons(resolution) -> list[ConflictReason]:
    """
    Return the refusal reasons of a resolution, in order.
    """
    return [refusal.reason for refusal in resolution.refusals]


def test_the_default_precedence_orders_the_sides() -> None:
    """
    Test exits come first, then entries, then the scaling sides.
    """
    assert DEFAULT_PRECEDENCE == (
        SignalSide.CLOSE_LONG,
        SignalSide.CLOSE_SHORT,
        SignalSide.SCALE_OUT,
        SignalSide.OPEN_LONG,
        SignalSide.OPEN_SHORT,
        SignalSide.SCALE_IN,
    )

    policy = ConflictPolicy()

    assert policy.precedence == DEFAULT_PRECEDENCE
    assert policy.priority_rank(SignalSide.CLOSE_LONG) == 0
    assert policy.priority_rank(SignalSide.SCALE_IN) == len(DEFAULT_PRECEDENCE) - 1
    assert policy.mode is ConflictMode.REFUSE


def test_a_side_absent_from_the_precedence_ranks_last() -> None:
    """
    Test a declared order that omits a side ranks it below every listed side.
    """
    policy = ConflictPolicy(precedence=(SignalSide.OPEN_LONG,))

    assert policy.priority_rank(SignalSide.OPEN_LONG) == 0
    assert policy.priority_rank(SignalSide.CLOSE_LONG) == 1


def test_a_precedence_that_repeats_a_side_is_refused() -> None:
    """
    Test an order with no unique rank for a side is refused.
    """
    with pytest.raises(ValueError, match="repeats a side"):
        ConflictPolicy(precedence=(SignalSide.OPEN_LONG, SignalSide.OPEN_LONG))


def test_a_configuration_that_is_not_a_side_is_refused() -> None:
    """
    Test the declared configuration is validated rather than accepted as text.
    """
    # The cast bypasses the type checker on purpose: the point of the test is that the runtime
    # validation refuses a name that is not a side, which is what a configuration file supplies.
    names = cast("Iterable[SignalSide]", ("open_long",))

    with pytest.raises(TypeError, match="not a SignalSide"):
        ConflictPolicy(precedence=names)

    with pytest.raises(TypeError, match="not a SignalSide"):
        ConflictPolicy(disabled_sides=names)


def test_the_mode_may_be_named() -> None:
    """
    Test configuration that arrives as text is resolved, and an unknown name is refused.
    """
    assert ConflictPolicy(mode="strength").mode is ConflictMode.STRENGTH

    with pytest.raises(ValueError, match="is not a conflict mode"):
        ConflictPolicy(mode="strongest")


def test_a_close_of_a_position_that_is_not_open_is_refused() -> None:
    """
    Test the position gate refuses a close the instrument is not in a position for.
    """
    resolution = ConflictPolicy().resolve([_intent(SignalSide.CLOSE_LONG)])

    assert resolution.admitted == ()
    assert _reasons(resolution) == [ConflictReason.POSITION_STATE]
    assert "requires a long position" in resolution.refusals[0].detail
    assert "the instrument is none" in resolution.refusals[0].detail


def test_an_open_while_a_position_is_open_is_refused() -> None:
    """
    Test the position gate refuses an open the instrument is already positioned for.
    """
    resolution = ConflictPolicy().resolve(
        [_intent(SignalSide.OPEN_LONG)],
        {BTC: PositionSide.LONG},
    )

    assert resolution.admitted == ()
    assert _reasons(resolution) == [ConflictReason.POSITION_STATE]
    assert resolution.resolutions[0].position is PositionSide.LONG


def test_a_scale_requires_some_position_to_scale() -> None:
    """
    Test a scaling side inherits the direction of the position it scales, so it is refused flat.
    """
    policy = ConflictPolicy()

    assert policy.resolve([_intent(SignalSide.SCALE_IN)]).admitted == ()

    refused = policy.resolve([_intent(SignalSide.SCALE_IN)])
    assert refused.refusals[0].reason is ConflictReason.POSITION_STATE

    scaled = policy.resolve([_intent(SignalSide.SCALE_OUT)], {BTC: PositionSide.SHORT})

    assert scaled.admitted[0].side is SignalSide.SCALE_OUT


def test_only_one_intention_is_admitted_per_instrument() -> None:
    """
    Test the single-slot rule admits the better-ranked intention and names it in the refusal.
    """
    resolution = ConflictPolicy().resolve(
        [_intent(SignalSide.SCALE_IN), _intent(SignalSide.CLOSE_LONG)],
        {BTC: PositionSide.LONG},
    )

    assert [intent.side for intent in resolution.admitted] == [SignalSide.CLOSE_LONG]
    assert _reasons(resolution) == [ConflictReason.LOWER_PRECEDENCE]
    assert "close_long was admitted" in resolution.refusals[0].detail
    assert "rank 0" in resolution.refusals[0].detail


def test_both_intentions_are_admitted_when_the_single_slot_rule_is_off() -> None:
    """
    Test a caller who owns the target path can admit more than one intention.
    """
    policy = ConflictPolicy(one_per_instrument=False)

    resolution = policy.resolve(
        [_intent(SignalSide.CLOSE_LONG), _intent(SignalSide.SCALE_IN)],
        {BTC: PositionSide.LONG},
    )

    assert [intent.side for intent in resolution.admitted] == [
        SignalSide.CLOSE_LONG,
        SignalSide.SCALE_IN,
    ]
    assert resolution.refusals == ()


def test_a_contradiction_is_refused_by_default() -> None:
    """
    Test a long and a short intention on one instrument are both refused rather than guessed at.
    """
    resolution = ConflictPolicy().resolve(
        [_intent(SignalSide.OPEN_LONG), _intent(SignalSide.OPEN_SHORT)]
    )

    assert resolution.admitted == ()
    assert _reasons(resolution) == [ConflictReason.DIRECTION_CONFLICT] * 2
    assert "refuses to choose" in resolution.refusals[0].detail


def test_priority_mode_keeps_the_better_ranked_direction() -> None:
    """
    Test the mutex can resolve a contradiction by the declared order instead of refusing it.
    """
    policy = ConflictPolicy(mode=ConflictMode.PRIORITY)

    resolution = policy.resolve([_intent(SignalSide.OPEN_SHORT), _intent(SignalSide.OPEN_LONG)])

    assert [intent.side for intent in resolution.admitted] == [SignalSide.OPEN_LONG]
    assert _reasons(resolution) == [ConflictReason.DIRECTION_CONFLICT]
    assert "decided by precedence" in resolution.refusals[0].detail


def test_strength_mode_keeps_the_stronger_direction() -> None:
    """
    Test the mutex can resolve a contradiction by the magnitude of the two intentions.
    """
    policy = ConflictPolicy(mode=ConflictMode.STRENGTH)

    resolution = policy.resolve(
        [
            _intent(SignalSide.OPEN_LONG, strength=0.2),
            _intent(SignalSide.OPEN_SHORT, strength=0.9),
        ]
    )

    assert [intent.side for intent in resolution.admitted] == [SignalSide.OPEN_SHORT]
    assert "decided by strength" in resolution.refusals[0].detail


def test_strength_mode_falls_back_to_the_precedence_on_a_tie() -> None:
    """
    Test equal magnitudes are decided by the declared order rather than by chance.
    """
    policy = ConflictPolicy(mode=ConflictMode.STRENGTH)

    resolution = policy.resolve(
        [
            _intent(SignalSide.OPEN_LONG, strength=0.5),
            _intent(SignalSide.OPEN_SHORT, strength=0.5),
        ]
    )

    assert [intent.side for intent in resolution.admitted] == [SignalSide.OPEN_LONG]
    assert "equal strength, decided by precedence" in resolution.refusals[0].detail


def test_the_mutex_can_be_turned_off() -> None:
    """
    Test a caller who wants both directions admitted gets one slot decided by rank instead.
    """
    policy = ConflictPolicy(direction_mutex=False)

    resolution = policy.resolve([_intent(SignalSide.OPEN_SHORT), _intent(SignalSide.OPEN_LONG)])

    assert [intent.side for intent in resolution.admitted] == [SignalSide.OPEN_LONG]
    assert _reasons(resolution) == [ConflictReason.LOWER_PRECEDENCE]


def test_a_disabled_side_is_refused() -> None:
    """
    Test a long-only policy refuses the short sides it was told not to admit.
    """
    policy = ConflictPolicy(disabled_sides=(SignalSide.OPEN_SHORT,))

    resolution = policy.resolve([_intent(SignalSide.OPEN_SHORT)])

    assert resolution.admitted == ()
    assert _reasons(resolution) == [ConflictReason.DISABLED_SIDE]


def test_an_expired_intention_is_refused_only_against_a_declared_instant() -> None:
    """
    Test expiry is read against the instant the caller resolves at.
    """
    policy = ConflictPolicy()
    intent = _intent(SignalSide.OPEN_LONG, expiry_ns=1_000)

    # With no instant declared the expiry is not read at all, so the intention is admitted.
    undeclared = policy.resolve([intent])

    assert undeclared.admitted == (intent,)
    assert undeclared.refusals == ()

    expired = policy.resolve([intent], at_ns=1_000)

    assert expired.admitted == ()
    assert _reasons(expired) == [ConflictReason.EXPIRED]

    assert policy.resolve([intent], at_ns=999).admitted[0].side is SignalSide.OPEN_LONG


def test_instruments_are_resolved_independently() -> None:
    """
    Test each instrument is decided against its own position state, in identifier order.
    """
    resolution = ConflictPolicy().resolve(
        [
            _intent(SignalSide.CLOSE_SHORT, instrument=ETH),
            _intent(SignalSide.CLOSE_LONG, instrument=BTC),
        ],
        {BTC: PositionSide.LONG, ETH: PositionSide.SHORT},
    )

    assert [str(item.instrument_id) for item in resolution.resolutions] == [
        "BTCUSDT.BINANCE",
        "ETHUSDT.BINANCE",
    ]
    assert [intent.side for intent in resolution.admitted] == [
        SignalSide.CLOSE_LONG,
        SignalSide.CLOSE_SHORT,
    ]


def test_the_trace_records_the_mode_the_admissions_and_every_refusal() -> None:
    """
    Test a decision can be read back without the classes.
    """
    resolution = ConflictPolicy(mode=ConflictMode.PRIORITY).resolve(
        [_intent(SignalSide.OPEN_LONG), _intent(SignalSide.OPEN_SHORT)]
    )

    trace = resolution.to_trace()

    assert trace["trace_version"] == 1
    assert trace["mode"] == "priority"
    instruments = trace["instruments"]
    assert isinstance(instruments, list)
    assert len(instruments) == 1
    assert instruments[0]["instrument_id"] == "BTCUSDT.BINANCE"
    assert instruments[0]["admitted"] == ["open_long"]
    refused = instruments[0]["refused"]
    assert len(refused) == 1
    assert refused[0]["side"] == "open_short"
    assert refused[0]["reason"] == "direction_conflict"
    assert refused[0]["detail"]
    assert json.loads(json.dumps(trace)) == trace
