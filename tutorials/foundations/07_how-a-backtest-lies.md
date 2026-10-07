# How a backtest lies: the standard ways a history can fool you

Date: 2026-10-07. Revision 1.

A backtest is a rehearsal of a strategy on prices that already happened, and it is the first thing a
beginner builds. This page explains what a backtest actually does, the standard ways it can mislead,
and two arithmetic facts that tell you how much to trust one.

## What a backtest is

A backtest takes a set of past prices and asks a simple question: if these rules had been followed
over that history, what would have happened? The rules are applied mechanically to each past day or
minute, the trades are recorded, and the gains and losses are added up. The output looks like a
statement, but it is not a record of anything that occurred. It is a simulation of a strategy that
nobody actually ran, on prices chosen after the fact. Everything it reports depends on the rules it
was given and the data it was given, and it can be made to look good simply by changing either.

A rule might be stated in words like "buy when the price has risen for five days in a row, and sell
when it has fallen for five days in a row". A backtest walks through the history and executes that
sentence mechanically wherever it applies, then adds up the result. The reader never sees the
sentence being tested; they see one final number, such as "this returned 12 percent a year", with no
information about how many versions of the sentence were tried or what the trades cost. That gap
between the visible number and the hidden process is where most of the deception lives.

## The standard ways a backtest deceives

There is a short list of well-known ways a backtest can mislead, and most of them are mistakes in how
the test was set up rather than mistakes in the arithmetic.

- Overfitting. This means tuning the rules again and again until they fit the past. Each adjustment
  makes the historical result a little better and the rule a little more tailored to quirks that will
  never repeat. The rule ends up describing the past instead of predicting anything.
- Trying many variations and reporting the best one. If you test a thousand rule sets and show only
  the winner, the winner looks impressive but is partly a matter of luck. Testing 240 accounting
  variables in every combination produced 18,113 strategies, and 30.17 percent of them cleared the
  standard significance bar while under pure noise only about 4.55 percent would
  (arXiv `2209.13623v3`).
- Look-ahead bias. This means using information at a past moment that was not actually available
  then, for example acting on a day's closing price before the day had closed. The backtest then
  knows the future and reports profits that could never have been earned.
- Survivorship bias. This means testing on a data set that contains only the companies that survived
  to today. The failures, which a real trader would have held and lost money on, were quietly
  removed, so the test is flattering for no trading reason.
- Ignoring costs. A backtest that leaves out the spread, the commission and the other charges reports
  a gross result. As the companion page shows, a strategy trading twenty times a year can pay four
  percent in costs and turn a four percent gross year into nothing; see
  [06_costs-fees-and-taxes.md](06_costs-fees-and-taxes.md).
- Assuming every order fills at the price on screen. A backtest often fills each order instantly at
  the last traded price. In reality you may get a worse price, or no fill at all, especially for a
  large order or a thin market.

## In-sample, out-of-sample and walk-forward testing

The cure for most of the list above is to stop testing the rules on the same data that shaped them.
The data a rule was designed on is called in-sample (inside the sample). Fresh data that the rule has
never seen is called out-of-sample (outside the sample). A rule that works in-sample but fails
out-of-sample was fitted to noise.

Walk-forward testing is the practical version of this. You fix the rules using one stretch of history,
then check them on the next stretch without changing anything, then move both windows forward and
repeat. This mimics the real sequence in which you would have learned a rule and then used it, and it
exposes a rule that only works when it can see its own answers. Even a walk-forward test can be
overfit, if the person running it tries enough rules and keeps the best, so the number of rules tried
must be reported alongside the result.

A concrete sequence makes the idea vivid. Suppose you design the rules using the years 2000 to 2009,
then test them untouched on 2010 to 2014, then design again using 2005 to 2014 and test on 2015 to
2019, and so on. At every step the rules are frozen before the test, so a rule that only works when
it can see its own answers is caught. A plain in-sample plus one out-of-sample split is weaker but
still far better than testing on everything at once.

## What a t-statistic of 1.96 means, and why half of published results fail it

A t-statistic (a "t") is a way of asking how far a measured result sits from the result you would
expect if there were no real effect at all. Divide the measured effect by the uncertainty around it
and you get a t. A t of zero means the result is indistinguishable from nothing; a larger t means the
result stands further from nothing.

The number 1.96 is the conventional bar. If the true effect were exactly zero, a measured t of 1.96
or more in either direction would show up by pure chance only about 5 percent of the time. So a t
above 1.96 is treated as "unlikely to be luck". It is a threshold, not proof, and the finance
literature disagrees on how high it should be. One review reports that the 1.96 bar implies a false
discovery rate of 8.8 percent, and that most published predictors clear t = 2.0 comfortably
(arXiv `2209.13623v3`). Other work reaches the opposite conclusion once multiple testing is counted.

The reason people say roughly half of published results fail the bar is visible when someone checks.
Hou, Xue and Zhang (2020) tried to reproduce 452 published stock-market patterns and found that about
65 percent of them did not clear t = 1.96 once the test was made stricter
([Replicating Anomalies](https://global-q.org/uploads/1/2/2/6/122679606/houxuezhang2020rfs.pdf)).
The disagreement between that measurement and the review above is unresolved, and it is reported here
rather than settled. For this repository's research brief on the subject, see
[strategies/books2/28_overfitting_and_research_integrity.md](../../strategies/books2/28_overfitting_and_research_integrity.md).

## How much data a strategy needs

Two quantities tell you how long a track record must be before it means anything. The Sharpe ratio
measures return per unit of wobble: the yearly return divided by how much that return bounces around.
A strategy returning 6 percent a year while swinging by 15 percent has a Sharpe of about 0.4. A higher
Sharpe is a smoother, steadier result.

For a long stretch of data the t-statistic follows a simple rule.

```text
t = Sharpe x square root(years)
```

- `t` is the t-statistic described above.
- `Sharpe` is the yearly return divided by the yearly wobble.
- `years` is the length of the track record in years.

Rearranged to find how long you must wait for a t of 1.96, the rule becomes
`years = (1.96 / Sharpe)^2`. Put a Sharpe of 0.4 into it: `(1.96 / 0.4)^2 = 4.9^2 = 24.01`. So a
strategy with a Sharpe ratio of 0.4 needs roughly 24 years of data before its result is
distinguishable from luck at the conventional bar. A backtest run on only a few years therefore
cannot yet tell a modest edge from chance. This is the arithmetic behind the research brief's warning
that a strategy search should record how many rules were tried and over what dates
([strategies/books2/28_overfitting_and_research_integrity.md](../../strategies/books2/28_overfitting_and_research_integrity.md)).

## Words used in this tutorial

- Backtest: a rehearsal of a strategy's rules on past prices to see what would have happened.
- In-sample: the stretch of history a rule was designed on.
- Out-of-sample: history a rule has never seen, used to check whether it works.
- Walk-forward test: fixing rules on one stretch of history, checking them on the next, and repeating
  with both windows moved forward.
- Overfitting: tuning rules until they fit the past's quirks instead of a repeatable effect.
- Look-ahead bias: using information at a past moment that was not available at that moment.
- Survivorship bias: testing on a data set that keeps only the companies that survived to today.
- t-statistic: a measure of how far a result sits from nothing, in units of its own uncertainty.
- Sharpe ratio: yearly return divided by how much the return wobbles, a measure of return per unit of
  risk.

## Where this came from

- [GLOSSARY.md](../GLOSSARY.md), the shared list of trading terms used across this collection.
- [strategies/books2/28_overfitting_and_research_integrity.md](../../strategies/books2/28_overfitting_and_research_integrity.md),
  this repository's research brief on overfitting, replication and how a strategy search should be
  recorded.
- [Replicating Anomalies](https://global-q.org/uploads/1/2/2/6/122679606/houxuezhang2020rfs.pdf),
  Hou, Xue and Zhang (2020), the source of the about-65-percent figure for published patterns that
  fail t = 1.96.
- `2209.13623v3`, a review of asset-pricing replication that reports the 1.96 bar implies an 8.8
  percent false discovery rate and that most published predictors clear t = 2.0. Link:
  https://arxiv.org/abs/2209.13623.
- The twenty-trade cost arithmetic and the 0.4-Sharpe example are made up for teaching; the 24-year
  figure is the result of the formula shown, not a measurement.
