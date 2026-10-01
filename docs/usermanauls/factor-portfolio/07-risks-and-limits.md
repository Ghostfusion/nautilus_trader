# 07 - Risks and limits

A factor study fails in five specific, predictable ways. Each one has a cheap test you can run
before you trust a result. This lecture names them, gives the test, then covers position sizing and
loss limits and says exactly what the engine enforces and what it does not.

## The five ways a study lies

### 1. Survivorship bias

**What it is.** Your history only contains the instruments that still exist today. The ones that
were delisted, merged away, or went to zero are missing, so your average result flatters you: the
losers were quietly removed from the sample.

**Why it happens here.** A membership rule evaluated today can only see the universe as it is now.
If you reconstruct a past universe by applying today's rule to yesterday's data, you have built the
survivor set, not the historical set. That is precisely why
`crates/research/src/membership.rs` stores membership as data and why
`MembershipSeries` is "the authority once written".

**One-line detection test.** Count the membership intervals that have an `exited_at` set. If the
count is zero across a multi-year history, your universe is a survivor set. A stored series that
never records an exit is a series that never records a failure.

### 2. Look-ahead bias

**What it is.** A feature reads a number that was not known at the row's instant. A price from
tomorrow, a rebalance from the future, or a value that was restated later.

**Why it happens here.** It is invisible in the shape of the result: a shifted series is still a
series. The engine makes it structural instead. `Panel::check` refuses a feature whose `as_of` is
later than its row's `ts_event` with `PanelError::Lookahead`; the message names the feature and the
offending instant. And a forward-reading operator such as `Lead` is rejected at parse time on the
inference side.

**One-line detection test.** For every row, assert that the latest input a feature reads is at or
before the row's own timestamp (`as_of <= ts_event`). If any row fails, the feature is reading the
future.

### 3. Multiple testing

**What it is.** You tried many factors or many parameters and reported the best one. The best of
many noisy estimates is positive by construction.

**Why it happens here.** Nothing about the winner's number records how many losers there were. That
is why `SharpeSample` requires the full list of trial Sharpes and why reporting the winner without
the family it came from is a selection, not an estimate.

**One-line detection test.** Count the distinct parameter sets you evaluated and compute
`deflated_sharpe_ratio` with that count. If the count is larger than what you reported against, the
reported probability is too high. Lecture 06 shows a real 0.882 falling to 0.865 when the count moves
from 12 to 20.

### 4. Transaction costs

**What it is.** The backtest reports gross returns. Real rebalancing pays commission on every trade
and slippage on every fill, and a daily factor book trades a lot.

**Why it happens here.** A factor's edge is often small, and a round trip pays commission twice. A
0.02% maker/taker rate -- the value in the canonical backtest shape at
`examples/backtest/fx_market_maker_gbpusd_bars.py` -- costs 0.04% per round trip. If your factor
earns 0.1% per rebalance before costs, more than a third is gone.

**One-line detection test.** Subtract `2 * commission_rate * turnover` from the gross return of every
rebalance and see whether the edge survives. If the plan cannot name its turnover, it cannot name its
cost.

### 5. Crowding

**What it is.** Many participants run the same factor. The edge is arbitraged away, and the exits are
crowded: when everyone unwinds at once, the price moves against everyone.

**Why it happens here.** A factor that worked spectacularly in one past era and not since is a
crowding candidate, not a discovery.

**One-line detection test.** Split your history in half and compute the factor's result in each half
separately. If the second half is a fraction of the first, the edge is decaying, and decay is what
crowding looks like before the crowd is visible.

## Position sizing

Sizing is where a ranking becomes a real risk. Three mechanisms matter.

**The target pipeline's weight cap.** `TargetPipelineConfig` carries `max_weight`, "the cap on the
magnitude of a constructed target weight". A signal resolves to a size and the exposure is expressed
as a weight capped by this value. Set it below 1.0 so one instrument can never become the portfolio.
See [Target pipeline](../../concepts/target_pipeline.md) and
`crates/trading/src/python/target_pipeline.rs`.

**The risk engine's notional cap.** `RiskEngineConfig` takes `max_notional_per_order`, a map from
instrument to a maximum notional. The Python constructor is in `crates/risk/src/python/config.rs`,
and it also accepts `max_order_submit_rate` and `max_order_modify_rate`. These are checked before an
order reaches a venue.

**The fixed-risk construction size.** The pipeline sizes a directional signal by the fixed-risk
sizing calculation in `nautilus_risk::sizing`, using `risk_per_trade` (the fraction of equity risked)
and `stop_loss_bps` (the stop distance in basis points of the entry price). Note carefully:
`stop_loss_bps` is a *sizing input*, not a resting stop order. It tells the sizer how far away a stop
would be, so that a fixed fraction of equity is risked. It does not place that stop.

## Loss limits

The engine has no automatic portfolio-level loss limit. There is nothing that watches your equity and
flattens the book at a drawdown. That is a strategy responsibility, and it must be written
explicitly.

Two things you can rely on:

- **Reduce-only orders.** They do not contribute to `balance_locked` on cash accounts and do not add
  to initial margin on margin accounts, since they can only decrease exposure
  (`docs/concepts/accounting.md`).
- **The target pipeline resolves a flat signal to reduce-to-zero.** A flat signal states
  reduce-to-zero and needs no portfolio context. That is the mechanism for closing a name the factor
  no longer wants.

A practical loss limit for a factor book is a hard cap on gross exposure and a hard cap on the
drawdown at which you stop touching the book and re-examine the study. Write both down before you
run live.

## What the engine enforces, and what it does not

| The engine enforces                                       | How                                                                     |
| --------------------------------------------------------- | ----------------------------------------------------------------------- |
| Point-in-time membership in a panel                       | `PanelError::MembershipMismatch` in `crates/research/src/panel.rs`      |
| No look-ahead in a feature                                | `PanelError::Lookahead` in `crates/research/src/panel.rs`               |
| A label returns an absence, never zero, at the data's end | `Label::compute` in `crates/research/src/label.rs`                      |
| A split excludes the labelled overlap it declares         | `LeakagePolicy` in `python/nautilus_trader/optimization/splits.py`      |
| A zero leakage interval must be justified in words        | `LeakagePolicy._validate_justification`                                 |
| An annualised Sharpe or an excess kurtosis is refused     | `SharpeSample` in `python/nautilus_trader/optimization/significance.py` |
| A per-order notional cap and submit/modify rate limits    | `RiskEngineConfig` in `crates/risk/src/python/config.rs`                |
| A target weight cap and a minimum order quantity          | `TargetPipelineConfig`                                                  |
| Only real differences become orders                       | reconciliation in `crates/trading/src/target.rs`                        |

| The engine does not enforce                             | Whose job it is |
| ------------------------------------------------------- | --------------- |
| That your universe is complete and historically correct | Yours           |
| That your factor is a good idea                         | Yours           |
| A portfolio-level drawdown or gross-exposure limit      | Yours           |
| A resting stop-loss at `stop_loss_bps`                  | Yours           |
| That your result will repeat                            | Nobody's        |

### A Rust-only limit you cannot set from Python

The risk engine also supports pre-trade **count caps**: windows over submitted, cancelled, and
filled orders. They are configured as `RiskEngineConfig.count_caps` (a `Vec<RiskCap>`) at
`crates/risk/src/engine/config.rs:71`, and the metrics are `Submit`, `Modify`, `Cancel`, `Fill`,
`Active` and `RepeatedRequest` in `crates/model/src/risk.rs`.

**This one is Rust only.** The Python `RiskEngineConfig` constructor
(`crates/risk/src/python/config.rs`) accepts only `bypass`, `max_order_submit_rate`,
`max_order_modify_rate`, `max_notional_per_order`, `full_position_exit_venues` and `debug`. There is
no Python argument for `count_caps`, so a Python user cannot set one. If you need a hard cap on how
many orders the engine will admit in a window, that is a Rust-level configuration in this release.

## Paper, then live

Run every factor in a backtest first, then against live data with no orders, then with a small size.
The backtest's fill and fee models are assumptions, not venue behaviour: see
[Backtesting](../../concepts/backtesting/) for fill models, slippage and the market-impact models. A
factor that only works under one fill model is a factor that only works in that model.

Next: [08 - Exercises](08-exercises.md).
