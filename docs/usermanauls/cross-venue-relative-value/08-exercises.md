# 08 - Exercises

Each exercise gives the exact lines to change, the observed result, and the reason it happened. The
starting point for every exercise is the complete program in `05-build-the-strategy.md`, section 11.
Run each one from a scratch file outside the repository.

Unless stated otherwise, the baseline is:

```text
binance_pnl=-107.20016 bybit_pnl=22.03454 net_pnl=-85.16562
```

## Exercise 1: widen the entry, remove the exit

Change the two threshold values in `build_engine`. The changed lines are exactly:

```diff
-                entry_bps=Decimal("12"),
-                exit_bps=Decimal("5"),
+                entry_bps=Decimal("14"),
+                exit_bps=Decimal("0"),
```

Observed output (the decision lines and the summary):

```text
[1] ENTER spread_bps=14.04
binance_pnl=-25.61576 bybit_pnl=-38.47680 net_pnl=-64.09256
```

Why: the spread never falls to 0 bps in this fixture, so the exit never fires and the pair is still
open when the data ends. The positions report confirms it:

```text
OPEN? BTCUSDT-PERP.BINANCE is_open=True is_closed=False qty=1.000 realized=-25.61576000 USDT
OPEN? BTCUSDT-PERP.BYBIT is_open=True is_closed=False qty=1.000 realized=-38.47680000 USDT
```

The two account totals in that run differ from the starting balances by exactly the two entry
commissions. An open position's price move is **unrealised**; it is not in the account balance, and
the engine reports it through `position.unrealized_pnl(price)` and the portfolio, not through the
cash balance. Reading `net_pnl` from an incomplete round trip and comparing it to a completed one is
the fourth way to misread a two-leg result.

## Exercise 2: halve the trade size

```diff
-                trade_size=Decimal("1.000"),
+                trade_size=Decimal("0.500"),
```

Observed output:

```text
[1] ENTER spread_bps=12.04
[1] EXIT spread_bps=4.94
binance_pnl=-53.60008 bybit_pnl=11.01727 net_pnl=-42.58281
```

Why: every term in the result is linear in quantity. `-85.16562 / 2 = -42.58281` exactly. Halving
the size halves the loss; it does not change the sign. Price and fee effects scale, so size is not
a lever for profitability.

## Exercise 3: make both venues equally cheap

Change the BYBIT taker rate to match BINANCE:

```diff
-    BYBIT: (Decimal("0.0002"), Decimal("0.0006")),
+    BYBIT: (Decimal("0.0002"), Decimal("0.0002")),
```

Observed output:

```text
[1] ENTER spread_bps=12.04
[1] EXIT spread_bps=4.94
binance_pnl=-107.20016 bybit_pnl=73.27818 net_pnl=-33.92198
```

Why: only the BYBIT leg changed, from `-76.86546` of fees to `-25.62182`, a saving of `51.24364`.
The round-trip cost falls from 20 bps to 8 bps and the trade is still unprofitable. The captured
basis was about 6.7 bps, so even at 8 bps of cost the trade loses. This is the arithmetic that
decides whether a relative-value strategy is viable, and it is why lecture 07 says to compare the
captured basis against `2 * (fee_A + fee_B)` before trading anything.

## Exercise 4: deny the second leg instead of the first

```diff
-NOTIONAL_CAP = None  # set to a dict to switch the first leg's risk cap on
+NOTIONAL_CAP = {"BTCUSDT-PERP.BYBIT": "1000"}
```

Observed output:

```text
[1] ENTER spread_bps=12.04
DENIED BTCUSDT-PERP.BYBIT O-20231114-221343-001-000-2 reason=NOTIONAL_EXCEEDS_MAX_PER_ORDER: max=1000.00000000 USDT, notional=64104.00000000 USDT
[1] EXIT spread_bps=4.94
DENIED BTCUSDT-PERP.BYBIT O-20231114-221606-001-000-4 reason=NOTIONAL_EXCEEDS_MAX_PER_ORDER: max=1000.00000000 USDT, notional=64005.10000000 USDT
binance_pnl=-107.20016 bybit_pnl=0.00000 net_pnl=-107.20016
```

Why: this is the mirror of lecture 05, section 10. Now the Binance leg is the one that lives and
the Bybit leg is denied, so the strategy holds a naked 1 BTC long for the whole window and loses
`107.20` instead of `85.17`. The direction of the loss depends on which leg is denied, which is
exactly why leg risk cannot be reasoned about after the fact. Note that the denied order id `-2` is
the Bybit leg, and the fills report contains only the two Binance orders.

## Exercise 5: signal on the executable spread, not the mid spread

The mid spread is not tradeable. Change the signal to the touch version: buy the cheap venue at its
ask, sell the rich venue at its bid.

```diff
         spread_bps = (mid(bybit) - mid(binance)) / mid(binance) * Decimal(10_000)
+        touch_bps = (
+            bybit.bid_price.as_decimal() - binance.ask_price.as_decimal()
+        ) / binance.ask_price.as_decimal() * Decimal(10_000)
         self._series.append(spread_bps)

-        if not self._open and spread_bps >= self.config.entry_bps:
+        if not self._open and touch_bps >= self.config.entry_bps:
             self._entries += 1
-            print(f"[{self._entries}] ENTER spread_bps={spread_bps:.2f}")
+            print(f"[{self._entries}] ENTER touch_bps={touch_bps:.2f}")
```

Why: with `entry_bps = Decimal("12")` and the touch spread, the entry fires later than before,
because the touch spread is about 0.2 bps below the mid spread in this fixture. The point of the
exercise is not the small delay; it is that the number your strategy reacts to should be a number
you could have traded at. Lecture 07, section 7, gives the three-number version of this check.

## Exercise 6: add funding to the pair

Give the perpetual on each venue a funding rate, so the carry is part of the trade while the pair is
open. Add the import:

```diff
 from nautilus_trader.model import Currency
+from nautilus_trader.model import FundingRateUpdate
 from nautilus_trader.model import InstrumentId
```

and add two data elements after the quote feeds:

```diff
     engine.add_data(load_quotes("btcusdt_perp_bybit_quotes.csv", BTCUSDT_PERP_BYBIT.id))
+    start_ns = 1_700_000_000_000_000_000
+    engine.add_data(
+        [
+            FundingRateUpdate(
+                instrument_id=BTCUSDT_PERP_BINANCE.id,
+                rate=Decimal("-0.0001"),
+                ts_event=start_ns,
+                ts_init=start_ns,
+                interval=480,
+                next_funding_ns=start_ns + 120_000_000_000,
+            ),
+            FundingRateUpdate(
+                instrument_id=BTCUSDT_PERP_BYBIT.id,
+                rate=Decimal("0.0001"),
+                ts_event=start_ns,
+                ts_init=start_ns,
+                interval=480,
+                next_funding_ns=start_ns + 120_000_000_000,
+            ),
+        ],
+    )
```

Observed output:

```text
[1] ENTER spread_bps=12.04
[1] EXIT spread_bps=4.94
binance_pnl=-100.80319 bybit_pnl=28.43907 net_pnl=-72.36412
```

Why: the pair is opened at `22:13:43` and closed at `22:16:06`, so the funding boundary at
`22:15:20` falls inside the holding period. A negative Binance rate means longs receive, and the
long Binance leg gained `6.39697`. A positive Bybit rate means shorts receive, and the short Bybit
leg gained `6.40453`. Both legs received, `12.8015` in total, and the loss shrank from `85.17` to
`72.36`. The two rates were chosen with opposite signs to show the mechanism in both directions; on
a real pair the sign is whatever the two venues publish, and it can be against you on both legs.

Lecture 06, section 5, shows the same mechanism one leg at a time, with the `FUNDING` position
adjustment printed directly.

## Exercise 7: break it on purpose

Remove the timestamp alignment check and see what the strategy reacts to.

```diff
-        if binance.ts_event != bybit.ts_event:
-            return  # not yet aligned: wait for the same instant on both venues
```

Observed output:

```text
[1] ENTER spread_bps=12.04
[1] EXIT spread_bps=4.92
spread_bps: 479 aligned observations, every 20th shown
  8.59 10.14 11.62 12.93 14.04 14.88 15.41 15.62 15.46 15.00 14.19 13.11 11.82 10.37 8.82 7.27 5.77 4.41 3.28 2.39 1.81 1.56 1.67 2.11
  min=1.41 mean=9.16 max=15.78
binance_pnl=-107.20016 bybit_pnl=22.13460 net_pnl=-85.06556
```

What broke:

1. The strategy evaluated the spread **479** times instead of **240**. The count in the printed line
   was produced by the same counter that normally counts aligned observations, so it is a direct
   measurement, not an estimate.
2. The spread series changed. With the alignment check the extremes were `min=1.56` and `max=15.62`;
   without it they are `min=1.41` and `max=15.78`. The series your thresholds are tuned on therefore
   depends on a line of code that has nothing to do with the market.
3. The execution changed. The exit fired at `4.92` instead of `4.94`, and the Bybit leg finished
   `22.13460` instead of `22.03454`. A hundredth of a basis point of signal drift moved real money.
4. Worst of all, the strategy is now free to act on a basis computed from one fresh and one stale
   quote. In this fixture the staleness is at most one second. In production, a venue that has not
   ticked for a minute will produce a basis that no participant can trade, and the strategy will
   trade it.

The fix is to restore the three lines. The deeper lesson is the one in
`docs/design/relative_value_screening.md`, section 2.8: the spread is defined by the windows and
conventions you choose, so those conventions belong in the strategy's declared parameters, not in
an accident of event ordering.

Continue to [09-go-further.md](09-go-further.md).
