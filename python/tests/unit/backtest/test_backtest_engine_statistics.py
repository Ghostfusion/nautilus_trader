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
Test backtest engine statistics behavior.
"""

import math
from decimal import Decimal

import pandas as pd

from nautilus_trader.analysis import create_tearsheet_from_stats
from nautilus_trader.backtest import BacktestEngine
from nautilus_trader.backtest import BacktestEngineConfig
from nautilus_trader.backtest import BacktestResult
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.model import AccountType
from nautilus_trader.model import Currency
from nautilus_trader.model import InstrumentId
from nautilus_trader.model import Money
from nautilus_trader.model import OmsType
from nautilus_trader.model import OrderSide
from nautilus_trader.model import Price
from nautilus_trader.model import Quantity
from nautilus_trader.model import QuoteTick
from nautilus_trader.model import Venue
from nautilus_trader.trading import Strategy
from nautilus_trader.trading import StrategyConfig
from tests.providers import TestInstrumentProvider


USDT = Currency.from_str("USDT")
ETHUSDT = TestInstrumentProvider.ethusdt_binance()

# 2020-09-13T12:26:40Z, so generated client order IDs are deterministic.
BASE_NS = 1_600_000_000_000_000_000
QUOTE_COUNT = 6
# A multiple of the instrument's lot size, so the venue accepts the order.
TRADE_SIZE = "1.000"
STARTING_BALANCE = 1_000_000.0

COST_ROW = "Cost (basis points of turnover)"
GROSS_ROW = "Gross Return"
NET_ROW = "Net Return"
COMMISSION_ROW = "Total Commissions"
TURNOVER_ROW = "Total Turnover"


def _float_maps_equal(a: dict[str, float], b: dict[str, float]) -> bool:
    """
    Return True if two str->float dicts are equal, treating NaN as equal to NaN.
    """
    if a.keys() != b.keys():
        return False
    for key in a:
        va, vb = a[key], b[key]
        if math.isnan(va) and math.isnan(vb):
            continue
        if va != vb:
            return False
    return True


def _nested_float_maps_equal(
    a: dict[str, dict[str, float]],
    b: dict[str, dict[str, float]],
) -> bool:
    """
    Return True if two str->str->float dicts are equal, treating NaN as equal to NaN.
    """
    if a.keys() != b.keys():
        return False
    return all(_float_maps_equal(a[key], b[key]) for key in a)


def _engine_with_account() -> BacktestEngine:
    engine = BacktestEngine(BacktestEngineConfig(bypass_logging=True))
    engine.add_venue(
        venue=Venue("SIM"),
        oms_type=OmsType.HEDGING,
        account_type=AccountType.MARGIN,
        base_currency=Currency.from_str("USD"),
        starting_balances=[Money(1_000_000.0, Currency.from_str("USD"))],
        fee_model=MakerTakerFeeModel(
            maker_rate=Decimal(0),
            taker_rate=Decimal(0),
        ),
    )
    return engine


def test_engine_exposes_portfolio_statistics() -> None:
    """
    Test engine exposes portfolio statistics.
    """
    engine = _engine_with_account()
    engine.run()
    stats = engine.portfolio.statistics()
    assert isinstance(stats.pnls, dict)
    assert isinstance(stats.returns, dict)
    assert isinstance(stats.general, dict)
    engine.dispose()


def test_engine_portfolio_statistics_equals_result() -> None:
    """
    Test engine portfolio statistics equals result.
    """
    engine = _engine_with_account()
    engine.run()
    stats = engine.portfolio.statistics()
    result = engine.get_result()
    assert _nested_float_maps_equal(stats.pnls, result.stats_pnls)
    assert _float_maps_equal(stats.returns, result.stats_returns)
    assert _float_maps_equal(stats.general, result.stats_general)
    engine.dispose()


def _quotes(count: int = QUOTE_COUNT) -> list[QuoteTick]:
    """
    Build a rising quote series for the test instrument.
    """
    ticks = []

    for i in range(count):
        mid = Decimal("2000.00") + Decimal("1.00") * i
        ticks.append(
            QuoteTick(
                instrument_id=ETHUSDT.id,
                bid_price=Price.from_decimal_dp(mid - Decimal("0.05"), ETHUSDT.price_precision),
                ask_price=Price.from_decimal_dp(mid + Decimal("0.05"), ETHUSDT.price_precision),
                bid_size=Quantity.from_decimal_dp(Decimal(10), ETHUSDT.size_precision),
                ask_size=Quantity.from_decimal_dp(Decimal(10), ETHUSDT.size_precision),
                ts_event=BASE_NS + (i * 1_000_000_000),
                ts_init=BASE_NS + (i * 1_000_000_000),
            ),
        )

    return ticks


class _OneShotBuyConfig(StrategyConfig):
    """
    Configure the one-shot buying strategy.
    """

    def __init__(self, *, instrument_id: InstrumentId, trade_size: str) -> None:
        """
        Initialize the instance.
        """
        super().__init__()
        self.instrument_id = instrument_id
        self.trade_size = trade_size


class _OneShotBuyStrategy(Strategy):
    """
    Submit one market buy on the first quote observed.
    """

    def __init__(self, config: _OneShotBuyConfig) -> None:
        """
        Initialize the instance.
        """
        super().__init__(config)
        self._instrument_id = config.instrument_id
        self._quantity = Quantity.from_str(config.trade_size)
        self.orders_submitted = 0

    def on_start(self) -> None:
        """
        Subscribe to the instrument's quotes.
        """
        self.subscribe_quotes(self._instrument_id)

    def on_quote(self, _quote: QuoteTick) -> None:
        """
        Submit one market order on the first quote seen.
        """
        if self.orders_submitted > 0:
            return

        self.orders_submitted += 1
        self.submit_order(
            self.order_factory.market(
                instrument_id=self._instrument_id,
                order_side=OrderSide.BUY,
                quantity=self._quantity,
            ),
        )


def _fee_engine(maker_rate: str, taker_rate: str) -> BacktestEngine:
    """
    Build an engine whose venue charges the given maker and taker rates.
    """
    engine = BacktestEngine(BacktestEngineConfig(bypass_logging=True, run_analysis=False))
    engine.add_venue(
        venue=Venue("BINANCE"),
        oms_type=OmsType.NETTING,
        account_type=AccountType.MARGIN,
        base_currency=USDT,
        starting_balances=[Money(STARTING_BALANCE, USDT)],
        fee_model=MakerTakerFeeModel(
            maker_rate=Decimal(maker_rate),
            taker_rate=Decimal(taker_rate),
        ),
    )
    engine.add_instrument(ETHUSDT)

    return engine


def _one_shot_run(maker_rate: str, taker_rate: str) -> BacktestResult:
    """
    Run the one-shot buy scenario against a venue charging the given rates.
    """
    engine = _fee_engine(maker_rate, taker_rate)
    strategy = _OneShotBuyStrategy(
        _OneShotBuyConfig(instrument_id=ETHUSDT.id, trade_size=TRADE_SIZE),
    )

    try:
        engine.add_strategy(strategy)
        engine.add_data(_quotes())
        engine.run()
        result = engine.get_result()
    finally:
        engine.dispose()

    assert strategy.orders_submitted == 1
    assert result.total_orders > 0
    assert result.total_positions > 0

    return result


def test_default_report_prints_the_cost_of_a_zero_fee_run() -> None:
    """
    Test a run that pays no fee reports a zero cost and identical gross and net returns.
    """
    result = _one_shot_run("0", "0")
    stats_returns = result.stats_returns

    assert COST_ROW in stats_returns
    assert GROSS_ROW in stats_returns
    assert NET_ROW in stats_returns
    assert stats_returns[COST_ROW] == 0.0
    assert stats_returns[GROSS_ROW] == stats_returns[NET_ROW]
    assert stats_returns[GROSS_ROW] != 0.0
    assert COMMISSION_ROW in result.stats_general
    assert TURNOVER_ROW in result.stats_general


def test_default_report_prints_the_cost_of_a_fee_run() -> None:
    """
    Test a run that pays a fee reports a positive cost and net below gross.
    """
    result = _one_shot_run("0.00002", "0.00002")
    stats_returns = result.stats_returns
    stats_general = result.stats_general

    cost_bps = stats_returns[COST_ROW]
    gross = stats_returns[GROSS_ROW]
    net = stats_returns[NET_ROW]
    commission = stats_general[COMMISSION_ROW]
    turnover = stats_general[TURNOVER_ROW]

    assert cost_bps > 0.0
    assert net < gross
    assert commission > 0.0
    assert turnover > 0.0
    # Two different ratios over the same commission: the gap is a return over the frame's
    # starting equity, the printed cost is a rate over the frame's turnover.
    assert math.isclose(
        (gross - net) * STARTING_BALANCE,
        cost_bps * turnover / 10_000.0,
        rel_tol=1e-9,
    )


def test_tearsheet_renders_the_cost_row() -> None:
    """
    Test the rendered table prints the cost row beside the returns.
    """
    result = _one_shot_run("0.00002", "0.00002")

    html = create_tearsheet_from_stats(
        stats_pnls=result.stats_pnls,
        stats_returns=result.stats_returns,
        stats_general=result.stats_general,
        returns=pd.Series(result.returns_series),
        output_path=None,
    )

    assert html is not None
    assert COST_ROW in html
    assert GROSS_ROW in html
    assert NET_ROW in html
