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
follow-up work. The decisions D12 to D19 in section 12 of the design document are binding inputs to
the corresponding workstreams, including D19, which makes capability parity the primary compatibility
target and Lean API compatibility out of scope. In implementation terms:

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
9. Every workstream passes the capability test in section 4 of the design document before
   implementation starts: it closes a demonstrated capability gap, NautilusTrader does not already
   provide an equivalent, and it does not weaken an invariant. Each workstream is classified as B
   (capability) or C (workflow); a class A (API compatibility) surface is never implemented, and no
   Lean-shaped type (`Insight`, `AlphaModel`, `PortfolioTarget`, `RiskModel`) is introduced for
   naming or API compatibility. This is D19.

Workstream classification:

| Workstream                        | Class | Note                                     |
| --------------------------------- | ----- | ---------------------------------------- |
| W1 regression scenarios           | B, C  | Capability and research workflow         |
| W2 trading calendars              | B, C  | Capability and research workflow         |
| W3 session-aware scheduled events | B, C  | Capability and research workflow         |
| W4 corporate actions and identity | B, C  | Capability and research workflow         |
| W5 universe and membership        | B, C  | Capability and research workflow         |
| W6 execution realism              | B, C  | Capability and research workflow         |
| W7 optional target construction   | B, C  | Capability and research workflow         |
| W8 optimization                   | B, C  | Capability and research workflow         |
| W9 research API                   | C     | Research workflow only                   |
| W10 configuration serialization   | C     | Tooling convenience, no capability claim |
| W11 data CLI                      | C     | Tooling convenience, no capability claim |

No workstream is class A. If a proposal starts from "Lean has this API", it fails the gate at the
first question.

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
`python/tests/acceptance/test_backtest.py` asserts golden values by hand. `BacktestEngine` exposes
the projection in Rust as `get_canonical_result()`, but the Python `BacktestResult` bindings in
`crates/backtest/src/python/result.rs` expose statistics without canonical bytes or a digest.

**Steps.**

1. Expose the canonical artifact to Python: add `get_canonical_result()` to the `BacktestEngine`
   bindings and a `CanonicalBacktestResult` class with `to_bytes()`, `digest()`, and
   `first_divergence(expected)` in `crates/backtest/src/python/result.rs`, delegating to the existing
   writer and classifying the decoding and exact-encoding failure paths under `# Errors`. The
   projection is fallible while `get_result()` is infallible, so the canonical artifact is a typed
   object rather than a field of `BacktestResult`; `first_divergence` gives the digest layer a named
   divergence path.
2. Add a scenario protocol under `python/tests/regression/` with three declared layers (D18):
   - Level 1, the expected canonical digest, with the recorded canonical document kept for
     divergence diagnostics.
   - Level 2, expected statistics such as orders, fills, positions, PnL, fees, slippage, maximum
     drawdown, and final equity, addressed as canonical document paths.
   - Level 3, semantic checkpoints addressed by record kind (`orders`, `fills`, `positions`,
     `position_snapshots`), instrument, and occurrence ordinal, for example the first `ORDER_FILLED`,
     `POSITION_OPENED`, and `POSITION_CLOSED` for an instrument. Checkpoints are semantic rather than
     positional so they survive unrelated record insertions.
3. Add a registry module (`python/tests/regression/registry.py`) that enumerates scenarios, so
   discovery is an explicit list.
4. Add a reset fixture that returns process state to a clean baseline between scenarios: scenarios
   create and dispose their own engine, cache, and clock, the fixture collects unreachable cycles,
   and the suite-wide bypassed logger baseline applies, matching Lean's hard reset
   ([AlgorithmRunner.cs](https://github.com/QuantConnect/Lean/blob/master/Tests/AlgorithmRunner.cs)).
5. Add regeneration through a pytest option or environment variable that rewrites committed
   expectations, all three layers in one command, producing a reviewable diff
   (`--regenerate-regression`, or `NAUTILUS_REGRESSION_REGENERATE=1`).
6. On mismatch, report the failing layer with its expected and actual values, then
   `first_divergence().path`, so failures name the value and the divergence rather than only a digest
   mismatch.
7. Define the scenario-per-feature requirement for later workstreams and record it in
   `docs/developer_guide/testing.md`; add `pytest-regression` to the Makefile and to `pre-flight`,
   and exclude `tests/regression` from the default `pytest` target so its test set is unchanged.

**Boundary.** Research.

**Acceptance.** The suite passes on a clean checkout without regeneration; perturbing a fill model
fails at the digest layer with a named divergence path; a scenario whose later checkpoint changes
fails at the checkpoint layer with expected and actual values; regeneration rewrites all three
layers and a subsequent run passes without regenerating; the default `make pytest` target runs the
same test set as before, and `pre-flight` runs the regression scenarios.

**Risks.** Digest churn on intentional simulation changes, and three layers to update on intentional
change. Mitigation: regeneration covers all layers in one command and is explicit and reviewed, and
`first_divergence()` carries the diagnostic burden.

## 4. W2: trading calendars as data (L4)

**Objective.** A venue calendar model loaded from data and used by scheduling and session gating.

**Steps.**

1. Add `crates/model/src/calendars/` with `TradingSession` (open and close in exchange local time),
   `TradingCalendar` (weekly sessions, holidays, early closes, time zone), and a calendar key of
   venue plus instrument class plus optional symbol.
2. Define the data file as JSON under `crates/model/resources/calendars/` with the schema identifier
   `nautilus-trading-calendar/v1`, loaded with `include_str!` for the bundled default and overridable
   by path. Bundle the four major FX sessions under the synthetic venue `FX` (keyed by session
   symbol) and the `XNYS` equity calendar for 2024 and 2025. Treat the calendar as an immutable input
   to a run. Sessions are declared per weekday, sorted, non-overlapping, and do not cross midnight: a
   market that trades overnight is described as two sessions.
3. Keep `activation_ns` and `expiration_ns` as the source of instrument lifetime; the calendar
   answers whether a given instant is tradeable.
4. Reimplement the FX helpers in `crates/trading/src/sessions.rs` as a thin wrapper over the
   calendar, keeping existing signatures and results. Retain the pre-migration weekday walk in the
   test module as a differential oracle over a spread of dates. The wrapper no longer panics when a
   candidate local time falls in a daylight saving gap, because the calendar resolves such a time
   with compatible disambiguation; the FX session times never fall in a gap, so results are
   unchanged.
5. Validate at load and warn when calendar coverage ends before the run end.
6. Expose the calendar through the Python `nautilus_trader.model` facade and regenerate stubs, with
   the surface limited to loading, key and coverage accessors, tradeability, session queries, and
   boundary resolution.
7. Document the calendars in `docs/concepts/trading_calendars.md` and link the page from
   `docs/concepts/index.md`.

**Boundary.** Engine and research.

**Acceptance.** A unit test resolves a known holiday and a half-day close; the FX session functions
return identical results to the pre-calendar implementation across a spread of dates; a user-supplied
calendar resolves its own sessions for a key that is also bundled; a Python test exercises the
exposed surface; the W1 regression scenarios still pass with unchanged digests.

**Risks.** Stale holiday data. Mitigation: versioned and overridable data, plus a coverage warning.

## 5. W3: session-aware scheduled events (L11)

**Objective.** Make a trading-session phase a first-class, market-anchored event, distinct from a
clock timer and delivered through the existing timer machinery.

**Steps.**

1. Add `crates/model/src/calendars/events.rs` with `SessionEventKind` (`Premarket`, `Open`,
   `OpeningRangeComplete`, `Midday`, `PreClose`, `Close`, `EarlyClose`), `SessionEvent` (kind,
   calendar key, exchange-local session date, session index, `ts_event`, `ts_init`), and
   `SessionScheduleConfig` (the three offsets plus the kinds to derive, each offset capped at one
   day). Each kind is a derivation of a session: `Premarket` is the open minus the premarket offset,
   `OpeningRangeComplete` the open plus the opening range, `Midday` the midpoint of the open and
   close, and `PreClose` the close minus the pre-close offset; `Open` is the session open, `Close` the
   close on a full day, and `EarlyClose` replaces `Close` on a day the calendar declares an early
   close.
2. Expand a calendar with `TradingCalendar::session_events(from, to, config)` over the half-open
   window `[from, to)`, ordered by instant then kind. Offsets are absolute elapsed time, not civil
   clock time. A derived instant that cannot be represented is omitted rather than saturated, and
   `OpeningRangeComplete` is omitted when it would land at or after the close. `ts_event` equals
   `ts_init`, so expanding a schedule reads no clock. The event name
   (`SESSION-{KIND}:{KEY}:{DATE}:{INDEX}`) identifies the event, so scheduling the same window twice
   is idempotent.
3. Deliver through the existing timer path. `crates/common/src/actor/session.rs` registers each event
   as a named time alert on the component clock, so ordering and firing are inherited from
   `crates/common/src/timer.rs` rather than reinvented. `DataActor::on_session_event` in
   `crates/common/src/actor/data_actor.rs` is the callback, separate from `on_time_event`; an event
   whose timer is already pending is not rescheduled.
4. Surface the expansion through `crates/model/src/python/calendars.rs` and `schedule_session_events`
   on the actor and strategy surfaces.
5. Document the kinds, their derivation, the half-open window, determinism, the event name, and the
   timer distinction in `docs/concepts/trading_calendars.md`.

**Boundary.** Engine and research. Expiration is already an engine timer named
`INSTRUMENT-EXPIRATION`, and a user timestamp is already a `Clock` time alert, so W3 duplicates
neither.

**Acceptance.** A unit test derives all six phases of a full day in order with the expected UTC
instants; a holiday derives no events; an early close reports `EarlyClose` and moves the midpoint and
pre-close with it; a half-open window keeps a phase exactly at `to` for the next expansion; disabled
kinds derive nothing; repeated expansion produces an identical result; and the event name identifies
the phase. An offset longer than one day is rejected, and the kinds round-trip through their
canonical strings.

**Risks.** Correctness depends on W2 data quality and on time zone handling. Mitigation: every phase
is a pure derivation of calendar data, so an impossible instant is omitted rather than approximated,
and expanding a schedule reads no clock.

## 6. W4: corporate actions, identity, and the data contract (L3, L9A)

**Objective.** Represent a corporate action as auxiliary data with one type and an exact, opt-in
adjustment, and define the on-disk contract for that data. The capability is class-scoped: it is
inert for assets where a corporate action cannot occur.

**Steps.**

1. `crates/model/src/data/corporate_action.rs` defines `CorporateAction` and `CorporateActionType`
   (`Split`, `Dividend`, `SymbolChange`, `Delisting`). One type carries every kind: `value` is a
   decimal, the new shares per old share for a split and the cash amount per share for a dividend,
   and zero otherwise; `new_symbol` carries the new venue symbol for a symbol change. `effective_ns`
   is when the action takes effect at the venue, separate from the record's `ts_event` and `ts_init`.
2. The type is registered in the `for_each_data_type!` table in `crates/model/src/data/mod.rs` with
   the catalog path prefix `corporate_actions` and added to `Data`, `DataRef`, `DataBatch`, and
   `NautilusDataType`, so catalog paths, the Arrow schema, and the bus topic follow one convention.
3. `crates/serialization/src/arrow/corporate_action.rs` defines the Arrow field specs and batch
   encode/decode, and `crates/common/src/msgbus/switchboard.rs` adds the
   `data.corporate_actions.{venue}.{symbol}` topic with its pipeline topic.
4. Persistence carries the type in `nautilus-persistence`: `write_corporate_actions` and the generic
   query in the Python catalog, `CatalogReader::corporate_actions`, and the delete, consolidation,
   and Feather session dispatch. The data directory is `data/corporate_actions/{instrument_id}/`.
5. `crates/model/src/data/adjustment.rs` defines `PriceRepresentation` (`Raw`, `Adjusted`) and
   `AdjustmentSeries`. The three representations stay distinct and non-interchangeable: raw input
   (immutable, the default for a run), the derived adjusted series (produced on demand), and the
   trading events (the actions themselves, delivered as data). The convention is exact decimal
   arithmetic: for each action in effect after the price instant, `adjusted = raw * split_factor -
   dividends` and `raw = (adjusted + dividends) / split_factor`, with no rounding.
6. The type and its kind enum are exposed through
   `crates/model/src/python/data/corporate_action.rs`, and `AdjustmentSeries` and
   `PriceRepresentation` through `crates/model/src/python/data/adjustment.rs`.
7. The contract is documented: the `## Corporate actions` section in
   `docs/concepts/data/catalog.md` and the colocated file-format contract
   `crates/persistence/src/catalog/README.md`.
8. Delivery reaches actors and strategies: `crates/common/src/actor/corporate_action.rs` registers a
   handler on the topic `data.corporate_actions.{venue}.{symbol}` that dispatches
   `DataActor::on_corporate_action`, and `DataActor::subscribe_corporate_actions` /
   `DataActor::unsubscribe_corporate_actions` (forwarded on `Strategy`) register and release it
   through the actor's existing topic subscriptions. In a backtest the action records reach a
   subscriber only when the data is loaded with an adjustment (or another configuration that
   replays them); an unconfigured run never opens the action stream.

**Not implemented yet.**

- Declared regression scenarios for a mid-series rename and a delisting close. Both behaviours are
  implemented and unit tested, but no declared scenario pins them.
- A declared regression scenario for the adjustment stage. `BacktestNode.get_engine_canonical_result`
  exposes the canonical document of a catalog-driven run, and a node run over catalog bars does fill
  an order: the bar establishes the market before the strategy sees it, so a market order for a
  lot-size multiple submitted from `on_bar` is matched against that book and appears as an order, a
  fill, and a position in the run's reports and canonical document, which
  `python/tests/integration/test_backtest_node_bar_fills.py` asserts. An earlier note in this section
  that such an order never fills was wrong. An empty `orders` array cannot come from a venue rejection,
  because every submitted order enters the execution cache before the venue processes it, so the
  earlier observation came from a run in which no order was submitted at all. What remains is the
  scenario itself.

**Boundary.** Engine and research. File formats are the data contract (L9A).

**Acceptance.** Unit tests in `crates/model/src/data/corporate_action.rs` cover the split ratio, an
exact dividend amount, a symbol change, `ts_init`, display, and metadata. Unit tests in
`crates/model/src/data/adjustment.rs` cover a 4:1 split adjusting and reversing, a dividend
subtracted, a split and dividend composed, two splits composed, a symbol change and a delisting not
scaling a price, `convert` between representations including identity, effect-time sorting, and the
rejection of a foreign instrument or a non-positive split. `crates/serialization/src/arrow/corporate_action.rs`
round-trips a batch, and `crates/backtest/src/config.rs` accepts `NautilusDataType::CorporateAction`
in the data configuration allow list. The stage is covered end to end by
`python/tests/integration/test_backtest_node_corporate_actions.py`, which runs a node over a
synthetic catalog holding a 4:1 split and a dividend and asserts the converted series, the reverse
conversion, an unconfigured run passing raw prices through while replaying no action record, and
that each action record is processed at its effective instant rather than at its announcement. That
suite also asserts delivery to a subscribed strategy: with an adjustment the strategy receives each
action re-stamped to its effective instant, and without one it receives nothing.
`BacktestNode.get_engine_canonical_result` exposes the canonical document of a catalog-driven run,
so a declared scenario could pin the stage using the shape that
`python/tests/integration/test_backtest_node_bar_fills.py` establishes for a node run over bars. The
unchanged default path is asserted by every scenario that predates the stage keeping its committed
digest.

**Risks.** Mixed adjusted and raw semantics inside one strategy is the classic silent error.
Mitigation: raw data immutable, adjustment opt-in per data configuration, and a representation
recorded per run in the canonical document. The residual risk is that an action cannot reach a
Python component, that a mid-series rename and a delisting close have no declared scenario, and that
the stage has no declared scenario yet.

## 7. W5: universe definition, selection, and membership (L2)

**Objective.** A selection model and membership lifecycle that add and remove instruments at runtime
through the existing subscription machinery.

**What is implemented.**

1. `crates/trading/src/universe/` holds the `Universe` component: an actor with a definition
   (`definition.rs`), selection rules (`rule.rs`: `StaticUniverseRule` and `ScheduledUniverseRule`),
   the component-side member record (`membership.rs`), and the component itself (`component.rs`).
   The membership values, the change record, and the transition table live in
   `crates/model/src/universe.rs`, because a change crosses component boundaries.
2. Removal is a process, not an immediate unsubscribe: a departing member moves to `REMOVING` and
   keeps its claims. The universe reports the open orders and positions it can see once per blocking
   condition, and completes the removal on a later selection step or on request.
   `UniverseRemovalPolicy::RequireFlat` is the default and `ReleaseRegardless` is opt-in. The
   universe never submits or cancels orders: the owning component acts on the change.
3. Member subscriptions go through the existing data command path, so a departing member releases
   only the claims the universe holds, and stopping the component releases every claim it holds.
   Unit tests assert the subscribe and unsubscribe balance per claim over an add, remove, and stop
   lifecycle.
4. Instrument definitions are requested through the existing flow: `request_instrument` per added
   member and `request_instruments` for the venue when the component starts.
5. Changes are published on `events.universe.{name}` and delivered to actors and strategies through
   `DataActor::on_universe_changed`, subscribed with `subscribe_universe_changes` and released with
   the component's other subscriptions.
6. The component is exposed through `crates/trading/src/python/universe.rs` and
   `nautilus_trader.trading`, and registered with a run through `add_universe` on the backtest engine
   and the backtest node.
7. A golden scenario (`python/tests/regression/cases/universe_membership.py`) covers a scheduled
   universe whose membership decides which instruments trade, and a Python integration test covers
   delivery to a subscribed strategy and the held removal of a departing member.
8. `docs/concepts/universes.md` documents the definition, the selection step, the removal process,
   and the subscription ownership rules.

**Measured cost.** An armed universe is not free, but its cost is the scheduled selection step and
not the event count. Measured over 40,000 quote events with a subscriber, comparing no universe
against a universe whose only selection step falls after the run ends: no universe 32.7 microseconds
per event, a universe selecting once per second 49.6 microseconds per event over 20,000 selection
steps, and the same universe selecting once per hour 32.8 microseconds per event over 5 steps, which
is within run-to-run noise of the baseline. The overhead is therefore attributable to the selection
step at roughly 34 microseconds per step, and it falls away as the interval widens: the component
adds no measurable per-event cost of its own, and a universe that is never configured adds none of
this at all. The existing scenarios keep their digests unchanged.

**Boundary.** Engine.

**Acceptance.** The component's unit tests cover the transition table, canonical selection, the held
removal, both removal policies, and the claim balance of an add, remove, and stop lifecycle. The
golden scenario and its committed expectations prove a scheduled universe reproduces its result
across runs, and the scenarios that predate it keep their digests unchanged. The Python integration
test covers delivery to a subscribed strategy through a real run. The live path is covered by
`test_live_node_universe_releases_member_subscriptions_on_stop` in `crates/live/src/node/mod.rs`,
which registers a recording data client through the node builder, drives the node's run loop, and
asserts that every subscription the universe made for its members is released when the node stops.
The per-event cost is measured above: an armed universe costs roughly 34 microseconds per selection
step and nothing attributable to the events themselves, so no acceptance item remains open.

**Risks.** Subscription ownership bugs and live metadata gaps. Mitigation: the claim-balance tests
over the data command path, a definition that declares its subscriptions explicitly, and the fact
that a member only becomes `ACTIVE` once the run knows its definition, so a member without metadata
is visible as `ADDED` rather than silently tradable.

## 8. W6: execution realism (L10)

**Objective.** Make the existing realism models explicit, composable, and inventoried, before any
optimization depends on them.

**What already exists.** `crates/execution/src/models/{fee,fill,latency}.rs`; eleven fill variants
including `OneTickSlippageFillModel`, `LimitOrderPartialFillModel`, `SizeAwareFillModel`,
`CompetitionAwareFillModel`, `VolumeSensitiveFillModel`, and `MarketHoursFillModel`; seeded
probabilistic fills via `ProbabilisticFillState`; `liquidity_consumption` and `queue_position` in
`OrderMatchingEngineConfig`; margin models in `crates/model/src/accounts/margin_model.rs`;
`MarketStatusAction::Halt`.

**What is implemented.**

1. The realism inventory. `docs/concepts/backtesting/fill-models.md` carries an `## Execution
   realism inventory` section that maps every fee model, fill model, latency model, queue position,
   liquidity consumption setting, market status handling, and partial-fill path to the source lines
   that run, and closes with the paths that are random or time dependent and whether they take a
   seed. It describes the code as it stands, so it is a step of this workstream rather than a settled
   contract: the later steps must keep it true. It also describes the configuration layer below.

2. Stage A, the configuration layer (D14). `FillModelKind` names the eleven fill variants and
   `FillModelConfig` describes one of them as data: the kind, `prob_fill_on_limit`, `prob_slippage`,
   `random_seed`, and the `CompetitionAware`-only `liquidity_factor`
   (`crates/execution/src/models/fill.rs:1453-1530`). `FillModelConfig::resolve` constructs the
   named model through that model's own constructor (`fill.rs:1544-1623`), so the configuration
   holds no fill behaviour of its own: it is a description that resolves to the existing
   implementation rather than a second one. A `liquidity_factor` supplied for any other kind is an
   error rather than a silently ignored setting, and an omitted factor uses the model default. The
   default configuration (`fill.rs:1532-1542`) resolves to `FillModelAny::default()`, so the default
   path is unchanged.
3. The compatibility adapter is the existing conversion path.
   `pyobject_to_fill_model_any` resolves a configuration before the model bindings
   (`crates/execution/src/python/fill.rs:157-160`), so `BacktestVenueConfig`,
   `BacktestEngine.add_venue`, `BacktestEngine.change_fill_model`, and the sandbox client
   configuration accept either a model object or a configuration. Nothing downstream of the adapter
   changed: the venue configuration still carries a `FillModelAny`, the exchange still carries a
   `FillModelHandle`, and the eleven model implementations are untouched.
4. `FillModelConfig` and `FillModelKind` are exposed through `nautilus_trader.execution`
   (`crates/execution/src/python/mod.rs:44-45`), with the Python stubs regenerated.
5. The unit tests in `crates/execution/src/models/fill.rs` cover that each kind resolves to the
   model it names, that the probabilistic parameters and the seed are forwarded, and that
   `CompetitionAware` alone consumes `liquidity_factor`. Python tests cover a venue configured by
   description and the rejection of a liquidity factor for another kind.
6. Step 3, the selection inheritance chain. `FillModelSelection`
   (`crates/execution/src/models/fill.rs:1643-1716`) is the single place the chain is resolved. It
   holds the levels the venue configuration sets, from least to most specific: the venue default
   and the per-instrument overrides. `resolve` returns the instrument's override when the
   instrument has one and the venue default otherwise, so a level that sets no model inherits.
   Underneath, the global level is the built-in model the venue default falls back to
   (`FillModelHandle::default()`), and above it the matching-engine level is the handle each engine
   holds, which `OrderMatchingEngine::set_fill_model` replaces at runtime.
   `SimulatedExchange` and the sandbox client both carry a `FillModelSelection` and resolve it once,
   when the matching engine for an instrument is created
   (`crates/backtest/src/exchange.rs:325-345,531`, `crates/adapters/sandbox/src/execution.rs:145-158,1017`).
   A venue-level `set_fill_model` replaces the venue default for every instrument without an
   override and leaves the overrides in place (`exchange.rs:325-336`), and
   `SimulatedExchange::fill_model_for` exposes the resolution for an instrument
   (`exchange.rs:343-344`). The overrides are configured exactly like `leverages`:
   `SimulatedVenueConfig.instrument_fill_models` (`crates/backtest/src/config.rs:296-301`) and
   `BacktestVenueConfig.instrument_fill_models` (`config.rs:550-554`), mapped on the node path
   (`crates/backtest/src/node.rs:267-277,306`) and accepted by `BacktestEngine.add_venue` and
   `SandboxExecutionClientConfig`. The sandbox field serializes as runtime-only, like its other
   models, and an empty map is omitted.

7. Step 4, Stage B independent components (D15). `crates/execution/src/models/slippage.rs`
   introduces `SlippageModel`, the second concern of the set: it answers only whether a fill price
   moves one tick against the order direction (`slippage.rs:33-40`). `ProbabilisticSlippageModel`
   is the built-in implementation (`slippage.rs:83-134`), carrying `prob_slippage` and
   `random_seed` and drawing from the same `ProbabilisticFillState` the fill models use, so a
   seeded model reproduces its draws and a decomposed configuration reproduces a composite one
   draw for draw. The matching engine carries an optional independent slippage model
   (`crates/execution/src/matching_engine/mod.rs:117,557-564,5137-5156`) and consults it in place
   of the fill model's own slippage when one is set; when none is set the fill model decides,
   which is the default path. Fill and fee were already independently configurable, and slippage
   joins them as `slippage_model`, a venue-level field on `SimulatedVenueConfig`
   (`crates/backtest/src/config.rs:302-307`), `BacktestVenueConfig` (`config.rs:555-560`),
   `BacktestEngine.add_venue`, and `SandboxExecutionClientConfig`
   (`crates/adapters/sandbox/src/config.rs:104-115`), mapped on the node path
   (`crates/backtest/src/node.rs:291-292,308`), so backtest and sandbox stay at parity.
   `ProbabilisticSlippageModel` is exposed through `nautilus_trader.execution`
   (`crates/execution/src/python/mod.rs:58`).

   The ordering the engine implements, and that the configuration documents, is: fill
   eligibility, then fill quantity, then base fill price, then the slippage adjustment, then the
   final fill price, then the fee. The fill model decides whether the order is eligible, how much
   fills, and at what base price. The slippage model then adjusts that base price by one price
   increment against the order direction on an L1 book, producing the final fill price. The fee
   model is charged last, on that adjusted price and the fill quantity.

   Stage B's acceptance evidence: a run with an independent `ProbabilisticSlippageModel`
   (`prob_slippage=0.5`, `random_seed=7`) produces the same canonical digest as a run with the
   composite `DefaultFillModel` that folds the same draw in, and a run without slippage differs,
   so the draws are not vacuous. The reproduction holds for a composite whose fill decision is
   deterministic; the limitation below records the boundary.

8. Steps 5 and 6, the optional model additions. Each candidate the design names was evaluated on its
   own merits and on its own seam, and one was adopted: market impact.
   `crates/execution/src/models/market_impact.rs` introduces the `MarketImpactModel` concern, whose
   `impact_increments(fill_quantity)` returns the number of price increments a fill moves against
   the order direction (`market_impact.rs:45-54`), a shared `MarketImpactModelHandle`
   (`market_impact.rs:58-90`), and the built-in `LinearMarketImpactModel`
   (`market_impact.rs:110-160`). The model moves the price one increment for every
   `quantity_per_increment` units filled, capped at `max_increments`, using exact decimal division
   and a floor, so it is deterministic and takes no random seed: no second random state is
   introduced and `ProbabilisticFillState` is untouched. The matching engine carries an optional
   model (`crates/execution/src/matching_engine/mod.rs:118`), set with `set_market_impact_model`
   (`mod.rs:566-574`), and applies it to a liquidity-taking L1 fill after the slippage adjustment
   (`mod.rs:5157-5190`); an L2/L3 fill, a resting maker fill, or a model that returns zero leaves
   the price unchanged. It is configured through `market_impact_model` on `SimulatedVenueConfig`
   (`crates/backtest/src/config.rs:313`), `BacktestVenueConfig` (`config.rs:566`, accessor
   `config.rs:813-815`), `BacktestEngine.add_venue`, and `SandboxExecutionClientConfig`
   (`crates/adapters/sandbox/src/config.rs:118-127`), mapped on the node path
   (`crates/backtest/src/node.rs:293-295,309`) and set on each sandbox matching engine
   (`crates/adapters/sandbox/src/execution.rs:166-168,179,919,1043-1045`), so backtest and sandbox
   stay at parity. `LinearMarketImpactModel` is exposed through `nautilus_trader.execution`
   (`crates/execution/src/python/mod.rs:59`). The field is absent by default, so a venue that sets
   none adjusts no fill price for size.

   The four candidates not adopted, each for a specific reason:

   - Spread. The synthetic spread belongs to the fill model: the models that supply or widen it
     (`MarketHoursFillModel`'s normal or one-tick-wider book, `BestPriceFillModel`,
     `OneTickSlippageFillModel`) build the synthetic book the engine fills against, so a separate
     spread model would be a second owner of that book and would duplicate those models rather than
     add a capability.
   - Partial-fill policy. Fill quantity is the fill model's concern in the documented ordering, and
     partial fills already arise from finite synthetic level sizes (`TwoTierFillModel`,
     `ThreeTierFillModel`, `LimitOrderPartialFillModel`, `SizeAwareFillModel`,
     `CompetitionAwareFillModel`, `VolumeSensitiveFillModel`, `MarketHoursFillModel`) and from queue
     position. A separate policy would be a second owner of fill quantity, not a new capability.
   - Borrow and locate availability for short selling. The simulated path has no locate data source
     and no seam: the only borrow concept is the `CashAccount::allow_borrowing` flag, and a margin
     account's short capacity is derived from the margin model, which exposes no availability hook.
     A locate model would have to invent a per-instrument borrow source and consumption accounting
     across the account layer, which is a new capability rather than an opt-in model with an
     existing seam.
   - Auction and halt policy. Halt is already a `MarketStatus` transition in
     `OrderMatchingEngine::process_status`, so a halt policy would duplicate it, and the matching
     engine has no auction or uncrossing routine for an auction policy to hook into. Auction
     behaviour is an engine capability, not an opt-in execution model.

   Step 6 for the adopted model: it is deterministic (no seed is required), opt-in (absent by
   default), and unit tested for exactness, capping, and repeatability
   (`market_impact.rs:198-273`), with the engine hook covered by
   `matching_engine/mod.rs:12164-12193`: the default path fills at the best ask and a configured
   model of one increment per 10 units fills three increments above it, reproducibly.

9. Step 7, the golden scenarios for the added models. Three declared scenarios in
   `python/tests/regression/cases/` pin the added execution realism in committed canonical
   documents: `venue_slippage_model` configures the seeded `ProbabilisticSlippageModel` alone,
   `market_impact_model` configures the `LinearMarketImpactModel` alone, and
   `execution_realism_composed` configures both. They share one harness
   (`python/tests/regression/execution_realism.py`), which runs a `BacktestNode` over a synthetic
   bar catalog with a strategy that submits one market order from `on_bar`, in the shape the node
   path integration test establishes, and each scenario declares a fixed run config ID.

   The scenarios pin the increment arithmetic of the ordering against a bar whose price is
   `100.00`: the default path fills the first fill at `100.00`, the seeded slip at `100.01`, the
   impact model at `100.02` (two increments for the 25 unit fill the synthetic book supplies, at
   one increment per 10 units), and the composition at `100.03`. The concerns are therefore
   additive and compose in the documented order, and the committed digests are distinct.

   Each scenario observes two fills, because the synthetic L1 book supplies 25 units of the 100
   unit order: the engine fills the remainder one increment beyond the last fill price
   (`crates/execution/src/matching_engine/mod.rs:5270-5341`), so the remainder carries the adjusted
   price forward and the checkpoints pin two prices per scenario rather than one.

   A node scenario needs a declared run config ID. A canonical document records
   `run/run_config_id`, and a generated one is random, so a scenario that lets the node generate it
   does not reproduce its own digest; each scenario passes a fixed ID to `BacktestRunConfig`.

**Not implemented yet.**

- The decomposition boundary for a stochastic composite. A composite fill model draws its
  limit-fill and slippage decisions from one random stream; an independent slippage model owns its
  own. A decomposed run therefore reproduces a composite run exactly only when the composite's fill
  decision is deterministic (`prob_fill_on_limit` of `0.0` or `1.0`), in which case the composite
  consumes no random bytes before its slippage draws and the two streams coincide. With a
  stochastic fill decision the draws are statistically equivalent but not identical, so the
  canonical digests differ. Any future composite that folds more than one random concern into one
  model inherits the same boundary.
- A slippage inheritance chain. Independent slippage is venue-level only: `slippage_model` has no
  per-instrument override level, and the matching-engine level holds whatever the venue resolved.
  This step adds one concern without the chain, which is the staged-migration rule.
- Order-specific fill model overrides, the last level of the chain. An order carries no fill model
  affiliation and the matching engine has no per-order hook, so there is nothing a caller could
  declare and nothing the engine could inherit from: the level would be a setting with no effect
  unless the order path itself changed. The delivered chain therefore ends at the per-instrument
  override.

**Boundary.** Engine and research. Models are used by both backtest and sandbox execution.

**Acceptance.** The inventory document matches the code. Stage A is complete only when
`old_digest == new_digest` for every existing golden scenario: the scenarios in
`python/tests/regression` pass with their committed expectations unchanged, and the default path
resolves as before. The selection chain adds no default behaviour, because a venue that configures
no instrument overrides resolves every instrument to the venue model it already used, so the same
gate covers it. Stage B is complete only when independently configured slippage reproduces the
composite behaviour it replaces, which this step demonstrates for a deterministic-fill composite
with matching canonical digests; the decomposition boundary above records where that reproduction
stops. Each new model is independently selectable and leaves the default path byte-identical,
seeded models reproduce across runs, and existing fill variants continue to pass their tests
unchanged. The `slippage_model` field is absent by default, so a venue that sets none resolves
slippage exactly as before. Steps 5 and 6 are separately complete when every candidate addition has
a recorded decision with its reason, the adopted market impact model is deterministic and opt-in,
and the `market_impact_model` field is absent by default so the default path stays byte-identical.
Step 7 is complete: the three added scenarios commit distinct canonical digests over the arithmetic
of the ordering, every scenario predating them passes with its committed expectations unchanged, and
each scenario reproduces its digest across repeated runs.

The evidence behind each stage: Stage A's is the scenarios that predate the workstream, whose
committed expectations are byte-identical after every step of it. Stage B's is the composite
reproduction above, bounded as recorded. Step 5's is a recorded decision and reason for each
candidate addition, and Step 6's is that the one adopted model is a pure function of the fill
quantity and the seeder is unchanged. The inventory's citations were audited against the code after
the final step: of 194 cited locations, 105 were already accurate and 89 pointed at lines that the
subsequent steps had shifted or that were imprecise from the start, and all 89 were corrected, so
every citation resolves to the construct its sentence describes. The audit changed no prose and no
code.

**Node path.** `python/tests/integration/test_backtest_node_bar_fills.py` runs a node over a
synthetic bar catalog with a strategy that submits a market order from `on_bar` and asserts that the
order, the fill, and the position appear in the node's reports and that the canonical document
differs from the same run without the order. The bar establishes the market before the strategy sees
it, so a fill needs no quote or trade data. The same test configures the venue with a
`OneTickSlippage` configuration and asserts the canonical document differs from the default one, so
the configuration layer reaches the matching engine on the node path, which the execution realism
scenarios now use for their committed digests. The venue-level `slippage_model` and
`market_impact_model` fields are mapped on the same path.

**Migration invariant.** Separating an abstraction must not automatically change simulation
semantics. This is the acceptance criterion that makes the staged migration safe.

**Risks.** Configuration surface growth and golden churn. Mitigation: one model per change, opt-in,
never more than one concern migrated at a time, and a golden scenario per model.

## 9. W7: optional target construction (L1)

**Objective.** Optional target construction (D12, D19) between decision-making and order submission.
The direct path and the order layer are unchanged, and both paths converge on `ExecutionAlgorithm`.
The deliverables use NautilusTrader-native names and lifecycles; no Lean-shaped type
(`Insight`, `AlphaModel`, `PortfolioTarget`, `RiskModel`) is introduced for API compatibility.

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

**Implementation.** Steps 1 to 4, delivered earlier, define the `TradingSignal` and `Target` values
in `crates/model/src/{signal,target}.rs`, the `TargetConstruction` stage with
`TargetConstructionConfig`, and the `TargetReconciler` with `ReconcileContext`, `TargetOrder`, and
`TargetReconcilerError` in `crates/trading/src/target.rs`. Steps 5 to 7 were delivered together:

5. The opt-in surface. `crates/trading/src/target_pipeline.rs` adds `TargetPipelineConfig`, which
   bundles a `TargetConstructionConfig` with a `min_order_quantity: Quantity`, and `TargetPipeline`,
   which holds a `TargetConstruction` and a `TargetReconciler` and delegates `construct` and
   `reconcile` to them without adding semantics. `StrategyCore` holds an `Option<TargetPipeline>`
   that is `None` until the strategy opts in, with `enable_target_pipeline`, `disable_target_pipeline`,
   `target_pipeline_enabled`, and `target_orders`. `target_orders` builds the construction and
   reconciliation contexts from what the strategy already holds: equity from the portfolio, positions
   and open orders from the cache, and prices and instrument definitions from the cache (a price
   prefers the last trade, then the mid quote, then the mark price). The `Strategy` trait forwards
   the three control methods and adds `submit_signals`, which submits each reconciled `TargetOrder`
   as a market order on the existing `submit_order` path and returns the client order IDs in the
   order emitted. Python exposes `TargetPipelineConfig` (in `nautilus_trader.trading`) and the four
   methods on `Strategy` (`crates/trading/src/python/strategy.rs`,
   `crates/trading/src/python/target_pipeline.rs`).
6. The three-layer separation is documented in `docs/concepts/target_pipeline.md`, linked from
   `docs/concepts/index.md`: a signal is not a target and a target is not an order, neither is a
   trading command, the cache and the portfolio stay authoritative, and the pipeline is opt-in while
   the direct path is unchanged.
7. The parity evidence and the golden scenario.
   `python/tests/integration/test_target_pipeline_parity.py` runs the same quote data twice: one
   strategy submits a market order of 100 units directly, and one enables the pipeline and submits a
   long signal whose constructed target, at 1,000,000 USD equity, a mid price of 100.000, a 1 per
   cent stop, and 0.0001 of equity risked, resolves through the fixed-risk sizing to the same 100
   units. The observed sets agree:

   - direct orders `[("AAPL.XNAS", "BUY", "100")]`, pipeline orders
     `[("AAPL.XNAS", "BUY", "100")]`;
   - direct fills `[("AAPL.XNAS", "BUY", "100.01", "100")]`, pipeline fills
     `[("AAPL.XNAS", "BUY", "100.01", "100")]`;
   - direct positions `[("AAPL.XNAS", "LONG", "100")]`, pipeline positions
     `[("AAPL.XNAS", "LONG", "100")]`.

   The golden scenario `python/tests/regression/cases/target_pipeline_parity.py` runs the pipeline
   path and declares its order and fill; its committed digest is
   `blake3:1f23f678e90160164393ac0632eb9956d224ddab3ebfcc2ac362121d1dee07a7`.

Acceptance is met for the simple case: the two paths produce equivalent orders for equivalent
intent, the pipeline is disabled by default, and the goldens that predate this change are unchanged.
The hot-path regression claim is not separately benchmarked here; the pipeline is inert when unused,
because a strategy holds no pipeline until it opts in and the direct path is untouched.

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
7. Expose a new top-level `optimize` command in `crates/cli/src/opt.rs`, alongside the existing
   `database`, `catalog`, and `blockchain` commands, as a thin front end that invokes the same
   Python optimization API, plus a Python helper for notebooks. There must be exactly one
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

**Objective.** A small data surface in `nautilus-cli` that extends the existing `catalog` command
over the catalog and existing loaders.

**Steps.**

1. Extend the existing `catalog` command in `crates/cli/src/opt.rs` with data subcommands: download,
   validate, convert, generate, inspect, each delegating to `CatalogReader` and `CatalogWriter` and
   the existing loaders. The existing `migrate-parquet` subcommand and the `database` and
   `blockchain` commands are unchanged.
2. Keep the surface explicit and licensing-aware; do not embed provider credentials or bundle data.
3. Emit machine-readable output for CI use.
4. Document the commands and the data contract from W4.

**Boundary.** Tooling.

**Acceptance.** `validate` reports a malformed catalog; `convert` produces a catalog that a backtest
reads; `inspect` lists data types, instruments, and coverage; no credentials are written to disk.

**Risks.** Scope growth into a data platform. Mitigation: the five documented subcommands are the
whole added surface, alongside the existing `migrate-parquet`.

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
- Each workstream passes the capability test (section 4 of the design document) and is classified B
  or C in section 1; no class A surface is added, and no Lean-shaped type is introduced for naming or
  API compatibility.

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
| Compatibility surface creep            | Medium | Capability test gate (rule 9); workstream classification table         |

## 17. Out of scope

- A cloud platform, job queue, or multi-tenant scheduler.
- Dynamic handler resolution by class-name string from configuration.
- A slice or time-batch delivery model in the runtime.
- A research engine mode or a query-only kernel mode.
- Transplanting Lean's `Security` model-slot hierarchy.
- A Lean-compatible user-facing programming model and any Lean-shaped type introduced for naming or
  API compatibility. This is compatibility target A, rejected by D19.
- A second optimizer implementation alongside the Python optimization API.
- Environment profiles or environment variables as a configuration source.
- AI or ML tooling.
- Changes to `.github/workflows` and `.github/actions`, which are maintainer-owned.
