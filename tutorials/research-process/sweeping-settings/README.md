# Searching settings, and why the winner looks too good

Date: 2026-10-08. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                      |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Nothing by itself: it is the order in which the candidate settings of a rule are tested, and the record that testing leaves behind                                                                                                                                                                         |
| How often it trades       | Not applicable; every candidate setting is run once over the same stretch of history                                                                                                                                                                                                                       |
| What you need             | Nothing but this page; the worked example needs only a table of four numbers and a calculator                                                                                                                                                                                                              |
| Where the rules come from | [The significance report in this repository](../../../python/nautilus_trader/optimization/significance.py), which records the number of trials with each reported ratio, and [the objective weights](../../../crates/analysis/src/objective.rs) used to rank candidates                                    |
| The underlying research   | Pav, [Post-Selection Estimation of Sharpe Ratios](https://arxiv.org/abs/2606.01650) (2026), on what a selected ratio really estimates; Chen and Zimmermann, [Publication Bias in Asset Pricing Research](https://arxiv.org/abs/2209.13623) (2023), on how large the correction turns out to be in practice |
| How well it held up       | Strong: several independent collections of published results agree that searching flatters the winner, and one meta-study measures the correction at 10 to 15 percent of the reported return, though the size depends on assumptions the papers state                                                      |
| Also appears in           | [Held-back data comes in two flavours](../two-kinds-of-out-of-sample/README.md), [Shuffling the past](../does-the-order-matter/README.md), [Scoring what survived](../scoring-what-survived/README.md), and [How a backtest lies](../../foundations/07_how-a-backtest-lies.md)                             |

## The idea in one paragraph

A rule almost always has settings: how many days to average, how far the price must move before you
act, how many positions to hold. Testing the rule means testing many combinations of those settings
on the same history and keeping the combination that scored best. The best of many tries is high
even when none of the tries has any real edge, in the same way that the best coin-flipper in a large
room looks gifted. The procedure here is to count the tries, write the count down beside the result,
and raise the bar the result must clear according to how many tries there were. It does not make a
good rule out of a bad one. It stops a coincidence from being reported as a discovery.

## Why anyone believed it

The pressure is simple. A number is requested, and a search produces one: the best combination will
always have a positive score, because it is the maximum of a set of numbers. Nobody in the chain
needs to intend any deception. The person who ran the search reports the winner, the winner is what
gets written down, and the discarded combinations leave no trace.

The belief that this is fine rests on an unstated assumption: that the settings are part of the
strategy's logic rather than a fit to this particular history. For a genuine mechanism, that can be
true. If a rule trades a real effect that appears over many years, most reasonable settings of it
will show something, and the spread between them will be narrow. What the arithmetic below shows is
that the spread is also wide enough for pure noise to produce an impressive winner, so the width of
the spread is the thing to measure rather than assume.

## An everyday comparison

Ninety-six people each flip a fair coin ten times. The best of them will get eight or nine heads -
someone always does - and that person can be introduced as the room's expert on coins. Nothing about
their technique caused the result; the size of the room caused it. If the same ninety-six people
flip again tomorrow, the room's best result will be about the same, but the person holding it will
usually be somebody else. A settings search is the room, the settings are the people, and the score
is the coin count.

## The rules, step by step

1. Write down the grid before running anything: each setting, and every value of it you intend to
   try. Multiply the counts to get the number of combinations. A grid of two timeframes, two
   averaging periods, two thresholds, two risk levels, three waiting periods and two directions has
   2 x 2 x 2 x 2 x 3 x 2, which is 96 combinations.
2. Choose the single scoring measure in advance, and write it down. Choosing it after seeing the
   results is a second search, and it is not counted anywhere.
3. Run every combination over the same stretch of history, collecting the score for each. Keep all
   96 scores, not only the winner.
4. Write down the spread of the 96 scores: the best, the worst and the typical one. A grid where the
   best is far above the typical is a grid of noise; a grid where most combinations score about the
   same is a grid where the setting hardly matters.
5. Prune the candidates that never traded enough to be judged. A workable set of gates: at least one
   closed trade in a window, the first three windows excluded as warm-up, trades in at least half
   the windows, unprofitable windows at most half, and a net gain above zero across the whole
   sample.
6. Hold back data before step 3, not after, and grade the winner on the held-back data only; the
   next page describes the two ways to do that.
7. Count the tries that did not survive the gates, and add the settings you tried in earlier
   sessions and discarded. The count is the input to the arithmetic below, and a count that omits
   the discards is the most common way this whole procedure is defeated.
8. Raise the bar. For a grid of 96, the best score expected from pure luck is about 3.0 standard
   units, so a winner at 1.5 has produced nothing.
9. Report the winner together with the count, the spread and the held-back score. A bare winner is
   not a result.

## The maths, with every symbol named

The score of a candidate, ranked within the set of candidates tried:

```text
score = sum over metrics of (weight * (value - smallest) / (largest - smallest))
```

- `metric` is one thing being scored, such as the ratio of return to variability, or the deepest
  fall from a peak.
- `weight` is how much that metric counts in the final ranking, negative for metrics where smaller
  is better.
- `value`, `smallest` and `largest` are the candidate's number and the extremes of that metric
  across all candidates tried.
- It means: every candidate is marked relative to the others in the same search, so a number on its
  own says nothing without the set it came from.

The expected best of a set of independent lucky draws, each measured in standard units:

```text
expected_best = square root of (2 * natural_logarithm_of(number_of_tries))
```

- `number_of_tries` is the count from step 7: every setting, including the discarded ones.
- `expected_best` is in standard units, so it can be read directly against a ratio of return to
  variability.
- It means: the best result a search produces by luck grows with the size of the search, slowly but
  without limit. Ninety-six tries average about 3.0; a million tries average about 5.3.

The bar a result must clear, stated as the usual test statistic:

```text
t = average_return / (standard_deviation / square root of number_of_observations)
```

- `average_return` is the mean of the period returns.
- `standard_deviation` is how far those returns typically sit from their mean.
- `number_of_observations` is how many periods the returns cover.
- It means: the statistic grows with the length of the sample and with the size of the effect. The
  researchers cited below argue that after a whole field has searched, the bar should be 3.0 rather
  than the traditional 2.0.

## A worked example

Suppose you test 96 combinations of settings over ten years of daily prices, and the best one
reports a return divided by variability of 1.8. Is that good? Work out what luck alone would have
produced.

| Number of tries | Natural logarithm | Two times the logarithm | Square root, the expected best |
| --------------- | ----------------- | ----------------------- | ------------------------------ |
| 10              | 2.302585          | 4.605170                | 2.15                           |
| 50              | 3.912023          | 7.824046                | 2.80                           |
| 96              | 4.564348          | 9.128696                | 3.02                           |
| 1,000           | 6.907755          | 13.815511               | 3.72                           |
| 1,000,000       | 13.815511         | 27.631021               | 5.26                           |

For 96 tries the expected best is 3.02. The reported winner is 1.8, which is below what the search
would have produced with no edge at all, so the winner has told you nothing and the search's own
noise has been the larger term. Now put two real published numbers beside it. A study of
microstructure signals over 100 American shares reports a ratio of return to variability of 0.33 on
data it never searched (`2512.12924v1`), and the paper on post-selection estimation notes that daily
ratios above about 2.5 a year are already generous for a real edge (`2606.01650v2`). A search of 96
settings reaches 3.0 by accident, which is above both. The arithmetic does not say that no rule can
have an edge; it says that the size of the search sets a floor under what any winner will look like.

Costs make the comparison worse. Suppose the winner trades 200 times a year and each round trip
costs 0.1 percent. That is 20 percent a year in costs before any gain, which is more than the whole
edge the reported ratios correspond to.

## What the research actually found

The sizes below are what the sources measured, not what any rule can be expected to do.

- The survey of published predictors states four facts from large collections of studies: almost all
  findings can be replicated, predictability persists out of sample, published test statistics are
  much larger than 2.0, and predictors are only weakly related to each other. Turning those facts
  into a correction, the average shrinkage of the reported return is 10 to 15 percent, and the
  chance of a finding pointing in the wrong direction is under 10 percent. The same survey reports
  that procedures which adjust for many trials use hurdles above 3.0, and that returns are 30 to 50
  percent weaker when portfolios are built a different way (`2209.13623v3`).
- A review of validation practice reports the opposite tail: of 452 published anomalies, 65 percent
  fail a single significance test using value-weighted returns, rising to 82 percent once multiple
  testing is accounted for; separately, 97 predictors lose 26 percent of their return out of sample
  and 58 percent after publication (`2512.12924v1`, reporting Hou and co-authors, and McLean and
  Pontiff).
- The paper on post-selection estimation takes the situation as given: strategies are devised and
  tested in the thousands, the best in-sample ratio is selected, and the honest question is what the
  selected one really estimates. Its simulations find a shrinkage estimator outperforms the raw
  ratio across realistic sample sizes and numbers of candidates (`2606.01650v2`).
- A study of a moving average rule over intraday data makes the count explicit and reports it: 81
  combinations of training and testing window lengths, an optimisation period of 19 months, then two
  settings carried into a held-back 21-month period and executed once. On the held-back data the
  rule performed similarly to simply holding the asset, with a smaller fall from the peak; the
  paper's own sensitivity check puts the fee at which it stops paying at about 0.4 percent per
  transaction (`2602.10785v1`).

The disagreement between the first two bullets is informative rather than confusing. The survey
found that corrections shrink reported returns by 10 to 15 percent; the replication studies found
that between a fifth and four fifths of published findings do not survive a stricter test. Both can
be true: most replicated findings are real, and the tail that is not is large enough to matter for
anyone who reads one paper at a time.

## How this project relates to it

This repository implements the count rather than the anecdote. The significance report at
[significance.py](../../../python/nautilus_trader/optimization/significance.py) contains a
deflated-ratio calculation that takes the number of trials as an input and adjusts the reported
ratio for it, together with a declaration of which statistical assumptions it is making, so the
correction cannot be applied silently. The candidate ranking lives at
[objective.rs](../../../crates/analysis/src/objective.rs), where each scored metric carries a weight
and a direction, which is the mechanism the formula at the top of this page describes.

Two further links. [How a backtest lies](../../foundations/07_how-a-backtest-lies.md) is the primer,
and it carries the same trial-count arithmetic with the number of published strategies behind it.
The runnable example of a condition that refuses to trade sits in
[validity_gated_walk_forward.py](../../../examples/backtest/validity_gated_walk_forward.py), run from
the repository root with:

```bash
python examples/backtest/validity_gated_walk_forward.py
```

It prints how often it evaluated its condition, how many of those evaluations passed, how many
trades it took, and the final account value, which is the count-then-report habit this page asks
for.

## Where it goes wrong

- The count is rarely honest. Nobody records the settings they tried last month, the thresholds
  they changed after seeing the chart, or the rule that was abandoned. The arithmetic is only as
  good as the count, and the count is the one input nobody can check.
- The formula overstates the penalty in some cases and understates it in others. Candidates in a
  settings grid are related to each other - neighbouring settings produce nearly the same trades -
  so the effective number of independent tries is smaller, which makes 3.0 too high. Where several
  rules were mined from the same data, the effective number is larger, which makes it too low.
- A gate can hide a fragile winner. Requiring trades in at least half the windows and unprofitable
  windows not to exceed half is a reasonable filter, but it also drops the settings whose weakness
  showed up as inactivity, and the survivors are then scored on the windows that suited them.
- The scoring measure decides the winner. Rank by the ratio of return to variability and you get
  the settings with the highest variability-adjusted result; rank by total return and you get the
  settings that took the most risk. Both are called "the best settings".
- The held-back test is usually run by the person who did the search, who stops at the first
  convenient result. Every additional look at the held-back data turns it into another in-sample
  sample.
- The correction is not a verdict. A deflated ratio of 1.2 after 500 tries is a better statement
  than a raw ratio of 2.1, but it is still one estimate from one sample, and it says nothing about
  the costs of the next ten years. The published decay numbers above were measured on predictors
  that were documented before they were traded, and they still lost a quarter to a half of their
  return once they were known.

## Try it yourself

You need a table of numbers you have already seen, and a calculator. No code.

1. Take any published table of strategy results you can find, and count the rows. That count is your
   number of tries.
2. Compute the natural logarithm of the count, double it, and take the square root. Write the answer
   down as the lucky best.
3. Convert the table's best reported result into standard units if the table gives a ratio of return
   to variability; if it reports only a return, note the columns you would need to compute one.
4. Compare the lucky best with the reported best. If the reported best is below the lucky best, the
   table has told you nothing.
5. Repeat with a table of 10 rows and one of 1,000 rows.

What to notice: as the table gets longer the lucky best rises, so a bigger search needs a bigger
result to be interesting. This is why a rule with three settings that were set years ago and never
changed is different evidence from a rule with three settings that were chosen from a grid of a
thousand.

## Where this came from

- Pav, [Post-Selection Estimation of Sharpe Ratios](https://arxiv.org/abs/2606.01650) (2026), the
  estimated ratio after choosing the best in-sample one, and the shrinkage estimators compared;
  identifier `2606.01650v2`.
- Chen and Zimmermann, [Publication Bias in Asset Pricing Research](https://arxiv.org/abs/2209.13623)
  (2023), the four facts, the 10 to 15 percent shrinkage, the false-discovery rate and the 3.0
  hurdle; identifier `2209.13623v3`.
- Deep, Deep and Lamptey, [Interpretable Hypothesis-Driven Trading](https://arxiv.org/abs/2512.12924)
  (2025), which reports the replication numbers of Hou and co-authors and of McLean and Pontiff in
  its review, and its own modest out-of-sample result; identifier `2512.12924v1`.
- Mroziewicz and Slepaczuk, [A novel approach to trading strategy parameter
  optimization](https://arxiv.org/abs/2602.10785) (2026), the 81 window-length combinations, the
  held-back period and the break-even fee; identifier `2602.10785v1`.
- [The parameter-sweep notebook](https://github.com/coding-kitties/investing-algorithm-framework/blob/main/examples/tutorial/notebooks/03_in_sample_param_sweep.ipynb)
  in the investing-algorithm-framework repository, which states the 96-combination grid and the
  pruning gates quoted in the rules above.
- [Overfitting, reproducibility and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md),
  this repository's brief on the same subject, which is where the trial-count arithmetic is applied
  to the strategy corpus.

## Words used in this tutorial

- backtest: a test of a rule on past prices to see what it would have done.
- in-sample: the stretch of history used to choose the settings.
- out-of-sample: history kept away from the choosing, used once to test the frozen rule.
- p-value: the share of random reruns that did at least as well as the real one; a small value means
  the real result is unusual among them.
- return: the percentage gain or loss a position produces over a period.
- Sharpe ratio: average return divided by the variability of return; a score with no units.
- standard deviation: how far a set of numbers typically sits from its own average.
- t-statistic: average return divided by its own uncertainty; the number that significance tests
  compare against a bar.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
