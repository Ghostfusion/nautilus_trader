# VectorBT lesson review: implementation record

Companion to [`vectorbt_lessons_design.md`](vectorbt_lessons_design.md) and to
[`vnpy_lessons_implementation.md`](vnpy_lessons_implementation.md). Section references of the form
`design N` point at the design document.

## 1. Status and scope

### 1.1 Nothing is implemented

This document records a probe, a comparison and a set of specified decisions. **No production code
was changed.** The authorising instruction permits code changes only for defects; no defect was found
in this repository (section 5.1), so the defect path was not exercised. Every accepted item is marked
**Not implemented** in section 6, and the mechanisms in section 4 are specifications, not descriptions
of existing code.

No test of this repository was executed as part of this review: the verification performed is source
inspection, and section 3 states what was inspected and where. Claims that would require execution to
establish are not made.

### 1.2 Revision history

| Version | Change                                                                                                                 |
| ------- | ---------------------------------------------------------------------------------------------------------------------- |
| 1       | Initial record from the vectorbt probe: licence finding, capability inventory, eleven decisions, implementation record |

The document is deliberately shaped like the VeighNa record so the two can be read side by side, and
so that a later reader can see which candidate learnings the two probes share (the label and dataset
layer) and which are peculiar to this one (multiple-testing correction, the split contract, the
dual-implementation protocol).

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
| Rust crate         | `vectorbt-rust` 1.1.1, `rust/Cargo.toml`, pyo3 0.29, numpy 0.29, ndarray 0.17                          |
| Licence            | Apache-2.0 with Commons Clause (`LICENSE.md`, "Fair Code" badge; GitHub API reports `NOASSERTION`)     |
| Size               | 118 Python files, 105,199 lines including tests; 8 Rust files; 11 Markdown pages under `docs/`         |
| Tests              | 16 files, 36,259 lines, of which `test_portfolio.py` 10,831 and `test_engine.py` 2,543                 |
| Stars, forks       | 9,242 and 1,183                                                                                        |
| Created, last push | 2017-11-14, 2026-09-26                                                                                 |
| Releases           | v1.1.1 2026-09-26, v1.1.0 2026-07-05, v1.0.0 2026-04-22 (introduced the Rust engine), v0.28.5, v0.28.4 |
| Runtime deps       | numpy>=2.4.6, pandas>=3.0.3, numba>=0.66, scipy, matplotlib, plotly, anywidget, scikit-learn           |

Coverage and method:

| Area                      | How it was read                                                                                           |
| ------------------------- | --------------------------------------------------------------------------------------------------------- |
| Engine dispatch           | `vectorbt/_engine.py`, `rust/README.md`, `tests/test_engine.py` in full                                   |
| Validation splits         | `vectorbt/generic/splitters.py` in full                                                                   |
| Portfolio simulation      | `vectorbt/portfolio/base.py` signatures, `nb.py` stop paths and trade reconstruction, `enums.py` dtypes   |
| Statistics                | `vectorbt/generic/stats_builder.py`, `returns/accessors.py`, `returns/metrics.py`, `generic/drawdowns.py` |
| Signals and indicators    | `signals/factory.py`, `indicators/factory.py`, `nb.py` look-ahead controls                                |
| Labels                    | `vectorbt/labels/` in full                                                                                |
| Data                      | `vectorbt/data/base.py`, `custom.py`, `updater.py`                                                        |
| Configuration and caching | `vectorbt/utils/config.py`, `utils/decorators.py`, `_settings.py`                                         |
| Licence and packaging     | `LICENSE.md`, `pyproject.toml`, `.github/workflows/` listing, release notes                               |
| Not read                  | `vectorbt/plotting`, notebooks, `apps/`, `benchmarks/`, VectorBT PRO (closed source)                      |

The clone was deleted after this record was written. Any claim below can be re-derived from the
pinned revision; the numbers in section 3 can be re-derived from the working tree.

## 3. Verified state of this repository

Every row was verified by reading the file at the cited line in the working tree at commit
`d71348e30a`. Rows marked "absent" record a search that returned no match, and the search scope is
stated.

| #   | Fact                                                                                         | Evidence                                                                                                                                                                                                                                                                                                                   |
| --- | -------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | Statistics are a Rust trait with one struct per statistic                                    | `crates/analysis/src/statistic.rs:36` `pub trait PortfolioStatistic: Debug {`, `:40` `fn name(&self) -> String;`, `:47` `fn calculate_from_returns(&self, returns: &Returns) -> Option<Self::Item>`, `:56` `calculate_from_realized_pnls`, `:68` `calculate_from_positions`, `:79` `calculate_from_returns_with_benchmark` |
| 2   | 34 statistics are declared, one module each                                                  | `crates/analysis/src/statistics/mod.rs:18-51`, 34 `pub mod` lines                                                                                                                                                                                                                                                          |
| 3   | There is a name-keyed registry with runtime registration and deregistration                  | `crates/analysis/src/analyzer.rs:38` `pub type Statistic = Arc<dyn PortfolioStatistic<Item = f64> + Send + Sync>;`, `:59` `pub statistics: AHashMap<String, Statistic>,`, `:123` `pub fn register_statistic(&mut self, statistic: Statistic)`, `:128` `pub fn deregister_statistic`                                        |
| 4   | Twenty statistics are registered by default                                                  | `crates/analysis/src/analyzer.rs:79-98`, twenty `register_statistic` calls                                                                                                                                                                                                                                                 |
| 5   | There is no metric metadata object                                                           | No match for `metric_config`, `MetricConfig`, `MetricRegistry`, `MetricSpec`, `stats_builder` anywhere in the repository                                                                                                                                                                                                   |
| 6   | Statistics return `Option` and a skipped statistic leaves no trace in the result             | `crates/analysis/src/statistic.rs:47`, `:56`, `:68` all return `Option<Self::Item>`; the analyzer inserts only `Some` values                                                                                                                                                                                               |
| 7   | Walk-forward validation exists as explicit stages                                            | `python/nautilus_trader/optimization/stages.py:51` `class WalkForwardWindow`, `:74` `def walk_forward_windows(`, `:128` `class TrainStage`, `:159` `class OptimizeStage`, `:217` `class ValidateStage`, `:263` `class OutOfSampleStage`, `:351` `class WalkForwardStage`                                                   |
| 8   | The stage vocabulary is closed and typed                                                     | `python/nautilus_trader/optimization/config.py:94` `STAGE_KINDS = ("optimize", "train", "validate", "out_of_sample", "walk_forward")`                                                                                                                                                                                      |
| 9   | One search strategy is implemented, and searching is pure enumeration                        | `python/nautilus_trader/optimization/search.py:37` `class SearchStrategy(Protocol)`, `:62` `class GridSearch`                                                                                                                                                                                                              |
| 10  | A run is identified by a canonical digest and recorded with its parameter mapping            | `python/nautilus_trader/optimization/space.py:47` `canonical_json`, `:74` `digest_of`, `:97` `class Experiment`, `python/nautilus_trader/optimization/runner.py:61` `class CanonicalRun`, `:85` `class FailedExperiment`                                                                                                   |
| 11  | A search reports results and failures together                                               | `python/nautilus_trader/optimization/report.py:120` `class SearchReport`                                                                                                                                                                                                                                                   |
| 12  | There is no purge or embargo rule                                                            | No match for `purge` or `embargo` as a validation concept in `python/nautilus_trader`, `crates`, or `docs`; the only hits are cache-instrument purges in adapters                                                                                                                                                          |
| 13  | There is no splitter contract and no cross-validation splitter                               | No match for `splitter`, `cross_val`, `cv_split` in `python/nautilus_trader`, `crates`, or `docs`                                                                                                                                                                                                                          |
| 14  | There is no multiple-testing correction                                                      | No match for `deflated`, `PBO`, `multiple testing`, or `bootstrap` in `python/nautilus_trader`, `crates/analysis`, `crates/backtest`, or `docs`                                                                                                                                                                            |
| 15  | Every hot path has exactly one implementation and it is Rust                                 | `python/nautilus_trader/indicators/__init__.py:26` `from nautilus_trader._libnautilus.indicators import *`, `:29` `fixup_module_names(globals(), __name__)`; `crates/indicators/src/python/mod.rs:40-41` `#[pymodule]` / `pub fn indicators`                                                                               |
| 16  | The compiled extension is mandatory                                                          | `python/nautilus_trader/__init__.py:26` unconditional star import of `nautilus_trader._libnautilus`; no `find_spec`, no engine setting, no pure-Python fallback for any hot path                                                                                                                                           |
| 17  | The Rust core owns every domain schema                                                       | `crates/model/src/events/order/filled.rs:40-47` `pyo3::pyclass(module = "nautilus_trader.model", from_py_object)` and `pyo3_stub_gen::derive::gen_stub_pyclass` above `pub struct OrderFilled {`; `crates/model/src/python/mod.rs:302` `m.add_class::<crate::events::OrderFilled>()?;`                                     |
| 18  | The version relationship is asserted at build time, not at run time                          | `crates/core/build.rs:30` `let nautilus_version = "2.0.0rc6";`; `python/nautilus_trader/__init__.py:29-34` derives `__version__` from installed distribution metadata; `Cargo.toml:55` `license = "LGPL-3.0-only"`                                                                                                         |
| 19  | Order denial already reports a stable machine-readable code                                  | `crates/model/src/events/order/denied_reason.rs:64-68` `#[strum_discriminants(name(OrderDeniedCode), derive(Display, AsRefStr, EnumIter, EnumString), strum(serialize_all = "SCREAMING_SNAKE_CASE"))]`; `:57` `Only the leading code is canonical. Consumers must not recover classification or control flow`              |
| 20  | The denial event itself carries only the rendered string                                     | `crates/model/src/events/order/denied.rs:61` `pub reason: Ustr,` within `pub struct OrderDenied`                                                                                                                                                                                                                           |
| 21  | Configuration is typed, rejects unknown keys, and collects every violation with a field path | `crates/common/src/cache/config.rs:43` `#[serde(default, deny_unknown_fields)]`; `crates/backtest/src/config.rs:624-625` `pub fn validate(&self) -> ConfigResult<()>` with `let mut errors = ConfigErrorCollector::new();`, pushing `ConfigError::empty_field` and `ConfigError::range`                                    |
| 22  | Bar-level ambiguity is resolved by a documented, configurable ordering policy                | `crates/backtest/src/config.rs:364` `pub bar_execution: bool`, `:367` `pub bar_adaptive_high_low_ordering: bool`, `:525-530` documents the fixed order Open, High, Low, Close and the adaptive heuristic                                                                                                                   |
| 23  | Catalog coverage and gap reporting exist and are mature                                      | `crates/persistence/src/backend/parquet/catalog/coverage.rs:77` `pub fn get_missing_intervals_for_request(`, `:263` `get_intervals`; `crates/persistence/src/backend/parquet/catalog/mod.rs:244` `pub struct ParquetDataCatalog {`                                                                                         |
| 24  | Nothing computes supervised labels, forward returns, extrema or trade MFE and MAE            | No match for `triple_barrier`, `forward_return`, `zigzag`, `local_extrema`, `mfe`, or `mae` in `crates/model`, `crates/data`, `crates/trading`, or `python/nautilus_trader`; the only Label concept is the proposal in `vnpy_lessons_design.md`                                                                            |

Two rows carry a correction that the review had to make against its own expectation. Row 1 shows
that a statistics registry already exists, so the vectorbt lesson is narrower than "add a registry"
and is stated as metadata plus availability reasons (design D5). Row 7 shows that walk-forward
validation already exists here, so the vectorbt lesson is the split *contract* and the leakage
interval, not the concept of a split (design D3). Both corrections came from reading the code rather
than the documentation, and both are recorded because the earlier probe produced a similar
correction for the universe and portfolio construction workstreams.

## 4. Mechanism specifications

Each specification is written against our own requirements and cites the source mechanism only for
provenance. None of them may be implemented by copying vectorbt code (design 7.3).

### 4.1 D1 Capability probe as a typed value

**Specification to implement.**

1. A capability answer is a value with a `code` drawn from a closed enum, a `detail` string for
   humans, and a list of required conversions or preconditions. It replaces any boolean or sentence
   returned by a "can this be done" query in the analysis, data and optimization layers.
2. The codes follow the discipline already documented for order denial: the code is canonical, the
   detail is not, and no caller may branch on the detail. `OrderDeniedReason` and `OrderDeniedCode`
   are the model, not a new invention.
3. Producer examples: whether a statistic can be computed from the available inputs and, if not,
   which input is missing; whether a requested catalog range is covered and, if not, where the gaps
   are; whether a requested split is representable and, if not, which length constraint failed.
4. The probe is a pure function of its inputs, so it is cheap to call before doing work.

**Acceptance and verification.** A unit test per producer asserting the code for at least two
refusal cases, and a test asserting that no detail string is matched anywhere in the source.

**Not implemented.**

### 4.2 D2 Dual-implementation protocol

**Specification to implement.** A document, linked from the crate that owns the boundary, stating:

1. One implementation is the reference. For the Cython-to-pyo3 transition that is the v1 engine,
   which the existing out-of-process benchmark harness already treats as the baseline.
2. A second implementation must mirror the reference's argument order, return shape, dtype and memory
   layout, and must be validated on parity, fallback, explicit-error and layout-sensitive cases
   before it is used for anything.
3. Where a capability cannot be preserved, the second implementation refuses and the reference runs;
   a forced request for the absent implementation raises a typed error rather than degrading.
4. Benchmarks follow parity and are never the justification for the implementation.
5. The reference implementation never imports the second one, and public callers never import an
   implementation directly; they go through the owning crate.
6. Version agreement between the halves is asserted at build time, which is already the case, and the
   assertion message names both versions.

**Acceptance and verification.** The document exists, is linked from the boundary crate, and its
checklist is followed if and when a second implementation appears. No acceptance condition applies
to code that does not exist; the acceptance is the presence of the protocol and its link.

**Not implemented** (no such document exists today).

### 4.3 D3 Split contract and leakage interval

**Specification to implement.**

1. A split is produced by a contract that yields, per split, a tuple of index arrays, one per set.
   The contract is duck-typed: anything with a `split(X, **kwargs)` method satisfies it.
2. Set lengths accept both fractions of the window and absolute counts; the lengths that are given
   determine all but one set, and the remaining set absorbs the remainder.
3. A direction flag decides which set absorbs the remainder: forward means the remainder joins the
   last set, reversed means it joins the first.
4. A minimum length filters windows before selection, and a requested number of splits selects that
   many evenly spaced windows rather than the first ones, so a coarse study still spans the sample.
5. An empty set and a request that cannot be satisfied raise, naming the constraint.
6. A leakage interval is part of the contract: a purge interval removed before each test set and an
   embargo removed after it, both expressed in bars and both defaulting to zero only if the caller
   says so explicitly. This is the item neither project implements and the reason the specification
   exists.
7. The existing optimization stages consume the contract rather than owning window generation, so
   walk-forward and split-based validation share one implementation of the bounds.
8. Statistics are computed per split by the caller, as they are today; the contract returns indices
   and does not aggregate.

**Acceptance and verification.** A contract test over several lengths and set-length combinations
including the fractional and absolute forms and the reversed direction; a test asserting that no
index appears in a test set and in its own in-sample set within the leakage interval; a test that an
unsatisfiable request raises with the constraint named; and a regression scenario asserting that
existing walk-forward stages produce identical windows after migrating onto the contract.

**Not implemented.**

### 4.4 D4 Deflated Sharpe ratio

**Specification to implement.**

1. A statistic computing the deflated Sharpe ratio from the estimated per-period Sharpe ratio, the
   variance of the Sharpe ratio across the trials, the number of trials, the backtest horizon in
   periods, the skew and the kurtosis, using the non-excess kurtosis convention.
2. Every input is per-period. Annualised figures are rejected at the boundary rather than divided
   silently, because the correction is not scale invariant in the way a reader assumes.
3. Missing returns are excluded rather than zero-filled, and the horizon is the count of periods that
   contributed.
4. The optimization report records the trial count that produced a run's score and exposes the
   correction alongside the score. The value is reported, never a gate.
5. Below the minimum trial count of design question 3 the statistic is unavailable with a code,
   consistent with D5, rather than computed from too little evidence.

**Acceptance and verification.** A hand-computed case pinned numerically against an independent
implementation of the published formula; a test that an annualised input is rejected; a test that a
run's report carries the trial count; and a regression scenario where a search over a known trial
count produces the expected correction.

**Not implemented.**

### 4.5 D5 Metric metadata and availability reasons

**Specification to implement.**

1. A statistic declares, beside its identity: a display title, units, and tags. Identity is stable
   and machine-facing; the title is presentational and may change without breaking a caller.
2. Parameters that currently appear inside a display name, such as the annualisation period, move
   into the metadata, and the title renders from them at presentation time.
3. A statistic that is registered but cannot be computed reports a code naming what is missing, for
   example a returns series, a benchmark, a frequency, or a non-empty position list. It is not
   silently dropped, and it is never reported as zero.
4. The result distinguishes three states per statistic: computed, unavailable with a reason, and not
   registered. This is the distinction that does not exist today.
5. Tags are a closed set, so a consumer can filter deterministically.

**Acceptance and verification.** A result containing at least one computed, one unavailable and one
unregistered statistic, asserted in a single test; a test that a title renders from metadata
parameters; a test that the units and tags sets are closed.

**Not implemented.**

### 4.6 D6 Ambiguity policy for bar-level fills

**Specification to implement.**

1. Every fill assumption that a bar-only replay cannot determine is documented in one place, in the
   user-facing documentation and not only in the source: the ordering of the bar's prices, the price
   used when a stop triggers, the priority between simultaneous triggers, and the treatment of a gap
   through a threshold.
2. Defaults are pessimistic. Where the outcome is unknowable, the resolution that is worse for the
   simulated strategy is selected, and the choice is stated.
3. Ambiguous configurations are rejected: a setting whose meaning depends on an unspecifiable
   ordering is a configuration error rather than a silent convention.
4. The policy that produced a result is recorded in the result, so two results can be compared
   without reading the source.
5. Nothing in this item changes behaviour without an explicit owner decision, because changing the
   default changes published numbers (design question 1).

**Acceptance and verification.** A test that each documented assumption has a matching assertion in
the simulation tests; a test that an ambiguous configuration is rejected with a named constraint; a
test that the policy appears in the result metadata.

**Not implemented.**

### 4.7 D7 Label policies

**Specification to implement.** As an extension of the Feature, Label and Dataset candidate in
`vnpy_lessons_design.md` (D5 and D13 there), not as a competing layer.

1. A first-hit breakout label over a forward window with independent positive and negative
   thresholds, reporting which threshold was breached first and zero when neither was, with a wait
   offset excluding the current bar.
2. A local-extrema state machine producing peaks and troughs under per-instrument thresholds, with
   the symmetric conversion of one threshold into the other recorded explicitly because the
   arithmetic is not the naive negation.
3. Trend encodings over the interval between two consecutive extrema: binary, continuous, and
   saturated, with the interval labelled by an event that occurs later by construction.
4. Future-aggregate primitives (mean, standard deviation, minimum, maximum) over a forward window,
   expressed with the explicit wait.
5. Every policy above exists only on the target path. A leakage test fails if a label value is read
   as a feature, and the target path is not reachable from the live strategy interface.

**Acceptance and verification.** The first-hit label pinned against a hand-computed case with
asymmetric thresholds; a leakage test; a test that per-instrument thresholds are applied per
instrument.

**Not implemented.**

### 4.8 D8 Research cache policy

**Specification to implement.**

1. A cache policy is declared, not implied: named conditions decide whether a computation is cached,
   with an allow list, a deny list, and a per-instance override.
2. Caching can be disabled globally and per computation, and disabling it must not change results.
3. An argument that cannot be hashed produces an uncached result rather than an error.
4. Invalidation is explicit: a cached value is cleared by name or by invalidating the object, and the
   policy states that a cached value assumes its inputs have not changed.
5. Cache keys are derived from the input identity, not from a hash of a tuple of values.

**Acceptance and verification.** A test that a cached and an uncached run agree exactly; a test that
an unhashable argument returns a result; a test that clearing invalidates.

**Not implemented.**

### 4.9 D9 Provider adapter contract

**Specification to implement.**

1. A provider adapter implements exactly two behaviours: fetch one instrument over a range, and
   incrementally update one instrument.
2. Everything else is owned elsewhere: storage, coverage, gap reporting and consolidation belong to
   the catalog, which already does this and better than the source library does.
3. Per-instrument arguments may differ within one call, so a multi-instrument fetch does not require
   one call per instrument or a lowest-common-denominator signature.
4. Only a United-States provider may be considered for a concrete adapter, and none is scheduled by
   this review. Non-United-States exchanges, their symbols and their conventions are out of scope.

**Acceptance and verification.** An interface test that an adapter is usable through the contract
with storage untouched; a test that two instruments with different arguments are fetched in one call;
and, if a concrete adapter is ever added, a coverage test asserting that the catalog reports the
gap it did not fill.

**Not implemented.**

### 4.10 D10 Numerical-stability test category

**Specification to implement.** A test category, applied to every research kernel, containing at
least:

1. running-variance and running-mean stability over a long series, compared against a compensated or
   two-pass computation;
2. empty input, single-element input, and input shorter than the window;
3. a minimum period greater than the series length, whose result must be NaN and not a partial
   aggregate;
4. NaN propagation, and agreement between implementations on where NaNs land;
5. layout cases: non-contiguous, transposed, single-column and single-row inputs;
6. a documented convention for the sample and population divisor, since a one-off difference here is
   invisible in a parity test but changes every later number.

**Acceptance and verification.** The category exists, at least the six cases above are present for
each research kernel that reduces a series, and the variance case is compared against an independent
method rather than against the kernel's own twin.

**Not implemented.**

## 5. Defects

### 5.1 No defect found in this repository

The checks below were run to decide whether the authorisation to fix defects was triggered. Each is
a search or a read, not an execution; no test was run, and this is stated rather than implied.

| Check                                                                                 | Outcome                                                                                                                                                                                                                                                                                                                                                       |
| ------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Statistics registry versus the vectorbt model: is a declarative metric layer missing? | No: a trait, a registry and 34 statistics exist (rows 1-4)                                                                                                                                                                                                                                                                                                    |
| Is a metric metadata mechanism missing?                                               | Yes, and it is a specification, not a defect: nothing is wrong today, a result is merely less informative than it could be (D5)                                                                                                                                                                                                                               |
| Is walk-forward validation missing?                                                   | No: stages exist (rows 7-8)                                                                                                                                                                                                                                                                                                                                   |
| Is a split contract missing?                                                          | Yes, and it is a specification (D3)                                                                                                                                                                                                                                                                                                                           |
| Is leakage control missing?                                                           | Yes, and this is the one finding with a correctness character: a walk-forward study conducted today cannot purge or embargo, so an in-sample window may overlap the label horizon of its own out-of-sample window. It is recorded as D3 with an explicit interval rather than as a defect, because no existing test or documented guarantee is violated today |
| Is a multiple-testing correction missing?                                             | Yes, and it is a specification (D4)                                                                                                                                                                                                                                                                                                                           |
| Is there a duplicate implementation to keep in parity?                                | No: one implementation per kernel (row 15)                                                                                                                                                                                                                                                                                                                    |
| Is the extension optional or switchable in a way that could silently change numerics? | No: it is mandatory and unconditional (row 16)                                                                                                                                                                                                                                                                                                                |
| Is the version relationship between the halves unverified?                            | No: asserted at build time (row 18)                                                                                                                                                                                                                                                                                                                           |
| Is a capability reason for analytics or data availability present?                    | Present for order denial (row 19), absent for analytics and data queries, which is D1                                                                                                                                                                                                                                                                         |
| Is bar-level ambiguity undocumented or optimistic?                                    | Documented and configurable (row 22); whether the default should be pessimistic is an owner decision under D6, not a defect                                                                                                                                                                                                                                   |
| Is dataset coverage and gap reporting missing?                                        | No: present and mature (row 23)                                                                                                                                                                                                                                                                                                                               |

The one item that is closest to a defect is the absence of any leakage control in walk-forward
validation. It is recorded as a specification because it does not violate a stated guarantee, but if
the owner considers an unpurged walk-forward result to be an incorrect result rather than a less
rigorous one, then the item moves from D3 to a defect and its fix becomes a bug fix with a regression
test. That judgement is the owner's.

### 5.2 Observations from the source library, as warnings

These are properties of vectorbt, recorded so that the same trap is not imported with the mechanism.

| Observation                                                                  | Evidence                                                                                                                                                                                                                                  | Why it matters here                                                                                                         |
| ---------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| Three numerical defects and one bounds defect were fixed in a single release | Release notes for v1.1.1: rolling standard deviation stability in both engines, deflated Sharpe kurtosis convention, expanding mean and standard deviation when the minimum period exceeds the length, out-of-bounds reads on empty input | Two implementations multiply exactly this class of risk; it is why D10 exists as a test category rather than a parity suite |
| A correction metric was wrong for a long time                                | The deflated Sharpe ratio used excess kurtosis and did not ignore missing returns until v1.1.1, and the notes state that results change                                                                                                   | A multiple-testing correction that is itself wrong is worse than none, which is why D4 adopts the corrected form explicitly |
| A runtime engine switch changes numerics silently                            | `vectorbt/_engine.py` resolves the engine per call from an argument, a global setting and availability; randomised kernels are pinned to one engine to preserve random streams                                                            | This is the mechanism D2 rejects: results must not depend on which engine happened to be installed                          |
| Cache keys are value hashes                                                  | Indicator caches key on a hash of the parameter values                                                                                                                                                                                    | Hash collisions are a silent correctness hazard in a cache that decides whether to recompute                                |
| Configuration can lose arguments when reconstructed                          | The `Configured` documentation warns that attributes outside `writeable_attrs` and arguments derived from global defaults are not preserved                                                                                               | Our typed configuration with explicit defaults does not have this failure mode, and it should stay that way                 |
| Some settings are read at call time and others only at construction time     | The settings module documents the distinction                                                                                                                                                                                             | A temporal footgun in a global settings tree; our configuration is passed explicitly                                        |
| The community edition withholds the features that matter most for validation | Purging and embargoing, portfolio optimization and parallel execution are paid features                                                                                                                                                   | Do not read the community edition as a complete reference for validation design                                             |
| The licence is not open in the OSI sense                                     | Apache-2.0 with Commons Clause, "Fair Code" badge                                                                                                                                                                                         | No code transfer (design 7.3)                                                                                               |
| Test volume exceeds library volume by a wide margin                          | 36,259 test lines against roughly 20,000 library lines                                                                                                                                                                                    | Evidence for the cost of two implementations, and for the value the project places on the parity suite                      |
| An ambiguous generator configuration is rejected                             | The signal generator refuses a zero wait on both the entry and exit side because same-bar entry and exit would be unorderable                                                                                                             | Adopted as a pattern in D6: ambiguity is an error, not a convention                                                         |

## 6. Acceptance criteria and verification, by item

This table is the detailed verification plan. The minimum acceptance contract per decision, which is
what makes the design independently reviewable, is stated in `design 12.1`.

| Item                   | Acceptance                                                                                    | Verification                                                                              | Status          |
| ---------------------- | --------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------- | --------------- |
| D1 Capability probe    | A probe returns a typed value with a closed code, and nothing branches on the detail text     | Two refusal cases per producer, and a source test against detail matching                 | Not implemented |
| D2 Dual implementation | A written protocol exists and is linked from the owning crate                                 | The document and its link; the checklist applies only if a second implementation appears  | Not implemented |
| D3 Split contract      | Any series length and any set-length mixture, with a leakage interval that is respected       | Contract tests, a leakage test, a refusal test, and window equality after migration       | Not implemented |
| D4 Deflated Sharpe     | Per-period inputs only, the trial count recorded with the result, the value pinned            | Hand-computed case, annualised-input rejection, report carries the trial count            | Not implemented |
| D5 Metric metadata     | Identity, title, units and tags declared; unavailable statistics carry a reason               | One test covering computed, unavailable and unregistered in a single result               | Not implemented |
| D6 Ambiguity policy    | Every bar-level assumption documented, pessimistic by default, and recorded in the result     | Assumption-to-test mapping, ambiguous-configuration rejection, policy present in metadata | Not implemented |
| D7 Labels              | Label policies exist only on the target path, with the first-hit label pinned                 | Hand-computed asymmetric case, leakage test, per-instrument threshold test                | Not implemented |
| D8 Cache policy        | Declarative, overridable, disableable, degrading on unhashable arguments                      | Cached and uncached runs agree; unhashable argument returns; clearing invalidates         | Not implemented |
| D9 Provider adapter    | Two behaviours implementable, storage untouched, per-instrument arguments, United States only | Interface test, mixed-argument fetch test, coverage test if an adapter is added           | Not implemented |
| D10 Stability tests    | Six cases present per reducing kernel, variance compared against an independent method        | The category exists and each kernel has the cases                                         | Not implemented |
| D11 Schema ownership   | No acceptance criterion: the decision is to change nothing                                    | Not applicable                                                                            | Not implemented |

## 7. Explicit non-goals

1. **No port of any vectorbt subsystem.** No accessor layer, no factory-generated indicator classes,
   no broadcast parameter grid, no plotting, no widgets.
2. **No dependency on the package**, and no vendored or transliterated code (design 7.3).
3. **No change to the execution model.** Research-layer mechanisms only; the engine, the exchange
   simulation and the order state machine are untouched.
4. **No behavior change without an owner decision**, including the pessimistic bar default.
5. **No new provider integration.** The adapter contract is specified; the provider list is not.
6. **No gate introduced by any metric.** A metric that gates is a risk rule and belongs with the
   risk caps, as recorded in the earlier review.
7. **No second implementation invented to justify D2.** The protocol is a document until a second
   implementation exists.

## 8. Outstanding measurements and open items

1. Whether the pessimistic bar default changes published results is unmeasured. Answering it requires
   running the existing backtest scenarios under both policies and comparing, which was not done in
   this review because the review runs no tests. This gates D6.
2. The correct leakage interval length is unmeasured and depends on the label horizon and bar spacing.
   No value is proposed here because a guessed interval is worse than an explicit zero. This gates D3.
3. The minimum trial count at which the deflated Sharpe ratio is meaningful is unmeasured here and
   is a literature question rather than a repository question. This gates D4.
4. The units and tags vocabulary does not exist yet, including whether it is closed. This gates D5.
5. Whether a cache policy is needed before the research layer exists is unanswered; the
   specification is cheap but speculative. This gates D8.
6. Whether the research layer belongs in this repository at all remains open from the earlier review,
   and every item except D2 and D10 is contingent on it.
7. The vectorbt clone was deleted, so any cited line must be re-derived from the pinned revision.
   The revision is recorded in section 2 and the re-derivation is a single shallow clone.
