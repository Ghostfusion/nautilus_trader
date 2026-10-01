# 05 - Build the strategy

This lecture builds the two-leg strategy in numbered steps. Each step has its code and the output
it produces. The complete program is printed in section 11, followed by the full run output.

## Step 1: read both files into quote ticks

```python
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
```

One call per venue, each with its own instrument id. The same function produces the Binance feed
and the Bybit feed; only the file name and the instrument id change.

```text
BINANCE feed: 240 quote ticks for BTCUSDT-PERP.BINANCE
BYBIT   feed: 240 quote ticks for BTCUSDT-PERP.BYBIT
```

## Step 2: build the engine with two venues

```python
FEE_RATES = {
    BINANCE: (Decimal("0.0002"), Decimal("0.0004")),
    BYBIT: (Decimal("0.0002"), Decimal("0.0006")),
}
```

```python
    for venue, (maker, taker) in FEE_RATES.items():
        engine.add_venue(
            venue=venue,
            oms_type=OmsType.NETTING,
            account_type=AccountType.MARGIN,
            base_currency=USDT,
            starting_balances=[Money(1_000_000, USDT)],
            fill_model=DefaultFillModel(prob_fill_on_limit=1.0, prob_slippage=0.0),
            fee_model=MakerTakerFeeModel(maker_rate=maker, taker_rate=taker),
        )
```

```text
venue BINANCE account: 1000000.00000000 USDT, maker 2 bps, taker 4 bps
venue BYBIT   account: 1000000.00000000 USDT, maker 2 bps, taker 6 bps
```

The two fee schedules differ on purpose. Binance taker is 4 bps, Bybit taker is 6 bps, and both
legs are market orders, so every round trip pays 20 bps in taker fees before any spread.

## Step 3: compute the spread series

```python
def mid(quote: QuoteTick) -> Decimal:
    return (quote.bid_price.as_decimal() + quote.ask_price.as_decimal()) / 2
```

```python
        spread_bps = (mid(bybit) - mid(binance)) / mid(binance) * Decimal(10_000)
        self._series.append(spread_bps)
```

Over the file this produces 240 aligned observations. Every 20th value:

```text
  8.59 11.62 14.04 15.41 15.46 14.19 11.82 8.82 5.77 3.28 1.81 1.67
  min=1.56 mean=9.16 max=15.62
```

This is the series the whole strategy is built on. It rises from 8.59 to a peak of 15.46, then
falls back to 1.67, and the full-file extremes are 1.56 and 15.62. An entry rule above 15.62 never
fires; an exit rule below 1.56 never closes.

## Step 4: subscribe to both venues and align the timestamps

```python
    def on_start(self) -> None:
        self.subscribe_quotes(self.config.binance_id)
        self.subscribe_quotes(self.config.bybit_id)

    def on_quote(self, quote: QuoteTick) -> None:
        self._latest[quote.instrument_id] = quote
        binance = self._latest.get(self.config.binance_id)
        bybit = self._latest.get(self.config.bybit_id)
        if binance is None or bybit is None:
            return
        if binance.ts_event != bybit.ts_event:
            return  # not yet aligned: wait for the same instant on both venues
```

```text
240 aligned observations out of 480 quote events
```

The engine delivers one callback per event, so 480 callbacks arrive. The timestamp test throws away
the half of them that would compute a spread from one fresh and one stale quote. Without it, the
strategy would evaluate the basis 480 times and could enter twice at the same second, once for each
venue's arrival order.

## Step 5: the entry rule

```python
        if not self._open and spread_bps >= self.config.entry_bps:
            self._entries += 1
            print(f"[{self._entries}] ENTER spread_bps={spread_bps:.2f}")
            self._open = True
            self._submit(self.config.binance_id, OrderSide.BUY)
            self._submit(self.config.bybit_id, OrderSide.SELL)
```

The rule is: when the BYBIT price is at least `entry_bps` above the BINANCE price, buy the cheap
venue and sell the expensive venue. With `entry_bps = 12`:

```text
[1] ENTER spread_bps=12.04
```

The `_open` flag is the state machine. It stops the strategy from re-entering while a pair is
already held. It is set before the orders are submitted, so it is a statement of intent, not a
confirmation. Section 10 shows why that distinction matters.

## Step 6: the exit rule

```python
        elif self._open and spread_bps <= self.config.exit_bps:
            print(f"[{self._entries}] EXIT spread_bps={spread_bps:.2f}")
            self._open = False
            self._submit(self.config.binance_id, OrderSide.SELL)
            self._submit(self.config.bybit_id, OrderSide.BUY)
```

With `exit_bps = 5`, the pair is closed when the basis has narrowed to 5 bps or less:

```text
[1] EXIT spread_bps=4.94
```

The exit reverses both legs: sell the instrument bought on BINANCE and buy back the instrument sold
on BYBIT. The basis at entry was 12.04 and at exit 4.94, so the capture on the mid is
`12.04 - 4.94 = 7.10` bps of the Binance price, about 4.5 USDT per BTC. Section 9 compares that to
what actually landed in the accounts.

## Step 7: submit both legs

```python
    def _submit(self, instrument_id: InstrumentId, side: OrderSide) -> None:
        instrument = self.cache.instrument(instrument_id)
        quantity = instrument.make_qty(self.config.trade_size)
        order = self.order_factory.market(instrument_id, side, quantity)
        self.submit_order(order)
```

Both legs are **market** orders, so both are takers and both pay the taker fee. `make_qty` rounds
the requested quantity to the instrument's lot step, which is `0.001` for both contracts, so
`Decimal("1.000")` becomes a valid `Quantity`.

There is no way to submit these two orders as one atomic unit. An order list requires every order
in it to use the same venue, and these two orders are on different venues; see
`docs/concepts/orders/advanced.md`. The strategy therefore submits them one after the other and
carries the leg risk itself.

## Step 8: run and read the fills report

```python
    engine.run()
    print(engine.generate_order_fills_report())
```

```text
                                  trader_id  ...   avg_px
client_order_id                              ...
O-20231114-221343-001-000-1  BACKTESTER-001  ...  64028.2
O-20231114-221343-001-000-2  BACKTESTER-001  ...  64104.0
O-20231114-221606-001-000-3  BACKTESTER-001  ...  63972.2
O-20231114-221606-001-000-4  BACKTESTER-001  ...  64005.1

[4 rows x 31 columns]
```

Four fills, two per venue. Order `-1` and `-3` are Binance, orders `-2` and `-4` are Bybit, in
submission order. The average prices are the actual traded prices:

- Binance entry `64028.2`, Binance exit `63972.2`, a gross loss of `56.0` per BTC on the long leg.
- Bybit entry `64104.0`, Bybit exit `64005.1`, a gross profit of `98.9` per BTC on the short leg.

The pair captured the basis, as intended: the short leg made more than the long leg lost. Section 9
shows that this was still not enough.

## Step 9: read the two-leg result

```python
    binance_report = engine.generate_account_report(BINANCE)
    bybit_report = engine.generate_account_report(BYBIT)
    print(binance_report)
    print(bybit_report)

    binance_pnl = Decimal(str(binance_report["total"].iloc[-1])) - Decimal(str(binance_report["total"].iloc[0]))
    bybit_pnl = Decimal(str(bybit_report["total"].iloc[-1])) - Decimal(str(bybit_report["total"].iloc[0]))
    print(f"binance_pnl={binance_pnl:.5f} bybit_pnl={bybit_pnl:.5f} net_pnl={binance_pnl + bybit_pnl:.5f}")
```

```text
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

The pair lost 85.16562 USDT. The trade did what it was designed to do and still lost money, because
of fees. The arithmetic:

```
binance: gross -56.0 - 2 * (0.0004 * ~64000) = -56.0 - 51.20016 = -107.20016
bybit  : gross +98.9 - 2 * (0.0006 * ~64050) = +98.9 - 76.86546 = +22.03454
net    : -107.20016 + 22.03454 = -85.16562
```

The Bybit leg is profitable in isolation and the Binance leg is not, and the sum is negative. The
reason is the fee schedule: Bybit's taker fee is 50 per cent higher than Binance's, and the leg with
the higher fee happens to be the profitable one. Lecture 06 unpacks this.

The `locked` column is worth reading as well. At the entry timestamp the BINANCE account shows
`160.0705` locked and the BYBIT account `160.26` locked: that is the initial margin reserved for
the open positions. Both return to zero when the pair closes, which confirms both legs are flat at
the end of the run.

## Step 10: what happens if the first leg is denied

Change one line at the top of the program:

```python
NOTIONAL_CAP = {"BTCUSDT-PERP.BINANCE": "1000"}  # denies any Binance leg over 1,000 USDT
```

and re-run. Everything else is unchanged.

```text
[1] ENTER spread_bps=12.04
DENIED BTCUSDT-PERP.BINANCE O-20231114-221343-001-000-1 reason=NOTIONAL_EXCEEDS_MAX_PER_ORDER: max=1000.00000000 USDT, notional=64028.20000000 USDT
[1] EXIT spread_bps=4.94
DENIED BTCUSDT-PERP.BINANCE O-20231114-221606-001-000-3 reason=NOTIONAL_EXCEEDS_MAX_PER_ORDER: max=1000.00000000 USDT, notional=63972.20000000 USDT
spread_bps: 240 aligned observations, every 20th shown
  8.59 11.62 14.04 15.41 15.46 14.19 11.82 8.82 5.77 3.28 1.81 1.67
  min=1.56 mean=9.16 max=15.62
                                  trader_id  ...   avg_px
client_order_id                              ...
O-20231114-221343-001-000-2  BACKTESTER-001  ...  64104.0
O-20231114-221606-001-000-4  BACKTESTER-001  ...  64005.1

[2 rows x 31 columns]
                                            type  ... is_snapshot
position_id                                       ...
BTCUSDT-PERP.BYBIT-CrossVenueBasis-000  Position  ...       False

[1 rows x 32 columns]
                                      total      locked  ... info base_currency
2023-11-14 22:13:20+00:00  1000000.00000000  0.00000000  ...   {}          USDT

[1 rows x 10 columns]
                                      total        locked  ... info base_currency
2023-11-14 22:13:20+00:00  1000000.00000000    0.00000000  ...   {}          USDT
2023-11-14 22:13:43+00:00   999961.53760000  160.26000000  ...   {}          USDT
2023-11-14 22:16:06+00:00  1000022.03454000    0.00000000  ...   {}          USDT

[3 rows x 10 columns]
binance_pnl=0.00000 bybit_pnl=22.03454 net_pnl=22.03454
```

What the engine did:

1. The Binance buy was **denied** by the risk engine, before it reached the matching engine. The
   reason is `NOTIONAL_EXCEEDS_MAX_PER_ORDER`, and the message states both the cap and the observed
   notional.
2. The second leg was still submitted and **filled**. Nothing cancelled it, and nothing told the
   strategy to stop. `_open` had already been set to `True` before either order was sent.
3. Between the entry and the exit the account held a naked Bybit short of 1 BTC with no hedge. In
   this fixture the price fell, so the unhedged leg made money; the result of the run is `+22.03`
   instead of `-85.17`. That outcome is luck, not design.
4. The positions report shows one position, not two. The Binance position never existed, so there
   was nothing to net.

This is the most important lesson in the manual: **the engine does not protect a pair trade.** It
treats each order independently, and a two-leg trade is an intention held in your code, not a
property of the engine. Lecture 07 covers how to defend it.

## 11. The complete program

```python
"""Cross-venue basis strategy: submit both legs, report the two-leg result."""

import csv
from decimal import Decimal
from pathlib import Path

from nautilus_trader.backtest import BacktestEngine
from nautilus_trader.common import LogLevel
from nautilus_trader.config import BacktestEngineConfig
from nautilus_trader.config import LoggerConfig
from nautilus_trader.config import RiskEngineConfig
from nautilus_trader.config import StrategyConfig
from nautilus_trader.execution import DefaultFillModel
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.model import AccountType
from nautilus_trader.model import CryptoPerpetual
from nautilus_trader.model import Currency
from nautilus_trader.model import InstrumentId
from nautilus_trader.model import Money
from nautilus_trader.model import OmsType
from nautilus_trader.model import OrderSide
from nautilus_trader.model import Price
from nautilus_trader.model import Quantity
from nautilus_trader.model import QuoteTick
from nautilus_trader.model import Symbol
from nautilus_trader.model import TraderId
from nautilus_trader.model import Venue
from nautilus_trader.testkit.providers import TestInstrumentProvider
from nautilus_trader.trading import Strategy

DATA = Path(
    "D:/Users/vince/PycharmProjects/nautilus_trader/docs/usermanauls/cross-venue-relative-value/sample_data",
)

BINANCE = Venue("BINANCE")
BYBIT = Venue("BYBIT")
USDT = Currency.from_str("USDT")
FEE_RATES = {
    BINANCE: (Decimal("0.0002"), Decimal("0.0004")),
    BYBIT: (Decimal("0.0002"), Decimal("0.0006")),
}
NOTIONAL_CAP = None  # set to a dict to switch the first leg's risk cap on

BTCUSDT_PERP_BINANCE = TestInstrumentProvider.btcusdt_perp_binance()
BTCUSDT_PERP_BYBIT = CryptoPerpetual(
    instrument_id=InstrumentId(Symbol("BTCUSDT-PERP"), BYBIT),
    raw_symbol=Symbol("BTCUSDT"),
    base_currency=Currency.from_str("BTC"),
    quote_currency=USDT,
    settlement_currency=USDT,
    is_inverse=False,
    price_precision=1,
    size_precision=3,
    price_increment=Price.from_str("0.1"),
    size_increment=Quantity.from_str("0.001"),
    ts_event=0,
    ts_init=0,
    max_quantity=Quantity.from_str("1000.000"),
    min_quantity=Quantity.from_str("0.001"),
    min_notional=Money(10.00, USDT),
    max_price=Price.from_str("809484.0"),
    min_price=Price.from_str("261.1"),
    margin_init=Decimal("0.0500"),
    margin_maint=Decimal("0.0250"),
)


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


def mid(quote: QuoteTick) -> Decimal:
    return (quote.bid_price.as_decimal() + quote.ask_price.as_decimal()) / 2


class CrossVenueBasisConfig(StrategyConfig):
    def __init__(
        self,
        *,
        binance_id: InstrumentId,
        bybit_id: InstrumentId,
        trade_size: Decimal,
        entry_bps: Decimal,
        exit_bps: Decimal,
        **_kwargs: object,
    ) -> None:
        super().__init__()
        self.binance_id = binance_id
        self.bybit_id = bybit_id
        self.trade_size = trade_size
        self.entry_bps = entry_bps
        self.exit_bps = exit_bps


class CrossVenueBasis(Strategy):
    def __init__(self, config: CrossVenueBasisConfig) -> None:
        super().__init__(config)
        self._latest: dict[InstrumentId, QuoteTick] = {}
        self._open = False
        self._entries = 0
        self._series: list[Decimal] = []

    def on_start(self) -> None:
        self.subscribe_quotes(self.config.binance_id)
        self.subscribe_quotes(self.config.bybit_id)

    def on_quote(self, quote: QuoteTick) -> None:
        self._latest[quote.instrument_id] = quote
        binance = self._latest.get(self.config.binance_id)
        bybit = self._latest.get(self.config.bybit_id)
        if binance is None or bybit is None:
            return
        if binance.ts_event != bybit.ts_event:
            return  # not yet aligned: wait for the same instant on both venues

        spread_bps = (mid(bybit) - mid(binance)) / mid(binance) * Decimal(10_000)
        self._series.append(spread_bps)

        if not self._open and spread_bps >= self.config.entry_bps:
            self._entries += 1
            print(f"[{self._entries}] ENTER spread_bps={spread_bps:.2f}")
            self._open = True
            self._submit(self.config.binance_id, OrderSide.BUY)
            self._submit(self.config.bybit_id, OrderSide.SELL)
        elif self._open and spread_bps <= self.config.exit_bps:
            print(f"[{self._entries}] EXIT spread_bps={spread_bps:.2f}")
            self._open = False
            self._submit(self.config.binance_id, OrderSide.SELL)
            self._submit(self.config.bybit_id, OrderSide.BUY)

    def on_stop(self) -> None:
        sampled = self._series[::20]
        print(f"spread_bps: {len(self._series)} aligned observations, every 20th shown")
        print("  " + " ".join(f"{value:.2f}" for value in sampled))
        print(
            f"  min={min(self._series):.2f} mean={sum(self._series) / len(self._series):.2f} "
            f"max={max(self._series):.2f}",
        )

    def _submit(self, instrument_id: InstrumentId, side: OrderSide) -> None:
        instrument = self.cache.instrument(instrument_id)
        quantity = instrument.make_qty(self.config.trade_size)
        order = self.order_factory.market(instrument_id, side, quantity)
        self.submit_order(order)

    def on_order_denied(self, event) -> None:
        print(f"DENIED {event.instrument_id} {event.client_order_id} reason={event.reason}")


def build_engine(notional_cap: dict[str, str] | None = None) -> BacktestEngine:
    engine = BacktestEngine(
        BacktestEngineConfig(
            trader_id=TraderId.from_str("BACKTESTER-001"),
            logging=LoggerConfig(stdout_level=LogLevel.ERROR),
            risk_engine=RiskEngineConfig(max_notional_per_order=notional_cap),
        ),
    )
    for venue, (maker, taker) in FEE_RATES.items():
        engine.add_venue(
            venue=venue,
            oms_type=OmsType.NETTING,
            account_type=AccountType.MARGIN,
            base_currency=USDT,
            starting_balances=[Money(1_000_000, USDT)],
            fill_model=DefaultFillModel(prob_fill_on_limit=1.0, prob_slippage=0.0),
            fee_model=MakerTakerFeeModel(maker_rate=maker, taker_rate=taker),
        )
    engine.add_instrument(BTCUSDT_PERP_BINANCE)
    engine.add_instrument(BTCUSDT_PERP_BYBIT)
    engine.add_data(load_quotes("btcusdt_perp_binance_quotes.csv", BTCUSDT_PERP_BINANCE.id))
    engine.add_data(load_quotes("btcusdt_perp_bybit_quotes.csv", BTCUSDT_PERP_BYBIT.id))
    engine.add_strategy(
        CrossVenueBasis(
            CrossVenueBasisConfig(
                binance_id=BTCUSDT_PERP_BINANCE.id,
                bybit_id=BTCUSDT_PERP_BYBIT.id,
                trade_size=Decimal("1.000"),
                entry_bps=Decimal("12"),
                exit_bps=Decimal("5"),
            ),
        ),
    )
    return engine


if __name__ == "__main__":
    engine = build_engine(NOTIONAL_CAP)
    engine.run()

    print(engine.generate_order_fills_report())
    print(engine.generate_positions_report())

    binance_report = engine.generate_account_report(BINANCE)
    bybit_report = engine.generate_account_report(BYBIT)
    print(binance_report)
    print(bybit_report)

    binance_pnl = Decimal(str(binance_report["total"].iloc[-1])) - Decimal(str(binance_report["total"].iloc[0]))
    bybit_pnl = Decimal(str(bybit_report["total"].iloc[-1])) - Decimal(str(bybit_report["total"].iloc[0]))
    print(f"binance_pnl={binance_pnl:.5f} bybit_pnl={bybit_pnl:.5f} net_pnl={binance_pnl + bybit_pnl:.5f}")

    engine.reset()
    engine.dispose()
```

Run it in the same way as lecture 03:

```bash
cd python
uv run --no-sync python C:/scratch/cross_venue_05.py
```

The full output of the unconstrained run, printed in section 8 and section 9, is reproduced in
[06-measure-and-evaluate.md](06-measure-and-evaluate.md) next to the interpretation.

Continue to [06-measure-and-evaluate.md](06-measure-and-evaluate.md).
