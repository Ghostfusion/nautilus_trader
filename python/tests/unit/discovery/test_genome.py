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
Tests for the genotype, its genes and the three degrees of freedom a search may exercise.

The three degrees are different guarantees rather than three names for a change. A parameter
mutation moves a value inside the bound the policy declared and keeps the hypothesis. A structural
mutation selects another declared alternative and keeps the hypothesis too. A family change replaces
the mechanism, and with it the hypothesis and falsification statement, which is why it is the only
degree that needs a new hypothesis. Every operator is pure: it takes a seeded generator and returns
a new genome, and a refusal is a value rather than an exception.
"""

from __future__ import annotations

import random
from dataclasses import replace
from decimal import Decimal
from typing import cast

import pytest

from nautilus_trader.discovery.compile import compile_specification
from nautilus_trader.discovery.compile import specification_digest_of
from nautilus_trader.discovery.features import FeatureRegistry
from nautilus_trader.discovery.genome import Gene
from nautilus_trader.discovery.genome import GeneticDegree
from nautilus_trader.discovery.genome import Genome
from nautilus_trader.discovery.genome import GenomeRefusal
from nautilus_trader.discovery.genome import Mechanism
from nautilus_trader.discovery.genome import genome_of
from nautilus_trader.discovery.genome import mutate
from nautilus_trader.discovery.genome import recombine
from nautilus_trader.discovery.genome import with_mechanism
from nautilus_trader.discovery.policy import DiscoveryPolicy
from nautilus_trader.discovery.schema import Bound
from nautilus_trader.discovery.schema import BoundKind
from nautilus_trader.discovery.schema import canonical_mapping
from nautilus_trader.optimization.space import canonical_json
from tests.unit.discovery.fixtures import LEGAL_LOOKBACK
from tests.unit.discovery.fixtures import fixture_policy
from tests.unit.discovery.fixtures import fixture_registry
from tests.unit.discovery.fixtures import legal_payload


# A replacement mechanism for a family the fixture policy declares, so the only statement the
# genome keeps is the one the caller supplied.
REPLACEMENT = Mechanism(
    family="absolute_threshold",
    mechanism="A fixed threshold is crossed on the feature.",
    hypothesis="The threshold predicts the next week's return.",
    falsification="Refuted when the threshold carries no return.",
)

# A second family, used to show that recombination refuses two different mechanisms.
OTHER_FAMILY = Mechanism(
    family="static_hold",
    mechanism="The eligible set is held without a signal.",
    hypothesis="The eligible set earns its drift.",
    falsification="Refuted when the held set does not beat cash.",
)

# The parameter genes the fixture payload yields: every bounded field it declares. The condition
# leaf is found by walking the tree, so its path carries no index from the parameter table.
PARAMETER_PATHS = {
    "entry.conditions.conditions[0].value",
    "exit.holding_period_cap",
    "features.declarations[0].lookback",
    "features.declarations[0].skip",
    "portfolio.max_position_weight",
    "portfolio.position_count",
    "risk.pre_trade.max_gross_exposure",
    "selection.percentile",
    "universe.min_dollar_volume",
    "universe.min_price",
}

# The structural genes the fixture payload yields: every field that selects a declared alternative,
# or takes a boolean. `selection.comparison` and `exit.comparison` are absent because this payload
# declares neither.
STRUCTURAL_PATHS = {
    "data.corporate_action_treatment",
    "data.decision_time",
    "data.execution_eligibility",
    "data.signal_frequency",
    "entry.direction",
    "exit.flatten_ineligible",
    "portfolio.cash_policy",
    "portfolio.weighting",
    "rebalance.frequency",
    "rebalance.timing",
    "risk.runtime.flatten_ineligible",
    "risk.runtime.halt_new_entries",
    "selection.mode",
    "selection.tie_break",
    "universe.membership_policy",
}


def compile_fixture() -> tuple[DiscoveryPolicy, FeatureRegistry, Genome]:
    """
    Return the fixture policy and registry, and the genome of the fixture payload they accept.

    Returns
    -------
    tuple[DiscoveryPolicy, FeatureRegistry, Genome]

    """
    policy = fixture_policy()
    registry = fixture_registry()
    result = compile_specification(legal_payload(), policy, registry)
    assert result.plan is not None

    return policy, registry, genome_of(result.plan.specification, policy)


def compiled_genome() -> Genome:
    """
    Return the genome of the fixture payload compiled against the fixture policy.

    Returns
    -------
    Genome

    """
    return compile_fixture()[2]


def gene_of(genome: Genome, path: str) -> Gene:
    """
    Return the gene at a path, refusing a path the genome does not carry.

    Parameters
    ----------
    genome : Genome
        The genome to read.
    path : str
        The specification path.

    Returns
    -------
    Gene

    """
    gene = genome.gene(path)
    assert gene is not None

    return gene


def seeded_generator(seed: int) -> random.Random:
    """
    Return the module's pseudo-random generator, seeded so a draw is reproducible.

    The operators ask their generator for `choice`, `randint` and `random`, all of which the module
    provides, so seeding it here is enough to make a draw deterministic without a second generator
    whose sequence could drift from the one the operators read.

    Parameters
    ----------
    seed : int
        The seed the generator is reset to before the draw.

    Returns
    -------
    random.Random

    """
    random.seed(seed)

    return cast("random.Random", random)


def test_a_degree_records_whether_it_replaces_the_hypothesis() -> None:
    """Only a family change produces a new hypothesis and falsification statement."""
    assert GeneticDegree.PARAMETER.requires_new_hypothesis is False
    assert GeneticDegree.STRUCTURAL.requires_new_hypothesis is False
    assert GeneticDegree.FAMILY.requires_new_hypothesis is True
    assert [degree.value for degree in GeneticDegree] == ["parameter", "structural", "family"]


def test_a_gene_offers_its_other_admissible_values() -> None:
    """A range is an interval, and a choice bound, a declared set and a boolean enumerate."""
    genome = compiled_genome()

    # A range is an interval rather than a list, so there is nothing discrete to offer.
    assert gene_of(genome, "universe.min_price").choices == ()

    # A declared set offers every member other than the one held, in sorted order.
    assert gene_of(genome, "selection.tie_break").choices == ("instrument_id_asc",)

    # A declared set with a single member has no alternative, so it offers nothing.
    assert gene_of(genome, "data.signal_frequency").choices == ()

    # A boolean with no declared set offers its negation.
    boolean = gene_of(genome, "exit.flatten_ineligible")
    assert boolean.value is True
    assert boolean.choices == (False,)

    # A choice bound offers its other choices, whatever the gene's degree.
    bound = Bound("synthetic", BoundKind.CHOICE, choices=(Decimal(1), Decimal(2), Decimal(3)))
    choice = Gene(
        name="synthetic",
        path="synthetic",
        degree=GeneticDegree.PARAMETER,
        value=Decimal(2),
        bound=bound,
    )
    assert choice.choices == (Decimal(1), Decimal(3))


def test_a_genome_enumerates_the_parameter_gene_of_every_bounded_field() -> None:
    """Every bounded field the payload declares is a parameter gene carrying that bound."""
    genome = compiled_genome()

    assert set(genome.parameter_paths()) == PARAMETER_PATHS
    assert all(gene_of(genome, path).degree is GeneticDegree.PARAMETER for path in PARAMETER_PATHS)
    assert all(gene_of(genome, path).bound is not None for path in PARAMETER_PATHS)

    # The feature lookback is a gene per declared feature, and the condition leaf is a gene found
    # by walking the tree rather than by an indexed path from the parameter table.
    assert gene_of(genome, "features.declarations[0].lookback").value == LEGAL_LOOKBACK
    assert gene_of(genome, "entry.conditions.conditions[0].value").name == "condition.value"


def test_a_genome_enumerates_the_structural_gene_of_every_switched_field() -> None:
    """Every field that selects a declared alternative, or takes a boolean, is structural."""
    genome = compiled_genome()

    assert set(genome.structural_paths()) == STRUCTURAL_PATHS
    assert all(
        gene_of(genome, path).degree is GeneticDegree.STRUCTURAL for path in STRUCTURAL_PATHS
    )
    assert all(gene_of(genome, path).bound is None for path in STRUCTURAL_PATHS)

    # A declared-set gene carries the policy's declared set; a boolean gene carries none.
    assert gene_of(genome, "selection.mode").declared == frozenset({"rank", "threshold", "none"})
    assert gene_of(genome, "rebalance.frequency").declared == frozenset(
        {"daily", "weekly", "monthly"},
    )
    assert gene_of(genome, "exit.flatten_ineligible").declared == frozenset()


def test_the_two_degrees_partition_the_genotype() -> None:
    """The genes are ordered, split into parameters and structure, and found by their path."""
    genome = compiled_genome()

    assert genome.paths() == tuple(sorted(genome.paths()))
    assert genome.paths() == tuple(sorted((*genome.parameter_paths(), *genome.structural_paths())))
    assert set(genome.parameter_paths()).isdisjoint(genome.structural_paths())

    # A gene is found by the path it occupies, and a statement is not a gene.
    assert genome.gene("selection.percentile") in genome.genes()
    assert genome.gene("no.such.path") is None
    assert genome.gene("identity.mechanism") is None


def test_a_genomes_digest_and_canonical_form_are_its_identity() -> None:
    """The digest is the specification identity, and the canonical form is the stored text."""
    genome = compiled_genome()

    assert genome.digest() == specification_digest_of(genome.specification)
    assert genome.canonical_form() == canonical_json(
        canonical_mapping(genome.specification.to_dict())
    )

    edited = genome.with_paths(["selection.percentile"], [Decimal("0.5")])
    assert edited.digest() != genome.digest()
    assert edited.canonical_form() != genome.canonical_form()


def test_a_genome_reads_its_mechanism_from_the_identity_block() -> None:
    """The mechanism, hypothesis and falsification statement are the identity block's own."""
    genome = compiled_genome()
    identity = genome.specification.identity

    assert genome.family == identity.family
    assert genome.mechanism == Mechanism(
        family=identity.family,
        mechanism=identity.mechanism,
        hypothesis=identity.hypothesis,
        falsification=identity.falsification,
    )
    assert genome.mechanism.to_dict() == {
        "family": identity.family,
        "mechanism": identity.mechanism,
        "hypothesis": identity.hypothesis,
        "falsification": identity.falsification,
    }


def test_a_genome_refuses_a_value_outside_its_bound() -> None:
    """A bounded value is required to sit inside its bound, at both ends."""
    genome = compiled_genome()
    selection = genome.specification.selection

    above = replace(genome.specification, selection=replace(selection, percentile=Decimal("1.5")))
    with pytest.raises(ValueError, match=r"selection\.percentile"):
        genome_of(above, genome.policy)

    below = replace(genome.specification, selection=replace(selection, percentile=Decimal("0.001")))
    with pytest.raises(ValueError, match="outside its bound"):
        genome_of(below, genome.policy)

    # The bounds are inclusive, so the limits themselves are admissible.
    at_lower = replace(
        genome.specification, selection=replace(selection, percentile=Decimal("0.01"))
    )
    assert gene_of(genome_of(at_lower, genome.policy), "selection.percentile").value == Decimal(
        "0.01",
    )
    at_upper = replace(genome.specification, selection=replace(selection, percentile=Decimal(1)))
    assert gene_of(genome_of(at_upper, genome.policy), "selection.percentile").value == Decimal(1)


def test_with_paths_records_values_without_mutating_the_genome() -> None:
    """Recording a value writes it into a new genome and leaves the original where it was."""
    genome = compiled_genome()

    edited = genome.with_paths(
        ["features.declarations[0].lookback", "selection.mode"],
        [100, "threshold"],
    )
    assert gene_of(edited, "features.declarations[0].lookback").value == 100
    assert gene_of(edited, "selection.mode").value == "threshold"
    assert gene_of(genome, "features.declarations[0].lookback").value == LEGAL_LOOKBACK
    assert gene_of(genome, "selection.mode").value == "rank"
    assert edited.digest() != genome.digest()


def test_with_paths_refuses_a_recorded_value_outside_its_bound() -> None:
    """A recorded value is checked against the bound, like any value a genome carries."""
    genome = compiled_genome()

    with pytest.raises(ValueError, match="outside its bound"):
        genome.with_paths(["features.declarations[0].lookback"], [1000])


def test_a_mutation_is_deterministic_under_its_seed() -> None:
    """The same seed draws the same values and produces the same genome, whatever it selects."""
    genome = compiled_genome()
    paths = ["selection.percentile", "portfolio.max_position_weight"]

    first = mutate(genome, genes=paths, rng=seeded_generator(11))
    second = mutate(genome, genes=paths, rng=seeded_generator(11))
    assert first.genome is not None
    assert second.genome is not None

    assert first.genome.digest() == second.genome.digest()
    assert [change.after for change in first.changes] == [change.after for change in second.changes]
    assert first.genome.digest() != genome.digest()


def test_a_mutation_draws_inside_the_bound_and_keeps_the_declared_type() -> None:
    """A drawn parameter stays inside its bound and keeps the type its field declares."""
    genome = compiled_genome()

    for path in genome.parameter_paths():
        gene = gene_of(genome, path)
        edit = mutate(genome, genes=[path], rng=seeded_generator(13))
        assert edit.genome is not None
        assert gene.bound is not None

        after = gene_of(edit.genome, path).value
        assert gene.bound.admits(Decimal(after))
        assert isinstance(after, int) == isinstance(gene.value, int)


def test_a_mutation_records_both_values_and_the_parameter_degree() -> None:
    """A parameter change reports the path, the degree, the value held and the value drawn."""
    genome = compiled_genome()

    edit = mutate(genome, genes=["selection.percentile"], rng=seeded_generator(17))
    assert edit.genome is not None

    change = edit.changes[0]
    assert change.path == "selection.percentile"
    assert change.name == "selection.percentile"
    assert change.degree is GeneticDegree.PARAMETER
    assert change.before == gene_of(genome, "selection.percentile").value
    assert change.after == gene_of(edit.genome, "selection.percentile").value
    assert change.after != change.before
    assert edit.degrees == frozenset({GeneticDegree.PARAMETER})
    assert edit.requires_new_hypothesis is False


def test_a_structural_mutation_selects_another_declared_alternative() -> None:
    """A structural change takes a different member of the declared set, and reports it."""
    genome = compiled_genome()

    edit = mutate(genome, genes=["selection.mode", "data.decision_time"], rng=seeded_generator(19))
    assert edit.genome is not None

    for change in edit.changes:
        assert change.degree is GeneticDegree.STRUCTURAL
        assert change.after in gene_of(genome, change.path).declared
        assert change.after != change.before
        assert gene_of(edit.genome, change.path).value == change.after

    assert edit.degrees == frozenset({GeneticDegree.STRUCTURAL})
    assert edit.requires_new_hypothesis is False


def test_a_parameter_and_a_structural_mutation_leave_the_hypothesis_alone() -> None:
    """Neither of the two degrees that stand under a mechanism touches its statements."""
    genome = compiled_genome()

    parameter = mutate(genome, genes=["portfolio.position_count"], rng=seeded_generator(23))
    structural = mutate(genome, genes=["rebalance.frequency"], rng=seeded_generator(23))
    assert parameter.genome is not None
    assert structural.genome is not None

    assert parameter.genome.mechanism == genome.mechanism
    assert structural.genome.mechanism == genome.mechanism
    assert parameter.genome.family == genome.family
    assert structural.genome.family == genome.family
    assert parameter.requires_new_hypothesis is False
    assert structural.requires_new_hypothesis is False


def test_a_mutation_refuses_a_path_it_cannot_resolve() -> None:
    """A path that is not a gene stops the whole edit rather than being skipped."""
    genome = compiled_genome()

    edit = mutate(genome, genes=["no.such.path"], rng=seeded_generator(29))
    assert edit.genome is None
    assert edit.changes == ()
    assert edit.refusal is GenomeRefusal.GENE_UNRESOLVED
    assert "no.such.path" in edit.reason


def test_a_mutation_refuses_the_mechanism_path() -> None:
    """A statement path is not a gene, and it is refused as a mechanism rather than as unknown."""
    genome = compiled_genome()

    edit = mutate(genome, genes=["identity.mechanism"], rng=seeded_generator(29))
    assert edit.genome is None
    assert edit.refusal is GenomeRefusal.GENE_NOT_MUTABLE
    assert "identity.mechanism" in edit.reason


def test_a_mutation_refuses_an_empty_selection() -> None:
    """Naming no gene is a refusal, not an edit that changed nothing."""
    genome = compiled_genome()

    edit = mutate(genome, genes=[], rng=seeded_generator(29))
    assert edit.genome is None
    assert edit.changes == ()
    assert edit.refusal is GenomeRefusal.GENE_UNRESOLVED
    assert "no gene" in edit.reason


def test_a_mutation_refuses_a_gene_with_no_other_admissible_value() -> None:
    """A single-member declared set and a degenerate range both leave a gene nothing to take."""
    genome = compiled_genome()

    singleton = mutate(genome, genes=["data.signal_frequency"], rng=seeded_generator(29))
    assert singleton.refusal is GenomeRefusal.NO_FREE_GENE
    assert singleton.genome is None

    # A range bound whose limits coincide admits only the value already held.
    policy = replace(
        fixture_policy(),
        bounds=(Bound("feature.skip", BoundKind.RANGE, lower=Decimal(0), upper=Decimal(0)),),
    )
    pinned = genome_of(genome.specification, policy)
    degenerate = mutate(pinned, genes=["features.declarations[0].skip"], rng=seeded_generator(29))
    assert degenerate.refusal is GenomeRefusal.NO_FREE_GENE
    assert degenerate.genome is None


def test_with_mechanism_replaces_the_statements_and_the_family() -> None:
    """A family change carries the replacement pair, and it is the degree that needs one."""
    genome = compiled_genome()

    edit = with_mechanism(genome, REPLACEMENT)
    assert edit.genome is not None

    assert edit.genome.family == REPLACEMENT.family
    assert edit.genome.mechanism == REPLACEMENT
    assert edit.degrees == frozenset({GeneticDegree.FAMILY})
    assert edit.requires_new_hypothesis is True

    # The old statements are recorded as the values replaced, not silently discarded.
    assert {change.path: change.before for change in edit.changes} == {
        "identity.family": genome.mechanism.family,
        "identity.mechanism": genome.mechanism.mechanism,
        "identity.hypothesis": genome.mechanism.hypothesis,
        "identity.falsification": genome.mechanism.falsification,
    }
    assert all(change.degree is GeneticDegree.FAMILY for change in edit.changes)


def test_with_mechanism_records_the_candidate_it_replaced() -> None:
    """The parent defaults to the replaced genome's digest, and the caller may name others."""
    genome = compiled_genome()

    default = with_mechanism(genome, REPLACEMENT)
    assert default.genome is not None
    assert default.genome.parents == (genome.digest(),)

    declared = with_mechanism(genome, REPLACEMENT, parents=("sha256:parent",))
    assert declared.genome is not None
    assert declared.genome.parents == ("sha256:parent",)

    # The genome that was replaced is itself untouched.
    assert genome.family == "cross_sectional_rank"
    assert genome.mechanism.family == "cross_sectional_rank"


def test_with_mechanism_refuses_an_incomplete_statement() -> None:
    """A blank statement field is refused rather than carried into the identity block."""
    genome = compiled_genome()
    incomplete = replace(REPLACEMENT, hypothesis="   ")

    edit = with_mechanism(genome, incomplete)
    assert edit.genome is None
    assert edit.refusal is GenomeRefusal.VALUE_NOT_DECLARED
    assert "hypothesis" in edit.reason


def test_with_mechanism_refuses_an_undeclared_family() -> None:
    """A family the policy does not declare is refused rather than accepted as a new one."""
    genome = compiled_genome()
    undeclared = replace(REPLACEMENT, family="unregistered_family")

    edit = with_mechanism(genome, undeclared)
    assert edit.genome is None
    assert edit.refusal is GenomeRefusal.VALUE_NOT_DECLARED
    assert "unregistered_family" in edit.reason


def test_with_mechanism_refuses_the_mechanism_the_genome_already_carries() -> None:
    """Replacing a mechanism with itself changes nothing, so it is refused as no free gene."""
    genome = compiled_genome()

    edit = with_mechanism(genome, genome.mechanism)
    assert edit.genome is None
    assert edit.changes == ()
    assert edit.refusal is GenomeRefusal.NO_FREE_GENE


def test_recombination_exchanges_a_shared_gene_within_a_family() -> None:
    """The donor's value at a shared path replaces the receiver's, and both parents survive."""
    genome = compiled_genome()
    donor = genome.with_paths(["selection.tie_break"], ["instrument_id_asc"])
    receiver_digest = genome.digest()

    edit = recombine(genome, donor, rng=seeded_generator(31))
    assert edit.genome is not None

    assert [change.path for change in edit.changes] == ["selection.tie_break"]
    assert edit.changes[0].before == "feature_desc"
    assert edit.changes[0].after == "instrument_id_asc"
    assert edit.changes[0].degree is GeneticDegree.STRUCTURAL
    assert edit.genome.digest() == donor.digest()

    # Neither parent is changed in place.
    assert genome.digest() == receiver_digest
    assert gene_of(genome, "selection.tie_break").value == "feature_desc"
    assert gene_of(donor, "selection.tie_break").value == "instrument_id_asc"


def test_recombination_is_deterministic_and_leaves_both_parents_alone() -> None:
    """The same seed draws the same split, and the exchange copies rather than edits."""
    genome = compiled_genome()
    donor = genome.with_paths(
        ["portfolio.cash_policy", "selection.tie_break"],
        ["reallocate", "instrument_id_asc"],
    )
    receiver_digest = genome.digest()
    donor_digest = donor.digest()

    first = recombine(genome, donor, rng=seeded_generator(37))
    second = recombine(genome, donor, rng=seeded_generator(37))
    assert first.genome is not None
    assert second.genome is not None

    assert first.genome.digest() == second.genome.digest()
    assert first.genome.digest() != receiver_digest

    changed = {change.path for change in first.changes}
    assert changed != set()
    assert changed <= {"portfolio.cash_policy", "selection.tie_break"}
    assert all(change.after == gene_of(donor, change.path).value for change in first.changes)

    assert genome.digest() == receiver_digest
    assert donor.digest() == donor_digest


def test_recombination_refuses_two_families_and_two_identical_parents() -> None:
    """Crossing mechanisms is refused, and so is a crossing that has nothing to exchange."""
    genome = compiled_genome()
    other = with_mechanism(genome, OTHER_FAMILY).genome
    assert other is not None

    cross = recombine(genome, other, rng=seeded_generator(41))
    assert cross.genome is None
    assert cross.refusal is GenomeRefusal.FAMILY_MISMATCH
    assert "cross_sectional_rank" in cross.reason
    assert "static_hold" in cross.reason

    identical = recombine(genome, genome, rng=seeded_generator(41))
    assert identical.genome is None
    assert identical.refusal is GenomeRefusal.NO_FREE_GENE


def test_a_mutated_genome_still_compiles_with_its_digest_reported() -> None:
    """A mutation produces a specification the compiler accepts under the digest it reports."""
    policy, registry, genome = compile_fixture()

    edit = mutate(
        genome,
        genes=["features.declarations[0].lookback", "selection.percentile"],
        rng=seeded_generator(43),
    )
    assert edit.genome is not None

    result = compile_specification(edit.genome.specification.to_dict(), policy, registry)
    assert result.accepted
    assert result.plan is not None
    assert result.plan.specification_digest == edit.genome.digest()
    assert result.plan.specification.to_dict() == edit.genome.specification.to_dict()
