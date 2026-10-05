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

//! Python bindings from [PyO3](https://pyo3.rs).

#![expect(
    clippy::missing_errors_doc,
    reason = "errors documented on underlying Rust methods"
)]

pub mod algorithm;
pub mod analytics;
pub mod controller;
pub mod sessions;
pub mod strategy;
pub mod target_pipeline;
pub mod universe;

#[cfg(feature = "examples")]
mod examples;

use pyo3::{prelude::*, pymodule};

/// Exposed through `nautilus_trader.trading`.
///
/// # Errors
///
/// Returns a `PyErr` if registering any module components fails.
#[pymodule]
pub fn trading(_: Python<'_>, m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<crate::sessions::ForexSession>()?;
    m.add_function(wrap_pyfunction!(sessions::py_fx_local_from_utc, m)?)?;
    m.add_function(wrap_pyfunction!(sessions::py_fx_next_start, m)?)?;
    m.add_function(wrap_pyfunction!(sessions::py_fx_prev_start, m)?)?;
    m.add_function(wrap_pyfunction!(sessions::py_fx_next_end, m)?)?;
    m.add_function(wrap_pyfunction!(sessions::py_fx_prev_end, m)?)?;
    m.add_class::<strategy::PyStrategy>()?;
    m.add_class::<crate::controller::ImportableControllerConfig>()?;
    m.add_class::<crate::strategy::StrategyConfig>()?;
    m.add_class::<crate::strategy::ImportableStrategyConfig>()?;
    m.add_class::<crate::target_pipeline::TargetPipelineConfig>()?;
    m.add_class::<algorithm::PyExecutionAlgorithm>()?;
    m.add_class::<crate::algorithm::ExecutionAlgorithmConfig>()?;
    m.add_class::<crate::algorithm::ImportableExecutionAlgorithmConfig>()?;
    m.add_class::<universe::PyUniverse>()?;
    m.add_class::<crate::universe::UniverseDefinition>()?;
    m.add_class::<crate::universe::StaticUniverseRule>()?;
    m.add_class::<crate::universe::ScheduledUniverseRule>()?;
    m.add_class::<crate::universe::UniverseSubscription>()?;
    m.add_class::<crate::universe::UniverseRemovalPolicy>()?;
    m.add_class::<crate::analytics::ExecutionTerms>()?;
    m.add_class::<crate::analytics::ExecutionObserver>()?;
    m.add_class::<crate::analytics::ReferencePoint>()?;
    m.add_class::<crate::analytics::BenchmarkInterval>()?;
    m.add_class::<crate::analytics::QuoteObservation>()?;
    m.add_class::<crate::analytics::TradeObservation>()?;
    m.add_class::<crate::analytics::FillObservation>()?;
    m.add_class::<crate::analytics::ChildObservation>()?;
    m.add_class::<crate::analytics::Metric>()?;
    m.add_class::<crate::analytics::ExecutionMetrics>()?;
    m.add_class::<crate::analytics::MetricDeclaration>()?;
    m.add_class::<crate::analytics::MetricUnits>()?;
    m.add_class::<crate::analytics::MetricDirection>()?;
    m.add_class::<crate::analytics::ReferencePriceSource>()?;
    m.add_class::<crate::analytics::ReferenceTimestamp>()?;
    m.add_class::<crate::analytics::DenominatorSource>()?;
    m.add_class::<crate::analytics::UnavailableReason>()?;
    m.add(
        stringify!(METRIC_IMPLEMENTATION_SHORTFALL_BPS),
        crate::analytics::METRIC_IMPLEMENTATION_SHORTFALL_BPS,
    )?;
    m.add(
        stringify!(METRIC_ARRIVAL_SLIPPAGE_BPS),
        crate::analytics::METRIC_ARRIVAL_SLIPPAGE_BPS,
    )?;
    m.add(
        stringify!(METRIC_DECISION_PRICE_SLIPPAGE_BPS),
        crate::analytics::METRIC_DECISION_PRICE_SLIPPAGE_BPS,
    )?;
    m.add(
        stringify!(METRIC_VWAP_SLIPPAGE_BPS),
        crate::analytics::METRIC_VWAP_SLIPPAGE_BPS,
    )?;
    m.add(
        stringify!(METRIC_TWAP_SLIPPAGE_BPS),
        crate::analytics::METRIC_TWAP_SLIPPAGE_BPS,
    )?;
    m.add(
        stringify!(METRIC_MIDPOINT_SLIPPAGE_BPS),
        crate::analytics::METRIC_MIDPOINT_SLIPPAGE_BPS,
    )?;
    m.add(
        stringify!(METRIC_SPREAD_CAPTURE),
        crate::analytics::METRIC_SPREAD_CAPTURE,
    )?;
    m.add(
        stringify!(METRIC_ADVERSE_SELECTION),
        crate::analytics::METRIC_ADVERSE_SELECTION,
    )?;
    m.add(
        stringify!(METRIC_FILL_RATIO),
        crate::analytics::METRIC_FILL_RATIO,
    )?;
    m.add(
        stringify!(METRIC_CANCEL_RATIO),
        crate::analytics::METRIC_CANCEL_RATIO,
    )?;
    m.add(
        stringify!(METRIC_COMPLETION_TIME_S),
        crate::analytics::METRIC_COMPLETION_TIME_S,
    )?;
    m.add(
        stringify!(METRIC_CHILD_COUNT),
        crate::analytics::METRIC_CHILD_COUNT,
    )?;
    m.add(
        stringify!(METRIC_CHILD_CHURN),
        crate::analytics::METRIC_CHILD_CHURN,
    )?;
    m.add(
        stringify!(METRIC_MEAN_CHILD_LIFETIME_S),
        crate::analytics::METRIC_MEAN_CHILD_LIFETIME_S,
    )?;
    m.add(
        stringify!(METRIC_PARTIAL_FILL_RATIO),
        crate::analytics::METRIC_PARTIAL_FILL_RATIO,
    )?;
    m.add(
        stringify!(METRIC_PRICE_IMPROVEMENT),
        crate::analytics::METRIC_PRICE_IMPROVEMENT,
    )?;
    #[cfg(feature = "examples")]
    m.add_class::<crate::examples::strategies::CompositeMarketMakerConfig>()?;
    #[cfg(feature = "examples")]
    m.add_class::<crate::examples::strategies::EmaCrossConfig>()?;
    #[cfg(feature = "examples")]
    m.add_class::<crate::examples::strategies::GridMarketMakerConfig>()?;
    #[cfg(feature = "examples")]
    m.add_class::<crate::examples::strategies::DeltaNeutralVolConfig>()?;
    #[cfg(feature = "examples")]
    m.add_class::<crate::examples::strategies::HurstVpinDirectionalConfig>()?;
    #[cfg(feature = "examples")]
    m.add_class::<crate::examples::actors::BookImbalanceActorConfig>()?;
    Ok(())
}
