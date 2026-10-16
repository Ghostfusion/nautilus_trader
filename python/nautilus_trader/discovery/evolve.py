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
The search: objectives, Pareto selection, deduplication, and the search policy's boundary.

Selection is Pareto over named objectives rather than a weighted sum, and it reads development
results only. That second half is not a preference: an out-of-sample fold the evolutionary loop
observes and optimises against is development data in all but name, so a result carries the role of
the partition it came from together with the lineage that consumed it, and selection refuses a
result whose role is anything other than development. `PartitionRoles` is what makes the boundary
survivable across generations and branches: once a partition's result has informed a lineage, that
partition is development data for that lineage, and no candidate descended from it may present a
result there as independent confirmation. The same partition stays locked for a lineage that has
never seen it.

The objectives are named as specific metrics with their sampling conventions rather than as summary
phrases, because "better" is not a measurable quantity. A candidate whose run produced no usable
return series has no objectives at all and is therefore not selected against; it stays in the
statistical inventory, where the multiplicity it contributes is counted.

Two quantities of the design are implemented here rather than argued. The equivalence rule is
pre-registered: it names which specification differences are a different configuration and which are
the same configuration under a different name, so that a rerun, a rename or a recompilation cannot
become several independent trials, and a genuinely distinct configuration cannot disappear from the
inventory. And the search policy names its mutable set: prompts, the mutation and recombination
operators, the exploration and exploitation balance, and the allocation of effort across families
and stages may change within a campaign and are versioned per generation, while every frozen object
of the design is refused by name rather than by convention.

The module is pure where the repository's existing search strategies are pure: `next_generation`
takes a population and the development results, draws from a seeded generator, and returns the plan
for the next generation. It never runs a backtest, never sees a result the caller did not hand it,
and is deterministic under its seed.
"""

from __future__ import annotations

import math
import random
from dataclasses import dataclass
from dataclasses import field
from dataclasses import replace
from enum import Enum
from typing import TYPE_CHECKING

from nautilus_trader.discovery.genome import Edit
from nautilus_trader.discovery.genome import GeneticDegree
from nautilus_trader.discovery.genome import mutate
from nautilus_trader.discovery.genome import recombine
from nautilus_trader.discovery.ledger import CampaignInventory
from nautilus_trader.discovery.ledger import DataRole
from nautilus_trader.discovery.ledger import EventKind
from nautilus_trader.discovery.schema import canonical_mapping
from nautilus_trader.optimization.space import JsonValue
from nautilus_trader.optimization.space import digest_of


if TYPE_CHECKING:
    from collections.abc import Mapping
    from collections.abc import Sequence

    from nautilus_trader.discovery.genome import Genome


EVOLUTION_VERSION = "discovery.evolve/v1"
SEARCH_POLICY_ID = "discovery.search_policy"
SEARCH_POLICY_VERSION = 1
EQUIVALENCE_RULE_ID = "discovery.equivalence/v1"

# The generation a policy applies to is a stamp rather than an instruction, so it advances every
# generation without being part of the mutable set the design names.
_GENERATION_STAMP = "generation"

# A generation breeds this many genes per parameter mutation, drawn from the genome's parameter
# genes, and this many for a structural mutation, drawn from its structural genes.
_PARAMETER_GENES_PER_MUTATION = 2
_STRUCTURAL_GENES_PER_MUTATION = 1

# The share of children that explore rather than exploit, before the search policy overrides it.
_EXPLORATION_SHARE = 0.25

# A mutation that could not produce a genome is retried this many times before the plan reports the
# refusal, because a drawn gene may be one whose only alternative is not admissible.
_EDIT_ATTEMPTS = 8


class Objective(Enum):
    """
    The named objectives selection ranks candidates by, each with its sampling convention.

    The convention travels with the name because a risk-adjusted return at a daily frequency and the
    same number at a monthly one are different quantities, and an objective whose convention is
    unstated cannot be compared with anything.
    """

    RISK_ADJUSTED_RETURN = "risk_adjusted_return"
    RETURN_DISPERSION = "return_dispersion"
    DRAWDOWN = "drawdown"
    TAIL_LOSS = "tail_loss"
    RELATIVE_RETURN = "relative_return"
    TURNOVER = "turnover"
    COMPLEXITY = "complexity"

    @property
    def convention(self) -> str:
        """
        The sampling convention the objective is measured under.

        Returns
        -------
        str

        """
        return _CONVENTIONS[self]

    @property
    def prefers_high(self) -> bool:
        """
        Whether a higher value is preferable.

        Returns
        -------
        bool

        """
        return self in _PREFERS_HIGH


_CONVENTIONS: Mapping[Objective, str] = {
    Objective.RISK_ADJUSTED_RETURN: (
        "the per-period Sharpe ratio of the development folds' return series at the declared "
        "signal frequency"
    ),
    Objective.RETURN_DISPERSION: (
        "the standard deviation of the per-fold risk-adjusted return across the development folds"
    ),
    Objective.DRAWDOWN: "the maximum drawdown of the development window",
    Objective.TAIL_LOSS: "the fifth percentile of the development per-period returns",
    Objective.RELATIVE_RETURN: (
        "the development return less the registered benchmark's over the same window"
    ),
    Objective.TURNOVER: "turnover under the policy's declared cost model",
    Objective.COMPLEXITY: "the condition-leaf count plus the declared parameter count",
}

_PREFERS_HIGH: frozenset[Objective] = frozenset(
    {
        Objective.RISK_ADJUSTED_RETURN,
        Objective.TAIL_LOSS,
        Objective.RELATIVE_RETURN,
    },
)

# The design's frozen objects, named so that a change to one is refused by name rather than by
# convention. Each entry is (field, why it is frozen).
FROZEN_FIELDS: tuple[tuple[str, str], ...] = (
    ("feature_allowlist", "the allowed feature set is frozen before a campaign opens"),
    ("bounds", "a parameter bound is the space the search moves inside, not part of the search"),
    ("evaluation_window", "the evaluation window is fixed and versioned before a campaign runs"),
    ("split_layout", "the split layout and its purge and embargo values are frozen"),
    ("cost_model", "the cost model and its parameters are frozen"),
    ("benchmark", "the benchmark is frozen"),
    ("risk_policy", "the risk policy and its limits are frozen"),
    ("confirmation_allocation", "the allocation of the confirmation set is frozen"),
)

# The specification differences that do not make a different configuration. A renamed hypothesis, a
# rewritten falsification statement, and provenance do not change what is being evaluated, so a
# rerun or a recompilation of the same behaviour is the same configuration under the rule.
EQUIVALENCE_IGNORED_PATHS: tuple[str, ...] = (
    "identity.hypothesis",
    "identity.falsification",
    "provenance.parents",
    "provenance.generator",
    "provenance.seed",
)


class SearchRefusal(Enum):
    """
    The vocabulary a search-level refusal is reported with.

    A candidate-level refusal stays a `RefusalCode` and a genotype-level one stays a
    `GenomeRefusal`; a refusal reported here is about the search's own rules: a result whose role
    does not match the stage that asked for it, an objective that was not declared, or a child the
    operators could not produce.
    """

    LOCKED_RESULT_IN_SELECTION = "locked_result_in_selection"
    ROLE_MISMATCH = "role_mismatch"
    UNKNOWN_OBJECTIVE = "unknown_objective"
    UNKNOWN_CANDIDATE = "unknown_candidate"
    MISSING_OBJECTIVE = "missing_objective"
    POPULATION_EMPTY = "population_empty"
    EDIT_REFUSED = "edit_refused"
    FROZEN_FIELD = "frozen_field"


@dataclass(frozen=True)
class DevelopmentResult:
    """
    One candidate's measured objectives on one partition, with the role and the lineage.

    Parameters
    ----------
    candidate_id : str
        The candidate's identity.
    digest : str
        The genome digest the result was measured for.
    family : str
        The declared family the candidate belongs to.
    partition : str
        The partition the result was measured on.
    role : DataRole
        The role of the partition for the consuming lineage.
    lineage_id : str
        The lineage that consumed the result, which is the family identity above the campaign.
    objectives : Mapping[Objective, float]
        The measured objectives, each with the convention its name carries.

    """

    candidate_id: str
    digest: str
    family: str
    partition: str
    role: DataRole
    lineage_id: str
    objectives: Mapping[Objective, float] = field(default_factory=dict)

    @property
    def complete(self) -> bool:
        """
        Whether every objective was measured, which is what selection requires.

        Returns
        -------
        bool

        """
        return all(
            objective in self.objectives and math.isfinite(self.objectives[objective])
            for objective in Objective
        )

    def value(self, objective: Objective) -> float | None:
        """
        Return one objective's value, or nothing when it was not measured.

        Parameters
        ----------
        objective : Objective
            The objective.

        Returns
        -------
        float | None

        """
        return self.objectives.get(objective)

    def to_dict(self) -> dict[str, object]:
        """
        Return the result as a serialisable mapping.

        Returns
        -------
        dict[str, object]

        """
        return {
            "candidate_id": self.candidate_id,
            "digest": self.digest,
            "family": self.family,
            "partition": self.partition,
            "role": self.role.value,
            "lineage_id": self.lineage_id,
            "objectives": {
                objective.value: self.objectives[objective]
                for objective in Objective
                if objective in self.objectives
            },
        }


@dataclass(frozen=True)
class Selection:
    """
    The outcome of applying the selection rule to a set of results.

    Parameters
    ----------
    results : tuple[DevelopmentResult, ...]
        The results selection may read.
    refusal : SearchRefusal | None
        The refusal that stopped selection, when one did.
    reason : str
        Why selection stopped, empty when it did not.

    """

    results: tuple[DevelopmentResult, ...] = ()
    refusal: SearchRefusal | None = None
    reason: str = ""

    @property
    def usable(self) -> bool:
        """
        Whether selection may proceed.

        Returns
        -------
        bool

        """
        return self.refusal is None


@dataclass(frozen=True)
class EquivalenceMerge:
    """
    One candidate recorded as the same configuration as another.

    Parameters
    ----------
    candidate_id : str
        The candidate that was merged away.
    merged_into : str
        The candidate it is the same configuration as.
    digest : str
        The equivalence key both candidates share.

    """

    candidate_id: str
    merged_into: str
    digest: str

    def to_dict(self) -> dict[str, object]:
        """
        Return the merge as a serialisable mapping.

        Returns
        -------
        dict[str, object]

        """
        return {
            "candidate_id": self.candidate_id,
            "merged_into": self.merged_into,
            "rule": EQUIVALENCE_RULE_ID,
            "digest": self.digest,
        }


@dataclass(frozen=True)
class Deduplication:
    """
    The survivors of an equivalence pass and the merges it recorded.

    Parameters
    ----------
    survivors : tuple[Genome, ...]
        The candidates that are distinct configurations, in the order they were given.
    merges : tuple[EquivalenceMerge, ...]
        The candidates that were the same configuration as an earlier one.

    """

    survivors: tuple[Genome, ...] = ()
    merges: tuple[EquivalenceMerge, ...] = ()


@dataclass(frozen=True)
class SearchDiagnosis:
    """
    What the search believes about its own behaviour, as a recorded artifact.

    The orchestrator may carry a diagnosis forward to parameterise the next generation's
    exploration, and it may not carry forward a change to any frozen object, so the diagnosis is a
    versioned record rather than a hidden state.

    Parameters
    ----------
    generation : int
        The generation the diagnosis was formed at.
    family_shares : Mapping[str, float]
        The share of selected candidates each family holds.
    turnover_median : float
        The median turnover of the selected candidates under the cost model.
    statement : str
        The diagnosis in words, which is what a reader is shown.

    """

    generation: int
    family_shares: Mapping[str, float] = field(default_factory=dict)
    turnover_median: float = 0.0
    statement: str = ""

    def to_dict(self) -> dict[str, object]:
        """
        Return the diagnosis as a serialisable mapping.

        Returns
        -------
        dict[str, object]

        """
        return {
            "generation": self.generation,
            "family_shares": dict(self.family_shares),
            "turnover_median": self.turnover_median,
            "statement": self.statement,
        }


@dataclass(frozen=True)
class SearchPolicy:
    """
    The instructions that generate candidates, of which only part may change within a campaign.

    The mutable set is named: the prompts, the mutation and recombination operators, the
    exploration and exploitation balance, and the allocation of effort across families and stages.
    Everything the design freezes is refused by name in `amend`, so a policy change cannot quietly
    move a bound, a window, a cost model, a benchmark, the risk policy or the allocation of the
    confirmation set.

    Parameters
    ----------
    prompts : Mapping[str, str]
        Prompt identifier to the version digest in force.
    mutation_rate : float
        The share of children bred by mutation rather than recombination.
    exploration_share : float
        The share of children that explore structurally rather than exploit a parameter.
    family_allocation : Mapping[str, float]
        The share of effort allocated to each family, normalised by `weights`.
    generation : int
        The generation the policy applies to.
    policy_id : str
        The policy's identifier.
    version : int
        The policy's version.

    """

    prompts: Mapping[str, str] = field(default_factory=dict)
    mutation_rate: float = 0.5
    exploration_share: float = _EXPLORATION_SHARE
    family_allocation: Mapping[str, float] = field(default_factory=dict)
    generation: int = 0
    policy_id: str = SEARCH_POLICY_ID
    version: int = SEARCH_POLICY_VERSION

    def __post_init__(self) -> None:
        """
        Refuse a policy whose rates are not shares or whose allocation is negative.
        """
        for name, share in (
            ("mutation_rate", self.mutation_rate),
            ("exploration_share", self.exploration_share),
        ):
            if not 0.0 <= share <= 1.0:
                raise ValueError(f"{name} must be a share, was {share}")
        for family, share in self.family_allocation.items():
            if share < 0.0:
                raise ValueError(f"the allocation for {family} must not be negative")

    @property
    def mutable_fields(self) -> tuple[str, ...]:
        """
        The fields a policy may change within a campaign.

        Returns
        -------
        tuple[str, ...]

        """
        return ("prompts", "mutation_rate", "exploration_share", "family_allocation")

    def to_dict(self) -> dict[str, object]:
        """
        Return the policy as a serialisable mapping.

        Returns
        -------
        dict[str, object]

        """
        return {
            "policy_id": self.policy_id,
            "version": self.version,
            "generation": self.generation,
            "mutable_fields": list(self.mutable_fields),
            "prompts": dict(sorted(self.prompts.items())),
            "mutation_rate": self.mutation_rate,
            "exploration_share": self.exploration_share,
            "family_allocation": dict(sorted(self.family_allocation.items())),
        }

    def digest(self) -> str:
        """
        Return the policy's canonical digest, which is what a generation records.

        Returns
        -------
        str

        """
        return digest_of(canonical_mapping(self.to_dict()))

    def weights(self, families: Sequence[str]) -> tuple[tuple[str, float], ...]:
        """
        Return the effort weights over a set of families, defaulting to an even allocation.

        Parameters
        ----------
        families : Sequence[str]
            The families present in the population.

        Returns
        -------
        tuple[tuple[str, float], ...]

        """
        declared = [
            (family, self.family_allocation.get(family, 0.0)) for family in sorted(set(families))
        ]
        total = sum(weight for _, weight in declared)
        if total <= 0.0:
            share = 1.0 / len(declared) if declared else 0.0
            return tuple((family, share) for family, _ in declared)
        return tuple((family, weight / total) for family, weight in declared)

    def amend(self, **changes: object) -> Amendment:
        """
        Return an amended policy, or the refusal that names the frozen field.

        Parameters
        ----------
        **changes : object
            The fields to change.

        Returns
        -------
        Amendment

        """
        frozen = dict(FROZEN_FIELDS)
        stamped = dict(changes)
        generation = int(stamped.pop(_GENERATION_STAMP, self.generation))
        for name in stamped:
            if name in frozen:
                return Amendment(
                    refused_field=name,
                    reason=f"{name} is frozen: {frozen[name]}",
                )
            if name not in self.mutable_fields:
                return Amendment(
                    refused_field=name,
                    reason=f"{name} is not part of the search policy's mutable set",
                )

        return Amendment(
            policy=replace(self, generation=generation, **stamped),
            refused_field="",
            reason="",
        )


@dataclass(frozen=True)
class Amendment:
    """
    The outcome of amending a search policy: an amended policy, or the refusal that stopped it.

    Parameters
    ----------
    policy : SearchPolicy | None
        The amended policy, or nothing when the amendment was refused.
    refused_field : str
        The field that was refused, empty when the amendment applied.
    reason : str
        Why the amendment was refused, empty when it applied.

    """

    policy: SearchPolicy | None = None
    refused_field: str = ""
    reason: str = ""

    @property
    def applied(self) -> bool:
        """
        Whether the amendment applied.

        Returns
        -------
        bool

        """
        return self.policy is not None


@dataclass(frozen=True)
class GenerationPlan:
    """
    The plan for one generation: the children, the merges, the diagnosis and any refusal.

    Parameters
    ----------
    generation : int
        The generation the plan is for.
    policy_digest : str
        The digest of the search policy the generation ran under.
    population : tuple[Genome, ...]
        The children the operators produced, after deduplication.
    edits : tuple[Edit, ...]
        Every operation attempted, including the ones that were refused.
    front : tuple[str, ...]
        The candidate identities on the Pareto front, ordered.
    merges : tuple[EquivalenceMerge, ...]
        The children that were the same configuration as an existing candidate.
    diagnosis : SearchDiagnosis
        What the search believes about its own behaviour at this generation.
    refusal : SearchRefusal | None
        The refusal that stopped the generation, when one did.
    reason : str
        Why the generation stopped, empty when it produced children.

    """

    generation: int
    policy_digest: str
    population: tuple[Genome, ...] = ()
    edits: tuple[Edit, ...] = ()
    front: tuple[str, ...] = ()
    merges: tuple[EquivalenceMerge, ...] = ()
    diagnosis: SearchDiagnosis | None = None
    refusal: SearchRefusal | None = None
    reason: str = ""

    @property
    def produced(self) -> bool:
        """
        Whether the generation produced children.

        Returns
        -------
        bool

        """
        return bool(self.population)

    @property
    def refusals(self) -> tuple[Edit, ...]:
        """
        The attempted operations that were refused.

        Returns
        -------
        tuple[Edit, ...]

        """
        return tuple(edit for edit in self.edits if not edit.produced)

    @property
    def degrees(self) -> frozenset[GeneticDegree]:
        """
        The degrees of freedom this generation exercised.

        Returns
        -------
        frozenset[GeneticDegree]

        """
        return frozenset(degree for edit in self.edits for degree in edit.degrees)

    def to_dict(self) -> dict[str, object]:
        """
        Return the plan as a serialisable mapping.

        Returns
        -------
        dict[str, object]

        """
        return {
            "schema": EVOLUTION_VERSION,
            "generation": self.generation,
            "policy_digest": self.policy_digest,
            "population": [genome.digest() for genome in self.population],
            "edits": [edit.to_dict() for edit in self.edits],
            "front": list(self.front),
            "merges": [merge.to_dict() for merge in self.merges],
            "diagnosis": None if self.diagnosis is None else self.diagnosis.to_dict(),
            "refusal": None if self.refusal is None else self.refusal.value,
            "reason": self.reason,
        }


def dominates(first: DevelopmentResult, second: DevelopmentResult) -> bool:
    """
    Whether the first result dominates the second under the named objectives.

    Parameters
    ----------
    first : DevelopmentResult
        The candidate that may dominate.
    second : DevelopmentResult
        The candidate that may be dominated.

    Returns
    -------
    bool

    """
    better = False
    for objective in Objective:
        left = first.value(objective)
        right = second.value(objective)
        if left is None or right is None:
            return False
        if objective.prefers_high:
            if left < right:
                return False
            better = better or left > right
        else:
            if left > right:
                return False
            better = better or left < right
    return better


def pareto_front(results: Sequence[DevelopmentResult]) -> tuple[DevelopmentResult, ...]:
    """
    Return the non-dominated results, ordered by candidate identity.

    A result missing an objective is not comparable and is left out of the front; it stays in the
    inventory, where its multiplicity is counted, rather than being selected against.

    Parameters
    ----------
    results : Sequence[DevelopmentResult]
        The results to rank.

    Returns
    -------
    tuple[DevelopmentResult, ...]

    """
    comparable = [result for result in results if result.complete]
    front = [
        candidate
        for candidate in comparable
        if not any(
            dominates(other, candidate)
            for other in comparable
            if other.candidate_id != candidate.candidate_id
        )
    ]
    return tuple(sorted(front, key=lambda result: result.candidate_id))


def equivalence_key(genome: Genome) -> str:
    """
    Return the equivalence key the pre-registered rule gives a candidate.

    Parameters
    ----------
    genome : Genome
        The candidate.

    Returns
    -------
    str

    """
    payload = canonical_mapping(genome.specification.to_dict())
    for path in EQUIVALENCE_IGNORED_PATHS:
        _drop(payload, path)
    return digest_of(payload)


def deduplicate(genomes: Sequence[Genome]) -> Deduplication:
    """
    Fold candidates that are the same configuration under the pre-registered rule.

    The first candidate carrying a key survives and the later ones are recorded as merges, so a
    duplicate is recorded once as a configuration rather than counted repeatedly, and no genuinely
    distinct configuration disappears.

    Parameters
    ----------
    genomes : Sequence[Genome]
        The candidates to fold, in the order they were proposed.

    Returns
    -------
    Deduplication

    """
    survivors: list[Genome] = []
    merges: list[EquivalenceMerge] = []
    first_by_key: dict[str, str] = {}
    for genome in genomes:
        digest = genome.digest()
        key = equivalence_key(genome)
        holder = first_by_key.get(key)
        if holder is None:
            first_by_key[key] = digest
            survivors.append(genome)
            continue
        merges.append(EquivalenceMerge(candidate_id=digest, merged_into=holder, digest=key))
    return Deduplication(survivors=tuple(survivors), merges=tuple(merges))


def selectable(results: Sequence[DevelopmentResult]) -> Selection:
    """
    Return the results selection may read, refusing when one is not development data.

    Parameters
    ----------
    results : Sequence[DevelopmentResult]
        The results offered to selection.

    Returns
    -------
    Selection

    """
    locked = [result for result in results if result.role is not DataRole.DEVELOPMENT]
    if locked:
        named = ", ".join(sorted(result.candidate_id for result in locked))
        return Selection(
            refusal=SearchRefusal.LOCKED_RESULT_IN_SELECTION,
            reason=(
                f"{named} carries {locked[0].role.value} data, and selection reads development "
                "results only"
            ),
        )
    return Selection(results=tuple(results))


@dataclass(frozen=True)
class RoleAssignment:
    """
    One candidate's recorded data role, together with the lineage that record belongs to.

    Parameters
    ----------
    candidate_id : str
        The candidate the job was submitted for.
    family_id : str
        The lineage the candidate belongs to, empty when the ledger holds no lineage record.
    campaign_id : str
        The campaign the candidate belongs to, empty when the ledger holds no lineage record.
    data_role : DataRole
        The role the result was submitted under.

    """

    candidate_id: str
    family_id: str
    campaign_id: str
    data_role: DataRole

    def to_dict(self) -> dict[str, object]:
        """
        Return the assignment as a serialisable mapping.

        Returns
        -------
        dict[str, object]

        """
        return {
            "candidate_id": self.candidate_id,
            "family_id": self.family_id,
            "campaign_id": self.campaign_id,
            "data_role": self.data_role.value,
        }


def role_audit(inventory: CampaignInventory) -> tuple[RoleAssignment, ...]:
    """
    Return the data role each submitted job carries, joined to the lineage of its candidate.

    The ledger records a role per job and a lineage per candidate, and the pair is what the design's
    boundary rule needs: a partition whose result influenced a lineage is development data for that
    lineage, and the record has to name both halves.

    Parameters
    ----------
    inventory : CampaignInventory
        The campaign's inventory.

    Returns
    -------
    tuple[RoleAssignment, ...]

    """
    lineages = {record.candidate_id: record for record in inventory.lineage()}
    assignments: list[RoleAssignment] = []
    for event in inventory.events():
        if event.kind is not EventKind.JOB_SUBMITTED:
            continue
        lineage = lineages.get(event.candidate_id)
        assignments.append(
            RoleAssignment(
                candidate_id=event.candidate_id,
                family_id="" if lineage is None else lineage.family_id,
                campaign_id="" if lineage is None else lineage.campaign_id,
                data_role=DataRole(str(event.payload.get("data_role", DataRole.DEVELOPMENT.value))),
            ),
        )
    return tuple(assignments)


@dataclass(frozen=True)
class PartitionRole:
    """
    The role one partition holds for one lineage.

    Parameters
    ----------
    partition : str
        The partition's identity.
    lineage_id : str
        The lineage the role applies to.
    role : DataRole
        The role.
    reason : str
        Why the partition holds the role, which is what a reader needs when it has changed.

    """

    partition: str
    lineage_id: str
    role: DataRole
    reason: str = ""

    def to_dict(self) -> dict[str, object]:
        """
        Return the role as a serialisable mapping.

        Returns
        -------
        dict[str, object]

        """
        return {
            "partition": self.partition,
            "lineage_id": self.lineage_id,
            "role": self.role.value,
            "reason": self.reason,
        }


@dataclass(frozen=True)
class PartitionRoles:
    """
    The role each partition holds for each lineage, and the downgrade the boundary rule implies.

    A partition's declared role is its role for a lineage that has never seen it. Once a result on
    that partition has influenced a lineage, the partition is development data for that lineage, and
    the assignment is recorded with the candidate whose result did the influencing. The downgrade
    travels with the lineage, so it cannot be shed by moving a candidate to a new branch or by
    re-proposing it under a new identity.

    Parameters
    ----------
    declared : Mapping[str, DataRole]
        The role each partition holds for a lineage that has never seen it.
    assignments : Mapping[tuple[str, str], PartitionRole]
        The per-lineage assignments, keyed by (partition, lineage).

    """

    declared: Mapping[str, DataRole] = field(default_factory=dict)
    assignments: Mapping[tuple[str, str], PartitionRole] = field(default_factory=dict)

    def role_for(self, partition: str, lineage_id: str) -> DataRole:
        """
        Return the role a partition holds for a lineage.

        Parameters
        ----------
        partition : str
            The partition.
        lineage_id : str
            The lineage.

        Returns
        -------
        DataRole

        """
        assigned = self.assignments.get((partition, lineage_id))
        if assigned is not None:
            return assigned.role
        return self.declared.get(partition, DataRole.DEVELOPMENT)

    def informed(self, partition: str, lineage_id: str, *, by: str) -> PartitionRoles:
        """
        Record that a partition's result influenced a lineage, downgrading it to development.

        Parameters
        ----------
        partition : str
            The partition whose result influenced the search.
        lineage_id : str
            The lineage it influenced.
        by : str
            The candidate whose result did the influencing.

        Returns
        -------
        PartitionRoles

        """
        assignment = PartitionRole(
            partition=partition,
            lineage_id=lineage_id,
            role=DataRole.DEVELOPMENT,
            reason=f"{by} was evaluated on {partition} and its result informed the search",
        )
        return replace(self, assignments={**self.assignments, (partition, lineage_id): assignment})

    def as_validation(self, partition: str, lineage_id: str) -> PartitionRole | SearchRefusal:
        """
        Return the assignment if the partition is still locked for the lineage, else the refusal.

        Parameters
        ----------
        partition : str
            The partition to evaluate a frozen candidate on.
        lineage_id : str
            The lineage that wants to present the result.

        Returns
        -------
        PartitionRole | SearchRefusal

        """
        if self.role_for(partition, lineage_id) is DataRole.LOCKED_VALIDATION:
            return PartitionRole(
                partition=partition,
                lineage_id=lineage_id,
                role=DataRole.LOCKED_VALIDATION,
                reason="the lineage has never consumed a result on this partition",
            )
        return SearchRefusal.ROLE_MISMATCH

    def to_dict(self) -> dict[str, object]:
        """
        Return the assignments as a serialisable mapping.

        Returns
        -------
        dict[str, object]

        """
        return {
            "declared": {
                partition: role.value for partition, role in sorted(self.declared.items())
            },
            "assignments": [
                assignment.to_dict()
                for _, assignment in sorted(self.assignments.items(), key=lambda item: item[0])
            ],
        }


@dataclass(frozen=True)
class Evolution:
    """
    One generation's plan, drawn from a population and the development results.

    Parameters
    ----------
    policy : SearchPolicy
        The search policy the generation runs under.
    seed : int
        The seed the generation's draws come from.
    population_size : int, default 12
        The number of children to breed.
    elite : int, default 2
        The number of survivors taken from the Pareto front before diversity is considered.
    family_floor : int, default 2
        The number of survivors each family represented on the front keeps.

    """

    policy: SearchPolicy
    seed: int
    population_size: int = 12
    elite: int = 2
    family_floor: int = 2

    def __post_init__(self) -> None:
        """
        Refuse an evolution whose population, elite or floor is not at least one.
        """
        for name, value in (
            ("population_size", self.population_size),
            ("elite", self.elite),
            ("family_floor", self.family_floor),
        ):
            if value < 1:
                raise ValueError(f"{name} must be at least one, was {value}")

    def rng_for(self, generation: int) -> random.Random:
        """
        Return the seeded generator a generation draws from.

        Parameters
        ----------
        generation : int
            The generation.

        Returns
        -------
        random.Random

        """
        return random.Random(f"{self.seed}:{generation}")  # noqa: S311 (a deterministic search RNG)

    def next_generation(
        self,
        *,
        population: Sequence[Genome],
        results: Sequence[DevelopmentResult],
        generation: int,
    ) -> GenerationPlan:
        """
        Return the plan for the next generation.

        The plan reads development results only, refuses rather than proceeds when one of them is
        not development data, and reports every operation it attempted, including the ones the
        genome's operators refused.

        Parameters
        ----------
        population : Sequence[Genome]
            The current population.
        results : Sequence[DevelopmentResult]
            The development results measured for that population.
        generation : int
            The generation being planned.

        Returns
        -------
        GenerationPlan

        """
        policy_digest = self.policy.digest()
        if not population:
            return self._refused(
                self._base(generation, policy_digest, ()),
                SearchRefusal.POPULATION_EMPTY,
            )

        selection = selectable(results)
        if not selection.usable:
            return self._refused(
                self._base(generation, policy_digest, ()),
                selection.refusal or SearchRefusal.LOCKED_RESULT_IN_SELECTION,
                selection.reason,
            )

        by_digest = {genome.digest(): genome for genome in population}
        front = pareto_front(selection.results)
        diagnosis = self._diagnose(generation, front)
        survivors = self._survivors(front, by_digest)
        if not survivors:
            # A front that names candidates the population does not hold and a front whose
            # candidates were never measured are different failures, and the plan says which.
            return self._refused(
                replace(self._base(generation, policy_digest, front), diagnosis=diagnosis),
                SearchRefusal.UNKNOWN_CANDIDATE if front else SearchRefusal.MISSING_OBJECTIVE,
                "the front names candidates the population does not hold"
                if front
                else "no candidate produced a complete set of development objectives",
            )

        rng = self.rng_for(generation)
        edits: list[Edit] = []
        children: list[Genome] = []
        for _ in range(self.population_size):
            edit = self._breed(survivors, rng)
            edits.append(edit)
            if edit.produced and edit.genome is not None:
                children.append(edit.genome)

        folded = deduplicate([*survivors, *children])
        survivors_by_digest = {genome.digest() for genome in survivors}
        produced = tuple(
            genome for genome in folded.survivors if genome.digest() not in survivors_by_digest
        )
        merges = tuple(
            merge for merge in folded.merges if merge.merged_into not in survivors_by_digest
        )

        refused = tuple(edit for edit in edits if not edit.produced)
        if not produced:
            # When every attempt was refused, the operators' own reason is the informative one; the
            # duplicate explanation belongs to the case where children were produced and folded.
            reason = (
                refused[0].reason
                if refused and len(refused) == len(edits)
                else "every child the operators produced was already in the population"
            )
            return self._refused(
                replace(
                    self._base(generation, policy_digest, front),
                    edits=tuple(edits),
                    merges=merges,
                    diagnosis=diagnosis,
                ),
                SearchRefusal.EDIT_REFUSED,
                reason,
            )

        return GenerationPlan(
            generation=generation,
            policy_digest=policy_digest,
            population=produced,
            edits=tuple(edits),
            front=tuple(result.candidate_id for result in front),
            merges=merges,
            diagnosis=diagnosis,
        )

    def _base(
        self,
        generation: int,
        policy_digest: str,
        front: Sequence[DevelopmentResult],
    ) -> GenerationPlan:
        return GenerationPlan(
            generation=generation,
            policy_digest=policy_digest,
            front=tuple(result.candidate_id for result in front),
        )

    def _refused(
        self, plan: GenerationPlan, refusal: SearchRefusal, reason: str = ""
    ) -> GenerationPlan:
        return replace(plan, refusal=refusal, reason=reason or refusal.value)

    def _survivors(
        self,
        front: Sequence[DevelopmentResult],
        by_digest: Mapping[str, Genome],
    ) -> tuple[Genome, ...]:
        selected: list[Genome] = []
        seen: set[str] = set()

        def take(result: DevelopmentResult) -> None:
            genome = by_digest.get(result.digest)
            if genome is None or result.digest in seen:
                return
            seen.add(result.digest)
            selected.append(genome)

        for result in front[: self.elite]:
            take(result)

        for family in sorted({result.family for result in front}):
            held = [result for result in front if result.family == family]
            for result in held[: self.family_floor]:
                take(result)

        for result in front:
            take(result)

        return tuple(selected)

    def _breed(self, survivors: Sequence[Genome], rng: random.Random) -> Edit:
        weights = self.policy.weights([genome.family for genome in survivors])
        family = _weighted(weights, rng)
        parents = [genome for genome in survivors if genome.family == family]
        if not parents:
            parents = list(survivors)

        explore = rng.random() < self.policy.exploration_share
        edit = self._attempt(parents, rng, explore=explore)
        for _ in range(_EDIT_ATTEMPTS - 1):
            if edit.produced:
                return edit
            edit = self._attempt(parents, rng, explore=explore)
        return edit

    def _attempt(
        self,
        parents: Sequence[Genome],
        rng: random.Random,
        *,
        explore: bool,
    ) -> Edit:
        if len(parents) > 1 and rng.random() >= self.policy.mutation_rate:
            return recombine(
                parents[rng.randrange(len(parents))],
                parents[rng.randrange(len(parents))],
                rng=rng,
            )
        parent = parents[rng.randrange(len(parents))]
        return mutate(parent, genes=self._genes(parent, rng, explore=explore), rng=rng)

    def _genes(
        self,
        genome: Genome,
        rng: random.Random,
        *,
        explore: bool,
    ) -> tuple[str, ...]:
        paths = genome.structural_paths() if explore else genome.parameter_paths()
        if not paths:
            paths = genome.paths()
        if not paths:
            return ()
        count = min(
            _STRUCTURAL_GENES_PER_MUTATION if explore else _PARAMETER_GENES_PER_MUTATION,
            len(paths),
        )
        return tuple(sorted(rng.sample(list(paths), count)))

    def _diagnose(self, generation: int, front: Sequence[DevelopmentResult]) -> SearchDiagnosis:
        if not front:
            return SearchDiagnosis(
                generation=generation,
                statement="no candidate produced a complete set of development objectives",
            )

        families = sorted({result.family for result in front})
        shares = {
            family: len([result for result in front if result.family == family]) / len(front)
            for family in families
        }
        turnovers = sorted(
            value for result in front if (value := result.value(Objective.TURNOVER)) is not None
        )
        median = turnovers[len(turnovers) // 2] if turnovers else 0.0
        concentrated = max(shares.values()) if shares else 1.0
        noun = "candidate is" if len(front) == 1 else "candidates are"
        statement = (
            f"{len(front)} {noun} non-dominated across {len(families)} families, the largest of "
            f"which holds {concentrated:.0%} of them, at a median turnover of {median:.4f}"
        )
        return SearchDiagnosis(
            generation=generation,
            family_shares=shares,
            turnover_median=median,
            statement=statement,
        )


def _weighted(weights: Sequence[tuple[str, float]], rng: random.Random) -> str:
    if not weights:
        return ""
    draw = rng.random()
    cumulative = 0.0
    for name, weight in weights:
        cumulative += weight
        if draw < cumulative:
            return name
    return weights[-1][0]


def _drop(payload: dict[str, JsonValue], path: str) -> None:
    """
    Drop a dotted path and anything flattened beneath it from a canonical mapping.

    The canonical form of a specification is a flat mapping keyed by dotted paths, so a tuple-valued
    field appears as the path itself or as indexed keys under it, and both have to go for the
    equivalence rule to ignore the field.
    """
    payload.pop(path, None)
    prefix = f"{path}."
    for key in [key for key in payload if key.startswith(prefix)]:
        payload.pop(key, None)
