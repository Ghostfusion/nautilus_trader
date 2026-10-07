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
Test analysis behavior.
"""

import math
import sys

import pytest

import nautilus_trader.analysis as analysis_module
from nautilus_trader.analysis import CAGR
from nautilus_trader.analysis import Alpha
from nautilus_trader.analysis import Autocorrelation
from nautilus_trader.analysis import AvgLoser
from nautilus_trader.analysis import AvgWinner
from nautilus_trader.analysis import BetaRatio
from nautilus_trader.analysis import BreakevenCost
from nautilus_trader.analysis import CalmarRatio
from nautilus_trader.analysis import CorrectionImpactReport
from nautilus_trader.analysis import CostBasisPoints
from nautilus_trader.analysis import DetectorReport
from nautilus_trader.analysis import DownCaptureRatio
from nautilus_trader.analysis import Expectancy
from nautilus_trader.analysis import ExpectedShortfall
from nautilus_trader.analysis import ExponentiallyWeightedSharpe
from nautilus_trader.analysis import GrossReturn
from nautilus_trader.analysis import InformationRatio
from nautilus_trader.analysis import LongRatio
from nautilus_trader.analysis import MaxDrawdown
from nautilus_trader.analysis import MaxDrawdownDuration
from nautilus_trader.analysis import MaxLoser
from nautilus_trader.analysis import MaxWinner
from nautilus_trader.analysis import MetricStage
from nautilus_trader.analysis import MinLoser
from nautilus_trader.analysis import MinWinner
from nautilus_trader.analysis import NetReturn
from nautilus_trader.analysis import OmegaRatio
from nautilus_trader.analysis import PortfolioAnalyzer
from nautilus_trader.analysis import PortfolioStatistic
from nautilus_trader.analysis import ProfitFactor
from nautilus_trader.analysis import RescaledRange
from nautilus_trader.analysis import ReturnsAverage
from nautilus_trader.analysis import ReturnsAverageLoss
from nautilus_trader.analysis import ReturnsAverageWin
from nautilus_trader.analysis import ReturnsKurtosis
from nautilus_trader.analysis import ReturnsSkewness
from nautilus_trader.analysis import ReturnsVolatility
from nautilus_trader.analysis import RiskReturnRatio
from nautilus_trader.analysis import SharpeRatio
from nautilus_trader.analysis import SortinoRatio
from nautilus_trader.analysis import TailRatio
from nautilus_trader.analysis import TotalCommissions
from nautilus_trader.analysis import TotalTurnover
from nautilus_trader.analysis import TrackingError
from nautilus_trader.analysis import TreynorRatio
from nautilus_trader.analysis import UlcerIndex
from nautilus_trader.analysis import UpCaptureRatio
from nautilus_trader.analysis import ValueAtRisk
from nautilus_trader.analysis import VarianceRatio
from nautilus_trader.analysis import WinRate
from nautilus_trader.analysis import tearsheet
from nautilus_trader.model import Currency
from nautilus_trader.model import Money
from nautilus_trader.model import Position
from nautilus_trader.model import PositionId
from tests.providers import TestInstrumentProvider
from tests.unit.model.factories import make_position_fill


NO_ARG_STATISTICS = [
    (AvgLoser, "Avg Loser"),
    (AvgWinner, "Avg Winner"),
    (BreakevenCost, "Breakeven Cost"),
    (CostBasisPoints, "Cost (basis points"),
    (Expectancy, "Expectancy"),
    (GrossReturn, "Gross Return"),
    (LongRatio, "Long Ratio"),
    (MaxDrawdown, "Max Drawdown"),
    (MaxDrawdownDuration, "Max Drawdown Duration"),
    (MaxLoser, "Max Loser"),
    (MaxWinner, "Max Winner"),
    (MinLoser, "Min Loser"),
    (MinWinner, "Min Winner"),
    (NetReturn, "Net Return"),
    (ProfitFactor, "Profit Factor"),
    (ReturnsAverage, "Average (Return"),
    (ReturnsAverageLoss, "Average Loss (Return"),
    (ReturnsAverageWin, "Average Win (Return"),
    (ReturnsKurtosis, "Returns Kurtosis"),
    (ReturnsSkewness, "Returns Skewness"),
    (RiskReturnRatio, "Risk Return Ratio"),
    (TailRatio, "Tail Ratio"),
    (TotalCommissions, "Total Commissions"),
    (TotalTurnover, "Total Turnover"),
    (UlcerIndex, "Ulcer Index"),
    (WinRate, "Win Rate"),
    (Autocorrelation, "Autocorrelation"),
    (VarianceRatio, "Variance Ratio"),
    (RescaledRange, "Rescaled Range"),
]

PERIOD_STATISTICS = [
    (CAGR, "CAGR"),
    (CalmarRatio, "Calmar Ratio"),
    (ReturnsVolatility, "Returns Volatility"),
    (SharpeRatio, "Sharpe Ratio"),
    (SortinoRatio, "Sortino Ratio"),
]

# Statistics carrying defaulted integer parameters that are not a single period.
PARAMETERISED_STATISTICS = [
    (ExponentiallyWeightedSharpe, "Exponentially Weighted Sharpe"),
]

# Statistics carrying a single float parameter (threshold / confidence).
THRESHOLD_STATISTICS = [
    (OmegaRatio, "Omega Ratio"),
    (ValueAtRisk, "Value at Risk"),
    (ExpectedShortfall, "Expected Shortfall"),
]

BENCHMARK_STATISTICS = [
    (Alpha, "Alpha"),
    (BetaRatio, "Beta"),
    (DownCaptureRatio, "Down Capture Ratio"),
    (InformationRatio, "Information Ratio"),
    (TrackingError, "Tracking Error"),
    (TreynorRatio, "Treynor Ratio"),
    (UpCaptureRatio, "Up Capture Ratio"),
]

# Statistics that carry the declared outcome metric a correction is measured against, or the
# confusion matrix a detector report is built from, so they are not constructible with no
# arguments.
REPORT_STATISTICS = [
    (CorrectionImpactReport, "Correction Impact"),
    (DetectorReport, "Detector Report"),
]

ALL_STATISTICS = (
    NO_ARG_STATISTICS
    + PERIOD_STATISTICS
    + PARAMETERISED_STATISTICS
    + THRESHOLD_STATISTICS
    + BENCHMARK_STATISTICS
    + REPORT_STATISTICS
)
STATISTIC_METHODS = (
    "calculate_from_positions",
    "calculate_from_realized_pnls",
    "calculate_from_returns",
)
# `PortfolioStatistic` shares the calculation surface but is the extension point for
# user-defined statistics, not a built-in, so it stays out of the built-in inventory.
EXPOSED_STATISTICS = sorted(
    (
        value
        for value in vars(analysis_module).values()
        if isinstance(value, type)
        and value is not PortfolioStatistic
        and all(hasattr(value, method) for method in STATISTIC_METHODS)
    ),
    key=lambda cls: cls.__name__,
)


def _construct_exposed_statistic(cls: object) -> object:
    # Every exposed statistic is constructible with no arguments except the two reports: the
    # correction-impact report requires the declared outcome metric it reports, and the detector
    # report requires the confusion matrix it reads.
    if cls is CorrectionImpactReport:
        return CorrectionImpactReport("sharpe_ratio", 1.0, 0.5)
    if cls is DetectorReport:
        return DetectorReport(1, 9, 989, 1)
    return cls()


@pytest.mark.parametrize(("cls", "expected_prefix"), NO_ARG_STATISTICS)
def test_statistic_construction_and_name(cls: object, expected_prefix: object) -> None:
    """
    Test statistic construction and name.
    """
    stat = cls()

    assert stat.name.startswith(expected_prefix)


@pytest.mark.parametrize(("cls", "expected_prefix"), PERIOD_STATISTICS)
def test_period_statistic_default_construction_and_name(
    cls: object,
    expected_prefix: object,
) -> None:
    """
    Test period statistic default construction and name.
    """
    stat = cls()

    assert stat.name.startswith(expected_prefix)


@pytest.mark.parametrize(("cls", "expected_prefix"), PERIOD_STATISTICS)
def test_period_statistic_custom_period(cls: object, expected_prefix: object) -> None:
    """
    Test period statistic custom period.
    """
    stat = cls(period=30)

    assert "30" in stat.name


@pytest.mark.parametrize(("cls", "expected_prefix"), PARAMETERISED_STATISTICS)
def test_parameterised_statistic_default_construction_and_name(
    cls: object,
    expected_prefix: object,
) -> None:
    """
    Test parameterised statistic default construction and name.
    """
    stat = cls()

    assert stat.name.startswith(expected_prefix)


@pytest.mark.parametrize(("cls", "expected_prefix"), PARAMETERISED_STATISTICS)
def test_parameterised_statistic_custom_parameters(cls: object, expected_prefix: object) -> None:
    """
    Test parameterised statistic custom parameters.
    """
    stat = cls(annualisation=365, halflife=10)

    assert stat.name.startswith(expected_prefix)
    assert "365" in stat.name
    assert "10" in stat.name


@pytest.mark.parametrize(("cls", "expected_prefix"), THRESHOLD_STATISTICS)
def test_threshold_statistic_default_construction_and_name(
    cls: object,
    expected_prefix: object,
) -> None:
    """
    Test threshold statistic default construction and name.
    """
    stat = cls()

    assert stat.name.startswith(expected_prefix)


@pytest.mark.parametrize("cls", [ValueAtRisk, ExpectedShortfall])
@pytest.mark.parametrize("confidence", [0.0, 1.0, 1.5, -0.5, float("nan"), float("inf")])
def test_confidence_statistic_rejects_invalid_confidence(cls: object, confidence: object) -> None:
    """
    Test confidence statistic rejects invalid confidence.
    """
    # `confidence` must be finite and in the open interval (0, 1); otherwise the
    # historical percentile index would be out of range.
    with pytest.raises(ValueError, match="confidence must be finite"):
        cls(confidence=confidence)


@pytest.mark.parametrize("threshold", [float("nan"), float("inf"), float("-inf")])
def test_omega_ratio_rejects_non_finite_threshold(threshold: object) -> None:
    """
    Test omega ratio rejects non finite threshold.
    """
    # `threshold` has no natural range but must be finite; a non-finite value
    # would silently poison the gain/loss split.
    with pytest.raises(ValueError, match="threshold must be finite"):
        OmegaRatio(threshold=threshold)


@pytest.mark.parametrize(
    ("cls", "_expected_prefix"),
    ALL_STATISTICS,
)
def test_pyo3_statistic_exposes_full_calculate_surface(
    cls: object,
    _expected_prefix: object,
) -> None:
    """
    Test pyo3 statistic exposes full calculate surface.
    """
    stat = _construct_exposed_statistic(cls)

    # Every pyo3 statistic must expose all three calculate_from_* methods so the
    # Python PortfolioAnalyzer can iterate registered stats without AttributeError.
    # Methods return None for inputs that do not apply to the underlying calculation.
    assert callable(stat.calculate_from_returns)
    assert callable(stat.calculate_from_realized_pnls)
    assert callable(stat.calculate_from_positions)


def test_long_ratio_custom_precision() -> None:
    """
    Test long ratio custom precision.
    """
    stat = LongRatio(precision=4)

    assert stat.name.startswith("Long Ratio")


def test_portfolio_analyzer_construction() -> None:
    """
    Test portfolio analyzer construction.
    """
    analyzer = PortfolioAnalyzer()

    assert analyzer.currencies() == []
    assert analyzer.returns() == {}
    assert analyzer.position_returns() == {}
    assert analyzer.portfolio_returns() == {}


def test_exposed_statistic_inventory_matches_constructor_matrix() -> None:
    """
    Test exposed statistic inventory matches constructor matrix.
    """
    expected = {cls for cls, _expected_prefix in ALL_STATISTICS}

    assert EXPOSED_STATISTICS
    assert set(EXPOSED_STATISTICS) == expected


@pytest.mark.parametrize("cls", EXPOSED_STATISTICS)
def test_portfolio_analyzer_register_and_deregister_statistic(cls: object) -> None:
    """
    Test portfolio analyzer register and deregister statistic.
    """
    analyzer = PortfolioAnalyzer()
    stat = _construct_exposed_statistic(cls)

    analyzer.register_statistic(stat)

    assert analyzer.statistic(stat.name) is not None

    analyzer.deregister_statistic(stat)

    assert analyzer.statistic(stat.name) is None


def test_portfolio_analyzer_adds_native_position() -> None:
    """
    Test portfolio analyzer adds native position.
    """
    analyzer = PortfolioAnalyzer()
    instrument = TestInstrumentProvider.audusd_sim()
    position = Position(instrument=instrument, fill=make_position_fill(instrument))
    analyzer.register_statistic(LongRatio())

    analyzer.add_positions([position])

    assert analyzer.get_performance_stats_general() == {"Long Ratio": 1.0}


def test_long_ratio_calculates_from_native_positions() -> None:
    """
    Test long ratio calculates from native positions.
    """
    instrument = TestInstrumentProvider.audusd_sim()
    position = Position(instrument=instrument, fill=make_position_fill(instrument))

    assert LongRatio().calculate_from_positions([position]) == 1.0
    assert LongRatio().calculate_from_positions([]) is None


class DuckTypedPosition:
    """
    The attribute shape the removed getattr-based extraction accepted.
    """

    entry = 1  # `OrderSide.BUY`


def test_long_ratio_rejects_non_position_objects() -> None:
    """
    Test long ratio rejects non position objects.
    """
    with pytest.raises(TypeError, match="Position"):
        LongRatio().calculate_from_positions([DuckTypedPosition()])


@pytest.mark.parametrize("cls", EXPOSED_STATISTICS)
def test_statistic_calculate_from_positions_rejects_non_position_objects(cls: object) -> None:
    """
    Test statistic calculate from positions rejects non position objects.
    """
    with pytest.raises(TypeError, match="Position"):
        _construct_exposed_statistic(cls).calculate_from_positions([DuckTypedPosition()])


def test_undefined_cagr_and_calmar_ratio_return_nan() -> None:
    """
    Test undefined cagr and calmar ratio return nan.
    """
    nanos_per_day = 86_400_000_000_000
    returns = {
        day * nanos_per_day: value for day, value in enumerate([-1.5, 0.0, 0.0, 0.0, 0.0], start=1)
    }

    cagr = CAGR().calculate_from_returns(returns)
    calmar_ratio = CalmarRatio().calculate_from_returns(returns)

    assert math.isnan(cagr)
    assert math.isnan(calmar_ratio)


def test_portfolio_analyzer_deregister_all_statistics() -> None:
    """
    Test portfolio analyzer deregister all statistics.
    """
    analyzer = PortfolioAnalyzer()
    analyzer.register_statistic(SharpeRatio())
    analyzer.register_statistic(WinRate())

    analyzer.deregister_statistics()

    assert analyzer.get_performance_stats_returns() == {}


def test_portfolio_analyzer_add_return_and_stats() -> None:
    """
    Test portfolio analyzer add return and stats.
    """
    analyzer = PortfolioAnalyzer()
    analyzer.register_statistic(ReturnsAverage())

    analyzer.add_return(1_000_000_000, 0.01)
    analyzer.add_return(2_000_000_000, -0.005)

    stats = analyzer.get_performance_stats_returns()

    assert len(stats) > 0


def test_portfolio_analyzer_add_position_return() -> None:
    """
    Test portfolio analyzer add position return.
    """
    analyzer = PortfolioAnalyzer()

    analyzer.add_position_return(1_000_000_000, 0.02)

    assert analyzer.position_returns() != {}


def test_portfolio_analyzer_reset() -> None:
    """
    Test portfolio analyzer reset.
    """
    analyzer = PortfolioAnalyzer()
    analyzer.add_return(1_000_000_000, 0.01)

    analyzer.reset()

    assert analyzer.returns() == {}
    assert analyzer.position_returns() == {}


def test_portfolio_analyzer_formatted_stats_empty() -> None:
    """
    Test portfolio analyzer formatted stats empty.
    """
    analyzer = PortfolioAnalyzer()

    assert analyzer.get_stats_returns_formatted() == []
    assert analyzer.get_stats_position_returns_formatted() == []
    assert analyzer.get_stats_portfolio_returns_formatted() == []
    assert analyzer.get_stats_general_formatted() == []


def test_portfolio_analyzer_realized_pnls_drops_recorded_snapshot_alias() -> None:
    """
    Test portfolio analyzer realized pnls drops recorded snapshot alias.
    """
    analyzer = PortfolioAnalyzer()
    usd = Currency.from_str("USD")
    position_id = PositionId("P-1")
    snapshot_id = PositionId(f"{position_id.value}-00000000-0000-4000-8000-000000000001")

    analyzer.add_trade(snapshot_id, 1, Money(10.0, usd))
    analyzer.record_trade(position_id, 1, Money(10.0, usd))

    pnls = analyzer.realized_pnls(usd)

    assert pnls == [(position_id.value, 1, 10.0)]


def test_portfolio_analyzer_realized_pnls_drops_recorded_snapshot_alias_without_timestamp() -> None:
    """
    Drop a recorded snapshot alias that has no timestamp.
    """
    analyzer = PortfolioAnalyzer()
    usd = Currency.from_str("USD")
    position_id = PositionId("P-1")
    snapshot_id = PositionId(f"{position_id.value}-00000000-0000-4000-8000-000000000001")

    analyzer.add_trade(snapshot_id, 0, Money(10.0, usd))
    analyzer.record_trade(position_id, 0, Money(10.0, usd))

    pnls = analyzer.realized_pnls(usd)

    assert pnls == [(position_id.value, 0, 10.0)]


def test_portfolio_analyzer_realized_pnls_keeps_unrecorded_snapshot_cycle() -> None:
    """
    Test portfolio analyzer realized pnls keeps unrecorded snapshot cycle.
    """
    analyzer = PortfolioAnalyzer()
    usd = Currency.from_str("USD")
    position_id = PositionId("P-1")
    snapshot_id = PositionId(f"{position_id.value}-00000000-0000-4000-8000-000000000001")

    analyzer.add_trade(snapshot_id, 1, Money(10.0, usd))
    analyzer.record_trade(position_id, 2, Money(25.0, usd))

    pnls = analyzer.realized_pnls(usd)

    assert pnls == [(snapshot_id.value, 1, 10.0), (position_id.value, 2, 25.0)]


class TradeCount(PortfolioStatistic):
    """
    Counts closed trades from the realized PnLs it is fed.
    """

    def calculate_from_realized_pnls(self, realized_pnls: list[float]) -> float | None:
        """
        Return the number of closed trades.
        """
        return float(len(realized_pnls))


class ReturnsSum(PortfolioStatistic):
    """
    Sums the returns it is fed.
    """

    def calculate_from_returns(self, returns: dict[int, float]) -> float | None:
        """
        Return the sum of the returns.
        """
        return sum(returns.values())


class PositionCount(PortfolioStatistic):
    """
    Counts the positions it is fed.
    """

    def calculate_from_positions(self, positions: list) -> float | None:
        """
        Return the number of positions.
        """
        return float(len(positions))


def test_custom_statistic_derives_name_from_class_name() -> None:
    """
    Test custom statistic derives name from class name.
    """
    assert TradeCount().name == "Trade Count"
    assert ReturnsSum().name == "Returns Sum"
    assert PositionCount().name == "Position Count"


def test_custom_statistic_name_can_be_overridden() -> None:
    """
    Test custom statistic name can be overridden.
    """

    class Renamed(PortfolioStatistic):
        @property
        def name(self) -> str:
            return "My Metric"

    analyzer = PortfolioAnalyzer()
    analyzer.register_statistic(Renamed())

    assert analyzer.statistic("My Metric") == "My Metric"


def test_custom_statistic_calculates_from_returns() -> None:
    """
    Test custom statistic calculates from returns.
    """
    analyzer = PortfolioAnalyzer()
    analyzer.register_statistic(ReturnsSum())
    analyzer.add_return(1, 0.25)
    analyzer.add_return(2, 0.75)

    assert analyzer.get_performance_stats_returns() == {"Returns Sum": 1.0}


def test_custom_statistic_calculates_from_realized_pnls() -> None:
    """
    Test custom statistic calculates from realized pnls.
    """
    analyzer = PortfolioAnalyzer()
    usd = Currency.from_str("USD")
    analyzer.register_statistic(TradeCount())
    analyzer.add_trade(PositionId("P-1"), 1, Money(10.0, usd))
    analyzer.add_trade(PositionId("P-2"), 2, Money(-4.0, usd))

    stats = analyzer.get_performance_stats_pnls(usd, None)

    assert stats["Trade Count"] == 2.0


def test_custom_statistic_runs_on_pnls_without_any_trades() -> None:
    """
    Test custom statistic runs on pnls without any trades.
    """
    analyzer = PortfolioAnalyzer()
    usd = Currency.from_str("USD")
    analyzer.register_statistic(TradeCount())

    stats = analyzer.get_performance_stats_pnls(usd, None)

    assert stats["Trade Count"] == 0.0


def test_pnl_statistics_reject_unresolved_currency() -> None:
    """
    Test pnl statistics reject unresolved currency.
    """
    analyzer = PortfolioAnalyzer()
    analyzer.register_statistic(TradeCount())
    analyzer.add_trade(PositionId("P-USD"), 1, Money(10.0, Currency.from_str("USD")))
    analyzer.add_trade(PositionId("P-EUR"), 2, Money(5.0, Currency.from_str("EUR")))

    with pytest.raises(ValueError, match="Currency must be specified"):
        analyzer.get_performance_stats_pnls(None, None)


def test_custom_statistic_calculates_from_positions() -> None:
    """
    Test custom statistic calculates from positions.
    """
    analyzer = PortfolioAnalyzer()
    instrument = TestInstrumentProvider.audusd_sim()
    position = Position(instrument=instrument, fill=make_position_fill(instrument))
    analyzer.register_statistic(PositionCount())

    analyzer.add_positions([position])

    assert analyzer.get_performance_stats_general() == {"Position Count": 1.0}


def test_custom_statistic_receives_native_positions() -> None:
    """
    Test custom statistic receives native positions.
    """
    received: list = []

    class CapturesPositions(PortfolioStatistic):
        def calculate_from_positions(self, positions: list) -> float | None:
            received.extend(positions)
            return float(len(positions))

    analyzer = PortfolioAnalyzer()
    instrument = TestInstrumentProvider.audusd_sim()
    position = Position(instrument=instrument, fill=make_position_fill(instrument))
    analyzer.register_statistic(CapturesPositions())
    analyzer.add_positions([position])

    analyzer.get_performance_stats_general()

    assert len(received) == 1
    assert isinstance(received[0], Position)
    assert received[0].id == position.id


def test_custom_statistic_calculates_from_returns_with_benchmark() -> None:
    """
    Test custom statistic calculates from returns with benchmark.
    """

    class ExcessReturn(PortfolioStatistic):
        def calculate_from_returns_with_benchmark(
            self,
            returns: dict[int, float],
            benchmark: dict[int, float],
        ) -> float | None:
            return sum(returns.values()) - sum(benchmark.values())

    analyzer = PortfolioAnalyzer()
    analyzer.register_statistic(ExcessReturn())
    analyzer.add_return(1, 0.5)

    stats = analyzer.get_performance_stats_returns_vs_benchmark({1: 0.2})

    assert stats == {"Excess Return": pytest.approx(0.3)}


def test_custom_statistic_unsupported_category_contributes_no_value() -> None:
    """
    Test custom statistic unsupported category contributes no value.
    """
    analyzer = PortfolioAnalyzer()
    analyzer.register_statistic(PositionCount())
    analyzer.add_return(1, 0.25)

    assert analyzer.get_performance_stats_returns() == {}
    assert analyzer.get_performance_stats_returns_vs_benchmark({1: 0.1}) == {}


def test_custom_statistic_replaces_statistic_with_matching_name() -> None:
    """
    Test custom statistic replaces statistic with matching name.
    """

    class FirstImpl(PortfolioStatistic):
        @property
        def name(self) -> str:
            return "Shared Name"

        def calculate_from_returns(self, _returns: dict[int, float]) -> float | None:
            return 1.0

    class SecondImpl(PortfolioStatistic):
        @property
        def name(self) -> str:
            return "Shared Name"

        def calculate_from_returns(self, _returns: dict[int, float]) -> float | None:
            return 99.0

    analyzer = PortfolioAnalyzer()
    analyzer.register_statistic(FirstImpl())
    analyzer.register_statistic(SecondImpl())
    analyzer.add_return(1, 0.25)

    assert analyzer.get_performance_stats_returns() == {"Shared Name": 99.0}


def test_custom_statistic_deregisters_by_name() -> None:
    """
    Test custom statistic deregisters by name.
    """
    analyzer = PortfolioAnalyzer()
    analyzer.register_statistic(ReturnsSum())
    analyzer.add_return(1, 0.25)

    analyzer.deregister_statistic(ReturnsSum())

    assert analyzer.statistic("Returns Sum") is None
    assert analyzer.get_performance_stats_returns() == {}


def test_custom_statistic_deregister_unregistered_is_noop() -> None:
    """
    Test custom statistic deregister unregistered is noop.
    """
    analyzer = PortfolioAnalyzer()
    analyzer.register_statistic(ReturnsSum())

    analyzer.deregister_statistic(TradeCount())

    assert analyzer.statistic("Returns Sum") == "Returns Sum"


def test_custom_statistic_retained_across_reset() -> None:
    """
    Test custom statistic retained across reset.
    """
    analyzer = PortfolioAnalyzer()
    analyzer.register_statistic(ReturnsSum())
    analyzer.add_return(1, 0.25)

    analyzer.reset()
    analyzer.add_return(2, 0.75)

    assert analyzer.get_performance_stats_returns() == {"Returns Sum": 0.75}


def test_custom_statistic_error_does_not_block_other_statistics() -> None:
    """
    Test custom statistic error does not block other statistics.
    """

    class Raises(PortfolioStatistic):
        def calculate_from_returns(self, _returns: dict[int, float]) -> float | None:
            raise RuntimeError("boom")

    analyzer = PortfolioAnalyzer()
    analyzer.register_statistic(Raises())
    analyzer.register_statistic(ReturnsSum())
    analyzer.add_return(1, 0.25)

    assert analyzer.get_performance_stats_returns() == {"Returns Sum": 0.25}


def test_custom_statistic_error_is_reported_as_unraisable() -> None:
    """
    Test custom statistic error is reported as unraisable.
    """

    class Raises(PortfolioStatistic):
        def calculate_from_returns(self, _returns: dict[int, float]) -> float | None:
            raise RuntimeError("boom")

    captured: list = []
    original_hook = sys.unraisablehook
    sys.unraisablehook = captured.append
    try:
        analyzer = PortfolioAnalyzer()
        analyzer.register_statistic(Raises())
        analyzer.add_return(1, 0.25)
        analyzer.get_performance_stats_returns()
    finally:
        sys.unraisablehook = original_hook

    assert [str(c.exc_value) for c in captured] == ["boom"]


def test_custom_statistic_non_numeric_value_is_skipped() -> None:
    """
    Test custom statistic non numeric value is skipped.
    """

    class ReturnsText(PortfolioStatistic):
        def calculate_from_returns(self, _returns: dict[int, float]) -> float | None:
            return "not a number"  # type: ignore[return-value]

    analyzer = PortfolioAnalyzer()
    analyzer.register_statistic(ReturnsText())
    analyzer.register_statistic(ReturnsSum())
    analyzer.add_return(1, 0.25)

    assert analyzer.get_performance_stats_returns() == {"Returns Sum": 0.25}


def test_custom_statistic_none_value_is_skipped() -> None:
    """
    Test custom statistic none value is skipped.
    """

    class Undefined(PortfolioStatistic):
        def calculate_from_returns(self, _returns: dict[int, float]) -> float | None:
            return None

    analyzer = PortfolioAnalyzer()
    analyzer.register_statistic(Undefined())
    analyzer.add_return(1, 0.25)

    assert analyzer.get_performance_stats_returns() == {}


@pytest.mark.parametrize("name", ["WinRate", "MaxDrawdown", "ProfitFactor", "Alpha", "CAGR"])
def test_custom_statistic_may_reuse_a_built_in_class_name(name: str) -> None:
    """
    Test custom statistic may reuse a built in class name.
    """

    def calculate_from_returns(self: object, _returns: dict[int, float]) -> float | None:
        return 7.0

    cls = type(name, (PortfolioStatistic,), {"calculate_from_returns": calculate_from_returns})
    analyzer = PortfolioAnalyzer()
    analyzer.register_statistic(cls())
    analyzer.add_return(1, 0.25)

    assert analyzer.get_performance_stats_returns() == {cls().name: 7.0}


def test_built_in_statistic_still_registers_natively() -> None:
    """
    Test built in statistic still registers natively.
    """
    analyzer = PortfolioAnalyzer()
    analyzer.register_statistic(WinRate())
    usd = Currency.from_str("USD")
    analyzer.add_trade(PositionId("P-1"), 1, Money(10.0, usd))
    analyzer.add_trade(PositionId("P-2"), 2, Money(-10.0, usd))

    stats = analyzer.get_performance_stats_pnls(usd, None)

    assert stats["Win Rate"] == 0.5


def test_duck_typed_statistic_without_base_class() -> None:
    """
    Test duck typed statistic without base class.
    """

    class Standalone:
        name = "Standalone"

        def calculate_from_returns(self, returns: dict[int, float]) -> float:
            return sum(returns.values()) * 2.0

    analyzer = PortfolioAnalyzer()
    analyzer.register_statistic(Standalone())
    analyzer.add_return(1, 0.25)

    assert analyzer.get_performance_stats_returns() == {"Standalone": 0.5}


@pytest.mark.parametrize(
    ("value", "expected"),
    [
        (3, 3.0),
        (3.5, 3.5),
        (True, 1.0),
    ],
)
def test_custom_statistic_coerces_numeric_value(value: object, expected: float) -> None:
    """
    Test custom statistic coerces numeric value.
    """

    class ReturnsValue(PortfolioStatistic):
        def calculate_from_returns(self, _returns: dict[int, float]) -> float | None:
            return value  # type: ignore[return-value]

    analyzer = PortfolioAnalyzer()
    analyzer.register_statistic(ReturnsValue())
    analyzer.add_return(1, 0.25)

    assert analyzer.get_performance_stats_returns() == {"Returns Value": expected}


@pytest.mark.parametrize(
    "statistic",
    [
        object(),
        type("NoName", (), {})(),
        type("EmptyName", (), {"name": ""})(),
        type("BlankName", (), {"name": "   "})(),
        type("NonStringName", (), {"name": 7})(),
    ],
)
def test_register_statistic_rejects_invalid_name(statistic: object) -> None:
    """
    Test register statistic rejects invalid name.
    """
    analyzer = PortfolioAnalyzer()

    with pytest.raises(ValueError, match="Invalid statistic"):
        analyzer.register_statistic(statistic)


@pytest.mark.skipif(not tearsheet.PLOTLY_AVAILABLE, reason="plotly is not installed")
def test_declared_scoring_chain_prints_each_stage_and_flags_a_missing_stage() -> None:
    """
    Test a declared scoring chain renders each metric in its stage and flags a missing stage.
    """

    class ForecastScore(PortfolioStatistic):
        """
        A forecast-stage metric, scored against labels.
        """

        @property
        def name(self) -> str:
            return "Forecast Score"

        @property
        def stage(self) -> MetricStage:
            return MetricStage.FORECAST

        def calculate_from_returns(self, returns: dict[int, float]) -> float | None:
            return float(len(returns))

    class AccountOutcome(PortfolioStatistic):
        """
        An account-stage metric, scored against the ledger.
        """

        @property
        def name(self) -> str:
            return "Account Outcome"

        @property
        def stage(self) -> MetricStage:
            return MetricStage.ACCOUNT

        def calculate_from_returns(self, returns: dict[int, float]) -> float | None:
            return sum(returns.values())

    analyzer = PortfolioAnalyzer()
    analyzer.deregister_statistics()
    analyzer.register_statistic(ForecastScore())
    analyzer.register_statistic(AccountOutcome())
    analyzer.add_return(1_000_000_000, 0.01)
    analyzer.add_return(2_000_000_000, -0.005)
    analyzer.set_declared_stages([MetricStage.FORECAST, MetricStage.ACCOUNT])

    assert analyzer.declared_stages() == [MetricStage.FORECAST, MetricStage.ACCOUNT]
    assert analyzer.metric_stages() == {
        "Forecast Score": "forecast",
        "Account Outcome": "account",
    }

    stats = analyzer.statistics()

    assert stats.returns["Forecast Score"] == 2.0
    assert stats.returns["Account Outcome"] == pytest.approx(0.005)
    assert not any(key.startswith("Metric Chain") for key in stats.general)

    # Each rendered metric is grouped under its declared stage.
    table = tearsheet._create_stats_table(
        {},
        stats.returns,
        stats.general,
        metric_stages=analyzer.metric_stages(),
    )
    labels = list(table.cells.values[0])
    assert "<b>Forecast Metrics</b>" in labels
    assert "<b>Account Metrics</b>" in labels
    assert labels[labels.index("<b>Forecast Metrics</b>") + 1] == "Forecast Score"
    assert labels[labels.index("<b>Account Metrics</b>") + 1] == "Account Outcome"
    assert "<b>Returns Statistics</b>" not in labels

    # The same run with the account metric removed flags the missing account stage, and does
    # not flag the forecast stage that still rendered.
    analyzer.deregister_statistic(AccountOutcome())
    stats = analyzer.statistics()

    assert stats.general["Metric Chain: account stage has no metric"] == 1.0
    assert "Metric Chain: forecast stage has no metric" not in stats.general
    assert "Account Outcome" not in stats.returns
    assert "Forecast Score" in stats.returns


def test_correction_impact_report_names_the_metric_and_both_streams() -> None:
    """
    Test correction impact report names the metric and both streams.
    """
    report = CorrectionImpactReport("sharpe_ratio", 1.518, 0.589)

    assert report.metric == "sharpe_ratio"
    assert report.uncorrected == pytest.approx(1.518)
    assert report.corrected == pytest.approx(0.589)
    assert report.delta == pytest.approx(-0.929)


def test_correction_impact_report_refuses_without_a_declared_metric() -> None:
    """
    Test correction impact report refuses without a declared metric.
    """
    # The refusal names what is missing rather than defaulting a metric for the caller.
    with pytest.raises(ValueError, match="a correction requires a declared outcome metric"):
        CorrectionImpactReport(None, 1.0, 0.5)

    with pytest.raises(ValueError, match="a correction requires a declared outcome metric"):
        CorrectionImpactReport("   ", 1.0, 0.5)


def test_correction_impact_report_keeps_a_zero_delta() -> None:
    """
    Test correction impact report keeps a zero delta.
    """
    report = CorrectionImpactReport("returns_volatility", 0.25, 0.25)

    assert report.delta == 0.0


def test_analyzer_renders_the_correction_impact_beside_its_counts() -> None:
    """
    Test analyzer renders the correction impact beside its counts.
    """
    analyzer = PortfolioAnalyzer()
    analyzer.set_correction_impact(CorrectionImpactReport("sharpe_ratio", 1.518, 0.589))

    stats = analyzer.statistics()

    assert stats.returns["Correction Impact: sharpe_ratio (uncorrected)"] == pytest.approx(1.518)
    assert stats.returns["Correction Impact: sharpe_ratio (corrected)"] == pytest.approx(0.589)
    assert stats.returns["Correction Impact: sharpe_ratio (delta)"] == pytest.approx(-0.929)


def test_analyzer_renders_a_zero_delta_rather_than_omitting_the_row() -> None:
    """
    Test analyzer renders a zero delta rather than omitting the row.
    """
    analyzer = PortfolioAnalyzer()
    analyzer.set_correction_impact(CorrectionImpactReport("returns_volatility", 0.25, 0.25))

    stats = analyzer.statistics()

    assert stats.returns["Correction Impact: returns_volatility (delta)"] == 0.0
    assert "Correction Impact: returns_volatility (uncorrected)" in stats.returns
    assert "Correction Impact: returns_volatility (corrected)" in stats.returns


def test_analyzer_renders_no_correction_impact_row_when_none_is_set() -> None:
    """
    Test analyzer renders no correction impact row when none is set.
    """
    analyzer = PortfolioAnalyzer()

    stats = analyzer.statistics()

    assert not any(key.startswith("Correction Impact") for key in stats.returns)


def test_detector_report_prints_a_high_accuracy_beside_a_low_base_rate() -> None:
    """
    Test detector report prints a high accuracy beside a low base rate.
    """
    # 1000 records, 2 truly positive (base rate 0.002). The detector marks 10 positive and gets
    # one right, so accuracy is 0.99 while nine in ten of its alarms are false.
    report = DetectorReport(1, 9, 989, 1)

    assert report.true_positives == 1
    assert report.false_positives == 9
    assert report.true_negatives == 989
    assert report.false_negatives == 1
    assert report.accuracy == pytest.approx(0.99)
    assert report.base_rate == pytest.approx(0.002)
    assert report.precision == pytest.approx(0.1)
    assert report.recall == pytest.approx(0.5)
    assert report.f1 == pytest.approx(1.0 / 6.0)
    assert report.false_discovery_rate == pytest.approx(0.9)
    assert "accuracy 0.99" in repr(report)
    assert "base rate 0.002" in repr(report)
    assert "false discovery rate 0.9" in repr(report)


def test_detector_report_refuses_an_accuracy_without_a_base_rate() -> None:
    """
    Test detector report refuses an accuracy without a base rate.
    """
    # The refusal names the missing base rate rather than defaulting one.
    with pytest.raises(ValueError, match="requires the positive base rate beside its accuracy"):
        DetectorReport.from_accuracy(0.99)

    with pytest.raises(ValueError, match="requires the confusion matrix"):
        DetectorReport.from_accuracy(0.99, 0.002)


def test_detector_report_refuses_a_mismatched_decision_and_label_count() -> None:
    """
    Test detector report refuses a mismatched decision and label count.
    """
    with pytest.raises(ValueError, match="one decision and one label per record"):
        DetectorReport.from_decisions_and_labels([True, False], [True])


def test_detector_report_refuses_an_empty_population() -> None:
    """
    Test detector report refuses an empty population.
    """
    with pytest.raises(ValueError, match="at least one evaluated record"):
        DetectorReport(0, 0, 0, 0)


def test_detector_report_from_decisions_and_labels_reads_the_matrix() -> None:
    """
    Test detector report from decisions and labels reads the matrix.
    """
    # Every record is marked positive and one is truly positive: the accuracy equals the base
    # rate, as does the precision, and the recall is 1.0.
    report = DetectorReport.from_decisions_and_labels(
        [True] * 10,
        [True, False, False, False, False, False, False, False, False, False],
    )

    assert report.true_positives == 1
    assert report.false_positives == 9
    assert report.true_negatives == 0
    assert report.false_negatives == 0
    assert report.accuracy == pytest.approx(report.base_rate)
    assert report.precision == pytest.approx(report.base_rate)
    assert report.recall == pytest.approx(1.0)


def test_analyzer_renders_the_detector_accuracy_with_its_base_rate() -> None:
    """
    Test analyzer renders the detector accuracy with its base rate.
    """
    analyzer = PortfolioAnalyzer()
    analyzer.set_detector_report(DetectorReport(1, 9, 989, 1))

    stats = analyzer.statistics()

    assert stats.returns["Detector Report: accuracy"] == pytest.approx(0.99)
    assert stats.returns["Detector Report: base rate"] == pytest.approx(0.002)
    assert stats.returns["Detector Report: precision"] == pytest.approx(0.1)
    assert stats.returns["Detector Report: recall"] == pytest.approx(0.5)
    assert stats.returns["Detector Report: false discovery rate"] == pytest.approx(0.9)
    assert stats.general["Detector Report: true positives"] == 1.0
    assert stats.general["Detector Report: false positives"] == 9.0
    assert stats.general["Detector Report: true negatives"] == 989.0
    assert stats.general["Detector Report: false negatives"] == 1.0


def test_detector_accuracy_and_base_rate_rows_cannot_be_separated() -> None:
    """
    Test detector accuracy and base rate rows cannot be separated.
    """
    analyzer = PortfolioAnalyzer()
    analyzer.set_detector_report(DetectorReport(45, 5, 45, 5))

    stats = analyzer.statistics()

    accuracy = "Detector Report: accuracy" in stats.returns
    base_rate = "Detector Report: base rate" in stats.returns

    assert accuracy
    assert accuracy == base_rate
    assert stats.returns["Detector Report: base rate"] == pytest.approx(0.5)
    assert stats.returns["Detector Report: accuracy"] == pytest.approx(0.9)

    analyzer.set_detector_report(None)
    stats = analyzer.statistics()

    assert not any(key.startswith("Detector Report") for key in stats.returns)
    assert not any(key.startswith("Detector Report") for key in stats.general)
