# Scoring what survived the held-back test

Date: 2026-10-08. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Nothing by itself: it turns an original run and a held-back run into one number per measurement, saying how much of the original result survived                                                                                                                                                                                                                                                                                                                     |
| How often it trades       | Not applicable; computed once per measurement for each pair of runs, usually after the held-back data has been used once                                                                                                                                                                                                                                                                                                                                             |
| What you need             | Nothing but this page; the worked example needs a table of five numbers and a calculator                                                                                                                                                                                                                                                                                                                                                                             |
| Where the rules come from | [The robustness ranking](https://github.com/coding-kitties/investing-algorithm-framework/blob/main/investing_algorithm_framework/analysis/robustness.py) and [the consistency scores](https://github.com/coding-kitties/investing-algorithm-framework/blob/main/investing_algorithm_framework/domain/backtesting/consistency.py) in the investing-algorithm-framework repository, and [the objective weights](../../../crates/analysis/src/objective.rs) in this one |
| The underlying research   | Smerlak, [Great year, bad Sharpe?](https://arxiv.org/abs/2302.08829) (2024), on the joint distribution of performance and risk-adjusted return; Bel Hadj Ayed, Loeper and Abergel, [Robustness of mathematical models and technical analysis strategies](https://arxiv.org/abs/1605.00173) (2016), on a simple rule beating a theoretically better one when the model is mis-specified                                                                               |
| How well it held up       | Mixed: the arithmetic is simple, checkable and reproducible, and the two sources show why the ranking depends on the measurement chosen, but the weights and the normalising constants in the quoted implementation are the author's choices rather than measured quantities                                                                                                                                                                                         |
| Also appears in           | [Held-back data comes in two flavours](../two-kinds-of-out-of-sample/README.md), [Searching settings](../sweeping-settings/README.md), [Shuffling the past](../does-the-order-matter/README.md), and [Scoring a strategy on its evidence, not its profit](../../project/scoring-evidence-not-profit/README.md)                                                                                                                                                       |

## The idea in one paragraph

A held-back test produces two numbers for every measurement: the original score and the score on
the data that was kept away. Reporting them side by side leaves the reader to do the comparison. The
procedure here does two things with them. First, it divides one by the other to get a fraction - how
much of the original the held-back data retained - with the division arranged so that a bigger
fraction always means "held up better", even for measurements where smaller is better. Second, it
scores how steady the result was across the periods of the test, since a rule that made all of its
money in one window is a different object from one that made a little in each. The output is a
short list of numbers that can be ranked, and the danger is that ranking by them quietly replaces
the question the reader actually cares about.

## Why anyone believed it

The attraction is that a fraction is comparable across rules and across measurements. An average
return of 6 percent a year and an average return of 3 percent a year belong to different histories
and cannot be ranked without a scale; the fractions 0.8 and 0.4 can be compared immediately, and
they say the same thing in both cases: how much of the original result the held-back data kept.

The belief rests on an assumption worth stating: that the original result was a fair estimate of
something real, so that a fraction of it is also meaningful. Where the original result was itself
the best of many tries, the fraction is a fraction of an inflated number, and the fraction can look
reassuring while the thing it is a fraction of was never there. The survey numbers quoted in
[Searching settings](../sweeping-settings/README.md) are the counterweight: reported returns shrink
by 10 to 15 percent after a correction for selection, but 26 percent of the return is lost simply by
moving to fresh data and 58 percent once the result is public.

## An everyday comparison

Two students sit a practice exam and a real one. The first scores 90 and then 60. The second scores
55 and then 50. On the real exam the first student did better; on the drop the second student did
better, keeping 0.91 of the score against 0.67. A teacher choosing who to coach would want to know
which question is being asked. And a third piece of information is missing from both: how steady
each student was across the five sections of the exam. A student who scores 70 overall by getting 70
in every section is a different proposition from one who scores 100, 100, 100, 0, 0 and averages the
same.

## The rules, step by step

1. Fix the measurements before the held-back run, exactly as with the trial count. Each measurement
   needs a direction: bigger is better, or smaller is better.
2. Compute one fraction per measurement: the held-back value divided by the original, or the
   original divided by the held-back value when smaller is better.
3. Decide how to combine several held-back samples - a time sample and an instrument sample, say -
   into one fraction per measurement. Averaging rewards overall retention; taking the smallest
   punishes any single bad sample hardest, and the choice must be stated.
4. Weight the measurements by how much you care about them, and average the fractions using the
   size of each weight. The direction is already inside each fraction, so a penalty measurement
   contributes with the size of its weight rather than its sign.
5. Split the run into periods - windows - and compute the average and the spread of each measurement
   across them, so a rule that earned everything in one window can be told apart from a steady one.
6. Turn the spread into a score between nothing and one, either by dividing the spread by the size
   of the average or by dividing it by a spread you consider the worst tolerable.
7. Combine the steadiness scores into one number, dropping any component you could not compute and
   renormalising the weights of the rest, so that a missing component does not silently count as
   zero.
8. Report the level as well as the fraction. A rule that kept 0.5 of a 4.0 ratio and a rule that
   kept 0.5 of a 0.4 ratio look identical in the fraction and are not the same discovery.
9. Write down the weights and any normalising constants you chose. They are choices, and two people
   with different choices will rank the same results differently.

## The maths, with every symbol named

The fraction retained, oriented so that bigger always means held up better:

```text
held_up = held_back_value / original_value          when bigger is better
held_up = original_value / held_back_value          when smaller is better
```

- `original_value` is the measurement on the data that was used to choose the settings.
- `held_back_value` is the same measurement on data kept away from the choosing.
- It means: the share of the original result that survived. 1.0 is unchanged, 0.5 is half retained,
  0.0 is nothing retained.

The combined score across measurements:

```text
robustness = sum over measurements of (fraction * absolute_weight)
             / sum over measurements of absolute_weight
```

- `fraction` is the held-up fraction for that measurement, already combined across held-back samples
  by averaging or by taking the smallest.
- `absolute_weight` is the size of the importance you gave the measurement, ignoring its sign,
  because the sign was used up in the direction of the division.
- It means: a weighted average of how well each measurement survived, in units of "share retained".

The steadiness of a measurement across windows:

```text
coefficient_of_variation = standard_deviation / absolute_value_of_the_average
consistency = 1 - coefficient_of_variation
stability = 1 - standard_deviation / worst_tolerable_spread
```

- `standard_deviation` is computed from the per-window values, dividing the sum of squared
  deviations by the number of windows minus one.
- `average` is the mean of the per-window values.
- `worst_tolerable_spread` is a spread chosen for the measurement, such as 2.0 for a ratio of return
  to variability or 50 for a percentage win rate.
- Both scores are then held between nothing and one, so a spread wider than the average cannot score
  below zero.
- It means: a rule whose windows agree scores near one; a rule whose windows disagree scores near
  nothing. Dividing by the average makes the score scale-free, and dividing by a chosen spread makes
  it scale-dependent.

## A worked example

Five windows of a held-back run, each three months long, with the return of each window.

| Window  | Return | Deviation from 2.0 | Squared deviation |
| ------- | ------ | ------------------ | ----------------- |
| 2019 Q1 | +4.0   | +2.0               | 4.00              |
| 2019 Q2 | -1.0   | -3.0               | 9.00              |
| 2019 Q3 | +3.0   | +1.0               | 1.00              |
| 2019 Q4 | +2.0   | 0.0                | 0.00              |
| 2020 Q1 | +2.0   | 0.0                | 0.00              |
| Sum     | +10.0  | 0.0                | 14.00             |

The average return is 10.0 divided by 5, which is 2.0 percent per window. The spread is the square
root of 14.00 divided by 4, which is the square root of 3.50, or 1.87. The coefficient of variation
is 1.87 divided by 2.0, which is 0.94, so the consistency is 1 minus 0.94, which is 0.06. The rule
earned 10 percent over the year and scores almost nothing for steadiness, because one window
contributed 4 points and another lost one.

Now the fractions. The original run reported a ratio of return to variability of 1.5, and the
held-back run reported 0.6, so the fraction retained is 0.6 divided by 1.5, which is 0.40. The
original deepest fall from a peak was 20 percent and the held-back one was 25 percent; that
measurement is better when smaller, so the fraction is 20 divided by 25, which is 0.80. With weights
of 1.5 and 0.8, chosen to suit the example:

| Measurement                 | Original | Held back | Fraction | Weight | Fraction times weight |
| --------------------------- | -------- | --------- | -------- | ------ | --------------------- |
| Ratio of return to variance | 1.50     | 0.60      | 0.40     | 1.5    | 0.600                 |
| Deepest fall from a peak    | 20.0     | 25.0      | 0.80     | 0.8    | 0.640                 |
| Sum                         |          |           |          | 2.3    | 1.240                 |

The combined robustness score is 1.240 divided by 2.3, which is 0.54: a little over half of the
original result survived the move to held-back data. Nothing in the table says whether 1.5 was
worth keeping in the first place, and the earlier pages in this group are about that question.

One more number from the same run, to show how a constant changes a verdict. The steadiness of the
return could also be scored against a worst tolerable spread of 2.0 rather than against the average,
giving 1 minus 1.87 divided by 2.0, which is 0.07 - about the same as before. Scored against a worst
tolerable spread of 100, as a returns-based implementation does, it would be 1 minus 0.019, which is
0.98. The same five numbers therefore score as a wildly inconsistent rule or as an almost perfect
one, depending on a constant somebody chose.

## What the research actually found

- The note on performance and risk-adjusted return shows that the two are not aligned when returns
  are heavy-tailed, which real returns are. Using both synthetic data and a set of 1,608 exchange
  traded funds, it finds that the investments with the best in-sample performance are never the ones
  with the best in-sample risk-adjusted return, and the reverse, as a consequence of how the mean
  and the spread of a heavy-tailed sample move together. As a scale, the same paper reports a
  theoretical ratio of 0.13 for its fund sample with a standard error of 1.00 (`2302.08829v2`). The
  practical consequence is that ranking candidates by return and ranking them by the ratio of return
  to variability select different rules.
- The study of a simple rule against a theoretically better one compares a cross moving averages rule
  with the optimal strategy of a model whose trend is not observable, when the model's settings are
  wrong. Its numerical examples find the cross moving averages rule more robust than the optimal
  strategy under mis-specification (`1605.00173v1`). The lesson for scoring is that the rule that
  looks best under ideal assumptions is not automatically the one that survives when assumptions
  are wrong, which is what a held-up fraction is trying to measure.
- The validation study behind the previous two pages reports its regime split rather than a single
  number: an average of plus 2.4 percent a year across the relevant held-back periods from 2020 to
  2024, against minus 0.16 percent a year from 2015 to 2019 (`2512.12924v1`). Read as a held-up
  fraction, those two periods do not merely differ in level; one of them has a negative value, which
  turns the fraction into a negative number and makes the mean-of-samples aggregate meaningless
  without the level being reported alongside it.
- The implementation quoted in the provenance table fixes the weights at 0.35 for returns, 0.25 for
  the win rate, 0.20 for the ratio of return to variability and 0.20 for the share of profitable
  windows, and it drops any component it cannot compute while renormalising the rest. Those four
  numbers are not derived from any measurement; they are the author's choice, and the page states
  them rather than repeating them as if they were findings.

## How this project relates to it

This repository's ranking machinery is at
[objective.rs](../../../crates/analysis/src/objective.rs), where each scored measurement carries a
weight and a direction and the candidate's score is the weighted sum of the raw values. The
difference from the pages above is deliberate: the objective works on raw values within one
comparison, while a held-up fraction works across two runs of the same rule. The two are
complementary, and neither is a substitute for reporting the level.

For the wider question of what a measurement is worth, the base page in this collection is
[Scoring a strategy on its evidence, not its profit](../../project/scoring-evidence-not-profit/README.md),
which explains why a repository reports the length of the sample, the behaviour on held-back data
and the sensitivity to settings instead of a single profit number. The measurement definitions
themselves, including the ratio of return to variability and the deepest fall from a peak, are
listed under [crates/analysis/src/statistics](../../../crates/analysis/src/statistics), and they
are the things a fraction would be computed from.

## Where it goes wrong

- A fraction hides the level. A rule that retained 0.5 of a ratio of 4.0 still reports a ratio of
  2.0 on held-back data and is a different proposition from a rule that retained 0.5 of 0.4. The
  fraction was invented to compare across scales and it is exactly this comparison that it hides.
- A fraction breaks when the original value is near zero. The implementation quoted above returns
  nothing rather than a number when the original is zero, which is honest, but it means a rule with
  a flat original result drops out of the ranking entirely and is then invisible rather than
  bottom-ranked.
- A negative held-back value makes the arithmetic meaningless. Dividing by a negative reverses the
  direction of the fraction, so a rule that lost money out of sample can appear to have retained a
  large share of its original, once either value crosses zero.
- Taking the smallest fraction across samples is decided by one period. It is a defensible
  worst-case rule, but with two held-back samples the score is the worse of the two, and with the
  regime dependence quoted above the worse of the two can be the one period in which everything
  went against the rule.
- The normalising constants decide the steadiness score. The same five windows score 0.06 against a
  tolerable spread of 2.0 and 0.98 against a tolerable spread of 100, so a constant chosen for one
  measurement silently flatters another.
- The weights are a choice with no measurement behind them, which is why step 9 asks for them to be
  written down. Two research groups with the same results and different weights will publish
  different rankings.
- Ranking by one measurement selects a different rule from ranking by another, and the difference is
  systematic rather than random when returns are heavy-tailed, which the note above demonstrates.
- The fractions are computed after the fact, by whoever ran the test, from whichever runs were kept.
  A fraction computed only for the rules that were reported is the same selection problem as before,
  one level up.

## Try it yourself

You need a calculator. No code.

1. Write five windows of returns: +4, -1, +3, +2, +2 percent.
2. Compute the average. Subtract it from each window and square the differences. Add the squares,
   divide by four, and take the square root. You should get about 1.87.
3. Divide that spread by the average to get about 0.94, and subtract from one to get about 0.06.
4. Now replace the -1 with +2 and repeat. The average becomes 2.6, the spread falls to about 0.89,
   and the steadiness rises to about 0.66.
5. Finally, take an original ratio of 1.2 and a held-back ratio of 0.3, and write the retained
   fraction. Then write one sentence saying what the reader still does not know from that fraction
   alone.

What to notice: one window out of five decides the steadiness score, and the fraction in step 5 is
the same number whether the original was 1.2 or 12, which is the property that makes it useful for
comparison and useless as a statement about the size of anything.

## Where this came from

- The robustness ranking in the investing-algorithm-framework repository, read at commit `f8e82c7`
  on the main branch: the division arranged by direction, the zero guards, the mean or smallest
  aggregation, the weighting by the size of each weight, and the note that a score of nothing is
  listed last
  ([the ranking file](https://github.com/coding-kitties/investing-algorithm-framework/blob/main/investing_algorithm_framework/analysis/robustness.py)).
- The consistency scores in the same repository at the same commit: the coefficient-of-variation
  score, the spread against a chosen maximum, the four weights of 0.35, 0.25, 0.20 and 0.20, and the
  renormalisation when a component is missing
  ([the consistency file](https://github.com/coding-kitties/investing-algorithm-framework/blob/main/investing_algorithm_framework/domain/backtesting/consistency.py)).
- Smerlak, [Great year, bad Sharpe?](https://arxiv.org/abs/2302.08829) (2024), the anti-alignment of
  the best performance and the best risk-adjusted return under heavy tails, and the 1,608 exchange
  traded funds with a theoretical ratio of 0.13 and a standard error of 1.00; identifier
  `2302.08829v2`.
- Bel Hadj Ayed, Loeper and Abergel, [Robustness of mathematical models and technical analysis
  strategies](https://arxiv.org/abs/1605.00173) (2016), the cross moving averages rule against the
  optimal strategy under mis-specification; identifier `1605.00173v1`.
- Deep, Deep and Lamptey, [Interpretable Hypothesis-Driven
  Trading](https://arxiv.org/abs/2512.12924) (2025), the regime split of plus 2.4 percent against
  minus 0.16 percent a year; identifier `2512.12924v1`.
- [objective.rs](../../../crates/analysis/src/objective.rs), this repository's weighted objective and
  its constraints.

## Words used in this tutorial

- baseline: the run on the data used to choose the settings, against which the held-back run is
  measured.
- coefficient of variation: the spread of a set of numbers divided by the size of their average, a
  scale-free measure of disagreement.
- consistency: one minus the coefficient of variation, held between nothing and one.
- deepest fall from a peak: the largest drop in account value from a previous high, called the
  maximum drawdown.
- held-up fraction: the held-back value divided by the baseline value, arranged so bigger is better.
- stability: one minus the spread divided by a spread you consider the worst tolerable.
- weight: how much a measurement counts in a combined score.
- window: one period of a split run, such as one quarter of a two-year test.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
