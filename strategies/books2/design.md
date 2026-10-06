# Design: what the general-finance briefs change about measurement in this platform

Date: 2026-10-06. Revision 1.

`README.md` in this directory states twelve findings that recur across the twenty-eight briefs, and
each brief maps its own findings onto parts of this repository. This document turns the six of
those findings that concern measurement, validation and data integrity into design decisions: what
is wrong today, what to build, what was rejected, and what observation would show the decision was
implemented. `implementation_plan.md` beside it turns each decision into numbered work items.

This is not a signal design and it contains no profitability claim. Every decision here either
makes a reported number harder to misread or refuses to report one at all. Several of them can only
make a recorded result look worse, which is their purpose.

## 1. Scope and principles

In scope: the reporting surface, the data-quality gate, the label and provenance types, the fitted
model surfaces (impact, volatility, synthetic flow), and the search bookkeeping in
`python/nautilus_trader/optimization/`.

Out of scope: strategy logic, execution algorithms, adapter connectivity, and anything that sizes a
position. No decision here changes the engine's authority to deny a request rather than resize it.

| #   | Principle                                                                    | Why                                                                                                                                                                                                                                                                                                            |
| --- | ---------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| P1  | A report states the number it optimises, and every number carries its basis. | A matched model swap moved value-weighted Sharpe from 1.39 to 2.08 on identical features (`2108.02283v7`); net-return and arithmetic-mean bookkeeping inflated the S&P 500 Sharpe ratio by nearly 30 percent and its 1960-2020 terminal value by 89 percent (`2405.10920v1`).                                  |
| P2  | Absence is stated, never defaulted.                                          | A metric that cannot be computed prints the missing input, not `0.0`. The execution analytics observer already reports an unavailable metric as absent with a reason rather than as zero, and this design extends that habit to the report.                                                                    |
| P3  | Every correction carries its cost.                                           | Deflating the top two volume quantiles as suspected wash trading destroyed legitimate flow and cut a portfolio Sharpe from 1.41 to 0.96 (`2404.07222v3`).                                                                                                                                                      |
| P4  | A derived field carries its provenance.                                      | A public prediction-market book feed recovered the trade aggressor only about 59 percent of the time, flipping the sign of direction-dependent measures (`2604.24366v2`); a collector-defined label collapsed from AUROC 0.8594 to 0.4642 on a two-week holdout with calibration slope 0.013 (`2607.02823v4`). |
| P5  | A fitted parameter carries an interval and a recovery test.                  | Two parameters of an extended Chiarella agent-based model enter the same term, so both cannot be calibrated (`2208.14207v1`); the kinetic-wealth Pareto tail is a finite-size artifact (`0809.4139v2`); the roughness statistic carries a finite-size bias of +0.323 at H = 0.9 (`2512.02352v3`).              |
| P6  | A detector reports its false positives, not only its hits.                   | A Z-score spoofing screen flagged orders priced $0.01 and missed inserted spoofing (`2308.08683v1`); classifier F1 halved from 80.40 to 46.08 once neutral states entered the label set (`2403.13429v1`); a 99.72 percent accuracy was reported on a 0.2 percent base rate (`2502.15822v1`).                   |
| P7  | A search records its specifications, not only its trial count.               | The multiple-testing bound on the same cross-section rises from 8.5 percent to 41.7 percent once value-weighting and a four-factor adjustment are declared (`2206.15365v10`).                                                                                                                                  |

## 2. What already exists, and therefore constrains the design

The platform already owns more of this than the corpus would predict, so most decisions are
extensions rather than new surfaces.

| Surface                    | Owner today                                                                                                                                                                                                                                                                                                                 | What it already gives us                                                                                                                                                                                                                                 |
| -------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Statistical contract       | `python/nautilus_trader/optimization/significance.py` (`StatisticalContract`, `ReturnCompounding`, `DivisorConvention`, `SharpeFrequency`, `TrialDependence`, `SharpeSample.nominal_trials`/`.counted_trials`)                                                                                                              | Compounding, divisor, frequency and trial dependence are already declared types, and the deflated Sharpe is a registered `PortfolioStatistic`, so a registered run's report carries its trial counts                                                     |
| Cost and return statistics | `crates/analysis/src/statistics/` (`gross_return.rs`, `net_return.rs`, `total_commissions.rs`, `total_turnover.rs`, `cost_basis_points.rs`, `breakeven_cost.rs`), registered by `PortfolioAnalyzer::default` in `crates/analysis/src/analyzer.rs`                                                                           | The cost row exists as statistics; it is produced through period metrics, so the default tearsheet does not render it                                                                                                                                    |
| Metric declaration         | `crates/analysis/src/metric.rs` (`MetricDefinition` with `units`, `tags`, `direction`, `target`, `inputs`, `derived`, and a `title_template` that renders declared parameters)                                                                                                                                              | A statistic can already name its own parameters in its rendered title                                                                                                                                                                                    |
| Data-quality gate          | `crates/data/src/engine/quality.rs` (`DataQualityViolation`, `DataQualityAction`, `DataQualityCounts`, `classify_quote`, `classify_trade`, `validate_open_interest_change`, `validate_settlement_total`), read by `DataEngine::data_quality_counts` in `crates/data/src/engine/mod.rs`                                      | Five violation kinds, per-kind counters, and forward-or-drop behaviour, all opt-in through `DataEngineConfig.data_quality_action`; the feed-identity pair is a standalone check an adapter runs over its own feed rather than a check the engine applies |
| Evidence and calibration   | `crates/research/src/measurement.rs` (`ProducerIdentity`, `AdmittedDecision`, `MetricEstimate`, `MetricComparison`, `confidence_calibration`, `CalibrationReport`, `redundancy`, `signal_quality`, `policy_effect`)                                                                                                         | Calibration by bucket, information coefficient, redundancy and policy effects already report as typed records                                                                                                                                            |
| Labels                     | `crates/research/src/label.rs` (`Label`, `LabelKind`, `LabelError`)                                                                                                                                                                                                                                                         | A label carries its kind                                                                                                                                                                                                                                 |
| Simulation realism         | `crates/backtest/src/config.rs` (`SimulatedVenueConfig` with `cancel_on_halt`, `price_band_bps`, `circuit_breaker`, `price_protection_points`, `liquidation_enabled`, `queue_position`, `liquidity_consumption`), `crates/backtest/src/synthetic.rs` (`SyntheticFlowConfig` with `target_hurst`, `impact_exponent`, `seed`) | Venue rules and a calibrated synthetic flow already exist as first-class config                                                                                                                                                                          |
| Search reporting           | `python/nautilus_trader/optimization/report.py` (`SearchReport`, `rank_results`, `sort_failures`)                                                                                                                                                                                                                           | Trials are ranked, digested and summarised                                                                                                                                                                                                               |

## 3. Decisions

### D1 A run declares its scoring chain, and the report renders a metric per stage

**Evidence.** The loss function moved value-weighted Sharpe from 1.39 to 2.08 with the same features
and models (`2108.02283v7`). A GARCH-GRU hybrid cut S&P 500 volatility MSE from 0.0575 to 0.0159
and Bitcoin MSE from 5.8356 to 0.3818, yet the Value-at-Risk and Expected Shortfall models were not
the hybrids and the hybrid Expected Shortfall backtests failed (`2310.01063v1`). Seven providers
scoring the same earnings calls agreed at a mean pairwise rank correlation of 0.52, with the
provider explaining 33.4 percent of score variance against 34.4 percent for the transcript
(`2609.31013v1`).

**Decision.** A metric declares its stage: forecast (scored against labels), decision (scored
against realized outcomes), or account (scored against the ledger). The stage is a declared
parameter on `MetricDefinition`, so it renders in the row's title, and a run that declares a chain
must render at least one row per declared stage. A run that declares a forecast metric with no
decision or account metric is flagged in the report rather than being read as an improvement.

**Where it lands.** `crates/analysis/src/metric.rs` (the declaration), `crates/analysis/src/statistics/`
(the built-ins declare their stage), `python/nautilus_trader/analysis/tearsheet.py` (the render).

**Rejected.** Inferring the stage from the metric's units. Units do not distinguish a Brier score
from a hit rate, and the point of the decision is that a human declares what they are optimising
before they see the number.

**Consequence.** A report becomes longer, and a run that optimises a proxy without an account metric
is visibly incomplete. That is the intended effect of `2310.01063v1` and `2609.31013v1`.

### D2 The bookkeeping basis is declared per statistic and the arithmetic construction is checked

**Evidence.** Net-return and arithmetic-mean bookkeeping inflated the S&P 500 Sharpe ratio by nearly
30 percent and, compounded, overstated the 1960-2020 terminal value by 89 percent (`2405.10920v1`).
A single erroneous odds row turned published ROIs of 17.29 and 28.82 percent into losses of -7.36
and -6.31 percent while the coefficients and bet sequence reproduced exactly (`2306.01740v4`).

**Decision.** Every return-based statistic declares its compounding and divisor in its
`MetricDefinition.parameters`, so `title_template` renders the basis in the row name and two
statistics computed on different bases can no longer be read as the same row. The platform also
computes, once per run, the terminal equity implied by compounding the arithmetic mean return and
the ratio to the realised terminal equity, and flags the run when that ratio exceeds a declared
tolerance. The declared convention types already exist in `significance.py`; this decision extends
them from the significance path to the analyzer's statistics.

**Where it lands.** `crates/analysis/src/metric.rs` and `crates/analysis/src/statistics/`, with a
new guard statistic beside `returns_avg.rs`; the row renders through
`python/nautilus_trader/analysis/tearsheet.py`.

**Rejected.** Changing the default basis. A platform that silently switched from arithmetic to
geometric would break every existing comparison; the decision is to declare, not to normalise.

**Consequence.** Row names get longer. A report that previously showed `Sharpe Ratio` now shows the
basis, and a mismatch between the arithmetic construction and the ledger is a visible number rather
than an assumption.

### D3 Derived fields are validated against an independent source, and a floor can refuse a metric

**Evidence.** The public order-book feed of a prediction market recovered the trade aggressor only
59.2 percent of the time by volume (panel mean 0.615, 95 percent interval [0.58, 0.65]), flipping
effective half-spread sign on 67 percent of comparable markets, against about 80 percent for
Lee-Ready on equities (`2604.24366v2`). Of 5,140 mini flash crashes in four volatile months of
2008-2010, 67.85 percent were ISO-initiated and 86.98 percent of the trades inside them were
ISO-marked (`1211.6667v1`).

**Decision.** Three changes, all opt-in. First, the data-quality gate infers a trade's aggressor side
with the tick rule and counts an `AggressorSignDisagreement` violation whenever the reported side
differs, so a run reports an agreement rate alongside its violation counts. Second, a declared
`aggressor_agreement_floor` lets a direction-dependent metric (effective spread sign, order-flow
imbalance, Kyle lambda) refuse below the floor, naming the observed rate and the floor in the
report, exactly as the engine denies rather than approximates elsewhere. Third, a fill carries its
cause (book match, halt, band, breaker, liquidation, corporate action) so a crash study can exclude
rule-driven prints instead of averaging them into a market outcome.

**Where it lands.** `crates/data/src/engine/quality.rs` (the inference, the new violation kind, and
`ALL`), `crates/data/src/engine/mod.rs` (`data_quality_counts` already carries the counts into the
backtest summary), `crates/execution/src/matching_engine/mod.rs` and `crates/backtest/src/exchange.rs`
(the fill cause, where the halt, band, breaker and liquidation paths already exist).

**Rejected.** Reconstructing aggressor sign from the feed alone and reporting signed metrics anyway.
That is precisely the failure the 59 percent measurement documents.

**Consequence.** A tape whose reported side is unreliable now yields a refusal rather than a
plausible-looking signed metric, and a crash study has to decide whether rule-driven prints belong
in its sample.

### D4 A fitted model must pass a parameter-recovery test, and an unidentified fit is labelled

**Evidence.** Two parameters of an extended Chiarella model enter the same term, so both cannot be
calibrated and one is fixed instead (`2208.14207v1`); the kinetic wealth-exchange tail has no
stationary distribution for any finite population and its power-law window lasts about 300 steps at
N = 10,000 (`0809.4139v2`); the log-periodic power law yields a distribution over the critical time,
and about one bubble in three ends without a crash (`1107.3171v3`); the roughness estimator's
finite-size bias grows to +0.323 at H = 0.9 (`2512.02352v3`).

**Decision.** A new `ParameterRecoveryReport` in `crates/research/src/measurement.rs`, alongside the
existing `CalibrationReport`. It takes a model that can both draw from itself at a known parameter
vector and refit, draws R seeded datasets, refits each, and reports per-parameter bias, RMSE, the
fraction of declared intervals covering the truth, and one of three verdicts: identified,
weakly identified, unidentified, against tolerances declared with the check. The market-impact
calibration extends this to its own interval by reporting coverage, and a venue whose model fails
recovery is labelled in the run summary rather than silently applied.

**Where it lands.** `crates/research/src/measurement.rs`; consumed by `crates/backtest/src/synthetic.rs`
(the generator estimates its own Hurst and impact exponent back) and by the impact calibration in
`crates/execution/src/models/market_impact.rs`.

**Rejected.** Requiring a model to be identified before it can be configured. The platform does not
know which parameters a caller's model means to fix; the decision is to measure and label, so that a
cost row computed with an unidentified model is readable as such.

**Consequence.** Fitting becomes a two-step procedure with a stated answer, and a model whose
parameters are not recoverable can no longer be described with a point estimate alone.

### D5 A correction reports the outcome it changed, and a detector reports its base rate

**Evidence.** Deflating suspected wash trades cut a Sharpe from 1.41 to 0.96 (`2404.07222v3`). A
conventional Z-score screen flagged $0.01 and $0.11 orders far from the spread and missed inserted
synthetic spoofing (`2308.08683v1`). A 99.72 percent accuracy and 0.987 AUC were reported where
fraud was about 0.2 percent of the set (`2502.15822v1`). Signal quality already reports calibration
and redundancy in this repository, so the missing half is the error side. Calibration is
`confidence_calibration`, the coefficient is `signal_quality`, and redundancy is `redundancy`; all
three report the score side of a model.

**Decision.** Two additions. First, a `CorrectionImpactReport`: a declared outcome metric computed on
the uncorrected and the corrected stream, with the delta printed beside the violation counts, and a
refusal to apply a correction in a report without its delta. Second, a detector-accuracy statistic
that, given decisions and labels, reports the confusion matrix, precision, recall, F1, the positive
base rate and the false-discovery rate, and refuses to print accuracy without the base rate in the
same report.

**Where it lands.** `crates/data/src/engine/quality.rs` (`DataQualityAction` already chooses forward
or drop, so the paired run is a configuration change), `crates/analysis/src/statistics/` (both new
reports), and the report surface in `python/nautilus_trader/analysis/`.

**Rejected.** Reporting only the count of violations, which is what the gate does today. A count
says a rule fired; it does not say whether the rule helped.

**Consequence.** Applying a filter to a data stream becomes a measured decision with a printed cost,
and a detector's headline number always appears next to the base rate that makes it interpretable.

### D6 A search records specifications, and the significance bound is reported at both extremes

**Evidence.** The multiple-testing bound on the cross-sectional factor literature is 8.5 percent on
equal-weighted findings and 41.7 percent after value-weighting and a four-factor adjustment
(`2206.15365v10`). Publication-bias corrections across three independent teams shrink in-sample
returns by only 10 to 15 percent with false discovery under 10 percent (`2209.13623v3`). A
cross-sectional finding therefore depends less on a threshold than on the specification.

**Decision.** A `TrialSpecification` records, per trial, the data window, the universe rule, the
weighting, the adjustment model and the exclusions, is carried into the deflated-Sharpe statistic's
declared parameters, and is summarised by `SearchReport.specification_spread()`. The significance
bound is computed under the most and least favourable declared specification pair and both are
printed, each naming its specification; a headline that reports one bound is refused.

**Where it lands.** `python/nautilus_trader/optimization/significance.py` (the contract types and the
deflated Sharpe already exist), `python/nautilus_trader/optimization/search.py` and `report.py`
(`SearchReport`), and `crates/research/src/measurement.rs` (`InformationCoefficient` for the
cross-sectional side).

**Rejected.** A fixed t-statistic threshold. The corpus's own reading is that the multiple-testing
hurdle is a priced policy, not a law, so the platform reports the policy's cost rather than
enforcing one.

**Consequence.** A search report answers "how many specifications, and which one made this number
look best" - and the deflated Sharpe row can no longer be read without knowing the search's spread.

## 4. Interfaces

All new fields are opt-in with an off default, and every one of them is additive to an existing
type, so a configuration written before this work keeps its behaviour.

| Interface                                                                     | Kind                                               | Home                                                                       |
| ----------------------------------------------------------------------------- | -------------------------------------------------- | -------------------------------------------------------------------------- |
| `MetricStage` (forecast, decision, account) as a declared metric parameter    | Rust enum plus metric parameter                    | `crates/analysis/src/metric.rs`                                            |
| `ReturnBasis` declared per statistic (compounding plus divisor)               | metric parameters reusing the existing conventions | `crates/analysis/src/metric.rs`, `statistics/`                             |
| `ArithmeticCompoundingGuard` (implied terminal equity, ratio, flag)           | statistic                                          | `crates/analysis/src/statistics/`                                          |
| `DataQualityViolation::AggressorSignDisagreement` and the tick-rule inference | violation kind plus helper                         | `crates/data/src/engine/quality.rs`                                        |
| `DataEngineConfig.aggressor_agreement_floor`                                  | config field                                       | `crates/data/src/engine/`                                                  |
| Fill cause (book, halt, band, breaker, liquidation, corporate action)         | fill attribute plus per-run counter                | `crates/execution/src/matching_engine/`, `crates/backtest/src/exchange.rs` |
| `LabelDefinition` (procedure, producer identity, polling interval, horizon)   | struct required on a labelled dataset              | `crates/research/src/label.rs`                                             |
| `ParameterRecoveryReport` and the fit contract                                | report plus tolerance declaration                  | `crates/research/src/measurement.rs`                                       |
| `CorrectionImpactReport` and `DetectorAccuracy`                               | statistics                                         | `crates/analysis/src/statistics/`                                          |
| `TrialSpecification` and `SearchReport.specification_spread()`                | dataclass plus report method                       | `python/nautilus_trader/optimization/`                                     |

Any new Python surface is exposed through the generator, never by hand: `python/generate_stubs.py`
owns every `.pyi` in the tree.

## 5. Non-goals

- No new strategy, signal, indicator or venue adapter.
- No change to the engine's right to deny rather than resize a request, and no automatic resizing.
- No dependency addition. Every decision above is arithmetic on data the platform already holds.
- No relaxation of an existing refusal to make a report look complete.
- No claim that any of these decisions improves a result. D1, D2 and D5 exist because results in the
  corpus were shown to be wrong, not because the correction is profitable.

## 6. Risks and open questions

**Thresholds are conventions.** The aggressor floor, the recovery tolerances and the plumbing guard's
tolerance are chosen values. Each is declared, printed with the run, and stored in the fixture, so a
reader can disagree with the number rather than with a hidden constant.

**Rendering changes the default report.** D1 and D2 add rows to a report that users already read.
The rows are additive and the flags are opt-in, but the tearsheet's section layout is a public
surface and the change has to be made without renaming an existing row.

**A paired correction doubles a run.** D5's delta requires the stream twice. It is opt-in and
restricted to the gate-enabled path.

**The evidence is directional, not calibrated.** Each decision is justified by a measurement in a
different market from ours. The corpus supports the mechanism (bookkeeping inflates a number, a
derived field can be wrong, a detector has false positives) and not a specific threshold, which is
why every threshold here is declared rather than assumed.

**One decision has no local precedent.** Nothing in the repository attributes a fill to a rule
today; the counting in D3 is the smallest change that makes an ISO-like print visible, and it stops
short of modelling an auction, which remains absent. Self-trade prevention exists today only as a
Binance order-submission parameter, not as a rule the matching engine enforces, so it is not
something this design extends.

## 7. Traceability

| Finding in `README.md`               | Decision   | Evidence ids                                                   | Implementation items   |
| ------------------------------------ | ---------- | -------------------------------------------------------------- | ---------------------- |
| 1 Evaluation design decides          | D1, D5     | `2108.02283v7`, `2310.01063v1`, `2609.31013v1`, `2502.15822v1` | I0.4, I3.2             |
| 2 Bookkeeping and definition errors  | D2         | `2405.10920v1`, `2306.01740v4`                                 | I0.1, I0.2, I0.3       |
| 3 Non-identifiable fitted models     | D4         | `0809.4139v2`, `2208.14207v1`, `1107.3171v3`, `2512.02352v3`   | I2.1, I2.2, I2.3, I2.4 |
| 4 Fragile tail measures              | D2, D4     | `2206.02582v2`, `1707.05596v2`                                 | I0.2, I2.3             |
| 5 Market data is not clean           | D3, D5     | `2604.24366v2`, `1211.6667v1`, `2606.31469v1`                  | I1.1, I1.2, I1.4, I3.1 |
| 6 Rules reroute activity             | D3, D5     | `2602.00138v1`, `2607.09514v1`, `2606.31675v1`                 | I1.4, I3.1             |
| 7 Every detector has false positives | D5         | `2308.08683v1`, `2403.13429v1`, `2404.07222v3`                 | I3.1, I3.2             |
| 8 Costs and mechanics dominate       | D2, D5     | `2112.09816v2`, `2007.00486v2`, `2507.21824v1`                 | I0.1, I0.3, I3.1       |
| 9 Concentration on both sides        | D3         | `2606.03153v1`, `2102.10096v2`, `2503.17778v1`                 | I1.3                   |
| 10 Non-monotone stability            | D3, D4     | `1306.3704v1`, `1402.4783v2`, `1403.1637v1`                    | I1.4, I2.4             |
| 11 Persistence is state-dependent    | D6         | `2209.13623v3`, `2206.15365v10`, `1410.6005v1`                 | I3.3, I3.4             |
| 12 Out-of-sample survival            | D1, D4, D6 | `2309.00875v3`, `2607.02823v4`, `2306.01740v4`                 | I0.4, I1.3, I2.1, I3.3 |

Every id above is a paper read in depth in one of the twenty-eight briefs in this directory; four of
them fall outside their category's top thirty and so are listed only under "Papers read in depth"
in that brief rather than in `corpus_map.md`. Every repository path named in this document exists at
commit `a21288e3c2`.
