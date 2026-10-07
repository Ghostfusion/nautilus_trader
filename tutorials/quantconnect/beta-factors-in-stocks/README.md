# Betting against beta: buying the steadiest shares and shorting the jumpiest

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                         |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of American companies, ten at a time, five held and five sold short                                                                                                                                                                                                                                    |
| How often it trades       | Once a calendar month, when the portfolio is rebuilt                                                                                                                                                                                                                                                          |
| What you need             | A spreadsheet, a year of daily prices for many shares, and a daily value for a broad market index                                                                                                                                                                                                             |
| Where the rules come from | [QuantConnect strategy library, beta factors in stocks](https://www.quantconnect.com/tutorials/strategy-library/beta-factors-in-stocks) and the [Quantpedia entry](https://quantpedia.com/strategies/betting-against-beta-factor-in-stocks) it cites                                                          |
| The underlying research   | Andrea Frazzini and Lasse Heje Pedersen, [Betting Against Beta](https://pages.stern.nyu.edu/~lpederse/papers/BettingAgainstBeta.pdf) (2014)                                                                                                                                                                   |
| How well it held up       | Disputed: the effect is documented across many markets and decades, but credible studies disagree on whether it is an inefficiency, a payment for hidden risks such as skewness or lottery demand, or an artefact of how the portfolio is built, and Quantpedia's own out-of-sample test is slightly negative |
| Also appears in           | Nothing else in this collection describes the low-beta factor; its nearest relatives are the covariance and leverage discussions in the portfolio briefs                                                                                                                                                      |

## The idea in one paragraph

Every share moves up and down partly with the whole market and partly on its own. The strength of
that shared movement is a number called beta: a beta of one means the share moves about as much as
the market, a beta of two means about twice as much, and a beta of a half means about half as much.
Some investors cannot borrow money to buy more shares, so the only way they can chase a bigger
return is to buy the shares that jump around the most. This strategy bets that those jumpy shares
have been overpaid for. It buys the shares with the lowest beta, sells short the ones with the highest
beta, and gives the most money to the very steadiest and the very jumpiest among them.

## Why anyone believed it

The argument is about limits on borrowing. If you could borrow freely, you would build one good basket
of shares and then choose how much of it to hold: more basket by borrowing, less by holding some cash.
Many investors cannot do that. A pension fund, a mutual fund or an individual may be forbidden, or
unwilling, to borrow. The only lever left is to buy shares that move more, because a share with a high
beta gives a bigger swing for the same money.

The counterparty, then, is the leverage-constrained investor who buys high-beta shares precisely
because they cannot borrow. If enough such buyers exist, they push the price of high-beta shares up
today, which lowers the return from owning them tomorrow. Investors who can borrow, the arbitrageurs,
take the other side: they buy the low-beta shares and borrow to make the position large, and they sell
the high-beta shares short. The extra return they collect is the reward for providing the leverage
that everyone else wants but cannot take for themselves.

## An everyday comparison

Imagine a shop with a rule that you may fill only one small trolley, never two. A shopper who wants to
spend more cannot buy more goods, so instead they fill the same small trolley with the most expensive
items on the shelf. Those expensive items get bought up regardless of whether they are good value, and
their prices creep higher relative to the ordinary goods. A shopper who is allowed to bring a second
trolley simply loads two trolleys of the ordinary goods and does better. Here the small trolley is the
investor's account, the expensive items are the high-beta shares, and the second trolley is the
ability to borrow.

## The rules, step by step

1. Start with all shares listed on the two main American exchanges, Nasdaq and the New York Stock
   Exchange. Drop any share whose price is below 5 dollars, because these barely trade and the quoted
   prices are unreliable. The library also uses the Wilshire 5000, a published index of almost every
   American share, as the stand-in for "the market" itself.
2. For each remaining share, collect its daily price over the past year and the daily value of the
   market index over the same year.
3. Turn both price series into daily returns, which are one day's price divided by the previous day's,
   minus one.
4. For each share, compute its beta against the index over that year, using the formula in the next
   section.
5. Rank all the shares from the lowest beta to the highest.
6. Buy the five shares with the lowest beta. Sell short the five with the highest beta. Selling short
   means borrowing the share, selling it now, and buying it back later, so that a fall is a gain.
7. Weight by rank. Inside the five bought shares, the lowest beta gets the largest share of the long
   money. Inside the five shorted shares, the highest beta gets the largest share of the short money.
   Give each side about half the account.
8. Hold for one calendar month. At the start of the next month, recompute step 4 for every share and
   repeat from step 5. Do not look at the prices in between.

One honest warning before the arithmetic: on the QuantConnect page the multiplier that scales the long
side is applied to six rank numbers, not five, so the five bought shares end up holding about
two thirds of the account while the five shorted shares hold one half. A portfolio meant to be
balanced is therefore not balanced as written. This tutorial uses clean weights that give each side
one half, and notes the difference again below.

## The maths, with every symbol named

Beta is the covariance of a share's returns with the market's, divided by the variance of the market's
returns:

```text
beta = cov(R_i, R_m) / var(R_m)
```

- `beta` is the share's sensitivity to the market, a plain number with no units.
- `R_i` is the share's daily return series over the year, a list of decimals.
- `R_m` is the market index's daily return series over the same days.
- `cov(R_i, R_m)` is the covariance: the average of the product of how far each share return sits from
  its own average and how far the matching market return sits from its own average. If the two tend to
  be above their averages on the same days, the covariance is positive.
- `var(R_m)` is the variance of the market returns: the average of the squared distance of each market
  return from its own average. It measures how much the market itself swings.

The same number has a second reading that is easier to picture. If you draw a line through a scatter
plot with market returns on the horizontal axis and share returns on the vertical axis, beta is the
slope of the best-fitting line. A slope of two means that, on average, when the market rose one
percent the share rose two.

The weighting by rank gives the extreme shares the most money. Number the five chosen shares on each
side from one, for the most extreme, to five. Give each a score, one for the least extreme and a
larger score for the more extreme, then divide by the total of the scores so the side adds up to one
half:

```text
score_j = 6 - j                 for the long side, j = 1 to 5
w_j = 0.5 * score_j / 15
```

- `j` is the rank position inside the long side, one for the lowest beta.
- `score_j` is the relative size of that position. For the lowest-beta share it is 5, then 4, then 3,
  then 2, then 1.
- 15 is the sum of those scores, 5 + 4 + 3 + 2 + 1.
- `w_j` is the fraction of the account placed in that share. The five weights add to one half.

The short side is the mirror image: the highest-beta share gets the largest short weight, so its
weight is the biggest negative number. The portfolio's result for the month, before costs, is each
weight multiplied by that share's return and all of them added:

```text
G = w_1 * R_1 + w_2 * R_2 + ... + w_10 * R_10
```

- `G` is the gross monthly return of the portfolio, as a decimal.
- `w_1` to `w_10` are the weights, positive on the long side and negative on the short side.
- `R_1` to `R_10` are the next-month returns of the ten shares.

Finally the cost of rebuilding the book each month:

```text
Cost = t * c
```

- `Cost` is the monthly cost, as a decimal.
- `t` is the traded notional: about 2.0 if the whole book is sold and replaced, because the half held
  long and the half held short are each closed once and opened once.
- `c` is the cost per unit traded, covering the gap between the buying and selling price plus
  commission. For large shares, 0.0005, five basis points, is a fair figure; for the smaller names the
  screen sometimes reaches, it is too low.

## A worked example

First, beta for one share over four invented days, to show the arithmetic. All returns are in percent.
The market's average return over the four days is 0.275 and the share's is 0.375.

| Day | Market return | Market minus mean | Share return | Share minus mean | Product of the two |
| --- | ------------- | ----------------- | ------------ | ---------------- | ------------------ |
| 1   | +1.0          | +0.725            | +1.5         | +1.125           | 0.8156             |
| 2   | -0.5          | -0.775            | -0.7         | -1.075           | 0.8331             |
| 3   | +0.8          | +0.525            | +1.1         | +0.725           | 0.3806             |
| 4   | -0.2          | -0.475            | -0.4         | -0.775           | 0.3681             |
| Sum |               |                   |              |                  | 2.3975             |

The average of the products is the covariance: 2.3975 divided by 4, which is 0.5994. The squared
market deviations, 0.525625, 0.600625, 0.275625 and 0.225625, add to 1.6275, and dividing by 4 gives
the market variance, 0.4069. So beta is 0.5994 divided by 0.4069, which is 1.47. This share moves
about one and a half times as much as the market, so it belongs on the short side. In real use the
window is a full year of daily returns, several hundred days rather than four, which makes the
estimate steadier but still not exact.

Now ten shares, ranked by their year-long beta, with their returns over the following month. The
weights follow the rank rule above.

| Share | Beta | Side  | Rank j | Weight  | Next-month return | Contribution |
| ----- | ---- | ----- | ------ | ------- | ----------------- | ------------ |
| A     | 0.42 | Long  | 1      | 0.1667  | +0.8%             | +0.1334%     |
| B     | 0.55 | Long  | 2      | 0.1333  | +1.6%             | +0.2133%     |
| C     | 0.68 | Long  | 3      | 0.1000  | -0.4%             | -0.0400%     |
| D     | 0.74 | Long  | 4      | 0.0667  | +0.9%             | +0.0600%     |
| E     | 0.85 | Long  | 5      | 0.0333  | +0.2%             | +0.0067%     |
| F     | 1.05 | Short | 5      | -0.0333 | -0.6%             | +0.0200%     |
| G     | 1.18 | Short | 4      | -0.0667 | +0.5%             | -0.0334%     |
| H     | 1.32 | Short | 3      | -0.1000 | -1.1%             | +0.1100%     |
| I     | 1.55 | Short | 2      | -0.1333 | +0.3%             | -0.0400%     |
| J     | 1.90 | Short | 1      | -0.1667 | -1.8%             | +0.3001%     |
| Total |      |       |        | 0.0000  |                   | +0.7301%     |

The long side contributes 0.3734 percent and the short side 0.3567 percent, for a gross month of 0.73
percent. Applying the cost:

```text
Cost = 2.0 * 0.0005 = 0.0010, that is 0.10 percent
Net  = 0.73 - 0.10 = 0.63 percent for the month
```

Twelve months at that rate is about 7.6 percent a year before compounding. The gross 0.73 percent
happens to be the same as the published monthly figure below, which is a coincidence of the invented
numbers and not a sign that the strategy works.

## What the research actually found

| Source                                                                                                                       | What it measured                                                                              | Result                                                                                                                                                                                                                                                                    |
| ---------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Frazzini and Pedersen, [Betting Against Beta](https://pages.stern.nyu.edu/~lpederse/papers/BettingAgainstBeta.pdf)           | 55,600 shares in 20 countries, United States data from January 1926 to March 2012             | The United States factor earned a reward-to-risk of 0.78 and a four-factor monthly return of 0.55 percent with a t-statistic of 5.59, and the raw monthly figure was 0.73 percent with a t-statistic of 7.39 (p.21)                                                       |
| The same study, across markets and asset classes                                                                             | 19 developed share markets, government bonds, corporate bonds, currency and commodity futures | The factor earned a positive risk-adjusted return in 18 of the 19 share markets (p.22), and also in government bonds, where its reward-to-risk was 0.81 (p.4)                                                                                                             |
| [Quantpedia, betting against beta factor in stocks](https://quantpedia.com/strategies/betting-against-beta-factor-in-stocks) | Bottom and top beta portfolios, United States, 1926 to 2009                                   | 8.86 percent a year, volatility 11.5 percent, worst fall 61.52 percent, reward-to-risk 0.42, before costs, from the same sample                                                                                                                                           |
| Quantpedia's confidence label                                                                                                | The same record                                                                               | "Moderate", because its own out-of-sample test is slightly negative and the edge looks like it is fading                                                                                                                                                                  |
| Novy-Marx and Velikov                                                                                                        | The factor's construction                                                                     | The strong performance comes from a non-standard weighting that effectively equal-weights shares and commits about 1.05 dollars to the smallest one percent of companies for every dollar invested, and is explained by tilts to profitability and investment after costs |
| Bali, Brown, Murray and Tang                                                                                                 | Competing explanations for the effect                                                         | The anomaly is caused by demand for lottery-like shares, and forcing the portfolios to be neutral to that demand removes the abnormal return                                                                                                                              |
| Schneider, Wagner and Zechner                                                                                                | Low-risk anomalies in general                                                                 | The low-beta and low-volatility effects are driven by return skewness, so their returns may be payment for a risk rather than a free gain                                                                                                                                 |

The disagreement here is not about whether the pattern appears; it appears in many markets over a
century, and that is well replicated. The disagreement is about why. Frazzini and Pedersen say it is
a reward for supplying leverage to constrained investors. Novy-Marx and Velikov say the reward comes
from how the portfolio is built and from hidden factor exposures. Bali and his co-authors, and
Schneider and his, say it is payment for risks such as a tendency to crash or a lottery-like payoff.
Those positions cannot all be exactly right at once, and the literature has not settled it. The
library's version is also not the paper's factor: it takes the bottom five and top five of a filtered
list rather than rescaling each side to a beta of one, so its measurement is a simplified relative.

## How this project relates to it

This repository measures the inputs this strategy depends on. The brief
[Portfolio construction and allocation](../../../strategies/books2/10_portfolio_and_allocation.md)
reports that a covariance matrix that is merely poorly conditioned pushes the mean-variance weight
vector away from the direction of the expected returns, with the size of that rotation bounded by the
condition number alone, citing `2107.06194v5` (p.4), and reports that a factor model's measured
performance depends on how the test portfolios are built, with a three-factor model winning in six of
eight daily cells while five- and six-factor models win in most buy-and-hold cells, citing
`2606.19550v1` (p.20, p.23). Beta is exactly such a covariance estimate, and the second finding is a
warning that a beta-sorted portfolio's measured edge can be a property of the construction.

The second brief is
[Portfolio construction under frictions](../../../strategies/books/15_portfolio_construction_under_frictions.md),
which is about the same borrowing limits the strategy is built on. It reports that liquidity-blind
backtests are systematically optimistic, that optimal exposure to shares falls toward cash as account
size grows, and that a quintile of funds that manages liquidity actively beat the least active
quintile by a spread worth about 4.2 percent a year as a market-adjusted alpha and 6.6 percent raw,
citing `2510.02741v1` (p.19-20). The leverage constraint that gives the low-beta effect its story is
the same constraint in both briefs.

## Where it goes wrong

- Beta is estimated, not known. A single year of daily data gives a noisy number, and the original
  paper shrinks each estimate about 60 percent of the way toward one precisely because of that noise.
  Two people ranking the same shares with two windows will get different portfolios.
- The effect may be a payment for hidden risks. Skewness and lottery demand both explain a large part
  of the return, and a strategy that collects a premium for crash risk falls hardest when the crash
  arrives.
- Shorting is not free or always possible. The shares on the short side are the jumpy ones, whose
  borrow fee rises exactly when the market is stressed, which is when the position is most valuable.
- Crowding and decay. Quantpedia's own out-of-sample test is slightly negative, and once the factor is
  known and cheap to trade through funds, the same logic that made it work erodes it.
- The construction decides the answer. Which window, which market index, how many shares, and how
  they are weighted all change the result, and the library's version changes all four at once.
- The two sides are unequal as written. The library's long side is about two thirds of the account and
  its short side one half, so a portfolio described as balanced is really a net bet on the market.
- The screen reaches small shares. The smallest names carry the strongest effect and the highest
  trading costs, and a backtest that assumes they can be traded at the quoted price is optimistic.

## Try it yourself

You need a spreadsheet, one year of daily closing prices for about thirty large shares, and a daily
value for a broad market index. Any public finance data page will give you both.

1. Build a sheet with one column per share and one row per day, holding the share price. Add a column
   for the index.
2. Add a second block of columns computing each day's return: today's price divided by yesterday's,
   minus one. Do the same for the index.
3. For each share, compute its average return and the index's average return over the year.
4. Add, for each share, a column of the product of the two deviations from those averages, and take
   its average. That is the covariance.
5. Take the average of the squared index deviations. That is the market variance.
6. Divide each covariance by the market variance. Those are the betas.
7. Sort the shares by beta and write down the lowest five and the highest five.

What to notice: if you do steps 3 to 6 a second time using only the first six months and then only the
second six, the beta of the same share will usually change, sometimes by a lot. The lower-beta shares
also tend to fall less than the higher-beta shares on the market's worst days, which is the risk story
behind the effect and the reason the strategy is not free money.

## Where this came from

- [QuantConnect strategy library: beta factors in stocks](https://www.quantconnect.com/tutorials/strategy-library/beta-factors-in-stocks),
  the implemented rules: the Nasdaq and NYSE universe, the price filter, the one-year beta, the bottom
  and top five, and the rank weighting.
- [Quantpedia: betting against beta factor in stocks](https://quantpedia.com/strategies/betting-against-beta-factor-in-stocks),
  the performance figures, the instrument count, the confidence label and the reference list.
- Andrea Frazzini and Lasse Heje Pedersen, [Betting Against Beta](https://pages.stern.nyu.edu/~lpederse/papers/BettingAgainstBeta.pdf),
  the model, the beta formula and the out-of-sample evidence.
- [Portfolio construction and allocation](../../../strategies/books2/10_portfolio_and_allocation.md),
  this repository's brief on covariance estimation and on how much a factor result depends on how the
  test portfolios are built, citing `2107.06194v5` and `2606.19550v1`.
- [Portfolio construction under frictions](../../../strategies/books/15_portfolio_construction_under_frictions.md),
  this repository's brief on leverage, sizing and the cost of acting on a factor, citing
  `2510.02741v1`.

## Words used in this tutorial

- beta: how strongly one price moves when the market moves; one means about the same as the market.
- covariance: a measure of how two sets of numbers move together, positive when they rise together.
- factor: a shared characteristic used to sort assets into groups, such as beta or size.
- leverage: using borrowed money so that a given price move produces a larger gain or loss.
- long: owning something, so that a rise in its price is a gain.
- market index: a published list whose combined price stands in for the whole market.
- rank weighting: giving larger amounts to the extremes of a sorted list and smaller amounts to the middle.
- short selling: borrowing something you do not own and selling it now, so that a fall is a gain.
- variance: the average of the squared distance of a set of numbers from their own average.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
