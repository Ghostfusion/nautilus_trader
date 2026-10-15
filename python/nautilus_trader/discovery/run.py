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
One experiment is one backtest, and one backtest produces one evidence package.

The runner does not add an execution path. It builds the same `BacktestNode` the optimization
subsystem builds, injects the interpreter as a live object rather than resolving an import path,
runs it once, and keeps the engine's own canonical result beside the identities of the artifact that
ran. The canonical bytes are the engine's, so a later recomputation compares an artifact rather than
a claim.

An experiment's identity is the compiled plan's behavioural identity, flattened: what ran is the
specification together with the policy, the resolved feature definitions, the declared execution
assumptions and the engine version. Two candidates that differ only in their specification identity
are therefore two experiments, and a candidate whose feature definition changed is a different
experiment even though its specification digest did not move.

A run that fails raises rather than returning a failure, because a failure is an outcome the ledger
records and the protocol that records it, together with the reconciliation and idempotency rules
around it, belongs to the governance milestone.
"""

from __future__ import annotations

from dataclasses import dataclass
from typing import TYPE_CHECKING

from nautilus_trader.backtest import BacktestNode
from nautilus_trader.backtest import BacktestRunConfig
from nautilus_trader.discovery.ledger import EvidencePackage
from nautilus_trader.discovery.ledger import RunIdentity
from nautilus_trader.discovery.schema import canonical_mapping
from nautilus_trader.optimization.metrics import statistic_values
from nautilus_trader.optimization.runner import CanonicalRun
from nautilus_trader.optimization.space import Experiment


if TYPE_CHECKING:
    from collections.abc import Sequence

    from nautilus_trader.backtest import BacktestDataConfig
    from nautilus_trader.backtest import BacktestEngineConfig
    from nautilus_trader.backtest import BacktestVenueConfig
    from nautilus_trader.discovery.compile import CompiledPlan
    from nautilus_trader.trading import Strategy


@dataclass(frozen=True)
class RunRequest:
    """
    Everything one experiment needs, so the runner invents nothing.

    Parameters
    ----------
    plan : CompiledPlan
        The compiled plan the experiment's identity is taken from.
    strategy : Strategy
        The interpreter, configured from the plan and injected as a live object.
    venues : Sequence[BacktestVenueConfig]
        The venue configurations the run trades through.
    data : Sequence[BacktestDataConfig]
        The data configurations the run replays.
    engine : BacktestEngineConfig
        The engine configuration.
    metrics : Sequence[str]
        The metric identifiers the run's values are computed for.
    seed : int
        The seed the run declares.
    snapshot : str
        The description of the data snapshot the run is bound to.
    run_id : str
        The run configuration's identifier, which defaults to the experiment digest.

    """

    plan: CompiledPlan
    strategy: Strategy
    venues: Sequence[BacktestVenueConfig]
    data: Sequence[BacktestDataConfig]
    engine: BacktestEngineConfig
    metrics: Sequence[str]
    seed: int
    snapshot: str
    run_id: str = ""


@dataclass(frozen=True)
class ExperimentOutcome:
    """
    What one experiment produced: the artifact, the engine's record of it, and its evidence.
    """

    experiment: Experiment
    run: CanonicalRun
    package: EvidencePackage


def experiment_for(plan: CompiledPlan) -> Experiment:
    """
    Return the experiment identity of a compiled plan.

    The parameters are the plan's identities rather than a strategy's configuration, because the
    artifact that runs is the specification and everything its behaviour depends on.

    Parameters
    ----------
    plan : CompiledPlan
        The compiled plan.

    Returns
    -------
    Experiment

    """
    return Experiment(
        parameters=canonical_mapping(
            {
                "schema": plan.specification.version,
                "specification": plan.specification_digest,
                "behavioural": plan.behavioural_digest,
                "policy": plan.policy_digest,
                "registry": plan.registry_digest,
                "assumptions": plan.assumptions_digest,
                "engine": plan.engine_version,
            },
        ),
    )


def run_experiment(request: RunRequest) -> ExperimentOutcome:
    """
    Run one experiment and return its evidence package.

    Parameters
    ----------
    request : RunRequest
        The experiment to run.

    Returns
    -------
    ExperimentOutcome

    """
    experiment = experiment_for(request.plan)
    config = BacktestRunConfig(
        venues=list(request.venues),
        data=list(request.data),
        engine=request.engine,
        id=request.run_id or experiment.digest,
        raise_exception=True,
        dispose_on_completion=False,
    )
    node = BacktestNode([config])

    try:
        node.build()
        node.add_strategy(config.id, request.strategy)
        results = node.run()
        canonical = node.get_engine_canonical_result(config.id)
    finally:
        node.dispose()

    values = statistic_values(results[0], list(request.metrics))
    run = CanonicalRun(
        experiment=experiment,
        canonical_digest=canonical.digest(),
        metric_values=values,
        canonical_result=canonical.to_bytes(),
    )
    package = EvidencePackage.from_run(
        run,
        RunIdentity.of(request.plan, snapshot=request.snapshot, seed=request.seed),
    )

    return ExperimentOutcome(experiment=experiment, run=run, package=package)
