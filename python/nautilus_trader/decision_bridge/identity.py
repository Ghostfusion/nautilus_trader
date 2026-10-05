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
Order identity: idempotent, per decision, per role, and immutable.

A decision produces at most one order set, ever, across restarts, re-reads and retries. The identity
of an order is derived from the identity of the decision plus what distinguishes one order from
another within it: the instrument, the leg and the order's role. Hashing the decision's key alone
would not be enough, because one decision may produce several orders.

The derivation is domain-separated and versioned, so a scheme change is visible in the order rather
than inferred from history. The identity is immutable for the lifetime of the decision: a venue
rejection, a cancellation, or a lost acknowledgement does not free it, so a replay produces no
second
order, and a deliberate retry requires a new execution-attempt identity derived from an explicit
retry
policy. The strictness is not caution -- from the engine's side a lost acknowledgement after venue
acceptance is indistinguishable from a rejection, so treating a rejection as permission to resubmit
risks duplicate exposure.

Three identifiers are deliberately distinct and must not be confused: the *decision* identity (one
instrument on one reference date), the artifact's `idempotency_key` (one producer run), and the
*client order id* (one order). This module mints only the third, from the second.

The encoding is chosen for the narrowest identifier budget known to this repository rather than for
the identifier's own check, which accepts any non-empty ASCII string. A 256-bit digest in
hexadecimal
does not fit a 36-character venue budget once a scheme token is prefixed, so the digest is encoded
in
lowercase RFC 4648 base32 and truncated to 130 bits, which keeps a bridge order id inside that
budget
while leaving a collision probability of order `n^2 / 2^131` -- far below the probability that two
distinct decisions share an idempotency key.
"""

from __future__ import annotations

from dataclasses import dataclass
from enum import Enum
from enum import unique
from hashlib import sha256
from typing import TYPE_CHECKING


if TYPE_CHECKING:
    from nautilus_trader.model import InstrumentId


# The scheme's full identity, recorded beside every order this module names so that a scheme change
# is visible rather than inferred.
ORDER_SCHEME = "ta-bridge-v1"

# The scheme's short token, which leads the identifier so an operator can recognise one by eye.
ORDER_ID_PREFIX = "ta1"

# The narrowest client order id budget this repository has to satisfy: a 36-character venue limit.
ORDER_ID_MAX_LENGTH = 36

# The digest body: 26 base32 symbols of 5 bits each, so 130 bits.
ORDER_ID_BODY_LENGTH = 26

# The width of the digest the body is taken from, and of one body symbol.
_DIGEST_BITS = 256
_BITS_PER_SYMBOL = 5

# RFC 4648 base32, lowercased: the identifier's permitted characters are any non-empty ASCII string,
# and a lowercase alphanumeric body is the subset every adapter's own check accepts.
_BASE32_ALPHABET = "abcdefghijklmnopqrstuvwxyz234567"


@unique
class OrderRole(Enum):
    """
    What an order does within a decision.

    The role is part of the identity because one decision can produce more than one order for the
    same
    instrument: the entry that opens a position and the exit that returns it to flat are different
    orders, and collapsing them would make the second one a replay of the first.
    """

    ENTRY = "ENTRY"
    EXIT = "EXIT"


@dataclass(frozen=True)
class OrderIdentity:
    """
    The identity of one order within one decision.

    Parameters
    ----------
    client_order_id : str
        The identifier to construct the order with.
    scheme : str
        The scheme the identifier was derived under, recorded beside the order.
    instrument_id : str
        The instrument the order is for.
    leg_id : str
        The leg the order is for. Empty for a single-leg instrument.
    order_role : OrderRole
        What the order does within the decision.
    attempt : str | None
        The execution-attempt identity, when an explicit retry policy minted one. `None` for a first
        attempt, so a retry is distinguishable from its original rather than a replay of it.

    """

    client_order_id: str
    scheme: str
    instrument_id: str
    leg_id: str
    order_role: OrderRole
    attempt: str | None


def order_payload(  # noqa: PLR0913 - the identity's declared fields
    *,
    idempotency_key: str,
    instrument_id: InstrumentId | str,
    leg_id: str = "",
    order_role: OrderRole,
    attempt: str | None = None,
    scheme: str = ORDER_SCHEME,
) -> str:
    """
    Return the domain-separated payload an order's identity is derived from.

    Parameters
    ----------
    idempotency_key : str
        The artifact's own key.
    instrument_id : InstrumentId | str
        The instrument the order is for.
    leg_id : str, optional
        The leg the order is for. Defaults to empty.
    order_role : OrderRole
        What the order does within the decision.
    attempt : str | None, optional
        The execution-attempt identity. Defaults to `None`.
    scheme : str, optional
        The scheme. Defaults to `ORDER_SCHEME`.

    Returns
    -------
    str

    """
    fields = (
        scheme,
        idempotency_key,
        str(instrument_id),
        leg_id,
        order_role.value,
        attempt or "",
    )
    return "|".join(fields)


def encode_order_id(payload: str) -> str:
    """
    Encode a payload as a bridge order identifier.

    Parameters
    ----------
    payload : str
        The domain-separated payload.

    Returns
    -------
    str
        An identifier no longer than `ORDER_ID_MAX_LENGTH`, in the identifier's permitted ASCII
        shape.

    """
    digest = sha256(payload.encode("utf-8")).digest()
    bits = int.from_bytes(digest, "big") >> (_DIGEST_BITS - ORDER_ID_BODY_LENGTH * _BITS_PER_SYMBOL)
    body = ""
    for _ in range(ORDER_ID_BODY_LENGTH):
        bits, index = divmod(bits, 32)
        body = _BASE32_ALPHABET[index] + body

    return f"{ORDER_ID_PREFIX}-{body}"


def order_identity(  # noqa: PLR0913 - the identity's declared fields
    *,
    idempotency_key: str,
    instrument_id: InstrumentId | str,
    order_role: OrderRole,
    leg_id: str = "",
    attempt: str | None = None,
    scheme: str = ORDER_SCHEME,
) -> OrderIdentity:
    """
    Derive one order's identity within a decision.

    Parameters
    ----------
    idempotency_key : str
        The artifact's own key.
    instrument_id : InstrumentId | str
        The instrument the order is for.
    order_role : OrderRole
        What the order does within the decision.
    leg_id : str, optional
        The leg the order is for. Defaults to empty.
    attempt : str | None, optional
        The execution-attempt identity. Defaults to `None`.
    scheme : str, optional
        The scheme. Defaults to `ORDER_SCHEME`.

    Returns
    -------
    OrderIdentity

    """
    payload = order_payload(
        idempotency_key=idempotency_key,
        instrument_id=instrument_id,
        leg_id=leg_id,
        order_role=order_role,
        attempt=attempt,
        scheme=scheme,
    )
    return OrderIdentity(
        client_order_id=encode_order_id(payload),
        scheme=scheme,
        instrument_id=str(instrument_id),
        leg_id=leg_id,
        order_role=order_role,
        attempt=attempt,
    )


def is_bridge_order_id(value: str) -> bool:
    """
    Return whether a value has the shape of a bridge-minted order identifier.

    The test is on the scheme's short token alone, so it can recognise an identifier this scheme
    minted without claiming that every identifier it recognises was minted here.

    Parameters
    ----------
    value : str
        The value to test.

    Returns
    -------
    bool

    """
    if not value.startswith(f"{ORDER_ID_PREFIX}-"):
        return False

    body = value[len(ORDER_ID_PREFIX) + 1 :]
    return len(body) == ORDER_ID_BODY_LENGTH and all(char in _BASE32_ALPHABET for char in body)
