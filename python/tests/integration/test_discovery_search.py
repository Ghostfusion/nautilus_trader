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
The search fixtures: selection isolation, the refusal path, and an inconclusive campaign.

`FX-M5-2` is the design's boundary rule: a partition whose result informed a lineage is development
data for that lineage, so a candidate descended from the one that informed it may not present a
result there as independent confirmation, while the same partition stays locked for a lineage that
never saw it. `FX-M5-3` is the refusal path: an unstructured response, a schema violation and an
insufficient-data outcome are three recorded results, and the campaign continues. `FX-M5-1` is a
campaign against data that cannot support the claim: it ends inconclusive with its reason and its
trial inventory rather than as a best-of-N finding.
"""

from __future__ import annotations

import json
import random
from pathlib import Path

from nautilus_trader.discovery.agents import AgentContext
from nautilus_trader.discovery.agents import AgentOutcome
from nautilus_trader.discovery.agents import AgentRequest
from nautilus_trader.discovery.agents import Language
from nautilus_trader.discovery.agents import ReplayModel
from nautilus_trader.discovery.agents import Role
from nautilus_trader.discovery.agents import invoke
from nautilus_trader.discovery.evolve import DevelopmentResult
from nautilus_trader.discovery.evolve import Objective
from nautilus_trader.discovery.evolve import PartitionRoles
from nautilus_trader.discovery.evolve import SearchRefusal
from nautilus_trader.discovery.evolve import role_audit
from nautilus_trader.discovery.evolve import selectable
from nautilus_trader.discovery.gates import Actor
from nautilus_trader.discovery.gates import CandidateState
from nautilus_trader.discovery.gates import GateEvidence
from nautilus_trader.discovery.gates import GateName
from nautilus_trader.discovery.gates import GatePolicy
from nautilus_trader.discovery.gates import GateService
from nautilus_trader.discovery.gates import GateVerdict
from nautilus_trader.discovery.gates import Lifecycle
from nautilus_trader.discovery.genome import Genome
from nautilus_trader.discovery.genome import genome_of
from nautilus_trader.discovery.genome import mutate
from nautilus_trader.discovery.ledger import AttemptOutcome
from nautilus_trader.discovery.ledger import CampaignInventory
from nautilus_trader.discovery.ledger import DataRole
from nautilus_trader.discovery.ledger import IdempotencyKey
from nautilus_trader.discovery.ledger import RunReceipt
from nautilus_trader.discovery.ledger import recorded_trial_sharpes
from nautilus_trader.discovery.ledger import research_family_id
from nautilus_trader.discovery.ledger import trial_mapping
from nautilus_trader.discovery.schema import parse_specification
from nautilus_trader.optimization.significance import TrialDependence
from nautilus_trader.optimization.splits import LeakagePolicy
from nautilus_trader.optimization.splits import SplitContract
from tests.unit.discovery.fixtures import fixture_policy
from tests.unit.discovery.fixtures import fixture_registry
from tests.unit.discovery.fixtures import legal_payload


POLICY = fixture_policy()
REGISTRY = fixture_registry()

_PARSED = parse_specification(legal_payload(), POLICY)
assert _PARSED.accepted, _PARSED.refusals
assert _PARSED.specification is not None

BASE = genome_of(_PARSED.specification, POLICY)
_DESCENDANT = mutate(BASE, genes=["exit.holding_period_cap"], rng=random.Random(2))  # noqa: S311
assert _DESCENDANT.genome is not None
DESCENDANT = _DESCENDANT.genome

VALIDATION_PARTITION = "validation-2021"
DEVELOPMENT_PARTITION = "development"
OTHER_LINEAGE = "family-never-informed"
POLICY_DIGEST = "sha256:" + "a" * 64
CONFIRMATION_SET = "sha256:" + "b" * 64
COSTS = "sha256:costs"
ARTIFACT = "blake3:" + "1" * 8
SECOND = 1_000_000_000

MANIFEST = {
    "run_id": "run-1",
    "manifest_digest": "sha256:" + "e" * 64,
    "build_identity": "nautilus_trader 2.0.0rc8",
    "configuration_digest": "sha256:" + "f" * 64,
    "seed": 1,
}

CONTEXT = AgentContext(
    model_identifier="operator-model/v3",
    sampling={"temperature": 0.0, "top_p": 1.0},
    knowledge_cutoff="2025-06-30",
    generated_on="2026-10-16",
    period_start="2026-01-02",
)

NOMINAL_OBJECTIVES: dict[Objective, float] = {
    Objective.RISK_ADJUSTED_RETURN: 1.2,
    Objective.RETURN_DISPERSION: 0.2,
    Objective.DRAWDOWN: 0.15,
    Objective.TAIL_LOSS: -0.01,
    Objective.RELATIVE_RETURN: 0.25,
    Objective.TURNOVER: 1.5,
    Objective.COMPLEXITY: 4.0,
}


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
        policy_digest=POLICY_DIGEST,
    )


def receipt() -> RunReceipt:
    """Return the receipt the fixture run emits."""
    return RunReceipt(
        run_id="run-1",
        manifest_digest=str(MANIFEST["manifest_digest"]),
        build_identity=str(MANIFEST["build_identity"]),
        configuration_digest=str(MANIFEST["configuration_digest"]),
        seed=1,
        artifact_reference=f"canonical/{ARTIFACT}",
        artifact_digest=ARTIFACT,
    )


def register(
    inventory: CampaignInventory,
    *,
    candidate: str,
    attempt: str = "",
    outcome: AttemptOutcome = AttemptOutcome.COMPLETED,
    role: DataRole = DataRole.DEVELOPMENT,
) -> None:
    """Run the registration protocol for one attempt of one candidate."""
    inventory.register_attempt(
        IdempotencyKey.of(candidate, snapshot="snapshot-1", seed=1, attempt=attempt),
        candidate_id=candidate,
        outcome=outcome,
        receipt=receipt(),
        manifest=MANIFEST,
        artifact_digest=ARTIFACT,
        data_role=role,
    )


def measure(
    inventory: CampaignInventory,
    candidate: str,
    *,
    attempt: str = "",
    trial_sharpes: tuple[float, ...] = (),
) -> None:
    """Close an attempt, declaring the per-period Sharpe ratios its run measured."""
    key = IdempotencyKey.of(candidate, snapshot="snapshot-1", seed=1, attempt=attempt)
    inventory.submit_job(key, candidate_id=candidate)
    inventory.close_attempt(
        key,
        outcome=AttemptOutcome.COMPLETED,
        trial_sharpes=trial_sharpes,
    )


def compiled(inventory: CampaignInventory, candidate: str, *, parent: str = "") -> None:
    """Record a compilation, which is what puts a candidate in the lineage."""
    inventory.record_compilation(
        candidate,
        specification_digest=candidate,
        new_configuration=True,
        parent_id=parent,
        generation=1 if parent else 0,
    )


def development(
    genome: Genome, *, partition: str, role: DataRole, lineage: str
) -> DevelopmentResult:
    """Return one candidate's development result on one partition."""
    return DevelopmentResult(
        candidate_id=genome.digest(),
        digest=genome.digest(),
        family=genome.family,
        partition=partition,
        role=role,
        lineage_id=lineage,
        objectives=NOMINAL_OBJECTIVES,
    )


def evidence(**overrides: object) -> GateEvidence:
    """Return evidence for a candidate that passes every gate, with the overrides applied."""
    base: dict[str, object] = {
        "candidate_id": BASE.digest(),
        "specification_digest": BASE.digest(),
        "policy_digest": POLICY_DIGEST,
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
        "confirmation_evaluation": CONFIRMATION_SET,
        "frozen_specification_digest": BASE.digest(),
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


def test_fx_m5_2_a_partition_that_informed_a_lineage_is_development_for_it(tmp_path: Path) -> None:
    """
    `FX-M5-2`: a partition that informed a lineage is development data for that lineage.

    Candidate A informs candidate B on the same partition, and the partition stays locked for a
    lineage that never consumed a result there.
    """
    inventory = campaign(tmp_path)
    lineage = inventory.family_id

    compiled(inventory, BASE.digest())
    register(inventory, candidate=BASE.digest(), role=DataRole.LOCKED_VALIDATION)

    roles = PartitionRoles(declared={VALIDATION_PARTITION: DataRole.LOCKED_VALIDATION})
    assert roles.role_for(VALIDATION_PARTITION, lineage) is DataRole.LOCKED_VALIDATION
    assert roles.role_for(VALIDATION_PARTITION, OTHER_LINEAGE) is DataRole.LOCKED_VALIDATION

    informed = roles.informed(VALIDATION_PARTITION, lineage, by=BASE.digest())
    assignment = informed.assignments[(VALIDATION_PARTITION, lineage)]
    assert assignment.role is DataRole.DEVELOPMENT
    assert BASE.digest() in assignment.reason
    assert informed.role_for(VALIDATION_PARTITION, lineage) is DataRole.DEVELOPMENT

    compiled(inventory, DESCENDANT.digest(), parent=BASE.digest())
    register(inventory, candidate=DESCENDANT.digest(), role=DataRole.LOCKED_VALIDATION)

    assert informed.as_validation(VALIDATION_PARTITION, lineage) is SearchRefusal.ROLE_MISMATCH
    assert informed.role_for(VALIDATION_PARTITION, OTHER_LINEAGE) is DataRole.LOCKED_VALIDATION

    locked = development(
        BASE,
        partition=VALIDATION_PARTITION,
        role=DataRole.LOCKED_VALIDATION,
        lineage=lineage,
    )
    refused = selectable([locked])
    assert refused.refusal is SearchRefusal.LOCKED_RESULT_IN_SELECTION
    assert not refused.usable

    released = development(
        DESCENDANT,
        partition=DEVELOPMENT_PARTITION,
        role=DataRole.DEVELOPMENT,
        lineage=lineage,
    )
    selected = selectable([released])
    assert selected.usable
    assert selected.results == (released,)

    audit = role_audit(inventory)
    assert {item.candidate_id for item in audit} == {BASE.digest(), DESCENDANT.digest()}
    assert all(item.family_id == lineage for item in audit)
    assert all(item.campaign_id == inventory.campaign_id for item in audit)
    assert all(item.data_role is DataRole.LOCKED_VALIDATION for item in audit)
    assert all(item.to_dict()["data_role"] == DataRole.LOCKED_VALIDATION.value for item in audit)


def test_fx_m5_3_an_unstructured_response_a_schema_violation_and_an_insufficient_data_reply(
    tmp_path: Path,
) -> None:
    """
    `FX-M5-3`: the three refusal outcomes are recorded results, and the campaign continues.
    """
    inventory = campaign(tmp_path)
    language = Language(policy=POLICY, registry=REGISTRY)
    request = AgentRequest.of(
        Role.STRATEGY_ENGINEER,
        values={
            "families": "cross_sectional_rank, absolute_threshold",
            "features": "ema_trend",
            "bounds": "exit.holding_period_cap in [1, 252]",
            "declared_sets": "selection_modes: rank, threshold",
            "information_set": "the policy's bounds and declared sets; no results",
        },
    )
    model = ReplayModel()

    unstructured = invoke(
        model := model.record(request, "The trend should continue, so buy the strongest names."),
        request,
        context=CONTEXT,
        language=language,
    )
    assert unstructured.outcome is AgentOutcome.UNSTRUCTURED
    assert unstructured.payload is None
    inventory.record_refusal(
        BASE.digest(),
        code=unstructured.outcome.value,
        path="response",
        generation=0,
    )

    broken = legal_payload()
    broken["identity"]["family"] = "invented_family"
    violation = invoke(
        model := model.record(request, json.dumps({"specification": broken})),
        request,
        context=CONTEXT,
        language=language,
    )
    assert violation.outcome is AgentOutcome.SCHEMA_VIOLATION
    assert violation.refusals
    inventory.record_refusal(
        BASE.digest(),
        code=violation.refusals[0].code.value,
        path=violation.refusals[0].path,
        generation=0,
    )

    insufficient = invoke(
        model := model.record(
            request,
            json.dumps({"outcome": "insufficient_data", "reason": "the bound excludes the idea"}),
        ),
        request,
        context=CONTEXT,
        language=language,
    )
    assert insufficient.insufficient
    assert insufficient.reason == "the bound excludes the idea"
    assert insufficient.record.outcome is AgentOutcome.INSUFFICIENT_DATA
    assert not insufficient.accepted

    accepted = invoke(
        model := model.record(request, json.dumps({"specification": legal_payload()})),
        request,
        context=CONTEXT,
        language=language,
    )
    assert accepted.accepted
    invocation = accepted.record.to_dict()["invocation"]
    assert isinstance(invocation, dict)
    inventory.record_proposal(
        BASE.digest(), generation=0, agent_invocation=invocation["input_digest"]
    )
    compiled(inventory, BASE.digest())

    assert inventory.counts().proposal_attempts == 1
    assert inventory.counts().unique_configurations == 1

    replayed = invoke(model, request, context=CONTEXT, language=language)
    assert replayed.accepted
    assert replayed.record.response_digest == accepted.record.response_digest

    for result in (unstructured, violation, insufficient, accepted):
        record = result.record.to_dict()
        assert record["validation_outcome"] == result.outcome.value
        assert record["context"] == CONTEXT.to_dict()
        assert record["invocation"] == request.to_dict()
    assert violation.to_dict()["refusals"] != []


def test_fx_m5_1_a_campaign_against_insufficient_power_ends_inconclusive(tmp_path: Path) -> None:
    """
    `FX-M5-1`: the campaign ends with an inconclusive verdict, its reason, and its trial inventory.
    """
    inventory = campaign(tmp_path)
    compiled(inventory, BASE.digest())
    measure(inventory, BASE.digest(), trial_sharpes=(0.01, -0.01, 0.02))
    register(inventory, candidate=BASE.digest())
    register(inventory, candidate=BASE.digest(), attempt="crash", outcome=AttemptOutcome.CRASHED)

    short = series(0.01, 0.002, count=12)
    report = GateService(GatePolicy(), inventory=inventory).evaluate(
        evidence(
            returns=short,
            fold_returns=tuple([tuple(series(0.01, 0.002, 3).values())] * 4),
            trial_sharpes=recorded_trial_sharpes(inventory),
            statistical_trials=1,
            executed_experiments=2,
        ),
    )

    assert report.passed is False
    assert report.stopped_at is GateName.G3_STATISTICAL_CREDIBILITY
    assert report.verdict_for(GateName.G3_STATISTICAL_CREDIBILITY) is not None
    assert report.outcome is GateVerdict.INCONCLUSIVE

    verdict = report.verdicts[-1]
    assert verdict.verdict is GateVerdict.INCONCLUSIVE
    assert verdict.reason != ""
    assert report.to_dict()["outcome"] == GateVerdict.INCONCLUSIVE.value

    lifecycle = Lifecycle(BASE.digest(), inventory=inventory)
    lifecycle.apply(report)
    assert lifecycle.state is CandidateState.PROPOSED
    assert lifecycle.outcomes[-1].verdict is GateVerdict.INCONCLUSIVE
    assert lifecycle.outcomes[-1].gate is GateName.G3_STATISTICAL_CREDIBILITY
    assert lifecycle.outcomes[-1].reason == verdict.reason
    assert not lifecycle.permits(CandidateState.VALIDATED, actor=Actor.HUMAN)

    assert recorded_trial_sharpes(inventory) == (0.01, -0.01, 0.02)
    assert inventory.attempts()[
        IdempotencyKey.of(BASE.digest(), snapshot="snapshot-1", seed=1).value
    ].trial_sharpes == (0.01, -0.01, 0.02)

    mapping = trial_mapping(inventory, dependence=TrialDependence.INDEPENDENT)
    assert mapping.counts.executed_experiments == 2
    assert mapping.counts.statistical_trials == 1
    assert mapping.unusable_attempts == 1
    assert mapping.executed_experiments == 2
    assert len(mapping.admitted) == 1
    assert len(mapping.unusable) == 1

    recorded = [
        event.payload.get("gate")
        for event in inventory.events()
        if event.payload.get("gate") is not None
    ]
    assert recorded[-1] == GateName.G3_STATISTICAL_CREDIBILITY.value
    assert GateName.G4_ROBUSTNESS.value not in recorded
    assert inventory.verify_chain(checkpoint=inventory.checkpoint()).ok
