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
The admission ledger: a first-class audit record of every research decision the bridge saw.

A refusal is an outcome, not a discarded record (design 5.1), so the ledger keeps every arrival,
including the ones that never became executable and the ones that were a repeated delivery. It is
the single place the chain from a decision to a fill can be read: a decision record carries the
signal, target, order and fill identifiers its later stages write, and the ledger answers the same
question in reverse, from a fill or an order identifier back to the decision that caused it.

The record is append-only. A later stage fills its own fields on the same row identity rather than
rewriting an earlier stage's answer, so a stage that did not run leaves its quantity absent
(`None`) rather than zero; a zero would claim the exposure was considered and refused when it was
never reached.

Persistence follows the optimization store's convention (`optimization/persistence.py`): one JSON
document per decision identity under a base path, plus a manifest. Loading reads those documents
directly, so a query never requires a replay of the stream that produced them.
"""

from __future__ import annotations

import json
import re
from dataclasses import dataclass
from dataclasses import replace
from pathlib import Path
from typing import TYPE_CHECKING

from nautilus_trader.decision_bridge.contract import Diagnostic
from nautilus_trader.decision_bridge.contract import RefusalCode


if TYPE_CHECKING:
    from collections.abc import Iterable


SCHEMA = "nautilus-decision-ledger/v1"
MANIFEST_NAME = "manifest.json"

_DECISIONS_DIR = "decisions"

# The result of the admission stage (design 5.1). The lifecycle states are `ADMITTED`, `REFUSED`
# and `DUPLICATE`; the first two are recorded for a validated or refused artifact, the third for a
# repeated delivery of one that was already admitted. The terminal dispositions of an admitted
# artifact (a signal, no signal, a denial, a fill) are recorded by the fields each stage writes, not
# by this value.
ADMITTED = "ADMITTED"
REFUSED = "REFUSED"
DUPLICATE = "DUPLICATE"

ADMISSION_RESULTS: tuple[str, ...] = (ADMITTED, REFUSED, DUPLICATE)

# The five exposure quantities of B15, in the order the exposure can shrink (design 6.5). The name
# of each is the name of the field on `DecisionRecord` that a stage writes.
EXPOSURE_STAGES: tuple[str, ...] = (
    "research_requested",
    "bridge_capped",
    "engine_constructed",
    "risk_approved",
    "actually_filled",
)

# What an order or fill with no decision identity is reported as, rather than being assigned to the
# nearest decision (B10).
UNATTRIBUTED = "UNATTRIBUTED"

_UNSAFE_KEY = re.compile(r"[^A-Za-z0-9._-]+")


def _key(decision_id: str) -> str:
    """
    Return a filesystem-safe key for a decision identity.
    """
    return _UNSAFE_KEY.sub("_", decision_id)


def _write_json(path: Path, payload: object) -> None:
    """
    Write the payload as strict, canonical, machine-readable JSON.
    """
    text = json.dumps(payload, sort_keys=True, indent=2, ensure_ascii=True, allow_nan=False)
    path.write_text(f"{text}\n", encoding="utf-8")


def _read_json(path: Path) -> dict:
    """
    Read a stored JSON document.
    """
    return json.loads(path.read_text(encoding="utf-8"))


@dataclass(frozen=True)
class DecisionRecord:
    """
    The append-only audit record of one research decision.

    Every field the design's ledger lists is present, and no stage may write a field another stage
    owns. The identifiers and the five exposure quantities are absent (`None`) until the stage that
    owns them runs.

    Parameters
    ----------
    decision_id : str
        The identity of the decision. It is the row identity of the ledger.
    artifact_sha256 : str
        The digest of the raw artifact.
    idempotency_key : str
        The artifact's own key, distinguishing a duplicate from a revision.
    instrument_id : str, optional
        The instrument the decision names. Absent when the instrument could not be resolved.
    produced_at : int, optional
        The instant the artifact existed, in nanoseconds since the UNIX epoch.
    received_at : int, optional
        The instant the bridge received the artifact, in nanoseconds since the UNIX epoch.
    effective_date : str, optional
        The reference date of the analysis.
    expires_at : int, optional
        The exclusive instant the artifact ceases to be actionable.
    actionable_at : int, optional
        The instant the artifact becomes actionable, resolved from the instrument's calendar.
    admission_timestamp : int, optional
        The instant the admission stage evaluated the artifact.
    admission_result : str, optional
        One of `ADMISSION_RESULTS`.
    refusal_reason : RefusalCode, optional
        The reason the artifact stopped, drawn from the closed vocabulary.
    diagnostics : tuple[Diagnostic, ...]
        Diagnostics recorded beside a record that continued.
    projected_signal_id : str, optional
        The identity of the projected signal, written by the projection stage.
    target_id : str, optional
        The identity of the constructed target, written by the construction stage.
    order_ids : tuple[str, ...]
        The identities of the orders the decision produced.
    fill_ids : tuple[str, ...]
        The identities of the fills the decision produced.
    attempts : tuple[str, ...]
        The caller-supplied labels of the deliberate retry attempts this decision has made, in the
        order they were made. Empty for a decision that was never retried: a repeated delivery of
        the same artifact is a `DuplicateArrival` and is recorded separately, never here, so an
        attempt and a duplicate arrival are distinguishable from the record alone.
    research_requested : float, optional
        The allocation the artifact requested.
    bridge_capped : float, optional
        The allocation after the bridge's ceiling.
    engine_constructed : float, optional
        The allocation the engine constructed.
    risk_approved : float, optional
        The allocation the risk engine approved.
    actually_filled : float, optional
        The allocation that was actually filled.

    """

    decision_id: str
    artifact_sha256: str
    idempotency_key: str
    instrument_id: str | None = None
    produced_at: int | None = None
    received_at: int | None = None
    effective_date: str | None = None
    expires_at: int | None = None
    actionable_at: int | None = None
    admission_timestamp: int | None = None
    admission_result: str | None = None
    refusal_reason: RefusalCode | None = None
    diagnostics: tuple[Diagnostic, ...] = ()
    projected_signal_id: str | None = None
    target_id: str | None = None
    order_ids: tuple[str, ...] = ()
    fill_ids: tuple[str, ...] = ()
    attempts: tuple[str, ...] = ()
    research_requested: float | None = None
    bridge_capped: float | None = None
    engine_constructed: float | None = None
    risk_approved: float | None = None
    actually_filled: float | None = None

    @property
    def not_reached(self) -> tuple[str, ...]:
        """
        Return the exposure stages the record never reached.

        A stage that did not run leaves its quantity absent, and this reports those as not reached
        rather than as a zero exposure (B15).
        """
        return tuple(name for name in EXPOSURE_STAGES if getattr(self, name) is None)


@dataclass(frozen=True)
class DuplicateArrival:
    """
    A repeated delivery of an artifact that was already admitted.

    A duplicate is an outcome rather than an error: it produces no second signal and no second
    order, and it is recorded here rather than rewriting the admitted record's answer.

    Parameters
    ----------
    decision_id : str
        The identity of the decision the artifact was already admitted under.
    artifact_sha256 : str
        The digest of the repeated artifact.
    idempotency_key : str
        The repeated artifact's own key.
    received_at : int, optional
        The instant the repeated delivery was received.
    admission_timestamp : int, optional
        The instant the repeated delivery was evaluated.
    reason : RefusalCode
        Always `RefusalCode.DUPLICATE`, recorded so the arrival is classifiable without prose.

    """

    decision_id: str
    artifact_sha256: str
    idempotency_key: str
    received_at: int | None = None
    admission_timestamp: int | None = None
    reason: RefusalCode = RefusalCode.DUPLICATE


def _record_to_json(record: DecisionRecord) -> dict:
    """
    Encode a record as a JSON document.
    """
    return {
        "decision_id": record.decision_id,
        "artifact_sha256": record.artifact_sha256,
        "idempotency_key": record.idempotency_key,
        "instrument_id": record.instrument_id,
        "produced_at": record.produced_at,
        "received_at": record.received_at,
        "effective_date": record.effective_date,
        "expires_at": record.expires_at,
        "actionable_at": record.actionable_at,
        "admission_timestamp": record.admission_timestamp,
        "admission_result": record.admission_result,
        "refusal_reason": None if record.refusal_reason is None else record.refusal_reason.value,
        "diagnostics": [diagnostic.value for diagnostic in record.diagnostics],
        "projected_signal_id": record.projected_signal_id,
        "target_id": record.target_id,
        "order_ids": list(record.order_ids),
        "fill_ids": list(record.fill_ids),
        "attempts": list(record.attempts),
        "research_requested": record.research_requested,
        "bridge_capped": record.bridge_capped,
        "engine_constructed": record.engine_constructed,
        "risk_approved": record.risk_approved,
        "actually_filled": record.actually_filled,
    }


def _record_from_json(doc: dict) -> DecisionRecord:
    """
    Decode a record from a JSON document.
    """
    refusal_reason = doc.get("refusal_reason")
    return DecisionRecord(
        decision_id=doc["decision_id"],
        artifact_sha256=doc["artifact_sha256"],
        idempotency_key=doc["idempotency_key"],
        instrument_id=doc.get("instrument_id"),
        produced_at=doc.get("produced_at"),
        received_at=doc.get("received_at"),
        effective_date=doc.get("effective_date"),
        expires_at=doc.get("expires_at"),
        actionable_at=doc.get("actionable_at"),
        admission_timestamp=doc.get("admission_timestamp"),
        admission_result=doc.get("admission_result"),
        refusal_reason=None if refusal_reason is None else RefusalCode(refusal_reason),
        diagnostics=tuple(Diagnostic(value) for value in doc.get("diagnostics", ())),
        projected_signal_id=doc.get("projected_signal_id"),
        target_id=doc.get("target_id"),
        order_ids=tuple(doc.get("order_ids", ())),
        fill_ids=tuple(doc.get("fill_ids", ())),
        attempts=tuple(doc.get("attempts", ())),
        research_requested=doc.get("research_requested"),
        bridge_capped=doc.get("bridge_capped"),
        engine_constructed=doc.get("engine_constructed"),
        risk_approved=doc.get("risk_approved"),
        actually_filled=doc.get("actually_filled"),
    )


def _duplicate_to_json(arrival: DuplicateArrival) -> dict:
    """
    Encode a duplicate arrival as a JSON document.
    """
    return {
        "decision_id": arrival.decision_id,
        "artifact_sha256": arrival.artifact_sha256,
        "idempotency_key": arrival.idempotency_key,
        "received_at": arrival.received_at,
        "admission_timestamp": arrival.admission_timestamp,
        "reason": arrival.reason.value,
    }


def _duplicate_from_json(doc: dict) -> DuplicateArrival:
    """
    Decode a duplicate arrival from a JSON document.
    """
    return DuplicateArrival(
        decision_id=doc["decision_id"],
        artifact_sha256=doc["artifact_sha256"],
        idempotency_key=doc["idempotency_key"],
        received_at=doc.get("received_at"),
        admission_timestamp=doc.get("admission_timestamp"),
        reason=RefusalCode(doc.get("reason", RefusalCode.DUPLICATE.value)),
    )


class AdmissionLedger:
    """
    An append-only, JSON-persisted ledger of research decisions.

    Parameters
    ----------
    directory : str or pathlib.Path
        The directory the ledger owns. It is created if it does not exist.

    """

    def __init__(self, directory: str | Path) -> None:
        """
        Initialize the instance and load every stored record.
        """
        self._directory = Path(directory)
        (self._directory / _DECISIONS_DIR).mkdir(parents=True, exist_ok=True)
        self._records: dict[str, DecisionRecord] = {}
        self._duplicates: dict[str, list[DuplicateArrival]] = {}
        self._order: list[str] = []
        self._load()

    @property
    def directory(self) -> Path:
        """
        The directory the ledger owns.
        """
        return self._directory

    def admit(  # noqa: PLR0913 - the stage order is the contract
        self,
        decision_id: str,
        *,
        artifact_sha256: str,
        idempotency_key: str,
        instrument_id: str | None = None,
        produced_at: int | None = None,
        received_at: int | None = None,
        effective_date: str | None = None,
        expires_at: int | None = None,
        actionable_at: int | None = None,
        admission_timestamp: int | None = None,
        admission_result: str = ADMITTED,
        refusal_reason: RefusalCode | None = None,
        diagnostics: Iterable[Diagnostic] = (),
    ) -> str:
        """
        Record the arrival of one artifact and return the admission result.

        A repeated delivery of an artifact already in the ledger is a no-op: it is recorded as a
        `DuplicateArrival` and this method returns `DUPLICATE` without touching the admitted
        record.

        Parameters
        ----------
        decision_id : str
            The identity of the decision.
        artifact_sha256 : str
            The digest of the raw artifact.
        idempotency_key : str
            The artifact's own key.
        instrument_id : str, optional
            The instrument the decision names.
        produced_at : int, optional
            The instant the artifact existed, in nanoseconds since the UNIX epoch.
        received_at : int, optional
            The instant the bridge received the artifact, in nanoseconds since the UNIX epoch.
        effective_date : str, optional
            The reference date of the analysis.
        expires_at : int, optional
            The exclusive instant the artifact ceases to be actionable.
        actionable_at : int, optional
            The resolved instant the artifact becomes actionable.
        admission_timestamp : int, optional
            The instant admission evaluated the artifact.
        admission_result : str, default 'ADMITTED'
            One of `ADMISSION_RESULTS`.
        refusal_reason : RefusalCode, optional
            The reason the artifact stopped, when it did not continue.
        diagnostics : Iterable[Diagnostic], default ()
            Diagnostics recorded beside a record that continued.

        Returns
        -------
        str
            The admission result recorded, which is `DUPLICATE` for a repeated delivery.

        Raises
        ------
        ValueError
            If `admission_result` is not in `ADMISSION_RESULTS`, or if `decision_id` already
            carries a different artifact.

        """
        if admission_result not in ADMISSION_RESULTS:
            raise ValueError(f"unknown admission result {admission_result!r}")

        existing = self._find_by_artifact(artifact_sha256, idempotency_key)
        if existing is not None:
            arrival = DuplicateArrival(
                decision_id=existing.decision_id,
                artifact_sha256=artifact_sha256,
                idempotency_key=idempotency_key,
                received_at=received_at,
                admission_timestamp=admission_timestamp,
            )
            self._duplicates.setdefault(existing.decision_id, []).append(arrival)
            self._persist(existing.decision_id)
            return DUPLICATE

        if decision_id in self._records:
            raise ValueError(
                f"decision_id {decision_id!r} is already recorded with a different artifact",
            )

        record = DecisionRecord(
            decision_id=decision_id,
            artifact_sha256=artifact_sha256,
            idempotency_key=idempotency_key,
            instrument_id=instrument_id,
            produced_at=produced_at,
            received_at=received_at,
            effective_date=effective_date,
            expires_at=expires_at,
            actionable_at=actionable_at,
            admission_timestamp=admission_timestamp,
            admission_result=admission_result,
            refusal_reason=refusal_reason,
            diagnostics=tuple(diagnostics),
        )
        self._records[decision_id] = record
        self._order.append(decision_id)
        self._persist(decision_id)
        return admission_result

    def record_signal(self, decision_id: str, signal_id: str) -> None:
        """
        Write the projected signal identity on a decision record.

        Parameters
        ----------
        decision_id : str
            The identity of the decision.
        signal_id : str
            The identity of the projected signal.

        Raises
        ------
        KeyError
            If the decision is not in the ledger.
        ValueError
            If the field was already written.

        """
        self._write_once(self._require(decision_id), "projected_signal_id", signal_id)

    def record_target(self, decision_id: str, target_id: str) -> None:
        """
        Write the constructed target identity on a decision record.

        Parameters
        ----------
        decision_id : str
            The identity of the decision.
        target_id : str
            The identity of the constructed target.

        Raises
        ------
        KeyError
            If the decision is not in the ledger.
        ValueError
            If the field was already written.

        """
        self._write_once(self._require(decision_id), "target_id", target_id)

    def record_order(self, decision_id: str, order_id: str) -> None:
        """
        Append an order identity to a decision record.

        Parameters
        ----------
        decision_id : str
            The identity of the decision.
        order_id : str
            The identity of the order the decision produced.

        Raises
        ------
        KeyError
            If the decision is not in the ledger.

        """
        record = self._require(decision_id)
        if order_id in record.order_ids:
            return
        self._store(replace(record, order_ids=(*record.order_ids, order_id)))

    def record_fill(self, decision_id: str, fill_id: str) -> None:
        """
        Append a fill identity to a decision record.

        Parameters
        ----------
        decision_id : str
            The identity of the decision.
        fill_id : str
            The identity of the fill the decision produced.

        Raises
        ------
        KeyError
            If the decision is not in the ledger.

        """
        record = self._require(decision_id)
        if fill_id in record.fill_ids:
            return
        self._store(replace(record, fill_ids=(*record.fill_ids, fill_id)))

    def record_attempt(self, decision_id: str, attempt: str) -> None:
        """
        Append a caller-supplied attempt label to a decision record.

        An attempt is a deliberate retry of a decision the engine refused, and it is recorded here
        so it is never confused with a duplicate arrival, which is a repeated delivery of the same
        artifact and is kept separately. A label already recorded is a no-op.

        Parameters
        ----------
        decision_id : str
            The identity of the decision.
        attempt : str
            The caller-supplied label of the retry attempt.

        Raises
        ------
        KeyError
            If the decision is not in the ledger.

        """
        record = self._require(decision_id)
        if attempt in record.attempts:
            return
        self._store(replace(record, attempts=(*record.attempts, attempt)))

    def attempt_count(self, decision_id: str) -> int:
        """
        Return the number of deliberate retry attempts recorded for a decision.

        Parameters
        ----------
        decision_id : str
            The identity of the decision.

        Returns
        -------
        int
            The count a retry permission is computed against.

        Raises
        ------
        KeyError
            If the decision is not in the ledger.

        """
        return len(self._require(decision_id).attempts)

    def record_exposure(self, decision_id: str, stage: str, quantity: float) -> None:
        """
        Write one exposure quantity on a decision record.

        Parameters
        ----------
        decision_id : str
            The identity of the decision.
        stage : str
            The exposure stage, one of `EXPOSURE_STAGES`.
        quantity : float
            The exposure the stage holds, in the artifact's allocation units.

        Raises
        ------
        KeyError
            If the decision is not in the ledger.
        ValueError
            If the stage is unknown or its quantity was already written.

        """
        if stage not in EXPOSURE_STAGES:
            raise ValueError(f"unknown exposure stage {stage!r}")

        record = self._require(decision_id)
        if getattr(record, stage) is not None:
            raise ValueError(f"exposure stage {stage!r} is already written")
        self._store(replace(record, **{stage: float(quantity)}))

    def record_risk_answer(self, decision_id: str, quantity: float) -> None:
        """
        Write the risk stage's answer, replacing an earlier answer of its own.

        Every other stage fills its field once and never rewrites it, so this method exists only
        for the risk stage and only because a decision may make several submission attempts. A
        retryable engine denial is recorded as a zero approved exposure, and a retry that then
        passes revises that zero to the exposure the risk engine approved; without the revision
        the record would claim the risk engine approved nothing while a fill exists beside it.

        Parameters
        ----------
        decision_id : str
            The identity of the decision.
        quantity : float
            The exposure the risk engine's answer holds, in the artifact's allocation units.

        Raises
        ------
        KeyError
            If the decision is not in the ledger.

        """
        record = self._require(decision_id)
        self._store(replace(record, risk_approved=float(quantity)))

    def record(self, decision_id: str) -> DecisionRecord | None:
        """
        Return the record for a decision, or `None` if it is not in the ledger.

        Parameters
        ----------
        decision_id : str
            The identity of the decision.

        Returns
        -------
        DecisionRecord or None

        """
        return self._records.get(decision_id)

    def records(self) -> tuple[DecisionRecord, ...]:
        """
        Return every decision record, in the order it was admitted.

        Returns
        -------
        tuple[DecisionRecord, ...]

        """
        return tuple(self._records[decision_id] for decision_id in self._order)

    def duplicates(self, decision_id: str) -> tuple[DuplicateArrival, ...]:
        """
        Return every duplicate arrival recorded for a decision.

        Parameters
        ----------
        decision_id : str
            The identity of the decision.

        Returns
        -------
        tuple[DuplicateArrival, ...]

        """
        return tuple(self._duplicates.get(decision_id, ()))

    def decision_for_order(self, order_id: str) -> str:
        """
        Return the decision that produced an order, or `UNATTRIBUTED`.

        An order that carries no decision identity is unattributed rather than assigned to the
        nearest decision (B10).

        Parameters
        ----------
        order_id : str
            The identity of the order.

        Returns
        -------
        str

        """
        for record in self._records.values():
            if order_id in record.order_ids:
                return record.decision_id
        return UNATTRIBUTED

    def decision_for_fill(self, fill_id: str) -> str:
        """
        Return the decision that produced a fill, or `UNATTRIBUTED`.

        Parameters
        ----------
        fill_id : str
            The identity of the fill.

        Returns
        -------
        str

        """
        for record in self._records.values():
            if fill_id in record.fill_ids:
                return record.decision_id
        return UNATTRIBUTED

    def revision_chain(self, decision_id: str) -> tuple[DecisionRecord, ...]:
        """
        Return the revision chain a decision belongs to.

        Two admitted artifacts for the same instrument and reference date are one chain, ordered by
        production time, so the later artifact is the later revision and the two are never
        confused.

        Parameters
        ----------
        decision_id : str
            The identity of the decision.

        Returns
        -------
        tuple[DecisionRecord, ...]

        Raises
        ------
        KeyError
            If the decision is not in the ledger.

        """
        record = self._require(decision_id)
        if record.instrument_id is None or record.effective_date is None:
            return (record,)
        chain = [
            candidate
            for candidate in self._records.values()
            if candidate.admission_result == ADMITTED
            and candidate.instrument_id == record.instrument_id
            and candidate.effective_date == record.effective_date
        ]
        chain.sort(
            key=lambda candidate: (candidate.produced_at is None, candidate.produced_at or 0),
        )
        return tuple(chain)

    def _require(self, decision_id: str) -> DecisionRecord:
        """
        Return a stored record or raise.
        """
        record = self._records.get(decision_id)
        if record is None:
            raise KeyError(f"no ledger record for decision_id {decision_id!r}")
        return record

    def _find_by_artifact(
        self,
        artifact_sha256: str,
        idempotency_key: str,
    ) -> DecisionRecord | None:
        """
        Return the record already carrying an artifact identity, if any.
        """
        for record in self._records.values():
            if (
                record.artifact_sha256 == artifact_sha256
                and record.idempotency_key == idempotency_key
            ):
                return record
        return None

    def _write_once(self, record: DecisionRecord, name: str, value: object) -> None:
        """
        Write a field that no stage may write twice.
        """
        if getattr(record, name) is not None:
            raise ValueError(f"{name} is already written for decision_id {record.decision_id!r}")
        self._store(replace(record, **{name: value}))

    def _store(self, record: DecisionRecord) -> None:
        """
        Replace a record with its own updated state and persist it.
        """
        self._records[record.decision_id] = record
        self._persist(record.decision_id)

    def _persist(self, decision_id: str) -> None:
        """
        Write the decision's document and the manifest.
        """
        record = self._records[decision_id]
        document = {
            "schema": SCHEMA,
            "record": _record_to_json(record),
            "duplicates": [
                _duplicate_to_json(arrival)
                for arrival in self._duplicates.get(decision_id, ())
            ],
        }
        _write_json(self._directory / _DECISIONS_DIR / f"{_key(decision_id)}.json", document)
        _write_json(
            self._directory / MANIFEST_NAME,
            {"schema": SCHEMA, "decisions": self._order},
        )

    def _load(self) -> None:
        """
        Load every stored record and duplicate arrival, without replaying the stream.
        """
        decisions_dir = self._directory / _DECISIONS_DIR
        loaded: dict[str, tuple[DecisionRecord, list[DuplicateArrival]]] = {}
        for path in sorted(decisions_dir.glob("*.json")):
            document = _read_json(path)
            record = _record_from_json(document["record"])
            arrivals = [_duplicate_from_json(item) for item in document.get("duplicates", ())]
            loaded[record.decision_id] = (record, arrivals)

        ordered: list[str] = []
        manifest_path = self._directory / MANIFEST_NAME
        if manifest_path.exists():
            manifest = _read_json(manifest_path)
            for decision_id in manifest.get("decisions", ()):
                if decision_id in loaded and decision_id not in ordered:
                    ordered.append(decision_id)
        for decision_id in loaded:
            if decision_id not in ordered:
                ordered.append(decision_id)

        self._order = ordered
        self._records = {decision_id: loaded[decision_id][0] for decision_id in ordered}
        self._duplicates = {decision_id: loaded[decision_id][1] for decision_id in ordered}
