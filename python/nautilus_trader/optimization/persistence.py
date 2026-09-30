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
from pathlib import Path

from nautilus_trader.optimization.report import ExperimentResult
from nautilus_trader.optimization.report import SearchReport
from nautilus_trader.optimization.runner import CanonicalRun
from nautilus_trader.optimization.runner import FailedExperiment
from nautilus_trader.optimization.space import Experiment


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
        return SearchReport(results, failures)

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
        payload = self._read(path)
        return FailedExperiment(
            Experiment(payload["experiment"]["parameters"]),
            payload["error_type"],
            payload["error_message"],
        )
