# 02 - The engine view of options

This lecture is a map. It names the objects and the files that represent options in this
repository, with no programs yet. Read it once and come back to it when a later lecture
uses a name you do not recognise. Everything here is stated in
[the options concept page](../../concepts/options.md) and the
[Greeks concept page](../../concepts/greeks.md); the point of this lecture is to put those
names in one place and to say which language owns each one.

## The option instruments

The engine defines five option instrument types. An instrument is the engine's full
description of a tradable thing: its identity, its price precision, its currency, and its
contract rules.

| Instrument           | What it is                                                                     | Per-contract details                                                     |
| -------------------- | ------------------------------------------------------------------------------ | ------------------------------------------------------------------------ |
| `OptionContract`     | An exchange-traded option on an underlying, with a strike and an expiry.       | Strike, option kind, expiry, underlying, multiplier.                     |
| `OptionSpread`       | An exchange-defined multi-leg option strategy, published as one tradable line. | Underlying, expiry, and a venue strategy code. No single strike or kind. |
| `CryptoOption`       | A crypto option with a crypto quote and settlement currency.                   | Full Greeks inputs, plus inverse or quanto settlement.                   |
| `CryptoOptionSpread` | A crypto option spread with inverse settlement and fractional size.            | Underlying, expiry, strategy code, inverse flag, settlement currency.    |
| `BinaryOption`       | A fixed-payout option that settles to 0 or 1.                                  | Expiry and an outcome, but no strike, kind, or underlying.               |

The per-type guide is at
[option_contract.md](../../concepts/instruments/option_contract.md),
[crypto_option.md](../../concepts/instruments/crypto_option.md), and
[binary_option.md](../../concepts/instruments/binary_option.md). The summary table above is
from [options.md](../../concepts/options.md).

An `OptionContract` needs a strike, so it carries `strike_price`, `option_kind`
(`CALL` or `PUT`), `expiration_ns`, `underlying`, and `multiplier`. An `OptionSpread` does
not: the exchange publishes it as one line, and any leg detail the venue sends is stored in
the free-form `info` field. This matters in code because you cannot read `strike_price`
from a spread.

## Exercise style is declared, not assumed

An option's early-exercise right changes its value, so the engine declares the style on the
instrument instead of assuming one per chain. `ExerciseStyle` is a Python-visible enum with
two members, `AMERICAN` and `EUROPEAN`
(`crates/model/src/python/enums.rs`, exercised in [05](05-build-the-strategy.md)).

- `OptionContract` declares `ExerciseStyle::American` by default, the listed equity
  convention, and can be built for a European contract.
- `CryptoOption` declares `ExerciseStyle::European`.

The Rust trait method `Instrument::exercise_style` returns the declaration for an option and
nothing for other instruments. Pricing reads that declaration, which is the subject of the
next section.

## Where each option capability lives

This table decides the language of every code sample in the manual. Read it before looking
for an import.

| Capability                                                                                                             | Surface                      | Where                                                                    |
| ---------------------------------------------------------------------------------------------------------------------- | ---------------------------- | ------------------------------------------------------------------------ |
| `ExerciseStyle`, option instruments, orders, data types                                                                | Python                       | `crates/model/src/python/`                                               |
| Option chain subscriptions and `OptionChainSlice`                                                                      | Python                       | `crates/model/src/python/`, `crates/data/src/`                           |
| Greece data type and subscriptions, local calculator                                                                   | Python                       | `crates/model/src/python/`, `crates/common/src/python/`                  |
| Low-level Black-Scholes helpers (`black_scholes_greeks`, `imply_vol`, `imply_vol_and_greeks`, `refine_vol_and_greeks`) | Python                       | `crates/model/src/data/greeks.rs`, exposed under `nautilus_trader.model` |
| Option pricing, Black-Scholes and the Cox-Ross-Rubinstein tree                                                         | Rust only, no Python binding | `crates/model/src/data/pricing.rs`, `crates/model/src/data/binomial.rs`  |
| Volatility surface calibration and query                                                                               | Rust only, no Python binding | `crates/model/src/data/volatility_surface.rs`                            |

The last two rows are the core fact of this manual. There is no Python function that prices
an American option and no Python function that builds a volatility surface. Those are Rust
subsystems. Lecture [05](05-build-the-strategy.md) demonstrates them by running the
`nautilus-model` crate's own tests with `cargo nextest`, and lecture [06](06-measure-and-evaluate.md)
explains what the surface checks guarantee.

## The data types

These are the types that carry option information through the engine.

| Type               | What it carries                                                       | Notes                                                                                                              |
| ------------------ | --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------ |
| `QuoteTick`        | Best bid and ask for one instrument.                                  | The chain's price input.                                                                                           |
| `OptionGreeks`     | Venue-provided sensitivities and implied volatility for one contract. | Delta, gamma, vega, theta, rho, mark, bid and ask implied volatility, underlying price, open interest, timestamps. |
| `OptionSeriesId`   | Venue, underlying, settlement currency, expiry.                       | Identifies one expiry series.                                                                                      |
| `StrikeRange`      | Which strikes a chain subscription keeps active.                      | `fixed`, `atm_relative`, `atm_percent`, or `delta`.                                                                |
| `OptionStrikeData` | The quote and the optional greeks for one strike.                     | Returned by the slice accessors.                                                                                   |
| `OptionChainSlice` | A point-in-time snapshot of a whole series.                           | Joins quotes and greeks by instrument and groups by strike and kind.                                               |
| `GreeksData`       | One local calculation's inputs and results.                           | From `GreeksCalculator`.                                                                                           |
| `PortfolioGreeks`  | Aggregated portfolio sensitivities.                                   | From `calculator.portfolio_greeks(...)`.                                                                           |

The fields of `OptionGreeks` are listed exactly in
[data/option_greeks.md](../../concepts/data/option_greeks.md) and in
[greeks.md](../../concepts/greeks.md). The chain slice's three properties and its accessor
methods are listed in [options.md](../../concepts/options.md#optionchainslice-data-type).

## Two subscription levels

There are exactly two ways to receive option data in Python.

1. **Per-instrument greeks.** Call `subscribe_option_greeks(instrument_id, client_id=...)`
   and implement `on_option_greeks(greeks)`. One call per contract.
2. **Option chain slices.** Call `subscribe_option_chain(series_id, strike_range=...,
   snapshot_interval_ms=...)` and implement `on_option_chain(slice)`. One subscription for
   a whole series.

Both are methods of `Strategy` and `DataActor`, declared in
`python/nautilus_trader/trading/__init__.pyi` and
`python/nautilus_trader/common/__init__.pyi`. The documentation for both levels, including
the strike-range variants and the snapshot versus raw mode, is in
[options.md](../../concepts/options.md#subscribing-to-greeks).

### The chain architecture

The chain is not built inside your strategy. The `DataEngine` creates one Rust
`OptionChainManager` per subscribed `OptionSeriesId` and owns the lifecycle. The manager
wraps an `OptionChainAggregator`, which keeps the latest quote and greeks per instrument,
and an `AtmTracker`, which derives the at-the-money strike from the `underlying_price`
field of incoming greeks. On a timer the manager publishes a snapshot; in raw mode it
publishes after every update. The component responsibilities are described in
[options.md](../../concepts/options.md#option-chain-architecture).

Two consequences a beginner should learn now.

- Greeks can arrive before a quote and a quote can arrive before greeks. The aggregator
  holds the first and attaches it when the second arrives.
- For a dynamic strike range the active set cannot be chosen until the at-the-money price
  is known, so the subscription is deferred until a reference price or the first greeks
  event supplies it.

## The configuration that matters

For an option-chain backtest the venue must have an explicit fee model. The engine refuses
a venue without one, as you will see in [03](03-first-run.md). Two option fee models exist
and are passed to the simulated venue, not inferred from the venue name
([options.md](../../concepts/options.md#backtesting-option-chains)):

- `CappedOptionFeeModel(maker_rate=..., taker_rate=...)` for venues that cap the fee.
- `TieredNotionalOptionFeeModel(maker_rate=..., taker_rate=...)` for notional tiers.

Data for an option-chain backtest is declared with `BacktestDataConfig`, one entry for
`NautilusDataType.QuoteTick` and one for `NautilusDataType.OptionGreeks`. The engine reads
from a Nautilus Parquet catalog and does not download missing data during the run. A
hand-built list of `QuoteTick` and `OptionGreeks` objects works as well, and that is what
[03](03-first-run.md) uses so you need no catalog.

## The pricing model in plain words

The Rust module `crates/model/src/data/pricing.rs` defines the shared parameter set and one
interface, `OptionPricingModel`, with a single `price` method. A helper, `price_option`,
looks at the declared `ExerciseStyle` and chooses the model:

- European exercise uses the repository's closed-form Black-Scholes price.
- American exercise uses a Cox-Ross-Rubinstein binomial tree with a default step count of
  512 (`crates/model/src/data/binomial.rs`).

The tree is an approximation. Its error shrinks roughly like one over the step count, with
a wobble that depends on whether the step count is odd or even, so the repository's tests
bound the error against the closed form instead of asserting one exact number. The same
tests check the identities an American price must satisfy: it is never below the European
price or the intrinsic value, and an American call on an underlying with no dividend equals
the European call.

The pricing parameters carry a cost of carry `b`. When `b` equals the risk-free rate the
model is Black-Scholes; when `b` is zero the model is Black-76 on a futures-style
underlying; when `b` is the rate minus a dividend yield it is Black-Scholes with a
continuous dividend. That single field is how one model covers all three cases.

The module also exposes `implied_forward_from_parity`, which turns one call price and one
put price at the same strike into an implied forward price of the underlying. Lecture
[05](05-build-the-strategy.md) runs it.

## Where to read more

- [options.md](../../concepts/options.md): instruments, exercise style, the surface, chains,
  and the architecture diagram.
- [greeks.md](../../concepts/greeks.md): venue greeks versus the local calculator, and the
  definition of every greek.
- [data/option_greeks.md](../../concepts/data/option_greeks.md): the field table for the data
  type.
- [data/index.md](../../concepts/data/index.md): where `OptionGreeks` sits among the built-in
  data types and how it is recorded and replayed.

With the map in place, [03](03-first-run.md) runs the smallest complete program.
