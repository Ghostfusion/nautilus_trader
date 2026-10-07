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
The report rows that render only when a report is explicitly configured.

A canonical run's default report carries the rows every run produces, so the rows a caller sets -
a correction's effect, a detector report, a deflated-Sharpe statistic carrying its trial
specification - are absent from every canonical document. This case builds them through the same
public API a caller uses and records them under the names the report renders, so a change to a
row's name or value is caught the same way a canonical divergence is. It also records the declared
trial specification and the significance bound at both specification extremes, which are the
provenance a search reports rather than a run's own numbers.

The refusal of a correction without a declared outcome metric is asserted rather than recorded: a
refusal raises, so it is a check the case performs rather than a row a report carries.
"""

from __future__ import annotations

from typing import Any

from nautilus_trader.analysis import CorrectionImpactReport
from nautilus_trader.analysis import DetectorReport
from nautilus_trader.analysis import PortfolioAnalyzer
from nautilus_trader.optimization import DeflatedSharpeRatio
from nautilus_trader.optimization import SharpeSample
from nautilus_trader.optimization import TrialDependence
from nautilus_trader.optimization import TrialSpecification
from nautilus_trader.optimization import deflated_sharpe_bounds
from tests.regression.scenario import ReportScenario


NAME = "report_rows"

# The outcome metric the correction is measured against and the pair of measurements it reports.
CORRECTION_METRIC = "sharpe_ratio"
UNCORRECTED = 1.518
CORRECTED = 0.589

# A 0.2 percent positive base rate: 1000 records, 2 truly positive, 10 marked positive.
DETECTOR = (1, 9, 989, 1)

# The row prefixes this case protects, so only the rows it declares are recorded.
PROTECTED_PREFIXES = ("Correction Impact:", "Detector Report:", "Deflated Sharpe Ratio")

# The trial Sharpes a sweep produced, above the contract's declared minimum trial count.
TRIAL_SHARPES = [0.2, 0.5, 0.8, 1.0, 0.75, 0.6, 0.9, 1.2, 0.35, 0.55, 0.7, 1.1]

# The two declared specifications the significance bound is read at, most and least favourable.
MOST_FAVOURABLE_SPECIFICATION = TrialSpecification(
    data_window=(1_000, 2_000),
    universe_rule="top_500_by_capitalisation",
    weighting="value",
)
LEAST_FAVOURABLE_SPECIFICATION = TrialSpecification(
    data_window=(1_000, 2_000),
    universe_rule="top_500_by_capitalisation",
    weighting="equal",
)

# A deterministic return series long enough for the declared minimum observation count. The
# deflated-Sharpe statistic reads its sample from this series, so the row is a real calculation.
RETURNS: dict[int, float] = {
    1_600_000_000_000_000_000 + index * 86_400_000_000_000: value
    for index, value in enumerate(
        [
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
        ],
    )
}


def _sample(specification: TrialSpecification, sharpe: float) -> SharpeSample:
    """
    Build a sample carrying the specification it was found under.
    """
    return SharpeSample(
        sharpe=sharpe,
        trial_sharpes=TRIAL_SHARPES,
        observations=len(RETURNS),
        skew=0.0,
        kurtosis=3.0,
        dependence=TrialDependence.INDEPENDENT,
        specification=specification,
    )


def _refusal_check() -> str:
    """
    Assert a correction without a declared outcome metric is refused, and return the reason.
    """
    try:
        CorrectionImpactReport(None, 1.0, 0.5)
    except ValueError as error:
        return f"a correction without a declared outcome metric is refused: {error}"
    raise AssertionError("a correction without a declared outcome metric must be refused")


def execute() -> dict[str, Any]:
    """
    Build the configured report's rows, the trial specification and the two significance bounds.
    """
    analyzer = PortfolioAnalyzer()
    for timestamp, value in RETURNS.items():
        analyzer.add_return(timestamp, value)

    analyzer.set_correction_impact(
        CorrectionImpactReport(CORRECTION_METRIC, UNCORRECTED, CORRECTED),
    )
    analyzer.set_detector_report(DetectorReport(*DETECTOR))
    analyzer.register_statistic(
        DeflatedSharpeRatio(TRIAL_SHARPES, specification=MOST_FAVOURABLE_SPECIFICATION),
    )

    stats = analyzer.statistics()
    rendered = {**stats.returns, **stats.general}

    rows: dict[str, Any] = {
        name: value for name, value in rendered.items() if name.startswith(PROTECTED_PREFIXES)
    }

    rows[f"Trial Specification: {MOST_FAVOURABLE_SPECIFICATION.label}"] = (
        MOST_FAVOURABLE_SPECIFICATION.to_dict()
    )

    bounds = deflated_sharpe_bounds(
        [
            _sample(MOST_FAVOURABLE_SPECIFICATION, 1.2),
            _sample(LEAST_FAVOURABLE_SPECIFICATION, 0.4),
        ],
    )
    rows["Specification Bound: most_favourable"] = bounds.most_favourable.to_dict()
    rows["Specification Bound: least_favourable"] = bounds.least_favourable.to_dict()

    return {"rows": rows, "checks": [_refusal_check()]}


SCENARIO = ReportScenario(name=NAME, execute=execute)
