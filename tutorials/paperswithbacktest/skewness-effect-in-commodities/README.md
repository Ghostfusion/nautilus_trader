# The skewness effect in commodities: buying the raw materials with the most one-sided bad luck

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                         |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Futures contracts (agreements to buy or sell a fixed amount at a set date) on about twenty-two raw materials, from cocoa and coffee to gold, copper and crude oil                                                                                                                             |
| How often it trades       | Once a month, when the ranking is redone and the portfolio is rebuilt                                                                                                                                                                                                                         |
| What you need             | A spreadsheet and twelve months of daily prices for each raw material                                                                                                                                                                                                                         |
| Where the rules come from | [Quantpedia, skewness effect in commodities](https://quantpedia.com/strategies/skewness-effect-in-commodities/)                                                                                                                                                                               |
| The underlying research   | Dujava and Vojtko, [An Evaluation of the Skewness Model on 22 Commodities Futures](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=4488725), on the earlier finding of Fernandez-Perez, Frijns, Fuertes and Miffre                                                                        |
| How well it held up       | Mixed: rests on a long-short version that earned 9.51 percent a year over 1990 to 2022 with a Sharpe ratio of 0.82 and a worst fall of 17.46 percent, an unusually tidy record, but one that comes from one asset class and one measure of skewness, with a short leg that carries real costs |
| Also appears in           | [The return asymmetry effect in commodity futures](../return-asymmetry-effect-in-commodity-futures/README.md) and [expected idiosyncratic skewness](../../../tutorials/quantconnect/expected-idiosyncratic-skewness/README.md)                                                                |

## The idea in one paragraph

For each raw material, measure how lopsided its daily price changes have been over the past year. A material whose
big moves are mostly upward has a positive measure called skewness; one whose big moves are mostly downward has a
negative one. The evidence is that materials with positive skewness earn less than materials with negative
skewness, because investors overpay for the chance of a large gain. This strategy sorts the materials into five
equal groups by skewness, buys the group with the lowest, most negative skewness, sells short the group with the
highest, most positive skewness, and rebuilds each month. It is a bet on the shape of each material's returns, not
on the direction of the market.

## Why anyone believed it

Most textbook models assume returns are shaped like the familiar bell curve, which is symmetric, so that a gain of
two percent and a loss of two percent are equally likely. Real returns are not symmetric. Some commodities have a
long right tail: nothing happens for months, then a shortage sends the price up sharply. Others have a long left
tail: nothing happens, then a glut or a collapse sends it down sharply.

If investors treat the right-tailed commodities as lottery tickets, they hold them for the chance of the jackpot
and accept a lower average return for the pleasure. That makes the right-tailed commodities expensive and their
future returns low. The left-tailed commodities are unpleasant to hold, so they must be cheaper and offer higher
future returns to attract anyone. The counterparty, then, is the lottery-seeking investor, and the evidence cited
below finds the pattern across a long commodity sample. This is the same economic story as the return-asymmetry
strategy in this collection; the difference is that this one measures the lopsidedness with the standard
statistical quantity rather than with a count of extreme days.

## An everyday comparison

Two students take the same exam. One scores 70 in every subject, so the marks cluster tightly around the middle.
The other scores 95 in the subjects they like and 40 in the subjects they do not, so the marks are spread out and
lopsided, with a long tail of high marks and a short tail of low ones. If a prize is awarded for a single brilliant
mark, people will queue for the second student and overestimate what the average result will be. The strategy is to
look for the unglamorous steady student and to avoid, or bet against, the one whose results depend on a rare lucky
day.

## The rules, step by step

1. Choose the raw materials. The paper uses twenty-two commodity futures; the implementation carries a longer list
   of about twenty-seven, adding contracts such as milk and gasoline.
2. For each material, collect the last twelve months of daily prices. The implementation uses 252 trading days,
   which is twelve months times twenty-one days.
3. Turn the prices into daily returns: today's price divided by yesterday's price, minus one.
4. Compute the skewness of those daily returns. Skewness is the third statistical moment, explained below; a
   positive value means the tail on the right is longer, a negative value means the tail on the left is longer.
5. Rank the materials by skewness, from the most negative to the most positive.
6. Split the ranked list into five equal groups, called quintiles. With twenty-two materials each group holds
   about four or five; the implementation divides the count by five and takes that many from each end.
7. Buy every material in the lowest group, giving each an equal share of the long money.
8. Sell short every material in the highest group, giving each an equal share of the short money. Selling short
   means borrowing something you do not own, selling it now, and buying it back later.
9. Hold for one month, then repeat from step 2. A material that is not in either group is closed out.

## The maths, with every symbol named

A distribution of returns is described by its moments, and the first three have plain meanings. The first moment is
the average, which says where the middle is. The second moment is the variance, and its square root is the standard
deviation, which says how spread out the returns are. The third moment, once it is scaled, is the skewness, which
says whether the spread is lopsided. A bell curve has skewness zero.

The average of the daily returns:

```text
r_mean = (r_1 + r_2 + ... + r_N) / N
```

- `r_mean` is the average daily return over the window.
- `r_i` is the return on day `i`, as a decimal, so 0.01 means 1 percent.
- `N` is the number of daily returns, 252 in the implementation.

The standard deviation:

```text
s = square root of ( (1/N) * sum of (r_i - r_mean) squared )
```

- `s` is the standard deviation, the typical distance of a daily return from the average.
- `sum` means add up the term for every day.

The skewness:

```text
skew = ( (1/N) * sum of (r_i - r_mean) cubed ) / (s cubed)
```

- `skew` is the skewness, a unit-free number.
- The numerator is the average of the cubed distances from the mean, which is the third moment.
- Dividing by `s` cubed removes the units, so a material with a ten percent standard deviation and a one percent
  standard deviation can be compared.
- A positive `skew` means the extreme values are mostly above the average; a negative `skew` means they are mostly
  below. Some software divides by one fewer observation than `N`, which changes the answer slightly for small
  samples but not the ranking.

The portfolio return is then the average of the chosen materials' returns, taken separately on each side:

```text
R_long = (R_1 + R_2 + ... + R_L) / L
R_short = (R_1 + R_2 + ... + R_S) / S
R_total = R_long - R_short
```

- `L` is the number of materials in the low group and `S` the number in the high group.
- `R_total` is the portfolio return; the short side's average return is subtracted because a fall there is a gain.
- The subtraction is written as minus because a short position gains when the average of its returns is negative.

## A worked example

Five materials, each with five monthly returns, reduced to a skewness. Five observations is far too few for real
use, but it shows the arithmetic. The calculation for wheat is spelled out in full; the others use the same steps.
The returns are invented but of the size commodity returns actually take.

| Material | Return 1 | Return 2 | Return 3 | Return 4 | Return 5 | Average | Standard deviation | Skewness | Rank |
| -------- | -------- | -------- | -------- | -------- | -------- | ------- | ------------------ | -------- | ---- |
| Silver   | +1.0     | +2.0     | +2.0     | +3.0     | +12.0    | 4.0     | 4.05               | +1.409   | 5    |
| Coffee   | -1.0     | 0.0      | +1.0     | +2.0     | +3.0     | 1.0     | 1.41               | 0.000    | 4    |
| Gold     | -3.0     | -1.0     | 0.0      | +2.0     | +2.0     | 0.0     | 1.90               | -0.351   | 3    |
| Copper   | -8.0     | -1.0     | 0.0      | +2.0     | +2.0     | -1.0    | 3.69               | -1.149   | 2    |
| Wheat    | -12.0    | +1.0     | +1.0     | +2.0     | +2.0     | -1.2    | 5.42               | -1.475   | 1    |

The wheat calculation, all figures in percent. The average is the sum of minus 12, plus 1, plus 1, plus 2, plus 2,
which is minus 6, divided by 5, giving minus 1.2. The five distances from the average are minus 10.8, plus 2.2,
plus 2.2, plus 3.2 and plus 3.2. Their squares add to 146.8, which divided by 5 gives a variance of 29.36, and the
square root is 5.42, the standard deviation. The cubes of the same five distances are minus 1259.7, plus 10.6,
plus 10.6, plus 32.8 and plus 32.8, which add to minus 1172.9. Dividing by 5 gives minus 234.6, and dividing that
by 5.42 cubed, which is 159.1, gives minus 1.475. The negative sign says the one large fall outweighs the small
rises.

With five materials, the lowest fifth and the highest fifth each hold a single material. The lowest fifth is wheat
and the highest fifth is silver. The real study has twenty-two to twenty-seven materials, so each fifth holds four
or five. Suppose the next month brings wheat plus 3.0 percent and silver minus 2.0 percent.

| Material | Side  | Weight | Next-month return | Contribution |
| -------- | ----- | ------ | ----------------- | ------------ |
| Wheat    | long  | 1.0000 | +3.0 percent      | +3.0000      |
| Silver   | short | 1.0000 | -2.0 percent      | +2.0000      |
| Total    |       | 2.0000 |                   | +5.0000      |

The gross return for the month is +5.0 percent. The portfolio holds 100 percent long and 100 percent short, so
replacing both legs completely trades four times the account's size, and at ten basis points per side:

```text
cost = 4 * 0.001 = 0.004, that is 0.4 percent of the account
net return for the month = 5.0 - 0.4 = 4.6 percent
```

Twelve months at that rate would be about 71 percent a year, which is far above the published 9.51 percent, and
that gap is deliberate: one invented month with a lucky ranking is not evidence. The published figure is the average
of hundreds of months, most of which are close to zero. If the short positions were held by borrowing shares rather
than by using futures, the borrow fee would come on top of the trading cost.

## What the research actually found

| Source                                                                | What it measured                                                                           | Result                                                                                                                                                                          |
| --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Dujava and Vojtko, An Evaluation of the Skewness Model                | Long the lowest skewness fifth and short the highest, twenty-two commodities, 1990 to 2022 | 9.51 percent a year, an annualized volatility of 11.64 percent, a worst fall of 17.46 percent and a Sharpe ratio of 0.82 on the long-short version                              |
| The same study, robustness checks                                     | Whether the result depends on one specification                                            | It held across long-only, short-only and combined versions, across time windows, and adding momentum did not improve it; skewness was called a stable cross-sectional predictor |
| Fernandez-Perez, Frijns, Fuertes and Miffre, Commodities as Lotteries | Buying low-skewness and shorting high-skewness commodity futures                           | The paper reports that the strategy generates a significant return, and ties it to the overpricing of lottery-like commodities                                                  |

The list that carries this strategy does not publish its own Sharpe ratio for it. Its general replication record,
which is the vendor's own measurement, is a median Sharpe ratio of 0.37 across the papers it has coded, with 48
percent clearing a t-statistic of 1.96. The 0.82 here is well above that median, and the 17.46 percent worst fall
is mild for a strategy that holds a short book. That combination is either the sign of a genuinely better effect or
the sign that one specification was found after trying several, and the sources used here cannot tell the two apart.

## How this project relates to it

The repository's commodity survey,
[Energy and commodity markets](../../../strategies/books2/18_energy_and_commodities.md), shows that the expected
shape of a commodity's returns is not the only thing that matters: the shape of its future price curve carries its
own information, and the level, slope and curvature of that curve explained on average 96.5 percent of the
variation across twenty-one futures. A strategy that ranks only on past return skewness leaves that information
unused.

The second link is
[Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md).
It reports that cross-sectional predictors such as this one are mostly genuine rather than artefacts of searching,
and that even a marginal strategy design, in its example a trend rule with one position, has a positive skewness
that depends on the horizon at which it is measured. That is the mirror image of this tutorial: here skewness is
the signal, but skewness is also a property of any trading rule, so a backtest's own skewness has to be measured at
the horizon it is meant to be held for. The same repository also carries a finished tutorial on idiosyncratic share
skewness in the sibling library:
[expected idiosyncratic skewness](../../../tutorials/quantconnect/expected-idiosyncratic-skewness/README.md).

## Where it goes wrong

- Shorting is expensive for the names most likely to be short. The high-skewness materials are the ones with the
  most exciting upside, so they are the ones most likely to be squeezed higher and hardest to borrow. A short leg
  can lose far more than the long leg gains when a shortage arrives.
- Skewness is unstable on a year of data. A single extreme day can flip a material from positive to negative,
  because the third moment puts heavy weight on the largest values. Rank changes from month to month are therefore
  common, and each change costs money.
- The measure is a fixed choice. Twelve months of daily data, five groups and equal weights are all decisions. The
  same data ranked by a different window or a different number of groups can give a different answer.
- One asset class, one measure. The published sample is commodity futures only, and the measure is the plain
  skewness of returns. Similar-sounding studies use other assets or other definitions, so they are not a
  replication.
- Returns are not returns to the account. The published 9.51 percent a year is before the costs of rolling futures
  forward and before any borrow fee, both of which are real and both of which fall on a strategy that rebuilds its
  whole book every month.
- A crowded trade. If many investors believe the lottery story at the same time, the cheap left-tailed materials
  stop being cheap, and the effect they were supposed to earn disappears.

## Try it yourself

You need a spreadsheet and a public source of daily commodity prices.

1. Build a sheet with one column per commodity and one row per trading day for the last two years.
2. Add a block that turns each pair of consecutive prices into a daily return.
3. For each commodity, take the most recent 252 returns and compute the average.
4. Add a column of the distance from the average for each of those 252 rows, a column of that distance squared, and
   a column of that distance cubed.
5. Average the squared column to get the variance, take its square root to get the standard deviation, cube that,
   and divide the average of the cubed column by it. That is the skewness.
6. Rank the commodities by skewness and note the five lowest and the five highest.
7. In the row below, average the next month's returns of the five lowest and separately of the five highest, and
   subtract the second from the first.

What to notice: the skewness of a commodity is dominated by its single most extreme day in the year. Delete that
one day and recompute; the rank often moves by several places. If the ranking is that fragile, a strategy built on
it is fragile in the same way.

## Where this came from

- [Quantpedia, skewness effect in commodities](https://quantpedia.com/strategies/skewness-effect-in-commodities/),
  the rules, the performance figures and the references.
- The implementation file the list carries, `static/strategies/skewness-effect-in-commodities.py`, whose header
  states the twenty-two contracts, the twelve-month skewness window, the five groups and the monthly rebuild.
- Dujava and Vojtko, [An Evaluation of the Skewness Model on 22 Commodities Futures](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=4488725),
  the study behind the rules.
- Fernandez-Perez, Frijns, Fuertes and Miffre, [Commodities as Lotteries: Skewness and the Returns of Commodity Futures](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2671165),
  the earlier finding the later study builds on.
- [Energy and commodity markets](../../../strategies/books2/18_energy_and_commodities.md) and
  [Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  this repository's surveys of the commodity and predictability evidence.

## Words used in this tutorial

- mean: the average of a set of values.
- moment: a summary number that describes the shape of a distribution; the first is the average, the second the
  spread, the third the lopsidedness.
- rank: the position of a value in a sorted list, where rank 1 is the smallest or largest as stated.
- short selling: borrowing something you do not own, selling it now, and buying it back later.
- skewness: a unit-free number saying whether a distribution's tail is longer on the right (positive) or the left
  (negative); a bell curve has skewness zero.
- standard deviation: a measure of how far values typically sit from their average; a larger number means more
  spread out.
- quantile: a group holding an equal share of a ranked list; a quintile is one of five such groups.
- variance: the square of the standard deviation, so a measure of spread in squared units.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
