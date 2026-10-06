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

//! One de-risking instruction halts, cancels and flattens across strategies and venues.

use std::{cell::RefCell, rc::Rc};

use nautilus_backtest::{
    config::{BacktestEngineConfig, SimulatedVenueConfig},
    engine::BacktestEngine,
};
use nautilus_common::actor::DataActor;
use nautilus_core::UnixNanos;
use nautilus_execution::models::fee::{FeeModelAny, MakerTakerFeeModel};
use nautilus_model::{
    data::{Data, QuoteTick},
    enums::{AccountType, BookType, OmsType, OrderSide},
    events::OrderDenied,
    identifiers::{InstrumentId, StrategyId, Venue},
    instruments::{
        Instrument, InstrumentAny,
        stubs::{audusd_sim, crypto_perpetual_ethusdt},
    },
    types::{Money, Price, Quantity},
};
use nautilus_system::trader::Trader;
use nautilus_trading::{
    Strategy, nautilus_strategy,
    strategy::{StrategyConfig, StrategyCore},
};
use rstest::rstest;

/// The quote each strategy acts on, counted per instrument, so the lever lands mid-run.
const ACT_MARKET: u64 = 1;
const ACT_LIMIT: u64 = 2;
const ACT_DERISK: u64 = 3;
const QUOTES: u64 = 6;
const TRIGGER: &str = "DERISK-TRIGGER";
const PASSIVE: &str = "DERISK-PASSIVE";

/// Trades its own instrument on a fixed schedule and, when told to, pulls the lever.
#[derive(Debug)]
struct DeriskingStrategy {
    core: StrategyCore,
    instrument_id: InstrumentId,
    /// The trader handle only the trigger holds. It is what makes the lever reachable in the
    /// middle of a run, where no caller outside the system can reach the trader at all.
    trader: Option<Rc<RefCell<Trader>>>,
    limit_price: Price,
    quotes: u64,
    /// Every denial the strategy saw, shared with the test rather than read back through the actor
    /// registry, so a denial recorded by either strategy is visible after the run.
    denials: Rc<RefCell<Vec<String>>>,
}

impl DeriskingStrategy {
    fn new(
        strategy_id: &str,
        order_id_tag: &str,
        instrument_id: InstrumentId,
        limit_price: Price,
        denials: Rc<RefCell<Vec<String>>>,
    ) -> Self {
        let config = StrategyConfig {
            strategy_id: Some(StrategyId::from(strategy_id)),
            order_id_tag: Some(order_id_tag.to_string()),
            ..Default::default()
        };

        Self {
            core: StrategyCore::new(config),
            instrument_id,
            trader: None,
            limit_price,
            quotes: 0,
            denials,
        }
    }

    fn trigger(mut self, trader: Rc<RefCell<Trader>>) -> Self {
        self.trader = Some(trader);
        self
    }

    fn trade_size(&self) -> Quantity {
        if self.instrument_id.venue == Venue::from("SIM") {
            Quantity::from("100000")
        } else {
            Quantity::from("1.000")
        }
    }

    fn submit_market(&mut self) -> anyhow::Result<()> {
        let order = self.order().market(
            self.instrument_id,
            OrderSide::Buy,
            self.trade_size(),
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

    fn submit_resting_limit(&mut self) -> anyhow::Result<()> {
        let order = self.order().limit(
            self.instrument_id,
            OrderSide::Buy,
            self.trade_size(),
            self.limit_price,
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

        self.submit_order(order, None, None, None)
    }
}

nautilus_strategy!(DeriskingStrategy, {
    fn on_order_denied(&mut self, event: OrderDenied) {
        self.denials.borrow_mut().push(event.reason.to_string());
    }
});

impl DataActor for DeriskingStrategy {
    fn on_start(&mut self) -> anyhow::Result<()> {
        self.subscribe_quotes(self.instrument_id, None, None);
        Ok(())
    }

    fn on_quote(&mut self, _quote: &QuoteTick) -> anyhow::Result<()> {
        self.quotes += 1;

        match self.quotes {
            ACT_MARKET => self.submit_market()?,
            ACT_LIMIT => self.submit_resting_limit()?,
            ACT_DERISK => {
                let Some(trader) = self.trader.clone() else {
                    // The passive strategy only waits to be de-risked.
                    return Ok(());
                };

                // One instruction, and every registered strategy is told to exit.
                let instructed = Trader::derisk_all(&trader);
                anyhow::ensure!(
                    instructed.len() == 2,
                    "both strategies should be instructed, was {instructed:?}",
                );

                // While the de-risking is carried out, exposure it would add is refused; the
                // reduce-only orders that flatten the positions are not.
                self.submit_market()?;
            }
            _ => {}
        }

        Ok(())
    }
}

fn quotes(instrument_id: InstrumentId, bid: &str, ask: &str, offset: u64) -> Vec<Data> {
    (0..QUOTES)
        .map(|i| {
            let ts = 1_600_000_000_000_000_000 + (i * 1_000_000_000) + offset;

            Data::Quote(QuoteTick {
                instrument_id,
                bid_price: Price::from(bid),
                ask_price: Price::from(ask),
                bid_size: Quantity::from("100"),
                ask_size: Quantity::from("100"),
                ts_event: UnixNanos::from(ts),
                ts_init: UnixNanos::from(ts),
            })
        })
        .collect()
}

#[rstest]
fn test_one_instruction_de_risks_every_strategy_and_venue() {
    let ethusdt = InstrumentAny::CryptoPerpetual(crypto_perpetual_ethusdt());
    let audusd = InstrumentAny::CurrencyPair(audusd_sim());

    let mut engine = BacktestEngine::new(BacktestEngineConfig::default()).unwrap();

    for (venue, instrument) in [("BINANCE", &ethusdt), ("SIM", &audusd)] {
        engine
            .add_venue(
                SimulatedVenueConfig::builder()
                    .venue(Venue::from(venue))
                    .oms_type(OmsType::Netting)
                    .account_type(AccountType::Margin)
                    .book_type(BookType::L1_MBP)
                    .starting_balances(vec![
                        Money::from("1_000_000 USD"),
                        Money::from("1_000_000 USDT"),
                    ])
                    .fee_model(FeeModelAny::MakerTaker(MakerTakerFeeModel::zero()).into())
                    .build()
                    .unwrap(),
            )
            .unwrap();
        engine.add_instrument(instrument).unwrap();
    }

    let trader = Rc::clone(&engine.kernel_mut().trader);
    let denials = Rc::new(RefCell::new(Vec::new()));

    engine
        .add_strategy(
            DeriskingStrategy::new(
                TRIGGER,
                "001",
                ethusdt.id(),
                Price::from("1000.00"),
                Rc::clone(&denials),
            )
            .trigger(trader),
        )
        .unwrap();
    engine
        .add_strategy(DeriskingStrategy::new(
            PASSIVE,
            "002",
            audusd.id(),
            Price::from("0.50000"),
            Rc::clone(&denials),
        ))
        .unwrap();

    // The two instruments are quoted a nanosecond apart, so each strategy acts in turn.
    let eth_quotes = quotes(ethusdt.id(), "2000.00", "2000.10", 0);
    let aud_quotes = quotes(audusd.id(), "1.00000", "1.00010", 1);
    let mut data = Vec::with_capacity(eth_quotes.len() + aud_quotes.len());

    for (eth, aud) in eth_quotes.into_iter().zip(aud_quotes) {
        data.push(eth);
        data.push(aud);
    }

    engine.add_data(data, None, true, true).unwrap();
    engine.run(None, None, None, false).unwrap();

    // The trigger was refused the exposure it tried to add after the lever, and nothing else was
    // refused, so the closing orders the de-risking submitted were permitted.
    assert_eq!(
        *denials.borrow(),
        vec!["MARKET_EXIT_IN_PROGRESS".to_string()],
        "the only refusal should be the exposure the exit forbids",
    );

    // Nothing is left open and nobody holds a position, on either venue.
    let cache = Rc::clone(&engine.kernel_mut().cache);
    let cache = cache.borrow();

    assert_eq!(
        cache.orders_open_count(None, None, None, None, None),
        0,
        "both strategies' resting orders should have been cancelled",
    );
    assert_eq!(
        cache.positions_open_count(None, None, None, None, None),
        0,
        "both strategies' positions should have been flattened",
    );
}
