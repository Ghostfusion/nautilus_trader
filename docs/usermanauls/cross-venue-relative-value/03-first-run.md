# 03 - First run

This lecture installs nothing new, starts a two-venue backtest engine, feeds it two committed CSV
files and prints the basis between the two venues. It is the smallest program in this manual that
does something a relative-value trader would recognise.

## 1. Prepare the environment

The repository keeps its Python environment in `python/.venv`, and this project's `uv` binary lives
at a fixed path. Open a Git Bash terminal at the repository root and set the path once:

```bash
export PATH="C:/Users/vince/.cargo/bin;C:/Users/vince/.local/uv012;C:/Users/vince/AppData/Local/Programs/Python/Python312/cpython-3.14-windows-x86_64-none;$PATH"
cd python
uv run --no-sync python --version
```

```text
Python 3.14.0
```

`--no-sync` tells `uv` to use the environment exactly as it is and not to resolve or install
anything. Everything in this manual runs offline.

## 2. Save the program

Save the program below in a scratch folder **outside** the repository, for example
`C:/scratch/cross_venue_01.py`. Do not put `.py` files inside `docs/`.

One line needs your attention: `DATA`. It points at the `sample_data` folder of this manual. Change
it to wherever your copy of the repository lives. The rest of the program works unchanged.

```python
"""Smallest complete cross-venue program: read two venues, print the spread."""

import csv
from decimal import Decimal
from pathlib import Path

from nautilus_trader.backtest import BacktestEngine
from nautilus_trader.config import BacktestEngineConfig
from nautilus_trader.common import LogLevel
from nautilus_trader.config import LoggerConfig
from nautilus_trader.config import StrategyConfig
from nautilus_trader.execution import DefaultFillModel
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.model import AccountType
from nautilus_trader.model import CryptoPerpetual
from nautilus_trader.model import Currency
from nautilus_trader.model import InstrumentId
from nautilus_trader.model import Money
from nautilus_trader.model import OmsType
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


class SpreadMonitorConfig(StrategyConfig):
    def __init__(
        self,
        *,
        binance_id: InstrumentId,
        bybit_id: InstrumentId,
        max_prints: int = 5,
        **_kwargs: object,
    ) -> None:
        super().__init__()
        self.binance_id = binance_id
        self.bybit_id = bybit_id
        self.max_prints = max_prints


class SpreadMonitor(Strategy):
    def __init__(self, config: SpreadMonitorConfig) -> None:
        super().__init__(config)
        self._latest: dict[InstrumentId, QuoteTick] = {}
        self._prints = 0

    def on_start(self) -> None:
        self.subscribe_quotes(self.config.binance_id)
        self.subscribe_quotes(self.config.bybit_id)

    def on_quote(self, quote: QuoteTick) -> None:
        self._latest[quote.instrument_id] = quote
        if len(self._latest) < 2 or self._prints >= self.config.max_prints:
            return

        binance = self._latest[self.config.binance_id]
        bybit = self._latest[self.config.bybit_id]
        binance_mid = (binance.bid_price.as_decimal() + binance.ask_price.as_decimal()) / 2
        bybit_mid = (bybit.bid_price.as_decimal() + bybit.ask_price.as_decimal()) / 2
        spread = bybit_mid - binance_mid
        spread_bps = spread / binance_mid * Decimal(10_000)
        touch = bybit.bid_price.as_decimal() - binance.ask_price.as_decimal()

        print(
            f"binance_mid={binance_mid} bybit_mid={bybit_mid} "
            f"mid_spread={spread} mid_spread_bps={spread_bps:.2f} "
            f"touch_spread_after_crossing={touch}",
        )
        self._prints += 1


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
    engine.add_venue(
        venue=BYBIT,
        oms_type=OmsType.NETTING,
        account_type=AccountType.MARGIN,
        base_currency=USDT,
        starting_balances=[Money(1_000_000, USDT)],
        fill_model=DefaultFillModel(prob_fill_on_limit=1.0, prob_slippage=0.0),
        fee_model=MakerTakerFeeModel(
            maker_rate=Decimal("0.0002"),
            taker_rate=Decimal("0.0006"),
        ),
    )

    engine.add_instrument(BTCUSDT_PERP_BINANCE)
    engine.add_instrument(BTCUSDT_PERP_BYBIT)
    engine.add_data(load_quotes("btcusdt_perp_binance_quotes.csv", BTCUSDT_PERP_BINANCE.id))
    engine.add_data(load_quotes("btcusdt_perp_bybit_quotes.csv", BTCUSDT_PERP_BYBIT.id))

    engine.add_strategy(
        SpreadMonitor(
            SpreadMonitorConfig(
                binance_id=BTCUSDT_PERP_BINANCE.id,
                bybit_id=BTCUSDT_PERP_BYBIT.id,
            ),
        ),
    )
    engine.run()

    print(engine.generate_account_report(BINANCE))
    engine.reset()
    engine.dispose()
```

## 3. Run it

```bash
cd python
uv run --no-sync python C:/scratch/cross_venue_01.py
```

```text
binance_mid=64000.0 bybit_mid=64055.0 mid_spread=55.0 mid_spread_bps=8.59 touch_spread_after_crossing=53.7
binance_mid=64001.3 bybit_mid=64055.0 mid_spread=53.7 mid_spread_bps=8.39 touch_spread_after_crossing=52.4
binance_mid=64001.3 bybit_mid=64057.3 mid_spread=56.0 mid_spread_bps=8.75 touch_spread_after_crossing=54.7
binance_mid=64002.7 bybit_mid=64057.3 mid_spread=54.6 mid_spread_bps=8.53 touch_spread_after_crossing=53.3
binance_mid=64002.7 bybit_mid=64059.7 mid_spread=57.0 mid_spread_bps=8.91 touch_spread_after_crossing=55.7
                                      total      locked  ... info base_currency
2023-11-14 22:13:20+00:00  1000000.00000000  0.00000000  ...   {}          USDT

[1 rows x 10 columns]
```

Read the five lines in order:

1. Both venues are around 64,000 USDT per BTC.
2. The BYBIT mid is 55.0 USDT above the BINANCE mid at the first observation. That is the basis.
3. In basis points it is 8.59, which is `55.0 / 64000.0 * 10,000`.
4. The executable spread is 53.7, not 55.0, because you buy at the BINANCE ask and sell at the
   BYBIT bid. Two half-spreads have already been paid, and the trade has not been sent yet.
5. The account report shows the venue's balance at the end of the run: 1,000,000 USDT, unchanged,
   because this program places no orders.

Notice lines 2 and 3, and again lines 4 and 5: two observations with the same timestamp appear as
two prints. The engine delivers each venue's quote as a separate event, so `on_quote` runs once for
`BINANCE` and once for `BYBIT` at each second. The second print recomputes a spread from one fresh
and one stale quote. Lecture 05 fixes this by acting only when the two venues' timestamps match.

## 4. Line-by-line

### Imports

- `csv` reads the sample files. `Decimal` is used for money because binary floating point cannot
  represent `0.1` exactly, and a basis of one tick must not round to zero. `Path` handles file
  paths.
- `BacktestEngine` and `BacktestEngineConfig` are the engine and its settings.
- `LogLevel` and `LoggerConfig` turn the engine's own log output down to errors, so the teaching
  output is readable. Remove the `logging` argument and the run will print the full engine banner,
  system specification and per-command logs.
- `StrategyConfig` and `Strategy` are the base classes for the strategy.
- `DefaultFillModel` and `MakerTakerFeeModel` are the simulated fill and fee behaviour of each
  venue. `add_venue` refuses to create a venue without an explicit fee model.
- The `nautilus_trader.model` names are the value types: `Price`, `Quantity`, `Money`, `Currency`,
  `Venue`, `InstrumentId`, `Symbol`, `QuoteTick`, and the enums `AccountType` and `OmsType`.
- `TestInstrumentProvider` supplies the Binance perpetual. The provider is part of the shipped
  test kit at `python/nautilus_trader/testkit/providers.py`.

### Constants

```python
DATA = Path(...)
BINANCE = Venue("BINANCE")
BYBIT = Venue("BYBIT")
USDT = Currency.from_str("USDT")
```

`Venue` and `Currency` are interned value objects: `Currency.from_str("USDT")` always returns the
same `USDT`, and equality is by value. `Venue("BYBIT")` is the venue id used in every instrument id
and order on that venue.

### The two instruments

```python
BTCUSDT_PERP_BINANCE = TestInstrumentProvider.btcusdt_perp_binance()
BTCUSDT_PERP_BYBIT = CryptoPerpetual(...)
```

The Binance contract comes from the test kit. The Bybit contract is constructed by hand, because
the test kit ships no such instrument, and a cross-venue manual needs two venues. Every field is
copied from the Binance contract so the two are comparable: same base and quote currency, same
price precision of 1, same size precision of 3, same tick of 0.1 and same lot step of 0.001. The
`instrument_id` is the only difference that matters to the engine.

`ts_event=0` and `ts_init=0` are acceptable for a static instrument in a backtest.

### `load_quotes`

```python
def load_quotes(filename, instrument_id) -> list[QuoteTick]:
```

It opens the CSV with `encoding="ascii"` and `newline=""`, iterates `csv.DictReader` rows, and
builds one `QuoteTick` per row. The timestamp column `ts_event_ns` is an integer number of
nanoseconds since 1 January 1970, which is the unit the engine uses everywhere. `ts_init` is set
to the same value as `ts_event`, because in a backtest the data arrives when it happened.

### `SpreadMonitorConfig`

```python
class SpreadMonitorConfig(StrategyConfig):
    def __init__(self, *, binance_id, bybit_id, max_prints=5, **_kwargs): ...
```

A strategy config is a plain class holding the parameters. The `**_kwargs` tail lets the framework
pass extra fields without an error. `super().__init__()` must be called first.

### `SpreadMonitor`

- `on_start` subscribes to the quote stream of both instruments. A subscription is per instrument
  and per venue; there is no "both venues" subscription.
- `on_quote` is called once per quote event, from either venue. It stores the quote in
  `self._latest` keyed by instrument id, so the dictionary always holds the newest quote of each
  venue.
- `len(self._latest) < 2` blocks the calculation until both venues have produced at least one
  quote.
- `max_prints` keeps the output short. Remove the condition and every one of the 480 events prints.
- The mid price is computed from `as_decimal()`, so the arithmetic is exact decimal arithmetic.
- `spread` is the basis in USDT, `spread_bps` divides by the Binance mid and multiplies by 10,000,
  and `touch` is the executable version: sell the expensive venue at its bid, buy the cheap venue
  at its ask.
- `print` is used instead of the strategy logger so the output is deterministic and easy to paste.

### `__main__`

1. `BacktestEngine(BacktestEngineConfig(...))` builds the engine.
2. `add_venue` twice: one simulated venue per real venue, each with `OmsType.NETTING`, a `MARGIN`
   account, USDT as the base currency, one million USDT of starting balance, a deterministic fill
   model, and its own maker and taker fee rates. The two fee rates differ on purpose: Binance
   charges 4 bps to take, Bybit charges 6 bps.
3. `add_instrument` twice, then `add_data` twice.
4. `add_strategy` registers the `SpreadMonitor`.
5. `run()` replays the 480 data events in timestamp order.
6. `generate_account_report(BINANCE)` returns a pandas DataFrame; printing it is the fastest way to
   see the venue's balance. `reset()` clears the engine's state and `dispose()` releases it.

## 5. A warning about other data loaders

`TestDataProvider` also offers loaders such as `quotes_from_fxcm_bars` and
`trades_from_binance_csv`. Those download their CSV from GitHub on first use
(`python/nautilus_trader/testkit/providers.py`, the file-open helper near the top of the module).
They need the network. The in-memory generators, such as `TestDataProvider.usdjpy_quotes(count)`,
and the committed CSVs in this manual's `sample_data/` folder do not.

Continue to [04-sample-data.md](04-sample-data.md).
