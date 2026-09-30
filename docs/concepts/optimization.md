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

## The statistics bridge

A default run reports its returns statistics in `BacktestResult.stats_returns`, but that set is the
engine's own default and does not include every built-in statistic. "Max Drawdown" is the important
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
sample run the engine reports `Sharpe Ratio (252 days)` as `-27.2553412003192`, and
`bridged_values` recomputes the same float from the run's returns series. The bridged
`Max Drawdown`, which the run does not report, is `-0.014306129144533997` for that run.

## Persistence

An `ExperimentStore` owns a caller-given directory and writes strict, digest-keyed JSON, so a
sweep can be reloaded without rerunning it:

- `experiments/<digest>.json`: an experiment's digest and parameter set.
- `results/<canonical-digest>.json`: a result's experiment, canonical digest, score, constraint
  outcome, and metric values.
- `canonical/<canonical-digest>.json`: the canonical backtest result document itself.
- `failures/<experiment-digest>.json`: a failure's experiment, error type, and error message.
- `report.json`: the sweep manifest, listing the ranked result digests and the failure digests.

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
- `SearchStrategy`, `GridSearch`: enumeration of experiments.
- `BacktestRunner`, `CanonicalRun`, `FailedExperiment`: one run through `BacktestNode`.
- `Optimizer`, `ExperimentResult`, `SearchReport`: the sweep and its ranked report.
- `ConcurrencyPolicy`: the memory-driven concurrency limit.
- `ExperimentStore`: digest-keyed persistence.
- `statistic_values`: the statistics bridge.
- `TrainStage`, `OptimizeStage`, `ValidateStage`, `OutOfSampleStage`, `WalkForwardStage`,
  `WalkForwardWindow`, `walk_forward_windows`, `WalkForwardReport`: the methodology stages.

## Where it lives

The subsystem lives in `python/nautilus_trader/optimization/`: the parameter model in `space.py`,
enumeration in `search.py`, execution in `runner.py`, the statistics bridge in `metrics.py`, result
aggregation in `report.py`, the sweep in `optimizer.py`, the stages in `stages.py`, persistence in
`persistence.py`, and process fan-out in `concurrency.py`. The objective and constraints are the
existing Rust types exposed from `nautilus_trader.analysis`; this subsystem adds no second
objective and no second execution path.
