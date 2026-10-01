# 05 - Build the strategy

This is the longest lecture. It takes the four-line idea from lecture 03 and adds the four things
that make a factor study honest: a stated factor, point-in-time membership, a forward label, and a
split that keeps the future out of the training data. Each step is numbered, and each has the code
and the output it produced.

The whole lecture reads the committed panel from [04](04-sample-data.md), so run everything from the
repository root with the environment from [03](03-first-run.md).

## Step 1: load the panel

```python
from pathlib import Path

PANEL = Path("docs/usermanauls/factor-portfolio/sample_data/price_panel.csv")
LOOKBACK = 5

closes = {}
for line in PANEL.read_text().splitlines()[1:]:
    _, instrument_id, close = line.split(",")
    closes.setdefault(instrument_id, []).append(float(close))
```

`closes` maps each instrument identifier to a list of prices, oldest first, in the file's order. The
day index is the position in that list: day 0 is 2025-01-01, day 35 is 2025-02-05. This loader is
the one from [03](03-first-run.md), where it was run against the same file; it is repeated here so
the later steps stand alone.

## Step 2: state the factor, and compute it by hand

A factor must be stated precisely enough that two people compute the same number. Ours is:

> For each instrument at day `t`, the five-day momentum is
> `close(t) / close(t - 5) - 1`.

Work one instrument by hand, at the last day, `t = 35`.

`AAA.SIM`: day 35 close is 114.00, day 30 close is 112.00.

```text
114.00 / 112.00 = 1.017857142857...
1.017857142857... - 1 = 0.017857142857... = 1.7857 percent
```

`BBB.SIM`: day 35 close is 51.75, day 30 close is 51.50.

```text
51.75 / 51.50 = 1.004854368932...
minus one      = 0.004854368932... = 0.4854 percent
```

`CCC.SIM`: day 35 close is 69.50, day 30 close is 71.00.

```text
69.50 / 71.00 = 0.978873239437...
minus one      = -0.021126760563... = -2.1127 percent
```

`DDD.SIM`: day 35 close is 28.75, day 30 close is 27.50.

```text
28.75 / 27.50 = 1.045454545455...
minus one      = 0.045454545455... = 4.5455 percent
```

The four numbers are the cross section at day 35. Nothing in this arithmetic used a price after day
35, which is the first habit to build.

## Step 3: rank, and produce target weights

Sort the scores, best first, and turn the order into weights. Here is the program. It computes the
factor, ranks it, and builds a rank-weighted dollar-neutral book exactly as in lecture 01's second
example.

```python
from decimal import Decimal
from pathlib import Path

from nautilus_trader.model import InstrumentId
from nautilus_trader.model import SignalDirection
from nautilus_trader.model import TradingSignal
from nautilus_trader.trading import TargetPipelineConfig

PANEL = Path("docs/usermanauls/factor-portfolio/sample_data/price_panel.csv")
LOOKBACK = 5

closes = {}
for line in PANEL.read_text().splitlines()[1:]:
    _, instrument_id, close = line.split(",")
    closes.setdefault(instrument_id, []).append(float(close))

factor = {
    iid: series[-1] / series[-1 - LOOKBACK] - 1.0
    for iid, series in closes.items()
}
order = sorted(factor, key=factor.get)
n = len(order)
mid = (n + 1) / 2
raw = {iid: index + 1 - mid for index, iid in enumerate(order)}
gross = sum(abs(value) for value in raw.values())
weights = {iid: raw[iid] / gross for iid in order}

print("instrument  factor      rank  weight")
for iid in order:
    print(f"{iid:<11} {factor[iid]:+.6f}  {order.index(iid) + 1:<4}  {weights[iid]:+.4f}")
print("gross:", sum(abs(w) for w in weights.values()), "net:", sum(weights.values()))
```

```text
instrument  factor      rank  weight
CCC.SIM     -0.021127  1     -0.3750
BBB.SIM     +0.004854  2     -0.1250
AAA.SIM     +0.017857  3     +0.1250
DDD.SIM     +0.045455  4     +0.3750
gross: 1.0 net: 0.0
```

`order` is ascending by factor, so `CCC.SIM` (worst) is rank 1 and `DDD.SIM` (best) is rank 4. The
demeaning by `mid = 2.5` makes the weights sum to zero, and dividing by the gross absolute sum (4.0)
makes their absolute values sum to one. The printed line `gross: 1.0 net: 0.0` confirms both.

Read the result as an instruction: go short 0.375 of equity in `CCC.SIM`, short 0.125 in `BBB.SIM`,
go long 0.125 in `AAA.SIM`, and long 0.375 in `DDD.SIM`. Two of the four positions lost money in the
next day of lecture 01's example; the shape still works because the ranking was right on average.

## Step 4: point-in-time membership, and why today's membership cheats

The ranking above used all four instruments. That is only correct if all four *were eligible* at day
35. In a real universe instruments enter and leave, and the rule is not optional: it is the
difference between a study and a fantasy.

The membership rule for this manual is:

> An instrument is a member of the `SIM` universe at day `d` if a stored membership interval for it
> covers `d`. An interval is half-open: it covers its entry day and every later day up to, but not
> including, its exit day.

That is exactly the semantics of `MembershipInterval` in `crates/research/src/membership.rs`: "the
instrument is a member from `ts_event` (its entry instant) up to but not including `exited_at`", and
`covers(ts)` returns `ts >= self.ts_event && self.exited_at.is_none_or(|exit| ts < exit)`.

Now suppose `DDD.SIM` did not list until day 20. The stored intervals are:

| Instrument | Entered (day) | Exited (day) |
| ---------- | ------------- | ------------ |
| AAA.SIM    | 0             | never        |
| BBB.SIM    | 0             | never        |
| CCC.SIM    | 0             | never        |
| DDD.SIM    | 20            | never        |

Rank the universe at day 10 two ways: once with today's membership (all four, including `DDD.SIM`)
and once with the membership that actually applied at day 10.

```python
from pathlib import Path

PANEL = Path("docs/usermanauls/factor-portfolio/sample_data/price_panel.csv")
LOOKBACK = 5

INTERVALS = [
    ("AAA.SIM", 0, None),
    ("BBB.SIM", 0, None),
    ("CCC.SIM", 0, None),
    ("DDD.SIM", 20, None),
]


def members_at(day):
    """Resolve the universe membership at a day from the stored intervals (half-open)."""
    return {
        iid
        for iid, entered, exited in INTERVALS
        if entered <= day and (exited is None or day < exited)
    }


closes = {}
for line in PANEL.read_text().splitlines()[1:]:
    _, instrument_id, close = line.split(",")
    closes.setdefault(instrument_id, []).append(float(close))

day = 10
factor_today = {iid: closes[iid][day] / closes[iid][day - LOOKBACK] - 1.0 for iid in closes}
factor_pit = {iid: value for iid, value in factor_today.items() if iid in members_at(day)}

rank_today = sorted(factor_today, key=factor_today.get, reverse=True)
rank_pit = sorted(factor_pit, key=factor_pit.get, reverse=True)

print("membership at day 10:", sorted(members_at(day)))
print("using today's membership (all four):")
for iid in rank_today:
    print(f"  {iid:<8} {factor_today[iid]:+.6f}")
print("  long top 2:", rank_today[:2])
print("using point-in-time membership:")
for iid in rank_pit:
    print(f"  {iid:<8} {factor_pit[iid]:+.6f}")
print("  long top 2:", rank_pit[:2])
```

```text
membership at day 10: ['AAA.SIM', 'BBB.SIM', 'CCC.SIM']
using today's membership (all four):
  DDD.SIM  +0.058824
  AAA.SIM  +0.019608
  BBB.SIM  +0.004975
  CCC.SIM  -0.019108
  long top 2: ['DDD.SIM', 'AAA.SIM']
using point-in-time membership:
  AAA.SIM  +0.019608
  BBB.SIM  +0.004975
  CCC.SIM  -0.019108
  long top 2: ['AAA.SIM', 'BBB.SIM']
```

This is the concrete cheat. Using today's membership gives you the top two `DDD.SIM` and `AAA.SIM`.
Using the membership that applied at day 10 gives you `AAA.SIM` and `BBB.SIM`. The two books are
different instruments entirely, and only one of them was investable on day 10: you cannot have
bought `DDD.SIM` eleven days before it listed. The cheat does not make noise; it makes a *different
portfolio*, and it will look better, because the instrument you wrongly included is included
precisely because it later became large enough to notice.

In the Rust pipeline this is not a convention you have to remember. `Panel::new` calls
`Panel::check`, which resolves every row's `member` field from the stored `MembershipSeries` at the
row's timestamp and refuses a mismatch:

```text
membership for {instrument_id} at {ts_event} in universe `{universe}` is {claimed}, but the
stored series records {actual}
```

That message is `PanelError::MembershipMismatch` in `crates/research/src/panel.rs`. The same check
also refuses a feature that reads past its own row with `PanelError::Lookahead`, whose message names
the feature and the offending instant. A panel that cannot be built honestly cannot be built at all.

The Python surface stores the same idea as a declaration, not a guarantee: `UniverseIdentity` in
`python/nautilus_trader/optimization/identity.py` records a `membership_policy_id` and a
`membership_as_of` time, and the concept page is explicit that "nothing in this repository stores
point-in-time membership history" on the Python side. Storing the series is the Rust crate's job.

## Step 5: label the outcome

A factor ranks instruments; a **label** says whether the ranking was right. The label is a future
outcome, measured per row. The Python label layer is `python/nautilus_trader/optimization/labels.py`.

State the label precisely: for each bar, the **five-day forward return**,
`close(t + 5) / close(t) - 1`, entered at the bar's own close (`wait = 0`,
`alignment = SIGNAL_BAR`). Then compute it.

```python
from pathlib import Path

from nautilus_trader.optimization import (
    LabelDefinition,
    LabelKind,
    label_series,
)

PANEL = Path("docs/usermanauls/factor-portfolio/sample_data/price_panel.csv")

closes = {}
for line in PANEL.read_text().splitlines()[1:]:
    ts_ns, instrument_id, close = line.split(",")
    closes.setdefault(instrument_id, []).append((int(ts_ns), float(close)))

definition = LabelDefinition(label_id="fwd5", kind=LabelKind.FORWARD_RETURN, horizon=5)
series = label_series(definition, closes["AAA.SIM"])
print("label digest:", definition.digest)
print("forward reach (ns):", series.forward_reach_ns)
for row in range(4):
    ts, close = closes["AAA.SIM"][row]
    print(f"  row {row}: close={close:.2f} label={series.value_at(ts)}")
print("last labelled row:", series.value_at(closes["AAA.SIM"][30][0]))
```

```text
label digest: sha256:6ee9d557fc529d02be702e901deb1279611f98040a6ca13bbe32dfc454cedebb
forward reach (ns): 432000000000000
  row 0: close=100.00 label=0.020000000000000018
  row 1: close=101.90 label=0.01962708537782132
  row 2: close=100.80 label=0.01984126984126977
  row 3: close=99.70 label=0.02006018054162495
last labelled row: 0.017857142857142794
```

Three things to read out of this output:

- **The digest is stable.** `sha256:6ee9...` is the identity of the policy "forward return, horizon
  5, wait 0, signal-bar alignment, propagate". Change any of those and the digest changes. Provenance
  is part of the meaning of a result.
- **The forward reach is 432000000000000 ns**, which is five days (`5 * 86_400_000_000_000`). The
  reach is *measured from the produced series*, not derived from the definition, so it accounts for
  the alignment, the wait, and a skipped window that spans more time. That number is exactly what
  the leakage policy in step 6 must cover.
- **The last labelled row is row 30.** Row 31 cannot be labelled: fewer than five future
  observations remain. The label returns an absence (`None`), never zero. `value_at` for row 31
  would print `None`.

The label is the target of prediction and may read forward. That is why the panel's no-look-ahead
rule exempts it, and why the leakage policy, not the panel, is what keeps a label from reaching into
the evaluation set.

## Step 6: split the period, and keep the future out

A backtest that trains on the whole period and reports a result on the same period proves nothing.
The period must be cut into sets, and the cut must exclude the information that leaks across the
boundary. The Python surface for this is `SplitContract` and `LeakagePolicy` in
`python/nautilus_trader/optimization/splits.py`.

State the split: a ten-day window laid out over the 36-day period, with a six-day training set, a
two-day validation set, and the rest for test. The validation set is the one whose information the
other sets may not overlap. Apply a one-day purge before, one day after, a one-day embargo between
splits, and enforce the five-day label horizon.

```python
from pathlib import Path

from nautilus_trader.optimization import (
    LabelOverlapRule,
    LeakagePolicy,
    SplitContract,
    split_contract_digest,
)

PANEL = Path("docs/usermanauls/factor-portfolio/sample_data/price_panel.csv")
DAY_NS = 86_400_000_000_000
START_NS = 1_735_689_600_000_000_000

contract = SplitContract(
    sets=("train", "validation", "test"),
    lengths={"train": 6 * DAY_NS, "validation": 2 * DAY_NS, "test": None},
    evaluation="validation",
    window=10 * DAY_NS,
    leakage=LeakagePolicy(
        purge_before=1 * DAY_NS,
        purge_after=1 * DAY_NS,
        embargo_after=1 * DAY_NS,
        label_overlap_rule=LabelOverlapRule.ENFORCE,
        label_horizon=5 * DAY_NS,
    ),
    n_splits=2,
)
print("split contract digest:", split_contract_digest(contract))
splits = contract.split(START_NS, START_NS + 36 * DAY_NS)
print("splits:", len(splits))
for index, split in enumerate(splits):
    print(f"  split {index}")
    for name, (start, stop) in split.bounds.items():
        first = (start - START_NS) // DAY_NS
        last = (stop - START_NS) // DAY_NS
        print(f"    {name:<11} day [{first}, {last}) width={stop - start} ns")
```

```text
split contract digest: sha256:09d6948f42383b4f7f98428e85c6c50086b9f0e5754868bc4829634f1758f342
splits: 2
  split 0
    train       day [0, 1) width=86400000000000 ns
    validation  day [6, 8) width=172800000000000 ns
    test        day [9, 10) width=86400000000000 ns
  split 1
    train       day [22, 23) width=86400000000000 ns
    validation  day [28, 30) width=172800000000000 ns
    test        day [31, 32) width=86400000000000 ns
```

Read the widths, not the day numbers. Training was declared six days long, but it is one day wide in
the output. The five-day label horizon was folded into the purge (because
`label_overlap_rule = ENFORCE` takes `max(purge_before, label_horizon)`), and training was cut back
to end at day 1 so that no training row's forward label reaches into the validation set that starts
at day 6. Test is one day wide because the one-day purge after the evaluation set pushed it forward
from day 8 to day 9.

That is the whole point of a split contract: it owns the layout rather than letting each caller
improvise one, and its digest (`sha256:09d6...`) names the layout so a result can say which one it
used. Lecture 06 explains purge, embargo and the horizon in more words.

## Step 7: from weights to orders

You now have weights. The engine turns them into orders through the **target pipeline**, documented
in [Target pipeline](../../concepts/target_pipeline.md). The pipeline is three layers: a **signal**
is a statement of view about one instrument; a **target** is the exposure that view resolves to; an
**order** is what reaches a venue. Only the order reaches a venue, and the pipeline itself submits
nothing: it returns order values as data.

Build the configuration and one signal per weight:

```python
from decimal import Decimal

from nautilus_trader.model import InstrumentId
from nautilus_trader.model import SignalDirection
from nautilus_trader.model import TradingSignal
from nautilus_trader.trading import TargetPipelineConfig

config = TargetPipelineConfig(
    risk_per_trade=Decimal("0.01"),
    stop_loss_bps=50,
    max_weight=Decimal("0.25"),
    commission_rate=Decimal("0.0002"),
)
print("target pipeline config:", config)

for iid, weight, direction in (
    ("CCC.SIM", 0.375, SignalDirection.SHORT),
    ("AAA.SIM", 0.125, SignalDirection.LONG),
):
    signal = TradingSignal(
        instrument_id=InstrumentId.from_str(iid),
        direction=direction,
        strength=weight,
        source="momentum_5d",
    )
    print(signal.instrument_id, signal.direction, signal.strength, signal.source)
```

```text
target pipeline config: TargetPipelineConfig { construction: TargetConstructionConfig { risk_per_trade: 0.01, stop_loss_bps: 50, max_weight: 0.25, commission_rate: 0.0002 }, min_order_quantity: Quantity(0) }
CCC.SIM SHORT 0.375 momentum_5d
AAA.SIM LONG 0.125 momentum_5d
```

`TargetPipelineConfig` bundles the fixed-risk construction settings (`risk_per_trade`,
`stop_loss_bps`, `max_weight`, `commission_rate`) with the reconciliation stage's
`min_order_quantity`. Its Python binding is `crates/trading/src/python/target_pipeline.rs`. A
strategy takes one with `enable_target_pipeline(config)` and then calls `submit_signals(signals)`:
construction resolves each signal to a size capped by `max_weight`, reconciliation nets each target
against the current position and resting orders in the cache snapshot, and one order is emitted per
target whose difference is worth submitting. A delta that is zero, below the minimum order quantity,
or that rounds to zero at the instrument's size increment produces no order.

Two properties matter for a rebalancing factor book:

- **The cache and the portfolio stay authoritative.** The pipeline builds a snapshot of their state
  and returns the minimal order set. It stores no position state of its own, so targets cannot drift
  from the account they describe.
- **It reports what it cannot resolve.** A signal naming an instrument the cache holds no definition
  or no price for is reported, and the submission does not happen, rather than being silently
  dropped. A dropped reduce-to-zero instruction would leave an unwanted position open.

## Step 8: the factor pipeline is Rust only

Steps 1 to 6 are Python because the split, label and significance layers are Python. The **factor
pipeline itself** is not. `crates/research/src/` has no `python/` directory and no PyO3 bindings;
you cannot `import nautilus_research` from Python. Everything in step 2 named as a Rust object --
`Feature::parse`, `Operator`, `Panel`, `PanelRow`, `MembershipSeries`, `DatasetDeclaration` -- is
Rust only.

The honest way to see it work is to run the crate's own tests. They are real, they pin hand-computed
numbers, and they run offline. From the repository root:

```bash
export CARGO_TARGET_DIR='D:/Users/vince/PycharmProjects/nautilus_trader/target'
cargo nextest run --locked -p nautilus-research
```

```text
   Compiling arrow-ipc v59.3.0
   Compiling arrow-cast v59.3.0
   Compiling arrow-json v59.3.0
   Compiling arrow-csv v59.3.0
   Compiling arrow-string v59.3.0
   Compiling arrow v59.3.0
   Compiling parquet v59.3.0
   Compiling tokio-stream v0.1.19
   Compiling codspeed-criterion-compat v5.0.2
   Compiling nautilus-model v0.65.0 (D:\Users\vince\PycharmProjects\nautilus_trader\crates\model)
   Compiling nautilus-persistence v0.65.0 (D:\Users\vince\PycharmProjects\nautilus_trader\crates\persistence)
   Compiling nautilus-research v0.65.0 (D:\Users\vince\PycharmProjects\nautilus_trader\crates\research)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 21m 33s
 Nextest run ID 85cb3afe-98fa-489f-910a-175ef6b14522 with nextest profile: default
    Starting 34 tests across 6 binaries
        PASS [   0.029s] ( 1/34) nautilus-research membership::tests::interval_is_half_open
        PASS [   0.036s] ( 2/34) nautilus-research operators::tests::cross_sectional_rank_breaks_ties_by_average
        PASS [   0.047s] ( 3/34) nautilus-research label::tests::missing_future_is_an_absence_not_a_zero
        PASS [   0.056s] ( 4/34) nautilus-research operators::tests::neutralisation_removes_the_group_mean
        PASS [   0.064s] ( 5/34) nautilus-research::factors combined_digest_is_independent_of_construction_order
        PASS [   0.072s] ( 6/34) nautilus-research label::tests::non_positive_price_is_a_typed_error
        PASS [   0.079s] ( 7/34) nautilus-research operators::tests::rolling_rank_breaks_ties_by_average
        PASS [   0.094s] ( 8/34) nautilus-research label::tests::forward_max_drawdown_is_non_positive
        PASS [   0.102s] ( 9/34) nautilus-research label::tests::forward_return_reads_the_horizon
        PASS [   0.111s] (10/34) nautilus-research operators::tests::undefined_statistics_are_absent
        PASS [   0.155s] (11/34) nautilus-research operators::tests::rolling_std_is_population
        PASS [   0.172s] (12/34) nautilus-research membership::tests::members_at_resolves_point_in_time
        PASS [   0.156s] (13/34) nautilus-research::factors cross_sectional_rank_orders_the_universe
        PASS [   0.162s] (14/34) nautilus-research::factors cross_sectional_scale_divides_by_the_absolute_sum
        PASS [   0.141s] (15/34) nautilus-research::factors definition_digest_separates_sides
        PASS [   0.151s] (16/34) nautilus-research::factors definition_digest_is_stable_and_pinned
        PASS [   0.158s] (17/34) nautilus-research::factors lag_reads_the_previous_observation
        PASS [   0.162s] (18/34) nautilus-research::factors neutralisation_subtracts_the_group_mean
        PASS [   0.177s] (19/34) nautilus-research::factors missing_history_is_an_absence_not_a_value
        PASS [   0.209s] (20/34) nautilus-research::factors rolling_mean_averages_the_trailing_window
        PASS [   0.216s] (21/34) nautilus-research::factors return_factor_is_the_simple_return
        PASS [   0.172s] (22/34) nautilus-research::feature_leakage forward_reading_feature_is_rejected_on_the_inference_side
        PASS [   0.159s] (23/34) nautilus-research::feature_leakage malformed_definitions_are_typed_errors_not_reinterpretations
        PASS [   0.246s] (24/34) nautilus-research::factors rolling_std_is_the_population_stddev
        PASS [   0.184s] (25/34) nautilus-research::leakage leaky_feature_reading_its_own_future_is_rejected
        PASS [   0.210s] (26/34) nautilus-research::feature_leakage forward_reading_feature_is_allowed_on_the_learning_side
        PASS [   0.238s] (27/34) nautilus-research::feature_leakage a_definition_cannot_be_used_on_the_other_side
        PASS [   0.212s] (28/34) nautilus-research::leakage feature_observed_at_the_row_timestamp_is_accepted
        PASS [   0.169s] (29/34) nautilus-research::membership panel_claiming_current_membership_for_a_past_timestamp_is_rejected
        PASS [   0.181s] (30/34) nautilus-research::membership panel_for_past_timestamp_excludes_a_later_joiner
        PASS [   0.119s] (31/34) nautilus-research::reproducibility combined_digests_are_order_independent
        PASS [   0.119s] (32/34) nautilus-research::reproducibility digest_is_independent_of_insertion_order
        PASS [   0.087s] (33/34) nautilus-research::reproducibility identical_declarations_digest_identically
        PASS [   0.259s] (34/34) nautilus-research::membership membership_series_round_trips_through_the_catalog
     Summary [   0.482s] 34 tests run: 34 passed, 0 skipped
```

(The run output above is verbatim except that thirteen of the `Compiling` lines have been collapsed
to three, and the horizontal separator lines nextest prints are omitted because this manual is
ASCII-only.)

Three of those test names are the ones this manual leaned on:

- `nautilus-research::membership panel_claiming_current_membership_for_a_past_timestamp_is_rejected`
  and `panel_for_past_timestamp_excludes_a_later_joiner` are step 4 made executable: a panel that
  claims today's membership for a past timestamp is refused, and a past-timestamp panel excludes an
  instrument that joined later.
- `nautilus-research::leakage leaky_feature_reading_its_own_future_is_rejected` builds a `PanelRow`
  whose feature's `as_of` is one nanosecond after the row timestamp and asserts the panel is refused
  with `PanelError::Lookahead`; `feature_observed_at_the_row_timestamp_is_accepted` builds a valid
  row and asserts it is accepted.
- `nautilus-research::factors definition_digest_is_stable_and_pinned` pins the digest
  `efbcf90a9df8567da04229361a3e5733d829b89effcde1896e2a370c768c3efe` for the "momentum" definition,
  and `definition_digest_separates_sides` shows the same tree on another side is another definition.

You now have a factor, a point-in-time universe, a ranking, target weights, a forward label, a split
with leakage, and a path from weights to orders. What you do not have is a reason to believe any of
it. That is [06](06-measure-and-evaluate.md).

Next: [06 - Measure and evaluate](06-measure-and-evaluate.md).
