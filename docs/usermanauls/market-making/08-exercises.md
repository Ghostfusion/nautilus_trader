# 08 - Exercises

Each exercise changes one thing in the lecture `03` program. The solution is given as the changed
lines plus the real output the change produced. Try the change before reading the solution.

The base program is the one in `03-first-run.md`, with these settings:

```text
count=1_000, step=3, levels=3, prob=1.0, maker=0.00002, max_position=1_500_000, trade_size=500_000
```

With those settings the base run fills 4 orders and opens 1 position.

## Exercise 1: read the base output

Question: in the base run, exactly one fill is a taker. Which order is it, why did it happen, and
why does the positions report say `FLAT` rather than `LONG` or `SHORT`?

Solution: it is `O-20190101-231639-001-001-13`, `BUY  MARKET  1500000`, `liquidity_side=TAKER`. It
is not a grid order. It is the strategy's `on_stop` rule closing the open inventory, which is why
its side is `BUY` while the grid sells were `SELL`. The position is `FLAT` because that market order
bought back exactly the `1,500,000` the three sells had accumulated, so the net quantity is zero.

## Exercise 2: shrink the size

Change `trade_size=500_000` to `trade_size=100_000` and lower `max_position` to `1_500_000` is no
longer needed; leave it. Predict the fill count, then run.

Solution changed line:

```diff
-        trade_size=Quantity.from_int(500_000),
+        trade_size=Quantity.from_int(100_000),
```

Observed:

```text
filled orders: 8 positions: 1
                              side    type quantity    price filled_qty   avg_px
client_order_id
O-20190101-230000-001-001-2   SELL   LIMIT   100000  109.538     100000  109.538
O-20190101-230000-001-001-4   SELL   LIMIT   100000  109.571     100000  109.572
O-20190101-230000-001-001-6   SELL   LIMIT   100000  109.604     100000  109.604
O-20190101-230348-001-001-10  SELL   LIMIT   100000  109.791     100000  109.792
O-20190101-230348-001-001-12  SELL   LIMIT   100000  109.824     100000  109.824
O-20190101-230348-001-001-8   SELL   LIMIT   100000  109.758     100000  109.758
O-20190101-230857-001-001-14  SELL   LIMIT   100000  109.978     100000  109.978
O-20190101-231639-001-001-19   BUY  MARKET   700000      NaN     700000  109.965
```

Smaller size fills more often and at more levels, because each fill consumes less of the grid's
capacity. The taker close is `700,000`, the sum of the seven sell fills. The lesson: `trade_size` is
not just risk per level, it also changes how many levels get touched.

## Exercise 3: widen the grid

Change `grid_step_bps=3` to `grid_step_bps=10`.

Solution changed line:

```diff
-        grid_step_bps=3,
+        grid_step_bps=10,
```

Observed:

```text
filled orders: 2 positions: 1
                              side    type    price filled_qty   avg_px
client_order_id
O-20190101-230000-001-001-2   SELL   LIMIT  109.615     500000  109.615
O-20190101-231639-001-001-13   BUY  MARKET      NaN     500000  109.966
```

Only the first sell and the closing buy filled. The wider grid captured more per fill
(`109.615 - 109.966` is still a loss on trending data, but fewer fills means less fee churn). The
lesson: `grid_step_bps` trades fill frequency for margin per fill.

## Exercise 4: cap the position below the level count

Use `num_levels=5` and `max_position=500_000`, with `requote_threshold_bps=1_000` and `count=120`.

Solution changed lines:

```diff
-        max_position=Quantity.from_int(1_500_000),
-        trade_size=Quantity.from_int(500_000),
-        num_levels=3,
+        max_position=Quantity.from_int(500_000),
+        trade_size=Quantity.from_int(500_000),
+        num_levels=5,
```

Observed:

```text
                             side    type quantity    price    status
client_order_id
O-20190101-230000-001-001-1   BUY   LIMIT   500000  109.472  CANCELED
O-20190101-230000-001-001-2  SELL   LIMIT   500000  109.538    FILLED
O-20190101-230159-001-001-3   BUY  MARKET   500000      NaN    FILLED
```

Five levels were requested; only one per side was placed. `max_position=500,000` and
`trade_size=500,000` allow exactly one level of projected exposure per side. The lesson:
`num_levels` is a request, and `max_position` is the binding constraint.

## Exercise 5: let five levels fit

Raise `max_position` to `2_500_000` and keep `num_levels=5`, `count=120`, `requote_threshold_bps=1_000`.

Solution changed line:

```diff
-        max_position=Quantity.from_int(500_000),
+        max_position=Quantity.from_int(2_500_000),
```

Observed:

```text
                              side    type quantity    price    status
client_order_id
O-20190101-230000-001-001-1    BUY   LIMIT   500000  109.472  CANCELED
O-20190101-230000-001-001-10  SELL   LIMIT   500000  109.670  CANCELED
O-20190101-230000-001-001-2   SELL   LIMIT   500000  109.538    FILLED
O-20190101-230000-001-001-3    BUY   LIMIT   500000  109.439  CANCELED
O-20190101-230000-001-001-4   SELL   LIMIT   500000  109.571    FILLED
O-20190101-230000-001-001-5    BUY   LIMIT   500000  109.406  CANCELED
O-20190101-230000-001-001-6   SELL   LIMIT   500000  109.604    FILLED
O-20190101-230000-001-001-7    BUY   LIMIT   500000  109.373  CANCELED
O-20190101-230000-001-001-8   SELL   LIMIT   500000  109.637  CANCELED
O-20190101-230000-001-001-9    BUY   LIMIT   500000  109.340  CANCELED
O-20190101-230159-001-001-11   BUY  MARKET  1500000      NaN    FILLED
```

Now five buys and five sells are placed. The lesson: sizing is a budget. `trade_size * num_levels`
must be at most `max_position` for the full grid to appear.

## Exercise 6: remove the maker fee

Change both `maker_rate` and `taker_rate` to `Decimal("0")`.

Solution changed lines:

```diff
-        fee_model=MakerTakerFeeModel(maker_rate=Decimal("0.00002"), taker_rate=Decimal("0.00002")),
+        fee_model=MakerTakerFeeModel(maker_rate=Decimal("0"), taker_rate=Decimal("0")),
```

Observed:

```text
                              side    price   avg_px commissions
client_order_id
O-20190101-230000-001-001-2   SELL  109.538  109.538     [0 JPY]
O-20190101-230000-001-001-4   SELL  109.571  109.572     [0 JPY]
O-20190101-230000-001-001-6   SELL  109.604  109.604     [0 JPY]
O-20190101-231639-001-001-13   BUY      NaN  109.965     [0 JPY]
```

Every commission is zero. Run it again with a position report and the `realized_pnl` becomes the
gross loss, `-590,500` JPY instead of `-597,086` JPY. The lesson: the fee model is the only thing
separating gross and net, and at `0.002%` per side it is worth about `6,500` yen on a `1.5` million
position.

## Exercise 7: break it on purpose

Set `grid_step_bps=0` and run the base program. Do not predict, just look.

Solution changed line:

```diff
-        grid_step_bps=3,
+        grid_step_bps=0,
```

Observed:

```text
filled orders: 4 positions: 1
                              side    type    price    status
client_order_id
O-20190101-230000-001-001-1    BUY   LIMIT  109.505  CANCELED
O-20190101-230000-001-001-2   SELL   LIMIT  109.505    FILLED
O-20190101-230000-001-001-3    BUY   LIMIT  109.505  CANCELED
O-20190101-230000-001-001-4   SELL   LIMIT  109.505    FILLED
O-20190101-230000-001-001-5    BUY   LIMIT  109.505  CANCELED
O-20190101-230000-001-001-6   SELL   LIMIT  109.505    FILLED
O-20190101-230000-001-001-7    BUY   LIMIT  109.725  CANCELED
O-20190101-230000-001-001-8    BUY   LIMIT  109.725  CANCELED
O-20190101-230000-001-001-9    BUY   LIMIT  109.725  CANCELED
O-20190101-230857-001-001-10   BUY   LIMIT  109.945  CANCELED
O-20190101-230857-001-001-11   BUY   LIMIT  109.945  CANCELED
O-20190101-230857-001-001-12   BUY   LIMIT  109.945  CANCELED
O-20190101-231639-001-001-13   BUY  MARKET      NaN    FILLED
```

With zero spacing every level collapses onto the same price: three identical buys and three
identical sells at the mid. The grid has become three copies of the same order on each side, and the
sell at the mid crosses the bid once the market ticks down. A market maker that posts its whole
size at one price has no ladder, no spread capture, and six times the size at a single level.

The general rule this exercise teaches: a parameter that spaces levels is not cosmetic. Setting it
to zero does not disable spacing; it stacks every level. Check every numeric parameter for the
degenerate value before you sweep it.

## Summary of the exercises

| Change                          | Effect                                             |
| ------------------------------- | -------------------------------------------------- |
| `trade_size` 500,000 to 100,000 | More fills, more levels touched, smaller position. |
| `grid_step_bps` 3 to 10         | Fewer fills, wider capture per fill.               |
| `max_position` below the budget | Levels silently skipped.                           |
| `max_position` raised           | Full grid appears.                                 |
| Fee rate to zero                | Gross equals net; commissions `[0 JPY]`.           |
| `grid_step_bps=0`               | All levels collapse onto one price.                |

Continue to [09-go-further.md](09-go-further.md).

Previous: [07-risks-and-limits.md](07-risks-and-limits.md) | Next: [09-go-further.md](09-go-further.md)
