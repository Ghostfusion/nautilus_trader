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
Tests for the research decision carriage and the admission ledger.
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from nautilus_trader.decision_bridge.carrier import ResearchDecisionCarrier
from nautilus_trader.decision_bridge.contract import Diagnostic
from nautilus_trader.decision_bridge.contract import RefusalCode
from nautilus_trader.decision_bridge.ledger import ADMITTED
from nautilus_trader.decision_bridge.ledger import DUPLICATE
from nautilus_trader.decision_bridge.ledger import EXPOSURE_STAGES
from nautilus_trader.decision_bridge.ledger import REFUSED
from nautilus_trader.decision_bridge.ledger import UNATTRIBUTED
from nautilus_trader.decision_bridge.ledger import AdmissionLedger
from nautilus_trader.model import CustomData
from nautilus_trader.model import DataType
from nautilus_trader.model import register_custom_data_class


def test_same_artifact_twice_yields_one_admitted_record_and_one_duplicate(
    tmp_path: Path,
) -> None:
    """
    A repeated delivery is a recorded no-op, not a second record and not an error.
    """
    ledger = AdmissionLedger(tmp_path)

    first = ledger.admit(
        "decision-1",
        artifact_sha256="sha-1",
        idempotency_key="key-1",
        instrument_id="AAPL.NASDAQ",
        produced_at=1,
        received_at=2,
        effective_date="2026-01-02",
        admission_timestamp=3,
    )
    second = ledger.admit(
        "decision-1",
        artifact_sha256="sha-1",
        idempotency_key="key-1",
        instrument_id="AAPL.NASDAQ",
        produced_at=1,
        received_at=4,
        effective_date="2026-01-02",
        admission_timestamp=5,
    )

    assert first == ADMITTED
    assert second == DUPLICATE
    assert len(ledger.records()) == 1
    assert ledger.records()[0].admission_result == ADMITTED

    duplicates = ledger.duplicates("decision-1")
    assert len(duplicates) == 1
    assert duplicates[0].reason is RefusalCode.DUPLICATE
    assert duplicates[0].received_at == 4


def test_two_artifacts_differing_only_in_production_time_form_one_revision_chain(
    tmp_path: Path,
) -> None:
    """
    Two artifacts for the same instrument and reference date are one ordered chain.
    """
    ledger = AdmissionLedger(tmp_path)

    ledger.admit(
        "decision-1",
        artifact_sha256="sha-1",
        idempotency_key="key-1",
        instrument_id="AAPL.NASDAQ",
        produced_at=100,
        received_at=110,
        effective_date="2026-01-02",
        admission_timestamp=120,
    )
    ledger.admit(
        "decision-2",
        artifact_sha256="sha-2",
        idempotency_key="key-2",
        instrument_id="AAPL.NASDAQ",
        produced_at=200,
        received_at=210,
        effective_date="2026-01-02",
        admission_timestamp=220,
    )

    chain = ledger.revision_chain("decision-1")

    assert [record.decision_id for record in chain] == ["decision-1", "decision-2"]
    assert [record.produced_at for record in chain] == [100, 200]
    assert ledger.revision_chain("decision-2") == chain


def test_decision_reaches_its_signal_target_orders_and_fills_from_the_ledger_alone(
    tmp_path: Path,
) -> None:
    """
    A decision's whole chain is readable after reload, with no replay of the stream.
    """
    ledger = AdmissionLedger(tmp_path)
    ledger.admit(
        "decision-1",
        artifact_sha256="sha-1",
        idempotency_key="key-1",
        instrument_id="AAPL.NASDAQ",
        produced_at=1,
        received_at=2,
        effective_date="2026-01-02",
        admission_timestamp=3,
    )
    ledger.record_signal("decision-1", "signal-1")
    ledger.record_target("decision-1", "target-1")
    ledger.record_order("decision-1", "O-1")
    ledger.record_order("decision-1", "O-2")
    ledger.record_fill("decision-1", "F-1")

    reloaded = AdmissionLedger(tmp_path)
    record = reloaded.record("decision-1")

    assert record is not None
    assert record.projected_signal_id == "signal-1"
    assert record.target_id == "target-1"
    assert record.order_ids == ("O-1", "O-2")
    assert record.fill_ids == ("F-1",)

    assert reloaded.decision_for_order("O-2") == "decision-1"
    assert reloaded.decision_for_fill("F-1") == "decision-1"


def test_diagnostics_are_readable_on_a_record_that_was_admitted(tmp_path: Path) -> None:
    """
    Diagnostics survive beside a record that continued.
    """
    ledger = AdmissionLedger(tmp_path)
    ledger.admit(
        "decision-1",
        artifact_sha256="sha-1",
        idempotency_key="key-1",
        instrument_id="AAPL.NASDAQ",
        produced_at=1,
        received_at=2,
        effective_date="2026-01-02",
        admission_timestamp=3,
        diagnostics=(Diagnostic.GATE_CONFLICT, Diagnostic.ACTIONABILITY_PAST),
    )

    record = AdmissionLedger(tmp_path).record("decision-1")

    assert record is not None
    assert record.admission_result == ADMITTED
    assert record.diagnostics == (Diagnostic.GATE_CONFLICT, Diagnostic.ACTIONABILITY_PAST)


def test_stage_that_did_not_run_leaves_its_quantity_absent(tmp_path: Path) -> None:
    """
    An unreached exposure stage is absent, never a zero.
    """
    ledger = AdmissionLedger(tmp_path)
    ledger.admit(
        "decision-1",
        artifact_sha256="sha-1",
        idempotency_key="key-1",
        instrument_id="AAPL.NASDAQ",
        produced_at=1,
        received_at=2,
        effective_date="2026-01-02",
        admission_timestamp=3,
    )
    ledger.admit(
        "decision-2",
        artifact_sha256="sha-2",
        idempotency_key="key-2",
        admission_result=REFUSED,
        refusal_reason=RefusalCode.SCHEMA_INVALID,
        admission_timestamp=4,
    )
    ledger.record_exposure("decision-1", "research_requested", 0.08)
    ledger.record_exposure("decision-1", "bridge_capped", 0.08)

    reloaded = AdmissionLedger(tmp_path)
    partial = reloaded.record("decision-1")
    refused = reloaded.record("decision-2")

    assert partial is not None
    assert partial.research_requested == 0.08
    assert partial.bridge_capped == 0.08
    assert partial.engine_constructed is None
    assert partial.not_reached == ("engine_constructed", "risk_approved", "actually_filled")

    assert refused is not None
    assert refused.not_reached == EXPOSURE_STAGES


def test_all_five_exposures_are_recorded_for_a_decision_that_fills(tmp_path: Path) -> None:
    """
    The five B15 quantities are readable on a decision that reached a fill.
    """
    ledger = AdmissionLedger(tmp_path)
    ledger.admit(
        "decision-1",
        artifact_sha256="sha-1",
        idempotency_key="key-1",
        instrument_id="AAPL.NASDAQ",
        produced_at=1,
        received_at=2,
        effective_date="2026-01-02",
        admission_timestamp=3,
    )
    ledger.record_exposure("decision-1", "research_requested", 0.08)
    ledger.record_exposure("decision-1", "bridge_capped", 0.08)
    ledger.record_exposure("decision-1", "engine_constructed", 0.05)
    ledger.record_exposure("decision-1", "risk_approved", 0.03)
    ledger.record_exposure("decision-1", "actually_filled", 0.027)

    record = AdmissionLedger(tmp_path).record("decision-1")

    assert record is not None
    assert (
        record.research_requested,
        record.bridge_capped,
        record.engine_constructed,
        record.risk_approved,
        record.actually_filled,
    ) == (0.08, 0.08, 0.05, 0.03, 0.027)
    assert record.not_reached == ()


def test_exposure_stage_cannot_be_rewritten(tmp_path: Path) -> None:
    """
    A later stage fills its own field rather than rewriting an earlier answer.
    """
    ledger = AdmissionLedger(tmp_path)
    ledger.admit(
        "decision-1",
        artifact_sha256="sha-1",
        idempotency_key="key-1",
        instrument_id="AAPL.NASDAQ",
        admission_timestamp=1,
    )
    ledger.record_exposure("decision-1", "risk_approved", 0.03)

    with pytest.raises(ValueError, match="already written"):
        ledger.record_exposure("decision-1", "risk_approved", 0.05)

    assert ledger.record("decision-1").risk_approved == 0.03


def test_order_without_a_decision_identity_is_reported_as_unattributed(
    tmp_path: Path,
) -> None:
    """
    An unknown order is unattributed rather than assigned to the nearest decision.
    """
    ledger = AdmissionLedger(tmp_path)
    ledger.admit(
        "decision-1",
        artifact_sha256="sha-1",
        idempotency_key="key-1",
        instrument_id="AAPL.NASDAQ",
        admission_timestamp=1,
    )

    assert ledger.decision_for_order("O-UNKNOWN") == UNATTRIBUTED
    assert ledger.decision_for_fill("F-UNKNOWN") == UNATTRIBUTED

    ledger.record_order("decision-1", "O-1")

    assert ledger.decision_for_order("O-1") == "decision-1"
    assert ledger.decision_for_order("O-2") == UNATTRIBUTED


def test_carrier_round_trips_through_custom_data_json() -> None:
    """
    The carrier survives the platform's custom-data JSON boundary unchanged.
    """
    register_custom_data_class(ResearchDecisionCarrier)
    document = json.dumps(
        {"ticker": "AAPL", "rating": "Buy", "recommended_allocation_pct": 8.0},
        sort_keys=True,
    )
    carrier = ResearchDecisionCarrier(
        decision_id="decision-1",
        artifact_sha256="sha-1",
        idempotency_key="key-1",
        produced_at="2026-01-02T09:31:00Z",
        effective_date="2026-01-02",
        document=document,
        ts_event=5,
        ts_init=7,
    )
    data_type = DataType("ResearchDecisionCarrier", {"source": "test"}, "decision-1")

    encoded = CustomData(data_type, carrier).to_json_bytes()
    restored = CustomData.from_json_bytes(encoded)

    assert restored.data_type == data_type
    assert restored.data == carrier
    assert restored.data.decision_id == "decision-1"
    assert restored.data.artifact_sha256 == "sha-1"
    assert restored.data.idempotency_key == "key-1"
    assert restored.data.produced_at == "2026-01-02T09:31:00Z"
    assert restored.data.effective_date == "2026-01-02"
    assert json.loads(restored.data.document)["ticker"] == "AAPL"
    assert restored.ts_event == 5
    assert restored.ts_init == 7


def test_deliberate_attempts_are_recorded_beside_the_orders(tmp_path: Path) -> None:
    """
    Retry attempts round-trip, dedupe, and are readable as a count.
    """
    ledger = AdmissionLedger(tmp_path)
    ledger.admit(
        "decision-1",
        artifact_sha256="sha-1",
        idempotency_key="key-1",
        instrument_id="AAPL.NASDAQ",
        admission_timestamp=1,
    )
    ledger.record_order("decision-1", "O-1")
    ledger.record_attempt("decision-1", "attempt-1")
    ledger.record_attempt("decision-1", "attempt-1")
    ledger.record_attempt("decision-1", "attempt-2")

    assert ledger.attempt_count("decision-1") == 2

    record = AdmissionLedger(tmp_path).record("decision-1")

    assert record is not None
    assert record.attempts == ("attempt-1", "attempt-2")
    assert record.order_ids == ("O-1",)


def test_the_risk_answer_is_the_only_stage_a_later_answer_may_revise(
    tmp_path: Path,
) -> None:
    """
    The risk stage revises its own field; every other stage still refuses a rewrite.
    """
    ledger = AdmissionLedger(tmp_path)
    ledger.admit(
        "decision-1",
        artifact_sha256="sha-1",
        idempotency_key="key-1",
        admission_timestamp=1,
    )
    ledger.record_exposure("decision-1", "engine_constructed", 0.5)
    ledger.record_risk_answer("decision-1", 0.0)
    ledger.record_risk_answer("decision-1", 0.5)

    reloaded = AdmissionLedger(tmp_path).record("decision-1")

    assert reloaded is not None
    assert reloaded.risk_approved == 0.5
    assert reloaded.engine_constructed == 0.5

    with pytest.raises(ValueError, match="already written"):
        ledger.record_exposure("decision-1", "engine_constructed", 0.9)


def test_a_document_without_the_attempts_field_loads_unchanged(tmp_path: Path) -> None:
    """
    An older ledger document loads with an empty attempt set rather than failing.
    """
    ledger = AdmissionLedger(tmp_path)
    ledger.admit(
        "decision-1",
        artifact_sha256="sha-1",
        idempotency_key="key-1",
        admission_timestamp=1,
    )
    ledger.record_attempt("decision-1", "attempt-1")

    path = next((tmp_path / "decisions").glob("*.json"))
    document = json.loads(path.read_text(encoding="utf-8"))
    assert document["record"]["attempts"] == ["attempt-1"]
    del document["record"]["attempts"]
    path.write_text(json.dumps(document), encoding="utf-8")

    record = AdmissionLedger(tmp_path).record("decision-1")

    assert record is not None
    assert record.attempts == ()
    assert ledger.attempt_count("decision-1") == 1
