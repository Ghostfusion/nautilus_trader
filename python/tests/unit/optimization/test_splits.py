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
Tests for the split contract and the leakage exclusion relation.
"""

from __future__ import annotations

from itertools import pairwise
from typing import TYPE_CHECKING

import pytest

from nautilus_trader.optimization.splits import LabelOverlapRule
from nautilus_trader.optimization.splits import LeakagePolicy
from nautilus_trader.optimization.splits import SplitContract
from nautilus_trader.optimization.splits import SplitDirection
from nautilus_trader.optimization.stages import WALK_FORWARD_LEAKAGE
from nautilus_trader.optimization.stages import WalkForwardWindow
from nautilus_trader.optimization.stages import walk_forward_windows


if TYPE_CHECKING:
    from collections.abc import Mapping

    from nautilus_trader.optimization.splits import SetLength
    from nautilus_trader.optimization.splits import Split


JUSTIFICATION = "The study declares no label horizon."


def leaky(
    purge_before: int | None = None,
    purge_after: int | None = None,
    embargo_after: int | None = None,
    label_overlap_rule: LabelOverlapRule = LabelOverlapRule.NONE,
    label_horizon: int | None = None,
) -> LeakagePolicy:
    """
    Build a policy, justifying every interval it leaves at zero.
    """
    return LeakagePolicy(
        purge_before=purge_before,
        purge_after=purge_after,
        embargo_after=embargo_after,
        label_overlap_rule=label_overlap_rule,
        label_horizon=label_horizon,
        zero_interval_justification=JUSTIFICATION,
    )


def zero_leakage() -> LeakagePolicy:
    """
    Build the declared zero policy, which applies no exclusion.
    """
    return leaky()


def two_sets(
    lengths: Mapping[str, SetLength | None] | None = None,
    leakage: LeakagePolicy | None = None,
    min_length: int | None = None,
    n_splits: int | None = None,
) -> SplitContract:
    """
    Build a two-set contract with an in-sample and an out-of-sample set.
    """
    return SplitContract(
        sets=("train", "test"),
        lengths={"train": 100, "test": 100} if lengths is None else lengths,
        leakage=zero_leakage() if leakage is None else leakage,
        min_length=min_length,
        n_splits=n_splits,
    )


def assert_sets_are_ordered(splits: tuple[Split, ...]) -> None:
    """
    Assert that the sets of every split follow one another without overlapping.
    """
    for split in splits:
        for current, following in pairwise(split.bounds.values()):
            assert current[1] <= following[0]


def test_absolute_lengths_lay_out_whole_windows() -> None:
    """
    Test absolute lengths cut the period into adjacent windows of their sum.
    """
    splits = two_sets().split(0, 1000)

    assert [split.bounds for split in splits] == [
        {"train": (0, 100), "test": (100, 200)},
        {"train": (200, 300), "test": (300, 400)},
        {"train": (400, 500), "test": (500, 600)},
        {"train": (600, 700), "test": (700, 800)},
        {"train": (800, 900), "test": (900, 1000)},
    ]
    assert_sets_are_ordered(splits)


def test_fractional_lengths_require_a_window_and_absorb_the_rest() -> None:
    """
    Test a fractional length resolves against the window and an omitted set takes the rest.
    """
    contract = SplitContract(
        sets=("train", "test"),
        lengths={"train": 0.75, "test": None},
        leakage=zero_leakage(),
        window=400,
    )

    assert contract.resolved_lengths() == {"train": 300, "test": 100}


def test_fractional_lengths_accounting_for_the_window_absorb_the_rounding() -> None:
    """
    Test fractions summing to one leave the last set the rounding difference.
    """
    contract = SplitContract(
        sets=("train", "test"),
        lengths={"train": 0.5, "test": 0.5},
        leakage=zero_leakage(),
        window=101,
    )

    assert contract.resolved_lengths() == {"train": 50, "test": 51}


@pytest.mark.parametrize(
    ("lengths", "window", "message"),
    [
        ({"train": 0.5, "test": None}, None, "requires an explicit window"),
        ({"train": 100, "test": None}, None, "requires an explicit window"),
        ({"train": 0.0, "test": None}, 400, "must be in"),
        ({"train": 1.0, "test": None}, 400, "must be in"),
        ({"train": 0, "test": None}, 400, "must be positive"),
    ],
)
def test_lengths_that_cannot_resolve_are_refused(
    lengths: dict[str, SetLength | None],
    window: int | None,
    message: str,
) -> None:
    """
    Test a length that cannot resolve to nanoseconds is refused, naming the constraint.
    """
    contract = SplitContract(
        sets=("train", "test"),
        lengths=lengths,
        leakage=zero_leakage(),
        window=window,
    )

    with pytest.raises(ValueError, match=message):
        contract.resolved_lengths()


def test_lengths_that_do_not_fill_the_window_are_refused() -> None:
    """
    Test set lengths that fall short of the declared window are refused rather than rescaled.
    """
    contract = SplitContract(
        sets=("train", "test"),
        lengths={"train": 100, "test": 100},
        leakage=zero_leakage(),
        window=400,
    )

    with pytest.raises(ValueError, match="not the declared window"):
        contract.resolved_lengths()


def test_direction_places_the_leftover_span() -> None:
    """
    Test each direction leaves the leftover span where it says it does.
    """
    lengths = {"train": 100, "test": 100}
    exact = SplitContract(
        sets=("train", "test"),
        lengths=lengths,
        leakage=zero_leakage(),
        direction=SplitDirection.EXACT,
    )
    forward = SplitContract(
        sets=("train", "test"),
        lengths=lengths,
        leakage=zero_leakage(),
        direction=SplitDirection.FORWARD,
    )
    reversed_ = SplitContract(
        sets=("train", "test"),
        lengths=lengths,
        leakage=zero_leakage(),
        direction=SplitDirection.REVERSED,
    )

    # A span of 450 ns leaves 50 ns beyond two 200 ns windows.
    assert [split.bounds["test"] for split in exact.split(0, 450)] == [(100, 200), (300, 400)]
    assert [split.bounds["test"] for split in forward.split(0, 450)] == [(100, 200), (300, 450)]
    assert [split.bounds["train"] for split in reversed_.split(0, 450)] == [(0, 150), (250, 350)]


def test_purge_before_trims_a_preceding_set() -> None:
    """
    Test the purge before the evaluation set removes the adjacent training tail.
    """
    contract = two_sets(
        lengths={"train": 400, "test": 200},
        leakage=leaky(purge_before=100),
    )

    assert contract.split(0, 600)[0].bounds == {"train": (0, 300), "test": (400, 600)}


def test_purge_after_moves_a_following_set() -> None:
    """
    Test the purge after the evaluation set moves a following set, not a preceding one.
    """
    contract = SplitContract(
        sets=("train", "test", "holdout"),
        lengths={"train": 200, "test": 100, "holdout": 100},
        leakage=leaky(purge_after=50),
        evaluation="test",
    )

    assert contract.split(0, 400)[0].bounds == {
        "train": (0, 200),
        "test": (200, 300),
        "holdout": (350, 400),
    }


def test_embargo_after_spaces_consecutive_splits() -> None:
    """
    Test the embargo widens the stride between splits, leaving the sets untouched.
    """
    lengths = {"train": 100, "test": 100}
    adjacent = two_sets(lengths=lengths)
    separated = two_sets(lengths=lengths, leakage=leaky(embargo_after=100))

    assert [split.bounds["train"][0] for split in adjacent.split(0, 600)] == [0, 200, 400]
    assert [split.bounds["train"][0] for split in separated.split(0, 600)] == [0, 300]


def test_label_overlap_rule_folds_the_horizon_into_the_purge() -> None:
    """
    Test the enforcing label rule takes the larger of the horizon and the declared purge.
    """
    horizon = LeakagePolicy(
        purge_before=40,
        label_overlap_rule=LabelOverlapRule.ENFORCE,
        label_horizon=100,
        zero_interval_justification=JUSTIFICATION,
    )
    longer = LeakagePolicy(
        purge_before=200,
        label_overlap_rule=LabelOverlapRule.ENFORCE,
        label_horizon=100,
        zero_interval_justification=JUSTIFICATION,
    )
    declared = leaky(purge_before=40)

    assert horizon.purge == 100
    assert longer.purge == 200
    assert declared.purge == 40


def test_no_training_observation_reaches_into_the_label_information() -> None:
    """
    Test no training observation's label window overlaps the evaluation window.
    """
    horizon = 100
    timestamps = list(range(0, 600, 10))
    policy = LeakagePolicy(
        label_overlap_rule=LabelOverlapRule.ENFORCE,
        label_horizon=horizon,
        zero_interval_justification=JUSTIFICATION,
    )
    contract = two_sets(lengths={"train": 400, "test": 200}, leakage=policy)

    positions = contract.split(0, 600)[0].indices(timestamps)
    training = [timestamps[index] for index in positions["train"]]
    evaluation = [timestamps[index] for index in positions["test"]]

    assert training
    assert evaluation
    assert max(training) + horizon <= min(evaluation)
    assert min(evaluation) == 400


def test_indices_are_the_positions_inside_the_half_open_bounds() -> None:
    """
    Test the index arrays are the observations inside each set's half-open bounds.
    """
    timestamps = [0, 10, 20, 30, 40, 50]
    contract = two_sets(lengths={"train": 30, "test": 30})

    positions = contract.split(0, 60)[0].indices(timestamps)

    assert positions == {"train": (0, 1, 2), "test": (3, 4, 5)}


def test_a_split_with_too_short_a_set_is_dropped() -> None:
    """
    Test the minimum length drops a split rather than shortening it silently.
    """
    lengths = {"train": 200, "test": 200}
    policy = leaky(purge_before=150)
    kept = two_sets(lengths=lengths, leakage=policy)
    dropped = two_sets(lengths=lengths, leakage=policy, min_length=100)

    assert kept.split(0, 400)[0].bounds["train"] == (0, 50)
    assert dropped.split(0, 400) == ()


def test_an_empty_set_after_the_exclusions_drops_the_split() -> None:
    """
    Test a purge that consumes the whole preceding set drops the split.
    """
    lengths = {"train": 200, "test": 200}
    contract = two_sets(lengths=lengths, leakage=leaky(purge_before=200))

    assert contract.split(0, 400) == ()


def test_a_period_shorter_than_a_window_yields_nothing() -> None:
    """
    Test a period that cannot hold one whole window yields no split.
    """
    assert two_sets().split(0, 199) == ()
    assert two_sets().split(100, 100) == ()


def test_n_splits_selects_evenly_spaced_splits() -> None:
    """
    Test a requested count selects that many evenly spaced splits, not the first ones.
    """
    splits = two_sets(n_splits=3).split(0, 1000)

    assert [split.bounds["train"][0] for split in splits] == [0, 400, 800]


def test_a_single_requested_split_is_the_middle_one() -> None:
    """
    Test a request for one split selects the middle split rather than the first.
    """
    splits = two_sets(n_splits=1).split(0, 1000)

    assert [split.bounds["train"][0] for split in splits] == [400]


def test_a_split_count_beyond_the_available_splits_is_refused() -> None:
    """
    Test a request for more splits than fit the period is refused, naming the constraint.
    """
    with pytest.raises(ValueError, match="n_splits requested 5"):
        two_sets(n_splits=5).split(0, 500)


def test_zero_intervals_require_a_justification() -> None:
    """
    Test a zero interval is refused when the policy does not say why it is zero.
    """
    assert zero_leakage().purge == 0

    with pytest.raises(ValueError, match="zero intervals"):
        LeakagePolicy()
    with pytest.raises(ValueError, match="zero intervals"):
        LeakagePolicy(purge_before=100)


def test_a_justification_without_a_zero_interval_is_refused() -> None:
    """
    Test a justification for intervals that are not zero is refused as a stale decision.
    """
    with pytest.raises(ValueError, match="while every interval is positive"):
        LeakagePolicy(
            purge_before=1,
            purge_after=1,
            embargo_after=1,
            zero_interval_justification=JUSTIFICATION,
        )


def test_zero_by_omission_is_distinguishable_from_zero_by_decision() -> None:
    """
    Test an unset interval and a decided zero have different identities.
    """
    omitted = zero_leakage()
    decided = LeakagePolicy(
        purge_before=0,
        purge_after=0,
        embargo_after=0,
        zero_interval_justification=JUSTIFICATION,
    )
    same = LeakagePolicy(
        purge_before=0,
        purge_after=0,
        embargo_after=0,
        zero_interval_justification=JUSTIFICATION,
    )

    assert omitted.purge == decided.purge == 0
    assert omitted.decided == frozenset()
    assert decided.decided == frozenset({"purge_before", "purge_after", "embargo_after"})
    assert omitted.digest != decided.digest
    assert decided.digest == same.digest


def test_a_label_horizon_without_the_enforcing_rule_is_refused() -> None:
    """
    Test a declared horizon that no rule applies is refused rather than ignored.
    """
    with pytest.raises(ValueError, match="requires label_overlap_rule 'enforce'"):
        LeakagePolicy(label_horizon=100, zero_interval_justification=JUSTIFICATION)

    with pytest.raises(ValueError, match="requires a positive label_horizon"):
        LeakagePolicy(label_overlap_rule=LabelOverlapRule.ENFORCE)


def test_negative_intervals_and_unknown_sets_are_refused() -> None:
    """
    Test a negative interval, an unknown set and a missing length are refused.
    """
    with pytest.raises(ValueError, match="must not be negative"):
        LeakagePolicy(purge_before=-1)

    with pytest.raises(ValueError, match="unknown sets"):
        two_sets(lengths={"train": 100, "test": 100, "holdout": 100})

    with pytest.raises(ValueError, match="lengths missing for sets"):
        SplitContract(sets=("train", "test"), lengths={"train": 100}, leakage=zero_leakage())

    with pytest.raises(ValueError, match="at most one set length may be omitted"):
        SplitContract(
            sets=("train", "test"),
            lengths={"train": None, "test": None},
            leakage=zero_leakage(),
            window=200,
        )

    with pytest.raises(ValueError, match="is not one of"):
        SplitContract(
            sets=("train", "test"),
            lengths={"train": 100, "test": 100},
            leakage=zero_leakage(),
            evaluation="holdout",
        )


def test_walk_forward_windows_come_from_the_contract() -> None:
    """
    Test the walk-forward windows are the contract's bounds for the same inputs.
    """
    windows = walk_forward_windows(0, 1000, in_sample=400, out_of_sample=200)
    contract = SplitContract(
        sets=("in_sample", "out_of_sample"),
        lengths={"in_sample": 400, "out_of_sample": 200},
        leakage=WALK_FORWARD_LEAKAGE,
        direction=SplitDirection.EXACT,
    )

    assert windows == (WalkForwardWindow(0, 400, 400, 600),)
    assert [
        (
            window.in_sample_start,
            window.in_sample_end,
            window.out_of_sample_start,
            window.out_of_sample_end,
        )
        for window in windows
    ] == [
        (*split.bounds["in_sample"], *split.bounds["out_of_sample"])
        for split in contract.split(0, 1000)
    ]


def test_walk_forward_windows_match_the_pinned_layout() -> None:
    """
    Test the layouts the stages produced before the migration are unchanged.
    """
    assert walk_forward_windows(1000, 5000, in_sample=2000, out_of_sample=1000) == (
        WalkForwardWindow(1000, 3000, 3000, 4000),
    )
    assert walk_forward_windows(0, 1000, in_sample=100, out_of_sample=100) == (
        WalkForwardWindow(0, 100, 100, 200),
        WalkForwardWindow(200, 300, 300, 400),
        WalkForwardWindow(400, 500, 500, 600),
        WalkForwardWindow(600, 700, 700, 800),
        WalkForwardWindow(800, 900, 900, 1000),
    )


def test_walk_forward_applies_the_declared_leakage() -> None:
    """
    Test a declared purge reaches the walk-forward windows.
    """
    windows = walk_forward_windows(
        0,
        600,
        in_sample=400,
        out_of_sample=200,
        leakage=leaky(purge_before=100),
    )

    assert windows == (WalkForwardWindow(0, 300, 400, 600),)


def test_walk_forward_requires_positive_segments() -> None:
    """
    Test a non-positive segment length is refused before any run.
    """
    with pytest.raises(ValueError, match="must be positive"):
        walk_forward_windows(0, 1000, in_sample=0, out_of_sample=100)

    with pytest.raises(ValueError, match="must be positive"):
        walk_forward_windows(0, 1000, in_sample=100, out_of_sample=0)

    assert walk_forward_windows(1000, 1000, in_sample=100, out_of_sample=100) == ()
