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

//! Autocorrelation statistic.

use std::fmt::Display;

use nautilus_core::correctness::check_predicate_true;
use nautilus_model::position::Position;

use crate::{
    Returns,
    metric::{MetricDefinition, MetricDirection, MetricInput, MetricTag, MetricUnits},
    statistic::PortfolioStatistic,
};

/// Calculates the lag-`k` autocorrelation of portfolio returns.
///
/// The autocorrelation at lag `k` measures how strongly each return is related to the
/// return `k` observations later. It is the Pearson correlation of the series with its
/// own `k`-lagged copy:
///
/// `rho_k = sum_{t=1..n-k} (x_t - mu)(x_{t+k} - mu) / sum_{t=1..n} (x_t - mu)^2`
///
/// where `mu` is the mean of the whole series and `n` is the number of returns. Two
/// choices follow from that definition and are deliberate: both the mean `mu` and the
/// denominator sum over the full series, while the numerator uses only the `n - k`
/// overlapping pairs `(x_t, x_{t+k})` that a lag-`k` shift admits. Using the full-series
/// dispersion in the denominator keeps the measure bounded to the unit interval.
///
/// A positive value indicates persistence (a return above the mean tends to be followed
/// by another), a negative value indicates mean reversion, and zero indicates no linear
/// dependence at that lag.
///
/// Returns `None` for a series with `n <= k + 1` observations (too few overlapping pairs)
/// or with zero dispersion (the denominator is zero).
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
pub struct Autocorrelation {
    /// The lag, in observations (default: 1).
    lag: usize,
}

impl Autocorrelation {
    /// Creates a new [`Autocorrelation`] instance.
    ///
    /// # Errors
    ///
    /// Returns an error if `lag` is zero.
    pub fn new(lag: Option<usize>) -> anyhow::Result<Self> {
        let lag = lag.unwrap_or(1);
        check_predicate_true(lag > 0, "lag must be greater than zero")?;
        Ok(Self { lag })
    }
}

impl Display for Autocorrelation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Autocorrelation (lag {})", self.lag)
    }
}

impl PortfolioStatistic for Autocorrelation {
    type Item = f64;

    fn name(&self) -> String {
        self.to_string()
    }

    fn definition(&self) -> MetricDefinition {
        MetricDefinition::new(
            "autocorrelation",
            "Autocorrelation (lag {lag})",
            MetricUnits::Ratio,
            MetricDirection::Informational,
            [MetricInput::Returns],
        )
        .with_count("lag", self.lag)
        .with_tags([MetricTag::Returns, MetricTag::Distribution])
    }

    fn calculate_from_returns(&self, raw_returns: &Returns) -> Option<Self::Item> {
        if !self.check_valid_returns(raw_returns) {
            return None;
        }

        let returns = self.downsample_to_daily_bins(raw_returns);
        let values: Vec<f64> = returns.values().copied().collect();
        let n = values.len();
        if n <= self.lag + 1 {
            return None;
        }

        let n_f = n as f64;
        let mean = values.iter().sum::<f64>() / n_f;
        let denominator = values.iter().map(|x| (x - mean).powi(2)).sum::<f64>();
        if denominator == 0.0 || !denominator.is_finite() {
            return None;
        }

        let numerator = values
            .iter()
            .take(n - self.lag)
            .enumerate()
            .map(|(t, x)| (x - mean) * (values[t + self.lag] - mean))
            .sum::<f64>();

        let result = numerator / denominator;
        if !result.is_finite() {
            return None;
        }

        Some(result)
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
    fn test_lag_one_of_a_linear_series_matches_hand_computation() {
        // values = [1, 2, 3, 4, 5], lag = 1, mean = 3.
        //   deviations: -2, -1, 0, 1, 2
        //   denominator = 4 + 1 + 0 + 1 + 4 = 10
        //   numerator = (-2)(-1) + (-1)(0) + (0)(1) + (1)(2) = 2 + 0 + 0 + 2 = 4
        //   rho_1 = 4 / 10 = 0.4
        let statistic = Autocorrelation::new(Some(1)).unwrap();
        let result = statistic
            .calculate_from_returns(&create_returns(&[1.0, 2.0, 3.0, 4.0, 5.0]))
            .unwrap();
        assert!(approx_eq!(f64, result, 0.4, epsilon = 1e-12));
    }

    #[rstest]
    fn test_zero_variance_series_has_no_autocorrelation() {
        let statistic = Autocorrelation::new(Some(1)).unwrap();
        let result = statistic.calculate_from_returns(&create_returns(&[0.01, 0.01, 0.01, 0.01]));
        assert_eq!(result, None);
    }

    #[rstest]
    #[case(&[1.0, 2.0], 1)]
    #[case(&[1.0, 2.0, 3.0], 2)]
    #[case(&[1.0, 2.0, 3.0], 3)]
    fn test_series_too_short_for_the_lag_has_no_autocorrelation(
        #[case] values: &[f64],
        #[case] lag: usize,
    ) {
        let statistic = Autocorrelation::new(Some(lag)).unwrap();
        let result = statistic.calculate_from_returns(&create_returns(values));
        assert_eq!(result, None);
    }

    #[rstest]
    fn test_display_name_carries_the_lag_but_metric_id_does_not() {
        let statistic = Autocorrelation::new(Some(4)).unwrap();
        assert_eq!(statistic.name(), "Autocorrelation (lag 4)");
        assert_eq!(statistic.definition().id(), "autocorrelation");
    }

    #[rstest]
    #[case(Some(0))]
    fn test_new_rejects_a_zero_lag(#[case] lag: Option<usize>) {
        assert!(Autocorrelation::new(lag).is_err());
    }

    #[rstest]
    #[case(None)]
    #[case(Some(1))]
    #[case(Some(5))]
    fn test_new_accepts_a_positive_lag(#[case] lag: Option<usize>) {
        assert!(Autocorrelation::new(lag).is_ok());
    }
}
