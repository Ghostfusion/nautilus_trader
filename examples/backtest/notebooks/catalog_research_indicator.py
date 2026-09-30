"""
Example of notebook research over a shared Parquet catalog.
"""

# ---
# jupyter:
#   jupytext:
#     formats: py:percent
#     text_representation:
#       extension: .py
#       format_name: percent
#       format_version: '1.3'
#       jupytext_version: 1.19.0
#   kernelspec:
#     display_name: Python 3 (ipykernel)
#     language: python
#     name: python3
# ---


# %% [markdown]
# # Research over a shared catalog
#
# Load a Parquet catalog through `ResearchData`, replay the bars in `ts_init` order, and run an
# existing indicator over them. The bar path and the DataFrame conversion are the same catalog and
# `query_catalog` code a `BacktestNode` uses, so the notebook cannot drift into a second data path.
#
# The repository ships no ready-made Parquet catalog of bars, so this example first writes one to a
# temporary directory from the tracked minute-bar CSV, then opens it exactly as a backtest would.

# %%
from pathlib import Path
from tempfile import TemporaryDirectory

from nautilus_trader.analysis.research import ResearchData
from nautilus_trader.indicators import ExponentialMovingAverage
from nautilus_trader.model import BarType
from nautilus_trader.model import NautilusDataType
from nautilus_trader.persistence import ParquetDataCatalog
from nautilus_trader.testkit.providers import TestDataProvider
from nautilus_trader.testkit.providers import TestInstrumentProvider


# %%
if __name__ == "__main__":
    instrument = TestInstrumentProvider.btcusdt_binance()
    bar_type = BarType.from_str(f"{instrument.id}-1-MINUTE-LAST-EXTERNAL")
    bars = TestDataProvider.bars_from_binance_csv(
        instrument,
        bar_type=bar_type,
        csv_name="btc-perp-20211231-20220201_1m.csv",
        max_rows=120,
    )

    with TemporaryDirectory() as tmp_dir:
        catalog_path = Path(tmp_dir)
        catalog = ParquetDataCatalog(str(catalog_path))
        catalog.write_instruments([instrument])
        catalog.write_bars(bars)

        research = ResearchData(catalog_path)

        loaded_bars = research.bars([str(bar_type)])
        print(f"loaded {len(loaded_bars)} bars from {catalog_path}")

        frame = research.to_dataframe(NautilusDataType.Bar, identifiers=[str(bar_type)])
        print(frame[["ts_init", "close"]].head())

        values = research.indicator(ExponentialMovingAverage(10), loaded_bars)
        print(f"ema_10 values: {values}")
