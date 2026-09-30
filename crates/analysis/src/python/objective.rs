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

use std::collections::HashMap;

use ahash::AHashMap;
use nautilus_core::python::to_pyvalue_err;
use pyo3::prelude::*;

use crate::objective::{
    Constraint, ConstraintComparison, Objective, ObjectiveDirection, ObjectiveTerm,
};

#[pymethods]
#[pyo3_stub_gen::derive::gen_stub_pymethods]
impl ObjectiveTerm {
    /// A single weighted, directed metric term of an `Objective`.
    #[new]
    #[pyo3(signature = (metric, weight, direction))]
    fn py_new(metric: String, weight: f64, direction: ObjectiveDirection) -> PyResult<Self> {
        Self::new(metric, weight, direction).map_err(to_pyvalue_err)
    }

    /// Returns the metric name this term references.
    #[getter]
    #[pyo3(name = "metric")]
    fn py_metric(&self) -> String {
        self.metric().to_string()
    }

    /// Returns the term weight.
    #[getter]
    #[pyo3(name = "weight")]
    const fn py_weight(&self) -> f64 {
        self.weight()
    }

    /// Returns the term direction.
    #[getter]
    #[pyo3(name = "direction")]
    const fn py_direction(&self) -> ObjectiveDirection {
        self.direction()
    }

    /// Returns this term's signed contribution to the objective score.
    ///
    /// # Errors
    ///
    /// Returns an error if `values` has no entry for the term's metric.
    #[pyo3(name = "contribution")]
    fn py_contribution(&self, values: HashMap<String, f64>) -> PyResult<f64> {
        let values: AHashMap<String, f64> = values.into_iter().collect();
        self.contribution(&values).map_err(to_pyvalue_err)
    }

    fn __repr__(&self) -> String {
        format!(
            "ObjectiveTerm(metric={:?}, weight={}, direction={:?})",
            self.metric(),
            self.weight(),
            self.direction()
        )
    }
}

#[pymethods]
#[pyo3_stub_gen::derive::gen_stub_pymethods]
impl Objective {
    /// A weighted combination of directed metric terms, maximised by a search algorithm.
    #[new]
    #[pyo3(signature = (terms))]
    fn py_new(terms: Vec<ObjectiveTerm>) -> PyResult<Self> {
        Self::new(terms).map_err(to_pyvalue_err)
    }

    /// Returns the objective terms in declaration order.
    #[getter]
    #[pyo3(name = "terms")]
    fn py_terms(&self) -> Vec<ObjectiveTerm> {
        self.terms().to_vec()
    }

    /// Evaluates the objective score for `values`.
    ///
    /// The score is the sum of the term contributions in declaration order.
    ///
    /// # Errors
    ///
    /// Returns an error if `values` has no entry for any referenced metric.
    #[pyo3(name = "evaluate")]
    fn py_evaluate(&self, values: HashMap<String, f64>) -> PyResult<f64> {
        let values: AHashMap<String, f64> = values.into_iter().collect();
        self.evaluate(&values).map_err(to_pyvalue_err)
    }

    fn __repr__(&self) -> String {
        format!("Objective(terms={})", self.terms().len())
    }
}

#[pymethods]
#[pyo3_stub_gen::derive::gen_stub_pymethods]
impl Constraint {
    /// A bound a metric value must satisfy.
    #[new]
    #[pyo3(signature = (metric, comparison, bound))]
    fn py_new(metric: String, comparison: ConstraintComparison, bound: f64) -> PyResult<Self> {
        Self::new(metric, comparison, bound).map_err(to_pyvalue_err)
    }

    /// Returns the metric name this constraint references.
    #[getter]
    #[pyo3(name = "metric")]
    fn py_metric(&self) -> String {
        self.metric().to_string()
    }

    /// Returns the constraint comparison.
    #[getter]
    #[pyo3(name = "comparison")]
    const fn py_comparison(&self) -> ConstraintComparison {
        self.comparison()
    }

    /// Returns the constraint bound.
    #[getter]
    #[pyo3(name = "bound")]
    const fn py_bound(&self) -> f64 {
        self.bound()
    }

    /// Returns whether the constraint is satisfied by `values`.
    ///
    /// The comparison is inclusive of the bound.
    ///
    /// # Errors
    ///
    /// Returns an error if `values` has no entry for the constraint's metric.
    #[pyo3(name = "is_satisfied")]
    fn py_is_satisfied(&self, values: HashMap<String, f64>) -> PyResult<bool> {
        let values: AHashMap<String, f64> = values.into_iter().collect();
        self.is_satisfied(&values).map_err(to_pyvalue_err)
    }

    fn __repr__(&self) -> String {
        format!(
            "Constraint(metric={:?}, comparison={:?}, bound={})",
            self.metric(),
            self.comparison(),
            self.bound()
        )
    }
}
