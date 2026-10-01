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
Experiment and result persistence, keyed by digest.

A store is a directory the caller owns. Under it, experiments, canonical results, failures, and a
sweep manifest are written as strict JSON, each keyed by a digest, so a sweep can be reloaded
without rerunning it. Non-finite metric values are written as the strings `"nan"`, `"inf"`, and
`"-inf"` because strict JSON has no non-finite numbers; they decode back to floats.
"""

from __future__ import annotations

import json
import math
from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING
from typing import cast

from nautilus_trader.optimization.report import ExperimentResult
from nautilus_trader.optimization.report import SearchReport
from nautilus_trader.optimization.run import ValidationScheme
from nautilus_trader.optimization.runner import CanonicalRun
from nautilus_trader.optimization.runner import FailedExperiment
from nautilus_trader.optimization.runner import RunOutcome
from nautilus_trader.optimization.space import Experiment
from nautilus_trader.optimization.space import digest_of


if TYPE_CHECKING:
    from collections.abc import Mapping

    from nautilus_trader.optimization.space import JsonValue


SCHEMA = "nautilus-optimization-store/v1"
MANIFEST_NAME = "report.json"

_EXPERIMENTS_DIR = "experiments"
_RESULTS_DIR = "results"
_FAILURES_DIR = "failures"
_CANONICAL_DIR = "canonical"


def _key(digest: str) -> str:
    """
    Return a filesystem-safe key for a digest, replacing the algorithm separator.
    """
    return digest.replace(":", "_")


def _write_json(path: Path, payload: object) -> None:
    """
    Write the payload as strict, canonical, machine-readable JSON.
    """
    text = json.dumps(payload, sort_keys=True, indent=2, ensure_ascii=True, allow_nan=False)
    path.write_text(f"{text}\n", encoding="utf-8")


def _encode_metric(value: float) -> float | str:
    """
    Encode a metric value, mapping non-finite floats to their string form.
    """
    if math.isfinite(value):
        return value
    return str(value)


def _decode_metric(value: float | str) -> float:
    """
    Decode a metric value, mapping the string forms back to floats.
    """
    return value if isinstance(value, int | float) else float(value)


class ExperimentStore:
    """
    A digest-keyed, machine-readable store of experiments and results.

    Parameters
    ----------
    directory : str or pathlib.Path
        The directory the store owns. It is created if it does not exist.

    """

    def __init__(self, directory: str | Path) -> None:
        """
        Initialize the instance.
        """
        self._directory = Path(directory)
        for name in (_EXPERIMENTS_DIR, _RESULTS_DIR, _FAILURES_DIR, _CANONICAL_DIR):
            (self._directory / name).mkdir(parents=True, exist_ok=True)

    @property
    def directory(self) -> Path:
        """
        The directory the store owns.
        """
        return self._directory

    def write_experiment(self, experiment: Experiment) -> Path:
        """
        Write the experiment keyed by its digest.

        Parameters
        ----------
        experiment : Experiment
            The experiment to write.

        Returns
        -------
        pathlib.Path

        """
        path = self._directory / _EXPERIMENTS_DIR / f"{_key(experiment.digest)}.json"
        _write_json(path, {"digest": experiment.digest, "parameters": dict(experiment.parameters)})
        return path

    def write_result(self, result: ExperimentResult) -> Path:
        """
        Write the result and its canonical document, keyed by digest.

        Parameters
        ----------
        result : ExperimentResult
            The result to write.

        Returns
        -------
        pathlib.Path

        """
        run = result.run
        key = _key(run.canonical_digest)
        (self._directory / _CANONICAL_DIR / f"{key}.json").write_bytes(run.canonical_result)
        path = self._directory / _RESULTS_DIR / f"{key}.json"
        _write_json(path, self._encode_result(result))
        return path

    def write_failure(self, failure: FailedExperiment) -> Path:
        """
        Write the failure keyed by experiment digest.

        Parameters
        ----------
        failure : FailedExperiment
            The failure to write.

        Returns
        -------
        pathlib.Path

        """
        path = self._directory / _FAILURES_DIR / f"{_key(failure.experiment.digest)}.json"
        _write_json(
            path,
            {
                "schema": SCHEMA,
                "experiment": {
                    "digest": failure.experiment.digest,
                    "parameters": dict(failure.experiment.parameters),
                },
                "error_type": failure.error_type,
                "error_message": failure.error_message,
            },
        )
        return path

    def write_report(self, report: SearchReport) -> Path:
        """
        Write a whole sweep: every result, every failure, and an ordered manifest.

        Parameters
        ----------
        report : SearchReport
            The report to write.

        Returns
        -------
        pathlib.Path

        """
        for result in report.results:
            self.write_experiment(result.experiment)
            self.write_result(result)
        for failure in report.failures:
            self.write_experiment(failure.experiment)
            self.write_failure(failure)

        manifest = {
            "schema": SCHEMA,
            "results": [result.run.canonical_digest for result in report.results],
            "failures": [failure.experiment.digest for failure in report.failures],
            "evaluated": report.evaluated,
            "space_size": report.space_size,
            "executions": report.executions,
            "seed": report.seed,
            "scheme": None if report.scheme is None else report.scheme.to_dict(),
        }
        path = self._directory / MANIFEST_NAME
        _write_json(path, manifest)
        return path

    def load_experiments(self) -> list[Experiment]:
        """
        Load every stored experiment.

        Returns
        -------
        list[Experiment]

        """
        directory = self._directory / _EXPERIMENTS_DIR
        return [
            Experiment(self._read(path)["parameters"]) for path in sorted(directory.glob("*.json"))
        ]

    def load_results(self) -> list[ExperimentResult]:
        """
        Load every stored result.

        Returns
        -------
        list[ExperimentResult]

        """
        directory = self._directory / _RESULTS_DIR
        return [self._decode_result(self._read(path)) for path in sorted(directory.glob("*.json"))]

    def load_report(self) -> SearchReport:
        """
        Load the stored sweep manifest and its results and failures.

        Returns
        -------
        SearchReport

        """
        manifest = self._read(self._directory / MANIFEST_NAME)
        results = tuple(self._load_result(digest) for digest in manifest["results"])
        failures = tuple(self._load_failure(digest) for digest in manifest["failures"])
        scheme = manifest.get("scheme")
        return SearchReport(
            results,
            failures,
            evaluated=manifest.get("evaluated", len(results) + len(failures)),
            space_size=manifest.get("space_size"),
            executions=manifest.get("executions", 0),
            scheme=None if scheme is None else ValidationScheme.from_dict(scheme),
            seed=manifest.get("seed"),
        )

    def load_failures(self) -> list[FailedExperiment]:
        """
        Load every stored failure.

        Returns
        -------
        list[FailedExperiment]

        """
        directory = self._directory / _FAILURES_DIR
        return [self._decode_failure(self._read(path)) for path in sorted(directory.glob("*.json"))]

    def canonical_bytes(self, canonical_digest: str) -> bytes:
        """
        Return the stored canonical document for a canonical digest.

        Parameters
        ----------
        canonical_digest : str
            The canonical backtest result digest.

        Returns
        -------
        bytes

        """
        path = self._directory / _CANONICAL_DIR / f"{_key(canonical_digest)}.json"
        return path.read_bytes()

    def _read(self, path: Path) -> dict:
        """
        Read a stored JSON document.
        """
        return json.loads(path.read_text(encoding="utf-8"))

    def _encode_result(self, result: ExperimentResult) -> dict:
        """
        Encode one result record for storage.
        """
        run = result.run
        return {
            "schema": SCHEMA,
            "experiment": {
                "digest": run.experiment.digest,
                "parameters": dict(run.experiment.parameters),
            },
            "canonical_digest": run.canonical_digest,
            "score": _encode_metric(result.score),
            "constraints_satisfied": result.constraints_satisfied,
            "metric_values": {
                name: _encode_metric(value) for name, value in run.metric_values.items()
            },
        }

    def _decode_result(self, payload: dict) -> ExperimentResult:
        """
        Decode one stored result record.
        """
        experiment = Experiment(payload["experiment"]["parameters"])
        canonical_digest = payload["canonical_digest"]
        run = CanonicalRun(
            experiment,
            canonical_digest,
            {name: _decode_metric(value) for name, value in payload["metric_values"].items()},
            self.canonical_bytes(canonical_digest),
        )
        return ExperimentResult(
            run,
            _decode_metric(payload["score"]),
            bool(payload["constraints_satisfied"]),
        )

    def _load_result(self, canonical_digest: str) -> ExperimentResult:
        """
        Load one result by canonical digest.
        """
        path = self._directory / _RESULTS_DIR / f"{_key(canonical_digest)}.json"
        return self._decode_result(self._read(path))

    def _load_failure(self, experiment_digest: str) -> FailedExperiment:
        """
        Load one failure by experiment digest.
        """
        path = self._directory / _FAILURES_DIR / f"{_key(experiment_digest)}.json"
        return self._decode_failure(self._read(path))

    @staticmethod
    def _decode_failure(payload: dict) -> FailedExperiment:
        """
        Decode one stored failure record.
        """
        return FailedExperiment(
            Experiment(payload["experiment"]["parameters"]),
            payload["error_type"],
            payload["error_message"],
        )


@dataclass(frozen=True)
class Evaluation:
    """
    One evaluated experiment: its outcome, its score and its feasibility.

    A failure carries no score, so its score is NaN and it is never feasible. Feasibility is a
    separate attribute from the score, so ranking can put every feasible evaluation above every
    infeasible one without pretending an infeasible score is merely low.

    Parameters
    ----------
    outcome : RunOutcome
        The canonical run or the typed failure.
    score : float
        The objective score, or NaN when there is none.
    feasible : bool
        Whether every constraint holds for the run's metric values.

    """

    outcome: RunOutcome
    score: float
    feasible: bool

    @property
    def experiment(self) -> Experiment:
        """
        The experiment's parameter set.
        """
        return self.outcome.experiment

    @property
    def digest(self) -> str:
        """
        The experiment's parameter digest, the memo key.
        """
        return self.outcome.experiment.digest


class EvaluationCache:
    """
    A memo of evaluations keyed by the canonical parameter digest.

    The cache is the record a resumed run consults before executing an experiment: a digest already
    present is reused, so a second run over the same store lowers its execution count to zero. When
    a store is given, every recorded evaluation is written through to it and the cache is loaded
    from it on construction.

    Parameters
    ----------
    store : ExperimentStore | None, default None
        The persistence store to load from and write through to, or None for an in-memory cache.

    """

    def __init__(self, store: ExperimentStore | None = None) -> None:
        """
        Initialize the instance, loading any evaluations the store already holds.
        """
        self._store = store
        self._entries: dict[str, Evaluation] = {}
        if store is not None:
            self._load()

    @property
    def store(self) -> ExperimentStore | None:
        """
        The store the cache writes through to, or None when it is in memory only.
        """
        return self._store

    def __len__(self) -> int:
        """
        Return the number of distinct evaluations the cache holds.
        """
        return len(self._entries)

    def __contains__(self, digest: str) -> bool:
        """
        Whether the cache holds the given parameter digest.
        """
        return digest in self._entries

    def get(self, digest: str) -> Evaluation | None:
        """
        Return the evaluation recorded for a parameter digest, or None when it has none.

        Parameters
        ----------
        digest : str
            The canonical parameter digest.

        Returns
        -------
        Evaluation | None

        """
        return self._entries.get(digest)

    @property
    def evaluations(self) -> tuple[Evaluation, ...]:
        """
        Return every evaluation, in parameter-digest order.

        Returns
        -------
        tuple[Evaluation, ...]

        """
        return tuple(self._entries[digest] for digest in sorted(self._entries))

    def put(self, evaluation: Evaluation) -> None:
        """
        Record an evaluation, writing it through to the store when one is declared.

        Parameters
        ----------
        evaluation : Evaluation
            The evaluation to record.

        """
        self._entries[evaluation.digest] = evaluation
        if self._store is None:
            return
        self._store.write_experiment(evaluation.experiment)
        if isinstance(evaluation.outcome, CanonicalRun):
            self._store.write_result(
                ExperimentResult(evaluation.outcome, evaluation.score, evaluation.feasible),
            )
        else:
            self._store.write_failure(evaluation.outcome)

    @property
    def digest(self) -> str:
        """
        The `sha256:<hex>` digest of the evaluations the cache holds.

        Non-finite scores are encoded as their string form so the digest stays strict JSON, exactly
        as the store encodes them.
        """
        payload = {
            digest: {
                "score": _encode_metric(entry.score),
                "feasible": entry.feasible,
            }
            for digest, entry in sorted(self._entries.items())
        }
        return digest_of(cast("Mapping[str, JsonValue]", payload))

    def _load(self) -> None:
        """
        Load the store's results and failures into the digest-keyed cache.
        """
        store = self._store
        if store is None:
            return
        for result in store.load_results():
            self._entries[result.experiment.digest] = Evaluation(
                result.run,
                result.score,
                result.constraints_satisfied,
            )
        for failure in store.load_failures():
            self._entries[failure.experiment.digest] = Evaluation(failure, math.nan, feasible=False)
