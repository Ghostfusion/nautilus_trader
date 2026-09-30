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
A venue whose linear market impact model moves a liquidity-taking fill by size.

The venue sets `market_impact_model` and nothing else, so the model is the only configured
concern. The model is a pure function of the fill quantity and takes no seed, so the committed
canonical digest is the reproducibility check for it.

The checkpoints are the two fills, the order, and the position. The first fill pins the price the
model produced for the quantity the book supplied; the second is the exhausted-volume remainder,
whose price is the first plus one increment, so the two checkpoints together pin how the size
adjusted decision propagates.
"""

from __future__ import annotations

from pathlib import Path
from tempfile import TemporaryDirectory
from typing import Any

from nautilus_trader.execution import LinearMarketImpactModel
from nautilus_trader.model import Quantity
from tests.regression.execution_realism import INSTRUMENT_ID_STR
from tests.regression.execution_realism import run_node
from tests.regression.scenario import Checkpoint
from tests.regression.scenario import Scenario


NAME = "market_impact_model"
# Fixed, because a canonical document records the run config ID and a generated one is random.
RUN_CONFIG_ID = "8a5c1d34-6f2b-4e07-b1a9-5c8d2e7f3b42"

# One price increment per 10 units filled, capped at five increments for a single fill.
# The synthetic L1 book supplies 25 units, so the first fill moves by two increments.
MARKET_IMPACT_MODEL = LinearMarketImpactModel(
    quantity_per_increment=Quantity.from_int(10),
    max_increments=5,
)

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
            market_impact_model=MARKET_IMPACT_MODEL,
        )


SCENARIO = Scenario(
    name=NAME,
    execute=execute,
    checkpoints=CHECKPOINTS,
)
