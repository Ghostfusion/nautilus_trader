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
Validate the M1.2 acceptance-evidence bundle skeleton for strategy discovery.

`discovery_acceptance_bundle.json` is the skeleton of the evidence bundle that
`docs/design/strategy_discovery_implementation.md` Section 15 requires: one entry per acceptance
criterion, each carrying the eight fields of the plan's Section 4.1 with the status vocabulary of
Section 4.2. Nothing has run, so every entry is `blocked` and its result fields are null. Each
requirement sentence is quoted verbatim from Section 15 of `docs/design/strategy_discovery_design.md`
and each fixture and milestone comes from the plan's Section 15 table.

The schema is validated by `validate_bundle`, which raises rather than asserting, so the negative
cases at the end of this module construct a broken bundle in memory and show it is refused.

"""

from __future__ import annotations

import copy
import json
import re
from pathlib import Path
from typing import Any

import pytest


BUNDLE_PATH = Path(__file__).resolve().parent / "discovery_acceptance_bundle.json"
DESIGN_PATH = (
    Path(__file__).resolve().parents[3] / "docs" / "design" / "strategy_discovery_design.md"
)
PLAN_PATH = (
    Path(__file__).resolve().parents[3] / "docs" / "design" / "strategy_discovery_implementation.md"
)

BUNDLE_SCHEMA = "discovery.acceptance-bundle/v1"
CRITERION_COUNT = 22
CRITERION_NUMBERS = tuple(range(1, CRITERION_COUNT + 1))

# Design Section 4.2
STATUS_VOCABULARY = ("passed", "failed", "blocked", "superseded")

# Design Section 15.1 fixes six of these and the plan's Section 4.1 adds the requirement and the
# artifact columns; the plan's Section 15 table supplies the fixture and milestone.
SECTION_4_1_FIELDS = (
    "criterion",
    "identity",
    "test_or_fixture",
    "procedure",
    "expected",
    "observed",
    "receipts",
    "status",
)
PROVENANCE_FIELDS = (
    "requirement",
    "requirement_source",
    "fixture",
    "milestone",
)
ENTRY_FIELDS = SECTION_4_1_FIELDS + PROVENANCE_FIELDS

# The plan's Section 15 table names fixtures as `FX-M<milestone>-<n>` against one or more milestones
FIXTURE_PATTERN = re.compile(r"^FX-M\d+-\d+$")
MILESTONE_PATTERN = re.compile(r"^M\d+(, M\d+)*$")

DESIGN_SECTION_START = "## 15. How you would know it worked"
DESIGN_SECTION_END = "### 15.1 The acceptance-evidence bundle"
PLAN_TABLE_HEADING = "## 15. The acceptance-evidence bundle skeleton"

REQUIREMENT_SOURCE_DOCUMENT = "docs/design/strategy_discovery_design.md"


class BundleValidationError(ValueError):
    """
    Raised when the acceptance-evidence bundle violates its documented schema.
    """


def load_bundle(path: Path = BUNDLE_PATH) -> dict[str, Any]:
    """
    Load the acceptance-evidence bundle from `path`.
    """
    return json.loads(path.read_text(encoding="utf-8"))


def validate_bundle(bundle: dict[str, Any]) -> None:
    """
    Validate `bundle` against the documented schema, raising on the first violation.
    """
    if not isinstance(bundle, dict):
        raise BundleValidationError(f"bundle must be a JSON object, was {type(bundle).__name__}")

    if bundle.get("schema") != BUNDLE_SCHEMA:
        raise BundleValidationError(
            f"bundle schema is {bundle.get('schema')!r}, not {BUNDLE_SCHEMA!r}"
        )

    entries = bundle.get("entries")
    if not isinstance(entries, list):
        raise BundleValidationError(f"bundle entries must be a list, was {type(entries).__name__}")

    if len(entries) != CRITERION_COUNT:
        raise BundleValidationError(
            f"bundle has {len(entries)} entries, expected exactly {CRITERION_COUNT}",
        )

    for entry in entries:
        _validate_entry(entry)

    _validate_criterion_numbers(entries)


def _validate_entry(entry: dict[str, Any]) -> None:
    """
    Validate one entry's field set, requirement provenance, fixture, milestone and status.
    """
    if not isinstance(entry, dict):
        raise BundleValidationError(f"entry must be a JSON object, was {type(entry).__name__}")

    number = entry.get("criterion")
    missing = [field for field in ENTRY_FIELDS if field not in entry]
    if missing:
        raise BundleValidationError(f"entry {number!r} is missing fields: {missing}")

    unknown = [field for field in entry if field not in ENTRY_FIELDS]
    if unknown:
        raise BundleValidationError(f"entry {number!r} has fields outside the schema: {unknown}")

    requirement = entry["requirement"]
    if not isinstance(requirement, str) or not requirement.strip():
        raise BundleValidationError(f"entry {number!r} carries no requirement sentence")

    source = entry["requirement_source"]
    if not isinstance(source, str) or REQUIREMENT_SOURCE_DOCUMENT not in source:
        raise BundleValidationError(
            f"entry {number!r} does not name where its requirement comes from: {source!r}",
        )

    fixture = entry["fixture"]
    if not isinstance(fixture, str) or not FIXTURE_PATTERN.match(fixture):
        raise BundleValidationError(
            f"entry {number!r} names no fixture of the plan's scheme: {fixture!r}"
        )

    milestone = entry["milestone"]
    if not isinstance(milestone, str) or not MILESTONE_PATTERN.match(milestone):
        raise BundleValidationError(f"entry {number!r} names no milestone: {milestone!r}")

    status = entry["status"]
    if status not in STATUS_VOCABULARY:
        raise BundleValidationError(
            f"entry {number!r} has status {status!r}, not one of {STATUS_VOCABULARY}",
        )


def _validate_criterion_numbers(entries: list[dict[str, Any]]) -> None:
    """
    Validate that the criterion numbers are exactly 1 to 22, with none duplicated or missing.
    """
    numbers = [entry["criterion"] for entry in entries]
    malformed = [
        number for number in numbers if not isinstance(number, int) or isinstance(number, bool)
    ]
    if malformed:
        raise BundleValidationError(f"criterion numbers are not integers: {malformed}")

    numbers.sort()
    if numbers != list(CRITERION_NUMBERS):
        raise BundleValidationError(
            f"criterion numbers are {numbers}, not exactly {list(CRITERION_NUMBERS)}",
        )


def _design_section_15_text() -> str:
    """
    Return Section 15 of the design document with its line wrapping collapsed.
    """
    lines = DESIGN_PATH.read_text(encoding="utf-8").splitlines()
    start = lines.index(DESIGN_SECTION_START)
    end = lines.index(DESIGN_SECTION_END)
    return " ".join(" ".join(lines[start:end]).split())


def _plan_section_15_rows() -> dict[int, tuple[str, str]]:
    """
    Return the fixture and milestone of every criterion row of the plan's Section 15 table.
    """
    lines = PLAN_PATH.read_text(encoding="utf-8").splitlines()
    start = lines.index(PLAN_TABLE_HEADING)
    rows: dict[int, tuple[str, str]] = {}

    for line in lines[start:]:
        cells = [cell.strip() for cell in line.strip().strip("|").split("|")]
        if len(cells) != 5 or not cells[0].isdigit() or not cells[2].startswith("`"):
            continue

        rows[int(cells[0])] = (cells[2].strip("`"), cells[3])

    return rows


def _verbatim_violations(bundle: dict[str, Any], section: str) -> list[int]:
    """
    Return the criterion numbers whose requirement sentence is not verbatim in `section`.
    """
    return [
        entry["criterion"]
        for entry in bundle["entries"]
        if entry["requirement"] not in section
        or f"Section 15 criterion {entry['criterion']}" not in entry["requirement_source"]
    ]


def test_bundle_parses_and_validates_against_the_documented_schema() -> None:
    """
    Test the committed skeleton parses as JSON and passes schema validation.
    """
    validate_bundle(load_bundle())


def test_bundle_has_exactly_twenty_two_entries() -> None:
    """
    Test the bundle carries one entry per acceptance criterion.
    """
    entries = load_bundle()["entries"]

    assert len(entries) == CRITERION_COUNT, (
        f"bundle has {len(entries)} entries, not {CRITERION_COUNT}"
    )


def test_criterion_numbers_are_one_to_twenty_two() -> None:
    """
    Test the entry criteria are the numbers 1 to 22, in order, with none duplicated or missing.
    """
    numbers = [entry["criterion"] for entry in load_bundle()["entries"]]

    assert numbers == list(CRITERION_NUMBERS), f"criterion numbers are {numbers}"


def test_the_entry_field_set_covers_the_eight_fields_of_section_4_1() -> None:
    """
    Test the schema's field set contains all eight fields of the plan's evidence model.
    """
    assert len(SECTION_4_1_FIELDS) == 8
    assert set(SECTION_4_1_FIELDS) <= set(ENTRY_FIELDS)


def test_every_entry_has_all_eight_fields() -> None:
    """
    Test every entry carries each field of Section 4.1 alongside its provenance fields.
    """
    bundle = load_bundle()

    for entry in bundle["entries"]:
        missing = [field for field in ENTRY_FIELDS if field not in entry]
        assert missing == [], f"criterion {entry['criterion']} is missing fields {missing}"


def test_every_status_is_in_the_vocabulary_of_section_4_2() -> None:
    """
    Test every entry's status is one of passed, failed, blocked or superseded.
    """
    bundle = load_bundle()

    for entry in bundle["entries"]:
        message = f"criterion {entry['criterion']} has status {entry['status']!r}"
        assert entry["status"] in STATUS_VOCABULARY, message


def test_every_entry_is_blocked_because_nothing_has_run() -> None:
    """
    Test the skeleton claims nothing: M6.5 completes these statuses by running the fixtures.
    """
    statuses = {entry["status"] for entry in load_bundle()["entries"]}

    assert statuses == {"blocked"}, f"statuses are {sorted(statuses)}"


def test_every_entry_names_a_fixture_and_a_milestone() -> None:
    """
    Test every entry names the fixture that will produce it and the milestone that owes it.
    """
    bundle = load_bundle()

    for entry in bundle["entries"]:
        fixture_message = f"criterion {entry['criterion']} names fixture {entry['fixture']!r}"
        milestone_message = f"criterion {entry['criterion']} names milestone {entry['milestone']!r}"
        assert isinstance(entry["fixture"], str), fixture_message
        assert FIXTURE_PATTERN.match(entry["fixture"]), fixture_message
        assert isinstance(entry["milestone"], str), milestone_message
        assert MILESTONE_PATTERN.match(entry["milestone"]), milestone_message


def test_every_requirement_is_quoted_verbatim_from_design_section_15() -> None:
    """
    Test each requirement sentence and its provenance appear verbatim in the design.
    """
    bundle = load_bundle()
    violations = _verbatim_violations(bundle, _design_section_15_text())

    assert violations == [], f"criteria not quoted verbatim from the design: {violations}"


def test_fixtures_and_milestones_come_from_the_plan_table() -> None:
    """
    Test each entry's fixture and milestone match the plan's Section 15 table row.
    """
    plan = _plan_section_15_rows()
    bundle = load_bundle()

    assert sorted(plan) == list(CRITERION_NUMBERS), f"the plan table lists criteria {sorted(plan)}"

    for entry in bundle["entries"]:
        expected = plan[entry["criterion"]]
        observed = (entry["fixture"], entry["milestone"])
        assert observed == expected, (
            f"criterion {entry['criterion']} has {observed}, not {expected}"
        )


def test_a_bundle_missing_an_entry_is_refused() -> None:
    """
    Test a bundle with one entry fewer than the design's twenty-two fails validation.
    """
    bundle = copy.deepcopy(load_bundle())
    bundle["entries"].pop()

    with pytest.raises(BundleValidationError, match="entries"):
        validate_bundle(bundle)


def test_a_duplicate_criterion_number_is_refused() -> None:
    """
    Test a bundle that repeats a criterion number and drops another fails validation.
    """
    bundle = copy.deepcopy(load_bundle())
    bundle["entries"][1]["criterion"] = bundle["entries"][0]["criterion"]

    with pytest.raises(BundleValidationError, match="criterion numbers"):
        validate_bundle(bundle)


def test_a_status_outside_the_vocabulary_is_refused() -> None:
    """
    Test a bundle whose status is not one of the four of Section 4.2 fails validation.
    """
    bundle = copy.deepcopy(load_bundle())
    bundle["entries"][0]["status"] = "green"

    with pytest.raises(BundleValidationError, match="status"):
        validate_bundle(bundle)


def test_an_entry_missing_a_field_is_refused() -> None:
    """
    Test an entry that drops one of the Section 4.1 fields fails validation.
    """
    bundle = copy.deepcopy(load_bundle())
    del bundle["entries"][0]["receipts"]

    with pytest.raises(BundleValidationError, match="missing fields"):
        validate_bundle(bundle)


def test_an_entry_with_a_field_outside_the_schema_is_refused() -> None:
    """
    Test an entry carrying a field the schema does not document fails validation.
    """
    bundle = copy.deepcopy(load_bundle())
    bundle["entries"][0]["score"] = 1.0

    with pytest.raises(BundleValidationError, match="outside the schema"):
        validate_bundle(bundle)


def test_an_entry_without_a_fixture_is_refused() -> None:
    """
    Test an entry that names no fixture fails validation.
    """
    bundle = copy.deepcopy(load_bundle())
    bundle["entries"][0]["fixture"] = None

    with pytest.raises(BundleValidationError, match="fixture"):
        validate_bundle(bundle)


def test_a_paraphrased_requirement_is_detected() -> None:
    """
    Test a requirement sentence that is not verbatim from the design is reported by criterion.
    """
    bundle = copy.deepcopy(load_bundle())
    bundle["entries"][0]["requirement"] = "A candidate can be compiled, run and recorded."

    violations = _verbatim_violations(bundle, _design_section_15_text())

    assert violations == [1], f"violations are {violations}"
