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
- `OptimizationConfig`, `load_config`, `run_config`: the JSON configuration-file entry point the
  `nautilus optimize` command and notebooks share.

## Where it lives

The subsystem lives in `python/nautilus_trader/optimization/`: the parameter model in `space.py`,
enumeration in `search.py`, execution in `runner.py`, the statistics bridge in `metrics.py`, result
aggregation in `report.py`, the sweep in `optimizer.py`, the stages in `stages.py`, persistence in
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
      {"metric": "Sharpe Ratio (252 days)", "weight": 1.0, "direction": "maximize"},
      {"metric": "Max Drawdown", "weight": 1.0, "direction": "maximize"}
    ]
  },
  "constraints": [
    {"metric": "Max Drawdown", "comparison": "at_least", "bound": -0.013}
  ],
  "stage": {"kind": "optimize"},
  "concurrency": {"max_workers": 1},
  "store": {"directory": "runs/optimization"}
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

| `stage.kind`    | Extra keys                         | Runs                                                         |
| --------------- | ---------------------------------- | ------------------------------------------------------------ |
| `optimize`      | none                               | The ranked report of a search.                               |
| `train`         | none                               | The best experiment selected on the window.                  |
| `validate`      | `parameters`                       | One experiment evaluated against the constraints.            |
| `out_of_sample` | `parameters`                       | One experiment evaluated on a window it was not selected on. |
| `walk_forward`  | `in_sample_ns`, `out_of_sample_ns` | A search and out-of-sample evaluation per window.            |

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
