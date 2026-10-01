# 05 - Build the strategy

Lecture `03` gave you one working program. This lecture builds the same program in numbered steps,
tunes it, and adds the two passive-quoting execution algorithms. Every step shows the code and the
real output it produced.

The strategy is the shipped grid market maker, registered by name. You are not writing a strategy
class; you are configuring and measuring one, then learning where its limits are.

## Step 1: the venue

A market maker must be a maker. Two venue settings decide that:

- The fill model decides whether a resting order fills.
- The fee model decides what each fill costs.

```python
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
```

`prob_fill_on_limit=1.0` makes the run deterministic and easy to reason about: a touched order
fills. `random_seed=42` pins the remaining draws. `prob_slippage=0.0` keeps the first lessons about
the spread rather than about slippage. Lecture `07` re-adds both.

## Step 2: the data

```python
USDJPY_SIM = TestInstrumentProvider.usdjpy_sim()
engine.add_instrument(USDJPY_SIM)
engine.add_data(TestDataProvider.usdjpy_quotes(count=10_000))
```

10,000 one-second quotes is about two hours and forty-seven minutes of sine-wave movement between
about `109.0` and `110.0`. That is long enough for the grid to accumulate and unload inventory
several times, which is what makes the next step worth reading.

To use the committed sample instead, replace the last line with the CSV loader from lecture `04`.

## Step 3: the strategy

```python
engine.add_builtin_strategy(
    "GridMarketMaker",
    GridMarketMakerConfig(
        instrument_id=USDJPY_SIM.id,
        max_position=Quantity.from_int(1_500_000),
        trade_size=Quantity.from_int(500_000),
        num_levels=3,
        grid_step_bps=3,
        skew_factor=0.0,
        requote_threshold_bps=20,
    ),
)
```

| Field                   | Meaning                                      | How to choose it                             |
| ----------------------- | -------------------------------------------- | -------------------------------------------- |
| `instrument_id`         | Instrument to quote.                         | Required. Must be added to the engine first. |
| `max_position`          | Hard cap on net inventory, long or short.    | At least `trade_size * num_levels`.          |
| `trade_size`            | Size of each level.                          | Your risk unit, not the venue minimum.       |
| `num_levels`            | Buy levels below mid, sell levels above.     | More levels means more resting size.         |
| `grid_step_bps`         | Spacing between levels, in basis points.     | Wider fills less but captures more.          |
| `skew_factor`           | Price shift per unit of net position.        | Start at `0.0`; see lecture `07`.            |
| `requote_threshold_bps` | Mid move that triggers a cancel and replace. | Small means responsive and churny.           |
| `expire_time_secs`      | Optional order expiry; `GTD` when set.       | `None` means `GTC`.                          |
| `on_cancel_resubmit`    | Rebuild the grid after an unexpected cancel. | Useful on venues that expire orders.         |

The remaining two fields, `use_uuid_client_order_ids` and `use_hyphens_in_client_order_ids`, control
order ID formatting and do not affect behavior
(`python/nautilus_trader/trading/__init__.pyi`).

There is one sizing rule the arithmetic makes obvious. `max_position` caps the *net* position, but
the strategy tracks the worst case per side. With `trade_size=500_000` and `num_levels=3`, the
worst-case long is three buys, `1,500,000`, and the cap must be at least that or levels get skipped.
Lecture `07` shows a run where it is not.

## Step 4: run a full session

```python
engine.run()
fills = engine.generate_order_fills_report()
positions = engine.generate_positions_report()
```

Observed output over the 10,000 quotes:

```text
orders that filled: 40
position cycles: 7
liquidity mix: {'MAKER': 39, 'TAKER': 1}

                                                             entry  side quantity  avg_px_open  avg_px_close realized_pnl commissions
position_id
USD/JPY.SIM-GRID_MM-001-6194fdb1-74a7-4fc6-af17-9b1c2c3c8800  SELL  FLAT        0   109.571333    109.879000  -468082 JPY  [6583 JPY]
USD/JPY.SIM-GRID_MM-001-39a8896a-b6ae-44c8-b537-a4ae1b396fb1   BUY  FLAT        0   109.774667    109.248667  -795569 JPY  [6570 JPY]
USD/JPY.SIM-GRID_MM-001-89167f75-2158-4ec7-a3ba-9cc741a4eb58  SELL  FLAT        0   109.353333    109.880000  -796576 JPY  [6577 JPY]
USD/JPY.SIM-GRID_MM-001-2268ca34-d8ca-4b84-8325-057a65191d9f   BUY  FLAT        0   109.775667    109.249333  -796069 JPY  [6570 JPY]
USD/JPY.SIM-GRID_MM-001-da787a04-8237-4918-9d23-63c1ada8a334  SELL  FLAT        0   109.354000    109.881000  -797077 JPY  [6577 JPY]
USD/JPY.SIM-GRID_MM-001-5a109bf0-2d6c-4d4b-b45b-72a22fb57f49   BUY  FLAT        0   109.777000    109.250333  -796571 JPY  [6571 JPY]
USD/JPY.SIM-GRID_MM-001                                       SELL  FLAT        0   109.354000    109.967000  -926080 JPY  [6580 JPY]

total realized PnL across cycles:
-5376024 JPY
```

Read this honestly. The strategy filled 40 orders, 39 as a maker. Every one of the seven position
cycles lost money. The total loss is about `5.38` million JPY.

Why? Look at the entry and exit prices of the second cycle: the strategy bought at `109.774667` and
sold at `109.248667`. The market rose while the sell orders were being hit and fell while the buy
orders were being hit. That is adverse selection at scale: on a smooth sine wave, the grid is always
on the wrong side of the next move.

This is the central lesson of the course. A market making strategy is not supposed to make money on
trending data. It makes money when the price oscillates around its quotes, and loses when it runs.
The lectures from here on are about measuring and bounding that loss.

## Step 5: see the difference `queue_position` makes

Real fills depend on your place in the queue. If you are first in line at a price, the next trade
fills you. If twenty million units are ahead of you, the price must trade through that depth before
your order can fill. The venue setting `queue_position` (default `false`) turns this tracking on
(`docs/concepts/backtesting/fill-models.md`).

Queue tracking needs trade data, because the engine counts down the depth ahead as trades print. The
program below adds 2,000 synthetic trade ticks at the touch and runs the same grid twice, once with
the option off and once on.

```python
import math

import pandas as pd

from nautilus_trader.model import AggressorSide
from nautilus_trader.model import Price
from nautilus_trader.model import TradeId
from nautilus_trader.model import TradeTick


BASE_NS = 1_546_383_600_000_000_000


def make_trades(instrument_id, count):
    trades = []
    for i in range(count):
        ts = BASE_NS + i * 1_000_000_000
        mid = 109.505 + 0.5 * math.sin(i / 500.0)
        if i % 2 == 0:
            price, side = mid + 0.01, AggressorSide.BUY
        else:
            price, side = mid - 0.01, AggressorSide.SELL
        trades.append(
            TradeTick(
                instrument_id=instrument_id,
                price=Price(price, precision=3),
                size=Quantity.from_int(250_000 + 10_000 * (i % 3)),
                aggressor_side=side,
                trade_id=TradeId(f"T-{i:06d}"),
                ts_event=ts,
                ts_init=ts,
            ),
        )
    return trades


def run(queue_position):
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
        queue_position=queue_position,
    )
    USDJPY_SIM = TestInstrumentProvider.usdjpy_sim()
    engine.add_instrument(USDJPY_SIM)
    engine.add_data(TestDataProvider.usdjpy_quotes(count=2_000))
    engine.add_data(make_trades(USDJPY_SIM.id, 2_000))
    engine.add_builtin_strategy(
        "GridMarketMaker",
        GridMarketMakerConfig(
            instrument_id=USDJPY_SIM.id,
            max_position=Quantity.from_int(1_500_000),
            trade_size=Quantity.from_int(500_000),
            num_levels=3,
            grid_step_bps=3,
            skew_factor=0.0,
            requote_threshold_bps=20,
        ),
    )
    engine.run()
    report = engine.generate_order_fills_report()
    engine.reset()
    engine.dispose()
    return report


pd.set_option("display.max_columns", None)
pd.set_option("display.width", 200)

for qp in (False, True):
    report = run(qp)
    print(f"queue_position={qp}  orders filled: {len(report)}")
    print(
        report[["side", "price", "avg_px", "commissions", "ts_last"]]
        .head(6)
        .to_string(),
    )
    print()
```

Observed output for the two modes:

```text
queue_position=False  orders filled: 10
                              side    price   avg_px commissions                   ts_last
client_order_id
O-20190101-230000-001-001-2   SELL  109.538  109.538  [1095 JPY] 2019-01-01 23:00:24+00:00
O-20190101-230000-001-001-4   SELL  109.571  109.572  [1096 JPY] 2019-01-01 23:00:58+00:00
O-20190101-230000-001-001-6   SELL  109.604  109.605  [1096 JPY] 2019-01-01 23:01:30+00:00
O-20190101-230857-001-001-10   BUY  109.912  109.912  [1099 JPY] 2019-01-01 23:17:57+00:00
O-20190101-230857-001-001-11   BUY  109.879  109.879  [1099 JPY] 2019-01-01 23:18:53+00:00
O-20190101-230857-001-001-12   BUY  109.846  109.846  [1098 JPY] 2019-01-01 23:19:43+00:00

queue_position=True  orders filled: 10
                              side    price     avg_px commissions                   ts_last
client_order_id
O-20190101-230000-001-001-2   SELL  109.538    109.539  [1096 JPY] 2019-01-01 23:00:26+00:00
O-20190101-230000-001-001-4   SELL  109.571  109.57344  [1096 JPY] 2019-01-01 23:01:00+00:00
O-20190101-230000-001-001-6   SELL  109.604  109.60550  [1096 JPY] 2019-01-01 23:01:32+00:00
O-20190101-230857-001-001-10   BUY  109.912  109.91154  [1100 JPY] 2019-01-01 23:18:01+00:00
O-20190101-230857-001-001-11   BUY  109.879  109.87704  [1098 JPY] 2019-01-01 23:18:57+00:00
O-20190101-230857-001-001-12   BUY  109.846  109.84552  [1098 JPY] 2019-01-01 23:19:45+00:00
```

Compare the same order, `O-...-001-2`:

- Off: filled at `23:00:24`, average price `109.538`, commission `1095 JPY`.
- On: filled at `23:00:26`, average price `109.539`, commission `1096 JPY`.

The order waited two seconds longer for the depth ahead of it to trade through, then filled at a
slightly different average price. Every fill shifted later and the commissions changed. The
`queue_position` option does not change how many orders fill here; it changes *when* and *at what
price*, which is exactly the difference a passive strategy cares about.

Practical rule: leave `queue_position` off while learning or while comparing strategies, because it
makes results depend on trade data you may not have. Turn it on before you believe a passive
strategy's fill prices.

## Step 6: passive quoting with execution algorithms

The grid is one way to quote passively. The repository also ships two execution algorithms that
split or repeg orders for you (`docs/concepts/execution/algorithms.md`). Both are registered by name
on the backtest engine (`crates/backtest/src/python/engine.rs:1203`).

### Iceberg

The iceberg algorithm sends one child at a time for `min(remaining, display_size)`. It needs
`display_size` and `requote_secs` in `exec_algorithm_params`
(`crates/trading/src/algorithm/iceberg.rs`).

```python
engine.add_native_exec_algorithm(
    "IcebergAlgorithm",
    ExecutionAlgorithmConfig(exec_algorithm_id=ExecAlgorithmId("ICEBERG-DEMO")),
)
```

A strategy submits a parent order routed to that algorithm:

```python
order = self.order_factory.limit(
    instrument_id=self.config.instrument_id,
    order_side=OrderSide.SELL,
    quantity=Quantity.from_int(400_000),
    price=Price.from_str("110.500"),
    post_only=True,
    exec_algorithm_id=ExecAlgorithmId("ICEBERG-DEMO"),
    exec_algorithm_params={"display_size": "100000", "requote_secs": "5"},
)
self.submit_order(order)
```

Observed output over 1,000 quotes:

```text
                                side   type quantity filled_qty    price       status exec_algorithm_id                exec_spawn_id
client_order_id
O-20190101-230000-001-000-1     SELL  LIMIT   300000          0  110.500  INITIALIZED      ICEBERG-DEMO  O-20190101-230000-001-000-1
O-20190101-230000-001-000-1-E1  SELL  LIMIT   100000          0  110.500     ACCEPTED      ICEBERG-DEMO  O-20190101-230000-001-000-1
total orders: 2
child orders with an exec_spawn_id: 2
```

The parent was 400,000. The algorithm sent one child of 100,000 (the `display_size`) and the
parent's remaining quantity is now 300,000. The child's ID, `...-E1`, comes from the parent ID plus
a spawn sequence, and the primary keeps `exec_spawn_id` pointing at itself
(`docs/concepts/execution/algorithms.md`). The parent shows `INITIALIZED` rather than `ACCEPTED`
while the child holds the resting quote.

### Quote pegged

The quote-pegged algorithm keeps one child working at the touch and re-quotes it when the touch
moves. It needs `pegging` (`passive` or `join`) and `requote_secs`
(`crates/trading/src/algorithm/quote_pegged.rs`).

```python
engine.add_native_exec_algorithm(
    "QuotePeggedAlgorithm",
    ExecutionAlgorithmConfig(exec_algorithm_id=ExecAlgorithmId("PEG-DEMO")),
)
```

With `pegging="passive"` and `requote_secs=2` over 1,000 quotes, the same parent requested 300,000:

```text
QuotePeggedAlgorithm (pegging=passive, requote_secs=2)
total orders: 376  child orders: 375
first child:
                                side   type quantity    price    status
client_order_id
O-20190101-230000-001-000-1-E1  SELL  LIMIT   300000  109.510  CANCELED
last three children:
                                 side   type quantity    price    status
client_order_id
O-20190101-230000-001-000-1-E97  SELL  LIMIT   300000  109.697  CANCELED
O-20190101-230000-001-000-1-E98  SELL  LIMIT   300000  109.699  CANCELED
O-20190101-230000-001-000-1-E99  SELL  LIMIT   300000  109.701  CANCELED
distinct child prices: 330
```

This is the same two-hour session that filled 10 orders in step 5. The quote-pegged parent produced
375 child orders and 330 distinct prices. That is churn: every re-quote is a cancel plus a submit,
and on a real venue that costs messages and, on some venues, fees. A pegged algorithm on a fast
market needs a `requote_secs` large enough that the touch does not move on every child.

## Step 7: what to change first

If you are tuning this strategy, change the parameters in this order:

1. `requote_threshold_bps`. It controls churn more than anything else. `20` is calm; `2` re-quotes
   almost every tick.
2. `grid_step_bps`. Wider captures more per fill and fills less often. `3` is tight for FX; `10` is
   calmer.
3. `num_levels` and `trade_size`, together with `max_position`. More levels means more resting size
   and more inventory if the market runs.
4. `skew_factor` last, once you can measure inventory. Lecture `07` explains the scale.

Continue to [06-measure-and-evaluate.md](06-measure-and-evaluate.md) to understand every number in
those reports.

Previous: [04-sample-data.md](04-sample-data.md) | Next: [06-measure-and-evaluate.md](06-measure-and-evaluate.md)
