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
The boundary reader: what an admitted artifact is, and what refuses it.

A record is admitted only if it validates against the schema version it declares, carries every
field
that version requires, its artifact hash recomputes, its producer is both identified and authorised,
its instrument resolves, and its advisory allocation is inside its declared domain. Anything else is
refused with a reason drawn from the closed vocabulary, and a field this reader does not know is
recorded rather than ignored -- the contract ignores unknown fields, so a reader that did the same
would lose a field in silence.

The hash is recomputed from the artifact *body*, and the recipe is the producer's own, because a
hash
that either side computes differently is not a check. The producer documents it as SHA-256 over
every
key except the two hash fields, serialised with sorted keys, and both sides must therefore agree on
the serialisation and not merely on the algorithm.

Three identifiers are deliberately distinct and must not be confused. The *decision* identity names
what the artifact is about -- one instrument on one reference date -- so a second analysis of the
same
name on the same day is a revision of a decision rather than a second decision. The *revision*
identity names one artifact, so re-reading the same document is a no-op. And the *order* identity,
which this module does not mint, names one order within a decision.

The producer's own vocabulary reserves three names it forbids itself to set; this reader therefore
never reads them as claims. A value under one of those names is an unknown field, recorded rather
than
honoured, so the day a producer sets one it is visible on the ledger instead of silently granting
permission.
"""

from __future__ import annotations

import json
import re
from collections.abc import Mapping
from dataclasses import dataclass
from datetime import UTC
from datetime import date
from datetime import datetime
from hashlib import sha256
from typing import TYPE_CHECKING
from typing import Any
from uuid import UUID

from nautilus_trader.decision_bridge.contract import RESERVED_PRODUCER_NAMES
from nautilus_trader.decision_bridge.contract import Refusal
from nautilus_trader.decision_bridge.contract import RefusalCode
from nautilus_trader.decision_bridge.numeric import is_allocation_in_domain


if TYPE_CHECKING:
    from collections.abc import Callable
    from collections.abc import Iterable

    from nautilus_trader.model import InstrumentId


# The producer's version pattern, MAJOR.MINOR.PATCH. An unknown MAJOR is refused rather than read:
# a schema this reader has never seen may mean anything, and guessing is what the fail-closed rule
# exists to prevent.
SCHEMA_VERSION_PATTERN = re.compile(r"^(?P<major>\d+)\.(?P<minor>\d+)\.(?P<patch>\d+)$")
SUPPORTED_MAJOR = 1
LEGACY_VERSION = "1.0.0"

# The field set that extends at 1.1.0, from the contract's own `x-required-when` block.
REQUIRED_AT_V1_1_0: tuple[str, ...] = (
    "expires_at",
    "idempotency_key",
    "producer",
    "artifact_sha256",
)

# The fields every version requires.
BASE_REQUIRED: tuple[str, ...] = ("ticker", "effective_date")

# The two fields the artifact's own hash covers by exclusion.
HASH_FIELDS: tuple[str, ...] = ("artifact_sha256", "decision_hash")

DATE_PATTERN = re.compile(r"^\d{4}-\d{2}-\d{2}$")
SHA256_PATTERN = re.compile(r"^[0-9a-f]{64}$")

DIRECTIONS: frozenset[str] = frozenset({"add", "buy", "exit", "hold", "none", "reduce", "sell"})
ACTIONS: frozenset[str] = frozenset({"BUY", "HOLD", "REDUCE", "EXIT", "NONE"})
DATA_QUALITY: frozenset[str] = frozenset({"fresh", "partial", "stale", "unknown"})

# Every top-level key this reader knows. Everything else is recorded as an unknown field rather than
# dropped, because the contract admits unknown fields and therefore a drifted producer would
# otherwise lose one in silence.
KNOWN_FIELDS: frozenset[str] = frozenset(
    {
        "schema_version",
        "ticker",
        "effective_date",
        "rating",
        "direction",
        "action",
        "opportunity_score",
        "opportunity_score_reason",
        "net_beta",
        "net_beta_reason",
        "confidence",
        "thesis",
        "rationale",
        "recommended_allocation_pct",
        "position",
        "entry_exit",
        "data_quality",
        "price_caliber",
        "invalidations",
        "guardrail_reason",
        "risk_context",
        "risk_gate",
        "disclosure",
        "security_signal",
        "portfolio_action",
        "combined_action",
        "gated",
        "binding_constraint",
        "action_basis",
        "binding_reason",
        "expires_at",
        "produced_at",
        "idempotency_key",
        "producer",
        "artifact_sha256",
        "decision_hash",
    },
)


def parse_schema_version(raw: Mapping[str, Any]) -> tuple[int, int, int] | None:
    """
    Return the schema version an artifact declares.

    Parameters
    ----------
    raw : Mapping[str, Any]
        The artifact as read from disk.

    Returns
    -------
    tuple[int, int, int] | None
        The version, or `None` when the field is absent, mistyped or does not match the pattern. A
        legacy artifact that omits the field reads as `1.0.0`, which is the producer's own rule.

    """
    declared = raw.get("schema_version")
    if declared is None:
        return (1, 0, 0)
    if not isinstance(declared, str):
        return None

    match = SCHEMA_VERSION_PATTERN.match(declared)
    if match is None:
        return None

    return (int(match["major"]), int(match["minor"]), int(match["patch"]))


def artifact_body(raw: Mapping[str, Any]) -> dict[str, Any]:
    """
    Return the artifact minus both of its hash fields.

    Parameters
    ----------
    raw : Mapping[str, Any]
        The artifact as read from disk.

    Returns
    -------
    dict[str, Any]

    """
    return {key: value for key, value in raw.items() if key not in HASH_FIELDS}


def artifact_digest(raw: Mapping[str, Any]) -> str:
    """
    Return the SHA-256 the artifact's own `artifact_sha256` field must equal.

    The recipe is the producer's, and it is the body rather than the file bytes, so re-indenting the
    document cannot change a hash. Sorted keys and the default `str` fallback are part of the
    interface, not an implementation detail: a hash either side computes differently is not a check.

    Parameters
    ----------
    raw : Mapping[str, Any]
        The artifact as read from disk.

    Returns
    -------
    str
        The digest, as lowercase hexadecimal.

    """
    encoded = json.dumps(artifact_body(raw), sort_keys=True, default=str).encode("utf-8")
    return sha256(encoded).hexdigest()


def decision_id_for(
    instrument_id: InstrumentId | str,
    effective_date: date,
    revision_id: str,
) -> str:
    """
    Return the ledger identity of one artifact.

    The *decision* is one instrument on one reference date, and it is a chain rather than a row: a
    second analysis of the same name on the same day is a revision of that decision, and the two
    must
    never be confused. The ledger's row identity is therefore the artifact itself -- the decision's
    own
    key plus the artifact's revision identity -- and the chain is readable from the instrument and
    the
    reference date the row already carries. The design's ledger field list has no separate revision
    field, which is why the row identity carries it.

    Parameters
    ----------
    instrument_id : InstrumentId | str
        The resolved instrument.
    effective_date : date
        The artifact's reference date.
    revision_id : str
        The artifact's own revision identity.

    Returns
    -------
    str

    """
    payload = f"ta-decision-v1|{instrument_id}|{effective_date.isoformat()}|{revision_id}"
    return sha256(payload.encode("utf-8")).hexdigest()


def revision_id_for(idempotency_key: str | None, artifact_sha256: str) -> str:
    """
    Return the identity of one artifact.

    Parameters
    ----------
    idempotency_key : str | None
        The artifact's declared idempotency key, when it declares one.
    artifact_sha256 : str
        The artifact's verified body digest.

    Returns
    -------
    str

    """
    payload = f"ta-revision-v1|{idempotency_key or ''}|{artifact_sha256}"
    return sha256(payload.encode("utf-8")).hexdigest()


def is_canonical_v4_uuid(value: str) -> bool:
    """
    Return whether a value is a canonical version 4 UUID.

    The producer's own conformance check requires the canonical lowercase form rather than merely a
    version-4 shape, so a key that would not compare equal to its own parse is refused here too.

    Parameters
    ----------
    value : str
        The value to test.

    Returns
    -------
    bool

    """
    try:
        parsed = UUID(value)
    except (TypeError, ValueError):
        return False

    return parsed.version == 4 and str(parsed) == value  # noqa: PLR2004 - schema major


def iso_to_unix_nanos(value: str) -> int:
    """
    Return the UNIX nanoseconds of an ISO 8601 date-time.

    Parameters
    ----------
    value : str
        The date-time, which must carry an offset. A naive stamp is refused by the caller, because
        an
        instant without a zone is an ambiguous instant and an artifact's availability is not.

    Returns
    -------
    int

    Raises
    ------
    ValueError
        If the value is not an ISO 8601 date-time, or carries no offset.

    """
    stamp = datetime.fromisoformat(value)
    if stamp.tzinfo is None:
        raise ValueError("a date-time without an offset is ambiguous")

    delta = stamp.astimezone(UTC) - datetime(1970, 1, 1, tzinfo=UTC)
    return (
        delta.days * 86_400_000_000_000
        + delta.seconds * 1_000_000_000
        + delta.microseconds * 1_000
    )


@dataclass(frozen=True)
class ResearchDecision:
    """
    An admitted research decision artifact.

    Parameters
    ----------
    decision_id : str
        The identity of the decision, over the instrument and the reference date.
    revision_id : str
        The identity of this artifact, over its idempotency key and verified body digest.
    instrument_id : InstrumentId
        The resolved instrument.
    schema_version : str
        The schema version the artifact declares.
    ticker : str
        The ticker the artifact states.
    effective_date : date
        The reference date of the analysis.
    artifact_sha256 : str
        The artifact's body digest, recomputed rather than trusted.
    produced_at : int | None
        The instant the artifact existed, as UNIX nanoseconds.
    expires_at : int | None
        The instant the artifact ceases to be actionable, as UNIX nanoseconds.
    rating : str | None
        The research view, unmapped. The rating policy maps it.
    allocation_pct : float | None
        The advisory allocation in its declared domain, unnormalised.
    idempotency_key : str | None
        The artifact's declared key.
    producer_service : str | None
        The producer's service identity.
    producer_run_id : str | None
        The producer's run identity, carried into the signal's provenance.
    gate_verdict : Any
        The permission claim's value, unread as a permission.
    gate_reasons : tuple[str, ...]
        The claim's own reasons, recorded as evidence.
    advisory : Mapping[str, Any]
        The advisory levels and quality facts, carried into the signal's provenance rather than
        consumed as an instruction.
    unknown_fields : tuple[str, ...]
        Top-level fields this reader does not know, recorded rather than ignored.
    reserved_names_present : tuple[str, ...]
        Reserved producer names that carried a value, recorded because a producer is forbidden to
        set
        them and their presence is evidence about the producer.

    """

    decision_id: str
    revision_id: str
    instrument_id: InstrumentId
    schema_version: str
    ticker: str
    effective_date: date
    artifact_sha256: str
    produced_at: int | None
    expires_at: int | None
    rating: str | None
    allocation_pct: float | None
    idempotency_key: str | None
    producer_service: str | None
    producer_run_id: str | None
    gate_verdict: Any
    gate_reasons: tuple[str, ...]
    advisory: Mapping[str, Any]
    unknown_fields: tuple[str, ...]
    reserved_names_present: tuple[str, ...]


def _require_string(raw: Mapping[str, Any], key: str) -> str | Refusal:
    value = raw.get(key)
    if not isinstance(value, str) or not value:
        return Refusal(RefusalCode.SCHEMA_INVALID, f"{key} must be a non-empty string")

    return value


def _optional_string(raw: Mapping[str, Any], key: str) -> str | Refusal | None:
    value = raw.get(key)
    if value is None:
        return None
    if not isinstance(value, str):
        return Refusal(RefusalCode.SCHEMA_INVALID, f"{key} must be a string or null")

    return value


def _optional_instant(raw: Mapping[str, Any], key: str) -> int | Refusal | None:
    value = raw.get(key)
    if value is None:
        return None
    if not isinstance(value, str):
        return Refusal(RefusalCode.SCHEMA_INVALID, f"{key} must be a date-time string or null")
    try:
        return iso_to_unix_nanos(value)
    except ValueError as exc:
        return Refusal(
            RefusalCode.SCHEMA_INVALID,
            f"{key}={value!r} is not a usable instant: {exc}",
        )


def _optional_number(raw: Mapping[str, Any], key: str) -> float | Refusal | None:
    value = raw.get(key)
    if value is None:
        return None
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        return Refusal(RefusalCode.SCHEMA_INVALID, f"{key} must be a number or null")

    return float(value)


def admit(  # noqa: C901, PLR0911, PLR0912, PLR0915 - the first failing stage decides the refusal
    raw: Any,  # noqa: ANN401 - the artifact is read before validation
    *,
    resolve_instrument: Callable[[str], InstrumentId | None],
    authorized_producers: Iterable[str],
    known_fields: frozenset[str] = KNOWN_FIELDS,
) -> ResearchDecision | Refusal:
    """
    Admit an artifact, or refuse it with the first failing stage's reason.

    The order of the checks is part of the interface: schema, then hash, then producer identity,
    then
    producer authorisation, then instrument, then the advisory allocation's domain. A record that
    fails two checks is refused with the earlier one's reason, so the refusal is a function of the
    artifact rather than of an implementation.

    Parameters
    ----------
    raw : Mapping[str, Any]
        The artifact as read from disk.
    resolve_instrument : Callable[[str], InstrumentId | None]
        Resolves the artifact's ticker to an instrument, returning `None` when the ticker is
        unknown.
    authorized_producers : Iterable[str]
        The producer services this bridge accepts.
    known_fields : frozenset[str], optional
        The top-level fields this reader knows. Defaults to `KNOWN_FIELDS`.

    Returns
    -------
    ResearchDecision | Refusal

    """
    # Stage: schema.
    if not isinstance(raw, Mapping):
        return Refusal(
            RefusalCode.SCHEMA_INVALID,
            f"the artifact is {type(raw).__name__}, not an object",
        )

    version = parse_schema_version(raw)
    if version is None:
        return Refusal(
            RefusalCode.SCHEMA_INVALID,
            f"schema_version={raw.get('schema_version')!r} does not match MAJOR.MINOR.PATCH",
        )
    if version[0] != SUPPORTED_MAJOR:
        return Refusal(
            RefusalCode.SCHEMA_INVALID,
            f"schema_version major {version[0]} is not supported "
            f"(this reader knows {SUPPORTED_MAJOR})",
        )

    for key in BASE_REQUIRED:
        if raw.get(key) is None:
            return Refusal(RefusalCode.SCHEMA_INVALID, f"{key} is required")

    if version >= (1, 1, 0):
        for key in REQUIRED_AT_V1_1_0:
            if raw.get(key) is None:
                return Refusal(
                    RefusalCode.SCHEMA_INVALID,
                    f"{key} is required at schema version "
                    f"{'.'.join(str(part) for part in version)}",
                )

    ticker = _require_string(raw, "ticker")
    if isinstance(ticker, Refusal):
        return ticker

    declared_date = _require_string(raw, "effective_date")
    if isinstance(declared_date, Refusal):
        return declared_date
    if DATE_PATTERN.match(declared_date) is None:
        return Refusal(
            RefusalCode.SCHEMA_INVALID,
            f"effective_date={declared_date!r} is not a YYYY-MM-DD date",
        )
    effective_date = date.fromisoformat(declared_date)

    rating = _optional_string(raw, "rating")
    if isinstance(rating, Refusal):
        return rating

    direction = _optional_string(raw, "direction")
    if isinstance(direction, Refusal):
        return direction
    if direction is not None and direction not in DIRECTIONS:
        return Refusal(
            RefusalCode.SCHEMA_INVALID,
            f"direction={direction!r} is not in the declared set",
        )

    action = _optional_string(raw, "action")
    if isinstance(action, Refusal):
        return action
    if action is not None and action not in ACTIONS:
        return Refusal(RefusalCode.SCHEMA_INVALID, f"action={action!r} is not in the declared set")

    data_quality = _optional_string(raw, "data_quality")
    if isinstance(data_quality, Refusal):
        return data_quality
    if data_quality is not None and data_quality not in DATA_QUALITY:
        return Refusal(
            RefusalCode.SCHEMA_INVALID,
            f"data_quality={data_quality!r} is not in the declared set",
        )

    confidence = _optional_number(raw, "confidence")
    if isinstance(confidence, Refusal):
        return confidence
    if confidence is not None and not 0.0 <= confidence <= 1.0:
        return Refusal(
            RefusalCode.SCHEMA_INVALID,
            f"confidence={confidence} is outside its declared domain 0..1",
        )

    allocation_pct = _optional_number(raw, "recommended_allocation_pct")
    if isinstance(allocation_pct, Refusal):
        return allocation_pct

    produced_at = _optional_instant(raw, "produced_at")
    if isinstance(produced_at, Refusal):
        return produced_at

    expires_at = _optional_instant(raw, "expires_at")
    if isinstance(expires_at, Refusal):
        return expires_at

    idempotency_key = _optional_string(raw, "idempotency_key")
    if isinstance(idempotency_key, Refusal):
        return idempotency_key
    if idempotency_key is not None and not is_canonical_v4_uuid(idempotency_key):
        return Refusal(
            RefusalCode.SCHEMA_INVALID,
            f"idempotency_key={idempotency_key!r} is not a canonical version 4 UUID",
        )

    declared_sha = raw.get("artifact_sha256")
    if declared_sha is not None and (
        not isinstance(declared_sha, str) or SHA256_PATTERN.match(declared_sha) is None
    ):
        return Refusal(
            RefusalCode.SCHEMA_INVALID,
            f"artifact_sha256={declared_sha!r} is not a lowercase SHA-256 hex digest",
        )

    producer = raw.get("producer")
    if producer is not None and not isinstance(producer, Mapping):
        return Refusal(RefusalCode.SCHEMA_INVALID, "producer must be an object or null")

    risk_gate = raw.get("risk_gate")
    if risk_gate is not None and not isinstance(risk_gate, Mapping):
        return Refusal(RefusalCode.SCHEMA_INVALID, "risk_gate must be an object or null")

    position = raw.get("position")
    if position is not None and not isinstance(position, Mapping):
        return Refusal(RefusalCode.SCHEMA_INVALID, "position must be an object or null")

    # Stage: hash.
    computed_sha = artifact_digest(raw)
    if declared_sha is not None and declared_sha != computed_sha:
        return Refusal(
            RefusalCode.HASH_INVALID,
            f"artifact_sha256={declared_sha} does not recompute from the artifact body",
        )

    # Stage: producer identity.
    service: str | None = None
    run_id: str | None = None
    if producer is not None:
        declared_service = producer.get("service")
        declared_run = producer.get("run_id")
        service = (
            declared_service if isinstance(declared_service, str) and declared_service else None
        )
        run_id = declared_run if isinstance(declared_run, str) and declared_run else None

    if service is None:
        return Refusal(
            RefusalCode.PRODUCER_UNKNOWN,
            "the artifact identifies no producer service",
        )

    # Stage: producer authorisation, which is a different property from identity.
    authorized = frozenset(authorized_producers)
    if service not in authorized:
        return Refusal(
            RefusalCode.PRODUCER_UNAUTHORIZED,
            f"producer {service!r} is identified but not authorised",
        )

    # Stage: instrument.
    instrument_id = resolve_instrument(ticker)
    if instrument_id is None:
        return Refusal(RefusalCode.INSTRUMENT_UNKNOWN, f"ticker {ticker!r} does not resolve")

    # Stage: the advisory allocation's domain, validated rather than clamped.
    if not is_allocation_in_domain(allocation_pct):
        return Refusal(
            RefusalCode.ALLOCATION_INVALID,
            f"recommended_allocation_pct={allocation_pct!r} is outside its declared domain 0..100",
        )

    revision_id = revision_id_for(idempotency_key, computed_sha)
    unknown_fields = tuple(sorted(key for key in raw if key not in known_fields))
    reserved_present = tuple(
        name for name in RESERVED_PRODUCER_NAMES if raw.get(name) not in (None, "", {}, [])
    )

    gate_reasons: tuple[str, ...] = ()
    gate_verdict: Any = None
    if risk_gate is not None:
        gate_verdict = risk_gate.get("verdict")
        raw_reasons = risk_gate.get("reasons")
        if isinstance(raw_reasons, (list, tuple)):
            gate_reasons = tuple(str(reason) for reason in raw_reasons)

    advisory: dict[str, Any] = {
        "confidence": confidence,
        "data_quality": data_quality,
        "binding_constraint": raw.get("binding_constraint"),
        "guardrail_reason": raw.get("guardrail_reason"),
        "price_caliber": raw.get("price_caliber"),
        "position": dict(position) if position is not None else None,
    }

    return ResearchDecision(
        decision_id=decision_id_for(instrument_id, effective_date, revision_id),
        revision_id=revision_id,
        instrument_id=instrument_id,
        schema_version=".".join(str(part) for part in version),
        ticker=ticker,
        effective_date=effective_date,
        artifact_sha256=computed_sha,
        produced_at=produced_at,
        expires_at=expires_at,
        rating=rating,
        allocation_pct=allocation_pct,
        idempotency_key=idempotency_key,
        producer_service=service,
        producer_run_id=run_id,
        gate_verdict=gate_verdict,
        gate_reasons=gate_reasons,
        advisory=advisory,
        unknown_fields=unknown_fields,
        reserved_names_present=reserved_present,
    )
