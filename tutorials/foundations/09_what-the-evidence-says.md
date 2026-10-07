# What the evidence says

Date: 2026-10-07. Revision 1.

Most trading strategies are published in a paper and then tested by other people. This page reports
the largest public attempt to test the whole published record, explains each number in plain words,
and says what a reader should do with it.

## What was measured

One vendor, paperswithbacktest, coded and ran 4,843 published strategies over their own full price
history and published the results on its
[awesome-systematic-trading list](https://github.com/paperswithbacktest/awesome-systematic-trading).
The numbers below come from that list and the method page it links to. They are one organisation's
measurements of its own replication run, not a peer-reviewed study, and they cover the strategies
that list chose to code rather than every strategy ever published.

## The six numbers, in plain words

| What was measured                                     | The number                               | What it means in plain words                                                                                                       |
| ----------------------------------------------------- | ---------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- |
| Median replication Sharpe ratio                       | 0.37                                     | The typical strategy earned a small return for the amount it wobbled, and half did worse.                                          |
| Strategies clearing a t-statistic of 1.96             | 48 percent                               | Half the strategies cannot be told apart from luck on their own sample.                                                            |
| Median test window                                    | 34 years                                 | The typical strategy was judged over a long stretch of prices, not a few months.                                                   |
| Median exposure to the wider market                   | beta of +0.17                            | The typical strategy rose and fell a little with the market, so part of its return was simply being invested.                      |
| Median information ratio after removing that exposure | 0.21                                     | Once the market's contribution is taken out, what is left is a very thin margin.                                                   |
| Measurable decay after publication                    | none within 0.2 percentage points a year | Over 2,838 papers with data on both sides of publication, the vendor could not detect the usual claim that a published edge fades. |

Read the table as a set of five facts about the typical strategy, not about the best one. The
median is the middle result, so half of the 4,843 strategies did better than the figures shown and
half did worse, and the strongest and weakest are not represented at all.

The rest of this section explains the words in that table. A Sharpe ratio is the average return of
something divided by the amount it typically wobbles: a value of 1 means the return is as large as
the wobble, and 0.37 means the return is smaller than the wobble. A t-statistic is a score that
says how far a measured result sits from pure chance; the conventional line is 1.96, and a value
below it is usually called indistinguishable from zero. Beta measures how much a portfolio moves
when the wider market moves by one; a beta of +0.17 means it travels with the market a little. An
information ratio is the same idea as the Sharpe ratio but measured against a comparison index
instead of against cash, so removing the market's contribution leaves the manager's own margin.
Decay is the process of an edge getting smaller over time, and the vendor's limit of 0.2
percentage points a year means its measurement could not see anything larger than a fifth of one
point per year.

## Why a small Sharpe ratio needs a long window

A Sharpe ratio of 0.37 does not sound large, and the arithmetic shows why it takes decades to
trust. A result needs roughly `(1.96 / Sharpe) squared` years of data for the t-statistic to reach
1.96 by chance alone, which is the list's own stated rule. For a Sharpe ratio of 0.4 that is about
24 years; for 0.37 it is about 28. The median strategy was measured over 34 years, just long enough
for a small edge to reveal itself, which is why the vendor reports the window at all. A strategy
with a low Sharpe ratio and only three years of history cannot be judged, no matter how attractive
its past curve looks. The formula also explains why a very small edge needs a very long record: at
a Sharpe ratio of 0.2 the required window is about 96 years, longer than any reliable price
history that exists.

## What these numbers do not prove

The measurements are one vendor's, taken on its own coding of the papers it selected. The method
and caveats live on the vendor's wiki, and no outside group has repeated the exercise. A median
hides a wide spread: 48 percent of strategies failed to clear the chance line, which means 52
percent did clear it, and the strong ones may be genuinely strong. "No measurable decay" is a
statement about the limit of a measurement, not proof that decay is zero; something smaller than a
fifth of a point a year could still be there. The record also says nothing about strategies that
were never published, and a strategy that failed badly is less likely to reach a paper in the
first place, so the sample may tilt toward the survivors. And a coded strategy is a
reimplementation, so part of any difference between the paper and the replication is a difference
in how the rules were translated into code.

## What a reader should take from it

Three practical readings follow, and all three are uncomfortable for anyone who wants a simple
answer.

- Most published strategies cannot be distinguished from luck. Half of them sit below the
  conventional chance line even on the sample the authors chose.
- Part of the published edge is just being exposed to the market. A beta of +0.17 means some of
  the return would have arrived anyway from holding the wider index, and after removing it the
  median margin is 0.21, which costs can easily erase.
- A claim needs three things before it is worth anything: the costs it includes, evidence from
  data its rules never saw, and a named counterparty who is on the other side and who has a reason
  to keep trading. The overfitting and research integrity brief explains why a claim that states
  none of these is close to unfalsifiable
  ([that brief](../../strategies/books2/28_overfitting_and_research_integrity.md)).

The uncomfortable conclusion is also the useful one. The published record is not a menu of free
money; it is a large collection of small, uncertain effects, most of which are too thin to trade
after costs.

A reader who keeps these numbers in mind will read every strategy page differently: the question
stops being "does this work" and becomes "how much of this survives when the costs, the market
exposure and the number of tries are put back in". That reframing is the whole point of the
foundations pages.

## Words used in this tutorial

- Sharpe ratio: average return divided by the size of the typical wobble; a small number means the
  return is small next to the risk taken.
- t-statistic: a score for how far a measured result sits from pure chance; 1.96 is the customary
  line.
- Beta: how much a portfolio tends to move when the wider market moves by one.
- Information ratio: like the Sharpe ratio, but measured against a comparison index instead of
  against cash.
- Decay: the shrinking of an edge over time.
- Out-of-sample: data kept aside and not used when the rules were chosen, so they can be tested
  fairly.
- Median: the middle value when all the results are lined up, with half above and half below.

See [GLOSSARY.md](../GLOSSARY.md) for the rest of the vocabulary used in this collection.

## Where this came from

- [awesome-systematic-trading](https://github.com/paperswithbacktest/awesome-systematic-trading),
  the vendor's list, which reports the Sharpe ratio, the t-statistic share, the 34-year window,
  the beta, the information ratio and the decay figure, and links its method on its wiki.
- [The overfitting and research integrity brief](../../strategies/books2/28_overfitting_and_research_integrity.md),
  on multiple testing, trial counts and what a research protocol has to record.
- [The previous foundations page](08_regimes-and-why-nothing-lasts.md), on why an edge can stop
  working even when nothing about the strategy changes.
- [GLOSSARY.md](../GLOSSARY.md), the shared list of terms.
