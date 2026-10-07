# Optimization

Parameter optimization is a research capability that composes backtest runs. It searches a
parameter space, executes each experiment through `BacktestNode`, scores the results with an
objective and constraints from `nautilus_trader.analysis`, and ranks the survivors. It is a
Python-only orchestration layer over the Rust engine: it composes runs and never alters one.

## Pipeline

The pipeline is a parameter space, a search strategy, a run, an evaluation, and a result:

1. **Parameter space.** A space holds fixed base values and named parameters with discrete,
   ordered choices. Expansion is deterministic: the Cartesian product in declaration order, with
   the last parameter varying fastest. The same space always yields the same sweep in the same
   order.
2. **Experiment.** A complete parameter set, canonically serialized (sorted keys, compact
   separators, strict JSON) and digested as `sha256:<hex>`. The digest depends only on the set,
   never on a clock or randomness, so equal sets produce equal digests.
3. **Search strategy.** Enumerates the experiments to evaluate. The grid strategy is the space's
   own expansion; a strategy never executes a run.
4. **Runner.** Builds exactly one `BacktestRunConfig` whose strategy config carries the parameter
   set through `ImportableStrategyConfig`, runs it through `BacktestNode`, and reads the canonical
   result and the run's statistics back. The run config ID is the experiment digest, so the same
   experiment always produces the same canonical result digest.
5. **Evaluation.** An `Objective` and any `Constraint`s from `nautilus_trader.analysis` are
   evaluated over each run's metric values. The objective is the only objective implementation;
   there is no name table in this subsystem.
6. **Result.** A canonical backtest result plus the parameter set, comparable by digest. A
   `SearchReport` ranks the survivors by descending score and records the failures.

A run that fails, and a run whose objective or constraints cannot be evaluated (a metric value is
missing, for example), is recorded as a failed experiment with its typed error. It is never scored
as zero, and it never aborts the sweep.

## Boundary

The optimizer composes runs and must never reach into one. Every backtest goes through
`BacktestNode`; this subsystem does not construct an engine, does not own the data or venue
configuration, and does not alter the deterministic semantics of an individual backtest. The
caller supplies a configuration factory that returns the venue, data, and engine configurations
for a run window; the runner adds only the run identity and the strategy.

## Stage model

The methodology stages are distinct, not one loop over a grid. The search is the shared primitive;
the stages differ in what they do with its outcome.

| Stage         | Does                                                                                                                                         | Does not                                            |
| ------------- | -------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------- |
| Train         | Searches the training window and selects one experiment, the model                                                                           | Rank beyond the selection or evaluate out of sample |
| Optimize      | Produces the ranked report of a search                                                                                                       | Force a single selection                            |
| Validate      | Evaluates one experiment and reports whether the constraints hold                                                                            | Search or reselect                                  |
| Out-of-sample | Evaluates the selected model on a window it was not selected on                                                                              | Reselect on that window                             |
| Walk-forward  | Splits a period into in-sample and out-of-sample segments, searches on the former and evaluates the selected model on the latter, per window | Pool the windows into one search                    |

`walk_forward_windows` cuts a period into consecutive windows of a fixed in-sample length followed
by a fixed out-of-sample length; the out-of-sample segment never outlives the period. Each stage
composes runs through the runner and re-windows the runner for its segment.

### Split contracts and leakage

The window bounds come from a `SplitContract`, which owns the layout rather than each caller
improvising one. The contract names its sets, takes each set's length in nanoseconds, as a fraction
of the window, or omitted for one set that absorbs the rest, and lays the windows out whole with
every set positively long. Its `direction` decides where a leftover span at the end of the period
goes, and `n_splits` selects that many evenly spaced windows rather than the first ones.

Leakage is an exclusion relation, not a time distance. A `LeakagePolicy` states it:

| Field                | Effect                                                                                               |
| -------------------- | ---------------------------------------------------------------------------------------------------- |
| `purge_before`       | Removes the training tail adjacent to the evaluation set.                                            |
| `purge_after`        | Moves a training set that follows the evaluation set.                                                |
| `embargo_after`      | Sets the minimum gap between one split and the next.                                                 |
| `label_overlap_rule` | Folds a declared `label_horizon` into `purge_before`, excluding an overlapping observation outright. |

Labels look forward, so a training observation within one label horizon of the evaluation start
carries evaluation information whatever the distance between the sets. A zero interval is
permitted, but not silently: an interval left unset and an interval decided to be zero are
distinguishable, and any zero interval requires a `zero_interval_justification`. The walk-forward
stages declare their zero policy with its reason in
`nautilus_trader.optimization.stages.WALK_FORWARD_LEAKAGE`, because those stages compute statistics
from results realised inside each window and declare no label horizon. A study whose labels or
features reach across a window boundary declares a policy of its own under `stage.leakage`.

### Execution assumptions

A bar records four prices and no path between them, so a bar-driven replay must assume an ordering,
a fill rule for a triggered stop, a precedence between simultaneous triggers, and a treatment for a
bar that opens beyond a trigger. Those are assumptions about an unknowable intrabar path, not market
rules, and this subsystem records them rather than owning them: the implementation lives in the
matching engine, and a study declares which rule set it ran under.

`BarAmbiguityPolicy` is that declaration, with an identity. Its `policy_id` and `version` name the
rule set, and its fields name each rule:

| Field                | Members                                              | Meaning                                                       |
| -------------------- | ---------------------------------------------------- | ------------------------------------------------------------- |
| `bar_execution`      | `bool`                                               | Whether bars drive order execution at all.                    |
| `intrabar_path`      | `OHLC_SEQUENCE`, `ADAPTIVE_NEAREST_TO_OPEN`, or None | The order the bar's prices are visited in.                    |
| `trigger_precedence` | `OHLC_SEQUENCE`                                      | Which trigger wins when several are reachable in one bar.     |
| `trigger_fill`       | `TRIGGER_PRICE_INSIDE_BAR`                           | The fill price when the bar does not open beyond the trigger. |
| `gap_handling`       | `MARKET_PRICE_BEYOND_TRIGGER`                        | The fill price when the bar opens beyond the trigger.         |

Two configurations that cannot both apply are refused rather than resolved: `from_venue_flags`
rejects adaptive ordering without bar execution, and a policy that declares an intrabar path while
bar execution is off (or bar execution with no declared path) raises at construction. An ambiguous
declaration is a configuration error, not a silent convention, so two studies cannot look identical
while assuming different things.

The declaration is a top-level `assumptions` block in a configuration file, spelled as the venue
configuration spells the flags:

```json
{
  "assumptions": {
    "bar_execution": true,
    "adaptive_high_low_ordering": false
  }
}
```

`BarAmbiguityPolicy.declared_default()` names the default a study runs under when it declares
nothing: bar execution with the fixed Open, High, Low, Close sequence, which is the behaviour the
engine already implements. The emitted result document carries the policy as `assumptions`, so a
result states the rule set that produced it and two results produced under different versions are
distinguishable by their policy digest.

Every assumption above maps to an implementation and an assertion:

| Assumption                                                                            | Implementation                                              | Assertion                                                                                                  |
| ------------------------------------------------------------------------------------- | ----------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| Bars drive execution only on an L1 book with external aggregation                     | `crates/execution/src/matching_engine/mod.rs:1907`, `:1914` | `crates/backtest/tests/integration/backtest_engine.rs` `test_add_data_rejects_bar_internal_aggregation`    |
| Fixed Open, High, Low, Close order                                                    | `crates/execution/src/matching_engine/mod.rs:2325`          | `crates/execution/tests/integration/matching_engine.rs` `test_bar_execution_fills_stop_order`              |
| Adaptive order visits the nearer extreme first                                        | `crates/execution/src/matching_engine/mod.rs:2325`, `:2040` | `test_bar_adaptive_ordering_fills_low_side_first`, `test_quote_bar_adaptive_ordering_fills_low_side_first` |
| A trigger inside the bar fills at the trigger price; a gap fills at the market price  | `crates/execution/src/matching_engine/mod.rs:4560-4590`     | `docs/concepts/backtesting/fill-prices-and-matching.md` and the engine's bar-fill tests                    |
| Simultaneous triggers follow the price sequence rather than a configurable precedence | `crates/execution/src/matching_engine/mod.rs:2325`          | `docs/concepts/backtesting/bar-execution.md` describes the ordering as the resolution                      |

This is a research record, not an execution model. Nothing here changes an engine configuration or
a fill; the policy states what a result assumed, which is why it stays on this side of the boundary
described above.

## Labels and the target path

A label is a future outcome. Every construction that produces one reads observations a feature cannot
see, so the label layer is a separate path with a separate type: no module under `trading`, `live`,
`backtest`, `execution`, `risk` or `adapters` imports it, and a label series handed back where market
data is expected is refused by name rather than read. Both are asserted, so a label value cannot
quietly become a feature.

`labels.py` carries the first tranche: the fixed-horizon forward return, the forward aggregates of a
window's per-bar returns (mean, standard deviation, minimum, maximum), and a first-hit label that
reports which of two independent thresholds the cumulative return from the entry breached first.

`LabelDefinition` states the outcome, the window and the alignment:

| Field                                      | Meaning                                                                                                                   |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------- |
| `label_id`                                 | The policy's identifier, unique within a study.                                                                           |
| `kind`                                     | `FORWARD_RETURN`, `FORWARD_AGGREGATE` or `FIRST_HIT_THRESHOLD`.                                                           |
| `horizon`                                  | The forward window length in observations.                                                                                |
| `wait`                                     | The observations to wait before the entry; a zero wait still excludes the anchor bar from the window.                     |
| `alignment`                                | Which label row a feature row is paired with: the anchor bar or the next bar.                                             |
| `missing_data_policy`                      | `PROPAGATE` requires every window observation present; `SKIP` counts present observations, so the window spans more time. |
| `aggregate`                                | The reduction for `FORWARD_AGGREGATE`, required by that kind and refused by the others.                                   |
| `positive_threshold`, `negative_threshold` | The barriers of a first-hit label; independent rather than one and its negation.                                          |

The window of a label anchored at a bar is the `horizon` observations after the `wait`, and its entry
reference is the close of the `wait`-th observation, so a zero wait enters at the anchor's own close
while the window still starts after it. `FORWARD_RETURN` measures the endpoint ratio over the window,
`FORWARD_AGGREGATE` reduces the window's per-bar returns (the mean with a compensated accumulation,
the standard deviation with the sample divisor the dispersion kernel uses), and
`FIRST_HIT_THRESHOLD` scans the cumulative return from the entry and reports `1` for the positive
barrier, `-1` for the negative one and `0` when neither is reached. The scan is over closes rather
than highs and lows, so one observation cannot breach both barriers; an intrabar comparison would need
the bar ordering policy, which is a study decision rather than a label one.

The alignment convention is first-class because it is invisible in the shape of the result. Pairing a
feature row with the label anchored at that row and with the label anchored at the next row produce
the same values shifted by one row, and they are not the same dataset: the second measures its outcome
over a later bar and reads one bar further into the future.

That reach is what a leakage policy has to cover. `LabelSeries.forward_reach_ns` is measured from the
produced series rather than derived from the definition, so it accounts for the alignment, for the
wait, for a first hit that stops early, and for a skipped window that spans more time than its bar
count. `validate_leakage` refuses a policy whose effective purge before the evaluation set is shorter
than the reach and reports the shortfall in nanoseconds, so the fix is a number rather than a
judgement; the intervals themselves stay a study decision, and the check states the requirement
without choosing the value.

Every field is part of the definition digest, so two datasets assembled under different conventions
cannot share an identity. Extrema and trend-state labels are not built: they need the dataset
contract of the earlier review, and neither it nor a stored point-in-time membership exists.

A label's provenance is part of the record. `DatasetDeclaration::new` requires a
`LabelDefinition`, so a dataset cannot be declared without stating how its labels were produced, and
the definition is part of the dataset's identity: two datasets whose labels were produced
differently are different datasets. In the measurement layer (`crates/research`), an admitted record
carries an optional `label_definition`, and a record whose definition is absent is counted by
`LabelProvenance.undefined` and is not scored - a coefficient or calibration over labels of unknown
provenance is refused rather than computed, and the count is carried into the report instead. A
stream with no undefined labels reports `is_fully_defined` true and an `undefined_share` of `0.0`;
an empty stream reports an `undefined_share` of `None`, because a share of nothing is not zero.

## Relative-value declarations

A screen over a universe of related series is a set of trials, and the trials are what the correction
needs. A screen that reports its survivors without the family they came from has reported a selection
with no count behind it, however good its gates are. `relative_value.py` carries the declarations a
screen has to make before it reports anything, each one data with a digest and each one answered by
the capability probe that owns its refusal.

- `ScreenFamily` declares the universe and enumerates every unique pair of it, so a screen over `n`
  series states `n * (n - 1) / 2` tests before any gate runs, and `ScreenOutcome` records the tests
  that were evaluated and the ones that survived. `ScreenOutcome.significance` computes the
  correction over the *evaluated* tests rather than over the survivors, because the dispersion the
  correction needs is the dispersion of the trials that ran.
- `PersistenceEstimate` carries a fitted coefficient on the lagged level together with the convention
  its half-life is converted under: `ln(0.5) / ln(1 + coefficient)` for the discrete convention and
  `-ln(2) / coefficient` for the continuous one, which are different numbers for one fit, so the
  convention is declared rather than assumed. A fit below `minimum_observations`, or one outside the
  revertible interval `(-1, 0)`, is answered as `unavailable` instead of being converted.
- `SignalWindow` declares the fit window, the measurement window and the label whose measured reach
  the signal carries, and refuses a declaration whose fit window does not end at or before the
  measurement window's start. The reach is measured from the produced label series rather than
  derived from the declaration, which is exactly what `leakage_capability` has to cover.
- `RecursiveMemory` declares the state noise and the observation noise of a recursion together with
  the memory the study believes it has and the warm-up it discards. A scalar recursion settles at
  `P = (Q + sqrt(Q^2 + 4QR)) / 2` and `K = P / (P + R)`, so its memory is `1 / K` and is set by both
  noises: a declaration whose memory is not the settled one, or whose warm-up is shorter than it, is
  refused at construction rather than producing a number nobody chose.

The refusals are the research vocabulary's own closed codes. A screen that declares no universe is
`EFFECTIVE_TRIALS_UNDECLARED` and one whose family exceeds its declared maximum is
`EFFECTIVE_TRIALS_OUT_OF_RANGE`; a gapped range, a history that begins after the window, two series
that are indistinguishable up to scale and shift, and a universe too small for the requested family
are `GAPPED_RANGE`, `HISTORY_AFTER_WINDOW`, `PAIR_INDISTINGUISHABLE` and `UNIVERSE_TOO_SMALL`; and a
fit that does not revert is `NOT_MEAN_REVERTING`. Each carries the requirement it did not meet, and
no caller reads a detail.

## Identity contracts

Provenance is part of the meaning of a result: a number that cannot name the study that produced it,
the data it read, the code that computed it and the assumptions it made is an observation, not a
research result. `identity.py` makes that naming mechanical, and the digests compose from values
that already exist where they can: the parameter digest is the experiment digest, the result digest
is the canonical backtest document digest, and the leakage and assumption policies carry their own.

| Contract              | Carries                                                                                                                                                                                            |
| --------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `StudyIdentity`       | The dataset identity, the parameter-space digest, the objective definition, the selection rule, the metric set, the split contract and leakage policy, and the study seed. Stable across a re-run. |
| `TrialIdentity`       | The study, the parameter values and their digest, the seed, the execution status, the objective value and the canonical result digest.                                                             |
| `TrialProvenance`     | The trial and failed-trial counts, and whether the study can distinguish a nominal count from an effective one.                                                                                    |
| `DatasetIdentity`     | The dataset digest, the source version, the as-of time, the calendar identity, the universe identity, the adjustment policy and the missing-data policy.                                           |
| `UniverseIdentity`    | The universe digest, the membership policy id and the membership as-of time.                                                                                                                       |
| `ComputationIdentity` | The code version, the schema version, the numeric kernel version, the numerical backend and the parameter digest.                                                                                  |
| `ResearchResult`      | The study, the trial, the computation identity, the assumption policy and the metric results.                                                                                                      |

```python
study = StudyIdentity(
    dataset=DatasetIdentity(
        dataset_digest="sha256:...",
        universe=UniverseIdentity(
            universe_digest="sha256:...",
            membership_policy_id="static",
        ),
        adjustment_policy="raw",
    ),
    parameter_space_digest="sha256:...",
    objective_definition=objective_definition_from_terms(objective.terms),
    selection_rule=SelectionRule.RANK_FIRST,
    metric_set=("Sharpe Ratio (simple, sample, 252 days)", "Max Drawdown (simple)"),
)
trial = trial_identity(study.study_id, report.best().run)
result = ResearchResult(
    study=study,
    trial=trial,
    computation=computation,
    assumption_policy=BarAmbiguityPolicy.declared_default(),
)
```

Three properties are structural rather than conventional. A result cannot be built without a study
identity, because its constructor requires one and rejects a trial that belongs to a different
study. A trial cannot record a parameter digest that does not describe its own parameter values, and
a completed trial must record its result digest. A changed numeric kernel version changes the
implementation identity while the study identity stays stable, so a numerical drift is visible
rather than a mystery.

Two boundaries are stated rather than implied:

- **The counts are not part of a study's identity.** They describe how a study ended, and a transient
  failure would otherwise change the identity of the study that suffered it. `TrialProvenance`
  carries them for the correction that needs them.
- **A universe identity is a declaration, not yet a guarantee.** Nothing in this repository stores
  point-in-time membership history: a `UniverseIdentity` records the policy and the as-of time its
  caller declares, and the stored-membership workstream proposed by the earlier design review
  (`vnpy_lessons_design.md`, decisions D5 and D13) is what would make membership recoverable rather
  than re-evaluated. Until then a study can state its membership, and the statement is auditable but
  not enforced.

## Multiple-testing-aware reporting

The best of a sweep is a selection, not an estimate. A study that ran five hundred parameter sets and
reports the winner's Sharpe ratio has reported the maximum of five hundred estimates, and the expected
maximum of many estimates is positive even when none of them has skill. `significance.py` carries the
correction and, before the correction, the statistical contract it is computed under: the contract
declares every element rather than leaving any of them to the implementation, carries an identity and a
digest, and is recorded with every result.

| Element                     | Declared as                                                                                       |
| --------------------------- | ------------------------------------------------------------------------------------------------- |
| Return definition           | The per-period excess return series, with the compounding convention stated (`simple` by default) |
| Risk-free treatment         | An explicit rate or an explicit zero, never an implicit assumption                                |
| Minimum observations        | Twenty contributing periods: below it the statistic is `unavailable`, not computed                |
| Minimum trials              | Ten trial estimates: below it the correction is noise on noise and the statistic is `unavailable` |
| Trial independence          | Declared by the study, not inferred: a dependent study must supply its effective trial count      |
| Trial dependence            | The nominal and effective counts are distinguished, and the counted one is recorded               |
| Sharpe convention           | Per-period, with the sample or population divisor pinned                                          |
| Variance, skew, kurtosis    | The estimators are named, and the kurtosis convention is non-excess                               |
| Annualisation               | Prohibited: an annualised input is refused at the boundary rather than divided silently           |
| Missing returns             | Excluded rather than zero-filled, and the horizon counts contributing periods                     |
| Failed and duplicate trials | A duplicate parameter set is refused; a failed trial is counted and identified                    |

The initial statistic is the deflated Sharpe ratio, the probability that the selected trial's Sharpe
ratio exceeds the maximum that the declared number of trials would have produced with no skill:

```text
SR0 = sqrt(V[SR]) * ((1 - gamma) * Phi^-1(1 - 1/N) + gamma * Phi^-1(1 - 1/(N * e)))
DSR = Phi((SR - SR0) * sqrt(T - 1) / sqrt(1 - skew * SR + ((kurtosis - 1) / 4) * SR^2))
```

`V[SR]` is the cross-trial variance of the Sharpe estimates, `N` the counted trial count, `T` the
contributing periods and `gamma` the Euler-Mascheroni constant. The value is a probability in
`[0, 1]`, and it falls as the trial count grows: that is the correction, not an artifact of it.

```python
estimate = per_period_sharpe(run.returns_series, risk_free_rate=0.0)
moments = return_moments(run.returns_series)
sample = SharpeSample(
    sharpe=estimate.value,
    trial_sharpes=[...],  # one per-period Sharpe per trial that produced one
    observations=estimate.observations,
    skew=moments.skew,
    kurtosis=moments.kurtosis,
    dependence=TrialDependence.INDEPENDENT,
)
result = deflated_sharpe_ratio(sample)
report = significance_report(study, runs, sample)
```

The statuses are the analysis statistics' own four-state vocabulary, so a correction is read the same
way a metric is. Below a minimum count, or with fewer than two trial estimates, the result is
`unavailable` with `insufficient_data`; a variance factor that is not positive is `invalid` with
`undefined_result`; and a result that is not computed always carries a reason and no value.

The bound is read at both specification extremes, not reported as one figure. The correction is
computed once per declared trial specification, and `deflated_sharpe_bounds()` returns the
`most_favourable` and `least_favourable` bounds as a pair, each naming the specification that
produced it: a bound carries its `extreme`, its `specification` and its `specification_label`
beside its `value`. A run that declares no specification, or a single one, is refused rather than
given one figure - `the significance bound could not be checked: the search declared a single
specification ({label}), and a bound is only a bound once it is read at two extremes` - and a bound
report carrying one extreme is refused with `a significance bound report must carry both
specification extremes: a single bound cannot be compared and is refused`. The Rust
`information_coefficient_bounds()` carries the same pair and refuses the same way. The statistic's
own row names the specification it was read under, as in
`Deflated Sharpe Ratio (12 trials, value, window 1000-2000)`.

Three boundaries are enforced rather than documented:

- **An annualised input is refused.** The built-in `Sharpe Ratio (simple, sample, 252 days)` statistic is annualised
  and tagged `Annualised`, and a correction defined per period would silently divide it. The sample
  declares its frequency, and the annualised declaration is an error.
- **The kurtosis convention is non-excess.** The fourth standardized moment is at least 1 for any
  distribution, so an excess kurtosis of 0, which is the wrong convention, is refused rather than
  reinterpreted.
- **Dependence is declared.** A sweep over adjacent parameters is not an independent sample, and the
  correction cannot tell from the values alone. A dependent study supplies its effective trial count;
  an independent study may not.

**The value is reported, never a gate.** Nothing here is consulted by a strategy, an order or a risk
check. The correction is also not wired into the emitted result document: the configuration declares
no dataset, and a run record retains its metric values and canonical document rather than its return
series, so a sweep cannot be corrected from what it currently keeps.

## The statistics bridge

A default run reports its returns statistics in `BacktestResult.stats_returns`, but that set is the
engine's own default and does not include every built-in statistic. `Max Drawdown (simple)` is the important
example: the kernel portfolio's analyzer does not register it, so it is absent from a run's
dictionary.

The bridge closes that gap without computing any metric itself. It feeds the completed run's own
`BacktestResult.returns_series` into a fresh `PortfolioAnalyzer` with the requested native statistic
registered, and reads the value back. The statistic implementations stay the project's own: the
bridge resolves a metric name to a statistic by instantiating the classes exported from
`nautilus_trader.analysis` and reading each instance's own `name`, so there is no hand-written name
table that could drift from the Rust statistics.

Values the run already reports are taken from the run; only metrics the run does not report are
bridged. The bridge is `statistic_values(result, metrics)`.

As evidence, the bridged Sharpe ratio equals the engine's own reported value exactly. For the
sample run the engine reports `Sharpe Ratio (simple, sample, 252 days)` as `-27.2553412003192`, and
`bridged_values` recomputes the same float from the run's returns series. The bridged
`Max Drawdown (simple)`, which the run does not report, is `-0.014306129144533997` for that run.

## Search strategies and the run description

`GridSearch` enumerates the whole space. Two further strategies implement the same `SearchStrategy`
protocol and are deterministic under a seed:

- `RandomSearch(seed, budget)`: a seeded sample drawn without replacement from the space's
  positions, so the same seed and space select the same experiments in the same order.
- `EvolutionarySearch(evaluations, seed, operators, budget)`: breeds generations from a seeded
  generator and the evaluations recorded so far. Parents are chosen by tournament over feasibility
  first and score second, elite individuals survive unchanged, and children are crossed over and
  mutated at the declared rates (`EvolutionaryOperators`). A search never computes a score.

A run is described by a `RunDescription`: the space digest, the validation scheme, the seed, the
search strategy, the evolutionary operators and the evaluation cache digest. Its digest identifies
the run, so two runs are comparable only when their descriptions are.

Each trial's own specification is recorded with its result rather than left to the caller's memory.
A `TrialSpecification` carries the data window, the universe rule, the weighting, the adjustment
model and the exclusions; the optimizer captures it per experiment and stores it on
`ExperimentResult`, and `SearchReport.specification_spread()` reports how many distinct
specifications the survivors recorded - so a search over three weightings and two windows records
six. A report whose results carry no specification reports `None`, an unknown spread rather than a
zero.

The validation scheme is part of the run, not a caller's convention:
`ValidationScheme.single_split(search, held_out)` declares the searched window and the held-out
window, and `ValidationScheme.walk_forward(windows)` declares a sequence of in-sample and
out-of-sample windows. Overlapping windows are refused, and when a scheme is given the optimizer
refuses a run whose window does not lie inside one of the scheme's search windows, so a held-out
window can never be scored by the search. The report carries the scheme so a reader can see which
windows were which.

Constraints are evaluated before the objective. An infeasible candidate is recorded with
`feasible=False` and no fictitious penalty in its score, and an evolutionary search reads
feasibility as a separate attribute rather than as a penalty. Every evaluation is keyed by the
canonical parameter digest in an `EvaluationCache`; the optimizer consults the cache before
executing an experiment, so a resumed run over the same `ExperimentStore` executes none of the
vectors it already holds. The `SearchReport` records the number of distinct evaluations, the
searched space size, the number of executions and the seed, and `evaluated_fraction` reports the
evaluated fraction of the space. A budget smaller than the space reports the best of what it
evaluated.

## Persistence

An `ExperimentStore` owns a caller-given directory and writes strict, digest-keyed JSON, so a
sweep can be reloaded without rerunning it:

- `experiments/<digest>.json`: an experiment's digest and parameter set.
- `results/<canonical-digest>.json`: a result's experiment, canonical digest, score, constraint
  outcome, and metric values.
- `canonical/<canonical-digest>.json`: the canonical backtest result document itself.
- `failures/<experiment-digest>.json`: a failure's experiment, error type, and error message.
- `report.json`: the sweep manifest, listing the ranked result digests, the failure digests, the
  number of evaluations, the searched space size, the number of executions, the seed and the
  validation scheme.

The digest's colon is replaced by an underscore in file names so the layout is valid on Windows.
Non-finite metric values are written as the strings `"nan"`, `"inf"`, and `"-inf"` because strict
JSON has no non-finite numbers; they decode back to floats.

## Concurrency

Backtest execution is single-threaded inside the kernel and Python holds the GIL, so runs fan out
to separate processes. Each worker loads the extension, a catalog slice, and an engine, so the
constraint is memory, not CPU. The default worker count is the available memory budget divided by a
per-run footprint estimate, not a processor count.

The per-run estimate is measured, not guessed. A fresh worker holds roughly 71 MiB before a run,
peaks near 160 MiB of working set, and commits near 472 MiB of pagefile for the sample run in the
measurement. `DEFAULT_PER_RUN_BYTES` is 512 MiB, rounded up from the committed footprint with
headroom for larger catalogs and engines. The default `ConcurrencyPolicy` uses 75 percent of
available physical memory, so the worker count is
`available_memory * 0.75 // 512 MiB`, clamped to the platform process limit (61 on Windows). On a
reference machine with about 57.9 GB available this derives 61 workers.

Memory is read from `psutil` when installed, otherwise from the platform API. A policy with one
worker runs the sweep in-process, sequentially; the fan-out and the sequential path run the same
runner and produce the same results, in the same order.

## Public API

- `Parameter`, `ParameterSpace`, `Experiment`: the parameter model and its digests.
- `SearchStrategy`, `GridSearch`, `RandomSearch`, `EvolutionarySearch`, `EvolutionaryOperators`:
  enumeration of experiments, the two seeded strategies and the operators they breed with.
- `BacktestRunner`, `CanonicalRun`, `FailedExperiment`: one run through `BacktestNode`.
- `Optimizer`, `ExperimentResult`, `SearchReport`: the sweep and its ranked report.
- `ValidationScheme`, `ValidationMode`: the validation scheme a run is searched under, and its
  searched and held-out windows.
- `RunDescription`: the run's description (space, scheme, seed, search strategy, operators, cache)
  and its digest.
- `Evaluation`, `EvaluationCache`: the evaluations memoized by canonical parameter digest, so a
  resumed run executes none of the vectors it already holds.
- `ConcurrencyPolicy`: the memory-driven concurrency limit.
- `ExperimentStore`: digest-keyed persistence.
- `statistic_values`: the statistics bridge.
- `TrainStage`, `OptimizeStage`, `ValidateStage`, `OutOfSampleStage`, `WalkForwardStage`,
  `WalkForwardWindow`, `walk_forward_windows`, `WalkForwardReport`: the methodology stages.
- `SplitContract`, `Split`, `SplitDirection`, `LeakagePolicy`, `LabelOverlapRule`: the split
  contract, the bounds it yields, and the leakage exclusion relation it applies.
- `LabelDefinition`, `LabelKind`, `ForwardAggregate`, `AlignmentConvention`, `MissingDataPolicy`,
  `LabelSeries`, `label_series`: the label policies on the target path, their alignment convention
  and the forward reach a leakage policy has to cover.
- `ResearchCapabilityCode`, `leakage_capability`, `significance_capability`, `split_capability`,
  `screen_family_capability`, `persistence_capability`, `gapped_range_capability`,
  `history_capability`, `pair_distinguishability_capability`: the research domain's closed refusal
  codes and the probes that answer whether a request can be served before the work is done.
- `ScreenFamily`, `ScreenOutcome`: the trial family a screen enumerates before its gates run, and the
  tests it evaluated with the survivors among them.
- `PersistenceEstimate`, `PersistenceConvention`: a fitted reversion speed with the convention its
  half-life is reported under.
- `SignalWindow`: the fit window, measurement window and label of a relative-value signal, with the
  reach it measures over the bars.
- `RecursiveMemory`: the calibration, memory and warm-up a recursive estimator declares.
- `BarAmbiguityPolicy`, `IntrabarPath`, `TriggerPrecedence`, `TriggerFill`, `GapHandling`: the
  declared bar-derived execution assumptions and their identity.
- `StudyIdentity`, `TrialIdentity`, `TrialProvenance`, `DatasetIdentity`, `UniverseIdentity`,
  `ComputationIdentity`, `ResearchResult`, `objective_definition_from_terms`, `trial_identity`,
  `split_contract_digest`: the provenance contracts and the bridges that build them from a run.
- `OptimizationConfig`, `load_config`, `run_config`: the JSON configuration-file entry point the
  `nautilus optimize` command and notebooks share.

## Where it lives

The subsystem lives in `python/nautilus_trader/optimization/`: the parameter model in `space.py`,
enumeration and the search strategies in `search.py`, the validation scheme and the run description
in `run.py`, execution in `runner.py`, the statistics bridge in `metrics.py`, result
aggregation in `report.py`, the sweep in `optimizer.py`, the stages in `stages.py`, the split
contract and the leakage policy in `splits.py`, the label policies and their leakage reach in
`labels.py`, the relative-value declarations and their refusals in `relative_value.py`, the execution
assumptions in `assumptions.py`, the identity contracts in `identity.py`, persistence in
`persistence.py`, process fan-out in `concurrency.py`, and the configuration-file entry point in
`config.py`. The objective and constraints are the existing Rust types exposed from
`nautilus_trader.analysis`; this subsystem adds no second objective and no second execution path.

## The `optimize` command

`nautilus optimize <CONFIG>` runs a declared optimization from a configuration file. It is a thin
front end: it locates a Python interpreter, invokes the `nautilus_trader.optimization.config`
entry point over the file, and lets the child's machine-readable JSON document go straight to
standard output. It contains no optimization logic: no search, no objective, and no aggregation.

```bash
nautilus optimize config.json
```

The command writes one JSON document to standard output and reports failure through both the
document and the process exit status, matching the catalog data subcommands. A successful sweep
exits `0`; a configuration that is missing a file, contains an unknown key, or fails to load
prints a document with `"status": "error"` and exits non-zero.

| Argument or flag  | Meaning                                                                            |
| ----------------- | ---------------------------------------------------------------------------------- |
| `<CONFIG>`        | Path to the JSON configuration file (required).                                    |
| `--python <PATH>` | Python interpreter to invoke, overriding `NAUTILUS_PYTHON` and the fallback order. |

## Configuration file

The configuration file is JSON, matching the convention of the configuration file loader in
`crates/system/src/config_file.rs`: a typed constructor remains the canonical API and a file is a
view of a typed configuration, so unknown keys are rejected rather than ignored. JSON is used
rather than YAML even though the Python project depends on `pyyaml`, because the loader convention
it follows is JSON and the format needs no third-party parser.

The document records the whole sweep: the strategy under test and its configuration factory, the
parameter space, the run window, the objective and constraints, the methodology stage, the
concurrency policy, and an optional persistence directory.

```json
{
  "schema": "nautilus.optimization.config/v1",
  "strategy": {
    "strategy_path": "my_package.strategies:EMACross",
    "config_path": "my_package.strategies:EMACrossConfig",
    "config_factory": "my_package.runs:config_factory"
  },
  "space": {
    "base": {
      "instrument_id": "BTCUSDT.BINANCE",
      "bar_type": "BTCUSDT.BINANCE-1-MINUTE-LAST-EXTERNAL",
      "trade_size": "0.010000"
    },
    "parameters": [
      {"name": "fast_ema_period", "choices": [5, 10]},
      {"name": "slow_ema_period", "choices": [20, 30]}
    ]
  },
  "window": {"start": null, "end": null},
  "objective": {
    "terms": [
      {"metric": "Sharpe Ratio (simple, sample, 252 days)", "weight": 1.0, "direction": "maximize"},
      {"metric": "Max Drawdown (simple)", "weight": 1.0, "direction": "maximize"}
    ]
  },
  "constraints": [
    {"metric": "Max Drawdown (simple)", "comparison": "at_least", "bound": -0.013}
  ],
  "stage": {"kind": "optimize"},
  "concurrency": {"max_workers": 1},
  "store": {"directory": "runs/optimization"},
  "assumptions": {"bar_execution": true, "adaptive_high_low_ordering": false}
}
```

- `strategy.config_factory` is an importable `module:function` reference to the configuration
  factory the runner calls for each run window; it returns the venue configurations, data
  configurations, and engine configuration. The configuration file describes no venue or data
  configuration itself, so it cannot become a second configuration path.
- `space.base` holds the fixed strategy values and `space.parameters` the named parameters with
  their ordered choices; the last parameter varies fastest.
- `window.start` and `window.end` are Unix nanoseconds, or `null` for the data's own bounds.
- `objective.terms[].direction` is `maximize` or `minimize`, and
  `constraints[].comparison` is `at_least` or `at_most`.
- `store` is optional; when present the sweep is persisted under its directory.
- `assumptions` declares the bar-derived execution assumptions, with the venue configuration's own
  spelling: `bar_execution` (default `true`) and `adaptive_high_low_ordering` (default `false`).
  When the key is absent the declared default applies, which is the fixed Open, High, Low, Close
  sequence. The emitted result document carries the resolved policy under `assumptions`, so a
  result states the rule set it was produced under. A declaration that cannot apply, such as
  adaptive ordering without bar execution, is rejected as a configuration error.
- `stage.leakage`, on `walk_forward` only, takes `purge_before_ns`, `purge_after_ns`,
  `embargo_after_ns`, `label_overlap_rule` (`none` or `enforce`), `label_horizon_ns`, and
  `zero_interval_justification`. A policy that leaves any interval at zero must justify it. When
  the key is absent the stage uses its declared default, which excludes nothing and says so.

| `stage.kind`    | Extra keys                                    | Runs                                                         |
| --------------- | --------------------------------------------- | ------------------------------------------------------------ |
| `optimize`      | none                                          | The ranked report of a search.                               |
| `train`         | none                                          | The best experiment selected on the window.                  |
| `validate`      | `parameters`                                  | One experiment evaluated against the constraints.            |
| `out_of_sample` | `parameters`                                  | One experiment evaluated on a window it was not selected on. |
| `walk_forward`  | `in_sample_ns`, `out_of_sample_ns`, `leakage` | A search and out-of-sample evaluation per window.            |

## Notebook helper

`examples/backtest/notebooks/optimization_sweep.py` drives the same workflow a notebook user
wants: it writes a configuration document, loads it with `load_config`, runs it with `run_config`,
and prints the best experiment's parameters, score, and canonical digest. The notebook and the CLI
call the same entry point, so there is one optimization implementation and one semantic model.

## Interpreter contract

The `optimize` command resolves the Python interpreter in this order, taking the first candidate
that resolves to an existing file:

1. `--python`, when given.
2. `NAUTILUS_PYTHON`, when set and non-empty.
3. `VIRTUAL_ENV\Scripts\python.exe` on Windows or `VIRTUAL_ENV/bin/python` elsewhere, when
   `VIRTUAL_ENV` is set.
4. `python3` on `PATH`, then `python` on `PATH`.

A candidate that contains a path separator is treated as a path; otherwise it is looked up on
`PATH` (with a `.exe` suffix tried on Windows). When no candidate resolves, the command fails and
names every candidate it tried. The interpreter must have `nautilus_trader` importable, because
the invoked module is the package's own optimization entry point.
