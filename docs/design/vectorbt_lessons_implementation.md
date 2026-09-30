# VectorBT lesson review: implementation record

Companion to [`vectorbt_lessons_design.md`](vectorbt_lessons_design.md) and to
[`vnpy_lessons_implementation.md`](vnpy_lessons_implementation.md). Section references of the form
`design N` point at the design document, and decision identifiers (`D1` to `D11`) are the revised
numbering introduced by revision 2 of this record.

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

| Version | Change                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1       | Initial record from the vectorbt probe: the licence finding, the capability inventory, eleven decisions and this implementation record                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| 2       | Applied design-review feedback. The decision set was renumbered and reclassified by layer; the deflated Sharpe ratio was rescoped from the requirement to the first statistic satisfying a trial-provenance requirement; the split leakage interval became a policy whose values are optional and whose concept is mandatory; the ambiguity resolution became a versioned policy with an identity recorded in the result; metrics gained a direction; the label work was split into tranches; caching was deferred with an identity model; the provider adapter was moved out of the document; a first-class study identity was added; the licence rule was restated as an engineering boundary with a provenance chain; the dependency graph was replaced by one that shows information flow, with the phases restated as work order; and the placement questions became preconditions rather than open questions |

The document is deliberately shaped like the VeighNa record so the two can be read side by side, and
so that a later reader can see which candidate learnings the two probes share (the label and dataset
layer) and which are peculiar to this one (multiple-testing-aware reporting, the split contract, the
parity protocol).

### 1.3 Renumbering map

Revision 2 renumbered the decisions. The old identifiers appear in the earlier commits and must not be
confused with the current ones.

| Revision 1 | Revision 2 | Subject                                            |
| ---------- | ---------- | -------------------------------------------------- |
| D5         | D1         | Metric identity, metadata and availability reasons |
| D3         | D2         | Reusable split contract with a leakage policy      |
| D6         | D3         | Explicit ambiguity policy with result provenance   |
| D4         | D4         | Multiple-testing-aware research reporting          |
| D10        | D5         | Numerical-stability testing                        |
| D7         | D6         | Label and target policy framework                  |
| D2         | D7         | Secondary implementation parity protocol           |
| D1         | D8         | Capability-result codes                            |
| D8         | D9         | Declarative research caching, deferred             |
| D9         | D10        | Provider adapter, moved out of the document        |
| D11        | D11        | Rust schema ownership, no change                   |

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

| #   | Fact                                                                                         | Evidence                                                                                                                                                                                                                                                                                                                                               |
| --- | -------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1   | Statistics are a Rust trait with one struct per statistic                                    | `crates/analysis/src/statistic.rs:36` `pub trait PortfolioStatistic: Debug {`, `:40` `fn name(&self) -> String;`, `:47` `fn calculate_from_returns(&self, returns: &Returns) -> Option<Self::Item>`, `:56` `calculate_from_realized_pnls`, `:68` `calculate_from_positions`, `:79` `calculate_from_returns_with_benchmark`                             |
| 2   | 34 statistics are declared, one module each                                                  | `crates/analysis/src/statistics/mod.rs:18-51`, 34 `pub mod` lines                                                                                                                                                                                                                                                                                      |
| 3   | There is a name-keyed registry with runtime registration and deregistration                  | `crates/analysis/src/analyzer.rs:38` `pub type Statistic = Arc<dyn PortfolioStatistic<Item = f64> + Send + Sync>;`, `:59` `pub statistics: AHashMap<String, Statistic>,`, `:123` `pub fn register_statistic(&mut self, statistic: Statistic)`, `:128` `pub fn deregister_statistic`                                                                    |
| 4   | Twenty statistics are registered by default                                                  | `crates/analysis/src/analyzer.rs:79-98`, twenty `register_statistic` calls                                                                                                                                                                                                                                                                             |
| 5   | There is no metric metadata object                                                           | No match for `metric_config`, `MetricConfig`, `MetricRegistry`, `MetricSpec`, `stats_builder` anywhere in the repository                                                                                                                                                                                                                               |
| 6   | Statistics return `Option` and a skipped statistic leaves no trace in the result             | `crates/analysis/src/statistic.rs:47`, `:56`, `:68` all return `Option<Self::Item>`; the analyzer inserts only `Some` values                                                                                                                                                                                                                           |
| 7   | Walk-forward validation exists as explicit stages                                            | `python/nautilus_trader/optimization/stages.py:51` `class WalkForwardWindow`, `:74` `def walk_forward_windows(`, `:128` `class TrainStage`, `:159` `class OptimizeStage`, `:217` `class ValidateStage`, `:263` `class OutOfSampleStage`, `:351` `class WalkForwardStage`                                                                               |
| 8   | The stage vocabulary is closed and typed                                                     | `python/nautilus_trader/optimization/config.py:94` `STAGE_KINDS = ("optimize", "train", "validate", "out_of_sample", "walk_forward")`                                                                                                                                                                                                                  |
| 9   | One search strategy is implemented, and searching is pure enumeration                        | `python/nautilus_trader/optimization/search.py:37` `class SearchStrategy(Protocol)`, `:62` `class GridSearch`                                                                                                                                                                                                                                          |
| 10  | A run is identified by a canonical digest and recorded with its parameter mapping            | `python/nautilus_trader/optimization/space.py:47` `canonical_json`, `:74` `digest_of`, `:97` `class Experiment`, `runner.py:61` `class CanonicalRun`, `:85` `class FailedExperiment`                                                                                                                                                                   |
| 11  | A search reports results and failures together                                               | `python/nautilus_trader/optimization/report.py:120` `class SearchReport`                                                                                                                                                                                                                                                                               |
| 12  | There is no purge or embargo rule                                                            | No match for `purge` or `embargo` as a validation concept in `python/nautilus_trader`, `crates`, or `docs`; the only hits are cache-instrument purges in adapters                                                                                                                                                                                      |
| 13  | There is no splitter contract and no cross-validation splitter                               | No match for `splitter`, `cross_val`, `cv_split` in `python/nautilus_trader`, `crates`, or `docs`                                                                                                                                                                                                                                                      |
| 14  | There is no multiple-testing correction                                                      | No match for `deflated`, `PBO`, `multiple testing`, or `bootstrap` in `python/nautilus_trader`, `crates/analysis`, `crates/backtest`, or `docs`                                                                                                                                                                                                        |
| 15  | Every hot path has exactly one implementation and it is Rust                                 | `python/nautilus_trader/indicators/__init__.py:26` `from nautilus_trader._libnautilus.indicators import *`, `:29` `fixup_module_names(globals(), __name__)`; `crates/indicators/src/python/mod.rs:40-41` `#[pymodule]` / `pub fn indicators`                                                                                                           |
| 16  | The compiled extension is mandatory                                                          | `python/nautilus_trader/__init__.py:26` unconditional star import of `nautilus_trader._libnautilus`; no `find_spec`, no engine setting, no pure-Python fallback for any hot path                                                                                                                                                                       |
| 17  | The Rust core owns every domain schema                                                       | `crates/model/src/events/order/filled.rs:40-47` `pyo3::pyclass(module = "nautilus_trader.model", from_py_object)` and `pyo3_stub_gen::derive::gen_stub_pyclass` above `pub struct OrderFilled {`; `crates/model/src/python/mod.rs:302` `m.add_class::<crate::events::OrderFilled>()?;`                                                                 |
| 18  | The version relationship is asserted at build time, not at run time                          | `crates/core/build.rs:30` `let nautilus_version = "2.0.0rc6";`; `python/nautilus_trader/__init__.py:29-34` derives `__version__` from installed distribution metadata; `Cargo.toml:55` `license = "LGPL-3.0-only"`                                                                                                                                     |
| 19  | Order denial already reports a stable machine-readable code                                  | `crates/model/src/events/order/denied_reason.rs:64-68` `#[strum_discriminants(name(OrderDeniedCode), derive(Display, AsRefStr, EnumIter, EnumString), strum(serialize_all = "SCREAMING_SNAKE_CASE"))]`; `:57` `Only the leading code is canonical. Consumers must not recover classification or control flow`                                          |
| 20  | The denial event itself carries only the rendered string                                     | `crates/model/src/events/order/denied.rs:61` `pub reason: Ustr,` within `pub struct OrderDenied`                                                                                                                                                                                                                                                       |
| 21  | Configuration is typed, rejects unknown keys, and collects every violation with a field path | `crates/common/src/cache/config.rs:43` `#[serde(default, deny_unknown_fields)]`; `crates/backtest/src/config.rs:624-625` `pub fn validate(&self) -> ConfigResult<()>` with `let mut errors = ConfigErrorCollector::new();`, pushing `ConfigError::empty_field` and `ConfigError::range`                                                                |
| 22  | Bar-level ambiguity is resolved by a documented, configurable ordering policy                | `crates/backtest/src/config.rs:364` `pub bar_execution: bool`, `:367` `pub bar_adaptive_high_low_ordering: bool`, `:525-530` documents the fixed order Open, High, Low, Close and the adaptive heuristic                                                                                                                                               |
| 23  | Catalog coverage and gap reporting exist and are mature                                      | `crates/persistence/src/backend/parquet/catalog/coverage.rs:77` `pub fn get_missing_intervals_for_request(`, `:263` `get_intervals`; `crates/persistence/src/backend/parquet/catalog/mod.rs:244` `pub struct ParquetDataCatalog {`                                                                                                                     |
| 24  | Nothing computes supervised labels, forward returns, extrema or trade MFE and MAE            | No match for `triple_barrier`, `forward_return`, `zigzag`, `local_extrema`, `mfe`, or `mae` in `crates/model`, `crates/data`, `crates/trading`, or `python/nautilus_trader`; the only Label concept is the proposal in `vnpy_lessons_design.md`                                                                                                        |
| 25  | A study identity does not exist; run identity is partial                                     | Rows 10 and 11 record a canonical digest, a run record and a report; no match for `study_id`, `trial_count`, `search_space_digest` or `validation_scheme_digest` in `python/nautilus_trader`, and no match for `seed` in `python/nautilus_trader/optimization`, so a run records no seed. Those identifiers appear only in this review's own documents |
| 26  | The result does not record which bar-ordering policy produced it                             | `crates/backtest/src/result.rs:105-114` records metrics and an elapsed time; no `ambiguity_policy`, `policy_id`, or ordering field is present                                                                                                                                                                                                          |

Three rows corrected an expectation held before reading the code. Row 1 shows that a statistics
registry already exists, so the lesson is metadata and availability reasons rather than a registry
(design D1). Row 7 shows that walk-forward validation already exists, so the lesson is the split
*contract* and its leakage policy rather than the concept of a split (design D2). Row 25 shows that
trial provenance is partial: a digest and a report exist, a study identity does not, which is why the
requirement in L4 is a record rather than a single statistic. The earlier probe produced a similar
correction for the universe and portfolio construction workstreams, and all of them are recorded
because the corrections are the evidence that the comparison was done against source.

## 4. Mechanism specifications

Each specification is written against our own requirements and cites the source mechanism only for
provenance. None of them may be implemented by copying vectorbt code (design 1.2 and 7.3). The
mechanism names are given as `L` identifiers from `design 8` and decision identifiers from
`design 12`.

### 4.1 D1 Metric identity, metadata and availability reasons

**Specification to implement** (`design 8 L1`).

1. A statistic declares, beside its identity: a display title, units, tags and a direction drawn from
   a closed set (`higher_is_better`, `lower_is_better`, `target`, `informational`). Identity is stable
   and machine-facing; the title is presentational and may change without breaking a caller.
2. Parameters that currently appear inside a display name, such as the annualisation period, move
   into the metadata, and the title renders from them at presentation time.
3. A statistic that is registered but cannot be computed reports a status and a reason code naming
   what is missing, for example a returns series, a benchmark, a frequency or a non-empty position
   list. It is not silently dropped, and it is never reported as zero.
4. The result distinguishes three states per statistic: computed, unavailable with a reason, and not
   registered. This is the distinction that does not exist today.
5. Direction is declarative and consumed by reporting and optimization; no optimization logic lives
   inside a statistic.

**Acceptance and verification.** A result containing at least one computed, one unavailable and one
unregistered statistic, asserted in a single test; a test that a title renders from metadata
parameters; a test that the units, tags and direction sets are closed.

**Not implemented.**

### 4.2 D2 Reusable split contract with a leakage policy

**Specification to implement** (`design 8 L2`).

1. A split is produced by a contract that yields, per split, a tuple of index arrays, one per set. The
   contract is duck-typed: anything with a `split(X, **kwargs)` method satisfies it.
2. Set lengths accept both fractions of the window and absolute counts; the lengths that are given
   determine all but one set, and the remaining set absorbs the remainder.
3. A direction flag decides which set absorbs the remainder: forward means the remainder joins the
   last set, reversed means it joins the first.
4. A minimum length filters windows before selection, and a requested number of splits selects that
   many evenly spaced windows rather than the first ones, so a coarse study still spans the sample.
5. An empty set and a request that cannot be satisfied raise, naming the constraint.
6. A leakage policy is part of the contract, expressed as a purge interval removed before each test
   set and an embargo removed after it, both in bars. The *concept* is mandatory: a splitter with no
   notion of leakage is the defect. The *values* are not: a zero purge and a zero embargo are
   permitted for a study with no label overlap, provided the study records the justification. A zero
   interval by omission and a zero interval by decision must be distinguishable.
7. The existing optimization stages consume the contract rather than owning window generation, so
   walk-forward and split-based validation share one implementation of the bounds.
8. Statistics are computed per split by the caller, as they are today; the contract returns indices
   and does not aggregate.

**Acceptance and verification.** A contract test over several lengths and set-length combinations
including the fractional and absolute forms and the reversed direction; a test asserting that no index
appears in a test set and in its own in-sample set within the leakage interval; a test that an
unsatisfiable request raises with the constraint named; a test that a zero interval without a
justification is refused; and a regression scenario asserting that existing walk-forward stages
produce identical windows after migrating onto the contract.

**Not implemented.**

### 4.3 D3 Explicit ambiguity policy with result provenance

**Specification to implement** (`design 8 L3`).

1. An ambiguity policy is a versioned value with an identity: the policy id, the trigger precedence,
   the intrabar ordering, the gap handling, the simultaneous-event handling and a version.
2. Every fill assumption that a bar-only replay cannot determine is documented in one place, in the
   user-facing documentation and not only in the source: the ordering of the bar's prices, the price
   used when a stop triggers, the priority between simultaneous triggers, and the treatment of a gap
   through a threshold.
3. Defaults are pessimistic. Where the outcome is unknowable, the resolution that is worse for the
   simulated strategy is selected, and the choice is stated.
4. Ambiguous configurations are rejected: a setting whose meaning depends on an unspecifiable
   ordering is a configuration error rather than a silent convention.
5. A result records the ambiguity policy identity that produced it, not the word "pessimistic", so a
   policy change does not silently rewrite the interpretation of historical results.
6. vectorbt's specific rules are not adopted as our fill rules: they belong to a bar simulator whose
   model is not ours.
7. Nothing in this item changes behaviour without an explicit owner decision, because changing the
   default changes published numbers (`design 13` question 1).

**Acceptance and verification.** A test that each documented assumption has a matching assertion in
the simulation tests; a test that an ambiguous configuration is rejected with a named constraint; a
test that the policy identity appears in the result metadata; and a test that two results produced
under different policy versions are distinguishable.

**Not implemented.**

### 4.4 D4 Multiple-testing-aware research reporting

**Specification to implement** (`design 8 L4`).

1. A study record carries the trial provenance: study id, dataset digest, search-space digest,
   parameter count, trial count, failed trial count, validation-scheme digest, objective definition,
   selection rule and seed. The requirement is the record; a correction is only as good as the count
   and the space it was drawn from.
2. The initial statistic is the deflated Sharpe ratio, computed from the estimated per-period Sharpe
   ratio, the variance of the Sharpe ratio across the trials, the number of trials, the backtest
   horizon in periods, the skew and the non-excess kurtosis.
3. Every input is per-period. Annualised figures are rejected at the boundary rather than divided
   silently, because the correction is not scale invariant in the way a reader assumes.
4. Missing returns are excluded rather than zero-filled, and the horizon is the count of periods that
   contributed.
5. The mathematical definition is pinned in our own terms before implementation, including the
   convention for the kurtosis and the treatment of missing returns. No release of another project is
   used as the definition.
6. The value is reported, never a gate. Below the minimum trial count of `design 13` question 3 the
   statistic is unavailable with a reason code, consistent with D1.
7. The record is forward compatible: adding further diagnostics, such as an overfitting probability or
   a bootstrap, changes no earlier part of the contract.

**Acceptance and verification.** Each of: a hand-computed deterministic case; zero, one and many trial
cases; a missing-return case; a non-excess-kurtosis case; a large trial-count case; an
insufficient-input case producing an unavailable status with a reason; and, where applicable, a
monotonicity check that more trials under the null do not raise the corrected value. The definition is
pinned in the specification before the first test is written.

**Not implemented.**

### 4.5 D5 Numerical-stability test category

**Specification to implement** (`design 8 L5`). A test category, applied to every research kernel
that reduces a series, containing at least:

1. running-variance and running-mean stability over a long series, compared against a compensated or
   two-pass computation rather than against the kernel's own twin;
2. empty input, single-element input, and input shorter than the window;
3. a minimum period greater than the series length, whose result must be NaN and not a partial
   aggregate;
4. NaN propagation, and agreement between implementations on where NaNs land;
5. overflow and underflow;
6. very large magnitudes and very small magnitudes;
7. catastrophic cancellation, for example a variance of nearly equal values;
8. monotonicity where it is mathematically required;
9. invariance under constant translation and scaling where applicable;
10. layout cases: non-contiguous, transposed, single-column and single-row inputs;
11. a documented divisor convention, sample or population, since a one-off difference here is
    invisible in a parity test but changes every later number.

**Acceptance and verification.** The category exists, at least the cases above that apply are present
for each reducing kernel, and the variance case is compared against an independent method. Parity
alone is explicitly not the standard.

**Not implemented.**

### 4.6 D6 Label and target policy framework, in tranches

**Specification to implement** (`design 8 L6`), as an extension of the Feature, Label and Dataset
candidate in `vnpy_lessons_design.md` (D5 and D13 there), not as a competing layer.

1. **First tranche.** A fixed-horizon forward return; future aggregates over a forward window (mean,
   standard deviation, minimum, maximum) with an explicit wait offset excluding the current bar; a
   first-hit threshold label over a forward window with independent positive and negative thresholds,
   reporting which threshold was breached first and zero when neither was; and dataset-level leakage
   validation.
2. **Second tranche, after the dataset and leakage contracts exist.** A local-extrema state machine
   producing peaks and troughs under per-instrument thresholds; the symmetric conversion of one
   threshold into the other, recorded explicitly because the arithmetic is not the naive negation;
   and trend encodings over the interval between two consecutive extrema, binary, continuous and
   saturated, with the interval labelled by an event that occurs later by construction.
3. Every policy exists only on the target path. A leakage test fails if a label value is read as a
   feature, and the target path is not reachable from the live strategy interface.
4. A parameter sweep is not a dataset: the dataset contract in the earlier review governs.

**Acceptance and verification.** The first-hit label pinned against a hand-computed case with
asymmetric thresholds; the wait convention pinned against a case where a zero wait would read the
current bar; a leakage test; and, for the second tranche, a test that per-instrument thresholds are
applied per instrument.

**Not implemented.**

### 4.7 D7 Secondary implementation parity protocol

**Specification to implement** (`design 8 L7`). A document, linked from the crate that owns the
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

**Acceptance and verification.** The document exists, is linked from the boundary crate, and its
checklist is followed if and when a second implementation appears. No acceptance condition applies to
code that does not exist; the acceptance is the presence of the protocol and its link.

**Not implemented** (no such document exists today).

### 4.8 D8 Capability-result codes

**Specification to implement** (`design 8 L8`).

1. A capability answer is a value with a `code` drawn from a closed enum, a `detail` string for
   humans, and a list of required preconditions or conversions. It replaces any boolean or sentence
   returned by a "can this be done" query in the analysis and data layers.
2. The codes follow the discipline already documented for order denial: the code is canonical, the
   detail is not, and no caller may branch on the detail. `OrderDeniedReason` and `OrderDeniedCode`
   are the model, not a new invention.
3. Producer examples: whether a statistic can be computed from the available inputs and, if not, which
   input is missing; whether a requested catalog range is covered and, if not, where the gaps are;
   whether a requested split is representable and, if not, which length constraint failed.
4. The probe is a pure function of its inputs, so it is cheap to call before doing work.

**Acceptance and verification.** A unit test per producer asserting the code for at least two refusal
cases, and a test asserting that no detail string is matched anywhere in the source.

**Not implemented.**

### 4.9 D9 Declarative research caching, deferred

**Specification to implement only when triggered** (`design 8 L9`). The trigger is a measured case of
the same expensive computation being repeated over an identical dataset, configuration and parameter
set. Until then the item is a recorded intent, not work.

1. The cache key is an identity, not a function signature: dataset digest, computation id,
   computation version, parameter digest and relevant configuration digest. The worst research bug in
   this area is a cache that returns a value computed over different data for the same arguments.
2. Caching is declared, not implied: named conditions decide whether a computation is cached, with an
   allow list, a deny list and a per-instance override.
3. Caching can be disabled globally and per computation, and disabling it must not change results.
4. An argument that cannot be hashed produces an uncached result rather than an error.
5. Invalidation is explicit, and the policy states that a cached value assumes its inputs have not
   changed.

**Acceptance and verification.** No acceptance criterion while deferred. If triggered: a test that a
cached and an uncached run agree exactly, a test that changing the dataset digest misses the cache, a
test that an unhashable argument returns a result, and a test that clearing invalidates.

**Not implemented and not scheduled.**

### 4.10 D10 Provider adapter, relocated

**Status.** No specification in this document. The provider adapter contract is moved to the
data-provider architecture review (`design 8 L10`, `design 12`).

The mechanism, recorded only so the move is auditable: a provider adapter implements exactly two
behaviours, fetch one instrument over a range and incrementally update one instrument, and everything
else, including storage, coverage, gap reporting and consolidation, belongs to the catalog, which
already does it better than the source library does. A market question arises in provider selection
that does not arise anywhere else in this review, which is the reason for the move rather than a
reason to answer it here.

**Acceptance and verification.** None in this document; acceptance belongs to the data-provider
review.

**Not implemented and not scheduled.**

### 4.11 Research study identity (a required concept, with no decision identifier)

**Specification to implement** (`design 7.6`). This concept was added during review rather than
adopted from the source library, so it carries no decision identifier and no provenance label beyond
the comparison that exposed the need.

1. A study identity exists as a first-class value: study id, dataset digest, split contract, label
   definition, feature definition, parameter space, objective, trial count, validation policy,
   ambiguity policy id, metric set, random seed and result digest.
2. A result carries its implementation identity beside it: study id, dataset digest, code version,
   schema version, kernel version, numerical backend, parameter digest and result digest.
3. The invariant, restated from `design 7.6`: a research result must be reproducible from its study
   identity, its dataset identity, its validation contract, its computation identity, its parameter
   identity and its assumption policy.
4. The field lists are a proposal to be pruned, not a schema to be filled in. Pruning before results
   exist is cheap; backfilling after is not.
5. The identity is composed from digests that already exist where possible: the experiment digest and
   the canonical result digest are present today (rows 10 and 11), the dataset digest belongs to the
   catalog, and the policy and kernel identities are new.

**Acceptance and verification.** A result cannot be produced without a study identity, asserted by
construction rather than by convention; a test that two runs of the same study over the same data
produce the same result digest; a test that a changed kernel version changes the implementation
identity while the study id stays stable.

**Not implemented.**

## 5. Defects

### 5.1 No defect found in this repository

The checks below were run to decide whether the authorisation to fix defects was triggered. Each is a
search or a read, not an execution; no test was run, and this is stated rather than implied.

| Check                                                                                 | Outcome                                                                                                                                                                                                                                                                                                                                                     |
| ------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Statistics registry versus the vectorbt model: is a declarative metric layer missing? | No: a trait, a registry and 34 statistics exist (rows 1 to 4)                                                                                                                                                                                                                                                                                               |
| Is a metric metadata mechanism missing?                                               | Yes, and it is a specification, not a defect: nothing is wrong today, a result is merely less informative than it could be (D1)                                                                                                                                                                                                                             |
| Is walk-forward validation missing?                                                   | No: stages exist (rows 7 and 8)                                                                                                                                                                                                                                                                                                                             |
| Is a split contract missing?                                                          | Yes, and it is a specification (D2)                                                                                                                                                                                                                                                                                                                         |
| Is leakage control missing?                                                           | Yes, and this is the one finding with a correctness character: a walk-forward study conducted today cannot purge or embargo, so an in-sample window may overlap the label horizon of its own out-of-sample window. It is recorded as D2 with an explicit policy rather than as a defect, because no existing test or documented guarantee is violated today |
| Is trial provenance recorded?                                                         | Partially (row 25): a digest and a report exist, a study identity does not, so this is D4 and D11 by specification                                                                                                                                                                                                                                          |
| Is a multiple-testing correction missing?                                             | Yes, and it is a specification (D4)                                                                                                                                                                                                                                                                                                                         |
| Is there a duplicate implementation to keep in parity?                                | No: one implementation per kernel (row 15)                                                                                                                                                                                                                                                                                                                  |
| Is the extension optional or switchable in a way that could silently change numerics? | No: it is mandatory and unconditional (row 16)                                                                                                                                                                                                                                                                                                              |
| Is the version relationship between the halves unverified?                            | No: asserted at build time (row 18)                                                                                                                                                                                                                                                                                                                         |
| Is a capability reason for analytics or data availability present?                    | Present for order denial (row 19), absent for analytics and data queries, which is D8                                                                                                                                                                                                                                                                       |
| Is bar-level ambiguity undocumented or optimistic?                                    | Documented and configurable (row 22), but the policy has no identity in the result (row 26), which is D3; whether the default should be pessimistic is an owner decision                                                                                                                                                                                    |
| Is dataset coverage and gap reporting missing?                                        | No: present and mature (row 23)                                                                                                                                                                                                                                                                                                                             |

The one item closest to a defect is the absence of any leakage control in walk-forward validation. It
is recorded as a specification because it does not violate a stated guarantee, but if the owner
considers an unpurged walk-forward result to be an incorrect result rather than a less rigorous one,
then the item moves from D2 to a defect and its fix becomes a bug fix with a regression test. That
judgement is the owner's.

### 5.2 Observations from the source library, as warnings

These are properties of vectorbt, recorded so that the same trap is not imported with the mechanism.

| Observation                                                                  | Evidence                                                                                                                                                                                                                                  | Why it matters here                                                                                                                                                |
| ---------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Three numerical defects and one bounds defect were fixed in a single release | Release notes for v1.1.1: rolling standard deviation stability in both engines, deflated Sharpe kurtosis convention, expanding mean and standard deviation when the minimum period exceeds the length, out-of-bounds reads on empty input | Two implementations multiply exactly this class of risk, which is why D5 exists as a test category rather than a parity suite                                      |
| A correction metric was wrong for a long time                                | The deflated Sharpe ratio used excess kurtosis and did not ignore missing returns until v1.1.1, and the notes state that results change                                                                                                   | A multiple-testing correction that is itself wrong is worse than none, which is why D4 pins the definition in our own terms and tests the convention               |
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
what makes the design independently reviewable, is stated in `design 12.1`.

| Item                            | Acceptance                                                                                                                       | Verification                                                                                                                            | Status          |
| ------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- | --------------- |
| D1 Metric identity              | Identity, title, units, tags and direction declared; unavailable statistics carry a status and a reason                          | One test covering computed, unavailable and unregistered in a single result; closed sets                                                | Not implemented |
| D2 Split contract               | Any series length and any set-length mixture, with a leakage policy that is respected and a zero interval that must be justified | Contract tests, a leakage test, a refusal test, a zero-interval justification test, window equality after migration                     | Not implemented |
| D3 Ambiguity policy             | Every bar-level assumption documented, pessimistic by default, ambiguous configurations rejected, policy identity recorded       | Assumption-to-test mapping, rejection test, policy identity in the result, policy-version distinguishability                            | Not implemented |
| D4 Multiple-testing reporting   | Trial provenance recorded; per-period inputs only; convention pinned in our own terms; reported and never a gate                 | Seven acceptance cases from section 4.4, including insufficient input and the monotonicity check                                        | Not implemented |
| D5 Numerical stability          | The applicable cases from section 4.5 present for each reducing kernel, variance compared against an independent method          | The category exists and each kernel has its cases                                                                                       | Not implemented |
| D6 Labels                       | The first tranche exists only on the target path, with the first-hit label and the wait convention pinned                        | Hand-computed asymmetric case, wait case, leakage test; per-instrument test in the second tranche                                       | Not implemented |
| D7 Parity protocol              | A written protocol exists and is linked from the owning crate                                                                    | The document and its link; the checklist applies only if a second implementation appears                                                | Not implemented |
| D8 Capability codes             | A probe returns a typed value with a closed code, and nothing branches on the detail text                                        | Two refusal cases per producer, and a source test against detail matching                                                               | Not implemented |
| D9 Cache                        | No acceptance criterion while deferred; if triggered, the key is the identity model and cached and uncached runs agree           | Deferred                                                                                                                                | Not implemented |
| D10 Provider adapter            | No acceptance criterion in this document                                                                                         | Belongs to the data-provider architecture review                                                                                        | Not implemented |
| D11 Schema ownership            | No acceptance criterion: the decision is to change nothing                                                                       | Not applicable                                                                                                                          | Not implemented |
| Study identity (no decision id) | A result cannot be produced without a study identity; the field lists are pruned, not filled in                                  | Construction-level assertion, digest stability under an unchanged study, kernel-version change reflected in the implementation identity | Not implemented |

## 7. Explicit non-goals

1. **No port of any vectorbt subsystem.** No accessor layer, no factory-generated indicator classes,
   no broadcast parameter grid, no plotting, no widgets.
2. **No dependency on the package**, and no vendored, transliterated or mechanically paraphrased code
   (design 1.2 and 7.3).
3. **No change to the execution model.** Research-layer mechanisms only; the engine, the exchange
   simulation and the order state machine are untouched.
4. **No behavior change without an owner decision**, including the pessimistic bar default.
5. **No provider integration here.** The provider adapter contract is moved to the data-provider
   architecture review, including the market question that goes with it.
6. **No gate introduced by any metric.** A metric that gates is a risk rule and belongs with the risk
   caps, as recorded in the earlier review.
7. **No second implementation invented to justify D7.** The protocol is a document until a second
   implementation exists.
8. **No speculative caching.** D9 is not started until a measured case of repeated expensive
   computation exists.

## 8. Outstanding measurements and open items

1. Whether the pessimistic bar default changes published results is unmeasured. Answering it requires
   running the existing backtest scenarios under both policies and comparing, which was not done in
   this review because the review runs no tests. This gates D3.
2. The correct leakage interval length is unmeasured and depends on the label horizon and bar spacing.
   No value is proposed here because a guessed interval is worse than an explicit zero. This gates D2.
3. The minimum trial count at which a correction is meaningful is unmeasured here and is a literature
   question rather than a repository question. This gates D4.
4. The units, tags and direction vocabulary does not exist yet, including whether it is closed. This
   gates D1.
5. Which further multiple-testing diagnostics are wanted beyond the first statistic is unanswered. The
   study record in D4 is what makes them possible later. This gates D4's second tranche.
6. The study-identity field lists are a proposal and need pruning. Every field kept must be justified
   by a comparison someone will actually perform.
7. The vectorbt clone was deleted, so any cited line must be re-derived from the pinned revision. The
   revision is recorded in section 2 and the re-derivation is a single shallow clone.
