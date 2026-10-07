# Telling which kind of market you are in

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                   |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Nothing. It measures the kind of market the data shows, so that a later decision rests on a number rather than an opinion                                                                               |
| How often it trades       | Never; it is repeated whenever a new set of data arrives                                                                                                                                                |
| What you need             | A spreadsheet and monthly or daily sector prices                                                                                                                                                        |
| Where the rules come from | [Sector Regime Engine user manual](../../../implementation/sector-regime-engine/USER_MANUAL.md), the evidence hierarchy panel                                                                           |
| The underlying research   | Molchanov and Stangl, [The Myth of Sector Rotation](https://acfr.aut.ac.nz/__data/assets/pdf_file/0005/294287/The-Myth-of-Sector-Rotation-non-blind.pdf), for how such tests behave on real sector data |
| How well it held up       | Mixed: the hierarchy is a sound methodological choice, but the strongest direct test on 2010 to 2026 sector data found no persistence, while older academic work found some in a different era          |
| Also appears in           | [Whether sector rules are allowed](../regime-verdict/README.md) and [Contrarian reversion](../contrarian-reversion/README.md) in this collection                                                        |

## The idea in one paragraph

Before you decide whether to follow the strongest group of shares or bet against it, you have to know
which kind of market you are in. There are three possibilities that matter: trends continue, trends
reverse, or neither. You tell them apart by measuring history, not by watching the news. Not every
measurement deserves the same trust, so the method sorts its tests into a hierarchy: some are strong
enough to decide, and some are shown only for information. The result is a label plus the evidence for
it, and the label decides which trading rules are even allowed.

## Why anyone believed it

The belief rests on a simple observation: a rule that follows leaders and a rule that fades them cannot
both be right at the same time. One of them must be losing money, and if you can tell which, you can
avoid it. That is a real prize, because most of the cost of a trading rule is not being wrong on
average; it is paying the gap between the buying and selling price again and again for no reason.

In a market where trends continue, the person selling you the strong group is often a fund that has to
sell for reasons of its own, so the price can keep drifting up and you can be paid for taking the other
side. In a market where trends reverse, that seller is an investor who has been frightened out of a
position that was already cheap, so the price tends to bounce back. Both stories need a counterparty
with a reason. In a market where neither happens, the person on the other side is just someone who
disagrees, and neither of you has an edge.

The idea of measuring this was also believed because markets are not stable. The kind of market can
change: a calm decade can be followed by a wild one. A measurement that is repeated as new data arrives
at least notices the change, whereas a fixed rule keeps running after its reason has gone.

## An everyday comparison

A sailor does not decide how to set the sails from the date on the calendar. Sails are set from the
wind being measured right now: its strength, its direction, and whether it is steady or gusting. The
same boat, the same sails and the same crew behave completely differently in a calm and in a gale. The
sailor's job is not to forecast the wind perfectly but to know which of the two is happening, because
the right setting for one is the wrong setting for the other.

This tutorial is the measuring part: the instruments and the reading. What you do with the reading
comes later.

## The rules, step by step

1. Gather total-return series for the sectors you actually trade. Total return means the price plus any
   dividends paid out, so the series measures what a holder really earned.
2. Decide every rule in advance: which tests, which horizon, which significance level, which
   thresholds. Write them down before looking at the results. This is called freezing the rules, and it
   is what stops the measurement from being tuned to flatter the past.
3. Run the tests listed in section 8 of the project's research, one after another, on the same data.
4. Put each test into one of three tiers. Primary tests are allowed to decide the answer. Secondary
   tests describe the market but are not allowed a vote. A diagnostic test is displayed for interest
   only.
5. Read only the primary tier to form the verdict. If a primary test is both significant and large, it
   points one way; if the primary tests disagree, or an effect is large but not significant, the answer
   is UNCERTAIN.
6. Report the verdict with its reasons and the number of observations it used, so that anyone can see
   exactly which measurements produced it.
7. Repeat the whole thing when new data arrives, using the same frozen rules.

## The maths, with every symbol named

The most directly relevant test, because a ranking rule needs rankings to predict rankings:

```text
IC_t = Corr(Rank_t, Rank_t+1)
```

- `IC_t` is the information coefficient for the step from period `t` to `t+1`: the correlation between
  one period's ranking of the sectors and the next period's ranking.
- `Rank_t` is the list of sectors sorted by how well they did in period `t`.
- A correlation near 0 means the ranking tells you nothing about the next one. A positive value means
  leaders tend to stay leaders; a negative value means they tend to reverse.

The comparison that asks whether an up move tends to follow an up move:

```text
P(R_t+1 > 0 | R_t > 0)
```

- `P(...)` means "the probability of", the fraction of times a thing happens.
- `R_t` is the return of a sector in period `t`, and `R_t+1` the next period's return.
- The vertical bar means "given that". So the formula reads: the chance of a positive period, given a
  positive period just before.

The variance ratio compares the spread of returns over several periods with the spread over one:

```text
VR(q) = variance_of_q_period_returns / (q * variance_of_one_period_returns)
```

- `VR(q)` is the variance ratio over `q` periods, `q = 5` by default.
- `variance` is the average squared distance of the returns from their own average, a measure of how
  spread out they are.
- A value above 1 says moves tend to extend; below 1 says they tend to reverse; 1 says neither.

Finally, the size of the rebalancing opening, which is measured on the other axis entirely:

```text
rebalancing_opening ~= sigma^2 * (1 - rho)
```

- `sigma` is the cross-sectional volatility: how far apart the sectors' returns are in a period.
- `rho` is the average pairwise correlation: how closely the sectors move together, between -1 and 1.
- The opening is large when sectors differ a lot and move apart, and small when they differ little or
  move together.

## A worked example

Three sectors, ranked every month for six months. Rank 1 is the best performer that month. The returns
themselves are not shown, only the order, because the rank test uses only the order.

| Month | Energy | Health care | Technology |
| ----- | ------ | ----------- | ---------- |
| 1     | 1      | 2           | 3          |
| 2     | 1      | 3           | 2          |
| 3     | 2      | 1           | 3          |
| 4     | 3      | 1           | 2          |
| 5     | 3      | 2           | 1          |
| 6     | 2      | 3           | 1          |

For each adjacent pair of months, take the two rank columns and compute their correlation. With three
ranks, the correlation can be found from the squared differences of the ranks:

```text
IC = 1 - (6 * sum_of_squared_rank_differences) / (3 * (9 - 1))
```

| Step   | Rank differences (Energy, Health care, Technology) | Sum of squares | IC    |
| ------ | -------------------------------------------------- | -------------- | ----- |
| 1 to 2 | 0, -1, +1                                          | 2              | +0.50 |
| 2 to 3 | -1, +2, -1                                         | 6              | -0.50 |
| 3 to 4 | -1, 0, +1                                          | 2              | +0.50 |
| 4 to 5 | 0, -1, +1                                          | 2              | +0.50 |
| 5 to 6 | +1, -1, 0                                          | 2              | +0.50 |

The average of the five values is `(0.50 - 0.50 + 0.50 + 0.50 + 0.50) / 5 = +0.30`. On its own that
looks like a real tendency for rankings to persist. Now the second required step, testing whether a
value of +0.30 could easily be luck. The five values are spread widely, from -0.50 to +0.50, so:

```text
average = 0.30
spread (standard deviation) = 0.4472
standard error = 0.4472 / sqrt(5) = 0.20
t-statistic = 0.30 / 0.20 = 1.50
```

- The `standard deviation` measures how far the five values sit from their average.
- The `standard error` is the spread of the average itself, smaller when more values are used.
- The `t-statistic` is how many standard errors the average sits away from zero.
- For a statistic of 1.50, the p-value is about 0.13: a 13 percent chance of a result this extreme by
  luck, which is well above the 0.0167 threshold.

So the effect is outside the deadband of 0.05, because 0.30 is larger, but it is not significant. The
honest verdict is UNCERTAIN. Five observations is far too few to trust, and the average is being pulled
around by one negative month out of five.

## What the research actually found

The most thorough direct test in the project's research, on American sectors from 1948 to 2018, ran a
regression of every sector's excess return on every other sector's at lags of one to twenty-four months.
Across 2,160 test statistics, 6 percent were significantly positive at the 10 percent level, against the
5 percent expected by chance alone, mostly at the shortest lag. The authors' reading was that
cross-sector predictability occurs only randomly.

A later pre-registered test on 2010 to 2026 data froze four popular rotation states before reading the
data and checked whether the next 10, 20 or 60 trading days differed from ordinary days. None of 24
cells was confirmed, across 4,482 sector events. The single effect visible in the first decade, a deeper
pullback after a crowded flag, shrank to nothing in the data held back for the honest test. The authors
pointed out that academic sector momentum uses six- to twelve-month windows and skips the most recent
month, which is exactly the reverse of the short horizon the test examined.

One diagnostic, the Hurst exponent, was reported by the search layer at 0.627 to 0.671 for sector funds,
which is described in the source as suggestive at most. Its estimates move around with the estimator,
the sample length and the frequency, so a value above 0.5 does not by itself establish a tradable edge.
That is why it sits in the lowest tier.

## How this project relates to it

The tests in this tutorial are the ones the app computes. In
[src/engine.js](../../../implementation/sector-regime-engine/src/engine.js), the function
`rankAutocorrelation` builds exactly the average information coefficient worked out above and attaches
a t-test to it; `varianceRatio` implements the Lo-MacKinlay version of the test; and `buildTests`
assigns each measurement its tier. Only the rows marked primary can reach `classifyRegime`.

The [user manual](../../../implementation/sector-regime-engine/USER_MANUAL.md) describes the same
hierarchy in panel 6, where each measurement is shown with a coloured tag of primary, secondary or
diagnostic, and explains in section 8 why the tiers exist. Section 8 of
[Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md) is the original
statement of the hierarchy and of each test.

## Where it goes wrong

- The ranking is a judgement. Which tests are primary, which horizon, which threshold: all are choices,
  and a different honest analyst could choose differently. The defence is to freeze them in advance and
  report them, not to pretend they are the only possible set.
- The Hurst exponent is fragile. Estimator, sample length, structural breaks and frequency all move it,
  which is why the source places it in the lowest tier.
- Non-stationarity. A market can change its behaviour mid-sample. An average over twenty years can
  describe neither the first ten nor the last ten.
- A short sample says almost anything. As the worked example shows, five observations can produce a
  large average with a huge p-value; the answer must be UNCERTAIN.
- Look-ahead. The tests must use only information available at the time of each decision. Using a
  ranking that quietly includes future data is how a dead market looks alive.
- The chosen universe. Testing on sectors that still exist today, in the form they take today, is a
  mild form of choosing the sample after seeing the answer.
- A large effect is not a profitable one. As in the sister tutorial, a result can be unmistakable and
  still too small to cover the cost of trading on it.

## Try it yourself

You need a spreadsheet and the monthly returns of three sectors for a year, all from any public source.

1. Make one row per month and one column per sector, holding each month's return.
2. Add three more columns holding the rank of each sector that month, 1 for the best return and 3 for
   the worst.
3. Add a column for each adjacent pair of months holding the squared differences of the ranks, and a
   summary column holding `1 - 6 * (sum of those squares) / 24`.
4. Average the twelve monthly IC values. That average is your measured rank persistence.
5. Average the squared distance of each IC from that average, divide by eleven, and take the square
   root. That is the spread. Divide it by the square root of twelve. That is the standard error.
6. Divide the average by the standard error. That is your t-statistic; compare it with 2, roughly the
   one-in-twenty line for a two-sided test of this kind.

What to notice: the monthly IC values jump around wildly, and the average over twelve months is small
and unstable. If your average looks large, check whether one or two months are doing all the work. That
instability, not the headline average, is the honest finding.

## Where this came from

- [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md), section 8: the
  evidence hierarchy, the eight tests, and the reasoning for each tier.
- [Sector Regime Engine user manual](../../../implementation/sector-regime-engine/USER_MANUAL.md),
  panel 6 (metrics and the evidence hierarchy) and section 8 (worked examples).
- [src/engine.js](../../../implementation/sector-regime-engine/src/engine.js), `rankAutocorrelation`,
  `varianceRatio`, `conditionalPersistence`, `buildTests` and `classifyRegime`.
- Molchanov and Stangl, [The Myth of Sector Rotation](https://acfr.aut.ac.nz/__data/assets/pdf_file/0005/294287/The-Myth-of-Sector-Rotation-non-blind.pdf),
  the 1948 to 2018 sector sample and the cross-sector regression test.
- Quant Data, [Does sector momentum persist?](https://quantdata.uk/research/does-sector-momentum-persist),
  the 2010 to 2026 pre-registered test and the held-out window.

## Words used in this tutorial

- correlation: a number between -1 and 1 describing whether two things move together; near 1 means they
  move almost identically, near 0 means they are unrelated.
- dispersion: how spread out the sector results are from each other in a given period.
- evidence hierarchy: a ranking of measurements by how much weight they are allowed to carry.
- information coefficient: the correlation between one period's ranking of sectors and the next.
- p-value: the chance of seeing a result this extreme purely by luck.
- rank: a sector's position in an ordered list, 1 for the best.
- rank turnover: the average change in rank between decision dates; high turnover is the signature of a
  world in which ranking rules fail.
- standard error: the spread of an average, used to judge how far it sits from zero.
- variance ratio: the spread of returns over several periods divided by the spread of one-period
  returns; above 1 means moves extend, below 1 means they reverse.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
