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
Result aggregation for a sweep: scores, feasible ranking, and failures.

An `ExperimentResult` pairs a canonical run with the objective score and the constraint outcome,
and carries the specification the trial ran under. A `SearchReport` collects the survivors and the
failures. Ranking is deterministic: by descending score, with ties broken by canonical digest and
then by experiment digest, because the objective itself does not order equal scores. The report
states its specification spread, so a bound can be read against what the search actually varied
rather than assumed.
"""

from __future__ import annotations

import math
from dataclasses import dataclass
from typing import TYPE_CHECKING

from nautilus_trader.optimization.search import TrialSpecification


if TYPE_CHECKING:
    from collections.abc import Iterable

    from nautilus_trader.optimization.run import ValidationScheme
    from nautilus_trader.optimization.runner import CanonicalRun
    from nautilus_trader.optimization.runner import FailedExperiment
    from nautilus_trader.optimization.space import Experiment


@dataclass(frozen=True)
class ExperimentResult:
    """
    A scored, feasible-or-not experiment outcome.

    Parameters
    ----------
    run : CanonicalRun
        The canonical run.
    score : float
        The objective score, which the search maximises.
    constraints_satisfied : bool
        Whether every constraint holds for the run's metric values.
    specification : TrialSpecification | None, default None
        The specification the trial was run under, captured when the trial ran, or None when the
        caller recorded none. A trial whose specification is unknown has no specification to read
        a bound against, so it is recorded as absent rather than defaulted.

    """

    run: CanonicalRun
    score: float
    constraints_satisfied: bool
    specification: TrialSpecification | None = None

    def __post_init__(self) -> None:
        """
        Validate the recorded specification.
        """
        if self.specification is not None and not isinstance(
            self.specification, TrialSpecification
        ):
            raise TypeError("specification must be a TrialSpecification or None")

    @property
    def experiment(self) -> Experiment:
        """
        The experiment's parameter set.
        """
        return self.run.experiment

    @property
    def digest(self) -> str:
        """
        The canonical backtest result digest.
        """
        return self.run.canonical_digest


def rank_results(results: Iterable[ExperimentResult]) -> tuple[ExperimentResult, ...]:
    """
    Rank results by descending score with deterministic tie-breaking.

    A non-finite score ranks last. Ties break by canonical digest, then by experiment digest.

    Parameters
    ----------
    results : Iterable[ExperimentResult]
        The results to rank.

    Returns
    -------
    tuple[ExperimentResult, ...]
        The ranked results.

    """

    def key(result: ExperimentResult) -> tuple[float, str, str]:
        score = result.score if math.isfinite(result.score) else -math.inf
        return (-score, result.run.canonical_digest, result.run.experiment.digest)

    return tuple(sorted(results, key=key))


def sort_failures(
    failures: Iterable[FailedExperiment],
) -> tuple[FailedExperiment, ...]:
    """
    Sort failures by experiment digest.

    Parameters
    ----------
    failures : Iterable[FailedExperiment]
        The failures to sort.

    Returns
    -------
    tuple[FailedExperiment, ...]
        The sorted failures.

    """
    return tuple(sorted(failures, key=lambda failure: failure.experiment.digest))


@dataclass(frozen=True)
class SearchReport:
    """
    The outcome of a sweep: ranked survivors and recorded failures.

    Parameters
    ----------
    results : tuple[ExperimentResult, ...]
        The ranked survivors.
    failures : tuple[FailedExperiment, ...]
        The recorded failures, sorted by experiment digest.
    evaluated : int, default 0
        The number of distinct experiments evaluated, whether executed or reused from the cache.
    space_size : int | None, default None
        The number of experiments the searched space expands to, or None when undeclared.
    executions : int, default 0
        The number of experiments actually executed, which is the evaluation count a resumed run
        lowers by reusing its cache.
    scheme : ValidationScheme | None, default None
        The validation scheme the search ran under, or None when undeclared.
    seed : int | None, default None
        The seed the search was derived from, or None when it declares none.

    """

    results: tuple[ExperimentResult, ...]
    failures: tuple[FailedExperiment, ...]
    evaluated: int = 0
    space_size: int | None = None
    executions: int = 0
    scheme: ValidationScheme | None = None
    seed: int | None = None

    @property
    def digests(self) -> tuple[str, ...]:
        """
        The canonical digests of the survivors, in ranked order.
        """
        return tuple(result.run.canonical_digest for result in self.results)

    @property
    def evaluated_fraction(self) -> float | None:
        """
        The fraction of the declared space that was evaluated, or None when the size is unknown.
        """
        if self.space_size is None or self.space_size <= 0:
            return None
        return self.evaluated / self.space_size

    def specification_spread(self) -> int | None:
        """
        Return the number of distinct specifications the survivors were found under.

        Two specifications are distinct exactly when their canonical mappings differ, which is the
        same identity their digest names, so a search over three weightings and two windows records
        six. Only the specs the survivors actually recorded count: a trial that recorded none is
        not a specification and does not contribute one.

        Returns
        -------
        int | None
            The number of distinct specifications recorded by the survivors, or None when no
            survivor recorded one. None rather than zero, because zero would assert that every
            trial shared one specification, which is not established by an absent record.

        """
        recorded = {
            result.specification.digest
            for result in self.results
            if result.specification is not None
        }
        return len(recorded) if recorded else None

    def best(self) -> ExperimentResult | None:
        """
        Return the highest-scoring survivor, or None when the sweep produced none.
        """
        return self.results[0] if self.results else None

    def best_feasible(self) -> ExperimentResult | None:
        """
        Return the highest-scoring survivor that satisfies every constraint, or None.
        """
        for result in self.results:
            if result.constraints_satisfied:
                return result
        return None
