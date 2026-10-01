# 02 - The engine view

This lecture maps the words from lecture 01 onto the objects the repository actually provides. There
is no code yet. Every name below is a Python type exported from `nautilus_trader.optimization` or
`nautilus_trader.analysis`, and every path is a file you can open. The overview is
`docs/concepts/optimization.md`.

## The pipeline

The subsystem is five stages (see the "Pipeline" section of `docs/concepts/optimization.md`):

1. a **parameter space** holding fixed base values and named parameters with ordered choices;
2. an **experiment**, one complete parameter set with a canonical digest;
3. a **search strategy** that enumerates the experiments to evaluate;
4. a **runner** that builds one backtest per experiment through `BacktestNode`;
5. an **evaluation** and a **result**, scored by an objective and constraints and ranked.

The pipelined objects live in `python/nautilus_trader/optimization/`:

| Object                                                                                 | Module                      | What it is                                            |
| -------------------------------------------------------------------------------------- | --------------------------- | ----------------------------------------------------- |
| `Parameter`, `ParameterSpace`, `Experiment`                                            | `space.py`                  | The parameter model and its digests.                  |
| `GridSearch`, `RandomSearch`, `EvolutionarySearch`, `EvolutionaryOperators`            | `search.py`                 | Enumeration and the seeded strategies.                |
| `BacktestRunner`, `CanonicalRun`, `FailedExperiment`                                   | `runner.py`                 | One run through `BacktestNode`.                       |
| `Optimizer`, `SearchReport`, `ExperimentResult`                                        | `optimizer.py`, `report.py` | The sweep and its ranked report.                      |
| `ValidationScheme`, `RunDescription`                                                   | `run.py`                    | The validation scheme and the run identity.           |
| `Evaluation`, `EvaluationCache`, `ExperimentStore`                                     | `persistence.py`            | The memo and its digest-keyed store.                  |
| `ConcurrencyPolicy`                                                                    | `concurrency.py`            | The memory-driven process fan-out.                    |
| `statistic_values`                                                                     | `metrics.py`                | The bridge that reads metrics the run did not report. |
| `TrainStage`, `OptimizeStage`, `ValidateStage`, `OutOfSampleStage`, `WalkForwardStage` | `stages.py`                 | The methodology stages.                               |
| `SplitContract`, `LeakagePolicy`, `LabelOverlapRule`                                   | `splits.py`                 | Window layout and the leakage relation.               |
| `LabelDefinition`, `LabelSeries`                                                       | `labels.py`                 | Forward-looking targets and their reach.              |
| `StatisticalContract`, `SharpeSample`, `deflated_sharpe_ratio`                         | `significance.py`           | The correction.                                       |
| `StudyIdentity`, `TrialIdentity`, `TrialProvenance`, `ResearchResult`                  | `identity.py`               | Provenance contracts.                                 |
| `BarAmbiguityPolicy`                                                                   | `assumptions.py`            | The declared bar-derived execution assumptions.       |
| `OptimizationConfig`, `load_config`, `run_config`                                      | `config.py`                 | The JSON configuration-file entry point.              |

## The parameter space and the experiment

A `Parameter` is a name and an ordered tuple of choices. A `ParameterSpace` is a mapping of fixed
`base` values plus the parameters. Expansion is deterministic: the Cartesian product in declaration
order, with the last parameter varying fastest (`python/nautilus_trader/optimization/space.py`). The
space always yields the same sweep in the same order, and its `digest` is a `sha256:` fingerprint of
the base and the choices.

An `Experiment` is one complete parameter set: the base merged with one choice per parameter. Its
canonical serialization is sorted-key, compact, strict JSON, and its digest depends only on the set,
never on a clock. Equal sets produce equal digests, which is what makes a cache possible.

## The search strategies

`GridSearch` enumerates the whole space. `RandomSearch(seed, budget)` draws a seeded sample without
replacement from the space's positions, so the same seed and space select the same experiments in
the same order. `EvolutionarySearch(evaluations, seed, operators, budget)` breeds generations from a
seeded generator and the evaluations recorded so far; parents are chosen by tournament over
feasibility first and score second. A search never computes a score
(`python/nautilus_trader/optimization/search.py`).

## The runner and the boundary

`BacktestRunner` holds an importable `config_factory` that returns the venue, data and engine
configurations for a run window, the strategy and config import paths, and the window bounds. For
each experiment it builds one `BacktestRunConfig`, whose ID is the experiment digest, runs it
through `BacktestNode`, and reads the canonical result back
(`python/nautilus_trader/optimization/runner.py`).

The boundary is explicit in `docs/concepts/optimization.md`: the optimizer composes runs and never
reaches into one. It does not construct an engine, does not own the data or venue configuration, and
does not alter the deterministic semantics of an individual backtest. The caller supplies the
configurations; the runner adds only the run identity and the strategy.

## Scoring

The objective and constraints are Rust types exposed from `nautilus_trader.analysis`; the subsystem
adds no second objective. An `Objective` is a list of `ObjectiveTerm`s, each a metric name, a weight
and a direction (`MAXIMIZE` or `MINIMIZE`). A `Constraint` is a metric, a comparison (`AT_LEAST` or
`AT_MOST`) and a bound. Constraints are evaluated before the objective, so an infeasible candidate is
recorded with `feasible=False` and no fictitious penalty in its score
(`python/nautilus_trader/optimization/optimizer.py`).

A run reports its returns statistics in `BacktestResult.stats_returns`, but that set does not include
every built-in statistic. The bridge `statistic_values` feeds the completed run's own
`returns_series` into a fresh `PortfolioAnalyzer` with the requested native statistic registered, and
reads the value back (`python/nautilus_trader/optimization/metrics.py`). "Max Drawdown" is the
important example: a run does not report it, and the bridge recomputes it. A metric that cannot be
computed is an error for the objective, never a zero.

## The validation scheme and the split contract

`ValidationScheme.single_split(search, held_out)` declares the searched window and the held-out
window. `ValidationScheme.walk_forward(windows)` declares a sequence of in-sample and out-of-sample
pairs. Overlapping windows are refused, and when a scheme is given the optimizer refuses a run whose
window does not lie inside one of the scheme's search windows
(`python/nautilus_trader/optimization/run.py`, `optimizer.py`).

Under the scheme, a `SplitContract` owns the layout: it names its sets, takes each set's length in
nanoseconds or as a fraction, and lays the windows out whole so every set is positively long. A
`LeakagePolicy` states the exclusion relation between the evaluation set and the training set, with
fields `purge_before`, `purge_after`, `embargo_after`, and `label_overlap_rule`. A zero interval is
permitted but not silently: it requires a `zero_interval_justification`
(`python/nautilus_trader/optimization/splits.py`).

## Labels

A label is a future outcome. `LabelDefinition` declares a `kind` (`FORWARD_RETURN`,
`FORWARD_AGGREGATE`, or `FIRST_HIT_THRESHOLD`), a `horizon`, a `wait`, an `alignment`, a
`missing_data_policy`, and the kind's own fields. `label_series` produces a `LabelSeries` whose
`forward_reach_ns` is measured from the produced series, and `validate_leakage` refuses a policy whose
purge is shorter than that reach (`python/nautilus_trader/optimization/labels.py`).

Labels are a separate type with a separate identity. No module under `trading`, `live`, `backtest`,
`execution`, `risk` or `adapters` imports the label layer, so a label value cannot quietly become a
feature (`docs/concepts/optimization.md`, "Labels and the target path").

## Provenance

`StudyIdentity` carries the dataset identity, the parameter-space digest, the objective definition,
the selection rule, the metric set, the split contract, the leakage policy and the study seed.
`TrialIdentity` carries the study, the parameter values and their digest, the seed, the execution
status, the objective value and the canonical result digest. `TrialProvenance` carries the trial and
failed-trial counts and whether the study distinguishes a nominal from an effective count
(`python/nautilus_trader/optimization/identity.py`).

Two boundaries are structural in that file: a result cannot be built without a study identity, and a
trial cannot record a parameter digest that does not describe its own values. A changed numeric
kernel version changes the computation identity while the study identity stays stable, so numerical
drift is visible.

## The correction

The best of a sweep is a selection, not an estimate. `significance.py` carries the statistical
contract and the deflated Sharpe ratio. The contract declares the return definition, the risk-free
treatment, the minimum observation and trial counts, the Sharpe convention and its divisor, the
estimators, the non-excess kurtosis convention, the prohibition on annualised inputs, the
missing-return rule, and the requirement that a study declare its trial dependence. It carries an
identity and a digest and is recorded with every result
(`python/nautilus_trader/optimization/significance.py`).

Three boundaries are enforced at the sample, not documented: an annualised input is refused, an
excess kurtosis is refused, and a dependent study must supply its effective trial count. The value is
reported, never a gate: nothing in the subsystem consults it before a strategy, an order or a risk
check.

## Persistence and concurrency

An `ExperimentStore` owns a caller-given directory and writes strict, digest-keyed JSON
(`python/nautilus_trader/optimization/persistence.py`):

```text
experiments/<digest>.json          the experiment and its parameter set
results/<canonical-digest>.json    the score, constraint outcome and metric values
canonical/<canonical-digest>.json  the canonical backtest result document
failures/<experiment-digest>.json  the typed failure
report.json                        the sweep manifest with its counts and scheme
```

The colon in a digest is replaced by an underscore in file names so the layout is valid on Windows.
Non-finite metric values are written as the strings `"nan"`, `"inf"` and `"-inf"`, because strict
JSON has no non-finite numbers.

Backtest execution is single-threaded inside the kernel and Python holds the GIL, so runs fan out to
separate processes. Each worker loads the extension, a catalog slice and an engine, so the constraint
is memory, not CPU. The default worker count is the available memory budget divided by a per-run
footprint estimate, not a processor count (`python/nautilus_trader/optimization/concurrency.py`).

## The configuration file

The same workflow is described as data in a JSON file, matching the loader convention in
`crates/system/src/config_file.rs`: unknown keys are rejected rather than ignored. The document
records the strategy and its configuration factory, the parameter space, the run window, the
objective and constraints, the methodology stage, the concurrency policy, an optional persistence
directory, and an `assumptions` block (`python/nautilus_trader/optimization/config.py`). The
`nautilus optimize` command is a thin front end over the package's own entry point, and a notebook
calls the same `load_config` and `run_config` functions, so there is one optimization
implementation and one semantic model.

## The bar-derived assumptions

A bar records four prices and no path between them, so a bar-driven replay must assume an ordering, a
fill rule for a triggered stop, a precedence between simultaneous triggers, and a treatment for a bar
that opens beyond a trigger. `BarAmbiguityPolicy` records which rule set a study ran under, with an
identity, and refuses two configurations that cannot both apply
(`python/nautilus_trader/optimization/assumptions.py`). When a study declares nothing, the declared
default is bar execution with the fixed Open, High, Low, Close sequence, which is what the engine
already implements (`docs/concepts/optimization.md`, "Execution assumptions").

## Where the rest lives

The objective, the statistics and the Sharpe implementation are Rust: `crates/analysis/src/python/`
and `crates/analysis/src/statistics/sharpe_ratio.rs`. The backtest engine and its Python bindings are
`crates/backtest/src/python/engine.rs`. The fill, slippage and latency models are
`crates/execution/src/models/fill.rs`, `slippage.rs` and `latency.rs`, documented in
`docs/concepts/backtesting/fill-models.md`. The risk-engine configuration is
`crates/risk/src/python/config.rs`. The factor pipeline, membership and dataset contract are
Rust-only, in `crates/research` with no Python bindings; lecture 09 says what that means for you.
