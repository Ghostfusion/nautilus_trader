# Measure and evaluate

A backtest that produced no fills taught you nothing about the signal. This lecture measures a run
that does fire, using two tools the repository gives you:

1. The builtin `BookImbalanceActor`, which watches a replayed book and prints cumulative bid and
   ask volume.
2. The engine's Python reports - account, order fills and positions - which tell you what the run
   actually traded.

Then it is honest about the third tool: **execution analytics is Rust only in this repository.** You
cannot call it from Python. The lecture shows the crate and runs its own test instead of pretending
a Python API exists.

## 1. The builtin imbalance actor

`BookImbalanceActor` is registered by name in both the backtest engine
(`crates/backtest/src/python/engine.rs`, `builtin_actor_register`) and the live node
(`crates/live/src/python/node.rs`). It subscribes to `L2_MBP` deltas, sums the resting size at each
updated level per side, accumulates running totals, and prints a summary when it stops. Its
implementation is `crates/trading/src/examples/actors/imbalance/actor.rs`; its config is
`crates/trading/src/examples/actors/imbalance/config.rs`.

Add it to the program from lecture 05, replacing the strategy with the actor:

```python
from nautilus_trader.trading import BookImbalanceActorConfig

engine.add_builtin_actor(
    "BookImbalanceActor",
    BookImbalanceActorConfig(instrument_ids=[instrument.id], log_interval=0),
)
```

The full program is the loader from lecture 05, then this venue setup, then the actor. Run it and
the actor prints one line per instrument:

```text
--- Book imbalance summary ---
  BTCUSDT-PERP.BINANCE  updates: 11  bid_vol: 5.50  ask_vol: 9.50  imbalance: -0.2667
```

Read it:

- `updates: 11` counts the delta batches the actor saw, one per applied delta.
- `bid_vol: 5.50` is the cumulative bid-side size it summed across all of them.
- `ask_vol: 9.50` is the same for the ask side.
- `imbalance: -0.2667` is `(bid_vol - ask_vol) / (bid_vol + ask_vol) = (5.5 - 9.5) / 15.0`. Negative
  means the ask side dominated this sample.

This number is a measurement of the *stream*, not of a strategy: it says the replayed book supplied
more ask size than bid size. It is the same quantity lecture 01 called imbalance, accumulated over
time instead of measured at one instant.

A builtin **strategy** registered by name, such as `HurstVpinDirectional` (confirmed in
`crates/backtest/src/python/engine.rs` and `crates/live/src/python/node.rs`), reports through the
same engine reports as the custom strategy: `generate_account_report`, `generate_order_fills_report`
and `generate_positions_report`. There is no separate builtin-only report to read; the strategy is
registered with `engine.add_builtin_strategy("HurstVpinDirectional", config)` and its output is the
ordinary set of reports plus its own log lines.

## 2. A run that fires

The committed sample is too small to trade. To measure a firing run, build a short delta sequence in
memory. This is a teaching fixture, not the committed sample; it is hand-built and deterministic.
The interesting part is the four price changes after the opening snapshot.

```python
T0 = 1_640_995_200_000_000_000
SECOND = 1_000_000_000


def delta(action, side, price, size, snap=False, ts=0):
    return OrderBookDelta(
        instrument_id,
        action,
        BookOrder(
            side,
            Price.from_decimal_dp(Decimal(price), instrument.price_precision),
            Quantity.from_decimal_dp(Decimal(size), instrument.size_precision),
            0,
        ),
        int(RecordFlag.F_SNAPSHOT.value) if snap else int(RecordFlag.F_LAST.value),
        1,
        ts,
        ts,
    )


deltas = [
    delta(BookAction.CLEAR, OrderSide.BUY, "0", "0", snap=True, ts=T0),
    delta(BookAction.ADD, OrderSide.BUY, "50000.0", "5", snap=True, ts=T0),
    delta(BookAction.ADD, OrderSide.SELL, "50010.0", "5", snap=True, ts=T0),
    delta(BookAction.UPDATE, OrderSide.SELL, "50010.0", "0.2", ts=T0 + 10 * SECOND),
    delta(BookAction.UPDATE, OrderSide.SELL, "50010.0", "5", ts=T0 + 12 * SECOND),
    delta(BookAction.UPDATE, OrderSide.BUY, "50000.0", "0.2", ts=T0 + 16 * SECOND),
    delta(BookAction.UPDATE, OrderSide.BUY, "50000.0", "5", ts=T0 + 18 * SECOND),
]
```

The first change thins the ask to 0.2 against a 5.0 bid: ratio 0.040, so the strategy buys at the
ask. The second change restores the ask, closing the imbalance. The third thins the bid to 0.2
against a 5.0 ask: the strategy sells at the bid. The fourth restores the bid. Feed `deltas` to the
engine with `engine.add_data(deltas)` instead of the CSV, add the strategy, and run. The strategy
prints its trigger count and then the reports:

```python
print("triggers:", strategy.trigger_count)
fills = engine.generate_order_fills_report()
print(fills[["instrument_id", "side", "quantity", "avg_px", "commissions", "liquidity_side", "ts_last"]].to_string())
positions = engine.generate_positions_report()
print(positions[["instrument_id", "side", "quantity", "avg_px_open", "avg_px_close", "realized_pnl"]].to_string())
print(engine.generate_account_report(BINANCE)[["total", "locked", "free", "base_currency"]].to_string())
```

Real output:

```text
triggers: 2
                                    instrument_id  side quantity   avg_px        commissions liquidity_side                   ts_last
client_order_id
O-20220101-000010-001-000-1  BTCUSDT-PERP.BINANCE   BUY    0.200  50010.0  [5.00100000 USDT]          TAKER 2022-01-01 00:00:10+00:00
O-20220101-000016-001-000-2  BTCUSDT-PERP.BINANCE  SELL    0.200  50000.0  [5.00000000 USDT]          TAKER 2022-01-01 00:00:16+00:00
                                                    instrument_id  side quantity  avg_px_open  avg_px_close       realized_pnl
position_id
BTCUSDT-PERP.BINANCE-OrderBookImbalance-000  BTCUSDT-PERP.BINANCE  FLAT    0.000      50010.0       50000.0  -12.00100000 USDT
                                     total       locked             free base_currency
2022-01-01 00:00:00+00:00  100000.00000000   0.00000000  100000.00000000          USDT
2022-01-01 00:00:10+00:00  100000.00000000  50.01000000   99949.99000000          USDT
2022-01-01 00:00:10+00:00   99994.99900000  25.00500000   99969.99400000          USDT
2022-01-01 00:00:16+00:00   99994.99900000  75.00500000   99919.99400000          USDT
2022-01-01 00:00:16+00:00   99987.99900000   0.00000000   99987.99900000          USDT
```

## 3. What each number means

**Fills report.** One row per fill.

| Column           | Meaning                         | Good                             | Bad                                 |
| ---------------- | ------------------------------- | -------------------------------- | ----------------------------------- |
| `side`           | Buy or sell.                    | -                                | -                                   |
| `quantity`       | Units filled.                   | Small enough to exit.            | Larger than the level could absorb. |
| `avg_px`         | Average fill price.             | Buys below the mid, sells above. | Buys above the mid.                 |
| `commissions`    | Fee paid, as a list of `Money`. | Zero or tiny.                    | Eats the whole edge.                |
| `liquidity_side` | `TAKER` or `MAKER`.             | `MAKER` earns the spread.        | `TAKER` pays it.                    |

The run bought 0.2 at 50010.0 and sold 0.2 at 50000.0. Both fills are `TAKER`: the strategy crossed
the spread twice, once each way.

**Positions report.** One row per position. `avg_px_open` is the average price the position was
opened at, `avg_px_close` the average it was closed at, and `realized_pnl` the profit or loss in
the account currency. Here the position is `FLAT` (fully closed) with `realized_pnl` of
`-12.00100000 USDT`. The loss is the price move against the position plus fees:

```
buy  0.2 at 50010.0  ->  cost 10002.00
sell 0.2 at 50000.0  ->  proceeds 10000.00
gross loss = 10002.00 - 10000.00 = 2.00
commissions = 5.001 + 5.000 = 10.001
total loss = 2.00 + 10.001 = 12.001 USDT
```

That is the whole economic story of a naive imbalance strategy: it bought because the book leaned
up, the book then leaned down, and it sold lower. The fee model alone turned a 2 USDT gross loss
into a 12 USDT net loss.

**Account report.** One row per account state change. `total` is total equity, `locked` is margin
locked by working orders, `free` is available. You can watch a lock appear when an order is
submitted (`50.01000000` at 00:00:10) and release when it fills. `total` falls from 100000.00 to
99987.999 - the 12.001 USDT loss, to the cent.

A good value for `realized_pnl` is positive and larger than total commissions. A bad value is
negative, or positive but smaller than commissions. This run is the second kind twice over.

## 4. Execution analytics is Rust only

The concepts behind the numbers above - implementation shortfall, arrival slippage, VWAP and TWAP
slippage, midpoint slippage, spread capture, adverse selection, fill ratio, cancel ratio, child
count and child churn - are implemented in this repository as **read-only execution analytics**,
and there is **no Python binding for them**. The crate is `crates/trading/src/analytics/`. It is
documented in `docs/design/vnpy_lessons_implementation.md` (section 4.13, decision D14) and
`docs/design/vnpy_lessons_design.md` (decision D14). The execution algorithms that the analytics
measure are documented in `docs/concepts/execution/algorithms.md`.

The module observes an execution and reports what happened; it never changes it. Every value
carries a `MetricDeclaration` naming its units, direction, reference price, denominator and
reference timestamp, so a number cannot be read without the convention that defines it. An
undefined metric is reported as `MetricValue::NotAvailable` with a reason, never as `0.0`.

Because it is Rust only, the manual does not show a Python snippet that would not work. Instead,
run the crate's own test, which pins the arithmetic. From the repository root:

```bash
export CARGO_TARGET_DIR='<repo-root>/target'
cargo nextest run --locked -p nautilus-trading --features python -E 'test(pinned_metrics_match_hand_computed_values)'
```

The test is `pinned_metrics_match_hand_computed_values` in `crates/trading/src/analytics/mod.rs`. It
builds an execution with a decision price of 100.00, an arrival price of 101.00, two fills at
102.00 and 102.02, and then asserts every metric against arithmetic printed beside it in the source.
The fill VWAP is `(102.00 * 50 + 102.02 * 50) / 100 = 102.01`, and the implementation shortfall
against the decision price is `((102.01 - 100.00) / 100.00) * 10_000 = 201` basis points.

The observed result of the command was:

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 22m 51s
 Nextest run ID e0fc31dc-3641-4961-ac7b-1367c26b282c with nextest profile: default
    Starting 1 test across 1 binary (968 tests skipped)
        PASS [   0.035s] (1/1) nautilus-trading analytics::tests::pinned_metrics_match_hand_computed_values
     Summary [   0.042s] 1 test run: 1 passed, 968 skipped
```

The first build takes about twenty minutes; later runs are fast. `nextest` normally surrounds the
run with separator lines drawn using box-drawing characters, which are omitted here to keep this
file ASCII. The `968 skipped` count is the rest of the crate's tests, filtered out by `-E`.

For a backtest of this style, the practical substitutes are the reports you read above and the
actor's stream summary. They are coarser than the Rust analytics, but they are what the Python user
has.

## 5. Three common beginner misreadings

**Misreading 1: "The strategy made a positive realized pnl, so the signal works."** A single firing
run with two fills is not evidence. `realized_pnl` is dominated by which way the price happened to
move in those seconds. To say anything about the signal you need many trades, across sessions and
venues, and you must compare against a run with the signal disabled. The repository's own tutorial
reports 47 orders and a net short on one day, and separately reports the gold example bleeding
steadily for a day. One day is not a result.

**Misreading 2: "`liquidity_side: TAKER` is fine because the fill happened."** A taker fill means
you paid the spread. For a strategy that triggers on every imbalance, spread cost is the dominant
term. Read `commissions` and the difference between `avg_px` and the midpoint before anything else.
In the run above, the 10.001 USDT of fees is five times the 2 USDT the price moved against the
position.

**Misreading 3: "`BookImbalanceActor` says imbalance is -0.27, so I should short."** The actor
accumulates quoted volume across every update it sees. A single level updated many times contributes
many times, so the total is a property of the feed's message pattern as much as of the market. It is
a diagnostic, not a trading signal. If you want a tradable imbalance, compute it from the current
book state at each decision instant, as the strategy of lecture 05 does.

## Next

[07](07-risks-and-limits.md) covers what breaks these measurements in production.
