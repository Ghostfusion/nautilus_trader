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
Tests for the masked-replay probe.
"""

from __future__ import annotations

import re

import pandas as pd
import pytest

from nautilus_trader.decision_bridge.knowledge import KnowledgeCounts
from nautilus_trader.decision_bridge.masking import IDENTIFIER_MASK
from nautilus_trader.decision_bridge.masking import date_only
from nautilus_trader.decision_bridge.masking import mask_dates
from nautilus_trader.decision_bridge.masking import mask_identifiers
from nautilus_trader.decision_bridge.masking import masked_replay
from nautilus_trader.decision_bridge.news import NewsItem
from nautilus_trader.testkit.providers import TestDataProvider


PUBLICATION_TS = 1_700_000_000_000_000_000
LATENCY_NS = 1_000_000_000
DECISION_TIME = PUBLICATION_TS + 10 * LATENCY_NS
CURRENCIES = ("CHF", "EUR", "USD", "GBP", "JPY")


def item(headline: str, *, publication_ts: int = PUBLICATION_TS) -> NewsItem:
    """
    Return a news item whose receipt is a fixed latency after its publication.
    """
    return NewsItem.received(
        item_id=f"news-{headline[:12]}",
        source="wire",
        headline=headline,
        publication_ts=publication_ts,
        receipt_ts=publication_ts + LATENCY_NS,
    )


def currency_signal(text: str) -> str | None:
    """
    Stand in for a model whose decision is a currency code it recognizes in the text.
    """
    for code in CURRENCIES:
        if code in text:
            return f"VIEW {code}"
    return None


def ticker_signal(text: str) -> str | None:
    """
    Stand in for a model whose decision is the leading three-letter code of the headline.
    """
    tokens = text.split()
    token = tokens[0] if tokens else ""
    return f"VIEW {token}" if len(token) == 3 and token.isupper() else None


def date_signal(text: str) -> str | None:
    """
    Stand in for a model that decides from the presence of a date alone.
    """
    return "PERIOD" if re.search(r"\d{4}-\d{2}-\d{2}", text) else None


def word_signal(text: str) -> str | None:
    """
    Stand in for a model whose decision is a word of the sentence, not an identifier.
    """
    return "BUY" if "beats" in text else None


def calendar_records(row_count: int = 12) -> tuple[list[NewsItem], int]:
    """
    Return news items built from the testkit's economic calendar, and a decision time.
    """
    frame = TestDataProvider().read_csv("news_events.csv", nrows=row_count)
    records = [
        item(
            f"{row['Currency']} {row['Start']} {row['Name']}",
            publication_ts=int(pd.Timestamp(row["Start"], tz="UTC").value),
        )
        for row in frame.to_dict("records")
    ]

    return records, sorted(record.receipt_ts for record in records)[len(records) // 2]


def test_masking_identifiers_is_whole_word_and_case_insensitive() -> None:
    """
    Test that masking replaces named identifiers only where they stand as a word.
    """
    masked, count = mask_identifiers(
        "Apple beats; APPLE raised guidance; pineapple prices fell",
        ["Apple"],
    )

    assert masked == (
        f"{IDENTIFIER_MASK} beats; {IDENTIFIER_MASK} raised guidance; pineapple prices fell"
    )
    assert count == 2


def test_masking_identifiers_replaces_the_longest_identifier_first() -> None:
    """
    Test that naming a longer and a shorter identifier masks both without a fragment.
    """
    masked, count = mask_identifiers("Apple Inc beat Apple", ["Apple", "Apple Inc"])

    assert masked == f"{IDENTIFIER_MASK} beat {IDENTIFIER_MASK}"
    assert count == 2


def test_masking_dates_covers_timestamps_and_date_only_keeps_them() -> None:
    """
    Test that the date mask covers a time of day and that the date-only control keeps the dates.
    """
    text = "Print at 2024-06-05T14:30:00 and again 2024-06-06"

    masked, count = mask_dates(text)

    assert masked == "Print at [DATE] and again [DATE]"
    assert count == 2
    assert date_only(text) == "2024-06-05T14:30:00 2024-06-06"
    assert date_only("no dates here") == ""


def test_the_probe_reports_a_signal_that_reads_identifiers() -> None:
    """
    Test that masking the identifier a signal depends on changes its decision.
    """
    records = [
        item("CHF GDP beats"),
        item("EUR PMI beats"),
        item("CHF retail misses"),
    ]

    report = masked_replay(
        currency_signal,
        records,
        decision_time=DECISION_TIME,
        identifiers=["CHF"],
    )

    assert report.considered == 3
    assert report.admitted == 3
    assert report.excluded == 0
    assert report.decisions_unmasked == 3
    assert report.decisions_masked == 1  # only the EUR record survives its mask
    assert report.changed_by_masking == 2
    assert report.identifiers_decide is True


def test_the_probe_reports_a_signal_that_reads_only_dates() -> None:
    """
    Test that a signal deciding from the dates alone is reported as date-only recall.
    """
    records = [item("2024-06-05 CPI print"), item("2024-06-06 payrolls")]

    report = masked_replay(date_signal, records, decision_time=DECISION_TIME)

    assert report.decisions_unmasked == 2
    assert report.decisions_date_only == 2
    assert report.changed_by_date_only == 0
    assert report.date_only_recall is True


def test_the_probe_reports_a_signal_that_survives_masking() -> None:
    """
    Test that a signal reading the sentence rather than an identifier is unchanged by masking.
    """
    records = [item("CHF GDP beats"), item("EUR PMI beats")]

    report = masked_replay(
        word_signal,
        records,
        decision_time=DECISION_TIME,
        identifiers=["CHF", "EUR"],
    )

    assert report.decisions_unmasked == report.decisions_masked == 2
    assert report.changed_by_masking == 0
    assert report.identifiers_decide is False
    assert report.decisions_date_only == 0
    assert report.date_only_recall is False


def test_the_probe_excludes_a_record_the_decision_could_not_have_knowledge_of() -> None:
    """
    Test that a record published after the decision is excluded and never replayed.
    """
    admitted_record = item("CHF GDP beats")
    late_record = item("CHF retail misses", publication_ts=DECISION_TIME + 1)
    counts = KnowledgeCounts()

    report = masked_replay(
        currency_signal,
        [admitted_record, late_record],
        decision_time=DECISION_TIME,
        identifiers=["CHF"],
        gate=counts,
    )

    assert report.gate is counts
    assert report.considered == 2
    assert report.admitted == 1
    assert report.excluded == 1
    assert counts.published_after_decision == 1
    assert report.decisions_unmasked == 1  # the late record was never replayed
    assert report.changed_by_masking == 1


def test_the_report_renders_the_exact_one_line() -> None:
    """
    Test that the report renders exactly one line naming all eight counts.
    """
    report = masked_replay(
        currency_signal,
        [item("CHF beats")],
        decision_time=DECISION_TIME,
        identifiers=["CHF"],
    )

    assert str(report) == (
        "masked_replay: considered=1 admitted=1 excluded=0 "
        "decisions_unmasked=1 decisions_masked=0 decisions_date_only=0 "
        "changed_by_masking=1 changed_by_date_only=1"
    )


def test_a_record_carrying_no_text_is_refused() -> None:
    """
    Test that a record without the text field is refused rather than replayed empty.
    """
    with pytest.raises(TypeError, match="carries no text"):
        masked_replay(
            word_signal,
            [{"knowledge_date": PUBLICATION_TS}],
            decision_time=DECISION_TIME,
        )


def test_the_probe_reports_an_identifier_reading_calendar_signal() -> None:
    """
    Test the probe on the testkit's economic calendar with a signal reading the leading code.
    """
    records, decision_time = calendar_records()
    identifiers = sorted({record.headline.split()[0] for record in records})

    report = masked_replay(
        ticker_signal,
        records,
        decision_time=decision_time,
        identifiers=identifiers,
    )

    assert report.considered == len(records)
    assert report.admitted + report.excluded == len(records)
    assert report.admitted > 0
    assert report.excluded > 0
    assert report.decisions_unmasked == report.admitted
    assert report.decisions_masked == 0
    assert report.changed_by_masking == report.admitted
    assert report.identifiers_decide is True


def test_the_probe_exposes_a_date_only_calendar_signal() -> None:
    """
    Test that the calendar's dates alone drive a date-only signal, which the probe reports.
    """
    records, decision_time = calendar_records()

    report = masked_replay(date_signal, records, decision_time=decision_time)

    assert report.decisions_unmasked == report.admitted
    assert report.decisions_date_only == report.admitted
    assert report.changed_by_date_only == 0
    assert report.date_only_recall is True
    assert report.admitted > 0
