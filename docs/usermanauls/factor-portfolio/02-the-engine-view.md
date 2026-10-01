# 02 - How this repository models the style

This lecture is a map. It names the objects and the files, and says which language each part lives
in. There is no code to type yet.

The style splits in two, and the split is important:

1. **Research** turns history into a factor, a label, and a tested result. In this repository the
   research pipeline is **Rust only**: it lives in `crates/research/src/` and has no Python binding.
   The parts of research that are Python are the statistics and provenance around the pipeline:
   splits, leakage, labels, significance and identity live in `python/nautilus_trader/optimization/`.
2. **Trading** turns the result into positions. This is the ordinary NautilusTrader path: a
   strategy, a signal, a target, an order, a risk check, a venue. It is Python.

You therefore write one pipeline in Rust (or you run the crate's tests to see it work) and the
trading side in Python.

## The research crate

The crate is `nautilus-research`, at `crates/research/src/`. Its own documentation
(`crates/research/src/lib.rs`) calls it "the dataset contract that keeps a research pipeline honest
about time", and notes it is deliberately independent of the trading engine: it depends only on the
domain model, the catalog, and serialization. That independence is the point. A research process
can build and version datasets without instantiating a trading node, and a factor value is
authoritative in the research pipeline and nowhere else.

| Module                              | The objects it defines                                                        |
| ----------------------------------- | ----------------------------------------------------------------------------- |
| `crates/research/src/membership.rs` | `MembershipInterval`, `MembershipSpell`, `MembershipRule`, `MembershipSeries` |
| `crates/research/src/panel.rs`      | `Panel`, `PanelRow`, `FeatureValue`, `PanelError`                             |
| `crates/research/src/feature.rs`    | `Feature`, `Expr`, `Provenance`, `TransformSide`, `FeatureError`              |
| `crates/research/src/operators.rs`  | `Operator`, the rolling and cross-sectional functions                         |
| `crates/research/src/label.rs`      | `Label`, `LabelKind`, `LabelError`                                            |
| `crates/research/src/dataset.rs`    | `DatasetDeclaration`, `DatasetSplit`, `SourceInterval`, `Digest`              |

### Membership: who was eligible, and when

An **instrument universe** is the set of instruments that are eligible at an instant. The engine
already has a runtime universe component for a live or backtested run; see
[Universes](../../concepts/universes.md). Research needs something different: *history*. A
membership rule evaluated today can only see the universe as it is now, so it cannot represent an
instrument that has since been delisted. `membership.rs` therefore stores membership as data.

- `MembershipInterval` is one continuous spell for one instrument, with an entry instant
  (`ts_event`, mirroring to `ts_init`) and an optional `exited_at`. The interval is **half-open**:
  the instrument is a member at its entry instant and no longer a member at its exit instant.
- `MembershipSeries` holds the intervals for one universe and source, and resolves membership at an
  instant with `members_at(ts)` and `is_member_at(instrument_id, ts)`.
- A `MembershipRule` seeds a series; after that the series is the authority, and `from_rule` stamps
  the rule's spells with the series' universe and source identities.
- `MembershipSeries::write_to_catalog` and `read_from_catalog` persist the series through the
  catalog's custom-data path, beside the market data it describes.

This is exactly the object whose absence makes survivorship bias possible. If you do not store who
was eligible at a past date, you can only re-evaluate today's rule over yesterday's prices, and that
is the mistake lecture 05 demonstrates.

### Panel: one row per instrument and instant

A `Panel` (`crates/research/src/panel.rs`) has one `PanelRow` per instrument and timestamp. Each row
carries:

- `instrument_id` and `ts_event` (the row's timestamp);
- `features`, a map from a canonical feature name to a `FeatureValue`;
- `label`, an optional forward outcome;
- `member`, the membership that applies at `ts_event`.

A `FeatureValue` is a value plus `as_of`, "the timestamp of the latest input the value reads". That
extra field is what makes the panel checkable. `Panel::new` runs `Panel::check`, which enforces two
structural rules and returns a typed `PanelError` instead of a silent wrong value:

- **No look-ahead.** If a feature's `as_of` is later than the row's `ts_event`, the panel is
  refused with `PanelError::Lookahead`. The error names the instrument, the row timestamp, the
  feature, and the offending instant.
- **Point-in-time membership.** Each row's `member` is resolved from the stored `MembershipSeries`
  at the row's timestamp. A row that claims membership the series does not record at that instant
  is refused with `PanelError::MembershipMismatch`.

Labels are exempt from the aperture rule, because a label is the target of prediction and may
legitimately read forward.

### Feature: a compiled expression tree

A **feature** (`crates/research/src/feature.rs`) is a deterministic transformation of market or
reference data, written once in a plain-text surface and compiled once into a typed tree. The
example in the module header is:

```text
rolling_mean(close, 20) / lag(close, 1)
```

The tree is the authority. There is no `eval` and no string interpreted at runtime: the parser
rejects an unknown function, a wrong argument count, a non-integer window and a group key that is
not a column, each as a typed error. An `Expr` is a `Literal`, a `Column`, or a `Call` to an
`Operator`. The compiled tree serializes canonically, independent of whitespace and of the order in
which definitions are combined, and digests to a stable identifier.

Every transform declares a **side** (`TransformSide`):

- `Inference` may read only the current observation and earlier ones.
- `Learning` may be fit over the whole learning window, including later observations.

A forward-reading operator such as `Lead` is permitted only on the learning side and is rejected at
parse time on the inference side. The side is part of the definition's digest, so the same tree on a
different side is a different definition.

### Operators: one meaning each

`crates/research/src/operators.rs` names the operator set and states each convention exactly: what an
absent input produces, population versus sample dispersion, how ties are ranked, whether a
regression carries an intercept, and how a neutralisation weights its groups. Two families exist:

- **Time-series** operators reduce over one instrument's history: `Lag`, `RollingMean`, `RollingStd`,
  `RollingCorrelation`, `RollingRank`, `RollingRegressionResidual`, and the arithmetic and
  comparison operators.
- **Cross-sectional** operators reduce over the instruments present at one timestamp: `rank`,
  `scale`, `sum`, and `neutralize`. The set is the point-in-time universe the caller supplies, so a
  cross section can never include an instrument that had not yet joined or that had already left.

Every operator yields an explicit absence (`None`) when its inputs are absent or a statistic is
undefined: a window that is not full, a zero denominator, a group with no observations. An
undefined statistic is never reported as zero.

### Label: the forward outcome

`crates/research/src/label.rs` defines a label with the same rigour as a feature: a named,
digestible declaration with a horizon and a stated terminal convention. Three kinds exist:

- `ForwardReturn`: `price[index + horizon] / price[index] - 1`.
- `ForwardMaxDrawdown`: the deepest peak-to-trough decline over the horizon, as a non-positive
  fraction.
- `ForwardRealizedVolatility`: the population standard deviation of the simple returns over the
  horizon.

The terminal convention is explicit: when fewer than `horizon` future observations remain,
`Label::compute` returns `None`, never zero. A non-positive price is a typed error.

### Dataset: the declaration with an identity

A `DatasetDeclaration` (`crates/research/src/dataset.rs`) is a declaration, not a caller's
convention. It names the catalog intervals it reads, the membership source that decides which
instruments are in scope, the digests of its feature and label definitions, and its train,
validation and test split (`DatasetSplit`). It serializes canonically (object keys sorted, every
collection sorted by its canonical fields) and digests to a stable identifier, so two processes that
build the same dataset agree on its identity regardless of the order in which its parts were
inserted. `Digest` is a BLAKE3 hash with a `combine` that sorts its parts first, so a set of partial
digests reduced across processes always yields the same combined digest.

## The Python research surface

The statistics and provenance around the pipeline are Python, in
`python/nautilus_trader/optimization/`. The concept page for all of it is
[Optimization](../../concepts/optimization.md).

| Module                                                | The objects it defines                                                                 |
| ----------------------------------------------------- | -------------------------------------------------------------------------------------- |
| `python/nautilus_trader/optimization/splits.py`       | `SplitContract`, `Split`, `SplitDirection`, `LeakagePolicy`, `LabelOverlapRule`        |
| `python/nautilus_trader/optimization/labels.py`       | `LabelDefinition`, `LabelSeries`, `label_series`, the label enums                      |
| `python/nautilus_trader/optimization/significance.py` | `StatisticalContract`, `SharpeSample`, `deflated_sharpe_ratio`, `per_period_sharpe`    |
| `python/nautilus_trader/optimization/identity.py`     | `StudyIdentity`, `DatasetIdentity`, `UniverseIdentity`, `ResearchResult`               |
| `python/nautilus_trader/optimization/capability.py`   | the closed refusal codes and the probes that answer them                               |
| `python/nautilus_trader/optimization/stages.py`       | `TrainStage`, `OptimizeStage`, `ValidateStage`, `OutOfSampleStage`, `WalkForwardStage` |
| `python/nautilus_trader/optimization/optimizer.py`    | `Optimizer`, `SearchReport`, the sweep itself                                          |

Three of these matter most in this manual:

- **`SplitContract`** owns the layout of a validation scheme rather than each caller improvising
  one. It names its sets, takes each set's length in nanoseconds or as a fraction of a window, and
  lays the windows out whole. Its `leakage` (`LeakagePolicy`) states the exclusion relation: what is
  purged before the evaluation set, after it, and the minimum gap between splits.
- **`LabelDefinition` and `label_series`** are the Python label layer, distinct from the Rust
  `Label`. A definition states the outcome, the horizon, the wait, the alignment
  (`AlignmentConvention`) and the missing-data policy; `label_series` computes one value per row.
  `LabelSeries.forward_reach_ns` is measured from the produced series, not derived from the
  definition.
- **`SharpeSample` and `deflated_sharpe_ratio`** are the multiple-testing correction. The sample
  declares the selected trial's Sharpe, every trial's Sharpe, the observations, the skew, the
  non-excess kurtosis, and whether the trials are independent.

## The trading side: signal, target, order

Once a factor produces weights, the trading side turns them into positions. The engine models this
as three separate layers, documented in [Target pipeline](../../concepts/target_pipeline.md):

- A **signal** is a statement of view about one instrument: a direction, an optional horizon,
  strength, source, expiry and provenance. It is not a trading command and carries no quantity.
- A **target** is a desired exposure for one instrument, stated as a quantity, a weight, or a
  notional. It is the exposure a view resolves to, not the change to the current exposure.
- An **order** is the existing order types. Only an order reaches a venue.

The pipeline lives in `crates/trading/src/target_pipeline.rs` (the two stages) and
`crates/trading/src/target.rs` (construction and reconciliation). It is opt-in per strategy and
disabled by default. From Python a strategy controls it with `enable_target_pipeline(config)`,
`disable_target_pipeline()`, `target_pipeline_enabled()`, and `submit_signals(signals)`; the
configuration value is `TargetPipelineConfig`, whose Python binding is
`crates/trading/src/python/target_pipeline.rs`. The pipeline never submits anything itself: it
returns order values as data, and the caller submits them on the existing strategy-to-execution
path.

The pipeline is where the weight you computed becomes an actual order, sized by a fixed-risk
calculation and capped by the configured `max_weight`, then reconciled against the current position
and resting orders so only real differences become orders.

## Where the numbers come from

- The **portfolio** is the authority for position, realised PnL, unrealised PnL and margin. See
  [Portfolio](../../concepts/portfolio.md).
- The **account model** is the other authority, for balances and margins. See
  [Accounting](../../concepts/accounting.md).
- The **performance-period frame** is the only periodic reduction of those authorities, one row per
  calendar period. See [Performance periods](../../concepts/performance_periods.md).

A statement of view can also be expressed as a signal on a live strategy
(`Strategy.on_signal`), and the runtime universe publishes membership changes on the topic
`events.universe.{name}`, received through `on_universe_changed`. Both are described in
[Universes](../../concepts/universes.md).

## What is enforced, and what is not

Enforced by the research crate, as typed errors: point-in-time membership in a panel, no look-ahead
in a feature, a label that returns an absence rather than a zero, and a dataset identity that
depends on every part of the declaration.

Enforced by the Python optimization surface: the split layout and the leakage exclusion relation,
the label's alignment and missing-data conventions, the statistical contract of the correction, and
the identity contracts that make a result nameable.

Not enforced anywhere: that your factor is a good idea, that your universe is complete, or that your
result will repeat. Those are your job, and lectures 06 and 07 are about doing it honestly.

Next: [03 - The first run](03-first-run.md).
