# 06 - Measure and evaluate

This lecture reads the result of the run built in lecture 05, explains why a hedged pair can still
lose money, and shows what a funding payment does to a held basis.

## 1. The run, end to end

Reproduced from lecture 05. The program is unchanged; `NOTIONAL_CAP` is `None`.

```text
[1] ENTER spread_bps=12.04
[1] EXIT spread_bps=4.94
spread_bps: 240 aligned observations, every 20th shown
  8.59 11.62 14.04 15.41 15.46 14.19 11.82 8.82 5.77 3.28 1.81 1.67
  min=1.56 mean=9.16 max=15.62
                                  trader_id  ...   avg_px
client_order_id                              ...
O-20231114-221343-001-000-1  BACKTESTER-001  ...  64028.2
O-20231114-221343-001-000-2  BACKTESTER-001  ...  64104.0
O-20231114-221606-001-000-3  BACKTESTER-001  ...  63972.2
O-20231114-221606-001-000-4  BACKTESTER-001  ...  64005.1

[4 rows x 31 columns]
                                              type  ... is_snapshot
position_id                                         ...
BTCUSDT-PERP.BINANCE-CrossVenueBasis-000  Position  ...       False
BTCUSDT-PERP.BYBIT-CrossVenueBasis-000    Position  ...       False

[2 rows x 32 columns]
                                      total        locked  ... info base_currency
2023-11-14 22:13:20+00:00  1000000.00000000    0.00000000  ...   {}          USDT
2023-11-14 22:13:43+00:00   999974.38872000  160.07050000  ...   {}          USDT
2023-11-14 22:16:06+00:00   999892.79984000    0.00000000  ...   {}          USDT

[3 rows x 10 columns]
                                      total        locked  ... info base_currency
2023-11-14 22:13:20+00:00  1000000.00000000    0.00000000  ...   {}          USDT
2023-11-14 22:13:43+00:00   999961.53760000  160.26000000  ...   {}          USDT
2023-11-14 22:16:06+00:00  1000022.03454000    0.00000000  ...   {}          USDT

[3 rows x 10 columns]
binance_pnl=-107.20016 bybit_pnl=22.03454 net_pnl=-85.16562
```

## 2. The four reports, and what each one answers

| Report                | Method                                                              | The question it answers                                  |
| --------------------- | ------------------------------------------------------------------- | -------------------------------------------------------- |
| Fills report          | `engine.generate_order_fills_report()`                              | What actually traded, at what price and size, per order. |
| Positions report      | `engine.generate_positions_report()`                                | What exposure existed, and how it closed.                |
| Account report        | `engine.generate_account_report(venue)`                             | What money sits on each venue, and how much is locked.   |
| Portfolio performance | printed by the engine at the end of a run (log level INFO or lower) | Trade-level statistics over closed positions.            |

The methods are declared in `python/nautilus_trader/backtest/__init__.pyi` and implemented in the
Rust backtest engine under `crates/backtest/`. The statistics classes behind the performance block
live in `crates/analysis/src/python/` and are exposed as `nautilus_trader.analysis`, with individual
statistics such as `Expectancy`, `ProfitFactor`, `MaxDrawdown` and `SharpeRatio` as separate
classes.

## 3. Reading the two-leg result

The fill report gives four prices:

| Order | Venue   | Side         | Price   | Money leg       |
| ----- | ------- | ------------ | ------- | --------------- |
| `-1`  | BINANCE | BUY (entry)  | 64028.2 | long 1.000 BTC  |
| `-2`  | BYBIT   | SELL (entry) | 64104.0 | short 1.000 BTC |
| `-3`  | BINANCE | SELL (exit)  | 63972.2 | closed          |
| `-4`  | BYBIT   | BUY (exit)   | 64005.1 | closed          |

Per leg, before fees:

```
binance gross = 63972.2 - 64028.2 = -56.0
bybit   gross = 64104.0 - 64005.1 = +98.9
gross pair    = -56.0 + 98.9 = +42.9 per BTC
```

That is the basis capture: the entry basis was `64104.0 - 64028.2 = 75.8` on the touched prices,
and the exit basis was `64005.1 - 63972.2 = 32.9`, so the pair made `75.8 - 32.9 = 42.9`. The trade
worked.

Now the fees, which is where the money went:

```
binance fees = 0.0004 * 64028.2 + 0.0004 * 63972.2 = 51.20016
bybit   fees = 0.0006 * 64104.0 + 0.0006 * 64005.1 = 76.86546
total fees   = 128.06562
net          = 42.9 - 128.06562 = -85.16562
```

Three times the captured basis was paid in taker fees. The pair was correct and the trade was
unprofitable, and both statements are true at the same time.

**Rule of thumb for this style:** a two-leg market-order trade costs
`fee_A + fee_B` per side, so `2 * (fee_A + fee_B)` per round trip. Here that is
`2 * (4 + 6) = 20` bps, and the strategy captured about 6.7 bps
(`42.9 / 64028.2 * 10,000`). The entry threshold was three times too small.

## 4. Why a hedged pair can still lose money

Five reasons, in the order they usually bite:

1. **Fees on both legs, twice.** You pay the taker fee on the entry and again on the exit, on both
   venues. This is the dominant cost above.
2. **A different fee on each venue.** The profitable leg here was BYBIT, whose taker fee is 6 bps;
   the unprofitable leg was BINANCE at 4 bps. A pair is only as good as the worse fee schedule,
   because the leg you make money on is not the leg you choose.
3. **The bid-ask spread on both legs.** The mid basis was larger than the executable basis by
   roughly two half-spreads at every decision.
4. **Divergence before convergence.** The basis can widen before it narrows. If your exit threshold
   or your stop is too tight, you exit at the worst point.
5. **Carry against you.** Funding can be paid by the side you are on. The next section measures it.

Being hedged removes **directional** risk, not cost. A pair trade is a bet that the difference
narrows faster than the cost accrues.

## 5. What a funding payment does to a held basis

A perpetual future has no expiry, so the venue replaces the expiry with a periodic payment between
longs and shorts, normally every eight hours. This is the mechanism that pulls the perpetual's price
toward the underlying, and it is also money that leaves or enters your account while you hold the
trade.

The data type is `FundingRateUpdate`, documented in
[`../../concepts/data/funding_rate_update.md`](../../concepts/data/funding_rate_update.md). Its
fields are `instrument_id`, `rate`, `ts_event`, `ts_init`, and optionally `interval` (the funding
interval in minutes) and `next_funding_ns` (the next funding timestamp). The page is explicit that
a funding rate is **reference data** and does not imply a payment was applied.

The backtest engine applies the payment. In `crates/backtest/src/exchange.rs`, a funding rate
defines a settlement boundary from `next_funding_ns` when it is present, and the engine settles
every open position in that instrument at the boundary, using the mark price or the top of book as
the settlement price. The payment arrives at the position as an adjustment of type `FUNDING`, which
[`../../concepts/positions.md`](../../concepts/positions.md) describes under "Funding payments": the
adjustment tracks the periodic payment **without changing the position quantity**.

Here is the effect measured directly. Save the program below outside the repository:

```python
"""Hold a perpetual long across one funding boundary and read the payment."""

import csv
from decimal import Decimal
from pathlib import Path

from nautilus_trader.backtest import BacktestEngine
from nautilus_trader.common import LogLevel
from nautilus_trader.config import BacktestEngineConfig
from nautilus_trader.config import LoggerConfig
from nautilus_trader.config import StrategyConfig
from nautilus_trader.execution import DefaultFillModel
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.model import AccountType
from nautilus_trader.model import Currency
from nautilus_trader.model import FundingRateUpdate
from nautilus_trader.model import InstrumentId
from nautilus_trader.model import Money
from nautilus_trader.model import OmsType
from nautilus_trader.model import OrderSide
from nautilus_trader.model import PositionId
from nautilus_trader.model import Price
from nautilus_trader.model import Quantity
from nautilus_trader.model import QuoteTick
from nautilus_trader.model import TraderId
from nautilus_trader.model import Venue
from nautilus_trader.testkit.providers import TestInstrumentProvider
from nautilus_trader.trading import Strategy

DATA = Path(
    "<repo-root>/docs/usermanauls/cross-venue-relative-value/sample_data",
)

BINANCE = Venue("BINANCE")
USDT = Currency.from_str("USDT")
BTCUSDT_PERP_BINANCE = TestInstrumentProvider.btcusdt_perp_binance()
START_NS = 1_700_000_000_000_000_000
FUNDING_BOUNDARY_NS = START_NS + 120_000_000_000


def load_quotes(filename: str, instrument_id: InstrumentId) -> list[QuoteTick]:
    ticks = []
    with (DATA / filename).open(encoding="ascii", newline="") as f:
        for row in csv.DictReader(f):
            ticks.append(
                QuoteTick(
                    instrument_id=instrument_id,
                    bid_price=Price.from_str(row["bid_price"]),
                    ask_price=Price.from_str(row["ask_price"]),
                    bid_size=Quantity.from_str(row["bid_size"]),
                    ask_size=Quantity.from_str(row["ask_size"]),
                    ts_event=int(row["ts_event_ns"]),
                    ts_init=int(row["ts_event_ns"]),
                ),
            )
    return ticks


class HoldLongConfig(StrategyConfig):
    def __init__(self, *, instrument_id: InstrumentId, **_kwargs: object) -> None:
        super().__init__()
        self.instrument_id = instrument_id


class HoldLong(Strategy):
    def __init__(self, config: HoldLongConfig) -> None:
        super().__init__(config)

    def on_start(self) -> None:
        instrument = self.cache.instrument(self.config.instrument_id)
        self.submit_order(
            self.order_factory.market(
                self.config.instrument_id,
                OrderSide.BUY,
                instrument.make_qty(Decimal("1.000")),
            ),
        )


if __name__ == "__main__":
    engine = BacktestEngine(
        BacktestEngineConfig(
            trader_id=TraderId.from_str("BACKTESTER-001"),
            logging=LoggerConfig(stdout_level=LogLevel.ERROR),
        ),
    )
    engine.add_venue(
        venue=BINANCE,
        oms_type=OmsType.NETTING,
        account_type=AccountType.MARGIN,
        base_currency=USDT,
        starting_balances=[Money(1_000_000, USDT)],
        fill_model=DefaultFillModel(prob_fill_on_limit=1.0, prob_slippage=0.0),
        fee_model=MakerTakerFeeModel(
            maker_rate=Decimal("0.0002"),
            taker_rate=Decimal("0.0004"),
        ),
    )
    engine.add_instrument(BTCUSDT_PERP_BINANCE)
    engine.add_data(load_quotes("btcusdt_perp_binance_quotes.csv", BTCUSDT_PERP_BINANCE.id))
    engine.add_data(
        [
            FundingRateUpdate(
                instrument_id=BTCUSDT_PERP_BINANCE.id,
                rate=Decimal("0.0001"),
                ts_event=START_NS,
                ts_init=START_NS,
                interval=480,
                next_funding_ns=FUNDING_BOUNDARY_NS,
            ),
        ],
    )
    engine.add_strategy(HoldLong(HoldLongConfig(instrument_id=BTCUSDT_PERP_BINANCE.id)))
    engine.run()

    position = engine.cache.position(
        PositionId("BTCUSDT-PERP.BINANCE-HoldLong-000"),
    )
    print(f"position signed_qty={position.signed_qty} avg_px_open={position.avg_px_open}")
    for adjustment in position.adjustments():
        print(
            f"adjustment type={adjustment.adjustment_type} "
            f"pnl_change={adjustment.pnl_change} reason={adjustment.reason}",
        )
    print(engine.generate_account_report(BINANCE))

    engine.reset()
    engine.dispose()
```

Run it, and the output is:

```text
position signed_qty=1.0 avg_px_open=64000.5
adjustment type=FUNDING pnl_change=-6.39697000 USDT reason=funding_settlement:46e7b3be-93dc-4d12-8daf-8c28f427fb0b
                                      total        locked  ... info base_currency
2023-11-14 22:13:20+00:00  1000000.00000000    0.00000000  ...   {}          USDT
2023-11-14 22:13:20+00:00   999974.39980000  160.00125000  ...   {}          USDT
2023-11-14 22:15:20+00:00   999968.00283000  160.00125000  ...   {}          USDT
2023-11-14 22:15:20+00:00   999968.00283000  160.00125000  ...   {}          USDT

[4 rows x 10 columns]
```

Two of the numbers in that output are fixed by the run and one is not. `signed_qty`, the prices, the
`pnl_change` and the balance figures are the same on every run; the UUID at the end of the `reason`
string is the funding event's own id, and it is different each time. The remaining facts are:

1. The position is `signed_qty=1.0`, opened at `64000.5`, and its quantity never changes. Funding
   does not resize a position.
2. One adjustment appeared, of type `FUNDING`, with `pnl_change=-6.39697000 USDT`. The reason
   string records the funding settlement and its event id.
3. The balance fell from `999974.39980000` (after the entry fee) to `999968.00283000` at
   `22:15:20`, exactly the funding boundary set in `next_funding_ns`. The difference is
   `6.39697000`, the funding payment.

The payment is `rate * settlement_price * quantity = 0.0001 * 63969.7 * 1.000`, where `63969.7` is
the mid at the boundary. A **long** position paid it, because the rate was positive. For a pair
trade, this is the point: if you are short the perpetual on the venue where funding is positive,
you **receive** the payment, and that payment is part of the trade's edge. If you are long it, the
funding is a cost that accrues while you wait for convergence.

Worked example for the pair in section 1: if the BYBIT short had been held across three funding
boundaries at a rate of `0.0001`, the payments would have been
`0.0001 * ~64000 * 1.000 * 3 = 19.2` USDT received, which covers a fifth of the 85.17 USDT loss.
Carry is real, and it is small next to taker fees.

## 6. The engine's own performance statistics, and how to misread them

Run lecture 05's program with `LoggerConfig(stdout_level=LogLevel.INFO)` and the engine prints a
portfolio performance block at the end. The text after the `nautilus_backtest::engine:` log prefix
is verbatim; the timestamps and prefixes are elided here for readability.

```text
Backtest range: 0 days 00:03:59.000000
Iterations: 480
Total events: 8
Total orders: 4
Total positions: 2
 PORTFOLIO PERFORMANCE
 PnL Statistics (USDT)
Avg Loser:                      -107.20
Avg Winner:                     22.03
Expectancy:                     -42.58
Max Loser:                      -107.20
Max Winner:                     22.03
Min Loser:                      -107.20
Min Winner:                     22.03
PnL (total):                    -85.17
PnL% (total):                   -0.00
Win Rate:                       0.50
 Returns Statistics
Profit Factor:                  0.00
Returns Kurtosis:               NaN
Returns Skewness:               NaN
Returns Volatility (252 days):  NaN
Sharpe Ratio (252 days):        NaN
```

What each number means, in plain words:

| Number                                                       | Meaning                                             | Good looks like                         | Bad looks like                                                                  |
| ------------------------------------------------------------ | --------------------------------------------------- | --------------------------------------- | ------------------------------------------------------------------------------- |
| `Iterations`                                                 | Data events replayed.                               | Matches your data row count: 480.       | A number smaller than your data, which means data was dropped.                  |
| `Total orders`                                               | Orders submitted.                                   | 4 for one round trip of a two-leg pair. | A number that keeps growing, which means the state machine is broken.           |
| `Total positions`                                            | Positions created.                                  | 2 for a two-venue pair.                 | 4 or 0, which means one leg opened twice or not at all.                         |
| `Avg Winner` / `Avg Loser`                                   | Mean PnL per winning and losing position.           | A winner much larger than the loser.    | A winner smaller than the loser, as here.                                       |
| `Expectancy`                                                 | Mean PnL per position, winners and losers together. | Positive.                               | Negative: `(-107.20 + 22.03) / 2 = -42.58`, the exact average of the two legs.  |
| `PnL (total)`                                                | Sum of realised PnL over closed positions.          | Positive.                               | `-85.17`.                                                                       |
| `Win Rate`                                                   | Share of positions that made money.                 | High, but meaningless alone.            | `0.50` here, which is one leg winning and one losing.                           |
| `Profit Factor`                                              | Gross profit divided by gross loss.                 | Above 1.                                | `0.00`, because the only loser's magnitude exceeds the winner's.                |
| `Sharpe Ratio`, `Returns Volatility`, `Skewness`, `Kurtosis` | Return-distribution statistics over the sample.     | Computed.                               | `NaN`, because two positions over four minutes cannot support these statistics. |

## 7. The three most common beginner misreadings

**Misreading 1: reading the two legs as two trades.** The performance block counts positions, and a
two-venue pair is two positions. `Win Rate: 0.50` does not mean the strategy wins half its trades.
It means that of the two legs of one trade, one leg made money. The correct unit of analysis for
this style is the **pair**, and the pair's PnL is the sum across venues, which the account reports
give: `-85.16562`. Always aggregate the legs before you judge the strategy.

**Misreading 2: treating `PnL% (total): -0.00` as "no loss".** It is `-85.17` on a starting balance
of `1,000,000` per venue, which rounds to zero per cent at the printed precision. The absolute
number is the only one worth reading at this scale, and even then it describes one four-minute
window in a synthetic fixture. A number that rounds to zero is not evidence of anything.

**Misreading 3: trusting a statistic that the sample cannot support.** `Sharpe Ratio (252 days):
NaN` and `Returns Volatility (252 days): NaN` are the engine refusing to compute an annualised
statistic from a four-minute sample. That is the honest answer, not a bug. The same refusal appears
in the design record for relative-value screens
(`docs/design/relative_value_screening.md`, sections 2.2 and 3.2): below a declared minimum the
estimate is not merely noisy, it is biased, and the repository's convention is to report that a
statistic cannot be computed rather than to print a number.

## 8. What to measure that the engine does not print

The engine reports what happened. For this style you need four more numbers, and they are all
computed by hand:

1. **Paper basis at signal, in bps.** At entry, `12.04`.
2. **Executable basis at signal, in bps.** The touch version, which is smaller. At entry the mid
   basis was 12.04; the executable version sells the BYBIT bid and buys the BINANCE ask, so it is
   lower by two half-spreads.
3. **Total cost in bps.** `2 * (fee_A + fee_B)` plus two half-spreads plus any carry paid.
4. **Ratio of 2 to 3.** If the captured basis is not several times the cost, the trade is a
   coin flip wearing a hedge.

Lecture 07 turns these four numbers into limits.

Continue to [07-risks-and-limits.md](07-risks-and-limits.md).
