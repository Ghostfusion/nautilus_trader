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
A news or sentiment observation carrying both its publication and its receipt instant.

A quote's timestamp is its observation instant, so one instant is enough for it. A news article is
stamped when its publisher released it and becomes observable to this process only when a feed
delivered it; the interval between the two is the information latency, and a type that carries one
instant and calls it both cannot express the leak it exists to prevent. This type carries both,
distinctly and explicitly, and refuses to be built without them.

`knowledge_date` is the receipt instant, because at a decision time a process can only read what it
had already received by then. That makes the knowledge gate of `decision_bridge.knowledge` an
availability gate in the strict sense; the publication instant answers the other question -- which
session an article may be tradable in, and how much latency was paid to trade it -- and stays on the
item for it. Collapsing the two, in either direction, is the error the type exists to make
impossible: using the publication instant as availability grants the process an article it had not
received, and using the receipt instant as the event time moves the news to the wrong session.

The item is a `@customdataclass` payload, so it travels the platform's existing custom-data path
unchanged: register it with `register_custom_data_class`, wrap it in `CustomData`, and it writes to
a catalog and reads back with both instants intact.
"""

from __future__ import annotations

from nautilus_trader.model.custom import customdataclass


@customdataclass()
class NewsItem:
    """
    One news or sentiment observation, as received.

    The two instants are both required and both explicit. An item cannot be received before it was
    published, and neither instant may be negative, so a defaulted or placeholder timestamp is
    refused rather than silently treated as the epoch.

    Parameters
    ----------
    item_id : str
        The identity of the item, unique within its source.
    source : str
        The feed, wire or venue the item was received from.
    headline : str
        The item's text, as published.
    publication_ts : int
        The instant the item was published, in nanoseconds since the epoch.
    receipt_ts : int
        The instant the item was received by this process, in nanoseconds since the epoch.
    symbols : str, default ""
        The comma-separated symbols the item concerns, empty when it is unscoped.
    sentiment : float, default 0.0
        The item's sentiment score, if the producer scored it. The range is a producer convention
        (bounded for a lexicon, unbounded for a model logit) and is not enforced here.

    """

    item_id: str
    source: str
    headline: str
    publication_ts: int
    receipt_ts: int
    symbols: str = ""
    sentiment: float = 0.0

    def __post_init__(self) -> None:
        """
        Validate that the two instants describe a receivable item.
        """
        if self.publication_ts < 0 or self.receipt_ts < 0:
            raise ValueError(
                f"a news instant cannot be negative: "
                f"publication_ts={self.publication_ts} receipt_ts={self.receipt_ts}",
            )

        if self.receipt_ts < self.publication_ts:
            raise ValueError(
                f"the receipt instant {self.receipt_ts} precedes the publication instant "
                f"{self.publication_ts}",
            )

    @property
    def knowledge_date(self) -> int:
        """
        Return the instant the item became knowable to this process: its receipt instant.
        """
        return self.receipt_ts

    @property
    def latency_ns(self) -> int:
        """
        Return the information latency, the interval from publication to receipt.
        """
        return self.receipt_ts - self.publication_ts

    @classmethod
    def received(  # noqa: PLR0913 - the item's declared fields
        cls,
        *,
        item_id: str,
        source: str,
        headline: str,
        publication_ts: int,
        receipt_ts: int,
        symbols: str = "",
        sentiment: float = 0.0,
    ) -> NewsItem:
        """
        Build an item and mirror its two instants onto the custom-data boundary.

        The boundary instants are set rather than left to the caller: `ts_event` is the publication
        instant, because that is when the news event occurred, and `ts_init` is the receipt
        instant, because that is when the datum entered the system. Building through this
        constructor is what keeps the payload, the boundary and the knowledge date consistent.

        Parameters
        ----------
        item_id : str
            The identity of the item, unique within its source.
        source : str
            The feed, wire or venue the item was received from.
        headline : str
            The item's text, as published.
        publication_ts : int
            The instant the item was published, in nanoseconds since the epoch.
        receipt_ts : int
            The instant the item was received by this process, in nanoseconds since the epoch.
        symbols : str, default ""
            The comma-separated symbols the item concerns, empty when it is unscoped.
        sentiment : float, default 0.0
            The item's sentiment score, if the producer scored it.

        Returns
        -------
        NewsItem

        """
        return cls(
            item_id=item_id,
            source=source,
            headline=headline,
            publication_ts=publication_ts,
            receipt_ts=receipt_ts,
            symbols=symbols,
            sentiment=sentiment,
            ts_event=publication_ts,
            ts_init=receipt_ts,
        )
