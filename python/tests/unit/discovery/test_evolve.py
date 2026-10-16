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
Tests for the search: the objectives, Pareto selection, deduplication, the policy and a generation.

Selection reads development results only, so the boundary is tested from both sides: a development
result is accepted and a locked or confirmation result is refused, and the same refusal stops a
generation before any child is bred. The equivalence rule is exercised through the two directions it
has to hold at once, a rename and a provenance rewrite that are the same configuration, and a
parameter change that is a different one.
"""

from __future__ import annotations

from pathlib import Path

import pytest

from nautilus_trader.discovery.evolve import EQUIVALENCE_RULE_ID
from nautilus_trader.discovery.evolve import FROZEN_FIELDS
from nautilus_trader.discovery.evolve import DevelopmentResult
from nautilus_trader.discovery.evolve import Evolution
from nautilus_trader.discovery.evolve import GenerationPlan
from nautilus_trader.discovery.evolve import Objective
from nautilus_trader.discovery.evolve import PartitionRole
from nautilus_trader.discovery.evolve import PartitionRoles
from nautilus_trader.discovery.evolve import SearchPolicy
from nautilus_trader.discovery.evolve import SearchRefusal
from nautilus_trader.discovery.evolve import deduplicate
from nautilus_trader.discovery.evolve import dominates
from nautilus_trader.discovery.evolve import equivalence_key
from nautilus_trader.discovery.evolve import pareto_front
from nautilus_trader.discovery.evolve import role_audit
from nautilus_trader.discovery.evolve import selectable
from nautilus_trader.discovery.genome import GeneticDegree
from nautilus_trader.discovery.genome import Genome
from nautilus_trader.discovery.ledger import CampaignInventory
from nautilus_trader.discovery.ledger import DataRole
from nautilus_trader.discovery.ledger import IdempotencyKey
from nautilus_trader.discovery.ledger import research_family_id
from nautilus_trader.discovery.schema import parse_specification
from tests.unit.discovery.fixtures import fixture_policy
from tests.unit.discovery.fixtures import legal_payload


POLICY = fixture_policy()
FAMILY_ID = research_family_id(snapshot="snapshot-1", selection_process="rank_first")
CAMPAIGN_ID = "campaign-1"
PARTITION = "development_folds"
LINEAGE = "lineage-1"
UNKNOWN = "sha256:" + "b" * 64


def base() -> Genome:
    """
    Return the fixture payload's genome under the fixture policy.
    """
    parsed = parse_specification(legal_payload(), POLICY)

    assert parsed.specification is not None

    return Genome(specification=parsed.specification, policy=POLICY)


def rewritten(paths: dict[str, object]) -> Genome:
    """
    Return the fixture candidate with each specification path set to its value.
    """
    candidate = base()

    return candidate.with_paths(list(paths), list(paths.values()))


def objective_values(**overrides: float) -> dict[Objective, float]:
    """
    Return every objective measured at one, with the named ones replaced.
    """
    values = dict.fromkeys(Objective, 1.0)
    values.update({Objective(name): value for name, value in overrides.items()})

    return values


def measured(
    candidate_id: str,
    *,
    digest: str | None = None,
    role: DataRole = DataRole.DEVELOPMENT,
    family: str = "cross_sectional_rank",
    objectives: dict[Objective, float] | None = None,
) -> DevelopmentResult:
    """
    Return a development result for a candidate, with every objective measured unless given.
    """
    return DevelopmentResult(
        candidate_id=candidate_id,
        digest=candidate_id if digest is None else digest,
        family=family,
        partition=PARTITION,
        role=role,
        lineage_id=LINEAGE,
        objectives=objective_values() if objectives is None else objectives,
    )


def generation(
    seed: int,
    *,
    policy: SearchPolicy | None = None,
    population: list[Genome] | None = None,
    results: list[DevelopmentResult] | None = None,
    number: int = 1,
    population_size: int = 4,
) -> GenerationPlan:
    """
    Return the plan one generation breeds under the given policy and seed.
    """
    members = [base()] if population is None else list(population)
    developments = (
        [measured(member.digest()) for member in members] if results is None else list(results)
    )
    evolution = Evolution(
        policy=SearchPolicy() if policy is None else policy,
        seed=seed,
        population_size=population_size,
    )

    return evolution.next_generation(
        population=members,
        results=developments,
        generation=number,
    )


# -------------------------------------------------------------------------------------------------
# Objectives and Pareto selection
# -------------------------------------------------------------------------------------------------


def test_every_objective_carries_its_own_sampling_convention() -> None:
    """
    Seven objectives are named as measurables, each with a convention that distinguishes it.
    """
    conventions = {objective: objective.convention for objective in Objective}

    assert len(Objective) == 7
    assert Objective.RISK_ADJUSTED_RETURN.value == "risk_adjusted_return"
    assert Objective.COMPLEXITY.value == "complexity"
    assert all(conventions.values())
    assert len(set(conventions.values())) == len(Objective)


def test_the_objectives_state_the_direction_they_prefer() -> None:
    """
    Return and tail loss are preferred high, and dispersion, drawdown, turnover, complexity low.
    """
    preferred_high = {objective for objective in Objective if objective.prefers_high}

    assert preferred_high == {
        Objective.RISK_ADJUSTED_RETURN,
        Objective.TAIL_LOSS,
        Objective.RELATIVE_RETURN,
    }
    assert not Objective.RETURN_DISPERSION.prefers_high
    assert not Objective.DRAWDOWN.prefers_high
    assert not Objective.TURNOVER.prefers_high
    assert not Objective.COMPLEXITY.prefers_high


def test_domination_is_no_worse_everywhere_and_better_somewhere() -> None:
    """
    A strictly better result dominates, while a tie and a mixed pair dominate in neither direction.
    """
    better = measured("candidate-a", objectives=objective_values(turnover=0.5))
    reference = measured("candidate-b")
    tie = measured("candidate-c")
    worse = measured("candidate-d", objectives=objective_values(turnover=2.0))
    mixed = measured("candidate-e", objectives=objective_values(drawdown=0.5, turnover=2.0))

    assert dominates(better, reference)
    assert not dominates(reference, better)
    assert not dominates(reference, tie)
    assert not dominates(worse, reference)
    assert not dominates(better, mixed)
    assert not dominates(mixed, better)


def test_a_result_missing_an_objective_is_not_comparable() -> None:
    """
    An objective that was not measured makes a result incomparable in both directions.
    """
    complete = measured("candidate-a")
    partial = measured("candidate-b", objectives={Objective.TURNOVER: 0.5})

    assert complete.complete
    assert partial.complete is False
    assert partial.value(Objective.DRAWDOWN) is None
    assert not dominates(partial, complete)
    assert not dominates(complete, partial)
    assert pareto_front([partial, complete]) == (complete,)


def test_the_front_keeps_a_tie_and_drops_a_dominated_candidate() -> None:
    """
    Two results equal on every objective are both non-dominated, and a worse one is dropped.
    """
    winner = measured("candidate-a", objectives=objective_values(turnover=0.5))
    tie = measured("candidate-b", objectives=objective_values(turnover=0.5))
    dominated = measured("candidate-c", objectives=objective_values(turnover=0.75))

    assert pareto_front([dominated, tie, winner]) == (winner, tie)


def test_selection_reads_development_results_only() -> None:
    """
    Development results are accepted, and a locked or confirmation result is refused by name.
    """
    development = measured("candidate-a")
    accepted = selectable([development])

    assert accepted.usable
    assert accepted.refusal is None
    assert accepted.results == (development,)

    for role in (DataRole.LOCKED_VALIDATION, DataRole.FINAL_CONFIRMATION):
        locked = measured("candidate-b", role=role)
        refused = selectable([development, locked])

        assert refused.usable is False
        assert refused.refusal is SearchRefusal.LOCKED_RESULT_IN_SELECTION
        assert refused.results == ()
        assert "candidate-b" in refused.reason
        assert role.value in refused.reason


# -------------------------------------------------------------------------------------------------
# The equivalence rule
# -------------------------------------------------------------------------------------------------


def test_a_rewritten_statement_and_different_provenance_are_the_same_configuration() -> None:
    """
    Restating the hypothesis or the falsification, or renaming the provenance, is the same candidate.
    """
    original = base()
    renamed = rewritten(
        {
            "identity.hypothesis": "The same behaviour, described in another sentence.",
            "identity.falsification": "Refuted when the same condition does not hold.",
        },
    )
    provenance = rewritten({"provenance.seed": 99, "provenance.generator": "recompiled"})

    assert renamed.digest() != original.digest()
    assert provenance.digest() != original.digest()
    assert equivalence_key(renamed) == equivalence_key(original)
    assert equivalence_key(provenance) == equivalence_key(original)
    assert equivalence_key(renamed) == equivalence_key(provenance)


def test_a_parameter_change_is_a_different_configuration() -> None:
    """
    A changed parameter is a different configuration under the rule, not a rerun of one.
    """
    original = base()
    changed = rewritten({"portfolio.position_count": 7})

    assert changed.digest() != original.digest()
    assert equivalence_key(changed) != equivalence_key(original)


def test_the_first_of_two_equivalent_candidates_survives_and_the_merge_is_recorded() -> None:
    """
    The earlier candidate keeps the configuration, and each later duplicate is one merge into it.
    """
    first = base()
    renamed = rewritten({"identity.hypothesis": "The same behaviour, restated."})
    distinct = rewritten({"portfolio.position_count": 7})
    recompiled = rewritten({"provenance.seed": 42})

    folded = deduplicate([first, renamed, distinct, recompiled])

    assert folded.survivors == (first, distinct)
    assert [merge.candidate_id for merge in folded.merges] == [
        renamed.digest(),
        recompiled.digest(),
    ]
    merge = folded.merges[0]

    assert merge.merged_into == first.digest()
    assert merge.digest == equivalence_key(first)
    assert merge.to_dict()["rule"] == EQUIVALENCE_RULE_ID
    assert merge.to_dict()["digest"] == equivalence_key(first)


def test_computing_an_equivalence_key_does_not_mutate_the_specification() -> None:
    """
    The key is read from the candidate, so computing it leaves the genotype it was read from alone.
    """
    candidate = base()
    before = candidate.specification.to_dict()

    assert equivalence_key(candidate) == equivalence_key(candidate)
    assert candidate.specification.to_dict() == before


# -------------------------------------------------------------------------------------------------
# The search policy
# -------------------------------------------------------------------------------------------------


def test_a_policy_digest_follows_its_content() -> None:
    """
    Equal policies digest equally, and a change to what a generation records moves the digest.
    """
    policy = SearchPolicy()

    assert policy.digest() == SearchPolicy().digest()
    assert SearchPolicy(generation=1).digest() != policy.digest()
    assert SearchPolicy(mutation_rate=0.25).digest() != policy.digest()
    assert SearchPolicy(prompts={"mutation": "sha256:prompt"}).digest() != policy.digest()


def test_the_mutable_set_is_named_and_a_generation_stamp_advances() -> None:
    """
    Only the named fields may change, and the generation is a stamp that is not one of them.
    """
    policy = SearchPolicy()

    assert policy.mutable_fields == (
        "prompts",
        "mutation_rate",
        "exploration_share",
        "family_allocation",
    )
    assert "generation" not in policy.mutable_fields

    amendment = policy.amend(
        mutation_rate=0.25,
        exploration_share=0.5,
        family_allocation={"cross_sectional_rank": 2.0},
        generation=4,
    )

    assert amendment.applied
    assert amendment.policy is not None
    amended = amendment.policy

    assert amended.mutation_rate == 0.25
    assert amended.exploration_share == 0.5
    assert amended.family_allocation == {"cross_sectional_rank": 2.0}
    assert amended.generation == 4
    assert amended.policy_id == policy.policy_id
    assert amended.digest() != policy.digest()


def test_weights_normalise_the_allocation_and_fall_back_evenly() -> None:
    """
    A declared allocation is normalised and a family without one gets nothing.

    No allocation at all is spread evenly rather than divided by zero.
    """
    allocated = SearchPolicy(family_allocation={"alpha": 3.0, "beta": 1.0})

    assert allocated.weights(["beta", "alpha", "gamma"]) == (
        ("alpha", 0.75),
        ("beta", 0.25),
        ("gamma", 0.0),
    )

    undeclared = SearchPolicy()

    assert undeclared.weights(["beta", "alpha"]) == (("alpha", 0.5), ("beta", 0.5))
    assert undeclared.weights(["alpha"]) == (("alpha", 1.0),)
    assert undeclared.weights([]) == ()

    zeroed = SearchPolicy(family_allocation={"alpha": 0.0, "beta": 0.0})

    assert zeroed.weights(["alpha", "beta"]) == (("alpha", 0.5), ("beta", 0.5))


def test_an_amendment_refuses_a_frozen_field_and_an_undeclared_one_by_name() -> None:
    """
    Every frozen object of the design is refused by name, and so is a field it never declared.
    """
    for field, _ in FROZEN_FIELDS:
        refusal = SearchPolicy().amend(**{field: "anything"})

        assert refusal.applied is False
        assert refusal.policy is None
        assert refusal.refused_field == field
        assert "frozen" in refusal.reason

    undeclared = SearchPolicy().amend(version=2)

    assert undeclared.applied is False
    assert undeclared.refused_field == "version"
    assert "mutable" in undeclared.reason


def test_a_policy_refuses_a_rate_that_is_not_a_share_and_a_negative_allocation() -> None:
    """
    A rate outside its range and a negative allocation are refused rather than normalised.
    """
    with pytest.raises(ValueError, match="must be a share"):
        SearchPolicy(mutation_rate=1.5)

    with pytest.raises(ValueError, match="must be a share"):
        SearchPolicy(exploration_share=-0.1)

    with pytest.raises(ValueError, match="must not be negative"):
        SearchPolicy(family_allocation={"alpha": -0.5})


# -------------------------------------------------------------------------------------------------
# The role audit and the partition boundary
# -------------------------------------------------------------------------------------------------


def test_the_role_audit_joins_a_submitted_job_to_its_candidate_lineage(tmp_path: Path) -> None:
    """
    A job's recorded role is joined to the lineage of the candidate it was submitted for.
    """
    candidate = base()
    campaign = CampaignInventory(
        tmp_path / CAMPAIGN_ID,
        writer="orchestrator",
        campaign_id=CAMPAIGN_ID,
        family_id=FAMILY_ID,
    )
    campaign.record_compilation(
        candidate.digest(),
        specification_digest="sha256:specification",
        new_configuration=True,
    )
    campaign.submit_job(
        IdempotencyKey.of(candidate.digest(), snapshot="snapshot-1", seed=1),
        candidate_id=candidate.digest(),
        data_role=DataRole.LOCKED_VALIDATION,
    )
    campaign.submit_job(
        IdempotencyKey.of(UNKNOWN, snapshot="snapshot-1", seed=1),
        candidate_id=UNKNOWN,
    )

    audit = role_audit(campaign)

    assert len(audit) == 2
    assert audit[0].candidate_id == candidate.digest()
    assert audit[0].family_id == FAMILY_ID
    assert audit[0].campaign_id == CAMPAIGN_ID
    assert audit[0].data_role is DataRole.LOCKED_VALIDATION
    assert audit[0].to_dict()["data_role"] == "locked_validation"

    # A job whose candidate has no lineage record carries the default role and no lineage.
    assert audit[1].candidate_id == UNKNOWN
    assert audit[1].family_id == ""
    assert audit[1].campaign_id == ""
    assert audit[1].data_role is DataRole.DEVELOPMENT


def test_an_unseen_partition_holds_its_declared_role() -> None:
    """
    A declared role holds for every lineage that has never consumed a result there.
    """
    roles = PartitionRoles(
        declared={"validation": DataRole.LOCKED_VALIDATION, "holdout": DataRole.DEVELOPMENT},
    )

    assert roles.role_for("validation", "lineage-1") is DataRole.LOCKED_VALIDATION
    assert roles.role_for("validation", "lineage-2") is DataRole.LOCKED_VALIDATION
    assert roles.role_for("holdout", "lineage-1") is DataRole.DEVELOPMENT
    assert roles.role_for("undeclared", "lineage-1") is DataRole.DEVELOPMENT


def test_an_informed_partition_is_downgraded_for_one_lineage_and_refuses_validation() -> None:
    """
    A partition that informed one lineage becomes development data for that lineage alone.

    That lineage cannot then present the partition as locked validation.
    """
    roles = PartitionRoles(declared={"validation": DataRole.LOCKED_VALIDATION})
    informed = roles.informed("validation", "lineage-1", by="sha256:candidate")

    # The recorded roles are replaced rather than rewritten, so the earlier value still holds.
    assert roles.role_for("validation", "lineage-1") is DataRole.LOCKED_VALIDATION
    assert informed.role_for("validation", "lineage-1") is DataRole.DEVELOPMENT
    assert informed.role_for("validation", "lineage-2") is DataRole.LOCKED_VALIDATION

    assignment = informed.assignments[("validation", "lineage-1")]

    assert assignment.reason == (
        "sha256:candidate was evaluated on validation and its result informed the search"
    )
    assert assignment.to_dict()["role"] == "development"

    untouched = informed.as_validation("validation", "lineage-2")

    assert isinstance(untouched, PartitionRole)
    assert untouched.role is DataRole.LOCKED_VALIDATION
    assert informed.as_validation("validation", "lineage-1") is SearchRefusal.ROLE_MISMATCH


# -------------------------------------------------------------------------------------------------
# One generation of evolution
# -------------------------------------------------------------------------------------------------


def test_a_generation_breeds_children_under_its_policy_and_reports_its_front() -> None:
    """
    Children are bred from the survivors, and the policy digest and the front are reported.

    No child is one of the candidates it was bred from.
    """
    candidate = base()
    policy = SearchPolicy()
    plan = generation(3, policy=policy, population_size=4)

    assert plan.produced
    assert plan.refusal is None
    assert plan.policy_digest == policy.digest()
    assert plan.front == (candidate.digest(),)
    assert plan.diagnosis is not None
    assert plan.diagnosis.generation == 1

    digests = [child.digest() for child in plan.population]

    assert digests
    assert len(set(digests)) == len(digests)
    assert candidate.digest() not in digests
    assert all(child.family == candidate.family for child in plan.population)


def test_the_plan_reports_the_degrees_the_generation_exercised() -> None:
    """
    An exploitation-only policy moves parameter genes and an exploring one moves structural genes.
    """
    exploited = generation(5, policy=SearchPolicy(exploration_share=0.0, mutation_rate=1.0))
    explored = generation(5, policy=SearchPolicy(exploration_share=1.0, mutation_rate=1.0))

    assert exploited.produced
    assert explored.produced
    assert exploited.degrees == frozenset({GeneticDegree.PARAMETER})
    assert explored.degrees == frozenset({GeneticDegree.STRUCTURAL})


def test_a_generation_is_deterministic_under_its_seed() -> None:
    """
    The same seed and generation breed the same children.

    Neither the seed nor the generation is interchangeable with another.
    """
    first = generation(3)
    repeated = generation(3)
    other_seed = generation(4)
    other_generation = generation(3, number=2)

    assert [child.digest() for child in first.population] == [
        child.digest() for child in repeated.population
    ]
    assert [child.digest() for child in first.population] != [
        child.digest() for child in other_seed.population
    ]
    assert [child.digest() for child in first.population] != [
        child.digest() for child in other_generation.population
    ]


def test_an_empty_population_is_refused_before_any_breeding() -> None:
    """
    A generation with nothing to breed reports the refusal and the policy it would have run under.
    """
    policy = SearchPolicy()
    plan = generation(1, policy=policy, population=[], results=[])

    assert plan.refusal is SearchRefusal.POPULATION_EMPTY
    assert plan.produced is False
    assert plan.population == ()
    assert plan.edits == ()
    assert plan.front == ()
    assert plan.diagnosis is None
    assert plan.policy_digest == policy.digest()
    assert plan.reason == SearchRefusal.POPULATION_EMPTY.value


def test_a_locked_result_stops_the_generation() -> None:
    """
    A result that is not development data refuses the generation before any breeding.

    The same result does not refuse it once its role is development.
    """
    candidate = base()
    locked = measured(
        "candidate-locked", digest=candidate.digest(), role=DataRole.LOCKED_VALIDATION
    )
    plan = generation(7, population=[candidate], results=[locked])

    assert plan.refusal is SearchRefusal.LOCKED_RESULT_IN_SELECTION
    assert plan.produced is False
    assert plan.population == ()
    assert plan.edits == ()
    assert "candidate-locked" in plan.reason
    assert "development results only" in plan.reason

    developed = measured("candidate-locked", digest=candidate.digest())
    accepted = generation(7, population=[candidate], results=[developed])

    assert accepted.refusal is None
    assert accepted.produced


def test_a_population_whose_results_are_all_incomplete_is_refused() -> None:
    """
    Without one complete development result there is no front and no survivor to breed from.
    """
    candidate = base()
    partial = measured(candidate.digest(), objectives={Objective.TURNOVER: 0.5})
    plan = generation(9, population=[candidate], results=[partial])

    assert plan.refusal is SearchRefusal.MISSING_OBJECTIVE
    assert plan.produced is False
    assert plan.front == ()
    assert plan.diagnosis is not None
    assert plan.diagnosis.statement == plan.reason

    non_finite = measured(
        candidate.digest(),
        objectives=objective_values() | {Objective.DRAWDOWN: float("nan")},
    )
    refused = generation(9, population=[candidate], results=[non_finite])

    assert refused.refusal is SearchRefusal.MISSING_OBJECTIVE
    assert refused.produced is False
