# 05 - Build the strategy

This lecture builds one rule from an empty file. Each step is numbered, and each step shows the
program and the output it produced. Follow along by saving each program outside the repository and
running it.

## The rule, written down

Write the rule in words before writing code. This is the whole point of the style.

1. Use one-minute bars of the USD/JPY mid price for the simulated venue.
2. Compute a fast exponential moving average over the last 10 bar closes.
3. Compute a slow exponential moving average over the last 30 bar closes.
4. When the fast average is above the slow average, the rule wants to be long.
5. When the fast average is below the slow average, the rule wants to be short.
6. Trade a fixed quantity of 100,000 units each time.
7. Hold nothing overnight; close any open position when the strategy stops.

An exponential moving average, or EMA, is a running average that gives more weight to recent
values. The engine's implementation is `ExponentialMovingAverage(period)` in
`nautilus_trader.indicators` (source under `crates/indicators`). It is fed by bars because
`register_indicator_for_bars` connects it to a bar type, so it updates once per completed bar.

The rule differs from the builtin `EmaCross` in lecture 03 in two ways: it acts on the level of the
averages rather than on a single crossover event, and it reverses the position instead of only
adding. Both differences matter, as the results show.

## Step 1: subscriptions only

Start with a strategy that does nothing but subscribe to bars and count them. This proves the data
is arriving before any trading logic exists.

```python
from decimal import Decimal

from nautilus_trader.backtest import BacktestEngine
from nautilus_trader.common import LoggerConfig
from nautilus_trader.config import BacktestEngineConfig
from nautilus_trader.config import StrategyConfig
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.model import AccountType
from nautilus_trader.model import Bar
from nautilus_trader.model import BarType
from nautilus_trader.model import Currency
from nautilus_trader.model import Money
from nautilus_trader.model import OmsType
from nautilus_trader.model import TraderId
from nautilus_trader.model import Venue
from nautilus_trader.testkit.providers import TestDataProvider
from nautilus_trader.testkit.providers import TestInstrumentProvider
from nautilus_trader.trading import Strategy


class RuleConfig(StrategyConfig):
    def __init__(self, *, instrument_id, bar_type, trade_size, fast_period=10, slow_period=30, **_kwargs):
        super().__init__()
        self.instrument_id = instrument_id
        self.bar_type = bar_type
        self.trade_size = trade_size
        self.fast_period = fast_period
        self.slow_period = slow_period


class RuleStrategy(Strategy):
    def __init__(self, config: RuleConfig) -> None:
        super().__init__(config)
        self.count = 0
        self.first = None

    def on_start(self) -> None:
        self.subscribe_bars(self.config.bar_type)

    def on_bar(self, bar: Bar) -> None:
        self.count += 1
        if self.first is None:
            self.first = bar
        if self.count <= 3:
            print("BAR", bar.bar_type, bar.close, bar.volume, bar.ts_event)

    def on_stop(self) -> None:
        print("bars_received:", self.count)


engine = BacktestEngine(
    BacktestEngineConfig(
        trader_id=TraderId.from_str("BACKTESTER-001"),
        logging=LoggerConfig(bypass_logging=True),
    ),
)
SIM = Venue("SIM")
USD = Currency.from_str("USD")
engine.add_venue(
    venue=SIM,
    oms_type=OmsType.NETTING,
    account_type=AccountType.MARGIN,
    base_currency=USD,
    starting_balances=[Money(1_000_000, USD)],
    fee_model=MakerTakerFeeModel(maker_rate=Decimal("0.00002"), taker_rate=Decimal("0.00002")),
)
USDJPY_SIM = TestInstrumentProvider.usdjpy_sim()
engine.add_instrument(USDJPY_SIM)
engine.add_data(TestDataProvider.usdjpy_quotes(count=10_000))
engine.add_strategy(
    RuleStrategy(
        RuleConfig(
            instrument_id=USDJPY_SIM.id,
            bar_type=BarType.from_str("USD/JPY.SIM-1-MINUTE-MID-INTERNAL"),
            trade_size=Decimal(100_000),
        ),
    ),
)
engine.run()
engine.reset()
engine.dispose()
```

Output:

```text
BAR USD/JPY.SIM-1-MINUTE-MID-INTERNAL 109.5050 1000000.0 1546383600000000000
BAR USD/JPY.SIM-1-MINUTE-MID-INTERNAL 109.5650 60000000.0 1546383660000000000
BAR USD/JPY.SIM-1-MINUTE-MID-INTERNAL 109.6240 60000000.0 1546383720000000000
bars_received: 167
```

The engine aggregated the 10,000 quote ticks into 167 one-minute bars, each delivered to `on_bar`.
The first bar holds a single quote, so its volume is 1,000,000; each later bar holds a full minute,
so its volume is 60,000,000. Note that `on_bar` receives the closed bar, so every decision in this
manual uses information that already existed.

## Step 2: add the indicators

Now add the two EMAs and register them against the bar type. `register_indicator_for_bars` connects
an indicator to a bar type so it updates automatically for each completed bar; then `on_bar` reads
`self.fast_ema.value` and `self.slow_ema.value`. `indicators_initialized()` is true only once both
averages have seen enough bars.

```python
from decimal import Decimal

from nautilus_trader.backtest import BacktestEngine
from nautilus_trader.common import LoggerConfig
from nautilus_trader.config import BacktestEngineConfig
from nautilus_trader.config import StrategyConfig
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.indicators import ExponentialMovingAverage
from nautilus_trader.model import AccountType
from nautilus_trader.model import Bar
from nautilus_trader.model import BarType
from nautilus_trader.model import Currency
from nautilus_trader.model import Money
from nautilus_trader.model import OmsType
from nautilus_trader.model import TraderId
from nautilus_trader.model import Venue
from nautilus_trader.testkit.providers import TestDataProvider
from nautilus_trader.testkit.providers import TestInstrumentProvider
from nautilus_trader.trading import Strategy


class RuleConfig(StrategyConfig):
    def __init__(self, *, instrument_id, bar_type, fast_period=10, slow_period=30, **_kwargs):
        super().__init__()
        self.instrument_id = instrument_id
        self.bar_type = bar_type
        self.fast_period = fast_period
        self.slow_period = slow_period


class RuleStrategy(Strategy):
    def __init__(self, config: RuleConfig) -> None:
        super().__init__(config)
        self.fast_ema = ExponentialMovingAverage(config.fast_period)
        self.slow_ema = ExponentialMovingAverage(config.slow_period)
        self.count = 0
        self.printed = 0

    def on_start(self) -> None:
        self.register_indicator_for_bars(self.config.bar_type, self.fast_ema)
        self.register_indicator_for_bars(self.config.bar_type, self.slow_ema)
        self.subscribe_bars(self.config.bar_type)

    def on_bar(self, bar: Bar) -> None:
        self.count += 1
        if self.indicators_initialized() and self.printed < 4:
            self.printed += 1
            print(f"bar {self.count}: fast={self.fast_ema.value:.4f} slow={self.slow_ema.value:.4f}")

    def on_stop(self) -> None:
        print("bars_received:", self.count)


engine = BacktestEngine(
    BacktestEngineConfig(
        trader_id=TraderId.from_str("BACKTESTER-001"),
        logging=LoggerConfig(bypass_logging=True),
    ),
)
SIM = Venue("SIM")
USD = Currency.from_str("USD")
engine.add_venue(
    venue=SIM,
    oms_type=OmsType.NETTING,
    account_type=AccountType.MARGIN,
    base_currency=USD,
    starting_balances=[Money(1_000_000, USD)],
    fee_model=MakerTakerFeeModel(maker_rate=Decimal("0.00002"), taker_rate=Decimal("0.00002")),
)
USDJPY_SIM = TestInstrumentProvider.usdjpy_sim()
engine.add_instrument(USDJPY_SIM)
engine.add_data(TestDataProvider.usdjpy_quotes(count=10_000))
engine.add_strategy(
    RuleStrategy(
        RuleConfig(
            instrument_id=USDJPY_SIM.id,
            bar_type=BarType.from_str("USD/JPY.SIM-1-MINUTE-MID-INTERNAL"),
        ),
    ),
)
engine.run()
engine.reset()
engine.dispose()
```

Output:

```text
bar 30: fast=109.5667 slow=109.6848
bar 31: fast=109.5153 slow=109.6589
bar 32: fast=109.4638 slow=109.6314
bar 33: fast=109.4129 slow=109.6025
bars_received: 167
```

The slow average needs 30 bars before it has a value, so the first printed line is bar 30. At bar 30
the fast average, 109.5667, is below the slow average, 109.6848, so the rule's first instruction
will be to be short.

## Step 3: add the entry rule

Decide the wanted direction from the two averages, print a line when the direction changes, and send
a market order when the strategy is flat. The helper `_quantity` asks the instrument to round the
size the way the venue expects.

```python
from decimal import Decimal

from nautilus_trader.backtest import BacktestEngine
from nautilus_trader.common import LoggerConfig
from nautilus_trader.config import BacktestEngineConfig
from nautilus_trader.config import StrategyConfig
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.indicators import ExponentialMovingAverage
from nautilus_trader.model import AccountType
from nautilus_trader.model import Bar
from nautilus_trader.model import BarType
from nautilus_trader.model import Currency
from nautilus_trader.model import Money
from nautilus_trader.model import OmsType
from nautilus_trader.model import OrderSide
from nautilus_trader.model import TraderId
from nautilus_trader.model import Venue
from nautilus_trader.testkit.providers import TestDataProvider
from nautilus_trader.testkit.providers import TestInstrumentProvider
from nautilus_trader.trading import Strategy


class RuleConfig(StrategyConfig):
    def __init__(self, *, instrument_id, bar_type, fast_period=10, slow_period=30, **_kwargs):
        super().__init__()
        self.instrument_id = instrument_id
        self.bar_type = bar_type
        self.fast_period = fast_period
        self.slow_period = slow_period


class RuleStrategy(Strategy):
    def __init__(self, config: RuleConfig) -> None:
        super().__init__(config)
        self.fast_ema = ExponentialMovingAverage(config.fast_period)
        self.slow_ema = ExponentialMovingAverage(config.slow_period)
        self.count = 0
        self.last_signal = None

    def on_start(self) -> None:
        self.register_indicator_for_bars(self.config.bar_type, self.fast_ema)
        self.register_indicator_for_bars(self.config.bar_type, self.slow_ema)
        self.subscribe_bars(self.config.bar_type)

    def on_bar(self, bar: Bar) -> None:
        self.count += 1
        if not self.indicators_initialized():
            return

        if self.fast_ema.value > self.slow_ema.value:
            signal = "LONG"
        elif self.fast_ema.value < self.slow_ema.value:
            signal = "SHORT"
        else:
            signal = None

        if signal != self.last_signal:
            print(f"bar {self.count}: signal={signal} fast={self.fast_ema.value:.4f} slow={self.slow_ema.value:.4f}")
            self.last_signal = signal

        if signal == "LONG" and self.portfolio.is_net_flat(self.config.instrument_id):
            order = self.order_factory.market(
                self.config.instrument_id, OrderSide.BUY, self._quantity(),
            )
            print(f"bar {self.count}: SUBMIT BUY  {order.quantity} {self.config.instrument_id}")
            self.submit_order(order)
        elif signal == "SHORT" and self.portfolio.is_net_flat(self.config.instrument_id):
            order = self.order_factory.market(
                self.config.instrument_id, OrderSide.SELL, self._quantity(),
            )
            print(f"bar {self.count}: SUBMIT SELL {order.quantity} {self.config.instrument_id}")
            self.submit_order(order)

    def _quantity(self):
        instrument = self.cache.instrument(self.config.instrument_id)
        return instrument.make_qty(Decimal(100_000))


engine = BacktestEngine(
    BacktestEngineConfig(
        trader_id=TraderId.from_str("BACKTESTER-001"),
        logging=LoggerConfig(bypass_logging=True),
    ),
)
SIM = Venue("SIM")
USD = Currency.from_str("USD")
engine.add_venue(
    venue=SIM,
    oms_type=OmsType.NETTING,
    account_type=AccountType.MARGIN,
    base_currency=USD,
    starting_balances=[Money(1_000_000, USD)],
    fee_model=MakerTakerFeeModel(maker_rate=Decimal("0.00002"), taker_rate=Decimal("0.00002")),
)
USDJPY_SIM = TestInstrumentProvider.usdjpy_sim()
engine.add_instrument(USDJPY_SIM)
engine.add_data(TestDataProvider.usdjpy_quotes(count=10_000))
engine.add_strategy(
    RuleStrategy(
        RuleConfig(
            instrument_id=USDJPY_SIM.id,
            bar_type=BarType.from_str("USD/JPY.SIM-1-MINUTE-MID-INTERNAL"),
        ),
    ),
)
engine.run()
engine.reset()
engine.dispose()
```

Output:

```text
bar 30: signal=SHORT fast=109.5667 slow=109.6848
bar 30: SUBMIT SELL 100000 USD/JPY.SIM
bar 54: signal=LONG fast=109.3360 slow=109.3165
bar 80: signal=SHORT fast=109.6821 slow=109.7035
bar 106: signal=LONG fast=109.3194 slow=109.3046
bar 132: signal=SHORT fast=109.6990 slow=109.7085
bar 158: signal=LONG fast=109.3027 slow=109.2987
```

The rule wants to be short at bar 30, so it sells. It sees no further instruction changes until bar
54, when it wants to be long, but the strategy is already short, and this step has no exit code, so
it does nothing. Every later change is likewise ignored. The program proves the entry works; it also
proves that entry alone is not a strategy.

## Step 4: add the exit rule

Entry without exit is not a rule; it is a wish. Add the exit: when the wanted direction changes,
close the opposite position and then open the new one. The close is submitted first, so the position
starts unwinding before any new exposure is created. `close_all_positions` builds a reduce-only
order, which the venue will not let increase exposure.

```python
from decimal import Decimal

from nautilus_trader.backtest import BacktestEngine
from nautilus_trader.common import LoggerConfig
from nautilus_trader.config import BacktestEngineConfig
from nautilus_trader.config import StrategyConfig
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.indicators import ExponentialMovingAverage
from nautilus_trader.model import AccountType
from nautilus_trader.model import Bar
from nautilus_trader.model import BarType
from nautilus_trader.model import Currency
from nautilus_trader.model import Money
from nautilus_trader.model import OmsType
from nautilus_trader.model import OrderSide
from nautilus_trader.model import TraderId
from nautilus_trader.model import Venue
from nautilus_trader.testkit.providers import TestDataProvider
from nautilus_trader.testkit.providers import TestInstrumentProvider
from nautilus_trader.trading import Strategy


class RuleConfig(StrategyConfig):
    def __init__(self, *, instrument_id, bar_type, fast_period=10, slow_period=30, **_kwargs):
        super().__init__()
        self.instrument_id = instrument_id
        self.bar_type = bar_type
        self.fast_period = fast_period
        self.slow_period = slow_period


class RuleStrategy(Strategy):
    def __init__(self, config: RuleConfig) -> None:
        super().__init__(config)
        self.fast_ema = ExponentialMovingAverage(config.fast_period)
        self.slow_ema = ExponentialMovingAverage(config.slow_period)
        self.count = 0
        self.last_signal = None

    def on_start(self) -> None:
        self.register_indicator_for_bars(self.config.bar_type, self.fast_ema)
        self.register_indicator_for_bars(self.config.bar_type, self.slow_ema)
        self.subscribe_bars(self.config.bar_type)

    def on_bar(self, bar: Bar) -> None:
        self.count += 1
        if not self.indicators_initialized():
            return

        if self.fast_ema.value > self.slow_ema.value:
            signal = "LONG"
        elif self.fast_ema.value < self.slow_ema.value:
            signal = "SHORT"
        else:
            signal = None

        if signal != self.last_signal:
            print(f"bar {self.count}: signal={signal}")
            self.last_signal = signal

        if signal == "LONG":
            if self.portfolio.is_net_short(self.config.instrument_id):
                self.close_all_positions(self.config.instrument_id)
                print(f"bar {self.count}: CLOSE SHORT")
            if self.portfolio.is_net_flat(self.config.instrument_id):
                print(f"bar {self.count}: SUBMIT BUY")
                self.submit_order(
                    self.order_factory.market(
                        self.config.instrument_id, OrderSide.BUY, self._quantity(),
                    ),
                )
        elif signal == "SHORT":
            if self.portfolio.is_net_long(self.config.instrument_id):
                self.close_all_positions(self.config.instrument_id)
                print(f"bar {self.count}: CLOSE LONG")
            if self.portfolio.is_net_flat(self.config.instrument_id):
                print(f"bar {self.count}: SUBMIT SELL")
                self.submit_order(
                    self.order_factory.market(
                        self.config.instrument_id, OrderSide.SELL, self._quantity(),
                    ),
                )

    def _quantity(self):
        instrument = self.cache.instrument(self.config.instrument_id)
        return instrument.make_qty(Decimal(100_000))


engine = BacktestEngine(
    BacktestEngineConfig(
        trader_id=TraderId.from_str("BACKTESTER-001"),
        logging=LoggerConfig(bypass_logging=True),
    ),
)
SIM = Venue("SIM")
USD = Currency.from_str("USD")
engine.add_venue(
    venue=SIM,
    oms_type=OmsType.NETTING,
    account_type=AccountType.MARGIN,
    base_currency=USD,
    starting_balances=[Money(1_000_000, USD)],
    fee_model=MakerTakerFeeModel(maker_rate=Decimal("0.00002"), taker_rate=Decimal("0.00002")),
)
USDJPY_SIM = TestInstrumentProvider.usdjpy_sim()
engine.add_instrument(USDJPY_SIM)
engine.add_data(TestDataProvider.usdjpy_quotes(count=10_000))
engine.add_strategy(
    RuleStrategy(
        RuleConfig(
            instrument_id=USDJPY_SIM.id,
            bar_type=BarType.from_str("USD/JPY.SIM-1-MINUTE-MID-INTERNAL"),
        ),
    ),
)
engine.run()
report = engine.generate_positions_report()
print("positions_in_report:", len(report))
engine.reset()
engine.dispose()
```

Output:

```text
bar 30: signal=SHORT
bar 30: SUBMIT SELL
bar 54: signal=LONG
bar 54: CLOSE SHORT
bar 55: SUBMIT BUY
bar 80: signal=SHORT
bar 80: CLOSE LONG
bar 81: SUBMIT SELL
bar 106: signal=LONG
bar 106: CLOSE SHORT
bar 107: SUBMIT BUY
bar 132: signal=SHORT
bar 132: CLOSE LONG
bar 133: SUBMIT SELL
bar 158: signal=LONG
bar 158: CLOSE SHORT
bar 159: SUBMIT BUY
positions_in_report: 6
```

Five round trips are closed during the run and a sixth position is still open at the last bar. The
six rows are the five closed cycles plus the live one; the closed cycles show up as snapshot rows,
which lecture 06 explains. Notice that the replacement order arrives on the bar after the close, not
in the same callback: within `on_bar` the portfolio still reports the old position until the close
order has filled, so the flat check that guards the new order only passes on the next bar.

## Step 5: sizing, stop, and the full run

The last step fixes the size, closes any open position when the strategy stops, and prints the
reports. Add `on_stop`, keep `trade_size` in the config so it can be changed without editing the
rule, and read the fills and positions.

```python
from decimal import Decimal

import pandas as pd

from nautilus_trader.backtest import BacktestEngine
from nautilus_trader.common import LoggerConfig
from nautilus_trader.config import BacktestEngineConfig
from nautilus_trader.config import StrategyConfig
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.indicators import ExponentialMovingAverage
from nautilus_trader.model import AccountType
from nautilus_trader.model import Bar
from nautilus_trader.model import BarType
from nautilus_trader.model import Currency
from nautilus_trader.model import InstrumentId
from nautilus_trader.model import Money
from nautilus_trader.model import OmsType
from nautilus_trader.model import OrderSide
from nautilus_trader.model import Quantity
from nautilus_trader.model import TraderId
from nautilus_trader.model import Venue
from nautilus_trader.testkit.providers import TestDataProvider
from nautilus_trader.testkit.providers import TestInstrumentProvider
from nautilus_trader.trading import Strategy


class IntradayRuleConfig(StrategyConfig):
    def __init__(
        self,
        *,
        instrument_id: InstrumentId,
        bar_type: BarType,
        trade_size: Decimal,
        fast_period: int = 10,
        slow_period: int = 30,
        **_kwargs: object,
    ) -> None:
        super().__init__()
        self.instrument_id = instrument_id
        self.bar_type = bar_type
        self.trade_size = trade_size
        self.fast_period = fast_period
        self.slow_period = slow_period


class IntradayRuleStrategy(Strategy):
    def __init__(self, config: IntradayRuleConfig) -> None:
        super().__init__(config)
        self.fast_ema = ExponentialMovingAverage(config.fast_period)
        self.slow_ema = ExponentialMovingAverage(config.slow_period)

    def on_start(self) -> None:
        self.register_indicator_for_bars(self.config.bar_type, self.fast_ema)
        self.register_indicator_for_bars(self.config.bar_type, self.slow_ema)
        self.subscribe_bars(self.config.bar_type)

    def on_bar(self, bar: Bar) -> None:
        if not self.indicators_initialized():
            return

        if self.fast_ema.value > self.slow_ema.value:
            if self.portfolio.is_net_short(self.config.instrument_id):
                self.close_all_positions(self.config.instrument_id)
            if self.portfolio.is_net_flat(self.config.instrument_id):
                self.submit_order(
                    self.order_factory.market(
                        self.config.instrument_id,
                        OrderSide.BUY,
                        self._quantity(),
                    ),
                )
        elif self.fast_ema.value < self.slow_ema.value:
            if self.portfolio.is_net_long(self.config.instrument_id):
                self.close_all_positions(self.config.instrument_id)
            if self.portfolio.is_net_flat(self.config.instrument_id):
                self.submit_order(
                    self.order_factory.market(
                        self.config.instrument_id,
                        OrderSide.SELL,
                        self._quantity(),
                    ),
                )

    def _quantity(self) -> Quantity:
        instrument = self.cache.instrument(self.config.instrument_id)
        return instrument.make_qty(self.config.trade_size)

    def on_stop(self) -> None:
        self.close_all_positions(self.config.instrument_id)


engine = BacktestEngine(
    BacktestEngineConfig(
        trader_id=TraderId.from_str("BACKTESTER-001"),
        logging=LoggerConfig(bypass_logging=True),
    ),
)
SIM = Venue("SIM")
USD = Currency.from_str("USD")
engine.add_venue(
    venue=SIM,
    oms_type=OmsType.NETTING,
    account_type=AccountType.MARGIN,
    base_currency=USD,
    starting_balances=[Money(1_000_000, USD)],
    fee_model=MakerTakerFeeModel(maker_rate=Decimal("0.00002"), taker_rate=Decimal("0.00002")),
)
USDJPY_SIM = TestInstrumentProvider.usdjpy_sim()
engine.add_instrument(USDJPY_SIM)
engine.add_data(TestDataProvider.usdjpy_quotes(count=10_000))
engine.add_strategy(
    IntradayRuleStrategy(
        IntradayRuleConfig(
            instrument_id=USDJPY_SIM.id,
            bar_type=BarType.from_str("USD/JPY.SIM-1-MINUTE-MID-INTERNAL"),
            trade_size=Decimal(100_000),
            fast_period=10,
            slow_period=30,
        ),
    ),
)
engine.run()

with pd.option_context("display.max_rows", 100, "display.max_columns", None, "display.width", 300):
    print(engine.generate_order_fills_report()[["instrument_id", "side", "filled_qty", "avg_px", "commissions"]])
    print(engine.generate_positions_report()[["side", "quantity", "avg_px_open", "avg_px_close", "realized_pnl", "is_snapshot"]])

result = engine.get_result()
print("PnL (total):", result.stats_pnls["USD"]["PnL (total)"])
print("Win Rate:", result.stats_pnls["USD"]["Win Rate"])
print("Profit Factor:", result.stats_returns["Profit Factor"])

engine.reset()
engine.dispose()
```

Output:

```text
                             instrument_id  side filled_qty   avg_px commissions
client_order_id
O-20190101-232900-001-000-1    USD/JPY.SIM  SELL     100000  109.334   [219 JPY]
O-20190101-235300-001-000-2    USD/JPY.SIM   BUY     100000  109.548   [219 JPY]
O-20190101-235400-001-000-3    USD/JPY.SIM   BUY     100000  109.608   [219 JPY]
O-20190102-001900-001-000-4    USD/JPY.SIM  SELL     100000  109.472   [219 JPY]
O-20190102-002000-001-000-5    USD/JPY.SIM  SELL     100000  109.413   [219 JPY]
O-20190102-004500-001-000-6    USD/JPY.SIM   BUY     100000  109.527   [219 JPY]
O-20190102-004600-001-000-7    USD/JPY.SIM   BUY     100000  109.587   [219 JPY]
O-20190102-011100-001-000-8    USD/JPY.SIM  SELL     100000  109.494   [219 JPY]
O-20190102-011200-001-000-9    USD/JPY.SIM  SELL     100000  109.434   [219 JPY]
O-20190102-013700-001-000-10   USD/JPY.SIM   BUY     100000  109.505   [219 JPY]
O-20190102-013800-001-000-11   USD/JPY.SIM   BUY     100000  109.565   [219 JPY]
O-20190102-014639-001-000-12   USD/JPY.SIM  SELL     100000  109.956   [220 JPY]
                                                    side quantity  avg_px_open  avg_px_close realized_pnl  is_snapshot
position_id
USD/JPY.SIM-IntradayRuleStrategy-000-33bdd80d-e...  FLAT        0      109.334       109.548   -21838 JPY         True
USD/JPY.SIM-IntradayRuleStrategy-000-d4feceff-7...  FLAT        0      109.608       109.472   -14038 JPY         True
USD/JPY.SIM-IntradayRuleStrategy-000-ecea7d58-1...  FLAT        0      109.413       109.527   -11838 JPY         True
USD/JPY.SIM-IntradayRuleStrategy-000-4982d1b2-5...  FLAT        0      109.587       109.494    -9738 JPY         True
USD/JPY.SIM-IntradayRuleStrategy-000-88c167ee-6...  FLAT        0      109.434       109.505    -7538 JPY         True
USD/JPY.SIM-IntradayRuleStrategy-000                FLAT        0      109.565       109.956    38661 JPY        False
PnL (total): -241.89000000013039
Win Rate: 0.16666666666666666
Profit Factor: 0.2029583278243312
```

The rule made six round trips, twelve fills, and lost money. One round trip out of six won. The
final position carried into `on_stop` was closed at the last available price, 109.956. The
`on_stop` close is what makes the report show six positions rather than five snapshots and an open
one.

Compare this result with lecture 03's builtin run, which made three round trips and won every one.
Same market data, same idea, opposite outcome. The difference is that this rule acts on the level of
the averages and reverses on every flip, so it trades twice as often and catches every whipsaw. That
is a real property of the rule, not a bug in the engine.

## What you built

You now have a complete, runnable, bar-driven rule strategy:

- It subscribes to bars and lets the engine aggregate them.
- It registers two indicators that update per bar.
- It decides a wanted direction from those indicators.
- It exits and reverses on a change of direction.
- It sizes every order from one configured number.
- It closes any open position at shutdown.

Next: how to read the numbers it produces, in [06-measure-and-evaluate](06-measure-and-evaluate.md).
