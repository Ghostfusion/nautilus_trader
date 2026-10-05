# 09 - Going further

## Honest gaps

The search support is real and it is also incomplete. These are the boundaries you will meet, stated
plainly so you do not discover them by surprise.

- **The correction is not wired into the result document.** The configuration declares no dataset,
  and a run record retains its metric values and canonical document rather than its return series, so
  a sweep cannot be corrected from what it currently keeps
  (`docs/concepts/optimization.md`, "Multiple-testing-aware reporting"). You supply the per-period
  returns yourself.
- **A universe identity is a declaration, not yet a guarantee.** Nothing in the repository stores
  point-in-time membership history. A `UniverseIdentity` records the policy and the as-of time its
  caller declares, and the stored-membership workstream that would make membership recoverable is not
  built (`python/nautilus_trader/optimization/identity.py`).
- **Extrema and trend-state labels are not built.** They need the dataset contract of an earlier
  design review, and neither it nor a stored point-in-time membership exists
  (`docs/concepts/optimization.md`, "Labels and the target path").
- **No Parquet catalog of bars ships with the repository.** The example first writes one to a
  temporary directory from a tracked minute-bar CSV, exactly as a backtest would read it
  (`examples/backtest/notebooks/optimization_sweep.py`).
- **The factor pipeline, membership, panel and dataset are Rust-only**, in `crates/research`, with
  no Python bindings. So are execution analytics (`crates/trading/src/analytics`), the notification
  router (`crates/common/src/notification`), and option pricing
  (`crates/model/src/data/pricing.rs`). Do not look for a Python API for them; exercise a Rust-only
  subsystem through its own crate tests.

## The concept pages to read

- [`docs/concepts/optimization.md`](../../concepts/optimization.md) - the whole subsystem: the
  pipeline, the stage model, splits and leakage, labels, identity contracts, multiple-testing-aware
  reporting, search strategies, persistence and concurrency, the public API, the `optimize` command,
  and the configuration file.
- [`docs/concepts/performance_periods.md`](../../concepts/performance_periods.md) - the periodic
  result frame that is a natural reward signal, and the four statistics defined over it.
- [`docs/concepts/backtesting/fill-models.md`](../../concepts/backtesting/fill-models.md) - the fill
  and slippage models, the latency model, and the exact statement of what is and is not reproducible.
- [`docs/concepts/backtesting/bar-execution.md`](../../concepts/backtesting/bar-execution.md) and
  [`docs/concepts/backtesting/fill-prices-and-matching.md`](../../concepts/backtesting/fill-prices-and-matching.md)
  - the assumptions a bar-driven replay makes, and how a fill price is chosen.
- [`docs/concepts/portfolio.md`](../../concepts/portfolio.md) and
  [`docs/concepts/accounting.md`](../../concepts/accounting.md) - the authorities a performance frame
  reduces.
- [`docs/concepts/execution/algorithms.md`](../../concepts/execution/algorithms.md) - execution
  algorithms, if your search tunes execution rather than signals.

## What to read next

- The [cross-venue and relative value manual](../cross-venue-relative-value/README.md) builds screens
  over a universe. It is the natural next step because a screen is a family of trials, and the trial
  family is exactly what the correction needs (`python/nautilus_trader/optimization/relative_value.py`).
- The [factor research manual](../factor-portfolio/README.md) builds a point-in-time dataset, a
  factor and a validated split. It goes deeper into the dataset and split contracts that this manual
  only introduces.
- The [production operations manual](../production-operations/README.md) covers the risk limits,
  reconciliation and state persistence you need before any tuned strategy runs for real.

## A closing checklist

Before you trust a search result, answer these in writing:

1. What is the parameter space, and how many experiments did it contain?
2. Which window was searched, and which window was held out?
3. Was every evaluation seeded, so the sweep repeats?
4. What is the nominal trial count, and is it the effective one?
5. What execution assumptions did the winning result run under?
6. What does the corrected value say, and what does its status mean?
7. What happens to the winner on the next period you did not search?

If any answer is missing, the number is an observation, not a research result
(`python/nautilus_trader/optimization/identity.py`).
