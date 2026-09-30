# VeighNa lesson review: implementation record

Companion to [`vnpy_lessons_design.md`](vnpy_lessons_design.md). Section numbers there are cited as
`design 6 L1` and so on.

## 1. Status and scope

**No production code was changed by this review.** The instruction that authorised it permits code
changes only for defects, and section 5.1 records the defect checks that were performed and their
outcome. What follows is therefore an evidence record and a specification, not a delivery record.

The accepted items in `design 9` are **not implemented**. Each is specified in section 4 below with
the mechanism it adopts, the parameters and arithmetic taken from VeighNa, the pitfalls found in
VeighNa's implementation, and the acceptance criteria and verification it would need. Nothing in
this document should be read as a statement that any of it works.

## 2. Probe provenance and reproduction

| Source                                       | Revision probed                                                                                                    |
| -------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| `https://github.com/vnpy/vnpy`               | `fa5206fe63836f3f8cd1ebd7168fbd19a5e2ff09`, working tree declares `__version__ = "4.4.0"` at `vnpy/__init__.py:24` |
| `https://github.com/vnpy/vnpy_ctastrategy`   | `main`                                                                                                             |
| `https://github.com/vnpy/vnpy_ctabacktester` | `main`                                                                                                             |
| `https://github.com/vnpy/vnpy_algotrading`   | `main`                                                                                                             |
| `https://github.com/vnpy/vnpy_riskmanager`   | `main`                                                                                                             |
| `https://github.com/vnpy/vnpy_optionmaster`  | `main`                                                                                                             |

Reproduction: `git clone --depth 1 <url>` for each source into a directory outside the tracked tree.
The probe used `target/vnpy_probe`, which is ignored by the repository's `*target/` rule, and the
directory was removed after this record was written, so the citations below reference upstream
paths, not local ones.

External metadata was read from the GitHub API rather than from secondary commentary:
`api.github.com/repos/vnpy/vnpy` (45,667 stars, 12,511 forks, MIT, created 2015-03-02, last push
2026-09-13) and `api.github.com/repos/vnpy/vnpy/releases` (4.0.0 2025-03-28, 4.1.0 2025-06-17,
4.2.0 2025-11-01, 4.3.0 2025-12-24, 4.4.0 2026-05-14). Release notes are cited below as evidence of
which mechanisms the maintainers consider load-bearing and which they have had to repair.

## 3. Verified state of this repository

Each row is a claim the design document relies on, with the evidence that establishes it.

| Claim                                                                                                                                                                                       | Evidence                                                                                                                                                                                                                                                                                                                        | How verified               |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------- |
| The risk engine validates price and quantity precision, quantity bounds, notional, reduce only, instrument existence and trading state, and denies with typed reasons                       | `crates/risk/src/engine/mod.rs:1683` (`PricePrecisionExceedsMaximum`), `:1693` (`PriceNotPositive`), `:1712` (`QuantityPrecisionExceedsMaximum`), `:1726` (`QuantityExceedsMaximum`), `:1736` (`QuantityBelowMinimum`), `deny_command` at `:1748`                                                                               | Read directly              |
| The risk engine accepts exactly five command kinds and logs an error for any other                                                                                                          | `crates/risk/src/engine/mod.rs:559` with the dispatch arms at `:565-572` (`SubmitOrder`, `SubmitOrderList`, `ModifyOrder`, `ModifyOrders`, `QueryAccount`)                                                                                                                                                                      | Read directly              |
| The risk configuration is three limits plus bypass, venue and debug flags                                                                                                                   | `crates/risk/src/engine/config.rs:41-67` (`bypass`, `max_order_submit: RateLimit`, `max_order_modify: RateLimit`, `max_notional_per_order: AHashMap<InstrumentId, Decimal>`, `full_position_exit_venues: AHashSet<Venue>`, `debug`)                                                                                             | Read directly              |
| There is no active order count, no repeated request guard and no submission or cancellation count cap                                                                                       | No occurrence of an order count concept in `crates/risk/src/`; the only duplicate concept is client order id idempotency at `crates/execution/src/engine/mod.rs:2071`                                                                                                                                                           | Search                     |
| Exactly one execution algorithm ships                                                                                                                                                       | `crates/trading/src/algorithm/` contains `config.rs`, `core.rs`, `mod.rs`, `twap.rs`; the trait is at `mod.rs:103`, the macro instantiates `TwapAlgorithm` at `twap.rs:115`                                                                                                                                                     | Directory listing and read |
| TWAP accepts market orders only, takes `horizon_secs` and `interval_secs`, floors the child size at instrument precision, carries the remainder as a final slice and spawns linked children | `crates/trading/src/algorithm/twap.rs:127` (order type check), `:224` and `:237` (floor and remainder), `:301-322` (scheduled sizes and the sum check), `:370` and `:437` (`spawn_market` on the initial slice and on each timer tick), `:382` (interval timer)                                                                 | Read directly              |
| There are 34 built-in statistics computed from returns or PnL series                                                                                                                        | `crates/analysis/src/statistics/` contains 34 modules besides `mod.rs`                                                                                                                                                                                                                                                          | Directory listing          |
| The backtest result exposes statistics as per currency PnL, return and general maps, with no periodic frame                                                                                 | `crates/backtest/src/result.rs:111-113` (`stats_pnls`, `stats_returns`, `stats_general`)                                                                                                                                                                                                                                        | Read directly              |
| There is no drawdown duration, exponentially weighted Sharpe, turnover, commission or slippage statistic                                                                                    | None of those names occurs in the 34 modules                                                                                                                                                                                                                                                                                    | Directory listing          |
| Black-Scholes and Black-76 are available through the cost of carry term, with implied volatility and greeks, and a Halley refinement step                                                   | `crates/model/src/data/black_scholes.rs:268` (`compute_greeks`), `:304-327` (`compute_iv_and_greeks`, documented as a refinement rather than a solver), `crates/model/src/data/greeks.rs:193` (`imply_vol`), `:239` (`imply_vol_and_greeks`), `:261` (`refine_vol_and_greeks`), using the `implied_vol` crate at `greeks.rs:23` | Read directly              |
| Portfolio greeks and a yield curve exist                                                                                                                                                    | `crates/model/src/data/greeks.rs:507` (`PortfolioGreeks`), `:631` (`YieldCurveData`), `:304` (`GreeksData`)                                                                                                                                                                                                                     | Read directly              |
| Option chain data and an ATM tracker exist                                                                                                                                                  | `crates/model/src/data/option_chain.rs` (834 lines), `crates/data/src/option_chains/` (`aggregator.rs`, `atm_tracker.rs`, `manager.rs`, `handlers.rs`)                                                                                                                                                                          | Directory listing          |
| There is no early exercise model and no volatility surface                                                                                                                                  | No occurrence of a binomial, trinomial, spline or surface concept in the pricing modules                                                                                                                                                                                                                                        | Search                     |
| Bar aggregation is richer than the comparison assumes and supports bar from bar and historical warm-up                                                                                      | `crates/data/src/aggregation.rs:69` (`BarAggregator`), `:148` (`BarBuilder`), `:300` (`update_bar`), `:1498` (`TimeBarAggregator`), `:1811` (`set_historical_events_internal`); `BarAggregation` includes `Week` at `crates/model/src/enums.rs:310`                                                                             | Read directly              |
| Strategy state persistence exists as an explicit byte state contract                                                                                                                        | `crates/trading/src/python/strategy.rs:392` and `:404` (dispatch), `:1183` and `:1188` (`on_save`, `on_load`)                                                                                                                                                                                                                   | Read directly              |
| The optimization package declares a search protocol, ships grid search only, and canonicalises parameters to a digest                                                                       | `python/nautilus_trader/optimization/search.py:37` (`SearchStrategy`), `:62` (`GridSearch`), `space.py:47` (`canonical_json`), `:74` (`digest_of`), `:129` (`Parameter`), `:158` (`ParameterSpace`), `:97` (`Experiment`)                                                                                                       | Read directly              |
| There is no notification or alert abstraction                                                                                                                                               | No occurrence of a notification concept in `crates/model/src`, `crates/trading/src` or `crates/common/src`                                                                                                                                                                                                                      | Search                     |

## 4. Mechanism specifications

### 4.1 D1: pre-trade risk caps

**What VeighNa does.** Rules are objects behind `RuleTemplate` (`vnpy_riskmanager/template.py`),
discovered by scanning a `rules` directory, instantiated with a persisted settings dictionary,
evaluated in order on the send path, and short circuited on the first rejection
(`vnpy_riskmanager/engine.py:129-130`, which replaces `MainEngine.send_order`).

Exact conditions and defaults, read from source:

- `active_order_rule.py`: rejects when `active_order_count >= active_order_limit` (default 50). The
  count is `len(active_orders)`, maintained from `on_order` by adding orders for which
  `order.is_active()` and removing those which are no longer active.
- `daily_limit_rule.py`: rejects on any of six counters reaching its limit. Defaults are 20,000
  orders, 10,000 cancels and 10,000 trades in total, and 2,000 orders, 1,000 cancels and 1,000
  trades per instrument. Orders and cancels are counted once each by order id sets, trades once each
  by trade id set.
- `duplicate_order_rule.py`: increments a per key counter on *every* check and rejects when it
  reaches 10. The key is `symbol|order_type|direction|offset|volume@price`.
- `order_size_rule.py`: rejects when volume exceeds 500, or when a limit order's
  `volume * price * contract_size` exceeds 1,000,000.
- `order_validity_rule.py`: rejects when the instrument is unknown, when the price is not a multiple
  of the price increment, or when volume is above `max_volume` or below `min_volume`.

**Pitfalls in that implementation, to be avoided.**

- The "daily" counters never reset. `on_init` is the only initialiser and no timer or session
  boundary resets them, so they are process lifetime counters carrying a period's name.
- The duplicate counter is cumulative and never decays, so once a request shape has been seen the
  limit number of times it is rejected forever, even after the condition that caused the repetition
  is long gone.
- Rejection is a boolean. The reason is a log line, and the caller receives an empty order id
  (`vnpy_riskmanager/engine.py:184-190` delegates to the original method only on success).
- Price alignment is checked with `req.price % pricetick` against a `1e-6` tolerance on floating
  point prices, which is not reliable for ordinary US equity prices.
- The checks run in Python on the per order path, which is why the rules have Cython twins and a
  micro benchmark harness (`script/benchmark_performance.py`).

**Specification to implement.**

1. Configuration additions to `RiskEngineConfig`, following the existing naming and validation
   style, in `crates/risk/src/engine/config.rs`: a maximum number of concurrently active orders
   (global and optionally per instrument), and count caps over a rolling window for submits, cancels
   and fills, per instrument and in total.
2. The count state lives in the risk engine, updated from the order and fill events it already
   receives, and is never a plugin's private state.
3. The window is a rolling duration, not a calendar day, because a venue independent day boundary
   does not exist in this project. The default duration is a decision recorded in `design 10`.
4. The repeated request guard keys on a canonical request identity, excluding the client order id,
   and counts within the rolling window only.
5. Denials use new `OrderDeniedReason` variants carrying the observed count and the limit, so the
   denial is observable and testable through the existing typed path.
6. Cancels must remain allowed when a cap is reached; a cap that prevents reducing exposure is
   wrong. This is also where the VeighNa design is silent, since it does not intercept cancel
   commands at all.
7. Nothing in the implementation may express a cap in terms of open and close offsets.

**Acceptance and verification.** Each cap needs a test that fails without it: a submit at the limit
is denied with the typed reason; a cancel that reduces the active count releases capacity; a partial
fill does not double count; the window expiry admits a new order; a restart with `load_state` does
not resurrect stale counters. Cost should be measured the way the repository measures hot paths,
with a criterion benchmark alongside `crates/risk/`.

### 4.2 D2: passive and touch aware execution algorithms

**What VeighNa does.** `AlgoTemplate` (`vnpy_algotrading/template.py`) is a state machine over
`RUNNING`, `PAUSED`, `STOPPED` and `FINISHED`, driven by ticks, a one second timer, orders and
trades, tracking a working order set, a traded quantity and a volume weighted traded price, with
`finish()` and `stop()` cancelling all working orders. Child orders are always plain limit orders.

- TWAP: `default_setting` is `{"time": 600, "interval": 60}` seconds. The child size is
  `volume / (time / interval)` rounded to the instrument minimum volume
  (`algos/twap_algo.py:44-46`). The timer counts down, cancels all working orders, and re-sends only
  when the touch is reachable (`ask_price_1 <= price` for a buy). The algorithm finishes when
  `total_count >= time`.
- Iceberg: `display_volume` and `interval`. One child is kept working; when it is no longer active
  the next is sent for `min(volume - traded, display_volume)`. If the market crosses the order price
  while it is working, the order is cancelled on the assumption it was missed.
- Sniper: on each tick, if the touch is reachable the algorithm sends a limit order for
  `min(volume - traded, size at the touch)`, and cancels on the next tick before re-attempting.
- BestLimit: `min_volume` and `max_volume`. It joins the passive touch on each tick with a child size
  drawn uniformly from the range, and cancels and re-quotes whenever the touch price moves. It
  validates its parameters at construction and finishes immediately if they are invalid.
- Stop: watches the last price against a fixed trigger, adds `price_add` to the resulting limit
  price, and clamps to the exchange price limit.

**Pitfalls in that implementation.**

- Cancel and re-send on every slot or every touch change is a message rate hazard, and nothing in
  VeighNa bounds it: the rate limits in the risk manager are self contained and the algorithms know
  nothing about them.
- Completion depends on the timer having run `time` times rather than on the traded quantity, and
  the traded quantity check `traded >= volume` is compared with floats.
- Child orders are tracked by a single order id variable in most algorithms, so a rejected or
  partially filled child is handled only by an `is_active` test.
- There is no persistence, so a restart abandons every algorithm mid sequence.
- Ticks are assumed to have a populated top of book; a subscription without depth is not handled.

**Specification to implement.** Three algorithms in `crates/trading/src/algorithm/`, each
instantiated through `nautilus_execution_algorithm!` and taking parameters at submit time through
`exec_algorithm_params`, as TWAP does today:

1. **Iceberg**: parameters `display_size` and `requote_secs`. Maintain at most one working child;
   send the next when the previous is terminal, for `min(remaining, display_size)` with
   `display_size` floored to instrument precision and never below the minimum quantity; on a
   crossing market, cancel and re-quote only if a `requote_secs` interval has elapsed since the last
   send.
2. **Quote pegged**: parameters `pegging` (passive or join) and `requote_secs`. Join the near or far
   touch, re-quote only when the touch has moved by at least one price increment per
   `requote_secs`, and never at a price that crosses the book.
3. **Sniper**: parameters `limit_price` and `max_children`. Sweep at the limit only when the touch is
   reachable, and cap each child by the displayed size at the touch, with a hard bound on the number
   of children that may be outstanding or have been sent in the sequence.

Requirements that apply to all three: the sequence completes when the remaining quantity is zero or
the deadline passes; a denied child is not retried indefinitely and must not leave the algorithm in
a state where it neither quotes nor completes; child orders are spawned through the engine so parent
and child are linked as TWAP does; and the churn bound must be checked against the risk engine's own
limits rather than assumed.

**Acceptance and verification.** One regression scenario per algorithm over a deterministic book,
pinned by the existing regression machinery, plus a unit test for each boundary: a partial fill, a
child denied by the risk engine, a touch that moves every tick, and a deadline that expires with
quantity remaining. The churn claim must be evidenced by counting submitted and cancelled child
orders in the scenario output, not asserted.

### 4.3 D3: periodic result frame and statistics

**What VeighNa does.** `DailyResult` (`vnpy_ctastrategy/backtesting.py:1058`) holds the date, close
price, start and end position, the day's trades, trade count, turnover, commission, slippage, the
trading PnL against the previous close, the holding PnL against the day close, the total and the net
PnL. `calculate_result` carries `pre_close` and `start_pos` forward across days and builds a frame
indexed by date (`:251-295`). `calculate_statistics` (`:296-525`) then computes 28 keys:

- counts: total days, profitable days, losing days;
- equity: ending balance, maximum drawdown, maximum drawdown percentage, maximum drawdown duration in
  calendar days, taken as the interval from the equity peak preceding the trough to the trough;
- PnL: total and mean per day net PnL;
- costs: total and mean per day commission, slippage, turnover and trade count;
- returns: total return, annual return, mean daily return, return standard deviation, where the
  daily return is the log of the balance ratio against a baseline of the initial capital;
- ratios: Sharpe from the mean daily return against a daily risk free rate scaled by the square root
  of the annualisation factor, an exponentially weighted Sharpe using a configurable halflife,
  a return to drawdown ratio, and a composite ratio;
- and a final pass that replaces non finite values with zero.

**Pitfalls in that implementation.**

- Every undefined statistic becomes zero (`:523`, `np.nan_to_num`), so a Sharpe ratio that cannot be
  computed is indistinguishable from a Sharpe ratio of zero.
- Returns are computed from the cumulative balance against the initial capital, so a run whose
  balance reaches zero has returns forced to zero by construction.
- The frame exists only inside the backtester; a live run has no equivalent.

**Specification to implement.**

1. A periodic reduction over what the engine already tracks, with one row per calendar day, ISO
   week, or month: opening and closing equity, net PnL, realised PnL, unrealised PnL, commission,
   slippage, turnover, trade count, and the drawdown against the running equity peak.
2. The realised and unrealised split is defined precisely once, in the frame, and the PnL statistics
   become views over the frame plus the existing returns series.
3. Four new statistics in `crates/analysis/src/statistics/`: maximum drawdown duration, an
   exponentially weighted Sharpe with a configurable halflife, total turnover, and total
   commissions; total slippage if the fills carry enough information to define it unambiguously.
4. **Undefined is not zero.** A statistic that cannot be computed returns the module's existing
   not available state, and the objective layer continues to treat a missing metric as an error, as
   it does today. This is a deliberate divergence from VeighNa and is recorded as such.
5. The composite ratio is not implemented.

**Acceptance and verification.** A regression scenario that asserts frame rows against a hand
computed expectation for a small deterministic run, including a day with no trades, a day with an
open position marked to market, and a run that ends flat. The four statistics each need a value
checked against an independent computation for the same inputs, including the undefined case.

### 4.4 D4: American exercise and a volatility surface

**What VeighNa does.** `pricing/black_scholes.py`, `pricing/black_76.py` and
`pricing/binomial_tree.py` implement the three models as stateless functions, with a Newton-Raphson
implied volatility solver shared across them, and a trading day convention in `time.py` (a 240 day
year against the Shanghai exchange calendar). `base.py` holds `OptionData` with size scaled greeks,
`ChainData` grouping by expiry and strike, and `PortfolioData` aggregating across the chain, plus a
put call parity correction that derives the implied forward from the underlying and a cubic spline
fit through the implied volatility curve. `algo.py` implements a quoting algorithm that converts a
volatility spread into a price spread through vega.

**Pitfalls in that implementation, and the maintainers' own evidence about this class of code.**

- The release notes for 4.3.0 record a fix for implied volatility convergence on deep out of the
  money options, and a fix for a theta formula error in Black-76. Both are formula and solver
  defects of exactly the kind that a self consistent test suite does not catch.
- The pricing verification script (`script/test_model.py`) prints Python and Cython outputs for a
  human to compare; it asserts nothing.
- The time to expiry convention and the calendar are exchange specific and are excluded here.

**Specification to implement.**

1. The energy to spend is on verification, not on new formulas. Our Black-Scholes and Black-76
   implementations and the implied volatility solver already exist and are benchmarked, so the work
   is an early exercise model plus the properties that make it checkable.
2. An early exercise model (a binomial tree with a configurable step count, or a finite difference
   scheme if preferred) selected by an explicit early exercise declaration on the option instrument,
   defaulting to the market convention for the instrument class. US listed equity options are
   American and US index options are European, and the model must be selected from the instrument
   rather than assumed.
3. A diagnostic that derives the implied forward of the underlying from put call parity across a
   chain, exposed as a value a caller can compare against the market's forward, and used to sanity
   check the inputs to greeks.
4. A surface over the chain with a documented fitting scheme, exposing an implied volatility for a
   strike and expiry that is not quoted, and reporting a fit quality measure rather than silently
   extrapolating.
5. Verification against an independent reference: published values, or a second implementation in
   the repository that shares no code with the first, for a set of cases including deep in the money
   and deep out of the money strikes, zero and high dividend yields, zero and high rates, and
   expiry. A self consistent expectation is not sufficient, on the evidence of the two defects
   above.

**Acceptance and verification.** A table of reference values checked into the repository with the
source of each value recorded, a test that fails if the early exercise declaration is ignored, and a
test for the surface that fails when the fit is asked to extrapolate outside its supported range
without reporting it.

### 4.5 D5: factor pipeline

**What VeighNa does.** `vnpy/alpha/dataset/utility.py` defines a `DataProxy` wrapper over Polars
expressions with operator overloading, evaluates factor definitions written as strings with Python's
`eval`, and exposes `register_functions` for custom operators. `ts_function.py` provides rolling per
symbol operators, `cs_function.py` cross sectional operators over the date axis, `math_function.py`
arithmetic and comparison helpers, and `ta_function.py` technical analysis through a pandas index
bridge. `dataset/template.py` assembles features, a label and a processor chain and computes them
over segments with multiple processes; `dataset/processor.py` applies NaN, infinity and
normalisation processors separately on the inference and learning sides. `model/template.py` defines
`fit`, `predict` and `detail`, with Lasso, LightGBM and multi layer perceptron implementations.
`dataset/datasets/alpha_101.py` and `alpha_158.py` declare the WorldQuant and Qlib factor sets.
`strategy/backtesting.py` replays bars for signal strategies with a daily mark to market pass.

**Pitfalls in that implementation.**

- Factor definitions are strings, evaluated at runtime with `eval`.
- `tests/test_alpha101.py` asserts only that the result contains a `data` column, in six places, so
  nothing verifies that the factors are numerically correct.

**Specification to implement, as a research capability outside the engine.**

1. A compiled expression tree with a typed node set, a parser for a plain text surface, and a
   canonical serialisation that digests to a stable identifier, so a factor definition can be pinned
   by a regression scenario and appear in an experiment digest.
2. Time series operators per instrument (shift, rolling mean, standard deviation, correlation,
   rank, regression residual) and cross sectional operators per timestamp (rank, scale, sum,
   neutralise against a grouping key).
3. A dataset contract producing a point in time panel from the catalog: one row per instrument and
   timestamp, feature columns, a label column, and an explicit declaration of the learning and
   inference sides of every transform so that no future information can leak into a feature.
4. Numeric verification against an independent reference for a factor set, rather than the existence
   check that VeighNa performs.
5. No second execution model. A signal strategy rebalances through the normal strategy and
   execution path.

**Acceptance and verification.** A scenario that pins a small factor set numerically for a fixture
catalog, a test that detects deliberate label leakage, and a test that the digest of a factor
definition is stable across processes.

### 4.6 D6: notification sinks

**What VeighNa does.** 4.4.0 added `MainEngine.send_notification` over an email engine and a chat
engine, with a per channel interval so that messages inside the interval are coalesced.

**Specification to implement.** A sink trait with a minimal contract (send a titled message with a
severity), configuration listing enabled sinks and their settings, an implementation for SMTP email
and for a generic HTTP webhook, a bounded queue with a drop policy that is observable, per sink
coalescing over a configurable interval, and no vendor SDK as a dependency of the core crates.

**Acceptance and verification.** A test per sink against a local server stub, a test that coalescing
bounds the send rate, and a test that a saturated queue drops rather than blocks the caller.

### 4.7 D8: search strategies and a memo cache

**What VeighNa does.** `vnpy/trader/optimize.py` runs a brute force grid over a process pool and a
genetic search using DEAP with configurable population, generation and operator parameters, over a
process pool with a manager dictionary caching evaluations by parameter vector. 4.1.0 made the
genetic hyperparameters user controllable.

**Specification to implement.** A random search and an evolutionary search implementing the existing
`SearchStrategy` protocol, both deterministic under a seeded generator, both recording every
evaluated parameter vector and its result in the existing persistence so that a resumed run skips
them, and both reporting the evaluated fraction and the best result through the existing report.
The memo cache is the canonical parameter digest that already exists.

**Acceptance and verification.** A test that a seeded run reproduces the same sequence of candidate
vectors and the same best result, a test that a resumed run does not re-evaluate a cached vector
(evidenced by the evaluation count), and a test that a search with a budget smaller than the space
still reports the best of what it evaluated.

## 5. Defects

### 5.1 In this repository: none found

The review looked for defects in the areas it touched, using the following checks. No defect was
found, so no code was changed.

| Area checked                          | Method                                                                                                                                                            | Result                                                                                                                                                            |
| ------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Risk engine validation coverage       | Read the validation path and the denial reasons, and searched for a quantity or notional check that could be bypassed by a quote quantity or a full position exit | The two bypasses are explicit and documented in the code at `crates/risk/src/engine/mod.rs:1719`; no unintended path found                                        |
| Risk engine command coverage          | Compared the handled command set with the denial path                                                                                                             | Cancel commands are not handled by the design and log an error if routed; this is a design boundary, not a defect, and it is the interaction that D1 must respect |
| TWAP arithmetic                       | Read the floor, remainder and sum verification                                                                                                                    | The scheduled sizes are verified against the parent quantity before submission; no defect found                                                                   |
| Statistics and the objective layer    | Confirmed that a missing metric is an error rather than a zero, which is the behaviour D3 must preserve                                                           | No defect found                                                                                                                                                   |
| Options greeks and implied volatility | Read the solver entry points and their documented limits                                                                                                          | The single step refinement is documented as not being a solver and the solver is delegated to a reviewed crate; no defect found                                   |

### 5.2 In VeighNa: recorded as design warnings

These are recorded because each one is a mistake that the items in section 4 could repeat. They are
not reported upstream; the review does not interact with that project's issue tracker.

| #   | Observation                                                                            | Evidence                                                                                                    | Why it matters                                                                                           |
| --- | -------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------- |
| 1   | Counters named for a period never reset                                                | `vnpy_riskmanager/rules/daily_limit_rule.py` initialises counters only in `on_init`                         | A cap silently becomes a lifetime quota, and an operator cannot reason about when capacity returns       |
| 2   | A cumulative counter permanently blocks a request shape                                | `vnpy_riskmanager/rules/duplicate_order_rule.py:29-40` increments on every check                            | A legitimate repeated order becomes permanently impossible                                               |
| 3   | Rejection is a boolean plus a log line, and the caller sees an empty order id          | `vnpy_riskmanager/engine.py:184-190`                                                                        | The failure is unobservable to the caller and untestable through the contract                            |
| 4   | The risk engine replaces a core method at runtime                                      | `vnpy_riskmanager/engine.py:129-130`                                                                        | Subsystem ordering becomes implicit                                                                      |
| 5   | Price increment alignment is checked with floating point modulo and a `1e-6` tolerance | `vnpy_riskmanager/rules/order_validity_rule.py:24-27`                                                       | False rejections and false acceptances at ordinary prices                                                |
| 6   | Cancel commands are not intercepted by the risk layer                                  | No cancellation interception in `vnpy_riskmanager/engine.py`                                                | There is no bound on cancellation traffic, only on its count                                             |
| 7   | The `WEEKLY` interval falls through to the daily window path                           | `vnpy/trader/utility.py:266-274` dispatches only `MINUTE` and `HOUR`, with the daily window as the fallback | A declared interval silently produces a different interval                                               |
| 8   | Undefined statistics are coerced to zero                                               | `vnpy_ctastrategy/backtesting.py:523`                                                                       | An unavailable risk measure looks like a measured one                                                    |
| 9   | Pricing correctness is verified by printing two implementations for comparison         | `vnpy_optionmaster/script/test_model.py`                                                                    | Formula defects such as the two repaired in 4.2.0 and 4.3.0 are exactly what this fails to catch         |
| 10  | The Alpha101 factor test asserts only that an output column exists                     | `tests/test_alpha101.py` asserts `"data" in result.columns` six times                                       | The factor set is unverified                                                                             |
| 11  | Tick to bar volume accumulates only non negative differences of cumulative volume      | `vnpy/trader/utility.py:255-265`                                                                            | Correct for out of order ticks, but a feed reset is unrecoverable and indistinguishable from zero volume |

## 6. Acceptance criteria and verification, by item

| Item                    | Acceptance                                                                                                                                        | Verification                                                                                          | Status             |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- | ------------------ |
| D1 Risk caps            | Each new cap denies with a typed reason, cancels release capacity, and the window expiry restores it                                              | Unit tests per cap plus a criterion benchmark on the send path                                        | Not implemented    |
| D2 Algorithms           | Iceberg, quote pegged and sniper execute a parent order to completion or deadline with bounded churn and correct behaviour when a child is denied | One regression scenario per algorithm plus boundary unit tests                                        | Not implemented    |
| D3 Frame and statistics | A periodic frame with the realised and unrealised split, and four statistics, computed from engine accounting                                     | Regression scenario against a hand computed expectation, and an independent value check per statistic | Not implemented    |
| D4 Options              | Early exercise is selectable per instrument and priced correctly; a surface interpolates inside its range and reports fit quality                 | Reference value table with recorded provenance and a test that ignores the declaration fails          | Not implemented    |
| D5 Factors              | Factor definitions compile to a digestible tree, the panel leaks no future information, and a factor set matches an independent reference         | Numeric pinning scenario, leakage test, digest stability test                                         | Not implemented    |
| D6 Notifications        | Sinks send, coalesce and drop under saturation without blocking the caller                                                                        | Local server stub tests                                                                               | Not implemented    |
| D7 Restart recovery     | No change                                                                                                                                         | Existing tests                                                                                        | No change required |
| D8 Search               | Seeded searches reproduce, resumed runs skip cached evaluations, and the report reflects the evaluated set                                        | Determinism and resume tests, evidenced by evaluation counts                                          | Not implemented    |

## 7. Explicit non-goals

Not to be implemented, whatever else changes: the offset model and its converter, settlement prices
and settlement based PnL, exchange price limits and clamping, foreign exchange and product
enumerations, the 240 day year and exchange calendar time conventions used by the options
application, regulatory ratio reporting, runtime plugin discovery, monkey-patching core methods,
string factor expressions evaluated with `eval`, non finite statistics coerced to zero, the
composite performance ratio, the desktop GUI and its applications, the web and RPC services, and the
internationalisation layer.

## 8. Outstanding measurements and open items

1. The churn hazard in 4.2 is an argument from code inspection of the VeighNa algorithms and from
   the absence of any bound in their design. It has not been measured on a live venue, and the
   bound chosen for our implementation should be justified by a measurement before it is fixed.
2. The cost of the D1 caps on the send path is unmeasured here. The VeighNa project's need for
   Cython twins and a micro benchmark is evidence that per order Python checks are not free, but it
   is not evidence about Rust.
3. The statistics work in 4.3 needs a decision on where the periodic frame is produced, recorded in
   `design 10` as an open question, before implementation starts.
4. The provenance of the reference values required by 4.4 and 4.5 must be identified during those
   workstreams; a reference whose origin cannot be recorded is not acceptable evidence.
