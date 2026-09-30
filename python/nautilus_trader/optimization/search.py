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
Search strategies that enumerate experiments from a parameter space.

Search and execution are separate concerns: a strategy only enumerates parameter sets, and a
runner executes one. A strategy never runs a backtest and never sees a result.
"""

from __future__ import annotations

from typing import TYPE_CHECKING
from typing import Protocol
from typing import runtime_checkable


if TYPE_CHECKING:
    from collections.abc import Iterator

    from nautilus_trader.optimization.space import Experiment
    from nautilus_trader.optimization.space import ParameterSpace


@runtime_checkable
class SearchStrategy(Protocol):
    """
    A strategy that enumerates the experiments to evaluate.

    Implementations are pure enumeration: they take a parameter space and yield experiments in a
    deterministic order. They do not execute runs.
    """

    def experiments(self, space: ParameterSpace) -> Iterator[Experiment]:
        """
        Yield the experiments to evaluate for the given space.

        Parameters
        ----------
        space : ParameterSpace
            The parameter space to enumerate.

        Yields
        ------
        Experiment

        """
        ...


class GridSearch:
    """
    A search strategy that evaluates every point of the parameter space.

    The enumeration is the space's own deterministic expansion, so a grid sweep is reproducible.
    """

    def experiments(self, space: ParameterSpace) -> Iterator[Experiment]:
        """
        Yield every experiment of the given space.

        Parameters
        ----------
        space : ParameterSpace
            The parameter space to expand.

        Yields
        ------
        Experiment

        """
        yield from space.expand()
