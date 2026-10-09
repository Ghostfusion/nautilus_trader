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
Permutation tests of a backtest result: whether the order of the outcomes was doing the work.

A backtest reports what a rule would have done on one ordering of history. Part of that result comes
from the rule and part from the ordering: a rule that buys after a rise looks better in a history
that rose early. A permutation test keeps every observed outcome and throws away only the order it
arrived in. The statistic is recomputed on each rearrangement, and the share of rearrangements that
did at least as well as the real one is the p-value. The null the share speaks about is that the
sequence carries nothing beyond the values in it.

Two units of rearrangement are supported, and the difference between them is what the null assumes:

- `ShuffleUnit.TRADE_ORDER` permutes the realized outcomes of the trades themselves, so the
  assumption is that any ordering of the outcome multiset was equally available. It is the test for
  "did this rule's sequence of trades do better than the same trades in another order".
- `ShuffleUnit.RETURN_BLOCK` permutes blocks of consecutive per-period returns, keeping the order
  within each block. Blocks preserve short-horizon structure - a run of calm days or of turbulence
  stays together - so the test is less harsh on a rule that trades persistence than a single-period
  shuffle would be, and the block size states how much structure was preserved.

Three boundaries are enforced rather than documented:

- **Prices are never reconstructed.** The implementation this module answers shuffles the four daily
  prices of a bar independently and rebuilds the bar from them, which produces days whose high is
  below their close: values that were never observed, compared against a rule that reads them. Here
  the unit shuffled is an outcome or a block of outcomes, so every rearranged series is a
  rearrangement of values that were observed, and no rearranged value can be one that was not.
- **The random generator is local.** The seed is carried by the result and a generator is created
  inside the call, so two runs with the same seed agree, and a run cannot be perturbed by anything
  else that drew random numbers first.
- **Refusals are counted, not hidden.** A rearrangement the statistic cannot score (a non-finite
  value) is counted in `refused` and excluded from the denominator, and the count is reported with
  the p-value, so a share computed from fewer runs than were asked for is visible.

The result is a report, never a gate: nothing here is consulted by a strategy, an order or a risk
check, and a small p-value is not a statement that a rule pays. The default thresholds this module
carries state when a share is worth printing rather than what to do about it.
"""

from __future__ import annotations

import math
import random
from dataclasses import dataclass
from enum import Enum
from typing import TYPE_CHECKING


if TYPE_CHECKING:
    from collections.abc import Callable
    from collections.abc import Iterator
    from collections.abc import Sequence


# The stable identity of the procedure this module implements, and its version. A change to any rule
# declared in the module docstring changes the version, so a recorded result names the rules it was
# produced under.
PERMUTATION_CONTRACT_ID = "permutation_test"
PERMUTATION_CONTRACT_VERSION = 1
PERMUTATION_RECORD_VERSION = 1

# The default number of rearrangements. It is a declared parameter rather than a statistical
# constant: a share computed from fewer rearrangements is coarser, and the count travels with the
# result so a reader can see which was used.
DEFAULT_ITERATIONS = 1000

# The fewest outcomes a rearrangement is defined over: one outcome has no other order to be compared
# against. It is a structural floor, not an evidence threshold, which is why the number of outcomes
# is reported with every result instead of being gated.
MINIMUM_OUTCOMES = 2


class ShuffleUnit(Enum):
    """
    What a rearrangement moves.
    """

    TRADE_ORDER = "trade_order"
    RETURN_BLOCK = "return_block"


class TestDirection(Enum):
    """
    Which rearrangements count as doing at least as well as the real one.
    """

    HIGHER_IS_BETTER = "higher_is_better"
    LOWER_IS_BETTER = "lower_is_better"
    TWO_SIDED = "two_sided"


@dataclass(frozen=True)
class PermutationTestResult:
    """
    The outcome of one permutation test.

    Parameters
    ----------
    metric : str
        The name of the statistic that was shuffled against.
    unit : ShuffleUnit
        What the rearrangement moved.
    direction : TestDirection
        Which rearrangements were counted as at least as good as the real one.
    observed : float
        The statistic computed on the real sequence.
    extreme_count : int
        How many scored rearrangements were at least as good as the real one.
    iterations : int
        How many rearrangements were asked for.
    refused : int
        How many rearrangements the statistic could not score. They are excluded from the share.
    p_value : float
        `extreme_count` divided by the scored rearrangements, or NaN when none could be scored.
    observations : int
        How many outcomes were rearranged. A share from a short sequence is a coarse share, which is
        why the count travels with it.
    seed : int
        The seed that produced the rearrangements, so the run can be repeated exactly.
    block_size : int or None
        The block size for a block rearrangement, or None for a single-outcome rearrangement.
    distribution : tuple of float
        Every statistic produced by the scored rearrangements, in the order they were drawn.

    """

    metric: str
    unit: ShuffleUnit
    direction: TestDirection
    observed: float
    extreme_count: int
    iterations: int
    refused: int
    p_value: float
    observations: int
    seed: int
    block_size: int | None
    distribution: tuple[float, ...]

    @property
    def scored(self) -> int:
        """
        Return how many rearrangements the statistic could score.
        """
        return self.iterations - self.refused

    def to_record(self) -> dict[str, object]:
        """
        Return a flat, JSON-serialisable record of the test.

        The record is flat and versioned so that it can be stored beside a result and read back
        without the class, and it states the seed, the counts and the unit, because a p-value
        without them cannot be checked.
        """
        return {
            "record_version": PERMUTATION_RECORD_VERSION,
            "contract": PERMUTATION_CONTRACT_ID,
            "contract_version": PERMUTATION_CONTRACT_VERSION,
            "metric": self.metric,
            "unit": self.unit.value,
            "direction": self.direction.value,
            "observed": self.observed,
            "extreme_count": self.extreme_count,
            "iterations": self.iterations,
            "refused": self.refused,
            "p_value": self.p_value,
            "observations": self.observations,
            "seed": self.seed,
            "block_size": self.block_size,
        }


def _outcomes(values: Sequence[float]) -> list[float]:
    """
    Return the outcomes to be rearranged, refusing anything that cannot be scored.

    A permutation test is defined over the observed values, so a missing value is a gap in the
    multiset the null speaks about and an infinity cannot be compared: both are refused rather than
    dropped, because dropping one changes the population being rearranged.
    """
    outcomes = [float(value) for value in values]

    if len(outcomes) < MINIMUM_OUTCOMES:
        raise ValueError(
            f"a permutation test needs at least {MINIMUM_OUTCOMES} outcomes to rearrange, "
            f"was given {len(outcomes)}"
        )

    for index, value in enumerate(outcomes):
        if not math.isfinite(value):
            raise ValueError(f"outcome at index {index} is {value!r}; every outcome must be finite")

    return outcomes


def _block_size(values: Sequence[float], block_size: int | None) -> int:
    """
    Return the block size to move, or 1 when the rearrangement moves single outcomes.
    """
    if block_size is None:
        return 1

    if not isinstance(block_size, int) or isinstance(block_size, bool):
        raise TypeError(f"block_size must be an int or None, was {type(block_size).__name__}")

    if block_size < 1:
        raise ValueError(f"block_size must be at least 1, was {block_size}")

    if block_size >= len(values):
        raise ValueError(
            f"block_size {block_size} is not smaller than the {len(values)} outcomes; "
            "a rearrangement that cannot move anything has no null to speak about"
        )

    return block_size


def shuffled_orders(
    values: Sequence[float],
    *,
    iterations: int = DEFAULT_ITERATIONS,
    seed: int = 0,
    block_size: int | None = None,
) -> Iterator[list[float]]:
    """
    Yield rearrangements of the given outcomes.

    Parameters
    ----------
    values : Sequence[float]
        The outcomes to rearrange, such as the realized PnL of each trade or the return of each
        period.
    iterations : int, default 1000
        How many rearrangements to yield.
    seed : int, default 0
        The seed of the generator. A generator local to this call is used, so the same seed yields
        the same rearrangements whatever else in the process draws random numbers.
    block_size : int, optional
        The number of consecutive outcomes to move as one block. When supplied it must be at least
        1 and smaller than the number of outcomes. Outcomes beyond the last whole block are left in
        place, so the multiset is preserved exactly and no outcome is dropped or duplicated.

    Yields
    ------
    list[float]
        A rearrangement of `values`.

    Raises
    ------
    ValueError
        If fewer than two outcomes are given, an outcome is not finite, `iterations` is less than
        one, or `block_size` is outside its range.
    TypeError
        If `block_size` is neither an int nor None.

    Examples
    --------
    >>> from nautilus_trader.optimization.permutation import shuffled_orders
    >>> orders = list(shuffled_orders([1.0, 2.0, 3.0], iterations=1, seed=7))
    >>> sorted(orders[0]) == [1.0, 2.0, 3.0]
    True

    """
    outcomes = _outcomes(values)

    if not isinstance(iterations, int) or isinstance(iterations, bool):
        raise TypeError(f"iterations must be an int, was {type(iterations).__name__}")

    if iterations < 1:
        raise ValueError(f"iterations must be at least 1, was {iterations}")

    size = _block_size(outcomes, block_size)
    rng = random.Random(seed)  # noqa: S311 (a deterministic permutation RNG, not security)

    # The whole-block prefix is rearranged; a trailing part-block is left in place. Moving a partial
    # block would either drop outcomes or shear the adjacency the block exists to preserve.
    prefix = len(outcomes) - (len(outcomes) % size)
    positions = list(range(0, prefix, size))

    for _ in range(iterations):
        rng.shuffle(positions)
        order: list[float] = []

        for start in positions:
            order.extend(outcomes[start : start + size])

        order.extend(outcomes[prefix:])
        yield order


def permutation_test(  # noqa: PLR0913 - the test's declared inputs
    values: Sequence[float],
    *,
    statistic: Callable[[Sequence[float]], float],
    metric: str,
    iterations: int = DEFAULT_ITERATIONS,
    seed: int = 0,
    direction: TestDirection = TestDirection.HIGHER_IS_BETTER,
    block_size: int | None = None,
) -> PermutationTestResult:
    """
    Test whether the order of the given outcomes was doing the work.

    Parameters
    ----------
    values : Sequence[float]
        The outcomes to rearrange, such as the realized PnL of each trade or the return of each
        period.
    statistic : Callable[[Sequence[float]], float]
        The statistic to recompute on each rearrangement. It takes a sequence of outcomes and
        returns one number. The same callable is applied to the real sequence, so the two are
        always comparable.
    metric : str
        The name of the statistic, recorded with the result.
    iterations : int, default 1000
        How many rearrangements to score.
    seed : int, default 0
        The seed of the generator, recorded with the result.
    direction : TestDirection, default TestDirection.HIGHER_IS_BETTER
        Which rearrangements count as at least as good as the real one. A metric that is better when
        smaller, such as the deepest fall from a peak or the volatility of returns, needs
        `LOWER_IS_BETTER`; counting it the other way inflates the p-value and makes a rule look
        worse than it is. `TWO_SIDED` compares magnitudes and is only meaningful where the sign of
        the statistic is the direction of the effect.
    block_size : int, optional
        The number of consecutive outcomes to move as one block, for a rearrangement that keeps
        short-horizon structure together. See `shuffled_orders`.

    Returns
    -------
    PermutationTestResult
        The observed statistic, the counted rearrangements and the p-value.

    Raises
    ------
    ValueError
        If the outcomes, the iteration count or the block size are not usable, or if the statistic
        is not finite on the real sequence.
    TypeError
        If `statistic` is not callable.

    Notes
    -----
    The p-value is `extreme_count / scored`, and it is never exactly the probability that the
    strategy loses money. A p-value of zero means no rearrangement out of the ones drawn did that
    well, not that none could.

    Examples
    --------
    >>> from nautilus_trader.optimization.permutation import permutation_test
    >>> result = permutation_test(
    ...     [1.0, 2.0, 3.0, 4.0],
    ...     statistic=max,
    ...     metric="largest outcome",
    ...     iterations=100,
    ...     seed=3,
    ... )
    >>> result.p_value
    1.0

    """
    if not callable(statistic):
        raise TypeError(f"statistic must be callable, was {type(statistic).__name__}")

    unit = ShuffleUnit.TRADE_ORDER if block_size is None else ShuffleUnit.RETURN_BLOCK
    observed = float(statistic(values))

    if not math.isfinite(observed):
        raise ValueError(
            f"the statistic {observed!r} on the real sequence is not finite; "
            "a test needs a value to compare rearrangements against"
        )

    extreme = 0
    refused = 0
    distribution: list[float] = []

    for order in shuffled_orders(
        values,
        iterations=iterations,
        seed=seed,
        block_size=block_size,
    ):
        value = float(statistic(order))

        if not math.isfinite(value):
            refused += 1
            continue

        distribution.append(value)

        if _is_extreme(value, observed, direction):
            extreme += 1

    scored = len(distribution)
    p_value = extreme / scored if scored else math.nan

    return PermutationTestResult(
        metric=metric,
        unit=unit,
        direction=direction,
        observed=observed,
        extreme_count=extreme,
        iterations=iterations,
        refused=refused,
        p_value=p_value,
        observations=len(values),
        seed=seed,
        block_size=block_size,
        distribution=tuple(distribution),
    )


def _is_extreme(value: float, observed: float, direction: TestDirection) -> bool:
    """
    Return whether a rearranged statistic is at least as good as the observed one.
    """
    if direction is TestDirection.HIGHER_IS_BETTER:
        return value >= observed

    if direction is TestDirection.LOWER_IS_BETTER:
        return value <= observed

    return abs(value) >= abs(observed)


def total_return(values: Sequence[float]) -> float:
    """
    Return the compounded return of a series of period returns.

    Parameters
    ----------
    values : Sequence[float]
        The return of each period, as a fraction, not annualised.

    Returns
    -------
    float
        The compounded return: the product of one plus each return, minus one.

    Notes
    -----
    Compounding is not commutative in floating-point arithmetic, so a rearrangement of the same
    returns can differ from the original in the last bits. That is a property of the arithmetic, not
    of the arrangement: a comparison against a tolerance, or an exactly representable series, is
    what keeps a test of an order-invariant statistic from reading noise.

    Examples
    --------
    >>> from nautilus_trader.optimization.permutation import total_return
    >>> round(total_return([0.10, -0.05]), 6)
    0.045

    """
    growth = 1.0

    for value in values:
        growth *= 1.0 + value

    return growth - 1.0


def ratio_of_mean_to_deviation(values: Sequence[float]) -> float:
    """
    Return the mean of the values divided by their sample deviation.

    Parameters
    ----------
    values : Sequence[float]
        The values, such as the return of each period.

    Returns
    -------
    float
        The mean divided by the sample standard deviation computed with the number of values minus
        one as the divisor, or NaN when fewer than two values are given or every value is the same.
        A series that never varies has no dispersion to divide by, and reporting a substituted
        number for it would compare two different statistics in one distribution.

    Notes
    -----
    This is the per-period ratio, not an annualised one. Annualising it inside a permutation test
    would scale every rearrangement by the same constant, which leaves the count of extreme
    rearrangements unchanged and the p-value identical: the scaling is allowed to happen outside the
    test, where a reader can see it.

    Examples
    --------
    >>> from nautilus_trader.optimization.permutation import ratio_of_mean_to_deviation
    >>> round(ratio_of_mean_to_deviation([0.01, -0.01, 0.02]), 6)
    0.436436

    """
    if len(values) < MINIMUM_OUTCOMES:
        return math.nan

    count = len(values)
    mean = math.fsum(values) / count
    variance = math.fsum((value - mean) ** 2 for value in values) / (count - 1)
    deviation = math.sqrt(variance)

    if deviation == 0.0:
        return math.nan

    return mean / deviation
