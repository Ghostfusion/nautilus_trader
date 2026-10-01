# EODHD

EODHD provides end-of-day and intraday historical market data for global equities, ETFs, and
funds, together with delayed quotes, dividends and splits, news, and macro series, through a
REST API.

NautilusTrader integrates with the EODHD REST API. The capabilities of this adapter include:

- An `EodhdDataClient` that streams bars and delayed quotes into a running node by polling.
- An `EodhdDataLoader` that converts EODHD rows into Nautilus `Bar` objects, and EODHD tickers
  into `Equity` instrument definitions.
- An `EodhdInstrumentProvider` that implements the Rust `InstrumentProvider` trait for
  programmatic universe loading.

:::info
EODHD is a historical data vendor, not a trading venue, so the adapter ships no execution client.
:::

:::info
An `EODHD_API_KEY` is required. See also [environment variables](#environment-variables).
:::

## Overview

The adapter is implemented in Rust with optional Python bindings. Its components are compiled into
NautilusTrader, so it does not require a separate EODHD client library installation. Consult the
[EODHD documentation](https://eodhd.com/financial-apis/) for the upstream API.

## Supported endpoints

| EODHD endpoint                     | Nautilus data type | Notes                                            |
| :--------------------------------- | :----------------- | :----------------------------------------------- |
| `/eod/{ticker}`                    | `Bar`              | Daily, weekly, and monthly periods.              |
| `/intraday/{ticker}`               | `Bar`              | One minute, five minute, and one hour intervals. |
| `/us-quote-delayed`                | `QuoteTick`        | Delayed best bid and offer for US symbols.       |
| `/exchange-symbol-list/{exchange}` | `Equity`           | Instrument discovery for one exchange.           |

**Notes:**

- The adapter loads the raw `close` price. EODHD also returns `adjusted_close`, which the adapter
  does not use: NautilusTrader applies corporate actions through instrument and adjustment models
  rather than silently substituting one price series for another.
- Share volume and quote sizes are loaded with a size precision of 0.
- Bar timestamps come from the row date for `/eod`, and from the row `timestamp` for `/intraday`,
  which is the authoritative epoch second. The intraday `datetime` field is rendered in the
  exchange offset and is not used.
- News, fundamentals, sentiment, macro, and commodities carry no engine semantics in
  NautilusTrader. See the [capability map](#capability-map).

## Bar intervals

| EODHD endpoint | Interval | Nautilus bar type        |
| :------------- | :------- | :----------------------- |
| `/eod`         | `d`      | `1-DAY-LAST-EXTERNAL`    |
| `/eod`         | `w`      | `1-WEEK-LAST-EXTERNAL`   |
| `/eod`         | `m`      | `1-MONTH-LAST-EXTERNAL`  |
| `/intraday`    | `1m`     | `1-MINUTE-LAST-EXTERNAL` |
| `/intraday`    | `5m`     | `5-MINUTE-LAST-EXTERNAL` |
| `/intraday`    | `1h`     | `1-HOUR-LAST-EXTERNAL`   |

EODHD aggregates these bars and publishes last prices, so a subscription for any other
aggregation, step, price type, or aggregation source is rejected with an explanatory error rather
than served a different series.

## Symbology

An EODHD ticker carries its exchange suffix, for example `AAPL.US` or `VOD.LSE`. The suffix becomes
the Nautilus venue, so an EODHD ticker round-trips through an `InstrumentId` with no lookup table:

| EODHD ticker | Nautilus `InstrumentId` | Nautilus venue |
| :----------- | :---------------------- | :------------- |
| `AAPL.US`    | `AAPL.US`               | `US`           |
| `VOD.LSE`    | `VOD.LSE`               | `LSE`          |

The `raw_symbol` retains the full EODHD ticker, and the venue carries the exchange code exactly as
EODHD publishes it. The adapter does not map EODHD exchange codes onto MIC venue codes, and the
data client is therefore multi-venue: `DataClient::venue` returns `None`, and each instrument
carries its own exchange as its venue.

:::warning
Because the venue is the EODHD exchange code, an instrument loaded here does not share an
`InstrumentId` with the same listing loaded from an exchange adapter. Keep one source per
instrument when writing to a catalog.
:::

## Environment variables

- `EODHD_API_KEY`: The API token used to authenticate requests.

The token may also be passed directly to a client or loader constructor. A token is always required
by the constructor. The `/eod` endpoint additionally accepts EODHD's public `demo` token, which is
limited to a small documented symbol set.

## Data client

`EodhdDataClient` streams bars and delayed quotes into a running node.

EODHD publishes no HTTP push channel for these endpoints, so the client streams by polling. Every
subscription owns a task that requests a bounded window, emits what is new or revised, and then
waits for its next tick. The request window is derived from the newest bar already emitted, so a
long-running subscription does not re-transfer its history on every poll.

### Configuration

| Field                | Type   | Default | Description                                                       |
| :------------------- | :----- | :------ | :---------------------------------------------------------------- |
| `api_key`            | `str`  | `None`  | The EODHD API token; falls back to `EODHD_API_KEY`.               |
| `http_base_url`      | `str`  | `None`  | Overrides the REST base URL.                                      |
| `proxy_url`          | `str`  | `None`  | Optional proxy URL for HTTP requests.                             |
| `exchange`           | `str`  | `"US"`  | The EODHD exchange whose instruments are loaded on connect.       |
| `poll_interval_secs` | `int`  | `60`    | How often each subscription polls, in seconds.                    |
| `backfill_days`      | `int`  | `5`     | How many days of history a bar subscription emits when it starts. |
| `price_precision`    | `int`  | `2`     | Price precision for instruments, bars, and quotes.                |
| `currency`           | `str`  | `None`  | Instrument currency code; defaults to `USD`.                      |
| `timeout_secs`       | `int`  | `None`  | HTTP request timeout; defaults to 30 seconds.                     |
| `load_instruments`   | `bool` | `True`  | Whether to load the `exchange` instruments on connect.            |

### Usage

```python
from nautilus_trader.adapters.eodhd import EodhdDataClientConfig
from nautilus_trader.adapters.eodhd import EodhdDataClientFactory
from nautilus_trader.common import Environment
from nautilus_trader.live import LiveNode
from nautilus_trader.model import BarType
from nautilus_trader.model import TraderId

trader_id = TraderId.from_str("TESTER-001")

node = (
    LiveNode.builder("EODHD-DATA-001", trader_id, Environment.LIVE)
    .add_data_client(
        None,
        EodhdDataClientFactory(),
        EodhdDataClientConfig(api_key="...", exchange="US", poll_interval_secs=60),
    )
    .build()
)
```

A strategy then subscribes as it would to any other data client:

```python
bar_type = BarType.from_str("AAPL.US-1-MINUTE-LAST-EXTERNAL")

self.subscribe_bars(bar_type)
```

### Streaming semantics

A forming bar is republished by the vendor with changing values. The client emits a bar when its
timestamp advances, and re-emits the newest bar when its values change, so a strategy observes the
same bar revision sequence a venue feed would produce. An unchanged repeat is suppressed.

Delayed quotes are emitted only when the quoted prices, sizes, or times change.

`request_bars` fetches a window without starting a poll, and is the right call for a historical
backfill. `subscribe_bars` emits the configured history on its first tick and then polls, which
warms a running node without a separate request.

## Capability map

The endpoints below are part of EODHD's entitlement tiers. The table states what the adapter does
with each, so a subscription decision can be made against the mapping rather than against the
marketing list.

| Endpoint                                     | NautilusTrader outcome                                                                                                                                           |
| :------------------------------------------- | :--------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `/eod/{SYMBOL}`                              | Mapped: `Bar`.                                                                                                                                                   |
| `/intraday/{SYMBOL}`                         | Mapped: `Bar`. Requires the intraday entitlement.                                                                                                                |
| `/us-quote-delayed`                          | Mapped: `QuoteTick`.                                                                                                                                             |
| `/exchange-symbol-list/{EXCHANGE}`           | Mapped: `Equity`.                                                                                                                                                |
| `/eod-bulk-last-day/{EXCHANGE}`              | Not used. One request returns the latest bar per symbol, which is the right shape for a large daily universe; per-symbol polling is used instead.                |
| `/div/{SYMBOL}`, `/splits/{SYMBOL}`          | Not mapped. Corporate actions reach the engine through instrument and adjustment models, not through the data client command surface.                            |
| `/exchanges-list/`                           | Not used. Exchange metadata is descriptive here.                                                                                                                 |
| `/search/{QUERY}`                            | Not used. Symbol discovery goes through the exchange list.                                                                                                       |
| `/real-time/{SYMBOL}`                        | Not mapped. The snapshot carries a last price and a cumulative daily volume, not a per-trade size or an aggressor, so a `TradeTick` would be fabricated from it. |
| `/news`, `/sentiments`, `/news-word-weights` | Not mapped. NautilusTrader has no news or sentiment data type.                                                                                                   |
| `/commodities/historical/{CODE}`             | Not mapped. No data client command reaches a commodity series.                                                                                                   |
| `/ust/...` rates                             | Not mapped. Yield series are not tradeable instruments here.                                                                                                     |

Real-time WebSocket channels (`wss://ws.eodhistoricaldata.com/ws/us` for trades,
`/ws/us-quote` for quotes, `/ws/us-status` and `/ws/eu-status` for status) are the correct path for
undelayed tick data. They require a streaming entitlement on the top tier plans and are **not
implemented** here: without an entitlement they cannot be verified, and the polling client above
covers bar and delayed quote streaming on the lower tiers.

## Loading EODHD historical data

### Python

```python
from nautilus_trader.adapters.eodhd import EodhdDataLoader
from nautilus_trader.model import InstrumentId
from nautilus_trader.persistence.catalog import ParquetDataCatalog

loader = EodhdDataLoader()  # Token from EODHD_API_KEY

instrument = loader.instrument("AAPL.US")
daily = await loader.bars(
    instrument_id=InstrumentId.from_str("AAPL.US"),
    start="2024-01-02",
    end="2024-01-10",
    period="d",
)
intraday = await loader.bars(
    instrument_id=InstrumentId.from_str("AAPL.US"),
    start="2024-01-02",
    end="2024-01-03",
    period="5m",
)

catalog = ParquetDataCatalog("./catalog")
catalog.write_instruments([instrument])
catalog.write_bars(daily)
```

`bars` and `instruments` are coroutines: each performs a network request. `instrument` is
synchronous because the ticker carries everything needed.

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

`EodhdDataLoader::instruments`, `EodhdDataClient`, and `EodhdInstrumentProvider` request
`/exchange-symbol-list/{exchange}` and keep the rows whose type is `Common Stock` or `ETF`. Rows
with no type are kept, because the endpoint omits it for some exchanges.

An `exchange` filter on `InstrumentProvider::load_all` selects the exchange; without one the
provider defaults to `US`.

## Limitations and considerations

- Polling is not a push feed. A forming bar is observed at the poll interval, and the newest bar
  may be revised after it has been emitted, which is normal for a vendor that publishes a partially
  complete period.
- The `/exchange-symbol-list` and `/us-quote-delayed` endpoints require an entitlement beyond the
  public demo token, which returns HTTP 403 for them. Bar loading works with the demo token for its
  documented symbol set.
- EODHD reports most failures as an HTTP 200 with a JSON error body rather than an error status.
  The client checks for that envelope before deserializing, so a rejected request surfaces as an
  error instead of an empty result.
- Requests are rate limited to 10 per second, which is well below EODHD's documented plan limits.
- The adapter does not retry a failed poll. A transient failure is logged and the next tick retries.
- EODHD is a third-party data vendor with its own licence terms. Ensure your use of the data
  complies with them.

## Contributing

:::info
For additional features or to contribute to the EODHD adapter, please see the
[contributing guide](https://github.com/nautechsystems/nautilus_trader/blob/develop/CONTRIBUTING.md).
:::
