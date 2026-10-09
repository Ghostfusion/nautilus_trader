# Held-back data comes in two flavours

Date: 2026-10-08. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                                                            |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| What it trades            | Nothing by itself: it is the choice of which dates and which instruments are kept away from the search, and the order in which they are used                                                                                                                                                                                                     |
| How often it trades       | Not applicable; the held-back data is used once per study, and a study may hold back two different samples at once                                                                                                                                                                                                                               |
| What you need             | Nothing but this page; the worked example needs only a calendar and a subtraction                                                                                                                                                                                                                                                                |
| Where the rules come from | [The split contract and leakage policy in this repository](../../../python/nautilus_trader/optimization/splits.py), which states the rules for purging and embargoing the boundary between a training stretch and a test stretch                                                                                                                 |
| The underlying research   | Mroziewicz and Slepaczuk, [A novel approach to trading strategy parameter optimization using double out-of-sample data and walk-forward techniques](https://arxiv.org/abs/2602.10785) (2026), and Deep, Deep and Lamptey, [Interpretable Hypothesis-Driven Trading](https://arxiv.org/abs/2512.12924) (2025), on repeated held-back test periods |
| How well it held up       | Mixed: held-back testing is standard and its value is measured repeatedly, but the numbers that matter most are the two independent decay measurements - 26 percent lost out of sample and 58 percent after publication - which say that a single held-back period proves less than readers assume                                               |
| Also appears in           | [Searching settings](../sweeping-settings/README.md), [Shuffling the past](../does-the-order-matter/README.md), [Regimes and why nothing lasts](../../foundations/08_regimes-and-why-nothing-lasts.md), and [How a backtest lies](../../foundations/07_how-a-backtest-lies.md)                                                                   |

## The idea in one paragraph

The settings of a rule are chosen on some data, and that same data can no longer be used to judge
the rule, because the rule was fitted to it. So a second stretch of data is kept aside before the
search begins and used once afterwards. That second stretch can be kept aside in two independent
ways: by taking dates the search never saw, and by taking instruments the search never saw. The two
answer different questions. Dates ask whether the rule still works in a different market period;
instruments ask whether the rule works on things it was not tuned for. A study can do both, and the
mistake this page is about is thinking that one held-back stretch, used once, has answered the
question on its own.

## Why anyone believed it

The idea of holding data back comes from medicine and from machine learning, where it is the
standard defence against a result that fits one sample and nothing else. It is attractive because it
is cheap: nothing about the rule changes, only the order in which history is presented to it.

The belief that one held-back stretch is enough rests on an assumption that is rarely stated: that
the sample kept aside is drawn from the same world as the sample used to fit. Trading breaks that
assumption in a specific way. A rule fitted on a market that rose for five years may be a bet on the
market rising, and the held-back period may also be a rising market, so the test passes while
teaching nothing. The two flavours of held-back data exist because the two failures they catch -
the world changed, or the instrument is different - are not the same failure.

## An everyday comparison

A language teacher writes a practice exam for her class and then writes the real one. If she writes
the real exam by copying the practice questions and changing one number, the class will do well and
she will have learned nothing about their French. Avoiding the cheat is easy: write the real exam
first, hand out only part of it as practice, and set the rest on different days. But there is a
second way to fool herself that is not about copying at all: choosing the ten best students from a
class of ninety-six on the practice exam, and then being surprised when their exam results are only
average. That is a held-back test in time that still answers nothing about the individual students.
The two flavours of trouble - a shared exam, and a selected group - map exactly onto the two ways a
trading result can pass a held-back test and still mean nothing.

## The rules, step by step

1. Fix the whole period you have data for before you look at any result, for example January 2015
   to December 2024.
2. Split it into a fitting stretch and a held-back stretch, and write the dates down with a rule,
   not by eye: for instance fit on 2015 to 2021 and hold back 2022 to 2024.
3. Put a gap between them of at least the longest look-back the rule uses. If the rule averages 200
   daily prices, the first test day sees 200 days before it, so any test day inside the fitting
   stretch leaks. A gap of 30 days covers any rule that looks back a month; a rule that looks back a
   year needs a gap of a year.
4. Choose a second held-back sample that is not a period but a set of instruments: the same dates as
   the fitting stretch, different symbols. Nothing about the dates is withheld; the point is that
   these instruments were never used to choose the settings.
5. Freeze the rule. No setting may be changed after seeing either held-back sample. Each change
   turns the sample into fitting data and the honest count of tries goes up by one.
6. Run the frozen rule on both held-back samples exactly once, and report both, including the case
   where the second is worse than the first.
7. Report the fitting result, both held-back results, and the gap you used. A held-back result with
   no stated gap is uninterpretable, because the reader cannot tell whether the boundary leaked.
8. If the samples disagree - the rule surviving the time test and failing the instrument test, or
   the reverse - report that as the finding. The disagreement is the most informative outcome the
   procedure can produce.
9. Keep the split fixed across studies of the same rule, so that a later study with a longer fitting
   stretch states which held-back data it gave up.

## The maths, with every symbol named

The share of the held-back stretch that the search could also see:

```text
leak_fraction = overlap_days / held_back_days
```

- `overlap_days` is the number of dates that fall inside both the fitting stretch and the held-back
  stretch, counted after the gap has been applied.
- `held_back_days` is the number of dates in the held-back stretch.
- It means: the fraction of the test that is not a test. Zero is the only acceptable value, and the
  gap is what makes it zero for a rule with a look-back.

The size of the gap a rule needs:

```text
minimum_gap_days = longest_lookback_in_days
```

- `longest_lookback_in_days` is the longest stretch of past prices any part of the rule reads,
  including any indicator that is itself smoothed.
- It means: the first day of the held-back stretch must be at least that far from the last day of
  the fitting stretch, or the first test days are being decided by data the search used.

The two held-back questions, stated as a pair:

```text
time_result = score of the frozen rule on dates the fitting never covered
instrument_result = score of the frozen rule on symbols the fitting never covered
```

- `score` is whatever single measure was fixed in advance, such as average return per year or the
  ratio of return to variability.
- It means: two numbers, from two samples that share nothing with the search, and neither number
  substitutes for the other.

## A worked example

The dates below are made up for the arithmetic. The rule reads 200 daily prices, so its look-back is
200 days and the first held-back day must sit at least 200 days after the last fitting day.

The fitting stretch is 1 January 2015 to 31 December 2021, which is 2,557 days, and the held-back
stretch is 1 January 2022 to 31 December 2024, which is 1,096 days.

| Held-back stretch                   | Gap from the fitting stretch | Dates in both | Held-back days whose look-back reaches back | Leak fraction |
| ----------------------------------- | ---------------------------- | ------------- | ------------------------------------------- | ------------- |
| 1 January 2022 to 31 December 2024  | none                         | 0 days        | 200 days                                    | 0.00          |
| 1 January 2022 to 31 December 2024  | 30 days                      | 0 days        | 170 days                                    | 0.00          |
| 1 January 2022 to 31 December 2024  | 200 days                     | 0 days        | 0 days                                      | 0.00          |
| 1 October 2021 to 30 September 2024 | none                         | 92 days       | 200 days                                    | 0.08          |

The two leak columns measure different mistakes. The third counts dates that belong to both
stretches, which has to be zero; the fourth counts held-back days whose indicator window reaches
into the fitting stretch, so they are computed partly from prices the search already used. With no
gap, that is the first 200 days of the held-back stretch; a 30-day gap reduces it to 170; only a gap
of 200 days, the rule's own look-back, removes it. The last row is the outright error: the held-back
stretch begins on 1 October 2021, inside the fitting stretch, so 92 of its 1,096 days, which is 0.08
of the test, were seen during the search and are not held back at all.

Now the second flavour. Suppose the fitting stretch used five symbols - Bitcoin, Ether, Cardano, Sol
and Polkadot - and the held-back sample uses five others over the same dates. The frozen rule is run
twice and reports:

| Sample                   | Dates        | Symbols                                        | Result after costs |
| ------------------------ | ------------ | ---------------------------------------------- | ------------------ |
| Fitting                  | 2015 to 2021 | Bitcoin, Ether, Cardano, Sol, Polkadot         | 1.20 per year      |
| Held back in time        | 2022 to 2024 | The same five symbols                          | 0.45 per year      |
| Held back in instruments | 2015 to 2021 | Chainlink, Avalanche, Cosmos, Algorand, Ripple | 0.30 per year      |

The honest reading is that 0.45 and 0.30, not 1.20, are what the rule has shown. The two held-back
results are lower than the fitting result, which is the usual outcome, and the instrument test is
lower than the time test, which says the advantage came partly from the five symbols it was tuned
on.

## What the research actually found

- The study of validation practice runs its framework over 34 independent held-back periods rather
  than one, across five signal types and 100 American shares from 2015 to 2024, with costs and
  position limits included. It reports annualised returns of 0.55 percent and a ratio of return to
  variability of 0.33, and states that the aggregate result is not statistically significant, with a
  p-value of 0.34. It also reports the split by regime, which is the part a single held-back period
  would have hidden: an average of plus 2.4 percent a year across the relevant periods from 2020 to
  2024, against minus 0.16 percent a year from 2015 to 2019 (`2512.12924v1`).
- The same paper's review reports the two decay measurements quoted in the provenance table: 97
  published predictors lose 26 percent of their return out of sample and 58 percent after
  publication, and of 452 published anomalies 65 percent fail a single significance test, rising to
  82 percent once many trials are accounted for (`2512.12924v1`, reporting McLean and Pontiff, and
  Hou and co-authors).
- The study of window lengths makes the discipline explicit. It parameterises the length of the
  training and testing windows, explores 81 combinations between one and 28 days over a 19-month
  optimisation period, then carries the two best settings into a 21-month period that was never
  used, and executes the rule once there. It reports that on the held-back data the rule performed
  similarly to simply holding the asset, with a smaller fall from the peak, and its own sensitivity
  check puts the fee at which the rule stops paying at about 0.4 percent per transaction, against
  the 0.1 percent charged in the main test (`2602.10785v1`).
- The same paper's third research question is whether the rule beats random alternatives, and it
  answers that with a bootstrap rather than a held-back period - which is the subject of the next
  page in this group.
- The two-flavour design is not a paper; it is the practice of one framework's tutorial, which
  labels its held-back studies as a time sample covering dates strictly before the search, and a
  universe sample covering symbols the search never saw over the search's own dates
  ([the tutorial's held-back notebook](https://github.com/coding-kitties/investing-algorithm-framework/blob/main/examples/tutorial/notebooks/05_out_sample_vector_backtest.ipynb)).
  The framework's own documentation of the two samples is a design table, not a measurement.

## How this project relates to it

This repository states the same rule as a contract in
[splits.py](../../../python/nautilus_trader/optimization/splits.py), and it goes one step further
than the sources above by naming the leak that the gap exists to prevent: a purge before and after
the boundary, an embargo period, and a check on whether a training label and a test label overlap in
time. The reason labels matter is that a label is the future the rule is being asked to predict, so
a training example whose label window reaches into the test stretch has already been given the
answer.

The primer for the wider subject is
[Regimes and why nothing lasts](../../foundations/08_regimes-and-why-nothing-lasts.md), which
explains why a held-back period that happens to sit in the same regime as the fitting period will
pass almost anything. The planned-holdings and gated examples in this collection, such as
[the gated walk-forward page](../../quant-trading/gated-walk-forward-regression/README.md), apply
the same idea inside a single price path by refitting on a moving window rather than splitting the
history once.

## Where it goes wrong

- The held-back stretch is used more than once. This is the commonest failure and the hardest to
  see, because every extra look is invisible in the final write-up. Three looks at the same
  held-back data turn it into a small fitting sample.
- One stretch in time answers one regime. The 34-period study above shows how much the answer moves
  between periods: plus 2.4 percent a year in one stretch of years and minus 0.16 percent in
  another, from the same rule and the same shares.
- The instrument sample can be structurally different rather than merely different. Holding back
  five crypto assets from a search on five crypto assets is still one kind of thing; holding back
  shares from a search on crypto is a different experiment and its result should be reported as
  such.
- The gap is chosen to fit the rule, not the rule to fit the gap. A rule with a long look-back and a
  short gap leaks silently, and the leak flatters the result because the leaked days are the days
  nearest the search period.
- Costs usually rise out of sample. The window-length study's own sensitivity check found the rule
  stopped paying at about 0.4 percent per transaction, four times the fee charged in its main test,
  which is the kind of headroom a reader should want before believing a held-back result.
- Held-back testing does not fix a selection made earlier. If the ten best settings were chosen on
  the fitting stretch, the held-back result belongs to the selection procedure, not to any one
  setting, and 58 percent of the return, on the survey evidence, is what selection costs after the
  result becomes known.

## Try it yourself

You need a calendar and no code.

1. Write down a ten-year period, choosing any start year, and mark a fitting stretch of seven years
   and a held-back stretch of three.
2. For each of these rules, write the smallest gap you would need: a rule that compares today's
   price with yesterday's; a rule that averages 200 daily prices; a rule that reads a twelve-month
   moving average of prices.
3. Now move the held-back stretch one month earlier at a time and count, for each position, how many
   of its days fall inside the fitting stretch. Stop when the count reaches zero.
4. Write the leak fraction for each position, as the count divided by the number of held-back days.
5. Finally, pick five instruments you would hold back and write down one sentence on how they differ
   from the five you would fit on.

What to notice: the gap needed is set by the slowest part of the rule, not by how often it trades,
and the leak fraction is largest exactly where a reader would least suspect it, in the first days of
the held-back stretch.

## Where this came from

- Mroziewicz and Slepaczuk, [A novel approach to trading strategy parameter
  optimization](https://arxiv.org/abs/2602.10785) (2026), the 81 window-length combinations, the
  19-month optimisation period, the held-back 21-month period executed once, and the break-even fee;
  identifier `2602.10785v1`.
- Deep, Deep and Lamptey, [Interpretable Hypothesis-Driven
  Trading](https://arxiv.org/abs/2512.12924) (2025), the 34 held-back periods, the 0.55 percent
  annualised return, the p-value of 0.34, the regime split, and the review numbers of McLean and
  Pontiff and of Hou and co-authors; identifier `2512.12924v1`.
- [The held-back notebook](https://github.com/coding-kitties/investing-algorithm-framework/blob/main/examples/tutorial/notebooks/05_out_sample_vector_backtest.ipynb)
  in the investing-algorithm-framework repository, the design table naming a time sample strictly
  before the search range and a universe sample of symbols the search never saw.
- [The rolling-window notebook](https://github.com/coding-kitties/investing-algorithm-framework/blob/main/examples/tutorial/notebooks/03_in_sample_param_sweep.ipynb)
  in the same repository, the 365-day training window, the 180-day test window and the 30-day gap
  between them.
- [splits.py](../../../python/nautilus_trader/optimization/splits.py), this repository's split
  contract, purge and embargo rules.

## Words used in this tutorial

- fitting stretch: the dates used to choose the settings, also called in-sample.
- gap: the stretch of time inserted between the fitting dates and the held-back dates so that the
  rule's look-back cannot reach across.
- held-back stretch: dates or instruments kept away from the choosing, also called out-of-sample.
- label: the future value a training example is supposed to predict; if its window reaches into the
  held-back dates, the answer has been given away.
- look-back: the longest stretch of past prices any part of the rule reads.
- regime: a period in which the market behaves in a recognisably similar way, such as a stretch of
  falling volatility.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
