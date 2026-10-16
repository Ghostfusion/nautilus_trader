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
The genotype a search operates on, its genes, and its three degrees of freedom.

A candidate specification is data, so a search can operate on it without executing anything. This
module turns that data into a genotype: the set of genes a search may change, each addressed by the
path it occupies in the specification and, where the policy declares a bound for it, carrying that
bound so a mutation cannot leave it.

Three degrees of freedom are distinguished and recorded separately, because they are not the same
kind of change. A parameter mutation changes a value inside one hypothesis. A structural mutation
changes the logic while the mechanism stands. A family or mechanism change replaces the mechanism,
and it produces a new hypothesis and a new falsification statement, because the previous pair
describes something that is no longer being tested. That rule is enforced by construction rather
than by review: the only operation that changes a mechanism takes the replacement pair as an
argument, so no call can change the mechanism and keep the old statements.

Two further limits are enforced here rather than left to a caller. Feature declarations are not
genes: a candidate may select a registered feature and set its lookback and skip, and it may not
define one, because a feature defined by the thing being tested is a degree of freedom the trial
inventory would not see. And recombination is within a family, because crossing a momentum entry
with a mean-reversion exit produces a specification with no mechanism behind it.

The operators are pure: they take a genome, the genes to change and a seeded `random.Random`, and
return a new genome with the changes that were made. They never run a backtest, never see a result,
and are deterministic under their seed, which is the contract the repository's existing search
strategies already state.
"""

from __future__ import annotations

from dataclasses import dataclass
from dataclasses import fields
from dataclasses import is_dataclass
from dataclasses import replace
from decimal import Decimal
from enum import Enum
from typing import TYPE_CHECKING

from nautilus_trader.discovery.compile import specification_digest_of
from nautilus_trader.discovery.schema import Bound
from nautilus_trader.discovery.schema import BoundKind
from nautilus_trader.discovery.schema import ConditionLeaf
from nautilus_trader.discovery.schema import Specification
from nautilus_trader.discovery.schema import canonical_mapping
from nautilus_trader.optimization.space import canonical_json


if TYPE_CHECKING:
    import random
    from collections.abc import Sequence

    from nautilus_trader.discovery.policy import DiscoveryPolicy


GENOME_VERSION = "discovery.genome/v1"

# A gene carries a numeric, string or boolean value. A numeric gene keeps the type its field
# declares, so a mutation writes back an integer where the block declares one and a decimal where it
# declares one. A numeric gene with a declared bound is a parameter gene; the string and boolean
# genes are structural, because they select a declared alternative rather than move inside a range.
type GeneValue = Decimal | int | str | bool

# A drawn value quantises to the current value's precision, never below this many places, and is
# then required to satisfy its bound. The quantum is a property of the operator, not of the
# candidate: a bound states an interval and a candidate states a value.
_MUTATION_PLACES = 8
_MUTATION_ATTEMPTS = 8

# Every structural gene, as (specification path, policy declared-set name). A gene whose
# declared-set name is empty takes a boolean.
_STRUCTURAL_GENES: tuple[tuple[str, str], ...] = (
    ("universe.membership_policy", "membership_policies"),
    ("data.signal_frequency", "signal_frequencies"),
    ("data.decision_time", "decision_times"),
    ("data.execution_eligibility", "execution_eligibilities"),
    ("data.corporate_action_treatment", "corporate_action_treatments"),
    ("selection.mode", "selection_modes"),
    ("selection.comparison", "operators"),
    ("selection.tie_break", "tie_breaks"),
    ("entry.direction", "entry_directions"),
    ("exit.comparison", "operators"),
    ("exit.flatten_ineligible", ""),
    ("portfolio.weighting", "weighting_methods"),
    ("portfolio.cash_policy", "cash_policies"),
    ("rebalance.frequency", "rebalance_frequencies"),
    ("rebalance.timing", "rebalance_timings"),
    ("risk.runtime.halt_new_entries", ""),
    ("risk.runtime.flatten_ineligible", ""),
)

# Every parameter gene, as (policy bound name, specification path). `{index}` expands once per
# element of the block its path names, so a feature declaration's lookback is a gene per feature.
_PARAMETER_GENES: tuple[tuple[str, str], ...] = (
    ("feature.lookback", "features.declarations[{index}].lookback"),
    ("feature.skip", "features.declarations[{index}].skip"),
    ("selection.percentile", "selection.percentile"),
    ("selection.count", "selection.count"),
    ("selection.threshold", "selection.threshold"),
    ("exit.holding_period_cap", "exit.holding_period_cap"),
    ("exit.rank_exit_count", "exit.rank_exit_count"),
    ("exit.stop_fraction", "exit.stop_fraction"),
    ("exit.target_fraction", "exit.target_fraction"),
    ("portfolio.position_count", "portfolio.position_count"),
    ("portfolio.max_position_weight", "portfolio.max_position_weight"),
    ("portfolio.max_sector_weight", "portfolio.max_sector_weight"),
    ("risk.max_gross_exposure", "risk.pre_trade.max_gross_exposure"),
    ("risk.max_drawdown", "risk.post_run.max_drawdown"),
    ("risk.max_turnover", "risk.post_run.max_turnover"),
    ("risk.max_cost_ratio", "risk.post_run.max_cost_ratio"),
    ("universe.min_price", "universe.min_price"),
    ("universe.min_dollar_volume", "universe.min_dollar_volume"),
)

# The one bound that applies to any number of condition leaves, so the leaves are found by walking
# the condition tree rather than by a path with an index.
_CONDITION_BOUND = "condition.value"

_STATEMENT_FIELDS: tuple[str, ...] = ("family", "mechanism", "hypothesis", "falsification")

# The paths that carry the statements, which are not genes: they are replaced through
# `with_mechanism`, which takes the replacement pair as an argument.
_MECHANISM_PATHS: tuple[str, ...] = tuple(f"identity.{name}" for name in _STATEMENT_FIELDS)


class GeneticDegree(Enum):
    """
    The kind of change a gene carries, which is what makes it a degree of freedom.

    The three are recorded separately because they are not the same kind of change: one moves a
    value inside a hypothesis, one changes the logic while the mechanism stands, and one replaces
    the mechanism and therefore the hypothesis and falsification statement with it.
    """

    PARAMETER = "parameter"
    STRUCTURAL = "structural"
    FAMILY = "family"

    @property
    def requires_new_hypothesis(self) -> bool:
        """
        Whether a change of this degree produces a new hypothesis and falsification statement.

        Returns
        -------
        bool

        """
        return self is GeneticDegree.FAMILY


class GenomeRefusal(Enum):
    """
    The vocabulary a genotype-level refusal is reported with.

    A candidate-level refusal stays a `RefusalCode`, because the compiler owns that vocabulary. A
    refusal reported here is about the search's own operators: a gene that cannot be changed, a
    value outside its bound, or a recombination the family rule does not permit.
    """

    GENE_UNRESOLVED = "gene_unresolved"
    GENE_NOT_MUTABLE = "gene_not_mutable"
    FAMILY_MISMATCH = "family_mismatch"
    VALUE_NOT_DECLARED = "value_not_declared"
    NO_FREE_GENE = "no_free_gene"


@dataclass(frozen=True)
class Gene:
    """
    One addressable position in a genome.

    Parameters
    ----------
    name : str
        The policy's bound name where one applies, otherwise the gene's path.
    path : str
        The path the gene occupies in the specification.
    degree : GeneticDegree
        The kind of change the gene carries.
    value : GeneValue
        The gene's current value.
    bound : Bound | None
        The declared bound, when the policy declares one for this gene.
    declared : frozenset[str]
        The declared set the gene selects from, when it selects a declared alternative.

    """

    name: str
    path: str
    degree: GeneticDegree
    value: GeneValue
    bound: Bound | None = None
    declared: frozenset[str] = frozenset()

    @property
    def choices(self) -> tuple[GeneValue, ...]:
        """
        The discrete values this gene may take other than the one it holds.

        A range bound is an interval rather than a list, so it has no discrete alternatives and the
        operator draws inside it instead.

        Returns
        -------
        tuple[GeneValue, ...]

        """
        if self.bound is not None and self.bound.kind is BoundKind.RANGE:
            return ()
        if self.bound is not None:
            return tuple(choice for choice in self.bound.choices if choice != self.value)
        if self.declared:
            return tuple(sorted(choice for choice in self.declared if choice != self.value))
        if isinstance(self.value, bool):
            return (not self.value,)
        return ()

    def to_dict(self) -> dict[str, object]:
        """
        Return the gene as a serialisable mapping.

        Returns
        -------
        dict[str, object]

        """
        return {
            "name": self.name,
            "path": self.path,
            "degree": self.degree.value,
            "value": _canonical_value(self.value),
            "bound": None if self.bound is None else self.bound.describe(),
            "declared": sorted(self.declared),
        }


@dataclass(frozen=True)
class Mechanism:
    """
    The statements that describe what a candidate is testing, and the family it belongs to.

    Parameters
    ----------
    family : str
        The declared family the mechanism belongs to.
    mechanism : str
        The economic mechanism the hypothesis rests on.
    hypothesis : str
        The falsifiable hypothesis.
    falsification : str
        The condition that would falsify it.

    """

    family: str
    mechanism: str
    hypothesis: str
    falsification: str

    def to_dict(self) -> dict[str, object]:
        """
        Return the mechanism as a serialisable mapping.

        Returns
        -------
        dict[str, object]

        """
        return {name: getattr(self, name) for name in _STATEMENT_FIELDS}


@dataclass(frozen=True)
class Change:
    """
    One gene a single operation changed, with its degree and both values.

    Parameters
    ----------
    path : str
        The path the gene occupies in the specification.
    name : str
        The policy's bound name where one applies, otherwise the path.
    degree : GeneticDegree
        The kind of change.
    before : GeneValue
        The value before the operation.
    after : GeneValue
        The value after the operation.

    """

    path: str
    name: str
    degree: GeneticDegree
    before: GeneValue
    after: GeneValue

    def to_dict(self) -> dict[str, object]:
        """
        Return the change as a serialisable mapping.

        Returns
        -------
        dict[str, object]

        """
        return {
            "path": self.path,
            "name": self.name,
            "degree": self.degree.value,
            "before": _canonical_value(self.before),
            "after": _canonical_value(self.after),
        }


@dataclass(frozen=True)
class Edit:
    """
    The outcome of an operator: a genome with its changes, or the refusal that stopped it.

    Parameters
    ----------
    genome : Genome | None
        The edited genome, or nothing when the operation was refused.
    changes : tuple[Change, ...]
        Every gene the operation changed, in path order.
    refusal : GenomeRefusal | None
        The refusal code, or nothing when the operation produced a genome.
    reason : str
        Why the operation was refused, empty when it produced a genome.

    """

    genome: Genome | None = None
    changes: tuple[Change, ...] = ()
    refusal: GenomeRefusal | None = None
    reason: str = ""

    @property
    def produced(self) -> bool:
        """
        Whether the operation produced a genome.

        Returns
        -------
        bool

        """
        return self.genome is not None

    @property
    def degrees(self) -> frozenset[GeneticDegree]:
        """
        The distinct degrees of freedom this edit exercised.

        Returns
        -------
        frozenset[GeneticDegree]

        """
        return frozenset(change.degree for change in self.changes)

    @property
    def requires_new_hypothesis(self) -> bool:
        """
        Whether this edit replaced the mechanism, and so the statements with it.

        Returns
        -------
        bool

        """
        return any(change.degree.requires_new_hypothesis for change in self.changes)

    def to_dict(self) -> dict[str, object]:
        """
        Return the edit as a serialisable mapping.

        Returns
        -------
        dict[str, object]

        """
        return {
            "genome": None if self.genome is None else self.genome.digest(),
            "changes": [change.to_dict() for change in self.changes],
            "degrees": sorted(degree.value for degree in self.degrees),
            "requires_new_hypothesis": self.requires_new_hypothesis,
            "refusal": None if self.refusal is None else self.refusal.value,
            "reason": self.reason,
        }


@dataclass(frozen=True)
class Genome:
    """
    A candidate specification together with the policy it is read against.

    The specification is the genotype: every gene this module reports is a path inside it. The
    policy travels with the genome because a bound and a declared set are policy objects, and a
    mutation that resolved bounds against a different policy would not be the same operation.

    Parameters
    ----------
    specification : Specification
        The candidate specification.
    policy : DiscoveryPolicy
        The policy whose bounds and declared sets the genes resolve against.

    Raises
    ------
    ValueError
        If a bounded value sits outside its declared bound, which would mean the specification did
        not come through the compiler.

    """

    specification: Specification
    policy: DiscoveryPolicy

    def __post_init__(self) -> None:
        """
        Refuse a genome whose bounded values do not satisfy the bounds they are read against.
        """
        for gene in self.genes():
            if gene.bound is None or not _is_numeric(gene.value):
                continue
            if not gene.bound.admits(Decimal(gene.value)):
                raise ValueError(f"{gene.path} is outside its bound {gene.bound.describe()}")

    @property
    def family(self) -> str:
        """
        The declared family the candidate belongs to.

        Returns
        -------
        str

        """
        return self.specification.identity.family

    @property
    def seed(self) -> int:
        """
        The declared seed of the candidate's provenance.

        Returns
        -------
        int

        """
        return self.specification.provenance.seed

    @property
    def parents(self) -> tuple[str, ...]:
        """
        The declared parent candidate identifiers.

        Returns
        -------
        tuple[str, ...]

        """
        return self.specification.provenance.parents

    @property
    def mechanism(self) -> Mechanism:
        """
        The mechanism, hypothesis and falsification statement the candidate carries.

        Returns
        -------
        Mechanism

        """
        identity = self.specification.identity
        return Mechanism(
            family=identity.family,
            mechanism=identity.mechanism,
            hypothesis=identity.hypothesis,
            falsification=identity.falsification,
        )

    def digest(self) -> str:
        """
        Return the specification's canonical digest, which is the candidate's identity.

        Returns
        -------
        str

        """
        return specification_digest_of(self.specification)

    def canonical_form(self) -> str:
        """
        Return the canonical JSON of the specification.

        Returns
        -------
        str

        """
        return canonical_json(canonical_mapping(self.specification.to_dict()))

    def genes(self) -> tuple[Gene, ...]:
        """
        Return every gene of the genome, ordered by path.

        Returns
        -------
        tuple[Gene, ...]

        """
        genes = [*self._parameter_genes(), *self._structural_genes()]
        return tuple(sorted(genes, key=lambda gene: gene.path))

    def gene(self, path: str) -> Gene | None:
        """
        Return the gene at a path, or nothing when the path is not a gene.

        Parameters
        ----------
        path : str
            The specification path.

        Returns
        -------
        Gene | None

        """
        for gene in self.genes():
            if gene.path == path:
                return gene
        return None

    def paths(self) -> tuple[str, ...]:
        """
        Return every gene path, ordered.

        Returns
        -------
        tuple[str, ...]

        """
        return tuple(gene.path for gene in self.genes())

    def parameter_paths(self) -> tuple[str, ...]:
        """
        Return the paths of the parameter genes.

        Returns
        -------
        tuple[str, ...]

        """
        return tuple(gene.path for gene in self.genes() if gene.degree is GeneticDegree.PARAMETER)

    def structural_paths(self) -> tuple[str, ...]:
        """
        Return the paths of the structural genes.

        Returns
        -------
        tuple[str, ...]

        """
        return tuple(gene.path for gene in self.genes() if gene.degree is GeneticDegree.STRUCTURAL)

    def with_paths(self, paths: Sequence[str], values: Sequence[GeneValue]) -> Genome:
        """
        Return a new genome with the given paths set to the given values.

        This is the recording path rather than the drawing path: a caller that already knows the
        values it wants applies them here, and `mutate` is what draws them.

        Parameters
        ----------
        paths : Sequence[str]
            The paths to set.
        values : Sequence[GeneValue]
            The values to set them to, one per path.

        Returns
        -------
        Genome

        """
        specification = self.specification
        for path, value in zip(paths, values, strict=True):
            specification = _write_path(specification, path, value)
        return Genome(specification=specification, policy=self.policy)

    def _parameter_genes(self) -> list[Gene]:
        genes: list[Gene] = []
        for name, pattern in _PARAMETER_GENES:
            bound = self.policy.bound(name)
            if bound is None:
                continue
            for path in _expand_path(pattern, self.specification):
                value = _read_path(self.specification, path)
                if not _is_numeric(value):
                    continue
                genes.append(
                    Gene(
                        name=name,
                        path=path,
                        degree=GeneticDegree.PARAMETER,
                        value=value,
                        bound=bound,
                    ),
                )
        genes.extend(self._condition_genes())
        return genes

    def _condition_genes(self) -> list[Gene]:
        bound = self.policy.bound(_CONDITION_BOUND)
        if bound is None:
            return []
        return [
            Gene(
                name=_CONDITION_BOUND,
                path=path,
                degree=GeneticDegree.PARAMETER,
                value=value,
                bound=bound,
            )
            for path in _condition_value_paths(self.specification)
            if _is_numeric(value := _read_path(self.specification, path))
        ]

    def _structural_genes(self) -> list[Gene]:
        genes: list[Gene] = []
        for path, declared_name in _STRUCTURAL_GENES:
            value = _read_path(self.specification, path)
            if not isinstance(value, str | bool):
                continue
            declared = (
                frozenset() if not declared_name else frozenset(self.policy.declared(declared_name))
            )
            genes.append(
                Gene(
                    name=path,
                    path=path,
                    degree=GeneticDegree.STRUCTURAL,
                    value=value,
                    declared=declared,
                ),
            )
        return genes


def genome_of(specification: Specification, policy: DiscoveryPolicy) -> Genome:
    """
    Return the genome of a specification read against a policy.

    Parameters
    ----------
    specification : Specification
        The candidate specification.
    policy : DiscoveryPolicy
        The policy whose bounds and declared sets the genes resolve against.

    Returns
    -------
    Genome

    """
    return Genome(specification=specification, policy=policy)


def mutate(genome: Genome, *, genes: Sequence[str], rng: random.Random) -> Edit:
    """
    Change the named genes to values drawn from their bounds, or refuse the whole edit.

    A parameter gene draws a value inside its declared bound or from its declared choices, and a
    structural gene takes another value from its declared set. A condition leaf's value is a
    parameter gene like any other. A path that is not a gene, or a mechanism, is refused rather
    than skipped, so a caller that asked for three genes is told which one stopped the edit. A path
    named twice is changed once, because the operation reports the genes it changed rather than the
    arguments it was given.

    Parameters
    ----------
    genome : Genome
        The genome to change.
    genes : Sequence[str]
        The specification paths to change.
    rng : random.Random
        The seeded generator the draws come from.

    Returns
    -------
    Edit

    """
    resolved: list[Gene] = []
    seen: set[str] = set()
    for path in genes:
        if path in seen:
            continue
        seen.add(path)
        gene = genome.gene(path)
        if gene is None:
            if path in _MECHANISM_PATHS:
                return _refused(
                    GenomeRefusal.GENE_NOT_MUTABLE,
                    f"{path} replaces the mechanism, which needs a replacement statement",
                )
            return _refused(GenomeRefusal.GENE_UNRESOLVED, f"{path} is not a gene")
        if gene.degree is GeneticDegree.FAMILY:
            return _refused(
                GenomeRefusal.GENE_NOT_MUTABLE,
                f"{path} replaces the mechanism, which needs a replacement statement",
            )
        if not _has_free_value(gene):
            return _refused(GenomeRefusal.NO_FREE_GENE, f"{path} has no other admissible value")
        resolved.append(gene)

    if not resolved:
        return _refused(GenomeRefusal.GENE_UNRESOLVED, "no gene was named")

    specification = genome.specification
    changes: list[Change] = []
    for gene in resolved:
        drawn = _draw(gene, rng)
        specification = _write_path(specification, gene.path, drawn)
        changes.append(
            Change(
                path=gene.path,
                name=gene.name,
                degree=gene.degree,
                before=gene.value,
                after=drawn,
            ),
        )

    return Edit(
        genome=Genome(specification=specification, policy=genome.policy),
        changes=tuple(changes),
    )


def with_mechanism(genome: Genome, mechanism: Mechanism, *, parents: tuple[str, ...] = ()) -> Edit:
    """
    Replace the mechanism, and with it the hypothesis and falsification statement.

    This is the only operation that changes a mechanism or a family, and it takes the replacement
    statements as an argument by requirement: the previous pair describes something that is no
    longer being tested, so no call can change the mechanism and keep it.

    Parameters
    ----------
    genome : Genome
        The genome to change.
    mechanism : Mechanism
        The replacement mechanism, hypothesis and falsification statement.
    parents : tuple[str, ...]
        The parent candidate identifiers, which default to this genome's digest.

    Returns
    -------
    Edit

    """
    missing = [name for name in _STATEMENT_FIELDS if not str(getattr(mechanism, name)).strip()]
    if missing:
        return _refused(
            GenomeRefusal.VALUE_NOT_DECLARED,
            f"the replacement statement is missing {', '.join(missing)}",
        )
    if mechanism.family not in genome.policy.declared("families"):
        return _refused(
            GenomeRefusal.VALUE_NOT_DECLARED,
            f"{mechanism.family} is not a declared family",
        )

    current = genome.mechanism
    changes = tuple(
        Change(
            path=f"identity.{name}",
            name=f"identity.{name}",
            degree=GeneticDegree.FAMILY,
            before=getattr(current, name),
            after=getattr(mechanism, name),
        )
        for name in _STATEMENT_FIELDS
        if getattr(current, name) != getattr(mechanism, name)
    )
    if not changes:
        return _refused(
            GenomeRefusal.NO_FREE_GENE,
            "the replacement mechanism is the one the genome already carries",
        )

    identity = replace(
        genome.specification.identity,
        family=mechanism.family,
        mechanism=mechanism.mechanism,
        hypothesis=mechanism.hypothesis,
        falsification=mechanism.falsification,
    )
    provenance = replace(
        genome.specification.provenance,
        parents=parents or (genome.digest(),),
    )
    specification = replace(genome.specification, identity=identity, provenance=provenance)

    return Edit(genome=Genome(specification=specification, policy=genome.policy), changes=changes)


def recombine(first: Genome, second: Genome, *, rng: random.Random) -> Edit:
    """
    Exchange compatible genes between two parents of the same family.

    Compatibility is the shared gene path: only a position both parents carry can be exchanged, and
    a value the receiving genome's own bound or declared set does not admit is left alone. The
    exchange is a block rather than a per-gene coin flip: the split is drawn from the positions
    where the parents differ, and every shared gene from that position onward takes the donor's
    value. A
    cross-over that could miss the only difference and report a refusal would make the operator's
    outcome depend on a draw rather than on the parents. Parents of different families are refused,
    because crossing one family's entry with another's exit produces a specification with no
    mechanism behind it.

    Parameters
    ----------
    first : Genome
        The genome that receives.
    second : Genome
        The genome that donates.
    rng : random.Random
        The seeded generator the exchange decisions come from.

    Returns
    -------
    Edit

    """
    if first.family != second.family:
        return _refused(
            GenomeRefusal.FAMILY_MISMATCH,
            f"{first.family} and {second.family} are different families",
        )

    donors = {gene.path: gene for gene in second.genes()}
    ordered = first.genes()
    differences = [
        index
        for index, gene in enumerate(ordered)
        if (donor := donors.get(gene.path)) is not None
        and donor.value != gene.value
        and _admits(gene, donor.value)
    ]
    if not differences:
        return _refused(GenomeRefusal.NO_FREE_GENE, "the parents share no admissible difference")

    split = rng.choice(differences)
    specification = first.specification
    changes: list[Change] = []
    for gene in ordered[split:]:
        donor = donors.get(gene.path)
        if donor is None or donor.value == gene.value or not _admits(gene, donor.value):
            continue
        specification = _write_path(specification, gene.path, donor.value)
        changes.append(
            Change(
                path=gene.path,
                name=gene.name,
                degree=gene.degree,
                before=gene.value,
                after=donor.value,
            ),
        )

    return Edit(
        genome=Genome(specification=specification, policy=first.policy),
        changes=tuple(changes),
    )


def _admits(gene: Gene, value: GeneValue) -> bool:
    if gene.bound is not None:
        if not _is_numeric(value) or not _is_numeric(gene.value):
            return False
        if isinstance(value, int) != isinstance(gene.value, int):
            return False
        return gene.bound.admits(Decimal(value))
    if gene.declared:
        return isinstance(value, str) and value in gene.declared
    return isinstance(value, bool)


def _is_numeric(value: object) -> bool:
    if isinstance(value, bool):
        return False
    return isinstance(value, int | Decimal)


def _has_free_value(gene: Gene) -> bool:
    if gene.bound is not None:
        if gene.bound.kind is BoundKind.CHOICE:
            return len(gene.choices) > 0
        return gene.bound.lower != gene.bound.upper
    return len(gene.choices) > 0


def _draw(gene: Gene, rng: random.Random) -> GeneValue:
    if gene.bound is not None and gene.bound.kind is BoundKind.CHOICE:
        # The gene's own alternatives exclude the value it holds, so a choice bound cannot be
        # "mutated" into the value it already carries.
        return rng.choice(gene.choices)
    if gene.bound is not None and isinstance(gene.value, int):
        return _draw_int(Decimal(gene.value), gene.bound, rng)
    if gene.bound is not None and isinstance(gene.value, Decimal):
        return _draw_decimal(gene.value, gene.bound, rng)

    choices = gene.choices
    if not choices:
        raise ValueError(f"{gene.path} has no other admissible value")
    return rng.choice(choices)


def _bounds(bound: Bound, current: Decimal) -> tuple[Decimal, Decimal, Decimal]:
    lower = bound.lower if bound.lower is not None else Decimal(0)
    upper = bound.upper if bound.upper is not None else lower
    places = max(_MUTATION_PLACES, -current.as_tuple().exponent)
    return lower, upper, Decimal(1).scaleb(-places)


def _draw_decimal(current: Decimal, bound: Bound, rng: random.Random) -> Decimal:
    lower, upper, quantiser = _bounds(bound, current)
    span = upper - lower

    for _ in range(_MUTATION_ATTEMPTS):
        offset = Decimal(repr(rng.random())) * span if span else Decimal(0)
        drawn = (lower + offset).quantize(quantiser)
        if bound.admits(drawn):
            return drawn

    raise ValueError(f"no value inside {bound.describe()} was drawn")


def _draw_int(current: Decimal, bound: Bound, rng: random.Random) -> int:
    lower, upper, _ = _bounds(bound, current)
    floor = int(lower.to_integral_value(rounding="ROUND_CEILING"))
    ceiling = int(upper.to_integral_value(rounding="ROUND_FLOOR"))
    if floor > ceiling:
        raise ValueError(f"no integer sits inside {bound.describe()}")

    for _ in range(_MUTATION_ATTEMPTS):
        drawn = rng.randint(floor, ceiling)
        if bound.admits(Decimal(drawn)):
            return drawn

    raise ValueError(f"no integer inside {bound.describe()} was drawn")


def _refused(code: GenomeRefusal, reason: str) -> Edit:
    return Edit(refusal=code, reason=reason)


def _canonical_value(value: GeneValue) -> object:
    if isinstance(value, Decimal):
        return str(value)
    return value


def _expand_path(pattern: str, specification: Specification) -> tuple[str, ...]:
    if "{index}" not in pattern:
        return (pattern,)

    head, _, tail = pattern.partition("[{index}]")
    collection = _read_path(specification, head)
    if not isinstance(collection, tuple):
        return ()
    return tuple(f"{head}[{index}]{tail}" for index in range(len(collection)))


def _condition_value_paths(specification: Specification) -> tuple[str, ...]:
    paths: list[str] = []
    node = specification.entry.conditions
    if node is not None:
        _walk_conditions(node, "entry.conditions", paths)
    return tuple(paths)


def _walk_conditions(node: object, path: str, paths: list[str]) -> None:
    if isinstance(node, ConditionLeaf):
        paths.append(f"{path}.value")
        return

    children = getattr(node, "conditions", ())
    for index, child in enumerate(children):
        _walk_conditions(child, f"{path}.conditions[{index}]", paths)


def _read_path(node: object, path: str) -> object:
    current = node
    for step, index in _steps(path):
        current = _descend(current, step)
        if index is None:
            continue
        if not isinstance(current, tuple) or index >= len(current):
            return None
        current = current[index]
    return current


def _write_path(node: object, path: str, value: GeneValue) -> Specification:
    rebuilt = _rebuild(node, _steps(path), value)
    if not isinstance(rebuilt, Specification):
        raise TypeError(f"{path} did not resolve to a specification")
    return rebuilt


def _rebuild(node: object, steps: tuple[tuple[str, int | None], ...], value: GeneValue) -> object:
    step, index = steps[0]
    if not is_dataclass(node) or isinstance(node, type):
        raise TypeError(f"{step} is not a block of a specification")

    holds = getattr(node, step, None)
    if index is None:
        if len(steps) == 1:
            return replace(node, **{step: value})
        return replace(node, **{step: _rebuild(holds, steps[1:], value)})

    if not isinstance(holds, tuple) or index >= len(holds):
        raise IndexError(f"{step}[{index}] does not exist")

    rebuilt = value if len(steps) == 1 else _rebuild(holds[index], steps[1:], value)
    return replace(node, **{step: (*holds[:index], rebuilt, *holds[index + 1 :])})


def _descend(node: object, step: str) -> object:
    if not is_dataclass(node) or isinstance(node, type):
        return None
    if step not in {field.name for field in fields(node)}:
        return None
    return getattr(node, step)


def _steps(path: str) -> tuple[tuple[str, int | None], ...]:
    steps: list[tuple[str, int | None]] = []
    for part in path.split("."):
        name, _, rest = part.partition("[")
        steps.append((name, int(rest.rstrip("]")) if rest else None))
    return tuple(steps)
