# 06 - Measure and evaluate

This lecture is about the number that says whether the slicing worked. It has two halves, and the
first half is a warning: the full execution-analytics subsystem in this repository is **Rust only**.
The second half shows what a Python user can measure today.

## Execution analytics is Rust only

The metrics you would expect from an execution desk live in `crates/trading/src/analytics/`:

| File                                          | Contents                                         |
| --------------------------------------------- | ------------------------------------------------ |
| `crates/trading/src/analytics/mod.rs`         | Module documentation and the `ExecutionObserver` |
| `crates/trading/src/analytics/metrics.rs`     | Metric identifiers, values, and the arithmetic   |
| `crates/trading/src/analytics/observation.rs` | The accumulator and the event collector          |
| `crates/trading/src/analytics/reference.rs`   | Metric declarations and reference conventions    |

There is no Python surface for it. The Python module `nautilus_trader.trading` registers
`ExecutionAlgorithm`, `ExecutionAlgorithmConfig`, and the strategy classes
(`crates/trading/src/python/mod.rs`), but it does not register anything from `analytics`. The
`nautilus-trading` crate has a `python` feature, and module `analytics` is declared at
`crates/trading/src/lib.rs:110`; the feature does not add a `python` submodule for analytics.
So a Python user cannot import `ExecutionMetrics` or `ExecutionObserver` today. Do not write Python
that pretends otherwise.

The module documentation states the design: it is "read-only execution analytics" that "observes an
execution and reports what happened" and "never changes it" (`crates/trading/src/analytics/mod.rs:20`).
A metric is always produced with the declaration that defines it, and an undefined metric is reported
as `MetricValue::NotAvailable` rather than as `0.0` (`crates/trading/src/analytics/metrics.rs:15-20`).

## The metric list

The identifiers are the string constants at `crates/trading/src/analytics/metrics.rs:39-69`. Each is a
field of `ExecutionMetrics` (`metrics.rs:156`).

| Identifier                     | Unit    | Direction        | Reference                        |
| ------------------------------ | ------- | ---------------- | -------------------------------- |
| `implementation_shortfall_bps` | bps     | lower is better  | decision price                   |
| `arrival_slippage_bps`         | bps     | lower is better  | arrival price                    |
| `decision_price_slippage_bps`  | bps     | lower is better  | decision price (delay component) |
| `vwap_slippage_bps`            | bps     | lower is better  | benchmark interval VWAP          |
| `twap_slippage_bps`            | bps     | lower is better  | benchmark interval TWAP          |
| `midpoint_slippage_bps`        | bps     | lower is better  | midpoint at arrival              |
| `spread_capture`               | ratio   | higher is better | quoted half-spread at arrival    |
| `adverse_selection`            | price   | lower is better  | midpoint at fill plus horizon    |
| `fill_ratio`                   | ratio   | -                | parent target quantity           |
| `cancel_ratio`                 | ratio   | -                | submitted child quantity         |
| `completion_time_s`            | seconds | -                | parent submission                |
| `child_count`                  | count   | -                | submitted children               |
| `child_churn`                  | ratio   | lower is better  | cancelled over submitted         |
| `mean_child_lifetime_s`        | seconds | -                | child submission to terminal     |
| `partial_fill_ratio`           | ratio   | -                | cancelled children               |
| `price_improvement`            | price   | higher is better | the order's own limit price      |

The sign convention for the bps metrics is cost positive, computed by `cost_bps`
(`metrics.rs:250`). The declarations, including the denominator and reference timestamp, are built in
`ExecutionMetrics::compute` (`metrics.rs:265`).

## Demonstrate it: run the crate's own tests

Because there is no Python path, the honest demonstration is the crate's test suite. From the
repository root:

```bash
export PATH="$HOME/.cargo/bin;$HOME/.local/uv012;$HOME/AppData/Local/Programs/Python/Python312/cpython-3.14-windows-x86_64-none;$PATH"
export CARGO_TARGET_DIR='<repo-root>/target'
cargo nextest run --locked -p nautilus-trading --features python -E 'test(analytics)'
```

Real output of that run:

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 22m 06s
------------
 Nextest run ID d93e875c-0773-4c90-9ae7-5839d58763f7 with nextest profile: default
    Starting 7 tests across 1 binary (962 tests skipped)
        PASS [   0.046s] (1/7) nautilus-trading analytics::tests::pinned_metrics_match_hand_computed_values
        PASS [   0.053s] (2/7) nautilus-trading analytics::tests::adverse_selection_uses_fill_plus_horizon_reference
        PASS [   0.066s] (3/7) nautilus-trading analytics::tests::observer_is_send_and_sync_friendly
        PASS [   0.072s] (4/7) nautilus-trading analytics::tests::passive_fill_spread_capture_is_positive
        PASS [   0.084s] (5/7) nautilus-trading analytics::tests::algorithm_child_counts_override_observed_counts
        PASS [   0.095s] (6/7) nautilus-trading analytics::tests::undefined_metric_is_not_available_never_zero
        PASS [   0.098s] (7/7) nautilus-trading analytics::tests::adverse_selection_requires_a_declared_horizon
------------
     Summary [   0.105s] 7 tests run: 7 passed, 962 skipped
```

Seven tests, all passing. The first build line is long because this compiles the whole crate; the
tests themselves take a tenth of a second. The test names tell you how the subsystem is expected to
behave: metrics match hand-computed values, an undefined metric is never `0.0`, an adverse-selection
reference requires a declared horizon, and an algorithm's own child counts override the observed
counts.

## What a Python user can measure today

Python cannot call the analytics crate, but it can read the results of a run. Three reports are
enough: the account report, the order fills report, and the positions report. Add this block to the
lecture 03 program, immediately before `engine.dispose()`:

```python
fills = engine.generate_order_fills_report()
account = engine.generate_account_report(SIM)
positions = engine.generate_positions_report()

total_qty = sum(float(q) for q in fills["filled_qty"])
notional = sum(float(q) * float(p) for q, p in zip(fills["filled_qty"], fills["avg_px"]))
fill_vwap = notional / total_qty
arrival = (109.500 + 109.510) / 2
shortfall_money = (fill_vwap - arrival) * total_qty
shortfall_bps = (fill_vwap - arrival) / arrival * 10_000
commission = sum(int(m.split()[0]) for c in fills["commissions"] for m in c)

print(f"fills: {len(fills)}")
print(f"total filled quantity: {total_qty}")
print(f"fill VWAP: {fill_vwap:.6f}")
print(f"arrival mid price: {arrival:.3f}")
print(f"implementation shortfall: {shortfall_money:.3f} JPY ({shortfall_bps:.3f} bps)")
print(f"total commission: {commission} JPY")
print()
print(fills.reset_index()[["client_order_id", "side", "quantity", "status", "exec_spawn_id", "avg_px"]].to_string(index=False))
print()
print(account[["total", "locked", "free", "base_currency"]].to_string())
print()
print(positions[["instrument_id", "quantity", "avg_px_open", "realized_pnl", "commissions"]].to_string())
```

Real output, after the order list from lecture 03:

```text
fills: 6
total filled quantity: 6000.0
fill VWAP: 109.536167
arrival mid price: 109.505
implementation shortfall: 187.000 JPY (2.846 bps)
total commission: 12 JPY

               client_order_id side quantity status               exec_spawn_id  avg_px
   O-20190101-230000-001-000-1  BUY     1000 FILLED O-20190101-230000-001-000-1 109.561
O-20190101-230000-001-000-1-E1  BUY     1000 FILLED O-20190101-230000-001-000-1 109.511
O-20190101-230000-001-000-1-E2  BUY     1000 FILLED O-20190101-230000-001-000-1 109.522
O-20190101-230000-001-000-1-E3  BUY     1000 FILLED O-20190101-230000-001-000-1 109.531
O-20190101-230000-001-000-1-E4  BUY     1000 FILLED O-20190101-230000-001-000-1 109.541
O-20190101-230000-001-000-1-E5  BUY     1000 FILLED O-20190101-230000-001-000-1 109.551

                                 total locked         free base_currency
2019-01-01 23:00:00+00:00  10000000.00   0.00  10000000.00           USD
2019-01-01 23:00:00+00:00   9999999.98   3.00   9999996.98           USD
2019-01-01 23:00:10+00:00   9999999.96   6.00   9999993.96           USD
2019-01-01 23:00:20+00:00   9999999.94   9.00   9999990.94           USD
2019-01-01 23:00:30+00:00   9999999.92  12.00   9999987.92           USD
2019-01-01 23:00:40+00:00   9999999.90  15.00   9999984.90           USD
2019-01-01 23:00:50+00:00   9999999.88  18.00   9999981.88           USD

                         instrument_id quantity  avg_px_open realized_pnl commissions
position_id
USD/JPY.SIM-TwapDemo-000   USD/JPY.SIM     6000   109.536167      -12 JPY    [12 JPY]
```

## What each number means

- **Fill VWAP 109.536167**. The quantity-weighted average of the six fills. This run is a buy, so a
  lower VWAP is better. It is what you actually paid.
- **Arrival mid price 109.505**. The midpoint of the first quote, `(bid + ask) / 2`. This is the
  reference. Here it is recomputed from the sample file rather than read from the engine, because the
  analytics crate that would record it is unavailable from Python.
- **Implementation shortfall 187.000 JPY (2.846 bps)**. The cost of the execution against arrival.
  For a reliable slice count this is the headline number. Whether 2.846 bps is good depends on the
  instrument: for a liquid FX pair one would hope for a few bps; 50 bps would say the execution was
  expensive or the market ran.
- **Total commission 12 JPY**. Six fills at 2 JPY each, from the maker/taker fee model. Costs paid to
  the venue are separate from impact and must be reported beside it.
- **Account rows**. The `locked` column rises by 3 JPY per fill while the order is working and
  settles when the position opens; `free` falls by the same amount. Nothing here tells you the
  execution cost.
- **Position row**. Quantity 6,000, `avg_px_open` 109.536167 (the fill VWAP), `realized_pnl` -12 JPY
  (the commission), and `commissions` `[12 JPY]`.

## Good and bad values

| Measurement              | Good look                          | Bad look                                   |
| ------------------------ | ---------------------------------- | ------------------------------------------ |
| Implementation shortfall | single-digit bps vs a liquid touch | tens of bps, or negative-looking for a buy |
| Fill ratio               | close to 1.0                       | well below 1.0 with quantity left over     |
| Child count              | close to the schedule              | far more children than slices (churn)      |
| Completion time          | at or under the horizon            | past the horizon with quantity open        |
| Commission               | small next to the shortfall        | commission dominating the shortfall        |

## Three common beginner misreadings

- **"The shortfall is the PnL."** It is not. It is a cost against a reference price. The position's
  profit or loss depends on where the price goes afterwards, which the execution algorithm does not
  control.
- **"A negative shortfall is always good."** For a buy, a fill below the reference gives a negative
  (favourable) shortfall, but with a small sample it is usually luck, not skill. Report it against
  many runs.
- **"The engine computed my VWAP slippage."** It did not. There is no Python bindings for the VWAP
  or TWAP slippage metrics; the VWAP above is computed by hand from the fills report. The Rust metric
  of the same name also weights by the benchmark interval's own trades and quotes, which this manual
  does not reconstruct.

Read [07-risks-and-limits.md](07-risks-and-limits.md) for what breaks and what the engine enforces.
