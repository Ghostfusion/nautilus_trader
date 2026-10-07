# Fundamental score and short-term reversals: buying recent losers that are financially sound

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                                                  |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of large American companies, bought and sold short in pairs                                                                                                                                                                                                                                                                     |
| How often it trades       | Once a month, when the list is rebuilt                                                                                                                                                                                                                                                                                                 |
| What you need             | A spreadsheet with monthly share prices and the last quarterly accounts of each company                                                                                                                                                                                                                                                |
| Where the rules come from | [The list's implementation file](https://github.com/paperswithbacktest/awesome-systematic-trading/blob/main/static/strategies/combining-fundamental-fscore-and-equity-short-term-reversals.py), which restates [the Quantpedia entry](https://quantpedia.com/strategies/combining-fundamental-fscore-and-equity-short-term-reversals/) |
| The underlying research   | Zhu, Sun and Chen, [Noise Trading, Slow Diffusion of Information, and Short-Term Reversals](https://ssrn.com/abstract=3097420), which uses the nine-point score from Piotroski's study of financial statements                                                                                                                         |
| How well it held up       | Weak: one American sample from 1984 to 2015 with a modest measured result, and Quantpedia's own out-of-sample check turned slightly negative, so Quantpedia rates its confidence in the idea as Moderate                                                                                                                               |
| Also appears in           | [Short-term reversal](../../../tutorials/quantconnect/short-term-reversal/README.md), [Short term reversal strategy in stocks](../../../tutorials/quantconnect/short-term-reversal-strategy-in-stocks/README.md) and [Earnings quality factor](../../../tutorials/quantconnect/earnings-quality-factor/README.md)                      |

## The idea in one paragraph

Shares that have just fallen often bounce back a little, and shares that have just risen often give some
back. That is the pattern called short-term reversal, and on its own it is a weak and expensive trade. This
strategy tries to keep only the reversals that have a reason behind them, by scoring every company on nine
simple questions taken from its financial statements. A company that answers most of them well is called
financially strong; one that answers most of them badly is financially weak. The strategy buys recent losers
that are strong and sells short recent winners that are weak, holding the pairs for a month. The bet is that
a fall in the price of a sound company is an over-reaction that corrects, while a rise in the price of a
fragile company is a mistake that also corrects.

## Why anyone believed it

Two different things push a share price away from what the accounts say it is worth. The first is noise
trading: people buy and sell for reasons unrelated to the company, such as needing cash or following a
headline, and those moves unwind over days and weeks. The second is under-reaction: when a company files its
accounts, the information spreads slowly, and the price adjusts over the following weeks instead of at once.

A price move on its own does not say which of the two is happening. Adding the accounts helps separate them.
If a company's price fell but its accounts are improving, the fall is more likely noise and will reverse. If
a company's price rose but its accounts are deteriorating, the rise is more likely noise too, and it also
reverses. The counterparty is the buyer who reacts to a headline without reading the filing, and the seller
who dumps a sound company because the price is falling.

## An everyday comparison

Think of two second-hand bicycles for sale on the same street, both priced lower this week than last. One
belongs to a careful owner who kept it well maintained, and the other has a cracked frame and worn brakes.
The lower price on the first is a bargain that will be recognised, while the lower price on the second is
the market waking up to a real problem, and it may fall further. A buyer who looks only at the price fall
cannot tell the two apart; a buyer who also checks the machine can. The nine-point score is the inspection
of the machine.

## The rules, step by step

1. Start with common shares listed on the New York, American and Nasdaq exchanges, priced above 5 dollars.
   The list's implementation takes the 500 most heavily traded American shares instead.
2. Restrict to the largest companies: keep the top 40 percent by company value, measured as the number of
   shares multiplied by the share price. The study reports the result for these large shares because the
   smallest companies behave differently for reasons of their own.
3. For each company, answer nine questions from its most recent quarterly accounts. Each question scores one
   point when the answer is good and zero when it is not, so the total runs from 0 to 9. The nine questions
   are listed in the maths section below.
4. Label the company. A score of 7, 8 or 9 is fundamentally strong. A score of 4, 5 or 6 is middle. A score
   of 0, 1, 2 or 3 is fundamentally weak.
5. For each company, compute its return over the past month: today's price divided by the price one month
   ago, minus one.
6. Buy the recent losers that are strong: companies whose past-month return is negative and whose score is 7
   or more.
7. Sell short the recent winners that are weak: companies whose past-month return is positive and whose
   score is 3 or less. Companies in the middle group are not traded, whichever way their price moved.
8. Give every position on a side the same amount of money, so with six long positions each gets one sixth of
   the long side's money.
9. Rebuild at the end of each month and pay the costs. Trading costs are roughly 0.05 to 0.15 percent of the
   traded amount per side for liquid shares, and anything held short pays a borrow fee, often around 0.5
   percent a year on the value borrowed.

## The maths, with every symbol named

The nine questions, one point each, no accounting background assumed:

1. `ROA > 0`: did the company make a profit? Return on assets is the yearly profit divided by everything the
   company owns, so a positive value means it earned more than it spent.
2. `CFO > 0`: did more cash come in from running the business than went out? This is the cash flow from
   operations, roughly the money the business generated before investing and financing.
3. `ROA > ROA_last_year`: is the profit per unit of assets higher than a year ago?
4. `CFO > ROA`: is the cash coming in larger than the reported profit? When profit is larger than the cash
   behind it, the difference is called accruals, and accounting adjustments are doing more of the work.
5. `Debt_ratio < Debt_ratio_last_year`: is long-term debt a smaller share of total assets than a year ago?
6. `Current_ratio > Current_ratio_last_year`: is the company better able to pay its short-term bills? The
   current ratio is short-term assets divided by short-term obligations, excluding borrowed money.
7. `Shares_issued = 0`: did the company avoid raising money by selling new shares? New shares dilute the
   existing owners, so not issuing them is the good outcome.
8. `Gross_margin > Gross_margin_last_year`: does the company keep more of each sale than a year ago? Gross
   margin is revenue minus the direct cost of producing what was sold, divided by revenue.
9. `Turnover > Turnover_last_year`: does the company generate more sales for each dollar of assets than a
   year ago? Turnover is revenue divided by total assets.

The score:

```text
FSCORE = (1 if ROA > 0 else 0) + (1 if CFO > 0 else 0) + ... + (1 if Turnover > Turnover_last_year else 0)
```

- `FSCORE` is the total, from 0 to 9.
- Each bracket contributes 1 when its condition is true and 0 when it is false.
- `else` means "otherwise", so the second value is used when the condition fails.

The past-month return and the two lists:

```text
R = P_today / P_one_month_ago - 1
Long list:  R < 0 and FSCORE >= 7
Short list: R > 0 and FSCORE <= 3
```

- `R` is the past-month return.
- `P_today` and `P_one_month_ago` are the share prices on those dates.

The portfolio return and the cost:

```text
R_portfolio = average of the long positions' returns - average of the short positions' returns
Cost = t * c + b_short
```

- `t` is the traded fraction of the account, counting both the sale and the purchase, so replacing the whole
  long side and the whole short side gives `t` of 4.0.
- `c` is the cost per side as a fraction of the amount traded, around 0.001 for 10 basis points, where one
  basis point is one hundredth of one percent.
- `b_short` is the borrow fee over one month on the value held short.

## A worked example

Eight companies are used. The nine answers are written as 1 or 0 in order, so `111111111` is a perfect
score. The numbers are invented. The last column is the past-month return.

| Company | 1   | 2   | 3   | 4   | 5   | 6   | 7   | 8   | 9   | FSCORE | Past month | Label  |
| ------- | --- | --- | --- | --- | --- | --- | --- | --- | --- | ------ | ---------- | ------ |
| A       | 1   | 1   | 1   | 1   | 1   | 0   | 1   | 1   | 1   | 7      | -4%        | Strong |
| B       | 1   | 1   | 1   | 0   | 1   | 1   | 1   | 1   | 1   | 7      | -2%        | Strong |
| C       | 1   | 1   | 1   | 1   | 1   | 1   | 1   | 1   | 1   | 9      | +1%        | Strong |
| D       | 0   | 0   | 1   | 0   | 0   | 1   | 1   | 0   | 1   | 4      | +6%        | Middle |
| E       | 0   | 0   | 0   | 0   | 0   | 1   | 1   | 0   | 0   | 2      | +5%        | Weak   |
| F       | 0   | 1   | 0   | 0   | 0   | 0   | 1   | 0   | 0   | 2      | +3%        | Weak   |
| G       | 1   | 1   | 1   | 1   | 0   | 0   | 1   | 1   | 1   | 7      | +2%        | Strong |
| H       | 0   | 0   | 0   | 0   | 1   | 1   | 0   | 0   | 0   | 2      | -1%        | Weak   |

Reading the selection rule: the long list needs a strong score and a negative past month, which gives A and
B. C and G are strong but rose, so they are not bought. The short list needs a weak score and a positive
past month, which gives E and F. H is weak but fell, so it is not shorted. D is in the middle and is ignored
either way.

Suppose the next month brings these returns:

| Position | Weight within its side | Next-month return | Contribution to the total |
| -------- | ---------------------- | ----------------- | ------------------------- |
| A long   | 0.50                   | +1.5%             | +0.75%                    |
| B long   | 0.50                   | -0.5%             | -0.25%                    |
| E short  | 0.50                   | -2.0%             | +1.00%                    |
| F short  | 0.50                   | -1.0%             | +0.50%                    |
| Total    |                        |                   | +2.00%                    |

The long side averaged +0.5 percent and the two shorted shares fell, which adds 1.5 percent. Now the costs.
Both positions on each side change, so the whole long book and the whole short book are replaced, giving a
traded fraction `t` of 4.0. At 10 basis points per side and a 0.5 percent annual borrow fee on the short
unit:

```text
Cost = 4.0 * 0.001 = 0.004, that is 0.40 percent
Borrow = 0.005 / 12 = 0.000417, that is 0.042 percent of the account
Net return = 2.00 - 0.40 - 0.04 = 1.56 percent
```

Over six months, a run of results could look like this. The numbers are invented; the point is the
arithmetic and the size of the cost line when both sides are replaced every month.

| Month | Long side | Short side | Gross | Trades | Cost  | Borrow | Net    |
| ----- | --------- | ---------- | ----- | ------ | ----- | ------ | ------ |
| 1     | +2.0%     | -1.0%      | +3.0% | 4.0    | 0.40% | 0.04%  | +2.56% |
| 2     | -1.5%     | +0.5%      | -2.0% | 4.0    | 0.40% | 0.04%  | -2.44% |
| 3     | +1.0%     | -2.0%      | +3.0% | 4.0    | 0.40% | 0.04%  | +2.56% |
| 4     | +0.5%     | -0.5%      | +1.0% | 4.0    | 0.40% | 0.04%  | +0.56% |
| 5     | +2.5%     | -1.5%      | +4.0% | 4.0    | 0.40% | 0.04%  | +3.56% |
| 6     | -0.5%     | +0.5%      | -1.0% | 4.0    | 0.40% | 0.04%  | -1.44% |

The six months total 5.36 percent, about 0.9 percent a month before anything goes wrong. The cost line, at
0.44 percent a month, is roughly 5.3 percent a year, which is a large part of the gross result. That is the
central lesson of the strategy: reversal trades are short-lived and expensive, and the fundamental score is
an attempt to make the surviving trades worth the cost.

## What the research actually found

| Source                                                                                    | What it measured                                                                                                                                | Result                                                                                                                                                                                                                                  |
| ----------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Zhu, Sun and Chen, Noise Trading, Slow Diffusion of Information, and Short-Term Reversals | New York, American and Nasdaq common shares, 1984 to 2015, large shares, buying losers with strong scores and shorting winners with weak scores | Quantpedia's page reports 12.01 percent a year, volatility 20.61 percent, a reward-to-risk of 0.39 and a significance value of 3.08, with a worst fall of 59.02 percent                                                                 |
| Quantpedia's out-of-sample check                                                          | The same strategy after the paper's sample                                                                                                      | Quantpedia reports that the out-of-sample backtest shows slightly negative performance and that the strategy's advantage appears to be deteriorating out of sample, which is why it rates its confidence as Moderate rather than Strong |
| Piotroski, Value Investing: The Use of Historical Financial Statement Information         | The nine-point score itself, applied to financially weak versus strong firms                                                                    | The score was originally built to separate winners from losers among companies that look cheap, and it scores a company on profitability, leverage, liquidity and operating efficiency                                                  |
| The awesome-systematic-trading list, its own replication record                           | 4,843 coded papers; aggregate statistics only                                                                                                   | The median strategy returns a reward-to-risk of 0.37 and 48 percent clear a statistical significance bar of 1.96; this is the list's own aggregate measurement, not a figure for this strategy                                          |

Read together: the paper reports that the fundamental-anchored reversal beats the plain reversal, and
Quantpedia's own extract says plain reversals do not survive costs while the fundamental version does. But
Quantpedia then reports that its own out-of-sample run turned slightly negative. That is the honest state of
this idea: an in-sample result with a large statistic on one long sample, and a replication that reduced it
to about nothing. The list publishes no reward-to-risk number for this strategy on its own.

## How this project relates to it

This repository has tutorials for the reversal half of the idea.
[Short-term reversal](../../../tutorials/quantconnect/short-term-reversal/README.md) and
[Short term reversal strategy in stocks](../../../tutorials/quantconnect/short-term-reversal-strategy-in-stocks/README.md)
measure the plain version, buying recent losers and selling recent winners with no fundamental check, and
both are the baseline this strategy claims to improve on. On the accounting half,
[Earnings quality factor](../../../tutorials/quantconnect/earnings-quality-factor/README.md) uses similar
statement items to separate companies whose reported profits are backed by cash from those where they are
not, which is the same distinction as question four of the score.

The evaluation question, whether an out-of-sample deterioration like this one should change a reader's mind,
is covered in
[strategies/books2/28_overfitting_and_research_integrity.md](../../../strategies/books2/28_overfitting_and_research_integrity.md),
whose first section reports what replication actually shows in asset pricing.

## Where it goes wrong

- The out-of-sample result is the headline. Quantpedia's own later run turned slightly negative. A strategy
  that was profitable in its sample and then stops is the most common way a published idea fails.
- Costs dominate a monthly reversal. The worked example shows 0.44 percent a month going to trading and
  borrowing, which is most of a modest gross return. Plain reversal strategies are widely reported to fail
  once costs are charged.
- Shorting is the hard half. Recent winners that are financially weak are often expensive to borrow, and a
  short position can lose more than the account's slice of it if the share keeps rising.
- The score uses stale data. The most recent quarterly accounts are up to three months old, and the
  year-on-year comparisons span a year, so the score describes the company as it was, not as it is.
- Small changes in the cut-offs matter. The boundaries 7 and 3, the top 40 percent size filter and the
  one-month reversal window are all choices made by the authors. Moving any of them changes which companies
  are traded.
- Accounting rules differ between companies and change over time, so the nine answers are not perfectly
  comparable across the whole list, and a company can score well by reclassification rather than by
  improving.

## Try it yourself

You need a spreadsheet, monthly prices for about 40 well-known companies, and their last two quarterly
filings, which are free on the company filings site.

1. Build a sheet with one row per company and columns for the eight raw numbers the nine questions need:
   `Profit`, `Assets`, `CashFromOperations`, `LongTermDebt`, `CurrentAssets`, `CurrentLiabilities`,
   `SharesOutstanding`, `Revenue`, `GrossProfit`, and one more column per figure for the same item a year
   earlier.
2. Add nine columns, one per question, each holding 1 or 0. For example, the first is `1` if `Profit / Assets`
   is above zero, otherwise `0`.
3. Add a column `FSCORE` equal to the sum of the nine columns.
4. Add columns `PriceToday`, `PriceOneMonthAgo` and `PastMonth` equal to `PriceToday / PriceOneMonthAgo - 1`.
5. Mark the companies on the buy list, where `PastMonth` is negative and `FSCORE` is 7 or more, and the
   companies on the short list, where `PastMonth` is positive and `FSCORE` is 3 or less.
6. In the next month's row, average the two lists as the rules describe, then subtract 0.44 percent for the
   costs and the borrow fee.

What to notice: the two lists are usually short, often only a handful of companies, so the month's result
depends on a few names. Also notice how many of the strongest-scoring companies are not traded at all,
because their price rose, and how many of the weakest are not traded because their price fell. The score on
its own does not decide anything; it only narrows the reversal trade, which is why the strategy is really a
reversal trade with a filter.

## Where this came from

- [The awesome-systematic-trading list](https://github.com/paperswithbacktest/awesome-systematic-trading), and
  its implementation file for
  [combining-fundamental-fscore-and-equity-short-term-reversals](https://github.com/paperswithbacktest/awesome-systematic-trading/blob/main/static/strategies/combining-fundamental-fscore-and-equity-short-term-reversals.py),
  which states the universe, the nine questions, the score bands and the monthly rebalancing.
- [Quantpedia: Combining Fundamental FSCORE and Equity Short-Term Reversals](https://quantpedia.com/strategies/combining-fundamental-fscore-and-equity-short-term-reversals),
  the rules, the extracted performance figures and Quantpedia's out-of-sample note.
- Zhu, Sun and Chen, [Noise Trading, Slow Diffusion of Information, and Short-Term Reversals](https://ssrn.com/abstract=3097420),
  the source paper.
- Piotroski, Value Investing: The Use of Historical Financial Statement Information to Separate Winners from
  Losers, the 2000 paper that introduced the nine-point score.
- [the overfitting brief](../../../strategies/books2/28_overfitting_and_research_integrity.md),
  this repository's brief on what replication shows and how to read an out-of-sample failure.

## Words used in this tutorial

- accruals: the part of reported profit that is not matched by cash coming in.
- basis point: one hundredth of one percent, so ten basis points is 0.10 percent.
- borrow fee: the charge for borrowing something you do not own in order to sell it short.
- current ratio: short-term assets divided by short-term obligations, a rough test of whether a company can
  pay its near-term bills.
- gross margin: the share of each sale that remains after the direct cost of what was sold.
- long: owning something, so you gain if its price rises.
- return on assets: a company's yearly profit divided by everything it owns.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
