# Betting against beta in country funds: the same trick on national stock markets

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                                   |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Funds that each track one country's stock market, in both directions: the calm countries bought with borrowed money, the jumpy countries sold short                                                                                                                                                                     |
| How often it trades       | Once a calendar month, when both baskets are rebuilt                                                                                                                                                                                                                                                                    |
| What you need             | A spreadsheet and a year of daily prices for a list of country funds and for a broad market index                                                                                                                                                                                                                       |
| Where the rules come from | [The list's implementation file](https://github.com/paperswithbacktest/awesome-systematic-trading/blob/main/static/strategies/betting-against-beta-factor-in-country-equity-indexes.py) and the [Quantpedia entry](https://quantpedia.com/strategies/betting-against-beta-factor-in-country-equity-indexes/) it repeats |
| The underlying research   | Frazzini and Pedersen, [Betting Against Beta](http://pages.stern.nyu.edu/~lpederse/papers/BettingAgainstBeta.pdf)                                                                                                                                                                                                       |
| How well it held up       | Mixed: the paper reports the effect across twenty international equity markets and Quantpedia grades it Strong, but the list publishes no separate result for the country-fund version, its single equity run returned a Sharpe ratio of 0.57, and country funds add thinner trading, currency swings and wider costs   |
| Also appears in           | [Beta factor in country equity indexes](../../quantconnect/beta-factor-in-country-equity-indexes/README.md) and [Beta factors in stocks](../../quantconnect/beta-factors-in-stocks/README.md) in this collection                                                                                                        |

## The idea in one paragraph

National stock markets move by different amounts when the world market moves. Japan and Switzerland
tend to move less than the average; Brazil and Turkey tend to move more. The argument behind this
strategy is the same one that applies to single shares: investors who cannot borrow money to make
their savings swing harder buy the jumpiest markets instead, which makes those markets expensive
and their later returns disappointing. So the strategy buys a basket of the calmest national
markets, borrows enough that the basket swings as much as the world market, sells short a basket of
the jumpiest markets with the opposite adjustment, and rebuilds once a month.

## Why anyone believed it

The reasoning is about who may borrow. A fund manager judged against a benchmark, or an ordinary
saver, often cannot take on debt to increase exposure. The substitute is to own the things that move
most, and country markets are one of the easiest places to do it, because a fund that tracks Brazil
or Turkey is as easy to buy as one that tracks Switzerland. That demand lifts the jumpy markets and
holds down the return they deliver for their risk, while the calm markets are left cheap.

The counterparty is the constrained investor who pays up for a jumpy market, and the winner is
whoever can borrow. The international version is worth testing separately because country markets
are large and liquid, so a strategy that survives here is harder to dismiss as a quirk of small or
illiquid shares. The honest catch is that a country fund quoted in dollars carries the local market
and the exchange rate at the same time, so its beta mixes two risks.

## An everyday comparison

Think of a ferry that carries two kinds of passenger: those who can pay extra for the fast boat and
those who cannot. The ones who cannot must take the slow boat, which is more likely to be delayed,
so they crowd onto it and it becomes overbooked and uncomfortable. The fast boat, with fewer
passengers willing to pay, runs half empty and is good value. The calm-country basket is the fast
boat and the jumpy-country basket is the slow one that everyone piles into.

## The rules, step by step

1. Assemble a list of country funds, each tracking one national stock market. The code uses 23
   funds managed by one provider, covering markets such as Australia, Brazil, Canada, France,
   Germany, Japan, Switzerland and the United Kingdom.
2. Choose a market index to represent the world market. The code uses the SPY fund, which tracks the
   largest American index, as the reference.
3. At the end of each month, estimate each fund's beta against that reference over the past year of
   daily prices, using the method given below.
4. Find the median beta across the funds: the middle value when the betas are sorted, so half sit
   above it and half below.
5. Put every fund below the median into the calm basket and every fund above it into the jumpy
   basket.
6. Weight each fund by how far its beta sits from the median. A country whose beta is far below the
   median gets a large weight in the calm basket; a country far above the median gets a large short
   in the jumpy basket; a country just beside the median gets almost nothing.
7. Rescale each basket so its beta is one, using the same rule as the share version. The calm basket
   is multiplied up by one divided by its average beta, capped at five; the jumpy basket is scaled
   down by one divided by its average beta.
8. Hold for one month, then rebuild. Pay trading costs, interest on the borrowed part of the calm
   basket, and a borrow fee on the shorted funds.

## The maths, with every symbol named

The beta of one fund, and the median that splits the list, are the two inputs:

```text
beta_i = Cov(r_i, r_m) / Var(r_m)
m = median of all the beta_i
```

- `beta_i` is the beta of country fund `i`, a plain number.
- `r_i` and `r_m` are the daily returns of the fund and of the market index over the past year.
- `Cov` is the covariance, how much the two return series move together; `Var` is the variance of
  the index returns, how much the index itself moves.
- `m` is the median beta, the value with half the funds above and half below.

Each fund's distance from the median becomes its weight:

```text
d_i = | m - beta_i |
w_i = (d_i / sum of d over its own basket) * L_basket
```

- `d_i` is the distance of fund `i` from the median beta, always positive.
- The fraction `d_i / sum of d` splits each basket in proportion to distance, so the weights inside
  a basket add to one.
- `L_basket` is the leverage of that basket, `1 / (average beta of the basket)`, capped at five.
- `w_i` is positive in the calm basket and negative in the jumpy basket, and it is the fraction of
  your money placed in fund `i`.

The monthly return is the sum of each weight times its fund's return:

```text
R = sum over all funds of ( w_i * r_i )
```

- `r_i` is the fund's return over the month.
- Because the negative weights belong to the shorted markets, a fall in a jumpy market adds to `R`.
- The calm basket's weights add up to `L_calm` and the jumpy basket's to `-L_jumpy`, so the net
  dollar position is positive even though the net beta is zero.

The monthly cost is:

```text
Cost_month = (L_calm + L_jumpy) * turn * c + (L_calm - 1) * i + L_jumpy * s
```

- `turn` is the fraction of each basket replaced at the rebuild, about 0.2 if a fifth of the funds
  change.
- `c` is the cost of one trade as a fraction of its value. Country funds are thinner than the biggest
  American shares, so 0.001, ten basis points, is a floor and 0.002 is realistic.
- `i` is the interest per month on the borrowed part of the calm basket, and `s` is the borrow fee
  per month on the shorted funds.

## A worked example

Eight country funds, with invented but plausible betas against the market index.

| Country        | Beta | Side  | Distance from 0.95 | Weight  |
| -------------- | ---- | ----- | ------------------ | ------- |
| Switzerland    | 0.55 | Calm  | 0.40               | +0.5541 |
| Japan          | 0.65 | Calm  | 0.30               | +0.4156 |
| United Kingdom | 0.75 | Calm  | 0.20               | +0.2771 |
| Germany        | 0.80 | Calm  | 0.15               | +0.2078 |
| Mexico         | 1.10 | Jumpy | 0.15               | -0.0824 |
| Canada         | 1.15 | Jumpy | 0.20               | -0.1099 |
| Brazil         | 1.40 | Jumpy | 0.45               | -0.2473 |
| Turkey         | 1.55 | Jumpy | 0.60               | -0.3297 |

Sorting the eight betas and taking the two middle values gives a median of `(0.80 + 1.10) / 2 =
0.95`. The calm basket's average beta is `(0.55 + 0.65 + 0.75 + 0.80) / 4 = 0.6875`, so its leverage
is `1 / 0.6875 = 1.4545`. The jumpy basket's average beta is `(1.10 + 1.15 + 1.40 + 1.55) / 4 =
1.30`, so its leverage is `1 / 1.30 = 0.7692`. Distances inside the calm basket add to `0.40 + 0.30
+ 0.20 + 0.15 = 1.05`, so, for example, Switzerland's weight is `(0.40 / 1.05) * 1.4545 = 0.5541`.

| Month | Calm basket | Jumpy basket | Gross    | Cost    | Net      | Cumulative |
| ----- | ----------- | ------------ | -------- | ------- | -------- | ---------- |
| 1     | +0.6511%    | +0.1401%     | +0.7912% | 0.1145% | +0.6768% | +0.6768%   |
| 2     | +0.2147%    | +0.3682%     | +0.5829% | 0.1145% | +0.4684% | +1.1485%   |
| 3     | +0.6303%    | +0.2940%     | +0.9243% | 0.1145% | +0.8099% | +1.9677%   |
| 4     | +0.0416%    | -0.2335%     | -0.1920% | 0.1145% | -0.3064% | +1.6553%   |
| 5     | +0.8173%    | +0.4039%     | +1.2212% | 0.1145% | +1.1068% | +2.7804%   |

The two basket columns are the weighted sums of the funds' returns with the weights above, so each
is a number in percent. The gross column adds them, which is the same as subtracting the jumpy
basket's raw return because its weights are negative. The cost of 0.1145 percent a month is the sum
of three pieces:

```text
Trading cost = (1.4545 + 0.7692) * 0.2 * 0.001 = 0.000445, that is 0.0445 percent
Interest     = (1.4545 - 1) * 0.01 / 12 = 0.000379, that is 0.0379 percent
Borrow fee   = 0.7692 * 0.005 / 12 = 0.000321, that is 0.0321 percent
```

The example ends about plus 2.8 percent over five months, and the small fourth month shows how
easily the bet goes quiet when the calm markets stop leading: the calm basket barely moved while
the jumpy one rose, so the short leg lost. As with the share version, the raw gross effect is small
and the costs, especially the borrow fee and the leverage, take a real bite.

## What the research actually found

| Source and what it measured                                                         | Result                                                                                                                                                                         |
| ----------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Frazzini and Pedersen, international equity markets, 1980 to 2009                   | The betting-against-beta factor earns a significant positive risk-adjusted return in the United States and in 20 international equity markets, as well as in bonds and futures |
| Berrada, Messikh, Oderda and Pictet, country indices                                | A theoretical model in which a long-calm, short-jumpy bet on country indices can beat the market with probability one under mild diversity conditions                          |
| Quantpedia's summary of the country version                                         | 6.8 percent a year at 13.1 percent volatility, worst fall 46.6 percent, a Sharpe ratio of 0.51 over 1980 to 2009, graded Strong, using thirteen country funds                  |
| The list's own measurement of its equity betting-against-beta rule, on its own data | A Sharpe ratio of 0.57, annual return 10.95 percent, volatility 22.87 percent, worst fall 60.0 percent, 1990 to 2026 (the vendor's own measurement)                            |

Two gaps should be stated plainly. First, the list publishes no separate backtest of the country
version; the only run it reports is the equity one, so the country figures here come from the papers
and from Quantpedia. Second, the same construction critique that applies to the share version
applies here: a follow-up argued that the measured performance of the factor comes largely from how
the baskets are weighted rather than from beta itself, and a separate line of work attributes the
effect to demand for lottery-like assets. The list's aggregate record, a median replication Sharpe
ratio of 0.37 across thousands of papers, is the right scale for judging a single positive number.

## How this project relates to it

The finished tutorial [Beta factor in country equity indexes](../../quantconnect/beta-factor-in-country-equity-indexes/README.md)
in this collection applies the ordinary beta sort to national funds, without the leverage-and-short
construction used here, and its sibling [Beta factors in stocks](../../quantconnect/beta-factors-in-stocks/README.md)
does the same for single shares. Reading the three together shows how much of the result depends on
the construction rather than on the underlying ranking.

The measurement side of the question is covered in
[Portfolio construction and allocation](../../../strategies/books2/10_portfolio_and_allocation.md),
which reports how strongly a factor's measured return depends on its weighting scheme, and in
[Banking, credit and funding liquidity](../../../strategies/books2/02_banking_credit_and_funding.md),
which explains why leverage rules tighten in a downturn, the moment this strategy is most exposed.

## Where it goes wrong

- Currency is bundled with the market. A country fund quoted in dollars moves with both the local
  market and the exchange rate, so a "calm" market may be calm only because two risks offset.
- The median split is unstable with few countries. With thirteen to twenty-three funds, one or two
  changing betas can flip a country from calm to jumpy and force a large trade.
- The funds are thinner than the largest shares. Country funds outside the biggest markets have
  wider gaps between buying and selling prices, so the realistic cost is nearer 0.002 than 0.001.
- Borrowing and shorting are expensive for the markets that matter. The jumpiest markets are often
  the hardest to borrow, and the leverage cap of five shows how much borrowing the calm side needs.
- The critique and the alternative explanation travel with the factor. If its return comes from
  weighting choices, or from demand for speculative assets rather than from a funding story, then
  the country version inherits the same problem.
- A run chosen after the fact flatters itself. Picking the countries, the window and the cap by
  looking at the result is the most common way a strategy like this looks better than it is.

## Try it yourself

You need a spreadsheet and daily closing prices for a handful of country funds and one index fund.

1. Pick eight country funds. Paste the last 252 daily closing prices for each in a row, and do the
   same for the index fund.
2. Add a row for each that computes daily returns: today's price divided by yesterday's, minus one.
3. For each fund compute the covariance of its returns with the index returns, and divide by the
   variance of the index returns. Those are the eight betas.
4. Sort them and write down the median; with eight values it is the average of the fourth and fifth.
5. Put the funds below the median in a calm column and those above in a jumpy column, and compute
   each fund's distance from the median.
6. Write down the weight of each fund as its distance divided by the total distance in its column,
   then multiply each column by one divided by its average beta.
7. Take next month's returns, multiply each by its weight, and add them up. Subtract 0.11 percent
   for costs.

What to notice: the weights do not obey intuition, because a country just beside the median
contributes nearly nothing while one far away dominates, and the calm column's weights add to more
than one. The bet is therefore concentrated in a few markets, which is a risk the average hides.

## Where this came from

- [The list's implementation of this strategy](https://github.com/paperswithbacktest/awesome-systematic-trading/blob/main/static/strategies/betting-against-beta-factor-in-country-equity-indexes.py),
  which states the country funds, the median split, the distance weighting and the leverage cap.
- [Quantpedia: betting against beta factor in international equities](https://quantpedia.com/strategies/betting-against-beta-factor-in-country-equity-indexes/),
  the performance figures, the confidence grade and the source-paper link.
- Frazzini and Pedersen, [Betting Against Beta](http://pages.stern.nyu.edu/~lpederse/papers/BettingAgainstBeta.pdf),
  the paper behind both the share and the country versions.
- Berrada, Messikh, Oderda and Pictet, [Beta-Arbitrage Strategies: When Do They Work, and Why?](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1976285),
  the theoretical case for country indices.
- The list's own measurement page at
  [paperswithbacktest.com/strategies/betting-against-beta](https://paperswithbacktest.com/strategies/betting-against-beta).
- [Portfolio construction and allocation](../../../strategies/books2/10_portfolio_and_allocation.md),
  this repository's brief on how weighting decides a factor's measured return.

## Words used in this tutorial

- beta: a number for how strongly one price moves when the market moves; one means it moves about as much.
- borrow fee: the rent paid for the funds you have shorted, charged for as long as the position is open.
- leverage: using borrowed money so that a given price move produces a larger gain or loss.
- median: the middle value of a sorted list, so half the values sit above it and half below.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- variance: a measure of how widely a series of numbers is spread around its average.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
