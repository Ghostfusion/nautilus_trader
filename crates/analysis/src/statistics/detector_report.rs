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

//! A detector's confusion matrix and the rates that read it, with the positive base rate always
//! printed beside the accuracy.
//!
//! A detector evaluated on a rare positive class can report a high accuracy while its errors are
//! almost all false positives: the accuracy is then largely a restatement of the base rate, not a
//! statement about the detector. The report therefore carries the whole confusion matrix - true and
//! false positives, true and false negatives - and the rates that read it, each named so the rates
//! cannot be confused:
//!
//! - The **positive base rate** (prevalence) is the share of evaluated records whose label is
//!   positive, `(true positives + false negatives) / records`. It is the number an accuracy on a
//!   rare class mostly restates.
//! - The **precision** (positive predictive value) is the share of the records the detector marked
//!   positive that are truly positive, `true positives / (true positives + false positives)`.
//! - The **false-discovery rate** is the share of the records the detector marked positive that are
//!   truly negative, `false positives / (true positives + false positives)`. It is `1 - precision`
//!   and is a statement about the detector's own positives, not about the population, which is why
//!   it is not a base rate and is never read as one.
//! - The **recall** (sensitivity, true-positive rate) is the share of the truly positive records
//!   that the detector marked positive, `true positives / (true positives + false negatives)`.
//! - The **F1** is the harmonic mean of precision and recall.
//! - The **accuracy** is the share of all evaluated records that were classified correctly,
//!   `(true positives + true negatives) / records`.
//!
//! The accuracy and its base rate are one report and are never separable: the report refuses to be
//! constructed from an accuracy stated without the base rate, rather than defaulting a plausible
//! base rate for the caller.

use std::fmt::Display;

use nautilus_model::position::Position;

use crate::{
    Returns,
    metric::{MetricDefinition, MetricDirection, MetricInput, MetricStage, MetricUnits},
    statistic::PortfolioStatistic,
};

/// The refusal raised when a detector report cannot be read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DetectorReportError {
    /// The decisions and labels did not pair one to one, so no matrix can be formed.
    LengthMismatch {
        /// The number of decisions supplied.
        decisions: usize,
        /// The number of labels supplied.
        labels: usize,
    },
    /// No record was supplied, so no matrix, accuracy or base rate exists.
    EmptyPopulation,
    /// An accuracy was stated without the positive base rate it must be printed beside.
    MissingBaseRate,
    /// An accuracy and base rate were stated without the confusion matrix that fixes them.
    MissingConfusionMatrix,
}

impl Display for DetectorReportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LengthMismatch { decisions, labels } => write!(
                f,
                "a detector report requires one decision and one label per record; \
                 got {decisions} decisions and {labels} labels"
            ),
            Self::EmptyPopulation => f.write_str(
                "a detector report requires at least one evaluated record; none was supplied",
            ),
            Self::MissingBaseRate => f.write_str(
                "a detector report requires the positive base rate beside its accuracy; \
                 an accuracy with no base rate is refused rather than given a default",
            ),
            Self::MissingConfusionMatrix => f.write_str(
                "a detector report requires the confusion matrix that fixes its counts, precision, \
                 recall and F1; a stated accuracy and base rate alone are refused",
            ),
        }
    }
}

impl std::error::Error for DetectorReportError {}

/// A detector's confusion matrix and the rates that read it.
///
/// The counts are the whole confusion matrix: `true_positives` and `true_negatives` are the records
/// classified correctly, `false_positives` and `false_negatives` the records classified wrongly.
/// Every rate is derived from them, so the accuracy cannot be reported without the positive base
/// rate that interprets it.
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
pub struct DetectorReport {
    true_positives: u64,
    false_positives: u64,
    true_negatives: u64,
    false_negatives: u64,
}

impl DetectorReport {
    /// Creates the report from the four counts of the confusion matrix.
    ///
    /// # Errors
    ///
    /// Returns [`DetectorReportError::EmptyPopulation`] when all four counts are zero, because an
    /// empty population has neither an accuracy nor a base rate to report.
    pub fn new(
        true_positives: u64,
        false_positives: u64,
        true_negatives: u64,
        false_negatives: u64,
    ) -> Result<Self, DetectorReportError> {
        if true_positives + false_positives + true_negatives + false_negatives == 0 {
            return Err(DetectorReportError::EmptyPopulation);
        }

        Ok(Self {
            true_positives,
            false_positives,
            true_negatives,
            false_negatives,
        })
    }

    /// Creates the report from the detector's decisions and the ground-truth labels.
    ///
    /// `decisions` is what the detector marked, `labels` what was true; the two are paired by
    /// index. A record marked positive on a positive label is a true positive, a positive on a
    /// negative label a false positive, and so on.
    ///
    /// # Errors
    ///
    /// Returns [`DetectorReportError::LengthMismatch`] when the two slices differ in length, since
    /// the pairs, and therefore the matrix, would be undefined, and
    /// [`DetectorReportError::EmptyPopulation`] when both are empty.
    pub fn from_decisions_and_labels(
        decisions: &[bool],
        labels: &[bool],
    ) -> Result<Self, DetectorReportError> {
        if decisions.len() != labels.len() {
            return Err(DetectorReportError::LengthMismatch {
                decisions: decisions.len(),
                labels: labels.len(),
            });
        }

        let mut true_positives = 0_u64;
        let mut false_positives = 0_u64;
        let mut true_negatives = 0_u64;
        let mut false_negatives = 0_u64;

        for (decision, label) in decisions.iter().zip(labels) {
            match (decision, label) {
                (true, true) => true_positives += 1,
                (true, false) => false_positives += 1,
                (false, false) => true_negatives += 1,
                (false, true) => false_negatives += 1,
            }
        }

        Self::new(
            true_positives,
            false_positives,
            true_negatives,
            false_negatives,
        )
    }

    /// Refuses a report stated as an accuracy rather than as a confusion matrix.
    ///
    /// This constructor exists so the refusal is a named seam rather than a caller-side
    /// convention. There is no report to build from a headline number: an accuracy stated without
    /// its positive base rate is refused rather than defaulted, and an accuracy with a base rate
    /// still cannot populate the counts, precision, recall or F1 the report must carry.
    ///
    /// # Errors
    ///
    /// Returns [`DetectorReportError::MissingBaseRate`] when `base_rate` is `None`, and
    /// [`DetectorReportError::MissingConfusionMatrix`] when it is `Some`.
    pub fn from_accuracy(
        accuracy: f64,
        base_rate: Option<f64>,
    ) -> Result<Self, DetectorReportError> {
        // The accuracy is the value being refused, not a value used to build the report.
        let _ = accuracy;

        match base_rate {
            None => Err(DetectorReportError::MissingBaseRate),
            Some(_) => Err(DetectorReportError::MissingConfusionMatrix),
        }
    }

    /// Returns the number of records the detector classified correctly as positive.
    #[must_use]
    pub const fn true_positives(&self) -> u64 {
        self.true_positives
    }

    /// Returns the number of records the detector marked positive that were truly negative.
    #[must_use]
    pub const fn false_positives(&self) -> u64 {
        self.false_positives
    }

    /// Returns the number of records the detector classified correctly as negative.
    #[must_use]
    pub const fn true_negatives(&self) -> u64 {
        self.true_negatives
    }

    /// Returns the number of records the detector marked negative that were truly positive.
    #[must_use]
    pub const fn false_negatives(&self) -> u64 {
        self.false_negatives
    }

    /// Returns the number of evaluated records.
    #[must_use]
    pub const fn total(&self) -> u64 {
        self.true_positives + self.false_positives + self.true_negatives + self.false_negatives
    }

    /// Returns the share of evaluated records whose label is positive, `(TP + FN) / N`.
    ///
    /// This is the positive base rate. A detector's accuracy on a rare positive class is largely a
    /// restatement of it, so the accuracy is never reported without it.
    #[must_use]
    pub fn base_rate(&self) -> f64 {
        (self.true_positives + self.false_negatives) as f64 / self.total() as f64
    }

    /// Returns the share of records classified correctly, `(TP + TN) / N`.
    #[must_use]
    pub fn accuracy(&self) -> f64 {
        (self.true_positives + self.true_negatives) as f64 / self.total() as f64
    }

    /// Returns precision, `TP / (TP + FP)`: of the records marked positive, the share truly
    /// positive.
    ///
    /// Returns `None` when the detector marked no record positive, since precision is then `0 / 0`.
    #[must_use]
    pub fn precision(&self) -> Option<f64> {
        let marked_positive = self.true_positives + self.false_positives;

        (marked_positive != 0).then(|| self.true_positives as f64 / marked_positive as f64)
    }

    /// Returns recall, `TP / (TP + FN)`: of the truly positive records, the share marked positive.
    ///
    /// Returns `None` when the population has no positive label, since recall is then `0 / 0`.
    #[must_use]
    pub fn recall(&self) -> Option<f64> {
        let positive = self.true_positives + self.false_negatives;

        (positive != 0).then(|| self.true_positives as f64 / positive as f64)
    }

    /// Returns the false-discovery rate, `FP / (TP + FP)`: of the records marked positive, the
    /// share truly negative.
    ///
    /// This is a statement about the detector's own positives, not about the population, and is
    /// `1 - precision` wherever precision is defined. Returns `None` when the detector marked no
    /// record positive, since the rate is then `0 / 0`.
    #[must_use]
    pub fn false_discovery_rate(&self) -> Option<f64> {
        let marked_positive = self.true_positives + self.false_positives;

        (marked_positive != 0).then(|| self.false_positives as f64 / marked_positive as f64)
    }

    /// Returns the F1 score, the harmonic mean of precision and recall.
    ///
    /// Returns `None` when either precision or recall is undefined. When both are defined but sum
    /// to zero, the F1 is exactly zero.
    #[must_use]
    pub fn f1(&self) -> Option<f64> {
        match (self.precision(), self.recall()) {
            (Some(precision), Some(recall)) => {
                let denominator = precision + recall;

                Some(if denominator == 0.0 {
                    0.0
                } else {
                    2.0 * precision * recall / denominator
                })
            }
            _ => None,
        }
    }

    /// Returns the accuracy and base-rate rows as one inseparable pair.
    ///
    /// The two are returned together so no caller can render the accuracy without the base rate
    /// that makes it interpretable.
    #[must_use]
    pub fn accuracy_rows(&self) -> [(&'static str, f64); 2] {
        [
            ("accuracy", self.accuracy()),
            ("base rate", self.base_rate()),
        ]
    }
}

/// Renders a rate that may be undefined.
fn optional_rate(value: Option<f64>) -> String {
    value.map_or_else(|| "undefined".to_string(), crate::metric::format_number)
}

impl Display for DetectorReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Detector Report: accuracy {} with base rate {} (tp {}, fp {}, tn {}, fn {}; \
             precision {}, recall {}, f1 {}, false discovery rate {})",
            crate::metric::format_number(self.accuracy()),
            crate::metric::format_number(self.base_rate()),
            self.true_positives,
            self.false_positives,
            self.true_negatives,
            self.false_negatives,
            optional_rate(self.precision()),
            optional_rate(self.recall()),
            optional_rate(self.f1()),
            optional_rate(self.false_discovery_rate()),
        )
    }
}

impl PortfolioStatistic for DetectorReport {
    type Item = f64;

    fn name(&self) -> String {
        "Detector Report".to_string()
    }

    fn definition(&self) -> MetricDefinition {
        MetricDefinition::new(
            "detector_report",
            "Detector Report (accuracy {accuracy}, base rate {base_rate})",
            MetricUnits::Fraction,
            MetricDirection::Informational,
            [MetricInput::Returns],
        )
        .with_parameter("accuracy", crate::metric::format_number(self.accuracy()))
        .with_parameter("base_rate", crate::metric::format_number(self.base_rate()))
        .with_parameter("precision", optional_rate(self.precision()))
        .with_parameter("recall", optional_rate(self.recall()))
        .with_parameter("f1", optional_rate(self.f1()))
        .with_parameter(
            "false_discovery_rate",
            optional_rate(self.false_discovery_rate()),
        )
        .with_parameter("true_positives", self.true_positives.to_string())
        .with_parameter("false_positives", self.false_positives.to_string())
        .with_parameter("true_negatives", self.true_negatives.to_string())
        .with_parameter("false_negatives", self.false_negatives.to_string())
        .with_stage(MetricStage::Forecast)
    }

    fn calculate_from_returns(&self, _returns: &Returns) -> Option<Self::Item> {
        Some(self.accuracy())
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
    fn test_report_carries_the_matrix_and_the_rates_that_read_it() {
        // 1000 records, 2 of them truly positive: a 0.002 base rate. The detector marks 10
        // positive and gets one right, so its accuracy is 0.99 while precision is 0.1 and nine
        // in ten of its alarms are false.
        let report = DetectorReport::new(1, 9, 989, 1).unwrap();

        assert_eq!(report.true_positives(), 1);
        assert_eq!(report.false_positives(), 9);
        assert_eq!(report.true_negatives(), 989);
        assert_eq!(report.false_negatives(), 1);
        assert_eq!(report.total(), 1000);
        assert_eq!(report.accuracy(), 0.99);
        assert_eq!(report.base_rate(), 0.002);
        assert_eq!(report.precision(), Some(0.1));
        assert_eq!(report.recall(), Some(0.5));
        assert_eq!(report.false_discovery_rate(), Some(0.9));
        assert!((report.f1().unwrap() - 1.0 / 6.0).abs() < 1e-12);
    }

    #[rstest]
    fn test_a_high_accuracy_on_a_rare_event_prints_the_low_base_rate_beside_it() {
        let report = DetectorReport::new(1, 9, 989, 1).unwrap();
        let rendered = report.to_string();

        assert_eq!(report.accuracy(), 0.99);
        assert_eq!(report.base_rate(), 0.002);
        assert!(rendered.contains("accuracy 0.99"));
        assert!(rendered.contains("base rate 0.002"));
        assert!(rendered.contains("precision 0.1"));
        assert!(rendered.contains("false discovery rate 0.9"));
    }

    #[rstest]
    fn test_a_balanced_report_reads_its_matrix() {
        // 100 records, half of them truly positive: base rate 0.5, accuracy 0.9, and precision,
        // recall and F1 all 0.9.
        let report = DetectorReport::new(45, 5, 45, 5).unwrap();

        assert_eq!(report.accuracy(), 0.9);
        assert_eq!(report.base_rate(), 0.5);
        assert_eq!(report.precision(), Some(0.9));
        assert_eq!(report.recall(), Some(0.9));
        assert_eq!(report.f1(), Some(0.9));
        assert_eq!(report.false_discovery_rate(), Some(0.1));
    }

    #[rstest]
    fn test_a_detector_marking_every_record_positive_reads_the_base_rate_as_its_accuracy() {
        // Ten records, one truly positive, and every record marked positive: no true negatives
        // and no false negatives. The accuracy is then exactly the base rate, as is the precision,
        // and the recall is 1.0.
        let decisions = [true; 10];
        let labels = [
            true, false, false, false, false, false, false, false, false, false,
        ];
        let report = DetectorReport::from_decisions_and_labels(&decisions, &labels).unwrap();

        assert_eq!(report.true_positives(), 1);
        assert_eq!(report.false_positives(), 9);
        assert_eq!(report.true_negatives(), 0);
        assert_eq!(report.false_negatives(), 0);
        assert_eq!(report.accuracy(), report.base_rate());
        assert_eq!(report.precision(), Some(report.base_rate()));
        assert_eq!(report.recall(), Some(1.0));
    }

    #[rstest]
    fn test_report_refuses_an_accuracy_without_a_base_rate() {
        let error = DetectorReport::from_accuracy(0.99, None).unwrap_err();

        assert_eq!(error, DetectorReportError::MissingBaseRate);
        assert_eq!(
            error.to_string(),
            "a detector report requires the positive base rate beside its accuracy; \
             an accuracy with no base rate is refused rather than given a default"
        );

        // A base rate without the matrix is still no report: the counts, precision, recall and F1
        // would have to be invented.
        assert_eq!(
            DetectorReport::from_accuracy(0.99, Some(0.002)).unwrap_err(),
            DetectorReportError::MissingConfusionMatrix
        );
    }

    #[rstest]
    fn test_report_refuses_a_mismatched_decision_and_label_count() {
        let error = DetectorReport::from_decisions_and_labels(&[true, false], &[true]).unwrap_err();

        assert_eq!(
            error,
            DetectorReportError::LengthMismatch {
                decisions: 2,
                labels: 1,
            }
        );
        assert_eq!(
            error.to_string(),
            "a detector report requires one decision and one label per record; \
             got 2 decisions and 1 labels"
        );
    }

    #[rstest]
    fn test_report_refuses_an_empty_population() {
        let error = DetectorReport::new(0, 0, 0, 0).unwrap_err();

        assert_eq!(error, DetectorReportError::EmptyPopulation);
        assert!(DetectorReport::from_decisions_and_labels(&[], &[]).is_err());
    }

    #[rstest]
    fn test_accuracy_and_its_base_rate_are_returned_as_one_pair() {
        let report = DetectorReport::new(1, 9, 989, 1).unwrap();

        assert_eq!(
            report.accuracy_rows(),
            [("accuracy", 0.99), ("base rate", 0.002)]
        );
        // The rendered report and the declared title both carry the base rate beside the accuracy,
        // so neither can be read alone.
        assert!(report.to_string().contains("with base rate"));
        assert!(report.definition().title().contains("base rate"));
    }

    #[rstest]
    fn test_statistic_reports_the_accuracy_and_declares_its_metric() {
        let report = DetectorReport::new(1, 9, 989, 1).unwrap();
        let definition = report.definition();

        assert_eq!(definition.id(), "detector_report");
        assert_eq!(definition.stage(), Some(MetricStage::Forecast));
        assert!(definition.is_defined_over(MetricInput::Returns));
        assert_eq!(report.name(), "Detector Report");
        assert_eq!(
            report.calculate_from_returns(&Returns::default()),
            Some(0.99)
        );
    }
}
