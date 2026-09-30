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
The runner that executes exactly one experiment through `BacktestNode`.

Execution is the only way this package touches a backtest: the runner builds one
`BacktestRunConfig` whose strategy config carries the experiment's parameter set, runs it through
`BacktestNode`, and reads back the canonical result and the run's statistics. It never constructs
an engine, never owns the data or venue configuration (the caller supplies those through a factory
that returns them per window), and never reaches into a run to alter it.

The run config ID is the experiment digest, so the same experiment always produces the same
canonical result digest. A run that fails is captured as a `FailedExperiment` with its typed error
rather than aborting the sweep.
"""

from __future__ import annotations

from dataclasses import dataclass
from dataclasses import replace
from typing import TYPE_CHECKING

from nautilus_trader.backtest import BacktestNode
from nautilus_trader.backtest import BacktestRunConfig
from nautilus_trader.optimization.metrics import statistic_values
from nautilus_trader.trading import ImportableStrategyConfig


if TYPE_CHECKING:
    from collections.abc import Callable
    from collections.abc import Iterable
    from collections.abc import Mapping
    from collections.abc import Sequence

    from nautilus_trader.backtest import BacktestDataConfig
    from nautilus_trader.backtest import BacktestEngineConfig
    from nautilus_trader.backtest import BacktestVenueConfig
    from nautilus_trader.optimization.space import Experiment


type RunConfigParts = tuple[
    Sequence[BacktestVenueConfig],
    Sequence[BacktestDataConfig],
    BacktestEngineConfig,
]


@dataclass(frozen=True)
class CanonicalRun:
    """
    A completed run: its parameter set, canonical digest, metrics, and canonical document.

    Parameters
    ----------
    experiment : Experiment
        The parameter set the run executed.
    canonical_digest : str
        The canonical backtest result digest, the comparability key.
    metric_values : Mapping[str, float]
        The metric values of the run, keyed by statistic name.
    canonical_result : bytes
        The canonical backtest result document.

    """

    experiment: Experiment
    canonical_digest: str
    metric_values: Mapping[str, float]
    canonical_result: bytes


@dataclass(frozen=True)
class FailedExperiment:
    """
    An experiment that could not be executed or evaluated.

    Parameters
    ----------
    experiment : Experiment
        The parameter set that failed.
    error_type : str
        The exception type name.
    error_message : str
        The exception message.

    """

    experiment: Experiment
    error_type: str
    error_message: str


type RunOutcome = CanonicalRun | FailedExperiment


@dataclass(frozen=True)
class BacktestRunner:
    """
    Executes one experiment through `BacktestNode`.

    The caller supplies a `config_factory` that returns the venue configurations, data
    configurations, and engine configuration for a run window. The runner composes those into a
    `BacktestRunConfig` with a deterministic ID and adds the strategy from the experiment's
    parameter set, so the caller keeps ownership of the data and venue setup and the runner adds
    only the run identity and the strategy.

    Parameters
    ----------
    config_factory : Callable[[int | None, int | None], RunConfigParts]
        Returns the venue configs, data configs, and engine config for the given window bounds.
        A module-level function is required so the runner can be sent to a worker process.
    strategy_path : str
        The importable path to the strategy class, for `ImportableStrategyConfig`.
    config_path : str
        The importable path to the strategy config class, for `ImportableStrategyConfig`.
    start : int | None, default None
        The run window start in Unix nanoseconds, or None for the data's own start.
    end : int | None, default None
        The run window end in Unix nanoseconds, or None for the data's own end.

    """

    config_factory: Callable[[int | None, int | None], RunConfigParts]
    strategy_path: str
    config_path: str
    start: int | None = None
    end: int | None = None

    def windowed(self, start: int | None, end: int | None) -> BacktestRunner:
        """
        Return a copy of this runner restricted to the given window.

        Parameters
        ----------
        start : int | None
            The run window start in Unix nanoseconds.
        end : int | None
            The run window end in Unix nanoseconds.

        Returns
        -------
        BacktestRunner

        """
        return replace(self, start=start, end=end)

    def run(self, experiment: Experiment, metrics: Iterable[str] | None = None) -> RunOutcome:
        """
        Execute exactly one experiment and return its canonical run or its typed failure.

        Parameters
        ----------
        experiment : Experiment
            The parameter set to run.
        metrics : Iterable[str] | None, default None
            The metric names to compute from the run's statistics.

        Returns
        -------
        RunOutcome
            A `CanonicalRun` on success, or a `FailedExperiment` carrying the typed error.

        """
        try:
            outcome = self._execute(experiment, metrics)
        except Exception as exc:  # noqa: BLE001 (any run failure is recorded, never aborts)
            return FailedExperiment(experiment, type(exc).__name__, str(exc))
        return outcome

    def _execute(self, experiment: Experiment, metrics: Iterable[str] | None) -> CanonicalRun:
        """
        Build, run, and read back one experiment, releasing the node before returning.
        """
        venues, data, engine = self.config_factory(self.start, self.end)
        config = BacktestRunConfig(
            venues=list(venues),
            data=list(data),
            engine=engine,
            id=experiment.digest,
            raise_exception=True,
            dispose_on_completion=False,
            start=self.start,
            end=self.end,
        )
        node = BacktestNode([config])
        try:
            node.build()
            node.add_strategy_from_config(
                config.id,
                ImportableStrategyConfig(
                    strategy_path=self.strategy_path,
                    config_path=self.config_path,
                    config=dict(experiment.parameters),
                ),
            )
            results = node.run()
            canonical = node.get_engine_canonical_result(config.id)
            digest = canonical.digest()
            document = canonical.to_bytes()
            result = results[0] if results else None
            if result is None:
                raise RuntimeError(f"Backtest produced no result for run config '{config.id}'")
            values = statistic_values(result, metrics)
            return CanonicalRun(experiment, digest, values, document)
        finally:
            node.dispose()
