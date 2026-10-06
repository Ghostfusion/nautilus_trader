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

//! The arithmetic compounding construction checked against the frame's realised terminal equity.
//!
//! A report is often read through a single per-period number - the arithmetic mean of the frame's
//! net returns - and a reader multiplies that number out over the period count to picture the
//! terminal value. That construction is not the frame's own arithmetic: the arithmetic mean of a
//! series of returns is at or above the rate the frame actually compounded (the arithmetic mean of
//! the growth factors is at or above their geometric mean), so compounding it overstates the
//! terminal equity whenever the per-period returns are not all equal. The gap is exactly the
//! volatility drag, and these statistics report it rather than leave it implicit in the returns
//! bookkeeping.
//!
//! The pair of equity rows are the two terminals: the equity implied by compounding the arithmetic
//! mean over the frame's period count and applying it to the frame's starting equity, and the
//! equity the frame actually realised (the last period's closing equity). The ratio reports their
//! quotient, and the flag reports whether that quotient is further from one than a declared
//! `tolerance`. All four values read the one frame; the returns are the frame's own
//! `performance.net_return` values and the equity is the frame's own accounting, so the comparison
//! is against the frame's realised arithmetic and not against a second ledger.
//!
//! Every value is defined only when the whole frame resolves: the first period's starting equity
//! and the last period's ending equity each resolve to the same single currency, and every period
//! has a defined net return. An empty frame, a frame whose equity spans more than one currency,
//! and a frame with a period whose return is not defined all report no value, because a comparison
//! against a terminal equity that itself cannot be formed is not a number.
//!
//! Money stays exact while it is read ([`Money`](nautilus_model::types::Money) and
//! [`CurrencyTotals`](crate::period::CurrencyTotals)); only the mean and the ratio are `f64`.

use std::fmt::Display;

use nautilus_core::correctness::check_predicate_true;
use nautilus_model::position::Position;
use rust_decimal::prelude::ToPrimitive;

use crate::{
    Returns,
    metric::{MetricDefinition, MetricDirection, MetricInput, MetricTag, MetricUnits},
    period::PerformancePeriod,
    statistic::PortfolioStatistic,
};

/// The default absolute tolerance the flagged row compares the ratio against.
const DEFAULT_TOLERANCE: f64 = 0.01;

/// Returns the checked tolerance, defaulting to [`DEFAULT_TOLERANCE`] when none is given.
///
/// # Errors
///
/// Returns an error if `tolerance` is not finite or is negative; a threshold that is not a
/// non-negative finite number cannot separate a flagged ratio from an unflagged one.
fn checked_tolerance(tolerance: Option<f64>) -> anyhow::Result<f64> {
    let tolerance = tolerance.unwrap_or(DEFAULT_TOLERANCE);
    check_predicate_true(
        tolerance.is_finite() && tolerance >= 0.0,
        "tolerance must be finite and non-negative",
    )?;
    Ok(tolerance)
}

/// The frame's arithmetic-compounding comparison.
///
/// The two equity terminals are always present when the frame reduces; the ratio carries the
/// division only where the realised terminal is a non-zero amount, so a frame that ended flat has
/// a quotient that cannot be formed rather than an infinite one.
#[derive(Debug, Clone, Copy, PartialEq)]
struct FrameArithmetic {
    implied_equity: f64,
    realised_equity: f64,
    ratio: Option<f64>,
}

/// Reduces a performance-period frame to its arithmetic-compounding comparison.
///
/// Returns `None` for an empty frame, a frame whose starting or ending equity is not a single
/// currency, a frame whose starting and ending currencies differ, a frame with a period whose net
/// return is not defined, and a frame whose implied terminal equity is not finite. The implied
/// equity is the starting equity grown by the arithmetic mean of the frame's net returns,
/// compounded once per period; the realised equity is the last period's closing equity.
fn frame_arithmetic(periods: &[PerformancePeriod]) -> Option<FrameArithmetic> {
    let first = periods.first()?;
    let last = periods.last()?;
    let starting = first.accounting.starting_equity.as_single_currency()?;
    let realised = last.accounting.ending_equity.as_single_currency()?;

    if starting.currency != realised.currency {
        return None;
    }

    let mut sum = 0.0;
    for period in periods {
        sum += period.performance.net_return?;
    }

    let count = periods.len() as f64;
    let mean = sum / count;
    let starting_value = starting.as_decimal().to_f64()?;
    let realised_value = realised.as_decimal().to_f64()?;
    let implied_value = starting_value * (1.0 + mean).powf(count);

    if !implied_value.is_finite() {
        return None;
    }

    let ratio = if realised_value == 0.0 {
        None
    } else {
        Some(implied_value / realised_value)
    };

    Some(FrameArithmetic {
        implied_equity: implied_value,
        realised_equity: realised_value,
        ratio,
    })
}

/// Calculates the terminal equity implied by compounding the arithmetic mean net return.
///
/// The frame's per-period net returns are averaged, the average is compounded once per period,
/// and the result is applied to the first period's starting equity. It is the terminal value a
/// reader reaches by multiplying the arithmetic mean out over the frame's period count, and it is
/// read beside [`ArithmeticCompoundingRealisedEquity`] to see whether the construction overstates
/// or understates the frame's own arithmetic.
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
pub struct ArithmeticCompoundingImpliedEquity {
    /// The absolute tolerance the companion ratio and flag rows compare against.
    tolerance: f64,
}

impl ArithmeticCompoundingImpliedEquity {
    /// Creates a new checked [`ArithmeticCompoundingImpliedEquity`] instance.
    ///
    /// # Errors
    ///
    /// Returns an error if `tolerance` is not finite or is negative.
    pub fn new_checked(tolerance: Option<f64>) -> anyhow::Result<Self> {
        Ok(Self {
            tolerance: checked_tolerance(tolerance)?,
        })
    }

    /// Creates a new [`ArithmeticCompoundingImpliedEquity`] instance.
    ///
    /// # Panics
    ///
    /// Panics if `tolerance` is not finite or is negative.
    #[must_use]
    pub fn new(tolerance: Option<f64>) -> Self {
        Self::new_checked(tolerance)
            .expect("Invalid `tolerance` for `ArithmeticCompoundingImpliedEquity`")
    }
}

impl Display for ArithmeticCompoundingImpliedEquity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Arithmetic Compounding Implied Equity (simple, tolerance {})",
            self.tolerance
        )
    }
}

impl PortfolioStatistic for ArithmeticCompoundingImpliedEquity {
    type Item = f64;

    fn name(&self) -> String {
        self.to_string()
    }

    fn definition(&self) -> MetricDefinition {
        MetricDefinition::new(
            "arithmetic_compounding_implied_equity",
            "Arithmetic Compounding Implied Equity (simple, tolerance {tolerance})",
            MetricUnits::Currency,
            MetricDirection::Informational,
            [MetricInput::PerformancePeriods],
        )
        .with_number("tolerance", self.tolerance)
        .with_tags([MetricTag::Returns])
    }

    fn calculate_from_returns(&self, _returns: &Returns) -> Option<Self::Item> {
        None
    }

    fn calculate_from_realized_pnls(&self, _realized_pnls: &[f64]) -> Option<Self::Item> {
        None
    }

    fn calculate_from_positions(&self, _positions: &[Position]) -> Option<Self::Item> {
        None
    }

    fn calculate_from_periods(&self, periods: &[PerformancePeriod]) -> Option<Self::Item> {
        Some(frame_arithmetic(periods)?.implied_equity)
    }
}

/// Calculates the frame's realised terminal equity, the last period's closing equity.
///
/// This is the terminal value the frame's own arithmetic reached, read beside
/// [`ArithmeticCompoundingImpliedEquity`] as the reference the construction is checked against.
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
pub struct ArithmeticCompoundingRealisedEquity {}

impl ArithmeticCompoundingRealisedEquity {
    /// Creates a new [`ArithmeticCompoundingRealisedEquity`] instance.
    ///
    /// The realised equity is a fact about the frame: it does not compare against a tolerance, so
    /// this statistic takes none, and its title declares none.
    #[must_use]
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for ArithmeticCompoundingRealisedEquity {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for ArithmeticCompoundingRealisedEquity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Arithmetic Compounding Realised Equity (simple)")
    }
}

impl PortfolioStatistic for ArithmeticCompoundingRealisedEquity {
    type Item = f64;

    fn name(&self) -> String {
        self.to_string()
    }

    fn definition(&self) -> MetricDefinition {
        MetricDefinition::new(
            "arithmetic_compounding_realised_equity",
            "Arithmetic Compounding Realised Equity (simple)",
            MetricUnits::Currency,
            MetricDirection::Informational,
            [MetricInput::PerformancePeriods],
        )
        .with_tags([MetricTag::Returns])
    }

    fn calculate_from_returns(&self, _returns: &Returns) -> Option<Self::Item> {
        None
    }

    fn calculate_from_realized_pnls(&self, _realized_pnls: &[f64]) -> Option<Self::Item> {
        None
    }

    fn calculate_from_positions(&self, _positions: &[Position]) -> Option<Self::Item> {
        None
    }

    fn calculate_from_periods(&self, periods: &[PerformancePeriod]) -> Option<Self::Item> {
        Some(frame_arithmetic(periods)?.realised_equity)
    }
}

/// Calculates the ratio of the implied terminal equity to the realised terminal equity.
///
/// A ratio above one is the amount by which compounding the arithmetic mean net return overstates
/// the frame's terminal equity; a ratio below one is the amount by which it understates it. A ratio
/// of exactly one is the degenerate case where the two constructions agree, which happens only
/// when the frame's per-period returns are all equal (a single-period frame among them). A frame
/// that ended flat has no ratio, because the quotient cannot be formed.
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
pub struct ArithmeticCompoundingRatio {
    /// The absolute tolerance the companion flag row compares this ratio against.
    tolerance: f64,
}

impl ArithmeticCompoundingRatio {
    /// Creates a new checked [`ArithmeticCompoundingRatio`] instance.
    ///
    /// # Errors
    ///
    /// Returns an error if `tolerance` is not finite or is negative.
    pub fn new_checked(tolerance: Option<f64>) -> anyhow::Result<Self> {
        Ok(Self {
            tolerance: checked_tolerance(tolerance)?,
        })
    }

    /// Creates a new [`ArithmeticCompoundingRatio`] instance.
    ///
    /// # Panics
    ///
    /// Panics if `tolerance` is not finite or is negative.
    #[must_use]
    pub fn new(tolerance: Option<f64>) -> Self {
        Self::new_checked(tolerance).expect("Invalid `tolerance` for `ArithmeticCompoundingRatio`")
    }
}

impl Display for ArithmeticCompoundingRatio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Arithmetic Compounding Ratio (simple, tolerance {})",
            self.tolerance
        )
    }
}

impl PortfolioStatistic for ArithmeticCompoundingRatio {
    type Item = f64;

    fn name(&self) -> String {
        self.to_string()
    }

    fn definition(&self) -> MetricDefinition {
        MetricDefinition::new(
            "arithmetic_compounding_ratio",
            "Arithmetic Compounding Ratio (simple, tolerance {tolerance})",
            MetricUnits::Ratio,
            MetricDirection::Informational,
            [MetricInput::PerformancePeriods],
        )
        .with_number("tolerance", self.tolerance)
        .with_tags([MetricTag::Returns])
    }

    fn calculate_from_returns(&self, _returns: &Returns) -> Option<Self::Item> {
        None
    }

    fn calculate_from_realized_pnls(&self, _realized_pnls: &[f64]) -> Option<Self::Item> {
        None
    }

    fn calculate_from_positions(&self, _positions: &[Position]) -> Option<Self::Item> {
        None
    }

    fn calculate_from_periods(&self, periods: &[PerformancePeriod]) -> Option<Self::Item> {
        frame_arithmetic(periods)?.ratio
    }
}

/// Flags a frame whose arithmetic-compounding ratio is further from one than the tolerance.
///
/// The value is `1.0` when `|ratio - 1| > tolerance` and `0.0` otherwise, so a report consumer
/// reads a single discrete verdict: the arithmetic construction materially disagrees with the
/// frame's realised terminal equity. The tolerance is a declared parameter of the definition, so
/// the threshold the verdict was taken at prints with the row and a reader can disagree with it.
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
pub struct ArithmeticCompoundingFlagged {
    /// The absolute tolerance the ratio is compared against.
    tolerance: f64,
}

impl ArithmeticCompoundingFlagged {
    /// Creates a new checked [`ArithmeticCompoundingFlagged`] instance.
    ///
    /// # Errors
    ///
    /// Returns an error if `tolerance` is not finite or is negative.
    pub fn new_checked(tolerance: Option<f64>) -> anyhow::Result<Self> {
        Ok(Self {
            tolerance: checked_tolerance(tolerance)?,
        })
    }

    /// Creates a new [`ArithmeticCompoundingFlagged`] instance.
    ///
    /// # Panics
    ///
    /// Panics if `tolerance` is not finite or is negative.
    #[must_use]
    pub fn new(tolerance: Option<f64>) -> Self {
        Self::new_checked(tolerance)
            .expect("Invalid `tolerance` for `ArithmeticCompoundingFlagged`")
    }
}

impl Display for ArithmeticCompoundingFlagged {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Arithmetic Compounding Flagged (simple, tolerance {})",
            self.tolerance
        )
    }
}

impl PortfolioStatistic for ArithmeticCompoundingFlagged {
    type Item = f64;

    fn name(&self) -> String {
        self.to_string()
    }

    fn definition(&self) -> MetricDefinition {
        MetricDefinition::new(
            "arithmetic_compounding_flagged",
            "Arithmetic Compounding Flagged (simple, tolerance {tolerance})",
            MetricUnits::Ratio,
            MetricDirection::Minimize,
            [MetricInput::PerformancePeriods],
        )
        .with_number("tolerance", self.tolerance)
        .with_tags([MetricTag::Returns])
    }

    fn calculate_from_returns(&self, _returns: &Returns) -> Option<Self::Item> {
        None
    }

    fn calculate_from_realized_pnls(&self, _realized_pnls: &[f64]) -> Option<Self::Item> {
        None
    }

    fn calculate_from_positions(&self, _positions: &[Position]) -> Option<Self::Item> {
        None
    }

    fn calculate_from_periods(&self, periods: &[PerformancePeriod]) -> Option<Self::Item> {
        let ratio = frame_arithmetic(periods)?.ratio?;
        Some(if (ratio - 1.0).abs() > self.tolerance {
            1.0
        } else {
            0.0
        })
    }
}

#[cfg(test)]
mod tests {
    use nautilus_core::{UnixNanos, approx_eq};
    use rstest::rstest;

    use super::*;
    use crate::period::{
        CurrencyTotals, PeriodAccounting, PeriodActivity, PeriodExposure, PeriodPerformance,
    };
    use nautilus_model::types::{Currency, Money};
    use rust_decimal::Decimal;

    /// Returns a period with the given starting equity, ending equity and net return.
    fn period(
        starting_equity: &[(Currency, f64)],
        ending_equity: &[(Currency, f64)],
        net_return: Option<f64>,
    ) -> PerformancePeriod {
        fn totals(amounts: &[(Currency, f64)]) -> CurrencyTotals {
            let mut totals = CurrencyTotals::new();
            for &(currency, amount) in amounts {
                totals.add_money(Money::new(amount, currency));
            }
            totals
        }

        PerformancePeriod {
            accounting: PeriodAccounting {
                start: UnixNanos::default(),
                end: UnixNanos::default(),
                starting_equity: totals(starting_equity),
                ending_equity: totals(ending_equity),
                realized_pnl: CurrencyTotals::new(),
                unrealized_pnl: CurrencyTotals::new(),
                commission: CurrencyTotals::new(),
            },
            activity: PeriodActivity {
                volume: Decimal::ZERO,
                turnover: CurrencyTotals::new(),
                trade_count: 0,
                winning_trades: 0,
                losing_trades: 0,
            },
            exposure: PeriodExposure {
                open_positions: 0,
                gross_exposure: CurrencyTotals::new(),
                net_exposure: CurrencyTotals::new(),
            },
            performance: PeriodPerformance {
                net_pnl: CurrencyTotals::new(),
                net_return,
                drawdown: CurrencyTotals::new(),
                drawdown_percentage: None,
            },
        }
    }

    /// Returns every one of the four values over `frame`.
    fn calculate_all(frame: &[PerformancePeriod]) -> Vec<Option<f64>> {
        vec![
            ArithmeticCompoundingImpliedEquity::new(None).calculate_from_periods(frame),
            ArithmeticCompoundingRealisedEquity::new().calculate_from_periods(frame),
            ArithmeticCompoundingRatio::new(None).calculate_from_periods(frame),
            ArithmeticCompoundingFlagged::new(None).calculate_from_periods(frame),
        ]
    }

    /// Returns a two-period frame of `+50%` then `-50%` on a 1,000 USD equity.
    ///
    /// The arithmetic mean net return is zero, so compounding it leaves the terminal equity at
    /// its starting 1,000 USD while the frame actually ended at 750 USD: the construction
    /// overstates the terminal value.
    fn overstated_frame() -> Vec<PerformancePeriod> {
        vec![
            period(
                &[(Currency::USD(), 1000.0)],
                &[(Currency::USD(), 1500.0)],
                Some(0.5),
            ),
            period(
                &[(Currency::USD(), 1500.0)],
                &[(Currency::USD(), 750.0)],
                Some(-0.5),
            ),
        ]
    }

    #[rstest]
    fn test_overstated_frame_reports_the_ratio_the_construction_produces() {
        let frame = overstated_frame();

        let implied = ArithmeticCompoundingImpliedEquity::new(None)
            .calculate_from_periods(&frame)
            .unwrap();
        let realised = ArithmeticCompoundingRealisedEquity::new()
            .calculate_from_periods(&frame)
            .unwrap();
        let ratio = ArithmeticCompoundingRatio::new(None)
            .calculate_from_periods(&frame)
            .unwrap();
        let flagged = ArithmeticCompoundingFlagged::new(None)
            .calculate_from_periods(&frame)
            .unwrap();

        // The mean is exactly zero, so `(1 + mean)^2 = 1` and the implied terminal is the start.
        assert!(approx_eq!(f64, implied, 1000.0, epsilon = 1e-9));
        assert!(approx_eq!(f64, realised, 750.0, epsilon = 1e-9));
        // 1,000 / 750, i.e. the frame's realised factor 0.75 against the construction's 1.0.
        assert!(ratio > 1.0);
        assert!(approx_eq!(f64, ratio, 4.0 / 3.0, epsilon = 1e-12));
        assert_eq!(flagged, 1.0);
    }

    #[rstest]
    fn test_single_period_frame_ratio_is_exactly_one_and_unflagged() {
        let frame = vec![period(
            &[(Currency::USD(), 1000.0)],
            &[(Currency::USD(), 1250.0)],
            Some(0.25),
        )];

        // With one period the mean is the period's own return, so both constructions agree and
        // the quotient is exact: 1,000 * 1.25 / 1,250.
        let ratio = ArithmeticCompoundingRatio::new(None)
            .calculate_from_periods(&frame)
            .unwrap();
        let flagged = ArithmeticCompoundingFlagged::new(None)
            .calculate_from_periods(&frame)
            .unwrap();

        assert_eq!(ratio, 1.0);
        assert_eq!(flagged, 0.0);
    }

    #[rstest]
    fn test_understated_frame_reports_a_ratio_below_one_and_is_flagged() {
        // The frame declared a 2% net return in each period but ended 10% up, so the frame's
        // realised rate (about 4.88% per period) sits above the 2% mean the construction uses.
        let frame = vec![
            period(
                &[(Currency::USD(), 1000.0)],
                &[(Currency::USD(), 1040.4)],
                Some(0.02),
            ),
            period(
                &[(Currency::USD(), 1040.4)],
                &[(Currency::USD(), 1100.0)],
                Some(0.02),
            ),
        ];

        let ratio = ArithmeticCompoundingRatio::new(None)
            .calculate_from_periods(&frame)
            .unwrap();
        let flagged = ArithmeticCompoundingFlagged::new(None)
            .calculate_from_periods(&frame)
            .unwrap();

        assert!(ratio < 1.0);
        assert_eq!(flagged, 1.0);
    }

    #[rstest]
    fn test_multi_currency_equity_is_undefined() {
        let frame = vec![
            period(
                &[(Currency::USD(), 1000.0), (Currency::EUR(), 500.0)],
                &[(Currency::USD(), 1500.0)],
                Some(0.5),
            ),
            period(
                &[(Currency::USD(), 1500.0)],
                &[(Currency::USD(), 750.0)],
                Some(-0.5),
            ),
        ];

        assert_eq!(calculate_all(&frame), vec![None, None, None, None]);
    }

    #[rstest]
    fn test_differing_equity_currencies_are_undefined() {
        let mut frame = overstated_frame();
        frame[1].accounting.ending_equity = CurrencyTotals::new();
        frame[1]
            .accounting
            .ending_equity
            .add_money(Money::new(750.0, Currency::EUR()));

        assert_eq!(calculate_all(&frame), vec![None, None, None, None]);
    }

    #[rstest]
    fn test_empty_frame_is_undefined() {
        assert_eq!(calculate_all(&[]), vec![None, None, None, None]);
    }

    #[rstest]
    fn test_a_period_without_a_defined_net_return_is_undefined() {
        let frame = vec![period(
            &[(Currency::USD(), 1000.0)],
            &[(Currency::USD(), 1100.0)],
            None,
        )];

        assert_eq!(calculate_all(&frame), vec![None, None, None, None]);
    }

    #[rstest]
    fn test_names_match_definition_titles() {
        for (name, definition) in [
            (
                ArithmeticCompoundingImpliedEquity::new(None).name(),
                ArithmeticCompoundingImpliedEquity::new(None).definition(),
            ),
            (
                ArithmeticCompoundingRealisedEquity::new().name(),
                ArithmeticCompoundingRealisedEquity::new().definition(),
            ),
            (
                ArithmeticCompoundingRatio::new(None).name(),
                ArithmeticCompoundingRatio::new(None).definition(),
            ),
            (
                ArithmeticCompoundingFlagged::new(None).name(),
                ArithmeticCompoundingFlagged::new(None).definition(),
            ),
        ] {
            assert_eq!(name, definition.title());
        }
    }

    #[rstest]
    fn test_tolerance_is_declared_and_rendered() {
        // A non-default tolerance is both declared on the definition and printed in the name, so a
        // reader of the rendered row sees the threshold the flag was taken at.
        for statistic in [
            ArithmeticCompoundingImpliedEquity::new(Some(0.05)).name(),
            ArithmeticCompoundingRatio::new(Some(0.05)).name(),
            ArithmeticCompoundingFlagged::new(Some(0.05)).name(),
        ] {
            assert!(statistic.contains("0.05"), "{statistic}");
        }

        let definition = ArithmeticCompoundingFlagged::new(Some(0.05)).definition();
        assert_eq!(
            definition.parameters().get("tolerance").map(String::as_str),
            Some("0.05")
        );
    }

    #[rstest]
    fn test_invalid_tolerance_is_refused_at_construction() {
        assert!(ArithmeticCompoundingImpliedEquity::new_checked(Some(-0.1)).is_err());
        assert!(ArithmeticCompoundingRatio::new_checked(Some(f64::INFINITY)).is_err());
        assert!(ArithmeticCompoundingFlagged::new_checked(Some(-1.0)).is_err());
    }
}
