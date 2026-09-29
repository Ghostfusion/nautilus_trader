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
Regression suite configuration: regeneration, reset, and suite-wide baseline.
"""

from __future__ import annotations

import os
from collections.abc import Iterator

import pytest

from tests.regression.scenario import clean_baseline


REGENERATE_ENV_VAR = "NAUTILUS_REGRESSION_REGENERATE"
_ENABLED_VALUES = frozenset({"1", "true", "yes", "on"})


def pytest_addoption(parser: pytest.Parser) -> None:
    """
    Add the regression regeneration option.
    """
    group = parser.getgroup("regression")
    group.addoption(
        "--regenerate-regression",
        action="store_true",
        default=False,
        help="Rewrite all three expectation layers for every declared regression scenario",
    )


def pytest_configure(config: pytest.Config) -> None:
    """
    Register the regression marker.
    """
    config.addinivalue_line(
        "markers",
        "regression: declared deterministic regression scenario",
    )


@pytest.fixture(scope="session")
def regenerate_regression(request: pytest.FixtureRequest) -> bool:
    """
    Whether expectations are rewritten instead of verified.
    """
    if request.config.getoption("--regenerate-regression"):
        return True
    return os.environ.get(REGENERATE_ENV_VAR, "").strip().lower() in _ENABLED_VALUES


@pytest.fixture
def regression_baseline() -> Iterator[None]:
    """
    Return engines, caches, loggers, and clock state to a clean baseline between scenarios.
    """
    clean_baseline()
    yield
    clean_baseline()
