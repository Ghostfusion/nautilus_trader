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
Test EODHD corporate actions delivered by the data client.
"""

import asyncio

import pytest

from nautilus_trader.adapters.eodhd import EodhdDataClientConfig
from nautilus_trader.adapters.eodhd import EodhdDataClientFactory
from nautilus_trader.common import Environment
from nautilus_trader.live import LiveNode
from nautilus_trader.model import BarType
from nautilus_trader.model import ClientId
from nautilus_trader.model import CorporateActionType
from nautilus_trader.model import InstrumentId
from nautilus_trader.model import TraderId
from nautilus_trader.trading import Strategy


BAR_TYPE = "AAPL.US-1-DAY-LAST-EXTERNAL"
INSTRUMENT_ID = "AAPL.US"

BAR_ROWS = [
    {
        "date": "2024-02-08",
        "open": 188.0,
        "high": 189.0,
        "low": 187.0,
        "close": 188.32,
        "adjusted_close": 188.32,
        "volume": 1000,
    },
]

DIVIDEND_ROWS = [
    {
        "date": "2024-02-09",
        "declarationDate": "2024-02-01",
        "recordDate": "2024-02-12",
        "paymentDate": "2024-02-15",
        "period": None,
        "value": 0.06,
        "unadjustedValue": 0.24,
        "currency": "USD",
    },
]

SPLIT_ROWS = [
    {"date": "2020-08-31", "split": "4.000000/1.000000"},
]


class _Collector(Strategy):
    """Collects the corporate actions the EODHD data client publishes."""

    def __init__(self) -> None:
        """Initialize the collector."""
        super().__init__()
        self.actions: list = []

    def on_start(self) -> None:
        """Subscribe to the actions before the bars, so none is published before the topic."""
        self.subscribe_corporate_actions(InstrumentId.from_str(INSTRUMENT_ID))
        self.subscribe_bars(BarType.from_str(BAR_TYPE), client_id=ClientId.from_str("EODHD"))

    def on_corporate_action(self, action) -> None:
        """Record a delivered corporate action."""
        self.actions.append(action)


@pytest.fixture()
def base_url(canned_server: str, canned_routes: dict) -> str:
    """Provide the base URL of a local server serving the corporate action responses."""
    canned_routes.update(
        {
            "/eod/AAPL.US": BAR_ROWS,
            "/div/AAPL.US": DIVIDEND_ROWS,
            "/splits/AAPL.US": SPLIT_ROWS,
        },
    )

    return canned_server


@pytest.mark.asyncio()
async def test_corporate_actions_reach_a_subscribed_strategy(base_url: str) -> None:
    """Test the actions in the window of a bar subscription are published to the strategy."""
    strategy = _Collector()
    node = (
        LiveNode.builder(
            "EODHD-ACTIONS-PYTEST-001",
            TraderId.from_str("TESTER-001"),
            Environment.SANDBOX,
        )
        .add_data_client(
            None,
            EodhdDataClientFactory(),
            EodhdDataClientConfig(
                api_key="test-token",
                http_base_url=base_url,
                load_instruments=False,
                load_corporate_actions=True,
                poll_interval_secs=1,
                backfill_days=2,
            ),
        )
        .build()
    )
    node.add_strategy(strategy)

    run_task = asyncio.create_task(node.run_async())
    await asyncio.sleep(6)
    node.handle().stop()
    await asyncio.wait_for(run_task, timeout=25)

    by_type = {action.action: action for action in strategy.actions}

    assert CorporateActionType.DIVIDEND in by_type, strategy.actions
    assert CorporateActionType.SPLIT in by_type, strategy.actions

    dividend = by_type[CorporateActionType.DIVIDEND]
    assert str(dividend.value) == "0.24"
    assert dividend.effective_ns == 1_707_436_800_000_000_000

    split = by_type[CorporateActionType.SPLIT]
    assert str(split.value) == "4"
    assert split.effective_ns == 1_598_832_000_000_000_000


@pytest.mark.asyncio()
async def test_corporate_actions_are_not_requested_unless_enabled(
    base_url: str,
    canned_requests: list,
) -> None:
    """Test that the default configuration transfers no corporate action data."""
    strategy = _Collector()
    node = (
        LiveNode.builder(
            "EODHD-ACTIONS-PYTEST-002",
            TraderId.from_str("TESTER-001"),
            Environment.SANDBOX,
        )
        .add_data_client(
            None,
            EodhdDataClientFactory(),
            EodhdDataClientConfig(
                api_key="test-token",
                http_base_url=base_url,
                load_instruments=False,
                poll_interval_secs=1,
                backfill_days=2,
            ),
        )
        .build()
    )
    node.add_strategy(strategy)

    run_task = asyncio.create_task(node.run_async())
    await asyncio.sleep(6)
    node.handle().stop()
    await asyncio.wait_for(run_task, timeout=25)

    assert strategy.actions == []
    assert not [path for path in canned_requests if path.startswith(("/div/", "/splits/"))]
