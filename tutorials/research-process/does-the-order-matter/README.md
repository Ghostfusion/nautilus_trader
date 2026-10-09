# Shuffling the past to ask whether the order mattered

Date: 2026-10-08. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                                                                                                                              |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| What it trades            | Nothing by itself: it is a test that rebuilds the same price changes in a different order and re-runs the rule on each rebuild                                                                                                                                                                                                                                                                                     |
| How often it trades       | Not applicable; the test runs the rule once on the real history and once per shuffle, usually a hundred times or more                                                                                                                                                                                                                                                                                              |
| What you need             | Nothing but this page; the worked example needs only a table of six numbers                                                                                                                                                                                                                                                                                                                                        |
| Where the rules come from | [The permutation test in this repository](../../../python/nautilus_trader/optimization/permutation.py), written to the procedure below, and [the version in the investing-algorithm-framework repository](https://github.com/coding-kitties/investing-algorithm-framework/blob/main/investing_algorithm_framework/infrastructure/services/backtesting/backtest_service.py), which gets one step of it wrong        |
| The underlying research   | Mroziewicz and Slepaczuk, [A novel approach to trading strategy parameter optimization](https://arxiv.org/abs/2602.10785) (2026), whose third research question is whether the tested rule beats randomly constructed alternatives, answered by bootstrap; Deep, Deep and Lamptey, [Interpretable Hypothesis-Driven Trading](https://arxiv.org/abs/2512.12924) (2025), on reporting the resulting p-value honestly |
| How well it held up       | Open question: the test is standard in the literature and the two papers above rely on it or on its bootstrap cousin, but this collection found no measurement of the exact rule on a rule of its own, and the version quoted above permutes the four daily prices independently, which produces days that could not have existed                                                                                  |
| Also appears in           | [Searching settings](../sweeping-settings/README.md), [Held-back data comes in two flavours](../two-kinds-of-out-of-sample/README.md), [Scoring what survived](../scoring-what-survived/README.md), and [How a backtest lies](../../foundations/07_how-a-backtest-lies.md)                                                                                                                                         |

## The idea in one paragraph

A backtest reports what a rule would have done on one particular sequence of prices. Some of that
result comes from the rule, and some of it comes from the sequence: a rule that buys after a rise
looks brilliant in a history that rose a lot. To separate the two, keep every price change exactly
as it was, throw away only the order they occurred in, rebuild a history, and run the same rule on
it. Repeat that a hundred times, each time with a different random order, and count how often the
rearranged histories did as well as the real one. That count, divided by the number of rearrangements,
is the answer. A small share means the rule did something the order of events mattered for; a share
near one half means it did nothing that this reshuffling can detect.

## Why anyone believed it

The reason to believe the test is that it asks a question the rule's author cannot answer by
choosing a better sample. It does not compare the rule with other rules, and it does not need a
second period of history. It uses the very prices the rule was tested on, so there is nothing to
choose and nothing to leak: the shuffled worlds contain the same rises, the same falls and the same
quiet days as the real one, only in a different sequence.

The assumption underneath is that the days are exchangeable - that if the rule has no real
relationship with the market, then the order of the days is irrelevant to how it scores. Where that
assumption fails, the test answers a different question, and the failure modes section below says
which ones and in which direction. The most common failure is not conceptual but mechanical: if the
rebuilding step produces prices that could not have existed, the test is measuring the effect of
malformed data rather than the effect of order.

## An everyday comparison

A card sharp claims he can tell you the next card. To test him, take yesterday's deck, which you
both saw, and offer to deal him a fresh hand. If he knows nothing, his accuracy over a hundred
deals will be about what guessing gives. But suppose you build the fresh hands by taking the top
card from one pile, the middle card from another and the bottom card from a third, all three drawn
from that same yesterday deck. You have kept every card, and thrown away only the order - and you
have also created hands that never existed, with two aces of spades. Any answer he gives is now
about your shuffling, not about his skill. That is exactly the mistake the framework quoted in the
provenance table makes with prices.

## The rules, step by step

1. Freeze the rule and run it on the real history. Record its score on each measurement you care
   about, such as average return, the ratio of return to variability, or the deepest fall from a
   peak.
2. Reduce the history to its changes rather than its levels. If the rule reads four prices a day,
   take four changes per day: the open against the previous close, the high against the open, the
   low against the open, and the close against the open.
3. Shuffle whole days, not single numbers. Keep each day's four changes together as one block and
   reorder the blocks. This keeps every day's internal shape - its own rise or fall and the distance
   between its high and its low - and it is the step the framework quoted above does not take.
4. Rebuild a price history from the shuffled blocks, and check it as you go: every rebuilt day must
   have a high at or above both its open and its close, and a low at or below both. If a rebuilt day
   fails that, the shuffle is malformed and the run must be discarded.
5. Run the frozen rule on the rebuilt history and record the same scores.
6. Repeat the shuffle at least a hundred times. Seed the random number generator inside the shuffle
   function, so that the same seed reproduces the same set of shuffled worlds and two people get the
   same answer.
7. Count how many shuffled runs scored at least as well as the real one, in the direction that
   favours the rule. For measurements where smaller is better, such as the deepest fall from a peak
   or the volatility of returns, count the runs that scored at least as badly instead.
8. Divide that count by the number of shuffled runs. That fraction is the p-value, and it is the
   answer to the question "how often does a world with the same price changes in a different order
   do this well".
9. Report it beside the real score, the number of shuffled runs, and the unit you shuffled. A
   p-value reported on its own, with no statement of what was rearranged, cannot be checked.

## The maths, with every symbol named

The reshuffled day, rebuilt from four changes that have been moved independently:

```text
open_today  = close_yesterday * exp(change_open)
high_today  = open_today * exp(change_high)
low_today   = open_today * exp(change_low)
close_today = open_today * exp(change_close)
```

- `exp` is the exponential function, which turns a change measured in logarithms back into a price
  ratio.
- `change_open` is the day's open against the previous close, in logarithms, and the other three are
  the day's high, low and close against that day's open.
- It means: the rebuild assumes the day's four changes are independent of each other, which is why
  moving them separately lets a high end up below a low.

The p-value:

```text
p = number_of_shuffled_runs_at_least_as_good / number_of_shuffled_runs
```

- `number_of_shuffled_runs_at_least_as_good` counts the rebuilt histories whose score was at least
  the real score, or at least as bad for measurements where smaller is better.
- `number_of_shuffled_runs` is how many rebuilds were made, at least a hundred.
- It means: the share of rearranged worlds that did as well. Small means the order mattered to the
  result. It does not mean the result is profitable.

The score whose extremes you care about, the ratio of return to variability:

```text
ratio = average_daily_return / standard_deviation_of_daily_returns
```

- `average_daily_return` is the mean of the day-to-day percentage changes in the account value.
- `standard_deviation_of_daily_returns` is how far those daily changes typically sit from their own
  average.
- It means: return per unit of wobble. The test can be run on this ratio, on the return itself, or
  on the deepest fall from a peak.

## A worked example

The real run reports a ratio of 1.5. A hundred shuffled histories are built by reordering whole
days, which is the sound version of step 3, and the frozen rule is re-run on each. The lowest and
highest are shown, and the four runs that matter for the count.

| Run               | Order of the days              | Ratio of return to variability | At least as good as the real 1.5 |
| ----------------- | ------------------------------ | ------------------------------ | -------------------------------- |
| Real history      | The actual sequence            | 1.50                           | not counted                      |
| Shuffled run 1    | Random order, seed 41          | -0.20                          | no                               |
| Shuffled run 2    | Random order, seed 42          | 0.35                           | no                               |
| Shuffled run 3    | Random order, seed 43          | 1.90                           | yes                              |
| Shuffled run 4    | Random order, seed 44          | 1.52                           | yes                              |
| Remaining 96 runs | Random orders, seeds 45 to 140 | Between -0.60 and 1.80         | 2 of the 96, both near 1.6       |

Four of the hundred shuffled runs scored at least 1.5, so the p-value is 4 divided by 100, which is
0.04. The reading is that a history with the same day-to-day changes in a different order reaches
1.5 about four times in a hundred, so the real sequence was unusual. That is a statement about the
order of the prices. It is not a statement that the rule pays: the fees, the borrowed money and the
market impact of the real trades have not been counted, and the shuffled worlds contain no real
trades at all.

Now the malformed version, which is what the framework in the provenance table actually does. It
takes the four daily changes, shuffles each of the four lists on its own, and rebuilds the day with
all four changes taken from places that no longer belong together. Suppose the previous close was
100, and the shuffled changes drawn for one day are +0.000 for the open, -0.005 for the high, -0.002
for the low and +0.010 for the close.

| Part of the day | Change drawn | Rebuilt price | How it is computed    |
| --------------- | ------------ | ------------- | --------------------- |
| Open            | +0.000       | 100.0000      | 100 times exp(0.000)  |
| High            | -0.005       | 99.5012       | 100 times exp(-0.005) |
| Low             | -0.002       | 99.8002       | 100 times exp(-0.002) |
| Close           | +0.010       | 101.0050      | 100 times exp(0.010)  |

The rebuilt high, 99.50, is below both the rebuilt low, 99.80, and the rebuilt close, 101.01, so this
is a day that could not have happened: its high is not the highest price of the day. A rule that
buys at the high, or that measures the day's range, is now being fed nonsense, and the p-value it
produces describes the nonsense rather than the market. The same function leaves the traded volume
in its original order while every price has moved, so volume and price no longer line up either.

## What the research actually found

- The window-length study asks its third research question in almost the words of this page: is the
  tested rule better than random? It answers with a bootstrap, generating randomly constructed
  alternatives and comparing the rule with them, and reports its conclusions on that basis
  (`2602.10785v1`). The bootstrap and the shuffle differ in how the alternative histories are made,
  but the logic is the same: keep the changes, break the order, see how often luck wins.
- The validation study reports its aggregate p-value rather than hiding it: 0.34, on a rule that
  returned 0.55 percent a year, and it presents that as the honest outcome of the procedure
  (`2512.12924v1`). A reader who wants to know what a non-significant result looks like in print
  can read that paper's results section.
- The survey of publication bias explains what a small p-value does and does not buy. Its four facts
  include that published test statistics are far above 2.0 and that almost all published findings
  replicate - and its own correction still shrinks reported returns by 10 to 15 percent, and finds
  the chance of a finding pointing the wrong way to be under 10 percent (`2209.13623v3`). A p-value
  of 0.04 from a shuffle test, standing alone, is therefore a weak form of evidence that the
  literature has already learned to discount.
- The framework that prompted the page also records which of its ten scored measurements are treated
  as better when smaller - the volatility of returns and the deepest fall from a peak - and flips
  the comparison for those two. That direction rule is the same one given in step 7 above, and it
  is the only part of its implementation this page adopts without change
  ([the test's result container](https://github.com/coding-kitties/investing-algorithm-framework/blob/main/investing_algorithm_framework/domain/backtesting/backtest_monte_carlo_test.py)).

## How this project relates to it

This repository ships the procedure above in
[permutation.py](../../../python/nautilus_trader/optimization/permutation.py). It rearranges one of
two units: the realized outcomes of the trades themselves, or blocks of consecutive period returns,
where a block keeps its internal order so that runs of calm and of turbulence stay together. The
statistic is supplied by the caller rather than fixed, so the test can be run on a return, on the
ratio of return to variability, or on the deepest fall from a peak, and the same statistic is
applied to the real sequence and to every rearrangement. Three choices in it answer the defects
above: nothing is ever reconstructed from separate price changes, so a rearranged value cannot be
one that was never observed; the random generator is created inside the call and the seed is
recorded with the result, so two runs with the same seed agree and nothing else in the process can
move them; and a rearrangement the statistic cannot score is counted and excluded from the share
rather than silently dropped. The result also carries the direction rule of step 7, so a
measurement that is better when smaller is counted the other way, and it can write itself out as a
flat record naming the seed, the counts and the unit. It is a report and never a gate: nothing in
the module is consulted by a strategy, an order or a risk check.

The tests show it working, and they are the one command that runs the rule from the repository's
`python` directory:

```bash
python/.venv/Scripts/python.exe -m pytest tests/unit/optimization/test_permutation.py -q
```

Two neighbours are worth naming, because each answers a different question. The significance report
at [significance.py](../../../python/nautilus_trader/optimization/significance.py) adjusts a
reported ratio for the number of trials rather than for the order of the prices; the trial count is
the subject of [Searching settings](../sweeping-settings/README.md), and the two ways of keeping
data away from the search are in
[Held-back data comes in two flavours](../two-kinds-of-out-of-sample/README.md). And the
measurements the test scores are the single-run ones listed under
[crates/analysis/src/statistics](../../../crates/analysis/src/statistics): the test is a way to
re-use the measurements a repository already has, not a new measurement.

## Where it goes wrong

- The four daily prices are permuted separately, which is the defect quoted at length above. It
  produces days whose high is below their close and days whose low is above their high, so the run
  measures the reshuffling, not the strategy. Shuffling whole days avoids it at no cost.
- The volume is left in its original order while the prices move. Any rule that reads volume, and
  any measure of the relationship between volume and price, is then comparing two series that were
  cut apart.
- The random number generator is seeded globally inside the loop rather than locally, which makes
  the test un-reproducible the moment two of these runs are done at once, and makes the p-value
  depend on everything else that used random numbers first.
- Shuffling days destroys the pattern of quiet and busy periods. A rule that trades changes in
  volatility, rather than changes in direction, will look worse against shuffled histories than it
  deserves, because the shuffled worlds have no runs of calm or of turbulence at all. A block
  shuffle, which moves groups of twenty days instead of single days, keeps more of that structure
  and is the usual repair.
- The direction of the comparison must be fixed before the count. Volatility and the deepest fall
  from a peak are better when smaller; counting them in the same direction as the return inflates
  the p-value and makes a rule look worse than it is, and choosing the direction after seeing the
  number is a second search.
- A small p-value is not a payday. Nothing in this procedure includes fees, borrowing costs or the
  effect of the rule's own orders on the price, and a rule can be genuinely different from its
  shuffled copies while still losing money after those three.
- The test presumes the days are exchangeable. Where a rule exploits a real, persistent property of
  markets - a slow drift, an upward tendency in shares - the shuffled worlds do not contain it, so
  the rule beats them for a reason that has nothing to do with the rule being well chosen.
- One convention question must be settled and stated: some writers report the count divided by the
  number of runs, which can be exactly zero, and others add one to both the count and the number of
  runs so that the answer never reaches zero. Either is defensible; mixing them across studies is
  not, and a reader cannot tell them apart without the number of runs being reported.

## Try it yourself

You need a calculator and a table of returns. No code.

1. Write down ten made-up daily returns as percentages: for example -1.0, +0.5, +2.0, -1.5, +0.3,
   +1.1, -0.7, +0.4, -2.2, +1.6.
2. Multiply them together as growth factors - 1 plus each return - to get the total return of
   holding this sequence.
3. Now write the same ten numbers in a different order, and multiply again. Do it three more times.
4. Record the four totals and the original one, and find the highest and lowest.
5. Decide whether the original sequence was unusually good compared with the four reshuffles.
6. Now do it for a rule: buy on a day after a rise, sell after a fall, and compute what that rule
   would return in the original order and in each reshuffle.

What to notice: for the plain holding of step 2 the order does not matter at all - the product is
the same in every arrangement - while for the rule in step 6 the results spread out. That spread is
what the shuffle test measures, and it is why a rule needs a reason to trade rather than to hold.

## Where this came from

- Mroziewicz and Slepaczuk, [A novel approach to trading strategy parameter
  optimization](https://arxiv.org/abs/2602.10785) (2026), whose third research question asks whether
  the rule beats randomly constructed alternatives and answers it by bootstrap; identifier
  `2602.10785v1`.
- Deep, Deep and Lamptey, [Interpretable Hypothesis-Driven
  Trading](https://arxiv.org/abs/2512.12924) (2025), which reports an aggregate p-value of 0.34
  alongside its own modest returns; identifier `2512.12924v1`.
- Chen and Zimmermann, [Publication Bias in Asset Pricing Research](https://arxiv.org/abs/2209.13623)
  (2023), on what a small p-value is worth once a whole field has searched; identifier
  `2209.13623v3`.
- [The permutation helper](https://github.com/coding-kitties/investing-algorithm-framework/blob/main/investing_algorithm_framework/infrastructure/services/backtesting/backtest_service.py)
  and [the result container](https://github.com/coding-kitties/investing-algorithm-framework/blob/main/investing_algorithm_framework/domain/backtesting/backtest_monte_carlo_test.py)
  in the investing-algorithm-framework repository, read at commit `f8e82c7` on the main branch: the
  four separate shuffles, the rebuild that takes the high, the low and the close from the open, the
  unshuffled volume, the global seed, the ten scored measurements and the two that are better when
  smaller.
- [How a backtest lies](../../foundations/07_how-a-backtest-lies.md), this repository's primer on
  the ways a history can mislead.

## Words used in this tutorial

- block: a group of consecutive days moved together during a shuffle, which preserves the shape of
  the period rather than scrambling its days.
- exchangeable: describing days whose order does not matter to the answer, which is the assumption
  the test rests on.
- log return: the change in price measured in logarithms, which can be added up across days and
  turned back into a price ratio with the exponential function.
- null hypothesis: the world the test imagines, here that the rule has no relationship with the
  market beyond its day-to-day changes.
- one-sided: counting only the shuffled runs that did better than the real one, rather than both
  directions.
- p-value: the share of shuffled runs that did at least as well as the real one.
- permutation test: a test that rearranges the data instead of collecting more of it.
- seed: the number that fixes a random sequence, so that the same seed produces the same shuffles.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
