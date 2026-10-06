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

//! Rescaled range (Hurst exponent) statistic.

use nautilus_model::position::Position;

use crate::{
    Returns,
    metric::{MetricDefinition, MetricDirection, MetricInput, MetricTag, MetricUnits},
    statistic::PortfolioStatistic,
};

/// Calculates the Hurst exponent of portfolio returns by the rescaled-range method.
///
/// The rescaled range `R/S` of a series of length `m` is the range of its mean-adjusted
/// cumulative deviation divided by its dispersion. For a window size `m` the series is
/// split into non-overlapping blocks of `m` observations (a trailing partial block is
/// dropped); within each block the mean-adjusted cumulative deviation is accumulated and
/// `R = max(cumsum) - min(cumsum)`, while `S` is the population standard deviation of the
/// block. The block contributes `R/S`, and the mean over the blocks is the value for `m`:
///
/// `(R/S)_m = mean over blocks of ( max(cumsum) - min(cumsum) ) / S`
///
/// The method uses a ladder of window sizes `m` that runs over the powers of two from `4`
/// up to and including the largest power of two not exceeding `n / 2`. The Hurst estimate
/// is the least-squares slope of `log((R/S)_m)` against `log(m)` over the ladder. A slope
/// above `0.5` indicates persistence (the series trends), `0.5` is the memoryless value of
/// a random walk, and a slope below `0.5` indicates anti-persistence (the series reverts).
///
/// A block whose `S` is zero contributes nothing and is skipped, and a window size with no
/// contributing block is dropped from the ladder. At least three ladder points with a
/// finite positive `R/S` are required; otherwise the estimate is undefined and returns
/// `None`. A series with fewer than `16` observations is likewise undefined.
#[repr(C)]
#[derive(Debug, Clone, Default)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.analysis", from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.analysis")
)]
pub struct RescaledRange {}

impl RescaledRange {
    /// Creates a new [`RescaledRange`] instance.
    #[must_use]
    pub fn new() -> Self {
        Self {}
    }
}

impl PortfolioStatistic for RescaledRange {
    type Item = f64;

    fn name(&self) -> String {
        "Rescaled Range (simple, population)".to_string()
    }

    fn definition(&self) -> MetricDefinition {
        MetricDefinition::new(
            "rescaled_range",
            "Rescaled Range ({compounding}, {divisor})",
            MetricUnits::Ratio,
            MetricDirection::Informational,
            [MetricInput::Returns],
        )
        // The series reaches the estimator as simple per-period returns and is summed unlogged;
        // the logarithms are taken inside the estimator, on its own ratios and window lengths,
        // so the basis of the series itself is simple.
        .with_parameter("compounding", "simple")
        .with_parameter("divisor", "population")
        .with_tags([MetricTag::Returns, MetricTag::Distribution])
    }

    fn calculate_from_returns(&self, raw_returns: &Returns) -> Option<Self::Item> {
        if !self.check_valid_returns(raw_returns) {
            return None;
        }

        let returns = self.downsample_to_daily_bins(raw_returns);
        let values: Vec<f64> = returns.values().copied().collect();
        let n = values.len();
        if n < 16 {
            return None;
        }

        let max_window = largest_power_of_two_at_most(n / 2);
        let mut ladder: Vec<(f64, f64)> = Vec::new();
        let mut window = 4;
        while window <= max_window {
            let mut ratios: Vec<f64> = Vec::new();
            for block_values in values.chunks_exact(window) {
                let block_mean = block_values.iter().sum::<f64>() / window as f64;
                let variance = block_values
                    .iter()
                    .map(|x| (x - block_mean).powi(2))
                    .sum::<f64>()
                    / window as f64;
                let std = variance.sqrt();
                if std == 0.0 || !std.is_finite() {
                    continue;
                }

                let mut cumulative = 0.0;
                let mut max = 0.0_f64;
                let mut min = 0.0_f64;
                for x in block_values {
                    cumulative += x - block_mean;
                    max = max.max(cumulative);
                    min = min.min(cumulative);
                }

                let ratio = (max - min) / std;
                if ratio > 0.0 && ratio.is_finite() {
                    ratios.push(ratio);
                }
            }

            if !ratios.is_empty() {
                let mean_ratio = ratios.iter().sum::<f64>() / ratios.len() as f64;
                if mean_ratio > 0.0 && mean_ratio.is_finite() {
                    ladder.push(((window as f64).ln(), mean_ratio.ln()));
                }
            }

            window *= 2;
        }

        if ladder.len() < 3 {
            return None;
        }

        let slope = least_squares_slope(&ladder);
        if !slope.is_finite() {
            return None;
        }

        Some(slope)
    }

    fn calculate_from_realized_pnls(&self, _realized_pnls: &[f64]) -> Option<Self::Item> {
        None
    }

    fn calculate_from_positions(&self, _positions: &[Position]) -> Option<Self::Item> {
        None
    }
}

/// Returns the largest power of two that does not exceed `value`.
fn largest_power_of_two_at_most(value: usize) -> usize {
    let mut power: usize = 1;
    while power.saturating_mul(2) <= value {
        power *= 2;
    }
    power
}

/// Returns the least-squares slope of `y` against `x` over the given points.
fn least_squares_slope(points: &[(f64, f64)]) -> f64 {
    let count = points.len() as f64;
    let mean_x = points.iter().map(|p| p.0).sum::<f64>() / count;
    let mean_y = points.iter().map(|p| p.1).sum::<f64>() / count;
    let covariance = points
        .iter()
        .map(|p| (p.0 - mean_x) * (p.1 - mean_y))
        .sum::<f64>();
    let variance = points.iter().map(|p| (p.0 - mean_x).powi(2)).sum::<f64>();

    covariance / variance
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use nautilus_core::UnixNanos;
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
    fn test_a_linear_ramp_has_a_slope_near_one() {
        // A deterministic ramp has R/S growing proportionally to the window size, so the
        // log-log slope sits at one; the assertion band absorbs the finite-sample drift.
        let ramp: Vec<f64> = (1..=64).map(f64::from).collect();
        let statistic = RescaledRange::new();
        let slope = statistic
            .calculate_from_returns(&create_returns(&ramp))
            .unwrap();
        assert!(
            (0.9..=1.1).contains(&slope),
            "expected a slope near one, got {slope}"
        );
    }

    #[rstest]
    fn test_a_constant_series_is_undefined() {
        let statistic = RescaledRange::new();
        let result = statistic.calculate_from_returns(&create_returns(&[5.0; 64]));
        assert_eq!(result, None);
    }

    #[rstest]
    fn test_a_series_shorter_than_sixteen_is_undefined() {
        let statistic = RescaledRange::new();
        let values: Vec<f64> = (0..10).map(f64::from).collect();
        let result = statistic.calculate_from_returns(&create_returns(&values));
        assert_eq!(result, None);
    }

    #[rstest]
    fn test_an_alternating_series_is_anti_persistent() {
        // A strictly alternating (+1, -1) series has a bounded range and unit dispersion at
        // every window, so its rescaled range is flat and the slope collapses to zero: the
        // tool separates anti-persistent from persistent structure.
        let alternating: Vec<f64> = (0..64)
            .map(|i| if i % 2 == 0 { 1.0 } else { -1.0 })
            .collect();
        let statistic = RescaledRange::new();
        let slope = statistic
            .calculate_from_returns(&create_returns(&alternating))
            .unwrap();
        assert!(
            slope < 0.6,
            "expected an anti-persistent slope, got {slope}"
        );
    }

    #[rstest]
    fn test_metric_id_is_stable() {
        let statistic = RescaledRange::new();
        assert_eq!(statistic.name(), "Rescaled Range (simple, population)");
        assert_eq!(statistic.definition().id(), "rescaled_range");
    }
}
