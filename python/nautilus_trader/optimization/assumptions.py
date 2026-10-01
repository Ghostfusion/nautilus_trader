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
Execution assumptions that a bar-only replay cannot determine.

A bar records four prices and no path between them, so an execution simulation must choose an
ordering, a fill rule for a triggered stop, a precedence between simultaneous triggers, and a
treatment for a bar that opens beyond a trigger. Those choices are *assumptions*: they are not
market rules, and they are not execution semantics this layer owns. What this module provides is
their identity, so a study names the assumptions it ran under and two results produced under
different assumptions are distinguishable.

The vocabulary here is deliberately about the **research record**, not about the engine. The
implementation of each rule lives in the Rust matching engine; the citations are in the
documentation and in `vectorbt_lessons_implementation.md` section 4.3.

Ambiguity is refused rather than resolved silently. A policy that declares an intrabar path while
bar execution is off, or that turns bar execution on without naming a path, is a configuration
error: the value of one field is meaningless without the other, and a silent convention would make
two different studies look identical.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from typing import TYPE_CHECKING

from nautilus_trader.optimization.space import digest_of


if TYPE_CHECKING:
    from nautilus_trader.optimization.space import JsonValue


class IntrabarPath(Enum):
    """
    The order in which a bar's prices are visited.
    """

    OHLC_SEQUENCE = "ohlc_sequence"
    ADAPTIVE_NEAREST_TO_OPEN = "adaptive_nearest_to_open"


class TriggerPrecedence(Enum):
    """
    Which trigger wins when several are reachable inside one bar.
    """

    OHLC_SEQUENCE = "ohlc_sequence"


class TriggerFill(Enum):
    """
    The price a triggered order is filled at when the bar does not open beyond its trigger.
    """

    TRIGGER_PRICE_INSIDE_BAR = "trigger_price_inside_bar"


class GapHandling(Enum):
    """
    How a bar that opens beyond a trigger is treated.
    """

    MARKET_PRICE_BEYOND_TRIGGER = "market_price_beyond_trigger"


# The stable identity of the assumption policy this module models, and its version. A change to any
# rule above changes the version, so a historical result names the rule set it was produced under.
BAR_AMBIGUITY_POLICY_ID = "bar_ohlc"
BAR_AMBIGUITY_POLICY_VERSION = 1


@dataclass(frozen=True)
class BarAmbiguityPolicy:
    """
    A declared set of bar-derived execution assumptions, with an identity.

    Parameters
    ----------
    bar_execution : bool
        Whether bars drive order execution at all.
    intrabar_path : IntrabarPath | None
        The order in which the bar's prices are visited, or None when bars do not drive execution.
    trigger_precedence : TriggerPrecedence
        The precedence between simultaneous triggers.
    trigger_fill : TriggerFill
        The fill price rule for a trigger inside the bar.
    gap_handling : GapHandling
        The rule for a bar that opens beyond a trigger.
    policy_id : str
        The stable policy identity.
    version : int
        The policy version, which changes when any rule above changes.

    Raises
    ------
    TypeError
        If a field has the wrong type.
    ValueError
        If the declaration is ambiguous: an intrabar path is declared while bar execution is off, or
        bar execution is on without a declared path.

    """

    bar_execution: bool = True
    intrabar_path: IntrabarPath | None = IntrabarPath.OHLC_SEQUENCE
    trigger_precedence: TriggerPrecedence = TriggerPrecedence.OHLC_SEQUENCE
    trigger_fill: TriggerFill = TriggerFill.TRIGGER_PRICE_INSIDE_BAR
    gap_handling: GapHandling = GapHandling.MARKET_PRICE_BEYOND_TRIGGER
    policy_id: str = BAR_AMBIGUITY_POLICY_ID
    version: int = BAR_AMBIGUITY_POLICY_VERSION

    def __post_init__(self) -> None:
        """
        Validate the declared assumptions and refuse an ambiguous declaration.
        """
        if not isinstance(self.bar_execution, bool):
            raise TypeError(
                f"bar_execution must be a bool, was {type(self.bar_execution).__name__}",
            )
        if self.intrabar_path is not None and not isinstance(self.intrabar_path, IntrabarPath):
            raise TypeError(
                f"intrabar_path must be an IntrabarPath or None, "
                f"was {type(self.intrabar_path).__name__}",
            )
        if not isinstance(self.trigger_precedence, TriggerPrecedence):
            raise TypeError("trigger_precedence must be a TriggerPrecedence")
        if not isinstance(self.trigger_fill, TriggerFill):
            raise TypeError("trigger_fill must be a TriggerFill")
        if not isinstance(self.gap_handling, GapHandling):
            raise TypeError("gap_handling must be a GapHandling")
        if not self.policy_id:
            raise ValueError("policy_id must not be empty")
        if self.version < 1:
            raise ValueError(f"version must be positive, was {self.version}")

        # Ambiguity is a configuration error, not a convention: the path and the flag that makes it
        # meaningful must agree, so a study cannot declare a rule that cannot apply.
        if self.bar_execution and self.intrabar_path is None:
            raise ValueError(
                "bar execution requires a declared intrabar path: "
                "set intrabar_path, or declare bar_execution false",
            )
        if not self.bar_execution and self.intrabar_path is not None:
            raise ValueError(
                f"intrabar_path {self.intrabar_path.value} is meaningless without bar execution: "
                "declare bar_execution true, or leave the path unset",
            )

    @classmethod
    def declared_default(cls) -> BarAmbiguityPolicy:
        """
        Return the policy a study runs under when it declares nothing.

        The default is the behaviour the engine already implements with bar execution enabled and
        adaptive ordering disabled: the fixed Open, High, Low, Close sequence. Naming it is the
        point: the default is a declared assumption rather than an accident of a venue default.

        Returns
        -------
        BarAmbiguityPolicy

        """
        return cls()

    @classmethod
    def from_venue_flags(
        cls,
        *,
        bar_execution: bool,
        adaptive_high_low_ordering: bool,
    ) -> BarAmbiguityPolicy:
        """
        Return the policy equivalent to a venue's bar configuration.

        This is the bridge between a declared assumption and the engine configuration it describes,
        and the single place where the ambiguous combination is refused.

        Parameters
        ----------
        bar_execution : bool
            Whether bars drive order execution.
        adaptive_high_low_ordering : bool
            Whether the price order adapts to the extreme nearest the open.

        Returns
        -------
        BarAmbiguityPolicy

        Raises
        ------
        ValueError
            If adaptive ordering is requested while bar execution is off.

        """
        if not bar_execution and adaptive_high_low_ordering:
            raise ValueError(
                "adaptive_high_low_ordering requires bar execution: "
                "set bar_execution true, or disable adaptive ordering",
            )

        path = None
        if bar_execution:
            path = (
                IntrabarPath.ADAPTIVE_NEAREST_TO_OPEN
                if adaptive_high_low_ordering
                else IntrabarPath.OHLC_SEQUENCE
            )

        return cls(bar_execution=bar_execution, intrabar_path=path)

    def to_dict(self) -> dict[str, JsonValue]:
        """
        Return the policy as a canonical mapping.

        Returns
        -------
        dict[str, JsonValue]

        """
        return {
            "policy_id": self.policy_id,
            "version": self.version,
            "bar_execution": self.bar_execution,
            "intrabar_path": self.intrabar_path.value if self.intrabar_path else None,
            "trigger_precedence": self.trigger_precedence.value,
            "trigger_fill": self.trigger_fill.value,
            "gap_handling": self.gap_handling.value,
        }

    @property
    def digest(self) -> str:
        """
        Return the digest of the declared assumptions.

        Returns
        -------
        str

        """
        return digest_of(self.to_dict())
