# Gradient boosting: many tiny rules combining into one forecast for the next ten minutes

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                        |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| What it trades            | Units of a fund that tracks the whole American share index, held for ten minutes at a time                                                                                                                   |
| How often it trades       | Many times a day; a position is opened and closed within the session, and nothing is held overnight                                                                                                          |
| What you need             | Python and a data file                                                                                                                                                                                       |
| Where the rules come from | [QuantConnect strategy library, gradient boosting model](https://www.quantconnect.com/tutorials/strategy-library/gradient-boosting-model)                                                                    |
| The underlying research   | Zhou and colleagues (2013), a study of gradient boosting on intraday index data, named on the library page; the page does not link the paper, and no matching paper is in this repository's harvested corpus |
| How well it held up       | Weak: the library page's five-year backtest lost to simply holding the index, with a reward for risk of -0.649 against +0.691, and the extreme figure the method is credited with was not reproduced         |
| Also appears in           | nothing else in this collection                                                                                                                                                                              |

## The idea in one paragraph

A model is not one clever rule but a committee of dozens of very simple ones. Each rule in the committee
asks a single yes-or-no question about the chart, such as "is the very short average price above the
medium average price?" and gives a small answer either way. The model starts with a plain guess, the
average of what actually happened in the recent past, and then adds the committee's small answers one at
a time. After each addition it measures what is still wrong, and the next rule is built to reduce
exactly that remaining error. When the twenty rules have all spoken, their small adjustments add up to
one forecast: the return of the index over the next ten minutes. The rules buy when the forecast is
above the cost of trading, sell short when it is below minus that cost, and close the position after ten
minutes.

## Why anyone believed it

The chart indicators that traders watch, such as moving averages, contain a little information about the
next few minutes, but no single one is reliable. Combining many weak signals into one stronger forecast
is a well-established idea in statistics, and gradient boosting is a careful way of doing the combining:
each new small rule is chosen because it fixes part of what the previous ones got wrong. The
counterparty is other short-term traders and the automated systems that follow the same indicators, so
that the same small pressures repeat minute after minute. The page also cites a study reporting a very
high reward for risk, which is what made the idea look worth building.

## An everyday comparison

Think of a pilot's pre-flight checklist. No single item on it - fuel level, tyre pressure, a lamp
working - decides whether the flight is safe, and any one of them alone is a poor guide. But each item
catches a mistake the others miss, and a checklist built by adding the item that catches the most
remaining mistakes, one at a time, ends up far better than any single check. Gradient boosting builds
its committee exactly that way: it adds the one small rule that fixes the most of what is still wrong,
twenty times over. The forecast is the sum of all the ticks on the checklist.

## The rules, step by step

1. Subscribe to minute bars of the index fund, giving an open, high, low, close and volume for each
   minute of the trading session.
2. Compute a set of technical indicators from those bars. The page uses pairs of moving averages, which
   are just the average price over a short and a longer window, taken across a range of window sizes
   from 0.5 up to 5 and back down in small steps.
3. At the end of each month, gather the previous four weeks of bars together with their indicators. For
   each bar, also compute the answer it should have predicted: the percentage change in price over the
   following ten minutes.
4. Fit a gradient boosting model made of twenty stumps. A stump is a rule with a single yes-or-no split,
   so it has exactly two possible answers, one for each side of the split.
5. The model starts from a single number: the average of the answers in the four weeks of training data.
6. Each stump is then fitted to what the current model still gets wrong, and its output is added to the
   running forecast scaled down by a learning rate, a fixed fraction that keeps each step small.
7. Repeat until all twenty stumps have been added. The forecast is the starting average plus the
   scaled-down sum of all twenty stump outputs.
8. Every minute, compute the indicators and ask the model for the next ten-minute return.
9. Buy if the forecast is greater than the combined cost of 0.05; sell short if it is less than minus
   0.05; otherwise stay out of the market. The 0.05 is the library page's sum of a commission of 0.02
   and a spread cost of 0.03, in the same units as the forecast return.
10. Close the position after ten minutes and stop trading before the market closes, so that nothing is
    held overnight. Refit the whole model at the end of the next month.

## The maths, with every symbol named

The model is judged during training by how far its forecast is from the answer, on average:

```text
MSE = average of (y - yhat)^2
```

- `y` is the actual next-ten-minute return for one training bar.
- `yhat` is what the model forecast for that bar.
- `MSE` is the mean squared error, the average of the squared misses; squaring makes a big miss count
  much more than several small ones, and the library page uses it as the fit measure.

The committee is built one rule at a time. Starting from a plain average and adding residuals:

```text
F0 = average of the answers in the training set
r = y - F_previous
F_new = F_previous + learning_rate * h
```

- `F0` is the starting forecast, one number, the average of the actual answers.
- `r` is the residual: what is still wrong after the rules added so far.
- `h` is the output of the new stump, chosen to follow `r` as closely as it can.
- `learning_rate` is the fraction each stump's output is scaled by, so no single rule can move the
  forecast very far.

After all twenty rules, the forecast is their sum:

```text
F_final = F0 + learning_rate * (h1 + h2 + ... + h20)
```

- `h1` to `h20` are the twenty stumps, each one a single yes-or-no split with two possible outputs.
- The sum of twenty small numbers is the whole forecast; no single stump is meant to matter much.

The trading decision and result follow from the forecast and the cost:

```text
long if F_final > 0.05; short if F_final < -0.05; otherwise nothing
net return = position * actual_return - cost
```

- `0.05` is the combined cost of commission and the gap between the buying and selling price, in the
  same units as the forecast. The library page prints 0.05 but does not name the units.
- `position` is +1 for a long bet and -1 for a short bet.
- `actual_return` is the real next-ten-minute return over the ten minutes the position is held.

## A worked example

The rows below are invented, but they use the shape of the library page's settings: a starting average,
a learning rate, and two stumps whose outputs are added. Suppose the training set's average answer is
plus 0.01 percent and the learning rate is 0.5. For the second row, the first stump says plus 0.20 and
the second says plus 0.02, so the forecast is 0.01 + 0.5 * (0.20 + 0.02) = 0.01 + 0.11 = 0.12 percent.
The other forecasts are given the same way. The cost is the page's 0.05, charged once per round trip,
and the position is held for ten minutes.

| Minute | Forecast (%) | Decision | Actual next 10 minutes (%) | Gross (%) | Cost (%) | Net (%) |
| ------ | ------------ | -------- | -------------------------- | --------- | -------- | ------- |
| 1      | +0.02        | nothing  | +0.07                      | 0.00      | 0.00     | 0.00    |
| 2      | +0.12        | buy      | +0.30                      | +0.30     | -0.05    | +0.25   |
| 3      | +0.08        | buy      | -0.10                      | -0.10     | -0.05    | -0.15   |
| 4      | -0.15        | sell     | +0.05                      | -0.05     | -0.05    | -0.10   |
| 5      | -0.20        | sell     | -0.25                      | +0.25     | -0.05    | +0.20   |
| 6      | +0.03        | nothing  | -0.04                      | 0.00      | 0.00     | 0.00    |
| 7      | +0.11        | buy      | +0.02                      | +0.02     | -0.05    | -0.03   |
| 8      | -0.13        | sell     | -0.08                      | +0.08     | -0.05    | +0.03   |

Minutes 1 and 6 produce no trade because the forecast did not clear the 0.05 cost. The gross column adds
to +0.50 percent, the six costs add to -0.30 percent, and the net column adds to +0.20 percent. Three of
the six bets won, and the strategy came out slightly ahead after cost in this invented run.

The important arithmetic is at minute 7. The model was right about the direction - the price did rise -
but it rose by only 0.02 percent, less than the 0.05 percent it cost to take the bet. A correct call
that is smaller than the cost is still a loss. That single row is the reason a high hit rate does not
guarantee money, and it is why the rules refuse to trade at all on minutes 1 and 6.

## What the research actually found

| Source                                               | What it measured                                                              | Result                                                                                                                                                                             |
| ---------------------------------------------------- | ----------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| QuantConnect library page, gradient boosting model   | Its own five-year backtest on the index fund                                  | Reward for risk of -0.649 against +0.691 for holding the index; in the 2020 crash, -2.688 against -1.467; in the recovery, -2.083 against +7.942                                   |
| The study the page cites, Zhou and colleagues (2013) | A gradient boosting model on intraday index data, as reported by the page     | A reward for risk above 20, which the page says it could not reproduce; the page also reports that the study's profit-based training objective, reapplied, produced poor forecasts |
| `2502.15822v1` (pp.2, 4)                             | A boosted model on card-fraud data where fraud is about 0.2 percent of rows   | Accuracy of 99.72 percent and a separation score of 0.987, figures that look excellent but are close to what a model that predicts "not fraud" every time would score on such data |
| `2602.06198v1` (abstract)                            | A gradient boosting classifier on insider purchase filings in small companies | Separation score of 0.70 and precision of 0.38 on unseen data; an honest, modest result rather than a near-perfect one                                                             |
| `2110.14914v2` (pp.5, 7, 8)                          | Classifiers calling the next move on futures, then traded                     | 52 to 56 percent accuracy, yet many configurations were unprofitable once normal slippage was charged                                                                              |

The page's own result is the headline and it is negative: over five years the rule made less than the
index after the wobble was counted, and during the 2020 recovery it did far worse than simply holding. A
reward for risk above 20, as the cited study reported, is so far outside the range that real strategies
produce that the page's failure to reproduce it is not surprising. The fraud paper is included for the
arithmetic rather than the topic: it shows how a near-perfect accuracy score can be produced by a
lopsided dataset, because when almost every row is the same answer, a model that always gives that
answer scores almost perfectly. The insider study shows what an honest gradient boosting result looks
like - useful separation, plenty of false alarms, and precision well below one.

## How this project relates to it

- [Machine Learning for Trading](../../../strategies/books/09_machine_learning_for_trading.md)
  is the closest match. Section 6 on costs, turnover and loss design is the one to read here: the page
  trains on the squared miss and trades a ten-minute horizon, and that brief's takeaway is to train on
  the trading objective instead, because a model that ranks prices accurately can still have no
  directional skill.
- [Machine learning for finance](../../../strategies/books2/04_machine_learning_for_finance.md)
  documents the loss-function experiment and the finding that a model can be accurate and still carry no
  directional information, which is exactly the situation minutes 1 and 6 and minute 7 illustrate.
- [Overfitting and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md)
  is the brief for the honest accounting: it records that mining many variables yields thousands of
  strategies of which a large fraction clear an ordinary significance bar by chance, and it lists what a
  research protocol must print, including the trial count and the held-out period.

The repository does not contain a runnable copy of this model. The nearest implemented thing is the
validation discipline those briefs describe, which is what a reader would use to judge the code if the
model were ported here.

## Where it goes wrong

- Overfitting the recent past. The model is refitted every month on the previous four weeks, so it
  learns whatever the market was doing during those four weeks. When the market's behaviour changes, the
  model is confidently fitted to a world that no longer exists. Boosting makes this easy to do: add
  enough stumps and the training error goes to zero while the forecasts get worse.
- A high accuracy score can still lose. The fraud paper shows the arithmetic of a lopsided sample, where
  always answering the common outcome scores almost perfectly. Here the sample is not lopsided, but the
  same trap applies in a different form: a model can be right about direction most of the time and still
  lose, because the moves it is wrong on are larger than the moves it is right on.
- Direction is not money. Minute 7 of the worked example is the whole point: a correct call of plus
  0.02 percent does not pay a 0.05 percent cost. The library page's own rule, which trades only when the
  forecast is larger than the cost, is the correct response to this, and it is worth noticing how much
  of the day it filters out.
- The cost figure is not explained. The page prints a commission of 0.02 and a spread cost of 0.03 and
  uses their sum as the bar a forecast must clear, but does not name the units. A reader trying to
  reproduce the result must resolve that, and any change to the cost changes whether the strategy
  trades at all.
- The horizon is very short. Ten minutes is close to the noise floor of a liquid index fund, where the
  typical move is comparable to the gap between the buying and selling price. The shorter the horizon,
  the smaller the forecast edge, and the more of it the cost consumes.
- The study's own objective was replaced. The page reports that when it trained the model using the
  study's profit-based objective, the forecasts became poor. That is a reminder that a method can rest
  on a choice - here, what the model is trained to minimise - that the headline result depends on and
  the page does not settle.

## Try it yourself

You need a spreadsheet and a public source of minute bars for the index fund. No money and no code are
involved.

1. Collect one week of minute bars: the time in column A, the closing price in column B, and the volume
   in column C.
2. In column D, compute a short moving average of the closing price, the average over the last five
   minutes at each row.
3. In column E, compute a longer moving average, the average over the last twenty minutes at each row.
4. In column F, subtract the longer average from the shorter one, so a positive value means the recent
   price is above its slower average.
5. In column G, compute the return over the following ten minutes: the close ten rows down divided by
   the current close, minus one.
6. In a spare cell, compute the average of column G but only for the rows where column F is positive,
   and in another cell the average of column G only for the rows where column F is negative.
7. Count how many rows are in each group.

What to notice: the two averages in step 6 will be close to zero, and usually opposite in sign to what
the indicator would suggest you to expect, because a positive value in column F often comes after the
price has already moved up. Compare the two averages with the cost of a round trip, about 0.02 percent of
the price for a fund this liquid. Unless one average is clearly larger than that bar, the indicator has
no forecast worth trading. Run the same exercise across several different weeks and see how often the
sign of the difference flips; that flipping is the reason the library page's own backtest lost to simply
holding the index.

## Where this came from

- [QuantConnect: gradient boosting](https://www.quantconnect.com/tutorials/strategy-library/gradient-boosting-model),
  the rules as implemented: minute bars of the index fund, twenty stumps, a monthly refit on four weeks
  of data, a ten-minute holding period, and a cost of 0.05 to be cleared before trading. The research
  page is at [research/15270](https://www.quantconnect.com/research/15270/gradient-boosting-model/).
- Zhou and colleagues (2013), the study the page cites for the reward-for-risk figure above 20. The page
  does not link the paper and it is not in the repository's harvested corpus, so it is named but not
  quoted.
- `2502.15822v1`, "Financial fraud detection system based on improved random forest and gradient boosting
  machine (GBM)", the source for the 99.72 percent accuracy and 0.987 separation score on data where
  fraud is about 0.2 percent of rows.
- `2602.06198v1`, "Insider Purchase Signals in Microcap Equities: Gradient Boosting Detection of
  Abnormal Returns", the source for the modest separation score of 0.70 and precision of 0.38.
- `2110.14914v2`, "Trading via Selective Classification", the source for the accuracy-versus-cost
  finding.
- [Machine Learning for Trading](../../../strategies/books/09_machine_learning_for_trading.md),
  [Machine learning for finance](../../../strategies/books2/04_machine_learning_for_finance.md) and
  [Overfitting and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md),
  this repository's own briefs on the questions this model raises.

## Words used in this tutorial

- accuracy: the share of predictions that were correct, out of all predictions made.
- base rate: how common the answer you are trying to find is in the data, before any model is applied.
- gradient boosting: building one forecast out of many small rules, each added to fix what the previous
  ones got wrong.
- learning rate: the fraction each small rule's output is scaled by, keeping each step modest.
- mean squared error: the average of the squared differences between forecasts and answers.
- overfitting: a model that has memorised the past it was shown rather than learned a pattern that
  carries into new data.
- residual: what is still wrong after the rules added so far, which the next rule is built to reduce.
- reward for risk (Sharpe ratio): the average return divided by how much it wobbled, usually written per
  year; zero means no reward after the wobble is counted.
- stump: a rule with a single yes-or-no split, giving one of two answers.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
