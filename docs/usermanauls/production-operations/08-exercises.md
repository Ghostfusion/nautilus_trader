# 08 - Exercises

Work each exercise before reading its solution. The solutions are given as changed lines, not as
whole new programs. Two exercises are checked by a program whose real output is pasted here.

## Exercise 1: the largest order under a cap

You trade AUD/USD at an ask of 0.71010 and you set `max_notional_per_order` to 100,000 USD. What is
the largest whole-unit quantity you may submit, and what is its notional? Check both directions.

### Solution

```text
100,000 / 0.71010 = 140,825.24 units
140,825 x 0.71010 = 99,999.83 USD   <= 100,000, passes
140,826 x 0.71010 = 100,000.54 USD  >  100,000, refused
```

The largest whole-unit quantity is 140,825. Verified by direct arithmetic: the two products are
99,999.83249999999 and 100,000.5426.

## Exercise 2: three bursts instead of one

Take the `RunawayLoop` strategy from lecture 03 and make it fire its burst on each of the first three
quote ticks rather than only the first. The rate limit stays `2/00:00:01`. How many of the 18 orders
are admitted?

### Solution

Change the guard from a one-shot flag to a counter:

```diff
 class RunawayLoop(Strategy):
     def __init__(self, config: RunawayConfig) -> None:
         super().__init__(config)
         self.denials: list[str] = []
-        self.fired = False
+        self.bursts_done = 0

     def on_quote(self, _tick: QuoteTick) -> None:
-        if self.fired:
+        if self.bursts_done >= 3:
             return
-        self.fired = True
+        self.bursts_done += 1
```

Observed output of a program built on this change:

```text
bursts                    : 3
orders attempted          : 18
orders denied             : 12
orders that reached venue : 6
```

Six orders are admitted, two per one-second interval, and twelve are refused. The rate limit is a
rolling budget, so a burst at each new second gets a fresh allowance.

## Exercise 3: read a rate limit string

What does `max_order_submit_rate="10/00:00:05"` mean in orders per second?

### Solution

The format is `limit/HH:MM:SS`. The interval `00:00:05` is five seconds and the limit is ten, so:

```text
10 / 5 = 2 submits per second
```

A burst of three orders at the same instant would therefore be refused at the third order.

## Exercise 4: add state hooks to the counter

Add `on_save` and `on_load` to the `Counter` strategy from lecture 07 so that `quote_count` and
`high` survive a restart. State what happens if you forget the cache backing.

### Solution

```python
    def on_save(self) -> dict:
        return {
            "quote_count": str(self.quote_count).encode(),
            "high": str(self.high).encode(),
        }

    def on_load(self, state: dict) -> None:
        self.quote_count = int(state["quote_count"].decode())
        self.high = Decimal(state["high"].decode())
```

Without a cache database backing, the kernel warns that `load_state` and `save_state` have no effect
and neither hook fires. The state round-trips correctly when called directly, which shows the
encoding is right and the wiring is what is missing.

## Exercise 5: reconcile the sample report

`sample_data/venue_fills_report.csv` contains two buys (100,000 and 150,000 units) and one sell
(50,000 units). Your cached position says 150,000 units long. What does reconciliation do?

### Solution

By hand:

```text
100,000 + 150,000 - 50,000 = 200,000 units long
```

The venue says 200,000 and the cache says 150,000, so the cache is 50,000 units short. This is the
long position quantity mismatch scenario: the engine generates a BUY LIMIT for the difference when
`generate_missing_orders` is enabled, and tags it `RECONCILIATION`. If the mismatch ran the other way
the engine would generate a SELL LIMIT, and if the sides disagreed it would flip the position.

## Exercise 6: choose a monitor threshold

You set `QueueMonitorConfig(queue_depth_trigger=1_000, queue_depth_clear=500)`. Queue depth readings
are 400, 1,200, 900, 700, 400. For each reading, does the node publish an event?

### Solution

| Reading | Comparison                | Result                                              |
| ------- | ------------------------- | --------------------------------------------------- |
| 400     | below clear 500           | No event while not triggered.                       |
| 1,200   | at or above 1,000         | Publishes `Backlogged` `Triggered`.                 |
| 900     | between clear and trigger | Between the thresholds: prior state kept, no event. |
| 700     | between clear and trigger | No event.                                           |
| 400     | at or below 500           | Publishes `Backlogged` `Cleared`.                   |

A value between the thresholds retains the prior state, so no duplicate event is published. Note that
each clear threshold must be lower than its trigger threshold, or configuration validation rejects
the pair.

## Exercise 7: break it on purpose

Set `bypass=True` on the risk engine and run exercise 2's program again. The rate limit is still
`2/00:00:01`.

### Solution

```diff
     RiskEngineConfig(
+        bypass=True,
         max_order_submit_rate="2/00:00:01",
     ),
```

Observed output:

```text
EXERCISE 7: three bursts of six, risk engine bypassed
bursts                    : 3
orders attempted          : 18
orders denied             : 0
orders that reached venue : 18
```

All 18 orders reach the venue. `bypass=True` skips the risk checks and the order rate limits, so the
limit you wrote in the same configuration does nothing. This is the single most dangerous setting in
this manual.

**Do not leave `bypass=True` in anything that will run unattended.** It is convenient while
developing a strategy and catastrophic in production. Note that the sandbox scripts in lecture 05 use
`bypass=True` deliberately, because they are demonstrating execution rather than risk.

## Exercise 8: write the alert you would want

You run a strategy overnight. Write the one alert, in one sentence, that you would want to receive if
the connection to the venue were lost at 02:00 and the strategy kept trying to trade.

### Solution

"I want to know, within one minute, that the venue transport is disconnected, which strategy is
affected, and whether any order outcome is still unknown."

The three parts map to real signals: the transport state from a `SocketStateChanged` event, the
strategy identity carried on a notification event, and the unknown outcome that only reconciliation
can resolve. A message that omits any of the three leaves you guessing at 02:00.

Continue to [09 - Go further](09-go-further.md).