#!/usr/bin/env python3
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
Example of a validity-gated walk-forward strategy.

The rules are a port of the petrocurrency rule in the repository `je-suis-tm/quant-trading`
(Apache-2.0): on every bar, refit a straight line that explains the traded price from a benchmark
that carries the same economic story, refuse to trade unless the fit clears a stated strength
floor, and take a two-standard-deviation break of the fitted value when it does. The position is
closed on a time cap or on an absolute stop, and the fit is refreshed on the next bar.

Two instruments are simulated: a currency futures contract whose level is driven by the price of a
commodity, and the commodity futures contract itself. The relationship holds for the first part of
the sample and breaks in the second, so the strength gate can be seen closing before the trading
rule ever runs.

The price paths are generated from a fixed formula of the bar index rather than at random, so this
example prints the same numbers on every run. They are synthetic and mean nothing about any real
market.
"""

import math
from collections import deque
from datetime import UTC
from datetime import datetime
from decimal import Decimal

from nautilus_trader.backtest import BacktestEngine
from nautilus_trader.config import BacktestEngineConfig
from nautilus_trader.config import StrategyConfig
from nautilus_trader.core.datetime import dt_to_unix_nanos
from nautilus_trader.execution import PerContractFeeModel
from nautilus_trader.model import AccountType
from nautilus_trader.model import AssetClass
from nautilus_trader.model import Bar
from nautilus_trader.model import BarType
from nautilus_trader.model import Currency
from nautilus_trader.model import FuturesContract
from nautilus_trader.model import InstrumentId
from nautilus_trader.model import Money
from nautilus_trader.model import OmsType
from nautilus_trader.model import OrderSide
from nautilus_trader.model import Price
from nautilus_trader.model import Quantity
from nautilus_trader.model import Symbol
from nautilus_trader.model import TraderId
from nautilus_trader.model import Venue
from nautilus_trader.trading import Strategy


USD = Currency.from_str("USD")

# The sample is split: the traded price follows the benchmark up to this bar and stops following
# it afterwards, which is what closes the strength gate.
BREAK_AT = 280
BAR_COUNT = 400


def pseudo_noise(index: int) -> float:
    """
    Return a deterministic value between minus one and one for a bar index.

    The example needs variation without a random number generator, so that repeated runs are
    identical. The value is not a sample from any distribution and carries no meaning of its own.
    """
    return math.sin(index * 12.9898) * math.cos(index * 78.233)


def synthetic_prices() -> tuple[list[float], list[float]]:
    """
    Return the benchmark and traded price paths, one value per bar.
    """
    benchmark = []
    traded = []

    for index in range(BAR_COUNT):
        # A smooth two-cycle commodity path with a small amount of texture.
        crude = (
            70.0
            + 6.0 * math.sin(index / 40.0)
            + 3.0 * math.sin(index / 13.0)
            + 0.4 * pseudo_noise(index)
        )
        # An occasional one-bar jump, taken from an aperiodic sequence of the bar index, so the
        # residual distribution has a tail and a two-standard-deviation break is reachable rather
        # than a rounding error.
        shock = 0.0040 if pseudo_noise(index * 13) > 0.6 else 0.0

        if index < BREAK_AT:
            level = 1.20 + 0.0020 * (crude - 70.0) + 0.00015 * pseudo_noise(index * 7) + shock
        else:
            # The currency stops following the benchmark: its level now oscillates at a frequency
            # far above anything the benchmark does, so a straight line through the benchmark
            # explains almost none of it and the gate closes.
            level = 1.20 + 0.00030 * math.sin(index * 2.7) + shock

        benchmark.append(crude)
        traded.append(level)

    return benchmark, traded


def create_contract(
    venue: Venue,
    symbol: str,
    underlying: str,
    price_precision: int,
    price_increment: str,
) -> FuturesContract:
    """
    Return a futures contract on the simulated venue.
    """
    instrument_symbol = Symbol(symbol)

    return FuturesContract(
        instrument_id=InstrumentId(instrument_symbol, venue),
        raw_symbol=instrument_symbol,
        asset_class=AssetClass.FX,
        currency=USD,
        price_precision=price_precision,
        price_increment=Price.from_str(price_increment),
        multiplier=Quantity.from_int(1000),
        lot_size=Quantity.from_int(1),
        underlying=underlying,
        activation_ns=0,
        expiration_ns=int(datetime(2026, 12, 17, 14, 16, tzinfo=UTC).timestamp() * 1e9),
        ts_event=0,
        ts_init=0,
        margin_init=Decimal("0.10"),
        margin_maint=Decimal("0.05"),
        exchange="SIM",
    )


class GatedWalkForwardConfig(StrategyConfig):
    """
    Configuration for the validity-gated walk-forward strategy.
    """

    def __init__(
        self,
        *,
        instrument_id: InstrumentId,
        benchmark_id: InstrumentId,
        bar_type: BarType,
        benchmark_bar_type: BarType,
        trade_size: Decimal,
        train_len: int = 50,
        rsquared_threshold: float = 0.7,
        sigma_multiple: float = 2.0,
        holding_threshold: int = 10,
        stop_points: float = 0.0040,
        **_kwargs: object,
    ) -> None:
        """
        Initialize a new instance.
        """
        super().__init__()
        self.instrument_id = instrument_id
        self.benchmark_id = benchmark_id
        self.bar_type = bar_type
        self.benchmark_bar_type = benchmark_bar_type
        self.trade_size = trade_size
        self.train_len = train_len
        self.rsquared_threshold = rsquared_threshold
        self.sigma_multiple = sigma_multiple
        self.holding_threshold = holding_threshold
        self.stop_points = stop_points


class GatedWalkForward(Strategy):
    """
    Trade a two-standard-deviation break of a refitted straight line.

    The line is refitted on every bar from a rolling window, and no trade is taken unless the fit
    clears a stated strength floor. A break above the fitted value is bought and a break below it
    is sold, which is the direction convention of the source rule: the break is treated as the
    start of a move rather than as a reversion to the line.
    """

    def __init__(self, config: GatedWalkForwardConfig) -> None:
        """
        Initialize a new instance.
        """
        super().__init__(config)
        self._pairs: deque[tuple[float, float]] = deque(maxlen=config.train_len)
        self._benchmark_closes: dict[int, float] = {}
        self._bars_held = 0
        self.evaluations = 0
        self.gate_open = 0
        self.entries = 0

    def on_start(self) -> None:
        """
        Subscribe to the bars of both instruments.
        """
        self.subscribe_bars(self.config.benchmark_bar_type)
        self.subscribe_bars(self.config.bar_type)

    def on_bar(self, bar: Bar) -> None:
        """
        Refit the line, apply the strength gate, and manage the open position.
        """
        if bar.bar_type == self.config.benchmark_bar_type:
            self._benchmark_closes[bar.ts_event] = float(bar.close)
            return

        benchmark_close = self._benchmark_closes.pop(bar.ts_event, None)
        if benchmark_close is None:
            return

        price = float(bar.close)
        self._pairs.append((benchmark_close, price))

        if not self.portfolio.is_net_flat(self.config.instrument_id):
            self._manage_open_position(price)
            return

        self._bars_held = 0

        if len(self._pairs) < self.config.train_len:
            return

        self.evaluations += 1
        fit = self._fit()

        if fit is None:
            return

        intercept, slope, r_squared, sigma = fit

        if r_squared <= self.config.rsquared_threshold:
            return

        self.gate_open += 1
        deviation = price - (intercept + slope * benchmark_close)
        threshold = self.config.sigma_multiple * sigma

        if deviation >= threshold:
            self._enter(OrderSide.BUY)
        elif deviation <= -threshold:
            self._enter(OrderSide.SELL)

    def on_stop(self) -> None:
        """
        Close any open position and report the counters.
        """
        self.close_all_positions(self.config.instrument_id)
        log_msg = (
            f"bars={BAR_COUNT} evaluations={self.evaluations} "
            f"gate_open={self.gate_open} entries={self.entries}"
        )
        self.log.info(log_msg)

    def _fit(self) -> tuple[float, float, float, float] | None:
        """
        Refit the straight line over the rolling window.

        Returns the intercept, the slope, the coefficient of determination and the residual
        standard deviation, or ``None`` when the window cannot be fitted.
        """
        pairs = self._pairs
        count = len(pairs)
        mean_benchmark = sum(benchmark for benchmark, _ in pairs) / count
        mean_price = sum(price for _, price in pairs) / count

        covariance = 0.0
        variance = 0.0

        for benchmark, price in pairs:
            covariance += (benchmark - mean_benchmark) * (price - mean_price)
            variance += (benchmark - mean_benchmark) ** 2

        if variance == 0.0:
            return None

        slope = covariance / variance
        intercept = mean_price - slope * mean_benchmark

        residual_sum = 0.0
        total_sum = 0.0

        for benchmark, price in pairs:
            residual = price - (intercept + slope * benchmark)
            residual_sum += residual * residual
            total_sum += (price - mean_price) ** 2

        if total_sum == 0.0 or count < 3:
            return None

        r_squared = 1.0 - residual_sum / total_sum
        sigma = math.sqrt(residual_sum / (count - 2))

        return intercept, slope, r_squared, sigma

    def _enter(self, side: OrderSide) -> None:
        """
        Submit a market order to open a position.
        """
        instrument = self.cache.instrument(self.config.instrument_id)
        order = self.order_factory.market(
            self.config.instrument_id,
            side,
            instrument.make_qty(self.config.trade_size),
        )
        self.submit_order(order)
        self._bars_held = 0
        self.entries += 1

    def _manage_open_position(self, price: float) -> None:
        """
        Close the position on the time cap or on the absolute stop.
        """
        positions = self.cache.positions_open(instrument_id=self.config.instrument_id)

        if not positions:
            return

        self._bars_held += 1
        entry_price = float(positions[0].avg_px_open)
        reached_stop = abs(price - entry_price) >= self.config.stop_points
        reached_time_cap = self._bars_held >= self.config.holding_threshold

        if reached_stop or reached_time_cap:
            self.close_all_positions(self.config.instrument_id)
            self._bars_held = 0


if __name__ == "__main__":
    engine = BacktestEngine(
        config=BacktestEngineConfig(trader_id=TraderId.from_str("TESTER-001")),
    )

    venue = Venue("SIM")
    engine.add_venue(
        venue=venue,
        oms_type=OmsType.NETTING,
        account_type=AccountType.MARGIN,
        base_currency=USD,
        fee_model=PerContractFeeModel(Money.from_str("2.50 USD")),
        starting_balances=[Money.from_str("100000 USD")],
    )

    benchmark_instrument = create_contract(venue, "CRUDE", "crude oil", 2, "0.01")
    traded_instrument = create_contract(venue, "PETRO", "petrocurrency futures", 5, "0.00001")
    engine.add_instrument(benchmark_instrument)
    engine.add_instrument(traded_instrument)

    benchmark_bar_type = BarType.from_str(
        f"{benchmark_instrument.id}-1-MINUTE-LAST-EXTERNAL",
    )
    traded_bar_type = BarType.from_str(f"{traded_instrument.id}-1-MINUTE-LAST-EXTERNAL")

    benchmark_prices, traded_prices = synthetic_prices()
    timestamp_base = dt_to_unix_nanos(datetime(2026, 1, 5, tzinfo=UTC))
    bars = []

    for index in range(BAR_COUNT):
        ts = timestamp_base + index * 60_000_000_000
        bars.append(
            Bar(
                bar_type=benchmark_bar_type,
                open=benchmark_instrument.make_price(benchmark_prices[index]),
                high=benchmark_instrument.make_price(benchmark_prices[index] + 0.05),
                low=benchmark_instrument.make_price(benchmark_prices[index] - 0.05),
                close=benchmark_instrument.make_price(benchmark_prices[index]),
                volume=Quantity.from_int(1000),
                ts_event=ts,
                ts_init=ts,
            ),
        )
        bars.append(
            Bar(
                bar_type=traded_bar_type,
                open=traded_instrument.make_price(traded_prices[index]),
                high=traded_instrument.make_price(traded_prices[index] + 0.0002),
                low=traded_instrument.make_price(traded_prices[index] - 0.0002),
                close=traded_instrument.make_price(traded_prices[index]),
                volume=Quantity.from_int(1000),
                ts_event=ts,
                ts_init=ts,
            ),
        )

    engine.add_data(bars)

    strategy = GatedWalkForward(
        GatedWalkForwardConfig(
            instrument_id=traded_instrument.id,
            benchmark_id=benchmark_instrument.id,
            bar_type=traded_bar_type,
            benchmark_bar_type=benchmark_bar_type,
            trade_size=Decimal(1),
        ),
    )
    engine.add_strategy(strategy)
    engine.run()

    print(engine.generate_order_fills_report())
    print(engine.generate_positions_report())
    print(engine.generate_account_report(venue=venue))

    engine.dispose()
