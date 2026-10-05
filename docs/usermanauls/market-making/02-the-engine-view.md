# 02 - The engine view

This lecture maps the market making ideas from lecture `01` onto the objects this repository
actually uses. There is no program to run here. Read it once, then run lecture `03`.

## The pieces of a market making system

A market making system needs five things, and this repository names each one:

| Need                       | Repository object                           | Where it lives                      |
| -------------------------- | ------------------------------------------- | ----------------------------------- |
| A stream of prices         | `QuoteTick`, `TradeTick`, `Bar`             | `nautilus_trader.model`             |
| A book to reason about     | `OrderBook`, `OrderBookDeltas`              | `nautilus_trader.model`             |
| Something to trade         | An instrument from `TestInstrumentProvider` | `nautilus_trader.testkit.providers` |
| Orders to post             | `LimitOrder` with `post_only`               | `nautilus_trader.model`             |
| Rules for how fills happen | A fill model and a fee model                | `nautilus_trader.execution`         |

The engine that ties them together is `BacktestEngine` (`nautilus_trader.backtest`), configured by
`BacktestEngineConfig` (`nautilus_trader.config`).

## Market data

A **quote** in this repository is a `QuoteTick`: two prices and two sizes at one instant.

- `bid_price` and `bid_size` are the best price and size on the buy side.
- `ask_price` and `ask_size` are the best price and size on the sell side.
- `ts_event` is when the venue produced the data, in integer nanoseconds since the Unix epoch.

A **trade** is a `TradeTick`: a price, a size, an `aggressor_side` (who crossed the spread), and a
`trade_id`. A **bar** is a `Bar`: an open, high, low, close, and volume over a time interval.

Market makers usually work from quotes, because the bid and the ask are exactly what they need to
place a two-sided quote. The deterministic generators in
`python/nautilus_trader/testkit/providers.py:653` (`usdjpy_quotes`) and `:680` (`audusd_quotes`)
return `QuoteTick` lists shaped like a sine wave, which is what every runnable example in this
course uses.

## The order book

An `OrderBook` holds the public depth for one instrument. This repository implements three book
types (`docs/concepts/order_book.md`):

- `L1_MBP`: Level 1 market-by-price. Only the best bid and best ask (the BBO). Quote ticks, trade
  ticks, and bars can drive an `L1_MBP` book.
- `L2_MBP`: Level 2 market-by-price. Aggregated quantity at each price level.
- `L3_MBO`: Level 3 market-by-order. Every individual order, keyed by order ID.

Your strategy can subscribe to the book in three shapes (`docs/concepts/order_book.md`):

- `subscribe_book_deltas(instrument_id, BookType.L2_MBP)` for incremental updates into
  `on_book_deltas`.
- `subscribe_book_depth(instrument_id, BookType.L2_MBP, managed=False)` for depth snapshots into
  `on_book_depth`.
- `subscribe_book_at_interval(instrument_id, BookType.L2_MBP, interval_ms=1000)` for a full book
  every interval into `on_book`.

The one cached book per instrument is shared. A detail that matters for market making: an
`OwnOrderBook` tracks your own working orders separately, so you can compute the liquidity that is
actually available after subtracting your own quotes (`docs/concepts/order_book.md`).

## Orders

A **limit order** states a price and a quantity. `post_only` is the market making flag: the venue
accepts the order only if it rests, and cancels or rejects it if it would cross the spread. All
grid orders posted by the shipped strategy set `post_only=true` (`crates/trading/src/examples/strategies/grid_mm/strategy.rs`).

Two more order features appear in market making:

- **Iceberg**: the order carries a `display_qty` smaller than its total quantity, so only part of it
  is visible on the book (`docs/concepts/orders/index.md`).
- **Time in force**: `GTC` means "good until canceled" and rests forever. `GTD` means "good until a
  date" and expires. The `expire_time_secs` config field turns the grid orders into `GTD` orders.

Order types, instructions, and their state transitions are documented in
`docs/concepts/orders/index.md`.

## Fill models: how the engine decides what fills

A backtest never sees the real counterparties to your order. A **fill model** decides whether your
limit order would have filled (`docs/concepts/backtesting/fill-models.md`). The eleven shipped
models are:

| Model                        | Behavior                                                |
| ---------------------------- | ------------------------------------------------------- |
| `DefaultFillModel`           | Uses the recorded book as-is.                           |
| `BestPriceFillModel`         | Unlimited size at the best bid and ask.                 |
| `OneTickSlippageFillModel`   | Unlimited size one tick beyond the best price.          |
| `ProbabilisticFillModel`     | Chooses the best price or one tick worse at random.     |
| `TwoTierFillModel`           | 10 units at best, the rest one tick worse.              |
| `ThreeTierFillModel`         | 50, 30, and 20 units across three levels.               |
| `LimitOrderPartialFillModel` | 5 units at best, the rest one tick worse.               |
| `SizeAwareFillModel`         | Changes the book shape at an order size of 10 units.    |
| `CompetitionAwareFillModel`  | Exposes a configurable fraction of 1,000 units at best. |
| `VolumeSensitiveFillModel`   | Places 25 percent of its internal volume at best.       |
| `MarketHoursFillModel`       | Normal or one-tick-wider synthetic spread.              |

The tier sizes are instrument quantity units, so check them against the scale of the instrument you
trade (`docs/concepts/backtesting/fill-models.md`).

Four fill-model settings matter most for a market maker:

- `prob_fill_on_limit`: the chance a touched limit order fills. `1.0` fills on every touch.
- `prob_slippage`: the chance an L1 fill moves one tick against you.
- `queue_position` (default `false`): turns on resting-order queue tracking, so a passive fill waits
  for the depth ahead of it to trade through. Lecture `05` runs this on and off.
- `liquidity_consumption` (default `false`): when on, a filled order consumes displayed size so the
  same size cannot fill a second order.

A fill model that is not set falls back to `DefaultFillModel` with `prob_fill_on_limit=1.0` and
`prob_slippage=0.0`.

## The fee model

`MakerTakerFeeModel(maker_rate=..., taker_rate=...)` charges a fraction of the traded value, split
by whether the fill was a maker or a taker. The example uses `0.00002` on both sides
(`examples/backtest/fx_market_maker_gbpusd_bars.py`), which is `0.002%` per side.

The commission is charged in the instrument's cost currency. For USD/JPY that is JPY, which is why
the reports in lecture `06` show commissions like `[1095 JPY]`.

## Venue configuration

`engine.add_venue(...)` takes the settings that define a simulated exchange:

- `oms_type`: the order management system. `OmsType.NETTING` keeps one net position per instrument
  per strategy. `OmsType.HEDGING` allows several positions at once. Market making usually wants
  `NETTING`, because the grid's buys and sells are meant to offset each other into one inventory.
- `account_type`: `MARGIN` lets you hold short positions and use leverage; `CASH` does not.
- `base_currency`: the currency of the account.
- `starting_balances`: a list of `Money`, the simulated starting cash.
- `book_type`: the `BookType` used for the venue, `L1_MBP` by default.
- `fill_model`, `fee_model`, `latency_model`, `slippage_model`: the models described above.

`docs/concepts/backtesting/data-and-venues.md` covers the rest of the venue surface.

## Inventory and positions

A **position** records your exposure to one instrument during an open-close cycle
(`docs/concepts/positions.md`). Under `NETTING` OMS the position ID is deterministic,
`{instrument_id}-{strategy_id}`. The position tracks `signed_qty` (positive is long, negative is
short, zero is flat), the average entry and exit prices, `realized_pnl`, and commissions.

The `Portfolio` (`docs/concepts/portfolio.md`) aggregates positions and accounts. Market making code
uses it to ask "how big is my inventory right now?" before quoting. The grid strategy in this
repository reads open positions directly from the cache and computes the net position and the
worst-case per-side exposure before placing each level
(`crates/trading/src/examples/strategies/grid_mm/strategy.rs`).

The report columns and PnL accounting rules are documented in `docs/concepts/reports.md` and
`docs/concepts/accounting.md`.

## The shipped grid market maker

`GridMarketMaker` is registered by name in the backtest engine's builtin strategy list. That list is
built under the `examples` feature: `GridMarketMaker`, `CompositeMarketMaker`, `DeltaNeutralVol`,
`EmaCross`, and `HurstVpinDirectional` (`crates/backtest/src/python/engine.rs:1186`).

Its behavior (`crates/trading/src/examples/strategies/grid_mm/strategy.rs`):

1. On every quote, compute the mid, `(bid + ask) / 2`.
2. Re-quote only if the mid has moved by at least `requote_threshold_bps` since the last grid, or if
   the grid is empty.
3. Cancel every open order, then place `num_levels` buys below the mid and `num_levels` sells above
   it, spaced by `grid_step_bps`, each of size `trade_size`.
4. Shift the whole grid by `skew_factor * net_position` to discourage inventory growth.
5. Skip any level whose projected exposure would breach `max_position`.
6. At stop, cancel all orders and close all positions.

The configuration fields are the Python `GridMarketMakerConfig`
(`python/nautilus_trader/trading/__init__.pyi`), and they are described in
`docs/tutorials/grid_market_maker_dydx.md`. Lecture `05` uses every one of them.

## Passive quoting with execution algorithms

Two native execution algorithms exist for splitting and repegging passive orders
(`docs/concepts/execution/algorithms.md`):

- `IcebergAlgorithm` (`crates/trading/src/algorithm/iceberg.rs`): takes a parent order and sends one
  child at a time for `min(remaining, display_size)`. The parameters are `display_size` and
  `requote_secs`.
- `QuotePeggedAlgorithm` (`crates/trading/src/algorithm/quote_pegged.rs`): keeps one child working
  at the touch. The parameters are `pegging` (`passive` or `join`) and `requote_secs`.

Both are registered with `engine.add_native_exec_algorithm("IcebergAlgorithm", ...)` and
`engine.add_native_exec_algorithm("QuotePeggedAlgorithm", ...)`
(`crates/backtest/src/python/engine.rs:1203`). They are separate tools from the grid strategy.
Lecture `05` runs both.

## What is Python and what is Rust

This decides which language every sample is written in. The Python-exposed subsystems you will use
here are:

- The backtest engine and the builtin strategies (`crates/backtest/src/python/`).
- Instruments, orders, data types, order books, positions, and reports
  (`crates/model/src/python/`, `crates/persistence/src/python/`).
- Fill, slippage, fee, and execution-algorithm config (`crates/execution/src/python/`).
- Portfolio statistics and the analyzer (`crates/analysis/src/python/`).

The pre-trade send/cancel/fill count caps are set on the risk engine as `RiskCap` values
(`RiskEngineConfig(count_caps=...)`, with the metric and scope vocabularies in
`nautilus_trader.risk`). Lecture `07` explains what that means for you.

Continue to [03-first-run.md](03-first-run.md) to run your first market making backtest.

Previous: [01-what-is-market-making.md](01-what-is-market-making.md) | Next: [03-first-run.md](03-first-run.md)
