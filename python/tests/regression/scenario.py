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
Declared regression scenarios with their three verification layers.

A scenario declares three complementary layers of expected behaviour:

1. The expected canonical digest of the projected backtest state.
2. The expected values of declared statistics, addressed as canonical document paths.
3. The expected values of semantic checkpoints, addressed by record kind, instrument, and
   occurrence ordinal, so a checkpoint survives unrelated record insertions.

Expectations are committed under `expected/{scenario name}.json` and are rewritten for every
declared scenario in one command with `--regenerate-regression`, or with the
`NAUTILUS_REGRESSION_REGENERATE` environment variable set.
"""

from __future__ import annotations

import gc
import json
from collections.abc import Callable
from collections.abc import Iterator
from contextlib import contextmanager
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from nautilus_trader.backtest import BacktestEngine


EXPECTATIONS_SCHEMA = "nautilus-regression-expectations/v1"
REPORT_SCHEMA = "nautilus-regression-report/v1"
EXPECTATIONS_DIR = Path(__file__).resolve().parent / "expected"

LAYER_DIGEST = "digest"
LAYER_STATISTICS = "statistics"
LAYER_CHECKPOINTS = "checkpoints"
LAYER_ROWS = "rows"
LAYER_CHECKS = "checks"

DEFAULT_STATISTICS = (
    "run.outcome",
    "run.iterations",
    "run.total_events",
    "run.total_orders",
    "run.total_positions",
    "count:accounts",
    "count:fills",
    "count:orders",
    "count:position_snapshots",
    "count:positions",
)

_MISSING = object()


def _variant_payload(value: Any) -> Any:
    """
    Unwrap a single-variant record such as `{"Market": {...}}` to its payload.
    """
    if isinstance(value, dict) and len(value) == 1:
        return next(iter(value.values()))
    return value


def _order_instrument_id(record: dict[str, Any]) -> Any:
    payload = _variant_payload(record)
    core = payload.get("core") if isinstance(payload, dict) else None
    return core.get("instrument_id") if isinstance(core, dict) else None


def _fill_instrument_id(record: dict[str, Any]) -> Any:
    payload = _variant_payload(record.get("event"))
    return payload.get("instrument_id") if isinstance(payload, dict) else None


def _position_instrument_id(record: dict[str, Any]) -> Any:
    return record.get("instrument_id")


# Canonical document array, and how to read the instrument identifier from one record. Orders and
# fills are externally tagged by order type and event variant, so their identifiers are read from
# the single variant payload.
_CHECKPOINT_SOURCES: dict[str, tuple[str, Callable[[dict[str, Any]], Any]]] = {
    "order": ("orders", _order_instrument_id),
    "fill": ("fills", _fill_instrument_id),
    "position": ("positions", _position_instrument_id),
    "position_snapshot": ("position_snapshots", _position_instrument_id),
}


@dataclass(frozen=True)
class Checkpoint:
    """
    A semantic checkpoint addressed by record kind, instrument, and occurrence ordinal.

    A checkpoint is semantic rather than positional, so it survives unrelated record insertions.
    The expected value is the whole canonical record at that ordinal, which keeps the declaration
    free of hand-written field lists.
    """

    kind: str
    instrument_id: str | None = None
    ordinal: int = 0

    def __post_init__(self) -> None:
        """
        Validate the checkpoint declaration.
        """
        if self.kind not in _CHECKPOINT_SOURCES:
            raise ValueError(
                f"Unknown checkpoint kind {self.kind!r}, "
                f"expected one of {sorted(_CHECKPOINT_SOURCES)}",
            )
        if self.ordinal < 0:
            raise ValueError("Checkpoint ordinal must not be negative")

    @property
    def key(self) -> str:
        """
        The stable key of this checkpoint, as `kind[instrument]#ordinal`.
        """
        instrument = self.instrument_id or "*"
        return f"{self.kind}[{instrument}]#{self.ordinal}"

    def select(self, document: dict[str, Any]) -> Any:
        """
        Select the value of this checkpoint from a canonical result document.
        """
        array_key, instrument_of = _CHECKPOINT_SOURCES[self.kind]
        records = document.get(array_key)
        if not isinstance(records, list):
            raise TypeError(f"Canonical document field {array_key!r} is not an array")
        if self.instrument_id is not None:
            records = [record for record in records if instrument_of(record) == self.instrument_id]
        if self.ordinal >= len(records):
            raise ValueError(
                f"Checkpoint {self.key} is out of range: "
                f"{len(records)} matching record(s) in {array_key!r}",
            )
        return records[self.ordinal]


def select_statistic(document: dict[str, Any], selector: str) -> Any:
    """
    Resolve a declared statistic from a canonical result document.

    Selectors are either `count:<array>` for the length of a document array, or a dotted path
    into the document.
    """
    if selector.startswith("count:"):
        array_key = selector.removeprefix("count:")
        records = document.get(array_key)
        if not isinstance(records, list):
            raise TypeError(f"Canonical document field {array_key!r} is not an array")
        return len(records)
    return read_path(document, selector)


def read_path(value: Any, path: str) -> Any:
    """
    Read a dotted path from a decoded canonical document.
    """
    current = value
    for part in path.split("."):
        if isinstance(current, dict) and part in current:
            current = current[part]
        elif isinstance(current, list) and part.isdigit() and int(part) < len(current):
            current = current[int(part)]
        else:
            raise ValueError(f"Path {path!r} is not present in the canonical document")
    return current


@dataclass(frozen=True)
class Scenario:
    """
    A declared regression scenario.
    """

    name: str
    execute: Callable[[], Any]
    statistics: tuple[str, ...] = DEFAULT_STATISTICS
    checkpoints: tuple[Checkpoint, ...] = ()

    @property
    def path(self) -> Path:
        """
        The committed expectations path of this scenario.
        """
        return EXPECTATIONS_DIR / f"{self.name}.json"


@dataclass(frozen=True)
class ReportScenario:
    """
    A declared regression scenario that records a report's rows rather than a canonical run.

    A row that renders only when a report is explicitly configured is absent from every canonical
    document, so the canonical layer cannot pin it. A report scenario builds those rows through the
    same public Python API a caller uses and records them under the names the report renders, so a
    change to a row's name or value is caught the same way a canonical divergence is. The declared
    checks are the behaviours the case asserts rather than rows it records, such as a refusal.
    """

    name: str
    execute: Callable[[], dict[str, Any]]

    @property
    def path(self) -> Path:
        """
        The committed expectations path of this scenario.
        """
        return EXPECTATIONS_DIR / f"{self.name}.json"


@dataclass(frozen=True)
class ScenarioOutcome:
    """
    The observed layers of one scenario run.
    """

    canonical: Any
    document: dict[str, Any]
    digest: str
    statistics: dict[str, Any]
    checkpoints: dict[str, Any]


def run_scenario(scenario: Scenario) -> ScenarioOutcome:
    """
    Run a scenario and collect its three observed layers.
    """
    canonical = scenario.execute()
    raw = canonical.to_bytes()
    document = json.loads(raw.decode("utf-8"))
    return ScenarioOutcome(
        canonical=canonical,
        document=document,
        digest=canonical.digest(),
        statistics={
            selector: select_statistic(document, selector) for selector in scenario.statistics
        },
        checkpoints={
            checkpoint.key: checkpoint.select(document) for checkpoint in scenario.checkpoints
        },
    )


def load_expectations(scenario: Scenario) -> dict[str, Any]:
    """
    Load the committed expectations of a scenario.
    """
    path = scenario.path
    if not path.exists():
        raise FileNotFoundError(
            f"Missing regression expectations: {path}. Regenerate them with "
            f"`make pytest-regression NAUTILUS_REGRESSION_REGENERATE=1`.",
        )
    with path.open(encoding="utf-8") as file:
        payload = json.load(file)
    if payload.get("schema") != EXPECTATIONS_SCHEMA:
        raise ValueError(
            f"Unsupported expectations schema in {path}: {payload.get('schema')!r}",
        )
    return payload


def write_expectations(scenario: Scenario, outcome: ScenarioOutcome) -> Path:
    """
    Rewrite all three expectation layers of a scenario.
    """
    path = scenario.path
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "schema": EXPECTATIONS_SCHEMA,
        "scenario": scenario.name,
        "digest": outcome.digest,
        "statistics": outcome.statistics,
        "checkpoints": outcome.checkpoints,
        "canonical": outcome.canonical.to_bytes().decode("utf-8"),
    }
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return path


def verify_scenario(
    scenario: Scenario,
    outcome: ScenarioOutcome,
    expected: dict[str, Any],
) -> None:
    """
    Verify all three layers of a scenario and report every failing layer.

    A digest mismatch is reported with the first divergence of the canonical document, so a
    failure names the differing value rather than only a digest.
    """
    lines: list[str] = []

    if expected.get("digest") != outcome.digest:
        lines.extend(_digest_failure(scenario, outcome, expected))

    expected_statistics = expected.get("statistics", {})
    for selector, value in outcome.statistics.items():
        expected_value = expected_statistics.get(selector, _MISSING)
        if expected_value is _MISSING:
            lines.append(f"[{LAYER_STATISTICS}] {selector}: missing from the declared expectations")
        elif expected_value != value:
            lines.append(
                f"[{LAYER_STATISTICS}] {selector}: "
                f"expected {_format(expected_value)}, actual {_format(value)}",
            )

    expected_checkpoints = expected.get("checkpoints", {})
    for key, value in outcome.checkpoints.items():
        expected_value = expected_checkpoints.get(key, _MISSING)
        if expected_value is _MISSING:
            lines.append(f"[{LAYER_CHECKPOINTS}] {key}: missing from the declared expectations")
        elif expected_value != value:
            lines.append(
                f"[{LAYER_CHECKPOINTS}] {key}: "
                f"expected {_format(expected_value)}, actual {_format(value)}",
            )

    if lines:
        report = "\n".join(f"  {line}" for line in lines)
        raise AssertionError(
            f"Regression scenario {scenario.name!r} diverged:\n{report}\n"
            f"  Regenerate with `--regenerate-regression` only if the divergence is intended.",
        )


def run_report_scenario(scenario: ReportScenario) -> dict[str, Any]:
    """
    Build the report document of a scenario.
    """
    document = scenario.execute()
    if not isinstance(document, dict) or "rows" not in document or "checks" not in document:
        raise TypeError(
            f"Report scenario {scenario.name!r} must return a mapping with 'rows' and 'checks'",
        )
    return document


def load_report_expectations(scenario: ReportScenario) -> dict[str, Any]:
    """
    Load the committed report expectations of a scenario.
    """
    path = scenario.path
    if not path.exists():
        raise FileNotFoundError(
            f"Missing regression report: {path}. Regenerate it with `--regenerate-regression`.",
        )
    with path.open(encoding="utf-8") as file:
        payload = json.load(file)
    if payload.get("schema") != REPORT_SCHEMA:
        raise ValueError(
            f"Unsupported report schema in {path}: {payload.get('schema')!r}",
        )
    return payload


def write_report_expectations(scenario: ReportScenario, document: dict[str, Any]) -> Path:
    """
    Rewrite the recorded rows and checks of a report scenario.
    """
    path = scenario.path
    path.parent.mkdir(parents=True, exist_ok=True)
    payload = {
        "schema": REPORT_SCHEMA,
        "scenario": scenario.name,
        "rows": document["rows"],
        "checks": document["checks"],
    }
    path.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    return path


def verify_report_scenario(
    scenario: ReportScenario,
    document: dict[str, Any],
    expected: dict[str, Any],
) -> None:
    """
    Verify the rows and checks of a report scenario and report every difference.

    A row that has disappeared from the report is reported as well as one whose value moved, so a
    rename is caught rather than read as a silently dropped row.
    """
    lines: list[str] = []

    expected_rows = expected.get("rows", {})
    actual_rows = document["rows"]
    for name, value in actual_rows.items():
        expected_value = expected_rows.get(name, _MISSING)
        if expected_value is _MISSING:
            lines.append(f"[{LAYER_ROWS}] {name}: missing from the declared expectations")
        elif expected_value != value:
            lines.append(
                f"[{LAYER_ROWS}] {name}: "
                f"expected {_format(expected_value)}, actual {_format(value)}",
            )
    lines.extend(
        f"[{LAYER_ROWS}] {name}: declared row is no longer rendered"
        for name in expected_rows
        if name not in actual_rows
    )

    expected_checks = expected.get("checks", [])
    if list(document["checks"]) != list(expected_checks):
        lines.append(
            f"[{LAYER_CHECKS}] expected {_format(expected_checks)}, "
            f"actual {_format(document['checks'])}",
        )

    if lines:
        report = "\n".join(f"  {line}" for line in lines)
        raise AssertionError(
            f"Regression report {scenario.name!r} diverged:\n{report}\n"
            f"  Regenerate with `--regenerate-regression` only if the divergence is intended.",
        )


def clean_baseline() -> None:
    """
    Return process-level state to a clean baseline between scenarios.

    Scenarios create and dispose their own engine, cache, and clock, and the suite-wide
    `bypass_logging` fixture supplies the logger baseline, so the remaining state to restore is the
    reference cycles of the disposed engines.
    """
    gc.collect()


@contextmanager
def backtest_engine(config: Any) -> Iterator[BacktestEngine]:
    """
    Create a backtest engine which is always disposed.
    """
    engine = BacktestEngine(config)
    try:
        yield engine
    finally:
        engine.dispose()
        gc.collect()


def _digest_failure(
    scenario: Scenario,
    outcome: ScenarioOutcome,
    expected: dict[str, Any],
) -> list[str]:
    lines = [
        f"[{LAYER_DIGEST}] expected {expected.get('digest')}, actual {outcome.digest}",
    ]
    canonical = expected.get("canonical")
    if not isinstance(canonical, str):
        lines.append("  no recorded canonical document, so the divergence path is unavailable")
        return lines
    divergence = outcome.canonical.first_divergence(canonical.encode("utf-8"))
    if divergence is None:
        lines.append(
            "  the recorded canonical document matches, so the digest difference was not "
            "reproduced by this run",
        )
        return lines
    path, expected_value, actual_value = divergence
    lines.append(
        f"  first divergence at {path}: "
        f"expected {_render(expected_value)}, actual {_render(actual_value)}",
    )
    return lines


def _render(encoded: str | None) -> str:
    if encoded is None:
        return "absent"
    return _format(json.loads(encoded))


def _format(value: Any, limit: int = 200) -> str:
    text = json.dumps(value, sort_keys=True)
    return text if len(text) <= limit else f"{text[:limit]}..."
