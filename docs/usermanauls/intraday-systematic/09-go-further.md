# 09 - Go further

You have built a complete bar-driven rule strategy, run it, read its reports honestly, and seen it
fail in three instructive ways. This lecture says what is missing from this manual and where to go
next.

## Honest gaps in this manual

- One rule only. Every lecture uses the same moving-average crossing. The mechanics transfer to other
  rules, but the results do not.
- One instrument and one venue, both simulated. Nothing here exercises a real venue's connectivity,
  throttling or partial fills.
- One day of synthetic data. The fixture is a smooth sine wave, so it understates whipsaw and
  overstates trend.
- No live trading. This manual never connects to a real account, so it cannot show reconciliation,
  reconnection or a real fill model.
- No parameter search. Choosing parameters by hand, as the exercises do, is a demonstration of the
  danger, not a method.
- No execution analysis. The engine can measure implementation shortfall, but that path is Rust only
  (`crates/trading/src/analytics`) and is outside this manual.
- The sample is far too short for the statistics to be meaningful. Nothing in this manual is a claim
  about profitability.

## The engine does not protect you from

- A rule with no exit.
- An oversized position. `trade_size` is your number.
- A rule that fits the past.
- A stop that slips.
- Costs you never measured.

## What to read next in this repository

The concept pages behind this manual, in the order they become useful:

| Page                                                               | Why                                                   |
| ------------------------------------------------------------------ | ----------------------------------------------------- |
| [Strategies](../../concepts/strategies.md)                         | The full callback surface a `Strategy` inherits       |
| [Bars](../../concepts/data/bar.md)                                 | Every bar field and the timestamp convention          |
| [Bar-based execution](../../concepts/backtesting/bar-execution.md) | How bars become fills, and the look-ahead warning     |
| [Trading calendars](../../concepts/trading_calendars.md)           | Sessions, holidays, early closes and local time       |
| [Deterministic simulation](../../concepts/dst.md)                  | Which sources of nondeterminism a seeded run controls |
| [Reports](../../concepts/reports.md)                               | Every report column and the statistics surface        |
| [Performance periods](../../concepts/performance_periods.md)       | Daily, weekly and monthly frames                      |
| [Portfolio](../../concepts/portfolio.md)                           | How the statistics are defined                        |
| [Positions](../../concepts/positions.md)                           | Long, short, netting and snapshots                    |
| [Data catalog](../../concepts/data/catalog.md)                     | Recording and querying real data                      |

Two example programs are worth reading whole: `docs/tutorials/ema_cross.py`, which is the custom
strategy this manual's shape comes from, and
`examples/backtest/fx_ema_cross_audusd_bars_from_ticks.py`, which builds bars the same way lecture 04
describes.

## Sibling manuals in this series

- [Market making](../market-making/README.md) is the other side of the same coin: instead of
  crossing the spread, the strategy earns it, and the risk becomes inventory rather than direction.
- [Execution algorithms](../execution-algorithms/README.md) covers slicing a parent order, which is
  how you reduce the spread and impact cost this manual showed you how to pay.
- [Factor portfolio](../factor-portfolio/README.md) covers point-in-time datasets and validated
  splits, which is the discipline lecture 07 only pointed at.
- [Training and tuning](../ai-training/README.md) covers seeded search and honest out-of-sample
  reporting, which is what to read before you tune this rule.
- [Running a strategy for real](../production-operations/README.md) covers the operational side this
  manual deliberately omits.

## What to learn next

1. Measure the same rule on a longer series with real gaps and a varying spread, and compare.
2. Add a session filter so the rule does not trade the first minutes after an open.
3. Replace the fixed size with a volatility-scaled size and observe what changes.
4. Write the rule's parameters down before you change them, and keep a log of every change and its
   reason.
5. Read the [training and tuning](../ai-training/README.md) manual, then run a seeded search over
   periods and see how many parameter sets look profitable in sample.

If you do only one of these, do the last. It is the fastest way to stop believing a backtest.
