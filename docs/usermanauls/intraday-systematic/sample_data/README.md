# Sample data

This folder holds the committed data for the intraday and medium-frequency systematic manual.

## Files

### `usdjpy_1min_bars.csv`

- What it is: 167 one-minute USD/JPY mid-price bars for the simulated venue `SIM`, covering
  2019-01-02 from 00:00:00 UTC to 01:46:00 UTC.
- Row count: 167 data rows plus the header.
- Format: ASCII, LF line endings, comma separated, header row, one trailing newline.
- Size in bytes: 10225.
- MD5: `5fcb9e52687930056c25b9fd0c42ff76`.

Columns:

| Column        | Unit                                           | Meaning                                                       |
| ------------- | ---------------------------------------------- | ------------------------------------------------------------- |
| `ts_event_ns` | integer nanoseconds since 1970-01-01T00:00:00Z | The timestamp of the bar, which is the close of its interval  |
| `open`        | yen per USD, 3 decimals                        | The first mid price of the minute                             |
| `high`        | yen per USD, 3 decimals                        | The highest mid price of the minute                           |
| `low`         | yen per USD, 3 decimals                        | The lowest mid price of the minute                            |
| `close`       | yen per USD, 3 decimals                        | The last mid price of the minute                              |
| `volume`      | quote size units                               | The accumulated quote size the engine recorded for the minute |

The first bar is a single quote, so its open, high, low and close are equal and its volume is
1,000,000. Every later bar holds a full minute and has a volume of 60,000,000. Every row satisfies
the engine's bar rules: `high` is at least `open`, `low` and `close`, and `low` is at most `open`
and `close` (`docs/concepts/data/bar.md`).

## Provenance

The data is synthetic. It is derived from the repository's own fixture
`TestDataProvider.usdjpy_quotes(count=10_000)`
(`python/nautilus_trader/testkit/providers.py`), which generates one-second USD/JPY quote ticks in
memory from a deterministic sine wave with a constant spread of 0.010. The generator runs those
ticks through the engine's internal one-minute `MID` bar aggregation and writes the engine's own
bars, rounded to the three decimals the USD/JPY instrument uses. No third-party or licensed market
data is involved, and the file carries the same licence as the repository.

No borrowed fixture from `crates/adapters/tardis/test_data/` is used in this folder.

## Generator

Save this as a file outside the repository, for example `/tmp/gen_bars.py`, and run it with the
repository's Python, passing the output path as the single argument. It writes the exact bytes of
the committed file.

```python
"""Generate sample_data/usdjpy_1min_bars.csv from the in-memory USD/JPY quote fixture."""

import sys
from decimal import Decimal

from nautilus_trader.backtest import BacktestEngine
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

OUT = sys.argv[1]


class CollectorConfig(StrategyConfig):
    def __init__(self, *, bar_type, **_kwargs):
        super().__init__()
        self.bar_type = bar_type


class Collector(Strategy):
    def __init__(self, config: CollectorConfig) -> None:
        super().__init__(config)
        self.bars: list[Bar] = []

    def on_start(self) -> None:
        self.subscribe_bars(self.config.bar_type)

    def on_bar(self, bar: Bar) -> None:
        self.bars.append(bar)


engine = BacktestEngine(BacktestEngineConfig(trader_id=TraderId.from_str("BACKTESTER-001")))
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
engine.add_instrument(TestInstrumentProvider.usdjpy_sim())
engine.add_data(TestDataProvider.usdjpy_quotes(count=10_000))
collector = Collector(CollectorConfig(bar_type=BarType.from_str("USD/JPY.SIM-1-MINUTE-MID-INTERNAL")))
engine.add_strategy(collector)
engine.run()

with open(OUT, "w", newline="\n") as f:
    f.write("ts_event_ns,open,high,low,close,volume\n")
    for bar in collector.bars:
        f.write(
            f"{bar.ts_event},{float(bar.open):.3f},{float(bar.high):.3f},"
            f"{float(bar.low):.3f},{float(bar.close):.3f},{float(bar.volume):.0f}\n",
        )

print("rows:", len(collector.bars))
engine.reset()
engine.dispose()
```

Command and observed output:

```bash
export PATH="C:/Users/vince/.cargo/bin;C:/Users/vince/.local/uv012;C:/Users/vince/AppData/Local/Programs/Python/Python312/cpython-3.14-windows-x86_64-none;$PATH"
cd python
uv run --no-sync python /tmp/gen_bars.py /tmp/usdjpy_1min_bars.csv
```

```text
rows: 167
```

The generated file is byte-for-byte identical to the committed one, so re-running the generator
changes nothing.
