# nautilus-eodhd

[![build](https://github.com/nautechsystems/nautilus_trader/actions/workflows/build.yml/badge.svg?branch=master)](https://github.com/nautechsystems/nautilus_trader/actions/workflows/build.yml)
[![Documentation](https://img.shields.io/docsrs/nautilus-eodhd)](https://docs.rs/nautilus-eodhd/latest/nautilus_eodhd/)
[![crates.io version](https://img.shields.io/crates/v/nautilus-eodhd.svg)](https://crates.io/crates/nautilus-eodhd)
![license](https://img.shields.io/github/license/nautechsystems/nautilus_trader?color=blue)
[![Discord](https://img.shields.io/badge/Discord-%235865F2.svg?logo=discord&logoColor=white)](https://discord.gg/NautilusTrader)

[NautilusTrader](https://nautilustrader.io) adapter for [EODHD](https://eodhd.com).

The `nautilus-eodhd` crate provides access to EODHD end-of-day historical market data. It is a
data-only adapter: it ships an instrument provider and a historical data loader, and provides no
live data client and no execution client.

## NautilusTrader

[NautilusTrader](https://nautilustrader.io) is an open-source, production-grade, Rust-native
engine for multi-asset, multi-venue trading systems.

The system spans research, deterministic simulation, and live execution within a single
event-driven architecture, providing research-to-live semantic parity.

## Feature flags

This crate provides feature flags to control source code inclusion during compilation,
depending on the intended use case:

- `extension-module`: Builds as a Python extension module.
- `high-precision` (default): Enables
  [high-precision mode](https://nautilustrader.io/docs/nightly/getting_started/installation/#precision-mode)
  to use 128-bit value types.
- `python`: Enables Python bindings from [PyO3](https://pyo3.rs).

## Scope

The adapter covers the EODHD REST endpoints needed to build a backtest universe and load
historical bars:

- `/eod/{ticker}` -> daily, weekly, or monthly `Bar` records.
- `/exchange-symbol-list/{exchange}` -> `Equity` instrument definitions.

News, fundamentals, sentiment, and intraday endpoints are out of scope. The platform has no data
type for them, and they are not required to run a bar-driven backtest.

## License

The source code for this crate is licensed under the GNU Lesser General Public License Version 3.0.
