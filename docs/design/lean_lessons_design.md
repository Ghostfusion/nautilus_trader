# Lean lesson review: design considerations

This document records what NautilusTrader can learn from QuantConnect Lean. It is a design
document: it states the problem, the candidate designs, and the tradeoffs. It is not a
commitment to implement anything, and it does not change the architecture of the engine.

The companion implementation plan is [lean_lessons_implementation.md](lean_lessons_implementation.md).

## 1. Purpose and scope

Goals:

- Identify capabilities Lean has that NautilusTrader does not, or implements differently.
- Judge each gap against NautilusTrader's existing design, not in isolation.
- Recommend a priority order with explicit tradeoffs and risks.

Non-goals:

- Copying Lean's technology stack. Lean is C#/.NET with Python via pythonnet; NautilusTrader is
  a Rust core with PyO3 bindings. The runtime differences are deliberate and are not reconsidered.
- Making NautilusTrader a multi-tenant cloud platform. Lean's cloud is a product, not an engine
  feature.
- Adding AI/ML tooling, which the project roadmap keeps out of scope.

## 2. What Lean is

Verified from the Lean repository (all links point at `master`):

- **Runtime loop.** `Engine/Engine.cs` composes an engine from pluggable handlers, and
  `Engine/AlgorithmManager.cs` consumes an ordered `IEnumerable<TimeSlice>` produced by
  `ISynchronizer.StreamData`. A `TimeSlice` bundles every subscription's data that is due at the
  current time frontier plus security updates, consolidator inputs, universe data, and
  `SecurityChanges`; the user-facing `Slice` is handed to `IAlgorithm.OnData` at most once per
  slice. See
  [ISynchronizer.cs](https://github.com/QuantConnect/Lean/blob/master/Engine/DataFeeds/ISynchronizer.cs),
  [TimeSlice.cs](https://github.com/QuantConnect/Lean/blob/master/Engine/DataFeeds/TimeSlice.cs).
- **Backtest and live share one engine.** Mode is determined by which handlers are composed.
  Backtest uses `FileSystemDataFeed` + `Synchronizer` + `BacktestingTransactionHandler` +
  `BacktestingBrokerage`; live uses `LiveTradingDataFeed` + `LiveSynchronizer` +
  `BrokerageTransactionHandler` + a real `IBrokerage`. Handler types are resolved by type-name
  string from `Launcher/config.json`.
  See [IBrokerage.cs](https://github.com/QuantConnect/Lean/blob/master/Common/Interfaces/IBrokerage.cs),
  [IDataFeed.cs](https://github.com/QuantConnect/Lean/blob/master/Engine/DataFeeds/IDataFeed.cs).
- **Algorithm Framework.** A strategy is decomposed into Universe Selection, Alpha, Portfolio
  Construction, Risk Management, and Execution models that communicate through a single `Insight`
  abstraction. See
  [QCAlgorithm.Framework.cs](https://github.com/QuantConnect/Lean/blob/master/Algorithm/QCAlgorithm.Framework.cs),
  [IAlphaModel.cs](https://github.com/QuantConnect/Lean/blob/master/Algorithm/Alphas/IAlphaModel.cs),
  [Insight.cs](https://github.com/QuantConnect/Lean/blob/master/Common/Algorithm/Framework/Alphas/Insight.cs).
- **Universe selection.** `Universe` and `UniverseSettings` drive dynamic symbol add/remove at
  runtime with coarse/fine fundamental chaining and `SecurityChanges` notifications; additions are
  deferred and flushed at the end of the time step.
  See [Universe.cs](https://github.com/QuantConnect/Lean/blob/master/Common/Data/UniverseSelection/Universe.cs),
  [QCAlgorithm.Universe.cs](https://github.com/QuantConnect/Lean/blob/master/Algorithm/QCAlgorithm.Universe.cs).
- **Instruments as composed objects.** `Security` carries model slots (fee, fill, slippage,
  buying power, settlement, margin interest, volatility) and `SecurityHolding` carries the
  `Target` used by portfolio construction.
  See [Security.cs](https://github.com/QuantConnect/Lean/blob/master/Common/Securities/Security.cs),
  [SecurityService.cs](https://github.com/QuantConnect/Lean/blob/master/Common/Securities/SecurityService.cs).
- **Metadata as data.** Trading hours live in `Data/market-hours/market-hours-database.json` and
  contract properties in `Data/symbol-properties/symbol-properties-database.csv`, both loaded by
  the engine and mutable at runtime.
  See [MarketHoursDatabase.cs](https://github.com/QuantConnect/Lean/blob/master/Common/Securities/MarketHoursDatabase.cs).
- **Corporate actions and identity.** Equity prices are adjusted from factor files
  (`date,priceFactor,splitFactor,referencePrice`) and ticker identity and delisting dates come
  from map files; `Split`, `Dividend`, `Delisting`, and `SymbolChangedEvent` are emitted as
  auxiliary data. Normalization is per subscription via `DataNormalizationMode`.
  See [FactorFile.cs](https://github.com/QuantConnect/Lean/blob/master/Common/Data/Auxiliary/FactorFile.cs),
  [MapFile.cs](https://github.com/QuantConnect/Lean/blob/master/Common/Data/Auxiliary/MapFile.cs),
  [SubscriptionDataReader.cs](https://github.com/QuantConnect/Lean/blob/master/Engine/DataFeeds/SubscriptionDataReader.cs).
- **Optimization.** A host (`Optimizer/LeanOptimizer.cs`) drives many backtests through a
  pluggable `IOptimizationStrategy` (grid search and Euler search) with objectives and
  constraints, streaming `OptimizationResult` values. See
  [LeanOptimizer.cs](https://github.com/QuantConnect/Lean/blob/master/Optimizer/LeanOptimizer.cs).
- **Research.** `QuantBook : QCAlgorithm` reuses the same security, slice, and history primitives
  in Jupyter. See [QuantBook.cs](https://github.com/QuantConnect/Lean/blob/master/Research/QuantBook.cs).
- **Regression suite.** Each scenario implements `IRegressionAlgorithmDefinition` with inline
  `ExpectedStatistics`, discovered by reflection; the runner resets global state, overrides
  handlers with regression-safe implementations, and asserts the statistics.
  See [RegressionTests.cs](https://github.com/QuantConnect/Lean/blob/master/Tests/RegressionTests.cs),
  [IRegressionAlgorithmDefinition.cs](https://github.com/QuantConnect/Lean/blob/master/Common/Interfaces/IRegressionAlgorithmDefinition.cs).
- **Distribution.** A single .NET launcher plus `quantconnect/lean` and `quantconnect/research`
  Docker images; the `lean` CLI orchestrates local runs and cloud runs from one project config.

## 3. Where NautilusTrader stands today

Verified in this repository:

- The kernel is single-threaded with a thread-local message bus, and components are
  `Rc<RefCell<_>>`; the engines are wired through bus endpoints in
  `crates/common/src/msgbus/switchboard.rs`.
- Strategies receive one callback per data event (`DataActor`/`Strategy` in
  `crates/trading/src/strategy/mod.rs`), not a time-batched slice.
- Venue connectivity is trait-based (`DataClient` and `ExecutionClient` in
  `crates/common/src/clients/`), with 18 adapters under `crates/adapters/`.
- Backtest and live share strategy code and the execution stack
  ([../concepts/architecture.md](../concepts/architecture.md)).
- Parameter values are typed `Price`/`Quantity`/`Money` with exact arithmetic, and time is
  nanosecond `UnixNanos`.
- Portfolio statistics are extensive: 35 implementations under `crates/analysis/src/statistics/`
  plus `PortfolioAnalyzer` and snapshot types in `crates/analysis/src/`.
- Deterministic result comparison already exists: `crates/backtest/src/result.rs` defines
  `CanonicalBacktestResult` with a versioned schema (`nautilus-backtest-result/v1`), identity
  normalization, `digest()`, and `first_divergence()`, and acceptance tests assert golden values,
  for example `test_backtest_cash_margin_account_order_fill_position_parity_golden` and
  `test_rerun_ema_cross_strategy_returns_identical_performance` in
  `python/tests/acceptance/test_backtest.py`.
- A generic user signal type exists (`Signal` in `crates/common/src/signal.rs`, name plus value
  plus timestamps), and option-chain aggregation exists (`crates/data/src/option_chains/`), but
  neither provides insight semantics or a portfolio-construction layer.
- There is no `Universe` selection model, no insight or alpha model, no portfolio-construction or
  target-reconciliation layer, no corporate-action or price-normalization handling, no
  trading-calendar database, and no parameter optimization. A repository search for those concepts
  returns nothing outside adapter-specific parsing.
- `crates/cli/src/opt.rs` exposes only `database` and `blockchain` subcommands.

## 4. Comparison

| Dimension              | QuantConnect Lean                         | NautilusTrader                                        | Assessment                                       |
| ---------------------- | ----------------------------------------- | ----------------------------------------------------- | ------------------------------------------------ |
| Delivery unit          | `TimeSlice` bundle per frontier           | One callback per data event                           | Ours gives lower latency; theirs gives coherence |
| Strategy decomposition | Five framework models + `Insight`         | `Strategy` + `ExecutionAlgorithm`                     | Gap: no signal/portfolio layer                   |
| Universe               | Dynamic, scheduled, coarse/fine           | Static subscriptions per instrument                   | Gap: no dynamic universe abstraction             |
| Instrument models      | Per-security model slots                  | Per-venue engines plus shared models                  | Different split of responsibility                |
| Corporate actions      | Factor and map files, normalization modes | None                                                  | Gap for equities and derivatives                 |
| Trading calendar       | Market-hours JSON as data                 | Per-instrument `activation_ns`/`expiration_ns`        | Gap: no venue calendar or holidays               |
| Optimization           | Engine component with strategies          | None                                                  | Gap                                              |
| Research               | `QuantBook` on engine primitives          | Catalog plus analysis tearsheets                      | Partial gap                                      |
| Regression tests       | Declared statistics, reflection discovery | Canonical result with digest and divergence reporting | Small workflow gap, high value                   |
| Packaging              | One CLI plus Docker plus cloud            | Make targets, wheels, Docker                          | Ours is comparable for self-hosting              |

## 5. Candidate learnings

### L1. A first-class signal and target layer

- **Lean mechanism.** `Insight` is the universal signal between models; portfolio construction
  emits `IPortfolioTarget` values, and execution consumes targets. See
  [PortfolioTarget.cs](https://github.com/QuantConnect/Lean/blob/master/Common/Algorithm/Framework/Portfolio/PortfolioTarget.cs).
- **Problem here.** NautilusTrader strategy code goes straight from a decision to
  `submit_order`. There is no typed representation of intent that can be produced by one component,
  modified by another, and reconciled against current positions.
- **Proposed design.** A `Signal` value type in `crates/model` (instrument, direction, period,
  magnitude, confidence, source, expiry, score) and an optional `PortfolioConstruction` layer that
  converts signals into `Target` values, with a reconciler that compares targets to
  `Portfolio` positions and emits the minimal order set. Strategies that submit orders directly
  remain unchanged.
- **Tradeoffs.** New abstraction in the hot path; must not slow the direct-order path. Target
  reconciliation introduces a second source of truth for position intent and needs careful
  interaction with `RiskEngine` checks and the order emulator.
- **Effort and impact.** High effort, high impact for systematic multi-signal strategies and for
  making alpha components independently testable.

### L2. Dynamic universe selection

- **Lean mechanism.** `Universe`, `UniverseSettings`, `SecurityChanges`, deferred additions,
  `MinimumTimeInUniverse`. See
  [Engine UniverseSelection.cs](https://github.com/QuantConnect/Lean/blob/master/Engine/DataFeeds/UniverseSelection.cs).
- **Problem here.** Instruments are added explicitly before a run, and subscriptions are managed
  per component. `DataEngine` supports runtime subscribe and unsubscribe, and `DataActor` has
  `on_instrument` and `on_instrument_status`, but there is no selection model, no membership
  lifecycle, and no scheduling.
- **Proposed design.** A `Universe` component type that owns a selection function over
  instruments, subscribes and unsubscribes through the existing data engine, schedules
  re-selection through the clock, and publishes membership changes on the bus for interested
  actors. Reuse the existing request/response path (`request_instruments`) rather than inventing a
  second data path.
- **Tradeoffs.** Live universes need venue metadata that some adapters cannot provide; removal
  semantics must respect open orders and positions. Backtest determinism depends on selection
  ordering, so selection must be driven by the clock, not wall time.
- **Effort and impact.** Medium to high effort, high impact for equity and options workflows.

### L3. Corporate actions, identity mapping, and price normalization

- **Lean mechanism.** Factor files scale prices; map files carry ticker identity, renames, and
  delisting dates; `Split`, `Dividend`, `Delisting`, `SymbolChangedEvent` are emitted as
  auxiliary data; normalization is per subscription.
- **Problem here.** The catalog stores raw venue data with no adjustment step, instruments are
  immutable definitions without symbol history, and there is no adjustment or delisting concept.
  A backtest over an adjusted-price provider silently mixes adjusted and unadjusted assumptions.
- **Proposed design.** Add auxiliary data types to `crates/model/src/data/` (split, dividend,
  delisting, symbol rename) with catalog persistence, plus an adjustment stage that consumes them:
  either applied at catalog read time or emitted as events on the bus. Instrument identity is
  handled by an explicit mapping from venue symbol to instrument id over time, resolved before
  data reaches the engines.
- **Tradeoffs.** Adjustment mutates price history, which conflicts with the project's exact-arithmetic
  and raw-data principles unless raw data is preserved. Requires a data-licensing story for
  factor data. Not meaningful for crypto and FX, so it must be opt-in.
- **Effort and impact.** Medium to high effort, high impact for equities, ETFs, and options.

### L4. Trading calendars as data

- **Lean mechanism.** `market-hours-database.json` holds time zones, session segments, and
  holidays; the engine loads it and allows runtime overrides.
- **Problem here.** Instruments carry `activation_ns` and `expiration_ns`, and `crates/trading/src/sessions.rs`
  covers four FX sessions in code. There is no venue calendar, no holiday handling, and no
  session-aware scheduling primitive.
- **Proposed design.** A calendar model in `crates/model` (sessions, time zones, holidays,
  early closes) with a data file format and loader, consumed by timers, universe scheduling, and
  any session-aware logic. Keep the FX helpers as a thin wrapper.
- **Tradeoffs.** Another external dataset to maintain; time zone and holiday data go stale.
  Must be optional so existing workflows do not pay for it.
- **Effort and impact.** Medium effort, medium impact; prerequisite for clean universe scheduling.

### L5. A declared, regenerable regression harness

- **Lean mechanism.** Scenario-declared `ExpectedStatistics`, reflection discovery, hard reset of
  global state, regression-safe handlers, per-scenario config overrides, and an in-place
  regeneration flag.
- **Problem here.** The deterministic machinery already exists: `CanonicalBacktestResult` in
  `crates/backtest/src/result.rs` provides a versioned canonical document, a `digest()`, and
  `first_divergence()`, and acceptance tests assert golden values. What is missing is the
  workflow around it: expectations are hand-written per test, scenarios are not auto-discovered,
  there is no registry that enumerates them, no single command regenerates committed expectations,
  and there is no documented hard reset of shared state between scenarios.
- **Proposed design.** Reuse `CanonicalBacktestResult` as the golden artifact rather than inventing
  a second hash. Each scenario declares expected statistics plus an expected digest; a registry
  enumerates scenarios for discovery; a regeneration flag rewrites the committed expectations from
  an actual run so updates appear as a reviewable diff; the harness resets engine and component
  state between scenarios.
- **Tradeoffs.** Digests are brittle by design; regeneration must be deliberate and reviewed.
  `first_divergence()` already mitigates failure diagnosis, so combine it with the digest rather
  than replacing it with a statistics-only comparison.
- **Effort and impact.** Low effort, high impact; the cheapest item to adopt.

### L6. Parameter optimization as a subsystem

- **Lean mechanism.** `LeanOptimizer` plus `IOptimizationStrategy` (grid and Euler search),
  objectives, constraints, and streamed results.
- **Problem here.** Users write their own sweeps. `nautilus-cli` has no optimize command, and
  there is no objective or constraint abstraction.
- **Proposed design.** An optimizer front end over `BacktestNode` runs, in `crates/cli` and the
  Python layer, with pluggable strategies, an objective function over existing portfolio
  statistics, constraints, and a result stream. Parallelism comes from process-level fan-out of
  independent backtests, not from threading inside one engine.
- **Tradeoffs.** Process fan-out multiplies memory and data loading cost; the catalog must be
  readable concurrently. Avoid a distributed scheduler in the core.
- **Effort and impact.** Medium effort, high impact for research workflows.

### L7. Research that shares engine primitives

- **Lean mechanism.** `QuantBook : QCAlgorithm` gives notebooks the same security, slice, and
  history APIs as backtests.
- **Problem here.** Research is catalog query plus wranglers plus pandas, which duplicates
  instrument loading and time semantics outside the engine.
- **Proposed design.** A research wrapper that boots engines in a query-only mode, exposing
  catalog queries, indicator computation, and event replay through the same instrument and bar
  types used in backtest. Keep it Python-side; do not add a new runtime mode to the kernel.
- **Tradeoffs.** Risk of a second, divergent data path. Keep the wrapper thin over existing
  components.
- **Effort and impact.** Low to medium effort, medium impact.

### L8. Single configuration schema with environment layering

- **Lean mechanism.** One `lean.json` with an `environment` `section` layered over top-level keys,
  consumed identically by local CLI, backtest, live, and cloud.
- **Problem here.** Configuration is typed and better validated, but there is no file-based
  configuration with environment layering or CLI-managed subsets; each run script constructs
  config objects.
- **Proposed design.** Optional YAML or JSON configuration loading that maps onto existing typed
  config builders, with named environments and a documented precedence order. No change to the
  in-memory config types.
- **Tradeoffs.** Two configuration surfaces to keep consistent; typed constructors must remain the
  source of truth.
- **Effort and impact.** Low effort, medium impact; mostly ergonomics for scripted and CI workflows.

### L9. Auxiliary metadata and data-management tooling

- **Lean mechanism.** A documented data folder contract, sidecar factor and map files, and
  `lean data download` and `lean data generate` commands.
- **Problem here.** The catalog layout is documented
  ([../concepts/data/catalog.md](../concepts/data/catalog.md)) and scripts exist, but there is no
  CLI for data acquisition or conversion.
- **Proposed design.** Extend `nautilus-cli` with data commands over the catalog and existing
  loaders, and formalize the sidecar file contract once L3 and L4 exist.
- **Tradeoffs.** Data acquisition is licensing-sensitive; keep the surface small and explicit.
- **Effort and impact.** Low effort, low to medium impact; depends on L3 and L4.

## 6. What not to copy

- **`TimeSlice` batching as the primary delivery model.** It guarantees coherence but adds latency
  and couples all subscriptions to one frontier. NautilusTrader's per-event callbacks and
  nanosecond ordering are a deliberate advantage for high-frequency work. Any slice-like view
  should be an optional convenience, not the runtime.
- **Handler resolution by type-name string.** Composing behaviour from untyped strings in a config
  file trades compile-time safety for flexibility. NautilusTrader's factories and typed configs are
  preferable.
- **Mutable-at-runtime global metadata.** Lean allows runtime edits to market hours and symbol
  properties. For a deterministic engine, treat calendar and metadata as immutable inputs to a run.
- **Config with comments and implicit layering.** A single sprawling config file invites hidden
  precedence. If file-based configuration is added (L8), precedence must be explicit and tested.
- **A cloud-coupled optimizer.** Keep optimization out of the kernel; it is a research tool.
- **Per-strategy bespoke expectations.** L5 should replace ad-hoc expectations with one harness,
  not add a second convention alongside it.

## 7. Recommended priority

| Priority | Item                        | Effort | Impact | Why now                                         |
| -------- | --------------------------- | ------ | ------ | ----------------------------------------------- |
| 1        | L5 regression harness       | Low    | High   | Cheap, independent, protects every later change |
| 2        | L4 trading calendars        | Medium | Medium | Prerequisite for L2 and L7 scheduling           |
| 3        | L2 universe selection       | High   | High   | Largest functional gap for equities workflows   |
| 4        | L6 optimization             | Medium | High   | Research workflows; independent of core         |
| 5        | L7 research wrapper         | Low    | Medium | Ergonomics; builds on L5 and the catalog        |
| 6        | L3 corporate actions        | High   | High   | Large data and semantics project; needs L4      |
| 7        | L8 file-based configuration | Low    | Medium | Ergonomics; independent                         |
| 8        | L1 signal and target layer  | High   | High   | Deepest change; do last and behind a feature    |
| 9        | L9 data CLI                 | Low    | Medium | Depends on L3 and L4                            |

## 8. Open questions

- Does the project want a portfolio-construction layer at all, or is direct order submission with
  `ExecutionAlgorithm` sufficient for the intended use cases? This determines whether L1 is worth
  the risk.
- Which asset classes justify L3? If the roadmap stays crypto, FX, and derivatives heavy, L3 is
  lower value than it appears.
- Where should optimization live: `nautilus-cli` as a Rust command, the Python layer, or a thin
  wrapper over both? Process fan-out interacts with the Python GIL and the Rust runtime.
- Is a file-based configuration format desired upstream, given the repository's typed-constructor
  convention?
- For L5, is a hash over order and fill sequences stable enough given intentional simulation
  changes, or should the golden be a statistics set plus a small, explicit event sample?
