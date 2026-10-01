# The engine view of an order book

This lecture names the pieces without asking you to run anything. Read it once, then return to it
when a later lecture uses a term you have forgotten. Every statement here cites the file that
implements it, so you can check it yourself.

## The book object

NautilusTrader maintains an order book per instrument, called `OrderBook`. The implementation is in
Rust; Python exposes the same object as `nautilus_trader.model.OrderBook`, and `OwnOrderBook` for
tracking your own resting orders separately. The concept page is `docs/concepts/order_book.md`.

The Python constructor takes two arguments:

```
OrderBook(instrument_id, book_type)
```

where `instrument_id` identifies the instrument, for example `BTCUSDT-PERP.BINANCE`, and
`book_type` selects the level of detail. The Python surface is declared in
`python/nautilus_trader/model/__init__.pyi`.

## Three book types

The engine can maintain a book at three levels of detail. The choice is set when you create the
book and when you configure the venue (`docs/concepts/order_book.md`).

| Book type | Name                      | What it keeps                              |
| --------- | ------------------------- | ------------------------------------------ |
| `L3_MBO`  | market by order           | Every individual order, keyed by order ID. |
| `L2_MBP`  | market by price           | One aggregated entry per price level.      |
| `L1_MBP`  | market by price, top only | Only the best bid and best ask.            |

A venue's raw feed determines which of these you can build. If the venue publishes individual
order IDs, you can maintain `L3_MBO`. If it publishes only price levels, you maintain `L2_MBP`. If
it publishes only the top of book - for example `QuoteTick` data - you maintain `L1_MBP`. Quote,
trade and bar data can drive an `L1_MBP` book.

For order book imbalance you usually want `L2_MBP`: it carries the per-level sizes that the signal
reads, without the per-order bookkeeping of `L3_MBO`.

## The data types

Four data types carry book information. Each has a concept page under `docs/concepts/data/`.

### OrderBookDelta

`OrderBookDelta` is one change to the book - the finest-grained update. Its fields are:

| Field           | Meaning                                                             |
| --------------- | ------------------------------------------------------------------- |
| `instrument_id` | The instrument whose book is changing.                              |
| `action`        | One of `ADD`, `UPDATE`, `DELETE`, `CLEAR`.                          |
| `order`         | A `BookOrder`: side, price, size, and order ID.                     |
| `flags`         | A bit field of event metadata, including `F_LAST` and `F_SNAPSHOT`. |
| `sequence`      | The venue sequence number, or zero if the venue gives none.         |
| `ts_event`      | When the venue produced the event, in nanoseconds.                  |
| `ts_init`       | When the engine initialised the object, in nanoseconds.             |

`BookAction.ADD` adds an order, `UPDATE` changes an existing one, `DELETE` removes it, and `CLEAR`
resets the whole book. Adds and updates require a positive size. A clear carries a null order.
`F_LAST` marks the end of a logical event group; `F_SNAPSHOT` marks data that belongs to a
snapshot. See `docs/concepts/data/order_book_delta.md`.

### OrderBookDeltas

`OrderBookDeltas` groups a non-empty batch of `OrderBookDelta` records that arrived together. It
reduces per-message overhead. Its `sequence`, `ts_event` and `ts_init` mirror the last contained
delta, and the batch must contain only one instrument's deltas. A snapshot batch usually begins
with a `CLEAR` and ends with `F_SNAPSHOT | F_LAST`. See `docs/concepts/data/order_book_deltas.md`.

### OrderBookDepth

`OrderBookDepth` is a self-contained snapshot of a variable number of price levels, rather than a
stream of changes. It carries parallel lists: `bids`, `asks`, `bid_counts`, `ask_counts`, plus the
same flags, sequence and timestamps. It is not interchangeable with an incremental delta stream.
See `docs/concepts/data/order_book_depth.md`. An older fixed-ten-level name,
`OrderBookDepth10`, is gone; only the legacy C FFI and the SBE encoding still require exactly ten
levels per side.

### QuoteTick and TradeTick

`QuoteTick` carries the top of book - `bid_price`, `ask_price`, `bid_size`, `ask_size` - and
`TradeTick` carries one execution - `price`, `size`, `aggressor_side`, `trade_id`. Both can drive
an `L1_MBP` book. They carry less information than deltas, which is why a top-of-book strategy can
run on quote data while a depth strategy cannot. See `docs/concepts/data/quote_tick.md` and
`docs/concepts/data/trade_tick.md`.

## The BookOrder payload

Every delta and depth level carries a `BookOrder`:

| Field      | Meaning                                              |
| ---------- | ---------------------------------------------------- |
| `side`     | `BUY`, `SELL`, or `None` when the feed does not say. |
| `price`    | The order price, as a `Price`.                       |
| `size`     | The order size, as a `Quantity`.                     |
| `order_id` | The source order ID, or 0 when absent.               |

A null/default order uses `None` for its side, zero price, zero size and zero order ID. For
`L2_MBP` input, the engine derives an order ID from the price, because a price level has no natural
order identity. A zero order ID signals missing identity; for top-of-book input the side is used as
the ID. These rules are implemented in `crates/model/src/data/delta.rs` and are documented in
`docs/concepts/data/order_book_delta.md`.

## The accessors you will use

Once a book is populated, you read it through accessors. In Python they are methods on `OrderBook`
(`python/nautilus_trader/model/__init__.pyi`):

| Accessor                            | Returns                                          |
| ----------------------------------- | ------------------------------------------------ |
| `best_bid_price()`                  | The best bid `Price`, or `None`.                 |
| `best_ask_price()`                  | The best ask `Price`, or `None`.                 |
| `best_bid_size()`                   | The size at the best bid, or `None`.             |
| `best_ask_size()`                   | The size at the best ask, or `None`.             |
| `spread()`                          | Best ask minus best bid, or `None`.              |
| `midpoint()`                        | The average of best bid and best ask, or `None`. |
| `bids(depth)` / `asks(depth)`       | Price levels up to `depth`.                      |
| `bids_to_dict()` / `asks_to_dict()` | Price to size maps.                              |
| `pprint(num_levels, group_size)`    | A printable table of the book.                   |

The same accessors exist in Rust (`docs/concepts/order_book.md`), along with analysis methods that
Python also exposes: `get_avg_px_for_quantity`, `get_quantity_for_price`,
`get_quantity_at_level`, and `simulate_fills`.

## Subscribing to book data

A strategy or actor does not create the book it uses; the data engine maintains one cached book per
instrument, and a subscription selects its update source. In Python a `Strategy` or actor calls one
of the following (`docs/concepts/order_book.md`):

| Call                                                                    | Handler it feeds         |
| ----------------------------------------------------------------------- | ------------------------ |
| `subscribe_book_deltas(instrument_id, book_type)`                       | `on_book_deltas(deltas)` |
| `subscribe_book_depth(instrument_id, book_type, managed=...)`           | `on_book_depth(depth)`   |
| `subscribe_book_at_interval(instrument_id, book_type, interval_ms=...)` | `on_book(order_book)`    |

Three rules matter for this style:

- A **managed** subscription updates the cached book, which you then read with
  `self.cache.order_book(instrument_id)`. The imbalance strategies use managed deltas.
- A managed depth subscription **cannot coexist** with managed deltas or an interval subscription
  for the same instrument. The engine rejects the conflicting request. To get depth callbacks
  beside managed deltas, pass `managed=False`; those callbacks then do not update the cached book.
- Interval delivery subscribes to deltas and publishes the cached book on a timer. `OrderBookDepth`
  events do not update a delta-managed book, including in backtests. During a feed outage the
  interval timer can keep publishing the last cached book, which is a trap: a stale book looks
  alive.

The strategies in this manual use `subscribe_book_deltas(..., managed=True)` and read the cached
book.

## Integrity checks

`OrderBook.check_integrity()` validates the book against its type:

- `L1_MBP`: no more than one level per side.
- `L2_MBP`: no more than one order per price level.
- `L3_MBO`: no additional per-level constraint.
- All types: the best bid must not exceed the best ask. A locked market (bid equals ask) is valid.

This is an explicit check: applying a delta does not call it. Separately, the Rust `apply_delta` and
`apply_deltas` validate the incoming instrument ID and return `InstrumentMismatch` on a mismatch.
Out-of-order deltas and depth snapshots are applied rather than rejected, so a venue that replays
events still reaches the state those events describe; only the book metadata is protected, and
`ts_last` and `sequence` never regress. The full rules are in `docs/concepts/order_book.md`.

## The venue configuration

A backtest venue must be told which book type to use for matching, and which fee model to apply.
In `examples/backtest/crypto_orderbook_imbalance.py` the venue is added with
`book_type=BookType.L2_MBP`. If you omit it, the matching engine may build a top-of-book book and
your depth-based limit orders will not match - a mistake you will meet again in lecture 05.

## Builtin strategies and actors

Two builtin components are registered **by name** and are directly relevant to this style. The
registration is gated behind the `examples` feature and lives in the two functions below.

| Name                   | Registered in backtest                 | Registered in live               | What it does                                                   |
| ---------------------- | -------------------------------------- | -------------------------------- | -------------------------------------------------------------- |
| `BookImbalanceActor`   | `crates/backtest/src/python/engine.rs` | `crates/live/src/python/node.rs` | Tracks cumulative bid/ask quoted volume from deltas.           |
| `HurstVpinDirectional` | `crates/backtest/src/python/engine.rs` | `crates/live/src/python/node.rs` | Trades a Hurst regime filter with a VPIN informed-flow signal. |

`BookImbalanceActor` subscribes to `L2_MBP` deltas, sums the resting size at each updated level per
side, accumulates running totals, and prints a per-instrument summary on stop. Its implementation
is `crates/trading/src/examples/actors/imbalance/actor.rs` and its config is
`crates/trading/src/examples/actors/imbalance/config.rs`.

`HurstVpinDirectional` is the closest builtin to a microstructure-informed directional strategy. It
combines a rescaled-range **Hurst exponent** (a measure of whether a price series trends or
mean-reverts) on dollar bars with a **VPIN** (volume-synchronised probability of informed trading)
signal, and gates entry timing on the live quote stream. Its config is
`crates/trading/src/examples/strategies/hurst_vpin_directional/config.rs`; it is exposed to Python
as `nautilus_trader.trading.HurstVpinDirectionalConfig`. You can register it in a backtest with
`engine.add_builtin_strategy("HurstVpinDirectional", config)`.

Both names are confirmed in `crates/backtest/src/python/engine.rs` (the
`builtin_strategy_register` and `builtin_actor_register` functions) and in
`crates/live/src/python/node.rs`. Lecture 06 uses `BookImbalanceActor` to measure a replayed book.

## Where the book data lives in Python

The Python model package `nautilus_trader.model` exposes all of the types above: `BookType`,
`BookAction`, `RecordFlag`, `BookOrder`, `OrderBookDelta`, `OrderBookDeltas`, `OrderBookDepth`,
`QuoteTick`, `TradeTick`, `OrderBook`, `OwnOrderBook`, `Price` and `Quantity`. Their declarations
are in `python/nautilus_trader/model/__init__.pyi`. The instrument types used in the examples
(`CurrencyPair`, `CryptoPerpetual`) come from the same package; test instruments are provided by
`nautilus_trader.testkit.providers.TestInstrumentProvider`.

## What the engine does not do

- It does not decide that a delta is fake. If the feed says a level exists, the book believes it.
- It does not call the integrity check for you on every delta.
- It does not reconcile one venue's book against another's.
- It does not stop a strategy from trading on a stale book after a feed outage.

Lecture 07 returns to each of these.

## Next

[03](03-first-run.md) contains the first program: build a book in memory, apply two deltas, and read
the top of book.
