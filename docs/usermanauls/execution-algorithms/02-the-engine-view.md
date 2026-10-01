# 02 - The engine view

This lecture maps the words from lecture 01 onto the objects in this repository. There is no code
here; the goal is that when you meet `exec_algorithm_id` in lecture 03 you already know what it is.

## The components

The execution model is described in [Execution](../../concepts/execution/index.md). A **strategy**
builds orders and sends commands. When an order carries an `exec_algorithm_id`, the command is routed
to an **execution algorithm** instead of straight to the risk engine. The route is documented as:

```text
Strategy -> OrderEmulator or ExecutionAlgorithm or RiskEngine
ExecutionAlgorithm -> RiskEngine -> ExecutionEngine -> ExecutionClient
```

An **execution algorithm** is a special `DataActor`: it can subscribe to data, set timers, and place
orders. It is defined by the `ExecutionAlgorithm` trait in `crates/trading/src/algorithm/mod.rs:112`.
NautilusTrader ships four native algorithms in `crates/trading/src/algorithm/`: TWAP, iceberg,
quote-pegged, and sniper.

## Intent and policy

When an algorithm receives an order it reads two things.

The **intent** is what you want executed. It is the `ExecutionIntent` struct at
`crates/trading/src/algorithm/policy.rs:151`, with fields `instrument_id`, `order_side`,
`order_type`, `quantity`, and an optional `price`. It is built from the parent order itself.

The **policy** is the constraints you declared. It is the `ExecutionPolicy` struct at
`crates/trading/src/algorithm/policy.rs:178`. Every field is optional; an absent field is a
constraint you did not declare.

| Policy field         | Parameter key        | Meaning                                                 |
| -------------------- | -------------------- | ------------------------------------------------------- |
| `horizon`            | `horizon_secs`       | Seconds to complete the execution in.                   |
| `participation_rate` | `participation_rate` | Fraction of market volume to be, above 0 and at most 1. |
| `price_limit`        | `price_limit`        | A price no child may trade outside.                     |
| `max_slippage_bps`   | `max_slippage_bps`   | Maximum slippage against the reference, in bps.         |
| `preference`         | `preference`         | `passive` (rest) or `aggressive` (cross).               |
| `urgency`            | `urgency`            | `low`, `normal`, or `high`.                             |

The key names are constants in `crates/trading/src/algorithm/policy.rs:43-54`. Each field is one
`PolicyPart` variant, listed at `crates/trading/src/algorithm/policy.rs:60`. The policy is parsed
from the order's parameters by `ExecutionPolicy::from_params` at
`crates/trading/src/algorithm/policy.rs:203`. A malformed or out-of-domain value is refused by name,
not defaulted.

## The rule that makes this safe

An algorithm declares the policy parts it can honour through `supported_policy_parts`, and any
declared part it cannot honour is **refused**, not ignored. The trait's default is to honour nothing
(`crates/trading/src/algorithm/mod.rs:126`), so an algorithm that cannot satisfy your constraint will
tell you so instead of silently ignoring it. The refusal names the parameter key, so you always know
which constraint was rejected.

## Parent and spawned orders

An order an algorithm receives is the **primary order** (also called the parent). Orders the
algorithm creates from it are **spawned orders** (children). Two fields identify them:

- `exec_spawn_id` on a spawned order is the primary order's `client_order_id`. On the primary itself
  the field equals its own `client_order_id`.
- A spawned order's own `client_order_id` follows `{exec_spawn_id}-E{sequence}`, for example
  `O-20230404-001-000-1-E1`.

Spawned orders are created with `spawn_market`, `spawn_limit`, or `spawn_market_to_limit`
(`crates/trading/src/algorithm/mod.rs:475,544,625`). By default the primary quantity is reduced by
the spawned quantity as each child is created; the flag `reduce_primary=False` keeps it unchanged.

The parent and child terminology is documented in
[Execution algorithms](../../concepts/execution/algorithms.md#spawned-orders). The quantity
conservation rule (what happens when a child is denied or cancelled) is in
[07-risks-and-limits.md](07-risks-and-limits.md).

## The four shipped algorithms

All four are native Rust. TWAP is documented on the concept page; the other three are described here
from their source.

| Algorithm    | Order type | Required parameters             | Honours                                  | Refuses                                          |
| ------------ | ---------- | ------------------------------- | ---------------------------------------- | ------------------------------------------------ |
| TWAP         | `MARKET`   | `horizon_secs`, `interval_secs` | horizon                                  | everything else                                  |
| Iceberg      | `LIMIT`    | `display_size`, `requote_secs`  | horizon, price limit, passive preference | everything else, aggressive preference           |
| Quote-pegged | `LIMIT`    | `pegging`, `requote_secs`       | horizon, price limit, passive preference | everything else, aggressive preference           |
| Sniper       | `LIMIT`    | `limit_price`, `max_children`   | horizon, aggressive preference           | everything else, passive preference, price limit |

Details and citations:

- **TWAP** (`crates/trading/src/algorithm/twap.rs`). Market orders only (`twap.rs:141`). Divides the
  quantity into `floor(horizon_secs / interval_secs)` slices, submitting the first immediately and
  the rest on a timer (`twap.rs:274`, `twap.rs:397`). It supports exactly one policy part, the
  horizon (`twap.rs:72`).
- **Iceberg** (`crates/trading/src/algorithm/iceberg.rs`). Limit orders only (`iceberg.rs:353`). Keeps
  one child working at a time, quoting `min(remaining, display_size)` (`iceberg.rs:176`), and
  re-quotes a child the market has crossed once `requote_secs` has elapsed (`iceberg.rs:14-18`). A
  passive preference is what it already does; an aggressive preference is refused
  (`iceberg.rs:400`). A declared price limit is checked against the order's own price
  (`iceberg.rs:409`).
- **Quote-pegged** (`crates/trading/src/algorithm/quote_pegged.rs`). Limit orders only
  (`quote_pegged.rs:419`). Keeps one child resting and re-quotes it when the touch moves and the
  re-quote interval has elapsed (`quote_pegged.rs:20-34`). The `pegging` value is `passive` (join the
  near touch) or `join` (one increment inside the opposite touch) (`quote_pegged.rs:29`). It needs a
  cached quote to price the child (`quote_pegged.rs:476`).
- **Sniper** (`crates/trading/src/algorithm/sniper.rs`). Limit orders only (`sniper.rs:368`). Sweeps
  displayed liquidity at a fixed `limit_price`, one slice at a time, only when the touch is reachable
  (`sniper.rs:18-26`). `max_children` is a hard bound on the number of children. An aggressive
  preference is what it does; a passive preference is refused (`sniper.rs:424`), and a declared
  `price_limit` is refused by name because the sweep price is its own parameter (`sniper.rs:403`).

## Configuration objects

The Python class `ExecutionAlgorithmConfig` (`python/nautilus_trader/trading/__init__.pyi:205`) has
three fields: `exec_algorithm_id`, `log_events`, and `log_commands`. `exec_algorithm_id` is required
when the config is used to register an algorithm on a backtest engine; the register functions check
it and raise otherwise, for example `crates/backtest/src/python/engine.rs:1212`.

## Where the code lives

| Concern                                       | Path                                                                      |
| --------------------------------------------- | ------------------------------------------------------------------------- |
| Execution algorithm trait and spawn machinery | `crates/trading/src/algorithm/mod.rs`                                     |
| TWAP, iceberg, quote-pegged, sniper           | `crates/trading/src/algorithm/*.rs`                                       |
| Policy parts and parsing                      | `crates/trading/src/algorithm/policy.rs`                                  |
| Execution analytics (metrics, Rust only)      | `crates/trading/src/analytics/`                                           |
| Concept page                                  | `docs/concepts/execution/algorithms.md`                                   |
| Policies and order-denied reasons             | `docs/concepts/execution/policies.md`, `docs/concepts/execution/index.md` |
| Backtest engine Python binding                | `crates/backtest/src/python/engine.rs:510`                                |

Python can configure the native algorithms and can host custom algorithms, but the analytics
subsystem in `crates/trading/src/analytics/` has no Python bindings. That distinction is the subject
of [06-measure-and-evaluate.md](06-measure-and-evaluate.md).

Read [03-first-run.md](03-first-run.md) to run TWAP.
