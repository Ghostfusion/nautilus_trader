# Implementing the Lean-derived improvements

This document is the implementation plan for the items selected in
[lean_lessons_design.md](lean_lessons_design.md). Each workstream is independently shippable,
independently revertible, and opt-in. Nothing here changes the kernel's execution model.

## 1. Ground rules

1. The runtime model is fixed: single-threaded kernel, thread-local message bus, per-event
   callbacks, `Rc<RefCell<_>>` components. No workstream may introduce a slice-style delivery
   model into the hot path.
2. New behaviour is opt-in. Default backtest and live behaviour after every workstream is
   byte-identical to before, which the existing golden tests must continue to prove.
3. New scheduling is driven by the `Clock`, never by wall time, so backtests stay deterministic.
4. Reuse existing seams: message-bus endpoints in `crates/common/src/msgbus/switchboard.rs`,
   the `DataClient` and `ExecutionClient` traits, client and config factories,
   `CatalogReader` and `CatalogWriter`, and the `KernelEventStore` trait. Do not add a parallel
   data or configuration path.
5. Every workstream ships code, tests, documentation, and regenerated stubs in the same change.
   In this repository that means `make py-stubs` and `make check-generated-drift` are part of the
   change, not a follow-up.

## 2. Repository mechanics

Where things go:

| Change                            | Location                                                                                    |
| --------------------------------- | ------------------------------------------------------------------------------------------- |
| Domain type or value              | `crates/model/src/<area>/`                                                                  |
| Shared service, cache, bus, clock | `crates/common/src/<area>/`                                                                 |
| Engine behaviour                  | `crates/<engine>/src/`                                                                      |
| Analysis or statistic             | `crates/analysis/src/statistics/`                                                           |
| Test fixtures and helpers         | `crates/testkit/src/`                                                                       |
| CLI surface                       | `crates/cli/src/` with subcommands declared in `crates/cli/src/opt.rs`                      |
| Python binding                    | `crates/<crate>/src/python/<module>.rs`, registered in `crates/<crate>/src/python/mod.rs`   |
| Pure Python API                   | `python/nautilus_trader/<subpackage>/` plus `EXTRA_REEXPORTS` in `python/generate_stubs.py` |
| Concept documentation             | `docs/concepts/`                                                                            |
| Contributor documentation         | `docs/developer_guide/`                                                                     |

Rules that are easy to miss:

- Adding a Python-visible class means: implement the binding file, register it in the crate's
  `#[pymodule]`, run `make py-stubs`, and commit the regenerated `.pyi` and generated docstrings.
- Adding a new Python submodule also means updating `crates/pyo3/src/lib.rs` and the
  `EXPECTED_PYO3_MODULES` allowlist in `.pre-commit-hooks/check_nautilus_conventions.sh`.
- Shipped data files use a `resources/` directory (the repository's text checks already exclude
  `*/resources/*.json`), with `include_str!` for embedded defaults.
- Generated artifacts are never edited by hand.

Verification for any workstream:

```bash
make build-debug
make cargo-test
make pytest
make py-stubs && make check-generated-drift
make format
make pre-commit
```

For changes in the backtest hot path, also run the relevant benchmarks
(`make cargo-ci-benches`) and compare.

## 3. Workstream W1: a declared, regenerable regression harness

**Objective (design L5).** Turn the existing deterministic comparison machinery into a declared,
auto-discovered, one-command-regenerable regression suite.

**What already exists.** `CanonicalBacktestResult` in `crates/backtest/src/result.rs` produces a
versioned canonical document (`nautilus-backtest-result/v1`) with identity normalization,
`digest()`, `to_bytes()`/`from_slice()`, and `first_divergence()`. `python/tests/acceptance/test_backtest.py`
asserts golden values by hand. `BacktestResult` Python bindings live in
`crates/backtest/src/python/result.rs` and expose statistics but not canonical bytes or a digest.

**Steps.**

1. Expose the canonical artifact to Python: add `canonical_bytes()` and `digest()` methods to the
   `BacktestResult` bindings in `crates/backtest/src/python/result.rs`, delegating to the existing
   `CanonicalBacktestResult` writer. Classify the exact-encoding failure path as `# Errors`.
2. Add a scenario protocol in Python under `python/tests/regression/`: a scenario declares its
   run configuration, strategy, expected statistics keys, and expected digest.
3. Add a registry module that enumerates scenarios, so discovery is a list, not reflection over
   the whole test tree.
4. Add a reset fixture that returns engines, caches, loggers, and clock state to a clean baseline
   between scenarios, matching Lean's hard-reset practice
   ([AlgorithmRunner.cs](https://github.com/QuantConnect/Lean/blob/master/Tests/AlgorithmRunner.cs)).
5. Add regeneration: a pytest option or environment variable that rewrites the committed
   expectations from an actual run, producing a reviewable diff.
6. On mismatch, print `first_divergence().path` so failures name the differing field.
7. Document the workflow in `docs/developer_guide/testing.md` and add the suite to the Makefile
   as `pytest-regression`, wired into `pre-flight`.

**Acceptance.** The suite passes on a clean checkout; perturbing a fill model produces a failure
naming a divergence path; regeneration rewrites the expectation and a second run passes without
regenerating; default `make pytest` behaviour is unchanged.

**Risks.** Digest churn on intentional simulation changes. Mitigation: regeneration is explicit
and reviewed, and `first_divergence()` carries the diagnostic burden.

## 4. Workstream W2: trading calendars as data

**Objective (design L4).** A venue calendar model loaded from data, used by scheduling and any
session-aware logic.

**Steps.**

1. Add `crates/model/src/calendars/` with `TradingSession` (open and close times in exchange
   local time), `TradingCalendar` (weekly sessions, holidays, early closes, time zone), and
   `CalendarKey` (venue plus instrument class plus optional symbol).
2. Define the data file as JSON under `crates/model/resources/calendars/` and load it with
   `include_str!` for a bundled default, with an override path for user data.
3. Integrate with existing instrument fields: `activation_ns` and `expiration_ns` stay the source
   of instrument lifetime; the calendar answers whether a given instant is tradeable.
4. Reimplement the FX helper in `crates/trading/src/sessions.rs` as a thin wrapper over the
   calendar, keeping the existing function signatures so current behaviour is unchanged.
5. Expose the calendar through the Python `nautilus_trader.model` facade and add a `make py-stubs`
   regeneration.

**Acceptance.** A unit test proves a known holiday and a half-day close resolve correctly; the FX
session functions return identical results to the current implementation for a spread of dates;
loading a user-supplied calendar overrides the bundled one.

**Risks.** Stale holiday data. Mitigation: the file is versioned, overridable, and validated at
load with a startup warning when a calendar's coverage ends before the run end.

## 5. Workstream W3: dynamic universe selection

**Objective (design L2).** A selection model that adds and removes instruments at runtime through
the existing subscription machinery.

**Steps.**

1. Add a `Universe` component under `crates/trading/src/universe/`, modelled on the existing
   actor lifecycle, with a selection function, membership set, and re-selection schedule.
2. Subscriptions go through the existing data command path: `DataEngine` already handles
   subscribe and unsubscribe commands and tracks subscription ownership in
   `crates/data/src/subscription.rs`. Universe membership must release its own subscriptions when
   an instrument leaves, without disturbing subscriptions owned by other components.
3. Instrument metadata is requested through the existing `request_instruments` flow, not a new
   provider interface.
4. Publish membership changes on the bus with a new topic in
   `crates/common/src/msgbus/switchboard.rs`, and surface an `on_universe_changed` callback on
   actors alongside the existing `on_instrument` callbacks.
5. Scheduling uses clock timers so backtest selection is deterministic.
6. Expose `Universe` through `crates/trading/src/python/` and the `nautilus_trader.trading`
   facade.
7. Guard removals: refuse to remove an instrument with open orders or a non-flat position and
   report the condition rather than silently dropping the subscription.

**Acceptance.** A backtest where a universe selects instruments on a schedule runs deterministically
and reproduces the same result across reruns; a live sandbox node can add and remove instruments
without leaking subscriptions; a removal with an open position is refused with a clear log.

**Risks.** Subscription ownership bugs and live metadata gaps. Mitigation: explicit ownership
tests in `crates/data/src/subscription.rs`, and a capability check that lets an adapter report it
cannot supply metadata.

## 6. Workstream W4: parameter optimization

**Objective (design L6).** A first-class optimizer front end over backtest runs.

**Steps.**

1. Define the objective as a function over the existing portfolio statistics in
   `crates/analysis/src/statistics/`, with constraints expressed over the same values.
2. Implement the search strategies separately from execution: a strategy enumerates parameter
   sets; a runner executes them. Mirror Lean's split between
   [GridSearchOptimizationStrategy.cs](https://github.com/QuantConnect/Lean/blob/master/Optimizer/Strategies/GridSearchOptimizationStrategy.cs)
   and the optimizer host, without adopting its single-process queue semantics.
3. Fan out runs at the process level: each parameter set runs in its own process using the built
   extension or CLI, so the single-threaded kernel is unchanged and the Python GIL is not a
   constraint.
4. Emit results as canonical backtest results plus the parameter set, using the existing
   `CanonicalBacktestResult` digest so runs are comparable.
5. Expose the surface in `crates/cli/src/opt.rs` as an `optimize` subcommand, and a Python
   helper in the backtest package for notebook use.
6. Constrain concurrency by available memory, not by CPU alone, because each run loads its own
   data.

**Acceptance.** A sweep over a small parameter grid returns the same best result as running the
same grid by hand; an objective over Sharpe ratio and maximum drawdown behaves as specified; a
failing run does not abort the sweep.

**Risks.** Memory blowup with concurrent catalog reads. Mitigation: a documented concurrency limit
and a warmup-free path that reuses a loaded catalog where the run configuration allows it.

## 7. Workstream W5: research that reuses engine primitives

**Objective (design L7).** Notebook workflows over the same instrument and data types as backtests.

**Steps.**

1. Add a Python-only helper in `python/nautilus_trader/analysis/research.py` that opens a
   `ParquetDataCatalog`, loads instruments and data with the existing Python catalog bindings,
   and exposes them as typed Nautilus objects plus a DataFrame conversion via
   `python/nautilus_trader/persistence/catalog_to_df.py`.
2. Compute indicators with the existing indicator API rather than a second implementation.
3. Provide a small replay helper that yields data in `ts_init` order so notebook code can mirror
   strategy logic without constructing an engine.
4. Do not add a research mode to the kernel; this stays a Python convenience over existing
   components.

**Acceptance.** A notebook example under `examples/backtest/notebooks/` loads a catalog, computes
an indicator, and reproduces the values a backtest of the same data produces.

**Risks.** A divergent second data path. Mitigation: the helper must call the same catalog and
wrangler code paths used by `BacktestNode`.

## 8. Workstream W6: corporate actions, identity, and price normalization

**Objective (design L3).** Represent and apply splits, dividends, delistings, and symbol changes.

**Steps.**

1. Define auxiliary data types in `crates/model/src/data/` and register them with the existing
   data-type macro so catalog paths, Arrow schemas, and bus topics are generated consistently.
2. Persist and query them through `CatalogReader` and `CatalogWriter`; the catalog layout gains
   the new type directories automatically.
3. Add an opt-in adjustment stage. The default keeps raw data untouched. Where enabled, the stage
   converts adjusted data to raw by applying the action series, and emits the actions as events on
   the bus so strategies can react.
4. Model identity separately from price: a mapping from venue symbol to instrument id over time,
   resolved before data reaches the engines, with a rename emitted as an event.
5. Handle delisting as a terminal instrument status with an explicit position outcome, reusing the
   existing `InstrumentClose` path rather than a new settlement mechanism.
6. Add a documented canonical form for the auxiliary files and extend the catalog documentation in
   `docs/concepts/data/catalog.md`.

**Acceptance.** A synthetic equity series with a 4:1 split and a dividend reproduces the expected
raw and adjusted prices; a rename mid-series resolves to a single instrument identity; a delisting
closes positions through the existing instrument-close path; the default path leaves all existing
golden results unchanged.

**Risks.** Adjusted-versus-raw ambiguity is the classic source of silent error. Mitigation: raw data
is never mutated, adjustment is opt-in, and the run records which normalization was applied.

## 9. Workstream W7: file-based configuration

**Objective (design L8).** Optional file configuration that maps onto existing typed configs.

**Steps.**

1. Define a schema that mirrors `NautilusKernelConfig`, `BacktestEngineConfig`, and
   `LiveNodeConfig` fields, with named environments layered over a base section.
2. Implement loading and validation in Rust so errors are raised at construction with typed
   messages, not at runtime.
3. Expose it through `nautilus-cli` and the Python constructors as an alternative input, with the
   typed constructors remaining authoritative.
4. Document the precedence order explicitly and test it, including the failure mode for unknown
   keys, consistent with the `deny_unknown_fields` convention used by adapter configs.

**Acceptance.** A YAML file reproduces a config constructed by hand; an unknown key fails
validation; environment layering resolves in the documented order.

**Risks.** Two configuration surfaces drifting. Mitigation: the file schema is generated from or
validated against the typed configs in a test.

## 10. Workstream W8: signal and target layer

**Objective (design L1).** A typed signal and target layer between decision-making and order
submission. Do this last and behind a feature.

**Steps.**

1. Define an `Insight`-style value type in `crates/model/` with direction, period or expiry,
   magnitude, confidence, source, and score. Note that `crates/common/src/signal.rs` already
   defines a generic `Signal` with name, value, and timestamps; the insight type is distinct and
   must not overload it.
2. Add an optional portfolio-construction component that converts insights into target positions,
   and a reconciler that compares targets with `Portfolio` positions and emits the minimal order
   set. Reuse `crates/risk/src/sizing.rs` for sizing and the `RiskEngine` for pre-trade checks.
3. Keep direct order submission from strategies fully supported; the layer is opt-in per strategy.
4. Expose the types in Python and document the composition in `docs/concepts/`.

**Acceptance.** A strategy using the insight layer produces identical orders to an equivalent
strategy submitting orders directly for a simple case; the layer is disabled by default; the
backtest hot path shows no measurable regression with the layer unused.

**Risks.** A second source of position intent. Mitigation: targets are reconciled against the cache
before any order is emitted, and the feature stays behind a flag until the semantics are proven.

## 11. Sequencing

| Phase | Workstreams | Rationale                                                          |
| ----- | ----------- | ------------------------------------------------------------------ |
| 1     | W1          | Protects every later change; no core risk                          |
| 2     | W2, W4, W5  | Independent; W2 unblocks scheduling, W4 and W5 are research-facing |
| 3     | W3, W7      | W3 depends on W2; W7 is ergonomic and independent                  |
| 4     | W6, W9      | Data-semantics project; W9 depends on W6                           |
| 5     | W8          | Deepest change; only after the layer above is stable               |

## 12. Cross-cutting acceptance criteria

- Default behaviour is unchanged: existing golden backtest results and their digests are identical
  before and after each workstream.
- `make format`, `make pre-commit`, `make cargo-test`, and `make pytest` pass for the affected
  areas.
- Regenerated stubs and docstrings are committed, and `make check-generated-drift` passes.
- New public Rust items carry `# Errors` and `# Panics` sections where the conventions require
  them, and new Python bindings follow the `Py*` wrapper and `py_*` method naming.
- New hot-path code has a benchmark comparison recorded in the pull request.
- Documentation is updated in the same change, including the concept page for the affected area.

## 13. Risks

| Risk                                | Impact | Mitigation                                            |
| ----------------------------------- | ------ | ----------------------------------------------------- |
| Scope creep into the kernel runtime | High   | Ground rule 1; reject any slice-style delivery change |
| Determinism regression              | High   | Clock-driven scheduling; golden digests in CI         |
| Subscription ownership bugs         | High   | Explicit ownership tests before W3 lands              |
| Two configuration surfaces          | Medium | Schema validated against typed configs in a test      |
| Golden churn                        | Medium | Explicit regeneration; divergence paths in failures   |
| Adjustment correctness              | High   | Raw data immutable; normalization recorded per run    |
| Memory blowup in optimization       | Medium | Documented concurrency limit; process fan-out only    |

## 14. Out of scope

- A cloud platform, job queue, or multi-tenant scheduler.
- Dynamic handler resolution by class-name string from configuration.
- A slice or time-batch delivery model in the runtime.
- AI or ML tooling.
- Changes to `.github/workflows` and `.github/actions`, which are maintainer-owned.
