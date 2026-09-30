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

from dataclasses import dataclass
from dataclasses import field
from dataclasses import replace
from typing import TYPE_CHECKING

from nautilus_trader.optimization.concurrency import ConcurrencyPolicy
from nautilus_trader.optimization.concurrency import execute_experiments
from nautilus_trader.optimization.report import ExperimentResult
from nautilus_trader.optimization.report import SearchReport
from nautilus_trader.optimization.report import rank_results
from nautilus_trader.optimization.report import sort_failures
from nautilus_trader.optimization.runner import FailedExperiment
from nautilus_trader.optimization.search import GridSearch


if TYPE_CHECKING:
    from collections.abc import Sequence

    from nautilus_trader.analysis import Constraint
    from nautilus_trader.analysis import Objective
    from nautilus_trader.optimization.persistence import ExperimentStore
    from nautilus_trader.optimization.runner import BacktestRunner
    from nautilus_trader.optimization.search import SearchStrategy
    from nautilus_trader.optimization.space import Experiment
    from nautilus_trader.optimization.space import ParameterSpace


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

    """

    runner: BacktestRunner
    objective: Objective
    constraints: tuple[Constraint, ...] = ()
    concurrency: ConcurrencyPolicy = field(default_factory=ConcurrencyPolicy)
    search: SearchStrategy = field(default_factory=GridSearch)

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
        self, space: ParameterSpace, *, store: ExperimentStore | None = None
    ) -> SearchReport:
        """
        Run the sweep and return the ranked report.

        Parameters
        ----------
        space : ParameterSpace
            The parameter space to sweep.
        store : ExperimentStore | None, default None
            When given, the report is persisted under the store's directory.

        Returns
        -------
        SearchReport

        """
        experiments = list(self.search.experiments(space))
        report = self.evaluate(experiments)
        if store is not None:
            store.write_report(report)
        return report

    def evaluate(self, experiments: Sequence[Experiment]) -> SearchReport:
        """
        Execute and score the given experiments.

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

        results: list[ExperimentResult] = []
        failures: list[FailedExperiment] = []
        for outcome in outcomes:
            if isinstance(outcome, FailedExperiment):
                failures.append(outcome)
                continue
            try:
                score = self.objective.evaluate(outcome.metric_values)
                satisfied = all(
                    constraint.is_satisfied(outcome.metric_values)
                    for constraint in self.constraints
                )
            except ValueError as exc:
                failures.append(
                    FailedExperiment(outcome.experiment, type(exc).__name__, str(exc)),
                )
                continue
            results.append(ExperimentResult(outcome, score, satisfied))

        return SearchReport(rank_results(results), sort_failures(failures))
