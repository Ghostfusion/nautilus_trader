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
Tests for the boundary reader.
"""

from __future__ import annotations

from nautilus_trader.decision_bridge.artifact import ResearchDecision
from nautilus_trader.decision_bridge.artifact import admit
from nautilus_trader.decision_bridge.artifact import artifact_digest
from nautilus_trader.decision_bridge.contract import STAGE_ORDER
from nautilus_trader.decision_bridge.contract import Refusal
from nautilus_trader.decision_bridge.contract import RefusalCode
from nautilus_trader.decision_bridge.contract import stage_of
from tests.unit.decision_bridge.fixtures import AAPL
from tests.unit.decision_bridge.fixtures import AUTHORIZED_PRODUCERS
from tests.unit.decision_bridge.fixtures import IDEMPOTENCY_KEY
from tests.unit.decision_bridge.fixtures import PRODUCER_SERVICE
from tests.unit.decision_bridge.fixtures import canonical_artifact
from tests.unit.decision_bridge.fixtures import producer_recipe_digest
from tests.unit.decision_bridge.fixtures import resolve_instrument


# The declared code set, restated here so that a member added or removed by the implementation is a
# visible change to a test rather than a silent widening of a closed vocabulary.
DECLARED_CODES = {
    "SCHEMA_INVALID",
    "HASH_INVALID",
    "PRODUCER_UNKNOWN",
    "PRODUCER_UNAUTHORIZED",
    "INSTRUMENT_UNKNOWN",
    "ALLOCATION_INVALID",
    "DUPLICATE",
    "MISSING_EXPIRY",
    "PRODUCED_AT_ABSENT",
    "CALENDAR_MISSING",
    "CALENDAR_UNCOVERED",
    "ARTIFACT_EXPIRED",
    "ACTIONABILITY_INVALID",
    "TRADABILITY_UNKNOWN",
    "TRADABILITY_REJECTED",
    "RISK_GATE_REJECT",
    "RATING_UNKNOWN",
    "UNSIZEABLE",
    "FLOAT_CONVERSION_OVERFLOW",
    "ENGINE_RISK_LIMIT",
}


def admit_document(document: dict) -> ResearchDecision | Refusal:
    """
    Admit a document using the fixtures' instrument resolver and authorized producers.
    """
    return admit(
        document,
        resolve_instrument=resolve_instrument,
        authorized_producers=AUTHORIZED_PRODUCERS,
    )


def test_the_refusal_code_set_is_closed_and_every_code_names_its_stage() -> None:
    """
    Test that the refusal codes match the declared set and each maps to a known stage.
    """
    assert {code.value for code in RefusalCode} == DECLARED_CODES
    assert {stage_of(code) for code in RefusalCode} == set(STAGE_ORDER)

    try:
        RefusalCode("NOT_A_DECLARED_CODE")
    except ValueError as exc:
        message = str(exc)
    else:  # pragma: no cover - a code outside the set must not be constructible
        raise AssertionError("an undeclared refusal code was constructible")

    assert "NOT_A_DECLARED_CODE" in message


def test_a_canonical_artifact_is_admitted_with_its_identity_derived_from_its_body() -> None:
    """
    Test that a canonical artifact is admitted with its identity and fields derived from its body.
    """
    document = canonical_artifact()
    decision = admit_document(document)

    assert isinstance(decision, ResearchDecision)
    assert decision.instrument_id == AAPL
    assert decision.ticker == "AAPL"
    assert decision.schema_version == "1.2.0"
    assert decision.artifact_sha256 == producer_recipe_digest(document)
    assert decision.artifact_sha256 == document["artifact_sha256"]
    assert decision.idempotency_key == IDEMPOTENCY_KEY
    assert decision.producer_service == PRODUCER_SERVICE
    assert decision.producer_run_id == "AAPL_20261002_120000"
    assert decision.rating == "Buy"
    assert decision.allocation_pct == 5.0
    assert decision.gate_verdict == "PASS"
    assert decision.unknown_fields == ()
    assert decision.reserved_names_present == ()


def test_a_hash_that_does_not_recompute_is_refused_rather_than_repaired() -> None:
    """
    Test that a document whose sealed hash no longer recomputes is refused as HASH_INVALID.
    """
    document = canonical_artifact()
    document["rating"] = "Sell"  # the body changes, the sealed hash does not

    refusal = admit_document(document)

    assert isinstance(refusal, Refusal)
    assert refusal.code is RefusalCode.HASH_INVALID
    assert artifact_digest(document) != document["artifact_sha256"]


def test_an_unknown_field_is_admitted_and_recorded_rather_than_ignored() -> None:
    """
    Test that an unknown field is admitted and recorded in the decision's unknown_fields.
    """
    document = canonical_artifact(gamma_exposure=12.5)
    decision = admit_document(document)

    assert isinstance(decision, ResearchDecision)
    assert decision.unknown_fields == ("gamma_exposure",)


def test_a_reserved_producer_name_is_recorded_rather_than_honoured() -> None:
    """
    Test that a reserved producer name is recorded without changing the gate verdict.
    """
    document = canonical_artifact(trade_permission="ALLOW")
    decision = admit_document(document)

    assert isinstance(decision, ResearchDecision)
    assert decision.reserved_names_present == ("trade_permission",)
    assert decision.gate_verdict == "PASS"


def test_a_legacy_artifact_that_declares_no_version_is_admitted_as_one_zero_zero() -> None:
    """
    Test that a legacy artifact with no declared version is admitted as 1.0.0.
    """
    document = canonical_artifact(
        seal=False,
        schema_version=None,
        expires_at=None,
        produced_at=None,
        idempotency_key=None,
    )
    document.pop("schema_version")
    decision = admit_document(document)

    assert isinstance(decision, ResearchDecision)
    assert decision.schema_version == "1.0.0"
    assert decision.expires_at is None
    assert decision.produced_at is None
    assert decision.idempotency_key is None
    assert decision.artifact_sha256 == producer_recipe_digest(document)


def test_a_producer_identity_is_required_at_every_schema_version() -> None:
    """
    Test that a document lacking a producer identity is refused as PRODUCER_UNKNOWN.
    """
    document = canonical_artifact(
        seal=False,
        schema_version=None,
        expires_at=None,
        produced_at=None,
        idempotency_key=None,
        producer=None,
    )
    document.pop("schema_version")
    refusal = admit_document(document)

    assert isinstance(refusal, Refusal)
    assert refusal.code is RefusalCode.PRODUCER_UNKNOWN


def test_a_version_that_is_not_major_minor_patch_is_refused() -> None:
    """
    Test that a schema version that is not major.minor.patch is refused as SCHEMA_INVALID.
    """
    refusal = admit_document(canonical_artifact(schema_version="1.2"))

    assert isinstance(refusal, Refusal)
    assert refusal.code is RefusalCode.SCHEMA_INVALID


def test_a_schema_major_this_reader_does_not_know_is_refused() -> None:
    """
    Test that an unknown schema major is refused as SCHEMA_INVALID, naming the major.
    """
    refusal = admit_document(canonical_artifact(schema_version="2.0.0"))

    assert isinstance(refusal, Refusal)
    assert refusal.code is RefusalCode.SCHEMA_INVALID
    assert "major" in refusal.detail


def test_a_version_that_requires_the_strict_fields_refuses_when_one_is_absent() -> None:
    """
    Test that a strict-fields version refuses as SCHEMA_INVALID when expires_at is absent.
    """
    document = canonical_artifact(seal=False)
    document.pop("expires_at")
    document["artifact_sha256"] = producer_recipe_digest(document)

    refusal = admit_document(document)

    assert isinstance(refusal, Refusal)
    assert refusal.code is RefusalCode.SCHEMA_INVALID
    assert "expires_at is required" in refusal.detail


def test_a_mistyped_field_is_refused() -> None:
    """
    Test that each mistyped field is refused as SCHEMA_INVALID.
    """
    for field, value in (
        ("ticker", 42),
        ("effective_date", "02/10/2026"),
        ("rating", 7),
        ("direction", "sideways"),
        ("action", "MAYBE"),
        ("data_quality", "excellent"),
        ("confidence", 1.5),
        ("confidence", "high"),
        ("recommended_allocation_pct", "5"),
        ("produced_at", "2026-10-02"),
        ("produced_at", "2026-10-02T17:00:00"),
        ("idempotency_key", "not-a-uuid"),
        ("artifact_sha256", "ABCDEF"),
        ("producer", "tradingagents"),
        ("risk_gate", "PASS"),
    ):
        document = canonical_artifact()
        document[field] = value
        refusal = admit_document(document)
        assert isinstance(refusal, Refusal), field
        assert refusal.code is RefusalCode.SCHEMA_INVALID, field


def test_an_absent_producer_identity_is_unknown_and_a_known_one_is_unauthorized() -> None:
    """
    Test that an absent producer is PRODUCER_UNKNOWN and an unauthorized one is refused.
    """
    unknown = admit_document(canonical_artifact(producer={"service": ""}))
    assert isinstance(unknown, Refusal)
    assert unknown.code is RefusalCode.PRODUCER_UNKNOWN

    unauthorized = admit(
        canonical_artifact(),
        resolve_instrument=resolve_instrument,
        authorized_producers=frozenset({"someone-else"}),
    )
    assert isinstance(unauthorized, Refusal)
    assert unauthorized.code is RefusalCode.PRODUCER_UNAUTHORIZED


def test_an_unknown_ticker_is_refused() -> None:
    """
    Test that an unknown ticker is refused as INSTRUMENT_UNKNOWN.
    """
    refusal = admit_document(canonical_artifact(ticker="MSFT"))

    assert isinstance(refusal, Refusal)
    assert refusal.code is RefusalCode.INSTRUMENT_UNKNOWN


def test_an_allocation_outside_its_domain_is_refused_and_never_clamped() -> None:
    """
    Test that an allocation outside its domain is refused rather than clamped.
    """
    for value in (-0.01, 100.01, float("nan"), float("inf"), float("-inf")):
        refusal = admit_document(canonical_artifact(recommended_allocation_pct=value))
        assert isinstance(refusal, Refusal), value
        assert refusal.code is RefusalCode.ALLOCATION_INVALID, value


def test_an_artifact_failing_two_stages_reports_the_earlier_stage() -> None:
    """
    Test that an artifact failing two stages reports the earlier admission-stage refusal.
    """
    document = canonical_artifact(seal=False, ticker="MSFT")
    document["artifact_sha256"] = "0" * 64

    refusal = admit_document(document)

    assert isinstance(refusal, Refusal)
    assert refusal.code is RefusalCode.HASH_INVALID
    assert refusal.stage.value == "ADMISSION"


def test_the_identity_distinguishes_a_revision_from_the_decision_it_revises() -> None:
    """
    Test that two documents sharing a date and instrument get distinct decision identities.
    """
    first = admit_document(canonical_artifact())
    second = admit_document(canonical_artifact(idempotency_key="8b1c9a3e-2f4d-4a77-9f6e-6c1b6a2d5e10"))

    assert isinstance(first, ResearchDecision)
    assert isinstance(second, ResearchDecision)
    assert first.decision_id != second.decision_id
    assert first.effective_date == second.effective_date
    assert first.instrument_id == second.instrument_id


def test_re_admitting_the_same_document_yields_the_same_identities() -> None:
    """
    Test that re-admitting the same document yields identical decision and revision identifiers.
    """
    first = admit_document(canonical_artifact())
    second = admit_document(canonical_artifact())

    assert isinstance(first, ResearchDecision)
    assert isinstance(second, ResearchDecision)
    assert first.decision_id == second.decision_id
    assert first.revision_id == second.revision_id
