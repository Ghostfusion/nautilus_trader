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

Two things this module deliberately does not do yet, because they belong to the governance
milestone rather than to the executor: the idempotency and reconciliation protocol that makes a
repeated or interrupted registration resolve to one entry, and the hash-chained inventory over the
entries. What it does do is refuse to overwrite an entry, which is the property the rest of the
protocol is built on.
"""

from __future__ import annotations

import json
import math
from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING

from nautilus_trader.discovery.schema import canonical_mapping
from nautilus_trader.optimization.space import digest_of


if TYPE_CHECKING:
    from collections.abc import Mapping
    from collections.abc import Sequence

    from nautilus_trader.discovery.compile import CompiledPlan
    from nautilus_trader.optimization.runner import CanonicalRun


# The identity of the ledger format, which a reader uses to know how to read an entry.
LEDGER_SCHEMA = "discovery.ledger/v1"

_ENTRIES_DIR = "entries"


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
