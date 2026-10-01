# Execution algorithms and best execution

An execution algorithm takes one large order and breaks it into smaller orders spread over time, so
that the market has less chance to move against the whole size at once. This manual calls that large
order the **parent order**, and it calls the small orders the algorithm creates the **spawned
orders**. This manual is about the price you pay for slicing: how to run the built-in TWAP algorithm
on a backtest, how to read the spawned orders it creates, and how to measure the cost of the whole
execution honestly.

The style is used by anyone who has to work a size that is large relative to what the market is
showing: a fund selling a position, a desk closing a hedge, or a program trading thousands of shares
in a day. Best execution is the discipline of measuring the difference between the price you decided
on and the price you actually got, and reporting it in money and in basis points.

## Who should read this

Read this manual if you want to understand order slicing and execution cost. It assumes no finance
background and no programming background. Every term is defined the first time it appears, and every
program is given whole. If you already trade, the middle lectures will look familiar; the value for
you is the measurement lecture, which shows what this repository can and cannot tell you.

## Prerequisites

- A working NautilusTrader development environment. See the
  [developer guide](../../developer_guide/).
- A terminal where you can run a command. Python knowledge is not required; the programs are
  short and every line is explained.
- The sample data in this folder. You do not need a market data subscription.

## The lectures

| Lecture                                                                  | What it covers                                                                      | Time   |
| ------------------------------------------------------------------------ | ----------------------------------------------------------------------------------- | ------ |
| [01-what-is-execution-algorithms.md](01-what-is-execution-algorithms.md) | Slippage, market impact, implementation shortfall, VWAP and TWAP in plain words     | 20 min |
| [02-the-engine-view.md](02-the-engine-view.md)                           | The domain objects: intent, policy, parent, spawn, and the four shipped algorithms  | 15 min |
| [03-first-run.md](03-first-run.md)                                       | The smallest complete TWAP run, with its real output                                | 30 min |
| [04-sample-data.md](04-sample-data.md)                                   | The committed quote and fill files, their columns, and how to inspect them          | 20 min |
| [05-build-the-strategy.md](05-build-the-strategy.md)                     | The step-by-step build of the TWAP program, and one iceberg and one sniper variant  | 45 min |
| [06-measure-and-evaluate.md](06-measure-and-evaluate.md)                 | Implementation shortfall, what is Rust only, and what Python can measure today      | 30 min |
| [07-risks-and-limits.md](07-risks-and-limits.md)                         | Denials, horizon expiry, churn, quantity conservation, and what the engine enforces | 25 min |
| [08-exercises.md](08-exercises.md)                                       | Seven exercises with worked solutions, including one deliberate breakage            | 40 min |
| [09-go-further.md](09-go-further.md)                                     | Honest gaps, the concept pages, and what to learn next                              | 15 min |

The lectures build on each other. Work through them in order.

## The sample data

Everything in this manual runs offline against two small files in
[sample_data/](sample_data/). See [sample_data/README.md](sample_data/README.md) for the full
description of each file.

| File                                                                               | What it is                                               | Rows |
| ---------------------------------------------------------------------------------- | -------------------------------------------------------- | ---- |
| [sample_data/usdjpy_quotes_sample.csv](sample_data/usdjpy_quotes_sample.csv)       | USD/JPY best bid and ask, one tick per second, sine wave | 120  |
| [sample_data/twap_child_fills_sample.csv](sample_data/twap_child_fills_sample.csv) | The six fills produced by the lecture 03 TWAP run        | 6    |

## The vocabulary used across the manuals

| Term                     | Plain meaning                                                                               |
| ------------------------ | ------------------------------------------------------------------------------------------- |
| Parent order             | The one order you submit; the algorithm slices it.                                          |
| Spawned order            | A smaller order the algorithm creates from the parent.                                      |
| Slippage                 | The difference between the price you expected and the price you got.                        |
| Market impact            | The price move caused by your own order being seen and traded.                              |
| Implementation shortfall | The total cost of executing, measured against the price at the decision moment.             |
| VWAP                     | Volume-weighted average price: the average price weighted by how much traded at each price. |
| TWAP                     | Time-weighted average price: the average price sampled evenly through a time window.        |
| Basis point (bps)        | One hundredth of one percent. 100 bps is 1 percent.                                         |
| Touch                    | The best bid and best ask: the prices at which the market is currently quoting.             |

## Conventions used in this manual

- Money and prices are decimal. Time is integer nanoseconds since the Unix epoch.
- Every code block was run, and the output under it is the real output of that run.
- A claim about the engine cites the file and line that does it.
- Where a feature exists only in Rust, the lecture says so instead of showing Python that does not
  work. This matters most in [06-measure-and-evaluate.md](06-measure-and-evaluate.md).

## Safety

NautilusTrader executes live trades with real capital. Run every example in a backtest first. Nothing
here is investment advice, and a good backtest number is not a promise about the future.
