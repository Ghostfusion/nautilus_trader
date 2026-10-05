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
Tests for the news and sentiment item.
"""

from __future__ import annotations

from pathlib import Path

import pytest

from nautilus_trader.decision_bridge.knowledge import KnowledgeCounts
from nautilus_trader.decision_bridge.knowledge import admitted_at
from nautilus_trader.decision_bridge.knowledge import knowledge_date_of
from nautilus_trader.decision_bridge.news import NewsItem
from nautilus_trader.model import CustomData
from nautilus_trader.model import DataType
from nautilus_trader.model import register_custom_data_class
from nautilus_trader.persistence import ParquetDataCatalog


PUBLICATION_TS = 1_700_000_000_000_000_000
LATENCY_NS = 900_000_000
RECEIPT_TS = PUBLICATION_TS + LATENCY_NS
DECISION_TIME = RECEIPT_TS + 1


def item(
    publication_ts: int = PUBLICATION_TS,
    receipt_ts: int = RECEIPT_TS,
) -> NewsItem:
    """
    Return an item with a distinct publication and receipt instant.
    """
    return NewsItem.received(
        item_id="news-0001",
        source="wire",
        headline="A headline",
        publication_ts=publication_ts,
        receipt_ts=receipt_ts,
        symbols="AAPL",
        sentiment=-0.25,
    )


def test_an_item_carries_both_instants_and_is_knowable_only_from_its_receipt() -> None:
    """
    Test that the two instants are distinct and that the knowledge date is the receipt.
    """
    news = item()

    assert news.publication_ts == PUBLICATION_TS
    assert news.receipt_ts == RECEIPT_TS
    assert news.publication_ts != news.receipt_ts
    assert news.knowledge_date == news.receipt_ts
    assert news.latency_ns == LATENCY_NS
    assert knowledge_date_of(news) == news.receipt_ts


def test_the_boundary_instants_mirror_publication_and_receipt() -> None:
    """
    Test that the custom-data boundary instants are the two news instants, not placeholders.
    """
    news = item()
    custom = CustomData(DataType("NewsItem"), news)

    assert custom.ts_event == news.publication_ts
    assert custom.ts_init == news.receipt_ts
    assert custom.ts_event != custom.ts_init


def test_a_receipt_that_precedes_publication_is_refused() -> None:
    """
    Test that an item cannot be received before it was published.
    """
    with pytest.raises(ValueError, match="precedes the publication instant"):
        item(publication_ts=RECEIPT_TS, receipt_ts=RECEIPT_TS - 1)


@pytest.mark.parametrize(
    ("publication_ts", "receipt_ts"),
    [
        (-1, RECEIPT_TS),
        (PUBLICATION_TS, -1),
        (-1, -1),
    ],
)
def test_a_negative_instant_is_refused(publication_ts: int, receipt_ts: int) -> None:
    """
    Test that a negative or defaulted instant is refused rather than read as the epoch.
    """
    with pytest.raises(ValueError, match="cannot be negative"):
        item(publication_ts=publication_ts, receipt_ts=receipt_ts)


def test_an_item_published_after_the_decision_is_excluded_and_counted() -> None:
    """
    Test that an item published after the decision time is excluded and counted.
    """
    counts = KnowledgeCounts()
    news = item(publication_ts=DECISION_TIME + 1, receipt_ts=DECISION_TIME + 2)

    assert admitted_at(news, DECISION_TIME, counts) is False
    assert counts.considered == 1
    assert counts.admitted == 0
    assert counts.excluded == 1
    assert counts.published_after_decision == 1


def test_an_item_published_before_the_decision_but_received_after_it_is_excluded() -> None:
    """
    Test that availability follows the receipt, not the publication.

    This is the case the type exists for: the article was published before the decision, so a gate
    reading the publication instant would admit it, but the process had not received it yet.
    """
    counts = KnowledgeCounts()
    news = item(publication_ts=PUBLICATION_TS, receipt_ts=DECISION_TIME + 1)

    assert news.publication_ts <= DECISION_TIME
    assert admitted_at(news, DECISION_TIME, counts) is False
    assert counts.considered == 1
    assert counts.admitted == 0
    assert counts.excluded == 1
    assert counts.published_after_decision == 1
    assert counts.missing_knowledge_date == 0
    assert str(counts) == (
        "knowledge: considered=1 admitted=0 excluded=1 "
        "published_after_decision=1 missing_knowledge_date=0"
    )


def test_an_item_received_at_the_decision_instant_is_admitted() -> None:
    """
    Test that the boundary is inclusive on the receipt instant.
    """
    counts = KnowledgeCounts()
    news = item(receipt_ts=DECISION_TIME)

    assert admitted_at(news, DECISION_TIME, counts) is True
    assert counts.admitted == 1
    assert counts.excluded == 0


def test_a_stream_of_items_is_counted_consistently() -> None:
    """
    Test that a mixed stream of items accumulates totals that stay consistent.
    """
    counts = KnowledgeCounts()
    stream = (
        item(receipt_ts=DECISION_TIME - 1),  # admitted
        item(receipt_ts=DECISION_TIME),  # admitted, inclusive boundary
        item(publication_ts=DECISION_TIME, receipt_ts=DECISION_TIME + 1),  # not yet received
        item(publication_ts=DECISION_TIME + 1, receipt_ts=DECISION_TIME + 2),  # not yet published
    )

    answers = [admitted_at(news, DECISION_TIME, counts) for news in stream]

    assert answers == [True, True, False, False]
    assert counts.considered == 4
    assert counts.admitted == 2
    assert counts.excluded == 2
    assert counts.published_after_decision == 2
    assert counts.missing_knowledge_date == 0
    assert counts.considered == counts.admitted + counts.excluded


def test_an_item_round_trips_through_custom_data_json() -> None:
    """
    Test that both instants survive the JSON bytes round trip and gate identically afterwards.
    """
    register_custom_data_class(NewsItem)
    news = item()
    custom = CustomData(DataType("NewsItem"), news)

    restored_custom = CustomData.from_json_bytes(bytes(custom.to_json_bytes()))
    restored = restored_custom.data

    assert isinstance(restored, NewsItem)
    assert restored.item_id == news.item_id
    assert restored.source == news.source
    assert restored.headline == news.headline
    assert restored.symbols == news.symbols
    assert restored.sentiment == news.sentiment
    assert restored.publication_ts == news.publication_ts
    assert restored.receipt_ts == news.receipt_ts
    assert restored.knowledge_date == news.knowledge_date
    assert restored_custom.ts_event == news.publication_ts
    assert restored_custom.ts_init == news.receipt_ts

    counts = KnowledgeCounts()
    assert admitted_at(restored, DECISION_TIME, counts) is True


def test_an_item_round_trips_through_the_catalog(tmp_path: Path) -> None:
    """
    Test that the item is writable custom data and reads back with both instants intact.
    """
    register_custom_data_class(NewsItem)
    catalog = ParquetDataCatalog(str(tmp_path))
    news = item()
    catalog.write_custom_data([CustomData(DataType("NewsItem"), news)])

    result = catalog.query_custom_data("NewsItem")

    assert len(result) == 1
    restored = result[0].data
    assert isinstance(restored, NewsItem)
    assert restored.publication_ts == news.publication_ts
    assert restored.receipt_ts == news.receipt_ts
    assert restored.latency_ns == news.latency_ns
    assert restored.knowledge_date == news.knowledge_date
    assert result[0].ts_event == news.publication_ts
    assert result[0].ts_init == news.receipt_ts
