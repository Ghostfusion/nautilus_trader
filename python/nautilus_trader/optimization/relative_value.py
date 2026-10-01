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
The declarations a relative-value screen has to make before it reports anything.

A screen over a universe of related series is a set of trials, so it needs the same declarations a
parameter sweep needs: the family it enumerated before its gates ran, the floor above which an
estimate is reported at all, the windows its signal is measured over, and the memory a recursive
estimate carries. Each one is stated here as data with a digest, and each one is answered by the
capability probe that owns its refusal rather than by an exception the caller has to guess at.

The declarations compose with the contracts that already own their subject: the screen's family
count is what the statistical contract's correction consumes, the signal's reach is what a leakage
policy has to cover, and the persistence floor is the same floor the significance contract applies
to its own estimate. Nothing here computes a hedge ratio, a cointegration statistic or a position
size, because a declaration is what makes those reviewable rather than a substitute for them.
"""

from __future__ import annotations

import math
from dataclasses import dataclass
from enum import Enum
from enum import unique
from itertools import combinations
from typing import TYPE_CHECKING
from typing import cast

from nautilus_trader.optimization.capability import persistence_capability
from nautilus_trader.optimization.capability import screen_family_capability
from nautilus_trader.optimization.labels import LabelDefinition
from nautilus_trader.optimization.labels import label_series
from nautilus_trader.optimization.significance import DEFAULT_MINIMUM_OBSERVATIONS
from nautilus_trader.optimization.significance import SharpeSample
from nautilus_trader.optimization.significance import SignificanceResult
from nautilus_trader.optimization.significance import StatisticalContract
from nautilus_trader.optimization.significance import deflated_sharpe_ratio
from nautilus_trader.optimization.space import digest_of


if TYPE_CHECKING:
    from collections.abc import Mapping
    from collections.abc import Sequence

    from nautilus_trader.core import Capability
    from nautilus_trader.optimization.space import JsonValue


# A fit is converted into a half-life from one coefficient and the observations it was fitted over,
# so two observations are the fewest a conversion is defined at and the fewest a floor may declare.
_MINIMUM_FIT_OBSERVATIONS = 2

# A dispersion needs two observations, so a measurement window shorter than two could not carry one.
_MINIMUM_MEASUREMENT_OBSERVATIONS = 2


def _digest(payload: Mapping[str, object]) -> str:
    """
    Return the digest of a declaration, which may carry a nested declaration.
    """
    return digest_of(cast("Mapping[str, JsonValue]", payload))


@unique
class PersistenceConvention(Enum):
    """
    The convention a fit's half-life is converted under.

    The two conventions convert one fit rather than estimating two: the discrete one reports the
    half-life of the autoregressive coefficient the fit estimated, and the continuous one reports
    the half-life of the continuous-time rate that coefficient approximates. They do not agree, and
    the difference grows as the process gets faster, so the convention is part of the estimate
    rather than a presentation choice.
    """

    DISCRETE = "discrete"
    CONTINUOUS = "continuous"


@dataclass(frozen=True)
class PersistenceEstimate:
    """
    A fitted mean-reversion speed and the convention its half-life is reported under.

    The coefficient is the fit of the spread's first difference on its lagged level, taken from the
    level, so a pair that reverts gives a negative one. The observations the fit used travel with it
    because the estimate is only reported above a floor, and the floor is declared rather than
    assumed: below it the estimate is biased toward reversion rather than merely noisy.

    Parameters
    ----------
    coefficient : float
        The fitted coefficient on the lagged level of the spread. Finite, and inside `(-1, 0)` for
        a half-life to be reported.
    observations : int
        The number of observations the fit used. Positive.
    convention : PersistenceConvention, default PersistenceConvention.DISCRETE
        The convention the half-life is converted under.
    minimum_observations : int, default DEFAULT_MINIMUM_OBSERVATIONS
        The declared floor below which no half-life is reported. At least two.

    Raises
    ------
    TypeError
        If a field has the wrong type.
    ValueError
        If the coefficient is not finite, the observations are not positive, or the floor is below
        the two observations a conversion is defined at.

    """

    coefficient: float
    observations: int
    convention: PersistenceConvention = PersistenceConvention.DISCRETE
    minimum_observations: int = DEFAULT_MINIMUM_OBSERVATIONS

    def __post_init__(self) -> None:
        """
        Validate the fit and the floor it is reported above.
        """
        if isinstance(self.coefficient, bool) or not isinstance(self.coefficient, (int, float)):
            raise TypeError(f"coefficient must be a number, was {self.coefficient!r}")
        if not math.isfinite(self.coefficient):
            raise ValueError(f"coefficient must be finite, was {self.coefficient!r}")
        if isinstance(self.observations, bool) or not isinstance(self.observations, int):
            raise TypeError(f"observations must be an int, was {self.observations!r}")
        if self.observations < 1:
            raise ValueError(f"observations must be positive, was {self.observations}")
        if not isinstance(self.convention, PersistenceConvention):
            raise TypeError(f"convention must be a PersistenceConvention, was {self.convention!r}")
        if isinstance(self.minimum_observations, bool) or not isinstance(
            self.minimum_observations, int
        ):
            raise TypeError(
                f"minimum_observations must be an int, was {self.minimum_observations!r}",
            )
        if self.minimum_observations < _MINIMUM_FIT_OBSERVATIONS:
            raise ValueError(
                f"minimum_observations must be at least {_MINIMUM_FIT_OBSERVATIONS}, "
                f"was {self.minimum_observations}",
            )

    @property
    def capability(self) -> Capability:
        """
        Answer whether the fit supports a half-life under the declared convention.
        """
        return persistence_capability(
            self.coefficient,
            self.observations,
            minimum_observations=self.minimum_observations,
        )

    @property
    def autoregressive_coefficient(self) -> float:
        """
        Return the autoregressive coefficient the fit implies, `1 + coefficient`.
        """
        return 1.0 + self.coefficient

    @property
    def half_life(self) -> float | None:
        """
        Return the half-life in observations under the declared convention, or None when refused.

        The discrete conversion is `ln(0.5) / ln(1 + coefficient)`, which is the exact number of
        observations the fitted autoregression takes to halve a deviation, and the continuous one is
        `-ln(2) / coefficient`, which is the continuous-time solution that fit approximates.

        """
        if not self.capability.is_available:
            return None

        if self.convention is PersistenceConvention.DISCRETE:
            return math.log(0.5) / math.log1p(self.coefficient)

        return -math.log(2.0) / self.coefficient

    def to_dict(self) -> dict[str, JsonValue]:
        """
        Return the estimate as a canonical-friendly mapping.
        """
        return {
            "coefficient": self.coefficient,
            "observations": self.observations,
            "convention": self.convention.value,
            "minimum_observations": self.minimum_observations,
        }

    @property
    def digest(self) -> str:
        """
        Return the `sha256:<hex>` digest of the estimate over its declared fields.
        """
        return digest_of(self.to_dict())


@dataclass(frozen=True)
class RecursiveMemory:
    """
    The memory of a recursive estimator, declared rather than inherited from a default.

    A scalar recursion whose state is a random walk settles at a gain that depends on the state
    noise and the observation noise together, so the observations it remembers are a function of
    both rather than of the state noise alone:

    ```text
    P = (Q + sqrt(Q^2 + 4 Q R)) / 2
    K = P / (P + R)
    memory = 1 / K
    ```

    The declaration states the calibration it was read under and the memory it claims, and the two
    have to agree to the declared tolerance, so a study cannot vary one parameter while the other
    one sets the answer. A warm-up shorter than the settled memory is refused as well: a value read
    inside the transient measures the recursion's initial condition rather than the series. Both
    refusals are raised at construction, so a recursion whose memory its study did not choose never
    produces a number.

    Parameters
    ----------
    state_noise : float
        The state noise `Q`, positive and finite.
    observation_noise : float
        The observation noise `R`, positive and finite.
    declared_memory : float
        The memory the study declares, in observations, positive and finite.
    warm_up : int
        The observations read and discarded before the estimate is used. Non-negative.
    tolerance : float, default 0.01
        The relative difference the declared memory may carry from the settled one, in `(0, 1)`.

    Raises
    ------
    TypeError
        If a field has the wrong type.
    ValueError
        If a noise parameter is not positive and finite, the declared memory is not positive, the
        tolerance is outside `(0, 1)`, or the declaration contradicts its own recursion.

    """

    state_noise: float
    observation_noise: float
    declared_memory: float
    warm_up: int
    tolerance: float = 0.01

    def __post_init__(self) -> None:
        """
        Validate the calibration, the declared memory and the warm-up.
        """
        self._validate_parameters()
        self._validate_memory()
        self._validate_warm_up()

    def _validate_parameters(self) -> None:
        """
        Validate the two noise parameters and the tolerance.
        """
        for name in ("state_noise", "observation_noise", "declared_memory"):
            value = getattr(self, name)
            if isinstance(value, bool) or not isinstance(value, (int, float)):
                raise TypeError(f"{name} must be a number, was {value!r}")
            if not math.isfinite(value):
                raise ValueError(f"{name} must be finite, was {value!r}")
            if value <= 0.0:
                raise ValueError(f"{name} must be positive, was {value!r}")

        if isinstance(self.warm_up, bool) or not isinstance(self.warm_up, int):
            raise TypeError(f"warm_up must be an int, was {self.warm_up!r}")
        if self.warm_up < 0:
            raise ValueError(f"warm_up must not be negative, was {self.warm_up}")
        if isinstance(self.tolerance, bool) or not isinstance(self.tolerance, (int, float)):
            raise TypeError(f"tolerance must be a number, was {self.tolerance!r}")
        if not 0.0 < self.tolerance < 1.0:
            raise ValueError(f"tolerance must be in (0, 1), was {self.tolerance!r}")

    def _validate_memory(self) -> None:
        """
        Validate that the declared memory is the memory its own calibration settles at.
        """
        settled = self.effective_memory
        if abs(settled - self.declared_memory) > self.tolerance * self.declared_memory:
            raise ValueError(
                f"the declared memory {self.declared_memory} observations is not the "
                f"{settled:.1f} observations that state noise {self.state_noise} and observation "
                f"noise {self.observation_noise} settle at, within a tolerance of {self.tolerance}",
            )

    def _validate_warm_up(self) -> None:
        """
        Validate that the warm-up covers the memory the recursion actually carries.
        """
        settled = self.effective_memory
        if self.warm_up < settled:
            raise ValueError(
                f"the warm-up of {self.warm_up} observations is shorter than the settled memory "
                f"of {settled:.1f} observations",
            )

    @property
    def converged_gain(self) -> float:
        """
        Return the gain the scalar recursion settles at.
        """
        root = math.sqrt(
            self.state_noise * self.state_noise + 4.0 * self.state_noise * self.observation_noise,
        )
        level = 0.5 * (self.state_noise + root)

        return level / (level + self.observation_noise)

    @property
    def effective_memory(self) -> float:
        """
        Return the observations the settled recursion remembers, `1 / K`.
        """
        return 1.0 / self.converged_gain

    def to_dict(self) -> dict[str, JsonValue]:
        """
        Return the declaration as a canonical-friendly mapping.
        """
        return {
            "state_noise": self.state_noise,
            "observation_noise": self.observation_noise,
            "declared_memory": self.declared_memory,
            "warm_up": self.warm_up,
            "tolerance": self.tolerance,
        }

    @property
    def digest(self) -> str:
        """
        Return the `sha256:<hex>` digest of the declaration over its declared fields.
        """
        return digest_of(self.to_dict())


@dataclass(frozen=True)
class SignalWindow:
    """
    The windows of a relative-value signal, declared rather than mixed into one.

    A screen that fits a slope over one slice, measures a dispersion over a second and reads a
    half-life from a third reports a distance from a mean that depends on three unrecorded choices.
    This declaration states them as offsets in observations from the decision bar, negative before
    it, and refuses a fit window that does not end at or before the measurement window's start: the
    slope that defines the spread is never fitted on the observations the dispersion is measured
    over.

    The reach is not a field, because a reach is measured rather than declared. It is read from the
    label series the declaration's label produces over the bars, so the label carries the alignment
    convention and the wait that the offsets cannot, and the reach it reports is what a leakage
    policy has to cover.

    Parameters
    ----------
    fit_start : int
        The offset in observations of the first observation of the fit window. Negative.
    fit_length : int
        The number of observations the fit window covers. Positive.
    measurement_start : int
        The offset in observations of the first observation of the measurement window. Negative,
        and at or after the end of the fit window.
    measurement_length : int
        The number of observations the measurement window covers. At least two.
    label : LabelDefinition
        The label the signal's reach is measured from.

    Raises
    ------
    TypeError
        If a field has the wrong type.
    ValueError
        If a window is empty, ends after the decision bar, or the fit window does not end at or
        before the measurement window starts.

    """

    fit_start: int
    fit_length: int
    measurement_start: int
    measurement_length: int
    label: LabelDefinition

    def __post_init__(self) -> None:
        """
        Validate that the fit window precedes the measurement window, both inside the observed past.
        """
        for name in ("fit_start", "fit_length", "measurement_start", "measurement_length"):
            value = getattr(self, name)
            if isinstance(value, bool) or not isinstance(value, int):
                raise TypeError(f"{name} must be an int, was {value!r}")

        if self.fit_length < 1:
            raise ValueError(f"fit_length must be positive, was {self.fit_length}")
        if self.measurement_length < _MINIMUM_MEASUREMENT_OBSERVATIONS:
            raise ValueError(
                f"measurement_length must be at least {_MINIMUM_MEASUREMENT_OBSERVATIONS}, "
                f"was {self.measurement_length}",
            )
        if self.measurement_end > 0:
            raise ValueError(
                f"the measurement window ends at {self.measurement_end}, which is after the "
                "decision bar",
            )
        if self.fit_end > self.measurement_start:
            raise ValueError(
                f"the fit window ends at {self.fit_end} and the measurement window starts at "
                f"{self.measurement_start}, so the fit window does not precede it",
            )
        if not isinstance(self.label, LabelDefinition):
            raise TypeError(f"label must be a LabelDefinition, was {self.label!r}")

    @property
    def fit_end(self) -> int:
        """
        Return the offset in observations one past the end of the fit window.
        """
        return self.fit_start + self.fit_length

    @property
    def measurement_end(self) -> int:
        """
        Return the offset in observations one past the end of the measurement window.
        """
        return self.measurement_start + self.measurement_length

    def reach_ns(self, series: Sequence[tuple[int, float | None]]) -> int:
        """
        Return the measured forward reach of the signal's label over the bars, in nanoseconds.

        The reach is measured from the produced series rather than derived from the declaration, so
        it accounts for the alignment convention, for the wait and for a missing-data policy that
        extends a window.

        Parameters
        ----------
        series : Sequence[tuple[int, float | None]]
            The bar closes the label is computed over, in nanoseconds and price.

        Returns
        -------
        int
            The largest forward reach of any computed label.

        """
        return label_series(self.label, series).forward_reach_ns

    def to_dict(self) -> dict[str, object]:
        """
        Return the declaration as a canonical-friendly mapping.
        """
        return {
            "fit_start": self.fit_start,
            "fit_length": self.fit_length,
            "measurement_start": self.measurement_start,
            "measurement_length": self.measurement_length,
            "label": self.label.to_dict(),
        }

    @property
    def digest(self) -> str:
        """
        Return the `sha256:<hex>` digest of the declaration over its declared fields.
        """
        return _digest(self.to_dict())


@dataclass(frozen=True)
class ScreenFamily:
    """
    The trial family a screen enumerates before any of its gates run.

    The members are the declared series in declared order, and the pairs are every unique unordered
    pair of them, so a screen over n series states `n * (n - 1) / 2` tests rather than however many
    survived. An empty universe is representable because it is a declaration a probe has to answer
    rather than a construction to refuse: the family it enumerates is zero, which no correction can
    use, and the screen says so with the capability code instead of a number.

    Parameters
    ----------
    members : tuple[str, ...], default ()
        The declared series, in order and unique.
    maximum_family : int | None, default None
        The largest family the screen may enumerate, at least one, or None when it states no bound.

    Raises
    ------
    TypeError
        If a field has the wrong type.
    ValueError
        If a member is not a non-empty string, members repeat, or the maximum is below one.

    """

    members: tuple[str, ...] = ()
    maximum_family: int | None = None

    def __post_init__(self) -> None:
        """
        Validate the declared members and the declared bound.
        """
        if not isinstance(self.members, tuple):
            raise TypeError(f"members must be a tuple, was {self.members!r}")
        for member in self.members:
            if not isinstance(member, str) or not member.strip():
                raise ValueError(f"every member must be a non-empty string, was {member!r}")
        if len(set(self.members)) != len(self.members):
            raise ValueError(f"members must be unique, was {list(self.members)}")
        if self.maximum_family is None:
            return
        if isinstance(self.maximum_family, bool) or not isinstance(self.maximum_family, int):
            raise TypeError(f"maximum_family must be an int, was {self.maximum_family!r}")
        if self.maximum_family < 1:
            raise ValueError(f"maximum_family must be at least 1, was {self.maximum_family}")

    @property
    def size(self) -> int:
        """
        Return the number of tests the family enumerates before any gate runs.
        """
        return len(self.members) * (len(self.members) - 1) // 2

    def pairs(self) -> tuple[tuple[str, str], ...]:
        """
        Return the enumerated pairs, in the declared member order.
        """
        return tuple((first, second) for first, second in combinations(self.members, 2))

    @property
    def capability(self) -> Capability:
        """
        Answer whether the declared universe enumerates a usable family.
        """
        return screen_family_capability(
            len(self.members),
            maximum_family=self.maximum_family,
        )

    def to_dict(self) -> dict[str, object]:
        """
        Return the family as a canonical-friendly mapping.
        """
        return {
            "members": list(self.members),
            "maximum_family": self.maximum_family,
        }

    @property
    def digest(self) -> str:
        """
        Return the `sha256:<hex>` digest of the family over its declared fields.
        """
        return _digest(self.to_dict())


@dataclass(frozen=True)
class ScreenOutcome:
    """
    What a screen actually evaluated over the family it declared.

    The evaluated tests are the ones that ran, and the survivors are the ones that passed every
    gate. Both are subsets of the enumerated family, and a test that never ran is recorded by its
    absence rather than by an assumption: a pair whose data is gapped or whose history begins after
    the window is not a test that ran and returned nothing. The correction is computed over the
    evaluated tests rather than over the survivors, because a selection is not a statistic until
    the trials that produced it are counted.

    Parameters
    ----------
    family : ScreenFamily
        The declared family the screen enumerated before its gates ran.
    evaluated : tuple[tuple[str, str], ...], default ()
        The pairs the gates actually tested, unique, each a pair of the family.
    survivors : tuple[tuple[str, str], ...], default ()
        The pairs that passed every gate, unique, each one evaluated.

    Raises
    ------
    TypeError
        If a field has the wrong type.
    ValueError
        If a test is not a pair of the family, repeats, or a survivor was never evaluated.

    """

    family: ScreenFamily
    evaluated: tuple[tuple[str, str], ...] = ()
    survivors: tuple[tuple[str, str], ...] = ()

    def __post_init__(self) -> None:
        """
        Validate that the record only holds tests the family enumerated.
        """
        if not isinstance(self.family, ScreenFamily):
            raise TypeError(f"family must be a ScreenFamily, was {self.family!r}")

        for name in ("evaluated", "survivors"):
            values = getattr(self, name)
            if not isinstance(values, tuple):
                raise TypeError(f"{name} must be a tuple, was {values!r}")
            if len(set(values)) != len(values):
                raise ValueError(f"{name} must be unique, was {list(values)}")

        enumerated = set(self.family.pairs())
        unknown = sorted(set(self.evaluated) - enumerated)
        if unknown:
            raise ValueError(f"evaluated tests are not pairs of the family: {unknown}")

        unpassed = sorted(set(self.survivors) - set(self.evaluated))
        if unpassed:
            raise ValueError(f"survivors were not evaluated: {unpassed}")

    @property
    def family_size(self) -> int:
        """
        Return the number of tests the family enumerated before any gate ran.
        """
        return self.family.size

    @property
    def evaluated_count(self) -> int:
        """
        Return the number of tests the screen actually evaluated.
        """
        return len(self.evaluated)

    @property
    def survivor_count(self) -> int:
        """
        Return the number of tests that passed every gate.
        """
        return len(self.survivors)

    def significance(
        self,
        sample: SharpeSample,
        *,
        contract: StatisticalContract | None = None,
    ) -> SignificanceResult:
        """
        Return the correction recomputed from the recorded family.

        The sample's trial estimates have to correspond one-to-one with the tests the screen
        evaluated, so the dispersion the correction uses is the dispersion of the trials that ran
        rather than of the survivors, and a screen that evaluated fewer tests than it enumerated
        states that difference by the count rather than by a footnote.

        Parameters
        ----------
        sample : SharpeSample
            The correction's sample, whose trial estimates must match the evaluated tests.
        contract : StatisticalContract | None, default None
            The contract to compute under, or None for the declared default.

        Returns
        -------
        SignificanceResult

        Raises
        ------
        ValueError
            If the sample's trial count is not the number of tests the screen evaluated.

        """
        if sample.nominal_trials != self.evaluated_count:
            raise ValueError(
                f"the sample carries {sample.nominal_trials} trial estimates but the screen "
                f"evaluated {self.evaluated_count} tests",
            )

        return deflated_sharpe_ratio(sample, contract=contract)

    def to_dict(self) -> dict[str, object]:
        """
        Return the record as a canonical-friendly mapping.
        """
        return {
            "family": self.family.to_dict(),
            "family_size": self.family_size,
            "evaluated": [list(pair) for pair in self.evaluated],
            "survivors": [list(pair) for pair in self.survivors],
        }

    @property
    def digest(self) -> str:
        """
        Return the `sha256:<hex>` digest of the record over its declared fields.
        """
        return _digest(self.to_dict())
