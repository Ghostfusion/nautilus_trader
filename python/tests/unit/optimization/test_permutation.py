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
Tests for the permutation test of a backtest result.

The invariants checked here are the ones a wrong implementation breaks silently: that a rearrangement
is a rearrangement and never a new value, that a block keeps its internal adjacency, that the p-value
is the counted share of the runs that were scored, and that the draw does not depend on the global
random state.
"""

import json
import math
import random
from typing import cast

import pytest

from nautilus_trader.optimization import PermutationTestResult
from nautilus_trader.optimization import ShuffleUnit
from nautilus_trader.optimization import TestDirection
from nautilus_trader.optimization import permutation_test
from nautilus_trader.optimization import ratio_of_mean_to_deviation
from nautilus_trader.optimization import shuffled_orders
from nautilus_trader.optimization import total_return


def _last(values) -> float:
    """
    Return the last outcome of a rearrangement.
    """
    return float(values[-1])


def _ratio_of_gap(values) -> float:
    """
    Return a statistic that is undefined when the two leading outcomes are equal.

    It stands for any statistic over a rearrangement that can refuse a run, which is what the
    refused count exists for.
    """
    first = float(values[0])
    second = float(values[1])

    if first == second:
        return math.nan

    return first / (first - second)


def test_a_rearrangement_preserves_the_multiset() -> None:
    """
    Test a rearranged sequence holds exactly the observed outcomes and invents none.
    """
    values = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]

    orders = list(shuffled_orders(values, iterations=20, seed=0))

    assert len(orders) == 20

    for order in orders:
        assert sorted(order) == sorted(values)


def test_a_block_rearrangement_keeps_adjacency_within_a_block() -> None:
    """
    Test the outcomes of a block stay together, which is what preserves short-horizon structure.
    """
    values = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]
    groups = [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]]

    orders = list(shuffled_orders(values, iterations=20, seed=1, block_size=3))

    for order in orders:
        assert order[0:3] in groups
        assert order[3:6] in groups
        assert order[0:3] != order[3:6]


def test_a_trailing_partial_block_is_left_in_place() -> None:
    """
    Test an outcome beyond the last whole block is not dropped and not sheared from its block.
    """
    values = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]

    orders = list(shuffled_orders(values, iterations=20, seed=2, block_size=3))

    for order in orders:
        assert order[-1] == 7.0


def test_the_same_seed_repeats_the_draw() -> None:
    """
    Test the rearrangements are a function of the seed and of nothing else.
    """
    values = [1.0, 2.0, 3.0, 4.0, 5.0]

    first = list(shuffled_orders(values, iterations=10, seed=42))
    second = list(shuffled_orders(values, iterations=10, seed=42))

    assert first == second


def test_a_rearrangement_can_move_something() -> None:
    """
    Test the draw is a rearrangement rather than the identity permutation repeated.
    """
    values = [1.0, 2.0, 3.0, 4.0, 5.0]

    orders = list(shuffled_orders(values, iterations=10, seed=0))

    assert len({tuple(order) for order in orders}) > 1


def test_a_sequence_too_short_to_rearrange_is_refused() -> None:
    """
    Test one outcome has no rearrangement to speak about.
    """
    with pytest.raises(ValueError, match="at least 2 outcomes"):
        list(shuffled_orders([1.0], iterations=1, seed=0))


def test_a_missing_outcome_is_refused_rather_than_dropped() -> None:
    """
    Test a non-finite outcome is refused, because dropping it changes the population rearranged.
    """
    with pytest.raises(ValueError, match="must be finite"):
        list(shuffled_orders([1.0, float("nan")], iterations=1, seed=0))

    with pytest.raises(ValueError, match="must be finite"):
        list(shuffled_orders([1.0, float("inf")], iterations=1, seed=0))


def test_a_non_positive_iteration_count_is_refused() -> None:
    """
    Test a draw of nothing is refused.
    """
    with pytest.raises(ValueError, match="at least 1"):
        list(shuffled_orders([1.0, 2.0], iterations=0, seed=0))


def test_a_block_that_cannot_move_is_refused() -> None:
    """
    Test a block size outside its range is refused rather than silently treated as a no-op.
    """
    with pytest.raises(ValueError, match="at least 1"):
        list(shuffled_orders([1.0, 2.0], iterations=1, seed=0, block_size=0))

    with pytest.raises(ValueError, match="not smaller than"):
        list(shuffled_orders([1.0, 2.0], iterations=1, seed=0, block_size=2))


def test_a_block_size_of_the_wrong_type_is_refused() -> None:
    """
    Test a block size that is not an integer is refused rather than truncated.
    """
    with pytest.raises(TypeError, match="block_size must be an int or None"):
        list(shuffled_orders([1.0, 2.0, 3.0], iterations=1, seed=0, block_size=1.5))


def test_a_statistic_the_order_cannot_change_scores_one() -> None:
    """
    Test a share of one for a statistic a rearrangement cannot improve on.
    """
    # The maximum of a rearrangement is always the observed maximum, so every scored run is at
    # least as good as the real one.
    result = permutation_test(
        [1.0, 2.0, 3.0, 4.0],
        statistic=max,
        metric="largest outcome",
        iterations=100,
        seed=3,
    )

    assert result.p_value == 1.0
    assert result.extreme_count == 100
    assert result.scored == 100
    assert result.unit is ShuffleUnit.TRADE_ORDER
    assert len(result.distribution) == 100


def test_the_p_value_is_the_counted_share_of_the_scored_runs() -> None:
    """
    Test the share is the counted extreme runs over the runs that were scored, and nothing else.
    """
    result = permutation_test(
        [1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
        statistic=_last,
        metric="last outcome",
        iterations=50,
        seed=0,
    )

    assert result.observed == 6.0
    assert result.p_value == result.extreme_count / result.scored
    assert 0 <= result.extreme_count < 50


def test_the_direction_decides_which_runs_count() -> None:
    """
    Test a metric that is better when smaller is counted the other way.
    """
    values = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]

    higher = permutation_test(
        values,
        statistic=_last,
        metric="last outcome",
        iterations=50,
        seed=0,
        direction=TestDirection.HIGHER_IS_BETTER,
    )
    lower = permutation_test(
        values,
        statistic=_last,
        metric="last outcome",
        iterations=50,
        seed=0,
        direction=TestDirection.LOWER_IS_BETTER,
    )

    # The observed last outcome is the largest, so nothing can beat it: only the runs that put it
    # last count as at least as good, while every run is at most as good as the real one.
    assert higher.p_value < 1.0
    assert higher.extreme_count < higher.scored
    assert lower.p_value == 1.0
    assert lower.extreme_count == lower.scored


def test_the_block_unit_is_recorded() -> None:
    """
    Test the record states which unit was moved, so a share cannot be read under the wrong null.
    """
    result = permutation_test(
        [1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
        statistic=_last,
        metric="last outcome",
        iterations=10,
        seed=0,
        block_size=3,
    )

    assert result.unit is ShuffleUnit.RETURN_BLOCK
    assert result.block_size == 3


def test_the_draw_does_not_depend_on_the_global_generator() -> None:
    """
    Test another consumer of random numbers cannot move the rearrangements.
    """
    values = [1.0, 2.0, 3.0, 4.0, 5.0]

    first = permutation_test(
        values,
        statistic=_last,
        metric="last outcome",
        iterations=20,
        seed=7,
    )

    random.seed(999)
    random.random()  # noqa: S311 (deliberately perturbing the global state under test)

    second = permutation_test(
        values,
        statistic=_last,
        metric="last outcome",
        iterations=20,
        seed=7,
    )

    assert first.distribution == second.distribution
    assert first.p_value == second.p_value


def test_a_rearrangement_the_statistic_cannot_score_is_counted_and_excluded() -> None:
    """
    Test an unscorable rearrangement leaves the denominator rather than the report.
    """
    # The two equal trailing outcomes make the statistic undefined on some rearrangements.
    values = [1.0, 2.0, 2.0]

    result = permutation_test(
        values,
        statistic=_ratio_of_gap,
        metric="ratio of the leading gap",
        iterations=20,
        seed=5,
    )

    assert result.refused >= 1
    assert result.scored == result.iterations - result.refused
    assert len(result.distribution) == result.scored
    assert result.p_value == result.extreme_count / result.scored


def test_a_test_with_nothing_scorable_reports_no_share() -> None:
    """
    Test a share that cannot be computed is reported as no share rather than as zero.
    """
    values = [1.0, 2.0, 3.0]

    # The statistic scores the caller's own sequence and nothing else, so every rearrangement is
    # refused and there is no share to report.
    result = permutation_test(
        values,
        statistic=lambda candidate: 1.0 if candidate is values else math.nan,
        metric="only the real order",
        iterations=10,
        seed=0,
    )

    assert result.refused == 10
    assert result.scored == 0
    assert math.isnan(result.p_value)


def test_a_non_finite_statistic_on_the_real_sequence_is_refused() -> None:
    """
    Test a real sequence with no finite statistic has nothing to compare against.
    """
    with pytest.raises(ValueError, match="is not finite"):
        permutation_test(
            [1.0, 2.0],
            statistic=lambda _candidate: math.nan,
            metric="undefined",
            iterations=1,
            seed=0,
        )


def test_a_statistic_that_is_not_callable_is_refused() -> None:
    """
    Test a statistic that cannot be called is refused at the boundary.
    """
    with pytest.raises(TypeError, match="statistic must be callable"):
        permutation_test(
            [1.0, 2.0],
            statistic=cast("object", 1.0),
            metric="not callable",
            iterations=1,
            seed=0,
        )


def test_the_record_is_flat_and_serialisable() -> None:
    """
    Test the record carries the seed, the counts and the unit without the class.
    """
    result: PermutationTestResult = permutation_test(
        [1.0, 2.0, 3.0, 4.0],
        statistic=_last,
        metric="last outcome",
        iterations=10,
        seed=1,
    )

    record = result.to_record()

    assert record["metric"] == "last outcome"
    assert record["unit"] == "trade_order"
    assert record["direction"] == "higher_is_better"
    assert record["iterations"] == 10
    assert record["observations"] == 4
    assert record["seed"] == 1
    assert record["block_size"] is None
    assert "distribution" not in record
    assert json.loads(json.dumps(record)) == record


def test_total_return_compounds_the_period_returns() -> None:
    """
    Test the compounded return is the product of one plus each return, minus one.
    """
    assert total_return([0.10, -0.05]) == pytest.approx(0.045, rel=1e-12)
    assert total_return([1.0, -0.5, 0.25]) == pytest.approx(0.25, rel=1e-12)


def test_the_ratio_is_the_mean_over_the_sample_deviation() -> None:
    """
    Test the ratio divides by the sample deviation, not the population one.
    """
    # Mean 0.0066666, sample deviation 0.0152752, ratio 0.4364357.
    assert ratio_of_mean_to_deviation([0.01, -0.01, 0.02]) == pytest.approx(0.4364357, rel=1e-6)


def test_a_series_without_dispersion_has_no_ratio() -> None:
    """
    Test a series that never varies reports no ratio rather than an infinity.
    """
    assert math.isnan(ratio_of_mean_to_deviation([0.01]))
    assert math.isnan(ratio_of_mean_to_deviation([0.01, 0.01, 0.01]))
