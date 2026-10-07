# Smart factors and the market: blending several style bets with a plain index holding

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                                    |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| What it trades            | Five style baskets built from the largest American shares, blended each month with one fund that tracks a broad American share index                                                                                                                                                                                     |
| How often it trades       | Once a month, when the weights are reset                                                                                                                                                                                                                                                                                 |
| What you need             | A spreadsheet and a monthly history of the five style baskets and the index                                                                                                                                                                                                                                              |
| Where the rules come from | [The list's implementation file](https://github.com/paperswithbacktest/awesome-systematic-trading/blob/main/static/strategies/combining-smart-factors-momentum-and-market-portfolio.py), which restates [the Quantpedia entry](https://quantpedia.com/strategies/combining-smart-factors-momentum-and-market-portfolio/) |
| The underlying research   | Matus Padysak, [The active vs passive: smart factors, market portfolio or both?](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=3745517)                                                                                                                                                                            |
| How well it held up       | Weak: a single author's study, on American and international samples, with no independent replication, even though Quantpedia rates its confidence in the idea as Strong                                                                                                                                                 |
| Also appears in           | [Fama French five factors](../../../tutorials/quantconnect/fama-french-five-factors/README.md) and [Momentum and style rotation effect](../../../tutorials/quantconnect/momentum-and-style-rotation-effect/README.md)                                                                                                    |

## The idea in one paragraph

A style basket is a bet on one kind of share: cheap shares, profitable shares, small shares, rising shares
or calm shares. Each basket is built by buying the best tenth of the largest American companies on that one
measure and selling short the worst tenth. This strategy watches how each basket has performed, gives more
money to the baskets that are moving most, and then blends that whole package with a simple holding in a
broad share index. The blend is decided month by month by comparing the average returns of the two sides
over several different time windows. The bet is that the style baskets and the index tend to do badly at
different times, so splitting the money between them produces a steadier result than either alone.

## Why anyone believed it

The style baskets are not all the same kind of bet. A cheap-shares basket tends to struggle when the market
is booming, because expensive, fast-growing companies lead. A small-shares basket struggles when investors
are nervous and prefer large, safe names. The author measured a negative relationship between the style
package and the index: when the market has a bad stretch, the style bets have often had a good one.

That relationship needs a counterparty on each side. The index is bought by people who want the market's
return and nothing else, including savers who buy automatically every month. The style baskets are sold by
people who abandon a style after a poor run, and bought by people who chase whichever style has just worked.
If those flows keep happening, the two sides keep moving out of step, and a rule that always holds some of
both can take advantage of it.

## An everyday comparison

Think of a household splitting its weekly food budget between a vegetable box delivered from a farm and a
standing order at the local supermarket. When the farm has a bad season, the box is poor value, but the
supermarket is reliable, and when the supermarket raises prices the box looks better. A shopper who moves
money towards whichever source has done better over the last few weeks, without ever abandoning either one
completely, ends up with a steadier supply than someone who switches entirely to the source that just had a
good month. That is the blend this strategy builds.

## The rules, step by step

1. Choose five style measures for the largest 1,500 American shares: value (cheapness), momentum (past
   strength), quality (profitability), size (smaller companies) and volatility (calmer shares). For each
   measure, build a basket that buys the top tenth and sells short the bottom tenth. Treat each basket as a
   single thing whose price moves.
2. For each basket, compute two readings. The fast reading is its return over the past one month. The slow
   reading is its return over the past twelve months.
3. For each reading separately, rank the five baskets by the size of the reading, ignoring whether it is
   positive or negative. The basket with the largest absolute reading gets rank 5, the smallest gets rank 1.
4. Turn each rank into a weight: divide the rank by 15, which is the sum of the numbers 1 to 5, and then
   attach the sign of the reading, plus or minus. So the largest positive reading gives weight 5/15, and the
   largest negative reading gives minus 5/15.
5. Blend the two readings for each basket: three quarters of the fast weight plus one quarter of the slow
   weight. The fast reading gets more weight because a twelve-month reading is slow to notice that a style
   has changed, while a one-month reading is quick but often wrong.
6. The style package's return for the month is the sum, over the five baskets, of each basket's weight
   multiplied by its return over that month.
7. Record the style package's return and the index's return for each of the past twelve months.
8. Compute twelve averages of the style package's returns: the average over the last 1 month, over the last
   2 months, and so on up to 12 months. Do the same for the index. For each of the twelve comparisons, give
   one point to whichever side has the higher average.
9. The index's share is its points divided by 12, and the style package's share is its points divided by 12.
   Put that much money in the index and scale every basket weight by the style package's share.
10. Rebuild at the start of each month and pay the costs, roughly 0.05 to 0.15 percent per side for liquid
    instruments, with a borrow fee on anything held short.

## The maths, with every symbol named

The two readings for each basket:

```text
F = P_today / P_one_month_ago - 1
S = P_today / P_twelve_months_ago - 1
```

- `F` is the fast reading, the basket's one-month return.
- `S` is the slow reading, the basket's twelve-month return.
- `P_today`, `P_one_month_ago` and `P_twelve_months_ago` are the basket's prices on those dates.

The rank weight, for each reading:

```text
weight = (rank / 15) * sign(reading)
```

- `rank` runs from 1 to 5, with 5 given to the largest absolute reading.
- 15 is the sum 1 + 2 + 3 + 4 + 5.
- `sign(reading)` is +1 if the reading is positive and -1 if it is negative.
- So a basket that moved most and moved upwards gets the largest positive weight, and one that moved most
  downwards gets the largest negative weight.

The blend of the two readings, and the package's monthly return:

```text
w_basket = 0.75 * w_fast + 0.25 * w_slow
R_package = sum over baskets of ( w_basket * R_basket )
```

- `w_fast` and `w_slow` are the rank weights from the fast and slow readings.
- `w_basket` is the final weight of a basket in the style package.
- `R_basket` is the basket's return over the month just ended, and `R_package` is the package's return.

The split between the package and the index:

```text
points_package = number of months, from 1 to 12, where the package's average return is higher
points_index   = 12 - points_package
weight_index   = points_index / 12
weight_package = points_package / 12
```

- "the package's average return" over `k` months means the mean of the package's last `k` monthly returns.
- The index weight and the package weight add to 1, so the account is always fully invested.

The cost:

```text
Cost = t * c
```

- `t` is the traded fraction of the account, counting both the sale and the purchase.
- `c` is the cost per side as a fraction of the amount traded, around 0.001 for 10 basis points.

## A worked example

First the basket weighting, for one month. The readings are invented. The sign is attached after ranking by
size, so the checks are on the absolute values.

| Basket     | Fast reading | Fast rank | Fast weight | Slow reading | Slow rank | Slow weight | Blended weight |
| ---------- | ------------ | --------- | ----------- | ------------ | --------- | ----------- | -------------- |
| Momentum   | +3.0%        | 5         | +0.3333     | +8.0%        | 5         | +0.3333     | +0.3333        |
| Value      | -1.5%        | 3         | -0.2000     | +2.0%        | 2         | +0.1333     | -0.1167        |
| Quality    | +2.0%        | 4         | +0.2667     | -1.0%        | 1         | -0.0667     | +0.1833        |
| Size       | +0.5%        | 1         | +0.0333     | -3.0%        | 3         | -0.2000     | -0.0250        |
| Volatility | +1.0%        | 2         | +0.1333     | +4.0%        | 4         | +0.2667     | +0.1667        |

The blend for a row is three quarters of the fast weight plus one quarter of the slow weight. For quality:

```text
0.75 * 0.2667 + 0.25 * (-0.0667) = 0.2000 - 0.0167 = 0.1833
```

The five blended weights do not add to 1; they add to about 0.54, because the weights are signed and some
baskets are bets against a style. The package is later scaled by its share of the account, so the scale does
not matter.

Now the split between the package and the index. Six months of returns are used, and the twelve monthly
averages are shortened to the six that the six months allow. The averages are computed from the start of the
window, so the average for `k` months uses the first `k` months.

| Month | Package return | Index return |
| ----- | -------------- | ------------ |
| 1     | +1.0%          | +0.5%        |
| 2     | +0.8%          | +1.2%        |
| 3     | -2.0%          | -1.0%        |
| 4     | +0.5%          | +2.0%        |
| 5     | +1.5%          | +0.3%        |
| 6     | +0.9%          | +0.4%        |

| Window length | Package average | Index average | Point goes to |
| ------------- | --------------- | ------------- | ------------- |
| 1 month       | +1.00%          | +0.50%        | Package       |
| 2 months      | +0.90%          | +0.85%        | Package       |
| 3 months      | -0.07%          | +0.23%        | Index         |
| 4 months      | +0.08%          | +0.68%        | Index         |
| 5 months      | +0.36%          | +0.60%        | Index         |
| 6 months      | +0.45%          | +0.57%        | Index         |

The package has 2 points and the index has 4, so the index's share is 4/6 = 0.6667 and the package's share
is 2/6 = 0.3333. The values are divided by 6 rather than 12 only because six months are shown.

In month 7 the index returns +1.0 percent and the package returns -1.0 percent. The account earns:

```text
0.6667 * 1.0 + 0.3333 * (-1.0) = 0.6667 - 0.3333 = 0.3333 percent
```

Costs: suppose a third of the account is moved between the index and the package and the baskets are partly
replaced, so the traded fraction `t` is about 0.5. At 10 basis points per side:

```text
Cost = 0.5 * 0.001 = 0.0005, that is 0.05 percent
Net return = 0.3333 - 0.05 = 0.28 percent
```

The worked example shows the mechanics, not a forecast. Its most important feature is that the account is
almost always holding some of each side, so it behaves like a share holding with a style tilt most of the
time.

## What the research actually found

| Source                                                                   | What it measured                                                                                 | Result                                                                                                                                                                                                                                                                                          |
| ------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Padysak, The active vs passive: smart factors, market portfolio or both? | Five style baskets and the top 1,500 American shares, 1993 to 2020, plus an international sample | One dollar invested at the end of 1993 became 20.28 dollars by 31 August 2020, against 14.52 dollars for the market alone; Quantpedia's page reports 11.91 percent a year with volatility 10.46 percent and a reward-to-risk of 1.14                                                            |
| The awesome-systematic-trading list, its own replication record          | 4,843 coded papers; aggregate statistics only                                                    | The median strategy returns a reward-to-risk of 0.37 and 48 percent clear a statistical significance bar of 1.96; the median strategy carries a market exposure of +0.17 to the index, and removing it takes the median information ratio to 0.21; this is the list's own aggregate measurement |
| The same paper's reported correlation                                    | The style package against the market                                                             | The paper reports a statistically significant negative correlation between the two, which is the whole reason for blending them                                                                                                                                                                 |

Read together: there is one study of this combination, by one author, on two regional samples. It reports a
large improvement over holding the market alone, and Quantpedia's page rates its confidence in the idea as
Strong. But no independent group has replicated the blend, and the strategies the author built on are
themselves factor portfolios whose returns are measured gross of the costs of constructing them. The
list's aggregate record is a useful caution here: the median strategy in the whole catalogue carries a
market exposure of +0.17, and a share-heavy blend will inherit a lot of the market's own return.

## How this project relates to it

This repository covers the building blocks of the style baskets.
[Fama French five factors](../../../tutorials/quantconnect/fama-french-five-factors/README.md) explains what
the value, size, profitability and investment baskets are and how they are formed.
[Momentum and style rotation effect](../../../tutorials/quantconnect/momentum-and-style-rotation-effect/README.md)
tests whether the styles themselves can be timed, which is the same question the fast and slow readings
answer here.

For how much of a blended cross-sectional signal survives when it is used to allocate, the brief
[strategies/books2/10_portfolio_and_allocation.md](../../../strategies/books2/10_portfolio_and_allocation.md)
is the closest thing here. Its section on factor models reports that a factor model's measured performance
depends heavily on the test assets and the rebalancing rule, and its section on what actually allocates
reports that most published predictors replicate but that their weak correlations mean a wide search
produces far less diversification than it looks.

## Where it goes wrong

- One study, one author. The blend is the newest and least tested part of this idea, and no independent
  group has run it. A single measured result from one history is not evidence of a durable edge.
- The style baskets are hard to build by hand. Buying the top tenth and shorting the bottom tenth of 1,500
  companies means hundreds of positions, and the costs of forming those baskets are not in the return series
  the study starts from.
- The market exposure is real. The blend holds the index for a large share of the time, and when it does it
  falls with the market. The list's own aggregate note that the median strategy carries a market exposure
  of +0.17 is a warning about how much of a published edge can be index return rather than skill.
- The comparison window is a choice. Using twelve monthly averages from 1 to 12 months is one of many ways
  to decide the split, and a different window or a different number of averages would produce a different
  allocation. Choosing the window after seeing the results is how a backtest is flattered.
- Costs and borrowing. Shorting the style baskets pays a borrow fee, and the fast signal changes often, so
  the baskets themselves turn over. The study's performance figures are for the blends before those
  construction costs are fully charged.
- The signals are absolute-value ranks. Giving the largest weight to the biggest move, whichever direction
  it goes, means the package is often simultaneously long and short the same style, which makes the net
  exposure hard to reason about from a spreadsheet alone.

## Try it yourself

You need a spreadsheet and monthly return series for five style baskets and one broad index. If you do not
have baskets, use five exchange-traded funds that track different styles plus a broad index fund.

1. Build a sheet with one column per basket plus one for the index, and one row per month, holding each
   month's return.
2. For a chosen month, add a row computing the fast and slow reading for each basket: sum the price changes
   over one month and over twelve months.
3. Add a rank based on the absolute value of each reading, from 1 to 5, and a weight equal to rank divided by
   15, carrying the sign of the reading.
4. Add a blended weight row equal to `0.75 * fast + 0.25 * slow`, and compute the package's return for the
   next month as the sum of weight times return.
5. Build a second block with the package's return and the index's return for the last twelve months, and
   compute twelve averages for each side, of lengths 1 to 12 months.
6. Count the points each side wins and compute the two shares.

What to notice: for most months the split stays fairly even, and the points swing only after a run of months
when one side has clearly done better. Also notice how much of the final result comes from the index when
the index side wins most of the windows. If you also track what the account would have earned by holding the
index alone, you will often find the two are close, which is the honest finding about blends that always
keep some market holding.

## Where this came from

- [The awesome-systematic-trading list](https://github.com/paperswithbacktest/awesome-systematic-trading), and
  its implementation file for
  [combining-smart-factors-momentum-and-market-portfolio](https://github.com/paperswithbacktest/awesome-systematic-trading/blob/main/static/strategies/combining-smart-factors-momentum-and-market-portfolio.py),
  which states the five baskets, the rank weights, the blend and the moving-average split.
- [Quantpedia: Combining Smart Factors Momentum and Market Portfolio](https://quantpedia.com/strategies/combining-smart-factors-momentum-and-market-portfolio),
  the rules and the extracted performance figures.
- Matus Padysak, [The active vs passive: smart factors, market portfolio or both?](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=3745517),
  the source paper, including the dollar-growth comparison and the correlation finding.
- [strategies/books2/10_portfolio_and_allocation.md](../../../strategies/books2/10_portfolio_and_allocation.md),
  this repository's brief on factor models, estimator error and what survives out of sample.

## Words used in this tutorial

- basis point: one hundredth of one percent, so ten basis points is 0.10 percent.
- factor: a single measurable characteristic, such as cheapness or size, used to sort shares.
- index fund: a fund that aims to copy a market index rather than to beat it.
- long: owning something, so you gain if its price rises.
- moving average: the average of the most recent fixed number of readings, recalculated as new readings
  arrive.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- volatility: how much a price moves around its average, measured as a percentage per year.
- weight: the share of your money placed in one holding.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
