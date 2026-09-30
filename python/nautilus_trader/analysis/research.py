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
Notebook research over the same catalog and indicator primitives a backtest uses.

The module is a thin view over `ParquetDataCatalog`: it loads instruments and data through the
existing bindings as typed Nautilus objects, converts to a DataFrame by delegating to
`query_catalog` in `nautilus_trader.persistence.catalog_to_df`, replays data in `ts_init` order,
and runs existing indicator objects over that replay. It adds no data path, no indicator
implementation, and no engine of its own, so research code sees exactly what `BacktestNode` sees.
"""

from __future__ import annotations

from collections.abc import Iterable
from collections.abc import Iterator
from pathlib import Path
from typing import Any

from nautilus_trader.model import Bar
from nautilus_trader.model import NautilusDataType
from nautilus_trader.model import QuoteTick
from nautilus_trader.model import TradeTick
from nautilus_trader.persistence import ParquetDataCatalog
from nautilus_trader.persistence.catalog_to_df import CatalogOutput
from nautilus_trader.persistence.catalog_to_df import query_catalog


__all__ = [
    "ResearchData",
    "compute_indicator",
    "replay",
]


def replay[T](data: Iterable[T], *, reverse: bool = False) -> Iterator[T]:
    """
    Yield the given data in `ts_init` order.

    The replay mirrors the order a backtest engine delivers events without constructing an engine,
    so notebook code can reproduce strategy logic as a plain iteration.

    Parameters
    ----------
    data : Iterable[T]
        The typed Nautilus data objects to replay. Each object must expose a `ts_init`.
    reverse : bool, default False
        If True, yield the data in descending `ts_init` order.

    Yields
    ------
    T

    """
    yield from sorted(data, key=lambda item: item.ts_init, reverse=reverse)


def compute_indicator(
    indicator: Any,
    data: Iterable[Any],
    *,
    reverse: bool = False,
) -> list[float]:
    """
    Feed the given data to an existing indicator and return its value after each item.

    The indicator is any object from `nautilus_trader.indicators`: the module dispatches
    `Bar`, `TradeTick`, and `QuoteTick` items to the indicator's own `handle_bar`,
    `handle_trade_tick`, and `handle_quote_tick` methods. No indicator is implemented here.

    Parameters
    ----------
    indicator : Any
        An indicator instance from the existing indicator API.
    data : Iterable[Any]
        The typed Nautilus data objects to feed to the indicator.
    reverse : bool, default False
        If True, feed the data in descending `ts_init` order.

    Returns
    -------
    list[float]
        The indicator `value` after each item, one value per item.

    Raises
    ------
    TypeError
        If an item is not a `Bar`, `TradeTick`, or `QuoteTick`.

    """
    values: list[float] = []

    for item in replay(data, reverse=reverse):
        if isinstance(item, Bar):
            indicator.handle_bar(item)
        elif isinstance(item, TradeTick):
            indicator.handle_trade_tick(item)
        elif isinstance(item, QuoteTick):
            indicator.handle_quote_tick(item)
        else:
            raise TypeError(f"Unsupported indicator input type: {type(item).__name__}")

        values.append(indicator.value)

    return values


class ResearchData:
    """
    A notebook-facing view over a `ParquetDataCatalog`.

    The catalog is opened once by path. Instrument and data loads go through the catalog's existing
    Rust-backed bindings and return typed Nautilus objects, while `to_dataframe` delegates to the
    shared `query_catalog` conversion, so research reads the same bytes a backtest reads.

    Parameters
    ----------
    catalog_path : str or pathlib.Path
        The path to the Parquet catalog.

    """

    def __init__(self, catalog_path: str | Path) -> None:
        """
        Initialize the instance.
        """
        self._catalog = ParquetDataCatalog(str(catalog_path))

    @property
    def catalog(self) -> ParquetDataCatalog:
        """
        The underlying Parquet catalog.
        """
        return self._catalog

    def instruments(
        self,
        instrument_ids: list[str] | None = None,
        *,
        start: int | None = None,
        end: int | None = None,
        where: str | None = None,
        instrument_type: Any | None = None,
    ) -> list[Any]:
        """
        Load typed instruments from the catalog.
        """
        return self._catalog.instruments(
            instrument_ids,
            start,
            end,
            where,
            instrument_type,
        )

    def bars(
        self,
        identifiers: list[str] | None = None,
        *,
        start: int | None = None,
        end: int | None = None,
        where: str | None = None,
    ) -> list[Bar]:
        """
        Load typed bars from the catalog.
        """
        return self._catalog.query_bars(identifiers, start, end, where)

    def quote_ticks(
        self,
        identifiers: list[str] | None = None,
        *,
        start: int | None = None,
        end: int | None = None,
        where: str | None = None,
    ) -> list[QuoteTick]:
        """
        Load typed quote ticks from the catalog.
        """
        return self._catalog.query_quote_ticks(identifiers, start, end, where)

    def trade_ticks(
        self,
        identifiers: list[str] | None = None,
        *,
        start: int | None = None,
        end: int | None = None,
        where: str | None = None,
    ) -> list[TradeTick]:
        """
        Load typed trade ticks from the catalog.
        """
        return self._catalog.query_trade_ticks(identifiers, start, end, where)

    def data(
        self,
        data_type: NautilusDataType,
        identifiers: list[str] | None = None,
        *,
        start: int | None = None,
        end: int | None = None,
        where: str | None = None,
    ) -> list[Any]:
        """
        Load typed data of the given type from the catalog.
        """
        return self._catalog.query(data_type, identifiers, start, end, where)

    def to_dataframe(
        self,
        data_type: Any,
        *,
        identifiers: list[str] | None = None,
        start: Any | None = None,
        end: Any | None = None,
        where: str | None = None,
        output: CatalogOutput = CatalogOutput.PANDAS,
        use_arrow_dtypes: bool = False,
    ) -> Any:
        """
        Convert a catalog query to a DataFrame through the shared `query_catalog` helper.
        """
        return query_catalog(
            self._catalog,
            data_type,
            output=output,
            identifiers=identifiers,
            start=start,
            end=end,
            where=where,
            use_arrow_dtypes=use_arrow_dtypes,
        )

    def replay[T](self, data: Iterable[T], *, reverse: bool = False) -> Iterator[T]:
        """
        Yield the given data in `ts_init` order.
        """
        return replay(data, reverse=reverse)

    def indicator(
        self,
        indicator: Any,
        data: Iterable[Any],
        *,
        reverse: bool = False,
    ) -> list[float]:
        """
        Run an existing indicator over the given data in `ts_init` order.
        """
        return compute_indicator(indicator, data, reverse=reverse)
