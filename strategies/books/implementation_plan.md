# The ten things worth acting on: implementation plan

Date: 2026-10-05. Revision 1.

`README.md` in this directory states the ten findings that recur across the sixteen briefs, and
each brief maps its own findings onto parts of this repository in its take-away table. This
document turns those ten into work: what exists today, what to build, in what order, and what
observation shows the work is done.

Every "Today" line names the file and symbol that owns the behaviour, checked against commit
`3a49c557c4`. If a line no longer holds, re-read the item before starting it.

## 1. How to read this plan

- Finding numbers and titles follow "What the corpus says collectively" in `README.md`. The
  numbers in parentheses are the briefs beside this file.
- "Today" is the state of this repository at revision 1, not of upstream NautilusTrader. Several
  surfaces named here are fork-local: the market impact model, the extended fill-model family,
  `python/nautilus_trader/optimization/` and `crates/research`.
- Work items carry handles (`W2.1`) so that a review comment can refer to one.
- "Acceptance" is an observation under a stated scenario, not a task list.
- Nothing here is a profitability claim, and nothing here relaxes the engine's authority to deny
  rather than resize a request. Several items can only make a simulated result worse; that is
  their purpose.
- Any new Python surface needs the stubs regenerated through `python/generate_stubs.py`. The
  generated `.pyi` files are never hand-edited.

## 2. State of play

### 2.1 Already in place

| Capability                                                        | Owner                                                                                                                                                                      |
| ----------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Fill models, user-extensible from Python                          | `crates/execution/src/models/fill.rs` (`FillModelAny`, eleven variants); `crates/execution/src/python/fill.rs` (`PyFillModel`); `BacktestEngine.add_venue(fill_model=...)` |
| Fee models with per-instrument overrides and rebates              | `crates/execution/src/models/fee.rs`; `crates/model/src/fees.rs` (`MakerTakerFeeRates`, `MakerTakerFeeSchedule`)                                                           |
| Simulated book with queue position                                | `crates/model/src/orderbook/`; `OrderMatchingEngineConfig.queue_position` and `.liquidity_consumption`                                                                     |
| Chronological splits with purge and embargo                       | `python/nautilus_trader/optimization/splits.py` (`SplitContract`, `LeakagePolicy`)                                                                                         |
| Deflated Sharpe with trial provenance                             | `python/nautilus_trader/optimization/significance.py`                                                                                                                      |
| Per-instrument market status enforced in the exchange             | `crates/execution/src/matching_engine/mod.rs` (`process_status`, the submit gate and the matching gate)                                                                    |
| Read-only execution analytics (shortfall, arrival, VWAP slippage) | `crates/trading/src/lib.rs` (`analytics`)                                                                                                                                  |
| Golden-output regression and accounting reconciliation            | `crates/backtest/benches/engine/canonical.rs`; `python/tests/regression/`; `crates/backtest/tests/performance_reconciliation.rs`                                           |

### 2.2 Partial

| Capability            | Today                                                                                                                          | Missing                                                                                          |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------ |
| Market impact         | `MarketImpactModel` with one linear variant                                                                                    | Concavity, prefactor calibration, a Python extension point                                       |
| Passive fill realism  | Probabilistic fills; queue position and liquidity consumption exist but default false                                          | Adverse-selection and size conditioning                                                          |
| Latency               | `StaticLatencyModel` over three order legs plus a base                                                                         | Market-data latency, competitor rank, a Python extension point                                   |
| Data quality          | Bar sequence validation (off by default); Python array monotonicity; adapters log and substitute `ts_init`                     | Quote and trade validation, crossed-print checks, feed-identity checks, offset estimation        |
| Point-in-time control | `crates/research` (`FeatureValue.as_of`, `Panel::check`, `AdmittedDecision.available_at`)                                      | Any Python binding; the crate has no dependents                                                  |
| Net-of-cost reporting | Commission netted into fills; `TotalCommissions`, `TotalTurnover`; `PeriodAccounting.fees` and `.slippage` built empty         | A cost report, breakeven cost, a visible fill assumption                                         |
| Tick rules            | `price_increment` known and precision enforced; alignment checked only when an instrument is redefined or a fill is normalized | Alignment at submit                                                                              |
| Trading state         | A halt denies new submits per instrument; global `TradingState` denies or restricts                                            | Cancel-on-halt; a Python setter for `TradingState`                                               |
| Risk limits           | `RiskCap` (metric, scope, limit, window) plus per-order notional, quantity, price and margin checks                            | Participation-rate and inventory caps; cross-strategy enforcement; a coordinated de-risking path |

### 2.3 Absent

| Capability                                       | Note                                                                                   |
| ------------------------------------------------ | -------------------------------------------------------------------------------------- |
| Auctions, price bands, circuit breakers          | No model or engine primitive; adapter metadata only                                    |
| Feed-identity validation                         | Nothing checks reported volume against open interest or a settlement total             |
| Cross-venue timestamp reconciliation             | Ordering is insertion order; `VirtualClock` monotonicity is per clock                  |
| Null-model or reference-distribution validation  | Only determinism and accounting reconciliation exist                                   |
| Shipped synthetic flow generator                 | Generators live inside examples and benches                                            |
| Engine-level random seed                         | Only `FillModelConfig.random_seed` and `use_random_ids`                                |
| Publication or knowledge date                    | `CustomData` carries `ts_init` only; `CorporateAction.effective_ns` is venue-effective |
| News or sentiment data type                      | News reaches the system as adapter metadata only                                       |
| Model-driven strategy example                    | No ML dependency and no serving hook                                                   |
| Global kill switch                               | Closest are the Rust-only `set_trading_state` and per-account backtest liquidation     |
| Impact decay or a transient/permanent split      | A search for `decay` in `crates/execution` and `crates/backtest` returns nothing       |
| Autocorrelation, variance-ratio or Hurst tooling | The one Hurst estimator sits inside a feature-gated example strategy                   |

## 3. Sequencing

| Phase                    | Content                      | Gate   | Why here                                                                                           |
| ------------------------ | ---------------------------- | ------ | -------------------------------------------------------------------------------------------------- |
| P0 Measurement honesty   | W1.3, W1.4, W3.1, W7.1, W7.4 | None   | Every later phase is read through a seed, a cost number and a way to detect a manufactured pattern |
| P1 Cost realism          | T1, T2, T4                   | P0     | Can only make a result worse, so it cannot create a false positive                                 |
| P2 Data risk             | T6                           | P0     | A cost study on dirty data is a study of the wrong data                                            |
| P3 Simulation validation | T7, T3                       | P1, P2 | The harness needs the models it validates                                                          |
| P4 Research to live      | T8, T5                       | P0     | Depends on the Python binding work, not on the cost work                                           |
| P5 Design and limits     | T9, T10                      | P1, P3 | Fees, ticks and caps set the payoff function that P1 measures                                      |

P0 first is deliberate: it adds no economic assumption, only observation.

Each phase is worth shipping alone. P1 needs P0 only for the seed and the cost report; if P2 never
happens, P1 still replaces a flat assumption with a structural one.

## 4. The ten

### T1 Costs, turnover and latency decide deployability, not forecast accuracy (09, 14, 16, 03)

**Corpus.** Every slice that netted out costs found the edge collapse: brief 14 reports signals
that survive at 1-5 bps and not beyond, brief 09 asks for a breakeven-cost column beside every
gross return, brief 16 reports full costs halving a CAGR, and brief 03 finds that fees plus
adverse selection consume the effective rebate.

**Today.** Costs enter only through the fee model, and commission is netted into
`Position.realized_pnl`. `PeriodAccounting` (`crates/analysis/src/period.rs:293`) carries
`fees` (`:312`) and `slippage` (`:318`) and both are constructed empty (`:586-587`) because
`OrderFilled` carries no fee or slippage field. `PortfolioStatistics` offers `TotalCommissions`
and `TotalTurnover` but no cost semantics. Rust has a read-only execution analytics module
(`crates/trading/src/lib.rs`: `ExecutionObserver`, `METRIC_IMPLEMENTATION_SHORTFALL_BPS`,
`METRIC_ARRIVAL_SLIPPAGE_BPS`, `METRIC_VWAP_SLIPPAGE_BPS`, `MetricValue::NotAvailable`) with no
Python exposure. The deflated Sharpe lives only in `optimization/significance.py`.

**Work.**

- W1.1 Expose the execution analytics module to Python so a backtest can report implementation
  shortfall, arrival slippage and VWAP slippage without re-deriving them.
- W1.2 Add a cost row to the report: gross return, commission, cost in basis points, net return,
  turnover and breakeven cost, next to Sharpe and Calmar.
- W1.3 Decide `PeriodAccounting.fees` and `.slippage`: populate them from the fee model and the
  analytics module, or delete them, so no consumer can read a silently empty field.
- W1.4 Put the deflated Sharpe beside the gross and net figures, with the trial count that
  produced it. Shared with W8.4.

**Acceptance.** With `MakerTakerFeeModel`, the report shows gross and net apart; setting both
rates to zero moves net onto gross and leaves gross unchanged. A backtest can print the deflated
Sharpe and the trial count without importing from `optimization` by hand.

### T2 Impact is concave near a square root, and the prefactor is uncertain (02, 07)

**Corpus.** The exponent is robust: delta = 0.489 +/- 0.0015 on TSE, 0.50 [0.32, 0.66] on AAPL.
The prefactor is not: c runs from 0.34 to 1.50 across three markets, and reconstructing metaorders
from an anonymous tape inflates it about twofold, so the honest output is a bounded range such as
0.34 to 0.69 rather than a point (`2606.24019v1`, `2411.13965v3`).

**Today.** `crates/execution/src/models/market_impact.rs` holds the trait
(`MarketImpactModel::impact_increments(fill_quantity) -> u64`), a handle, and exactly one
implementation: `LinearMarketImpactModel`, whose adjustment is
`floor(quantity / quantity_per_increment)` capped by `max_increments`. `MarketImpactModelAny` has
one variant. Impact applies only to an `L1_MBP` taker fill, after the slippage adjustment
(`crates/execution/src/matching_engine/mod.rs`). There is no calibration tooling and the Python
binding accepts the built-in only, so a venue-specific calibration needs a Rust rebuild.

**Work.**

- W2.1 Add a concave model to the same trait and the same composition slot, with a prefactor and a
  reference quantity or volume as parameters.
- W2.2 Make the model carry an interval, not a point, and make the report print both bounds with
  the calibration source, so a tape-derived prefactor cannot be read as measured.
- W2.3 Add calibration tooling that fits the prefactor from fills or a tape and applies the
  anonymous-tape de-bias as an explicit step.
- W2.4 Add a Python protocol for impact models, mirroring the duck typing already used for fill
  models (`crates/execution/src/python/fill.rs`), so a calibration does not require a rebuild.
- W2.5 Decide whether impact applies beyond `L1_MBP` taker fills, and record the decision in the
  trait's doc comment with a test either way.

**Acceptance.** Doubling the filled quantity less than doubles the impact. Impact is
schedule-invariant for a fixed size over volume. The regression case
`python/tests/regression/cases/market_impact_model.py` gains a concave scenario with a
fingerprint, and a tape-derived calibration reports two bounds.

### T3 Order flow is long-memory, and linear impact manufactures returns (07, 08, 02)

**Corpus.** Order flow is persistently autocorrelated (H roughly 0.7 in FX spot, splitting
effects sub-hour) and prices remain close to diffusive only because impact is sublinear
(`2502.17906v4`, `2608.00988v1`). A simulator with linear impact and persistent flow produces
predictable returns that the market does not have.

**Today.** The impact model is a pure function of the fill quantity: it never updates quotes and
never feeds back into later fills. No autocorrelation, variance-ratio or unit-root tooling exists
in `crates/analysis` or `crates/research`; the only Hurst estimator is
`HurstVpinDirectional::estimate_hurst`, inside the feature-gated examples.

**Work.**

- W3.1 Add autocorrelation, variance-ratio and rescaled-range tools to the analysis surface, and
  expose them, so a claim about persistence is testable rather than asserted.
- W3.2 Make the linear model's failure mode observable: a test and a documented scenario in which
  a persistent synthetic flow plus linear impact yields a variance ratio above one, so the
  artifact is named rather than mistaken for a result.
- W3.3 Require the flow generator of W7.4 to take a target Hurst exponent and an impact exponent
  no greater than one half, per the diffusiveness condition the briefs cite.

**Acceptance.** The variance-ratio tool reports above one for the linear-impact synthetic run and
does not for the concave model on the same flow. The same tool is callable from Python.

### T4 Passive fills are adversely selected (03, 04)

**Corpus.** Passive fills line up with subsequent adverse moves: formally the expected drift
conditional on a fill is negative whenever the fill rate is below one, and the under-fill and
over-fill penalties are large enough to consume the effective rebate (brief 03). Fill
inter-arrivals are heavier-tailed than exponential.

**Today.** Passive fills are probabilistic (`ProbabilisticFillState`) and seeded. Queue position
and liquidity consumption exist on `OrderMatchingEngineConfig` but both default to false. The
passive models do not condition fill probability on order size, queue ahead or toxicity, and a
user fill model receives only the best bid and ask and returns a synthetic book
(`FillModel::get_orderbook_for_fill_simulation`), never the real one.

**Work.**

- W4.1 Add a passive fill model conditioned on adverse selection: fill probability falls with
  queue ahead and with measured toxicity, calibrated so that fills coincide with adverse moves.
- W4.2 Value passive fills against the imbalance-adjusted microprice rather than the mid, per
  brief 05.
- W4.3 Report the fill assumption as a first-class row: full, probabilistic, or
  adverse-selected, so a full-fill backtest cannot be read as passive performance.

**Acceptance.** The same strategy's passive result falls when adverse selection is enabled, and
changes only through the seed when it is not. The report names the fill assumption used.

### T5 Speed is ordinal, and the aggressive ceiling is collapsing (05)

**Corpus.** The closest trader to the book averaged $2,681.04 a day while the second closest lost
$3,297.30, so the edge lives in rank. Aggressive upper-bound profit for 19 NASDAQ stocks fell from
$3.4 billion at a 10 s holding period to $62,000 at 10 ms. Cross-venue lead-lag carries a
single-vantage offset ambiguity of about +/-99 ms even with drift bounded to 6 ms.

**Today.** `crates/execution/src/models/latency.rs` has a `LatencyModel` trait over base, insert,
update and delete latency, with exactly one implementation, `StaticLatencyModel`. Latency applies
to order commands only; there is no market-data or risk latency and no notion of a competitor.
`crates/execution/src/python/latency.rs` extracts the built-in only, so Python cannot inject a
model. Cross-venue ordering in the backtest is insertion order, and the message bus dispatches
synchronously with a static priority.

**Work.**

- W5.1 Add a competitor set and a rank-at-decision-time read, so a backtest can answer whether the
  order would have arrived first rather than only how long it took.
- W5.2 Add a Python protocol for latency models, matching the fill-model pattern.
- W5.3 Record decision-to-execution delay as a first-class live metric (brief 09).
- W5.4 Refuse to print a cross-venue lead-lag number without an offset-ambiguity bound and a drift
  statement, per the null result in brief 05.

**Acceptance.** Holding the signal fixed, moving only the competitor's latency changes the fill
outcome; moving the whole cohort together does not. The lead-lag tool cannot emit a point estimate
without its bound.

### T6 Market data is a risk, not an input (05, 10, 13)

**Corpus.** A consolidated feed misordered 66.2% of AAPL's 482,578 trades on 11 Aug 2015, and the
rate scales with volume (slope 0.66, R-squared 0.99). Cross-venue offsets are ambiguous at about
+/-99 ms. Reported open interest violates its own identity against volume in crypto venues.

**Today.** `DataEngineConfig.validate_data_sequence` defaults to false and covers bars only, where
a late bar is dropped. Quotes and trades pass through unchecked, and `QuoteTick::new_checked`
enforces precision equality and nothing else. There is no crossed-print guard and no aggregator
sanity check. Adapters log and substitute `ts_init` for an unparseable venue timestamp. The
`OrderBook` warns on an out-of-order update and clamps its high-water mark. The Python bulk
converters enforce monotonically increasing `ts_init`, and the matching engine skips a quote or
trade older than the book. Nothing validates volume against open interest, and no cross-venue
reconciliation exists.

**Work.**

- W6.1 Add a data-quality gate for quotes and trades: monotonicity, crossed or inverted prints, and
  optional reordering, with a config field.
- W6.2 Count and report every violation, so the gate produces a number in the backtest or live
  report rather than a log line.
- W6.3 Add feed-identity validators for volume against open interest and for settlement totals.
- W6.4 Estimate the clock offset between venues with a drift bound, and make a `ts_init`
  substitution a countable data-quality event.

**Acceptance.** A crossed quote, a monotone-violating trade and a record whose volume cannot
reconcile with its open interest each produce a counted violation, and with the gate enabled a
denial or a flag rather than silent acceptance.

### T7 Simulation needs nulls, robustness and out-of-sample calibration (08, 12, 10)

**Corpus.** Stylized-fact matching is weak validation: parameter degeneracy, single-mean
reporting and undocumented agent logic make agent-based conclusions fragile, and the same
simulator can reproduce tails while getting interval memory wrong. Validation needs nulls, rule
and horizon robustness, and out-of-sample calibration.

**Today.** Validation proves determinism and bookkeeping, not realism:
`crates/backtest/benches/engine/canonical.rs` fingerprints canonical scenarios with blake3,
`python/tests/regression/` digests declared scenarios including the impact, slippage and composed
execution-realism cases, and `crates/backtest/tests/performance_reconciliation.rs` checks the
period frame against the portfolio authority. No null model, reference distribution, robustness
sweep or out-of-sample calibration exists. There is no shipped synthetic data generator and no
engine-level seed: the only seeded randomness is `FillModelConfig.random_seed`, and
`use_random_ids` toggles unseeded UUIDs.

**Work.**

- W7.1 Add a seed to the backtest engine configuration so a run is reproducible end to end;
  fill-model draws default to unseeded when `random_seed` is unset.
- W7.2 Add a null-model harness that runs the same strategy against a reference process and prints
  a distribution rather than one number.
- W7.3 Add a robustness runner that sweeps seeds and horizon rules, reports distributions, and
  flags parameter sets whose results are degenerate.
- W7.4 Ship a synthetic flow generator calibrated to a target Hurst exponent and impact exponent,
  which W3.3 and T3 depend on.

**Acceptance.** A run that matches a stylized fact but fails an interval-memory check is reported
as failing. Two runs at the same seed fingerprint identically; two seeds produce different
distributions with the spread reported.

### T8 Text and machine-learning signals need explicit leakage controls (14, 09)

**Corpus.** Publication timestamps, model data-freshness cutoffs, chronological splits and
deflated significance move reported results from implausible to modest. Masking identifiers beats
a prompt instruction, and date-only recall exposes memorisation.

**Today.** This is the strongest area and the most disconnected.
`python/nautilus_trader/optimization/splits.py` has `SplitContract`, `LeakagePolicy` with purge
and embargo, `LabelOverlapRule` and `SplitDirection`, and splits cannot shuffle by construction
(`split()` enumerates windows in time order). `optimization/significance.py` has
`deflated_sharpe_ratio`, `SharpeSample`, `StatisticalContract` and `trial_provenance_from_runs`,
consumed by `relative_value.py`. Neither reaches the statistics or tearsheet path.
`crates/research` enforces point-in-time structurally (`Panel::check` rejects a feature whose
`as_of` is after the row's `ts_event`; `AdmittedDecision.available_at`; membership spells), but the
crate has no dependents and no Python module, so none of it is reachable from a strategy. No
publication or knowledge date exists on any model data type, there is no news or sentiment type,
and no ML integration or model-driven example exists.

**Work.**

- W8.1 Bind the point-in-time minimum of `crates/research` to Python: `Panel`, `FeatureValue.as_of`
  and point-in-time membership. This is the enabler for everything else in T8.
- W8.2 Add a knowledge date to the data path, most cheaply as an extension of `CustomData`, so
  "this datum was not known until T" is expressible without Rust.
- W8.3 Add a news or sentiment data type carrying both a publication and a receipt timestamp.
- W8.4 Put the deflated Sharpe beside the gross and net figures. Shared with W1.4.
- W8.5 Add a masked-replay probe for text and language-model signals, building on `decision_bridge`
  and `testkit`.

**Acceptance.** A record published after the decision time is excluded and counted. A feature that
reads its own future is rejected with the reasoning already tested in
`crates/research/tests/leakage.rs`. The report prints the deflated Sharpe with its trial count.

### T9 Fee, tick and halt rules are part of the payoff function (11)

**Corpus.** A rebate flips the undercutting incentive and makes net taker cost non-monotone in the
rebate. A tick reduction helps small orders and hurts large ones. Price bands and halts are not
equivalent, and the auction duration optimum is asset-specific.

**Today.** Fees are complete for a flat schedule: `MakerTakerFeeModel` with per-instrument
overrides and negative maker rates as rebates (`crates/model/src/fees.rs`), plus `FixedFeeModel`,
`PerContractFeeModel`, `ProbabilityPriceFeeModel`, `CappedOptionFeeModel` and
`TieredNotionalOptionFeeModel`, and a Python `FeeModel` subclass. No schedule is a function of
cumulative volume or account tier. Tick size exists as `price_increment`; precision is enforced in
the risk engine (`check_price`) and the matching engine, but alignment is not: a price that is
precision-legal and not a tick multiple passes at submission.
`Instrument::try_normalize_price` and the matching engine's `price_matches_tick` do perform that
check and are called only for instrument-update compatibility and fill normalization, where an
incompatible fill is skipped. Halts are enforced per instrument at submission and matching, and
the global `TradingState` denies or restricts at the risk engine, but a halt does not cancel
resting orders and `RiskEngine::set_trading_state` has no Python setter. Auctions, price bands and
circuit breakers are absent.

**Work.**

- W9.1 Enforce tick alignment at submission as a venue-configurable rule, reusing the existing
  alignment logic, instead of only at fill normalization.
- W9.2 Add a cancel-on-halt option, and expose `set_trading_state` to Python.
- W9.3 Add price-band and circuit-breaker primitives, as a submission rule plus a halt window,
  since the briefs show bands and halts are not interchangeable.
- W9.4 Add a volume-tiered rebate schedule, extending `MakerTakerFeeSchedule` from
  per-instrument rates to per-tier rates.

**Acceptance.** A precision-legal, non-aligned price is denied or rounded according to the venue
rule, with a named denial. With cancel-on-halt enabled, a halt empties the book. A tiered rebate
changes net cost monotonically in the tier input.

### T10 Individually prudent risk limits can worsen collective crashes (12)

**Corpus.** Non-monotone responses to participation limits and inventory caps appear in every
flash-crash simulation: tightening a per-participant cap can deepen the crash. Leverage-driven
deleveraging is contagion, crowding fattens tails, and sequential clearing carries a bias.

**Today.** The pre-trade limit inventory is: submission and modify rate limits, `count_caps`
(`RiskCap` over metric x scope x limit x window, with metrics Active, Submit, Modify, Cancel,
Fill, RepeatedRequest and scopes Global, Strategy, Account, Instrument, Venue,
StrategyInstrument), per-instrument and configured maximum notional, instrument minimum notional,
price and quantity precision and bounds, GTD validity, reduce-only consistency, cash-sell
collateral, and initial margin against one account's free balance. There is no participation-rate
limit and no inventory cap. The caps have no coupling to simulated impact or liquidity, so the
non-monotone behaviour cannot be expressed. `Portfolio` computes net exposures, but that
aggregation is observational: nothing enforces a portfolio-level or cross-strategy limit. There is
no global kill switch; the closest mechanisms are the Rust-only `set_trading_state`, per-strategy
`cancel_all_orders` and `close_all_positions`, and backtest-only per-account liquidation
(`liquidation_enabled`, `liquidation_trigger_ratio`, `liquidation_cancel_open_orders`,
`SimulatedExchange::process_liquidations`).

**Work.**

- W10.1 Add a participation-rate metric and an inventory or position metric to the cap vocabulary,
  so both reuse the existing scope, window and decision-history machinery.
- W10.2 Feed the portfolio-level aggregation into a cross-strategy, cross-venue pre-trade check
  instead of leaving it observational.
- W10.3 Add a coordinated de-risking path (halt, cancel, flatten) across strategies and venues as
  the missing global lever.
- W10.4 Add the non-monotonicity harness: a scenario in which raising a participation limit or an
  inventory cap increases simulated crash severity must be expressible and its result reported.

**Acceptance.** The tightening scenario runs and prints a severity comparison. The new caps deny
with named reasons and appear in the cap decision history. A portfolio-level limit denies an order
that is admissible per strategy but not in aggregate.

## 5. Cross-cutting work

- **C1. One Python binding pattern.** W1.1, W3.1, W5.3 and W8.1 all expose existing Rust surfaces.
  Do them with the same pattern and regenerate the stubs once per batch. Regeneration is a
  prerequisite, not a follow-up: the generated `python/nautilus_trader/risk/__init__.pyi` still
  omits `RiskCap` and `count_caps` even though the Rust and Python-facing surfaces exist, so any
  risk work written from the stub is written against stale declarations.
- **C2. The seed comes first.** W7.1 gates W7.2, W7.3 and W10.4; without it no distribution can be
  reproduced.
- **C3. The generator comes first.** W7.4 gates T3's acceptance and T7's nulls.
- **C4. One report row for cost and fill assumption.** T1 and T4 write into the same report; W1.2
  should leave room for the fill-assumption row of W4.3.
- **C5. Every new denial is a vocabulary change.** Tick alignment, participation and price bands
  each add a denial or flag. Add it to `crates/model/src/events/order/denied_reason.rs` and to
  `python/nautilus_trader/decision_bridge/execution.py`'s transcribed `DENIAL_CODES`, which a test
  pins against the Rust enum.

## 6. Non-goals

- No alpha research and no profitability claim. Every item is about measurement, realism or
  control.
- No change to deny-rather-than-resize. A limit that would clip a request stays out; I13 remains
  the sizing authority.
- This plan does not edit `README.md` or the sixteen briefs. They are the evidence record.
- No new external data vendor dependency and no ML framework dependency.
- Not a rewrite of the matching engine. Queue position and liquidity consumption already exist and
  several items switch them on rather than replace them.
- Nothing under `docs/` changes navigation or the published site.

## 7. What would move this plan

- If another workstream binds `crates/research` to Python, W8.1 shrinks to verifying the seam
  rather than building it.
- If a book-aware impact or fill hook is added for another reason, W2.5 and W4.1 inherit it.
- Every magnitude quoted here comes from one arXiv category and mostly from single markets. The
  plan uses them to choose what to build, not to predict a result; re-estimate per venue.

## 8. Revision history

| Revision | Date       | Change                                                                                         |
| -------- | ---------- | ---------------------------------------------------------------------------------------------- |
| 1        | 2026-10-05 | First plan, from the ten findings in `README.md` and a symbol-level survey of this repository. |
