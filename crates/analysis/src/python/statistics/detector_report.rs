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

use std::collections::BTreeMap;

use nautilus_core::python::to_pyvalue_err;
use nautilus_model::position::Position;
use pyo3::prelude::*;

use super::transform_returns;
use crate::{statistic::PortfolioStatistic, statistics::detector_report::DetectorReport};

#[pymethods]
#[pyo3_stub_gen::derive::gen_stub_pymethods]
impl DetectorReport {
    /// A detector's confusion matrix and the rates that read it.
    ///
    /// The counts are the whole confusion matrix: `true_positives` and `true_negatives` are the records
    /// classified correctly, `false_positives` and `false_negatives` the records classified wrongly.
    /// Every rate is derived from them, so the accuracy cannot be reported without the positive base
    /// rate that interprets it.
    #[new]
    fn py_new(
        true_positives: u64,
        false_positives: u64,
        true_negatives: u64,
        false_negatives: u64,
    ) -> PyResult<Self> {
        Self::new(
            true_positives,
            false_positives,
            true_negatives,
            false_negatives,
        )
        .map_err(|e| to_pyvalue_err(e.to_string()))
    }

    /// Creates the report from the detector's decisions and the ground-truth labels.
    ///
    /// `decisions` is what the detector marked, `labels` what was true; the two are paired by
    /// index. A record marked positive on a positive label is a true positive, a positive on a
    /// negative label a false positive, and so on.
    ///
    /// # Errors
    ///
    /// Returns `DetectorReportError.LengthMismatch` when the two slices differ in length, since
    /// the pairs, and therefore the matrix, would be undefined, and
    /// `DetectorReportError.EmptyPopulation` when both are empty.
    #[staticmethod]
    #[pyo3(name = "from_decisions_and_labels")]
    #[expect(clippy::needless_pass_by_value)]
    fn py_from_decisions_and_labels(decisions: Vec<bool>, labels: Vec<bool>) -> PyResult<Self> {
        Self::from_decisions_and_labels(&decisions, &labels)
            .map_err(|e| to_pyvalue_err(e.to_string()))
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
    /// Returns `DetectorReportError.MissingBaseRate` when `base_rate` is `None`, and
    /// `DetectorReportError.MissingConfusionMatrix` when it is `Some`.
    #[staticmethod]
    #[pyo3(name = "from_accuracy")]
    #[pyo3(signature = (accuracy, base_rate=None))]
    fn py_from_accuracy(accuracy: f64, base_rate: Option<f64>) -> PyResult<Self> {
        Self::from_accuracy(accuracy, base_rate).map_err(|e| to_pyvalue_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        self.to_string()
    }

    #[getter]
    #[pyo3(name = "name")]
    fn py_name(&self) -> String {
        self.name()
    }

    /// Returns the number of records the detector classified correctly as positive.
    #[getter]
    #[pyo3(name = "true_positives")]
    const fn py_true_positives(&self) -> u64 {
        self.true_positives()
    }

    /// Returns the number of records the detector marked positive that were truly negative.
    #[getter]
    #[pyo3(name = "false_positives")]
    const fn py_false_positives(&self) -> u64 {
        self.false_positives()
    }

    /// Returns the number of records the detector classified correctly as negative.
    #[getter]
    #[pyo3(name = "true_negatives")]
    const fn py_true_negatives(&self) -> u64 {
        self.true_negatives()
    }

    /// Returns the number of records the detector marked negative that were truly positive.
    #[getter]
    #[pyo3(name = "false_negatives")]
    const fn py_false_negatives(&self) -> u64 {
        self.false_negatives()
    }

    /// Returns the share of records classified correctly, `(TP + TN) / N`.
    #[getter]
    #[pyo3(name = "accuracy")]
    fn py_accuracy(&self) -> f64 {
        self.accuracy()
    }

    /// Returns the share of evaluated records whose label is positive, `(TP + FN) / N`.
    ///
    /// This is the positive base rate. A detector's accuracy on a rare positive class is largely a
    /// restatement of it, so the accuracy is never reported without it.
    #[getter]
    #[pyo3(name = "base_rate")]
    fn py_base_rate(&self) -> f64 {
        self.base_rate()
    }

    /// Returns precision, `TP / (TP + FP)`: of the records marked positive, the share truly
    /// positive.
    ///
    /// Returns `None` when the detector marked no record positive, since precision is then `0 / 0`.
    #[getter]
    #[pyo3(name = "precision")]
    fn py_precision(&self) -> Option<f64> {
        self.precision()
    }

    /// Returns recall, `TP / (TP + FN)`: of the truly positive records, the share marked positive.
    ///
    /// Returns `None` when the population has no positive label, since recall is then `0 / 0`.
    #[getter]
    #[pyo3(name = "recall")]
    fn py_recall(&self) -> Option<f64> {
        self.recall()
    }

    /// Returns the F1 score, the harmonic mean of precision and recall.
    ///
    /// Returns `None` when either precision or recall is undefined. When both are defined but sum
    /// to zero, the F1 is exactly zero.
    #[getter]
    #[pyo3(name = "f1")]
    fn py_f1(&self) -> Option<f64> {
        self.f1()
    }

    /// Returns the false-discovery rate, `FP / (TP + FP)`: of the records marked positive, the
    /// share truly negative.
    ///
    /// This is a statement about the detector's own positives, not about the population, and is
    /// `1 - precision` wherever precision is defined. Returns `None` when the detector marked no
    /// record positive, since the rate is then `0 / 0`.
    #[getter]
    #[pyo3(name = "false_discovery_rate")]
    fn py_false_discovery_rate(&self) -> Option<f64> {
        self.false_discovery_rate()
    }

    #[pyo3(name = "calculate_from_returns")]
    #[expect(clippy::needless_pass_by_value)]
    fn py_calculate_from_returns(&mut self, raw_returns: BTreeMap<u64, f64>) -> Option<f64> {
        self.calculate_from_returns(&transform_returns(&raw_returns))
    }

    #[pyo3(name = "calculate_from_realized_pnls")]
    fn py_calculate_from_realized_pnls(&mut self, _realized_pnls: Vec<f64>) -> Option<f64> {
        None
    }

    #[pyo3(name = "calculate_from_positions")]
    fn py_calculate_from_positions(&mut self, _positions: Vec<Position>) -> Option<f64> {
        None
    }
}
