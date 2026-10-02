# nautilus-moomoo

[![build](https://github.com/nautechsystems/nautilus_trader/actions/workflows/build.yml/badge.svg?branch=master)](https://github.com/nautechsystems/nautilus_trader/actions/workflows/build.yml)
[![Documentation](https://img.shields.io/docsrs/nautilus-moomoo)](https://docs.rs/nautilus-moomoo/latest/nautilus_moomoo/)
[![crates.io version](https://img.shields.io/crates/v/nautilus-moomoo.svg)](https://crates.io/crates/nautilus-moomoo)
![license](https://img.shields.io/github/license/nautechsystems/nautilus_trader?color=blue)
[![Discord](https://img.shields.io/badge/Discord-%235865F2.svg?logo=discord&logoColor=white)](https://discord.gg/NautilusTrader)

[NautilusTrader](https://nautilustrader.io) adapter for the [moomoo](https://www.moomoo.com) OpenD
gateway.

The `nautilus-moomoo` crate speaks the gateway's frame protocol directly over a local OpenD
instance. The vendor's Python client is not used, because this repository's adapter namespace is
Rust-backed and expects a compiled extension.

## NautilusTrader

[NautilusTrader](https://nautilustrader.io) is an open-source, production-grade, Rust-native
engine for multi-asset, multi-venue trading systems.

The system spans research, deterministic simulation, and live execution within a single
event-driven architecture, providing research-to-live semantic parity.

## Scope

The intended scope is market data for US and HK equities: instruments, bars, trades, quotes, order
book, and corporate actions, over a local gateway. Execution is out of scope, and so are US options
and US futures, which the current entitlement refuses.

The crate currently provides the protocol layer: the vendored `proto2` schema in `proto/`, the Rust
types generated from it under `src/generated/`, and the inert generator described in
[`proto/README.md`](proto/README.md). The codec, connection, entitlement record, and data client are
built on that layer.

## Prerequisites

A running OpenD gateway reachable from the host. The gateway holds the login; the adapter has no API
key of its own.

## License

The source code for this crate is licensed under the GNU Lesser General Public License Version 3.0.
