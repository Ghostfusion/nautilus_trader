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

//! End-to-end assertion that the default backtest report carries the cost row.
//!
//! Two runs over the same small deterministic scenario differ only in the fee model: one charges
//! no maker-taker fee and one charges ten basis points on both legs. In both, the portfolio's
//! statistics snapshot must carry the frame's cost and return rows without the caller registering
//! anything, and the rows must obey the frame's own arithmetic: the gap between the gross and net
//! returns is the commission as a share of the starting equity, and the printed cost row is the
//! same commission per unit of turnover.

use nautilus_analysis::period::{CurrencyTotals, PerformancePeriod};
use nautilus_backtest::{
    config::{BacktestEngineConfig, SimulatedVenueConfig},
    engine::BacktestEngine,
    result::BacktestResult,
};
use nautilus_common::actor::DataActor;
use nautilus_core::{UnixNanos, approx_eq};
use nautilus_execution::models::fee::{FeeModelAny, MakerTakerFeeModel};
use nautilus_model::{
    data::{Data, QuoteTick},
    enums::{AccountType, BookType, OmsType, OrderSide},
    identifiers::{InstrumentId, StrategyId},
    instruments::{CryptoPerpetual, Instrument, InstrumentAny, stubs::crypto_perpetual_ethusdt},
    types::{Currency, Money, Price, Quantity},
};
use nautilus_trading::{
    nautilus_strategy,
    strategy::{Strategy, StrategyConfig, StrategyCore},
};
use rstest::*;
use rust_decimal::{Decimal, prelude::ToPrimitive};

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

/// Sums a money field across every period with exact money arithmetic.
fn sum_money(
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

/// A strategy that submits one market order per tick listed in its plan.
struct CostStrategy {
    core: StrategyCore,
    instrument_id: InstrumentId,
    trade_size: Quantity,
    plan: Vec<(u64, OrderSide)>,
    tick_count: u64,
}

impl CostStrategy {
    fn new(instrument_id: InstrumentId, plan: Vec<(u64, OrderSide)>) -> Self {
        let config = StrategyConfig {
            strategy_id: Some(StrategyId::from("COST-REPORT-001")),
            order_id_tag: Some("001".to_string()),
            ..Default::default()
        };
        Self {
            core: StrategyCore::new(config),
            instrument_id,
            trade_size: Quantity::from("1.000"),
            plan,
            tick_count: 0,
        }
    }

    fn submit_market(&mut self, side: OrderSide) -> anyhow::Result<()> {
        let order = self.order().market(
            self.instrument_id,
            side,
            self.trade_size,
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

nautilus_strategy!(CostStrategy);

impl std::fmt::Debug for CostStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(CostStrategy)).finish()
    }
}

impl DataActor for CostStrategy {
    fn on_start(&mut self) -> anyhow::Result<()> {
        self.subscribe_quotes(self.instrument_id, None, None);
        Ok(())
    }

    fn on_quote(&mut self, _quote: &QuoteTick) -> anyhow::Result<()> {
        self.tick_count += 1;

        if let Some((_, side)) = self.plan.iter().find(|(tick, _)| *tick == self.tick_count) {
            let side = *side;
            self.submit_market(side)?;
        }

        Ok(())
    }
}

/// The observable outcome of one run: the report and the frame it was reduced from.
struct Run {
    result: BacktestResult,
    periods: Vec<PerformancePeriod>,
}

/// Runs one deterministic scenario with the given fee model.
///
/// Day 0 opens a long at 1000 and day 2 closes it at 1200; the position is marked at 1200 through
/// day 3, so the run ends flat at a profit. The strategy's trade size is fixed, so both runs fill
/// identically and differ only in what each fill cost.
fn run(instrument: CryptoPerpetual, fee_model: FeeModelAny) -> Run {
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

    engine
        .add_strategy(CostStrategy::new(
            instrument_id,
            vec![(1, OrderSide::Buy), (7, OrderSide::Sell)],
        ))
        .unwrap();

    let mut quotes = day_quotes(instrument_id, 0, 1000.0);
    quotes.extend(day_quotes(instrument_id, 1, 1200.0));
    quotes.extend(day_quotes(instrument_id, 2, 1200.0));
    quotes.extend(day_quotes(instrument_id, 3, 1200.0));
    engine.add_data(quotes, None, true, true).unwrap();

    engine.run(None, None, None, false).unwrap();

    let result = engine.get_result();
    let periods = engine.kernel().portfolio.borrow().performance_periods();

    Run { result, periods }
}

#[rstest]
fn test_default_report_carries_the_cost_row(crypto_perpetual_ethusdt: CryptoPerpetual) {
    let free = run(
        crypto_perpetual_ethusdt.clone(),
        FeeModelAny::MakerTaker(MakerTakerFeeModel::zero()),
    );
    let charged = run(
        crypto_perpetual_ethusdt,
        FeeModelAny::MakerTaker(MakerTakerFeeModel::new(
            Decimal::new(1, 3),
            Decimal::new(1, 3),
        )),
    );

    // The default analyzer carries the cost row, so the report reads it without the caller
    // registering anything. Both runs fill identically and therefore trade the same turnover.
    for (label, run) in [("zero fee", &free), ("charged", &charged)] {
        for name in [
            "Cost (basis points of turnover)",
            "Gross Return",
            "Net Return",
        ] {
            assert!(
                run.result.stats_returns.contains_key(name),
                "{label}: missing `{name}` in the returns rows",
            );
        }
        for name in ["Total Commissions", "Total Turnover"] {
            assert!(
                run.result.stats_general.contains_key(name),
                "{label}: missing `{name}` in the general rows",
            );
        }
    }

    // With no fee the result is identical before and after costs, and the cost rate is a genuine
    // zero because the run traded and paid nothing.
    let free_gross = free.result.stats_returns["Gross Return"];
    let free_net = free.result.stats_returns["Net Return"];
    assert!(approx_eq!(f64, free_gross, free_net, epsilon = 1e-12));
    assert_eq!(
        free.result.stats_returns["Cost (basis points of turnover)"],
        0.0
    );
    assert_eq!(free.result.stats_general["Total Commissions"], 0.0);

    // With a fee the net return sits below the gross return, and the cost row is funded by the
    // commission the frame recorded.
    let gross = charged.result.stats_returns["Gross Return"];
    let net = charged.result.stats_returns["Net Return"];
    let cost = charged.result.stats_returns["Cost (basis points of turnover)"];
    assert!(
        net < gross,
        "net must sit below gross when fees are charged, got {net} and {gross}",
    );

    let commission = amount(
        &sum_money(&charged.periods, |row| row.accounting.commission.clone()),
        "USDT",
    );
    let turnover = amount(
        &sum_money(&charged.periods, |row| row.activity.turnover.clone()),
        "USDT",
    );
    let starting_equity = charged
        .periods
        .first()
        .map(|row| amount(&row.accounting.starting_equity, "USDT"))
        .expect("the charged run must have reduced at least one period");

    assert!(!commission.is_zero(), "the charged run paid no commission");
    assert!(!turnover.is_zero(), "the charged run traded no turnover");

    let commission = commission.to_f64().unwrap();
    let turnover = turnover.to_f64().unwrap();
    let starting_equity = starting_equity.to_f64().unwrap();

    // (a) The gap between the gross and the net return is the commission as a share of the
    // starting equity.
    assert!(
        approx_eq!(
            f64,
            gross - net,
            commission / starting_equity,
            epsilon = 1e-12
        ),
        "gross {gross} net {net} commission {commission} equity {starting_equity}",
    );

    // (b) The printed cost row is the same commission per unit of turnover, in basis points.
    //
    // These are two different ratios - (a) is over equity, (b) over turnover - so the gap and the
    // basis-point figure are not numerically equal, and this test does not assert that they are.
    assert!(
        approx_eq!(f64, cost, commission / turnover * 10_000.0, epsilon = 1e-9),
        "cost {cost} commission {commission} turnover {turnover}",
    );
}
