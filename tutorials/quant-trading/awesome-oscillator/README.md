# The awesome oscillator: the gap between two averages of the midpoint

Date: 2026-10-08. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                   |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | One company's shares, or one fund that tracks an index, bought and sold whole, with the signal read from each day's high and low rather than from the close                                                                             |
| How often it trades       | A handful of times a year, once for each time the fast average swaps places with the slow one or a saucer appears                                                                                                                       |
| What you need             | A spreadsheet                                                                                                                                                                                                                           |
| Where the rules come from | The awesome oscillator section of the [quant-trading README](https://github.com/je-suis-tm/quant-trading/blob/master/README.md) and the [TradingView wiki entry](https://www.tradingview.com/wiki/Awesome_Oscillator_(AO)) it points to |
| The underlying research   | None, this is a practitioner's rule of thumb: the indicator is credited to Bill Williams in the 1990s, and no study of this exact rule was found in this collection                                                                     |
| How well it held up       | Open question: the source states the rule and reports no measured result for it, only the author's comment that the choice between it and its control group depends on how much loss a reader can tolerate                              |
| Also appears in           | [The moving average convergence divergence](../../fastquant/macd/README.md) and [the moving average crossover](../../fastquant/smac/README.md) in this collection, whose slower crossover rule is the cousin                            |

## The idea in one paragraph

Look at the midpoint between each day's highest and lowest price. Average the last five midpoints to
get a fast line, and the last thirty-four to get a slow one. The slow line is the memory of the last
few weeks, and the fast line is what the last week is doing. Subtract the slow line from the fast
line, and the result is a momentum reading that sits above zero while recent prices are above their
own recent average and below zero while they are under it. Buy when the reading crosses up through
zero, sell when it crosses down, and add an earlier trigger, called a saucer, that fires while the
reading is still below zero but has started to curl upward. The whole thing is then set against a
slower, exponential version of the same idea, to ask whether the faster rule earns enough extra to
justify its extra trades.

## Why anyone believed it

The story is the ordinary trend story. News travels slowly, some buyers and sellers act late, and a
price that has been moving in one direction tends to keep moving for a while. The counterparty is
whoever is still reacting to the last headline or is forced to trade for reasons of their own, such
as a fund that must buy when new money arrives or sell when investors ask for their cash back. While
those orders are still arriving in one direction, the fast average of the price has already turned,
the slow average has not, and the gap between them turns first.

Two details make this version of the idea feel sharper than a plain average crossover. The first is
that the midpoint, the average of the day's high and low, ignores the opening and closing prints and
so smooths some of the noise of a single trade at the bell. The second is that the gap between the
two averages is read as a speedometer rather than as a level: it says whether the last week is
stronger or weaker than the last seven weeks, not whether the price is high or low in any absolute
sense. The author of the source also adds the saucer, a three-day shape meant to fire a few days
before the slow average would ever turn, on the theory that an early signal is worth more than a
late one.

## An everyday comparison

A household keeps a shopping list whose total cost changes every week. A single week's total is
noisy: one week has a birthday, the next has a stock-up on tins. So the household tracks two averages
of the weekly total, one over the last month and one over the last six months. When prices are
rising, the monthly average pulls ahead of the six-month average, and the gap between them is a
speedometer for how fast the basket is getting dearer. A shopper who watches the gap turn from
negative to positive is being told that the recent trend has changed direction, several weeks before
the six-month average would show it. That is exactly what the fast average minus the slow average is
doing with prices.

## The rules, step by step

The data are daily prices for one instrument: for each day the highest price traded (the high), the
lowest price traded (the low), the first price traded (the open) and the last price traded (the
close). Everything below is recomputed once a day, after the close.

1. For each day, compute the midpoint: the average of that day's high and that day's low.
2. Keep two averages of the midpoint. The fast average is the average of the last five midpoints. The
   slow average is the average of the last thirty-four midpoints. The slow average therefore needs
   thirty-four days of history before it exists at all.
3. Subtract the slow average from the fast average to get the reading. A positive reading means the
   last five midpoints sit above the last thirty-four; a negative reading means the opposite.
4. Crossover buy: when the fast average rises above the slow average, so the reading turns positive,
   buy. Crossover sell: when the fast average falls back below the slow average, sell. Hold the
   position between those two events; there is no price stop and no profit target.
5. Saucer buy, the earlier trigger, taken while the reading is still below zero. Two days in a row
   must have closed above their open, with the second day's reading higher than the first, and the
   day after them must have closed below its open, with the reading still below zero. Buy on that
   third day. The source's comment calls this branch a "bearish saucer", but its code buys here, so
   the label and the action disagree; see "Where it goes wrong".
6. Saucer sell, the mirror image, taken while the reading is still above zero. Two days in a row must
   have closed below their open, with the second day's reading lower than the first, and the day
   after them must have closed above its open, with the reading still above zero. Sell on that third
   day. The source calls this branch a "bullish saucer" while its code sells here.
7. Crossover fallback, so the rule still has a trigger when no saucer appears: use the plain
   crossover of step 4. The source skips a crossover signal that would open a second position in the
   same direction while one is already held.
8. Size and review. The source buys or sells 100 shares for each signal, starting from a pot of
   5,000. Review once a day, after the close, and act on the next day's price.

## The maths, with every symbol named

The indicator, built from the midpoint and two simple moving averages:

```text
midpoint[d] = (high[d] + low[d]) / 2
fast[d]     = (midpoint[d] + midpoint[d-1] + ... + midpoint[d-4]) / 5
slow[d]     = (midpoint[d] + midpoint[d-1] + ... + midpoint[d-33]) / 34
reading[d]  = fast[d] - slow[d]
```

- `d` is the day, and `d-1` is the day before it.
- `high[d]` and `low[d]` are that day's highest and lowest traded price.
- `midpoint[d]` is their average, the price the indicator is built from.
- `fast[d]` is the average of the last five midpoints; `slow[d]` is the average of the last
  thirty-four.
- `reading[d]` is the gap between them. It is measured in the same units as the price, so a reading
  of plus two on a 20-dollar instrument is not the same event as a reading of plus two on a
  2,000-dollar one.

The reading is a momentum measure because it compares a short window with a long one. If the price
has simply been flat, the two averages are equal and the reading is zero. If the recent days are
higher than the older ones, the fast average is above the slow one and the reading is positive, and
the further the recent days have run, the larger the reading.

The two numbers the source uses to compare its rules:

```text
reward_to_variability = (final_value / starting_value - 1) / std(daily_returns)
deepest_fall          = min over d of ( value[d] / max(value[0..d]) - 1 )
```

- `final_value` is the value of the pot on the last day; `starting_value` is 5,000.
- `daily_returns` are the day-to-day percentage changes of the pot's value.
- `std(...)` is the standard deviation, a measure of how widely those daily changes scatter.
- `reward_to_variability` is total growth divided by the daily scatter. The source sets the
  risk-free rate to zero, so this is not the annualised reward-to-variability figure that a fund
  report would quote.
- `value[d]` is the pot's value on day `d`; `max(value[0..d])` is its highest value up to that day.
- `deepest_fall` is the largest drop from a peak to the following low, as a negative fraction.

## A worked example

Eight made-up days of prices. The real rule uses a five-day and a thirty-four-day average; both
windows are shrunk here, to two days and five days, so the whole calculation fits on the page. The
mechanism is identical: the fast line is the shorter average and the slow line is the longer one.

| Day | Low   | High  | Midpoint | Fast 2-bar | Slow 5-bar | Reading |
| --- | ----- | ----- | -------- | ---------- | ---------- | ------- |
| 1   | 9.80  | 10.20 | 10.00    | -          | -          | -       |
| 2   | 10.20 | 10.60 | 10.40    | 10.20      | -          | -       |
| 3   | 10.60 | 11.00 | 10.80    | 10.60      | -          | -       |
| 4   | 11.00 | 11.40 | 11.20    | 11.00      | -          | -       |
| 5   | 11.40 | 11.80 | 11.60    | 11.40      | 10.80      | +0.60   |
| 6   | 11.00 | 11.40 | 11.20    | 11.40      | 11.04      | +0.36   |
| 7   | 10.60 | 11.00 | 10.80    | 11.00      | 11.12      | -0.12   |
| 8   | 10.80 | 11.20 | 11.00    | 10.90      | 11.16      | -0.26   |

Read the fast column on day 5: the last two midpoints are 11.20 and 11.60, so the fast line is
11.40. The slow column on day 5 is the five midpoints from day 1 to day 5, which add to 54.00 and
average 10.80. The reading is 11.40 minus 10.80, which is plus 0.60. On day 6 the fast line stays at
11.40 while the slow line rises to 11.04, so the reading shrinks to plus 0.36: the price has stopped
accelerating, and the gap is closing. On day 7 the fast line falls to 11.00 while the slow line
rises to 11.12, so the reading turns negative at minus 0.12, and on day 8 it is minus 0.26. That is
the whole indicator: a short average and a long average, and the difference between them.

The first day on which both averages exist is day 5, and the fast line is already above the slow
one, so the crossover rule buys that day at the closing price of 11.70. It holds through day 6, when
the reading is still positive, and sells on day 7, when the reading turns negative, at the closing
price of 10.70. The source charges nothing for trading, but a reader's arithmetic should, so charge
0.1 percent of the traded value each way.

| Day | Action | Price | Shares | Value    | Commission |
| --- | ------ | ----- | ------ | -------- | ---------- |
| 5   | buy    | 11.70 | 100    | 1,170.00 | 1.17       |
| 7   | sell   | 10.70 | 100    | 1,070.00 | 1.07       |

The shares were bought for 1,170.00 and sold for 1,070.00, a gross loss of 100.00. The two
commissions add 1.17 and 1.07, which is 2.24, so the net loss is 102.24. Against the starting pot of
5,000 that is a fall of 2.04 percent, leaving 4,897.76. Notice that the reading turned down while the
slow average was still rising, and the rule still lost money on the round trip, which is the honest
shape of a single example: the signal can be early and still be wrong.

To see a saucer, take a reading that is below zero and curling up. Suppose the last three days read
minus 1.20, then minus 0.80, then minus 0.50, and the first two days closed above their open while
the third closed below its open:

| Day | Bar                   | Reading |
| --- | --------------------- | ------- |
| -2  | closed above its open | -1.20   |
| -1  | closed above its open | -0.80   |
| 0   | closed below its open | -0.50   |

The reading is still below zero on all three days, but it has stopped falling, so the source's code
buys here. The mirror image, with the reading above zero and falling, is where its code sells.

## What the research actually found

The source measures nothing that is written down in advance. The script prints a small table of two
reward-to-variability readings and two deepest falls, computed on whatever instrument and dates the
user types in, and then shows four charts. There is no stored result, no fixed sample and no
out-of-sample period, so the page cannot report an effect size from it.

What the source does report is the author's own summary comment, in the script itself: in his tests
the control group, an exponential version of the moving average convergence divergence, had the
higher reward-to-variability reading, traded less often and brought more profit, but had the deeper
worst fall, and his closing question is which is better, answered with "it depends on your risk
averse level". Read plainly, his conclusion is that the choice between the two rules depends on how
much loss the reader can tolerate: the faster rule gave up some reward for a shallower worst fall,
and the slower rule did the opposite. The README states the same idea as a caution: a faster response
"may sound awesome, but it does not guarantee a less risky outcome or a more profitable outcome",
which is why the slower rule is carried along as a control group.

The comparison is not clean, which is a defect of the design rather than of the arithmetic. Both
rules use the windows 5 and 34, but the awesome oscillator averages the midpoint with plain simple
averages while the control averages the closing price with exponential averages. Two things change
at once, so any difference between the two results cannot be attributed to the oscillator alone.

The one piece of independent evidence is about the control group, not about this rule. A study of
moving average convergence divergence rules on the Dow Jones, Nasdaq and S&P 500 constituents from
2015 to 2021 reports that the plain crossover version wins under half its trades, with win rates
between 0.40 and 0.49 (`2206.12282v1`, p.15). That is a different parameterisation (12, 26 and 9)
and a different rule set, so it does not measure the awesome oscillator, but it is the closest
published result to the control group the source uses.

## How this project relates to it

This repository implements the building block directly. The plain rolling average that both lines
are made of is in [crates/indicators/src/average/sma.rs](../../../crates/indicators/src/average/sma.rs),
where a reader can see a fixed-window average recomputed each day. The control group, the gap
between two exponential averages, is in
[crates/indicators/src/momentum/macd.rs](../../../crates/indicators/src/momentum/macd.rs). The
crossover idea itself, in plain words and without the midpoint, is taught in
[the moving average crossover](../../fastquant/smac/README.md) and in
[the moving average convergence divergence](../../fastquant/macd/README.md). A survey of nine
classic indicators on one stock, several of which ask the same question in different clothes, is in
[classic indicators](../../backtrader/multi-indicator/README.md). The sibling page from the same
external repository, on the London session breakout, is
[the session opening range breakout](../session-opening-range-breakout/README.md).

## Where it goes wrong

- The trades are frictionless in the source. Its README states once, for the whole repository, that
  "all trades are frictionless. No slippage, no surcharge, no illiquidity", and the script charges
  nothing, so every result it prints is before costs. A rule that trades a few times a year can
  survive that, but the printed numbers are not what a real account would see.
- The branch labels are inverted. The comment calls one branch a bearish saucer while the code buys
  there, and calls the other a bullish saucer while the code sells there, so a reader who copies the
  comment rather than the code takes the opposite side of the trade. The source's own definition of
  a green bar, "open price is higher than close price", is also the reverse of the usual
  convention.
- The comparison mixes two changes at once. Midpoint against close, and simple average against
  exponential average, are both different, so the test cannot say which one caused any difference.
- Nothing is fixed. The ticker and the dates are typed in by the user, so the author's "from my
  tests" comment has no stated sample, and no reader can reproduce it from the file.
- The reading is in price units, not percent. A threshold or a reading that looks large on one
  instrument is small on another, and the 5-and-34 setting that suits one market may not suit the
  next.
- The reward-to-variability reading is not the standard one. It divides total growth by the daily
  scatter with the risk-free rate set to zero, and does not annualise, so it cannot be compared with
  the figures a fund report or a paper would quote.

## Try it yourself

You need a spreadsheet and about forty days of daily high and low prices for one instrument, such as
a share or an index fund.

1. One row per day, with columns headed Date, High, Low and Close.
2. Add a column headed Midpoint, equal to the average of High and Low.
3. Add a column headed Fast, the average of the last five Midpoint cells.
4. Add a column headed Slow, the average of the last thirty-four Midpoint cells.
5. Add a column headed Reading, equal to Fast minus Slow.
6. Add a column headed Signal that says "buy" on the day Reading turns positive and "sell" on the day
   it turns negative.
7. Count the buy signals and multiply the number by 0.001 times the price, to see the commission a
   real account would pay.

What to notice: how often the reading flips sign while barely moving, and how many of those flips
would be counted as trades. Then look at where the slow line turns compared with where the reading
turned, and see how many days early the reading was. Finally, ask whether the early warning was
right or merely early, which is the question the source's comparison was trying to answer.

## Where this came from

- The awesome oscillator section of the [quant-trading README](https://github.com/je-suis-tm/quant-trading/blob/master/README.md)
  and the script it links to,
  [Awesome Oscillator backtest.py](https://github.com/je-suis-tm/quant-trading/blob/master/Awesome%20Oscillator%20backtest.py),
  both Apache-2.0.
- The rule source that README points to, the
  [TradingView wiki entry](https://www.tradingview.com/wiki/Awesome_Oscillator_(AO)).
- `2206.12282v1`, the comparative study of moving average convergence divergence rules, quoted for
  the control group's win rates, read through this repository's own brief,
  [predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md).
- The sibling page from the same external repository,
  [the session opening range breakout](../session-opening-range-breakout/README.md).

## Words used in this tutorial

- awesome oscillator: the gap between a fast and a slow simple average of the midpoint price.
- midpoint price: the average of a day's highest and lowest traded price.
- moving average: the average of the last N readings, recomputed each day.
- momentum: the tendency of a price that has been moving one way to keep moving that way for a while.
- oscillator: a number that swings above and below zero to show speed rather than level.
- saucer: a three-day shape in which the reading curls and reverses while still on one side of zero.
- reward-to-variability reading: total growth divided by how much the pot's value swung day to day.
- deepest fall: the largest drop from a peak in the pot's value to the following low.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
