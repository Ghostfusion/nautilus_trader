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
Declared regression scenarios for deterministic backtest verification.

Each scenario verifies all three declared layers of a deterministic backtest: the canonical
digest, the declared statistics, and the declared semantic checkpoints. Expectations are
committed; regenerate them in one command with `--regenerate-regression` and review the diff.
"""

from __future__ import annotations

import pytest

from tests.regression.registry import REPORT_SCENARIOS
from tests.regression.registry import SCENARIOS
from tests.regression.scenario import ReportScenario
from tests.regression.scenario import Scenario
from tests.regression.scenario import load_expectations
from tests.regression.scenario import load_report_expectations
from tests.regression.scenario import run_report_scenario
from tests.regression.scenario import run_scenario
from tests.regression.scenario import verify_report_scenario
from tests.regression.scenario import verify_scenario
from tests.regression.scenario import write_expectations
from tests.regression.scenario import write_report_expectations


@pytest.mark.regression
@pytest.mark.parametrize("scenario", SCENARIOS, ids=[scenario.name for scenario in SCENARIOS])
def test_declared_regression_scenario(
    scenario: Scenario,
    regression_baseline: None,
    regenerate_regression: bool,
) -> None:
    """
    Test the declared digest, statistics, and semantic checkpoints of a scenario.
    """
    outcome = run_scenario(scenario)
    if regenerate_regression:
        write_expectations(scenario, outcome)
        return
    verify_scenario(scenario, outcome, load_expectations(scenario))


@pytest.mark.regression
@pytest.mark.parametrize(
    "scenario",
    REPORT_SCENARIOS,
    ids=[scenario.name for scenario in REPORT_SCENARIOS],
)
def test_declared_regression_report(
    scenario: ReportScenario,
    regression_baseline: None,
    regenerate_regression: bool,
) -> None:
    """
    Test the declared rows and checks of a report scenario.
    """
    document = run_report_scenario(scenario)
    if regenerate_regression:
        write_report_expectations(scenario, document)
        return
    verify_report_scenario(scenario, document, load_report_expectations(scenario))
