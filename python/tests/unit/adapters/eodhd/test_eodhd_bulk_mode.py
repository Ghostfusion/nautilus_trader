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
Test EODHD bulk last-day mode against a local server.
"""

import asyncio

import pytest

from nautilus_trader.adapters.eodhd import EodhdDataClientConfig
from nautilus_trader.adapters.eodhd import EodhdDataClientFactory
from nautilus_trader.common import Environment
from nautilus_trader.live import LiveNode
from nautilus_trader.model import BarType
from nautilus_trader.model import ClientId
from nautilus_trader.model import Price
from nautilus_trader.model import TraderId
from nautilus_trader.trading import Strategy


BAR_TYPE = "AAPL.US-1-DAY-LAST-EXTERNAL"

# The seed window serves one day per symbol; the bulk route serves a newer day for the whole
# exchange, including a second symbol the subscription did not ask for.
SEED_ROWS = [
    {
        "date": "2024-01-02",
        "open": 185.0,
        "high": 186.0,
        "low": 184.0,
        "close": 185.5,
        "adjusted_close": 185.5,
        "volume": 1000,
    },
]

BULK_ROWS = [
    {
        "code": "AAPL",
        "exchange_short_name": "US",
        "date": "2024-01-03",
        "open": 186.0,
        "high": 190.0,
        "low": 185.0,
        "close": 189.25,
        "adjusted_close": 189.25,
        "volume": 2500,
    },
    {
        "code": "MSFT",
        "exchange_short_name": "US",
        "date": "2024-01-03",
        "open": 1.0,
        "high": 2.0,
        "low": 0.5,
        "close": 1.5,
        "adjusted_close": 1.5,
        "volume": 10,
    },
]


class _Collector(Strategy):
    """Collects the bars the EODHD data client delivers."""

    def __init__(self) -> None:
        """Initialize the collector."""
        super().__init__()
        self.bars: list = []

    def on_start(self) -> None:
        """Subscribe to the daily bar type on the EODHD client."""
        self.subscribe_bars(BarType.from_str(BAR_TYPE), client_id=ClientId.from_str("EODHD"))

    def on_bar(self, bar) -> None:
        """Record a delivered bar."""
        self.bars.append(bar)


@pytest.fixture()
def base_url(canned_server: str, canned_routes: dict) -> str:
    """Provide the base URL of a local server serving the bulk canned responses."""
    canned_routes.update(
        {
            "/eod/AAPL.US": SEED_ROWS,
            "/eod-bulk-last-day/US": BULK_ROWS,
        },
    )

    return canned_server


@pytest.mark.asyncio()
async def test_bulk_mode_serves_daily_bars_from_one_exchange_request(
    base_url: str,
    canned_requests: list,
) -> None:
    """Test that a daily subscription on a bulk exchange is polled through the bulk endpoint."""
    strategy = _Collector()
    node = (
        LiveNode.builder(
            "EODHD-BULK-PYTEST-001",
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
                bulk_exchanges=["US"],
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

    closes = [bar.close for bar in strategy.bars]

    assert Price.from_str("185.5") in closes, f"the seeded bar was not delivered: {closes}"
    assert Price.from_str("189.25") in closes, f"the bulk bar was not delivered: {closes}"

    paths = list(canned_requests)

    assert paths.count("/eod-bulk-last-day/US") >= 1, paths
    assert paths.count("/eod/AAPL.US") == 1, f"the seed must not repeat on every poll: {paths}"
    assert all("MSFT" not in path for path in paths), paths
