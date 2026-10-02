# Data provider failover design: a priority chain across market-data feeds

**Status:** design only. No code was written.

The request this document answers is to run data providers as **one chain rather than side by
side**: a priority list where the highest-priority provider that can serve a demand serves it, and a
failure falls through to the next provider. The intended order is EODHD first, moomoo second, with
further providers ordered by data coverage and rate limits.

This document records what the platform can and cannot express today, where the chain would have to
live, the preconditions that make it work at all, and the failure modes that make a naive version
dangerous.

| Revision | Change                                                                                                                                                                                             |
| -------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 1        | Initial design from the platform routing and adapter sources, with the EODHD and moomoo limits read from code and from the live probe                                                              |
| 2        | Closed the open decisions: composite client, hop on entitlement refusal, checkpoint fail-back, EODHD authoritative for definitions, no hopping in research builds, one try per provider per demand |
| 3        | Split retry from failover: transient failures are retried at most once on the same provider, and no 4xx response is ever retried                                                                   |

## 1. Summary of the finding

The chain is **buildable, but it is a new component, not a configuration value**, and it is only
usable once a second equity adapter exists.

Three facts decide the shape of the work:

1. The data engine routes each command to exactly one client, chosen statically. There is no second
   choice and no failure memory (section 3).
2. A chain only works if every provider in it produces the **same instrument identifier** for the
   same security, which the current venue decision already gives for EODHD and moomoo, and which a
   provider-scoped venue would have broken (section 5).
3. The secondary provider's own quota caps how much it can absorb, so failover **degrades** rather
   than scales (section 7).

## 2. What was asked, restated precisely

- Providers are chained, not parallel.
- Priority: EODHD, then moomoo, then others ordered by data coverage and rate limits.
- On failure of a higher-priority provider, the next serves the demand.
- Example given: for an endpoint returning the same or similar data, use EODHD first, and if it
  fails for any reason, call moomoo.

The phrase **"for any reason"** is the part that needs the most care, because several things that
look like failures are not, and treating them as failures silently substitutes a different dataset.
Section 8 turns that phrase into an explicit classification.

## 3. Feasibility: what the platform provides today

### 3.1 Routing is static and picks one client

The data engine holds a client table, a venue-to-client map, and one default client:

- `crates/data/src/engine/mod.rs:466-479` registers a client, optionally inserting a venue route,
  and promotes the first venue-less client to the default.
- `crates/data/src/engine/mod.rs:549-570` sets a venue route and **errors** if that venue is already
  routed to a different client.
- `crates/data/src/engine/mod.rs:855-876` resolves a command in this order: the command's explicit
  client ID, then the venue route, then the default client.

`docs/concepts/live.md` states the same rule for execution clients, and the data path shares it:
*A command's client_id selects a registered client directly. Venue and default routes select a
client when neither the command's client_id nor account-based routing resolves one.*

There is no second candidate. Once the engine has chosen a client, a failure in that client is a
failure; nothing else is tried.

### 3.2 No failover, priority, or fallback exists anywhere

A search across the live, data, and common crates for failover, fallback, and priority returns only
unrelated machinery: execution reconciliation fallbacks, message-bus handler *dispatch ordering*,
option-chain ATM fallback windows, and log rotation precedence. None of them selects a data provider.

**Conclusion: the chain cannot be expressed as configuration today. It must be a component.**

### 3.3 The legal extension points

Two exist, and both avoid touching core:

- **A data client and factory.** `DataClientFactory` is a public trait, and
  `LiveNodeBuilder::add_data_client_with_routing(name, factory, config, routing)`
  (`crates/live/src/node/builder.rs:449-488`) registers a client under a name with a routing config.
  A composite that owns several provider clients behind one factory registers as one client.
- **A plug-in.** `PluginConfig` (`crates/live/src/node/config.rs:720`) loads a Rust-native cdylib
  plug-in configured on the node, so a chain could ship without being compiled into an adapter.

## 4. Where the chain lives

| Option | Shape                                                | Cost                                                      | Verdict                                                       |
| ------ | ---------------------------------------------------- | --------------------------------------------------------- | ------------------------------------------------------------- |
| A      | One composite `DataClient` owning N provider clients | New component, no core change                             | Chosen                                                        |
| B      | Priority list in the data engine's routing           | Core change affecting every adapter and every client type | Not now; keep as the long-term home if the platform adopts it |
| C      | A user-space actor issuing requests to a provider    | Requests only, bypasses subscriptions and the engine      | Rejected as the primary mechanism                             |

Option A works because the engine's view stays simple: it sees one client with one client ID and one
`venue()` answer, and everything about priority is invisible to it. The composite receives the
engine's event sender, hands it to the child clients, and routes each subscribe, unsubscribe, and
request command to the provider that currently serves that demand.

**The one hard problem in option A is event identity.** Venue payloads are converted to domain
events by the child adapters, and those events carry the client identity of the child. The composite
must present a single identity to the engine, so it owns the identity rewrite at the boundary, and
that rewrite is the first thing a test must pin down. This is the main implementation risk of A.

## 5. The precondition: one instrument identity across the chain

Failover is meaningless unless every provider in the chain answers with the **same `InstrumentId`**
for the same security. Otherwise the cache holds two instruments, the strategy subscribes to one,
and the other leg can never serve it.

The current venue decision already gives this:

- EODHD yields `AAPL.US` (`crates/adapters/eodhd/src/providers.rs:40-61`).
- moomoo yields `AAPL.US` (design D4), because its market code is `US`.

Two consequences follow, and both are load-bearing:

1. **This requirement rules out a provider-scoped venue.** If moomoo instruments were `AAPL.MOOMOO`,
   they would not be the same instrument as EODHD's `AAPL.US`, and a chain between the two could not
   be built without a translation layer. The chain requirement and the provider-scoped venue
   proposal in `moomoo_adapter_design.md` section 12.1 are in direct tension, and the chain
   requirement wins on the requester's stated intent.
2. **Providers whose venues differ cannot join without a mapping.** Databento addresses instruments
   by MIC venue (`XNAS`, `XLON`; `crates/adapters/databento/src/symbology.rs:107-110`), so it does
   not join an EODHD-moomoo chain as-is. It would need a venue mapping that makes `AAPL.XNAS` and
   `AAPL.US` the same instrument, which is a deliberate identity decision and not a free one.

A second identity problem sits beside the first: **two providers can disagree about the same
instrument's definition.** Price precision, lot size, and currency may differ between EODHD's
derived precision and moomoo's derived precision. The chain must nominate **one provider as
authoritative for instrument definitions** and refuse to let a fallback overwrite the cached
definition. EODHD is that provider, with the coverage rule recorded in section 14.4.

## 6. Coverage matrix: what can chain with what

Only two market-data providers for equities exist in this repository today, so the chain starts with
two legs. The remaining adapters are either venues in their own right (the crypto exchanges), a
different asset class (Tardis, crypto), or address instruments by a different venue scheme
(Databento).

| Data type                              | EODHD                                    | moomoo             | Notes                                                           |
| -------------------------------------- | ---------------------------------------- | ------------------ | --------------------------------------------------------------- |
| Instruments                            | Yes                                      | Yes                | Different precision derivation; one must be authoritative       |
| Historical bars (daily/weekly/monthly) | Yes                                      | Yes                | EODHD `/eod`; moomoo historical k-lines against a 30-day quota  |
| Historical bars (intraday)             | Yes                                      | Yes                | EODHD `/intraday` 1m/5m/1h; moomoo intraday with session mode   |
| Live bars                              | Polled                                   | Push               | EODHD has no push channel and polls; moomoo pushes (unverified) |
| Trades (live)                          | WebSocket, when entitled                 | Push, tick-by-tick | Different granularity                                           |
| Trades (historical)                    | Per the subscription path                | Yes                |                                                                 |
| Quotes (live)                          | Delayed REST, or WebSocket when entitled | Book level one     | Different provenance and latency                                |
| Order book                             | **No**                                   | Yes                | EODHD cannot serve this at all                                  |
| Corporate actions                      | Yes                                      | Yes                | Both map to `CorporateAction`                                   |
| Instrument status                      | Limited                                  | Yes                |                                                                 |

The matrix decides the **per-data-type** priority, which is not the same as the per-provider
priority. EODHD is first for bars and delayed quotes as requested; for the order book the chain must
start at moomoo, because the higher-priority provider has no such data.

## 7. Rate limits, and the capacity constraint that decides the design

Verified limits:

| Provider | Limit                                                         | Source                                                           |
| -------- | ------------------------------------------------------------- | ---------------------------------------------------------------- |
| EODHD    | 10 REST requests per second                                   | `crates/adapters/eodhd/src/common.rs:77-80` (`EODHD_REST_QUOTA`) |
| moomoo   | 100 concurrent subscriptions, shared                          | Live probe, `sub_quota`                                          |
| moomoo   | 100 historical requests per 30 days, shared, 27 already spent | Live probe, `history_kl_quota`                                   |

**The secondary must be able to absorb the primary's load.** EODHD at ten requests per second can
absorb a large symbol set. moomoo cannot: beyond roughly a hundred concurrent subscriptions, or past
its historical allowance, the second leg is full, and a chain that falls over to it does not fail
over at all, it fails.

So failover here **degrades rather than scales**. The chain needs three behaviours the word
"failover" alone does not imply:

1. **Admission control on the fallback**, because the fallback is the scarcer resource.
2. **Load shedding with a reported reason**, rather than a queue that grows silently.
3. **An escalation signal**, so that a primary outage is visible to the operator instead of being
   absorbed invisibly for weeks.

## 8. Failure classification: when to retry and when to hop

"Fails for any reason" is not implementable as written. The repository already has the right shape
for this problem in the execution path, where a command outcome is classified as a definite local
failure, a definite venue rejection, or an ambiguous outcome, rather than collapsed into one error.
The chain needs the same discipline for data.

| Event                                                | Retry the same provider | Hop to the next provider                                     |
| ---------------------------------------------------- | ----------------------- | ------------------------------------------------------------ |
| Connection refused (OpenD down, DNS failure)         | No                      | Yes                                                          |
| Request timeout                                      | Once                    | Yes, if the retry also fails                                 |
| HTTP 5xx, transport reset                            | Once                    | Yes, if the retry also fails                                 |
| Ambiguous (request sent, no answer)                  | Once                    | Yes, if the retry also fails                                 |
| HTTP 429, rate limited                               | **No**                  | Yes, recorded as a rate-limit event                          |
| HTTP 403, forbidden (credential or plan)             | **No**                  | Yes, recorded as a credential or capability gap              |
| HTTP 404, resource or symbol unknown to the provider | **No**                  | Yes: the provider does not have it                           |
| Any other 4xx (400, 401, 422)                        | **No**                  | **No**: a caller or configuration defect, not a provider gap |
| Entitlement refusal carried in the payload           | No                      | Yes, and recorded                                            |
| Response parse failure                               | No                      | Yes, and recorded: the provider's payload changed            |
| HTTP 200 with an empty result                        | No                      | **No**: the provider answered, and the data does not exist   |
| Instrument absent from a 200 response                | No                      | **No**: same as above                                        |
| Invalid input from the caller                        | No                      | **No**: hopping would hide a caller bug                      |

Two of these lines carry most of the risk. **Empty results must never trigger a hop**, or a symbol
that legitimately has no data on the primary would quietly be served from a different source with
different semantics. And **entitlement refusals must be recorded**, or a plan downgrade turns into
an unexplained change of data quality.

The retry and the hop are separate decisions, and keeping them separate is the point. **A retry is
for a transient condition on a provider that is otherwise the right one; a hop is a deliberate move
to a different dataset.** Two rules follow, and they govern every line above.

- **No 4xx response is ever retried.** A 4xx is the provider stating that the request, the
  credential, or the caller's entitlement is wrong, or that the caller is being throttled. A second
  identical request cannot change the answer, and for a rate limit it makes the condition worse.
  Stating it as "no 4xx is retried" covers 429, 403, and 404 and every related client error with one
  rule rather than an enumeration that would drift.
- **The retry budget is one attempt, and only for a transient failure**: a timeout, a 5xx, a
  transport reset, or a request that was sent but never answered. The retry is safe here because
  this is a read-only data client, so a repeated request is idempotent; the same budget would not be
  safe on an order path, where a retry can duplicate a state change.

A 4xx still hops where it means the provider cannot help at all, so the hop policy is unchanged:
a rate limit, a credential or plan refusal, and a resource the provider does not carry each move to
the next provider once, and are never retried on the provider that refused.

## 9. What failover means per data class

### 9.1 Historical requests: safe, and this is the case in the request

A historical bar, instrument, or corporate-action request is a single exchange with a definite
answer. If the primary cannot answer, asking the next provider is sound, provided the classification
above is respected and the result is tagged with the provider that served it.

### 9.2 Instrument loading: safe, with the authoritative-definition rule

Loading instruments from the next provider is safe, but a fallback load must not overwrite a
definition already cached from the authoritative provider, or precision can change under a running
strategy (section 5).

### 9.3 Live subscriptions: not safe by default, and where the design is strict

A live subscription is a stream, not a request, and a mid-stream switch is not a retry. To the
strategy it is indistinguishable from a venue outage followed by a new venue: the timestamps,
granularity, latency, and even the meaning of fields change. Concretely, on this pair:

- EODHD streamed quotes are delayed REST or WebSocket; moomoo quotes are derived from book level one.
- EODHD trade aggressor is synthesized; moomoo's is a real venue value.
- EODHD bars are polled; moomoo bars are pushed.

The rules that follow:

1. **Switch only on a whole-provider failure**, never per instrument, so that one instrument cannot
   come from two sources at once.
2. **Never interleave**: the composite unsubscribes from the old provider before subscribing to the
   new one for the same demand.
3. **Emit an explicit continuity event** so the change of source is observable rather than silent.
4. **Fail-back must be deliberate.** Returning to the primary means an unsubscribe and a fresh
   subscription, and moomoo refuses to release a subscription within its one-minute minimum
   (`moomoo_adapter_design.md` section 3.3), so a chain that flaps between providers burns quota and
   produces gaps. Prefer staying on the secondary until a scheduled checkpoint.

## 10. Availability is not consistency

This is the caveat that matters most for research integrity, and it is the reason the chain must be
recorded rather than merely implemented.

The two legs **do** agree on bar adjustment, which is what makes chaining bars defensible at all:
the EODHD adapter serves the raw `close` price and prefers the as-reported dividend amount over the
split-adjusted one precisely because *an action describes the same price series the bars describe*
(`crates/adapters/eodhd/src/corporate_actions.rs:38-41`), and the moomoo adapter now defaults to raw
bars (D10). Without that agreement, a backtest spanning a hop boundary would silently mix two series.

They do **not** agree on everything else: tick granularity, quote provenance and latency, trade
aggressor, precision derivation, and session handling. A strategy that hops therefore receives a
different quality of data under the same `InstrumentId` without any code change.

Two rules follow:

- **Tag every emitted dataset with the provider that served it**, so a result can always be traced.
- **Forbid hopping inside a single backtest or research build** unless explicitly enabled, because
  a dataset assembled from two providers is not the dataset either provider published.

## 11. Health, stickiness, and recovery

- Health is tracked per provider, per data type, and per market: EODHD can be down for equities and
  healthy for nothing else, and moomoo's US entitlement differs from its HK entitlement.
- A **circuit breaker** opens after a small number of consecutive failures and probes once after a
  cooldown, so a dead primary is not retried on every tick.
- **Stickiness**: once a demand is being served by a fallback, it stays there until a defined
  checkpoint, rather than snapping back the moment the primary answers again.
- **Observability**: every hop records why, from which provider, to which provider, and for which
  demand, and the counts are exported. A chain that hides its hops hides the outage.

## 12. Risks

- **Silent substitution.** The largest risk, and the reason section 8 exists. A chain that hops on
  empty or ambiguous answers returns a plausible dataset that is not the requested one.
- **Secondary exhaustion.** moomoo's hundred subscriptions and thirty-day historical allowance are
  shared with the operator's other tools, so the fallback can be full before it is needed.
- **Definition drift.** Two providers, one `InstrumentId`, two definitions of precision or lot size.
- **Concealed outages.** A primary that has been down for a week looks like a working system unless
  hops are surfaced.
- **Quota burn.** Falling over to a metered provider spends a monthly allowance for a transient
  primary blip, unless the breaker and the stickiness rules prevent it.
- **Mixed-series research.** A dataset assembled across a hop is neither provider's dataset
  (section 10).

## 13. Sequencing

The chain cannot be used before it has a second leg, so the order is forced:

| Step | Content                                                                    | Exit                                                                                                         |
| ---- | -------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| 1    | Chain interface, failure classification, and a deterministic fake provider | Hop and no-hop cases pass without any network                                                                |
| 2    | Composite data client and factory, requests only                           | Instruments and historical bars served by the primary, and by the secondary when the primary is made to fail |
| 3    | Health, circuit breaker, stickiness, hop observability                     | A forced primary outage hops once, reports why, and does not retry per tick                                  |
| 4    | Live subscriptions under the strict rules of section 9.3                   | A streamed demand moves source exactly once, with a continuity event and no interleaving                     |
| 5    | moomoo adapter, the second real leg                                        | The chain serves a US equity from EODHD and from moomoo by the same identifier                               |

Step 5 is a prerequisite in practice, not a detail: until a second equity adapter exists, the chain
has nothing to fall over to.

## 14. Resolved decisions

All six decisions are closed. Where a decision contradicted nothing else in the document it is
adopted as written; the authoritative-provider decision needed one addition, recorded in 14.4.

| #   | Question                                          | Decision                                                                     |
| --- | ------------------------------------------------- | ---------------------------------------------------------------------------- |
| 1   | Where the chain lives                             | A composite data client behind the public factory trait, with no core change |
| 2   | Hop on entitlement refusal                        | Yes, and the refusal is recorded against the demand that triggered it        |
| 3   | Fail-back policy                                  | Stay on the secondary until a scheduled checkpoint                           |
| 4   | Authoritative provider for instrument definitions | EODHD, wherever EODHD covers the instrument                                  |
| 5   | May a backtest hop                                | No by default                                                                |
| 6   | Maximum hops per demand                           | Each provider is tried at most once per demand                               |

### 14.1 Where the chain lives

Adopted. The composite owns one client identity, N provider clients under it, and the command
routing between them, while the engine continues to see a single client. Section 4 records the
option and the event-identity risk the first test must pin down. A platform-level priority list
remains the cleaner long-term home if the engine ever adopts one, and nothing here prevents that
move later.

### 14.2 Hop on entitlement refusal

Adopted. A capability gap is a data-availability gap, so the next provider is tried, but the refusal
is recorded and surfaced against the demand that triggered it. A plan downgrade must appear as a
reported reason, never as an unexplained change in data quality.

### 14.3 Fail-back policy

Adopted. A demand that has moved to the secondary stays there until a scheduled checkpoint rather
than returning the moment the primary answers again. Immediate fail-back would thrash against
moomoo's one-minute minimum subscription duration and spend quota on each oscillation.

### 14.4 Authoritative provider for instrument definitions: EODHD

EODHD is the authoritative source for instrument definitions. This is coherent with EODHD being the
primary provider, and it settles the precision and lot-size question raised in section 5.

One rule is required to make it safe: **EODHD is authoritative wherever it covers the instrument**.
That has two consequences.

- **Covered instrument, primary temporarily unreachable.** A fallback definition is **not** accepted,
  because accepting it would let precision or lot size change under a running strategy. The
  instrument is either deferred until EODHD answers, or loaded as explicitly provisional and
  replaced from EODHD at the next deliberate load, never silently overwritten.
- **Instrument EODHD does not cover.** Where EODHD has no coverage, for example an instrument outside
  its exchange list or a market it does not serve, the highest-priority provider that does cover it
  is authoritative for that instrument.

A provisional definition must be distinguishable from an authoritative one, in the cache or in the
chain's own bookkeeping, so a later reload cannot be mistaken for a correction of live data.

### 14.5 May a backtest hop

Adopted: no by default. A dataset assembled across a hop is neither provider's dataset (section 10),
so a research build is served by one provider unless hopping is deliberately enabled and the
resulting provenance is recorded alongside the result.

### 14.6 Retry and hop budget

Adopted, with the retry policy split out from the hop policy.

- **Each provider is retried at most once per demand**, and only for a transient failure: a timeout,
  a 5xx, a transport reset, or an unanswered request (section 8).
- **No 4xx response is retried.** A rate limit, a credential or plan refusal, or a resource the
  provider does not carry moves straight on, because a second identical request cannot change the
  answer and a rate limit would only be aggravated.
- **Each provider is entered at most once per demand**, so the chain never returns to a provider it
  has already left, and a hop always moves to the next provider in the priority order.
- **The chain stops** when the providers are exhausted, and reports the last failure rather than
  looping.

On a two-leg chain this bounds a demand at one retry per provider and one hop: at most three
attempts in the worst case. The rule is unchanged as legs are added, and it bounds latency and
request volume by the number of configured providers rather than by an unbounded retry count.

## 15. As implemented: the classification and the chain

Step 1 of the sequencing is built as the `nautilus-failover` crate, which holds the part of the
arrangement that is not a provider. Three things were decided while writing it.

- **The class is decided once, in one place.** A failure is classified as transient, a provider gap,
  or a defect, and the two decisions follow from the class rather than from the failure: a transient
  failure costs one retry and then hops, a gap hops at once, and a defect stops the chain. The
  status reading lives with the classification, so no call site decides for itself whether a 403 is
  worth another attempt.
- **Two rows of the table are one variant each.** A request that timed out and a request whose
  outcome is unknown because the transport ended are the same condition for a read-only client,
  where a repeated request is idempotent, so they are one `Unanswered` failure rather than two. A 5xx
  is one `ServerUnavailable` carrying its status.
- **An unconfigured chain is not a failed one.** A chain with no providers returns its own outcome
  rather than a provider failure, because there is no provider to blame and a caller that registered
  none has a configuration defect rather than an outage.

The retry budget is an argument to the decision rather than a counter inside it, which is what keeps
the budget per demand: the caller that knows where a demand begins is the caller that knows how much
of it has been spent.

The crate ships a provider that answers from a script, so that what a chain does is provable without
a network, and a trace that names every provider that was asked and why each could not answer. The
trace is what makes a hop explainable rather than merely visible in the data, and it is the input a
later step needs to export hop counts.

## 16. As implemented: the composite boundary

Step 2 needs four things settled before any request work, and reading the engine settled them. They
are recorded here because two of them change what section 4 promised.

**The rewrite is the client identity, and the correlation needs no rewrite at all.** A provider
answers a request with a `DataResponse` whose `correlation_id` is the *request's* own identifier. The
composite therefore forwards a request unchanged, and the identifier the engine matches the answer
by comes back through it untouched. Had the provider minted an identifier of its own, the composite
would have had to hold a mapping from it to the caller's for the lifetime of every request, which is
a table to leak and a correlation to get wrong; it does not, because the response is built from the
request.

**The client identity is normalised rather than left to the provider.** A provider builds the
response's `client_id` from the request when the request names one and from its own configuration
when it does not, so whether a forwarded request comes back with the chain's identity depends on the
provider honouring a field it has no obligation to honour. The composite therefore sets it rather
than trusting it: what the engine sees is the chain, whoever answered.

**One of the two legs answered a request with data and no response at all.** Neither the EODHD
client nor the moomoo one sent a `DataResponse` for a bar request, so neither completed a request in
the engine's own pipeline: the data arrived and the caller was left to time out. The moomoo client now
answers a bar request and an instrument request with a response carrying the caller's identifier,
both of which were checked against the gateway, and a request for a universe the same way; the EODHD
client still does not. It
is recorded here because the composite's hop depends on the same response: a provider that cannot
answer is only distinguishable from one that has not answered yet if it says so.

**A provider's failure is not visible from its request API.** `DataClient::request_bars` and its
siblings return whether the command was *sent*; the outcome arrives later on the event channel, and
a provider that cannot answer reports it there or only in a log. A hop on a request is therefore not
possible from the request call alone, and the composite has to see what its providers emit.

**The composite can see what its providers emit, without their cooperation.** A provider reads its
data event sender from the thread it is constructed on, so a composite that replaces that sender
while it builds its providers receives everything they emit. That is the tap: the providers are
unchanged, the identity rewrite happens once, at the boundary, and the engine is given a single
client's worth of events. It also means the composite is the only thing that can decide a request
has failed, which is what the second finding requires.

**The boundary is a value, not a task.** It is built as a tap that reads one event from a provider
and writes one event for the engine, with the rewrite as a pure function. That is deliberate: the
property the engine depends on is that the correlation identifier is the caller's and comes back
untouched, and a pure function makes that a test rather than an argument. Only a response carries an
identity at all, so only a response is rewritten, and market data passes through unchanged.

**The sender a provider captures and the task that drains it belong together.** A provider takes the
sender it finds when it is constructed, so a provider built while the engine's own sender is
installed writes past the chain entirely and nothing about it is observed. The two halves are
therefore created as one thing and handed out as one thing, which is why the composite installs the
boundary only for as long as it is building its legs.

**What answers a demand is a response carrying the caller's correlation identifier, and nothing
else.** So the chain keeps a register of the demands it is waiting on, and an answer is matched
against it on the way out. A demand that has stopped waiting is removed rather than counted, so a
late answer cannot stand in for a demand made later - which is the difference between a chain that
hops because it saw a provider refuse and one that hops because it looked away too soon.

**A live request cannot wait for its answer.** The engine calls a client's request method from its own
loop, so that call has to return rather than wait: waiting would stall the engine and everything
queued behind it, and a client is not `Send`, so the wait cannot be moved to a task instead. A hop on
a live request is therefore decided from what a provider says when it is asked - that it took the
demand, or why it would not - while a provider that took a demand and then failed to answer is learnt
off the events, for the next demand rather than for the one already gone.

**A provider that will not take a demand is unreachable, whatever its client's error says.** What a
request call can report is only that the client could not hand the demand over, and that is what
`Unreachable` means: there is nothing on the other end for it. The chain moves on rather than
repeating the attempt, as it does for a rate limit and a refusal. Reading every such error the same
way costs one wasted request when the error was the caller's own defect, and it costs nothing else:
the next provider refuses for the same reason, and the chain stops at the end of the legs.

**The boundary is installed for exactly as long as the legs are being built.** A provider takes the
data event sender it finds when it is constructed, so the sender has to be the chain's at that
moment - and must be put back immediately afterwards, because anything else built on the same thread
would otherwise be rewritten into the chain's identity and the engine would be told about a client
that does not exist. The window is closed by the scope rather than by remembering to close it.

**A leg is a factory and a configuration, and neither is serializable.** The engine's own
configuration path is serde, and a `ClientConfig` is a trait object, so a chain's configuration
cannot be a serialized struct of legs the way a single provider's configuration is. The chain is
therefore assembled rather than deserialized, in the same way a node already assembles its clients,
which is a public API decision rather than a detail: a chain is built by naming its legs in priority
order, and each leg is the same pair of factory and configuration that a node would register on its
own.

## 17. As implemented: health, and what a chain does after a hop

**A demand being answered is the only proof that a provider served it.** A deadline is what turns that
around: a demand that a provider took and did not answer within its deadline is a provider that did
not serve, and the boundary is the only place that can notice. It is held against the provider that
took the demand, so the *next* demand does not go there. The demand already lost stays lost: the
chain cannot re-ask a provider from its own accounts, because it is not the thread that owns the
client, and pretending otherwise would be pretending a request had never been made.

**Stickiness is the ordering rather than a flag.** A demand starts where the chain last answered, and
the priority order fills in behind it. A provider that has recovered is therefore not asked again the
moment it is willing: it is asked when the provider that replaced it fails, or when the chain is
deliberately brought back. Fail-back is a decision rather than a side effect of recovery, which is what
section 14.3 asks for, and it is also what keeps a metered fallback from oscillating.

**The retry budget is not reachable from a live request.** A retry is for a transient failure, and the
only failures a live request can see are the ones a provider reports when it is asked, which are gaps
rather than transients. The budget remains as policy for a caller that drives the chain itself and so
owns its providers' failures; from the composite, a failure is a hop or it is nothing.

**What the chain knows about itself is part of its interface.** Every provider's attempts, failures,
answers and hops are kept, together with why it last failed and whether it has been set aside. A chain
that kept its hops to itself would be a system whose outages are invisible precisely because it
survived them.

## 18. As implemented: streams

**A stream is a demand the chain holds rather than one it answers.** The chain records what it is
streaming and which provider serves it, and that record is what makes the strict rules of section 9.3
implementable at all: what moves is a provider's streams, in the order they were taken, rather than
one instrument at a time.

**The old subscription is given up before the new one is taken.** A stream that is briefly absent is
visible to whoever is watching it; a stream served from two providers at once is not, and it is two
datasets where the caller asked for one. The order is pinned by a test over the calls both clients
were given, because neither client can see it alone.

**A move is announced.** The chain writes a `CustomData` event under the `FailoverContinuity` data
type, carrying the stream, the provider it came from, the provider it went to, and the reason - which
is the failure the provider was set aside for. It is written by the chain about itself rather than by
a provider, so it goes to the engine directly: there is no provider's identity in it to rewrite.

**Fail-back returns to the priority order, and only when asked.** Stickiness is what a checkpoint is
an exception to, so a checkpoint deliberately ignores the provider currently being served from and
looks for the highest-priority provider that can serve. Nothing in the chain does that on its own,
because moving back means a fresh subscription and a provider that may refuse to release the old one
for a minute.

**A stream moves when the chain is next asked to do something.** The boundary cannot call a provider
from its own task, and a provider is not `Send`, so a stream whose provider dies does not move by
itself: the move happens at the next streamed demand or at a checkpoint. This is the same limit as
for requests in section 17, and it is the reason an operator or an adapter's own reconnect path has
to give the chain a chance to act rather than expecting the chain to notice on its own.
