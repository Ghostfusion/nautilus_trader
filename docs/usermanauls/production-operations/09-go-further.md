# 09 - Go further

You have configured a risk engine, watched it refuse a runaway loop, read the count-cap
code, run the crate's own tests, traded in the sandbox against a live feed, and worked through the
state and reconciliation contracts. This lecture says what the manual did not cover, and where to go
next.

## Honest gaps in this manual

Read these before you trust anything you built from this manual.

| Gap                                                             | What it means for you                                                                                                                                                                                           |
| --------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| No notification sink is demonstrated from Python                | The notification subsystem is Rust only. A Python user cannot register a sink or read `drop_count`; that requires a Rust-native node.                                                                           |
| The default transports are plaintext                            | No TLS transport is demonstrated here. A deployment that needs TLS implements a custom `NotificationTransport`.                                                                                                 |
| No database backing was run                                     | Redis and Postgres were not available where this manual was written. The `on_save`/`on_load` contract was proven by direct call, and the no-backing behavior was observed, but a real restart was not executed. |
| The sandbox demonstration is not byte-reproducible              | It reads a live public feed, so prices and fills differ on every run. The listed output is one observed run.                                                                                                    |
| Event-store replay was not run                                  | `replay_from_run_id`, recovery sealing, and cache replay are described from `docs/concepts/event_sourcing.md` and the crate source.                                                                             |
| The `nautilus-common` lib test target does not build on Windows | Unused imports are treated as errors in this checkout. The notification tests were therefore run through the integration test binary, which builds and passes.                                                  |
| No live venue was contacted                                     | Every command in this manual ran in a backtest or in the sandbox. No real order was ever sent.                                                                                                                  |
| The measured cap configuration is not advice                    | The 20,000 / 10,000 / 10,000 / 2,000 / 50 table is the benchmark's input, not a recommendation. Derive your own values.                                                                                         |

## The concept documents behind every claim

Each of these is a page in this repository. Read the first three whatever you do next.

| Document                                    | What it answers                                                          |
| ------------------------------------------- | ------------------------------------------------------------------------ |
| `docs/concepts/live.md`                     | Live node lifecycle, dispatch priority, queue monitor, socket state.     |
| `docs/concepts/execution/index.md`          | Pre-trade checks, account selection, count caps, whole-position exits.   |
| `docs/concepts/execution/reconciliation.md` | Every reconciliation scenario, invariant, and failure path.              |
| `docs/concepts/notifications.md`            | The alert event set, the router, the queue, the drop policy, transports. |
| `docs/concepts/event_sourcing.md`           | The durable record, recovery sealing, and replay.                        |
| `docs/concepts/accounting.md`               | Balances, locking, margin scopes, and who owns the numbers.              |
| `docs/concepts/logging.md`                  | Log levels, rotation, and component filtering.                           |
| `docs/concepts/networking.md`               | Heartbeats, idle timeouts, reconnect throttling, and state reporting.    |
| `docs/concepts/cache.md`                    | What the cache stores and what state persistence needs.                  |
| `docs/concepts/orders/index.md`             | Order types, statuses, and the state machine.                            |
| `docs/concepts/positions.md`                | Position lifecycle and snapshotting.                                     |
| `docs/concepts/portfolio.md`                | Equity, mark-to-market, and PnL authority.                               |
| `docs/concepts/reports.md`                  | The report tables and every column.                                      |
| `docs/concepts/configuration.md`            | How configuration objects handle defaults and builders.                  |
| `docs/how_to/configure_live_trading.md`     | Building a live node, including the cache database backing.              |

## The source paths behind the Rust-only claims

| Path                               | Why you would open it                                          |
| ---------------------------------- | -------------------------------------------------------------- |
| `crates/risk/src/engine/config.rs` | The `count_caps` field and what a cap predicate holds.         |
| `crates/risk/src/engine/cap.rs`    | Cap evaluation, the counter keys, and the refusal record.      |
| `crates/model/src/risk.rs`         | `RiskCapScope`, `RiskCapMetric`, and `RiskRequestKey`.         |
| `crates/risk/src/python/config.rs` | The exact Python surface of the risk engine.                   |
| `crates/common/src/notification/`  | The event set, the sink queue, the transports, and the router. |
| `crates/adapters/sandbox/src/`     | The sandbox execution client and its matching-engine wiring.   |
| `crates/execution/src/models/`     | Fill, fee, latency, slippage, and market impact models.        |

## What to learn next

1. **Backtesting properly.** Return to `docs/concepts/backtesting/` and learn the fill models, the
   venue configuration, and why a backtest with a zero-fee model is optimistic. Everything you
   configured in lecture 03 works the same way in a larger backtest.
2. **Adapters.** Read `docs/concepts/adapters.md` and the integration guide for the venue you intend
   to use. The adapter decides which order types exist and which reconciliation reports are
   available, so it decides half of your operational design.
3. **Execution algorithms.** Read `docs/concepts/execution/algorithms.md`. Slicing a large order
   changes the rate-limit arithmetic and the reconciliation surface.
4. **The message bus.** Read `docs/concepts/message_bus.md` to understand where the events in
   lecture 06 come from and how an external system could consume them.
5. **Logging and observability.** Read `docs/concepts/logging.md` and configure file logging with
   rotation before you run anything overnight.

## A closing checklist

Before a strategy trades real money, every line below should have a yes.

- A risk engine configuration exists, with a rate limit and a per-order notional cap you chose.
- A count cap exists if you configure it, derived from your venue's message limits.
- Denials are logged and someone reads them.
- A transport-state signal exists and reaches a human.
- State is saved and loaded with a database backing, and you have tested a restart.
- The event store is enabled and its retention is planned.
- Reconciliation is enabled and its lookback is set deliberately, not left to chance.
- The strategy has run in the sandbox and its shutdown behavior has been observed.
- The alert path has been proven with `sent_count` rising, and `drop_count` is monitored.
- No credential appears in any notification body, subject, or URL.
- `bypass` is `False`.

Return to the [course index](README.md).