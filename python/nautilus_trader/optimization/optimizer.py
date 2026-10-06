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
The optimizer: a parameter space, a search strategy, a runner, and an objective.

The optimizer ties the search and execution split together. It enumerates experiments with a
search strategy, executes them through the runner (sequentially or fanned out by process), and
evaluates the `nautilus_trader.analysis` objective and constraints over each run's metric values.
It composes runs and never alters one: every backtest goes through `BacktestNode`.

A run that fails, and a run whose objective or constraints cannot be evaluated (a missing metric
value, for example), is recorded as a `FailedExperiment` rather than scored as zero, so a sweep
completes and reports its survivors and its failures.
"""

from __future__ import annotations

import math
from dataclasses import dataclass
from dataclasses import field
from dataclasses import replace
from typing import TYPE_CHECKING

from nautilus_trader.optimization.concurrency import ConcurrencyPolicy
from nautilus_trader.optimization.concurrency import execute_experiments
from nautilus_trader.optimization.persistence import Evaluation
from nautilus_trader.optimization.persistence import EvaluationCache
from nautilus_trader.optimization.report import ExperimentResult
from nautilus_trader.optimization.report import SearchReport
from nautilus_trader.optimization.report import rank_results
from nautilus_trader.optimization.report import sort_failures
from nautilus_trader.optimization.runner import FailedExperiment
from nautilus_trader.optimization.search import EvolutionarySearch
from nautilus_trader.optimization.search import GridSearch


if TYPE_CHECKING:
    from collections.abc import Callable
    from collections.abc import Sequence

    from nautilus_trader.analysis import Constraint
    from nautilus_trader.analysis import Objective
    from nautilus_trader.optimization.persistence import ExperimentStore
    from nautilus_trader.optimization.run import ValidationScheme
    from nautilus_trader.optimization.run import WindowBounds
    from nautilus_trader.optimization.runner import BacktestRunner
    from nautilus_trader.optimization.runner import RunOutcome
    from nautilus_trader.optimization.search import SearchStrategy
    from nautilus_trader.optimization.search import TrialSpecification
    from nautilus_trader.optimization.space import Experiment
    from nautilus_trader.optimization.space import ParameterSpace


def _window_within(
    window: tuple[int | None, int | None],
    bounds: WindowBounds,
) -> bool:
    """
    Return whether a declared run window lies inside the given half-open bounds.

    An undeclared bound (`None`) cannot be shown to lie inside any bounds, so it is never within.
    """
    start, end = window
    if start is None or end is None:
        return False
    return bounds[0] <= start and end <= bounds[1]


@dataclass(frozen=True)
class Optimizer:
    """
    Runs a sweep over a parameter space and ranks the survivors.

    Parameters
    ----------
    runner : BacktestRunner
        The runner that executes one experiment.
    objective : Objective
        The objective over the runs' statistics, from `nautilus_trader.analysis`.
    constraints : tuple[Constraint, ...], default ()
        The constraints over the same values, from `nautilus_trader.analysis`.
    concurrency : ConcurrencyPolicy, default ConcurrencyPolicy()
        The memory-driven concurrency limit.
    search : SearchStrategy, default GridSearch()
        The search strategy that enumerates experiments.
    specification : Callable[[Experiment], TrialSpecification] | None, default None
        Reads the specification a trial ran under from its experiment, or None when the caller
        records none. It is read where the result is formed, so the specification travels with the
        trial's result rather than being reconstructed by a report afterwards. A factory that
        cannot read a specification for an experiment raises rather than returning a default.

    """

    runner: BacktestRunner
    objective: Objective
    constraints: tuple[Constraint, ...] = ()
    concurrency: ConcurrencyPolicy = field(default_factory=ConcurrencyPolicy)
    search: SearchStrategy = field(default_factory=GridSearch)
    specification: Callable[[Experiment], TrialSpecification] | None = None

    def required_metrics(self) -> frozenset[str]:
        """
        Return the metric names the objective and constraints reference.

        Returns
        -------
        frozenset[str]

        """
        names = {term.metric for term in self.objective.terms}
        for constraint in self.constraints:
            names.add(constraint.metric)
        return frozenset(names)

    def with_window(self, start: int | None, end: int | None) -> Optimizer:
        """
        Return a copy of this optimizer restricted to the given run window.

        Parameters
        ----------
        start : int | None
            The run window start in Unix nanoseconds.
        end : int | None
            The run window end in Unix nanoseconds.

        Returns
        -------
        Optimizer

        """
        return replace(self, runner=self.runner.windowed(start, end))

    def optimize(
        self,
        space: ParameterSpace,
        *,
        store: ExperimentStore | None = None,
        cache: EvaluationCache | None = None,
        scheme: ValidationScheme | None = None,
    ) -> SearchReport:
        """
        Run the sweep and return the ranked report.

        When a store or a cache is given, the sweep is memoized: an experiment whose parameter
        digest is already recorded is reused rather than executed, so a resumed run skips it and
        its execution count falls. An evolutionary search always needs the record, so it is driven
        through the memo path as well. The report carries the number of distinct evaluations, the
        size of the searched space and the number actually executed.

        Parameters
        ----------
        space : ParameterSpace
            The parameter space to sweep.
        store : ExperimentStore | None, default None
            When given, the report and every evaluation are persisted under the store's directory.
        cache : EvaluationCache | None, default None
            The evaluation memo, or None to build one over the store (or an in-memory one).
        scheme : ValidationScheme | None, default None
            The validation scheme the search ran under, recorded in the report. When given, the
            runner's own window must lie inside one of the scheme's search windows; otherwise the
            sweep is refused rather than scoring a window the scheme held out.

        Returns
        -------
        SearchReport

        """
        if scheme is not None:
            self._enforce_scheme(scheme)
        if cache is not None or store is not None or isinstance(self.search, EvolutionarySearch):
            report = self._optimize_memoized(space, cache=cache, store=store, scheme=scheme)
        else:
            experiments = list(self.search.experiments(space))
            report = replace(
                self.evaluate(experiments),
                space_size=space.size,
                scheme=scheme,
                seed=getattr(self.search, "seed", None),
            )
        if store is not None:
            store.write_report(report)
        return report

    def _enforce_scheme(self, scheme: ValidationScheme) -> None:
        """
        Refuse a run whose window the declared scheme does not score.

        The runner's window is its declared `(start, end)` bounds; a run with undeclared bounds
        cannot be shown to lie inside a search window, so it is refused as well. A run inside a
        held-out window is named as such; any other run outside every search window is refused for
        not being covered. The scheme is read, never repaired.
        """
        window = (self.runner.start, self.runner.end)
        for held_out in scheme.held_out_windows:
            if _window_within(window, held_out):
                raise ValueError(
                    f"run window {window} falls inside the scheme's held-out window {held_out}, "
                    "so it would be scored by the search",
                )
        for search in scheme.search_windows:
            if _window_within(window, search):
                return
        raise ValueError(
            f"run window {window} is not inside any of the scheme's search windows "
            f"{scheme.search_windows}",
        )

    def evaluate(self, experiments: Sequence[Experiment]) -> SearchReport:
        """
        Execute and score the given experiments.

        Constraints are evaluated before the objective, so an infeasible experiment whose objective
        cannot be evaluated is recorded as infeasible rather than as a failure, and no candidate is
        recorded as a low score merely for violating a constraint.

        Parameters
        ----------
        experiments : Sequence[Experiment]
            The experiments to execute and score, in sweep order.

        Returns
        -------
        SearchReport

        """
        metrics = self.required_metrics()
        outcomes = execute_experiments(self.runner, experiments, metrics, self.concurrency)
        evaluations = [self._evaluate_outcome(outcome) for outcome in outcomes]

        return self._report_from_evaluations(
            evaluations,
            space_size=None,
            executions=len(outcomes),
            scheme=None,
            seed=None,
        )

    def _optimize_memoized(
        self,
        space: ParameterSpace,
        *,
        cache: EvaluationCache | None = None,
        store: ExperimentStore | None = None,
        scheme: ValidationScheme | None = None,
    ) -> SearchReport:
        """
        Stream the search, reuse cached evaluations, and execute only the unseen experiments.
        """
        metrics = self.required_metrics()
        memo = cache if cache is not None else EvaluationCache(store)
        evaluations: list[Evaluation] = []
        executions = 0

        for experiment in self.search.experiments(space):
            evaluation = memo.get(experiment.digest)
            if evaluation is None:
                outcome = self.runner.run(experiment, metrics)
                executions += 1
                evaluation = self._evaluate_outcome(outcome)
                memo.put(evaluation)
            evaluations.append(evaluation)

        return self._report_from_evaluations(
            evaluations,
            space_size=space.size,
            executions=executions,
            scheme=scheme,
            seed=getattr(self.search, "seed", None),
        )

    def _evaluate_outcome(self, outcome: RunOutcome) -> Evaluation:
        """
        Evaluate one run, checking the constraints before the objective.
        """
        if isinstance(outcome, FailedExperiment):
            return Evaluation(outcome, math.nan, feasible=False)

        values = outcome.metric_values
        try:
            feasible = all(constraint.is_satisfied(values) for constraint in self.constraints)
        except ValueError as exc:
            return Evaluation(
                FailedExperiment(outcome.experiment, type(exc).__name__, str(exc)),
                math.nan,
                feasible=False,
            )

        try:
            score = self.objective.evaluate(values)
        except ValueError as exc:
            if feasible:
                return Evaluation(
                    FailedExperiment(outcome.experiment, type(exc).__name__, str(exc)),
                    math.nan,
                    feasible=False,
                )
            return Evaluation(outcome, math.nan, feasible=False)

        return Evaluation(outcome, score, feasible)

    def _report_from_evaluations(
        self,
        evaluations: Sequence[Evaluation],
        *,
        space_size: int | None = None,
        executions: int = 0,
        scheme: ValidationScheme | None = None,
        seed: int | None = None,
    ) -> SearchReport:
        """
        Rank the memorized evaluations into a search report.

        Each survivor's specification is read from its experiment here, where the trial's result is
        formed, and carried on the result, so the report never reconstructs it from the space.
        """
        specification_of = self.specification
        results: list[ExperimentResult] = []
        failures: list[FailedExperiment] = []
        for evaluation in evaluations:
            outcome = evaluation.outcome
            if isinstance(outcome, FailedExperiment):
                failures.append(outcome)
            else:
                specification = (
                    None if specification_of is None else specification_of(outcome.experiment)
                )
                results.append(
                    ExperimentResult(
                        outcome,
                        evaluation.score,
                        evaluation.feasible,
                        specification,
                    ),
                )

        return SearchReport(
            rank_results(results),
            sort_failures(failures),
            evaluated=len(evaluations),
            space_size=space_size,
            executions=executions,
            scheme=scheme,
            seed=seed,
        )
