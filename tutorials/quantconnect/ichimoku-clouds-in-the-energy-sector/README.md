# The Ichimoku cloud, traded on the biggest energy companies

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                      |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| What it trades            | Shares of the ten largest American energy companies                                                                                                                                        |
| How often it trades       | A few times a year per company, whenever the lines cross                                                                                                                                   |
| What you need             | A spreadsheet                                                                                                                                                                              |
| Where the rules come from | [QuantConnect strategy library, Ichimoku clouds in the energy sector](https://www.quantconnect.com/tutorials/strategy-library/ichimoku-clouds-in-the-energy-sector)                        |
| The underlying research   | Gurrib, Kamalov and Elshareif, [Can the Leading US Energy Stock Prices be Predicted using the Ichimoku Cloud?](https://doi.org/10.32479/ijeep.10260)                                       |
| How well it held up       | Disputed: the source study reports strong gains while the library page's own replication, which removes a way of seeing the future, finds the strategy failing to beat a plain energy fund |
| Also appears in           | Nothing else in this collection                                                                                                                                                            |

## The idea in one paragraph

The Ichimoku cloud is a Japanese charting method, published by a journalist writing under the name
Ichimoku Sanjin and later reshaped by Hidenobu Sasaki, that draws five lines from a price chart and
shades a band between two of them. The lines are all built from the highest high and the lowest low
over fixed numbers of recent days, so they summarise the middle of the recent trading range over
different spans. The shaded band, drawn a month into the future, is treated as a barrier: while the
price is above it, the chart is read as rising; while the price is below it, as falling. This strategy
buys the ten largest American energy companies when the comparison line rises above the top of the
band, and sells them when it falls below the bottom. It holds each company until the opposite signal
appears.

## Why anyone believed it

Technical analysis of this kind rests on two ideas. The first is that prices respect levels they have
turned at before: a price that stalls repeatedly near a certain level suggests buyers and sellers
there, and the middle of a recent range is a cheap stand-in for such a level. The second is that
trends persist, so a break out of a range tends to continue. Energy shares fit the second idea
unusually well, because their profits swing with the oil price, and the oil price moves in long
cycles driven by investment and demand rather than in a day.

The counterparty is therefore the investor who reacts to falling oil prices with a delay, and the
fund that has to trim its energy holdings after a run of losses. The cloud is meant to detect the
change of direction a little before the slowest of those sellers, by comparing the price with a
barrier built from a month or two of range.

## An everyday comparison

Think of a coastal path with a rope fence along it. The fence is built from the average of where the
path has run over the last few weeks, so it lags behind, but it is still a useful guide: if a walker
stays on one side of the fence for a while, the path is heading somewhere. The Ichimoku method is a
fence of that kind. It also peeks forward, drawing the fence a month ahead of where the path is now,
but the fence itself is only made of old measurements. The strategy follows the path while it stays
above the fence and gives up when it drops below.

## The rules, step by step

The five lines come first. Each is built from daily prices: the high, the low and the close.

1. The conversion line is the middle of the recent range over nine days: add the highest high of the
   last nine days to the lowest low of the last nine days and divide by two.
2. The base line is the same idea over twenty-six days, roughly a month of trading.
3. Leading span A is the average of the conversion line and the base line.
4. Leading span B is the middle of the range over fifty-two days, roughly a quarter.
5. Leading spans A and B are both shifted twenty-six days into the future, and the band between them
   is the cloud. The top of the cloud is whichever span is higher that day; the bottom is the lower.
6. The lagging line is simply today's closing price drawn twenty-six days into the past.

The strategy then reads the lagging line against the cloud:

7. Buy a company when its lagging line crosses from below to above the top of its cloud.
8. Sell a company short when its lagging line crosses from above to below the bottom of its cloud.
9. Keep holding until the opposite crossing appears. Ignore days when the lagging line is inside the
   cloud, which is treated as no trend.
10. Choose the ten largest American energy companies by market value, refresh that list once a month
    so that the largest companies are always used, and hold every position in equal size.

One caution is worth stating plainly. Because the leading spans are drawn forward and the lagging line
is drawn backward, the comparison on any given day is between a price from twenty-six days ago and a
cloud built from data before that. The library page calls this a way of seeing the future, or
look-ahead bias, and removes it by using only information available at the time. A reader building
this on a spreadsheet should do the same.

## The maths, with every symbol named

Each line is a midpoint of a range.

```text
conversion(t) = (highest high over the last 9 days + lowest low over the last 9 days) / 2
base(t)       = (highest high over the last 26 days + lowest low over the last 26 days) / 2
span A(t)     = (conversion(t) + base(t)) / 2
span B(t)     = (highest high over the last 52 days + lowest low over the last 52 days) / 2
```

- `conversion(t)` and `base(t)` are the two shorter midpoints on day `t`, called Tenkan-sen and
  Kijun-sen in Japanese.
- `span A(t)` and `span B(t)` are the two lines that will be drawn twenty-six days ahead of day `t`.
- `highest high` is the largest high price seen in the window, and `lowest low` the smallest low.

The cloud on any day is the band between the two spans, and the location of the lagging line is a
three-way test:

```text
top(t)    = max(span A(t - 26), span B(t - 26))
bottom(t) = min(span A(t - 26), span B(t - 26))
location(t) = +1 if lagging(t) > top(t), -1 if lagging(t) < bottom(t), 0 otherwise
```

- `top(t)` and `bottom(t)` are the edges of the cloud on day `t`, built from spans computed
  twenty-six days earlier.
- `lagging(t)` is the closing price of twenty-six days ago, drawn on day `t`.
- `location(t)` is `+1` above the cloud, `-1` below, and `0` inside.

A buy signal fires when the location changes from below or inside to `+1`; a sell signal when it
changes to `-1`. When the strategy is long, the return over the holding period is the change in the
share price; when it is short, the return is the negative of that change:

```text
long return  = P_end / P_start - 1
short return = -(P_end / P_start - 1)
```

- `P_start` and `P_end` are the share prices when the position was opened and closed.

## A worked example

A made-up but plausible sequence of six days for one energy company. The cloud edges and the lagging
line are given, as if computed from the formulas above; the strategy only needs to read the
comparison. The position column shows what the rule holds after reading each day.

| Day | Lagging | Cloud top | Cloud bottom | Location | Crossing        | Position |
| --- | ------- | --------- | ------------ | -------- | --------------- | -------- |
| 1   | 80.00   | 78.00     | 74.00        | +1       | none            | long     |
| 2   | 79.50   | 78.20     | 74.10        | +1       | none            | long     |
| 3   | 77.00   | 78.50     | 74.50        | 0        | none            | long     |
| 4   | 76.00   | 78.80     | 74.80        | 0        | none            | long     |
| 5   | 73.00   | 79.00     | 75.00        | -1       | below the cloud | short    |
| 6   | 72.00   | 79.20     | 75.20        | -1       | none            | short    |

On day 5 the lagging line of 73.00 falls below the cloud bottom of 75.00, so the long is closed and a
short is opened. Days 3 and 4, when the line is inside the cloud, produce no change, which is why the
rule can appear to ignore the chart for weeks at a time.

Now the money. The table's lagging column is the comparison line, not the share price being traded,
so the trade prices are stated separately: the long was opened earlier at a share price of 80.00 and
is sold at the day 5 price of 74.00, and the short is opened at 74.00 and closed at the day 6 price
of 71.00.

```text
Long leg:  74.00 / 80.00 - 1 = -0.0750, that is -7.50 percent
Short leg: -(71.00 / 74.00 - 1) = -(-0.0405) = +4.05 percent
Combined:  0.9250 * 1.0405 = 0.9625, that is -3.75 percent
```

Four one-way trades were made: the earlier entry, the sale on day 5, the short on day 5, and the
closing purchase. At 0.05 percent per one-way trade:

```text
Cost = 4 * 0.0005 = 0.0020, that is 0.20 percent
Net = -3.75 - 0.20 = -3.95 percent
```

So the strategy lost 3.95 percent over the holding period. The long leg lost 7.5 percent; the short
leg recovered some of it. Two things are worth noticing. First, the rule was late: it held the long
through days 1 to 4 while the price was already turning. Second, the whipsaw risk is real: every
crossing costs two one-way trades, and a chart that crosses back and forth pays again each time. The
example says nothing about whether the strategy works over years; it only shows how to apply the rules
and how the costs accumulate.

## What the research actually found

| Source                                       | What it measured                                                            | Result                                                                                                                                                                                                  |
| -------------------------------------------- | --------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Gurrib, Kamalov and Elshareif                | The ten largest energy companies, daily data from April 2012 to July 2019   | The Ichimoku rule's total return ranged from 124 to 260 percent per company, against a range of minus 46 to plus 135 percent for simply holding each share, with reward-to-risk figures of 0.30 to 0.53 |
| The same paper, reported by the library page | A summary comparison, seven-year horizon                                    | The average return of the basket rose from 21.5 percent for holding to 194 percent for the rule                                                                                                         |
| QuantConnect's own replication               | The same universe, January 2015 to August 2020, with the look-ahead removed | Reward-to-risk of minus 0.31, against minus 0.083 for holding an energy sector fund; the rule beat the fund only during the 2020 oil price crash, with a reward-to-risk of 176.5                        |
| The same replication, the 2020 recovery      | March to June 2020                                                          | The rule's reward-to-risk fell to minus 1.56 while the energy fund earned 46.07, because the rule stayed short through the rebound                                                                      |

This is a direct disagreement between credible sources, and it is not a small one: one study finds
the rule turning a bare 21.5 percent into 194 percent, and the other finds it losing to a plain
energy fund over a similar period. The proposed explanation is that the original study chose its
universe using the index weights of the whole period, including companies that later left the index,
which is a form of using information from the future; the library page rebuilds the universe month by
month so that only companies known to be large at the time are used. Removing that advantage, and
running the rule from 2015, flips the conclusion. The grade is Disputed because the two measurements
cannot both describe the same tradable strategy, and the disagreement is unresolved.

## How this project relates to it

This repository does not implement the Ichimoku method, and it says so explicitly. The design note
[strategies/entry_exit_engine_design.md](../../../strategies/entry_exit_engine_design.md) lists
Ichimoku, together with parabolic SAR, Keltner channels, Fibonacci levels and pivot points, in its
Section 8, Rejected, and why. The stated reason is indicator proliferation: each of these is another
parameter with no independent evidence, and the note's governing constraint is that a rule is only
admitted on significant, material, out-of-sample evidence. That is the same standard the Disputed
grade above applies.

The closest measured evidence sits in the repository's
[predictability brief](../../../strategies/books2/08_predictability_and_trading_strategies.md).
Its Section 4 reports a study of indicator-only trading rules that win fewer than half their trades
and optimise into outliers, and its recommendation is to treat parameters chosen by search as
in-sample artefacts. An Ichimoku rule has at least three such parameters (the 9, 26 and 52 day
windows), which is exactly the pattern the note warns about.

## Where it goes wrong

- The window lengths are parameters. Nine, twenty-six and fifty-two days are conventions, not
  results. Trying several sets and keeping the best is how a rule is fitted to its own sample.
- Look-ahead in the universe. Choosing today's energy companies and running the rule back over their
  history uses the fact that they survived and grew, which flatters the result; the library page
  documents how much it matters.
- Whipsaw. Every crossing trades twice, and a chart that crosses back and forth in a sideways market
  pays repeatedly without a trend to earn back the cost.
- The comparison lines are lagged by design. The cloud is drawn twenty-six days forward, so its
  signal describes the past, and the paper reports that buy and sell signals mostly occurred outside
  periods of a strengthening trend, which weakens the interpretation that the cloud predicts turning
  points.
- Sample choice. The source study's window ends before the 2020 crash; the library page's window
  includes it, which supplies the one period where the rule shone. A reader should ask which period
  was chosen before or after seeing the result.
- The energy sector is small and correlated. The ten largest energy companies move together with the
  oil price, so the ten positions are close to one bet, and the apparent diversification is thin.

## Try it yourself

You need nothing but a spreadsheet and a public source of daily prices for one energy company, say
Exxon Mobil.

1. Build a sheet with one row per day: the date, the open, the high, the low and the close.
2. Add a column for the highest high over the last nine days, and one for the lowest low over the
   last nine days; average them to get the conversion line.
3. Do the same with twenty-six days to get the base line, and with fifty-two days to get span B.
4. Average the conversion and base lines to get span A.
5. Shift both spans down twenty-six rows, so that each day sees the spans computed twenty-six days
   earlier. Those are the cloud edges.
6. Add a column for the lagging line: the close, shifted up twenty-six rows so it appears as if drawn
   twenty-six days ago.
7. Add a column that says above, inside or below, comparing the lagging line with the cloud edges.
8. Mark the days the location changes to above (buy) and to below (sell), count the switches, and
   subtract 0.05 percent for each one-way trade.

What to notice: the line sits inside the cloud for long stretches, so the rule often does nothing for
weeks. Over the days it holds a position, compare the result with the result of simply holding the
share. Do this for two different companies and two different periods, and see whether the answer is
the same. If it is not, you have reproduced the disagreement above.

## Where this came from

- [QuantConnect strategy library: Ichimoku clouds in the energy sector](https://www.quantconnect.com/tutorials/strategy-library/ichimoku-clouds-in-the-energy-sector),
  the rules as implemented: the ten largest energy companies, the lagging line against the cloud, and
  the performance table.
- [Gurrib, Kamalov and Elshareif, Can the Leading US Energy Stock Prices be Predicted using the Ichimoku Cloud?](https://doi.org/10.32479/ijeep.10260),
  the underlying study. This tutorial cites its line definitions from p.4 and its performance table
  from p.9, and the library page's summary of its 21.5 percent to 194 percent result.
- [strategies/entry_exit_engine_design.md](../../../strategies/entry_exit_engine_design.md), this
  repository's design note listing Ichimoku among the rejected indicators and the reason.
- [The predictability brief](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  this repository's study of indicator-only rules and their in-sample optimism.

## Words used in this tutorial

- drawdown: the fall from a peak to the following low, measured in percent.
- indicator: a number computed from past prices and drawn on a chart to summarise them.
- look-ahead bias: using information that was not available at the time a decision would have been made.
- moving average: the average of the last fixed number of prices, updated as new prices arrive.
- range: the distance between the highest and lowest price over a chosen number of days.
- selling short: borrowing something you do not own, selling it, and buying it back later.
- whipsaw: a price that crosses a signal line back and forth, causing repeated losing trades.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
