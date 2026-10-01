# 09 - Going further

You have read the words, run a chain backtest, opened the sample data, built a pipeline,
measured the numbers, and seen the failure modes. This lecture is honest about what the
manual did not cover and points at the files to read next.

## Honest gaps

**Option pricing is Rust only.** There is no Python function that prices an American option.
The models live in `crates/model/src/data/pricing.rs` and
`crates/model/src/data/binomial.rs`, and the manual demonstrated them by running the
`nautilus-model` crate's tests with `cargo nextest` and a small scratch crate. If you need
pricing inside Python, you must either call the low-level Black-Scholes helpers that are
exposed (`black_scholes_greeks`, `imply_vol`, `imply_vol_and_greeks`, `refine_vol_and_greeks`)
or do the pricing outside the Python process.

**The volatility surface is Rust only.** `crates/model/src/data/volatility_surface.rs` has
no Python binding. The manual explained its checks in [06](06-measure-and-evaluate.md) and
ran its tests, but you cannot query a surface from a Python strategy today. Use venue
greeks for live decisions and treat the surface as a research and validation tool.

**Python does not expose the declared exercise style on an instrument.** The `ExerciseStyle`
enum is available, and an `OptionContract` can be constructed from Python, but the
per-instrument getter that returns the declaration is a Rust trait method. In practice this
means Python code should treat the style as known from how the instrument was built, not
read it back.

**American greeks from the local calculator are approximations.** The calculator prices
American options as European for the greeks computation. The exact American price is the
Rust tree. Do not rely on calculator greeks for a deeply in-the-money American put near
expiry.

**Option backtests need recorded data.** The engine does not download or request missing
catalog data during a run. A chain backtest needs `QuoteTick` records and `OptionGreeks`
records for the same instruments, plus the instruments themselves. The
[Tardis option chain example](../../../examples/backtest/tardis_option_chain.py) states this
requirement and cannot run without such a catalog.

**The option matching model is quote-driven.** Market orders and marketable limits fill as
takers against the replayed best bid and offer. Passive limits rest and can fill as makers.
The model does not simulate level-two queue position for options, so a passive fill in a
backtest is optimistic compared with a real queue.

**One live strategy ships as Rust.** `DeltaNeutralVol` is registered in the live node's Rust
register (`crates/live/src/python/node.rs`) and runs from the Rust tutorial. There is no
Python builtin option strategy in the backtest register.

## The concept docs to read next

| Doc                                                                             | Why                                                                                                                                                                  |
| ------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [options.md](../../concepts/options.md)                                         | The instrument types, exercise style, the surface, the subscription API, and the chain architecture diagram. Read it in full; this manual is its beginner companion. |
| [greeks.md](../../concepts/greeks.md)                                           | Venue greeks versus the local calculator, every field, and the two paths' trade-offs.                                                                                |
| [data/option_greeks.md](../../concepts/data/option_greeks.md)                   | The exact field table for the greeks data type.                                                                                                                      |
| [data/index.md](../../concepts/data/index.md)                                   | Where `OptionGreeks` sits among the built-in data types and how it is recorded and replayed.                                                                         |
| [instruments/option_contract.md](../../concepts/instruments/option_contract.md) | The full `OptionContract` definition.                                                                                                                                |
| [instruments/crypto_option.md](../../concepts/instruments/crypto_option.md)     | The crypto option definition, including inverse and quanto settlement.                                                                                               |
| [instruments/index.md](../../concepts/instruments/index.md)                     | The whole instrument taxonomy.                                                                                                                                       |
| [custom_data.md](../../concepts/custom_data.md)                                 | Where to put greeks a venue does not fit into the native type, such as vanna, volga, or charm.                                                                       |

## The tutorials and examples

- [options_data_bybit.md](../../tutorials/options_data_bybit.md): the Rust tutorial for live
  per-instrument greeks and chain snapshots on Bybit. It needs no API key for public data.
- [delta_neutral_options_bybit.md](../../tutorials/delta_neutral_options_bybit.md): the Rust
  tutorial for a live short-strangle strategy hedged with a perpetual. It trades real money
  on mainnet; read the warnings before running it.
- [bybit_option_greeks.py](../../../examples/live/bybit/bybit_option_greeks.py) and
  [deribit_option_greeks.py](../../../examples/live/deribit/deribit_option_greeks.py):
  Python live examples that subscribe to venue greeks. They require a connected adapter.
- [deribit_option_chain.py](../../../examples/live/deribit/deribit_option_chain.py): a Python
  live example that subscribes to an aggregated chain. It requires a connected adapter.
- [tardis_option_chain.py](../../../examples/backtest/tardis_option_chain.py): a Python
  backtest that replays a catalog. It requires a Nautilus Parquet catalog with option
  instruments, quotes, and greeks.

## What to learn next, in order

1. **Record and replay greeks.** Write a small run that subscribes to venue greeks, records
   them with the catalog, and replays them in a backtest. This closes the loop between live
   and simulated option data.
2. **Build a delta-neutral position in Python.** Use venue deltas to compute portfolio delta
   and submit a hedge, following the logic in the delta-neutral tutorial but on the backtest
   engine with recorded data.
3. **Understand the surface coordinates.** Learn total implied variance and log moneyness
   well enough to read a slice by hand. The tests in
   `crates/model/tests/volatility_surface.rs` are a good worked reference.
4. **Run the Rust pricing tests and read them.** The file
   `crates/model/tests/option_pricing.rs` documents the identities and the reference values,
   and it is the shortest way to understand what the tree guarantees.
5. **Study the risk limits.** Read `crates/risk/src/python/config.rs` and
   `crates/risk/src/config.rs` and decide which caps your strategy needs. Remember the
   pre-trade count caps are Rust only.

## Closing note

Options reward precision. A sign error on a delta, a stale quote in a chain, or a European
contract declared as American all cost money, and none of them announces itself. The engine
gives you the tools to see the numbers and the checks to refuse bad data, but it does not
make the decision for you. Read the chain, check the forward, respect the decay, and size so
that being wrong is survivable.
