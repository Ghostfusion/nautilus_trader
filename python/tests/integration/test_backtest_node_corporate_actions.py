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
End-to-end tests for the `BacktestNode` corporate action adjustment stage.

The `BacktestNode` is the only path that applies the opt-in adjustment stage: it loads raw bars
from a catalog, converts their prices between representations through the instrument's corporate
action series, and appends the actions as data at their effective instants.

These tests run the node against a temporary Parquet catalog holding bars plus a 4:1 split and a
cash dividend, and observe the prices a strategy receives on `on_bar` and the run's processed
record count and final boundary. The node does not expose a canonical backtest result (its
`run()` returns `BacktestResult` objects and only the engine exposes the canonical projection), so
the adjustment is verified through the node's observable outputs rather than the regression
harness. A subscribed strategy observes the actions themselves on `on_corporate_action`, but only
when the data is loaded with an adjustment: without one the action records are never loaded, so a
subscriber receives nothing.
"""

from __future__ import annotations

from decimal import Decimal
from pathlib import Path
from typing import Any

from nautilus_trader.backtest import BacktestDataConfig
from nautilus_trader.backtest import BacktestEngineConfig
from nautilus_trader.backtest import BacktestNode
from nautilus_trader.backtest import BacktestRunConfig
from nautilus_trader.backtest import BacktestVenueConfig
from nautilus_trader.backtest import DataAdjustment
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.model import Bar
from nautilus_trader.model import BarType
from nautilus_trader.model import CorporateAction
from nautilus_trader.model import CorporateActionType
from nautilus_trader.model import InstrumentId
from nautilus_trader.model import NautilusDataType
from nautilus_trader.model import Price
from nautilus_trader.model import PriceRepresentation
from nautilus_trader.model import Quantity
from nautilus_trader.persistence import ParquetDataCatalog
from nautilus_trader.trading import Strategy
from nautilus_trader.trading import StrategyConfig
from tests.providers import TestInstrumentProvider


INSTRUMENT = TestInstrumentProvider.aapl_equity()
INSTRUMENT_ID: InstrumentId = INSTRUMENT.id
BAR_TYPE = BarType.from_str(f"{INSTRUMENT_ID}-1-MINUTE-LAST-EXTERNAL")

MINUTE_NS = 60_000_000_000
TS_START = 1_704_067_200_000_000_000  # 2024-01-01T00:00:00Z, a minute boundary.
SPLIT_NS = TS_START + MINUTE_NS + 30_000_000_000
DIVIDEND_NS = TS_START + 2 * MINUTE_NS + 30_000_000_000
# A dividend effective after the final bar, used to prove the action data is delivered at its
# effective instant: the run boundary can only reach it if the action record is re-stamped.
TRAILING_DIVIDEND_NS = TS_START + 3 * MINUTE_NS + 30_000_000_000
# The action records are announced well before they take effect; `ts_event`/`ts_init` must not be
# mistaken for the effective instant.
ANNOUNCE_NS = TS_START - 10 * MINUTE_NS

# Raw catalog prices: two bars before the split, two after it and before the dividend.
RAW_PRICES = ("400.00", "400.00", "200.00", "200.00")
# Adjusted: 400 * (1/4) - 2.50 = 97.50 before both actions; 200 - 2.50 = 197.50 after the split
# but before the dividend; 200.00 after both.
ADJUSTED_PRICES = ("97.50", "97.50", "197.50", "200.00")


class RecordingConfig(StrategyConfig):
    """
    Configure the bar-recording strategy.
    """

    def __init__(self, *, bar_type: str) -> None:
        """
        Initialize the instance.
        """
        super().__init__()
        self.bar_type = bar_type


class BarRecordingStrategy(Strategy):
    """
    Subscribe to one bar type and record every bar close the node delivers.
    """

    def __init__(self, config: RecordingConfig) -> None:
        """
        Initialize the instance.
        """
        super().__init__(config)
        self._bar_type = BarType.from_str(config.bar_type)
        self.closes: list[tuple[int, Decimal]] = []
        self.actions: list[CorporateAction] = []

    def on_start(self) -> None:
        """
        On start.
        """
        self.subscribe_bars(self._bar_type)
        self.subscribe_corporate_actions(INSTRUMENT_ID)

    def on_bar(self, bar: Bar) -> None:
        """
        On bar.
        """
        self.closes.append((bar.ts_event, bar.close.as_decimal()))

    def on_corporate_action(self, action: CorporateAction) -> None:
        """
        On corporate action.
        """
        self.actions.append(action)


def _bars(prices: tuple[str, ...]) -> list[Bar]:
    bars: list[Bar] = []
    for index, value in enumerate(prices):
        ts = TS_START + index * MINUTE_NS
        price = Price.from_str(value)
        bars.append(
            Bar(
                BAR_TYPE,
                price,
                price,
                price,
                price,
                Quantity.from_int(100),
                ts,
                ts,
            ),
        )
    return bars


def _corporate_actions(
    split_ns: int = SPLIT_NS,
    dividend_ns: int = DIVIDEND_NS,
) -> list[CorporateAction]:
    return [
        CorporateAction(
            INSTRUMENT_ID,
            CorporateActionType.SPLIT,
            Decimal(4),
            None,
            split_ns,
            ANNOUNCE_NS,
            ANNOUNCE_NS,
        ),
        CorporateAction(
            INSTRUMENT_ID,
            CorporateActionType.DIVIDEND,
            Decimal("2.50"),
            None,
            dividend_ns,
            ANNOUNCE_NS,
            ANNOUNCE_NS,
        ),
    ]


def _run_node(
    catalog_path: Path,
    prices: tuple[str, ...],
    actions: list[CorporateAction],
    adjustment: DataAdjustment | None,
) -> tuple[BarRecordingStrategy, Any]:
    catalog = ParquetDataCatalog(str(catalog_path))
    catalog.write_instruments([INSTRUMENT])
    catalog.write_bars(_bars(prices))
    catalog.write_corporate_actions(actions)

    venue = BacktestVenueConfig(
        name="XNAS",
        oms_type="NETTING",
        account_type="CASH",
        starting_balances=["1_000_000 USD"],
        book_type="L1_MBP",
        fee_model=MakerTakerFeeModel(maker_rate=Decimal(0), taker_rate=Decimal(0)),
    )
    data = BacktestDataConfig(
        data_type=NautilusDataType.Bar,
        catalog_path=str(catalog_path),
        instrument_id=INSTRUMENT_ID,
        bar_types=[str(BAR_TYPE)],
        data_adjustment=adjustment,
    )
    config = BacktestRunConfig(
        venues=[venue],
        data=[data],
        engine=BacktestEngineConfig(bypass_logging=True, run_analysis=False),
        dispose_on_completion=False,
    )
    node = BacktestNode([config])
    node.build()
    strategy = BarRecordingStrategy(RecordingConfig(bar_type=str(BAR_TYPE)))
    node.add_strategy(config.id, strategy)
    try:
        result = node.run()[0]
    finally:
        node.dispose()
    return strategy, result


def test_node_adjusts_bars_from_raw_to_adjusted(tmp_path: Path) -> None:
    """
    Test the node converts catalog raw bars into the adjusted series.
    """
    strategy, result = _run_node(
        tmp_path,
        RAW_PRICES,
        _corporate_actions(),
        DataAdjustment(PriceRepresentation.RAW, PriceRepresentation.ADJUSTED),
    )
    closes = strategy.closes

    assert [value for _, value in closes] == [Decimal(value) for value in ADJUSTED_PRICES]
    assert [ts for ts, _ in closes] == [TS_START + index * MINUTE_NS for index in range(4)]
    # Four bars plus the two appended action records were processed.
    assert result.iterations == 6
    assert result.backtest_end == TS_START + 3 * MINUTE_NS


def test_node_adjusts_bars_from_adjusted_to_raw(tmp_path: Path) -> None:
    """
    Test the node converts catalog adjusted bars back into the raw series.
    """
    strategy, _ = _run_node(
        tmp_path,
        ADJUSTED_PRICES,
        _corporate_actions(),
        DataAdjustment(PriceRepresentation.ADJUSTED, PriceRepresentation.RAW),
    )

    assert [value for _, value in strategy.closes] == [Decimal(value) for value in RAW_PRICES]


def test_node_default_path_leaves_raw_prices_untouched(tmp_path: Path) -> None:
    """
    Test an unconfigured node passes the raw catalog bars through exactly.
    """
    strategy, result = _run_node(tmp_path, RAW_PRICES, _corporate_actions(), None)

    assert [value for _, value in strategy.closes] == [Decimal(value) for value in RAW_PRICES]
    # The action records are only appended when the adjustment is active.
    assert result.iterations == 4
    assert result.backtest_end == TS_START + 3 * MINUTE_NS


def test_node_delivers_corporate_actions_at_their_effective_instants(tmp_path: Path) -> None:
    """
    Test the node replays each action as data at its effective instant, not its announcement.
    """
    strategy, result = _run_node(
        tmp_path,
        RAW_PRICES,
        _corporate_actions(dividend_ns=TRAILING_DIVIDEND_NS),
        DataAdjustment(PriceRepresentation.RAW, PriceRepresentation.ADJUSTED),
    )

    # All four bars precede the trailing dividend, so it is subtracted from each; the split only
    # affects the first two.
    expected = ("97.50", "97.50", "197.50", "197.50")
    assert [value for _, value in strategy.closes] == [Decimal(value) for value in expected]
    assert result.iterations == 6
    # The final boundary is the trailing action's effective instant. It can only be reached if the
    # action record was re-stamped to its effective instant; the announcement is earlier.
    assert result.backtest_end == TRAILING_DIVIDEND_NS
    assert result.backtest_end != ANNOUNCE_NS


def test_node_delivers_corporate_actions_to_a_subscribed_strategy(tmp_path: Path) -> None:
    """
    Test a subscribed strategy receives each action, ordered and re-stamped to its effective
    instant, when the data is loaded with an adjustment.
    """
    strategy, _ = _run_node(
        tmp_path,
        RAW_PRICES,
        _corporate_actions(),
        DataAdjustment(PriceRepresentation.RAW, PriceRepresentation.ADJUSTED),
    )

    received = [
        (str(action.action), action.value, action.effective_ns, action.ts_init, action.ts_event)
        for action in strategy.actions
    ]

    assert received == [
        ("SPLIT", Decimal(4), SPLIT_NS, SPLIT_NS, ANNOUNCE_NS),
        ("DIVIDEND", Decimal("2.50"), DIVIDEND_NS, DIVIDEND_NS, ANNOUNCE_NS),
    ]


def test_node_without_adjustment_delivers_no_corporate_actions(tmp_path: Path) -> None:
    """
    Test a subscribed strategy receives nothing when the action records are not loaded.
    """
    strategy, result = _run_node(tmp_path, RAW_PRICES, _corporate_actions(), None)

    assert strategy.actions == []
    assert result.iterations == 4


def _canonical_digest(catalog_path: Path, run_id: str) -> str:
    catalog = ParquetDataCatalog(str(catalog_path))
    catalog.write_instruments([INSTRUMENT])
    catalog.write_bars(_bars(RAW_PRICES))
    catalog.write_corporate_actions(_corporate_actions())

    venue = BacktestVenueConfig(
        name="XNAS",
        oms_type="NETTING",
        account_type="CASH",
        starting_balances=["1_000_000 USD"],
        book_type="L1_MBP",
        fee_model=MakerTakerFeeModel(maker_rate=Decimal(0), taker_rate=Decimal(0)),
    )
    data = BacktestDataConfig(
        data_type=NautilusDataType.Bar,
        catalog_path=str(catalog_path),
        instrument_id=INSTRUMENT_ID,
        bar_types=[str(BAR_TYPE)],
        data_adjustment=DataAdjustment(PriceRepresentation.RAW, PriceRepresentation.ADJUSTED),
    )
    config = BacktestRunConfig(
        id=run_id,
        venues=[venue],
        data=[data],
        engine=BacktestEngineConfig(bypass_logging=True, run_analysis=False),
        dispose_on_completion=False,
    )
    node = BacktestNode([config])
    node.build()
    try:
        node.run()
        return node.get_engine_canonical_result(run_id).digest()
    finally:
        node.dispose()


def test_node_canonical_result_is_reproducible_across_catalog_directories(
    tmp_path: Path,
) -> None:
    """
    Test a catalog run projects the same canonical document from a different catalog directory.
    """
    first_dir = tmp_path / "first"
    second_dir = tmp_path / "second"
    first_dir.mkdir()
    second_dir.mkdir()

    first = _canonical_digest(first_dir, "corporate-action-digest")
    second = _canonical_digest(second_dir, "corporate-action-digest")

    assert first.startswith("blake3:")
    assert first == second
