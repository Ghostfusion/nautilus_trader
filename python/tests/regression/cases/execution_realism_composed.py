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
A venue with an independent slippage model and a market impact model composed.

The venue sets both concerns at once, which is the composition the documented ordering promises:
the fill model decides eligibility, quantity, and the base fill price, the slippage model adjusts
that base price, and the market impact model moves the adjusted price by size. The scenario
configures the same two models its single-model scenarios configure, taken from those modules, so
the composed result checkpoints the ordering itself rather than a third behaviour.

The checkpoints are the two fills, the order, and the position. The first fill pins the price both
models produced in order, which is the composition itself; the second is the exhausted-volume
remainder, whose price is the first plus one increment, so the two checkpoints together pin that
the impact composes after the slippage adjustment and carries into the remainder.
"""

from __future__ import annotations

from pathlib import Path
from tempfile import TemporaryDirectory
from typing import Any

from tests.regression.cases.market_impact_model import MARKET_IMPACT_MODEL
from tests.regression.cases.venue_slippage_model import SLIPPAGE_MODEL
from tests.regression.execution_realism import INSTRUMENT_ID_STR
from tests.regression.execution_realism import run_node
from tests.regression.scenario import Checkpoint
from tests.regression.scenario import Scenario


NAME = "execution_realism_composed"
# Fixed, because a canonical document records the run config ID and a generated one is random.
RUN_CONFIG_ID = "b7e4a9c2-3d18-4f56-8c0a-6e1d4f9b2c73"

CHECKPOINTS = (
    Checkpoint("order", instrument_id=INSTRUMENT_ID_STR, ordinal=0),
    Checkpoint("fill", instrument_id=INSTRUMENT_ID_STR, ordinal=0),
    Checkpoint("fill", instrument_id=INSTRUMENT_ID_STR, ordinal=1),
    Checkpoint("position", instrument_id=INSTRUMENT_ID_STR, ordinal=0),
)


def execute() -> Any:
    """
    Run the scenario and return its canonical result.
    """
    with TemporaryDirectory() as directory:
        return run_node(
            Path(directory),
            run_config_id=RUN_CONFIG_ID,
            slippage_model=SLIPPAGE_MODEL,
            market_impact_model=MARKET_IMPACT_MODEL,
        )


SCENARIO = Scenario(
    name=NAME,
    execute=execute,
    checkpoints=CHECKPOINTS,
)
