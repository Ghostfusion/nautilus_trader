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
Test the optimization objective and constraint over portfolio statistics.
"""

import pytest

from nautilus_trader.analysis import Constraint
from nautilus_trader.analysis import ConstraintComparison
from nautilus_trader.analysis import Objective
from nautilus_trader.analysis import ObjectiveDirection
from nautilus_trader.analysis import ObjectiveTerm


def test_objective_over_sharpe_and_max_drawdown() -> None:
    """
    Test an objective over Sharpe ratio and maximum drawdown.

    Max Drawdown is reported as a negative fraction, so a shallower drawdown is a larger
    value and is preferred with MAXIMIZE.
    """
    objective = Objective(
        [
            ObjectiveTerm(
                "Sharpe Ratio (simple, sample, 252 days)", 1.0, ObjectiveDirection.MAXIMIZE
            ),
            ObjectiveTerm("Max Drawdown (simple)", 1.0, ObjectiveDirection.MAXIMIZE),
        ],
    )
    values = {"Sharpe Ratio (simple, sample, 252 days)": 2.0, "Max Drawdown (simple)": -0.25}

    # 1.0 * 2.0 + 1.0 * -0.25
    assert objective.evaluate(values) == 1.75


def test_constraint_verdict_on_max_drawdown() -> None:
    """
    Test a constraint verdict on maximum drawdown.
    """
    values = {"Sharpe Ratio (simple, sample, 252 days)": 2.0, "Max Drawdown (simple)": -0.25}

    satisfied = Constraint("Max Drawdown (simple)", ConstraintComparison.AT_LEAST, -0.30)
    violated = Constraint("Max Drawdown (simple)", ConstraintComparison.AT_LEAST, -0.20)

    assert satisfied.is_satisfied(values) is True
    assert violated.is_satisfied(values) is False


def test_objective_term_with_unknown_metric_raises() -> None:
    """
    Test an objective term with an unknown metric raises.
    """
    with pytest.raises(ValueError, match="unknown metric"):
        ObjectiveTerm("Not A Metric", 1.0, ObjectiveDirection.MAXIMIZE)


def test_constraint_with_unknown_metric_raises() -> None:
    """
    Test a constraint with an unknown metric raises.
    """
    with pytest.raises(ValueError, match="unknown metric"):
        Constraint("Not A Metric", ConstraintComparison.AT_LEAST, 1.0)


def test_objective_evaluate_with_missing_metric_value_raises() -> None:
    """
    Test evaluating an objective with a missing metric value raises.
    """
    objective = Objective(
        [ObjectiveTerm("Max Drawdown (simple)", 1.0, ObjectiveDirection.MAXIMIZE)],
    )

    with pytest.raises(ValueError, match="no value supplied"):
        objective.evaluate({"Sharpe Ratio (simple, sample, 252 days)": 2.0})
