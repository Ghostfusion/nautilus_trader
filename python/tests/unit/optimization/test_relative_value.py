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
Tests for the relative-value declarations and the refusals they answer with.

Each declaration is asserted against the contract it composes with rather than on its own: the
screen's count is the trial count the correction consumes, the signal's reach is what a leakage
policy covers, and the persistence floor is the floor below which no half-life is reported. The
half-life conversions and the settled memory of a recursion are recomputed here from their formulas
in the test, so a test does not repeat the production expression it is meant to check.
"""

import math

import pytest

from nautilus_trader.analysis import MetricStatus
from nautilus_trader.optimization import AlignmentConvention
from nautilus_trader.optimization import LabelDefinition
from nautilus_trader.optimization import LabelKind
from nautilus_trader.optimization import LeakagePolicy
from nautilus_trader.optimization import PersistenceConvention
from nautilus_trader.optimization import PersistenceEstimate
from nautilus_trader.optimization import RecursiveMemory
from nautilus_trader.optimization import ResearchCapabilityCode
from nautilus_trader.optimization import ScreenFamily
from nautilus_trader.optimization import ScreenOutcome
from nautilus_trader.optimization import SharpeSample
from nautilus_trader.optimization import SignalWindow
from nautilus_trader.optimization import TrialDependence
from nautilus_trader.optimization import gapped_range_capability
from nautilus_trader.optimization import history_capability
from nautilus_trader.optimization import label_series
from nautilus_trader.optimization import leakage_capability
from nautilus_trader.optimization import pair_distinguishability_capability
from nautilus_trader.optimization import screen_family_capability


DAILY_NS = 86_400_000_000_000

# A spread of eleven sector series, which is the family the source's screen enumerates.
SECTORS = tuple(f"SECTOR{index}" for index in range(11))

# Twelve trial estimates, so the correction has more trials than the contract's declared minimum.
TRIAL_SHARPES = tuple(0.01 * index for index in range(1, 13))


def daily(closes: list[float]) -> list[tuple[int, float]]:
    """
    Return a bar series of one observation per day from the given closes.
    """
    return [(index * DAILY_NS, close) for index, close in enumerate(closes)]


def forward(alignment: AlignmentConvention) -> LabelDefinition:
    """
    Return a two-observation forward label under the given alignment.
    """
    return LabelDefinition(
        label_id="spread",
        kind=LabelKind.FORWARD_RETURN,
        horizon=2,
        alignment=alignment,
    )


def window(alignment: AlignmentConvention) -> SignalWindow:
    """
    Return a signal window whose only declared difference is its alignment.
    """
    return SignalWindow(
        fit_start=-6,
        fit_length=4,
        measurement_start=-2,
        measurement_length=2,
        label=forward(alignment),
    )


def sample(trials: int) -> SharpeSample:
    """
    Return a correction sample carrying the given number of trial estimates.
    """
    return SharpeSample(
        sharpe=0.12,
        trial_sharpes=TRIAL_SHARPES[:trials],
        observations=252,
        skew=-0.4,
        kurtosis=3.1,
        dependence=TrialDependence.INDEPENDENT,
    )


def reference_memory(state_noise: float, observation_noise: float) -> tuple[float, float, float]:
    """
    Return the settled level, gain and memory of a scalar recursion, from its own formulas.
    """
    level = 0.5 * (
        state_noise + math.sqrt(state_noise * state_noise + 4.0 * state_noise * observation_noise)
    )
    gain = level / (level + observation_noise)

    return level, gain, 1.0 / gain


def test_a_screen_reports_the_family_it_enumerated_before_its_gates() -> None:
    """
    Test the family size is the enumerated count rather than the surviving one.
    """
    family = ScreenFamily(members=SECTORS)
    pairs = family.pairs()

    assert family.size == 55
    assert len(pairs) == 55
    assert pairs[0] == ("SECTOR0", "SECTOR1")
    assert pairs[-1] == ("SECTOR9", "SECTOR10")

    outcome = ScreenOutcome(family=family, evaluated=pairs[:12], survivors=pairs[:3])

    assert outcome.family_size == 55
    assert outcome.evaluated_count == 12
    assert outcome.survivor_count == 3
    assert outcome.family.capability.is_available


def test_an_undeclared_universe_is_answered_rather_than_counted() -> None:
    """
    Test a screen that declares no members answers with the correction's own code.
    """
    undeclared = ScreenFamily()

    assert undeclared.size == 0
    assert undeclared.pairs() == ()

    refused = undeclared.capability

    assert not refused.is_available
    assert refused.code == ResearchCapabilityCode.EFFECTIVE_TRIALS_UNDECLARED.value
    assert refused.requirements == ["declare the screen universe"]


def test_a_family_outside_the_declared_bounds_is_refused_with_the_bound() -> None:
    """
    Test the family size is checked against the maximum it declares and the pairs it requires.
    """
    bounded = ScreenFamily(members=SECTORS, maximum_family=20)
    refused = bounded.capability

    assert refused.code == ResearchCapabilityCode.EFFECTIVE_TRIALS_OUT_OF_RANGE.value
    assert refused.requirements == ["a family of at most 20 pairs"]

    seven = tuple(f"SECTOR{index}" for index in range(7))
    assert ScreenFamily(members=seven, maximum_family=21).capability.is_available

    small = screen_family_capability(4, required_pairs=10)

    assert small.code == ResearchCapabilityCode.UNIVERSE_TOO_SMALL.value
    assert small.requirements == ["a universe enumerating at least 10 pairs"]
    assert screen_family_capability(5, required_pairs=10).is_available


def test_a_screened_result_recomputes_the_correction_from_the_recorded_family() -> None:
    """
    Test the correction is computed over the evaluated tests the record holds.
    """
    family = ScreenFamily(members=SECTORS)
    outcome = ScreenOutcome(
        family=family, evaluated=family.pairs()[:12], survivors=family.pairs()[:2]
    )

    result = outcome.significance(sample(12))

    assert result.status is MetricStatus.COMPUTED
    assert result.value is not None

    with pytest.raises(ValueError, match="evaluated 12 tests"):
        outcome.significance(sample(11))


def test_a_screen_record_only_holds_tests_its_family_enumerated() -> None:
    """
    Test a record cannot claim a test outside its family, or a survivor it never evaluated.
    """
    family = ScreenFamily(members=SECTORS)
    pairs = family.pairs()

    with pytest.raises(ValueError, match="not pairs of the family"):
        ScreenOutcome(family=family, evaluated=(("SECTOR0", "OTHER"),))

    with pytest.raises(ValueError, match="survivors were not evaluated"):
        ScreenOutcome(family=family, evaluated=(pairs[0],), survivors=(pairs[1],))


def test_a_persistence_fit_reports_the_convention_it_converts_under() -> None:
    """
    Test the two conventions convert one fit into different half-lives.
    """
    coefficient = -0.05
    discrete = PersistenceEstimate(coefficient=coefficient, observations=252)
    continuous = PersistenceEstimate(
        coefficient=coefficient,
        observations=252,
        convention=PersistenceConvention.CONTINUOUS,
    )

    assert discrete.autoregressive_coefficient == pytest.approx(0.95)
    assert discrete.half_life == pytest.approx(math.log(0.5) / math.log(0.95), rel=1e-12)
    assert continuous.half_life == pytest.approx(-math.log(2.0) / coefficient, rel=1e-12)

    assert discrete.half_life == pytest.approx(13.513, abs=1e-3)
    assert continuous.half_life == pytest.approx(13.863, abs=1e-3)
    assert discrete.half_life != continuous.half_life
    assert discrete.to_dict()["convention"] == "discrete"
    assert continuous.to_dict()["convention"] == "continuous"


def test_a_fit_below_the_declared_floor_reports_the_periods_still_required() -> None:
    """
    Test a fit shorter than the floor answers with a capability rather than a half-life.
    """
    estimate = PersistenceEstimate(coefficient=-0.05, observations=12)
    refused = estimate.capability

    assert estimate.half_life is None
    assert not refused.is_available
    assert refused.code == ResearchCapabilityCode.INSUFFICIENT_OBSERVATIONS.value
    assert refused.requirements == ["8 more observations"]

    declared = PersistenceEstimate(coefficient=-0.05, observations=12, minimum_observations=10)

    assert declared.capability.is_available
    assert declared.half_life is not None


def test_a_fit_that_does_not_revert_is_a_distinct_answer_from_a_short_one() -> None:
    """
    Test a non-negative or overshooting coefficient is refused by a different code.
    """
    for coefficient in (0.0, 0.02, -1.0, -1.5):
        refused = PersistenceEstimate(coefficient=coefficient, observations=252).capability

        assert refused.code == ResearchCapabilityCode.NOT_MEAN_REVERTING.value
        assert refused.code != ResearchCapabilityCode.INSUFFICIENT_OBSERVATIONS.value
        assert PersistenceEstimate(coefficient=coefficient, observations=252).half_life is None


def test_the_alignment_convention_moves_the_measured_reach() -> None:
    """
    Test two signals that declare the same windows but different alignment reach differently.
    """
    bars = daily([100.0, 101.0, 102.0, 101.5, 103.0, 102.0, 104.0, 103.0, 105.0, 104.0])

    signal = window(AlignmentConvention.SIGNAL_BAR)
    next_bar = window(AlignmentConvention.NEXT_BAR)

    assert signal.reach_ns(bars) == 2 * DAILY_NS
    assert next_bar.reach_ns(bars) == 3 * DAILY_NS
    assert signal.digest != next_bar.digest


def test_a_leakage_policy_shorter_than_the_reach_is_refused_with_the_shortfall() -> None:
    """
    Test the reach a signal reports is the coverage its leakage policy has to provide.
    """
    bars = daily([100.0, 101.0, 102.0, 101.5, 103.0, 102.0, 104.0, 103.0, 105.0, 104.0])
    signal = window(AlignmentConvention.SIGNAL_BAR)
    short = LeakagePolicy(purge_before=DAILY_NS, zero_interval_justification="test")
    covering = LeakagePolicy(purge_before=2 * DAILY_NS, zero_interval_justification="test")
    series = label_series(signal.label, bars)

    refused = leakage_capability(series, short)

    assert refused.code == ResearchCapabilityCode.LEAKAGE_NOT_COVERED.value
    assert refused.requirements == [f"cover {DAILY_NS} ns more before the evaluation set"]
    assert leakage_capability(series, covering).is_available
    assert series.forward_reach_ns == signal.reach_ns(bars)


def test_a_fit_window_that_does_not_precede_the_measurement_window_is_refused() -> None:
    """
    Test a declaration that measures over the observations it fitted on is a construction error.
    """
    with pytest.raises(ValueError, match="does not precede it"):
        SignalWindow(
            fit_start=-4,
            fit_length=4,
            measurement_start=-2,
            measurement_length=2,
            label=forward(AlignmentConvention.SIGNAL_BAR),
        )

    with pytest.raises(ValueError, match="after the decision bar"):
        SignalWindow(
            fit_start=-8,
            fit_length=4,
            measurement_start=-1,
            measurement_length=2,
            label=forward(AlignmentConvention.SIGNAL_BAR),
        )

    with pytest.raises(ValueError, match="measurement_length must be at least 2"):
        SignalWindow(
            fit_start=-6,
            fit_length=4,
            measurement_start=-1,
            measurement_length=1,
            label=forward(AlignmentConvention.SIGNAL_BAR),
        )


def test_the_declared_memory_is_the_memory_its_calibration_settles_at() -> None:
    """
    Test the settled gain and memory are the ones the state and observation noise imply.
    """
    calibration = (
        (1e-4, 1.0, 9.95e-3, 100.5),
        (1e-4, 0.01, 9.51e-2, 10.5),
        (1e-5, 1.0, 3.16e-3, 316.7),
        (1e-3, 1.0, 3.11e-2, 32.1),
    )

    for state_noise, observation_noise, documented_gain, documented_memory in calibration:
        _, gain, memory = reference_memory(state_noise, observation_noise)
        declaration = RecursiveMemory(
            state_noise=state_noise,
            observation_noise=observation_noise,
            declared_memory=memory,
            warm_up=math.ceil(memory),
        )

        assert declaration.converged_gain == pytest.approx(gain, rel=1e-12)
        assert declaration.effective_memory == pytest.approx(memory, rel=1e-12)
        assert gain == pytest.approx(documented_gain, rel=2e-3)
        assert memory == pytest.approx(documented_memory, rel=2e-3)


def test_a_declared_memory_the_recursion_contradicts_is_refused() -> None:
    """
    Test varying one noise parameter cannot leave the declared memory standing.
    """
    _, _, memory = reference_memory(1e-4, 1.0)

    with pytest.raises(ValueError, match="is not the"):
        RecursiveMemory(
            state_noise=1e-4,
            observation_noise=1.0,
            declared_memory=memory / 10.0,
            warm_up=math.ceil(memory),
        )

    with pytest.raises(ValueError, match="is not the"):
        RecursiveMemory(
            state_noise=1e-4,
            observation_noise=0.01,
            declared_memory=memory,
            warm_up=math.ceil(memory),
        )


def test_a_warm_up_shorter_than_the_settled_memory_is_refused_naming_both() -> None:
    """
    Test a warm-up inside the transient is refused with both numbers in the answer.
    """
    _, _, memory = reference_memory(1e-4, 1.0)

    with pytest.raises(
        ValueError,
        match=r"warm-up of 30 observations is shorter than the settled memory of \d+\.\d observations",
    ):
        RecursiveMemory(
            state_noise=1e-4,
            observation_noise=1.0,
            declared_memory=memory,
            warm_up=30,
        )

    settled = RecursiveMemory(
        state_noise=1e-4,
        observation_noise=1.0,
        declared_memory=memory,
        warm_up=math.ceil(memory),
    )

    assert settled.digest == settled.digest
    assert (
        settled.digest
        != RecursiveMemory(
            state_noise=1e-4,
            observation_noise=1.0,
            declared_memory=memory,
            warm_up=math.ceil(memory) + 1,
        ).digest
    )


def test_a_gapped_range_is_refused_with_the_observations_it_is_missing() -> None:
    """
    Test a range below its declared cadence answers with the missing observations.
    """
    refused = gapped_range_capability((0, 100, 300), start=0, end=400, stride=100)

    assert refused.code == ResearchCapabilityCode.GAPPED_RANGE.value
    assert refused.requirements == ["1 more observation between 0 and 400"]
    assert gapped_range_capability((0, 100, 200, 300), start=0, end=400, stride=100).is_available
    assert gapped_range_capability((0,), start=0, end=400, stride=100).requirements == [
        "3 more observations between 0 and 400",
    ]

    with pytest.raises(ValueError, match="outside the range"):
        gapped_range_capability((0, 400), start=0, end=400, stride=100)

    with pytest.raises(ValueError, match="strictly increase"):
        gapped_range_capability((100, 100), start=0, end=400, stride=100)


def test_a_series_that_begins_after_the_window_is_refused_with_the_shortfall() -> None:
    """
    Test a history shorter than the window answers with the nanoseconds it is short by.
    """
    refused = history_capability(5 * DAILY_NS, required_start=3 * DAILY_NS)

    assert refused.code == ResearchCapabilityCode.HISTORY_AFTER_WINDOW.value
    assert refused.requirements == [f"{2 * DAILY_NS} ns of earlier history"]
    assert history_capability(3 * DAILY_NS, required_start=3 * DAILY_NS).is_available


def test_a_pair_that_is_indistinguishable_is_refused_before_it_is_tested() -> None:
    """
    Test a series that is another series up to scale and shift is not a pair.
    """
    left = [1.0, 2.0, 3.0, 4.0, 5.0]
    copy = [2.0 * value + 10.0 for value in left]

    refused = pair_distinguishability_capability(left, copy, tolerance=1e-6)

    assert refused.code == ResearchCapabilityCode.PAIR_INDISTINGUISHABLE.value
    assert refused.requirements == ["a pair separated by more than 1e-06"]
    assert pair_distinguishability_capability(
        left, [5.0, 1.0, 4.0, 2.0, 3.0], tolerance=1e-6
    ).is_available

    with pytest.raises(ValueError, match="must share its rows"):
        pair_distinguishability_capability(left, left[:3], tolerance=1e-6)

    with pytest.raises(ValueError, match="no dispersion"):
        pair_distinguishability_capability(left, [3.0] * 5, tolerance=1e-6)


def test_every_refusal_carries_the_requirement_it_did_not_meet() -> None:
    """
    Test the refusals report a canonical code and an actionable requirement, never only a detail.
    """
    refusals = (
        ScreenFamily().capability,
        screen_family_capability(11, maximum_family=20),
        PersistenceEstimate(coefficient=-0.05, observations=3).capability,
        PersistenceEstimate(coefficient=0.02, observations=252).capability,
        gapped_range_capability((0,), start=0, end=300, stride=100),
        history_capability(DAILY_NS, required_start=0),
        pair_distinguishability_capability([1.0, 2.0, 3.0], [2.0, 4.0, 6.0], tolerance=1e-6),
    )

    for refusal in refusals:
        assert not refusal.is_available
        assert refusal.code in {code.value for code in ResearchCapabilityCode}
        assert refusal.requirements
