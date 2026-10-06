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
Base class for user-defined portfolio statistics.
"""

from __future__ import annotations

import re
from typing import TYPE_CHECKING

from nautilus_trader._libnautilus.analysis import MetricDirection
from nautilus_trader._libnautilus.analysis import MetricInput
from nautilus_trader._libnautilus.analysis import MetricStage
from nautilus_trader._libnautilus.analysis import MetricTag
from nautilus_trader._libnautilus.analysis import MetricUnits


if TYPE_CHECKING:
    from nautilus_trader.model import Position


class PortfolioStatistic:
    """
    The base class for all portfolio performance statistics.

    Subclass this and override the calculation methods for the input categories the
    statistic supports. The analyzer feeds each category separately, so a statistic
    contributes a value only where it overrides the matching method.

    Register an implementation with `Portfolio.register_statistic()` to include it in
    `Portfolio.statistics()`, backtest results, and post-run analysis logs.

    Notes
    -----
    The return value must be a float, or ``None`` when the statistic is undefined for the
    given data.

    """

    @property
    def name(self) -> str:
        """
        Return the name for the statistic.

        The default splits the class name on word boundaries, so `MyCustomRatio` becomes
        "My Custom Ratio". Override this to choose the name directly.

        Returns
        -------
        str

        """
        klass = type(self).__name__
        matches = re.finditer(".+?(?:(?<=[a-z])(?=[A-Z])|(?<=[A-Z])(?=[A-Z][a-z])|$)", klass)
        return " ".join([m.group(0) for m in matches])

    @property
    def metric_id(self) -> str:
        """
        Return the stable machine-facing identity for the metric.

        The default is the class name in snake_case, so `MyCustomRatio` becomes
        `my_custom_ratio`. Override this to pin an identity that must not move when the class
        is renamed, because a result reports this id rather than the display name.

        Returns
        -------
        str

        """
        klass = type(self).__name__
        matches = re.finditer(".+?(?:(?<=[a-z])(?=[A-Z])|(?<=[A-Z])(?=[A-Z][a-z])|$)", klass)
        return "_".join([m.group(0).lower() for m in matches])

    @property
    def units(self) -> MetricUnits:
        """
        Return the units the value is expressed in.

        The default is `MetricUnits.RATIO`. Override this to declare what the value means.

        Returns
        -------
        MetricUnits

        """
        return MetricUnits.RATIO

    @property
    def tags(self) -> tuple[MetricTag, ...]:
        """
        Return the cross-cutting tags for the metric.

        The default is empty. Override this to declare the facets a report consumer can
        select or group by.

        Returns
        -------
        tuple[MetricTag, ...]

        """
        return ()

    @property
    def direction(self) -> MetricDirection:
        """
        Return the direction in which a consumer rewards the value.

        The default is `MetricDirection.INFORMATIONAL`, which claims no direction. Override
        this to declare that a larger or smaller value is preferred, or that a target value
        is preferred together with `target`.

        Returns
        -------
        MetricDirection

        """
        return MetricDirection.INFORMATIONAL

    @property
    def target(self) -> float | None:
        """
        Return the target value for a `MetricDirection.TARGET` metric.

        The default is `None`. It is only read when `direction` is `MetricDirection.TARGET`.

        Returns
        -------
        float or ``None``

        """
        return None

    @property
    def inputs(self) -> tuple[MetricInput, ...]:
        """
        Return the inputs the definition requires.

        The default declares every input category the analyzer feeds. Override this to narrow
        it, so a metric is reported as unavailable rather than omitted when the source it
        needs was not supplied.

        Returns
        -------
        tuple[MetricInput, ...]

        """
        return (
            MetricInput.RETURNS,
            MetricInput.REALIZED_PNLS,
            MetricInput.POSITIONS,
        )

    @property
    def stage(self) -> MetricStage | None:
        """
        Return the scoring-chain stage the metric belongs to.

        The default is `None`, which claims no stage. Override this to declare whether the
        metric is a `MetricStage.FORECAST` score, a `MetricStage.DECISION` trade outcome, or
        an `MetricStage.ACCOUNT` ledger figure, so a report can say which it is reading.

        Returns
        -------
        MetricStage or ``None``

        """
        return None

    def calculate_from_returns(self, returns: dict[int, float]) -> float | None:
        """
        Calculate the statistic value from the given returns.

        Parameters
        ----------
        returns : dict[int, float]
            The returns keyed by UNIX timestamp (nanoseconds).

        Returns
        -------
        float or ``None``

        """
        # Override in implementation

    def calculate_from_realized_pnls(self, realized_pnls: list[float]) -> float | None:
        """
        Calculate the statistic value from the given realized PnLs.

        Parameters
        ----------
        realized_pnls : list[float]
            The realized PnLs for one currency, in ascending event-time order.

        Returns
        -------
        float or ``None``

        """
        # Override in implementation

    def calculate_from_positions(self, positions: list[Position]) -> float | None:
        """
        Calculate the statistic value from the given positions.

        Parameters
        ----------
        positions : list[Position]
            The positions to use for the calculation.

        Returns
        -------
        float or ``None``

        """
        # Override in implementation

    def calculate_from_returns_with_benchmark(
        self,
        returns: dict[int, float],
        benchmark: dict[int, float],
    ) -> float | None:
        """
        Calculate the statistic value from the given returns relative to a benchmark.

        Parameters
        ----------
        returns : dict[int, float]
            The strategy returns keyed by UNIX timestamp (nanoseconds).
        benchmark : dict[int, float]
            The benchmark returns keyed by UNIX timestamp (nanoseconds).

        Returns
        -------
        float or ``None``

        """
        # Override in implementation
