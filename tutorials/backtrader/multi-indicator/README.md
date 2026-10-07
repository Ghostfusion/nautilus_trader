# Classic indicators: one stock, nine ways of asking the same question

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of a single large American technology company on daily bars, plus one Chinese stock and one futures contract in two of the nine                               |
| How often it trades       | A few times a year for most of the nine; the busiest made a few hundred trades over five years                                                                       |
| What you need             | A spreadsheet and five years of daily share prices, or Python and a data file                                                                                        |
| Where the rules come from | [The Strategy Compendium, article 17, classic single-indicator strategies](https://backtrader.readthedocs.io/en/latest/strategies-series/en/17-multi-indicator.html) |
| The underlying research   | None as a paper: the indicators come from their authors' books and magazine articles of the 1970s and 1980s and are restated in the library article                  |
| How well it held up       | Weak: one controlled backtest per indicator, on one share over five years, with no replication, and returns close to zero before any real trading cost               |
| Also appears in           | Nothing else in this collection                                                                                                                                      |

## The idea in one paragraph

Almost every famous indicator answers one question: where is today's price inside its recent range,
or how fast is it changing? Buy when the answer is at one extreme, sell when it is at the other. This
category runs nine of those indicators over the same five years of the same share, with the same
starting cash and the same commission, and lines up the results. The point is not that any one of
them is special. The point is that a controlled comparison is possible at all: one set of prices,
nine rules, one table. What the table shows is how little separates them and how much of each result
is noise.

## Why anyone believed it

The indicators were invented before backtesting existed. Their authors compressed market observations
into a formula on graph paper and tested them by hand, on the markets they traded. The reason the
rules survived is that they impose discipline: a trader following a stochastic rule buys in the same
situation every time instead of improvising, and the formula is transparent enough to argue about.
The economic story behind oversold-and-rebound is the same as in other reversal systems: a fast fall
forces some holders to sell at any price, and a patient buyer collects a small fee for absorbing it.

The counterparty is the forced or frightened seller again, and the reason some of these rules fail is
that a fall is sometimes news rather than panic. A range indicator cannot tell the two apart, which
is why the ones that trade least tend to look best.

## An everyday comparison

Nine cooks are given the same ingredients and asked for soup, and one judge tastes all nine. The
judge does not learn which cook is best; the judge learns that soup made from the same vegetables
tastes broadly the same, and that any difference between two bowls is as likely to be the taster's
mood as the recipe. Running nine indicators on one price series is that tasting. It is a fair test of
a method, not a search for the best recipe.

## The rules, step by step

All nine rules share the same settings: the share's daily bars from 2010 to 2014, a starting account
of 100,000, a commission of 0.1 percent per side, and a fixed purchase of 10 shares per trade. The
last detail matters and is discussed below.

1. Williams %R. Compute the indicator over 14 days, a number between -100 and 0. Buy when yesterday's
   reading was below -80 and today's reading is higher than yesterday's, which the library calls
   turning up out of oversold. Sell the whole position when today's reading is above -20.
2. Stochastic KD. Compute the fast line over 14 days and a slow line as its 3-day average. Buy when
   the fast line crosses above the slow line while the fast line is below 20. Sell when the fast line
   crosses below the slow line while the fast line is above 80.
3. Commodity Channel Index. Buy when the index crosses up through -100 and sell when it falls back
   below +100. The index measures how far the price has strayed from its own recent average, scaled
   by how much it usually strays.
4. Parabolic SAR. Buy when the price crosses above the trailing stop and sell when it crosses below.
   The stop accelerates as a trend continues.
5. TRIX. Compute the rate of change of a triple-smoothed average of the price. Buy when it crosses up
   through zero and sell when it crosses down. The triple smoothing makes it the slowest and least
   noisy momentum measure in the set.
6. Ultimate Oscillator. Compute the blended momentum measure over 7, 14 and 28 days. Buy when it is
   below 30 and sell when it is above 70.
7. Aberration. Compute 200-day Bollinger bands at two standard deviations around a moving average.
   Buy when the price breaks above the upper band, sell short when it breaks below the lower band,
   and close either position when the price returns to the middle line.
8. Aberration on shares. The same rule with the same periods, run on 22 years of a Chinese bank share.
9. UDVD. Go long when the three-day average of the candle bodies, the distance from open to close, is
   positive, and flat when it is negative. This is the simplest rule in the set.

## The maths, with every symbol named

The category rests on three families of formula.

Williams %R, where today's close sits in the recent range:

```text
%R = -100 * (HH_n - C) / (HH_n - LL_n)
```

- `HH_n` is the highest high over the last `n` days; `LL_n` the lowest low.
- `C` is today's closing price.
- `n` is the window, 14 days here.
- `%R` runs from 0, when the close sits at the top of the range, to -100, when it sits at the bottom.

The stochastic fast line, a rescaled version of the same idea:

```text
%K = 100 * (C - LL_n) / (HH_n - LL_n)
%D = the average of the last 3 values of %K
```

- `%K` runs from 0, when the close is at the low of the window, to 100, at the high.
- `%D` is a three-day average of `%K`, called the slow line, which lags by design.
- The classic parameters are 14 days for `%K` and 3 days for `%D`.

The Ultimate Oscillator, which blends three windows so that a short window's speed is tempered by a
long window's steadiness:

```text
BP   = C - min(L, PrevC)
TR   = max(H, PrevC) - min(L, PrevC)
avg7 = (sum of BP over 7 days)  / (sum of TR over 7 days)
avg14 = (sum of BP over 14 days) / (sum of TR over 14 days)
avg28 = (sum of BP over 28 days) / (sum of TR over 28 days)
UO   = 100 * (4 * avg7 + 2 * avg14 + 1 * avg28) / 7
```

- `C`, `H`, `L` are today's close, high and low; `PrevC` is yesterday's close.
- `BP`, buying pressure, measures how far the close rose above the weaker of the low and yesterday's
  close; it cannot go negative.
- `TR`, true range, is the day's full span including any gap.
- `avg7`, `avg14`, `avg28` are the ratios of total buying pressure to total range over each window.
- `UO` runs from 0 to 100. The weights 4, 2 and 1 give the shortest window the loudest vote, and the
  division by 7 keeps the result on the same 0-to-100 scale.

## A worked example

Williams %R on eight made-up daily bars for a share trading near 30. The library uses a 14-day
window; the window is shortened to 3 here so the arithmetic fits on the page and the rule is
unchanged.

| Day | High  | Low   | Close | %R (3) | Action        |
| --- | ----- | ----- | ----- | ------ | ------------- |
| 1   | 31.00 | 29.00 | 30.00 |        | watch         |
| 2   | 30.00 | 28.50 | 29.00 |        | watch         |
| 3   | 29.50 | 28.00 | 28.40 | -86.67 | watch         |
| 4   | 28.20 | 26.50 | 26.70 | -94.29 | watch         |
| 5   | 27.50 | 26.10 | 26.30 | -94.12 | buy at 26.30  |
| 6   | 27.00 | 26.00 | 26.70 | -68.18 | hold          |
| 7   | 27.80 | 26.60 | 27.60 | -11.11 | sell at 27.60 |
| 8   | 28.60 | 27.40 | 28.40 | -7.69  | flat          |

Day 3's reading is `-100 * (31.00 - 28.40) / (31.00 - 28.00) = -86.67`. Day 4's is
`-100 * (30.00 - 26.70) / (30.00 - 26.50) = -94.29`. On day 5, yesterday's reading of -94.29 is below
-80 and today's reading of -94.12 is higher, so the rule buys at the close of 26.30. On day 6 the
reading of -68.18 is not above -20, so the position is held. On day 7 the reading of -11.11 is above
-20, so the rule sells at the close of 27.60.

The purchase is 10 shares, so the money involved is small next to the 100,000 account:

```text
Gross profit per share = 27.60 - 26.30 = 1.30
Gross profit on 10 shares = 13.00
Buy commission  = 0.001 * 26.30 * 10 = 0.26
Sell commission = 0.001 * 27.60 * 10 = 0.28
Net profit = 13.00 - 0.26 - 0.28 = 12.46, about 0.012 percent of the account
```

That last line is the honest surprise of this category. The rules are tested on a 100,000 account but
buy only 10 shares, so even a good trade barely moves the needle. The five-year final value of
100,102.86 is a handful of such trades, and it is a rounding error on the starting cash. Whatever the
indicators prove here, they do not prove much about size.

## What the research actually found

The library reports nine backtests on the same share and period. The numbers below are its own.

| Rule                | Bars  | Final value | Reward-to-risk | Worst fall | Annual return |
| ------------------- | ----- | ----------- | -------------- | ---------- | ------------- |
| Williams %R         | 1,244 | 100,102.86  | 0.479          | 9.84%      | 0.0206%       |
| Stochastic KD       | 1,239 | 100,219.02  | 0.692          | 8.50%      | 0.0439%       |
| Ultimate Oscillator | 1,229 | 100,199.75  | 2.226          | 6.37%      | 0.0400%       |
| Parabolic SAR       | 1,255 | 100,044.47  | 0.158          | 14.47%     | (not given)   |

The reward-to-risk column is the return earned per unit of the strategy's own wobble, also called the
Sharpe ratio. The Ultimate Oscillator posts the highest of the group and the smallest worst fall, and
its author's point is exactly the blending of three windows. But an annual return of about 0.04
percent before any real cost is not a result to act on; it is a leaderboard among near-zero outcomes.
Parabolic SAR loses to the others because it is chopped about in a range, which its own authors say to
expect.

The two Aberration tests make a different point. The futures version made 94 trades and finished at
1,079,820 from a million, a reward-to-risk of 0.55. The same rule on 22 years of a Chinese bank share
turned 100,000 into 423,916.71, with a worst fall of 46.5 percent. Identical logic, violently
different risk portraits, because the market changed.

Nothing here is a paper, and every number is a single run. The grade is weak.

One thing must be said plainly, because it is the heart of this whole group of tutorials. Every
backtest in the compendium asserts its final value, its reward-to-risk ratio and its worst fall
against numbers captured when the strategy was migrated. The Ultimate Oscillator file, for example,
asserts 1,229 bars, a final value of 100,199.75, a reward-to-risk of 2.2256 and a worst fall of 6.37
percent, each to a set tolerance. Passing proves the engine computes exactly what the file says. It
proves nothing about whether the strategy earns anything.

## How this project relates to it

The repository's brief
[Overfitting, reproducibility and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md)
explains why a nine-way comparison like this one is easy to over-read. It records that mining simple
functions of 240 accounting variables produced 18,113 candidate strategies, of which about 30 percent
cleared a conventional significance bar (`2209.13623v3`, p.7), and it warns that the practical risk in
a search is the analyst's own choices rather than any single threshold. Running nine indicators and
picking the winner is a small version of that search, and the winner is chosen partly for having had
a good five years.

The same brief's companion in this collection,
[Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
records that indicator-only momentum rules win under half their trades, with parameters that differ
for every index (`2206.12282v1`). That is the pattern the table above shows directly.

## Where it goes wrong

- The stake, not the rule, decides the size. Ten shares against 100,000 makes every result a rounding
  error. A reader who scales the position up changes the drawdowns too, and the leaderboard may not
  survive.
- One share, one period. Five years of one large technology company cannot separate an indicator from
  its era. The Chinese share test shows how differently the same logic behaves elsewhere.
- The parameters are fitted. Period 14, thresholds 20 and 80, bands at two standard deviations: each
  is a choice, and the best choice on this sample need not be the best on the next.
- Reward-to-risk flatters small samples. A high Sharpe ratio over five years with a handful of trades
  is not stable; the Ultimate Oscillator's leader position rests on very few decisions.
- Costs are only commission. No spread, no slippage, and no penalty for the fact that a rule reading a
  close decides to trade after that close has already happened.
- Oversold is not the same as cheap. Every indicator here measures the shape of recent prices, not
  whether the share is worth its price, so a fall on bad news looks identical to a fall on panic.

## Try it yourself

You need a spreadsheet and one company's daily high, low and close for a few years.

1. Put the date, high, low and close in four columns.
2. Add a column for the 14-day highest high and one for the 14-day lowest low, each the largest and
   smallest value in the last 14 rows.
3. Add the Williams %R column: `-100 times (highest high minus close) divided by (highest high minus
   lowest low)`.
4. Add a column that marks buy when the previous row's value is below -80 and today's is higher, and
   sell when today's is above -20.
5. Mark the trades on a chart of the close.
6. Finally, count how many of the buy signals happened in the two months after the whole market
   dropped.

What to notice: the signals cluster in fast falls, and many of them are followed by further falls.
The rule has no way to know whether the fall was panic or news, which is why a filter of some kind is
always added in practice, and why the filter becomes the real strategy.

## Where this came from

- [The Strategy Compendium, article 17, classic single-indicator strategies](https://backtrader.readthedocs.io/en/latest/strategies-series/en/17-multi-indicator.html),
  the nine-strategy inventory, the indicator rules and the backtest figures quoted above.
- [Overfitting, reproducibility and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md),
  this repository's brief on multiple testing and the cost of a wide search (`2209.13623v3`).
- [Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  the brief recording that indicator-only rules win under half their trades (`2206.12282v1`).

## Words used in this tutorial

- Bollinger bands: a moving average with an upper and lower line drawn a few standard deviations away.
- buying pressure: a measure of how far the close rose above the weaker of the low and the prior close.
- commission: the fee a broker charges for a trade.
- oscillator: an indicator that swings between a fixed high and low rather than trending.
- oversold: a reading near the bottom of an indicator's range.
- range: the distance between the highest and lowest price over a chosen window.
- standard deviation: a measure of how widely a set of numbers is spread around its average.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
