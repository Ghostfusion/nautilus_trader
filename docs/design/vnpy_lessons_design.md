# VeighNa lesson review: design considerations

Status: design review. No production code changes accompany this document. The companion record is
[`vnpy_lessons_implementation.md`](vnpy_lessons_implementation.md).

## 1. Purpose and scope

VeighNa (formerly vn.py) is the most widely used open source Python trading framework in the Chinese
retail and proprietary trading market, and it has existed since 2015. It is the closest thing to a
peer of this project in terms of scope: an event-driven engine, a strategy host, a backtester, an
optimizer, a risk layer, live trading adapters, and an options analytics application.

This review asks one question: **what does VeighNa implement, or know, that an event-driven
framework for United States markets should adopt.** It is deliberately not a feature comparison and
not a conversion plan. Nothing here is a commitment; the decisions in section 9 decide the
mechanisms, and the companion record carries the evidence and the acceptance criteria for each.

Two constraints are binding:

1. **United States only.** Logic may be imported only if it applies to US markets and US
   securities. Chinese and other foreign market machinery is out of scope, is listed in section 8,
   and is never a prerequisite for an accepted item.
2. **No code changes for improvements.** The instruction that authorised this review allows changes
   only for defects. Items accepted below are therefore specified, ordered, and given acceptance
   criteria, and are not implemented by this workstream. Section 8 of the companion record states
   which checks were performed while looking for defects in this repository.

## 2. Method and evidence

The review was performed against source, not documentation or secondary commentary.

| Source                    | Revision                                                  | Size                          |
| ------------------------- | --------------------------------------------------------- | ----------------------------- |
| `vnpy/vnpy`               | `fa5206fe63836f3f8cd1ebd7168fbd19a5e2ff09`, version 4.4.0 | 60 Python files, 12,840 lines |
| `vnpy/vnpy_ctastrategy`   | clone of `main`                                           | 20 files, 5,053 lines         |
| `vnpy/vnpy_ctabacktester` | clone of `main`                                           | 7 files, 2,067 lines          |
| `vnpy/vnpy_algotrading`   | clone of `main`                                           | 14 files, 1,704 lines         |
| `vnpy/vnpy_riskmanager`   | clone of `main`                                           | 16 files, 1,666 lines         |
| `vnpy/vnpy_optionmaster`  | clone of `main`                                           | 20 files, 5,778 lines         |

The core repository contains only `vnpy/trader` (22 files, 6,523 lines), `vnpy/event` (2 files, 153
lines), `vnpy/alpha` (25 files, 4,680 lines), `vnpy/chart` (6 files, 1,133 lines) and `vnpy/rpc` (4
files, 327 lines). Every application named in the project README lives in a sibling repository and
is loaded at runtime as a plugin, so the applications were cloned separately. The plugin model is
itself one of the observable facts of the design: the framework core is small and the application
surface is federated.

Repository scale, for context rather than as an argument: 45,667 stars, 12,511 forks, MIT licence,
created 2015-03-02, last push 2026-09-13. Releases are rare and deliberate: 4.0.0 (2025-03-28),
4.1.0 (2025-06-17), 4.2.0 (2025-11-01), 4.3.0 (2025-12-24), 4.4.0 (2026-05-14). Release notes are
used below as evidence of what the maintainers themselves consider load-bearing, and of defects
they have had to fix in exactly the areas this review proposes to touch.

The headings that follow use `L` for a candidate learning and `D` for a decision.

## 3. What VeighNa is, mechanically

Four properties describe the framework closely enough to reason about it.

**A synchronous event bus with a wall-clock timer.** `EventEngine` (`vnpy/event/engine.py`) owns a
single `Queue`, one consumer thread, and a timer thread that emits `EVENT_TIMER` once per second
(`vnpy/event/engine.py`). Gateways push data by calling `on_event`, which wraps the payload in an
`Event` whose type is a string, suffixed per symbol and per order id so that subscribers can filter
without inspecting the payload. Everything downstream, including risk checks and execution
algorithms, is driven by that queue and that one-second timer.

**One mutable global object graph.** `MainEngine` (`vnpy/trader/engine.py`) is a registry of
engines, applications and gateways. `OmsEngine` keeps plain dictionaries of ticks, orders, trades,
positions, accounts, contracts and quotes, and derives the set of active orders from the order
dictionary. Orders carry a `reference` string that identifies the strategy that sent them, and event
dispatch to a strategy is by that string.

**Applications as plugins over the core.** An application is a `BaseEngine` subclass registered with
`main_engine.add_app`. It may monkey-patch core behaviour: the risk manager replaces
`MainEngine.send_order` at runtime (`vnpy_riskmanager/engine.py`). Applications own their own
persistence, their own settings files under `.vntrader`, and their own GUI widgets.

**A flat, untyped data model.** Market and order objects are `@dataclass` types with an `extra`
dictionary for venue specific fields, an id derivation step in `__post_init__`, and dataclass
generated equality over all fields (`vnpy/trader/object.py`). Prices, quantities and money are
Python floats throughout.

The single most important structural difference follows from the last two points. VeighNa's design
assumes a human at a desktop: the framework is a product, the GUI is the primary interface, and the
programmatic surface is what the GUI happens to call. This project's design assumes a library and a
runner: the programmatic surface is the product, and the GUI is someone else's program. That
difference, not any individual feature, is why most of what follows is a small set of mechanisms
rather than a port.

## 4. Where NautilusTrader stands today

Every claim in this table was verified against this repository; the companion record carries the
exact lines.

| Area                           | VeighNa                                                                                                                                             | NautilusTrader today                                                                                                                                                                                                                    |
| ------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Bar aggregation                | Tick to minute, minute to hour, day; weekly declared but routed through the daily path                                                              | Time, tick, volume, value, tick and volume imbalance and runs, Renko, and bar from bar, plus historical warm-up events                                                                                                                  |
| Data model                     | Floats, dataclasses, `extra` dictionary                                                                                                             | Typed prices and quantities, a closed `Data` enum, `CustomData` for venue extensions                                                                                                                                                    |
| Catalog and history            | `BaseDatabase` with per-driver plugins, a bar and tick overview per symbol                                                                          | Parquet catalog with typed query, consolidation, coverage reporting through the catalog CLI                                                                                                                                             |
| Strategy state across restarts | Declared variables scraped to JSON on every trade and restored at init                                                                              | An explicit `on_save` and `on_load` byte-state contract per strategy, gated by `load_state`                                                                                                                                             |
| Pre-trade risk                 | Five rules behind a plugin template, evaluated in Python per order                                                                                  | A Rust engine in the send path: submit and modify rate limits, per instrument notional, reduce only, price and quantity precision, minimum and maximum quantity, trading state, typed denial reasons                                    |
| Execution algorithms           | TWAP, Iceberg, Sniper, BestLimit, Stop, all limit order quoting algorithms on a one second timer                                                    | The `ExecutionAlgorithm` trait and its macro, and one shipped algorithm: TWAP, market orders only, slicing with remainder handling and parent and child linkage                                                                         |
| Backtest statistics            | 28 keys computed from a per day result frame, including drawdown duration and trade cost aggregates                                                 | 34 statistics over a returns or PnL series, with no periodic frame, no drawdown duration, no turnover, commission or slippage aggregate                                                                                                 |
| Optimization                   | Grid and genetic search over a process pool, with a shared evaluation cache                                                                         | Grid search behind a `SearchStrategy` protocol, memory driven fan out, experiment digests, stages, persistence and report                                                                                                               |
| Options                        | Black-Scholes, Black-76, a CRR binomial tree for American exercise, implied volatility, a volatility surface, portfolio greeks, a quoting algorithm | Black-Scholes and Black-76 through the cost of carry term, greeks, implied volatility through the `implied_vol` crate, portfolio greeks, a yield curve, option chain data and an ATM tracker; no American exercise model and no surface |
| Factor research                | An operator algebra, a dataset and model pipeline, and the Alpha101 and Alpha158 factor sets                                                        | A research API over the catalog and indicators, with replay; no factor layer, no panel or dataset abstraction, no model templates                                                                                                       |
| Notifications                  | One call site, email and chat channels, per channel interval coalescing                                                                             | None                                                                                                                                                                                                                                    |
| Event core                     | One queue, one consumer thread, one second timer, string typed events                                                                               | A message bus, dispatch minimisation, and a clock with scheduled timers                                                                                                                                                                 |
| Netting and hedging            | A converter that splits net orders into open, close today and close yesterday                                                                       | `OmsType::Netting` and `OmsType::Hedging` with a reconciliation path                                                                                                                                                                    |

Three of these rows deserve a comment, because they are the reason the accepted list is short. Bar
aggregation and the data model are areas where this project is not merely ahead but built on a
different foundation, so there is nothing to take. Netting and hedging look superficially similar
but are not comparable: VeighNa's converter exists to satisfy Chinese futures offset semantics,
which are exactly the foreign market machinery that this review excludes.

## 5. Comparison

| Dimension              | VeighNa                                                                                                              | NautilusTrader                                                                         |
| ---------------------- | -------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| Language and runtime   | Python, with Cython twins for the hot rule and pricing paths                                                         | Rust core, Python extension                                                            |
| Correctness of numbers | Floating point prices and quantities                                                                                 | Fixed point decimals, exact arithmetic for prices, quantities and money                |
| Extension model        | Runtime plugin discovery, monkey-patching permitted                                                                  | Compile time crates, typed traits                                                      |
| Interface priority     | Desktop GUI                                                                                                          | Library and runner                                                                     |
| Market focus           | Chinese futures, securities and ETF options, with global futures and Interactive Brokers adapters                    | Global, multi venue, with US equities and derivatives among the supported set          |
| Verification           | Manual comparison scripts and smoke tests in places; the strongest artefact is a Python versus Cython parity harness | Regression scenarios with pinned expectations, digests, property tests, and benchmarks |

## 6. Candidate learnings

### L1. Pre-trade risk rules: counts, sizes, and repeated orders

VeighNa's risk manager is a list of independent rules evaluated synchronously before an order
reaches the gateway. A rule subclasses `RuleTemplate` (`vnpy_riskmanager/template.py`), declares
`name`, `parameters` and `variables`, implements `check_allowed(request, gateway_name) -> bool`, and
may implement `on_order`, `on_trade` and `on_timer` callbacks. The engine discovers rules by
scanning a `rules` directory for `*Rule` classes, persists their settings to
`risk_manager_setting.json`, and subscribes to the event types that at least one rule overrides.
Five rules ship, with these defaults:

| Rule                 | Condition                                                                                             | Default                                                                      |
| -------------------- | ----------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| `ActiveOrderRule`    | Count of active orders at or above the limit                                                          | 50                                                                           |
| `DailyLimitRule`     | Orders, cancels and fills, per instrument and in total, at or above the limit                         | 20,000 and 10,000 and 10,000 total; 2,000 and 1,000 and 1,000 per instrument |
| `DuplicateOrderRule` | Occurrences of an identical request string at or above the limit                                      | 10                                                                           |
| `OrderSizeRule`      | Volume above the limit, or limit order value above the limit                                          | 500 and 1,000,000                                                            |
| `OrderValidityRule`  | Instrument unknown, price not a multiple of the price increment, volume outside the instrument bounds | none                                                                         |

This project already validates price and quantity precision, price positivity, instrument existence,
minimum and maximum quantity, per instrument notional, reduce only semantics and trading state, and
it denies through typed `OrderDeniedReason` variants rather than a boolean
(`crates/risk/src/engine/mod.rs`). Three capabilities are genuinely absent:

- a cap on the number of concurrently active orders, which bounds the exposure of a malfunctioning
  strategy without regard to rate;
- a guard against repeated identical requests, which catches a strategy that has entered a loop
  while respecting the configured rate limits;
- count caps over a defined interval for submits, cancels and fills, per instrument and in total.

**Decision (D1): adopt the three caps, reject the architecture.** The caps belong in the existing
Rust risk engine, checked on the same path and denied with the same typed reasons, not in a plugin
list evaluated in Python. The reset boundary must be explicit and documented, in contrast to
VeighNa, whose counters are process lifetime and are never reset despite the rule being named
"daily". The duplicate guard must be a rolling window over a canonical request key, not a
cumulative counter, because the VeighNa form permanently blocks a request shape after the limit is
reached once.

### L2. Execution algorithms: passive and aggressive quoting

VeighNa's algo trading application is a template plus five algorithms
(`vnpy_algotrading/template.py` and `algos/`):

| Algorithm | Mechanism                                                                                                                                                                                                                                                     |
| --------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| TWAP      | Splits the order over `time` seconds in `interval` second slots, child size `volume / (time / interval)` rounded to the instrument minimum, cancels working orders and re-sends when the touch price is reachable, finishes when `total_count` reaches `time` |
| Iceberg   | Keeps one child order of `display_volume` working, sends the next when the previous is done, and force cancels when the market crosses the order price                                                                                                        |
| Sniper    | On each tick, if the touch price is reachable, sweeps at the limit price capped by the displayed size at the touch, and cancels before re-attempting                                                                                                          |
| BestLimit | Joins the touch on the passive side with a child size drawn uniformly from a range, and cancels and re-quotes when the touch moves                                                                                                                            |
| Stop      | Watches the last price against a fixed trigger, adds a slippage offset to the resulting limit price, and clamps to the exchange price limit                                                                                                                   |

The template is a state machine over `RUNNING`, `PAUSED`, `STOPPED` and `FINISHED` with a one second
timer, trade and tick dispatch, a working order set, a traded quantity and a volume weighted traded
price, and `finish()` plus `stop()` that cancel everything.

This project ships the trait, the macro, a Python template example, and exactly one algorithm: TWAP
(`crates/trading/src/algorithm/twap.rs`). Our TWAP is more careful than VeighNa's about arithmetic
and about the framework contract. It validates `horizon_secs` and `interval_secs`, floors the child
size at instrument precision and carries the remainder as a final slice, verifies that the scheduled
sizes sum to the parent quantity, refuses sizes below the minimum quantity by submitting the whole
order instead, and spawns children through the engine so that parent and child are linked. What it
does not do is quote passively or interact with the touch, because it submits market orders only.

**Decision (D2): adopt passive and touch aware algorithms, reject the rest.** The gap is the class
of algorithms that manage a resting order against a moving book. Three are worth implementing in
Rust behind the existing macro: an iceberg, a quote pegged or best limit algorithm, and a sniper.
The stop algorithm is not adopted because this project models stop and trailing stop orders
natively, and reimplementing them as an algorithm would create a second, weaker implementation of
an existing guarantee. Two requirements are added over the VeighNa design, and they are the reason
this is a real workstream rather than a port: child order churn must be bounded and reverified
against the risk engine rate limits, since the VeighNa algorithms cancel and re-send on every touch
change or slot boundary; and an algorithm must tolerate a denied child order, which is possible here
because the risk engine sits in the send path but is not part of VeighNa's model at all.

### L3. A periodic result frame and the cost statistics that fall out of it

VeighNa's backtester maintains `DailyResult` per trading day
(`vnpy_ctastrategy/backtesting.py`), carrying close price, start and end position, the trades of the
day, trade count, turnover, commission, slippage, trading PnL against the previous close, holding
PnL against the day close, total PnL and net PnL. `calculate_result` walks the days carrying
`pre_close` and `start_pos` forward, and `calculate_statistics` computes 28 keys from the resulting
frame, including profit and loss day counts, maximum drawdown and its percentage, maximum drawdown
duration in calendar days measured from the preceding equity peak, total and per day net PnL,
commission, slippage, turnover and trade count, total and annual and daily return, return standard
deviation, Sharpe, an exponentially weighted Sharpe with a configurable halflife, a return to
drawdown ratio, and a composite ratio it calls RGR.

This project computes 34 statistics in `crates/analysis/src/statistics/`, but from a returns or PnL
series rather than from a periodic frame, and the backtest result carries them as
`stats_pnls`, `stats_returns` and `stats_general` maps (`crates/backtest/src/result.rs`). Notably
absent are the periodic frame itself, drawdown duration, the exponentially weighted Sharpe, and any
aggregate of turnover, commission or slippage; and the PnL statistics do not separate realised from
unrealised results.

**Decision (D3): adopt the frame and four statistics, reject the composite.** A periodic result
frame per calendar day, week or month, with closing equity, net PnL, the realised and unrealised
split, commission, slippage, turnover and trade count, is worth having because it is what users
currently reconstruct by hand from fills and account state, and because it makes the four missing
statistics computable from the engine's own accounting rather than from a second marking pass. The
frame must be derived from what the engine already knows, so that it cannot disagree with the
account and the fills. The composite ratio is rejected: it is a weighted combination of six inputs
with no external reference and no theoretical justification, and shipping it would imply a
significance it does not have.

### L4. Options: American exercise and a volatility surface

VeighNa's options application ships three models (`vnpy_optionmaster/pricing/`): Black-Scholes for
spot, Black-76 for futures, and a Cox-Ross-Rubinstein binomial tree that handles American exercise
with a fixed step count. All three share a Newton-Raphson implied volatility solver. On top of the
models it builds a per instrument data model with size scaled greeks, a chain model that aggregates
by expiry and strike, a portfolio model that aggregates greeks across the chain
(`vnpy_optionmaster/base.py`), a put-call parity correction that derives the implied forward of the
underlying, an implied volatility curve fitted with a cubic spline, a delta hedging engine, and a
quoting algorithm that converts a volatility spread into a price spread through vega.

This project already has more of this than expected. `crates/model/src/data/black_scholes.rs` and
`crates/model/src/data/greeks.rs` provide Black-Scholes and Black-76 through the cost of carry term,
exact and approximate greeks, an implied volatility solver built on the `implied_vol` crate with a
Halley refinement step, `PortfolioGreeks`, and a yield curve, with criterion benchmarks. The option
chain types and an ATM tracker live in `crates/model/src/data/option_chain.rs` and
`crates/data/src/option_chains/`. Two things are missing:

- **American exercise.** US listed equity options are American; US index options are European. A
  closed form European model is wrong for the first of those, and the difference is largest for deep
  in the money puts and for high dividend yields.
- **A volatility surface.** Implied volatility is computed per instrument; there is no surface or
  curve fitted across strikes and expiries, so there is nothing to interpolate, to check a quote
  against, or to feed a quoting rule.

**Decision (D4): adopt both, as an analytics module.** A binomial or similar early exercise model
must be added, and each option instrument must be able to declare whether it is American or
European, so that pricing selects the right model instead of assuming one. A surface module must be
built over the chain, with the fitting scheme documented and its error bounded. The parity implied
forward is worth adopting as a diagnostic, and the vega based quoting rule is a possible follow on
to D2 rather than part of this item.

### L5. A factor pipeline between the catalog and a model

VeighNa's `vnpy.alpha` module is a research pipeline: a `DataProxy` wrapper over Polars expressions
with operator overloading, a string expression engine that evaluates factor definitions with
Python's `eval`, a `register_functions` hook for custom operators, four operator families
(time series per symbol, cross sectional over the date axis, mathematical, and technical), a dataset
template that assembles features, a label and a processor chain and computes them over segments
with multiple processes, processors applied separately on the inference and learning sides, an
abstract model with Lasso, LightGBM and multi layer perceptron implementations, and a signal
strategy backtester that replays bars with a daily mark to market pass. The factor sets are ports of
WorldQuant's Alpha101 and Qlib's Alpha158, both credited.

This project has a research API over the catalog and the indicator library with a replay facility,
and no factor layer at all: no expression abstraction, no point in time panel, no cross sectional
operators, no separate inference and learning preprocessing, and no model templates.

**Decision (D5): adopt the shape, reject the implementation.** The shape is right and it is squarely
applicable to US equities, since the factor sets involved are US equity factors in origin. A factor
expression layer that compiles definitions to a typed tree and produces a point in time panel from
the catalog, cross sectional operators including ranking and neutralisation, and a dataset contract
with an explicit inference and learning split, are all worth having as a research capability that
lives outside the event driven engine. The VeighNa implementation must not be copied: a string
expression evaluated with `eval` is unacceptable at this project's correctness bar, and the
replacement, a compiled tree with a canonical serialisation, fits the existing experiment digest
pattern and lets a factor definition be pinned by a regression scenario. Execution stays where it
is; a signal strategy rebalances by sending normal orders. The review adds one requirement from what
VeighNa's own tests show: the factor layer must be verified numerically against an independent
reference, because VeighNa's Alpha101 test asserts only that an output column exists.

### L6. Notification sinks

VeighNa 4.4.0 added `MainEngine.send_notification`, one call site that reaches both an email engine
and a chat engine, with a per channel interval so that messages arriving inside the interval are
coalesced rather than sent individually. The chat channel is specific and is not adopted; the
mechanism is not.

This project has no notification or alert abstraction. Alerts are whatever the operator builds
around logs.

**Decision (D6): adopt a minimal sink interface.** A trait plus configuration with at least an email
sink and a generic webhook sink, a bounded queue, per sink coalescing over a configurable interval,
and no vendor SDK in the core crates. Adoption is cheap, the failure mode it prevents (a strategy
that needs a human but can only log) is real, and it has no US market dependency at all.

### L7. Restart recovery and parameter metadata: already covered

VeighNa declares `parameters` and `variables` as class attributes on a strategy, uses them for
settings application, for display, for optimization ranges and for persistence, and flushes the
declared variables to JSON on every trade so that a restart can restore them (`vnpy_ctastrategy`).

This project has an explicit contract: `on_save` returns a per strategy byte state map and `on_load`
receives it (`crates/trading/src/python/strategy.rs`), gated by the `load_state` configuration flag,
and the optimization package declares an explicit parameter space rather than scraping class
attributes (`python/nautilus_trader/optimization/space.py`).

**Decision (D7): no change.** The VeighNa mechanism couples persistence, display and optimization to
one reflection convention, and flushes state on the trade path. Ours separates them and makes the
state an explicit, versionable artefact. There is nothing to adopt here; the row is recorded because
the absence of a decision would otherwise look like an oversight.

### L8. Optimization: other search strategies and a shared evaluation cache

VeighNa's optimizer offers a brute force grid over a process pool, and a genetic search using the
DEAP library with configurable population, generation and operator parameters, over a process pool
with a manager dictionary that caches evaluations by parameter vector
(`vnpy/trader/optimize.py`). The evaluation cache is the point: a genetic search re-evaluates
descendants across generations, and the cache turns that into a lookup. The release notes for 4.1.0
record making the genetic algorithm hyperparameters fully user controllable.

This project's optimization package defines a `SearchStrategy` protocol and ships one implementation,
a grid search, with memory driven fan out, canonical experiment digests and persistence
(`python/nautilus_trader/optimization/`).

**Decision (D8): adopt, behind the existing protocol.** A random search and an evolutionary search
implementation are worth adding, together with a memo cache keyed by the parameter digest, which the
existing canonicalisation already provides. This is the cheapest accepted item: the protocol, the
digest, the persistence and the report already exist, and a new strategy is one class.

## 7. What not to copy

**Foreign market machinery, excluded by instruction.** The open and close offset model with close
today and close yesterday, the `OffsetConverter`, settlement prices and settlement based PnL,
exchange price limits and clamping to them, Chinese exchanges, products, currencies, contract
multipliers and trading sessions, the 240 day year and exchange calendar conventions in the options
application, regulatory ratio reporting, and the Chinese and global futures and securities gateway
plugins. None of these may be a prerequisite for an accepted item above, and none is.

**Architecture that this project cannot adopt.**

- Monkey-patching a core method to install a subsystem. The risk manager replaces
  `MainEngine.send_order` at runtime.
- Signalling rejection with an empty order id and a log line. Our denial is a typed reason on a
  denial path, and it is part of the observable behaviour.
- Counters that are named for a period but live for the process, and cumulative counters that
  permanently block a request shape.
- Important logic in Python on the per order path, with Cython twins and a micro benchmark used to
  compensate. The equivalent work here is Rust in the send path.
- Runtime plugin discovery from a directory, with `importlib` reload of user files.
- A string expression DSL evaluated with `eval`.
- Floating point prices, quantities and money, and a general purpose `extra` dictionary on every
  data object.
- Statistics that are undefined reported as zero. VeighNa coerces non finite statistics with
  `np.nan_to_num`; this project treats a missing metric as an error rather than a zero, and that
  distinction must survive D3.
- Manual comparison scripts as the verification of pricing mathematics. VeighNa's option pricing
  test prints Python and Cython outputs for a human to compare, and its Alpha101 test asserts only
  that a column exists.

**Product surface that is out of scope, not wrong.** The desktop GUI, the chart application, the
Excel RTD bridge, the web trader, the RPC service, the script and notebook REPL, the internationalisation
layer, and the account and position manager applications. These solve distribution and presentation
problems that this project does not attempt, and the two data ones (a recorder and a data manager)
are already covered by the catalog and the catalog CLI.

**Also not adopted, with reasons.** `ArrayManager` and its technical analysis wrappers, because the
indicator library is in Rust and benchmarked. `BarGenerator`, because bar aggregation here is
strictly richer, including bar from bar and imbalance bars. The paper trading account, because the
sandbox execution client already provides a local matching simulation. The trading day based time to
expiry convention in the options application, because calendars are data here.

## 8. Recommended order

Ordered by value against risk, not by size.

1. **D3, the periodic result frame and four statistics.** Contained, purely additive, immediately
   useful to users, and it forces the realised and unrealised split to be stated precisely, which is
   valuable on its own.
2. **D1, the risk caps.** Small, and the caps are the ones a live system is most likely to miss. The
   reset boundary and the duplicate window need care, which is why it is second and not first.
3. **D8, additional search strategies with a memo cache.** Cheapest accepted item; depends only on
   what already exists.
4. **D2, passive and touch aware execution algorithms.** Highest user visible value of the accepted
   items, and the one with the most real design work: churn bounds, denied child handling, and a
   regression scenario per algorithm.
5. **D6, notification sinks.** Independent of everything else, useful, and can land at any time.
6. **D4, American exercise and the volatility surface.** The largest item, and the one where
   correctness is hardest to demonstrate, so it should follow the cheaper items and arrive with
   reference values from an independent implementation.
7. **D5, the factor pipeline.** Research only, largest design surface, and the only accepted item
   whose shape is exploratory rather than settled. Deliberately last.

## 9. Decisions

| Decision | Scope                                   | Outcome                                                                                                                                                                                                                       |
| -------- | --------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D1       | Risk caps                               | Adopt an active order cap, a rolling repeated request guard, and per instrument and total count caps with an explicit reset boundary, in the Rust risk engine                                                                 |
| D2       | Execution algorithms                    | Adopt an iceberg, a quote pegged and a sniper algorithm behind the existing macro, with bounded churn and denied child tolerance; do not adopt a stop algorithm                                                               |
| D3       | Result frame and statistics             | Adopt a periodic result frame with a realised and unrealised split and cost columns, and the drawdown duration, exponentially weighted Sharpe, turnover, commission and slippage statistics; do not adopt the composite ratio |
| D4       | Options analytics                       | Adopt an early exercise model with an American and European declaration per instrument, and a fitted volatility surface; adopt the parity implied forward as a diagnostic                                                     |
| D5       | Factor pipeline                         | Adopt the shape as a research capability outside the engine, with a compiled expression tree and numeric verification; do not adopt the evaluated string DSL                                                                  |
| D6       | Notifications                           | Adopt a minimal sink trait with email and webhook sinks, coalescing and a bounded queue                                                                                                                                       |
| D7       | Restart recovery and parameter metadata | No change; the existing explicit state contract and declared parameter space are retained                                                                                                                                     |
| D8       | Optimization search                     | Adopt random and evolutionary search strategies behind the `SearchStrategy` protocol, with a digest keyed evaluation cache                                                                                                    |

Cross-cutting constraints that apply to every accepted item:

- **United States only.** No accepted item may depend on a foreign market rule, and the options work
  must not import a foreign calendar or settlement convention.
- **No foreign market machinery may be introduced as a side effect.** In particular, the count caps
  in D1 must not be expressed in terms of open and close offset semantics.
- **Verification.** An accepted item is not complete without a test that fails without it, and the
  numerical items (D3, D4, D5) require an independent reference rather than a self consistent
  expectation.

## 10. Remaining open questions

1. **The reset boundary for D1.** A trading day is not the same interval for every venue, and this
   project does not require one. A rolling window with an explicit duration, defaulted from the
   session calendar where one exists, is the likely answer, but it changes the meaning of the cap
   and needs to be decided before implementation.
2. **Where the periodic frame in D3 lives.** It could be produced by the backtest engine, by the
   analyzer, or by a separate reducer over account state and fills. Producing it in the analyzer
   keeps it available to live runs as well, which is probably the right answer, but it implies the
   analyzer must see fills rather than only returns.
3. **Whether the realised and unrealised split belongs in the frame or in the statistics.**
   Both are defensible; the frame is the more general choice.
4. **The volatility surface's interpolation scheme in D4.** Spline fitting is what VeighNa does, but
   arbitrage free alternatives exist and the choice affects every downstream number.
5. **Whether the D5 factor layer belongs in this repository** or in a sibling package that depends on
   it, given that it has no runtime dependency from the engine.
6. **Whether the risk caps in D1 and the algorithms in D2 should share a configuration surface**, or
   whether the caps stay in the risk engine configuration and the algorithms take parameters at
   submit time as TWAP does today.
