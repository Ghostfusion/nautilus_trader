# Buying the shares whose profit most beat what was expected of them

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                          |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of about a thousand large and heavily traded American companies                                                                                                                                         |
| How often it trades       | Once a month, though the signal itself changes only when companies report their quarterly profits                                                                                                              |
| What you need             | A spreadsheet and several years of quarterly profit figures and monthly prices                                                                                                                                 |
| Where the rules come from | [QuantConnect strategy library, standardized unexpected earnings](https://www.quantconnect.com/tutorials/strategy-library/standardized-unexpected-earnings)                                                    |
| The underlying research   | Foster, Olsen and Shevlin, [Earnings Releases, Anomalies, and the Behavior of Security Returns](https://www.jstor.org/stable/247321), and the earnings-surprise line of work it began                          |
| How well it held up       | Strong: the grade rests on a published anomaly that has been replicated across markets and decades, with the caveat that the reward concentrates in the smallest, least traded shares, where costs are highest |
| Also appears in           | [Price and earnings momentum](../price-and-earnings-momentum/README.md), which uses the same profit-growth idea alongside price strength                                                                       |

## The idea in one paragraph

Every three months a company reports how much profit it made per share, called earnings per share.
Before the report, analysts and the company's own history set an expectation. When the report beats
that expectation, the share price often rises, but not all at once: the price tends to keep drifting
up for weeks or months afterwards. This strategy measures how far each company's profit beat
expectation, divides that surprise by how much its surprises normally vary so that large companies
and small ones can be compared, and buys the top slice. The QuantConnect page buys the top five
percent and rebuilds the list monthly.

## Why anyone believed it

The counterparty is an investor who does not pay attention to quarterly reports, or who sells for
reasons unrelated to them. A company that beats expectations is genuinely better than the market
thought, but the news spreads slowly: it takes several days for it to reach every holder, analysts
revise their forecasts gradually, and some investors only react when the next report confirms the
first. That slowness is what creates the drift.

The deeper argument is about attention. Investors must choose what to read, and there is far more
company news than anyone can follow. A surprising report is easy to miss, especially for a company
that is not in the headlines. The strategy sits on the other side of that inattention by reading
every report and acting on the ones that surprise. A separate part of the literature argues the
effect is a reward for a risk that is hard to diversify, rather than an error by other investors, and
the disagreement between those two explanations is still open.

## An everyday comparison

Think of a school where each pupil takes a test every term and the teacher expects a certain score
from each. One pupil always scores about 70, so a score of 74 is a small surprise of four points.
Another always scores about 50 but wobbles by twenty points, so 74 is also four points above
expectation but, relative to that pupil's own wobble, it means much less. A teacher ranking the class
by "how far above expectation, given how much this pupil normally varies" is doing exactly what this
strategy does. The pupil with the best surprise score is the one whose good result stands out
against their own record.

## The rules, step by step

1. At the start of each month, build a list of about a thousand shares of American companies that
   are heavily traded, cost more than five dollars, and have quarterly profit data available.
2. For each share, collect its reported earnings per share for the last twelve quarters. Earnings
   per share is the company's quarterly profit divided by the number of shares it has outstanding.
3. For the most recent quarter, compute the surprise: this quarter's earnings per share minus the
   earnings per share four quarters earlier, which is the same quarter one year ago.
4. For each of the last eight quarters, also compute the same four-quarter change. Work out the
   standard deviation of those eight numbers, which is how much the company's surprise normally
   varies.
5. Divide the most recent surprise by that standard deviation. The result is the standardized
   unexpected earnings score. A score of +2 means the latest surprise was twice the usual size of
   this company's surprises, in the good direction.
6. Rank all the shares by the score, best first.
7. Buy the top five percent in equal amounts. With a thousand shares that is fifty shares, each with
   two percent of the money. Do not buy anything else and do not sell anything short.
8. Hold for a month. At the start of the next month, rebuild the list, sell whatever has dropped out
   and buy whatever has entered. The scores move from month to month because companies report at
   different times, so the list changes gradually rather than all at once.
9. Allow a warm-up of about three years at the start, because a company needs a dozen quarters of
   profit history before any score can be computed.

## The maths, with every symbol named

The score is one subtraction and one division, repeated for every share.

The four-quarter change in earnings per share:

```text
d_q = EPS_q - EPS_(q-4)
```

- `d_q` is the change in earnings per share for quarter `q`, in currency units per share.
- `EPS_q` is the reported earnings per share for the most recent quarter.
- `EPS_(q-4)` is the reported earnings per share four quarters earlier, the same quarter one year
  ago, which is the market's simplest estimate of what to expect.

The scale of the company's usual surprises:

```text
sigma = SQRT( (1/8) * SUM( (d_j - mean_d)^2 ) )
```

- `sigma` is the standard deviation of the eight most recent quarterly changes.
- `d_j` is the change for each of those eight quarters, computed the same way as `d_q`.
- `mean_d` is the average of those eight changes.
- `SQRT` is the square root, and `SUM` adds up the eight quantities.

The standardized unexpected earnings score:

```text
SUE = d_q / sigma
```

- `SUE` is the score. A value of +2 means the latest change was twice the size of the company's
  typical change, in the direction of more profit; a value of -1 means it was one typical size below
  the recent norm.
- Dividing by `sigma` is what makes a small company with volatile profits comparable to a large,
  steady one. Without the division, the ranking would simply be a list of the companies whose
  profits jump around the most.

Then the selection and the return of the book:

```text
buy the shares with the highest SUE, enough of them to make up five percent of the list
w_i = 1 / (number of shares bought)
R_month = SUM( w_i * r_i )
Cost = t * c
```

- `w_i` is the fraction of the money placed in share `i`; all the bought shares get the same weight.
- `r_i` is the return of share `i` over the following month.
- `R_month` is the return of the whole book for the month.
- `t` is the traded fraction; the whole book is replaced only gradually, so `t` sits between 0 and
  2.0, and a month in which three of five holdings change costs about 1.2.
- `c` is the cost per side as a fraction of the amount traded, covering the bid-ask gap and any
  commission. For shares of this size a realistic figure is 0.001, that is ten basis points, where
  one basis point is one hundredth of one percent.

## A worked example

Twelve quarters of earnings per share for one company, in currency units. The eight changes, each
this quarter minus four quarters earlier, are on the right.

| Quarter | EPS  | Change (this quarter minus four quarters earlier) |
| ------- | ---- | ------------------------------------------------- |
| Q1      | 1.00 |                                                   |
| Q2      | 1.05 |                                                   |
| Q3      | 0.95 |                                                   |
| Q4      | 1.20 |                                                   |
| Q5      | 0.98 | -0.02                                             |
| Q6      | 1.30 | +0.25                                             |
| Q7      | 1.10 | +0.15                                             |
| Q8      | 1.05 | -0.15                                             |
| Q9      | 1.15 | +0.17                                             |
| Q10     | 1.25 | -0.05                                             |
| Q11     | 1.45 | +0.35                                             |
| Q12     | 1.60 | +0.55                                             |

The average of the eight changes is +0.15625. Squaring each change's distance from that average and
averaging gives a variance of 0.04613, so the standard deviation is the square root of that, 0.2148.
The most recent change is +0.55, so the score is 0.55 / 0.2148 = 2.56. This company has produced a
surprise a little more than twice as large as its usual one, in the good direction.

Now suppose the thousand-share universe has been ranked and the top eight candidates, highest score
first, are these. The top five percent of a hundred-share universe would be five shares, each with a
weight of one fifth.

| Share | SUE   | Selected | Next-month return | Contribution    |
| ----- | ----- | -------- | ----------------- | --------------- |
| A     | 2.56  | yes      | +3.0 percent      | +0.6000 percent |
| B     | 2.10  | yes      | +1.5 percent      | +0.3000 percent |
| C     | 1.80  | yes      | -0.5 percent      | -0.1000 percent |
| D     | 1.35  | yes      | +2.0 percent      | +0.4000 percent |
| E     | 1.10  | yes      | +0.8 percent      | +0.1600 percent |
| F     | 0.70  | no       |                   |                 |
| G     | 0.30  | no       |                   |                 |
| H     | -0.40 | no       |                   |                 |

The five selected shares gained 1.36 percent between them before costs. Suppose three of the five
change at the next monthly rebuild, so the traded fraction is `t = 2 * (3 / 5) = 1.2`, and the cost
at ten basis points per side is 1.2 * 0.001 = 0.0012, that is 0.12 percent. The net return for the
month is 1.36 - 0.12 = 1.24 percent. Twelve months at that rate is about 17.6 percent a year before
costs and 15.9 percent after, which is of the same order as the published figures below and is shown
only to check the arithmetic, not to forecast anything.

## What the research actually found

| Source                                                       | What it measured                                                                                    | Result                                                                                                                                                                                                                |
| ------------------------------------------------------------ | --------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Foster, Olsen and Shevlin, the study the anomaly dates from  | Returns after quarterly profit reports, US shares                                                   | Returns in the weeks after a report move in the direction of the surprise; this is the post-earnings drift the score is built on                                                                                      |
| Quantpedia, the post-earnings entry                          | A long-short strategy sorted on the profit surprise and the price reaction, US shares, 1987 to 2004 | About 15 percent a year in the source paper's own table, a worst fall of about 11.2 percent, with the validity of the anomaly rated Strong and about a thousand instruments needed (Quantpedia entry)                 |
| The source paper Quantpedia quotes                           | Profit-surprise and price-reaction sorts separately and together                                    | The profit-surprise sort alone earns about 6.25 percent a year more than nothing, the price-reaction sort about 7.55 percent more, and the two combined about 12.5 percent, in the paper's own summary of its results |
| QuantConnect, the implementation on the library page         | The top five percent by score, monthly rebuild, December 2009 to September 2019                     | A reward-for-risk figure of 0.602 against 0.43 for the index it compared with; the page's closing note reports 0.83 against 0.88 on a slightly different window                                                       |
| Hou, Xue and Zhang, Replicating Anomalies, cited by the page | Hundreds of published signals re-tested                                                             | The general finding is that many published anomalies shrink a great deal once the very smallest companies are excluded and returns are weighted by company size                                                       |

The picture is that the drift after a profit surprise is one of the older and more widely reported
patterns, and the QuantConnect implementation is one of the better-performing ones on this site. The
honest qualifications are the reason the grade comes with a caveat. The reward is concentrated in
smaller, less heavily traded companies, which is where the bid-ask gap and the effect of your own
trades on the price are both largest. The QuantConnect page's own suggested improvements point the
same way: use analyst estimates instead of the simple one-year-ago comparison, and consider smaller
companies, where the page says the drift is stronger but the trading is harder.

## How this project relates to it

This repository does not implement an earnings-surprise rule. The closest things are the brief
[Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md)
and the tutorial [Price and earnings momentum](../price-and-earnings-momentum/README.md).

The brief's most useful finding for this strategy is about how such a claim should be trusted. It
reports that the cross-sectional predictors surveyed in the literature are mostly genuine rather than
statistical accidents, but that the false-discovery rate for a claim rises sharply once returns are
weighted by company size and adjusted for known factors, from under 10 percent to about 42 percent.
An earnings-surprise rule is exactly the kind of fundamental signal that this warning is aimed at, so
the brief's recommended discipline applies: restate the result under value weighting and against a
factor model before treating it as a signal. The sibling tutorial takes the same profit-growth idea
and pairs it with price momentum, which is a different way of using the same underlying data.

## Where it goes wrong

- The score depends on the comparison you choose. Using the same quarter a year ago as the
  expectation is the simplest possible guess. If analysts usually forecast better than that, the score
  is measuring the wrong surprise, which is why both the paper and the page suggest analyst estimates.
- Small shares carry the reward and the risk. The drift is strongest where the companies are small
  and lightly traded, and those are the shares where the bid-ask gap is widest and a modest amount of
  money moves the price against you. A rule that looks strong before costs can vanish after them.
- Reporting dates are not aligned. Companies report at different times, so on any given day the
  scoring uses reports of different ages. A share whose news is three months old has already had its
  drift, and including it makes the signal look older or fresher than it is.
- The warm-up and the history matter. A company needs a dozen quarters of data, so young companies
  are excluded, and a company that restated its accounts will have a distorted history. Using a
  database of current members only, rather than the companies that actually existed at the time,
  flatters the result.
- Crowding and decay. The anomaly has been public since 1984 in some form. A rule that is easy to
  describe and trade on a screen attracts money, and the drift in the largest and most followed
  companies has had the most time to be competed away.
- What would have to be true for the idea to be false: that after a profit surprise, prices adjust
  fully within a day, so there is nothing left to drift. The measured record says that is not true,
  but it does not say how much remains after costs in the names you can actually buy.

## Try it yourself

You need a spreadsheet and, for a handful of well-known companies, the reported earnings per share
for the last dozen quarters plus the monthly share prices. Company investor-relations pages and any
finance website publish both.

1. Build one row per company and one block of columns holding EPS for the last twelve quarters, oldest
   on the left.
2. Add eight columns, one per quarter, computing that quarter's EPS minus the EPS four columns to its
   left. These are the changes.
3. Add a cell computing the standard deviation of the eight changes, and a cell computing the latest
   change divided by that standard deviation. That is the score.
4. Repeat for twenty or so companies, then sort the scores from highest to lowest.
5. Write down the top two and the bottom two. In a fresh set of columns, look up each company's return
   over the following month and average the top two, then the bottom two.
6. Do this for a year of monthly rebuilds, always using only the reports a reader would have had on
   that date, and subtract 0.10 percent per month for costs.

What to notice: the top of the list is often dominated by small or unusual companies, and the score
of a company that has recently reported can leap while its neighbours barely move. Compare the top
two with the bottom two over the year; the gap is the drift, and it is usually smaller than the
month-to-month noise. The most useful thing you will learn is how much of the score comes from the
denominator, the company's own wobble, rather than from the surprise itself.

## Where this came from

- [QuantConnect strategy library: standardized unexpected earnings](https://www.quantconnect.com/tutorials/strategy-library/standardized-unexpected-earnings),
  the universe, the score formula, the top five percent and the reported reward for risk.
- Foster, Olsen and Shevlin,
  [Earnings Releases, Anomalies, and the Behavior of Security Returns](https://www.jstor.org/stable/247321),
  the original study of the drift after a profit surprise.
- [Quantpedia: post-earnings announcement effect](https://quantpedia.com/strategies/post-earnings-announcement-effect),
  the restatement of the anomaly, the validity rating and the performance figures for the source
  paper.
- [Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  this repository's brief on which cross-sectional claims survive and how to test them.

## Words used in this tutorial

- basis point: one hundredth of one percent, so ten basis points is 0.10 percent.
- bid-ask gap: the difference between the price at which you can sell and the price at which you can
  buy, which is a cost every time you trade.
- earnings per share: a company's quarterly profit divided by the number of shares it has issued.
- standard deviation: how far a typical observation sits from the average of the observations.
- standardized: divided by a measure of the usual size of the thing, in order to compare large and
  small cases on the same scale.
- surprise: the part of a reported figure that differs from what was expected.
- weight: the share of your money placed in one holding.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
