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
The governance fixtures: one campaign per acceptance criterion the milestone owes.

Each test is the fixture the plan names, run as a campaign with the defects its Expected column
states: a campaign with a duplicate, a crash and a refusal (`FX-M4-3`); the ledger protocol under
interruption, concurrency and repetition (`FX-M4-8`); the confirmation state machine's five
negative cases (`FX-M4-5`); the admission assessment's four outcomes (`FX-M4-4`); the calibration
suite over the seven classes of control (`FX-M4-7`, with the planted defects of `FX-M4-2`); the
overfitting matrix's four states (`FX-M4-9`); and the recomputation across the two stores over the
whole envelope (`FX-M4-1`, `FX-M4-6`).
"""

from __future__ import annotations

import threading
from pathlib import Path

import pytest

from nautilus_trader.backtest import CanonicalBacktestResult
from nautilus_trader.discovery.custody import ConfirmationCustody
from nautilus_trader.discovery.custody import CustodyError
from nautilus_trader.discovery.custody import CustodyRefusalCode
from nautilus_trader.discovery.gates import AdmissionAssessment
from nautilus_trader.discovery.gates import AdmissionInputs
from nautilus_trader.discovery.gates import AdmissionOutcome
from nautilus_trader.discovery.gates import CalibrationCase
from nautilus_trader.discovery.gates import ControlClass
from nautilus_trader.discovery.gates import GateEvidence
from nautilus_trader.discovery.gates import GateName
from nautilus_trader.discovery.gates import GatePolicy
from nautilus_trader.discovery.gates import PboRow
from nautilus_trader.discovery.gates import PboState
from nautilus_trader.discovery.gates import assess_admission
from nautilus_trader.discovery.gates import evaluate_pbo_matrix
from nautilus_trader.discovery.gates import run_calibration
from nautilus_trader.discovery.ledger import BUNDLE_SCHEMA
from nautilus_trader.discovery.ledger import AttemptOutcome
from nautilus_trader.discovery.ledger import CampaignInventory
from nautilus_trader.discovery.ledger import DataRole
from nautilus_trader.discovery.ledger import DiscoveryLedger
from nautilus_trader.discovery.ledger import EnvelopeKind
from nautilus_trader.discovery.ledger import EnvelopeReference
from nautilus_trader.discovery.ledger import EvidencePackage
from nautilus_trader.discovery.ledger import IdempotencyKey
from nautilus_trader.discovery.ledger import RegistrationOutcome
from nautilus_trader.discovery.ledger import RegistrationState
from nautilus_trader.discovery.ledger import ReproducibilityEnvelope
from nautilus_trader.discovery.ledger import RunReceipt
from nautilus_trader.discovery.ledger import bundle_identity
from nautilus_trader.discovery.ledger import recompute
from nautilus_trader.discovery.ledger import research_family_id
from nautilus_trader.discovery.ledger import trial_mapping
from nautilus_trader.discovery.ledger import verify_bundle
from nautilus_trader.optimization.persistence import ExperimentStore
from nautilus_trader.optimization.report import ExperimentResult
from nautilus_trader.optimization.runner import CanonicalRun
from nautilus_trader.optimization.significance import TrialDependence
from nautilus_trader.optimization.space import Experiment
from nautilus_trader.optimization.splits import LeakagePolicy
from nautilus_trader.optimization.splits import SplitContract


CANDIDATE = "sha256:" + "a" * 64
OTHER = "sha256:" + "b" * 64
SPECIFICATION = "sha256:" + "c" * 64
COSTS = "sha256:costs"
POLICY = "sha256:policy"
HOLDOUT = "sha256:holdout-evaluation"
CONFIRMATION_SET = "sha256:confirmation-set"
ROOT = "E:/holdout/confirmation"

MANIFEST = {
    "run_id": "run-1",
    "manifest_digest": "sha256:" + "e" * 64,
    "build_identity": "nautilus_trader 2.0.0rc8",
    "configuration_digest": "sha256:" + "f" * 64,
    "seed": 1,
}
# The canonical document the fixture run retained, and the digest the engine's own hashing step
# gives it, so the recomputation verifies content it can read rather than a literal.
CANONICAL_DOCUMENT = b'{"schema":"nautilus.canonical/v1","fixture":"FX-M4-1"}'
ARTIFACT = CanonicalBacktestResult.digest_of_bytes(CANONICAL_DOCUMENT)

SECOND = 1_000_000_000


def series(mean: float, spread: float, count: int = 250) -> dict[int, float]:
    """Return an alternating return series with a known mean and dispersion."""
    return {index: mean + (spread if index % 2 else -spread) for index in range(count)}


def campaign(directory: Path, *, name: str = "campaign-1") -> CampaignInventory:
    """Return a campaign inventory of the fixture research family."""
    return CampaignInventory(
        directory / name,
        writer="orchestrator",
        campaign_id=name,
        family_id=research_family_id(snapshot="snapshot-1", selection_process="rank_first"),
        policy_digest=POLICY,
    )


def receipt() -> RunReceipt:
    """Return the receipt the fixture run emits."""
    return RunReceipt(
        run_id="run-1",
        manifest_digest=MANIFEST["manifest_digest"],
        build_identity=MANIFEST["build_identity"],
        configuration_digest=MANIFEST["configuration_digest"],
        seed=1,
        artifact_reference=f"canonical/{ARTIFACT}",
        artifact_digest=ARTIFACT,
    )


def key(attempt: str = "") -> IdempotencyKey:
    """Return the idempotency key of one attempt of the fixture candidate."""
    return IdempotencyKey.of(CANDIDATE, snapshot="snapshot-1", seed=1, attempt=attempt)


def register(
    inventory: CampaignInventory,
    *,
    candidate: str = CANDIDATE,
    attempt: str = "",
    outcome: AttemptOutcome = AttemptOutcome.COMPLETED,
    artifact_digest: str | None = None,
    role: DataRole = DataRole.LOCKED_VALIDATION,
) -> RegistrationOutcome:
    """Run the registration protocol for one attempt."""
    return inventory.register_attempt(
        key(attempt),
        candidate_id=candidate,
        outcome=outcome,
        receipt=receipt(),
        manifest=MANIFEST,
        artifact_digest=ARTIFACT if artifact_digest is None else artifact_digest,
        data_role=role,
    )


def package(candidate: str = CANDIDATE, *, status: str = "completed") -> EvidencePackage:
    """Return the evidence package one run of the fixture candidate produced."""
    return EvidencePackage(
        candidate_id=candidate,
        specification_digest=SPECIFICATION,
        behavioural_digest=candidate,
        policy_digest=POLICY,
        registry_digest="sha256:0" * 4,
        experiment_digest="sha256:1" * 4,
        canonical_digest=ARTIFACT,
        canonical_bytes=CANONICAL_DOCUMENT,
        metric_values={"Sharpe Ratio": 1.5},
        engine_version=MANIFEST["build_identity"],
        snapshot="snapshot-1",
        seed=1,
        status=status,
    )


def canonical_store(directory: Path) -> ExperimentStore:
    """Return a store that retains the canonical document the fixture run produced."""
    store = ExperimentStore(directory)
    store.write_result(
        ExperimentResult(
            run=CanonicalRun(
                Experiment({"fixture": "FX-M4-1"}),
                ARTIFACT,
                {"Sharpe Ratio": 1.5},
                CANONICAL_DOCUMENT,
            ),
            score=1.5,
            constraints_satisfied=True,
        ),
    )
    return store


def envelope(*, unresolved: bool = False) -> ReproducibilityEnvelope:
    """Return a complete envelope, optionally holding one unresolved reference."""
    references = []
    for kind in EnvelopeKind:
        if kind is EnvelopeKind.EXTERNAL_ARTIFACT:
            continue
        if unresolved and kind is EnvelopeKind.CONFIRMATION_DEFINITION:
            references.append(
                EnvelopeReference(name=kind.value, kind=kind, digest="sha256:" + "9" * 64),
            )
            continue
        references.append(EnvelopeReference.of(kind.value, kind, {"declared": kind.value}))

    return ReproducibilityEnvelope(references=tuple(references))


def test_fx_m4_3_counts_and_lineage_over_a_duplicate_a_crash_and_a_refusal(
    tmp_path: Path,
) -> None:
    """
    `FX-M4-3`: one campaign reports four separate counts and the lineage of what it proposed.

    The campaign proposes twice, refuses one payload, compiles once, executes the same
    configuration twice, crashes once, and admits exactly the verified execution.
    """
    inventory = campaign(tmp_path)
    inventory.record_proposal(CANDIDATE, generation=0, agent_invocation="invocation-1")
    inventory.record_proposal(OTHER, generation=1, agent_invocation="invocation-2")
    inventory.record_refusal(OTHER, code="CODE_REFERENCE_REFUSED", path="entry.conditions.0")
    inventory.record_compilation(
        CANDIDATE,
        specification_digest=SPECIFICATION,
        new_configuration=True,
        parent_id="",
        generation=0,
    )

    first = register(inventory)
    duplicate = register(inventory, attempt="rerun")
    crash = register(inventory, attempt="crashed", outcome=AttemptOutcome.CRASHED)

    assert first.reconciled is True
    assert duplicate.reconciled is True
    assert crash.state is RegistrationState.INCOMPLETE

    counts = inventory.counts()
    assert counts.proposal_attempts == 2
    assert counts.unique_configurations == 1
    assert counts.executed_experiments == 3
    assert counts.statistical_trials == 2

    lineage = inventory.lineage()
    assert len(lineage) == 1
    assert lineage[0].candidate_id == CANDIDATE
    assert lineage[0].agent_invocation == "invocation-1"
    assert lineage[0].family_id == inventory.family_id
    assert lineage[0].campaign_id == inventory.campaign_id

    mapping = trial_mapping(inventory, dependence=TrialDependence.DEPENDENT, effective_trials=2)
    assert mapping.statistical_trials == counts.statistical_trials
    assert mapping.executed_experiments == counts.executed_experiments
    assert mapping.unusable_attempts == 1
    assert inventory.verify_chain(checkpoint=inventory.checkpoint()).ok


def test_fx_m4_8_the_protocol_holds_under_interruption_concurrency_and_repetition(
    tmp_path: Path,
) -> None:
    """
    `FX-M4-8`: injected failures, two concurrent registrations and a tampered receipt.

    No experiment receives an invented return, duplicates are idempotent, incomplete records stay
    incomplete, and reconciliation fails closed when the artifact or the receipt disagrees.
    """
    inventory = campaign(tmp_path)

    # An interruption before registration: the attempt closed, no artifact exists to verify.
    interrupted = register(inventory, attempt="before", artifact_digest="")
    assert interrupted.state is RegistrationState.INCOMPLETE
    assert interrupted.counted is False

    # During persistence: the artifact exists but its digest is not the one the receipt names.
    during = register(inventory, attempt="during", artifact_digest="blake3:" + "2" * 8)
    assert during.state is RegistrationState.INCOMPLETE
    assert "artifact_digest" in (during.reconciliation.mismatched if during.reconciliation else ())

    # Two concurrent registrations of one experiment contend for one key.
    outcomes: list[object] = []
    barrier = threading.Barrier(2)

    def worker() -> None:
        barrier.wait()
        outcomes.append(register(inventory, attempt="concurrent"))

    threads = [threading.Thread(target=worker) for _ in range(2)]
    for thread in threads:
        thread.start()
    for thread in threads:
        thread.join()

    counted = [outcome for outcome in outcomes if getattr(outcome, "counted", False)]
    assert len(counted) == 1

    # A duplicate submission after registration is idempotent rather than additive.
    before = inventory.counts()
    duplicate = register(inventory, attempt="concurrent")
    assert duplicate.idempotent is True
    assert inventory.counts() == before

    # A receipt whose research side moved fails closed.
    tampered = inventory.register_attempt(
        key("tampered"),
        candidate_id=CANDIDATE,
        outcome=AttemptOutcome.COMPLETED,
        receipt=receipt(),
        manifest=dict(MANIFEST, seed=2),
        artifact_digest=ARTIFACT,
    )
    assert tampered.state is RegistrationState.INCOMPLETE
    assert "seed" in tampered.reason

    # Nothing was invented: every admitted trial has a verified receipt, and the counts agree with
    # the mapping that every method reads.
    admitted = inventory.attempts()
    assert all(record.admitted is False or record.receipt_digest for record in admitted.values())
    assert inventory.counts() == trial_mapping(inventory).counts
    assert inventory.verify_chain(checkpoint=inventory.checkpoint()).ok

    # A record removed from the inventory is detectable against a checkpoint held outside it.
    checkpoint = inventory.checkpoint()
    path = inventory.directory / "inventory.jsonl"
    lines = path.read_text(encoding="utf-8").splitlines()
    path.write_text("\n".join(lines[:-1]) + "\n", encoding="utf-8")
    verification = inventory.verify_chain(checkpoint=checkpoint)
    assert verification.ok is False
    assert any("checkpoint" in finding for finding in verification.findings)


def test_fx_m4_5_the_confirmation_set_cannot_influence_selection(tmp_path: Path) -> None:
    """
    `FX-M4-5`: the five negative cases over one campaign's confirmation set.

    A second candidate, a mutation after the freeze, a policy change, a read without a reservation
    and an unlabelled reuse are each refused, and the result the campaign may cite is bound to the
    frozen candidate.
    """
    inventory = campaign(tmp_path)
    custody = ConfirmationCustody(inventory, dataset=CONFIRMATION_SET, root=ROOT)

    with pytest.raises(CustodyError) as searching:
        custody.read_holdout(CANDIDATE, purpose="early")
    assert searching.value.code is CustodyRefusalCode.NOT_FROZEN

    custody.freeze(CANDIDATE, specification_digest=SPECIFICATION, policy_digest=POLICY)

    with pytest.raises(CustodyError) as mutated:
        custody.guard_mutation(CANDIDATE, specification_digest="sha256:mutated")
    assert mutated.value.code is CustodyRefusalCode.FROZEN_CANDIDATE_MUTATED

    with pytest.raises(CustodyError) as policy:
        custody.guard_policy("sha256:other")
    assert policy.value.code is CustodyRefusalCode.POLICY_CHANGED

    with pytest.raises(CustodyError) as second:
        custody.read_holdout(OTHER, purpose="pick a winner")
    assert second.value.code is CustodyRefusalCode.SECOND_CANDIDATE

    access = custody.read_holdout(CANDIDATE, purpose="the campaign's one confirmation")
    assert access.label == "confirmation"

    with pytest.raises(CustodyError) as spent:
        custody.read_holdout(CANDIDATE, purpose="again")
    assert spent.value.code is CustodyRefusalCode.RESERVATION_CONSUMED

    verdict = custody.confirm(
        CANDIDATE,
        policy_digest=POLICY,
        evidence_digest=HOLDOUT,
        summary="the frozen candidate holds on unseen data",
    )
    assert custody.citation(CANDIDATE) == verdict

    with pytest.raises(CustodyError) as replacement:
        custody.citation(OTHER)
    assert replacement.value.code is CustodyRefusalCode.VERDICT_CANDIDATE_MISMATCH

    assert custody.worker_environment("worker-1", {"HOLDOUT": ROOT}).holds is False
    events = [event.payload["transition"] for event in inventory.events()]
    assert events.count("frozen") == 1
    assert events.count("confirmed") == 1


def admission(**overrides: object) -> AdmissionAssessment:
    """Return an admission assessment over the fixture campaign."""
    base: dict[str, object] = {
        "sample_observations": 1500,
        "effective_observations": 1200,
        "label_horizon": 5,
        "planned_configurations": 200,
        "reserved_observations": 120,
        "holdout_fraction": 0.1,
        "target_sharpe": 0.3,
        "cross_trial_dispersion": 0.05,
    }
    base.update(overrides)

    return assess_admission(AdmissionInputs(**base))  # type: ignore[arg-type]


def test_fx_m4_4_an_over_budget_search_is_refused_before_the_first_backtest() -> None:
    """
    `FX-M4-4`: an admitted plan, a narrowed one, a hopeless one and an unestimable input.

    Every outcome carries the formula, its inputs' provenance, the sensitivity of each assumption
    and the claims the assessment can and cannot support, and it answers before any run.
    """
    admitted = admission()
    assert admitted.outcome is AdmissionOutcome.ADMIT
    assert admitted.required_observations is not None
    plan = admitted.inputs["planned_configurations"]
    assert plan["source"] == "campaign plan"  # type: ignore[index]
    sensitivity = admitted.assumptions["target_sharpe"]["sensitivity"]  # type: ignore[index]
    assert sensitivity["measurements"]["lower"] is not None  # type: ignore[index]
    assert any("profitability" in claim for claim in admitted.withholds)
    assert admitted.limitations

    narrowed = admission(target_sharpe=0.16)
    assert narrowed.outcome is AdmissionOutcome.REDUCE
    assert narrowed.reduced_configurations is not None
    assert narrowed.reduced_configurations < 200

    hopeless = admission(effective_observations=20, reserved_observations=0, holdout_fraction=0.0)
    assert hopeless.outcome is AdmissionOutcome.REJECT
    assert "no allocation fits" in hopeless.reason

    unestimable = admission(cross_trial_dispersion=None)
    assert unestimable.outcome is AdmissionOutcome.REJECT
    assert unestimable.required_observations is None
    assert "dispersion" in unestimable.reason


def evidence(**overrides: object) -> GateEvidence:
    """Return evidence for a candidate that passes every gate, with the overrides applied."""
    base: dict[str, object] = {
        "candidate_id": CANDIDATE,
        "specification_digest": SPECIFICATION,
        "policy_digest": POLICY,
        "price_series_statement": "adjusted closes, no distributions",
        "returns": series(0.01, 0.02),
        "benchmark_returns": tuple(series(0.0, 0.004).values()),
        "cost_configuration": COSTS,
        "data_role": DataRole.LOCKED_VALIDATION,
        "split": _split(),
        "fold_returns": tuple([tuple(series(0.01, 0.02, 50).values())] * 4),
        "trial_sharpes": tuple([0.05, -0.05] * 5),
        "statistical_trials": 10,
        "executed_experiments": 10,
        "name_shares": {"A": 0.5, "B": 0.5},
        "sector_shares": {"tech": 0.6, "energy": 0.4},
        "period_shares": {"2024": 0.5, "2025": 0.5},
        "neighbourhood_objectives": (1.0, 0.95, 0.9),
        "stress_returns": {"doubled costs": 0.004, "overnight gaps": 0.005},
        "drawdown": 0.2,
        "confirmation_evaluation": HOLDOUT,
        "frozen_specification_digest": SPECIFICATION,
        "evidence_package_complete": True,
        "human_decision": "promoted 2026-01-01T00:00:00Z",
    }
    base.update(overrides)

    return GateEvidence(**base)  # type: ignore[arg-type]


def _split() -> SplitContract:
    """Return the split contract the walk-forward evaluation declares."""
    return SplitContract(
        sets=("train", "test"),
        lengths={"train": 6 * SECOND, "test": 4 * SECOND},
        leakage=LeakagePolicy(purge_before=SECOND, purge_after=SECOND, embargo_after=SECOND),
    )


def test_fx_m4_7_and_fx_m4_2_the_gate_is_calibrated_on_seven_classes_of_control() -> None:
    """
    `FX-M4-7`, with the planted defects of `FX-M4-2`.

    The seven classes of control are detected by the gate that carries them, and the detected and
    missed sets are reported.

    The planted defects are criterion 5's cases: a candidate that profits only from reading the
    future fails G0, a cost-blind one fails G1, a single-name one fails G4, and one that works at a
    single parameter value fails G4's neighbourhood test.
    """
    null = evidence(
        returns=series(0.0005, 0.01),
        trial_sharpes=tuple([0.08, -0.08] * 100),
        statistical_trials=200,
        executed_experiments=200,
    )
    cases = [
        CalibrationCase(
            name="look_ahead",
            control_class=ControlClass.LEAKAGE,
            evidence=evidence(timing_violations=("close_to_close_settlement",)),
            expectation="detected",
            expected_gate=GateName.G0_DATA_INTEGRITY,
        ),
        CalibrationCase(
            name="cost_blind",
            control_class=ControlClass.COST,
            evidence=evidence(returns=series(-0.002, 0.02)),
            expectation="detected",
            expected_gate=GateName.G1_ECONOMIC_VIABILITY,
        ),
        CalibrationCase(
            name="single_name",
            control_class=ControlClass.CONCENTRATION,
            evidence=evidence(name_shares={"A": 0.9}),
            expectation="detected",
            expected_gate=GateName.G4_ROBUSTNESS,
        ),
        CalibrationCase(
            name="parameter_spike",
            control_class=ControlClass.PARAMETER_SPIKE,
            evidence=evidence(neighbourhood_objectives=(1.0, 0.05, 0.1)),
            expectation="detected",
            expected_gate=GateName.G4_ROBUSTNESS,
        ),
        CalibrationCase(
            name="accounting",
            control_class=ControlClass.ACCOUNTING,
            evidence=evidence(entitlement_defects=("dividend", "split")),
            expectation="detected",
            expected_gate=GateName.G0_DATA_INTEGRITY,
        ),
        CalibrationCase(
            name="null",
            control_class=ControlClass.NULL,
            evidence=null,
            expectation="detected",
            expected_gate=GateName.G3_STATISTICAL_CREDIBILITY,
        ),
        CalibrationCase(
            name="positive",
            control_class=ControlClass.POSITIVE,
            evidence=evidence(),
            expectation="accepted",
        ),
    ]
    report = run_calibration(cases, policy=GatePolicy())

    assert report.calibrated is True
    assert report.detected == (
        "look_ahead",
        "cost_blind",
        "single_name",
        "parameter_spike",
        "accounting",
        "null",
    )
    assert report.missed == ()
    assert report.false_accepts == ()
    assert report.false_rejects == ()
    assert set(report.detected_classes) == {
        "leakage",
        "cost",
        "concentration",
        "parameter_spike",
        "accounting",
        "null",
    }

    # The suite reports a control it fails to detect rather than failing itself.
    missed = [
        *cases[:-1],
        CalibrationCase(
            name="undetected",
            control_class=ControlClass.NULL,
            evidence=evidence(),
            expectation="detected",
            expected_gate=GateName.G3_STATISTICAL_CREDIBILITY,
        ),
    ]
    reported = run_calibration(missed, policy=GatePolicy())
    assert reported.missed == ("undetected",)
    assert reported.calibrated is False


def test_fx_m4_9_the_overfitting_matrix_reports_its_four_states() -> None:
    """
    `FX-M4-9`: a matrix valid with documented exclusions, and one that is not computable.

    The statistic never rejects a candidate; the exclusions are reported beside it, and a matrix
    that fails its preconditions produces no number at all.
    """
    overfit = {index: (0.05 if index < 40 else -0.01) for index in range(80)}
    rows = [
        PboRow("overfit", returns=overfit),
        PboRow("steady", returns=dict.fromkeys(range(80), 0.005)),
        PboRow("ineligible", eligible=False, ineligible_rule="failed the data-integrity gate"),
        PboRow("failed", failure_class="economically_failed"),
    ]
    evaluation = evaluate_pbo_matrix(rows)

    assert evaluation.state is PboState.VALID_WITH_EXCLUSIONS
    assert evaluation.probability is not None
    assert evaluation.configurations == 2
    assert {exclusion.state for exclusion in evaluation.exclusions} == {
        PboState.INELIGIBLE,
        PboState.ELIGIBLE_BUT_FAILED,
    }
    assert evaluation.reason.endswith("exclusions accounted for")

    not_computable = evaluate_pbo_matrix(
        [PboRow("a", returns=series(0.01, 0.02, 6)), PboRow("b", returns=series(0.0, 0.02, 6))],
    )
    assert not_computable.state is PboState.NOT_COMPUTABLE
    assert not_computable.probability is None
    assert "common index" in not_computable.reason or "fewer than" in not_computable.reason


def test_fx_m4_1_and_fx_m4_6_every_number_recomputes_over_the_envelope(tmp_path: Path) -> None:
    """
    `FX-M4-1`, `FX-M4-6`: the numbers recompute from the two stores and the envelope resolves.

    The research entry and the campaign's admitted trial explain each other, the envelope's
    references resolve from retained content, and an entry no admitted trial explains is reported as
    a disagreement rather than resolved in one store's favour.
    """
    inventory = campaign(tmp_path)
    register(inventory)
    ledger = DiscoveryLedger(tmp_path / "ledger")
    ledger.append(package())

    store = canonical_store(tmp_path)
    recomputed = recompute(ledger, inventory=inventory, store=store, envelope=envelope())
    assert recomputed.agrees is True, "every number recomputes and nothing is unavailable"
    assert recomputed.disagreements == ()
    assert recomputed.unresolved == ()
    assert recomputed.counts.statistical_trials == 1
    assert recomputed.unavailable == ()

    # A reference whose content is gone makes the package incomplete rather than passing.
    incomplete = recompute(ledger, inventory=inventory, envelope=envelope(unresolved=True))
    assert incomplete.unresolved == ("confirmation_definition",)

    # An entry no admitted trial explains is a disagreement rather than resolved in one store's
    # favour: the research side holds a package for a candidate the campaign never admitted.
    extra = DiscoveryLedger(tmp_path / "extra-ledger")
    extra.append(package(OTHER))
    mismatched = recompute(extra, inventory=inventory)
    assert mismatched.disagreements
    assert any(name.endswith(OTHER) for name in mismatched.disagreements)
    assert "research_entries" not in mismatched.disagreements

    # The bundle's entries cite receipts the ledger holds, and both are recomputed.
    recorded = receipt()
    components = {"fixture": "FX-M4-6", "test": "tests/x.py", "engine": "build", "data": "snapshot"}
    document = {
        "schema": BUNDLE_SCHEMA,
        "entries": [
            {
                "criterion": 10,
                "requirement": "Every number recomputes.",
                "requirement_source": "design Section 15 criterion 10",
                "fixture": "FX-M4-6",
                "milestone": "M4",
                "status": "passed",
                "identity": {
                    "digest": bundle_identity(components),
                    "components": components,
                },
                "test_or_fixture": "tests/x.py",
                "procedure": "pytest tests/x.py",
                "expected": "the numbers recompute",
                "observed": "they recompute, with nothing unavailable",
                "receipts": {
                    "run": {**recorded.to_dict(), "digest": recorded.digest()},
                    "artifacts": [{"reference": ARTIFACT, "digest": ARTIFACT}],
                    "engine_build": recorded.build_identity,
                    "policy_digest": POLICY,
                },
            },
        ],
    }
    verification = verify_bundle(document, resolve={ARTIFACT: ARTIFACT})
    assert verification.ok is True
    assert verification.resolved == (ARTIFACT,)

    unresolved = verify_bundle(document, resolve={})
    assert unresolved.ok is False
    assert any("retained content" in finding.describe() for finding in unresolved.findings)
