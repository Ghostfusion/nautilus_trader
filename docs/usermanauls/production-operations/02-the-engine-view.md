# 02 - The engine view

This lecture names the parts of NautilusTrader that production operations touches. There is no code
here. Read it once so that the later lectures have something to attach to.

## The process

Two programs run the same strategy logic.

- The **backtest engine** (`nautilus_trader.backtest.BacktestEngine`) reads prices from memory or a
  file, advances a controlled clock, and executes orders on simulated venues. It is one process and
  it cannot be surprised by the network.
- The **live node** (`nautilus_trader.live.LiveNode`) connects to real venues and coordinates with
  systems outside the process: a data feed, an execution channel, a clock that is the real clock, and
  a cache that can be backed by a database. `docs/concepts/live.md` describes its lifecycle: restore
  cached state, connect data clients, connect execution clients, run reconciliation, start the
  strategies, then run an event loop until a stop request.

The **sandbox environment** sits between them. It is a live node with a real data feed and a
simulated matching engine in place of a venue connection, so no real order is ever sent. Lecture 05
runs one.

## The objects you will meet

| Object               | What it is                                                                    |
| -------------------- | ----------------------------------------------------------------------------- |
| `TraderId`           | The identity of the whole trading process, for example `OPS-001`.             |
| `StrategyId`         | The identity of one strategy instance inside that process.                    |
| `ClientId`           | The identity of one data or execution client.                                 |
| `InstrumentId`       | Venue and symbol together, for example `AUD/USD.SIM`.                         |
| `Venue`              | A named market, for example `SIM` in a backtest.                              |
| `Order`              | One instruction. It carries a side, a type, a quantity, a status, and a time. |
| `OrderDenied`        | The event the risk engine emits when it refuses an order.                     |
| `OrderFilled`        | The event emitted when an order trades, wholly or partly.                     |
| `Position`           | The net holding in one instrument for one strategy.                           |
| `AccountState`       | A venue report of balances and margins for one account.                       |
| `AccountBalance`     | `total`, `locked`, and `free` money in one currency.                          |
| `QueueStateChanged`  | An event saying a runner channel became backlogged or slow.                   |
| `SocketStateChanged` | An event saying a transport connected or disconnected.                        |
| `NotificationEvent`  | One operational alert carrying a class, a severity, and a message.            |
| `RiskCap`            | A Rust-only rule: a metric, a scope, a limit, and a window.                   |

## The data types

NautilusTrader uses exact decimal arithmetic for money and prices. The important types:

| Type        | Used for                                                                   |
| ----------- | -------------------------------------------------------------------------- |
| `Price`     | A price with a fixed number of decimal places set by the instrument.       |
| `Quantity`  | A size with a fixed number of decimal places set by the instrument.        |
| `Money`     | An amount with a currency, for example `100000.00 USD`.                    |
| `Decimal`   | The Python exact decimal type used where a value is not yet a `Money`.     |
| `QuoteTick` | A bid price and an ask price with their sizes and a timestamp.             |
| `TradeTick` | A traded price and size with a timestamp.                                  |
| `OrderBook` | The current resting bid and ask levels for one instrument.                 |
| `UnixNanos` | An integer count of nanoseconds since 1970-01-01, the engine's clock unit. |

Timestamps matter in every lecture. The engine stores them as integer nanoseconds, and every sample
data file in this manual uses that unit so that a row can be replayed exactly.

## The configuration objects

| Config                         | What it decides                                                                |
| ------------------------------ | ------------------------------------------------------------------------------ |
| `BacktestEngineConfig`         | Trader identity, logging, cache, risk engine, execution engine, state options. |
| `RiskEngineConfig`             | Rate limits, per-order notional caps, and (Rust only) count caps.              |
| `LiveRiskEngineConfig`         | The same surface for a live node, plus the live-only fields.                   |
| `LiveNodeConfig`               | Node-level options such as the queue monitor and shutdown on error.            |
| `LiveExecutionEngineConfig`    | Reconciliation lookback, snapshots, and continuous check intervals.            |
| `SandboxExecutionClientConfig` | Balances, fill model, fee model, and matching behavior for paper trading.      |
| `LoggerConfig`                 | The minimum log level for stdout and for files, and per-component overrides.   |
| `CacheConfig`                  | Encoding and flush behavior for a database-backed cache.                       |

## What Python can set, and what only Rust can set

This distinction decides the language of every sample in the manual, so learn it now.

Python can set, on `nautilus_trader.risk.RiskEngineConfig`:

- `bypass`
- `max_order_submit_rate`
- `max_order_modify_rate`
- `max_notional_per_order`
- `full_position_exit_venues`
- `debug`

Evidence: `crates/risk/src/python/config.rs`, which exposes exactly these constructor arguments. The
rate limits are strings of the form `limit/HH:MM:SS`, for example `2/00:00:01`.

Rust can additionally set `count_caps`, a list of `RiskCap` rules. There is no Python binding for
this field. Evidence: `crates/risk/src/engine/config.rs` declares `count_caps: Vec<RiskCap>`, and no
PyO3 constructor argument maps to it. A Python user cannot set a send, cancel, or fill count cap;
only a Rust caller can.

The same split appears elsewhere:

| Subsystem                                         | Surface   | Crate path                                               |
| ------------------------------------------------- | --------- | -------------------------------------------------------- |
| Backtest engine, live node, strategies, adapters  | Python    | `crates/backtest/src/python/`, `crates/live/src/python/` |
| Risk limits (`max_order_submit_rate` and friends) | Python    | `crates/risk/src/python/config.rs`                       |
| Pre-trade count caps (`count_caps`)               | Rust only | `crates/risk/src/engine/config.rs`                       |
| Notification router and sinks                     | Rust only | `crates/common/src/notification/`                        |

When a lecture reaches a Rust-only subsystem it says so, shows the crate path, and proves the
behavior with that crate's own test target. It never fakes a Python snippet for a Rust-only feature.

## The files this manual reads

| Path                                        | Why it matters here                                |
| ------------------------------------------- | -------------------------------------------------- |
| `docs/concepts/live.md`                     | The live node lifecycle and dispatch model.        |
| `docs/concepts/execution/index.md`          | Pre-trade risk checks and the four cap dimensions. |
| `docs/concepts/notifications.md`            | The alert event set, the queue, the drop counter.  |
| `docs/concepts/event_sourcing.md`           | The durable record and recovery sealing.           |
| `docs/concepts/accounting.md`               | Balances, locking, and who owns the numbers.       |
| `docs/concepts/execution/reconciliation.md` | Startup and continuous reconciliation.             |
| `docs/concepts/logging.md`                  | Where the operational log lines come from.         |
| `docs/concepts/networking.md`               | Heartbeats, idle timeouts, and reconnect.          |
| `docs/concepts/cache.md`                    | What can be saved and what cannot.                 |
| `docs/how_to/configure_live_trading.md`     | Wiring a live node, including the cache backing.   |

## The shape of an operational run

Read the four stories in lecture 01 again against this list. Every control is a stop in one of these
phases.

1. **Startup.** Load cached state, connect clients, reconcile against the venue, then start
   strategies. `docs/concepts/live.md` gives the exact order.
2. **Steady state.** The runner dispatches messages from seven internal channels in a fixed
   priority. The channels are unbounded, so the node does not slow a feed down when it falls behind;
   the queue monitor turns that pressure into an event the application can act on.
3. **Failure.** A transport can be lost, a submission outcome can become unknown, and a process can
   be killed. Reconciliation and the event store exist for this phase.
4. **Shutdown.** State is saved, the event store run is sealed, clients disconnect, and the process
   exits.

Continue to [03 - First run](03-first-run.md).