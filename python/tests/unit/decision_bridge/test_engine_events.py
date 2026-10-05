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
Tests for the engine-constructed exposure, the risk stage, and deliberate retries.
"""

from __future__ import annotations

from dataclasses import dataclass
from decimal import Decimal
from pathlib import Path

import pytest

from nautilus_trader.decision_bridge.contract import Outcome
from nautilus_trader.decision_bridge.execution import RETRYABLE_DENIAL_CODES
from nautilus_trader.decision_bridge.execution import RetryDecision
from nautilus_trader.decision_bridge.execution import RetryPolicy
from nautilus_trader.decision_bridge.execution import leading_denial_code
from nautilus_trader.decision_bridge.execution import record_engine_event
from nautilus_trader.decision_bridge.execution import retry_decision
from nautilus_trader.decision_bridge.execution import submit_once
from nautilus_trader.decision_bridge.ledger import UNATTRIBUTED
from nautilus_trader.decision_bridge.ledger import AdmissionLedger
from nautilus_trader.model import InstrumentId
from nautilus_trader.model import Quantity
from nautilus_trader.model import Target
from nautilus_trader.trading import TargetPipelineConfig


IID = InstrumentId.from_str("AAPL.XNAS")
DECISION_ID = "decision-1"

BASE_PIPELINE = TargetPipelineConfig(
    risk_per_trade=Decimal("0.01"),
    stop_loss_bps=100,
    max_weight=Decimal("1.0"),
    commission_rate=Decimal(0),
)


@dataclass
class _Decision:
    decision_id: str
    allocation_pct: float | None = None


@dataclass
class _Projection:
    decision: _Decision | None
    signal: object | None = object()
    outcome: Outcome = Outcome.SIGNAL
    detail: str = ""


class _FakeStrategy:
    """
    A strategy stand-in that returns the constructed targets and order ids it was given.

    The real `Strategy.targets` binding may not exist in a build that has not been rebuilt, so the
    unit tests stand the engine aside and exercise the bridge's own reads.
    """

    def __init__(
        self,
        *,
        targets: tuple[object, ...] = (),
        order_id_sets: tuple[tuple[str, ...], ...] = ((),),
        targets_error: Exception | None = None,
        submit_error_at: int | None = None,
    ) -> None:
        self._targets = list(targets)
        self._order_id_sets = [list(ids) for ids in order_id_sets]
        self._targets_error = targets_error
        self._submit_error_at = submit_error_at
        self.enabled: TargetPipelineConfig | None = None
        self.submit_calls = 0

    def enable_target_pipeline(self, config: TargetPipelineConfig) -> None:
        self.enabled = config

    def targets(self, _signals: object) -> list[object]:
        if self._targets_error is not None:
            raise self._targets_error
        return list(self._targets)

    def submit_signals(self, _signals: object) -> list[str]:
        self.submit_calls += 1
        if self._submit_error_at == self.submit_calls:
            raise RuntimeError("STREAM_RECONCILING: execution stream unavailable or recovering")
        index = min(self.submit_calls - 1, len(self._order_id_sets) - 1)
        return list(self._order_id_sets[index])


def _ledger(tmp_path: Path) -> AdmissionLedger:
    ledger = AdmissionLedger(tmp_path)
    ledger.admit(
        DECISION_ID,
        artifact_sha256="sha-1",
        idempotency_key="key-1",
        instrument_id=str(IID),
        admission_timestamp=1,
    )
    return ledger


def _permitted_retry(attempts: int, code: str = "STREAM_RECONCILING") -> RetryDecision:
    return retry_decision(
        policy=RetryPolicy(max_attempts=5),
        attempts=attempts,
        denial_code=f"{code}: the execution stream is recovering, retry after recovery",
        attempt="attempt-1",
    )


# --- Change 1: the engine-constructed exposure ------------------------------------------------


def test_submit_once_records_the_weight_target_as_percent(tmp_path: Path) -> None:
    """
    A weight target's fraction is recorded as the artifact's percent units.
    """
    ledger = _ledger(tmp_path)
    strategy = _FakeStrategy(
        targets=(Target.from_weight(IID, 0.005, 1, 2),),
        order_id_sets=(("O-1",),),
    )

    result = submit_once(
        ledger=ledger,
        projection=_Projection(decision=_Decision(DECISION_ID)),
        strategy=strategy,
        base_pipeline=BASE_PIPELINE,
        ceiling=None,
    )

    record = ledger.record(DECISION_ID)
    assert result.outcome is Outcome.SIGNAL
    assert record is not None
    assert record.engine_constructed == 0.5
    assert record.target_id is None


def test_submit_once_refuses_to_convert_a_non_weight_target(tmp_path: Path) -> None:
    """
    A quantity target records nothing and the order is still placed.
    """
    ledger = _ledger(tmp_path)
    strategy = _FakeStrategy(
        targets=(Target.from_quantity(IID, Quantity.from_int(10), 1, 2),),
        order_id_sets=(("O-1",),),
    )

    result = submit_once(
        ledger=ledger,
        projection=_Projection(decision=_Decision(DECISION_ID)),
        strategy=strategy,
        base_pipeline=BASE_PIPELINE,
        ceiling=None,
    )

    record = ledger.record(DECISION_ID)
    assert result.outcome is Outcome.SIGNAL
    assert record is not None
    assert record.engine_constructed is None
    assert record.target_id is None
    assert "engine_constructed" in record.not_reached


def test_the_constructed_weight_and_the_ceiling_agree_when_the_ceiling_binds(
    tmp_path: Path,
) -> None:
    """
    A binding ceiling makes the engine construction equal the bridge's own cap, in percent units.
    """
    ledger = _ledger(tmp_path)
    strategy = _FakeStrategy(
        targets=(Target.from_weight(IID, 0.005, 1, 2),),
        order_id_sets=(("O-1",),),
    )

    submit_once(
        ledger=ledger,
        projection=_Projection(decision=_Decision(DECISION_ID, allocation_pct=0.5)),
        strategy=strategy,
        base_pipeline=BASE_PIPELINE,
        ceiling=Decimal("0.005"),
    )

    record = ledger.record(DECISION_ID)
    assert record is not None
    assert record.research_requested == 0.5
    assert record.bridge_capped == 0.5
    assert record.engine_constructed == 0.5


def test_a_targets_failure_becomes_the_existing_denied_outcome(tmp_path: Path) -> None:
    """
    A construction failure is the same typed denial as a submission failure, not a new mode.
    """
    ledger = _ledger(tmp_path)
    strategy = _FakeStrategy(targets_error=RuntimeError("no price in the construction context"))

    result = submit_once(
        ledger=ledger,
        projection=_Projection(decision=_Decision(DECISION_ID)),
        strategy=strategy,
        base_pipeline=BASE_PIPELINE,
        ceiling=None,
    )

    record = ledger.record(DECISION_ID)
    assert result.outcome is Outcome.DENIED
    assert result.engine_message == "no price in the construction context"
    assert record is not None
    assert record.order_ids == ()


def test_an_already_recorded_construction_is_not_rewritten_by_a_retry(tmp_path: Path) -> None:
    """
    Attribution never changes whether an order is placed: a repeat construction is a no-op.
    """
    ledger = _ledger(tmp_path)
    ledger.record_exposure(DECISION_ID, "engine_constructed", 0.5)
    strategy = _FakeStrategy(
        targets=(Target.from_weight(IID, 0.9, 1, 2),),
        order_id_sets=(("O-1",),),
    )

    result = submit_once(
        ledger=ledger,
        projection=_Projection(decision=_Decision(DECISION_ID)),
        strategy=strategy,
        base_pipeline=BASE_PIPELINE,
        ceiling=None,
    )

    record = ledger.record(DECISION_ID)
    assert result.outcome is Outcome.SIGNAL
    assert record is not None
    assert record.engine_constructed == 0.5
    assert record.order_ids == ("O-1",)


# --- Change 2: the risk engine's answer --------------------------------------------------------


def test_record_engine_event_writes_nothing_for_an_unattributed_order(tmp_path: Path) -> None:
    """
    An order with no decision identity is reported as unattributed and changes no record.
    """
    ledger = _ledger(tmp_path)

    attribution = record_engine_event(
        ledger=ledger,
        client_order_id="O-UNKNOWN",
        reason="VALIDATION_FAILED: detail",
    )

    record = ledger.record(DECISION_ID)
    assert attribution == UNATTRIBUTED
    assert record is not None
    assert record.risk_approved is None


def test_a_denial_records_a_measured_zero(tmp_path: Path) -> None:
    """
    A denial is the risk stage's own answer, recorded as zero rather than left absent.
    """
    ledger = _ledger(tmp_path)
    ledger.record_order(DECISION_ID, "O-1")

    attribution = record_engine_event(
        ledger=ledger,
        client_order_id="O-1",
        reason="NOTIONAL_EXCEEDS_MAX_PER_ORDER: max=1 USD, notional=2 USD",
    )

    record = ledger.record(DECISION_ID)
    assert attribution == DECISION_ID
    assert record is not None
    assert record.risk_approved == 0.0


def test_an_unrecognised_or_empty_denial_still_records_zero(tmp_path: Path) -> None:
    """
    The denial is the event, not its classified token: an unknown or empty token is still a denial.
    """
    ledger = AdmissionLedger(tmp_path)
    cases = (
        ("decision-unknown", "O-UNKNOWN", "NOT_A_KNOWN_CODE: detail"),
        ("decision-empty", "O-EMPTY", ""),
    )
    for decision_id, order_id, reason in cases:
        ledger.admit(
            decision_id,
            artifact_sha256=f"sha-{decision_id}",
            idempotency_key=f"key-{decision_id}",
            admission_timestamp=1,
        )
        ledger.record_exposure(decision_id, "engine_constructed", 0.5)
        ledger.record_order(decision_id, order_id)

        record_engine_event(ledger=ledger, client_order_id=order_id, reason=reason)

        record = ledger.record(decision_id)
        assert record is not None
        assert record.risk_approved == 0.0


def test_a_submitted_order_records_the_constructed_allocation(tmp_path: Path) -> None:
    """
    A non-denial follows a passed gate, so the risk stage copies the constructed allocation.
    """
    ledger = _ledger(tmp_path)
    ledger.record_exposure(DECISION_ID, "engine_constructed", 0.5)
    ledger.record_order(DECISION_ID, "O-1")

    attribution = record_engine_event(
        ledger=ledger,
        client_order_id="O-1",
        reason=None,
    )

    record = ledger.record(DECISION_ID)
    assert attribution == DECISION_ID
    assert record is not None
    assert record.risk_approved == 0.5


def test_a_submitted_order_without_a_construction_writes_nothing(tmp_path: Path) -> None:
    """
    With no constructed allocation to copy, the risk stage stays not reached.
    """
    ledger = _ledger(tmp_path)
    ledger.record_order(DECISION_ID, "O-1")

    record_engine_event(ledger=ledger, client_order_id="O-1", reason=None)

    record = ledger.record(DECISION_ID)
    assert record is not None
    assert record.risk_approved is None
    assert "risk_approved" in record.not_reached


def test_a_retry_success_revises_an_earlier_denial(tmp_path: Path) -> None:
    """
    The risk stage revises its own zero when a later attempt passes the gate.
    """
    ledger = _ledger(tmp_path)
    ledger.record_order(DECISION_ID, "O-1")
    record_engine_event(ledger=ledger, client_order_id="O-1", reason="RATE_LIMIT_EXCEEDED")
    ledger.record_exposure(DECISION_ID, "engine_constructed", 0.5)
    ledger.record_order(DECISION_ID, "O-2")

    record_engine_event(ledger=ledger, client_order_id="O-2", reason=None)

    record = ledger.record(DECISION_ID)
    assert record is not None
    assert record.risk_approved == 0.5


# --- Change 3: deliberate retries ---------------------------------------------------------------


def test_the_retryable_set_is_exactly_the_five_windowed_or_stated_denials() -> None:
    """
    Only the five codes whose condition the engine states clears are retryable.
    """
    expected = {
        "STREAM_RECONCILING",
        "RATE_LIMIT_EXCEEDED",
        "ORDER_COUNT_LIMIT_REACHED",
        "ACTIVE_ORDER_LIMIT_REACHED",
        "REPEATED_REQUEST_LIMIT_REACHED",
    }
    assert expected == RETRYABLE_DENIAL_CODES

    for code in expected:
        assert _permitted_retry(0, code).permitted is True

    for code in ("NOTIONAL_EXCEEDS_MAXIMUM", "VALIDATION_FAILED", "SUBMIT_FAILED"):
        decision = _permitted_retry(0, code)
        assert decision.permitted is False
        assert decision.denial_code == code


def test_classification_reads_the_leading_token_alone() -> None:
    """
    A retryable word in the diagnostic suffix must not make a terminal denial retryable.
    """
    disguised = retry_decision(
        policy=RetryPolicy(max_attempts=5),
        attempts=0,
        denial_code="VALIDATION_FAILED: detail mentions STREAM_RECONCILING",
        attempt="attempt-1",
    )
    prefix = retry_decision(
        policy=RetryPolicy(max_attempts=5),
        attempts=0,
        denial_code="STREAM_RECONCILING: NOTIONAL problems later",
        attempt="attempt-1",
    )

    assert disguised.permitted is False
    assert disguised.denial_code == "VALIDATION_FAILED"
    assert prefix.permitted is True
    assert prefix.denial_code == "STREAM_RECONCILING"


def test_an_unrecognised_token_yields_no_code_and_is_not_retryable() -> None:
    """
    An unrecognised leading token fails closed rather than being treated as transient.
    """
    assert leading_denial_code("not a canonical code") is None
    assert leading_denial_code("") is None
    assert leading_denial_code(None) is None

    decision = retry_decision(
        policy=RetryPolicy(max_attempts=5),
        attempts=0,
        denial_code="not a canonical code",
        attempt="attempt-1",
    )
    assert decision.permitted is False
    assert decision.denial_code is None


def test_the_retry_budget_exhausts_and_the_policy_boundary_is_defined() -> None:
    """
    `max_attempts` is the number of retries permitted, and only a positive budget is a policy.
    """
    policy = RetryPolicy(max_attempts=1)

    assert retry_decision(
        policy=policy,
        attempts=0,
        denial_code="RATE_LIMIT_EXCEEDED",
        attempt="attempt-1",
    ).permitted is True
    assert retry_decision(
        policy=policy,
        attempts=1,
        denial_code="RATE_LIMIT_EXCEEDED",
        attempt="attempt-1",
    ).permitted is False

    with pytest.raises(ValueError, match="at least 1"):
        RetryPolicy(max_attempts=0)
    with pytest.raises(ValueError, match="must be an int"):
        RetryPolicy(max_attempts=True)


def test_no_permission_keeps_the_duplicate_rule(tmp_path: Path) -> None:
    """
    With no retry permission, a decision that already produced an order set is a duplicate.
    """
    ledger = _ledger(tmp_path)
    strategy = _FakeStrategy(
        order_id_sets=(("O-1",), ("O-2",)),
    )
    projection = _Projection(decision=_Decision(DECISION_ID))

    first = submit_once(
        ledger=ledger,
        projection=projection,
        strategy=strategy,
        base_pipeline=BASE_PIPELINE,
        ceiling=None,
    )
    second = submit_once(
        ledger=ledger,
        projection=projection,
        strategy=strategy,
        base_pipeline=BASE_PIPELINE,
        ceiling=None,
    )

    assert first.outcome is Outcome.SIGNAL
    assert second.outcome is Outcome.DUPLICATE
    assert second.client_order_ids == ("O-1",)
    assert strategy.submit_calls == 1


def test_a_permitted_retry_submits_a_fresh_attempt_and_records_it(tmp_path: Path) -> None:
    """
    A permitted retry reaches the engine again and is recorded as an attempt, not a duplicate.
    """
    ledger = _ledger(tmp_path)
    strategy = _FakeStrategy(order_id_sets=(("O-1",), ("O-2",)))
    projection = _Projection(decision=_Decision(DECISION_ID))

    submit_once(
        ledger=ledger,
        projection=projection,
        strategy=strategy,
        base_pipeline=BASE_PIPELINE,
        ceiling=None,
    )

    retry = _permitted_retry(ledger.attempt_count(DECISION_ID))
    assert retry.permitted is True

    result = submit_once(
        ledger=ledger,
        projection=projection,
        strategy=strategy,
        base_pipeline=BASE_PIPELINE,
        ceiling=None,
        retry=retry,
    )

    record = ledger.record(DECISION_ID)
    assert result.outcome is Outcome.SIGNAL
    assert result.client_order_ids == ("O-2",)
    assert strategy.submit_calls == 2
    assert record is not None
    assert record.order_ids == ("O-1", "O-2")
    assert record.attempts == ("attempt-1",)


def test_a_retry_without_a_label_records_a_count_derived_one(tmp_path: Path) -> None:
    """
    When the caller supplies no label, the attempt is named from the recorded count.
    """
    ledger = _ledger(tmp_path)
    strategy = _FakeStrategy(order_id_sets=(("O-1",), ("O-2",)))
    projection = _Projection(decision=_Decision(DECISION_ID))

    submit_once(
        ledger=ledger,
        projection=projection,
        strategy=strategy,
        base_pipeline=BASE_PIPELINE,
        ceiling=None,
    )
    retry = retry_decision(
        policy=RetryPolicy(max_attempts=5),
        attempts=ledger.attempt_count(DECISION_ID),
        denial_code="STREAM_RECONCILING: retry after recovery",
    )
    assert retry.attempt == ""

    submit_once(
        ledger=ledger,
        projection=projection,
        strategy=strategy,
        base_pipeline=BASE_PIPELINE,
        ceiling=None,
        retry=retry,
    )

    record = ledger.record(DECISION_ID)
    assert record is not None
    assert record.attempts == ("attempt-1",)


def test_a_refused_retry_submits_nothing_and_keeps_the_duplicate_outcome(tmp_path: Path) -> None:
    """
    A terminal denial yields no permission, and a refused permission is a duplicate call.
    """
    ledger = _ledger(tmp_path)
    strategy = _FakeStrategy(order_id_sets=(("O-1",), ("O-2",)))
    projection = _Projection(decision=_Decision(DECISION_ID))

    submit_once(
        ledger=ledger,
        projection=projection,
        strategy=strategy,
        base_pipeline=BASE_PIPELINE,
        ceiling=None,
    )

    refused = retry_decision(
        policy=RetryPolicy(max_attempts=5),
        attempts=ledger.attempt_count(DECISION_ID),
        denial_code="NOTIONAL_EXCEEDS_MAXIMUM: max=1 USD",
        attempt="attempt-1",
    )
    assert refused.permitted is False

    result = submit_once(
        ledger=ledger,
        projection=projection,
        strategy=strategy,
        base_pipeline=BASE_PIPELINE,
        ceiling=None,
        retry=refused,
    )

    record = ledger.record(DECISION_ID)
    assert result.outcome is Outcome.DUPLICATE
    assert result.detail == refused.reason
    assert strategy.submit_calls == 1
    assert record is not None
    assert record.order_ids == ("O-1",)
    assert record.attempts == ()


def test_a_stale_permission_is_refused(tmp_path: Path) -> None:
    """
    A permission computed against an older attempt count submits nothing.
    """
    ledger = _ledger(tmp_path)
    strategy = _FakeStrategy(order_id_sets=(("O-1",), ("O-2",)))
    projection = _Projection(decision=_Decision(DECISION_ID))

    submit_once(
        ledger=ledger,
        projection=projection,
        strategy=strategy,
        base_pipeline=BASE_PIPELINE,
        ceiling=None,
    )

    stale = _permitted_retry(ledger.attempt_count(DECISION_ID))
    ledger.record_attempt(DECISION_ID, "attempt-1")

    result = submit_once(
        ledger=ledger,
        projection=projection,
        strategy=strategy,
        base_pipeline=BASE_PIPELINE,
        ceiling=None,
        retry=stale,
    )

    record = ledger.record(DECISION_ID)
    assert result.outcome is Outcome.DUPLICATE
    assert "stale" in result.detail
    assert strategy.submit_calls == 1
    assert record is not None
    assert record.order_ids == ("O-1",)


def test_a_retry_that_fails_again_records_no_attempt(tmp_path: Path) -> None:
    """
    A retry whose submission fails is a denial and consumes no attempt.
    """
    ledger = _ledger(tmp_path)
    strategy = _FakeStrategy(
        order_id_sets=(("O-1",),),
        submit_error_at=2,
    )
    projection = _Projection(decision=_Decision(DECISION_ID))

    submit_once(
        ledger=ledger,
        projection=projection,
        strategy=strategy,
        base_pipeline=BASE_PIPELINE,
        ceiling=None,
    )
    retry = _permitted_retry(ledger.attempt_count(DECISION_ID))
    result = submit_once(
        ledger=ledger,
        projection=projection,
        strategy=strategy,
        base_pipeline=BASE_PIPELINE,
        ceiling=None,
        retry=retry,
    )

    record = ledger.record(DECISION_ID)
    assert result.outcome is Outcome.DENIED
    assert record is not None
    assert record.attempts == ()
    assert record.order_ids == ("O-1",)
