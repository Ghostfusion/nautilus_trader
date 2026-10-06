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

//! Label definitions: the forward outcome a feature predicts.
//!
//! A label is defined with the same rigour as a feature: a named, digestible declaration with a
//! horizon and a stated terminal convention. Unlike a feature it legitimately reads forward - it
//! is the target of prediction - so it carries no side and is by definition a learning-side
//! target; the feature aperture rule of the panel does not apply to it.
//!
//! Three labels are defined:
//!
//! - a forward return, `price[index + horizon] / price[index] - 1`;
//! - a forward maximum drawdown, the deepest peak-to-trough decline over the horizon, expressed as
//!   a non-positive fraction;
//! - a forward realised volatility, the population standard deviation of the `horizon` simple
//!   returns that follow the row.
//!
//! **Terminal convention.** A label over `horizon` requires `horizon` future observations. When
//! fewer remain at the end of the data, [`Label::compute`] returns an explicit absence
//! ([`None`]) - it never reports a missing future as zero. A degenerate input, such as a
//! non-positive price, is a typed error rather than a value.
//!
//! **Provenance.** A label value is only interpretable with the procedure that produced it.
//! [`LabelDefinition`] records that procedure by name, the [`ProducerIdentity`] that ran it, the
//! polling interval it observed (absent when the label was not built by polling) and its horizon,
//! and refuses at construction rather than defaulting a missing field. A dataset declares its
//! definition, and a record carries the definition that produced its label so a measurement can
//! count labels of unknown provenance rather than score them.

use nautilus_core::UnixNanos;
use serde::Serialize;
use serde_json::json;
use thiserror::Error;

use crate::dataset::{Digest, DigestInput, Label as LabelTrait};
use crate::measurement::ProducerIdentity;
use crate::operators;

/// The kind of a forward label and its horizon.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LabelKind {
    /// The forward simple return over the horizon.
    ForwardReturn {
        /// The number of future observations the label spans, at least one.
        horizon: usize,
    },
    /// The forward maximum drawdown over the horizon, as a non-positive fraction.
    ForwardMaxDrawdown {
        /// The number of future observations the label spans, at least one.
        horizon: usize,
    },
    /// The forward realised volatility: the population standard deviation of the simple returns
    /// over the horizon.
    ForwardRealizedVolatility {
        /// The number of future observations the label spans, at least one.
        horizon: usize,
    },
}

impl LabelKind {
    /// Returns the canonical lowercase name of the label kind.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::ForwardReturn { .. } => "forward_return",
            Self::ForwardMaxDrawdown { .. } => "forward_max_drawdown",
            Self::ForwardRealizedVolatility { .. } => "forward_realized_volatility",
        }
    }

    /// Returns the horizon of the label.
    #[must_use]
    pub const fn horizon(self) -> usize {
        match self {
            Self::ForwardReturn { horizon }
            | Self::ForwardMaxDrawdown { horizon }
            | Self::ForwardRealizedVolatility { horizon } => horizon,
        }
    }
}

/// A failure to define or compute a label.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum LabelError {
    /// A label was declared with a horizon of zero.
    #[error("label `{name}` must have a horizon of at least one observation")]
    ZeroHorizon {
        /// The label name.
        name: String,
    },
    /// A label was computed at an index outside the price series.
    #[error("label index {index} is outside a price series of length {length}")]
    IndexOutOfRange {
        /// The requested index.
        index: usize,
        /// The length of the price series.
        length: usize,
    },
    /// A price the label reads is not positive, so the outcome is undefined.
    #[error("label reads a non-positive price at index {index}")]
    NonPositivePrice {
        /// The index of the offending price.
        index: usize,
    },
    /// A label definition was declared with an empty procedure name.
    #[error("a label definition must have a non-empty procedure name")]
    EmptyProcedure,
    /// A label definition was declared with a polling interval of zero.
    #[error(
        "label definition `{procedure}` must have a polling interval of at least one nanosecond"
    )]
    ZeroPollInterval {
        /// The name of the procedure.
        procedure: String,
    },
    /// A label definition named an unknown producer.
    #[error("label definition `{procedure}` must name the producer that ran the procedure")]
    UnidentifiedProducer {
        /// The name of the procedure.
        procedure: String,
    },
}

/// The definition that produced a label (`design 4 I13`).
///
/// A label value is only interpretable with the procedure that produced it, so the definition
/// records the procedure by name, the [`ProducerIdentity`] that ran it, the polling interval it
/// observed and its horizon. The interval is an explicit absence ([`None`]) when the label was not
/// built by polling; it is never defaulted to zero. Every degenerate field is refused at
/// construction with a [`LabelError`] rather than silently defaulted, matching the way [`Label`]
/// treats a zero horizon.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct LabelDefinition {
    procedure: String,
    producer: ProducerIdentity,
    poll_interval: Option<UnixNanos>,
    horizon: usize,
}

impl LabelDefinition {
    /// Creates a label definition from a procedure name, its producer, its polling interval and
    /// its horizon.
    ///
    /// # Errors
    ///
    /// Returns [`LabelError::EmptyProcedure`] if the procedure name is empty,
    /// [`LabelError::ZeroHorizon`] if the horizon is zero, [`LabelError::ZeroPollInterval`] if a
    /// polling interval is present but zero, and [`LabelError::UnidentifiedProducer`] if the
    /// producer identity is unknown.
    pub fn new(
        procedure: impl Into<String>,
        producer: ProducerIdentity,
        poll_interval: Option<UnixNanos>,
        horizon: usize,
    ) -> Result<Self, LabelError> {
        let procedure = procedure.into();
        if procedure.trim().is_empty() {
            return Err(LabelError::EmptyProcedure);
        }
        if horizon == 0 {
            return Err(LabelError::ZeroHorizon { name: procedure });
        }
        if let Some(interval) = poll_interval
            && interval.as_u64() == 0
        {
            return Err(LabelError::ZeroPollInterval { procedure });
        }
        if producer.is_unknown() {
            return Err(LabelError::UnidentifiedProducer { procedure });
        }
        Ok(Self {
            procedure,
            producer,
            poll_interval,
            horizon,
        })
    }

    /// Returns the name of the procedure that produced the label.
    #[must_use]
    pub fn procedure(&self) -> &str {
        &self.procedure
    }

    /// Returns the producer identity that ran the procedure.
    #[must_use]
    pub const fn producer(&self) -> &ProducerIdentity {
        &self.producer
    }

    /// Returns the polling interval the procedure observed, absent when the label was not built by
    /// polling.
    #[must_use]
    pub const fn poll_interval(&self) -> Option<UnixNanos> {
        self.poll_interval
    }

    /// Returns the horizon of the label.
    #[must_use]
    pub const fn horizon(&self) -> usize {
        self.horizon
    }

    /// Returns the canonical JSON serialization of the definition.
    #[must_use]
    pub fn canonical_json(&self) -> String {
        json!({
            "procedure": self.procedure,
            "producer": self.producer,
            "poll_interval": self.poll_interval,
            "horizon": self.horizon,
        })
        .to_string()
    }

    /// Returns the stable digest of the definition's canonical serialization.
    #[must_use]
    pub fn digest(&self) -> Digest {
        Digest::of(self.canonical_json().as_bytes())
    }

    /// Returns the definition's procedure name and digest as a dataset digest input.
    #[must_use]
    pub fn digest_input(&self) -> DigestInput {
        DigestInput::new(self.procedure.clone(), self.digest())
    }
}

/// A declared forward label.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Label {
    name: String,
    kind: LabelKind,
}

impl Label {
    /// Creates a label from a name and a kind.
    ///
    /// # Errors
    ///
    /// Returns [`LabelError::ZeroHorizon`] if the horizon is zero.
    pub fn new(name: impl Into<String>, kind: LabelKind) -> Result<Self, LabelError> {
        let name = name.into();
        if kind.horizon() == 0 {
            return Err(LabelError::ZeroHorizon { name });
        }
        Ok(Self { name, kind })
    }

    /// Creates a forward return label.
    ///
    /// # Errors
    ///
    /// Returns [`LabelError::ZeroHorizon`] if the horizon is zero.
    pub fn forward_return(name: impl Into<String>, horizon: usize) -> Result<Self, LabelError> {
        Self::new(name, LabelKind::ForwardReturn { horizon })
    }

    /// Creates a forward maximum drawdown label.
    ///
    /// # Errors
    ///
    /// Returns [`LabelError::ZeroHorizon`] if the horizon is zero.
    pub fn forward_max_drawdown(
        name: impl Into<String>,
        horizon: usize,
    ) -> Result<Self, LabelError> {
        Self::new(name, LabelKind::ForwardMaxDrawdown { horizon })
    }

    /// Creates a forward realised volatility label.
    ///
    /// # Errors
    ///
    /// Returns [`LabelError::ZeroHorizon`] if the horizon is zero.
    pub fn forward_realized_volatility(
        name: impl Into<String>,
        horizon: usize,
    ) -> Result<Self, LabelError> {
        Self::new(name, LabelKind::ForwardRealizedVolatility { horizon })
    }

    /// Returns the canonical label name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the label kind.
    #[must_use]
    pub const fn kind(&self) -> LabelKind {
        self.kind
    }

    /// Returns the horizon of the label.
    #[must_use]
    pub const fn horizon(&self) -> usize {
        self.kind.horizon()
    }

    /// Returns the canonical JSON serialization of the definition.
    #[must_use]
    pub fn canonical_json(&self) -> String {
        json!({
            "name": self.name,
            "kind": self.kind.name(),
            "horizon": self.kind.horizon(),
        })
        .to_string()
    }

    /// Returns the stable digest of the definition's canonical serialization.
    #[must_use]
    pub fn digest(&self) -> Digest {
        Digest::of(self.canonical_json().as_bytes())
    }

    /// Returns the definition's name and digest as a dataset digest input.
    #[must_use]
    pub fn digest_input(&self) -> DigestInput {
        DigestInput::new(self.name.clone(), self.digest())
    }

    /// Computes the label at `index` over an ascending price series.
    ///
    /// # Errors
    ///
    /// Returns [`LabelError::IndexOutOfRange`] if `index` is outside the series and
    /// [`LabelError::NonPositivePrice`] if a price the label reads is not positive.
    pub fn compute(&self, prices: &[f64], index: usize) -> Result<Option<f64>, LabelError> {
        if index >= prices.len() {
            return Err(LabelError::IndexOutOfRange {
                index,
                length: prices.len(),
            });
        }

        let horizon = self.horizon();
        let terminal = index + horizon;
        if terminal >= prices.len() {
            return Ok(None);
        }

        match self.kind {
            LabelKind::ForwardReturn { .. } => {
                let base = require_positive(prices, index)?;
                let end = require_positive(prices, terminal)?;
                Ok(Some(end / base - 1.0))
            }
            LabelKind::ForwardMaxDrawdown { .. } => {
                let mut peak = require_positive(prices, index)?;
                let mut drawdown = 0.0;
                for offset in 1..=horizon {
                    let price = require_positive(prices, index + offset)?;
                    if price > peak {
                        peak = price;
                    }
                    let decline = price / peak - 1.0;
                    if decline < drawdown {
                        drawdown = decline;
                    }
                }
                Ok(Some(drawdown))
            }
            LabelKind::ForwardRealizedVolatility { .. } => {
                let mut returns = Vec::with_capacity(horizon);
                for offset in 1..=horizon {
                    let previous = require_positive(prices, index + offset - 1)?;
                    let current = require_positive(prices, index + offset)?;
                    returns.push(current / previous - 1.0);
                }
                Ok(operators::rolling_std(&returns))
            }
        }
    }
}

impl LabelTrait for Label {
    fn name(&self) -> &str {
        &self.name
    }

    fn digest(&self) -> Digest {
        Digest::of(self.canonical_json().as_bytes())
    }
}

fn require_positive(prices: &[f64], index: usize) -> Result<f64, LabelError> {
    let price = prices[index];
    if price > 0.0 {
        Ok(price)
    } else {
        Err(LabelError::NonPositivePrice { index })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forward_return_reads_the_horizon() {
        let label = Label::forward_return("ret", 2).unwrap();
        // 2.0 -> 4.0 over two steps: 4 / 2 - 1 = 1.0.
        assert_eq!(label.compute(&[2.0, 3.0, 4.0], 0), Ok(Some(1.0)));
    }

    #[test]
    fn missing_future_is_an_absence_not_a_zero() {
        let label = Label::forward_return("ret", 2).unwrap();
        assert_eq!(label.compute(&[2.0, 3.0], 0), Ok(None));
    }

    #[test]
    fn forward_max_drawdown_is_non_positive() {
        let label = Label::forward_max_drawdown("dd", 2).unwrap();
        // Peak 10 then trough 7: 7 / 10 - 1 = -0.3 (within floating point).
        let drawdown = label.compute(&[10.0, 8.0, 7.0], 0).unwrap().unwrap();
        assert!((drawdown - (-0.3)).abs() < 1e-12);
    }

    #[test]
    fn non_positive_price_is_a_typed_error() {
        let label = Label::forward_return("ret", 1).unwrap();
        assert_eq!(
            label.compute(&[0.0, 1.0], 0),
            Err(LabelError::NonPositivePrice { index: 0 })
        );
    }
}
