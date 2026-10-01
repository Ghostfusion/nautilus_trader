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

from enum import Enum
from enum import unique
from typing import TYPE_CHECKING

from nautilus_trader.core import Capability
from nautilus_trader.optimization.significance import StatisticalContract
from nautilus_trader.optimization.significance import TrialDependence


if TYPE_CHECKING:
    from nautilus_trader.optimization.labels import LabelSeries
    from nautilus_trader.optimization.splits import LeakagePolicy
    from nautilus_trader.optimization.splits import SplitContract


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
