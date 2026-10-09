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

//! Optimization objectives and constraints over the existing portfolio statistics.
//!
//! An [`Objective`] is a weighted combination of directed metric terms, and a [`Constraint`]
//! is a bound a metric value must satisfy. Both are pure, deterministic functions of a
//! `statistic name -> value` map (the same map shape the analyzer produces), so the objective
//! model exists once, in Rust, and a Python orchestrator supplies only the values.
//!
//! # Metric names
//!
//! A metric is named by the exact string the statistic's own [`PortfolioStatistic::name`]
//! returns, for example `"Sharpe Ratio (simple, sample, 252 days)"` or `"Max Drawdown (simple)"`. The accepted set is
//! derived from the crate's built-in statistics rather than a hand-written table, so the names
//! cannot drift from the statistics themselves. A name outside that set is rejected at
//! construction with [`ObjectiveError::UnknownMetric`].
//!
//! # Direction
//!
//! Each term carries a [`ObjectiveDirection`]. The objective is the sum, in declaration order,
//! of `weight * value` for a [`Maximize`](ObjectiveDirection::Maximize) term and
//! `-(weight * value)` for a [`Minimize`](ObjectiveDirection::Minimize) term. The score is what
//! a search algorithm maximises; the direction is always applied to the metric's reported value.
//!
//! The caller must therefore know the sign convention of each metric. For example,
//! [`MaxDrawdown`](crate::statistics::max_drawdown::MaxDrawdown) reports a *negative* fraction,
//! so a shallower drawdown is a larger value and is preferred with
//! [`Maximize`](ObjectiveDirection::Maximize) (equivalently, `Minimize` with a negative weight).
//! A metric that is reported as a positive magnitude and is better when smaller (returns
//! volatility, for example) is preferred with [`Minimize`](ObjectiveDirection::Minimize).
//!
//! # Determinism and ties
//!
//! Evaluation adds the term contributions sequentially in declaration order and performs no
//! rounding, clamping, or defaulting: a metric referenced by a term or constraint that is
//! absent from the supplied values is a [`ObjectiveError::MissingMetricValue`] error, never a
//! zero. Two candidates that produce the same score are not ordered by the objective;
//! tie-breaking is the caller's decision. Weights and bounds must be finite.

use std::{fmt::Display, sync::Arc, sync::LazyLock};

use ahash::AHashMap;

use crate::{
    analyzer::Statistic,
    statistics::{
        alpha::Alpha, average_monthly_return::AverageMonthlyReturn,
        average_trade_duration::AverageTradeDuration, beta_ratio::BetaRatio,
        breakeven_cost::BreakevenCost, cagr::CAGR, calmar_ratio::CalmarRatio,
        cost_basis_points::CostBasisPoints, down_capture_ratio::DownCaptureRatio,
        expectancy::Expectancy, expected_shortfall::ExpectedShortfall,
        exponentially_weighted_sharpe::ExponentiallyWeightedSharpe, exposure_ratio::ExposureRatio,
        gross_return::GrossReturn, information_ratio::InformationRatio, long_ratio::LongRatio,
        loser_avg::AvgLoser, loser_max::MaxLoser, loser_min::MinLoser, max_drawdown::MaxDrawdown,
        max_drawdown_duration::MaxDrawdownDuration, net_return::NetReturn, omega_ratio::OmegaRatio,
        profit_factor::ProfitFactor, returns_avg::ReturnsAverage,
        returns_avg_loss::ReturnsAverageLoss, returns_avg_win::ReturnsAverageWin,
        returns_kurtosis::ReturnsKurtosis, returns_skewness::ReturnsSkewness,
        returns_volatility::ReturnsVolatility, risk_return_ratio::RiskReturnRatio,
        sharpe_ratio::SharpeRatio, sortino_ratio::SortinoRatio, tail_ratio::TailRatio,
        total_commissions::TotalCommissions, total_turnover::TotalTurnover,
        tracking_error::TrackingError, treynor_ratio::TreynorRatio, ulcer_index::UlcerIndex,
        up_capture_ratio::UpCaptureRatio, value_at_risk::ValueAtRisk, win_loss_ratio::WinLossRatio,
        win_rate::WinRate, winner_avg::AvgWinner, winner_max::MaxWinner, winner_min::MinWinner,
        winning_month_share::WinningMonthShare,
    },
};

/// The direction in which an objective term rewards a metric.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(
        eq,
        eq_int,
        frozen,
        hash,
        module = "nautilus_trader.analysis",
        from_py_object,
        rename_all = "SCREAMING_SNAKE_CASE",
    )
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass_enum(module = "nautilus_trader.analysis")
)]
pub enum ObjectiveDirection {
    /// A larger reported metric value is preferred.
    Maximize,
    /// A smaller reported metric value is preferred.
    Minimize,
}

impl ObjectiveDirection {
    /// Returns the sign this direction applies to a term's weighted value.
    #[must_use]
    pub const fn sign(self) -> f64 {
        match self {
            Self::Maximize => 1.0,
            Self::Minimize => -1.0,
        }
    }
}

/// How a constraint compares a metric value against its bound.
///
/// Both comparisons are inclusive of the bound: a value exactly equal to the bound satisfies
/// the constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(
        eq,
        eq_int,
        frozen,
        hash,
        module = "nautilus_trader.analysis",
        from_py_object,
        rename_all = "SCREAMING_SNAKE_CASE",
    )
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass_enum(module = "nautilus_trader.analysis")
)]
pub enum ConstraintComparison {
    /// The value must be greater than or equal to the bound.
    AtLeast,
    /// The value must be less than or equal to the bound.
    AtMost,
}

/// A typed error from constructing or evaluating an objective or constraint.
#[derive(Debug, Clone, PartialEq)]
pub enum ObjectiveError {
    /// The objective was constructed with no terms.
    EmptyObjective,
    /// The metric name does not match any built-in statistic.
    UnknownMetric(String),
    /// No value was supplied for a metric referenced by an objective term or constraint.
    MissingMetricValue(String),
    /// A term weight was not finite.
    NonFiniteWeight(f64),
    /// A constraint bound was not finite.
    NonFiniteBound(f64),
}

impl Display for ObjectiveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyObjective => write!(f, "objective must have at least one term"),
            Self::UnknownMetric(name) => write!(f, "unknown metric `{name}`"),
            Self::MissingMetricValue(name) => write!(f, "no value supplied for metric `{name}`"),
            Self::NonFiniteWeight(weight) => write!(f, "weight must be finite, was {weight}"),
            Self::NonFiniteBound(bound) => write!(f, "bound must be finite, was {bound}"),
        }
    }
}

impl std::error::Error for ObjectiveError {}

/// A single weighted, directed metric term of an [`Objective`].
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.analysis", from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.analysis")
)]
pub struct ObjectiveTerm {
    metric: String,
    weight: f64,
    direction: ObjectiveDirection,
}

impl ObjectiveTerm {
    /// Creates a new [`ObjectiveTerm`].
    ///
    /// # Errors
    ///
    /// Returns an error if `weight` is not finite, or if `metric` does not name a built-in
    /// statistic.
    pub fn new(
        metric: impl Into<String>,
        weight: f64,
        direction: ObjectiveDirection,
    ) -> Result<Self, ObjectiveError> {
        if !weight.is_finite() {
            return Err(ObjectiveError::NonFiniteWeight(weight));
        }

        let metric = metric.into();
        if !is_supported_metric(&metric) {
            return Err(ObjectiveError::UnknownMetric(metric));
        }

        Ok(Self {
            metric,
            weight,
            direction,
        })
    }

    /// Returns the metric name this term references.
    #[must_use]
    pub fn metric(&self) -> &str {
        &self.metric
    }

    /// Returns the term weight.
    #[must_use]
    pub const fn weight(&self) -> f64 {
        self.weight
    }

    /// Returns the term direction.
    #[must_use]
    pub const fn direction(&self) -> ObjectiveDirection {
        self.direction
    }

    /// Returns this term's signed contribution to the objective score.
    ///
    /// # Errors
    ///
    /// Returns an error if `values` has no entry for the term's metric.
    pub fn contribution(&self, values: &AHashMap<String, f64>) -> Result<f64, ObjectiveError> {
        let value = self
            .lookup(values)
            .ok_or_else(|| ObjectiveError::MissingMetricValue(self.metric.clone()))?;

        Ok(self.weight * self.direction.sign() * value)
    }

    fn lookup(&self, values: &AHashMap<String, f64>) -> Option<f64> {
        values.get(&self.metric).copied()
    }
}

/// A weighted combination of directed metric terms, maximised by a search algorithm.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.analysis", skip_from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.analysis")
)]
pub struct Objective {
    terms: Vec<ObjectiveTerm>,
}

impl Objective {
    /// Creates a new [`Objective`] from `terms`.
    ///
    /// # Errors
    ///
    /// Returns an error if `terms` is empty.
    pub fn new(terms: Vec<ObjectiveTerm>) -> Result<Self, ObjectiveError> {
        if terms.is_empty() {
            return Err(ObjectiveError::EmptyObjective);
        }

        Ok(Self { terms })
    }

    /// Returns the objective terms in declaration order.
    #[must_use]
    pub fn terms(&self) -> &[ObjectiveTerm] {
        &self.terms
    }

    /// Evaluates the objective score for `values`.
    ///
    /// The score is the sum of the term contributions in declaration order.
    ///
    /// # Errors
    ///
    /// Returns an error if `values` has no entry for any referenced metric.
    pub fn evaluate(&self, values: &AHashMap<String, f64>) -> Result<f64, ObjectiveError> {
        let mut score = 0.0;

        for term in &self.terms {
            score += term.contribution(values)?;
        }

        Ok(score)
    }
}

/// A bound a metric value must satisfy.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.analysis", skip_from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.analysis")
)]
pub struct Constraint {
    metric: String,
    comparison: ConstraintComparison,
    bound: f64,
}

impl Constraint {
    /// Creates a new [`Constraint`].
    ///
    /// # Errors
    ///
    /// Returns an error if `bound` is not finite, or if `metric` does not name a built-in
    /// statistic.
    pub fn new(
        metric: impl Into<String>,
        comparison: ConstraintComparison,
        bound: f64,
    ) -> Result<Self, ObjectiveError> {
        if !bound.is_finite() {
            return Err(ObjectiveError::NonFiniteBound(bound));
        }

        let metric = metric.into();
        if !is_supported_metric(&metric) {
            return Err(ObjectiveError::UnknownMetric(metric));
        }

        Ok(Self {
            metric,
            comparison,
            bound,
        })
    }

    /// Returns the metric name this constraint references.
    #[must_use]
    pub fn metric(&self) -> &str {
        &self.metric
    }

    /// Returns the constraint comparison.
    #[must_use]
    pub const fn comparison(&self) -> ConstraintComparison {
        self.comparison
    }

    /// Returns the constraint bound.
    #[must_use]
    pub const fn bound(&self) -> f64 {
        self.bound
    }

    /// Returns whether the constraint is satisfied by `values`.
    ///
    /// The comparison is inclusive of the bound.
    ///
    /// # Errors
    ///
    /// Returns an error if `values` has no entry for the constraint's metric.
    pub fn is_satisfied(&self, values: &AHashMap<String, f64>) -> Result<bool, ObjectiveError> {
        let value = values
            .get(&self.metric)
            .copied()
            .ok_or_else(|| ObjectiveError::MissingMetricValue(self.metric.clone()))?;

        Ok(match self.comparison {
            ConstraintComparison::AtLeast => value >= self.bound,
            ConstraintComparison::AtMost => value <= self.bound,
        })
    }
}

/// Returns whether `name` is the accepted name of a built-in statistic.
#[must_use]
fn is_supported_metric(name: &str) -> bool {
    supported_metric_names()
        .binary_search_by(|candidate| candidate.as_str().cmp(name))
        .is_ok()
}

/// The accepted metric names, sorted, derived from the built-in statistics' own names.
static SUPPORTED_METRIC_NAMES: LazyLock<Vec<String>> = LazyLock::new(|| {
    let mut names: Vec<String> = builtin_statistics()
        .iter()
        .map(|statistic| statistic.name())
        .collect();
    names.sort();
    names.dedup();
    names
});

/// Returns the accepted metric names, sorted.
///
/// The names are the `name()` output of the crate's built-in statistics at their default
/// parameters, so a statistic constructed with non-default parameters (for example
/// `SharpeRatio::new(Some(365))`) produces a name outside this set.
#[must_use]
pub fn supported_metric_names() -> &'static [String] {
    &SUPPORTED_METRIC_NAMES
}

/// Returns one instance of every built-in statistic, at default parameters.
///
/// This is the matching source for [`supported_metric_names`]: the accepted strings are these
/// statistics' own names, so they cannot drift from the statistics.
pub(crate) fn builtin_statistics() -> Vec<Statistic> {
    vec![
        // Returns-based
        Arc::new(CAGR::new(None)),
        Arc::new(CalmarRatio::new(None)),
        Arc::new(ExponentiallyWeightedSharpe::new(None, None)),
        Arc::new(MaxDrawdown::new()),
        Arc::new(MaxDrawdownDuration::new()),
        Arc::new(OmegaRatio::new(None)),
        Arc::new(ProfitFactor {}),
        Arc::new(ReturnsAverage {}),
        Arc::new(ReturnsAverageLoss {}),
        Arc::new(ReturnsAverageWin {}),
        Arc::new(ReturnsKurtosis::new()),
        Arc::new(ReturnsSkewness::new()),
        Arc::new(ReturnsVolatility::new(None)),
        Arc::new(RiskReturnRatio {}),
        Arc::new(SharpeRatio::new(None)),
        Arc::new(SortinoRatio::new(None)),
        Arc::new(TailRatio {}),
        Arc::new(UlcerIndex::new()),
        Arc::new(ValueAtRisk::new(None)),
        Arc::new(ExpectedShortfall::new(None)),
        // Frame-based
        Arc::new(BreakevenCost::new()),
        Arc::new(CostBasisPoints::new()),
        Arc::new(GrossReturn::new()),
        Arc::new(NetReturn::new()),
        Arc::new(TotalCommissions::new()),
        Arc::new(TotalTurnover::new()),
        Arc::new(WinningMonthShare {}),
        Arc::new(AverageMonthlyReturn::new(None)),
        Arc::new(ExposureRatio {}),
        // PnL-based
        Arc::new(Expectancy {}),
        Arc::new(AvgLoser {}),
        Arc::new(MaxLoser {}),
        Arc::new(MinLoser {}),
        Arc::new(WinRate {}),
        Arc::new(WinLossRatio {}),
        Arc::new(AvgWinner {}),
        Arc::new(MaxWinner {}),
        Arc::new(MinWinner {}),
        // Position-based
        Arc::new(LongRatio::new(None)),
        Arc::new(AverageTradeDuration::new(None)),
        // Benchmark-relative
        Arc::new(Alpha::new(None, None)),
        Arc::new(BetaRatio::new()),
        Arc::new(DownCaptureRatio::new(None)),
        Arc::new(InformationRatio::new(None)),
        Arc::new(TrackingError::new(None)),
        Arc::new(TreynorRatio::new(None, None)),
        Arc::new(UpCaptureRatio::new(None)),
    ]
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    fn values(entries: &[(&str, f64)]) -> AHashMap<String, f64> {
        entries
            .iter()
            .map(|(name, value)| ((*name).to_string(), *value))
            .collect()
    }

    #[rstest]
    fn test_objective_weighted_combination() {
        let objective = Objective::new(vec![
            ObjectiveTerm::new(
                "Sharpe Ratio (simple, sample, 252 days)",
                2.0,
                ObjectiveDirection::Maximize,
            )
            .unwrap(),
            ObjectiveTerm::new("Max Drawdown (simple)", 1.0, ObjectiveDirection::Maximize).unwrap(),
        ])
        .unwrap();

        let score = objective
            .evaluate(&values(&[
                ("Sharpe Ratio (simple, sample, 252 days)", 2.0),
                ("Max Drawdown (simple)", -0.25),
            ]))
            .unwrap();

        // 2.0 * 2.0 + 1.0 * -0.25
        assert_eq!(score, 3.75);
    }

    #[rstest]
    fn test_objective_direction_convention() {
        let input = values(&[("Returns Volatility (simple, sample, 252 days)", 0.25)]);

        let maximize = Objective::new(vec![
            ObjectiveTerm::new(
                "Returns Volatility (simple, sample, 252 days)",
                4.0,
                ObjectiveDirection::Maximize,
            )
            .unwrap(),
        ])
        .unwrap();
        let minimize = Objective::new(vec![
            ObjectiveTerm::new(
                "Returns Volatility (simple, sample, 252 days)",
                4.0,
                ObjectiveDirection::Minimize,
            )
            .unwrap(),
        ])
        .unwrap();

        assert_eq!(maximize.evaluate(&input).unwrap(), 1.0);
        assert_eq!(minimize.evaluate(&input).unwrap(), -1.0);
    }

    #[rstest]
    fn test_objective_direction_on_negative_max_drawdown() {
        let input = values(&[("Max Drawdown (simple)", -0.25)]);

        let maximize = Objective::new(vec![
            ObjectiveTerm::new("Max Drawdown (simple)", 1.0, ObjectiveDirection::Maximize).unwrap(),
        ])
        .unwrap();
        let minimize = Objective::new(vec![
            ObjectiveTerm::new("Max Drawdown (simple)", 1.0, ObjectiveDirection::Minimize).unwrap(),
        ])
        .unwrap();

        // Maximize penalises a deeper drawdown; Minimize rewards it because the value is negative.
        assert_eq!(maximize.evaluate(&input).unwrap(), -0.25);
        assert_eq!(minimize.evaluate(&input).unwrap(), 0.25);
    }

    #[rstest]
    #[case(1.5, true)]
    #[case(1.499_999, false)]
    fn test_constraint_at_least_boundary_is_inclusive(#[case] value: f64, #[case] expected: bool) {
        let constraint = Constraint::new(
            "Sharpe Ratio (simple, sample, 252 days)",
            ConstraintComparison::AtLeast,
            1.5,
        )
        .unwrap();

        let verdict = constraint
            .is_satisfied(&values(&[(
                "Sharpe Ratio (simple, sample, 252 days)",
                value,
            )]))
            .unwrap();

        assert_eq!(verdict, expected);
    }

    #[rstest]
    #[case(-0.3, true)]
    #[case(-0.299_999_9, false)]
    fn test_constraint_at_most_boundary_is_inclusive(#[case] value: f64, #[case] expected: bool) {
        let constraint =
            Constraint::new("Max Drawdown (simple)", ConstraintComparison::AtMost, -0.3).unwrap();

        let verdict = constraint
            .is_satisfied(&values(&[("Max Drawdown (simple)", value)]))
            .unwrap();

        assert_eq!(verdict, expected);
    }

    #[rstest]
    fn test_objective_term_unknown_metric_errors() {
        let result = ObjectiveTerm::new("Not A Metric", 1.0, ObjectiveDirection::Maximize);

        assert_eq!(
            result,
            Err(ObjectiveError::UnknownMetric("Not A Metric".to_string()))
        );
    }

    #[rstest]
    fn test_constraint_unknown_metric_errors() {
        let result = Constraint::new("Not A Metric", ConstraintComparison::AtLeast, 1.0);

        assert_eq!(
            result,
            Err(ObjectiveError::UnknownMetric("Not A Metric".to_string()))
        );
    }

    #[rstest]
    fn test_objective_missing_metric_value_errors() {
        let objective = Objective::new(vec![
            ObjectiveTerm::new(
                "Sharpe Ratio (simple, sample, 252 days)",
                1.0,
                ObjectiveDirection::Maximize,
            )
            .unwrap(),
        ])
        .unwrap();

        let result = objective.evaluate(&values(&[]));

        assert_eq!(
            result,
            Err(ObjectiveError::MissingMetricValue(
                "Sharpe Ratio (simple, sample, 252 days)".to_string()
            ))
        );
    }

    #[rstest]
    fn test_constraint_missing_metric_value_errors() {
        let constraint =
            Constraint::new("Max Drawdown (simple)", ConstraintComparison::AtLeast, -0.5).unwrap();

        let result = constraint.is_satisfied(&values(&[]));

        assert_eq!(
            result,
            Err(ObjectiveError::MissingMetricValue(
                "Max Drawdown (simple)".to_string()
            ))
        );
    }

    #[rstest]
    fn test_objective_empty_errors() {
        assert_eq!(Objective::new(vec![]), Err(ObjectiveError::EmptyObjective));
    }

    #[rstest]
    fn test_objective_term_non_finite_weight_errors() {
        let result = ObjectiveTerm::new(
            "Sharpe Ratio (simple, sample, 252 days)",
            f64::NAN,
            ObjectiveDirection::Maximize,
        );

        assert!(matches!(result, Err(ObjectiveError::NonFiniteWeight(_))));
    }

    #[rstest]
    fn test_supported_metric_names_are_the_builtin_statistic_names() {
        let expected: Vec<String> = [
            "Alpha (simple, sample, 252 days)",
            "Average (Return, simple)",
            "Average Loss (Return, simple)",
            "Average Monthly Return (all)",
            "Average Trade Duration (all, days)",
            "Average Win (Return, simple)",
            "Avg Loser",
            "Avg Winner",
            "Beta (simple, sample)",
            "Breakeven Cost (basis points of turnover)",
            "CAGR (simple, 252 days)",
            "Calmar Ratio (simple, 252 days)",
            "Cost (basis points of turnover)",
            "Down Capture Ratio (simple, 252 days)",
            "Expectancy",
            "Expected Shortfall (simple, confidence 0.95)",
            "Exponentially Weighted Sharpe (simple, population, 252 days, halflife 6)",
            "Exposure Ratio (share of periods held)",
            "Gross Return",
            "Information Ratio (simple, sample, 252 days)",
            "Long Ratio",
            "Max Drawdown (simple)",
            "Max Drawdown Duration (days)",
            "Max Loser",
            "Max Winner",
            "Min Loser",
            "Min Winner",
            "Net Return",
            "Omega Ratio (simple, threshold 0)",
            "Profit Factor (simple)",
            "Returns Kurtosis (simple, sample)",
            "Returns Skewness (simple, sample)",
            "Returns Volatility (simple, sample, 252 days)",
            "Risk Return Ratio (simple, sample)",
            "Sharpe Ratio (simple, sample, 252 days)",
            "Sortino Ratio (simple, population, 252 days)",
            "Tail Ratio (simple)",
            "Total Commissions",
            "Total Turnover",
            "Tracking Error (simple, sample, 252 days)",
            "Treynor Ratio (simple, sample, 252 days)",
            "Ulcer Index (simple, population)",
            "Up Capture Ratio (simple, 252 days)",
            "Value at Risk (simple, confidence 0.95)",
            "Win Rate",
            "Win/Loss Ratio",
            "Winning Month Share",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        assert_eq!(supported_metric_names(), expected.as_slice());
        assert!(!is_supported_metric("Not A Metric"));
    }
}
