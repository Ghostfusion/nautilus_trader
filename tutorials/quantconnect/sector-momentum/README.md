# Sector momentum: buying what has already been winning, one sector at a time

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                               |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Baskets of American shares, one basket per industry sector, held through funds that track each sector                                                                                                                               |
| How often it trades       | Once a month, when the portfolio is rebuilt                                                                                                                                                                                         |
| What you need             | A spreadsheet and twelve months of sector prices                                                                                                                                                                                    |
| Where the rules come from | [QuantConnect strategy library, sector momentum](https://www.quantconnect.com/tutorials/strategy-library/sector-momentum) and the [Quantpedia entry](https://quantpedia.com/strategies/sector-momentum-rotational-system) it cites  |
| The underlying research   | Mebane Faber, [Relative Strength Strategies for Investing](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1585517)                                                                                                             |
| How well it held up       | Mixed: a long published sample with a modest reward for the risk, replications that survive costs with funds, and independent tests that shrink the effect once the market exposure and the number of rules tried are accounted for |
| Also appears in           | [Asset class momentum](../asset-class-momentum/README.md) in this collection, its close relative                                                                                                                                    |

## The idea in one paragraph

Divide the stock market into its industries, such as technology, energy and utilities. Over the past
year some of those industries have done much better than others. This strategy buys the three
industries that did best over the past twelve months, in equal amounts, and holds them for a month.
At the start of the next month it looks again, sells whatever has dropped out of the top three, buys
whatever has entered, and repeats. The bet is that an industry that has been strong tends to stay
strong a little longer, so the portfolio spends its time in the parts of the market that are rising.

## Why anyone believed it

An industry's fortunes do not change overnight. When oil prices rise, energy companies earn more for
several quarters, and the news arrives gradually rather than all at once. Investors who hear the
news late buy after the first move, which pushes the price further. Money also moves in herds: funds
that are judged against their peers are reluctant to be the only one not holding the sector that is
working, so they buy into strength.

The counterparty, then, is the investor who is slow to update, or who is forced to sell for reasons
unrelated to the outlook: a fund facing redemptions, a manager trimming a position that has grown
too large, or a trader who takes profits simply because a price has risen. If those sellers keep
appearing, the winner keeps winning.

## An everyday comparison

Think of a university canteen with ten counters: noodles, salad, curry, soup and so on. On any given
day the queue at each counter is different, and the queue you can see is the result of what everyone
decided minutes ago. A diner who always joins the longest queue does not eat better than one who
walks straight to the shortest counter, but on the days when the popular counter is popular for a
real reason (fresh bread came out of the oven) the queue keeps growing, because each new arrival sees
the crowd and joins it. The strategy here is to join the longest three queues each month.

## The rules, step by step

1. Choose ten sector funds. In the American market these are commonly the ten Select Sector funds
   that together cover almost the whole large-company index.
2. For each sector fund, compute its return over the past twelve months: take the price twelve months
   ago, take today's price, and divide. A fund that went from 100.00 to 134.00 has a twelve-month
   return of 34 percent.
3. Rank the ten sectors by that return, best first.
4. Buy the top three, in equal amounts: one third of the money in each.
5. Hold for one month. Do not look at the prices in between.
6. At the start of the next month, recompute step 2 for all ten sectors and repeat from step 3.
   Sell anything that has fallen out of the top three and buy whatever has taken its place.
7. If you want the long and short version, additionally sell short the market index and hold the
   proceeds in cash. This tutorial describes the long-only version, which is the one both sources
   state.

One refinement worth knowing: many researchers skip the most recent month when measuring momentum,
using the return from twelve months ago to one month ago. The reason is that over days and weeks
prices tend to bounce back after a sharp move, which works against the signal. That convention is
usually written as 12-1 momentum.

## The maths, with every symbol named

The whole strategy is one calculation repeated ten times and one sort.

The twelve-month return of a sector:

```text
M = P_today / P_twelve_months_ago - 1
```

- `M` is the momentum score of the sector, written as a decimal: 0.34 means 34 percent.
- `P_today` is the fund's price today.
- `P_twelve_months_ago` is the fund's price on the same day twelve months earlier.

Then rank the ten sectors by `M`, from largest to smallest, and keep the top three. Give each of the
three the same weight:

```text
w_i = 1 / 3 for each of the three selected sectors, and 0 for the other seven
```

- `w_i` is the fraction of the money placed in sector `i`.
- The weights of the chosen three add up to 1, so the whole account is invested, in three equal
  slices.

The portfolio's return over the following month is then the average of the three sector returns,
because the slices are equal:

```text
R_portfolio = (R_1 + R_2 + R_3) / 3
```

- `R_1`, `R_2` and `R_3` are the next-month returns of the three chosen sectors.
- Dividing by three is the same as multiplying each by one third and adding.

Finally, the cost of rebuilding the portfolio each month. If a fraction `t` of the money is traded,
the cost in that month is:

```text
Cost = t * c
```

- `t` is the traded fraction: 2.0 if the entire portfolio is sold and replaced at once, because
  selling the old holdings and buying the new ones counts twice, and less than 2.0 when some
  holdings are kept.
- `c` is the cost per trade as a fraction of the amount traded, covering the gap between the buying
  and selling price plus commission. A realistic figure for large sector funds is 0.0005 to 0.001,
  that is five to ten basis points, where one basis point is one hundredth of one percent.

## A worked example

Ten sectors, ranked by their return over the past twelve months. The returns below are invented, but
they are of the size that sector returns actually take.

| Sector fund            | Price a year ago | Price today | M     | Rank |
| ---------------------- | ---------------- | ----------- | ----- | ---- |
| Technology             | 100.00           | 134.00      | 0.34  | 1    |
| Energy                 | 80.00            | 97.60       | 0.22  | 2    |
| Financials             | 120.00           | 141.60      | 0.18  | 3    |
| Industrials            | 90.00            | 102.60      | 0.14  | 4    |
| Materials              | 110.00           | 119.90      | 0.09  | 5    |
| Consumer staples       | 75.00            | 80.25       | 0.07  | 6    |
| Health care            | 140.00           | 147.00      | 0.05  | 7    |
| Utilities              | 60.00            | 61.20       | 0.02  | 8    |
| Consumer discretionary | 130.00           | 126.10      | -0.03 | 9    |
| Real estate            | 95.00            | 87.40       | -0.08 | 10   |

The three chosen sectors are technology, energy and financials, one third of the money each. Now
suppose the next month produces these returns:

| Sector held | Weight | Next-month return | Contribution    |
| ----------- | ------ | ----------------- | --------------- |
| Technology  | 0.3333 | +2.0 percent      | +0.6667 percent |
| Energy      | 0.3333 | -1.0 percent      | -0.3333 percent |
| Financials  | 0.3333 | +1.5 percent      | +0.5000 percent |
| Total       | 1.0000 |                   | +0.8333 percent |

So the portfolio gained 0.8333 percent before costs. The two sectors that fell out of the top three
had to be sold, and the two that entered had to be bought; in this example four of the six traded
positions changed, so roughly two thirds of the money moved, counting both sides:

```text
t = 2 * (2 / 3) = 1.33
Cost = 1.33 * 0.001 = 0.00133, that is 0.133 percent
Net return for the month = 0.8333 - 0.133 = 0.70 percent
```

Twelve months at that rate is about 8.7 percent a year before costs and 8.1 percent after, which
sits close to the published figures below. Two things are worth noticing. First, the cost line is
large enough to matter: a strategy that trades the whole portfolio every month pays roughly 2.4
percent a year at ten basis points per trade. Second, the worked example says nothing about whether
the strategy works. It only shows how to apply the rules and how the arithmetic behaves.

## What the research actually found

The published record for sector momentum is unusually long, and unusually split.

| Source                                                                        | What it measured                                                       | Result                                                                                                                                                                                                                       |
| ----------------------------------------------------------------------------- | ---------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Quantpedia, summarising Faber's relative strength work                        | Top three sectors of ten, monthly rebalancing, American data           | 13.94 percent a year, volatility 18.38 percent, worst fall 46.29 percent, reward-to-risk 0.54, over 1928 to 2009, about four points a year better than holding the index                                                     |
| Faber, Relative Strength Strategies for Investing                             | Relative strength on the Fama and French sector data back to the 1920s | Beat buy and hold in roughly seventy percent of years, with returns persistent across time; adding a trend filter reduced the worst falls                                                                                    |
| Andreu, Swinkels and Tjong-A-Tjoe                                             | Tradable country and industry funds, actual prices                     | About five percent a year more than the benchmark over the periods the funds existed, with the gap between buying and selling prices well below the level that would erase it                                                |
| Huhn                                                                          | Industry portfolios, adjusting for the market and factor exposures     | Profitability shrinks once the exposure is accounted for, and becomes statistically indistinguishable from zero for rules based on the most recent month                                                                     |
| Arnott, Clements, Kalesnik and Linnainmaa                                     | The cross-section of industry returns                                  | Industry momentum is largely a by-product of momentum in the underlying factors; the predictability is strongest at the one-month horizon                                                                                    |
| Molchanov and Stangl, reported in this repository's own sector rotation study | 1,022 possible rotation rules, American sectors                        | The average rule returned 0.86 percent a month against 0.89 percent for simply holding the market; 132 rules of 1,022 beat buy and hold, and the authors concluded the outperformance was the result of trying so many rules |

Read together, the picture is this. There is a real, replicated tendency for strong industries to stay
strong over months, and it is one of the better documented patterns in the literature. But the size of
the prize is modest once the market exposure is accounted for, the number of rules that were tried in
the literature is large, and a rotation fund run by professionals over 2010 to 2018 returned 7.34
percent against 12.23 percent for the index, according to the study in this repository. A long-only
sector strategy is, in the end, a different way of being invested in the stock market.

## How this project relates to it

This repository contains its own study of exactly this question, written from a harvest of research
papers: [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md). Section
3.2 of that document states the condition the strategy needs (leaders that stay leaders for several
months) and Section 3.3 collects the tests above, including the 1,022-rule experiment and the
cross-sector regression test whose 2,160 test statistics came out consistent with chance.

The second related piece is the engine in
[implementation/sector-regime-engine](../../../implementation/sector-regime-engine/README.md). It
does not run sector momentum as a strategy. It measures whether a market is in a state where such a
rule is eligible to be run at all, and it reports the evidence for that verdict rather than a profit
figure, following the study above. Read its
[user manual](../../../implementation/sector-regime-engine/USER_MANUAL.md) before touching it; it
states plainly what the app does not do.

## Where it goes wrong

- Momentum crashes. After a market-wide fall, the sectors that fell furthest are the ones that led
  the previous ranking, and they often bounce hardest. A strategy that bought them just before the
  fall takes the loss twice, which is where the published worst fall of 46 percent comes from.
- Crowding. Once a rule is published and easy to trade through funds, more money runs it. The
  buying happens earlier, the returns arrive sooner and smaller, and the latecomers are the ones who
  pay for the reversal.
- It is mostly market exposure. The long-only version is invested in shares at all times, so it
  falls when the market falls. The measured improvement over the index is a few points a year, and
  part of that is the market's own rise.
- The number of rules tried. There are many ways to define momentum: which twelve months, whether to
  skip the most recent one, how many sectors to hold, how often to rebuild, whether to demand the
  sector also be above its own average. Choosing the best of these after seeing the results is how
  the 1,022-rule experiment produced its conclusion.
- Costs on a monthly rebuild. Every rebuild pays the gap between buying and selling prices and any
  commission. The more often the ranking changes, the more the strategy pays, and a strategy that
  changes its mind rarely is not the same strategy.
- The signal can invert. Over very short horizons, days to weeks, prices tend to reverse rather than
  continue, which is why the 12-1 convention exists and why a rule built on one month of returns can
  lose money even though the twelve-month version earned it.

## Try it yourself

You need nothing but a spreadsheet and a public source of sector prices; both the QuantConnect page
and any finance website will give you monthly closing prices for the ten Select Sector funds.

1. Build a sheet with one column per sector fund and one row per month for the last five years.
2. Add a column that computes the twelve-month return: today's price divided by the price twelve
   rows up, minus one.
3. Add a column, for each month, naming the three sectors with the highest value in that row. This is
   what the rules would have bought at that moment.
4. In the next row down, average the next month's returns of those three sectors. That is the
   strategy's return for the month.
5. Do the same for the market index on its own, so you have two columns to compare.
6. Finally, add a column that subtracts the cost: about 0.10 percent for each side of every position
   that changed from the previous month.

What to notice: on some rows the three chosen sectors are almost the same as the previous month, so
nothing trades and the cost is near zero, while on others all three change. The result over five
years will usually be close to the index, sometimes above it and sometimes below. That outcome is the
honest finding of the literature. If your sheet shows the strategy winning by a wide margin, the
likely cause is that the sector list changed over time (funds that did not exist twenty years ago),
which is a form of looking into the future.

## Where this came from

- [QuantConnect strategy library: sector momentum](https://www.quantconnect.com/tutorials/strategy-library/sector-momentum),
  the rules as implemented: ten sector funds, the top three by twelve-month return, equal weights,
  rebuilt monthly.
- [Quantpedia: sector momentum rotational system](https://quantpedia.com/strategies/sector-momentum-rotational-system),
  the performance figures, the instrument count and the underlying papers.
- Mebane Faber, [Relative Strength Strategies for Investing](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1585517),
  the original relative strength study on sector data.
- Moskowitz and Grinblatt, [Do Industries Explain Momentum?](https://onlinelibrary.wiley.com/doi/abs/10.1111/0022-1082.00146),
  the paper that established that industry momentum accounts for much of the momentum seen in
  individual shares.
- [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md), this
  repository's own study of the question, and specifically its Sections 3.2 and 3.3, which is where
  the 1,022-rule experiment, the cross-sector regression test and the fund comparison come from.

## Words used in this tutorial

- basis point: one hundredth of one percent, so ten basis points is 0.10 percent.
- drawdown: the fall from a peak to the following low, measured in percent.
- momentum: the tendency of something that has been rising to keep rising for a while.
- sector: a group of companies in the same line of business, such as energy or health care.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- volatility: how much a price moves around its average, measured as a percentage per year.
- weight: the share of your money placed in one holding.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
