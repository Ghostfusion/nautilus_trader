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
A venue whose concave market impact model moves a liquidity-taking fill by a square root of size.

The venue sets `market_impact_model` and nothing else, so the model is the only configured
concern. The model is a pure function of the fill quantity and takes no seed, so the committed
canonical digest is the reproducibility check for it.

The checkpoints are the two fills, the order, and the position. The synthetic L1 book supplies 25
units, so the first fill pins what the model returns for the reference quantity itself, four
increments, where the linear case beside this one returns two. The second is the exhausted-volume
remainder, which fills at the first fill's price plus the book's one-increment step to its next
level, so the pair pins the adjustment the model applied rather than only that it applied one.

The prefactor is calibrated over 1.0 to 4.0 from an anonymous tape, so the committed digest also
pins which bound of the interval the model applies: the four increments are the upper bound, and a
model that applied the lower one would move the price by a single increment instead.
"""

from __future__ import annotations

from pathlib import Path
from tempfile import TemporaryDirectory
from typing import Any

from nautilus_trader.execution import ImpactCalibrationSource
from nautilus_trader.execution import PrefactorInterval
from nautilus_trader.execution import SquareRootMarketImpactModel
from nautilus_trader.model import Quantity
from tests.regression.execution_realism import INSTRUMENT_ID_STR
from tests.regression.execution_realism import run_node
from tests.regression.scenario import Checkpoint
from tests.regression.scenario import Scenario


NAME = "market_impact_square_root"
# Fixed, because a canonical document records the run config ID and a generated one is random.
RUN_CONFIG_ID = "c41f9a2b-7d38-4e5c-9b06-2af41d7c8e13"

# Calibrated over 1.0 to 4.0 from an anonymous tape; the model applies the upper bound, so the
# reference quantity of 25 units moves by four increments and a fill twice the reference moves by
# fewer than eight: floor(4 * sqrt(2)) = 5 rather than 8.
MARKET_IMPACT_MODEL = SquareRootMarketImpactModel(
    prefactor=PrefactorInterval(
        lower=1.0,
        upper=4.0,
        source=ImpactCalibrationSource.ANONYMOUS_TAPE_DEBIASED,
    ),
    reference_quantity=Quantity.from_int(25),
    max_increments=25,
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
