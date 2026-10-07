# Ranking large shares by the part of their return the market does not explain

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                              |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of the thirty large American companies that make up the Dow Jones Industrial Average                                                                                                                                        |
| How often it trades       | Once a month, when the two chosen shares are swapped for whichever two now rank highest                                                                                                                                            |
| What you need             | A spreadsheet and daily prices for the shares and one broad market index fund                                                                                                                                                      |
| Where the rules come from | [QuantConnect strategy library, CAPM alpha ranking on the Dow 30](https://www.quantconnect.com/tutorials/strategy-library/capm-alpha-ranking-strategy-on-dow-30-companies)                                                         |
| The underlying research   | William Sharpe's Nobel lecture, [Capital Asset Prices](https://www.nobelprize.org/uploads/2018/06/sharpe-lecture.pdf), the model the ranking is built from                                                                         |
| How well it held up       | Weak: one tutorial states the rule, no independent study repeats it, and the tutorial's own test of 2015 lost about 15 percent                                                                                                     |
| Also appears in           | [Residual momentum](../residual-momentum/README.md), which ranks shares by the very same leftover part of the return, and [Beta factors in stocks](../beta-factors-in-stocks/README.md) on the market-sensitivity number used here |

## The idea in one paragraph

When you own a share, part of its day-to-day movement comes from the whole market moving and part
comes from the company itself. The market part is easy to explain and gives no special credit. The
company part is the leftover. This strategy measures, for each of the thirty Dow shares, how large
that leftover has been over the past month, ranks the thirty shares by it, and buys the two with the
largest leftover. It holds those two for a month, then repeats. The bet is that a company whose
shares have quietly outrun what the market alone would predict will keep doing so for a while longer,
because news about a company spreads gradually.

## Why anyone believed it

Company news does not arrive all at once. A retailer may report good sales, then a broker raises its
view, then a supplier mentions strong orders; each step nudges the price a little. Investors who hear
the news late, or who need time to study it, buy after the first move, so the price keeps drifting up
for weeks. This is called underreaction: the market updates too little and too slowly.

The counterparty, then, is the slow or distracted investor, and the fund that is forced to sell for
reasons unrelated to the outlook, such as paying withdrawals or trimming a position that grew too
large. If those sellers keep reappearing, the share that has been quietly beating its market
prediction keeps beating it for a while. The strategy is a way of finding those shares with a ruler
rather than a hunch.

## An everyday comparison

Think of delivery times in a city. On a rainy rush hour every courier is slower, and a courier who
takes forty minutes when the city average is forty also is doing nothing special. But one courier
takes thirty-five minutes when the average is forty, and thirty on a day when the average is
thirty-five. That courier is consistently ahead of what the weather and traffic alone would predict.
You would expect that courier to be ahead again next week, because the advantage is a route the
others do not use, not a lucky day. The strategy looks for the couriers, meaning the shares, that are
persistently ahead of the city.

## The rules, step by step

1. Write down the thirty shares that make up the Dow index today. The list changes rarely; the
   QuantConnect page notes that the last change before it was written happened on 19 March 2015, so
   the earliest date it can be tested honestly is that day.
2. At the start of each month, collect the daily closing prices of those thirty shares and of one
   broad market index fund, such as the fund that tracks the S&P 500, over the last twenty-one
   trading days.
3. Turn each price series into daily returns: today's closing price divided by yesterday's closing
   price, minus one.
4. For each share, fit a straight line that predicts the share's daily return from the market's daily
   return. The line has two numbers: a slope, which measures how strongly the share moves with the
   market, and an intercept, which is the average amount the share returned beyond what the market
   move alone predicts.
5. Rank the thirty shares by that intercept, largest first.
6. Sell everything the account holds, then buy the top two shares, putting the whole account value
   into each one. That means the account holds twice its own money, so it borrows the rest; this is a
   leverage of two.
7. Hold the two shares for one month without looking at them, then repeat from step 2.

One honest note on the page's own code. The rules above describe the standard measure, in which the
share's return is predicted from the market's. The published implementation fits the line the other
way round, predicting the market from the share, so the number it calls the intercept is not the same
quantity the text describes. A reader applying the rules should use the standard direction; the
discrepancy is reported here because it changes what the ranking means.

## The maths, with every symbol named

The model behind the ranking is the capital asset pricing model. It says a share's return is the
reward for the market's movement plus a leftover:

```text
r_a = r_f + beta_a * (r_m - r_f) + error
```

- `r_a` is the return of the share over one period, written as a decimal.
- `r_f` is the interest you could earn with no risk, such as a short-term government bill.
- `r_m` is the return of the whole market over the same period.
- `beta_a` is how strongly the share moves when the market moves.
- `error` is the leftover part, the return the market does not explain.

Because the risk-free interest appears on both sides once you rearrange, the practical form of the
model is a straight line fitted to the data:

```text
r_a = alpha_a + beta_a * r_m + error
```

- `alpha_a` is the intercept of the fitted line, the average leftover return.
- The slope `beta_a` and the intercept `alpha_a` come from the fitted line, and their formulas are:

```text
beta_a  = Cov(r_a, r_m) / Var(r_m)
alpha_a = mean(r_a) - beta_a * mean(r_m)
```

- `Cov(r_a, r_m)` is the covariance, the average of the two series' movements measured together.
- `Var(r_m)` is the variance of the market returns, the average squared distance from their mean.
- `mean(r_a)` and `mean(r_m)` are the ordinary averages of the two return series.

The slope is how much the share moves per unit of market movement, and the intercept is what is left
once you have removed that predictable part. Ranking the thirty shares by `alpha_a` and buying the top
two is the whole strategy.

The cost of rebuilding the portfolio each month is one line:

```text
Cost = t * c
```

- `t` is the value traded, counting both the selling and the buying, divided by the value of the
  account. A completely replaced holding of the whole account gives `t = 2.0`.
- `c` is the cost of one trade as a fraction of the amount traded, covering the gap between the buying
  and selling price plus any commission. Five basis points, that is 0.0005, is a cautious figure for
  very liquid American shares; one basis point is one hundredth of one percent.

## A worked example

The numbers below are invented, and the daily returns are exaggerated so the arithmetic is easy to
follow. Real daily differences are a small fraction of these. Six days of daily returns in percent:

| Day  | Market | Share A | Share B | Share C | Share E |
| ---- | ------ | ------- | ------- | ------- | ------- |
| 1    | 1.00   | 1.10    | 0.70    | -0.80   | 1.40    |
| 2    | -0.50  | -0.40   | -0.05   | -0.05   | -0.85   |
| 3    | 0.50   | 0.60    | 0.45    | -0.55   | 0.65    |
| 4    | 1.50   | 1.60    | 0.95    | -1.05   | 2.15    |
| 5    | -0.50  | -0.40   | -0.05   | -0.05   | -0.85   |
| 6    | 1.00   | 1.10    | 0.70    | -0.80   | 1.40    |
| Mean | 0.50   | 0.60    | 0.45    | -0.55   | 0.65    |

Now fit the line for share A, using the formulas above. The market mean is 0.50 and the squared
distances from it are 0.25, 1.00, 0.00, 1.00, 1.00 and 0.25, which add to 3.50. Share A is exactly
the market plus 0.10 each day, so its distances from its own mean are the same six numbers, and:

```text
Var(r_m) = 3.50 / 6 = 0.58333
Cov(r_a, r_m) = 3.50 / 6 = 0.58333
beta_A  = 0.58333 / 0.58333 = 1.00
alpha_A = 0.60 - 1.00 * 0.50 = 0.10
```

Applying the same steps to the other shares gives the table below; the fitted values are exact here
because the series were built that way, while real data always has scatter around the line.

| Share | Beta  | Alpha (percent per day) | Rank |
| ----- | ----- | ----------------------- | ---- |
| B     | 0.50  | 0.20                    | 1    |
| A     | 1.00  | 0.10                    | 2    |
| E     | 1.50  | -0.10                   | 3    |
| C     | -0.50 | -0.30                   | 4    |

The two shares with the largest leftover are B and A, so the account buys B and A. Suppose the
previous month it had held C and E, so all four positions change, and suppose over the coming month
B returns 4.0 percent while A returns -2.0 percent. Because each share is bought with the full
account value:

```text
Return before costs = 1.00 * 4.0 + 1.00 * (-2.0) = +2.0 percent
```

All of the old holding is sold and all of the new holding is bought, so `t = 4.0` (two positions sold,
two bought, each equal to the whole account):

```text
Cost = 4.0 * 0.0005 = 0.002, that is 0.20 percent
Net return for the month = 2.0 - 0.20 = +1.80 percent
```

Two things are worth noticing. First, the cost line is not small, because the leverage doubles every
traded amount. Second, the example says nothing about whether the rule works; it only shows how to
apply it. The published test of this rule did not make money.

## What the research actually found

| Source                                                                  | What it measured                                                       | Result                                                                                                                                                                                                       |
| ----------------------------------------------------------------------- | ---------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Sharpe, the Nobel lecture and the model it describes                    | American shares from the 1920s onward                                  | A share's movement is largely explained by the market, and the leftover part does not command a permanent reward simply for existing; the model is a description of returns, not a promise about a rule      |
| QuantConnect tutorial, the rule's own test                              | Dow 30, twenty-one days of daily returns, top two by intercept, 2015   | A return of -15.463 percent for the year and a margin call in January; the page attributes the loss to higher market volatility, which reduces how reliable the fitted slope and intercept are               |
| Most claimed cross-sectional findings are likely true (`2206.15365v10`) | The published record of sorting shares by some measured characteristic | The false-discovery estimate is at or below 25 percent on the easy bound and 8.5 percent on a tighter one, so most such signals are probably real, but the bound rises to 41.7 percent under value weighting |
| This repository's own reading of the forecasting evidence               | The cross-sectional predictability literature                          | Findings in the cross-section are mostly genuine, so the binding problems are not "does the signal exist" but decay, crowding and how honestly the test is run                                               |

Read together, the picture is this. The general idea, that a share can quietly outrun its market
prediction and keep doing so for a while, sits inside a broad body of evidence that some such
sorting signals are real. But the specific rule here, ranking thirty shares by one month of leftover
return and holding the top two with borrowed money, has no independent test, and the one test that
exists lost money in a turbulent year. The tutorial itself claims the rule "performs well when the
market is smooth", which is a statement about a regime, not a measured effect. The grade is Weak.

## How this project relates to it

This repository holds a tutorial on the closest possible relative of this idea,
[Residual momentum](../residual-momentum/README.md), which ranks shares by the part of the return the
market does not explain and then trades a long-and-short portfolio rather than a leveraged pair. Its
research brief reports a long American sample and a later global replication, and is the place to see
whether the leftover part of a return is worth ranking at all. The number that does the explaining,
the market sensitivity called beta, is the subject of
[Beta factors in stocks](../beta-factors-in-stocks/README.md).

There is also the repository's own teaching manual on the general craft,
[Factor research and portfolio construction](../../../docs/usermanauls/factor-portfolio/README.md).
It builds one complete ranking study end to end and shows the two steps this strategy skips: it uses
only information that existed at the decision moment, and it tests the ranking on data the ranking
never saw. The brief [Portfolio construction and allocation](../../../strategies/books2/10_portfolio_and_allocation.md)
is where this repository collects what happens when the inputs to such a ranking are estimated
rather than known.

## Where it goes wrong

- The intercept is estimated from twenty-one days. A month of daily returns is a very short sample,
  and the statistical error around the intercept is large relative to its size, so much of the
  ranking may be noise rearranged rather than signal.
- Leverage doubles every move. Holding each of two shares with the whole account value means the
  account is twice invested, so an ordinary bad month can trigger a margin call, which the page
  reports happening in January. Margin interest is another unlisted cost.
- The coefficients are unstable in volatile markets. The page's own diagnosis is that when market
  volatility rises, the fitted line's reliability falls and the rule stops working, which is exactly
  when losses arrive.
- The share list is chosen after the fact. Using today's thirty names and starting only in 2015
  avoids the worst of it, but any test that used an older list would quietly use shares that were
  later removed for poor performance.
- Crowding. Ranking shares by recent out-performance is a crowded idea, and the evidence in this
  repository says published signals decay as more money runs them.
- The rule can be inverted by a different definition. Whether "better than the market" means a large
  positive intercept, a large positive slope, or the reverse, changes which shares are bought; the
  page's own code and text disagree on the direction of the fit, which is a warning about how fragile
  the specification is.

## Try it yourself

You need a spreadsheet and any free source of daily prices for a handful of large shares and one
index fund.

1. Build a sheet with one column per share and one column for the index fund, one row per day for
   about thirty days.
2. Add a return column for each: today's price divided by yesterday's price, minus one.
3. For one share, compute the market's average return and each day's distance from it, then square
   those distances and average them. That is the variance.
4. Multiply each day's market distance by that day's share distance, and average the products. That
   is the covariance.
5. Divide the covariance by the variance to get beta, and subtract beta times the market average from
   the share average to get the intercept.
6. Repeat for every share and sort the intercepts.
7. Take the top two, and note what they returned over the following thirty days, after subtracting
   about five basis points on each side of each position.

What to notice: the ordering changes a lot from month to month, and the intercepts are tiny compared
with the daily wiggles that produced them. If the sorted shares look like a reliable team, you have
probably sorted the same lucky month twice rather than found a durable quality.

## Where this came from

- [QuantConnect strategy library: CAPM alpha ranking on the Dow 30](https://www.quantconnect.com/tutorials/strategy-library/capm-alpha-ranking-strategy-on-dow-30-companies),
  the rules as implemented: twenty-one days of daily returns, a line fitted per share, the top two by
  intercept, rebuilt monthly, and the page's own 2015 result.
- William Sharpe, [Capital Asset Prices, the Nobel lecture](https://www.nobelprize.org/uploads/2018/06/sharpe-lecture.pdf),
  the model that splits a share's return into a market part and a leftover.
- [Dow Jones Industrial Average](https://en.wikipedia.org/wiki/Dow_Jones_Industrial_Average), the
  index and its component history, which the library page cites.
- `2206.15365v10`, Most claimed statistical findings in cross-sectional return predictability are
  likely true (2022), as reported in
  [this repository's predictability brief](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  which is where the false-discovery figures come from.
- [Portfolio construction and allocation](../../../strategies/books2/10_portfolio_and_allocation.md),
  this repository's brief on what happens when the inputs to a ranking are estimates rather than
  facts.

## Words used in this tutorial

- alpha: the part of a share's return that the market's own movement does not explain.
- beta: a number for how strongly a share moves when the market moves; a beta of one moves about as
  much as the market.
- covariance: an average of how two series move together, positive when they tend to rise and fall in
  step.
- leverage: investing with borrowed money, so a small move in the holdings becomes a larger move in
  the account.
- margin call: a demand from the broker to add money or sell holdings, because borrowed money has
  been lost on falling prices.
- regression: fitting a straight line through points, here used to split a share's return into a
  market part and a leftover.
- tracking error: the leftover return a fitted model does not explain.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
