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
The rating policy: a research view is not a direction.

`BUY`, `HOLD` and `SELL` are not a one-to-one encoding of long, flat and short, because the
interpretation of a view depends on the position that already exists: `HOLD` with a position is not
the
same instruction as `HOLD` flat, and `SELL` flat is not the same as `SELL` long. The policy is
therefore total over the cross product of the producer's rating vocabulary and the position states,
and it states its output for every cell rather than leaving an implementer to invent one.

Three consequences are deliberate. `HOLD` produces no signal at all rather than a directional signal
carrying a "maintain" flag, because this repository's `TradingSignal` has no such flag and inventing
one would change a platform type for a bridge policy. A `SELL` on a long position resolves to a zero
target rather than a partial reduction, because a partial exit needs a specification the artifact
does
not carry, and letting `SELL` imply an arbitrary percentage would be the bridge inventing a position
size. And revision 1 is long-only, so a `SELL` on a flat instrument produces no signal and the
branch
that would permit a short is closed rather than left to a configuration flag.

The producer's rating vocabulary is five-valued and title-cased, while the design's table is
upper-cased and states three rows. The two weaker siblings are resolved to their directional family:
`Overweight` is a positive view and takes the same row as `BUY`, and `Underweight` is a negative
view
and takes the same row as `SELL`. That is the only total mapping which neither refuses two of the
producer's five legitimate ratings nor invents a direction for them, and the difference between
`BUY`
and `Overweight` is not expressible in a long-only ceiling: the artifact carries no magnitude that
distinguishes them, and inventing one would be the bridge choosing a position size. An unrecognised
rating refuses with `RATING_UNKNOWN` rather than resolving to anything.

The bridge never invents a direction. A positive view is never resolved to flat or short, a negative
view is never resolved to long, and the only way a view changes direction is by the rating itself.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from enum import unique

from nautilus_trader.decision_bridge.contract import Refusal
from nautilus_trader.decision_bridge.contract import RefusalCode


@unique
class ResearchView(Enum):
    """
    The producer's rating vocabulary, as this bridge understands it.
    """

    BUY = "BUY"
    OVERWEIGHT = "OVERWEIGHT"
    HOLD = "HOLD"
    UNDERWEIGHT = "UNDERWEIGHT"
    SELL = "SELL"


@unique
class PositionState(Enum):
    """
    The instrument's position, as the bridge reads it from the cache and portfolio.

    `SHORT` exists because the engine can report a negative net position -- a book may hold one from
    another strategy -- and the bridge must answer for it rather than fail to have a state for it.
    It
    is not a state revision 1 trades in: every row for it resolves to no signal.
    """

    LONG = "LONG"
    FLAT = "FLAT"
    SHORT = "SHORT"


@unique
class RatingResolution(Enum):
    """
    What a research view resolves to, given the position.
    """

    LONG_SIGNAL = "LONG_SIGNAL"
    FLAT_TARGET = "FLAT_TARGET"
    NO_SIGNAL = "NO_SIGNAL"


# The declared, closed mapping from the producer's rating strings onto the view vocabulary. Both
# casings are accepted because the wire contract declares the field as a plain string while the
# producer's own vocabulary is title-cased; the mapping is closed, so a new producer rating is a
# visible change here rather than a silent one at a call site.
VIEW_MAPPING: dict[str, ResearchView] = {
    "BUY": ResearchView.BUY,
    "OVERWEIGHT": ResearchView.OVERWEIGHT,
    "HOLD": ResearchView.HOLD,
    "UNDERWEIGHT": ResearchView.UNDERWEIGHT,
    "SELL": ResearchView.SELL,
}

# The policy table, stated for every cell of the cross product so that totality is a property a test
# can check rather than a claim in prose.
POLICY: dict[tuple[ResearchView, PositionState], RatingResolution] = {
    (ResearchView.BUY, PositionState.FLAT): RatingResolution.LONG_SIGNAL,
    (ResearchView.BUY, PositionState.LONG): RatingResolution.LONG_SIGNAL,
    (ResearchView.OVERWEIGHT, PositionState.FLAT): RatingResolution.LONG_SIGNAL,
    (ResearchView.OVERWEIGHT, PositionState.LONG): RatingResolution.LONG_SIGNAL,
    (ResearchView.HOLD, PositionState.FLAT): RatingResolution.NO_SIGNAL,
    (ResearchView.HOLD, PositionState.LONG): RatingResolution.NO_SIGNAL,
    (ResearchView.UNDERWEIGHT, PositionState.FLAT): RatingResolution.NO_SIGNAL,
    (ResearchView.UNDERWEIGHT, PositionState.LONG): RatingResolution.FLAT_TARGET,
    (ResearchView.SELL, PositionState.FLAT): RatingResolution.NO_SIGNAL,
    (ResearchView.SELL, PositionState.LONG): RatingResolution.FLAT_TARGET,
    (ResearchView.BUY, PositionState.SHORT): RatingResolution.NO_SIGNAL,
    (ResearchView.OVERWEIGHT, PositionState.SHORT): RatingResolution.NO_SIGNAL,
    (ResearchView.HOLD, PositionState.SHORT): RatingResolution.NO_SIGNAL,
    (ResearchView.UNDERWEIGHT, PositionState.SHORT): RatingResolution.NO_SIGNAL,
    (ResearchView.SELL, PositionState.SHORT): RatingResolution.NO_SIGNAL,
}

# Why each row resolves the way it does. A declined view is recorded with its reason so that a
# policy decision is distinguishable from an artifact that never arrived.
RATIONALE: dict[tuple[ResearchView, PositionState], str] = {
    (ResearchView.BUY, PositionState.FLAT): (
        "a positive view with no exposure resolves to a long signal"
    ),
    (ResearchView.BUY, PositionState.LONG): (
        "a positive view keeps the ceiling; the reconciler emits the delta only"
    ),
    (ResearchView.OVERWEIGHT, PositionState.FLAT): (
        "a positive view with no exposure resolves to a long signal"
    ),
    (ResearchView.OVERWEIGHT, PositionState.LONG): (
        "a positive view keeps the ceiling; the reconciler emits the delta only"
    ),
    (ResearchView.HOLD, PositionState.FLAT): (
        "the artifact states no view and there is nothing to maintain"
    ),
    (ResearchView.HOLD, PositionState.LONG): (
        "the engine's own reconciliation already maintains an exposure"
    ),
    (ResearchView.UNDERWEIGHT, PositionState.FLAT): (
        "revision 1 is long-only, so a negative view with no exposure does nothing"
    ),
    (ResearchView.UNDERWEIGHT, PositionState.LONG): (
        "a negative view on a long position resolves to a zero target"
    ),
    (ResearchView.SELL, PositionState.FLAT): (
        "revision 1 is long-only, so a negative view with no exposure does nothing"
    ),
    (ResearchView.SELL, PositionState.LONG): (
        "a negative view on a long position resolves to a zero target"
    ),
    (ResearchView.BUY, PositionState.SHORT): (
        "revision 1 is long-only, so the bridge does not act on a short position"
    ),
    (ResearchView.OVERWEIGHT, PositionState.SHORT): (
        "revision 1 is long-only, so the bridge does not act on a short position"
    ),
    (ResearchView.HOLD, PositionState.SHORT): (
        "revision 1 is long-only, so the bridge does not act on a short position"
    ),
    (ResearchView.UNDERWEIGHT, PositionState.SHORT): (
        "revision 1 is long-only, so the bridge does not act on a short position"
    ),
    (ResearchView.SELL, PositionState.SHORT): (
        "revision 1 is long-only, so the bridge does not act on a short position"
    ),
}


def view_of(rating: object) -> ResearchView | Refusal:
    """
    Map a rating string onto the view vocabulary.

    Parameters
    ----------
    rating : object
        The rating as read from the artifact.

    Returns
    -------
    ResearchView | Refusal
        The view, or `RATING_UNKNOWN` when the value is absent or outside the mapping. An absent
        rating is not a `HOLD`: the artifact stating no view and the artifact stating none are
        different records.

    """
    if isinstance(rating, str):
        view = VIEW_MAPPING.get(rating.strip().upper())
        if view is not None:
            return view

    return Refusal(
        RefusalCode.RATING_UNKNOWN,
        f"rating={rating!r} is not in the declared rating vocabulary",
    )


@dataclass(frozen=True)
class RatingDecision:
    """
    What a rating and a position resolve to, with the reason stated.

    Parameters
    ----------
    view : ResearchView
        The mapped view.
    position : PositionState
        The position the decision was resolved against.
    resolution : RatingResolution
        The engine result.
    rationale : str
        Why this cell resolves the way it does.

    """

    view: ResearchView
    position: PositionState
    resolution: RatingResolution
    rationale: str


def is_policy_total() -> bool:
    """
    Return whether the policy states a cell for every rating and position pair.

    Both tables are checked, because a resolution without a stated reason is a policy the bridge
    would be unable to record.

    Returns
    -------
    bool

    """
    expected = {(view, position) for view in ResearchView for position in PositionState}
    return set(POLICY) == expected and set(RATIONALE) == expected


def decide_rating(rating: object, position: PositionState) -> RatingDecision | Refusal:
    """
    Resolve a rating against the position.

    Parameters
    ----------
    rating : object
        The rating as read from the artifact.
    position : PositionState
        The instrument's position.

    Returns
    -------
    RatingDecision | Refusal
        The resolution with its reason, or `RATING_UNKNOWN`.

    """
    view = view_of(rating)
    if isinstance(view, Refusal):
        return view

    return RatingDecision(
        view=view,
        position=position,
        resolution=POLICY[(view, position)],
        rationale=RATIONALE[(view, position)],
    )
