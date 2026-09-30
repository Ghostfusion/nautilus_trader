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
A venue whose independent slippage model adjusts the fill price.

The venue sets `slippage_model` and nothing else, so the fill model decides eligibility, quantity,
and the base fill price exactly as it does by default, and the slippage model is the only
configured concern. The model is seeded, so its draws reproduce across runs and the committed
canonical digest is the reproducibility check for the concern the fill model no longer owns.

The checkpoints are the two fills, the order, and the position. The first fill pins the price the
slippage model produced; the second is the exhausted-volume remainder, whose price is the first
plus one increment, so the two checkpoints together pin how the decision propagates.
"""

from __future__ import annotations

from pathlib import Path
from tempfile import TemporaryDirectory
from typing import Any

from nautilus_trader.execution import ProbabilisticSlippageModel
from tests.regression.execution_realism import INSTRUMENT_ID_STR
from tests.regression.execution_realism import run_node
from tests.regression.scenario import Checkpoint
from tests.regression.scenario import Scenario


NAME = "venue_slippage_model"
# Fixed, because a canonical document records the run config ID and a generated one is random.
RUN_CONFIG_ID = "4d0b2f6a-2f0e-4a64-9a2d-3e5b7c1f8a11"

# A seeded draw per fill, so the adjusted prices are a deterministic function of the run.
SLIPPAGE_MODEL = ProbabilisticSlippageModel(prob_slippage=0.5, random_seed=7)

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
        )


SCENARIO = Scenario(
    name=NAME,
    execute=execute,
    checkpoints=CHECKPOINTS,
)
