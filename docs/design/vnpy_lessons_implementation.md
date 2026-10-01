# VeighNa lesson review: implementation record

Companion to [`vnpy_lessons_design.md`](vnpy_lessons_design.md). Section numbers there are cited as
`design 8 L1` and so on.

## 1. Status and scope

### 1.1 Defect-only mandate

**No production code was changed by this review.** The instruction that authorised it permits code
changes only for defects, and section 5.1 records the defect checks that were performed and their
outcome. What follows is therefore an evidence record and a specification, not a delivery record.

The accepted items in `design 12` are **not implemented**. Each is specified in section 4 below with
the mechanism it adopts, the parameters and arithmetic taken from VeighNa, the pitfalls found in
VeighNa's implementation, and the acceptance criteria and verification it would need. Nothing in
this document should be read as a statement that any of it works, except for the items recorded as
implemented in section 1.3.

The mandate was later extended to implement the accepted items one at a time, which is what the
defect-only instruction had previously prevented. Section 1.3 is the delivery log for that work, and
the status column in section 6 records per item what is implemented and what was verified. An item
that is not marked implemented remains a specification.

### 1.2 Revision history

The design document was revised in response to review feedback. This record was revised with it, and
the following changed. All eight original items (D1 to D8) were retained, and D7 remained a no-change
decision.

| Change                 | Detail                                                                                                                                                                                                                                           |
| ---------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Framing                | The review is stated as capability harvesting with this project's architecture as the centre of gravity, and every decision now records whether VeighNa supplies the capability or the comparison merely exposed it                              |
| Taxonomy added         | Capabilities are classified into layers A to H with an owner in this project, so they cannot become one subsystem                                                                                                                                |
| Invariants added       | Canonical state ownership with a single authority per information type, and a prohibition on a second authoritative copy                                                                                                                         |
| D1 strengthened        | Caps are dimensioned by scope and window rather than hard coded, and denials must carry a structured decision record                                                                                                                             |
| D2 strengthened        | Parent and child ownership is stated, and the algorithms are preceded by a shared execution intent                                                                                                                                               |
| D3 strengthened        | The frame is specified as a `PerformancePeriod` record reduced from the portfolio's existing realised and unrealised accounting                                                                                                                  |
| D4 split               | Into D4A, early exercise and an exercise style per instrument, and D4B, a volatility surface specified down to its filters, coordinates, extrapolation policy and confidence measure                                                             |
| D5 strengthened        | Named Feature, Label and Dataset contracts, with membership and splits, per D13                                                                                                                                                                  |
| D6 strengthened        | Notifications become an event set consumed by a router rather than sinks called directly                                                                                                                                                         |
| D8 strengthened        | The objective and constraints become first class, not only new search strategies                                                                                                                                                                 |
| New decisions          | D11 execution intent, D12 canonical accounting, D13 point in time dataset, D14 execution analytics                                                                                                                                               |
| Corrections            | Portfolio construction (D9) and universe membership (D10) are recorded as mechanisms that already exist in this project rather than as missing capabilities; the adjacent gaps are named and attributed to the comparison rather than to VeighNa |
| Dependency graph added | The layer dependencies and the notification path are drawn explicitly                                                                                                                                                                            |
| Order revised          | Risk before accounting, so that the accounting tests exercise the safety-controlled path                                                                                                                                                         |

A second review round accepted the architecture and asked for nine contract items to be resolved
before implementation. They are resolved as follows, in the design document and mirrored in the
specifications below.

| Item                                       | Resolution                                                                                                                                                                                                                                                                                                     |
| ------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D1 counter and reset semantics             | Counters and decisions are separate types; a window is a rolling duration over event timestamps, so there is no reset boundary to define                                                                                                                                                                       |
| D3 period trigger and field taxonomy       | Fields grouped as accounting, trading activity, exposure and derived performance; emitted by an interval trigger shared by live and backtest                                                                                                                                                                   |
| D11 intent versus policy versus parameters | Three types; an algorithm refuses a policy it cannot honour, and TWAP migrates onto the pair                                                                                                                                                                                                                   |
| D4B surface methodology                    | Total implied variance against log moneyness, monotone and convexity preserving, calendar monotonic, flat extrapolation, arbitrage validation as the gate, parameterised family as fallback                                                                                                                    |
| D5 and D13 membership representation       | Membership history is stored data; a re-evaluated rule cannot reproduce delisted instruments                                                                                                                                                                                                                   |
| D8 validation scheme                       | The validation scheme joins the run description, and a held-out evaluation is never scored by the search                                                                                                                                                                                                       |
| D14 benchmark timestamps                   | Every metric declares its reference price, denominator and reference timestamp                                                                                                                                                                                                                                 |
| Research provenance                        | A new design invariant: every research value resolves to its dataset, feature, membership and experiment digests, its as-of timestamp and its source version                                                                                                                                                   |
| Parent and child conservation              | A new design invariant: parent target equals executed plus remaining plus cancelled unfilled, and child submissions never exceed the parent without a declared overshoot policy                                                                                                                                |
| Improvements                               | The dependency statement now distinguishes a code dependency from a data flow; the exercise model names Cox-Ross-Rubinstein as the first implementation; D3's contract may be developed while D1 is implemented; wording on structural claims and on the factor pipeline versus domain score engines corrected |

### 1.3 Implementation log

Implementation of the accepted items is tracked here, one row per item, with the verification that
was actually exercised. An item is only marked implemented when the acceptance criteria in section 6
have a passing test or benchmark behind them.

| Item | Change                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    | Verification                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| ---- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D1   | `RiskEngineConfig.count_caps` dimensions a cap over a scope, a metric and a rolling window. The counters and the decision records live in the risk engine, keyed per rule and per concrete scope, and a repeated request cap adds the canonical request identity (the instrument, side, order type, quantity and price, excluding the client order id) so each request shape holds its own counter. `OrderDeniedReason` carries the observed count, the limit, the scope and the window, and a cancellation is never refused by a cap. The cap vocabulary is in `nautilus_model::risk`.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Eight acceptance tests over the send path in `crates/risk/tests/risk_engine.rs`: refusal at the limit, the active order count and a cancellation releasing capacity, a partial fill counted once and its void releasing the occurrence, window expiry admitting a new order, a strategy scoped cap not denying a second strategy, a cancel cap gating submits without refusing a cancellation, a repeated request refused while a different request shape is admitted, and a reset clearing the counters. Denial records are asserted field by field, not through the rendered string. The exercised verification: `cargo clippy --locked -p nautilus-model -p nautilus-risk --all-targets --features python -- -D warnings` clean; `cargo nextest run --locked -p nautilus-model -p nautilus-risk --lib --features python` 3917 passed; `cargo nextest run --locked -p nautilus-risk --test risk_engine -E 'test(/cap/)'` 20 passed; the generated denial table regenerated for the three new codes, so `generated_table_is_in_sync` passes. Cost, from `cargo bench --locked -p nautilus-risk --bench caps`: the allow path over five configured caps whose windows each hold 1,000 occurrences took 6.49 to 6.85 microseconds, and a refusal at the first cap took 63.4 to 66.7 nanoseconds.                                                                                                                             |
| D2   | Three algorithms join TWAP in `crates/trading/src/algorithm/`. `IcebergAlgorithm` hides size with at most one child working, sending the next on terminal for `min(remaining, display_size)` floored to the instrument precision and refused below the size increment or the minimum quantity, and cancelling and re-quoting a child the market has crossed only once `requote_secs` has elapsed since it was sent. `QuotePeggedAlgorithm` joins the near touch (`pegging=passive`) or quotes one increment inside the opposite touch (`pegging=join`), carries the primary's whole remaining quantity in one child, re-quotes only when the touch has moved by at least one price increment and `requote_secs` has elapsed, and never at a price that crosses the book or is off the tick grid. `SniperAlgorithm` sweeps at its own `limit_price` only when the touch is reachable, caps each child at the displayed size at the touch and never sends more than `max_children`. All three spawn children through the core with `reduce_primary`, so parent and child are linked as TWAP's are; a terminal child restores its unfilled quantity to the primary; a denied or rejected child is not retried, the sequence completing with the primary carrying the refusal; the sequence completes on zero remaining or on the policy horizon deadline; and each exposes `counts(primary_id)` as the children submitted and cancelled, which is the churn evidence. Each declares the policy parts it can honour and refuses the rest with a denial naming the field, including the price limit for the sniper, whose sweep price is its own parameter. The three are registered for Python use in `crates/backtest/src/python/engine.rs`. | Unit tests in each algorithm's file, over the same helpers TWAP's tests use: the parts each refuses and the malformed parameters, the quote price for both `pegging` values, the display size floored and refused below the increment, the child capped by the displayed size at the touch, the re-quote gate (a touch that moves every tick still sends one child per terminal event), a terminal child sending exactly one successor, a denied child denying the primary, a deadline with quantity remaining cancelling and completing, and the churn counts. Engine level scenarios in `crates/backtest/tests/integration/backtest_engine.rs` drive one algorithm each over a deterministic book and assert conservation from the cache: the executed quantity plus the quantity still outstanding equals the parent target, and the quantity a cancel released is reported as the churn evidence rather than as a third additive term. The three term form of the conservation invariant in 4.2 holds while a cancel abandons quantity; where the algorithm re-quotes the released quantity instead, it is outstanding again, and counting it as cancelled as well would double count it. Exercised with `cargo clippy -p nautilus-trading --all-targets --features python -- -D warnings` clean, 962 tests in the crate's library suite, and `cargo check -p nautilus-backtest --all-targets --features python` clean. |
| D11  | `ExecutionIntent` and `ExecutionPolicy`, in `crates/trading/src/algorithm/policy.rs`, are the two declarations above the algorithms: the intent is the order's own terms, taken from the order the algorithm receives, and the policy is the constraints and preferences declared in `exec_algorithm_params` (`horizon_secs`, `participation_rate`, `price_limit`, `max_slippage_bps`, `preference`, `urgency`), with a malformed or out of domain field refused and named. An algorithm declares the `PolicyPart`s it can honour through `ExecutionAlgorithm::supported_policy_parts`, which defaults to none, and a declared part it cannot honour is refused with a denial naming the field rather than relaxed. TWAP takes its horizon from the policy, declares the horizon as the only part it can honour, and refuses the other five.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | `policy.rs` tests cover the intent taken from the order, every declared part, every malformed field and its message, the unsupported part named in declaration order, and a passive preference refused for a market intent. `twap.rs` tests refuse each of the five parts TWAP cannot honour with the field named, refuse a malformed policy field before the unsupported part, and reproduce the size schedule for the equivalent pre-policy declaration (three 0.4 slices for 1.2 over 60 seconds in 20 second intervals, the first spawned, the primary reduced to 0.8, the last two scheduled), with every pre-existing schedule, remainder, precision, completion and refusal test unchanged and passing. Exercised with `cargo clippy -p nautilus-trading --all-targets --features python -- -D warnings` clean and 882 tests in the crate's library suite. The price constraint case lands with D2, whose iceberg and quote pegged algorithms honour a price limit by refusing an order whose price, or whose computed quote, breaches it, so no child is sent outside a declared limit.                                                                                                                                                                                                                                                                                                                             |

Not done as part of D1, and recorded as open items in section 8: no cap value or window duration is
shipped as a default, the caps have no Python or live configuration surface yet, and a window that is
a session is not built because the trading calendar is not. The acceptance criterion that a restart
must not resurrect stale counters has no subject: the risk engine persists no state, so there is no
`load_state` for a counter to survive, and the tested guarantee is instead that `reset` clears both
the counters and the decision records.

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

| Claim                                                                                                                                                                                       | Evidence                                                                                                                                                                                                                                                                                                                                                                                              | How verified               |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------- |
| The risk engine validates price and quantity precision, quantity bounds, notional, reduce only, instrument existence and trading state, and denies with typed reasons                       | `crates/risk/src/engine/mod.rs:1683` (`PricePrecisionExceedsMaximum`), `:1693` (`PriceNotPositive`), `:1712` (`QuantityPrecisionExceedsMaximum`), `:1726` (`QuantityExceedsMaximum`), `:1736` (`QuantityBelowMinimum`), `deny_command` at `:1748`                                                                                                                                                     | Read directly              |
| The risk engine accepts exactly five command kinds and logs an error for any other                                                                                                          | `crates/risk/src/engine/mod.rs:559` with the dispatch arms at `:565-572` (`SubmitOrder`, `SubmitOrderList`, `ModifyOrder`, `ModifyOrders`, `QueryAccount`)                                                                                                                                                                                                                                            | Read directly              |
| The risk configuration is three limits plus bypass, venue and debug flags                                                                                                                   | `crates/risk/src/engine/config.rs:41-67` (`bypass`, `max_order_submit: RateLimit`, `max_order_modify: RateLimit`, `max_notional_per_order: AHashMap<InstrumentId, Decimal>`, `full_position_exit_venues: AHashSet<Venue>`, `debug`)                                                                                                                                                                   | Read directly              |
| There is no active order count, no repeated request guard and no submission or cancellation count cap                                                                                       | No occurrence of an order count concept in `crates/risk/src/`; the only duplicate concept is client order id idempotency at `crates/execution/src/engine/mod.rs:2071`                                                                                                                                                                                                                                 | Search                     |
| Exactly one execution algorithm ships                                                                                                                                                       | `crates/trading/src/algorithm/` contains `config.rs`, `core.rs`, `mod.rs`, `twap.rs`; the trait is at `mod.rs:103`, the macro instantiates `TwapAlgorithm` at `twap.rs:115`                                                                                                                                                                                                                           | Directory listing and read |
| TWAP accepts market orders only, takes `horizon_secs` and `interval_secs`, floors the child size at instrument precision, carries the remainder as a final slice and spawns linked children | `crates/trading/src/algorithm/twap.rs:127` (order type check), `:224` and `:237` (floor and remainder), `:301-322` (scheduled sizes and the sum check), `:370` and `:437` (`spawn_market` on the initial slice and on each timer tick), `:382` (interval timer)                                                                                                                                       | Read directly              |
| There are 34 built-in statistics computed from returns or PnL series                                                                                                                        | `crates/analysis/src/statistics/` contains 34 modules besides `mod.rs`                                                                                                                                                                                                                                                                                                                                | Directory listing          |
| The backtest result exposes statistics as per currency PnL, return and general maps, with no periodic frame                                                                                 | `crates/backtest/src/result.rs:111-113` (`stats_pnls`, `stats_returns`, `stats_general`)                                                                                                                                                                                                                                                                                                              | Read directly              |
| There is no drawdown duration, exponentially weighted Sharpe, turnover, commission or slippage statistic                                                                                    | None of those names occurs in the 34 modules                                                                                                                                                                                                                                                                                                                                                          | Directory listing          |
| Black-Scholes and Black-76 are available through the cost of carry term, with implied volatility and greeks, and a Halley refinement step                                                   | `crates/model/src/data/black_scholes.rs:268` (`compute_greeks`), `:304-327` (`compute_iv_and_greeks`, documented as a refinement rather than a solver), `crates/model/src/data/greeks.rs:193` (`imply_vol`), `:239` (`imply_vol_and_greeks`), `:261` (`refine_vol_and_greeks`), using the `implied_vol` crate at `greeks.rs:23`                                                                       | Read directly              |
| Portfolio greeks and a yield curve exist                                                                                                                                                    | `crates/model/src/data/greeks.rs:507` (`PortfolioGreeks`), `:631` (`YieldCurveData`), `:304` (`GreeksData`)                                                                                                                                                                                                                                                                                           | Read directly              |
| Option chain data and an ATM tracker exist                                                                                                                                                  | `crates/model/src/data/option_chain.rs` (834 lines), `crates/data/src/option_chains/` (`aggregator.rs`, `atm_tracker.rs`, `manager.rs`, `handlers.rs`)                                                                                                                                                                                                                                                | Directory listing          |
| There is no early exercise model and no volatility surface                                                                                                                                  | No occurrence of a binomial, trinomial, spline or surface concept in the pricing modules                                                                                                                                                                                                                                                                                                              | Search                     |
| Bar aggregation is richer than the comparison assumes and supports bar from bar and historical warm-up                                                                                      | `crates/data/src/aggregation.rs:69` (`BarAggregator`), `:148` (`BarBuilder`), `:300` (`update_bar`), `:1498` (`TimeBarAggregator`), `:1811` (`set_historical_events_internal`); `BarAggregation` includes `Week` at `crates/model/src/enums.rs:310`                                                                                                                                                   | Read directly              |
| Strategy state persistence exists as an explicit byte state contract                                                                                                                        | `crates/trading/src/python/strategy.rs:392` and `:404` (dispatch), `:1183` and `:1188` (`on_save`, `on_load`)                                                                                                                                                                                                                                                                                         | Read directly              |
| The optimization package declares a search protocol, ships grid search only, and canonicalises parameters to a digest                                                                       | `python/nautilus_trader/optimization/search.py:37` (`SearchStrategy`), `:62` (`GridSearch`), `space.py:47` (`canonical_json`), `:74` (`digest_of`), `:129` (`Parameter`), `:158` (`ParameterSpace`), `:97` (`Experiment`)                                                                                                                                                                             | Read directly              |
| There is no notification or alert abstraction                                                                                                                                               | No occurrence of a notification concept in `crates/model/src`, `crates/trading/src` or `crates/common/src`                                                                                                                                                                                                                                                                                            | Search                     |
| The portfolio is the authority for position and realised and unrealised PnL                                                                                                                 | `crates/portfolio/src/portfolio.rs:186` (`Portfolio`), `:513` (`unrealized_pnls`), `:581` (`realized_pnls`), `:666`, `:712` and `:742` (per instrument), documented in `docs/concepts/accounting.md`                                                                                                                                                                                                  | Read directly              |
| Universe membership exists with static and scheduled rules and change events                                                                                                                | `crates/model/src/universe.rs:182` (`UniverseChange`), `crates/trading/src/universe.rs` (`UniverseDefinition`, `Universe`, static and scheduled rules), `crates/trading/src/python/universe.rs:57` (registered Python class), `crates/common/src/actor/universe.rs:38` (subscription helper), `docs/concepts/universes.md`, `python/tests/regression/cases/universe_membership.py`                    | Read and listing           |
| Target construction sizes one instrument at a time with a risk-scaled weight, and no cross-sectional weighting scheme exists                                                                | `crates/trading/src/target.rs:51-75` (weight definition and the risk over stop distance formula), `:161` (`TargetConstructionConfig`), `:432` (`TargetConstruction`), `:834` (`TargetReconciler`), `crates/trading/src/target_pipeline.rs:85` (`TargetPipeline`), `docs/concepts/target_pipeline.md`; no occurrence of a weighting scheme such as risk parity, inverse volatility or minimum variance | Read and search            |
| A denial carries the trader, strategy, instrument, client order id and a formatted reason string, not structured limit fields                                                               | `crates/model/src/events/order/denied.rs:51-70` (fields including `reason: Ustr`), emitted at `crates/risk/src/engine/mod.rs:1800`                                                                                                                                                                                                                                                                    | Read directly              |
| There is no execution analytics of any kind                                                                                                                                                 | No occurrence of an implementation shortfall, arrival price, adverse selection or fill ratio concept outside the adapters                                                                                                                                                                                                                                                                             | Search                     |
| The event architecture exposes subscription points a notification router could use                                                                                                          | `crates/common/src/msgbus/api.rs:441` (`subscribe_order_events`), `:453` (`subscribe_position_events`), `crates/common/src/actor/data_actor.rs:1597` (`subscribe_signal`)                                                                                                                                                                                                                             | Search                     |

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

Counters and decisions are separate types, and the separation is the extensibility point:

```text
RiskCounter  ->  rule evaluation  ->  RiskDecision  ->  OrderDenied or order accepted
```

A counter is state for one (scope, metric) pair, timestamped, derived from events the risk engine
already receives. A rule holds the limit and the window. A decision is an immutable record of rule,
scope, observed value, limit, window, timestamp and outcome. The counter is never the decision, and
the decision is never a string. Notional, exposure, volatility, drawdown and concentration limits are
later rules over the same pipeline rather than new mechanisms.

1. Configuration additions to `RiskEngineConfig`, following the existing naming and validation
   style, in `crates/risk/src/engine/config.rs`. A cap is not three hard coded counters; it is a
   predicate over a scope, a metric and a window. The scope is one of global, strategy, account,
   instrument, venue, or strategy by instrument. The metric is submit, modify, cancel or fill. The
   concrete caps required are a maximum number of concurrently active orders, which is a count over
   the open order set rather than an event count, and count caps for submits, cancels and fills, at a
   minimum globally and per instrument.
2. The count state lives in the risk engine, updated from the order and fill events it already
   receives, and is never a plugin's private state.
3. The window is a rolling duration evaluated from event timestamps, which resolves the reset
   question by construction: there is no reset boundary to define and no process lifetime
   accumulation, in contrast to VeighNa. Where a window is the session, the boundary comes from the
   trading calendar rather than from a wall-clock day, since a venue independent day does not exist
   in this project. The concrete default durations are an open item in `design 13`.
4. The repeated request guard keys on a canonical request identity, excluding the client order id,
   and counts within the rolling window only.
5. Denials use new `OrderDeniedReason` variants carrying the observed count, the limit, the scope
   and the window, so the denial is observable and testable through the existing typed path.
6. Every denial produces a structured decision record, not only a formatted string. The record
   carries the rule identity, the scope, the observed value, the limit, the window, and the
   timestamp, alongside the trader, strategy, instrument and client order id that `OrderDenied`
   already carries. The string reason is a rendering of that record, not its source, which is the
   difference from the current state where the typed reason is flattened into `reason: Ustr` before
   an observer sees it.
7. Cancels must remain allowed when a cap is reached; a cap that prevents reducing exposure is
   wrong. This is also where the VeighNa design is silent, since it does not intercept cancel
   commands at all.
8. Nothing in the implementation may express a cap in terms of open and close offsets.

**Acceptance and verification.** Each cap needs a test that fails without it: a submit at the limit
is denied with the typed reason; a cancel that reduces the active count releases capacity; a partial
fill does not double count; the window expiry admits a new order; a restart with `load_state` does
not resurrect stale counters. Cost should be measured the way the repository measures hot paths,
with a criterion benchmark alongside `crates/risk/`.

Two further cases follow from the dimensioning. A cap scoped to one strategy must not deny a second
strategy, which is the test that the counters are actually per scope rather than global. And each
denial record must be asserted field by field - rule, scope, observed value, limit, window,
timestamp - because a structured record that is only tested through its rendered string is the
current state, not the intended one.

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

**Ownership, stated before the algorithms.** Adding three algorithms without stating ownership is
how an execution layer becomes a collection of unrelated implementations.

- The **parent order** owns the target quantity, the execution objective and the algorithm state.
- A **child order** owns its venue order id, price, quantity, status and timestamps.
- The **execution algorithm instance** owns scheduling, quoting, cancellation and replacement, and
  the completion decision.

This is also the boundary that section 7.1 of the design requires: child order state is authoritative
in the execution engine and the algorithm holds a schedule, not a second order book. The algorithms
below are written against the execution intent of `design 8 L11` rather than each inventing a
parameter vocabulary, and the existing TWAP migrates to it rather than keeping a second contract.

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

Requirements that apply to all three, the first being the conservation invariant of `design 7.5`:
the parent target always equals executed plus remaining plus cancelled unfilled quantity, and child
submissions never exceed the parent target without an overshoot policy the caller authorised; the
sequence completes when the remaining quantity is zero or the deadline passes; a denied child is not retried indefinitely and must not leave the algorithm in
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

1. A `PerformancePeriod` record, one row per calendar day, ISO week or month, whose fields are
   grouped by what they are rather than presented as a flat bag:
   **accounting** - period bounds, starting equity, ending equity, realised PnL, unrealised PnL,
   commission, fees, slippage;
   **trading activity** - volume, turnover, trade count, winning trades, losing trades;
   **exposure** - open position count, gross exposure, net exposure;
   **derived performance** - net PnL, net return, drawdown and drawdown percentage against the
   running equity peak.
2. The record is a reduction of authorities that already exist, not a second ledger. Realised and
   unrealised PnL per instrument are already maintained by the portfolio
   (`crates/portfolio/src/portfolio.rs:581` and `:513`), so the frame reduces portfolio and fill
   state rather than recomputing PnL from prices. The split is therefore defined once, in the
   portfolio, and the frame attributes it to a period.
3. PnL statistics become views over the frame plus the existing returns series, so that backtest,
   live, portfolio and strategy reporting do not each reconstruct the same numbers.
4. Four new statistics in `crates/analysis/src/statistics/`: maximum drawdown duration, an
   exponentially weighted Sharpe with a configurable halflife, total turnover, and total
   commissions; total slippage if the fills carry enough information to define it unambiguously.
5. **Undefined is not zero.** A statistic that cannot be computed returns the module's existing
   not available state, and the objective layer continues to treat a missing metric as an error, as
   it does today. This is a deliberate divergence from VeighNa and is recorded as such.
6. The composite ratio is not implemented.
7. The period is emitted by an interval trigger rather than a backtest callback: a calendar boundary
   of day, ISO week or month computed from the configuration and the trading calendar, plus an
   explicit flush on shutdown and on demand. A backtest runs the same trigger off the simulated
   clock, so live and backtest produce the same rows for the same period and the same accounting,
   which is the property VeighNa cannot offer because its frame exists only inside the backtester.

**Acceptance and verification.** A regression scenario that asserts frame rows against a hand
computed expectation for a small deterministic run, including a day with no trades, a day with an
open position marked to market, and a run that ends flat. The four statistics each need a value
checked against an independent computation for the same inputs, including the undefined case.

### 4.4 D4A and D4B: early exercise and a volatility surface

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

**Specification, D4A: exercise style and pricing.**

1. The energy to spend is on verification, not on new formulas. Our Black-Scholes and Black-76
   implementations and the implied volatility solver already exist and are benchmarked, so the work
   is an early exercise model plus the properties that make it checkable.
2. An exercise style declared on the option instrument, American or European, defaulted to the
   market convention of the instrument class. United States listed equity options are American and
   United States index options are European, and pricing selects the model from the instrument
   rather than assuming one style for a chain.
3. The first implementation is a Cox-Ross-Rubinstein binomial tree with a configurable step count,
   which is the smallest thing that prices an American option correctly. A tree price is an
   approximation, so the convergence behaviour is documented and the error bounded rather than
   asserted. The pricing interface stays model agnostic, so a finite difference scheme or a more
   general exercise framework can replace the tree without changing callers; the interface is the
   deliverable and the tree is one implementation of it.
4. A diagnostic that derives the implied forward of the underlying from put call parity across a
   chain, exposed as a value a caller can compare against the market's forward and used to sanity
   check the inputs to greeks.
5. Verification against an independent reference: published values, or a second implementation in
   the repository sharing no code with the first, covering deep in the money and deep out of the
   money strikes, zero and high dividend yields, zero and high rates, and expiry, for both exercise
   styles. A self consistent expectation is not sufficient, on the evidence of the two defects
   above.

**Specification, D4B: volatility surface.** A surface is a subsystem with declared inputs and
declared limits, not a fitting function.

1. **Observations.** The source is the option chain. The filters are named: bid and ask rather than
   mid, a minimum bid, a maximum spread, a minimum time to expiry, a minimum volume where available,
   rejected crossed markets, and stale quotes rejected by age.
2. **Coordinates.** Total implied variance against log moneyness, with time to expiry as the second
   axis. This is not a stylistic choice: in this parameterisation the no-arbitrage conditions are
   statements about monotonicity and convexity of the interpolant, whereas in quoted price space
   they are not.
3. **Interpolation.** Monotone and convexity preserving within the quoted region, so the surface
   cannot introduce a butterfly arbitrage that was not in the observations. VeighNa fits a cubic
   spline, which preserves neither property in general; the scheme is chosen by the conditions it
   must satisfy, not by what VeighNa uses.
4. **Calendar consistency.** For each log moneyness, total variance must be non-decreasing in time
   to expiry.
5. **Extrapolation.** Flat total variance beyond the quoted range, with every extrapolated query
   flagged. Flat total variance preserves no-arbitrage; a continued spline does not, so the two
   policies are not equivalent choices.
6. **Arbitrage validation.** The monotonicity, convexity and calendar conditions are checked and
   reported, never silently repaired, and a surface that fails its own validation is refused rather
   than served. A parameterised arbitrage-free family, such as a stochastic volatility inspired
   parameterisation, is the fallback if the interpolation family cannot satisfy the conditions in
   practice; the conditions are the requirement and the family is the implementation.
7. **Calibration.** The trigger (on a timer, on new quotes, or on demand), the frequency, and the
   behaviour when the observation set is too small to fit.
8. **Confidence.** Every query returns a fit error or confidence alongside the value, derived from
   the residuals at the observed points, and the number of observations contributing.

**Acceptance and verification.** A table of reference values checked into the repository with the
source of each value recorded; a test that fails if the exercise style declaration is ignored; a test
that the tree price converges as the step count rises; and, for the surface, a test that a query
outside the fitted region reports its extrapolation, a test with a deliberately arbitrageable input
that the validation check rejects, and a test with too few observations that calibration refuses
rather than returning a degenerate surface.

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

**Specification to implement, as a research capability outside the engine.** Three named concepts,
because a utility library with unnamed parts cannot be verified.

1. **Feature**: a deterministic transformation of market or reference data, declared once and
   reusable. A compiled expression tree with a typed node set, a parser for a plain text surface,
   and a canonical serialisation that digests to a stable identifier, so a definition can be pinned
   by a regression scenario and appear in an experiment digest.
2. **Label**: a future outcome, defined with the same rigour as a feature, including a forward
   return over a horizon, a forward maximum drawdown, and a forward realised volatility, each with
   its horizon and its terminal convention stated.
3. **Dataset** (D13): a point in time panel with one row per instrument and timestamp, the feature
   columns, the label column, the membership that applies at each timestamp, the train, validation
   and test split, and the metadata required to reproduce it. Every transform declares whether it is
   computed on the learning side or the inference side, so that no future information can leak into
   a feature or into a normalisation.
4. Operators: time series per instrument (lag, rolling mean, rolling standard deviation, rolling
   correlation, rolling rank, rolling regression residual) and cross sectional per timestamp (rank,
   scale, sum, and neutralisation against a grouping key).
5. Membership is point in time. Whether membership history is stored data or a rule re-evaluated at
   each historical timestamp is an open question in `design 13`; either way a panel may not use
   today's membership for a past timestamp, which is the structural defence against survivorship and
   look-ahead bias.
6. Numeric verification against an independent reference for a factor set, rather than the existence
   check that VeighNa performs.
7. No second execution model. A signal strategy rebalances through the normal strategy and execution
   path. A factor value is authoritative in the research pipeline and nowhere else.
8. The pipeline is a general research primitive and does not replace domain-specific score engines
   or their contracts. A deterministic score and a learned factor may consume the same data
   infrastructure, but they are not one subsystem and neither may absorb the other, which is the
   drift this item exists to prevent.
9. Provenance, per `design 7.4`: every value the pipeline emits resolves to its dataset, feature,
   membership and experiment digests, its as-of timestamp and the source version of its data. A
   value that cannot be attributed is not a result.

**Acceptance and verification.** A scenario that pins a small factor set numerically for a fixture
catalog, a test that detects deliberate label leakage, and a test that the digest of a factor
definition is stable across processes.

### 4.6 D6: notification events and sinks

**What VeighNa does.** 4.4.0 added `MainEngine.send_notification` over an email engine and a chat
engine, with a per channel interval so that messages inside the interval are coalesced. Note the
shape: a call site every subsystem must know about, rather than an event every subsystem already
publishes.

**Specification to implement.** A notification event, a router, coalescing and sinks, so that
notifications are another consumer of the event architecture rather than another cross-cutting
dependency. The router subscribes on the existing subscription points
(`crates/common/src/msgbus/api.rs:441` and `:453`, `crates/common/src/actor/data_actor.rs:1597`).

1. **The event set**, named rather than open ended: a risk limit breach, an order rejection, a
   strategy stop, an execution completion, a drawdown threshold breach, a data feed disconnect, a
   broker disconnect, a backtest completion and an optimization completion. Each carries the
   identity a sink needs to be useful (strategy, instrument, account, run) and a severity.
2. **The router**, which maps an event to the sinks subscribed to that class, applies per sink
   coalescing over a configurable interval, and never blocks the publisher.
3. **The sinks**, a trait with a minimal contract of send a titled message with a severity, with SMTP
   email and a generic HTTP webhook as the initial implementations and no vendor SDK in the core
   crates.
4. **A bounded queue** per sink with an observable drop policy, since a saturated sink must not grow
   without limit or stall the caller.
5. A sink failure is reported through the ordinary logging path and never re-enters the router, so a
   broken sink cannot loop.

**Acceptance and verification.** A test per sink against a local server stub; a test that coalescing
bounds the send rate under a burst; a test that a saturated queue drops and counts the drops rather
than blocking; and a test that a subsystem publishing a notification event does not depend on any
sink being configured.

### 4.7 D8: search strategies and a memo cache

**What VeighNa does.** `vnpy/trader/optimize.py` runs a brute force grid over a process pool and a
genetic search using DEAP with configurable population, generation and operator parameters, over a
process pool with a manager dictionary caching evaluations by parameter vector. 4.1.0 made the
genetic hyperparameters user controllable.

Correction to the earlier framing, which treated the objective as implicit: it is already first
class in this project: the
analyzer provides a weighted composite objective with direction per term and constraints with a
comparison, over the 34 built-in metric names, and a missing metric is an error rather than a zero
(`crates/analysis/src/objective.rs`). So the objective model is not a gap; what is missing is
search, and the metrics that would let an objective express execution-aware goals, which D3
supplies.

**Specification to implement.** A random search and an evolutionary search implementing the existing
`SearchStrategy` protocol, both deterministic under a seeded generator, both recording every
evaluated parameter vector and its result in the existing persistence so that a resumed run skips
them, and both reporting the evaluated fraction and the best result through the existing report.

The run is described by its parameter space, objective, constraints, dataset, validation scheme,
experiment digest, seed, search strategy, evaluation cache, results and report. Of those, the space,
objective, constraints, digest, persistence and report exist; the validation scheme, the seed, the
evaluation cache and the evolutionary operators are what this item adds.

The validation scheme is part of the run rather than a caller's convention: which portion of the
dataset is used for search, what is held out, and whether the evaluation is a single split or a
walk-forward sequence of fits and out-of-sample windows. Without it, in-sample and out-of-sample
results are indistinguishable in the report, which is the failure mode that makes an optimizer
dangerous. A held-out evaluation is never scored by the search, and the report states which windows
were search and which were held out. The memo cache is keyed by the canonical parameter
digest that already exists, which is the same key that makes a resumed run reproducible. A
constraint must be evaluated before the objective, so an infeasible candidate is recorded as
infeasible rather than as a low score, and an evolutionary search must treat constraints as
feasibility rather than as a penalty added to the score.

**Acceptance and verification.** A test that a seeded run reproduces the same sequence of candidate
vectors and the same best result, a test that a resumed run does not re-evaluate a cached vector
(evidenced by the evaluation count), and a test that a search with a budget smaller than the space
still reports the best of what it evaluated.

### 4.8 D9: portfolio construction, no change and an adjacent gap

**What VeighNa does.** `TargetPosTemplate` reconciles a target position for one instrument by sending
or cancelling orders until the position matches. This project's target construction and reconciler
cover the same ground with a sized target, a risk-scaled weight and reconciliation against positions
and resting orders, and the path is pinned by a parity regression scenario. VeighNa therefore
contributes nothing here.

**Specification.** No change. If the cross-sectional layer is ever built, it belongs ahead of the
existing construction stage as a weighting step over a set of signals, producing per instrument
targets that the existing stages consume, and it must not introduce a second targeting mechanism.
The constraints that would have to be expressible are gross and net exposure, factor, sector, dollar
and beta neutrality, turnover, and per name caps. Recorded but not scheduled, and attributed to the
comparison rather than to VeighNa.

### 4.9 D10: universe, no change

**What VeighNa does.** Nothing beyond a watch list.

**Specification.** No change. Runtime membership exists with static and scheduled rules, change
events with reasons and per-universe subscriptions, and it is pinned by a membership regression
scenario. The surviving research requirement is point-in-time historical membership, which is
D13 and not a runtime membership change.

### 4.10 D11: execution intent

**Specification to implement.** A shared contract above the algorithms, so that TWAP, VWAP,
percentage of volume, arrival price, iceberg, quote pegged and sniper are different ways to satisfy
one declaration rather than seven parameter vocabularies.

1. Three types, not one, because collapsing them is how an algorithm becomes a dumping ground for
   strategy specific parameters: an **intent** is what the caller wants (buy 10,000 shares), a
   **policy** is the constraints and preferences (maximise passive execution, 30 minute horizon,
   15 basis point slippage cap), and an **algorithm** is how the system attempts it (iceberg).
2. The policy carries urgency, a participation rate, a price or slippage constraint, a passive or
   aggressive preference, and a duration or horizon, and it is validated at submit time in the same
   way TWAP validates its parameters today, with a denial that names the offending field.
3. An algorithm declares which parts of a policy it can honour. An algorithm that cannot honour a
   constrained policy must refuse the order rather than silently relax the constraint, which is the
   failure mode that makes per-algorithm parameter vocabularies dangerous.
4. The shipped TWAP migrates onto the intent and policy pair rather than keeping its own parameters,
   so there is one contract and not two, and the migration must reproduce the existing schedule for
   an equivalent declaration.
5. The intent is not a second execution study: it declares intent and the algorithm owns scheduling,
   quoting and completion, per the ownership statement in 4.2.

**Acceptance and verification.** A test that each algorithm refuses an intent it cannot honour; a
test that an intent with a price constraint is never satisfied by a child order outside it; and a
test that the migrated TWAP produces the same schedule as before for an equivalent intent. The
price-constraint case needs an algorithm that honours a price limit, so it is exercised when the
first such algorithm lands (D2) rather than by TWAP, which refuses a declared price limit.

### 4.11 D12: canonical accounting

**Specification.** Formalise what already holds rather than adding a component. The portfolio and the
account model are authoritative for position, realised PnL, unrealised PnL and margin
(`crates/portfolio/src/portfolio.rs:186`, `:513`, `:581`, documented in `docs/concepts/accounting.md`),
and the D3 frame is the only periodic reduction of them. No new component may recompute PnL from
prices, and no analytics module may keep a parallel ledger.

**Acceptance and verification.** A test that the frame's totals reconcile to the portfolio's own
totals for the same run, and a review rule that a new component touching PnL either consumes the
portfolio or states in its documentation why it does not.

### 4.12 D13: point in time dataset

**Specification to implement,** as the dataset contract of D5.

1. Membership history is stored data rather than a rule re-evaluated at a past timestamp. A rule
   evaluated today cannot represent an instrument that has since delisted, and the instrument
   definition needed to evaluate it may itself no longer be available, so a re-evaluated rule is
   structurally unable to reproduce the past. The membership rule remains the mechanism that
   produces the series, and the series is persisted in the catalog alongside the data it describes
   and resolved per timestamp when a panel is built.
2. A panel may never use current membership for a past timestamp, and a feature may never read a
   value with a timestamp later than its own.
3. The split is part of the dataset rather than a caller's convention, so that a model cannot be fit
   across a boundary it was not given.
4. The metadata records the source catalog intervals, the membership source, the feature and label
   digests, and the split boundaries, so the panel can be reproduced and digested.

**Acceptance and verification.** A leakage test: a deliberately leaky feature must be rejected or
detected. A membership test: a panel for a past timestamp must not contain an instrument that joined
later. A reproducibility test: the same dataset declaration produces the same digest across
processes.

### 4.13 D14: execution analytics

**Specification to implement,** alongside D2, as read-only observation of fills, orders and the book.

1. Arrival price metrics: implementation shortfall, arrival slippage and decision price slippage,
   each with its reference price named (the decision price, the arrival price, or the open of the
   period) because the metric is otherwise undefined.
2. Execution quality metrics: VWAP and TWAP slippage against an interval, midpoint slippage, spread
   capture, adverse selection over a stated horizon, fill ratio and cancel ratio.
3. Parent and child metrics: completion time against the horizon, child count, child churn,
   mean child lifetime, partial fill ratio and price improvement per child.
4. Every metric states its reference price, its denominator and its reference timestamp, because the
   metric is otherwise ambiguous. The reference timestamp is declared per metric as one of the
   decision timestamp, the arrival timestamp, the parent submission timestamp, the first child
   timestamp, or the benchmark interval start; "arrival price" without a timestamp convention is not
   a definition.
5. The analytics keep no state of their own beyond the observation window, per section 7.1, and they
   are read-only observers of fills, orders and the book.

**Acceptance and verification.** A scenario with a known arrival price, a known VWAP over a known
interval, and a hand-computed implementation shortfall and slippage, so the metric definitions are
pinned numerically rather than described. The churn and child count metrics also serve the D2
acceptance requirement that churn is evidenced rather than asserted.

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

This table is the detailed verification plan. The minimum acceptance contract per decision, which is
what makes the design independently reviewable, is stated in `design 12.1`.

| Item                      | Acceptance                                                                                                                                                                                    | Verification                                                                                                                 | Status                                                     |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------- |
| D1 Risk caps              | Each new cap denies with a typed reason, cancels release capacity, and the window expiry restores it                                                                                          | Unit tests per cap plus a criterion benchmark on the send path                                                               | Implemented (1.3)                                          |
| D2 Algorithms             | Iceberg, quote pegged and sniper execute a parent to completion or deadline with bounded churn, correct behaviour when a child is denied, and conservation of parent quantity across children | One regression scenario per algorithm plus boundary unit tests, with the conservation identity asserted per scenario         | Implemented (1.3)                                          |
| D3 Frame and statistics   | A periodic frame with the realised and unrealised split, four statistics, an interval trigger shared by live and backtest, and equity that reconciles exactly to the portfolio                | Regression scenario against a hand computed expectation, an independent value check per statistic, and a reconciliation test | Not implemented                                            |
| D4A Exercise              | The exercise style is honoured per instrument, and the tree price converges as its step count rises                                                                                           | Reference value table with recorded provenance, and a test that ignoring the declaration fails                               | Not implemented                                            |
| D4B Volatility surface    | Queries inside the fitted region interpolate, outside it report extrapolation, arbitrage is detected, and a thin observation set is refused                                                   | Surface tests including an arbitrageable input and an underdetermined calibration                                            | Not implemented                                            |
| D5 Factors                | Factor definitions compile to a digestible tree, the panel leaks no future information, and a factor set matches an independent reference                                                     | Numeric pinning scenario, leakage test, digest stability test                                                                | Not implemented                                            |
| D6 Notifications          | Sinks send, coalesce and drop under saturation without blocking the caller                                                                                                                    | Local server stub tests                                                                                                      | Not implemented                                            |
| D7 Restart recovery       | No change                                                                                                                                                                                     | Existing tests                                                                                                               | No change required                                         |
| D8 Search                 | Seeded searches reproduce, resumed runs skip cached evaluations, and the report reflects the evaluated set                                                                                    | Determinism and resume tests, evidenced by evaluation counts                                                                 | Not implemented                                            |
| D9 Portfolio construction | No change: the mechanism exists; the cross-sectional layer is recorded, not built                                                                                                             | Existing target pipeline parity scenario                                                                                     | No change required                                         |
| D10 Universe              | No change: runtime membership exists; historical point in time membership is D13                                                                                                              | Existing universe membership scenario                                                                                        | No change required                                         |
| D11 Execution intent      | Algorithms implement one intent contract, refuse an intent they cannot honour, and TWAP migrates without changing its schedule for an equivalent intent                                       | Refusal tests, price constraint tests, and a schedule equivalence test for the migrated TWAP                                 | Implemented (1.3); the price constraint case lands with D2 |
| D12 Canonical accounting  | The frame reconciles to the portfolio's own totals, and no component recomputes PnL from prices                                                                                               | A reconciliation test per run, and a documented authority for every component touching PnL                                   | Not implemented                                            |
| D13 Point in time dataset | Membership is resolved per timestamp, the split is part of the dataset, and the panel is reproducible by digest                                                                               | Leakage test, later membership test, and a cross-process digest test                                                         | Not implemented                                            |
| D14 Execution analytics   | Every metric states its reference price, denominator and reference timestamp, and is pinned numerically                                                                                       | A scenario with a known arrival price and VWAP and hand-computed shortfall and slippage                                      | Not implemented                                            |

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
   the absence of any bound in their design. Our three algorithms now count the children they
   submit and cancel, and each engine scenario reports those counts, so a bound can be justified by
   a measurement rather than asserted; what is still unmeasured is churn against a live venue's
   message limits, which no local scenario can produce.
2. The cost of the D1 caps on the send path is now measured on this machine by
   `cargo bench --locked -p nautilus-risk --bench caps`. The allow path over five configured caps
   whose windows each hold 1,000 occurrences took 6.49 to 6.85 microseconds, and a refusal at the
   first cap took 63.4 to 66.7 nanoseconds. The allow figure is linear in the occurrences the
   windows hold rather than in the number of caps, because a counter drops expired occurrences as it
   is seen; a window seeded with a configuration's whole limit (20,000 submissions in the VeighNa
   values) would be correspondingly more expensive per evaluation. The measurement has not been
   taken against a live feed, and no optimisation is proposed on the strength of a benchmark alone.
3. The concrete cap values and default window durations for D1 are unsettled. A default that is too
   low will deny legitimate strategies, and the value cannot be derived from the mechanism, so it
   needs a judgement recorded next to the configuration (`design 13`). The mechanism is now
   implemented with no cap configured by default, so an engine with an unconfigured
   `RiskEngineConfig` denies nothing, and no default value is proposed here. The VeighNa values
   (20,000 submits, 10,000 cancels and 10,000 fills globally, 2,000 submits per instrument, 50
   concurrent active orders) appear only as inputs to the benchmark, not as recommended settings.
4. The D2 policy vocabulary is partly settled and partly not. The vocabulary now exists
   (`ExecutionPolicy` and `PolicyPart`), an algorithm declares the parts it can honour, and TWAP
   declares the horizon alone and refuses the other five with a denial naming the field. What stays
   open is which parts iceberg, quote pegged and sniper must honour, and in particular whether a
   price limit is honoured by refusing to cross it or by never quoting outside it; that answer
   decides how much of D11 each one implements (`design 13`).
5. The D4B calibration trigger and minimum observation count are unsettled, and they decide when a
   surface is refused rather than served (`design 13`).
6. The provenance of the reference values required by 4.4, 4.5 and 4.13 must be identified during
   those workstreams; a reference whose origin cannot be recorded is not acceptable evidence.
7. D9's cross-sectional layer is recorded but unscheduled, and it has no justification yet beyond
   the fact that the layer is absent. It should only be scheduled against a stated research
   requirement, not because a taxonomy has a box for it.
8. Membership history is now decided to be stored data in 4.12, but its cost is unmeasured: the
   series must be produced for a whole history before a panel over that history can be built, and
   that backfill is a prerequisite the workstream has not costed.
9. The D14 metric set has no acceptance threshold for any metric. Whether an implementation shortfall
   is good or bad depends on the strategy and the venue, so the analytics should report and pin
   values without asserting that any value is acceptable.
10. The D1 caps are reachable only through the Rust `RiskEngineConfig`. Neither the Python
    `RiskEngineConfig` nor `LiveRiskEngineConfig` carries them, so a Python user or a live deployment
    cannot set a cap, and how a cap should be written from Python (a sequence, a mapping, or a
    dedicated object) is a public API decision that has not been made. Adding only the Rust field
    kept the change to the minimum surface the specification asked for; the Python surface is a
    separate decision.
