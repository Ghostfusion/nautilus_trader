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
Test EODHD loader behavior against a local server.
"""

import json
from collections.abc import Iterator
from http.server import BaseHTTPRequestHandler
from http.server import ThreadingHTTPServer
from threading import Thread
from typing import ClassVar
from urllib.parse import urlparse

import pytest

from nautilus_trader.adapters.eodhd import EodhdDataLoader
from nautilus_trader.model import InstrumentId
from nautilus_trader.model import Price
from nautilus_trader.model import Quantity


EOD_ROWS = [
    {
        "date": "2024-01-02",
        "open": 187.15,
        "high": 188.44,
        "low": 183.89,
        "close": 185.64,
        "adjusted_close": 183.404,
        "volume": 82488700,
    },
    {
        "date": "2024-01-03",
        "open": 184.22,
        "high": 185.88,
        "low": 183.43,
        "close": 184.25,
        "adjusted_close": 182.036,
        "volume": 58414500,
    },
]

SYMBOL_ROWS = [
    {
        "Code": "AAPL",
        "Name": "Apple Inc",
        "Country": "USA",
        "Exchange": "US",
        "Currency": "USD",
        "Type": "Common Stock",
        "Isin": "US0378331005",
    },
    {
        "Code": "SPY",
        "Name": "SPDR S&P 500 ETF Trust",
        "Country": "USA",
        "Exchange": "US",
        "Currency": "USD",
        "Type": "ETF",
        "Isin": "US78462F1030",
    },
    {
        "Code": "XYZF",
        "Name": "Some Mutual Fund",
        "Country": "USA",
        "Exchange": "US",
        "Currency": "USD",
        "Type": "Mutual Fund",
        "Isin": "US0000000001",
    },
]

# 2024-01-02T00:00:00Z in nanoseconds
JAN_2_2024_NS = 1_704_153_600_000_000_000


class _CannedHandler(BaseHTTPRequestHandler):
    """Serve canned EODHD responses over loopback."""

    routes: ClassVar[dict] = {}

    def do_GET(self) -> None:  # noqa: N802
        """Serve a canned response for the request path."""
        body = self.routes.get(urlparse(self.path).path)

        if body is None:
            self.send_response(404)
            self.end_headers()
            return

        payload = body if isinstance(body, bytes) else json.dumps(body).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    def log_message(self, _format: str, *_args: object) -> None:
        """Silence the request log."""


@pytest.fixture(scope="module")
def base_url() -> Iterator[str]:
    """Provide the base URL of a local server serving canned EODHD responses."""
    _CannedHandler.routes = {
        "/eod/AAPL.US": EOD_ROWS,
        "/eod/AAPL.NOPE": {"code": 404, "message": "Not found"},
        "/eod/AAPL.EMPTY": [],
        "/exchange-symbol-list/US": SYMBOL_ROWS,
    }

    server = ThreadingHTTPServer(("127.0.0.1", 0), _CannedHandler)
    thread = Thread(target=server.serve_forever, daemon=True)
    thread.start()

    yield f"http://127.0.0.1:{server.server_address[1]}"

    server.shutdown()
    thread.join(timeout=5)


@pytest.fixture()
def loader(base_url: str) -> EodhdDataLoader:
    """Provide a loader pointed at the local server."""
    return EodhdDataLoader(api_key="test-token", base_url=base_url)


def test_loader_requires_an_api_key(monkeypatch: pytest.MonkeyPatch) -> None:
    """Test that a missing API token fails fast."""
    monkeypatch.delenv("EODHD_API_KEY", raising=False)

    with pytest.raises(ValueError, match="EODHD API token is not set"):
        EodhdDataLoader()


def test_loader_masks_the_api_key(loader: EodhdDataLoader) -> None:
    """Test that the API token is not disclosed by the masked accessor."""
    assert loader.api_key_masked() != "test-token"


def test_instrument_maps_the_exchange_suffix_to_the_venue(loader: EodhdDataLoader) -> None:
    """Test that an EODHD ticker round-trips through an instrument ID."""
    instrument = loader.instrument("AAPL.US")

    assert instrument.id == InstrumentId.from_str("AAPL.US")
    assert instrument.raw_symbol.value == "AAPL.US"


def test_instrument_rejects_a_ticker_without_an_exchange(loader: EodhdDataLoader) -> None:
    """Test that a ticker without an exchange suffix is rejected."""
    with pytest.raises(ValueError, match="expected '<code>.<exchange>'"):
        loader.instrument("AAPL")


@pytest.mark.asyncio()
async def test_bars_loads_the_rows_in_date_order(loader: EodhdDataLoader) -> None:
    """Test that daily bars are converted in the supplied row order."""
    bars = await loader.bars(
        instrument_id=InstrumentId.from_str("AAPL.US"),
        start="2024-01-02",
        end="2024-01-03",
    )

    assert len(bars) == 2
    assert str(bars[0].bar_type) == "AAPL.US-1-DAY-LAST-EXTERNAL"
    assert bars[0].open == Price.from_str("187.15")
    assert bars[0].close == Price.from_str("185.64")
    assert bars[0].volume == Quantity.from_str("82488700")
    assert bars[0].ts_event == JAN_2_2024_NS
    assert bars[0].ts_event < bars[1].ts_event


@pytest.mark.asyncio()
async def test_bars_maps_the_period_to_the_aggregation(loader: EodhdDataLoader) -> None:
    """Test that the period code selects the bar aggregation."""
    bars = await loader.bars(
        instrument_id=InstrumentId.from_str("AAPL.US"),
        start="2024-01-02",
        end="2024-01-03",
        period="w",
    )

    assert str(bars[0].bar_type) == "AAPL.US-1-WEEK-LAST-EXTERNAL"


@pytest.mark.asyncio()
async def test_bars_rejects_an_unknown_period(loader: EodhdDataLoader) -> None:
    """Test that an unsupported period code is rejected."""
    with pytest.raises(RuntimeError, match="Unsupported EODHD period"):
        await loader.bars(
            instrument_id=InstrumentId.from_str("AAPL.US"),
            start="2024-01-02",
            end="2024-01-03",
            period="x",
        )


@pytest.mark.asyncio()
async def test_bars_raises_on_the_provider_error_envelope(loader: EodhdDataLoader) -> None:
    """Test that a 200 response carrying the error envelope is surfaced as an error."""
    with pytest.raises(RuntimeError, match="Not found"):
        await loader.bars(
            instrument_id=InstrumentId.from_str("AAPL.NOPE"),
            start="2024-01-02",
            end="2024-01-03",
        )


@pytest.mark.asyncio()
async def test_bars_returns_empty_for_an_empty_payload(loader: EodhdDataLoader) -> None:
    """Test that an empty payload yields no bars."""
    bars = await loader.bars(
        instrument_id=InstrumentId.from_str("AAPL.EMPTY"),
        start="2024-01-02",
        end="2024-01-03",
    )

    assert bars == []


@pytest.mark.asyncio()
async def test_instruments_keeps_only_equity_types(loader: EodhdDataLoader) -> None:
    """Test that the symbol list is filtered to equity instrument types."""
    instruments = await loader.instruments("US")

    assert [instrument.id.value for instrument in instruments] == ["AAPL.US", "SPY.US"]
