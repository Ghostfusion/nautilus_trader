# 08 - Exercises

Six exercises and one deliberate breakage. Each solution is the changed lines and the real output
they produce. All of them read the committed panel from [04](04-sample-data.md) and run from the
repository root with the environment from [03](03-first-run.md).

Keep a copy of the lecture 03 program and the lecture 05 programs near you; most exercises are a
one- or two-line change to one of them.

## Exercise 1: change the lookback

Change the factor window from five days to ten. Which instruments change rank, and which do not?

**Solution.** Change `LOOKBACK = 5` to `LOOKBACK = 10` in the lecture 03 program. The factor line
already reads `closes[-1 - lookback]`, so no other change is needed.

```text
exercise 1: 10-day momentum at day 35
  DDD.SIM  +0.095238
  AAA.SIM  +0.036364
  BBB.SIM  +0.009756
  CCC.SIM  -0.041379
```

The order is unchanged -- `DDD.SIM`, `AAA.SIM`, `BBB.SIM`, `CCC.SIM` -- but every magnitude is
larger, because a longer window accumulates more price change. This is why the lookback is a
*parameter*: it changes the factor's values without changing the factor's idea. It is also why you
must declare how many lookbacks you tried.

## Exercise 2: long only the best one

Instead of equal-weighting the top two, hold only the best instrument.

**Solution.** Replace the last two lines of the lecture 03 program:

```python
top = ranked[:1]
weight = 1.0 / len(top)
print("\nsingle-name long:", {iid: round(weight, 4) for iid in top})
```

```text
exercise 2: long the top one only
  weight: {'DDD.SIM': 1.0}
```

The book is now 100% in one instrument. That is a *concentration* risk: one bad day in `DDD.SIM` is
one bad day for the whole portfolio. Spreading across the top `k` is what `k` buys you, and `k` is a
choice, not a detail.

## Exercise 3: state the book's gross and net

For the rank-weighted book, report gross exposure (the sum of absolute weights) and net exposure
(the sum of weights).

**Solution.** In the lecture 05 step 3 program, the last line already does this:

```python
print("gross:", sum(abs(w) for w in weights.values()), "net:", sum(weights.values()))
```

```text
gross: 1.0 net: 0.0
```

Gross 1.0 means the book trades one equity's worth in each direction. Net 0.0 means it is
dollar-neutral. If your net were not zero, the book would have market exposure, and a rising market
would make an unskilled factor look good.

## Exercise 4: see the label's absence

Print the label for the last few rows and show what happens at the end of the data.

**Solution.** Extend the lecture 05 step 5 program:

```python
for row in (30, 31, 35):
    ts = closes["AAA.SIM"][row][0]
    print(f"row {row}: label={series.value_at(ts)}")
```

```text
row 30: label=0.017857142857142794
row 31: label=None
row 35: label=None
```

Row 30 is the last row whose five-day window is full. Rows 31 and 35 have fewer than five future
observations, so the label is `None`. It is **not** zero. If your code filled `None` with zero, those
rows would enter your statistics as "no move", which is false; they had no measurable outcome at all.

## Exercise 5: raise the purge

Raise `purge_before` from one day to two days. Does the training set move?

**Solution.** Change `purge_before=1 * DAY_NS` to `purge_before=2 * DAY_NS` in the lecture 05 step 6
program and print the training bounds twice: once with the label horizon enforced, once without.

```python
def train_width(purge_days, enforce):
    label_rule = LabelOverlapRule.ENFORCE if enforce else LabelOverlapRule.NONE
    horizon = 5 * DAY_NS if enforce else None
    contract = SplitContract(
        sets=("train", "validation", "test"),
        lengths={"train": 6 * DAY_NS, "validation": 2 * DAY_NS, "test": None},
        evaluation="validation",
        window=10 * DAY_NS,
        leakage=LeakagePolicy(
            purge_before=purge_days * DAY_NS,
            purge_after=1 * DAY_NS,
            embargo_after=1 * DAY_NS,
            label_overlap_rule=label_rule,
            label_horizon=horizon,
        ),
        n_splits=1,
    )
    return contract.split(START_NS, START_NS + 36 * DAY_NS)[0].bounds["train"]
```

```text
exercise 5: purge_before raised to 2 days
  enforce=True purge_before=1d -> train day [11, 12)
  enforce=True purge_before=2d -> train day [11, 12)
  enforce=False purge_before=1d -> train day [11, 16)
  enforce=False purge_before=2d -> train day [11, 15)
```

With `enforce=True`, the purge is `max(purge_before, label_horizon)`, so both values give the same
five-day purge and the training set does not move. With `enforce=False`, the label horizon is not
applied at all, and the training set ends one day earlier when the purge goes from one to two days.
The lesson: the horizon, not the purge you typed, is what usually sets the exclusion when labels
reach forward.

## Exercise 6: shift the alignment

Compute the same label with `AlignmentConvention.NEXT_BAR` instead of `SIGNAL_BAR`. What changes?

**Solution.** Add `alignment=AlignmentConvention.NEXT_BAR` to the `LabelDefinition` and compare.

```text
exercise 6: NEXT_BAR alignment shifts the label
  row 0: signal_bar=0.020000 next_bar=0.01962708537782132
  row 1: signal_bar=0.019627 next_bar=0.01984126984126977
  row 2: signal_bar=0.019841 next_bar=0.02006018054162495
```

The values are the same array shifted by one row: `next_bar` at row 0 equals `signal_bar` at row 1.
This is not cosmetic. `next_bar` measures its outcome over a later bar and reads one bar further into
the future, so it reaches further past its feature row, and the purge a split must apply is larger.
That is why the alignment convention is part of the definition's digest.

## Break it on purpose

Delete the purge justification and confirm the engine refuses. Change the leakage policy to leave an
interval unset with no reason:

```python
from nautilus_trader.optimization import LeakagePolicy

try:
    LeakagePolicy(purge_before=None, purge_after=1 * DAY_NS, embargo_after=1 * DAY_NS)
except ValueError as error:
    print("refused:", error)
```

```text
break it: an unjustified zero interval
  refused: zero intervals ['purge_before'] require a zero_interval_justification
```

A blank field and a decided zero are different values with different digests. The engine will not let
you leave the most dangerous field in the study ambiguous. **Do not "fix" this by deleting the
purge.** The refusal is the feature: it forces you to decide, in writing, why the exclusion is what
it is.

## A note on what these exercises did not test

None of these exercises computed a deflated Sharpe ratio with the right trial count, and none used
point-in-time membership for the whole backtest. Those are the two things that decide whether a
factor result is believable, and they are the subject of [05](05-build-the-strategy.md) and
[06](06-measure-and-evaluate.md). Doing them once by hand is the point of this manual.

Next: [09 - Go further](09-go-further.md).
