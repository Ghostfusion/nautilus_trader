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

use std::{fmt::Debug, sync::Arc};

use nautilus_core::python::to_pyvalue_err;
use nautilus_model::position::Position;
use pyo3::{
    exceptions::PyAttributeError,
    prelude::*,
    types::{PyDict, PyList},
};

use crate::{
    Returns,
    analyzer::Statistic,
    metric::{MetricDefinition, MetricDirection, MetricInput, MetricStage, MetricTag, MetricUnits},
    period::PerformancePeriod,
    statistic::PortfolioStatistic,
    statistics::{
        alpha::Alpha,
        arithmetic_compounding::{
            ArithmeticCompoundingFlagged, ArithmeticCompoundingImpliedEquity,
            ArithmeticCompoundingRatio, ArithmeticCompoundingRealisedEquity,
        },
        beta_ratio::BetaRatio,
        cagr::CAGR,
        calmar_ratio::CalmarRatio,
        correction_impact::CorrectionImpactReport,
        detector_report::DetectorReport,
        down_capture_ratio::DownCaptureRatio,
        expectancy::Expectancy,
        expected_shortfall::ExpectedShortfall,
        exponentially_weighted_sharpe::ExponentiallyWeightedSharpe,
        information_ratio::InformationRatio,
        long_ratio::LongRatio,
        loser_avg::AvgLoser,
        loser_max::MaxLoser,
        loser_min::MinLoser,
        max_drawdown::MaxDrawdown,
        max_drawdown_duration::MaxDrawdownDuration,
        omega_ratio::OmegaRatio,
        profit_factor::ProfitFactor,
        returns_avg::ReturnsAverage,
        returns_avg_loss::ReturnsAverageLoss,
        returns_avg_win::ReturnsAverageWin,
        returns_kurtosis::ReturnsKurtosis,
        returns_skewness::ReturnsSkewness,
        returns_volatility::ReturnsVolatility,
        risk_return_ratio::RiskReturnRatio,
        sharpe_ratio::SharpeRatio,
        sortino_ratio::SortinoRatio,
        tail_ratio::TailRatio,
        total_commissions::TotalCommissions,
        total_turnover::TotalTurnover,
        tracking_error::TrackingError,
        treynor_ratio::TreynorRatio,
        ulcer_index::UlcerIndex,
        up_capture_ratio::UpCaptureRatio,
        value_at_risk::ValueAtRisk,
        win_rate::WinRate,
        winner_avg::AvgWinner,
        winner_max::MaxWinner,
        winner_min::MinWinner,
    },
};

/// A [`PortfolioStatistic`] implemented in Python.
///
/// Wraps a user-defined Python object and dispatches each input category the analyzer feeds
/// to the method of the same name. A category the object does not define, or for which it
/// returns `None`, contributes no value.
///
/// Calculated values must be numeric, matching the `f64` item type the analyzer collects.
pub struct PythonStatistic {
    name: String,
    definition: MetricDefinition,
    statistic: Py<PyAny>,
}

impl Debug for PythonStatistic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(PythonStatistic))
            .field("name", &self.name)
            .finish_non_exhaustive()
    }
}

impl PythonStatistic {
    /// Creates a new [`PythonStatistic`] wrapping `statistic`.
    ///
    /// The name is resolved once at construction, so it stays stable for the registration key
    /// and every later lookup.
    ///
    /// # Errors
    ///
    /// Returns an error if `statistic` has no `name` attribute resolving to a non-empty string.
    pub fn new(py: Python<'_>, statistic: Py<PyAny>) -> PyResult<Self> {
        let name = statistic
            .getattr(py, "name")
            .and_then(|name| name.extract::<String>(py))
            .map_err(|e| {
                to_pyvalue_err(format!(
                    "Invalid statistic: `name` must resolve to a string, was {e}"
                ))
            })?;

        if name.trim().is_empty() {
            return Err(to_pyvalue_err(
                "Invalid statistic: `name` must not be empty".to_string(),
            ));
        }

        let definition = Self::metric_definition(py, &name, &statistic)?;

        Ok(Self {
            name,
            definition,
            statistic,
        })
    }

    /// Builds the error for a mistyped declared attribute.
    fn invalid_attribute(attribute: &str, expected: &str, error: impl std::fmt::Display) -> PyErr {
        to_pyvalue_err(format!(
            "Invalid statistic: `{attribute}` must be {expected}, was {error}"
        ))
    }

    /// Builds the metric definition the wrapped object declares.
    ///
    /// The base class `nautilus_trader.analysis.PortfolioStatistic` declares every attribute, so a
    /// subclass that overrides nothing still registers. An object that declares nothing at all -
    /// a duck-typed statistic exposing only `name` and the calculation methods - also registers,
    /// with its identity derived from its name and its metadata defaulted, and the definition is
    /// marked [`MetricDefinition::is_derived`] so the difference is visible rather than silent. A
    /// declaration that is present but mistyped is an error: a value outside the vocabulary is a
    /// defect, not a default.
    fn metric_definition(
        py: Python<'_>,
        name: &str,
        statistic: &Py<PyAny>,
    ) -> PyResult<MetricDefinition> {
        let bound = statistic.bind(py);

        // Returns `None` when the attribute is absent, which is what marks the definition derived;
        // any other lookup failure is an error rather than a silent default.
        let attribute = |attribute: &str| -> PyResult<Option<Bound<'_, PyAny>>> {
            match bound.getattr(attribute) {
                Ok(value) => Ok(Some(value)),
                Err(e) if e.is_instance_of::<PyAttributeError>(py) => Ok(None),
                Err(e) => Err(to_pyvalue_err(format!(
                    "Invalid statistic: `{attribute}` could not be read, was {e}"
                ))),
            }
        };

        let id = match attribute("metric_id")? {
            Some(value) => Some(
                value
                    .extract::<String>()
                    .map_err(|e| Self::invalid_attribute("metric_id", "a string", e))?,
            ),
            None => None,
        };

        if id.as_ref().is_some_and(|id| id.trim().is_empty()) {
            return Err(to_pyvalue_err(
                "Invalid statistic: `metric_id` must not be empty".to_string(),
            ));
        }

        let units = match attribute("units")? {
            Some(value) => Some(
                value
                    .extract::<MetricUnits>()
                    .map_err(|e| Self::invalid_attribute("units", "a MetricUnits", e))?,
            ),
            None => None,
        };

        let tags = match attribute("tags")? {
            Some(value) => Some(
                value
                    .extract::<Vec<MetricTag>>()
                    .map_err(|e| Self::invalid_attribute("tags", "a sequence of MetricTag", e))?,
            ),
            None => None,
        };

        let direction = match attribute("direction")? {
            Some(value) => Some(
                value
                    .extract::<MetricDirection>()
                    .map_err(|e| Self::invalid_attribute("direction", "a MetricDirection", e))?,
            ),
            None => None,
        };

        let target = match attribute("target")? {
            Some(value) => Some(
                value
                    .extract::<Option<f64>>()
                    .map_err(|e| Self::invalid_attribute("target", "a float or None", e))?,
            ),
            None => None,
        };

        let inputs =
            match attribute("inputs")? {
                Some(value) => Some(value.extract::<Vec<MetricInput>>().map_err(|e| {
                    Self::invalid_attribute("inputs", "a sequence of MetricInput", e)
                })?),
                None => None,
            };

        // `stage` is optional by definition, so its absence derives nothing: a statistic that
        // declares none is simply not placed on the chain.
        let stage = match attribute("stage")? {
            Some(value) => Some(
                value
                    .extract::<Option<MetricStage>>()
                    .map_err(|e| Self::invalid_attribute("stage", "a MetricStage or None", e))?,
            ),
            None => None,
        };

        // `target` is optional by definition, so its absence derives nothing.
        let derived = id.is_none()
            || units.is_none()
            || tags.is_none()
            || direction.is_none()
            || inputs.is_none();

        let definition = MetricDefinition::new(
            id.unwrap_or_else(|| derived_metric_id(name)),
            name.to_string(),
            units.unwrap_or(MetricUnits::Ratio),
            direction.unwrap_or(MetricDirection::Informational),
            inputs.unwrap_or_else(defined_over_every_input),
        )
        .with_tags(tags.unwrap_or_default());

        let definition = match target.flatten() {
            Some(target) => definition.with_target(target),
            None => definition,
        };

        let definition = match stage.flatten() {
            Some(stage) => definition.with_stage(stage),
            None => definition,
        };

        Ok(if derived {
            definition.as_derived()
        } else {
            definition
        })
    }

    /// Returns the bound `method` callable, or `None` when the statistic does not define it.
    fn method<'py>(&self, py: Python<'py>, method: &str) -> Option<Bound<'py, PyAny>> {
        match self.statistic.bind(py).getattr(method) {
            Ok(callable) => Some(callable),
            Err(e) if e.is_instance_of::<PyAttributeError>(py) => None,
            Err(e) => {
                self.report(py, method, "failed attribute lookup for", e);
                None
            }
        }
    }

    /// Returns the numeric value from `result`, reporting a raised or non-numeric outcome.
    ///
    /// The trait has no error channel, so a failure is reported here and skipped rather than
    /// propagated, leaving the remaining statistics to calculate.
    fn value(
        &self,
        py: Python<'_>,
        method: &str,
        result: PyResult<Bound<'_, PyAny>>,
    ) -> Option<f64> {
        let value = match result {
            Ok(value) => value,
            Err(e) => {
                self.report(py, method, "raised in", e);
                return None;
            }
        };

        if value.is_none() {
            return None;
        }

        match value.extract::<f64>() {
            Ok(value) => Some(value),
            Err(e) => {
                self.report(py, method, "returned a non-numeric value from", e);
                None
            }
        }
    }

    /// Reports a Python-side failure through both the log and `sys.unraisablehook`.
    ///
    /// The log alone is not enough: the `log` facade is a no-op until a logger is installed,
    /// which is the usual case for a standalone analyzer, so the traceback also goes to
    /// `sys.unraisablehook` where Python surfaces uncatchable callback errors.
    fn report(&self, py: Python<'_>, method: &str, what: &str, e: PyErr) {
        log::error!("Statistic `{}` {what} `{method}`: {e}", self.name);

        e.write_unraisable(py, Some(&self.statistic.bind(py).clone()));
    }

    /// Converts `returns` into the `dict[int, float]` shape the Python analyzer surface uses.
    fn returns_dict<'py>(
        &self,
        py: Python<'py>,
        method: &str,
        returns: &Returns,
    ) -> Option<Bound<'py, PyDict>> {
        let dict = PyDict::new(py);

        for (timestamp, value) in returns {
            self.converted(method, dict.set_item(timestamp.as_u64(), value))?;
        }

        Some(dict)
    }

    /// Returns the converted `value`, logging a conversion failure against `method`.
    fn converted<T>(&self, method: &str, value: PyResult<T>) -> Option<T> {
        match value {
            Ok(value) => Some(value),
            Err(e) => {
                log::error!(
                    "Statistic `{}` could not receive input for `{method}`: {e}",
                    self.name
                );
                None
            }
        }
    }
}

/// Returns the metric identity derived from a statistic's display name.
///
/// A statistic that declares no identity gets its name in `snake_case`: lowercased, with each run of
/// non-alphanumeric characters collapsed into a single underscore and the ends trimmed, so
/// `"Category Sentinel"` becomes `category_sentinel`.
fn derived_metric_id(name: &str) -> String {
    let mut id = String::with_capacity(name.len());
    let mut separator = false;

    for character in name.chars() {
        if character.is_alphanumeric() {
            if separator && !id.is_empty() {
                id.push('_');
            }

            id.extend(character.to_lowercase());
            separator = false;
        } else {
            separator = true;
        }
    }

    id
}

/// Returns the inputs a statistic that declares none is assumed to be defined over.
fn defined_over_every_input() -> Vec<MetricInput> {
    vec![
        MetricInput::Returns,
        MetricInput::RealizedPnls,
        MetricInput::Positions,
        MetricInput::PerformancePeriods,
    ]
}

impl PortfolioStatistic for PythonStatistic {
    type Item = f64;

    fn name(&self) -> String {
        self.name.clone()
    }

    fn definition(&self) -> MetricDefinition {
        self.definition.clone()
    }

    fn calculate_from_returns(&self, returns: &Returns) -> Option<f64> {
        const METHOD: &str = "calculate_from_returns";

        Python::attach(|py| {
            let method = self.method(py, METHOD)?;
            let returns = self.returns_dict(py, METHOD, returns)?;
            self.value(py, METHOD, method.call1((returns,)))
        })
    }

    fn calculate_from_realized_pnls(&self, realized_pnls: &[f64]) -> Option<f64> {
        const METHOD: &str = "calculate_from_realized_pnls";

        Python::attach(|py| {
            let method = self.method(py, METHOD)?;
            let realized_pnls = self.converted(METHOD, PyList::new(py, realized_pnls))?;
            self.value(py, METHOD, method.call1((realized_pnls,)))
        })
    }

    fn calculate_from_positions(&self, positions: &[Position]) -> Option<f64> {
        const METHOD: &str = "calculate_from_positions";

        Python::attach(|py| {
            let method = self.method(py, METHOD)?;
            let positions = self.converted(METHOD, PyList::new(py, positions.iter().cloned()))?;
            self.value(py, METHOD, method.call1((positions,)))
        })
    }

    fn calculate_from_periods(&self, periods: &[PerformancePeriod]) -> Option<f64> {
        const METHOD: &str = "calculate_from_periods";

        Python::attach(|py| {
            let method = self.method(py, METHOD)?;
            let periods = self.converted(METHOD, PyList::new(py, periods.iter().cloned()))?;
            self.value(py, METHOD, method.call1((periods,)))
        })
    }

    fn calculate_from_returns_with_benchmark(
        &self,
        returns: &Returns,
        benchmark: &Returns,
    ) -> Option<f64> {
        const METHOD: &str = "calculate_from_returns_with_benchmark";

        Python::attach(|py| {
            let method = self.method(py, METHOD)?;
            let returns = self.returns_dict(py, METHOD, returns)?;
            let benchmark = self.returns_dict(py, METHOD, benchmark)?;
            self.value(py, METHOD, method.call1((returns, benchmark)))
        })
    }
}

/// Converts `statistic` into a registrable [`Statistic`].
///
/// A built-in statistic type converts to its native Rust implementation, keeping calculation in
/// Rust. Any other object is wrapped as a [`PythonStatistic`] and dispatched back into Python on
/// calculation, including a user-defined class whose name matches a built-in.
///
/// # Errors
///
/// Returns an error if the object's class cannot be resolved, or if a user-defined statistic has
/// no `name` attribute resolving to a non-empty string.
pub fn statistic_from_pyobject(py: Python<'_>, statistic: Py<PyAny>) -> PyResult<Statistic> {
    let type_name = statistic
        .getattr(py, "__class__")?
        .getattr(py, "__name__")?
        .extract::<String>(py)?;

    if let Some(statistic) = native_statistic(py, &statistic, &type_name) {
        return Ok(statistic);
    }

    Ok(Arc::new(PythonStatistic::new(py, statistic)?))
}

/// Returns the native implementation when `statistic` is an instance of the built-in `type_name`.
///
/// The name selects which built-in type to try, and extraction then confirms the instance. A
/// user-defined class that only shares a built-in name fails that check and falls through to the
/// Python bridge, so built-in names stay usable for user-defined statistics.
fn native_statistic(py: Python<'_>, statistic: &Py<PyAny>, type_name: &str) -> Option<Statistic> {
    fn extract<T>(py: Python<'_>, statistic: &Py<PyAny>) -> Option<Statistic>
    where
        T: PortfolioStatistic<Item = f64>
            + Send
            + Sync
            + 'static
            + for<'a, 'py> FromPyObject<'a, 'py>,
    {
        statistic
            .extract::<T>(py)
            .ok()
            .map(|statistic| Arc::new(statistic) as Statistic)
    }

    match type_name {
        "MaxWinner" => extract::<MaxWinner>(py, statistic),
        "MinWinner" => extract::<MinWinner>(py, statistic),
        "AvgWinner" => extract::<AvgWinner>(py, statistic),
        "MaxLoser" => extract::<MaxLoser>(py, statistic),
        "MinLoser" => extract::<MinLoser>(py, statistic),
        "AvgLoser" => extract::<AvgLoser>(py, statistic),
        "Expectancy" => extract::<Expectancy>(py, statistic),
        "WinRate" => extract::<WinRate>(py, statistic),
        "ReturnsVolatility" => extract::<ReturnsVolatility>(py, statistic),
        "ReturnsAverage" => extract::<ReturnsAverage>(py, statistic),
        "ReturnsAverageLoss" => extract::<ReturnsAverageLoss>(py, statistic),
        "ReturnsAverageWin" => extract::<ReturnsAverageWin>(py, statistic),
        "SharpeRatio" => extract::<SharpeRatio>(py, statistic),
        "SortinoRatio" => extract::<SortinoRatio>(py, statistic),
        "ProfitFactor" => extract::<ProfitFactor>(py, statistic),
        "RiskReturnRatio" => extract::<RiskReturnRatio>(py, statistic),
        "LongRatio" => extract::<LongRatio>(py, statistic),
        "CAGR" => extract::<CAGR>(py, statistic),
        "CalmarRatio" => extract::<CalmarRatio>(py, statistic),
        "CorrectionImpactReport" => extract::<CorrectionImpactReport>(py, statistic),
        "DetectorReport" => extract::<DetectorReport>(py, statistic),
        "MaxDrawdown" => extract::<MaxDrawdown>(py, statistic),
        "MaxDrawdownDuration" => extract::<MaxDrawdownDuration>(py, statistic),
        "ExponentiallyWeightedSharpe" => extract::<ExponentiallyWeightedSharpe>(py, statistic),
        "TotalCommissions" => extract::<TotalCommissions>(py, statistic),
        "TotalTurnover" => extract::<TotalTurnover>(py, statistic),
        "Alpha" => extract::<Alpha>(py, statistic),
        "BetaRatio" => extract::<BetaRatio>(py, statistic),
        "DownCaptureRatio" => extract::<DownCaptureRatio>(py, statistic),
        "InformationRatio" => extract::<InformationRatio>(py, statistic),
        "TrackingError" => extract::<TrackingError>(py, statistic),
        "TreynorRatio" => extract::<TreynorRatio>(py, statistic),
        "ReturnsSkewness" => extract::<ReturnsSkewness>(py, statistic),
        "ReturnsKurtosis" => extract::<ReturnsKurtosis>(py, statistic),
        "TailRatio" => extract::<TailRatio>(py, statistic),
        "UlcerIndex" => extract::<UlcerIndex>(py, statistic),
        "OmegaRatio" => extract::<OmegaRatio>(py, statistic),
        "ValueAtRisk" => extract::<ValueAtRisk>(py, statistic),
        "ExpectedShortfall" => extract::<ExpectedShortfall>(py, statistic),
        "UpCaptureRatio" => extract::<UpCaptureRatio>(py, statistic),
        "ArithmeticCompoundingFlagged" => extract::<ArithmeticCompoundingFlagged>(py, statistic),
        "ArithmeticCompoundingImpliedEquity" => {
            extract::<ArithmeticCompoundingImpliedEquity>(py, statistic)
        }
        "ArithmeticCompoundingRatio" => extract::<ArithmeticCompoundingRatio>(py, statistic),
        "ArithmeticCompoundingRealisedEquity" => {
            extract::<ArithmeticCompoundingRealisedEquity>(py, statistic)
        }
        _ => None,
    }
}
