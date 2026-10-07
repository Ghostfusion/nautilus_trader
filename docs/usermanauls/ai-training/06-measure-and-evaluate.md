# 06 - Measuring and evaluating the search

This lecture reads the report a sweep produces and then measures the report itself. It explains the
deflated Sharpe ratio and the trial-provenance requirement in plain words, and shows why the
annualisation and kurtosis conventions have to be declared rather than assumed.

## The report numbers

A `SearchReport` carries a ranked list of results (see
`python/nautilus_trader/optimization/report.py`):

| Field                | Meaning                                                                       |
| -------------------- | ----------------------------------------------------------------------------- |
| `results`            | The survivors, ranked by descending score with deterministic tie-breaking.    |
| `failures`           | The experiments that raised or could not be scored, sorted by digest.         |
| `evaluated`          | The number of distinct experiments scored, executed or reused.                |
| `space_size`         | The number of experiments the searched space expands to.                      |
| `executions`         | The number actually executed; a resumed run lowers this by reusing its cache. |
| `evaluated_fraction` | `evaluated / space_size`, or None when the size is undeclared.                |
| `scheme`             | The validation scheme the search ran under, or None.                          |
| `seed`               | The seed the search was derived from, or None.                                |

A single `ExperimentResult` carries the canonical run, the objective `score`, and
`constraints_satisfied`. Constraints are evaluated before the objective, so an infeasible candidate
is recorded with `feasible=False` and no fictitious penalty in its score: a constraint violation is
not a low score, it is a separate attribute.

## What a good and a bad value look like

- **Score.** Its scale is set by the objective. A Sharpe term of 9 is high and, when it is the best
  of a search, suspicious for exactly that reason. A negative Sharpe term is normal for a strategy
  with no edge. The score is only interpretable against the space it came from.
- **`failures`.** Zero failures on a run whose objective references a metric the run does not report
  is a warning sign you have configured a metric that is silently available; failures on a specific
  experiment pattern usually mean a parameter value the strategy cannot handle.
- **`evaluated_fraction`.** Below 1.0 means the search sampled the space. That is fine, but the
  winner is then the best of what was tried, not the best of the space.
- **`executions` versus `evaluated`.** Equal on a first run; `executions` lower than `evaluated` on a
  resumed run means the cache did its job.

## The three most common misreadings

### Misreading 1: the best score is an estimate

It is a selection. A sweep that ran 500 parameter sets and reports the winner's Sharpe ratio has
reported the maximum of 500 estimates. Lecture 01 computed the arithmetic: 100 noise trials with a
0.10 spread give an expected best annualised Sharpe near 4 with no skill at all. The correction below
is what turns a maximum back into a probability.

### Misreading 2: an unavailable number is zero

A statistic that cannot be computed is not zero. `per_period_sharpe` returns a value of `None` when
fewer than two periods contribute or when the excess returns have no dispersion
(`python/nautilus_trader/optimization/significance.py`), and the correction reports `unavailable`
with a reason rather than a value. In an objective, a missing metric is an error that fails the
experiment, never a zero that ranks it.

### Misreading 3: two searches are comparable because both print a Sharpe ratio

They are comparable only when their declarations are. The `RunDescription` digest covers the space,
the validation scheme, the seed, the search strategy and the cache; the `StatisticalContract` digest
covers the correction's rules; the `assumptions` block covers the bar-execution rules. Two numbers
produced under different declarations are not the same measurement.

## The deflated Sharpe ratio in plain words

Suppose you search many parameter sets and report the best one's Sharpe ratio `SR`. Ask a different
question: if all of those parameter sets had no skill, what is the largest Sharpe ratio the search
would likely have produced? Call that `SR0`. The correction reports the probability that the selected
trial's value exceeds that no-skill maximum. The formula the engine implements is:

```text
SR0 = sqrt(V[SR]) * ((1 - gamma) * Phi^-1(1 - 1/N) + gamma * Phi^-1(1 - 1/(N * e)))
DSR = Phi((SR - SR0) * sqrt(T - 1) / sqrt(1 - skew * SR + ((kurtosis - 1) / 4) * SR^2))
```

`V[SR]` is the cross-trial variance of the Sharpe estimates, `N` the counted trial count, `T` the
contributing periods, and `gamma` is the Euler-Mascheroni constant. `Phi` is the standard normal
distribution, so the value is a probability in `[0, 1]`. A larger `N` raises `SR0`, which lowers the
corrected value: that is the correction, not an artifact of it. The implementation is
`deflated_sharpe_ratio` in `python/nautilus_trader/optimization/significance.py`.

## The trial-provenance requirement in plain words

The correction cannot be computed from the winning number alone. It needs to know how many trials
were run: the nominal count, and whether some of them were dependent so that an effective count is
the honest one. It also needs the failed trials counted and identified, because a trial that failed
is still a trial that was searched. `TrialProvenance` carries the trial count, the failed-trial count,
and whether the study distinguishes a nominal from an effective count
(`python/nautilus_trader/optimization/identity.py`). `trial_provenance_from_runs` refuses a duplicate
parameter set rather than counting it twice, because two runs of one experiment are one trial.

## Why the conventions must be declared

Two conventions change the corrected number even when the underlying data is the same, so the engine
refuses to guess.

- **Annualisation is prohibited.** A Sharpe ratio annualised with `sqrt(252)` is not a per-period
  Sharpe ratio, and a correction defined per period would silently divide it. The built-in
  `Sharpe Ratio (simple, sample, 252 days)` statistic is annualised and tagged `Annualised`
  (`crates/analysis/src/statistics/sharpe_ratio.rs`, `docs/concepts/optimization.md`). The sample
  declares its frequency, and the annualised declaration is an error; a caller with an annualised
  value must rescale it before the correction.
- **Kurtosis is non-excess.** The fourth standardized moment is at least 1 for any distribution, so a
  caller passing an excess kurtosis (0 for normal returns) is using the wrong convention and is
  refused rather than silently reinterpreted.

## The correction program

Save as `significance.py` in the repository root and run it with `uv run --project python --no-sync
python significance.py`. It builds 40 synthetic per-period Sharpe estimates under no skill, then
corrects the best of them for the trial count.

```python
from __future__ import annotations

import random

from nautilus_trader.optimization import (
    SharpeFrequency,
    SharpeSample,
    StatisticalContract,
    TrialDependence,
    deflated_sharpe_ratio,
    per_period_sharpe,
    return_moments,
)

PERIODS = 250
RNG = random.Random(2026)


def build_trials(count: int) -> list[tuple[list[float], float]]:
    trials = []
    for _ in range(count):
        returns = [RNG.gauss(0.0, 0.01) for _ in range(PERIODS)]
        estimate = per_period_sharpe(returns)
        assert estimate.value is not None
        trials.append((returns, estimate.value))
    return trials


if __name__ == "__main__":
    contract = StatisticalContract.declared_default()
    declared = contract.to_dict()
    print("contract:", contract.contract_id, "v", contract.version)
    print("annualisation:", declared["annualisation"])
    print("kurtosis_convention:", declared["kurtosis_convention"])
    print(
        "minimum_observations:",
        declared["minimum_observations"],
        "minimum_trials:",
        declared["minimum_trials"],
    )

    trials = build_trials(40)
    selected_returns, selected_sharpe = max(trials, key=lambda item: item[1])
    moments = return_moments(selected_returns)
    assert moments is not None
    print(f"best-of-40 per-period Sharpe: {selected_sharpe:.4f}")
    print(f"moments: skew {moments.skew:.4f}, non-excess kurtosis {moments.kurtosis:.4f}")

    for count in (10, 20, 40):
        sample = SharpeSample(
            sharpe=selected_sharpe,
            trial_sharpes=[sharpe for _, sharpe in trials[:count]],
            observations=PERIODS,
            skew=moments.skew,
            kurtosis=moments.kurtosis,
            dependence=TrialDependence.INDEPENDENT,
        )
        result = deflated_sharpe_ratio(sample)
        value = "unavailable" if result.value is None else f"{result.value:.6f}"
        print(f"N={count}: DSR={value} status={result.to_dict()['status']}")

    try:
        SharpeSample(
            sharpe=selected_sharpe,
            trial_sharpes=[sharpe for _, sharpe in trials[:10]],
            observations=PERIODS,
            skew=moments.skew,
            kurtosis=moments.kurtosis,
            dependence=TrialDependence.INDEPENDENT,
            frequency=SharpeFrequency.ANNUALISED,
        )
    except ValueError as exc:
        print("annualised refused:", exc)

    try:
        SharpeSample(
            sharpe=selected_sharpe,
            trial_sharpes=[sharpe for _, sharpe in trials[:10]],
            observations=PERIODS,
            skew=moments.skew,
            kurtosis=0.0,
            dependence=TrialDependence.INDEPENDENT,
        )
    except ValueError as exc:
        print("excess kurtosis refused:", exc)
```

The observed output:

```text
contract: deflated_sharpe_ratio v 1
annualisation: prohibited
kurtosis_convention: non_excess
minimum_observations: 20 minimum_trials: 10
best-of-40 per-period Sharpe: 0.1579
moments: skew 0.2527, non-excess kurtosis 3.0487
N=10: DSR=0.759091 status=computed
N=20: DSR=0.603103 status=computed
N=40: DSR=0.536042 status=computed
annualised refused: the correction is per-period: an annualised input (annualised) must be rescaled by its caller, not divided here
excess kurtosis refused: kurtosis must be the non-excess convention and therefore at least 1, was 0.0: an excess kurtosis of 0 is not a kurtosis here
```

The best of 40 noise trials has a per-period Sharpe of 0.1579, and the corrected probability falls as
the trial count grows: 0.759 for 10 trials, 0.603 for 20, 0.536 for 40. The direction is the point.
The two refusals show the conventions are enforced at the boundary, not left to the reader.

## The provenance program

The correction needs counts, and the counts come from the runs. This driver adds provenance to the
search program from lecture 05; save it as `provenance.py` in the repository root, reusing the
strategy, the configuration factory and the data loader from that lecture. Add these imports:

```python
from nautilus_trader.optimization import (
    DatasetIdentity,
    SelectionRule,
    StudyIdentity,
    UniverseIdentity,
    objective_definition_from_terms,
    trial_provenance_from_runs,
)
from nautilus_trader.optimization.runner import FailedExperiment
from nautilus_trader.optimization.space import Experiment
```

```python
if __name__ == "__main__":
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        _CATALOG_PATH = root / "catalog"
        _CATALOG_PATH.mkdir()
        catalog = ParquetDataCatalog(str(_CATALOG_PATH))
        catalog.write_instruments([INSTRUMENT])
        catalog.write_bars(load_bars())

        space = ParameterSpace(
            base={
                "instrument_id": INSTRUMENT_ID,
                "bar_type": BAR_TYPE,
                "trade_size": TRADE_SIZE,
            },
            parameters=(
                Parameter("fast_ema_period", (5, 10, 15)),
                Parameter("slow_ema_period", (20, 30, 40)),
            ),
        )

        def make_optimizer(search):
            return Optimizer(
                runner=BacktestRunner(
                    config_factory=config_factory,
                    strategy_path="__main__:EmaCrossStrategy",
                    config_path="__main__:EmaCrossConfig",
                ),
                objective=Objective(
                    [
                        ObjectiveTerm(SHARPE, 1.0, ObjectiveDirection.MAXIMIZE),
                        ObjectiveTerm(DRAWDOWN, 1.0, ObjectiveDirection.MAXIMIZE),
                    ],
                ),
                constraints=(Constraint(DRAWDOWN, ConstraintComparison.AT_LEAST, -0.05),),
                concurrency=ConcurrencyPolicy(max_workers=1),
                search=search,
            )

        search = RandomSearch(seed=7, budget=6)
        report = make_optimizer(search).optimize(space)
        runs = [result.run for result in report.results]

        provenance = trial_provenance_from_runs(runs)
        print("provenance:", provenance.trial_count, "trials,", provenance.failed_trial_count, "failed")
        print("nominal_and_effective:", provenance.nominal_and_effective)

        failed = FailedExperiment(Experiment({"fast_ema_period": 7}), "ValueError", "demo")
        with_failure = trial_provenance_from_runs([*runs, failed])
        print("with one failure:", with_failure.trial_count, "trials,", with_failure.failed_trial_count, "failed")

        try:
            trial_provenance_from_runs([*runs, runs[0]])
        except ValueError as exc:
            print("duplicate refused:", exc)

        study = StudyIdentity(
            dataset=DatasetIdentity(
                dataset_digest="sha256:demo-dataset",
                universe=UniverseIdentity(
                    universe_digest="sha256:demo-universe",
                    membership_policy_id="static",
                ),
                adjustment_policy="raw",
            ),
            parameter_space_digest=space.digest,
            objective_definition=objective_definition_from_terms(
                make_optimizer(search).objective.terms,
            ),
            selection_rule=SelectionRule.RANK_FIRST,
            metric_set=(SHARPE, DRAWDOWN),
            study_seed=search.seed,
        )
        print("study_id:", study.study_id[:24])
```

The observed output:

```text
provenance: 6 trials, 0 failed
nominal_and_effective: False
with one failure: 7 trials, 1 failed
duplicate refused: duplicate parameter sets in one study: ['sha256:cbaad00ab8161fbee1285f5002df39d0568fd0c3f8839b1fcd2cc9860122241e']
study_id: sha256:2c1b9ab5ca05bf3f8
```

The six runs are six trials, none failed, and the study declares independent trials so
`nominal_and_effective` is `False`. Adding one failure makes seven trials with one failed. Adding a
duplicate of an existing run is refused: two runs of one experiment are one trial, and counting them
as two would inflate `N` and over-correct the result.

## A boundary you must know

The correction is not wired into the emitted result document. The configuration declares no dataset,
and a run record retains its metric values and canonical document rather than its return series, so a
sweep cannot be corrected from what it currently keeps (`docs/concepts/optimization.md`,
"Multiple-testing-aware reporting"). To correct a real sweep you supply the per-period returns of the
trials yourself. The program above uses a clearly labelled synthetic family to demonstrate the
arithmetic; wiring the correction to the bars of a real sweep is your job, because the run record
does not carry the returns.

## A note on the reward signal

For reinforcement-learning or reward-shaping work, a periodic result frame is the natural reward
source. `PerformancePeriod` attributes accounting, trading activity and exposure to a calendar period
and adds derived performance such as net return and drawdown
(`docs/concepts/performance_periods.md`). Its boundaries are plain UTC calendar boundaries and the
same reducer runs in a live process and a backtest, so the same reward definition applies in both.
Use the frame as the reward, and remember the same overfitting arithmetic applies: a reward the agent
maximises over many episodes is a selection too.
