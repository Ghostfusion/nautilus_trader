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
Integration composition tests for the strategy-discovery matrix (design Section 14).

One test per composition point of Section 14 that can be composed from Python without a backtest
engine run. Each test constructs the declared contract, drives it, and reads the declared result
back, and its docstring names the Section 14 row it proves. Nothing here starts a node, a worker
process or a live adapter, and nothing reads an adapter environment variable: the only inputs are
the bundled test kit, the decision bridge's own fixtures, and `tmp_path`.
"""

from __future__ import annotations

import json
from decimal import Decimal
from pathlib import Path

import pytest

import nautilus_trader.optimization
from nautilus_trader.adapters.sandbox import SandboxExecutionClientConfig
from nautilus_trader.adapters.sandbox import SandboxExecutionClientFactory
from nautilus_trader.analysis import Constraint
from nautilus_trader.analysis import ConstraintComparison
from nautilus_trader.analysis import MetricDefinition
from nautilus_trader.analysis import MetricDirection
from nautilus_trader.analysis import MetricInput
from nautilus_trader.analysis import MetricReason
from nautilus_trader.analysis import MetricReport
from nautilus_trader.analysis import MetricStage
from nautilus_trader.analysis import MetricStatus
from nautilus_trader.analysis import MetricTag
from nautilus_trader.analysis import MetricUnits
from nautilus_trader.analysis import Objective
from nautilus_trader.analysis import ObjectiveDirection
from nautilus_trader.analysis import ObjectiveTerm
from nautilus_trader.analysis import PortfolioAnalyzer
from nautilus_trader.analysis import SharpeRatio
from nautilus_trader.core import Capability
from nautilus_trader.decision_bridge.contract import Outcome
from nautilus_trader.decision_bridge.projection import ProjectionConfig
from nautilus_trader.decision_bridge.projection import ProjectionContext
from nautilus_trader.decision_bridge.projection import project
from nautilus_trader.decision_bridge.ratings import PositionState
from nautilus_trader.decision_bridge.tradability import Tradability
from nautilus_trader.model import AccountId
from nautilus_trader.model import Currency
from nautilus_trader.model import InstrumentId
from nautilus_trader.model import Money
from nautilus_trader.model import SignalDirection
from nautilus_trader.model import Venue
from nautilus_trader.optimization import BacktestRunner
from nautilus_trader.optimization import BarAmbiguityPolicy
from nautilus_trader.optimization import CanonicalRun
from nautilus_trader.optimization import ConcurrencyPolicy
from nautilus_trader.optimization import DatasetIdentity
from nautilus_trader.optimization import ExecutionStatus
from nautilus_trader.optimization import Experiment
from nautilus_trader.optimization import ExperimentResult
from nautilus_trader.optimization import ExperimentStore
from nautilus_trader.optimization import FailedExperiment
from nautilus_trader.optimization import IntrabarPath
from nautilus_trader.optimization import LabelOverlapRule
from nautilus_trader.optimization import LeakagePolicy
from nautilus_trader.optimization import Optimizer
from nautilus_trader.optimization import OptimizeStage
from nautilus_trader.optimization import OutOfSampleStage
from nautilus_trader.optimization import ResearchCapabilityCode
from nautilus_trader.optimization import SelectionRule
from nautilus_trader.optimization import Split
from nautilus_trader.optimization import SplitContract
from nautilus_trader.optimization import StatisticalContract
from nautilus_trader.optimization import StudyIdentity
from nautilus_trader.optimization import TrainStage
from nautilus_trader.optimization import TrialIdentity
from nautilus_trader.optimization import TrialProvenance
from nautilus_trader.optimization import UniverseIdentity
from nautilus_trader.optimization import ValidateStage
from nautilus_trader.optimization import WalkForwardReport
from nautilus_trader.optimization import WalkForwardResult
from nautilus_trader.optimization import WalkForwardStage
from nautilus_trader.optimization import WalkForwardWindow
from nautilus_trader.optimization import gapped_range_capability
from nautilus_trader.optimization import history_capability
from nautilus_trader.optimization import objective_definition_from_terms
from nautilus_trader.optimization import pair_distinguishability_capability
from nautilus_trader.optimization import permutation_test
from nautilus_trader.optimization import persistence_capability
from nautilus_trader.optimization import screen_family_capability
from nautilus_trader.optimization import significance_capability
from nautilus_trader.optimization import split_capability
from nautilus_trader.optimization import total_return
from nautilus_trader.optimization import trial_identity
from nautilus_trader.optimization import trial_provenance_from_runs
from nautilus_trader.optimization import walk_forward_windows
from nautilus_trader.optimization.concurrency import DEFAULT_PER_RUN_BYTES
from nautilus_trader.optimization.concurrency import memory_budget_fraction
from nautilus_trader.optimization.permutation import PERMUTATION_CONTRACT_ID
from nautilus_trader.optimization.permutation import PERMUTATION_CONTRACT_VERSION
from nautilus_trader.optimization.permutation import PERMUTATION_RECORD_VERSION
from nautilus_trader.optimization.significance import SIGNIFICANCE_CONTRACT_VERSION
from nautilus_trader.optimization.space import digest_of
from nautilus_trader.research import FeatureValue
from nautilus_trader.research import MembershipInterval
from nautilus_trader.research import MembershipSeries
from nautilus_trader.research import Panel
from nautilus_trader.research import PanelRow
from tests.providers import TestInstrumentProvider
from tests.unit.decision_bridge import fixtures


NS = 1_000_000_000
SECOND = NS
START = 1_700_000_000 * SECOND
GIB = 1024 * 1024 * 1024

SHARPE_METRIC = "Sharpe Ratio (simple, sample, 252 days)"
SHARPE_METRIC_ID = "sharpe_ratio"

HORIZON_NS = 5 * 86_400 * NS
RECEIVED_AT = fixtures.unix_ns(2025, 6, 2, 17)


def _no_backtest_factory(_start: int | None, _end: int | None) -> tuple[object, ...]:
    """
    Refuse to build a run configuration: no composition test executes a backtest.
    """
    raise AssertionError("a composition test may not execute a backtest")


def _optimizer() -> Optimizer:
    """
    Build an optimizer whose stages are composed but whose runner is never asked to run.
    """
    return Optimizer(
        runner=BacktestRunner(
            config_factory=_no_backtest_factory,
            strategy_path="strategies.ema_cross:EMACross",
            config_path="strategies.ema_cross:EMACrossConfig",
        ),
        objective=Objective([ObjectiveTerm(SHARPE_METRIC, 1.0, ObjectiveDirection.MAXIMIZE)]),
        concurrency=ConcurrencyPolicy.sequential(),
    )


def _canonical_run(parameters: dict[str, object]) -> CanonicalRun:
    """
    Build a canonical run over a parameter set, without executing a backtest.

    The canonical digest is derived from the experiment digest, so it names the result rather than
    the parameter set and is distinct from the experiment's own digest.
    """
    experiment = Experiment(parameters)
    document = json.dumps(
        {"schema": "nautilus-backtest-result/v1", "parameters": dict(parameters)},
        sort_keys=True,
        separators=(",", ":"),
    ).encode("utf-8")
    return CanonicalRun(
        experiment=experiment,
        canonical_digest=digest_of({"result": experiment.digest}),
        metric_values={SHARPE_METRIC: 1.0},
        canonical_result=document,
    )


def _study() -> StudyIdentity:
    """
    Build the declared study identity the identity test constructs twice.
    """
    objective = Objective([ObjectiveTerm(SHARPE_METRIC, 1.0, ObjectiveDirection.MAXIMIZE)])
    return StudyIdentity(
        dataset=DatasetIdentity(
            dataset_digest=digest_of({"dataset": "btc-perp-20211231-20220201_1m", "rows": 120}),
            universe=UniverseIdentity(
                universe_digest=digest_of({"membership": "static", "members": 1}),
                membership_policy_id="static",
                membership_as_of=START,
            ),
            source_version="test-data",
            as_of=START,
            calendar_identity="BINANCE",
            adjustment_policy="raw",
            missing_data_policy="drop",
        ),
        parameter_space_digest=digest_of({"base": "ema_cross", "swept": 2}),
        objective_definition=objective_definition_from_terms(objective.terms),
        selection_rule=SelectionRule.RANK_FIRST,
        metric_set=(SHARPE_METRIC,),
    )


def _positive_leakage() -> LeakagePolicy:
    """
    Build a leakage policy with every interval decided, so no zero needs a justification.
    """
    return LeakagePolicy(
        purge_before=SECOND,
        purge_after=SECOND,
        embargo_after=SECOND,
    )


def _split_contract(*, n_splits: int | None = None) -> SplitContract:
    """
    Build a two-set split contract of a ten-second window.
    """
    return SplitContract(
        sets=("train", "test"),
        lengths={"train": 6 * SECOND, "test": 4 * SECOND},
        leakage=_positive_leakage(),
        n_splits=n_splits,
    )


def test_a_split_contract_with_purge_embargo_and_label_overlap_declares_its_bounds() -> None:
    """
    Test the Section 14 row "Split contracts with purge and embargo" (`optimization/splits.py`).

    A contract with purge, embargo and a label-overlap rule produces splits with the declared
    bounds, and its leakage relation is mandatory.
    """
    leakage = LeakagePolicy(
        purge_before=SECOND,
        purge_after=SECOND,
        embargo_after=SECOND // 2,
        label_overlap_rule=LabelOverlapRule.ENFORCE,
        label_horizon=2 * SECOND,
    )
    contract = SplitContract(
        sets=("train", "test"),
        lengths={"train": 6 * SECOND, "test": 4 * SECOND},
        leakage=leakage,
    )

    assert contract.evaluation_set == "test"
    assert contract.resolved_lengths() == {"train": 6 * SECOND, "test": 4 * SECOND}
    # The label horizon is folded into the purge before the evaluation set, so it decides it.
    assert leakage.purge == 2 * SECOND
    assert leakage.decided == frozenset({"purge_before", "purge_after", "embargo_after"})

    splits = contract.split(START, START + 30 * SECOND)

    assert all(isinstance(split, Split) for split in splits)
    assert len(splits) == 2
    assert splits[0].bounds == {
        "train": (START, START + 4 * SECOND),
        "test": (START + 6 * SECOND, START + 10 * SECOND),
    }
    assert splits[1].bounds == {
        "train": (START + 10 * SECOND + SECOND // 2, START + 14 * SECOND + SECOND // 2),
        "test": (START + 16 * SECOND + SECOND // 2, START + 20 * SECOND + SECOND // 2),
    }
    assert splits[0].lengths() == {"train": 4 * SECOND, "test": 4 * SECOND}
    # The embargo spaces the splits, so the stride is the window plus the declared gap.
    assert splits[1].bounds["test"][1] - splits[0].bounds["test"][1] == 10 * SECOND + SECOND // 2

    timestamps = [START + index * SECOND for index in range(30)]
    assert splits[0].indices(timestamps) == {"train": (0, 1, 2, 3), "test": (6, 7, 8, 9)}

    with pytest.raises(TypeError, match="leakage must be a LeakagePolicy"):
        SplitContract(
            sets=("train", "test"),
            lengths={"train": 6 * SECOND, "test": 4 * SECOND},
            leakage=None,
        )


def test_walk_forward_windows_are_produced_and_the_stages_compose() -> None:
    """
    Test the Section 14 row "Walk-forward windows and stages" (`optimization/stages.py`).

    The windows are produced from the period and the stage dataclasses compose; no stage is asked
    to run, because a run executes backtests.
    """
    windows = walk_forward_windows(
        START,
        START + 25 * SECOND,
        in_sample=8 * SECOND,
        out_of_sample=2 * SECOND,
    )

    assert windows == (
        WalkForwardWindow(START, START + 8 * SECOND, START + 8 * SECOND, START + 10 * SECOND),
        WalkForwardWindow(
            START + 10 * SECOND,
            START + 18 * SECOND,
            START + 18 * SECOND,
            START + 20 * SECOND,
        ),
    )
    # The default policy declares no exclusion, so each window's segments are adjacent.
    for window in windows:
        assert window.in_sample_end == window.out_of_sample_start
        assert window.out_of_sample_end <= START + 25 * SECOND

    optimizer = _optimizer()
    stages = (
        TrainStage(optimizer=optimizer),
        OptimizeStage(optimizer=optimizer),
        ValidateStage(optimizer=optimizer),
        OutOfSampleStage(optimizer=optimizer),
    )

    assert [stage.optimizer for stage in stages] == [optimizer] * len(stages)

    walk_forward = WalkForwardStage(in_sample=optimizer, out_of_sample=optimizer)
    assert walk_forward.in_sample is optimizer
    assert walk_forward.out_of_sample is optimizer

    record = WalkForwardResult(
        window=windows[0],
        training=None,
        selected=None,
        out_of_sample=None,
    )
    report = WalkForwardReport((record,))

    assert report.windows == (record,)
    assert report.out_of_sample_results() == ()


def test_the_statistical_contract_round_trips_and_provenance_refuses_duplicates() -> None:
    """
    Test the Section 14 row "Deflated Sharpe ratio and trial provenance" (`significance.py`).

    The statistical contract round-trips through `to_dict()` with a stable digest, and
    `trial_provenance_from_runs` refuses duplicate parameter sets.
    """
    contract = StatisticalContract()
    other = StatisticalContract.declared_default()

    assert contract.to_dict() == other.to_dict()
    assert contract.digest == other.digest
    assert contract.digest.startswith("sha256:")

    payload = contract.to_dict()
    assert json.loads(json.dumps(payload)) == payload
    assert payload["version"] == SIGNIFICANCE_CONTRACT_VERSION

    changed = StatisticalContract(risk_free_rate=0.001)
    assert changed.to_dict()["return_definition"]["risk_free_rate"] == 0.001
    assert changed.digest != contract.digest

    first = _canonical_run({"fast_ema_period": 5})
    second = _canonical_run({"fast_ema_period": 10})
    failure = FailedExperiment(Experiment({"fast_ema_period": 20}), "RuntimeError", "boom")

    provenance = trial_provenance_from_runs((first, second, failure))

    assert provenance == TrialProvenance(trial_count=3, failed_trial_count=1)
    assert provenance.to_dict() == {
        "trial_count": 3,
        "failed_trial_count": 1,
        "nominal_and_effective": False,
    }

    with pytest.raises(ValueError, match="duplicate parameter sets"):
        trial_provenance_from_runs((first, _canonical_run({"fast_ema_period": 5})))


def test_the_permutation_test_returns_a_versioned_result_record() -> None:
    """
    Test the Section 14 row "Permutation test" (`optimization/permutation.py`).

    The test returns a result whose record states the record version and the contract version it
    was produced under, and the same seed reproduces the same record.
    """
    outcomes = [0.01, -0.02, 0.03, 0.005, -0.001, 0.02]

    result = permutation_test(
        outcomes,
        statistic=total_return,
        metric="total_return",
        iterations=200,
        seed=7,
    )
    record = result.to_record()

    assert record["record_version"] == PERMUTATION_RECORD_VERSION
    assert record["contract"] == PERMUTATION_CONTRACT_ID
    assert record["contract_version"] == PERMUTATION_CONTRACT_VERSION
    assert record["metric"] == "total_return"
    assert record["seed"] == 7
    assert record["iterations"] == 200
    assert record["observations"] == len(outcomes)
    assert record["refused"] == 0
    assert json.loads(json.dumps(record)) == record
    assert result.scored == result.iterations - result.refused
    assert 0.0 < result.p_value <= 1.0

    repeated = permutation_test(
        outcomes,
        statistic=total_return,
        metric="total_return",
        iterations=200,
        seed=7,
    )

    assert repeated.to_record() == record


def test_study_trial_and_universe_identities_are_stable_across_constructions() -> None:
    """
    Test the Section 14 row "Study, trial and universe identity" (`identity.py`, `space.py`).

    `StudyIdentity`, `TrialIdentity`, `TrialProvenance`, `trial_identity` and `digest_of` produce
    the same digests from two independent constructions. `digest_of` is reachable only as
    `nautilus_trader.optimization.space.digest_of`, because it is not in the package `__all__`,
    which the last assertion states.
    """
    first = _study()
    second = _study()

    assert first.study_id == second.study_id
    assert first.study_id.startswith("sha256:")
    assert len(first.study_id) == len("sha256:") + 64

    run = _canonical_run({"fast_ema_period": 5, "slow_ema_period": 20})
    first_trial = trial_identity(first.study_id, run, seed=7, objective_value=1.25)
    second_trial = trial_identity(second.study_id, run, seed=7, objective_value=1.25)

    assert first_trial.trial_id == second_trial.trial_id
    assert first_trial.parameter_digest == run.experiment.digest
    assert dict(first_trial.parameter_values) == dict(run.experiment.parameters)
    assert first_trial.execution_status is ExecutionStatus.COMPLETED
    assert first_trial.result_digest == run.canonical_digest

    from_values = TrialIdentity(
        study_id=first.study_id,
        parameter_digest=first_trial.parameter_digest,
        parameter_values=dict(run.experiment.parameters),
        execution_status=ExecutionStatus.COMPLETED,
        seed=7,
        objective_value=1.25,
        result_digest=run.canonical_digest,
    )

    assert from_values == first_trial

    with pytest.raises(ValueError, match="does not digest the parameter values"):
        TrialIdentity(
            study_id=first.study_id,
            parameter_digest=digest_of({"fast_ema_period": 6}),
            parameter_values=dict(run.experiment.parameters),
            execution_status=ExecutionStatus.COMPLETED,
            result_digest=run.canonical_digest,
        )

    provenance = TrialProvenance(trial_count=2, failed_trial_count=1)

    assert provenance.to_dict() == {
        "trial_count": 2,
        "failed_trial_count": 1,
        "nominal_and_effective": False,
    }

    with pytest.raises(ValueError, match="exceeds"):
        TrialProvenance(trial_count=1, failed_trial_count=2)

    parameters = {"fast_ema_period": 5, "slow_ema_period": 20}

    assert digest_of(parameters) == digest_of(parameters)
    assert digest_of(parameters) == run.experiment.digest
    # The digest is over the canonically serialized set, so key order does not change it.
    assert digest_of({"slow_ema_period": 20, "fast_ema_period": 5}) == run.experiment.digest
    assert digest_of({"fast_ema_period": 5, "slow_ema_period": 21}) != run.experiment.digest
    assert "digest_of" not in nautilus_trader.optimization.__all__


def test_the_experiment_store_keys_a_result_by_its_canonical_digest(tmp_path: Path) -> None:
    """
    Test the Section 14 row "Digest-keyed artifact store" (`optimization/persistence.py`).

    A store over a `tmp_path` directory writes a result under its canonical digest and reads it,
    its metric values and its canonical document back.
    """
    store = ExperimentStore(tmp_path / "store")

    assert store.directory == tmp_path / "store"
    for name in ("experiments", "results", "failures", "canonical"):
        assert (store.directory / name).is_dir()

    run = _canonical_run({"fast_ema_period": 5})
    result = ExperimentResult(run=run, score=1.25, constraints_satisfied=True)

    experiment_key = run.experiment.digest.replace(":", "_")
    canonical_key = run.canonical_digest.replace(":", "_")

    experiment_path = store.write_experiment(run.experiment)
    result_path = store.write_result(result)

    assert run.canonical_digest != run.experiment.digest
    assert experiment_path == store.directory / "experiments" / f"{experiment_key}.json"
    assert result_path == store.directory / "results" / f"{canonical_key}.json"
    assert store.canonical_bytes(run.canonical_digest) == run.canonical_result

    loaded = store.load_results()

    assert len(loaded) == 1
    assert loaded[0].digest == run.canonical_digest
    assert loaded[0].score == 1.25
    assert loaded[0].constraints_satisfied is True
    assert dict(loaded[0].experiment.parameters) == {"fast_ema_period": 5}
    assert loaded[0].run.metric_values == {SHARPE_METRIC: 1.0}
    assert loaded[0].run.canonical_result == run.canonical_result
    assert [experiment.digest for experiment in store.load_experiments()] == [
        run.experiment.digest,
    ]


def test_the_concurrency_policy_derives_its_worker_count_from_memory() -> None:
    """
    Test the Section 14 row "Process fan-out and memory budget" (`concurrency.py`).

    The policy declares the memory budget and the pure arithmetic derives the worker count from it;
    the multi-worker path is not run, because it starts processes.
    """
    assert DEFAULT_PER_RUN_BYTES == 512 * 1024 * 1024

    assert memory_budget_fraction(available=8 * GIB, per_run_bytes=GIB, fraction=0.75) == 6
    assert memory_budget_fraction(available=GIB, per_run_bytes=GIB, fraction=1.0) == 1
    # The derived count is floored at one rather than reporting an unusable zero.
    assert memory_budget_fraction(available=GIB // 2, per_run_bytes=GIB, fraction=0.75) == 1
    assert memory_budget_fraction(available=0, per_run_bytes=GIB, fraction=0.75) == 1

    sequential = ConcurrencyPolicy.sequential()

    assert sequential.max_workers == 1
    assert sequential.worker_count() == 1

    capped = ConcurrencyPolicy(per_run_bytes=GIB, memory_fraction=0.5, max_workers=3)

    assert 1 <= capped.worker_count() <= 3
    assert capped.per_run_bytes == GIB
    assert ConcurrencyPolicy().per_run_bytes == DEFAULT_PER_RUN_BYTES

    with pytest.raises(ValueError, match="per_run_bytes must be positive"):
        ConcurrencyPolicy(per_run_bytes=0)
    with pytest.raises(ValueError, match="memory_fraction must be in"):
        ConcurrencyPolicy(memory_fraction=0.0)
    with pytest.raises(ValueError, match="max_workers must be at least 1"):
        ConcurrencyPolicy(max_workers=0)


def test_the_capability_probes_answer_with_a_code_from_the_closed_set() -> None:
    """
    Test the Section 14 row "Capability probes" (`optimization/capability.py`).

    Every probe returns a capability: a refusal carries a code drawn from the domain's closed code
    set and a requirement, and an available answer carries no code.
    """
    codes = frozenset(code.value for code in ResearchCapabilityCode)

    assert len(codes) == len(ResearchCapabilityCode)

    available = Capability.available()

    assert available.is_available
    assert available.code is None

    refusals = {
        "significance": significance_capability(observations=5, trials=2),
        "history": history_capability(START + SECOND, required_start=START),
        "gapped_range": gapped_range_capability(
            [START, START + 2 * SECOND],
            start=START,
            end=START + 4 * SECOND,
            stride=SECOND,
        ),
        "screen_family": screen_family_capability(0),
        "persistence": persistence_capability(0.5, 100),
        "pair": pair_distinguishability_capability(
            [1.0, 2.0, 3.0],
            [1.0, 2.0, 3.0],
            tolerance=0.5,
        ),
        "split": split_capability(
            _split_contract(n_splits=5),
            start=START,
            end=START + 12 * SECOND,
        ),
    }

    for name, capability in refusals.items():
        assert not capability.is_available, name
        assert capability.code in codes, name
        assert capability.requirements, name
        assert capability.detail, name

    assert (
        significance_capability(observations=5, trials=2).code
        == ResearchCapabilityCode.INSUFFICIENT_OBSERVATIONS.value
    )
    assert (
        history_capability(START + SECOND, required_start=START).code
        == ResearchCapabilityCode.HISTORY_AFTER_WINDOW.value
    )
    assert (
        screen_family_capability(0).code == ResearchCapabilityCode.EFFECTIVE_TRIALS_UNDECLARED.value
    )
    assert persistence_capability(0.5, 100).code == ResearchCapabilityCode.NOT_MEAN_REVERTING.value
    assert (
        pair_distinguishability_capability(
            [1.0, 2.0, 3.0],
            [1.0, 2.0, 3.0],
            tolerance=0.5,
        ).code
        == ResearchCapabilityCode.PAIR_INDISTINGUISHABLE.value
    )
    assert (
        split_capability(
            _split_contract(n_splits=5),
            start=START,
            end=START + 12 * SECOND,
        ).code
        == ResearchCapabilityCode.SPLIT_LAYOUT_UNSATISFIABLE.value
    )

    # A contract that fits the period is available rather than refused.
    assert split_capability(_split_contract(), start=START, end=START + 12 * SECOND).is_available
    assert significance_capability(observations=100, trials=20).is_available


def test_the_declared_bar_ambiguity_policy_round_trips_with_a_stable_digest() -> None:
    """
    Test the Section 14 row "Declared execution assumptions" (`optimization/assumptions.py`).

    `BarAmbiguityPolicy.declared_default()` round-trips through its canonical mapping with a stable
    digest, and an ambiguous declaration is refused rather than resolved by convention.
    """
    policy = BarAmbiguityPolicy.declared_default()
    other = BarAmbiguityPolicy.declared_default()

    assert policy == other
    assert policy.digest == other.digest
    assert policy.digest.startswith("sha256:")

    payload = policy.to_dict()

    assert json.loads(json.dumps(payload)) == payload
    assert policy.digest == digest_of(payload)
    assert payload["policy_id"] == "bar_ohlc"
    assert payload["version"] == 1
    assert payload["bar_execution"] is True
    assert payload["intrabar_path"] == IntrabarPath.OHLC_SEQUENCE.value
    assert policy.intrabar_path is IntrabarPath.OHLC_SEQUENCE
    assert (
        BarAmbiguityPolicy.from_venue_flags(bar_execution=True, adaptive_high_low_ordering=False)
        == policy
    )

    with pytest.raises(ValueError, match="intrabar path"):
        BarAmbiguityPolicy(bar_execution=True, intrabar_path=None)


def test_a_panel_and_a_membership_series_are_built_and_read_back() -> None:
    """
    Test the Section 14 row "Point-in-time panel, features, labels, membership, operators".

    The row's Python surface is `nautilus_trader.research`, the only research surface bound to
    Python: a `Panel` is built from `PanelRow`/`FeatureValue` rows over a `MembershipSeries` built
    from `MembershipInterval` entries, and a value is read back.
    """
    universe = "TEST-UNIVERSE"
    source = "bundled"
    instrument = TestInstrumentProvider.aapl_equity().id
    other = InstrumentId.from_str("MSFT.XNAS")

    memberships = MembershipSeries(universe, source)
    memberships.push(MembershipInterval(universe, source, instrument, entered_at=START))
    memberships.push(
        MembershipInterval(
            universe,
            source,
            other,
            entered_at=START,
            exited_at=START + 10 * SECOND,
        ),
    )

    assert memberships.universe() == universe
    assert memberships.source() == source
    assert sorted(str(item) for item in memberships.instruments()) == [str(instrument), str(other)]
    assert sorted(str(item) for item in memberships.members_at(START)) == [
        str(instrument),
        str(other),
    ]
    assert memberships.is_member_at(instrument, START) is True
    assert memberships.is_member_at(other, START + 20 * SECOND) is False
    assert len(memberships.intervals()) == 2

    feature = FeatureValue(191.34, START)

    assert feature.value == 191.34
    assert feature.as_of == START

    rows = [
        PanelRow(instrument, START)
        .with_feature("close", feature.value, feature.as_of)
        .with_label(0.0025)
        .with_member(True),
        PanelRow(other, START).with_member(True),
    ]
    panel = Panel(memberships, rows)
    panel.check()

    assert panel.len() == 2
    assert not panel.is_empty()
    assert panel.universe() == universe

    read_back = panel.rows()[0]

    assert read_back.instrument_id == instrument
    assert read_back.ts_event == START
    assert read_back.member is True
    assert read_back.label == 0.0025
    assert read_back.features["close"].value == 191.34
    assert read_back.features["close"].as_of == START


def test_the_analyzer_metadata_and_objective_address_one_metric() -> None:
    """
    Test the Section 14 row "Portfolio statistics and objective registry" (`crates/analysis/src/`).

    `PortfolioAnalyzer` registers a statistic, the metric metadata types describe it, and
    `Objective`/`ObjectiveTerm`/`Constraint` construct over the metric it names. The report addresses
    the metric by its stable id and the objective and constraint by the statistic's declared name,
    which the metadata renders as the same metric's title.
    """
    analyzer = PortfolioAnalyzer()
    analyzer.register_statistic(SharpeRatio())

    definitions = analyzer.metric_definitions()

    assert len(definitions) == 1

    definition = definitions[0]

    assert isinstance(definition, MetricDefinition)
    assert definition.id == SHARPE_METRIC_ID
    assert definition.title == SHARPE_METRIC
    assert definition.title == SharpeRatio().name
    assert definition.units is MetricUnits.RATIO
    assert definition.direction is MetricDirection.MAXIMIZE
    assert definition.inputs == [MetricInput.RETURNS]
    assert MetricTag.RISK_ADJUSTED in definition.tags
    assert definition.is_derived is False
    # The declared stage travels with the metadata and is the stage the analyzer reports.
    assert analyzer.metric_stages() == {definition.title: definition.parameters["stage"]}
    assert definition.parameters["stage"] == "account"

    analyzer.set_declared_stages([MetricStage.ACCOUNT, MetricStage.DECISION])
    assert analyzer.declared_stages() == [MetricStage.ACCOUNT, MetricStage.DECISION]

    report = analyzer.report_returns_metrics([definition.id])

    assert isinstance(report, MetricReport)
    assert len(report) == 1

    reported = report.get(definition.id)

    assert reported is not None
    assert reported.id == SHARPE_METRIC_ID
    assert reported.title == SHARPE_METRIC
    assert reported.status is MetricStatus.UNAVAILABLE
    assert reported.reason is MetricReason.INSUFFICIENT_DATA

    unavailable = report.with_status(MetricStatus.UNAVAILABLE)

    assert [item.id for item in unavailable] == [definition.id]
    assert unavailable[0].reason is MetricReason.INSUFFICIENT_DATA
    assert report.with_status(MetricStatus.COMPUTED) == []

    objective = Objective([ObjectiveTerm(definition.title, 1.0, ObjectiveDirection.MAXIMIZE)])
    constraint = Constraint(definition.title, ConstraintComparison.AT_LEAST, 1.0)
    values = {definition.title: 1.5}

    assert objective.terms[0].metric == definition.title
    assert objective.terms[0].contribution(values) == 1.5
    assert objective.evaluate(values) == 1.5
    assert constraint.metric == definition.title
    assert constraint.is_satisfied(values) is True
    assert constraint.is_satisfied({definition.title: 0.5}) is False


def test_the_decision_bridge_projects_one_artifact() -> None:
    """
    Test the Section 14 row "Decision-to-next-session projection" (`decision_bridge/`).

    One canonical artifact is projected with a context and a config built the way the bridge's own
    tests build them, and the projected signal reads back its instrument, direction and ceiling.
    """
    config = ProjectionConfig(
        horizon_ns=HORIZON_NS,
        authorized_producers=fixtures.AUTHORIZED_PRODUCERS,
    )
    context = ProjectionContext(
        received_at=RECEIVED_AT,
        tradability=Tradability.TRADABLE,
        position=PositionState.FLAT,
        engine_eligible=True,
        calendar=fixtures.bundled_calendar(),
        regime=None,
    )

    result = project(
        fixtures.covered_artifact(),
        context=context,
        config=config,
        resolve_instrument=fixtures.resolve_instrument,
    )

    assert not result.refused
    assert result.outcome is Outcome.SIGNAL
    assert result.actionable_at == RECEIVED_AT

    signal = result.signal

    assert signal is not None
    assert signal.instrument_id == fixtures.AAPL
    assert signal.direction is SignalDirection.LONG
    assert signal.horizon_ns == HORIZON_NS
    assert signal.ts_event == RECEIVED_AT
    assert result.ceiling == Decimal("0.05")


def test_the_sandbox_execution_client_config_and_factory_report_their_names() -> None:
    """
    Test the Section 14 row "Sandbox venue for shadow fills" (`adapters/sandbox/`).

    The execution client config and its factory construct and report their names, without starting
    a node or a live adapter.
    """
    venue = Venue("SANDBOX")
    balances = [Money(1_000_000, Currency.from_str("USD"))]

    config = SandboxExecutionClientConfig(venue=venue, starting_balances=balances)
    factory = SandboxExecutionClientFactory()

    assert factory.name() == "SANDBOX"
    assert config.venue == venue
    assert str(config.venue) == "SANDBOX"
    assert config.account_id == AccountId("SANDBOX-SANDBOX-001")
    assert config.starting_balances == balances
    assert config.bar_execution is True
    assert config.trade_execution is True
