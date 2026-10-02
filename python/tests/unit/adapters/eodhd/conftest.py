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
Shared fixtures for the EODHD adapter tests.
"""

import json
from collections.abc import Iterator
from http.server import BaseHTTPRequestHandler
from http.server import ThreadingHTTPServer
from threading import Thread
from typing import ClassVar
from urllib.parse import urlparse

import pytest


class CannedHandler(BaseHTTPRequestHandler):
    """Serve canned EODHD responses over loopback and record the requested paths."""

    routes: ClassVar[dict] = {}
    requests: ClassVar[list] = []

    def do_GET(self) -> None:  # noqa: N802
        """Serve a canned response for the request path."""
        path = urlparse(self.path).path
        self.requests.append(path)
        body = self.routes.get(path)

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


@pytest.fixture()
def canned_routes() -> dict:
    """Provide the route table the canned server serves, keyed by request path."""
    return CannedHandler.routes


@pytest.fixture()
def canned_requests() -> list:
    """Provide the request paths the canned server has served, in order."""
    return CannedHandler.requests


@pytest.fixture()
def canned_server(canned_routes: dict) -> Iterator[str]:
    """Provide the base URL of a local server serving canned EODHD responses."""
    # Cleared in place so a fixture that holds the list observes the requests.
    canned_routes.clear()
    CannedHandler.requests.clear()

    server = ThreadingHTTPServer(("127.0.0.1", 0), CannedHandler)
    thread = Thread(target=server.serve_forever, daemon=True)
    thread.start()

    try:
        yield f"http://127.0.0.1:{server.server_address[1]}"
    finally:
        server.shutdown()
        thread.join(timeout=5)
