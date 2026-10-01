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

//! The operator set and the conventions that define each operator.
//!
//! A factor definition is only reproducible if each operator has one meaning. This module names
//! the operators and states the convention exactly: what an absent input produces, population
//! versus sample dispersion, how ties are ranked, whether a regression carries an intercept, and
//! how a neutralisation weights its groups. The pure functions here are the definitions the
//! compiled expression tree calls, so a value computed from a definition and a value pinned by a
//! test cannot disagree.
//!
//! Two families exist, and they differ in the axis they reduce over:
//!
//! - Time-series operators reduce over one instrument's history. They read the current
//!   observation and earlier ones, never later ones.
//! - Cross-sectional operators reduce over the instruments present at one timestamp. The set is
//!   the point-in-time universe the caller supplies, so a cross section can never include an
//!   instrument that had not yet joined or that had already left.
//!
//! Every operator yields an explicit absence ([`None`]) when its inputs are absent or a statistic
//! is undefined - a window that is not full, a zero denominator, a group with no observations.
//! An undefined statistic is never reported as zero.

use std::cmp::Ordering;
use std::fmt;

/// The side a transform is computed on.
///
/// A transform declared [`Learning`](Self::Learning) may be fit over the whole learning window,
/// including observations later than the value it produces. A transform declared
/// [`Inference`](Self::Inference) may read only the current observation and earlier ones, so a
/// value it produces at a timestamp is knowable at that timestamp. The side is part of a
/// definition's identity and is enforced when a definition is parsed and when it is used.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TransformSide {
    /// Computed on the learning side, where fitting over the learning window is permitted.
    Learning,
    /// Computed on the inference side, where only current and earlier observations may be read.
    Inference,
}

impl TransformSide {
    /// Returns the canonical lowercase name of the side.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Learning => "learning",
            Self::Inference => "inference",
        }
    }
}

impl fmt::Display for TransformSide {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A typed operator over a factor expression.
///
/// Parameterised variants carry their window or lag directly, so the parameter is part of the
/// operator's identity and therefore of the definition's digest. A window or lag is at least one
/// (`0` is rejected at parse time); a lag reads strictly earlier observations and a lead reads
/// strictly later ones.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Operator {
    /// Arithmetic negation of its single argument.
    Neg,
    /// Addition.
    Add,
    /// Subtraction, `lhs - rhs`.
    Sub,
    /// Multiplication.
    Mul,
    /// Division; a zero divisor yields an explicit absence.
    Div,
    /// Less than, evaluated as `1.0` when true and `0.0` when false.
    Less,
    /// Less than or equal.
    LessOrEqual,
    /// Greater than.
    Greater,
    /// Greater than or equal.
    GreaterOrEqual,
    /// Exact equality, evaluated as `1.0` when true and `0.0` when false.
    Equal,
    /// Inequality.
    NotEqual,
    /// Reads the observation `n` periods before the current one; the lag is at least one.
    Lag(usize),
    /// Reads the observation `n` periods after the current one; the lead is at least one.
    ///
    /// A lead reads future data, so it is permitted only on the learning side. On the inference
    /// side the parser rejects it as a typed error.
    Lead(usize),
    /// Rolling arithmetic mean over the trailing window, including the current observation.
    RollingMean(usize),
    /// Rolling population standard deviation over the trailing window (divides by the window
    /// length, not the window length minus one).
    RollingStd(usize),
    /// Rolling Pearson correlation of two series over the trailing window, computed with the
    /// population covariance and population standard deviations.
    RollingCorrelation(usize),
    /// Rolling rank of the current observation within the trailing window, ascending from `1`
    /// with tied values taking the average of the positions they span.
    RollingRank(usize),
    /// Rolling residual of the current observation from an ordinary least squares regression of
    /// the target on the factor over the trailing window, with an intercept.
    RollingRegressionResidual(usize),
    /// Cross-sectional rank, ascending from `1`, with tied values taking the average rank.
    CrossSectionalRank,
    /// Cross-sectional scale: each value divided by the sum of the absolute values of the cross
    /// section, so the result sums to one in absolute value.
    CrossSectionalScale,
    /// Cross-sectional sum, broadcast to every instrument in the cross section.
    CrossSectionalSum,
    /// Cross-sectional neutralisation: each value less the equal-weighted mean of its group.
    Neutralize,
}

impl Operator {
    /// Returns the canonical function name of the operator.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Neg => "neg",
            Self::Add => "add",
            Self::Sub => "sub",
            Self::Mul => "mul",
            Self::Div => "div",
            Self::Less => "less",
            Self::LessOrEqual => "less_or_equal",
            Self::Greater => "greater",
            Self::GreaterOrEqual => "greater_or_equal",
            Self::Equal => "equal",
            Self::NotEqual => "not_equal",
            Self::Lag(_) => "lag",
            Self::Lead(_) => "lead",
            Self::RollingMean(_) => "rolling_mean",
            Self::RollingStd(_) => "rolling_std",
            Self::RollingCorrelation(_) => "rolling_correlation",
            Self::RollingRank(_) => "rolling_rank",
            Self::RollingRegressionResidual(_) => "rolling_regression_residual",
            Self::CrossSectionalRank => "rank",
            Self::CrossSectionalScale => "scale",
            Self::CrossSectionalSum => "sum",
            Self::Neutralize => "neutralize",
        }
    }

    /// Returns the window or lag parameter, if the operator has one.
    #[must_use]
    pub const fn parameter(self) -> Option<usize> {
        match self {
            Self::Lag(n)
            | Self::Lead(n)
            | Self::RollingMean(n)
            | Self::RollingStd(n)
            | Self::RollingCorrelation(n)
            | Self::RollingRank(n)
            | Self::RollingRegressionResidual(n) => Some(n),
            _ => None,
        }
    }

    /// Returns the infix symbol of a binary operator, if it has one.
    #[must_use]
    pub const fn infix_symbol(self) -> Option<&'static str> {
        match self {
            Self::Add => Some("+"),
            Self::Sub => Some("-"),
            Self::Mul => Some("*"),
            Self::Div => Some("/"),
            Self::Less => Some("<"),
            Self::LessOrEqual => Some("<="),
            Self::Greater => Some(">"),
            Self::GreaterOrEqual => Some(">="),
            Self::Equal => Some("=="),
            Self::NotEqual => Some("!="),
            _ => None,
        }
    }
}

/// Returns the arithmetic mean of a window.
///
/// The window is the trailing slice with the current observation last. An empty window is an
/// explicit absence.
#[must_use]
pub fn rolling_mean(window: &[f64]) -> Option<f64> {
    if window.is_empty() {
        return None;
    }
    Some(window.iter().sum::<f64>() / window.len() as f64)
}

/// Returns the population standard deviation of a window.
///
/// The variance divides by the window length rather than the window length minus one. An empty
/// window is an explicit absence.
#[must_use]
pub fn rolling_std(window: &[f64]) -> Option<f64> {
    if window.is_empty() {
        return None;
    }
    let n = window.len() as f64;
    let mean = window.iter().sum::<f64>() / n;
    let variance = window
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / n;
    Some(variance.sqrt())
}

/// Returns the Pearson correlation of two equal-length windows.
///
/// The correlation is the population covariance divided by the product of the population standard
/// deviations. Windows shorter than two observations, mismatched lengths, or a zero denominator
/// are an explicit absence.
#[must_use]
pub fn rolling_correlation(lhs: &[f64], rhs: &[f64]) -> Option<f64> {
    if lhs.len() != rhs.len() || lhs.len() < 2 {
        return None;
    }
    let n = lhs.len() as f64;
    let mean_lhs = lhs.iter().sum::<f64>() / n;
    let mean_rhs = rhs.iter().sum::<f64>() / n;

    let mut covariance = 0.0;
    let mut variance_lhs = 0.0;
    let mut variance_rhs = 0.0;
    for (lhs, rhs) in lhs.iter().zip(rhs) {
        let dev_lhs = lhs - mean_lhs;
        let dev_rhs = rhs - mean_rhs;
        covariance += dev_lhs * dev_rhs;
        variance_lhs += dev_lhs * dev_lhs;
        variance_rhs += dev_rhs * dev_rhs;
    }

    let denominator = (variance_lhs * variance_rhs).sqrt();
    if denominator <= 0.0 {
        return None;
    }
    Some(covariance / denominator)
}

/// Returns the ascending rank of the current observation (the last element) within a window.
///
/// Ranks ascend from `1`; tied values take the average of the positions they span. An empty window
/// is an explicit absence.
#[must_use]
pub fn rolling_rank(window: &[f64]) -> Option<f64> {
    let current = *window.last()?;
    let mut less = 0usize;
    let mut equal = 0usize;
    for value in window {
        match value.partial_cmp(&current) {
            Some(Ordering::Less) => less += 1,
            Some(Ordering::Equal) => equal += 1,
            _ => {}
        }
    }

    let first = less as f64 + 1.0;
    let last = (less + equal) as f64;
    Some(f64::midpoint(first, last))
}

/// Returns the residual of the current observation from an OLS regression of `target` on `factor`.
///
/// The regression carries an intercept and is fit over the trailing window with the current
/// observation last. The residual is `target_last - (intercept + slope * factor_last)`. Windows
/// shorter than two observations, mismatched lengths, or a zero factor variance are an explicit
/// absence.
#[must_use]
pub fn rolling_regression_residual(target: &[f64], factor: &[f64]) -> Option<f64> {
    if target.len() != factor.len() || target.len() < 2 {
        return None;
    }
    let n = target.len() as f64;
    let mean_target = target.iter().sum::<f64>() / n;
    let mean_factor = factor.iter().sum::<f64>() / n;

    let mut covariance = 0.0;
    let mut variance = 0.0;
    for (target, factor) in target.iter().zip(factor) {
        covariance += (factor - mean_factor) * (target - mean_target);
        variance += (factor - mean_factor) * (factor - mean_factor);
    }
    if variance <= 0.0 {
        return None;
    }

    let slope = covariance / variance;
    let intercept = mean_target - slope * mean_factor;
    let target_last = *target.last()?;
    let factor_last = *factor.last()?;
    Some(target_last - (intercept + slope * factor_last))
}

/// Returns the ascending cross-sectional ranks of a slice, aligned to the input.
///
/// Ranks ascend from `1`; tied values take the average of the positions they span. NaN values are
/// ordered last and rank together.
#[must_use]
pub fn cross_sectional_rank(values: &[f64]) -> Vec<f64> {
    let mut order: Vec<usize> = (0..values.len()).collect();
    order.sort_by(|&a, &b| values[a].partial_cmp(&values[b]).unwrap_or(Ordering::Equal));

    let mut ranks = vec![0.0; values.len()];
    let mut start = 0;
    while start < order.len() {
        let mut end = start;
        while end + 1 < order.len() && values[order[end + 1]] == values[order[start]] {
            end += 1;
        }
        let average = f64::midpoint((start + 1) as f64, (end + 1) as f64);
        for &index in order.iter().take(end + 1).skip(start) {
            ranks[index] = average;
        }
        start = end + 1;
    }
    ranks
}

/// Returns the cross-sectional scale of a slice: each value divided by the sum of the absolute
/// values.
///
/// A slice whose absolute values sum to zero is an explicit absence.
#[must_use]
pub fn cross_sectional_scale(values: &[f64]) -> Option<Vec<f64>> {
    let total: f64 = values.iter().map(|value| value.abs()).sum();
    if total <= 0.0 {
        return None;
    }
    Some(values.iter().map(|value| value / total).collect())
}

/// Returns the cross-sectional sum of a slice, broadcast to every instrument.
#[must_use]
pub fn cross_sectional_sum(values: &[f64]) -> f64 {
    values.iter().sum()
}

/// Returns the equal-weighted neutralisation of a slice against grouping keys, aligned to the
/// input.
///
/// Each value loses the mean of every value in its group; groups are compared by exact key
/// equality and are expected to be integral codes. Every group is weighted equally.
#[must_use]
pub fn neutralize(values: &[f64], groups: &[f64]) -> Vec<f64> {
    let mut result = vec![0.0; values.len()];
    for (index, value) in values.iter().enumerate() {
        let mut sum = 0.0;
        let mut count = 0usize;
        for (other, group) in values.iter().zip(groups) {
            if *group == groups[index] {
                sum += other;
                count += 1;
            }
        }
        result[index] = value - sum / count as f64;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rolling_std_is_population() {
        // Window [2, 4, 8]: mean 14/3, variance (56/3)/3 = 56/9, std sqrt(56)/3.
        let std = rolling_std(&[2.0, 4.0, 8.0]).unwrap();
        assert!((std - (56.0f64).sqrt() / 3.0).abs() < 1e-12);
    }

    #[test]
    fn rolling_rank_breaks_ties_by_average() {
        // [5, 5]: both span positions 1 and 2, so both rank 1.5.
        assert_eq!(rolling_rank(&[5.0, 5.0]), Some(1.5));
        // [2, 4, 8], current 8 is the largest, rank 3.
        assert_eq!(rolling_rank(&[2.0, 4.0, 8.0]), Some(3.0));
    }

    #[test]
    fn cross_sectional_rank_breaks_ties_by_average() {
        let ranks = cross_sectional_rank(&[10.0, 5.0, 5.0]);
        assert_eq!(ranks, vec![3.0, 1.5, 1.5]);
    }

    #[test]
    fn neutralisation_removes_the_group_mean() {
        // Values [1, 3, 10] with groups [1, 1, 2]: group 1 mean 2, group 2 mean 10.
        let neutral = neutralize(&[1.0, 3.0, 10.0], &[1.0, 1.0, 2.0]);
        assert_eq!(neutral, vec![-1.0, 1.0, 0.0]);
    }

    #[test]
    fn undefined_statistics_are_absent() {
        assert_eq!(rolling_mean(&[]), None);
        assert_eq!(rolling_correlation(&[1.0, 1.0], &[1.0, 2.0]), None);
        assert_eq!(cross_sectional_scale(&[0.0, 0.0]), None);
    }
}
