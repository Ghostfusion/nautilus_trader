# Moomoo

Moomoo provides market data for United States and Hong Kong listed securities through OpenD, a
gateway that runs on the trader's own machine and holds the market data login. NautilusTrader
integrates with that gateway by speaking its own frame protocol, so the data reaches the engine
without the vendor's Python client in the path.

The capabilities of this adapter include:

- A `MoomooDataClient` that serves instruments, bars, trades, quotes, and order book deltas from a
  running gateway, and publishes corporate actions for the securities it is asked about.
- Instrument loading for both markets it serves, in one request per market.
- A `MoomooDataClientConfig` and a `MoomooDataClientFactory` for registering the client on a node.

:::info
The gateway is a prerequisite, not an optional extra. The adapter opens a socket to it, so a node
configured for moomoo data has to have OpenD running and logged in on the same machine or on a host
it can reach. See [Prerequisites](#prerequisites).
:::

:::info
There is no API key. The gateway holds the login, and the adapter's only credential-like concern is
not leaking the address of a remote gateway into logs.
:::

:::warning
Moomoo is a market data source here, not a trading venue: the adapter ships no execution client. See
the [capability map](#capability-map) for what else it deliberately does not map.
:::

## Overview

The adapter is implemented in Rust with optional Python bindings, and its components are compiled
into NautilusTrader, so no separate client library is installed. The gateway's own schema is
vendored into the crate and the Rust types are generated from it, which is what lets the adapter
speak the protocol without depending on the vendor's Python package or on the Python version that
package requires.

## Prerequisites

- **OpenD running and logged in.** The gateway is the market data login; the adapter cannot
  substitute for it.
- **Its address.** The default is `127.0.0.1:11111`, and `MOOMOO_HOST` and `MOOMOO_PORT` override
  it. A remote gateway is reached over a tunnel, and the address is the only thing about the
  connection that has to be configured.
- **An entitlement for the market.** The gateway reports what the login may do when the client
  connects, and the record is logged. The observed record on the development login was level two
  for United States equities, level one for Hong Kong equities, and one hundred subscriptions and
  one hundred historical securities available.

## Requests and pushes

| Gateway request                | Python name                          | NautilusTrader outcome                                       |
| :----------------------------- | :----------------------------------- | :----------------------------------------------------------- |
| Static information, 3202       | `get_stock_basicinfo`                | `Equity`.                                                    |
| Security snapshot, 3203        | `get_market_snapshot`                | The price precision. The snapshot carries no bid or ask.     |
| Historical K-line, 3103        | `request_history_kline`              | `Bar`, paginated and deduplicated.                           |
| Historical allowance, 3104     | `get_history_kl_quota`               | Accounting only.                                             |
| Subscribe, 3001                | `subscribe`                          | The venue's subscription, one per security and data type.    |
| Subscription information, 3003 | `get_subscription`                   | The subscription allowance.                                  |
| K-line push, 3007              | `on_recv_rsp`                        | `Bar`, for the interval subscribed.                          |
| Ticker push, 3011              | `on_recv_rsp`                        | `TradeTick`.                                                 |
| Order book push, 3013          | `on_recv_rsp`                        | `QuoteTick` and `OrderBookDeltas`.                           |
| Order book request, 3012       | `get_orderbook`                      | `OrderBookDeltas`. Served only while the book is subscribed. |
| Dividends, 3234                | `get_corporate_actions_dividends`    | `CorporateAction`.                                           |
| Stock splits, 3236             | `get_corporate_actions_stock_splits` | `CorporateAction`.                                           |
| Entitlement, 1005              | `get_user_info`                      | The capability record written to the log on connect.         |

## Symbology

The gateway addresses a security as a market and a code, such as `US.AAPL`. The market becomes the
NautilusTrader venue and the code becomes the symbol:

| Gateway code | Nautilus `InstrumentId` | Nautilus venue | Nautilus `raw_symbol` |
| :----------- | :---------------------- | :------------- | :-------------------- |
| `US.AAPL`    | `AAPL.US`               | `US`           | `US.AAPL`             |
| `HK.00700`   | `00700.HK`              | `HK`           | `HK.00700`            |

The venue is the market and not the listing exchange. The gateway also reports an exchange type,
such as `NASDAQ`, and folding that into the venue would make this adapter's identifiers disagree
with every other provider's for the same security.

The client is therefore multi-venue: `DataClient::venue` returns `None`, and each instrument carries
its own market as its venue. The client identifier is `MOOMOO`, which is the provider rather than a
venue.

:::tip
Because the venue is the market, a `AAPL.US` instrument loaded here and one loaded from another
provider of United States equities are the same identifier, so a catalog or a failover arrangement
can treat them as one instrument.
:::

## Bars

The gateway names its intervals itself, and not every step is offered:

| Interval    | Nautilus bar type         |
| :---------- | :------------------------ |
| One minute  | `1-MINUTE-LAST-EXTERNAL`  |
| Three       | `3-MINUTE-LAST-EXTERNAL`  |
| Five        | `5-MINUTE-LAST-EXTERNAL`  |
| Ten         | `10-MINUTE-LAST-EXTERNAL` |
| Fifteen     | `15-MINUTE-LAST-EXTERNAL` |
| Thirty      | `30-MINUTE-LAST-EXTERNAL` |
| Hourly      | `1-HOUR-LAST-EXTERNAL`    |
| Two hourly  | `2-HOUR-LAST-EXTERNAL`    |
| Four hourly | `4-HOUR-LAST-EXTERNAL`    |
| Daily       | `1-DAY-LAST-EXTERNAL`     |
| Weekly      | `1-WEEK-LAST-EXTERNAL`    |
| Monthly     | `1-MONTH-LAST-EXTERNAL`   |
| Quarterly   | `3-MONTH-LAST-EXTERNAL`   |
| Yearly      | `1-YEAR-LAST-EXTERNAL`    |

An interval the gateway does not offer is refused with an explanatory error rather than rounded to
one it does, because a strategy asking for a bar it will never receive has a defect that rounding
would hide.

**A request and a subscription are different paths.** `request_bars` reads a range from the
gateway's history endpoint: it paginates until the gateway stops returning a continuation key, and
returns one series sorted by event time with a repeated timestamp keeping the later row, because a
consumer that saw both a bar and its correction could not tell which was which. `subscribe_bars`
takes the gateway's K-line push for one interval, which is a subscription against the allowance and
arrives with the gateway's cached history for that interval first.

**Adjustment and session are stated, not inherited.** The gateway's own default for a K-line push is
a forward adjustment, so leaving the field out would serve a different series from the one the
adapter's corporate actions describe. Both are configuration fields, and both are sent with a
request and with a subscription.

**Timestamps.** A daily or coarser bar carries a date, which is placed at UTC midnight so that it
agrees with the daily bar another provider serves for the same instrument. A bar finer than a day
carries the exchange's own wall clock reading and is resolved through that market's IANA time zone,
so a bar is placed correctly either side of a daylight saving change rather than an hour out.

**Blank rows are skipped.** The gateway states an interval in which nothing traded as a row with a
time and nothing else, and emitting it would invent a bar whose open, high, low and close are the
same number.

## Trades

A trade arrives on the gateway's ticker push, carrying the venue's own aggressor direction and its
own sequence number. The sequence becomes the trade identifier, which is what makes a replayed
subscription deduplicate rather than repeat.

Two kinds of record are not trades, and neither is emitted:

- **The cached replay.** The first push of a fresh subscription is the gateway replaying the last
  value it holds, at the previous session's price. Emitting it would open every session with a trade
  that never happened, so it is refused before mapping.
- **A record with no usable size.** The gateway pushes ticker records for changes that are not
  trades, observed live as records carrying no size and as fractions below half a share. Both round
  to a quantity that cannot have traded, so they are ignored rather than reported as failures.

Outside market hours the cached replay is usually the only ticker push a subscription receives, so a
trade subscription taken then produces no ticks until the market opens. That is the venue's
behaviour and not a stalled adapter.

## Quotes and the order book

The gateway's quote snapshot carries no bid and no ask, so the quote path is built on the order
book: a quote tick is the book's best bid and best ask. A side the gateway has none of, or a level
priced at zero, produces no tick rather than a tick quoting a price nothing can trade at.

Three properties of the book push are the gateway's and shape what the adapter can promise:

- **The push is the whole book, not the changes to it.** Every push carries the complete ladder, so
  each record is mapped as a snapshot that replaces the book rather than as deltas against it.
- **The push ignores the depth the subscription named.** Sixty levels a side arrive whatever was
  asked for, so the configured `book_depth` is applied by the adapter. The gateway's own ceiling is
  sixty levels: a book request is answered with exactly the depth it named up to that, and one that
  names more is answered with sixty.
- **The push may be from the odd-lot board only if that board was asked for.** The record's own
  board field is what is checked, because a request for the odd board is answered with the ordinary
  one.

The book *request* is a companion to a held subscription rather than a standalone read: without a
subscription the gateway refuses it with a message naming the missing subscription. The push is
also what carries the venue's receive time, which the adapter places rather than the local clock,
and it treats the zero the gateway reports for the cached replay as no instant rather than as the
epoch.

Quotes and book deltas are served from one gateway subscription while remaining separate to a
consumer: a consumer that asked for quotes is given a quote, one that asked for a book is given
deltas, and one that asked for neither is given nothing. The subscription is released only when both
have gone.

## Subscriptions

The gateway's subscription model differs from a strategy's, and the adapter absorbs the difference:

- **A subscription is a counted resource, not a property of one consumer.** Two entities asking for
  the same security and data type share one subscription, and it is released only when both have
  lost interest.
- **A release cannot happen immediately.** The gateway refuses to release a subscription within one
  minute of it being taken, so a release is delayed past that minimum. A security wanted again
  during the delay is not re-subscribed, because it was never released.
- **The allowance is small and shared.** The observed login carried one hundred subscriptions across
  every market. A subscription that would exceed the allowance is refused, naming the allowance,
  rather than evicting another consumer's subscription.
- **Intent outlives the socket.** A subscription is re-issued after a reconnect, including a session
  the adapter had to re-establish itself, and the order book is reseeded afterwards because the
  gateway refuses a book request for a security it does not currently hold a subscription for.

## Corporate actions

Corporate actions are published on the instrument's corporate action topic rather than returned from
a data command, so a strategy receives them through `subscribe_corporate_actions`:

```python
class MyStrategy(Strategy):
    def on_start(self):
        self.subscribe_corporate_actions(InstrumentId.from_str("AAPL.US"))

    def on_corporate_action(self, action):
        self.log.info(f"{action.instrument_id} {action.action} {action.value}")
```

- **A dividend's amount is read from a sentence.** The gateway's dividend record has no amount
  field; its only descriptive field is an English statement such as
  `Cash Dividend: 0.27 USD Per Share`. A record whose statement is not that form is refused, which
  is what keeps a distribution in specie, stated as `Distribution in Specie: 1.00000 MEITUAN-W Share
  for Every 10.00000 Shares Held`, from being fed into an adjustment stage as a cash amount. Read
  across whole series, every United States dividend was valued and the five refusals in the Hong
  Kong series were all distributions in specie.
- **The date fields are `MM/DD/YYYY`.** The schema's own comment says otherwise and is wrong.
- **A split's ratio is the new shares per old share**, read from the two counts the gateway states.
  A consolidation is the same quantity below one.
- **The effective date is placed at UTC midnight**, from the calendar date the gateway states, so
  that an action and the daily bar it adjusts sit on one instant. The gateway also states the epoch
  seconds of that date, and those are the market's own midnight: using them would place a United
  States action four hours from its own bar.

## Configuration

| Field               | Type        | Default                    | Description                                                            |
| :------------------ | :---------- | :------------------------- | :--------------------------------------------------------------------- |
| `host`              | `str`       | `None`, then `MOOMOO_HOST` | The gateway address.                                                   |
| `port`              | `int`       | `None`, then `MOOMOO_PORT` | The gateway port.                                                      |
| `markets`           | `list[str]` | `["US", "HK"]`             | The markets whose instruments are loaded on connect.                   |
| `adjustment`        | `str`       | `"none"`                   | The bar adjustment: `none`, `forward` or `backward`.                   |
| `session`           | `str`       | `"regular"`                | The session the bars cover: `regular`, `extended`, `all`, `overnight`. |
| `book_depth`        | `int`       | `10`                       | Book levels served, capped at the gateway's ceiling of sixty.          |
| `subscribe_trades`  | `bool`      | `True`                     | Whether a trade subscription is honoured.                              |
| `subscribe_quotes`  | `bool`      | `True`                     | Whether quote and book subscriptions are honoured.                     |
| `snapshot_universe` | `bool`      | `True`                     | Whether a market's instruments are priced from a snapshot.             |
| `load_instruments`  | `bool`      | `True`                     | Whether the configured markets are loaded on connect.                  |
| `timeout_secs`      | `int`       | `None`                     | Request timeout, defaulting to the transport's ten seconds.            |

The words given for `markets`, `adjustment` and `session` are the words the configuration also
serializes to, so a configuration read from a file and one built in Python cannot mean different
things. A word that names nothing is refused with the words that do.

`subscribe_trades` and `subscribe_quotes` are promises about what the client will do rather than
tuning knobs: turning one off makes the corresponding subscription fail with a message naming the
setting, instead of quietly taking a subscription that spends an allowance which is small and
shared.

### Usage

```python
from nautilus_trader.adapters.moomoo import MoomooDataClientConfig
from nautilus_trader.adapters.moomoo import MoomooDataClientFactory
from nautilus_trader.common import Environment
from nautilus_trader.live import LiveNode
from nautilus_trader.model import BarType
from nautilus_trader.model import TraderId

trader_id = TraderId.from_str("TESTER-001")

node = (
    LiveNode.builder("MOOMOO-DATA-001", trader_id, Environment.LIVE)
    .add_data_client(
        None,
        MoomooDataClientFactory(),
        MoomooDataClientConfig(markets=["US"], adjustment="none", session="regular"),
    )
    .build()
)
```

The client is multi-venue, so it is registered without a venue and its instruments carry their own.
A strategy then subscribes as it would to any other data client:

```python
self.subscribe_quotes(InstrumentId.from_str("AAPL.US"))
self.subscribe_trades(InstrumentId.from_str("AAPL.US"))
self.subscribe_bars(BarType.from_str("AAPL.US-1-MINUTE-LAST-EXTERNAL"))
self.request_bars(BarType.from_str("AAPL.US-1-DAY-LAST-EXTERNAL"))
```

## Environment variables

- `MOOMOO_HOST`: The gateway address. Defaults to `127.0.0.1`.
- `MOOMOO_PORT`: The gateway port. Defaults to `11111`.

There is no API key and no secret. The gateway holds the login.

## Instrument loading

`load_instruments` loads the configured markets when the client connects, and each market is one
request to the gateway's static information endpoint: the observed United States list held 13,112
securities and the Hong Kong list 3,799.

Pricing those instruments is a second, optional request. A security's price precision is derived
from the snapshot's price spread, which is the only field on either record that states a tick, so
`snapshot_universe` decides whether the loaded instruments carry a tick the gateway stated or the
market's fallback:

- A snapshot request names at most four hundred securities, and the gateway refuses a request that
  names more.
- A batch the gateway refuses is refused as a whole, which measured true for every United States
  batch across the list: the list carries over-the-counter codes throughout, and a batch cannot be
  answered for the members it does hold. Every Hong Kong batch was served.
- A refused batch falls back to the market's fallback precision, and the load reports how many
  instruments that happened to, so a wrong tick is something an operator reads rather than
  discovers. A United States load therefore costs one refused request per four hundred securities,
  which is what `snapshot_universe=False` avoids when the precision is not needed.

Requesting one instrument by name loads just that instrument, from the same two records, and does
not need the market's universe: `request_instrument` for an instrument that is not loaded loads it
and emits it.

## Capability map

The gateway exposes more than this adapter uses. The table states what the adapter does with each,
so a decision can be made against the mapping rather than against the vendored schema.

| Gateway feature                             | NautilusTrader outcome                                                                                                      |
| :------------------------------------------ | :-------------------------------------------------------------------------------------------------------------------------- |
| Static information, snapshot                | Mapped: instruments, and the price precision a snapshot states.                                                             |
| Historical K-line request                   | Mapped: `Bar`.                                                                                                              |
| K-line push                                 | Mapped: `Bar`, one interval per subscription.                                                                               |
| Ticker push                                 | Mapped: `TradeTick`.                                                                                                        |
| Order book push and request                 | Mapped: `QuoteTick` and `OrderBookDeltas`.                                                                                  |
| Dividend and split feeds                    | Mapped: `CorporateAction`.                                                                                                  |
| Basic quote snapshot and its push           | Not used. The snapshot carries no bid or ask, and the quote path is built on the order book.                                |
| Historical tick-by-tick request             | Not used. Trades arrive on the push, and the request exists for seeding and for checking a push against it.                 |
| Rehabilitation feed                         | Not used. The event feeds carry the actions themselves, and the factors would describe a series the adapter does not serve. |
| Gateway notifications                       | Received and reported at debug. They carry no data a consumer can be given.                                                 |
| Exchange status, broker queue, capital flow | Not mapped. No NautilusTrader data type carries them.                                                                       |
| United States options and futures           | Not mapped. The observed entitlement reports no options right for the United States and no futures right at all.            |
| Order submission and management             | Out of scope: this adapter is a data adapter.                                                                               |

## Operations notes

- **A gateway that is not reachable is a connection error, not something retried behind the
  caller.** The client reports it from `connect`, so a node's start-up is where it is noticed.
- **A dropped socket is replaced.** The connection outlives the sockets that carry it: it
  re-handshakes, re-issues the subscriptions the consumers still want, and reseeds the order book.
  Requests outstanding when a socket drops are released rather than retried, because whether a
  request may be sent twice is a property of the request.
- **The historical allowance is counted per security, not per request.** The observed allowance was
  one hundred securities per period, and the adapter checks it before the first page of a series
  rather than per page, because a series for a security already used in the period costs nothing.
  Pagination is therefore cheap and a new symbol is what spends the allowance.
- **The subscription allowance is counted per security and data type.** A one-minute and a
  five-minute bar subscription on one security are two subscriptions, not one.
- **An exhausted allowance refuses rather than evicts.** Nothing is unsubscribed to make room for
  something else, because the victim's data would stop without its consent.
- **The logs name what happened.** The entitlement record is logged on connect, a market load
  reports how many instruments it priced, and a refused subscription names the gateway's own
  message.

### Known limitations

- **Outside market hours a trade subscription produces no ticks.** The only ticker push is the
  gateway's cached replay, which is refused by design.
- **The United States universe is not priced from snapshots.** The gateway refuses every batch of
  it, so those instruments carry the market's fallback precision.
- **A Hong Kong instrument's tick size is banded by price**, so a fallback precision is wrong for the
  instruments whose band it does not describe. The load reports how many were priced from the
  gateway, which is what identifies them.
- **The book push carries the gateway's full ladder** whatever depth was subscribed, so the depth is
  applied by the adapter and a consumer's book is never deeper than configured.
