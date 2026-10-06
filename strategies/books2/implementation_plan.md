# The measurement work the general-finance briefs call for: implementation plan

Date: 2026-10-06. Revision 1.

`design.md` beside this file states six design decisions from the twelve collective findings in
`README.md`. This document turns those decisions into work: what exists today, what to build, in
what order, and what observation shows the work is done.

Every "Today" line names the file and symbol that owns the behaviour, checked against commit
`a21288e3c2`. If a line no longer holds, re-read the item before starting it.

## 1. How to read this plan

- Decision handles (`D1` to `D6`) follow `design.md`; work items carry handles (`I2.3`) so a review
  comment can refer to one.
- "Acceptance" is an observation under a stated scenario, not a task list.
- Every new field is opt-in with an off default, and every change is additive to an existing type,
  so a configuration written before this work keeps its behaviour.
- Nothing here is a profitability claim, and nothing here relaxes the engine's authority to deny
  rather than resize a request. Several items can only make a simulated result look worse; that is
  their purpose.
- Any new Python surface needs the stubs regenerated through `python/generate_stubs.py`. Generated
  `.pyi` files are never hand-edited.
- No production code gains a test-only branch, attribute or interface. Where a check needs synthetic
  inputs, the input is generated in the test, not in the library.

## 2. State of play

### 2.1 Already in place

| Capability                         | Owner                                                                                                                                                                                                                                            |
| ---------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Statistical conventions declared   | `python/nautilus_trader/optimization/significance.py` (`StatisticalContract`, `ReturnCompounding`, `DivisorConvention`, `SharpeFrequency`, `TrialDependence`, `SharpeSample.nominal_trials`/`.counted_trials`)                                   |
| Deflated Sharpe as a report row    | `python/nautilus_trader/optimization/significance.py` (the statistic), registered on a run's report with its trial counts                                                                                                                        |
| Cost and return statistics         | `crates/analysis/src/statistics/gross_return.rs`, `net_return.rs`, `total_commissions.rs`, `total_turnover.rs`, `cost_basis_points.rs`, `breakeven_cost.rs`, all registered by `PortfolioAnalyzer::default` in `crates/analysis/src/analyzer.rs` |
| Metric declaration with parameters | `crates/analysis/src/metric.rs` (`MetricDefinition` with `units`, `tags`, `direction`, `target`, `inputs`, `derived` and a `title_template`)                                                                                                     |
| Data-quality gate with counters    | `crates/data/src/engine/quality.rs` (`DataQualityViolation` with five kinds and `ALL`, `DataQualityAction`, `DataQualityCounts`, `classify_quote`, `classify_trade`, `validate_open_interest_change`, `validate_settlement_total`)               |
| Counts reach the backtest summary  | `crates/data/src/engine/mod.rs` (`DataEngine::data_quality_counts`, `DataEngineConfig.data_quality_action`)                                                                                                                                      |
| Calibration and evidence reporting | `crates/research/src/measurement.rs` (`ProducerIdentity`, `AdmittedDecision`, `MetricEstimate`, `MetricComparison`, `CalibrationReport`, `confidence_calibration`, `signal_quality`, `policy_effect`, `redundancy`)                              |
| Labels with a kind                 | `crates/research/src/label.rs` (`Label`, `LabelKind`, `LabelError`)                                                                                                                                                                              |
| Venue rules as config              | `crates/backtest/src/config.rs` (`SimulatedVenueConfig.cancel_on_halt`, `price_band_bps`, `circuit_breaker`, `price_protection_points`, `liquidation_enabled`); `crates/execution/src/matching_engine/mod.rs`                                    |
| Calibrated synthetic flow          | `crates/backtest/src/synthetic.rs` (`SyntheticFlowConfig.target_hurst`, `impact_exponent`, `seed`)                                                                                                                                               |
| Impact prefactor as an interval    | `crates/execution/src/models/market_impact.rs` (interval with its calibration source, applying the upper bound)                                                                                                                                  |
| Ranked, digested search report     | `python/nautilus_trader/optimization/report.py` (`SearchReport`, `rank_results`, `sort_failures`)                                                                                                                                                |
| Golden-output fixtures             | `python/tests/regression/expected/` (`market_impact_square_root.json`, `optimization_golden.json`, `multi_venue_parity.json`)                                                                                                                    |

### 2.2 Partial

| Capability                | Today                                                                                                                                                                                                                             | Missing                                                                                                                                                                              |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Cost row in a report      | The six cost statistics are registered on the default analyzer, and the tearsheet renders a run-information and account-summary section beside the `stats_pnls`, `stats_returns` and `stats_general` dictionaries                 | The cost figures are produced through period metrics, so a default tearsheet never shows what the run cost                                                                           |
| Bookkeeping basis         | Compounding and divisor are declared types, used by the significance path                                                                                                                                                         | The analyzer's return statistics declare no basis, so two differently computed figures can look like one row                                                                         |
| Calendar and return basis | `ReturnCompounding` and `DivisorConvention` exist as enums                                                                                                                                                                        | Nothing compares the arithmetic construction with realized terminal equity                                                                                                           |
| Metric declaration        | `MetricDefinition` carries units, tags, direction and inputs                                                                                                                                                                      | No stage, so a report cannot tell a forecast score from an account outcome                                                                                                           |
| Trade-side integrity      | The gate checks crossed quotes, non-positive values and `ts_event` order over the records it sees; the feed-identity checks for open interest and settlement totals exist as standalone helpers an adapter runs over its own feed | No aggressor inference, no agreement rate, no floor that refuses a signed metric                                                                                                     |
| Fill attribution          | Halts, bands, breakers, price protection and liquidation all exist, each with a named reason                                                                                                                                      | No cause on a fill, so a rule-driven print is averaged into a market outcome                                                                                                         |
| Label provenance          | `Label` carries a kind; `ProducerIdentity` exists in measurement                                                                                                                                                                  | Nothing records how a label was produced (procedure, polling interval, horizon)                                                                                                      |
| Parameter identification  | The impact model carries a prefactor interval; calibration tooling fits it                                                                                                                                                        | No recovery test, no verdict for an unidentified fit, and no coverage number for a fitted parameter interval (score coverage exists separately, as `ScoreObservation::WithCoverage`) |
| Correction measurement    | The gate can forward or drop a record and count the violation                                                                                                                                                                     | No measure of what a correction did to a declared outcome metric                                                                                                                     |
| Detector error reporting  | The measurement module reports an information coefficient (`signal_quality`), calibration by bucket (`confidence_calibration`) and redundancy (`redundancy`)                                                                      | No confusion matrix, precision, recall, base rate or false-discovery rate                                                                                                            |
| Search provenance         | Trials are ranked, digested and summarised; the deflated Sharpe carries a trial count                                                                                                                                             | No per-trial specification (window, universe, weighting, adjustment, exclusions), so a bound cannot be read                                                                          |

### 2.3 Absent

| Capability                                    | Note                                                                                                                                                                                                                                                        |
| --------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Aggressor-sign agreement as a run figure      | Nothing infers a trade's aggressor side; `DataQualityViolation::ALL` is 5                                                                                                                                                                                   |
| Specification spread in a search report       | No specification is recorded per trial                                                                                                                                                                                                                      |
| Weighting sensitivity of a significance bound | The bound is one number per run                                                                                                                                                                                                                             |
| Detector base rate in a report                | No statistic reports a base rate beside an accuracy                                                                                                                                                                                                         |
| Fill cause counter                            | No per-run count of fills by cause                                                                                                                                                                                                                          |
| Auction modelling                             | No auction matching model exists; only a market-status value names an auction. Self-trade prevention exists as a Binance order-submission parameter (`BinanceSelfTradePreventionMode`), not as a rule the matching engine enforces, so D3 does not touch it |

## 3. Sequencing

| Phase                       | Items        | Gate   | Why here                                                                                      |
| --------------------------- | ------------ | ------ | --------------------------------------------------------------------------------------------- |
| P0 Report honesty           | I0.1 to I0.4 | None   | Every later phase is read through the report, so the report must name its basis and its stage |
| P1 Data and label integrity | I1.1 to I1.4 | P0     | A study on an unvalidated tape is a study of the wrong tape                                   |
| P2 Fitted models            | I2.1 to I2.4 | P0     | Independent of P1; needs only the seeded generator and the reporting it lands in              |
| P3 Detectors and search     | I3.1 to I3.4 | P1, P2 | The correction deltas are measured on the stream the gate validates                           |
| P4 Documentation and parity | I4.1 to I4.3 | P0     | The vocabulary must exist before it is documented                                             |

## 4. Work items

### P0 Report honesty

#### I0.1 Render the cost row in the default report

**Target.** `python/nautilus_trader/analysis/tearsheet.py` (`_create_stats_table`),
`python/nautilus_trader/analysis/statistic.py`, `crates/analysis/src/analyzer.rs`.

**Today.** `PortfolioAnalyzer::default` registers `GrossReturn`, `NetReturn`, `TotalCommissions`,
`TotalTurnover`, `CostBasisPoints` and `BreakevenCost`, but they are produced through period metrics
and none of the six reaches the rendered statistics: the tearsheet prints a run-information and
account-summary section beside the PnL, returns and general dictionaries, so a default tearsheet
shows the result without showing what it cost. Each of the six returns `None` from
`calculate_from_returns` and is computed only from period metrics.

**Work.** Give the analyzer's period metrics a route into the rendered statistics, and render the
six figures in the returns table with their units (per unit of turnover for the cost pair, per unit
of starting equity for the returns). A statistic whose input is absent renders the missing input by
name rather than `0.0`.

**Acceptance.** A backtest on a zero-commission venue prints cost 0.00 bps and identical gross and
net returns; the same run with a maker-taker fee prints net below gross, and the gap equals the
printed cost in basis points.

#### I0.2 Declare the bookkeeping basis on every return statistic

**Target.** `crates/analysis/src/metric.rs` (`MetricDefinition.parameters`, `title_template`),
`crates/analysis/src/statistics/` (each return-based statistic).

**Today.** The conventions exist as `ReturnCompounding` and `DivisorConvention` in the significance
path; the analyzer's statistics carry no basis and render as `Sharpe Ratio`, `Returns Volatility` and
so on.

**Work.** Every return-based statistic declares its compounding and divisor as metric parameters, so
the rendered row names the basis, reusing the existing convention vocabulary rather than a second
one.

**Acceptance.** A report renders a basis in the title of every return-based row, and a test asserts
that no statistic built from a returns series renders without one.

#### I0.3 Check the arithmetic construction against realised terminal equity

**Target.** A new statistic beside `crates/analysis/src/statistics/returns_avg.rs`, rendered through
the tearsheet.

**Today.** Nothing compares the terminal equity implied by compounding the arithmetic mean return
with the realised terminal equity.

**Work.** Report the implied terminal equity, the realised one, their ratio, and a flag when the
ratio exceeds a declared tolerance. The tolerance is a declared metric parameter, so it prints with
the row.

**Acceptance.** On a synthetic series constructed so that compounding the arithmetic mean overstates
the terminal value, the row reports the ratio the construction produces and the run is flagged; on a
single-period run the ratio is 1.0 and the run is not flagged.

#### I0.4 Declare the scoring chain and require an outcome metric

**Target.** `crates/analysis/src/metric.rs` (a `MetricStage` parameter), `crates/analysis/src/statistics/`
(the built-ins declare their stage), `python/nautilus_trader/analysis/tearsheet.py` (a per-stage
section or group).

**Today.** `MetricDefinition` carries units, tags and direction, and the tearsheet groups rows into
sections, but nothing distinguishes a forecast score from a decision metric from an account outcome.

**Work.** Add the stage as a declared parameter, have each built-in statistic declare one, and make a
run that declares a chain render at least one row per declared stage. A declared forecast metric with
no decision or account metric is flagged in the report.

**Acceptance.** A run declaring a forecast metric and an account metric prints both, each in its
stage; the same run with the account metric removed prints the flag naming the missing stage.

### P1 Data and label integrity

#### I1.1 Infer the aggressor side and count its disagreement

**Target.** `crates/data/src/engine/quality.rs` (`classify_trade`, `DataQualityViolation`,
`DataQualityViolation::ALL`, `DataQualityCounts`).

**Today.** The gate classifies a trade for non-positive values and for `ts_event` order only; it
never reads the reported aggressor side, and the violation list has exactly five kinds with a stable
`ALL` array used for the count line.

**Work.** Add the tick-rule inference beside `classify_trade`, a `AggressorSignDisagreement`
violation kind, its `Display` token, and extend `ALL`. Record the compared total beside the counts so
an agreement rate is computable without re-reading the tape.

**Acceptance.** On a tape whose reported side is replaced by a random sign the run reports an
agreement rate near 0.5 and the disagreement count is non-zero; on a tape whose reported side matches
the inference the rate is 1.0 and the count is zero. No existing count line changes for a tape that
passes the gate.

#### I1.2 A declared floor refuses a direction-dependent metric

**Target.** `crates/data/src/engine/` (the config field and the rate it carries into a run),
`crates/analysis/src/statistics/` (the signed metrics), `python/nautilus_trader/analysis/`.

**Today.** No floor exists; a signed metric is computed from whatever the feed reports.

**Work.** Add `DataEngineConfig.aggressor_agreement_floor`, carry the observed rate into the run's
analysis, and make each direction-dependent metric refuse below the floor with the observed rate and
the floor named in the report, matching the way an unavailable metric is reported elsewhere in the
platform rather than printing a number.

**Acceptance.** At a floor of 0.9 against a tape whose agreement rate is about 0.59, the report
prints the rate and refuses the signed metrics naming the floor; at a floor of 0.5 on the same tape
it refuses nothing.

#### I1.3 A label carries the definition that produced it

**Target.** `crates/research/src/label.rs` (`Label`, `LabelKind`), `crates/research/src/measurement.rs`
(`ProducerIdentity`, `Disposition`, `signal_quality`).

**Today.** A label carries a kind, and measurement reports outcomes by disposition and producer, but
nothing records how the label was produced.

**Work.** Add a `LabelDefinition` (procedure name, producer identity, polling interval, horizon),
required for a labelled dataset, and report the count of records carrying no definition as a
first-class number. A model trained on undefined labels is reported as such rather than scored.

**Acceptance.** A dataset built without a definition reports 100 percent undefined; a dataset built
with a 60-second poll and a fixed horizon reports the interval and the horizon in the measurement
report, and the undefined count is zero.

#### I1.4 Count a fill by its cause

**Target.** `crates/execution/src/matching_engine/mod.rs` (the halt, band, breaker and protection
paths and the `MARKET_HALTED` reason), `crates/backtest/src/exchange.rs` (liquidation and lifecycle
processing), `crates/analysis/` (the per-run counter and its row).

**Today.** Halts cancel the book, bands reject a submission, the breaker opens a halt window, price
protection caps an aggressive fill and margin breach liquidates, each with a named reason; nothing
counts a fill by which of them caused it.

**Work.** Record a cause on a fill and count the per-run totals by cause, reported as one row with
one column per cause.

**Acceptance.** A run in which the breaker trips reports the breaker's cancels and its halt-window
fills separately from ordinary book matches and reports zero for the causes that did not occur; a run
without any venue rule reports all fills under the book cause.

### P2 Fitted models

#### I2.1 Parameter recovery as a report

**Target.** `crates/research/src/measurement.rs`, beside `confidence_calibration` and
`CalibrationReport`.

**Today.** The module reports calibration by bucket, information coefficient, redundancy and policy
effects; nothing measures whether a model's own parameters can be recovered.

**Work.** Add a `ParameterRecoveryReport` over a fit contract: draw R seeded datasets from the model
at a known parameter vector, refit each, and report per-parameter bias, RMSE, interval coverage and a
verdict of identified, weakly identified or unidentified against tolerances declared with the check.

**Acceptance.** A fixture whose two parameters enter one term reports unidentified with intervals far
wider than the truth's separation; a fixture with two separable parameters reports identified. The
verdict is deterministic at a seed.

#### I2.2 The synthetic flow estimates its own parameters back

**Target.** `crates/backtest/src/synthetic.rs` (`SyntheticFlowConfig`, `SyntheticFlow`).

**Today.** The generator builds a persistent flow and its induced price path from a target Hurst
exponent and an impact exponent, deterministically under a seed; it never estimates either back.

**Work.** Expose an estimator alongside the generator so the recovery report can be run on the
generator itself, and state the length at which the estimate is worth reading.

**Acceptance.** At a target Hurst exponent of 0.6 and a long generated series, the recovered estimate
lies inside the declared band, the report prints the remaining bias, and at a short series the report
says the estimate is not yet readable rather than printing a number.

#### I2.3 Report the coverage of the impact interval

**Target.** `crates/execution/src/models/market_impact.rs`, `crates/backtest/src/synthetic.rs`.

**Today.** The concave model carries its prefactor as an interval with its calibration source and
applies the upper bound; the tooling fits the interval from observed impacts; nothing reports whether
the interval is calibrated.

**Work.** Fit the interval on paths generated at a known prefactor and report the fraction of
repetitions in which the interval contains it, beside the interval and its source.

**Acceptance.** A deliberately over-tight interval reports coverage below its nominal level, and the
nominal level and the repetition count print with the figure.

#### I2.4 Label an unidentified model instead of applying it silently

**Target.** `crates/backtest/src/config.rs` (`SimulatedVenueConfig.market_impact_model`), the run
summary in `crates/backtest/src/`, `crates/analysis/` (the cost rows).

**Today.** A configured model is applied, and only its calibration source and interval are recorded.

**Work.** When a venue's model fails recovery, report it in the run summary as unidentified with the
failing parameter named, and mark the cost rows that its fills contribute to. The model still runs:
the platform labels the number rather than dropping it.

**Acceptance.** A run with an unidentified impact model prints the label beside the affected cost
rows and names the parameter; a run with an identified model prints no label.

### P3 Detectors and search

#### I3.1 A correction reports the outcome it changed

**Target.** `crates/data/src/engine/quality.rs` (`DataQualityAction`), a new statistic in
`crates/analysis/src/statistics/`, the report surface in `python/nautilus_trader/analysis/`.

**Today.** The gate forwards or drops a record and counts the violation; nothing measures what the
correction did to a result.

**Work.** Add a `CorrectionImpactReport` that computes a declared outcome metric on the uncorrected
and the corrected stream and prints both with the delta, beside the violation counts. A report that
applies a correction without its delta is refused.

**Acceptance.** Deflating a synthetic tape contaminated with wash-like prints moves the declared
metric in the direction the corpus reports (a lower Sharpe), and the report prints the uncorrected
value, the corrected value and the delta; a run that enables the correction without declaring an
outcome metric is refused with that reason.

#### I3.2 Detector accuracy always prints the base rate

**Target.** A new statistic in `crates/analysis/src/statistics/`, with a Python route through
`python/nautilus_trader/analysis/statistic.py`.

**Today.** No confusion matrix, precision, recall, base rate or false-discovery rate is reported
anywhere.

**Work.** Given decisions and labels, report the confusion matrix, precision, recall, F1, the
positive base rate and the false-discovery rate. Accuracy is never printed without the base rate in
the same report.

**Acceptance.** A detector that marks every record positive reports accuracy equal to the base rate,
precision equal to the base rate and recall 1.0; a detector evaluated on a 0.2 percent base rate
prints that base rate beside its accuracy, and a test asserts the two rows cannot be separated.

#### I3.3 Record the specification of every trial

**Target.** `python/nautilus_trader/optimization/search.py` and `significance.py` (a
`TrialSpecification`), `python/nautilus_trader/optimization/report.py` (`SearchReport`).

**Today.** A trial is ranked, digested and summarised, and the deflated Sharpe statistic carries a
nominal and a counted trial count, but no specification is recorded.

**Work.** Record the window, universe rule, weighting, adjustment model and exclusions per trial,
carry the specification into the deflated-Sharpe statistic's parameters so the row names its
provenance, and add `SearchReport.specification_spread()`.

**Acceptance.** A search over three weightings and two windows reports a spread of six, and each
trial's specification is recoverable from the report rather than from the caller's memory.

#### I3.4 Report the significance bound at both specification extremes

**Target.** `python/nautilus_trader/optimization/significance.py`,
`crates/research/src/measurement.rs` (`InformationCoefficient`).

**Today.** The multiple-testing bound is one number per run.

**Work.** Compute the bound under the most and least favourable declared specification pair and
print both, each naming its specification. A run reporting only one bound is refused.

**Acceptance.** The report prints two bounds with their specifications, and a run that declares a
single weighting prints that the bound could not be checked rather than printing one figure.

### P4 Documentation and parity

#### I4.1 Document the new vocabulary

**Target.** `docs/concepts/reports.md`, `docs/concepts/optimization.md`,
`docs/concepts/performance_periods.md`.

**Today.** Those pages describe the tearsheet, the search and the period metrics without a basis,
stage, floor, recovery verdict, correction delta or specification spread.

**Work.** Add each new row and field with the reason it exists, and the command that reproduces it.

**Acceptance.** `bash .pre-commit-hooks/check_docs_conventions.sh` passes, and each new row appears
on the page that owns its surface.

#### I4.2 Regenerate the stubs through the generator

**Target.** `python/generate_stubs.py`, the `.pyi` files it owns.

**Today.** Any new Python surface needs regenerated stubs.

**Work.** Regenerate after every Python-facing addition; never hand-edit a `.pyi`, and revert the
unrelated churn the generator produces in files outside the change.

**Acceptance.** A fresh regeneration leaves the working tree unchanged for the touched modules.

#### I4.3 Record a regression fixture for the new rows

**Target.** `python/tests/regression/expected/`, following `market_impact_square_root.json` and
`optimization_golden.json`.

**Today.** Golden fixtures exist for the impact model, the optimizer and multi-venue parity.

**Work.** Record one canonical run's new rows: the cost pair, the basis, the construction ratio, the
aggressor rate, the recovery verdict and the specification spread.

**Acceptance.** Changing one statistic's declared basis, or the arithmetic guard's tolerance, makes
the fixture fail; an unrelated change to a run's seed does not.

## 5. Verification matrix

| Item | Observation                                                          | Where the proof lives                                |
| ---- | -------------------------------------------------------------------- | ---------------------------------------------------- |
| I0.1 | Cost row renders, zero-fee run shows gross equal to net              | `python/tests/unit/analysis/test_tearsheet.py`       |
| I0.2 | Every return row names its basis                                     | `python/tests/unit/analysis/test_metric_identity.py` |
| I0.3 | Construction ratio flags an inflated series                          | `crates/analysis` statistic tests                    |
| I0.4 | A declared chain renders a row per stage, a missing stage is flagged | `python/tests/unit/analysis/test_analysis.py`        |
| I1.1 | Agreement rate and disagreement count on a scrambled tape            | `crates/data` gate tests                             |
| I1.2 | Signed metrics refuse below the floor                                | `crates/analysis` statistics plus a gate fixture     |
| I1.3 | Undefined-label count and the recorded definition                    | `crates/research/tests/reproducibility.rs`           |
| I1.4 | Fills counted by cause under a breaker trip                          | `crates/backtest/tests/integration/`                 |
| I2.1 | Identified and unidentified fixtures give their verdicts             | `crates/research` measurement tests                  |
| I2.2 | Hurst recovered within the band, or refused as unreadable            | `crates/backtest` synthetic tests                    |
| I2.3 | Interval coverage below nominal for a tight interval                 | `crates/execution` impact tests                      |
| I2.4 | Unidentified model labelled in the run summary                       | backtest run summary fixture                         |
| I3.1 | Paired correction prints both values and the delta                   | `python/tests/regression/`                           |
| I3.2 | Base rate printed beside accuracy                                    | `python/tests/unit/analysis/`                        |
| I3.3 | Specification spread of six and per-trial provenance                 | `python/tests/unit/optimization/`                    |
| I3.4 | Two bounds, each naming its specification                            | `python/tests/unit/optimization/`                    |
| I4.1 | Docs conventions pass with the new vocabulary                        | `.pre-commit-hooks/check_docs_conventions.sh`        |
| I4.2 | Regeneration is idempotent                                           | a fresh `generate_stubs.py` run                      |
| I4.3 | Fixture fails on a basis change, passes otherwise                    | `python/tests/regression/`                           |

## 6. Mechanics

- Run the smallest relevant test while developing; before a pull request, run `make format`,
  `make pre-commit` and the tests for the touched crates locally, as the repository's instructions
  require.
- A Rust change touches `crates/analysis`, `crates/data`, `crates/research`, `crates/execution` or
  `crates/backtest`; a Python change touches `python/nautilus_trader/analysis` or
  `python/nautilus_trader/optimization`. Keep each change inside one owner.
- Adding a `DataQualityViolation` variant requires updating `ALL` and the `Display` token in the same
  change, because the count line is rendered from `ALL`.
- Every acceptance line above is an observation under a stated scenario. Where the observation needs
  a contaminated or scrambled input, the input is generated inside the test.

## 7. Definition of done

A phase is done when every item in it has its acceptance observation recorded, the fixtures it
touched are updated deliberately rather than regenerated blindly, and the documentation for any new
row exists on the page that owns its surface. The plan is done when a run of this platform can state,
without a reader having to ask: what was optimised, on what basis, over which specification, on data
whose provenance was validated, with parameters whose identifiability was measured, and with the cost
of every correction printed beside the correction.
