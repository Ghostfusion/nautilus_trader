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
Tests for multiple-testing-aware reporting: the statistical contract and the deflated Sharpe ratio.

The expected values are recomputed here by an independent route wherever the formula allows it: the
distribution function is `math.erfc`, and the quantiles are found by pure bisection, so a test does
not repeat the production quantile search it is meant to check.
"""

import math
from typing import cast

import pytest

from nautilus_trader._libnautilus.analysis import MetricDirection
from nautilus_trader._libnautilus.analysis import MetricInput
from nautilus_trader._libnautilus.analysis import MetricUnits
from nautilus_trader.analysis import MetricReason
from nautilus_trader.analysis import MetricStatus
from nautilus_trader.analysis import PortfolioAnalyzer
from nautilus_trader.optimization import CanonicalRun
from nautilus_trader.optimization import DatasetIdentity
from nautilus_trader.optimization import DeflatedSharpeRatio
from nautilus_trader.optimization import DivisorConvention
from nautilus_trader.optimization import Experiment
from nautilus_trader.optimization import FailedExperiment
from nautilus_trader.optimization import SelectionRule
from nautilus_trader.optimization import SharpeFrequency
from nautilus_trader.optimization import SharpeSample
from nautilus_trader.optimization import SignificanceResult
from nautilus_trader.optimization import SpecificationBound
from nautilus_trader.optimization import SpecificationBounds
from nautilus_trader.optimization import SpecificationExtreme
from nautilus_trader.optimization import StatisticalContract
from nautilus_trader.optimization import StudyIdentity
from nautilus_trader.optimization import TrialDependence
from nautilus_trader.optimization import TrialSpecification
from nautilus_trader.optimization import UniverseIdentity
from nautilus_trader.optimization import deflated_sharpe_bounds
from nautilus_trader.optimization import deflated_sharpe_ratio
from nautilus_trader.optimization import per_period_sharpe
from nautilus_trader.optimization import return_moments
from nautilus_trader.optimization import significance_report
from nautilus_trader.optimization import trial_provenance_from_runs


# The correction's quantile search is a safeguarded Newton iteration; the reference below is a pure
# bisection on the `math.erfc` distribution function, so it shares no step with it. The reference
# computes the same published formula, which is the point: the formula is the specification, and the
# two implementations of it must agree.
_SQRT_TWO = math.sqrt(2.0)
_EULER_MASCHERONI = 0.5772156649015329

# A contract for the cases that are about the formula rather than about the thresholds. The declared
# default thresholds are exercised on their own, at their boundary.
_FORMULA_CONTRACT = StatisticalContract(minimum_observations=12, minimum_trials=2)


def _reference_cdf(value: float) -> float:
    """
    Return the distribution function of the standard normal by `math.erfc`.
    """
    return 0.5 * math.erfc(-value / _SQRT_TWO)


def _reference_ppf(probability: float) -> float:
    """
    Return the quantile function of the standard normal by pure bisection.
    """
    lower = -40.0
    upper = 40.0

    for _ in range(200):
        middle = 0.5 * (lower + upper)
        if _reference_cdf(middle) < probability:
            lower = middle
        else:
            upper = middle

    return 0.5 * (lower + upper)


def _reference_deflated_sharpe(
    sharpe: float,
    trial_sharpes: list[float],
    observations: int,
    skew: float,
    kurtosis: float,
    counted_trials: int,
) -> float:
    """
    Return the deflated Sharpe ratio by the independent route.
    """
    mean = math.fsum(trial_sharpes) / len(trial_sharpes)
    variance = math.fsum((value - mean) ** 2 for value in trial_sharpes) / (len(trial_sharpes) - 1)
    expected_maximum = math.sqrt(variance) * (
        (1.0 - _EULER_MASCHERONI) * _reference_ppf(1.0 - 1.0 / counted_trials)
        + _EULER_MASCHERONI * _reference_ppf(1.0 - 1.0 / (counted_trials * math.e))
    )
    variance_factor = 1.0 - skew * sharpe + ((kurtosis - 1.0) / 4.0) * sharpe * sharpe
    argument = (
        (sharpe - expected_maximum) * math.sqrt(observations - 1) / math.sqrt(variance_factor)
    )

    return _reference_cdf(argument)


def _universe() -> UniverseIdentity:
    """
    Return a declared universe identity.
    """
    return UniverseIdentity(
        universe_digest="sha256:00000000000000000000000000000000000000000000000000000000000000aa",
        membership_policy_id="top_500_by_capitalisation",
        membership_as_of=1_600_000_000_000_000_000,
    )


def _dataset() -> DatasetIdentity:
    """
    Return a declared dataset identity.
    """
    return DatasetIdentity(
        dataset_digest="sha256:00000000000000000000000000000000000000000000000000000000000000bb",
        universe=_universe(),
        source_version="2026-01-01",
        as_of=1_600_000_000_000_000_000,
        calendar_identity="nautilus-trading-calendar/v1",
        adjustment_policy="raw",
        missing_data_policy="exclude",
    )


def _study() -> StudyIdentity:
    """
    Return a declared study identity.
    """
    return StudyIdentity(
        dataset=_dataset(),
        parameter_space_digest="sha256:00000000000000000000000000000000000000000000000000000000000000cc",
        objective_definition={"terms": [{"metric": "sharpe_ratio", "weight": 1.0}]},
        selection_rule=SelectionRule.RANK_FIRST_FEASIBLE,
        metric_set=("sharpe_ratio",),
        study_seed=42,
    )


def _run(period: int) -> CanonicalRun:
    """
    Return a completed run of one experiment.
    """
    experiment = Experiment({"period": period})

    return CanonicalRun(
        experiment=experiment,
        canonical_digest=f"sha256:{period:064d}",
        metric_values={"Sharpe Ratio (252 days)": 1.0},
        canonical_result=b"{}",
    )


def _failed(period: int) -> FailedExperiment:
    """
    Return a failed run of one experiment.
    """
    return FailedExperiment(Experiment({"period": period}), "RuntimeError", "no data")


def _sample(**overrides: object) -> SharpeSample:
    """
    Return a sample with the given overrides.
    """
    fields: dict[str, object] = {
        "sharpe": 1.2,
        "trial_sharpes": [0.4, 1.2, 0.8, 1.6],
        "observations": 12,
        "skew": 0.0,
        "kurtosis": 3.0,
        "dependence": TrialDependence.INDEPENDENT,
    }
    fields.update(overrides)

    return SharpeSample(**fields)  # type: ignore[arg-type]


def _specification() -> TrialSpecification:
    """
    Return a declared trial specification.
    """
    return TrialSpecification(
        data_window=(1_000, 2_000),
        universe_rule="top_500_by_capitalisation",
        weighting="value",
        adjustment_model="four_factor",
        exclusions=("financials",),
    )


def test_the_contract_declares_every_element_of_the_statistical_contract() -> None:
    """
    Test the emitted contract states the whole table rather than only the parameters.
    """
    contract = StatisticalContract()
    declared = contract.to_dict()

    for element in (
        "return_definition",
        "minimum_observations",
        "minimum_trials",
        "sharpe_convention",
        "trial_variance_estimator",
        "skew_estimator",
        "kurtosis_estimator",
        "kurtosis_convention",
        "annualisation",
        "missing_returns",
        "duplicate_trials",
        "failed_trials",
        "trial_independence",
    ):
        assert element in declared

    return_definition = cast("dict[str, object]", declared["return_definition"])
    sharpe_convention = cast("dict[str, object]", declared["sharpe_convention"])

    assert return_definition["risk_free_is_explicit"] is True
    assert return_definition["risk_free_rate"] == 0.0
    assert return_definition["compounding"] == "simple"
    assert sharpe_convention["frequency"] == "per_period"
    assert sharpe_convention["divisor"] == "sample"
    assert declared["kurtosis_convention"] == "non_excess"
    assert declared["annualisation"] == "prohibited"
    assert declared["missing_returns"] == "excluded"
    assert declared["duplicate_trials"] == "rejected"
    assert declared["trial_independence"] == "declared_by_the_study"
    assert contract.digest == StatisticalContract().digest
    assert contract.digest != StatisticalContract(minimum_trials=11).digest


def test_the_contract_refuses_thresholds_it_cannot_compute_from() -> None:
    """
    Test a contract that cannot produce a number is refused rather than recorded.
    """
    with pytest.raises(ValueError, match="minimum_observations"):
        StatisticalContract(minimum_observations=1)
    with pytest.raises(ValueError, match="minimum_trials"):
        StatisticalContract(minimum_trials=1)
    with pytest.raises(ValueError, match="risk_free_rate"):
        StatisticalContract(risk_free_rate=math.nan)
    with pytest.raises(ValueError, match="version"):
        StatisticalContract(version=0)
    with pytest.raises(ValueError, match="contract_id"):
        StatisticalContract(contract_id="")


def test_per_period_sharpe_pins_the_divisor_and_excludes_missing_returns() -> None:
    """
    Test the divisor is a declared convention and a missing return is not a zero return.
    """
    estimate = per_period_sharpe([0.01, None, math.nan, 0.02])

    assert estimate.value == pytest.approx(2.1213203435596424, rel=1e-15)
    assert estimate.observations == 2

    population = per_period_sharpe(
        [0.01, None, math.nan, 0.02], divisor=DivisorConvention.POPULATION
    )

    assert population.value == pytest.approx(3.0, rel=1e-15)
    assert population.observations == 2


def test_per_period_sharpe_is_undefined_rather_than_zero_filled() -> None:
    """
    Test a series that cannot produce a Sharpe ratio reports that rather than a number.
    """
    short = per_period_sharpe([0.01, None, math.nan])

    assert short.value is None
    assert short.observations == 1

    flat = per_period_sharpe([0.01, 0.01, 0.01])

    assert flat.value is None
    assert flat.observations == 3

    with pytest.raises(ValueError, match="finite"):
        per_period_sharpe([0.01, math.inf])


def test_per_period_sharpe_states_the_risk_free_treatment() -> None:
    """
    Test the risk-free rate is subtracted rather than assumed away.
    """
    assert per_period_sharpe({1_000: 0.02, 2_000: 0.04}).value == pytest.approx(
        2.1213203435596424,
        rel=1e-15,
    )
    assert per_period_sharpe([0.02, 0.04], risk_free_rate=0.01).value == pytest.approx(
        math.sqrt(2.0),
        rel=1e-15,
    )


def test_return_moments_pin_the_non_excess_kurtosis() -> None:
    """
    Test the moments are the population standardized moments with the non-excess convention.
    """
    moments = return_moments([-2.0, -1.0, 1.0, 2.0])

    assert moments is not None
    assert moments.skew == 0.0
    assert moments.kurtosis == pytest.approx(1.36, rel=1e-15)
    assert moments.observations == 4

    # A two-point distribution has kurtosis exactly 1 under the non-excess convention, where the
    # excess convention would give -2.
    two_point = return_moments([-1.0, 1.0])

    assert two_point is not None
    assert two_point.kurtosis == pytest.approx(1.0, rel=1e-15)

    asymmetric = return_moments([1.0, 1.0, 3.0])

    assert asymmetric is not None
    assert asymmetric.skew == pytest.approx(1.0 / math.sqrt(2.0), rel=1e-15)

    assert return_moments([1.0, 1.0, 1.0]) is None
    assert return_moments([1.0]) is None


def test_the_sample_refuses_an_excess_kurtosis_and_an_annualised_sharpe() -> None:
    """
    Test the two conventions the formula would otherwise silently reinterpret.
    """
    with pytest.raises(ValueError, match="non-excess"):
        _sample(kurtosis=0.0)
    with pytest.raises(ValueError, match="per-period"):
        _sample(frequency=SharpeFrequency.ANNUALISED)
    with pytest.raises(ValueError, match="finite"):
        _sample(sharpe=math.nan)
    with pytest.raises(ValueError, match="finite"):
        _sample(trial_sharpes=[1.0, math.inf])


def test_a_dependent_study_must_declare_its_effective_trial_count() -> None:
    """
    Test dependence is declared, and the declared count is the one the correction uses.
    """
    with pytest.raises(ValueError, match="effective trial count"):
        _sample(dependence=TrialDependence.DEPENDENT)
    with pytest.raises(ValueError, match="must not declare"):
        _sample(effective_trials=4)
    with pytest.raises(ValueError, match="exceeds the nominal"):
        _sample(dependence=TrialDependence.DEPENDENT, effective_trials=5)
    with pytest.raises(ValueError, match="positive"):
        _sample(dependence=TrialDependence.DEPENDENT, effective_trials=0)

    dependent = _sample(dependence=TrialDependence.DEPENDENT, effective_trials=4)

    assert dependent.counted_trials == 4
    assert dependent.nominal_trials == 4

    # A dependent study with an effective count equal to another study's nominal count is corrected
    # identically, because the effective count is what the expected maximum is taken over.
    counted = _sample(trial_sharpes=[0.4, 1.2, 0.8, 1.6])

    assert dependent.nominal_trials == counted.nominal_trials

    dependent_result = deflated_sharpe_ratio(dependent, contract=_FORMULA_CONTRACT)
    counted_result = deflated_sharpe_ratio(counted, contract=_FORMULA_CONTRACT)

    assert dependent_result.value == pytest.approx(counted_result.value, rel=1e-15)


def test_zero_one_and_many_trials_are_distinguished() -> None:
    """
    Test the three trial counts the contract distinguishes.
    """
    none_result = deflated_sharpe_ratio(_sample(trial_sharpes=[]), contract=_FORMULA_CONTRACT)
    one_result = deflated_sharpe_ratio(_sample(trial_sharpes=[0.8]), contract=_FORMULA_CONTRACT)
    many_result = deflated_sharpe_ratio(
        _sample(trial_sharpes=[0.4, 1.6]), contract=_FORMULA_CONTRACT
    )

    assert none_result.status is MetricStatus.UNAVAILABLE
    assert none_result.reason is not None
    assert none_result.reason == MetricReason.INSUFFICIENT_DATA
    assert none_result.value is None
    assert one_result.status is MetricStatus.UNAVAILABLE
    assert one_result.value is None
    assert many_result.is_computed
    assert many_result.value is not None


def test_the_minimum_counts_are_the_declared_boundary() -> None:
    """
    Test the declared default thresholds are the exact boundary they claim to be.
    """
    trials = [0.4, 1.6] * 5

    at_the_boundary = deflated_sharpe_ratio(_sample(trial_sharpes=trials, observations=20))
    below_observations = deflated_sharpe_ratio(_sample(trial_sharpes=trials, observations=19))
    below_trials = deflated_sharpe_ratio(_sample(trial_sharpes=[0.4, 1.6], observations=250))

    assert at_the_boundary.is_computed
    assert below_observations.status is MetricStatus.UNAVAILABLE
    assert below_observations.reason is not None
    assert below_observations.reason == MetricReason.INSUFFICIENT_DATA
    assert below_observations.value is None
    assert below_trials.status is MetricStatus.UNAVAILABLE
    assert below_trials.value is None


def test_a_degenerate_sample_deflates_to_one_half_exactly() -> None:
    """
    Test a sample with no dispersion and no Sharpe ratio has an argument of zero.
    """
    result = deflated_sharpe_ratio(
        _sample(sharpe=0.0, trial_sharpes=[0.0, 0.0, 0.0]),
        contract=_FORMULA_CONTRACT,
    )

    assert result.is_computed
    assert result.value == 0.5


def test_the_deflated_sharpe_ratio_matches_an_independent_recomputation() -> None:
    """
    Test the formula against a recomputation that shares no step with the production one.
    """
    sample = _sample()

    result = deflated_sharpe_ratio(sample, contract=_FORMULA_CONTRACT)
    reference = _reference_deflated_sharpe(
        sharpe=sample.sharpe,
        trial_sharpes=list(sample.trial_sharpes),
        observations=sample.observations,
        skew=sample.skew,
        kurtosis=sample.kurtosis,
        counted_trials=sample.counted_trials,
    )

    assert result.value == pytest.approx(reference, rel=1e-12, abs=1e-15)
    assert result.trial_variance == pytest.approx(0.26666666666666666, rel=1e-15)

    # Pinned as a literal as well, so a change to either implementation is visible as a difference
    # rather than only as an agreement between two moving parts.
    assert result.value == pytest.approx(0.9516126683175339, rel=1e-12)

    # A hand case with an exact distribution value: no dispersion and a zero Sharpe leave an
    # argument of zero, and two periods make the argument exactly the Sharpe, so the value is half
    # the complementary error function of minus a half over the square root of two.
    exact = deflated_sharpe_ratio(
        SharpeSample(
            sharpe=0.5,
            trial_sharpes=[0.5, 0.5, 0.5],
            observations=2,
            skew=0.0,
            kurtosis=1.0,
            dependence=TrialDependence.INDEPENDENT,
        ),
        contract=StatisticalContract(minimum_observations=2, minimum_trials=2),
    )

    assert exact.value == 0.5 * math.erfc(-0.5 / _SQRT_TWO)
    assert exact.value == pytest.approx(0.691462461274013, rel=1e-15)


def test_more_trials_do_not_raise_the_corrected_value() -> None:
    """
    Test the correction falls as the selection it accounts for grows.
    """
    # The two-point trial sample has the same dispersion at every length, so the only thing that
    # changes between the cases is the trial count the expected maximum is taken over.
    values: list[float] = []

    for count in (2, 4, 6, 10, 20, 100, 1000):
        result = deflated_sharpe_ratio(
            _sample(trial_sharpes=[0.4, 1.6] * (count // 2)),
            contract=_FORMULA_CONTRACT,
        )
        assert result.value is not None
        values.append(result.value)

    assert all(values[index] >= values[index + 1] for index in range(len(values) - 1))
    assert values[0] > values[-1]


def test_a_large_trial_count_is_still_computed_and_is_smaller() -> None:
    """
    Test a thousand-trial sweep is corrected rather than refused.
    """
    small = deflated_sharpe_ratio(
        _sample(trial_sharpes=[0.4, 1.6] * 5),
        contract=_FORMULA_CONTRACT,
    )
    large = deflated_sharpe_ratio(
        _sample(trial_sharpes=[0.4, 1.6] * 500),
        contract=_FORMULA_CONTRACT,
    )

    assert small.value is not None
    assert large.value is not None
    assert math.isfinite(large.value)
    assert large.value < small.value


def test_invalid_is_distinguished_from_unavailable_on_the_same_sample() -> None:
    """
    Test a defective input and a thin input are different states, not one dropped row.
    """
    invalid = deflated_sharpe_ratio(
        _sample(skew=1_000.0, kurtosis=1.0),
        contract=_FORMULA_CONTRACT,
    )
    unavailable = deflated_sharpe_ratio(
        _sample(observations=10),
        contract=_FORMULA_CONTRACT,
    )
    computed = deflated_sharpe_ratio(_sample(), contract=_FORMULA_CONTRACT)

    assert invalid.status is MetricStatus.INVALID
    assert invalid.reason is not None
    assert invalid.reason == MetricReason.UNDEFINED_RESULT
    assert invalid.value is None
    assert unavailable.status is MetricStatus.UNAVAILABLE
    assert unavailable.reason is not None
    assert unavailable.reason == MetricReason.INSUFFICIENT_DATA
    assert unavailable.value is None
    assert computed.status is MetricStatus.COMPUTED
    assert computed.reason is None
    assert computed.value is not None


def test_a_result_that_is_not_computed_carries_a_reason_and_no_value() -> None:
    """
    Test the result's status is a state rather than a label.
    """
    sample = _sample(observations=10)

    with pytest.raises(ValueError, match="computed result carries a value"):
        SignificanceResult(
            sample=sample,
            contract=StatisticalContract(),
            status=MetricStatus.COMPUTED,
            value=None,
            trial_variance=None,
        )
    with pytest.raises(ValueError, match="carries a reason and no value"):
        SignificanceResult(
            sample=sample,
            contract=StatisticalContract(),
            status=MetricStatus.UNAVAILABLE,
            value=0.5,
            trial_variance=None,
        )


def test_the_contract_is_recorded_with_the_result() -> None:
    """
    Test a number can name the rules it was computed under.
    """
    contract = StatisticalContract(minimum_observations=30, minimum_trials=4)
    result = deflated_sharpe_ratio(_sample(observations=29), contract=contract)

    assert result.contract is contract
    assert result.to_dict()["contract_digest"] == contract.digest
    assert result.to_dict()["status"] == "unavailable"
    assert result.to_dict()["reason"] == "insufficient_data"
    assert result.to_dict()["nominal_trials"] == 4
    assert result.to_dict()["counted_trials"] == 4

    changed = deflated_sharpe_ratio(
        _sample(observations=29),
        contract=StatisticalContract(minimum_observations=30, minimum_trials=5),
    )

    assert changed.to_dict()["contract_digest"] != contract.digest


def test_duplicate_parameter_sets_are_detected_rather_than_counted_twice() -> None:
    """
    Test two runs of one experiment are one trial, not two.
    """
    with pytest.raises(ValueError, match="duplicate parameter sets"):
        trial_provenance_from_runs([_run(10), _run(10), _run(12)])

    provenance = trial_provenance_from_runs([_run(10), _run(12), _failed(14)])

    assert provenance.trial_count == 3
    assert provenance.failed_trial_count == 1
    assert provenance.nominal_and_effective is False

    dependent = trial_provenance_from_runs(
        [_run(10), _run(12), _failed(14)],
        dependence=TrialDependence.DEPENDENT,
        effective_trials=2,
    )

    assert dependent.nominal_and_effective is True
    assert dependent.trial_count == 3


def test_the_report_records_the_study_the_trials_the_provenance_and_the_correction() -> None:
    """
    Test the study-level record carries everything a reader needs, and digests stably.
    """
    study = _study()
    runs = [_run(10), _run(12), _failed(14)]
    report = significance_report(
        study,
        runs,
        _sample(trial_sharpes=[0.4, 1.6]),
        contract=_FORMULA_CONTRACT,
    )

    assert report.study_id == study.study_id
    assert len(report.trials) == 3
    assert report.provenance.trial_count == 3
    assert report.provenance.failed_trial_count == 1
    assert report.result.is_computed
    assert report.provenance.trial_count - report.provenance.failed_trial_count == 2
    assert report.to_dict()["study_id"] == study.study_id
    assert len(cast("list[object]", report.to_dict()["trial_ids"])) == 3

    identical = significance_report(
        study,
        runs,
        _sample(trial_sharpes=[0.4, 1.6]),
        contract=_FORMULA_CONTRACT,
    )
    changed = significance_report(
        study,
        runs,
        _sample(trial_sharpes=[0.4, 1.7]),
        contract=_FORMULA_CONTRACT,
    )

    assert report.digest == identical.digest
    assert report.digest != changed.digest


def test_the_report_refuses_a_sample_that_does_not_match_the_runs() -> None:
    """
    Test the dispersion cannot be drawn from trials that were not run.
    """
    with pytest.raises(ValueError, match="trial estimates"):
        significance_report(
            _study(),
            [_run(10), _run(12), _failed(14)],
            _sample(),
            contract=_FORMULA_CONTRACT,
        )


def test_the_estimation_chain_is_computable_from_a_return_series() -> None:
    """
    Test the whole chain, from a return series with gaps to a reported correction.
    """
    returns = [0.004, -0.002, None, 0.006, 0.001, math.nan, 0.003, -0.001]

    estimate = per_period_sharpe(returns)
    moments = return_moments(returns)

    assert estimate.value is not None
    assert moments is not None
    assert estimate.observations == moments.observations == 6

    result = deflated_sharpe_ratio(
        SharpeSample(
            sharpe=estimate.value,
            trial_sharpes=[estimate.value, estimate.value * 0.9, estimate.value * 1.1, 0.0],
            observations=estimate.observations,
            skew=moments.skew,
            kurtosis=moments.kurtosis,
            dependence=TrialDependence.INDEPENDENT,
        ),
        contract=StatisticalContract(minimum_observations=6, minimum_trials=4),
    )

    assert result.is_computed
    assert result.value is not None
    assert 0.0 <= result.value <= 1.0


def _statistic_trials() -> list[float]:
    """
    Return the trial Sharpe estimates a sweep produced, above the declared minimum trial count.
    """
    return [0.2, 0.5, 0.8, 1.0, 0.75, 0.6, 0.9, 1.2, 0.35, 0.55, 0.7, 1.1]


def _statistic_returns() -> dict[int, float]:
    """
    Return a deterministic return series long enough for the declared minimum observation count.
    """
    values = [
        0.012,
        -0.004,
        0.007,
        0.002,
        -0.006,
        0.011,
        0.003,
        -0.002,
        0.005,
        0.009,
        0.001,
        0.004,
        0.008,
        -0.003,
        0.006,
        0.002,
        -0.001,
        0.010,
        0.003,
        -0.005,
        0.007,
        0.001,
        0.004,
        0.006,
    ]

    return {
        1_600_000_000_000_000_000 + index * 86_400_000_000_000: value
        for index, value in enumerate(values)
    }


def test_the_deflated_sharpe_statistic_matches_an_independent_recomputation() -> None:
    """
    Test the statistic corrects the run's own returns by the declared trial set.
    """
    returns = _statistic_returns()
    statistic = DeflatedSharpeRatio(_statistic_trials())

    estimate = per_period_sharpe(returns)
    moments = return_moments(returns)
    assert estimate.value is not None
    assert moments is not None

    expected = _reference_deflated_sharpe(
        estimate.value,
        _statistic_trials(),
        estimate.observations,
        moments.skew,
        moments.kurtosis,
        len(_statistic_trials()),
    )

    assert statistic.calculate_from_returns(returns) == pytest.approx(expected)


def test_the_deflated_sharpe_statistic_states_the_trial_counts_it_used() -> None:
    """
    Test the row names the nominal and effective counts, which have no measurable unit.
    """
    independent = DeflatedSharpeRatio(_statistic_trials())
    dependent = DeflatedSharpeRatio(
        _statistic_trials(),
        dependence=TrialDependence.DEPENDENT,
        effective_trials=2,
    )

    assert independent.name == "Deflated Sharpe Ratio (12 trials)"
    assert dependent.name == "Deflated Sharpe Ratio (12 trials, 2 effective)"
    assert dependent.metric_id == independent.metric_id == "deflated_sharpe_ratio"
    assert independent.inputs == (MetricInput.RETURNS,)
    assert independent.units is MetricUnits.RATIO
    assert independent.direction is MetricDirection.MAXIMIZE


def test_the_sample_records_the_specification_it_was_found_under() -> None:
    """
    Test the row's declared parameters carry the specification, not only a trial count.
    """
    specification = _specification()
    sample = _sample(specification=specification)

    assert sample.to_dict()["specification"] == specification.to_dict()
    assert sample.to_dict()["effective_trials"] is None


def test_the_deflated_sharpe_statistic_names_the_specification_it_was_found_under() -> None:
    """
    Test the row names its provenance rather than leaving the reader to guess the search.
    """
    specification = _specification()
    statistic = DeflatedSharpeRatio(_statistic_trials(), specification=specification)

    assert statistic.specification == specification
    assert statistic.name == "Deflated Sharpe Ratio (12 trials, value, window 1000-2000)"

    # A statistic without a declared specification keeps the row it had before.
    assert DeflatedSharpeRatio(_statistic_trials()).name == "Deflated Sharpe Ratio (12 trials)"


def test_a_sample_refuses_a_specification_of_the_wrong_type() -> None:
    """
    Test the declaration is validated when the sample is built.
    """
    with pytest.raises(TypeError, match="specification must be a TrialSpecification"):
        _sample(specification="value")


def test_the_deflated_sharpe_statistic_refuses_an_incomplete_declaration() -> None:
    """
    Test the declaration is validated when the statistic is built, not when it is calculated.
    """
    with pytest.raises(ValueError, match="dependent trials must declare"):
        DeflatedSharpeRatio(_statistic_trials(), dependence=TrialDependence.DEPENDENT)

    with pytest.raises(ValueError, match="independent trials must not declare"):
        DeflatedSharpeRatio(_statistic_trials(), effective_trials=2)

    with pytest.raises(ValueError, match="must be finite"):
        DeflatedSharpeRatio([0.5, float("nan")])


def test_the_deflated_sharpe_statistic_is_reported_beside_the_run_statistics() -> None:
    """
    Test a registered statistic reaches the analyzer's return statistics by its own name.
    """
    returns = _statistic_returns()
    statistic = DeflatedSharpeRatio(_statistic_trials())

    analyzer = PortfolioAnalyzer()
    analyzer.register_statistic(statistic)
    for timestamp, value in returns.items():
        analyzer.add_return(timestamp, value)

    value = statistic.calculate_from_returns(returns)
    assert value is not None
    assert analyzer.get_performance_stats_returns() == {statistic.name: pytest.approx(value)}


def test_the_deflated_sharpe_statistic_is_unavailable_rather_than_zero() -> None:
    """
    Test a series the correction cannot compute from reports nothing rather than a zero.
    """
    statistic = DeflatedSharpeRatio(_statistic_trials())

    # One period cannot produce a Sharpe ratio.
    assert statistic.calculate_from_returns({1_600_000_000_000_000_000: 0.01}) is None

    # A flat series has no dispersion, so there is no Sharpe ratio to correct.
    flat = {1_600_000_000_000_000_000 + index: 0.0 for index in range(24)}

    assert statistic.calculate_from_returns(flat) is None

    # Below the default contract's minimum observation count the correction is not computed.
    short = {
        1_600_000_000_000_000_000 + index * 86_400_000_000_000: value
        for index, value in enumerate([0.012, -0.004, 0.007, 0.002, 0.005, 0.001])
    }

    assert statistic.calculate_from_returns(short) is None


def _equal_weighted_specification() -> TrialSpecification:
    """
    Return a second declared specification, distinct from `_specification`.
    """
    return TrialSpecification(
        data_window=(1_000, 2_000),
        universe_rule="top_500_by_capitalisation",
        weighting="equal",
    )


def test_the_bound_is_reported_at_both_specification_extremes() -> None:
    """
    Test a bound that differs across the declared specifications is reported as both, named.
    """
    equal = _equal_weighted_specification()
    value_weighted = _specification()

    # The specifications read the same search differently: the value-weighted, four-factor reading
    # deflates the selected estimate further, which is the least favourable reading of the two.
    favourable = _sample(specification=equal)
    unfavourable = _sample(specification=value_weighted, sharpe=0.6)

    bounds = deflated_sharpe_bounds([favourable, unfavourable], contract=_FORMULA_CONTRACT)

    assert bounds.most_favourable.value == pytest.approx(0.9516126683175339, rel=1e-12)
    assert bounds.least_favourable.value < bounds.most_favourable.value

    # Each bound names the specification that produced it, and states which extreme it is.
    assert bounds.most_favourable.extreme is SpecificationExtreme.MOST_FAVOURABLE
    assert bounds.most_favourable.specification == equal
    assert bounds.least_favourable.extreme is SpecificationExtreme.LEAST_FAVOURABLE
    assert bounds.least_favourable.specification == value_weighted

    emitted = bounds.to_dict()
    assert emitted["most_favourable"]["extreme"] == "most_favourable"
    assert emitted["most_favourable"]["specification_label"] == equal.label
    assert emitted["least_favourable"]["extreme"] == "least_favourable"
    assert emitted["least_favourable"]["specification_label"] == value_weighted.label
    assert emitted["most_favourable"]["specification"] == equal.to_dict()
    assert (
        bounds.digest
        == deflated_sharpe_bounds(
            [favourable, unfavourable],
            contract=_FORMULA_CONTRACT,
        ).digest
    )


def test_a_single_bound_report_is_refused() -> None:
    """
    Test one bound is not a comparison and is refused rather than defaulted.
    """
    bound = SpecificationBound(
        extreme=SpecificationExtreme.MOST_FAVOURABLE,
        specification=_equal_weighted_specification(),
        value=0.5,
    )

    with pytest.raises(ValueError, match="must carry both specification extremes"):
        SpecificationBounds.from_bounds([bound])

    with pytest.raises(ValueError, match="must carry both specification extremes"):
        SpecificationBounds.from_bounds([bound, bound])


def test_a_search_that_declared_one_specification_could_not_be_checked() -> None:
    """
    Test a single declared specification is refused rather than printed as one figure.
    """
    specification = _equal_weighted_specification()

    with pytest.raises(ValueError, match="could not be checked"):
        deflated_sharpe_bounds(
            [_sample(specification=specification)],
            contract=_FORMULA_CONTRACT,
        )

    # Two samples that record one specification are still one declared specification.
    with pytest.raises(ValueError, match="one bound per declared specification"):
        deflated_sharpe_bounds(
            [
                _sample(specification=specification),
                _sample(specification=specification, sharpe=0.6),
            ],
            contract=_FORMULA_CONTRACT,
        )

    # A bound whose specification is unnamed cannot be compared.
    with pytest.raises(ValueError, match="must name the specification"):
        deflated_sharpe_bounds(
            [_sample(specification=specification), _sample()],
            contract=_FORMULA_CONTRACT,
        )


def test_coincident_extremes_read_as_two_equal_bounds_naming_one_specification() -> None:
    """
    Test a search that varied nothing that moves the bound is not read as one bound.
    """
    first = _equal_weighted_specification()
    second = TrialSpecification(
        data_window=(1_000, 2_000),
        universe_rule="all_listed",
        weighting="equal",
    )

    bounds = deflated_sharpe_bounds(
        [_sample(specification=first), _sample(specification=second)],
        contract=_FORMULA_CONTRACT,
    )

    assert bounds.most_favourable.value == bounds.least_favourable.value
    assert bounds.most_favourable.specification == first
    assert bounds.least_favourable.specification == first
    assert bounds.most_favourable.extreme is not bounds.least_favourable.extreme


def test_a_bound_pair_refuses_an_unordered_or_mislabelled_pair() -> None:
    """
    Test the pair states which extreme is which rather than relying on its order.
    """
    low = SpecificationBound(
        extreme=SpecificationExtreme.MOST_FAVOURABLE,
        specification=_equal_weighted_specification(),
        value=0.2,
    )
    high = SpecificationBound(
        extreme=SpecificationExtreme.LEAST_FAVOURABLE,
        specification=_specification(),
        value=0.8,
    )

    with pytest.raises(ValueError, match="must not be below"):
        SpecificationBounds(most_favourable=low, least_favourable=high)

    labelled_most = SpecificationBound(
        extreme=SpecificationExtreme.MOST_FAVOURABLE,
        specification=_specification(),
        value=0.8,
    )

    with pytest.raises(ValueError, match="must be labelled least favourable"):
        SpecificationBounds(most_favourable=labelled_most, least_favourable=labelled_most)
