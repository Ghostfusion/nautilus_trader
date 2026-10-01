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
Domain-scoped capability answers for research requests.

A capability answers whether a request can be served before the work is done, as data rather than as
an exception, so a caller can plan around a refusal instead of catching one. The shape is shared and
is `nautilus_trader.core.Capability`; the code sets are not, and this module owns the research one.

The code is canonical and the detail is not. A caller may branch on the code and must never branch
on the detail, which exists for a human; the requirements list what a caller can act on, in
nanoseconds, counts or words as the domain measures them. Every probe here is a pure function of its
inputs, so it is cheap to call before doing the work it predicts, and each one answers the question
the contract it consults would otherwise raise about.
"""

from __future__ import annotations

import math
from enum import Enum
from enum import unique
from typing import TYPE_CHECKING

from nautilus_trader.core import Capability
from nautilus_trader.optimization.significance import DEFAULT_MINIMUM_OBSERVATIONS
from nautilus_trader.optimization.significance import StatisticalContract
from nautilus_trader.optimization.significance import TrialDependence


if TYPE_CHECKING:
    from collections.abc import Sequence

    from nautilus_trader.optimization.labels import LabelSeries
    from nautilus_trader.optimization.splits import LeakagePolicy
    from nautilus_trader.optimization.splits import SplitContract


# A standardised series needs a dispersion to scale by, which two observations is the fewest that
# can define. A shorter series is a construction to refuse rather than a capability to answer.
_PAIR_MINIMUM_OBSERVATIONS = 2


@unique
class ResearchCapabilityCode(Enum):
    """
    Why a research request cannot be served.

    The set is closed and belongs to this domain: every code states the constraint that failed, so
    a caller can act on it and a reader can enumerate what the research surface refuses.
    """

    LEAKAGE_NOT_COVERED = "LEAKAGE_NOT_COVERED"
    INSUFFICIENT_OBSERVATIONS = "INSUFFICIENT_OBSERVATIONS"
    INSUFFICIENT_TRIALS = "INSUFFICIENT_TRIALS"
    EFFECTIVE_TRIALS_UNDECLARED = "EFFECTIVE_TRIALS_UNDECLARED"
    EFFECTIVE_TRIALS_UNEXPECTED = "EFFECTIVE_TRIALS_UNEXPECTED"
    EFFECTIVE_TRIALS_OUT_OF_RANGE = "EFFECTIVE_TRIALS_OUT_OF_RANGE"
    SPLIT_LAYOUT_UNSATISFIABLE = "SPLIT_LAYOUT_UNSATISFIABLE"
    NOT_MEAN_REVERTING = "NOT_MEAN_REVERTING"
    GAPPED_RANGE = "GAPPED_RANGE"
    HISTORY_AFTER_WINDOW = "HISTORY_AFTER_WINDOW"
    PAIR_INDISTINGUISHABLE = "PAIR_INDISTINGUISHABLE"
    UNIVERSE_TOO_SMALL = "UNIVERSE_TOO_SMALL"


def leakage_capability(series: LabelSeries, policy: LeakagePolicy) -> Capability:
    """
    Answer whether a leakage policy covers a label's forward reach.

    Parameters
    ----------
    series : LabelSeries
        The label series whose reach is the requirement.
    policy : LeakagePolicy
        The exclusion relation the dataset would apply.

    Returns
    -------
    Capability
        Available when the policy's effective purge covers the reach, and otherwise the shortfall in
        the requirements.

    """
    shortfall = series.leakage_shortfall_ns(policy)
    if shortfall == 0:
        return Capability.available()

    definition = series.definition
    code = ResearchCapabilityCode.LEAKAGE_NOT_COVERED.value

    return Capability.unavailable(
        code,
        f"the label {definition.label_id!r} reaches {series.forward_reach_ns} ns beyond its "
        f"feature row while the leakage policy purges {policy.purge} ns",
    ).requiring(f"cover {shortfall} ns more before the evaluation set")


def _declaration_capability(
    dependence: object,
    effective_trials: int | None,
    trials: int,
) -> Capability | None:
    """
    Answer whether a dependence declaration is complete, or None when it is.

    The dependence is read as an object rather than a `TrialDependence` so that the check below is
    a real branch: a declaration of the wrong type must not be read as an independent one.

    Raises
    ------
    TypeError
        If the dependence is not a `TrialDependence`.

    """
    if not isinstance(dependence, TrialDependence):
        raise TypeError(f"dependence must be a TrialDependence, was {dependence!r}")

    if dependence is not TrialDependence.DEPENDENT:
        if effective_trials is None:
            return None

        code = ResearchCapabilityCode.EFFECTIVE_TRIALS_UNEXPECTED.value

        return Capability.unavailable(
            code,
            "the study declares independent trials, so the nominal count is the effective one",
        ).requiring("remove the effective trial count or declare the dependence")

    if effective_trials is None:
        code = ResearchCapabilityCode.EFFECTIVE_TRIALS_UNDECLARED.value

        return Capability.unavailable(
            code,
            "the study declares dependent trials and the correction cannot infer the effective "
            "count from the values",
        ).requiring("declare the effective trial count")

    if effective_trials < 1 or effective_trials > trials:
        code = ResearchCapabilityCode.EFFECTIVE_TRIALS_OUT_OF_RANGE.value

        return Capability.unavailable(
            code,
            f"the effective trial count {effective_trials} is outside 1..{trials}",
        ).requiring(f"an effective count of at least 1 and at most {trials}")

    return None


def significance_capability(
    observations: int,
    trials: int,
    *,
    dependence: TrialDependence = TrialDependence.INDEPENDENT,
    effective_trials: int | None = None,
    contract: StatisticalContract | None = None,
) -> Capability:
    """
    Answer whether a multiple-testing correction can be computed for a study.

    The constraints are the sample's own, checked in the order they are reported: the contract
    minimums first, and the dependence declaration after them.

    Parameters
    ----------
    observations : int
        The contributing periods of the selected estimate.
    trials : int
        The number of trial estimates the study produced.
    dependence : TrialDependence, default TrialDependence.INDEPENDENT
        Whether the trials are independent draws.
    effective_trials : int | None, default None
        The effective trial count, required by a dependent study and refused by an independent one.
    contract : StatisticalContract | None, default None
        The statistical contract whose minimums apply.

    Returns
    -------
    Capability
        Available when the correction has what it needs.

    """
    limits = contract or StatisticalContract()

    if observations < limits.minimum_observations:
        code = ResearchCapabilityCode.INSUFFICIENT_OBSERVATIONS.value
        missing = limits.minimum_observations - observations

        return Capability.unavailable(
            code,
            f"the correction needs {limits.minimum_observations} contributing periods and the "
            f"study has {observations}",
        ).requiring(f"{missing} more contributing periods")

    if trials < limits.minimum_trials:
        code = ResearchCapabilityCode.INSUFFICIENT_TRIALS.value
        missing = limits.minimum_trials - trials

        return Capability.unavailable(
            code,
            f"the correction needs {limits.minimum_trials} trial estimates and the study has "
            f"{trials}",
        ).requiring(f"{missing} more trial estimates")

    refusal = _declaration_capability(dependence, effective_trials, trials)

    return refusal if refusal is not None else Capability.available()


def split_capability(contract: SplitContract, *, start: int, end: int) -> Capability:
    """
    Answer whether a split contract is representable over a period.

    The contract's own layout is what decides, so the probe asks it rather than predicting it: the
    refusal it raises becomes the detail, and no caller reads a reason out of that text.

    Parameters
    ----------
    contract : SplitContract
        The split contract to lay out.
    start : int
        The period start in nanoseconds.
    end : int
        The period end in nanoseconds.

    Returns
    -------
    Capability
        Available when the contract yields its splits over the period.

    """
    try:
        contract.split(start, end)
    except ValueError as exc:
        code = ResearchCapabilityCode.SPLIT_LAYOUT_UNSATISFIABLE.value
        declared = contract.n_splits

        return Capability.unavailable(
            code,
            f"the contract is not representable over {start}..{end}: {exc}",
        ).requiring(
            "a layout that fits the period"
            if declared is None
            else f"a layout of {declared} split(s) inside the period",
        )

    return Capability.available()


def persistence_capability(
    coefficient: float,
    observations: int,
    *,
    minimum_observations: int = DEFAULT_MINIMUM_OBSERVATIONS,
) -> Capability:
    """
    Answer whether a mean-reversion fit supports a half-life.

    The coefficient is the fitted coefficient on the lagged level of the spread, so a spread that
    reverts gives a negative one. The floor is reported first, because below it the estimate is
    biased toward reversion rather than merely noisy, and a fit outside the revertible interval is
    a different answer from a fit that is simply too short.

    A coefficient of zero means the level does not enter the first difference at all and a positive
    one means the spread departs further each observation, so both are refused as not
    mean-reverting; so is a coefficient at or below minus one, whose implied autoregressive
    coefficient is not positive and whose discrete half-life therefore does not exist.

    Parameters
    ----------
    coefficient : float
        The fitted coefficient on the lagged level of the spread.
    observations : int
        The number of observations the fit used.
    minimum_observations : int, default DEFAULT_MINIMUM_OBSERVATIONS
        The declared floor below which the fit is not reported.

    Returns
    -------
    Capability
        Available when the fit is long enough and inside the revertible interval.

    """
    if observations < minimum_observations:
        code = ResearchCapabilityCode.INSUFFICIENT_OBSERVATIONS.value
        missing = minimum_observations - observations

        return Capability.unavailable(
            code,
            f"a persistence estimate needs {minimum_observations} observations and the fit used "
            f"{observations}",
        ).requiring(f"{missing} more observations")

    if coefficient >= 0.0 or coefficient <= -1.0:
        code = ResearchCapabilityCode.NOT_MEAN_REVERTING.value

        return Capability.unavailable(
            code,
            f"the fitted coefficient {coefficient!r} on the lagged level is outside the revertible "
            "interval (-1, 0)",
        ).requiring("a fit whose level coefficient is negative and above minus one")

    return Capability.available()


def screen_family_capability(
    members: int,
    *,
    required_pairs: int | None = None,
    maximum_family: int | None = None,
) -> Capability:
    """
    Answer whether a screen over a declared universe enumerates a usable trial family.

    The family is counted before any gate runs, because the correction needs the number of tests the
    screen started from rather than the number that survived it. An undeclared universe is answered
    rather than refused at construction, so a screen that never stated its members cannot report a
    family; a universe too small for the requested family, and a family above the declared maximum,
    are the two bounds on the size.

    Parameters
    ----------
    members : int
        The number of declared series in the universe.
    required_pairs : int | None, default None
        The number of pairs the screen requires, or None when it states no floor.
    maximum_family : int | None, default None
        The largest family the screen is allowed to enumerate, or None when it states no bound.

    Returns
    -------
    Capability
        Available when the universe is declared and the family is inside both bounds.

    """
    if members < 1:
        code = ResearchCapabilityCode.EFFECTIVE_TRIALS_UNDECLARED.value

        return Capability.unavailable(
            code,
            "the screen declares no universe, so the correction cannot count its trial family",
        ).requiring("declare the screen universe")

    family = members * (members - 1) // 2

    if required_pairs is not None and family < required_pairs:
        code = ResearchCapabilityCode.UNIVERSE_TOO_SMALL.value

        return Capability.unavailable(
            code,
            f"the universe of {members} series enumerates {family} pairs and the screen requires "
            f"{required_pairs}",
        ).requiring(f"a universe enumerating at least {required_pairs} pairs")

    if maximum_family is not None and family > maximum_family:
        code = ResearchCapabilityCode.EFFECTIVE_TRIALS_OUT_OF_RANGE.value

        return Capability.unavailable(
            code,
            f"the universe of {members} series enumerates {family} pairs, above the declared "
            f"maximum of {maximum_family}",
        ).requiring(f"a family of at most {maximum_family} pairs")

    return Capability.available()


def gapped_range_capability(
    observed: Sequence[int],
    *,
    start: int,
    end: int,
    stride: int,
) -> Capability:
    """
    Answer whether a range carries the cadence it declares without gaps.

    The observations are the timestamps inside the half-open range, and the declared cadence is the
    spacing they are expected at. A range with fewer observations than its cadence accounts for has
    a gap of that many observations, which is a missing sample rather than a shorter range: a test
    that silently drops a series with a gap reports the survivors as the universe.

    Parameters
    ----------
    observed : Sequence[int]
        The observation timestamps in the range, strictly increasing.
    start : int
        The range start in nanoseconds, inclusive.
    end : int
        The range end in nanoseconds, exclusive.
    stride : int
        The declared cadence in nanoseconds, positive.

    Returns
    -------
    Capability
        Available when the range carries at least the observations its cadence expects.

    Raises
    ------
    ValueError
        If the cadence or the range is not positive, or an observation is outside the range or
        does not increase.

    """
    if stride <= 0:
        raise ValueError(f"stride must be positive, was {stride}")
    if end <= start:
        raise ValueError(f"the range must be positive, was {start}..{end}")

    carried = 0
    previous: int | None = None

    for timestamp in observed:
        if timestamp < start or timestamp >= end:
            raise ValueError(f"observation {timestamp} is outside the range {start}..{end}")
        if previous is not None and timestamp <= previous:
            raise ValueError(
                f"observations must strictly increase, was {previous} then {timestamp}"
            )

        previous = timestamp
        carried += 1

    expected = -(-(end - start) // stride)
    if carried >= expected:
        return Capability.available()

    code = ResearchCapabilityCode.GAPPED_RANGE.value
    missing = expected - carried
    unit = "observation" if missing == 1 else "observations"

    return Capability.unavailable(
        code,
        f"the range {start}..{end} carries {carried} observations where a cadence of {stride} ns "
        f"expects {expected}",
    ).requiring(f"{missing} more {unit} between {start} and {end}")


def history_capability(first_observation: int, *, required_start: int) -> Capability:
    """
    Answer whether a series begins early enough for the window that reads it.

    A series whose history begins inside the window is shorter than the window rather than
    approximately it, and the number of nanoseconds it is short by is the requirement.

    Parameters
    ----------
    first_observation : int
        The first observation timestamp of the series, in nanoseconds.
    required_start : int
        The window start the series has to reach, in nanoseconds.

    Returns
    -------
    Capability
        Available when the series begins at or before the window start.

    """
    if first_observation <= required_start:
        return Capability.available()

    code = ResearchCapabilityCode.HISTORY_AFTER_WINDOW.value
    shortfall = first_observation - required_start

    return Capability.unavailable(
        code,
        f"the series begins at {first_observation} while the window starts at {required_start}",
    ).requiring(f"{shortfall} ns of earlier history")


def _standardised(values: Sequence[float], where: str) -> tuple[float, ...]:
    """
    Return the values centred on their mean and scaled by their sample dispersion.

    Raises
    ------
    ValueError
        If the series is shorter than two observations, carries a non-finite value, or has no
        dispersion to scale by.

    """
    if len(values) < _PAIR_MINIMUM_OBSERVATIONS:
        raise ValueError(
            f"{where} needs at least {_PAIR_MINIMUM_OBSERVATIONS} observations, was {len(values)}",
        )

    for value in values:
        if not math.isfinite(value):
            raise ValueError(f"{where} carries a non-finite value, was {value!r}")

    mean = math.fsum(values) / len(values)
    squared = math.fsum((value - mean) ** 2 for value in values)
    dispersion = math.sqrt(squared / (len(values) - 1))
    if dispersion == 0.0:
        raise ValueError(f"{where} has no dispersion, so it cannot be standardised")

    return tuple((value - mean) / dispersion for value in values)


def pair_distinguishability_capability(
    left: Sequence[float],
    right: Sequence[float],
    *,
    tolerance: float,
) -> Capability:
    """
    Answer whether two series are distinguishable rather than one series up to scale and shift.

    The separation is the dispersion of the difference of the two standardised series, so an exact
    affine copy of one series by the other has zero separation and a near-collinear pair lands
    inside the tolerance. That is the input a cointegration test reports as an infinite statistic
    and a p-value of zero, so a pair with no separation of its own is refused before it is tested.

    Parameters
    ----------
    left : Sequence[float]
        The observations of the first series.
    right : Sequence[float]
        The observations of the second series, over the same rows.
    tolerance : float
        The separation at or below which the two series are not distinguished, non-negative.

    Returns
    -------
    Capability
        Available when the two series are separated by more than the tolerance.

    Raises
    ------
    ValueError
        If the series are of different lengths, either is shorter than two observations, either
        carries a non-finite value or no dispersion, or the tolerance is negative.

    """
    if len(left) != len(right):
        raise ValueError(
            f"a pair must share its rows, was {len(left)} and {len(right)} observations",
        )
    if not math.isfinite(tolerance) or tolerance < 0.0:
        raise ValueError(f"tolerance must be finite and non-negative, was {tolerance!r}")

    left_standard = _standardised(left, "the left series")
    right_standard = _standardised(right, "the right series")
    differences = [a - b for a, b in zip(left_standard, right_standard, strict=True)]
    mean = math.fsum(differences) / len(differences)
    squared = math.fsum((value - mean) ** 2 for value in differences)
    separation = math.sqrt(squared / (len(differences) - 1))

    if separation > tolerance:
        return Capability.available()

    code = ResearchCapabilityCode.PAIR_INDISTINGUISHABLE.value

    return Capability.unavailable(
        code,
        f"the two series are separated by {separation!r} standard deviations, at or below the "
        f"declared tolerance of {tolerance!r}",
    ).requiring(f"a pair separated by more than {tolerance}")
