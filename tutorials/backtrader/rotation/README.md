# Rotation: hold whatever has been strongest, and step aside when nothing is

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                             |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | A handful of broad holdings - gold, silver, a share index fund, a government bond fund, a commodity fund, and the Japanese yen or Swiss franc as defensive currencies - held one or two at a time                 |
| How often it trades       | About once a month, when the ranking is recomputed                                                                                                                                                                |
| What you need             | A spreadsheet and two or more years of monthly prices for a few funds                                                                                                                                             |
| Where the rules come from | [The compendium's rotation article](https://backtrader.readthedocs.io/en/latest/strategies-series/en/23-rotation.html)                                                                                            |
| The underlying research   | Moskowitz, Ooi and Pedersen, [Time Series Momentum](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2089463), and Antonacci, [Absolute Momentum](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2244633) |
| How well it held up       | Mixed: momentum across assets is one of the better documented patterns, but the reward shrinks once turnover, costs and the number of rules tried are counted                                                     |
| Also appears in           | [Asset class momentum](../../quantconnect/asset-class-momentum/README.md) and [Momentum and style rotation effect](../../quantconnect/momentum-and-style-rotation-effect/README.md) in this collection            |

## The idea in one paragraph

Imagine a shelf with four or five different things you can own: gold, shares, bonds, commodities, and
a defensive currency. This family of rules looks at the shelf once a month and asks a simple
question: which of these has gone up the most over the last few months? It puts the money into the
best one, or the best two. That is the "relative" part - the best performer is judged against the
others on the shelf, not against a fixed target. The "absolute" part is a second question: has the
winner actually risen, or is it just the least bad of a falling set? If nothing has genuinely risen,
the money goes to a safe place instead. The bet is that a thing that has been strong keeps being
strong for a while, and that stepping aside when everything falls avoids the worst of the damage.

## Why anyone believed it

Prices do not absorb news all at once. When demand for copper rises, the metal's price moves over
weeks, and the companies and currencies tied to copper move with it. Investors who hear the news
late buy after the first move and push it further, and funds measured against their peers are loath
to be the only ones not holding whatever is working. Meanwhile the people on the other side of the
trade are often selling for reasons that have nothing to do with the outlook: a fund meeting
withdrawals, a manager trimming a holding that grew too large, or a trader taking a profit simply
because the price went up. If those sellers keep appearing, the recent winner keeps winning a little
longer, which is what the ranking rule tries to catch.

## An everyday comparison

Think of a school football team picked each week. The coach does not know who will play well next
week, but the players who have scored in the last few games are a better guess than the ones who
have not. So the coach picks whoever has been playing well lately, and keeps picking them while they
stay in form. The coach also has a rule for the day when the whole squad is out of form: play
defensively rather than pick the best of a bad bunch. Nothing here guarantees a win; it is a way of
tilting the odds using no more information than the recent record.

## The rules, step by step

Every file in this category shares one skeleton: line up several holdings on the same dates, rank
them periodically by how much they have risen, hold the leader or leaders, and keep a defensive
holding on standby for the months when nothing confirms.

1. Choose the shelf. Two to five holdings, aligned so that each has a price on the same days. One
   variant uses a single holding compared with its own past.
2. Define the look-back. Most files use the return over the past 63 trading days, the past 126, or a
   blend of both. A return is the percentage change in price over a period.
3. On the chosen date - usually the end of a month, or every twenty-one trading days - compute each
   holding's return over the look-back.
4. Rank the holdings from the strongest to the weakest. Give the strongest rank one.
5. Decide what to hold. Most files hold the top one or top two; one file holds a set fixed in
   advance with a cap on any single holding.
6. Apply the absolute check. Ask whether the chosen holding has risen over the look-back, or whether
   it sits above its own long average. If yes, hold it. If no, move the money to a safe holding - a
   government bond fund, or cash.
7. Hold until the next review, then repeat from step 3. Sell whatever has dropped out of the chosen
   set and buy whatever has taken its place, paying the cost of both sides.

The strategies in this category. Six files are grouped here. Each clause says in one line what the
file does.

| File                                           | Shelf                              | What it does in one clause                                                                                 |
| ---------------------------------------------- | ---------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| `test_0001_gold_asset_rotation`                | Gold, shares, bonds, commodities   | Ranks four assets by three-month return, holds the top two 70/30, flees to bonds if the winner is negative |
| `test_0002_safe_haven_rotation`                | Gold, silver, yen, franc, bonds    | Blends two momentum periods, holds the strongest safe haven that is also above its trend                   |
| `test_0003_timing_bond_rotation`               | One share fund and four bond funds | Holds shares above their 200-day average, otherwise the strongest bond                                     |
| `test_0004_monthly_rotation_ranking`           | A single asset                     | Ranks the asset's own return against its past year and holds when the rank is in the top half              |
| `test_0005_three_factor_etf_rotation_strategy` | Five country and asset funds       | Scores by three months of return, twenty days of return and twenty days of calm, holds the top three       |
| `test_0006_rotational_trading_strategy`        | Shares, bonds, gold, commodities   | Ranks four assets by six-month return, holds the top two with a 50% cap on each                            |

Two of these repay close reading.

The safe-haven rotation (`test_0002_safe_haven_rotation.py`). Five candidates: gold, silver, the
Japanese yen, the Swiss franc, and a bond fund as the fallback. For the two currencies the file
inverts the quoted price so that all five series mean "this thing is getting stronger". Each month
it computes each candidate's return over 63 days and over 126 days, ranks the five candidates on
each period, and adds the two ranks into a blend score. The lowest blend score wins. A candidate is
only taken, however, if it is also above its own 63-day simple moving average - the trend gate. If
the top-ranked candidate fails the gate, the rule looks at the next candidates in order; if none
confirms, the money goes to the bond fund. There is no "go long" and "go flat" here: the position is
always fully in one of the five, chosen monthly.

The monthly rotation ranking (`test_0004_monthly_rotation_ranking.py`). This one has no second asset
at all. It computes the asset's 63-day return, then ranks that number against the last 252 trading
days of the same number, giving a percentile from 0 to 1 - "is now stronger than at most times in
the past year?". Every twenty-one days it checks. If it holds nothing and the percentile is above
0.5, it buys. If it holds and the percentile falls below 0.3, it sells. Between 0.3 and 0.5 it does
nothing, which is the whole design: a buffer that stops the rule buying and selling on every small
wobble of the rank.

## The maths, with every symbol named

The return over a look-back, which is the ranking input:

```text
M = P_now / P_then - 1
```

- `M` is the momentum (the recent strength of the price), written as a decimal: 0.12 means twelve
  percent.
- `P_now` is the price today.
- `P_then` is the price `k` trading days ago, where `k` is the look-back, such as 63.

Rank the holdings by `M`, largest first, and give each a rank number where one is best. To hold the
top two in equal amounts:

```text
w_i = 1/2 for each of the two best holdings, and 0 for the rest
```

- `w_i` is the fraction of the pot in holding `i`, written as a decimal.

A blend of two look-backs, as in the safe-haven file, uses the sum of the rank numbers:

```text
Score_i = rank_i(63 days) + rank_i(126 days)
```

- `rank_i(63 days)` is where holding `i` placed on the 63-day return, from 1 (best) to `n` (worst).
- `rank_i(126 days)` is where it placed on the 126-day return.
- `n` is the number of candidates.
- The lowest `Score_i` is the winner, because a rank of one is best and two rank numbers are added.

The single-asset percentile rank is:

```text
R = (number of past readings of M that are at most today's M) / (number of past readings)
```

- `R` is a fraction from 0 to 1: 0.9 means today's reading is above 90 percent of the past year's
  readings.

The absolute check, and the switch to the safe holding:

```text
if M_winner < 0 or P_winner < average(P_winner over the last m days), hold the safe holding
```

- `M_winner` is the winner's return over the look-back.
- `average(...)` is the simple moving average, the plain average of the last `m` prices.

A moving average is a smoothed line through the price; the price being above it is the simplest
statement that the recent path is rising. Finally, the cost of a switch:

```text
Cost = (amount sold + amount bought) * c
```

- `c` is the cost per trade as a fraction of the amount traded, covering the gap between the buying
  and selling prices plus any commission. For large funds a figure of 0.0005 to 0.001 is realistic.

## A worked example

The numbers are invented but of the size these holdings really take. Costs are counted at 0.0005 of
the amount traded, on both sides.

First, a four-asset rotation holding the top two in equal amounts, checked at five month-ends. The
look-back returns are the numbers used to rank; the "next month" returns are what the holdings then
did.

| Month end | IVV | IEF | GLD | DBC | Chosen   | Returns next month | Portfolio move |
| --------- | --- | --- | --- | --- | -------- | ------------------ | -------------- |
| 1         | 8%  | 2%  | 12% | -3% | GLD, IVV | GLD +4%, IVV -2%   | +1.000%        |
| 2         | 6%  | 2%  | 14% | 3%  | GLD, IVV | GLD -6%, IVV +3%   | -1.500%        |
| 3         | 9%  | 1%  | 5%  | 4%  | IVV, GLD | IVV +2%, GLD -1%   | +0.500%        |
| 4         | 10% | 2%  | 3%  | 8%  | IVV, DBC | IVV +1%, DBC -4%   | -1.500%        |
| 5         | 11% | 3%  | 2%  | 1%  | IVV, IEF | IVV -3%, IEF +2%   | -0.500%        |

Start with 1,000.00. At month end 1 the rule buys GLD and IVV, so the whole pot is traded in one
step: 1,000.00 times 0.0005 is a cost of 0.50, leaving 999.50, which then gains 1.000 percent to
1,009.50. At month end 2 nothing changes, so there is no cost; the pot falls 1.500 percent to
994.36. At month end 3 nothing changes again, and the pot gains 0.500 percent to 999.33. At month
end 4 GLD drops out and DBC enters, so half the pot is sold and half bought, a traded amount of
1.00, a cost of 1.00 in total; after the cost and the -1.500 percent move the pot is 984.41. At
month end 5 DBC leaves and IEF enters, another full switch costing about 0.98, and after the -0.500
percent move the pot is 979.90. Over five months the rule turned 1,000.00 into 979.90. Switching
three times cost about 2.50 in total, small at five basis points but eight times larger at a
forty-basis-point gap between the buying and selling prices.

Second, the single-asset rank buffer. Suppose the percentile ranks over seven months are 0.20, 0.45,
0.62, 0.71, 0.55, 0.28, 0.40. Walk the rule through them.

| Month | Rank | Holdings after this month           |
| ----- | ---- | ----------------------------------- |
| 1     | 0.20 | flat, because rank is not above 0.5 |
| 2     | 0.45 | flat                                |
| 3     | 0.62 | hold, because rank rose above 0.5   |
| 4     | 0.71 | hold                                |
| 5     | 0.55 | hold                                |
| 6     | 0.28 | flat, because rank fell below 0.3   |
| 7     | 0.40 | flat, because rank is not above 0.5 |

The interesting month is the last one. At 0.40 the rank is in the buffer zone, where a holder would
keep holding but a flat rule does not enter. That gap between 0.5 for buying and 0.3 for selling is
what stops the rule trading every time the rank crosses a single line.

## What the research actually found

Momentum is one of the most heavily studied patterns in finance. Moskowitz, Ooi and Pedersen
documented in 2012 that many futures markets, across asset classes, tend to keep moving in the
direction they have been moving over the following one to twelve months, and that this held across
58 instruments. Antonacci later packaged the same idea for individual investors as "relative
momentum selects the asset, absolute momentum decides whether to be invested at all", which is
almost the exact structure of the files here.

The results on the compendium's own data are mixed and are quoted from the category article. The
monthly rank-buffer rule, on eighteen years of gold, made 20 trades over 4,324 daily bars, of which
12 won and 7 lost, with a final value of 2,631,363.63 from 1,000,000. The safe-haven rotation, over
2008 to 2025, rebalanced 123 times but completed only 3 round-trip trades, all of them winners - a
small number that says more about how long it held each safe haven than about skill. The plain
cross-asset rotation rebalanced 216 times over 4,518 bars. The article's honest summary is that
momentum's low turnover is visible, and that most of the difference between these files is seasoning
on one plain dish.

One thing must be said plainly, because it applies to every file in this category and to every
other backtest in this compendium. Each backtest asserts its final value, its reward-to-risk ratio
and its worst fall against a stored baseline, and it must produce identical numbers in the engine's
two calculation modes. The reward-to-risk ratio is the average return divided by how much the pot
swung, and the worst fall is the largest drop from a peak to the following low. Passing those
assertions proves the engine computes exactly what the file says it computes, on the stored data.
It does not prove the strategy earns anything. The assertion is a test of the software, not a claim
about the market. A file can pass every assertion and still describe a plan that loses money.

## How this project relates to it

This repository contains its own study of the general question, harvested from the research
literature: [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md). It
separates three things that are often confused - changing exposure because you believe you can
predict relative returns, choosing a fixed allocation as a policy, and simply rebalancing back to
target - and concludes that under genuinely memoryless returns no directional selection rule has an
edge, while rebalancing and allocation still matter. Its sharpest number is that of 1,022 possible
rotation rules tested on American sector data, the average rule returned 0.86 percent a month
against 0.89 percent for simply holding the market, and the authors concluded the handful of winners
were the result of trying so many rules.

The second related piece is the engine in
[implementation/sector-regime-engine](../../../implementation/sector-regime-engine/README.md). It
does not run a rotation rule. It measures whether the market is in a state where such a rule is
eligible at all, and returns one of four verdicts with the evidence for it. Its
[user manual](../../../implementation/sector-regime-engine/USER_MANUAL.md) states plainly that the
app contains no price feed and makes no recommendation.

## Where it goes wrong

- Momentum crashes. After a market-wide fall the assets that led the previous ranking are often the
  ones that fell furthest, and they can bounce hardest or keep falling. A rule that bought them just
  before the fall loses on both legs.
- Crowding. Once a ranking rule is published and easy to trade through funds, more money runs it.
  The buying happens earlier, the returns arrive sooner and smaller, and latecomers pay for the
  reversal.
- The number of rules tried. There are many ways to define momentum: which look-back, whether to
  skip the most recent month, how many holdings, how often to review. Choosing the best of these
  after seeing the results is how the 1,022-rule experiment produced its conclusion.
- Costs on a monthly rebuild. Every switch pays the gap between buying and selling prices on both
  sides. The more often the ranking changes, the more the rule pays, and a rule that rarely changes
  its mind is a different rule.
- A short look-back can invert. Over days and weeks, prices tend to reverse rather than continue, so
  a rule built on one month of returns can lose even though a twelve-month version earned.
- The safe haven is a choice made with hindsight. Gold, the yen and the franc look defensive because
  of the last few decades; there is no guarantee the same holdings are defensive in the next
  crisis.

## Try it yourself

You need a spreadsheet and monthly prices for three funds - an equity index fund, a bond fund and a
gold fund - for the last ten years.

1. One row per month, one column per fund, one column for each fund's twelve-month return (this
   month's price divided by the price twelve rows up, minus one).
2. Add a column naming the fund with the highest twelve-month return in that row.
3. Add a column that on the next row computes that fund's return for the following month; that is the
   rule's monthly return.
4. Do the same for a rule that holds the top two funds in equal amounts.
5. Beside both, add a column for simply holding the equity fund alone.
6. Finally, subtract 0.0005 for every position that changed from the previous month.

What to notice: on many months the name in step 2 is the same as the previous month, so nothing
trades and the cost is near zero; on some months it changes. Over ten years the rule will usually
sit close to the equity fund, sometimes ahead and sometimes behind. If it wins by a wide margin,
check whether the fund list is one you chose after seeing which funds did well, which is the
look-ahead trap the sector study warns about.

## Where this came from

- [The compendium's rotation article](https://backtrader.readthedocs.io/en/latest/strategies-series/en/23-rotation.html),
  the rules, the inventory and the file-level results quoted above.
- Moskowitz, Ooi and Pedersen, [Time Series Momentum](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2089463),
  the study of momentum across 58 futures markets.
- Antonacci, [Absolute Momentum](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2244633), the
  framework that combines relative selection with an absolute gate.
- [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md), this
  repository's own study, including the 1,022-rule experiment.

## Words used in this tutorial

- momentum: the tendency of something that has been rising to keep rising for a while.
- look-back: the number of past days or months used to measure a return.
- moving average: the plain average of the last `m` prices, a smoothed line through the price.
- percentile rank: where a number sits within a set of past numbers, from 0 to 1.
- safe haven: a holding expected to hold its value, or rise, when riskier things fall.
- turnover: how much of the pot is traded over a period.
- trend: a persistent direction in a price, up or down.
- weight: the share of your money placed in one holding.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
