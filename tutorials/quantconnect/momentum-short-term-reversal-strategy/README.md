# Momentum with a deceleration filter: buying winners that have started to slow, and selling losers that have started to recover

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                                                                                           |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of American companies listed on the two large American exchanges, bought and sold short at the same time                                                                                                                                                                                                                                                                 |
| How often it trades       | Once a month, when the whole book of roughly thirty shares is rebuilt                                                                                                                                                                                                                                                                                                           |
| What you need             | A spreadsheet and thirteen monthly prices for a large list of shares                                                                                                                                                                                                                                                                                                            |
| Where the rules come from | [QuantConnect strategy library, momentum short term reversal strategy](https://www.quantconnect.com/tutorials/strategy-library/momentum-short-term-reversal-strategy) and the premium [Quantpedia entry](https://quantpedia.com/Screener/Details/51) it cites                                                                                                                   |
| The underlying research   | Jegadeesh and Titman, [Returns to Buying Winners and Selling Losers](https://onlinelibrary.wiley.com/doi/10.1111/j.1540-6261.1993.tb04702.x), for the momentum being harvested and De Bondt and Thaler, [Does the Stock Market Overreact?](https://onlinelibrary.wiley.com/doi/10.1111/j.1540-6261.1985.tb05004.x), for the reversal the deceleration filter is trying to avoid |
| How well it held up       | Mixed: the momentum premium it harvests is one of the best-replicated effects in the literature, but the deceleration filter that defines this particular version is published once, on one implementation, with no source paper named publicly, so the grade rests on the underlying premium rather than on the refinement                                                     |
| Also appears in           | [Sector momentum](../sector-momentum/README.md) and [Short term reversal](../short-term-reversal/README.md) in this collection, the two halves of this idea applied to whole sectors and to plain reversal                                                                                                                                                                      |

## The idea in one paragraph

A share that has risen a lot over the past year tends to keep rising a little longer, and a share
that has fallen a lot tends to keep falling: that is momentum. But the last stretch of a long move is
often the point at which the price has been pushed too far, and after that it turns back. This
strategy tries to buy only the winners whose rise has already started to slow down, and to sell short
only the losers whose fall has already started to slow, on the theory that a slowing move is nearer
its end than an accelerating one. It holds about fifteen shares on each side for a month, in equal
amounts, and rebuilds the list every month.

## Why anyone believed it

Momentum itself has a plausible story. News about a company arrives over weeks and months, not all at
once, and investors who are slow to update buy after the first move, which extends it. Funds that are
judged against their peers are reluctant to be the only one not holding what is working. On the other
side of the trade is the investor who sells for reasons unrelated to the outlook, or who takes
profits simply because a price has risen.

The extra step in this strategy comes from a second, opposite story. Over very short periods, prices
tend to bounce back after a sharp move, because the buyers who panicked or the sellers who were
forced have finished, and the price corrects. If you can tell the winner that is still being bought
from the winner that has already been pushed too far, you keep the momentum reward and avoid the
bounce. The measure used here is intended to make that distinction.

## An everyday comparison

A shop selling a fashionable item. Two months after launch the weekly sales are still rising, and the
shop's sales for the year so far are large. Six months after launch the yearly total is even larger,
but the latest week's sales have started to fall, because everyone who wanted one has one. Both items
show a big yearly figure; only one is still moving up. The strategy here buys the item whose latest
week is still improving and sets aside the one whose latest week has turned, and it does the reverse
for a product that is failing but has just had its first good week.

## The rules, step by step

1. Take every share listed on the two large American exchanges whose price is above 10.00.
2. At the start of each month, record its price. Keep the last thirteen monthly prices, which is
   enough for one year of monthly returns plus the starting price.
3. From those prices compute two things for each share: its return over the past twelve months, and
   its twelve separate one-month returns.
4. Rank every share by its twelve-month return. The best 30 percent are called the winners, the worst
   30 percent the losers, and the middle 40 percent is ignored entirely.
5. For every share compute a momentum ratio: the most recent month's return converted to a one-year
   pace, divided by the one-year pace implied by the whole twelve months. The next section gives the
   formulas. A ratio above one means the latest month ran faster than the year's average pace, so the
   move is speeding up; a ratio below one means it is slowing down.
6. Among the winners keep the fifteen with the lowest ratio: the ones whose momentum is slowing.
   Among the losers keep the fifteen with the highest ratio: the ones whose fall is slowing.
7. Buy the fifteen winners and sell short the fifteen losers, giving each side half the money so the
   two halves are equal: each of the thirty shares gets one thirtieth of the account, in size.
8. Hold for a month, then recompute everything and rebuild the list. Nothing is traded in between.

Two things about the source deserve care. The page's written description says the winner subset has
thirteen shares while its code keeps fifteen, and this tutorial follows the code, since the code is
what would run. And the rule that half the account is long and half is short is what "dollar neutral"
means: the gains and losses from the whole market moving up or down largely cancel, and what is left
is the difference between the slowing winners and the slowing losers.

## The maths, with every symbol named

Every share is judged by three numbers. First, its one-month return for month `i`:

```text
r_i = P_i / P_(i-1) - 1
```

- `r_i` is the return of the share over month `i`, as a decimal: 0.05 means 5 percent.
- `P_i` is the share price at the start of month `i`.
- `P_(i-1)` is the price one month earlier.

Second, the one-year pace implied by the last twelve months, and the one-year pace implied by the
most recent month alone. Both are averages per month, expressed as a yearly rate:

```text
GARR_12 = (1 + r_1) * (1 + r_2) * ... * (1 + r_12) raised to the power 1/12, minus 1
GARR_1  = (1 + r_1) raised to the power 1/12, minus 1
```

- `GARR_12` is the geometric average rate of return over the twelve months: the single monthly rate
  that, compounded for a year, would produce the same total as the twelve actual months. The name is
  the source's, short for "geometric average rate of return".
- `GARR_1` is the same idea applied to the most recent month alone, `r_1`.
- Taking the twelfth root turns a one-month return into a per-month figure on a comparable scale, so
  the ratio below compares like with like.

Third, the ratio the strategy sorts on:

```text
GARR_Ratio = GARR_1 / GARR_12
```

- `GARR_Ratio` above 1 means the latest month was faster than the year's average pace, so the move is
  accelerating; below 1 means it is decelerating. It is a pure number, with no units.

The twelve-month return used for the winner and loser ranking is simply the price today divided by the
price twelve months ago, minus one, which is `(1 + r_1) * ... * (1 + r_12) - 1`.

The weights and the result. If there are `n` shares on each side, each one gets:

```text
w = 0.5 / n for a share that is bought, and w = -0.5 / n for a share sold short
```

- `w` is the fraction of the account held in that share. The bought shares add up to +0.5 and the
  sold shares to -0.5, so the two halves are equal and the account is half long, half short.
- The portfolio's return over the month is the sum of `w` times the share's return for every share.

The costs. The whole book is replaced each month, so both halves are sold and rebuilt:

```text
Cost = 2 * 0.5 * c_long + 2 * 0.5 * c_short + borrow
```

- `c_long` and `c_short` are the cost of one trade as a fraction of the money traded, covering the gap
  between the buying and selling price plus any commission. One basis point is one hundredth of one
  percent. The example below uses 0.001, that is ten basis points, for both.
- The two factors of 2 are the sale and the replacement of each half, and the 0.5 is the size of each
  half, so the trade is 1.0 of the account on the long side and 1.0 on the short side, 2.0 in total.
- `borrow` is the fee paid for borrowing the shares that were sold short, charged on the short half
  of the account for as long as the position is open.

At `c` of 0.001 the monthly trading cost is `2 * 0.001 = 0.2` percent of the account, or about 2.4
percent a year, before the borrow fee. That is a large hurdle for a monthly strategy and it is the
first thing a reader should check against any claimed result.

## A worked example

The selection step, in one month, for ten invented shares. The middle group is omitted, as the rules
say. The one-year pace `GARR_12` is the geometric average of each share's twelve monthly returns; the
one-month pace `GARR_1` is the twelfth root of its most recent month's return minus one.

| Share | 12-month return | Group  | Recent-month return | GARR_12 | GARR_1   | GARR_Ratio |
| ----- | --------------- | ------ | ------------------- | ------- | -------- | ---------- |
| W1    | +52 percent     | winner | -1.5 percent        | 0.03551 | -0.00126 | -0.035     |
| W2    | +44 percent     | winner | +1.0 percent        | 0.03085 | +0.00083 | +0.027     |
| W3    | +38 percent     | winner | +3.5 percent        | 0.02720 | +0.00287 | +0.106     |
| L1    | +8 percent      | loser  | +4.0 percent        | 0.00643 | +0.00327 | +0.509     |
| L2    | +4 percent      | loser  | +1.5 percent        | 0.00327 | +0.00124 | +0.379     |
| L3    | +1 percent      | loser  | +2.0 percent        | 0.00083 | +0.00165 | +1.991     |

The lowest-ratio winner is W1, whose most recent month actually fell, so its ratio is negative; the
highest-ratio loser is L3, whose twelve months barely gained but whose most recent month ran at twice
its yearly pace. The rule therefore buys W1 and sells L3 short. In the real strategy there would be
fifteen shares on each side; one on each side is used here so the arithmetic is visible.

Now run six months, assuming the selection is redone each month and the two chosen shares produce the
returns below. The gross result is `0.5 * long_return - 0.5 * short_return`, because a short position
gains when its share falls. Costs are 0.20 percent for the traded book plus 0.01 percent of borrow on
the short half.

| Month | Bought share return | Sold share return | Gross         | Net after costs |
| ----- | ------------------- | ----------------- | ------------- | --------------- |
| 1     | +4.0 percent        | -2.0 percent      | +3.00 percent | +2.79 percent   |
| 2     | -3.0 percent        | +1.0 percent      | -2.00 percent | -2.21 percent   |
| 3     | +2.0 percent        | +3.0 percent      | -0.50 percent | -0.71 percent   |
| 4     | +5.0 percent        | -4.0 percent      | +4.50 percent | +4.29 percent   |
| 5     | +1.0 percent        | -1.0 percent      | +1.00 percent | +0.79 percent   |
| 6     | -1.0 percent        | -2.0 percent      | +0.50 percent | +0.29 percent   |

Check month 2: the bought share fell 3 percent, which costs `0.5 * -3 = -1.5` percent, and the sold
share rose 1 percent, which costs a further `0.5 * 1 = 0.5` percent, so the gross is -2.00 percent;
after the 0.21 percent of costs the month is -2.21 percent. Compounding the six net returns gives
`1.0279 * 0.9779 * 0.9929 * 1.0429 * 1.0079 * 1.0029 = 1.0521`, about +5.21 percent over half a
year. Two of the six months lost money, which is normal for a long-short book, and the invented run
ends positive only because the two big winning months outweigh the losses.

## What the research actually found

The rule itself is a practitioner variant, and its record is thin. The idea it is built on is not.

| Source                                                                                                                | What it measured                                                           | Result                                                                                                                                                                                                          |
| --------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| The QuantConnect page                                                                                                 | The rules and the intended effect                                          | States that buying winners about to reverse and shorting losers about to reverse should improve on plain momentum; gives no performance figures                                                                 |
| The Quantpedia entry it cites, which is a premium entry                                                               | Performance, sample period and the paper behind it                         | Not publicly readable; the entry is behind a subscription and its named source paper is not shown, so no performance number or sample period can be quoted from it here                                         |
| Jegadeesh and Titman (1993), the momentum paper                                                                       | American shares, 1965 to 1989, buying past winners and selling past losers | Zero-cost winner-minus-loser books earned about one percent a month before costs and before any risk adjustment, which is the classic momentum result this strategy is trying to refine                         |
| De Bondt and Thaler (1985), the overreaction paper                                                                    | Long-horizon losers and winners, American shares                           | Over three to five years the earlier losers went on to beat the earlier winners, which is the reversal the deceleration filter is meant to side-step nearer in time                                             |
| Design and analysis of momentum trading strategies, reported in a brief in this repository (`2101.01006v2`, p.1, p.3) | The shape of momentum returns                                              | Momentum trading returns are positively skewed by construction, even when the underlying returns are not, so a strategy can win fewer than half its months and still be worth running                           |
| Maximum drawdown, recovery, and momentum, reported in the same brief (`1403.8125v4`, p.18)                            | Weekly contrarian portfolios on the S&P 500                                | Ranking by recovery rather than by cumulative return raised the weekly return from 0.042 percent to 0.087 percent and cut the worst fall from 72.56 percent to 43.99 percent, so how a ranking is built matters |
| Physical approach to price momentum, reported in the same brief (`1208.2775v5`, p.14)                                 | Weekly contrarian portfolios on the Korean index                           | The best momentum definition returned 0.2607 percent a week against 0.0685 percent for plain cumulative return, with a lower standard deviation, but transaction costs were not applied in the comparison       |

Read together: the momentum premium is real and heavily studied, and the way a ranking is built,
including the deceleration idea, can change the result. But the specific ratio used here, applied to
the winners and losers of a large American universe, has no independent replication that could be
found. The published record for such refinements, once costs are added and once the effect has been
published, is a decline rather than an increase: the survey reported in this repository's overfitting
brief notes that a set of 97 replicated cross-sectional predictors lost 26 percent of their in-sample
return immediately after their sample ended (`2209.13623v3`, as reported in the overfitting brief).

## How this project relates to it

This repository's own research brief on predictability and trading strategies carries the design
results this strategy depends on. It is
[Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md).
Its third section shows that a momentum book's skewness is a property of the position rule rather
than of the market, and its fourth section shows that ranking shares by how far they have recovered
from a fall beat ranking them by cumulative return, with the improvement concentrated in the losing
book. That is the same question this strategy is asking with its deceleration ratio, and the brief is
the repository's measurement of it.

The building block itself exists in code. The rate-of-change indicator in
[crates/indicators/src/momentum/roc.rs](../../../crates/indicators/src/momentum/roc.rs) computes a
return over a chosen number of bars, which is the arithmetic this rule uses to get twelve monthly and
one monthly return. The strategy that combines them, however, is not implemented here: no file in
this repository selects shares on the ratio of a recent pace to a yearly pace. The closely related
study in [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md) covers
the momentum family in general and is the nearest measurement this repository has.

## Where it goes wrong

- Momentum crashes. After a market-wide fall, the shares that fell furthest are the ones in the
  short book, and they often bounce hardest. A strategy that is short them takes the loss twice, and
  the recovery month is exactly the month this book suffers most. The honest record of long-short
  momentum includes years of loss far larger than the gains of ordinary years.
- The short leg is the expensive one. The shares this strategy sells short are, by construction,
  the weakest, and the weakest shares are the ones most likely to be hard or expensive to borrow.
  A borrow fee of a few percent a year on a large short book removes the edge; an unavailable share
  cannot be shorted at all, and the rule as written does not check whether it can be.
- The ratio is unstable near a zero denominator. The whole twelve-month pace sits in the denominator
  of the ratio. When a share's twelve months are close to flat, that denominator is near zero and the
  ratio swings wildly, so the ranking of two shares can be decided by an unimportant rounding. The
  ratio is also a twelfth root of a monthly return, which makes its scale hard to read.
- Crowding and decay. Momentum is one of the most widely traded effects in finance, and the survey
  reported in this repository's overfitting brief finds that replicated predictors lose a large part
  of their edge once their sample is past. A refinement published for anyone to copy is unlikely to
  be the exception.
- The refinement is unreplicated. The core effect is not in doubt; the claim that slowing winners do
  better than accelerating ones is, and there is no independent study of it that could be found. A
  rule that has been checked once, on one implementation, is the definition of thin evidence.
- Data care matters more than usual. The universe, the 30 percent and 70 percent cut, and the prices
  all have to be as they were at the time; a list of today's shares applied to ten years of history
  looks into the future, and unadjusted prices for dividends and splits break the monthly returns.

## Try it yourself

You need a spreadsheet and thirteen monthly closing prices for about twenty well-known shares. Pick
shares that have existed for the whole period, and use prices adjusted for dividends and splits if
your source offers them.

1. Lay out the prices as rows of months and columns of shares, oldest first, with thirteen rows.
2. Add a row per share for the twelve-month return: the last price divided by the first, minus one.
3. Add twelve rows of monthly returns: each month's price divided by the previous month's, minus one.
4. For each share compute `GARR_12` as the product of the twelve `(1 + monthly return)` terms raised
   to the power one twelfth, minus one, and `GARR_1` as the latest monthly return raised to the power
   one twelfth, minus one.
5. Compute the ratio `GARR_1 / GARR_12` for every share, and note which winners have a ratio below one
   and which losers have a ratio above one.
6. Write down the one slowing winner and one slowing loser the rule would pick, then track their
   returns over the next six months and subtract 0.21 percent a month in costs.

What to notice: the rule almost always picks shares whose latest month was weak, not the shares with
the biggest twelve-month return, which is the whole point and also the whole risk. With only twenty
shares the top and bottom 30 percent is a handful of names, so a single share can decide the month.
And the monthly cost of rebuilding both sides of the book is large enough that a reader should
compare every claimed result against it before believing the result.

## Where this came from

- [QuantConnect strategy library: momentum short term reversal strategy](https://www.quantconnect.com/tutorials/strategy-library/momentum-short-term-reversal-strategy),
  the rules as implemented: rank on the twelve-month return, split into winners and losers, sort each
  by the ratio of the recent pace to the yearly pace, buy the slowing winners and short the slowing
  losers, rebuild monthly.
- [Quantpedia: momentum short term reversal strategy](https://quantpedia.com/Screener/Details/51),
  the premium entry the QuantConnect page points to; it is not publicly readable, so its performance
  figures and source paper could not be quoted.
- Jegadeesh and Titman, [Returns to Buying Winners and Selling Losers: Implications for Stock Market Efficiency](https://onlinelibrary.wiley.com/doi/10.1111/j.1540-6261.1993.tb04702.x),
  the classic momentum study.
- De Bondt and Thaler, [Does the Stock Market Overreact?](https://onlinelibrary.wiley.com/doi/10.1111/j.1540-6261.1985.tb05004.x),
  the overreaction and reversal study behind the deceleration idea.
- [Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  this repository's brief, whose third and fourth sections carry the momentum-shape and
  drawdown-ranking results quoted above.
- [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md), the
  repository's study of the momentum family.

## Words used in this tutorial

- borrow fee: the charge for borrowing a share in order to sell it short.
- long: owning something, so that a rise in its price is a gain.
- momentum: the tendency of something that has been rising to keep rising for a while.
- rebalance: to rebuild a portfolio back to its intended weights.
- return: the percentage change in the value of something over a period.
- short selling: borrowing something you do not own, selling it, and buying it back later, so that a
  fall in its price is a gain.
- turnover: how much of a portfolio is traded over a period.
- weight: the share of your money placed in one holding.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
