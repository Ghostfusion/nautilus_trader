# Exercises

Do these with the code from lecture 05 in front of you. Exercises 1 and 6 are paper only; the rest
change code. Solutions are given as diffs or as the changed lines, and the outputs shown are real.

## Exercise 1: hand-compute the first snapshot

The first snapshot in the sample is a bid at 50000.0 for 1.0 and an ask at 50001.0 for 2.0. Compute
the spread, the midpoint, the difference-form imbalance, and the ratio.

**Solution.**

```
spread    = 50001.0 - 50000.0 = 1.0
midpoint  = (50000.0 + 50001.0) / 2 = 50000.5
smaller   = min(1.0, 2.0) = 1.0
larger    = max(1.0, 2.0) = 2.0
ratio     = 1.0 / 2.0 = 0.500
imbalance = (1.0 - 2.0) / (1.0 + 2.0) = -1.0 / 3.0 = -0.333
```

The ask side is heavier. The ratio 0.500 is above the default trigger of 0.20, so this snapshot does
not trade.

## Exercise 2: lower the trigger threshold

Change `trigger_imbalance_ratio` from 0.20 to 0.10 in `run_imbalance.py` and predict what happens to
the committed sample before you run it.

**Solution.** One changed line:

```diff
     OrderBookImbalanceConfig(
         instrument_id=str(instrument.id),
         max_trade_size="1",
         trigger_min_size=1.0,
-        trigger_imbalance_ratio=0.20,
+        trigger_imbalance_ratio=0.10,
         min_seconds_between_triggers=0.0,
     ),
```

The sample's two snapshot ratios are 0.500 and 0.750. Both are above 0.10, so the trigger count is
still zero. Lowering the threshold only matters when some observed ratio falls between the old and
new thresholds. To see a difference you would need a longer stream; the in-memory fixture of lecture
06 reaches ratio 0.040 and 0.080, so under 0.10 it still fires, while under 0.05 only the first
would fire. Use that fixture to experiment.

## Exercise 3: depth-weighted imbalance

The top-of-book signal ignores everything below the best price. Replace the best sizes with the sum
of the top five levels on each side.

**Solution.** Add this after the book is built, using the loader from lecture 05:

```python
bid_sum = sum(level.size() for level in book.bids(5))
ask_sum = sum(level.size() for level in book.asks(5))
print("top-5 bid size:", bid_sum)
print("top-5 ask size:", ask_sum)
print("depth ratio:", f"{min(bid_sum, ask_sum) / max(bid_sum, ask_sum):.3f}")
```

Run against the committed sample, whose book has exactly two levels per side. Real output:

```text
top-5 bid size: 4.0
top-5 ask size: 6.0
depth ratio: 0.667
top-of-book ratio: 0.750
```

The top-of-book ratio is 0.750 (bid 3, ask 4) but the five-level ratio is 0.667, because the resting
bid at 50099.0 for 1.0 adds to the bid side. The two numbers differ; a signal that used the top
five levels would behave differently from the top-of-book strategy even though both read the same
book. `BookLevel.size()` returns a `float`; the best-of-book accessors return `Quantity`, which is
why the two code paths look different.

## Exercise 4: add a position limit

The lecture 05 strategy can keep adding to a position. Add a check so that it never holds more than
2 units (long or short).

**Solution.** Add a bound to the config and a guard to `on_book_deltas`:

```diff
 class OrderBookImbalanceConfig(StrategyConfig):
     def __init__(
         self,
         *,
         instrument_id,
         max_trade_size,
+        max_position="2",
         trigger_min_size=100.0,
@@
         self.max_trade_size = max_trade_size
+        self.max_position = max_position
@@
 class OrderBookImbalance(Strategy):
     def __init__(self, config):
@@
         self._max_trade_size = Decimal(config.max_trade_size)
+        self._max_position = Decimal(config.max_position)
@@
         if bid > ask:
             side, price, level_size = OrderSide.BUY, book.best_ask_price(), ask
         else:
             side, price, level_size = OrderSide.SELL, book.best_bid_price(), bid
         if price is None:
             return
+        net = self.portfolio.net_position(self._instrument_id)
+        if net is not None:
+            net = Decimal(net)
+            if side == OrderSide.BUY and net >= self._max_position:
+                return
+            if side == OrderSide.SELL and net <= -self._max_position:
+                return
         self._last_trigger_ns = now
```

`self.portfolio.net_position(...)` returns the current net position in the instrument's base units.
The guard refuses to add in the direction that would break the bound. Note this is your strategy's
own check; the engine does not enforce a position limit (lecture 07).

## Exercise 5: make the loader handle negative amounts

Some feeds signal a removal with a negative amount instead of zero. Change `load_deltas` so that
any amount less than or equal to zero becomes a `DELETE`.

**Solution.** The existing `else` branch already catches everything that is not a positive amount,
including negative values, so the code is already correct. This is worth knowing: the condition
that matters is `amount > 0` for an update, and everything else is a removal. If you prefer the
intent to be explicit, split the two removal cases:

```diff
-        elif amount > 0:
-            action = BookAction.UPDATE
-            flags = int(RecordFlag.F_LAST.value)
-        else:
-            action = BookAction.DELETE
-            flags = int(RecordFlag.F_LAST.value)
+        elif amount > 0:
+            action = BookAction.UPDATE
+            flags = int(RecordFlag.F_LAST.value)
+        elif amount == 0:
+            action = BookAction.DELETE
+            flags = int(RecordFlag.F_LAST.value)
+        else:
+            # A negative amount is also a removal.
+            action = BookAction.DELETE
+            flags = int(RecordFlag.F_LAST.value)
```

Both forms produce identical books. The dangerous form is the opposite mistake: treating `amount < 0`
as an update, which would add a negative size and corrupt the level.

## Exercise 6: choose the right book for a quote feed

A venue publishes only top-of-book quotes and trades, no depth. Which book type do you maintain, and
which subscription method do you call? What can you and cannot you compute?

**Solution.** You maintain `L1_MBP`. `QuoteTick` and `TradeTick` can drive an `L1_MBP` book
(`docs/concepts/order_book.md`, `docs/concepts/data/quote_tick.md`). In a strategy you subscribe with
`self.subscribe_quotes(instrument_id)` and read `quote.bid_size` / `quote.ask_size` directly, as
`examples/live/architect_ax/strategies.py` does. You can compute the top-of-book ratio and the
spread. You cannot compute anything that needs level two or deeper: you have no per-level sizes, no
depth-weighted imbalance, and no `get_quantity_at_level`-style access. The Ax example is exactly
this case, which is why it uses `mbp-1` quote data rather than a full L2 feed.

## Exercise 7 (break it on purpose): remove the snapshot clear

Delete the `CLEAR` that the loader emits before a new snapshot. Run the reconstruction from lecture
05 and explain every difference in the final book.

**Solution.** Remove this block from `load_deltas`:

```diff
-        if is_snapshot and not previous_was_snapshot:
-            deltas.append(
-                OrderBookDelta(
-                    instrument.id,
-                    BookAction.CLEAR,
-                    BookOrder(
-                        OrderSide.BUY,
-                        Price.from_decimal_dp(Decimal("0"), instrument.price_precision),
-                        Quantity.from_decimal_dp(Decimal("0"), instrument.size_precision),
-                        0,
-                    ),
-                    int(RecordFlag.F_SNAPSHOT.value),
-                    sequence,
-                    ts_ns,
-                    ts_ns,
-                ),
-            )
```

Real output of the broken run:

```text
bids: {Decimal('50100.0'): Decimal('3.000'), Decimal('50099.0'): Decimal('1.000'), Decimal('50000.0'): Decimal('1.000'), Decimal('49999.0'): Decimal('0.500')}
asks: {Decimal('50001.0'): Decimal('2.000'), Decimal('50002.0'): Decimal('1.500'), Decimal('50101.0'): Decimal('4.000'), Decimal('50102.0'): Decimal('2.000')}
top-5 bid size: 5.5
top-5 ask size: 9.5
depth ratio: 0.579
top-of-book ratio: 0.667
```

Four differences, and each one matters:

1. **Stale levels survive.** The first snapshot's bid at 50000.0 and ask at 50001.0, and the earlier
   updates at 49999.0 and 50002.0, are still in the book. The real book at that instant had two
   levels per side.
2. **Two levels per side became four.** The two clears that would have removed the old levels are
   missing.
3. **The book is crossed.** The best bid is 50100.0, but the stale best ask is 50001.0 - a bid above
   an ask. The engine applied the deltas anyway, because out-of-order and snapshot deltas are
   applied rather than rejected (`docs/concepts/order_book.md`). `book.check_integrity()` would flag
   this.
4. **The signal is wrong.** The top-of-book ratio is 0.667 (bid 3.0 against the *stale* ask 2.0 at
   50001.0), not the correct 0.750 (bid 3.0 against ask 4.0 at 50101.0). A strategy reading this
   book would buy at 50001.0, which no longer exists in the real market.

This is the single most common order book bug, and it fails silently. Always emit the clear, and
always call `check_integrity()` after a rebuild.

## Exercise 8: prove the unit bug

Remove the `* 1000` from `ts_ns = int(row["timestamp"]) * 1000` and print `book.ts_last`. What year
does it correspond to, and what does that tell you about how the engine uses timestamps?

**Solution.** The fixture `timestamp` is a microsecond epoch. Without the multiplication, the values
are interpreted as nanoseconds, so a 2022 date becomes a 1970 date. Printing `book.ts_last` would
show `1640995302000000`, which as nanoseconds is 1970-01-19. The engine does not correct this; it
stores and orders events by the number it is given. A wrong unit silently reorders your whole
session. Lecture 04 and lecture 07 both cover this.

## Next

[09](09-go-further.md) lists the honest gaps and the pages to read after this manual.
