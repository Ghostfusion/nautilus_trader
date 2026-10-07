# Momentum and reversal with volatility: best performers among the jumpiest large shares

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                                                                  |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| What it trades            | Shares of large American companies, bought and sold short in two baskets chosen from the most jumpy names                                                                                                                                                                                                                                              |
| How often it trades       | Once a month for one sixth of the book, with each basket held for six months                                                                                                                                                                                                                                                                           |
| What you need             | A spreadsheet and six months of daily share prices                                                                                                                                                                                                                                                                                                     |
| Where the rules come from | [QuantConnect strategy library, momentum and reversal combined with volatility effect](https://www.quantconnect.com/tutorials/strategy-library/momentum-and-reversal-combined-with-volatility-effect-in-stocks) and the [Quantpedia entry](https://quantpedia.com/strategies/momentum-and-reversal-combined-with-volatility-effect-in-stocks) it cites |
| The underlying research   | Wei, [Do Momentum and Reversals Coexist?](http://papers.ssrn.com/sol3/papers.cfm?abstract_id=1679464)                                                                                                                                                                                                                                                  |
| How well it held up       | Weak: one American sample from 1964 to 2009, extended by the same authors on the same data with no independent replication, and the effect shows up only inside the large-company half and only among the jumpiest names                                                                                                                               |
| Also appears in           | [Sector momentum](../sector-momentum/README.md) in this collection, which buys recent winners the same way                                                                                                                                                                                                                                             |

## The idea in one paragraph

Momentum is the habit of what has recently risen to keep rising for a while. Reversal is the opposite:
what has just moved far snaps back. This strategy uses the fact that both can be true at once, in
different kinds of shares. It first puts large companies into groups by how jumpy their prices have
been over the past six months. Among the calm shares, the recent winners tend to give back some of
their gains. Among the jumpy shares, the recent winners tend to keep winning and the recent losers
tend to keep losing. So the rule buys the best recent performers inside the jumpiest group and sells
short the worst recent performers in that same group.

## Why anyone believed it

The story is about how people react to news that is hard to interpret. When a calm, familiar company
reports something, the market digests it quickly, and any overshoot is corrected soon after. When a
jumpy company reports something, the news is vaguer and people disagree about what it means, so the
price drifts in the same direction for weeks as each new investor forms a view. That is the momentum
side: information arrives slowly and the price keeps catching up.

Among calm shares the opposite happens. Any sharp move is quickly recognized as noise or as an
over-reaction, and the price is pulled back. The counterparty, then, is the investor who over-reacts
to a surprise in a quiet company, and the investor who under-reacts to a surprise in a jumpy one. If
those two habits persist, the same rule can profit from both.

## An everyday comparison

Think of two kinds of students waiting for an exam result. The one who always scores near the same
mark gets a grade that is easy to read: an unusually high mark is probably luck, and everyone expects
the next one to return to normal. The one whose marks swing wildly is harder to read: an unusually
high mark might be a real improvement, and the tutor keeps expecting the next result to confirm it.
The strategy takes the jumpy student's improving marks at face value and discounts the calm
student's surprising ones. It is a bet about how much a single surprising result tells you, given how
surprising results usually are from that source.

## The rules, step by step

1. Start with every share listed on the New York Stock Exchange, the American Stock Exchange and
   Nasdaq, and keep those priced above five dollars.
2. Compute each company's size as the share price multiplied by the number of shares. Sort by size and
   keep the larger half. The rule trades only large companies, because that is where the published
   effect appears.
3. For each remaining share, take the daily prices of the last six months, enough to give about 126
   trading days.
4. Compute the realized six-month return: the last price divided by the first price, minus one.
5. Compute the realized volatility: the standard deviation of the daily percentage changes, then
   multiplied by the square root of 252 to express it as a yearly figure.
6. Ignore the most recent five trading days when computing both. That week is skipped because the
   prices in it are distorted by the mechanics of trading around the month end.
7. Sort the large shares by volatility and keep the jumpiest fifth, the top twenty percent.
8. Inside that jumpy fifth, sort by the six-month return. Buy the best fifth and sell short the worst
   fifth, in equal amounts within each side.
9. Hold each basket for six months. Form a new basket every month, so the book always holds six, and
   each month one sixth of the book is replaced. Repeat.

## The maths, with every symbol named

The realized return over the window:

```text
R = (P_end / P_start) - 1
```

- `P_start` is the price at the beginning of the six-month window.
- `P_end` is the price at the end, five trading days before the formation date.

The daily percentage change:

```text
r_i = (P_i / P_(i-1)) - 1
```

- `P_i` is the closing price on day `i`, and `P_(i-1)` the day before.

The average daily change and the daily volatility:

```text
r_avg = (r_1 + r_2 + ... + r_n) / n
```

```text
sigma_daily = sqrt( ((r_1 - r_avg)^2 + ... + (r_n - r_avg)^2) / (n - 1) )
```

- `n` is the number of daily changes in the window.
- `r_avg` is their ordinary average.
- `sigma_daily` is the standard deviation, a measure of how far the daily changes typically sit from
  that average.

The annualized volatility, the number the sorting uses:

```text
sigma_year = sigma_daily * sqrt(252)
```

- `252` is the approximate number of trading days in a year.
- `sigma_year` is written as a decimal, so 0.60 means a share whose price typically moves 60 percent a
  year.

The contribution of one basket to the whole book, given that each basket is one sixth of the long
side and one sixth of the short side:

```text
Contribution = (R_long - R_short) / 6
```

- `R_long` is the equal-weighted return of the chosen best performers over the holding period.
- `R_short` is the equal-weighted return of the shorted worst performers over the same period, so
  subtracting it is the same as subtracting their price move.

The whole book holds six baskets at once, so its return over a six-month window is the sum of the six
contributions. Finally the cost, at the level of the book:

```text
Cost = t * c
```

- `t` is the fraction of the book traded in a year. One sixth of the book is replaced each month on
  the long side and one sixth on the short side, giving about 8.0 times the book over a year once both
  the selling and the buying are counted.
- `c` is the cost per trade as a fraction of the amount traded, covering the gap between buying and
  selling prices plus commission. A realistic figure for large American shares is 0.001, that is ten
  basis points, where one basis point is one hundredth of one percent.

## A worked example

Ten invented large shares, with the volatility and six-month return that the rule computes for each.
The point of the first table is the two-step sort: volatility picks the pool, and return picks the two
ends of it.

| Share | Annualized volatility | Six-month return | In the jumpiest fifth? |
| ----- | --------------------- | ---------------- | ---------------------- |
| A     | 0.62                  | +20%             | yes                    |
| B     | 0.58                  | -14%             | yes                    |
| C     | 0.45                  | +6%              | no                     |
| D     | 0.42                  | -3%              | no                     |
| E     | 0.38                  | +9%              | no                     |
| F     | 0.35                  | +1%              | no                     |
| G     | 0.33                  | -6%              | no                     |
| H     | 0.30                  | +4%              | no                     |
| I     | 0.28                  | -1%              | no                     |
| J     | 0.25                  | +2%              | no                     |

The jumpiest fifth of ten is two shares, A and B. Inside them the six-month return decides: A at plus
20 percent is the best and is bought, B at minus 14 percent is the worst and is sold short. The other
eight shares are ignored this month.

Now six invented baskets, each one sixth of the long side and one sixth of the short side of the book.
Each row follows one basket over its own six-month holding period. The contribution is
`(R_long - R_short) / 6`, and the cost is four sides of one sixth of the book at ten basis points,
which is about 0.067 percent.

| Basket | Long side | Short side | Contribution | Cost   | Net     |
| ------ | --------- | ---------- | ------------ | ------ | ------- |
| 1      | +11.0%    | -7.0%      | +3.000%      | 0.067% | +2.933% |
| 2      | +3.0%     | +4.0%      | -0.167%      | 0.067% | -0.233% |
| 3      | +14.0%    | -2.0%      | +2.667%      | 0.067% | +2.600% |
| 4      | -5.0%     | +6.0%      | -1.833%      | 0.067% | -1.900% |
| 5      | +9.0%     | -10.0%     | +3.167%      | 0.067% | +3.100% |
| 6      | +6.0%     | +1.0%      | +0.833%      | 0.067% | +0.767% |

Work one row: basket 1 has a long side of plus 11 percent and a short side of minus 7 percent, so the
contribution before costs is (11 minus minus 7) divided by 6, which is 18 divided by 6, or 3.000
percent. Subtracting the 0.067 percent cost gives 2.933 percent.

Because the book holds all six baskets, its six-month result is the sum of the six net figures, which
is 7.267 percent. Compounding two such half-years gives 1.07267 times 1.07267, which is 1.1506, or
about 15.1 percent a year after costs. That sits near the published figure below, but the example is
invented and cannot show that the rule will pay in future.

## What the research actually found

| Source                                                          | What it measured                                                                         | Result                                                                                                                                                                                                                                                                                                                       |
| --------------------------------------------------------------- | ---------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Quantpedia, summarising Wei                                     | Large American shares with high volatility, ranking and holding six months, 1964 to 2009 | 16.46 percent a year from a 1.278 percent monthly figure, volatility 19.22 percent, worst fall 36.59 percent, reward-to-risk 0.65                                                                                                                                                                                            |
| Wei, Do Momentum and Reversals Coexist?                         | NYSE, AMEX and Nasdaq shares, 1964 to 2009                                               | Momentum prevailed among small shares, but among large shares momentum and reversal coexisted for holding periods up to six months, dividing along volatility: large low-volatility shares reversed while large high-volatility shares kept moving; the author states this cannot be fully explained by risk or by behaviour |
| Wei and Yang, Short-Term Momentum and Reversals in Large Stocks | The same sample and finding                                                              | Confirms the pattern and proposes that investors become overconfident in vague situations, which lets under-reaction and over-reaction occur together                                                                                                                                                                        |
| Chiang, Kirby and Nie                                           | The relation between volatility and return                                               | The relation is not a simple straight line: it is negative for big losers and positive for big winners, which is why the rule splits winners from losers inside the volatile pool                                                                                                                                            |

The published result is strong on paper, with a long sample and a clear economic story. It is also
narrow. It comes from one study and its companion paper, on one country and one period, and the
authors themselves report that the effect is not explained by the usual risk factors or by a simple
behavioural account. There is no independent replication in a second sample, which is why the grade
below the table treats it as weak rather than strong.

## How this project relates to it

This repository's study of rotation carries the closest relevant measurement, in
[strategies/sector_rotation_strategies.md](../../../strategies/sector_rotation_strategies.md). Section
5 states the rule that matters here: a stock-level pattern such as this one cannot be transferred to
another universe without being tested there, because aggregation removes much of the effect. Section
7.1 then reports a pre-registered test on 2010 to 2026 data in which 24 of 24 tested short-horizon
rotation states were not confirmed, and notes that academic momentum uses formation windows of six to
twelve months and skips the most recent month precisely because one-month returns tend to reverse.
The lesson for this rule is that the six-month window and the skipped five days are not decoration;
they are the parts the evidence actually supports.

## Where it goes wrong

- One sample, no replication. The result comes from a single American study and its companion, on the
  same data, so there is no second team or market to confirm it.
- It lives in the most expensive shares. Volatile shares have wider spreads and cost more to borrow
  for a short sale, which is exactly the cost that a momentum rule on jumpy names must pay.
- Momentum crashes. After a market-wide fall, the shares that fell hardest are the ones this rule
  wants to buy, and they can bounce or fall further violently, so the losses arrive in bursts.
- Only the large half is used. The effect does not appear in small shares, where ordinary momentum
  dominates, so the rule discards most of the universe and most of the diversification.
- The five-day skip is doing work. Removing the most recent week changes the answer, which hints that
  some of the measured return comes from the mechanics of month-end trading rather than from a lasting
  edge.
- Signals decay. A published rule that is easy to describe attracts money, and the returns arrive
  earlier and smaller as more participants trade it.

## Try it yourself

You need a spreadsheet and six months of daily closing prices for a handful of shares, available from
any finance website.

1. Make one column per share holding the daily closing price, and one row per day.
2. Next to each price, compute the day's percentage change: today divided by yesterday, minus one.
3. For each share, compute the average of those daily changes, then the standard deviation of them,
   which most spreadsheets provide with a built-in function.
4. Multiply the standard deviation by the square root of 252 to get a yearly volatility figure.
5. Also compute the six-month return: the last price divided by the first, minus one. Drop the most
   recent five days from both calculations.
6. Sort the shares by volatility, keep the top fifth, then sort that group by the six-month return and
   buy the best fifth of it and short the worst fifth.

What to notice: the two sorts usually pick a very small group, sometimes only one or two shares, so the
result swings on a single name. Also notice how much the volatility ranking changes when you extend
the window from six months to a year, which tells you how sensitive the whole rule is to choices you
made rather than choices the data made.

## Where this came from

- [QuantConnect strategy library: momentum and reversal combined with volatility effect in stocks](https://www.quantconnect.com/tutorials/strategy-library/momentum-and-reversal-combined-with-volatility-effect-in-stocks),
  the rules as implemented: the five-dollar filter, the large half by size, the two-step sort, and the
  six-month holding with monthly one-sixth rebalancing.
- [Quantpedia: momentum and reversal combined with volatility effect in stocks](https://quantpedia.com/strategies/momentum-and-reversal-combined-with-volatility-effect-in-stocks),
  the performance figures, the sample and the underlying papers.
- Wei, [Do Momentum and Reversals Coexist?](http://papers.ssrn.com/sol3/papers.cfm?abstract_id=1679464),
  the source paper.
- Wei and Yang, [Short-Term Momentum and Reversals in Large Stocks](http://papers.ssrn.com/sol3/papers.cfm?abstract_id=2029984),
  the companion paper on the same data.
- [strategies/sector_rotation_strategies.md](../../../strategies/sector_rotation_strategies.md),
  this repository's study of rotation, whose Sections 5 and 7.1 carry the transfer warning and the
  pre-registered short-horizon test.

## Words used in this tutorial

- holding period: the length of time a position is kept open.
- momentum: the tendency of something that has been rising to keep rising for a while.
- percentile: the value below which a given share of a list falls, used here as a fifth, the top
  twenty percent.
- quintile: one of five equal groups formed by sorting, so the top quintile is the best fifth.
- reversal: a move in the opposite direction to the recent one.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- standard deviation: a measure of how far a set of values typically sits from its average.
- volatility: how much a price moves around its average, usually quoted as a yearly percentage.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
