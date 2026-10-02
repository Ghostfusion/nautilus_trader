# moomoo adapter design: a push-driven OpenD client for US and HK market data

**Status:** design only. No adapter code was written by this probe.

The companion implementation plan is
[moomoo_adapter_implementation.md](moomoo_adapter_implementation.md).

moomoo (the international brand of Futu) does not publish a REST market-data API. Its clients talk
to a locally installed gateway process, OpenD, over a TCP connection carrying a fixed binary frame
around Protobuf payloads, with subscription-based entitlements. That shape decides almost every
design question below, and it makes this adapter structurally different from the REST-polling
adapters in this repository.

Every technical statement in this document was checked against a running daemon and against the
gateway client shipped with it, on the machine used for the probe. Section 2 records the method and
section 10 lists what remains unverified.

| Revision | Change                                                                                                                                                                                         |
| -------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1        | Initial record from a live probe of OpenD with `moomoo-api` 10.10.7008, a plan-and-entitlement read, and a 39-case endpoint sweep                                                              |
| 2        | Resolved the six open questions from a design review; the venue decision was checked against the repository's provider adapters and was not changed, with the reasoning recorded in section 12 |

## 1. Purpose and scope

The question this document answers is narrow: **can this repository gain a moomoo adapter, what
would it be able to do on a standard entitlement, and what must the design get right before any
code is written.**

The answer is yes for market data, with two product families unavailable and one mapping
correction that matters more than the rest.

Proposed first release scope:

- US and HK equity instruments, loaded from the venue's static-info and snapshot endpoints.
- Historical bars, daily and intraday, with explicit adjustment and session semantics.
- Live trades, from the tick-by-tick channel.
- Live quotes and order book, with a caveat about where the bid and ask actually live (section 6.4).
- Corporate actions, from the rehabilitation endpoint.

Explicitly out of this scope, and why, in section 7.3: US options, US futures, capital flow, macro
indicators, rankings, calendars, news, and all order submission. Several of these work and return
data; none of them has an obvious home in the existing domain model, and two of them are refused by
the entitlement anyway.

## 2. Method and evidence

### 2.1 What was probed

The probe ran against a locally installed OpenD with no configuration change:

| Item                  | Observed                                                                              |
| --------------------- | ------------------------------------------------------------------------------------- |
| Gateway               | `MOOMOO_HOST=127.0.0.1`, `MOOMOO_PORT=11111` from the repository `.env`               |
| Listener              | TCP 11111 accepts; 11112 does not                                                     |
| Client library        | `moomoo-api` 10.10.7008, under the system Python 3.12 site-packages                   |
| Login state           | `qot_logined: true`, `trd_logined: true`, program status `READY`, server version 1010 |
| Fraud/disclaimer gate | `is_need_agree_disclaimer: false`                                                     |

Four probe scripts called 39 distinct endpoint groups and printed every return code, so a refusal is
recorded as a refusal rather than as an absence. Field-level evidence, such as the column set of a
frame, was read from the returned objects.

### 2.2 Evidence classes

Three classes of statement appear below, and they are labelled:

- **Verified** - observed in a live call or read directly from a file on disk, with the location
  given.
- **Inferred** - follows from a verified fact, stated as a consequence rather than as an
  observation.
- **Unverified** - cannot be established on this entitlement or setup; listed in section 10.

## 3. What moomoo is, mechanically

### 3.1 A gateway, not a service

The adapter's peer is a process on the same machine, not a remote host. OpenD owns the connection
to the venue, holds the login, and multiplexes every client request over one socket. The
consequences for this repository are practical:

- The adapter has an installation prerequisite that no other adapter here has. There is no
  credential that works without a running local daemon.
- Transport failures are local process failures, not network failures. "Refused connection" means
  OpenD is not running, not that a venue is down.
- The daemon runs on the operator's machine only. A hosted deployment cannot use this adapter
  without a network-reachable gateway, which is a different security conversation (section 10).

### 3.2 The wire protocol, verified

The frame is 44 bytes of little-endian header followed by the payload. The client library's own
codec was read directly, so this is not a transcription from documentation:

| Offset | Field            | Type       | Value observed                                |
| ------ | ---------------- | ---------- | --------------------------------------------- |
| 0..2   | header flag      | `[u8; 2]`  | `0x46 0x54`, the ASCII characters `F` and `T` |
| 2..6   | protocol id      | `u32`      | see the table below                           |
| 6..7   | payload format   | `u8`       | `0` Protobuf, `1` JSON                        |
| 7..8   | protocol version | `u8`       | `0`                                           |
| 8..12  | serial number    | `u32`      | request sequence, monotonic                   |
| 12..16 | body length      | `u32`      | byte length of the payload                    |
| 16..36 | body SHA-1       | `[u8; 20]` | SHA-1 digest of the payload bytes             |
| 36..44 | reserved         | `[u8; 8]`  | zeroed                                        |

The library expresses this as the struct format string `"<1s1sI2B2I20s8s"` in
`moomoo/common/constant.py`, and packs it in `_joint_head` in `moomoo/common/utils.py`, where the
SHA-1 is computed over the serialised payload before encryption. The protocol version constant is
zero.

Protocol identifiers, read from the identifier enumeration in the shipped client:

| Id   | Name                         | Use                                        |
| ---- | ---------------------------- | ------------------------------------------ |
| 1001 | `InitConnect`                | handshake, returns the keep-alive interval |
| 1002 | `GetGlobalState`             | login state and per-market status          |
| 1003 | `Notify`                     | server-initiated notifications             |
| 1004 | `KeepAlive`                  | heartbeat                                  |
| 1005 | `GetUserInfo`                | entitlements and quotas                    |
| 2001 | `Trd_GetAccList`             | trading accounts                           |
| 2101 | `Trd_GetFunds`               | account funds                              |
| 2102 | `Trd_GetPositionList`        | positions                                  |
| 3001 | `Qot_Sub`                    | subscribe and unsubscribe                  |
| 3002 | `Qot_RegQotPush`             | register for push                          |
| 3003 | `Qot_GetSubInfo`             | current subscriptions                      |
| 3004 | `Qot_GetBasicQot`            | quote snapshot                             |
| 3005 | `Qot_UpdateBasicQot`         | quote push                                 |
| 3006 | `Qot_GetKL`                  | k-line request                             |
| 3007 | `Qot_UpdateKL`               | k-line push                                |
| 3010 | `Qot_GetTicker`              | tick-by-tick request                       |
| 3011 | `Qot_UpdateTicker`           | tick-by-tick push                          |
| 3012 | `Qot_GetOrderBook`           | order book request                         |
| 3013 | `Qot_UpdateOrderBook`        | order book push                            |
| 3103 | `Qot_RequestHistoryKL`       | historical k-lines, paginated              |
| 3202 | `Qot_GetStaticInfo`          | static instrument information              |
| 3203 | `Qot_GetSecuritySnapshot`    | snapshot                                   |
| 3211 | `Qot_GetCapitalFlow`         | capital flow                               |
| 3212 | `Qot_GetCapitalDistribution` | capital distribution                       |

Two facts about the message definitions are worth stating because they remove work: **184
Protobuf definition files ship with the client**, so a Rust implementation generates its types from
the venue's own schema rather than reverse-engineering them; and the definitions include
`Qot_GetStaticInfo`, which the Python client of this version does not expose as a method at all.
A Rust client reaches protocol capabilities that the Python surface of the same version hides.

Encryption was read but not exercised. The client applies RSA to the handshake body only when a key
file is configured, and per-connection symmetric encryption only when the gateway negotiated it;
against a local gateway neither is active. A remote gateway is therefore a materially different
implementation, and is out of scope.

### 3.3 The subscription model and its quotas

Real-time data is not free-form. A symbol and data type pair must be subscribed before its push or
query endpoint will answer, and subscriptions are counted against an allowance. The allowance is
reported, not guessed:

| Quantity                      | Observed                               |
| ----------------------------- | -------------------------------------- |
| `sub_quota`                   | 100 concurrent subscriptions           |
| `history_kl_quota`            | 100, of which 27 used and 73 remaining |
| Option subscription allowance | 20                                     |
| Minimum subscription duration | 60 seconds                             |

The one-minute minimum is not documentation, it is enforced: two attempts to release a
subscription seconds after acquiring it were refused with `The Ticker subscription duration for
US.AAPL is too short. Minimum subscription duration is 1 minute.` This is a hard constraint on any
adapter that maps its own subscription lifecycle onto NautilusTrader's, because NautilusTrader
strategy subscriptions can appear and disappear much faster than that.

The historical allowance is the sharper constraint of the two. A hundred requests per thirty days,
shared with the operator's other tools, is roughly one backfill per instrument for a hundred
instruments. Any design that backfills per subscription will exhaust it in an afternoon.

### 3.4 Entitlements as observed

Read from the entitlement endpoint, which reports per-market quote rights:

| Entitlement                | Value | Consequence                            |
| -------------------------- | ----- | -------------------------------------- |
| `us_qot_right`             | `LV2` | US equities and depth, usable          |
| `hk_qot_right`             | `LV1` | HK equities, level 1                   |
| `us_option_qot_right`      | `NO`  | US option chains and analytics refused |
| `us_future_qot_right`      | `N/A` | US futures refused                     |
| `us_future_qot_right_cme`  | `NO`  | CME futures refused                    |
| `us_future_qot_right_cboe` | `NO`  | CBOE futures refused                   |
| `api_level`                | `N/A` | not a gating value here                |
| `update_type`              | `NO`  | no entitlement-upgrade flag in play    |

The refusals are explicit rather than empty: an option-chain request answers `No permission to get
quotes for US.AAPL. Please check US MarketOptions quote permissions.`, and a futures request
answers `Insufficient quote permission. Please go to the Quote Store to purchase a quote card.`
That is useful for the design because a refusal is distinguishable from an error, and can be
reported to the user as a purchased capability rather than as a bug.

## 4. The transport decision

### 4.1 Why the client library cannot be the transport here

The natural implementation would import the vendor's Python client and wrap it. That is not
available: `moomoo-api` 10.10.7008 imports only from the system Python 3.12 installation, while
this project's environment is the `python/.venv` interpreter, which is 3.14 and has no `moomoo`
package. Independently of the interpreter, this repository's adapter namespace is Rust-backed and
gated by a public-export test, so a pure-Python adapter cannot occupy the
`nautilus_trader.adapters.moomoo` namespace.

### 4.2 Options considered

| Option                                                  | Verdict                                                                |
| ------------------------------------------------------- | ---------------------------------------------------------------------- |
| Python adapter wrapping the vendor client               | Not possible: interpreter mismatch, and the namespace is Rust-backed   |
| Rust crate shelling out to a Python helper process      | Rejected: a subprocess per client, two runtimes, and no typed boundary |
| Rust crate implementing the frame and Protobuf directly | Chosen                                                                 |
| Skip moomoo, keep EODHD                                 | Kept as the fallback if the subscription model proves unworkable       |

### 4.3 Decision

**The adapter is a Rust crate that speaks the OpenD frame protocol directly, generating its message
types with `prost` from the Protobuf definitions shipped with the gateway client.**

Three findings make this a bounded piece of work rather than a research project: the frame layout is
fully specified above; the message definitions are available as source rather than as a black box;
and against a local gateway the transport is plaintext, so there is no cryptographic surface in the
first release.

The cost is a new protobuf toolchain dependency in the workspace, which currently has none. That is
a deliberate, reviewable addition, and the build-time code generation is confined to one crate.

## 5. Architecture

### 5.1 Crate and module layout

Following the adapter layout used by the existing data adapters:

```
crates/adapters/moomoo/
  build.rs                 # prost-build over the vendored proto subset
  proto/                   # vendored .proto files, with their source revision recorded
  src/
    lib.rs
    common.rs              # constants, venue, credential, endpoint resolution
    config.rs              # MoomooDataClientConfig
    connection.rs          # socket, handshake, keep-alive, framing, dispatch
    codec.rs               # the 44-byte frame: encode, decode, validate
    subscription.rs        # quota-aware subscription manager
    entitlement.rs         # capability gate and typed refusals
    instruments.rs         # static info and snapshot to Instrument
    bars.rs                # k-lines to Bar
    trades.rs              # ticker frames to TradeTick
    quotes.rs              # book level 1 to QuoteTick
    orderbook.rs           # order book to OrderBookDeltas
    corporate_actions.rs   # rehabilitation rows to CorporateAction
    data.rs                # MoomooDataClient
    factories.rs           # MoomooDataClientFactory
    python/                # configuration projection
```

The crate exposes the same feature shape as the other adapters, so that a data-only adapter is the
default and the Python projection is additive.

### 5.2 Connection manager

One connection serves one client. The manager owns the socket, performs the handshake, and is the
only place that knows about serial numbers.

The handshake returns the keep-alive interval, and the gateway disconnects a socket that goes quiet
for longer than that window, so the heartbeat is not optional: it is part of the connection's
liveness, and its failure is the signal that the connection is gone.

The manager must handle three message kinds on one socket: responses to requests, pushes that
correlate to no request, and server notifications. Responses are matched by serial number; pushes
and notifications are routed to the client by protocol id.

### 5.3 Request and response correlation

Each request carries a serial number and the response echoes it, so a pending-request map keyed by
serial, with a one-shot channel per entry, is the natural shape. The design constraints are:

- The map must be bounded. A gateway that never answers must not grow the map without limit.
- A dropped waiting task must not leave a live entry.
- Responses that arrive after a timeout must be discarded rather than delivered late, because a
  late quote is worse than no quote.

### 5.4 Push routing

Pushes are the primary data path. Subscribe with push enabled, and map the four push protocol
identifiers to domain events: quotes, trades, order book, and k-lines. This is where the adapter is
structurally better than a polling one: the venue sends what changed rather than the client asking
on a timer, so a quiet instrument costs nothing and a busy one is not sampled.

### 5.5 Subscription manager

The subscription manager exists because the venue's model does not match NautilusTrader's:

- NautilusTrader subscriptions are reference-counted and can start and stop at any moment.
- The venue charges an allowance per symbol and type, and refuses to release a subscription for a
  minute after acquiring it.

The manager therefore holds intent separately from venue state: it tracks what the strategies want,
what the venue currently grants, and schedules releases so that an instrument dropped and re-wanted
seconds later reuses the existing venue subscription instead of churning through the allowance.
Quota exhaustion is an admission-control decision, reported to the caller, not a silent failure.

### 5.6 Entitlement gate

Capabilities are decided once, at start, from the entitlement read, and recorded. When a requested
capability is not entitled, the adapter refuses it with a message that names the missing purchase,
using the venue's own words. This keeps "you have not bought this" distinct from "this is broken",
which is the difference between an operator action and an incident.

## 6. Data mapping

### 6.1 Symbols and venue

The venue addresses instruments as `US.AAPL`, market prefix first. NautilusTrader uses a venue
suffix, `AAPL.US`. The mapping is a relocation of the market segment, not a transformation of the
symbol, and the market segment is the venue. The venue is therefore the market code the instrument
came from, matching the rule already established for the EODHD adapter.

The same relocation applies to plates, which are venue objects with their own identifiers and must
not be mistaken for instruments.

Provider identity and instrument identity are separate in this repository, and this adapter keeps
them separate. The client ID is `MOOMOO`; the instrument venue is the market the data came from.
Databento and Tardis, the repository's other data providers, do the same: Databento exposes exchange
venues such as `XNAS` rather than its own name, and Tardis takes the exchange from the payload,
keeping `TARDIS` for the client identity. The alternative, a provider-scoped venue such as
`AAPL.MOOMOO`, was considered and rejected; section 12.1 records the evidence.

### 6.2 Instruments

Two endpoints carry instrument data. The static-info endpoint returns the definition fields and the
snapshot returns the live fields, and the adapter needs both.

Fields available include the code, name, lot size, listing time, security type, exchange type,
contract metadata for derivatives, and per-market status. Note that the listing time is a date
without a timezone, and that exchange type is the listing venue rather than the market prefix.

Price and size precision are not returned as decimal counts. They must be derived, from the
snapshot's price spread where it is present, and from per-market defaults otherwise. This mirrors
the EODHD decision that precision is a configuration default rather than a venue guarantee.

### 6.3 Bars

Historical k-lines return, per row: code, name, time key, open, close, high, low, price-earnings
ratio, turnover rate, volume, turnover, change rate, and previous close.

Three semantics must be explicit:

- **Adjustment.** The request takes an adjustment mode and the venue defaults to forward-adjusted,
  which is exactly the class of error that silently corrupts a backtest. The adapter therefore
  serves **raw bars by default** and requires an explicit opt-in for an adjusted series, so the
  venue's default never becomes this adapter's default. Raw is the canonical series here because it
  is what executed, what a trade and a quote carry, and what a corporate action can be applied to
  deliberately; an adjusted series is a transformation of that, not a property of it. The
  repository's existing rule for the EODHD adapter, that the series served is the series described,
  applies unchanged.
- **Session.** Intraday requests take a session mode, and extended-hours data must be requested
  rather than assumed.
- **Pagination.** A request returns at most a page, with a continuation key. A full history is a
  loop, and each iteration spends one unit of a small shared allowance.

Timestamps are exchange-local and carry no offset, so the conversion to a nanosecond UTC event time
requires a per-market timezone rule. This is the design's most ordinary-looking correctness risk
and its most expensive one to get wrong: the repository already tests timezone and daylight-saving
conventions for this reason, and a naive local-to-UTC conversion is wrong twice a year.

### 6.4 Quotes, and the top-of-book problem

This is the finding that most changes the design.

The quote snapshot returns 62 fields: last, open, high, low, previous close, volume, turnover,
turnover rate, amplitude, suspension flag, listing date, price spread, dark status, security status,
the full option analytics block, and complete pre-market, after-hours, and overnight blocks. It
contains **no bid price and no ask price, at any level** - the column set was enumerated and no
field name matches either concept.

A NautilusTrader quote tick requires both a bid and an ask. Therefore:

**A `QuoteTick` must be built from order book level one, not from the quote snapshot.** The quote
snapshot maps to nothing in the domain model as a tick; its useful fields beyond the book are last
price, session prices, and status.

This is a design correction, not a limitation: the data is available, the book carries it, but an
adapter that emits quote ticks from the snapshot endpoint will emit either invented prices or
nothing at all.

### 6.5 Trades

The tick-by-tick endpoint returns, per row: code, name, time with millisecond precision, price,
volume, turnover, ticker direction, sequence, and ticker type. A live row read:

```
US.AAPL  2026-10-01 16:00:01.554  330.32  2320.0  766342.40  NEUTRAL  7691791651198470516  AUTO_MATCH
```

Two of those fields matter more than the rest. The **direction** is a real venue value, with an
enumeration of bid, ask, and neutral, where bid and ask denote the side that initiated, so a trade
tick can carry a true `AggressorSide` for two of the three cases and no-aggressor for the third. The
**sequence** is a monotonic venue sequence number, so a trade tick can carry a real identifier
rather than a synthesised one.

Both are improvements over the EODHD adapter, which had neither and had to document a synthetic
aggressor and a synthetic identifier.

### 6.6 Order book

The book returns a structured object with bid and ask arrays, plus the receive timestamps for each
side and a book type. Each level is a tuple of price, volume, order count, and an auxiliary field
that was empty at every level observed on this entitlement.

Level sanitiser. The observed entitlement returns depths beyond the default five, and the venue's
book type distinguishes the ordinary book from an odd-lot book. Odd-lot books must not be merged
into the main book.

### 6.7 Corporate actions

The rehabilitation endpoint returns adjustment and distribution history per instrument: ex-dividend
date, split base and result, join base and result, split ratio, per-cash dividend, special dividend,
bonus base and result, per-share dividend ratio, transfer base and result, and per-share transfer
ratio. A companion pair of endpoints returns dividends and stock splits as separate feeds, which is
closer to the domain model, with the rehabilitation feed available for the adjustment factors.

The mapping to `CorporateAction` follows the same rule established for EODHD: the action describes
the price series the bars describe, so the dividend amount must be the one consistent with the
chosen adjustment mode.

### 6.8 Product families and the boundary around the core domain

The probe found more data than the domain model has types for. Rather than declare the excess
permanently out of scope, the design draws three levels, and only the first is in the first release.

**Level 1, core market data, implemented now:** instruments, bars, trades, quotes, order book,
corporate actions, and instrument status. These have NautilusTrader domain types and are the subject
of this document.

**Level 2, standard domain types that already exist:** if a product maps onto a type the platform
already models, such as option greeks or a funding rate, it becomes a later capability slice with
its own conversion rather than a new invention. On this entitlement the two candidates, US options
and US futures, are refused by the venue, so level 2 is empty today.

**Level 3, venue-specific research data, kept outside the core model but not discarded:** capital
flow and distribution, macro indicator lists and history, Fed watch target rate and dot plot,
pre-market, after-hours and overnight rankings, top movers, period change, dividend and
earnings-beat rankings, short interest and daily short volume, valuation detail, company profile,
ownership and holder changes, plate and sector membership, IPO lists, and search. None of these is a
tick, a bar, an instrument, or a corporate action, so forcing one into a core type would be
invention rather than mapping.

Level 3 gets an explicit extension boundary instead of a permanent exclusion: the data client may
later expose it beside its standard data, in its own types, without entering the core market-data
path. That keeps the option open for research uses such as flow, crowding, or event scoring while
keeping the core model honest. Decision D14 records this.

Refused by entitlement, so unusable in the first release for a second reason: US options and US
futures.

## 7. Architectural invariants

1. **No vendor library in the build.** The adapter speaks the protocol; it does not link or shell
   out to the vendor client. This is what makes the adapter buildable in this repository at all,
   and it keeps the gateway's version independent of the adapter's.
2. **No vendor type crosses the crate boundary.** Protobuf messages are decoded at the edge and
   converted to domain types immediately, as the repository requires of every adapter.
3. **Subscriptions have intent and state, separately.** Strategy intent is not the venue's state,
   and the difference is where the one-minute rule is absorbed.
4. **A refusal is typed.** An unentitled capability and a transport failure are different outcomes
   and must not collapse into one error.
5. **The allowance is counted, not assumed.** Historical requests spend a shared quota, so the
   adapter accounts for it and reports exhaustion rather than discovering it.
6. **Time is converted once, with a market timezone.** Exchange-local timestamps become nanosecond
   UTC event times in exactly one place.
7. **The book is the price source for quotes.** No field is invented to satisfy a domain type.
8. **Local only, in the first release.** The encrypted remote-gateway path is unimplemented and
   documented as such rather than half-built. A remote gateway is reserved for a later release and
   not ruled out, so the crate must not assume its peer is on the loopback interface.
9. **Provider identity is preserved at the client boundary.** The adapter's client ID is `MOOMOO`
   and its instruments carry the market venue. A node may run this adapter beside another United
   States source without either impersonating the other, and the two are addressed by client ID
   rather than by renaming the instrument.

## 8. Limits and failure modes

| Condition                          | Behaviour required                                                            |
| ---------------------------------- | ----------------------------------------------------------------------------- |
| OpenD not running                  | Fail fast at connect with a message naming the daemon and the address         |
| Keep-alive missed, gateway drops   | Reconnect, replay subscriptions, resynchronise book state                     |
| Subscription quota exhausted       | Refuse the new subscription, name the allowance, keep existing ones working   |
| Historical quota exhausted         | Refuse the backfill, report the remaining allowance                           |
| Unentitled product                 | Refuse with the venue's own reason, once, at capability resolution            |
| Push gap after reconnect           | Re-subscribe and re-seed the book from a snapshot rather than trusting deltas |
| Duplicate or replayed tick         | Deduplicate on the venue sequence number                                      |
| Malformed frame                    | Reject the frame, keep the connection, log the protocol id and length         |
| Clock skew between local and venue | Timestamps originate from the venue, never from the local clock               |

## 9. Verification plan

The adapter cannot be considered proven without all of the following, in this order:

1. **Codec vectors.** Frame encode and decode proven against fixtures, including a wrong magic, a
   truncated header, a mismatched body length, and a mismatched SHA-1.
2. **Protocol conformance.** A raw exchange with a controlled server proving the handshake, the
   keep-alive, and serial-number correlation.
3. **Mapping fixtures.** Every domain mapping proven from recorded venue payloads, not from
   hand-written approximations.
4. **Subscription manager.** Reference counting, the release delay, quota admission, and replay
   after reconnect.
5. **Live smoke.** One instrument, one bar type, one trade, one quote, one book, against the real
   daemon, with the output shown rather than asserted.
6. **Documentation hooks.** The repository's documentation conventions, link checking, and table
   formatting.

Items that need a purchase cannot be verified at all and must be reported as capability-gated
rather than as tested.

## 10. Risks and unverified areas

**Unverified on this setup:**

- **Push delivery.** The push protocol identifiers exist and subscriptions succeed, but no push
  frame was observed being delivered, because the probe queried rather than listened and the market
  was outside the regular session. Push is the primary data path, so this is the first thing the
  implementation must confirm before anything is built on top of it.
- **The encrypted and remote gateway path.** Read in the client, not exercised.
- **The JSON payload format.** The frame supports it; only Protobuf was used.
- **Push ordering and gap semantics** across a reconnect.
- **Extended-hours and overnight session boundaries**, which the snapshot exposes as separate price
  blocks and which a US equities adapter must handle deliberately.
- **The static-info protocol from Rust**, which the shipped Protobuf definitions support and the
  Python client does not expose, so it cannot be cross-checked against a working Python call.

**Risks inherent to the design:**

- **The historical allowance is small and shared.** This is the single most likely cause of an
  unhappy first experience, because it fails after the user has already decided the adapter works.
- **The subscription allowance is small and shared**, and the one-minute release delay means the
  adapter's own lifecycle churn consumes it.
- **A local gateway is a deployment constraint**, and a remote gateway is a security decision that
  this design deliberately defers.
- **Correctness here is more about time and adjustment than about parsing.** The parsing is
  mechanical; the two ways to be silently wrong are the adjustment mode and the timezone.

## 11. Decisions

| Id  | Decision                                                                                | Rationale                                                                                                                 |
| --- | --------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------- |
| D1  | Rust crate speaking the frame protocol directly                                         | The vendor client is unusable here, and the protocol is specified                                                         |
| D2  | Generate message types from the gateway's shipped Protobuf files                        | The schema is available; reverse engineering is unnecessary                                                               |
| D3  | Push-driven, not polling                                                                | The venue offers push, and polling spends a subscription allowance                                                        |
| D4  | Venue is the market code, symbols relocate their market segment                         | Matches every provider adapter here: the provider is the client ID, the venue is the traded market                        |
| D5  | `QuoteTick` is built from book level one                                                | The quote snapshot has no bid and no ask                                                                                  |
| D6  | `TradeTick` carries the venue direction and sequence                                    | Both are real venue values, unlike the synthesised EODHD equivalents                                                      |
| D7  | Capabilities resolved once at start and recorded                                        | Keeps "not purchased" distinct from "broken"                                                                              |
| D8  | Historical requests are quota-accounted                                                 | The allowance is small and shared; the operator owns it and the adapter enforces admission                                |
| D9  | Subscription intent and venue state are separate                                        | Absorbs the one-minute release rule without leaking it into the strategy                                                  |
| D10 | Bars default to raw, and adjustment and session are explicit                            | The two silent corruption paths, and the venue default is adjusted                                                        |
| D11 | First release is US and HK equities, market data only                                   | Options and futures are unentitled; the rest is a preserved extension, not a discard                                      |
| D12 | Local gateway for the first release, a remote gateway reserved for a later one          | Half-built cryptography is worse than a documented boundary; a local-only invariant would needlessly constrain deployment |
| D13 | Provider identity is the client ID, not the instrument venue                            | Keeps instrument identity comparable across providers and matches the existing adapters                                   |
| D14 | Venue-specific research data stays outside the core model, behind an extension boundary | It has no core type, and discarding it permanently would foreclose research use                                           |

## 12. Resolved decisions

The six questions raised in revision 1 are decided. Each records the recommendation received, the
outcome, and the evidence. Five were adopted as recommended; the venue question was not, because the
repository's own adapters answer it, and the invariant it motivated was kept in an adapted form.

| #   | Question            | Decision                                                                              |
| --- | ------------------- | ------------------------------------------------------------------------------------- |
| 1   | Venue identity      | The market code remains the venue; `MOOMOO` is the client ID (D4, D13)                |
| 2   | Adjustment default  | Raw bars by default, adjusted only on request (D10)                                   |
| 3   | Quota ownership     | Owned by the operator, accounted and enforced by the adapter (D8)                     |
| 4   | Push confirmation   | Push is confirmed before the first release; no polling fallback (D3)                  |
| 5   | Deployment          | Local gateway in the first release; a remote gateway is reserved, not ruled out (D12) |
| 6   | Outside-domain data | Outside the core model, behind a preserved extension boundary (D14)                   |

### 12.1 Venue identity: the market code remains the venue

**Recommendation received:** use `MOOMOO` as a provider-scoped venue, so the identifier shows which
feed the data came from, on the grounds that two feeds for the same market are not interchangeable.

**Outcome: not adopted.** The repository already separates the two concepts the recommendation would
merge. The instrument identifier names the security; the client identifier names the feed. Every
provider and broker adapter here follows that split:

- **EODHD**, a data provider, carries the EODHD exchange code as the venue (`AAPL.US`) and returns
  `None` from `DataClient::venue` as a multi-venue client, keeping `EODHD` for the client identity.
- **Databento**, a data provider, exposes real exchange venues such as `XNAS` and `XLON`, mapping
  each dataset onto them; `DATABENTO` is the client identity.
- **Tardis**, a data provider, takes the exchange from the payload as the venue (`BITMEX`,
  `BINANCE`) and keeps `TARDIS` for the client identity.
- **Interactive Brokers**, a broker, routes data instruments to `SMART` and exchange venues while
  `IB` is the client identity and the broker venue used for execution.

A provider-scoped venue would make `AAPL.MOOMOO` and `AAPL.US` two instruments where the repository
treats them as one security reached by two feeds, and it would make this adapter the only one that
reasons that way. The recommendation's real concern, that the feeds are not interchangeable, is met
where the repository already meets it: the two clients are told apart by client identity and by the
subscriptions addressed to each, and this design states plainly that its data is the moomoo feed and
not a substitute for another.

The decision is recorded rather than merely executed, so a future change of platform convention can
be applied deliberately. If feed-scoped identity were ever required, the change is confined to
symbol and instrument construction; nothing in the transport or the mappings depends on it.

A later requirement settled the question in the same direction. A provider failover chain needs
every provider in it to answer with the same instrument identifier, so a provider-scoped venue would
make EODHD instruments and moomoo instruments that a chain could not join without a translation
layer. See [the provider failover design](data_provider_failover_design.md), section 5.

### 12.2 Adjustment default: raw

Adopted as recommended. The venue defaults to forward-adjusted bars, and this adapter does not
inherit that default. Raw is the canonical series: it is what executed, it is what trades and quotes
carry, and a corporate action can be applied to it deliberately. Adjustment becomes an explicit
transformation the caller asks for, so the two silent-corruption paths named in section 6.3 stay
visible instead of hidden behind a vendor default.

### 12.3 Quota ownership: operator-owned, adapter-enforced

Adopted as recommended, with the nuance that neither extreme is correct. The allowance is not the
adapter's to own: it is shared with the operator's other tools, and the probe showed 27 of 100
historical requests already spent before this adapter existed. It is also not something merely to
report: an adapter that spends the allowance and then announces it has failed the user.

The model is therefore external ownership with local admission control. The adapter tracks what it
has spent, refreshes the venue's authoritative count, treats its own tally as an estimate rather
than the truth, and refuses a request it cannot afford instead of discovering the exhaustion
mid-pagination. Historical requests are explicit, deduplicated, and cached, and the adapter never
backfills automatically as a side effect of a subscription.

### 12.4 Push confirmation: a gate, not a fallback

Adopted as recommended. Push is the adapter's primary data path and the reason it is not a polling
client. No push frame was observed during the probe, so the first implementation task is to confirm
push delivery on a live session across the four channels, and the first release does not ship until
it is confirmed. A polling fallback would not be a safety net: it would spend the subscription
allowance, add latency and load, and, worst of all, leave the event path unproven while appearing to
work.

### 12.5 Deployment: local first, remote reserved

Adopted as recommended, in the wording that keeps the door open. A local gateway is the first
release. A remote gateway is a later release with its own transport and encryption slice, not an
invariant that the peer must always be on the loopback interface. The practical consequence for the
first release is small: the crate must not hard-code the assumption that the address is local, and
the remote path stays documented and unimplemented rather than half-built.

### 12.6 Outside-domain data: a preserved boundary

Adopted as recommended. The probe found capital flow, macro, rankings, short interest, valuation,
ownership, plates, and more, and none has a core domain type. Section 6.8 replaces the earlier "no
domain home" framing with three levels, and the design keeps the third level addressable from the
data client rather than discarding it. Nothing from level 3 is implemented in the first release, and
nothing in the core model is invented to hold it.
