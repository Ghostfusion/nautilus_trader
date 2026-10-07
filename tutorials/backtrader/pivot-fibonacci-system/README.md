# Pivot points and Fibonacci: price levels that everyone watches

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                           |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Spot gold on 15-minute bars, using price levels computed from the previous day's high, low and close                                                                                                            |
| How often it trades       | Several times a day for the pivot and Fibonacci rules, once every few days for the most patient one                                                                                                             |
| What you need             | Python and a data file of 15-minute gold bars                                                                                                                                                                   |
| Where the rules come from | [The Strategy Compendium, article 24, pivot and Fibonacci systems](https://backtrader.readthedocs.io/en/latest/strategies-series/en/24-pivot-fibonacci.html)                                                    |
| The underlying research   | None as a paper: the pivot formula is anonymous floor arithmetic and the Fibonacci ratios come from a thirteenth-century number sequence                                                                        |
| How well it held up       | Weak: six rules published as trading-platform expert advisors with no independent test, backtested on three months of one instrument, with the busiest finishing a fraction of a percent below where it started |
| Also appears in           | Nothing else in this collection                                                                                                                                                                                 |

## The idea in one paragraph

Before computers, traders on the floor did the same sum every morning: yesterday's high plus low
plus close, divided by three. That number is the pivot. From it you draw three rungs above and three
below, a price map for the day. A second family of levels comes from a different idea: when a price
has moved from a low to a high, the points at 38.2 percent, 50 percent and 61.8 percent of that
journey are said to be where it will pause or turn. This category of six strategies computes those
levels and then asks a separate question, whether the market is strong enough to trade toward them.
The levels answer where a trade might happen; trend and momentum measures answer whether it may.

## Why anyone believed it

These rules survive because enough people watch the same numbers. If thousands of traders compute the
same pivot and rest buy orders on it, the price really does react there, not because the arithmetic
has power but because the orders do. Economists call this a self-fulfilling prophecy. The Fibonacci
ratios go further still: 38.2 and 61.8 percent have no physical basis in a market, but every charting
package draws lines at the same values, and expectation manufactures support and resistance.

The counterparty, if there is one, is the crowd of level-watchers itself, and the risk is that the
story cuts both ways. The same level that holds a price up while buyers defend it can break sharply
when those buyers are filled and the next wave of sellers arrives. A level is not a wall; it is a
queue, and queues move.

## An everyday comparison

Think of the busiest crossing in a town. Pedestrians wait at the painted lines because that is where
the traffic lights are, and the traffic waits because that is where the pedestrians are. The lines
themselves have no power; the behaviour around them does. Now imagine that one morning the lights
fail. The crossing that was safe for years becomes the most dangerous spot in town, and it is the
same paint. A pivot level behaves the same way: what made it work can disappear in a moment.

## The rules, step by step

All six strategies run on 15-minute gold bars over the same three months, from 3 December 2025 to 10
March 2026.

1. Each day, take yesterday's high, low and close, and compute the pivot and its rungs (the formulas
   are in the next section). In one strategy the levels come from the previous trading session rather
   than the previous calendar day.
2. MostasHaR15 divides the price axis into twelve segments using thirteen levels, finds which segment
   the price sits in, and refuses to buy unless there are more than 14 points of room below the next
   level above. That is a simple rule against buying right under a ceiling.
3. MostasHaR15 then demands four confirmations on the hourly timeframe before it acts: a trend
   strength reading above 20, a rising positive direction line that is above the negative one, two
   short averages of the close and the open spreading apart for two bars, and a momentum histogram
   that is climbing. Only then does it buy.
4. Fibo iSAR takes direction from two Parabolic SAR stops, a fast one and a slow one. When the slow
   stop is below the fast stop and both are below the price, it looks for a long trade; the mirror
   image looks for a short.
5. Fibo iSAR then places a limit order at the 50 percent retracement of the most recent swing, not at
   the market. It only fills if the price pulls back to that level. The order expires after three
   15-minute bars, so a level that is not reached is recomputed rather than left sitting.
6. Fibo iSAR's profit target is the 161.8 percent extension of the same swing, and its stop is 30
   trade units beyond the swing extreme. Once the trade is 10 units in profit, the stop trails in
   steps of 5 units.
7. SimplePivot uses only the midpoint of yesterday's high and low as its line. It is always in the
   market: if the open is below yesterday's high but above the midpoint it sells, otherwise it buys,
   and it flips by closing first and reopening.
8. PivotHeiken 3 smooths a Heikin-Ashi momentum reading and treats a turn below the daily pivot as a
   reason to trade back toward the pivot. FiboCandles turns the day's range into a colour-flip
   candle using one of five Fibonacci ratios. Volatility Pivot replaces the fixed pivot with a moving
   line that breathes with the market's average range.

## The maths, with every symbol named

The pivot and its rungs, computed from yesterday's bar:

```text
P  = (H + L + C) / 3
R1 = 2P - L
S1 = 2P - H
R2 = P + (H - L)
S2 = P - (H - L)
R3 = 2P + (H - 2L)
S3 = 2P - (2H - L)
```

- `H`, `L`, `C` are yesterday's highest, lowest and closing prices.
- `P` is the pivot, the average of the three.
- `R1`, `R2`, `R3` are three resistance rungs above the pivot; `S1`, `S2`, `S3` are three support
  rungs below it.
- The two strategies that need more levels fill the gaps: `M4 = (R1 + R2) / 2` sits halfway between
  the first and second resistance rungs, and so on, producing thirteen levels in all.

The Fibonacci retracement level between a swing low and a swing high:

```text
Level = Low + (High - Low) * ratio
```

- `Low` and `High` are the two ends of the recent swing, the lowest low and the highest high found
  over the search window.
- `ratio` is one of 0.236, 0.382, 0.500, 0.618 or 0.786 for a retracement, or 1.618 for an extension
  beyond the swing.
- `Level` is the price where the strategy places an order. A retracement sits inside the swing; an
  extension sits past its end.

The average true range, used by the one strategy whose pivot moves:

```text
TR_t  = the largest of: (H_t - L_t), |H_t - C_(t-1)|, |L_t - C_(t-1)|
ATR_t = the average of the last K values of TR
```

- `H_t`, `L_t`, `C_t` are today's high, low and close.
- `TR_t` is today's true range, the biggest of three measures of how far the price travelled.
- `K` is the window, here 100 days.
- `ATR_t` is the average of that over `K` periods, a moving measure of how much the price roams.
  The volatility pivot moves the line by three times this amount.

## A worked example

Start with yesterday's level, then a Fibonacci entry. Suppose yesterday's high was 2010.00, the low
1980.00 and the close 2000.00.

```text
P  = (2010 + 1980 + 2000) / 3 = 1996.67
R1 = 2 * 1996.67 - 1980      = 2013.33
S1 = 2 * 1996.67 - 2010      = 1983.33
R2 = 1996.67 + 30             = 2026.67
S2 = 1996.67 - 30             = 1966.67
```

Now the Fibonacci trade. After a decline, the most recent swing runs from a high of 2010.00 down to a
low of 1985.00. The two Parabolic SAR lines confirm that the downtrend has stopped, so the strategy
looks for a long entry at the 50 percent retracement.

| Bar | High   | Low    | Close  | Level used       | Action                |
| --- | ------ | ------ | ------ | ---------------- | --------------------- |
| 1   | 2010.0 | 2000.0 | 2004.0 |                  | watch                 |
| 2   | 2008.0 | 1996.0 | 2000.0 |                  | watch                 |
| 3   | 2005.0 | 1992.0 | 1996.0 |                  | watch                 |
| 4   | 2002.0 | 1988.0 | 1992.0 |                  | watch                 |
| 5   | 2000.0 | 1985.0 | 1990.0 | swing low 1985.0 | place limit at 1997.5 |
| 6   | 2001.0 | 1996.0 | 2000.0 | 1997.5           | limit fills at 1997.5 |
| 7   | 2015.0 | 1999.0 | 2012.0 | 2025.45          | hold, stop trails     |
| 8   | 2028.0 | 2012.0 | 2026.0 | 2025.45          | sell at 2025.45       |

The 50 percent entry level is `1985.00 + (2010.00 - 1985.00) * 0.50 = 1997.50`. The 161.8 percent
extension is `1985.00 + 25.00 * 1.618 = 2025.45`. The stop is 30 trade units below the swing low,
where one trade unit is 0.10, so the stop sits at `1985.00 - 3.00 = 1982.00`.

```text
Gross return = 2025.45 / 1997.50 - 1 = 0.01399, that is 1.399 percent
Cost         = 0 in this file: the library charges no commission and no spread
Net return   = 1.399 percent
```

The example is invented and it wins; that is the luck of a short table. Its point is the mechanics:
the order waits at a level instead of chasing, it expires if the level is not reached, and the target
is a fixed multiple of the same swing. The gap between buying and selling prices is the honest cost
here, and the library sets it to zero, which is the single most generous assumption in these six
files.

## What the research actually found

The library reports six backtests, all on 15-minute gold over the same three months, all starting
from a million dollars and charging no commission. The numbers below are its own.

| Strategy          | Bars  | Trades | Wins | Losses | Final value  |
| ----------------- | ----- | ------ | ---- | ------ | ------------ |
| MostasHaR15 pivot | 6,001 | 387    | 200  | 187    | 999,163.70   |
| Fibo iSAR         | 6,128 | 335    | 194  | 141    | 1,005,690.90 |
| PivotHeiken 3     | 6,038 | 1,584  |      |        | (not given)  |
| FiboCandles       | 6,093 | 95     | 56   | 39     | (not given)  |
| Volatility pivot  | 4,446 | 9      |      |        | (not given)  |

The busiest strategy made 387 round trips and finished about 0.08 percent below where it started.
Fibo iSAR, which waits for a pullback, finished about 0.57 percent above. Those are three months of
work on a million dollars, before any real cost, and they are the strongest caution in this category:
a high trade count on a small edge is a machine for paying costs.

Nothing here is a paper. The pivot formula is anonymous, the Fibonacci ratios come from a number
sequence, and no independent test of these six rules exists. The grade is weak.

One thing must be said plainly, because it is the heart of this whole group of tutorials. Every
backtest in the compendium asserts its final value, its reward-to-risk ratio (the return earned per
unit of the strategy's own wobble, also called the Sharpe ratio) and its worst fall against numbers
captured when the strategy was migrated. Fibo iSAR, for example, asserts 6,128 bars, 335 trades, 194
wins, 141 losses and a final value of 1,005,690.90 to the cent. Passing proves the engine computes
exactly what the file says. It proves nothing about whether the strategy earns anything.

## How this project relates to it

This repository does not implement pivot or Fibonacci systems anywhere. The closest thing is the
brief [Limit order book dynamics](../../../strategies/books/03_limit_order_book_dynamics.md), which
measures what happens to an order that waits at a price. One finding there lands directly on Fibo
iSAR: a live study of ten-year Treasury futures found that the price drifts down by about half a tick
after a passive order fills (`2407.16527v1`), because a limit order fills preferentially when the
market is moving against it. That is the hidden cost the library's zero-commission setting ignores:
the 50 percent entry fills most often precisely when the level is about to break.

The mechanical side of the same idea is in the foundations tutorial
[Orders and how they execute](../../foundations/03_orders-and-how-they-execute.md), which explains why
a limit order is a promise to trade at a price rather than a guarantee of a fill.

## Where it goes wrong

- The levels have no power of their own. They work only while enough other people act on them. Crowd
  behaviour changes, and the day it changes the level becomes a trap rather than a floor.
- Costs decide a busy rule. MostasHaR15 made 387 round trips in three months with no commission. Add
  a fraction of a percent a side and the result is clearly negative.
- The backtest ignores the fill problem. A limit order at the 50 percent level is assumed to fill
  whenever the price touches it, and to be safe when it does not. Real passive fills carry a negative
  drift, as the brief above documents.
- The sample is tiny. Three months of one instrument cannot separate a rule from noise, and none of
  these rules has been tested elsewhere.
- Levels are recomputed with hindsight. The swing high and low are found by searching backwards over
  a small window, and a slightly different window gives a slightly different level, so the rule hides
  a parameter that changes its trades.
- The self-fulfilling story is unfalsifiable as stated. If the level holds, the story is confirmed; if
  it breaks, the story is that the crowd moved on. A claim that explains both outcomes explains
  nothing.

## Try it yourself

You need nothing but yesterday's numbers and a calculator.

1. Write down any day's high, low and close for gold or a large share.
2. Compute the pivot as the average of the three, then the first and second resistance and support
   rungs with the formulas above.
3. On the next day, mark where the price touched each rung. Did it react, pass straight through, or
   ignore the level altogether?
4. Do this for twenty days and tally how often the price touched a rung and how often it turned there.
5. Now do the same with three made-up levels that you choose at random, and compare the tallies.

What to notice: prices touch many levels during a day, so "it reacted at the pivot" is easy to say
after the fact. The comparison with random levels is the whole exercise. If the pivot rungs are not
touched or turned at more often than random lines, then the map is a description of where people
happen to look, not a source of edges.

## Where this came from

- [The Strategy Compendium, article 24, pivot and Fibonacci systems](https://backtrader.readthedocs.io/en/latest/strategies-series/en/24-pivot-fibonacci.html),
  the six-strategy inventory, the pivot formula and the Fibo iSAR rules quoted above.
- [Limit order book dynamics](../../../strategies/books/03_limit_order_book_dynamics.md), this
  repository's brief on queue position, fill probability and the negative drift of a passive fill.
- [Orders and how they execute](../../foundations/03_orders-and-how-they-execute.md), the foundations
  tutorial on limit and market orders.

## Words used in this tutorial

- average true range: a moving measure of how far a price travels in a period, averaging the day's
  high-low span and gaps.
- extension: a Fibonacci level beyond the end of the swing, used as a profit target.
- limit order: an instruction to trade only at a named price or better.
- Parabolic SAR: a trailing stop that tightens as a trend continues.
- pivot point: the average of the previous day's high, low and close, used as a central price level.
- retracement: a Fibonacci level inside a price swing, where a pullback might end.
- self-fulfilling prophecy: something that happens because enough people expect it to.
- support and resistance: price levels where buying or selling has repeatedly appeared.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
