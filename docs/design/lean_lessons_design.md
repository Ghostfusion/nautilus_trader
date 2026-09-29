# Lean lesson review: design considerations

This document records what NautilusTrader can learn from QuantConnect Lean. It is a design
document: it states the problem, the candidate designs, the boundaries, and the tradeoffs. It is
not a commitment to implement anything, and it does not change the architecture of the engine.

The companion implementation plan is [lean_lessons_implementation.md](lean_lessons_implementation.md).

Revision note: this revision adds the architectural invariants, the anti-porting specification, the
engine/research/tooling classification, and workstreams L10 (execution realism) and L11
(session-aware scheduled events), and reorders the roadmap so equity identity and market time
precede optimization.

## 1. Purpose and scope

Goals:

- Identify capabilities Lean has that NautilusTrader does not, or implements differently.
- Judge each gap against NautilusTrader's existing design, not in isolation.
- Recommend a priority order with explicit tradeoffs, invariants, and risks.

Non-goals:

- Copying Lean's technology stack. Lean is C#/.NET with Python via pythonnet; NautilusTrader is a
  Rust core with PyO3 bindings. The runtime differences are deliberate and are not reconsidered.
- Making NautilusTrader a multi-tenant cloud platform. Lean's cloud is a product, not an engine
  feature.
- Adding AI/ML tooling, which the project roadmap keeps out of scope.

### The central thesis

> Borrow semantics from Lean, not implementation mechanisms.
>
> A Lean mechanism may be adopted only when its semantics close a demonstrated NautilusTrader
> capability gap without weakening NautilusTrader's event-driven, deterministic, typed, low-latency
> architecture.

This is the guiding principle for every item below, and it is the reason some Lean capabilities are
deliberately rejected in section 9. "Lean has X and we do not" is not by itself a reason to build X.

## 2. Architectural invariants

These are the properties the Lean integration must preserve. They are the acceptance criteria that
outrank feature completion: a workstream that delivers a feature while breaking an invariant is
rejected, not merged with follow-up work.

1. **Event-driven execution.** No mandatory global time-slice barrier. Data continues to be
   delivered event by event. Any coherence convenience is opt-in and must not sit in the data path.
2. **Nanosecond event ordering.** Equal timestamps keep deterministic ordering. New components must
   not introduce an ordering that depends on hash iteration, wall time, or thread scheduling.
3. **Backtest and live parity.** Strategy logic must not fork because a feature was inspired by
   Lean. Anything that changes strategy-visible behaviour changes it in both environments.
4. **Typed Rust core.** No string-based runtime dependency injection in the kernel. Extension
   points stay typed traits resolved by factories, not by class-name lookup in a configuration file.
5. **Exact arithmetic.** Corporate actions, normalization, and target sizing must use the project
   domain types or `Decimal`. No adjustment layer may silently destroy raw-price semantics.
6. **Single source of truth for position state.** Signal, target, and order layers must not create
   competing ownership of position intent. The cache and portfolio remain authoritative.
7. **Deterministic replay.** Universe selection, calendars, corporate actions, and optimization
   must produce reproducible results for the same inputs.
8. **Direct-order compatibility.** Existing strategies that submit orders directly must continue to
   work unchanged, with no requirement to adopt new framework abstractions.
9. **No mandatory overhead when a feature is unused.** Universe selection, portfolio construction,
   optimization, and corporate actions must cost nothing when they are not used.

## 3. Feature boundaries: engine, research, tooling

Every item is classified so research functionality does not gradually leak into the trading kernel.
"Engine" means it affects the runtime path and is subject to the invariants in section 2.
"Research" means it runs outside the runtime, over backtest or catalog data. "Tooling" means it
supports workflows without participating in a run.

| Item                           | Engine | Research | Tooling |
| ------------------------------ | ------ | -------- | ------- |
| L1 Signal, Target, Order       | Yes    |          |         |
| L2 Universe and membership     | Yes    |          |         |
| L3 Corporate actions, identity | Yes    | Yes      |         |
| L4 Trading calendar            | Yes    | Yes      |         |
| L5 Regression harness          |        | Yes      |         |
| L6 Optimization                |        | Yes      |         |
| L7 Research API                |        | Yes      |         |
| L8 Configuration               |        |          | Yes     |
| L9A Data contract              | Yes    | Yes      |         |
| L9B Data CLI                   |        |          | Yes     |
| L10 Execution realism          | Yes    | Yes      |         |
| L11 Session-aware scheduling   | Yes    | Yes      |         |

Rules that follow from the classification:

- Nothing in the research column may be imported by the kernel crates.
- Research features compose `BacktestNode` runs; they never modify engine semantics.
- Tooling may depend on both engine and research crates, and nothing may depend on tooling.

## 4. Anti-porting specification

This is the explicit map of what to copy, what to adapt, and what to reject. "Concept" means adopt
the semantics with a Nautilus-native implementation. "Reject" means the mechanism must not appear in
NautilusTrader.

| Lean                               | NautilusTrader target                              | Decision |
| ---------------------------------- | -------------------------------------------------- | -------- |
| `TimeSlice` batching               | Per-event callbacks                                | Reject   |
| `Insight`                          | New typed signal value                             | Concept  |
| `PortfolioTarget`                  | New typed target value                             | Concept  |
| Portfolio construction models      | New optional signal-to-target layer                | Concept  |
| Universe and security changes      | New universe component and membership              | Concept  |
| `Security` model slots             | Existing per-engine model handles                  | Reject   |
| Factor files                       | New corporate-action data and adjustment           | Concept  |
| Map files                          | Instrument identity mapping over time              | Concept  |
| Market hours database              | Calendar model loaded from data                    | Concept  |
| `LeanOptimizer`                    | External research subsystem                        | Concept  |
| `QuantBook`                        | Thin research API over catalog primitives          | Concept  |
| Regression framework               | Existing canonical result plus a declared registry | Adapt    |
| `.NET` configuration with comments | Typed configuration plus optional file input       | Reject   |
| Cloud platform and job queue       | Self-hosted tooling                                | Reject   |
| Runtime type-name resolution       | Typed factories                                    | Reject   |
| Mutable global metadata at runtime | Immutable inputs to a run                          | Reject   |

## 5. What Lean is

Verified from the Lean repository (all links point at `master`):

- **Runtime loop.** `Engine/Engine.cs` composes an engine from pluggable handlers, and
  `Engine/AlgorithmManager.cs` consumes an ordered `IEnumerable<TimeSlice>` produced by
  `ISynchronizer.StreamData`. A `TimeSlice` bundles every subscription's data due at the current
  time frontier plus security updates, consolidator inputs, universe data, and `SecurityChanges`;
  the user-facing `Slice` reaches `IAlgorithm.OnData` at most once per slice. See
  [ISynchronizer.cs](https://github.com/QuantConnect/Lean/blob/master/Engine/DataFeeds/ISynchronizer.cs),
  [TimeSlice.cs](https://github.com/QuantConnect/Lean/blob/master/Engine/DataFeeds/TimeSlice.cs).
- **Backtest and live share one engine.** Mode is determined by which handlers are composed:
  `FileSystemDataFeed` + `Synchronizer` + `BacktestingTransactionHandler` + `BacktestingBrokerage`
  for backtest, `LiveTradingDataFeed` + `LiveSynchronizer` + `BrokerageTransactionHandler` + a real
  `IBrokerage` for live. Handler types are resolved by type-name string from `Launcher/config.json`.
  See [IBrokerage.cs](https://github.com/QuantConnect/Lean/blob/master/Common/Interfaces/IBrokerage.cs).
- **Algorithm Framework.** A strategy decomposes into Universe Selection, Alpha, Portfolio
  Construction, Risk Management, and Execution models that communicate through `Insight`. See
  [QCAlgorithm.Framework.cs](https://github.com/QuantConnect/Lean/blob/master/Algorithm/QCAlgorithm.Framework.cs),
  [Insight.cs](https://github.com/QuantConnect/Lean/blob/master/Common/Algorithm/Framework/Alphas/Insight.cs).
- **Universe selection.** `Universe` and `UniverseSettings` drive dynamic symbol add and remove at
  runtime with coarse and fine fundamental chaining and `SecurityChanges` notifications; additions
  are deferred to the end of the time step. See
  [Universe.cs](https://github.com/QuantConnect/Lean/blob/master/Common/Data/UniverseSelection/Universe.cs),
  [QCAlgorithm.Universe.cs](https://github.com/QuantConnect/Lean/blob/master/Algorithm/QCAlgorithm.Universe.cs).
- **Scheduling.** `Schedule` combines date and time rules to fire session events, and
  `IRealTimeHandler` services them; `ScheduledUniverseSelectionModel` uses the same rule types. See
  [IRealTimeHandler.cs](https://github.com/QuantConnect/Lean/blob/master/Engine/RealTime/IRealTimeHandler.cs).
- **Reality modeling.** Fill, fee, slippage, buying-power, settlement, and margin-interest models
  are per-security slots. See
  [IFillModel.cs](https://github.com/QuantConnect/Lean/blob/master/Common/Orders/Fills/IFillModel.cs),
  [ISlippageModel.cs](https://github.com/QuantConnect/Lean/blob/master/Common/Orders/Slippage/ISlippageModel.cs),
  [IBuyingPowerModel.cs](https://github.com/QuantConnect/Lean/blob/master/Common/Securities/IBuyingPowerModel.cs).
- **Metadata as data.** Trading hours live in `Data/market-hours/market-hours-database.json` and
  contract properties in `Data/symbol-properties/symbol-properties-database.csv`, both mutable at
  runtime. See
  [MarketHoursDatabase.cs](https://github.com/QuantConnect/Lean/blob/master/Common/Securities/MarketHoursDatabase.cs).
- **Corporate actions and identity.** Prices are adjusted from factor files
  (`date,priceFactor,splitFactor,referencePrice`); ticker identity and delisting dates come from map
  files; `Split`, `Dividend`, `Delisting`, and `SymbolChangedEvent` are auxiliary data; normalization
  is per subscription. See
  [FactorFile.cs](https://github.com/QuantConnect/Lean/blob/master/Common/Data/Auxiliary/FactorFile.cs),
  [MapFile.cs](https://github.com/QuantConnect/Lean/blob/master/Common/Data/Auxiliary/MapFile.cs),
  [SubscriptionDataReader.cs](https://github.com/QuantConnect/Lean/blob/master/Engine/DataFeeds/SubscriptionDataReader.cs).
- **Optimization.** A host drives many backtests through a pluggable strategy (grid and Euler
  search) with objectives and constraints. See
  [LeanOptimizer.cs](https://github.com/QuantConnect/Lean/blob/master/Optimizer/LeanOptimizer.cs).
- **Research.** `QuantBook : QCAlgorithm` reuses the same security, slice, and history primitives in
  Jupyter. See [QuantBook.cs](https://github.com/QuantConnect/Lean/blob/master/Research/QuantBook.cs).
- **Regression suite.** Each scenario implements `IRegressionAlgorithmDefinition` with inline
  `ExpectedStatistics`, discovered by reflection; the runner resets global state, overrides handlers,
  and asserts the statistics. See
  [RegressionTests.cs](https://github.com/QuantConnect/Lean/blob/master/Tests/RegressionTests.cs).

## 6. Where NautilusTrader stands today

Verified in this repository:

- The kernel is single-threaded with a thread-local message bus, and components are
  `Rc<RefCell<_>>`; engines are wired through bus endpoints in
  `crates/common/src/msgbus/switchboard.rs`.
- Strategies receive one callback per data event (`DataActor` and `Strategy` in
  `crates/trading/src/strategy/mod.rs`), not a time-batched slice.
- Venue connectivity is trait-based (`DataClient` and `ExecutionClient` in
  `crates/common/src/clients/`), with 18 adapters under `crates/adapters/`.
- Backtest and live share strategy code and the execution stack
  ([../concepts/architecture.md](../concepts/architecture.md)).
- Values are typed `Price`, `Quantity`, and `Money` with exact arithmetic; time is nanosecond
  `UnixNanos`.
- Portfolio statistics are extensive: 35 implementations under `crates/analysis/src/statistics/`
  plus `PortfolioAnalyzer` and snapshot types in `crates/analysis/src/`.
- Deterministic result comparison already exists: `crates/backtest/src/result.rs` defines
  `CanonicalBacktestResult` with a versioned schema (`nautilus-backtest-result/v1`), identity
  normalization, `digest()`, and `first_divergence()`, and acceptance tests assert golden values,
  for example `test_backtest_cash_margin_account_order_fill_position_parity_golden` and
  `test_rerun_ema_cross_strategy_returns_identical_performance` in
  `python/tests/acceptance/test_backtest.py`.
- Execution realism is substantial, not absent: `crates/execution/src/models/` provides fee, fill,
  and latency models; the fill family includes `DefaultFillModel`, `BestPriceFillModel`,
  `OneTickSlippageFillModel`, `ProbabilisticFillModel`, `TwoTierFillModel`, `ThreeTierFillModel`,
  `LimitOrderPartialFillModel`, `SizeAwareFillModel`, `CompetitionAwareFillModel`,
  `VolumeSensitiveFillModel`, and `MarketHoursFillModel`; probabilistic fills take an explicit
  `random_seed` for determinism. `OrderMatchingEngineConfig` in
  `crates/execution/src/matching_engine/config.rs` exposes `liquidity_consumption`, `queue_position`,
  `bar_execution`, `trade_execution`, `price_protection_points`, and related flags. Margin models
  live in `crates/model/src/accounts/margin_model.rs`, and `MarketStatusAction::Halt` already exists
  in `crates/model/src/data/status.rs`.
- A generic user signal type exists (`Signal` in `crates/common/src/signal.rs`: name, value,
  timestamps), and option-chain aggregation exists (`crates/data/src/option_chains/`), but neither
  provides insight semantics or a portfolio-construction layer.
- Instruments carry `activation_ns` and `expiration_ns` but no session, holiday, or calendar data
  (`crates/model/src/instruments/`). `MarketHoursFillModel` is a probabilistic fill variant with a
  low-liquidity flag, not a session calendar. `crates/trading/src/sessions.rs` hard-codes four FX
  sessions in code.
- There is no universe selection model, no insight or alpha model, no portfolio-construction or
  target-reconciliation layer, no corporate-action or price-normalization handling, no trading
  calendar, and no parameter optimization. A repository search for those concepts returns nothing
  outside adapter-specific parsing.
- `crates/cli/src/opt.rs` exposes only `database` and `blockchain` subcommands.

## 7. Comparison

| Dimension              | QuantConnect Lean                    | NautilusTrader                       | Assessment                                  |
| ---------------------- | ------------------------------------ | ------------------------------------ | ------------------------------------------- |
| Delivery unit          | `TimeSlice` bundle per frontier      | One callback per data event          | Ours gives lower latency; theirs coherence  |
| Strategy decomposition | Five framework models plus `Insight` | `Strategy` plus `ExecutionAlgorithm` | Gap: no signal and target layers            |
| Universe               | Dynamic, scheduled, coarse and fine  | Static subscriptions per instrument  | Gap: no dynamic universe abstraction        |
| Scheduling             | Rule-based session events            | Clock timers                         | Gap: no session-aware scheduling            |
| Instrument models      | Per-security model slots             | Per-engine model handles             | Different split; ours is adequate           |
| Execution realism      | Fill, fee, slippage, margin slots    | Fill, fee, latency, queue, liquidity | Comparable; slippage is not a separate slot |
| Corporate actions      | Factor and map files, normalization  | None                                 | Correctness gap for equities                |
| Trading calendar       | Market-hours JSON as data            | Per-instrument activation and expiry | Gap: no venue calendar or holidays          |
| Optimization           | Engine component with strategies     | None                                 | Gap                                         |
| Research               | `QuantBook` on engine primitives     | Catalog plus analysis tearsheets     | Partial gap                                 |
| Regression tests       | Declared statistics, discovery       | Canonical result with digest         | Small workflow gap, high value              |
| Packaging              | One CLI plus Docker plus cloud       | Make targets, wheels, Docker         | Comparable for self-hosting                 |

## 8. Candidate learnings

Each item states the Lean mechanism, the demonstrated gap, the proposed Nautilus-native design, the
boundary (engine, research, or tooling), the tradeoffs, and the invariant it stresses.

### L1. Signal, target, and order as three distinct layers

- **Lean mechanism.** `Insight` carries the alpha view; portfolio construction converts insights
  into `PortfolioTarget` values; execution consumes targets.
- **Demonstrated gap.** Strategy code goes directly from a decision to `submit_order`. There is no
  typed representation of intent that one component can produce, another can modify, and a third can
  reconcile against current positions.
- **Proposed design.** Three distinct semantic layers, deliberately not Lean's object hierarchy:
  - `Signal`: direction, horizon, strength, source, expiry, provenance. A statement of view.
  - `Target`: instrument plus a target quantity, weight, or notional. A statement of desired
    exposure, after portfolio context and risk constraints.
  - `Order`: the existing order types. The only thing that reaches a venue.
  Inputs and their provenance are kept separate from the derived target, and the target is separate
  from the order. The pipeline is: data events, then strategy or alpha, then signal, then portfolio
  and risk context, then target construction, then target reconciliation, then existing risk checks,
  then orders.
- **Boundary.** Engine, opt-in per strategy.
- **Invariants stressed.** 6 (single source of truth) and 8 (direct-order compatibility). The
  reconciler must treat the cache and portfolio as authoritative; targets never become a second
  position store. Fields named confidence or score are deliberately avoided in `Signal` so that the
  type does not imply a probability it cannot guarantee.
- **Tradeoffs.** A new abstraction near the hot path. It must be inert when unused, and the
  reconciler must not fight the order emulator or `RiskEngine`.
- **Effort and impact.** High effort, high impact. Do this last and behind a feature.

### L2. Universe definition, selection, and membership

- **Lean mechanism.** `Universe`, `UniverseSettings`, `SecurityChanges`, deferred additions,
  `MinimumTimeInUniverse`.
- **Demonstrated gap.** Instruments are added explicitly before a run. `DataEngine` supports runtime
  subscribe and unsubscribe and `DataActor` has `on_instrument` and `on_instrument_status`, but
  there is no selection model, no membership lifecycle, and no scheduling.
- **Proposed design.** Three separate concepts with distinct ownership:
  - Universe definition: the rule and settings that describe what is eligible.
  - Universe selection: the scheduled evaluation that decides membership, driven by the clock.
  - Universe membership: explicit per-instrument state, `ADDED`, `ACTIVE`, `REMOVING`, `REMOVED`.
  Removal is a process, not an immediate unsubscribe: membership becomes `REMOVING`, open orders are
  cancelled or reconciled, the position policy is evaluated, the data subscription is released, and
  membership becomes `REMOVED`. Subscriptions go through the existing data command path, which
  already tracks ownership in `crates/data/src/subscription.rs`, so a departing instrument must
  release only its own claims.
- **Boundary.** Engine.
- **Invariants stressed.** 2 (ordering), 7 (determinism), and 9 (no cost when unused). Selection
  must be clock-driven, and an unused universe must add no per-event cost.
- **Tradeoffs.** Live universes need venue metadata that some adapters cannot supply; live
  membership timing must match backtest membership timing for parity.
- **Effort and impact.** High effort, high impact for equity workflows.

### L3. Corporate actions, instrument identity, and normalization semantics

- **Lean mechanism.** Factor files scale prices; map files carry identity, renames, and delisting
  dates; `Split`, `Dividend`, `Delisting`, and `SymbolChangedEvent` are auxiliary data; normalization
  is per subscription.
- **Demonstrated gap.** The catalog stores raw venue data with no adjustment step, instruments have
  no symbol history, and delisting has no representation. A backtest over an adjusted-price provider
  can silently mix adjusted history, raw execution prices, adjusted indicators, and raw portfolio
  accounting inside one strategy.
- **Proposed design.** Keep three representations explicitly distinct and never interchangeable:
  - Raw input: raw prices, raw volumes, and raw corporate actions, immutable.
  - Derived series: adjusted prices, adjusted OHLC, and total-return series, produced on demand.
  - Trading events: split, dividend, delisting, and symbol change, delivered as data so strategies
    can react.
  Auxiliary types are registered with the existing data-type macro so catalog paths, Arrow schemas,
  and bus topics follow the generated convention. Identity is modelled separately from price: a
  mapping from venue symbol to instrument id over time, resolved before data reaches the engines.
  Delisting is modelled as a terminal instrument status that reuses the existing `InstrumentClose`
  path. The engine records which representation a run consumed.
- **Boundary.** Engine and research. Data contract for the files (L9A).
- **Invariants stressed.** 5 (exact arithmetic, raw semantics preserved) and 7 (determinism). Raw
  data is never mutated; adjustment is opt-in and recorded.
- **Tradeoffs.** Adjustment requires a licensing story for factor data and is not meaningful for
  crypto and FX, so it must be opt-in and asset-class aware.
- **Effort and impact.** High effort, high impact. This is a correctness item for equities, not a
  convenience.

### L4. Trading calendars as data

- **Lean mechanism.** `market-hours-database.json` holds time zones, session segments, and holidays;
  the engine loads it and permits runtime overrides.
- **Demonstrated gap.** Instruments carry `activation_ns` and `expiration_ns`, and
  `crates/trading/src/sessions.rs` hard-codes four FX sessions. There is no venue calendar, no
  holiday handling, and no session-aware scheduling primitive. `MarketHoursFillModel` names market
  hours but only models a low-liquidity flag.
- **Proposed design.** A calendar model in `crates/model` (sessions, early closes, holidays, time
  zone) loaded from a versioned data file with an override path. The calendar is an input to a run,
  never code, and never mutated at runtime.
- **Boundary.** Engine and research.
- **Invariants stressed.** 4 (typed, data-driven), 7 (determinism), and 9 (no cost when unused).
- **Impact.** High for equities, not medium: the calendar underpins session gating, daily risk
  resets, premarket and postmarket logic, half-days, expirations, and scheduled events (L11).
- **Tradeoffs.** An external dataset that goes stale; must be versioned, validated, and warn when
  coverage ends before the run end.
- **Effort and impact.** Medium effort, high impact for equities. Prerequisite for L2 and L11.

### L5. Regression scenarios as verification infrastructure

- **Lean mechanism.** Scenario-declared `ExpectedStatistics`, discovery, hard reset of global state,
  regression-safe handlers, and in-place regeneration.
- **Demonstrated gap.** The deterministic machinery already exists, but expectations are written by
  hand per test, scenarios are not enumerated, no single command regenerates expectations, and there
  is no documented reset between scenarios.
- **Proposed design.** Adapt rather than copy: reuse `CanonicalBacktestResult` as the golden artifact.
  Each scenario declares expected statistics plus an expected digest; a registry enumerates scenarios;
  a regeneration mode rewrites expectations so updates are a reviewable diff; the harness resets
  engine, cache, logger, and clock state between scenarios; failures report
  `first_divergence().path`.
  Additionally, make this the verification infrastructure for the whole programme: every later
  workstream must add at least one golden scenario, for example a deterministic membership scenario
  for L2, a split and dividend scenario for L3, a holiday and early-close scenario for L4, a
  reproducible optimization scenario for L6, and a target-to-order parity scenario for L1.
- **Boundary.** Research.
- **Invariants stressed.** 7 (determinism) and 9 (unused features cost nothing, which a golden
  scenario proves by remaining byte-identical).
- **Tradeoffs.** Digests are brittle by design; regeneration must be explicit and reviewed.
- **Effort and impact.** Low effort, high impact. Do this first.

### L6. Optimization as an external research subsystem

- **Lean mechanism.** A host drives many backtests through a pluggable search strategy with
  objectives and constraints.
- **Demonstrated gap.** Users write their own sweeps; there is no objective or constraint
  abstraction and no CLI surface.
- **Proposed design.** An explicit pipeline of parameter space, search strategy, backtest, objective,
  constraint, and result, with process-level fan-out so the single-threaded kernel and the Python GIL
  are not constraints. Results are canonical backtest results plus the parameter set, comparable by
  digest. The subsystem also exposes the methodology boundary around it: train, optimize, validate,
  out-of-sample, and walk-forward are distinct stages, not one loop over a grid.
- **Boundary.** Research. Never the kernel.
- **Invariants stressed.** 7 (determinism) and 8 (a strategy that does not use it is unaffected).
- **Hard boundary statement.** Optimization must never alter the deterministic semantics of an
  individual backtest. It composes runs; it does not reach into them. Without this rule the project
  builds a brute-force parameter engine rather than a research subsystem.
- **Tradeoffs.** Process fan-out multiplies memory and data-loading cost; document a concurrency
  limit.
- **Effort and impact.** Medium effort, high impact for research workflows.

### L7. Research API over shared primitives

- **Lean mechanism.** `QuantBook` reuses the security, slice, and history primitives.
- **Demonstrated gap.** Research is catalog query plus wranglers plus pandas, duplicating instrument
  loading and time semantics outside the engine.
- **Proposed design.** A thin research API over the catalog, instrument, and historical-data
  primitives. The lesson from QuantBook is that research should use the same semantic primitives as
  backtesting, not that a research engine should exist. Do not boot an engine in a query-only mode;
  expose primitives and let the backtest path remain the only runtime.
- **Boundary.** Research.
- **Invariants stressed.** 3 (parity of semantics) and 8 (no new runtime mode).
- **Tradeoffs.** Risk of a divergent second data path; the API must call the same catalog and
  wrangler code as `BacktestNode`.
- **Effort and impact.** Low to medium effort, medium impact.

### L8. Configuration with explicit precedence

- **Lean mechanism.** One `lean.json` with an `environment` section layered over top-level keys,
  consumed identically by local CLI, backtest, live, and cloud.
- **Demonstrated gap.** Configuration is typed and well validated, but there is no file-based input
  with environment layering, and no documented precedence order.
- **Proposed design.** Optional file input mapping onto existing typed configs, with an explicit and
  tested precedence chain: built-in defaults, then config file, then environment profile, then
  environment variables, then CLI overrides. The format is secondary; the precedence order is the
  design. Unknown keys fail validation, consistent with the `deny_unknown_fields` convention in
  adapter configs.
- **Boundary.** Tooling.
- **Invariants stressed.** 4 (typed configs remain authoritative).
- **Tradeoffs.** Two configuration surfaces can drift; validate the file schema against the typed
  configs in a test.
- **Effort and impact.** Low effort, medium impact.

### L9A. Data contract

- **Lean mechanism.** A documented data folder contract, sidecar factor and map files, and a
  documented directory pattern per resolution.
- **Demonstrated gap.** The catalog layout is documented
  ([../concepts/data/catalog.md](../concepts/data/catalog.md)) but there is no contract for sidecar
  metadata, no symbol map, and no calendar or corporate-action file format.
- **Proposed design.** A versioned, documented contract for catalog layout, auxiliary metadata,
  symbol maps, corporate actions, and calendar data, with colocated readme files. This is
  architectural and depends on L3 and L4.
- **Boundary.** Engine and research.
- **Invariants stressed.** 7 (determinism through a stable on-disk contract).
- **Effort and impact.** Medium effort, medium impact.

### L9B. Data CLI

- **Lean mechanism.** `lean init`, `lean data download`, and `lean data generate`.
- **Demonstrated gap.** Scripts exist, but there is no CLI for data acquisition, validation,
  conversion, or inspection.
- **Proposed design.** Data subcommands in `nautilus-cli` over the catalog and existing loaders:
  download, validate, convert, generate, and inspect. Keep the surface small and explicit because
  data acquisition is licensing-sensitive.
- **Boundary.** Tooling.
- **Invariants stressed.** None directly; it must not become a data path inside a run.
- **Effort and impact.** Low effort, medium impact. Depends on L9A.

### L10. Execution realism as an explicit, extensible model set

- **Lean mechanism.** Per-security fill, fee, slippage, buying-power, settlement, and margin-interest
  model slots, with slippage as its own abstraction.
- **Demonstrated gap.** NautilusTrader is not missing realism, but its realism is unevenly exposed
  and not composable per instrument:
  - What exists: `crates/execution/src/models/{fee,fill,latency}.rs`, eleven fill-model variants,
    seeded probabilistic fills, `liquidity_consumption` and `queue_position` in
    `OrderMatchingEngineConfig`, margin models in `crates/model/src/accounts/margin_model.rs`, and
    `MarketStatusAction::Halt`.
  - What is missing or implicit: slippage is folded into specific fill models rather than being a
    separate, composable concern; market impact is not modelled beyond book consumption; borrow and
    locate availability for short selling is only `allow_borrowing` on cash accounts; auction and
    halt behaviour is represented by market status without an explicit execution policy; and the
    matrix of which model applies to which instrument and venue is not documented in one place.
- **Proposed design.** Treat realism as a model set with defined interfaces over the existing
  execution simulator, not as a Lean port:
  - Inventory and document the existing matrix of fee, fill, latency, queue, and liquidity
    behaviour per venue and instrument class.
  - Introduce an explicit slippage concern if it can be separated from fill without breaking the
    eleven existing variants and their golden results.
  - Decide whether model selection must become per instrument rather than per matching engine;
    Lean's per-security slots are the pressure that exposes this, but the decision is ours.
  - Additions considered, each independently optional: market impact, spread, partial-fill policy,
    borrow and locate availability, auction and halt policy.
- **Boundary.** Engine and research. Models are set per instrument or venue and used by both
  backtest and sandbox execution.
- **Invariants stressed.** 3 (parity, since sandbox and backtest share the matching engine), 5
  (exact arithmetic in fees and fills), 7 (determinism, already supported by seeded probabilistic
  fills), and 9 (unused models cost nothing).
- **Why it precedes optimization.** Optimizing against an unrealistic fill model produces
  precisely optimized nonsense. Realism must be defined before a search is meaningful.
- **Tradeoffs.** More models mean more configuration surface and more golden scenarios; keep each
  addition independent and opt-in.
- **Effort and impact.** Medium effort, high impact for day trading.

### L11. Session-aware scheduled events

- **Lean mechanism.** `Schedule` with date and time rules, serviced by `IRealTimeHandler`, used for
  algorithm events and universe refresh.
- **Demonstrated gap.** NautilusTrader has clock timers (`crates/common/src/timer.rs`, the `Clock`
  trait, and `on_time_event`) but no notion of a trading-session event. The two concepts are
  different: a clock timer is an interval; a session event is anchored to the market calendar.
- **Proposed design.** A scheduling layer over the L4 calendar that distinguishes:
  - Session events: premarket, open, opening-range complete, midday, pre-close, close, early close.
  - Calendar events: holidays, expiration dates, and user-supplied timestamps such as economic
    releases.
  Events are delivered through the existing timer and callback machinery, so they are ordered and
  deterministic, and they are scheduled in exchange local time and converted to UTC at schedule
  time. Session-aware scheduling is a first-class primitive for intraday work, not an extra of the
  calendar.
- **Boundary.** Engine and research.
- **Invariants stressed.** 1 (events, not barriers), 2 (ordering), 7 (determinism).
- **Tradeoffs.** Scheduling correctness depends on L4 data quality; early closes and half-days are
  the classic failure mode.
- **Effort and impact.** Medium effort, high impact for intraday strategies. Depends on L4.

## 9. What not to copy

- **`TimeSlice` batching as the primary delivery model.** It guarantees coherence but adds latency
  and couples all subscriptions to one frontier. NautilusTrader's per-event callbacks and nanosecond
  ordering are a deliberate advantage. Any slice-like view is an optional convenience at most.
- **Handler resolution by type-name string.** Composing behaviour from untyped strings in a config
  file trades compile-time safety for flexibility. Typed factories are preferable.
- **Mutable global metadata at runtime.** For a deterministic engine, calendars and instrument
  metadata are immutable inputs to a run.
- **Configuration with comments and implicit layering.** If file configuration is added (L8),
  precedence must be explicit and tested.
- **A cloud-coupled optimizer.** Optimization stays out of the kernel and out of the runtime path.
- **`Security` model slots as a replacement for engine-owned models.** The existing per-engine
  handles work; per-instrument selection is a decision to make deliberately under L10, not by
  transplanting Lean's object hierarchy.
- **Porting Lean's object hierarchy.** Adopt the semantics of separation, not the class graph.
- **Per-strategy bespoke expectations.** L5 replaces ad-hoc expectations with one harness rather
  than adding a second convention.

## 10. Recommended order

The order follows the dependency graph rather than feature attractiveness. Equity identity and
market time precede optimization because optimizing on incorrect or unmodelled market state is not
meaningful.

| Phase | Workstreams | Contents                                                                                             | Rationale                                                |
| ----- | ----------- | ---------------------------------------------------------------------------------------------------- | -------------------------------------------------------- |
| 0     | Contract    | Invariants, boundaries, determinism rules, raw versus derived semantics, signal and target ownership | Makes step 2 enforceable instead of aspirational         |
| 1     | L5          | Regression scenarios, golden artifacts, digest, divergence, reset, regeneration                      | Verification foundation for every later phase            |
| 2     | L4, L11     | Calendar, time zone, sessions, holidays, early closes, session-aware scheduling                      | Market-time foundation; unblocks L2 and equity workflows |
| 3     | L3, L9A     | Corporate actions, symbol mapping, delisting, normalization, data contract                           | Equity identity and data correctness                     |
| 4     | L2          | Universe definition, selection, membership lifecycle, subscription lifecycle                         | Depends on market time and identity                      |
| 5     | L10         | Realism inventory, slippage separation, per-instrument model decision, optional model additions      | Must precede optimization                                |
| 6     | L1          | Signal, target construction, target reconciliation                                                   | Deepest engine change; only after identity and realism   |
| 7     | L6, L7      | Optimizer, research API, walk-forward tooling                                                        | Research over a correct and realistic engine             |
| 8     | L8, L9B     | Configuration precedence, data CLI                                                                   | Developer ergonomics once semantics are settled          |

```text
             +-------------------+
             |   Phase 0         |
             |   Contract        |
             +---------+---------+
                       |
                       v
             +-------------------+
             |   Phase 1  L5     |
             |   Verification    |
             +---------+---------+
                       |
                       v
             +-------------------+
             |   Phase 2  L4/L11 |
             |   Market time     |
             +---------+---------+
                       |
                       v
             +-------------------+
             |   Phase 3  L3/L9A |
             |   Equity identity |
             +---------+---------+
                       |
                       v
             +-------------------+
             |   Phase 4  L2     |
             |   Universe        |
             +---------+---------+
                       |
                       v
             +-------------------+
             |   Phase 5  L10    |
             |   Realism         |
             +---------+---------+
                       |
                       v
             +-------------------+
             |   Phase 6  L1     |
             |   Signal/Target   |
             +---------+---------+
                       |
             +---------+---------+
             |                   |
             v                   v
       +-----------+      +-----------+
       | Phase 7 L6|      | Phase 7 L7|
       | Optimize  |      | Research  |
       +-----+-----+      +-----+-----+
             |                  |
             +--------+---------+
                      v
             +-------------------+
             |   Phase 8  L8/L9B |
             |   Ergonomics      |
             +-------------------+
```

## 11. Target architecture

Engine side:

```text
                        Market data
                             |
                    +--------v--------+
                    | Calendar and    |   L4
                    | session context |   L11
                    +--------+--------+
                             |
                    +--------v--------+
                    | Universe and    |   L2
                    | membership      |
                    +--------+--------+
                             |
                    +--------v--------+
                    | Strategy/alpha  |   L1
                    | produces Signal |
                    +--------+--------+
                             |
                        +----v-----+
                        |  Signal  |
                        +----+-----+
                             |
             +---------------+---------------+
             |                               |
      Portfolio context                Risk context
             |                               |
             +---------------+---------------+
                             v
                    +-----------------+
                    | Target          |   L1
                    | construction    |
                    +--------+--------+
                             v
                    +-----------------+
                    | Target          |
                    | reconciliation  |
                    +--------+--------+
                             v
                       Order intent
                             v
                    +-----------------+
                    | Nautilus risk   |
                    | and execution   |
                    +--------+--------+
                             v
                    +-----------------+
                    | Execution model |   L10
                    | fill slippage   |
                    | fees latency    |
                    | queue liquidity |
                    +--------+--------+
                             v
                         Portfolio
```

Research side, deliberately outside the kernel:

```text
             +-------------------------------+
             |   Research subsystem          |
             |   L5 regression               |
             |   L6 optimization             |
             |   L7 research API             |
             +---------------+---------------+
                             |
                        BacktestNode
                             |
                             v
                     Nautilus engine
```

Corporate actions and identity (L3) sit underneath the data layer, not inside the strategy layer,
because they determine what the market data means before any strategy sees it.

## 12. Open questions

- Does the project want a portfolio-construction layer at all, or is direct order submission with
  `ExecutionAlgorithm` sufficient? This determines whether L1 is worth its risk.
- Which asset classes justify L3? If the roadmap stays crypto, FX, and derivatives heavy, L3 is
  lower value than it appears for equities.
- Should execution models be selected per instrument rather than per matching engine (L10), and does
  that change the eleven existing fill variants' configuration surface?
- Can slippage be separated from fill without changing existing golden results (L10)?
- Where should optimization live: a Rust CLI command, the Python layer, or both (L6)?
- Is a file-based configuration format desired upstream, given the typed-constructor convention (L8)?
- For L5, is a digest plus declared statistics stable enough given intentional simulation changes,
  or should the golden also pin a small explicit event sample?
