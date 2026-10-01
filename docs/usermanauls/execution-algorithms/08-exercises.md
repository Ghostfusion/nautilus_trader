# 08 - Exercises

Work each exercise before reading its solution. Every solution shows the changed lines and the real
output. Exercises 1 to 6 are normal; exercise 7 is a deliberate breakage.

## Exercise 1: predict a schedule

Starting from `twap_run.py`, change the parameters to:

```python
exec_algorithm_params={"horizon_secs": "30", "interval_secs": "5"},
```

Before running, write down: how many slices, what size each, and when the last one arrives. Then run
it and check.

**Solution.** The slice count is `floor(30 / 5) = 6`, and `floor(6000 / 6) = 1000` per slice. The
first slice is immediate and the rest are five seconds apart, so the last arrives 25 seconds after
the first. Real output:

```text
FILL child=O-20190101-230000-001-000-1-E1 qty=1000 px=109.511 ts=1546383600000000000
FILL child=O-20190101-230000-001-000-1-E2 qty=1000 px=109.517 ts=1546383605000000000
FILL child=O-20190101-230000-001-000-1-E3 qty=1000 px=109.521 ts=1546383610000000000
FILL child=O-20190101-230000-001-000-1-E4 qty=1000 px=109.526 ts=1546383615000000000
FILL child=O-20190101-230000-001-000-1-E5 qty=1000 px=109.531 ts=1546383620000000000
FILL child=O-20190101-230000-001-000-1 qty=1000 px=109.536 ts=1546383625000000000
```

The timestamps differ by `5000000000` nanoseconds, which is five seconds.

## Exercise 2: compute the shortfall by hand

Using [sample_data/twap_child_fills_sample.csv](sample_data/twap_child_fills_sample.csv), compute the
fill VWAP, then the implementation shortfall against an arrival mid of 109.505. Give the answer in
JPY and in bps.

**Solution.** Sum `qty * price` and divide by the total quantity. Every quantity is 1000 and there are
six rows:

```text
prices        = 109.511, 109.522, 109.531, 109.541, 109.551, 109.561
sum           = 657.217
mean (VWAP)   = 657.217 / 6          = 109.5361667
cost per unit = 109.5361667 - 109.505 = 0.0311667
money         = 0.0311667 * 6000      = 187.00 JPY
bps           = 0.0311667 / 109.505 * 10000 = 2.846 bps
```

The lecture 06 program prints the same numbers: `fill VWAP: 109.536167`, `implementation shortfall:
187.000 JPY (2.846 bps)`. If you got 0.0311667 and rounded too early to 0.031, you would report 2.83
bps; keep the full precision until the last step.

## Exercise 3: make TWAP give up on slicing

Set the horizon to 600 seconds and the interval to 60, keeping the quantity at 6000. Predict what
happens, run it, and explain the log line.

```python
exec_algorithm_params={"horizon_secs": "600", "interval_secs": "60"},
```

**Solution.** The slice size is `floor(6000 / 10) = 600`, which is below the instrument's minimum
quantity of 1000. TWAP detects this and submits the whole parent in one order instead of spawning
slices. Real output:

```text
[WARN] ...nautilus_trading::algorithm::twap: Submitting for entire size: qty_per_interval=600 < min_quantity=1000
FILL child=O-20190101-230000-001-000-1 qty=6000 px=109.511 ts=1546383600000000000
--- orders in the cache ---
O-20190101-230000-001-000-1 MARKET BUY 6000 FILLED spawn: O-20190101-230000-001-000-1
```

One order, no children. The source branches are `twap.rs:297` and `twap.rs:307`. This is a useful
safety valve: a schedule that cannot produce valid children degrades to a single order rather than
producing invalid ones.

## Exercise 4: declare a policy part TWAP cannot honour

Add `"price_limit": "109.400"` to the TWAP parameters and run. What reason does the denial give, and
which line of `twap.rs` produced it?

```python
exec_algorithm_params={"horizon_secs": "60", "interval_secs": "10", "price_limit": "109.400"},
```

**Solution.** The order is denied, and no child is spawned. Real output:

```text
[WARN] ...nautilus_trading::strategy: TwapDemo-000 <--[EVT] OrderDenied(instrument_id=USD/JPY.SIM, client_order_id=O-20190101-230000-001-000-1, reason='VALIDATION_FAILED: price_limit is not supported by this execution algorithm')
```

The supported policy parts are the single-element list at `twap.rs:72`, and the refusal is emitted at
`twap.rs:243-251`. The point of the exercise is that the constraint is refused by name, not ignored.

## Exercise 5: an iceberg display size that is too small

Change the iceberg parameters to `{"display_size": "500", "requote_secs": "5"}` and run. Then try
`"display_size": "1000"` again.

**Solution.** 500 is below the instrument minimum quantity of 1000, so the parent is denied and no
child is sent. Real output:

```text
[WARN] ...nautilus_trading::strategy: IcebergDemo-000 <--[EVT] OrderDenied(instrument_id=USD/JPY.SIM, client_order_id=O-20190101-230000-001-000-1, reason='VALIDATION_FAILED: display_size=500 is below the instrument minimum quantity 1000')
O-20190101-230000-001-000-1 LIMIT BUY 6000 DENIED spawn: O-20190101-230000-001-000-1
```

The check is `iceberg.rs:684`. With `display_size` 1000 the run executes as in lecture 05.

## Exercise 6: an aggressive-only algorithm with a passive preference

Add `"preference": "passive"` to the sniper parameters and run. Explain why the algorithm refuses it.

**Solution.** The sniper crosses the touch deliberately, so it cannot be passive. Real output:

```text
[WARN] ...nautilus_trading::strategy: SniperDemo-000 <--[EVT] OrderDenied(instrument_id=USD/JPY.SIM, client_order_id=O-20190101-230000-001-000-1, reason='VALIDATION_FAILED: preference=passive must not be declared: this algorithm crosses the touch deliberately')
O-20190101-230000-001-000-1 LIMIT BUY 6000 DENIED spawn: O-20190101-230000-001-000-1
```

The refusal is at `sniper.rs:424-431`. The mirror case is the iceberg refusing an aggressive
preference at `iceberg.rs:400`.

## Exercise 7: break it on purpose

Move the order submission from `on_quote` back into `on_start`, so the strategy becomes:

```python
    def on_start(self) -> None:
        self.subscribe_quotes(self.config.instrument_id)
        order = self.order_factory.market(
            instrument_id=self.config.instrument_id,
            order_side=OrderSide.BUY,
            quantity=Quantity.from_str(self.config.order_quantity),
            exec_algorithm_id=ExecAlgorithmId("TWAP"),
            exec_algorithm_params={"horizon_secs": "60", "interval_secs": "10"},
        )
        self.submit_order(order)

    def on_quote(self, _quote) -> None:
        pass
```

Run it and explain why nothing slices.

**Solution.** The order is created and submitted while the execution algorithm is still starting. The
command is not handled, no slice schedule is built, and the parent stays `INITIALIZED` for the whole
run. Real output:

```text
--- orders in the cache ---
O-20190101-230000-001-000-1 MARKET BUY 6000 INITIALIZED spawn: O-20190101-230000-001-000-1
```

One order, no children, no fills. The parent is not even `SUBMITTED`. This is why lecture 03 uses the
first quote: the algorithm must be running before it can receive the command. Move the submission to
`on_quote` guarded by a flag to fix it. The repository's own test uses the first-quote pattern at
`crates/backtest/tests/integration/backtest_engine.rs:428-456`.

## Exercise 8 (extra): count the churn

Run the iceberg exercise with `"requote_secs": "1"` instead of `"5"` and count the children. Compare
with `"requote_secs": "5"`.

**Solution.** A shorter re-quote interval lets the algorithm cancel and re-send more often, so the
child count rises while the filled quantity does not improve in proportion. The schedules keep
`submitted` and `cancelled` counters precisely so this ratio can be reported
(`iceberg.rs:625-628`). Watch the count of `-E` children in `engine.cache.orders()`, not just the
fills.

## What to take away

- A schedule that cannot create valid children is refused or degraded, never silently violated.
- A constraint an algorithm cannot honour produces a named denial.
- Restoring and conserving quantity is the engine's job; a half-executed parent is a normal outcome.
- Timing matters: an algorithm that is not yet running cannot slice your order.

Read [09-go-further.md](09-go-further.md) for the gaps and the next steps.
