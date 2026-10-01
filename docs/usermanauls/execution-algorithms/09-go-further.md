# 09 - Go further

This lecture is honest about what this manual did not cover, points at the concept pages that explain
the engine in depth, and suggests what to learn next.

## Honest gaps

- **Only TWAP is documented on the concept page.** `docs/concepts/execution/algorithms.md` documents
  TWAP and custom algorithms; iceberg, quote-pegged, and sniper are shipped in
  `crates/trading/src/algorithm/` but are described there only by their source. This manual described
  all four from the source, and cited the lines, but the source remains the authority.
- **There is no VWAP algorithm.** The trait documentation mentions "order slicing algorithms like
  TWAP and VWAP" (`crates/trading/src/algorithm/mod.rs:91`), but the four native algorithms are TWAP,
  iceberg, quote-pegged, and sniper. VWAP slicing would be a custom algorithm.
- **Every shipped algorithm refuses most policy parts.** None of the four honours
  `participation_rate`, and none honours `max_slippage_bps` or `urgency`. The horizon is the only part
  all four honour. A strategy that declares these constraints is denied, by design.
- **The analytics subsystem is Rust only.** There is no Python binding for implementation shortfall,
  spread capture, adverse selection, or the rest of the metrics list. This manual measured what
  Python can measure and showed how to run the Rust tests, but it did not build a Rust consumer of
  the analytics API.
- **Count caps are Rust only.** The pre-trade count caps in `crates/risk/src/config.rs` cannot be set
  from Python in this version.
- **The sample data is synthetic.** It is a deterministic sine wave, not a market recording. A real
  study needs recorded data from a venue adapter or a data catalog.
- **Quote-pegged was not run here.** Its behaviour was described from `quote_pegged.rs` and cited, but
  no program was run for it in this manual. It needs a cached quote before it can price a child.
- **Custom execution algorithms were not built.** Python can define one by subclassing
  `ExecutionAlgorithm` and implementing `on_order`, and the engine can register it with
  `add_exec_algorithm`. That is a good next project; see the concept page.

## The concept pages to read next

| Page                                                                               | Why read it                                                 |
| ---------------------------------------------------------------------------------- | ----------------------------------------------------------- |
| [Execution](../../concepts/execution/index.md)                                     | The components, routing, risk checks, and denial reasons.   |
| [Execution algorithms](../../concepts/execution/algorithms.md)                     | TWAP, custom algorithms, spawned orders, and cache queries. |
| [Execution policies](../../concepts/execution/policies.md)                         | Order state, command outcomes, and retry limits.            |
| [Orders](../../concepts/orders/index.md)                                           | Order types, statuses, and the primary state flow.          |
| [Trade-based execution](../../concepts/backtesting/trade-execution.md)             | How a fill is produced from trades and queue position.      |
| [Fill prices and matching](../../concepts/backtesting/fill-prices-and-matching.md) | Fill prices per order type and book type.                   |
| [Execution reconciliation](../../concepts/execution/reconciliation.md)             | Live recovery and reconciliation invariants.                |

## Go deeper in the code

- `crates/trading/src/algorithm/mod.rs` is the `ExecutionAlgorithm` trait: spawn methods, the
  quantity-restore machinery, and the event dispatch.
- `crates/trading/src/algorithm/core.rs` holds `ExecutionAlgorithmCore` and the spawn counters.
- `crates/trading/src/analytics/` is the read-only metrics subsystem, documented in its own module.
- `crates/backtest/tests/integration/backtest_engine.rs` has a Rust test of TWAP routing
  (`test_twap_exec_algorithm_routes_and_slices_order`) that asserts the parent/child relationship and
  the quantity conservation rule.
- `examples/backtest/` has complete backtest programs to adapt.

## Suggested next steps

1. Write a custom execution algorithm in Python that honours a `participation_rate` by pacing slices
   against observed volume, and register it with `add_exec_algorithm`.
2. Write a small Rust binary or test that feeds an `ExecutionObserver` and prints the full metric set,
   so you can see implementation shortfall and spread capture computed by the engine rather than by
   hand.
3. Re-run this manual's TWAP program over recorded data from a venue adapter, with a real fee model,
   and compare the shortfall with the synthetic-data number.
4. Read the [market-making](../market-making/README.md) manual for the other side of the coin: resting
   orders, inventory, and maker fees.
5. Read the [microstructure-signals](../microstructure-signals/README.md) manual for reading the order
   book shape that an execution algorithm must work inside.

## Final reminder

Execution algorithms protect an edge; they do not create one. The number to watch is the
implementation shortfall against a reference you trust. Report it in money and in basis points, keep
the benchmark with the number, and never compare a shortfall computed against one reference with a
shortfall computed against another.
