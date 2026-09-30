# VectorBT lesson review: implementation record

Companion to [`vectorbt_lessons_design.md`](vectorbt_lessons_design.md) and to
[`vnpy_lessons_implementation.md`](vnpy_lessons_implementation.md). Section references of the form
`design N` point at the design document, and decision identifiers (`D1` to `D11`) are the revised
numbering introduced in revision 2 of this record.

## 1. Status and scope

### 1.1 One item is implemented, the rest are not

This document records a probe, a comparison and a set of specified decisions. The probe changed no
production code: the authorising instruction permitted changes only for defects, no defect was found
in this repository (section 5.1), so the defect path was not exercised, and the probe's verification
was source inspection rather than execution (section 3).

The owner then authorised implementation work in the order of `design 12`. **D2, the split contract
and its leakage exclusion relation, is implemented** (`python/nautilus_trader/optimization/splits.py`,
consumed by the walk-forward stages), and section 4.2 records what was built and how it was
verified. Every other item remains **Not implemented** in section 6, and its mechanism in section 4
remains a specification rather than a description of code.

The D2 work was verified by execution, not inspection: `pytest tests/unit/optimization` (54 tests,
including 31 for the contract), `pytest tests/integration/test_optimization.py` (9 tests, including
the walk-forward scenario that pins the pre-migration windows), and the declared regression scenario
`optimization_golden` (1 test) all pass. A throwaway script exercised the configuration path end to
end, from a JSON document with a `stage.leakage` block through the emitted window records, and was
deleted afterwards. The statement that no test was run applies to the probe alone.

### 1.2 Revision history

| Version | Change                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| ------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| 1       | Initial record from the vectorbt probe: the licence finding, the capability inventory, eleven decisions and this implementation record                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| 2       | Applied design-review feedback. Decisions renumbered and classified by layer; the deflated Sharpe ratio rescoped from the requirement to the first statistic behind a trial-provenance requirement; leakage became a policy whose values are optional and whose concept is mandatory; the ambiguity resolution became a versioned policy recorded by identity; metrics gained a direction; labels split into tranches; caching deferred with an identity model; the provider adapter relocated; study identity added; the licence rule restated as an engineering boundary with a provenance chain; the dependency graph replaced and the phases restated as work order                                                                                                                                                                                                                                                                                                                                                                                |
| 3       | Applied the second review. Identity promoted to a first-class contract covering study, trial, dataset, universe and result, with the trial level made explicit, and moved into `design 8`; the design's dependency section split into a contract graph and a work order so the graph no longer contradicts its own explanation; the leakage policy became an exclusion relation with purge before, purge after, embargo after and a label overlap rule; the ambiguity default became an owner decision separate from the policy contract, and ambiguity was separated from execution simulation; the statistical contract for the multiple-testing correction was specified in full; the stability obligation was classified by kernel type; the metric status vocabulary gained `invalid` and the direction vocabulary became action-oriented; capability results became domain-scoped; the label definition gained an alignment convention; a no-decision-authority invariant was added; dataset and trial identity were added to the open questions |
| 4       | The owner authorised implementation in the work order of `design 12` and D2 was implemented: a `SplitContract` with named sets, absolute, fractional or omitted lengths, a layout direction, a minimum length and an evenly spaced split count; a `LeakagePolicy` with purge before, purge after, an embargo gap, a label overlap rule that folds a label horizon into the purge, and a justified zero; and the walk-forward stages migrated onto the contract with their windows unchanged. The evidence is in section 4.2 and the verification in section 6                                                                                                                                                                                                                                                                                                                                                                                                                                                                                          |

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

| #   | Fact                                                                                                                        | Evidence                                                                                                                                                                                                                                                                                                                                                                                                    |
| --- | --------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | Statistics are a Rust trait with one struct per statistic                                                                   | `crates/analysis/src/statistic.rs:36` `pub trait PortfolioStatistic: Debug {`, `:40` `fn name(&self) -> String;`, `:47` `fn calculate_from_returns(&self, returns: &Returns) -> Option<Self::Item>`, `:56` `calculate_from_realized_pnls`, `:68` `calculate_from_positions`, `:79` `calculate_from_returns_with_benchmark`                                                                                  |
| 2   | 34 statistics are declared, one module each                                                                                 | `crates/analysis/src/statistics/mod.rs:18-51`, 34 `pub mod` lines                                                                                                                                                                                                                                                                                                                                           |
| 3   | There is a name-keyed registry with runtime registration and deregistration                                                 | `crates/analysis/src/analyzer.rs:38` `pub type Statistic = Arc<dyn PortfolioStatistic<Item = f64> + Send + Sync>;`, `:59` `pub statistics: AHashMap<String, Statistic>,`, `:123` `pub fn register_statistic(&mut self, statistic: Statistic)`, `:128` `pub fn deregister_statistic`                                                                                                                         |
| 4   | Twenty statistics are registered by default                                                                                 | `crates/analysis/src/analyzer.rs:79-98`, twenty `register_statistic` calls                                                                                                                                                                                                                                                                                                                                  |
| 5   | There is no metric metadata object                                                                                          | No match for `metric_config`, `MetricConfig`, `MetricRegistry`, `MetricSpec`, `stats_builder` anywhere in the repository                                                                                                                                                                                                                                                                                    |
| 6   | Statistics return `Option` and a skipped statistic leaves no trace in the result                                            | `crates/analysis/src/statistic.rs:47`, `:56`, `:68` all return `Option<Self::Item>`; the analyzer inserts only `Some` values                                                                                                                                                                                                                                                                                |
| 7   | Walk-forward validation exists as explicit stages                                                                           | `python/nautilus_trader/optimization/stages.py:51` `class WalkForwardWindow`, `:74` `def walk_forward_windows(`, `:128` `class TrainStage`, `:159` `class OptimizeStage`, `:217` `class ValidateStage`, `:263` `class OutOfSampleStage`, `:351` `class WalkForwardStage`                                                                                                                                    |
| 8   | The stage vocabulary is closed and typed                                                                                    | `python/nautilus_trader/optimization/config.py:94` `STAGE_KINDS = ("optimize", "train", "validate", "out_of_sample", "walk_forward")`                                                                                                                                                                                                                                                                       |
| 9   | One search strategy is implemented, and searching is pure enumeration                                                       | `python/nautilus_trader/optimization/search.py:37` `class SearchStrategy(Protocol)`, `:62` `class GridSearch`                                                                                                                                                                                                                                                                                               |
| 10  | A run is identified by a canonical digest and recorded with its parameter mapping                                           | `python/nautilus_trader/optimization/space.py:47` `canonical_json`, `:74` `digest_of`, `:97` `class Experiment`, `runner.py:61` `class CanonicalRun`, `:85` `class FailedExperiment`                                                                                                                                                                                                                        |
| 11  | A search reports results and failures together                                                                              | `python/nautilus_trader/optimization/report.py:120` `class SearchReport`                                                                                                                                                                                                                                                                                                                                    |
| 12  | There was no purge or embargo rule, and the split contract now states one                                                   | `python/nautilus_trader/optimization/splits.py:120` `class LeakagePolicy` with `purge_before`, `purge_after` and `embargo_after`, `:194` `purge` folding in the label horizon, `:572` `_excluded` applying the relation; outside it, `python/nautilus_trader`, `crates` and `docs` still contain no purge or embargo as a validation concept, the only other hits being cache-instrument purges in adapters |
| 13  | There is no cross-validation splitter, and the split contract this review specified now exists                              | `python/nautilus_trader/optimization/splits.py:290` `class SplitContract`, `:247` `class Split`, `:104` `class SplitDirection`, `:120` `class LeakagePolicy`, `:89` `class LabelOverlapRule`, consumed by `python/nautilus_trader/optimization/stages.py:92` `walk_forward_windows`; no match for `cross_val` or `cv_split` anywhere in `python/nautilus_trader`, `crates`, or `docs`                       |
| 14  | There is no multiple-testing correction                                                                                     | No match for `deflated`, `PBO`, `multiple testing`, or `bootstrap` in `python/nautilus_trader`, `crates/analysis`, `crates/backtest`, or `docs`                                                                                                                                                                                                                                                             |
| 15  | Every hot path has exactly one implementation and it is Rust                                                                | `python/nautilus_trader/indicators/__init__.py:26` `from nautilus_trader._libnautilus.indicators import *`, `:29` `fixup_module_names(globals(), __name__)`; `crates/indicators/src/python/mod.rs:40-41` `#[pymodule]` / `pub fn indicators`                                                                                                                                                                |
| 16  | The compiled extension is mandatory                                                                                         | `python/nautilus_trader/__init__.py:26` unconditional star import of `nautilus_trader._libnautilus`; no `find_spec`, no engine setting, no pure-Python fallback for any hot path                                                                                                                                                                                                                            |
| 17  | The Rust core owns every domain schema                                                                                      | `crates/model/src/events/order/filled.rs:40-47` `pyo3::pyclass(module = "nautilus_trader.model", from_py_object)` and `pyo3_stub_gen::derive::gen_stub_pyclass` above `pub struct OrderFilled {`; `crates/model/src/python/mod.rs:302` `m.add_class::<crate::events::OrderFilled>()?;`                                                                                                                      |
| 18  | The version relationship is asserted at build time, not at run time                                                         | `crates/core/build.rs:30` `let nautilus_version = "2.0.0rc6";`; `python/nautilus_trader/__init__.py:29-34` derives `__version__` from installed distribution metadata; `Cargo.toml:55` `license = "LGPL-3.0-only"`                                                                                                                                                                                          |
| 19  | Order denial already reports a stable machine-readable code                                                                 | `crates/model/src/events/order/denied_reason.rs:64-68` `#[strum_discriminants(name(OrderDeniedCode), derive(Display, AsRefStr, EnumIter, EnumString), strum(serialize_all = "SCREAMING_SNAKE_CASE"))]`; `:57` `Only the leading code is canonical. Consumers must not recover classification or control flow`                                                                                               |
| 20  | The denial event itself carries only the rendered string                                                                    | `crates/model/src/events/order/denied.rs:61` `pub reason: Ustr,` within `pub struct OrderDenied`                                                                                                                                                                                                                                                                                                            |
| 21  | Configuration is typed, rejects unknown keys, and collects every violation with a field path                                | `crates/common/src/cache/config.rs:43` `#[serde(default, deny_unknown_fields)]`; `crates/backtest/src/config.rs:624-625` `pub fn validate(&self) -> ConfigResult<()>` with `let mut errors = ConfigErrorCollector::new();`, pushing `ConfigError::empty_field` and `ConfigError::range`                                                                                                                     |
| 22  | Bar-level ambiguity is resolved by a documented, configurable ordering policy                                               | `crates/backtest/src/config.rs:364` `pub bar_execution: bool`, `:367` `pub bar_adaptive_high_low_ordering: bool`, `:525-530` documents the fixed order Open, High, Low, Close and the adaptive heuristic                                                                                                                                                                                                    |
| 23  | Catalog coverage and gap reporting exist and are mature                                                                     | `crates/persistence/src/backend/parquet/catalog/coverage.rs:77` `pub fn get_missing_intervals_for_request(`, `:263` `get_intervals`; `crates/persistence/src/backend/parquet/catalog/mod.rs:244` `pub struct ParquetDataCatalog {`                                                                                                                                                                          |
| 24  | Nothing computes supervised labels, forward returns, extrema or trade MFE and MAE                                           | No match for `triple_barrier`, `forward_return`, `zigzag`, `local_extrema`, `mfe`, or `mae` in `crates/model`, `crates/data`, `crates/trading`, or `python/nautilus_trader`; the only Label concept is the proposal in `vnpy_lessons_design.md`                                                                                                                                                             |
| 25  | A study identity and a trial identity do not exist; run identity is partial                                                 | Rows 10 and 11 record a canonical digest, a run record and a report; no match for `study_id`, `trial_id`, `trial_count`, `search_space_digest` or `validation_scheme_digest` in `python/nautilus_trader`, and no match for `seed` in `python/nautilus_trader/optimization`, so a run records no seed. Those identifiers appear only in this review's own documents                                          |
| 26  | The result does not record which bar-ordering policy produced it                                                            | `crates/backtest/src/result.rs:105-114` records metrics and an elapsed time; no `ambiguity_policy`, `policy_id`, or ordering field is present                                                                                                                                                                                                                                                               |
| 27  | There is no dataset or universe identity vocabulary, and the name `kernel_version` is already taken by the operating system | No match for `dataset_identity`, `dataset_digest`, `universe_digest` or `membership_as_of` in `python/nautilus_trader` or `crates`; `as_of` appears only in `python/nautilus_trader/persistence/catalog_to_df.py`; `crates/common/src/logging/headers.rs:85` uses `kernel_version` for the OS kernel, so a numerical kernel version needs a distinguished name                                              |

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

**Specification to implement** (`design 9 L1`).

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

**Not implemented.**

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

**Specification to implement** (`design 9 L3`).

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

**Not implemented.**

### 4.4 D4 Multiple-testing-aware research reporting

**Specification to implement** (`design 9 L4`).

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

**Not implemented.**

### 4.5 D5 Numerical-stability obligation, classified by kernel type

**Specification to implement** (`design 9 L5`). Every research kernel that reduces, transforms or
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

**Not implemented.**

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

**Acceptance and verification.** The first-hit label pinned against a hand-computed case with
asymmetric thresholds; the wait convention pinned against a case where a zero wait would read the
current bar; the alignment convention pinned against both pairings so the two are distinguishable; a
leakage test; and, for the second tranche, a test that per-instrument thresholds are applied per
instrument.

**Not implemented.**

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

**Not implemented** (no such document exists today).

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

**Not implemented.**

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

**Not implemented and not scheduled.**

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

**Specification to implement** (`design 8`). These contracts are cross-cutting requirements that the
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

**Not implemented.**

## 5. Defects

### 5.1 No defect found in this repository

The checks below were run to decide whether the authorisation to fix defects was triggered. Each is a
search or a read, not an execution; no test was run, and this is stated rather than implied.

| Check                                                                                 | Outcome                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| ------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Statistics registry versus the vectorbt model: is a declarative metric layer missing? | No: a trait, a registry and 34 statistics exist (rows 1 to 4)                                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| Is a metric metadata mechanism missing?                                               | Yes, and it is a specification, not a defect: nothing is wrong today, a result is merely less informative than it could be (D1)                                                                                                                                                                                                                                                                                                                                                                                               |
| Is walk-forward validation missing?                                                   | No: stages exist (rows 7 and 8)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| Is a split contract missing?                                                          | It was, and it is now implemented (section 4.2)                                                                                                                                                                                                                                                                                                                                                                                                                                                                               |
| Is leakage control missing?                                                           | It was, and it was the one finding with a correctness character: a walk-forward study could not purge or embargo, so an in-sample segment could overlap the label horizon of its own out-of-sample segment. It was recorded as D2 with an explicit policy rather than as a defect, because no existing test or documented guarantee was violated. The mechanism now exists (section 4.2) and the stages apply a declared zero policy by default, so a study that needs exclusions declares them and one that does not says so |
| Is trial provenance recorded?                                                         | Partially (row 25): a digest and a report exist, and the study and trial levels do not, so this is D4 and the identity contracts by specification                                                                                                                                                                                                                                                                                                                                                                             |
| Is dataset or universe identity recorded?                                             | No (row 27), and it is the identity contract with the largest open question attached                                                                                                                                                                                                                                                                                                                                                                                                                                          |
| Is a multiple-testing correction missing?                                             | Yes, and it is a specification (D4)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| Is there a duplicate implementation to keep in parity?                                | No: one implementation per kernel (row 15)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                    |
| Is the extension optional or switchable in a way that could silently change numerics? | No: it is mandatory and unconditional (row 16)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| Is the version relationship between the halves unverified?                            | No: asserted at build time (row 18)                                                                                                                                                                                                                                                                                                                                                                                                                                                                                           |
| Is a capability reason for analytics or data availability present?                    | Present for order denial (row 19), absent for analytics and data queries, which is D8                                                                                                                                                                                                                                                                                                                                                                                                                                         |
| Is bar-level ambiguity undocumented or optimistic?                                    | Documented and configurable (row 22), but the policy has no identity in the result (row 26), which is D3; whether the default should be pessimistic is an owner decision                                                                                                                                                                                                                                                                                                                                                      |
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

| Item                                | Acceptance                                                                                                                                                                            | Verification                                                                                                                                              | Status                                                                                                           |
| ----------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- |
| D1 Metric identity                  | Identity, title, units, tags and direction declared; a four-state status with domain-owned reason codes                                                                               | One test covering computed, unavailable, invalid and unregistered in a single result; `invalid` versus `unavailable` on one metric; closed sets           | Not implemented                                                                                                  |
| D2 Split contract                   | Any series length and any set-length mixture; a leakage relation expressing purge before, purge after, embargo after and a label overlap rule; a zero interval that must be justified | Contract tests, a label-overlap test, a refusal test, a zero-interval justification test, three-exclusion expressibility, window equality after migration | Implemented (4.2); 31 contract tests, 23 configuration tests, 9 integration tests and `optimization_golden` pass |
| D3 Ambiguity policy                 | Every assumption documented, ambiguous configurations rejected, an identity recorded in the result, the policy distinct from execution simulation, the default named by the owner     | Assumption-to-test mapping, rejection test, identity in the result, policy-version distinguishability, a test asserting the named default                 | Not implemented                                                                                                  |
| D4 Multiple-testing reporting       | Study and trial identity recorded; the full statistical contract specified before testing; reported and never a gate                                                                  | Nine acceptance cases from section 4.4, including dependent trials and the annualisation rejection                                                        | Not implemented                                                                                                  |
| D5 Numerical stability              | Each kernel classified by class, and the obligation for its class met; running variance compared against an independent method                                                        | The classification exists and each kernel has its class obligation                                                                                        | Not implemented                                                                                                  |
| D6 Labels                           | The first tranche exists only on the target path, the definition includes the alignment convention, a leakage test fails if a label value is read as a feature                        | Hand-computed asymmetric case, wait case, both alignment pairings, leakage test; per-instrument test in the second tranche                                | Not implemented                                                                                                  |
| D7 Parity protocol                  | A written protocol exists and is linked from the owning crate; not a research prerequisite                                                                                            | The document and its link; the checklist applies only if a second implementation appears                                                                  | Not implemented                                                                                                  |
| D8 Capability results               | One shared shape with domain-scoped closed code sets; nothing branches on detail text                                                                                                 | Two refusal cases per domain, and a source test against detail matching                                                                                   | Not implemented                                                                                                  |
| D9 Cache                            | No acceptance criterion while deferred; if triggered, the key is the identity model and cached and uncached runs agree                                                                | Deferred                                                                                                                                                  | Not implemented                                                                                                  |
| D10 Provider adapter                | No acceptance criterion in this document                                                                                                                                              | Belongs to the data-provider architecture review                                                                                                          | Not implemented                                                                                                  |
| D11 Schema ownership                | No acceptance criterion: the decision is to change nothing                                                                                                                            | Not applicable                                                                                                                                            | Not implemented                                                                                                  |
| Identity contracts (no decision id) | A result cannot be produced without a study identity; the trial identity is sufficient to re-run a trial; field lists are pruned, not filled in                                       | Construction-level assertion, digest stability, kernel-version change reflected in the implementation identity, trial re-run                              | Not implemented                                                                                                  |

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

1. Which ambiguity policy is the default is an owner decision, and whether a pessimistic default
   changes published results is unmeasured. Answering it requires running the existing backtest
   scenarios under both policies and comparing, which was not done in this review because the review
   runs no tests. This gates D3.
2. The leakage rule's mechanism now exists (section 4.2) and its values remain a study decision: what
   observations are forbidden from training because their feature and label information overlaps the
   evaluation information. No interval length is proposed here because a guessed interval is worse
   than an explicit zero, which is why the stages declare a zero with its reason and why a zero
   without one is refused. Still gates D6's dataset-level validation and the values of every D2
   policy.
3. The minimum observation count and the minimum trial count at which a correction says anything are
   unmeasured here and are a literature question rather than a repository question. This gates D4.
4. Trial dependence in a typical sweep is unmeasured, and whether the chosen correction needs an
   effective trial count is unanswered. This gates D4's second tranche.
5. The units, tags and direction vocabulary does not exist yet, including whether it is closed. This
   gates D1.
6. The kernel classification is incomplete until every research kernel is assigned a class. This gates
   D5.
7. What constitutes dataset identity is the largest open question, and it gates the reproducibility of
   every study rather than any single decision.
8. What constitutes trial identity decides whether a trial can be re-run at all, and it gates D4.
9. The identity field lists are a proposal and need pruning. Every field kept must be justified by a
   comparison someone will actually perform.
10. The vectorbt clone was deleted, so any cited line must be re-derived from the pinned revision. The
    revision is recorded in section 2 and the re-derivation is a single shallow clone.
