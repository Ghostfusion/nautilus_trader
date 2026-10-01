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
Integration tests for the Python optimization orchestration.

A temporary Parquet catalog holds one instrument and a slice of minute bars. Every sweep runs the
same EMA cross strategy through `BacktestNode` with the experiment's parameter set. The tests pin
the best result against a hand-run `BacktestNode` invocation, exercise the objective over Sharpe
ratio and maximum drawdown, confirm a failing experiment does not abort a sweep, and prove that
process fan-out and sequential execution agree on the best result and the digests.
"""

from __future__ import annotations

import dataclasses
from decimal import Decimal
from functools import partial
from pathlib import Path

import pytest

from nautilus_trader.analysis import Constraint
from nautilus_trader.analysis import ConstraintComparison
from nautilus_trader.analysis import Objective
from nautilus_trader.analysis import ObjectiveDirection
from nautilus_trader.analysis import ObjectiveTerm
from nautilus_trader.backtest import BacktestDataConfig
from nautilus_trader.backtest import BacktestEngineConfig
from nautilus_trader.backtest import BacktestNode
from nautilus_trader.backtest import BacktestRunConfig
from nautilus_trader.backtest import BacktestVenueConfig
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.model import BarType
from nautilus_trader.model import InstrumentId
from nautilus_trader.model import NautilusDataType
from nautilus_trader.optimization import BacktestRunner
from nautilus_trader.optimization import BarAmbiguityPolicy
from nautilus_trader.optimization import ComputationIdentity
from nautilus_trader.optimization import ConcurrencyPolicy
from nautilus_trader.optimization import DatasetIdentity
from nautilus_trader.optimization import Experiment
from nautilus_trader.optimization import ExperimentResult
from nautilus_trader.optimization import ExperimentStore
from nautilus_trader.optimization import Optimizer
from nautilus_trader.optimization import OptimizeStage
from nautilus_trader.optimization import Parameter
from nautilus_trader.optimization import ParameterSpace
from nautilus_trader.optimization import ResearchResult
from nautilus_trader.optimization import SelectionRule
from nautilus_trader.optimization import StudyIdentity
from nautilus_trader.optimization import TrainStage
from nautilus_trader.optimization import UniverseIdentity
from nautilus_trader.optimization import ValidateStage
from nautilus_trader.optimization import ValidationResult
from nautilus_trader.optimization import WalkForwardStage
from nautilus_trader.optimization import WalkForwardWindow
from nautilus_trader.optimization import objective_definition_from_terms
from nautilus_trader.optimization import statistic_values
from nautilus_trader.optimization import trial_identity
from nautilus_trader.optimization import walk_forward_windows
from nautilus_trader.optimization.metrics import bridged_values
from nautilus_trader.persistence import ParquetDataCatalog
from nautilus_trader.trading import ImportableStrategyConfig
from tests.providers import TestDataProvider
from tests.providers import TestInstrumentProvider


INSTRUMENT = TestInstrumentProvider.btcusdt_binance()
INSTRUMENT_ID = INSTRUMENT.id
BAR_TYPE = "BTCUSDT.BINANCE-1-MINUTE-LAST-EXTERNAL"
CSV_NAME = "btc-perp-20211231-20220201_1m.csv"
MAX_ROWS = 120
TRADE_SIZE = "0.010000"
STRATEGY = "strategies.ema_cross:EMACross"
STRATEGY_CONFIG = "strategies.ema_cross:EMACrossConfig"
SHARPE = "Sharpe Ratio (252 days)"
DRAWDOWN = "Max Drawdown"


def _config_parts(start, end, *, catalog_path, instrument_id, bar_type):
    """
    Build the venue, data, and engine configurations for a run window.
    """
    venue = BacktestVenueConfig(
        name="BINANCE",
        oms_type="NETTING",
        account_type="CASH",
        starting_balances=["10 BTC", "10_000_000 USDT"],
        book_type="L1_MBP",
        fee_model=MakerTakerFeeModel(maker_rate=Decimal("0.001"), taker_rate=Decimal("0.001")),
    )
    data = BacktestDataConfig(
        data_type=NautilusDataType.Bar,
        catalog_path=catalog_path,
        instrument_id=InstrumentId.from_str(instrument_id),
        bar_types=[bar_type],
        start_time=start,
        end_time=end,
    )
    engine = BacktestEngineConfig(bypass_logging=True, run_analysis=False)
    return [venue], [data], engine


def _build_catalog(directory: Path) -> Path:
    """
    Write one instrument and a slice of minute bars into a fresh Parquet catalog.
    """
    directory.mkdir(parents=True, exist_ok=True)
    catalog = ParquetDataCatalog(str(directory))
    catalog.write_instruments([INSTRUMENT])
    catalog.write_bars(_bars(MAX_ROWS))
    return directory


def _bars(max_rows: int):
    return TestDataProvider.bars_from_binance_csv(
        INSTRUMENT,
        bar_type=BarType.from_str(BAR_TYPE),
        csv_name=CSV_NAME,
        max_rows=max_rows,
    )


def _runner(catalog_path: Path) -> BacktestRunner:
    """
    Build the runner over the given catalog.
    """
    factory = partial(
        _config_parts,
        catalog_path=str(catalog_path),
        instrument_id=str(INSTRUMENT_ID),
        bar_type=BAR_TYPE,
    )
    return BacktestRunner(
        config_factory=factory, strategy_path=STRATEGY, config_path=STRATEGY_CONFIG
    )


def _space() -> ParameterSpace:
    """
    Build the small grid: two fast periods by two slow periods.
    """
    return ParameterSpace(
        base={
            "instrument_id": str(INSTRUMENT_ID),
            "bar_type": BAR_TYPE,
            "trade_size": TRADE_SIZE,
        },
        parameters=(
            Parameter("fast_ema_period", (5, 10)),
            Parameter("slow_ema_period", (20, 30)),
        ),
    )


def _objective() -> Objective:
    """
    Build the objective over Sharpe ratio and maximum drawdown.
    """
    return Objective(
        [
            ObjectiveTerm(SHARPE, 1.0, ObjectiveDirection.MAXIMIZE),
            ObjectiveTerm(DRAWDOWN, 1.0, ObjectiveDirection.MAXIMIZE),
        ],
    )


def _hand_run(catalog_path: Path, parameters, metrics):
    """
    Run one experiment by hand through `BacktestNode` and return its digest, values, score, run.

    This is the manual path the optimizer must reproduce: it builds the run config directly, adds
    the strategy from the parameter set, runs the node, and reads the canonical digest and the
    statistics back. The run config ID is the experiment digest, matching the optimizer.
    """
    venues, data, engine = _config_parts(
        None,
        None,
        catalog_path=str(catalog_path),
        instrument_id=str(INSTRUMENT_ID),
        bar_type=BAR_TYPE,
    )
    experiment = Experiment(parameters)
    config = BacktestRunConfig(
        venues=list(venues),
        data=list(data),
        engine=engine,
        id=experiment.digest,
        raise_exception=True,
        dispose_on_completion=False,
    )
    node = BacktestNode([config])
    node.build()
    node.add_strategy_from_config(
        config.id,
        ImportableStrategyConfig(
            strategy_path=STRATEGY,
            config_path=STRATEGY_CONFIG,
            config=dict(parameters),
        ),
    )
    try:
        results = node.run()
        canonical = node.get_engine_canonical_result(config.id)
        values = statistic_values(results[0], metrics)
        score = _objective().evaluate(values)
    finally:
        node.dispose()
    return canonical.digest(), values, score, results[0]


def test_grid_matches_hand_run(tmp_path: Path) -> None:
    """
    Test the optimizer's best result matches running that grid point by hand.
    """
    catalog_path = _build_catalog(tmp_path / "catalog")
    optimizer = Optimizer(
        runner=_runner(catalog_path),
        objective=_objective(),
        concurrency=ConcurrencyPolicy.sequential(),
    )
    report = optimizer.optimize(_space())
    best = report.best()
    assert best is not None
    assert len(report.results) == 4
    assert report.failures == ()

    hand_digest, hand_values, hand_score, _ = _hand_run(
        catalog_path,
        dict(best.experiment.parameters),
        optimizer.required_metrics(),
    )

    print(f"optimizer_best_params={dict(best.experiment.parameters)}")  # noqa: T201
    print(f"optimizer_best_score={best.score!r}")  # noqa: T201
    print(f"optimizer_best_digest={best.digest}")  # noqa: T201
    print(f"hand_score={hand_score!r}")  # noqa: T201
    print(f"hand_digest={hand_digest}")  # noqa: T201
    print(f"hand_values={hand_values}")  # noqa: T201

    assert best.experiment.parameters["fast_ema_period"] == 10
    assert best.experiment.parameters["slow_ema_period"] == 30
    assert best.score == hand_score
    assert best.digest == hand_digest
    assert hand_digest == (
        "blake3:199f2eb6524925b88ebd6c5cfd0fdc599ec9dc3da3b736ac104178e1592f961f"
    )
    assert hand_score == -27.269647329463734


def test_bridge_reproduces_engine_statistic(tmp_path: Path) -> None:
    """
    Test the returns bridge reproduces the engine's own statistic value exactly.

    The run reports Sharpe ratio itself, and the bridge recomputes it from the run's returns
    series with the native statistic. The two values are the same float, which is the evidence
    that the bridge reads the project's own implementation rather than a second one.
    """
    catalog_path = _build_catalog(tmp_path / "catalog")
    _, _, _, result = _hand_run(
        catalog_path,
        {
            "instrument_id": str(INSTRUMENT_ID),
            "bar_type": BAR_TYPE,
            "trade_size": TRADE_SIZE,
            "fast_ema_period": 10,
            "slow_ema_period": 30,
        },
        frozenset({SHARPE, DRAWDOWN}),
    )

    bridged = bridged_values(result.returns_series, frozenset({SHARPE}))
    engine_value = result.stats_returns[SHARPE]

    print(f"bridged_sharpe={bridged[SHARPE]!r}")  # noqa: T201
    print(f"engine_sharpe={engine_value!r}")  # noqa: T201
    assert bridged[SHARPE] == engine_value


def test_objective_over_sharpe_and_drawdown(tmp_path: Path) -> None:
    """
    Test the objective combines the Sharpe ratio and maximum drawdown as specified.
    """
    catalog_path = _build_catalog(tmp_path / "catalog")
    objective = _objective()
    optimizer = Optimizer(
        runner=_runner(catalog_path),
        objective=objective,
        constraints=(Constraint(DRAWDOWN, ConstraintComparison.AT_LEAST, -0.013),),
        concurrency=ConcurrencyPolicy.sequential(),
    )
    report = optimizer.optimize(_space())
    assert len(report.results) == 4

    for result in report.results:
        values = result.run.metric_values
        sharpe = values[SHARPE]
        drawdown = values[DRAWDOWN]
        print(  # noqa: T201
            f"fast={result.experiment.parameters['fast_ema_period']} "
            f"slow={result.experiment.parameters['slow_ema_period']} "
            f"sharpe={sharpe!r} drawdown={drawdown!r} score={result.score!r} "
            f"feasible={result.constraints_satisfied}",
        )
        assert drawdown < 0.0
        assert result.constraints_satisfied == (drawdown >= -0.013)
        assert result.score == pytest.approx(sharpe + drawdown, rel=0.0, abs=1e-12)

    best = report.best()
    assert best is not None
    assert best.run.metric_values[SHARPE] == -27.2553412003192
    assert best.run.metric_values[DRAWDOWN] == -0.014306129144533997
    assert best.score == -27.269647329463734
    assert not best.constraints_satisfied
    assert sum(result.constraints_satisfied for result in report.results) == 2
    assert report.best_feasible() is not None
    assert report.best_feasible().experiment.parameters["fast_ema_period"] == 5


def test_failing_run_does_not_abort_sweep(tmp_path: Path) -> None:
    """
    Test a failing experiment is recorded and the sweep still reports its survivors.
    """
    catalog_path = _build_catalog(tmp_path / "catalog")
    optimizer = Optimizer(
        runner=_runner(catalog_path),
        objective=_objective(),
        concurrency=ConcurrencyPolicy.sequential(),
    )
    space = ParameterSpace(
        base={
            "instrument_id": str(INSTRUMENT_ID),
            "bar_type": BAR_TYPE,
            "fast_ema_period": 10,
            "slow_ema_period": 30,
        },
        parameters=(Parameter("trade_size", (TRADE_SIZE, "not-a-decimal")),),
    )

    report = optimizer.optimize(space)

    print(f"survivors={len(report.results)} failures={len(report.failures)}")  # noqa: T201
    for failure in report.failures:
        print(f"failure={failure.error_type}: {failure.error_message}")  # noqa: T201

    assert len(report.results) == 1
    assert len(report.failures) == 1
    assert report.results[0].experiment.parameters["trade_size"] == TRADE_SIZE
    assert report.best() is not None
    assert report.failures[0].experiment.parameters["trade_size"] == "not-a-decimal"
    assert report.failures[0].error_type == "RuntimeError"
    assert "not-a-decimal" in report.failures[0].error_message


def test_stages_are_distinct(tmp_path: Path) -> None:
    """
    Test train, optimize, and validate are distinct stages over the same search.
    """
    catalog_path = _build_catalog(tmp_path / "catalog")
    optimizer = Optimizer(
        runner=_runner(catalog_path),
        objective=_objective(),
        constraints=(Constraint(DRAWDOWN, ConstraintComparison.AT_LEAST, -0.013),),
        concurrency=ConcurrencyPolicy.sequential(),
    )

    trained = TrainStage(optimizer).run(_space())
    optimized = OptimizeStage(optimizer).run(_space())
    assert trained is not None
    assert optimized.best() is not None
    assert trained.digest == optimized.best().digest

    validated = ValidateStage(optimizer).run(trained.experiment)
    assert isinstance(validated, ValidationResult)
    assert not validated.constraints_satisfied
    assert validated.violations == (DRAWDOWN,)

    print(f"trained={dict(trained.experiment.parameters)}")  # noqa: T201
    print(f"trained_score={trained.score!r}")  # noqa: T201
    print(f"validate_violations={validated.violations}")  # noqa: T201


def test_unevaluable_objective_is_recorded(tmp_path: Path) -> None:
    """
    Test a run whose objective cannot be evaluated is recorded, never scored as zero.
    """
    catalog_path = _build_catalog(tmp_path / "catalog")
    optimizer = Optimizer(
        runner=_runner(catalog_path),
        objective=Objective([ObjectiveTerm("Expectancy", 1.0, ObjectiveDirection.MAXIMIZE)]),
        concurrency=ConcurrencyPolicy.sequential(),
    )
    space = ParameterSpace(
        base={
            "instrument_id": str(INSTRUMENT_ID),
            "bar_type": BAR_TYPE,
            "trade_size": TRADE_SIZE,
            "slow_ema_period": 30,
        },
        parameters=(Parameter("fast_ema_period", (10,)),),
    )

    report = optimizer.optimize(space)

    print(f"survivors={len(report.results)} failures={len(report.failures)}")  # noqa: T201
    for failure in report.failures:
        print(f"failure={failure.error_type}: {failure.error_message}")  # noqa: T201

    assert report.results == ()
    assert report.best() is None
    assert len(report.failures) == 1
    assert report.failures[0].error_type == "ValueError"
    assert "Expectancy" in report.failures[0].error_message


def test_fan_out_matches_sequential(tmp_path: Path) -> None:
    """
    Test process fan-out and sequential execution agree on the best result and the digests.
    """
    catalog_path = _build_catalog(tmp_path / "catalog")
    runner = _runner(catalog_path)
    objective = _objective()

    sequential = Optimizer(
        runner=runner,
        objective=objective,
        concurrency=ConcurrencyPolicy.sequential(),
    ).optimize(_space())
    fanned = Optimizer(
        runner=runner,
        objective=objective,
        concurrency=ConcurrencyPolicy(max_workers=2),
    ).optimize(_space())

    sequential_best = sequential.best()
    fanned_best = fanned.best()
    assert sequential_best is not None
    assert fanned_best is not None

    print(f"sequential_digests={sequential.digests}")  # noqa: T201
    print(f"fanned_digests={fanned.digests}")  # noqa: T201
    print(f"sequential_best={sequential_best.score!r} {sequential_best.digest}")  # noqa: T201
    print(f"fanned_best={fanned_best.score!r} {fanned_best.digest}")  # noqa: T201

    assert fanned.digests == sequential.digests
    assert fanned_best.digest == sequential_best.digest
    assert fanned_best.score == sequential_best.score
    assert dict(fanned_best.experiment.parameters) == dict(sequential_best.experiment.parameters)


def test_two_runs_of_one_study_produce_the_same_result_digest(tmp_path: Path) -> None:
    """
    Test a re-run of the same study over the same data reproduces the result digest.

    A changed numerical kernel changes the implementation identity while the study identity is
    stable, so a numerical drift is visible rather than a mystery.
    """
    catalog_path = _build_catalog(tmp_path / "catalog")
    runner = _runner(catalog_path)
    objective = _objective()
    space = _space()

    study = StudyIdentity(
        dataset=DatasetIdentity(
            dataset_digest="sha256:" + "0" * 64,
            universe=UniverseIdentity(
                universe_digest="sha256:" + "1" * 64,
                membership_policy_id="static",
            ),
            adjustment_policy="raw",
        ),
        parameter_space_digest="sha256:" + "2" * 64,
        objective_definition=objective_definition_from_terms(objective.terms),
        selection_rule=SelectionRule.RANK_FIRST,
        metric_set=tuple(sorted(term.metric for term in objective.terms)),
    )

    first = Optimizer(
        runner=runner,
        objective=objective,
        concurrency=ConcurrencyPolicy.sequential(),
    ).optimize(space)
    second = Optimizer(
        runner=runner,
        objective=objective,
        concurrency=ConcurrencyPolicy.sequential(),
    ).optimize(space)

    first_best = first.best()
    second_best = second.best()
    assert first_best is not None
    assert second_best is not None

    first_trial = trial_identity(study.study_id, first_best.run, objective_value=first_best.score)
    second_trial = trial_identity(
        study.study_id, second_best.run, objective_value=second_best.score
    )

    assert first_trial.trial_id == second_trial.trial_id
    assert first_trial.result_digest == second_trial.result_digest

    computation = ComputationIdentity(
        code_version="2.0.0rc6",
        numeric_kernel_version="nautilus-analysis-0.65.0",
        numerical_backend="rust",
        parameter_digest=first_trial.parameter_digest,
    )
    first_result = ResearchResult(
        study=study,
        trial=first_trial,
        computation=computation,
        assumption_policy=BarAmbiguityPolicy.declared_default(),
        metric_results=tuple(sorted(term.metric for term in objective.terms)),
    )
    second_result = ResearchResult(
        study=study,
        trial=second_trial,
        computation=computation,
        assumption_policy=BarAmbiguityPolicy.declared_default(),
        metric_results=tuple(sorted(term.metric for term in objective.terms)),
    )

    print(f"study_id={study.study_id}")  # noqa: T201
    print(f"trial_id={first_trial.trial_id}")  # noqa: T201
    print(f"result_digest={first_result.result_digest}")  # noqa: T201

    assert first_result.result_digest == second_result.result_digest

    # A different numerical kernel is a different implementation identity, and the study is
    # unchanged: the drift is visible rather than a mystery.
    other_computation = dataclasses.replace(computation, numeric_kernel_version="0.66.0")
    other_result = ResearchResult(
        study=study,
        trial=first_trial,
        computation=other_computation,
        assumption_policy=BarAmbiguityPolicy.declared_default(),
    )

    assert other_result.study.study_id == first_result.study.study_id
    assert other_result.computation.digest != first_result.computation.digest
    assert other_result.result_digest != first_result.result_digest


def test_experiment_store_round_trip(tmp_path: Path) -> None:
    """
    Test a sweep reloads from its digest-keyed store with the same best result.
    """
    catalog_path = _build_catalog(tmp_path / "catalog")
    store = ExperimentStore(tmp_path / "store")
    optimizer = Optimizer(
        runner=_runner(catalog_path),
        objective=_objective(),
        concurrency=ConcurrencyPolicy.sequential(),
    )
    report = optimizer.optimize(_space(), store=store)

    reloaded = store.load_report()

    print(f"store_files={sorted(p.name for p in store.directory.rglob('*.json'))}")  # noqa: T201
    assert reloaded.digests == report.digests
    assert len(reloaded.results) == len(report.results)
    assert reloaded.best() is not None
    assert report.best() is not None
    assert reloaded.best().digest == report.best().digest
    assert reloaded.best().score == report.best().score
    assert reloaded.best().run.canonical_result == report.best().run.canonical_result
    assert len(store.load_experiments()) == 4


def test_walk_forward_splits_and_evaluates_out_of_sample(tmp_path: Path) -> None:
    """
    Test walk-forward searches in-sample and evaluates the selected model out-of-sample.
    """
    catalog_path = _build_catalog(tmp_path / "catalog")
    bars = _bars(MAX_ROWS)
    start = bars[0].ts_init
    end = bars[-1].ts_init
    span = end - start
    windows = walk_forward_windows(
        start,
        end,
        in_sample=span // 2,
        out_of_sample=span // 4,
    )
    assert windows == (
        WalkForwardWindow(
            start,
            start + span // 2,
            start + span // 2,
            start + span // 2 + span // 4,
        ),
    )

    optimizer = Optimizer(
        runner=_runner(catalog_path),
        objective=_objective(),
        concurrency=ConcurrencyPolicy.sequential(),
    )
    report = WalkForwardStage(in_sample=optimizer, out_of_sample=optimizer).run(
        _space(),
        windows,
    )

    window = report.windows[0]
    assert window.selected is not None
    assert isinstance(window.out_of_sample, ExperimentResult)
    assert window.training is not None
    assert window.training.best() is not None
    assert window.out_of_sample.digest != window.training.best().digest
    assert len(report.out_of_sample_results()) == 1

    print(f"selected={dict(window.selected.parameters)}")  # noqa: T201
    print(  # noqa: T201
        f"out_of_sample_digest={window.out_of_sample.digest} score={window.out_of_sample.score!r}",
    )
