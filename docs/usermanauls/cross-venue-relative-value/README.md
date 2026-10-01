# Cross-venue and relative value

The same economic thing is priced differently in two places, or two instruments should move
together, and you trade the difference. This manual teaches that style from zero: no finance
background and no programming background are assumed. You build a two-venue backtest, compute the
basis between two perpetual futures on the same asset, submit both legs, and measure why a
correctly hedged pair can still lose money.

## Who should read it

- A beginner who has never placed an order and wants a complete, runnable example.
- A trader who runs single-venue strategies and wants to see what changes when a position spans two
  venues: two accounts, two fee schedules, two margin calculations and no atomic execution.
- A developer who wants to know exactly what NautilusTrader does and does not enforce for a two-leg
  trade.

## Lectures

| Lecture                                                                              | Content                                                                                                               | Estimated time                         |
| ------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------- | -------------------------------------- |
| [01-what-is-cross-venue-relative-value.md](01-what-is-cross-venue-relative-value.md) | The style in plain language, the vocabulary, and three worked examples done by hand.                                  | 30 minutes reading                     |
| [02-the-engine-view.md](02-the-engine-view.md)                                       | How the engine represents two venues, the instruments, the data types, the accounts and the execution paths. No code. | 25 minutes reading                     |
| [03-first-run.md](03-first-run.md)                                                   | Setup and the smallest complete program: two venues, two instruments, the spread printed.                             | 20 minutes reading, 10 minutes running |
| [04-sample-data.md](04-sample-data.md)                                               | The two committed CSV files, their columns and units, timestamp alignment, and real inspection output.                | 20 minutes reading, 10 minutes running |
| [05-build-the-strategy.md](05-build-the-strategy.md)                                 | The step-by-step build of the two-leg strategy, including what happens when the first leg is denied.                  | 45 minutes reading, 20 minutes running |
| [06-measure-and-evaluate.md](06-measure-and-evaluate.md)                             | Reading a two-leg result, why a hedged pair loses money, and what a funding payment does.                             | 35 minutes reading, 15 minutes running |
| [07-risks-and-limits.md](07-risks-and-limits.md)                                     | Leg risk, partial fills, venue outages, transfer time, sizing, loss limits and what the engine enforces.              | 30 minutes reading                     |
| [08-exercises.md](08-exercises.md)                                                   | Seven exercises with worked solutions, including one deliberate breakage.                                             | 45 minutes working                     |
| [09-go-further.md](09-go-further.md)                                                 | Honest gaps, the concept pages to read next, and what to learn after this manual.                                     | 15 minutes reading                     |

Working through the whole manual, running every program, takes about three hours.

## Prerequisites

- A working NautilusTrader development environment. See the [developer guide](../../developer_guide/).
- Comfort with running commands in a terminal. Python knowledge is not required: every program is
  given whole and explained line by line.
- No network access. All data is committed in `sample_data/`, and every run in this manual is
  offline.

## Sample data

| File                                          | Rows | What it is                                                        |
| --------------------------------------------- | ---- | ----------------------------------------------------------------- |
| `sample_data/btcusdt_perp_binance_quotes.csv` | 240  | Top-of-book quotes for a BTC/USDT perpetual on the BINANCE venue. |
| `sample_data/btcusdt_perp_bybit_quotes.csv`   | 240  | The same underlying, same timestamps, on the BYBIT venue.         |

Full column, unit, generation and provenance details are in
[sample_data/README.md](sample_data/README.md). The files are synthetic, deterministic, ASCII, LF,
and small enough to read by hand.

## What you end up with

A backtest that holds two venues at once, computes a basis series, enters on a wide basis, exits on
a narrow one, and reports the result per leg and per venue:

```text
[1] ENTER spread_bps=12.04
[1] EXIT spread_bps=4.94
binance_pnl=-107.20016 bybit_pnl=22.03454 net_pnl=-85.16562
```

That negative number is the point. The trade captured its basis and still lost money, and the manual
shows exactly where the money went: two taker fees per round trip, on two different fee schedules.

## Safety notice

NautilusTrader executes live trades involving real capital. Work through this manual in a backtest
first. Nothing here is investment advice, and a good backtest result is not a promise about the
future. The sample data is synthetic and the fee rates are illustrative; both are chosen to make the
arithmetic visible, not to represent any venue's real schedule.
