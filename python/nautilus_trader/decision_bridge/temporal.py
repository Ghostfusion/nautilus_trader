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
Actionability resolved from the instrument's calendar, and bounded below by receipt.

A datum's availability is not its reference time. The artifact carries a reference *date* (the day
the
analysis is about), an instant it existed, and an instant it stops being usable; the bridge holds a
fourth instant it does not trust from the artifact, the moment this engine received it, because a
producer cannot know when its output will be read. A backtest that timestamps a record at its
reference date grants the strategy every hour between midnight and production, and a signal whose
actionability already lies in the past is an instruction to act retroactively.

The resolution is a pure function of the artifact, the receipt instant and the instrument's
calendar.
It reads no clock. The six steps below are the design's, in the design's order, so that the first
failing step decides the reason:

1. the calendar does not cover `produced_at` refuses with `CALENDAR_UNCOVERED`
2. an absent `expires_at` refuses with `MISSING_EXPIRY`
3. `produced_at` at or after `expires_at` refuses with `ARTIFACT_EXPIRED`
4. the resolved instant is `produced_at` when it is tradeable and the next open otherwise, and an
   absence of both refuses with `ACTIONABILITY_INVALID`
5. an instant earlier than receipt is clamped to receipt and recorded as `ACTIONABILITY_PAST`
6. an instant at or after `expires_at` refuses with `ARTIFACT_EXPIRED`

Two prerequisites of step 1 are named rather than left implicit, because the producer's contract
leaves both cases schema-valid. An absent `produced_at` refuses with `PRODUCED_AT_ABSENT`, because
there is no instant to resolve availability *from*; a key with no calendar refuses with
`CALENDAR_MISSING`, because "I have no calendar for this instrument" and "the calendar I have does
not
cover this instant" are different absences with different remedies.

The clamp in step 5 is a diagnostic rather than a refusal: the artifact is not at fault, the engine
arrived late, and that is information about the pipeline. Expiry is exclusive, so an instant exactly
equal to the expiry is refused -- the producer's own contract notes that a session's last hour is a
window an artifact must not be actionable after, which is the same rule seen from the producer's
side.
An expiry is never derived from the horizon: a horizon is semantic and an expiry is operational, and
silently converting one into the other would let a vocabulary word decide when an artifact dies.

The rule reads its three answers from a `CalendarView`, so it accepts any calendar that answers
them, and the bridge is not limited to the calendars bundled with the crate. `JsonCalendarView` is
the path-backed source: it reads a directory of `nautilus-trading-calendar/v1` documents once, keys
them by the calendar's own key, and resolves that key to the same three answers. A key it does not
carry resolves to `None`, which the rule reports as `CALENDAR_MISSING`; a document it cannot read or
parse is treated as absent for the same reason rather than allowed to crash the resolution. The view
reads no clock and never re-reads a file, so a replay sees the same directory it was constructed
with.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import TYPE_CHECKING

from nautilus_trader.decision_bridge.contract import Diagnostic
from nautilus_trader.decision_bridge.contract import Refusal
from nautilus_trader.decision_bridge.contract import RefusalCode
from nautilus_trader.model import TradingCalendar


if TYPE_CHECKING:
    from typing import Protocol

    class CalendarView(Protocol):
        """
        The three answers the resolution reads from a trading calendar.

        The resolution depends on three questions only, so it states that rather than the whole
        calendar interface: a caller may supply the bundled calendar, a calendar loaded from a file,
        or a test double, and the resolution cannot read anything else by construction.
        """

        def covers(self, ts_ns: int) -> bool:
            """
            Return whether the calendar's data covers an instant.
            """

        def is_tradeable(self, ts_ns: int) -> bool:
            """
            Return whether an instant falls inside a trading session.
            """

        def next_open(self, ts_ns: int) -> int | None:
            """
            Return the next session's open, or `None` when there is none.
            """


class JsonCalendarView:
    """
    A directory of trading-calendar documents, each loaded once and resolved by the calendar's key.

    The view is the path-backed counterpart to `TradingCalendar.bundled`: it answers the same
    questions for the same key, but the data comes from a directory the caller names rather than
    from the crate's bundle. Every `*.json` entry in the directory is read once, at construction,
    through `TradingCalendar.from_json_path`, and indexed by the calendar's own `key` (for example
    `XNYS.EQUITY` or `FX.FX.LONDON`). `calendar_for` builds that key exactly as `CalendarKey`
    renders it -- `venue.asset_class` with an optional `venue.asset_class.symbol` -- which is the
    same convention `TradingCalendar.bundled` takes its arguments in, so a caller can swap one
    source for the other without a second wording.

    The view reads only the directory, and never the bundled set, so it cannot mask a bundled
    calendar silently. When a directory entry and a bundled calendar carry the same key, the caller
    composes the two explicitly (`view.calendar_for(...) or TradingCalendar.bundled(...)`) and
    thereby states the precedence; that composition prefers the directory entry, because a locally
    supplied document is the operator's own statement about the key and the only way to serve
    instants beyond the bundled `valid_until`. Because the two sources are never merged here, the
    collision is a call-site decision rather than an override applied inside this class.

    A document that cannot be read or parsed is treated as absent rather than as a failure: the key
    it would have served resolves to `None`, and the rule reports `CALENDAR_MISSING`. Two documents
    that declare the same key are a configuration error rather than a choice, and raise here.

    Parameters
    ----------
    directory : str | pathlib.Path
        The directory whose `*.json` documents are the calendars. It is read once, here; the view
        reads no clock and re-reads no file afterwards.

    Raises
    ------
    ValueError
        If two documents in the directory declare the same calendar key, because preferring one
        source over the other without saying so would be a silent resolution.

    """

    def __init__(self, directory: str | Path) -> None:
        """
        Initialize the instance and load every calendar document once.
        """
        documents: dict[str, TradingCalendar] = {}
        paths: dict[str, Path] = {}
        for path in sorted(Path(directory).glob("*.json")):
            try:
                calendar = TradingCalendar.from_json_path(str(path))
            except ValueError:
                # `TradingCalendar.from_json_path` maps both an unreadable file and malformed
                # content to `ValueError`; a document the loader cannot produce is treated as an
                # absent calendar, so that its key resolves to `CALENDAR_MISSING` rather than
                # raising out of construction or out of the rule.
                continue
            previous = paths.get(calendar.key)
            if previous is not None:
                raise ValueError(
                    f"duplicate calendar key {calendar.key!r}: {previous} and {path}",
                )
            documents[calendar.key] = calendar
            paths[calendar.key] = path
        self._documents = documents

    @property
    def keys(self) -> tuple[str, ...]:
        """
        Return the calendar keys the directory carries, in sorted order.
        """
        return tuple(sorted(self._documents))

    def calendar_for(
        self,
        venue: str,
        asset_class: str,
        symbol: str | None = None,
    ) -> CalendarView | None:
        """
        Return the calendar for a key, or `None` when the directory carries none.

        Parameters
        ----------
        venue : str
            The venue, exactly as `TradingCalendar.bundled` expects it.
        asset_class : str
            The instrument class, exactly as `TradingCalendar.bundled` expects it.
        symbol : str | None
            The optional symbol, for a calendar that is specific to a listing.

        Returns
        -------
        CalendarView | None
            The parsed calendar, which answers the three questions the resolution reads, or `None`
            when no document in the directory carries the key.

        """
        key = f"{venue}.{asset_class}" if symbol is None else f"{venue}.{asset_class}.{symbol}"
        return self._documents.get(key)


@dataclass(frozen=True)
class Actionability:
    """
    The instant an artifact becomes actionable, and what was recorded on the way.

    Parameters
    ----------
    actionable_at : int
        The resolved instant, as UNIX nanoseconds. It is the projected signal's event time.
    diagnostics : tuple[Diagnostic, ...]
        Diagnostics recorded while resolving, which do not stop the artifact.

    """

    actionable_at: int
    diagnostics: tuple[Diagnostic, ...] = ()


def resolve_actionability(  # noqa: PLR0911 - the first failing step decides the reason
    *,
    produced_at: int | None,
    expires_at: int | None,
    received_at: int,
    calendar: CalendarView | None,
) -> Actionability | Refusal:
    """
    Resolve the instant an artifact becomes actionable.

    Parameters
    ----------
    produced_at : int | None
        The instant the artifact existed, as UNIX nanoseconds, or `None` when the artifact does not
        carry one.
    expires_at : int | None
        The instant the artifact ceases to be actionable, as UNIX nanoseconds, or `None` when
        absent.
    received_at : int
        The moment this engine admitted the artifact, as UNIX nanoseconds. Supplied by the caller
        rather than read from the artifact or from a clock.
    calendar : CalendarView | None
        The instrument's trading calendar, or `None` when no calendar is available for its key. The
        protocol states the three questions the resolution reads, so a bundled calendar, a calendar
        loaded from a file and a test double are all accepted and nothing else can be read.

    Returns
    -------
    Actionability | Refusal
        The resolved instant, or the reason it could not be resolved.

    """
    # Prerequisite of step 1.
    if produced_at is None:
        return Refusal(
            RefusalCode.PRODUCED_AT_ABSENT,
            "the artifact carries no produced_at, so availability cannot be resolved from it",
        )

    # Prerequisite of step 1: an absent calendar and an uncovered instant are different absences.
    if calendar is None:
        return Refusal(
            RefusalCode.CALENDAR_MISSING,
            "no trading calendar is available for the instrument's key",
        )

    # Step 1.
    if not calendar.covers(produced_at):
        return Refusal(
            RefusalCode.CALENDAR_UNCOVERED,
            f"the calendar does not cover produced_at={produced_at}",
        )

    # Step 2.
    if expires_at is None:
        return Refusal(
            RefusalCode.MISSING_EXPIRY,
            "the artifact carries no expires_at; an expiry is required and is never derived",
        )

    # Step 3.
    if produced_at >= expires_at:
        return Refusal(
            RefusalCode.ARTIFACT_EXPIRED,
            f"produced_at={produced_at} is at or after expires_at={expires_at}",
        )

    # Step 4.
    if calendar.is_tradeable(produced_at):
        resolved = produced_at
    else:
        next_open = calendar.next_open(produced_at)
        if next_open is None:
            return Refusal(
                RefusalCode.ACTIONABILITY_INVALID,
                f"produced_at={produced_at} is not tradeable and no next open exists",
            )
        resolved = next_open

    # Step 5.
    diagnostics: tuple[Diagnostic, ...] = ()
    if resolved < received_at:
        resolved = received_at
        diagnostics = (Diagnostic.ACTIONABILITY_PAST,)

    # Step 6.
    if resolved >= expires_at:
        return Refusal(
            RefusalCode.ARTIFACT_EXPIRED,
            f"the resolved instant {resolved} is at or after expires_at={expires_at}",
        )

    return Actionability(actionable_at=resolved, diagnostics=diagnostics)
