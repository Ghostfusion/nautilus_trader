# Lean lesson review: design considerations

This document records what NautilusTrader can learn from QuantConnect Lean. It is a design
document: it states the problem, the candidate designs, the boundaries, and the tradeoffs. It is
not a commitment to implement anything, and it does not change the architecture of the engine.

The companion implementation plan is [lean_lessons_implementation.md](lean_lessons_implementation.md).

Revision note: the first revision added the architectural invariants, the anti-porting
specification, the engine/research/tooling classification, and workstreams L10 (execution realism)
and L11 (session-aware scheduled events), and reordered the roadmap so equity identity and market
time precede optimization. The second revision converted the open questions into binding decisions
D12 to D18, reduced the remaining uncertainty to one open question, the compatibility target
(section 13), strengthened the central contract, and recast L1 as an optional pipeline, L3 as an
asset-class-scoped capability, L5 as a three-layer oracle, L6 as Python orchestration over Rust
execution, L8 as optional convenience, and L10 as a two-stage migration. The third revision adds
D19 (compatibility target: B primary, C secondary, A out of scope), the compatibility hierarchy and
capability test in section 4, and L1 as optional target construction. There are no remaining open
questions.

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

### The central contract

> Extract selected, Lean-proven abstractions and research capabilities while preserving
> NautilusTrader's event-driven runtime, typed domain model, deterministic simulation, and
> backtest/live execution parity.
>
> Borrow semantics from Lean, not implementation mechanisms.
>
> A Lean mechanism may be adopted only when its semantics close a demonstrated NautilusTrader
> capability gap without weakening NautilusTrader's event-driven, deterministic, typed, low-latency
> architecture.

Three consequences follow, and they are binding. They are formalised as D19 in section 12.

1. **Capability parity is the primary target, not programming-model compatibility.** The project
   adopts capabilities Lean has demonstrated (dynamic universes, portfolio targets, corporate
   actions, calendars, optimization, research APIs, regression, and reality modeling) and implements
   them with NautilusTrader's event-driven, typed, deterministic architecture. It is explicitly not
   an `Insight`/`AlphaModel`/`RiskModel`-shaped user-facing API that mirrors Lean.
2. **Research-workflow parity is secondary and selective.** Notebook research, parameter
   optimization, walk-forward analysis, regression testing, and data tooling are supported where they
   materially improve the workflow, and they consume Nautilus-native APIs rather than reproducing
   Lean's programming model.
3. **Both execution paths are first-class.** Lean's framework abstraction and NautilusTrader's
   existing direct-execution model are not in competition. The project supports both; it does not
   choose.

The governing rule is: **adopt Lean semantics and proven capabilities, not Lean APIs or
implementation mechanisms.** Nautilus is the architecture; Lean is the source of proven ideas. This
is the reason some Lean capabilities are deliberately rejected in section 9, and it is why "Lean has
X and we do not" is not by itself a reason to build X. Section 4 gives the decision procedure.

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
| L1 Target construction         | Yes    |          |         |
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

### Compatibility hierarchy

| Level                | Goal                                                      | Decision       |
| -------------------- | --------------------------------------------------------- | -------------- |
| A, API compatibility | Lean users can port code with minimal changes             | Not a goal     |
| B, capability        | NautilusTrader provides comparable important capabilities | Primary goal   |
| C, workflow          | Users can perform comparable research workflows           | Secondary goal |

B is normative. C is supportive. A is incidental. This is decided by D19 in section 12.

### Capability test

A Lean-shaped abstraction is implemented only when all of the following hold:

- It closes a demonstrated capability gap.
- NautilusTrader does not already provide an equivalent.
- It can be implemented without weakening the invariants in section 2.

API resemblance is not a justification. "Adopt semantics, not APIs" is the working rule.

```text
Does Lean have it?
        |
        v
Does it provide a useful capability?
        |
       no ----> do not implement
       yes
        v
Does NautilusTrader already provide it?
        |
       yes ---> reuse or extend the NautilusTrader abstraction
       no
        v
Can the capability fit NautilusTrader semantics?
        |
       no ----> do not implement
       yes
        v
Implement a NautilusTrader-native equivalent
```

### Classification by item

| Feature                                                             | B, capability | C, workflow | A, API |
| ------------------------------------------------------------------- | ------------- | ----------- | ------ |
| L1 Signal and target construction                                   | Yes           | Yes         | No     |
| L2 Dynamic universe and membership                                  | Yes           | Yes         | No     |
| L3 Corporate actions and instrument identity                        | Yes           | Yes         | No     |
| L4 Trading calendar                                                 | Yes           | Yes         | No     |
| L5 Regression                                                       | Yes           | Yes         | No     |
| L6 Optimization                                                     | Yes           | Yes         | No     |
| L10 Execution realism                                               | Yes           | Yes         | No     |
| L11 Session-aware scheduling                                        | Yes           | Yes         | No     |
| L7 Research API                                                     |               | Yes         | No     |
| L8 Configuration serialization                                      |               | Yes         | No     |
| L9B Data CLI                                                        |               | Yes         | No     |
| Lean-named types (`Insight`, `AlphaModel`, `PortfolioTarget`, etc.) |               |             | No     |
| Lean API compatibility                                              |               |             | No     |

The last two rows carry the point. `Insight` is not a feature; a signal abstraction is.
`AlphaModel` is not a feature; composable alpha sources are. `PortfolioTarget` semantics are
valuable; the exact Lean class is not.

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

Each item states the Lean mechanism, the demonstrated gap, the Nautilus-native design, the boundary
(engine, research, or tooling), the invariant it stresses, and where applicable the decision that
settles it.

### L1. Optional target construction

- **Lean mechanism.** `Insight` carries the alpha view; portfolio construction converts insights
  into `PortfolioTarget` values; execution consumes targets. Portfolio construction is optional in
  Lean itself: `NullPortfolioConstructionModel` is the default model, returns no targets, and exists
  specifically to bypass the stage, for example to analyse an alpha model in isolation
  ([supported models](https://www.quantconnect.com/docs/v2/writing-algorithms/algorithm-framework/portfolio-construction/supported-models)).
- **Demonstrated gap.** Strategy code goes directly from a decision to `submit_order`. There is no
  typed representation of intent that one component can produce, another can modify, and a third can
  reconcile against current positions, so multi-asset allocation and strategy composition must be
  hand-rolled.
- **Decision (D12, D19).** Adopt optional target construction. Direct order submission through
  `ExecutionAlgorithm` remains a first-class path. This is the NautilusTrader-native semantic
  equivalent of Lean's signal-to-target flow, not a reproduction of Lean's Algorithm Framework
  under a compatibility requirement.
- **Proposed design.** Two supported paths, not a mandatory hierarchy:
  - Direct path: strategy, then `ExecutionAlgorithm`, then orders. Unchanged.
  - Framework path: strategy or alpha, then `Signal`, then portfolio context and risk context, then
    target construction, then target reconciliation, then `ExecutionAlgorithm`, then orders.
  Three distinct semantic objects:
  - `Signal`: direction, horizon, strength, source, expiry, provenance. A statement of view. It is
    not a trading command, and it does not know an order quantity.
  - `Target`: instrument plus a target quantity, weight, or notional. A statement of desired
    exposure, after portfolio and risk context.
  - `Order`: the existing order types. The only thing that reaches a venue.
  Do not create a mandatory `AlphaModel`, `PortfolioConstructionModel`, `RiskManagementModel`, and
  `ExecutionModel` hierarchy. Use optional Nautilus-native interfaces. Because both paths converge
  on `ExecutionAlgorithm`, the layer is additive rather than invasive.
- **Boundary.** Engine, opt-in per strategy.
- **Invariants stressed.** 6 (single source of truth) and 8 (direct-order compatibility). The
  reconciler must treat the cache and portfolio as authoritative; targets never become a second
  position store. Fields named confidence or score are deliberately avoided in `Signal` so that the
  type does not imply a probability it cannot guarantee.
- **Justification.** Multi-asset allocation, strategy composition, and research workflows. If those
  are not goals, the layer is not needed.
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

### L3. Instrument identity, corporate actions, and historical normalization

- **Lean mechanism.** Factor files scale prices; map files carry identity, renames, and delisting
  dates; `Split`, `Dividend`, `Delisting`, and `SymbolChangedEvent` are auxiliary data; normalization
  is per subscription.
- **Demonstrated gap.** The catalog stores raw venue data with no adjustment step, instruments have
  no symbol history, and delisting has no representation. A backtest over an adjusted-price provider
  can silently mix adjusted history, raw execution prices, adjusted indicators, and raw portfolio
  accounting inside one strategy.
- **Decision (D13).** Implement L3 as an asset-class-scoped
  identity, corporate-action, and normalization capability, not a universal engine concern.
- **Scope.** The name matters, because the problem is larger than splits and dividends:
  instrument identity, symbol mapping, corporate actions, historical normalization, and
  delisting or survivorship handling. It is required for equity-capable research and trading and
  must not impose equity-specific runtime behaviour on crypto, FX, or derivatives. Structure it as a
  capability keyed by instrument class, where unsupported classes are an explicit no-op rather than
  a silent default. Crypto and FX strategies must not pay for equity machinery, in configuration or
  in the event path.
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
- **Tradeoffs.** Adjustment requires a licensing story for factor data. The capability must be
  class-scoped so it is inert for assets where it does not apply.
- **Effort and impact.** High effort, high impact for equity-capable workflows. This is a
  correctness item for equities, not a convenience.

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
- **Decision (D18).** Regression validation uses three complementary layers: canonical digest,
  declared statistics, and deterministic semantic checkpoints. A single digest is too opaque to
  diagnose; a full event-stream snapshot is too brittle to maintain.
- **Proposed design.** Adapt rather than copy: reuse `CanonicalBacktestResult` as the golden artifact
  and add two layers around it.
  - Layer 1, canonical digest: detects that something changed at all. Cheap enough for CI.
  - Layer 2, declared statistics: orders, fills, positions, PnL, fees, slippage, maximum drawdown,
    and final equity. Explains what changed in aggregate.
  - Layer 3, semantic checkpoints: named events identified by kind, instrument, and occurrence
    ordinal, for example the first `ORDER_FILLED` on an instrument, the first `POSITION_OPENED`, and
    the first `POSITION_CLOSED`. Locates where behaviour changed without pinning the whole stream.
  Checkpoints are semantic rather than positional, so they survive unrelated event insertions:

  ```yaml
  checkpoints:
    - event: ORDER_FILLED
      instrument: AAPL
      occurrence: 1
    - event: POSITION_OPENED
      instrument: AAPL
      occurrence: 1
  ```

  A failure reports the layer that failed, the expected and actual values, and then
  `first_divergence().path`, so the output names the value and the divergence instead of only
  "digest mismatch". A registry enumerates scenarios; a regeneration mode rewrites expectations so
  updates are a reviewable diff; the harness resets engine, cache, logger, and clock state between
  scenarios.
  This is the safety harness around every other workstream: L5 validates L3, L4, L10, L1, and L2.
  Every later workstream adds at least one scenario, for example a split and dividend scenario for
  L3, a holiday and early-close scenario for L4, a model-parity scenario for L10, a deterministic
  membership scenario for L2, a target-to-order parity scenario for L1, and a reproducible
  optimization scenario for L6.
- **Boundary.** Research.
- **Invariants stressed.** 7 (determinism) and 9 (unused features cost nothing, which a golden
  scenario proves by remaining byte-identical).
- **Tradeoffs.** Three layers mean three places to update on intentional change, so regeneration must
  cover all of them in one command. Digests remain brittle by design.
- **Effort and impact.** Low effort, high impact. Do this first.

### L6. Optimization as an external research subsystem

- **Lean mechanism.** A host drives many backtests through a pluggable search strategy with
  objectives and constraints.
- **Demonstrated gap.** Users write their own sweeps; there is no objective or constraint
  abstraction and no CLI surface.
- **Decision (D16).** Optimization is a research and orchestration capability implemented primarily
  in Python, while backtest execution remains in the Rust engine. A CLI front end invokes the same
  Python API; there is no second optimizer implementation and no second semantic model.
- **Proposed design.** Split responsibilities by capability, which matches the project's existing
  control-plane split:
  - Rust owns backtest execution, simulation, event processing, deterministic calculations, and
    result production.
  - Python owns parameter spaces, experiment generation, search algorithms, result aggregation,
    walk-forward experiments, and experiment persistence.
  - The CLI is thin: `nautilus optimize config.yaml` calls the Python optimization API.
  The pipeline is parameter space, search strategy, backtest, objective, constraint, and result, with
  process-level fan-out so the single-threaded kernel and the Python GIL are not constraints.
  Results are canonical backtest results plus the parameter set, comparable by digest. The
  methodology boundary is explicit: train, optimize, validate, out-of-sample, and walk-forward are
  distinct stages, not one loop over a grid.
- **Boundary.** Research. Never the kernel.
- **Invariants stressed.** 7 (determinism) and 8 (a strategy that does not use it is unaffected).
- **Hard boundary statement.** Optimization must never alter the deterministic semantics of an
  individual backtest. It composes runs; it does not reach into them. Without this rule the project
  builds a brute-force parameter engine rather than a research subsystem.
- **Tradeoffs.** Process fan-out multiplies memory and data-loading cost; document a concurrency
  limit. A Python-only orchestration layer must not become a second execution path for backtests.
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

### L8. Configuration serialization as optional convenience

- **Lean mechanism.** One `lean.json` with an `environment` section layered over top-level keys,
  consumed identically by local CLI, backtest, live, and cloud.
- **Demonstrated gap.** Configuration is typed and well validated; there is no optional serialization
  or file-loading path.
- **Decision (D17).** Typed constructors remain the canonical configuration API. File-based
  configuration is optional serialization and loading functionality, not part of the core
  architecture. YAML or JSON must not become a new source of configuration semantics.
- **Proposed design.** If implemented at all, the typed config object stays canonical and
  serialization is a view of it: Python constructor first, then optional JSON or YAML serialization,
  then optional CLI loading. Any layering belongs to the loader and is limited to defaults, file, and
  explicit overrides, with unknown keys rejected. The deliverable is operator and CI convenience,
  not a new configuration model.
- **Boundary.** Tooling.
- **Invariants stressed.** 4 (typed configs remain authoritative).
- **Downgrade.** This is developer and operator convenience rather than a Lean-inspired capability.
  It stays at the end of the roadmap and is a prerequisite for nothing else.
- **Tradeoffs.** A second input surface can drift from the typed configs. Validate the schema against
  the typed configs in a test, or omit the feature.
- **Effort and impact.** Low effort, low impact.

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
- **Decisions (D14, D15).** Execution configuration supports instrument-level overrides while
  retaining global, venue, and matching-engine defaults. Fill, slippage, and fee become conceptually
  separate interfaces, but existing composite fill behaviour is preserved in the first
  implementation.
- **Proposed design.** Treat realism as a model set with defined interfaces over the existing
  execution simulator, not as a Lean port, and migrate in two stages:
  - Stage A, abstraction without behaviour change: introduce a configuration layer around the
    existing models and prove the default path is unchanged, digest for digest. The eleven existing
    fill variants are wrapped, not rewritten, and migrate incrementally through a compatibility
    adapter.
  - Stage B, independent components: make fill, slippage, and fee independently configurable, with
    semantics ordered as fill eligibility, then fill quantity, then base fill price, then slippage
    adjustment, then final fill price, then fees.
  Model selection follows an inheritance chain so the surface stays manageable: global defaults, then
  venue defaults, then matching-engine defaults, then instrument overrides, then order-specific
  overrides. Each level overrides only what it changes; configuration is not duplicated at every
  level.
  Also inventory and document the existing matrix of fee, fill, latency, queue, and liquidity
  behaviour per venue and instrument class. Additions are considered one at a time and are
  independently opt-in: market impact, spread, partial-fill policy, borrow and locate availability
  for short selling, and auction and halt policy.
- **Boundary.** Engine and research. Models are used by both backtest and sandbox execution.
- **Invariants stressed.** 3 (parity, since sandbox and backtest share the matching engine), 5
  (exact arithmetic in fees and fills), 7 (determinism, already supported by seeded probabilistic
  fills), and 9 (unused models cost nothing).
- **Workstream acceptance criterion.** Separating an abstraction must not automatically change
  simulation semantics. Stage A is complete only when `old_digest == new_digest` for every existing
  golden scenario, and Stage B is complete only when independently configured slippage reproduces
  the composite behaviour it replaces.
- **Why it precedes optimization.** Optimizing against an unrealistic fill model produces precisely
  optimized nonsense. Realism must be defined before a search is meaningful.
- **Tradeoffs.** More models mean more configuration surface and more golden scenarios. Keep each
  addition independent and opt-in, and never migrate more than one concern at a time.
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
- **A Lean compatibility layer.** Lean-shaped types and lifecycles (`Insight`, `AlphaModel`,
  `IPortfolioConstructionModel`, `IExecutionModel`, `IRiskManagementModel`) are not adopted for
  naming or API compatibility. Doing so would accumulate compatibility surface without adding
  trading capability. Each of these is allowed only where its semantics pass the capability test in
  section 4, and then under a NautilusTrader-native name and lifecycle.

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

### Layers

The L1 pipeline is optional. Both paths converge on `ExecutionAlgorithm` and then orders, so the
framework path adds a route rather than replacing the existing one.

| Layer        | Content                              | Decision                                     |
| ------------ | ------------------------------------ | -------------------------------------------- |
| Runtime      | Existing event engine                | Preserve                                     |
| Runtime      | L10 execution realism                | Add incrementally, Stage A then Stage B      |
| Portfolio    | L1 signal to target                  | Optional, convergent on `ExecutionAlgorithm` |
| Universe     | L2 universe lifecycle                | Add                                          |
| Instrument   | L3 identity and corporate actions    | Add for supported instrument classes         |
| Time         | L4 calendar and sessions, L11 events | Add                                          |
| Verification | L5 regression                        | Add first; harness for every other layer     |
| Research     | L6 optimization, L7 research API     | Python orchestration over Rust execution     |
| Developer    | L8 configuration serialization       | Optional, low priority                       |
| Data tooling | L9A contract, L9B CLI                | Contract with L3 and L4; CLI later           |

### Dependencies

```text
                  +-----------------------+
                  | Nautilus core runtime |
                  +-----------+-----------+
                              |
           +------------------+------------------+
           |                  |                  |
           v                  v                  v
       Calendar          Instrument          Execution
        L4/L11            identity L3           L10
           |                  |                  |
           +--------+---------+------------------+
                    |
                    v
              Universe L2
                    |
                    v
        Signal -> Target L1 (optional)
                    |
                    v
           ExecutionAlgorithm (direct path also enters here)
                    |
                    v
                  Orders
```

```text
L5 regression
      |
      +-- validates L4 and L11
      +-- validates L3
      +-- validates L10
      +-- validates L2
      +-- validates L1

L6 optimization
      |
      +-- calls the entire deterministic stack
```

That dependency structure is the reason L5 is not merely another feature: it is the safety harness
around every other feature, and it is why L5 is delivered first.

## 12. Decisions

These replace the open questions from the previous revision. Each is a decision, not a preference,
and each is traceable to the L-item it governs.

### D12. Portfolio construction

Adopt an optional signal-to-target-to-execution pipeline. Direct order submission through
`ExecutionAlgorithm` remains a first-class path. Portfolio construction must not become mandatory or
introduce a second source of position truth. `Signal`, `Target`, and `Order` are distinct semantic
objects. The feature is justified primarily for multi-asset allocation, strategy composition, and
research workflows. Governs L1.

### D13. Corporate-action scope

Implement L3 as an asset-class-scoped instrument identity, corporate action, and historical
normalization capability. It is required for equity-capable research and trading, but it must not
impose equity-specific runtime behaviour on crypto, FX, or derivatives. Corporate-action semantics,
symbol identity, and historical normalization remain separate from the core event engine. Governs
L3 and L9A.

### D14. Execution model scope

Execution configuration supports instrument-level overrides while retaining matching-engine and venue
defaults. Existing fill variants stay compatible through a configuration and adapter layer. The
initial implementation must preserve existing golden results. Governs L10.

### D15. Slippage separation

Separate fill, slippage, and fee as conceptual interfaces, but initially preserve existing composite
fill behaviour. Introduce the abstractions without changing default semantics; independently
configurable slippage becomes a subsequent migration step validated against regression artifacts.
Governs L10.

### D16. Optimization boundary

Optimization is a research and orchestration capability implemented primarily in Python, while
backtest execution remains in the Rust engine. CLI interfaces may invoke the same optimization API
but must not create a second optimizer implementation or a second semantic model. Governs L6.

### D17. Configuration

Typed constructors remain the canonical configuration API. File-based configuration is optional
serialization and loading functionality and is not required for the core architecture. YAML or JSON
must not become a new source of configuration semantics or precedence complexity. Governs L8.

### D18. Regression oracle

Regression validation uses three complementary layers: canonical digest, declared statistics, and
deterministic semantic checkpoints. The digest detects any change, statistics explain aggregate
differences, and explicit event checkpoints localize meaningful behavioural divergence. Full
event-stream snapshots are avoided unless a specific scenario requires one. Governs L5.

### D19. Compatibility target

Decision: B, architectural capability parity, is the primary compatibility target. C, selective
research-workflow parity, is secondary. A, Lean user-facing programming-model compatibility, is
explicitly not a project goal.

The project selectively adopts capabilities demonstrated by Lean when they provide material value to
the NautilusTrader architecture. Candidate capabilities include dynamic universe selection and
membership, signal-to-target portfolio intent, corporate actions and instrument identity, trading
calendars, execution and reality modeling, deterministic regression, optimization, research APIs,
and data tooling.

These capabilities must be implemented using NautilusTrader's existing architectural principles:
event-driven execution, typed domain models, deterministic simulation, backtest and live parity, and
existing execution and risk semantics.

Lean-shaped types such as `Insight`, `AlphaModel`, `PortfolioTarget`, `Universe`, and `RiskModel`
are not compatibility requirements. Equivalent NautilusTrader-native abstractions may use different
names, lifecycles, interfaces, or internal implementations. A Lean abstraction is adopted only when
its semantics provide a concrete capability, composability, testability, or research benefit.

The rule that follows is: **adopt Lean semantics and proven capabilities, not Lean APIs or
implementation mechanisms.**

Compatibility hierarchy:

1. B, capability parity: required target for selected features.
2. C, workflow parity: selectively supported where valuable.
3. A, API and programming-model compatibility: explicitly out of scope.

Compatibility test: a Lean feature is implemented only when it closes a demonstrated capability gap,
NautilusTrader does not already provide an equivalent, and the capability can be implemented without
weakening the invariants in section 2. Section 4 holds the procedure and the per-item
classification.

Research-workflow parity is a secondary objective. The project supports useful equivalents for
notebook research, parameter optimization, walk-forward analysis, regression testing, and data
tooling where they materially improve the workflow, but these consume NautilusTrader-native APIs
rather than reproducing Lean's programming model.

## 13. Remaining open questions

None at this time. D19 resolves the last question from the previous revision. New questions are
recorded here as they arise.
