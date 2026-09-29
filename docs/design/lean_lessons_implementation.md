# Implementing the Lean-derived improvements

This document is the implementation plan for the items selected in
[lean_lessons_design.md](lean_lessons_design.md). Each workstream is independently shippable,
independently revertible, and opt-in. Nothing here changes the kernel's execution model.

Revision note: this revision adopts the design document's architectural invariants and feature
boundaries as implementation gates, adds workstreams W6 (execution realism, L10) and W3
(session-aware scheduling, L11), and reorders delivery into phases 0 to 8 so equity identity and
market time precede optimization.

## 1. Ground rules

The nine architectural invariants in
[lean_lessons_design.md](lean_lessons_design.md) section 2 are acceptance criteria, not guidance. A
workstream that delivers its feature while breaking an invariant is rejected, not merged with
follow-up work. The decisions D12 to D18 in section 12 of the design document are binding inputs to
the corresponding workstreams. In implementation terms:

1. The runtime model is fixed: single-threaded kernel, thread-local message bus, per-event
   callbacks, `Rc<RefCell<_>>` components. No workstream may introduce a slice-style delivery model
   into the hot path.
2. New behaviour is opt-in and costs nothing when unused. Default backtest and live behaviour after
   every workstream is byte-identical to before, which the existing golden tests and the declared
   regression scenarios must continue to prove.
3. New scheduling is driven by the `Clock`, never by wall time, so backtests stay deterministic.
   Session-aware scheduling converts exchange local time to UTC at schedule time.
4. Reuse existing seams: message-bus endpoints in `crates/common/src/msgbus/switchboard.rs`, the
   `DataClient` and `ExecutionClient` traits, client and config factories, `CatalogReader` and
   `CatalogWriter`, and the `KernelEventStore` trait. Do not add a parallel data or configuration
   path.
5. Respect the feature boundary. Research workstreams compose `BacktestNode` runs and must not be
   imported by kernel crates. Tooling may depend on engine and research crates; nothing depends on
   tooling. Research code must never change the deterministic semantics of an individual backtest.
6. Raw data is immutable. Adjustment, normalization, and derived series are opt-in, recorded per
   run, and never overwrite raw prices.
7. Every workstream ships code, tests, documentation, and regenerated stubs in the same change. In
   this repository that means `make py-stubs` and `make check-generated-drift` are part of the
   change, not a follow-up.
8. Every workstream adds at least one golden regression scenario (W1). This is the mechanism that
   proves invariants 2 and 8 for the following workstreams.

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

## 3. W1: declared, regenerable regression scenarios (L5)

**Objective.** Turn the existing deterministic comparison machinery into declared, auto-discovered,
one-command-regenerable verification infrastructure, and make it the gate for every later
workstream.

**What already exists.** `CanonicalBacktestResult` in `crates/backtest/src/result.rs` produces a
versioned canonical document (`nautilus-backtest-result/v1`) with identity normalization,
`digest()`, `to_bytes()` and `from_slice()`, and `first_divergence()`.
`python/tests/acceptance/test_backtest.py` asserts golden values by hand. `BacktestResult` bindings
live in `crates/backtest/src/python/result.rs` and expose statistics but not canonical bytes or a
digest.

**Steps.**

1. Expose the canonical artifact to Python: add `canonical_bytes()` and `digest()` to the
   `BacktestResult` bindings in `crates/backtest/src/python/result.rs`, delegating to the existing
   `CanonicalBacktestResult` writer and classifying the exact-encoding failure path under
   `# Errors`.
2. Add a scenario protocol under `python/tests/regression/` with three declared layers (D18):
   - Level 1, the expected canonical digest.
   - Level 2, expected statistics such as orders, fills, positions, PnL, fees, slippage, maximum
     drawdown, and final equity.
   - Level 3, semantic checkpoints identified by event kind, instrument, and occurrence ordinal, for
     example the first `ORDER_FILLED`, `POSITION_OPENED`, and `POSITION_CLOSED` for an instrument.
     Checkpoints are semantic rather than positional so they survive unrelated event insertions.
3. Add a registry module that enumerates scenarios, so discovery is an explicit list.
4. Add a reset fixture that returns engines, caches, loggers, and clock state to a clean baseline
   between scenarios, matching Lean's hard reset
   ([AlgorithmRunner.cs](https://github.com/QuantConnect/Lean/blob/master/Tests/AlgorithmRunner.cs)).
5. Add regeneration through a pytest option or environment variable that rewrites committed
   expectations, all three layers in one command, producing a reviewable diff.
6. On mismatch, report the failing layer with its expected and actual values, then
   `first_divergence().path`, so failures name the value and the divergence rather than only a digest
   mismatch.
7. Define the scenario-per-feature requirement for later workstreams and record it in
   `docs/developer_guide/testing.md`; add `pytest-regression` to the Makefile and to `pre-flight`.

**Boundary.** Research.

**Acceptance.** The suite passes on a clean checkout; perturbing a fill model fails at the digest
layer with a named divergence path; a scenario whose later checkpoint changes fails at the checkpoint
layer with expected and actual values; regeneration rewrites all three layers and a subsequent run
passes without regenerating; default `make pytest` behaviour is unchanged.

**Risks.** Digest churn on intentional simulation changes, and three layers to update on intentional
change. Mitigation: regeneration covers all layers in one command and is explicit and reviewed, and
`first_divergence()` carries the diagnostic burden.

## 4. W2: trading calendars as data (L4)

**Objective.** A venue calendar model loaded from data and used by scheduling and session gating.

**Steps.**

1. Add `crates/model/src/calendars/` with `TradingSession` (open and close in exchange local time),
   `TradingCalendar` (weekly sessions, holidays, early closes, time zone), and a calendar key of
   venue plus instrument class plus optional symbol.
2. Define the data file as JSON under `crates/model/resources/calendars/`, loaded with `include_str!`
   for the bundled default and overridable by path. Treat the calendar as an immutable input to a
   run.
3. Keep `activation_ns` and `expiration_ns` as the source of instrument lifetime; the calendar
   answers whether a given instant is tradeable.
4. Reimplement the FX helpers in `crates/trading/src/sessions.rs` as a thin wrapper over the
   calendar, keeping existing signatures and results.
5. Validate at load and warn when calendar coverage ends before the run end.
6. Expose the calendar through the Python `nautilus_trader.model` facade and regenerate stubs.

**Boundary.** Engine and research.

**Acceptance.** A unit test resolves a known holiday and a half-day close; the FX session functions
return identical results to the current implementation across a spread of dates; a user-supplied
calendar overrides the bundled one; a golden scenario proves existing runs are unchanged.

**Risks.** Stale holiday data. Mitigation: versioned and overridable data, plus a coverage warning.

## 5. W3: session-aware scheduled events (L11)

**Objective.** Distinguish clock timers from trading-session events and deliver both through the
existing timer machinery.

**Steps.**

1. Add a scheduling layer over W2 that produces session events (premarket, open, opening-range
   complete, midday, pre-close, close, early close) and calendar events (holidays, expirations, and
   user timestamps such as economic releases).
2. Deliver events through the existing `Clock` and timer callback path
   (`crates/common/src/timer.rs`) so ordering and determinism are inherited, not reinvented.
3. Convert exchange local time to UTC at schedule time so an event fires once per backtest and once
   per live session, with no wall-clock dependence in backtest.
4. Surface the events to actors and strategies alongside the existing `on_time_event` callback, and
   document the distinction in `docs/concepts/`.
5. Add a golden scenario for a holiday and an early close.

**Boundary.** Engine and research.

**Acceptance.** A strategy scheduled at market open fires at the session open in backtest and live
sandbox; an early close shifts dependent events; a half-day produces the documented event set; no
wall-clock read appears on the backtest path.

**Risks.** Correctness depends on W2 data quality and on time zone handling. Mitigation: property
tests over session boundaries and a documented failure mode when a calendar is missing.

## 6. W4: corporate actions, identity, and the data contract (L3, L9A)

**Objective.** Represent and apply splits, dividends, delistings, and symbol changes, and define the
on-disk contract for that auxiliary data.

**Steps.**

1. Define auxiliary data types in `crates/model/src/data/` and register them with the existing
   data-type macro so catalog paths, Arrow schemas, and bus topics follow the generated convention.
2. Persist and query them through `CatalogReader` and `CatalogWriter`; the catalog gains the new
   type directories automatically.
3. Keep three representations explicitly distinct and non-interchangeable: raw input (immutable),
   derived series (adjusted prices, adjusted OHLC, total return, produced on demand), and trading
   events (split, dividend, delisting, symbol change, delivered as data).
4. Add an opt-in adjustment stage selected per data configuration. The default keeps raw data
   untouched. Where enabled, the stage converts adjusted input to raw by applying the action series
   and emits the actions as events.
5. Model identity separately from price: a mapping from venue symbol to instrument id over time,
   resolved before data reaches the engines, with renames emitted as events.
6. Model delisting as a terminal instrument status whose position outcome reuses the existing
   `InstrumentClose` path.
7. Document the contract and the file formats, with a colocated readme, and extend
   `docs/concepts/data/catalog.md`.
8. Record which representation a run consumed, so a result is interpretable later.

**Boundary.** Engine and research. File formats are the data contract (L9A).

**Acceptance.** A synthetic series with a 4:1 split and a dividend reproduces the expected raw and
adjusted prices; a rename mid-series resolves to one identity; a delisting closes positions through
the existing instrument-close path; the default path leaves existing golden digests unchanged; a
golden scenario covers split and dividend handling.

**Risks.** Mixed adjusted and raw semantics inside one strategy is the classic silent error.
Mitigation: raw data immutable, adjustment opt-in, representation recorded per run, and a golden
scenario asserting the default path is unchanged.

## 7. W5: universe definition, selection, and membership (L2)

**Objective.** A selection model and membership lifecycle that add and remove instruments at runtime
through the existing subscription machinery.

**Steps.**

1. Add a `Universe` component under `crates/trading/src/universe/`, modelled on the existing actor
   lifecycle, with a definition (rule and settings), a clock-driven selection step, and an explicit
   membership state machine: `ADDED`, `ACTIVE`, `REMOVING`, `REMOVED`.
2. Implement removal as a process, not an immediate unsubscribe: membership moves to `REMOVING`,
   open orders are cancelled or reconciled, the position policy is evaluated, the data subscription
   is released, then membership becomes `REMOVED`. Refuse removal while a position is non-flat and
   report the condition.
3. Route subscriptions through the existing data command path, respecting the ownership tracking in
   `crates/data/src/subscription.rs`, so a departing instrument releases only its own claims.
4. Request instrument metadata through the existing `request_instruments` flow, not a new provider
   interface.
5. Publish membership changes on a new bus topic in
   `crates/common/src/msgbus/switchboard.rs` and surface an `on_universe_changed` callback alongside
   the existing `on_instrument` callbacks.
6. Expose the component through `crates/trading/src/python/` and `nautilus_trader.trading`.
7. Add a golden scenario with deterministic scheduled membership.

**Boundary.** Engine.

**Acceptance.** A scheduled universe backtest reproduces the same result across reruns; a live
sandbox node can add and remove instruments without leaking subscriptions; removal with an open
position is refused with a clear log; an unused universe adds no measurable per-event cost.

**Risks.** Subscription ownership bugs and live metadata gaps. Mitigation: explicit ownership tests
in `crates/data/src/subscription.rs` and a capability check letting an adapter report that it cannot
supply metadata.

## 8. W6: execution realism (L10)

**Objective.** Make the existing realism models explicit, composable, and inventoried, before any
optimization depends on them.

**What already exists.** `crates/execution/src/models/{fee,fill,latency}.rs`; eleven fill variants
including `OneTickSlippageFillModel`, `LimitOrderPartialFillModel`, `SizeAwareFillModel`,
`CompetitionAwareFillModel`, `VolumeSensitiveFillModel`, and `MarketHoursFillModel`; seeded
probabilistic fills via `ProbabilisticFillState`; `liquidity_consumption` and `queue_position` in
`OrderMatchingEngineConfig`; margin models in `crates/model/src/accounts/margin_model.rs`;
`MarketStatusAction::Halt`.

**Steps.**

1. Inventory and document the matrix of fee, fill, latency, queue, and liquidity behaviour per venue
   and instrument class, in `docs/concepts/backtesting/fill-models.md` and the matching engine
   documentation.
2. Stage A, abstraction without behaviour change (D14): introduce a configuration layer around the
   existing models and prove the default path is unchanged. Wrap the eleven existing fill variants
   rather than rewriting them, and migrate them incrementally through a compatibility adapter.
3. Implement model selection as an inheritance chain: global defaults, then venue defaults, then
   matching-engine defaults, then instrument overrides, then order-specific overrides. Each level
   overrides only what it changes; do not duplicate configuration at every level.
4. Stage B, independent components (D15): make fill, slippage, and fee independently configurable,
   with semantics ordered as fill eligibility, then fill quantity, then base fill price, then
   slippage adjustment, then final fill price, then fees. Stage B lands only after Stage A is proven.
5. Evaluate each candidate addition independently and opt-in: market impact, spread, partial-fill
   policy, borrow and locate availability for short selling, and auction and halt policy. Prefer
   extending `OrderMatchingEngineConfig` and adding a `FillModel` implementation over a new
   abstraction.
6. Ensure every addition is deterministic, taking an explicit seed where randomness is involved.
7. Add one golden scenario per added model.

**Boundary.** Engine and research. Models are used by both backtest and sandbox execution.

**Acceptance.** The inventory document matches the code. Stage A is complete only when
`old_digest == new_digest` for every existing golden scenario. Stage B is complete only when
independently configured slippage reproduces the composite behaviour it replaces. Each new model is
independently selectable and leaves the default path byte-identical, seeded models reproduce across
runs, and existing fill variants continue to pass their tests unchanged.

**Migration invariant.** Separating an abstraction must not automatically change simulation
semantics. This is the acceptance criterion that makes the staged migration safe.

**Risks.** Configuration surface growth and golden churn. Mitigation: one model per change, opt-in,
never more than one concern migrated at a time, and a golden scenario per model.

## 9. W7: optional Signal, Target, and Execution pipeline (L1)

**Objective.** An optional pipeline (D12) between decision-making and order submission. The direct
path and the order layer are unchanged, and both paths converge on `ExecutionAlgorithm`.

**Steps.**

1. Define a signal value in `crates/model/` carrying direction, horizon, strength, source, expiry,
   and provenance. Do not include fields named confidence or score. Note that
   `crates/common/src/signal.rs` already defines a generic `Signal` with name, value, and
   timestamps; the new type is distinct and must not overload it. A signal is not a trading command
   and carries no order quantity.
2. Define a target value carrying an instrument plus a target quantity, weight, or notional.
3. Add an optional portfolio-construction component that consumes portfolio context and risk
   context and produces targets, reusing `crates/risk/src/sizing.rs` for sizing and the `RiskEngine`
   for pre-trade checks. Do not introduce mandatory `AlphaModel`, `PortfolioConstructionModel`,
   `RiskManagementModel`, or `ExecutionModel` interfaces.
4. Add a reconciler that compares targets with cache and portfolio state and emits the minimal order
   set through `ExecutionAlgorithm`. The cache and portfolio stay authoritative; targets never
   become a second position store.
5. Keep direct order submission fully supported and unchanged; the pipeline is opt-in per strategy,
   and both paths must produce equivalent orders for equivalent intent.
6. Document the three-layer separation in `docs/concepts/`, including that signal is not target and
   target is not order, and that neither is a trading command.
7. Add a target-to-order parity golden scenario.

**Boundary.** Engine, optional per strategy.

**Acceptance.** A strategy using the pipeline produces identical orders to an equivalent strategy
that submits orders directly for a simple case; the pipeline is disabled by default; the hot path
shows no measurable regression when it is unused; the parity golden scenario passes.

**Risks.** Competing ownership of position intent. Mitigation: reconciliation against the
authoritative cache before any order is emitted, and the feature stays behind a flag until proven.

## 10. W8: optimization as an external research subsystem (L6)

**Objective.** A research optimizer over backtest runs with an explicit methodology boundary, split
between Python orchestration and Rust execution (D16).

**Steps.**

1. Define the objective as a function over the existing portfolio statistics in
   `crates/analysis/src/statistics/`, with constraints over the same values.
2. Split responsibilities: Rust owns backtest execution, simulation, and result production; Python
   owns parameter spaces, experiment generation, search algorithms, result aggregation, walk-forward
   experiments, and experiment persistence.
3. Separate search from execution: a strategy enumerates parameter sets; a runner executes them.
   Mirror the split between Lean's host and
   [GridSearchOptimizationStrategy.cs](https://github.com/QuantConnect/Lean/blob/master/Optimizer/Strategies/GridSearchOptimizationStrategy.cs)
   without adopting its single-process queue semantics.
4. Fan out runs at the process level so the single-threaded kernel and the Python GIL are not
   constraints, with a documented concurrency limit driven by memory rather than CPU.
5. Emit results as canonical backtest results plus the parameter set, comparable by digest.
6. Model the methodology stages explicitly: train, optimize, validate, out-of-sample, walk-forward.
   These are distinct stages, not one loop over a grid.
7. Expose an `optimize` subcommand in `crates/cli/src/opt.rs` as a thin front end that invokes the
   same Python optimization API, plus a Python helper for notebooks. There must be exactly one
   optimization implementation and one semantic model.

**Boundary.** Research. The optimizer composes runs and must never reach into one; it may not alter
the deterministic semantics of an individual backtest.

**Acceptance.** A sweep over a small grid returns the same best result as running the grid by hand;
an objective over Sharpe ratio and maximum drawdown behaves as specified; a failing run does not
abort the sweep; the CLI and the Python API return identical results for the same configuration; a
reproducible optimization golden scenario passes.

**Risks.** Memory blowup under concurrent catalog reads; methodology being conflated with search; the
Python orchestration layer becoming a second execution path for backtests. Mitigation: documented
concurrency limit, explicit stage separation, and routing all execution through `BacktestNode`.

## 11. W9: research API over shared primitives (L7)

**Objective.** Notebook workflows over the same primitives as backtests, without a second runtime.

**Steps.**

1. Add a Python research module under `python/nautilus_trader/analysis/` that opens a
   `ParquetDataCatalog`, loads instruments and data with the existing bindings, and exposes typed
   Nautilus objects plus a DataFrame conversion via
   `python/nautilus_trader/persistence/catalog_to_df.py`.
2. Compute indicators with the existing indicator API rather than a second implementation.
3. Provide a small replay helper that yields data in `ts_init` order so notebook code mirrors
   strategy logic without constructing an engine.
4. Do not add a research mode or a query-only engine to the kernel.
5. Add a notebook example under `examples/backtest/notebooks/` and a test that reproduces the values
   a backtest of the same data produces.

**Boundary.** Research.

**Acceptance.** The example loads a catalog, computes an indicator, and matches the backtest values
for the same data; the module imports only catalog, wrangler, and indicator code also used by
`BacktestNode`.

**Risks.** A divergent second data path. Mitigation: reuse the catalog and wrangler code paths, and
assert equality against a backtest in a test.

## 12. W10: optional configuration serialization (L8)

**Objective.** Optional serialization and file loading for existing typed configs. Typed constructors
remain the canonical API (D17); this workstream is developer and operator convenience and is a
prerequisite for nothing else.

**Steps.**

1. Define a schema mirroring `NautilusKernelConfig`, `BacktestEngineConfig`, and `LiveNodeConfig`
   fields, treated as a view of the typed configs rather than a second configuration model.
2. If file loading is implemented, keep layering inside the loader and limited to built-in defaults,
   the file, and explicit overrides. Do not add environment profiles or environment variables as a
   configuration source.
3. Validate in Rust so errors are raised at construction with typed messages, and reject unknown
   keys consistent with the `deny_unknown_fields` convention in adapter configs.
4. Expose it through `nautilus-cli` and the Python constructors; the typed constructors remain
   authoritative, and serialization must round-trip to an equal typed config.
5. Add a test asserting the file schema stays in step with the typed configs.

**Boundary.** Tooling.

**Acceptance.** A serialized file round-trips to an equal typed config; an unknown key fails
validation; the schema-drift test fails when a typed config field is added without updating the file
schema; omitting the feature entirely leaves all other workstreams unaffected.

**Risks.** A second input surface drifting from the typed configs. Mitigation: the schema-drift test,
or omit the feature.

## 13. W11: data CLI (L9B)

**Objective.** A small data surface in `nautilus-cli` over the catalog and existing loaders.

**Steps.**

1. Add data subcommands in `crates/cli/src/opt.rs`: download, validate, convert, generate, inspect,
   each delegating to `CatalogReader` and `CatalogWriter` and the existing loaders.
2. Keep the surface explicit and licensing-aware; do not embed provider credentials or bundle data.
3. Emit machine-readable output for CI use.
4. Document the commands and the data contract from W4.

**Boundary.** Tooling.

**Acceptance.** `validate` reports a malformed catalog; `convert` produces a catalog that a backtest
reads; `inspect` lists data types, instruments, and coverage; no credentials are written to disk.

**Risks.** Scope growth into a data platform. Mitigation: the five documented subcommands are the
whole surface.

## 14. Sequencing

| Phase | Workstreams | Contents                                                                                             | Rationale                                    |
| ----- | ----------- | ---------------------------------------------------------------------------------------------------- | -------------------------------------------- |
| 0     | Contract    | Invariants, boundaries, determinism rules, raw versus derived semantics, signal and target ownership | Makes the remaining phases enforceable       |
| 1     | W1          | Regression scenarios and verification infrastructure                                                 | Gate for every later phase                   |
| 2     | W2, W3      | Calendar, sessions, holidays, early closes, session scheduling                                       | Market-time foundation                       |
| 3     | W4          | Corporate actions, identity, delisting, normalization, data contract                                 | Equity correctness before optimization       |
| 4     | W5          | Universe definition, selection, membership                                                           | Depends on market time and identity          |
| 5     | W6          | Execution realism inventory and additions                                                            | Must precede optimization                    |
| 6     | W7          | Signal, target, reconciliation                                                                       | Deepest engine change                        |
| 7     | W8, W9      | Optimizer, research API, walk-forward                                                                | Research over a correct and realistic engine |
| 8     | W10, W11    | Configuration precedence, data CLI                                                                   | Ergonomics once semantics are settled        |

Phase 0 is documentation and review, not code: it is the acceptance of section 2 of the design
document. Each later phase depends only on the phases above it.

## 15. Cross-cutting acceptance criteria

- Default behaviour is unchanged: existing golden backtest results and their digests are identical
  before and after each workstream, which W1 makes checkable.
- Each workstream adds at least one declared regression scenario covering its feature.
- `make format`, `make pre-commit`, `make cargo-test`, and `make pytest` pass for the affected
  areas.
- Regenerated stubs and docstrings are committed, and `make check-generated-drift` passes.
- New public Rust items carry `# Errors` and `# Panics` sections where the conventions require them,
  and new Python bindings follow the `Py*` wrapper and `py_*` method naming.
- New hot-path code has a benchmark comparison recorded in the pull request.
- Documentation is updated in the same change, including the concept page for the affected area.
- No research or tooling crate is imported by a kernel crate.

## 16. Risks

| Risk                                   | Impact | Mitigation                                                             |
| -------------------------------------- | ------ | ---------------------------------------------------------------------- |
| Invariant erosion through feature work | High   | Section 1 gates; a golden scenario per feature                         |
| Scope creep into the kernel runtime    | High   | Ground rule 1; reject any slice-style delivery change                  |
| Determinism regression                 | High   | Clock-driven scheduling; digest checks in CI                           |
| Adjusted and raw semantics mixed       | High   | Immutable raw data; opt-in adjustment; representation recorded per run |
| Subscription ownership bugs            | High   | Ownership tests before W5 lands                                        |
| Optimizing on unrealized execution     | High   | W6 precedes W8 by construction                                         |
| Golden churn                           | Medium | Explicit regeneration; divergence paths in failures                    |
| Two configuration surfaces             | Medium | Schema-drift test                                                      |
| Memory blowup in optimization          | Medium | Documented concurrency limit; process fan-out only                     |
| Research leaking into the kernel       | Medium | Feature boundary rules and a no-research-dependency check              |

## 17. Out of scope

- A cloud platform, job queue, or multi-tenant scheduler.
- Dynamic handler resolution by class-name string from configuration.
- A slice or time-batch delivery model in the runtime.
- A research engine mode or a query-only kernel mode.
- Transplanting Lean's `Security` model-slot hierarchy.
- A Lean-compatible user-facing programming model, that is, compatibility target A in the design
  document.
- A second optimizer implementation alongside the Python optimization API.
- Environment profiles or environment variables as a configuration source.
- AI or ML tooling.
- Changes to `.github/workflows` and `.github/actions`, which are maintainer-owned.
