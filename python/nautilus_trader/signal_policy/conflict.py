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
Arbitrating competing signal intentions on one instrument.

A strategy component rarely speaks alone: a trend rule wants to open, a mean-reversion rule wants to
close, and both fire on the same bar for the same instrument. Without a declared policy the winner
is whatever the submission code happened to evaluate first, which is an accident of implementation
rather than a decision. `ConflictPolicy` decides in one place, in a declared order, and records
every intention it refused and why.

The policy is pure: it reads no clock, touches no cache and submits nothing. The position state it
needs is passed in, so the same intention set always resolves to the same answer, and the answer can
be tested without an engine. Four decisions are declared rather than implied:

- **Precedence.** Sides are ordered, and the default order exits before entries before scaling -
  `CLOSE_LONG, CLOSE_SHORT, SCALE_OUT, OPEN_LONG, OPEN_SHORT, SCALE_IN` - so a policy that is given
  no configuration still prefers to reduce risk rather than add to it.
- **The direction mutex.** A long intention and a short intention on one instrument are a
  contradiction. It is refused by default rather than resolved silently, because a strategy that
  wants both at once is usually a defect; `PRIORITY` and `STRENGTH` resolve it anyway, and state
  which rule they used.
- **The position gate.** An intention is meaningless in the wrong state: an open requires no
  position, a close requires the side it closes, and a scale requires some position to scale. The
  gate refuses the rest, so a close of a position that is not open cannot reach an order.
- **One slot per instrument.** The engine's target path builds one target per instrument per
  submission, so the policy admits one intention per instrument by default and refuses the losers
  with the rank that beat them, rather than letting two intentions race.

What the policy does not do is size anything. A tranche percentage, a cooldown measured in bars
and a capacity or exposure cap all need state this module does not own; they belong where that
state lives. The refusals are the record of what the policy decided, and `to_trace` renders them
as a flat, JSON-serialisable structure so a decision can be audited after the fact.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from typing import TYPE_CHECKING

from nautilus_trader.signal_policy.intent import PositionSide
from nautilus_trader.signal_policy.intent import SignalIntent
from nautilus_trader.signal_policy.intent import SignalSide


if TYPE_CHECKING:
    from collections.abc import Iterable
    from collections.abc import Mapping
    from collections.abc import Sequence

    from nautilus_trader.model import InstrumentId


# The declared default order. Exits come first, then entries, then the scaling sides, so a policy
# given no configuration reduces exposure before it adds any.
DEFAULT_PRECEDENCE: tuple[SignalSide, ...] = (
    SignalSide.CLOSE_LONG,
    SignalSide.CLOSE_SHORT,
    SignalSide.SCALE_OUT,
    SignalSide.OPEN_LONG,
    SignalSide.OPEN_SHORT,
    SignalSide.SCALE_IN,
)

CONFLICT_TRACE_VERSION = 1


class ConflictMode(Enum):
    """
    How a contradiction between a long and a short intention on one instrument is handled.
    """

    REFUSE = "refuse"
    PRIORITY = "priority"
    STRENGTH = "strength"


class ConflictReason(Enum):
    """
    Why an intention was refused.
    """

    DISABLED_SIDE = "disabled_side"
    EXPIRED = "expired"
    POSITION_STATE = "position_state"
    DIRECTION_CONFLICT = "direction_conflict"
    LOWER_PRECEDENCE = "lower_precedence"


@dataclass(frozen=True)
class ConflictRefusal:
    """
    One refused intention and the reason it was refused.

    Parameters
    ----------
    intent : SignalIntent
        The intention that was not admitted.
    reason : ConflictReason
        Why it was not admitted.
    detail : str
        A readable statement of the case, recorded so the refusal can be checked without re-running
        the policy.

    """

    intent: SignalIntent
    reason: ConflictReason
    detail: str


@dataclass(frozen=True)
class InstrumentResolution:
    """
    What the policy decided for one instrument.

    Parameters
    ----------
    instrument_id : InstrumentId
        The instrument the decision is about.
    admitted : tuple of SignalIntent
        The intentions that survived, in the caller's order.
    refusals : tuple of ConflictRefusal
        The intentions that did not, with their reasons.
    position : PositionSide
        The position state the instrument was resolved against.

    """

    instrument_id: InstrumentId
    admitted: tuple[SignalIntent, ...]
    refusals: tuple[ConflictRefusal, ...]
    position: PositionSide

    @property
    def admitted_sides(self) -> tuple[SignalSide, ...]:
        """
        Return the sides that were admitted.
        """
        return tuple(intent.side for intent in self.admitted)


@dataclass(frozen=True)
class ConflictResolution:
    """
    What the policy decided for a whole submission.

    Parameters
    ----------
    resolutions : tuple of InstrumentResolution
        One decision per instrument, ordered by instrument identifier.
    mode : ConflictMode
        The mode the policy was configured with, recorded with the decision.

    """

    resolutions: tuple[InstrumentResolution, ...]
    mode: ConflictMode

    @property
    def admitted(self) -> tuple[SignalIntent, ...]:
        """
        Return every admitted intention across instruments.
        """
        return tuple(intent for item in self.resolutions for intent in item.admitted)

    @property
    def refusals(self) -> tuple[ConflictRefusal, ...]:
        """
        Return every refusal across instruments.
        """
        return tuple(refusal for item in self.resolutions for refusal in item.refusals)

    def to_trace(self) -> dict[str, object]:
        """
        Return a flat, JSON-serialisable record of the decision.

        The record is written so that a decision can be read back without the classes: the mode, and
        for each instrument the sides admitted and, for each refusal, the side, the reason and the
        case that produced it.
        """
        return {
            "trace_version": CONFLICT_TRACE_VERSION,
            "mode": self.mode.value,
            "instruments": [
                {
                    "instrument_id": str(item.instrument_id),
                    "position": item.position.value,
                    "admitted": [side.value for side in item.admitted_sides],
                    "refused": [
                        {
                            "side": refusal.intent.side.value,
                            "reason": refusal.reason.value,
                            "detail": refusal.detail,
                        }
                        for refusal in item.refusals
                    ],
                }
                for item in self.resolutions
            ],
        }


def _conflict_mode(value: str) -> ConflictMode:
    """
    Return the mode a name refers to, refusing a name that is not a mode.

    A policy is configuration, and configuration arrives as text, so the name is resolved here and
    an unknown name is refused rather than carried as an unset mode through the four gates.
    """
    try:
        return ConflictMode(value)
    except ValueError as exc:
        raise ValueError(
            f"{value!r} is not a conflict mode; the modes are "
            f"{[mode.value for mode in ConflictMode]}"
        ) from exc


class ConflictPolicy:
    """
    A declared order over competing signal intentions.

    Parameters
    ----------
    precedence : Iterable[SignalSide], optional
        The order sides are ranked in, best first. Defaults to `DEFAULT_PRECEDENCE`. A side missing
        from the order ranks below every listed side.
    mode : ConflictMode, default ConflictMode.REFUSE
        How a long and a short intention on one instrument is handled.
    disabled_sides : Iterable[SignalSide], optional
        Sides that are refused outright, for a strategy that is long only or short only.
    direction_mutex : bool, default True
        Whether a contradiction between the two directions is decided by the policy at all. When
        False, both directions are admitted and the single-slot rule decides between them.
    one_per_instrument : bool, default True
        Whether only one intention per instrument may be admitted. The engine builds one target per
        instrument per submission, so admitting two would submit a race.

    Raises
    ------
    ValueError
        If `precedence` repeats a side, or `mode` is not a `ConflictMode`.
    TypeError
        If `precedence` or `disabled_sides` contains something that is not a `SignalSide`.

    """

    def __init__(
        self,
        precedence: Iterable[SignalSide] | None = None,
        *,
        mode: ConflictMode | str = ConflictMode.REFUSE,
        disabled_sides: Iterable[SignalSide] = (),
        direction_mutex: bool = True,
        one_per_instrument: bool = True,
    ) -> None:
        """
        Create a policy with the declared order, mode and gates.
        """
        order = tuple(DEFAULT_PRECEDENCE if precedence is None else precedence)

        for side in order:
            if not isinstance(side, SignalSide):
                raise TypeError(f"precedence contains {side!r}, which is not a SignalSide")

        if len(set(order)) != len(order):
            raise ValueError(f"precedence repeats a side: {[side.value for side in order]}")

        if isinstance(mode, str):
            mode = _conflict_mode(mode)

        disabled = tuple(disabled_sides)

        for side in disabled:
            if not isinstance(side, SignalSide):
                raise TypeError(f"disabled_sides contains {side!r}, which is not a SignalSide")

        self._precedence = order
        self._mode = mode
        self._disabled = frozenset(disabled)
        self._direction_mutex = direction_mutex
        self._one_per_instrument = one_per_instrument

    @property
    def mode(self) -> ConflictMode:
        """
        Return the mode the policy was configured with.
        """
        return self._mode

    @property
    def precedence(self) -> tuple[SignalSide, ...]:
        """
        Return the declared order of the sides.
        """
        return self._precedence

    def priority_rank(self, side: SignalSide) -> int:
        """
        Return the rank of a side, where a smaller rank wins.

        Parameters
        ----------
        side : SignalSide
            The side to rank.

        Returns
        -------
        int
            Its position in the declared order, or one past the end when it is not listed.

        """
        try:
            return self._precedence.index(side)
        except ValueError:
            return len(self._precedence)

    def resolve(
        self,
        intents: Sequence[SignalIntent],
        positions: Mapping[InstrumentId, PositionSide] | None = None,
        *,
        at_ns: int | None = None,
    ) -> ConflictResolution:
        """
        Decide which intentions are admitted.

        Parameters
        ----------
        intents : Sequence[SignalIntent]
            The intentions submitted together, in the order the caller evaluated them.
        positions : Mapping[InstrumentId, PositionSide], optional
            The position state per instrument. An instrument that is absent is treated as flat.
        at_ns : int, optional
            The instant to test expiries against, in nanoseconds since the epoch. When omitted, no
            intention is treated as expired.

        Returns
        -------
        ConflictResolution
            One decision per instrument, with every refusal and its reason.

        Examples
        --------
        >>> policy = ConflictPolicy()
        >>> policy.priority_rank(SignalSide.CLOSE_LONG)
        0

        """
        state = positions or {}
        grouped: dict[str, list[SignalIntent]] = {}

        for intent in intents:
            grouped.setdefault(str(intent.instrument_id), []).append(intent)

        resolutions = []

        for key in sorted(grouped):
            group = grouped[key]
            position = state.get(group[0].instrument_id, PositionSide.NONE)
            resolutions.append(self._resolve_instrument(group, position, at_ns))

        return ConflictResolution(resolutions=tuple(resolutions), mode=self._mode)

    def _resolve_instrument(
        self,
        group: Sequence[SignalIntent],
        position: PositionSide,
        at_ns: int | None,
    ) -> InstrumentResolution:
        """
        Decide one instrument's intentions in the declared order of the four gates.
        """
        refusals: list[ConflictRefusal] = []
        live: list[SignalIntent] = []

        # Gate one: a disabled side and an expired intention cannot be admitted at all.
        for intent in group:
            if intent.side in self._disabled:
                refusals.append(
                    ConflictRefusal(
                        intent=intent,
                        reason=ConflictReason.DISABLED_SIDE,
                        detail=f"the side {intent.side.value} is disabled by this policy",
                    )
                )
                continue

            if at_ns is not None and intent.is_expired(at_ns):
                refusals.append(
                    ConflictRefusal(
                        intent=intent,
                        reason=ConflictReason.EXPIRED,
                        detail=f"expired at {intent.expiry_ns}, resolved at {at_ns}",
                    )
                )
                continue

            live.append(intent)

        # Gate two: the position state the side is meaningful in.
        satisfied: list[SignalIntent] = []

        for intent in live:
            requirement = intent.required_position()

            if requirement.satisfied_by(position):
                satisfied.append(intent)
                continue

            refusals.append(
                ConflictRefusal(
                    intent=intent,
                    reason=ConflictReason.POSITION_STATE,
                    detail=(
                        f"the side {intent.side.value} requires a {requirement.value} position, "
                        f"the instrument is {position.value}"
                    ),
                )
            )

        # Gate three: the direction mutex, which the two directional sides can contradict. The
        # scaling sides carry no direction of their own, so they never trigger it.
        survivors = self._apply_direction_mutex(satisfied, refusals)

        # Gate four: one slot per instrument, because the target path builds one target per
        # instrument per submission.
        admitted = self._apply_single_slot(survivors, refusals)

        return InstrumentResolution(
            instrument_id=group[0].instrument_id,
            admitted=tuple(admitted),
            refusals=tuple(refusals),
            position=position,
        )

    def _apply_direction_mutex(
        self,
        intents: Sequence[SignalIntent],
        refusals: list[ConflictRefusal],
    ) -> list[SignalIntent]:
        """
        Resolve a long and a short intention on one instrument, or return the intentions unchanged.
        """
        if not self._direction_mutex:
            return list(intents)

        longs = [intent for intent in intents if intent.side.is_long]
        shorts = [intent for intent in intents if intent.side.is_short]

        if not longs or not shorts:
            return list(intents)

        if self._mode is ConflictMode.REFUSE:
            refusals.extend(
                ConflictRefusal(
                    intent=intent,
                    reason=ConflictReason.DIRECTION_CONFLICT,
                    detail=(
                        "a long and a short intention were submitted for one instrument and "
                        "the policy refuses to choose between them"
                    ),
                )
                for intent in (*longs, *shorts)
            )

            return [
                intent for intent in intents if not (intent.side.is_long or intent.side.is_short)
            ]

        if self._mode is ConflictMode.STRENGTH:
            strongest_long = max(intent.strength for intent in longs)
            strongest_short = max(intent.strength for intent in shorts)

            if strongest_long == strongest_short:
                winning_long = self.priority_rank(longs[0].side) <= self.priority_rank(
                    shorts[0].side
                )
                rule = "equal strength, decided by precedence"
            else:
                winning_long = strongest_long > strongest_short
                rule = "decided by strength"
        else:
            winning_long = self.priority_rank(longs[0].side) <= self.priority_rank(shorts[0].side)
            rule = "decided by precedence"

        losers = shorts if winning_long else longs
        winners = longs if winning_long else shorts
        winner_sides = ", ".join(sorted({intent.side.value for intent in winners}))

        refusals.extend(
            ConflictRefusal(
                intent=intent,
                reason=ConflictReason.DIRECTION_CONFLICT,
                detail=f"the other direction won this instrument ({winner_sides}), {rule}",
            )
            for intent in losers
        )

        losing = {id(intent) for intent in losers}
        return [intent for intent in intents if id(intent) not in losing]

    def _apply_single_slot(
        self,
        intents: Sequence[SignalIntent],
        refusals: list[ConflictRefusal],
    ) -> list[SignalIntent]:
        """
        Admit at most one intention, the best by rank and then by strength.
        """
        if not self._one_per_instrument or len(intents) <= 1:
            return list(intents)

        ranked = sorted(
            intents,
            key=lambda intent: (self.priority_rank(intent.side), -intent.strength),
        )
        winner = ranked[0]

        refusals.extend(
            ConflictRefusal(
                intent=intent,
                reason=ConflictReason.LOWER_PRECEDENCE,
                detail=(
                    f"the side {winner.side.value} was admitted for this instrument first "
                    f"(rank {self.priority_rank(winner.side)}, strength {winner.strength:g})"
                ),
            )
            for intent in ranked[1:]
        )

        return [winner]
