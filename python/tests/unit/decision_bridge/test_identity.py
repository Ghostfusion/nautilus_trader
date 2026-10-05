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
Tests for the order identity scheme.
"""

from __future__ import annotations

from nautilus_trader.decision_bridge.identity import ORDER_ID_MAX_LENGTH
from nautilus_trader.decision_bridge.identity import ORDER_SCHEME
from nautilus_trader.decision_bridge.identity import OrderRole
from nautilus_trader.decision_bridge.identity import encode_order_id
from nautilus_trader.decision_bridge.identity import is_bridge_order_id
from nautilus_trader.decision_bridge.identity import order_identity
from nautilus_trader.decision_bridge.identity import order_payload
from nautilus_trader.model import ClientOrderId


KEY = "3f2504e0-4f89-41d3-9a0c-0305e82c3301"


def test_the_same_decision_instrument_leg_and_role_always_mint_the_same_identifier() -> None:
    """
    Test that identical key, instrument, and role inputs always mint the same identifier.
    """
    first = order_identity(
        idempotency_key=KEY,
        instrument_id="AAPL.XNAS",
        order_role=OrderRole.ENTRY,
    )
    second = order_identity(
        idempotency_key=KEY,
        instrument_id="AAPL.XNAS",
        order_role=OrderRole.ENTRY,
    )

    assert first.client_order_id == second.client_order_id


def test_orders_differing_only_by_role_have_different_identifiers() -> None:
    """
    Test that orders differing only by role receive different identifiers.
    """
    entry = order_identity(idempotency_key=KEY, instrument_id="AAPL.XNAS", order_role=OrderRole.ENTRY)
    exit_ = order_identity(idempotency_key=KEY, instrument_id="AAPL.XNAS", order_role=OrderRole.EXIT)

    assert entry.client_order_id != exit_.client_order_id


def test_orders_differing_only_by_instrument_have_different_identifiers() -> None:
    """
    Test that orders differing only by instrument receive different identifiers.
    """
    first = order_identity(idempotency_key=KEY, instrument_id="AAPL.XNAS", order_role=OrderRole.ENTRY)
    second = order_identity(idempotency_key=KEY, instrument_id="MSFT.XNAS", order_role=OrderRole.ENTRY)

    assert first.client_order_id != second.client_order_id


def test_orders_differing_only_by_leg_have_different_identifiers() -> None:
    """
    Test that orders differing only by leg identifier receive different identifiers.
    """
    first = order_identity(
        idempotency_key=KEY,
        instrument_id="ESM6.XCME",
        order_role=OrderRole.ENTRY,
        leg_id="ESM6.XCME",
    )
    second = order_identity(
        idempotency_key=KEY,
        instrument_id="ESM6.XCME",
        order_role=OrderRole.ENTRY,
        leg_id="NQM6.XCME",
    )

    assert first.client_order_id != second.client_order_id


def test_a_retry_under_an_explicit_attempt_identity_is_distinguishable_from_its_original() -> None:
    """
    Test that an explicit attempt identity yields an identifier distinct from the original.
    """
    original = order_identity(
        idempotency_key=KEY,
        instrument_id="AAPL.XNAS",
        order_role=OrderRole.ENTRY,
    )
    retry = order_identity(
        idempotency_key=KEY,
        instrument_id="AAPL.XNAS",
        order_role=OrderRole.ENTRY,
        attempt="attempt-2",
    )

    assert original.attempt is None
    assert retry.client_order_id != original.client_order_id


def test_the_scheme_version_is_part_of_the_payload_and_of_the_identity() -> None:
    """
    Test that the scheme version appears in the payload and distinguishes the identifier.
    """
    identity = order_identity(
        idempotency_key=KEY,
        instrument_id="AAPL.XNAS",
        order_role=OrderRole.ENTRY,
    )

    assert identity.scheme == ORDER_SCHEME
    assert ORDER_SCHEME in order_payload(
        idempotency_key=KEY,
        instrument_id="AAPL.XNAS",
        order_role=OrderRole.ENTRY,
    )
    assert identity.client_order_id != encode_order_id(
        order_payload(
            idempotency_key=KEY,
            instrument_id="AAPL.XNAS",
            order_role=OrderRole.ENTRY,
            scheme="ta-bridge-v2",
        ),
    )


def test_the_identifier_is_a_shape_the_identifier_check_accepts() -> None:
    """
    Test that a minted identifier is a bounded shape that ClientOrderId and the check accept.
    """
    identity = order_identity(
        idempotency_key=KEY,
        instrument_id="AAPL.XNAS",
        order_role=OrderRole.ENTRY,
    )

    identifier = ClientOrderId(identity.client_order_id)

    assert str(identifier) == identity.client_order_id
    assert len(identity.client_order_id) <= ORDER_ID_MAX_LENGTH
    assert is_bridge_order_id(identity.client_order_id)


def test_an_identifier_this_scheme_did_not_mint_is_not_recognised() -> None:
    """
    Test that identifiers not minted by this scheme are rejected.
    """
    assert not is_bridge_order_id("O-20261003-000001")
    assert not is_bridge_order_id("ta1-notbase32")
    assert not is_bridge_order_id("")
