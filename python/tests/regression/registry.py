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
The declared regression scenario registry.

Discovery is an explicit list rather than reflection: a new scenario is added here, and its
expectations are generated with `--regenerate-regression` and reviewed like any other change.
"""

from __future__ import annotations

from tests.regression.cases.btcusdt_ema_cross import SCENARIO as BTCUSDT_EMA_CROSS
from tests.regression.cases.equity_session_events import SCENARIO as EQUITY_SESSION_EVENTS
from tests.regression.cases.execution_realism_composed import SCENARIO as EXECUTION_REALISM_COMPOSED
from tests.regression.cases.market_impact_model import SCENARIO as MARKET_IMPACT_MODEL
from tests.regression.cases.multi_venue_parity import SCENARIO as MULTI_VENUE_PARITY
from tests.regression.cases.target_pipeline_parity import SCENARIO as TARGET_PIPELINE_PARITY
from tests.regression.cases.universe_membership import SCENARIO as UNIVERSE_MEMBERSHIP
from tests.regression.cases.venue_slippage_model import SCENARIO as VENUE_SLIPPAGE_MODEL
from tests.regression.scenario import Scenario


SCENARIOS: tuple[Scenario, ...] = (
    MULTI_VENUE_PARITY,
    BTCUSDT_EMA_CROSS,
    EQUITY_SESSION_EVENTS,
    UNIVERSE_MEMBERSHIP,
    VENUE_SLIPPAGE_MODEL,
    MARKET_IMPACT_MODEL,
    EXECUTION_REALISM_COMPOSED,
    TARGET_PIPELINE_PARITY,
)
