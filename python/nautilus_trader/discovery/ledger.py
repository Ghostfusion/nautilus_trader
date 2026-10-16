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
The discovery ledger: what was compiled, what ran, and what was observed.

The ledger is digest-keyed files rather than a database, which is what lets a reader reconstruct a
campaign from the files alone, and it is append-only, so a candidate's record cannot be rewritten
once something has been learned from it. One entry is written per experiment and keyed by the
candidate's behavioural identity, which is the identity of the artifact that ran rather than of the
name it was submitted under.

Two artifacts live here and they are read together. The *entries* are the evidence packages a
campaign's experiments produced. The *inventory* is the hash-chained, append-only event log the
governance protocol writes: every proposal, refusal, compilation, job, closed attempt, verified
registration, admitted trial, gate verdict, custody event and search-policy record, in one order,
under one writer. The inventory owns the four counts of design Section 10.5 and the lineage of
design Section 8.4, and it is where the registration protocol of that section is implemented: one
authoritative writer, a stable idempotency key per job, registration as an explicit state
transition, nothing counted before its artifact exists and its run identity reconciles, an
interrupted registration replayable to one trial, a duplicate registration idempotent rather than
additive, and an incomplete record explicitly incomplete rather than absent or complete.

Reconciliation is the check that earns the layer its place, and it fails closed. It compares the
receipt a run emitted against the manifest it claims and against the research artifact that cites
it, and a disagreement in the seed, the build identity, the configuration digest or the artifact
digest leaves the registration unverified rather than counted. Two limitations are named rather
than worked around: the engine's own run manifest has no Python binding, so the manifest a receipt
is checked against is the one its caller supplies until the narrow upstream change of decision
M1.3 exists, and the engine's canonical digest is computed by the extension from the document it
holds, so a retained document's blake3 digest cannot be recomputed in Python; the ledger records
both the digest and the document's SHA-256, and the recomputation check verifies the latter while
naming the former as unavailable.
"""

from __future__ import annotations

import hashlib
import json
import math
import threading
from dataclasses import dataclass
from dataclasses import field
from enum import Enum
from pathlib import Path
from typing import TYPE_CHECKING

from nautilus_trader.discovery.schema import canonical_mapping
from nautilus_trader.optimization.significance import TrialDependence
from nautilus_trader.optimization.space import digest_of


if TYPE_CHECKING:
    from collections.abc import Iterable
    from collections.abc import Mapping
    from collections.abc import Sequence

    from nautilus_trader.discovery.compile import CompiledPlan
    from nautilus_trader.optimization.persistence import ExperimentStore
    from nautilus_trader.optimization.runner import CanonicalRun


# The identity of the ledger format, which a reader uses to know how to read an entry.
LEDGER_SCHEMA = "discovery.ledger/v2"

# The identity of the campaign inventory format: one JSON object per line, hash-chained.
INVENTORY_SCHEMA = "discovery.inventory/v1"

# The versioned identity of the event-to-count mapping. A change to any rule that maps an event to
# a count changes this value, and therefore the identity of every result that recorded it.
COUNTS_MAPPING = "discovery.counts/v1"

# The identity of the reproducibility envelope format.
ENVELOPE_SCHEMA = "discovery.envelope/v1"

# The identity of the acceptance-evidence bundle format.
BUNDLE_SCHEMA = "discovery.acceptance-bundle/v2"

_ENTRIES_DIR = "entries"
_EVENTS_FILE = "inventory.jsonl"
_WRITER_FILE = "inventory.writer"

# The components a rerun does not reproduce from the same specification, snapshot, configuration,
# seed and binary. The list travels with every recomputation, because a determinism claim that
# does not name what it excludes is not checkable.
NON_DETERMINISTIC_COMPONENTS: tuple[tuple[str, str], ...] = (
    (
        "model_invocation",
        "an agent's response is replayed from its recorded response rather than recalled",
    ),
    (
        "fill_model_stochasticity",
        "any stochasticity a declared fill model introduces, parameterised by the run seed",
    ),
    (
        "slippage_model_stochasticity",
        "any stochasticity a declared slippage model introduces, parameterised by the run seed",
    ),
)


def non_deterministic_digest() -> str:
    """
    Return the digest of the declared non-deterministic components.

    Returns
    -------
    str
        The `sha256:<hex>` digest.

    """
    return digest_of(
        canonical_mapping({"components": [list(item) for item in NON_DETERMINISTIC_COMPONENTS]})
    )


@dataclass(frozen=True)
class RunIdentity:
    """
    The identities a run names, gathered for an evidence package to carry.

    Parameters
    ----------
    specification_digest : str
        The digest of the submitted specification.
    behavioural_digest : str
        The digest of the compiled plan the run executed.
    policy_digest : str
        The digest of the pre-registered policy.
    registry_digest : str
        The digest of the feature registry.
    engine_version : str
        The engine version the run was produced with.
    snapshot : str
        The description of the data snapshot the run was bound to.
    seed : int
        The seed the run declared.

    """

    specification_digest: str
    behavioural_digest: str
    policy_digest: str
    registry_digest: str
    engine_version: str
    snapshot: str
    seed: int

    @classmethod
    def of(cls, plan: CompiledPlan, *, snapshot: str, seed: int) -> RunIdentity:
        """
        Return the identity of a compiled plan.

        Parameters
        ----------
        plan : CompiledPlan
            The compiled plan the run executes.
        snapshot : str
            The description of the data snapshot.
        seed : int
            The seed the run declares.

        Returns
        -------
        RunIdentity

        """
        return cls(
            specification_digest=plan.specification_digest,
            behavioural_digest=plan.behavioural_digest,
            policy_digest=plan.policy_digest,
            registry_digest=plan.registry_digest,
            engine_version=plan.engine_version,
            snapshot=snapshot,
            seed=seed,
        )


@dataclass(frozen=True)
class EvidencePackage:
    """
    What one experiment can say about itself, recomputable from the ledger and the snapshot.

    The package carries the two identities the design keeps apart: the specification identity, which
    is what was submitted, and the behavioural identity, which is what ran, including the resolved
    feature definitions, the policy and the declared execution assumptions. It also carries the
    engine's own canonical result digest and bytes, so a reader compares an artifact rather than a
    claim.

    Parameters
    ----------
    candidate_id : str
        The candidate's identifier in the ledger, which is its behavioural digest.
    specification_digest : str
        The digest of the submitted specification.
    behavioural_digest : str
        The digest of the compiled plan the run executed.
    policy_digest : str
        The digest of the pre-registered policy the candidate was compiled and run under.
    registry_digest : str
        The digest of the feature registry the candidate's features were resolved against.
    experiment_digest : str
        The digest of the experiment, which is the parameter set the run was identified by.
    canonical_digest : str
        The engine's canonical result digest.
    canonical_bytes : bytes
        The engine's canonical result bytes, which the digest is over.
    metric_values : Mapping[str, float]
        The metric values the run produced.
    engine_version : str
        The engine version the run was produced with.
    snapshot : str
        The description of the data snapshot the run was bound to.
    seed : int
        The seed the run declared.
    status : str
        The run's outcome, as `completed` or `failed`.

    """

    candidate_id: str
    specification_digest: str
    behavioural_digest: str
    policy_digest: str
    registry_digest: str
    experiment_digest: str
    canonical_digest: str
    canonical_bytes: bytes
    metric_values: Mapping[str, float]
    engine_version: str
    snapshot: str
    seed: int
    status: str

    @classmethod
    def from_run(
        cls,
        run: CanonicalRun,
        identity: RunIdentity,
        *,
        status: str = "completed",
    ) -> EvidencePackage:
        """
        Assemble the package for one canonical run.

        Parameters
        ----------
        run : CanonicalRun
            The run, as the runner recorded it.
        identity : RunIdentity
            The identities the run is named by.
        status : str, default "completed"
            The run's outcome.

        Returns
        -------
        EvidencePackage

        """
        return cls(
            candidate_id=identity.behavioural_digest,
            specification_digest=identity.specification_digest,
            behavioural_digest=identity.behavioural_digest,
            policy_digest=identity.policy_digest,
            registry_digest=identity.registry_digest,
            experiment_digest=run.experiment.digest,
            canonical_digest=run.canonical_digest,
            canonical_bytes=run.canonical_result,
            metric_values=dict(run.metric_values),
            engine_version=identity.engine_version,
            snapshot=identity.snapshot,
            seed=identity.seed,
            status=status,
        )

    def to_dict(self) -> dict[str, object]:
        """
        Return the package as plain data.

        A metric that is not a finite number is written as its own text, which is the convention the
        experiment store already uses: a metric undefined for a run is a real outcome, and a record
        that cannot be serialised would lose the run rather than the value.

        Returns
        -------
        dict[str, object]

        """
        return {
            "candidate_id": self.candidate_id,
            "specification_digest": self.specification_digest,
            "behavioural_digest": self.behavioural_digest,
            "policy_digest": self.policy_digest,
            "registry_digest": self.registry_digest,
            "experiment_digest": self.experiment_digest,
            "canonical_digest": self.canonical_digest,
            "canonical_bytes_sha256": digest_of({"bytes": self.canonical_bytes.hex()}),
            "metric_values": {
                name: float(value) if math.isfinite(value) else str(value)
                for name, value in self.metric_values.items()
            },
            "engine_version": self.engine_version,
            "snapshot": self.snapshot,
            "seed": self.seed,
            "status": self.status,
        }

    def digest(self) -> str:
        """
        Return the digest of the package, which is what the ledger keys an entry by.

        Returns
        -------
        str
            The `sha256:<hex>` digest.

        """
        return digest_of(canonical_mapping(self.to_dict()))


class DiscoveryLedger:
    """
    An append-only record of the experiments a campaign has run.

    Parameters
    ----------
    directory : str | Path
        The directory the entries are written into, which is created when it does not exist.

    """

    def __init__(self, directory: str | Path) -> None:
        """
        Initialize the ledger over the given directory.
        """
        self._directory = Path(directory)
        self._entries_dir = self._directory / _ENTRIES_DIR

    @property
    def directory(self) -> Path:
        """
        The directory the entries are written into.
        """
        return self._directory

    def append(self, package: EvidencePackage) -> Path:
        """
        Append one experiment's evidence package.

        Parameters
        ----------
        package : EvidencePackage
            The package to append.

        Returns
        -------
        Path
            The path the entry was written to.

        Raises
        ------
        FileExistsError
            If an entry for the same candidate identity already exists, because an append-only
            ledger does not rewrite what was already learned.

        """
        self._entries_dir.mkdir(parents=True, exist_ok=True)
        path = self._entries_dir / f"{_key(package.candidate_id)}.json"

        if path.exists():
            raise FileExistsError(
                f"an entry for candidate {package.candidate_id} already exists at {path}",
            )

        payload = {"schema": LEDGER_SCHEMA, "entry": package.to_dict()}
        path.write_text(f"{json.dumps(payload, indent=2, sort_keys=True)}\n", encoding="utf-8")

        return path

    def entries(self) -> Sequence[Mapping[str, object]]:
        """
        Return the entries in the order their files sort.

        Returns
        -------
        Sequence[Mapping[str, object]]

        """
        if not self._entries_dir.exists():
            return ()

        read: list[Mapping[str, object]] = []
        for path in sorted(self._entries_dir.glob("*.json")):
            payload = json.loads(path.read_text(encoding="utf-8"))
            entry = payload.get("entry") if isinstance(payload, dict) else None
            if isinstance(entry, dict):
                read.append(entry)

        return tuple(read)

    def candidate_ids(self) -> tuple[str, ...]:
        """
        Return the identities the ledger holds an entry for.

        Returns
        -------
        tuple[str, ...]

        """
        return tuple(str(entry.get("candidate_id")) for entry in self.entries())

    def __len__(self) -> int:
        """
        Return the number of entries.
        """
        return len(self.entries())


def _key(digest: str) -> str:
    """
    Return the file name a digest is keyed by.

    Parameters
    ----------
    digest : str
        The `sha256:<hex>` digest.

    Returns
    -------
    str

    """
    return digest.replace(":", "_")


class EventKind(Enum):
    """
    The closed set of inventory events.

    Only the events in this vocabulary change a count, and each one changes exactly the count the
    mapping names. An event outside the set cannot be written, so a caller cannot invent an event
    that a later reader would have to interpret.
    """

    PROPOSAL_RECORDED = "proposal_recorded"
    SPECIFICATION_REFUSED = "specification_refused"
    COMPILATION_RECORDED = "compilation_recorded"
    EQUIVALENCE_MERGE = "equivalence_merge"
    JOB_SUBMITTED = "job_submitted"
    EXECUTION_ATTEMPT_CLOSED = "execution_attempt_closed"
    REGISTRATION_VERIFIED = "registration_verified"
    TRIAL_ADMITTED = "trial_admitted"
    REGISTRATION_INCOMPLETE = "registration_incomplete"
    DUPLICATE_REGISTRATION = "duplicate_registration"
    GATE_VERDICT = "gate_verdict"
    CUSTODY_EVENT = "custody_event"
    SEARCH_POLICY_RECORDED = "search_policy_recorded"


class CountName(Enum):
    """
    The four quantities the ledger maintains separately.
    """

    PROPOSAL_ATTEMPT = "proposal_attempt_count"
    UNIQUE_CONFIGURATION = "unique_configuration_count"
    EXECUTED_EXPERIMENT = "executed_experiment_count"
    STATISTICAL_TRIAL = "statistical_trial_count"


class DataRole(Enum):
    """
    The role the data behind a result plays, from design Section 8.6.

    The rule that makes the classification meaningful is about feedback rather than about access
    lists: a result whose value influenced the next mutation is development data, whatever split it
    came from. A role is therefore recorded on a job together with the lineage that consumed it,
    because the same partition stays locked for a lineage that has never seen it and is development
    data for one that has.
    """

    DEVELOPMENT = "development"
    LOCKED_VALIDATION = "locked_validation"
    FINAL_CONFIRMATION = "final_confirmation"

    @property
    def admissible_as_validation(self) -> bool:
        """
        Whether a result under this role may be read as validation evidence.
        """
        return self is DataRole.LOCKED_VALIDATION


class AttemptOutcome(Enum):
    """
    How an execution attempt closed, which is the set the plan's mapping table names.
    """

    COMPLETED = "completed"
    ECONOMICALLY_FAILED = "economically_failed"
    ERRORED = "errored"
    TIMED_OUT = "timed_out"
    CRASHED = "crashed"

    @property
    def produced_a_return_series(self) -> bool:
        """
        Whether the attempt produced a usable return series, which is what admission requires.

        An attempt that errored, timed out or crashed produced no usable series, so it stays in the
        execution count and is never admitted to the statistical inventory. Giving it an imputed
        return would be worse than recording the failure: it is a refusal that looks like a result.
        """
        return self in (AttemptOutcome.COMPLETED, AttemptOutcome.ECONOMICALLY_FAILED)


class RegistrationState(Enum):
    """
    The registration states of design Section 8.4, plus the incomplete state it requires by name.

    Pending, running, completed, failed and registration-reconciled are the states the design
    enumerates. `INCOMPLETE` is the state a registration that failed part-way leaves, which the
    design requires to be explicit rather than absent or complete, and it is retryable rather than
    terminal because recovery repeats the registration step against the same idempotency key.
    """

    PENDING = "pending"
    RUNNING = "running"
    COMPLETED = "completed"
    FAILED = "failed"
    REGISTRATION_RECONCILED = "registration-reconciled"
    INCOMPLETE = "incomplete"


# The transitions the registration protocol permits, keyed by the state a transition starts in.
_PERMITTED_REGISTRATIONS: Mapping[RegistrationState, frozenset[RegistrationState]] = {
    RegistrationState.PENDING: frozenset(
        {RegistrationState.RUNNING, RegistrationState.FAILED, RegistrationState.INCOMPLETE},
    ),
    RegistrationState.RUNNING: frozenset(
        {
            RegistrationState.COMPLETED,
            RegistrationState.FAILED,
            RegistrationState.INCOMPLETE,
        },
    ),
    RegistrationState.COMPLETED: frozenset(
        {RegistrationState.REGISTRATION_RECONCILED, RegistrationState.INCOMPLETE},
    ),
    RegistrationState.FAILED: frozenset(
        {RegistrationState.REGISTRATION_RECONCILED, RegistrationState.INCOMPLETE},
    ),
    RegistrationState.INCOMPLETE: frozenset(
        {RegistrationState.REGISTRATION_RECONCILED, RegistrationState.INCOMPLETE},
    ),
    RegistrationState.REGISTRATION_RECONCILED: frozenset(
        {RegistrationState.REGISTRATION_RECONCILED},
    ),
}


@dataclass(frozen=True)
class Counts:
    """
    The four counts of design Section 10.5, reported separately and never conflated.

    Parameters
    ----------
    proposal_attempts : int
        Every proposal from an agent or an operator, compiled or not.
    unique_configurations : int
        Distinct configurations under the pre-registered equivalence rule.
    executed_experiments : int
        Every attempt that closed, whatever its outcome, duplicates included.
    statistical_trials : int
        Verified executions admitted to the campaign's selection process.
    mapping : str, default `COUNTS_MAPPING`
        The versioned identity of the mapping that produced the counts.

    """

    proposal_attempts: int = 0
    unique_configurations: int = 0
    executed_experiments: int = 0
    statistical_trials: int = 0
    mapping: str = COUNTS_MAPPING

    def __post_init__(self) -> None:
        """
        Refuse a count that cannot hold, and refuse a campaign whose trials exceed its executions.
        """
        for name in CountName:
            value = self.value_of(name)
            if value < 0:
                raise ValueError(f"{name.value} must not be negative, was {value}")
        if self.statistical_trials > self.executed_experiments:
            raise ValueError(
                f"statistical_trials {self.statistical_trials} exceeds executed_experiments "
                f"{self.executed_experiments}",
            )
        if not self.mapping:
            raise ValueError("mapping must not be empty")

    def value_of(self, name: CountName) -> int:
        """
        Return one count by its name.

        Parameters
        ----------
        name : CountName
            The count to read.

        Returns
        -------
        int

        """
        return {
            CountName.PROPOSAL_ATTEMPT: self.proposal_attempts,
            CountName.UNIQUE_CONFIGURATION: self.unique_configurations,
            CountName.EXECUTED_EXPERIMENT: self.executed_experiments,
            CountName.STATISTICAL_TRIAL: self.statistical_trials,
        }[name]

    def to_dict(self) -> dict[str, object]:
        """
        Return the counts as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "mapping": self.mapping,
            CountName.PROPOSAL_ATTEMPT.value: self.proposal_attempts,
            CountName.UNIQUE_CONFIGURATION.value: self.unique_configurations,
            CountName.EXECUTED_EXPERIMENT.value: self.executed_experiments,
            CountName.STATISTICAL_TRIAL.value: self.statistical_trials,
        }

    def digest(self) -> str:
        """
        Return the digest of the counts under their mapping.

        Returns
        -------
        str
            The `sha256:<hex>` digest.

        """
        return digest_of(canonical_mapping(self.to_dict()))


def counts_from_events(events: Iterable[LedgerEvent]) -> Counts:
    """
    Return the four counts of the given events under the versioned mapping.

    The fold is the only place a count is taken, so no method can define its own counting rule, and
    it is a fold rather than a counter so the counts are recomputable from the ledger file alone.
    A refusal is a proposal attempt and nothing else, a compilation is a configuration only when
    the equivalence rule reports the identity as new, an attempt that closed is an execution
    whatever its outcome, and a duplicate registration changes nothing. Trials are deduplicated by
    idempotency key as well as by event, so a replayed registration that somehow wrote two
    admissions would still count once.

    Parameters
    ----------
    events : Iterable[LedgerEvent]
        The events, in the order they were written.

    Returns
    -------
    Counts

    """
    proposals = 0
    configurations = 0
    executions = 0
    admitted: set[str] = set()

    for event in events:
        if event.kind is EventKind.PROPOSAL_RECORDED:
            proposals += 1
        elif event.kind is EventKind.COMPILATION_RECORDED:
            configurations += 1 if event.payload.get("new_configuration") else 0
        elif event.kind is EventKind.EXECUTION_ATTEMPT_CLOSED:
            executions += 1
        elif event.kind is EventKind.TRIAL_ADMITTED:
            admitted.add(event.key)

    return Counts(
        proposal_attempts=proposals,
        unique_configurations=configurations,
        executed_experiments=executions,
        statistical_trials=len(admitted),
    )


@dataclass(frozen=True)
class IdempotencyKey:
    """
    The key a job carries, derived from its declared inputs rather than from its result.

    Two submissions that declare the same candidate, snapshot, seed and attempt are one job, so a
    resubmission is recognised without comparing results. A deliberately repeated execution of one
    configuration declares a different attempt identifier and is therefore a distinct job.

    Parameters
    ----------
    value : str
        The digest of the declared inputs.

    """

    value: str

    def __post_init__(self) -> None:
        """
        Refuse an empty key, because an empty key makes every job one job.
        """
        if not self.value:
            raise ValueError("value must not be empty")

    @classmethod
    def of(
        cls,
        candidate_id: str,
        *,
        snapshot: str,
        seed: int,
        attempt: str = "",
    ) -> IdempotencyKey:
        """
        Return the key of a job's declared inputs.

        Parameters
        ----------
        candidate_id : str
            The candidate's behavioural identity.
        snapshot : str
            The description of the data snapshot the job reads.
        seed : int
            The seed the job declares.
        attempt : str, default ""
            A caller-declared attempt identifier, which is what makes a duplicate execution a
            distinct execution event rather than a duplicate registration.

        Returns
        -------
        IdempotencyKey

        """
        if not candidate_id:
            raise ValueError("candidate_id must not be empty")

        return cls(
            digest_of(
                canonical_mapping(
                    {
                        "candidate_id": candidate_id,
                        "snapshot": snapshot,
                        "seed": seed,
                        "attempt": attempt,
                    },
                ),
            ),
        )


@dataclass(frozen=True)
class LedgerEvent:
    """
    One record in the campaign inventory.

    The event's digest is recomputed from its own fields together with the digest of the event
    before it, which is what makes a deletion, an insertion or a reordering detectable against a
    checkpoint held outside the write path. It offers nothing against an actor who holds write
    access and recomputes the chain from the start, and the design does not claim otherwise.

    Parameters
    ----------
    sequence : int
        The event's position in the inventory, from zero.
    kind : EventKind
        The event's kind, from the closed vocabulary.
    key : str
        The idempotency key the event is about, or the empty string when it is not about a job.
    candidate_id : str
        The candidate the event is about, or the empty string when it is not about one.
    payload : Mapping[str, object]
        What the event records, in the fields its kind requires.
    previous : str
        The digest of the event before this one, or the empty string for the first event.

    """

    sequence: int
    kind: EventKind
    key: str
    candidate_id: str
    payload: Mapping[str, object]
    previous: str

    def __post_init__(self) -> None:
        """
        Validate the event's shape.
        """
        if self.sequence < 0:
            raise ValueError(f"sequence must not be negative, was {self.sequence}")
        if not isinstance(self.kind, EventKind):
            raise TypeError("kind must be an EventKind")

    def to_dict(self) -> dict[str, object]:
        """
        Return the event as plain data, which is the form its digest is taken over.

        Returns
        -------
        dict[str, object]

        """
        return {
            "sequence": self.sequence,
            "kind": self.kind.value,
            "key": self.key,
            "candidate_id": self.candidate_id,
            "payload": dict(self.payload),
            "previous": self.previous,
        }

    @classmethod
    def from_dict(cls, payload: Mapping[str, object]) -> LedgerEvent:
        """
        Return the event a written record holds.

        Parameters
        ----------
        payload : Mapping[str, object]
            The record, as it was read from the inventory file.

        Returns
        -------
        LedgerEvent

        Raises
        ------
        ValueError
            If the record does not carry the fields an event requires.

        """
        try:
            sequence = int(str(payload["sequence"]))
            kind = EventKind(str(payload["kind"]))
        except KeyError as exc:
            raise ValueError(f"an inventory record is missing {exc.args[0]}") from exc
        except ValueError as exc:
            raise ValueError(f"an inventory record names an unknown event: {exc}") from exc

        recorded = payload.get("payload")
        if not isinstance(recorded, dict):
            raise TypeError("an inventory record must carry a payload mapping")

        return cls(
            sequence=sequence,
            kind=kind,
            key=str(payload.get("key", "")),
            candidate_id=str(payload.get("candidate_id", "")),
            payload=recorded,
            previous=str(payload.get("previous", "")),
        )

    def digest(self) -> str:
        """
        Return the digest of the event and the digest it chains to.

        Returns
        -------
        str
            The `sha256:<hex>` digest.

        """
        return digest_of(canonical_mapping(self.to_dict()))


@dataclass(frozen=True)
class ChainVerification:
    """
    The outcome of verifying the inventory's chain.

    Parameters
    ----------
    events : int
        How many events were read.
    head : str
        The digest of the last event, which is what a checkpoint compares against.
    findings : tuple[str, ...]
        What disagrees with the chain, empty when nothing does.

    """

    events: int
    head: str
    findings: tuple[str, ...] = ()

    @property
    def ok(self) -> bool:
        """
        Whether the chain holds.
        """
        return not self.findings

    def require(self) -> None:
        """
        Raise unless the chain holds.

        Raises
        ------
        ValueError
            If the chain does not hold, naming what disagrees.

        """
        if self.findings:
            raise ValueError("the inventory chain does not hold: " + "; ".join(self.findings))


@dataclass(frozen=True)
class RunReceipt:
    """
    What a runner emits about one engine run, which the ledger references rather than copies.

    The receipt is the join between the engine's own run history and the research artifact that
    describes the same run. Reconciliation verifies it against the manifest it claims and against
    the artifact that cites it, and fails closed.

    Parameters
    ----------
    run_id : str
        The engine run's identity.
    manifest_digest : str
        The digest of the canonical run manifest the engine wrote.
    build_identity : str
        The binary or build identity the run was produced with.
    configuration_digest : str
        The digest of the run's resolved configuration.
    seed : int
        The seed the run declared.
    artifact_reference : str
        Where the research artifact lives, so a reader resolves content rather than a name.
    artifact_digest : str
        The digest of that artifact, which is the value the research side is checked against.

    """

    run_id: str
    manifest_digest: str
    build_identity: str
    configuration_digest: str
    seed: int
    artifact_reference: str
    artifact_digest: str

    def __post_init__(self) -> None:
        """
        Refuse a receipt that leaves a joined field unnamed.
        """
        for name in (
            "run_id",
            "manifest_digest",
            "build_identity",
            "configuration_digest",
            "artifact_reference",
            "artifact_digest",
        ):
            if not getattr(self, name):
                raise ValueError(f"{name} must not be empty")

    def to_dict(self) -> dict[str, object]:
        """
        Return the receipt as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "run_id": self.run_id,
            "manifest_digest": self.manifest_digest,
            "build_identity": self.build_identity,
            "configuration_digest": self.configuration_digest,
            "seed": self.seed,
            "artifact_reference": self.artifact_reference,
            "artifact_digest": self.artifact_digest,
        }

    def digest(self) -> str:
        """
        Return the digest of the receipt.

        Returns
        -------
        str
            The `sha256:<hex>` digest.

        """
        return digest_of(canonical_mapping(self.to_dict()))


@dataclass(frozen=True)
class Reconciliation:
    """
    Whether a receipt reconciles against the manifest and the artifact that cite it.

    Parameters
    ----------
    checked : tuple[str, ...]
        The fields that were compared.
    mismatched : tuple[str, ...]
        The fields that disagree, a field that is absent on either side included.
    detail : Mapping[str, str]
        What each mismatched field held on each side.

    """

    checked: tuple[str, ...]
    mismatched: tuple[str, ...] = ()
    detail: Mapping[str, str] = field(default_factory=dict)

    @property
    def ok(self) -> bool:
        """
        Whether every checked field agrees.
        """
        return not self.mismatched

    def describe(self) -> str:
        """
        Return what disagrees, in the vocabulary the coding standards require.

        Returns
        -------
        str

        """
        if self.ok:
            return "the receipt reconciles"

        rendered = (f"{name}: {self.detail.get(name, 'disagrees')}" for name in self.mismatched)

        return "; ".join(rendered)


def reconcile_receipt(
    receipt: RunReceipt,
    *,
    manifest: Mapping[str, object],
) -> Reconciliation:
    """
    Reconcile a receipt against the manifest it claims.

    A field that is absent on either side is a mismatch rather than a skip, because the check's
    purpose is to catch a research record citing a run whose binary, configuration or seed differs
    from the artifact that claims it, and a missing field is exactly the case where it cannot tell.

    Parameters
    ----------
    receipt : RunReceipt
        The receipt the run emitted.
    manifest : Mapping[str, object]
        The engine-side facts about the run, keyed by the receipt's field names.

    Returns
    -------
    Reconciliation

    """
    checked = ("run_id", "manifest_digest", "build_identity", "configuration_digest", "seed")
    mismatched: list[str] = []
    detail: dict[str, str] = {}

    for name in checked:
        recorded = getattr(receipt, name)
        declared = manifest.get(name)
        if declared is None:
            mismatched.append(name)
            detail[name] = f"the manifest does not declare it, the receipt holds {recorded!r}"
            continue
        if isinstance(recorded, int):
            agrees = str(declared) == str(recorded)
        else:
            agrees = declared == recorded
        if not agrees:
            mismatched.append(name)
            detail[name] = f"the receipt holds {recorded!r}, the manifest holds {declared!r}"

    return Reconciliation(checked=checked, mismatched=tuple(mismatched), detail=detail)


@dataclass(frozen=True)
class RegistrationOutcome:
    """
    What one registration attempt did.

    Parameters
    ----------
    key : str
        The idempotency key the attempt was made under.
    state : RegistrationState
        The state the registration is in afterwards.
    counted : bool
        Whether this call admitted a statistical trial. A replayed or duplicated registration
        admits nothing, which is what idempotent means here.
    idempotent : bool
        Whether the call found an existing reconciliation and changed no count.
    reason : str, default ""
        Why the registration is not reconciled, present exactly when it is not.
    reconciliation : Reconciliation | None, default None
        The field-level comparison, or None when no comparison was made.

    """

    key: str
    state: RegistrationState
    counted: bool
    idempotent: bool
    reason: str = ""
    reconciliation: Reconciliation | None = None

    @property
    def reconciled(self) -> bool:
        """
        Whether the registration verified and the artifact it cites reconciles.
        """
        return self.state is RegistrationState.REGISTRATION_RECONCILED

    @property
    def retryable(self) -> bool:
        """
        Whether a recovery may repeat the registration step under the same key.
        """
        return self.state in (RegistrationState.INCOMPLETE, RegistrationState.PENDING)

    def to_dict(self) -> dict[str, object]:
        """
        Return the outcome as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "key": self.key,
            "state": self.state.value,
            "counted": self.counted,
            "idempotent": self.idempotent,
            "reason": self.reason,
            "reconciliation": (
                None
                if self.reconciliation is None
                else {
                    "checked": list(self.reconciliation.checked),
                    "mismatched": list(self.reconciliation.mismatched),
                    "detail": dict(self.reconciliation.detail),
                }
            ),
        }


@dataclass(frozen=True)
class AttemptRecord:
    """
    One job's registration, folded from its events.

    Parameters
    ----------
    key : str
        The idempotency key.
    candidate_id : str
        The candidate the attempt is about.
    state : RegistrationState
        The state its events leave it in.
    outcome : AttemptOutcome | None
        How the attempt closed, or None when it has not closed.
    receipt_digest : str
        The digest of the reconciled receipt, or the empty string when none reconciled.
    admitted : bool
        Whether a statistical trial was admitted for it.
    data_role : DataRole
        The role the data behind the result plays.

    """

    key: str
    candidate_id: str
    state: RegistrationState
    outcome: AttemptOutcome | None = None
    receipt_digest: str = ""
    admitted: bool = False
    data_role: DataRole = DataRole.DEVELOPMENT
    trial_sharpes: tuple[float, ...] = ()

    def to_dict(self) -> dict[str, object]:
        """
        Return the attempt as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "key": self.key,
            "candidate_id": self.candidate_id,
            "state": self.state.value,
            "outcome": None if self.outcome is None else self.outcome.value,
            "receipt_digest": self.receipt_digest,
            "admitted": self.admitted,
            "data_role": self.data_role.value,
            "trial_sharpes": list(self.trial_sharpes),
        }


@dataclass(frozen=True)
class LineageRecord:
    """
    A candidate's place in the search that produced it.

    Parentage, campaign and research family are three different relations and all three are
    recorded: a child inherits nothing from its parent, a campaign keeps its own inventory, and a
    family is what a successor campaign inherits its spent trials through.

    Parameters
    ----------
    candidate_id : str
        The candidate.
    campaign_id : str
        The campaign that compiled it.
    family_id : str
        The research family the campaign belongs to.
    parent_id : str
        The candidate it was mutated from, or the empty string when it has none.
    generation : int
        The generation of the search it was proposed in.
    agent_invocation : str
        The recorded invocation that proposed it, or the empty string when an operator did.

    """

    candidate_id: str
    campaign_id: str
    family_id: str
    parent_id: str = ""
    generation: int = 0
    agent_invocation: str = ""

    def to_dict(self) -> dict[str, object]:
        """
        Return the record as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "candidate_id": self.candidate_id,
            "campaign_id": self.campaign_id,
            "family_id": self.family_id,
            "parent_id": self.parent_id,
            "generation": self.generation,
            "agent_invocation": self.agent_invocation,
        }

    def digest(self) -> str:
        """
        Return the digest of the lineage record.

        Returns
        -------
        str
            The `sha256:<hex>` digest.

        """
        return digest_of(canonical_mapping(self.to_dict()))


@dataclass(frozen=True)
class GateVerdictEntry:
    """
    One gate's verdict as the ledger recorded it.

    A verdict is evidence rather than multiplicity, and it is recorded with the reason the gate
    reached it, so a reader can reconstruct not only that a candidate was rejected but why. The
    evidence digest is the digest of the evidence the verdict was reached on, which binds the reason
    and the inputs it explains to the same record.

    Parameters
    ----------
    candidate_id : str
        The candidate the verdict is about.
    gate : str
        The gate's name, from the gate service's vocabulary.
    status : str
        The verdict, from the gate service's vocabulary.
    reason : str, default ""
        Why the gate reached it.
    evidence_digest : str, default ""
        The digest of the evidence the verdict was reached on.
    sequence : int, default 0
        The event's position in the chain.

    """

    candidate_id: str
    gate: str
    status: str
    reason: str = ""
    evidence_digest: str = ""
    sequence: int = 0

    def to_dict(self) -> dict[str, object]:
        """
        Return the verdict as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "candidate_id": self.candidate_id,
            "gate": self.gate,
            "status": self.status,
            "reason": self.reason,
            "evidence_digest": self.evidence_digest,
            "sequence": self.sequence,
        }


def research_family_id(
    *,
    snapshot: str,
    selection_process: str,
    mapping: str = COUNTS_MAPPING,
    search_policy_digest: str = "",
) -> str:
    """
    Return the identity of a research family.

    A family is one search over one set of observations. Two campaigns belong to one family when
    they read the same snapshot and select through the same process under the same counting
    mapping, which is the condition under which a successor inherits what its predecessor spent
    against that data rather than starting clean.

    Parameters
    ----------
    snapshot : str
        The description of the data snapshot both campaigns read.
    selection_process : str
        The declared selection process the trials enter.
    mapping : str, default `COUNTS_MAPPING`
        The counting mapping in force.
    search_policy_digest : str, default ""
        The digest of the search policy both campaigns ran under, or the empty string when the
        family spans more than one.

    Returns
    -------
    str
        The `sha256:<hex>` digest.

    """
    return digest_of(
        canonical_mapping(
            {
                "snapshot": snapshot,
                "selection_process": selection_process,
                "mapping": mapping,
                "search_policy_digest": search_policy_digest,
            },
        ),
    )


# What every event kind must record, so a reader never has to interpret a bare record.
_REQUIRED_FIELDS: Mapping[EventKind, tuple[str, ...]] = {
    EventKind.PROPOSAL_RECORDED: (),
    EventKind.SPECIFICATION_REFUSED: ("code",),
    EventKind.COMPILATION_RECORDED: ("specification_digest", "new_configuration"),
    EventKind.EQUIVALENCE_MERGE: ("merged_into", "rule"),
    EventKind.JOB_SUBMITTED: (),
    EventKind.EXECUTION_ATTEMPT_CLOSED: ("outcome",),
    EventKind.REGISTRATION_VERIFIED: ("receipt", "receipt_digest"),
    EventKind.TRIAL_ADMITTED: ("receipt_digest",),
    EventKind.REGISTRATION_INCOMPLETE: ("reason",),
    EventKind.DUPLICATE_REGISTRATION: ("reason",),
    EventKind.GATE_VERDICT: ("gate", "status", "evidence_digest"),
    EventKind.CUSTODY_EVENT: ("transition", "detail"),
    EventKind.SEARCH_POLICY_RECORDED: ("search_policy_digest",),
}


class CampaignInventory:
    """
    The append-only, hash-chained inventory of one campaign, with one authoritative writer.

    The inventory is the campaign's source of truth about what was proposed, compiled, executed,
    verified, admitted and judged. It is a file of one JSON object per line, written by one writer,
    and every count is a fold over it, so a reader recomputes the counts, the lineage and the
    registration states from the file rather than trusting a counter.

    Parameters
    ----------
    directory : str | Path
        The directory the inventory is written into, which is created when it does not exist.
    writer : str
        The identity of the writer that owns every append. A second writer on the same directory is
        refused rather than serialised, because the protocol's claims are about one writer.
    campaign_id : str, default ""
        The campaign's identity.
    family_id : str, default ""
        The research family the campaign belongs to, or the empty string when it declares none.
    policy_digest : str, default ""
        The digest of the pre-registered policy the campaign runs under.

    Raises
    ------
    ValueError
        If the writer identity is empty.
    RuntimeError
        If another writer already owns the directory.

    """

    def __init__(
        self,
        directory: str | Path,
        *,
        writer: str,
        campaign_id: str = "",
        family_id: str = "",
        policy_digest: str = "",
    ) -> None:
        """
        Initialize the inventory over the given directory and claim its writer slot.
        """
        if not writer:
            raise ValueError("writer must not be empty")

        self._directory = Path(directory)
        self._events_path = self._directory / _EVENTS_FILE
        self._writer_path = self._directory / _WRITER_FILE
        self._lock = threading.RLock()
        self._writer = writer
        self._campaign_id = campaign_id
        self._family_id = family_id
        self._policy_digest = policy_digest

        self._directory.mkdir(parents=True, exist_ok=True)
        self._claim_writer()

    @property
    def directory(self) -> Path:
        """
        The directory the inventory is written into.
        """
        return self._directory

    @property
    def writer(self) -> str:
        """
        The identity of the writer that owns every append.
        """
        return self._writer

    @property
    def campaign_id(self) -> str:
        """
        The campaign's identity.
        """
        return self._campaign_id

    @property
    def family_id(self) -> str:
        """
        The research family the campaign belongs to.
        """
        return self._family_id

    @property
    def policy_digest(self) -> str:
        """
        The digest of the pre-registered policy the campaign runs under.
        """
        return self._policy_digest

    def _claim_writer(self) -> None:
        """
        Claim the directory's writer slot, or refuse the claim another writer holds.
        """
        if self._writer_path.exists():
            holder = self._writer_path.read_text(encoding="utf-8").strip()
            if holder != self._writer:
                raise RuntimeError(
                    f"the inventory at {self._directory} is owned by the writer {holder!r}: "
                    f"one authoritative writer owns every append",
                )
        self._writer_path.write_text(f"{self._writer}\n", encoding="utf-8")

    def events(self) -> tuple[LedgerEvent, ...]:
        """
        Return the events, in the order they were written.

        Returns
        -------
        tuple[LedgerEvent, ...]

        Raises
        ------
        ValueError
            If a record cannot be read as an event.

        """
        if not self._events_path.exists():
            return ()

        read: list[LedgerEvent] = []
        for line in self._events_path.read_text(encoding="utf-8").splitlines():
            if not line.strip():
                continue
            payload = json.loads(line)
            if not isinstance(payload, dict):
                raise TypeError("an inventory record must be a mapping")
            read.append(LedgerEvent.from_dict(payload))

        return tuple(read)

    def counts(self) -> Counts:
        """
        Return the four counts of the events recorded so far.

        Returns
        -------
        Counts

        """
        return counts_from_events(self.events())

    def lineage(self) -> tuple[LineageRecord, ...]:
        """
        Return the lineage every compilation recorded.

        Returns
        -------
        tuple[LineageRecord, ...]

        """
        records: list[LineageRecord] = []
        seen: set[str] = set()
        invocations: dict[str, str] = {}

        for event in self.events():
            if event.kind is EventKind.PROPOSAL_RECORDED:
                invocations[event.candidate_id] = str(event.payload.get("agent_invocation", ""))

        for event in self.events():
            if event.kind is not EventKind.COMPILATION_RECORDED:
                continue
            if event.candidate_id in seen:
                continue
            seen.add(event.candidate_id)
            payload = event.payload
            records.append(
                LineageRecord(
                    candidate_id=event.candidate_id,
                    campaign_id=str(payload.get("campaign_id", self._campaign_id)),
                    family_id=str(payload.get("family_id", self._family_id)),
                    parent_id=str(payload.get("parent_id", "")),
                    generation=int(str(payload.get("generation", 0))),
                    agent_invocation=invocations.get(event.candidate_id, ""),
                ),
            )

        return tuple(records)

    def attempts(self) -> Mapping[str, AttemptRecord]:
        """
        Return every job's registration, folded from its events.

        Returns
        -------
        Mapping[str, AttemptRecord]

        """
        folded: dict[str, dict[str, object]] = {}

        for event in self.events():
            if not event.key:
                continue
            record = folded.setdefault(
                event.key,
                {
                    "key": event.key,
                    "candidate_id": event.candidate_id,
                    "state": RegistrationState.PENDING,
                    "outcome": None,
                    "receipt_digest": "",
                    "admitted": False,
                    "data_role": DataRole.DEVELOPMENT,
                },
            )
            if event.kind is EventKind.JOB_SUBMITTED:
                record["state"] = RegistrationState.RUNNING
                record["data_role"] = DataRole(str(event.payload.get("data_role", "development")))
            elif event.kind is EventKind.EXECUTION_ATTEMPT_CLOSED:
                outcome = AttemptOutcome(str(event.payload["outcome"]))
                record["outcome"] = outcome
                declared = event.payload.get("trial_sharpes", ())
                if isinstance(declared, list | tuple):
                    record["trial_sharpes"] = tuple(float(value) for value in declared)
                record["state"] = (
                    RegistrationState.COMPLETED
                    if outcome.produced_a_return_series
                    else RegistrationState.FAILED
                )
            elif event.kind is EventKind.REGISTRATION_VERIFIED:
                record["state"] = RegistrationState.REGISTRATION_RECONCILED
                record["receipt_digest"] = str(event.payload.get("receipt_digest", ""))
            elif event.kind is EventKind.TRIAL_ADMITTED:
                record["admitted"] = True
            elif event.kind is EventKind.REGISTRATION_INCOMPLETE:
                record["state"] = RegistrationState.INCOMPLETE
            elif event.kind is EventKind.DUPLICATE_REGISTRATION:
                continue

        return {
            key: AttemptRecord(**record)  # type: ignore[arg-type]
            for key, record in folded.items()
        }

    def gate_verdicts(self) -> tuple[GateVerdictEntry, ...]:
        """
        Return every gate verdict, in the order the gates reached them.

        A verdict is recorded for each gate that ran and for the one that stopped the chain, so this
        is the judgement history of the campaign rather than of one candidate. A reader that wants
        one candidate's severs it on `candidate_id`; the entries are returned in chain order because
        the last verdict for a candidate is the one that stopped its evaluation.

        Returns
        -------
        tuple[GateVerdictEntry, ...]

        """
        read: list[GateVerdictEntry] = []
        for event in self.events():
            if event.kind is not EventKind.GATE_VERDICT:
                continue
            payload = event.payload
            read.append(
                GateVerdictEntry(
                    candidate_id=event.candidate_id,
                    gate=str(payload.get("gate", "")),
                    status=str(payload.get("status", "")),
                    reason=str(payload.get("reason", "")),
                    evidence_digest=str(payload.get("evidence_digest", "")),
                    sequence=event.sequence,
                ),
            )

        return tuple(read)

    def receipts(self) -> Mapping[str, RunReceipt]:
        """
        Return every reconciled receipt, keyed by its idempotency key.

        Returns
        -------
        Mapping[str, RunReceipt]

        """
        read: dict[str, RunReceipt] = {}

        for event in self.events():
            if event.kind is not EventKind.REGISTRATION_VERIFIED:
                continue
            payload = event.payload.get("receipt")
            if isinstance(payload, dict):
                read[event.key] = RunReceipt(**payload)  # type: ignore[arg-type]

        return read

    def state_of(self, key: IdempotencyKey | str) -> RegistrationState:
        """
        Return one job's registration state.

        Parameters
        ----------
        key : IdempotencyKey | str
            The idempotency key.

        Returns
        -------
        RegistrationState

        """
        value = key.value if isinstance(key, IdempotencyKey) else key
        record = self.attempts().get(value)

        return RegistrationState.PENDING if record is None else record.state

    def record_proposal(
        self,
        candidate_id: str,
        *,
        generation: int = 0,
        agent_invocation: str = "",
    ) -> LedgerEvent:
        """
        Record a proposal, which is what a proposal attempt counts.

        Parameters
        ----------
        candidate_id : str
            The candidate the proposal names.
        generation : int, default 0
            The generation of the search it was proposed in.
        agent_invocation : str, default ""
            The recorded invocation that proposed it, or the empty string for an operator.

        Returns
        -------
        LedgerEvent

        """
        return self._append(
            EventKind.PROPOSAL_RECORDED,
            key="",
            candidate_id=candidate_id,
            payload={"generation": generation, "agent_invocation": agent_invocation},
        )

    def record_refusal(
        self,
        candidate_id: str,
        *,
        code: str,
        path: str = "",
        generation: int = 0,
    ) -> LedgerEvent:
        """
        Record a specification refused by the validator or the compiler.

        A refusal is a result rather than an error, and it is a proposal attempt rather than a
        statistical trial: it produced no return series and cannot inflate the maximum.

        Parameters
        ----------
        candidate_id : str
            The candidate the refusal is about, which for an uncompilable payload is its
            specification digest.
        code : str
            The refusal's code, from the closed vocabulary.
        path : str, default ""
            The dotted location the refusal is about.
        generation : int, default 0
            The generation of the search it was refused in.

        Returns
        -------
        LedgerEvent

        """
        if not code:
            raise ValueError("code must not be empty")

        return self._append(
            EventKind.SPECIFICATION_REFUSED,
            key="",
            candidate_id=candidate_id,
            payload={"code": code, "path": path, "generation": generation},
        )

    def record_compilation(
        self,
        candidate_id: str,
        *,
        specification_digest: str,
        new_configuration: bool,
        parent_id: str = "",
        generation: int = 0,
    ) -> LedgerEvent:
        """
        Record a compilation, which is what a unique configuration counts.

        Parameters
        ----------
        candidate_id : str
            The candidate's behavioural identity.
        specification_digest : str
            The digest of the submitted specification.
        new_configuration : bool
            Whether the equivalence rule reports the behavioural identity as new.
        parent_id : str, default ""
            The candidate it was mutated from, or the empty string when it has none.
        generation : int, default 0
            The generation of the search it was compiled in.

        Returns
        -------
        LedgerEvent

        """
        return self._append(
            EventKind.COMPILATION_RECORDED,
            key="",
            candidate_id=candidate_id,
            payload={
                "specification_digest": specification_digest,
                "new_configuration": new_configuration,
                "parent_id": parent_id,
                "generation": generation,
                "campaign_id": self._campaign_id,
                "family_id": self._family_id,
            },
        )

    def record_equivalence_merge(
        self,
        candidate_id: str,
        *,
        merged_into: str,
        rule: str,
    ) -> LedgerEvent:
        """
        Record that the equivalence rule merged an identity into an existing configuration.

        Both identities are recorded beside the merge, so a genuinely distinct configuration cannot
        disappear from the inventory and a coincidence cannot be performed silently.

        Parameters
        ----------
        candidate_id : str
            The identity that was merged.
        merged_into : str
            The identity it merged into.
        rule : str
            The pre-registered rule that performed the merge.

        Returns
        -------
        LedgerEvent

        """
        return self._append(
            EventKind.EQUIVALENCE_MERGE,
            key="",
            candidate_id=candidate_id,
            payload={"merged_into": merged_into, "rule": rule},
        )

    def submit_job(
        self,
        key: IdempotencyKey,
        *,
        candidate_id: str,
        data_role: DataRole = DataRole.DEVELOPMENT,
    ) -> LedgerEvent:
        """
        Submit a job under its idempotency key, or recognise a resubmission of one.

        A resubmission declares the same key and writes nothing: recognising it without comparing
        results is exactly what the key is for.

        Parameters
        ----------
        key : IdempotencyKey
            The job's idempotency key.
        candidate_id : str
            The candidate the job runs.
        data_role : DataRole, default DEVELOPMENT
            The role the data behind the result plays, recorded with the job because the same
            partition is development data for one lineage and locked for another.

        Returns
        -------
        LedgerEvent

        """
        with self._lock:
            existing = self._event_for(key.value, EventKind.JOB_SUBMITTED)
            if existing is not None:
                return existing

            return self._append(
                EventKind.JOB_SUBMITTED,
                key=key.value,
                candidate_id=candidate_id,
                payload={
                    "candidate_id": candidate_id,
                    "data_role": data_role.value,
                    "non_deterministic": non_deterministic_digest(),
                },
            )

    def close_attempt(
        self,
        key: IdempotencyKey,
        *,
        outcome: AttemptOutcome,
        detail: str = "",
        trial_sharpes: tuple[float, ...] = (),
    ) -> LedgerEvent:
        """
        Close an execution attempt, which is what an executed experiment counts.

        The close is written once per key: a replay of the registration step does not close the
        same attempt a second time, so an interrupted registration cannot become two executions.

        The runner is where a return series exists, so the per-period Sharpe ratios it measured are
        declared here rather than by whoever later reads the ledger: a gate that consumes the
        cross-trial dispersion can then read it from the inventory instead of being told it. The
        ratios are per-period rather than annualised, because the correction that consumes them is
        per-period, and an attempt that produced no usable series declares none.

        Parameters
        ----------
        key : IdempotencyKey
            The job's idempotency key.
        outcome : AttemptOutcome
            How the attempt closed.
        detail : str, default ""
            What the caller recorded about the outcome, such as the failure class.
        trial_sharpes : tuple[float, ...], default ()
            The per-period Sharpe ratios the attempt's return series produced, in period order.

        Returns
        -------
        LedgerEvent

        Raises
        ------
        ValueError
            If the key was never submitted, or its attempt already closed.

        """
        with self._lock:
            if self._event_for(key.value, EventKind.JOB_SUBMITTED) is None:
                raise ValueError(f"the key {key.value} was never submitted as a job")
            existing = self._event_for(key.value, EventKind.EXECUTION_ATTEMPT_CLOSED)
            if existing is not None:
                recorded = str(existing.payload.get("outcome", ""))
                if recorded != outcome.value:
                    raise ValueError(
                        f"the attempt {key.value} closed as {recorded} and cannot close as "
                        f"{outcome.value}",
                    )
                return existing

            return self._transition(
                key.value,
                EventKind.EXECUTION_ATTEMPT_CLOSED,
                candidate_id=str(self._candidate_for(key.value)),
                payload={
                    "outcome": outcome.value,
                    "detail": detail,
                    "trial_sharpes": list(trial_sharpes),
                },
            )

    def mark_incomplete(self, key: IdempotencyKey, *, reason: str) -> RegistrationOutcome:
        """
        Mark a registration incomplete, which is a state rather than an absence.

        An incomplete record is never counted and is never completed by inference. It is retryable,
        because recovery repeats the registration step against the same key, and that replay yields
        one statistical trial rather than two.

        Parameters
        ----------
        key : IdempotencyKey
            The job's idempotency key.
        reason : str
            Why the registration did not complete.

        Returns
        -------
        RegistrationOutcome

        Raises
        ------
        ValueError
            If the key was never submitted, or if its registration already reconciled, because
            nothing resets a reconciled registration.

        """
        if not reason:
            raise ValueError("reason must not be empty")

        with self._lock:
            return self._fail(key, reason=reason)

    def register_attempt(  # noqa: PLR0913 - the protocol's declared inputs
        self,
        key: IdempotencyKey,
        *,
        candidate_id: str,
        outcome: AttemptOutcome,
        receipt: RunReceipt,
        manifest: Mapping[str, object],
        artifact_digest: str | None = None,
        data_role: DataRole = DataRole.DEVELOPMENT,
    ) -> RegistrationOutcome:
        """
        Run the registration protocol for one attempt, replayable to one statistical trial.

        The steps are the design's: the job is submitted under its key, the attempt is closed with
        its outcome, the result is verified against its artifact and its receipt, and only then is
        a statistical trial admitted. Each step is guarded by the event it writes, so repeating the
        whole call after an interruption closes no attempt twice, verifies no result twice and
        admits one trial in total. A registration that already reconciled writes the duplicate
        event and changes no count.

        Parameters
        ----------
        key : IdempotencyKey
            The job's idempotency key.
        candidate_id : str
            The candidate the attempt ran.
        outcome : AttemptOutcome
            How the attempt closed.
        receipt : RunReceipt
            The receipt the runner emitted.
        manifest : Mapping[str, object]
            The engine-side facts the receipt is reconciled against.
        artifact_digest : str | None, default None
            The digest of the artifact as the research side holds it, or None to check the receipt
            against itself. A value that differs from the receipt's is a tampered receipt or a
            moved artifact, and it fails closed.
        data_role : DataRole, default DEVELOPMENT
            The role the data behind the result plays, recorded with the job.

        Returns
        -------
        RegistrationOutcome

        """
        with self._lock:
            state = self.state_of(key)
            if state is RegistrationState.REGISTRATION_RECONCILED:
                self._append(
                    EventKind.DUPLICATE_REGISTRATION,
                    key=key.value,
                    candidate_id=candidate_id,
                    payload={
                        "reason": "the attempt already reconciled under this key",
                        "receipt_digest": receipt.digest(),
                    },
                )
                return RegistrationOutcome(
                    key=key.value,
                    state=state,
                    counted=False,
                    idempotent=True,
                    reason="already reconciled",
                )

            self.submit_job(key, candidate_id=candidate_id, data_role=data_role)
            self.close_attempt(key, outcome=outcome)

            if not outcome.produced_a_return_series:
                return self._fail(
                    key,
                    reason=(
                        f"the attempt {outcome.value} and produced no usable return series, so it "
                        f"is never given an imputed one"
                    ),
                )

            reconciliation = reconcile_receipt(receipt, manifest=manifest)
            mismatched = list(reconciliation.mismatched)
            detail = dict(reconciliation.detail)

            if artifact_digest is not None and artifact_digest != receipt.artifact_digest:
                mismatched.append("artifact_digest")
                detail["artifact_digest"] = (
                    f"the receipt holds {receipt.artifact_digest!r}, the artifact is "
                    f"{artifact_digest!r}"
                )

            if mismatched:
                failed = Reconciliation(
                    checked=reconciliation.checked,
                    mismatched=tuple(mismatched),
                    detail=detail,
                )
                return self._fail(key, reason=failed.describe(), reconciliation=failed)

            self._transition(
                key.value,
                EventKind.REGISTRATION_VERIFIED,
                candidate_id=candidate_id,
                payload={"receipt": receipt.to_dict(), "receipt_digest": receipt.digest()},
            )
            self._transition(
                key.value,
                EventKind.TRIAL_ADMITTED,
                candidate_id=candidate_id,
                payload={"receipt_digest": receipt.digest()},
            )

            return RegistrationOutcome(
                key=key.value,
                state=RegistrationState.REGISTRATION_RECONCILED,
                counted=True,
                idempotent=False,
                reconciliation=reconciliation,
            )

    def record_gate_verdict(
        self,
        candidate_id: str,
        *,
        gate: str,
        status: str,
        evidence_digest: str,
        reason: str = "",
    ) -> LedgerEvent:
        """
        Record a gate's verdict, which is evidence rather than multiplicity.

        The reason is recorded with the verdict, because a reader reconstructing why a candidate was
        rejected needs the gate's own explanation rather than only the fact that it stopped there.

        Parameters
        ----------
        candidate_id : str
            The candidate the verdict is about.
        gate : str
            The gate's name.
        status : str
            The verdict, from the gate's own vocabulary.
        evidence_digest : str
            The digest of the evidence the verdict was reached on.
        reason : str, default ""
            Why the gate reached the verdict.

        Returns
        -------
        LedgerEvent

        """
        return self._append(
            EventKind.GATE_VERDICT,
            key="",
            candidate_id=candidate_id,
            payload={
                "gate": gate,
                "status": status,
                "evidence_digest": evidence_digest,
                "reason": reason,
            },
        )

    def record_custody_event(
        self,
        candidate_id: str,
        *,
        transition: str,
        detail: Mapping[str, object],
    ) -> LedgerEvent:
        """
        Record a custody transition, which is written with the access it consumed.

        Parameters
        ----------
        candidate_id : str
            The candidate the event is about.
        transition : str
            The transition, from the custody vocabulary.
        detail : Mapping[str, object]
            The reservation, the access and the identities the transition is bound to.

        Returns
        -------
        LedgerEvent

        """
        return self._append(
            EventKind.CUSTODY_EVENT,
            key="",
            candidate_id=candidate_id,
            payload={"transition": transition, "detail": dict(detail)},
        )

    def record_search_policy(
        self,
        *,
        search_policy_digest: str,
        generation: int,
    ) -> LedgerEvent:
        """
        Record the search policy a generation ran under.

        Parameters
        ----------
        search_policy_digest : str
            The digest of the search policy.
        generation : int
            The generation it was declared for.

        Returns
        -------
        LedgerEvent

        """
        return self._append(
            EventKind.SEARCH_POLICY_RECORDED,
            key="",
            candidate_id="",
            payload={"search_policy_digest": search_policy_digest, "generation": generation},
        )

    def checkpoint(self) -> str:
        """
        Return the digest of the last event, which is what a checkpoint holds.

        Returns
        -------
        str
            The `sha256:<hex>` digest, or the empty string for an empty inventory.

        """
        events = self.events()

        return "" if not events else events[-1].digest()

    def write_checkpoint(self, path: str | Path) -> Path:
        """
        Write the current head digest to a path outside the write path.

        Parameters
        ----------
        path : str | Path
            Where the checkpoint is written.

        Returns
        -------
        Path

        """
        target = Path(path)
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(
            json.dumps(
                {
                    "schema": INVENTORY_SCHEMA,
                    "campaign_id": self._campaign_id,
                    "family_id": self._family_id,
                    "events": len(self.events()),
                    "head": self.checkpoint(),
                },
                indent=2,
                sort_keys=True,
            )
            + "\n",
            encoding="utf-8",
        )

        return target

    def verify_chain(self, *, checkpoint: str | None = None) -> ChainVerification:
        """
        Verify the chain: every link, every sequence number and the checkpoint when given.

        Parameters
        ----------
        checkpoint : str | None, default None
            A head digest held outside the write path, or None to verify only the chain itself.

        Returns
        -------
        ChainVerification

        """
        events = self.events()
        findings: list[str] = []
        previous = ""

        for position, event in enumerate(events):
            if event.sequence != position:
                findings.append(
                    f"event {position} records sequence {event.sequence}: a record was removed "
                    f"or inserted",
                )
            if event.previous != previous:
                findings.append(
                    f"event {position} chains to {event.previous!r} instead of {previous!r}",
                )
            previous = event.digest()

        head = previous
        if checkpoint is not None and checkpoint != head:
            findings.append(
                f"the inventory ends at {head!r} and the checkpoint holds {checkpoint!r}",
            )

        return ChainVerification(events=len(events), head=head, findings=tuple(findings))

    def _candidate_for(self, key: str) -> str:
        """
        Return the candidate a submitted key runs.
        """
        submitted = self._event_for(key, EventKind.JOB_SUBMITTED)
        if submitted is None:
            raise ValueError(f"the key {key} was never submitted as a job")

        return submitted.candidate_id

    def _event_for(self, key: str, kind: EventKind) -> LedgerEvent | None:
        """
        Return the first event of a kind recorded for a key, or None.
        """
        for event in self.events():
            if event.key == key and event.kind is kind:
                return event

        return None

    def _fail(
        self,
        key: IdempotencyKey,
        *,
        reason: str,
        reconciliation: Reconciliation | None = None,
    ) -> RegistrationOutcome:
        """
        Write an incomplete registration and report it as retryable.
        """
        self._transition(
            key.value,
            EventKind.REGISTRATION_INCOMPLETE,
            candidate_id=self._candidate_for(key.value),
            payload={"reason": reason},
        )

        return RegistrationOutcome(
            key=key.value,
            state=RegistrationState.INCOMPLETE,
            counted=False,
            idempotent=False,
            reason=reason,
            reconciliation=reconciliation,
        )

    def _transition(
        self,
        key: str,
        kind: EventKind,
        *,
        candidate_id: str,
        payload: Mapping[str, object],
    ) -> LedgerEvent:
        """
        Append an event that moves a key's registration, refusing an illegal transition.
        """
        current = self.state_of(key)
        if kind is EventKind.EXECUTION_ATTEMPT_CLOSED:
            closed = AttemptOutcome(str(payload["outcome"]))
            target = (
                RegistrationState.COMPLETED
                if closed.produced_a_return_series
                else RegistrationState.FAILED
            )
        elif kind is EventKind.REGISTRATION_INCOMPLETE:
            target = RegistrationState.INCOMPLETE
        elif kind in (EventKind.REGISTRATION_VERIFIED, EventKind.TRIAL_ADMITTED):
            target = RegistrationState.REGISTRATION_RECONCILED
        else:
            raise ValueError(f"{kind.value} is not a registration transition")

        if target not in _PERMITTED_REGISTRATIONS[current]:
            raise ValueError(
                f"the registration {key} is {current.value} and may not move to {target.value}",
            )

        return self._append(kind, key=key, candidate_id=candidate_id, payload=payload)

    def _append(
        self,
        kind: EventKind,
        *,
        key: str,
        candidate_id: str,
        payload: Mapping[str, object],
    ) -> LedgerEvent:
        """
        Write one event, with the chain's previous digest and the next sequence number.
        """
        required = _REQUIRED_FIELDS[kind]
        missing = [name for name in required if name not in payload]
        if missing:
            raise ValueError(f"a {kind.value} event is missing {sorted(missing)}")

        with self._lock:
            events = self.events()
            event = LedgerEvent(
                sequence=len(events),
                kind=kind,
                key=key,
                candidate_id=candidate_id,
                payload=dict(payload),
                previous="" if not events else events[-1].digest(),
            )
            with self._events_path.open("a", encoding="utf-8") as handle:
                handle.write(json.dumps(event.to_dict(), sort_keys=True) + "\n")

            return event


class EnvelopeKind(Enum):
    """
    What a reproducibility-envelope reference names, from the design's list of contents.

    Every kind is required of a complete envelope except `EXTERNAL_ARTIFACT`, which is declared as
    often as a run depends on something outside the repository. A kind that genuinely holds nothing
    for a run declares that as its content rather than being omitted, so "nothing" is a value a
    reader can see rather than a gap a reader has to interpret.
    """

    SNAPSHOT = "snapshot"
    FEATURE_DEFINITIONS = "feature_definitions"
    SPECIFICATION = "specification"
    COMPILED_PLAN = "compiled_plan"
    ENGINE_BUILD = "engine_build"
    CONFIGURATION = "configuration"
    SEED = "seed"
    STOCHASTIC_PARAMETERS = "stochastic_parameters"
    COST_POLICY = "cost_policy"
    FILL_POLICY = "fill_policy"
    LATENCY_POLICY = "latency_policy"
    CORPORATE_ACTION_POLICY = "corporate_action_policy"
    SPLIT_DEFINITION = "split_definition"
    CONFIRMATION_DEFINITION = "confirmation_definition"
    ANALYSIS_IMPLEMENTATION = "analysis_implementation"
    STATISTICAL_CONTRACT = "statistical_contract"
    EXTERNAL_ARTIFACT = "external_artifact"


# The kinds a complete envelope must declare, which is every kind but the external one.
REQUIRED_REFERENCE_KINDS: tuple[EnvelopeKind, ...] = tuple(
    kind for kind in EnvelopeKind if kind is not EnvelopeKind.EXTERNAL_ARTIFACT
)


@dataclass(frozen=True)
class TrialMapping:
    """
    What the ledger supplies to every method that needs a count, under the versioned mapping.

    The mapping is one implementation rather than one per method, which is the mechanical
    consequence the plan states: a method cannot quietly define its own counting rule. It carries
    the four counts, the admitted trials by identity, the attempts that produced no usable return
    series, and the data role each job ran under, so a gate reads its multiplicity and its
    admissibility from the ledger rather than from a number a caller supplies.

    Parameters
    ----------
    counts : Counts
        The four counts, folded from the inventory's events.
    admitted : Mapping[str, str]
        The admitted trials, keyed by idempotency key, valued by the candidate they belong to.
    unusable : Mapping[str, str]
        The attempts that produced no usable return series, keyed by key and valued by the reason.
    data_roles : Mapping[str, DataRole]
        The role each submitted job ran under, keyed by idempotency key.
    effective_trials : int | None
        The effective trial count the dependence declaration produced, or None when the statistical
        policy declares the trials independent.

    """

    counts: Counts
    admitted: Mapping[str, str] = field(default_factory=dict)
    unusable: Mapping[str, str] = field(default_factory=dict)
    data_roles: Mapping[str, DataRole] = field(default_factory=dict)
    effective_trials: int | None = None

    @property
    def statistical_trials(self) -> int:
        """
        The number of verified executions admitted to the selection process.
        """
        return self.counts.statistical_trials

    @property
    def executed_experiments(self) -> int:
        """
        Every attempt that closed, whatever its outcome.
        """
        return self.counts.executed_experiments

    @property
    def unusable_attempts(self) -> int:
        """
        How many of them produced no usable return series.
        """
        return len(self.unusable)

    def to_dict(self) -> dict[str, object]:
        """
        Return the mapping as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "counts": self.counts.to_dict(),
            "admitted": dict(self.admitted),
            "unusable": dict(self.unusable),
            "data_roles": {key: role.value for key, role in self.data_roles.items()},
            "effective_trials": self.effective_trials,
        }


def recorded_trial_sharpes(inventory: CampaignInventory) -> tuple[float, ...]:
    """
    Return the per-period Sharpe ratios every attempt declared, in idempotency key order.

    This is the reader a gate uses instead of being told the cross-trial dispersion: the ratios are
    read from the inventory, so a correction that consumes them is computed from what the runs
    measured rather than from what a caller asserted. An attempt that declared none contributes
    nothing, which is the same statement as an attempt that produced no usable return series.

    Parameters
    ----------
    inventory : CampaignInventory
        The campaign's inventory.

    Returns
    -------
    tuple[float, ...]

    """
    return tuple(
        sharpes
        for _, record in sorted(inventory.attempts().items())
        for sharpes in record.trial_sharpes
    )


def trial_mapping(
    inventory: CampaignInventory,
    *,
    dependence: TrialDependence = TrialDependence.INDEPENDENT,
    effective_trials: int | None = None,
) -> TrialMapping:
    """
    Return the mapping from the inventory's events to the inputs every method consumes.

    A method that needs a trial count reads it here, from the ledger, rather than from a number a
    caller supplies: that is what makes the deflation's multiplicity auditable. Raw and effective
    counts are separate numbers and are never substituted for each other.

    Parameters
    ----------
    inventory : CampaignInventory
        The campaign's inventory.
    dependence : TrialDependence, default INDEPENDENT
        Whether the campaign's trials are independent draws.
    effective_trials : int | None, default None
        The effective trial count, required when the trials are dependent.

    Returns
    -------
    TrialMapping

    Raises
    ------
    ValueError
        If the dependence declaration and the effective count disagree.

    """
    if dependence is TrialDependence.DEPENDENT and effective_trials is None:
        raise ValueError("a campaign with dependent trials must declare its effective trial count")
    if dependence is TrialDependence.INDEPENDENT and effective_trials is not None:
        raise ValueError("a campaign with independent trials must not declare an effective count")

    admitted: dict[str, str] = {}
    unusable: dict[str, str] = {}
    roles: dict[str, DataRole] = {}

    for event in inventory.events():
        if event.kind is EventKind.TRIAL_ADMITTED:
            admitted[event.key] = event.candidate_id
        elif event.kind is EventKind.REGISTRATION_INCOMPLETE:
            unusable[event.key] = str(event.payload.get("reason", ""))
        elif event.kind is EventKind.JOB_SUBMITTED:
            roles[event.key] = DataRole(str(event.payload.get("data_role", "development")))

    return TrialMapping(
        counts=inventory.counts(),
        admitted=admitted,
        unusable=unusable,
        data_roles=roles,
        effective_trials=effective_trials,
    )


def _judgement_outcome(
    *,
    refusals: tuple[str, ...],
    attempts: tuple[str, ...],
    admitted: int,
    verdicts: tuple[GateVerdictEntry, ...],
) -> str:
    """
    Derive one candidate's summary outcome from the facts the ledger retains.

    The rule is stated rather than inferred from the fields, because a reader wants one word and the
    fields behind it are all present: the last verdict's status when a gate judged the candidate,
    otherwise whether it ran and was admitted, otherwise whether it was refused, otherwise that it
    was only proposed.
    """
    if verdicts:
        return verdicts[-1].status
    if attempts:
        return "admitted" if admitted else "ran"

    return "refused" if refusals else "proposed"


@dataclass(frozen=True)
class CandidateJudgement:
    """
    One candidate's history, as the ledger retains it.

    The four facts criterion 16 asks for are separate fields rather than one summary: the refusals
    the validator recorded, the attempts that ran and how each closed, how many of them entered the
    statistical inventory, and the gate verdicts with their reasons. `outcome` is the derived
    one-word answer, and every fact behind it remains readable.

    Parameters
    ----------
    candidate_id : str
        The candidate.
    generation : int, default 0
        The generation of the search it was proposed in.
    parent_id : str, default ""
        The candidate it was mutated from, or the empty string when it has none.
    refusals : tuple[str, ...], default ()
        The refusal codes recorded for it, in the order they were recorded.
    attempts : tuple[str, ...], default ()
        The idempotency keys of its executions, in key order.
    outcomes : tuple[str, ...], default ()
        How those attempts closed, in the same order.
    admitted : int, default 0
        How many of its attempts entered the statistical inventory.
    verdicts : tuple[GateVerdictEntry, ...], default ()
        The gate verdicts reached about it, in the order the gates reached them.
    outcome : str, default ""
        The derived summary: the last verdict's status, `admitted`, `ran`, `refused` or `proposed`.

    """

    candidate_id: str
    generation: int = 0
    parent_id: str = ""
    refusals: tuple[str, ...] = ()
    attempts: tuple[str, ...] = ()
    outcomes: tuple[str, ...] = ()
    admitted: int = 0
    verdicts: tuple[GateVerdictEntry, ...] = ()
    outcome: str = ""

    @property
    def reasons(self) -> tuple[str, ...]:
        """
        Why the gates reached the verdicts they did, in the order they were reached.
        """
        return tuple(verdict.reason for verdict in self.verdicts)

    def to_dict(self) -> dict[str, object]:
        """
        Return the judgement as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "candidate_id": self.candidate_id,
            "generation": self.generation,
            "parent_id": self.parent_id,
            "refusals": list(self.refusals),
            "attempts": list(self.attempts),
            "outcomes": list(self.outcomes),
            "admitted": self.admitted,
            "verdicts": [verdict.to_dict() for verdict in self.verdicts],
            "outcome": self.outcome,
        }


@dataclass(frozen=True)
class CampaignAudit:
    """
    A whole campaign as the ledger, the policy digest and the engine version reconstruct it.

    This is a derived view, not a new record: it reads the chain, the lineage, the registrations and
    the verdicts and states what they show. The policy digest and the engine version are inputs
    rather than ledger contents, and the audit reports what the ledger does and does not corroborate
    instead of asserting that they agree: the receipts' build identities are reported as recorded,
    and the seed, binary and configuration join of decision M1.3 remains the open item it is.

    Parameters
    ----------
    campaign_id : str
        The campaign the inventory was opened for.
    family_id : str
        The research family that campaign belongs to.
    events : int, default 0
        How many events the chain holds.
    head : str, default ""
        The digest of the last event, which is what a checkpoint holds.
    counts : Counts
        The four counts under the versioned mapping.
    policy_digest : str, default ""
        The policy digest the reader is holding, or the empty string when none was declared.
    recorded_policy_digest : str, default ""
        The policy digest the inventory was opened with.
    search_policy_digests : tuple[str, ...], default ()
        The digests the generations recorded, in the order they were recorded.
    engine_version : str, default ""
        The engine version the reader is holding, or the empty string when none was declared.
    build_identities : tuple[str, ...], default ()
        The build identities the reconciled receipts carry, in sorted order.
    candidates : tuple[CandidateJudgement, ...], default ()
        Every candidate the ledger names, in first-seen order.

    """

    campaign_id: str
    family_id: str
    events: int = 0
    head: str = ""
    counts: Counts = field(default_factory=Counts)
    policy_digest: str = ""
    recorded_policy_digest: str = ""
    search_policy_digests: tuple[str, ...] = ()
    engine_version: str = ""
    build_identities: tuple[str, ...] = ()
    candidates: tuple[CandidateJudgement, ...] = ()

    @property
    def policy_matches(self) -> bool | None:
        """
        Whether the declared policy digest is the one the inventory was opened with, or None.

        None means the reader declared none, which is the honest answer rather than a failed match:
        a policy digest is an input to the audit, not something the ledger can invent.
        """
        if not self.policy_digest:
            return None

        return self.policy_digest == self.recorded_policy_digest

    @property
    def judged(self) -> tuple[CandidateJudgement, ...]:
        """
        The candidates a gate judged, in ledger order.
        """
        return tuple(candidate for candidate in self.candidates if candidate.verdicts)

    def to_dict(self) -> dict[str, object]:
        """
        Return the audit as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "campaign_id": self.campaign_id,
            "family_id": self.family_id,
            "events": self.events,
            "head": self.head,
            "counts": self.counts.to_dict(),
            "policy_digest": self.policy_digest,
            "recorded_policy_digest": self.recorded_policy_digest,
            "policy_matches": self.policy_matches,
            "search_policy_digests": list(self.search_policy_digests),
            "engine_version": self.engine_version,
            "build_identities": list(self.build_identities),
            "candidates": [candidate.to_dict() for candidate in self.candidates],
        }

    def digest(self) -> str:
        """
        Return the digest of the audit.

        Returns
        -------
        str
            The `sha256:<hex>` digest.

        """
        return digest_of(canonical_mapping(self.to_dict()))


def audit_campaign(
    inventory: CampaignInventory,
    *,
    policy_digest: str = "",
    engine_version: str = "",
) -> CampaignAudit:
    """
    Reconstruct a whole campaign from what the ledger retained.

    A reader with the ledger, the policy digest and the engine version can reconstruct which
    candidates existed, which ran, which were judged, under which policy, and why each was rejected.
    Every candidate the ledger names is reported, including one that was only refused: the lineage
    holds the compiled ones, and a refusal, a verdict or an attempt names the rest.

    Parameters
    ----------
    inventory : CampaignInventory
        The campaign's inventory.
    policy_digest : str, default ""
        The policy digest the reader holds, compared against the recorded one.
    engine_version : str, default ""
        The engine version the reader holds, reported beside the receipts' build identities.

    Returns
    -------
    CampaignAudit

    """
    events = inventory.events()

    refusals: dict[str, list[str]] = {}
    verdicts: dict[str, list[GateVerdictEntry]] = {}
    search_policies: list[str] = []

    for event in events:
        if event.kind is EventKind.SPECIFICATION_REFUSED:
            refusals.setdefault(event.candidate_id, []).append(str(event.payload.get("code", "")))
        elif event.kind is EventKind.GATE_VERDICT:
            verdicts.setdefault(event.candidate_id, []).append(
                GateVerdictEntry(
                    candidate_id=event.candidate_id,
                    gate=str(event.payload.get("gate", "")),
                    status=str(event.payload.get("status", "")),
                    reason=str(event.payload.get("reason", "")),
                    evidence_digest=str(event.payload.get("evidence_digest", "")),
                    sequence=event.sequence,
                ),
            )
        elif event.kind is EventKind.SEARCH_POLICY_RECORDED:
            search_policies.append(str(event.payload.get("search_policy_digest", "")))

    attempts: dict[str, list[AttemptRecord]] = {}
    for _, record in sorted(inventory.attempts().items()):
        attempts.setdefault(record.candidate_id, []).append(record)

    lineage = {record.candidate_id: record for record in inventory.lineage()}
    named: dict[str, None] = dict.fromkeys(lineage)
    for source in (refusals, verdicts, attempts):
        for candidate_id in source:
            named.setdefault(candidate_id, None)

    candidates: list[CandidateJudgement] = []
    for candidate_id in named:
        record = lineage.get(candidate_id)
        records = tuple(attempts.get(candidate_id, ()))
        reached = tuple(verdicts.get(candidate_id, ()))
        codes = tuple(refusals.get(candidate_id, ()))
        admitted = sum(1 for attempt in records if attempt.admitted)
        keys = tuple(attempt.key for attempt in records)
        outcomes = tuple(
            attempt.outcome.value for attempt in records if attempt.outcome is not None
        )
        candidates.append(
            CandidateJudgement(
                candidate_id=candidate_id,
                generation=0 if record is None else record.generation,
                parent_id="" if record is None else record.parent_id,
                refusals=codes,
                attempts=keys,
                outcomes=outcomes,
                admitted=admitted,
                verdicts=reached,
                outcome=_judgement_outcome(
                    refusals=codes,
                    attempts=keys,
                    admitted=admitted,
                    verdicts=reached,
                ),
            ),
        )

    return CampaignAudit(
        campaign_id=inventory.campaign_id,
        family_id=inventory.family_id,
        events=len(events),
        head=inventory.checkpoint(),
        counts=inventory.counts(),
        policy_digest=policy_digest,
        recorded_policy_digest=inventory.policy_digest,
        search_policy_digests=tuple(search_policies),
        engine_version=engine_version,
        build_identities=tuple(
            sorted({receipt.build_identity for receipt in inventory.receipts().values()}),
        ),
        candidates=tuple(candidates),
    )


@dataclass(frozen=True)
class EnvelopeReference:
    """
    One named input a recomputation needs, with the content that proves it is still available.

    A digest proves an artifact's identity and not its availability, so a reference carries the
    content it was taken over whenever that content was retained. A reference whose content was not
    retained names its digest and nothing else, and it does not resolve: the envelope is incomplete
    until the content is produced.

    Parameters
    ----------
    name : str
        The reference's name, unique within its envelope.
    kind : EnvelopeKind
        What the reference names.
    digest : str
        The digest of the content, or the empty string for a reference with no retained content.
    content : Mapping[str, object] | None, default None
        The retained content, or None when the content was not retained.

    """

    name: str
    kind: EnvelopeKind
    digest: str = ""
    content: Mapping[str, object] | None = None

    def __post_init__(self) -> None:
        """
        Refuse a reference that names nothing and holds nothing.
        """
        if not self.name:
            raise ValueError("name must not be empty")
        if not isinstance(self.kind, EnvelopeKind):
            raise TypeError("kind must be a EnvelopeKind")

    @classmethod
    def of(
        cls,
        name: str,
        kind: EnvelopeKind,
        content: Mapping[str, object],
    ) -> EnvelopeReference:
        """
        Return a reference whose digest is taken over the content it retains.

        Parameters
        ----------
        name : str
            The reference's name.
        kind : EnvelopeKind
            What the reference names.
        content : Mapping[str, object]
            The content, retained so a reader resolves it rather than trusting its digest.

        Returns
        -------
        EnvelopeReference

        """
        return cls(
            name=name,
            kind=kind,
            digest=digest_of(canonical_mapping(content)),
            content=dict(content),
        )

    @property
    def resolves(self) -> bool:
        """
        Whether the retained content still digests to the declared value.
        """
        return self.content is not None and self.recomputed_digest() == self.digest

    def recomputed_digest(self) -> str:
        """
        Return the digest of the retained content, or the empty string when none is retained.

        Returns
        -------
        str

        """
        if self.content is None:
            return ""

        return digest_of(canonical_mapping(self.content))

    def to_dict(self) -> dict[str, object]:
        """
        Return the reference as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "name": self.name,
            "kind": self.kind.value,
            "digest": self.digest,
            "content": None if self.content is None else dict(self.content),
        }


@dataclass(frozen=True)
class ReproducibilityEnvelope:
    """
    Everything a recomputation needs, declared twice over: as a digest and as retained content.

    Parameters
    ----------
    references : tuple[EnvelopeReference, ...]
        The references, at most one per required kind.
    schema : str, default `ENVELOPE_SCHEMA`
        The format identity.

    Raises
    ------
    ValueError
        If a required kind is missing, a kind repeats, a name repeats, or the references carry an
        external artifact under a name that already exists.

    """

    references: tuple[EnvelopeReference, ...]
    schema: str = ENVELOPE_SCHEMA

    def __post_init__(self) -> None:
        """
        Validate that the envelope declares every required kind exactly once.
        """
        names = [reference.name for reference in self.references]
        if len(set(names)) != len(names):
            raise ValueError("envelope reference names must be unique")

        declared = [reference.kind for reference in self.references]
        missing = [kind.value for kind in REQUIRED_REFERENCE_KINDS if kind not in declared]
        if missing:
            raise ValueError(f"the envelope does not declare {missing}")

        for kind in REQUIRED_REFERENCE_KINDS:
            if declared.count(kind) > 1:
                raise ValueError(f"the envelope declares {kind.value} more than once")

    def reference(self, kind: EnvelopeKind) -> EnvelopeReference:
        """
        Return the reference that names a kind.

        Parameters
        ----------
        kind : EnvelopeKind
            The kind to read.

        Returns
        -------
        EnvelopeReference

        Raises
        ------
        KeyError
            If the envelope declares no reference of that kind.

        """
        for reference in self.references:
            if reference.kind is kind:
                return reference

        raise KeyError(kind.value)

    def unresolved(self) -> tuple[str, ...]:
        """
        Return the names of the references whose content does not resolve.

        Returns
        -------
        tuple[str, ...]

        """
        return tuple(reference.name for reference in self.references if not reference.resolves)

    def digest(self) -> str:
        """
        Return the digest of the whole envelope.

        Returns
        -------
        str
            The `sha256:<hex>` digest.

        """
        return digest_of(
            canonical_mapping(
                {
                    "schema": self.schema,
                    "references": [reference.to_dict() for reference in self.references],
                },
            ),
        )


@dataclass(frozen=True)
class RecomputedQuantity:
    """
    One quantity, recomputed from a second source and compared with what was recorded.

    Parameters
    ----------
    name : str
        The quantity's name, which names the source it was recomputed from.
    recorded : str
        What the research side recorded.
    recomputed : str
        What the recomputation produced.
    agrees : bool
        Whether the two are equal.

    """

    name: str
    recorded: str
    recomputed: str
    agrees: bool

    def to_dict(self) -> dict[str, object]:
        """
        Return the quantity as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "name": self.name,
            "recorded": self.recorded,
            "recomputed": self.recomputed,
            "agrees": self.agrees,
        }


@dataclass(frozen=True)
class Recomputation:
    """
    What a reader recomputed from the ledger, the inventory and the retained artifacts.

    Parameters
    ----------
    quantities : tuple[RecomputedQuantity, ...]
        Every quantity that was recomputed and compared.
    unresolved : tuple[str, ...]
        Envelope references that do not resolve, which make the package incomplete.
    unavailable : tuple[str, ...]
        What cannot be recomputed here, each with the dependency that blocks it named.
    non_deterministic : str
        The digest of the non-deterministic components the run declared.
    counts : Counts
        The four counts, folded from the inventory's events.

    """

    quantities: tuple[RecomputedQuantity, ...]
    unresolved: tuple[str, ...] = ()
    unavailable: tuple[str, ...] = ()
    non_deterministic: str = ""
    counts: Counts = field(default_factory=Counts)

    @property
    def disagreements(self) -> tuple[str, ...]:
        """
        The names of the quantities whose two values differ.
        """
        return tuple(quantity.name for quantity in self.quantities if not quantity.agrees)

    @property
    def agrees(self) -> bool:
        """
        Whether every quantity recomputed, every reference resolved, and nothing is unavailable.
        """
        return not self.disagreements and not self.unresolved and not self.unavailable

    def require(self) -> None:
        """
        Raise unless every number recomputed and every reference resolved.

        Raises
        ------
        ValueError
            If a quantity disagrees, a reference does not resolve, or an input is unavailable.

        """
        if self.agrees:
            return

        problems: list[str] = []
        if self.disagreements:
            problems.append(f"quantities that disagree: {list(self.disagreements)}")
        if self.unresolved:
            problems.append(f"references that do not resolve: {list(self.unresolved)}")
        if self.unavailable:
            problems.append(f"inputs that are unavailable: {list(self.unavailable)}")

        raise ValueError("the recomputation is not complete: " + "; ".join(problems))

    def to_dict(self) -> dict[str, object]:
        """
        Return the recomputation as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "quantities": [quantity.to_dict() for quantity in self.quantities],
            "unresolved": list(self.unresolved),
            "unavailable": list(self.unavailable),
            "non_deterministic": self.non_deterministic,
            "counts": self.counts.to_dict(),
            "agrees": self.agrees,
        }


def recompute(
    ledger: DiscoveryLedger,
    *,
    inventory: CampaignInventory,
    store: ExperimentStore | None = None,
    envelope: ReproducibilityEnvelope | None = None,
    unavailable: Sequence[str] = (),
) -> Recomputation:
    """
    Recompute a campaign's numbers from the ledger, the inventory and the retained artifacts.

    The recomputation is a cross-store check rather than a re-derivation of one store from itself:
    each entry in the ledger is the research side's claim, the inventory is the campaign's record of
    what it admitted, and the retained artifacts are the content the two cite. A claim that only one
    side supports is a disagreement, and a reference whose content is gone is unresolved.

    Two inputs cannot be recomputed here and are named with the dependency that blocks each, so a
    caller reports a blocked check rather than passing on the parts it could do: the engine's
    canonical digest is computed by the extension from the document it holds, and the engine-side
    manifest a receipt is checked against has no Python binding until decision M1.3 is implemented.

    Parameters
    ----------
    ledger : DiscoveryLedger
        The research side: one evidence package per experiment.
    inventory : CampaignInventory
        The campaign's event log.
    store : ExperimentStore | None, default None
        The store that retains canonical artifacts, or None when the caller holds none.
    envelope : ReproducibilityEnvelope | None, default None
        The reproducibility envelope, or None when the run declared none.
    unavailable : Sequence[str], default ()
        Further inputs the caller knows cannot be recomputed, each with its dependency named.

    Returns
    -------
    Recomputation

    """
    quantities: list[RecomputedQuantity] = []
    admitted = {
        record.candidate_id: key for key, record in inventory.attempts().items() if record.admitted
    }
    entries = ledger.entries()
    receipts = inventory.receipts()

    def compare(name: str, recorded: str, recomputed: str) -> None:
        quantities.append(
            RecomputedQuantity(
                name=name,
                recorded=recorded,
                recomputed=recomputed,
                agrees=recorded == recomputed,
            ),
        )

    compare("research_entries", str(len(entries)), str(len(admitted)))

    blocked: list[str] = []
    for entry in entries:
        candidate = str(entry.get("candidate_id"))
        key = admitted.get(candidate, "")
        recorded_canonical = str(entry.get("canonical_digest", ""))
        receipt = receipts.get(key)
        compare(
            f"canonical_digest:{candidate}",
            recorded_canonical,
            "" if not key or receipt is None else receipt.artifact_digest,
        )

        recorded_bytes = str(entry.get("canonical_bytes_sha256", ""))
        if store is None:
            blocked.append(
                "the retained canonical artifact of "
                f"{candidate}, which needs the store that holds it",
            )
            continue

        try:
            retained = store.canonical_bytes(recorded_canonical)
        except FileNotFoundError:
            retained = b""
        compare(
            f"canonical_bytes_sha256:{candidate}",
            recorded_bytes,
            "" if not retained else f"sha256:{hashlib.sha256(retained).hexdigest()}",
        )

    declarations = {
        str(event.payload.get("non_deterministic", ""))
        for event in inventory.events()
        if event.kind is EventKind.JOB_SUBMITTED
    }
    match len(declarations):
        case 0:
            declared = ""
        case 1:
            declared = next(iter(declarations))
        case _:
            declared = ",".join(sorted(declarations))
    compare("non_deterministic_components", declared, non_deterministic_digest())

    unresolved: tuple[str, ...] = ()
    if envelope is not None:
        for reference in envelope.references:
            compare(
                f"envelope:{reference.name}",
                reference.digest,
                reference.recomputed_digest(),
            )
        unresolved = envelope.unresolved()

    blocked.extend(
        (
            (
                "the engine's canonical blake3 digest, which the extension computes from the "
                "document it holds and no Python surface recomputes from retained bytes"
            ),
            (
                "the engine-side run manifest a receipt is checked against, which has no Python "
                "binding until decision M1.3 is implemented"
            ),
        ),
    )
    blocked.extend(unavailable)

    return Recomputation(
        quantities=tuple(quantities),
        unresolved=unresolved,
        unavailable=tuple(blocked),
        non_deterministic=non_deterministic_digest(),
        counts=inventory.counts(),
    )


# The fields every bundle entry carries, in the identity scheme of design Section 4.3.
_BUNDLE_FIELDS: tuple[str, ...] = (
    "criterion",
    "requirement",
    "requirement_source",
    "fixture",
    "milestone",
    "status",
    "identity",
    "test_or_fixture",
    "procedure",
    "expected",
    "observed",
    "receipts",
)

_BUNDLE_STATUSES: frozenset[str] = frozenset({"passed", "failed", "blocked", "superseded"})


@dataclass(frozen=True)
class BundleFinding:
    """
    One problem a bundle verification found.

    Parameters
    ----------
    criterion : int
        The criterion the entry claims, or zero for a problem with the document itself.
    field : str
        The field the problem is in.
    problem : str
        What is wrong.

    """

    criterion: int
    field: str
    problem: str

    def describe(self) -> str:
        """
        Return the finding in the vocabulary the coding standards require.

        Returns
        -------
        str

        """
        where = "the document" if not self.criterion else f"criterion {self.criterion}"

        return f"{where}, {self.field}: {self.problem}"


@dataclass(frozen=True)
class BundleVerification:
    """
    The outcome of verifying an acceptance-evidence bundle.

    Parameters
    ----------
    entries : int
        How many entries the bundle holds.
    findings : tuple[BundleFinding, ...]
        What is wrong, empty when the bundle holds.
    resolved : tuple[str, ...]
        The artifact references whose digests the caller resolved.
    named_only : tuple[int, ...]
        The criteria whose identity is held as prose rather than as components, which are named
        rather than recomputed until the bundle is completed at M6.5.

    """

    entries: int
    findings: tuple[BundleFinding, ...] = ()
    resolved: tuple[str, ...] = ()
    named_only: tuple[int, ...] = ()

    @property
    def ok(self) -> bool:
        """
        Whether the bundle holds.
        """
        return not self.findings

    def require(self) -> None:
        """
        Raise unless the bundle holds.

        Raises
        ------
        ValueError
            If the bundle does not hold, naming every finding.

        """
        if self.findings:
            raise ValueError(
                "the acceptance bundle does not hold: "
                + "; ".join(finding.describe() for finding in self.findings),
            )


def verify_bundle(
    document: Mapping[str, object],
    *,
    resolve: Mapping[str, str] | None = None,
) -> BundleVerification:
    """
    Verify an acceptance-evidence bundle by recomputation rather than by inspection.

    The check does three things a reader would otherwise do by hand. It validates the document's
    structure: the closed field set, every criterion once, a status from the closed vocabulary, and
    a block that names its dependency or a failure that names its defect. It recomputes each
    entry's identity digest from the components the entry declares, so an entry cannot claim an
    identity it does not hold. And it verifies each entry's receipts: the run receipt's digest is
    recomputed from its own fields, and every artifact the entry cites either resolves against the
    resolver the caller supplies or is reported as unresolved.

    Parameters
    ----------
    document : Mapping[str, object]
        The bundle, as it was read from its file.
    resolve : Mapping[str, str] | None, default None
        A mapping from a retained artifact's reference to the digest it was read at, or None when
        the caller holds no retained artifacts. A reference that is absent from the mapping is
        unresolved, which is how a lost artifact is reported rather than assumed.

    Returns
    -------
    BundleVerification

    """
    findings: list[BundleFinding] = []
    resolved: list[str] = []

    if document.get("schema") != BUNDLE_SCHEMA:
        found = document.get("schema")
        findings.append(BundleFinding(0, "schema", f"expected {BUNDLE_SCHEMA!r}, found {found!r}"))

    entries = document.get("entries")
    if not isinstance(entries, list):
        findings.append(BundleFinding(0, "entries", "the bundle holds no entry sequence"))

        return BundleVerification(entries=0, findings=tuple(findings))

    named_only: list[int] = []
    seen: set[int] = set()
    for entry in entries:
        if not isinstance(entry, dict):
            findings.append(BundleFinding(0, "entries", "an entry is not a mapping"))
            continue

        try:
            criterion = int(str(entry.get("criterion")))
        except ValueError:
            findings.append(
                BundleFinding(0, "criterion", f"an entry names {entry.get('criterion')!r}")
            )
            continue

        if criterion in seen:
            findings.append(BundleFinding(criterion, "criterion", "the criterion appears twice"))
        seen.add(criterion)
        if isinstance(entry.get("identity"), str):
            named_only.append(criterion)

        findings.extend(_verify_entry(criterion, entry))
        findings.extend(_verify_identity(criterion, entry.get("identity")))
        findings.extend(_verify_receipts(criterion, entry.get("receipts"), resolve, resolved))

    return BundleVerification(
        entries=len(entries),
        findings=tuple(findings),
        resolved=tuple(resolved),
        named_only=tuple(named_only),
    )


def _verify_entry(criterion: int, entry: Mapping[str, object]) -> list[BundleFinding]:
    """
    Verify one entry's declared fields, status and block or defect.
    """
    findings: list[BundleFinding] = []

    missing = [name for name in _BUNDLE_FIELDS if name not in entry]
    if missing:
        findings.append(BundleFinding(criterion, "fields", f"missing {sorted(missing)}"))

    status = str(entry.get("status", ""))
    if status not in _BUNDLE_STATUSES:
        findings.append(BundleFinding(criterion, "status", f"{status!r} is not a status"))
    if status == "blocked" and not str(entry.get("dependency", "")).strip():
        findings.append(BundleFinding(criterion, "dependency", "a blocked entry names none"))
    if status == "failed" and not str(entry.get("defect", "")).strip():
        findings.append(BundleFinding(criterion, "defect", "a failed entry names none"))

    return findings


def bundle_identity(components: Mapping[str, object]) -> str:
    """
    Return the digest an entry's identity components compose to.

    Parameters
    ----------
    components : Mapping[str, object]
        The declared identity components: the fixture, the test module, the engine build and the
        data snapshot.

    Returns
    -------
    str
        The `sha256:<hex>` digest.

    """
    return digest_of(canonical_mapping(components))


def _verify_identity(criterion: int, identity: object) -> list[BundleFinding]:
    """
    Recompute one entry's identity digest from the components it declares.

    An identity held as prose is a named identity rather than a recomputable one: it is accepted
    here and reported, because the entries the earlier milestones produced carry their components as
    a sentence until the bundle is completed at M6.5.
    """
    if identity is None or isinstance(identity, str):
        return []
    if not isinstance(identity, dict):
        return [BundleFinding(criterion, "identity", "an identity must be a mapping")]

    digest = str(identity.get("digest", ""))
    components = identity.get("components")
    if not digest:
        return [BundleFinding(criterion, "identity.digest", "the identity declares no digest")]
    if not isinstance(components, dict) or not components:
        return [
            BundleFinding(criterion, "identity.components", "the identity declares no components")
        ]

    recomputed = bundle_identity(components)
    if recomputed != digest:
        return [
            BundleFinding(
                criterion,
                "identity.digest",
                f"the components digest to {recomputed} and the entry records {digest}",
            ),
        ]

    return []


def _verify_receipts(
    criterion: int,
    receipts: object,
    resolve: Mapping[str, str] | None,
    resolved: list[str],
) -> list[BundleFinding]:
    """
    Verify one entry's receipts: the run receipt's digest, and every artifact it cites.
    """
    if receipts is None or isinstance(receipts, str):
        return []
    if not isinstance(receipts, dict):
        return [BundleFinding(criterion, "receipts", "receipts must be a mapping")]

    for name in ("engine_build", "policy_digest"):
        if not str(receipts.get(name, "")).strip():
            return [BundleFinding(criterion, f"receipts.{name}", "the entry declares none")]

    findings: list[BundleFinding] = []
    run = receipts.get("run")
    if isinstance(run, dict):
        findings.extend(_verify_run_receipt(criterion, run))

    artifacts = receipts.get("artifacts", [])
    if not isinstance(artifacts, list):
        return [*findings, BundleFinding(criterion, "receipts.artifacts", "must be a sequence")]

    findings.extend(_verify_artifacts(criterion, artifacts, resolve, resolved))

    return findings


def _verify_run_receipt(criterion: int, run: Mapping[str, object]) -> list[BundleFinding]:
    """
    Recompute the digest of one entry's run receipt from its own fields.
    """
    recorded = str(run.get("digest", ""))
    recomputed = RunReceipt(
        run_id=str(run.get("run_id", "")),
        manifest_digest=str(run.get("manifest_digest", "")),
        build_identity=str(run.get("build_identity", "")),
        configuration_digest=str(run.get("configuration_digest", "")),
        seed=int(str(run.get("seed", 0))),
        artifact_reference=str(run.get("artifact_reference", "")),
        artifact_digest=str(run.get("artifact_digest", "")),
    ).digest()

    if recomputed == recorded:
        return []

    return [
        BundleFinding(
            criterion,
            "receipts.run.digest",
            f"the receipt digests to {recomputed} and the entry records {recorded}",
        ),
    ]


def _verify_artifacts(
    criterion: int,
    artifacts: Sequence[object],
    resolve: Mapping[str, str] | None,
    resolved: list[str],
) -> list[BundleFinding]:
    """
    Verify the artifacts an entry cites, resolving each that the caller retains.
    """
    findings: list[BundleFinding] = []

    for artifact in artifacts:
        if not isinstance(artifact, dict):
            findings.append(
                BundleFinding(criterion, "receipts.artifacts", "an artifact is not a mapping"),
            )
            continue

        reference = str(artifact.get("reference", ""))
        digest = str(artifact.get("digest", ""))
        if not reference or not digest:
            findings.append(
                BundleFinding(
                    criterion,
                    "receipts.artifacts",
                    "an artifact names no reference or no digest",
                ),
            )
            continue
        if resolve is None:
            continue

        retained = resolve.get(reference)
        if retained != digest:
            findings.append(
                BundleFinding(
                    criterion,
                    f"receipts.artifacts[{reference}]",
                    f"the retained content is {retained!r} and the entry records {digest}",
                ),
            )
        else:
            resolved.append(reference)

    return findings
