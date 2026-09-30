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
The methodology stages: train, optimize, validate, out-of-sample, and walk-forward.

These are distinct stages, not one loop over a grid. Each stage owns a different decision:

- Train selects one experiment from a search on the training window: the model.
- Optimize produces the ranked report of a search without forcing a selection.
- Validate evaluates one experiment and reports whether the constraints hold.
- Out-of-sample evaluates the selected model on a window it was not selected on.
- Walk-forward splits a period by time into an in-sample and an out-of-sample segment, searches on
  the former, and evaluates the selected model on the latter, for every window.

The search itself is the shared primitive; the stages differ in what they do with its outcome. No
stage alters a run: each composes runs through the runner.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING

from nautilus_trader.optimization.report import ExperimentResult
from nautilus_trader.optimization.report import SearchReport


if TYPE_CHECKING:
    from collections.abc import Mapping
    from collections.abc import Sequence

    from nautilus_trader.optimization.optimizer import Optimizer
    from nautilus_trader.optimization.runner import FailedExperiment
    from nautilus_trader.optimization.space import Experiment
    from nautilus_trader.optimization.space import ParameterSpace


@dataclass(frozen=True)
class WalkForwardWindow:
    """
    One walk-forward window: an in-sample segment followed by an out-of-sample segment.

    Parameters
    ----------
    in_sample_start : int
        The in-sample start in Unix nanoseconds.
    in_sample_end : int
        The in-sample end in Unix nanoseconds.
    out_of_sample_start : int
        The out-of-sample start in Unix nanoseconds.
    out_of_sample_end : int
        The out-of-sample end in Unix nanoseconds.

    """

    in_sample_start: int
    in_sample_end: int
    out_of_sample_start: int
    out_of_sample_end: int


def walk_forward_windows(
    start: int,
    end: int,
    *,
    in_sample: int,
    out_of_sample: int,
) -> tuple[WalkForwardWindow, ...]:
    """
    Split a period into consecutive in-sample/out-of-sample windows.

    The period is cut into as many windows as fit, each an in-sample segment of `in_sample`
    nanoseconds immediately followed by an out-of-sample segment of `out_of_sample` nanoseconds.
    The split is deterministic and the out-of-sample segment never outlives the period.

    Parameters
    ----------
    start : int
        The period start in Unix nanoseconds.
    end : int
        The period end in Unix nanoseconds.
    in_sample : int
        The in-sample length in nanoseconds. Must be positive.
    out_of_sample : int
        The out-of-sample length in nanoseconds. Must be positive.

    Returns
    -------
    tuple[WalkForwardWindow, ...]

    """
    if in_sample <= 0 or out_of_sample <= 0:
        raise ValueError("in_sample and out_of_sample must be positive")
    if end <= start:
        return ()

    step = in_sample + out_of_sample
    count = (end - start) // step
    windows = []
    cursor = start
    for _ in range(count):
        in_sample_end = cursor + in_sample
        windows.append(
            WalkForwardWindow(
                cursor,
                in_sample_end,
                in_sample_end,
                in_sample_end + out_of_sample,
            ),
        )
        cursor += step
    return tuple(windows)


@dataclass(frozen=True)
class TrainStage:
    """
    Trains a model by selecting the best experiment from a search on the training window.

    Parameters
    ----------
    optimizer : Optimizer
        An optimizer whose runner is restricted to the training window.

    """

    optimizer: Optimizer

    def run(self, space: ParameterSpace) -> ExperimentResult | None:
        """
        Search the space and return the selected model, or None when nothing survives.

        Parameters
        ----------
        space : ParameterSpace
            The parameter space to train on.

        Returns
        -------
        ExperimentResult | None

        """
        return self.optimizer.optimize(space).best()


@dataclass(frozen=True)
class OptimizeStage:
    """
    Optimizes by producing the ranked report of a search, without forcing a selection.

    Parameters
    ----------
    optimizer : Optimizer
        The optimizer to run the search with.

    """

    optimizer: Optimizer

    def run(self, space: ParameterSpace) -> SearchReport:
        """
        Search the space and return the ranked report.

        Parameters
        ----------
        space : ParameterSpace
            The parameter space to optimize over.

        Returns
        -------
        SearchReport

        """
        return self.optimizer.optimize(space)


@dataclass(frozen=True)
class ValidationResult:
    """
    The outcome of validating one experiment.

    Parameters
    ----------
    experiment : Experiment
        The validated experiment.
    metric_values : Mapping[str, float]
        The metric values the objective and constraints were evaluated over.
    score : float
        The objective score.
    constraints_satisfied : bool
        Whether every constraint holds.
    violations : tuple[str, ...]
        The metric names whose constraints do not hold.

    """

    experiment: Experiment
    metric_values: Mapping[str, float]
    score: float
    constraints_satisfied: bool
    violations: tuple[str, ...]


@dataclass(frozen=True)
class ValidateStage:
    """
    Validates one experiment against the objective and constraints.

    Parameters
    ----------
    optimizer : Optimizer
        The optimizer whose objective and constraints define validation.

    """

    optimizer: Optimizer

    def run(self, experiment: Experiment) -> ValidationResult | FailedExperiment:
        """
        Evaluate the experiment and report its constraint outcome.

        Parameters
        ----------
        experiment : Experiment
            The experiment to validate.

        Returns
        -------
        ValidationResult | FailedExperiment

        """
        report = self.optimizer.evaluate((experiment,))
        if not report.results:
            return report.failures[0]
        result = report.results[0]
        violations = tuple(
            constraint.metric
            for constraint in self.optimizer.constraints
            if not constraint.is_satisfied(result.run.metric_values)
        )
        return ValidationResult(
            result.experiment,
            result.run.metric_values,
            result.score,
            result.constraints_satisfied,
            violations,
        )


@dataclass(frozen=True)
class OutOfSampleStage:
    """
    Evaluates the selected model on a window it was not selected on.

    Parameters
    ----------
    optimizer : Optimizer
        An optimizer whose runner is restricted to the out-of-sample window.

    """

    optimizer: Optimizer

    def run(self, experiment: Experiment) -> ExperimentResult | FailedExperiment:
        """
        Evaluate the experiment on the out-of-sample window.

        Parameters
        ----------
        experiment : Experiment
            The selected model to evaluate.

        Returns
        -------
        ExperimentResult | FailedExperiment

        """
        report = self.optimizer.evaluate((experiment,))
        if report.results:
            return report.results[0]
        return report.failures[0]


@dataclass(frozen=True)
class WalkForwardResult:
    """
    The outcome of one walk-forward window.

    Parameters
    ----------
    window : WalkForwardWindow
        The window.
    training : SearchReport | None
        The in-sample search report, or None when no experiment was run.
    selected : Experiment | None
        The model selected on the in-sample segment, or None.
    out_of_sample : ExperimentResult | FailedExperiment | None
        The out-of-sample evaluation of the selected model, or None.

    """

    window: WalkForwardWindow
    training: SearchReport | None
    selected: Experiment | None
    out_of_sample: ExperimentResult | FailedExperiment | None


@dataclass(frozen=True)
class WalkForwardReport:
    """
    The outcome of a walk-forward run.

    Parameters
    ----------
    windows : tuple[WalkForwardResult, ...]
        The per-window outcomes, in window order.

    """

    windows: tuple[WalkForwardResult, ...]

    def out_of_sample_results(self) -> tuple[ExperimentResult, ...]:
        """
        Return the out-of-sample results that completed, in window order.

        Returns
        -------
        tuple[ExperimentResult, ...]

        """
        return tuple(
            window.out_of_sample
            for window in self.windows
            if isinstance(window.out_of_sample, ExperimentResult)
        )


@dataclass(frozen=True)
class WalkForwardStage:
    """
    Runs a walk-forward experiment over a sequence of windows.

    Each window searches on its in-sample segment and evaluates the selected model on its
    out-of-sample segment. The stages are composed from separate in-sample and out-of-sample
    optimizers; this stage re-windows them per window.

    Parameters
    ----------
    in_sample : Optimizer
        The optimizer used for the in-sample search.
    out_of_sample : Optimizer
        The optimizer used for the out-of-sample evaluation.

    """

    in_sample: Optimizer
    out_of_sample: Optimizer

    def run(
        self,
        space: ParameterSpace,
        windows: Sequence[WalkForwardWindow],
    ) -> WalkForwardReport:
        """
        Run the walk-forward experiment.

        Parameters
        ----------
        space : ParameterSpace
            The parameter space to search on each in-sample segment.
        windows : Sequence[WalkForwardWindow]
            The windows to run, in order.

        Returns
        -------
        WalkForwardReport

        """
        results = []
        for window in windows:
            training = self.in_sample.with_window(
                window.in_sample_start,
                window.in_sample_end,
            ).optimize(space)
            selected = training.best()
            experiment = selected.experiment if selected is not None else None
            out_of_sample = self._evaluate_out_of_sample(window, experiment)
            results.append(WalkForwardResult(window, training, experiment, out_of_sample))
        return WalkForwardReport(tuple(results))

    def _evaluate_out_of_sample(
        self,
        window: WalkForwardWindow,
        experiment: Experiment | None,
    ) -> ExperimentResult | FailedExperiment | None:
        """
        Evaluate the selected model on the window's out-of-sample segment, when one was selected.
        """
        if experiment is None:
            return None
        optimizer = self.out_of_sample.with_window(
            window.out_of_sample_start,
            window.out_of_sample_end,
        )
        report = optimizer.evaluate((experiment,))
        if report.results:
            return report.results[0]
        return report.failures[0]
