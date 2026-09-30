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

//! Trading signal values.
//!
//! A signal states a view: which instrument, in which direction, over what horizon, with what
//! magnitude, from which source, and until when. It is a statement of opinion, not a trading
//! command: it carries no quantity, no price, and no execution instruction. Converting a view into
//! a desired exposure is the job of a target, and converting a target into orders is the job of
//! the order layer. The cache and the portfolio remain the single source of truth for position
//! state; a signal never owns position intent.
//!
//! This type is distinct from the generic `Signal { name, value, ts_event, ts_init }` defined by
//! the common crate, which is a component-facing notification value. The two are not
//! interchangeable, and no conversion exists between them.

use std::fmt::Display;

use anyhow::{Result, ensure};
use nautilus_core::{Params, UnixNanos};
use ustr::Ustr;

use crate::identifiers::InstrumentId;

/// The direction of a trading signal.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    strum::Display,
    strum::AsRefStr,
    strum::EnumString,
    strum::FromRepr,
    strum::EnumIter,
)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(
        frozen,
        eq,
        eq_int,
        module = "nautilus_trader.model",
        from_py_object,
        rename_all = "SCREAMING_SNAKE_CASE",
    )
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass_enum(module = "nautilus_trader.model")
)]
pub enum SignalDirection {
    /// A view that the instrument will rise, calling for long exposure.
    Long = 1,
    /// A view that the instrument will fall, calling for short exposure.
    Short = 2,
    /// A view that the instrument will not move, calling for no exposure.
    Flat = 3,
}

impl SignalDirection {
    /// Returns the canonical string representation.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.as_ref()
    }
}

/// A statement of view about one instrument.
///
/// The direction and horizon describe the view, `strength` is its magnitude, and `source` names
/// the component or model that emitted it. `expiry_ns` bounds how long the view is valid for, and
/// `provenance` carries free-form origin detail. A signal is not a trading command and carries no
/// order quantity: it never belongs to the order or position layers.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.model", from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.model")
)]
pub struct TradingSignal {
    instrument_id: InstrumentId,
    direction: SignalDirection,
    horizon_ns: Option<u64>,
    strength: Option<f64>,
    source: Option<Ustr>,
    expiry_ns: Option<UnixNanos>,
    provenance: Option<Params>,
    ts_event: UnixNanos,
    ts_init: UnixNanos,
}

impl TradingSignal {
    /// Creates a new [`TradingSignal`].
    ///
    /// # Errors
    ///
    /// Returns an error if a strength is present and is not finite or is negative, if a horizon
    /// is present and is zero, or if an expiry is present and is before `ts_event`.
    #[expect(
        clippy::too_many_arguments,
        reason = "a signal carries its full statement of view as one value"
    )]
    pub fn new(
        instrument_id: InstrumentId,
        direction: SignalDirection,
        horizon_ns: Option<u64>,
        strength: Option<f64>,
        source: Option<Ustr>,
        expiry_ns: Option<UnixNanos>,
        provenance: Option<Params>,
        ts_event: UnixNanos,
        ts_init: UnixNanos,
    ) -> Result<Self> {
        if let Some(strength) = strength {
            ensure!(
                strength.is_finite(),
                "strength {strength} is not finite; strength is a magnitude"
            );
            ensure!(
                strength >= 0.0,
                "strength {strength} is negative; strength is a magnitude"
            );
        }

        if let Some(horizon_ns) = horizon_ns {
            ensure!(
                horizon_ns > 0,
                "horizon_ns is zero; a horizon must be a positive duration"
            );
        }

        if let Some(expiry_ns) = expiry_ns {
            ensure!(
                expiry_ns >= ts_event,
                "expiry_ns {expiry_ns} is before ts_event {ts_event}"
            );
        }

        Ok(Self {
            instrument_id,
            direction,
            horizon_ns,
            strength,
            source,
            expiry_ns,
            provenance,
            ts_event,
            ts_init,
        })
    }

    /// Returns the instrument the view is about.
    #[must_use]
    pub const fn instrument_id(&self) -> InstrumentId {
        self.instrument_id
    }

    /// Returns the direction of the view.
    #[must_use]
    pub const fn direction(&self) -> SignalDirection {
        self.direction
    }

    /// Returns the expected holding horizon (nanoseconds), if any.
    #[must_use]
    pub const fn horizon_ns(&self) -> Option<u64> {
        self.horizon_ns
    }

    /// Returns the magnitude of the view, if any.
    ///
    /// This is a magnitude, not a probability: it is not bounded by one and does not sum to one
    /// across the signals of a portfolio.
    #[must_use]
    pub const fn strength(&self) -> Option<f64> {
        self.strength
    }

    /// Returns the emitting component or model, if any.
    #[must_use]
    pub const fn source(&self) -> Option<Ustr> {
        self.source
    }

    /// Returns when the view lapses, if it does.
    #[must_use]
    pub const fn expiry_ns(&self) -> Option<UnixNanos> {
        self.expiry_ns
    }

    /// Returns the free-form origin detail, if any.
    #[must_use]
    pub const fn provenance(&self) -> Option<&Params> {
        self.provenance.as_ref()
    }

    /// Returns the instant the view was stated.
    #[must_use]
    pub const fn ts_event(&self) -> UnixNanos {
        self.ts_event
    }

    /// Returns the instant the instance was created.
    #[must_use]
    pub const fn ts_init(&self) -> UnixNanos {
        self.ts_init
    }

    /// Returns whether the view has lapsed at the given instant.
    ///
    /// A signal with no expiry never lapses.
    #[must_use]
    pub fn is_expired(&self, at: UnixNanos) -> bool {
        self.expiry_ns.is_some_and(|expiry_ns| expiry_ns <= at)
    }
}

impl Display for TradingSignal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TradingSignal(instrument_id={}, direction={}",
            self.instrument_id, self.direction
        )?;
        if let Some(strength) = self.strength {
            write!(f, ", strength={strength}")?;
        }
        if let Some(horizon_ns) = self.horizon_ns {
            write!(f, ", horizon_ns={horizon_ns}")?;
        }
        if let Some(source) = self.source {
            write!(f, ", source={source}")?;
        }
        if let Some(expiry_ns) = self.expiry_ns {
            write!(f, ", expiry_ns={expiry_ns}")?;
        }
        write!(f, ", ts_event={}, ts_init={})", self.ts_event, self.ts_init)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    fn instrument_id() -> InstrumentId {
        InstrumentId::from("AAPL.XNYS")
    }

    fn signal(expiry_ns: Option<UnixNanos>) -> TradingSignal {
        TradingSignal::new(
            instrument_id(),
            SignalDirection::Long,
            Some(3_600_000_000_000),
            Some(0.75),
            Some(Ustr::from("ema_cross")),
            expiry_ns,
            None,
            1_000.into(),
            2_000.into(),
        )
        .unwrap()
    }

    #[rstest]
    fn test_construction_exposes_the_statement_of_view() {
        let mut provenance = Params::new();
        provenance.insert("model".to_string(), serde_json::json!("ema"));

        let signal = TradingSignal::new(
            instrument_id(),
            SignalDirection::Short,
            Some(1_800_000_000_000),
            Some(0.5),
            Some(Ustr::from("momentum")),
            Some(5_000.into()),
            Some(provenance),
            1_000.into(),
            2_000.into(),
        )
        .unwrap();

        assert_eq!(signal.instrument_id(), instrument_id());
        assert_eq!(signal.direction(), SignalDirection::Short);
        assert_eq!(signal.horizon_ns(), Some(1_800_000_000_000));
        assert_eq!(signal.strength(), Some(0.5));
        assert_eq!(signal.source(), Some(Ustr::from("momentum")));
        assert_eq!(signal.expiry_ns(), Some(5_000.into()));
        assert_eq!(
            signal.provenance().and_then(|p| p.get_str("model")),
            Some("ema")
        );
        assert_eq!(signal.ts_event(), UnixNanos::from(1_000));
        assert_eq!(signal.ts_init(), UnixNanos::from(2_000));
    }

    #[rstest]
    fn test_optional_fields_default_to_absent() {
        let signal = TradingSignal::new(
            instrument_id(),
            SignalDirection::Flat,
            None,
            None,
            None,
            None,
            None,
            1.into(),
            2.into(),
        )
        .unwrap();

        assert_eq!(signal.horizon_ns(), None);
        assert_eq!(signal.strength(), None);
        assert_eq!(signal.source(), None);
        assert_eq!(signal.expiry_ns(), None);
        assert!(signal.provenance().is_none());
    }

    #[rstest]
    #[case(f64::NAN)]
    #[case(f64::INFINITY)]
    #[case(f64::NEG_INFINITY)]
    fn test_rejects_a_non_finite_strength(#[case] strength: f64) {
        let result = TradingSignal::new(
            instrument_id(),
            SignalDirection::Long,
            None,
            Some(strength),
            None,
            None,
            None,
            1.into(),
            2.into(),
        );

        assert!(result.is_err());
    }

    #[rstest]
    fn test_rejects_a_negative_strength() {
        let result = TradingSignal::new(
            instrument_id(),
            SignalDirection::Long,
            None,
            Some(-0.1),
            None,
            None,
            None,
            1.into(),
            2.into(),
        );

        assert!(result.is_err());
    }

    #[rstest]
    fn test_accepts_a_zero_strength() {
        let signal = TradingSignal::new(
            instrument_id(),
            SignalDirection::Flat,
            None,
            Some(0.0),
            None,
            None,
            None,
            1.into(),
            2.into(),
        )
        .unwrap();

        assert_eq!(signal.strength(), Some(0.0));
    }

    #[rstest]
    fn test_rejects_a_zero_horizon() {
        let result = TradingSignal::new(
            instrument_id(),
            SignalDirection::Long,
            Some(0),
            None,
            None,
            None,
            None,
            1.into(),
            2.into(),
        );

        assert!(result.is_err());
    }

    #[rstest]
    fn test_rejects_an_expiry_before_the_event_time() {
        let result = TradingSignal::new(
            instrument_id(),
            SignalDirection::Long,
            None,
            None,
            None,
            Some(999.into()),
            None,
            1_000.into(),
            2_000.into(),
        );

        assert!(result.is_err());
    }

    #[rstest]
    fn test_accepts_an_expiry_at_the_event_time() {
        let signal = TradingSignal::new(
            instrument_id(),
            SignalDirection::Long,
            None,
            None,
            None,
            Some(1_000.into()),
            None,
            1_000.into(),
            2_000.into(),
        )
        .unwrap();

        assert_eq!(signal.expiry_ns(), Some(1_000.into()));
    }

    #[rstest]
    fn test_is_expired_is_false_before_the_expiry() {
        let signal = signal(Some(5_000.into()));

        assert!(!signal.is_expired(4_999.into()));
    }

    #[rstest]
    fn test_is_expired_is_true_at_the_expiry() {
        let signal = signal(Some(5_000.into()));

        assert!(signal.is_expired(5_000.into()));
    }

    #[rstest]
    fn test_is_expired_is_true_after_the_expiry() {
        let signal = signal(Some(5_000.into()));

        assert!(signal.is_expired(5_001.into()));
    }

    #[rstest]
    fn test_a_signal_without_an_expiry_never_expires() {
        let signal = signal(None);

        assert!(!signal.is_expired(UnixNanos::from(0)));
        assert!(!signal.is_expired(UnixNanos::from(u64::MAX)));
    }

    #[rstest]
    #[case(SignalDirection::Long, "LONG")]
    #[case(SignalDirection::Short, "SHORT")]
    #[case(SignalDirection::Flat, "FLAT")]
    fn test_direction_round_trips_through_canonical_string(
        #[case] direction: SignalDirection,
        #[case] expected: &str,
    ) {
        assert_eq!(direction.as_str(), expected);
        assert_eq!(direction.to_string(), expected);
        assert_eq!(expected.parse::<SignalDirection>().unwrap(), direction);
    }

    #[rstest]
    fn test_display_names_the_view() {
        let signal = signal(Some(5_000.into()));

        assert_eq!(
            signal.to_string(),
            "TradingSignal(instrument_id=AAPL.XNYS, direction=LONG, strength=0.75, \
             horizon_ns=3600000000000, source=ema_cross, expiry_ns=5000, ts_event=1000, \
             ts_init=2000)"
        );
    }
}
