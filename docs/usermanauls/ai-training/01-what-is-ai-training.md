# 01 - What is AI training and tuning here?

## The plain-language idea

A trading strategy has settings. A crossover strategy, for example, buys when a fast average of the
price rises above a slow average, and the two averages have lengths in bars. Those lengths are
**parameters**: numbers you choose before the strategy runs. **Training** is running the strategy
over a past period again and again, once per setting, and scoring each run. **Tuning** is keeping
the settings that scored best.

The engine supports this as a first-class research capability. It is not a neural network by
default. The default is a search over a discrete set of parameter values, scored with the same
metrics a single backtest reports. A search can enumerate every combination, sample a seeded random
subset, or breed generations from recorded results. Reinforcement learning is a further option: you
can treat a successive block of the data as a reward and let an external agent choose settings or
actions, but nothing in this manual requires it.

Where does profit come from? Only from a real, repeatable relationship between the settings (or the
signals they produce) and future prices. The search does not create that relationship. It only finds
the setting that happened to fit the past best, and that setting may fit noise. Everything below is
about telling the two apart.

## Vocabulary

| Term                  | One-sentence meaning                                                               |
| --------------------- | ---------------------------------------------------------------------------------- |
| Parameter             | A strategy setting you choose before the run, such as a moving-average length.     |
| Parameter space       | The base settings plus the named parameters and their allowed values.              |
| Experiment            | One complete set of parameter values, one point of the space.                      |
| Sweep                 | Running an experiment for every point a search strategy selects.                   |
| Objective             | The rule that turns a run's metrics into one score to maximise.                    |
| Constraint            | A condition a run must satisfy to count as feasible, checked before the objective. |
| Search strategy       | The rule that decides which experiments to evaluate.                               |
| Seed                  | The integer that makes a random or evolutionary search repeatable.                 |
| Digest                | A short fingerprint of data; equal inputs give equal digests.                      |
| Validation scheme     | A declaration of which windows are searched and which are held out.                |
| In-sample             | Data the search is allowed to score.                                               |
| Out-of-sample         | Data the search is not allowed to score during selection.                          |
| Walk-forward          | Repeated in-sample/out-of-sample windows in time order.                            |
| Leakage               | Information from the evaluation window reaching the training window.               |
| Label                 | A future outcome used as a target; it looks forward and must not leak.             |
| Overfitting           | Fitting the past so closely that the fit does not carry to the future.             |
| Trial                 | One evaluated experiment, counted so a correction can account for it.              |
| Trial provenance      | The count and status of the trials a reported result came from.                    |
| Deflated Sharpe ratio | A Sharpe ratio corrected for how many trials were searched.                        |
| Cache                 | A digest-keyed store of evaluations, so a resumed sweep skips finished work.       |

## Three worked examples by hand

### Example 1: the size of a grid

Suppose you sweep four parameters with five choices each. The number of experiments is the product
of the sizes:

```text
5 * 5 * 5 * 5 = 625 experiments
```

If one run takes 2 seconds, the sweep takes:

```text
625 * 2 = 1250 seconds = 20 minutes 50 seconds
```

Every extra parameter multiplies the cost. A fifth parameter with five choices takes the same sweep
to:

```text
625 * 5 = 3125 experiments = 6250 seconds = 1 hour 44 minutes
```

So a search is cheap to describe and expensive to run. Prefer few parameters, and narrow the choices
to those you can defend before you see the results.

### Example 2: the best of many looks good even with no skill

This is the central arithmetic of the whole manual. Imagine 100 parameter sets, none of which has
any real edge. Each one's per-period Sharpe ratio is therefore just noise: a random draw from a
distribution centred on zero. Say the standard deviation of those draws is 0.10, a plausible spread
for a daily Sharpe estimate over a short sample.

The **best** of 100 draws is not centred on zero. The expected maximum of 100 independent standard
normal draws is 2.5306 (this is the quantity the deflated Sharpe ratio uses; see
`python/nautilus_trader/optimization/significance.py`). Scaled by the spread:

```text
expected best per-period Sharpe = 2.5306 * 0.10 = 0.2531
```

Annualise by the square root of 252 trading days:

```text
0.2531 * sqrt(252) = 0.2531 * 15.8745 = 4.02
```

None of the 100 parameter sets has any edge, and the winner still shows an annualised Sharpe ratio
near 4. That number is the expected maximum of a search, not an estimate of skill. Running more
trials raises it:

```text
N = 10  -> 1.5746 * 0.10 * 15.8745 = 2.50
N = 100 -> 2.5306 * 0.10 * 15.8745 = 4.02
N = 500 -> 3.0525 * 0.10 * 15.8745 = 4.85
```

This is why lecture 06 exists: a reported best is a selection, and a selection needs a correction
before it is read as an estimate.

### Example 3: a cache turns a re-run into almost nothing

Suppose your sweep has 500 experiments and one parameter changes, so 100 of the new experiments are
new and 400 are repeats. The engine keys each evaluation by the experiment's parameter digest and
consults its cache before running anything (`python/nautilus_trader/optimization/optimizer.py`,
`python/nautilus_trader/optimization/persistence.py`). So a resumed sweep executes only the new
work:

```text
executions = 100 new experiments
reused     = 400 cached experiments
```

If those 400 were already run in an earlier process, the second run does 100 backtests instead of
500. The report records both numbers, so you can see the reuse rather than guess it.
