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
A datum's knowledge date: whether it could have been known when the decision was made.

A record's reference time is not its knowledge time. A bar stamped at the open of a session may be
published an hour later; a research note about last night may be written this morning. A backtest
that reads a datum at its reference time grants the strategy information that did not yet exist,
and the leak is silent because every field of the record is internally consistent. The knowledge
date is the instant at which the datum became knowable, carried beside the payload rather than
inferred from it, and the gate below is the rule that reads it.

The gate is deliberately cheap and additive: it is a convention over a payload the bridge already
carries, not a change to `CustomData`. A payload that carries `knowledge_date` (nanoseconds since
the epoch) can be tested against a decision time; a payload that does not is excluded rather than
assumed fresh, because an absent provenance cannot be shown to have been knowable. The gate reads
no clock -- the decision time is supplied by the caller -- so a replay of the same records against
the same decision time sees the same answer.

The rule is exclusive and fail-closed, in that order of precedence:

1. an absent knowledge date is excluded and counted as `missing_knowledge_date`
2. a knowledge date strictly later than the decision time is excluded and counted as
   `published_after_decision`
3. a knowledge date at or earlier than the decision time is admitted

The boundary is inclusive: a datum published at exactly the decision instant was knowable at it,
so it is admitted. Every outcome is counted, in the shape the data-quality gate uses, so a run
reports a number rather than a log line, and the two exclusion reasons are drawn from the bridge's
closed refusal vocabulary so an excluded record can be named without prose.
"""

from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass
from typing import Any


# The payload key and object attribute that carries the knowledge date. A datum is knowable from
# this instant onward; a payload that does not carry it is excluded rather than assumed fresh.
KNOWLEDGE_DATE_FIELD = "knowledge_date"


def knowledge_date_of(record: Any) -> int | None:  # noqa: ANN401 - the payload is read before its shape is known
    """
    Read the knowledge date from a record, or `None` when it carries none.

    The reader accepts any payload shape the bridge already carries: a mapping (a `CustomData`
    payload dict, for example) is read by key, and any other object is read by attribute. A value
    that is present but is not an `int` (or is a `bool`, which is an `int` subclass but not an
    instant) is treated as absent, because the reader never invents a date and an unusable one
    cannot be shown to be knowable.

    Parameters
    ----------
    record : Any
        The record whose knowledge date is read.

    Returns
    -------
    int | None
        The knowledge date in nanoseconds since the epoch, or `None` when it is absent or unusable.

    """
    if isinstance(record, Mapping):
        value = record.get(KNOWLEDGE_DATE_FIELD)
    else:
        value = getattr(record, KNOWLEDGE_DATE_FIELD, None)

    if isinstance(value, bool) or not isinstance(value, int):
        return None

    return value


@dataclass
class KnowledgeCounts:
    """
    The knowledge-date gate's totals and per-reason exclusion counts.

    The shape mirrors the engine's data-quality counts: a total of records seen, a total admitted,
    a total excluded, and a count per exclusion reason, so a run reports a number rather than a log
    line. The invariant is structural rather than checked: an admission increments `considered` and
    `admitted`, and an exclusion increments `considered`, `excluded` and exactly one reason, so
    `excluded == published_after_decision + missing_knowledge_date` holds after every call.

    Parameters
    ----------
    considered : int
        The number of records the gate has seen.
    admitted : int
        The number of records admitted.
    excluded : int
        The number of records excluded.
    published_after_decision : int
        The number of records excluded because they were published after the decision time.
    missing_knowledge_date : int
        The number of records excluded because they carried no knowledge date.

    """

    considered: int = 0
    admitted: int = 0
    excluded: int = 0
    published_after_decision: int = 0
    missing_knowledge_date: int = 0

    def record_admitted(self) -> None:
        """
        Record an admitted record.
        """
        self.considered += 1
        self.admitted += 1

    def record_published_after_decision(self) -> None:
        """
        Record a record excluded because it was published after the decision time.
        """
        self.considered += 1
        self.excluded += 1
        self.published_after_decision += 1

    def record_missing_knowledge_date(self) -> None:
        """
        Record a record excluded because it carried no knowledge date.
        """
        self.considered += 1
        self.excluded += 1
        self.missing_knowledge_date += 1

    def __str__(self) -> str:
        """
        Return the counts as one line naming every total and reason count.
        """
        return (
            f"knowledge: considered={self.considered} admitted={self.admitted} "
            f"excluded={self.excluded} "
            f"published_after_decision={self.published_after_decision} "
            f"missing_knowledge_date={self.missing_knowledge_date}"
        )


def admitted_at(
    record: Any,  # noqa: ANN401 - the payload is read before its shape is known
    decision_time: int,
    exclusions: KnowledgeCounts,
) -> bool:
    """
    Return whether a record was knowable at the decision time, recording the outcome.

    The rule is exclusive and fail-closed: an absent knowledge date is excluded and counted as
    missing, because the datum's provenance is unknown and it cannot be shown to have been knowable;
    a knowledge date strictly later than `decision_time` is excluded and counted as published after
    the decision; and a knowledge date at or earlier than `decision_time` is admitted, so the
    boundary is inclusive. Every outcome is recorded on `exclusions`, so the caller's totals stay
    consistent with the returned answer. The function reads no clock.

    Parameters
    ----------
    record : Any
        The record to test, as a mapping or an object.
    decision_time : int
        The decision instant, in nanoseconds since the epoch.
    exclusions : KnowledgeCounts
        The accumulator to record the outcome on.

    Returns
    -------
    bool
        Whether the record was knowable at the decision time.

    """
    knowledge_date = knowledge_date_of(record)

    if knowledge_date is None:
        exclusions.record_missing_knowledge_date()
        return False

    if knowledge_date > decision_time:
        exclusions.record_published_after_decision()
        return False

    exclusions.record_admitted()
    return True
