# 09 - Go further

This lecture is honest about what this course did not cover, lists the concept documents worth
reading next, and gives you a path from the grid market maker to a strategy you could actually run.

## What this course covered

- What market making is, where the profit comes from, and the three ways it goes wrong.
- How this repository models quotes, books, fills, fees, positions, and the grid strategy.
- A complete runnable program, a committed sample-data set, and the reports and statistics.
- The limits you set (`max_position`, `trade_size`, `skew_factor`) and what the engine enforces.

## Honest gaps

Read these as a list of things to learn, not a list of things that were hidden.

### No queue-position model of the real book

`queue_position` in a backtest tracks the displayed depth ahead of your order and lets it consume
that depth on trades (`docs/concepts/backtesting/fill-models.md`). A real venue has hidden
liquidity, iceberg orders, and a priority rule that depends on arrival time and on the matching
algorithm. The backtest queue is a model, not the venue.

### No latency

The backtest applies no latency to market data replay. The latency model only delays trading
commands from the moment they are generated
(`docs/concepts/backtesting/fill-models.md`). A real market maker loses to latency in ways a
backtest will never show, because the price that triggered your quote may be gone before your
order arrives.

### No market impact on your own quotes

The fill models decide whether a resting order fills. They do not model your own quote changing
other participants' behavior. On a small market your quotes move the mid, and the backtest does
not know that.

### Execution analytics are Rust only

The repository measures execution quality with implementation shortfall, arrival slippage, and
related metrics in `crates/trading/src/analytics/`. There is no Python binding for that module, so
this manual cannot show you a Python call. It is a real gap for a Python user who wants to grade
their own executions.

You can still run the module's own tests, which pin the arithmetic by hand. From the repository
root:

```bash
export CARGO_TARGET_DIR='<repo-root>/target'
cargo nextest run --locked -p nautilus-trading -E 'test(pinned_metrics_match_hand_computed_values)'
```

Observed output:

```text
    Blocking waiting for file lock on build directory
   Compiling regex-automata v0.4.18
   Compiling serde_json v1.0.151
   Compiling nautilus-common v0.65.0 (<repo-root>\crates\common)
   Compiling sysinfo v0.39.6
   Compiling tokio v1.53.1
   Compiling nautilus-core v0.65.0 (<repo-root>\crates\core)
   Compiling regex v1.13.1
   Compiling rstest_macros v0.27.0
   Compiling rstest v0.27.0
   Compiling nautilus-model v0.65.0 (<repo-root>\crates\model)
   Compiling nautilus-analysis v0.65.0 (<repo-root>\crates\analysis)
   Compiling nautilus-portfolio v0.65.0 (<repo-root>\crates\portfolio)
   Compiling nautilus-execution v0.65.0 (<repo-root>\crates\execution)
   Compiling nautilus-risk v0.65.0 (<repo-root>\crates\risk)
   Compiling nautilus-trading v0.65.0 (<repo-root>\crates\trading)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 25m 24s
------------
 Nextest run ID 1aca55ea-13b3-4dce-b7cc-14ec40598a92 with nextest profile: default
    Starting 1 test across 1 binary (579 tests skipped)
        PASS [   0.038s] (1/1) nautilus-trading analytics::tests::pinned_metrics_match_hand_computed_values
------------
     Summary [   0.043s] 1 test run: 1 passed, 579 skipped
```

That test computes an execution's fill VWAP by hand, then asserts that implementation shortfall
against a decision price is `201` basis points and that arrival slippage against the arrival price
is the expected value (`crates/trading/src/analytics/mod.rs`). It is the closest thing in this
repository to the "how good was that execution?" measurement a market maker needs.

### Notification sinks are Rust only

Related gaps: the notification router and its sinks live only in Rust
(`crates/common/src/notification`). The pre-trade send/cancel/fill count caps are not in that
category: `RiskEngineConfig.count_caps` takes `RiskCap` values from Python, beside
`max_order_submit_rate`, `max_order_modify_rate` and `max_notional_per_order`.

### No full training of a market making strategy

This course configures a shipped strategy. It does not write a custom `Strategy` subclass with your
own quoting logic. That is the natural next step, and it is well supported: see
`docs/tutorials/ema_cross.py` for a complete custom strategy with config, indicators, and handlers,
and `examples/backtest/crypto_ema_cross_ethusdt_trade_ticks.py` for one run end to end.

## The concept documents to read next

In reading order:

- `docs/concepts/order_book.md`: book types, subscriptions, the own-order book, and how to subtract
  your own orders from the public depth.
- `docs/concepts/backtesting/fill-models.md`: all eleven fill models, slippage, market impact,
  liquidity consumption, queue position, and the order the concerns compose in.
- `docs/concepts/backtesting/fill-prices-and-matching.md`: how a fill price is chosen and how order
  book immutability affects repeated fills.
- `docs/concepts/execution/algorithms.md`: the TWAP implementation and how to write your own
  execution algorithm.
- `docs/concepts/portfolio.md` and `docs/concepts/positions.md`: inventory, netting, realized and
  unrealized PnL, and how closed cycles are archived.
- `docs/concepts/reports.md` and `docs/concepts/accounting.md`: every report column and the
  accounting authority behind each number.
- `docs/concepts/live.md` and `docs/concepts/execution/reconciliation.md`: connection state,
  reconnection, and how state is recovered after a disconnect.

## The project's own market making material

- `docs/tutorials/grid_market_maker_dydx.md`: the same grid strategy on a real venue, with the
  order-expiry and short-term-order mechanics of a live network.
- `docs/tutorials/lighter_rwa_composite_mm.md`: a two-input market maker that quotes one market
  using a signal from another, and the adapter wiring behind it.
- `examples/backtest/fx_market_maker_gbpusd_bars.py`: the canonical backtest this course adapted.
- `crates/trading/src/examples/strategies/grid_mm/`: the strategy source, its config, and its own
  tests.

## What to build next

Three exercises, in increasing difficulty:

1. **A custom quoting strategy.** Write a `Strategy` subclass that quotes a symmetric bid and ask
   around the mid, with `max_position` and a `skew_factor` of your own. Reuse the grid strategy's
   structure but make the spread your own function of volatility. Compare its statistics to the
   grid's with `engine.get_result()`.
2. **An inventory-aware grid.** Change the skew from a linear function of position to a function
   that grows faster as the position approaches `max_position`. Measure whether `peak_qty` falls and
   whether the win rate falls with it.
3. **A hedging market maker.** Quote one market and hedge the net inventory in a correlated one.
   This is the idea behind `CompositeMarketMaker` and `DeltaNeutralVol`, both registered in the
   live node (`crates/live/src/python/node.rs`).

## The one sentence to remember

A market maker is paid to be available, and being available means being wrong sometimes; the
business is in bounding how wrong.

That is the end of the course. If you build something from it, run it on data that oscillates
before you run it on data that trends, and read the positions report before you read the PnL.

Previous: [08-exercises.md](08-exercises.md) | Next: [README.md](README.md)
