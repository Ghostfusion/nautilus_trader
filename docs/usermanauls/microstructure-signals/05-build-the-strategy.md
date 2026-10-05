# Build the imbalance strategy

This lecture rebuilds a book from the committed deltas, computes the imbalance ratio step by step,
turns the ratio into a trading decision, and runs a complete backtest. Each step is numbered, each
step has its code, and each code block is followed by the real output of running it.

The strategy itself is the repository's own teaching strategy. Its source of truth is
`docs/tutorials/orderbook_imbalance.py`; the version here is the same algorithm written out for
reading. It has **no edge** and is not meant to trade real money - the repository says so in
`examples/backtest/architect_ax_book_imbalance.py`.

## Step 1: turn CSV rows into engine objects

The engine does not read the CSV. You do, and you produce `OrderBookDelta` objects. Save this loader
as part of `build_book.py` outside the repository.

```python
import csv
from decimal import Decimal

from nautilus_trader.testkit.providers import TestInstrumentProvider
from nautilus_trader.model import (
    BookAction,
    BookOrder,
    BookType,
    InstrumentId,
    OrderBook,
    OrderBookDelta,
    OrderSide,
    Price,
    Quantity,
    RecordFlag,
)


def load_deltas(path, instrument):
    """Convert a Tardis-style CSV into engine OrderBookDelta objects."""
    with open(path, newline="") as f:
        rows = list(csv.DictReader(f))

    deltas = []
    previous_was_snapshot = False
    sequence = 0

    for row in rows:
        sequence += 1
        is_snapshot = row["is_snapshot"] == "true"
        side = OrderSide.BUY if row["side"] == "bid" else OrderSide.SELL
        amount = Decimal(row["amount"])
        ts_ns = int(row["timestamp"]) * 1000  # fixture timestamps are microseconds

        if is_snapshot and not previous_was_snapshot:
            deltas.append(
                OrderBookDelta(
                    instrument.id,
                    BookAction.CLEAR,
                    BookOrder(
                        OrderSide.BUY,
                        Price.from_decimal_dp(Decimal("0"), instrument.price_precision),
                        Quantity.from_decimal_dp(Decimal("0"), instrument.size_precision),
                        0,
                    ),
                    int(RecordFlag.F_SNAPSHOT.value),
                    sequence,
                    ts_ns,
                    ts_ns,
                ),
            )

        if is_snapshot:
            action = BookAction.ADD
            flags = int(RecordFlag.F_SNAPSHOT.value)
        elif amount > 0:
            action = BookAction.UPDATE
            flags = int(RecordFlag.F_LAST.value)
        else:
            action = BookAction.DELETE
            flags = int(RecordFlag.F_LAST.value)

        deltas.append(
            OrderBookDelta(
                instrument.id,
                action,
                BookOrder(
                    side,
                    Price.from_decimal_dp(Decimal(row["price"]), instrument.price_precision),
                    Quantity.from_decimal_dp(Decimal(row["amount"]), instrument.size_precision),
                    0,
                ),
                flags,
                sequence,
                ts_ns,
                ts_ns,
            ),
        )
        previous_was_snapshot = is_snapshot

    return deltas
```

This is the most important code in the manual. Four decisions inside it:

1. **A new snapshot emits a `CLEAR` first.** When `is_snapshot` turns true after having been false,
   the loader first appends a `BookAction.CLEAR` delta. Without it, the second snapshot's levels
   are added on top of the old book, and cancelled levels from the first snapshot survive. The
   engine applies deltas in order, so the clear must come first.
2. **`is_snapshot` maps to `ADD` with `F_SNAPSHOT`.** Snapshot rows are full levels, not changes.
3. **`amount == 0` maps to `DELETE`.** As lecture 04 explained, Tardis signals a removal with size
   zero.
4. **Prices and sizes are quantised to the instrument's precision.** `Price.from_decimal_dp(value,
   precision)` rounds to the instrument's price precision, and the size equivalent does the same.
   The instrument comes from `TestInstrumentProvider.btcusdt_perp_binance()`. If you skip this and
   use `Price.from_str`, the engine raises an error beginning `Invalid delta order size precision 1,
   expected 3`, because the instrument's size precision is three. This is the precision rule lecture
   03 warned about.

Note what the loader does **not** do: it does not sort. The sample rows are already in
publication order. With real data you must sort by `ts_init` before replaying, as
`examples/backtest/crypto_orderbook_imbalance.py` does.

## Step 2: apply the deltas and watch the book

Append this to `build_book.py` and run it.

```python
path = "docs/usermanauls/microstructure-signals/sample_data/order_book_deltas_binance.csv"
instrument = TestInstrumentProvider.btcusdt_perp_binance()
instrument_id = instrument.id

deltas = load_deltas(path, instrument)
book = OrderBook(instrument_id, BookType.L2_MBP)

header = f"{'#':>2} {'action':<6} {'side':<5} {'price':>9} {'size':>7} | {'bid':>7} {'ask':>7} {'ratio':>6}"
print(header)
for index, delta in enumerate(deltas, start=1):
    book.apply_delta(delta)
    bid_size = book.best_bid_size()
    ask_size = book.best_ask_size()
    if bid_size is None or ask_size is None:
        ratio = bid_text = ask_text = "-"
    else:
        bid_dec = bid_size.as_decimal()
        ask_dec = ask_size.as_decimal()
        bid_text = str(bid_dec)
        ask_text = str(ask_dec)
        ratio = f"{min(bid_dec, ask_dec) / max(bid_dec, ask_dec):.3f}"
    print(
        f"{index:>2} {delta.action.name:<6} {delta.order.side.name:<5} "
        f"{str(delta.order.price):>9} {str(delta.order.size):>7} | "
        f"{bid_text:>7} {ask_text:>7} {ratio:>6}",
    )

print()
print("bids:", book.bids_to_dict())
print("asks:", book.asks_to_dict())
```

Run it from the repository root:

```bash
uv run --project python --no-sync python <temp-dir>/scratch/build_book.py
```

Real output:

```text
 # action side      price    size |     bid     ask  ratio
 1 CLEAR  BUY         0.0   0.000 |       -       -      -
 2 ADD    BUY     50000.0   1.000 |       -       -      -
 3 ADD    SELL    50001.0   2.000 |   1.000   2.000  0.500
 4 UPDATE BUY     49999.0   0.500 |   1.000   2.000  0.500
 5 UPDATE SELL    50002.0   1.500 |   1.000   2.000  0.500
 6 DELETE BUY     49998.0   0.000 |   1.000   2.000  0.500
 7 CLEAR  BUY         0.0   0.000 |       -       -      -
 8 ADD    BUY     50100.0   3.000 |       -       -      -
 9 ADD    SELL    50101.0   4.000 |   3.000   4.000  0.750
10 UPDATE BUY     50099.0   1.000 |   3.000   4.000  0.750
11 UPDATE SELL    50102.0   2.000 |   3.000   4.000  0.750

bids: {Decimal('50100.0'): Decimal('3.000'), Decimal('50099.0'): Decimal('1.000')}
asks: {Decimal('50101.0'): Decimal('4.000'), Decimal('50102.0'): Decimal('2.000')}
```

Read the table slowly, because it is the whole lecture in miniature.

- Row 1 is the `CLEAR` the loader injected. Both sides are empty, so the ratio column shows `-`.
- Row 2 adds the bid at 50000.0. There is still no ask.
- Row 3 adds the ask at 50001.0. Now both sides exist: bid 1.000, ask 2.000, ratio
  `min(1, 2) / max(1, 2) = 1 / 2 = 0.500`.
- Rows 4 to 6 are the incremental changes. They move size at 49999.0 and 50002.0 and remove
  49998.0, but the *best* bid and *best* ask do not change, so the top-of-book columns stay at
  1.000 and 2.000. These rows prove that a delta can change the book without changing the top of
  book.
- Row 7 is the second snapshot's `CLEAR`. The old book is discarded.
- Rows 8 and 9 build the new book: bid 3.000 at 50100.0, ask 4.000 at 50101.0, ratio 0.750.
- Rows 10 and 11 add depth behind the top of book; the top of book is again unchanged.

The final book has two levels per side and update count 11, one per applied delta including the two
clears.

## Step 3: compute the imbalance ratio by hand

Take the final state of the book:

```
best bid size = 3.000   at 50100.0
best ask size = 4.000   at 50101.0
```

The ratio the strategy uses is `smaller / larger`:

```
smaller = min(3.000, 4.000) = 3.000
larger  = max(3.000, 4.000) = 4.000
ratio   = 3.000 / 4.000     = 0.750
```

The difference form from lecture 01 gives:

```
imbalance = (3.000 - 4.000) / (3.000 + 4.000) = -1.000 / 7.000 = -0.143
```

Both say the same thing: the ask side is slightly heavier. A ratio of 0.750 is far from the
strategy's default trigger of 0.20, so this snapshot would not trade.

The first snapshot tells the same story at a different price: bid 1.000 against ask 2.000 gives
ratio 0.500, imbalance `-1 / 3 = -0.333`. Still not enough to trigger.

## Step 4: turn the ratio into a decision

The strategy's rule, from `docs/tutorials/orderbook_imbalance.py`, is:

- Read `bid_size` and `ask_size` from the top of the book.
- Compute `smaller = min(bid, ask)`, `larger = max(bid, ask)`, `ratio = smaller / larger`.
- If `larger` is not greater than `trigger_min_size`, stop. A tiny book is ignored.
- If `ratio` is greater than or equal to `trigger_imbalance_ratio`, stop. The book is balanced.
- Otherwise the book leans. Buy at the best ask if the bid side is heavier, otherwise sell at the
  best bid. Size the order as `min(level_size, max_trade_size)`.

This is the code. Save it as `signal.py`.

```python
from decimal import Decimal

from nautilus_trader.model import (
    BookAction,
    BookOrder,
    BookType,
    InstrumentId,
    OrderBook,
    OrderBookDelta,
    OrderSide,
    Price,
    Quantity,
    RecordFlag,
)

instrument_id = InstrumentId.from_str("BTCUSDT-PERP.BINANCE")


def add_delta(action, side, price, size):
    return OrderBookDelta(
        instrument_id,
        action,
        BookOrder(
            side,
            Price.from_decimal_dp(Decimal(price), 1),
            Quantity.from_decimal_dp(Decimal(size), 3),
            0,
        ),
        int(RecordFlag.F_LAST.value),
        1,
        1,
        1,
    )


def evaluate(book, trigger_min_size, trigger_ratio):
    bid = book.best_bid_size().as_decimal()
    ask = book.best_ask_size().as_decimal()
    smaller = min(bid, ask)
    larger = max(bid, ask)
    ratio = smaller / larger
    triggered = larger > trigger_min_size and ratio < trigger_ratio
    side = "BUY" if bid > ask else "SELL"
    print(
        f"bid={bid:.3f} ask={ask:.3f} larger={larger:.3f} "
        f"ratio={ratio:.3f} -> triggered={triggered} side={side if triggered else '-'}",
    )


book = OrderBook(instrument_id, BookType.L2_MBP)
book.apply_delta(add_delta(BookAction.ADD, OrderSide.BUY, "50000.0", "5"))
book.apply_delta(add_delta(BookAction.ADD, OrderSide.SELL, "50010.0", "5"))
print("balanced top of book:")
evaluate(book, Decimal("1"), Decimal("0.20"))

book.apply_delta(add_delta(BookAction.UPDATE, OrderSide.SELL, "50010.0", "0.2"))
print("ask thinned to 0.2:")
evaluate(book, Decimal("1"), Decimal("0.20"))

book.apply_delta(add_delta(BookAction.UPDATE, OrderSide.SELL, "50010.0", "0.4"))
print("ask restored to 0.4, larger side is now 5:")
evaluate(book, Decimal("1"), Decimal("0.20"))

book.apply_delta(add_delta(BookAction.ADD, OrderSide.BUY, "49990.0", "50"))
print("a far bid of 50 is added; top of book is unchanged:")
evaluate(book, Decimal("1"), Decimal("0.20"))
```

Run it and you get:

```text
balanced top of book:
bid=5.000 ask=5.000 larger=5.000 ratio=1.000 -> triggered=False side=-
ask thinned to 0.2:
bid=5.000 ask=0.200 larger=5.000 ratio=0.040 -> triggered=True side=BUY
ask restored to 0.4, larger side is now 5:
bid=5.000 ask=0.400 larger=5.000 ratio=0.080 -> triggered=True side=BUY
a far bid of 50 is added; top of book is unchanged:
bid=5.000 ask=0.400 larger=5.000 ratio=0.080 -> triggered=True side=BUY
```

The fourth case is a warning. Fifty units of bid size were added, but at 49990.0, far from the top.
The top of book did not change and the signal did not change. **Imbalance only sees the best few
prices.** A wall of size ten levels down is invisible to it. If you want depth-weighted imbalance,
you must sum several levels yourself, using `book.bids(depth)`.

## Step 5: run the complete backtest

Now the strategy runs in the engine. The code below is a complete program. It reuses `load_deltas`
from step 1, defines the strategy, and replays the committed sample. Save it as
`run_imbalance.py`.

```python
import csv
from decimal import Decimal

from nautilus_trader.backtest import BacktestEngine
from nautilus_trader.common import LogLevel, LoggerConfig
from nautilus_trader.config import BacktestEngineConfig, StrategyConfig
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.model import (
    AccountType,
    BookAction,
    BookOrder,
    BookType,
    Currency,
    InstrumentId,
    Money,
    OmsType,
    OrderBookDelta,
    OrderSide,
    Price,
    Quantity,
    RecordFlag,
    TimeInForce,
    TraderId,
    Venue,
)
from nautilus_trader.testkit.providers import TestInstrumentProvider
from nautilus_trader.trading import Strategy


def load_deltas(path, instrument):
    with open(path, newline="") as f:
        rows = list(csv.DictReader(f))
    deltas = []
    previous_was_snapshot = False
    sequence = 0
    for row in rows:
        sequence += 1
        is_snapshot = row["is_snapshot"] == "true"
        side = OrderSide.BUY if row["side"] == "bid" else OrderSide.SELL
        amount = Decimal(row["amount"])
        ts_ns = int(row["timestamp"]) * 1000
        if is_snapshot and not previous_was_snapshot:
            deltas.append(
                OrderBookDelta(
                    instrument.id,
                    BookAction.CLEAR,
                    BookOrder(
                        OrderSide.BUY,
                        Price.from_decimal_dp(Decimal("0"), instrument.price_precision),
                        Quantity.from_decimal_dp(Decimal("0"), instrument.size_precision),
                        0,
                    ),
                    int(RecordFlag.F_SNAPSHOT.value),
                    sequence,
                    ts_ns,
                    ts_ns,
                ),
            )
        if is_snapshot:
            action = BookAction.ADD
            flags = int(RecordFlag.F_SNAPSHOT.value)
        elif amount > 0:
            action = BookAction.UPDATE
            flags = int(RecordFlag.F_LAST.value)
        else:
            action = BookAction.DELETE
            flags = int(RecordFlag.F_LAST.value)
        deltas.append(
            OrderBookDelta(
                instrument.id,
                action,
                BookOrder(
                    side,
                    Price.from_decimal_dp(Decimal(row["price"]), instrument.price_precision),
                    Quantity.from_decimal_dp(Decimal(row["amount"]), instrument.size_precision),
                    0,
                ),
                flags,
                sequence,
                ts_ns,
                ts_ns,
            ),
        )
        previous_was_snapshot = is_snapshot
    return deltas


class OrderBookImbalanceConfig(StrategyConfig):
    def __init__(
        self,
        *,
        instrument_id,
        max_trade_size,
        trigger_min_size=100.0,
        trigger_imbalance_ratio=0.20,
        min_seconds_between_triggers=1.0,
        book_type="L2_MBP",
        **_kwargs,
    ):
        super().__init__()
        self.instrument_id = instrument_id
        self.max_trade_size = max_trade_size
        self.trigger_min_size = trigger_min_size
        self.trigger_imbalance_ratio = trigger_imbalance_ratio
        self.min_seconds_between_triggers = min_seconds_between_triggers
        self.book_type = book_type


class OrderBookImbalance(Strategy):
    def __init__(self, config):
        super().__init__(config)
        self._instrument_id = InstrumentId.from_str(config.instrument_id)
        self._book_type = BookType.from_str(config.book_type)
        self._max_trade_size = Decimal(config.max_trade_size)
        self._trigger_min_size = Decimal(str(config.trigger_min_size))
        self._trigger_imbalance_ratio = Decimal(str(config.trigger_imbalance_ratio))
        self._trigger_interval_ns = int(config.min_seconds_between_triggers * 1_000_000_000)
        self._instrument = None
        self._last_trigger_ns = None
        self.trigger_count = 0

    def on_start(self):
        self._instrument = self.cache.instrument(self._instrument_id)
        if self._instrument is None:
            self.log.error(f"Could not find instrument for {self._instrument_id}")
            self.stop()
            return
        self.subscribe_book_deltas(self._instrument_id, self._book_type, managed=True)

    def on_book_deltas(self, _deltas):
        book = self.cache.order_book(self._instrument_id)
        if book is None or not book.spread():
            return
        bid_size = book.best_bid_size()
        ask_size = book.best_ask_size()
        if bid_size is None or bid_size <= 0 or ask_size is None or ask_size <= 0:
            return
        bid = bid_size.as_decimal()
        ask = ask_size.as_decimal()
        smaller = min(bid, ask)
        larger = max(bid, ask)
        if larger <= self._trigger_min_size or smaller / larger >= self._trigger_imbalance_ratio:
            return
        now = self.clock.timestamp_ns()
        if self._last_trigger_ns is not None and now - self._last_trigger_ns < self._trigger_interval_ns:
            return
        if self.cache.orders_inflight(strategy_id=self.strategy_id):
            return
        if bid > ask:
            side, price, level_size = OrderSide.BUY, book.best_ask_price(), ask
        else:
            side, price, level_size = OrderSide.SELL, book.best_bid_price(), bid
        if price is None:
            return
        self._last_trigger_ns = now
        self.trigger_count += 1
        self.submit_order(
            self.order_factory.limit(
                instrument_id=self._instrument_id,
                order_side=side,
                quantity=Quantity.from_decimal_dp(
                    min(level_size, self._max_trade_size),
                    self._instrument.size_precision,
                ),
                price=price,
                time_in_force=TimeInForce.FOK,
                post_only=False,
            ),
        )

    def on_stop(self):
        self.cancel_all_orders(self._instrument_id)


path = "docs/usermanauls/microstructure-signals/sample_data/order_book_deltas_binance.csv"
instrument = TestInstrumentProvider.btcusdt_perp_binance()

engine = BacktestEngine(
    BacktestEngineConfig(
        trader_id=TraderId.from_str("BACKTESTER-001"),
        logging=LoggerConfig(stdout_level=LogLevel.ERROR, print_config=False),
    ),
)
USDT = Currency.from_str("USDT")
BINANCE = Venue("BINANCE")
engine.add_venue(
    venue=BINANCE,
    oms_type=OmsType.NETTING,
    account_type=AccountType.MARGIN,
    base_currency=USDT,
    starting_balances=[Money(100_000, USDT)],
    book_type=BookType.L2_MBP,
    fee_model=MakerTakerFeeModel(maker_rate=Decimal("0.0002"), taker_rate=Decimal("0.0005")),
)
engine.add_instrument(instrument)
engine.add_data(load_deltas(path, instrument))

strategy = OrderBookImbalance(
    OrderBookImbalanceConfig(
        instrument_id=str(instrument.id),
        max_trade_size="1",
        trigger_min_size=1.0,
        trigger_imbalance_ratio=0.20,
        min_seconds_between_triggers=0.0,
    ),
)
engine.add_strategy(strategy)
engine.run()

print("deltas replayed:", len(load_deltas(path, instrument)))
print("triggers:", strategy.trigger_count)
print("fills:")
print(engine.generate_order_fills_report())
engine.reset()
engine.dispose()
```

Run it and you get:

```text
deltas replayed: 11
triggers: 0
fills:
Empty DataFrame
Columns: []
Index: []
```

**Zero triggers, and that is the correct, honest result.** The strategy's own tutorial,
`docs/tutorials/backtest_orderbook_binance.py`, says the same thing about its 100-row sample: "the
sample runs end to end but is too short to trigger the strategy." Eleven deltas span two snapshots
and never push one side below a fifth of the other. The backtest proves the plumbing works; it does
not prove the signal works.

Three details in the venue setup are load-bearing:

- `book_type=BookType.L2_MBP` tells the matching engine to hold a depth book. Without it, depth
  orders do not match and every order ends cancelled. `examples/backtest/crypto_orderbook_imbalance.py`
  sets it for the same reason.
- `fee_model=MakerTakerFeeModel(...)` is required; a backtest venue refuses to be added without an
  explicit fee model, even a zero-fee one.
- `trader_id` and `starting_balances` are accounting boilerplate; the balances are in USDT because
  the instrument settles in USDT.

## Step 6: what needs real book data

Everything up to the decision is in-memory and offline. The one thing the committed sample cannot
give you is a **realistic, long L2 stream**. To see the strategy actually fire, replay real depth
data. `docs/tutorials/backtest_orderbook_binance.py` describes the data: Binance USD-M futures
`T_DEPTH` CSVs for one day, placed under `NAUTILUS_DATA_DIR/Binance/` (default
`~/Downloads/Data/Binance/`). The full update file for BTCUSDT on 2022-11-01 is about 12 GB, roughly
110 million rows, and the tutorial caps the read at one million rows. Either acquire that day from
Binance's public data archive, or set `NAUTILUS_DATA_DIR` to a directory containing your own
`Binance/` folder. Without either, the tutorial falls back to a 100-row sample that does not
trigger.

The larger Ax example for gold needs an external Databento file, so it cannot be faked offline.
`examples/backtest/architect_ax_book_imbalance.py` reads the file path from the environment variable
`GC_DBN` (default `gc_gold_quotes.dbn.zst`) and raises `FileNotFoundError` if it is missing. To
obtain it, follow `docs/tutorials/gold_book_imbalance_ax.md`:

```python
import databento as db
from pathlib import Path

data_path = Path("gc_gold_quotes.dbn.zst")
if not data_path.exists():
    client = db.Historical()
    data = client.timeseries.get_range(
        dataset="GLBX.MDP3",
        symbols=["GC.v.0"],
        stype_in="continuous",
        schema="mbp-1",
        start="2024-11-15",
        end="2024-11-16",
    )
    data.to_file(data_path)
```

That requires a Databento API key and the `databento` client, so it is not part of the offline
manual. The lecture names the requirement rather than pretending the run happens.

## Where the signal comes from, again

The strategy reads only the top of book. If you want a signal that uses more of the book - for
example, total bid size in the top five levels against total ask size - replace `best_bid_size()`
and `best_ask_size()` with sums over `book.bids(5)` and `book.asks(5)`. The rest of the logic is
identical. Lecture 08 has an exercise for exactly this.

## Next

[06](06-measure-and-evaluate.md) measures a run that does fire, using the builtin imbalance actor
and the engine reports.
