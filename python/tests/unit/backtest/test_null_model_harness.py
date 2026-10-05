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
Tests for the null-model harness: a generated synthetic flow is fed into a backtest run.

A trend-following probe earns more, on the same seeds, when the reference process carries interval
memory than when it does not, so the result is reported as a distribution over seeds rather than as
one number. The runs are deterministic: the same seed reproduces the same result exactly.
"""

from __future__ import annotations

import logging
from decimal import Decimal
from statistics import quantiles

from nautilus_trader.backtest import BacktestEngine
from nautilus_trader.backtest import BacktestEngineConfig
from nautilus_trader.backtest import SyntheticFlowConfig
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.model import AccountType
from nautilus_trader.model import Bar
from nautilus_trader.model import BarType
from nautilus_trader.model import Currency
from nautilus_trader.model import InstrumentId
from nautilus_trader.model import Money
from nautilus_trader.model import OmsType
from nautilus_trader.model import OrderSide
from nautilus_trader.model import Price
from nautilus_trader.model import Quantity
from nautilus_trader.model import Venue
from nautilus_trader.trading import Strategy
from nautilus_trader.trading import StrategyConfig
from tests.providers import TestInstrumentProvider


logger = logging.getLogger(__name__)

VENUE = Venue("BINANCE")
CURRENCY = Currency.from_str("USDT")
STARTING_BALANCE = 10_000_000.0
PRICE_BASE = Decimal("1000.00")
PRICE_STEP = Decimal("0.01")
TRADE_SIZE = Quantity.from_decimal_dp(Decimal("1.000"), 6)
START_NS = 1_600_000_000_000_000_000
BAR_COUNT = 512
PERSISTENT_HURST = 0.6
MEMORYLESS_HURST = 0.51
SEEDS = (1, 2, 3, 4, 5, 6, 7, 8)


class TrendProbe(Strategy):
    """
    Trade the direction of the last bar; a rule whose result depends on the process's memory.
    """

    def __init__(self, config: StrategyConfig) -> None:
        """
        Initialize the instance.
        """
        super().__init__(config)
        self.bar_type: BarType | None = None
        self.instrument_id: InstrumentId | None = None
        self.previous_close: float | None = None
        self.direction = 0

    def configure(self, bar_type: str, instrument_id: str) -> None:
        """
        Configure the probe before the run.
        """
        self.bar_type = BarType.from_str(bar_type)
        self.instrument_id = InstrumentId.from_str(instrument_id)

    def on_start(self) -> None:
        """
        On start.
        """
        assert self.bar_type is not None
        self.subscribe_bars(self.bar_type)

    def on_bar(self, bar: Bar) -> None:
        """
        On bar.
        """
        assert self.instrument_id is not None
        close = bar.close.as_double()
        if self.previous_close is not None:
            direction = 1 if close > self.previous_close else -1
            if direction != self.direction:
                self.close_all_positions(self.instrument_id)
                self.direction = direction
                self.submit_order(
                    self.order_factory.market(
                        instrument_id=self.instrument_id,
                        order_side=OrderSide.BUY if direction > 0 else OrderSide.SELL,
                        quantity=TRADE_SIZE,
                    ),
                )
        self.previous_close = close


def bars_from_prices(instrument: object, bar_type: BarType, prices: list[float]) -> list[Bar]:
    """
    Build one-minute bars from a price path, guarding the OHLC invariants.
    """
    bars = []
    previous = PRICE_BASE

    for index, level in enumerate(prices):
        close = PRICE_BASE + Decimal(str(level)).quantize(PRICE_STEP)
        bars.append(
            Bar(
                bar_type=bar_type,
                open=Price.from_decimal_dp(previous, instrument.price_precision),
                high=Price.from_decimal_dp(
                    max(previous, close) + PRICE_STEP,
                    instrument.price_precision,
                ),
                low=Price.from_decimal_dp(
                    min(previous, close) - PRICE_STEP,
                    instrument.price_precision,
                ),
                close=Price.from_decimal_dp(close, instrument.price_precision),
                volume=Quantity.from_decimal_dp(Decimal("5.0"), instrument.size_precision),
                ts_event=START_NS + index * 60_000_000_000,
                ts_init=START_NS + index * 60_000_000_000,
            ),
        )
        previous = close

    return bars


def run_flow(hurst: float, seed: int) -> float:
    """
    Run the probe over a generated flow and return the result statistic as net PnL in USDT.
    """
    instrument = TestInstrumentProvider.btcusdt_binance()
    flow = SyntheticFlowConfig(hurst, 0.5, BAR_COUNT, seed).generate()
    bar_type = BarType.from_str(f"{instrument.id}-1-MINUTE-LAST-EXTERNAL")

    engine = BacktestEngine(BacktestEngineConfig(bypass_logging=True, run_analysis=False))
    engine.add_venue(
        venue=VENUE,
        oms_type=OmsType.NETTING,
        account_type=AccountType.MARGIN,
        base_currency=CURRENCY,
        starting_balances=[Money(STARTING_BALANCE, CURRENCY)],
        fee_model=MakerTakerFeeModel(maker_rate=Decimal(0), taker_rate=Decimal(0)),
    )
    engine.add_instrument(instrument)
    engine.add_data(bars_from_prices(instrument, bar_type, flow.prices))

    strategy = TrendProbe(StrategyConfig())
    strategy.configure(str(bar_type), str(instrument.id))
    engine.add_strategy(strategy)
    engine.run()

    account = engine.portfolio.account(VENUE)
    assert account is not None
    equity = account.balance_total(CURRENCY).as_double()
    engine.dispose()

    return equity - STARTING_BALANCE


def report(label: str, values: list[float]) -> list[float]:
    """
    Log a distribution and return its quartiles.
    """
    ordered = sorted(values)
    quartiles = quantiles(values, n=4)
    logger.info(
        "%s: n=%d min=%.2f q1=%.2f median=%.2f q3=%.2f max=%.2f",
        label,
        len(values),
        ordered[0],
        quartiles[0],
        quartiles[1],
        quartiles[2],
        ordered[-1],
    )
    return quartiles


def test_the_same_seed_reproduces_the_same_result() -> None:
    """
    Test that a run is reproducible end to end from the engine seed.
    """
    first = run_flow(PERSISTENT_HURST, seed=42)
    second = run_flow(PERSISTENT_HURST, seed=42)

    assert first == second


def test_the_result_is_reported_as_a_distribution_over_seeds() -> None:
    """
    Test that the harness reports a spread, and that it separates the two regimes.
    """
    persistent = [run_flow(PERSISTENT_HURST, seed) for seed in SEEDS]
    memoryless = [run_flow(MEMORYLESS_HURST, seed) for seed in SEEDS]

    persistent_quartiles = report("persistent flow", persistent)
    memoryless_quartiles = report("memoryless flow", memoryless)

    assert persistent_quartiles[1] > memoryless_quartiles[1], (
        f"persistent median {persistent_quartiles[1]:.2f} did not exceed "
        f"memoryless median {memoryless_quartiles[1]:.2f}"
    )
    assert max(persistent) > min(persistent)
