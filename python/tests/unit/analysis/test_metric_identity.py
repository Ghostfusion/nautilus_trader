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
Tests for metric identity, metadata and status.
"""

import math

import pytest

from nautilus_trader.analysis import LongRatio
from nautilus_trader.analysis import MaxDrawdown
from nautilus_trader.analysis import MetricDirection
from nautilus_trader.analysis import MetricInput
from nautilus_trader.analysis import MetricReason
from nautilus_trader.analysis import MetricStatus
from nautilus_trader.analysis import MetricTag
from nautilus_trader.analysis import MetricUnits
from nautilus_trader.analysis import PortfolioAnalyzer
from nautilus_trader.analysis import PortfolioStatistic
from nautilus_trader.analysis import SharpeRatio


ONE_DAY_NS = 86_400_000_000_000
START_NS = 1_600_000_000_000_000_000


def _analyzer(*statistics: PortfolioStatistic) -> PortfolioAnalyzer:
    analyzer = PortfolioAnalyzer()
    for statistic in statistics:
        analyzer.register_statistic(statistic)

    return analyzer


def _add_returns(analyzer: PortfolioAnalyzer, values: list[float]) -> None:
    for i, value in enumerate(values):
        analyzer.add_return(START_NS + i * ONE_DAY_NS, value)


class DeclaredStatistic(PortfolioStatistic):
    """A user statistic that declares its metric identity."""

    metric_id = "declared_statistic"
    units = MetricUnits.FRACTION
    tags = (MetricTag.RETURNS,)
    direction = MetricDirection.MINIMIZE
    inputs = (MetricInput.RETURNS,)

    def calculate_from_returns(self, _returns: dict[int, float]) -> float:
        """Return a fixed value for the test."""
        return 0.5


class UndeclaredStatistic(PortfolioStatistic):
    """A user statistic that declares nothing, inheriting the base class declarations."""

    def calculate_from_returns(self, _returns: dict[int, float]) -> float:
        """Return a fixed value for the test."""
        return 0.25


class MisdeclaredStatistic(PortfolioStatistic):
    """A user statistic whose units declaration is not a MetricUnits value."""

    units = "fraction"


def test_base_class_derives_a_stable_identity_from_the_class_name() -> None:
    """
    Test the base class derives a stable identity from the class name.
    """
    statistic = DeclaredStatistic()

    assert statistic.metric_id == "declared_statistic"
    assert UndeclaredStatistic().metric_id == "undeclared_statistic"


def test_metric_report_distinguishes_the_four_states() -> None:
    """
    Test a single report distinguishes computed, unavailable and not registered.
    """
    analyzer = _analyzer(SharpeRatio(), MaxDrawdown(), LongRatio())
    _add_returns(analyzer, [0.02, -0.05, 0.01, -0.02])

    report = analyzer.report_returns_metrics(["max_drawdown", "long_ratio", "no_such_metric"])

    computed = report.get("max_drawdown")
    assert computed.status == MetricStatus.COMPUTED
    assert computed.value is not None
    assert computed.reason is None
    assert computed.title == "Max Drawdown"

    # A position statistic requested from a returns report is available but not applicable.
    unavailable = report.get("long_ratio")
    assert unavailable.status == MetricStatus.UNAVAILABLE
    assert unavailable.reason == MetricReason.UNSUPPORTED_INPUT
    assert unavailable.value is None

    # A metric outside the metric set is distinguishable from one that could not be computed.
    not_registered = report.get("no_such_metric")
    assert not_registered.status == MetricStatus.NOT_REGISTERED
    assert not_registered.reason == MetricReason.NOT_IN_METRIC_SET

    assert len(report) == 3
    assert len(report.with_status(MetricStatus.COMPUTED)) == 1
    assert len(report.with_status(MetricStatus.UNAVAILABLE)) == 1
    assert len(report.with_status(MetricStatus.NOT_REGISTERED)) == 1


def test_metric_report_separates_invalid_from_unavailable() -> None:
    """
    Test invalid and unavailable are distinguishable on the same metric.
    """
    analyzer = _analyzer(SharpeRatio())
    _add_returns(analyzer, [0.02, -0.05, 0.01, -0.02, math.nan])

    result = analyzer.report_returns_metrics(["sharpe_ratio"]).get("sharpe_ratio")

    # The inputs were present and defective, which is not an applicability judgement.
    assert result.status == MetricStatus.INVALID
    assert result.reason == MetricReason.NON_FINITE_INPUT
    assert result.value is None
    assert result.title == "Sharpe Ratio (252 days)"

    # The same metric with no inputs at all is unavailable, not invalid.
    analyzer_empty = _analyzer(SharpeRatio())
    unavailable = analyzer_empty.report_returns_metrics(["sharpe_ratio"]).get("sharpe_ratio")
    assert unavailable.status == MetricStatus.UNAVAILABLE
    assert unavailable.reason == MetricReason.INSUFFICIENT_DATA
    assert unavailable.status != result.status


def test_metric_report_addresses_a_metric_by_id_and_by_name() -> None:
    """
    Test a metric is addressable by its stable identity and by its display name.
    """
    analyzer = _analyzer(SharpeRatio())
    _add_returns(analyzer, [0.02, -0.05, 0.01, -0.02])

    by_id = analyzer.report_returns_metrics(["sharpe_ratio"]).get("sharpe_ratio")
    by_name = analyzer.report_returns_metrics(["Sharpe Ratio (252 days)"]).get("sharpe_ratio")

    assert by_id.id == by_name.id
    assert by_id.title == by_name.title
    assert by_id.status == by_name.status
    assert by_id.value == by_name.value


def test_metric_definitions_declare_the_metadata_of_every_registered_statistic() -> None:
    """
    Test every registered statistic declares its metric metadata.
    """
    analyzer = _analyzer(SharpeRatio(), MaxDrawdown())

    definitions = {definition.id: definition for definition in analyzer.metric_definitions()}
    assert set(definitions) == {"sharpe_ratio", "max_drawdown"}

    sharpe = definitions["sharpe_ratio"]
    assert sharpe.title == "Sharpe Ratio (252 days)"
    assert sharpe.title_template == "Sharpe Ratio ({annualisation} days)"
    assert sharpe.parameters == {"annualisation": "252"}
    assert sharpe.units == MetricUnits.RATIO
    assert MetricTag.RISK_ADJUSTED in sharpe.tags
    assert MetricTag.ANNUALISED in sharpe.tags
    assert sharpe.direction == MetricDirection.MAXIMIZE
    assert sharpe.target is None
    assert MetricInput.RETURNS in sharpe.inputs

    drawdown = definitions["max_drawdown"]
    assert drawdown.direction == MetricDirection.MAXIMIZE
    assert MetricTag.DRAWDOWN in drawdown.tags


def test_a_user_statistic_declares_its_own_metric_identity() -> None:
    """
    Test a user statistic's declarations reach the definition and a defaulted one still works.
    """
    analyzer = _analyzer(DeclaredStatistic(), UndeclaredStatistic())

    definitions = {definition.id: definition for definition in analyzer.metric_definitions()}

    declared = definitions["declared_statistic"]
    assert declared.title == "Declared Statistic"
    assert declared.units == MetricUnits.FRACTION
    assert declared.direction == MetricDirection.MINIMIZE
    assert declared.tags == [MetricTag.RETURNS]

    # A statistic that declares nothing inherits the base class declarations.
    undeclared = definitions["undeclared_statistic"]
    assert undeclared.units == MetricUnits.RATIO
    assert undeclared.direction == MetricDirection.INFORMATIONAL

    _add_returns(analyzer, [0.02, -0.05, 0.01, -0.02])
    report = analyzer.report_returns_metrics(["declared_statistic", "undeclared_statistic"])

    assert report.get("declared_statistic").value == 0.5
    assert report.get("undeclared_statistic").value == 0.25


def test_a_misdeclared_statistic_is_rejected_at_registration() -> None:
    """
    Test a declaration outside the vocabulary is rejected at registration.
    """
    analyzer = PortfolioAnalyzer()

    with pytest.raises(ValueError, match="`units` must be a MetricUnits"):
        analyzer.register_statistic(MisdeclaredStatistic())


def test_a_duck_typed_statistic_registers_with_a_derived_definition() -> None:
    """
    Test a statistic declaring nothing still registers, with a marked derived definition.
    """

    class Standalone:
        name = "Category Sentinel"

        def calculate_from_returns(self, _returns: dict[int, float]) -> float:
            return 0.5

    analyzer = PortfolioAnalyzer()
    analyzer.register_statistic(Standalone())

    definition = analyzer.metric_definitions()[0]
    assert definition.id == "category_sentinel"
    assert definition.title == "Category Sentinel"
    assert definition.is_derived is True
    assert definition.direction == MetricDirection.INFORMATIONAL

    # A statistic whose declarations are inherited from the base class is declared, not derived.
    declaring = _analyzer(UndeclaredStatistic())
    assert declaring.metric_definitions()[0].is_derived is False

    _add_returns(declaring, [0.02, -0.05])
    report = declaring.report_returns_metrics(["undeclared_statistic"])
    assert report.get("undeclared_statistic").status == MetricStatus.COMPUTED


def test_metric_vocabularies_are_closed() -> None:
    """
    Test the metric vocabularies are closed and their members are distinct.
    """
    assert MetricUnits.RATIO != MetricUnits.FRACTION
    assert MetricUnits.RATIO == MetricUnits.RATIO
    assert MetricStatus.COMPUTED != MetricStatus.INVALID
    assert MetricStatus.UNAVAILABLE != MetricStatus.NOT_REGISTERED
    assert MetricDirection.TARGET != MetricDirection.MAXIMIZE
    assert MetricTag.RISK_ADJUSTED != MetricTag.ANNUALISED
    assert MetricInput.BENCHMARK != MetricInput.POSITIONS
    assert MetricReason.NON_FINITE_INPUT != MetricReason.UNDEFINED_RESULT

    # A member outside the vocabulary does not exist, on any of the six sets.
    for vocabulary, attribute in (
        (MetricUnits, "UNKNOWN"),
        (MetricTag, "UNKNOWN"),
        (MetricDirection, "UNKNOWN"),
        (MetricInput, "UNKNOWN"),
        (MetricStatus, "UNKNOWN"),
        (MetricReason, "UNKNOWN"),
    ):
        with pytest.raises(AttributeError):
            getattr(vocabulary, attribute)
