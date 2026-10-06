# nautilus-research

[![build](https://github.com/nautechsystems/nautilus_trader/actions/workflows/build.yml/badge.svg?branch=master)](https://github.com/nautechsystems/nautilus_trader/actions/workflows/build.yml)
[![Documentation](https://img.shields.io/docsrs/nautilus-research)](https://docs.rs/nautilus-research/latest/nautilus_research/)
[![crates.io version](https://img.shields.io/crates/v/nautilus-research.svg)](https://crates.io/crates/nautilus-research)
![license](https://img.shields.io/github/license/nautechsystems/nautilus_trader?color=blue)
[![Discord](https://img.shields.io/badge/Discord-%235865F2.svg?logo=discord&logoColor=white)](https://discord.gg/NautilusTrader)

Point-in-time research datasets for [NautilusTrader](https://nautilustrader.io).

The `nautilus-research` crate defines the dataset contract that keeps a research pipeline
honest about time. It is deliberately independent of the trading engine, so a research process
can build and version datasets without instantiating a node:

- Universe membership history stored as data beside the data it describes.
- The dataset declaration, its canonical serialization, and its stable digest.
- A point-in-time panel whose membership and feature apertures are checked by construction.
- The time-series and cross-sectional operator set, with stated conventions.
- Compiled features and labels, and the measurement of an admitted decision stream.

## NautilusTrader

[NautilusTrader](https://nautilustrader.io) is an open-source, production-grade, Rust-native
engine for multi-asset, multi-venue trading systems.

The system spans research, deterministic simulation, and live execution within a single
event-driven architecture, providing research-to-live semantic parity.

## Feature flags

This crate provides feature flags to control source code inclusion during compilation:

- `extension-module`: Builds as a Python extension module.
- `python`: Enables Python bindings from [PyO3](https://pyo3.rs).

## Documentation

See [the docs](https://docs.rs/nautilus-research) for more detailed usage.

## License

The source code for NautilusTrader is available on GitHub under the [GNU Lesser General Public License v3.0](https://www.gnu.org/licenses/lgpl-3.0.en.html).

---

NautilusTrader™ is developed and maintained by Nautech Systems, a technology
company specializing in the development of high-performance trading systems.
For more information, visit <https://nautilustrader.io>.
