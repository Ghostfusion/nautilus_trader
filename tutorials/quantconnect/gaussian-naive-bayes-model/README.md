# Gaussian naive Bayes: guessing whether tomorrow will be up or down from the shape of recent returns

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                        |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Ten of the largest technology shares, held in equal amounts whenever a bet is on                                                                                                             |
| How often it trades       | Once a day, from one morning's opening price to the next morning's opening price                                                                                                             |
| What you need             | Python and a data file                                                                                                                                                                       |
| Where the rules come from | [QuantConnect strategy library, Gaussian naive Bayes model](https://www.quantconnect.com/tutorials/strategy-library/gaussian-naive-bayes-model)                                              |
| The underlying research   | Lu (2016) and Imandoust and Bolandraftar (2014), named on the library page as the studies it follows; neither is in this repository's harvested corpus                                       |
| How well it held up       | Weak: the library page's five-year backtest scored a reward for risk of 0.011 against 0.729 for holding the index, and its only good stretch, the 2020 crash, was given back in the recovery |
| Also appears in           | nothing else in this collection                                                                                                                                                              |

## The idea in one paragraph

For each share, look at the last four daily returns, where a daily return is the change from that
morning's opening price to that afternoon's closing price. Then ask how often, in the recent past, a
similar set of four returns came just before a day that ended higher, and how often it came before a
day that ended lower. The method turns those counts into a probability that tomorrow will rise. If the
probability is above 55 percent the rules buy the share, if it is below 45 percent they sell it short,
and in between they do nothing. When several shares pass the filter at once, the money is split equally
between them. Every position is held from one morning's open to the next morning's open, and the whole
thing is repeated the next day.

## Why anyone believed it

The method is famous from an unrelated problem: sorting email into spam and not-spam. There, each word
in a message shifts the odds a little, and multiplying the shifts together gives a good answer even
though the words are treated as if they never appeared together. The same machinery applies to returns:
each of the last four days shifts the odds that tomorrow is up, and the shifts are multiplied. The
person on the other side is the trader who reacts to the most recent day's move and to nothing else, and
whose predictable reaction the four-day pattern is meant to be ahead of. The appeal is that the method
is transparent - you can read the average and the spread of the up days and the down days directly off
the training data - and it needs very little data to be fitted.

## An everyday comparison

Think of a doctor diagnosing from symptoms. A cough on its own means little; a fever on its own means a
little; a rash on its own means something else. But each symptom shifts the odds of each illness, and a
doctor who multiplies those shifts together, one symptom at a time, arrives at a considered guess. The
doctor is not treating the symptoms as independent - a fever and a rash often come from the same cause -
but the multiplication still works well enough to be useful. Naive Bayes does the same thing with four
days of returns, and the word "naive" in its name is an honest admission that it multiplies as if the
days had nothing to do with each other.

## The rules, step by step

1. Choose the ten largest technology shares by company value. The page rebuilds this list once a month.
2. For each share, and for each of the past 100 trading days, compute that day's open-to-close return:
   the closing price divided by the opening price, minus one.
3. For each of those days, also compute the answer it should predict: the return from the next day's
   opening price to the day after's opening price. Label the answer "up" if that return is positive,
   "down" if it is negative, and "flat" if it is exactly zero.
4. Store, for each share, the last four daily open-to-close returns as its features, and the label as
   the answer. One hundred such feature-and-answer pairs make the training set.
5. From the training set, compute the average and the spread of the feature values separately for the up
   days, the down days and the flat days. This is what the word "Gaussian" adds: each feature within
   each class is summarised by a bell-shaped curve with its own centre and width.
6. To predict, take today's four returns. For each class, ask how typical those four values are,
   according to that class's curves, and multiply the four answers together. Multiply again by how
   common the class was in training. Whichever class ends up with the largest number is the prediction.
7. Buy the share if the chance of "up" is above 55 percent, sell it short if the chance of "down" is
   above 55 percent, and do nothing otherwise. The page instead uses the single most likely class, with
   no threshold; the threshold version below is the same idea with a safety margin.
8. Split the money equally between all the shares that passed the filter, and hold from today's open to
   the next morning's open.
9. Retrain whenever the list of shares changes, using the most recent 100 days.

## The maths, with every symbol named

Everything rests on one rearrangement of a rule about conditional chances:

```text
posterior = prior * likelihood / evidence
```

- `prior` is how common a class was in the training data: if 55 of 100 days were up, the prior for "up"
  is 0.55.
- `likelihood` is how typical the observed feature values are for that class, read off the bell curve.
- `evidence` is how common the observed feature values are overall, the same for every class.
- `posterior` is the updated chance of the class after seeing the features.

Because the evidence is the same for every class, it can be dropped when comparing them:

```text
score(class) = prior(class) * likelihood(feature 1) * ... * likelihood(feature 4)
answer = the class with the largest score
```

- `score` is a number used only for ranking the classes; it is not itself a probability.
- Multiplying the four likelihoods together is the "naive" step: it treats the four days as if they were
  unrelated to one another.
- `answer` is whichever class ends up on top after the multiplication.

A "Gaussian" likelihood is just a bell curve, and its shape is set by two numbers read off the training
data, the average and the spread:

```text
likelihood(value, class) = (1 / (spread * square root of 2 * pi)) * exp( - (value - average)^2 / (2 * spread^2) )
```

- `average` is the mean of that feature over the training days of that class.
- `spread` is how far the values sit from that average, measured in the same units as the feature.
- `exp` is the exponential function; the part inside it is always zero or negative, so the likelihood is
  largest when the observed value equals the class average and falls away smoothly on both sides.
- Two classes always share the same leading factor when their spreads are equal, so the factor cancels
  and only the exponential part decides which class wins.

Finally, turning the score into a decision and a result:

```text
P(up) = score(up) / (score(up) + score(down))
weight = 1 / number of shares passing the filter
net return = weight * sign(bet) * actual_return - cost
```

- `P(up)` is the share of belief placed on "up" once the two classes are put side by side; the flat class
  is left out here for clarity.
- `weight` is the fraction of the account placed on each chosen share, so ten names means one tenth each.
- `sign(bet)` is +1 for a long bet and -1 for a short bet.
- `actual_return` is the return that actually happened from open to open.

## A worked example

The training summary below is invented, and it is deliberately simple: one feature instead of four, ten
up days and ten down days, so the arithmetic can be followed. Up days averaged plus 0.50 percent with a
spread of 1.00 percent; down days averaged minus 0.50 percent with the same spread of 1.00 percent. The
prior is therefore 0.5 for each, since ten days of each class were in the training set. Today's return
is plus 0.30 percent.

The leading factor of the bell curve is the same for both classes, so it cancels when they are compared.
What is left is the part inside the exponential, which is minus the squared gap from the class average,
divided by twice the squared spread:

| Class | Gap from average    | Squared gap | Divided by 2 * spread^2 | Score ratio versus the other class |
| ----- | ------------------- | ----------- | ----------------------- | ---------------------------------- |
| up    | 0.30 - 0.50 = -0.20 | 0.04        | 0.04 / 2.00 = 0.020     | exp(0.30) = 1.35                   |
| down  | 0.30 + 0.50 = 0.80  | 0.64        | 0.64 / 2.00 = 0.320     | (the other side of the ratio)      |

The up class's score is exp(-0.020) and the down class's score is exp(-0.320). Dividing one by the other
gives exp(-0.020 + 0.320) = exp(0.30), which is 1.35. The up class is 1.35 times as likely as the down
class given this one observation. With equal priors, the chance of up is 1.35 / (1.35 + 1) = 0.574,
which is 57.4 percent, above the 55 percent line, so the rules buy.

Now run that logic over six days on an account that holds ten shares, so that the share followed here is
worth 10,000 dollars. The returns, the cost of 0.04 percent per round trip (two basis points per side),
and the outcomes are invented for the example; the arithmetic is what matters.

| Day | Today's return | Chance of up | Decision | Next-day return | Gross | Cost | Net  |
| --- | -------------- | ------------ | -------- | --------------- | ----- | ---- | ---- |
| 1   | +0.30%         | 0.574        | buy      | +0.40%          | +$40  | -$4  | +$36 |
| 2   | +0.80%         | 0.690        | buy      | -0.50%          | -$50  | -$4  | -$54 |
| 3   | -0.20%         | 0.450        | nothing  | +0.10%          | $0    | $0   | $0   |
| 4   | -0.60%         | 0.354        | sell     | -0.70%          | +$70  | -$4  | +$66 |
| 5   | +0.10%         | 0.525        | nothing  | -0.30%          | $0    | $0   | $0   |
| 6   | +0.55%         | 0.634        | buy      | +0.20%          | +$20  | -$4  | +$16 |

The gross column adds to +80 dollars, the four costs to -16 dollars, and the net column to +64 dollars,
which is 0.64 percent of the 10,000 dollars set aside for this share over six days. Two rows carry the
lesson. On day 2 the chance of up was the highest in the table, 0.690, and the day fell anyway: the
model's confidence was built from a return of plus 0.80 percent, which is unusual, and unusual days are
exactly where the bell curve's assumptions are least trustworthy. On day 4 the model called a fall and
the share fell, and that was the largest gain in the table - a reminder that the size of the move, not
whether the call was right, decides the money.

## What the research actually found

| Source                                          | What it measured                                                  | Result                                                                                                                                                                                  |
| ----------------------------------------------- | ----------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| QuantConnect library page, Gaussian naive Bayes | Its own five-year backtest on ten large technology shares         | Reward for risk of 0.011 against 0.729 for holding the index; in the 2020 crash, -1.433 against -1.467; in the recovery, -0.156 against +4.497                                          |
| Lu (2016) and Imandoust and Bolandraftar (2014) | The studies the page follows, named there but not linked          | The page states these are its models; the trading evidence in this tutorial is the page's own backtest, not the papers', which are not in this repository's harvested corpus            |
| `2603.16886v1` (p.1)                            | Nine deep-learning architectures, 918 runs, three asset classes   | Directional accuracy was indistinguishable from a coin toss across all 54 model-category-horizon combinations, so a simple probabilistic classifier should not be expected to do better |
| `2110.14914v2` (pp.5, 7, 8)                     | Classifiers calling the next move on five futures, then traded    | 52 to 56 percent accuracy, yet many configurations were unprofitable once normal slippage was charged                                                                                   |
| `2502.15822v1` (pp.2, 4)                        | A boosted model on data where the rare event is about 0.2 percent | Accuracy of 99.72 percent, which is close to what always answering "nothing happened" would score, showing how easily an accuracy figure misleads                                       |

The page's own result is the one to hold on to. Over five years the rule made almost nothing after the
wobble was counted, against a strongly positive result for simply holding the index. Its only clearly
good stretch was the market crash of early 2020, when it lost slightly less than the index, and it gave
that advantage straight back in the recovery that followed, scoring -0.156 against +4.497. That pattern -
holding up in a fall and lagging in a rise - is the signature of a strategy that is quietly short the
market, not of one that predicts direction. The page is also honest about its own assumptions: it says
it does not test whether the four daily returns are independent of each other or whether they follow a
bell curve, and leaves both as future work. Those two assumptions are exactly what the word "naive
Bayes" and the word "Gaussian" each stand for, so the reader should treat them as unverified.

## How this project relates to it

- [Machine Learning for Trading](../../../strategies/books/09_machine_learning_for_trading.md)
  is the closest match. Section 6 on costs, turnover and loss design is the one to read here: the page
  trains a classifier and trades its output every day, and that brief's takeaway is that a directional
  classifier which is right a little more often than not still has to clear the cost of trading.
- [Machine learning for finance](../../../strategies/books2/04_machine_learning_for_finance.md)
  is where the accuracy-versus-outcome distinction is documented, including the experiment showing that
  changing what the model is trained to get right moved the portfolio result more than changing the
  model did.
- [Overfitting and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md)
  covers the danger of a model fitted to a short recent window and the requirement to record the number
  of settings tried, which matters here because the page's feature length of four days and training
  length of 100 days are both free choices.

The repository does not implement a naive Bayes predictor. The nearest implemented thing is the
validation discipline those briefs describe, which is what a reader would use to judge this model if it
were ported here.

## Where it goes wrong

- Overfitting a short window. The model is trained on the last 100 days and refitted whenever the share
  list changes. One hundred days is a small sample, and the averages and spreads it learns describe
  whatever the market happened to be doing in those months, not a stable property of the share.
- The independence assumption is false. The four daily returns of one share are plainly related to each
  other, so multiplying their likelihoods as if they were separate overstates how much the pattern
  tells you. The method's name admits this; the page does not test how much harm it does.
- The bell-curve assumption is false. Daily returns have fatter tails than a bell curve allows, meaning
  extreme days happen far more often than the model expects. That is why the worked example's day 2,
  built on an unusual plus 0.80 percent day, went the wrong way.
- High accuracy can still mean nothing. The fraud paper shows the arithmetic of a lopsided sample, where
  always giving the common answer scores close to perfectly. Here there is a different version of the
  same trap: a model that is right about direction most of the time can still lose, because the days it
  is wrong on are larger than the days it is right on.
- The result is mostly market exposure. Holding ten technology shares while "up" is predicted is very
  like holding technology shares, so the strategy rises and falls with them. The backtest's shape -
  losing less in the crash and lagging in the recovery - is what that looks like in the numbers.
- Costs and daily turnover. The rules can change the position every day, and each change pays the gap
  between the buying and selling price plus any commission. The page prints no cost figure, and the
  returns it reports are gross of whatever trading costs a real account would pay.

## Try it yourself

You need a spreadsheet and a public source of daily open and close prices for one large technology
share. No money and no code are involved.

1. Put the dates in column A, the opening prices in column B and the closing prices in column C, for the
   last 120 trading days.
2. In column D, compute the open-to-close return: column C divided by column B, minus one, on the same
   row.
3. In column E, compute the next day's overnight return: the opening price two rows down divided by the
   opening price one row down, minus one.
4. In column F, write "up" if column E is positive, "down" if negative, and "flat" if zero.
5. In columns G, H, I and J, copy the previous four days' open-to-close returns, so that each row holds
   the four days ending the day before the one being predicted.
6. Use rows 1 to 80 as the training set: for each column G to J, and separately for the "up" rows and
   the "down" rows, compute the average and the spread.
7. For each of the last 40 rows, compare that day's last return against the up average and the down
   average and note by how far the value sits from each. The class whose average is nearer gets the
   larger score, all else equal.

What to notice: the flat class will be almost empty, so the problem is really up against down. Compare
the average of the up rows with the average of the down rows; they will be nearly the same number,
because the last few days' returns carry little information about the next day. Now count how often your
row's nearest average is the up one, and then check how often the day after actually rose. The two
counts will be close, which is the same near-coin-toss the library page's backtest produced.

## Where this came from

- [QuantConnect: naive Bayes](https://www.quantconnect.com/tutorials/strategy-library/gaussian-naive-bayes-model),
  the rules as implemented: ten large technology shares, four daily returns as features, 100 training
  samples, an up-or-down-or-flat label, and a daily hold from open to open. The research page is at
  [research/15268](https://www.quantconnect.com/research/15268/gaussian-naive-bayes-model/).
- Lu (2016) and Imandoust and Bolandraftar (2014), the studies the page says it follows. Neither is
  linked on the page nor present in the repository's harvested corpus, so they are named but not quoted.
- `2603.16886v1`, "A Controlled Comparison of Deep Learning Architectures for Multi-Horizon Financial
  Forecasting: Evidence from 918 Experiments", the source for the coin-toss directional accuracy finding.
- `2110.14914v2`, "Trading via Selective Classification", the source for the accuracy-versus-cost
  finding.
- `2502.15822v1`, "Financial fraud detection system based on improved random forest and gradient boosting
  machine (GBM)", the source for the accuracy illusion on a lopsided sample.
- [Machine Learning for Trading](../../../strategies/books/09_machine_learning_for_trading.md),
  [Machine learning for finance](../../../strategies/books2/04_machine_learning_for_finance.md) and
  [Overfitting and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md),
  this repository's own briefs on the questions this model raises.

## Words used in this tutorial

- accuracy: the share of predictions that were correct, out of all predictions made.
- average (mean): the sum of a set of values divided by how many there are.
- bell curve: a smooth, symmetrical shape that is tallest at its centre and falls away on both sides.
- features: the measurements a model is given as its input.
- likelihood: how typical an observed value is for a particular class, read off that class's curve.
- naive Bayes: a method that multiplies per-feature likelihoods together as if the features were
  unrelated, and picks the class with the largest product.
- overfitting: a model that has memorised the past it was shown rather than learned a pattern that
  carries into new data.
- prior: how common a class was in the training data, before any features are looked at.
- reward for risk (Sharpe ratio): the average return divided by how much it wobbled, usually written per
  year; zero means no reward after the wobble is counted.
- spread (standard deviation): how far a set of values typically sits from their average.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
