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
Builders shared by the decision-bridge tests.

The builder for a canonical artifact seals it with an implementation of the producer's hash recipe
written *here* rather than with the bridge's own, so the bridge's recomputation is checked against an
independent implementation of the same published interface rather than against itself.
"""

from __future__ import annotations

import json
from datetime import UTC
from datetime import datetime
from hashlib import sha256
from typing import Any

from nautilus_trader.model import InstrumentId
from nautilus_trader.model import TradingCalendar


NS = 1_000_000_000

AAPL = InstrumentId.from_str("AAPL.XNAS")
PRODUCER_SERVICE = "tradingagents"
RUN_ID = "AAPL_20261002_120000"
IDEMPOTENCY_KEY = "3f2504e0-4f89-41d3-9a0c-0305e82c3301"
AUTHORIZED_PRODUCERS: frozenset[str] = frozenset({PRODUCER_SERVICE})


def unix_ns(year: int, month: int, day: int, hour: int = 0, minute: int = 0) -> int:
    """
    Return the UNIX nanoseconds of a UTC instant.
    """
    return int(datetime(year, month, day, hour, minute, tzinfo=UTC).timestamp()) * NS


def producer_recipe_digest(document: dict[str, Any]) -> str:
    """
    Return the artifact digest by the producer's published recipe.

    The recipe is SHA-256 over the document's body -- every key but the two hash fields -- serialised
    with sorted keys and the default string fallback. It is implemented here independently of the
    bridge so that the bridge's recomputation is checked against the interface rather than itself.
    """
    body = {key: value for key, value in document.items() if key not in ("artifact_sha256", "decision_hash")}
    return sha256(json.dumps(body, sort_keys=True, default=str).encode("utf-8")).hexdigest()


def canonical_artifact(*, seal: bool = True, **fields: Any) -> dict[str, Any]:
    """
    Return a canonical artifact, sealed unless the caller wants an unsealed one.
    """
    document: dict[str, Any] = {
        "schema_version": "1.2.0",
        "ticker": "AAPL",
        "effective_date": "2026-10-02",
        "rating": "Buy",
        "direction": None,
        "thesis": "a thesis",
        "rationale": "a rationale",
        "recommended_allocation_pct": 5.0,
        "confidence": 0.72,
        "position": {
            "target_notional": None,
            "entry_price": 191.34,
            "stop_loss": 188.0,
            "take_profit": 200.0,
            "size_pct_book": None,
        },
        "data_quality": "fresh",
        "price_caliber": None,
        "invalidations": ["a break below the stop"],
        "guardrail_reason": None,
        "risk_gate": {"verdict": "PASS", "reasons": []},
        "binding_constraint": None,
        "disclosure": {"sources_used": 3, "sources_empty": 1},
        "expires_at": "2026-10-05T20:00:00+00:00",
        "produced_at": "2026-10-02T17:00:00+00:00",
        "idempotency_key": IDEMPOTENCY_KEY,
        "producer": {"service": PRODUCER_SERVICE, "git_sha": "abc1234", "run_id": RUN_ID},
    }
    document.update(fields)
    if seal:
        document.pop("artifact_sha256", None)
        document["artifact_sha256"] = producer_recipe_digest(document)

    return document


def resolve_instrument(ticker: str) -> InstrumentId | None:
    """
    Resolve a ticker the way a configured bridge would.
    """
    return AAPL if ticker == "AAPL" else None


def bundled_calendar() -> TradingCalendar | None:
    """
    Return the bundled calendar the bridge resolves an instrument's key against.
    """
    return TradingCalendar.bundled("XNYS", "EQUITY")


def actionability_window() -> tuple[int, int]:
    """
    Return a produced/received pair inside the bundled calendar's coverage.
    """
    return unix_ns(2025, 6, 2, 17), unix_ns(2025, 6, 2, 17)


def covered_artifact(*, seal: bool = True, **fields: Any) -> dict[str, Any]:
    """
    Return a canonical artifact whose instants fall inside the bundled calendar's coverage.
    """
    document = canonical_artifact(
        seal=False,
        effective_date="2025-06-02",
        produced_at="2025-06-02T17:00:00+00:00",
        expires_at="2025-06-06T20:00:00+00:00",
    )
    document.update(fields)
    if seal:
        document.pop("artifact_sha256", None)
        document["artifact_sha256"] = producer_recipe_digest(document)

    return document
