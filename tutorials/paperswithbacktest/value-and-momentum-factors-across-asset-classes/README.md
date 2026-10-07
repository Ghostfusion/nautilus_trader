# Value and momentum across asset classes: buying markets that are cheap and rising

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                                                                                                                                   |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Twelve broad markets: American large and mid shares, American property shares, British, Japanese and emerging-market shares, American government, investment-grade and high-yield bonds, German and Japanese government bonds, and American cash, each through one fund or futures contract                                                                                                                             |
| How often it trades       | Once a month, when the whole list is rebuilt                                                                                                                                                                                                                                                                                                                                                                            |
| What you need             | A spreadsheet and, for each market, a monthly price, plus an earnings multiple or a yield                                                                                                                                                                                                                                                                                                                               |
| Where the rules come from | [The list's implementation file](https://github.com/paperswithbacktest/awesome-systematic-trading/blob/main/static/strategies/value-and-momentum-factors-across-asset-classes.py), which restates [the Quantpedia entry](https://quantpedia.com/strategies/value-and-momentum-factors-across-asset-classes/)                                                                                                            |
| The underlying research   | Blitz and van Vliet, [Global Tactical Cross-Asset Allocation](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1079975), and Asness, Moskowitz and Pedersen, [Value and Momentum Everywhere](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1363476)                                                                                                                                                            |
| How well it held up       | Strong: two independent academic studies covering many markets, with an out-of-sample period and costs the authors say the effect clears, although the tradable version here holds only a handful of instruments                                                                                                                                                                                                        |
| Also appears in           | [Value effect within countries](../../../tutorials/quantconnect/value-effect-within-countries/README.md), [Momentum effect in country equity indexes](../../../tutorials/quantconnect/momentum-effect-in-country-equity-indexes/README.md), [Asset class momentum](../../../tutorials/quantconnect/asset-class-momentum/README.md) and [Forex carry trade](../../../tutorials/quantconnect/forex-carry-trade/README.md) |

## The idea in one paragraph

Take a short list of very different markets: shares in several regions, property, government bonds, company
bonds and cash. For each, work out how expensive it is, meaning how much income you get for the price, and
how strong it has been over the past month and the past year. Score every market by blending those readings,
buying the cheap and strong ones and selling short the expensive and weak ones. Rebuild the list once a
month, betting that the cheapness and strength patterns seen inside one market also appear across very
different ones.

## Why anyone believed it

Two separate patterns are being used at once. The value pattern says that when something is priced to
deliver a lot of income for the money, its price tends to catch up over the following years. The momentum
pattern says that when something has been rising, it tends to keep rising for a while. The two are useful
together because they tend to fail at different times: cheap things can stay cheap for a long while, and
things that are rising can suddenly reverse.

The counterparty differs for each half. Value needs someone forced to sell a cheap market because it looks
bad, or unwilling to hold it while the crowd ignores it; momentum needs the late buyer and the early profit
taker. Too little money bets across whole countries and asset classes, so the gaps between them persist.

## An everyday comparison

Think of shopping for the same household item in several shops on one street. One shop is having a
clearance sale, so the item is cheap there; another has been raising its price for months because it is the
fashionable place to buy. A shopper who buys only at the cheapest shop may wait a long time for the price to
normalise, while one who buys only at the rising shop arrives as the fashion fades.

## The rules, step by step

1. Choose the twelve markets and one tracking fund, exchange-traded fund or futures contract for each. The
   list uses American large shares, American mid shares, American property shares, British shares, Japanese
   shares, emerging-market shares, American investment-grade bonds, American high-yield bonds, American,
   German and Japanese government bonds, and American cash.
2. For each market, compute the twelve-month return: today's price divided by the price twelve months ago,
   minus one.
3. Compute the one-month return the same way, using the price one month ago.
4. Compute the valuation number. For share markets this is the earnings yield, which is the company earnings
   for the past year divided by the price, expressed as a percentage; if you have a price-to-earnings
   multiple instead, divide one by it. For bond markets this is the yield to maturity, the total annual
   return you would earn by holding the bond to the end and reinvesting its coupons. Cash uses its interest
   rate. Property shares use their dividend yield.
5. Adjust each valuation number so the markets are comparable, by subtracting the fixed amounts the source
   study uses: 1 percentage point from American, Japanese and German government bonds, 2 points from
   American investment-grade bonds, 6 points from American high-yield bonds, 1 point from emerging-market
   shares and 2 points from property shares. The idea is to remove the part of the yield that is
   compensation for expected default or inflation rather than cheapness.
6. Rank the markets three times, once for each reading. The best market on a reading gets the highest rank
   number, and with twelve markets the ranks run from 12 down to 1: the strongest twelve-month performance
   gets 12, the weakest gets 1. Do the same for the one-month performance and for the valuation number,
   where the cheapest gets 12.
7. Score each market by blending the ranks: one quarter of the twelve-month rank, one quarter of the
   one-month rank, and one half of the valuation rank. Half the weight sits on cheapness because the source
   paper weights the value signal the most.
8. Sort the markets by score. With twelve markets the top three form the long side and the bottom three form
   the short side. Give each market on a side an equal share: one third of that side's money each, so the
   long side holds one unit of the account and the short side holds minus one unit.
9. Rebuild at the start of every month, selling what has dropped out of the long side and buying whatever
   has entered it, and doing the same in reverse on the short side.
10. Pay the costs. Trading costs are roughly 0.05 to 0.15 percent of the traded amount per side for liquid
    funds, and anything sold short also pays a borrow fee, often around 0.5 percent a year on the value
    borrowed.

## The maths, with every symbol named

The two momentum readings:

```text
R_12 = P_today / P_twelve_months_ago - 1
R_1  = P_today / P_one_month_ago - 1
```

- `R_12` is the twelve-month return, written as a decimal: 0.14 means 14 percent.
- `R_1` is the one-month return.
- `P_today`, `P_twelve_months_ago` and `P_one_month_ago` are the market's prices on those dates.

The valuation number, and its adjustment:

```text
V = yield + a
```

- `V` is the adjusted valuation number, in percentage points. A larger `V` means cheaper.
- `yield` is the earnings yield for shares, the yield to maturity for bonds, or the dividend yield for
  property shares, in percentage points.
- `a` is the fixed adjustment in percentage points from step 5, for example -6 for American high-yield
  bonds.

The blended score:

```text
Score = 0.25 * rank_12 + 0.25 * rank_1 + 0.50 * rank_V
```

- `rank_12`, `rank_1` and `rank_V` are the three rank numbers, each between 1 and the number of markets, so
  the best market scores highest and the ranking is the same direction on all three.
- The weights 0.25, 0.25 and 0.50 add to 1, so the score is on the same scale as a single rank.

The portfolio return over the next month:

```text
R_portfolio = average of the long markets' returns - average of the short markets' returns
```

- "average of the long markets' returns" is the mean next-month return of the markets held, because each
  long holding has the same weight.
- "average of the short markets' returns" is the mean next-month return of the markets sold short; it is
  subtracted because a short position gains what the shorted market loses.

The cost of rebuilding:

```text
Cost = t * c + b_short
```

- `t` is the traded fraction of the account, counting both the sale and the purchase, so replacing half the
  long side and half the short side gives `t` of 2.0.
- `c` is the cost per side as a fraction of the amount traded, around 0.001 for 10 basis points, where one
  basis point is one hundredth of one percent.
- `b_short` is the borrow fee over the month on the value held short, for example 0.5 percent a year divided
  by 12.

## A worked example

Eight markets are used to keep the arithmetic short. The numbers are invented but of ordinary size. The
ranks run from 8 (best) down to 1 (worst) on each reading.

| Market          | 1-month | 12-month | Valuation | rank 1m | rank 12m | rank value |
| --------------- | ------- | -------- | --------- | ------- | -------- | ---------- |
| US large shares | +2.0%   | +14%     | 4.5%      | 6       | 7        | 6          |
| US property     | +1.0%   | +6%      | 3.8%      | 5       | 5        | 4          |
| Japan shares    | +3.0%   | +20%     | 6.0%      | 7       | 8        | 7          |
| Emerging shares | +4.0%   | +8%      | 7.5%      | 8       | 6        | 8          |
| US gov bonds    | -0.5%   | -2%      | 3.3%      | 2       | 2        | 3          |
| US high yield   | +0.5%   | +4.2%    | 1.0%      | 4       | 3        | 1          |
| Germany bonds   | -1.0%   | -4%      | 1.5%      | 1       | 1        | 2          |
| US cash         | +0.4%   | +4.5%    | 4.4%      | 3       | 4        | 5          |

Applying the blend, for example to emerging shares:

```text
Score = 0.25 * 8 + 0.25 * 6 + 0.50 * 8 = 2.00 + 1.50 + 4.00 = 7.50
```

| Market          | Score |
| --------------- | ----- |
| Emerging shares | 7.50  |
| Japan shares    | 7.25  |
| US large shares | 6.25  |
| US property     | 4.50  |
| US cash         | 4.25  |
| US gov bonds    | 2.50  |
| US high yield   | 2.25  |
| Germany bonds   | 1.50  |

With eight markets the top quarter is two markets, so the long side is emerging shares and Japan shares, and
the short side is Germany bonds and US high yield. Each side holds one unit of the account, split equally
between its two markets.

Suppose the next month brings these returns:

| Position        | Weight within its side | Next-month return | Contribution to the total |
| --------------- | ---------------------- | ----------------- | ------------------------- |
| Emerging shares | 0.50 long              | +3.0%             | +1.50%                    |
| Japan shares    | 0.50 long              | +1.0%             | +0.50%                    |
| Germany bonds   | 0.50 short             | -1.0%             | +0.50%                    |
| US high yield   | 0.50 short             | -0.5%             | +0.25%                    |
| Total           |                        |                   | +2.75%                    |

The two short positions each gained half a percent of the account because the shorted markets fell. Now the
costs. Suppose one market on each side is replaced, so half of each side is sold and half bought: the traded
fraction `t` is 2.0. At 10 basis points per side and a 0.5 percent annual borrow fee on the short unit:

```text
Cost = 2.0 * 0.001 = 0.002, that is 0.20 percent
Borrow = 0.005 / 12 = 0.000417, that is 0.042 percent of the account
Net return = 2.75 - 0.20 - 0.04 = 2.51 percent
```

Over six months, a run of results could look like this. The numbers are invented; the point is the
arithmetic and the size of the cost line.

| Month | Long side | Short side | Gross | Trades | Cost  | Borrow | Net    |
| ----- | --------- | ---------- | ----- | ------ | ----- | ------ | ------ |
| 1     | +2.0%     | -1.0%      | +3.0% | 2.0    | 0.20% | 0.04%  | +2.76% |
| 2     | -1.5%     | +0.5%      | -2.0% | 2.0    | 0.20% | 0.04%  | -2.24% |
| 3     | +1.0%     | -2.0%      | +3.0% | 2.0    | 0.20% | 0.04%  | +2.76% |
| 4     | +0.5%     | -0.5%      | +1.0% | 2.0    | 0.20% | 0.04%  | +0.76% |
| 5     | +2.5%     | -1.5%      | +4.0% | 2.0    | 0.20% | 0.04%  | +3.76% |
| 6     | -0.5%     | +0.5%      | -1.0% | 2.0    | 0.20% | 0.04%  | -1.24% |

The six months total 6.56 percent, about 1.1 percent a month. The cost line alone is 0.24 percent a month,
roughly 2.9 percent a year, which is most of a modest edge.

## What the research actually found

| Source                                                          | What it measured                                                                                                             | Result                                                                                                                                                                                                                                |
| --------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Blitz and van Vliet, Global Tactical Cross-Asset Allocation     | Twelve asset classes, 1986 to 2007, long the top quarter and short the bottom quarter on a blended momentum and value signal | More than 9 percent a year, and Quantpedia's page reports 11.9 percent a year with volatility 10 percent and a reward-to-risk of 0.79 (as extracted on that page)                                                                     |
| Asness, Moskowitz and Pedersen, Value and Momentum Everywhere   | Individual shares, country equity indices, government bonds, currencies and commodities                                      | Value and momentum both produce returns in every asset class; the two are negatively correlated within and across asset classes, which is why combining them helps                                                                    |
| The awesome-systematic-trading list, its own replication record | 4,843 coded papers; aggregate statistics only                                                                                | The median strategy returns a reward-to-risk of 0.37 and 48 percent clear a statistical significance bar of 1.96, with a median test window of 34 years; this is the list's own aggregate measurement, not a figure for this strategy |

Read together: the cross-asset version of value and momentum has broader support than most single-market
strategies, because two independent studies reached it on different samples. The list publishes no
reward-to-risk number for this strategy on its own. The practical weakness is breadth: a quartile long and
quartile short out of twelve markets is three positions a side, so any single market can dominate a month.

## How this project relates to it

This repository studies the same two ideas, split by market.
[Value effect within countries](../../../tutorials/quantconnect/value-effect-within-countries/README.md)
tests cheapness across national share markets, and
[Momentum effect in country equity indexes](../../../tutorials/quantconnect/momentum-effect-in-country-equity-indexes/README.md)
tests strength across the same kind of list.
[Asset class momentum](../../../tutorials/quantconnect/asset-class-momentum/README.md)
extends strength to a mixed list of markets, which is the closest thing here to this strategy's universe.
[Forex carry trade](../../../tutorials/quantconnect/forex-carry-trade/README.md) is the same cheapness idea
applied to currencies, where the valuation number is the interest rate difference.

The broader question of whether a cross-sectional predictor survives when it is used to allocate is covered
in [the allocation brief](../../../strategies/books2/10_portfolio_and_allocation.md),
whose section on what actually allocates reports that most published predictors replicate but that weak
correlations between them mean ten thousand signals are not ten thousand independent bets.

## Where it goes wrong

- Twelve markets is a thin cross-section. Three long and three short positions give one market a large say
  in the month's result, and one surprise, such as an interest-rate decision, can dominate everything.
- Valuation numbers are not comparable by construction. An earnings yield, a bond yield and a cash rate mean
  different things, and the fixed adjustments in step 5 are the source paper's judgement rather than a
  universal rule. Change them and the ranking changes.
- Shorting bonds and property is not free or easy. Borrow fees vary, some instruments are hard to borrow,
  and the short side is where the costs and the squeeze risk live.
- Momentum and value can both fail together. The claim that they are negatively correlated is an average,
  not a promise, and there are periods when cheap markets fall and expensive rising markets keep rising.
- The ranking changes every month, so costs recur. A rule that rebalances monthly pays the gap between the
  buying and selling price twelve times a year, and the worked example shows that this line is large.
- The implementation's valuation data stops in 2019, after which the last known value is carried forward.
  That means the value half of the signal becomes stale in the later part of any backtest, which is a
  measurement defect rather than a property of the idea.

## Try it yourself

You need a spreadsheet and monthly prices for a handful of markets, plus one yield for each. Ten markets is
enough to see the mechanics.

1. Build a sheet with one row per market per month and columns `Market`, `Date`, `Price`, `EarningsYield`,
   `YieldAdjustment`.
2. Add a column `Valuation` equal to `EarningsYield + YieldAdjustment`.
3. For each month, add three rank columns using a rank function on the market list: rank of the one-month
   return, rank of the twelve-month return, rank of `Valuation`, all with the best market scoring highest.
4. Add a column `Score` equal to `0.25 * Rank1m + 0.25 * Rank12m + 0.5 * RankValue`.
5. Highlight the top quarter and the bottom quarter of markets each month. Those are the long and short
   lists.
6. In the next row of the sheet, compute the average next-month return of the long list minus the average
   next-month return of the short list, then subtract 0.20 percent for a full turnover.

What to notice: on many months the same markets stay near the top, so the long list barely changes and the
cost is small; on others the whole list rotates. Compare the twelve-month momentum ranks with the value
ranks and see how often they point in opposite directions. When they disagree, the score is doing its
blending job, and the result tends to be a smaller move in either direction.

## Where this came from

- [The awesome-systematic-trading list](https://github.com/paperswithbacktest/awesome-systematic-trading), and
  its implementation file for
  [value-and-momentum-factors-across-asset-classes](https://github.com/paperswithbacktest/awesome-systematic-trading/blob/main/static/strategies/value-and-momentum-factors-across-asset-classes.py),
  which states the universe, the adjustments, the blend weights and the monthly rebalancing.
- [Quantpedia: Value and Momentum Factors across Asset Classes](https://quantpedia.com/strategies/value-and-momentum-factors-across-asset-classes),
  the rules and the extracted performance figures.
- Blitz and van Vliet, [Global Tactical Cross-Asset Allocation](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1079975),
  the source paper for the twelve-market universe and the blended signal.
- Asness, Moskowitz and Pedersen, [Value and Momentum Everywhere](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1363476),
  the paper that established that value and momentum work across asset classes and are negatively correlated.
- [strategies/books2/10_portfolio_and_allocation.md](../../../strategies/books2/10_portfolio_and_allocation.md),
  this repository's brief on how much of a cross-sectional signal survives when it is used to allocate.

## Words used in this tutorial

- basis point: one hundredth of one percent, so ten basis points is 0.10 percent.
- borrow fee: the charge for borrowing something you do not own in order to sell it short.
- long: owning something, so you gain if its price rises.
- momentum: the tendency of something that has been rising to keep rising for a while.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- yield to maturity: the total annual return from holding a bond to the end, including its coupons.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
