# Market making: a beginner course

Market making is the business of standing on both sides of a market at once: you post a price at
which you will buy, and a price at which you will sell, and you hope both prices get hit. Your
profit comes from the difference between your buy price and your sell price (the spread), not from
the market going up or down. The catch is that you do not control which side fills first, so every
fill leaves you holding inventory that you must manage until the other side trades.

This course teaches market making with the NautilusTrader code base. It assumes **no financial
background and no programming background**. Every term is defined the first time it appears. Every
claim about the engine is checked against a file in this repository and the file path is written
next to the claim.

Part of the [Trading style user manuals](../README.md).

## Who should read this

- A beginner who wants to understand how quoting both sides of a market makes (or loses) money.
- A developer who wants a runnable, deterministic example before touching live capital.
- Anyone who has read a market making description and wants the real mechanics: fees, adverse
  selection, inventory limits, and what the engine enforces versus what it leaves to you.

## What you need

- This repository, checked out at the `develop` branch.
- The Python environment described in `README.md` at the repository root, with `uv` available.
- About four to six hours to read the lectures and run the examples.

No prior experience with the order book, with basis points, or with pandas is assumed.

## The lectures

| Lecture                                                    | What it covers                                                   | Time   |
| ---------------------------------------------------------- | ---------------------------------------------------------------- | ------ |
| [01-what-is-market-making.md](01-what-is-market-making.md) | The business, the jargon, and three worked numeric examples.     | 45 min |
| [02-the-engine-view.md](02-the-engine-view.md)             | How this repository models a market maker: data, orders, config. | 30 min |
| [03-first-run.md](03-first-run.md)                         | Your first complete program, run line by line.                   | 40 min |
| [04-sample-data.md](04-sample-data.md)                     | The committed sample files, their columns, and how to read them. | 30 min |
| [05-build-the-strategy.md](05-build-the-strategy.md)       | Building and tuning the grid market maker step by step.          | 60 min |
| [06-measure-and-evaluate.md](06-measure-and-evaluate.md)   | The three reports and how to read every column.                  | 45 min |
| [07-risks-and-limits.md](07-risks-and-limits.md)           | Position limits, skew, fees, and what breaks in production.      | 45 min |
| [08-exercises.md](08-exercises.md)                         | Six exercises with solutions and one deliberate break.           | 45 min |
| [09-go-further.md](09-go-further.md)                       | Honest gaps and where to go next.                                | 15 min |

Read them in order. [03-first-run.md](03-first-run.md) is the first one with a program you run
yourself.

## The sample data

The `sample_data/` directory holds four small deterministic files. They are committed to the
repository, so nothing in this course needs a network connection:

- [usdjpy_quotes_sample.csv](sample_data/usdjpy_quotes_sample.csv) - 300 USD/JPY quote updates, the
  first 300 rows of the in-memory generator in `python/nautilus_trader/testkit/providers.py`.
- [usdjpy_trades_sample.csv](sample_data/usdjpy_trades_sample.csv) - 300 USD/JPY trade prints that
  occur at those quotes.
- [gbpusd_quotes_sample.csv](sample_data/gbpusd_quotes_sample.csv) - 40 GBP/USD quotes around the
  worked example market in [01-what-is-market-making.md](01-what-is-market-making.md).
- [book_snapshot_deltas.csv](sample_data/book_snapshot_deltas.csv) - three order book delta rows
  borrowed from the committed Tardis test fixture
  `crates/adapters/tardis/test_data/csv/deltas_1.csv`.

[sample_data/README.md](sample_data/README.md) describes every column, unit, row count, generator
command, and provenance.

## One warning before you start

Market making looks easy in a backtest and is hard in production. This course shows losing runs on
purpose. A strategy that quotes both sides will lose money on data that trends, and the first
lecture explains exactly why. Never treat a backtest profit as a promise.

Next: [01-what-is-market-making.md](01-what-is-market-making.md).
