# Data provider failover design: a priority chain across market-data feeds

**Status:** design only. No code was written.

The request this document answers is to run data providers as **one chain rather than side by
side**: a priority list where the highest-priority provider that can serve a demand serves it, and a
failure falls through to the next provider. The intended order is EODHD first, moomoo second, with
further providers ordered by data coverage and rate limits.

This document records what the platform can and cannot express today, where the chain would have to
live, the preconditions that make it work at all, and the failure modes that make a naive version
dangerous.

| Revision | Change                                                                                                                                |
| -------- | ------------------------------------------------------------------------------------------------------------------------------------- |
| 1        | Initial design from the platform routing and adapter sources, with the EODHD and moomoo limits read from code and from the live probe |

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
definition.

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

## 8. Failure classification: which events cause a hop

"Fails for any reason" is not implementable as written. The repository already has the right shape
for this problem in the execution path, where a command outcome is classified as a definite local
failure, a definite venue rejection, or an ambiguous outcome, rather than collapsed into one error.
The chain needs the same discipline for data.

| Event                                                   | Hop?                                   | Reason                                                                                            |
| ------------------------------------------------------- | -------------------------------------- | ------------------------------------------------------------------------------------------------- |
| Connection refused (OpenD down, DNS failure)            | Yes                                    | The provider cannot serve anything                                                                |
| Request timeout                                         | Yes, after one retry                   | The provider did not answer                                                                       |
| HTTP 5xx, transport reset                               | Yes                                    | Provider-side failure                                                                             |
| HTTP 429 / documented rate limit                        | Yes                                    | The provider is refusing load, not answering                                                      |
| Entitlement refusal (a purchased capability is missing) | Yes, and recorded                      | A capability gap is a data-availability gap, but it must be reported, not hidden                  |
| Response parse failure                                  | Yes, and recorded                      | The provider's payload changed; this is a defect to surface                                       |
| Empty result                                            | **No**                                 | The provider answered; the data does not exist. Hopping would silently substitute another dataset |
| Instrument not found                                    | **No**                                 | Same as above                                                                                     |
| Invalid input from the caller                           | **No**                                 | Not the provider's fault; hopping hides a caller bug                                              |
| Ambiguous (request sent, no answer)                     | Retry the same provider once, then hop | Mirrors the execution ambiguity policy                                                            |

Two of these lines carry most of the risk. **Empty results must never trigger a hop**, or a symbol
that legitimately has no data on the primary would quietly be served from a different source with
different semantics. And **entitlement refusals must be recorded**, or a plan downgrade turns into
an unexplained change of data quality.

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

## 14. Open decisions

1. **Where the chain lives.** This document chooses a composite client with no core change. A
   platform-level priority list is the cleaner long-term home if the platform adopts it.
2. **Hop on entitlement refusal.** Recommended yes, with the refusal recorded; the alternative is to
   treat a capability gap as a hard failure.
3. **Fail-back policy.** Staying on the secondary until a checkpoint is recommended; immediate
   fail-back causes thrashing against moomoo's one-minute release minimum.
4. **Authoritative provider for instrument definitions.** Must be exactly one.
5. **Whether a backtest may hop at all.** Recommended no by default.
6. **Maximum hops** per demand, to bound latency in the worst case.
