# Volume systems: letting the busiest bars speak louder

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| What it trades            | Spot gold on 15-minute bars, weighted by the number of quote updates recorded inside each bar, with signals read from a slower timeframe                                             |
| How often it trades       | A few times a week at most; the quietest rule made 14 trades in three months                                                                                                         |
| What you need             | Python and a data file that includes a per-bar volume or quote-count column                                                                                                          |
| Where the rules come from | [The Strategy Compendium, article 19, volume systems](https://backtrader.readthedocs.io/en/latest/strategies-series/en/19-volume-system.html)                                        |
| The underlying research   | None as a paper: the Ergodic family descends from William Blau's practitioner book Momentum, Direction and Divergence, and the rest are ports of commercial-platform expert advisors |
| How well it held up       | Weak: no independent test, three months of one instrument at zero commission, and final values that sit within about one and a half percent of the starting million                  |
| Also appears in           | Nothing else in this collection                                                                                                                                                      |

## The idea in one paragraph

The old saying is that volume precedes price: a move is more convincing when many trades back it. This
category feeds a market's activity into classic indicators and lets the busiest bars carry more
weight. A volume-weighted moving average is a plain average in which a bar that traded twice as much
counts twice as much, so a heavy day leaves a deep mark and a quiet day barely moves the line. The
strategies then trade the direction the line turns, or the crossover between the activity measure and
its own smoothed copy, with a fixed profit target and stop. The subtlety is that spot gold has no
central exchange and therefore no official volume, so the count of quote updates in each bar stands in
for it.

## Why anyone believed it

A price can move on a single trade, but a price that moves on a flood of trades is a price many people
agreed on. If a rise is accompanied by rising activity, the story says, the buyers are real and the
move is more likely to continue; if a rise happens on almost no activity, it may be a thin market
flickering rather than a genuine re-pricing. Volume is also harder to fake than price, at least in
principle, because moving a price requires only one order while moving volume requires many.

The counterparty is the trader who moves the price and then wants out. Someone who buys a large
quantity pushes the price up; the volume that accompanies that push is a record of their footprint,
and the volume-weighted line follows it. The strategies here are trying to ride the wake of large
orders rather than to anticipate them.

## An everyday comparison

Picture a market at dawn. One trader buys a single crate of oranges and the price on the board ticks
up, but nobody believes it, because one crate is noise. By mid-morning a hundred traders have bought
and the price is higher, and now the sign is credible: the crowd has voted with its money. The
volume-weighted average is like reading the board and weighting each price change by how many crates
changed hands at it, so the morning's single-crate tick is nearly ignored.

## The rules, step by step

The category holds seven strategies. All run on the same architecture: 15-minute gold bars execute
the orders, and bars resampled to four, six or eight hours compute the signals. The window runs from
3 December 2025 to 10 March 2026, about 6,129 15-minute bars, on a million-dollar account with no
commission.

1. Build two price series from the same data: the 15-minute bars used for buying and selling, and a
   slower series formed by grouping the 15-minute bars into four-hour blocks.
2. For each bar in the slow series, record the price and the number of quote updates that occurred
   inside it, which stands in for volume.
3. The plain volume-weighted rule, `Exp_Volume_Weighted_MA`. Compute a volume-weighted average of the
   last 12 slow bars. Read three values in a row. If the oldest of the three is the highest and the
   newest is higher than the middle one, the line has turned up: buy. If the oldest is the lowest and
   the newest is lower than the middle one, the line has turned down: sell short.
4. Give every position a stop 1,000 points away and a target 2,000 points away, where one point is
   0.01 of a gold price. That is a stop of 10.00 and a target of 20.00. Check the levels on every
   15-minute bar, not only when a new signal appears.
5. The remaining rules recombine the same parts. One paints candles from the volume-weighted average
   and trades the colour flip. One rounds the average into a high-low channel. One divides the
   average's change by its own recent wobble to make a standardised score with two thresholds.
6. The Ergodic Tick Volume rules split each bar's activity between buyers and sellers using the bar's
   open-to-close move, smooth both sides several times, and trade the crossover between the resulting
   line and its signal.
7. The Price-Volume Trend rule keeps a running total that adds each bar's volume multiplied by the
   percentage price change, and trades the crossover between that running total and its own five-bar
   average.

## The maths, with every symbol named

The volume-weighted moving average, the centre of six of the seven rules:

```text
VWMA_t = (price_1 * volume_1 + ... + price_L * volume_L) / (volume_1 + ... + volume_L)
```

- `price_i` is the applied price of bar `i` in the window, here the closing price.
- `volume_i` is that bar's activity, here its tick volume: the number of quote updates inside it.
- `L` is the window length, 12 bars on the slower timeframe.
- `VWMA_t` is the average in which a busy bar pulls harder than a quiet one. A plain average is the
  special case where every volume is equal.

The Tick Volume Index, the activity measure split into buying and selling:

```text
up_ticks   = (volume + (close - open) / point) / 2
down_ticks = volume - up_ticks
```

- `volume` is the bar's tick volume.
- `close - open` is the bar's net price move; dividing by `point`, the price value of one quoted tick,
  turns it into ticks.
- `up_ticks` is the share of the activity credited to buyers; `down_ticks` is what is left.
- A bar that closed higher than it opened sends more activity to the buyers, and one that closed lower
  sends it to the sellers.

Both sides are then smoothed twice, and the index itself is a rescaled difference:

```text
TVI = 100 * (smoothed_up - smoothed_down) / (smoothed_up + smoothed_down)
```

- `smoothed_up`, `smoothed_down` are the double-smoothed versions of the two tick counts.
- `TVI` runs from -100 to +100. A positive reading means the smoothed buying activity exceeds selling;
  a negative reading means the reverse.

The Price-Volume Trend total, the running ledger:

```text
PVT_t = PVT_(t-1) + volume_t * (price_t - price_(t-1)) / price_(t-1)
```

- `PVT_t` is the running total; the library starts it at the first bar's volume.
- `volume_t` is that bar's activity.
- `(price_t - price_(t-1)) / price_(t-1)` is the bar's percentage price change.
- A 1 percent rise on heavy volume moves the total more than a 5 percent rise on thin volume, which is
  the whole point of the construction.

## A worked example

A volume-weighted average over three bars, on made-up but plausible figures. The window is shortened
to three so the arithmetic fits on the page; the library uses twelve.

| Bar | Price  | Tick volume | VWMA (3) | Note                        |
| --- | ------ | ----------- | -------- | --------------------------- |
| 1   | 2000.0 | 100         |          |                             |
| 2   | 2005.0 | 150         |          |                             |
| 3   | 2010.0 | 200         | 2006.11  | rising                      |
| 4   | 2000.0 | 120         | 2005.85  | rising but slowing          |
| 5   | 1995.0 | 100         | 2003.57  | falling                     |
| 6   | 1998.0 | 300         | 1997.88  | heavy volume, still falling |
| 7   | 2008.0 | 400         | 2002.63  | heavy volume, turning up    |
| 8   | 2015.0 | 250         | 2006.68  | rising                      |

Bar 3's value is `(2000*100 + 2005*150 + 2010*200) / (100+150+200) = 902,750 / 450 = 2006.11`. Bar 6's
is `(2000*120 + 1995*100 + 1998*300) / 520 = 1,038,900 / 520 = 1997.88`.

The rule reads three consecutive values and asks whether the line has formed a V. On bar 7 the three
values are 2002.63 (today), 1997.88 (yesterday) and 2003.57 (the day before). Yesterday is lower than
the day before, so the line was falling, and today is higher than yesterday, so it has turned up. The
rule buys at the current price of 2008.00. The stop is 10.00 below, at 1998.00, and the target is
20.00 above, at 2028.00.

| Bar | Price  | Stop   | Target | Action         |
| --- | ------ | ------ | ------ | -------------- |
| 7   | 2008.0 | 1998.0 | 2028.0 | buy at 2008.0  |
| 8   | 2015.0 | 1998.0 | 2028.0 | hold           |
| 9   | 2030.0 | 1998.0 | 2028.0 | sell at 2028.0 |

```text
Gross return = 2028.00 / 2008.00 - 1 = 0.00996, that is 0.996 percent
Cost         = 0 in this file: the library charges no commission and no spread
Net return   = 0.996 percent
```

The example is invented and it wins. Its point is the mechanics: a slow line turns, a fixed target is
placed well beyond the stop, and the position is closed on the target rather than on the line's next
turn. With a target twice the stop, a rule that wins less than half its trades can still come out
ahead, which is exactly what the library reports.

## What the research actually found

The library reports seven backtests, all on 15-minute gold over the same three months, all starting
from a million dollars and charging no commission. The numbers below are its own.

| Strategy                 | Trades | Wins   | Profit factor | Final value  |
| ------------------------ | ------ | ------ | ------------- | ------------ |
| VWMA slope (H4)          | 54     | 42.59% | 1.154         | 1,000,646.80 |
| Ergodic Tick Volume (H6) | 14     | 57.14% | 2.04          | 1,005,203.90 |
| Price-Volume Trend (H4)  | 49     | 48.98% | 3.26          | 1,015,722.30 |

Profit factor is the total won divided by the total lost; above 1 means the winners outweigh the
losers. The pattern here is common to short-horizon rules: the win rates hover around or below one
half, and the outcome depends on the winners being larger than the losers. The Price-Volume Trend rule
finished about 1.57 percent up over three months, which is the best of the group and still a small
number on a million dollars before any real cost.

Nothing here is a paper. The Ergodic Tick Volume indicator descends from William Blau's 1995
practitioner book, and the rest are ports of commercial expert advisors. No independent test exists,
and the sample is one instrument over three months at zero cost. The grade is weak.

One thing must be said plainly, because it is the heart of this whole group of tutorials. Every
backtest in the compendium asserts its final value, its reward-to-risk ratio (the return earned per
unit of the strategy's own wobble, also called the Sharpe ratio) and its worst fall against numbers
captured when the strategy was migrated. Passing those assertions proves the engine computes exactly
what the file says, including the resampling of 15-minute bars into four-hour ones and the alignment
of the two feeds to the bar. It proves nothing about whether the strategy earns anything.

## How this project relates to it

This repository does not implement volume-weighted indicators as strategies, but two of its research
briefs explain why volume appears in cost and price models at all. In
[Market impact and trading cost](../../../strategies/books/02_market_impact_and_trading_cost.md), the
measured impact of a large order is a concave power law in the order's size divided by daily volume,
with an exponent near one half across three independent markets (`2411.13965v3`). Volume is the
denominator of that formula, which is the rigorous version of the folklore that heavy activity and
light activity are different regimes.

The second is [Market making and inventory](../../../strategies/books/04_market_making_and_inventory.md),
which warns that backtests usually assume a trade fills at the price it was aimed at, and that this
assumption, not the quoting rule, often decides the measured edge (`1105.3115v5`). These seven
strategies fill at exact prices on every 15-minute bar, so their final values inherit that optimism.

## Where it goes wrong

- Tick volume is not volume. It counts quote updates, and its link to real traded size is loose.
  Feeding it to a price-average changes the indicator's meaning, not just its inputs.
- Two timeframes mean two chances to be one bar wrong. The signal is read from a completed four-hour
  bar while orders execute on 15-minute bars, so the alignment of the two feeds decides the result;
  the library's own notes treat this as the family's commonest migration bug.
- Zero costs and a tiny sample. Three months of one instrument at no commission cannot support a
  conclusion, and the differences between these seven final values are within the noise of the test.
- The stop and target are the strategy. Most of the outcome comes from the 1,000-point stop and the
  2,000-point target, not from the volume weighting. Change those two numbers and the results move.
- Heavy bars are often reaction, not leadership. A volume spike frequently marks the end of a move,
  when the last buyers arrive, rather than its beginning, which is the opposite of the folklore.
- The fill assumption is generous. A stop at a price is assumed to execute at that price, even though
  a fast market jumps over it. That is the optimistic assumption the inventory brief warns about.

## Try it yourself

You need a spreadsheet and a table of any instrument's daily bars with a volume column.

1. Put the date, closing price and volume in three columns.
2. Add a two-bar volume-weighted average: at each row, compute `price today times volume today, plus
   price yesterday times volume yesterday, divided by the sum of the two volumes`.
3. Add a plain two-bar average for comparison, which is just the average of the two prices.
4. Put both columns side by side and find the rows where they differ most. Those are the days when the
   busier of the two bars was much busier.
5. Now look at what the price did on the following day for the ten largest differences.

What to notice: the volume-weighted average lags the plain average on quiet days and jumps on busy
ones. Ask whether the direction of that jump told you anything about the next day, or whether it just
restated the day that had already happened. The volume-weights mostly track the price move they were
supposed to predict.

## Where this came from

- [The Strategy Compendium, article 19, volume systems](https://backtrader.readthedocs.io/en/latest/strategies-series/en/19-volume-system.html),
  the seven-strategy inventory, the VWMA slope rule, the TVI pipeline and the backtest figures quoted
  above.
- [Market impact and trading cost](../../../strategies/books/02_market_impact_and_trading_cost.md),
  this repository's brief on the square-root impact law and the role of volume in it.
- [Market making and inventory](../../../strategies/books/04_market_making_and_inventory.md), the brief
  behind the warning about assumed fills.

## Words used in this tutorial

- exponential smoothing: an average that gives more weight to recent values and decays the old ones.
- fill: the moment an order is actually executed at a price.
- profit factor: the total money won divided by the total money lost, across all closed trades.
- resampling: grouping many short bars into a smaller number of longer ones.
- stop: an order that closes a losing position at a chosen price.
- target: an order that closes a winning position at a chosen price.
- tick volume: the number of quote updates recorded inside a bar, used as a stand-in for traded size.
- volume-weighted moving average: an average in which each price is weighted by that bar's activity.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
