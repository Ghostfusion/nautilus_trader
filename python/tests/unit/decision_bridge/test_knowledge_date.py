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
Tests for the knowledge-date gate.
"""

from __future__ import annotations

from dataclasses import dataclass

from nautilus_trader.decision_bridge.contract import RefusalCode
from nautilus_trader.decision_bridge.contract import Stage
from nautilus_trader.decision_bridge.contract import stage_of
from nautilus_trader.decision_bridge.execution import DENIAL_CODES
from nautilus_trader.decision_bridge.knowledge import KNOWLEDGE_DATE_FIELD
from nautilus_trader.decision_bridge.knowledge import KnowledgeCounts
from nautilus_trader.decision_bridge.knowledge import admitted_at
from nautilus_trader.decision_bridge.knowledge import knowledge_date_of


DECISION_TIME = 1_000_000_000


@dataclass(frozen=True)
class Datum:
    """
    An object payload carrying a knowledge date attribute.
    """

    knowledge_date: int | None = None


@dataclass(frozen=True)
class Payload:
    """
    An object payload that carries no knowledge date attribute at all.
    """

    value: int = 0


def test_a_record_published_after_the_decision_is_excluded_and_counted() -> None:
    """
    Test that a knowledge date later than the decision excludes the record and counts it.
    """
    counts = KnowledgeCounts()
    record = {"knowledge_date": DECISION_TIME + 1}

    assert admitted_at(record, DECISION_TIME, counts) is False
    assert counts.considered == 1
    assert counts.admitted == 0
    assert counts.excluded == 1
    assert counts.published_after_decision == 1
    assert counts.missing_knowledge_date == 0


def test_a_record_known_at_the_decision_instant_is_admitted() -> None:
    """
    Test that the boundary is inclusive: a knowledge date equal to the decision is admitted.
    """
    counts = KnowledgeCounts()
    record = {"knowledge_date": DECISION_TIME}

    assert admitted_at(record, DECISION_TIME, counts) is True
    assert counts.considered == 1
    assert counts.admitted == 1
    assert counts.excluded == 0


def test_a_record_known_before_the_decision_is_admitted() -> None:
    """
    Test that a knowledge date earlier than the decision is admitted.
    """
    counts = KnowledgeCounts()
    record = {"knowledge_date": DECISION_TIME - 1}

    assert admitted_at(record, DECISION_TIME, counts) is True
    assert counts.admitted == 1


def test_a_record_with_no_knowledge_date_is_excluded_as_missing_not_assumed_fresh() -> None:
    """
    Test that an absent knowledge date is excluded as missing rather than assumed knowable.
    """
    counts = KnowledgeCounts()

    assert admitted_at({}, DECISION_TIME, counts) is False
    assert admitted_at(Payload(value=1), DECISION_TIME, counts) is False
    assert counts.considered == 2
    assert counts.admitted == 0
    assert counts.excluded == 2
    assert counts.published_after_decision == 0
    assert counts.missing_knowledge_date == 2


def test_the_counts_render_the_exact_one_line() -> None:
    """
    Test that the counts render exactly one line naming all five counts.
    """
    empty = KnowledgeCounts()
    assert str(empty) == (
        "knowledge: considered=0 admitted=0 excluded=0 "
        "published_after_decision=0 missing_knowledge_date=0"
    )

    counts = KnowledgeCounts()
    counts.record_admitted()
    counts.record_published_after_decision()
    counts.record_published_after_decision()
    counts.record_missing_knowledge_date()

    assert str(counts) == (
        "knowledge: considered=4 admitted=1 excluded=3 "
        "published_after_decision=2 missing_knowledge_date=1"
    )


def test_a_mixed_stream_accumulates_consistent_totals() -> None:
    """
    Test that a mixed stream splits `excluded` across both reasons and stays consistent.
    """
    counts = KnowledgeCounts()
    stream = (
        {"knowledge_date": DECISION_TIME - 10},  # admitted
        {"knowledge_date": DECISION_TIME},  # admitted, inclusive boundary
        {"knowledge_date": DECISION_TIME + 1},  # published after the decision
        {},  # missing knowledge date
        {"knowledge_date": DECISION_TIME + 100},  # published after the decision
    )

    answers = [admitted_at(record, DECISION_TIME, counts) for record in stream]

    assert answers == [True, True, False, False, False]
    assert counts.considered == 5
    assert counts.admitted == 2
    assert counts.excluded == 3
    assert counts.published_after_decision == 2
    assert counts.missing_knowledge_date == 1
    assert counts.excluded == counts.published_after_decision + counts.missing_knowledge_date
    assert counts.considered == counts.admitted + counts.excluded


def test_the_reader_accepts_a_mapping_and_an_object_payload() -> None:
    """
    Test that the reader reads both a mapping key and an object attribute.
    """
    assert knowledge_date_of({"knowledge_date": 42}) == 42
    assert knowledge_date_of(Datum(knowledge_date=42)) == 42
    assert KNOWLEDGE_DATE_FIELD == "knowledge_date"

    assert knowledge_date_of({}) is None
    assert knowledge_date_of(Payload(value=1)) is None
    assert knowledge_date_of(Datum(knowledge_date=None)) is None


def test_an_unusable_knowledge_date_is_treated_as_absent() -> None:
    """
    Test that a value which is not an instant is treated as absent rather than invented.
    """
    assert knowledge_date_of({"knowledge_date": "yesterday"}) is None
    assert knowledge_date_of({"knowledge_date": True}) is None
    assert knowledge_date_of(Datum(knowledge_date=1.5)) is None  # type: ignore[arg-type]


def test_the_exclusion_reasons_are_named_in_the_closed_refusal_vocabulary() -> None:
    """
    Test that both exclusion reasons are named in the refusal and denial vocabularies.
    """
    assert RefusalCode.PUBLISHED_AFTER_DECISION.value == "PUBLISHED_AFTER_DECISION"
    assert RefusalCode.MISSING_KNOWLEDGE_DATE.value == "MISSING_KNOWLEDGE_DATE"
    assert stage_of(RefusalCode.PUBLISHED_AFTER_DECISION) is Stage.AVAILABILITY
    assert stage_of(RefusalCode.MISSING_KNOWLEDGE_DATE) is Stage.AVAILABILITY
    assert "PUBLISHED_AFTER_DECISION" in DENIAL_CODES
    assert "MISSING_KNOWLEDGE_DATE" in DENIAL_CODES
