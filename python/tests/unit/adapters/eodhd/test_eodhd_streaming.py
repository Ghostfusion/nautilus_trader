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
Test EODHD streaming against a local WebSocket server.

The frames are the ones the live channels send, so the parsing under test is the parsing the
provider produces.
"""

import asyncio
import base64
import hashlib
import json
import struct
from collections.abc import Iterator
from http.server import BaseHTTPRequestHandler
from http.server import ThreadingHTTPServer
from threading import Thread
from typing import ClassVar

import pytest

from nautilus_trader.adapters.eodhd import EodhdDataClientConfig
from nautilus_trader.adapters.eodhd import EodhdDataClientFactory
from nautilus_trader.common import Environment
from nautilus_trader.live import LiveNode
from nautilus_trader.model import ClientId
from nautilus_trader.model import InstrumentId
from nautilus_trader.model import Price
from nautilus_trader.model import Quantity
from nautilus_trader.model import TraderId
from nautilus_trader.trading import Strategy


WS_GUID = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"

INSTRUMENT_ID = "AAPL.US"

# Frames captured from the live channels.
AUTHORIZED = '{"status_code":200,"message":"Authorized"}'
TRADE_FRAMES = [
    '{"s":"AAPL","p":330.78,"c":[],"v":5,"dp":false,"ms":"extended-hours","t":1790899194028}',
]
QUOTE_FRAMES = [
    '{"s":"AAPL","ap":330.8,"as":15,"bp":330,"bs":67,"t":1790899198000}',
]


class _StreamWebSocketHandler(BaseHTTPRequestHandler):
    """Serve a canned WebSocket stream over loopback and record subscribe requests."""

    protocol_version = "HTTP/1.1"
    received: ClassVar[dict] = {}

    def do_GET(self) -> None:  # noqa: N802
        """Complete the WebSocket handshake and answer each subscription."""
        key = self.headers.get("Sec-WebSocket-Key", "")
        accept = base64.b64encode(
            hashlib.sha1((key + WS_GUID).encode()).digest(),
        ).decode()

        self.send_response(101)
        self.send_header("Upgrade", "websocket")
        self.send_header("Connection", "Upgrade")
        self.send_header("Sec-WebSocket-Accept", accept)
        self.end_headers()

        self._send_text(AUTHORIZED)

        frames = QUOTE_FRAMES if self.path.startswith("/ws/us-quote") else TRADE_FRAMES

        while True:
            frame = self._read_frame()

            if frame is None:
                return

            opcode, payload = frame

            if opcode == 0x8:
                return

            if opcode == 0x9:
                self._send_frame(0xA, payload)
                continue

            if opcode != 0x1:
                continue

            text = payload.decode("utf-8", "replace")
            self.received.setdefault(self.path, []).append(json.loads(text))

            if json.loads(text).get("action") == "subscribe":
                for canned in frames:
                    self._send_text(canned)

    def _read_exact(self, count: int) -> bytes | None:
        data = b""

        while len(data) < count:
            chunk = self.rfile.read(count - len(data))

            if not chunk:
                return None

            data += chunk

        return data

    def _read_frame(self) -> tuple[int, bytes] | None:
        header = self._read_exact(2)

        if header is None:
            return None

        opcode = header[0] & 0x0F
        masked = bool(header[1] & 0x80)
        length = header[1] & 0x7F

        if length == 126:
            extra = self._read_exact(2)
            if extra is None:
                return None
            length = struct.unpack(">H", extra)[0]
        elif length == 127:
            extra = self._read_exact(8)
            if extra is None:
                return None
            length = struct.unpack(">Q", extra)[0]

        mask = self._read_exact(4) if masked else None
        payload = self._read_exact(length) if length else b""

        if payload is None:
            return None

        if mask:
            payload = bytes(byte ^ mask[index % 4] for index, byte in enumerate(payload))

        return opcode, payload

    def _send_text(self, text: str) -> None:
        self._send_frame(0x1, text.encode())

    def _send_frame(self, opcode: int, payload: bytes) -> None:
        header = bytes([0x80 | opcode])
        size = len(payload)

        if size < 126:
            header += bytes([size])
        elif size < 65536:
            header += bytes([126]) + struct.pack(">H", size)
        else:
            header += bytes([127]) + struct.pack(">Q", size)

        self.wfile.write(header + payload)
        self.wfile.flush()

    def log_message(self, _format: str, *_args: object) -> None:
        """Silence the request log."""


class _Collector(Strategy):
    """Collects the trades and quotes the EODHD data client streams."""

    def __init__(self) -> None:
        """Initialize the collector."""
        super().__init__()
        self.trades: list = []
        self.quotes: list = []

    def on_start(self) -> None:
        """Subscribe to trades and quotes on the EODHD client."""
        instrument_id = InstrumentId.from_str(INSTRUMENT_ID)
        self.subscribe_trades(instrument_id, client_id=ClientId.from_str("EODHD"))
        self.subscribe_quotes(instrument_id, client_id=ClientId.from_str("EODHD"))

    def on_trade(self, trade) -> None:
        """Record a delivered trade."""
        self.trades.append(trade)

    def on_quote(self, quote) -> None:
        """Record a delivered quote."""
        self.quotes.append(quote)


@pytest.fixture()
def ws_base_url() -> Iterator[str]:
    """Provide the streaming base URL of a local WebSocket server."""
    _StreamWebSocketHandler.received = {}

    server = ThreadingHTTPServer(("127.0.0.1", 0), _StreamWebSocketHandler)
    thread = Thread(target=server.serve_forever, daemon=True)
    thread.start()

    try:
        yield f"ws://127.0.0.1:{server.server_address[1]}/ws"
    finally:
        server.shutdown()
        thread.join(timeout=5)


@pytest.mark.asyncio()
async def test_streaming_delivers_trades_and_quotes_to_a_strategy(ws_base_url: str) -> None:
    """Test that the streaming channels reach a subscribed strategy."""
    strategy = _Collector()
    node = (
        LiveNode.builder(
            "EODHD-STREAM-PYTEST-001",
            TraderId.from_str("TESTER-001"),
            Environment.SANDBOX,
        )
        .add_data_client(
            None,
            EodhdDataClientFactory(),
            EodhdDataClientConfig(
                api_key="test-token",
                ws_base_url=ws_base_url,
                load_instruments=False,
                streaming=True,
            ),
        )
        .build()
    )
    node.add_strategy(strategy)

    run_task = asyncio.create_task(node.run_async())
    await asyncio.sleep(8)
    node.handle().stop()
    await asyncio.wait_for(run_task, timeout=25)

    assert len(strategy.trades) == 1, strategy.trades
    trade = strategy.trades[0]
    assert trade.instrument_id == InstrumentId.from_str(INSTRUMENT_ID)
    assert trade.price == Price.from_str("330.78")
    assert trade.size == Quantity.from_str("5")
    assert trade.aggressor_side.name == "NO_AGGRESSOR"
    assert int(trade.ts_event) == 1_790_899_194_028_000_000

    assert len(strategy.quotes) == 1, strategy.quotes
    quote = strategy.quotes[0]
    assert quote.instrument_id == InstrumentId.from_str(INSTRUMENT_ID)
    assert quote.bid_price == Price.from_str("330.00")
    assert quote.bid_size == Quantity.from_str("67")
    assert quote.ask_price == Price.from_str("330.80")
    assert quote.ask_size == Quantity.from_str("15")
    assert int(quote.ts_event) == 1_790_899_198_000_000_000


@pytest.mark.asyncio()
async def test_streaming_sends_the_documented_subscribe_message(ws_base_url: str) -> None:
    """Test that a subscription carries the symbol as one comma separated string."""
    strategy = _Collector()
    node = (
        LiveNode.builder(
            "EODHD-STREAM-PYTEST-002",
            TraderId.from_str("TESTER-001"),
            Environment.SANDBOX,
        )
        .add_data_client(
            None,
            EodhdDataClientFactory(),
            EodhdDataClientConfig(
                api_key="test-token",
                ws_base_url=ws_base_url,
                load_instruments=False,
                streaming=True,
            ),
        )
        .build()
    )
    node.add_strategy(strategy)

    run_task = asyncio.create_task(node.run_async())
    await asyncio.sleep(8)
    node.handle().stop()
    await asyncio.wait_for(run_task, timeout=25)

    received = _StreamWebSocketHandler.received

    assert received["/ws/us?api_token=test-token"] == [
        {"action": "subscribe", "symbols": "AAPL"},
    ], received
    assert received["/ws/us-quote?api_token=test-token"] == [
        {"action": "subscribe", "symbols": "AAPL"},
    ], received


def test_streaming_defaults_to_disabled() -> None:
    """Test that the WebSocket path is opt in."""
    assert EodhdDataClientConfig().streaming is False
