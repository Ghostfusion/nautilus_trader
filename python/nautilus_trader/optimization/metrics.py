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
The bridge from a completed run's statistics to the objective's metric values.

A default run reports its returns statistics in `BacktestResult.stats_returns`, but that set is
the engine's own default and does not include every built-in statistic. "Max Drawdown" is the
important example: the kernel portfolio's analyzer does not register it, so it is absent from a
run's dictionary. The bridge closes that gap without computing any metric itself.

The bridge feeds the completed run's own `BacktestResult.returns_series` into a fresh
`nautilus_trader.analysis.PortfolioAnalyzer` with the requested native statistic registered, and
reads the value back from the analyzer. The statistic implementations stay the project's own: the
bridge resolves a metric name to a statistic by instantiating the statistic classes exported from
`nautilus_trader.analysis` and reading each instance's own `name`, so there is no hand-written name
table that could drift from the Rust statistics.
"""

from __future__ import annotations

from functools import lru_cache
from importlib import import_module
from typing import TYPE_CHECKING

from nautilus_trader.analysis import PortfolioAnalyzer


if TYPE_CHECKING:
    from collections.abc import Iterable
    from collections.abc import Mapping

    from nautilus_trader.backtest import BacktestResult


def statistic_values(
    result: BacktestResult,
    metrics: Iterable[str] | None = None,
) -> dict[str, float]:
    """
    Return the metric values of a completed run.

    Values the run already reports (its general and returns statistics) are taken from the run
    itself. Any requested metric that the run does not report is resolved through the statistics
    bridge, which recomputes it from the run's own returns series with the native statistic.
    A metric that cannot be resolved is simply absent from the result, so a later objective
    evaluation reports it as a missing value rather than scoring it as zero.

    Parameters
    ----------
    result : BacktestResult
        The completed backtest result.
    metrics : Iterable[str] | None, default None
        The metric names to return. When None, every metric the run reports plus every
        returns-based statistic the bridge can compute is returned.

    Returns
    -------
    dict[str, float]
        The metric values keyed by the statistic's own name.

    """
    requested = None if metrics is None else frozenset(metrics)

    values: dict[str, float] = {}
    values.update(result.stats_general)
    values.update(result.stats_returns)

    if requested is None:
        missing: frozenset[str] | None = None
    else:
        missing = frozenset(name for name in requested if name not in values)

    values.update(bridged_values(result.returns_series, missing))

    if requested is not None:
        return {name: value for name, value in values.items() if name in requested}
    return values


def bridged_values(
    returns_series: Mapping[int, float],
    metrics: Iterable[str] | None = None,
) -> dict[str, float]:
    """
    Return the returns-based statistic values the bridge can compute for the given series.

    The series is fed, in order, into a fresh `PortfolioAnalyzer` with only the requested native
    statistics registered, and the analyzer's return statistics are read back. A statistic that a
    requested name does not resolve to is omitted.

    Parameters
    ----------
    returns_series : Mapping[int, float]
        The run's returns keyed by Unix nanosecond timestamp.
    metrics : Iterable[str] | None, default None
        The metric names to compute. When None, every returns-based statistic is computed.

    Returns
    -------
    dict[str, float]
        The computed values keyed by the statistic's own name.

    """
    available = _returns_statistics_by_name()
    requested = None if metrics is None else frozenset(metrics)

    selected = {
        name: statistic
        for name, statistic in available.items()
        if requested is None or name in requested
    }
    if not selected:
        return {}

    analyzer = PortfolioAnalyzer()
    for statistic in selected.values():
        analyzer.register_statistic(statistic)
    for timestamp, value in returns_series.items():
        analyzer.add_return(timestamp, value)
    return dict(analyzer.get_performance_stats_returns())


@lru_cache(maxsize=1)
def _returns_statistics_by_name() -> dict[str, object]:
    """
    Discover the native returns-based statistics and index them by their own name.

    The set is derived from `nautilus_trader.analysis` at runtime: each exported class is
    instantiated with no arguments, and an instance that exposes both a `name` and a
    `calculate_from_returns` is kept. The names come from the statistic objects themselves, so
    this is not a hand-maintained name table. Classes that need constructor arguments, enums, and
    the analyzer types are skipped.
    """
    analysis = import_module("nautilus_trader.analysis")
    statistics: dict[str, object] = {}
    for attribute_name in dir(analysis):
        candidate = getattr(analysis, attribute_name)
        if not isinstance(candidate, type) or candidate is analysis.PortfolioStatistic:
            continue
        if not hasattr(candidate, "calculate_from_returns"):
            continue
        instance = _instantiate(candidate)
        if instance is None:
            continue
        name = _metric_name(instance)
        if name is not None and name not in statistics:
            statistics[name] = instance
    return statistics


def _metric_name(statistic: object) -> str | None:
    """
    Return the statistic's own name, or None when it is not a string-valued name.
    """
    try:
        name = statistic.name
    except (AttributeError, NotImplementedError):
        return None
    return name if isinstance(name, str) else None


def _instantiate(candidate: type) -> object | None:
    """
    Instantiate the class with no arguments, or return None when it cannot be constructed.
    """
    try:
        return candidate()
    except (TypeError, ValueError):
        return None
