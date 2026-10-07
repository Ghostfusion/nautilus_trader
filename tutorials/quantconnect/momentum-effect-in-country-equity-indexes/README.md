# Momentum in country equity indexes: buying the national markets that have been rising

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                 |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Funds that each track one country's main stock market index, such as a fund holding the largest shares of Brazil or of Japan                                                                                                                                                                          |
| How often it trades       | About once a month, when the holding list is rebuilt                                                                                                                                                                                                                                                  |
| What you need             | A spreadsheet and six months of prices for a list of country index funds                                                                                                                                                                                                                              |
| Where the rules come from | [QuantConnect strategy library, momentum effect in country equity indexes](https://www.quantconnect.com/tutorials/strategy-library/momentum-effect-in-country-equity-indexes) and the [Quantpedia entry](https://quantpedia.com/strategies/momentum-factor-effect-in-country-equity-indexes) it cites |
| The underlying research   | Muller and Ward, [Momentum effects in country equity indexes](http://repository.up.ac.za/bitstream/handle/2263/14149/Muller_Momentum%282010%29.pdf?sequence=1)                                                                                                                                        |
| How well it held up       | Mixed: one long sample reports about ten percent a year more than an equal-weighted benchmark and says funds could capture it, but the reward differs by roughly a factor of two between sources and later work reports the edge fading after about 2010                                              |
| Also appears in           | [Momentum effect in stocks](../momentum-effect-in-stocks/README.md), the same ranking applied to single companies, and [Sector momentum](../sector-momentum/README.md), the same idea applied to industries                                                                                           |

## The idea in one paragraph

Divide the world into countries, and represent each country by one fund that holds that country's
largest shares. Over the past six months some national markets have done much better than others.
This strategy buys the five countries that did best over the past six months, in equal amounts, and
holds them for a month. At the start of the next month it looks again, sells the countries that
have dropped out, buys the ones that have entered, and repeats. The bet is that a national market
that has been rising tends to keep rising for a while, so the money sits in the parts of the world
that are already moving up.

## Why anyone believed it

A country's economy is a large, slow object. When a currency weakens, when a central bank cuts
interest rates, or when the price of the country's main export climbs, company profits react over
several quarters rather than on a single morning. Investors who hear the news late buy after the
first move, which pushes prices further, and money crosses borders slowly as funds adjust their
international holdings.

The counterparty, then, is the investor who reacts slowly to news about a country's economy, or who
sells for reasons that have nothing to do with the outlook: a fund cutting its emerging-market
weight, a manager taking a profit after a big run, or a saver pulling money out at the wrong time.
The researchers Bhojraj and Swaminathan argued that these patterns come from investors
mis-reacting to news about the whole economy rather than to news about any single company, and that
the mis-reaction is larger for countries than for companies.

## An everyday comparison

Think of a row of neighbouring towns each with its own weekly food market. One town builds a new
road, so more shoppers come, the traders earn more, and they hire more staff. Word spreads, and over
the next several months the successful market keeps attracting more people. The market that was
busy last month tends to be busy again next month, because the reasons for its success do not
disappear overnight. The strategy here is to spend each month at the five busiest markets, on the
bet that the crowd will keep coming.

## The rules, step by step

1. Choose a list of country index funds. A country index fund is a single listed thing that holds
   the main shares of one country, so buying it is a way to own that country's whole market in one
   trade. The library page uses thirty-five such funds.
2. For each fund, compute its return over the past six months: take the price six months ago, take
   today's price, and divide. A fund that went from 40.00 to 52.00 has a six-month return of 30
   percent.
3. Rank the funds by that return, best first.
4. Buy the best five, in equal amounts: one fifth of the money in each. The library page uses the
   best five of thirty-five; the worked example below holds the best two of ten, which is the same
   one-in-five fraction.
5. Hold for one month. Do not buy or sell in between.
6. At the start of the next month, recompute step 2 for all the funds and repeat. Sell anything that
   has fallen out of the best five and buy whatever has taken its place.
7. Note the choice of six months. The source paper used eleven months, and Quantpedia reports that
   look-back periods of ten to twelve months work best. The library page uses six. Which period is
   used is a decision, not a fact, and different choices give different results.

## The maths, with every symbol named

The strategy is one calculation repeated for each country fund, one sort, and a cost line.

The six-month return, called the momentum score:

```text
M = P_today / P_six_months_ago - 1
```

- `M` is the momentum score of the country fund, as a decimal: 0.30 means 30 percent.
- `P_today` is the fund's price today.
- `P_six_months_ago` is the fund's price six months earlier.

Rank the funds by `M`, largest first, and keep the top five. Give each the same weight:

```text
w_i = 1 / 5 for each of the five selected funds, and 0 for the rest
```

- `w_i` is the fraction of the money placed in fund `i`.
- The five weights add up to 1, so the whole account is invested, in five equal slices.

The portfolio's return over the following month is the average of the five fund returns, because the
slices are equal:

```text
R_portfolio = (R_1 + R_2 + R_3 + R_4 + R_5) / 5
```

- `R_1` to `R_5` are the next-month returns of the five chosen funds.
- Dividing by five is the same as multiplying each return by one fifth and adding them up.

The cost of rebuilding the list each month:

```text
Cost = t * c
```

- `t` is the traded fraction: 2.0 if the whole portfolio is sold and replaced at once, because each
  sale and each purchase counts as one trade, and less than 2.0 when some funds are kept.
- `c` is the cost per trade as a fraction of the amount traded, covering the gap between the buying
  and selling price plus commission. Large country index funds are among the cheapest things to
  trade, so a realistic figure is 0.0005 to 0.001, that is five to ten basis points, where one basis
  point is one hundredth of one percent.

## A worked example

The table below uses a made-up list of ten country funds, standing in for the real thirty-five. The
rule is to hold the best fifth, which here means the best two. The returns are invented but are of
the size national markets actually take over half a year.

| Country fund   | Price six months ago | Price today | M              | Rank |
| -------------- | -------------------- | ----------- | -------------- | ---- |
| Brazil         | 40.00                | 52.00       | +0.30 (30 pct) | 1    |
| India          | 90.00                | 112.50      | +0.25 (25 pct) | 2    |
| China          | 60.00                | 72.00       | +0.20 (20 pct) | 3    |
| Germany        | 100.00               | 113.00      | +0.13 (13 pct) | 4    |
| Japan          | 130.00               | 140.40      | +0.08 (8 pct)  | 5    |
| United States  | 150.00               | 156.00      | +0.04 (4 pct)  | 6    |
| United Kingdom | 80.00                | 80.80       | +0.01 (1 pct)  | 7    |
| France         | 110.00               | 106.70      | -0.03 (3 pct)  | 8    |
| Canada         | 70.00                | 66.50       | -0.05 (5 pct)  | 9    |
| Australia      | 50.00                | 45.00       | -0.10 (10 pct) | 10   |

The two chosen funds are Brazil and India, half the money in each. Now suppose the next month
produces these returns:

| Fund held | Weight | Next-month return | Contribution  |
| --------- | ------ | ----------------- | ------------- |
| Brazil    | 0.50   | +3.0 percent      | +1.50 percent |
| India     | 0.50   | -1.0 percent      | -0.50 percent |
| Total     | 1.00   |                   | +1.00 percent |

So the portfolio gained 1.00 percent before costs. Now suppose that when the list is rebuilt, Brazil
stays in the top two but India drops out and a new fund enters. One of the two holdings is sold and
one is bought, which is half the money traded on each side:

```text
t = 2 * (1 / 2) = 1.0
Cost = 1.0 * 0.001 = 0.001, that is 0.10 percent
Net return for the month = 1.00 - 0.10 = 0.90 percent
```

If that same figure repeated every month for a year it would compound to about 12.7 percent before
costs and 11.4 percent after. Two things are worth noticing. First, the cost line is small here
because country funds are cheap to trade: a list that changes half its holdings every month pays
roughly 1.2 percent a year at ten basis points per trade. Second, the example says nothing about
whether the strategy works. It only shows how to apply the rules and how the arithmetic behaves.

## What the research actually found

The published record for country momentum is long, and the sources do not agree on its size.

| Source                            | What it measured                                                               | Result                                                                                                                                                                                 |
| --------------------------------- | ------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Muller and Ward, the source paper | The 70 country indexes of the MSCI World index, 1970 to 2009                   | Holding for one month the four best markets over the previous eleven months beat an equal-weighted benchmark by around 10 percent a year                                               |
| Quantpedia, summarising the paper | Top five countries by a look-back of ten to twelve months, monthly rebalancing | 17.7 percent a year, worst fall 65.39 percent, over 1968 to 2009                                                                                                                       |
| QuantConnect's own page           | Thirty-five country funds, top five by six-month return, 2002 to 2022          | Claims the rule out-performed an equal-weighted fund by "around 90 percent per annum", a figure no other source repeats and which is almost certainly a typo for a much smaller number |
| Andreu, Swinkels and Tjong-A-Tjoe | Actual traded country and industry funds, real prices                          | About five percent a year more than the benchmark over the periods the funds existed, with the gap between buying and selling prices far below the level that would erase it           |
| Bhojraj and Swaminathan           | Portfolios of international stock indexes                                      | Strong momentum up to a year after a portfolio is formed, then reversals over the following two years                                                                                  |
| Balvers and Wu                    | Eighteen developed markets, monthly                                            | Rules combining momentum and its opposite beat either alone; momentum works best when the tendency to revert is also taken into account                                                |
| Du and Vojtko                     | Country and asset funds, recent years                                          | The effect was robust until about 2010 and then showed signs of fading                                                                                                                 |

Read together, the picture is this. There is a real, replicated tendency for strong national markets
to stay strong over months, and funds make it possible to trade with costs low enough to matter less
than the signal. But the reported size of the prize ranges from about five percent a year to about
eighteen, the single most eye-catching figure on the library page is not credible, and the most
recent studies find the edge weaker than it used to be. Reporting the disagreement is the honest
result.

## How this project relates to it

This repository studies the same question at the level of industries and countries in [Profiting
from sector rotation](../../../strategies/sector_rotation_strategies.md). Section 7 of that
document shows that momentum in individual shares is largely momentum in the group they belong to,
which is the same mechanism that makes a whole country's market move together, and it warns that
the strong historical samples end in the 1990s.

The research brief
[the momentum design brief](../../../strategies/books2/08_predictability_and_trading_strategies.md)
covers momentum as a design problem: it records that turnover and costs decide whether a paper
profit survives, and that the number of look-back and holding choices that were tried is itself a
reason to distrust the best-looking result.

## Where it goes wrong

- Crowding. Country funds are cheap and easy to trade, which is exactly what lets many people run
  the same rule. The buying happens earlier, the reward arrives sooner and smaller, and the late
  entrants pay for the reversal.
- It is mostly world equity exposure. The long-only version is invested in shares at all times, so
  it falls when world markets fall. The measured improvement over a simple benchmark is a few points
  a year, not a new source of return.
- The number of choices tried. Six months or eleven or twelve, five countries or four, monthly or
  quarterly: each combination gives a different answer, and the one that looks best in hindsight is
  the one most likely to be luck.
- Costs and access on the smaller countries. The funds of the smallest markets have wider gaps
  between buying and selling prices, and the rule tends to buy the countries that have just run
  hardest, which are often the small and less liquid ones.
- Recent decay. The most recent study in the table finds the effect weaker after about 2010, which
  is what one should expect once a rule is widely known.
- Long-only momentum can be a bad hedge. When world markets fall together, country funds tend to
  fall together too, so the rule offers little protection just when protection is wanted.

## Try it yourself

You need nothing but a spreadsheet and a public source of country fund prices; a finance website
will give you monthly closing prices for country index funds.

1. Build a sheet with one column per country fund and one row per month for the last five years.
2. Add a column that computes the six-month return: today's price divided by the price six rows up,
   minus one.
3. Add a column, for each month, naming the two funds with the highest value in that row. This is
   what the rules would have bought at that moment.
4. In the next row down, average the next month's returns of those two funds. That is the strategy's
   return for the month.
5. Do the same for a single world market fund on its own, so you have two columns to compare.
6. Finally, add a column that subtracts the cost: about 0.10 percent for each side of every position
   that changed from the previous month.

What to notice: the reported edge is small enough that the cost line and the choice of look-back
period can change the answer. If you repeat the sheet with a twelve-month look-back instead of six,
you will usually get a visibly different result, which is the point. If your sheet shows the rule
winning by a wide margin, the likely cause is that the list of funds changed over time or that you
have chosen the best look-back after seeing the answer.

## Where this came from

- [QuantConnect strategy library: momentum effect in country equity indexes](https://www.quantconnect.com/tutorials/strategy-library/momentum-effect-in-country-equity-indexes),
  the rules as implemented: thirty-five country funds, the six-month ranking, the top five and the
  monthly rebuild.
- [Quantpedia: momentum factor effect in country equity indexes](https://quantpedia.com/strategies/momentum-factor-effect-in-country-equity-indexes),
  the indicative performance, the instrument count and the list of underlying papers.
- Muller and Ward, [Momentum effects in country equity indexes](http://repository.up.ac.za/bitstream/handle/2263/14149/Muller_Momentum%282010%29.pdf?sequence=1),
  the source paper behind the rule.
- Balvers and Wu, momentum and mean reversion across national equity markets, the paper that shows
  momentum and reversion are two faces of the same data.
- [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md), this
  repository's own study, especially its Section 7 on when group-level persistence is tradable.
- [the momentum design brief](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  this repository's brief on momentum design, turnover and costs.

## Words used in this tutorial

- benchmark: a simple, well-known holding used as a yardstick, such as an index.
- drawdown: the fall from a peak in value to a later low, measured as a percentage of the peak.
- ETF: an exchange-traded fund, a single listed thing that holds a basket of other assets and trades
  like a share.
- index: a published list of shares whose combined price is tracked as a single number.
- look-back: the length of the past window over which the ranking is measured.
- momentum: the tendency of something that has been rising to keep rising for a while.
- rebalance: adjusting a portfolio back to its intended weights by buying and selling.
- universe: the full set of things a rule is allowed to choose from.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
