# Intraday and medium-frequency systematic trading

This manual teaches a style of trading in which one written rule decides every buy and sell, and
that rule is tested on recorded prices before any money is risked. You will load price data, turn a
rule into a small program, run it through the backtest engine in this repository, and read the
result honestly. The rule used here is a moving-average crossing on bars, which is the simplest
complete example of the style; the same shape carries over to any other rule you can write down.

Up: [Trading style user manuals](../README.md).

## Who should read this

Read this if you have no finance background and no programming background, and you want to
understand what a systematic rule is, how it is tested, and how it fails. You do not need to know
what a share or a currency pair is. You do need to be able to run a command in a terminal and to
copy a program from this manual into a file.

Warning: NautilusTrader executes live trades involving real capital. Work through this manual in a
backtest first. Nothing here is investment advice, and a good backtest result is not a promise
about the future.

## The lecture list

Every lecture builds on the one before it.

| Lecture                                                             | What it covers                                                | Estimated time |
| ------------------------------------------------------------------- | ------------------------------------------------------------- | -------------- |
| [01-what-is-intraday-systematic](01-what-is-intraday-systematic.md) | The style, the vocabulary, and three numbers worked by hand   | 25 minutes     |
| [02-the-engine-view](02-the-engine-view.md)                         | How this repository represents the style, before any code     | 15 minutes     |
| [03-first-run](03-first-run.md)                                     | Setup and the smallest complete program, with its real output | 30 minutes     |
| [04-sample-data](04-sample-data.md)                                 | The committed sample bars and the in-memory quote fixture     | 20 minutes     |
| [05-build-the-strategy](05-build-the-strategy.md)                   | The rule built in five numbered steps from an empty file      | 60 minutes     |
| [06-measure-and-evaluate](06-measure-and-evaluate.md)               | Reports, statistics, and three ways to misread them           | 35 minutes     |
| [07-risks-and-limits](07-risks-and-limits.md)                       | Stops, sizing, look-ahead, curve fitting, and engine limits   | 35 minutes     |
| [08-exercises](08-exercises.md)                                     | Eight exercises with solutions, including one breakage        | 45 minutes     |
| [09-go-further](09-go-further.md)                                   | Honest gaps and what to read next                             | 10 minutes     |

## Prerequisites

- A working NautilusTrader development environment. See the [developer guide](../../developer_guide/index.md).
- A terminal where you can run a command. Every program is given whole, and each line is explained.
- The sample data in this folder. It is committed, so you do not need a market data subscription.

## The sample data

| File                               | Rows | What it is                                                                    |
| ---------------------------------- | ---- | ----------------------------------------------------------------------------- |
| `sample_data/usdjpy_1min_bars.csv` | 167  | One-minute USD/JPY mid-price bars derived from the repository's quote fixture |

Full column definitions, units, provenance and the generator command are in
[sample_data/README.md](sample_data/README.md).

## Conventions used in this manual

- Prices and quantities are decimal. Time is integer nanoseconds since the Unix epoch, which is the
  engine's clock (`docs/concepts/data/bar.md`).
- Every code block is a complete, runnable program or a complete step of one. Output below a block
  is real output from running it.
- Where a feature exists only in Rust, the lecture says so instead of showing Python that does not
  work.
- The builtin strategy used in lecture 03 is quote-driven, not bar-driven. Lecture 05 builds the
  bar-driven rule from scratch and explains the difference.
