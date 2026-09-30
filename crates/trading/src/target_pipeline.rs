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

//! The optional target pipeline.
//!
//! A [`TargetPipeline`] is the two deterministic stages of the optional target path assembled into
//! one value: a [`TargetConstruction`] stage that resolves signals into desired exposures, and a
//! [`TargetReconciler`] stage that reconciles those targets against an authoritative snapshot into
//! the minimal order set. The pipeline is a convenience over the two stages; it adds no semantics
//! of its own.
//!
//! The pipeline is the middle of the three layers: a signal is a statement of view, a target is
//! the exposure that view resolves to, and an order is how the exposure is reached. A signal is
//! not a target and a target is not an order, and neither is a trading command. The pipeline owns
//! no position state, submits, cancels, and modifies no order, and reads no clock; the cache and
//! the portfolio remain authoritative. The caller submits the returned [`TargetOrder`] values on
//! the existing strategy-to-[`crate::ExecutionAlgorithm`] path, so the direct order path is
//! unchanged by the pipeline's presence.
//!
//! The pipeline is opt-in. A strategy holds none until it enables one, so a strategy that never
//! enables it produces byte-identical behaviour to one that predates the pipeline.

use nautilus_model::{signal::TradingSignal, target::Target, types::Quantity};

use crate::target::{
    ReconcileContext, TargetConstruction, TargetConstructionConfig, TargetConstructionContext,
    TargetConstructionError, TargetOrder, TargetReconciler, TargetReconcilerError,
};

/// The configuration of a [`TargetPipeline`].
///
/// It bundles the [`TargetConstructionConfig`] of the construction stage with the minimum order
/// quantity of the reconciliation stage, so the two stages of one pipeline are configured as a
/// single value. The construction configuration is still validated by the construction stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.trading", from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.trading")
)]
pub struct TargetPipelineConfig {
    /// The construction stage configuration.
    pub construction: TargetConstructionConfig,
    /// The minimum magnitude of a delta worth submitting as an order.
    ///
    /// A delta below this threshold produces no order. It is a threshold only: it is not rounded,
    /// and an emitted quantity is rounded to the instrument's size increment rather than to it.
    pub min_order_quantity: Quantity,
}

impl TargetPipelineConfig {
    /// Creates a new [`TargetPipelineConfig`].
    #[must_use]
    pub const fn new(construction: TargetConstructionConfig, min_order_quantity: Quantity) -> Self {
        Self {
            construction,
            min_order_quantity,
        }
    }
}

/// The optional target pipeline: construction and reconciliation as one value.
///
/// The pipeline delegates every step to the two stages it holds and adds no semantics of its own.
/// It holds configuration only: it owns no position state, is not registered on an engine or a
/// strategy, reads neither the cache nor the portfolio, submits, cancels, or modifies no order, and
/// reads no clock. It is not constructed by a strategy until that strategy opts in, so the direct
/// order path is unchanged by its existence.
#[derive(Clone, Debug)]
pub struct TargetPipeline {
    config: TargetPipelineConfig,
    construction: TargetConstruction,
    reconciler: TargetReconciler,
}

impl TargetPipeline {
    /// Creates a new [`TargetPipeline`] from `config`.
    ///
    /// # Errors
    ///
    /// Returns a [`TargetConstructionError::InvalidConfig`] if the construction configuration is
    /// outside its documented domain, so a pipeline cannot be created from an unvalidated
    /// configuration.
    pub fn new(config: TargetPipelineConfig) -> Result<Self, TargetConstructionError> {
        let construction = TargetConstruction::new(config.construction)?;
        let reconciler = TargetReconciler::new(config.min_order_quantity);

        Ok(Self {
            config,
            construction,
            reconciler,
        })
    }

    /// Returns the pipeline configuration.
    #[must_use]
    pub const fn config(&self) -> &TargetPipelineConfig {
        &self.config
    }

    /// Returns the construction stage.
    #[must_use]
    pub const fn construction(&self) -> &TargetConstruction {
        &self.construction
    }

    /// Returns the reconciliation stage.
    #[must_use]
    pub const fn reconciler(&self) -> &TargetReconciler {
        &self.reconciler
    }

    /// Constructs one target per signal, delegating to [`TargetConstruction::construct`].
    ///
    /// # Errors
    ///
    /// Returns an error if a signal cannot be resolved, naming the instrument. See
    /// [`TargetConstruction::construct`] for the cases.
    pub fn construct(
        &self,
        signals: &[TradingSignal],
        context: &TargetConstructionContext,
    ) -> Result<Vec<Target>, TargetConstructionError> {
        self.construction.construct(signals, context)
    }

    /// Reconciles targets to the minimal order set, delegating to [`TargetReconciler::reconcile`].
    ///
    /// # Errors
    ///
    /// Returns an error if a target that would produce an order cannot be resolved, naming the
    /// instrument. See [`TargetReconciler::reconcile`] for the cases.
    pub fn reconcile(
        &self,
        targets: &[Target],
        context: &ReconcileContext,
    ) -> Result<Vec<TargetOrder>, TargetReconcilerError> {
        self.reconciler.reconcile(targets, context)
    }
}

#[cfg(test)]
mod tests {
    use nautilus_core::UnixNanos;
    use nautilus_model::{
        instruments::{Instrument, InstrumentAny, stubs::equity_aapl},
        signal::SignalDirection,
        types::{Currency, Money, Price},
    };
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    use super::*;

    fn config() -> TargetPipelineConfig {
        TargetPipelineConfig::new(
            TargetConstructionConfig::new(dec!(0.01), 100, dec!(1), Decimal::ZERO).unwrap(),
            Quantity::zero(0),
        )
    }

    #[test]
    fn test_config_bundles_the_construction_config_and_threshold() {
        let config = config();

        assert_eq!(config.min_order_quantity, Quantity::zero(0));
        assert_eq!(config.construction.risk_per_trade, dec!(0.01));
        assert_eq!(config.construction.stop_loss_bps, 100);
    }

    #[test]
    fn test_new_rejects_an_invalid_construction_config() {
        let config = TargetPipelineConfig::new(
            TargetConstructionConfig {
                risk_per_trade: dec!(0),
                stop_loss_bps: 100,
                max_weight: dec!(1),
                commission_rate: Decimal::ZERO,
            },
            Quantity::zero(0),
        );

        assert!(matches!(
            TargetPipeline::new(config),
            Err(TargetConstructionError::InvalidConfig { .. })
        ));
    }

    #[test]
    fn test_accessors_expose_the_stages() {
        let pipeline = TargetPipeline::new(config()).unwrap();

        assert_eq!(pipeline.config().min_order_quantity, Quantity::zero(0));
        assert_eq!(pipeline.construction().config().stop_loss_bps, 100);
        assert_eq!(
            pipeline.reconciler().min_order_quantity(),
            Quantity::zero(0)
        );
    }

    #[test]
    fn test_construct_and_reconcile_delegate_to_the_stages() {
        let pipeline = TargetPipeline::new(config()).unwrap();
        let instrument = InstrumentAny::Equity(equity_aapl());
        let instrument_id = instrument.id();
        let signal = TradingSignal::new(
            instrument_id,
            SignalDirection::Long,
            None,
            None,
            None,
            None,
            None,
            UnixNanos::from(1_000_u64),
            UnixNanos::from(2_000_u64),
        )
        .unwrap();

        let construction_context = TargetConstructionContext {
            equity: Money::new(100_000.0, Currency::USD()),
            instruments: vec![instrument],
            prices: vec![(instrument_id, Price::from("100.00"))],
            positions: Vec::new(),
        };
        let targets = pipeline
            .construct(&[signal], &construction_context)
            .unwrap();
        assert_eq!(targets.len(), 1);

        let reconcile_context = ReconcileContext {
            instruments: construction_context.instruments,
            prices: construction_context.prices,
            equity: construction_context.equity,
            positions: Vec::new(),
            open_orders: Vec::new(),
        };
        let orders = pipeline.reconcile(&targets, &reconcile_context).unwrap();
        assert_eq!(orders.len(), 1);
        assert_eq!(orders[0].instrument_id, instrument_id);
    }

    #[test]
    fn test_min_order_quantity_suppresses_a_delta_below_the_threshold() {
        let config = TargetPipelineConfig::new(
            TargetConstructionConfig::new(dec!(0.01), 100, dec!(1), Decimal::ZERO).unwrap(),
            Quantity::from(1_500),
        );
        let pipeline = TargetPipeline::new(config).unwrap();
        let instrument = InstrumentAny::Equity(equity_aapl());
        let instrument_id = instrument.id();
        let signal = TradingSignal::new(
            instrument_id,
            SignalDirection::Long,
            None,
            None,
            None,
            None,
            None,
            UnixNanos::from(1_000_u64),
            UnixNanos::from(2_000_u64),
        )
        .unwrap();

        let construction_context = TargetConstructionContext {
            equity: Money::new(100_000.0, Currency::USD()),
            instruments: vec![instrument],
            prices: vec![(instrument_id, Price::from("100.00"))],
            positions: Vec::new(),
        };
        let targets = pipeline
            .construct(&[signal], &construction_context)
            .unwrap();

        let reconcile_context = ReconcileContext {
            instruments: construction_context.instruments,
            prices: construction_context.prices,
            equity: construction_context.equity,
            positions: Vec::new(),
            open_orders: Vec::new(),
        };
        let orders = pipeline.reconcile(&targets, &reconcile_context).unwrap();
        assert!(orders.is_empty());
    }
}
