# 07 - Risks and limits

A market maker survives by bounding loss, not by predicting price. This lecture answers four
questions: how big a position may I hold, how do I discourage the position I do not want, what does
the engine enforce, and what happens when the venue disappears?

## Position limits: `max_position`, `trade_size`, and `num_levels`

Three fields decide your exposure:

- `trade_size` is the size of each quoted level. It is your risk unit.
- `num_levels` is how many buying levels and how many selling levels the grid posts.
- `max_position` is the hard cap on net inventory, long or short.

The relationship is arithmetic. The worst-case exposure on one side is `trade_size * num_levels`,
because that is the most the grid can accumulate before the other side starts reducing it. Set
`max_position` below that and the strategy skips levels, which is exactly what this run shows.

The program places one grid and never re-quotes (`requote_threshold_bps=1_000`), so the number of
orders is the number of levels that fit:

```python
def placed_grid(max_position, trade_size, count=300):
    engine = BacktestEngine(
        BacktestEngineConfig(
            trader_id=TraderId.from_str("BACKTESTER-001"),
            logging=LoggerConfig(stdout_level=LogLevel.OFF, print_config=False),
        ),
    )
    SIM = Venue("SIM")
    USD = Currency.from_str("USD")
    engine.add_venue(
        venue=SIM,
        oms_type=OmsType.NETTING,
        account_type=AccountType.MARGIN,
        base_currency=USD,
        starting_balances=[Money(10_000_000, USD)],
        fill_model=ProbabilisticFillModel(prob_fill_on_limit=1.0, prob_slippage=0.0, random_seed=42),
        fee_model=MakerTakerFeeModel(maker_rate=Decimal("0.00002"), taker_rate=Decimal("0.00002")),
    )
    USDJPY_SIM = TestInstrumentProvider.usdjpy_sim()
    engine.add_instrument(USDJPY_SIM)
    engine.add_data(TestDataProvider.usdjpy_quotes(count=count))
    engine.add_builtin_strategy(
        "GridMarketMaker",
        GridMarketMakerConfig(
            instrument_id=USDJPY_SIM.id,
            max_position=Quantity.from_int(max_position),
            trade_size=Quantity.from_int(trade_size),
            num_levels=3,
            grid_step_bps=3,
            skew_factor=0.0,
            requote_threshold_bps=1_000,
        ),
    )
    engine.run()
    report = engine.generate_orders_report()
    engine.reset()
    engine.dispose()
    return report


for max_pos in (500_000, 1_500_000):
    report = placed_grid(max_pos, 500_000)
    print(f"max_position={max_pos}, trade_size=500000")
    print(report[["side", "type", "quantity", "price", "status"]].to_string())
```

Observed:

```text
max_position=500000, trade_size=500000
                             side    type quantity    price    status
client_order_id
O-20190101-230000-001-001-1   BUY   LIMIT   500000  109.472  CANCELED
O-20190101-230000-001-001-2  SELL   LIMIT   500000  109.538    FILLED
O-20190101-230459-001-001-3   BUY  MARKET   500000      NaN    FILLED
max_position=1500000, trade_size=500000
                             side    type quantity    price    status
client_order_id
O-20190101-230000-001-001-1   BUY   LIMIT   500000  109.472  CANCELED
O-20190101-230000-001-001-2  SELL   LIMIT   500000  109.538    FILLED
O-20190101-230000-001-001-3   BUY   LIMIT   500000  109.439  CANCELED
O-20190101-230000-001-001-4  SELL   LIMIT   500000  109.571    FILLED
O-20190101-230000-001-001-5   BUY   LIMIT   500000  109.406  CANCELED
O-20190101-230000-001-001-6  SELL   LIMIT   500000  109.604    FILLED
O-20190101-230459-001-001-7   BUY  MARKET  1500000      NaN    FILLED
```

With `max_position=500,000` the grid placed one level per side. With `max_position=1,500,000` it
placed the full three per side. The cap did not shrink the orders; it stopped the strategy from
placing levels that could breach it.

The `BUY MARKET` order at the bottom of each run is the strategy's `on_stop` rule closing whatever
inventory had accumulated. A `MARKET` order always fills, and it pays the taker fee. On a venue
where the market has moved away, that closing order is where the largest single loss usually lands.

The implementation detail that makes this safer than it sounds: before placing each level the
strategy tracks the *worst case per side*, including orders that are still in flight from a cancel
that has not been acknowledged (`crates/trading/src/examples/strategies/grid_mm/strategy.rs`).
Cancel is asynchronous; a pending order can still fill. Counting it prevents momentary
over-exposure during a cancel-and-replace wave.

### Choosing the numbers

- `trade_size`: choose it so one full grid (`trade_size * num_levels`) is a position you are willing
  to hold through a bad day. If that thought is uncomfortable, it is too big.
- `num_levels`: more levels means more resting size and more inventory when the market trends. Three
  to five is typical for a beginner.
- `max_position`: at least `trade_size * num_levels`, and no larger than the position you can afford
  to lose on. This is not a suggestion the engine checks against your balance; it is the number you
  pick.

## Skew: leaning the grid away from your inventory

A symmetric grid is a fair coin: it accumulates whichever direction the market moves. **Skew** makes
the grid lean. When you are long you want to sell, so you move both quotes down: the sell side comes
closer to the market and the buy side moves away. When you are short you move both quotes up.

The formula in the shipped strategy is exactly one line
(`crates/trading/src/examples/strategies/grid_mm/strategy.rs`):

```text
skew = skew_factor * net_position
buy_n  = mid * (1 - grid_step_bps/10000)^n - skew
sell_n = mid * (1 + grid_step_bps/10000)^n - skew
```

`net_position` is your current inventory in instrument units (positive long, negative short), so
`skew` is in price units. Read that as "price shift per unit of inventory" and the scale matters
enormously. With a mid of `109.505` and `grid_step_bps=3`, one grid step is about `0.033` JPY. A
skew that is a meaningful fraction of a step is about `0.01` JPY. If your position is `500,000`
units, the `skew_factor` that produces `0.01` is `0.01 / 500000 = 0.00000002`.

| `net_position` | Desired `skew` (JPY) | `skew_factor` |
| -------------- | -------------------- | ------------- |
| 500,000        | 0.01                 | 0.00000002    |
| 1,500,000      | 0.05                 | 0.000000033   |
| 1,500,000      | 0.50                 | 0.000000333   |

This is a real trap in the canonical example
`examples/backtest/fx_market_maker_gbpusd_bars.py`: it uses `skew_factor=0.5`. Run the grid with
`skew_factor=0.5` on the USD/JPY sine data and watch what happens once a position exists:

```text
total orders: 2323
statuses: {'REJECTED': 2313, 'FILLED': 4, 'CANCELED': 3, 'DENIED': 3}
                              side   type quantity       price    status
client_order_id
O-20190101-230000-001-001-1    BUY  LIMIT   500000     109.472  CANCELED
O-20190101-230000-001-001-2   SELL  LIMIT   500000     109.538    FILLED
O-20190101-230000-001-001-3    BUY  LIMIT   500000     109.439  CANCELED
O-20190101-230000-001-001-4   SELL  LIMIT   500000     109.571    FILLED
O-20190101-230000-001-001-5    BUY  LIMIT   500000     109.406  CANCELED
O-20190101-230000-001-001-6   SELL  LIMIT   500000     109.604    FILLED
O-20190101-230348-001-001-7    BUY  LIMIT   500000  750109.692    DENIED
O-20190101-230348-001-001-8    BUY  LIMIT   500000  750109.659    DENIED
O-20190101-230348-001-001-9    BUY  LIMIT   500000  750109.626    DENIED
O-20190101-230349-001-001-10   BUY  LIMIT   500000  750109.693  REJECTED
```

The first grid behaves normally. Once the three sells have filled, the net position is
`-1,500,000` and the skew is `0.5 * -1500000 = -750000`. Buy prices become `109.4 - (-750000)`,
about `750,109`. Those are not real prices. The venue rejects them, the grid is empty, and the
strategy retries on every quote: 2,323 orders and 2,313 rejects in under three minutes. A
`skew_factor` that is not scaled to the instrument does not lean the grid, it destroys it.

Two practical rules:

- Set `skew_factor = 0.0` while learning. Turn it on only after you can read the positions report.
- Compute the maximum skew once: `skew_factor * max_position`. If that is more than `grid_step_bps`
  worth of price, the strategy can push its whole grid past the market and stop quoting.

Skew is not free. It trades fills for safety: a skewed grid fills less often on the side you want it
to avoid, and less often overall. That is the point, but it shows up as a lower fill count in your
reports.

## What the engine enforces, and what it does not

### What it enforces

The risk engine runs a set of checks before any order reaches the venue. In Python you can configure
three of them through `RiskEngineConfig` (`python/nautilus_trader/risk/__init__.pyi`):

| Setting                  | What it does                                                       |
| ------------------------ | ------------------------------------------------------------------ |
| `max_order_submit_rate`  | Caps order submissions per interval, for example `"100/00:00:01"`. |
| `max_order_modify_rate`  | Caps order modifies and cancels per interval.                      |
| `max_notional_per_order` | Caps the value of a single order, per instrument ID.               |

A program that sets `max_notional_per_order={"USD/JPY.SIM": 50_000_000}` and then tries to place
500,000-unit orders (about 54.7 million yen of value each) gets this:

```python
from decimal import Decimal

import pandas as pd

from nautilus_trader.backtest import BacktestEngine
from nautilus_trader.common import LogLevel
from nautilus_trader.common import LoggerConfig
from nautilus_trader.config import BacktestEngineConfig
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.execution import ProbabilisticFillModel
from nautilus_trader.model import AccountType
from nautilus_trader.model import Currency
from nautilus_trader.model import Money
from nautilus_trader.model import OmsType
from nautilus_trader.model import Quantity
from nautilus_trader.model import TraderId
from nautilus_trader.model import Venue
from nautilus_trader.risk import RiskEngineConfig
from nautilus_trader.testkit.providers import TestDataProvider
from nautilus_trader.testkit.providers import TestInstrumentProvider
from nautilus_trader.trading import GridMarketMakerConfig

engine = BacktestEngine(
    BacktestEngineConfig(
        trader_id=TraderId.from_str("BACKTESTER-001"),
        logging=LoggerConfig(stdout_level=LogLevel.OFF, print_config=False),
        risk_engine=RiskEngineConfig(
            max_order_submit_rate="100/00:00:01",
            max_notional_per_order={"USD/JPY.SIM": 50_000_000},
        ),
    ),
)
SIM = Venue("SIM")
USD = Currency.from_str("USD")
engine.add_venue(
    venue=SIM,
    oms_type=OmsType.NETTING,
    account_type=AccountType.MARGIN,
    base_currency=USD,
    starting_balances=[Money(10_000_000, USD)],
    fill_model=ProbabilisticFillModel(prob_fill_on_limit=1.0, prob_slippage=0.0, random_seed=42),
    fee_model=MakerTakerFeeModel(maker_rate=Decimal("0.00002"), taker_rate=Decimal("0.00002")),
)
USDJPY_SIM = TestInstrumentProvider.usdjpy_sim()
engine.add_instrument(USDJPY_SIM)
engine.add_data(TestDataProvider.usdjpy_quotes(count=300))
engine.add_builtin_strategy(
    "GridMarketMaker",
    GridMarketMakerConfig(
        instrument_id=USDJPY_SIM.id,
        max_position=Quantity.from_int(1_500_000),
        trade_size=Quantity.from_int(500_000),
        num_levels=3,
        grid_step_bps=3,
        skew_factor=0.0,
        requote_threshold_bps=1_000,
    ),
)
engine.run()
orders = engine.generate_orders_report()
pd.set_option("display.max_columns", None)
pd.set_option("display.width", 200)
print(orders[["side", "type", "quantity", "price", "status"]].head(6).to_string())
engine.reset()
engine.dispose()
```

Observed output (the first six of 1,800):

```text
                             side   type quantity    price  status
client_order_id
O-20190101-230000-001-001-1   BUY  LIMIT   500000  109.472  DENIED
O-20190101-230000-001-001-2  SELL  LIMIT   500000  109.538  DENIED
O-20190101-230000-001-001-3   BUY  LIMIT   500000  109.439  DENIED
O-20190101-230000-001-001-4  SELL  LIMIT   500000  109.571  DENIED
O-20190101-230000-001-001-5   BUY  LIMIT   500000  109.406  DENIED
O-20190101-230000-001-001-6  SELL  LIMIT   500000  109.604  DENIED
```

Every order is `DENIED` before it leaves the engine. Raise the cap to a very large number and the
same program places the grid normally. Two lessons:

1. The risk engine is your last line of defence against a mistaken size, and it works before the
   order reaches anyone else.
2. Because denied orders leave the grid empty, the strategy re-tries on every quote. This run
   produced about 1,800 denied orders in 300 seconds. A misconfigured cap becomes an order storm.
   Watch the denied count, not just the fills.

### What it does not enforce

- **No total-loss limit.** Nothing stops the strategy from losing more than a day, a month, or any
  other budget. `max_position` bounds exposure, not loss. Only a higher-level component that stops
  the run can do that.
- **No volatility or news gate.** The grid quotes through anything, including the moment a central
  bank speaks.
- **No kill switch in Python.** Nothing in the shipped Python surface flattens and stops a run. The
  pre-trade count caps described in `docs/design/vnpy_lessons_implementation.md` (the D1 item) are
  `RiskEngineConfig.count_caps`, settable from Python as `RiskCap` values, and they cap message
  counts rather than take you out of the market.

The honest summary: the engine enforces per-order and per-rate limits well, and leaves portfolio
survival to you.

## What happens on a venue disconnect

A disconnect is not a fill decision. The engine is explicit about this
(`docs/concepts/live.md`):

- A lost connection raises a socket-state event. `Connected` reports transport availability only. It
  does not mean authentication, subscriptions, or order flow recovered.
- A deliberate shutdown raises no disconnect event.
- A disconnect by itself does **not** reject, cancel, or resolve an in-flight command. Only stream
  updates, queries, or reconciliation provide that evidence (`docs/concepts/live.md`).
- Clients reconnect on heartbeat timeout. The default timeout is three heartbeat intervals, and
  `heartbeat_timeout_secs` overrides it.
- A strategy or actor can call `reconnect_socket(client_id, endpoint)` to reconnect one transport.
  This is fire-and-observe: a successful return means the command passed local validation, and the
  socket-state events confirm what happened.

For a market maker the danger is not the gap itself, it is the state when the feed comes back:

1. Your quotes may still be live at the venue while you cannot see the market.
2. The venue may have filled them.
3. Your local position may disagree with the venue's.

That third point is why **execution reconciliation** exists. At startup, reconciliation aligns
cached order and position state with venue reports before the trader components start; continuous
checks can then monitor in-flight orders, open orders, positions, and own order books
(`docs/concepts/live.md`, `docs/concepts/execution/reconciliation.md`).

The practical rules for a market making deployment:

- Cancel quotes on a disconnect, and re-quote only after reconciliation proves your position. A
  market maker quoting blind is quoting without a `max_position` in reality.
- Do not assume a stale order is dead. It is live until the venue says otherwise.
- Configure a heartbeat and a reconciliation run before you go live, not after the first surprise.

In a backtest, the engine's equivalent protections are the market-status rules
(`docs/concepts/backtesting/fill-models.md`): matching happens only while the market status is
`Open`, orders submitted while the market is not open are rejected with a message naming the
status, and expiration cancels open orders. There is no auction or uncrossing routine in the
matching engine, so a reopening market resumes ordinary matching.

## A production checklist

Before any real venue:

- [ ] `max_position` is a position you can lose on, not a position you can survive overnight.
- [ ] `trade_size * num_levels <= max_position`.
- [ ] `skew_factor * max_position` is smaller than one grid step.
- [ ] `max_notional_per_order` is set and tested by deliberately breaching it in a backtest.
- [ ] The denied-order count is monitored; a storm of denies means a misconfiguration, not a venue
      problem.
- [ ] Reconciliation and a heartbeat are configured, and you have a rule for what the strategy does
      after a disconnect.

Continue to [08-exercises.md](08-exercises.md).

Previous: [06-measure-and-evaluate.md](06-measure-and-evaluate.md) | Next: [08-exercises.md](08-exercises.md)
