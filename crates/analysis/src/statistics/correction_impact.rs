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

//! The measured effect of a data-quality correction on a declared outcome metric.
//!
//! A correction - a flagged record or a dropped one - changes the stream a result is computed
//! from, and a count of what was fixed says nothing about what fixing it did to the numbers the
//! study is about. The report therefore names the declared outcome metric and carries its value on
//! the stream as the gate received it and on the stream after the correction, so the correction
//! and its effect are read together. The delta is `corrected - uncorrected` and is reported even
//! when it is exactly zero: a correction that changed nothing is a fact, not an omission.
//!
//! The report refuses to exist without a declared outcome metric. A correction applied without one
//! cannot be reported, so it is refused at the point of applying it rather than given a metric the
//! caller did not choose. The refusal is an error naming what is missing, never a silent no-op.

use std::fmt::Display;

use nautilus_model::position::Position;

use crate::{
    Returns,
    metric::{MetricDefinition, MetricDirection, MetricInput, MetricStage, MetricUnits},
    statistic::PortfolioStatistic,
};

/// The refusal raised when a correction is reported without the outcome metric it changed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CorrectionImpactError {
    /// No outcome metric was declared, so the correction's effect cannot be reported.
    MissingOutcomeMetric,
}

impl Display for CorrectionImpactError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingOutcomeMetric => {
                f.write_str("a correction requires a declared outcome metric; none was declared")
            }
        }
    }
}

impl std::error::Error for CorrectionImpactError {}

/// Reports a correction's measured effect on a declared outcome metric.
///
/// The report is the pair of measurements a correction is read through: `uncorrected` is the
/// metric on the stream as the gate received it and `corrected` is the metric on the stream after
/// the correction. The delta is their difference, so a correction that lowered a Sharpe ratio from
/// 1.518 to 0.589 reports a delta of -0.929 beside the count of records it removed.
#[repr(C)]
#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.analysis", from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.analysis")
)]
pub struct CorrectionImpactReport {
    metric: String,
    uncorrected: f64,
    corrected: f64,
}

impl CorrectionImpactReport {
    /// Creates the report for `outcome_metric` measured on both streams.
    ///
    /// # Errors
    ///
    /// Returns [`CorrectionImpactError::MissingOutcomeMetric`] when `outcome_metric` is `None` or
    /// blank. The refusal is raised here rather than defaulting a metric for the caller: a report
    /// that cannot say what moved cannot be read, so it is not built.
    pub fn new(
        outcome_metric: Option<&str>,
        uncorrected: f64,
        corrected: f64,
    ) -> Result<Self, CorrectionImpactError> {
        let metric = outcome_metric
            .map(str::trim)
            .filter(|metric| !metric.is_empty())
            .ok_or(CorrectionImpactError::MissingOutcomeMetric)?;

        Ok(Self {
            metric: metric.to_string(),
            uncorrected,
            corrected,
        })
    }

    /// Returns the declared outcome metric the correction was measured against.
    #[must_use]
    pub fn metric(&self) -> &str {
        &self.metric
    }

    /// Returns the metric's value on the stream as the gate received it.
    #[must_use]
    pub const fn uncorrected(&self) -> f64 {
        self.uncorrected
    }

    /// Returns the metric's value on the stream after the correction.
    #[must_use]
    pub const fn corrected(&self) -> f64 {
        self.corrected
    }

    /// Returns the change the correction made to the declared metric.
    ///
    /// The delta is `corrected - uncorrected`. It is exactly zero when the correction changed
    /// nothing, and it is reported rather than omitted.
    #[must_use]
    pub fn delta(&self) -> f64 {
        self.corrected - self.uncorrected
    }
}

impl Display for CorrectionImpactReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Correction Impact: {} (uncorrected {}, corrected {}, delta {})",
            self.metric,
            crate::metric::format_number(self.uncorrected),
            crate::metric::format_number(self.corrected),
            crate::metric::format_number(self.delta())
        )
    }
}

impl PortfolioStatistic for CorrectionImpactReport {
    type Item = f64;

    fn name(&self) -> String {
        format!("Correction Impact: {}", self.metric)
    }

    fn definition(&self) -> MetricDefinition {
        MetricDefinition::new(
            "correction_impact",
            "Correction Impact: {metric} (uncorrected {uncorrected} -> corrected {corrected}, \
             delta {delta})",
            MetricUnits::Ratio,
            MetricDirection::Informational,
            [MetricInput::Returns],
        )
        .with_parameter("metric", self.metric.clone())
        .with_number("uncorrected", self.uncorrected)
        .with_number("corrected", self.corrected)
        .with_number("delta", self.delta())
        .with_stage(MetricStage::Account)
    }

    fn calculate_from_returns(&self, _returns: &Returns) -> Option<Self::Item> {
        Some(self.delta())
    }

    fn calculate_from_realized_pnls(&self, _realized_pnls: &[f64]) -> Option<Self::Item> {
        None
    }

    fn calculate_from_positions(&self, _positions: &[Position]) -> Option<Self::Item> {
        None
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    fn test_report_names_the_metric_and_both_streams() {
        let report = CorrectionImpactReport::new(Some("sharpe_ratio"), 1.518, 0.589).unwrap();

        assert_eq!(report.metric(), "sharpe_ratio");
        assert_eq!(report.uncorrected(), 1.518);
        assert_eq!(report.corrected(), 0.589);
        assert!((report.delta() + 0.929).abs() < 1e-12);
        assert_eq!(
            report.to_string(),
            "Correction Impact: sharpe_ratio (uncorrected 1.518, corrected 0.589, delta -0.929)"
        );
    }

    #[rstest]
    fn test_report_refuses_without_a_declared_metric() {
        let error = CorrectionImpactReport::new(None, 1.0, 0.5).unwrap_err();

        assert_eq!(error, CorrectionImpactError::MissingOutcomeMetric);
        assert_eq!(
            error.to_string(),
            "a correction requires a declared outcome metric; none was declared"
        );
        assert!(CorrectionImpactReport::new(Some("  "), 1.0, 0.5).is_err());
    }

    #[rstest]
    fn test_report_keeps_a_zero_delta_rather_than_omitting_it() {
        let report = CorrectionImpactReport::new(Some("returns_volatility"), 0.25, 0.25).unwrap();

        assert_eq!(report.delta(), 0.0);
        assert!(report.to_string().contains("delta 0"));
    }

    #[rstest]
    fn test_statistic_reports_the_delta_and_declares_its_metric() {
        let report = CorrectionImpactReport::new(Some("sharpe_ratio"), 1.518, 0.589).unwrap();
        let definition = report.definition();

        assert_eq!(definition.id(), "correction_impact");
        assert_eq!(definition.stage(), Some(MetricStage::Account));
        assert!(definition.is_defined_over(MetricInput::Returns));
        assert_eq!(report.name(), "Correction Impact: sharpe_ratio");
        assert_eq!(
            report.calculate_from_returns(&Returns::default()),
            Some(-0.929)
        );
    }
}
