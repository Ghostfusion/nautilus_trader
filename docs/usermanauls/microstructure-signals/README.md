# Order book and microstructure signals

An **order book** is the list of resting buy and sell orders a venue is currently holding, arranged
by price. **Microstructure signals** are numbers you compute from that list - mostly from how much
size sits on each side - to guess what the price will do in the next few seconds. The whole craft
is remembering that the order book is a crowded, noisy, constantly rewritten list, so a signal
that looks strong on one snapshot is usually weak once you try to trade it.

This manual builds one complete order book study. You read a small committed delta file, rebuild
the book from it, compute the imbalance between the two sides, run a strategy that reacts to that
imbalance, and then measure what the run actually did. Along the way you learn what a spread is,
why the signal is weak, and how clock and sequencing problems quietly corrupt book state.

## Who should read this

You have no finance background and no programming background. You can run a command in a terminal.
Every program is given whole and explained line by line, so you do not need to write code from
scratch. If you have never seen a price chart, [01](01-what-is-microstructure-signals.md) starts
from what a bid and an ask are.

## The lectures

| Lecture                                                                   | What it covers                                                             | Time   |
| ------------------------------------------------------------------------- | -------------------------------------------------------------------------- | ------ |
| [01-what-is-microstructure-signals](01-what-is-microstructure-signals.md) | Books, spread, imbalance, the vocabulary, three hand-worked examples       | 40 min |
| [02-the-engine-view](02-the-engine-view.md)                               | How this repository models books and deltas                                | 25 min |
| [03-first-run](03-first-run.md)                                           | Setup and the smallest complete program that builds a book                 | 30 min |
| [04-sample-data](04-sample-data.md)                                       | The committed delta files: their columns, units, and how to inspect them   | 30 min |
| [05-build-the-strategy](05-build-the-strategy.md)                         | Rebuild a book from deltas and compute the imbalance signal, step by step  | 60 min |
| [06-measure-and-evaluate](06-measure-and-evaluate.md)                     | The reports, the builtin imbalance actor, and the Rust-only analytics      | 50 min |
| [07-risks-and-limits](07-risks-and-limits.md)                             | Signal decay, one-venue overfitting, snapshot versus stream, clock hazards | 45 min |
| [08-exercises](08-exercises.md)                                           | Six exercises with solutions, plus one deliberate breakage                 | 50 min |
| [09-go-further](09-go-further.md)                                         | Honest gaps, the concept pages to read next, and what to learn after this  | 15 min |

## Prerequisites

- A working NautilusTrader development environment. See the
  [developer guide](../../developer_guide/).
- Comfort running a command in a terminal.
- The sample files in [`sample_data/`](sample_data/README.md). They are committed copies of this
  repository's own test fixtures, so you need no market data subscription and no network access to
  follow the whole manual.

The **backtest engine, live node, instruments, data types and order books are Python**. Execution
analytics (implementation shortfall and its relatives) is **Rust only**: there is no Python binding
for it, so lecture 06 says so plainly and demonstrates it by running the crate's own test instead
of showing Python that does not work.

## Conventions used here

- Prices and quantities are decimal. Time is integer nanoseconds since the Unix epoch (1 January
  1970), which is how the engine stores every timestamp.
- Every code block is a complete program or a complete step of one, and the output below it is real
  output from running it outside this repository.
- A statement about what the engine does cites the file that does it.
- Where a feature exists only in Rust, the lecture says so instead of showing Python that does not
  work.

## Sample data inventory

| File                                                                                     | Rows | What it is                                      |
| ---------------------------------------------------------------------------------------- | ---- | ----------------------------------------------- |
| [`sample_data/order_book_deltas_binance.csv`](sample_data/order_book_deltas_binance.csv) | 9    | Snapshot plus incremental L2 deltas for BTCUSDT |
| [`sample_data/tardis_deltas_1.csv`](sample_data/tardis_deltas_1.csv)                     | 2    | The minimal two-row delta fixture, verbatim     |

See [`sample_data/README.md`](sample_data/README.md) for every column, its units, the row count,
the exact copy command that produced each file, and the provenance.
