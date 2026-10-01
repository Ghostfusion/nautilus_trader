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
Search strategies that enumerate experiments from a parameter space.

Search and execution are separate concerns: a strategy only enumerates parameter sets, and a
runner executes one. A strategy never runs a backtest and never sees a result.

A random search draws a fixed subset of the space and an evolutionary search breeds from the
evaluations a driver records. Both are deterministic under their seed: the same seed, space and
evaluation record always yield the same sequence of candidates.
"""

from __future__ import annotations

import math
import random
from dataclasses import dataclass
from dataclasses import field
from typing import TYPE_CHECKING
from typing import Protocol
from typing import runtime_checkable


if TYPE_CHECKING:
    from collections.abc import Iterator
    from collections.abc import Sequence

    from nautilus_trader.optimization.space import Experiment
    from nautilus_trader.optimization.space import ParameterSpace


# A crossover takes each gene from the first parent or the second with equal probability.
_CROSSOVER_SPLIT = 0.5


class EvaluationLike(Protocol):
    """
    An evaluated experiment as a search reads it.

    A search never computes a score; it reads the feasibility and the objective value the driver
    recorded for a parameter digest. Feasibility is a separate attribute rather than a penalty in
    the score, so an infeasible candidate is never traded off against a feasible one.
    """

    score: float
    feasible: bool


class EvaluationSource(Protocol):
    """
    The record of evaluations a search consults while it enumerates.

    The driver writes an evaluation before the search resumes, so an adaptive search sees the
    outcome of the individual it just yielded and can select the next generation from it.
    """

    def get(self, digest: str) -> EvaluationLike | None:
        """
        Return the evaluation recorded for a parameter digest, or None when it has none.

        Parameters
        ----------
        digest : str
            The canonical parameter digest.

        Returns
        -------
        EvaluationLike | None

        """
        ...


@runtime_checkable
class SearchStrategy(Protocol):
    """
    A strategy that enumerates the experiments to evaluate.

    Implementations are pure enumeration: they take a parameter space and yield experiments in a
    deterministic order. They do not execute runs.
    """

    def experiments(self, space: ParameterSpace) -> Iterator[Experiment]:
        """
        Yield the experiments to evaluate for the given space.

        Parameters
        ----------
        space : ParameterSpace
            The parameter space to enumerate.

        Yields
        ------
        Experiment

        """
        ...


class GridSearch:
    """
    A search strategy that evaluates every point of the parameter space.

    The enumeration is the space's own deterministic expansion, so a grid sweep is reproducible.
    """

    def experiments(self, space: ParameterSpace) -> Iterator[Experiment]:
        """
        Yield every experiment of the given space.

        Parameters
        ----------
        space : ParameterSpace
            The space to expand.

        Yields
        ------
        Experiment

        """
        yield from space.expand()


@dataclass(frozen=True)
class RandomSearch:
    """
    A search strategy that draws a seeded random subset of the parameter space.

    The subset is drawn without replacement from the space's mixed-radix positions, so the same
    seed and space always select the same experiments in the same order. A budget smaller than the
    space evaluates that many experiments; a budget at or above the space evaluates all of them.

    Parameters
    ----------
    seed : int
        The generator seed the draw is derived from.
    budget : int | None, default None
        The maximum number of experiments to draw, or None for the whole space.

    """

    seed: int
    budget: int | None = None

    def __post_init__(self) -> None:
        """
        Validate the budget.
        """
        if self.budget is not None and self.budget < 1:
            raise ValueError(f"budget must be at least 1 when set, was {self.budget}")

    def experiments(self, space: ParameterSpace) -> Iterator[Experiment]:
        """
        Yield the seeded random subset of the given space.

        Parameters
        ----------
        space : ParameterSpace
            The space to sample.

        Yields
        ------
        Experiment

        """
        total = space.size
        count = total if self.budget is None else min(self.budget, total)
        rng = random.Random(self.seed)  # noqa: S311 (a deterministic search RNG, not security)
        for index in rng.sample(range(total), count):
            yield space.experiment_at(index)


@dataclass(frozen=True)
class EvolutionaryOperators:
    """
    The evolutionary operators and their rates.

    The population is ranked by feasibility and then by score, so an infeasible individual is never
    selected as a parent while a feasible one exists. Elite survivors are copied unchanged, parents
    are drawn by tournament, and each child is crossed over and mutated with the given rates.

    Parameters
    ----------
    population_size : int, default 20
        The number of individuals in a generation.
    generations : int, default 5
        The number of generations to breed.
    elite : int, default 1
        The number of top-ranked individuals carried over unchanged.
    tournament_size : int, default 2
        The number of contestants in each parent tournament.
    crossover_rate : float, default 0.5
        The probability a child is crossed over rather than copied from one parent.
    mutation_rate : float, default 0.1
        The per-gene probability of replacing a choice with a random one.

    """

    population_size: int = 20
    generations: int = 5
    elite: int = 1
    tournament_size: int = 2
    crossover_rate: float = 0.5
    mutation_rate: float = 0.1

    def __post_init__(self) -> None:
        """
        Validate the operators.
        """
        if self.population_size < 1:
            raise ValueError(f"population_size must be at least 1, was {self.population_size}")
        if self.generations < 1:
            raise ValueError(f"generations must be at least 1, was {self.generations}")
        if not 0 <= self.elite <= self.population_size:
            raise ValueError(f"elite must be in [0, population_size], was {self.elite}")
        if self.tournament_size < 1:
            raise ValueError(f"tournament_size must be at least 1, was {self.tournament_size}")
        if not 0.0 <= self.crossover_rate <= 1.0:
            raise ValueError(f"crossover_rate must be in [0, 1], was {self.crossover_rate}")
        if not 0.0 <= self.mutation_rate <= 1.0:
            raise ValueError(f"mutation_rate must be in [0, 1], was {self.mutation_rate}")


@dataclass(frozen=True)
class EvolutionarySearch:
    """
    A search strategy that breeds generations from a seeded generator and the recorded evaluations.

    Each generation yields its individuals; the driver evaluates them and records the outcomes in
    the evaluation source before the generation completes, so the next generation is selected from
    the recorded feasibility and score. The generator never computes a score and never treats a
    constraint as a penalty: it reads `feasible` as a separate attribute.

    Parameters
    ----------
    evaluations : EvaluationSource
        The record of evaluations, written by the driver between yields.
    seed : int, default 0
        The generator seed the search is derived from.
    operators : EvolutionaryOperators, default EvolutionaryOperators()
        The operators and their rates.
    budget : int | None, default None
        The maximum number of individuals to yield, or None for the whole space.

    """

    evaluations: EvaluationSource = field(compare=False, repr=False)
    seed: int = 0
    operators: EvolutionaryOperators = field(default_factory=EvolutionaryOperators)
    budget: int | None = None

    def __post_init__(self) -> None:
        """
        Validate the budget.
        """
        if self.budget is not None and self.budget < 1:
            raise ValueError(f"budget must be at least 1 when set, was {self.budget}")

    def experiments(self, space: ParameterSpace) -> Iterator[Experiment]:
        """
        Yield the individuals of every generation, breeding from the recorded evaluations.

        Parameters
        ----------
        space : ParameterSpace
            The space to search.

        Yields
        ------
        Experiment

        """
        rng = random.Random(self.seed)  # noqa: S311 (a deterministic search RNG, not security)
        operators = self.operators
        total = space.size
        budget = total if self.budget is None else min(self.budget, total)
        population = self._initial_population(rng, space, total)
        yielded = 0

        for _ in range(operators.generations):
            for indices in population:
                if yielded >= budget:
                    return
                yield space.experiment_from_indices(indices)
                yielded += 1
            if yielded >= budget:
                return
            population = self._breed(rng, space, population)

    def _initial_population(
        self,
        rng: random.Random,
        space: ParameterSpace,
        total: int,
    ) -> list[tuple[int, ...]]:
        """
        Return the distinct seeded starting individuals.
        """
        count = min(self.operators.population_size, total)
        return [space.indices_at(index) for index in rng.sample(range(total), count)]

    def _breed(
        self,
        rng: random.Random,
        space: ParameterSpace,
        population: Sequence[tuple[int, ...]],
    ) -> list[tuple[int, ...]]:
        """
        Return the next generation: the elite, then crossed-over and mutated children.
        """
        operators = self.operators
        ranked = sorted(
            population,
            key=lambda indices: self._rank(self._fitness(space, indices)),
            reverse=True,
        )
        offspring = list(ranked[: operators.elite])
        while len(offspring) < operators.population_size:
            first = self._select(rng, space, population)
            second = self._select(rng, space, population)
            child = self._crossover(rng, first, second)
            offspring.append(self._mutate(rng, space, child))
        return offspring[: operators.population_size]

    def _select(
        self,
        rng: random.Random,
        space: ParameterSpace,
        population: Sequence[tuple[int, ...]],
    ) -> tuple[int, ...]:
        """
        Return one parent by tournament over feasibility and score.
        """
        contestants = rng.sample(
            range(len(population)),
            min(self.operators.tournament_size, len(population)),
        )
        best = max(
            contestants,
            key=lambda i: self._rank(self._fitness(space, population[i])),
        )
        return population[best]

    def _crossover(
        self,
        rng: random.Random,
        first: tuple[int, ...],
        second: tuple[int, ...],
    ) -> tuple[int, ...]:
        """
        Return a child of two parents, cross-bred with the declared rate.
        """
        if rng.random() >= self.operators.crossover_rate:
            return first
        return tuple(
            a if rng.random() < _CROSSOVER_SPLIT else b for a, b in zip(first, second, strict=True)
        )

    def _mutate(
        self,
        rng: random.Random,
        space: ParameterSpace,
        indices: tuple[int, ...],
    ) -> tuple[int, ...]:
        """
        Return an individual with each gene replaced by chance with a random choice.
        """
        genes = list(indices)
        for position, parameter in enumerate(space.parameters):
            if rng.random() < self.operators.mutation_rate:
                genes[position] = rng.randrange(len(parameter.choices))
        return tuple(genes)

    def _fitness(self, space: ParameterSpace, indices: tuple[int, ...]) -> float:
        """
        Return the recorded objective value, or NaN when the individual is infeasible or unseen.

        Feasibility is read as a separate attribute: an infeasible individual has no fitness and
        is never selected, rather than carrying a score with a penalty added to it.
        """
        evaluation = self.evaluations.get(space.experiment_from_indices(indices).digest)
        if evaluation is None or not evaluation.feasible:
            return math.nan
        return evaluation.score

    @staticmethod
    def _rank(score: float) -> tuple[int, float]:
        """
        Return a comparable rank that puts every feasible individual above every infeasible one.
        """
        if math.isnan(score):
            return (0, 0.0)
        return (1, score)
