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
Tests for the bar-derived execution assumption policy.
"""

import pytest

from nautilus_trader.optimization.assumptions import BAR_AMBIGUITY_POLICY_ID
from nautilus_trader.optimization.assumptions import BAR_AMBIGUITY_POLICY_VERSION
from nautilus_trader.optimization.assumptions import BarAmbiguityPolicy
from nautilus_trader.optimization.assumptions import GapHandling
from nautilus_trader.optimization.assumptions import IntrabarPath
from nautilus_trader.optimization.assumptions import TriggerFill
from nautilus_trader.optimization.assumptions import TriggerPrecedence


def test_declared_default_is_the_fixed_ohlc_path() -> None:
    """
    Test the declared default states the fixed OHLC path rather than assuming it.
    """
    policy = BarAmbiguityPolicy.declared_default()

    assert policy.policy_id == BAR_AMBIGUITY_POLICY_ID
    assert policy.version == BAR_AMBIGUITY_POLICY_VERSION
    assert policy.bar_execution is True
    assert policy.intrabar_path is IntrabarPath.OHLC_SEQUENCE
    assert policy.trigger_precedence is TriggerPrecedence.OHLC_SEQUENCE
    assert policy.trigger_fill is TriggerFill.TRIGGER_PRICE_INSIDE_BAR
    assert policy.gap_handling is GapHandling.MARKET_PRICE_BEYOND_TRIGGER

    # The default is the venue configuration with adaptive ordering disabled, named.
    assert policy == BarAmbiguityPolicy.from_venue_flags(
        bar_execution=True,
        adaptive_high_low_ordering=False,
    )
    assert (
        policy.digest
        == BarAmbiguityPolicy.from_venue_flags(
            bar_execution=True,
            adaptive_high_low_ordering=False,
        ).digest
    )


def test_adaptive_ordering_selects_the_adaptive_path() -> None:
    """
    Test adaptive ordering is expressed as an intrabar path rather than a flag.
    """
    policy = BarAmbiguityPolicy.from_venue_flags(
        bar_execution=True,
        adaptive_high_low_ordering=True,
    )

    assert policy.intrabar_path is IntrabarPath.ADAPTIVE_NEAREST_TO_OPEN
    assert policy.digest != BarAmbiguityPolicy.declared_default().digest


def test_no_bar_execution_leaves_the_path_unset() -> None:
    """
    Test a policy without bar execution declares no intrabar path.
    """
    policy = BarAmbiguityPolicy.from_venue_flags(
        bar_execution=False,
        adaptive_high_low_ordering=False,
    )

    assert policy.bar_execution is False
    assert policy.intrabar_path is None
    assert policy.digest != BarAmbiguityPolicy.declared_default().digest


def test_ambiguous_declarations_are_refused_with_a_named_constraint() -> None:
    """
    Test an assumption that cannot apply is a configuration error, not a convention.
    """
    with pytest.raises(ValueError, match="adaptive_high_low_ordering requires bar execution"):
        BarAmbiguityPolicy.from_venue_flags(
            bar_execution=False,
            adaptive_high_low_ordering=True,
        )

    with pytest.raises(ValueError, match="requires a declared intrabar path"):
        BarAmbiguityPolicy(bar_execution=True, intrabar_path=None)

    with pytest.raises(ValueError, match="meaningless without bar execution"):
        BarAmbiguityPolicy(bar_execution=False, intrabar_path=IntrabarPath.OHLC_SEQUENCE)


def test_policy_versions_are_distinguishable() -> None:
    """
    Test two policies produced under different versions are distinguishable.
    """
    declared = BarAmbiguityPolicy.declared_default()
    later = BarAmbiguityPolicy(version=BAR_AMBIGUITY_POLICY_VERSION + 1)

    assert later.version != declared.version
    assert later.digest != declared.digest

    # The same declaration digests the same, whatever the instance.
    assert declared.digest == BarAmbiguityPolicy.declared_default().digest
    assert declared.digest.startswith("sha256:")


def test_every_documented_assumption_is_in_the_identity() -> None:
    """
    Test the identity payload carries every assumption the documentation names.
    """
    policy = BarAmbiguityPolicy.declared_default()

    assert set(policy.to_dict()) == {
        "policy_id",
        "version",
        "bar_execution",
        "intrabar_path",
        "trigger_precedence",
        "trigger_fill",
        "gap_handling",
    }

    # Each rule axis is a member of a closed set, so a new rule cannot appear unnoticed.
    for axis, members in (
        (IntrabarPath, {"ohlc_sequence", "adaptive_nearest_to_open"}),
        (TriggerPrecedence, {"ohlc_sequence"}),
        (TriggerFill, {"trigger_price_inside_bar"}),
        (GapHandling, {"market_price_beyond_trigger"}),
    ):
        assert {member.value for member in axis} == members


def test_a_mistyped_declaration_is_refused() -> None:
    """
    Test a declaration outside the vocabulary types is refused.
    """
    with pytest.raises(TypeError, match="intrabar_path must be an IntrabarPath or None"):
        BarAmbiguityPolicy(intrabar_path="ohlc_sequence")  # type: ignore[arg-type]

    with pytest.raises(TypeError, match="bar_execution must be a bool"):
        BarAmbiguityPolicy(bar_execution=1)  # type: ignore[arg-type]

    with pytest.raises(ValueError, match="version must be positive"):
        BarAmbiguityPolicy(version=0)
