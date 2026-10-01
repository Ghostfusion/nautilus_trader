# 02 - The engine view

This lecture describes how the repository models the style, before any code. Everything named here
can be found in the paths cited. The only thing Rust-specific is called out as such.

## The domain objects

The engine is an event-driven system. A strategy receives events, decides something, and sends
commands. For this style the events are market data and the commands are orders.

| Object           | What it is                                           | Where it lives                   |
| ---------------- | ---------------------------------------------------- | -------------------------------- |
| `InstrumentId`   | The identity of an instrument, such as `USD/JPY.SIM` | `crates/model/src/identifiers`   |
| `Bar`            | One interval of open, high, low, close and volume    | `crates/model/src/data/bar.rs`   |
| `BarType`        | The full description of a bar series                 | `crates/model/src/data/bar.rs`   |
| `QuoteTick`      | A bid and an ask with sizes                          | `crates/model/src/data/quote.rs` |
| `Order`          | An instruction to trade, with a status               | `crates/model/src/orders`        |
| `Position`       | The net holding in one instrument                    | `crates/model/src/position.rs`   |
| `Strategy`       | Your code, which receives events and sends orders    | `crates/trading/src/strategy`    |
| `Portfolio`      | Gains and losses across all positions                | `crates/portfolio`               |
| `Cache`          | The in-memory record of what the engine has seen     | `crates/common/src/cache`        |
| `BacktestEngine` | Replays recorded data through a simulated venue      | `crates/backtest`                |

The Python classes are exposed with the same names, documented in
`python/nautilus_trader/*/__init__.pyi`. For example `Bar`, `BarType` and `QuoteTick` are importable
from `nautilus_trader.model`.

## The bar type string

A `BarType` is written as a single string and decodes into four parts. The one used in this manual
is:

```text
USD/JPY.SIM-1-MINUTE-MID-INTERNAL
```

Read it left to right:

1. `USD/JPY.SIM` is the instrument and venue.
2. `1-MINUTE` is the aggregation, one bar per minute.
3. `MID` is the price type: the midpoint between bid and ask.
4. `INTERNAL` means the engine builds the bar itself from ticks.

`EXTERNAL` instead means the bars arrive already built, from a venue or a file. The distinction
matters for execution: only external bars update the matching engine's book
(`docs/concepts/backtesting/bar-execution.md`).

## How a bar series comes to exist

There are two paths, and both end in the same `Bar` object.

1. Aggregation. The strategy subscribes to a bar type whose source is `INTERNAL`. The data engine
   builds bars from quote or trade ticks. The machinery is in `crates/data/src/aggregation.rs`,
   and the time-bar logic is what closes each bar at its interval boundary.
2. Loading. Bars already exist as `Bar` objects, either committed as a file or read from the
   Parquet catalog. They are handed to the engine with `add_data`, and the strategy subscribes to
   the matching bar type (`docs/concepts/data/catalog.md`).

## The config objects

Configuration in this repository is an explicit object rather than a loose set of arguments. The
ones this style touches:

| Config                 | Controls                                                 | Source                                                       |
| ---------------------- | -------------------------------------------------------- | ------------------------------------------------------------ |
| `BacktestEngineConfig` | Trader id, logging, risk engine, data engine             | `crates/backtest/src/config.rs`                              |
| `StrategyConfig`       | Base class for a strategy's own config                   | `crates/trading/src/strategy/config.rs`                      |
| `RiskEngineConfig`     | Pre-trade checks: submit rate, modify rate, notional cap | `python/nautilus_trader/risk/__init__.pyi`                   |
| `EmaCrossConfig`       | The builtin quote-driven EMA cross                       | `crates/trading/src/examples/strategies/ema_cross/config.rs` |

A venue is added with `engine.add_venue(...)`, which takes the account type, the order management
system type, the starting balances, and the fee, fill and slippage models. The full argument list
is in `python/nautilus_trader/backtest/__init__.pyi`.

## The fee and fill models

A backtest is only as honest as its cost model.

- `MakerTakerFeeModel(maker_rate, taker_rate)` charges a fraction of notional per fill. A taker is
  the side that crosses the spread; a market order is a taker.
- `ProbabilisticFillModel` and `ProbabilisticSlippageModel` make fills and slippage probabilistic
  for strategies that rest orders in the book.
- `DefaultFillModel` fills at the book.

All of these are Python-exposed (`crates/execution/src/python/`). For a market-order rule the
important one is the fee model, because every entry and exit is a taker fill.

## The two order management systems

`OmsType.NETTING` keeps one position per instrument, so a buy after a sell reduces or flips the
single net position. `OmsType.HEDGING` keeps long and short positions separately. This manual uses
`NETTING`, which is what the examples use. See `docs/concepts/positions.md` for the difference.

## What is Python and what is Rust only

The rule the manual follows: if there is no PyO3 binding, the manual says so and does not show
Python.

| Subsystem                                                                  | Surface                                                                         |
| -------------------------------------------------------------------------- | ------------------------------------------------------------------------------- |
| Backtest engine, live node, strategies, adapters                           | Python (`crates/backtest/src/python/`, `crates/live/src/python/`)               |
| Instruments, orders, bar and tick data, order books, analysis, persistence | Python (`crates/model/src/python/`, `crates/persistence/src/python/`)           |
| Fill, slippage, fee and market impact models                               | Python (`crates/execution/src/python/`)                                         |
| Risk config: submit rate, modify rate, notional cap, and order sizing      | Python (`crates/risk/src/python/config.rs`)                                     |
| Portfolio statistics and analyzer                                          | Python (`crates/analysis/src/python/`)                                          |
| The pre-trade send, cancel and fill count caps                             | Rust only (`crates/risk/src/engine/config.rs`, `crates/risk/src/engine/cap.rs`) |
| Factor pipeline, membership, panel and dataset                             | Rust only (`crates/research`)                                                   |
| Execution analytics such as implementation shortfall                       | Rust only (`crates/trading/src/analytics`)                                      |
| Option pricing and volatility surfaces                                     | Rust only (`crates/model/src/data/`)                                            |

Two consequences for this style. First, the per-message count caps are not settable from Python;
you configure `max_order_submit_rate`, `max_order_modify_rate` and `max_notional_per_order`, but the
hard cap on send, cancel and fill counts lives in Rust. Second, execution analytics that measure
implementation shortfall are Rust only, so this manual does not show Python for them.

## Why session time matters

A minute bar is one minute of exchange time, not one minute of your local clock. The engine resolves
sessions through a trading calendar in the exchange's time zone, so an equity session opens at the
same local time whether or not daylight saving is in force (`docs/concepts/trading_calendars.md`).
A calendar also supplies holidays and early closes, and it never extends an instrument's lifetime.

Backtests are also required to be reproducible. The engine's deterministic simulation contract
(`docs/concepts/dst.md`) explains which sources of nondeterminism are controlled and which are not;
for this manual it is enough to know that the same data and the same configuration produce the same
result.

Next: the smallest complete program, in [03-first-run](03-first-run.md).
