# EODHD

EODHD provides end-of-day historical market data for global equities, ETFs, and funds through a
REST API.

NautilusTrader integrates with the EODHD REST API to build a backtest universe and load historical
bars. The capabilities of this adapter include:

- An `EodhdDataLoader` that converts EODHD end-of-day rows into Nautilus `Bar` objects, and EODHD
  tickers into `Equity` instrument definitions.
- An `EodhdInstrumentProvider` that implements the Rust `InstrumentProvider` trait for programmatic
  universe loading.

:::info
This is a **data-only** adapter. It ships no live data client and no execution client: EODHD is a
historical data vendor, not a trading venue.
:::

:::info
An `EODHD_API_KEY` is required for instrument metadata calls. See also
[environment variables](#environment-variables).
:::

## Overview

The adapter is implemented in Rust with optional Python bindings. Its components are compiled into
NautilusTrader, so it does not require a separate EODHD client library installation. Consult the
[EODHD documentation](https://eodhd.com/financial-apis/) for the upstream API.

## Supported endpoints

The adapter covers the endpoints needed to run a bar-driven backtest. Every other EODHD endpoint is
out of scope.

| EODHD endpoint                     | Nautilus data type |
| :--------------------------------- | :----------------- |
| `/eod/{ticker}`                    | `Bar`              |
| `/exchange-symbol-list/{exchange}` | `Equity`           |

**Notes:**

- `/eod` accepts a `period` of `d` (daily), `w` (weekly), or `m` (monthly), and is queried with an
  inclusive `from` and `to` date range.
- The adapter loads the raw `close` price. EODHD also returns `adjusted_close`, which the adapter
  does not use: NautilusTrader applies corporate actions through instrument and adjustment models
  rather than silently substituting one price series for another.
- Share volume is loaded with a size precision of 0.
- Bar timestamps are the UTC midnight of the row date, used for both `ts_event` and `ts_init`.
- News, fundamentals, sentiment, macro, and intraday endpoints are not supported. The platform has
  no data type for them.

## Symbology

An EODHD ticker carries its exchange suffix, for example `AAPL.US` or `VOD.LSE`. The suffix becomes
the Nautilus venue, so an EODHD ticker round-trips through an `InstrumentId` with no lookup table:

| EODHD ticker | Nautilus `InstrumentId` | Nautilus venue |
| :----------- | :---------------------- | :------------- |
| `AAPL.US`    | `AAPL.US`               | `US`           |
| `VOD.LSE`    | `VOD.LSE`               | `LSE`          |

The `raw_symbol` retains the full EODHD ticker, and the venue carries the exchange code exactly as
EODHD publishes it. The adapter does not attempt to map EODHD exchange codes onto MIC venue codes.

:::warning
Because the venue is the EODHD exchange code, an instrument loaded here does not share an
`InstrumentId` with the same listing loaded from an exchange adapter. Keep one source per
instrument when writing to a catalog.
:::

## Environment variables

- `EODHD_API_KEY`: The API token used to authenticate requests.

The token may also be passed directly to the loader constructor. A token is always required by the
constructor; the `/eod` endpoint additionally accepts EODHD's public `demo` token, which is limited
to a small documented symbol set.

## Loading EODHD historical data

### Python

```python
from nautilus_trader.adapters.eodhd import EodhdDataLoader
from nautilus_trader.model import InstrumentId
from nautilus_trader.persistence.catalog import ParquetDataCatalog

loader = EodhdDataLoader()  # Token from EODHD_API_KEY

instrument = loader.instrument("AAPL.US")
bars = await loader.bars(
    instrument_id=InstrumentId.from_str("AAPL.US"),
    start="2024-01-02",
    end="2024-01-10",
    period="d",
)

catalog = ParquetDataCatalog("./catalog")
catalog.write_instruments([instrument])
catalog.write_bars(bars)
```

`bars` and `instruments` are coroutines: the first performs a network request, the second performs
one request per exchange. `instrument` is synchronous because the ticker carries everything needed.

### Rust

```rust
use nautilus_eodhd::{EodhdDataLoader, providers::EodhdInstrumentProvider};
use nautilus_model::identifiers::InstrumentId;

let loader = EodhdDataLoader::new(None, None, None, None, None, None)?;

let instrument = loader.instrument("AAPL.US")?;
let bars = loader
    .bars(InstrumentId::from("AAPL.US"), "2024-01-02", "2024-01-10", "d")
    .await?;

let client = loader.client().clone();
let mut provider = EodhdInstrumentProvider::new(client, Currency::USD(), 2);
provider.load_all(None).await?;  // Fetches and caches the exchange universe
```

## Instrument metadata

`EodhdDataLoader::instruments` and `EodhdInstrumentProvider` request
`/exchange-symbol-list/{exchange}` and keep the rows whose type is `Common Stock` or `ETF`. Rows
with no type are kept, because the endpoint omits it for some exchanges.

An `exchange` filter on `InstrumentProvider::load_all` selects the exchange; without one the
provider defaults to `US`.

## Limitations and considerations

- The `/exchange-symbol-list` endpoint rejects the public `demo` token with an HTTP 403, so
  universe loading requires a paid token. Bar loading works with the demo token for its documented
  symbol set.
- EODHD reports most failures as an HTTP 200 with a JSON error body rather than an error status.
  The client checks for that envelope before deserializing, so a rejected request surfaces as an
  error instead of an empty result.
- Requests are rate limited to 10 per second, which is well below EODHD's documented plan limits.
- A bar whose OHLC values violate an ordering relationship is reported as an error rather than
  silently repaired.
- EODHD is a third-party data vendor with its own licence terms. Ensure your use of the data
  complies with them.

## Contributing

:::info
For additional features or to contribute to the EODHD adapter, please see the
[contributing guide](https://github.com/nautechsystems/nautilus_trader/blob/develop/CONTRIBUTING.md).
:::
