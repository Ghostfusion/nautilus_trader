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

| Capability                                                                                     | Owner                                                                                                                                                                                                                                                                            |
| ---------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Fill models, user-extensible from Python                                                       | `crates/execution/src/models/fill.rs` (`FillModelAny`, eleven variants); `crates/execution/src/python/fill.rs` (`PyFillModel`); `BacktestEngine.add_venue(fill_model=...)`                                                                                                       |
| Fee models with per-instrument overrides and rebates                                           | `crates/execution/src/models/fee.rs`; `crates/model/src/fees.rs` (`MakerTakerFeeRates`, `MakerTakerFeeSchedule`)                                                                                                                                                                 |
| Simulated book with queue position                                                             | `crates/model/src/orderbook/`; `OrderMatchingEngineConfig.queue_position` and `.liquidity_consumption`                                                                                                                                                                           |
| Chronological splits with purge and embargo                                                    | `python/nautilus_trader/optimization/splits.py` (`SplitContract`, `LeakagePolicy`)                                                                                                                                                                                               |
| Deflated Sharpe with trial provenance                                                          | `python/nautilus_trader/optimization/significance.py` (`DeflatedSharpeRatio` is a `PortfolioStatistic`, so a registered run's report carries the corrected row with its trial counts in the name, W1.4)                                                                          |
| Per-instrument market status enforced in the exchange                                          | `crates/execution/src/matching_engine/mod.rs` (`process_status`, the submit gate and the matching gate)                                                                                                                                                                          |
| Read-only execution analytics (shortfall, arrival, VWAP slippage, decision-to-execution delay) | `crates/trading/src/lib.rs` (`analytics`)                                                                                                                                                                                                                                        |
| Golden-output regression and accounting reconciliation                                         | `crates/backtest/benches/engine/canonical.rs`; `python/tests/regression/`; `crates/backtest/tests/performance_reconciliation.rs`                                                                                                                                                 |
| Net-of-cost reporting                                                                          | `crates/analysis/src/analyzer.rs` (the default analyzer's cost row); `crates/analysis/src/statistics/` (gross and net return, and the cost and breakeven rates in basis points of turnover); `crates/execution/src/models/fill.rs` (each fill model's declared `FillAssumption`) |

### 2.2 Partial

| Capability                      | Today                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   | Missing                                                                                                                                                                                                                                           |
| ------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Market impact                   | `MarketImpactModel` with a linear variant and a concave square-root variant, the latter carrying its prefactor as an interval with its calibration source and applying the upper bound; the tooling to fit that interval from observed impacts and de-bias a tape-derived one, all constructible from Python, selectable per venue, extensible with a caller's own model through `add_venue`, and scoped to L1 taker fills with the decision recorded on the trait                                                                                                                                                      | A caller's own model in the declarative venue config, which mirrors the fill model's limitation                                                                                                                                                   |
| Passive fill realism            | Probabilistic fills plus an adverse-selection model whose fill probability falls with the queue ahead and with trade-flow toxicity, both seeded and constructible from Python, with the conditioning reachable through a provided trait method that leaves every existing model unchanged; passive fills can be valued at the imbalance-adjusted microprice, off by default, and each built-in declares its fill assumption so a result can be read against it                                                                                                                                                          | A real-book handoff for a user fill model, which still receives a synthetic book; declarations for the built-ins whose passive decision is neither a probability nor the queue and flow                                                           |
| Latency                         | `StaticLatencyModel` over three order legs plus a base; a `CompetitorSet` of declared rival latencies whose ordinal arrival rank is computed by the backtest exchange from the venue's own insert latency and read by the passive fill context, where `AdverseSelectionFillModel` scales its probability by `1/rank` above rank one (W5.1)                                                                                                                                                                                                                                                                              | Market-data latency; no Python fill model can see the fill context at all, so a Python model cannot read the competitor rank; any rank outside the passive path, so the aggressive ceiling and the sandbox execution client are untouched         |
| Data quality                    | Bar sequence validation (off by default); Python array monotonicity; a gate for quotes and trades behind a config action, checking the touch for a crossing, values for positivity and `ts_event` for order per instrument, counting each violation by kind and reporting the counts in the backtest summary; feed-identity validators for a reported open-interest change against traded volume and for a settlement total against its components (W6.3); `ClockOffsetEstimator` for a cross-venue offset with a drift bound, and the bybit adapter counts a `ts_init` substitution rather than only logging it (W6.4) | Cross-venue reconciliation; a counted substitution at every adapter that falls back to `ts_init`                                                                                                                                                  |
| Point-in-time control           | `crates/research` (`FeatureValue.as_of`, `Panel::check`, `AdmittedDecision.available_at`), reachable from Python as the `nautilus_trader.research` module (W8.1)                                                                                                                                                                                                                                                                                                                                                                                                                                                        | A dependent inside the engine: no strategy or engine component consults a panel yet                                                                                                                                                               |
| Execution analytics access      | The observer, its term and observation types, the metric set and its vocabulary are callable from Python, and the observer takes the model's `QuoteTick` and `TradeTick` directly; a metric carries its declaration, and an unavailable one is `None` with a reason, never `0.0`                                                                                                                                                                                                                                                                                                                                        | A bus-integrated collector binding, so the caller feeds the observer rather than the engine                                                                                                                                                       |
| Tick rules                      | `price_increment` known and precision enforced; alignment checked only when an instrument is redefined or a fill is normalized                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          | Alignment at submit                                                                                                                                                                                                                               |
| Trading state                   | A halt denies new submits per instrument; global `TradingState` denies or restricts                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Cancel-on-halt; a Python setter for `TradingState`                                                                                                                                                                                                |
| Risk limits                     | `RiskCap` (metric, scope, limit, window) plus per-order notional, quantity, price and margin checks                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     | Participation-rate and inventory caps; cross-strategy enforcement; a coordinated de-risking path                                                                                                                                                  |
| Simulation seeding              | `BacktestEngineConfig.random_seed` seeds every built-in fill model that declares no seed, all eleven of them, venue-level and per-instrument; a model declaring its own seed keeps it                                                                                                                                                                                                                                                                                                                                                                                                                                   | Seeding a foreign (Python) fill model's own draws; seeding the latency and slippage models                                                                                                                                                        |
| Synthetic flow                  | `crates/backtest/src/synthetic.rs` generates a persistent flow and its induced price path from a target Hurst exponent and an impact exponent bounded at one half, exposed to Python and deterministic under its seed                                                                                                                                                                                                                                                                                                                                                                                                   | The bridge, the null and robustness harnesses that consume it (W7.2, W7.3) and the variance-ratio scenario (W3.2) all landed; what remains is a flow the engine can consume directly, since the harness builds bars from the price path in Python |
| Interval memory                 | `Autocorrelation`, `VarianceRatio` and `RescaledRange` compute the lag autocorrelation, the overlapping-sum variance ratio and the rescaled-range Hurst slope from daily-binned returns, exposed to Python                                                                                                                                                                                                                                                                                                                                                                                                              | The run harness that reads them back from a backtest (W3.2) and the null and robustness harnesses (W7.2, W7.3) landed; what remains is unit-root tooling                                                                                          |
| Null-model validation           | A null-model harness feeds a generated flow into a backtest run and reports the result as a distribution over seeds, identical at one seed; a robustness runner sweeps the engine seed and the horizon rule, reports every cell as a distribution and flags a degenerate parameter set three ways (W7.2, W7.3)                                                                                                                                                                                                                                                                                                          | A reference distribution drawn from real instruments, and out-of-sample calibration                                                                                                                                                               |
| Knowledge date on the data path | A datum may carry `knowledge_date` (nanoseconds since the epoch) beside its payload; a fail-closed gate reads it, excludes a datum published after the decision time or carrying none, admits the inclusive boundary, counts every outcome by reason in the data-quality gate's shape, and names both exclusions in the bridge's closed refusal vocabulary (`DENIAL_CODES`, not retryable) (W8.2)                                                                                                                                                                                                                       | A knowledge date on the Rust model data types, and adapters that attach one where they parse a publication time                                                                                                                                   |
| News or sentiment item          | `NewsItem` carries a publication and a receipt instant, both required and distinct, as a `@customdataclass` payload: the knowledge date is the receipt, the information latency is the interval between them, the boundary instants mirror the two, and the item round trips through custom-data JSON and the parquet catalog (W8.3)                                                                                                                                                                                                                                                                                    | An adapter that emits one, so news still arrives as adapter metadata                                                                                                                                                                              |
| Text-signal leakage controls    | `mask_identifiers`, `mask_dates` and `date_only` mask the identifiers, the dates or everything but the dates in a headline, and `masked_replay` replays a caller's text signal under the identifier and date-only controls, knowledge-gated by the same rule as any other datum, reporting every count and both divergences (W8.5)                                                                                                                                                                                                                                                                                      | A corpus or adapter that supplies the text, and a model's disclosed cutoff dates as a declared field                                                                                                                                              |

### 2.3 Absent

| Capability                                   | Note                                                                                                              |
| -------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| Auctions, price bands, circuit breakers      | No model or engine primitive; adapter metadata only                                                               |
| Feed-identity validation                     | Nothing checks reported volume against open interest or a settlement total                                        |
| Cross-venue timestamp reconciliation         | Ordering is insertion order; `VirtualClock` monotonicity is per clock                                             |
| Reference distribution from real instruments | The null model is synthetic only; nothing draws a reference distribution from real instruments                    |
| Knowledge date on the Rust data types        | The bridge reads a `knowledge_date` payload field and counts its exclusions (W8.2); no Rust data type carries one |
| News or sentiment adapters                   | The item exists as a custom-data payload carrying both instants (W8.3); no adapter emits one                      |
| Model-driven strategy example                | No ML dependency and no serving hook                                                                              |
| Global kill switch                           | Closest are the Rust-only `set_trading_state` and per-account backtest liquidation                                |
| Impact decay or a transient/permanent split  | A search for `decay` in `crates/execution` and `crates/backtest` returns nothing                                  |

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
`commission` only: the empty `fees` and `slippage` fields were deleted at revision 2 (W1.3),
because nothing read them, neither was exposed to Python, and `nautilus-analysis` cannot depend on
a fee model or the analytics module. The default analyzer carries a cost row as of revision 8
(W1.2): total commissions, total turnover, gross and net return, and the cost and breakeven rates
in basis points of turnover, so a period report reads what the result cost beside the result
itself. Rust has a read-only execution analytics module under
`crates/trading/src/analytics/` (declared at `crates/trading/src/lib.rs:110`):
`ExecutionObserver` with its observation types, seventeen `METRIC_*` constants including
`METRIC_IMPLEMENTATION_SHORTFALL_BPS`, `METRIC_ARRIVAL_SLIPPAGE_BPS`,
`METRIC_VWAP_SLIPPAGE_BPS` and `METRIC_DECISION_TO_EXECUTION_DELAY_S`, and the metric vocabulary
that declares what each value is measured against.
The module is exposed to Python as of revision 7 (W1.1), so a caller can read those numbers back
without re-deriving them; the bus-integrated collector and a report row are not part of that
exposure. The deflated Sharpe lives in
`optimization/significance.py`, where the trial counts travel on `SharpeSample` rather than as a
call argument, and as of revision 3 it is a portfolio statistic, so a run's report carries the
corrected row with the trial counts in its name (W1.4).

**Work.**

- W1.1 (done at revision 7) The pure observer surface is exposed to Python: `ExecutionObserver`
  with every `set_*` and `observe_*` method, the term and observation types, the sixteen
  `METRIC_*` names, and the metric vocabulary (`Metric`, `MetricDeclaration`, `UnavailableReason`
  and the reference enums), so a caller reads implementation shortfall, arrival slippage and VWAP
  slippage back without re-deriving them. A metric arrives as a `Metric` carrying its declaration,
  so a value cannot be read without the convention that defines it; an undefined metric is `None`
  with its reason, never `0.0`, and a genuine zero stays `0.0`. Two limits are deliberate.
  `MetricValue` is not bound as a class, because `value` and `reason` carry the same distinction
  and the report layer's `MetricResult` already exposes it that way. The bus-integrated
  `ExecutionAnalyticsCollector` is not bound either: registering a Rust actor from Python needs an
  actor-registration path this crate does not own, and every metric it forwards is reachable
  through the observer. `MetricUnits` and `MetricDirection` keep their Rust names, so the
  execution vocabulary and the report vocabulary share two short names while remaining distinct
  types: passing one where the other is expected raises rather than converting.
- W1.2 (done at revision 8) The cost row is six period statistics, all registered in the default
  analyzer so a report reads them without the caller registering anything: `TotalCommissions` and
  `TotalTurnover` (which existed, registered nowhere), and four new ones: `GrossReturn` and
  `NetReturn` as a fraction of the frame's starting equity, and `CostBasisPoints` and
  `BreakevenCost` in basis points of turnover. Gross return adds the recorded commission back to
  the net PnL, so the two return rows differ by exactly the cost the frame paid; the cost rate is
  what the frame paid per unit traded, and the breakeven rate is the gross edge it earned per unit
  traded, i.e. the cost it could have absorbed. All four are classified as normalizations in the
  kernel table, which is what obliges them to be scale-invariant with a defined zero-denominator
  case, and the frame is reduced once by `PeriodFrameTotals` rather than per statistic. Two
  consequences followed. `MetricUnits` gained `BasisPoints`, because a value quoted in basis points
  had no honest unit in a vocabulary whose whole point is that a value cannot be read without its
  declaration. And the four statistics became built-ins, so the tables that classify every built-in
  statistic, the pinned built-in count and the pinned set of objective metric names all moved with
  them. One pre-existing limit is worth naming: the Python `PortfolioAnalyzer` starts empty and a
  Python caller cannot construct a period frame, because the frame's accounting and activity types
  are not exposed, so the row is registerable from Python while the acceptance below is proven
  over an engine run in Rust.
- W1.3 (done at revision 2) Decided to delete `PeriodAccounting.fees` and `.slippage`: nothing
  read either field, `nautilus-analysis` cannot depend on the fee model or on the analytics
  module, and population would have added a field to `OrderFilled` to feed two unread duplicates
  of `commission`. The cost row of W1.2 reads `commission`; the slippage numbers belong to W1.1.
- W1.4 (done at revision 3) `DeflatedSharpeRatio` reports the correction as a portfolio statistic:
  it takes the trial declaration, computes the value from the returns the analyzer feeds it, and
  the run's report carries the row once it is registered. Two premises in this item did not hold.
  There is no gross-return artifact to sit beside, so the row joins the returns statistics the
  report already renders. And a count has no unit in `MetricUnits` (currency, fraction, ratio), so
  the trial counts are stated in the row's name rather than as a second metric. The declaration is
  extrinsic to a run, so one import and one registration remain necessary; what the item removes is
  building a `SharpeSample` and calling the correction by hand. A run whose returns history falls
  below the contract's declared minimums (20 periods and 10 trials by default) reports the row as
  unavailable, and a Python statistic that returns nothing is omitted from the report rather than
  shown as NaN. Shared with W8.4.

**Acceptance.** With `MakerTakerFeeModel`, the report shows gross and net apart; setting both
rates to zero moves net onto gross and leaves gross unchanged. Registering
`DeflatedSharpeRatio` puts the corrected value and the trial counts into the run's returns
statistics beside its other rows, without building a sample or calling the correction by hand.

### T2 Impact is concave near a square root, and the prefactor is uncertain (02, 07)

**Corpus.** The exponent is robust: delta = 0.489 +/- 0.0015 on TSE, 0.50 [0.32, 0.66] on AAPL.
The prefactor is not: c runs from 0.34 to 1.50 across three markets, and reconstructing metaorders
from an anonymous tape inflates it about twofold, so the honest output is a bounded range such as
0.34 to 0.69 rather than a point (`2606.24019v1`, `2411.13965v3`).

**Today.** `crates/execution/src/models/market_impact.rs` holds the trait
(`MarketImpactModel::impact_increments(fill_quantity) -> u64`), a handle, and two implementations
as of revision 9 (W2.1): `LinearMarketImpactModel`, whose adjustment is
`floor(quantity / quantity_per_increment)` capped by `max_increments`, and
`SquareRootMarketImpactModel`, whose adjustment is
`floor(prefactor * sqrt(quantity / reference_quantity))` capped the same way.
`MarketImpactModelAny` has both variants and the Python binding accepts both. Impact applies only
to an `L1_MBP` taker fill, after the slippage adjustment
(`crates/execution/src/matching_engine/mod.rs`), which is the decision taken at revision 13 (W2.5)
and recorded on the trait: a book with more than one level prices its own depth, so the adjustment
is not consulted for a fill on another book type. The prefactor travels as an interval with the
calibration source that produced it as of revision 10 (W2.2), and the concave model applies the
interval's upper bound. The tooling that fits the interval from observed impacts, and de-biases a
tape-derived one as an explicit step, lands at revision 11 (W2.3), and a caller's own model reaches
a venue through `BacktestEngine.add_venue` as of revision 12 (W2.4). The declarative venue config
still carries built-in models only, exactly as it does for fill models.

**Work.**

- W2.1 (done at revision 9) `SquareRootMarketImpactModel` joins the trait and the same composition
  slot, with `prefactor`, `reference_quantity` and the same `max_increments` cap the linear model
  carries: `increments = floor(prefactor * sqrt(quantity / reference_quantity))`. The exponent is
  fixed at one half rather than exposed, because the corpus finds it robust across markets while
  the prefactor is not, so the prefactor is the parameter the later calibration work reports as an
  interval. The square root is evaluated in binary floating point and floored to whole increments,
  so the count is exact while the ratio under it is not rational in general; a fill whose
  continuous impact is below one increment leaves the price unchanged, as it does in the linear
  model. `MarketImpactModelAny` gains the variant, the Python binding constructs and extracts it,
  and a scenario fingerprints it end to end over the same synthetic book as the linear case: the
  first fill is the reference quantity at four increments where the linear case returns two, and
  the exhausted-volume remainder fills one book increment above it, so the pair pins the
  adjustment the model applied rather than only that it applied one. The acceptance's
  "schedule-invariant for a fixed size over volume" is read as the model holding no state between
  calls, and the test asserts that interleaving fills leaves every adjustment unchanged. The
  scenario lands in its own case module rather than in the linear one's, because the registry
  declares exactly one scenario per module.
- W2.2 (done at revision 10) The concave model carries its prefactor as a `PrefactorInterval`
  rather than a number: `lower`, `upper`, and the `ImpactCalibrationSource` that produced it,
  which is either fitted from the venue's own fills, reconstructed from an anonymous tape with or
  without the de-bias applied, or declared by the caller. A bare float is no longer accepted, so a
  tape-derived prefactor cannot be read as measured. The model applies the upper bound, which is
  the decision taken here: the pessimistic bound is the one that cannot flatter a result, and the
  regression case now calibrates its prefactor over 1.0 to 4.0 from an anonymous tape, so the
  committed fingerprint also pins which bound was applied. An interval with equal bounds is
  allowed, because a prefactor fitted from observable fills really is a point, and the source is
  what records which of the two a reader has. Both bounds and the source appear in the interval's
  display and in the model's, and the Python binding exposes the interval, so a report that prints
  the model prints what the calibration is worth.
- W2.3 (done at revision 11) `crates/execution/src/models/market_impact_calibration.rs` fits the
  prefactor from observed impacts: an `ImpactObservation` carries the aggressive quantity, the
  reference volume and the price movement in whole increments, and implies
  `increments / sqrt(quantity / reference_quantity)`. A fit reduces a series of those to the span
  of its estimates, which is the same cross-sectional reading the corpus reports when it places the
  prefactor between 0.34 and 1.50 across markets. Two entry points differ in the source they
  attribute: a fill-derived fit is measured, while a tape-derived fit carries the twofold inflation
  that reconstructing metaorders from clips introduces, so `PrefactorInterval::debiased` is an
  explicit step that divides both bounds, records the result as de-biased, and refuses a prefactor
  that was measured rather than inferred. A series in which no observation moved the price by an
  increment fits nothing, because a span of zeros is not a usable interval. The reconstruction of
  metaorders from clips stays the caller's step; the tooling fits what it is handed and records
  which of the two sources produced it. The observation type, both entry points and the de-bias are
  callable from Python.
- W2.4 (done at revision 12) A caller's own model is a Python object with an
  `impact_increments(fill_quantity)` method and nothing else, mirroring the fill-model duck typing:
  `PythonMarketImpactModel` adapts the object to the trait, and
  `pyobject_to_market_impact_model_handle` accepts a built-in binding first and any object carrying
  the method second, so a calibration does not require a rebuild. `BacktestEngine.add_venue` uses
  that conversion, which is the path the fill model already documents. A return that is not a whole
  number, and a raised exception, both abort the fill: the engine logs the error and fills nothing,
  which is the handling every fill-path error gets, so a model that cannot answer cannot produce a
  fill price. A test asserts exactly that with a real run over the shared synthetic book: the model
  is asked about the fills the engine decided, its adjustment moves the fill price, and a model
  that raises leaves the order unfilled. The declarative venue config still carries built-ins only,
  which is the fill model's own limitation and is left as it stands rather than extended for one
  concern.
- W2.5 (done at revision 13) The decision is that impact stays where it is: it applies to a
  liquidity-taking fill that consumes an `L1_MBP` book, and to no other fill. A book with more than
  one level prices its own depth through the levels a taker walks, so an increment-per-fill
  adjustment on top of it would count the same size twice. The decision is recorded on the trait,
  the code gate on `book_type == L1_MBP` is unchanged, and a matching-engine test pins both
  halves with a model that counts its consultations: the L1 taker fill consults it once and fills
  three increments above the ask, and an L2 fill consults it never and fills where the book put the
  price. The test harness that runs a taker market order now takes the book type as an argument
  rather than hard-coding the L1 case, so the two halves are the same scenario once.

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
never feeds back into later fills. Three persistence tools now sit on the analysis surface
(`Autocorrelation`, `VarianceRatio` and `RescaledRange`, W3.1); unit-root tooling is still absent,
and the only other Hurst estimator is `HurstVpinDirectional::estimate_hurst`, inside the
feature-gated examples.

**Work.**

- W3.1 (done at revision 6) Three statistics join the analysis surface, each exposed to Python and
  registered like `CalmarRatio` rather than reported by default: `Autocorrelation` (lag, default 1,
  the full-series mean with the overlapping-pair numerator), `VarianceRatio` (aggregation period,
  default 2, overlapping sums with population variances throughout) and `RescaledRange` (no
  arguments, the Hurst slope from the least-squares fit of log R/S against log window size over a
  powers-of-two ladder). Each returns nothing rather than a number when the series cannot support
  it. All three read the daily-binned returns the crate's other statistics read, so the aggregation
  period counts days and not raw observations.
- W3.2 (done at revision 20, and it corrected the plan rather than confirming it)
  `python/tests/unit/analysis/test_variance_ratio_impact.py` is the documented scenario: a persistent
  flow from the W7.4 generator priced with a linear impact response reports a variance ratio of 1.533
  at period 20, so the artifact is named rather than mistaken for a result. Measuring the concave
  response on the same flow showed the previous acceptance was wrong: the concave path is also above
  one (1.391), because the flow is persistent by construction and every monotone impact response
  leaves the induced price positively autocorrelated. What separates the models is the size of the
  inflation, which the test asserts, and the T3 acceptance above now says that instead. The
  diffusiveness condition bounds the exponent the generator accepts, not the variance ratio a
  persistent flow produces, and the linear path had to be constructed in the test because the
  generator refuses the exponent a linear model implies.
- W3.3 (done at revision 19, satisfied by W7.4 rather than reimplemented) `SyntheticFlowConfig::new`
  takes the target Hurst exponent and the impact exponent, refuses a Hurst exponent at or below one
  half and an impact exponent above one half with the diffusiveness reason, and
  `test_the_config_refuses_an_impact_exponent_above_one_half` covers the refusal. The confirmation is
  a reading of the shipped constructor and its test, not new code: the requirement is W7.4's own
  validation.

**Acceptance.** The variance-ratio tool reports above one for the linear-impact synthetic run, and
reports less inflation for the concave model on the same flow (also above one, because the flow is
persistent by construction and every monotone response leaves the induced price autocorrelated).
The same tool is callable from Python.

### T4 Passive fills are adversely selected (03, 04)

**Corpus.** Passive fills line up with subsequent adverse moves: formally the expected drift
conditional on a fill is negative whenever the fill rate is below one, and the under-fill and
over-fill penalties are large enough to consume the effective rebate (brief 03). Fill
inter-arrivals are heavier-tailed than exponential.

**Today.** Passive fills are probabilistic (`ProbabilisticFillState`) and seeded, and as of
revision 14 a model can also condition its fill decision on what the book and the recent flow say:
`PassiveFillContext` carries the queue ahead, the order's own quantity and a signed trade-flow
toxicity, the `FillModel` trait gained `is_limit_filled_with_context` as a provided method that
defaults to the old decision, and `AdverseSelectionFillModel` (W4.1) makes a resting order fill
less often the more is ahead of it and the more the flow has turned against it. Queue position and
liquidity consumption exist on `OrderMatchingEngineConfig` but both default to false, and an
imbalance-adjusted microprice valuation for passive fills exists off by default (W4.2). A user fill
model still receives only the best bid and ask and returns a synthetic book
(`FillModel::get_orderbook_for_fill_simulation`), never the real one. Each built-in model declares
its passive fill assumption as of revision 15 (W4.3), and the declaration travels with the handle a
venue is configured with.

**Work.**

- W4.1 (done at revision 14) A passive model can now condition its decision on what the engine
  knows: `PassiveFillContext` carries the order's side and quantity, the quantity resting ahead of
  it, and a signed trade-flow toxicity in `[-1, 1]` whose magnitude is the last trade's size as a
  fraction of the order's own quantity and whose sign is whether that flow ran against the resting
  side. The `FillModel` trait gained `is_limit_filled_with_context` as a provided method that
  delegates to `is_limit_filled`, so every existing model, and the Python duck-typed one, keeps
  exactly the decision it made before. Toxicity is the last trade's aggressor direction rather
  than the book's imbalance, because the plan asks for flow as well as the book and the aggressor
  direction is the only genuinely flow-shaped signal available at the decision point; it needed one
  engine field (`last_trade_aggressor`), set and cleared with the last trade size, so a decision
  driven by a quote carries no signal rather than a stale one. `AdverseSelectionFillModel` makes
  the fill probability
  `p = prob_fill_on_limit * exp(-(queue_sensitivity * queue_ahead / (queue_ahead + order_quantity)
  + toxicity_sensitivity * max(toxicity, 0)))`, which is strictly falling in the queue ahead and
  never rising in adverse flow; it is seeded through the same `ProbabilisticFillState` the
  probabilistic model uses, responds to the engine-level seed, validates its parameters, appears in
  `FillModelAny`, `FillModelKind` and `FillModelConfig`, and is constructible from Python. The
  acceptance is pinned by tests: on a neutral context the model draws exactly as the probabilistic
  model does at the same seed, and on an adverse context it fills fewer times.
- W4.2 (done at revision 14) A passive fill can be valued at the imbalance-adjusted microprice of
  the touch instead of the order's own limit price:
  `microprice = (best_bid * ask_size + best_ask * bid_size) / (bid_size + ask_size)`, the standard
  opposite-side weighting in which the resting size that the next aggressive trade consumes prices
  the touch. The valuation is bounded by the maker's limit, so a BUY never values above its limit
  and a SELL never below, and it is gated by `OrderMatchingEngineConfig::passive_fill_microprice`,
  which defaults to false: with the flag off, or with no two-sided size at the touch, the limit
  price is kept and behaviour is unchanged. Tests pin that a balanced touch values at the mid, that
  a heavy side pulls the price toward it, and that the buy and sell bounds hold.
- W4.3 (done at revision 15) The fill assumption is declared rather than implied: `FillAssumption`
  names the three classes the plan lists (`Full`, `Probabilistic`, `AdverseSelected`), the
  `FillModel` trait gained `fill_assumption` as a provided method whose default declares nothing,
  and the built-ins declare what they are: a model whose passive probability is one is full-fill,
  anything less is probabilistic, and the adverse-selection model is adverse-selected. A model that
  declares nothing reads as undeclared rather than being assumed to fill in full, which is the
  direction that cannot flatter a result. The declaration travels with the `FillModelHandle`, so a
  venue configuration printed through it names the assumption a result was produced under, and the
  enum and the per-model getters are exposed to Python so a report of the caller's own can print
  it. Two deviations from the item's wording are deliberate. It is not a report row: the fork's
  report is portfolio-level and a fill assumption is a venue property, so there is no row to put it
  in, and the assumption is named wherever the venue's fill model is printed instead. And the
  declarations cover the built-ins whose passive decision is a probability or the queue and flow;
  the remaining models are left undeclared rather than guessed at, which the row reports honestly.

**Acceptance.** The same strategy's passive result falls when adverse selection is enabled, and
changes only through the seed when it is not. The report names the fill assumption used.

### T5 Speed is ordinal, and the aggressive ceiling is collapsing (05)

**Corpus.** The closest trader to the book averaged $2,681.04 a day while the second closest lost
$3,297.30, so the edge lives in rank. Aggressive upper-bound profit for 19 NASDAQ stocks fell from
$3.4 billion at a 10 s holding period to $62,000 at 10 ms. Cross-venue lead-lag carries a
single-vantage offset ambiguity of about +/-99 ms even with drift bounded to 6 ms.

**Today.** `crates/execution/src/models/latency.rs` has a `LatencyModel` trait over base, insert,
update and delete latency, with exactly one implementation, `StaticLatencyModel`. Latency applies
to order commands only; there is no market-data or risk latency. A competitor set and its ordinal
read exist as of revision 30 (W5.1): `crates/execution/src/models/competition.rs` declares a cohort
of rival latencies and reads the order's arrival rank at the decision instant, the backtest exchange
computes that rank from the venue's own insert latency where the arrival is already known, and the
rank reaches `PassiveFillContext::competitor_rank`, where `AdverseSelectionFillModel` scales its
passive probability by `1/rank` above rank one. What is still absent is a rank anywhere but the
passive path, and any cohort in the sandbox execution client.
`crates/execution/src/python/latency.rs` exposes a `LatencyModel` base class and accepts any
Python object carrying the four leg methods as of revision 31 (W5.2), so a Python model reaches a
backtest through `BacktestEngine.add_venue` while the declarative venue configs stay built-ins-only,
exactly as the fill model's do. The decision-to-execution delay is a first-class metric as of
revision 32 (W5.3): the execution analytics metric set carries `decision_to_execution_delay_s`,
measured from the declared decision instant to the first fill, so a live run can read how long the
market took to answer the agent rather than only how far the price moved while it waited.
Cross-venue ordering in the backtest is insertion order, and the message bus dispatches
synchronously with a static priority.

**Work.**

- W5.1 (done at revision 30) `crates/execution/src/models/competition.rs` declares a `CompetitorSet`
  -- a cohort of rival latencies, with `new`, `uniform` and the declared rivals readable back -- and
  its read, `rank_at_decision_time(our_latency, ts_decision) -> ArrivalRank`. The read is ordinal by
  construction: our order and every rival share the decision instant, so that instant cancels and
  only the latency differences matter, which is what the corpus says when the venue's closest trader
  earned while the second closest lost. `ArrivalRank` carries our arrival, the rank (`1 + ahead`),
  the ahead/tied/behind split, whether we arrived first, and the gap to the closest rival arriving
  later; the comparison is strict, so a rival tied with us is not ahead of us. A `CompetitorSetHandle`
  shares one cohort between the config and the exchange. The rank is computed where the arrival is
  already known and nowhere else: the backtest exchange holds the only latency model, so
  `generate_inflight_command` ranks each submission there from the same insert latency it uses for
  the arrival timestamp, the rank travels with the inflight command through the message queue, and
  `OrderMatchingEngine::set_competitor_rank` stores it per client order id for
  `passive_fill_context` to read as `competitor_rank`, which is new on `PassiveFillContext`
  (and is `0` when no cohort is configured, so no model that ignores it changes behaviour).
  `AdverseSelectionFillModel::fill_probability` reads it: the order is one of `rank` at the level, so
  above rank one the probability is scaled by `1/rank`, and rank `0` or `1` leaves the value exactly
  as it was. The acceptance is pinned at both ends rather than asserted as one number: a rival moving
  across our latency changes the rank (`test_strictly_faster_rival_gives_rank_two`) and a changed
  rank changes a seeded model's decision (`test_competitor_rank_flows_into_the_passive_fill_decision`),
  so moving one competitor's latency moves the fill outcome; and adding a shared offset to our
  latency and every rival's, or scaling them all, leaves the rank unchanged
  (`test_rank_is_ordinal_under_shared_offset_and_scaling`), with rank `0` and `1` giving bit-identical
  decisions, so moving the whole cohort together does not. Named gaps: no Python surface (a Python
  fill model cannot see the context at all), no rank on the aggressive path, no cohort in the sandbox
  execution client, and the `use_message_queue = false` dispatch mode bypasses both latency and the
  rank, as it already bypasses latency.
- W5.2 (done at revision 31) A Python latency model is now a real protocol rather than a built-in
  extraction. `crates/execution/src/python/latency.rs` exposes `LatencyModel` as a subclassable base
  class whose four leg methods return nanoseconds and default to zero, so a user overrides only the
  legs they care about, and `pyobject_to_latency_model_handle` accepts any object carrying all four
  methods after trying the built-ins, wrapping it in an adapter that reads the Rust trait. The
  `LatencyModel` trait keeps its infallible signature, so the adapter's error convention is pinned
  and documented: a raised exception, a missing method or a non-integer return panics naming the
  call -- `Python LatencyModel.get_insert_latency failed: ...` -- because substituting a value for a
  latency that cannot be read would silently mis-time every arrival in the run, which is the same
  refusal the market-impact adapter documents. Only `BacktestEngine.add_venue` switched to the
  handle converter: the declarative `BacktestVenueConfig` and the sandbox config stay built-ins-only,
  which is the asymmetry the fill model already has, and the `LatencyModelAny` converter is
  untouched, so `latency_model()` still round-trips built-ins. Registration in `python/mod.rs` plus
  the stub regeneration put `LatencyModel` in `nautilus_trader.execution` the same way `FillModel`
  is there, and the debug extension was rebuilt before the Python test ran. Tests: four in the
  binding's module (an inline Python type's four legs reach the trait, a model missing a method is
  refused by name, a built-in still converts, and a raising model panics with the call, the
  exception type and its message), and one behavioural Python test -- a `LatencyModel` subclass
  injected through `add_venue` delays a limit order past the run's three-minute bar window so it
  fills zero times, while the same order with a zero-latency model fills. That test also exposed
  something worth recording: the backtest's shutdown path settles inflight commands against the
  final market, so the first draft's marketable limit filled anyway; the committed version rests
  below the walked-up market, which makes arrival time the deciding factor. One stated limit: the
  protocol exposes `get_base_latency`, but the exchange reads only the three order legs, exactly as
  it does for `StaticLatencyModel`, which folds base into the legs at construction.
- W5.3 (done at revision 32) The decision-to-execution delay is now a first-class metric rather than
  a duration each caller measures for itself, which is what brief 09 asks for when it says the market
  does not wait for the agent. `crates/trading/src/analytics/metrics.rs` declares
  `METRIC_DECISION_TO_EXECUTION_DELAY_S` (`decision_to_execution_delay_s`) as the module's
  seventeenth metric, carried on `ExecutionMetrics` beside the decision-price slippage it prices, so
  the delay itself is readable and not only the price move it caused. It is measured from the
  declared decision instant to the **first** fill: the market answering the order is what ends the
  delay, and the value is readable while the parent is still working, unlike the completion time,
  which needs the terminal event. That choice is pinned by evidence rather than asserted -- the
  recorded scenario declares a decision at 1.85s with fills at 2.4s and 2.6s, and the metric reads
  0.55 seconds where measuring to the last fill would read 0.75. Its declaration names seconds,
  lower-is-better and the decision reference timestamp, so the value cannot be misread as a price
  move. Two absences stay distinct rather than collapsing into one: with no declared decision instant
  it reports `NotAvailable(NoTimestamp)`, because there is no instant to measure from, and with a
  declared decision and nothing filled it reports `NotAvailable(NoObservations)`, because nothing has
  executed yet; a fill recorded before the decision instant yields a real `0.0` rather than a
  negative delay, which is the module's absence-versus-zero rule. The constant is registered
  explicitly in `crates/trading/src/python/mod.rs` beside the other sixteen, because that binding is
  an `m.add` list rather than something derived from the Rust constants -- the first smoke run failed
  on exactly that omission, which is why the metric is asserted through the real interpreter and not
  only in Rust. One stated choice: `docs/concepts/live.md` documents the live runner's dispatch
  counters, not the execution metric vocabulary, so the metric is documented where the vocabulary
  lives, in `docs/usermanauls/execution-algorithms/06-measure-and-evaluate.md`, whose metric table
  gained its row. That page asserted the analytics surface was unreachable from Python, which had been
  stale since revision 7 (W1.1); the claim and the recorded test run beside it are corrected here,
  the latter because the two new tests changed the suite from seven to nine. Named gap: the collector
  is still not exposed, so a live Python caller drives the observer itself rather than a registered
  actor.
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
a late bar is dropped. `QuoteTick::new_checked` enforces precision equality and nothing else, and
`TradeTick::new_checked` a positive size. A data-quality gate now covers quotes and trades as of
revision 16 (W6.1, W6.2): `DataEngineConfig.data_quality_action` is `None` by default, and when set
to `Flag` or `Drop` the engine checks every quote and trade for a crossed touch, a non-positive
value and an out-of-order `ts_event` per instrument, counts each violation by kind, and either
forwards or refuses the record, with the counts reported in the backtest summary rather than only
logged. Adapters still log and substitute `ts_init` for an unparseable venue timestamp without
counting it. The `OrderBook` warns on an out-of-order update and clamps its high-water mark. The
Python bulk converters enforce monotonically increasing `ts_init`, and the matching engine skips a
quote or trade older than the book. Feed-identity checks now exist as pure validators an adapter runs
over its own feed (W6.3): whether a venue's reported change in open interest is explicable by the
volume traded over the same interval, and whether a reported settlement total reconciles with its
components, each returning the gate's vocabulary so an adapter counts it in the same totals. A
cross-venue clock offset can now be estimated with the bound its samples support
(`ClockOffsetEstimator`), and the bybit adapter counts a `ts_init` substitution rather than only
logging it. No cross-venue reconciliation exists, and the other adapters that fall back to `ts_init`
still only log.

**Work.**

- W6.1 (done at revision 16) `crates/data/src/engine/quality.rs` holds the gate's vocabulary and
  checks: `DataQualityViolation` names the three kinds it detects on a quote or a trade (a crossed
  touch where the bid is above the ask, a non-positive price or size, and an out-of-order
  `ts_event`), `DataQualityAction` is what the gate does with a violating record (`Flag` forwards it
  and counts it, `Drop` refuses it and counts it), and `DataQualityCounts` carries the per-kind
  counts and the accepted total, modelled on the rejection counts the volatility surface already
  keeps. `DataEngineConfig.data_quality_action` selects the action and defaults to `None`, which
  applies no checks and records nothing, so every existing configuration is unchanged, and the live
  data-engine config carries the same field through its conversion. All four ingest paths, the live
  quote and trade handlers and the two replay pipeline handlers, go through one helper, which is
  what keeps them from drifting. The order check compares against the engine's own last-seen
  `ts_event` per instrument rather than the cache, because the replay pipeline does not always write
  the cache and a cache-based check would silently skip records. This is also how the item's
  "optional reordering" is expressed: `Flag` accepts an out-of-order record and counts it, leaving
  any reordering to the consumer, while `Drop` refuses it.
- W6.2 (done at revision 16) The counts are the gate's own, and they reach the backtest report: the
  engine builds its summary from the data engine's counts and adds one `data_quality` entry naming
  the total and each kind's count, but only when at least one record was rejected, so a run with the
  gate off or a clean run keeps the summary it had. A number in the report replaces a warning in the
  log, which is what the item asks for. The two items are committed together because the counts are
  what the gate does with a violation rather than a separable layer. One limit is deliberate: the
  live node exposes no counter surface today, so the counts are read from the data engine in Rust
  rather than through a live report.
- W6.3 (done at revision 17) The gate's vocabulary carries two more kinds and the pure validators
  that produce them, for the checks the engine cannot make because it never sees the numbers: a
  venue's reported change in open interest versus the volume traded over the same interval
  (`validate_open_interest_change`), and a reported settlement total versus the sum of its
  components within a tolerance (`validate_settlement_total`). The fork has no open-interest data
  type, so the open-interest check is the conservation identity an adapter can evaluate from two
  reports: open interest moves only through trades and one traded unit moves it by at most one unit,
  so the absolute change cannot exceed the traded volume. The settlement check requires every
  component and the tolerance to be in the reported total's currency, because a component in another
  currency is itself a mismatch. Both return a `DataQualityViolation`, and `DataQualityCounts::record`
  is now public so an adapter's own check is counted in the same totals the gate reports.
- W6.4 (done at revision 18) Two halves. `ClockOffsetEstimator`, in `crates/common/src/clock/offset.rs`
  so every crate that holds a clock can reach it, pairs a venue timestamp with the local one and
  reports an offset with the bound the evidence supports: a bucket's smallest reading is the least
  contaminated one, because a delay in flight can only make the venue's timestamp look older, and the
  spread of the bucket minima over a twelve-bucket window is the drift bound. It refuses to estimate
  from fewer than three completed buckets, because a bound drawn from less is a bound the data cannot
  support, and it discards a pair whose difference cannot be a clock offset at all rather than let one
  bad timestamp widen every bound after it. The second half is the counted substitution: the bybit
  wallet path, which falls back to `ts_init` when the venue's `creation_time` cannot be read, now
  counts the substitution on the dispatch state it already carries, names the running count in the
  warning, and exposes the count through `WsDispatchState::timestamp_substitutions`. The remaining
  adapters that fall back to `ts_init` (binance, coinbase, polymarket, architect_ax) still only log,
  and the architect_ax overflow sites fall back to zero rather than to `ts_init`; the section's Today
  line says so.

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
period frame against the portfolio authority. No robustness sweep or out-of-sample calibration
exists, and no reference distribution is drawn from real instruments. A synthetic flow generator
ships
(`crates/backtest/src/synthetic.rs`, W7.4), the engine-level seed exists
(`BacktestEngineConfig.random_seed`, W7.1), and a null-model harness now feeds a generated flow into
a run, reports a distribution over seeds, and sweeps the horizon rule with a degenerate-set check
(`python/tests/unit/backtest/test_null_model_harness.py`, W7.2 and W7.3). What does not ship is an
out-of-sample calibration and a reference distribution drawn from real instruments.

**Work.**

- W7.1 (done at revision 4) `BacktestEngineConfig.random_seed` seeds simulation components that
  declare no seed of their own. `SimulatedVenueConfig::seed_fill_models` applies it to the venue's
  fill model and to every per-instrument override before the exchange is built, and a model that
  declares its own seed keeps it, so an unset engine seed leaves the draws unseeded exactly as
  before. All eleven fill models carry the same probabilistic state, `MarketHours` included, so the
  seed reaches each of them; a foreign fill model is unaffected because the two trait methods have
  defaults. What is not seeded from here is a foreign model's own draws and the latency and
  slippage models.
- W7.2 (done at revision 23) `python/tests/unit/backtest/test_null_model_harness.py` runs a
  trend-following probe over a generated flow and reports the result as a distribution over eight
  seeds rather than as one number: the persistent arm (Hurst 0.6) reports min -0.06, q1 14.94, median
  27.15, q3 49.71 and max 55.86, and the memoryless arm (Hurst 0.51, the lowest the generator
  accepts) reports min -24.61, q1 -10.51, median 9.60, q3 17.29 and max 37.54, on the same seeds,
  while the same seed run twice reproduces the statistic exactly. That is both halves of T7's
  acceptance: identical at one seed, a reported spread across seeds. The harness belongs in Python,
  where the generator (`SyntheticFlowConfig`, W7.4), the engine seed
  (`BacktestEngineConfig.random_seed`, W7.1) and the result surfaces already meet, and the bridge
  W7.4 named as missing is the harness's own bar builder: the flow's price path starts at zero and can
  go negative, so bars are built on a price base, and the matching engine skips a bar whose volume
  precision differs from the instrument's — which rejects every order as "no market" without saying
  why — so the volume is built at the instrument's size precision.
  The null itself needs no purchased data; a vendor series enters only as the comparator that says
  where a real instrument's statistic falls inside the null band. If that comparator is added, the
  spec the statistic imposes is: one-minute bars as the primary rung plus 5m, 1h and 1d for the same
  instrument from one source, because the horizon sweep of W7.3 needs the ladder and mixing vendors
  across rungs contaminates it; roughly ten thousand bars per series, since the Lo-MacKinlay variance
  of a variance ratio at a scale of twenty is about 0.157 at one thousand bars and 0.050 at ten
  thousand, which rules daily data out; three to five instruments across asset classes, so the
  cross-sectional spread is a second independent distribution; prices adjusted for splits and
  dividends, since EODHD documents its intraday data as unadjusted and an unadjusted series carries
  fake jumps; UTC timestamps with session semantics and the DST question answered; volume retained
  for the impact and fill models; and a provenance header plus a pass through the W6.1-W6.3 gate
  before the series feeds a run. Three providers are already wired here: `nautilus-databento`
  (historical and live, billed per uncompressed gigabyte, US equities from 2018 and CME Globex with
  the full book), `nautilus-tardis` (crypto and derivatives, one synchronized clock at 100ns, replay
  support) and `nautilus-eodhd` (daily, weekly and monthly only, so intraday would have to be added to
  that adapter). A free comparator fixture exists without new integration: crypto klines or trades
  through the shipped Binance and Bybit path, or a Tardis trial month. Whatever is chosen, purchased
  data is never committed: the fetcher plus a free-source or synthetic fixture goes in, vendor data
  stays git-ignored. A cross-venue read is paired with `ClockOffsetEstimator` and carries its
  `drift_bound_ns`, or it is not printed at all (W5.4).
- W7.3 (done at revision 24) The robustness runner lives in the same module as W7.2's harness and
  sweeps two axes: the engine seed and the horizon rule, the latter as the number of bars the probe's
  direction rule looks back. Every cell is reported as a distribution over six seeds, and the reported
  quantity is the paired difference between the two regimes, so the claim under test is "more on a
  persistent flow than on a memoryless one at the same seed and horizon". At a lookback of one bar the
  runner raises no flag — the difference is positive at every seed (min 4.07, q1 5.15, median 7.39, q3
  10.15, max 13.73), so the verdict holds and the spread does not swallow the effect. At a lookback of
  eight bars it raises two flags: the spread is wider than the effect (an interquartile spread of 20.73
  against a median of 14.83) and the verdict flips between seeds (the difference runs from -4.30 to
  22.23). That is the runner doing its job — the persistent-flow advantage is not resolvable at that
  horizon on six seeds, and a single mean would have hidden exactly that. At a lookback longer than the
  sample the probe never receives a signal, every seed returns exactly zero, and the runner raises "no
  movement across seeds". A cell is therefore reported as degenerate when its statistic does not move
  across seeds, when its interquartile spread exceeds the effect it is meant to support, or when its
  verdict changes sign across seeds.
- W7.4 (done at revision 5) `crates/backtest/src/synthetic.rs` ships the generator, exposed to
  Python as `SyntheticFlowConfig` and `SyntheticFlow` in the backtest module. It takes a target
  Hurst exponent and an impact exponent, both validated at construction: the Hurst must lie
  strictly inside (0.5, 1.0) because 0.5 is the memoryless boundary, and the impact exponent
  inside (0.0, 0.5], refused above one half with the diffusiveness reason, which is W3.3's
  requirement. The flow is a truncated fractional moving average whose coefficients follow from
  the Hurst exponent, drawn from a seeded `StdRng` and normalised so the marginal variance does
  not depend on the target; the price path is the cumulative impact of each period's flow at the
  declared exponent, with a unit impact coefficient. One test pins a variance ratio above one for
  a generated persistent flow, another pins the price change as exactly the declared power of the
  flow, and the rest pin the refusals and the seed's determinism. What does not ship is the bridge
  into a run, which W7.2's harness supplies as of revision 23 by building bars from the price path.

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
consumed by `relative_value.py`. The correction reaches the report as of revision 3 (W1.4):
`DeflatedSharpeRatio` is a `PortfolioStatistic`, so a registered run prints it beside its returns
statistics with the trial counts in the row's name, and the tearsheet renders that section
generically. A masked-replay probe exists as of revision 29 (`decision_bridge.masking`): it replays
the same records through the same text signal with the identifiers masked and with only the dates
kept, knowledge-gated by the same rule as any other datum, and reports where the replays diverge.
`crates/research` enforces point-in-time structurally (`Panel::check` rejects a feature whose
`as_of` is after the row's `ts_event`; `AdmittedDecision.available_at`; membership spells) and now
reaches Python as the `nautilus_trader.research` module (W8.1). What it still lacks is a dependent
inside the engine, a knowledge date on any Rust model data type, an adapter that emits a news item,
and any ML integration or model-driven example.

**Work.**

- W8.1 (done at revision 25) `crates/research` now ships Python bindings, registered as the
  `nautilus_trader.research` module: `FeatureValue`, `PanelRow`, `Panel`, `MembershipInterval` and
  `MembershipSeries`, with fallible calls mapped to `ValueError` and instants exchanged as
  nanoseconds, because `UnixNanos` is not a Python type on this boundary and the repo's convention is
  to pass `u64`. The crate gained the same `python`/`extension-module` feature pair the analysis crate
  uses, `nautilus-pyo3` registers the submodule next to `analysis`, and the Python facade is
  `python/nautilus_trader/research/__init__.py`. Two facts about the Rust surface shape the binding:
  `as_of` is a public field of `FeatureValue` rather than a method, so it is exposed as a property, and
  `MembershipInterval` mirrors its entry instant onto both `ts_event` and `ts_init`. The catalog round
  trip (`write_to_catalog`/`read_from_catalog`) is deliberately not bound, because it would couple this
  crate's Python surface to nautilus-persistence's. The binding is proven from Python by
  `python/tests/unit/research/test_point_in_time.py`, which pins the same rules as
  `crates/research/tests/leakage.rs`: a feature observed after its row timestamp is rejected with the
  feature named, the row timestamp is an inclusive boundary, and membership is resolved from the
  stored series, including the half-open interval and the claimed-but-absent membership failure. This
  is the enabler for everything else in T8.
- W8.2 (done at revision 26) The knowledge date is a convention over a payload the bridge already
  carries, not a change to `CustomData`: a datum may carry `knowledge_date` (nanoseconds since the
  epoch) beside its payload, and `python/nautilus_trader/decision_bridge/knowledge.py` reads it
  (`KNOWLEDGE_DATE_FIELD`, `knowledge_date_of`), tests it (`admitted_at`) and counts every outcome
  (`KnowledgeCounts`). The rule is exclusive and fail-closed, in that order of precedence: an absent —
  or present-but-unusable — knowledge date is excluded and counted `missing_knowledge_date`, because
  an absent provenance cannot be shown to have been knowable; a date strictly later than the decision
  time is excluded and counted `published_after_decision`; a date at or earlier than the decision time
  is admitted, so the boundary is inclusive. The gate reads no clock, so replaying the same records
  against the same decision time gives the same answer. Both exclusions are named in the bridge's own
  closed vocabulary — `RefusalCode.PUBLISHED_AFTER_DECISION` and `MISSING_KNOWLEDGE_DATE` in
  `contract.py`, and both added to `DENIAL_CODES` in `execution.py` — but deliberately not to
  `RETRYABLE_DENIAL_CODES`, because an exclusion is terminal rather than retryable. The counts render
  as `knowledge: considered=N admitted=N excluded=N published_after_decision=N
  missing_knowledge_date=N`, mirroring the data-quality gate's shape so a run reports a number rather
  than a log line. What the item does not do is wire the gate into the artifact admission path: no
  existing call site changes result, so the exclusion is available and counted when a caller consults
  it rather than applied behind every caller's back.
- W8.3 (done at revision 27) A news or sentiment item now exists as the `@customdataclass` payload
  `NewsItem` in `python/nautilus_trader/decision_bridge/news.py`: `item_id`, `source`, `headline`,
  `publication_ts`, `receipt_ts`, `symbols` and `sentiment`, with the two instants required and
  distinct. The knowledge date is the receipt instant, not the publication, because at a decision time
  a process can only read what it had already received; the publication instant stays on the item
  because it answers the other question -- which session the news may be tradable in, and how much
  latency was paid to trade it -- and the two are carried together rather than collapsed, since using
  the publication instant as availability grants the process an article it had not received, and using
  the receipt instant as the event time moves the news to the wrong session. `latency_ns` is the
  difference; `NewsItem.received()` mirrors the two instants onto the custom-data boundary (`ts_event`
  is the publication, `ts_init` the receipt) so a caller cannot leave the boundary at the epoch; and
  the constructor is fail-closed, refusing a receipt that precedes publication or a negative instant
  rather than reading it as the epoch. Proving it is a real payload took the platform path, not the
  object alone: the tests gate a stream of items, then round trip one through custom-data JSON and
  through a parquet catalog and gate the decoded item again. What it is not is wired to an adapter:
  nothing emits a `NewsItem` yet, so news still reaches the system as adapter metadata, and that gap
  is recorded in the plan's absent table rather than left implied.
- W8.4 (closed at revision 28; satisfied by W1.4 at revision 3) The item asked for the deflated
  Sharpe beside the gross and net figures with its trial count, and it is the same work as W1.4,
  which the plan had already marked shared. The mechanism exists: `DeflatedSharpeRatio` is a
  `PortfolioStatistic`, so registering it puts the corrected value into the run's returns
  statistics, and the row's name states the trial counts because a count is provenance rather than a
  performance metric -- `Deflated Sharpe Ratio (12 trials)`, or `(12 trials, 2 effective)` for a
  study that declares dependence. Two premises in the item's text did not hold and W1.4 records
  them: there is no gross-return artifact in that section to sit beside, so the row joins the returns
  statistics the report already renders, and nothing else moved. Closing it needed the report path
  rather than a reading of the code, so it was measured: 24 returns and a 12-trial declaration
  through `PortfolioAnalyzer`, registered, give the row `Deflated Sharpe Ratio (12 trials)` at
  0.7531, equal to `calculate_from_returns` on the same series, and the tearsheet rendered from those
  statistics carries both the row name and the value in its Returns Statistics section -- which is
  T8's acceptance sentence, read literally. Nothing in the report path needed changing, so nothing
  was changed.
- W8.5 (done at revision 29) `python/nautilus_trader/decision_bridge/masking.py` carries the probe and
  its two controls. `mask_identifiers` replaces each identifier the caller names, whole-word and
  case-insensitively and longest first, so naming both `Apple` and `Apple Inc` does not leave a
  fragment of the longer one behind and `pineapple` is untouched; `date_only` reduces the text to the
  dates it carries, which is the input that exposes a signal deciding from the period rather than the
  record. `masked_replay` calls the signal three times per admitted record -- the full text, the text
  with identifiers masked, and the dates alone -- and counts where each control's decision differs
  from the unmasked one. The signal is a duck-typed callable taking the text and returning a decision
  or nothing, the same extension convention the fill and impact models use, so nothing here needs to
  know what a decision means. The replay is knowledge-gated by W8.2's rule: a record whose knowledge
  date is absent or later than the decision time is excluded and never replayed, so a probe cannot
  report a decision that was not knowable when it was made, and the report carries the gate's counts
  and renders all eight counts as one line. The probe reads no clock, so the same records against the
  same decision time give the same report. Built on `testkit` as the item asks: the tests load the
  real economic calendar through `TestDataProvider` and build `NewsItem` records (W8.3) from it,
  which is what makes the controls observable rather than asserted -- a stand-in reading the leading
  currency code answers on all seven admitted records and on none of them once the identifiers are
  masked, the same stand-in with nothing masked diverges nowhere, and a stand-in reading the date
  alone answers on all seven from the dates alone. Measured: `considered=12 admitted=7 excluded=5
  decisions_unmasked=7 decisions_masked=0 decisions_date_only=0 changed_by_masking=7
  changed_by_date_only=7` with the codes masked, `changed_by_masking=0` with none masked, and
  `decisions_date_only=7 changed_by_date_only=0` for the date-only control.

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

| Revision | Date       | Change                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| -------- | ---------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1        | 2026-10-05 | First plan, from the ten findings in `README.md` and a symbol-level survey of this repository.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| 2        | 2026-10-05 | W1.3 done: `PeriodAccounting.fees` and `.slippage` are deleted. T1's Today line and the 2.2 net-of-cost row are rechecked against the tree.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| 3        | 2026-10-05 | W1.4 done: the deflated Sharpe is a portfolio statistic, so a run's report carries the row. T1's W1.4 text records the premises that did not hold (no gross-return artifact exists, and the metric vocabulary has no count unit) and the contract minimums that make a short run report the row as unavailable.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| 4        | 2026-10-05 | W7.1 done: `BacktestEngineConfig.random_seed` seeds every built-in fill model that declares no seed, venue-level and per-instrument, so a run is reproducible end to end. The engine-level seed row moves from 2.3 absent to 2.2 partial, with the limits stated: a foreign fill model and the latency and slippage models are not seeded from it.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| 5        | 2026-10-05 | W7.4 done: the synthetic flow generator ships with the Hurst and impact-exponent calibration, the exponent refused above one half, exposed to Python and deterministic under its seed. The synthetic-flow row moves from 2.3 absent to 2.2 partial, and T7's Today line is corrected: the engine seed exists as of revision 4 and the generator ships now, while neither is yet consumed by a run.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| 6        | 2026-10-05 | W3.1 done: autocorrelation, variance-ratio and rescaled-range statistics exist on the analysis surface and are callable from Python, reading the same daily-binned returns as the other statistics. The interval-memory row moves from 2.3 absent to 2.2 partial and T3's Today line is corrected. P0 is complete; the acceptance that compares a linear and a concave impact run on one flow stays with W3.2, which needs the concave model of W2.1.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 7        | 2026-10-05 | W1.1 done: the execution analytics observer, its observations and its metric vocabulary are callable from Python, with the declaration travelling beside every value and unavailability reported as `None` plus a reason. A new 2.2 row records what the access is and what is still missing, a collector binding and the report row of W1.2. T1's Today line is corrected, including a constant count that said fifteen where the module declares sixteen.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| 8        | 2026-10-05 | W1.2 done: the default analyzer carries a cost row of six period statistics, commissions, turnover, gross and net return, and the cost and breakeven rates in basis points of turnover, with the frame reduced once and the four new kernels classified as normalizations. `MetricUnits` gained `BasisPoints`, because a basis-point value had no honest unit. The net-of-cost row's missing column narrows to the fill assumption of W4.3, and the pinned built-in count and objective metric-name set move with the four new built-ins.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| 9        | 2026-10-05 | W2.1 done: a concave square-root impact model joins the trait and the composition slot, with a prefactor, a reference quantity and the same cap as the linear model, constructible from Python and carrying its own fingerprint scenario over the same synthetic book. The market-impact row's missing column narrows to the interval and the calibration work, and the acceptance's schedule-invariance is read as the model holding no state between fills. The scenario is its own case module because the registry declares one scenario per module.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| 10       | 2026-10-05 | W2.2 done: the concave prefactor is a `PrefactorInterval` with its calibration source rather than a number, and the model applies the interval's upper bound, so a tape-derived prefactor cannot be read as measured or flatter a result. The regression case calibrates over a tape-derived 1.0 to 4.0 interval, so its fingerprint also pins the bound applied. The market-impact row's missing column narrows to the calibration tooling and the Python extension point.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| 11       | 2026-10-05 | W2.3 done: the calibration tooling fits the prefactor from observed price impacts and reduces them to the span of their estimates, with two entry points that differ in the source they attribute and an explicit de-bias for a tape-derived fit that refuses a measured one. The market-impact row's missing column narrows to the Python extension point of W2.4.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 12       | 2026-10-05 | W2.4 done: a caller's own impact model is a Python object with an `impact_increments` method, adapted to the trait and accepted by `BacktestEngine.add_venue`, so a calibration needs no rebuild; an exception or a non-integer return aborts the fill rather than producing one. The market-impact row's missing column narrows to the scope decision of W2.5 and the declarative config's built-ins only, which mirrors the fill model.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| 13       | 2026-10-05 | W2.5 done: impact stays on liquidity-taking fills that consume an L1 book, recorded on the trait with the reason a deeper book already prices its own depth, and pinned by a matching-engine test that counts the model's consultations on both book types. T2 is complete; the market-impact row's remaining gap is the declarative venue config's built-ins only, which mirrors the fill model.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| 14       | 2026-10-05 | W4.1 and W4.2 done together, because both edit the matching engine's passive-fill path and cannot be separated into commits that each stand alone: a passive model can condition on queue ahead and trade-flow toxicity through a provided trait method that leaves every existing model unchanged, the adverse-selection model is seeded and constructible from Python, and passive fills can be valued at the imbalance-adjusted microprice behind a flag that defaults off. The passive-fill row's missing column narrows to the fill-assumption row of W4.3 and the synthetic book a user fill model still receives.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| 15       | 2026-10-05 | W4.3 done: every built-in whose passive decision is a probability or the queue and flow declares its fill assumption, a model that declares nothing reads as undeclared, and the declaration travels with the handle a venue is configured with so a printed configuration names it. The item asked for a report row, and there is none to put it in: the fork's report is portfolio-level while a fill assumption is a venue property, so the assumption is named where the venue's model is printed and is readable from Python. T4 is complete; the passive-fill row's remaining gap is the synthetic book a user fill model still receives and the declarations the remaining built-ins do not make.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| 16       | 2026-10-05 | W6.1 and W6.2 done together, because the counts are what the gate does with a violation rather than a separable layer: a config action turns on a gate over every quote and trade that checks the touch for a crossing, values for positivity and per-instrument `ts_event` order, counts each violation by kind, and either forwards or refuses the record, with the counts reaching the backtest summary only when something was rejected so a clean run's summary is unchanged. The data-quality row's missing column narrows to the feed-identity checks and the clock-offset estimate. The net-of-cost reporting row moves from partial to in place, since W1.2's cost row and W4.3's fill-assumption declaration are both done.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 17       | 2026-10-05 | W6.3 done: the gate's vocabulary carries two feed-identity kinds and the pure validators that produce them, for the checks the engine cannot make because it never sees the numbers. The fork has no open-interest data type, so the open-interest check is the conservation identity an adapter can evaluate from two reports (open interest moves only through trades, and one traded unit moves it by at most one, so the absolute change cannot exceed the traded volume); the settlement check reconciles a reported total against the sum of its components within a tolerance and refuses a component in another currency. `DataQualityCounts::record` is public so an adapter's own check is counted in the same totals. The data-quality row's missing column is now only the clock-offset estimate and the counted `ts_init` substitution.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| 18       | 2026-10-05 | W6.4 done, in two halves. `ClockOffsetEstimator` lives with the clock in `nautilus-common` so any crate holding a clock can reach it: it applies a minimum filter per bucket, reports the spread of the bucket minima over a twelve-bucket window as the drift bound, refuses to estimate below three completed buckets, and discards a pair whose difference cannot be a clock offset. The counted substitution landed at the bybit wallet path, which already carries dispatch state: the fallback to `ts_init` increments a counter, the warning names the running count, and the count is readable through `WsDispatchState::timestamp_substitutions`. The remaining adapters that fall back to `ts_init` still only log, and the architect_ax overflow sites fall back to zero; the data-quality row names both as its remaining gap, so P2 closes with a stated inventory rather than a silent one. The estimator's own tests could not run on this machine because `nautilus-common`'s test target does not compile here for a pre-existing reason, so its behaviour was exercised through a temporary probe in `nautilus-data` (a crate whose test target does compile), which passed and was then removed.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 19       | 2026-10-05 | W3.3 marked done without new code: `SyntheticFlowConfig::new` already takes the target Hurst exponent and the impact exponent, refuses a Hurst exponent at or below one half and an exponent above one half with the diffusiveness reason, and its refusal test ships with it. The confirmation is a reading of the shipped constructor and test, recorded here rather than reimplemented, which is what the item asked for once W7.4 landed.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| 20       | 2026-10-05 | W3.2 done as a Python test over the shipped generator and the shipped statistic, and it **corrected the plan**. Measured at a Hurst exponent of 0.6, four thousand periods and a scale of twenty: the linear impact response reports a variance ratio of 1.533 while the concave response reports 1.391 on the same flow and seed. The plan's acceptance said the concave model would not be above one; it is, because a flow that is persistent by construction leaves any monotone impact response positively autocorrelated, and the concave response only inflates less. The acceptance now states the comparison that is true and the numbers behind it, and the diffusiveness condition is recorded as bounding the exponent the generator accepts rather than the variance ratio a persistent flow produces. The linear path is built inside the test because the generator refuses the exponent a linear model implies.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| 21       | 2026-10-05 | W7.2 and W7.3 scoped, not started. The plan now records where the null-model harness belongs (Python, where the W7.4 generator, the W7.1 engine seed and the result surfaces already meet), what the reference process is (the generated flow itself, persistent against memoryless), what reporting a distribution rather than one number means (minimum, quartiles and maximum over seeds, with the seed count), the first thing to implement (the bridge from a generated flow into a backtest's data, which W7.4 names as the missing half), and what makes a W7.3 parameter set degenerate. No code was written for either item in this session; the notes exist so the next one starts from the decisions instead of re-deriving them.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| 22       | 2026-10-05 | The W7.2 note gained the data decision, from a survey of the providers. The null needs no purchased data; a vendor series enters only as the comparator that locates a real statistic inside the null band, and the spec is then fixed by the statistic: one-minute bars plus 5m, 1h and 1d from one source for the horizon ladder, roughly ten thousand bars per series (the Lo-MacKinlay variance of a variance ratio at a scale of twenty is 0.157 at a thousand bars and 0.050 at ten thousand, which rules daily data out on length alone), three to five instruments across asset classes, split- and dividend-adjusted prices because unadjusted intraday carries fake jumps, UTC timestamps with the DST question answered, volume retained for the impact and fill models, and a provenance header plus a pass through the W6.1-W6.3 gate. Three providers are already wired in this fork — databento, tardis and eodhd — and only eodhd lacks the intraday endpoint, so it needs work before it can serve as the comparator; crypto klines or trades through the shipped Binance and Bybit path, or a Tardis trial month, give a free fixture with no new integration. Purchased data is never committed: the fetcher and a free-source or synthetic fixture go in, vendor data stays git-ignored. The cross-venue case is paired with `ClockOffsetEstimator` and carries its drift bound or is not printed (W5.4).                                                                                                                                                                                                                                                                                                                                                                                          |
| 23       | 2026-10-05 | W7.2 done, and with it the bridge W7.4 recorded as missing. The harness is `python/tests/unit/backtest/test_null_model_harness.py`: a trend-following probe runs over a flow generated at a chosen Hurst exponent and the result is reported as a distribution over eight seeds — the persistent arm (0.6) reports min -0.06, q1 14.94, median 27.15, q3 49.71, max 55.86, the memoryless arm (0.51, the lowest the generator accepts) reports min -24.61, q1 -10.51, median 9.60, q3 17.29, max 37.54 on the same seeds, and the same seed run twice reproduces the statistic exactly, which satisfies both halves of T7's acceptance in one artifact. The bridge is the harness's own bar builder: `SyntheticFlowConfig(...).generate().prices` -> `Bar` list -> `engine.add_data`. Two properties of the fork cost time and are recorded so they do not cost it again. First, the flow's price path starts at zero and can go negative, so bars are built on a price base rather than from the level directly, and the bar's OHLC must be ordered (open, then high/low as the extremes of open and close). Second, the matching engine skips any bar whose volume precision differs from the instrument's expected precision, and the visible symptom is not that warning but every order rejected as "no market", so volume is built at the instrument's size precision. No production code changed; the item is a test plus the plan. W7.3's dependency is therefore satisfied and its note now names the horizon ladder it will sweep.                                                                                                                                                                                                                                                                           |
| 24       | 2026-10-05 | W7.3 done, in the same module as W7.2's harness. The runner sweeps the engine seed and the horizon rule, the latter as the probe's lookback in bars, reports every cell as a distribution over six seeds, and tests the paired difference between the persistent and memoryless regimes so the claim is scoped to one seed and one horizon. All three degenerate-set flags fire on real cells rather than invented ones: a lookback of one bar raises nothing (difference 4.07 to 13.73, positive at all six seeds), a lookback of eight bars raises "spread wider than the effect" and "verdict flips between seeds" (difference -4.30 to 22.23 with an interquartile spread of 20.73 against a median of 14.83), and a lookback beyond the sample raises "no movement across seeds" because the probe never receives a signal and every seed returns exactly zero. The middle cell is the point of the item: the persistent-flow advantage is not resolvable at that horizon, and one number would have hidden it. The lookback refactor preserved the one-bar semantics exactly, which the unchanged W7.2 numbers confirm. Verified: 3 passed in 8.94s, ruff check and format clean, table check pass 2 exits 0.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 25       | 2026-10-05 | W8.1 done: `crates/research` now has Python bindings registered as `nautilus_trader.research` — `FeatureValue`, `PanelRow`, `Panel`, `MembershipInterval` and `MembershipSeries` — with the crate's `python`/`extension-module` feature pair mirroring the analysis crate, the submodule registered in `nautilus-pyo3` beside `analysis`, and a `nautilus-research` entry added to the workspace dependencies so the extension crate can depend on it. Two facts about the Rust surface shaped the binding: `as_of` is a public field of `FeatureValue`, not a method, so it is exposed as a property; and `UnixNanos` is not a Python type on this boundary, so instants cross as nanoseconds per the repo's convention, which also means `MembershipInterval` mirrors its entry instant onto both `ts_event` and `ts_init`. The catalog round trip is deliberately unbound so this crate's Python surface does not couple to nautilus-persistence's. Proof comes from Python: `python/tests/unit/research/test_point_in_time.py` reproduces the rules of `crates/research/tests/leakage.rs` through the binding — the look-ahead rejection names the offending feature, the row timestamp is an inclusive boundary, and membership resolves from the stored series including the claimed-but-absent failure — 4 passed, with `cargo nextest run -p nautilus-research` at 46 passed and `cargo check -p nautilus-pyo3 --features extension-module` clean. The debug wheel was rebuilt and its `.pyd` extracted before the tests ran, so the surface was exercised through the real interpreter rather than assumed.                                                                                                                                                                                                   |
| 27       | 2026-10-05 | W8.3 done: `NewsItem` in `python/nautilus_trader/decision_bridge/news.py`, a `@customdataclass` payload carrying **both** instants -- the publication instant and the receipt instant -- required and distinct, so the information latency is expressible rather than assumed away. The knowledge date is the receipt, not the publication: at a decision time a process can only read what it had received, so W8.2's gate becomes an availability gate in the strict sense, and an item published before the decision but received after it is excluded and counted -- the case a publication-only type would silently admit. The publication instant stays on the item for the scheduling question (which session may trade it), and `latency_ns` exposes the interval the chapter prices at 2.0 to 2.6 Sharpe between a same-day and a tradable portfolio. `received()` mirrors the two instants onto the custom-data boundary, and the constructor is fail-closed: a receipt that precedes publication, or a negative instant that would otherwise read as the epoch, is refused. Certification went through the platform path rather than the object alone: an item round trips through custom-data JSON and through a parquet catalog, and the decoded item gates identically. Measured from the interpreter: `publication 1000000000 receipt 1900000000 knowledge 1900000000 latency_ns 900000000`, with a decision one nanosecond before the receipt refused and the boundary instant admitted, rendering `knowledge: considered=2 admitted=1 excluded=1 published_after_decision=1 missing_knowledge_date=0`. The plan's absent row narrowed to the adapter that would emit an item, since the payload itself now exists. Verified: 157 passed in `tests/unit/decision_bridge`, ruff check and format clean. |
| 28       | 2026-10-05 | W8.4 closed as satisfied by W1.4 rather than reimplemented, and the plan corrected where it had said otherwise. The item -- the deflated Sharpe beside the gross and net figures with its trial count -- is the same work W1.4 did at revision 3, and the plan had already marked it shared; the mechanism is a `PortfolioStatistic` whose row name states the counts, so a run's report carries it without the caller building a sample or calling the correction by hand. Two premises in the item's text had already been corrected in T1's W1.4 entry: there is no gross-return artifact to sit beside, and a count has no unit in the metric vocabulary, so the counts live in the row name. What remained was a false sentence: T8's today text still claimed that neither the splits contract nor the significance module reached the statistics or tearsheet path, which stopped being true of the significance module at revision 3. That sentence now says what the code does, and the same claim in T1's today text was qualified the same way. Closure was measured over the report path rather than inferred: 26 significance tests pass, and a thrown-away smoke registered `DeflatedSharpeRatio` on a `PortfolioAnalyzer` fed 24 returns and a 12-trial declaration, read the row `Deflated Sharpe Ratio (12 trials)` at 0.7531, confirmed it equals `calculate_from_returns` on the same series, and rendered the tearsheet from those statistics with both the row name and the value present in the HTML. No code changed, so this commit is the plan's correction and this record.
| 29       | 2026-10-05 | W8.5 done: `python/nautilus_trader/decision_bridge/masking.py`, a masked-replay probe for text and language-model signals, built on the bridge and the testkit as the item asked. A text signal can read an identifier it saw during pretraining or a date it simply remembers, and both leaks survive a prompt instruction because the instruction is part of the input rather than a property of it. The probe replays the same records through the same signal three times -- full text, identifiers masked, dates alone -- and counts where each control's decision diverges from the unmasked one, so the dependence is reported as a number rather than argued about. `mask_identifiers` is whole-word, case-insensitive and longest-first, so `Apple Inc` does not leave `Apple` behind and `pineapple` is untouched. The signal is a duck-typed callable, the same extension convention as the fill and impact models. The replay is knowledge-gated by W8.2's rule -- a record whose knowledge date is absent or later than the decision time is excluded and never replayed -- so the probe cannot report a decision that was not knowable when it was made, and the report renders all eight counts in the data-quality gate's shape. The tests build `NewsItem` records (W8.3) from the testkit's real economic calendar through `TestDataProvider`, which is what makes the controls observable rather than asserted: a stand-in reading the leading currency code answers on all seven admitted records and on none once the identifiers are masked (`changed_by_masking=7`), the same stand-in diverges nowhere with nothing masked (`changed_by_masking=0`), and a stand-in reading the date alone answers on all seven from the dates alone (`decisions_date_only=7` at `considered=12 admitted=7 excluded=5`). The plan's partial table gained the row, and T8's today text names the probe. Verified: 168 passed in `tests/unit/decision_bridge` (11 of them new), ruff check and format clean.
| 30       | 2026-10-05 | W5.1 done: `crates/execution/src/models/competition.rs` adds a competitor set and the ordinal read T5's acceptance asks for. `CompetitorSet` declares a cohort of rival latencies and `rank_at_decision_time(our_latency, ts_decision)` returns `ArrivalRank` -- our arrival, the rank as `1 + ahead`, the ahead/tied/behind split, whether we arrived first, and the gap to the closest rival behind us -- with a strict comparison so a rival tied with us is not ahead. The read is ordinal because every participant shares the decision instant and that instant cancels, which is the corpus's point: the venue's closest trader earned while the second closest lost. The rank is computed only where the arrival is already known: the backtest exchange holds the only latency model, so `generate_inflight_command` ranks each submission from the same insert latency it uses for the arrival timestamp, the rank rides the inflight command through the message queue, and `OrderMatchingEngine::set_competitor_rank` stores it per client order id for `passive_fill_context` to read as the new `PassiveFillContext::competitor_rank`. `AdverseSelectionFillModel::fill_probability` scales by `1/rank` above rank one, because the order is one of rank at the level; rank 0 (no cohort configured) and rank 1 are exactly today's value, so nothing changes by default and no existing test or fingerprint moved. Both halves of the acceptance are pinned rather than asserted as one number: a rival crossing our latency changes the rank and a changed rank changes a seeded model's decision, so moving one competitor moves the fill outcome; a shared offset or a uniform scaling of the whole cohort leaves the rank unchanged, and rank 0 versus rank 1 gives bit-identical decisions, so moving the whole cohort together does not. Verified: `cargo fmt -p nautilus-execution -p nautilus-backtest -- --check` clean; `cargo nextest run -p nautilus-execution` 2406 passed; `cargo nextest run -p nautilus-backtest -E 'not test(canonical_backtest_workload_matrix)'` 458 passed, 1 skipped (that digest failure is pre-existing and untouched); clippy clean on both crates with `--all-targets --no-deps`; the ten new tests pass; and both crates also build with the `python` feature. Named gaps: no Python surface (and no Python fill model can see the context at all), no rank on the aggressive path, no cohort in the sandbox execution client, and the `use_message_queue = false` dispatch mode bypasses latency and the rank alike.
| 31       | 2026-10-05 | W5.2 done: a Python latency model is a protocol rather than a built-in extraction. `crates/execution/src/python/latency.rs` exposes `LatencyModel` as a subclassable base class whose four leg methods return nanoseconds and default to zero, so a user overrides only the legs they care about, and `pyobject_to_latency_model_handle` accepts any object carrying all four methods after trying the built-in bindings, wrapping it in an adapter that reads the Rust trait. The trait itself was not changed: it stays infallible, so the adapter's error convention is pinned and documented -- a raised exception, a missing method or a non-integer return panics naming the call, `Python LatencyModel.get_insert_latency failed: ...` -- because substituting a value for a latency that cannot be read would silently mis-time every arrival in the run, which is the same refusal the market-impact adapter documents. Only `BacktestEngine.add_venue` moved to the handle converter; the declarative `BacktestVenueConfig` and the sandbox config stay built-ins-only, the asymmetry the fill model already has, and `latency_model_any_to_pyobject` is untouched so the getter still round-trips built-ins. Registration plus stub regeneration put `LatencyModel` in `nautilus_trader.execution` as `FillModel` is there, and the debug extension was rebuilt before any Python test ran. Tests: four in the binding module (an inline Python type's four legs reach the trait, a model missing a leg is refused by type name, a built-in still converts, and a raising model panics carrying the call, the exception type and its message) and one behavioural Python test that injects a subclass through `add_venue`: with a zero-latency model the limit fills, with an hour of insert latency it arrives after the three-minute bar window and fills zero times. That test also surfaced a real property of the harness: the backtest shutdown path settles inflight commands against the final market, so the first draft's marketable limit filled anyway, and the committed order rests below the walked-up market so arrival time decides. One stated limit: `get_base_latency` is exposed on the protocol but the exchange reads only the three order legs, exactly as it does for `StaticLatencyModel`, which folds base into the legs at construction. Verified: `cargo fmt` clean for both crates; `cargo nextest run -p nautilus-execution` 2406 passed and the four new binding tests pass under the `python` feature; `cargo nextest run -p nautilus-backtest -E 'not test(canonical_backtest_workload_matrix)'` 458 passed, 1 skipped with that digest failure pre-existing; clippy clean for both crates including the `python` feature; `cargo check -p nautilus-pyo3 --features extension-module` passed; the Python test passes against the rebuilt extension and `from nautilus_trader.execution import LatencyModel` resolves and subclasses.                                                                                                                                                                                                                  |
| 32       | 2026-10-05 | W5.3 done: `decision_to_execution_delay_s` is the seventeenth execution metric, the elapsed time from the declared decision instant to the **first** fill, so a live run reads how long the market took to answer the agent and not only how far the price moved while it waited. It is declared as seconds, lower-is-better and measured from the decision reference timestamp, and carried on `ExecutionMetrics` beside the decision-price slippage it prices. Two absences stay distinct rather than collapsed: no declared decision instant reports `NotAvailable(NoTimestamp)` and a declared decision with nothing filled reports `NotAvailable(NoObservations)`, while a fill recorded before the decision instant yields a real `0.0` instead of a negative delay. The constant is registered explicitly in `crates/trading/src/python/mod.rs` because that binding is an `m.add` list rather than something derived from the Rust constants, which the first smoke run caught when the import failed, and the metric is therefore proven through the interpreter as well as in Rust: 9 analytics tests and 5 Python tests pass, with a decision at 1.85s and fills at 2.4s and 2.6s reading 0.55 s where measuring to the last fill would read 0.75. The lecture that owns the metric table gained the row, and its claim that the analytics surface is unreachable from Python -- stale since revision 7 (W1.1) -- is corrected together with the recorded test run its two new tests changed from seven to nine. |
