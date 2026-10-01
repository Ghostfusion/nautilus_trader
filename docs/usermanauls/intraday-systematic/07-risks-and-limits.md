# 07 - Risks and limits

This lecture is about the gap between a rule that works in a backtest and a rule that survives real
trading. It covers stops, sizing, look-ahead, curve fitting, and the parts of that gap the engine
closes for you and the parts it does not.

## What breaks in production for this style

| Failure       | What it looks like                        | Why it happens                                                  |
| ------------- | ----------------------------------------- | --------------------------------------------------------------- |
| Whipsaw       | Many small losses in a row                | Price oscillates around the slow average                        |
| Cost bleed    | Gross looks fine, net is negative         | Spread and commissions on every trip                            |
| Thin evidence | A beautiful equity curve from five trades | Too few samples to mean anything                                |
| Look-ahead    | A flawless backtest                       | The rule used a price before it existed                         |
| Curve fitting | Perfect in the test, useless live         | Periods tuned to the test data                                  |
| Stale fills   | Live PnL worse than simulated             | The venue did not fill your market order at the simulated price |
| Session edges | Strange fills near the open or close      | Time zones, auctions, reduced liquidity                         |
| Data gaps     | A bar every ten minutes instead of one    | The feed dropped data                                           |

## Stop losses

A stop loss is an order that stays inactive until a trigger price trades, then becomes a market
order. In this repository it is created with `order_factory.stop_market(instrument_id, order_side,
quantity, trigger_price)`. `stop_market` is defined in `python/nautilus_trader/common/__init__.pyi`
alongside the other order factories.

The program below buys once when the fast average is below the slow average, then places a
protective sell stop 0.050 below the entry. Save it as `/tmp/stop_demo.py` and run it.

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
from nautilus_trader.model import Money
from nautilus_trader.model import OmsType
from nautilus_trader.model import OrderSide
from nautilus_trader.model import Price
from nautilus_trader.model import Quantity
from nautilus_trader.model import TraderId
from nautilus_trader.model import Venue
from nautilus_trader.testkit.providers import TestDataProvider
from nautilus_trader.testkit.providers import TestInstrumentProvider
from nautilus_trader.trading import Strategy


class StopConfig(StrategyConfig):
    def __init__(self, *, instrument_id, bar_type, trade_size, stop_distance, **_kwargs):
        super().__init__()
        self.instrument_id = instrument_id
        self.bar_type = bar_type
        self.trade_size = trade_size
        self.stop_distance = stop_distance


class StopDemo(Strategy):
    def __init__(self, config: StopConfig) -> None:
        super().__init__(config)
        self.fast_ema = ExponentialMovingAverage(10)
        self.slow_ema = ExponentialMovingAverage(30)
        self.entry_px = None
        self.stop_order = None

    def on_start(self) -> None:
        self.register_indicator_for_bars(self.config.bar_type, self.fast_ema)
        self.register_indicator_for_bars(self.config.bar_type, self.slow_ema)
        self.subscribe_bars(self.config.bar_type)

    def on_bar(self, bar: Bar) -> None:
        if not self.indicators_initialized() or self.entry_px is not None:
            return
        if self.fast_ema.value >= self.slow_ema.value:
            return  # wait for a long entry
        self.entry_px = bar.close
        self.submit_order(
            self.order_factory.market(self.config.instrument_id, OrderSide.BUY, self._qty()),
        )
        trigger = Price.from_str(f"{float(bar.close) - float(self.config.stop_distance):.3f}")
        self.stop_order = self.order_factory.stop_market(
            self.config.instrument_id,
            OrderSide.SELL,
            self._qty(),
            trigger,
        )
        self.submit_order(self.stop_order)

    def on_order_filled(self, event) -> None:
        print("FILL", event.order_side, event.last_qty, event.last_px, event.order_type)

    def _qty(self) -> Quantity:
        return self.cache.instrument(self.config.instrument_id).make_qty(self.config.trade_size)

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
    StopDemo(
        StopConfig(
            instrument_id=USDJPY_SIM.id,
            bar_type=BarType.from_str("USD/JPY.SIM-1-MINUTE-MID-INTERNAL"),
            trade_size=Decimal(100_000),
            stop_distance=Decimal("0.050"),
        ),
    ),
)
engine.run()

with pd.option_context("display.max_rows", 100, "display.max_columns", None, "display.width", 300):
    print(
        engine.generate_orders_report()[["type", "side", "quantity", "trigger_price", "status", "avg_px"]],
    )

engine.reset()
engine.dispose()
```

Output:

```text
FILL BUY 100000 109.344 MARKET
FILL SELL 100000 109.289 STOP_MARKET
                                    type  side quantity trigger_price  status   avg_px
client_order_id
O-20190101-232900-001-000-1       MARKET   BUY   100000           NaN  FILLED  109.344
O-20190101-232900-001-000-2  STOP_MARKET  SELL   100000       109.289  FILLED  109.289
```

The stop did its job: it limited the loss to 0.055 per unit rather than letting the position run. The
orders report shows the trigger price of 109.289 separately from the fill price of 109.289, and the
status FILLED. Worked example 3 in lecture 01 computes the resulting loss by hand: 5,938 yen.

Three limits of stops that beginners miss:

1. A stop becomes a market order when it triggers, so it can slip. The trigger price and the fill
   price are not guaranteed to be equal, especially in a fast or thin market.
2. When a stop and a target both sit inside one bar, which one triggers first is a guess, because a
   bar does not record the intrabar path (`docs/concepts/backtesting/bar-execution.md`).
3. A stop does not cap your loss at exactly the distance. It caps where the order starts, not where
   it ends.

## Position sizing

Position sizing decides how much one trade can lose, before the entry. Three simple rules, in
increasing order of care:

1. Fixed size. The same quantity every time, as in this manual. Easy to explain, and it makes the
   other numbers comparable.
2. Fixed fraction. The size is a fixed fraction of account equity, so a shrinking account trades
   smaller.
3. Volatility scaled. The size is set so that a typical adverse move costs a fixed fraction of
   equity. This is the shape professional sizing takes.

For any of them, the number that matters is the loss per trade if the rule is wrong, not the
notional traded. With a 100,000 unit position and a 0.055 stop, the planned loss is about 5,500 yen
before costs; if that is more than you are willing to lose, the size is too large, whatever the
backtest says.

## What the engine does and does not enforce

The risk engine sits between the strategy and the venue and can refuse an order before it is sent.
The Python-settable fields are `bypass`, `max_order_submit_rate`, `max_order_modify_rate`,
`max_notional_per_order`, `full_position_exit_venues` and `debug`
(`python/nautilus_trader/risk/__init__.pyi`).

This program is the lecture 05 program with two changes, shown here as the changed lines; the rest
of the file is identical. The rule keeps ordering 100,000 units while the cap is 10,000.

```python
from nautilus_trader.config import RiskEngineConfig

engine = BacktestEngine(
    BacktestEngineConfig(
        trader_id=TraderId.from_str("BACKTESTER-001"),
        logging=LoggerConfig(bypass_logging=True),
        risk_engine=RiskEngineConfig(max_notional_per_order={"USD/JPY.SIM": "10000"}),
    ),
)
```

Output, from the same rule and the same data:

```text
                               type  side quantity  status
client_order_id
O-20190101-232900-001-000-1  MARKET  SELL   100000  DENIED
O-20190101-233000-001-000-2  MARKET  SELL   100000  DENIED
O-20190101-233100-001-000-3  MARKET  SELL   100000  DENIED
total orders: 138
status counts: {'DENIED': 138}
```

Every order is refused, and the strategy keeps trying for the rest of the run because its position
never opens. A denied order is a hard limit that protects you from yourself. Notice also the second
lesson: the rule has no idea it was refused unless you handle it, so a misconfigured limit silently
turns a strategy into a generator of rejected orders.

What the engine does not do:

- It does not force you to set a stop. A rule with no exit runs until the data ends or the account
  is empty.
- It does not size your positions. `trade_size` is your number.
- It does not know your account's real value in live trading unless you reconcile it.
- It cannot simulate the intrabar path or a fill you did not get.
- The pre-trade count caps on send, cancel and fill counts are Rust only (`count_caps` in
  `crates/risk/src/engine/config.rs` and `crates/risk/src/engine/cap.rs`); a Python user cannot set
  one.

## Look-ahead bias

Look-ahead bias is using information that was not available when the decision was made. The classic
intraday mistake is acting on a bar before it has closed. In this repository the guard is
mechanical:

- A strategy receives a bar in `on_bar` after the bar is complete.
- For execution simulation, a bar's `ts_init` must represent the close of its interval, so the
  complete bar cannot become visible before it formed
  (`docs/concepts/backtesting/bar-execution.md`).
- Orders submitted from `on_bar` are processed after that bar's four synthetic price points, so a
  strategy cannot trade the same bar it is reading
  (`docs/concepts/backtesting/bar-execution.md`).

The subtler form of look-ahead is in your own code: computing an indicator over the whole series
before the run, or reading the next bar's open from the current callback. Indicators registered with
`register_indicator_for_bars` update one bar at a time, which is why this manual uses them rather
than precomputing a column.

## A rule that fits the past against a rule that generalises

A rule that fits the past is one whose parameters were chosen by looking at the data it is then
measured on. Changing the periods until the curve looks good is the most common way to do this, and
it is invisible in the printed statistics.

A rule that generalises was fixed before it met the test data, and was checked on data it had never
seen. To separate the two:

1. Write the rule and its parameters down first.
2. Measure on one period, then measure on a later period without changing anything.
3. Change one thing at a time, and keep the change only if it helps on both.
4. Prefer fewer parameters. A rule with two averages is easier to trust than one with twelve knobs.
5. Report the number of trades alongside every result.

Validation is a discipline of its own. Two sibling manuals go deeper: the
[factor-portfolio](../factor-portfolio/README.md) manual covers point-in-time datasets and validated
splits, and the [ai-training](../ai-training/README.md) manual covers seeded search and honest
out-of-sample reporting. Read them before you let a search tune this rule.

## Session time and the calendar

A minute is exchange local time. Around a session open or close, liquidity changes and spreads widen,
and a rule that trades every bar will trade the worst bars too. The engine resolves sessions through
a calendar in the exchange's time zone and can derive session phases such as the open and the close
(`docs/concepts/trading_calendars.md`). A robust intraday rule usually refuses to trade in the first
minutes after an open and the last minutes before a close.

Next: practice, in [08-exercises](08-exercises.md).
