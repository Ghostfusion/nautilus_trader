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

//! Python bindings for risk engine configuration.

use std::{collections::HashMap, str::FromStr};

use ahash::AHashMap;
use nautilus_common::throttler::RateLimit;
use nautilus_core::{DurationNanos, python::to_pyvalue_err};
use nautilus_model::{
    identifiers::{InstrumentId, Venue},
    risk::{RiskCapMetric, RiskCapScope},
};
use pyo3::{Py, PyAny, PyResult, Python, prelude::PyAnyMethods, pymethods};
use rust_decimal::Decimal;

use crate::{engine::cap::RiskCap, engine::config::RiskEngineConfig};

fn format_rate_limit(rate: &RateLimit) -> String {
    let total_secs = rate.interval_ns().as_secs();
    let hours = total_secs / 3_600;
    let minutes = (total_secs % 3_600) / 60;
    let seconds = total_secs % 60;
    format!("{}/{hours:02}:{minutes:02}:{seconds:02}", rate.limit())
}

fn parse_rate_limit(name: &str, value: &str) -> PyResult<RateLimit> {
    let (limit, interval) = value
        .split_once('/')
        .ok_or_else(|| to_pyvalue_err(format!("invalid `{name}`: expected 'limit/HH:MM:SS'")))?;

    let limit = limit
        .parse::<usize>()
        .map_err(|e| to_pyvalue_err(format!("invalid `{name}` limit: {e}")))?;

    let mut total_secs: u64 = 0;
    let mut parts = interval.split(':');
    for (label, multiplier) in [("hours", 3_600), ("minutes", 60), ("seconds", 1)] {
        let component = parts
            .next()
            .ok_or_else(|| {
                to_pyvalue_err(format!(
                    "invalid `{name}`: expected 'limit/HH:MM:SS' interval"
                ))
            })?
            .parse::<u64>()
            .map_err(|e| to_pyvalue_err(format!("invalid `{name}` {label}: {e}")))?;

        total_secs = total_secs.saturating_add(component.saturating_mul(multiplier));
    }

    if parts.next().is_some() {
        return Err(to_pyvalue_err(format!(
            "invalid `{name}`: expected 'limit/HH:MM:SS'"
        )));
    }

    let interval_ns = DurationNanos::try_from_secs(total_secs)
        .map_err(|e| to_pyvalue_err(format!("invalid `{name}`: {e}")))?;
    RateLimit::new_checked(limit, interval_ns)
        .map_err(|e| to_pyvalue_err(format!("invalid `{name}`: {e}")))
}

fn coerce_max_notional_per_order(
    raw: HashMap<String, Py<PyAny>>,
) -> PyResult<AHashMap<InstrumentId, Decimal>> {
    Python::attach(|py| -> PyResult<AHashMap<InstrumentId, Decimal>> {
        let mut result = AHashMap::with_capacity(raw.len());
        for (instrument_id, value) in raw {
            let parsed_id = InstrumentId::from_str(&instrument_id).map_err(|e| {
                to_pyvalue_err(format!(
                    "invalid `max_notional_per_order` instrument ID {instrument_id:?}: {e}"
                ))
            })?;
            let value_str: String = value.bind(py).str()?.extract()?;
            let notional = Decimal::from_str(&value_str).map_err(|e| {
                to_pyvalue_err(format!(
                    "invalid `max_notional_per_order` notional {value_str:?}: {e}"
                ))
            })?;
            result.insert(parsed_id, notional);
        }
        Ok(result)
    })
}

/// The metric a risk cap counts.
///
/// This is the Python-visible vocabulary of [`RiskCapMetric`]: the variants are class attributes,
/// so a cap names its metric the same way the Rust configuration does.
#[derive(Clone, Debug)]
#[pyo3::pyclass(
    frozen,
    name = "RiskCapMetric",
    module = "nautilus_trader.risk",
    from_py_object
)]
#[pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.risk")]
pub struct PyRiskCapMetric {
    inner: RiskCapMetric,
}

impl PyRiskCapMetric {
    /// Creates a new [`PyRiskCapMetric`] from the given value.
    #[must_use]
    pub const fn new(inner: RiskCapMetric) -> Self {
        Self { inner }
    }

    /// Returns the wrapped value.
    #[must_use]
    pub const fn inner(&self) -> RiskCapMetric {
        self.inner
    }
}

impl From<RiskCapMetric> for PyRiskCapMetric {
    fn from(value: RiskCapMetric) -> Self {
        Self::new(value)
    }
}

#[pyo3_stub_gen::derive::gen_stub_pymethods]
#[pymethods]
impl PyRiskCapMetric {
    /// Orders currently open; takes no window.
    #[classattr]
    #[expect(
        non_snake_case,
        clippy::use_self,
        reason = "PyO3 stub generation needs the concrete Python enum type"
    )]
    fn Active() -> PyRiskCapMetric {
        Self::new(RiskCapMetric::Active)
    }

    /// Orders admitted for submission.
    #[classattr]
    #[expect(
        non_snake_case,
        clippy::use_self,
        reason = "PyO3 stub generation needs the concrete Python enum type"
    )]
    fn Submit() -> PyRiskCapMetric {
        Self::new(RiskCapMetric::Submit)
    }

    /// Orders admitted for modification.
    #[classattr]
    #[expect(
        non_snake_case,
        clippy::use_self,
        reason = "PyO3 stub generation needs the concrete Python enum type"
    )]
    fn Modify() -> PyRiskCapMetric {
        Self::new(RiskCapMetric::Modify)
    }

    /// Cancellations observed.
    #[classattr]
    #[expect(
        non_snake_case,
        clippy::use_self,
        reason = "PyO3 stub generation needs the concrete Python enum type"
    )]
    fn Cancel() -> PyRiskCapMetric {
        Self::new(RiskCapMetric::Cancel)
    }

    /// Fills observed.
    #[classattr]
    #[expect(
        non_snake_case,
        clippy::use_self,
        reason = "PyO3 stub generation needs the concrete Python enum type"
    )]
    fn Fill() -> PyRiskCapMetric {
        Self::new(RiskCapMetric::Fill)
    }

    /// Requests repeated with the same canonical identity as the one being evaluated.
    #[classattr]
    #[expect(
        non_snake_case,
        clippy::use_self,
        reason = "PyO3 stub generation needs the concrete Python enum type"
    )]
    fn RepeatedRequest() -> PyRiskCapMetric {
        Self::new(RiskCapMetric::RepeatedRequest)
    }

    /// Constructs a metric from its canonical token.
    #[staticmethod]
    #[pyo3(name = "from_str")]
    fn py_from_str(value: &str) -> PyResult<Self> {
        value
            .parse::<RiskCapMetric>()
            .map_err(to_pyvalue_err)
            .map(Self::new)
    }

    fn __repr__(&self) -> String {
        format!("RiskCapMetric.{}", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }
}

/// The scope a risk cap counts over.
///
/// This is the Python-visible vocabulary of [`RiskCapScope`]: the variants are class attributes,
/// so a cap names its scope the same way the Rust configuration does.
#[derive(Clone, Debug)]
#[pyo3::pyclass(
    frozen,
    name = "RiskCapScope",
    module = "nautilus_trader.risk",
    from_py_object
)]
#[pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.risk")]
pub struct PyRiskCapScope {
    inner: RiskCapScope,
}

impl PyRiskCapScope {
    /// Creates a new [`PyRiskCapScope`] from the given value.
    #[must_use]
    pub const fn new(inner: RiskCapScope) -> Self {
        Self { inner }
    }

    /// Returns the wrapped value.
    #[must_use]
    pub const fn inner(&self) -> RiskCapScope {
        self.inner
    }
}

impl From<RiskCapScope> for PyRiskCapScope {
    fn from(value: RiskCapScope) -> Self {
        Self::new(value)
    }
}

#[pyo3_stub_gen::derive::gen_stub_pymethods]
#[pymethods]
impl PyRiskCapScope {
    /// Every order the engine sees.
    #[classattr]
    #[expect(
        non_snake_case,
        clippy::use_self,
        reason = "PyO3 stub generation needs the concrete Python enum type"
    )]
    fn Global() -> PyRiskCapScope {
        Self::new(RiskCapScope::Global)
    }

    /// The orders of one strategy.
    #[classattr]
    #[expect(
        non_snake_case,
        clippy::use_self,
        reason = "PyO3 stub generation needs the concrete Python enum type"
    )]
    fn Strategy() -> PyRiskCapScope {
        Self::new(RiskCapScope::Strategy)
    }

    /// The orders of one account.
    #[classattr]
    #[expect(
        non_snake_case,
        clippy::use_self,
        reason = "PyO3 stub generation needs the concrete Python enum type"
    )]
    fn Account() -> PyRiskCapScope {
        Self::new(RiskCapScope::Account)
    }

    /// The orders of one instrument.
    #[classattr]
    #[expect(
        non_snake_case,
        clippy::use_self,
        reason = "PyO3 stub generation needs the concrete Python enum type"
    )]
    fn Instrument() -> PyRiskCapScope {
        Self::new(RiskCapScope::Instrument)
    }

    /// The orders of one venue.
    #[classattr]
    #[expect(
        non_snake_case,
        clippy::use_self,
        reason = "PyO3 stub generation needs the concrete Python enum type"
    )]
    fn Venue() -> PyRiskCapScope {
        Self::new(RiskCapScope::Venue)
    }

    /// The orders of one strategy in one instrument.
    #[classattr]
    #[expect(
        non_snake_case,
        clippy::use_self,
        reason = "PyO3 stub generation needs the concrete Python enum type"
    )]
    fn StrategyInstrument() -> PyRiskCapScope {
        Self::new(RiskCapScope::StrategyInstrument)
    }

    /// Constructs a scope from its canonical token.
    #[staticmethod]
    #[pyo3(name = "from_str")]
    fn py_from_str(value: &str) -> PyResult<Self> {
        value
            .parse::<RiskCapScope>()
            .map_err(to_pyvalue_err)
            .map(Self::new)
    }

    fn __repr__(&self) -> String {
        format!("RiskCapScope.{}", self.inner)
    }

    fn __str__(&self) -> String {
        self.inner.to_string()
    }
}

/// A configured cap: what it counts, over what scope, and how many are allowed.
///
/// A cap refuses the actions that increase exposure when the count it observes reaches its limit,
/// and it never refuses a cancellation. Construct one from a [`RiskCapMetric`], a
/// [`RiskCapScope`], a positive `limit`, and an optional rolling `window` in nanoseconds (omitted
/// for an `Active` cap, which counts the open order set and takes no window).
#[derive(Clone, Debug)]
#[pyo3::pyclass(
    frozen,
    name = "RiskCap",
    module = "nautilus_trader.risk",
    from_py_object
)]
#[pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.risk")]
pub struct PyRiskCap {
    inner: RiskCap,
}

impl PyRiskCap {
    /// Creates a new [`PyRiskCap`] from the given value.
    #[must_use]
    pub const fn new(inner: RiskCap) -> Self {
        Self { inner }
    }

    /// Returns the wrapped value.
    #[must_use]
    pub const fn inner(&self) -> &RiskCap {
        &self.inner
    }
}

impl From<RiskCap> for PyRiskCap {
    fn from(value: RiskCap) -> Self {
        Self::new(value)
    }
}

impl From<PyRiskCap> for RiskCap {
    fn from(value: PyRiskCap) -> Self {
        value.inner
    }
}

#[pyo3_stub_gen::derive::gen_stub_pymethods]
#[pymethods]
impl PyRiskCap {
    /// Creates a new `RiskCap`.
    ///
    /// `window` is the rolling window in nanoseconds, omitted for an `Active` cap.
    #[new]
    #[pyo3(signature = (metric, scope, limit, window=None))]
    #[allow(
        clippy::needless_pass_by_value,
        reason = "PyO3 #[new] requires owned params"
    )]
    fn py_new(
        metric: PyRiskCapMetric,
        scope: PyRiskCapScope,
        limit: u32,
        window: Option<u64>,
    ) -> Self {
        Self::new(RiskCap::new(
            metric.inner(),
            scope.inner(),
            limit,
            window.map(DurationNanos::new),
        ))
    }

    /// Returns the metric counted.
    #[getter]
    #[pyo3(name = "metric")]
    fn py_metric(&self) -> PyRiskCapMetric {
        PyRiskCapMetric::new(self.inner.metric)
    }

    /// Returns the scope counted over.
    #[getter]
    #[pyo3(name = "scope")]
    fn py_scope(&self) -> PyRiskCapScope {
        PyRiskCapScope::new(self.inner.scope)
    }

    /// Returns the number of occurrences allowed before the cap refuses an action.
    #[getter]
    #[pyo3(name = "limit")]
    const fn py_limit(&self) -> u32 {
        self.inner.limit
    }

    /// Returns the rolling window in nanoseconds, `None` for an `Active` cap.
    #[getter]
    #[pyo3(name = "window")]
    fn py_window(&self) -> Option<u64> {
        self.inner.window.map(|window| window.as_u64())
    }

    fn __repr__(&self) -> String {
        format!(
            "RiskCap(metric={}, scope={}, limit={}, window={:?})",
            self.inner.metric, self.inner.scope, self.inner.limit, self.inner.window,
        )
    }

    fn __str__(&self) -> String {
        format!("{self:?}")
    }
}

#[pymethods]
#[pyo3_stub_gen::derive::gen_stub_pymethods]
impl RiskEngineConfig {
    /// Configuration for `RiskEngine` instances.
    #[new]
    #[pyo3(signature = (
        bypass = None,
        max_order_submit_rate = None,
        max_order_modify_rate = None,
        max_notional_per_order = None,
        full_position_exit_venues = None,
        debug = None,
        count_caps = None,
    ))]
    fn py_new(
        bypass: Option<bool>,
        max_order_submit_rate: Option<String>,
        max_order_modify_rate: Option<String>,
        max_notional_per_order: Option<HashMap<String, Py<PyAny>>>,
        full_position_exit_venues: Option<Vec<Venue>>,
        debug: Option<bool>,
        count_caps: Option<Vec<PyRiskCap>>,
    ) -> PyResult<Self> {
        let default = Self::default();

        let max_order_submit = match max_order_submit_rate {
            Some(value) => parse_rate_limit("max_order_submit_rate", &value)?,
            None => default.max_order_submit,
        };
        let max_order_modify = match max_order_modify_rate {
            Some(value) => parse_rate_limit("max_order_modify_rate", &value)?,
            None => default.max_order_modify,
        };
        let max_notional_per_order = match max_notional_per_order {
            Some(raw) => coerce_max_notional_per_order(raw)?,
            None => default.max_notional_per_order,
        };
        let full_position_exit_venues = full_position_exit_venues
            .map(|venues| venues.into_iter().collect())
            .unwrap_or(default.full_position_exit_venues);
        let count_caps = match count_caps {
            Some(caps) => caps.into_iter().map(RiskCap::from).collect(),
            None => default.count_caps,
        };

        Self::builder()
            .bypass(bypass.unwrap_or(default.bypass))
            .max_order_submit(max_order_submit)
            .max_order_modify(max_order_modify)
            .max_notional_per_order(max_notional_per_order)
            .full_position_exit_venues(full_position_exit_venues)
            .debug(debug.unwrap_or(default.debug))
            .count_caps(count_caps)
            .build()
            .map_err(to_pyvalue_err)
    }

    #[getter]
    #[pyo3(name = "bypass")]
    const fn py_bypass(&self) -> bool {
        self.bypass
    }

    #[getter]
    #[pyo3(name = "max_order_submit_rate")]
    fn py_max_order_submit_rate(&self) -> String {
        format_rate_limit(&self.max_order_submit)
    }

    #[getter]
    #[pyo3(name = "max_order_modify_rate")]
    fn py_max_order_modify_rate(&self) -> String {
        format_rate_limit(&self.max_order_modify)
    }

    #[getter]
    #[pyo3(name = "max_notional_per_order")]
    fn py_max_notional_per_order(&self) -> HashMap<String, String> {
        self.max_notional_per_order
            .iter()
            .map(|(id, notional)| (id.to_string(), notional.to_string()))
            .collect()
    }

    #[getter]
    #[pyo3(name = "full_position_exit_venues")]
    fn py_full_position_exit_venues(&self) -> Vec<Venue> {
        let mut venues = self
            .full_position_exit_venues
            .iter()
            .copied()
            .collect::<Vec<_>>();
        venues.sort_unstable();
        venues
    }

    #[getter]
    #[pyo3(name = "debug")]
    const fn py_debug(&self) -> bool {
        self.debug
    }

    #[getter]
    #[pyo3(name = "count_caps")]
    fn py_count_caps(&self) -> Vec<PyRiskCap> {
        self.count_caps
            .iter()
            .cloned()
            .map(PyRiskCap::from)
            .collect()
    }

    fn __repr__(&self) -> String {
        format!("{self:?}")
    }

    fn __str__(&self) -> String {
        format!("{self:?}")
    }
}
