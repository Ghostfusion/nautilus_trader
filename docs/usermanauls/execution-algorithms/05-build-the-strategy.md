# 05 - Build the strategy

This lecture builds the execution step by step, then varies it. It assumes you have the
`twap_run.py` program from [03-first-run.md](03-first-run.md) working. Each step shows the code that
changed and the real output it produced.

## Step 1: submit the parent order

The whole mechanism starts with one order that names an execution algorithm. In the strategy:

```python
order = self.order_factory.market(
    instrument_id=self.config.instrument_id,
    order_side=OrderSide.BUY,
    quantity=Quantity.from_str(self.config.order_quantity),
    exec_algorithm_id=ExecAlgorithmId("TWAP"),
    exec_algorithm_params={"horizon_secs": "60", "interval_secs": "10"},
)
self.submit_order(order)
```

Two arguments make it an execution-algorithm order:

- `exec_algorithm_id` selects the algorithm. The message bus routes the order to the algorithm whose
  id matches (`docs/concepts/execution/algorithms.md`).
- `exec_algorithm_params` is a `Mapping[str, str]`: every key and every value is a string, including
  numbers. TWAP parses `interval_secs` first and the policy parts after
  (`crates/trading/src/algorithm/twap.rs:170`, `twap.rs:243`).

## Step 2: register the algorithm on the engine

An id in an order does nothing unless an algorithm is registered under that id. Register it once,
before adding the strategy:

```python
engine.add_native_exec_algorithm(
    "TwapAlgorithm",
    ExecutionAlgorithmConfig(exec_algorithm_id=ExecAlgorithmId("TWAP")),
)
```

`add_native_exec_algorithm` is the Python binding at `crates/backtest/src/python/engine.rs:510`. It
accepts the type name and an `ExecutionAlgorithmConfig`; the config must set `exec_algorithm_id` or
the engine raises (`engine.rs:1212`). The four accepted type names are listed in
`native_exec_algorithm_register` at `engine.rs:1200-1207`: `TwapAlgorithm`, `IcebergAlgorithm`,
`QuotePeggedAlgorithm`, and `SniperAlgorithm`.

## Step 3: submit on the first quote

Submit the parent from `on_quote`, once, guarded by a flag:

```python
def on_start(self) -> None:
    self.subscribe_quotes(self.config.instrument_id)

def on_quote(self, _quote) -> None:
    if self._submitted:
        return
    self._submitted = True
    # ... build and submit the order ...
```

The algorithm must be running before it can handle the command. Submitting from `on_start` can race
the algorithm's start; the first quote is the earliest safe moment. The repository's own Rust test
uses the same pattern (`crates/backtest/tests/integration/backtest_engine.rs:428-456`).

## Step 4: read the spawned orders

Run the program. The important tail is:

```text
--- orders in the cache ---
O-20190101-230000-001-000-1 MARKET BUY 1000 FILLED spawn: O-20190101-230000-001-000-1
O-20190101-230000-001-000-1-E1 MARKET BUY 1000 FILLED spawn: O-20190101-230000-001-000-1
O-20190101-230000-001-000-1-E2 MARKET BUY 1000 FILLED spawn: O-20190101-230000-001-000-1
O-20190101-230000-001-000-1-E3 MARKET BUY 1000 FILLED spawn: O-20190101-230000-001-000-1
O-20190101-230000-001-000-1-E4 MARKET BUY 1000 FILLED spawn: O-20190101-230000-001-000-1
O-20190101-230000-001-000-1-E5 MARKET BUY 1000 FILLED spawn: O-20190101-230000-001-000-1
```

The parent/child relationship is visible in two places:

- The parent `O-20190101-230000-001-000-1` has `exec_spawn_id` equal to its own `client_order_id`.
- Each child ends in `-E1`, `-E2`, `-E3`, `-E4`, `-E5`, and every child's `exec_spawn_id` is the
  parent's id `O-20190101-230000-001-000-1`.

That is the documented pattern: a spawned order's own id is `{exec_spawn_id}-E{sequence}`, and its
`exec_spawn_id` is the primary's `client_order_id`
([Spawned orders](../../concepts/execution/algorithms.md#spawned-orders)). Internally, the spawn
counter lives in `ExecutionAlgorithmCore` (`crates/trading/src/algorithm/core.rs:83`).

The parent's quantity is `1000`, not `6000`, because the parent is reduced by each child as it is
created. During the run you can watch the reduction: the engine prints `OrderUpdated` events with
quantity `5_000`, `4_000`, `3_000`, `2_000`, `1_000` before each child. The final slice is submitted
by the parent itself (`twap.rs:445`).

## Step 5: change the schedule

TWAP turns `horizon_secs` and `interval_secs` into a slice count:

```text
num_intervals   = floor(horizon_secs / interval_secs)
qty_per_slice   = floor(order_quantity / num_intervals)   # floored to size precision
```

The code is at `twap.rs:274` and `twap.rs:281`. With a 30 second horizon and a 5 second interval the
count is still six, but the slices arrive twice as fast. Change the two parameters and the quantity
to `"6000"`:

```python
exec_algorithm_params={"horizon_secs": "30", "interval_secs": "5"},
```

Real output:

```text
FILL child=O-20190101-230000-001-000-1-E1 qty=1000 px=109.511 ts=1546383600000000000
FILL child=O-20190101-230000-001-000-1-E2 qty=1000 px=109.517 ts=1546383605000000000
FILL child=O-20190101-230000-001-000-1-E3 qty=1000 px=109.521 ts=1546383610000000000
FILL child=O-20190101-230000-001-000-1-E4 qty=1000 px=109.526 ts=1546383615000000000
FILL child=O-20190101-230000-001-000-1-E5 qty=1000 px=109.531 ts=1546383620000000000
FILL child=O-20190101-230000-001-000-1 qty=1000 px=109.536 ts=1546383625000000000
```

The timestamps step by `5_000_000_000` nanoseconds, five seconds, instead of ten. The first slice
still arrives immediately (`twap.rs:397`).

## Step 6: the remainder slice

When the quantity does not divide evenly, TWAP schedules the floor for every interval and appends one
more slice for the remainder (`twap.rs:295-320`). Change the quantity to `"6001"` and keep the 60/10
params. Real output:

```text
FILL child=O-20190101-230000-001-000-1-E1 qty=1000 px=109.511 ts=1546383600000000000
FILL child=O-20190101-230000-001-000-1-E2 qty=1000 px=109.522 ts=1546383610000000000
FILL child=O-20190101-230000-001-000-1-E3 qty=1000 px=109.531 ts=1546383620000000000
FILL child=O-20190101-230000-001-000-1-E4 qty=1000 px=109.541 ts=1546383630000000000
FILL child=O-20190101-230000-001-000-1-E5 qty=1000 px=109.551 ts=1546383640000000000
FILL child=O-20190101-230000-001-000-1-E6 qty=1000 px=109.561 ts=1546383650000000000
O-20190101-230000-001-000-1 1 DENIED spawn: O-20190101-230000-001-000-1
O-20190101-230000-001-000-1-E1 1000 FILLED spawn: O-20190101-230000-001-000-1
O-20190101-230000-001-000-1-E2 1000 FILLED spawn: O-20190101-230000-001-000-1
O-20190101-230000-001-000-1-E3 1000 FILLED spawn: O-20190101-230000-001-000-1
O-20190101-230000-001-000-1-E4 1000 FILLED spawn: O-20190101-230000-001-000-1
O-20190101-230000-001-000-1-E5 1000 FILLED spawn: O-20190101-230000-001-000-1
O-20190101-230000-001-000-1-E6 1000 FILLED spawn: O-20190101-230000-001-000-1
```

Six children of 1,000 fill, then the parent's final slice of 1 is **denied**. The reason, from the
engine log at warning level, is:

```text
SubmitOrder for O-20190101-230000-001-000-1 DENIED: QUANTITY_BELOW_MINIMUM: effective=1, min=1000
```

The instrument's minimum quantity is 1000, so a 1-unit slice cannot be accepted. Six thousand units
filled and the final 1 unit was correctly refused; no quantity is lost, because the remaining 1 unit
was never valid to submit. Do not read this as an error in TWAP; it is the risk engine refusing an
order that the instrument's rules forbid ([Order denied reasons](../../concepts/execution/index.md#order-denied-reasons)).
[07-risks-and-limits.md](07-risks-and-limits.md) covers what happens to the parent in that case.

## Step 7: the iceberg variant

The iceberg algorithm rests a limit order and keeps one child working at a time. It is a different
program because the order is a limit order and the parameters are different. The registrations are:

```python
engine.add_native_exec_algorithm(
    "IcebergAlgorithm",
    ExecutionAlgorithmConfig(exec_algorithm_id=ExecAlgorithmId("ICEBERG")),
)
```

and the order:

```python
order = self.order_factory.limit(
    instrument_id=self.config.instrument_id,
    order_side=OrderSide.BUY,
    quantity=Quantity.from_str("6000"),
    price=Price.from_str("109.500"),
    exec_algorithm_id=ExecAlgorithmId("ICEBERG"),
    exec_algorithm_params={"display_size": "1000", "requote_secs": "5"},
)
```

The market data is `TestDataProvider.usdjpy_quotes(count=3000)`, which is the same sine wave over a
longer window so the price comes back down to the resting order. Real output:

```text
O-20190101-230000-001-000-1 LIMIT BUY 0 INITIALIZED spawn: O-20190101-230000-001-000-1
O-20190101-230000-001-000-1-E1 LIMIT BUY 1000 CANCELED spawn: O-20190101-230000-001-000-1
O-20190101-230000-001-000-1-E2 LIMIT BUY 1000 FILLED spawn: O-20190101-230000-001-000-1
O-20190101-230000-001-000-1-E3 LIMIT BUY 1000 FILLED spawn: O-20190101-230000-001-000-1
O-20190101-230000-001-000-1-E4 LIMIT BUY 1000 FILLED spawn: O-20190101-230000-001-000-1
O-20190101-230000-001-000-1-E5 LIMIT BUY 1000 FILLED spawn: O-20190101-230000-001-000-1
O-20190101-230000-001-000-1-E6 LIMIT BUY 1000 FILLED spawn: O-20190101-230000-001-000-1
O-20190101-230000-001-000-1-E7 LIMIT BUY 1000 FILLED spawn: O-20190101-230000-001-000-1
```

What to notice:

- Seven children, not six. The first was cancelled because the market crossed it and `requote_secs`
  had elapsed, so it was re-quoted (`iceberg.rs:14-18`). The cancelled slice's quantity returns to
  the parent and is sent again, which is why a seventh child exists.
- The parent ends `INITIALIZED` with quantity `0`. Unlike TWAP, the iceberg never submits the parent
  itself; it spawns the whole quantity. The parent stays a local record with no remaining quantity.
- Each child is a `LIMIT` order, because iceberg refuses any other order type (`iceberg.rs:353`).

## Step 8: the sniper variant

The sniper sweeps displayed liquidity at a fixed limit price only when the touch is reachable.

```python
engine.add_native_exec_algorithm(
    "SniperAlgorithm",
    ExecutionAlgorithmConfig(exec_algorithm_id=ExecAlgorithmId("SNIPER")),
)
order = self.order_factory.limit(
    instrument_id=self.config.instrument_id,
    order_side=OrderSide.BUY,
    quantity=Quantity.from_str("6000"),
    price=Price.from_str("109.600"),
    exec_algorithm_id=ExecAlgorithmId("SNIPER"),
    exec_algorithm_params={"limit_price": "109.600", "max_children": "3"},
)
```

Real output:

```text
O-20190101-230000-001-000-1 LIMIT BUY 0 INITIALIZED spawn: O-20190101-230000-001-000-1
O-20190101-230000-001-000-1-E1 LIMIT BUY 6000 FILLED spawn: O-20190101-230000-001-000-1
```

One child carried the whole 6,000 because the displayed size at the touch was 1,000,000, and the
child quantity is `min(remaining, displayed size at the touch)` (`sniper.rs:24`). The `max_children`
bound of 3 was never reached because one child filled the lot. To see several children, the displayed
size must be smaller than the parent.

## Step 9: what each algorithm refuses

Declaring a parameter an algorithm cannot honour is refused by name. For example, adding
`"price_limit": "109.400"` to a TWAP order produces a denial naming `price_limit`, because TWAP's
only supported policy part is the horizon (`twap.rs:72`). The full honour/refuse table is in
[02-the-engine-view.md](02-the-engine-view.md#the-four-shipped-algorithms).

## Summary of the build

1. Register the algorithm with `add_native_exec_algorithm`, giving it the id you will use.
2. Build an order and set `exec_algorithm_id` and `exec_algorithm_params`.
3. Submit it from the first quote, once.
4. Read `engine.cache.orders()` and use `exec_spawn_id` to find the children.
5. Measure the result, which is the next lecture.

Read [06-measure-and-evaluate.md](06-measure-and-evaluate.md) to turn this run into a cost number.
