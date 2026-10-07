# 06 - Measure and evaluate

A factor that ranks does not yet have an edge. This lecture is about the two questions that turn a
ranking into a claim: "was the training honest?" and "was the result more than luck?". The first is
the split contract; the second is the deflated Sharpe ratio. Both are Python, in
`python/nautilus_trader/optimization/`, and both are demonstrated with output below.

## The split contract, in plain words

A **split** cuts a period into named sets. The names in this manual are `train`, `validation` and
`test`:

- **Train**: the data you search on. You pick your factor, its window, and its parameters here.
- **Validation**: a later, untouched slice you check the selected model on while you are still
  allowed to change your mind.
- **Test**: the slice you touch once, at the end, to see whether the thing generalises.

The order matters. A split contract is the object that owns this layout, so that every study using
it lays the period out the same way. `SplitContract` is in
`python/nautilus_trader/optimization/splits.py`, and the concept page is
[Optimization](../../concepts/optimization.md).

A split contract declares:

- the **sets** and their **lengths**, in nanoseconds or as a fraction of a window;
- which set is the **evaluation set**, whose information the others may not overlap (default: the
  last set);
- the **leakage policy**, the exclusion relation;
- optional layout: `window`, `stride`, `min_length`, `n_splits`, and `direction`.

`n_splits` selects that many evenly spaced windows rather than the first ones. `direction`
(`SplitDirection.EXACT`, `FORWARD`, or `REVERSED`) decides where a leftover span at the end of the
period goes.

### Purge, embargo, and the horizon

**Leakage** is not a distance in time; it is an exclusion relation. A `LeakagePolicy` states it, and
its fields are:

| Field                         | What it removes                                                    |
| ----------------------------- | ------------------------------------------------------------------ |
| `purge_before`                | The training tail adjacent to the evaluation set.                  |
| `purge_after`                 | Moves a training set that follows the evaluation set away from it. |
| `embargo_after`               | The minimum gap between one split and the next.                    |
| `label_overlap_rule`          | Whether a declared label horizon is folded into the purge.         |
| `label_horizon`               | The label horizon in nanoseconds, required by `ENFORCE`.           |
| `zero_interval_justification` | Why any interval is zero. Required whenever one is.                |

"Purge" means *throw away*. If your training set ends right where your validation set begins, the
last training rows know what happened in the first validation rows. Purging the tail removes them.

"Embargo" is the same idea between splits: after one split's evaluation, wait before the next split
starts, so a label that reaches forward cannot cross the seam.

The horizon is why leakage is not a distance. A label *looks forward*. If your label measures the
five-day forward return, then a training row that is one day before the validation start still has
four days of its outcome inside the validation set. No amount of "leave a gap" fixes that unless the
gap is at least the horizon. `label_overlap_rule = ENFORCE` says exactly that: fold a declared
`label_horizon` into `purge_before`, so an overlapping observation is excluded outright.
`LeakagePolicy.purge` is then `max(purge_before, label_horizon)`.

The engine refuses to guess. An interval left unset (`None`) and an interval decided to be zero
(`0`) are different values with different digests, and any zero interval requires a written
`zero_interval_justification`. A study that genuinely has no leakage must say so, in words, rather
than leaving a field blank.

### Proving the exclusion covers the label

Declaring a horizon is not the same as covering it. `LabelSeries.forward_reach_ns` measures how far
past its feature row a label actually reaches, and `leakage_capability` answers whether a policy
covers that reach *before* the work is done. Try it with a two-day purge against the five-day label
from lecture 05:

```python
from pathlib import Path

from nautilus_trader.optimization import (
    LabelDefinition,
    LabelKind,
    LabelOverlapRule,
    LeakagePolicy,
    label_series,
    leakage_capability,
)

PANEL = Path("docs/usermanauls/factor-portfolio/sample_data/price_panel.csv")
DAY_NS = 86_400_000_000_000

closes = {}
for line in PANEL.read_text().splitlines()[1:]:
    ts_ns, instrument_id, close = line.split(",")
    closes.setdefault(instrument_id, []).append((int(ts_ns), float(close)))

definition = LabelDefinition(label_id="fwd5", kind=LabelKind.FORWARD_RETURN, horizon=5)
series = label_series(definition, closes["AAA.SIM"])

too_short = LeakagePolicy(purge_before=2 * DAY_NS, purge_after=1 * DAY_NS, embargo_after=1 * DAY_NS)
capability = leakage_capability(series, too_short)
print("code:", capability.code)
print("detail:", capability.detail)
print("requirements:", capability.requirements)

covering = LeakagePolicy(
    purge_before=2 * DAY_NS,
    purge_after=1 * DAY_NS,
    embargo_after=1 * DAY_NS,
    label_overlap_rule=LabelOverlapRule.ENFORCE,
    label_horizon=5 * DAY_NS,
)
print("covered available:", leakage_capability(series, covering).available())
```

```text
code: LEAKAGE_NOT_COVERED
detail: the label 'fwd5' reaches 432000000000000 ns beyond its feature row while the leakage policy purges 172800000000000 ns
requirements: ['cover 259200000000000 ns more before the evaluation set']
covered available: available
```

The code is the canonical part, and it is a refusal: `LEAKAGE_NOT_COVERED` means the request cannot
be served as stated. The requirements line turns that into a number you can act on: cover
`259200000000000` ns (three days) more before the evaluation set. Enforcing the five-day horizon
instead of a two-day purge makes the same probe answer `available`.

This is the shape of every capability in `python/nautilus_trader/optimization/capability.py`: it
answers as data rather than an exception, so a caller can branch on the code and act on the
requirements. The code is canonical; the detail is for a human.

## The deflated Sharpe ratio, in plain words

Every factor study is a search. You try a five-day momentum factor, then a ten-day one, then a
twenty-day one, and you report the best. The best of many trials is a **maximum**, and the maximum
of many noisy numbers is positive even when none of them has any skill. If you flip a hundred fair
coins and report the coin with the most heads, you will report a coin with more than fifty heads.

The **deflated Sharpe ratio** corrects for this. It answers one question:

> Given how many things I tried, and how spread out those tries were, what is the probability that
> the best Sharpe ratio I found exceeds what pure luck would have produced?

It is a probability between 0 and 1. A high value (close to 1) means the winner clears the luck bar;
a low value means it does not. The formula, from `python/nautilus_trader/optimization/significance.py`, is:

```text
SR0 = sqrt(V[SR]) * ((1 - gamma) * Phi^-1(1 - 1/N) + gamma * Phi^-1(1 - 1/(N * e)))
DSR = Phi((SR - SR0) * sqrt(T - 1) / sqrt(1 - skew * SR + ((kurtosis - 1) / 4) * SR^2))
```

Read the pieces:

- `SR` is the selected trial's Sharpe ratio, expressed **per period** (per day here), not per year.
- `V[SR]` is the **cross-trial variance** of the Sharpe estimates: how spread out your tries were.
- `N` is the **trial count**.
- `T` is the number of contributing periods.
- `skew` and `kurtosis` describe the shape of the selected strategy's returns.
- `gamma` is the Euler-Mascheroni constant, and `Phi` is the standard normal distribution.

`SR0` is the **expected maximum** of `N` independent Sharpe estimates that have no skill. It grows
as `N` grows, because the expected best of more random draws is better. `DSR` is the probability
that your `SR` beats that hurdle.

### Why more trials mean a higher bar

The whole correction lives in `SR0 = sqrt(V[SR]) * (... N ...)`. As `N` grows, the two normal
quantiles grow, so `SR0` grows, so `SR - SR0` shrinks, so `DSR` falls. That is not a flaw of the
formula; it is the formula's purpose. A bigger search needs a bigger winner before you should
believe it.

Here is a real factor study built on the committed panel: a simple backtest that each day holds the
top two five-day-momentum instruments next to one another and measures the portfolio return, run for
twenty different lookbacks (2 through 21 days). The selected trial is the best of them.

```python
from pathlib import Path

from nautilus_trader.optimization import (
    SharpeSample,
    TrialDependence,
    deflated_sharpe_ratio,
    per_period_sharpe,
    return_moments,
)

PANEL = Path("docs/usermanauls/factor-portfolio/sample_data/price_panel.csv")

closes, stamps = {}, {}
for line in PANEL.read_text().splitlines()[1:]:
    ts_ns, instrument_id, close = line.split(",")
    closes.setdefault(instrument_id, []).append(float(close))
    stamps.setdefault(instrument_id, []).append(int(ts_ns))

IDS = sorted(closes)
DAYS = len(closes[IDS[0]])


def strategy_returns(lookback):
    """Hold the top two momentum names, equal weight, from each day to the next."""
    out = {}
    for day in range(lookback, DAYS - 1):
        factor = {iid: closes[iid][day] / closes[iid][day - lookback] - 1.0 for iid in IDS}
        top = sorted(factor, key=factor.get, reverse=True)[:2]
        forward = sum(closes[iid][day + 1] / closes[iid][day] - 1.0 for iid in top) / 2
        out[stamps[IDS[0]][day + 1]] = forward
    return out


trials = {lookback: strategy_returns(lookback) for lookback in range(2, 22)}
sharpes = {lookback: per_period_sharpe(returns) for lookback, returns in trials.items()}
trial_values = [estimate.value for estimate in sharpes.values()]
print("trial count:", len(trial_values))
print("trial Sharpe min/median/max:",
      f"{min(trial_values):.4f}",
      f"{sorted(trial_values)[len(trial_values) // 2]:.4f}",
      f"{max(trial_values):.4f}")


def sample(n):
    lookbacks = list(trials)[:n]
    selected = max(lookbacks, key=lambda k: sharpes[k].value)
    chosen = sharpes[selected]
    chosen_moments = return_moments(trials[selected])
    return SharpeSample(
        sharpe=chosen.value,
        trial_sharpes=[sharpes[k].value for k in lookbacks],
        observations=chosen.observations,
        skew=chosen_moments.skew,
        kurtosis=chosen_moments.kurtosis,
        dependence=TrialDependence.INDEPENDENT,
    )


for n in (12, 20):
    result = deflated_sharpe_ratio(sample(n))
    print(f"N={n:<3} counted={result.sample.counted_trials:<3} status={result.status!s} "
          f"deflated={result.value:.6f}")
```

```text
trial count: 20
trial Sharpe min/median/max: 0.0814 0.3857 0.4560
N=12  counted=12  status=MetricStatus.COMPUTED deflated=0.882066
N=20  counted=20  status=MetricStatus.COMPUTED deflated=0.864627
```

The same winner, deflated against twelve trials, scores 0.882; against twenty trials, 0.865. Nothing
about the winner changed. The bar rose, so its probability fell. If you had declared twenty trials
but reported the twelve-trial figure, you would be flattering yourself by about 0.017, and that gap
is exactly the kind of thing that compounds into a false discovery.

### What the sample refuses

The correction is not a number you can feed anything into. `SharpeSample` refuses three declarations
that would otherwise produce a plausible-looking result:

```python
from nautilus_trader.optimization import (
    SharpeFrequency,
    SharpeSample,
    TrialDependence,
)

try:
    SharpeSample(
        sharpe=0.5,
        trial_sharpes=[0.4, 0.3, 0.2, 0.1] * 3,
        observations=21,
        skew=0.0,
        kurtosis=3.0,
        dependence=TrialDependence.INDEPENDENT,
        frequency=SharpeFrequency.ANNUALISED,
    )
except ValueError as error:
    print("annualised refused:", error)
```

```text
annualised refused: the correction is per-period: an annualised input (annualised) must be rescaled by its caller, not divided here
```

- **An annualised Sharpe is refused.** The correction is per period. Dividing an annualised figure by
  an assumed number of periods per year would produce a number no reader could audit.
- **The kurtosis must be non-excess.** The fourth standardized moment is at least 1 for any
  distribution, so an excess kurtosis of `0` -- the wrong convention -- is refused rather than
  reinterpreted.
- **Dependence must be declared.** A sweep over adjacent parameters is not an independent sample,
  and the correction cannot tell from the values alone. A dependent study must supply its effective
  trial count; an independent study may not.

Below the contract's minimums (twenty contributing observations, ten trial estimates) the status is
`unavailable` with `insufficient_data`, and a non-positive variance factor is `invalid` with
`undefined_result`. A result that is not computed always carries a reason and no value.

### The value is reported, never a gate

`deflated_sharpe_ratio` is a report. Nothing consults it: not a strategy, not an order, not a risk
check. The correction is also not wired into the emitted result document, because a run record keeps
its metric values rather than its return series. You compute it, you read it, and you decide. The
temptation to turn it into a pass/fail switch is what the design deliberately avoids.

## Reports and statistics

A backtest returns statistics through `BacktestResult.stats_pnls`, `stats_returns` and
`stats_general`, plus `returns_series`. The portfolio's own `Portfolio.statistics()` computes
`WinRate`, `ProfitFactor`, `SharpeRatio` and `LongRatio` by default; register another built-in such
as `MaxDrawdown` with `Portfolio.register_statistic()`. See
[Portfolio](../../concepts/portfolio.md).

One trap: a run's default returns dictionary does not include every built-in statistic. "Max
Drawdown" is the important example, because the kernel portfolio's analyzer does not register it, so
it is absent from a run's dictionary. The Python bridge `statistic_values` closes that gap without
computing any metric itself: it feeds the completed run's own `returns_series` into a fresh
`PortfolioAnalyzer` with the requested native statistic registered, and reads the value back. For
the sample run in the concept page the engine reports `Sharpe Ratio (simple, sample, 252 days)` as
`-27.2553412003192`, and the bridged `Max Drawdown (simple)`, which the run does not report, is
`-0.014306129144533997`.

Periodic attribution uses the performance-period frame: one row per UTC day, ISO week or month,
with accounting, trading activity, exposure, and derived performance. It is a projection of the
portfolio's own numbers, never a second ledger. See
[Performance periods](../../concepts/performance_periods.md). A period with no observation has no
row: an absent authority is not rendered as a zero.

## What good and bad look like

| Number          | What it means                        | A cautious reading                                       |
| --------------- | ------------------------------------ | -------------------------------------------------------- |
| Sharpe ratio    | Return per unit of risk.             | Under about 0.5 per year, most of it is noise and costs. |
| Max drawdown    | Worst peak-to-trough fall in equity. | Your worst day is not your worst day of the past.        |
| Deflated Sharpe | Probability the winner beats luck.   | Below roughly 0.95, treat the result as unproven.        |
| Turnover        | How much you trade.                  | Each round trip costs commission twice.                  |
| Win rate        | Fraction of trades that made money.  | A high win rate with one huge loss is not good.          |
| Profit factor   | Gross profit divided by gross loss.  | Sensitive to one outlier; read with the drawdown.        |

None of these thresholds is a rule. They are a starting point for asking "how much of this is
luck?".

## The three most common beginner misreadings

1. **Reading the Sharpe ratio of a sweep's winner as if it were the strategy's Sharpe ratio.** It is
   the maximum of many estimates. The deflated Sharpe ratio exists to say so. Reporting the winner
   without the trial count behind it is a selection, not an estimate.

2. **Reading an absent metric as zero.** The engine is careful here: an undefined statistic returns
   a not-available state with a reason, never `0.0`; a genuine zero (no commissions, for example) is
   a measurement. If you fill a missing drawdown with zero, you will believe a risk that did not
   happen did not exist.

3. **Reading an annualised number as a per-period number.** `Sharpe Ratio (simple, sample, 252 days)` is annualised.
   The correction is per period and refuses annualised input. Mixing them silently divides one by
   the other and inflates or deflates the result by the square root of the compounding factor.

Next: [07 - Risks and limits](07-risks-and-limits.md).
