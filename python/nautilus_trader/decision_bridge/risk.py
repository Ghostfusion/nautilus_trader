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
The risk-engine configuration the bridge requires, and the vocabulary of its limits.

The bridge's only obligation to the risk engine is to configure it so that a decision cannot exceed
a limit. The engine stays the single authority on whether an order may exist: the bridge does not
pre-check a limit, because a second opinion on a limit is a second authority (`design 4 I5`). The
refusal a breach produces is therefore the engine's own typed denial, naming the limit that bound,
rather than a bridge-side rejection that happened to agree with it.

Settable from Python are the per-instrument notional caps (a mapping of instrument id to a decimal
string, in the instrument's quote currency), the count caps (`RiskCap`, each a metric, a scope, a
positive limit and an optional rolling window in nanoseconds), the order submit and modify rate
limits (as `limit/HH:MM:SS` strings), and the full-position-exit venues. There is deliberately no
`bypass` field: a configuration whose purpose is to enforce limits must not carry a switch that
disables them.

The count caps were the one limit this module could not set, and the gap was recorded here rather
than emulated (`UNREACHABLE_FROM_PYTHON`). Revision 6 closed it: `RiskEngineConfig` takes
`count_caps`, `RiskCap` and its two vocabularies are Python classes, and the live configuration
carries the same caps in its own string form -- `METRIC/SCOPE/LIMIT[/WINDOW_NS]`, parsed into the
same `RiskCap` values and validated by the same builder -- so a cap declared for a live node is not
dropped on the way to the engine.

The design's `UNCERTAIN` reduction factor is not here either: it shapes the signal rather than
judging an order, so it is projection configuration (`design 3.8`), not a risk-engine limit.
"""

from __future__ import annotations

from dataclasses import dataclass
from dataclasses import field
from typing import TYPE_CHECKING

from nautilus_trader.risk import RiskEngineConfig


if TYPE_CHECKING:
    from collections.abc import Mapping
    from collections.abc import Sequence

    from nautilus_trader.model import Venue
    from nautilus_trader.risk import RiskCap


# The `RiskEngineConfig` fields this bridge cannot set from Python, so the reachability is pinned by
# a test rather than rediscovered. The tuple is empty as of revision 6: `count_caps` was the last
# entry, and it is now settable here and carried through the live configuration.
UNREACHABLE_FROM_PYTHON: tuple[str, ...] = ()


@dataclass(frozen=True)
class RiskLimits:
    """
    The desk's declared risk limits, which are policy and not derivable here.

    Every cap is explicit configuration: the notional caps and both rate limits have no default,
    because a default would be a limit the desk did not declare. Only the full-position-exit venues
    default, and they default to empty because they grant no leniency beyond the venues' own
    conditional-exit handling.

    Parameters
    ----------
    max_notional_per_order : Mapping[str, str]
        Maximum notional per order by instrument id, as a decimal string in the instrument's quote
        currency. At least one entry is required: an empty mapping would configure no per-instrument
        cap at all, which is the state this type exists to rule out.
    count_caps : Sequence[RiskCap]
        Count caps, each a metric, a scope, a positive limit and an optional rolling window. Like
        the notional caps they are declared rather than defaulted: a count cap that was not declared
        is not a limit the desk stated, and the bridge must not choose one for it.
    max_order_submit_rate : str
        Rate limit for order submission commands, as `limit/HH:MM:SS`.
    max_order_modify_rate : str
        Rate limit for order modifications, as `limit/HH:MM:SS`.
    full_position_exit_venues : Sequence[Venue], optional
        Venues whose execution clients enforce whole-position conditional exits. Defaults to empty.

    Raises
    ------
    ValueError
        If `max_notional_per_order` is empty. A bridge that declared no per-instrument cap would not
        be enforcing the limit it exists to enforce.

    """

    max_notional_per_order: Mapping[str, str]
    count_caps: Sequence[RiskCap]
    max_order_submit_rate: str
    max_order_modify_rate: str
    full_position_exit_venues: Sequence[Venue] = field(default_factory=tuple)

    def __post_init__(self) -> None:
        """
        Validate that the limits are declared rather than defaulted.
        """
        if not self.max_notional_per_order:
            raise ValueError(
                "max_notional_per_order must declare at least one per-instrument cap",
            )


def build_risk_engine_config(limits: RiskLimits) -> RiskEngineConfig:
    """
    Build the `RiskEngineConfig` from the desk's declared limits.

    The returned configuration is what makes the bridge's obligation to the risk engine real: the
    per-instrument notional caps bound an order, the count caps bound how many actions of a kind may
    be taken over a scope and a window, the rate limits bound submission and modification, and a
    breach is refused by the engine with its own typed denial. The bridge never reads this
    configuration to pre-check an order, because the engine owns that judgement.

    Parameters
    ----------
    limits : RiskLimits
        The desk's declared limits.

    Returns
    -------
    RiskEngineConfig
        The engine configuration, validated by the engine's own constructor.

    """
    return RiskEngineConfig(
        max_notional_per_order=dict(limits.max_notional_per_order),
        count_caps=list(limits.count_caps),
        max_order_submit_rate=limits.max_order_submit_rate,
        max_order_modify_rate=limits.max_order_modify_rate,
        full_position_exit_venues=list(limits.full_position_exit_venues),
    )
