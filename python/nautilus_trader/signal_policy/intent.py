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
Signal intents: what a strategy means to do, before it becomes an order.

A `TradingSignal` states a view with three directions - long, short and flat - and the engine
turns it into a target. Three useful intentions have no representation there: adding to a position
in tranches, trimming one in tranches, and saying that a close is a close rather than a statement
that the instrument will not move. `SignalSide` names all six transitions explicitly, and
`SignalIntent` pairs one with an instrument, a strength and an expiry so that a set of intentions
can be arbitrated before anything is submitted.

One boundary is enforced rather than documented: the mapping back onto a `TradingSignal` is partial,
and the partiality is refused rather than approximated. An open maps to a long or short view, and a
close maps to a flat view, which is exactly "no exposure" and is the engine's own statement of a
close. The two scaling sides raise instead, because a scale has no target of its own: the target
path sizes a position from risk, so a tranche would be a second position rather than an addition
to the first, and a partial trim has no target representation at all. `SignalSide.to_direction`
names each mapping and refuses the rest, so a caller who wants tranches keeps the tranche
arithmetic in their own strategy rather than having it silently flattened into a target.
"""

from __future__ import annotations

import math
from dataclasses import dataclass
from dataclasses import replace
from enum import Enum

from nautilus_trader.model import InstrumentId
from nautilus_trader.model import SignalDirection
from nautilus_trader.model import TradingSignal


class SignalSide(Enum):
    """
    A named position transition.

    The six values are the transitions a strategy makes: opening and closing each direction, and
    scaling in and out of whatever position is open.
    """

    OPEN_LONG = "open_long"
    CLOSE_LONG = "close_long"
    SCALE_IN = "scale_in"
    SCALE_OUT = "scale_out"
    OPEN_SHORT = "open_short"
    CLOSE_SHORT = "close_short"

    @property
    def is_open(self) -> bool:
        """
        Return whether the side opens a position from none.
        """
        return self in (SignalSide.OPEN_LONG, SignalSide.OPEN_SHORT)

    @property
    def is_close(self) -> bool:
        """
        Return whether the side closes the whole position.
        """
        return self in (SignalSide.CLOSE_LONG, SignalSide.CLOSE_SHORT)

    @property
    def is_scale(self) -> bool:
        """
        Return whether the side changes the size of an open position in tranches.
        """
        return self in (SignalSide.SCALE_IN, SignalSide.SCALE_OUT)

    @property
    def is_long(self) -> bool:
        """
        Return whether the side is the long direction of the instrument.
        """
        return self in (SignalSide.OPEN_LONG, SignalSide.CLOSE_LONG)

    @property
    def is_short(self) -> bool:
        """
        Return whether the side is the short direction of the instrument.
        """
        return self in (SignalSide.OPEN_SHORT, SignalSide.CLOSE_SHORT)

    @property
    def is_entry(self) -> bool:
        """
        Return whether the side increases exposure.

        A scaling in is an entry in this sense - it commits more of the account - which is why the
        entries are the sides an exposure or capacity check applies to.
        """
        return self in (SignalSide.OPEN_LONG, SignalSide.OPEN_SHORT, SignalSide.SCALE_IN)

    @property
    def is_exit(self) -> bool:
        """
        Return whether the side reduces exposure.
        """
        return self in (SignalSide.CLOSE_LONG, SignalSide.CLOSE_SHORT, SignalSide.SCALE_OUT)

    @property
    def requirement(self) -> PositionRequirement:
        """
        Return the position state the side requires before it is meaningful.
        """
        if self in (SignalSide.OPEN_LONG, SignalSide.OPEN_SHORT):
            return PositionRequirement.FLAT

        if self is SignalSide.CLOSE_LONG:
            return PositionRequirement.LONG

        if self is SignalSide.CLOSE_SHORT:
            return PositionRequirement.SHORT

        return PositionRequirement.ANY

    def to_direction(self) -> SignalDirection:
        """
        Return the `SignalDirection` this side maps onto.

        Returns
        -------
        SignalDirection
            `LONG` for an open long, `SHORT` for an open short, and `FLAT` for either close, which
            is the engine's own statement of no exposure.

        Raises
        ------
        ValueError
            If the side is a scaling side. A tranche has no target of its own: the target path sizes
            a position from risk, so flattening a scale into a target would replace the position
            rather than add to or trim it.

        """
        if self is SignalSide.OPEN_LONG:
            return SignalDirection.LONG

        if self is SignalSide.OPEN_SHORT:
            return SignalDirection.SHORT

        if self.is_close:
            return SignalDirection.FLAT

        raise ValueError(
            f"the side {self.value!r} is a scaling side and has no signal direction; "
            "the target path cannot express a tranche"
        )


class PositionSide(Enum):
    """
    The position state of one instrument, as the policy layer is told it.
    """

    NONE = "none"
    LONG = "long"
    SHORT = "short"


class PositionRequirement(Enum):
    """
    The position state a side requires before it is meaningful.
    """

    FLAT = "flat"
    LONG = "long"
    SHORT = "short"
    ANY = "any"

    def satisfied_by(self, side: PositionSide) -> bool:
        """
        Return whether the given position state satisfies this requirement.

        Parameters
        ----------
        side : PositionSide
            The position state of the instrument.

        Returns
        -------
        bool
            True when the requirement is met.

        """
        if self is PositionRequirement.FLAT:
            return side is PositionSide.NONE

        if self is PositionRequirement.LONG:
            return side is PositionSide.LONG

        if self is PositionRequirement.SHORT:
            return side is PositionSide.SHORT

        return side is not PositionSide.NONE


@dataclass(frozen=True)
class SignalIntent:
    """
    One intended position transition, before it is submitted.

    Parameters
    ----------
    instrument_id : InstrumentId
        The instrument the intention is about.
    side : SignalSide
        The transition intended.
    strength : float, default 1.0
        The magnitude of the intention, from 0.0 to 1.0. A strength above one is refused rather than
        clamped, because clamping it silently would make two different intentions indistinguishable.
    source : str, optional
        The component or model that produced the intention.
    horizon_ns : int, optional
        How far ahead the intention is a view about, in nanoseconds.
    expiry_ns : int, optional
        The instant the intention stops being valid, in nanoseconds since the epoch.

    Raises
    ------
    ValueError
        If `strength` is not finite, is negative or exceeds one, if `horizon_ns` is not positive, or
        if `expiry_ns` is negative.

    """

    instrument_id: InstrumentId
    side: SignalSide
    strength: float = 1.0
    source: str | None = None
    horizon_ns: int | None = None
    expiry_ns: int | None = None

    def __post_init__(self) -> None:
        """
        Refuse a strength that is not a magnitude and a horizon or expiry that cannot be read.
        """
        if not math.isfinite(self.strength):
            raise ValueError(f"strength {self.strength!r} is not finite; strength is a magnitude")

        if self.strength < 0.0:
            raise ValueError(f"strength {self.strength!r} is negative; strength is a magnitude")

        if self.strength > 1.0:
            raise ValueError(
                f"strength {self.strength!r} exceeds one; an intention is a magnitude, not a size"
            )

        if self.horizon_ns is not None and self.horizon_ns <= 0:
            raise ValueError(
                f"horizon_ns {self.horizon_ns!r} is not positive; a horizon is a duration"
            )

        if self.expiry_ns is not None and self.expiry_ns < 0:
            raise ValueError(f"expiry_ns {self.expiry_ns!r} is negative; it is an instant")

    def __str__(self) -> str:
        """
        Return a readable rendering of the intention.
        """
        return (
            f"SignalIntent({self.instrument_id}, {self.side.value}, "
            f"strength={self.strength:g}, source={self.source or 'none'})"
        )

    def with_strength(self, strength: float) -> SignalIntent:
        """
        Return a copy of this intention with a different strength.

        Parameters
        ----------
        strength : float
            The new strength.

        Returns
        -------
        SignalIntent
            The copy.

        """
        return replace(self, strength=strength)

    def is_expired(self, at_ns: int) -> bool:
        """
        Return whether the intention is no longer valid at the given instant.

        Parameters
        ----------
        at_ns : int
            The instant to test, in nanoseconds since the epoch.

        Returns
        -------
        bool
            True when an expiry is declared and the instant is at or after it.

        """
        return self.expiry_ns is not None and at_ns >= self.expiry_ns

    def required_position(self) -> PositionRequirement:
        """
        Return the position state the intention requires.
        """
        return self.side.requirement

    def to_signal(
        self,
        *,
        ts_event: int,
        ts_init: int,
        provenance: dict | None = None,
    ) -> TradingSignal:
        """
        Return the `TradingSignal` this intention maps onto.

        Parameters
        ----------
        ts_event : int
            The event timestamp of the signal, in nanoseconds since the epoch.
        ts_init : int
            The initialization timestamp of the signal, in nanoseconds since the epoch.
        provenance : dict, optional
            Free-form origin detail carried with the signal.

        Returns
        -------
        TradingSignal
            A long or short view for an open, and a flat view for a close.

        Raises
        ------
        ValueError
            If the side is a scaling side, which has no signal direction.

        """
        return TradingSignal(
            self.instrument_id,
            self.side.to_direction(),
            self.horizon_ns,
            self.strength,
            self.source,
            self.expiry_ns,
            provenance,
            ts_event,
            ts_init,
        )
