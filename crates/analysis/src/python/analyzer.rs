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

use std::collections::{BTreeMap, HashMap};

use nautilus_core::{UnixNanos, python::to_pyvalue_err};
use nautilus_model::{
    identifiers::PositionId,
    position::Position,
    types::{Currency, Money},
};
use pyo3::prelude::*;
use rust_decimal::{Decimal, prelude::ToPrimitive};

use crate::{
    Returns,
    analyzer::PortfolioAnalyzer,
    metric::{MetricDefinition, MetricReport, MetricStage},
    period::PerformancePeriod,
    python::statistic::statistic_from_pyobject,
    snapshot::PortfolioStatistics,
    statistics::{correction_impact::CorrectionImpactReport, detector_report::DetectorReport},
};

#[pymethods]
#[pyo3_stub_gen::derive::gen_stub_pymethods]
impl PortfolioAnalyzer {
    /// Analyzes portfolio performance and calculates various statistics.
    ///
    /// The `PortfolioAnalyzer` tracks account balances, positions, and realized PnLs
    /// to provide portfolio analysis including returns, PnL calculations,
    /// and customizable statistics.
    #[new]
    #[must_use]
    pub fn py_new() -> Self {
        Self::new()
    }

    fn __repr__(&self) -> String {
        format!("PortfolioAnalyzer(currencies={})", self.currencies().len())
    }

    /// Returns all tracked currencies.
    #[pyo3(name = "currencies")]
    fn py_currencies(&self) -> Vec<Currency> {
        self.currencies().into_iter().copied().collect()
    }

    /// Gets all return-based performance statistics.
    #[pyo3(name = "get_performance_stats_returns")]
    fn py_get_performance_stats_returns(&self) -> HashMap<String, f64> {
        self.get_performance_stats_returns().into_iter().collect()
    }

    /// Gets all position-return-based performance statistics.
    #[pyo3(name = "get_performance_stats_position_returns")]
    fn py_get_performance_stats_position_returns(&self) -> HashMap<String, f64> {
        self.get_performance_stats_position_returns()
            .into_iter()
            .collect()
    }

    /// Gets all portfolio-return-based performance statistics.
    #[pyo3(name = "get_performance_stats_portfolio_returns")]
    fn py_get_performance_stats_portfolio_returns(&self) -> HashMap<String, f64> {
        self.get_performance_stats_portfolio_returns()
            .into_iter()
            .collect()
    }

    /// Gets all benchmark-relative return statistics for the primary returns.
    ///
    /// This is stateless: the `benchmark` series is supplied by the caller rather
    /// than stored on the analyzer. Only statistics that override
    /// `PortfolioStatistic.calculate_from_returns_with_benchmark` (the benchmark-relative
    /// statistics) contribute values; all others return `None` and are skipped.
    #[pyo3(name = "get_performance_stats_returns_vs_benchmark")]
    fn py_get_performance_stats_returns_vs_benchmark(
        &self,
        benchmark: BTreeMap<u64, f64>,
    ) -> HashMap<String, f64> {
        let benchmark: Returns = benchmark
            .into_iter()
            .map(|(k, v)| (UnixNanos::from(k), v))
            .collect();
        self.get_performance_stats_returns_vs_benchmark(&benchmark)
            .into_iter()
            .collect()
    }

    /// Gets all PnL-related performance statistics.
    ///
    /// # Errors
    ///
    /// Returns an error if PnL calculations fail, for example due to:
    ///
    /// - No currency specified for a multi-currency portfolio.
    /// - Unrealized PnL currency not matching the specified currency.
    /// - Specified currency not found in account balances.
    #[pyo3(name = "get_performance_stats_pnls")]
    fn py_get_performance_stats_pnls(
        &self,
        currency: Option<&Currency>,
        unrealized_pnl: Option<&Money>,
    ) -> PyResult<HashMap<String, f64>> {
        self.get_performance_stats_pnls(currency, unrealized_pnl)
            .map(|m| m.into_iter().collect())
            .map_err(to_pyvalue_err)
    }

    /// Gets general portfolio statistics.
    #[pyo3(name = "get_performance_stats_general")]
    fn py_get_performance_stats_general(&self) -> HashMap<String, f64> {
        self.get_performance_stats_general().into_iter().collect()
    }

    /// Collects an owned `PortfolioStatistics` snapshot from the current analyzer state.
    ///
    /// The period frame's rows are merged in beside the returns and general rows. A row is
    /// grouped by its registered statistic's declared units: a `MetricUnits.Currency` amount
    /// is a money total and belongs in `general`, while every other unit (`Ratio`, `Fraction`,
    /// `BasisPoints`) is a dimensionless figure and belongs in `returns`. The frame's cost and
    /// return metrics are ratios and rates, so they read beside the return statistics; only the
    /// money totals are general.
    ///
    /// With an empty frame no period row is produced (`calculate_from_periods` returns `None`),
    /// so the two maps are exactly what the returns and general calculations yield on their own.
    ///
    /// When stages have been declared with `Self.set_declared_stages`, a declared stage that
    /// produced no rendered row is reported as one row in `general`, named for the stage, so a
    /// chain that intends a stage it never scores is visible rather than silent.
    ///
    /// When fill-cause totals have been set with `Self.set_fill_cause_counts`, one row per
    /// cause is added to `general`, named for the cause, including the causes at zero: a zero is a
    /// fact and an omitted row is not. With no totals set, no cause row is produced.
    ///
    /// When a correction impact has been set with `Self.set_correction_impact`, three rows are
    /// added to `returns`, naming the declared metric and the stream each value came from: the
    /// uncorrected value, the corrected value and their delta. A correction that changed nothing
    /// renders a delta of exactly zero rather than omitting the row. With no impact set, no
    /// correction-impact row is produced.
    ///
    /// When a detector report has been set with `Self.set_detector_report`, the accuracy and its
    /// positive base rate are added to `returns` as one inseparable pair, beside `precision`,
    /// `recall`, `F1` and the false-discovery rate, and the four counts of the confusion matrix are
    /// added to `general`. A rate that is undefined is omitted rather than given a plausible value,
    /// and the accuracy and base rate are always both present or both absent. With no report set,
    /// no detector row is produced.
    #[pyo3(name = "statistics")]
    fn py_statistics(&self) -> PortfolioStatistics {
        self.statistics()
    }

    /// Returns the metric definition of every registered statistic, ordered by identity.
    ///
    /// This is the declarative metadata behind a report: the stable identity, the title
    /// rendered from its parameters, and the units, tags, direction and inputs of each metric.
    #[pyo3(name = "metric_definitions")]
    fn py_metric_definitions(&self) -> Vec<MetricDefinition> {
        let mut definitions: Vec<MetricDefinition> = self
            .statistics
            .values()
            .map(|statistic| statistic.definition())
            .collect();
        definitions.sort_by(|a, b| a.id().cmp(b.id()));

        definitions
    }

    /// Returns the scoring-chain stage of every registered statistic that declares one.
    ///
    /// Keyed by the statistic's rendered name, which is the key its rows carry in a report. A
    /// statistic that declares no stage is absent rather than defaulted, so a consumer can tell
    /// an undeclared metric from a declared one.
    #[pyo3(name = "metric_stages")]
    fn py_metric_stages(&self) -> HashMap<String, String> {
        self.metric_stages()
            .into_iter()
            .map(|(name, stage)| (name, stage.as_str().to_string()))
            .collect()
    }

    /// Sets the scoring-chain stages this run declares it intends to report.
    ///
    /// Replaces any declaration already held. An empty declaration leaves every stage unclaimed,
    /// so `Self.statistics` adds no missing-stage row and the report is what it was before the
    /// chain existed.
    #[pyo3(name = "set_declared_stages")]
    fn py_set_declared_stages(&mut self, stages: Vec<MetricStage>) {
        self.set_declared_stages(stages);
    }

    /// Returns the scoring-chain stages this run has declared.
    #[pyo3(name = "declared_stages")]
    fn py_declared_stages(&self) -> Vec<MetricStage> {
        self.declared_stages().to_vec()
    }

    /// Reports the requested return-based metrics, one result per request.
    ///
    /// Every requested metric appears in the report. A metric that is not registered, is not
    /// defined over returns, requires a benchmark that was not supplied, or produced no
    /// meaningful value is reported with its status and reason rather than dropped, which is
    /// what distinguishes this from `Self.get_performance_stats_returns`.
    ///
    /// A request is matched against a statistic's stable definition id first and its display
    /// name second, so both `"sharpe_ratio"` and `"Sharpe Ratio (simple, sample, 252 days)"`
    /// address the same metric. A statistic whose definition declares the benchmark input is
    /// calculated from the returns and the supplied benchmark; when the definition requires a
    /// benchmark and `benchmark` is `None` the metric is reported `unavailable` with
    /// `MetricReason.MissingBenchmark` rather than calculated from the returns alone.
    #[expect(clippy::needless_pass_by_value)]
    #[pyo3(name = "report_returns_metrics", signature = (requested, benchmark=None))]
    fn py_report_returns_metrics(
        &self,
        requested: Vec<String>,
        benchmark: Option<BTreeMap<u64, f64>>,
    ) -> MetricReport {
        let benchmark: Option<Returns> = benchmark.map(|benchmark| {
            benchmark
                .into_iter()
                .map(|(timestamp, value)| (UnixNanos::from(timestamp), value))
                .collect()
        });
        let requested: Vec<&str> = requested.iter().map(String::as_str).collect();

        self.report_returns_metrics(&requested, benchmark.as_ref())
    }

    /// Reports the requested position-based metrics, one result per request.
    ///
    /// See `Self.report_returns_metrics` for the status semantics.
    #[expect(clippy::needless_pass_by_value)]
    #[pyo3(name = "report_position_metrics")]
    fn py_report_position_metrics(&self, requested: Vec<String>) -> MetricReport {
        let requested: Vec<&str> = requested.iter().map(String::as_str).collect();

        self.report_position_metrics(&requested)
    }

    /// Reports the requested realized-PnL-based metrics, one result per request.
    ///
    /// A metric whose definition requires the realized PnL input is reported `unavailable` with
    /// `MetricReason.UnresolvedCurrency` when the portfolio holds realized PnLs in more than
    /// one currency and the requested currency does not resolve, rather than being calculated
    /// from an arbitrary subset of them.
    #[expect(clippy::needless_pass_by_value)]
    #[pyo3(name = "report_pnls_metrics", signature = (requested, currency=None))]
    fn py_report_pnls_metrics(
        &self,
        requested: Vec<String>,
        currency: Option<&Currency>,
    ) -> MetricReport {
        let requested: Vec<&str> = requested.iter().map(String::as_str).collect();

        self.report_pnls_metrics(&requested, currency)
    }

    /// Reports the requested period-frame-based metrics, one result per request.
    ///
    /// Mirrors `Self.report_returns_metrics` for the performance-period input: a metric whose
    /// definition is defined over `MetricInput.PerformancePeriods` is calculated from `periods`,
    /// and one that is not is reported `unavailable` with `MetricReason.UnsupportedInput`. An
    /// empty frame is reported `unavailable` with `MetricReason.InsufficientData`; a statistic
    /// that declines to reduce a present frame is reported the same way.
    #[expect(clippy::needless_pass_by_value)]
    #[pyo3(name = "report_period_metrics")]
    fn py_report_period_metrics(
        &self,
        requested: Vec<String>,
        periods: Vec<PerformancePeriod>,
    ) -> MetricReport {
        let requested: Vec<&str> = requested.iter().map(String::as_str).collect();

        self.report_period_metrics(&requested, &periods)
    }

    /// Reports the requested tape-fed metrics, one result per request.
    ///
    /// Mirrors `Self.report_period_metrics` for the tape input: a metric whose definition is
    /// defined over `MetricInput.Trades` is calculated from the signed order-flow imbalance
    /// carried by `Self.set_signed_order_flow_imbalance`, and one that is not is reported
    /// `unavailable` with `MetricReason.UnsupportedInput`. A direction-dependent metric (one
    /// whose definition declares `MetricTag.DirectionDependent`) is refused with
    /// `MetricReason.AggressorAgreementBelowFloor` when the declared floor is above the
    /// observed rate, and the refusal's detail names both; nothing is printed as a number in that
    /// case.
    ///
    /// The floor is a precondition, not a calculation: it refuses only when a rate was observed,
    /// because a floor cannot be breached by a rate that was never measured. An unmeasured rate
    /// therefore leaves the metric computed, which is the conservative reading of an absent
    /// measurement.
    #[expect(clippy::needless_pass_by_value)]
    #[pyo3(name = "report_tape_metrics")]
    fn py_report_tape_metrics(&self, requested: Vec<String>) -> MetricReport {
        let requested: Vec<&str> = requested.iter().map(String::as_str).collect();

        self.report_tape_metrics(&requested)
    }

    /// Sets the observed aggressor-agreement rate of the run's trade tape and the declared floor.
    ///
    /// The rate is `None` when no trade was comparable, which is not a zero rate: a value that was
    /// never measured cannot breach a floor, so a direction-dependent metric is computed as usual.
    /// The floor is `None` when the run declared none, which disables the gate. Both are replaced
    /// together because the gate reads them together.
    #[pyo3(name = "set_aggressor_agreement", signature = (observed, floor=None))]
    fn py_set_aggressor_agreement(&mut self, observed: Option<f64>, floor: Option<f64>) {
        self.set_aggressor_agreement(observed, floor);
    }

    /// Returns the observed aggressor-agreement rate of the run's trade tape, if any.
    #[getter]
    #[pyo3(name = "aggressor_agreement")]
    const fn py_aggressor_agreement(&self) -> Option<f64> {
        self.aggressor_agreement()
    }

    /// Returns the declared aggressor-agreement floor, if any.
    #[getter]
    #[pyo3(name = "aggressor_agreement_floor")]
    const fn py_aggressor_agreement_floor(&self) -> Option<f64> {
        self.aggressor_agreement_floor()
    }

    /// Sets the signed order-flow imbalance accumulated from the run's trade tape.
    ///
    /// The quantity is already signed by the reported aggressor side; the analyzer is a carrier,
    /// not a reconstructor, of its sign. Replaces any value already held.
    #[pyo3(name = "set_signed_order_flow_imbalance")]
    fn py_set_signed_order_flow_imbalance(&mut self, value: Option<f64>) {
        self.set_signed_order_flow_imbalance(value.and_then(Decimal::from_f64_retain));
    }

    /// Returns the signed order-flow imbalance accumulated from the run's trade tape, if any.
    #[getter]
    #[pyo3(name = "signed_order_flow_imbalance")]
    fn py_signed_order_flow_imbalance(&self) -> Option<f64> {
        self.signed_order_flow_imbalance()
            .and_then(|value| value.to_f64())
    }

    /// Sets the measured effect of the data-quality correction applied to the run's stream.
    ///
    /// The report is built by the caller that holds both measurements of the declared outcome
    /// metric; the analyzer is a carrier, not a measurer, of it. `None` when no correction was
    /// applied or none declared a metric, in which case no correction-impact row is rendered: an
    /// absent measurement is not a zero delta. When `Some`, the uncorrected value, the corrected
    /// value and their delta are rendered, each naming the stream it came from, and the zero delta
    /// of a correction that changed nothing is rendered rather than omitted.
    #[pyo3(name = "set_correction_impact")]
    fn py_set_correction_impact(&mut self, impact: Option<CorrectionImpactReport>) {
        self.set_correction_impact(impact);
    }

    /// Returns the measured effect of the data-quality correction applied to the run's stream, if
    /// any.
    #[getter]
    #[pyo3(name = "correction_impact")]
    fn py_correction_impact(&self) -> Option<CorrectionImpactReport> {
        self.correction_impact().cloned()
    }

    /// Sets the detector's confusion matrix and the rates that read it.
    ///
    /// The report is built by the caller that holds the detector's decisions and the ground-truth
    /// labels; the analyzer is a carrier, not a measurer, of it. `None` when no detector was
    /// evaluated, in which case no detector-report row is rendered. When `Some`, the accuracy is
    /// rendered as one inseparable pair with its positive base rate, beside the counts, precision,
    /// recall, F1 and the false-discovery rate.
    #[pyo3(name = "set_detector_report")]
    fn py_set_detector_report(&mut self, report: Option<DetectorReport>) {
        self.set_detector_report(report);
    }

    /// Returns the detector's confusion matrix and the rates that read it, if any.
    #[getter]
    #[pyo3(name = "detector_report")]
    fn py_detector_report(&self) -> Option<DetectorReport> {
        self.detector_report().cloned()
    }

    /// Records a position return at a specific timestamp.
    #[pyo3(name = "add_position_return")]
    fn py_add_position_return(&mut self, timestamp: u64, value: f64) {
        self.add_position_return(UnixNanos::from(timestamp), value);
    }

    /// Records a return at a specific timestamp.
    ///
    /// This is a backward-compatible alias for `Self.add_position_return`.
    #[pyo3(name = "add_return")]
    fn py_add_return(&mut self, timestamp: u64, value: f64) {
        self.add_return(UnixNanos::from(timestamp), value);
    }

    /// Resets all analysis data to initial state.
    ///
    /// Registered statistics are retained; use `Self.deregister_statistics` to clear them.
    #[pyo3(name = "reset")]
    fn py_reset(&mut self) {
        self.reset();
    }

    /// Registers a new portfolio statistic for calculation.
    #[pyo3(name = "register_statistic")]
    fn py_register_statistic(&mut self, py: Python, statistic: Py<PyAny>) -> PyResult<()> {
        self.register_statistic(statistic_from_pyobject(py, statistic)?);
        Ok(())
    }

    /// Removes a specific statistic from calculation.
    #[pyo3(name = "deregister_statistic")]
    fn py_deregister_statistic(&mut self, py: Python, statistic: Py<PyAny>) -> PyResult<()> {
        self.deregister_statistic(&statistic_from_pyobject(py, statistic)?);
        Ok(())
    }

    /// Removes all registered statistics.
    #[pyo3(name = "deregister_statistics")]
    fn py_deregister_statistics(&mut self) {
        self.deregister_statistics();
    }

    /// Adds new positions for analysis.
    #[pyo3(name = "add_positions")]
    #[expect(clippy::needless_pass_by_value)]
    fn py_add_positions(&mut self, py: Python, positions: Vec<Py<PyAny>>) -> PyResult<()> {
        let positions: Vec<Position> = positions
            .iter()
            .map(|position| position.extract::<Position>(py).map_err(Into::into))
            .collect::<PyResult<Vec<Position>>>()?;

        self.add_positions(&positions);
        Ok(())
    }

    /// Records a trade's PnL realized at `ts_event`.
    #[pyo3(name = "add_trade")]
    #[allow(
        clippy::trivially_copy_pass_by_ref,
        reason = "matches underlying add_trade signature"
    )]
    fn py_add_trade(&mut self, position_id: &PositionId, ts_event: u64, realized_pnl: &Money) {
        self.add_trade(position_id, UnixNanos::from(ts_event), realized_pnl);
    }

    /// Records a trade's PnL realized at `ts_event`, observed during portfolio processing.
    #[pyo3(name = "record_trade")]
    #[allow(
        clippy::trivially_copy_pass_by_ref,
        reason = "matches underlying record_trade signature"
    )]
    fn py_record_trade(&mut self, position_id: &PositionId, ts_event: u64, realized_pnl: &Money) {
        self.record_trade(position_id, UnixNanos::from(ts_event), realized_pnl);
    }

    // Note: calculate_statistics is not exposed to Python because it requires
    // complex conversions of Account and dict types. Use the Python analyzer.py wrapper instead.

    /// Retrieves a specific statistic by name.
    #[pyo3(name = "statistic")]
    fn py_statistic(&self, name: &str) -> Option<String> {
        self.statistic(name).map(|s| s.name())
    }

    /// Returns the primary calculated returns.
    ///
    /// This returns portfolio returns when available, otherwise it falls back
    /// to position returns for backward compatibility.
    #[pyo3(name = "returns")]
    fn py_returns(&self, py: Python) -> PyResult<Py<PyAny>> {
        // Convert BTreeMap<UnixNanos, f64> to Python dict
        let dict = pyo3::types::PyDict::new(py);
        for (timestamp, value) in self.returns() {
            dict.set_item(timestamp.as_u64(), value)?;
        }
        Ok(dict.into())
    }

    /// Returns the per-position calculated returns.
    #[pyo3(name = "position_returns")]
    fn py_position_returns(&self, py: Python) -> PyResult<Py<PyAny>> {
        let dict = pyo3::types::PyDict::new(py);
        for (timestamp, value) in self.position_returns() {
            dict.set_item(timestamp.as_u64(), value)?;
        }
        Ok(dict.into())
    }

    /// Returns the portfolio calculated returns.
    #[pyo3(name = "portfolio_returns")]
    fn py_portfolio_returns(&self, py: Python) -> PyResult<Py<PyAny>> {
        let dict = pyo3::types::PyDict::new(py);
        for (timestamp, value) in self.portfolio_returns() {
            dict.set_item(timestamp.as_u64(), value)?;
        }
        Ok(dict.into())
    }

    /// Retrieves realized PnLs for a specific currency.
    ///
    /// Each record is `(position_id, ts_event, realized_pnl)`, in ascending `ts_event` order.
    /// Returns `None` if no PnLs exist, or if multiple currencies exist without an explicit
    /// currency specified.
    #[pyo3(name = "realized_pnls")]
    fn py_realized_pnls(&self, py: Python, currency: Option<&Currency>) -> PyResult<Py<PyAny>> {
        match self.realized_pnls(currency) {
            Some(pnls) => {
                let list = pyo3::types::PyList::empty(py);
                for (position_id, ts_event, pnl) in pnls {
                    list.append((position_id.to_string(), ts_event.as_u64(), pnl))?;
                }
                Ok(list.into())
            }
            None => Ok(py.None()),
        }
    }

    /// Calculates total PnL including unrealized PnL if provided.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No currency is specified in a multi-currency portfolio.
    /// - The specified currency is not found in account balances.
    /// - The unrealized PnL currency does not match the specified currency.
    #[pyo3(name = "total_pnl")]
    fn py_total_pnl(
        &self,
        currency: Option<&Currency>,
        unrealized_pnl: Option<&Money>,
    ) -> PyResult<f64> {
        self.total_pnl(currency, unrealized_pnl)
            .map_err(to_pyvalue_err)
    }

    /// Calculates total PnL as a percentage of starting balance.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No currency is specified in a multi-currency portfolio.
    /// - The specified currency is not found in account balances.
    /// - The unrealized PnL currency does not match the specified currency.
    #[pyo3(name = "total_pnl_percentage")]
    fn py_total_pnl_percentage(
        &self,
        currency: Option<&Currency>,
        unrealized_pnl: Option<&Money>,
    ) -> PyResult<f64> {
        self.total_pnl_percentage(currency, unrealized_pnl)
            .map_err(to_pyvalue_err)
    }

    /// Gets formatted PnL statistics as strings.
    ///
    /// # Errors
    ///
    /// Returns an error if PnL statistics calculation fails.
    #[pyo3(name = "get_stats_pnls_formatted")]
    fn py_get_stats_pnls_formatted(
        &self,
        currency: Option<&Currency>,
        unrealized_pnl: Option<&Money>,
    ) -> PyResult<Vec<String>> {
        self.get_stats_pnls_formatted(currency, unrealized_pnl)
            .map_err(to_pyvalue_err)
    }

    /// Gets formatted return statistics as strings.
    #[pyo3(name = "get_stats_returns_formatted")]
    fn py_get_stats_returns_formatted(&self) -> Vec<String> {
        self.get_stats_returns_formatted()
    }

    /// Gets formatted position-return statistics as strings.
    #[pyo3(name = "get_stats_position_returns_formatted")]
    fn py_get_stats_position_returns_formatted(&self) -> Vec<String> {
        self.get_stats_position_returns_formatted()
    }

    /// Gets formatted portfolio-return statistics as strings.
    #[pyo3(name = "get_stats_portfolio_returns_formatted")]
    fn py_get_stats_portfolio_returns_formatted(&self) -> Vec<String> {
        self.get_stats_portfolio_returns_formatted()
    }

    /// Gets formatted general statistics as strings.
    #[pyo3(name = "get_stats_general_formatted")]
    fn py_get_stats_general_formatted(&self) -> Vec<String> {
        self.get_stats_general_formatted()
    }
}
