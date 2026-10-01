# -------------------------------------------------------------------------------------------------
#  Copyright (C) 2015-2026 Nautech Systems Pty Ltd. All rights reserved.
#  https://nautechsystems.io
#
#  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
#  You may not use this file except in compliance with the License.
#  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
#
#  Unless required by applicable law or agreed to in writing, software
#  distributed under the License is distributed on an "AS IS" BASIS,
#  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#  See the License for the specific language governing permissions and
#  limitations under the License.
# -------------------------------------------------------------------------------------------------
"""
Tests for the label policies: the forward window, the alignment convention and the leakage reach.

Every expected value is hand-computed from the closes in the test, so a test does not repeat the
reduction it is meant to check: the aggregates are recomputed from their definition, the dispersion
with the sample divisor is written out rather than imported, and the first-hit case is chosen so an
asymmetric threshold and a cumulative barrier give different answers from their naive alternatives.
"""

import math
from collections.abc import Sequence
from pathlib import Path
from typing import Any
from typing import cast

import pytest

from nautilus_trader.optimization import AlignmentConvention
from nautilus_trader.optimization import ForwardAggregate
from nautilus_trader.optimization import LabelDefinition
from nautilus_trader.optimization import LabelKind
from nautilus_trader.optimization import LabelOverlapRule
from nautilus_trader.optimization import LabelSeries
from nautilus_trader.optimization import LeakagePolicy
from nautilus_trader.optimization import MissingDataPolicy
from nautilus_trader.optimization import label_series


DAILY_NS = 86_400_000_000_000
PACKAGE_ROOT = Path(__file__).resolve().parents[3] / "nautilus_trader"
LIVE_PACKAGES = ("adapters", "backtest", "execution", "live", "risk", "trading")

FORWARD = LabelDefinition(label_id="forward", kind=LabelKind.FORWARD_RETURN, horizon=2)
FIRST_HIT = LabelDefinition(
    label_id="first-hit",
    kind=LabelKind.FIRST_HIT_THRESHOLD,
    horizon=3,
    positive_threshold=0.02,
    negative_threshold=-0.05,
)


def daily(closes: Sequence[float | None]) -> list[tuple[int, float | None]]:
    """
    Return a bar series of one observation per day from the given closes.
    """
    return [(index * DAILY_NS, close) for index, close in enumerate(closes)]


def forward_aggregate(aggregate: ForwardAggregate, horizon: int = 2) -> LabelDefinition:
    """
    Return a forward-aggregate definition for the given reduction.
    """
    return LabelDefinition(
        label_id=f"aggregate-{aggregate.value}",
        kind=LabelKind.FORWARD_AGGREGATE,
        horizon=horizon,
        aggregate=aggregate,
    )


def test_a_forward_return_is_the_endpoint_ratio_from_the_entry() -> None:
    """
    Test a forward return is the endpoint ratio from the entry.
    """
    closes = [100.0, 101.0, 103.0, 102.0, 104.0, 110.0]

    series = label_series(FORWARD, daily(closes))

    assert len(series) == len(closes)
    assert series.computed == 4
    assert series.values[0] == pytest.approx(103 / 100 - 1)
    assert series.values[1] == pytest.approx(102 / 101 - 1)
    assert series.values[2] == pytest.approx(104 / 103 - 1)
    assert series.values[3] == pytest.approx(110 / 102 - 1)
    assert series.values[4:] == (None, None)
    assert series.forward_reach_ns == 2 * DAILY_NS


def test_a_zero_wait_enters_at_the_anchor_and_a_positive_wait_enters_at_the_next_bar() -> None:
    """
    Test a zero wait enters at the anchor's close and a positive wait does not.
    """
    closes = [100.0, 101.0, 103.0, 102.0]
    immediate = LabelDefinition(label_id="immediate", kind=LabelKind.FORWARD_RETURN, horizon=1)
    waited = LabelDefinition(label_id="waited", kind=LabelKind.FORWARD_RETURN, horizon=1, wait=1)

    now = label_series(immediate, daily(closes))
    later = label_series(waited, daily(closes))

    # A zero wait reads the anchor bar's own close as the entry, and the window still starts after
    # it; a wait of one enters at the next bar's close instead.
    assert now.values[0] == pytest.approx(101 / 100 - 1)
    assert later.values[0] == pytest.approx(103 / 101 - 1)
    assert now.values[1] == pytest.approx(103 / 101 - 1)
    assert immediate.entry_offset == 0
    assert waited.entry_offset == 1


def test_the_two_alignment_conventions_differ_by_a_row_and_by_the_reach() -> None:
    """
    Test both alignment pairings are distinguishable, by the values and by the reach.
    """
    closes = [100.0, 101.0, 103.0, 102.0, 104.0, 110.0]
    next_bar = LabelDefinition(
        label_id="forward",
        kind=LabelKind.FORWARD_RETURN,
        horizon=2,
        alignment=AlignmentConvention.NEXT_BAR,
    )

    signal = label_series(FORWARD, daily(closes))
    aligned = label_series(next_bar, daily(closes))

    for index in range(3):
        assert aligned.values[index] == signal.values[index + 1]
    assert aligned.values[3:] == (None, None, None)

    # Anchoring one bar later reaches one bar further beyond the feature row, so a split has to
    # exclude one more bar of training data.
    assert signal.forward_reach_ns == 2 * DAILY_NS
    assert aligned.forward_reach_ns == 3 * DAILY_NS
    assert next_bar.anchor_offset == 1


def test_the_first_hit_label_reports_the_first_breach_of_an_asymmetric_pair() -> None:
    """
    Test the first-hit label reports the first breach with asymmetric thresholds.
    """
    closes = [100.0, 94.0, 105.0, 106.0, 106.1, 106.2]

    series = label_series(FIRST_HIT, daily(closes))

    # The first row breaches -5% on its first bar (-6%), the second breaches +2% on its first bar
    # (+11.7%), and the third breaches neither (+0.95%, +1.05%, +1.14%).
    assert series.values[:3] == (-1.0, 1.0, 0.0)
    assert series.values[3:] == (None, None, None)

    # The thresholds are independent, so loosening the positive one changes the third row from no
    # breach to a breach, which a negated negative threshold could not express.
    loosened = LabelDefinition(
        label_id="first-hit-loose",
        kind=LabelKind.FIRST_HIT_THRESHOLD,
        horizon=3,
        positive_threshold=0.01,
        negative_threshold=-0.05,
    )

    assert label_series(loosened, daily(closes)).values[2] == 1.0


def test_a_first_hit_barrier_is_the_cumulative_return_from_the_entry() -> None:
    """
    Test a first-hit barrier is measured from the entry rather than per bar.
    """
    closes = [100.0, 101.0, 102.5, 102.5]
    definition = LabelDefinition(
        label_id="first-hit",
        kind=LabelKind.FIRST_HIT_THRESHOLD,
        horizon=2,
        positive_threshold=0.02,
        negative_threshold=-0.05,
    )

    series = label_series(definition, daily(closes))

    # The second bar's own return is +1.49%, which reaches neither threshold; the holding period
    # from the entry is +2.5%, which reaches the positive one.
    assert series.values[0] == 1.0
    assert series.values[1] == 0.0


def test_an_early_first_hit_reaches_less_far_than_a_window_end() -> None:
    """
    Test the reach of a first-hit label stops at the observation it reads.
    """
    closes = [100.0, 103.0, 104.0, 105.0]
    unreached = LabelDefinition(
        label_id="unreached",
        kind=LabelKind.FIRST_HIT_THRESHOLD,
        horizon=3,
        positive_threshold=0.1,
        negative_threshold=-0.05,
    )

    # +3% on the first bar breaches the first definition, which reads one bar; the second reads the
    # whole window looking for +10% and never finds it.
    assert label_series(FIRST_HIT, daily(closes)).forward_reach_ns == DAILY_NS
    assert label_series(unreached, daily(closes)).forward_reach_ns == 3 * DAILY_NS


def test_a_missing_close_propagates_or_is_skipped_with_a_longer_reach() -> None:
    """
    Test the two missing-data policies, and that skipping covers more time.
    """
    closes = [100.0, None, 103.0, 104.0, 105.0]
    skipping = LabelDefinition(
        label_id="forward",
        kind=LabelKind.FORWARD_RETURN,
        horizon=2,
        missing_data_policy=MissingDataPolicy.SKIP,
    )

    propagated = label_series(FORWARD, daily(closes))
    skipped = label_series(skipping, daily(closes))

    # Propagating: only the third row has a complete window, and it reaches two days.
    assert propagated.computed == 1
    assert propagated.values[0] is None
    assert propagated.values[1] is None
    assert propagated.values[2] == pytest.approx(105 / 103 - 1)
    assert propagated.forward_reach_ns == 2 * DAILY_NS

    # Skipping: the first row measures the two present observations after its own, which spans three
    # days, and the next row's window starts at the first present observation. The missing bar is
    # not counted, so two adjacent rows then share one window and carry the same label, which is the
    # reason the policy is declared rather than assumed.
    assert skipped.computed == 3
    assert skipped.values[0] == pytest.approx(104 / 100 - 1)
    assert skipped.values[1] == pytest.approx(105 / 103 - 1)
    assert skipped.values[1] == skipped.values[2]
    assert skipped.values[3] is None
    assert skipped.forward_reach_ns == 3 * DAILY_NS


def test_a_window_with_fewer_present_observations_than_the_horizon_has_no_label() -> None:
    """
    Test a window short of its horizon has no label under either policy.
    """
    closes = [100.0, None, 103.0]
    skipping = LabelDefinition(
        label_id="forward",
        kind=LabelKind.FORWARD_RETURN,
        horizon=2,
        missing_data_policy=MissingDataPolicy.SKIP,
    )

    assert label_series(FORWARD, daily(closes)).computed == 0
    skipped = label_series(skipping, daily(closes))

    assert skipped.computed == 0
    assert skipped.forward_reach_ns == 0


def test_the_forward_aggregates_reduce_the_per_bar_returns() -> None:
    """
    Test the aggregates reduce the window's per-bar returns with the sample divisor.
    """
    closes = [100.0, 101.0, 103.0, 100.0]
    first = 101 / 100 - 1
    second = 103 / 101 - 1
    mean = (first + second) / 2
    squared = (first - mean) ** 2 + (second - mean) ** 2
    sample = math.sqrt(squared)
    population = math.sqrt(squared / 2)

    values = {
        aggregate: label_series(forward_aggregate(aggregate), daily(closes)).values[0]
        for aggregate in ForwardAggregate
    }

    assert values[ForwardAggregate.MEAN] == pytest.approx(mean)
    assert values[ForwardAggregate.STD] == pytest.approx(sample)
    assert values[ForwardAggregate.STD] != pytest.approx(population)
    assert values[ForwardAggregate.MIN] == pytest.approx(first)
    assert values[ForwardAggregate.MAX] == pytest.approx(second)


def test_a_label_series_is_refused_as_market_data() -> None:
    """
    Test a label value cannot be read into the feature path.

    A label is an outcome of the future, so reading one where market data is expected is the leakage
    the target path exists to prevent. The mistake is refused by name.
    """
    label = label_series(FORWARD, daily([100.0, 101.0, 103.0]))
    smuggled: object = label

    with pytest.raises(TypeError, match="cannot be read as market data"):
        label_series(FORWARD, cast("list[tuple[int, float | None]]", smuggled))

    assert isinstance(label, LabelSeries)


def test_the_label_path_is_not_reachable_from_the_live_packages() -> None:
    """
    Test no live-path package imports the label policies.
    """
    offenders = []
    for package in LIVE_PACKAGES:
        for path in (PACKAGE_ROOT / package).rglob("*.py"):
            source = path.read_text(encoding="utf-8")
            if "optimization.labels" in source or "optimization import labels" in source:
                offenders.append(str(path.relative_to(PACKAGE_ROOT)))

    assert not offenders, f"live-path modules import the label policies: {offenders}"


def test_a_leakage_policy_that_does_not_cover_the_reach_is_refused() -> None:
    """
    Test the dataset-level leakage validation refuses an uncovered label reach.
    """
    series = label_series(FORWARD, daily([100.0, 101.0, 103.0, 102.0, 104.0]))
    short = LeakagePolicy(purge_before=DAILY_NS, zero_interval_justification="test")
    covering = LeakagePolicy(purge_before=2 * DAILY_NS, zero_interval_justification="test")

    assert series.forward_reach_ns == 2 * DAILY_NS
    assert series.leakage_shortfall_ns(short) == DAILY_NS
    assert series.leakage_shortfall_ns(covering) == 0
    series.validate_leakage(covering)

    with pytest.raises(ValueError, match="would carry evaluation information"):
        series.validate_leakage(short)


def test_a_declared_label_horizon_covers_the_reach_only_when_long_enough() -> None:
    """
    Test the leakage policy's declared label horizon is compared against the reach.
    """
    series = label_series(FORWARD, daily([100.0, 101.0, 103.0, 102.0, 104.0]))
    short = LeakagePolicy(
        label_overlap_rule=LabelOverlapRule.ENFORCE,
        label_horizon=DAILY_NS,
        zero_interval_justification="test",
    )
    long_enough = LeakagePolicy(
        label_overlap_rule=LabelOverlapRule.ENFORCE,
        label_horizon=2 * DAILY_NS,
        zero_interval_justification="test",
    )

    # An enforced label horizon is folded into the purge, so a declared horizon that covers the
    # reach is enough without a purge interval of its own.
    assert long_enough.purge == 2 * DAILY_NS
    assert series.leakage_shortfall_ns(long_enough) == 0
    series.validate_leakage(long_enough)

    assert series.leakage_shortfall_ns(short) == DAILY_NS

    with pytest.raises(ValueError, match="would carry evaluation information"):
        series.validate_leakage(short)


def test_a_definition_refuses_a_field_its_kind_does_not_use() -> None:
    """
    Test a definition refuses a field that its kind never applies.
    """
    with pytest.raises(ValueError, match="aggregate is only applied"):
        LabelDefinition(
            label_id="x",
            kind=LabelKind.FORWARD_RETURN,
            horizon=2,
            aggregate=ForwardAggregate.MEAN,
        )

    with pytest.raises(ValueError, match="thresholds are only applied"):
        LabelDefinition(
            label_id="x",
            kind=LabelKind.FORWARD_AGGREGATE,
            horizon=2,
            aggregate=ForwardAggregate.MEAN,
            positive_threshold=0.02,
        )

    with pytest.raises(ValueError, match="requires an aggregate"):
        LabelDefinition(label_id="x", kind=LabelKind.FORWARD_AGGREGATE, horizon=2)

    with pytest.raises(ValueError, match="requires both thresholds"):
        LabelDefinition(label_id="x", kind=LabelKind.FIRST_HIT_THRESHOLD, horizon=2)

    with pytest.raises(ValueError, match="positive_threshold must be strictly positive"):
        LabelDefinition(
            label_id="x",
            kind=LabelKind.FIRST_HIT_THRESHOLD,
            horizon=2,
            positive_threshold=0.0,
            negative_threshold=-0.05,
        )

    with pytest.raises(ValueError, match="negative_threshold must be strictly negative"):
        LabelDefinition(
            label_id="x",
            kind=LabelKind.FIRST_HIT_THRESHOLD,
            horizon=2,
            positive_threshold=0.02,
            negative_threshold=0.05,
        )

    with pytest.raises(ValueError, match="at least 2 observations"):
        LabelDefinition(
            label_id="x",
            kind=LabelKind.FORWARD_AGGREGATE,
            horizon=1,
            aggregate=ForwardAggregate.STD,
        )

    with pytest.raises(ValueError, match="horizon must be at least 1"):
        LabelDefinition(label_id="x", kind=LabelKind.FORWARD_RETURN, horizon=0)

    with pytest.raises(ValueError, match="wait must not be negative"):
        LabelDefinition(label_id="x", kind=LabelKind.FORWARD_RETURN, horizon=2, wait=-1)

    with pytest.raises(ValueError, match="label_id must be a non-empty string"):
        LabelDefinition(label_id=" ", kind=LabelKind.FORWARD_RETURN, horizon=2)

    not_an_int: Any = 2.0

    with pytest.raises(TypeError, match="horizon must be an int"):
        LabelDefinition(label_id="x", kind=LabelKind.FORWARD_RETURN, horizon=not_an_int)


def test_the_digest_separates_the_alignment_and_the_thresholds() -> None:
    """
    Test the definition digest follows every field.
    """
    next_bar = LabelDefinition(
        label_id="forward",
        kind=LabelKind.FORWARD_RETURN,
        horizon=2,
        alignment=AlignmentConvention.NEXT_BAR,
    )
    same = LabelDefinition(label_id="forward", kind=LabelKind.FORWARD_RETURN, horizon=2)
    widened = LabelDefinition(
        label_id="first-hit",
        kind=LabelKind.FIRST_HIT_THRESHOLD,
        horizon=3,
        positive_threshold=0.03,
        negative_threshold=-0.05,
    )

    assert FORWARD.digest == same.digest
    assert FORWARD.digest != next_bar.digest
    assert FORWARD.digest != widened.digest
    assert FIRST_HIT.digest != widened.digest
    assert FORWARD.digest.startswith("sha256:")


def test_an_infinite_close_is_refused_and_a_nan_close_is_missing() -> None:
    """
    Test a missing value is never read as data.
    """
    with pytest.raises(ValueError, match="non-finite close"):
        label_series(FORWARD, daily([100.0, math.inf, 103.0]))

    series = label_series(FORWARD, daily([100.0, math.nan, 103.0]))

    assert series.computed == 0


def test_a_series_is_read_by_timestamp_and_refuses_an_unknown_row() -> None:
    """
    Test a label series can be read by timestamp and refuses a row it does not have.
    """
    series = label_series(FORWARD, daily([100.0, 101.0, 103.0]))

    assert series.value_at(0) == pytest.approx(103 / 100 - 1)

    with pytest.raises(KeyError, match="is not a feature row"):
        series.value_at(5 * DAILY_NS)


def test_an_empty_series_has_no_labels() -> None:
    """
    Test an empty bar series produces an empty label series.
    """
    series = label_series(FORWARD, [])

    assert len(series) == 0
    assert series.computed == 0
    assert series.forward_reach_ns == 0


def test_timestamps_must_strictly_increase() -> None:
    """
    Test a repeated timestamp is refused.
    """
    with pytest.raises(ValueError, match="must strictly increase"):
        label_series(FORWARD, [(0, 100.0), (0, 101.0)])
