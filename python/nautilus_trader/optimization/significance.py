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
Multiple-testing-aware research reporting: a specified contract and the deflated Sharpe ratio.

A sweep reports the best of many trials. That maximum is a selection, not an estimate, and a
correction is only as good as the provenance it is drawn from: the count, the space and the
individual trials. The study and trial identities carry that provenance, and this module carries
the correction, so a report can state what a result's Sharpe ratio is worth once the search that
produced it is accounted for.

The statistical contract is *specified here rather than left to the implementation*: the return
definition, the risk-free treatment, the minimum observation and trial counts, the Sharpe
convention and its divisor, the estimators and the non-excess kurtosis convention, the prohibition
on annualised inputs, the missing-return rule, the treatment of failed and duplicate trials, and
the requirement that a study declare whether its trials are independent. `StatisticalContract`
declares all of them, carries an identity and a digest, and is recorded with every result, so a
number can name the rules it was computed under.

Three boundaries are enforced rather than documented:

- **Annualised inputs are refused.** The built-in
  `Sharpe Ratio (simple, sample, 252 days)` statistic is annualised and tagged `Annualised`;
  feeding it to a correction that works per period would silently divide it and produce a number
  nobody could audit. `SharpeFrequency` exists so the declaration is explicit and the annualised
  case is an error at the boundary.
- **The kurtosis convention is non-excess.** `kurtosis` below 1 is impossible for any distribution,
  so a caller passing an excess kurtosis (0 for normal returns) is refused rather than silently
  given a different statistic than the one asked for.
- **Dependence is declared, not assumed.** A study whose trials are a sweep over adjacent parameters
  is not a study with independent trials, and the correction states which count it used. A dependent
  study must supply its effective trial count; an independent study may not.

A run's report carries the correction too: `DeflatedSharpeRatio` is a portfolio statistic that takes
the trial declaration and computes the corrected value from the returns a backtest feeds it, so the
value is printed beside the run's own statistics with the trial counts stated in the row's name.
Nothing in a single run carries the search that produced it, so the declaration stays explicit.

The value is reported, never a gate: this module has no decision authority, and nothing here is
consulted by a strategy, an order or a risk check. The correction is deliberately outside the
compiled kernels, in agreement with the analysis statistics: it is a study-level statistic, not a
run-level one.
"""

from __future__ import annotations

import math
from collections.abc import Mapping
from dataclasses import dataclass
from dataclasses import replace
from enum import Enum
from typing import TYPE_CHECKING
from typing import cast

from nautilus_trader._libnautilus.analysis import MetricDirection
from nautilus_trader._libnautilus.analysis import MetricInput
from nautilus_trader._libnautilus.analysis import MetricTag
from nautilus_trader._libnautilus.analysis import MetricUnits
from nautilus_trader.analysis import MetricReason
from nautilus_trader.analysis import MetricStatus
from nautilus_trader.analysis import PortfolioStatistic
from nautilus_trader.optimization.identity import TrialIdentity
from nautilus_trader.optimization.identity import TrialProvenance
from nautilus_trader.optimization.identity import trial_identity
from nautilus_trader.optimization.runner import FailedExperiment
from nautilus_trader.optimization.space import digest_of


if TYPE_CHECKING:
    from collections.abc import Sequence

    from nautilus_trader.optimization.identity import StudyIdentity
    from nautilus_trader.optimization.runner import CanonicalRun
    from nautilus_trader.optimization.space import JsonValue


# The stable identity of the statistical contract this module implements, and its version. A change
# to any rule the contract declares changes the version, so a historical result names the rules it
# was produced under.
SIGNIFICANCE_CONTRACT_ID = "deflated_sharpe_ratio"
SIGNIFICANCE_CONTRACT_VERSION = 1

# The declared default thresholds. They are declared parameters rather than statistical constants:
# a study that declares its own minimums states why, and a study below them reports `unavailable`
# rather than a number the sample cannot support.
DEFAULT_MINIMUM_OBSERVATIONS = 20
DEFAULT_MINIMUM_TRIALS = 10

# The rules that have no alternative in this contract, declared once so the emitted record states
# every element of the contract rather than only the parameterised ones.
KURTOSIS_CONVENTION_NON_EXCESS = "non_excess"
ANNUALISATION_RULE_PROHIBITED = "prohibited"
MISSING_RETURNS_RULE_EXCLUDED = "excluded"
DUPLICATE_TRIALS_RULE_REJECTED = "rejected"
FAILED_TRIALS_RULE_COUNTED = "counted_and_identified"
TRIAL_INDEPENDENCE_RULE_DECLARED = "declared_by_the_study"

# The Euler-Mascheroni constant, which appears in the expected maximum of a set of independent
# standard normal draws.
EULER_MASCHERONI = 0.5772156649015329

_SQRT_TWO = math.sqrt(2.0)
_SQRT_TWO_PI = math.sqrt(2.0 * math.pi)

# A standard deviation, a variance and a standardized moment all need two observations. The
# separations and the squared deviations are summed with a compensated sum, so the estimates here
# do not lose precision on a long series.
_MINIMUM_MOMENT_OBSERVATIONS = 2

# The vocabulary names, so a record states a status and a reason as text. The binding's enumeration
# exposes its members as values without a name or a value string, so the mapping is explicit: a
# change to the vocabulary fails here rather than silently emitting a repr.
_STATUS_NAMES: dict[MetricStatus, str] = {
    MetricStatus.COMPUTED: "computed",
    MetricStatus.UNAVAILABLE: "unavailable",
    MetricStatus.INVALID: "invalid",
    MetricStatus.NOT_REGISTERED: "not_registered",
}

_REASON_NAMES: dict[MetricReason, str] = {
    MetricReason.INSUFFICIENT_DATA: "insufficient_data",
    MetricReason.UNSUPPORTED_INPUT: "unsupported_input",
    MetricReason.MISSING_BENCHMARK: "missing_benchmark",
    MetricReason.NON_FINITE_INPUT: "non_finite_input",
    MetricReason.UNDEFINED_RESULT: "undefined_result",
    MetricReason.UNRESOLVED_CURRENCY: "unresolved_currency",
    MetricReason.NOT_IN_METRIC_SET: "not_in_metric_set",
}

# The bracket width at which the quantile search stops refining. It is far below the precision any
# probability needs and above the point where the bracket would stop shrinking.
_QUANTILE_TOLERANCE = 1e-16


class ReturnCompounding(Enum):
    """
    How a return series was compounded before it reached the Sharpe ratio.
    """

    SIMPLE = "simple"
    LOG = "log"


class DivisorConvention(Enum):
    """
    The divisor a dispersion estimate uses.
    """

    SAMPLE = "sample"
    POPULATION = "population"


class SharpeFrequency(Enum):
    """
    The period a Sharpe ratio is expressed over.

    `ANNUALISED` exists so a caller's declaration is refused rather than silently rescaled: the
    correction is defined per period, and an annualised value divided by an assumed number of
    periods per year would be a number no reader could audit.
    """

    PER_PERIOD = "per_period"
    ANNUALISED = "annualised"


class TrialDependence(Enum):
    """
    Whether a study's trials are independent draws from the search space.
    """

    INDEPENDENT = "independent"
    DEPENDENT = "dependent"


def _digest(payload: Mapping[str, object]) -> str:
    """
    Return the digest of a JSON-serialisable payload.

    `digest_of` is typed for scalar values, and the payloads here are nested mappings and
    sequences, which `canonical_json` serializes exactly as JSON. The cast states that expectation
    rather than widening the shared parameter type.
    """
    return digest_of(cast("Mapping[str, JsonValue]", payload))


def _normal_cdf(value: float) -> float:
    """
    Return the standard normal cumulative distribution function.

    The complement of `math.erfc` is used rather than a series, so accuracy does not degrade in the
    upper tail where the correction's quantiles live.
    """
    return 0.5 * math.erfc(-value / _SQRT_TWO)


def _normal_pdf(value: float) -> float:
    """
    Return the standard normal probability density function.
    """
    return math.exp(-0.5 * value * value) / _SQRT_TWO_PI


def _normal_ppf(probability: float) -> float:
    """
    Return the standard normal quantile function.

    The root of `cdf(x) - p` is found by Newton's method on a maintained bracket, falling back to
    bisection whenever a step would leave it. The bracket keeps the far tail safe: there the density
    underflows and an unguarded Newton step would diverge.
    """
    if not 0.0 < probability < 1.0:
        raise ValueError(f"probability must be in (0, 1), was {probability}")

    lower = -40.0
    upper = 40.0
    value = 0.0

    for _ in range(300):
        error = _normal_cdf(value) - probability
        if error < 0.0:
            lower = value
        else:
            upper = value

        density = _normal_pdf(value)
        candidate = value - error / density if density > 0.0 else 0.5 * (lower + upper)
        if not lower < candidate < upper:
            candidate = 0.5 * (lower + upper)
        if candidate == value or upper - lower < _QUANTILE_TOLERANCE:
            return candidate

        value = candidate

    return value


def _finite_values(values: Sequence[float | None]) -> list[float]:
    """
    Return the contributing values: `None` and NaN are missing, an infinity is a defect.

    Missing returns are excluded rather than zero-filled, because a zero return is a real
    observation and a missing one is not, and the horizon must count only the periods that
    contributed.
    """
    contributing: list[float] = []

    for value in values:
        if value is None:
            continue
        if math.isnan(value):
            continue
        if math.isinf(value):
            raise ValueError("a return must be finite: an infinity is a defect, not a gap")
        contributing.append(float(value))

    return contributing


def _variance(values: Sequence[float], divisor: DivisorConvention) -> float:
    """
    Return the sample or population variance of the given values.

    The two-pass form is used rather than a sum of squares: subtracting the mean first keeps the
    precision that a long series of similar values would otherwise lose.
    """
    mean = math.fsum(values) / len(values)
    squared = math.fsum((value - mean) ** 2 for value in values)
    denominator = len(values) - 1 if divisor is DivisorConvention.SAMPLE else len(values)
    return squared / denominator


def _dispersion(values: Sequence[float], divisor: DivisorConvention) -> float:
    """
    Return the sample or population standard deviation of the given values.
    """
    return math.sqrt(_variance(values, divisor))


@dataclass(frozen=True)
class StatisticalContract:
    """
    The statistical contract a correction is computed under, with an identity.

    Every element of the contract is declared here rather than left to the implementation, and the
    contract is recorded with the result it produced.

    Parameters
    ----------
    minimum_observations : int, default 20
        The contributing periods below which the correction is `unavailable`. A Sharpe ratio and
        the third and fourth standardized moments of a return series are estimated quantities, and
        below this count their own sampling error dominates what the correction reports.
    minimum_trials : int, default 10
        The number of trial estimates below which the correction is `unavailable`. Below it the
        expected maximum is dominated by the error in the cross-trial variance.
    risk_free_rate : float, default 0.0
        The per-period risk-free rate subtracted from a return series. The default is an explicit
        zero rather than an implicit assumption, and a caller with a rate series supplies excess
        returns instead.
    return_compounding : ReturnCompounding, default SIMPLE
        How the return series was compounded, so the Sharpe ratio's input is stated.
    sharpe_divisor : DivisorConvention, default SAMPLE
        The divisor the Sharpe ratio's standard deviation uses.
    trial_variance_divisor : DivisorConvention, default SAMPLE
        The divisor the cross-trial variance of the Sharpe estimates uses.
    contract_id : str, default `SIGNIFICANCE_CONTRACT_ID`
        The stable contract identity.
    version : int, default `SIGNIFICANCE_CONTRACT_VERSION`
        The contract version, which changes when any rule above changes.

    """

    minimum_observations: int = DEFAULT_MINIMUM_OBSERVATIONS
    minimum_trials: int = DEFAULT_MINIMUM_TRIALS
    risk_free_rate: float = 0.0
    return_compounding: ReturnCompounding = ReturnCompounding.SIMPLE
    sharpe_divisor: DivisorConvention = DivisorConvention.SAMPLE
    trial_variance_divisor: DivisorConvention = DivisorConvention.SAMPLE
    contract_id: str = SIGNIFICANCE_CONTRACT_ID
    version: int = SIGNIFICANCE_CONTRACT_VERSION

    def __post_init__(self) -> None:
        """
        Validate the declared contract.
        """
        if self.minimum_observations < _MINIMUM_MOMENT_OBSERVATIONS:
            raise ValueError(
                f"minimum_observations must be at least {_MINIMUM_MOMENT_OBSERVATIONS}, "
                f"was {self.minimum_observations}",
            )
        if self.minimum_trials < _MINIMUM_MOMENT_OBSERVATIONS:
            raise ValueError(
                f"minimum_trials must be at least {_MINIMUM_MOMENT_OBSERVATIONS}, "
                f"was {self.minimum_trials}",
            )
        if not math.isfinite(self.risk_free_rate):
            raise ValueError(f"risk_free_rate must be finite, was {self.risk_free_rate}")
        if not isinstance(self.return_compounding, ReturnCompounding):
            raise TypeError("return_compounding must be a ReturnCompounding")
        if not isinstance(self.sharpe_divisor, DivisorConvention):
            raise TypeError("sharpe_divisor must be a DivisorConvention")
        if not isinstance(self.trial_variance_divisor, DivisorConvention):
            raise TypeError("trial_variance_divisor must be a DivisorConvention")
        if not self.contract_id:
            raise ValueError("contract_id must not be empty")
        if self.version < 1:
            raise ValueError(f"version must be positive, was {self.version}")

    @classmethod
    def declared_default(cls) -> StatisticalContract:
        """
        Return the contract a study runs under when it declares nothing.

        Returns
        -------
        StatisticalContract

        """
        return cls()

    def to_dict(self) -> dict[str, JsonValue]:
        """
        Return the contract as a canonical mapping, every element stated.

        The elements that have no alternative in this contract are emitted as their declared rule
        rather than omitted, so a reader of the record sees the whole contract.

        Returns
        -------
        dict[str, JsonValue]

        """
        return {
            "contract_id": self.contract_id,
            "version": self.version,
            "return_definition": {
                "series": "per_period_excess_returns",
                "compounding": self.return_compounding.value,
                "risk_free_rate": self.risk_free_rate,
                "risk_free_is_explicit": True,
            },
            "minimum_observations": self.minimum_observations,
            "minimum_trials": self.minimum_trials,
            "sharpe_convention": {
                "frequency": SharpeFrequency.PER_PERIOD.value,
                "divisor": self.sharpe_divisor.value,
            },
            "trial_variance_estimator": {
                "statistic": "variance",
                "divisor": self.trial_variance_divisor.value,
            },
            "skew_estimator": "population_standardized_third_moment",
            "kurtosis_estimator": "population_standardized_fourth_moment",
            "kurtosis_convention": KURTOSIS_CONVENTION_NON_EXCESS,
            "annualisation": ANNUALISATION_RULE_PROHIBITED,
            "missing_returns": MISSING_RETURNS_RULE_EXCLUDED,
            "duplicate_trials": DUPLICATE_TRIALS_RULE_REJECTED,
            "failed_trials": FAILED_TRIALS_RULE_COUNTED,
            "trial_independence": TRIAL_INDEPENDENCE_RULE_DECLARED,
        }

    @property
    def digest(self) -> str:
        """
        Return the digest of the declared contract.

        Returns
        -------
        str

        """
        return _digest(self.to_dict())


@dataclass(frozen=True)
class SharpeEstimate:
    """
    A per-period Sharpe ratio and the periods that contributed to it.

    Parameters
    ----------
    value : float | None
        The per-period Sharpe ratio, or None when the series could not produce one.
    observations : int
        The contributing periods, which is the horizon the correction counts.

    """

    value: float | None
    observations: int

    def __post_init__(self) -> None:
        """
        Validate the estimate.
        """
        if self.observations < 0:
            raise ValueError(f"observations must not be negative, was {self.observations}")
        if self.value is not None and not math.isfinite(self.value):
            raise ValueError(f"value must be finite when present, was {self.value}")

    def to_dict(self) -> dict[str, JsonValue]:
        """
        Return the estimate as a canonical mapping.

        Returns
        -------
        dict[str, JsonValue]

        """
        return {"value": self.value, "observations": self.observations}


@dataclass(frozen=True)
class ReturnMoments:
    """
    The moments the correction needs from the selected strategy's return series.

    Parameters
    ----------
    skew : float
        The sample skewness.
    kurtosis : float
        The sample kurtosis, **non-excess**: the fourth standardized moment, which is 3 for normal
        returns. The convention is named because the correction's formula requires it, and the
        excess convention would give a different statistic that looks plausible.
    observations : int
        The contributing periods.

    """

    skew: float
    kurtosis: float
    observations: int

    def to_dict(self) -> dict[str, JsonValue]:
        """
        Return the moments as a canonical mapping.

        Returns
        -------
        dict[str, JsonValue]

        """
        return {
            "skew": self.skew,
            "kurtosis": self.kurtosis,
            "kurtosis_convention": KURTOSIS_CONVENTION_NON_EXCESS,
            "observations": self.observations,
        }


def per_period_sharpe(
    returns: Sequence[float | None] | Mapping[int, float | None],
    *,
    risk_free_rate: float = 0.0,
    divisor: DivisorConvention = DivisorConvention.SAMPLE,
) -> SharpeEstimate:
    """
    Return the per-period Sharpe ratio of a return series.

    Missing returns are excluded rather than zero-filled, and the estimate reports how many periods
    contributed, so the horizon a correction counts is the horizon that produced the value.

    Parameters
    ----------
    returns : Sequence[float | None] | Mapping[int, float | None]
        The per-period returns, either in order or keyed by Unix nanosecond timestamp. A `None` or
        NaN value is missing; an infinity is refused.
    risk_free_rate : float, default 0.0
        The per-period risk-free rate to subtract from every return.
    divisor : DivisorConvention, default SAMPLE
        The divisor the standard deviation uses.

    Returns
    -------
    SharpeEstimate
        The per-period Sharpe ratio and the contributing periods. The value is None when fewer than
        two periods contributed or when the excess returns have no dispersion.

    Raises
    ------
    ValueError
        If the risk-free rate is not finite, or a return is infinite.

    """
    if not math.isfinite(risk_free_rate):
        raise ValueError(f"risk_free_rate must be finite, was {risk_free_rate}")
    if not isinstance(divisor, DivisorConvention):
        raise TypeError("divisor must be a DivisorConvention")

    values = returns.values() if isinstance(returns, Mapping) else returns
    contributing = _finite_values(list(values))

    if len(contributing) < _MINIMUM_MOMENT_OBSERVATIONS:
        return SharpeEstimate(None, len(contributing))

    excess = [value - risk_free_rate for value in contributing]
    standard_deviation = _dispersion(excess, divisor)
    if standard_deviation <= 0.0:
        return SharpeEstimate(None, len(contributing))

    return SharpeEstimate(math.fsum(excess) / len(excess) / standard_deviation, len(contributing))


def return_moments(
    returns: Sequence[float | None] | Mapping[int, float | None],
) -> ReturnMoments | None:
    """
    Return the skewness and the non-excess kurtosis of a return series.

    The standardized moments are population moments: the divisor cancels in the ratio, so there is
    no convention to declare here.

    Parameters
    ----------
    returns : Sequence[float | None] | Mapping[int, float | None]
        The per-period returns, either in order or keyed by Unix nanosecond timestamp. A `None` or
        NaN value is missing; an infinity is refused.

    Returns
    -------
    ReturnMoments | None
        The moments and the contributing periods, or None when fewer than two periods contributed
        or the series has no dispersion.

    """
    values = returns.values() if isinstance(returns, Mapping) else returns
    contributing = _finite_values(list(values))

    if len(contributing) < _MINIMUM_MOMENT_OBSERVATIONS:
        return None

    mean = math.fsum(contributing) / len(contributing)
    deviations = [value - mean for value in contributing]
    second = math.fsum(deviation**2 for deviation in deviations) / len(deviations)
    if second <= 0.0:
        return None

    third = math.fsum(deviation**3 for deviation in deviations) / len(deviations)
    fourth = math.fsum(deviation**4 for deviation in deviations) / len(deviations)

    return ReturnMoments(
        skew=third / second**1.5,
        kurtosis=fourth / second**2,
        observations=len(contributing),
    )


@dataclass(frozen=True)
class SharpeSample:
    """
    The sample a correction is computed from.

    The sample is where the boundaries are enforced, because a sample that declares an annualised
    value, an excess kurtosis or an undeclared dependence is not a sample the correction can be
    computed from.

    Parameters
    ----------
    sharpe : float
        The selected trial's per-period Sharpe ratio, which is the estimate being deflated.
    trial_sharpes : Sequence[float]
        The per-period Sharpe ratio of every trial that produced an estimate, so the cross-trial
        dispersion is the dispersion of the selection that produced the maximum.
    observations : int
        The contributing periods of the selected estimate, which is the backtest horizon.
    skew : float
        The sample skewness of the selected strategy's returns.
    kurtosis : float
        The sample kurtosis of the selected strategy's returns, **non-excess**.
    dependence : TrialDependence
        Whether the trials are independent draws. The declaration is required, because a sweep over
        adjacent parameters is not an independent sample and a correction cannot tell from the
        values alone.
    effective_trials : int | None, default None
        The effective trial count, required when the study declares dependence and refused when it
        declares independence.
    frequency : SharpeFrequency, default PER_PERIOD
        The period the Sharpe values are expressed over. The annualised case is refused.

    Raises
    ------
    TypeError
        If a declaration has the wrong type.
    ValueError
        If a value is not finite, the kurtosis is not the non-excess convention, the frequency is
        annualised, or the dependence declaration is incomplete.

    """

    sharpe: float
    trial_sharpes: Sequence[float]
    observations: int
    skew: float
    kurtosis: float
    dependence: TrialDependence
    effective_trials: int | None = None
    frequency: SharpeFrequency = SharpeFrequency.PER_PERIOD

    def __post_init__(self) -> None:
        """
        Validate the sample and refuse a declaration the correction cannot be computed from.
        """
        self._validate_values()
        self._validate_dependence()

    def _validate_values(self) -> None:
        """
        Validate the values and the frequency, which is where the annualisation is refused.
        """
        if self.frequency is not SharpeFrequency.PER_PERIOD:
            raise ValueError(
                f"the correction is per-period: an annualised input "
                f"({self.frequency.value}) must be rescaled by its caller, not divided here",
            )
        if not math.isfinite(self.sharpe):
            raise ValueError(f"sharpe must be finite, was {self.sharpe}")
        if not math.isfinite(self.skew):
            raise ValueError(f"skew must be finite, was {self.skew}")
        if not math.isfinite(self.kurtosis):
            raise ValueError(f"kurtosis must be finite, was {self.kurtosis}")
        if self.kurtosis < 1.0:
            raise ValueError(
                f"kurtosis must be the non-excess convention and therefore at least 1, "
                f"was {self.kurtosis}: an excess kurtosis of 0 is not a kurtosis here",
            )
        if self.observations < _MINIMUM_MOMENT_OBSERVATIONS:
            raise ValueError(
                f"observations must be at least {_MINIMUM_MOMENT_OBSERVATIONS}, "
                f"was {self.observations}",
            )

        for value in self.trial_sharpes:
            if not math.isfinite(value):
                raise ValueError(f"every trial Sharpe must be finite, found {value}")

    def _validate_dependence(self) -> None:
        """
        Validate the dependence declaration, which the correction cannot infer from the values.
        """
        if not isinstance(self.dependence, TrialDependence):
            raise TypeError("dependence must be a TrialDependence")

        if self.dependence is not TrialDependence.DEPENDENT:
            if self.effective_trials is not None:
                raise ValueError(
                    "a study with independent trials must not declare an effective trial count: "
                    "the nominal count is the effective count",
                )
            return

        if self.effective_trials is None:
            raise ValueError("a study with dependent trials must declare its effective trial count")
        if self.effective_trials < 1:
            raise ValueError(f"effective_trials must be positive, was {self.effective_trials}")
        if self.effective_trials > len(self.trial_sharpes):
            raise ValueError(
                f"effective_trials {self.effective_trials} exceeds the nominal count "
                f"{len(self.trial_sharpes)}",
            )

    @property
    def nominal_trials(self) -> int:
        """
        The number of trial estimates the sample carries.
        """
        return len(self.trial_sharpes)

    @property
    def counted_trials(self) -> int:
        """
        The trial count the correction uses: the effective count for dependent trials.
        """
        if self.dependence is TrialDependence.DEPENDENT:
            return cast("int", self.effective_trials)
        return self.nominal_trials

    def to_dict(self) -> dict[str, JsonValue]:
        """
        Return the sample as a canonical mapping.

        Returns
        -------
        dict[str, JsonValue]

        """
        return {
            "sharpe": self.sharpe,
            "trial_sharpes": list(self.trial_sharpes),
            "observations": self.observations,
            "skew": self.skew,
            "kurtosis": self.kurtosis,
            "kurtosis_convention": KURTOSIS_CONVENTION_NON_EXCESS,
            "frequency": self.frequency.value,
            "dependence": self.dependence.value,
            "effective_trials": self.effective_trials,
        }


@dataclass(frozen=True)
class SignificanceResult:
    """
    The correction's outcome: a value and its status, or a reason it was not computed.

    Parameters
    ----------
    sample : SharpeSample
        The sample the correction was computed from.
    contract : StatisticalContract
        The contract it was computed under.
    status : MetricStatus
        The status of the result, from the same four-state vocabulary the analysis statistics
        report.
    value : float | None
        The deflated Sharpe ratio, or None when it was not computed.
    trial_variance : float | None
        The cross-trial variance of the Sharpe estimates, or None when it was not computed.
    reason : MetricReason | None, default None
        The reason the value was not computed, present exactly when the status is not `computed`.

    """

    sample: SharpeSample
    contract: StatisticalContract
    status: MetricStatus
    value: float | None
    trial_variance: float | None
    reason: MetricReason | None = None

    def __post_init__(self) -> None:
        """
        Validate the result's status, which is a state rather than a label.
        """
        computed = self.status is MetricStatus.COMPUTED
        if computed and (self.value is None or self.reason is not None):
            raise ValueError("a computed result carries a value and no reason")
        if not computed and (self.value is not None or self.reason is None):
            raise ValueError("a result that is not computed carries a reason and no value")

    @property
    def is_computed(self) -> bool:
        """
        Whether the correction produced a value.
        """
        return self.status is MetricStatus.COMPUTED

    def to_dict(self) -> dict[str, JsonValue]:
        """
        Return the result as a canonical mapping.

        Returns
        -------
        dict[str, JsonValue]

        """
        return {
            "status": _STATUS_NAMES[self.status],
            "value": self.value,
            "reason": None if self.reason is None else _REASON_NAMES[self.reason],
            "trial_variance": self.trial_variance,
            "nominal_trials": self.sample.nominal_trials,
            "counted_trials": self.sample.counted_trials,
            "sample": self.sample.to_dict(),
            "contract": self.contract.to_dict(),
            "contract_digest": self.contract.digest,
        }


def deflated_sharpe_ratio(
    sample: SharpeSample,
    *,
    contract: StatisticalContract | None = None,
) -> SignificanceResult:
    """
    Return the deflated Sharpe ratio of a sample.

    The correction is the probability that the selected trial's Sharpe ratio exceeds the maximum
    that a set of independent trials with no skill would have produced, given the dispersion of the
    trial estimates:

    ```text
    SR0 = sqrt(V[SR]) * ((1 - gamma) * Phi^-1(1 - 1/N) + gamma * Phi^-1(1 - 1/(N e)))
    DSR = Phi((SR - SR0) * sqrt(T - 1) / sqrt(1 - skew * SR + ((kurtosis - 1) / 4) * SR^2))
    ```

    where `V[SR]` is the cross-trial variance of the Sharpe estimates, `N` the trial count the
    sample declares, `T` the contributing periods, and `gamma` the Euler-Mascheroni constant. The
    two quantiles are the expected maximum of `N` independent standard normal draws; a larger `N`
    therefore lowers the corrected value, which is the whole point of the correction.

    The value is reported, never a gate.

    Parameters
    ----------
    sample : SharpeSample
        The selected estimate, the trial estimates and the moments.
    contract : StatisticalContract | None, default None
        The contract to compute under, or None for the declared default.

    Returns
    -------
    SignificanceResult
        The value and its status, or the reason it was not computed. Below the contract's minimum
        observation or trial count the status is `unavailable` with `insufficient_data`; a
        non-positive variance factor is `invalid` with `undefined_result`.

    """
    declared = StatisticalContract.declared_default() if contract is None else contract

    trial_variance = None
    if sample.nominal_trials >= _MINIMUM_MOMENT_OBSERVATIONS:
        trial_variance = _variance(list(sample.trial_sharpes), declared.trial_variance_divisor)

    if (
        sample.observations < declared.minimum_observations
        or sample.counted_trials < declared.minimum_trials
        or trial_variance is None
    ):
        return SignificanceResult(
            sample=sample,
            contract=declared,
            status=MetricStatus.UNAVAILABLE,
            value=None,
            trial_variance=trial_variance,
            reason=MetricReason.INSUFFICIENT_DATA,
        )

    expected_maximum = math.sqrt(trial_variance) * (
        (1.0 - EULER_MASCHERONI) * _normal_ppf(1.0 - 1.0 / sample.counted_trials)
        + EULER_MASCHERONI * _normal_ppf(1.0 - 1.0 / (sample.counted_trials * math.e))
    )

    variance_factor = (
        1.0
        - sample.skew * sample.sharpe
        + ((sample.kurtosis - 1.0) / 4.0) * sample.sharpe * sample.sharpe
    )

    if variance_factor <= 0.0:
        return SignificanceResult(
            sample=sample,
            contract=declared,
            status=MetricStatus.INVALID,
            value=None,
            trial_variance=trial_variance,
            reason=MetricReason.UNDEFINED_RESULT,
        )

    argument = (
        (sample.sharpe - expected_maximum)
        * math.sqrt(sample.observations - 1)
        / math.sqrt(variance_factor)
    )
    value = _normal_cdf(argument)
    if not math.isfinite(value):
        return SignificanceResult(
            sample=sample,
            contract=declared,
            status=MetricStatus.INVALID,
            value=None,
            trial_variance=trial_variance,
            reason=MetricReason.UNDEFINED_RESULT,
        )

    return SignificanceResult(
        sample=sample,
        contract=declared,
        status=MetricStatus.COMPUTED,
        value=value,
        trial_variance=trial_variance,
    )


def trial_provenance_from_runs(
    runs: Sequence[CanonicalRun | FailedExperiment],
    *,
    dependence: TrialDependence = TrialDependence.INDEPENDENT,
    effective_trials: int | None = None,
) -> TrialProvenance:
    """
    Return the trial provenance of a study's runs.

    A duplicate parameter set is refused rather than counted twice: two runs of one experiment are
    one trial, and counting them as two would inflate the correction's `N`.

    Parameters
    ----------
    runs : Sequence[CanonicalRun | FailedExperiment]
        The study's runs, as the optimizer records them.
    dependence : TrialDependence, default INDEPENDENT
        Whether the study's trials are independent draws.
    effective_trials : int | None, default None
        The effective trial count for a dependent study.

    Returns
    -------
    TrialProvenance
        The trial count, the failed count, and whether the nominal and effective counts are
        distinguished.

    Raises
    ------
    ValueError
        If two runs share a parameter set, or the dependence declaration is incomplete.

    """
    digests = [run.experiment.digest for run in runs]
    duplicates = sorted({digest for digest in digests if digests.count(digest) > 1})
    if duplicates:
        raise ValueError(f"duplicate parameter sets in one study: {duplicates}")

    failed = sum(1 for run in runs if isinstance(run, FailedExperiment))
    distinguished = dependence is TrialDependence.DEPENDENT
    if distinguished and effective_trials is None:
        raise ValueError("a study with dependent trials must declare its effective trial count")
    if not distinguished and effective_trials is not None:
        raise ValueError(
            "a study with independent trials must not declare an effective trial count",
        )

    return TrialProvenance(
        trial_count=len(runs),
        failed_trial_count=failed,
        nominal_and_effective=distinguished,
    )


@dataclass(frozen=True)
class SignificanceReport:
    """
    The study-level record: the study, its trials, their provenance and the correction.

    Parameters
    ----------
    study : StudyIdentity
        The declared study.
    trials : Sequence[TrialIdentity]
        One identity per trial, failed trials included, so a failed trial is identified rather than
        only counted.
    provenance : TrialProvenance
        The counts a correction needs.
    result : SignificanceResult
        The correction's outcome.

    """

    study: StudyIdentity
    trials: Sequence[TrialIdentity]
    provenance: TrialProvenance
    result: SignificanceResult

    def __post_init__(self) -> None:
        """
        Validate the record, including that every trial belongs to the study.
        """
        for trial in self.trials:
            if trial.study_id != self.study.study_id:
                raise ValueError(
                    f"trial belongs to study {trial.study_id}, not {self.study.study_id}",
                )

    @property
    def study_id(self) -> str:
        """
        The digest of the declared study.
        """
        return self.study.study_id

    def to_dict(self) -> dict[str, JsonValue]:
        """
        Return the record as a canonical mapping.

        Returns
        -------
        dict[str, JsonValue]

        """
        return {
            "study_id": self.study.study_id,
            "study": self.study.to_dict(),
            "trial_ids": [trial.trial_id for trial in self.trials],
            "trials": [trial.to_dict() for trial in self.trials],
            "provenance": self.provenance.to_dict(),
            "significance": self.result.to_dict(),
        }

    @property
    def digest(self) -> str:
        """
        Return the digest of the record.

        Returns
        -------
        str

        """
        return _digest(self.to_dict())


def significance_report(
    study: StudyIdentity,
    runs: Sequence[CanonicalRun | FailedExperiment],
    sample: SharpeSample,
    *,
    contract: StatisticalContract | None = None,
) -> SignificanceReport:
    """
    Return the study-level significance record for a study's runs.

    The sample's trial estimates must correspond one-to-one with the runs that produced an
    estimate, so the dispersion the correction uses is the dispersion of the trials actually run
    and a failed trial is recorded rather than silently imputed.

    Parameters
    ----------
    study : StudyIdentity
        The declared study.
    runs : Sequence[CanonicalRun | FailedExperiment]
        The study's runs, as the optimizer records them.
    sample : SharpeSample
        The correction's sample, whose trial estimates must match the completed runs.
    contract : StatisticalContract | None, default None
        The contract to compute under, or None for the declared default.

    Returns
    -------
    SignificanceReport

    Raises
    ------
    ValueError
        If two runs share a parameter set, or the completed run count does not match the number of
        trial estimates.

    """
    completed = sum(1 for run in runs if not isinstance(run, FailedExperiment))
    if completed != sample.nominal_trials:
        raise ValueError(
            f"the sample carries {sample.nominal_trials} trial estimates but the study has "
            f"{completed} completed runs",
        )

    provenance = trial_provenance_from_runs(
        runs,
        dependence=sample.dependence,
        effective_trials=sample.effective_trials,
    )
    trials = tuple(trial_identity(study.study_id, run) for run in runs)

    return SignificanceReport(
        study=study,
        trials=trials,
        provenance=provenance,
        result=deflated_sharpe_ratio(sample, contract=contract),
    )


class DeflatedSharpeRatio(PortfolioStatistic):
    """
    The deflated Sharpe ratio of a run's returns, for the trial set that produced them.

    A single run carries its returns but not the search that selected it, so the trial set is
    declared here while the sample's other fields come from the returns the analyzer feeds. Once
    registered on a portfolio or an analyzer, the corrected value is reported beside the run's own
    statistics, and the row's name states the trial counts the correction used, because a count is
    provenance rather than a performance metric and the metric units the analysis surface declares
    are financial.

    The contract's declared minimum counts are enforced: a run whose horizon or trial set falls
    below them is reported as unavailable rather than computed under unstated bounds.

    The value is reported, never a gate.

    Parameters
    ----------
    trial_sharpes : Sequence[float]
        The per-period Sharpe ratio of every trial that produced an estimate.
    dependence : TrialDependence, default INDEPENDENT
        Whether the trials are independent draws.
    effective_trials : int | None, default None
        The effective trial count, required when the study declares dependence and refused when it
        declares independence.
    contract : StatisticalContract | None, default None
        The contract to compute under, or None for the declared default.

    Raises
    ------
    TypeError
        If a declaration has the wrong type.
    ValueError
        If the declaration is incomplete or a trial Sharpe is not finite.

    """

    def __init__(
        self,
        trial_sharpes: Sequence[float],
        *,
        dependence: TrialDependence = TrialDependence.INDEPENDENT,
        effective_trials: int | None = None,
        contract: StatisticalContract | None = None,
    ) -> None:
        """
        Initialize the statistic with the trial declaration it corrects for.
        """
        # A probe sample validates the declaration through the sample's own rules - the dependence
        # declaration, the effective count and the trial Sharpes - and carries the trial counts.
        # Every calculation replaces the fields the return series supplies.
        self._declaration = SharpeSample(
            sharpe=0.0,
            trial_sharpes=tuple(trial_sharpes),
            observations=_MINIMUM_MOMENT_OBSERVATIONS,
            skew=0.0,
            kurtosis=3.0,
            dependence=dependence,
            effective_trials=effective_trials,
        )
        self._contract = contract

    @property
    def name(self) -> str:
        """
        Return the name for the statistic, stating the trial counts it used.

        Returns
        -------
        str

        """
        nominal = self._declaration.nominal_trials
        counted = self._declaration.counted_trials

        if counted == nominal:
            return f"Deflated Sharpe Ratio ({nominal} trials)"
        return f"Deflated Sharpe Ratio ({nominal} trials, {counted} effective)"

    @property
    def metric_id(self) -> str:
        """
        Return the stable identity, which does not carry the trial counts.

        Returns
        -------
        str

        """
        return "deflated_sharpe_ratio"

    @property
    def units(self) -> MetricUnits:
        """
        Return the units the value is expressed in.

        Returns
        -------
        MetricUnits

        """
        return MetricUnits.RATIO

    @property
    def tags(self) -> tuple[MetricTag, ...]:
        """
        Return the cross-cutting tags for the metric.

        Returns
        -------
        tuple[MetricTag, ...]

        """
        return (MetricTag.RISK_ADJUSTED,)

    @property
    def direction(self) -> MetricDirection:
        """
        Return the direction in which a consumer rewards the value.

        Returns
        -------
        MetricDirection

        """
        return MetricDirection.MAXIMIZE

    @property
    def inputs(self) -> tuple[MetricInput, ...]:
        """
        Return the inputs the definition requires.

        Returns
        -------
        tuple[MetricInput, ...]

        """
        return (MetricInput.RETURNS,)

    def calculate_from_returns(self, returns: dict[int, float]) -> float | None:
        """
        Calculate the deflated Sharpe ratio of the given returns.

        Parameters
        ----------
        returns : dict[int, float]
            The returns keyed by UNIX timestamp (nanoseconds).

        Returns
        -------
        float or ``None``
            The corrected value, or ``None`` when the series cannot produce one.

        """
        estimate = per_period_sharpe(returns)
        if estimate.value is None:
            return None

        moments = return_moments(returns)
        if moments is None:
            return None

        sample = replace(
            self._declaration,
            sharpe=estimate.value,
            observations=estimate.observations,
            skew=moments.skew,
            kurtosis=moments.kurtosis,
        )

        return deflated_sharpe_ratio(sample, contract=self._contract).value
