# VectorBT lesson review: implementation record

Companion to [`vectorbt_lessons_design.md`](vectorbt_lessons_design.md) and to
[`vnpy_lessons_implementation.md`](vnpy_lessons_implementation.md). Section references of the form
`design N` point at the design document, and decision identifiers (`D1` to `D11`) are the revised
numbering introduced in revision 2 of this record.

## 1. Status and scope

### 1.1 Seven decisions, the identity contracts and the parity protocol are implemented, the rest are not

This document records a probe, a comparison and a set of specified decisions. The probe changed no
production code: the authorising instruction permitted changes only for defects, no defect was found
in this repository (section 5.1), so the defect path was not exercised, and the probe's verification
was source inspection rather than execution (section 3).

The owner then authorised implementation work in the order of `design 12`. **D2, the split contract
and its leakage exclusion relation, is implemented** (`python/nautilus_trader/optimization/splits.py`,
consumed by the walk-forward stages), and section 4.2 records what was built and how it was verified.
**D1, metric identity, metadata, status and reason codes, is implemented** (`crates/analysis/src/metric.rs`,
required by the `PortfolioStatistic` trait and declared by all 34 built-in statistics), and section
4.1 records what was built, its deviations and its verification. **D3, the ambiguity policy identity,
is implemented** (`python/nautilus_trader/optimization/assumptions.py`, declared in a configuration
file and recorded in the emitted result document), and section 4.3 records what was built and its
decisions. **The identity contracts for a study, a trial, a dataset, a universe and a result are
implemented** (`python/nautilus_trader/optimization/identity.py`), and section 4.11 records what was
built, what was pruned and why. **D4, multiple-testing-aware reporting, is implemented**
(`python/nautilus_trader/optimization/significance.py`): the statistical contract is declared whole,
with an identity and a digest, and the deflated Sharpe ratio is computed under it from a sample that
must declare its annualisation, its kurtosis convention and its trial dependence; section 4.4 records
what was built, the ten decisions taken and the verification. **D5, the classified numerical-stability obligation, is implemented**
(`crates/analysis/src/kernel.rs`): every built-in statistic and the three shared kernels are assigned
a class whose obligation is stated, the classification is checked against the registry in both
directions, and the obligations are exercised per class, which found and fixed four missing-value
defects and one summation that could not be trusted over a long series; section 4.5 records what was
built, the defects and the verification. **D7, the secondary implementation parity protocol, exists
as a document** (`docs/developer_guide/parity_protocol.md`, linked from `crates/pyo3/README.md`),
with its claims re-derived from this repository rather than restated; section 4.7 records what it
states and what was corrected. **D8, domain-scoped capability results, is implemented**
(`crates/core/src/capability.rs` with its Python binding, plus one producer in each of the order,
analysis, data and research domains): one shape, four closed code sets, and a source test that no
caller branches on a detail; section 4.8 records what was built. **D9 remains deferred, and its trigger is now measured rather than assumed**
(section 4.9): a ledger at the runner boundary shows the richest shipped composition executing nine
runs with no repeat, while the control composition reports exactly one, so the measurement is
sensitive and the condition is unmet. **D6, the first tranche of the label policies, is
implemented** (`python/nautilus_trader/optimization/labels.py`): a label definition carries the
outcome, the forward window, the wait, the missing-data policy and the alignment convention; the
fixed-horizon forward return, the forward aggregates of the window's per-bar returns and a first-hit
label over two independent thresholds are computed; the forward reach of every produced label is
measured from the series so a leakage policy that does not cover it is refused with the shortfall;
and the target path is quarantined from the live packages and from the feature inputs. Section 4.6
records what was built and what is deliberately not built. Every other item remains **Not implemented** in section 6,
and its mechanism in section 4 remains a specification rather than a description of code.

All three items were verified by execution, not inspection. D2: `pytest tests/unit/optimization` (54
tests, including 31 for the contract), `pytest tests/integration/test_optimization.py` (9 tests,
including the walk-forward scenario that pins the pre-migration windows), and the declared regression
scenario `optimization_golden` (1 test) all pass; a throwaway script exercised the configuration path
end to end, from a JSON document with a `stage.leakage` block through the emitted window records, and
was deleted afterwards. D1: `cargo nextest run -p nautilus-analysis --lib --features python` runs 308
tests, all passing, including seven new tests for the contract, and `cargo clippy` with `-D warnings`
is clean on the crate; `pytest` over the analysis, portfolio, optimization and integration suites
runs 365 tests, all passing. D3: `pytest tests/unit/optimization` now runs 61 tests, all passing,
including seven new ones, and the declared regression scenarios pass with their expectations
unmodified. The identity contracts: `pytest tests/unit/optimization` runs 68 tests and `pytest
tests/integration/test_optimization.py` runs 10, all passing, including seven unit tests for the
contracts and one integration test that proves two real sweeps of one study reproduce the result
digest. D4: `pytest tests/unit/optimization/test_significance.py` runs 21 tests, all passing, and the
formula is compared against a recomputation that shares no step with it and agrees to `1e-12`. D5:
`cargo nextest run -p nautilus-analysis --lib --features python` runs 319 tests, all passing,
including eleven for the classification and its obligations, and `cargo clippy` with `-D warnings` is
clean on the crate. D6: `pytest tests/unit/optimization/test_labels.py` runs 19 tests, all passing,
including the hand-computed asymmetric first-hit case, the cumulative-barrier case, both alignment
pairings, the zero-wait case, the two missing-data policies, the leakage refusal with its shortfall
and the two quarantine tests. D8: `cargo nextest run` over the core, model, analysis and persistence
crates passes, including 13 tests for the capability shape and the four producers, `cargo clippy`
with `-D warnings` is clean on all four, and on the Python side 9 capability tests pass, including
the source scan; the extension was rebuilt and the stubs regenerated for the new binding. D9:
`pytest tests/integration/test_optimization.py` runs 11 tests, all passing, including the trigger
measurement and its control. The statement that no test was run applies to the probe alone.

### 1.2 Revision history

| Version | Change                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| ------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1       | Initial record from the vectorbt probe: the licence finding, the capability inventory, eleven decisions and this implementation record                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| 2       | Applied design-review feedback. Decisions renumbered and classified by layer; the deflated Sharpe ratio rescoped from the requirement to the first statistic behind a trial-provenance requirement; leakage became a policy whose values are optional and whose concept is mandatory; the ambiguity resolution became a versioned policy recorded by identity; metrics gained a direction; labels split into tranches; caching deferred with an identity model; the provider adapter relocated; study identity added; the licence rule restated as an engineering boundary with a provenance chain; the dependency graph replaced and the phases restated as work order                                                                                                                                                                                                                                                                                                                                                                                                                                                                            |
| 3       | Applied the second review. Identity promoted to a first-class contract covering study, trial, dataset, universe and result, with the trial level made explicit, and moved into `design 8`; the design's dependency section split into a contract graph and a work order so the graph no longer contradicts its own explanation; the leakage policy became an exclusion relation with purge before, purge after, embargo after and a label overlap rule; the ambiguity default became an owner decision separate from the policy contract, and ambiguity was separated from execution simulation; the statistical contract for the multiple-testing correction was specified in full; the stability obligation was classified by kernel type; the metric status vocabulary gained `invalid` and the direction vocabulary became action-oriented; capability results became domain-scoped; the label definition gained an alignment convention; a no-decision-authority invariant was added; dataset and trial identity were added to the open questions                                                                                             |
| 4       | The owner authorised implementation in the work order of `design 12` and D2 was implemented: a `SplitContract` with named sets, absolute, fractional or omitted lengths, a layout direction, a minimum length and an evenly spaced split count; a `LeakagePolicy` with purge before, purge after, an embargo gap, a label overlap rule that folds a label horizon into the purge, and a justified zero; and the walk-forward stages migrated onto the contract with their windows unchanged. The evidence is in section 4.2 and the verification in section 6                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| 5       | D1 was implemented: a closed metric vocabulary (units, tags, action-oriented directions with an optional target, declared inputs, four statuses and seven reason codes), a `MetricDefinition` whose title renders from named parameters while the display string is preserved, a required `definition()` on the `PortfolioStatistic` trait and a declaration for all 34 built-in statistics, and report methods that keep every requested metric visible with a status and a reason instead of dropping it. The vocabulary is proposed by the implementation as the answer to `design 14` question 6, and the two deviations from the specification are recorded in section 4.1                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| 6       | D3 was implemented: `BarAmbiguityPolicy` names the four bar-derived assumptions (bar execution, the intrabar path, trigger precedence, the trigger fill rule and gap handling) with a policy id, a version and a digest; the declaration lives in a configuration file under `assumptions`, is refused as a configuration error when ambiguous, and the resolved policy is recorded in the emitted result document. No Rust type or behaviour changed, the default is named as the existing behaviour rather than changed, and the assumption-to-assertion map is documented and cited in section 4.3                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| 7       | The identity contracts were implemented: `StudyIdentity`, `TrialIdentity`, `TrialProvenance`, `DatasetIdentity`, `UniverseIdentity`, `ComputationIdentity` and `ResearchResult` with digests that compose from the digests that already exist, two closed vocabularies for the selection rule and the execution status, and bridges from an objective's terms, a split contract and an optimizer run. The field lists were pruned rather than filled in - the counts moved to a provenance record so a transient failure cannot move a study's identity, and the feature and label definitions were dropped because nothing in the repository computes either - and the contracts are not yet wired into the pipeline, which is recorded with its reason in section 4.11                                                                                                                                                                                                                                                                                                                                                                           |
| 8       | D4 was implemented: `python/nautilus_trader/optimization/significance.py` declares the whole statistical contract with an identity and a digest (the return definition and compounding, the explicit risk-free treatment, the minimum observation and trial counts, the Sharpe frequency and divisor, the trial-variance estimator, the skew and non-excess-kurtosis estimators, the prohibition on annualised inputs, the missing-return rule, the duplicate-trial rule, the failed-trial rule and the required trial-independence declaration), computes the deflated Sharpe ratio under it with `unavailable` and `invalid` distinguished in D1's vocabulary, and records the study, its trials and their provenance in a report that digests stably. The annualised case and the excess-kurtosis case are refused at the boundary, a dependent study must declare its effective trial count, and the two thresholds are declared parameters with defaults the owner may replace. Section 4.4 records the decisions and the verification, and the correction is deliberately not wired into the emitted document, for the reason recorded there |
| 9       | D5 was implemented: `crates/analysis/src/kernel.rs` declares the seven kernel classes with the cases each obliges, classifies every built-in statistic and the three shared kernels by their dominant numerical operation, and is checked against the registry in both directions so a new statistic cannot enter unclassified. The class obligations are exercised by eleven tests, including a missing-value rule (a kernel must propagate or completely exclude a missing observation, never keep it in a count or a rank), a divisor and minimum-observation case, and a dispersion comparison against a compensated two-pass computation. That comparison found `calculate_std` wrong by 0.71 relative on a hundred thousand values with a large offset; it now sums with Neumaier compensation and agrees to 1.6e-16. The rule also found and fixed four missing-value defects (`max_drawdown`, `value_at_risk`, `expected_shortfall` and `win_rate`). Section 4.5 records the decisions, the fixes and the verification                                                                                                                     |
| 10      | D6 was implemented, first tranche only: `python/nautilus_trader/optimization/labels.py` declares a `LabelDefinition` carrying the outcome, the forward window in observations, the wait, the missing-data policy and the alignment convention as part of the definition; computes the fixed-horizon forward return, the forward aggregates of the window's per-bar returns with the sample divisor of the D5 dispersion kernel, and a first-hit label over two independent thresholds measured from the entry; and measures the forward reach of every produced label from the series rather than deriving it, so a `LeakagePolicy` shorter than the reach is refused with the shortfall in nanoseconds. The target path is quarantined by a source test over the live packages and by refusing a label series offered as market data. The second tranche (extrema and trend-state labels) is not built, because the dataset contract of the earlier review is not implemented and the leakage intervals remain a study decision                                                                                                                   |
| 11      | D7 was implemented as a document: `docs/developer_guide/parity_protocol.md` states the seven rules (one reference implementation, mirroring argument order and return shape, refusal over degradation, benchmarks after parity, the import direction, the build-time version agreement and the status as policy rather than a research prerequisite) with a nine-step checklist. It is linked from `crates/pyo3/README.md`, which owns the boundary, and listed in the developer guide's contents. Its version-agreement claim was re-derived from `crates/core/build.rs` rather than assumed, and the reference-generation claim from `scripts/benchmark-backtest-versions.py`, so the document records what this repository does rather than what the source library does                                                                                                                                                                                                                                                                                                                                                                        |
| 12      | D8 was implemented: `Capability` in `crates/core/src/capability.rs` is the shared shape (availability, a canonical code, a human-readable detail and the requirements not met), exposed to Python as `nautilus_trader.core.Capability` with the code validated where it is built from a string. The code sets are per domain: order reuses `OrderDeniedCode`, analysis reuses `MetricReason`, data declares coverage codes and research declares `ResearchCapabilityCode` with three probes. A source test rejects any comparison, membership test or prefix search on a detail, in either language                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| 13      | D9's trigger was measured and remains unmet: a ledger at the runner boundary recorded every executed run of the richest shipped composition (a walk-forward study plus a search), which executed nine runs with no repeat, while the control composition, validating an experiment the search already ran on the same window, reported exactly one. The identity half of the cache key exists in `ExperimentStore`, which writes and can load by digest, and the policy half does not: `Optimizer.optimize` never consults a store to skip a run. The item stays deferred, and the test would report the day that changes                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |

### 1.3 Reference maps

Two renumberings affect how earlier revisions are read.

| Revision 1 decision | Revision 2 and 3 decision | Subject                                            |
| ------------------- | ------------------------- | -------------------------------------------------- |
| D5                  | D1                        | Metric identity, metadata and availability reasons |
| D3                  | D2                        | Reusable split contract with a leakage policy      |
| D6                  | D3                        | Explicit ambiguity policy with result provenance   |
| D4                  | D4                        | Multiple-testing-aware research reporting          |
| D10                 | D5                        | Numerical-stability testing                        |
| D7                  | D6                        | Label and target policy framework                  |
| D2                  | D7                        | Secondary implementation parity protocol           |
| D1                  | D8                        | Capability-result codes                            |
| D8                  | D9                        | Declarative research caching, deferred             |
| D9                  | D10                       | Provider adapter, relocated                        |
| D11                 | D11                       | Rust schema ownership, no change                   |

Revision 3 also moved sections in the design document, so a citation taken from revision 2 of this
record must be shifted: `design 8 L*` is now `design 9 L*`, `design 12` is now `design 13`,
`design 12.1` is now `design 13.1`, and the open questions moved from `design 13` to `design 14`.
Identity moved out of an invariant into `design 8`, and the invariants renumbered accordingly.

## 2. Probe provenance and reproduction

```bash
git clone --depth 1 https://github.com/polakowo/vectorbt.git target/vbt_probe
cd target/vbt_probe && git log --oneline -1
# ceffc50 docs: update VectorBT PRO feature links to the new page layout
# ceffc501f2d37033a79dd86a9f883e69ec6977bd
```

| Property           | Value                                                                                                  |
| ------------------ | ------------------------------------------------------------------------------------------------------ |
| Revision           | `ceffc501f2d37033a79dd86a9f883e69ec6977bd` (shallow clone of `master`)                                 |
| Package version    | `1.1.1`, from `vectorbt/_version.py`                                                                   |
| Rust crate         | `vectorbt-rust` 1.1.1, `rust/Cargo.toml`, pyo3 0.29, numpy 0.29, ndarray 0.17, rand 0.10               |
| Licence            | Apache-2.0 with Commons Clause (`LICENSE.md`, "Fair Code" badge; GitHub API reports `NOASSERTION`)     |
| Size               | 118 Python files, 105,199 lines including tests; 8 Rust files; 11 Markdown pages under `docs/`         |
| Tests              | 16 files, 36,259 lines, of which `test_portfolio.py` 10,831 and `test_engine.py` 2,543                 |
| Stars, forks       | 9,242 and 1,183                                                                                        |
| Created, last push | 2017-11-14, 2026-09-26                                                                                 |
| Releases           | v1.1.1 2026-09-26, v1.1.0 2026-07-05, v1.0.0 2026-04-22 (introduced the Rust engine), v0.28.5, v0.28.4 |
| Runtime deps       | numpy>=2.4.6, pandas>=3.0.3, numba>=0.66, scipy, matplotlib, plotly, anywidget, scikit-learn           |

Coverage and method:

| Area                      | How it was read                                                                                             |
| ------------------------- | ----------------------------------------------------------------------------------------------------------- |
| Engine dispatch           | `vectorbt/_engine.py`, `rust/README.md` and `tests/test_engine.py` in full                                  |
| Validation splits         | `vectorbt/generic/splitters.py` in full                                                                     |
| Portfolio simulation      | `vectorbt/portfolio/base.py` signatures, the `nb.py` stop paths and trade reconstruction, `enums.py` dtypes |
| Statistics                | `vectorbt/generic/stats_builder.py`, `returns/accessors.py`, `returns/metrics.py`, `generic/drawdowns.py`   |
| Signals and indicators    | `signals/factory.py`, `indicators/factory.py`, and the look-ahead controls in `nb.py`                       |
| Labels                    | `vectorbt/labels/` in full                                                                                  |
| Data                      | `vectorbt/data/base.py`, `custom.py`, `updater.py`                                                          |
| Configuration and caching | `vectorbt/utils/config.py`, `utils/decorators.py`, `_settings.py`                                           |
| Licence and packaging     | `LICENSE.md`, `pyproject.toml`, the `.github/workflows` listing and the release notes                       |
| Not read                  | `vectorbt/plotting`, the notebooks, `apps/`, `benchmarks/`, and VectorBT PRO (closed source)                |

The clone was deleted after this record was written. Any claim below can be re-derived from the pinned
revision; the numbers in section 3 can be re-derived from the working tree.

## 3. Verified state of this repository

Every row was verified by reading the file at the cited line in the working tree at code revision
`d71348e30a`. Rows marked "absent" record a search that returned no match, and the search scope is
stated.

| #   | Fact                                                                                                                                                 | Evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| --- | ---------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | Statistics are a Rust trait with one struct per statistic                                                                                            | `crates/analysis/src/statistic.rs:36` `pub trait PortfolioStatistic: Debug {`, `:40` `fn name(&self) -> String;`, `:47` `fn calculate_from_returns(&self, returns: &Returns) -> Option<Self::Item>`, `:56` `calculate_from_realized_pnls`, `:68` `calculate_from_positions`, `:79` `calculate_from_returns_with_benchmark`                                                                                                                                                                                                                                                                                                                     |
| 2   | 34 statistics are declared, one module each                                                                                                          | `crates/analysis/src/statistics/mod.rs:18-51`, 34 `pub mod` lines                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              |
| 3   | There is a name-keyed registry with runtime registration and deregistration                                                                          | `crates/analysis/src/analyzer.rs:38` `pub type Statistic = Arc<dyn PortfolioStatistic<Item = f64> + Send + Sync>;`, `:59` `pub statistics: AHashMap<String, Statistic>,`, `:123` `pub fn register_statistic(&mut self, statistic: Statistic)`, `:128` `pub fn deregister_statistic`                                                                                                                                                                                                                                                                                                                                                            |
| 4   | Twenty statistics are registered by default                                                                                                          | `crates/analysis/src/analyzer.rs:79-98`, twenty `register_statistic` calls                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                     |
| 5   | There was no metric metadata object, and D1 now declares one                                                                                         | `crates/analysis/src/metric.rs:452` `pub struct MetricDefinition` with the closed vocabularies at `:64` `MetricUnits`, `:112` `MetricTag`, `:196` `MetricDirection`, `:256` `MetricInput`, `:315` `MetricStatus`, `:374` `MetricReason`; `crates/analysis/src/statistic.rs:48` `fn definition(&self) -> MetricDefinition;`, implemented by all 34 statistics in `crates/analysis/src/statistics/*.rs`                                                                                                                                                                                                                                          |
| 6   | Statistics return `Option` and a skipped statistic leaves no trace in the name-keyed dictionaries, and D1 now reports it                             | `crates/analysis/src/statistic.rs:47`, `:56`, `:68` all return `Option<Self::Item>`; the analyzer inserts only `Some` values, which `crates/analysis/src/analyzer.rs:855` `report_returns_metrics`, `:916` `report_position_metrics` and `:955` `report_pnls_metrics` replace with a status and a reason per requested metric                                                                                                                                                                                                                                                                                                                  |
| 7   | Walk-forward validation exists as explicit stages                                                                                                    | `python/nautilus_trader/optimization/stages.py:51` `class WalkForwardWindow`, `:74` `def walk_forward_windows(`, `:128` `class TrainStage`, `:159` `class OptimizeStage`, `:217` `class ValidateStage`, `:263` `class OutOfSampleStage`, `:351` `class WalkForwardStage`                                                                                                                                                                                                                                                                                                                                                                       |
| 8   | The stage vocabulary is closed and typed                                                                                                             | `python/nautilus_trader/optimization/config.py:94` `STAGE_KINDS = ("optimize", "train", "validate", "out_of_sample", "walk_forward")`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| 9   | One search strategy is implemented, and searching is pure enumeration                                                                                | `python/nautilus_trader/optimization/search.py:37` `class SearchStrategy(Protocol)`, `:62` `class GridSearch`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                  |
| 10  | A run is identified by a canonical digest and recorded with its parameter mapping                                                                    | `python/nautilus_trader/optimization/space.py:47` `canonical_json`, `:74` `digest_of`, `:97` `class Experiment`, `runner.py:61` `class CanonicalRun`, `:85` `class FailedExperiment`                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| 11  | A search reports results and failures together                                                                                                       | `python/nautilus_trader/optimization/report.py:120` `class SearchReport`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| 12  | There was no purge or embargo rule, and the split contract now states one                                                                            | `python/nautilus_trader/optimization/splits.py:120` `class LeakagePolicy` with `purge_before`, `purge_after` and `embargo_after`, `:194` `purge` folding in the label horizon, `:572` `_excluded` applying the relation; outside it, `python/nautilus_trader`, `crates` and `docs` still contain no purge or embargo as a validation concept, the only other hits being cache-instrument purges in adapters                                                                                                                                                                                                                                    |
| 13  | There is no cross-validation splitter, and the split contract this review specified now exists                                                       | `python/nautilus_trader/optimization/splits.py:290` `class SplitContract`, `:247` `class Split`, `:104` `class SplitDirection`, `:120` `class LeakagePolicy`, `:89` `class LabelOverlapRule`, consumed by `python/nautilus_trader/optimization/stages.py:92` `walk_forward_windows`; no match for `cross_val` or `cv_split` anywhere in `python/nautilus_trader`, `crates`, or `docs`                                                                                                                                                                                                                                                          |
| 14  | There is no multiple-testing correction                                                                                                              | No match for `deflated`, `PBO`, `multiple testing`, or `bootstrap` in `python/nautilus_trader`, `crates/analysis`, `crates/backtest`, or `docs`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| 15  | Every hot path has exactly one implementation and it is Rust                                                                                         | `python/nautilus_trader/indicators/__init__.py:26` `from nautilus_trader._libnautilus.indicators import *`, `:29` `fixup_module_names(globals(), __name__)`; `crates/indicators/src/python/mod.rs:40-41` `#[pymodule]` / `pub fn indicators`                                                                                                                                                                                                                                                                                                                                                                                                   |
| 16  | The compiled extension is mandatory                                                                                                                  | `python/nautilus_trader/__init__.py:26` unconditional star import of `nautilus_trader._libnautilus`; no `find_spec`, no engine setting, no pure-Python fallback for any hot path                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| 17  | The Rust core owns every domain schema                                                                                                               | `crates/model/src/events/order/filled.rs:40-47` `pyo3::pyclass(module = "nautilus_trader.model", from_py_object)` and `pyo3_stub_gen::derive::gen_stub_pyclass` above `pub struct OrderFilled {`; `crates/model/src/python/mod.rs:302` `m.add_class::<crate::events::OrderFilled>()?;`                                                                                                                                                                                                                                                                                                                                                         |
| 18  | The version relationship is asserted at build time, not at run time                                                                                  | `crates/core/build.rs:30` `let nautilus_version = "2.0.0rc6";`; `python/nautilus_trader/__init__.py:29-34` derives `__version__` from installed distribution metadata; `Cargo.toml:55` `license = "LGPL-3.0-only"`                                                                                                                                                                                                                                                                                                                                                                                                                             |
| 19  | Order denial already reports a stable machine-readable code                                                                                          | `crates/model/src/events/order/denied_reason.rs:64-68` `#[strum_discriminants(name(OrderDeniedCode), derive(Display, AsRefStr, EnumIter, EnumString), strum(serialize_all = "SCREAMING_SNAKE_CASE"))]`; `:57` `Only the leading code is canonical. Consumers must not recover classification or control flow`                                                                                                                                                                                                                                                                                                                                  |
| 20  | The denial event itself carries only the rendered string                                                                                             | `crates/model/src/events/order/denied.rs:61` `pub reason: Ustr,` within `pub struct OrderDenied`                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| 21  | Configuration is typed, rejects unknown keys, and collects every violation with a field path                                                         | `crates/common/src/cache/config.rs:43` `#[serde(default, deny_unknown_fields)]`; `crates/backtest/src/config.rs:624-625` `pub fn validate(&self) -> ConfigResult<()>` with `let mut errors = ConfigErrorCollector::new();`, pushing `ConfigError::empty_field` and `ConfigError::range`                                                                                                                                                                                                                                                                                                                                                        |
| 22  | Bar-level ambiguity is resolved by a documented, configurable ordering policy                                                                        | `crates/backtest/src/config.rs:364` `pub bar_execution: bool`, `:367` `pub bar_adaptive_high_low_ordering: bool`, `:525-530` documents the fixed order Open, High, Low, Close and the adaptive heuristic; D3 now records the assumption policy this implies in the research layer, `python/nautilus_trader/optimization/assumptions.py:88` `BarAmbiguityPolicy`                                                                                                                                                                                                                                                                                |
| 23  | Catalog coverage and gap reporting exist and are mature                                                                                              | `crates/persistence/src/backend/parquet/catalog/coverage.rs:77` `pub fn get_missing_intervals_for_request(`, `:263` `get_intervals`; `crates/persistence/src/backend/parquet/catalog/mod.rs:244` `pub struct ParquetDataCatalog {`                                                                                                                                                                                                                                                                                                                                                                                                             |
| 24  | Nothing computes supervised labels, forward returns, extrema or trade MFE and MAE                                                                    | No match for `triple_barrier`, `forward_return`, `zigzag`, `local_extrema`, `mfe`, or `mae` in `crates/model`, `crates/data`, `crates/trading`, or `python/nautilus_trader`; the only Label concept is the proposal in `vnpy_lessons_design.md`                                                                                                                                                                                                                                                                                                                                                                                                |
| 25  | There was no study or trial identity, and the research layer now declares both while the pipeline does not emit them                                 | Before the identity contracts: rows 10 and 11 record a canonical digest, a run record and a report, and no match for `study_id`, `trial_id`, `trial_count`, `search_space_digest` or `validation_scheme_digest` existed in `python/nautilus_trader`, nor any `seed` in `python/nautilus_trader/optimization`. Now `python/nautilus_trader/optimization/identity.py:349` `StudyIdentity` and `:490` `TrialIdentity` exist, while the optimizer and the emitted document still record neither; the wiring decision is recorded in section 4.11                                                                                                   |
| 26  | The backtest result does not record which bar-ordering policy produced it, and the research layer now does                                           | `crates/backtest/src/result.rs:105-114` records metrics and an elapsed time; no `ambiguity_policy`, `policy_id`, or ordering field is present. D3 records the resolved policy in the research layer's emitted document instead, `python/nautilus_trader/optimization/config.py:606`, so the canonical backtest document and its pinned expectations are untouched                                                                                                                                                                                                                                                                              |
| 27  | There was no dataset or universe identity vocabulary, the name `kernel_version` is taken by the operating system, and the contracts now declare both | Before the identity contracts: no match for `dataset_identity`, `dataset_digest`, `universe_digest` or `membership_as_of` in `python/nautilus_trader` or `crates`; `as_of` appeared only in `python/nautilus_trader/persistence/catalog_to_df.py`; `crates/common/src/logging/headers.rs:85` uses `kernel_version` for the OS kernel. Now `identity.py:204` `DatasetIdentity` and `:146` `UniverseIdentity` declare the fields, while the catalog still has no dataset digest, the Parquet backend still rejects every as-of but the latest, and no stored membership history exists, so both are declarations rather than enforced guarantees |

Three rows corrected an expectation held before reading the code. Row 1 shows that a statistics
registry already exists, so the lesson is metadata and status rather than a registry (design D1). Row
7 shows that walk-forward validation already exists, so the lesson is the split *contract* and its
leakage policy rather than the concept of a split (design D2). Row 25 shows that trial provenance is
partial: a digest and a report exist, and the study and trial levels do not, which is why the
requirement in L4 is an identity record rather than a single statistic. The earlier probe produced a
similar correction for the universe and portfolio construction workstreams, and all of them are
recorded because the corrections are the evidence that the comparison was done against source.

## 4. Mechanism specifications

Each specification is written against our own requirements and cites the source mechanism only for
provenance. None of them may be implemented by copying vectorbt code (design 1.2 and 7.3). Mechanism
names are given as `L` identifiers from `design 9` and decision identifiers from `design 13`.

### 4.1 D1 Metric identity, metadata, status and reason codes

**Specification, implemented in revision 5** (`design 9 L1`); the implementation record and its
deviations are at the end of this section.

1. A statistic declares, beside its identity: a display title, units, tags and a direction drawn from
   a closed, action-oriented set (`maximize`, `minimize`, `target`, `informational`). A metric with a
   target value carries it in the definition, so a tracking error or a distance to a target is
   expressible without inventing a non-monotonic direction.
2. Parameters that currently appear inside a display name, such as the annualisation period, move
   into the metadata, and the title renders from them at presentation time.
3. A statistic reports a status from a four-state vocabulary, and the four states are semantically
   distinct: `computed`, `unavailable` (an input the definition requires was absent),
   `invalid` (the inputs were present and no meaningful value could be produced, for example a
   non-finite input or a zero denominator), and `not_registered` (the metric is not in the metric
   set). Every state other than `computed` carries a reason code from a closed, domain-owned set.
4. `invalid` must not be collapsed into `unavailable`, because that hides a data defect behind an
   applicability rule, and neither may be collapsed into a dropped row, which is what happens today.
5. Direction and status are declarative and consumed by reporting; no optimization logic lives inside
   a statistic.

**Acceptance and verification.** A result containing a computed, an unavailable, an invalid and an
unregistered statistic, asserted in a single test; a test that `invalid` and `unavailable` are
distinguishable on the same metric; a test that a title renders from metadata parameters; a test that
the units, tags and direction sets are closed.

| Item                                                                                                                                                                                                                                                                          | Where                                                                                                                                                                                                          |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| The closed vocabularies: units `Ratio`/`Fraction`/`Currency`, ten tags, directions `Maximize`/`Minimize`/`Target`/`Informational`, inputs `Returns`/`Benchmark`/`RealizedPnls`/`Positions`, the four statuses and seven reason codes, each with `ALL`, `as_str` and `Display` | `crates/analysis/src/metric.rs:64` `MetricUnits`, `:112` `MetricTag`, `:196` `MetricDirection`, `:256` `MetricInput`, `:315` `MetricStatus`, `:374` `MetricReason`                                             |
| The definition: stable `id`, a title template rendered from named parameters, `units`, `tags`, `direction`, `target` and `inputs`, with `is_defined_over`                                                                                                                     | `crates/analysis/src/metric.rs:452` `MetricDefinition`, `:730` `format_number`                                                                                                                                 |
| The result and the report: `computed`, `not_computed` with a status-dependent reason, `results`, `get`, `with_status`                                                                                                                                                         | `crates/analysis/src/metric.rs:593` `MetricResult`, `:683` `MetricReport`                                                                                                                                      |
| The trait requirement, so no statistic can reach a report without declaring what it reports and over what                                                                                                                                                                     | `crates/analysis/src/statistic.rs:48` `PortfolioStatistic::definition`                                                                                                                                         |
| The three report methods and their classification helper                                                                                                                                                                                                                      | `crates/analysis/src/analyzer.rs:855` `report_returns_metrics`, `:916` `report_position_metrics`, `:955` `report_pnls_metrics`, `:1200` `classify`                                                             |
| The metadata of all 34 built-in statistics                                                                                                                                                                                                                                    | `crates/analysis/src/statistics/*.rs`, one `definition()` per module                                                                                                                                           |
| The Python surface: the six vocabulary classes, the three value classes, `metric_definitions`, the three report methods, and the declarations a Python subclass inherits                                                                                                      | `crates/analysis/src/python/{metric,analyzer,statistic}.rs`, `python/nautilus_trader/analysis/statistic.py:70` `metric_id`, `:88` `units`, `:102` `tags`, `:117` `direction`, `:133` `target`, `:147` `inputs` |
| The concept documentation                                                                                                                                                                                                                                                     | `docs/concepts/portfolio.md` section "Metric identity, metadata and status"                                                                                                                                    |

The vocabulary, which is the answer to `design 14` question 6 and is proposed by the implementation
rather than by an owner decision:

- **Units** (closed, three members): `ratio` a dimensionless value not bounded to a unit interval,
  `fraction` a dimensionless value bounded to it, `currency` money. Nothing in the built-in set
  needs a fourth, and an unused variant would be vocabulary without a member.
- **Tags** (closed, ten members): `returns`, `risk`, `risk_adjusted`, `drawdown`, `trade`, `exposure`,
  `benchmark_relative`, `distribution`, `tail`, `annualised`. Every member has at least one built-in
  statistic.
- **Directions** (closed, four members): `maximize`, `minimize`, `target`, `informational`, as the
  design specified. `target` is exercised by `TrackingError`, which declares target `0.0`.
- **Inputs** (closed, four members): `returns`, `benchmark`, `realized_pnls`, `positions`.
- **Statuses** (closed, four members): `computed`, `unavailable`, `invalid`, `not_registered`.
- **Reason codes** (closed, seven members, each mapped to exactly one status): `insufficient_data`,
  `unsupported_input` and `missing_benchmark` and `unresolved_currency` are `unavailable`;
  `non_finite_input` and `undefined_result` are `invalid`; `not_in_metric_set` is `not_registered`.

Two deviations from the specification, both deliberate, and two additions:

1. **The display string is preserved; identity is new.** The specification asked for parameters to
   move out of the display name into the metadata. That is implemented - the parameter is a declared
   `parameters` entry and `title()` renders it from the template - but the *rendered* string is
   unchanged, so `Sharpe Ratio (252 days)` remains the reporting key in the name-keyed dictionaries.
   Renaming those keys would change the canonical backtest document, and with it every pinned
   expectation in the regression suite, which the working agreement forbids regenerating. The stable
   identity is therefore additive: `sharpe_ratio` addresses the metric, the title stays as it was.
2. **`definition()` is required, not defaulted.** A default derived from `name()` would fabricate
   `units` and `direction` for a statistic that declared nothing, which is the silence the design's
   fourth point rejects. The Python base class supplies the declarations a subclass does not
   override, and the Rust trait does not.
3. **The reason is derived, not declared per statistic.** `invalid` versus `unavailable` is decided
   from the input state and the produced value (an empty input, a non-finite input, a non-finite
   result, or a declined calculation), rather than by teaching all 34 statistics to name a reason.
   The declared `inputs` metadata does the applicability work instead, and the test in
   `analyzer.rs` asserts that a statistic which computes from an input declares it.
4. **A metric can be addressed by identity or by name.** `find_metric` resolves the definition id
   first and the registry name second, so adopting an identity does not break existing callers that
   pass a display string.

**Verified by execution.** `cargo nextest run -p nautilus-analysis --lib --features python` runs 308
tests, all passing. The seven new Rust tests cover: the four statuses in a single report
(`max_drawdown` computed, `long_ratio` unavailable with `unsupported_input`, an unknown request
not registered, and every non-computed result carrying a reason); `invalid` and `unavailable`
distinguished on the same metric (`sharpe_ratio` invalid with `non_finite_input` on a series
containing `NaN`, against `beta` unavailable with `missing_benchmark` when no benchmark is supplied,
which becomes computed once one is); the same metric addressed by id and by name; an empty input
reported unavailable with `insufficient_data`; a definition marked derived rather than declared; and
an invariant test over all 34 built-in statistics that `definition().title() == name()`, that the
identities are unique across the set, and that any input a statistic computes from is declared.
`cargo clippy --locked -p nautilus-analysis --all-targets --features python -- -D warnings` is clean.
On the Python side, `pytest tests/unit/analysis tests/unit/portfolio tests/unit/optimization
tests/integration/test_optimization.py` runs 365 tests, all passing, including the nine metric
identity tests (the vocabularies, the four statuses through the binding, addressing by id and by
name, a user statistic's own declaration, a duck-typed statistic registering with a derived
definition, a mistyped declaration rejected at registration), and `pytest
tests/regression/test_regression.py` runs all nine declared scenarios with
`python/tests/regression/expected` unmodified. The extension was rebuilt and the type stubs
regenerated from it, so the Python evidence is against the change rather than a stale build.

**Not measured.** The report methods and the vocabulary were exercised through the Rust tests, the
Python unit tests and the regression scenarios; the tearsheet, reporter and optimization consumers
were left on the name-keyed dictionaries they already use, because migrating them is a
behaviour-changing decision rather than part of this contract. Whether the vocabulary's membership
should differ is the owner's call, not a measurement.

### 4.2 D2 Reusable split contract with a leakage exclusion relation

**Specification, implemented in revision 4** (`design 9 L2`); the implementation record and its
deviations are at the end of this section.

1. A split is produced by a contract that yields, per split, a tuple of index arrays, one per set. The
   contract is duck-typed: anything with a `split(X, **kwargs)` method satisfies it.
2. Set lengths accept both fractions of the window and absolute counts; the lengths that are given
   determine all but one set, and the remaining set absorbs the remainder.
3. A direction flag decides which set absorbs the remainder: forward means the remainder joins the
   last set, reversed means it joins the first.
4. A minimum length filters windows before selection, and a requested number of splits selects that
   many evenly spaced windows rather than the first ones, so a coarse study still spans the sample.
5. An empty set and a request that cannot be satisfied raise, naming the constraint.
6. The leakage policy is expressed as an exclusion relation, not as a time distance alone: purge
   before, purge after, embargo after, and a label overlap rule. The label overlap rule is the
   operative part, because a label at `t` covering `t+1` to `t+20` makes training observations near
   the test boundary overlap the target period of an evaluation observation, and no arrangement of
   purge distances repairs that unless the rule is stated. The first implementation may derive the
   three intervals from one duration, but the contract must be capable of expressing them separately.
7. The concept is mandatory and the values are not: a zero purge and a zero embargo are legitimate
   for a study with no label overlap, provided the study records the justification, and a zero
   interval by omission must be distinguishable from a zero interval by decision.
8. The existing optimization stages consume the contract rather than owning window generation, so
   walk-forward and split-based validation share one implementation of the bounds.
9. Statistics are computed per split by the caller, as they are today; the contract returns indices
   and does not aggregate.

**Acceptance and verification.** A contract test over several lengths and set-length combinations
including the fractional and absolute forms and the reversed direction; a test asserting that no
training observation overlaps the label information of an evaluation observation; a test that an
unsatisfiable request raises with the constraint named; a test that a zero interval without a
justification is refused; a test that the three exclusions can be expressed independently; and a
regression scenario asserting that existing walk-forward stages produce identical windows after
migrating onto the contract.

**Implemented.** `python/nautilus_trader/optimization/splits.py` (new module):

| Item                                                                                                                                                                                                       | Where                                                                                                   |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| `SplitContract` with named sets, lengths, evaluation set, window, stride, minimum length, split count and direction                                                                                        | `splits.py:290`, validation `:333-387`, `resolved_lengths` `:396`                                       |
| The layout: whole windows, a fractional length, an omitted length, and `EXACT`/`FORWARD`/`REVERSED` for the leftover span                                                                                  | `splits.py:487` `split`, `_layout` `:532`, `_bounds` `:547`                                             |
| The exclusion relation applied per split, dropping a split whose set becomes empty or too short                                                                                                            | `splits.py:572` `_excluded`                                                                             |
| Evenly spaced selection of a requested split count                                                                                                                                                         | `splits.py:601` `_select`                                                                               |
| `LeakagePolicy` with `purge_before`, `purge_after`, `embargo_after`, `label_overlap_rule`, `label_horizon` and `zero_interval_justification`, and its `purge`, `after`, `gap`, `decided`, `digest` surface | `splits.py:120`, `:193-237`                                                                             |
| `Split` with the set bounds and the index arrays for a caller's timestamp sequence                                                                                                                         | `splits.py:247`, `indices` `:266`                                                                       |
| The walk-forward stages migrated onto the contract, with a declared zero policy carrying its reason                                                                                                        | `stages.py:77` `WALK_FORWARD_SETS`, `:82` `WALK_FORWARD_LEAKAGE`, `:92` `walk_forward_windows`          |
| The policy reachable from a configuration document, refused outside the walk-forward stage                                                                                                                 | `config.py:99` `_LEAKAGE_KEYS`, `:189` `StageSpec.leakage`, `:515`, `:741`, `:755` `_parse_leakage`     |
| The public surface, the package docstring and the concept documentation                                                                                                                                    | `optimization/__init__.py:43-47`, `docs/concepts/optimization.md` section "Split contracts and leakage" |

Two deviations from the specification, both deliberate, and two additions:

1. **The contract is a dataclass, not a duck-typed protocol.** The specification asked for anything
   with a `split` method to satisfy it. There is exactly one consumer today, so a protocol would be
   an unused indirection; the walk-forward path depends on `SplitContract` directly. If a second
   splitter is ever wanted, the protocol is the refactor to make then, with a second implementation
   to justify it.
2. **The contract returns half-open nanosecond bounds per set, not index arrays.** Bounds are the
   domain of this package, which works in run-window nanoseconds; `Split.indices(timestamps)` turns
   them into the index arrays for a caller's own observations, which is what the specification's
   requirement means in use. A timestamp sequence is the missing input that made the literal reading
   incoherent inside a nanosecond window.
3. **`SplitDirection.EXACT` was added** to the forward and reversed directions the specification
   named, because the existing walk-forward windows drop the leftover span and the migration had to
   preserve them. `FORWARD` and `REVERSED` behave as specified.
4. **`embargo_after` is enforced as a minimum gap between splits rather than a within-split
   interval.** That is the only reading in which the embargo differs from the two purges: within one
   window the sets are disjoint by construction, so a within-split embargo would be a second name for
   `purge_after`. The design's own requirement that the three intervals be independently expressible
   is what forces the reading, and the tests assert each interval's separate effect.

**Verified by execution.** `pytest tests/unit/optimization/test_splits.py` (31 tests) covers the
absolute, fractional and omitted lengths, all three directions, each exclusion on its own, the label
overlap invariant `max(training) + horizon <= min(evaluation)`, the index arrays, the drop on an
empty or too short set, the evenly spaced selection, the refusal of an unsatisfiable split count, the
refusal of an unjustified zero and the distinguishability of omission from a decided zero. `pytest
tests/unit/optimization/test_config.py` (23 tests) covers the configuration path, including the
refusal of a leakage policy outside the walk-forward stage. `pytest
tests/integration/test_optimization.py` (9 tests) passes unchanged, including the scenario that pins
the pre-migration windows, and the declared regression scenario `optimization_golden` passes with its
expected document unmodified. A throwaway script drove `_walk_forward_document` end to end over a
3,000 ns period with 400 ns in-sample and 200 ns out-of-sample segments: five windows by default
(first `in_sample_end` 400), five windows with `purge_before` 100 and the first `in_sample_end` 300
leaving a 100 ns gap before the 400 to 600 evaluation window, and four windows with `embargo_after`
100 starting at 0, 700, 1,400 and 2,100. The script was deleted afterwards.

One observation for the owner, not a defect: because a policy must justify every interval it leaves
at zero, declaring a partial policy such as `purge_before` alone requires the justification as well.
The alternative, justifying only an all-zero policy, would make a partially applied policy's
silent zeros the norm, which is what the rule exists to prevent.

### 4.3 D3 Ambiguity policy identity, separate from execution simulation

**Specification, implemented in revision 6** (`design 9 L3`); the implementation record and its
decisions are at the end of this section.

1. An ambiguity policy is a versioned value with an identity: the policy id, the trigger precedence,
   the intrabar ordering, the gap handling, the simultaneous-event handling and a version.
2. Every assumption that a bar-only replay cannot determine is documented in one place, in the
   user-facing documentation and not only in the source: the ordering of the bar's prices, the price
   used when a stop triggers, the priority between simultaneous triggers, and the treatment of a gap
   through a threshold.
3. A result records the ambiguity policy identity that produced it, not the word "pessimistic", so a
   policy change does not silently rewrite the interpretation of historical results.
4. Ambiguous configurations are rejected: a setting whose meaning depends on an unspecifiable
   ordering is a configuration error rather than a silent convention.
5. The ambiguity policy is distinct from an execution simulation policy. The research layer may record
   an execution assumption; it must not own execution semantics, because owning them would place venue
   and market-rule concerns inside the research layer (design 1.1).
6. The contract and the default are separate decisions. The architecture requires that ambiguity is
   explicit, identified and unable to resolve silently. Which policy is the default is an owner
   decision, because it changes published numbers (`design 14` question 1). If the selected default is
   pessimistic, that property is tested rather than assumed.
7. vectorbt's specific rules are not adopted as our fill rules: they belong to a bar simulator whose
   model is not ours.

**Acceptance and verification.** A test that each documented assumption has a matching assertion in
the simulation tests; a test that an ambiguous configuration is rejected with a named constraint; a
test that the policy identity appears in the result metadata; a test that two results produced under
different policy versions are distinguishable; and, once the owner names the default, a test that
asserts the named default rather than an assumed one.

| Item                                                                                                                     | Where                                                                                                                                        |
| ------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------- |
| The four rule axes as closed vocabularies                                                                                | `python/nautilus_trader/optimization/assumptions.py:48` `IntrabarPath`, `:57` `TriggerPrecedence`, `:65` `TriggerFill`, `:73` `GapHandling`  |
| The policy value with its identity, its canonical mapping and its digest                                                 | `assumptions.py:88` `BarAmbiguityPolicy`, `:165` `declared_default`, `:181` `from_venue_flags`, `:226` `to_dict`, `:246` `digest`            |
| The declaration in a configuration file, under `assumptions`, refused when it is ambiguous                               | `python/nautilus_trader/optimization/config.py:138` `_ASSUMPTION_KEYS`, `:236` `OptimizationConfig.assumptions`, `:300` `_parse_assumptions` |
| The policy identity recorded in the emitted result document                                                              | `config.py:606` `_attach_context`, which writes `document["assumptions"]`                                                                    |
| The public surface                                                                                                       | `python/nautilus_trader/optimization/__init__.py`, `BarAmbiguityPolicy` and the four vocabulary names in `__all__`                           |
| The documented assumptions, each paired with its implementation site and its assertion, in the user-facing documentation | `docs/concepts/optimization.md` section "Execution assumptions"                                                                              |

Six decisions, all deliberate:

1. **The research layer records the assumptions; the engine keeps the rules.** No Rust type,
   configuration or behaviour changed. The policy names the engine's rules rather than reimplementing
   them, which is what keeps venue and market-rule concerns out of this layer (design 1.1 and D3
   point 5).
2. **The named default is the status quo, not a new policy.** `declared_default()` names the
   behaviour the engine already implements: bar execution on, the fixed Open, High, Low, Close
   sequence, adaptive ordering off. The design makes the default an owner decision because changing
   it changes published numbers; naming the existing behaviour changes none, and `from_venue_flags`
   is the single place the mapping changes if the owner selects a different default. The test asserts
   the named default rather than an assumed one, as the acceptance requires, and the owner can
   replace the name without touching the contract.
3. **Ambiguity is refused at both levels.** `from_venue_flags` refuses adaptive ordering without bar
   execution, and the policy constructor refuses an intrabar path declared while bar execution is
   off and bar execution declared with no path. The declaration is therefore total: no field is
   meaningless, and two studies cannot look identical while assuming different things.
4. **Three of the four axes are single-member sets.** The engine implements one rule for trigger
   precedence, one fill rule for a trigger inside the bar and one gap rule, and the design asks the
   policy to carry them as identified values rather than as prose. A second member is what a future
   policy version adds, and the version is part of the identity, so a change is visible rather than
   silent. `IntrabarPath` has two members because the engine has two behaviours.
5. **The declaration is spelled as a venue configuration spells it** (`bar_execution`,
   `adaptive_high_low_ordering`), so a study author recognises the setting, while the resolved policy
   records the axis it implies (`intrabar_path`).
6. **The emitted document gained an `assumptions` key.** The CLI document is not pinned wholesale by
   the regression scenarios; they were run and pass unchanged, which is stated with the evidence
   rather than assumed.

**Verified by execution.** `pytest tests/unit/optimization` runs 61 tests, all passing, of which
seven are new: the declared default asserted axis by axis and shown equal to the venue flags with
adaptive ordering off; adaptive ordering selecting the adaptive path with a different digest; no bar
execution leaving the path unset; the three ambiguous declarations refused with their named
constraints; two versions distinguishable by digest; the identity payload carrying all four axes,
with each axis's members pinned; and a mistyped declaration refused. `pytest
tests/integration/test_optimization.py` (9 tests) and all nine declared regression scenarios pass
with `python/tests/regression/expected` unmodified, so adding the recorded policy to the emitted
document moves no pinned expectation. `ruff check` and `ruff format --check` are clean on the
changed files.

**The assumption-to-assertion map was checked by reading, not by running.** Each of the four
assumptions is paired in the documentation with an implementation site and a test that asserts it:
`crates/execution/src/matching_engine/mod.rs:1907` and `:1914` with
`crates/backtest/tests/integration/backtest_engine.rs:1411`
`test_add_data_rejects_bar_internal_aggregation`; `matching_engine/mod.rs:2325` `bar_high_first` with
`crates/execution/tests/integration/matching_engine.rs:9447`
`test_bar_execution_fills_stop_order`; the adaptive path at `:2325` and `:2040` with
`matching_engine.rs:9514` `test_bar_adaptive_ordering_fills_low_side_first` and `:9599`
`test_quote_bar_adaptive_ordering_fills_low_side_first`; and the trigger-inside-bar versus gap rule
at `matching_engine/mod.rs:4560-4570` with `docs/concepts/backtesting/fill-prices-and-matching.md`.
Those test names and sites were re-read at those lines while writing this record. **Not measured:**
the Rust tests themselves were not executed for this item, because nothing in Rust changed, so the
mapping is a citation rather than an exercised assertion.

### 4.4 D4 Multiple-testing-aware research reporting

**Specification, implemented in revision 8** (`design 9 L4`); the implementation record and its
deviations are at the end of this section.

1. The study record carries the trial provenance: study id, dataset identity, search-space digest,
   parameter count, trial count, failed trial count, validation-scheme digest, objective definition,
   selection rule and study seed, and each trial carries its own identity (`design 8.2`). The
   requirement is the record; a correction is only as good as the count, the space and the trials it
   was drawn from.
2. The initial statistic is the deflated Sharpe ratio, computed from the estimated per-period Sharpe
   ratio, the variance of the Sharpe ratio across the trials, the number of trials, the backtest
   horizon in periods, the skew and the non-excess kurtosis.
3. The statistical contract is specified before the first test is written, and every element of the
   table below is part of the specification rather than left to the implementation.
4. The value is reported, never a gate (design 7.6). Below the minimum observations or the minimum
   trials it is `unavailable` with a reason code, consistent with D1.
5. The record is forward compatible: adding further diagnostics, such as an overfitting probability or
   a bootstrap, changes no earlier part of the contract.

| Element                     | Requirement                                                                                               |
| --------------------------- | --------------------------------------------------------------------------------------------------------- |
| Return definition           | The return series that feeds the Sharpe ratio is stated, including the compounding convention             |
| Risk-free treatment         | A stated rate series or an explicit zero, never an implicit assumption                                    |
| Minimum observations        | Below this count the statistic is `unavailable` with a reason, not computed                               |
| Minimum trials              | Below this count the correction is noise on noise and the statistic is `unavailable`                      |
| Trial independence          | Stated explicitly; a study whose trials are dependent must say so                                         |
| Trial dependence            | Nominal and effective trial counts are distinguished wherever the chosen correction needs them            |
| Sharpe convention           | Per-period, with the estimator and the divisor convention pinned                                          |
| Variance, skew, kurtosis    | Estimators named; the kurtosis convention is non-excess                                                   |
| Annualisation               | Prohibited: annualised inputs are rejected at the boundary rather than divided silently                   |
| Missing returns             | Excluded rather than zero-filled, and the horizon counts contributing periods                             |
| Failed and duplicate trials | A failed trial is counted and identified; a duplicate parameter set is detected rather than counted twice |

**Acceptance and verification.** Each of: a hand-computed deterministic case; zero, one and many trial
cases; a missing-return case; a non-excess-kurtosis case; a large trial-count case; an annualised-input
rejection; an insufficient-input case producing `unavailable` with a reason; a dependent-trial case
that is required to declare its dependence; and, where applicable, a monotonicity check that more
trials under the null do not raise the corrected value.

**Implemented.** `python/nautilus_trader/optimization/significance.py` holds the contract and the
correction, and the tables below record what was built, the decisions taken and the verification.

| Item                                                                           | Where                                                                                                                          |
| ------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------ |
| The statistical contract, every element declared and digested                  | `significance.py` `StatisticalContract`, `SIGNIFICANCE_CONTRACT_ID`, `SIGNIFICANCE_CONTRACT_VERSION`                           |
| The two closed vocabularies the contract needs, and the annualisation boundary | `significance.py` `DivisorConvention`, `SharpeFrequency`, and `ReturnCompounding` for the stated series                        |
| The dependence declaration and the nominal/effective distinction               | `significance.py` `TrialDependence`, `SharpeSample.dependence`, `SharpeSample.effective_trials`, `SharpeSample.counted_trials` |
| The estimation chain, so the whole input is computable from a return series    | `significance.py` `per_period_sharpe` (`SharpeEstimate`), `return_moments` (`ReturnMoments`)                                   |
| The correction, with `unavailable` and `invalid` distinct                      | `significance.py` `deflated_sharpe_ratio`, `SignificanceResult`, `_normal_ppf`                                                 |
| The trial provenance from the optimizer's own records                          | `significance.py` `trial_provenance_from_runs`, `significance_report`, `SignificanceReport`                                    |
| The concept documentation                                                      | `docs/concepts/optimization.md` section "Multiple-testing-aware reporting"                                                     |
| The tests                                                                      | `python/tests/unit/optimization/test_significance.py`                                                                          |

Facts the implementation fixes, in our own terms rather than as a citation:

1. **The correction is a study-level statistic**, deliberately outside the compiled kernels and not a
   `PortfolioStatistic`: a run-level statistic is computed from one run's returns, and this one is
   computed from a set of trials. It therefore carries its own contract identity rather than a
   `MetricDefinition`.
2. **The value is a probability** in `[0, 1]`: the probability that the selected trial's per-period
   Sharpe ratio exceeds the maximum that the declared number of trials would have produced with no
   skill. `SR0 = sqrt(V[SR]) * ((1 - gamma) * Phi^-1(1 - 1/N) + gamma * Phi^-1(1 - 1/(N*e)))`, where
   `V[SR]` is the cross-trial variance of the Sharpe estimates, `N` the counted trial count, and
   `gamma` the Euler-Mascheroni constant; `DSR = Phi((SR - SR0) * sqrt(T - 1) /
   sqrt(1 - skew * SR + ((kurtosis - 1) / 4) * SR^2))` with `T` the contributing periods. A larger
   `N` lowers the value, which is the correction rather than an artifact of it.
3. **The distribution function is `math.erfc`**, not a series, so accuracy does not degrade in the
   upper tail where the quantiles live, and the quantile function is a safeguarded Newton iteration
   on a maintained bracket, falling back to bisection whenever a step would leave it. The far tail is
   safe because the bracket is: there the density underflows and an unguarded Newton step diverges.
   No dependency is added.
4. **Statuses and reasons are D1's vocabulary, not a new one.** Below the minimum observation or the
   minimum trial count, and whenever fewer than two trial estimates exist, the status is `computed`'s
   opposite `unavailable` with `insufficient_data`; a variance factor that is not positive is
   `invalid` with `undefined_result`. The counts are recorded beside the reason, so which threshold
   was short is visible. `not_registered` does not arise: there is no registry here.
5. **The annualisation rejection is a required declaration**, not a magnitude heuristic: a sample that
   declares `SharpeFrequency.ANNUALISED` is refused, because a correction defined per period would
   otherwise silently divide an annualised value by an assumed number of periods per year. The
   built-in `sharpe_ratio` statistic is annualised (`crates/analysis/src/statistics/sharpe_ratio.rs`
   multiplies by `sqrt(period)`) and tagged `Annualised`, so the built-in Sharpe ratio cannot be fed
   to the correction unnoticed.
6. **The non-excess kurtosis convention is enforced by the mathematics.** The fourth standardized
   moment is at least 1 for any distribution, so a value below 1 is refused: an excess kurtosis of 0,
   which is the wrong convention and would otherwise look plausible, is a declaration error rather
   than a number.
7. **Dependence is declared and the effective count replaces the nominal one** in the expected
   maximum; the cross-trial variance is still estimated from the trial estimates themselves, because
   that dispersion is what the maximum was drawn from. A dependent study without an effective count,
   and an independent study with one, are both refused.
8. **A duplicate parameter set is refused rather than counted twice.** Two runs of one experiment are
   one trial, and counting them twice would inflate `N`, which is the direction that flatters a
   result. A failed trial is counted and identified: it is recorded in the provenance and gets a
   trial identity of its own, and it is excluded from the correction's `N` because it produced no
   estimate for the maximum to have been selected over.
9. **The minimums are declared defaults the owner may replace**: twenty contributing periods and ten
   trial estimates. They are parameters of the contract rather than statistical constants, they are
   recorded with every result, and they are the boundary at which the statistic is `unavailable`
   rather than a number the sample cannot support. This is the implementation's answer to `design 14`
   questions 3 and 4, and it is replaceable without touching the mechanism.
10. **The correction is not wired into the emitted result document or the CLI.** The identity
    contracts were deliberately not wired either (section 4.11), and the same reason applies: the
    configuration declares no dataset, and a study's per-period trial Sharpe ratios are not retained
    by a run record, so a sweep's report cannot honestly be corrected from what it currently keeps
    (see "Not measured" below). The API is the deliverable, and D4's acceptance is a study record, a
    specified contract and a reported value.

**Verified by execution.** `pytest tests/unit/optimization/test_significance.py` runs 21 tests, all
passing, and they cover every acceptance case in the list above: the hand-computed deterministic case
in two forms (a sample with no dispersion and no Sharpe ratio is exactly `0.5`, and a two-period case
is exactly half the complementary error function of minus a half over the square root of two); zero,
one and many trials as three distinct statuses; a missing-return case that excludes `None` and NaN
rather than zero-filling them and reports two contributing periods; a non-excess-kurtosis case whose
expected values are hand-computed (`[-2, -1, 1, 2]` has kurtosis `1.36`, a two-point sample exactly
`1.0`, and `[1, 1, 3]` has skew exactly `1 / sqrt(2)`); a large trial-count case (a thousand trials,
still computed, and smaller than the ten-trial case); the annualised-input rejection; the
insufficient-input case as `unavailable` with `insufficient_data` and the boundary asserted from both
sides (twenty and nineteen observations, ten and two trials); the dependent-trial case, refused
without a declaration and corrected identically to an independent study of the same effective count;
and a monotonicity check in which a two-point trial sample with a constant variance never rises as
the trial count grows from two to a thousand. The formula is additionally compared against a
recomputation that shares no step with it: the same published formula with `math.erfc` and a pure
bisection quantile, agreeing to `1e-12`. `pytest tests/unit/optimization tests/integration/test_optimization.py`
runs 99 tests, `pytest tests/unit/analysis tests/unit/portfolio` runs 295, and
`pytest tests/regression/test_regression.py` runs all nine declared scenarios, with
`python/tests/regression/expected` unmodified. `ruff check` and `ruff format --check` are clean on
the module, the package initialiser and the tests.

**Not measured.** The correction reads its trial Sharpe ratios from the caller: a `SearchReport`
retains each run's metric values and canonical document, not its return series, so a sweep cannot be
corrected end to end from what it currently keeps, and no such end-to-end test exists. That is a
limitation of the run record rather than of the correction, and it is the same boundary recorded in
point 10. The two declared default thresholds are a documented floor rather than a measured one: no
comparison of corrected values across candidate thresholds was run, because that is the owner's
decision and would need a sweep to compare over.

### 4.5 D5 Numerical-stability obligation, classified by kernel type

**Specification, implemented in revision 9** (`design 9 L5`); the implementation record and its
deviations are at the end of this section. Every research kernel that reduces, transforms or
estimates is classified, and its class determines the obligation. The classification table in
`design 9 L5` is the specification, reproduced here with the obligations:

| Kernel class            | Obligation                                                                                                             |
| ----------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| `reduction`             | Empty input, single element, NaN propagation, layout                                                                   |
| `rolling_reduction`     | The reduction cases plus running-variance stability over a long series and the minimum-period-greater-than-length case |
| `cumulative`            | The reduction cases plus overflow, underflow and catastrophic cancellation                                             |
| `normalization`         | The reduction cases plus invariance under translation and scaling, and a zero-denominator case                         |
| `statistical_estimator` | The reduction cases plus a documented divisor convention, a minimum-observation case and an independent recomputation  |
| `transform`             | Empty input, single element, NaN edges and layout only                                                                 |
| `label`                 | The transform cases plus the wait convention and a leakage boundary case                                               |

1. A kernel that is not classified is an incomplete obligation, not an exempt one; classification is
   part of the work rather than an escape from it.
2. The running-variance case is compared against a compensated or two-pass computation, never against
   the kernel's own twin.
3. The minimum-period-greater-than-length case must yield NaN rather than a partial aggregate.
4. The divisor convention, sample or population, is documented per estimator, because a one-off
   difference is invisible in a parity test and changes every later number.

**Acceptance and verification.** The classification exists for every research kernel, and for each
kernel the obligation for its class is met. Parity alone is explicitly not the standard.

**Implemented.** `crates/analysis/src/kernel.rs` holds the classification and its obligations, and
the tests below exercise them. The work also found and fixed four kernels whose missing-value
handling and one whose summation did not meet the obligation their class assigns them.

| Item                                                                  | Where                                                                                                                     |
| --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| The seven classes, each with its stable name and the cases it obliges | `kernel.rs` `KernelClass`, `KernelClass::ALL`, `as_str`, `obligation`                                                     |
| The classification of every research kernel, with its source          | `kernel.rs` `KernelClassification`, `RESEARCH_KERNELS`, `classify`, `kernels_of_class`                                    |
| The compensated summation the dispersion kernel now uses              | `statistic.rs` `compensated_sum`, called by `calculate_std`                                                               |
| The missing-value rule, enforced where it was violated                | `statistics/max_drawdown.rs`, `statistics/value_at_risk.rs`, `statistics/expected_shortfall.rs`, `statistics/win_rate.rs` |
| The concept documentation                                             | `docs/concepts/portfolio.md` section "Numerical stability of the research kernels"                                        |
| The tests                                                             | `kernel.rs` test module, eleven tests                                                                                     |

Facts the implementation fixes:

1. **The class is the kernel's dominant numerical operation**, not a name for its output. A kernel
   that aggregates without a denominator is a `reduction`; one that divides by a level, a level's
   change or a count is a `normalization`; one that estimates a distribution parameter is a
   `statistical_estimator`; one that accumulates along the series is `cumulative`; one that maps a
   series to a series is a `transform`. Of the thirty-four built-in statistics, ten are reductions,
   thirteen normalizations, eight statistical estimators and three cumulative, with the three shared
   kernels (`calculate_std`, `downsample_to_daily_bins`, `align_returns`) classified beside them.
2. **`rolling_reduction` and `label` have no member yet, and they are declared anyway.** A kernel
   that joins them inherits an obligation rather than an exemption, and the table says so rather than
   omitting the classes. This is the same reasoning as "a kernel that is not classified is an
   incomplete obligation": an empty class is a statement about the crate, not a gap in the table.
3. **The classification cannot drift from the statistics.** A test compares the table against
   `builtin_statistics()` in both directions: every built-in statistic is classified, and every entry
   whose source is under `statistics/` is a built-in statistic. Adding a statistic without
   classifying it fails the test.
4. **The missing-value rule is mechanical rather than a convention.** A kernel must either propagate
   a missing observation (a non-finite result) or exclude it completely, and a test compares each
   kernel's result on a series containing a missing observation against the same series without it.
   The third outcome, keeping the observation in a count or a rank, is a silent reinterpretation and
   is what the test detects. The rule is the kernel-level form of the reporting rule that a missing
   return is excluded rather than zero-filled.
5. **The dispersion kernel is compared against a compensated computation, not against its own twin.**
   A hundred thousand values with a large offset relative to their spread is the adversarial case a
   parity test cannot see: the previous plain summation was wrong by 0.71 relative to a compensated
   two-pass computation, and the compensated kernel now agrees to 1.6e-16 (`statistic.rs`,
   `compensated_sum`). The test asserts both halves, so it cannot pass by being vacuous: the kernel
   must agree tightly *and* the naive form must still be materially wrong on the same input.
6. **The divisor convention is pinned by an exact value.** `calculate_std` uses Bessel's correction
   (the sample divisor `n - 1`), asserted on a two-element sample where the two conventions differ by
   a factor of two, and the minimum-observation case yields `NaN` for one element and for none.

The four missing-value defects found by the rule, and their fix:

| Kernel               | Observed                                                                                                                                                         | Fix                                                                                                                                                                                   |
| -------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `max_drawdown`       | A `NaN` return made the equity path unknown, every comparison against it false, and the kernel reported `-0.0` where the series without the return drew down 50% | A non-finite return propagates: the drawdown of an unknown path is unknown, and the kernel returns `NaN`                                                                              |
| `value_at_risk`      | A `NaN` return was carried in the sample and kept its rank, shifting the quantile by a whole observation                                                         | Non-finite returns are excluded from the sample before ranking, and an empty sample returns `NaN`                                                                                     |
| `expected_shortfall` | The same sample, so the tail and its bounding quantile could disagree                                                                                            | The same exclusion, because the value at risk and the tail it bounds must be taken from the same observed returns                                                                     |
| `win_rate`           | A `NaN` PnL fell into the losers' bucket and counted in the denominator, deflating the rate from 0.5 to 0.33                                                     | A missing PnL is excluded: a trade that was not recorded is not a trade that lost. A recorded breakeven trade still counts, which is the pre-existing behaviour and its existing test |

**Verified by execution.** `cargo nextest run -p nautilus-analysis --lib --features python` runs 319
tests, all passing: the 308 pre-existing tests, including every statistic's own pinned value tests,
and eleven new ones - the classification's completeness in both directions, the class vocabulary and
its obligations, the missing-value rule over the returns, PnL and benchmark categories, the
short-finite-series infinity rule, the divisor and minimum-observation case, the compensated
dispersion comparison, the cumulative kernels against a log-space recomputation over a hundred
thousand observations, the transform layout, and the order-independence of a reduction.
`cargo clippy --locked -p nautilus-analysis --all-targets --features python -- -D warnings` is clean
and `cargo fmt` is applied. `pytest tests/unit/analysis tests/unit/portfolio tests/unit/optimization
tests/integration/test_optimization.py` and `pytest tests/regression/test_regression.py` pass with
`python/tests/regression/expected` unmodified, so no published number moved: the four fixes change
behaviour only for non-finite inputs, and the compensation changes a dispersion only where the plain
sum's error was already comparable to the value.

**Not measured.** The obligations that need inputs this crate cannot construct are exercised at the
class level rather than per kernel: no test builds a `Position`, so the position-based `long_ratio`
and the position category of the PnL statistics are covered only where they return `None`, and the
benchmark-relative kernels are exercised with one synthetic benchmark pair rather than a family of
market regimes. The `normalization` class's invariance clause is exercised as bracket-scale
invariance only: this crate has no geometric normalization, so the translation-invariance clause has
no member to test. The `rolling_reduction` and `label` obligations are declared and unexercised
because no kernel occupies those classes. Whether the class assignments themselves are the right ones
is a review question, not a measurement: the assignment is recorded per kernel with its source so it
can be disagreed with in one place.

### 4.6 D6 Label and target policy framework, in tranches, with alignment

**Specification to implement** (`design 9 L6`), as an extension of the Feature, Label and Dataset
candidate in `vnpy_lessons_design.md` (D5 and D13 there), not as a competing layer.

1. A label definition carries an identity, a horizon, a direction, a threshold, a wait, an
   aggregation, a missing-data policy and an **alignment convention**. The alignment convention is
   first-class because pairing a feature at `t` with a label at `t` and pairing it with a label at
   `t+1` produce materially different datasets while looking superficially equivalent.
2. **First tranche.** A fixed-horizon forward return; future aggregates over a forward window (mean,
   standard deviation, minimum, maximum) with an explicit wait excluding the current bar; a first-hit
   threshold label over a forward window with independent positive and negative thresholds, reporting
   which threshold was breached first and zero when neither was; and dataset-level leakage validation
   against the leakage policy of D2.
3. **Second tranche, after the dataset and leakage contracts exist.** A local-extrema state machine
   under per-instrument thresholds; the symmetric conversion of one threshold into the other, recorded
   explicitly because the arithmetic is not the naive negation; and trend encodings over the interval
   between two consecutive extrema, binary, continuous and saturated.
4. Every policy exists only on the target path. A leakage test fails if a label value is read as a
   feature, and the target path is not reachable from the live strategy interface.
5. A parameter sweep is not a dataset: the dataset contract in the earlier review governs.

**Implemented in revision 10** (`python/nautilus_trader/optimization/labels.py`). The first tranche,
plus one decision the specification left open:

- `LabelDefinition` carries the outcome (`LabelKind`), the window (`horizon` in observations and
  `wait`), the missing-data policy, the alignment convention, and the fields one kind uses: the
  `aggregate` of a forward aggregate and the two thresholds of a first-hit label. A field a kind does
  not apply is refused rather than ignored, and a definition whose kind could never produce a finite
  value (a standard deviation below two observations) is refused at construction.
- The window of an anchor is the `horizon` observations after the `wait`, and the entry reference is
  the close of the `wait`-th observation, so a zero wait enters at the anchor's own close while the
  window still starts after it. That is the pairing the wait convention is pinned against.
- `AlignmentConvention` decides whether a feature row pairs with the label anchored at its own bar or
  at the next bar. The two are one row apart in the values and one bar apart in the reach, and both
  are asserted.
- `FORWARD_RETURN` is the endpoint ratio over the window; `FORWARD_AGGREGATE` reduces the window's
  per-bar returns (the mean with a compensated accumulation, the standard deviation with the sample
  divisor of the D5 dispersion kernel, the minimum and maximum of single bars); and
  `FIRST_HIT_THRESHOLD` scans the cumulative return from the entry and reports which barrier it
  reaches first, zero when neither. The scan is over closes rather than highs and lows, so one
  observation cannot breach both barriers and the first hit is total; an intrabar comparison would
  need the bar-ordering policy of D3, which this layer does not own, and is not in this tranche.
- `LabelSeries.forward_reach_ns` is measured from the produced series rather than derived from the
  definition, so it accounts for the alignment convention, the wait, a first hit that stops early,
  and a window that skips missing observations and therefore spans more time than its bar count. A
  first-hit label that hits early carries less information than one that scans its whole window, and
  the reach says so.
- `validate_leakage` refuses a `LeakagePolicy` whose effective purge before the evaluation set is
  shorter than the reach, and `leakage_shortfall_ns` returns the difference so the fix is a number
  rather than a judgement. The intervals themselves remain a study decision: the check states the
  requirement and does not choose the value, which is `design 14` question 2's answer.
- The target path is quarantined twice. No module under `trading`, `live`, `backtest`, `execution`,
  `risk` or `adapters` imports the label layer, asserted by a source test, so the target path is not
  reachable from the live strategy interface; and a `LabelSeries` offered where market data is
  expected is refused by name, which is the leakage test the acceptance asks for.
- The second tranche is not built: extrema and trend-state labels need the dataset contract of the
  earlier review, and neither it nor a stored point-in-time membership exists (section 4.11 and
  section 8 item 7).

**Not measured**: no dataset contract or panel exists in this repository, so a caller supplies the
bar series and the label definition digest does not yet join a dataset identity; section 8 item 9
records that boundary. Nothing computes a label in the live path, so no published number moved and
`python/tests/regression/expected` was not regenerated.

### 4.7 D7 Secondary implementation parity protocol

**Specification to implement** (`design 9 L7`). A document, linked from the crate that owns the
boundary, stating:

1. One implementation is the reference. For the Cython-to-pyo3 transition that is the v1 engine, which
   the existing out-of-process benchmark harness already treats as the baseline.
2. A second implementation must mirror the reference's argument order, return shape, dtype and memory
   layout, and must be validated on parity, fallback, explicit-error and layout-sensitive cases before
   it is used for anything.
3. Where a capability cannot be preserved, the second implementation refuses and the reference runs; a
   forced request for the absent implementation raises a typed error rather than degrading.
4. Benchmarks follow parity and are never the justification for the implementation.
5. The reference implementation never imports the second one, and public callers never import an
   implementation directly; they go through the owning crate.
6. Version agreement between the halves is asserted at build time, which is already the case, and the
   assertion message names both versions.
7. The protocol is not a prerequisite for any research work; it becomes actionable only when a second
   implementation exists.

**Acceptance and verification.** The document exists, is linked from the boundary crate, and its
checklist is followed if and when a second implementation appears. No acceptance condition applies to
code that does not exist; the acceptance is the presence of the protocol and its link.

**Implemented in revision 11.** The protocol is `docs/developer_guide/parity_protocol.md`, linked
from `crates/pyo3/README.md`, which owns the boundary between the two implementations, and listed in
the developer guide's contents. It states the seven rules of the specification and a nine-step
checklist for the day a second implementation appears. Every claim in it was re-derived from this
repository rather than restated:

- The reference is the v1 engine, and the claim that the out-of-process harness already treats the
  generations that way is verified in `scripts/benchmark-backtest-versions.py`, which builds one
  isolated environment per runtime (`Runtime(version="1.231.0", backend="cython")` against
  `Runtime(version="2.0.0rc6", backend="pyo3")`) and refuses to compare them unless the Python
  version and the precision mode agree.
- The build-time version agreement is `crates/core/build.rs`, which compares the version declared in
  `python/pyproject.toml` against the version the Rust build carries and fails with a message naming
  both, rather than the assertion this specification assumed was already in place; the value reaches
  the runtime as `nautilus_trader.core.NAUTILUS_VERSION`, while `python/nautilus_trader/__init__.py`
  derives `__version__` from the installed distribution metadata.
- No code acceptance condition applies, because no second implementation exists: the acceptance is
  the document and its link (section 6).

### 4.8 D8 Domain-scoped capability results

**Specification to implement** (`design 9 L8`).

1. One shape: a capability result carries availability, a code, a human-readable detail and the
   requirements or preconditions that were not met.
2. The code set is domain-scoped, not universal: order capability reuses the existing
   `OrderDeniedCode` set, and analytics, data and research each own their own closed set. A single
   universal enum would accumulate every refusal reason in the system and stop being checkable.
3. The codes follow the discipline already documented for order denial: the code is canonical, the
   detail is not, and no caller may branch on the detail.
4. Producer examples: whether a statistic can be computed from the available inputs and, if not, which
   input is missing; whether a requested catalog range is covered and, if not, where the gaps are;
   whether a requested split or study is representable and, if not, which constraint failed.
5. The probe is a pure function of its inputs, so it is cheap to call before doing work.

**Acceptance and verification.** A unit test per domain asserting the code for at least two refusal
cases, and a test asserting that no detail string is matched anywhere in the source.

**Implemented in revision 12.** The shape is `crates/core/src/capability.rs`:
`Capability { available, code, detail, requirements }`, with `is_canonical_code` and a `Display` that
renders `CODE: detail`. An available answer carries no code and an unavailable answer always carries
one, so the two cannot disagree. The Python binding is `crates/core/src/python/capability.rs`,
registered in the `nautilus_trader.core` module and exposed as `available()`, `unavailable(code,
detail)`, `requiring(requirement)` and the four properties; it validates the code and raises
`ValueError` for prose, because a Python caller passes a string while a Rust caller passes the
variant of a closed enum. The stubs were regenerated.

The code sets are per domain, and three of the four already existed:

- **Order** reuses `OrderDeniedCode`: `OrderDeniedReason::capability` reports the denial as
  unavailable, and splits the rendered message at the documented `": "` boundary so the code appears
  once rather than in both the code and the detail, which the test asserting that the capability
  renders exactly like the denial pins.
- **Analysis** reuses `MetricReason`: `MetricResult::capability` is available when a value was
  computed and otherwise carries the result's reason, so the analysis domain gains no second
  vocabulary for the same facts.
- **Data** declares its own: `coverage_capability` in `nautilus-persistence` answers a requested
  closed interval against a key's coverage, reporting `RANGE_NOT_COVERED` when no stored data covers
  the range and `RANGE_GAPS` when it is covered in part, with every missing span as a requirement. A
  known-empty interval is a gap rather than coverage, because it records that no data exists there.
- **Research** declares `ResearchCapabilityCode` (seven codes, one per constraint that failed) in
  `python/nautilus_trader/optimization/capability.py`, with three probes that answer before the work
  is done: `leakage_capability` over a label series and a leakage policy, `significance_capability`
  over the study's counts and its dependence declaration, and `split_capability` over a split
  contract and a period. The last one asks the contract rather than predicting it and carries the
  refusal it raises as the detail, which is what the rule that a detail is not canonical is for.

The rule that no caller branches on a detail is enforced by a source test over the compiled and the
Python sources, which rejects a comparison, a membership test and a prefix or substring search on a
detail. The producers and the tests avoid matching details themselves, so the scan needs no
exceptions beyond its own file.

**Not measured**: no caller consumes a capability answer yet beyond the tests, so the producers are
the mechanism rather than a decision that already depends on them; the probe and the contract it
consults both refuse the same request, so a caller can choose either; and the Python binding cannot
check that a code belongs to the domain's set, only that it is a canonical token, because the set
membership is Rust-side.

### 4.9 D9 Declarative research caching, deferred

**Specification to implement only when triggered** (`design 9 L9`). The trigger is a measured case of
the same expensive computation being repeated over an identical dataset, configuration and parameter
set. Until then the item is a recorded intent, not work.

1. The cache key is an identity from `design 8`, not a function signature: dataset digest, computation
   id, computation version, parameter digest and relevant configuration digest. The worst research bug
   in this area is a cache that returns a value computed over different data for the same arguments.
2. Caching is declared, not implied: named conditions decide whether a computation is cached, with an
   allow list, a deny list and a per-instance override.
3. Caching can be disabled globally and per computation, and disabling it must not change results.
4. An argument that cannot be hashed produces an uncached result rather than an error.
5. Invalidation is explicit, and the policy states that a cached value assumes its inputs have not
   changed.

**Acceptance and verification.** No acceptance criterion while deferred. If triggered: a test that a
cached and an uncached run agree exactly, a test that changing the dataset digest misses the cache, a
test that an unhashable argument returns a result, and a test that clearing invalidates.

**Not implemented, and its trigger is now measured rather than assumed (revision 13).** The condition
the item waits for is a measured case of the same expensive computation repeated over an identical
dataset, configuration and parameter set. That was measured rather than reasoned about:
`test_a_composed_study_runs_every_experiment_on_each_window_once` in
`python/tests/integration/test_optimization.py` runs the richest shipped composition, a walk-forward
study plus a search, through the real runner with a ledger at the runner boundary, which is where the
expensive work happens, recording the window and the experiment digest of every executed run. The
composed study executed nine runs with no repeat: four search experiments, four in-session searches
across the two windows and one out-of-sample evaluation.

The measurement carries its own control, because an empty result is worth nothing without one: the
second half of the same test composes the one pattern that does repeat, an experiment the search has
already run being validated again on the same window, and asserts that the ledger reports exactly one
repeated run. The detector works, so the empty result above is a fact about the compositions rather
than about the detector.

**What exists already is the identity half.** `ExperimentStore` writes an experiment, a result and a
report under a digest-keyed directory and can load them back, which is the cache-key model of the
specification's item 1; `Optimizer.optimize` passes a store through to write the report and never
consults one to skip an experiment, so the policy half, which the specification's items 2 to 5
describe, does not exist. The item stays deferred: it becomes work when a composition repeats an
identical run, and the test above is what would report that.

**Not measured**: no repeat was measured in any shipped composition, which is the finding rather than
a gap; what remains unmeasured is whether a *caller's* composition outside the shipped stages would
repeat one, since a study assembled by hand can ask for anything, including the redundant validation
the control exercises.

### 4.10 D10 Provider adapter, relocated

**Status.** No specification in this document. The provider adapter contract is moved to the
data-provider architecture review (`design 9 L10`, `design 13`).

The mechanism, recorded only so the move is auditable: a provider adapter implements exactly two
behaviours, fetch one instrument over a range and incrementally update one instrument, and everything
else, including storage, coverage, gap reporting and consolidation, belongs to the catalog, which
already does it better than the source library does. A market question arises in provider selection
that does not arise anywhere else in this review, which is the reason for the move rather than a
reason to answer it here.

**Acceptance and verification.** None in this document; acceptance belongs to the data-provider
review.

**Not implemented and not scheduled.**

### 4.11 Identity contracts for study, trial, dataset, universe and result

**Specification, implemented in revision 7** (`design 8`); the implementation record and its pruning
decisions are at the end of this section. These contracts are cross-cutting requirements that the
decisions above express, so they carry no decision identifier of their own.

1. **Study identity** carries the study id, the dataset identity, the feature and label definitions,
   the split contract, the leakage policy, the parameter space, the objective definition, the
   **selection rule**, the validation policy, the metric set, the study seed and the trial counts.
2. **Trial identity** carries the trial id, the study id, the parameter digest, the parameter values,
   the seed, the execution status, the objective value and the result digest. The trial level is what
   makes a trial count meaningful: knowing that 500 trials occurred is not knowing which 500.
3. **Dataset identity** carries the digest, the source version, the as-of time, the calendar identity,
   the instrument universe identity, the adjustment policy and the missing-data policy. The principle
   is that the identity identifies the information state available to the study, not merely the bytes
   consumed, because byte-identical files can differ in universe membership, calendar or adjustment
   policy.
4. **Universe identity** carries the universe digest, the membership policy id and the membership
   as-of time, and it consumes the point-in-time and membership workstream proposed in the earlier
   review rather than starting a new one.
5. **Result identity** carries the study id, the trial id, the dataset identity, the computation
   identity (code version, schema version, kernel version, numerical backend and parameter digest),
   the assumption policy and the metric results, with a result digest.
6. The invariant, restated: a research result must be reproducible from its study identity, its
   dataset identity, its validation contract, its computation identity, its parameter identity and its
   assumption policy.
7. The field lists are a proposal to be pruned, not a schema to be filled in. Pruning before results
   exist is cheap; backfilling after is not.
8. The identities compose from digests that already exist where possible: the experiment digest and
   the canonical result digest are present today (rows 10 and 11), the dataset digest belongs to the
   catalog, and the policy and numerical kernel identities are new. The name `kernel_version` is
   already used for the operating system (row 27), so the numerical kernel version needs a
   distinguished name.

**Acceptance and verification.** A result cannot be produced without a study identity, asserted by
construction rather than convention; two runs of the same study over the same data produce the same
result digest; a changed kernel version changes the implementation identity while the study id stays
stable; and the trial identity is sufficient to re-run an individual trial.

| Item                                                                                                                                  | Where                                                                                                       |
| ------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| The universe identity: the membership digest, the membership policy and the membership as-of time                                     | `python/nautilus_trader/optimization/identity.py:146` `UniverseIdentity`                                    |
| The dataset identity: the seven declared fields, so the identity names the information state rather than the bytes                    | `identity.py:204` `DatasetIdentity`                                                                         |
| The computation identity, with a distinguished numeric kernel version                                                                 | `identity.py:277` `ComputationIdentity`, `:78` `COMPUTATION_SCHEMA`                                         |
| The study identity and its digest over the declared study                                                                             | `identity.py:349` `StudyIdentity`, `:424` `study_id`                                                        |
| The trial provenance record, kept out of the study identity                                                                           | `identity.py:437` `TrialProvenance`                                                                         |
| The trial identity and its digest, which is stable across a re-run of the same trial                                                  | `identity.py:490` `TrialIdentity`, `:558` `trial_id`                                                        |
| The result record, which requires a study and a trial of that study                                                                   | `identity.py:577` `ResearchResult`, `:643` `result_digest`, `:81` `RESULT_SCHEMA`                           |
| The two closed vocabularies the identities use                                                                                        | `identity.py:127` `SelectionRule`, `:136` `ExecutionStatus`                                                 |
| The bridges: a declared objective definition from the Rust terms, a split-contract digest, and a trial identity from an optimizer run | `identity.py:655` `objective_definition_from_terms`, `:95` `split_contract_digest`, `:697` `trial_identity` |
| The public surface                                                                                                                    | `python/nautilus_trader/optimization/__init__.py`, the twelve identity names and functions in `__all__`     |
| The documentation                                                                                                                     | `docs/concepts/optimization.md` section "Identity contracts"                                                |

Six decisions, all deliberate, and the first three are the pruning the specification asked for:

1. **The counts are pruned out of the study identity.** The specification lists `trial_count` and
   `failed_trial_count` in the study block. They describe how a study *ended*, not what it declared:
   a transient failure would change the identity of the study that suffered it, and a set of trials
   that is "the same study" would stop being comparable. `TrialProvenance` carries them for the
   correction that needs them, and the equality of a study identity across a re-run is asserted.
2. **`feature_definition` and `label_definition` are pruned, and `validation_policy` is represented
   by the split-contract and leakage digests.** No label or feature framework exists in this
   repository (verified-state row 24), so a field would be a placeholder; the validation policy *is*
   the split contract plus the leakage policy, and both carry or compose a digest already.
3. **`kernel_version` is named `numeric_kernel_version`.** The name is already the operating system
   kernel's (row 27), and a result computed before and after a numerical change is not the same
   result, so the numerical one is distinguished rather than renamed over the top of it.
4. **The dataset fields are declared, not derived.** The catalog has no dataset digest, the Parquet
   backend rejects every as-of value but the latest, and no stored membership history exists
   (verified-state row 27), so `DatasetIdentity` and `UniverseIdentity` record what their caller
   declares. A universe identity is therefore auditable but not yet enforced: the stored-membership
   workstream of the earlier review (`vnpy_lessons_design.md` D5 and D13, which this contract was
   specified to consume rather than compete with) is what would make membership recoverable rather
   than re-evaluated, and it is not implemented.
5. **The contracts are not wired into the command-line document.** The configuration file declares no
   dataset, and its data configuration comes from a caller-supplied factory, so this layer cannot
   honestly derive a dataset identity; writing one anyway would be inventing an unverifiable digest.
   The contracts are the API a notebook or a study runner uses, and D4 is their first consumer.
6. **The split-contract digest is composed rather than declared.** `SplitContract` carries no digest,
   so `split_contract_digest` builds one from the declared layout and the leakage policy's own
   digest, instead of the identity accepting an unverified string from its caller.

**Verified by execution.** `pytest tests/unit/optimization` runs 68 tests, all passing, of which
seven are new: a result requiring a study identity and refusing a trial from another study; a
dataset identity distinguishing the membership as-of time, the membership policy, the calendar and
the adjustment policy, each of which also moves the study identity; a study identity stable across a
re-run while a numeric-kernel change moves the computation and result digests without moving the
study; a trial identity whose recorded values reconstruct the experiment digest, refusing a digest
that does not describe its values and refusing a completed trial with no result digest; trial
provenance refusing incoherent counts; the objective definition carried in a declared form and
sensitive to a weight, a term and the selection rule; and the split contract's digest and its effect
on the study identity. `pytest tests/integration/test_optimization.py` runs 10 tests, all passing,
of which one is new and is the acceptance case rather than a tautology: two real sweeps of the same
study over the same catalog produce equal trial ids and equal result digests, and a changed numeric
kernel version then changes the implementation and result digests while the study identity stays
stable. All nine declared regression scenarios pass with `python/tests/regression/expected`
unmodified. `ruff check` and `ruff format --check` are clean on the changed files.

**Not measured.** No consumer records an identity automatically yet: the optimizer, the runner and
the command-line document were left unchanged, so nothing in the delivered pipeline emits a
`ResearchResult` on its own. The contracts and their bridges are exercised by the tests above, and
the wiring decision is recorded as decision 5 with its reason.

## 5. Defects

### 5.1 No defect found in this repository

The checks below were run to decide whether the authorisation to fix defects was triggered. Each is a
search or a read, not an execution; no test was run, and this is stated rather than implied.

| Check                                                                                 | Outcome                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| ------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Statistics registry versus the vectorbt model: is a declarative metric layer missing? | No: a trait, a registry and 34 statistics exist (rows 1 to 4)                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| Is a metric metadata mechanism missing?                                               | Yes, and it is a specification, not a defect: nothing is wrong today, a result is merely less informative than it could be (D1). It is now implemented (section 4.1) as an additive contract, so the name-keyed result shape and every pinned expectation are unchanged                                                                                                                                                                                                                                                       |
| Is walk-forward validation missing?                                                   | No: stages exist (rows 7 and 8)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| Is a split contract missing?                                                          | It was, and it is now implemented (section 4.2)                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| Is leakage control missing?                                                           | It was, and it was the one finding with a correctness character: a walk-forward study could not purge or embargo, so an in-sample segment could overlap the label horizon of its own out-of-sample segment. It was recorded as D2 with an explicit policy rather than as a defect, because no existing test or documented guarantee was violated. The mechanism now exists (section 4.2) and the stages apply a declared zero policy by default, so a study that needs exclusions declares them and one that does not says so |
| Is trial provenance recorded?                                                         | It was not (row 25). The study, trial and provenance contracts now exist (section 4.11), and nothing in the pipeline emits them yet, so what remains is D4, which is the consumer rather than the contract                                                                                                                                                                                                                                                                                                                    |
| Is dataset or universe identity recorded?                                             | It was not (row 27). The contracts now declare the fields (section 4.11), but the repository still has no dataset digest, no point-in-time read and no stored membership history, so an identity is a declaration its caller makes rather than a guarantee the repository enforces; the stored-membership workstream of the earlier review is what would change that                                                                                                                                                          |
| Is a multiple-testing correction missing?                                             | Yes, and it is a specification (D4)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| Is there a duplicate implementation to keep in parity?                                | No: one implementation per kernel (row 15)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| Is the extension optional or switchable in a way that could silently change numerics? | No: it is mandatory and unconditional (row 16)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| Is the version relationship between the halves unverified?                            | No: asserted at build time (row 18)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| Is a capability reason for analytics or data availability present?                    | Present for order denial (row 19), absent for analytics and data queries, which is D8                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| Is bar-level ambiguity undocumented or optimistic?                                    | Documented and configurable (row 22), but the policy has no identity in the result (row 26), which is D3. The policy now has an identity in the research layer's result (section 4.3), named as the behaviour the engine already implements rather than changed; whether the default should instead be pessimistic remains an owner decision                                                                                                                                                                                  |
| Is dataset coverage and gap reporting missing?                                        | No: present and mature (row 23)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |

The one item closest to a defect is the absence of any leakage control in walk-forward validation. It
is recorded as a specification because it does not violate a stated guarantee, but if the owner
considers an unpurged walk-forward result to be an incorrect result rather than a less rigorous one,
then the item moves from D2 to a defect and its fix becomes a bug fix with a regression test. That
judgement is the owner's.

### 5.2 Observations from the source library, as warnings

These are properties of vectorbt, recorded so that the same trap is not imported with the mechanism.

| Observation                                                                  | Evidence                                                                                                                                                                                                                                  | Why it matters here                                                                                                                                                |
| ---------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Three numerical defects and one bounds defect were fixed in a single release | Release notes for v1.1.1: rolling standard deviation stability in both engines, deflated Sharpe kurtosis convention, expanding mean and standard deviation when the minimum period exceeds the length, out-of-bounds reads on empty input | Two implementations multiply exactly this class of risk, which is why D5 exists as a classified test obligation rather than a parity suite                         |
| A correction metric was wrong for a long time                                | The deflated Sharpe ratio used excess kurtosis and did not ignore missing returns until v1.1.1, and the notes state that results change                                                                                                   | A multiple-testing correction that is itself wrong is worse than none, which is why D4 specifies the statistical contract in full and pins it in our own terms     |
| A runtime engine switch changes numerics silently                            | `vectorbt/_engine.py` resolves the engine per call from an argument, a global setting and availability; randomised kernels are pinned to one engine to preserve random streams                                                            | This is the mechanism D7 rejects: results must not depend on which engine happened to be installed                                                                 |
| Cache keys are value hashes                                                  | Indicator caches key on a hash of the parameter values                                                                                                                                                                                    | Hash collisions and dataset mismatch are the two silent correctness hazards in a research cache, which is why the D9 identity model starts from the dataset digest |
| Configuration can lose arguments when reconstructed                          | The `Configured` documentation warns that attributes outside `writeable_attrs` and arguments derived from global defaults are not preserved                                                                                               | Our typed configuration with explicit defaults does not have this failure mode, and it should stay that way                                                        |
| Some settings are read at call time and others only at construction time     | The settings module documents the distinction                                                                                                                                                                                             | A temporal footgun in a global settings tree; our configuration is passed explicitly                                                                               |
| The community edition withholds the features that matter most for validation | Purging and embargoing, portfolio optimization and parallel execution are paid features                                                                                                                                                   | Do not read the community edition as a complete reference for validation design                                                                                    |
| The licence is not open in the OSI sense                                     | Apache-2.0 with Commons Clause, "Fair Code" badge                                                                                                                                                                                         | No code transfer (design 1.2 and 7.3)                                                                                                                              |
| Test volume exceeds library volume by a wide margin                          | 36,259 test lines against roughly 20,000 library lines                                                                                                                                                                                    | Evidence for the cost of two implementations, and for the value the project places on the parity suite                                                             |
| An ambiguous generator configuration is rejected                             | The signal generator refuses a zero wait on both the entry and exit side because same-bar entry and exit would be unorderable                                                                                                             | Adopted as a pattern in D3: ambiguity is an error, not a convention                                                                                                |

## 6. Acceptance criteria and verification, by item

This table is the detailed verification plan. The minimum acceptance contract per decision, which is
what makes the design independently reviewable, is stated in `design 13.1`.

| Item                                | Acceptance                                                                                                                                                                            | Verification                                                                                                                                              | Status                                                                                                                                                                                                                                       |
| ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| D1 Metric identity                  | Identity, title, units, tags and direction declared; a four-state status with domain-owned reason codes                                                                               | One test covering computed, unavailable, invalid and unregistered in a single result; `invalid` versus `unavailable` on one metric; closed sets           | Implemented (4.1); 308 analysis crate tests and 365 Python tests pass, including seven contract tests; clippy clean; the vocabulary's membership is a proposal the owner may replace                                                         |
| D2 Split contract                   | Any series length and any set-length mixture; a leakage relation expressing purge before, purge after, embargo after and a label overlap rule; a zero interval that must be justified | Contract tests, a label-overlap test, a refusal test, a zero-interval justification test, three-exclusion expressibility, window equality after migration | Implemented (4.2); 31 contract tests, 23 configuration tests, 9 integration tests and `optimization_golden` pass                                                                                                                             |
| D3 Ambiguity policy                 | Every assumption documented, ambiguous configurations rejected, an identity recorded in the result, the policy distinct from execution simulation, the default named by the owner     | Assumption-to-test mapping, rejection test, identity in the result, policy-version distinguishability, a test asserting the named default                 | Implemented (4.3); 61 optimization tests pass, including seven for the policy; the regression scenarios pass with expectations unmodified; the default is named as the existing behaviour, so replacing it is an owner decision              |
| D4 Multiple-testing reporting       | Study and trial identity recorded; the full statistical contract specified before testing; reported and never a gate                                                                  | Nine acceptance cases from section 4.4, including dependent trials and the annualisation rejection                                                        | Implemented (4.4); 21 unit tests pass and the formula agrees with an independent recomputation to 1e-12; the two defaults are declared and replaceable, and the value is never a gate                                                        |
| D5 Numerical stability              | Each kernel classified by class, and the obligation for its class met; running variance compared against an independent method                                                        | The classification exists and each kernel has its class obligation                                                                                        | Implemented (4.5); 319 analysis tests pass, the dispersion kernel agrees with a compensated computation to 1.6e-16 where the plain summation was wrong by 0.71, and four missing-value defects in the statistics were found and fixed        |
| D6 Labels                           | The first tranche exists only on the target path, the definition includes the alignment convention, a leakage test fails if a label value is read as a feature                        | Hand-computed asymmetric case, wait case, both alignment pairings, leakage test; per-instrument test in the second tranche                                | Implemented (4.6); 19 label tests pass, including the hand-computed asymmetric first-hit case, the cumulative-barrier case, both alignment pairings, the zero-wait case, the leakage refusal with its shortfall and the two quarantine tests |
| D7 Parity protocol                  | A written protocol exists and is linked from the owning crate; not a research prerequisite                                                                                            | The document and its link; the checklist applies only if a second implementation appears                                                                  | Implemented (4.7); the document exists, is linked from `crates/pyo3/README.md` and listed in the developer guide, and its checklist applies only when a second implementation appears                                                        |
| D8 Capability results               | One shared shape with domain-scoped closed code sets; nothing branches on detail text                                                                                                 | Two refusal cases per domain, and a source test against detail matching                                                                                   | Implemented (4.8); the shared shape is `nautilus_trader.core.Capability`, the four domain sets are closed and owned, 13 Rust and 9 Python capability tests pass, and a source test rejects any match on a detail                             |
| D9 Cache                            | No acceptance criterion while deferred; if triggered, the key is the identity model and cached and uncached runs agree                                                                | Deferred                                                                                                                                                  | Deferred, and the trigger is measured as unmet (4.9); a ledger at the runner boundary reports no repeated run in the shipped compositions, with a control that proves the ledger detects one                                                 |
| D10 Provider adapter                | No acceptance criterion in this document                                                                                                                                              | Belongs to the data-provider architecture review                                                                                                          | Not implemented                                                                                                                                                                                                                              |
| D11 Schema ownership                | No acceptance criterion: the decision is to change nothing                                                                                                                            | Not applicable                                                                                                                                            | Not implemented                                                                                                                                                                                                                              |
| Identity contracts (no decision id) | A result cannot be produced without a study identity; the trial identity is sufficient to re-run a trial; field lists are pruned, not filled in                                       | Construction-level assertion, digest stability, kernel-version change reflected in the implementation identity, trial re-run                              | Implemented (4.11); 68 optimization unit tests and 10 integration tests pass, including the two-sweep digest equality; the field lists were pruned and each pruning is justified in 4.11                                                     |

## 7. Explicit non-goals

1. **No port of any vectorbt subsystem.** No accessor layer, no factory-generated indicator classes,
   no broadcast parameter grid, no plotting, no widgets.
2. **No dependency on the package**, and no vendored, transliterated or mechanically derived code or
   documentation (design 1.2 and 7.3).
3. **No change to the execution model.** Research-layer mechanisms only; the engine, the exchange
   simulation and the order state machine are untouched, and the research layer records execution
   assumptions without owning execution semantics.
4. **No behavior change without an owner decision**, including the ambiguity default.
5. **No provider integration here.** The provider adapter contract is moved to the data-provider
   architecture review, including the market question that goes with it.
6. **No decision authority for the research layer.** No metric, label or diagnostic gates an order or
   a live control (design 7.6).
7. **No second implementation invented to justify D7.** The protocol is a document until a second
   implementation exists.
8. **No speculative caching.** D9 is not started until a measured case of repeated expensive
   computation exists.
9. **No total ordering imposed where none exists.** The phases are a work order, not a dependency
   chain, and the contract graph in `design 11` is the only dependency statement.

## 8. Outstanding measurements and open items

1. Which ambiguity policy is the default remains an owner decision, and whether a pessimistic default
   changes published results is unmeasured. Answering it requires running the existing backtest
   scenarios under both policies and comparing, which was not done in this review because the review
   runs no tests. What is now implemented (section 4.3) is the contract and a *named* default equal to
   the behaviour the engine already implements, so no published number moved; selecting a different
   default is the owner's call and is the one change this item can still make.
2. The leakage rule's mechanism now exists (section 4.2) and its values remain a study decision: what
   observations are forbidden from training because their feature and label information overlaps the
   evaluation information. No interval length is proposed here because a guessed interval is worse
   than an explicit zero, which is why the stages declare a zero with its reason and why a zero
   without one is refused. The dataset-level validation now exists (section 4.6): the label layer
   measures its forward reach from the produced series and refuses a policy whose effective purge is
   shorter than it, reporting the shortfall, so what remains open is the value of every interval
   rather than whether the requirement can be checked.
3. The minimum observation count and the minimum trial count at which a correction says anything are
   still unmeasured and are a literature question rather than a repository question. The implementation
   (section 4.4) now declares them as contract parameters with defaults of twenty contributing periods
   and ten trial estimates, records them with every result and refuses to compute below them, so the
   question is now which declared value to use rather than whether one exists. Replacing them is the
   owner's call and touches no mechanism.
4. Trial dependence in a typical sweep is still unmeasured, and whether the chosen correction needs an
   effective trial count is decided for the initial statistic rather than measured: the implementation
   (section 4.4) requires the declaration, refuses an undeclared dependent study, and lets the
   effective count replace the nominal one in the expected maximum. Whether a clustering rule should
   *derive* the effective count from the parameter space is unmeasured and is the second tranche's
   question.
5. The units, tags and direction vocabulary now exists and is closed (section 4.1), proposed by the
   implementation rather than by an owner decision, because a closed set is mechanically replaceable
   and the mechanism does not depend on which members it holds. The owner can still replace the
   membership without touching the contract.
6. The kernel classification now exists (section 4.5): every built-in statistic and the three shared
   kernels are assigned a class, a test compares the table against the registry in both directions so
   a new statistic cannot enter unclassified, and the obligations are exercised per class. Two classes
   (`rolling_reduction` and `label`) are declared with no member yet, and the position categories are
   covered only where they return `None`, because this crate cannot construct a `Position` in a unit
   test; whether the class assignments are the right ones remains a review question.
7. What constitutes dataset identity is the largest open question, and it gates the reproducibility of
   every study rather than any single decision. The contract now declares the seven fields (section
   4.11) and its caller makes the declaration; which of them a catalog can *derive* remains open,
   because the repository has no dataset digest, no point-in-time read and no stored membership.
8. What constitutes trial identity decides whether a trial can be re-run at all, and it gates D4. The
   contract now carries the parameters and their digest, the seed, the execution status, the objective
   value and the result digest (section 4.11), so a trial can be reconstructed; D4 is what counts over
   the trial level.
9. The identity field lists were pruned rather than filled in (section 4.11): the counts moved to a
   provenance record so a transient failure cannot move a study's identity, and the feature and label
   definitions were dropped because nothing in the repository computes either. Every field kept is
   justified in the implementation record. The label definition now exists with its own digest
   (section 4.6), so it can join a dataset identity; it does not yet, because the dataset contract of
   the earlier review is not implemented and no panel is assembled. A feature definition still does
   not exist.
10. The vectorbt clone was deleted, so any cited line must be re-derived from the pinned revision. The
    revision is recorded in section 2 and the re-derivation is a single shallow clone.
11. The caching trigger is measured as unmet (section 4.9) for the compositions the shipped stages
    build, and remains unmeasured for a composition a caller assembles by hand: a study can ask for
    the same experiment on the same window twice, and the control in the trigger test does exactly
    that. Whether a cache should serve a caller that asks twice is the policy question D9 defers,
    and the measurement gives it a starting point rather than an answer.
