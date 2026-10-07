# Two ways to decide whether two prices move together

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                                       |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Two funds at once, one bought and one sold short, so the bet is on how the two prices move relative to each other rather than on the market as a whole                                                                                                                                                                      |
| How often it trades       | The pair is chosen once a month and the trade can open and close within that month, sometimes daily                                                                                                                                                                                                                         |
| What you need             | A spreadsheet and a few years of daily prices for the two funds you want to compare                                                                                                                                                                                                                                         |
| Where the rules come from | [QuantConnect strategy library, pairs trading copula vs cointegration](https://www.quantconnect.com/tutorials/strategy-library/pairs-trading-copula-vs-cointegration)                                                                                                                                                       |
| The underlying research   | Stander, Marais and Botha, [Trading strategies with copulas](https://jefjournal.org.za/index.php/jef/article/view/278) (2013), and the comparison in Rad, Low and Faff (2016)                                                                                                                                               |
| How well it held up       | Disputed: the library's page and one strand of the literature find the copula method better, while a large independent study finds it clearly worse than the older cointegration method, and the disagreement is unsettled                                                                                                  |
| Also appears in           | [Pairs trading with stocks](../pairs-trading-with-stocks/README.md), [Intraday pairs trading with cointegration](../intraday-dynamic-pairs-trading-using-correlation-and-cointegration-approach/README.md) and [Mean-reversion statistical arbitrage](../mean-reversion-statistical-arbitrage-strategy-in-stocks/README.md) |

## The idea in one paragraph

Pairs trading picks two things whose prices have moved together, and bets that when they drift apart
they will come back together. The whole job is deciding when the pair has drifted far enough to be
worth a bet, and this page compares two ways of deciding. The older way, cointegration, fits a
straight line through the two prices and measures how far the pair has wandered from it. The newer
way, a copula, describes the two prices by how unusual each one's recent move is and how those
unusual moves tend to arrive together. Neither forecasts direction; both bet that a stretch springs
back.

## Why anyone believed it

Two funds that hold similar things should cost similar amounts. If one of them is a broad technology
fund and the other a narrower technology fund, their prices are driven by the same companies and the
same news, so a sudden gap between them is more likely to be a temporary mistake than a permanent
change. The seller of the cheap fund is often someone who needs cash, or who is rebalancing, or who
simply noticed one price move and not the other. The buyer of the expensive one is often chasing a
move that has already happened. Both are behaving for their own reasons, not because the two funds
have become genuinely different. It can keep working because the two funds' holdings do not change
quickly: as long as they still own similar things, a gap should close. The moment that stops being
true the gap is real and the bet loses. The craft is separating a temporary stretch from a permanent
break, and that is where the two methods disagree.

## An everyday comparison

Two friends walk to the same station by the same route, one slightly ahead. Most days their distance
is about the same, and when one stops to tie a shoe you would expect the gap to close at the next
corner. That is cointegration: the gap around a stable path is what matters. But some days one takes
a shortcut and the ordinary gap rule gives a false alarm. A copula method asks a different question:
given how late one friend is, how late is the other likely to be, and is this pair of delays unusual
for these two. The first method watches the distance; the second watches the joint pattern.

## The rules, step by step

Both methods start the same way. Assemble a list of funds that hold something in common, such as two
technology funds or two gold funds, so that a relationship is plausible before any number is computed.

The cointegration method:

1. Each month, take the daily closing prices of the two funds over the last twelve months, and work
   with the natural logarithm of each price, which turns percentage moves into plain differences.
2. Fit a line predicting the log price of the second fund from the log price of the first. The slope,
   called beta, says how many units of the second fund move with one unit of the first. It also sets
   the size ratio between the two sides of the trade.
3. Form the spread: the log price of the first fund minus beta times the log price of the second.
   When the spread is low, the first fund is cheap relative to the second.
4. Compute the average and the standard deviation of that spread over the same twelve months.
5. Open a trade when the spread is more than one standard deviation from its average: buy the cheap
   fund and sell short the expensive one, sizing the two legs by beta, and close when the spread
   returns to its average. Repeat each month.

The copula method:

1. Each month, take the same twelve months of daily prices and convert them to daily log returns.
2. Measure how the two return series move together with a rank correlation, which counts how often the
   two series rise and fall on the same days rather than how large the moves are, and choose the
   candidate pair with the highest value.
3. For each fund, estimate from the data how its returns are distributed, and turn each day's return
   into a rank between 0 and 1. The best day of the year becomes a rank near 1 and the worst near 0.
4. Fit a copula, a formula describing how two ranks arrive together, and pick the best-fitting one.
5. Each day, read two numbers from the fitted copula, each between 0 and 1. They say how high each
   fund's rank is given the other's.
6. Open a trade when one number is above 0.95 and the other below 0.05: sell short the fund whose
   rank is high and buy the fund whose rank is low. Close it when the numbers return to the middle,
   and re-fit the copula and the pair once a month.

## The maths, with every symbol named

The cointegration method rests on a spread and its distance from average:

```text
spread = log(P_x) - beta * log(P_y)
z = (spread - mean) / std
```

- `P_x` and `P_y` are the two funds' prices.
- `log` is the natural logarithm.
- `beta` is the slope from fitting `log(P_x)` on `log(P_y)`, the size ratio between the two legs.
- `mean` and `std` are the average and standard deviation of the spread over the formation window.
- `z` is how many standard deviations the spread sits from its average. The trade opens when `z` is
  above 1 or below -1 and closes when `z` crosses zero.

The copula method rests on ranks and a copula formula. Given two return series, the ranks are:

```text
u = F_x(return of x),   v = F_y(return of y)
```

- `F_x` and `F_y` are the two funds' own distributions, estimated from the data, and `u` and `v` are
  the ranks, each between 0 and 1.

A copula describes the two ranks together, `C(u, v)` being the chance that both are at or below their
values. The simplest of the three fitted here is the Clayton copula:

```text
C(u, v) = (u^(-t) + v^(-t) - 1)^(-1/t)
tau = t / (t + 2),  so  t = 2 * tau / (1 - tau)
```

- `t` is the copula's single number, called theta, which controls how tightly the two ranks move
  together.
- `tau` is the rank correlation between the two return series.
- The Gumbel copula uses `tau = 1 - 1/t`, so `t = 1 / (1 - tau)`; the Frank copula is fitted by a
  small numerical search instead of a formula.

The two trading numbers are the copula's conditional probabilities, obtained by differentiating it:

```text
MI_xy = dC(u, v) / dv      (how high x's rank is, given y's)
MI_yx = dC(u, v) / du      (how high y's rank is, given x's)
```

- Each is between 0 and 1. A value near 1 means the fund sits high in its own distribution; near 0
  means low.
- The trade opens when one is above 0.95 and the other below 0.05; the high-ranking fund is sold
  short and the low-ranking fund is bought.

The best copula is chosen by the Akaike information criterion:

```text
AIC = -2 * L + 2 * k
```

- `L` is the total log-likelihood, the sum of the log of the copula's density at each observed rank
  pair, so a larger `L` means a better fit; `k` is the number of adjustable numbers, here 1.
- The copula with the lowest `AIC` is kept. The `+ 2 * k` term penalises a more flexible copula, so it
  must fit clearly better to win.

The account return and the cost are the same lines as any two-leg trade:

```text
R_account = w_long * R_long - w_short * R_short   and   Cost = t * c
```

- `w_long` and `w_short` are the account fractions on each side, set by `beta`; `R_long` and
  `R_short` are the two funds' returns, with the short leg subtracted.
- `t` is the value traded, counting both sides, divided by the account value; `c` is the cost of one
  trade as a fraction of the amount traded, where five basis points is 0.0005, and one basis point is
  one hundredth of one percent.

## A worked example

First the copula signal. Suppose the fitted copula is Clayton with `t = 1`, today the first fund's
rank is `u = 0.90` and the second's is `v = 0.20`. Then:

```text
C(0.90, 0.20) = (1.11111 + 5.00000 - 1)^(-1) = 1 / 5.11111 = 0.19565
MI_xy = dC/dv = 0.20^(-2) * (5.11111)^(-2) = 25 * 0.038280 = 0.95701
MI_yx = dC/du = 0.90^(-2) * (5.11111)^(-2) = 1.23457 * 0.038280 = 0.04726
```

`MI_xy` is above 0.95 and `MI_yx` is below 0.05, so the signal fires: the first fund ranks high, the
second low, so the rule sells short the first and buys the second.

Now the cointegration signal, on six days of prices with `beta = 1`. The spread is the log of the
first price minus the log of the second, and the first five days form the window:

| Day | Price X | Price Y | log(X) - log(Y) | Distance from average |
| --- | ------- | ------- | --------------- | --------------------- |
| 1   | 100.0   | 50.0    | 0.69315         | -0.00872              |
| 2   | 101.0   | 49.5    | 0.71312         | +0.01126              |
| 3   | 100.5   | 50.2    | 0.69402         | -0.00784              |
| 4   | 102.0   | 50.0    | 0.71303         | +0.01117              |
| 5   | 101.5   | 50.6    | 0.69599         | -0.00587              |
| 6   | 103.0   | 50.0    | 0.72271         | +0.02084              |

The average of the first five spreads is 0.70186 and their standard deviation is 0.00920, so on day 6:

```text
z = (0.72271 - 0.70186) / 0.00920 = 2.27
```

The spread is more than two standard deviations above its average, so the first fund is expensive
relative to the second: sell short X and buy Y, with weights 0.5 each because `beta = 1`. Suppose the
spread returns to its average when X is 101.0 and Y is 50.5. Then X fell 1.94 percent and Y rose 1.00
percent:

```text
Return before costs = 0.5 * 1.94 + 0.5 * 1.00 = +1.47 percent
t = 2.0  (the whole position is opened once and closed once)
Cost = 2.0 * 0.0005 = 0.001, that is 0.10 percent
Net return for the trade = 1.47 - 0.10 = +1.37 percent
```

## What the research actually found

| Source                                                                                       | What it measured                                                | Result                                                                                                                                                                                                                                                                                                               |
| -------------------------------------------------------------------------------------------- | --------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| QuantConnect tutorial, its own comparison                                                    | Two fund pairs, daily data, about five to nine years            | Copula: 498 trades, 7.057 percent total profit, reward-for-risk figure 0.098, worst fall 24.0 percent. Cointegration: 126 trades, 4.506 percent, 0.179, worst fall 3.9 percent. The page calls the copula method superior, yet its own numbers show cointegration with better reward for risk and a far smaller fall |
| Rad, Low and Faff (2016), and da Silva, Ziegelmann and Caldeira, both via Quantpedia entries | American shares, 1962 to 2014, and S&P 500 shares, 1990 to 2015 | The two studies disagree. Rad, Low and Faff find the copula method worst after costs, at 5 basis points a month against 36 for distance and 33 for cointegration. da Silva and co-authors find a mixed copula beats the distance method, with reward-for-risk figures up to 0.88                                     |
| Gatev, Goetzmann and Rouwenhorst, via the Quantpedia pairs entry                             | American shares, 1962 to 2002, forty instruments                | The classical pairs result: about 11.16 percent a year after estimated costs, volatility 5.85 percent, worst fall 17 percent, reward-for-risk figure 1.22, described by Quantpedia as strong                                                                                                                         |
| Fil, Gold Standard Pairs Trading Rules: Are They Valid? (`2010.01157v1`)                     | American shares, 1990 to 2020, including the pandemic           | Naive pairs trading failed to beat the market, 0.11 percent a month against 0.47; it earned about 2 percent a month in falling markets but needed to know in advance that they were falling. Cointegration's best case was 1.03 percent a month against the distance method's 0.64                                   |
| Tadi and Witzany, copula trading of cointegrated pairs (`2305.06961v2`)                      | Twenty cryptocurrencies, hourly, early 2021 to early 2023       | A copula signal on cointegrated pairs returned about 37 percent a year at a reward-for-risk figure near 0.97, beating simply holding the coins, with costs equal to 11.7 percent of gross profit; a single market and period                                                                                         |
| Clegg, via the Quantpedia pairs entry                                                        | Over 860,000 American share pairs, 2002 to 2012                 | Cointegration is not a persistent property: a pair found cointegrated in one period is not reliably cointegrated in the next, which undercuts the premise of the method                                                                                                                                              |

Read together, the picture is genuinely split. The library's page and one group of studies favour
copulas, arguing that cointegration assumes a statistical shape that prices often do not have, while a
copula captures unusual joint moves a straight line misses. Another group, including a large and
carefully costed study, finds the copula method far less profitable than the plain cointegration or
distance method. The classical pairs result was also strong on old data, and the modern replication
shows the profits largely gone except during market falls. The grade is Disputed, because credible
sources reach opposite conclusions about which method is better and nothing here settles it.

## How this project relates to it

This repository has its own tutorial on the plain version of the idea,
[Dispersion and relative value](../../project/dispersion-and-relative-value/README.md), which trades
the gap between two baskets and leans on the same cointegration logic. Its design note,
[Relative value screening](../../../docs/design/relative_value_screening.md), is the repository's
written rule for what such a screen must declare before anyone trusts it: it must estimate the ratio
between the two sides, test whether the relationship is real, and count how many pairs were tried
before the winner was chosen. That counting is the part amateur studies skip, and it is why the
disagreement above is hard to settle.

For the statistics, the brief
[Predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md)
collects the strongest recent pairs result, a model that lets the spread switch between regimes and
beats a fixed band on crude oil futures (`2309.00875v3`), and
[Risk measures, tails and drawdowns](../../../strategies/books2/22_risk_measures_and_drawdowns.md)
explains why the copula family you choose changes the risk number you get, since a copula fitted to
the ordinary days also fixes the behaviour of the extreme ones.

## Where it goes wrong

- Cointegration does not last. A pair that moved together for a year can stop, and the study of over
  860,000 pairs found the property does not carry over. It can also be spurious: choosing from a large
  list and reporting only the best pair is the classic way to find a relationship that is not there.
- Costs and borrowing. Every round trip pays the gap between buying and selling prices on both legs,
  and the short leg pays a borrow fee. The classical result survives conservative costs, but later
  work finds the margin much thinner.
- The measurement is chosen after the fact. The formation window, the threshold, the pair list and the
  copula family are all choices, and trying several and reporting the best is how a lucky
  configuration is mistaken for a method. The copula answer in particular depends on which family is
  fitted and which threshold is used.

## Try it yourself

You need a spreadsheet and daily prices for two funds that hold similar things, for about two years.

1. Build a sheet with a date column, one column of prices per fund, and two columns holding the
   natural logarithm of each price.
2. For the first year, fit a line predicting one log price from the other, and write down the slope,
   which you will call beta.
3. Add a spread column: the first log price minus beta times the second, then columns for the spread's
   running average, its standard deviation, and the distance from average divided by the standard
   deviation. That last column is `z`.
4. Mark every day in the second year where `z` goes above 1 or below -1, and the direction of the
   trade each would open.
5. On paper, hold each trade until `z` crosses zero again, and subtract about five basis points on
   each side of each leg.

What to notice: how long a trade sometimes has to wait for `z` to cross zero, and how often the pair
never comes back at all. Those stalled trades are the ones that turn a tidy relationship into a loss,
and they are the reason the second method exists.

## Where this came from

- [QuantConnect strategy library: pairs trading copula vs cointegration](https://www.quantconnect.com/tutorials/strategy-library/pairs-trading-copula-vs-cointegration),
  the rules, the copula formulas, the pair choices and the page's own comparison table.
- Stander, Marais and Botha, [Trading strategies with copulas](https://jefjournal.org.za/index.php/jef/article/view/278),
  Journal of Economic and Financial Sciences (2013), the copula rules the page follows.
- Rad, Low and Faff (2016), [The profitability of pairs trading strategies](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2614233),
  the comparison that finds the copula method worst, quoted through the Quantpedia pairs entry.
- [Quantpedia: pairs trading with stocks](https://quantpedia.com/strategies/pairs-trading-with-stocks),
  the classical result and the source-paper list.
- `2010.01157v1`, Gold Standard Pairs Trading Rules: Are They Valid?, the 1990 to 2020 replication.
- `2305.06961v2`, Copula-Based Trading of Cointegrated Cryptocurrency Pairs, with the performance table.
- `2309.00875v3`, on regime-switching cointegration in crude oil futures, cited through
  [Predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md).
- [Relative value screening](../../../docs/design/relative_value_screening.md), this repository's
  design for what a relative-value screen must declare.

## Words used in this tutorial

- beta, in this page: the fitted ratio between the two legs of the trade, telling you how much of one
  to trade for each unit of the other.
- cointegration: a stable long-run relationship between two wandering prices, so that a particular
  combination of them drifts around a fixed average instead of wandering off.
- copula: a formula describing how two things move together when each is measured only by its rank.
- hedge ratio: the size of one leg of a trade relative to the other, here given by beta.
- rank correlation: a correlation computed from the ordering of the values rather than their size,
  so a few huge moves cannot dominate it.
- spread: the quantity made by combining the two prices, whose distance from its average drives the
  cointegration trade.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
