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

//! Variance ratio statistic.

use std::fmt::Display;

use nautilus_core::correctness::check_predicate_true;
use nautilus_model::position::Position;

use crate::{
    Returns,
    metric::{MetricDefinition, MetricDirection, MetricInput, MetricTag, MetricUnits},
    statistic::PortfolioStatistic,
};

/// Calculates the variance ratio of portfolio returns over an aggregation scale `q`.
///
/// The variance ratio compares the variance of `q`-period aggregated returns to `q`
/// times the variance of one-period returns. With `Var_1 = (1/n) * sum (x_t - mu)^2`,
/// `mu` the full-series mean, and `S_t = sum_{i=t..t+q-1} x_i` the overlapping `q`-period
/// sums for `t = 1..=n-q+1`:
///
/// `VR(q) = [ (1/(n-q+1)) * sum_t (S_t - q*mu)^2 ] / (q * Var_1)`
///
/// The aggregation sums `S_t` overlap rather than tile the series, which keeps every
/// observation in the estimate. Both variances divide by the number of observations
/// (the population divisor), so this is the finite-sample (biased) estimator rather than
/// an unbiased correction.
///
/// The diffusive value of the tool is `1.0`. A value above `1.0` indicates persistence on
/// the aggregation scale `q` (aggregated variance grows faster than linearly in `q`, as a
/// trend or momentum would produce), and a value below `1.0` indicates anti-persistence
/// (mean reversion, as an alternating series would produce).
///
/// Returns `None` when `n < 2q` (too few observations to aggregate) or when `Var_1` is
/// zero (the ratio is undefined).
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
pub struct VarianceRatio {
    /// The aggregation scale `q`, in observations (default: 2).
    period: usize,
}

impl VarianceRatio {
    /// Creates a new [`VarianceRatio`] instance.
    ///
    /// # Errors
    ///
    /// Returns an error if `period` is less than two.
    pub fn new(period: Option<usize>) -> anyhow::Result<Self> {
        let period = period.unwrap_or(2);
        check_predicate_true(period >= 2, "period must be at least 2")?;
        Ok(Self { period })
    }
}

impl Display for VarianceRatio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Variance Ratio (period {})", self.period)
    }
}

impl PortfolioStatistic for VarianceRatio {
    type Item = f64;

    fn name(&self) -> String {
        self.to_string()
    }

    fn definition(&self) -> MetricDefinition {
        MetricDefinition::new(
            "variance_ratio",
            "Variance Ratio (period {period})",
            MetricUnits::Ratio,
            MetricDirection::Informational,
            [MetricInput::Returns],
        )
        .with_count("period", self.period)
        .with_tags([MetricTag::Returns, MetricTag::Distribution])
    }

    fn calculate_from_returns(&self, raw_returns: &Returns) -> Option<Self::Item> {
        if !self.check_valid_returns(raw_returns) {
            return None;
        }

        let returns = self.downsample_to_daily_bins(raw_returns);
        let values: Vec<f64> = returns.values().copied().collect();
        let n = values.len();
        let q = self.period;
        if n < 2 * q {
            return None;
        }

        let n_f = n as f64;
        let mean = values.iter().sum::<f64>() / n_f;
        let var_one = values.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n_f;
        if var_one == 0.0 || !var_one.is_finite() {
            return None;
        }

        let overlap_count = n - q + 1;
        let mut sum_squared = 0.0;
        for window in values.windows(q) {
            let block_sum = window.iter().sum::<f64>();
            let deviation = block_sum - q as f64 * mean;
            sum_squared += deviation * deviation;
        }
        let aggregate_variance = sum_squared / overlap_count as f64;

        let ratio = aggregate_variance / (q as f64 * var_one);
        if !ratio.is_finite() {
            return None;
        }

        Some(ratio)
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
    use std::collections::BTreeMap;

    use nautilus_core::{UnixNanos, approx_eq};
    use rstest::rstest;

    use super::*;

    fn create_returns(values: &[f64]) -> BTreeMap<UnixNanos, f64> {
        values
            .iter()
            .enumerate()
            .map(|(i, &value)| {
                (
                    UnixNanos::from(1_600_000_000_000_000_000 + i as u64 * 86_400_000_000_000),
                    value,
                )
            })
            .collect()
    }

    #[rstest]
    fn test_period_two_of_a_four_value_series_matches_hand_computation() {
        // values = [1, 2, 3, 4], q = 2, n = 4, mean = 2.5.
        //   Var_1 = (2.25 + 0.25 + 0.25 + 2.25) / 4 = 5 / 4 = 1.25
        //   S_t = [3, 5, 7], q*mean = 5, deviations = [-2, 0, 2]
        //   aggregate variance = (4 + 0 + 4) / 3 = 8 / 3
        //   VR(2) = (8/3) / (2 * 1.25) = (8/3) / 2.5 = 16 / 15
        let statistic = VarianceRatio::new(Some(2)).unwrap();
        let result = statistic
            .calculate_from_returns(&create_returns(&[1.0, 2.0, 3.0, 4.0]))
            .unwrap();
        assert!(approx_eq!(f64, result, 16.0 / 15.0, epsilon = 1e-12));
    }

    #[rstest]
    fn test_an_increasing_series_is_persistent_at_period_two() {
        let statistic = VarianceRatio::new(Some(2)).unwrap();
        let result = statistic
            .calculate_from_returns(&create_returns(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]))
            .unwrap();
        assert!(result > 1.0, "expected persistence, got {result}");
    }

    #[rstest]
    fn test_series_shorter_than_twice_the_period_is_undefined() {
        let statistic = VarianceRatio::new(Some(2)).unwrap();
        let result = statistic.calculate_from_returns(&create_returns(&[1.0, 2.0, 3.0]));
        assert_eq!(result, None);
    }

    #[rstest]
    fn test_zero_variance_series_is_undefined() {
        let statistic = VarianceRatio::new(Some(2)).unwrap();
        let result = statistic.calculate_from_returns(&create_returns(&[2.0, 2.0, 2.0, 2.0]));
        assert_eq!(result, None);
    }

    #[rstest]
    fn test_display_name_carries_the_period_but_metric_id_does_not() {
        let statistic = VarianceRatio::new(Some(5)).unwrap();
        assert_eq!(statistic.name(), "Variance Ratio (period 5)");
        assert_eq!(statistic.definition().id(), "variance_ratio");
    }

    #[rstest]
    #[case(Some(0))]
    #[case(Some(1))]
    fn test_new_rejects_a_period_below_two(#[case] period: Option<usize>) {
        assert!(VarianceRatio::new(period).is_err());
    }

    #[rstest]
    #[case(None)]
    #[case(Some(2))]
    #[case(Some(20))]
    fn test_new_accepts_a_period_of_at_least_two(#[case] period: Option<usize>) {
        assert!(VarianceRatio::new(period).is_ok());
    }
}
