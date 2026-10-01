# Go further

## Honest gaps in this manual

- **The sample is short.** Nine and two rows cannot show a realistic run. The manual says so at
  every point where it matters, and shows real triggering behaviour only with a small in-memory
  fixture. To go further you need a real L2 stream.
- **No tick-size or fee model was fitted.** The examples use one arbitrary fee model. Real venues
  charge different rates for makers and takers, and the imbalance strategy's profitability is
  dominated by that difference.
- **No multi-venue book.** The manual reads one venue's book. The same instrument trades elsewhere,
  and this manual did not consolidate. That is a whole subject on its own.
- **No latency model.** The backtest replays events as fast as it can. It does not model the delay
  between seeing a delta and your order reaching the venue, which is the delay that decides whether
  a microstructure signal earns anything.
- **No order-size market impact.** The orders here are tiny relative to the book. Larger orders
  would move the price, and the reports would not show that.
- **The gold/Databento example was not run.** It needs an external file (`GC_DBN`) that is not
  committed. The manual states the requirement and the download command instead of faking a run.

## The concept pages to read next

| Page                                                                 | Why read it                                                                                  |
| -------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| [Order book](../../concepts/order_book.md)                           | The full rules: book types, subscriptions, integrity checks, stale handling, filtered views. |
| [OrderBookDelta](../../concepts/data/order_book_delta.md)            | Every field of a delta, and the `ADD`/`UPDATE`/`DELETE`/`CLEAR` semantics.                   |
| [OrderBookDeltas](../../concepts/data/order_book_deltas.md)          | Batching, and how a batch's metadata mirrors its last delta.                                 |
| [OrderBookDepth](../../concepts/data/order_book_depth.md)            | Depth snapshots, and why they are not interchangeable with deltas.                           |
| [QuoteTick](../../concepts/data/quote_tick.md)                       | Top-of-book data, and how it drives an `L1_MBP` book.                                        |
| [TradeTick](../../concepts/data/trade_tick.md)                       | Executions and aggressor side, used by information-driven bars.                              |
| [Execution algorithms](../../concepts/execution/algorithms.md)       | TWAP, iceberg, quote-pegged and sniper, the algorithms the Rust analytics measure.           |
| [Reports](../../concepts/reports.md)                                 | The account, order and position reports of lecture 06.                                       |
| [Data catalog](../../concepts/data/catalog.md)                       | How real book data is stored and queried for a `BacktestNode`.                               |
| [Backtesting fill models](../../concepts/backtesting/fill-models.md) | How simulated fills differ from real ones.                                                   |

## The repository examples to study

| Example                                            | What it teaches                                                             |
| -------------------------------------------------- | --------------------------------------------------------------------------- |
| `docs/tutorials/orderbook_imbalance.py`            | The imbalance strategy used throughout this manual.                         |
| `docs/tutorials/orderbook_data.py`                 | Converting venue rows into deltas, and snapshot handling.                   |
| `docs/tutorials/backtest_orderbook_binance.py`     | Replaying a real Binance depth day through `BacktestNode`.                  |
| `examples/backtest/crypto_orderbook_imbalance.py`  | The same strategy as a standalone engine backtest.                          |
| `examples/live/architect_ax/strategies.py`         | A quote-driven (top-of-book) imbalance strategy for live trading.           |
| `examples/live/architect_ax/ax_book_imbalance.py`  | The live node wiring for that strategy.                                     |
| `examples/backtest/architect_ax_book_imbalance.py` | The backtest that needs the external Databento file.                        |
| `docs/tutorials/gold_book_imbalance_ax.md`         | The gold proxy backtest, its data download, and its honest negative result. |

## The Rust-only parts

If you need the pre-trade count caps or the execution analytics, you are leaving Python:

- **Execution analytics** - `crates/trading/src/analytics/`. Read-only metrics with declared
  references. Documented in `docs/design/vnpy_lessons_implementation.md` section 4.13.
- **Count caps (D1)** - `crates/risk/src/engine/config.rs`, `count_caps: Vec<RiskCap>`. Not
  reachable from Python.
- **Factor pipeline, membership, panel, dataset** - `crates/research`. Not used by this style, but
  relevant if you take a microstructure feature into a portfolio study.
- **Notification router and sinks** - `crates/common/src/notification`.

Each of these has its own Rust tests. The manual demonstrated the analytics one with `cargo
nextest`; the same pattern works for the others.

## What to learn next, in order

1. **Read `docs/concepts/order_book.md` end to end.** It is the single most useful page for this
   style, and this manual only summarised it.
2. **Replay a real day.** Acquire one day of L2 data for an instrument you understand, load it with
   `docs/tutorials/backtest_orderbook_binance.py`, and reproduce its reported trigger count before
   changing anything.
3. **Build a no-signal baseline.** Run the same strategy with the trigger disabled (or with a random
   trigger of the same frequency) and compare the reports. A signal is only interesting relative to
   a baseline.
4. **Add a position limit and a risk engine limit.** Then verify that the limit actually refuses the
   order.
5. **Compute a depth-weighted imbalance.** Extend exercise 3 to more levels, and measure whether it
   survives a different venue.
6. **Measure the signal's decay.** For each trigger, record the mid price one second before and one,
   five and thirty seconds after. That curve, not the pnl, tells you whether the signal exists.
7. **Then** consider the analytics in `crates/trading/src/analytics/`, once you have orders worth
   measuring.

## Where this style sits

Book imbalance is the smallest, fastest, most venue-specific signal in this collection of manuals.
It is useful as a teaching example because every failure mode is visible: a wrong timestamp, a
missing clear, a crossed book and a spread cost all show up in the reports immediately. Treat it as
a laboratory, not as a product. The market making manual is where the same signal is used as a risk
control rather than as a forecast, and that is the more realistic use.
