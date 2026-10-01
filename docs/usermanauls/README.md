# Trading style user manuals

These manuals teach a trading style from zero: no finance background and no programming background
are assumed. Each manual is a short course in its own folder, made of numbered lectures that build
one working example from the first line of code to a measured result.

The subject is NautilusTrader. **NautilusTrader executes live trades involving real capital.** Work
through a manual in a backtest first. Nothing in these manuals is investment advice, and a good
backtest result is not a promise about the future.

## The manuals

| Manual                                                             | Style                                      | What you build                                                                                    |
| ------------------------------------------------------------------ | ------------------------------------------ | ------------------------------------------------------------------------------------------------- |
| [market-making](market-making/README.md)                           | Market making and liquidity provision      | A two-sided quoting strategy with inventory limits, measured on maker and taker fees              |
| [execution-algorithms](execution-algorithms/README.md)             | Execution algorithms and best execution    | A parent order sliced by TWAP and by the touch-aware algorithms, with the cost measured           |
| [microstructure-signals](microstructure-signals/README.md)         | Order book and microstructure signals      | A signal read from the shape of the order book, backtested on book data                           |
| [intraday-systematic](intraday-systematic/README.md)               | Intraday and medium-frequency systematic   | A bar-driven rule strategy with indicators, sized and stopped                                     |
| [cross-venue-relative-value](cross-venue-relative-value/README.md) | Cross-venue and relative value             | The same underlying priced on two venues, with the spread traded and the basis tracked            |
| [factor-portfolio](factor-portfolio/README.md)                     | Factor research and portfolio construction | A point-in-time dataset, a factor, a validated split, and a rebalance through the target pipeline |
| [options](options/README.md)                                       | Options                                    | An option chain, the greeks, an exercise style, a price, and a volatility surface                 |
| [production-operations](production-operations/README.md)           | Running a strategy for real                | Risk limits, notifications, state persistence, reconciliation, and the sandbox client             |
| [ai-training](ai-training/README.md)                               | Training and tuning a strategy             | A seeded search over parameters, a validation scheme, a memo cache, and an honest report          |

## The lecture map

Every manual uses the same lecture order, so what you learn in one transfers to the next.

| Lecture                   | Content                                                                  |
| ------------------------- | ------------------------------------------------------------------------ |
| `01-what-is-...`          | The style in plain language, with the vocabulary and hand-worked numbers |
| `02-the-engine-view`      | How this repository represents the style, before any code                |
| `03-first-run`            | Setup and the smallest complete program, with its expected output        |
| `04-sample-data`          | The sample data, its columns, its units, and how to inspect it           |
| `05-build-the-strategy`   | The step-by-step build, one numbered step at a time                      |
| `06-measure-and-evaluate` | Reports and statistics, and how to avoid misreading them                 |
| `07-risks-and-limits`     | What breaks, sizing, loss limits, and what the engine does not enforce   |
| `08-exercises`            | Practice with worked solutions, including one deliberate breakage        |
| `09-go-further`           | Honest gaps and the concept pages to read next                           |

## Prerequisites

- A working NautilusTrader development environment. See the [developer guide](../developer_guide/).
- Comfort with running a command in a terminal. Python knowledge is not required: every program is
  given whole, and each line is explained.
- Sample data for each manual is committed in that manual's `sample_data/` folder, so you do not
  need a market data subscription to follow along.

## Conventions used in these manuals

- Values are shown with the exact type the engine uses. Prices and quantities are decimal, and time
  is integer nanoseconds since the Unix epoch.
- Every code block is a complete, runnable program or a complete step of one. Output shown below a
  block is real output from running it.
- A statement that the engine does something cites the file that does it.
- Where a feature exists only in Rust, the lecture says so instead of showing Python that does not
  work.
