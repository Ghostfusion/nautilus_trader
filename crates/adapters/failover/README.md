# nautilus-failover

[![build](https://github.com/nautechsystems/nautilus_trader/actions/workflows/build.yml/badge.svg?branch=master)](https://github.com/nautechsystems/nautilus_trader/actions/workflows/build.yml)
[![Documentation](https://img.shields.io/docsrs/nautilus-failover)](https://docs.rs/nautilus-failover/latest/nautilus_failover/)
[![crates.io version](https://img.shields.io/crates/v/nautilus-failover.svg)](https://crates.io/crates/nautilus-failover)
![license](https://img.shields.io/github/license/nautechsystems/nautilus_trader?color=blue)
[![Discord](https://img.shields.io/badge/Discord-%235865F2.svg?logo=discord&logoColor=white)](https://discord.gg/NautilusTrader)

[NautilusTrader](https://nautilustrader.io) support for chaining market data providers in priority
order, so that a demand is served by the first provider that can answer it.

The `nautilus-failover` crate holds the part of that arrangement which is not a provider: what a
failure means, whether it is worth another attempt at the same provider, and when a demand moves to
the next one. It ships no provider of its own, and it is not an adapter for a venue.

## NautilusTrader

[NautilusTrader](https://nautilustrader.io) is an open-source, production-grade, Rust-native
engine for multi-asset, multi-venue trading systems.

The system spans research, deterministic simulation, and live execution within a single
event-driven architecture, providing research-to-live semantic parity.

## Scope

- `failure`: why a provider could not answer, and what the chain does about it.
- `chain`: one provider as the chain sees it, one demand run across an ordered list of them, and the
  trace of what was tried.
- `tap`: the boundary between a chain's providers and the engine, where a provider's identity becomes
  the chain's and the caller's correlation identifier is left alone.
- `pump`: the sender a chain gives its providers and the task that drains it, together with the
  register of which demand each answer answers.
- `health`: what the chain knows about its providers, when one of them has stopped being worth
  asking, and the counts that make an outage visible rather than merely survived.
- `composite`: the data client the engine sees, which serves a demand from the first provider that
  will take it, and is assembled from legs rather than configured.
- `testing`: providers and clients whose answers are written down in advance, so that a chain can be
  exercised without a network.

Two rules decide everything: a retry is for a transient condition on a provider that is otherwise
the right one, and a hop is a deliberate move to a different dataset. An empty answer is neither: it
is a provider answering, and the chain stops there.

## License

The source code for this crate is licensed under the GNU Lesser General Public License Version 3.0.
