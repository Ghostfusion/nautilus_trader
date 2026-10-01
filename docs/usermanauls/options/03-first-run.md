# 03 - Your first option run

This lecture produces the smallest complete option program: a backtest that builds its own
option chain data, subscribes to the chain as a strategy, and prints one snapshot. It needs
no network, no catalog, and no credentials. Every line of output shown below was produced by
running the program exactly as printed.

## 1. Set up the environment

Open a shell. The repository uses `uv` to run Python against the checked-in virtual
environment. On Windows with Git Bash, the commands are:

```bash
export PATH="C:/Users/vince/.cargo/bin;C:/Users/vince/.local/uv012;C:/Users/vince/AppData/Local/Programs/Python/Python312/cpython-3.14-windows-x86_64-none;$PATH"
cd python
```

The first command puts the toolchain and a Python 3.14 interpreter on the path. The second
moves into the `python` directory, where the project file lives. Check that the engine
imports before you write code:

```bash
uv run --no-sync python -c "import nautilus_trader; print(nautilus_trader.__version__)"
```

```text
2.0.0rc6
```

If that prints a version, the environment is ready. If it fails, the virtual environment
has not been built and you should follow the repository's installation instructions before
continuing.

## 2. Write the program

Create a file named `option_chain_backtest.py` in a directory outside this repository, for
example your temporary directory. The manual does not add Python files under `docs/`, so
keep your scratch work elsewhere.

```python
from decimal import Decimal

from nautilus_trader.backtest import BacktestEngine
from nautilus_trader.config import BacktestEngineConfig, StrategyConfig
from nautilus_trader.execution import MakerTakerFeeModel
from nautilus_trader.model import (
    AccountType, AssetClass, Currency, InstrumentId, Money, OmsType, OptionContract,
    OptionGreeks, OptionKind, OptionSeriesId, Price, Quantity, QuoteTick, StrikeRange,
    Symbol, TraderId, Venue,
)
from nautilus_trader.trading import Strategy


class ChainStrategyConfig(StrategyConfig):
    def __init__(self, series_id) -> None:
        super().__init__()
        self.series_id = series_id


class ChainStrategy(Strategy):
    def __init__(self, config: ChainStrategyConfig) -> None:
        super().__init__(config)
        self.snapshots = 0

    def on_start(self) -> None:
        self.subscribe_option_chain(
            self.config.series_id,
            strike_range=StrikeRange.atm_relative(2, 2),
            snapshot_interval_ms=1000,
        )

    def on_option_chain(self, chain) -> None:
        self.snapshots += 1
        if self.snapshots == 1:
            self.log.info(f"chain strikes={chain.strike_count()} atm={chain.atm_strike}")
            for strike in chain.strikes():
                call = chain.get_call(strike)
                if call and call.greeks:
                    self.log.info(f"CALL {strike} delta={call.greeks.delta:.4f}")

    def on_stop(self) -> None:
        self.log.info(f"snapshots={self.snapshots}")


def make_option(cid, kind, strike, venue, expiry_ns):
    return OptionContract(
        instrument_id=InstrumentId.from_str(f"{cid}.{venue}"),
        raw_symbol=Symbol(cid), asset_class=AssetClass.CRYPTOCURRENCY, underlying="BTC",
        option_kind=kind, strike_price=Price.from_str(strike),
        currency=Currency.from_str("USD"), activation_ns=0, expiration_ns=expiry_ns,
        price_precision=2, price_increment=Price.from_str("0.01"),
        multiplier=Quantity.from_int(1), lot_size=Quantity.from_int(1), ts_event=0, ts_init=0,
    )


engine = BacktestEngine(BacktestEngineConfig(trader_id=TraderId.from_str("BACKTESTER-001")))
SIM = Venue("SIM")
USD = Currency.from_str("USD")
engine.add_venue(venue=SIM, oms_type=OmsType.NETTING, account_type=AccountType.MARGIN,
                 base_currency=USD, starting_balances=[Money(1_000_000, USD)],
                 fee_model=MakerTakerFeeModel(maker_rate=Decimal("0"), taker_rate=Decimal("0")))

series = OptionSeriesId.from_expiry("SIM", "BTC", "USD", "2026-12-25")
expiry_ns = series.expiration_ns
TS = 1_790_000_000_000_000_000
contracts = []
for i, strike in enumerate(["64000.00", "65000.00", "66000.00"]):
    for kind, tag in ((OptionKind.CALL, "C"), (OptionKind.PUT, "P")):
        cid = f"BTC-20261225-{int(float(strike))}-{tag}"
        inst = make_option(cid, kind, strike, SIM, expiry_ns)
        contracts.append((inst, i, kind))
        engine.add_instrument(inst)

data = []
for step in range(6):
    ts = TS + step * 1_000_000_000
    spot = 65000.0 + step * 10.0
    for inst, i, kind in contracts:
        mid = 8000.0 - i * 500.0
        q = QuoteTick(instrument_id=inst.id,
                      bid_price=Price.from_str(f"{mid - 10:.2f}"),
                      ask_price=Price.from_str(f"{mid + 10:.2f}"),
                      bid_size=Quantity.from_int(5), ask_size=Quantity.from_int(5),
                      ts_event=ts, ts_init=ts)
        delta = 0.5 - i * 0.15 if kind == OptionKind.CALL else -(0.5 - i * 0.15)
        g = OptionGreeks(instrument_id=inst.id, delta=delta, gamma=0.0002, vega=12.5, theta=-3.2,
                         mark_iv=0.6, bid_iv=0.59, ask_iv=0.61,
                         underlying_price=spot, open_interest=100.0, ts_event=ts, ts_init=ts)
        data.extend([q, g])

engine.add_data(data)
engine.add_strategy(ChainStrategy(ChainStrategyConfig(series_id=series)))
engine.run()
print(engine.generate_account_report(SIM))
print(engine.generate_order_fills_report())
print(engine.generate_positions_report())
engine.reset()
engine.dispose()
```

## 3. Run it

From the `python` directory:

```bash
uv run --no-sync python /absolute/path/to/option_chain_backtest.py
```

The engine logs a lot of startup detail. The lines that matter are the strategy's. This is
the real output, with the startup banner and repeated subscription lines removed and one
line elided where marked:

```text
2026-09-21T14:13:20.000000000Z [INFO] BACKTESTER-001.nautilus_data::option_chains::manager: Bootstrapped option chain for SIM:BTC:USD:2026-12-25T00:00:00Z (6 active instruments)
2026-09-21T14:13:20.000000000Z [INFO] BACKTESTER-001.ChainStrategy-000: Running
2026-09-21T14:13:22.000000000Z [INFO] BACKTESTER-001.ChainStrategy-000: chain strikes=3 atm=65000.00
2026-09-21T14:13:22.000000000Z [INFO] BACKTESTER-001.ChainStrategy-000: CALL 64000.00 delta=0.5000
2026-09-21T14:13:22.000000000Z [INFO] BACKTESTER-001.ChainStrategy-000: CALL 65000.00 delta=0.3500
2026-09-21T14:13:22.000000000Z [INFO] BACKTESTER-001.ChainStrategy-000: CALL 66000.00 delta=0.2000
2026-09-21T14:13:25.000000000Z [INFO] BACKTESTER-001.ChainStrategy-000: snapshots=4
```

The account report, the fills report, and the positions report follow. The timestamps, the
run identifier, and the elapsed time vary between runs; the balances and the empty frames do
not.

```text
                                total locked  ... info base_currency
2026-09-21 14:13:20+00:00  1000000.00   0.00  ...   {}           USD

[1 rows x 10 columns]
Empty DataFrame
Columns: []
Index: []
Empty DataFrame
Columns: []
Index: []
```

## 4. Reading the program line by line

1. The imports bring in one backtest engine, its config, one option fee model, the model
   types the program constructs, and the base `Strategy` class.
2. `ChainStrategyConfig` is the strategy's configuration. It holds the series to subscribe
   to. `super().__init__()` initialises the base config.
3. `on_start` calls `subscribe_option_chain`. This is the whole subscription. The
   `StrikeRange.atm_relative(2, 2)` argument asks for two strikes above and two below the
   at-the-money strike. `snapshot_interval_ms=1000` asks for one aggregated snapshot per
   second.
4. `on_option_chain` receives an `OptionChainSlice`. On the first slice it logs the number
   of active strikes and the at-the-money strike, then loops over `chain.strikes()` and
   reads the call's greeks through `get_call(strike)`.
5. `make_option` builds one `OptionContract`. The fields are the identity, the asset class,
   the underlying, the option kind, the strike, the currency, the activation and expiration
   timestamps, the price and size precision, and the multiplier.
6. The engine is created with a trader identifier. A simulated venue is added with netting
   accounts, margin accounting, a USD balance, and an explicit zero-fee model. The venue
   refuses to start without a fee model, which is why `MakerTakerFeeModel` is passed even at
   zero.
7. `OptionSeriesId.from_expiry("SIM", "BTC", "USD", "2026-12-25")` builds the series
   identifier from venue, underlying, settlement currency, and a date string. Its
   `expiration_ns` is the expiry in integer nanoseconds, which each contract reuses.
8. Six contracts are created, three strikes by two kinds, and added to the engine's cache.
   The cache is what the chain manager uses to resolve the series.
9. Data is built for six one-second steps. Each step adds a `QuoteTick` and an
   `OptionGreeks` for every contract. The greeks carry an `underlying_price`, which is what
   the engine uses to find the at-the-money strike. Without that field the dynamic strike
   range cannot bootstrap.
10. `add_data` hands the list to the engine, `add_strategy` registers the strategy, and
    `run` replays the data. `reset` and `dispose` release engine resources.

## 5. What just happened

The engine created one `OptionChainManager` for the series, resolved the six instruments
from the cache, and subscribed them. When the first greeks event carried an
`underlying_price`, the manager bootstrapped the active strike set and logged the
`Bootstrapped option chain` line with six active instruments. On each one-second timer the
aggregator produced an `OptionChainSlice`, and the strategy counted four of them before the
data ended. The chain reports three distinct strikes because the two states of each strike,
call and put, are grouped under the same strike.

Two beginner traps are visible in the log and worth naming now.

- The engine warns that a `SubscribeOptionGreeks` command is "handler not implemented" on
  the backtest data client. That warning is expected here. In a backtest the venue
  publishes greeks as recorded data, so the backtest data client has no wire-subscription
  handler for greeks. The chain manager still receives every `OptionGreeks` event through
  the data path, which is why the snapshots contain greeks.
- The at-the-money strike moves if `underlying_price` moves. Here it starts at 65000 and
  the snapshot at second two still reports 65000 because the active window is wide enough
  to include it.

You have now run a complete option program. The next lecture opens the data these prices
and greeks represent, so you can check the numbers by hand.
