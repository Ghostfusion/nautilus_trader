# 09 - Go further

You built a factor, ranked a universe, produced target weights, labelled the outcome, split the
period with leakage, corrected for the number of trials, and saw the path from weights to orders.
This lecture is honest about what the manual left out and where to read next.

## Honest gaps in this release

These are real limitations of the code as it stands, not things the manual forgot.

- **The factor pipeline has no Python binding.** `crates/research/src/` is Rust only. You cannot
  build a `Panel`, a `MembershipSeries` or a `DatasetDeclaration` from Python in this release. The
  Python research surface covers splits, leakage, labels, significance, identity and assumptions,
  but the pipeline that computes features and panels is Rust.

- **Point-in-time membership is not stored on the Python side.** `UniverseIdentity` in
  `python/nautilus_trader/optimization/identity.py` records a `membership_policy_id` and a
  `membership_as_of` time, but the [Optimization](../../concepts/optimization.md) concept page states
  it plainly: "nothing in this repository stores point-in-time membership history: a `UniverseIdentity`
  records the policy and the as-of time its caller declares, and the stored-membership workstream
  proposed by the earlier design review (`vnpy_lessons_design.md`, decisions D5 and D13) is what
  would make membership recoverable rather than re-evaluated. Until then a study can state its
  membership, and the statement is auditable but not enforced." The Rust `MembershipSeries` is a
  stored series; the Python identity is a declaration.

- **The count caps are Rust only.** `RiskEngineConfig.count_caps` cannot be set from Python. See
  [07](07-risks-and-limits.md).

- **Some label kinds are not built.** The concept page says: "Extrema and trend-state labels are not
  built: they need the dataset contract of the earlier review, and neither it nor a stored
  point-in-time membership exists."

- **The composite performance ratio is deliberately not implemented** (`docs/concepts/performance_periods.md`).

- **The significance correction is not wired into a result document.** A run record keeps its metric
  values and canonical document, not its return series, so a sweep cannot be corrected from what it
  currently keeps. You call `deflated_sharpe_ratio` yourself.

- **The sample is too small to be evidence.** Four instruments and 36 days cannot support a real
  inference. The fixture exists to teach the machinery, and every number in it is made up.

## The concept pages to read next

Read these in roughly this order.

| Page                                                         | Why                                                                                                                  |
| ------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------- |
| [Optimization](../../concepts/optimization.md)               | The whole research pipeline: spaces, splits, leakage, labels, significance, identity, stages, the `optimize` command |
| [Target pipeline](../../concepts/target_pipeline.md)         | How a signal becomes a sized target and then orders                                                                  |
| [Universes](../../concepts/universes.md)                     | Runtime membership, selection, and removal as a process                                                              |
| [Portfolio](../../concepts/portfolio.md)                     | The authority for position, PnL, exposure and equity                                                                 |
| [Accounting](../../concepts/accounting.md)                   | Balances, margins, and the accounting authority rule                                                                 |
| [Performance periods](../../concepts/performance_periods.md) | The periodic frame and its statistics                                                                                |
| [Backtesting](../../concepts/backtesting/)                   | Fill models, slippage and market impact                                                                              |

## The source files to read

- The factor pipeline: `crates/research/src/{membership,panel,feature,operators,label,dataset}.rs`.
- Its tests, which pin the hand-computed numbers: `crates/research/tests/{factors,leakage,membership,reproducibility,feature_leakage}.rs`.
- The Python research surface: `python/nautilus_trader/optimization/`.
- The target pipeline: `crates/trading/src/target.rs`, `crates/trading/src/target_pipeline.rs`, and
  the Python binding `crates/trading/src/python/target_pipeline.rs`.
- The risk engine configuration: `crates/risk/src/python/config.rs` and
  `crates/risk/src/engine/config.rs`.
- The canonical backtest shape: `examples/backtest/fx_market_maker_gbpusd_bars.py`.

## Running the research crate

The one command that shows the Rust pipeline working is the crate's own test target:

```bash
export CARGO_TARGET_DIR='D:/Users/vince/PycharmProjects/nautilus_trader/target'
cargo nextest run --locked -p nautilus-research
```

The crate has no cargo features, so no `--features` flag is needed. The full observed output of this
command is in [05](05-build-the-strategy.md); all 34 tests pass.

The tests in `crates/research/tests/factors.rs` pin hand-computed factor values and a digest
(`efbcf90a9df8567da04229361a3e5733d829b89effcde1896e2a370c768c3efe` for the "momentum" definition);
`crates/research/tests/leakage.rs` shows both a rejected look-ahead row and an accepted valid row.

## What to learn after this

1. **Work with real panel data.** Read bars from a `ParquetDataCatalog`, adjust for splits and
   dividends, and build a panel with more instruments and a longer horizon. Watch the balanced-count
   check from lecture 04 fail on real data, and fix it.

2. **Learn cross-sectional operators properly.** `rank`, `scale`, `sum` and `neutralize` are the
   vocabulary of factor portfolios. Neutralising against a sector is how you separate "this
   instrument is good" from "this sector is good".

3. **Learn the Sharpe ratio and the drawdown.** Read
   [performance periods](../../concepts/performance_periods.md) and register `MaxDrawdown` so you
   read risk as well as return.

4. **Learn the walk-forward stage.** `WalkForwardStage` splits a period into in-sample and
   out-of-sample segments, searches on the former and evaluates on the latter, per window. That is
   the honest way to test a factor over many periods. See
   `python/nautilus_trader/optimization/stages.py`.

5. **Learn the identity contracts.** `StudyIdentity`, `TrialIdentity`, `DatasetIdentity` and
   `ResearchResult` are how a result names the study, the data, the code and the assumptions that
   produced it. A number that cannot name those is an observation, not a research result.

6. **Read about the two famous factors.** Momentum and value have long, well-documented histories.
   The machinery in this manual is what you need to test a claim about them on your own data.

## A closing warning

NautilusTrader executes live trades involving real capital. A factor that looks excellent on a
36-day fixture of four made-up instruments is a coincidence. The purpose of this manual was never
to give you a profitable factor; it was to give you the habit of asking, of every number, "was this
knowable at the time, and how many things did I try before I found it?". If you keep that habit, the
machinery is doing its job.

Back to the [manual index](README.md), or on to another style in the
[user manuals](../README.md).
