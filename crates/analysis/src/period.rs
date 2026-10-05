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

//! The performance-period frame and its reduction trigger.
//!
//! A [`PerformancePeriod`] is one row of a periodic result frame: it attributes the portfolio's
//! own accounting and the trading activity observed on fills to a calendar period. The frame is
//! a **projection of authorities that already exist**, never a second ledger:
//!
//! - the portfolio is the authority for realised and unrealised PnL per instrument, for equity
//!   and for the position and exposure counts, and it supplies them through [`PeriodObservation`];
//! - the fills are the authority for trading activity and per-fill costs, and they are supplied
//!   through [`PerformancePeriodReducer::on_fill`].
//!
//! The reducer never recomputes PnL from prices. It differences the portfolio's realised and
//! unrealised PnL between the period's opening and closing observations, and it takes the period's
//! starting and ending equity straight from those observations. A period that has no observation
//! has no row: the portfolio's numbers are required, so an absent authority is not rendered as a
//! zero.
//!
//! # Boundary convention
//!
//! Periods are half-open, `[start, end)`: `start` is inclusive and `end` is exclusive. Boundaries
//! are plain UTC civil-calendar boundaries and no exchange trading calendar is applied - there is
//! none in this repository and none is built here:
//!
//! - [`PeriodKind::Day`] - one UTC day, beginning at `00:00:00` UTC.
//! - [`PeriodKind::IsoWeek`] - the ISO-8601 Monday-to-Sunday week, beginning at the Monday
//!   `00:00:00` UTC (the ISO week *number* is date arithmetic only and is not used here).
//! - [`PeriodKind::Month`] - one UTC calendar month, beginning on the first day at `00:00:00` UTC.
//!
//! Because the boundary is derived from the timestamp alone, the same reducer is driven by a real
//! clock in a live run and by a simulated clock in a backtest, and both produce the same rows for
//! the same period and the same accounting.

use std::{
    collections::BTreeMap,
    fmt::Display,
    ops::{Add, Sub},
    str::FromStr,
};

use nautilus_core::UnixNanos;
use nautilus_model::{
    events::OrderFilled,
    identifiers::InstrumentId,
    types::{Currency, Money},
};
use rust_decimal::{Decimal, prelude::ToPrimitive};

/// The number of nanoseconds in one UTC day.
const NANOS_PER_DAY: u64 = 86_400_000_000_000;

/// The calendar period a [`PerformancePeriod`] covers.
///
/// The set is closed and each variant has a stable string, so a period kind is checkable rather
/// than a spelling competition.
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
pub enum PeriodKind {
    /// One UTC day, beginning at `00:00:00` UTC.
    Day,
    /// The ISO-8601 Monday-to-Sunday week, beginning at the Monday `00:00:00` UTC.
    IsoWeek,
    /// One UTC calendar month, beginning on the first day at `00:00:00` UTC.
    Month,
}

impl PeriodKind {
    /// All variants of the closed vocabulary.
    pub const ALL: &'static [Self] = &[Self::Day, Self::IsoWeek, Self::Month];

    /// Returns the stable string for this period kind.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Day => "day",
            Self::IsoWeek => "iso_week",
            Self::Month => "month",
        }
    }
}

impl Display for PeriodKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for PeriodKind {
    type Err = PeriodKindParseError;

    /// Parses a period kind from its stable string.
    ///
    /// # Errors
    ///
    /// Returns an error if `s` is not one of the closed vocabulary's stable strings.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "day" => Ok(Self::Day),
            "iso_week" => Ok(Self::IsoWeek),
            "month" => Ok(Self::Month),
            _ => Err(PeriodKindParseError {
                value: s.to_string(),
            }),
        }
    }
}

/// The error returned when a string is not a [`PeriodKind`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeriodKindParseError {
    value: String,
}

impl Display for PeriodKindParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "unknown period kind `{}`, expected one of day, iso_week, month",
            self.value
        )
    }
}

impl std::error::Error for PeriodKindParseError {}

/// A deterministic per-currency total of exact money amounts.
///
/// The map is ordered by currency code, so iteration and [`Display`] are stable across runs; the
/// totals are exact [`Money`] values, never a floating point sum, so a multi-currency portfolio
/// does not have its amounts collapsed through an arbitrary conversion.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.analysis", from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.analysis")
)]
pub struct CurrencyTotals {
    totals: BTreeMap<String, Money>,
}

impl CurrencyTotals {
    /// Creates a new empty [`CurrencyTotals`] instance.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns whether no currency is present.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.totals.is_empty()
    }

    /// Returns the number of currencies present.
    #[must_use]
    pub fn len(&self) -> usize {
        self.totals.len()
    }

    /// Adds `money` to its currency's running total.
    pub fn add_money(&mut self, money: Money) {
        self.totals
            .entry(money.currency.code.to_string())
            .and_modify(|existing| *existing = *existing + money)
            .or_insert(money);
    }

    /// Subtracts `money` from its currency's running total, inserting a negative amount when the
    /// currency is not yet present.
    pub fn subtract_money(&mut self, money: Money) {
        self.totals
            .entry(money.currency.code.to_string())
            .and_modify(|existing| *existing = *existing - money)
            .or_insert(-money);
    }

    /// Adds every amount in `other`.
    pub fn add_totals(&mut self, other: &Self) {
        for money in other.totals.values() {
            self.add_money(*money);
        }
    }

    /// Returns the total for `currency`, if present.
    #[must_use]
    pub fn get(&self, currency: &Currency) -> Option<Money> {
        self.totals.get(&currency.code.to_string()).copied()
    }

    /// Returns every amount, ordered by currency code.
    pub fn values(&self) -> impl Iterator<Item = &Money> {
        self.totals.values()
    }

    /// Returns the single amount when exactly one currency is present.
    ///
    /// Returns `None` when the totals are empty or span more than one currency, which is the
    /// condition under which a dimensionless derivative such as a return is not defined.
    #[must_use]
    pub fn as_single_currency(&self) -> Option<Money> {
        if self.totals.len() == 1 {
            self.totals.values().next().copied()
        } else {
            None
        }
    }

    /// Returns the element-wise maximum of the two totals over their union of currencies.
    #[must_use]
    pub fn max(mut self, other: &Self) -> Self {
        for (code, money) in &other.totals {
            match self.totals.get(code) {
                Some(existing) if existing.as_decimal() >= money.as_decimal() => {}
                _ => {
                    self.totals.insert(code.clone(), *money);
                }
            }
        }

        self
    }
}

impl Add for CurrencyTotals {
    type Output = Self;

    fn add(mut self, rhs: Self) -> Self::Output {
        self.add_totals(&rhs);
        self
    }
}

impl Sub for CurrencyTotals {
    type Output = Self;

    fn sub(mut self, rhs: Self) -> Self::Output {
        for money in rhs.totals.into_values() {
            self.subtract_money(money);
        }
        self
    }
}

impl Display for CurrencyTotals {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("{")?;

        for (index, money) in self.totals.values().enumerate() {
            if index > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{money}")?;
        }

        f.write_str("}")
    }
}

/// The accounting fields of a [`PerformancePeriod`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeriodAccounting {
    /// The inclusive start of the period.
    pub start: UnixNanos,
    /// The exclusive end of the period.
    pub end: UnixNanos,
    /// The equity at the period start, from the opening observation.
    pub starting_equity: CurrencyTotals,
    /// The equity at the period end, from the closing observation.
    pub ending_equity: CurrencyTotals,
    /// The realised PnL accrued over the period, as the change in the portfolio's realised PnL.
    pub realized_pnl: CurrencyTotals,
    /// The unrealised PnL accrued over the period, as the change in the portfolio's unrealised PnL.
    pub unrealized_pnl: CurrencyTotals,
    /// The commission paid on the period's fills.
    pub commission: CurrencyTotals,
}

/// The trading-activity fields of a [`PerformancePeriod`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeriodActivity {
    /// The total filled quantity, as an exact decimal sum over the period's fills.
    pub volume: Decimal,
    /// The total notional turnover over the period's fills, per currency.
    pub turnover: CurrencyTotals,
    /// The number of fills in the period.
    pub trade_count: u64,
    /// The number of fills that closed a trade at a positive realised PnL.
    pub winning_trades: u64,
    /// The number of fills that closed a trade at a negative realised PnL.
    pub losing_trades: u64,
}

/// The exposure fields of a [`PerformancePeriod`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeriodExposure {
    /// The number of open positions at the period end.
    pub open_positions: usize,
    /// The gross exposure at the period end, per currency.
    pub gross_exposure: CurrencyTotals,
    /// The net exposure at the period end, per currency.
    pub net_exposure: CurrencyTotals,
}

/// The derived-performance fields of a [`PerformancePeriod`].
#[derive(Debug, Clone, PartialEq)]
pub struct PeriodPerformance {
    /// The net PnL over the period, as the change in equity.
    pub net_pnl: CurrencyTotals,
    /// The net return over the period, when the equity resolves to one currency with a non-zero
    /// starting level; `None` otherwise, because a return across currencies is not defined.
    pub net_return: Option<f64>,
    /// The drawdown at the period end against the running equity peak, per currency.
    pub drawdown: CurrencyTotals,
    /// The drawdown at the period end as a fraction of the running peak, under the same
    /// single-currency condition as [`Self::net_return`].
    pub drawdown_percentage: Option<f64>,
}

/// One row of the periodic result frame.
///
/// The fields are grouped by what they are - accounting, trading activity, exposure and derived
/// performance - so the contract is not a flat bag of numbers.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.analysis", from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.analysis")
)]
pub struct PerformancePeriod {
    /// The accounting fields.
    pub accounting: PeriodAccounting,
    /// The trading-activity fields.
    pub activity: PeriodActivity,
    /// The exposure fields.
    pub exposure: PeriodExposure,
    /// The derived-performance fields.
    pub performance: PeriodPerformance,
}

/// A snapshot of the portfolio's own accounting, the authority the reducer differences.
///
/// Realised and unrealised PnL are keyed by instrument exactly as the portfolio maintains them;
/// equity and the exposure totals are per currency.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PeriodObservation {
    /// The portfolio's realised PnL per instrument.
    pub realized_pnls: BTreeMap<InstrumentId, Money>,
    /// The portfolio's unrealised PnL per instrument.
    pub unrealized_pnls: BTreeMap<InstrumentId, Money>,
    /// The portfolio's total equity per currency.
    pub equity: CurrencyTotals,
    /// The number of open positions.
    pub open_positions: usize,
    /// The gross exposure per currency.
    pub gross_exposure: CurrencyTotals,
    /// The net exposure per currency.
    pub net_exposure: CurrencyTotals,
}

impl PeriodObservation {
    /// Creates a new [`PeriodObservation`] instance.
    #[must_use]
    pub fn new(
        equity: CurrencyTotals,
        open_positions: usize,
        gross_exposure: CurrencyTotals,
        net_exposure: CurrencyTotals,
        realized_pnls: BTreeMap<InstrumentId, Money>,
        unrealized_pnls: BTreeMap<InstrumentId, Money>,
    ) -> Self {
        Self {
            realized_pnls,
            unrealized_pnls,
            equity,
            open_positions,
            gross_exposure,
            net_exposure,
        }
    }
}

/// Reduces the portfolio's accounting and fill activity into [`PerformancePeriod`] rows at UTC
/// civil-calendar boundaries.
///
/// The reducer is clock-driven and engine-agnostic: [`Self::advance`] is given a timestamp and
/// emits a row per boundary crossed, and [`Self::flush`] closes the partial period at an arbitrary
/// timestamp on shutdown or on demand. A live run drives it from the real clock and a backtest
/// from the simulated clock, producing the same rows for the same period and the same accounting.
///
/// The caller supplies the portfolio's authority through [`Self::observe`] and the fill activity
/// through [`Self::on_fill`]. The first observation establishes the opening state; without any
/// observation no row is emitted, because an absent authority is not rendered as a zero.
#[derive(Debug)]
pub struct PerformancePeriodReducer {
    kind: PeriodKind,
    period_start: UnixNanos,
    period_end: UnixNanos,
    volume: Decimal,
    turnover: CurrencyTotals,
    commission: CurrencyTotals,
    trade_count: u64,
    winning_trades: u64,
    losing_trades: u64,
    previous: Option<PeriodObservation>,
    latest: Option<PeriodObservation>,
    peak_equity: Option<CurrencyTotals>,
}

impl PerformancePeriodReducer {
    /// Creates a new [`PerformancePeriodReducer`] instance.
    ///
    /// `start` is the inclusive start of the first period; it does not have to be aligned to a
    /// boundary, in which case the first period is a partial one up to the next boundary.
    #[must_use]
    pub fn new(kind: PeriodKind, start: UnixNanos) -> Self {
        Self {
            kind,
            period_start: start,
            period_end: UnixNanos::from(next_boundary(start.as_u64(), kind)),
            volume: Decimal::ZERO,
            turnover: CurrencyTotals::new(),
            commission: CurrencyTotals::new(),
            trade_count: 0,
            winning_trades: 0,
            losing_trades: 0,
            previous: None,
            latest: None,
            peak_equity: None,
        }
    }

    /// Returns the period kind.
    #[must_use]
    pub const fn kind(&self) -> PeriodKind {
        self.kind
    }

    /// Returns the inclusive start of the current period.
    #[must_use]
    pub const fn period_start(&self) -> UnixNanos {
        self.period_start
    }

    /// Returns the exclusive end of the current period.
    #[must_use]
    pub const fn period_end(&self) -> UnixNanos {
        self.period_end
    }

    /// Records a fill's activity and costs against the current period.
    ///
    /// The fill's quantity and notional (price times quantity, in the fill's currency) are added
    /// to the activity totals and its commission, when present, to the period's commission.
    /// `realized_pnl` is the realised PnL the portfolio attributes to the closed portion of this
    /// fill, if any; a positive value counts as a winning trade, a negative value as a losing one,
    /// and `None` or zero counts as neither.
    pub fn on_fill(&mut self, fill: &OrderFilled, realized_pnl: Option<Money>) {
        self.volume += fill.last_qty.as_decimal();
        self.trade_count += 1;

        let notional = fill.last_px.as_decimal() * fill.last_qty.as_decimal();
        if let Ok(money) = Money::from_decimal(notional, fill.currency) {
            self.turnover.add_money(money);
        }

        if let Some(commission) = fill.commission {
            self.commission.add_money(commission);
        }

        if let Some(pnl) = realized_pnl {
            if pnl.as_decimal() > Decimal::ZERO {
                self.winning_trades += 1;
            } else if pnl.as_decimal() < Decimal::ZERO {
                self.losing_trades += 1;
            }
        }
    }

    /// Records a snapshot of the portfolio's own accounting.
    ///
    /// The first observation establishes the opening state the first period is measured from;
    /// later observations update the closing state. The caller should supply the opening snapshot
    /// before the first period's activity so the first row is complete.
    pub fn observe(&mut self, observation: PeriodObservation) {
        if self.previous.is_none() && self.latest.is_none() {
            self.previous = Some(observation.clone());
        }
        self.latest = Some(observation);
    }

    /// Advances the clock to `ts`, emitting one row for every boundary crossed.
    ///
    /// A boundary is crossed when `ts >= period_end`; the fill at exactly `period_end` therefore
    /// belongs to the period that begins there. A crossed boundary with no observation emits no
    /// row, but the calendar still advances.
    pub fn advance(&mut self, ts: UnixNanos) -> Vec<PerformancePeriod> {
        let mut periods = Vec::new();

        while ts >= self.period_end {
            if let Some(period) = self.build_period(self.period_end) {
                periods.push(period);
            }
            self.period_start = self.period_end;
            self.period_end = UnixNanos::from(next_boundary(self.period_start.as_u64(), self.kind));
        }

        periods
    }

    /// Closes the partial current period at `ts` on shutdown or on demand.
    ///
    /// Returns the partial `[period_start, ts)` row, or `None` when the window is empty or no
    /// observation has been supplied. The calendar restarts at `ts`.
    pub fn flush(&mut self, ts: UnixNanos) -> Option<PerformancePeriod> {
        let period = if ts > self.period_start {
            self.build_period(ts)
        } else {
            None
        };

        self.period_start = ts;
        self.period_end = UnixNanos::from(next_boundary(ts.as_u64(), self.kind));

        period
    }

    /// Builds the row for `[period_start, end)` and resets the per-period accumulators.
    fn build_period(&mut self, end: UnixNanos) -> Option<PerformancePeriod> {
        let latest = self.latest.clone()?;
        let previous = self.previous.clone()?;

        let accounting = PeriodAccounting {
            start: self.period_start,
            end,
            starting_equity: previous.equity.clone(),
            ending_equity: latest.equity.clone(),
            realized_pnl: difference_totals(&latest.realized_pnls, &previous.realized_pnls),
            unrealized_pnl: difference_totals(&latest.unrealized_pnls, &previous.unrealized_pnls),
            commission: std::mem::take(&mut self.commission),
        };

        let activity = PeriodActivity {
            volume: std::mem::take(&mut self.volume),
            turnover: std::mem::take(&mut self.turnover),
            trade_count: std::mem::replace(&mut self.trade_count, 0),
            winning_trades: std::mem::replace(&mut self.winning_trades, 0),
            losing_trades: std::mem::replace(&mut self.losing_trades, 0),
        };

        let exposure = PeriodExposure {
            open_positions: latest.open_positions,
            gross_exposure: latest.gross_exposure.clone(),
            net_exposure: latest.net_exposure.clone(),
        };

        let net_pnl = latest.equity.clone() - previous.equity.clone();
        let net_return = single_currency_return(&previous.equity, &latest.equity);

        let peak = match self.peak_equity.take() {
            Some(peak) => peak.max(&latest.equity),
            None => latest.equity.clone(),
        };
        let drawdown = peak.clone() - latest.equity.clone();
        let drawdown_percentage = single_currency_ratio(&peak, &drawdown);
        self.peak_equity = Some(peak);

        let performance = PeriodPerformance {
            net_pnl,
            net_return,
            drawdown,
            drawdown_percentage,
        };

        self.previous = Some(latest);

        Some(PerformancePeriod {
            accounting,
            activity,
            exposure,
            performance,
        })
    }
}

/// Returns the change in the portfolio's PnL, summing each side per currency first.
fn difference_totals(
    latest: &BTreeMap<InstrumentId, Money>,
    previous: &BTreeMap<InstrumentId, Money>,
) -> CurrencyTotals {
    let mut totals = CurrencyTotals::new();
    for money in latest.values() {
        totals.add_money(*money);
    }

    let mut prior = CurrencyTotals::new();
    for money in previous.values() {
        prior.add_money(*money);
    }

    totals - prior
}

/// Returns `(end - start) / start` when both totals resolve to the same single currency.
fn single_currency_return(start: &CurrencyTotals, end: &CurrencyTotals) -> Option<f64> {
    let start_money = start.as_single_currency()?;
    let end_money = end.as_single_currency()?;

    if start_money.currency != end_money.currency {
        return None;
    }

    let start_value = start_money.as_decimal();
    if start_value.is_zero() {
        return None;
    }

    ((end_money.as_decimal() - start_value) / start_value).to_f64()
}

/// Returns `part / whole` when both totals resolve to the same single currency.
fn single_currency_ratio(whole: &CurrencyTotals, part: &CurrencyTotals) -> Option<f64> {
    let whole_money = whole.as_single_currency()?;
    let part_money = part.as_single_currency()?;

    if whole_money.currency != part_money.currency {
        return None;
    }

    let whole_value = whole_money.as_decimal();
    if whole_value.is_zero() {
        return None;
    }

    (part_money.as_decimal() / whole_value).to_f64()
}

/// Returns the exclusive end of the period that begins at the boundary at or before `start`.
fn next_boundary(start: u64, kind: PeriodKind) -> u64 {
    let day = start / NANOS_PER_DAY;

    match kind {
        PeriodKind::Day => (day + 1) * NANOS_PER_DAY,
        PeriodKind::IsoWeek => {
            // Day 0 (1970-01-01) was a Thursday; shifting by three makes Monday the zero index.
            let weekday = (day + 3) % 7;
            let week_start = day - weekday;
            (week_start + 7) * NANOS_PER_DAY
        }
        PeriodKind::Month => {
            let (year, month, _) = civil_from_days(day);
            let (next_year, next_month) = if month == 12 {
                (year + 1, 1)
            } else {
                (year, month + 1)
            };
            let start_days = days_from_civil(next_year, next_month, 1);
            u64::try_from(start_days).expect("days since epoch is non-negative") * NANOS_PER_DAY
        }
    }
}

/// Maps a count of days since the Unix epoch to a civil `(year, month, day)`.
///
/// Howard Hinnant's civil-calendar algorithm, integer arithmetic only; the input is UTC, so no
/// timezone or leap-second rule is involved.
fn civil_from_days(days_since_epoch: u64) -> (u64, u32, u32) {
    let z = i64::try_from(days_since_epoch).expect("days since epoch fits i64") + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    let year = year + i64::from(month <= 2);

    (
        u64::try_from(year).expect("year is non-negative for representable timestamps"),
        u32::try_from(month).expect("month is positive"),
        u32::try_from(day).expect("day is positive"),
    )
}

/// Maps a civil `(year, month, day)` to a count of days since the Unix epoch.
fn days_from_civil(year: u64, month: u32, day: u32) -> i64 {
    let year = i64::try_from(year).expect("year fits i64");
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let month_prime = (i64::from(month) + 9) % 12;
    let doy = (153 * month_prime + 2) / 5 + i64::from(day) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}
