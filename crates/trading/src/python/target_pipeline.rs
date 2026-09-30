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

//! Python bindings for [`TargetPipelineConfig`].

use nautilus_core::python::to_pyvalue_err;
use nautilus_model::types::Quantity;
use pyo3::prelude::*;
use rust_decimal::Decimal;

use crate::{target::TargetConstructionConfig, target_pipeline::TargetPipelineConfig};

#[pyo3_stub_gen::derive::gen_stub_pymethods]
#[pymethods]
impl TargetPipelineConfig {
    /// The configuration of a `TargetPipeline`.
    ///
    /// It bundles the `TargetConstructionConfig` of the construction stage with the minimum order
    /// quantity of the reconciliation stage, so the two stages of one pipeline are configured as a
    /// single value. The construction configuration is still validated by the construction stage.
    #[new]
    #[pyo3(signature = (
        risk_per_trade,
        stop_loss_bps,
        max_weight,
        commission_rate,
        min_order_quantity=None,
    ))]
    fn py_new(
        risk_per_trade: Decimal,
        stop_loss_bps: u32,
        max_weight: Decimal,
        commission_rate: Decimal,
        min_order_quantity: Option<Quantity>,
    ) -> PyResult<Self> {
        let construction = TargetConstructionConfig::new(
            risk_per_trade,
            stop_loss_bps,
            max_weight,
            commission_rate,
        )
        .map_err(to_pyvalue_err)?;

        Ok(Self::new(
            construction,
            min_order_quantity.unwrap_or_else(|| Quantity::zero(0)),
        ))
    }

    /// The fraction of equity risked per constructed position.
    #[getter]
    fn risk_per_trade(&self) -> Decimal {
        self.construction.risk_per_trade
    }

    /// The stop distance used to derive a position size, in basis points of the entry price.
    #[getter]
    fn stop_loss_bps(&self) -> u32 {
        self.construction.stop_loss_bps
    }

    /// The cap on the magnitude of a constructed target weight.
    #[getter]
    fn max_weight(&self) -> Decimal {
        self.construction.max_weight
    }

    /// The commission rate passed through to the sizing calculation.
    #[getter]
    fn commission_rate(&self) -> Decimal {
        self.construction.commission_rate
    }

    /// The minimum magnitude of a delta worth submitting as an order.
    #[getter]
    fn min_order_quantity(&self) -> Quantity {
        self.min_order_quantity
    }

    fn __repr__(&self) -> String {
        format!("{self:?}")
    }
}
