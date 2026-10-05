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

//! End-to-end assertion of the performance-period frame and its statistics for a small
//! deterministic four-day run.
//!
//! The run exercises a day with no trades (day 0), a day that opens a position (day 1), a day with
//! an open position marked to market and no trades (day 2), and a day that closes the position so
//! the book ends flat and the equity returns to its opening level (day 3). Every expected value is
//! computed by hand in this file, independently of the implementation.

use std::{collections::BTreeMap, str::FromStr};

use nautilus_analysis::{
    analyzer::PortfolioAnalyzer,
    metric::{MetricReason, MetricStatus},
    period::{
        CurrencyTotals, PerformancePeriod, PerformancePeriodReducer, PeriodKind, PeriodObservation,
    },
    statistic::PortfolioStatistic,
    statistics::{
        breakeven_cost::BreakevenCost, cost_basis_points::CostBasisPoints,
        exponentially_weighted_sharpe::ExponentiallyWeightedSharpe, gross_return::GrossReturn,
        max_drawdown_duration::MaxDrawdownDuration, net_return::NetReturn,
        total_commissions::TotalCommissions, total_turnover::TotalTurnover,
    },
};
use nautilus_core::{UnixNanos, approx_eq};
use nautilus_model::{
    events::order::spec::OrderFilledSpec,
    identifiers::InstrumentId,
    types::{Currency, Money, Price, Quantity},
};
use rust_decimal::Decimal;

/// The number of nanoseconds in one UTC day.
const NANOS_PER_DAY: u64 = 86_400_000_000_000;

/// 2020-01-01T00:00:00Z, a UTC midnight and therefore a day boundary.
const BASE_NS: u64 = 1_577_836_800_000_000_000;

fn at(days: u64) -> UnixNanos {
    UnixNanos::from(BASE_NS + days * NANOS_PER_DAY)
}

fn usd(amount: f64) -> Money {
    Money::new(amount, Currency::USD())
}

/// Builds an observation whose realised and unrealised PnL are single-instrument maps.
fn observation(
    equity: f64,
    open_positions: usize,
    gross: f64,
    net: f64,
    realized: Option<f64>,
    unrealized: Option<f64>,
) -> PeriodObservation {
    let instrument = InstrumentId::from_str("AAPL.XNAS").unwrap();

    let mut equity_totals = CurrencyTotals::new();
    equity_totals.add_money(usd(equity));

    let mut realized_pnls = BTreeMap::new();
    if let Some(pnl) = realized {
        realized_pnls.insert(instrument, usd(pnl));
    }

    let mut unrealized_pnls = BTreeMap::new();
    if let Some(pnl) = unrealized {
        unrealized_pnls.insert(instrument, usd(pnl));
    }

    let mut gross_exposure = CurrencyTotals::new();
    gross_exposure.add_money(usd(gross));
    let mut net_exposure = CurrencyTotals::new();
    net_exposure.add_money(usd(net));

    PeriodObservation::new(
        equity_totals,
        open_positions,
        gross_exposure,
        net_exposure,
        realized_pnls,
        unrealized_pnls,
    )
}

fn fill(
    day: u64,
    quantity: &str,
    price: &str,
    commission: f64,
) -> nautilus_model::events::OrderFilled {
    OrderFilledSpec::builder()
        .last_qty(Quantity::from(quantity))
        .last_px(Price::from(price))
        .currency(Currency::USD())
        .commission(usd(commission))
        .ts_event(at(day))
        .build()
}

/// Builds and drives the four-day run, returning the frame.
fn run() -> Vec<PerformancePeriod> {
    let mut reducer = PerformancePeriodReducer::new(PeriodKind::Day, at(0));

    // Day 0: opening state, no trades. Equity 1000.
    reducer.observe(observation(1000.0, 0, 0.0, 0.0, None, None));
    let mut periods = reducer.advance(at(1));
    assert_eq!(periods.len(), 1);

    // Day 1: buy 10 at 100, commission 5. Equity 1015, unrealised +20.
    reducer.on_fill(&fill(1, "10", "100.00", 5.0), None);
    reducer.observe(observation(1015.0, 1, 1000.0, 1000.0, None, Some(20.0)));
    periods.append(&mut reducer.advance(at(2)));
    assert_eq!(periods.len(), 2);

    // Day 2: no trades, position marked down to 97. Equity 985, unrealised -10.
    reducer.observe(observation(985.0, 1, 970.0, 970.0, None, Some(-10.0)));
    periods.append(&mut reducer.advance(at(3)));
    assert_eq!(periods.len(), 3);

    // Day 3: sell 10 at 101, commission 5, realising +10. Equity 1000, book flat.
    reducer.on_fill(&fill(3, "10", "101.00", 5.0), Some(usd(10.0)));
    reducer.observe(observation(1000.0, 0, 0.0, 0.0, Some(10.0), None));
    let last = reducer.flush(at(4)).expect("flush closes the partial day");
    periods.push(last);

    periods
}

#[test]
fn test_frame_rows_match_hand_computed_expectation() {
    let periods = run();
    assert_eq!(periods.len(), 4);

    // Day 0: no trades, equity unchanged.
    let day0 = &periods[0];
    assert_eq!(day0.accounting.start, at(0));
    assert_eq!(day0.accounting.end, at(1));
    assert_eq!(
        day0.accounting.starting_equity.get(&Currency::USD()),
        Some(usd(1000.0))
    );
    assert_eq!(
        day0.accounting.ending_equity.get(&Currency::USD()),
        Some(usd(1000.0))
    );
    assert_eq!(day0.activity.trade_count, 0);
    assert_eq!(day0.activity.volume, Decimal::ZERO);
    assert!(day0.activity.turnover.is_empty());
    assert_eq!(day0.exposure.open_positions, 0);
    assert_eq!(
        day0.performance.net_pnl.get(&Currency::USD()),
        Some(usd(0.0))
    );
    assert!(approx_eq!(
        f64,
        day0.performance.net_return.unwrap(),
        0.0,
        epsilon = 1e-12
    ));
    assert_eq!(
        day0.performance.drawdown.get(&Currency::USD()),
        Some(usd(0.0))
    );

    // Day 1: one buy fill; unrealised +20; commission 5; equity 1015.
    let day1 = &periods[1];
    assert_eq!(day1.accounting.start, at(1));
    assert_eq!(day1.accounting.end, at(2));
    assert_eq!(
        day1.accounting.starting_equity.get(&Currency::USD()),
        Some(usd(1000.0))
    );
    assert_eq!(
        day1.accounting.ending_equity.get(&Currency::USD()),
        Some(usd(1015.0))
    );
    assert_eq!(day1.accounting.realized_pnl.get(&Currency::USD()), None);
    assert_eq!(
        day1.accounting.unrealized_pnl.get(&Currency::USD()),
        Some(usd(20.0))
    );
    assert_eq!(
        day1.accounting.commission.get(&Currency::USD()),
        Some(usd(5.0))
    );
    assert_eq!(day1.activity.volume, Decimal::from(10));
    assert_eq!(
        day1.activity.turnover.get(&Currency::USD()),
        Some(usd(1000.0))
    );
    assert_eq!(day1.activity.trade_count, 1);
    assert_eq!(day1.activity.winning_trades, 0);
    assert_eq!(day1.activity.losing_trades, 0);
    assert_eq!(day1.exposure.open_positions, 1);
    assert_eq!(
        day1.exposure.gross_exposure.get(&Currency::USD()),
        Some(usd(1000.0))
    );
    assert_eq!(
        day1.exposure.net_exposure.get(&Currency::USD()),
        Some(usd(1000.0))
    );
    assert_eq!(
        day1.performance.net_pnl.get(&Currency::USD()),
        Some(usd(15.0))
    );
    assert!(approx_eq!(
        f64,
        day1.performance.net_return.unwrap(),
        (1015.0 - 1000.0) / 1000.0,
        epsilon = 1e-12
    ));
    assert_eq!(
        day1.performance.drawdown.get(&Currency::USD()),
        Some(usd(0.0))
    );

    // Day 2: no trades; the open position is marked to market; equity 985.
    let day2 = &periods[2];
    assert_eq!(day2.accounting.start, at(2));
    assert_eq!(day2.accounting.end, at(3));
    assert_eq!(
        day2.accounting.starting_equity.get(&Currency::USD()),
        Some(usd(1015.0))
    );
    assert_eq!(
        day2.accounting.ending_equity.get(&Currency::USD()),
        Some(usd(985.0))
    );
    assert_eq!(
        day2.accounting.unrealized_pnl.get(&Currency::USD()),
        Some(usd(-30.0))
    );
    assert_eq!(day2.accounting.commission.get(&Currency::USD()), None);
    assert_eq!(day2.activity.trade_count, 0);
    assert_eq!(day2.exposure.open_positions, 1);
    assert_eq!(
        day2.performance.net_pnl.get(&Currency::USD()),
        Some(usd(-30.0))
    );
    assert!(approx_eq!(
        f64,
        day2.performance.net_return.unwrap(),
        (985.0 - 1015.0) / 1015.0,
        epsilon = 1e-12
    ));
    assert_eq!(
        day2.performance.drawdown.get(&Currency::USD()),
        Some(usd(30.0))
    );
    assert!(approx_eq!(
        f64,
        day2.performance.drawdown_percentage.unwrap(),
        30.0 / 1015.0,
        epsilon = 1e-12
    ));

    // Day 3: closing sell; realised +10; equity returns to the opening 1000; book flat.
    let day3 = &periods[3];
    assert_eq!(day3.accounting.start, at(3));
    assert_eq!(day3.accounting.end, at(4));
    assert_eq!(
        day3.accounting.starting_equity.get(&Currency::USD()),
        Some(usd(985.0))
    );
    assert_eq!(
        day3.accounting.ending_equity.get(&Currency::USD()),
        Some(usd(1000.0))
    );
    assert_eq!(
        day3.accounting.realized_pnl.get(&Currency::USD()),
        Some(usd(10.0))
    );
    assert_eq!(
        day3.accounting.unrealized_pnl.get(&Currency::USD()),
        Some(usd(10.0))
    );
    assert_eq!(
        day3.accounting.commission.get(&Currency::USD()),
        Some(usd(5.0))
    );
    assert_eq!(day3.activity.volume, Decimal::from(10));
    assert_eq!(
        day3.activity.turnover.get(&Currency::USD()),
        Some(usd(1010.0))
    );
    assert_eq!(day3.activity.trade_count, 1);
    assert_eq!(day3.activity.winning_trades, 1);
    assert_eq!(day3.activity.losing_trades, 0);
    assert_eq!(day3.exposure.open_positions, 0);
    assert_eq!(
        day3.performance.net_pnl.get(&Currency::USD()),
        Some(usd(15.0))
    );
    assert!(approx_eq!(
        f64,
        day3.performance.net_return.unwrap(),
        (1000.0 - 985.0) / 985.0,
        epsilon = 1e-12
    ));
    assert_eq!(
        day3.performance.drawdown.get(&Currency::USD()),
        Some(usd(15.0))
    );
    assert!(approx_eq!(
        f64,
        day3.performance.drawdown_percentage.unwrap(),
        15.0 / 1015.0,
        epsilon = 1e-12
    ));
}

#[test]
fn test_max_drawdown_duration_matches_hand_computation() {
    let periods = run();

    // Equity series 1000, 1015, 985, 1000. The peak of 1015 is carried by the period ending at
    // the day 2 boundary and the maximum decline of 30 by the period ending at the day 3 boundary,
    // so the duration is one calendar day.
    let statistic = MaxDrawdownDuration::new();
    let value = statistic.calculate_from_periods(&periods).unwrap();
    assert!(approx_eq!(f64, value, 1.0, epsilon = 1e-12));

    // Undefined for an empty frame.
    assert_eq!(statistic.calculate_from_periods(&[]), None);
}

#[test]
fn test_exponentially_weighted_sharpe_matches_hand_computation() {
    // Three daily returns, oldest first. With halflife 1 the decay is exactly 0.5, so the weights
    // are 0.25, 0.5 and 1.0 from oldest to newest.
    let returns: BTreeMap<UnixNanos, f64> = [(at(0), 0.01), (at(1), 0.02), (at(2), 0.03)]
        .into_iter()
        .collect();

    let expected = {
        let (w_old, w_mid, w_new) = (0.25_f64, 0.5_f64, 1.0_f64);
        let weight_sum = w_old + w_mid + w_new;
        let mean = (0.01 * w_old + 0.02 * w_mid + 0.03 * w_new) / weight_sum;
        let variance = (w_old * (0.01 - mean).powi(2)
            + w_mid * (0.02 - mean).powi(2)
            + w_new * (0.03 - mean).powi(2))
            / weight_sum;
        mean / variance.sqrt() * 252.0_f64.sqrt()
    };

    let statistic = ExponentiallyWeightedSharpe::new(Some(252), Some(1));
    let value = statistic.calculate_from_returns(&returns).unwrap();
    assert!(approx_eq!(f64, value, expected, epsilon = 1e-12));

    // Undefined for an empty returns series.
    assert_eq!(statistic.calculate_from_returns(&BTreeMap::new()), None);
}

#[test]
fn test_cost_totals_match_hand_computation() {
    let periods = run();

    // Turnover: 10 * 100 + 10 * 101 = 2010 USD.
    let turnover = TotalTurnover::new();
    let value = turnover.calculate_from_periods(&periods).unwrap();
    assert!(approx_eq!(f64, value, 2010.0, epsilon = 1e-12));

    // Commissions: 5 + 5 = 10 USD.
    let commissions = TotalCommissions::new();
    let value = commissions.calculate_from_periods(&periods).unwrap();
    assert!(approx_eq!(f64, value, 10.0, epsilon = 1e-12));

    // Undefined for an empty frame; a frame with no trades is a genuine zero.
    assert_eq!(turnover.calculate_from_periods(&[]), None);
    assert_eq!(commissions.calculate_from_periods(&[]), None);
    assert_eq!(turnover.calculate_from_periods(&periods[0..1]), Some(0.0));
    assert_eq!(
        commissions.calculate_from_periods(&periods[0..1]),
        Some(0.0)
    );
}

#[test]
fn test_cost_row_matches_hand_computation() {
    let periods = run();

    // The frame opens and closes at 1000 USD equity, pays 10 USD of commission over 2010 USD of
    // turnover, and trades 10 at 100 then 10 at 101.
    let net = NetReturn::new().calculate_from_periods(&periods).unwrap();
    assert!(approx_eq!(f64, net, 0.0, epsilon = 1e-12));

    let gross = GrossReturn::new().calculate_from_periods(&periods).unwrap();
    assert!(approx_eq!(f64, gross, 0.01, epsilon = 1e-12));

    let cost = CostBasisPoints::new()
        .calculate_from_periods(&periods)
        .unwrap();
    assert!(approx_eq!(
        f64,
        cost,
        10.0 / 2010.0 * 10_000.0,
        epsilon = 1e-9
    ));

    // The frame's gross PnL is exactly its commission, so the rate it could have paid equals the
    // rate it paid; the two rows are the same number here and different in general.
    let breakeven = BreakevenCost::new()
        .calculate_from_periods(&periods)
        .unwrap();
    assert!(approx_eq!(f64, breakeven, cost, epsilon = 1e-9));

    // An empty frame defines none of the four.
    assert_eq!(NetReturn::new().calculate_from_periods(&[]), None);
    assert_eq!(GrossReturn::new().calculate_from_periods(&[]), None);
    assert_eq!(CostBasisPoints::new().calculate_from_periods(&[]), None);
    assert_eq!(BreakevenCost::new().calculate_from_periods(&[]), None);
}

#[test]
fn test_cost_row_is_in_the_default_report_beside_the_returns_metrics() {
    let periods = run();
    let analyzer = PortfolioAnalyzer::default();

    // None of the cost row is registered by the caller: the default analyzer carries it.
    let report = analyzer.report_period_metrics(
        &[
            "sharpe_ratio",
            "gross_return",
            "net_return",
            "cost_basis_points",
            "breakeven_cost",
            "total_commissions",
            "total_turnover",
        ],
        &periods,
    );

    for id in [
        "gross_return",
        "net_return",
        "cost_basis_points",
        "breakeven_cost",
        "total_commissions",
        "total_turnover",
    ] {
        let result = report.get(id).unwrap();
        assert_eq!(result.status(), MetricStatus::Computed, "{id}");
        assert!(result.value().is_some(), "{id}");
    }

    // A returns-based metric is not defined over the frame and is reported, not dropped.
    let sharpe = report.get("sharpe_ratio").unwrap();
    assert_eq!(sharpe.status(), MetricStatus::Unavailable);
    assert_eq!(sharpe.reason(), Some(MetricReason::UnsupportedInput));

    // The gross and net rows stay apart, and their difference is the cost the frame paid.
    let gross = report.get("gross_return").unwrap().value().unwrap();
    let net = report.get("net_return").unwrap().value().unwrap();
    assert!(approx_eq!(f64, gross - net, 0.01, epsilon = 1e-12));
}

#[test]
fn test_analyzer_report_classifies_period_metrics() {
    let periods = run();
    let mut analyzer = PortfolioAnalyzer::default();

    // The default analyzer carries the frame statistics, so only the duration statistic needs
    // registering, exactly as a caller would to add a metric to a report.
    analyzer.register_statistic(std::sync::Arc::new(MaxDrawdownDuration::new()));

    let report = analyzer.report_period_metrics(
        &["max_drawdown_duration", "total_commissions", "sharpe_ratio"],
        &periods,
    );

    let duration = report.get("max_drawdown_duration").unwrap();
    assert_eq!(duration.status(), MetricStatus::Computed);
    assert!(duration.value().is_some());

    let commissions = report.get("total_commissions").unwrap();
    assert_eq!(commissions.status(), MetricStatus::Computed);

    // A returns-based metric is not defined over the frame and is reported, not dropped.
    let sharpe = report.get("sharpe_ratio").unwrap();
    assert_eq!(sharpe.status(), MetricStatus::Unavailable);
    assert_eq!(sharpe.reason(), Some(MetricReason::UnsupportedInput));

    // An empty frame is unavailable with the insufficient-data reason.
    let empty = analyzer.report_period_metrics(&["total_turnover"], &[]);
    let turnover = empty.get("total_turnover").unwrap();
    assert_eq!(turnover.status(), MetricStatus::Unavailable);
    assert_eq!(turnover.reason(), Some(MetricReason::InsufficientData));
}
