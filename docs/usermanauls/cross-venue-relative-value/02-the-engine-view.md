# 02 - The engine view

This lecture describes how NautilusTrader represents a cross-venue relative-value trade. There is
no code in this lecture. Every claim names the file that carries it.

## 1. The objects you will touch

| Object              | What it is                                                                    | Where it is defined                                |
| ------------------- | ----------------------------------------------------------------------------- | -------------------------------------------------- |
| `Venue`             | The name of one trading venue, for example `"BINANCE"`.                       | `crates/model/src/identifiers/venue.rs`            |
| `InstrumentId`      | A symbol plus a venue, for example `BTCUSDT-PERP.BINANCE`.                    | `crates/model/src/identifiers/instrument_id.rs`    |
| `CurrencyPair`      | A spot instrument: a base asset priced in a quote asset.                      | `crates/model/src/instruments/currency_pair.rs`    |
| `CryptoPerpetual`   | A perpetual future on a crypto asset.                                         | `crates/model/src/instruments/crypto_perpetual.rs` |
| `AccountType`       | `CASH`, `MARGIN`, `BETTING` or `WALLET`. Decides how balances are locked.     | `crates/model/src/enums.rs`                        |
| `OmsType`           | `NETTING` or `HEDGING`. Decides how fills become positions.                   | `crates/model/src/enums.rs`                        |
| `QuoteTick`         | One two-sided top-of-book update: bid, ask and their sizes.                   | `crates/model/src/data/quote.rs`                   |
| `FundingRateUpdate` | The funding rate of a perpetual future, sometimes with the next funding time. | `crates/model/src/data/funding.rs`                 |
| `IndexPriceUpdate`  | The venue's external index price for the underlying.                          | `crates/model/src/data/prices.rs`                  |
| `MarkPriceUpdate`   | The price the venue uses for margin and liquidation.                          | `crates/model/src/data/prices.rs`                  |

The concept pages for the data types are
[`docs/concepts/data/quote_tick.md`](../../concepts/data/quote_tick.md),
[`docs/concepts/data/funding_rate_update.md`](../../concepts/data/funding_rate_update.md),
[`docs/concepts/data/index_price_update.md`](../../concepts/data/index_price_update.md) and
[`docs/concepts/data/mark_price_update.md`](../../concepts/data/mark_price_update.md).

## 2. One engine, several venues

A backtest holds as many venues as you register. Each call to `BacktestEngine.add_venue` creates a
separate simulated venue with:

- its own matching engine per instrument,
- its own account, named `<VENUE>-001` (this is the account id you see in the account report),
- its own fee model, fill model, latency model and book type,
- its own order management system type.

The signature is in `python/nautilus_trader/backtest/__init__.pyi` (`BacktestEngine.add_venue`).
The parameters that matter for this style are:

| Parameter           | Why it matters for relative value                                                                                    |
| ------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `venue`             | Which venue the instrument and account belong to.                                                                    |
| `oms_type`          | `NETTING` keeps one position per instrument and strategy; `HEDGING` keeps several.                                   |
| `account_type`      | `MARGIN` is what a perpetual future requires.                                                                        |
| `base_currency`     | The currency the venue's account reports in. Use the same currency on both venues so the two results are comparable. |
| `starting_balances` | Money per venue. Two venues means two balances.                                                                      |
| `fee_model`         | Two venues means two fee schedules. This is where most of the basis is lost.                                         |
| `fill_model`        | How much of an order fills and whether it slips.                                                                     |

`add_venue` requires an explicit `fee_model`, including an explicit zero-fee model. A venue added
without one raises `ValueError: Backtest venue requires an explicit fee_model, including an
explicit zero-fee model`. That error is deliberate: a silent zero-fee default would make every
backtest look profitable.

**Key consequence:** a position belongs to one venue. Two venues always means two positions, two
balances and two margin calculations. Nothing in the engine merges them into one "pair" position.
See [`docs/concepts/accounting.md`](../../concepts/accounting.md), which states that the portfolio
and the account model are the only authority for position state, realised PnL, unrealised PnL and
margin.

## 3. The instruments of this manual

The manual uses the same underlying on two venues:

| Instrument id          | Type              | Venue   | Where it comes from                                                                              |
| ---------------------- | ----------------- | ------- | ------------------------------------------------------------------------------------------------ |
| `BTCUSDT-PERP.BINANCE` | `CryptoPerpetual` | BINANCE | `TestInstrumentProvider.btcusdt_perp_binance()` in `python/nautilus_trader/testkit/providers.py` |
| `BTCUSDT-PERP.BYBIT`   | `CryptoPerpetual` | BYBIT   | Constructed in the lecture, because the provider ships no such contract                          |

Both are linear perpetuals: base `BTC`, quote `USDT`, settlement `USDT`. Both use price precision
1 (a tick of `0.1`) and size precision 3 (a lot step of `0.001`). That makes their prices directly
comparable, which is what a basis calculation needs.

The two instruments are separate `CryptoPerpetual` objects with different ids. The engine will
never assume they are related. If you want the engine to compute the difference for you, that is a
`synthetic instrument`, described in section 6.

## 4. What makes a perpetual different from a spot price

A spot market has one important price: the last trade, and the bid and ask around it. A perpetual
future has three reference prices, and the venue uses them for different things:

- `IndexPriceUpdate` - an external index over several spot venues. It is the venue's opinion of
  what the asset is worth, and it is used to compute the other two prices.
- `MarkPriceUpdate` - the price the venue uses for margin, unrealised PnL and liquidation. It is
  usually built from the index and the perpetual's own order book.
- `FundingRateUpdate` - the rate that is exchanged between longs and shorts at each funding
  boundary, which is the mechanism that pulls the perpetual back toward the index.

The behaviour notes in the three concept pages are explicit: index prices, mark prices and funding
rates are **reference data** and do not imply a trade occurred, and a funding rate does not imply a
payment was applied. This is exactly why a held basis is not a static number. While you hold the
pair, funding keeps being charged or paid, and the mark price decides whether each leg survives.

The backtest engine can settle funding. `crates/backtest/src/exchange.rs` derives a settlement
boundary from a `FundingRateUpdate` (from `next_funding_ns` when it is set, otherwise from the
event time when the interval aligns) and, at that boundary, applies a funding payment to every
open position in the instrument. The payment arrives as a position adjustment of type `FUNDING`,
which is described in [`docs/concepts/positions.md`](../../concepts/positions.md) under "Funding
payments": the adjustment tracks the periodic payment without changing position quantity.

## 5. Two legs, two positions, one portfolio

The engine's position model is in [`docs/concepts/positions.md`](../../concepts/positions.md).
Three facts decide how a two-leg relative-value trade is reported:

1. A position is created on the first fill, and its id under `NETTING` is
   `{instrument_id}-{strategy_id}`. With two venues you therefore get two positions with two ids.
2. `NETTING` combines all fills for one instrument and strategy into a single position. A fill that
   crosses zero closes the cycle and opens the opposite one. That is what a round trip on one leg
   looks like in the report.
3. `HEDGING` allows several simultaneous long and short positions in the same instrument. It does
   **not** net across positions, and it can increase margin requirements, because the venue may
   hold both sides of the same instrument separately.

`docs/concepts/positions.md` also documents the reversal rule you will meet in the exercises: an
opposite-side fill larger than the open quantity closes the existing exposure and opens the
remainder in the other direction, and only the closing portion realises PnL.

Because the portfolio aggregates across venues, the **net** result of a pair is the sum of two
per-venue results. Each venue reports its own realised PnL, its own commissions and its own
adjustments. Section 6 of `docs/concepts/accounting.md` describes the query surface; the account
report printed by the backtest engine is the easiest way to see the two results side by side.

## 6. Synthetic instruments, universes and continuous futures

Three features are adjacent to this style. Two are useful, and one is a trap if you misread it.

**Synthetic instruments** let the engine compute a derived price, such as a spread, from component
instruments, and publish it as a normal instrument on the reserved venue code `SYNTH`. The formula
language is described in [`docs/concepts/synthetics.md`](../../concepts/synthetics.md), and the
component references are raw instrument ids, so `BTCUSDT-PERP.BYBIT - BTCUSDT-PERP.BINANCE` is a
valid formula. The warning in that page is what matters here: **synthetic instruments cannot be
traded directly**. They exist locally as analytical tools, for subscriptions and for emulated-order
triggers. You cannot submit an order for a synthetic. The manual does not use one, because a
beginner should see the spread computed explicitly and the two real legs submitted explicitly.

**Universes** make instrument membership a property of the run rather than of the code, so that
instruments can enter and leave play while the system runs. A `UniverseDefinition` carries a venue,
a rule, the subscriptions each member holds, a selection interval and a removal policy; selection
is clock-driven and canonical, and removal is a process, not an immediate unsubscribe. See
[`docs/concepts/universes.md`](../../concepts/universes.md). A relative-value screen that trades the
widest basis across a changing set of venues is a natural universe user, but a universe never
submits or cancels orders, so the trading logic stays in your strategy.

**Continuous futures** splice consecutive expiring contracts into one adjusted series, with the
roll transitions supplied by the caller. See
[`docs/concepts/continuous_futures.md`](../../concepts/continuous_futures.md). A perpetual future
does not need this, because it does not expire. It is listed here because a relative-value desk
that trades futures calendars will meet it, and because the adjustment modes (`BACKWARD_SPREAD`,
`FORWARD_SPREAD`, `BACKWARD_RATIO`, `FORWARD_RATIO`) change what "the price" of a spliced series
means. Comparing a continuous series against a raw contract price without declaring the adjustment
is a way to invent a basis that does not exist.

## 7. Execution reality

A cross-venue strategy submits two orders that the engine routes to two different execution
clients. [`docs/concepts/execution/index.md`](../../concepts/execution/index.md) states the routes:
`submit_order` goes to the order emulator for emulated orders, to an execution algorithm when
`exec_algorithm_id` is set, and to the risk engine otherwise, and the risk engine passes the
command to the execution engine and then the client.

Three consequences for this style:

- **There is no atomic two-venue order.** Order lists require every order in a list to use the same
  venue (see [`docs/concepts/orders/advanced.md`](../../concepts/orders/advanced.md)). Two venues
  therefore means two separate submissions, and the engine does not roll one back when the other is
  denied or rejected. Lecture 05 measures this.
- **Each leg is risk-checked separately.** The risk engine checks price precision, quantity
  precision, minimum and maximum notional, `max_notional_per_order`, position-reducing exposure,
  the trading state of the instrument, and more, per order. A cap hit on one leg denies that leg,
  and nothing else.
- **Reconciliation is per client.** `docs/concepts/live.md` states that at startup, reconciliation
  aligns cached order and position state with venue reports before trader startup, and
  [`../../concepts/execution/reconciliation.md`](../../concepts/execution/reconciliation.md)
  describes the venue-specific detail. Each execution client reconciles its own venue, so with two
  venues there are two reconciliations and two states, and your strategy is the only thing that
  knows the two legs belong to one trade.

## 8. Configuration you will use

| Config                 | Field                                                      | Purpose here                                                                   |
| ---------------------- | ---------------------------------------------------------- | ------------------------------------------------------------------------------ |
| `BacktestEngineConfig` | `trader_id`                                                | Identifies the run.                                                            |
| `BacktestEngineConfig` | `logging=LoggerConfig(stdout_level=LogLevel.ERROR)`        | Keeps the engine's own logs out of the way so the teaching output is readable. |
| `BacktestEngineConfig` | `risk_engine=RiskEngineConfig(max_notional_per_order=...)` | Per-instrument notional cap; used in lecture 05 to deny one leg on purpose.    |
| `MakerTakerFeeModel`   | `maker_rate`, `taker_rate`                                 | Two venues, two fee schedules.                                                 |
| `DefaultFillModel`     | `prob_fill_on_limit`, `prob_slippage`                      | Deterministic full fills at the touch with `1.0` and `0.0`.                    |
| `StrategyConfig`       | your own fields                                            | The instrument ids, the trade size and the entry and exit thresholds.          |

`MakerTakerFeeModel` and `DefaultFillModel` are defined in `crates/execution/src/models/` with
Python bindings in `crates/execution/src/python/`. `DefaultFillModel` is documented in its own
source as a probabilistic state around `prob_fill_on_limit` and `prob_slippage`; its default is
`(1.0, 0.0)`, which fills every order and never slips.

## 9. Where the pieces live

| Piece                                | Path                                                               |
| ------------------------------------ | ------------------------------------------------------------------ |
| Backtest engine and simulated venue  | `crates/backtest/`, bindings in `crates/backtest/src/python/`      |
| Instruments, data types, identifiers | `crates/model/src/`, bindings in `crates/model/src/python/`        |
| Fill, slippage and fee models        | `crates/execution/src/models/`                                     |
| Risk checks and notional caps        | `crates/risk/src/`                                                 |
| Portfolio, accounts and PnL          | `crates/portfolio/`, concept page in `docs/concepts/accounting.md` |
| Test instrument and data providers   | `python/nautilus_trader/testkit/providers.py`                      |

A worked relative-value style tutorial already exists for options and is written in Rust:
[`docs/tutorials/delta_neutral_options_bybit.md`](../../tutorials/delta_neutral_options_bybit.md)
and [`docs/tutorials/delta_neutral_options_derive.md`](../../tutorials/delta_neutral_options_derive.md).
Read them after this manual if you want to see the same idea with three legs (a call, a put and a
perpetual hedge) instead of two, and with a venue-provided delta instead of a price difference.

Continue to [03-first-run.md](03-first-run.md).
