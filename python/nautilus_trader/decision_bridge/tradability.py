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
Tradability is positively established, per leg, and unknown is not tradable.

No instrument reaches construction unless the engine has positively answered that it is tradeable.
The answer is three-valued rather than two-valued, and a requirement of the form "the status is not
a
rejection" is insufficient because it admits the unknown case -- and an instrument whose venue or
instrument class this bridge does not recognise is exactly the case that must not reach
construction.

The state is kept here rather than read from the execution engine's own market status, and that is a
deliberate choice with evidence behind it. The engine's one status-to-action funnel matches a
trading
or pre-open action and drops every other action through a wildcard arm, so a "not available for
trading" action changes nothing there; its initial and reset state is open, so an instrument that
never receives a status is tradeable by default; and the venue-supplied
`InstrumentStatus.is_trading` flag is read by no engine, risk or strategy path at all. Deriving
tradability from that mapping would therefore answer "permitted" for instruments the venue has
explicitly withdrawn, which is the wrong direction of failure for a gate that guards real capital.
Keeping the state beside the decision it guards leaves every existing strategy's behaviour
unchanged.

The requirement is per leg, not per strategy: a multi-leg instrument's every leg is established
separately, because a spread whose second leg cannot trade is not tradable. A stale observation
never replaces a newer one, because a replay can deliver statuses out of order and the newest
statement about
an instrument is the only one that is true.
"""

from __future__ import annotations

from dataclasses import dataclass
from dataclasses import field
from enum import Enum
from enum import unique
from typing import TYPE_CHECKING

from nautilus_trader.decision_bridge.contract import Refusal
from nautilus_trader.decision_bridge.contract import RefusalCode
from nautilus_trader.model import MarketStatusAction


if TYPE_CHECKING:
    from collections.abc import Sequence

    from nautilus_trader.model import InstrumentId
    from nautilus_trader.model import InstrumentStatus


@unique
class Tradability(Enum):
    """
    What the engine has positively established about an instrument.

    `UNKNOWN` is not tradable. It means "no statement was made", not "no objection was raised".
    """

    TRADABLE = "TRADABLE"
    NOT_TRADABLE = "NOT_TRADABLE"
    UNKNOWN = "UNKNOWN"


# The actions that positively establish that an instrument is tradeable. Only a trading action says
# trading is open; a pre-open or quoting action says the session exists, which is not the same
# claim.
POSITIVELY_TRADABLE_ACTIONS: frozenset[MarketStatusAction] = frozenset({MarketStatusAction.TRADING})

# The actions that positively refuse. Everything else establishes nothing and therefore resolves to
# `UNKNOWN` rather than to permission.
REFUSING_ACTIONS: frozenset[MarketStatusAction] = frozenset(
    {
        MarketStatusAction.CLOSE,
        MarketStatusAction.PRE_CLOSE,
        MarketStatusAction.POST_CLOSE,
        MarketStatusAction.HALT,
        MarketStatusAction.PAUSE,
        MarketStatusAction.SUSPEND,
        MarketStatusAction.NOT_AVAILABLE_FOR_TRADING,
    },
)


def from_status(status: InstrumentStatus) -> Tradability:
    """
    Derive one instrument's tradability from a status event.

    Parameters
    ----------
    status : InstrumentStatus
        The status event, as the venue reported it.

    Returns
    -------
    Tradability
        `TRADABLE` only when the venue said so, either through its own trading flag or through a
        trading action. Every unmapped action establishes nothing.

    """
    is_trading = status.is_trading
    if is_trading is True:
        return Tradability.TRADABLE
    if is_trading is False:
        return Tradability.NOT_TRADABLE

    action = status.action
    if action in POSITIVELY_TRADABLE_ACTIONS:
        return Tradability.TRADABLE
    if action in REFUSING_ACTIONS:
        return Tradability.NOT_TRADABLE

    return Tradability.UNKNOWN


def combine(states: Sequence[Tradability]) -> Tradability:
    """
    Combine the tradability of an instrument's legs.

    A spread whose second leg cannot trade is not tradable, so a refusal anywhere refuses the whole,
    and an unresolvable leg is not permission for the rest.

    Parameters
    ----------
    states : Sequence[Tradability]
        The per-leg states. An empty sequence is `UNKNOWN`: no leg was established.

    Returns
    -------
    Tradability

    """
    if not states:
        return Tradability.UNKNOWN
    if any(state is Tradability.NOT_TRADABLE for state in states):
        return Tradability.NOT_TRADABLE
    if any(state is Tradability.UNKNOWN for state in states):
        return Tradability.UNKNOWN

    return Tradability.TRADABLE


def require(state: Tradability, instrument_id: InstrumentId | str) -> Refusal | None:
    """
    Return a refusal unless tradability was positively established.

    Parameters
    ----------
    state : Tradability
        The established state.
    instrument_id : InstrumentId | str
        The instrument the state is about, for the refusal's detail.

    Returns
    -------
    None | Refusal
        `None` when the instrument may be acted on, otherwise `TRADABILITY_UNKNOWN` for a state that
        establishes nothing and `TRADABILITY_REJECTED` for one that positively refuses.

    """
    if state is Tradability.TRADABLE:
        return None
    if state is Tradability.NOT_TRADABLE:
        return Refusal(
            RefusalCode.TRADABILITY_REJECTED,
            f"{instrument_id} is positively not tradable",
        )

    return Refusal(
        RefusalCode.TRADABILITY_UNKNOWN,
        f"tradability of {instrument_id} has not been positively established",
    )


@dataclass
class TradabilityTracker:
    """
    The bridge's own three-valued tradability state, per instrument.

    The tracker is a plain observer of status events: it reads no clock, holds no engine reference,
    and changes no engine behaviour. Its initial state for every instrument is `UNKNOWN`, which is
    not
    tradable, so an instrument that never reports a status can never be acted on.
    """

    _states: dict[str, tuple[int, Tradability]] = field(default_factory=dict)

    def observe(self, status: InstrumentStatus) -> Tradability:
        """
        Record a status event.

        A status whose event time is older than the newest one already recorded is ignored, and the
        state already held is returned: a replay can deliver statuses out of order, and the newest
        statement about an instrument is the only one that is still true.

        Parameters
        ----------
        status : InstrumentStatus
            The status event.

        Returns
        -------
        Tradability
            The state now held for the instrument.

        """
        key = str(status.instrument_id)
        observed = from_status(status)
        event_ns = int(status.ts_event)
        held = self._states.get(key)
        if held is not None and event_ns < held[0]:
            return held[1]

        self._states[key] = (event_ns, observed)
        return observed

    def state_of(self, instrument_id: InstrumentId | str) -> Tradability:
        """
        Return the state held for one instrument.

        Parameters
        ----------
        instrument_id : InstrumentId | str
            The instrument.

        Returns
        -------
        Tradability
            `UNKNOWN` when no status has been observed for it.

        """
        held = self._states.get(str(instrument_id))
        return Tradability.UNKNOWN if held is None else held[1]

    def state_of_legs(self, legs: Sequence[InstrumentId | str]) -> Tradability:
        """
        Return the combined state of an instrument's legs.

        Parameters
        ----------
        legs : Sequence[InstrumentId | str]
            Every leg, established separately.

        Returns
        -------
        Tradability

        """
        return combine([self.state_of(leg) for leg in legs])
