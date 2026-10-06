# 06 - Measure and evaluate

This lecture is about the number that says whether the slicing worked. It has two halves, and the
first half is a warning about reading a number without its convention. The second half shows what a
Python user can measure today.

## Execution analytics lives in Rust

The metrics you would expect from an execution desk live in `crates/trading/src/analytics/`:

| File                                          | Contents                                         |
| --------------------------------------------- | ------------------------------------------------ |
| `crates/trading/src/analytics/mod.rs`         | Module documentation and the `ExecutionObserver` |
| `crates/trading/src/analytics/metrics.rs`     | Metric identifiers, values, and the arithmetic   |
| `crates/trading/src/analytics/observation.rs` | The accumulator and the event collector          |
| `crates/trading/src/analytics/reference.rs`   | Metric declarations and reference conventions    |

Module `analytics` is declared at `crates/trading/src/lib.rs:110`, and the `python` feature exposes
the reader surface through `crates/trading/src/python/analytics.rs`, registered in
`nautilus_trader.trading` at `crates/trading/src/python/mod.rs:63`: `ExecutionObserver` with every
`set_*` and `observe_*` method, the term and observation types, the metric vocabulary and the
`METRIC_*` identifiers. A caller feeds the observer the same observations the Rust API takes and
reads the metrics back, so no Python code has to re-derive a slippage figure. Two parts stay
Rust-only: the bus-integrated `ExecutionAnalyticsCollector`, because registering a Rust actor from
Python needs an actor-registration path this crate does not own, and the periodic report row. Every
metric the collector forwards is reachable through the observer.

The module documentation states the design: it is "read-only execution analytics" that "observes an
execution and reports what happened" and "never changes it" (`crates/trading/src/analytics/mod.rs:20`).
A metric is always produced with the declaration that defines it, and an undefined metric is reported
as `MetricValue::NotAvailable` rather than as `0.0` (`crates/trading/src/analytics/metrics.rs:15-20`).

## The metric list

The identifiers are the string constants at `crates/trading/src/analytics/metrics.rs:39-71`. Each
is a field of `ExecutionMetrics` (`metrics.rs:191`).

| Identifier                      | Unit    | Direction        | Reference                        |
| ------------------------------- | ------- | ---------------- | -------------------------------- |
| `implementation_shortfall_bps`  | bps     | lower is better  | decision price                   |
| `arrival_slippage_bps`          | bps     | lower is better  | arrival price                    |
| `decision_price_slippage_bps`   | bps     | lower is better  | decision price (delay component) |
| `decision_to_execution_delay_s` | seconds | lower is better  | declared decision to first fill  |
| `vwap_slippage_bps`             | bps     | lower is better  | benchmark interval VWAP          |
| `twap_slippage_bps`             | bps     | lower is better  | benchmark interval TWAP          |
| `midpoint_slippage_bps`         | bps     | lower is better  | midpoint at arrival              |
| `spread_capture`                | ratio   | higher is better | quoted half-spread at arrival    |
| `adverse_selection`             | price   | lower is better  | midpoint at fill plus horizon    |
| `fill_ratio`                    | ratio   | -                | parent target quantity           |
| `cancel_ratio`                  | ratio   | -                | submitted child quantity         |
| `completion_time_s`             | seconds | -                | parent submission                |
| `child_count`                   | count   | -                | submitted children               |
| `child_churn`                   | ratio   | lower is better  | cancelled over submitted         |
| `mean_child_lifetime_s`         | seconds | -                | child submission to terminal     |
| `partial_fill_ratio`            | ratio   | -                | cancelled children               |
| `price_improvement`             | price   | higher is better | the order's own limit price      |

The sign convention for the bps metrics is cost positive, computed by `cost_bps`
(`metrics.rs:289`). The declarations, including the denominator and reference timestamp, are built
in `ExecutionMetrics::compute` (`metrics.rs:311`). `decision_to_execution_delay_s` is the elapsed
time from the declared decision timestamp to the **first** fill, so it stops when the market first
answers the order rather than when the parent finishes, and it is readable while the parent is still
working. It reports `NotAvailable(NoTimestamp)` when no decision instant was declared and
`NotAvailable(NoObservations)` when nothing has filled yet; a fill recorded before the decision
instant yields a real `0.0` rather than a negative delay.

## Demonstrate it: run the crate's own tests

The crate's own test suite is the most direct demonstration of the arithmetic the Python surface of
the next section calls into. From the repository root:

```bash
export PATH="$HOME/.cargo/bin;$HOME/.local/uv012;$HOME/AppData/Local/Programs/Python/Python312/cpython-3.14-windows-x86_64-none;$PATH"
export CARGO_TARGET_DIR='<repo-root>/target'
cargo nextest run -p nautilus-trading --features python -E 'test(analytics)'
```

Real output of that run:

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 4m 11s
------------
 Nextest run ID e3a495f2-3f10-4ee4-be77-e53658a80cdd with nextest profile: default
    Starting 9 tests across 1 binary (964 tests skipped)
        PASS [   0.026s] (1/9) nautilus-trading analytics::tests::adverse_selection_uses_fill_plus_horizon_reference
        PASS [   0.027s] (2/9) nautilus-trading analytics::tests::observer_is_send_and_sync_friendly
        PASS [   0.041s] (3/9) nautilus-trading analytics::tests::decision_to_execution_delay_reports_the_missing_side
        PASS [   0.044s] (4/9) nautilus-trading analytics::tests::undefined_metric_is_not_available_never_zero
        PASS [   0.052s] (5/9) nautilus-trading analytics::tests::adverse_selection_requires_a_declared_horizon
        PASS [   0.058s] (6/9) nautilus-trading analytics::tests::pinned_metrics_match_hand_computed_values
        PASS [   0.064s] (7/9) nautilus-trading analytics::tests::algorithm_child_counts_override_observed_counts
        PASS [   0.070s] (8/9) nautilus-trading analytics::tests::decision_to_execution_delay_declares_what_it_measures
        PASS [   0.077s] (9/9) nautilus-trading analytics::tests::passive_fill_spread_capture_is_positive
------------
     Summary [   0.082s] 9 tests run: 9 passed, 964 skipped
```

Nine tests, all passing. The build line is long because this compiles the whole crate; the tests
themselves take under a tenth of a second. The test names tell you how the subsystem is expected to
behave: metrics match hand-computed values, an undefined metric is never `0.0`, an adverse-selection
reference requires a declared horizon, an algorithm's own child counts override the observed counts,
and the decision-to-execution delay measures to the first fill and reports which side is missing.

## What a Python user can measure today

Python can drive the analytics observer, and it can also read the results of a run. The observer is
what turns an execution into the metric list above; the reports below are what a program that never
constructs one can still read. Three reports are enough: the account report, the order fills report,
and the positions report. Add this block to the lecture 03 program, immediately before
`engine.dispose()`:

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
  reference. Here it is recomputed by hand because this program never feeds an observer; declaring
  the arrival reference and reading `arrival_slippage_bps` back is what the observer is for.
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
- **"The engine computed my VWAP slippage."** It did not, in this program: the VWAP above is computed
  by hand from the fills report because no benchmark interval was fed to an observer. The metric is
  available from Python, and it weights by the benchmark interval's own trades and quotes, which this
  manual does not reconstruct.

Read [07-risks-and-limits.md](07-risks-and-limits.md) for what breaks and what the engine enforces.
