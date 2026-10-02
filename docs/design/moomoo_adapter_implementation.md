# moomoo adapter implementation plan

**Status:** plan only. No adapter code was written.

Companion to [`moomoo_adapter_design.md`](moomoo_adapter_design.md), which records the live probe
evidence this plan is built on. Section references of the form "design 6.4" point there.

This plan maps the work onto the phase sequence in
[the adapter developer guide](../developer_guide/adapters.md), which describes dependencies rather
than release gates. A market-data-only adapter omits the execution phases entirely, and this one
does: there is no order submission in the first release.

| Revision | Change                                                                                                    |
| -------- | --------------------------------------------------------------------------------------------------------- |
| 1        | Initial plan from the live OpenD probe, with the protocol core proven from the shipped client's own codec |

## 1. Scope and reading order

The plan covers the market-data path for US and HK equities: instruments, bars, trades, quotes,
order book, and corporate actions, over a local OpenD gateway, implemented as a Rust crate.

Read the design document first. Three of its findings change what would otherwise be obvious
implementation choices, and skimming past them produces code that compiles and is wrong:

1. The vendor's Python client cannot be used here (design 4.1), so the crate speaks the protocol.
2. The quote snapshot has no bid and no ask (design 6.4), so quote ticks come from book level one.
3. The venue charges a small shared allowance and will not release a subscription for a minute
   (design 3.3), so subscription lifecycle is a subsystem rather than a call site.

## 2. Phase map

| Guide phase             | In this adapter                                                                                |
| ----------------------- | ---------------------------------------------------------------------------------------------- |
| 0 Define scope          | Done by the probe; capability matrix fixed in design 3.4 and design 6.8                        |
| 1 Protocol core         | The frame codec, the connection, and the request/response correlation. Highest risk, first     |
| 2 Instruments           | Static info plus snapshot to `Instrument`                                                      |
| 3 Market data           | Bars, trades, quotes from book level one, order book, corporate actions                        |
| 4 Execution             | Omitted: no order flow in the first release                                                    |
| 5 Optional capabilities | Omitted until the entitlement is purchased: options, futures                                   |
| 6 Factories, projection | Config, factory, PyO3 registration, Python package, stubs                                      |
| 7 Conformance           | Codec vectors, mock-server protocol tests, mapping fixtures, live smoke                        |
| 8 Performance           | Frame microbenchmarks and decoding invariants only; no hot path worth canonical benchmarks yet |
| 9 Documentation         | Integration guide, capability matrix, operations notes                                         |

## 3. Slice 1: the smallest end-to-end path

The first slice proves one path completely rather than sketching several:

**Connect to OpenD, resolve entitlement, load one instrument, backfill daily bars, subscribe to that
instrument's trades, and emit a `TradeTick` that a strategy receives.**

It is chosen because it touches every subsystem exactly once: the codec, the connection, the
handshake, the entitlement read, a paginated historical request, a subscription, a push frame, and
a domain conversion. If any of those is structurally wrong, this slice finds it before there is
code to refactor.

Explicitly not in slice 1: quotes, order book, corporate actions, corporate-action-aware
adjustment, HK, reconnect, and the Python projection. Each is its own slice with its own exit
criteria, listed in section 14.

## 4. Crate layout and feature flags

```
crates/adapters/moomoo/
  Cargo.toml
  build.rs
  README.md
  proto/                     vendored definitions, with the source revision recorded
  src/
    lib.rs                   module tree and rustdoc feature list
    common.rs                constants, venue, credential, address resolution
    codec.rs                 the 44-byte frame
    connection.rs            socket, handshake, keep-alive, dispatch
    mappers/                 protobuf to domain, one module per data type
      instrument.rs
      bars.rs
      trades.rs
      quotes.rs
      orderbook.rs
      corporate_actions.rs
    subscription.rs          intent, venue state, allowance
    entitlement.rs           capability resolution
    config.rs                MoomooDataClientConfig
    data.rs                  MoomooDataClient
    factories.rs             MoomooDataClientFactory
    python/
      mod.rs
      config.rs
  tests/
    data_tester.rs           the repository's data testing entry point
```

Feature flags follow the existing adapters: a default that includes the live client and
high-precision types, a `live` feature for the client, a `python` feature for the projection, and an
`extension-module` feature for the built extension. The README's feature list and the crate's
rustdoc feature list must agree with the manifest exactly, in the same order, because the
documentation conventions hook compares them.

New dependencies are deliberately few and all justified:

| Crate     | Purpose                                      | Justification                                            |
| --------- | -------------------------------------------- | -------------------------------------------------------- |
| `prost`   | Protobuf encoding and decoding               | The venue's schema is Protobuf; hand-rolling it is worse |
| `sha1`    | The frame's body digest                      | Part of the verified header                              |
| `zeroize` | Clearing the secret-bearing buffers          | Matches the existing adapter credential handling         |
| `tokio`   | The socket, the heartbeat, the dispatch loop | Already the async runtime in use                         |

`prost` and `prost-build` are new to the workspace and are the only additions that need review on
their own merits. The vendored `.proto` files remove any need for a network fetch at build time.

## 5. Vendoring the message definitions

Copy the subset of the gateway client's Protobuf definitions that the adapter uses into `proto/`,
with a file recording the source package and version, and generate Rust types in `build.rs`:

```
proto/
  Common.proto
  InitConnect.proto
  KeepAlive.proto
  GetGlobalState.proto
  GetUserInfo.proto
  Notify.proto
  Qot_Common.proto
  Qot_Sub.proto
  Qot_GetSubInfo.proto
  Qot_GetBasicQot.proto
  Qot_UpdateBasicQot.proto
  Qot_GetKL.proto
  Qot_UpdateKL.proto
  Qot_RequestHistoryKL.proto
  Qot_GetTicker.proto
  Qot_UpdateTicker.proto
  Qot_GetOrderBook.proto
  Qot_UpdateOrderBook.proto
  Qot_GetStaticInfo.proto
  Qot_GetSecuritySnapshot.proto
```

Two rules for the vendored set. First, vendor only what is used, so the crate's generated surface
is reviewable; the full set is 184 files and the slice above is a fraction of that. Second, record
the source revision in the directory, so that a later gateway version can be diffed rather than
rediscovered.

The definitions are from the same package version whose codec was read during the probe, so the
generated types and the verified frame layout are known to be mutually consistent.

## 6. Protocol core: the frame codec

The codec is the one part of this adapter that has an exact specification, and it should be
implemented as one small module with no dependencies on the rest of the crate:

- Encode: magic bytes, protocol id, format type, protocol version, serial, body length, SHA-1 of
  the body, then eight zero bytes, all little-endian, followed by the body.
- Decode: read exactly 44 bytes, validate the magic, read the body length, read exactly that many
  bytes, and verify the digest.

Validation is not optional decoration. A mismatched body length desynchronises the stream, so a
frame that fails validation must close the connection rather than be skipped: after a bad length
there is no way to know where the next frame starts. A digest mismatch, by contrast, is safe to
reject frame-by-frame, and should be reported with the protocol id and the length.

The protocol version field is zero, the format type is Protobuf, and the header length is a derived
constant rather than a literal, so that a future field cannot silently shift the body offset.

## 7. Protocol core: the connection

### 7.1 Handshake

The first frame sent is the handshake, carrying the client version, a client identifier, and the
requested push format. The handshake response is where the gateway reports the keep-alive interval,
and that interval governs the heartbeat, so it must be captured before the heartbeat task can be
started.

The client version field is taken from the value the shipped client uses for this gateway version;
it is a constant in the vendor package and should be read from there rather than invented. The
probe verified the handshake fields and their names but did not need the version value, because it
used the vendor client itself.

### 7.2 Heartbeat

A task owns the heartbeat on the negotiated interval. Its failure is the connection's failure
signal. Since the gateway drops a socket that stays quiet beyond the window, a missing heartbeat
manifests first as a closed socket, which is the intended detection path.

### 7.3 Dispatch

One reader task owns the socket's read half and performs exactly one job: decode frames and hand
them to a router. The router classifies by protocol id into three groups:

- A response whose serial number is in the pending map, delivered to the waiting task.
- A push message, forwarded to the data client.
- A notification, logged with its payload.

The pending map is bounded, entries are removed on delivery, timeout, or task cancellation, and a
response that arrives after its entry is gone is discarded and counted rather than delivered.

### 7.4 Reconnect

A dropped connection is expected, not exceptional. Recovery is: reconnect, handshake, re-read the
entitlement, replay every subscription currently held as intent, and re-seed any order book state
from a fresh snapshot rather than continuing from deltas. A book that resumes mid-stream from
deltas after a gap is wrong in a way that is invisible to the consumer, which is why the design
requires the snapshot reseed.

Subscriptions are replayed from intent, not from venue state, because the venue's state died with
the socket and the strategies' intentions did not.

## 8. Subscription manager

The manager is the piece of this adapter that has no equivalent in the REST adapters, because the
venue's model is adversarial to a strategy's natural behaviour.

State it holds:

- **Intent**: which symbol and data type each subscriber wants, reference counted.
- **Venue state**: which symbol and type pairs the gateway currently grants.
- **Allowance**: the subscription quota, the count in use, and the option allowance separately.

Behaviour it must implement:

1. **Acquire**: if the pair is already granted, increment the reference count. If not, admit against
   the allowance, subscribe, and record the acquisition time.
2. **Release**: decrement, and only when the count reaches zero schedule a release after the minimum
   duration has elapsed since acquisition. A symbol re-wanted during the delay cancels the release.
3. **Admission failure**: when the allowance is exhausted, refuse the subscription and name the
   allowance in the error. Never evict another subscriber's subscription to make room, because the
   victim's data would stop without its consent.
4. **Replay**: after reconnect, re-issue every granted pair.

The release delay is the reason intent and venue state are separate fields rather than one map: the
venue state can outlive the last release of intent for up to a minute by design.

## 9. Entitlement gate

Resolved once at start from the entitlement read and stored as a capability record:

- US equity quotes: available at the observed level, level two.
- HK equity quotes: available at level one.
- US options: unavailable, with the venue's reason recorded.
- US futures: unavailable, with the venue's reason recorded.
- Subscription allowance and historical allowance: captured at start and tracked as they are spent.

A capability check happens before the corresponding request is made, and an unentitled request
fails immediately with the venue's own text. The text matters: it tells the operator to buy a quote
card, which is actionable, where "insufficient permission" is not.

## 10. Instruments

Two calls feed the instrument provider, and both are required:

1. **Static information** returns identity and definition fields: code, name, lot size, listing
   time, security type, exchange type, and derivative contract metadata.
2. **Snapshot** returns the live state and the price spread, which is the only field available for
   inferring the price precision.

Mapping rules:

- The market prefix becomes the venue suffix: `US.AAPL` becomes an instrument with venue `US`.
- The exchange type is the listing venue, not the market prefix, and is not the NautilusTrader
  venue. Conflating the two is the same class of mistake as the one already corrected for the EODHD
  adapter, where a listing venue was mistakenly used as a ticker suffix.
- Precision is derived, from the price spread when present and from a per-market default otherwise,
  and the derivation is a single function so that a venue change is one edit.
- Security types the adapter does not model are filtered out rather than approximated into an equity.

## 11. Bars

The request takes a symbol, a range, an interval, an adjustment mode, a session mode, a page size,
and a continuation key.

Implementation requirements:

- **Pagination**: loop until the continuation key is absent, and spend one unit of the historical
  allowance per request, refusing when the allowance is exhausted rather than looping until the
  venue errors.
- **Adjustment**: an explicit, configured choice, defaulting to the value documented in the
  integration guide, and never silently following the venue default.
- **Session**: an explicit choice for intraday intervals, because extended-hours rows are not
  returned otherwise.
- **Ordering and dedup**: rows arrive oldest first within a page, and the adapter must assemble
  pages into one ordered, deduplicated series before emitting, because a consumer that sees a
  duplicate bar cannot tell it from a correction.
- **Time**: the exchange-local timestamp is converted with the market's timezone to a nanosecond
  event time in one function, tested across a daylight-saving transition for the US market.

The interval vocabulary is the venue's, and the adapter maps its own interval type onto it
explicitly, refusing intervals the venue does not offer rather than rounding to the nearest one.

## 12. Trades

The ticker endpoint and its push counterpart carry code, name, time with millisecond precision,
price, volume, turnover, direction, sequence, and type.

Mapping:

- **Direction** maps to the aggressor side with a three-way enumeration: the two directional values
  map to buyer and seller, and the neutral value maps to no-aggressor. This is a real venue value
  and must not be replaced by a synthesised one.
- **Sequence** is a monotonic venue sequence number and becomes the trade identifier, which also
  gives deduplication for free after a reconnect.
- **Time** is exchange-local with millisecond precision, converted by the same single time function
  as bars.

The push path is the primary source. The query path exists for seeding and for verifying a push
against a query during conformance testing.

## 13. Quotes and the order book

### 13.1 Quotes

A quote tick requires a bid and an ask. The quote snapshot provides neither (design 6.4), so the
quote path is built on the order book:

- On a book update, emit a quote tick from level one, using the best bid and the best ask.
- Never fabricate a side. If the book has no bid or no ask, emit nothing rather than a zero price.

The snapshot's other fields are not discarded: last price, session prices, and instrument status
have uses, but none of them is a quote tick, and the adapter must not pretend otherwise.

### 13.2 Order book

The book arrives as a structured object with bid and ask arrays, each level a tuple of price,
volume, order count, and an auxiliary field.

Implementation requirements:

- Preserve the venue's book type, and never merge an odd-lot book into the ordinary book.
- Seed from a snapshot before applying deltas, and reseed after any gap.
- Respect the venue's maximum depth, and do not request more levels than the entitlement serves.
- Emit `OrderBookDeltas` with the venue's receive timestamp for the changed side, not the local
  clock.

## 14. Corporate actions

Three feeds exist. The rehabilitation feed carries ex-dividend dates, split ratios, and adjustment
factors; the separate dividend and split feeds carry the events themselves.

Implementation order: start with the event feeds, because they map to the domain model directly,
and add the rehabilitation factors only when something needs them.

The mapping rule is inherited from the existing adapter work: a corporate action describes the same
price series the bars describe. If bars are served adjusted, the action amounts must be consistent
with that choice, and the choice must be documented in the integration guide.

Dates arrive as calendar dates, not instants, so the effective date is set at the market's midnight
in UTC while the observation time remains the event's own timestamp, matching the separation the
domain type already draws between the two.

## 15. Configuration and environment

Configuration fields, following the existing adapter shape:

| Field              | Purpose                                      |
| ------------------ | -------------------------------------------- |
| `host`             | Gateway address, defaulting to localhost     |
| `port`             | Gateway port, defaulting to the standard one |
| `markets`          | Which markets to load instruments for        |
| `adjustment`       | Bar adjustment mode                          |
| `session`          | Intraday session mode                        |
| `book_depth`       | Requested depth, capped by entitlement       |
| `subscribe_trades` | Whether to hold trade subscriptions          |
| `subscribe_quotes` | Whether to hold quote subscriptions          |
| `timeout_secs`     | Request timeout                              |

There is no API key and no secret. The gateway holds the login, and the adapter's only credential-like
concern is not leaking the gateway address into logs when it is remote.

Environment variables read from the process environment must be registered in
`scripts/strip-adapter-env.bash`, which this repository requires so that a pre-flight run cannot
silently depend on locally configured values. The names the repository's `.env` already uses are
the bare `MOOMOO_` forms.

## 16. Python projection

The Python surface, in the shape the other adapters use:

```
python/nautilus_trader/adapters/moomoo/
  __init__.py     star-imports the compiled submodule and re-exports the public names
  __init__.pyi    generated stubs
```

Requirements:

- The facade's ownership, its `__all__`, and its stub must agree, because
  `tests/unit/adapters/test_public_exports.py` gates exactly that.
- Configuration projection is Rust-side, and stubs are regenerated from the Rust definitions rather
  than written by hand.
- No venue constants are exported from the facade, which the same test also enforces.

## 17. Testing

| Level                 | What it proves                                                                           |
| --------------------- | ---------------------------------------------------------------------------------------- |
| Codec unit tests      | Frame round trip, wrong magic, short header, bad length, bad digest                      |
| Protocol tests        | Handshake, heartbeat, serial correlation, and a raw exchange against a controlled server |
| Mapper fixtures       | Each domain conversion, from payloads recorded during the probe rather than invented     |
| Subscription manager  | Reference counting, release delay, admission refusal, replay after reconnect             |
| Connection tests      | Drop, reconnect, resynchronise, discard late responses                                   |
| Python boundary tests | Import, config extraction, factory construction, public exports                          |
| Live smoke            | One instrument end to end against the real gateway, with output shown                    |

Two rules from the repository's contributor guidance apply with force here. Tests must not depend on
adapter environment variables, hence the registration requirement above. And a test must not be
weakened to obtain a pass: where the venue refuses a capability, the correct artefact is a recorded
capability gate and a documented reason, not a test that accepts either outcome.

Fixtures come from the probe's recorded payloads. Where a fixture cannot be recorded on this
entitlement, the corresponding mapping is covered by a unit test and the live case is recorded as
unverified.

## 18. Documentation deliverables

- `docs/integrations/moomoo.md`, following the structure of the other integration guides: endpoints
  and their mapped types, configuration, usage, limitations, and the gateway prerequisite.
- An entry in `docs/integrations/index.md`, marked as beta while the adapter is new.
- `crates/adapters/moomoo/README.md`, with a feature list that matches the manifest exactly.
- The capability matrix in the integration guide, reconciled against what the tests actually
  proved, including every entitlement-gated refusal.
- An operations section covering the gateway prerequisite, the allowances, and what each failure
  message means.

`RELEASES.md` is not modified, and no workflow or action files are touched, per the repository's
contributor rules.

## 19. Verification commands

This host has no `make`, so the equivalents are run directly. Per slice:

```
cargo fmt -p nautilus-moomoo
cargo clippy -p nautilus-moomoo --lib --no-default-features
cargo clippy -p nautilus-moomoo --lib --features live
cargo clippy -p nautilus-moomoo --lib --features python --no-deps -- -D warnings
cargo nextest run -p nautilus-moomoo
```

A bare clippy run with the Python feature cannot complete on this host because a dependency outside
this adapter does not pass yet, which is pre-existing and unrelated; the run above uses `--no-deps`
for that reason and the limitation is reported rather than hidden.

When the Python surface changes, the extension is rebuilt and the stubs regenerated, and then:

```
python -m pytest tests/unit/adapters/moomoo tests/unit/adapters/test_public_exports.py -q --no-header
```

Documentation changes run the repository's documentation hooks: table formatting, offline link
checking, typography, and the conventions checks. The live smoke is run with the gateway up and the
actual output recorded.

## 20. Explicit non-goals

- **Order submission, modification, and cancellation.** The probe confirmed a paper account and its
  read-only queries, and nothing else about trading. Execution is a separate body of work with its
  own reconciliation requirements, and it is out of scope here.
- **US options and US futures.** Refused by the current entitlement, so they cannot be verified at
  all. They are gated and documented rather than implemented blind.
- **Capital flow, macro, Fed watch, rankings, calendars, news, search, ownership, entering and
  exiting positions in plates.** Data is available for most of these; none has a home in the domain
  model, and the design document declines to invent one.
- **Remote gateway encryption.** Documented as a boundary, not implemented.
- **HK market specifics beyond instrument loading and bars**, until a US path is proven end to end.

## 21. Sequencing

Each step is a separate reviewable change, and each has a stated exit condition.

| Step | Content                                                        | Exit                                                          |
| ---- | -------------------------------------------------------------- | ------------------------------------------------------------- |
| 1    | Crate skeleton, vendored protos, generated types               | The crate builds and the generated types match the schema     |
| 2    | Codec                                                          | Codec vectors pass, including the malformed cases             |
| 3    | Connection: handshake, heartbeat, dispatch, serial correlation | A raw exchange completes against a controlled server          |
| 4    | Entitlement read and capability record                         | The observed entitlements are printed from a live call        |
| 5    | Instruments                                                    | One US equity resolves to a complete instrument               |
| 6    | Bars, with pagination and the allowance accounted              | A paginated range returns one ordered, deduplicated series    |
| 7    | Subscription manager and trades                                | A subscription is held, pushed, released, and replayed        |
| 8    | Order book and quotes from level one                           | A quote tick carries a real bid and ask                       |
| 9    | Corporate actions                                              | Dividends and splits map to the domain type                   |
| 10   | Reconnect and recovery                                         | A forced drop recovers with subscriptions replayed and no gap |
| 11   | Config, factory, Python projection, stubs                      | The public export tests pass                                  |
| 12   | Documentation and the capability matrix                        | A user can configure and operate the adapter from the docs    |

Steps 1 to 4 are the risk: they are where an incorrect protocol assumption would have to be
unwound. Steps 5 to 12 are mechanical once the protocol core is proven, which is the reason the
slice order puts the connection before the data.
