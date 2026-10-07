# Consistent momentum: buying only the winners that keep winning

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                      |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| What it trades            | Shares of American companies in both directions: the winners that were near the top in two overlapping windows, and the losers that were near the bottom in both                                                                                                                           |
| How often it trades       | The screen is run monthly, but a selection is held for six months without being touched, so most months nothing moves                                                                                                                                                                      |
| What you need             | A spreadsheet and about seven months of monthly prices for a list of shares                                                                                                                                                                                                                |
| Where the rules come from | [The list's implementation file](https://github.com/paperswithbacktest/awesome-systematic-trading/blob/main/static/strategies/consistent-momentum-strategy.py) and the [Quantpedia entry](https://quantpedia.com/strategies/consistent-momentum-strategy/) it repeats                      |
| The underlying research   | Chen, Chou and Hsieh, [Persistency of the Momentum Effect: The Role of Consistent Winners and Losers](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2652592)                                                                                                                         |
| How well it held up       | Mixed: the paper reports the consistent version beating ordinary momentum, and the list's own run returned a Sharpe ratio of 0.63 over 1990 to 2026, but that run carried 31 percent volatility and a 66 percent worst fall, and it is one measured sample rather than an independent test |
| Also appears in           | [Momentum in stocks](../../quantconnect/momentum-effect-in-stocks/README.md) and [Combining momentum with volume](../../quantconnect/combining-momentum-effect-with-volume/README.md) in this collection                                                                                   |

## The idea in one paragraph

Ordinary momentum buys the shares that rose the most over the past several months and sells short
the ones that fell the most, on the theory that the move continues. Its weakness is that a share can
jump once and otherwise do nothing, and a single jump is more likely to be given back than a rise
built from many small steps. This strategy keeps only the shares that climbed steadily: it measures
each share over two six-month windows shifted by a month, and buys only the shares that sit in the
top tenth on both measures. It sells short the shares that sit in the bottom tenth on both, holds
them for six months, and does not touch the book in between.

## Why anyone believed it

Momentum is meant to exist because investors under-react to news: the price adjusts slowly instead
of all at once, so a share that has been rising keeps rising for a while. But not every jump is
information arriving slowly. A single large move can come from a one-off event, or from an
overreaction, and those moves tend to be given back. A share that scores at the top in two windows
that overlap by five months has, almost by construction, risen in many of those months rather than
in one. The researchers argue that this steadiness is a sign of gradual, repeated good news, and
that the under-reaction has more time to work itself out, so the climb continues.

The counterparty is the investor who is slow to update, and the paper's own evidence points to two
sources: shares with more information asymmetry, where news reaches some people before others, and
shares with more disagreement among investors about what the news is worth. On both, the price
adjusts slowly, and the consistent winner is the name where that slow adjustment is still running.
The flip side, the consistent loser, is the share that keeps drifting down because bad news keeps
arriving gradually.

## An everyday comparison

Think of two students who both score top of the class. One aced a single test after being handed a
topic they happened to know well; the other has been near the top of every test for a term. If you
had to bet on next term's ranking, the steady one is the safer bet, because a single good result can
be luck while a run of them is hard to fake. The strategy here is the examiner who admits only the
steady students, and who also identifies the steady underperformers to bet against.

## The rules, step by step

1. Assemble a list of shares traded on the large American exchanges, each with at least seven months
   of price history. The code takes the 500 most heavily traded and skips property trusts.
2. At the end of each month, compute two returns for every share. The first is the price one month
   ago divided by the price seven months ago, minus one: the six months that ended a month ago. The
   second is today's price divided by the price six months ago, minus one: the six months that end
   today. The two windows overlap by five months.
3. Rank the shares by each measure separately, best first. Call the top tenth by the first measure
   the fast winners and the top tenth by the second the late winners.
4. A consistent winner is a share that is in the top tenth on both measures. A consistent loser is a
   share in the bottom tenth on both. A share that is a winner on one window and a loser on the other
   is discarded.
5. Buy the consistent winners in equal amounts and sell short the consistent losers in equal amounts,
   giving each leg the same total money.
6. Wait one month after forming the list, then hold the basket for six months without rebalancing.
   Do not add or remove shares in between.
7. Before the six months are up, form the next list and repeat, so that a new basket is opened at
   regular intervals.
8. Pay the gap between buying and selling prices on entry, the same again on exit, and a borrow fee
   on the shorted shares for the whole holding period.

## The maths, with every symbol named

The two scores, both measured at the end of month `t`:

```text
R1_i = P_i,t-1 / P_i,t-7 - 1
R2_i = P_i,t   / P_i,t-6 - 1
```

- `P_i,t` is the closing price of share `i` at the end of month `t`.
- `R1_i` is the six-month return that ended one month ago, so the most recent month is left out.
- `R2_i` is the six-month return that ends today, so it includes the most recent month.
- The two windows share five of their six months, which is what makes a share that did well in both a
  steady rather than a lucky performer.

The two baskets are chosen from the ranks:

```text
Winners = { i : R1_i is in the top 10 percent and R2_i is in the top 10 percent }
Losers  = { i : R1_i is in the bottom 10 percent and R2_i is in the bottom 10 percent }
```

- `Winners` and `Losers` are sets of shares, not numbers.
- A share must clear both bars to qualify; being best on only one window is not enough.

The return over the holding period:

```text
R_long  = (1 / n_L) * sum over Winners of r_i
R_short = (1 / n_S) * sum over Losers  of r_i
R_strategy = R_long - R_short
```

- `r_i` is the share's return over the six-month holding period, as a decimal.
- `n_L` and `n_S` are the counts of the two sets, so each leg is equal-weighted.
- `R_strategy` is stated per unit of one leg, and the two legs are equal in money.

The cost, charged once on entry and once on exit because the book is not rebalanced:

```text
Cost_entry = 2 * c
Cost_exit  = 2 * c
Borrow     = s * months
```

- `c` is the cost of one trade as a fraction of its value, about 0.001, ten basis points, covering
  the gap between buying and selling prices. One basis point is one hundredth of one percent.
- The factor two counts both legs: buying the winners and selling short the losers.
- `s` is the borrow fee per month on the shorted shares, and `months` is the holding length.

## A worked example

Ten invented shares, each with a price seven months ago, six months ago, one month ago and now.

| Share | 7 months ago | 6 months ago | 1 month ago | Now | R1      | R2      | Neighbours |
| ----- | ------------ | ------------ | ----------- | --- | ------- | ------- | ---------- |
| S1    | 100          | 105          | 120         | 130 | 0.2000  | 0.2381  | winner     |
| S2    | 100          | 98           | 90          | 88  | -0.1000 | -0.1020 |            |
| S3    | 50           | 52           | 58          | 62  | 0.1600  | 0.1923  |            |
| S4    | 80           | 84           | 98          | 110 | 0.2250  | 0.3095  | winner     |
| S5    | 60           | 58           | 52          | 48  | -0.1333 | -0.1724 | loser      |
| S6    | 40           | 41           | 45          | 44  | 0.1250  | 0.0732  |            |
| S7    | 70           | 68           | 62          | 58  | -0.1143 | -0.1471 | loser      |
| S8    | 90           | 95           | 104         | 100 | 0.1556  | 0.0526  |            |
| S9    | 30           | 32           | 36          | 34  | 0.2000  | 0.0625  |            |
| S10   | 120          | 118          | 110         | 104 | -0.0833 | -0.1186 |            |

For S4 the first score is `98 / 80 - 1 = 0.2250` and the second is `110 / 84 - 1 = 0.3095`. The
published rule takes the top tenth; with ten shares that is one share, so the example takes the best
two to make the table less fragile. S4 and S1 are the best two on both scores, so they are the
consistent winners. S9 is tied with S1 on the first score but ranks seventh on the second, so it is
discarded, which is exactly the filter doing its work. S5 and S7 are worst on both, so they are the
consistent losers.

| Month | Winner basket | Loser basket | Gross    | Cost    | Net      | Cumulative |
| ----- | ------------- | ------------ | -------- | ------- | -------- | ---------- |
| 1     | +2.5000%      | -1.5000%     | +4.0000% | 0.2417% | +3.7583% | +3.7583%   |
| 2     | +0.0000%      | -0.5000%     | +0.5000% | 0.0417% | +0.4583% | +4.2339%   |
| 3     | +0.0000%      | -1.2500%     | +1.2500% | 0.0417% | +1.2083% | +5.4934%   |
| 4     | +1.7500%      | +0.0000%     | +1.7500% | 0.0417% | +1.7083% | +7.2956%   |
| 5     | -0.7500%      | +0.2500%     | -1.0000% | 0.0417% | -1.0417% | +6.1779%   |
| 6     | +0.0000%      | -0.2500%     | +0.2500% | 0.2417% | +0.0083% | +6.1867%   |

The two basket columns are the averages of the two chosen shares' returns in that month, and the
gross column is the winner average minus the loser average. The cost is two trades at the start of
0.1 percent each, `2 * 0.001 = 0.2` percent, plus a borrow fee of half a percent a year on the short
leg, `0.005 / 12 = 0.0417` percent a month; the final month adds another 0.2 percent for closing the
book. The example ends about plus 6.2 percent over six months, and month five shows the familiar
hazard: the losers bounced and the steady winners gave some back, so even a consistent basket is not
safe.

## What the research actually found

| Source and what it measured                               | Result                                                                                                                                                                                                                                       |
| --------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Chen, Chou and Hsieh, American shares, 1980 to 2011       | Consistent winners beat inconsistent winners, and consistent losers fall further than inconsistent losers; the strategy earned 1.25 percent a month against 1.06 percent for ordinary momentum and 0.47 percent for the inconsistent version |
| The same paper's explanation                              | The persistence is stronger where information is more unevenly spread and where investors disagree more, which the authors read as evidence of slow under-reaction                                                                           |
| Quantpedia's summary of the paper                         | 16.08 percent a year at 25.3 percent volatility, worst fall 59.3 percent, a Sharpe ratio of 0.48 over 1980 to 2011, graded Strong                                                                                                            |
| The list's own measurement, on its own data, 1990 to 2026 | A Sharpe ratio of 0.63, annual return 16.00 percent, volatility 31.23 percent, worst fall 66.4 percent (the vendor's own measurement)                                                                                                        |

The two measured windows are different and their numbers are not directly comparable, but they point
the same way: the list's own run earns a little more per year than the paper reports and takes a
great deal more volatility, which is why its Sharpe ratio of 0.63 is not far from the paper's 0.48.
The annual return sounds large, but so is the risk: a strategy with 31 percent volatility can lose a
third of its value in a bad stretch, and the worst fall recorded is two thirds. For scale, the list
reports that across thousands of papers the median replication has a Sharpe ratio of 0.37 and 48
percent clear a t-statistic of 1.96, so a figure near 0.6 is better than the median but still within
the range that one lucky sample can produce.

## How this project relates to it

This repository's own study of when leadership persists, and therefore when a momentum rule is even
eligible to be run, is [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md).
Its section 3.2 states that momentum needs positive serial dependence, meaning winners must keep
winning for several periods, which is precisely the property the consistency filter tries to
isolate.

The wider question of how momentum should be designed, and how much of a measured result is the
design rather than the signal, is covered in
[Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md).
That brief reads the recent research and reports that momentum returns are positively skewed by
construction and that the ranking rule is itself a design choice with measurable consequences. The
finished tutorial [Momentum in stocks](../../quantconnect/momentum-effect-in-stocks/README.md) works
the plain, unfiltered version of the same bet, so the two can be read side by side.

## Where it goes wrong

- The book is stale for six months. A share can turn from winner to loser long before the basket is
  rebuilt, and nothing in the rules reacts.
- The filter can pick a jumpy name. Overlapping windows share five months, so a share that jumped
  once early in the six months can still rank high on both, even though the intent was to avoid it.
- The intersection is small. Requiring the top tenth on two measures at once leaves few shares, so
  the basket is concentrated and a single bad name matters.
- The extremes are populated by small, hard-to-borrow shares. That makes the borrow fee high and the
  measured return sensitive to whether those shares were tradable.
- Turnover is lumpy. Most months nothing trades, then the whole book is replaced at once, so costs
  arrive in bursts and the strategy's capacity is limited by how much it can trade on one day.
- The measurement is fragile. A different start month, a different skip convention, or counting only
  surviving shares will all change the result, which is why a single Sharpe ratio is weak evidence.

## Try it yourself

You need a spreadsheet and monthly closing prices for a list of shares. Ten shares and a year of
data is enough to learn the mechanics.

1. Put one share in each row, and in four columns write its price one, six, seven and, if you have
   it, twelve months ago.
2. Add a column for the first score: the price one month ago divided by the price seven months ago,
   minus one.
3. Add a column for the second score: the current price divided by the price six months ago, minus
   one.
4. Sort the sheet by each score in turn and write down which shares are in the top two and bottom
   two on both scores.
5. Those two lists are what the rules would buy and short. Follow them for the next six months,
   adding each month's return to a running total for the winner basket and the loser basket.
6. Subtract the loser total from the winner total, then subtract about 0.4 percent for entry and exit
   costs and the borrow fee.

What to notice: several shares that were best on one score drop out on the other, and often no share
ever qualifies as both a top winner and a bottom loser, which is how the filter shrinks the list.
The result for ten shares over six months is noise, but the mechanics of the overlap are visible,
and that is the point of the exercise.

## Where this came from

- [The list's implementation of this strategy](https://github.com/paperswithbacktest/awesome-systematic-trading/blob/main/static/strategies/consistent-momentum-strategy.py),
  which states the two overlapping windows, the deciles and the six-month holding.
- [Quantpedia: consistent momentum strategy](https://quantpedia.com/strategies/consistent-momentum-strategy/),
  the performance figures, the confidence grade and the source-paper link.
- Chen, Chou and Hsieh, [Persistency of the Momentum Effect: The Role of Consistent Winners and Losers](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2652592),
  the paper behind the rule.
- The list's own measurement page at
  [paperswithbacktest.com/strategies/persistency-of-the-momentum-effect](https://paperswithbacktest.com/strategies/persistency-of-the-momentum-effect).
- [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md), this
  repository's study of when leadership persists.
- [Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  this repository's brief on momentum design and on how much a result depends on the rule.

## Words used in this tutorial

- decile: one tenth of a sorted list, so the top decile is the best ten percent.
- holding period: the length of time a position is kept open.
- momentum: the idea that what has recently risen tends to keep rising for a while, and the strategies that trade on it.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- turnover: how often a portfolio's holdings are replaced, which drives how much is paid in costs.
- volatility: how much a return moves around its average, usually quoted as a yearly percentage.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
