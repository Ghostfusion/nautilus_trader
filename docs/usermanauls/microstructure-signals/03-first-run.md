# Your first run

This lecture gets you from nothing to a program that builds an order book and reads it. The program
is small on purpose: it uses no data file, no venue and no strategy, only the model objects from
lecture 02.

## 1. Set up the environment

The commands in this manual were run from the repository root in Git Bash on Windows, using the
project's own virtual environment and `uv`. From the repository root:

```bash
export PATH="$HOME/.cargo/bin;$HOME/.local/uv012;$HOME/AppData/Local/Programs/Python/Python312/cpython-3.14-windows-x86_64-none;$PATH"
cd python
uv run --no-sync python --version
```

Throughout this manual, scripts are run from the repository root with `uv run --project python
--no-sync python <script>`, which selects the same interpreter without changing directory. The two
forms are equivalent; the second keeps relative paths such as `docs/...` working.

The expected output is a Python version line, for example:

```text
Python 3.14.0
```

If your paths differ, substitute your own `uv` location. The important part is that `uv run
--no-sync` runs the interpreter inside `python/.venv`, which already has NautilusTrader installed.
Do not install anything else to follow this manual.

Write your programs in a folder **outside** the repository, for example
`<temp-dir>/scratch/first_book.py`. The manual never adds `.py` files under `docs/`.

## 2. The smallest complete program

Two deltas are enough for a book: one bid and one ask. Save the following as `first_book.py`.

```python
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

bid = OrderBookDelta(
    instrument_id=instrument_id,
    action=BookAction.ADD,
    order=BookOrder(
        OrderSide.BUY,
        Price.from_str("50000.0"),
        Quantity.from_str("5.000"),
        0,
    ),
    flags=int(RecordFlag.F_LAST.value),
    sequence=1,
    ts_event=1_640_995_200_000_000_000,
    ts_init=1_640_995_200_000_000_000,
)
ask = OrderBookDelta(
    instrument_id=instrument_id,
    action=BookAction.ADD,
    order=BookOrder(
        OrderSide.SELL,
        Price.from_str("50010.0"),
        Quantity.from_str("5.000"),
        0,
    ),
    flags=int(RecordFlag.F_LAST.value),
    sequence=2,
    ts_event=1_640_995_200_000_000_000,
    ts_init=1_640_995_200_000_000_000,
)

book = OrderBook(instrument_id, BookType.L2_MBP)
book.apply_delta(bid)
book.apply_delta(ask)

bid_size = book.best_bid_size().as_decimal()
ask_size = book.best_ask_size().as_decimal()
imbalance = (bid_size - ask_size) / (bid_size + ask_size)

print("best bid price:", book.best_bid_price())
print("best ask price:", book.best_ask_price())
print("best bid size: ", bid_size)
print("best ask size: ", ask_size)
print("spread:        ", book.spread())
print("midpoint:      ", book.midpoint())
print("imbalance:     ", imbalance)
print("bids:", book.bids_to_dict())
print("asks:", book.asks_to_dict())
```

Run it from the repository root:

```bash
uv run --project python --no-sync python <temp-dir>/scratch/first_book.py
```

## 3. The real output

```text
best bid price: 50000.0
best ask price: 50010.0
best bid size:  5.000
best ask size:  5.000
spread:         10.0
midpoint:       50005.0
imbalance:      0
bids: {Decimal('50000.0'): Decimal('5.000')}
asks: {Decimal('50010.0'): Decimal('5.000')}
```

The last two lines are the return values of `book.bids_to_dict()` and `book.asks_to_dict()`: maps
from price to size, exactly the two levels you applied. The book also has a `pprint` method that
draws a table with bids on the left, prices in the centre and asks on the right, but it uses
box-drawing characters, so this manual prints the dicts instead.

## 4. Line by line

**The imports.** All ten names come from `nautilus_trader.model`, whose declarations are in
`python/nautilus_trader/model/__init__.pyi`. `BookType` selects the book's level of detail,
`BookAction` says what a delta does, `RecordFlag` carries event metadata, `BookOrder` is the order
payload, `OrderBookDelta` is one change, and `OrderBook` is the book itself.

**`InstrumentId.from_str("BTCUSDT-PERP.BINANCE")`.** The instrument is named
`SYMBOL.VENUE`. The part before the dot is the symbol; the part after the dot is the venue.

**Constructing a delta.** `OrderBookDelta` is built with keyword arguments in Python. `action` is
`BookAction.ADD`, because the order is new. `order` is a `BookOrder(OrderSide.BUY, price, size,
order_id)`. The `order_id` is 0: no venue order ID is available, which is normal for a price-level
feed. `flags` is `F_LAST`, marking the end of a logical event group. `sequence` is a venue counter;
here 1 and 2. `ts_event` and `ts_init` are integer nanoseconds since 1 January 1970; the value
`1_640_995_200_000_000_000` is 2022-01-01T00:00:00Z. A **nanosecond** is one billionth of a second.

**`book = OrderBook(instrument_id, BookType.L2_MBP)`.** Creates the book. `L2_MBP` means one entry
per price level, which is what a depth feed provides. Creating the book does not populate it.

**`book.apply_delta(bid)` then `book.apply_delta(ask)`.** Applies each change in order. After the
first call only a bid exists, so `best_ask_price()` would be `None`; after the second both sides
exist.

**Reading the book.** `best_bid_size()` returns a `Quantity`; `.as_decimal()` converts it to a
`Decimal`, which is an exact decimal number. `book.spread()` returns `10.0` because
`50010.0 - 50000.0 = 10.0`. `book.midpoint()` returns `50005.0`.

**`imbalance`.** `(5 - 5) / (5 + 5) = 0`. Both sides hold the same size, so the signal is neutral.
Change one of the two sizes and the number moves away from zero.

**`bids_to_dict()` and `asks_to_dict()`.** Return maps from price to size for the levels the book
holds. The bid at 50000.0 appears with size 5.000, and the ask at 50010.0 with size 5.000.

## 5. What happened, in engine terms

You used the same `OrderBook` object the engine uses in a backtest or a live node. You fed it
`OrderBookDelta` records by hand instead of receiving them from a data subscription. That is the
whole idea behind the in-memory route used later in this manual: the book does not care where the
deltas came from.

Two details to notice now, because they will matter:

- The sizes printed as `5.000`, not `5`. The engine keeps a fixed precision per instrument, in this
  case three decimal places, derived from the instrument definition. When you load real data you
  must round prices and sizes to that precision, or `apply_delta` raises an error. Lecture 05 shows
  the exact message.
- The book also tracks `sequence`, `ts_last` and `update_count`. The engine uses `sequence` to
  detect stale updates; it never lets it go backwards. You can print them with
  `book.sequence`, `book.ts_last` and `book.update_count`.

## 6. If it does not work

- `ModuleNotFoundError: nautilus_trader` means you ran a system Python. Run the script with
  `uv run --project python --no-sync python <script>` from the repository root, not with a bare
  `python`.
- `ValueError` mentioning precision means your `Price` or `Quantity` has the wrong number of
  decimal places for the instrument.
- An empty book after applying deltas usually means the `instrument_id` of the delta does not match
  the book's. The Rust `apply_delta` validates this and returns `InstrumentMismatch`; in Python the
  mismatch surfaces as an error from the same code path (`docs/concepts/order_book.md`).

## Next

[04](04-sample-data.md) introduces the committed delta files and shows how to read their columns.
