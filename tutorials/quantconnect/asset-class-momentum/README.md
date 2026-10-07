# Asset class momentum: buying the kinds of investment that have been winning

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                         |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Five funds, one for each kind of investment: American shares, foreign shares, bonds, property shares and commodities                                                                                                                                          |
| How often it trades       | About once a month, when the portfolio is rebuilt                                                                                                                                                                                                             |
| What you need             | A spreadsheet and twelve months of prices for five funds                                                                                                                                                                                                      |
| Where the rules come from | [QuantConnect strategy library, asset class momentum](https://www.quantconnect.com/tutorials/strategy-library/asset-class-momentum) and the [Quantpedia entry](https://quantpedia.com/strategies/asset-class-momentum-rotational-system) it cites             |
| The underlying research   | Mebane Faber, [Relative Strength Strategies for Investing](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1585517)                                                                                                                                       |
| How well it held up       | Mixed: a long study back to 1973 with a modest reward for the risk, but the asset list and the twelve-month window were chosen by the rule's author, and independent tests show the cross-asset momentum premium is real yet smaller and rougher under stress |
| Also appears in           | [Sector momentum](../sector-momentum/README.md) in this collection, the same rule applied to industry groups instead of asset classes                                                                                                                         |

## The idea in one paragraph

An asset class is a whole family of investments that behave alike: company shares, government
bonds, property, commodities and so on. In any year some families do much better than others,
because the economy treats them differently. This strategy takes five funds, one for each family,
measures how each has done over the past twelve months, and puts all the money into the three that
did best, in equal amounts. A month later it looks again, swaps out whatever has fallen out of the
top three, and repeats. The bet is that a family that has been strong tends to stay strong for a
while, so the money sits in whatever is currently rising.

## Why anyone believed it

Different kinds of investment respond to the business cycle at different times. When interest rates
fall, bonds rise first and property and shares follow; when inflation runs hot, commodities and
property do better than bonds. Because those cycles move slowly, the leader among the families
changes slowly too, and money that arrives late keeps pushing the leader higher. This is the same
mechanism as in the sector version of the rule, but the groups are broader.

The counterparty is whoever is slow to move between families. A pension fund with a fixed target
allocation must sell whatever has grown past its target, which means selling the winner. A nervous
investor who exits shares after a bad year and buys bonds does the opposite of the rule at exactly
the wrong moment. If enough such sellers keep appearing, the winning family keeps winning.

## An everyday comparison

Think of a household deciding where to spend its monthly savings: a bank deposit, a government bond,
a rented flat, a gold coin, or company shares. Most families do not switch between these every
month; they stick with what they know, and they notice which one has been discussed favourably in
the news. If everyone gradually tilts toward the one that has lately done well, that one receives
more money and does even better for a while. The rule here is simply to be the first in the
household to notice the tilt.

## The rules, step by step

1. Choose five funds, one for each family. Quantpedia names exactly these: SPY for American shares,
   EFA for foreign shares, BND for bonds, VNQ for property shares and GSG for commodities.
2. For each fund, compute its return over the past twelve months: today's price divided by the price
   twelve months ago, minus one. A fund that went from 100.00 to 128.00 has a return of 28 percent.
3. Rank the five funds by that return, best first.
4. Buy the top three, in equal amounts: one third of the money in each.
5. Hold for one month without watching the prices.
6. At the start of the next month, recompute step 2 for all five funds and repeat from step 3. Sell
   whatever has dropped out of the top three and buy whatever has entered.
7. This tutorial describes the long-only version, which is the one both sources state. A long and
   short version, which also sells the weakest families, is a separate idea and is not what the
   published numbers below measure.

One refinement worth knowing: many researchers skip the most recent month when measuring momentum,
using the return from twelve months ago to one month ago. The reason is that over days and weeks a
price tends to bounce back after a sharp move, which works against the signal. That convention is
usually written as 12-1 momentum. The numbers below use the plain twelve-month version unless stated.

## The maths, with every symbol named

The whole strategy is one calculation repeated five times and one sort.

The twelve-month return of one fund:

```text
M = P_today / P_twelve_months_ago - 1
```

- `M` is the momentum score of the fund, written as a decimal: 0.28 means 28 percent.
- `P_today` is the fund's price today.
- `P_twelve_months_ago` is the fund's price on the same day twelve months earlier.

Rank the five funds by `M`, from largest to smallest, and keep the top three. Give each of the three
the same share of the money:

```text
w_i = 1 / 3 for each of the three chosen funds, and 0 for the other two
```

- `w_i` is the fraction of the money placed in fund `i`.
- The weights of the chosen three add up to 1, so the whole account is invested, in three equal
  slices.

The portfolio's return over the following month is then the average of the three funds' returns,
because the slices are equal:

```text
R_portfolio = (R_1 + R_2 + R_3) / 3
```

- `R_1`, `R_2` and `R_3` are the next-month returns of the three chosen funds.
- Dividing by three is the same as giving each a weight of one third and adding.

Finally, the cost of rebuilding the portfolio each month. If a fraction `t` of the money is traded,
the cost in that month is:

```text
Cost = t * c
```

- `t` is the traded fraction: 2.0 if the whole portfolio is sold and replaced at once, because
  selling the old holdings and buying the new ones counts twice, and less than 2.0 when some
  holdings are kept.
- `c` is the cost per trade as a fraction of the amount traded, covering the gap between the buying
  and selling price plus commission. A realistic figure for broad funds is 0.0005 to 0.001, that is
  five to ten basis points, where one basis point is one hundredth of one percent.

## A worked example

Five funds, ranked by their return over the past twelve months. The returns below are invented, but
they are of the size that asset-class returns actually take.

| Fund            | Price a year ago | Price today | M    | Rank |
| --------------- | ---------------- | ----------- | ---- | ---- |
| American shares | 100.00           | 128.00      | 0.28 | 1    |
| Foreign shares  | 80.00            | 92.80       | 0.16 | 2    |
| Property shares | 90.00            | 103.50      | 0.15 | 3    |
| Commodities     | 60.00            | 66.00       | 0.10 | 4    |
| Bonds           | 75.00            | 77.25       | 0.03 | 5    |

The three chosen funds are American shares, foreign shares and property shares, one third of the
money each. Now suppose the next month produces these returns:

| Fund held       | Weight | Next-month return | Contribution    |
| --------------- | ------ | ----------------- | --------------- |
| American shares | 0.3333 | +2.0 percent      | +0.6667 percent |
| Foreign shares  | 0.3333 | +1.0 percent      | +0.3333 percent |
| Property shares | 0.3333 | -1.0 percent      | -0.3333 percent |
| Total           | 1.0000 |                   | +0.6667 percent |

So the portfolio gained 0.6667 percent before costs. Suppose that last month the money had been in
American shares, foreign shares and commodities, so commodities dropped out and property shares came
in. That is one position sold and one bought, which is one third of the money moved on each side:

```text
t = 2 * (1 / 3) = 0.6667
Cost = 0.6667 * 0.0005 = 0.000333, that is 0.0333 percent
Net return for the month = 0.6667 - 0.0333 = 0.6334 percent
```

Two things are worth noticing. First, the cost line is small in a month when only one fund changes,
but a month when all three change costs three times as much. Second, the worked example says nothing
about whether the strategy works. It only shows how to apply the rules and how the arithmetic
behaves.

## What the research actually found

| Source                                                              | What it measured                                                                  | Result                                                                                                                                                           |
| ------------------------------------------------------------------- | --------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Quantpedia, summarising Faber's relative strength work              | Five asset classes, top three by twelve-month return, monthly, 1973-2009          | 14.49 percent a year, volatility 11 percent, worst fall 47.77 percent, reward-to-risk 0.78, from exhibit 10.6 of the source                                      |
| Faber, Relative Strength Strategies for Investing                   | Relative strength across global asset classes, plus sector data back to the 1920s | Beat buy and hold in roughly seventy percent of years on sector data, with returns persistent across time; the global asset-class test gave supporting results   |
| Kessler and Scherer, Macro Momentum and the Economy                 | Relative momentum between currencies, equities, real estate and commodities       | Stable and robust outperformance that survived transaction costs and stability tests, and was strongest in times of macroeconomic uncertainty                    |
| Antonacci, Optimal Momentum                                         | ETF data, 2002-2010                                                               | Global stock index funds gave the best risk-adjusted momentum results, but with very high volatility; letting bonds into the portfolio acted as a timing overlay |
| Geczy and Samonov, 215 Years of Global Multi-Asset Momentum         | Global asset returns, 1800-2014                                                   | Confirms a significant momentum premium inside and across asset classes, but with large variation in the portfolio's market exposure over time                   |
| Pu, Roberts, Dong and Zohren, Network Momentum across Asset Classes | 64 futures across commodities, equities, bonds and currencies, 2000-2022          | A momentum-spillover strategy reached a reward-to-risk of 1.5 and 22 percent a year after scaling for volatility, using only price data                          |

Read together, the picture is this. Momentum inside and across asset classes is one of the better
documented patterns in finance, and the historical return of the five-fund version is high. But the
high return comes with a large fall of 47.77 percent in the same study, the reward for the risk is
only 0.78, and the asset list, the twelve-month window and the number three were all chosen by the
author after looking at the data. The independent tests mostly agree that the pattern exists; they
disagree on how much of the return is a reward for taking on more risk, and they show the result is
much bumpier around market stress.

## How this project relates to it

This repository contains its own study of the closely related question, written from a harvest of
research papers: [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md).
Its Section 1 keeps three different ideas apart, and Section 2 sets out when a ranking rule is
eligible at all. The asset-class version inherits the same warning: a rule that merely reorders a
fixed set of investments is still mostly a different way of being invested in the market.

The second related piece is the harvest of allocation evidence in
[portfolio construction and allocation](../../../strategies/books2/10_portfolio_and_allocation.md).
It reports that a factor model's ranking is not a property of the model but of how the test
portfolios are built, which is the same fragility that makes the five-fund version sensitive to the
choice of funds.

The engine in [implementation/sector-regime-engine](../../../implementation/sector-regime-engine/README.md)
does not run this strategy. It measures whether a market is in a state where such a rule is eligible
to be run at all, and reports the evidence for that verdict rather than a profit figure. Read its
[user manual](../../../implementation/sector-regime-engine/USER_MANUAL.md) first; it states plainly
what the app does not do.

## Where it goes wrong

- Only five funds. The rule compares five things, so one unusual family that happens to be in the
  news can dominate the ranking. The published study used long index histories, not the funds a
  reader can buy today, and several of those funds did not exist in the 1970s.
- It is mostly market exposure. Bonds and commodities are part of the set, but the equity-like
  funds win most of the time, so the long-only version falls when shares fall, and the measured gain
  over a simple mix is modest.
- Momentum crashes. After a market-wide fall, the families that had been strongest are often the
  ones that rebound hardest or fall hardest, and a strategy that bought them just before the turn
  takes the loss twice. That is where the 47.77 percent fall comes from.
- The rules were chosen after the fact. The five funds, the twelve-month window and the number
  three all came from the same study that reports the result, so part of the measured reward is the
  comfort of a rule that was fitted to history.
- Costs on a monthly rebuild. Each rebuild pays the gap between buying and selling prices. A
  strategy that changes its mind rarely is not the same strategy once the costs of a busy month are
  included.
- The signal can invert. Over very short horizons, days to weeks, prices tend to reverse rather than
  continue, which is why the 12-1 convention exists.

## Try it yourself

You need nothing but a spreadsheet and a public source of prices for the five funds or their
indexes.

1. Build a sheet with one column per fund and one row per month for the last five years.
2. Add a column that computes the twelve-month return: today's price divided by the price twelve
   rows up, minus one.
3. Add a column naming, for each month, the three funds with the highest value in that row.
4. In the next row down, average the next month's returns of those three funds. That is the
   strategy's return for the month.
5. Build a second column that just holds all five funds equally every month, as a comparison.
6. Subtract the cost: about 0.05 percent for each side of every position that changed from the
   previous month.

What to notice: on some rows the three chosen funds barely change, so hardly anything trades, while
on others all three change. The result over five years will usually be close to the comparison,
sometimes above it and sometimes below. If your sheet shows a wide win, the likely cause is a fund
that did not exist at the start of the period, which is a form of looking into the future.

## Where this came from

- [QuantConnect strategy library: asset class momentum](https://www.quantconnect.com/tutorials/strategy-library/asset-class-momentum),
  the rules as implemented: five asset-class funds, the top three by twelve-month return, equal
  weights, rebuilt monthly.
- [Quantpedia: momentum asset allocation](https://quantpedia.com/strategies/asset-class-momentum-rotational-system),
  the performance figures, the five funds and the underlying papers.
- Mebane Faber, [Relative Strength Strategies for Investing](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1585517),
  the original relative strength study.
- Kessler and Scherer, [Macro Momentum and the Economy](https://workspace.imperial.ac.uk/business-school/Public/research/annadvanceshedgefunds5/12_Kessler.pdf),
  the independent cross-asset test.
- `2308.11294v1`, Network Momentum across Asset Classes (2023), the machine-learning spillover test
  on 64 futures.
- [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md), this
  repository's own study of the related question, and specifically its Sections 1 and 2.

## Words used in this tutorial

- asset class: a whole family of investments that behave alike, such as company shares or bonds.
- basis point: one hundredth of one percent, so ten basis points is 0.10 percent.
- drawdown: the fall from a peak to the following low, measured in percent.
- momentum: the tendency of something that has been rising to keep rising for a while.
- rebalance: to sell some holdings and buy others so the portfolio matches the rule again.
- share: a unit of ownership in a company.
- volatility: how much a price moves around its average, measured as a percentage per year.
- weight: the share of your money placed in one holding.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
