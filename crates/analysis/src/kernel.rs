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

//! The numerical-stability classification of the research kernels.
//!
//! A research kernel is a function that reduces, transforms or estimates over returns, PnLs or
//! positions. The standard this crate holds them to is not parity with another implementation:
//! parity tests, mathematical property tests and adversarial numerical tests are three different
//! things, and two implementations can agree and both be wrong.
//!
//! "Every reducing kernel must pass the full adversarial suite" would turn a good test category
//! into an unbounded audit, so the obligation is **classified by kernel type**. Each kernel is
//! assigned exactly one [`KernelClass`], and the class determines the cases the kernel must meet.
//! A trivial element-wise kernel carries a trivial obligation and says so; a dispersion estimator
//! carries the heavy one.
//!
//! The classification is part of the work rather than an escape from it: a kernel that is not in
//! [`RESEARCH_KERNELS`] is an incomplete obligation, not an exempt one, and a test asserts that
//! every built-in statistic is classified rather than trusting the table to be maintained.
//!
//! # Scope
//!
//! The table covers the research surface of this crate: the built-in portfolio statistics and the
//! shared kernels they are composed from. The indicator and strategy kernels belong to the trading
//! layer, which has its own parity harness, and the research layer of the Python package
//! (`nautilus_trader.optimization`) carries its own kernel obligations beside its tests.
//!
//! # The classes
//!
//! | Class | Obligation |
//! | --- | --- |
//! | [`Reduction`](KernelClass::Reduction) | Empty input, single element, missing-value propagation, layout |
//! | [`RollingReduction`](KernelClass::RollingReduction) | The reduction cases, plus running-variance stability over a long series and a minimum period greater than the length |
//! | [`Cumulative`](KernelClass::Cumulative) | The reduction cases, plus overflow, underflow and catastrophic cancellation |
//! | [`Normalization`](KernelClass::Normalization) | The reduction cases, plus invariance under scaling, and a zero-denominator case |
//! | [`StatisticalEstimator`](KernelClass::StatisticalEstimator) | The reduction cases, plus a documented divisor convention, a minimum-observation case and an independent recomputation |
//! | [`Transform`](KernelClass::Transform) | Empty input, single element, missing-value edges and layout only |
//! | [`Label`](KernelClass::Label) | The transform cases, plus the wait convention and a leakage boundary case |

/// The class of a numerical kernel, which determines the stability obligation it carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum KernelClass {
    /// A sum, mean, minimum, maximum or count over a series.
    Reduction,
    /// A windowed or expanding reduction over a series.
    RollingReduction,
    /// A value accumulated along a series, such as compounding or a running maximum.
    Cumulative,
    /// A value rescaled by a level or a count, such as a ratio or a proportion.
    Normalization,
    /// A distribution parameter estimated from a series, such as a dispersion or a regression slope.
    StatisticalEstimator,
    /// A series mapped to a series, such as a resampling or an alignment.
    Transform,
    /// A target or feature series derived from a forward window.
    Label,
}

impl KernelClass {
    /// Every class, in obligation order.
    pub const ALL: [Self; 7] = [
        Self::Reduction,
        Self::RollingReduction,
        Self::Cumulative,
        Self::Normalization,
        Self::StatisticalEstimator,
        Self::Transform,
        Self::Label,
    ];

    /// Returns the class's stable name.
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Reduction => "reduction",
            Self::RollingReduction => "rolling_reduction",
            Self::Cumulative => "cumulative",
            Self::Normalization => "normalization",
            Self::StatisticalEstimator => "statistical_estimator",
            Self::Transform => "transform",
            Self::Label => "label",
        }
    }

    /// Returns the cases a kernel of this class must meet.
    #[must_use]
    pub const fn obligation(&self) -> &'static [&'static str] {
        match self {
            Self::Reduction => &[
                "empty input",
                "single element",
                "missing-value propagation",
                "layout independence",
            ],
            Self::RollingReduction => &[
                "empty input",
                "single element",
                "missing-value propagation",
                "layout independence",
                "running-variance stability over a long series",
                "minimum period greater than the length",
            ],
            Self::Cumulative => &[
                "empty input",
                "single element",
                "missing-value propagation",
                "layout independence",
                "overflow and underflow",
                "catastrophic cancellation",
            ],
            Self::Normalization => &[
                "empty input",
                "single element",
                "missing-value propagation",
                "layout independence",
                "invariance under scaling",
                "zero denominator",
            ],
            Self::StatisticalEstimator => &[
                "empty input",
                "single element",
                "missing-value propagation",
                "layout independence",
                "a documented divisor convention",
                "a minimum-observation case",
                "an independent recomputation",
            ],
            Self::Transform => &[
                "empty input",
                "single element",
                "missing-value edges",
                "layout",
            ],
            Self::Label => &[
                "empty input",
                "single element",
                "missing-value edges",
                "layout",
                "the wait convention",
                "a leakage boundary",
            ],
        }
    }
}

/// One research kernel and the class whose obligation it must meet.
#[derive(Clone, Copy, Debug)]
pub struct KernelClassification {
    /// The kernel's stable identity: a metric definition id, or a shared kernel's name.
    pub identity: &'static str,
    /// The class whose obligation the kernel must meet.
    pub class: KernelClass,
    /// Where the kernel is implemented, so the classification is auditable.
    pub source: &'static str,
}

/// The classification of every research kernel in this crate.
///
/// The order is by class, then by identity. `identity` is the statistic's own
/// `MetricDefinition::id` where it has one, so the table cannot drift from the statistics whose
/// definitions a test compares it against.
pub const RESEARCH_KERNELS: &[KernelClassification] = &[
    // Reductions: an aggregate with no denominator.
    KernelClassification {
        identity: "returns_average",
        class: KernelClass::Reduction,
        source: "statistics/returns_avg.rs",
    },
    KernelClassification {
        identity: "returns_average_loss",
        class: KernelClass::Reduction,
        source: "statistics/returns_avg_loss.rs",
    },
    KernelClassification {
        identity: "returns_average_win",
        class: KernelClass::Reduction,
        source: "statistics/returns_avg_win.rs",
    },
    KernelClassification {
        identity: "avg_loser",
        class: KernelClass::Reduction,
        source: "statistics/loser_avg.rs",
    },
    KernelClassification {
        identity: "max_loser",
        class: KernelClass::Reduction,
        source: "statistics/loser_max.rs",
    },
    KernelClassification {
        identity: "min_loser",
        class: KernelClass::Reduction,
        source: "statistics/loser_min.rs",
    },
    KernelClassification {
        identity: "avg_winner",
        class: KernelClass::Reduction,
        source: "statistics/winner_avg.rs",
    },
    KernelClassification {
        identity: "max_winner",
        class: KernelClass::Reduction,
        source: "statistics/winner_max.rs",
    },
    KernelClassification {
        identity: "min_winner",
        class: KernelClass::Reduction,
        source: "statistics/winner_min.rs",
    },
    KernelClassification {
        identity: "expectancy",
        class: KernelClass::Reduction,
        source: "statistics/expectancy.rs",
    },
    KernelClassification {
        identity: "total_commissions",
        class: KernelClass::Reduction,
        source: "statistics/total_commissions.rs",
    },
    KernelClassification {
        identity: "total_turnover",
        class: KernelClass::Reduction,
        source: "statistics/total_turnover.rs",
    },
    // Cumulative: a value accumulated along the series.
    KernelClassification {
        identity: "cagr",
        class: KernelClass::Cumulative,
        source: "statistics/cagr.rs",
    },
    KernelClassification {
        identity: "max_drawdown",
        class: KernelClass::Cumulative,
        source: "statistics/max_drawdown.rs",
    },
    KernelClassification {
        identity: "max_drawdown_duration",
        class: KernelClass::Cumulative,
        source: "statistics/max_drawdown_duration.rs",
    },
    KernelClassification {
        identity: "ulcer_index",
        class: KernelClass::Cumulative,
        source: "statistics/ulcer_index.rs",
    },
    KernelClassification {
        identity: "downsample_to_daily_bins",
        class: KernelClass::Cumulative,
        source: "statistic.rs",
    },
    // Normalizations: a value rescaled by a level, a level's change or a count.
    KernelClassification {
        identity: "calmar_ratio",
        class: KernelClass::Normalization,
        source: "statistics/calmar_ratio.rs",
    },
    KernelClassification {
        identity: "down_capture_ratio",
        class: KernelClass::Normalization,
        source: "statistics/down_capture_ratio.rs",
    },
    KernelClassification {
        identity: "information_ratio",
        class: KernelClass::Normalization,
        source: "statistics/information_ratio.rs",
    },
    KernelClassification {
        identity: "long_ratio",
        class: KernelClass::Normalization,
        source: "statistics/long_ratio.rs",
    },
    KernelClassification {
        identity: "omega_ratio",
        class: KernelClass::Normalization,
        source: "statistics/omega_ratio.rs",
    },
    KernelClassification {
        identity: "profit_factor",
        class: KernelClass::Normalization,
        source: "statistics/profit_factor.rs",
    },
    KernelClassification {
        identity: "risk_return_ratio",
        class: KernelClass::Normalization,
        source: "statistics/risk_return_ratio.rs",
    },
    KernelClassification {
        identity: "exponentially_weighted_sharpe",
        class: KernelClass::Normalization,
        source: "statistics/exponentially_weighted_sharpe.rs",
    },
    KernelClassification {
        identity: "sharpe_ratio",
        class: KernelClass::Normalization,
        source: "statistics/sharpe_ratio.rs",
    },
    KernelClassification {
        identity: "sortino_ratio",
        class: KernelClass::Normalization,
        source: "statistics/sortino_ratio.rs",
    },
    KernelClassification {
        identity: "tail_ratio",
        class: KernelClass::Normalization,
        source: "statistics/tail_ratio.rs",
    },
    KernelClassification {
        identity: "treynor_ratio",
        class: KernelClass::Normalization,
        source: "statistics/treynor_ratio.rs",
    },
    KernelClassification {
        identity: "up_capture_ratio",
        class: KernelClass::Normalization,
        source: "statistics/up_capture_ratio.rs",
    },
    KernelClassification {
        identity: "win_rate",
        class: KernelClass::Normalization,
        source: "statistics/win_rate.rs",
    },
    KernelClassification {
        identity: "breakeven_cost",
        class: KernelClass::Normalization,
        source: "statistics/breakeven_cost.rs",
    },
    KernelClassification {
        identity: "cost_basis_points",
        class: KernelClass::Normalization,
        source: "statistics/cost_basis_points.rs",
    },
    KernelClassification {
        identity: "gross_return",
        class: KernelClass::Normalization,
        source: "statistics/gross_return.rs",
    },
    KernelClassification {
        identity: "net_return",
        class: KernelClass::Normalization,
        source: "statistics/net_return.rs",
    },
    // Statistical estimators: a distribution parameter estimated from the series.
    KernelClassification {
        identity: "alpha",
        class: KernelClass::StatisticalEstimator,
        source: "statistics/alpha.rs",
    },
    KernelClassification {
        identity: "beta",
        class: KernelClass::StatisticalEstimator,
        source: "statistics/beta_ratio.rs",
    },
    KernelClassification {
        identity: "expected_shortfall",
        class: KernelClass::StatisticalEstimator,
        source: "statistics/expected_shortfall.rs",
    },
    KernelClassification {
        identity: "returns_kurtosis",
        class: KernelClass::StatisticalEstimator,
        source: "statistics/returns_kurtosis.rs",
    },
    KernelClassification {
        identity: "returns_skewness",
        class: KernelClass::StatisticalEstimator,
        source: "statistics/returns_skewness.rs",
    },
    KernelClassification {
        identity: "returns_volatility",
        class: KernelClass::StatisticalEstimator,
        source: "statistics/returns_volatility.rs",
    },
    KernelClassification {
        identity: "tracking_error",
        class: KernelClass::StatisticalEstimator,
        source: "statistics/tracking_error.rs",
    },
    KernelClassification {
        identity: "value_at_risk",
        class: KernelClass::StatisticalEstimator,
        source: "statistics/value_at_risk.rs",
    },
    KernelClassification {
        identity: "calculate_std",
        class: KernelClass::StatisticalEstimator,
        source: "statistic.rs",
    },
    // Transforms: a series mapped to a series.
    KernelClassification {
        identity: "align_returns",
        class: KernelClass::Transform,
        source: "statistic.rs",
    },
];

/// Returns the class of the kernel with the given identity, or `None` when it is unclassified.
#[must_use]
pub fn classify(identity: &str) -> Option<KernelClass> {
    RESEARCH_KERNELS
        .iter()
        .find(|kernel| kernel.identity == identity)
        .map(|kernel| kernel.class)
}

/// Returns the identities of every kernel in the given class.
#[must_use]
pub fn kernels_of_class(class: KernelClass) -> Vec<&'static str> {
    RESEARCH_KERNELS
        .iter()
        .filter(|kernel| kernel.class == class)
        .map(|kernel| kernel.identity)
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use nautilus_core::{UnixNanos, approx_eq};
    use rstest::rstest;

    use super::*;
    use crate::{
        Returns,
        analyzer::Statistic,
        metric::MetricInput,
        objective::builtin_statistics,
        statistic::PortfolioStatistic,
        statistics::{
            max_drawdown::MaxDrawdown, returns_volatility::ReturnsVolatility,
            ulcer_index::UlcerIndex,
        },
    };

    const BASE_NS: u64 = 1_600_000_000_000_000_000;
    const NANOS_PER_DAY: u64 = 86_400_000_000_000;

    /// Returns the given values at the given day offsets from the base timestamp.
    fn days_at(offsets: &[u64], values: &[f64]) -> Returns {
        offsets
            .iter()
            .zip(values)
            .map(|(&offset, &value)| (UnixNanos::from(BASE_NS + offset * NANOS_PER_DAY), value))
            .collect::<BTreeMap<_, _>>()
    }

    /// Returns the given values one day apart, starting from the base timestamp.
    fn days(values: &[f64]) -> Returns {
        let offsets: Vec<u64> = (0..values.len() as u64).collect();
        days_at(&offsets, values)
    }

    /// Returns the identity of a statistic.
    fn identity(statistic: &Statistic) -> String {
        statistic.definition().id().to_string()
    }

    /// Returns a message when a missing observation is neither propagated nor excluded.
    ///
    /// The rule is the kernel-level form of the reporting rule that a missing return is excluded
    /// rather than zero-filled: a kernel may report a non-finite value when an observation is
    /// missing, or it may ignore the observation entirely, but it may not keep it in a count or in
    /// a rank and treat it as data. The third outcome is a silent reinterpretation, and comparing
    /// against the same series without the missing observation is what detects it.
    fn missing_violation(
        with_missing: Option<f64>,
        without_missing: Option<f64>,
    ) -> Option<String> {
        let value = match with_missing {
            None => return None,
            Some(value) if !value.is_finite() => return None,
            Some(value) => value,
        };

        match without_missing {
            None => Some(format!(
                "produced {value} where the series without the missing observation produced nothing"
            )),
            Some(reference) if value == reference => None,
            Some(reference) => Some(format!(
                "produced {value}, which is neither propagated nor the value {reference} of the series without the missing observation"
            )),
        }
    }

    #[rstest]
    fn test_every_builtin_statistic_is_classified() {
        let statistics = builtin_statistics();
        let classified = RESEARCH_KERNELS
            .iter()
            .filter(|kernel| kernel.source.starts_with("statistics/"))
            .count();

        for statistic in &statistics {
            let definition = statistic.definition();
            let id = definition.id();
            assert!(classify(id).is_some(), "{id} is not classified");
        }

        assert_eq!(
            classified,
            statistics.len(),
            "the table must classify every built-in statistic and no other statistic"
        );

        for kernel in RESEARCH_KERNELS {
            if !kernel.source.starts_with("statistics/") {
                continue;
            }
            let exists = statistics
                .iter()
                .any(|statistic| statistic.definition().id() == kernel.identity);
            assert!(
                exists,
                "{} is classified but is not a built-in statistic",
                kernel.identity
            );
        }
    }

    #[rstest]
    fn test_every_class_states_its_obligation() {
        let names: Vec<&str> = KernelClass::ALL.iter().map(KernelClass::as_str).collect();
        assert_eq!(names.len(), 7);

        let mut unique = names.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), names.len(), "class names must be unique");

        for class in KernelClass::ALL {
            assert!(
                !class.obligation().is_empty(),
                "{} states no obligation",
                class.as_str()
            );
        }

        assert!(!kernels_of_class(KernelClass::Reduction).is_empty());
        assert!(!kernels_of_class(KernelClass::Cumulative).is_empty());
        assert!(!kernels_of_class(KernelClass::Normalization).is_empty());
        assert!(!kernels_of_class(KernelClass::StatisticalEstimator).is_empty());
        assert!(!kernels_of_class(KernelClass::Transform).is_empty());

        // No member yet: these classes are declared so a kernel that joins them inherits an
        // obligation rather than an exemption, and nothing in this crate occupies them.
        assert!(kernels_of_class(KernelClass::RollingReduction).is_empty());
        assert!(kernels_of_class(KernelClass::Label).is_empty());

        assert_eq!(classify("not_a_kernel"), None);
    }

    #[rstest]
    fn test_missing_returns_are_propagated_or_excluded_for_every_returns_kernel() {
        let offsets = [0, 1, 2, 3];
        let with_missing = days_at(&offsets, &[0.01, f64::NAN, -0.5, 0.03]);
        let without_missing = days_at(&[0, 2, 3], &[0.01, -0.5, 0.03]);

        let mut violations = Vec::new();

        for statistic in builtin_statistics() {
            if !statistic
                .definition()
                .inputs()
                .contains(&MetricInput::Returns)
            {
                continue;
            }
            let id = identity(&statistic);
            let violation = missing_violation(
                statistic.calculate_from_returns(&with_missing),
                statistic.calculate_from_returns(&without_missing),
            );
            if let Some(message) = violation {
                violations.push(format!("{id}: {message}"));
            }
        }

        assert!(
            violations.is_empty(),
            "missing returns: {}",
            violations.join("; ")
        );
    }

    #[rstest]
    fn test_missing_pnls_are_propagated_or_excluded_for_every_pnl_kernel() {
        let with_missing = vec![1.0, f64::NAN, -2.0];
        let without_missing = vec![1.0, -2.0];

        let mut violations = Vec::new();

        for statistic in builtin_statistics() {
            if !statistic
                .definition()
                .inputs()
                .contains(&MetricInput::RealizedPnls)
            {
                continue;
            }
            let id = identity(&statistic);
            let violation = missing_violation(
                statistic.calculate_from_realized_pnls(&with_missing),
                statistic.calculate_from_realized_pnls(&without_missing),
            );
            if let Some(message) = violation {
                violations.push(format!("{id}: {message}"));
            }
        }

        assert!(
            violations.is_empty(),
            "missing PnLs: {}",
            violations.join("; ")
        );
    }

    #[rstest]
    fn test_missing_benchmark_returns_are_propagated_or_excluded_for_every_benchmark_kernel() {
        let returns = days(&[0.02, -0.01, 0.04, -0.03]);
        let offsets = [0, 1, 2, 3];
        let with_missing = days_at(&offsets, &[0.01, f64::NAN, 0.03, -0.02]);
        let without_missing = days_at(&[0, 2, 3], &[0.01, 0.03, -0.02]);

        let mut violations = Vec::new();

        for statistic in builtin_statistics() {
            if !statistic
                .definition()
                .inputs()
                .contains(&MetricInput::Benchmark)
            {
                continue;
            }
            let id = identity(&statistic);
            let violation = missing_violation(
                statistic.calculate_from_returns_with_benchmark(&returns, &with_missing),
                statistic.calculate_from_returns_with_benchmark(&returns, &without_missing),
            );
            if let Some(message) = violation {
                violations.push(format!("{id}: {message}"));
            }
        }

        assert!(
            violations.is_empty(),
            "missing benchmark returns: {}",
            violations.join("; ")
        );
    }

    #[rstest]
    fn test_a_short_finite_series_never_produces_an_infinity() {
        let cases: Vec<(&str, Returns)> = vec![
            ("empty", BTreeMap::new()),
            ("one", days(&[0.01])),
            ("flat", days(&[0.01, 0.01, 0.01, 0.01])),
            ("zero", days(&[0.0, 0.0, 0.0])),
            ("no_losses", days(&[0.01, 0.02, 0.03, 0.04])),
            ("no_wins", days(&[-0.01, -0.02, -0.03, -0.04])),
            ("mixed", days(&[0.01, -0.02, 0.03, -0.04])),
        ];

        let mut violations = Vec::new();

        for statistic in builtin_statistics() {
            if !statistic
                .definition()
                .inputs()
                .contains(&MetricInput::Returns)
            {
                continue;
            }
            let id = identity(&statistic);
            for (label, returns) in &cases {
                if let Some(value) = statistic.calculate_from_returns(returns)
                    && value.is_infinite()
                {
                    violations.push(format!("{id} on {label}"));
                }
            }
            for (label, values) in [
                ("pnl_empty", vec![]),
                ("pnl_zero", vec![0.0, 0.0]),
                ("pnl_one", vec![1.0]),
            ] {
                if let Some(value) = statistic.calculate_from_realized_pnls(&values)
                    && value.is_infinite()
                {
                    violations.push(format!("{id} on {label}"));
                }
            }
        }

        assert!(
            violations.is_empty(),
            "a finite input produced an infinity: {}",
            violations.join("; ")
        );
    }

    /// Returns the given values one nanosecond apart, so every value shares one daily bin.
    fn nanoseconds(values: &[f64]) -> Returns {
        values
            .iter()
            .enumerate()
            .map(|(index, &value)| (UnixNanos::from(BASE_NS + index as u64), value))
            .collect::<BTreeMap<_, _>>()
    }

    /// Returns the relative difference between two values, treating equal specials as equal.
    fn relative_difference(left: f64, right: f64) -> f64 {
        if left == right {
            return 0.0;
        }
        (left - right).abs() / left.abs().max(right.abs()).max(f64::MIN_POSITIVE)
    }

    /// Returns the sum of the values with Neumaier compensation.
    fn compensated_sum(values: impl Iterator<Item = f64>) -> f64 {
        let mut sum = 0.0;
        let mut compensation = 0.0;

        for value in values {
            let tentative = sum + value;
            compensation += if sum.abs() >= value.abs() {
                (sum - tentative) + value
            } else {
                (value - tentative) + sum
            };
            sum = tentative;
        }

        sum + compensation
    }

    /// Returns the sample variance with a compensated two-pass computation.
    fn compensated_variance(values: &[f64]) -> f64 {
        let count = values.len() as f64;
        let mean = compensated_sum(values.iter().copied()) / count;
        compensated_sum(values.iter().map(|value| (value - mean).powi(2))) / (count - 1.0)
    }

    /// Returns the sample variance with the naive two-pass computation.
    fn naive_variance(values: &[f64]) -> f64 {
        let count = values.len() as f64;
        let mean = values.iter().sum::<f64>() / count;
        values
            .iter()
            .map(|value| (value - mean).powi(2))
            .sum::<f64>()
            / (count - 1.0)
    }

    #[rstest]
    fn test_the_dispersion_kernel_pins_its_divisor_and_minimum_observation() {
        let statistic = ReturnsVolatility::new(None);

        // Bessel's correction: the sample divisor n - 1, not the population divisor n. The two
        // conventions differ by a factor of two on this sample, so the assertion is decisive.
        let two = nanoseconds(&[1.0, 2.0]);
        assert!(approx_eq!(
            f64,
            statistic.calculate_std(&two),
            0.5_f64.sqrt(),
            epsilon = 1e-15
        ));

        assert!(statistic.calculate_std(&nanoseconds(&[1.0])).is_nan());
        assert!(statistic.calculate_std(&BTreeMap::new()).is_nan());
    }

    #[rstest]
    fn test_the_dispersion_kernel_matches_a_compensated_computation_over_a_long_series() {
        let statistic = ReturnsVolatility::new(None);

        // A long series with a large offset relative to its spread: the sum of the values is many
        // orders of magnitude larger than the variance, which is where a naive summation loses the
        // precision a compensated one keeps.
        let values: Vec<f64> = (0..100_000)
            .map(|index| 1e9 + f64::from((index * 7919) % 1000) * 1e-6)
            .collect();
        let returns = nanoseconds(&values);

        let kernel = statistic.calculate_std(&returns).powi(2);
        let reference = compensated_variance(&values);
        let naive = naive_variance(&values);

        let kernel_error = relative_difference(kernel, reference);
        let naive_error = relative_difference(naive, reference);

        assert!(
            kernel_error <= 1e-9,
            "the variance kernel differs from the compensated computation by {kernel_error}, against {naive_error} for the naive form"
        );
        assert!(
            naive_error > 1e-6,
            "the case is not adversarial: the naive form differs by only {naive_error}"
        );
    }

    #[rstest]
    fn test_cumulative_kernels_agree_with_an_independent_recomputation_over_a_long_series() {
        let values: Vec<f64> = (0..100_000)
            .map(|index| if index % 3 == 0 { 0.004 } else { -0.003 })
            .collect();
        let returns = nanoseconds(&values);

        let drawdown = MaxDrawdown::new()
            .calculate_from_returns(&returns)
            .expect("the drawdown is computed for a non-empty series");
        let ulcer = UlcerIndex::new()
            .calculate_from_returns(&returns)
            .expect("the ulcer index is computed for a non-empty series");

        assert!(drawdown.is_finite());
        assert!(ulcer.is_finite());

        // An independent recomputation of the drawdown in log space, which accumulates the same
        // equity path by a different arithmetic route rather than by the kernel's own twin.
        let mut log_level = 0.0_f64;
        let mut log_peak = 0.0_f64;
        let mut worst = 0.0_f64;

        for value in &values {
            log_level += (1.0 + value).ln();
            log_peak = log_peak.max(log_level);
            worst = worst.max(1.0 - (log_level - log_peak).exp());
        }

        assert!(
            relative_difference(drawdown, -worst) <= 1e-9,
            "the kernel reported {drawdown}, the log-space recomputation {worst}"
        );
    }

    #[rstest]
    fn test_transform_kernels_keep_the_layout() {
        let statistic = ReturnsVolatility::new(None);

        assert!(
            statistic
                .downsample_to_daily_bins(&BTreeMap::new())
                .is_empty()
        );

        let one = statistic.downsample_to_daily_bins(&nanoseconds(&[0.01]));
        assert_eq!(one.len(), 1);

        // Three returns within one second compound into one daily bin, at the floor of their day.
        let intraday = statistic.downsample_to_daily_bins(&nanoseconds(&[0.01, 0.02, -0.03]));
        assert_eq!(intraday.len(), 1);
        let (timestamp, value) = intraday.iter().next().expect("one bin");
        // The bin is keyed by the floor of the timestamp to the UTC day.
        assert_eq!(
            *timestamp,
            UnixNanos::from(BASE_NS / NANOS_PER_DAY * NANOS_PER_DAY)
        );
        assert!(approx_eq!(
            f64,
            *value,
            1.01 * 1.02 * 0.97 - 1.0,
            epsilon = 1e-15
        ));

        // The alignment is an inner join on the shared timestamps: a timestamp present in one series
        // only is dropped rather than zero-filled, and the pairs stay in ascending order.
        let (strategy, benchmark) = statistic.align_returns(
            &days_at(&[0, 1, 2], &[0.01, 0.02, 0.03]),
            &days_at(&[1, 2, 3], &[0.04, 0.05, 0.06]),
        );

        assert_eq!(strategy.len(), 2);
        assert!(approx_eq!(f64, strategy[0], 0.02, epsilon = 1e-15));
        assert!(approx_eq!(f64, strategy[1], 0.03, epsilon = 1e-15));
        assert!(approx_eq!(f64, benchmark[0], 0.04, epsilon = 1e-15));
        assert!(approx_eq!(f64, benchmark[1], 0.05, epsilon = 1e-15));

        let (empty_strategy, empty_benchmark) =
            statistic.align_returns(&BTreeMap::new(), &days_at(&[0], &[0.01]));

        assert!(empty_strategy.is_empty());
        assert!(empty_benchmark.is_empty());
    }

    #[rstest]
    fn test_the_layout_does_not_change_a_reduction() {
        let forward = vec![100.0, -50.0, 0.0, -20.0, 30.0];
        let reversed: Vec<f64> = forward.iter().rev().copied().collect();

        let mut violations = Vec::new();

        for statistic in builtin_statistics() {
            let class = classify(statistic.definition().id());
            if !matches!(
                class,
                Some(KernelClass::Reduction | KernelClass::Normalization)
            ) || !statistic
                .definition()
                .inputs()
                .contains(&MetricInput::RealizedPnls)
            {
                continue;
            }

            let id = identity(&statistic);
            let ordered = statistic.calculate_from_realized_pnls(&forward);
            let shuffled = statistic.calculate_from_realized_pnls(&reversed);
            let same = match (ordered, shuffled) {
                (None, None) => true,
                (Some(left), Some(right)) => {
                    (left.is_nan() && right.is_nan()) || relative_difference(left, right) <= 1e-12
                }
                _ => false,
            };
            if !same {
                violations.push(format!("{id}: {ordered:?} against {shuffled:?}"));
            }
        }

        assert!(
            violations.is_empty(),
            "the input order changed a reduction: {}",
            violations.join("; ")
        );
    }
}
