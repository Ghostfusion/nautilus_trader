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
A masked-replay probe for text and language-model signals.

A text signal can read two things that a price series cannot: an identifier it may have seen
during pretraining, and a date it may simply remember. Both leaks survive a prompt instruction,
because the instruction is part of the input rather than a property of it, and both are invisible
in the reported result because the signal's output looks the same either way. The probe makes the
dependence observable by replaying the same records through the same signal under two controls and
reporting where the two replays disagree.

The first control masks identifiers. Every identifier the caller names is replaced, whole-word and
case-insensitively, with a placeholder, so a signal whose decision changes when the identifier is
gone was reading the identifier rather than the sentence. The chapter's finding is that this beats
instructing a model to ignore identifiers, because masking removes the evidence while an instruction
leaves it in place to be attended to anyway.

The second control keeps only the dates. A signal that still decides from a date alone, with the
rest of the text removed, is recalling the period rather than reading the record, which is what the
disclosed-cutoff staging exists to detect.

The replay is knowledge-gated: a record whose knowledge date is absent or later than the decision
time is excluded by the same rule `decision_bridge.knowledge` applies to any datum, and it is not
replayed at all, so a probe cannot report a decision that was not knowable when it was made. The
probe reads no clock -- the decision time is supplied by the caller -- so the same records replayed
against the same decision time give the same report.

Every count is reported, in the data-quality gate's shape, so the probe produces a number rather
than an opinion: how many records the gate admitted and excluded, how many decisions each of the
three replays made, and where the masked and date-only replays diverged from the unmasked one.
"""

from __future__ import annotations

import re
from collections.abc import Callable
from collections.abc import Iterable
from collections.abc import Mapping
from collections.abc import Sequence
from dataclasses import dataclass
from typing import Any

from nautilus_trader.decision_bridge.knowledge import KnowledgeCounts
from nautilus_trader.decision_bridge.knowledge import admitted_at


# The placeholders the two controls substitute. They are fixed rather than configurable so a report
# can be compared across probes, and they carry no digits so they cannot be mistaken for a date.
IDENTIFIER_MASK = "[IDENTIFIER]"
DATE_MASK = "[DATE]"

# ISO 8601 calendar dates, with an optional time of day and either separator, which is the form the
# test data and the platform's instants use.
_DATE_PATTERN = re.compile(r"\d{4}-\d{2}-\d{2}(?:[T ]\d{2}:\d{2}(?::\d{2})?)?")


def mask_identifiers(
    text: str,
    identifiers: Iterable[str],
    *,
    replacement: str = IDENTIFIER_MASK,
) -> tuple[str, int]:
    """
    Replace every named identifier in the text with a placeholder.

    Matching is whole-word and case-insensitive, and the longest identifier is replaced first, so
    naming both `Apple` and `Apple Inc` does not leave a fragment of the longer one behind. An
    identifier that appears as part of a longer word is left alone, because `APP` in `APPLE` is a
    different token, and a caller who wants it masked names the whole word.

    Parameters
    ----------
    text : str
        The text to mask.
    identifiers : Iterable[str]
        The identifiers to replace, in any order.
    replacement : str, default IDENTIFIER_MASK
        The placeholder written in place of each identifier.

    Returns
    -------
    tuple[str, int]
        The masked text and the number of replacements made.

    """
    masked = text
    count = 0

    for identifier in sorted(set(identifiers), key=len, reverse=True):
        if not identifier:
            continue
        pattern = rf"(?<!\w){re.escape(identifier)}(?!\w)"
        masked, replaced = re.subn(pattern, replacement, masked, flags=re.IGNORECASE)
        count += replaced

    return masked, count


def mask_dates(text: str, *, replacement: str = DATE_MASK) -> tuple[str, int]:
    """
    Replace every ISO date in the text, with its time of day when present, with a placeholder.

    Parameters
    ----------
    text : str
        The text to mask.
    replacement : str, default DATE_MASK
        The placeholder written in place of each date.

    Returns
    -------
    tuple[str, int]
        The masked text and the number of replacements made.

    """
    masked, count = _DATE_PATTERN.subn(replacement, text)
    return masked, count


def date_only(text: str) -> str:
    """
    Return the dates the text carries, in order, and nothing else.

    This is the date-only control's input: a signal that still decides from this string decided
    from the period alone.

    Parameters
    ----------
    text : str
        The text to reduce.

    Returns
    -------
    str
        The dates joined by a single space, or an empty string when the text carries none.

    """
    return " ".join(_DATE_PATTERN.findall(text))


@dataclass
class MaskedReplayReport:
    """
    The masked-replay probe's counts and its two controls' divergence.

    Parameters
    ----------
    gate : KnowledgeCounts
        The knowledge gate's counts over every record the probe saw.
    decisions_unmasked : int
        The number of decisions the signal made on the full text.
    decisions_masked : int
        The number of decisions the signal made with the identifiers masked.
    decisions_date_only : int
        The number of decisions the signal made from the dates alone.
    changed_by_masking : int
        The number of admitted records whose decision changed when identifiers were masked.
    changed_by_date_only : int
        The number of admitted records whose decision changed when only the dates remained.

    """

    gate: KnowledgeCounts
    decisions_unmasked: int = 0
    decisions_masked: int = 0
    decisions_date_only: int = 0
    changed_by_masking: int = 0
    changed_by_date_only: int = 0

    @property
    def considered(self) -> int:
        """
        The number of records the probe saw.
        """
        return self.gate.considered

    @property
    def admitted(self) -> int:
        """
        The number of records the knowledge gate admitted and the probe replayed.
        """
        return self.gate.admitted

    @property
    def excluded(self) -> int:
        """
        The number of records the knowledge gate excluded and the probe did not replay.
        """
        return self.gate.excluded

    @property
    def identifiers_decide(self) -> bool:
        """
        Whether masking the identifiers changed any decision.
        """
        return self.changed_by_masking > 0

    @property
    def date_only_recall(self) -> bool:
        """
        Whether the signal decided from the dates alone on any record.
        """
        return self.decisions_date_only > 0

    def __str__(self) -> str:
        """
        Return the counts as one line naming every total and every control's outcome.
        """
        return (
            f"masked_replay: considered={self.considered} admitted={self.admitted} "
            f"excluded={self.excluded} "
            f"decisions_unmasked={self.decisions_unmasked} "
            f"decisions_masked={self.decisions_masked} "
            f"decisions_date_only={self.decisions_date_only} "
            f"changed_by_masking={self.changed_by_masking} "
            f"changed_by_date_only={self.changed_by_date_only}"
        )


def _text_of(record: Any, field: str) -> str:  # noqa: ANN401 - the payload is read before its shape is known
    """
    Read a record's text, from a mapping key or an object attribute.

    Parameters
    ----------
    record : Any
        The record whose text is read.
    field : str
        The key or attribute that carries it.

    Returns
    -------
    str

    Raises
    ------
    TypeError
        If the record carries no text under that name, or the value is not a string.

    """
    value = record.get(field) if isinstance(record, Mapping) else getattr(record, field, None)

    if not isinstance(value, str):
        raise TypeError(f"record carries no text as a string under {field!r}")

    return value


def masked_replay(  # noqa: PLR0913 - the probe's declared inputs
    signal: Callable[[str], str | None],
    records: Iterable[Any],
    *,
    decision_time: int,
    identifiers: Sequence[str] = (),
    text_field: str = "headline",
    gate: KnowledgeCounts | None = None,
) -> MaskedReplayReport:
    """
    Replay records through a text signal under the identifier and date-only controls.

    A record is replayed only when the knowledge gate admits it: an absent knowledge date, or one
    later than `decision_time`, excludes it by the same rule that gate applies everywhere else, so a
    record the strategy could not have read is never counted as a decision it made. The signal is
    called with one argument, the text, and answers a decision or nothing; a decision of `None` is
    an abstention and is not counted. Anything truthy is a decision, so a label, a direction or a
    score all work without this module knowing what they mean.

    The report is a value, and the gate's counts are the caller's own accumulator when one is
    passed, so a study can accumulate across probes and read one line per probe.

    Parameters
    ----------
    signal : Callable[[str], str | None]
        The text signal to probe, taking the text and returning a decision or `None`.
    records : Iterable[Any]
        The records to replay, as mappings or objects.
    decision_time : int
        The decision instant, in nanoseconds since the epoch.
    identifiers : Sequence[str], default ()
        The identifiers to mask in the masked control.
    text_field : str, default "headline"
        The key or attribute carrying each record's text.
    gate : KnowledgeCounts | None, default None
        The accumulator to record the gate's outcomes on, or None for the report's own.

    Returns
    -------
    MaskedReplayReport

    Raises
    ------
    TypeError
        If an admitted record carries no text under `text_field`.

    """
    counts = gate if gate is not None else KnowledgeCounts()
    report = MaskedReplayReport(gate=counts)

    for record in records:
        if not admitted_at(record, decision_time, counts):
            continue

        text = _text_of(record, text_field)
        unmasked = signal(text)
        masked = signal(mask_identifiers(text, identifiers)[0])
        dated = signal(date_only(text))

        if unmasked is not None:
            report.decisions_unmasked += 1
        if masked is not None:
            report.decisions_masked += 1
        if dated is not None:
            report.decisions_date_only += 1
        if masked != unmasked:
            report.changed_by_masking += 1
        if dated != unmasked:
            report.changed_by_date_only += 1

    return report
