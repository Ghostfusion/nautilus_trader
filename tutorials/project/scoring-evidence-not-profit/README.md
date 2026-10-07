# Scoring a strategy on its evidence, not its profit

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Nothing by itself: it is the set of measurements used to grade a strategy that does trade                                                                                                                            |
| How often it trades       | Not applicable; the measurements are taken once per run, or once per period within a run                                                                                                                             |
| What you need             | Nothing but this page; the worked example needs only a table of six numbers                                                                                                                                          |
| Where the rules come from | [The measurement work the general-finance briefs call for](../../../strategies/books2/implementation_plan.md) and the metric declarations in [crates/analysis/src/metric.rs](../../../crates/analysis/src/metric.rs) |
| The underlying research   | [Overfitting, reproducibility and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md), this repository's brief on what replication of published findings actually shows         |
| How well it held up       | Mixed: the strongest evidence says most published predictors replicate, but the briefs are explicit that the studies are mostly single-market and single-period, and one bad data row can still erase a result       |
| Also appears in           | [Attribution of results](../attribution-of-results/README.md) and [Cost, and the break-even price](../cost-basis-and-breakeven/README.md)                                                                            |

## The idea in one paragraph

A backtest result is a number produced by choices: which data was used, which period, which settings,
and how many versions of the rule were tried before one was kept. Grading a strategy on that number
treats it as an estimate of what the rule can do. Grading it on its evidence instead asks how much of
the number could have been luck. This repository reports the length of the sample, how the rule
behaves on data kept aside, how sensitive the result is to the settings, and whether costs are
included, and it labels every measurement with the stage of the chain it belongs to, so a score that
merely predicts cannot be read as profit. A measurement that reports information is deliberately
marked as carrying no direction, because it is not a forecast of money.

## Why anyone believed it

The pressure to report one number is real. Clients, headlines and internal reviews all ask for the
return, and a search over many settings will always produce a setting with a good return, because the
best of many tries is by definition high. If nobody counts the tries, the number looks like evidence
of skill.

The belief behind evidence grading is that the luck can be measured rather than argued about. Count
the settings tried, measure how far apart their results were, note how long the sample was, and run
the frozen rule once on data not used to build it. Then some of the good number is attributable to the
search itself, and the literature adds that the remaining risk is the analyst's own data choices.

## An everyday comparison

A doctor hears that one patient took a new medicine and recovered. That is a fact about one patient,
not about the medicine, because most people recover anyway and nobody recorded the patients who took
it and did not. So a trial is run: hundreds of patients, half given the medicine and half given a
dummy, split by chance. The doctor then asks how long the trial ran, whether the benefit showed up in
a second hospital, whether it survives when the dose is changed, and whether the side effects were
counted. Grading a strategy on its evidence is that trial design: the sample, the held-back patients,
the changed dose, and the side effects, which in trading are the costs.

## The rules, step by step

1. Count the sample before reading the result. A result over twenty periods is a description, not
   evidence. The engine reports a measurement as unavailable with the reason "insufficient data"
   rather than printing a number it cannot support.
2. Hold a stretch of the most recent data aside and do not look at it while building the rule. Run
   the frozen rule on it once, afterwards.
3. Charge the costs. A rule that trades often pays the gap between the buying and selling price, the
   commission, and the cost of its own orders moving the price.
4. Move the settings. Change each setting by a little and rerun; a good result that only exists at one
   exact setting is a fitted result.
5. Count the tries, including the versions that were discarded, and carry that count with the
   reported number.
6. Check the rate that a claim has to beat. For a rule that flags rare events, the fraction of records
   that really are events, called the base rate, controls what accuracy means, so accuracy is never
   reported without it.
7. Put every measurement in its stage: a forecast score is measured against what happened next; a
   decision measurement against realised trades; an account measurement against the money.
8. Refuse rather than default, and state the direction or state that there is none. A measurement
   whose input was absent is unavailable by name rather than zero, one whose input is present but
   nonsensical is invalid, and its direction is maximise, minimise, target or informational.

## The maths, with every symbol named

The central correction is the deflated Sharpe ratio. A Sharpe ratio is a return above the rate paid on
safe cash, divided by how much the return swings around its average, so it measures reward per unit of
wobble. The correction asks: how good would the best of several attempts look if none of them had any
skill at all? That threshold is

```text
SR0 = sqrt(V) * ((1 - g) * q(1 - 1/N) + g * q(1 - 1/(N * e)))
```

- `SR0` is the Sharpe ratio that the best of `N` skill-free attempts would be expected to reach.
- `V` is the variance of the Sharpe ratios of the attempts, which is the average squared distance of
  those numbers from their own mean.
- `N` is the number of attempts; in the code this is the counted trial count, which for dependent
  attempts is the effective count the study declares.
- `g` is a fixed constant, about 0.5772.
- `q` is the function that turns a probability into the value a standard bell curve sits below.
- `e` is the fixed number about 2.718 used by the logarithm.
- A larger `N` raises `SR0`, which is the whole point: the more you try, the higher the score that
  luck alone can produce.

The result itself is then turned into a probability:

```text
DSR = P( (SR - SR0) * sqrt(T - 1) / sqrt(1 - skew * SR + ((kurtosis - 1) / 4) * SR * SR) )
```

- `DSR` is the deflated Sharpe ratio, a number between 0 and 1. Higher is better; 0.5 is a coin toss
  against the skill-free threshold.
- `SR` is the best result's Sharpe ratio.
- `T` is the number of periods in the sample, so a longer sample pushes `DSR` up.
- `skew` and `kurtosis` describe the shape of the returns: skew is the lean of the distribution, and
  kurtosis is how heavy its tails are.
- `P` is the chance a standard bell curve falls below the value in brackets, and the formula uses the
  non-excess kurtosis, so an ordinary bell-shaped series has 3.

Two smaller formulae from the same repository, used when a rule flags events:

```text
precision = TP / (TP + FP)              recall = TP / (TP + FN)
false_discovery_rate = FP / (TP + FP)   base_rate = (TP + FN) / N
```

- `TP`, `FP`, `TN` and `FN` count the records the rule marked correctly, marked wrongly as events,
  correctly left alone, and missed. Those four counts are the whole confusion matrix.
- `precision` is the share of the flagged records that were real; `recall` is the share of the real
  records that were flagged; `false_discovery_rate` is the share of the flags that were wrong.
- `N` is the number of records, so the base rate is simply how common the event is.

## A worked example

Six versions of one invented rule were tried on the same data, each with a different setting. The
Sharpe ratios are made up but of the size these numbers take.

| Attempt | What changed              | Sharpe ratio |
| ------- | ------------------------- | ------------ |
| 1       | fastest setting           | -0.80        |
| 2       | one step slower           | -0.30        |
| 3       | two steps slower          | +0.20        |
| 4       | three steps slower        | +0.70        |
| 5       | four steps slower         | +0.10        |
| 6       | slowest setting, selected | +1.00        |

Now the arithmetic, step by step.

```text
mean = ( -0.80 - 0.30 + 0.20 + 0.70 + 0.10 + 1.00 ) / 6 = 0.90 / 6 = 0.15
deviations = -0.95, -0.45, +0.05, +0.55, -0.05, +0.85     (sum 0.00, as it must)
squares    =  0.9025, 0.2025, 0.0025, 0.3025, 0.0025, 0.7225   (sum 2.1350)
V (sample variance) = 2.1350 / 5 = 0.4270
spread = sqrt(V) = 0.6535
factor for N = 6 = 0.4228 * 0.9674 + 0.5772 * 1.5439 = 1.3002
SR0 = 0.6535 * 1.3002 = 0.85
```

So the best of six tries, with no skill at all, would be expected to produce a Sharpe near 0.85. The
selected result was 1.00, above that threshold but not far above it. Now the probability:

```text
skew = 0, kurtosis = 3, SR = 1.00, T = 60
variance factor = 1 - 0 * 1.00 + ((3 - 1) / 4) * 1.00 * 1.00 = 1.50
argument = (1.00 - 0.85) * sqrt(60 - 1) / sqrt(1.50)
         = 0.15 * 7.6811 / 1.2247
         = 0.94
DSR = P(0.94) = 0.83
```

Read plainly: there is about an 83 percent chance that the best of these six attempts would look at
least this good even with no skill, given how far apart the six were. That is a probability, not a
verdict. Sample length moves it most; with the same six attempts and the same selected Sharpe of 1.00:

| Periods in the sample (T) | Skill-free threshold (SR0) | Deflated Sharpe |
| ------------------------- | -------------------------- | --------------- |
| 60                        | 0.85                       | 0.83            |
| 252                       | 0.85                       | 0.97            |

The threshold does not move, because it depends on the spread of the attempts rather than on the
sample. The confidence moves a great deal, which is why a report states the number of periods
alongside every statistic.

## What the research actually found

The brief behind this tutorial reads six papers; its strongest source is a review of meta-studies of
207 published predictors drawn from 140 papers. It reports that almost all of those findings
replicate, that predictability persists out of sample, and that the statistics are far above the usual
threshold: 183 of the 207 exceed a t-statistic of 2.0, 74 exceed 4.0 and 26 exceed 6.0, where a
t-statistic is an estimate divided by its own uncertainty. Publication-bias corrections shrank
in-sample returns by only 10 to 15 percent across three independent teams, with false discovery rates
under 10 percent. Three hurdle settings are recorded for a single test: 1.96 implies a false discovery
rate of 8.8 percent, 2.3 gives 5 percent, and 3.0 leaves 81 percent of the rejected findings true.

The same paper is blunt about the cost of a wide search. Sorting stocks on simple functions of 240
accounting variables produced 18,113 strategies, and 30.17 percent of them cleared a t-statistic of
2.0 while 8.40 percent cleared 4.0, against chance rates of 4.55 percent and 0.0063 percent. The
lesson the brief draws is that the hurdle is a policy choice with a price, and that a search which
reports one statistic and no trial count is reporting a number whose threshold is undefined.

The other paper worth naming is a correction study. Two published betting returns of 17.29 and 28.82
percent were traced by an exact replication - same data, same algorithm, identical sequence of bets -
to a single erroneous data row, and turned into losses of 7.36 and 6.31 percent. After the correction,
one strategy survived at 12.44 percent, then earned nothing over three further years of cleaned data.
The two papers together say that publication bias is smaller than the folklore claims, while the
analyst's own data choices are where results actually break.

## How this project relates to it

The vocabulary of scoring lives in the
[metric declarations](../../../crates/analysis/src/metric.rs). Every measurement declares a definition
with a stable identifier, a title rendered from its settings, its units, the inputs it needs, and a
stage from the closed set `Forecast`, `Decision` and `Account`. A measurement's outcome is not an
optional number but a result with a status, so absent input and nonsensical input are told apart.

Two measurements show the refusal built in. The
[detector report](../../../crates/analysis/src/statistics/detector_report.rs) computes the confusion
matrix and every rate read from it, and returns the accuracy and the base rate as one inseparable
pair; a constructor that tries to build the report from a headline accuracy alone is refused by name.
The [correction-impact report](../../../crates/analysis/src/statistics/correction_impact.rs) measures
a declared outcome metric on the uncorrected and the corrected stream and reports both with their
difference, and refuses to exist when no outcome metric was declared.

The larger measurement programme is the
[measurement module](../../../crates/research/src/measurement.rs). It keeps three separate experiments -
research signal quality against a forward return, the policy's effect, and the execution realisation -
as three distinct types with no function that pools them, so a strong information coefficient with
weak realised profit stays a finding about the second or third stage rather than the first. It also
measures a model's own parameters by drawing seeded datasets at a known parameter vector, refitting
each, and returning a verdict of identified, weakly identified or unidentified, with no verdict that
means "not measured". Where a measurement is quoted in the information currency, the direction type
cannot be a direction at all: `MetricDirection::Informational` states that no direction is claimed.

The programme that ties these together is
[strategies/books2/implementation_plan.md](../../../strategies/books2/implementation_plan.md). Its
acceptance lines are observations rather than task lists, and its final section states what a run of
the platform should be able to say without being asked: what was optimised, on what basis, over which
specification, on data whose provenance was checked, with parameters whose identifiability was
measured, and with the cost of every correction printed beside the correction.

## Where it goes wrong

- The trial count can be forgotten. The correction needs the number of attempts, including the ones
  nobody wrote down, and a run that holds only a number and no count cannot be corrected at all.
- Accuracy without a base rate is close to meaningless. On an event that happens in 0.2 percent of
  records, a rule that flags nothing scores 99.8 percent accuracy and finds nothing.
- Statistics computed over different bases can look like one row. Compounding and divisor
  conventions change the number, so every return measurement has to name its basis.
- Grading on evidence does not make a strategy good. It establishes that the number is not obviously
  the product of luck; whether anything is left after costs is answered by the account-stage
  measurements, not by the evidence ones.
- The evidence itself can be thin. The briefs are explicit that most of their studies are
  single-market and single-period, so a replicated finding can still be a fact about one market over
  one decade.
- A label of unknown provenance cannot be scored at all. When nothing records how the thing being
  predicted was produced, the measurement refuses to score it rather than guessing.

## Try it yourself

You need a table of six numbers and nothing else. The exercise is to feel how much the correction
depends on the spread of the attempts.

1. Open a spreadsheet with two columns: attempt number, 1 to 6, and a Sharpe ratio for each. Use the
   six values from the worked example: -0.80, -0.30, 0.20, 0.70, 0.10, 1.00.
2. Compute the mean, then each value's distance from the mean, then the square of each distance.
3. Add the squares and divide by five, which is one less than six; take the square root. This is the
   spread, 0.65 in the example.
4. Replace the six values with six that are close together, for example 0.90, 0.95, 1.00, 0.85, 0.92,
   0.98, and repeat steps 2 and 3. The spread collapses to roughly 0.05.
5. Now change only the count: pretend there were 100 attempts instead of six, keeping the first set's
   spread. The multiplier rises from 1.30 to about 2.5, roughly doubling the luck threshold.

What to notice: a set of attempts that all look alike produces a very low luck threshold, so the best
of them looks convincing, and the same best result can be convincing or not depending on two things
that have nothing to do with the strategy - how many settings were tried and how far apart their
results were. This is why a report states the trial count and the spread rather than only the winner.

## Where this came from

- [The metric declarations](../../../crates/analysis/src/metric.rs), the definition, the closed
  vocabularies for units, direction, inputs and stages, and the four-state status with its reasons.
- [The detector report](../../../crates/analysis/src/statistics/detector_report.rs), the confusion
  matrix and the rates, and the refusal to report accuracy without the base rate.
- [The correction-impact report](../../../crates/analysis/src/statistics/correction_impact.rs), the
  measurement of what a data correction did to a declared outcome metric.
- [The significance module](../../../python/nautilus_trader/optimization/significance.py), the
  deflated Sharpe ratio, its contract, and the trial counts it carries.
- [The measurement module](../../../crates/research/src/measurement.rs), the three experiments kept
  apart, the calibration by bucket, the redundancy report and the parameter-recovery verdicts.
- [The overfitting brief](../../../strategies/books2/28_overfitting_and_research_integrity.md), whose
  "short answer" is the source of the numbers above: 207 predictors in the review, 183 above a
  t-statistic of 2.0, 74 above 4.0, 26 above 6.0, the 10 to 15 percent publication-bias shrinkage, the
  hurdle table for 1.96, 2.3 and 3.0, the 18,113 mined strategies with 30.17 and 8.40 percent
  clearing the two hurdles, and the betting correction that turned 17.29 and 28.82 percent into
  -7.36 and -6.31 percent.
- `2209.13623v3`, the review of meta-studies on publication bias in asset pricing, cited by the brief
  for every replication and multiple-testing number above, and `2306.01740v4`, the betting correction
  study, cited for the single erroneous data row and the out-of-sample decay.

## Words used in this tutorial

- base rate: the share of records in which the event being looked for really happened.
- deflated Sharpe ratio: the chance that the best of many attempts would look this good with no skill
  at all, given how far apart the attempts were.
- information coefficient: how strongly a score is related to what happens next, measured across many
  things at once; it carries no direction.
- out of sample: data kept aside while a rule is built, then used once to test it honestly.
- Sharpe ratio: a return above the safe rate divided by how much the return swings around its average.
- stage: which step of the chain a measurement belongs to, here a forecast score, a trade decision or
  the account.
- t-statistic: an estimate divided by its own uncertainty; larger means further from zero relative to
  its noise.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
