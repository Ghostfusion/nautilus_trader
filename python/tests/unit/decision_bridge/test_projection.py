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
Tests for the projection: one artifact becomes one statement of view, or a named refusal.
"""

from __future__ import annotations

from decimal import Decimal

from nautilus_trader.decision_bridge.contract import Diagnostic
from nautilus_trader.decision_bridge.contract import Outcome
from nautilus_trader.decision_bridge.contract import Refusal
from nautilus_trader.decision_bridge.contract import RefusalCode
from nautilus_trader.decision_bridge.gate import DispositionConfig
from nautilus_trader.decision_bridge.projection import ProjectionConfig
from nautilus_trader.decision_bridge.projection import ProjectionContext
from nautilus_trader.decision_bridge.projection import ProjectionResult
from nautilus_trader.decision_bridge.projection import project
from nautilus_trader.decision_bridge.ratings import PositionState
from nautilus_trader.decision_bridge.tradability import Tradability
from nautilus_trader.model import SignalDirection
from tests.unit.decision_bridge.fixtures import AAPL
from tests.unit.decision_bridge.fixtures import AUTHORIZED_PRODUCERS
from tests.unit.decision_bridge.fixtures import NS
from tests.unit.decision_bridge.fixtures import PRODUCER_SERVICE
from tests.unit.decision_bridge.fixtures import RUN_ID
from tests.unit.decision_bridge.fixtures import bundled_calendar
from tests.unit.decision_bridge.fixtures import covered_artifact
from tests.unit.decision_bridge.fixtures import resolve_instrument
from tests.unit.decision_bridge.fixtures import unix_ns


HORIZON_NS = 5 * 86_400 * NS
RECEIVED_AT = unix_ns(2025, 6, 2, 17)
EXPIRES_AT = unix_ns(2025, 6, 6, 20)


def projection_config(**overrides: object) -> ProjectionConfig:
    """
    Build a projection config from the defaults, applying any keyword overrides.
    """
    settings: dict[str, object] = {
        "horizon_ns": HORIZON_NS,
        "authorized_producers": AUTHORIZED_PRODUCERS,
    }
    settings.update(overrides)
    return ProjectionConfig(**settings)  # type: ignore[arg-type]


def projection_context(**overrides: object) -> ProjectionContext:
    """
    Build a projection context from the defaults, applying any keyword overrides.
    """
    settings: dict[str, object] = {
        "received_at": RECEIVED_AT,
        "tradability": Tradability.TRADABLE,
        "position": PositionState.FLAT,
        "engine_eligible": True,
        "calendar": bundled_calendar(),
        "regime": None,
    }
    settings.update(overrides)
    return ProjectionContext(**settings)  # type: ignore[arg-type]


def run(document: dict, context: ProjectionContext | None = None, config: ProjectionConfig | None = None) -> ProjectionResult:
    """
    Project the given document, supplying default context and config if omitted.
    """
    return project(
        document,
        context=projection_context() if context is None else context,
        config=projection_config() if config is None else config,
        resolve_instrument=resolve_instrument,
    )


def test_a_canonical_artifact_projects_to_a_signal_whose_every_field_is_derived() -> None:
    """
    Test that a canonical artifact projects to a fully derived long signal.
    """
    result = run(covered_artifact())

    assert not result.refused
    assert result.outcome is Outcome.SIGNAL
    signal = result.signal
    assert signal is not None
    assert signal.instrument_id == AAPL
    assert signal.direction is SignalDirection.LONG
    assert signal.horizon_ns == HORIZON_NS
    assert signal.strength is None
    assert signal.source == PRODUCER_SERVICE
    assert signal.expiry_ns == EXPIRES_AT
    assert signal.ts_event == RECEIVED_AT
    assert signal.ts_init == RECEIVED_AT
    assert result.actionable_at == RECEIVED_AT
    assert result.ceiling == Decimal("0.05")


def test_the_provenance_carries_the_decision_identity_and_the_advisory_levels() -> None:
    """
    Test that the signal provenance records the artifact's identity and advisory levels.
    """
    result = run(covered_artifact())
    assert result.signal is not None
    provenance = result.signal.provenance

    assert provenance["decision_id"] == result.decision.decision_id  # type: ignore[union-attr]
    assert provenance["rating"] == "Buy"
    assert provenance["view"] == "BUY"
    assert provenance["producer_run_id"] == RUN_ID
    assert provenance["gate_claim"] == "PASS"
    assert provenance["gate_claim_mapped"] is True
    assert provenance["gate_effective"] == "PERMIT"
    assert provenance["position"]["entry_price"] == 191.34
    assert provenance["position"]["stop_loss"] == 188.0
    assert provenance["ceiling_fraction"] == "0.050000"
    assert provenance["allocation_pct"] == 5.0


def test_the_models_confidence_is_carried_unchanged_and_never_becomes_the_strength() -> None:
    """
    Test that the model confidence passes through provenance and strength stays None.
    """
    result = run(covered_artifact(confidence=0.72))
    assert result.signal is not None

    assert result.signal.provenance["confidence"] == 0.72
    assert result.signal.strength != 0.72
    assert result.signal.strength is None


def test_an_uncertain_resolution_carries_the_configured_reduction_and_not_the_confidence() -> None:
    """
    Test that a WARN gate yields the configured uncertain reduction as the strength.
    """
    result = run(
        covered_artifact(risk_gate={"verdict": "WARN", "reasons": ["wide spread"]}, confidence=0.9),
        config=projection_config(disposition=DispositionConfig(uncertain_reduction=0.25)),
    )
    assert result.signal is not None

    assert result.signal.strength == 0.25
    assert result.signal.provenance["confidence"] == 0.9


def test_two_projections_of_the_same_artifact_are_equal() -> None:
    """
    Test that two projections of the same artifact are equal.
    """
    first = run(covered_artifact())
    second = run(covered_artifact())

    assert first.signal is not None
    assert second.signal is not None
    assert str(first.signal) == str(second.signal)
    assert first.outcome is second.outcome
    assert first.actionable_at == second.actionable_at
    assert first.ceiling == second.ceiling


def test_the_first_failing_stage_names_the_refusal() -> None:
    """
    Test that the earliest failing stage determines the refusal code.
    """
    admission_first = run(
        covered_artifact(ticker="MSFT", expires_at="2025-06-02T17:00:00+00:00"),
    )
    assert isinstance(admission_first.refusal, Refusal)
    assert admission_first.refusal.code is RefusalCode.INSTRUMENT_UNKNOWN

    availability_first = run(covered_artifact(rating="Accumulate", expires_at="2025-06-02T17:00:00+00:00"))
    assert isinstance(availability_first.refusal, Refusal)
    assert availability_first.refusal.code is RefusalCode.ARTIFACT_EXPIRED

    projection_last = run(covered_artifact(rating="Accumulate"))
    assert isinstance(projection_last.refusal, Refusal)
    assert projection_last.refusal.code is RefusalCode.RATING_UNKNOWN


def test_a_refusal_is_never_a_partial_projection() -> None:
    """
    Test that a refusal yields no signal while the decision is still projected.
    """
    result = run(covered_artifact(rating=None))

    assert result.refused
    assert result.signal is None
    assert result.outcome is Outcome.REFUSED
    assert result.decision is not None


def test_a_restrictive_claim_closes_the_gate_before_any_signal_exists() -> None:
    """
    Test that a REJECT risk gate refuses with a risk gate code and yields no signal.
    """
    result = run(covered_artifact(risk_gate={"verdict": "REJECT", "reasons": ["halt"]}))

    assert result.refused
    assert result.refusal is not None
    assert result.refusal.code is RefusalCode.RISK_GATE_REJECT
    assert result.signal is None


def test_a_research_permit_beside_an_engine_refusal_produces_no_order() -> None:
    """
    Test that a permit claim beside an ineligible engine refuses with a gate conflict.
    """
    result = run(
        covered_artifact(risk_gate={"verdict": "PASS", "reasons": []}),
        context=projection_context(engine_eligible=False),
    )

    assert result.refused
    assert result.refusal is not None
    assert result.refusal.code is RefusalCode.RISK_GATE_REJECT
    assert Diagnostic.GATE_CONFLICT in result.diagnostics


def test_an_unmapped_claim_value_resolves_to_restriction() -> None:
    """
    Test that an unrecognized claim verdict resolves to restriction and refuses.
    """
    result = run(covered_artifact(risk_gate={"verdict": "MAYBE", "reasons": []}))

    assert result.refused
    assert result.refusal is not None
    assert result.refusal.code is RefusalCode.RISK_GATE_REJECT


def test_tradability_that_is_not_positively_established_produces_no_order() -> None:
    """
    Test that unknown and not-tradable contexts each refuse with their own code.
    """
    unknown = run(covered_artifact(), context=projection_context(tradability=Tradability.UNKNOWN))
    assert unknown.refusal is not None
    assert unknown.refusal.code is RefusalCode.TRADABILITY_UNKNOWN
    assert unknown.signal is None

    rejected = run(covered_artifact(), context=projection_context(tradability=Tradability.NOT_TRADABLE))
    assert rejected.refusal is not None
    assert rejected.refusal.code is RefusalCode.TRADABILITY_REJECTED
    assert rejected.signal is None


def test_a_directional_view_beside_a_zero_allocation_yields_no_signal_and_a_conflict() -> None:
    """
    Test that a directional view with a zero allocation yields no signal and a conflict.
    """
    result = run(
        covered_artifact(rating="Buy", recommended_allocation_pct=0),
        context=projection_context(position=PositionState.FLAT),
    )

    assert result.outcome is Outcome.NO_SIGNAL
    assert result.signal is None
    assert Diagnostic.GATE_CONFLICT in result.diagnostics
    assert result.ceiling == Decimal(0)


def test_a_negative_view_on_a_long_position_resolves_to_a_flat_signal_and_records_the_conflict() -> None:
    """
    Test that a sell view on a long position flattens the signal and records a conflict.
    """
    result = run(
        covered_artifact(rating="Sell", recommended_allocation_pct=5.0),
        context=projection_context(position=PositionState.LONG),
    )

    assert result.outcome is Outcome.SIGNAL
    assert result.signal is not None
    assert result.signal.direction is SignalDirection.FLAT
    assert Diagnostic.GATE_CONFLICT in result.diagnostics


def test_a_hold_produces_no_signal_in_both_position_states() -> None:
    """
    Test that a hold rating yields no signal in every position state without refusing.
    """
    for position in (PositionState.FLAT, PositionState.LONG, PositionState.SHORT):
        result = run(
            covered_artifact(rating="Hold"),
            context=projection_context(position=position),
        )
        assert result.outcome is Outcome.NO_SIGNAL, position
        assert result.signal is None, position
        assert result.outcome is not Outcome.REFUSED, position


def test_a_negative_view_on_a_flat_position_produces_no_signal_because_revision_one_is_long_only() -> None:
    """
    Test that a sell view on a flat position yields no signal under the long-only revision.
    """
    result = run(
        covered_artifact(rating="Sell"),
        context=projection_context(position=PositionState.FLAT),
    )

    assert result.outcome is Outcome.NO_SIGNAL
    assert result.signal is None
    assert result.refusal is None


def test_an_allocation_outside_its_domain_stops_at_admission() -> None:
    """
    Test that an allocation above its domain is refused at admission.
    """
    result = run(covered_artifact(recommended_allocation_pct=101.0))

    assert result.refusal is not None
    assert result.refusal.code is RefusalCode.ALLOCATION_INVALID


def test_a_projection_reads_no_clock_and_no_global() -> None:
    """
    Test that actionability derives from the supplied received_at with no clock.
    """
    first = run(covered_artifact(), context=projection_context(received_at=RECEIVED_AT))
    second = run(covered_artifact(), context=projection_context(received_at=RECEIVED_AT + 3_600 * NS))

    assert first.actionable_at == RECEIVED_AT
    assert second.actionable_at == RECEIVED_AT + 3_600 * NS
    assert first.signal is not None
    assert second.signal is not None
    assert first.signal.ts_init == first.actionable_at
    assert second.signal.ts_init == second.actionable_at
