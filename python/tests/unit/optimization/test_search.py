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
Tests for the random and evolutionary searches, the evaluation cache and the run description.
"""

from __future__ import annotations

import math
from pathlib import Path
from typing import TYPE_CHECKING

import pytest

from nautilus_trader.analysis import Constraint
from nautilus_trader.analysis import ConstraintComparison
from nautilus_trader.analysis import Objective
from nautilus_trader.analysis import ObjectiveDirection
from nautilus_trader.analysis import ObjectiveTerm
from nautilus_trader.optimization.concurrency import ConcurrencyPolicy
from nautilus_trader.optimization.optimizer import Optimizer
from nautilus_trader.optimization.persistence import Evaluation
from nautilus_trader.optimization.persistence import EvaluationCache
from nautilus_trader.optimization.persistence import ExperimentStore
from nautilus_trader.optimization.run import RunDescription
from nautilus_trader.optimization.run import ValidationMode
from nautilus_trader.optimization.run import ValidationScheme
from nautilus_trader.optimization.runner import CanonicalRun
from nautilus_trader.optimization.search import EvolutionaryOperators
from nautilus_trader.optimization.search import EvolutionarySearch
from nautilus_trader.optimization.search import GridSearch
from nautilus_trader.optimization.search import RandomSearch
from nautilus_trader.optimization.search import TrialSpecification
from nautilus_trader.optimization.space import Parameter
from nautilus_trader.optimization.space import ParameterSpace


if TYPE_CHECKING:
    from collections.abc import Iterable

    from nautilus_trader.optimization.runner import RunOutcome
    from nautilus_trader.optimization.search import SearchStrategy
    from nautilus_trader.optimization.space import Experiment


SCORE = "Sharpe Ratio (simple, sample, 252 days)"
GATE = "Max Drawdown (simple)"

SEARCH_WINDOW = (1_000, 2_000)
HELD_OUT_WINDOW = (2_000, 3_000)


class FakeRunner:
    """
    Execute an experiment as a pure function of its parameters, recording every run.

    The score is `x * 10 + y` and the gate is `x`. When `hide_score_above` is set, the score is
    omitted for any `x` above it, so an experiment can be feasible and scored or infeasible and
    unscoreable.
    """

    def __init__(
        self,
        ledger: list[str] | None = None,
        hide_score_above: int | None = None,
        start: int | None = None,
        end: int | None = None,
    ) -> None:
        """
        Initialize the fake runner with an optional shared ledger, score gate and run window.
        """
        self.ledger: list[str] = [] if ledger is None else ledger
        self.hide_score_above = hide_score_above
        self.start = start
        self.end = end

    def windowed(self, start: int | None, end: int | None) -> FakeRunner:
        """
        Return a copy restricted to the given window, sharing the ledger.
        """
        return FakeRunner(self.ledger, self.hide_score_above, start, end)

    def run(self, experiment: Experiment, _metrics: Iterable[str] | None = None) -> RunOutcome:
        """
        Record the run and return a canonical run with the derived metric values.
        """
        self.ledger.append(experiment.digest)
        canonical = experiment.canonical.encode("utf-8")
        x = int(experiment.parameters["x"])
        y = int(experiment.parameters["y"])
        values = {GATE: float(x), SCORE: float(x * 10 + y)}
        if self.hide_score_above is not None and x > self.hide_score_above:
            values.pop(SCORE)
        return CanonicalRun(experiment, experiment.digest, values, canonical)


def _space() -> ParameterSpace:
    """
    Build a two-parameter space of twelve experiments.
    """
    return ParameterSpace(
        base={"mode": "test"},
        parameters=(
            Parameter("x", (0, 1, 2)),
            Parameter("y", (0, 1, 2, 3)),
        ),
    )


def _objective() -> Objective:
    """
    Build the maximize-score objective.
    """
    return Objective((ObjectiveTerm(SCORE, 1.0, ObjectiveDirection.MAXIMIZE),))


def _scorer(experiment: Experiment) -> float:
    """
    Return the score the fake runner derives from an experiment's parameters.
    """
    return float(int(experiment.parameters["x"]) * 10 + int(experiment.parameters["y"]))


def _feasible(experiment: Experiment) -> bool:
    """
    Return the feasibility the gate constraint derives from an experiment's parameters.
    """
    return int(experiment.parameters["x"]) <= 1


def _optimizer(
    runner: FakeRunner,
    *,
    search: SearchStrategy | None = None,
    constraints: tuple[Constraint, ...] = (),
) -> Optimizer:
    """
    Build an optimizer over the fake runner.
    """
    return Optimizer(
        runner,
        _objective(),
        constraints,
        ConcurrencyPolicy.sequential(),
        search if search is not None else GridSearch(),
    )


def test_experiment_at_matches_the_deterministic_expansion_order() -> None:
    """
    Test a mixed-radix position maps to the same experiment the sweep reaches.
    """
    space = _space()
    expanded = list(space.expand())
    assert space.size == len(expanded)
    for index in range(space.size):
        assert space.experiment_at(index).canonical == expanded[index].canonical


def test_random_search_is_deterministic_under_a_seed() -> None:
    """
    Test a seeded random search yields the same candidates in the same order.
    """
    space = _space()
    first = list(RandomSearch(seed=11, budget=5).experiments(space))
    second = list(RandomSearch(seed=11, budget=5).experiments(space))
    other = list(RandomSearch(seed=12, budget=5).experiments(space))

    assert len(first) == 5
    assert [experiment.digest for experiment in first] == [
        experiment.digest for experiment in second
    ]
    assert [experiment.digest for experiment in first] != [
        experiment.digest for experiment in other
    ]


def test_seeded_random_run_reproduces_the_best_result() -> None:
    """
    Test two seeded random runs report the same best result.
    """
    space = _space()
    search = RandomSearch(seed=3, budget=8)
    first = _optimizer(FakeRunner(), search=search).optimize(space, cache=EvaluationCache())
    second = _optimizer(FakeRunner(), search=RandomSearch(seed=3, budget=8)).optimize(
        space,
        cache=EvaluationCache(),
    )

    assert first.best() is not None
    assert second.best() is not None
    assert first.best().digest == second.best().digest
    assert first.best().score == second.best().score
    assert first.digests == second.digests


def test_seeded_evolutionary_run_reproduces_the_candidate_sequence_and_best() -> None:
    """
    Test two seeded evolutionary runs evaluate the same candidates and report the same best.
    """
    space = _space()
    operators = EvolutionaryOperators(
        population_size=4,
        generations=4,
        elite=1,
        tournament_size=2,
        crossover_rate=0.5,
        mutation_rate=0.25,
    )
    first_ledger: list[str] = []
    second_ledger: list[str] = []
    first_search = EvolutionarySearch(EvaluationCache(), seed=5, operators=operators, budget=10)
    second_search = EvolutionarySearch(EvaluationCache(), seed=5, operators=operators, budget=10)

    first = _optimizer(FakeRunner(first_ledger), search=first_search).optimize(
        space,
        cache=first_search.evaluations,
    )
    second = _optimizer(FakeRunner(second_ledger), search=second_search).optimize(
        space,
        cache=second_search.evaluations,
    )

    assert first_ledger == second_ledger
    assert first.evaluated == second.evaluated
    assert first.best() is not None
    assert second.best() is not None
    assert first.best().digest == second.best().digest


def test_resumed_run_does_not_reevaluate_a_cached_vector(tmp_path: Path) -> None:
    """
    Test a second run over the same store reuses every evaluation instead of executing it.
    """
    space = _space()
    store = ExperimentStore(tmp_path / "store")

    first_ledger: list[str] = []
    first = _optimizer(FakeRunner(first_ledger), search=GridSearch()).optimize(space, store=store)
    assert first.executions == space.size
    assert first.evaluated == space.size

    second_ledger: list[str] = []
    second = _optimizer(FakeRunner(second_ledger), search=GridSearch()).optimize(space, store=store)
    assert second.executions == 0
    assert second.evaluated == space.size
    assert second_ledger == []
    assert second.best() is not None
    assert first.best() is not None
    assert second.best().digest == first.best().digest


def test_budget_smaller_than_the_space_reports_the_best_of_what_it_evaluated() -> None:
    """
    Test a budgeted search reports the best of its evaluated subset, not the whole space.
    """
    space = _space()
    budget = 5
    report = _optimizer(FakeRunner(), search=RandomSearch(seed=1, budget=budget)).optimize(space)

    assert report.evaluated == budget
    assert report.space_size == space.size
    assert report.evaluated_fraction == pytest.approx(budget / space.size)
    assert report.best() is not None

    expected = max(_scorer(result.experiment) for result in report.results)
    assert report.best().score == expected
    assert report.best().score == _scorer(report.best().experiment)


def test_infeasible_candidate_is_recorded_as_infeasible_not_as_a_failure() -> None:
    """
    Test a constraint is checked before the objective, so an infeasible unscoreable run is kept.
    """
    space = _space()
    constraint = Constraint(GATE, ConstraintComparison.AT_MOST, 1.0)
    report = _optimizer(
        FakeRunner(hide_score_above=1),
        constraints=(constraint,),
    ).optimize(space)

    assert report.failures == ()
    assert report.best_feasible() is not None
    assert report.best_feasible().constraints_satisfied

    infeasible = [result for result in report.results if not _feasible(result.experiment)]
    assert infeasible
    assert all(not result.constraints_satisfied for result in infeasible)
    assert all(not math.isfinite(result.score) for result in infeasible)


def test_evolutionary_search_selects_on_feasibility_not_a_penalty() -> None:
    """
    Test the elite survives on feasibility even when an infeasible candidate scores higher.

    The search is driven by hand so the generation boundary is visible: the first generation is
    seeded randomly, and with crossover and mutation off every later generation copies the elite.
    An infeasible candidate in the first generation scores strictly higher than every feasible one,
    so a penalty-then-rank search would keep the infeasible candidate; a feasibility-first search
    keeps a feasible one.
    """
    space = _space()

    class Source:
        def __init__(self) -> None:
            self.entries: dict[str, Evaluation] = {}

        def get(self, digest: str) -> Evaluation | None:
            return self.entries.get(digest)

        def put(self, evaluation: Evaluation) -> None:
            self.entries[evaluation.digest] = evaluation

    source = Source()
    operators = EvolutionaryOperators(
        population_size=4,
        generations=3,
        elite=1,
        tournament_size=2,
        crossover_rate=0.0,
        mutation_rate=0.0,
    )
    search = EvolutionarySearch(source, seed=7, operators=operators)

    candidates = search.experiments(space)
    first_generation = [next(candidates) for _ in range(operators.population_size)]

    # One first-generation individual is recorded as infeasible with a score far above every
    # feasible one, so a penalty-then-rank search would carry it into the next generation.
    infeasible = first_generation[0]
    source.put(Evaluation(CanonicalRun(infeasible, infeasible.digest, {}, b""), 1000.0, False))
    for experiment in first_generation[1:]:
        outcome = CanonicalRun(experiment, experiment.digest, {}, b"")
        source.put(Evaluation(outcome, _scorer(experiment), True))

    later_generations = [next(candidates) for _ in range(operators.population_size)]
    assert all(
        source.get(experiment.digest).score < 1000.0  # type: ignore[union-attr]
        for experiment in later_generations
    )
    assert all(
        source.get(experiment.digest).feasible  # type: ignore[union-attr]
        for experiment in later_generations
    )


def test_validation_scheme_refuses_a_held_out_window_that_overlaps_search() -> None:
    """
    Test a held-out window that overlaps the search window is refused.
    """
    with pytest.raises(ValueError, match="overlaps search window"):
        ValidationScheme.single_split(SEARCH_WINDOW, (1_500, 2_500))


def test_validation_scheme_allows_adjacent_windows() -> None:
    """
    Test adjacent search and held-out windows do not overlap.
    """
    scheme = ValidationScheme.single_split(SEARCH_WINDOW, HELD_OUT_WINDOW)

    assert scheme.mode is ValidationMode.SINGLE_SPLIT
    assert scheme.search_windows == (SEARCH_WINDOW,)
    assert scheme.held_out_windows == (HELD_OUT_WINDOW,)
    assert ValidationScheme.from_dict(scheme.to_dict()) == scheme


def test_report_records_the_windows_the_search_ran_under() -> None:
    """
    Test the report carries the validation scheme that distinguishes search from held out.
    """
    space = _space()
    scheme = ValidationScheme.single_split(SEARCH_WINDOW, HELD_OUT_WINDOW)
    report = _optimizer(
        FakeRunner(start=1_500, end=1_800),
        search=RandomSearch(seed=2, budget=4),
    ).optimize(
        space,
        cache=EvaluationCache(),
        scheme=scheme,
    )

    assert report.scheme == scheme
    assert report.seed == 2


def test_optimizer_refuses_a_scheme_that_does_not_cover_the_run_window() -> None:
    """
    Test a scheme whose search windows do not cover the runner's window is refused.
    """
    space = _space()
    scheme = ValidationScheme.single_split(SEARCH_WINDOW)

    with pytest.raises(ValueError, match="not inside any of the scheme's search windows"):
        _optimizer(FakeRunner(start=5_000, end=6_000), search=GridSearch()).optimize(
            space,
            scheme=scheme,
        )


def test_optimizer_refuses_a_run_scored_in_a_held_out_window() -> None:
    """
    Test a run whose window falls in the scheme's held-out window is refused.
    """
    space = _space()
    scheme = ValidationScheme.single_split(SEARCH_WINDOW, HELD_OUT_WINDOW)

    with pytest.raises(ValueError, match="held-out window"):
        _optimizer(FakeRunner(start=2_500, end=2_800), search=GridSearch()).optimize(
            space,
            scheme=scheme,
        )


def test_optimizer_allows_a_run_inside_a_declared_search_window() -> None:
    """
    Test a run inside a declared search window still optimizes normally.
    """
    space = _space()
    scheme = ValidationScheme.single_split(SEARCH_WINDOW, HELD_OUT_WINDOW)
    report = _optimizer(FakeRunner(start=1_200, end=1_800), search=GridSearch()).optimize(
        space,
        scheme=scheme,
    )

    assert report.best() is not None
    assert report.scheme == scheme
    assert report.evaluated == space.size


def test_run_description_records_the_seed_the_operators_and_the_cache() -> None:
    """
    Test the run description names the seed, the evolutionary operators and the cache digest.
    """
    space = _space()
    scheme = ValidationScheme.single_split(SEARCH_WINDOW, HELD_OUT_WINDOW)
    cache = EvaluationCache()
    operators = EvolutionaryOperators(population_size=3, generations=2)
    search = EvolutionarySearch(cache, seed=13, operators=operators)

    description = RunDescription.of(space, scheme, search=search, cache=cache)

    assert description.search_strategy == "EvolutionarySearch"
    assert description.operators == operators
    assert description.cache_digest == cache.digest
    assert description.digest == RunDescription.of(space, scheme, search=search, cache=cache).digest


def test_run_description_digest_changes_with_the_seed() -> None:
    """
    Test a different seed declares a different run.
    """
    space = _space()
    scheme = ValidationScheme.single_split(SEARCH_WINDOW, HELD_OUT_WINDOW)

    first = RunDescription.of(space, scheme, search=RandomSearch(seed=1, budget=4))
    second = RunDescription.of(space, scheme, search=RandomSearch(seed=2, budget=4))

    assert first.digest != second.digest


class SpecificationRunner:
    """
    Execute an experiment without reading its parameters, so the specification can be the point.

    The score and the gate are constant, so the ranking is deterministic and the test is only
    about the specification each trial records.
    """

    def __init__(self, start: int | None = None, end: int | None = None) -> None:
        """
        Initialize the runner with an optional run window.
        """
        self.start = start
        self.end = end

    def windowed(self, start: int | None, end: int | None) -> SpecificationRunner:
        """
        Return a copy restricted to the given window.
        """
        return SpecificationRunner(start, end)

    def run(self, experiment: Experiment, _metrics: Iterable[str] | None = None) -> RunOutcome:
        """
        Return a canonical run with constant values.
        """
        values = {SCORE: 1.0, GATE: 0.0}
        canonical = experiment.canonical.encode("utf-8")
        return CanonicalRun(experiment, experiment.digest, values, canonical)


def _specification_space() -> ParameterSpace:
    """
    Build the three-weighting by two-window space the specification spread is read over.
    """
    return ParameterSpace(
        base={"mode": "test"},
        parameters=(
            Parameter("weighting", ("equal", "value", "rank")),
            Parameter("window", (1_000, 2_000)),
        ),
    )


def _specification_of(experiment: Experiment) -> TrialSpecification:
    """
    Read the specification a trial ran under from its experiment.
    """
    window = int(experiment.parameters["window"])
    return TrialSpecification(
        data_window=(window, window + 500),
        universe_rule="top_500_by_capitalisation",
        weighting=str(experiment.parameters["weighting"]),
        adjustment_model="four_factor",
        exclusions=("financials",),
    )


def _specification_optimizer() -> Optimizer:
    """
    Build an optimizer that records each trial's specification from its experiment.
    """
    return Optimizer(
        SpecificationRunner(),
        _objective(),
        concurrency=ConcurrencyPolicy.sequential(),
        specification=_specification_of,
    )


def test_a_search_over_three_weightings_and_two_windows_reports_a_spread_of_six() -> None:
    """
    Test the observed spread: a distinct specification is a distinct canonical mapping.

    The space sweeps three weightings and two windows, so its six trials are six distinct
    specifications and the report's spread is six.
    """
    report = _specification_optimizer().optimize(_specification_space())

    assert report.evaluated == 6
    assert report.specification_spread() == 6


def test_a_trials_specification_is_recoverable_from_the_report() -> None:
    """
    Test the caller reads a trial's specification back out of the report, not from memory.
    """
    report = _specification_optimizer().optimize(_specification_space())
    selected = next(
        result
        for result in report.results
        if result.experiment.parameters["weighting"] == "value"
        and result.experiment.parameters["window"] == 2_000
    )

    assert selected.specification == TrialSpecification(
        data_window=(2_000, 2_500),
        universe_rule="top_500_by_capitalisation",
        weighting="value",
        adjustment_model="four_factor",
        exclusions=("financials",),
    )


def test_a_report_without_specifications_has_an_unknown_spread() -> None:
    """
    Test a report whose trials recorded no specification returns None, not zero.

    Zero would assert that every trial shared one specification; an absent record establishes no
    such thing, so the spread is unknown.
    """
    report = _optimizer(FakeRunner()).optimize(_space())

    assert all(result.specification is None for result in report.results)
    assert report.specification_spread() is None


def test_two_specifications_are_distinct_exactly_when_their_mappings_differ() -> None:
    """
    Test the digest names the canonical mapping, so equal mappings share a digest.
    """
    equal = TrialSpecification(
        data_window=(1_000, 2_000),
        universe_rule="top_500_by_capitalisation",
        weighting="equal",
    )
    same = TrialSpecification(
        data_window=(1_000, 2_000),
        universe_rule="top_500_by_capitalisation",
        weighting="equal",
    )
    value = TrialSpecification(
        data_window=(1_000, 2_000),
        universe_rule="top_500_by_capitalisation",
        weighting="value",
    )

    assert equal.digest == same.digest
    assert equal.digest != value.digest
    assert TrialSpecification.from_dict(equal.to_dict()) == equal


def test_trial_specification_refuses_an_incomplete_declaration() -> None:
    """
    Test a specification the caller cannot state in full is refused rather than defaulted.
    """
    with pytest.raises(ValueError, match="universe_rule must not be empty"):
        TrialSpecification(data_window=(1_000, 2_000), universe_rule="", weighting="equal")

    with pytest.raises(TypeError, match="weighting must be a str"):
        TrialSpecification(  # type: ignore[arg-type]
            data_window=(1_000, 2_000),
            universe_rule="top_500_by_capitalisation",
            weighting=7,
        )

    with pytest.raises(ValueError, match="non-empty and increasing"):
        TrialSpecification(
            data_window=(2_000, 1_000),
            universe_rule="top_500_by_capitalisation",
            weighting="equal",
        )

    with pytest.raises(ValueError, match="exclusion name must not be empty"):
        TrialSpecification(
            data_window=(1_000, 2_000),
            universe_rule="top_500_by_capitalisation",
            weighting="equal",
            exclusions=("",),
        )
