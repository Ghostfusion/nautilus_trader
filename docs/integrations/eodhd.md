# EODHD

EODHD provides end-of-day and intraday historical market data for global equities, ETFs, and
funds, together with delayed quotes, dividends and splits, news, and macro series, through a
REST API.

NautilusTrader integrates with the EODHD REST API. The capabilities of this adapter include:

- An `EodhdDataClient` that streams bars and delayed quotes into a running node by polling, and
  publishes corporate actions for the bars it serves.
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

| EODHD endpoint                     | Nautilus data type | Notes                                                       |
| :--------------------------------- | :----------------- | :---------------------------------------------------------- |
| `/eod/{ticker}`                    | `Bar`              | Daily, weekly, and monthly periods.                         |
| `/intraday/{ticker}`               | `Bar`              | One minute, five minute, and one hour intervals.            |
| `/us-quote-delayed`                | `QuoteTick`        | Delayed best bid and offer for US symbols.                  |
| `/exchange-symbol-list/{exchange}` | `Equity`           | Instrument discovery for one exchange.                      |
| `/eod-bulk-last-day/{exchange}`    | `Bar`              | The latest day for every symbol, when bulk mode is enabled. |
| `/div/{ticker}`                    | `CorporateAction`  | Dividends, when corporate action loading is enabled.        |
| `/splits/{ticker}`                 | `CorporateAction`  | Splits, when corporate action loading is enabled.           |

**Notes:**

- The adapter loads the raw `close` price. EODHD also returns `adjusted_close`, which the adapter
  does not use: NautilusTrader applies corporate actions through instrument and adjustment models
  rather than silently substituting one price series for another.
- For the same reason, a dividend carries the as-reported `unadjustedValue` rather than the split
  adjusted `value`. The action describes the same price series the bars describe.
- Both corporate action endpoints return the full history, from 1987 for `AAPL.US`. The adapter
  bounds every request by the window it was asked for: a 2024 dividend window returns four rows
  where the unbounded request returns ninety-two.
- Share volume and quote sizes are loaded with a size precision of 0.
- Bar timestamps come from the row date for `/eod`, and from the row `timestamp` for `/intraday`,
  which is the authoritative epoch second. The intraday `datetime` field is rendered in the
  exchange offset and is not used.
- News, fundamentals, sentiment, macro, and commodities carry no engine semantics in
  NautilusTrader. See the [capability map](#capability-map).
- `/intraday` is served only on plans that carry the intraday entitlement, and `/eod-bulk-last-day`
  only on plans that carry the bulk entitlement. Both are absent from some paid end-of-day tiers,
  where the endpoint returns HTTP 403.

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

That suffix is the exchange code the symbol list was requested with, which for the 70 entries of
`/exchanges-list` is also the code EODHD addresses data requests with. It is not the listing venue
a symbol row reports: a `US` list returns rows whose `Exchange` field names `NYSE`, `NASDAQ`,
`PINK`, `NMFQS` and a dozen other venues, and EODHD rejects every one of them as a suffix. Request
`AAPL.US`, not `AAPL.NASDAQ`.

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

| Field                    | Type        | Default | Description                                                       |
| :----------------------- | :---------- | :------ | :---------------------------------------------------------------- |
| `api_key`                | `str`       | `None`  | The EODHD API token; falls back to `EODHD_API_KEY`.               |
| `http_base_url`          | `str`       | `None`  | Overrides the REST base URL.                                      |
| `proxy_url`              | `str`       | `None`  | Optional proxy URL for HTTP requests.                             |
| `exchange`               | `str`       | `"US"`  | The EODHD exchange whose instruments are loaded on connect.       |
| `bulk_exchanges`         | `list[str]` | `[]`    | Exchanges whose daily bars poll through `/eod-bulk-last-day`.     |
| `poll_interval_secs`     | `int`       | `60`    | How often each subscription polls, in seconds.                    |
| `backfill_days`          | `int`       | `5`     | How many days of history a bar subscription emits when it starts. |
| `price_precision`        | `int`       | `2`     | Price precision for instruments, bars, and quotes.                |
| `currency`               | `str`       | `None`  | Instrument currency code; defaults to `USD`.                      |
| `timeout_secs`           | `int`       | `None`  | HTTP request timeout; defaults to 30 seconds.                     |
| `load_instruments`       | `bool`      | `True`  | Whether to load the `exchange` instruments on connect.            |
| `load_corporate_actions` | `bool`      | `False` | Whether to publish the corporate actions in a bar window.         |

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

### Corporate actions

A corporate action is auxiliary data rather than a market event: it reports a change to the
economic meaning or identity of an instrument. No data client command carries one. The engine
publishes a `CorporateAction` on the instrument's corporate action topic, and a strategy receives
it through `subscribe_corporate_actions` and `on_corporate_action`.

Set `load_corporate_actions` and the client publishes the actions that fall in the window of bars
it was asked for, which puts an action next to the bars it adjusts:

```python
class MyStrategy(Strategy):
    def on_start(self):
        # Subscribe before the bars, so the topic is open before the first poll publishes.
        self.subscribe_corporate_actions(InstrumentId.from_str("AAPL.US"))
        self.subscribe_bars(BarType.from_str("AAPL.US-1-DAY-LAST-EXTERNAL"))

    def on_corporate_action(self, action):
        self.log.info(f"{action.instrument_id} {action.action} {action.value}")
```

- A dividend arrives with the as-reported cash amount per share, and a split with the new shares
  per old share, both as an exact decimal rather than a rounded float.
- `effective_ns` is the ex-date for a dividend and the split date for a split, while `ts_event` is
  when the engine observed the record. A `CorporateAction` keeps the two separate because an
  announced action takes effect at a date that is not when it is observed.
- The window is the subscription's own window: `backfill_days` at subscribe time, or the requested
  range for `request_bars`. A short window therefore reports only a recent action, and a decade-long
  window reports the decade. The actions are published once per request rather than on every poll.
- Each bar request and each bar subscription costs two further requests while this is enabled,
  which is why it is off by default.

### Bulk last-day mode

Polling one request per symbol does not scale to a daily universe. `bulk_exchanges` names the
exchanges whose daily bars are instead served from `/eod-bulk-last-day`, where a single request
returns the latest bar for every symbol the exchange lists. A daily subscription on one of those
exchanges is served by that shared request, so a universe of two hundred symbols costs one request
per poll rather than two hundred.

```python
EodhdDataClientConfig(
    api_key="...",
    bulk_exchanges=["US"],
    poll_interval_secs=60,
)
```

Subscribing still seeds `backfill_days` of history for the instrument itself, from `/eod`, so a
strategy sees the same history in either mode. Only the ongoing polls are shared.

- The endpoint has no filter for a subset of symbols, so the whole exchange is transferred and the
  rows a subscription asked for are selected from it. The `US` list carries about 51,000 rows at
  roughly 5 MB compressed per poll; enable bulk for an exchange when the universe is large enough
  to pay for that, and set `poll_interval_secs` with the transfer in mind.
- Bars for other intervals, and for exchanges not listed, are still polled per symbol.
- The endpoint also accepts a `date` query for a historical day. The adapter does not use it: for a
  single instrument, history is cheaper from `/eod`.
- A cancelled subscription stops contributing, and an exchange with no subscriptions left costs no
  request at all.

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
| `/eod-bulk-last-day/{EXCHANGE}`              | Mapped: `Bar`. Serves every daily subscription on that exchange from one request per poll, when the exchange is named in `bulk_exchanges`.                       |
| `/div/{SYMBOL}`, `/splits/{SYMBOL}`          | Mapped: `CorporateAction`, published on the instrument's corporate action topic when `load_corporate_actions` is set.                                            |
| `/exchanges-list/`                           | Not used. Exchange metadata is descriptive here.                                                                                                                 |
| `/search/{QUERY}`                            | Not used. Symbol discovery goes through the exchange list.                                                                                                       |
| `/real-time/{SYMBOL}`                        | Not mapped. The snapshot carries a last price and a cumulative daily volume, not a per-trade size or an aggressor, so a `TradeTick` would be fabricated from it. |
| `/news`, `/sentiments`, `/news-word-weights` | Not mapped. NautilusTrader has no news or sentiment data type.                                                                                                   |
| `/commodities/historical/{CODE}`             | Not mapped. No data client command reaches a commodity series.                                                                                                   |
| `/ust/...` rates                             | Not mapped. Yield series are not tradeable instruments here.                                                                                                     |

## WebSocket channels

Real-time WebSocket channels are the correct path for undelayed tick data:

| Channel                                       | Data                    |
| :-------------------------------------------- | :---------------------- |
| `wss://ws.eodhistoricaldata.com/ws/us`        | US trades.              |
| `wss://ws.eodhistoricaldata.com/ws/us-quote`  | US quotes.              |
| `wss://ws.eodhistoricaldata.com/ws/us-status` | US market status.       |
| `wss://ws.eodhistoricaldata.com/ws/eu-status` | European market status. |

The token travels as the `api_token` query parameter, as it does on the REST API, and a
subscription is requested with `{"action":"subscribe","symbols":"AAPL,MSFT"}`, where `symbols` is a
comma separated string rather than an array.

A WebSocket client is **not implemented**. It requires a streaming entitlement, and the channels
cannot be verified without one: against a token from a plan without streaming, a connection
completes the WebSocket handshake and then receives `{"status":403,"message":"Server error"}` and
closes, before any subscribe message is sent. The polling client above covers bars and delayed
quotes on the tiers that do not carry streaming.

This section records what an implementation starts from. The payload field names published
upstream are deliberately not reproduced here, because nothing in this adapter has verified them
against a live feed, and a price parsed from a guessed field name is worse than no price at all.
See the [provider's documentation](https://eodhd.com/financial-apis/new-real-time-data-api-websockets)
for the frames.

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

actions = await loader.corporate_actions(
    instrument_id=InstrumentId.from_str("AAPL.US"),
    start="2020-01-01",
    end="2024-12-31",
)

catalog = ParquetDataCatalog("./catalog")
catalog.write_instruments([instrument])
catalog.write_bars(daily)
```

`corporate_actions` returns the dividends and splits for the range together, ordered by the time
they take effect, with the same exact values the data client publishes.

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
let actions = loader
    .corporate_actions(InstrumentId::from("AAPL.US"), "2020-01-01", "2024-12-31")
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
provider defaults to `US`. The filter takes an `/exchanges-list` code, which is also the ticker
suffix, rather than a listing venue such as `NASDAQ`.

## Limitations and considerations

- Polling is not a push feed. A forming bar is observed at the poll interval, and the newest bar
  may be revised after it has been emitted, which is normal for a vendor that publishes a partially
  complete period.
- Endpoint availability depends on the plan, and the tiers are not nested. The public `demo` token
  serves `/eod`, `/intraday`, `/div`, `/splits` and `/fundamentals`, while returning HTTP 403 for
  `/exchange-symbol-list`, `/us-quote-delayed`, `/eod-bulk-last-day`, `/exchanges-list`, `/search`
  and the `/ust` series. A paid end-of-day plan can be the inverse: measured against one, `/eod`,
  `/eod-bulk-last-day`, `/exchange-symbol-list`, `/exchanges-list`, `/search`, `/us-quote-delayed`,
  `/real-time`, `/div`, `/splits`, `/news` and `/sentiments` returned data, while `/intraday`,
  `/technical`, `/fundamentals` and `/screener` returned HTTP 403 and the WebSocket channels closed
  with `{"status":403}`. Confirm an endpoint against your own token rather than assuming the tier.
  See [WebSocket channels](#websocket-channels) for the streaming entitlement in detail.
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
