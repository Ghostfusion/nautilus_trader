// -------------------------------------------------------------------------------------------------
//  Copyright (C) 2015-2026 Nautech Systems Pty Ltd. All rights reserved.
//  https://nautechsystems.io
//
//  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
//  You may not use this file except in compliance with the License.
//  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
// -------------------------------------------------------------------------------------------------

//! Optional target construction: resolving signals into desired exposures.
//!
//! This module is the construction stage of the optional target pipeline. It consumes the signals
//! a strategy or an alpha model states together with a portfolio context, and returns one portfolio
//! target per signal, in the order the signals are given. It is the middle of three layers: a
//! signal is a statement of view, a target is the exposure that view resolves to, and an order is
//! how the exposure is reached.
//!
//! The stage is standalone and optional. It holds no engine, no actor, and no strategy
//! registration: it takes plain data in and returns plain data out, so it is unit testable without
//! an engine. It never submits, cancels, or modifies an order, and it reads or writes neither the
//! cache nor the portfolio. The direct order path is unchanged by its presence.
//!
//! This module is a single file rather than a directory because the repository ignores any
//! directory named `target` (`.gitignore`, the Rust build artifact rule), which would leave a
//! `src/target/` directory untracked. It matches the layout of the target and signal values in
//! `nautilus-model`.
//!
//! # Sizing
//!
//! A directional signal is sized by the existing fixed-risk sizing calculation in
//! `nautilus_risk::sizing`, which is the only sizing implementation the stage uses. The entry price
//! comes from the context and the stop price is derived from it by
//! [`TargetConstructionConfig::stop_loss_bps`] for the signal's direction: below the entry for a
//! long and above it for a short, rounded to the instrument's price precision. The context equity,
//! the risk derived from the configuration and the signal strength, the configured commission
//! rate, an exchange rate of one, and the instrument's size increment are passed through, so the
//! result follows the sizing calculation's own conventions.
//!
//! # Direction
//!
//! `SignalDirection::Long` constructs a positive weight, `SignalDirection::Short` a negative one,
//! and `SignalDirection::Flat` a flat target. A flat signal is how a caller states reduce-to-zero,
//! and it is resolved without reading the context at all.
//!
//! # Strength
//!
//! `TradingSignal::strength` scales the risk the position is sized from, by a factor of:
//!
//! - `1` when the strength is absent, so the risk is the configured
//!   [`TargetConstructionConfig::risk_per_trade`];
//! - the strength clamped to the unit interval `[0, 1]` when it is present.
//!
//! The risk used is therefore `risk_per_trade * factor`, which is never more than `risk_per_trade`:
//! a strength below one risks proportionally less, and a strength at or above one risks the
//! configured amount. The signal documents strength as an unbounded magnitude rather than a
//! probability, so this clamp is the stage's definition of how a magnitude maps to risk, and it is
//! pinned by a test. A strength of zero scales the risk to zero, which a directional signal cannot
//! be sized from, so it is reported (see below).
//!
//! # Weight
//!
//! A constructed target carries a weight: the notional of the sized quantity at the entry price as
//! a fraction of the context equity, signed by the direction and capped in magnitude by
//! [`TargetConstructionConfig::max_weight`]. Where the instrument's granularity does not constrain
//! the size, the weight is `risk / (stop_loss_bps / 10_000)`, so the stop distance alone decides
//! how much exposure a given risk buys, and the cap is what keeps a tight stop from asking for
//! more exposure than the caller allows.
//!
//! # Degenerate inputs
//!
//! A construction step reports a typed error rather than silently omitting an input. A caller
//! cannot otherwise tell a dropped signal from a signal that carries no view, and a dropped
//! reduce-to-zero instruction would leave an unwanted position open: an omission would also turn a
//! directional signal whose size floors to zero into a flat target, which instructs the opposite
//! of the signal. The cases are:
//!
//! - the context holds no instrument definition for the signal's instrument,
//!   [`TargetConstructionError::UnknownInstrument`];
//! - the context holds no price for the signal's instrument,
//!   [`TargetConstructionError::MissingPrice`];
//! - a directional signal cannot be resolved to a positive exposure, because the effective risk is
//!   zero, the context equity or price is not positive, the derived stop price is not representable
//!   at the instrument's precision, or the sized quantity is zero,
//!   [`TargetConstructionError::UnsizeableSignal`] and [`TargetConstructionError::StopPrice`];
//! - the delegated sizing calculation fails, [`TargetConstructionError::Sizing`].
//!
//! A caller that wants to skip an instrument it cannot act on filters its own signals before
//! calling the stage. A flat signal is the one input that needs no context, so a caller can always
//! state reduce-to-zero for an instrument the stage cannot size.
//!
//! # Determinism
//!
//! The stage reads no clock and no shared state. For equal signals and an equal context it returns
//! equal targets in the same order, and it mutates neither input; the first instrument definition
//! and the first price for an identifier win when a context repeats one.

use std::fmt::Display;

use nautilus_model::{
    identifiers::InstrumentId,
    instruments::{Instrument, InstrumentAny},
    signal::{SignalDirection, TradingSignal},
    target::Target,
    types::{Money, Price, Quantity},
};
use nautilus_risk::sizing::calculate_fixed_risk_position_size;
use rust_decimal::{
    Decimal,
    prelude::{FromPrimitive, ToPrimitive},
};

/// The upper bound of `stop_loss_bps`, which is a stop distance of the whole entry price.
const MAX_STOP_LOSS_BPS: u32 = 10_000;

/// The number of basis points in one unit of price, which converts a basis point distance to a
/// fraction of the entry price.
const BASIS_POINTS_PER_UNIT: u32 = 10_000;

/// The configuration of a [`TargetConstruction`] stage.
///
/// Every field is validated by [`Self::validate`], and the values are public so a configuration can
/// also be built as plain data. The stage validates again when it is constructed, so a
/// configuration built literally cannot reach a construction step unvalidated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TargetConstructionConfig {
    /// The fraction of equity risked per constructed position.
    ///
    /// This is the base risk a signal's `strength` scales, in the interval `(0, 1]`: risking more
    /// than the whole account in one position is not a fixed-risk allocation.
    pub risk_per_trade: Decimal,
    /// The stop distance used to derive a position size from an entry price, in basis points of
    /// the entry price.
    ///
    /// This is in the interval `[1, 10_000]` basis points, so the derived stop is always a real
    /// distance from the entry and never beyond the entry going to zero.
    pub stop_loss_bps: u32,
    /// The cap on the magnitude of a constructed target weight.
    ///
    /// This must be positive. There is no upper bound, because a weight above one expresses
    /// leverage, which the target value documents as representable.
    pub max_weight: Decimal,
    /// The commission rate passed through to the sizing calculation.
    ///
    /// This is in the interval `[0, 1/2)`: the sizing calculation charges a round-trip commission
    /// of twice the rate against the risk budget, so a rate of one half or more consumes the whole
    /// budget and can only ever size a position of zero.
    pub commission_rate: Decimal,
}

impl TargetConstructionConfig {
    /// Creates a new [`TargetConstructionConfig`].
    ///
    /// # Errors
    ///
    /// Returns a [`TargetConstructionError::InvalidConfig`] if any value is outside its documented
    /// domain:
    ///
    /// - `risk_per_trade` must be positive and at most one;
    /// - `stop_loss_bps` must be between 1 and 10,000 basis points;
    /// - `max_weight` must be positive;
    /// - `commission_rate` must be non-negative and below one half.
    pub fn new(
        risk_per_trade: Decimal,
        stop_loss_bps: u32,
        max_weight: Decimal,
        commission_rate: Decimal,
    ) -> Result<Self, TargetConstructionError> {
        let config = Self {
            risk_per_trade,
            stop_loss_bps,
            max_weight,
            commission_rate,
        };
        config.validate()?;
        Ok(config)
    }

    /// Validates every value against its documented domain.
    ///
    /// # Errors
    ///
    /// Returns a [`TargetConstructionError::InvalidConfig`] naming the first rejected field and
    /// the rule it violates.
    pub fn validate(&self) -> Result<(), TargetConstructionError> {
        if self.risk_per_trade <= Decimal::ZERO || self.risk_per_trade > Decimal::ONE {
            return Err(Self::invalid(
                "risk_per_trade",
                "must be positive and at most one",
                self.risk_per_trade.to_string(),
            ));
        }

        if !(1..=MAX_STOP_LOSS_BPS).contains(&self.stop_loss_bps) {
            return Err(Self::invalid(
                "stop_loss_bps",
                "must be between 1 and 10000 basis points",
                self.stop_loss_bps.to_string(),
            ));
        }

        if self.max_weight <= Decimal::ZERO {
            return Err(Self::invalid(
                "max_weight",
                "must be positive",
                self.max_weight.to_string(),
            ));
        }

        let round_trip = self
            .commission_rate
            .checked_mul(Decimal::TWO)
            .is_some_and(|rate| rate < Decimal::ONE);
        if self.commission_rate < Decimal::ZERO || !round_trip {
            return Err(Self::invalid(
                "commission_rate",
                "must be non-negative and below one half, the rate at which the round-trip \
                 commission consumes the risk budget",
                self.commission_rate.to_string(),
            ));
        }

        Ok(())
    }

    /// Returns the basis point distance as a fraction of the entry price.
    ///
    /// This is the only conversion of `stop_loss_bps` the stage performs, and it is exact for
    /// every basis point value in the accepted range.
    fn stop_loss_fraction(&self) -> Decimal {
        Decimal::from(self.stop_loss_bps) / Decimal::from(BASIS_POINTS_PER_UNIT)
    }

    /// Returns an invalid configuration error for `field`.
    fn invalid(
        field: &'static str,
        reason: &'static str,
        value: String,
    ) -> TargetConstructionError {
        TargetConstructionError::InvalidConfig {
            field,
            reason,
            value,
        }
    }
}

/// The portfolio context a [`TargetConstruction`] step resolves signals against.
///
/// The context is plain data, so the step is unit testable without an engine. It is supplied by the
/// caller and never read from or written back to the cache or the portfolio: `equity` is the
/// account equity the risk is taken from, `instruments` are the definitions a signal needs to be
/// sized, and `prices` are the entry prices.
///
/// `positions` is the caller's position state, carried for the stages that consume targets. A
/// construction step reads no position state, because a target states a desired exposure rather
/// than a change to the current one.
#[derive(Clone, Debug, PartialEq)]
pub struct TargetConstructionContext {
    /// The account equity the risk is taken from.
    pub equity: Money,
    /// The instrument definitions available to the step.
    pub instruments: Vec<InstrumentAny>,
    /// The entry prices available to the step.
    pub prices: Vec<(InstrumentId, Price)>,
    /// The caller's position state.
    pub positions: Vec<(InstrumentId, Quantity)>,
}

impl TargetConstructionContext {
    /// Returns the instrument definition for `instrument_id`, if the context holds one.
    ///
    /// The first match wins, so a context that repeats an identifier resolves deterministically.
    #[must_use]
    pub fn instrument(&self, instrument_id: InstrumentId) -> Option<&InstrumentAny> {
        self.instruments
            .iter()
            .find(|instrument| instrument.id() == instrument_id)
    }

    /// Returns the entry price for `instrument_id`, if the context holds one.
    ///
    /// The first match wins, so a context that repeats an identifier resolves deterministically.
    #[must_use]
    pub fn price(&self, instrument_id: InstrumentId) -> Option<Price> {
        self.prices
            .iter()
            .find(|(id, _)| *id == instrument_id)
            .map(|(_, price)| *price)
    }
}

/// The reason a target construction stage rejected a configuration or a signal.
///
/// Every rejected input is reported rather than omitted, so a caller can tell which signal was not
/// resolved and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TargetConstructionError {
    /// A configuration value is outside its documented domain.
    InvalidConfig {
        /// The configuration field that was rejected.
        field: &'static str,
        /// The rule the rejected value violates.
        reason: &'static str,
        /// The rejected value.
        value: String,
    },
    /// The context holds no instrument definition for the signal's instrument.
    UnknownInstrument {
        /// The instrument the signal names.
        instrument_id: InstrumentId,
    },
    /// The context holds no price for the signal's instrument.
    MissingPrice {
        /// The instrument the signal names.
        instrument_id: InstrumentId,
    },
    /// The stop price derived from the entry price is not representable at the instrument's
    /// precision.
    StopPrice {
        /// The instrument the signal names.
        instrument_id: InstrumentId,
        /// The underlying failure.
        reason: String,
    },
    /// The delegated sizing calculation failed.
    Sizing {
        /// The instrument the signal names.
        instrument_id: InstrumentId,
        /// The underlying failure.
        reason: String,
    },
    /// A directional signal cannot be resolved to a positive exposure.
    UnsizeableSignal {
        /// The instrument the signal names.
        instrument_id: InstrumentId,
        /// Why the signal cannot be resolved.
        reason: &'static str,
    },
}

impl Display for TargetConstructionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidConfig {
                field,
                reason,
                value,
            } => write!(
                f,
                "invalid target construction config `{field}`: {reason}, was {value}"
            ),
            Self::UnknownInstrument { instrument_id } => write!(
                f,
                "no instrument definition for {instrument_id} in the construction context"
            ),
            Self::MissingPrice { instrument_id } => {
                write!(
                    f,
                    "no price for {instrument_id} in the construction context"
                )
            }
            Self::StopPrice {
                instrument_id,
                reason,
            } => write!(
                f,
                "cannot derive a stop price for {instrument_id}: {reason}"
            ),
            Self::Sizing {
                instrument_id,
                reason,
            } => write!(f, "position sizing failed for {instrument_id}: {reason}"),
            Self::UnsizeableSignal {
                instrument_id,
                reason,
            } => write!(
                f,
                "signal for {instrument_id} cannot be resolved to an exposure: {reason}"
            ),
        }
    }
}

impl std::error::Error for TargetConstructionError {}

/// The optional target construction stage.
///
/// The stage resolves each signal to a desired exposure using the portfolio context the caller
/// supplies, and returns one target per signal in the order the signals are given. It delegates
/// sizing to the fixed-risk sizing calculation in `nautilus_risk::sizing` rather than implementing
/// a second sizing convention.
///
/// The stage holds configuration only: it is not registered on an engine or a strategy, it reads
/// neither the cache nor the portfolio, and it submits, cancels, or modifies no order.
#[derive(Clone, Debug)]
pub struct TargetConstruction {
    config: TargetConstructionConfig,
}

impl TargetConstruction {
    /// Creates a new [`TargetConstruction`] stage.
    ///
    /// # Errors
    ///
    /// Returns a [`TargetConstructionError::InvalidConfig`] if a configuration value is outside
    /// its documented domain, so a configuration built as plain data cannot reach a construction
    /// step unvalidated.
    pub fn new(config: TargetConstructionConfig) -> Result<Self, TargetConstructionError> {
        config.validate()?;
        Ok(Self { config })
    }

    /// Returns the stage configuration.
    #[must_use]
    pub const fn config(&self) -> &TargetConstructionConfig {
        &self.config
    }

    /// Constructs one target per signal, in the order the signals are given.
    ///
    /// # Errors
    ///
    /// Returns an error if a signal cannot be resolved, naming the instrument:
    ///
    /// - a directional signal whose instrument the context does not define,
    ///   [`TargetConstructionError::UnknownInstrument`];
    /// - a directional signal whose instrument has no price in the context,
    ///   [`TargetConstructionError::MissingPrice`];
    /// - a directional signal that cannot be sized to a positive quantity or whose target weight
    ///   cannot be represented, [`TargetConstructionError::UnsizeableSignal`];
    /// - a directional signal whose derived stop price is not representable at the instrument's
    ///   precision, [`TargetConstructionError::StopPrice`];
    /// - a delegated sizing calculation that fails, [`TargetConstructionError::Sizing`].
    ///
    /// A flat signal needs no context and always resolves, so a caller can always state
    /// reduce-to-zero for an instrument the stage cannot size.
    pub fn construct(
        &self,
        signals: &[TradingSignal],
        context: &TargetConstructionContext,
    ) -> Result<Vec<Target>, TargetConstructionError> {
        let mut targets = Vec::with_capacity(signals.len());

        for signal in signals {
            targets.push(self.construct_one(signal, context)?);
        }

        Ok(targets)
    }

    fn construct_one(
        &self,
        signal: &TradingSignal,
        context: &TargetConstructionContext,
    ) -> Result<Target, TargetConstructionError> {
        let instrument_id = signal.instrument_id();
        let direction = signal.direction();

        // A flat signal states no exposure, so it needs no instrument and no price.
        if direction == SignalDirection::Flat {
            return target(instrument_id, 0.0, signal);
        }

        let instrument = context
            .instrument(instrument_id)
            .ok_or(TargetConstructionError::UnknownInstrument { instrument_id })?;
        let entry = context
            .price(instrument_id)
            .ok_or(TargetConstructionError::MissingPrice { instrument_id })?;

        let equity = context.equity.as_decimal();
        if equity <= Decimal::ZERO {
            return Err(TargetConstructionError::UnsizeableSignal {
                instrument_id,
                reason: "the context equity is not positive",
            });
        }
        if entry.as_decimal() <= Decimal::ZERO {
            return Err(TargetConstructionError::UnsizeableSignal {
                instrument_id,
                reason: "the context price is not positive",
            });
        }

        let risk = self.effective_risk(signal)?;
        let stop_loss = self.stop_price(instrument, entry, direction)?;

        let quantity = calculate_fixed_risk_position_size(
            instrument,
            entry,
            stop_loss,
            context.equity,
            risk,
            self.config.commission_rate,
            Decimal::ONE,
            None,
            instrument.size_increment().as_decimal(),
            1,
        )
        .map_err(|e| TargetConstructionError::Sizing {
            instrument_id,
            reason: e.to_string(),
        })?;

        if quantity.is_zero() {
            return Err(TargetConstructionError::UnsizeableSignal {
                instrument_id,
                reason: "the sized quantity is zero",
            });
        }

        let magnitude = self.weight(quantity, entry, equity, instrument_id)?;
        let weight = match direction {
            SignalDirection::Long => magnitude,
            SignalDirection::Short => -magnitude,
            SignalDirection::Flat => 0.0,
        };

        target(instrument_id, weight, signal)
    }

    /// Returns the risk the position is sized from, scaled by the signal's strength.
    fn effective_risk(&self, signal: &TradingSignal) -> Result<Decimal, TargetConstructionError> {
        let instrument_id = signal.instrument_id();
        let factor = match signal.strength() {
            None => 1.0,
            Some(strength) => strength.clamp(0.0, 1.0),
        };
        let factor =
            Decimal::from_f64(factor).ok_or(TargetConstructionError::UnsizeableSignal {
                instrument_id,
                reason: "the signal strength is not a representable decimal",
            })?;
        let risk = self.config.risk_per_trade.checked_mul(factor).ok_or(
            TargetConstructionError::UnsizeableSignal {
                instrument_id,
                reason: "the effective risk overflows decimal arithmetic",
            },
        )?;

        if risk <= Decimal::ZERO {
            return Err(TargetConstructionError::UnsizeableSignal {
                instrument_id,
                reason: "the effective risk is not positive",
            });
        }

        Ok(risk)
    }

    /// Returns the stop price derived from the entry price for the signal's direction.
    ///
    /// The stop is below the entry for a long and above it for a short, rounded to the
    /// instrument's price precision. The sizing calculation can only observe the distance, so this
    /// direction is pinned by a test of its own.
    fn stop_price(
        &self,
        instrument: &InstrumentAny,
        entry: Price,
        direction: SignalDirection,
    ) -> Result<Price, TargetConstructionError> {
        let instrument_id = instrument.id();
        let distance = entry
            .as_decimal()
            .checked_mul(self.config.stop_loss_fraction())
            .ok_or(TargetConstructionError::UnsizeableSignal {
                instrument_id,
                reason: "the stop distance overflows decimal arithmetic",
            })?;
        let raw = match direction {
            SignalDirection::Long => entry.as_decimal().checked_sub(distance),
            SignalDirection::Short => entry.as_decimal().checked_add(distance),
            SignalDirection::Flat => {
                return Err(TargetConstructionError::UnsizeableSignal {
                    instrument_id,
                    reason: "a flat signal has no stop price",
                });
            }
        }
        .ok_or(TargetConstructionError::UnsizeableSignal {
            instrument_id,
            reason: "the derived stop price overflows decimal arithmetic",
        })?;

        instrument.try_make_price_from_decimal(raw).map_err(|e| {
            TargetConstructionError::StopPrice {
                instrument_id,
                reason: e.to_string(),
            }
        })
    }

    /// Returns the weight of a sized quantity, capped in magnitude by `max_weight`.
    fn weight(
        &self,
        quantity: Quantity,
        entry: Price,
        equity: Decimal,
        instrument_id: InstrumentId,
    ) -> Result<f64, TargetConstructionError> {
        let notional = quantity
            .as_decimal()
            .checked_mul(entry.as_decimal())
            .ok_or(TargetConstructionError::UnsizeableSignal {
                instrument_id,
                reason: "the position notional overflows decimal arithmetic",
            })?;
        let weight =
            notional
                .checked_div(equity)
                .ok_or(TargetConstructionError::UnsizeableSignal {
                    instrument_id,
                    reason: "the target weight overflows decimal arithmetic",
                })?;

        weight.min(self.config.max_weight).to_f64().ok_or(
            TargetConstructionError::UnsizeableSignal {
                instrument_id,
                reason: "the target weight is not representable as a float",
            },
        )
    }
}

/// Returns the target stating a weight, stamped with the signal's own instants.
fn target(
    instrument_id: InstrumentId,
    weight: f64,
    signal: &TradingSignal,
) -> Result<Target, TargetConstructionError> {
    Target::from_weight(instrument_id, weight, signal.ts_event(), signal.ts_init()).map_err(|_| {
        TargetConstructionError::UnsizeableSignal {
            instrument_id,
            reason: "the target weight is not finite",
        }
    })
}

#[cfg(test)]
mod tests {
    use nautilus_core::UnixNanos;
    use nautilus_model::{
        identifiers::InstrumentId,
        instruments::stubs::equity_aapl,
        signal::{SignalDirection, TradingSignal},
        target::Target,
        types::{Currency, Money, Price},
    };
    use rstest::{fixture, rstest};
    use rust_decimal_macros::dec;

    use super::*;

    /// The account equity the fixtures size against.
    const EQUITY: f64 = 100_000.0;

    /// The AAPL equity price the fixtures size from, at a two decimal precision.
    const ENTRY: &str = "100.00";

    #[fixture]
    fn instrument() -> InstrumentAny {
        InstrumentAny::Equity(equity_aapl())
    }

    #[fixture]
    fn instrument_id() -> InstrumentId {
        InstrumentId::from("AAPL.XNAS")
    }

    #[fixture]
    fn context(
        instrument: InstrumentAny,
        instrument_id: InstrumentId,
    ) -> TargetConstructionContext {
        TargetConstructionContext {
            equity: Money::new(EQUITY, Currency::USD()),
            instruments: vec![instrument],
            prices: vec![(instrument_id, Price::from(ENTRY))],
            positions: Vec::new(),
        }
    }

    /// A stage risking one per cent of equity with a one per cent stop and no commission.
    #[fixture]
    fn config() -> TargetConstructionConfig {
        TargetConstructionConfig::new(dec!(0.01), 100, dec!(1), Decimal::ZERO).unwrap()
    }

    #[fixture]
    fn stage(config: TargetConstructionConfig) -> TargetConstruction {
        TargetConstruction::new(config).unwrap()
    }

    fn signal(
        instrument_id: InstrumentId,
        direction: SignalDirection,
        strength: Option<f64>,
    ) -> TradingSignal {
        TradingSignal::new(
            instrument_id,
            direction,
            None,
            strength,
            None,
            None,
            None,
            1_000.into(),
            2_000.into(),
        )
        .unwrap()
    }

    fn weights(targets: &[Target]) -> Vec<Option<f64>> {
        targets
            .iter()
            .map(|target| target.value().weight())
            .collect()
    }

    fn weight(targets: &[Target]) -> Option<f64> {
        targets.first().and_then(|target| target.value().weight())
    }

    #[rstest]
    fn test_long_signal_constructs_a_positive_weight(
        stage: TargetConstruction,
        context: TargetConstructionContext,
        instrument_id: InstrumentId,
    ) {
        let targets = stage
            .construct(
                &[signal(instrument_id, SignalDirection::Long, None)],
                &context,
            )
            .unwrap();

        assert_eq!(targets.len(), 1);
        let target = &targets[0];
        assert_eq!(target.instrument_id(), instrument_id);
        assert_eq!(target.kind(), "WEIGHT");
        assert_eq!(target.value().weight(), Some(1.0));
        assert!(!target.is_flat());
        assert_eq!(target.ts_event(), UnixNanos::from(1_000_u64));
        assert_eq!(target.ts_init(), UnixNanos::from(2_000_u64));
    }

    #[rstest]
    fn test_short_signal_constructs_a_negative_weight(
        stage: TargetConstruction,
        context: TargetConstructionContext,
        instrument_id: InstrumentId,
    ) {
        let targets = stage
            .construct(
                &[signal(instrument_id, SignalDirection::Short, None)],
                &context,
            )
            .unwrap();

        assert_eq!(weight(&targets), Some(-1.0));
        assert!(!targets[0].is_flat());
    }

    #[rstest]
    fn test_flat_signal_constructs_a_flat_target_without_a_context(
        stage: TargetConstruction,
        instrument_id: InstrumentId,
    ) {
        // A flat signal states reduce-to-zero, so it resolves even when the context carries no
        // instrument definition, no price, and no equity to size from.
        let context = TargetConstructionContext {
            equity: Money::new(0.0, Currency::USD()),
            instruments: Vec::new(),
            prices: Vec::new(),
            positions: Vec::new(),
        };

        let targets = stage
            .construct(
                &[signal(instrument_id, SignalDirection::Flat, None)],
                &context,
            )
            .unwrap();

        assert_eq!(targets.len(), 1);
        assert!(targets[0].is_flat());
        assert_eq!(targets[0].value().weight(), Some(0.0));
    }

    #[rstest]
    fn test_strength_scales_the_risk(
        stage: TargetConstruction,
        context: TargetConstructionContext,
        instrument_id: InstrumentId,
    ) {
        let absent = stage
            .construct(
                &[signal(instrument_id, SignalDirection::Long, None)],
                &context,
            )
            .unwrap();
        let full = stage
            .construct(
                &[signal(instrument_id, SignalDirection::Long, Some(1.0))],
                &context,
            )
            .unwrap();
        let half = stage
            .construct(
                &[signal(instrument_id, SignalDirection::Long, Some(0.5))],
                &context,
            )
            .unwrap();
        let above = stage
            .construct(
                &[signal(instrument_id, SignalDirection::Long, Some(2.0))],
                &context,
            )
            .unwrap();

        // An absent strength uses the configured risk, which a one per cent stop against a one per
        // cent risk turns into a full-equity weight. Half the strength risks half as much, and a
        // strength above one is clamped to the configured risk.
        assert_eq!(weight(&absent), Some(1.0));
        assert_eq!(weight(&full), Some(1.0));
        assert_eq!(weight(&half), Some(0.5));
        assert_eq!(weight(&above), Some(1.0));
        assert_eq!(absent, full);
        assert_eq!(absent, above);
    }

    #[rstest]
    fn test_zero_strength_is_reported(
        stage: TargetConstruction,
        context: TargetConstructionContext,
        instrument_id: InstrumentId,
    ) {
        let error = stage
            .construct(
                &[signal(instrument_id, SignalDirection::Long, Some(0.0))],
                &context,
            )
            .unwrap_err();

        assert_eq!(
            error,
            TargetConstructionError::UnsizeableSignal {
                instrument_id,
                reason: "the effective risk is not positive",
            }
        );
    }

    #[rstest]
    fn test_max_weight_caps_the_constructed_weight(
        context: TargetConstructionContext,
        instrument_id: InstrumentId,
    ) {
        let long = |stage: &TargetConstruction| {
            stage
                .construct(
                    &[signal(instrument_id, SignalDirection::Long, None)],
                    &context,
                )
                .unwrap()
        };
        let capped = TargetConstruction::new(
            TargetConstructionConfig::new(dec!(0.01), 100, dec!(0.25), Decimal::ZERO).unwrap(),
        )
        .unwrap();
        let uncapped = TargetConstruction::new(config()).unwrap();

        assert_eq!(weight(&long(&capped)), Some(0.25));
        assert_eq!(weight(&long(&uncapped)), Some(1.0));

        let short = capped
            .construct(
                &[signal(instrument_id, SignalDirection::Short, None)],
                &context,
            )
            .unwrap();
        assert_eq!(weight(&short), Some(-0.25));
    }

    #[rstest]
    fn test_missing_instrument_is_reported(stage: TargetConstruction, instrument_id: InstrumentId) {
        let context = TargetConstructionContext {
            equity: Money::new(EQUITY, Currency::USD()),
            instruments: Vec::new(),
            prices: vec![(instrument_id, Price::from(ENTRY))],
            positions: Vec::new(),
        };

        let error = stage
            .construct(
                &[signal(instrument_id, SignalDirection::Long, None)],
                &context,
            )
            .unwrap_err();

        assert_eq!(
            error,
            TargetConstructionError::UnknownInstrument { instrument_id }
        );
    }

    #[rstest]
    fn test_missing_price_is_reported(
        stage: TargetConstruction,
        instrument: InstrumentAny,
        instrument_id: InstrumentId,
    ) {
        let context = TargetConstructionContext {
            equity: Money::new(EQUITY, Currency::USD()),
            instruments: vec![instrument],
            prices: Vec::new(),
            positions: Vec::new(),
        };

        let error = stage
            .construct(
                &[signal(instrument_id, SignalDirection::Long, None)],
                &context,
            )
            .unwrap_err();

        assert_eq!(
            error,
            TargetConstructionError::MissingPrice { instrument_id }
        );
    }

    #[rstest]
    fn test_zero_sized_result_is_reported(
        stage: TargetConstruction,
        instrument: InstrumentAny,
        instrument_id: InstrumentId,
    ) {
        // An equity of 50 sizes half a unit at a one per cent stop, which the unit size increment
        // floors to zero rather than to a directional exposure.
        let context = TargetConstructionContext {
            equity: Money::new(50.0, Currency::USD()),
            instruments: vec![instrument],
            prices: vec![(instrument_id, Price::from(ENTRY))],
            positions: Vec::new(),
        };

        let error = stage
            .construct(
                &[signal(instrument_id, SignalDirection::Long, None)],
                &context,
            )
            .unwrap_err();

        assert_eq!(
            error,
            TargetConstructionError::UnsizeableSignal {
                instrument_id,
                reason: "the sized quantity is zero",
            }
        );
    }

    #[rstest]
    fn test_construction_is_deterministic_and_mutates_nothing(
        stage: TargetConstruction,
        context: TargetConstructionContext,
        instrument_id: InstrumentId,
    ) {
        let signals = vec![
            signal(instrument_id, SignalDirection::Long, None),
            signal(instrument_id, SignalDirection::Short, Some(0.5)),
            signal(instrument_id, SignalDirection::Flat, None),
        ];
        let snapshot = context.clone();

        let first = stage.construct(&signals, &context).unwrap();
        let second = stage.construct(&signals, &context).unwrap();

        assert_eq!(first, second);
        assert_eq!(context, snapshot);
        assert_eq!(first.len(), signals.len());
    }

    #[rstest]
    fn test_signals_resolve_in_input_order(
        stage: TargetConstruction,
        context: TargetConstructionContext,
        instrument_id: InstrumentId,
    ) {
        // A repeated instrument resolves once per signal, in the order the signals are given.
        let signals = vec![
            signal(instrument_id, SignalDirection::Long, Some(0.5)),
            signal(instrument_id, SignalDirection::Short, None),
            signal(instrument_id, SignalDirection::Flat, None),
        ];

        let targets = stage.construct(&signals, &context).unwrap();

        assert_eq!(weights(&targets), vec![Some(0.5), Some(-1.0), Some(0.0)]);
    }

    #[rstest]
    fn test_commission_rate_reduces_the_size(
        context: TargetConstructionContext,
        instrument_id: InstrumentId,
    ) {
        // The sizing charges a round-trip commission of twice the rate, so a rate of one quarter
        // halves the risk and the weight.
        let stage = TargetConstruction::new(
            TargetConstructionConfig::new(dec!(0.01), 100, dec!(1), dec!(0.25)).unwrap(),
        )
        .unwrap();

        let targets = stage
            .construct(
                &[signal(instrument_id, SignalDirection::Long, None)],
                &context,
            )
            .unwrap();

        assert_eq!(weight(&targets), Some(0.5));
    }

    #[rstest]
    fn test_stop_price_is_derived_for_the_direction(
        stage: TargetConstruction,
        instrument: InstrumentAny,
    ) {
        let entry = Price::from(ENTRY);

        assert_eq!(
            stage
                .stop_price(&instrument, entry, SignalDirection::Long)
                .unwrap(),
            Price::from("99.00")
        );
        assert_eq!(
            stage
                .stop_price(&instrument, entry, SignalDirection::Short)
                .unwrap(),
            Price::from("101.00")
        );
    }

    #[rstest]
    fn test_stop_price_is_reported_for_a_flat_signal(
        stage: TargetConstruction,
        instrument: InstrumentAny,
        instrument_id: InstrumentId,
    ) {
        let error = stage
            .stop_price(&instrument, Price::from(ENTRY), SignalDirection::Flat)
            .unwrap_err();

        assert_eq!(
            error,
            TargetConstructionError::UnsizeableSignal {
                instrument_id,
                reason: "a flat signal has no stop price",
            }
        );
    }

    #[rstest]
    fn test_stop_price_is_derived_from_the_entry_price(instrument: InstrumentAny) {
        // A half per cent stop distance on a two decimal instrument price rounds to the
        // instrument's price precision.
        let stage = TargetConstruction::new(
            TargetConstructionConfig::new(dec!(0.01), 50, dec!(1), Decimal::ZERO).unwrap(),
        )
        .unwrap();

        assert_eq!(
            stage
                .stop_price(&instrument, Price::from(ENTRY), SignalDirection::Long)
                .unwrap(),
            Price::from("99.50")
        );
        assert_eq!(
            stage
                .stop_price(&instrument, Price::from(ENTRY), SignalDirection::Short)
                .unwrap(),
            Price::from("100.50")
        );
    }

    #[rstest]
    #[case(dec!(0), 100, dec!(1), dec!(0), "risk_per_trade")]
    #[case(dec!(-0.01), 100, dec!(1), dec!(0), "risk_per_trade")]
    #[case(dec!(1.01), 100, dec!(1), dec!(0), "risk_per_trade")]
    #[case(dec!(0.01), 0, dec!(1), dec!(0), "stop_loss_bps")]
    #[case(dec!(0.01), 10_001, dec!(1), dec!(0), "stop_loss_bps")]
    #[case(dec!(0.01), 100, dec!(0), dec!(0), "max_weight")]
    #[case(dec!(0.01), 100, dec!(-1), dec!(0), "max_weight")]
    #[case(dec!(0.01), 100, dec!(1), dec!(-0.01), "commission_rate")]
    #[case(dec!(0.01), 100, dec!(1), dec!(0.5), "commission_rate")]
    fn test_rejects_an_invalid_config(
        #[case] risk_per_trade: Decimal,
        #[case] stop_loss_bps: u32,
        #[case] max_weight: Decimal,
        #[case] commission_rate: Decimal,
        #[case] field: &str,
    ) {
        let error = TargetConstructionConfig::new(
            risk_per_trade,
            stop_loss_bps,
            max_weight,
            commission_rate,
        )
        .unwrap_err();

        match error {
            TargetConstructionError::InvalidConfig {
                field: rejected,
                value,
                ..
            } => {
                assert_eq!(rejected, field);
                assert!(!value.is_empty());
            }
            other => panic!("expected an invalid config error, was {other}"),
        }
    }

    #[rstest]
    #[case(dec!(0.01), 1, dec!(1), dec!(0))]
    #[case(dec!(1), 10_000, dec!(1), dec!(0.49))]
    fn test_accepts_the_documented_config_bounds(
        #[case] risk_per_trade: Decimal,
        #[case] stop_loss_bps: u32,
        #[case] max_weight: Decimal,
        #[case] commission_rate: Decimal,
    ) {
        assert!(
            TargetConstructionConfig::new(
                risk_per_trade,
                stop_loss_bps,
                max_weight,
                commission_rate
            )
            .is_ok()
        );
    }

    #[rstest]
    fn test_stage_rejects_a_config_built_as_plain_data() {
        // The values are public, so the stage validates again rather than trusting the caller.
        let config = TargetConstructionConfig {
            risk_per_trade: dec!(0),
            stop_loss_bps: 100,
            max_weight: dec!(1),
            commission_rate: Decimal::ZERO,
        };

        assert!(TargetConstruction::new(config).is_err());
    }
}
