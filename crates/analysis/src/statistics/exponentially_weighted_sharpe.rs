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

use nautilus_model::position::Position;

use crate::{
    Returns,
    metric::{MetricDefinition, MetricDirection, MetricInput, MetricTag, MetricUnits},
    period::PerformancePeriod,
    statistic::PortfolioStatistic,
};

/// The default annualisation period, in observations.
const DEFAULT_ANNUALISATION: usize = 252;

/// The default halflife, in observations.
const DEFAULT_HALFLIFE: usize = 6;

/// Calculates an exponentially weighted Sharpe ratio for portfolio returns.
///
/// The ratio is the exponentially weighted mean return divided by the exponentially weighted
/// standard deviation, annualised by the square root of the annualisation period:
/// `ew_mean / ew_std * sqrt(annualisation)`.
///
/// The weights decay by a factor of `0.5` every `halflife` observations, with the most recent
/// observation carrying weight one. The weighted standard deviation uses the weighted population
/// divisor (the sum of the weights), and the returns are first compounded to daily bins exactly
/// as the arithmetic Sharpe ratio does.
///
/// A series with no dispersion, or an empty series, has no defined ratio and returns `None`.
/// Non-finite inputs propagate to a non-finite result rather than being dropped.
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
pub struct ExponentiallyWeightedSharpe {
    /// The annualisation period (default: 252 for daily data).
    annualisation: usize,
    /// The weight halflife in observations (default: 6, minimum: 1).
    halflife: usize,
}

impl ExponentiallyWeightedSharpe {
    /// Creates a new [`ExponentiallyWeightedSharpe`] instance.
    #[must_use]
    pub fn new(annualisation: Option<usize>, halflife: Option<usize>) -> Self {
        Self {
            annualisation: annualisation.unwrap_or(DEFAULT_ANNUALISATION),
            halflife: halflife.unwrap_or(DEFAULT_HALFLIFE).max(1),
        }
    }
}

impl std::fmt::Display for ExponentiallyWeightedSharpe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Exponentially Weighted Sharpe (simple, population, {} days, halflife {})",
            self.annualisation, self.halflife
        )
    }
}

impl PortfolioStatistic for ExponentiallyWeightedSharpe {
    type Item = f64;

    fn name(&self) -> String {
        self.to_string()
    }

    fn definition(&self) -> MetricDefinition {
        MetricDefinition::new(
            "exponentially_weighted_sharpe",
            "Exponentially Weighted Sharpe ({compounding}, {divisor}, {annualisation} days, halflife {halflife})",
            MetricUnits::Ratio,
            MetricDirection::Maximize,
            [MetricInput::Returns],
        )
        .with_parameter("compounding", "simple")
        .with_parameter("divisor", "population")
        .with_count("annualisation", self.annualisation)
        .with_count("halflife", self.halflife)
        .with_tags([MetricTag::RiskAdjusted, MetricTag::Annualised])
    }

    fn calculate_from_returns(&self, raw_returns: &Returns) -> Option<Self::Item> {
        if !self.check_valid_returns(raw_returns) {
            return None;
        }

        let returns = self.downsample_to_daily_bins(raw_returns);
        let values: Vec<f64> = returns.values().copied().collect();
        let decay = 0.5_f64.powf(1.0 / self.halflife as f64);

        let mut weight_sum = 0.0;
        let mut weighted_sum = 0.0;
        let mut weight = 1.0;
        for value in values.iter().rev() {
            weighted_sum = value.mul_add(weight, weighted_sum);
            weight_sum += weight;
            weight *= decay;
        }
        let mean = weighted_sum / weight_sum;

        let mut weighted_variance = 0.0;
        let mut weight = 1.0;
        for value in values.iter().rev() {
            let deviation = value - mean;
            weighted_variance += weight * deviation * deviation;
            weight *= decay;
        }
        let variance = weighted_variance / weight_sum;

        if !variance.is_finite() || variance <= 0.0 {
            return Some(f64::NAN);
        }

        let std = variance.sqrt();
        if !std.is_finite() || std < f64::EPSILON {
            return Some(f64::NAN);
        }

        Some((mean / std) * (self.annualisation as f64).sqrt())
    }

    fn calculate_from_realized_pnls(&self, _realized_pnls: &[f64]) -> Option<Self::Item> {
        None
    }

    fn calculate_from_positions(&self, _positions: &[Position]) -> Option<Self::Item> {
        None
    }

    fn calculate_from_periods(&self, _periods: &[PerformancePeriod]) -> Option<Self::Item> {
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
    fn test_empty_returns_is_undefined() {
        let statistic = ExponentiallyWeightedSharpe::new(None, None);
        assert_eq!(statistic.calculate_from_returns(&create_returns(&[])), None);
    }

    #[rstest]
    fn test_zero_dispersion_is_undefined() {
        let statistic = ExponentiallyWeightedSharpe::new(None, None);
        let value = statistic
            .calculate_from_returns(&create_returns(&[0.01; 10]))
            .unwrap();
        assert!(value.is_nan());
    }

    #[rstest]
    fn test_name_reports_parameters() {
        let statistic = ExponentiallyWeightedSharpe::new(Some(252), Some(6));
        assert_eq!(
            statistic.name(),
            "Exponentially Weighted Sharpe (simple, population, 252 days, halflife 6)"
        );
    }

    #[rstest]
    fn test_two_observations_halflife_one_matches_hand_computation() {
        // With halflife 1 the decay is exactly 0.5, so the older observation carries weight 0.5
        // and the newer weight 1.0. For returns [0.01, 0.03] the weighted mean is
        // (0.5*0.01 + 0.03) / 1.5 and the weighted population variance follows from it.
        let statistic = ExponentiallyWeightedSharpe::new(Some(252), Some(1));
        let ratio = statistic
            .calculate_from_returns(&create_returns(&[0.01, 0.03]))
            .unwrap();

        let mean = (0.5_f64 * 0.01 + 0.03) / 1.5;
        let variance = (0.5 * (0.01 - mean).powi(2) + (0.03 - mean).powi(2)) / 1.5;
        let expected = mean / variance.sqrt() * 252.0_f64.sqrt();

        assert!(approx_eq!(f64, ratio, expected, epsilon = 1e-12));
    }
}
