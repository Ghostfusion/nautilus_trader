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

//! End-to-end reconciliation of the performance-period frame against the portfolio authority.
//!
//! Each test runs one small deterministic backtest and then drives
//! [`PerformancePeriodReducer`] from the portfolio's **own** numbers for that same run: the
//! per-instrument realised and unrealised PnL, the per-currency equity, the open-position count
//! and the per-currency net exposure are captured from the portfolio (through the read-only
//! [`PortfolioApi`] on each quote and from the portfolio itself at the end of the run), never
//! recomputed from prices by the test.
//!
//! The frame is asserted to be a *reduction* of the authority, not a second ledger:
//!
//! - the total of a field summed across every period equals the portfolio's end-of-run total for
//!   that field (equity is reconciled as the change from the opening observation, because the
//!   frame's `net_pnl` is an equity difference);
//! - the last period's closing equity and exposure snapshot equal the portfolio's end-of-run
//!   values.
//!
//! A flat run (no open position at the end) reconciles to zero unrealised PnL, while a run that
//! ends holding an open position does not.

use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

use nautilus_analysis::{
    analyzer::PortfolioAnalyzer,
    metric::MetricStatus,
    period::{
        CurrencyTotals, PerformancePeriod, PerformancePeriodReducer, PeriodKind, PeriodObservation,
    },
};
use nautilus_backtest::{
    config::{BacktestEngineConfig, SimulatedVenueConfig},
    engine::BacktestEngine,
};
use nautilus_common::actor::DataActor;
use nautilus_core::{UnixNanos, approx_eq};
use nautilus_execution::models::fee::{FeeModelAny, MakerTakerFeeModel};
use nautilus_model::{
    data::{Data, QuoteTick},
    enums::{AccountType, BookType, OmsType, OrderSide},
    events::OrderFilled,
    identifiers::{InstrumentId, StrategyId, Venue},
    instruments::{CryptoPerpetual, Instrument, InstrumentAny, stubs::crypto_perpetual_ethusdt},
    types::{Currency, Money, Price, Quantity},
};
use nautilus_portfolio::Portfolio;
use nautilus_trading::{
    nautilus_strategy,
    strategy::{PortfolioApi, Strategy, StrategyConfig, StrategyCore},
};
use rstest::*;
use rust_decimal::Decimal;

/// The number of nanoseconds in one UTC day.
const NANOS_PER_DAY: u64 = 86_400_000_000_000;

/// 2020-01-01T00:00:00Z, a UTC midnight and therefore a day boundary.
const BASE_NS: u64 = 1_577_836_800_000_000_000;

/// A quote whose mid follows the two-sided spread around `mid`.
fn quote(instrument_id: InstrumentId, mid: f64, ts: UnixNanos) -> Data {
    let spread = 0.10;
    let bid = format!("{:.2}", mid - spread / 2.0);
    let ask = format!("{:.2}", mid + spread / 2.0);
    Data::Quote(QuoteTick::new(
        instrument_id,
        Price::from(bid.as_str()),
        Price::from(ask.as_str()),
        Quantity::from("1.000"),
        Quantity::from("1.000"),
        ts,
        ts,
    ))
}

/// Builds the three hourly quotes of one UTC day at `mid`.
fn day_quotes(instrument_id: InstrumentId, day: u64, mid: f64) -> Vec<Data> {
    (0..3)
        .map(|slot| {
            let ts = UnixNanos::from(BASE_NS + day * NANOS_PER_DAY + slot * 3_600_000_000_000);
            quote(instrument_id, mid, ts)
        })
        .collect()
}

/// Collapses a per-instrument PnL map into per-currency totals, exactly as the reducer does.
fn collapse(pnls: &BTreeMap<InstrumentId, Money>) -> CurrencyTotals {
    let mut totals = CurrencyTotals::new();
    for money in pnls.values() {
        totals.add_money(*money);
    }
    totals
}

/// Sums a field of every period, using exact money arithmetic.
fn sum_by(
    rows: &[PerformancePeriod],
    field: impl Fn(&PerformancePeriod) -> CurrencyTotals,
) -> CurrencyTotals {
    rows.iter()
        .fold(CurrencyTotals::new(), |acc, row| acc + field(row))
}

/// Returns the amount held in `code`, treating an absent currency as zero.
fn amount(totals: &CurrencyTotals, code: &str) -> Decimal {
    totals
        .get(&Currency::from(code))
        .map_or(Decimal::ZERO, |money| money.as_decimal())
}

/// Asserts two totals hold the same amount in every currency they mention, treating an absent
/// currency as zero so an empty total reconciles with an explicit zero.
fn assert_totals_reconcile(frame: &CurrencyTotals, authority: &CurrencyTotals, context: &str) {
    let mut codes: Vec<String> = frame
        .values()
        .map(|money| money.currency.code.to_string())
        .collect();
    codes.extend(
        authority
            .values()
            .map(|money| money.currency.code.to_string()),
    );
    codes.sort();
    codes.dedup();

    for code in codes {
        assert_eq!(
            amount(frame, &code),
            amount(authority, &code),
            "{context}: currency {code}",
        );
    }
}

/// Captures the portfolio's accounting through its read-only strategy API.
fn capture_api(
    api: &PortfolioApi<'_>,
    instrument_id: &InstrumentId,
    venue: &Venue,
) -> PeriodObservation {
    let mut realized_pnls = BTreeMap::new();
    if let Some(money) = api.realized_pnl(instrument_id) {
        realized_pnls.insert(*instrument_id, money);
    }

    let mut unrealized_pnls = BTreeMap::new();
    if let Some(money) = api.unrealized_pnl(instrument_id) {
        unrealized_pnls.insert(*instrument_id, money);
    }

    let mut equity = CurrencyTotals::new();
    for money in api.equity(venue, None).into_values() {
        equity.add_money(money);
    }

    let mut net_exposure = CurrencyTotals::new();
    if let Some(exposures) = api.net_exposures(venue, None) {
        for money in exposures.into_values() {
            net_exposure.add_money(money);
        }
    }

    let open_positions = usize::from(!api.net_position(instrument_id).is_zero());

    PeriodObservation::new(
        equity,
        open_positions,
        // The portfolio exposes no gross-exposure aggregate, so the gross field is left empty.
        CurrencyTotals::new(),
        net_exposure,
        realized_pnls,
        unrealized_pnls,
    )
}

/// Captures the portfolio's accounting directly, for the end-of-run snapshot.
fn capture_portfolio(
    portfolio: &mut Portfolio,
    instrument_id: &InstrumentId,
    venue: &Venue,
) -> PeriodObservation {
    let mut realized_pnls = BTreeMap::new();
    if let Some(money) = portfolio.realized_pnl(instrument_id) {
        realized_pnls.insert(*instrument_id, money);
    }

    let mut unrealized_pnls = BTreeMap::new();
    if let Some(money) = portfolio.unrealized_pnl(instrument_id) {
        unrealized_pnls.insert(*instrument_id, money);
    }

    let mut equity = CurrencyTotals::new();
    for money in portfolio.equity(venue, None).into_values() {
        equity.add_money(money);
    }

    let mut net_exposure = CurrencyTotals::new();
    if let Some(exposures) = portfolio.net_exposures(venue, None, None) {
        for money in exposures.into_values() {
            net_exposure.add_money(money);
        }
    }

    let open_positions = usize::from(!portfolio.net_position(instrument_id).is_zero());

    PeriodObservation::new(
        equity,
        open_positions,
        CurrencyTotals::new(),
        net_exposure,
        realized_pnls,
        unrealized_pnls,
    )
}

/// The result of one reconciled run.
struct Scenario {
    rows: Vec<PerformancePeriod>,
    opening: PeriodObservation,
    closing: PeriodObservation,
}

/// A strategy that flips a netting margin position on demand and snapshots the portfolio's own
/// accounting on every quote.
struct PeriodReconciler {
    core: StrategyCore,
    instrument_id: InstrumentId,
    venue: Venue,
    trade_size: Quantity,
    plan: Vec<(u64, OrderSide)>,
    tick_count: u64,
    snapshots: Rc<RefCell<Vec<(UnixNanos, PeriodObservation)>>>,
    fills: Rc<RefCell<Vec<OrderFilled>>>,
}

impl PeriodReconciler {
    fn new(
        instrument_id: InstrumentId,
        venue: Venue,
        plan: Vec<(u64, OrderSide)>,
        snapshots: Rc<RefCell<Vec<(UnixNanos, PeriodObservation)>>>,
        fills: Rc<RefCell<Vec<OrderFilled>>>,
    ) -> Self {
        let config = StrategyConfig {
            strategy_id: Some(StrategyId::from("PERIOD-RECONCILE-001")),
            order_id_tag: Some("001".to_string()),
            ..Default::default()
        };
        Self {
            core: StrategyCore::new(config),
            instrument_id,
            venue,
            trade_size: Quantity::from("1.000"),
            plan,
            tick_count: 0,
            snapshots,
            fills,
        }
    }

    fn submit_market(&mut self, side: OrderSide) -> anyhow::Result<()> {
        let instrument_id = self.instrument_id;
        let trade_size = self.trade_size;
        let order = self.order().market(
            instrument_id,
            side,
            trade_size,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        self.submit_order(order, None, None, None)
    }
}

nautilus_strategy!(PeriodReconciler, {
    fn on_order_filled(&mut self, event: &OrderFilled) {
        self.fills.borrow_mut().push(event.clone());
    }
});

impl std::fmt::Debug for PeriodReconciler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(PeriodReconciler)).finish()
    }
}

impl DataActor for PeriodReconciler {
    fn on_start(&mut self) -> anyhow::Result<()> {
        self.subscribe_quotes(self.instrument_id, None, None);
        Ok(())
    }

    fn on_quote(&mut self, quote: &QuoteTick) -> anyhow::Result<()> {
        self.tick_count += 1;

        if let Some((_, side)) = self.plan.iter().find(|(tick, _)| *tick == self.tick_count) {
            let side = *side;
            self.submit_market(side)?;
        }

        let observation = {
            let api = self.portfolio();
            capture_api(&api, &self.instrument_id, &self.venue)
        };
        let snapshots = Rc::clone(&self.snapshots);
        snapshots.borrow_mut().push((quote.ts_event, observation));

        Ok(())
    }
}

/// Runs one backtest with a zero fee model and reduces the portfolio's own accounting into rows.
fn run_scenario(
    instrument: CryptoPerpetual,
    quotes: Vec<Data>,
    plan: Vec<(u64, OrderSide)>,
) -> Scenario {
    run_scenario_with_fee(
        instrument,
        quotes,
        plan,
        FeeModelAny::MakerTaker(MakerTakerFeeModel::zero()),
    )
}

/// Runs one backtest and reduces the portfolio's own accounting into period rows.
///
/// The reducer is driven by the observations captured from the portfolio during the run, closing
/// on the portfolio's end-of-run snapshot.
fn run_scenario_with_fee(
    instrument: CryptoPerpetual,
    quotes: Vec<Data>,
    plan: Vec<(u64, OrderSide)>,
    fee_model: FeeModelAny,
) -> Scenario {
    let instrument_id = instrument.id();
    let venue = instrument_id.venue;

    let mut engine = BacktestEngine::new(BacktestEngineConfig::default()).unwrap();
    engine
        .add_venue(
            SimulatedVenueConfig::builder()
                .venue(venue)
                .oms_type(OmsType::Netting)
                .account_type(AccountType::Margin)
                .book_type(BookType::L1_MBP)
                .starting_balances(vec![Money::from("1_000_000 USDT")])
                .fee_model(fee_model.into())
                .build()
                .unwrap(),
        )
        .unwrap();
    engine
        .add_instrument(&InstrumentAny::CryptoPerpetual(instrument))
        .unwrap();

    let snapshots = Rc::new(RefCell::new(Vec::new()));
    let fills = Rc::new(RefCell::new(Vec::new()));
    engine
        .add_strategy(PeriodReconciler::new(
            instrument_id,
            venue,
            plan,
            Rc::clone(&snapshots),
            Rc::clone(&fills),
        ))
        .unwrap();

    engine.add_data(quotes, None, true, true).unwrap();
    engine.run(None, None, None, false).unwrap();

    let closing = {
        let mut portfolio = engine.kernel().portfolio.borrow_mut();
        capture_portfolio(&mut portfolio, &instrument_id, &venue)
    };

    let captured = snapshots.borrow().clone();
    let fills = fills.borrow().clone();
    assert!(
        captured.len() >= 2,
        "expected portfolio observations from the run, got {}",
        captured.len(),
    );

    let (start, opening) = captured.first().unwrap().clone();
    let last_ts = captured.last().unwrap().0;

    let mut reducer = PerformancePeriodReducer::new(PeriodKind::Day, start);
    let mut rows = Vec::new();
    let mut fill_iter = fills.into_iter().peekable();

    for (ts, observation) in captured {
        while let Some(fill) = fill_iter.peek() {
            if fill.ts_event <= ts {
                let fill = fill_iter.next().unwrap();
                reducer.on_fill(&fill, None);
            } else {
                break;
            }
        }

        reducer.observe(observation);
        rows.extend(reducer.advance(ts));
    }

    for fill in fill_iter {
        reducer.on_fill(&fill, None);
    }

    reducer.observe(closing.clone());
    let end = UnixNanos::from(last_ts.as_u64() + 1);
    if let Some(row) = reducer.flush(end) {
        rows.push(row);
    }

    Scenario {
        rows,
        opening,
        closing,
    }
}

/// Asserts that the frame's totals reconcile to the portfolio's end-of-run totals.
fn assert_frame_reconciles(scenario: &Scenario) {
    let rows = &scenario.rows;
    let closing = &scenario.closing;

    assert!(
        rows.len() >= 2,
        "expected multiple periods, got {}",
        rows.len(),
    );

    // Each accounting total summed across every period must equal the authority's end-of-run
    // total: the frame is the reduction, not an independent ledger.
    assert_totals_reconcile(
        &sum_by(rows, |row| row.accounting.realized_pnl.clone()),
        &collapse(&closing.realized_pnls),
        "realized_pnl",
    );
    assert_totals_reconcile(
        &sum_by(rows, |row| row.accounting.unrealized_pnl.clone()),
        &collapse(&closing.unrealized_pnls),
        "unrealized_pnl",
    );

    // The frame's net PnL is an equity difference, so it reconciles against the change in the
    // portfolio's equity from the opening observation to the end of the run.
    let equity_change = closing.equity.clone() - scenario.opening.equity.clone();
    assert_totals_reconcile(
        &sum_by(rows, |row| row.performance.net_pnl.clone()),
        &equity_change,
        "net_pnl",
    );

    // The last period closes on the portfolio's end-of-run equity and exposure snapshot.
    let last = rows.last().unwrap();
    assert_totals_reconcile(
        &last.accounting.ending_equity,
        &closing.equity,
        "ending_equity",
    );
    assert_totals_reconcile(
        &last.exposure.net_exposure,
        &closing.net_exposure,
        "net_exposure",
    );
    assert_eq!(
        last.exposure.open_positions, closing.open_positions,
        "open_positions",
    );
}

#[rstest]
fn test_flat_run_reconciles_to_portfolio(crypto_perpetual_ethusdt: CryptoPerpetual) {
    let instrument_id = crypto_perpetual_ethusdt.id();

    // Day 0 opens a long at 1000; day 1 marks it at 1200; day 2 closes it at 1200, so the run
    // ends flat at a profit and day 3 carries no open position.
    let mut quotes = day_quotes(instrument_id, 0, 1000.0);
    quotes.extend(day_quotes(instrument_id, 1, 1200.0));
    quotes.extend(day_quotes(instrument_id, 2, 1200.0));
    quotes.extend(day_quotes(instrument_id, 3, 1200.0));

    let plan = vec![(1, OrderSide::Buy), (7, OrderSide::Sell)];
    let scenario = run_scenario(crypto_perpetual_ethusdt, quotes, plan);

    assert_frame_reconciles(&scenario);

    // A flat run ends with no open position and therefore no unrealised PnL, in both the
    // authority and the frame's total across periods.
    assert_eq!(scenario.closing.open_positions, 0);
    assert!(
        amount(&collapse(&scenario.closing.unrealized_pnls), "USDT").is_zero(),
        "a flat run must reconcile to zero unrealised PnL",
    );
    assert!(
        amount(
            &sum_by(&scenario.rows, |row| row.accounting.unrealized_pnl.clone()),
            "USDT",
        )
        .is_zero(),
        "the frame's unrealised total must be zero when the run ends flat",
    );

    // The run realised a profit, so the reconciliation is not trivially zero.
    assert!(
        !amount(&collapse(&scenario.closing.realized_pnls), "USDT").is_zero(),
        "the flat run is expected to realise a non-zero PnL",
    );
}

#[rstest]
fn test_open_run_reconciles_nonzero_unrealized(crypto_perpetual_ethusdt: CryptoPerpetual) {
    let instrument_id = crypto_perpetual_ethusdt.id();

    // Day 0 opens a long at 1000 and never closes it; the position is marked at 1200 through to
    // the end of the run, so the run ends holding an open position with unrealised PnL.
    let mut quotes = day_quotes(instrument_id, 0, 1000.0);
    quotes.extend(day_quotes(instrument_id, 1, 1200.0));
    quotes.extend(day_quotes(instrument_id, 2, 1200.0));
    quotes.extend(day_quotes(instrument_id, 3, 1200.0));

    let plan = vec![(1, OrderSide::Buy)];
    let scenario = run_scenario(crypto_perpetual_ethusdt, quotes, plan);

    assert_frame_reconciles(&scenario);

    // An open run ends holding one position and a non-zero unrealised PnL, and the frame's total
    // across periods matches it exactly.
    assert_eq!(scenario.closing.open_positions, 1);
    let authority_unrealized = amount(&collapse(&scenario.closing.unrealized_pnls), "USDT");
    assert!(
        !authority_unrealized.is_zero(),
        "the open run is expected to hold non-zero unrealised PnL",
    );
    assert_eq!(
        amount(
            &sum_by(&scenario.rows, |row| row.accounting.unrealized_pnl.clone()),
            "USDT",
        ),
        authority_unrealized,
        "the frame's unrealised total must equal the portfolio's end-of-run unrealised PnL",
    );

    // Nothing was closed, so realised PnL stays zero in both.
    assert!(
        amount(&collapse(&scenario.closing.realized_pnls), "USDT").is_zero(),
        "an open run must not realise PnL",
    );
}

#[rstest]
fn test_cost_row_separates_the_cost_from_the_result(crypto_perpetual_ethusdt: CryptoPerpetual) {
    /// Reads one cost-row metric from the default analyzer over a scenario's frame.
    fn row(scenario: &Scenario, id: &str) -> f64 {
        let analyzer = PortfolioAnalyzer::default();
        let report = analyzer.report_period_metrics(&[id], &scenario.rows);
        let result = report.get(id).unwrap();
        assert_eq!(result.status(), MetricStatus::Computed, "{id}");
        result.value().unwrap()
    }

    let instrument_id = crypto_perpetual_ethusdt.id();

    // Day 0 opens a long at 1000, day 1 marks it at 1200 and day 2 closes it at 1200, so the run
    // ends flat at a profit. The strategy's trade size is fixed, so both runs fill identically.
    let quotes = || {
        let mut quotes = day_quotes(instrument_id, 0, 1000.0);
        quotes.extend(day_quotes(instrument_id, 1, 1200.0));
        quotes.extend(day_quotes(instrument_id, 2, 1200.0));
        quotes.extend(day_quotes(instrument_id, 3, 1200.0));
        quotes
    };
    let plan = vec![(1, OrderSide::Buy), (7, OrderSide::Sell)];

    // Ten basis points on both legs, then the same run with both rates set to zero.
    let charged = run_scenario_with_fee(
        crypto_perpetual_ethusdt.clone(),
        quotes(),
        plan.clone(),
        FeeModelAny::MakerTaker(MakerTakerFeeModel::new(
            Decimal::new(1, 3),
            Decimal::new(1, 3),
        )),
    );
    let free = run_scenario_with_fee(
        crypto_perpetual_ethusdt,
        quotes(),
        plan,
        FeeModelAny::MakerTaker(MakerTakerFeeModel::zero()),
    );

    // The default analyzer carries the cost row, so the report reads it without the caller
    // registering anything.
    let charged_gross = row(&charged, "gross_return");
    let charged_net = row(&charged, "net_return");
    let charged_cost = row(&charged, "cost_basis_points");
    let charged_breakeven = row(&charged, "breakeven_cost");

    // Both legs paid ten basis points, so the reported cost rate is ten basis points of turnover.
    assert!(
        (9.5..=10.5).contains(&charged_cost),
        "expected a cost rate near ten basis points, got {charged_cost}"
    );

    // The fees put the result and its cost apart, and the edge the run earned was worth more than
    // the cost it paid, so the breakeven rate sits above the charged rate.
    assert!(
        charged_gross > charged_net,
        "gross must sit above net when fees are charged, got {charged_gross} and {charged_net}"
    );
    assert!(
        charged_breakeven > charged_cost,
        "the edge per unit traded must exceed the cost per unit traded, got {charged_breakeven}"
    );

    // With both rates set to zero the net return moves onto the gross return, and the gross
    // return is unchanged: the same fills produce the same result before costs.
    let free_gross = row(&free, "gross_return");
    let free_net = row(&free, "net_return");

    assert!(approx_eq!(f64, free_gross, charged_gross, epsilon = 1e-12));
    assert!(approx_eq!(f64, free_net, free_gross, epsilon = 1e-12));
    assert_eq!(row(&free, "cost_basis_points"), 0.0);
    assert_eq!(row(&free, "total_commissions"), 0.0);
}
