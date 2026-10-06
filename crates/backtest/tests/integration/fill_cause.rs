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

//! Fills counted by cause, and the rows a run reports for them.

use std::fmt::Debug;

use nautilus_backtest::{
    config::{BacktestEngineConfig, SimulatedVenueConfig},
    engine::BacktestEngine,
};
use nautilus_common::actor::DataActor;
use nautilus_execution::{
    matching_engine::{config::CircuitBreakerConfig, fill_cause::FillCause},
    models::fee::{FeeModelAny, MakerTakerFeeModel},
};
use nautilus_model::{
    data::{Data, QuoteTick},
    enums::{AccountType, BookType, OmsType, OrderSide},
    identifiers::{InstrumentId, StrategyId, Venue},
    instruments::{CryptoPerpetual, Instrument, InstrumentAny, stubs::crypto_perpetual_ethusdt},
    types::{Money, Price, Quantity},
};
use nautilus_trading::{Strategy, StrategyConfig, StrategyCore, nautilus_strategy};
use rstest::rstest;

/// Submits one market buy on the first quote, and optionally one resting limit bid.
struct SubmitOnceStrategy {
    core: StrategyCore,
    instrument_id: InstrumentId,
    resting_bid: bool,
    submitted: bool,
}

impl SubmitOnceStrategy {
    fn new(instrument_id: InstrumentId, resting_bid: bool) -> Self {
        let config = StrategyConfig {
            strategy_id: Some(StrategyId::from("FILL-CAUSE-001")),
            order_id_tag: Some("001".to_string()),
            ..Default::default()
        };
        Self {
            core: StrategyCore::new(config),
            instrument_id,
            resting_bid,
            submitted: false,
        }
    }
}

nautilus_strategy!(SubmitOnceStrategy);

impl Debug for SubmitOnceStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(SubmitOnceStrategy)).finish()
    }
}

impl DataActor for SubmitOnceStrategy {
    fn on_start(&mut self) -> anyhow::Result<()> {
        self.subscribe_quotes(self.instrument_id, None, None);
        Ok(())
    }

    fn on_quote(&mut self, _quote: &QuoteTick) -> anyhow::Result<()> {
        if self.submitted {
            return Ok(());
        }
        self.submitted = true;

        let instrument_id = self.instrument_id;
        let market = self.order().market(
            instrument_id,
            OrderSide::Buy,
            Quantity::from("1.000"),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        self.submit_order(market, None, None, None)?;

        if self.resting_bid {
            let resting = self.order().limit(
                instrument_id,
                OrderSide::Buy,
                Quantity::from("1.000"),
                Price::from("1499.00"),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            );
            self.submit_order(resting, None, None, None)?;
        }

        Ok(())
    }
}

fn engine_with(breaker: Option<CircuitBreakerConfig>, cancel_on_halt: bool) -> BacktestEngine {
    let config = BacktestEngineConfig {
        bypass_logging: true,
        ..Default::default()
    };
    let mut engine = BacktestEngine::new(config).unwrap();
    let venue_config = SimulatedVenueConfig::builder()
        .venue(Venue::from("BINANCE"))
        .oms_type(OmsType::Netting)
        .account_type(AccountType::Margin)
        .book_type(BookType::L1_MBP)
        .starting_balances(vec![Money::from("1_000_000 USDT")])
        .fee_model(FeeModelAny::MakerTaker(MakerTakerFeeModel::zero()).into())
        .cancel_on_halt(cancel_on_halt)
        .maybe_circuit_breaker(breaker)
        .build()
        .unwrap();
    engine.add_venue(venue_config).unwrap();
    engine
}

fn quote(instrument_id: InstrumentId, bid: &str, ask: &str, ts: u64) -> Data {
    Data::Quote(QuoteTick::new(
        instrument_id,
        Price::from(bid),
        Price::from(ask),
        Quantity::from("1.000"),
        Quantity::from("1.000"),
        ts.into(),
        ts.into(),
    ))
}

fn cause_row(engine: &BacktestEngine, cause: FillCause) -> f64 {
    let key = format!("Fill Cause: {}", cause.as_str());
    *engine
        .get_result()
        .stats_general
        .get(&key)
        .unwrap_or_else(|| panic!("missing report row `{key}`"))
}

#[rstest]
fn test_breaker_run_reports_the_breakers_cancels_and_book_fills_separately(
    crypto_perpetual_ethusdt: CryptoPerpetual,
) {
    let instrument = InstrumentAny::CryptoPerpetual(crypto_perpetual_ethusdt);
    let instrument_id = instrument.id();
    let mut engine = engine_with(
        Some(CircuitBreakerConfig {
            move_bps: 100,
            window_ns: 60_000_000_000,
            halt_ns: 30_000_000_000,
        }),
        true,
    );
    engine.add_instrument(&instrument).unwrap();
    engine
        .add_strategy(SubmitOnceStrategy::new(instrument_id, true))
        .unwrap();

    // The first quote fills the market buy and rests the limit bid; the second quote moves the
    // reference two percent, which is beyond the one-percent breaker, so the trip cancels the bid.
    let data = vec![
        quote(instrument_id, "1500.00", "1500.10", 1_000_000_000),
        quote(instrument_id, "1530.00", "1530.10", 2_000_000_000),
    ];
    engine.add_data(data, None, true, true).unwrap();
    engine.run(None, None, None, false).unwrap();

    assert_eq!(cause_row(&engine, FillCause::BookMatch), 1.0);
    assert_eq!(cause_row(&engine, FillCause::CircuitBreaker), 1.0);
    assert_eq!(cause_row(&engine, FillCause::Halt), 0.0);
    assert_eq!(cause_row(&engine, FillCause::PriceBand), 0.0);
    assert_eq!(cause_row(&engine, FillCause::Liquidation), 0.0);
    assert_eq!(cause_row(&engine, FillCause::CorporateAction), 0.0);
}

#[rstest]
fn test_no_rule_run_reports_every_fill_under_book(crypto_perpetual_ethusdt: CryptoPerpetual) {
    let instrument = InstrumentAny::CryptoPerpetual(crypto_perpetual_ethusdt);
    let instrument_id = instrument.id();
    let mut engine = engine_with(None, false);
    engine.add_instrument(&instrument).unwrap();
    engine
        .add_strategy(SubmitOnceStrategy::new(instrument_id, false))
        .unwrap();

    let data = vec![
        quote(instrument_id, "1500.00", "1500.10", 1_000_000_000),
        quote(instrument_id, "1501.00", "1501.10", 2_000_000_000),
    ];
    engine.add_data(data, None, true, true).unwrap();
    engine.run(None, None, None, false).unwrap();

    assert_eq!(cause_row(&engine, FillCause::BookMatch), 1.0);
    for cause in FillCause::ALL {
        if *cause != FillCause::BookMatch {
            assert_eq!(cause_row(&engine, *cause), 0.0, "{cause} should be zero");
        }
    }
}
