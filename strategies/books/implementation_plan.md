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

| Capability                                                        | Owner                                                                                                                                                                                                                                                                            |
| ----------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Fill models, user-extensible from Python                          | `crates/execution/src/models/fill.rs` (`FillModelAny`, eleven variants); `crates/execution/src/python/fill.rs` (`PyFillModel`); `BacktestEngine.add_venue(fill_model=...)`                                                                                                       |
| Fee models with per-instrument overrides and rebates              | `crates/execution/src/models/fee.rs`; `crates/model/src/fees.rs` (`MakerTakerFeeRates`, `MakerTakerFeeSchedule`)                                                                                                                                                                 |
| Simulated book with queue position                                | `crates/model/src/orderbook/`; `OrderMatchingEngineConfig.queue_position` and `.liquidity_consumption`                                                                                                                                                                           |
| Chronological splits with purge and embargo                       | `python/nautilus_trader/optimization/splits.py` (`SplitContract`, `LeakagePolicy`)                                                                                                                                                                                               |
| Deflated Sharpe with trial provenance                             | `python/nautilus_trader/optimization/significance.py`                                                                                                                                                                                                                            |
| Per-instrument market status enforced in the exchange             | `crates/execution/src/matching_engine/mod.rs` (`process_status`, the submit gate and the matching gate)                                                                                                                                                                          |
| Read-only execution analytics (shortfall, arrival, VWAP slippage) | `crates/trading/src/lib.rs` (`analytics`)                                                                                                                                                                                                                                        |
| Golden-output regression and accounting reconciliation            | `crates/backtest/benches/engine/canonical.rs`; `python/tests/regression/`; `crates/backtest/tests/performance_reconciliation.rs`                                                                                                                                                 |
| Net-of-cost reporting                                             | `crates/analysis/src/analyzer.rs` (the default analyzer's cost row); `crates/analysis/src/statistics/` (gross and net return, and the cost and breakeven rates in basis points of turnover); `crates/execution/src/models/fill.rs` (each fill model's declared `FillAssumption`) |

### 2.2 Partial

| Capability                 | Today                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Missing                                                                                                                                                                                 |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Market impact              | `MarketImpactModel` with a linear variant and a concave square-root variant, the latter carrying its prefactor as an interval with its calibration source and applying the upper bound; the tooling to fit that interval from observed impacts and de-bias a tape-derived one, all constructible from Python, selectable per venue, extensible with a caller's own model through `add_venue`, and scoped to L1 taker fills with the decision recorded on the trait | A caller's own model in the declarative venue config, which mirrors the fill model's limitation                                                                                         |
| Passive fill realism       | Probabilistic fills plus an adverse-selection model whose fill probability falls with the queue ahead and with trade-flow toxicity, both seeded and constructible from Python, with the conditioning reachable through a provided trait method that leaves every existing model unchanged; passive fills can be valued at the imbalance-adjusted microprice, off by default, and each built-in declares its fill assumption so a result can be read against it     | A real-book handoff for a user fill model, which still receives a synthetic book; declarations for the built-ins whose passive decision is neither a probability nor the queue and flow |
| Latency                    | `StaticLatencyModel` over three order legs plus a base                                                                                                                                                                                                                                                                                                                                                                                                             | Market-data latency, competitor rank, a Python extension point                                                                                                                          |
| Data quality               | Bar sequence validation (off by default); Python array monotonicity; a gate for quotes and trades behind a config action, checking the touch for a crossing, values for positivity and `ts_event` for order per instrument, counting each violation by kind and reporting the counts in the backtest summary; adapters log and substitute `ts_init`                                                                                                                | Feed-identity checks against open interest and settlement totals (W6.3); a clock-offset estimate with a drift bound and a counted `ts_init` substitution (W6.4)                         |
| Point-in-time control      | `crates/research` (`FeatureValue.as_of`, `Panel::check`, `AdmittedDecision.available_at`)                                                                                                                                                                                                                                                                                                                                                                          | Any Python binding; the crate has no dependents                                                                                                                                         |
| Execution analytics access | The observer, its term and observation types, the metric set and its vocabulary are callable from Python, and the observer takes the model's `QuoteTick` and `TradeTick` directly; a metric carries its declaration, and an unavailable one is `None` with a reason, never `0.0`                                                                                                                                                                                   | A bus-integrated collector binding, so the caller feeds the observer rather than the engine                                                                                             |
| Tick rules                 | `price_increment` known and precision enforced; alignment checked only when an instrument is redefined or a fill is normalized                                                                                                                                                                                                                                                                                                                                     | Alignment at submit                                                                                                                                                                     |
| Trading state              | A halt denies new submits per instrument; global `TradingState` denies or restricts                                                                                                                                                                                                                                                                                                                                                                                | Cancel-on-halt; a Python setter for `TradingState`                                                                                                                                      |
| Risk limits                | `RiskCap` (metric, scope, limit, window) plus per-order notional, quantity, price and margin checks                                                                                                                                                                                                                                                                                                                                                                | Participation-rate and inventory caps; cross-strategy enforcement; a coordinated de-risking path                                                                                        |
| Simulation seeding         | `BacktestEngineConfig.random_seed` seeds every built-in fill model that declares no seed, all eleven of them, venue-level and per-instrument; a model declaring its own seed keeps it                                                                                                                                                                                                                                                                              | Seeding a foreign (Python) fill model's own draws; seeding the latency and slippage models                                                                                              |
| Synthetic flow             | `crates/backtest/src/synthetic.rs` generates a persistent flow and its induced price path from a target Hurst exponent and an impact exponent bounded at one half, exposed to Python and deterministic under its seed                                                                                                                                                                                                                                              | A bridge from a generated flow into a run's data; the null and robustness harnesses that consume it (W7.2, W7.3), and the scenario that reads a variance ratio back (W3.2)              |
| Interval memory            | `Autocorrelation`, `VarianceRatio` and `RescaledRange` compute the lag autocorrelation, the overlapping-sum variance ratio and the rescaled-range Hurst slope from daily-binned returns, exposed to Python                                                                                                                                                                                                                                                         | A run harness that reads them back from a backtest (W3.2); unit-root tooling; the null and robustness harnesses (W7.2, W7.3)                                                            |

### 2.3 Absent

| Capability                                      | Note                                                                                   |
| ----------------------------------------------- | -------------------------------------------------------------------------------------- |
| Auctions, price bands, circuit breakers         | No model or engine primitive; adapter metadata only                                    |
| Feed-identity validation                        | Nothing checks reported volume against open interest or a settlement total             |
| Cross-venue timestamp reconciliation            | Ordering is insertion order; `VirtualClock` monotonicity is per clock                  |
| Null-model or reference-distribution validation | Only determinism and accounting reconciliation exist                                   |
| Publication or knowledge date                   | `CustomData` carries `ts_init` only; `CorporateAction.effective_ns` is venue-effective |
| News or sentiment data type                     | News reaches the system as adapter metadata only                                       |
| Model-driven strategy example                   | No ML dependency and no serving hook                                                   |
| Global kill switch                              | Closest are the Rust-only `set_trading_state` and per-account backtest liquidation     |
| Impact decay or a transient/permanent split     | A search for `decay` in `crates/execution` and `crates/backtest` returns nothing       |

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
`ExecutionObserver` with its observation types, sixteen `METRIC_*` constants including
`METRIC_IMPLEMENTATION_SHORTFALL_BPS`, `METRIC_ARRIVAL_SLIPPAGE_BPS` and
`METRIC_VWAP_SLIPPAGE_BPS`, and the metric vocabulary that declares what each value is measured
against.
The module is exposed to Python as of revision 7 (W1.1), so a caller can read those numbers back
without re-deriving them; the bus-integrated collector and a report row are not part of that
exposure. The deflated Sharpe lives only
in `optimization/significance.py`, where the trial counts travel on `SharpeSample` rather than as a
call argument.

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
a late bar is dropped. `QuoteTick::new_checked` enforces precision equality and nothing else, and
`TradeTick::new_checked` a positive size. A data-quality gate now covers quotes and trades as of
revision 16 (W6.1, W6.2): `DataEngineConfig.data_quality_action` is `None` by default, and when set
to `Flag` or `Drop` the engine checks every quote and trade for a crossed touch, a non-positive
value and an out-of-order `ts_event` per instrument, counts each violation by kind, and either
forwards or refuses the record, with the counts reported in the backtest summary rather than only
logged. Adapters still log and substitute `ts_init` for an unparseable venue timestamp without
counting it. The `OrderBook` warns on an out-of-order update and clamps its high-water mark. The
Python bulk converters enforce monotonically increasing `ts_init`, and the matching engine skips a
quote or trade older than the book. Nothing validates volume against open interest, and no
cross-venue reconciliation or clock-offset estimate exists.

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
sweep or out-of-sample calibration exists. A synthetic flow generator ships
(`crates/backtest/src/synthetic.rs`, W7.4) and the engine-level seed exists
(`BacktestEngineConfig.random_seed`, W7.1), but nothing yet feeds a generated flow into a run and
nothing reports a distribution over seeds.

**Work.**

- W7.1 (done at revision 4) `BacktestEngineConfig.random_seed` seeds simulation components that
  declare no seed of their own. `SimulatedVenueConfig::seed_fill_models` applies it to the venue's
  fill model and to every per-instrument override before the exchange is built, and a model that
  declares its own seed keeps it, so an unset engine seed leaves the draws unseeded exactly as
  before. All eleven fill models carry the same probabilistic state, `MarketHours` included, so the
  seed reaches each of them; a foreign fill model is unaffected because the two trait methods have
  defaults. What is not seeded from here is a foreign model's own draws and the latency and
  slippage models.
- W7.2 Add a null-model harness that runs the same strategy against a reference process and prints
  a distribution rather than one number.
- W7.3 Add a robustness runner that sweeps seeds and horizon rules, reports distributions, and
  flags parameter sets whose results are degenerate.
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
  into a run: nothing yet feeds a generated flow into a backtest's data.

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

| Revision | Date       | Change                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| -------- | ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1        | 2026-10-05 | First plan, from the ten findings in `README.md` and a symbol-level survey of this repository.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                        |
| 2        | 2026-10-05 | W1.3 done: `PeriodAccounting.fees` and `.slippage` are deleted. T1's Today line and the 2.2 net-of-cost row are rechecked against the tree.                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| 3        | 2026-10-05 | W1.4 done: the deflated Sharpe is a portfolio statistic, so a run's report carries the row. T1's W1.4 text records the premises that did not hold (no gross-return artifact exists, and the metric vocabulary has no count unit) and the contract minimums that make a short run report the row as unavailable.                                                                                                                                                                                                                                                                                                                                                                                                                       |
| 4        | 2026-10-05 | W7.1 done: `BacktestEngineConfig.random_seed` seeds every built-in fill model that declares no seed, venue-level and per-instrument, so a run is reproducible end to end. The engine-level seed row moves from 2.3 absent to 2.2 partial, with the limits stated: a foreign fill model and the latency and slippage models are not seeded from it.                                                                                                                                                                                                                                                                                                                                                                                    |
| 5        | 2026-10-05 | W7.4 done: the synthetic flow generator ships with the Hurst and impact-exponent calibration, the exponent refused above one half, exposed to Python and deterministic under its seed. The synthetic-flow row moves from 2.3 absent to 2.2 partial, and T7's Today line is corrected: the engine seed exists as of revision 4 and the generator ships now, while neither is yet consumed by a run.                                                                                                                                                                                                                                                                                                                                    |
| 6        | 2026-10-05 | W3.1 done: autocorrelation, variance-ratio and rescaled-range statistics exist on the analysis surface and are callable from Python, reading the same daily-binned returns as the other statistics. The interval-memory row moves from 2.3 absent to 2.2 partial and T3's Today line is corrected. P0 is complete; the acceptance that compares a linear and a concave impact run on one flow stays with W3.2, which needs the concave model of W2.1.                                                                                                                                                                                                                                                                                 |
| 7        | 2026-10-05 | W1.1 done: the execution analytics observer, its observations and its metric vocabulary are callable from Python, with the declaration travelling beside every value and unavailability reported as `None` plus a reason. A new 2.2 row records what the access is and what is still missing, a collector binding and the report row of W1.2. T1's Today line is corrected, including a constant count that said fifteen where the module declares sixteen.                                                                                                                                                                                                                                                                           |
| 8        | 2026-10-05 | W1.2 done: the default analyzer carries a cost row of six period statistics, commissions, turnover, gross and net return, and the cost and breakeven rates in basis points of turnover, with the frame reduced once and the four new kernels classified as normalizations. `MetricUnits` gained `BasisPoints`, because a basis-point value had no honest unit. The net-of-cost row's missing column narrows to the fill assumption of W4.3, and the pinned built-in count and objective metric-name set move with the four new built-ins.                                                                                                                                                                                             |
| 9        | 2026-10-05 | W2.1 done: a concave square-root impact model joins the trait and the composition slot, with a prefactor, a reference quantity and the same cap as the linear model, constructible from Python and carrying its own fingerprint scenario over the same synthetic book. The market-impact row's missing column narrows to the interval and the calibration work, and the acceptance's schedule-invariance is read as the model holding no state between fills. The scenario is its own case module because the registry declares one scenario per module.                                                                                                                                                                              |
| 10       | 2026-10-05 | W2.2 done: the concave prefactor is a `PrefactorInterval` with its calibration source rather than a number, and the model applies the interval's upper bound, so a tape-derived prefactor cannot be read as measured or flatter a result. The regression case calibrates over a tape-derived 1.0 to 4.0 interval, so its fingerprint also pins the bound applied. The market-impact row's missing column narrows to the calibration tooling and the Python extension point.                                                                                                                                                                                                                                                           |
| 11       | 2026-10-05 | W2.3 done: the calibration tooling fits the prefactor from observed price impacts and reduces them to the span of their estimates, with two entry points that differ in the source they attribute and an explicit de-bias for a tape-derived fit that refuses a measured one. The market-impact row's missing column narrows to the Python extension point of W2.4.                                                                                                                                                                                                                                                                                                                                                                   |
| 12       | 2026-10-05 | W2.4 done: a caller's own impact model is a Python object with an `impact_increments` method, adapted to the trait and accepted by `BacktestEngine.add_venue`, so a calibration needs no rebuild; an exception or a non-integer return aborts the fill rather than producing one. The market-impact row's missing column narrows to the scope decision of W2.5 and the declarative config's built-ins only, which mirrors the fill model.                                                                                                                                                                                                                                                                                             |
| 13       | 2026-10-05 | W2.5 done: impact stays on liquidity-taking fills that consume an L1 book, recorded on the trait with the reason a deeper book already prices its own depth, and pinned by a matching-engine test that counts the model's consultations on both book types. T2 is complete; the market-impact row's remaining gap is the declarative venue config's built-ins only, which mirrors the fill model.                                                                                                                                                                                                                                                                                                                                     |
| 14       | 2026-10-05 | W4.1 and W4.2 done together, because both edit the matching engine's passive-fill path and cannot be separated into commits that each stand alone: a passive model can condition on queue ahead and trade-flow toxicity through a provided trait method that leaves every existing model unchanged, the adverse-selection model is seeded and constructible from Python, and passive fills can be valued at the imbalance-adjusted microprice behind a flag that defaults off. The passive-fill row's missing column narrows to the fill-assumption row of W4.3 and the synthetic book a user fill model still receives.                                                                                                              |
| 15       | 2026-10-05 | W4.3 done: every built-in whose passive decision is a probability or the queue and flow declares its fill assumption, a model that declares nothing reads as undeclared, and the declaration travels with the handle a venue is configured with so a printed configuration names it. The item asked for a report row, and there is none to put it in: the fork's report is portfolio-level while a fill assumption is a venue property, so the assumption is named where the venue's model is printed and is readable from Python. T4 is complete; the passive-fill row's remaining gap is the synthetic book a user fill model still receives and the declarations the remaining built-ins do not make.                              |
| 16       | 2026-10-05 | W6.1 and W6.2 done together, because the counts are what the gate does with a violation rather than a separable layer: a config action turns on a gate over every quote and trade that checks the touch for a crossing, values for positivity and per-instrument `ts_event` order, counts each violation by kind, and either forwards or refuses the record, with the counts reaching the backtest summary only when something was rejected so a clean run's summary is unchanged. The data-quality row's missing column narrows to the feed-identity checks and the clock-offset estimate. The net-of-cost reporting row moves from partial to in place, since W1.2's cost row and W4.3's fill-assumption declaration are both done. |
