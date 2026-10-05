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
Tests for the rating policy, the disposition mapping and the gate aggregation.
"""

from __future__ import annotations

from itertools import permutations

import pytest

from nautilus_trader.decision_bridge.contract import GATE_SEVERITY_MAPPING
from nautilus_trader.decision_bridge.contract import REGIME_SEVERITY_MAPPING
from nautilus_trader.decision_bridge.contract import Diagnostic
from nautilus_trader.decision_bridge.contract import Refusal
from nautilus_trader.decision_bridge.contract import RefusalCode
from nautilus_trader.decision_bridge.contract import Severity
from nautilus_trader.decision_bridge.contract import most_restrictive
from nautilus_trader.decision_bridge.gate import DispositionConfig
from nautilus_trader.decision_bridge.gate import resolve_gate
from nautilus_trader.decision_bridge.gate import strength_for
from nautilus_trader.decision_bridge.ratings import POLICY
from nautilus_trader.decision_bridge.ratings import RATIONALE
from nautilus_trader.decision_bridge.ratings import PositionState
from nautilus_trader.decision_bridge.ratings import RatingResolution
from nautilus_trader.decision_bridge.ratings import ResearchView
from nautilus_trader.decision_bridge.ratings import decide_rating
from nautilus_trader.decision_bridge.ratings import is_policy_total
from nautilus_trader.decision_bridge.ratings import view_of


def test_the_rating_policy_states_a_resolution_and_a_reason_for_every_cell() -> None:
    """
    Test that the rating policy covers every view and position with a resolution and rationale.
    """
    assert is_policy_total()
    assert len(POLICY) == len(ResearchView) * len(PositionState)
    assert set(POLICY) == set(RATIONALE)


def test_the_producer_five_valued_vocabulary_maps_completely() -> None:
    """
    Test that every producer rating, including case and whitespace variants, maps to a view.
    """
    for rating in ("Buy", "Overweight", "Hold", "Underweight", "Sell"):
        view = view_of(rating)
        assert not isinstance(view, Refusal), rating
    for rating in ("buy", "BUY", " overweight ", "sElL"):
        assert not isinstance(view_of(rating), Refusal), rating


def test_an_absent_or_unrecognised_rating_refuses_rather_than_resolving_to_a_hold() -> None:
    """
    Test that absent or unrecognised ratings refuse with an unknown-rating code.
    """
    for rating in (None, "", "Accumulate", 7, "strong buy"):
        refusal = view_of(rating)
        assert isinstance(refusal, Refusal), rating
        assert refusal.code is RefusalCode.RATING_UNKNOWN, rating


def test_every_row_of_the_design_table_resolves_as_stated() -> None:
    """
    Test that every row of the design table resolves to its stated rating resolution.
    """
    expected = [
        ("Buy", PositionState.FLAT, RatingResolution.LONG_SIGNAL),
        ("Buy", PositionState.LONG, RatingResolution.LONG_SIGNAL),
        ("Hold", PositionState.FLAT, RatingResolution.NO_SIGNAL),
        ("Hold", PositionState.LONG, RatingResolution.NO_SIGNAL),
        ("Sell", PositionState.LONG, RatingResolution.FLAT_TARGET),
        ("Sell", PositionState.FLAT, RatingResolution.NO_SIGNAL),
    ]
    for rating, position, resolution in expected:
        decision = decide_rating(rating, position)
        assert not isinstance(decision, Refusal), (rating, position)
        assert decision.resolution is resolution, (rating, position)


def test_the_weak_siblings_take_their_directional_family_row() -> None:
    """
    Test that Overweight and Underweight adopt the resolutions of Buy and Sell respectively.
    """
    overweight = decide_rating("Overweight", PositionState.FLAT)
    buy = decide_rating("Buy", PositionState.FLAT)
    underweight = decide_rating("Underweight", PositionState.LONG)
    sell = decide_rating("Sell", PositionState.LONG)

    assert not isinstance(overweight, Refusal)
    assert not isinstance(buy, Refusal)
    assert not isinstance(underweight, Refusal)
    assert not isinstance(sell, Refusal)
    assert overweight.resolution is buy.resolution
    assert underweight.resolution is sell.resolution


def test_a_short_position_never_produces_a_direction() -> None:
    """
    Test that no rating produces a directional signal for a short position.
    """
    for rating in ("Buy", "Overweight", "Hold", "Underweight", "Sell"):
        decision = decide_rating(rating, PositionState.SHORT)
        assert not isinstance(decision, Refusal)
        assert decision.resolution is RatingResolution.NO_SIGNAL, rating


def test_no_row_of_the_policy_resolves_a_negative_view_to_a_long() -> None:
    """
    Test that no policy row maps a negative view to a long signal.
    """
    for (view, _position), resolution in POLICY.items():
        if view in (ResearchView.SELL, ResearchView.UNDERWEIGHT):
            assert resolution is not RatingResolution.LONG_SIGNAL, view


def test_the_declared_claim_mapping_is_the_only_thing_that_permits() -> None:
    """
    Test that the gate and regime severity mappings match their declared values exactly.
    """
    assert GATE_SEVERITY_MAPPING == {
        "PASS": Severity.PERMIT,
        "WARN": Severity.UNCERTAIN,
        "REJECT": Severity.RESTRICT,
    }
    assert REGIME_SEVERITY_MAPPING == {
        "PASS": Severity.PERMIT,
        "UNCERTAIN": Severity.UNCERTAIN,
        "REJECT": Severity.RESTRICT,
    }


def test_the_effective_gate_is_the_most_restrictive_of_every_input() -> None:
    """
    Test that the effective gate equals the most restrictive severity across every input.
    """
    verdicts = ("PASS", "WARN", "REJECT", "unknown-value")
    for verdict in verdicts:
        for regime in ("PASS", "UNCERTAIN", "REJECT", None):
            for engine_eligible in (True, False):
                resolution = resolve_gate(
                    claim=verdict,
                    engine_eligible=engine_eligible,
                    regime=regime,
                )
                expected = most_restrictive(
                    resolution.claim_severity,
                    resolution.disposition,
                    resolution.engine_severity,
                )
                assert resolution.effective is expected, (verdict, regime, engine_eligible)


def test_an_unmapped_claim_value_resolves_closed_and_reports_that_it_was_unmapped() -> None:
    """
    Test that an unmapped claim resolves closed and reports itself as unmapped.
    """
    resolution = resolve_gate(claim="GLITCH", engine_eligible=True)

    assert resolution.effective is Severity.RESTRICT
    assert resolution.claim_severity is Severity.RESTRICT
    assert resolution.claim_mapped is False
    assert resolution.claim == "GLITCH"


def test_a_permissive_claim_beside_a_restrictive_engine_keeps_the_engine_answer() -> None:
    """
    Test that a permissive claim cannot override a restrictive engine answer.
    """
    resolution = resolve_gate(claim="PASS", engine_eligible=False)

    assert resolution.effective is Severity.RESTRICT
    assert not resolution.permits
    assert Diagnostic.GATE_CONFLICT in resolution.diagnostics


def test_engine_eligibility_cannot_be_raised_by_a_restrictive_claim() -> None:
    """
    Test that a restrictive claim does not raise engine eligibility.
    """
    resolution = resolve_gate(claim="REJECT", engine_eligible=True)

    assert resolution.effective is Severity.RESTRICT
    assert resolution.engine_eligible is True


def test_the_aggregation_does_not_depend_on_the_order_its_inputs_are_considered_in() -> None:
    """
    Test that gate aggregation is invariant to the order of its severity inputs.
    """
    for claim in ("PASS", "WARN", "REJECT", "GLITCH"):
        for regime in ("PASS", "UNCERTAIN", "REJECT"):
            for eligible in (True, False):
                resolution = resolve_gate(claim=claim, engine_eligible=eligible, regime=regime)
                parts = [
                    resolution.claim_severity,
                    resolution.disposition,
                    resolution.engine_severity,
                ]
                for order in permutations(parts):
                    assert most_restrictive(*order) is resolution.effective, (
                        claim,
                        regime,
                        eligible,
                        order,
                    )


def test_a_reduction_beside_an_eligible_engine_is_not_a_conflict() -> None:
    """
    Test that an uncertain reduction beside an eligible engine raises no conflict.
    """
    resolution = resolve_gate(claim="WARN", engine_eligible=True)

    assert resolution.effective is Severity.UNCERTAIN
    assert resolution.diagnostics == ()


def test_a_permit_contributes_no_strength_and_an_uncertain_disposition_contributes_the_reduction() -> None:
    """
    Test that a permit contributes no strength and uncertainty contributes the reduction.
    """
    config = DispositionConfig(uncertain_reduction=0.25)

    assert strength_for(Severity.PERMIT, config) is None
    assert strength_for(Severity.UNCERTAIN, config) == pytest.approx(0.25)
    with pytest.raises(ValueError, match="no signal"):
        strength_for(Severity.RESTRICT, config)


def test_the_reduction_factor_is_configuration_and_cannot_increase_risk() -> None:
    """
    Test that the reduction factor is configurable within bounds and rejects invalid values.
    """
    assert DispositionConfig().uncertain_reduction == 0.5
    assert DispositionConfig(uncertain_reduction=1.0).uncertain_reduction == 1.0

    for factor in (0.0, -0.1, 1.5, float("nan"), "0.5", True):
        with pytest.raises(ValueError, match="uncertain_reduction"):
            DispositionConfig(uncertain_reduction=factor)  # type: ignore[arg-type]


def test_a_reduction_never_exceeds_the_configured_budget() -> None:
    """
    Test that an uncertain reduction stays within the configured budget.
    """
    for factor in (0.1, 0.25, 0.5, 1.0):
        strength = strength_for(Severity.UNCERTAIN, DispositionConfig(uncertain_reduction=factor))
        assert strength is not None
        assert 0.0 < strength <= 1.0
