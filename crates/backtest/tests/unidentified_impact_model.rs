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

//! Acceptance for labelling an unidentified market impact model (`design D4`, work item I2.4).
//!
//! The three runs below configure the same impact model and execute the same scenario; they differ
//! only in what the caller declares about the model's recovery. An unidentified declaration marks
//! every cost row the model's fills contribute to with the label naming the failing parameter, an
//! identified declaration prints no label at all, and no declaration behaves as before the change.
//! The model still runs in all three: the platform labels the number rather than dropping it.

use nautilus_backtest::{
    config::{
        BacktestEngineConfig, ImpactModelVerdict, MarketImpactIdentification, SimulatedVenueConfig,
    },
    engine::BacktestEngine,
    result::BacktestResult,
};
use nautilus_common::actor::DataActor;
use nautilus_core::UnixNanos;
use nautilus_execution::models::{
    fee::{FeeModelAny, MakerTakerFeeModel},
    market_impact::{
        ImpactCalibrationSource, MarketImpactModelHandle, PrefactorInterval,
        SquareRootMarketImpactModel,
    },
};
use nautilus_model::{
    data::{Data, QuoteTick},
    enums::{AccountType, BookType, OmsType, OrderSide},
    identifiers::{InstrumentId, StrategyId},
    instruments::{Instrument, InstrumentAny, stubs::crypto_perpetual_ethusdt},
    types::{Money, Price, Quantity},
};
use nautilus_trading::{
    nautilus_strategy,
    strategy::{Strategy, StrategyConfig, StrategyCore},
};

/// The number of nanoseconds in one UTC day.
const NANOS_PER_DAY: u64 = 86_400_000_000_000;

/// 2020-01-01T00:00:00Z, a UTC midnight and therefore a day boundary.
const BASE_NS: u64 = 1_577_836_800_000_000_000;

/// The label a not-identified declaration carries, naming the failing parameter.
const LABEL: &str = "unidentified impact model: prefactor";

/// The cost rows a market-impact-adjusted fill contributes to, split by the map they land in.
const COST_ROWS_RETURNS: [&str; 4] = [
    "Breakeven Cost (basis points of turnover)",
    "Cost (basis points of turnover)",
    "Gross Return",
    "Net Return",
];

const COST_ROWS_GENERAL: [&str; 2] = ["Total Commissions", "Total Turnover"];

/// Returns the marked name of a cost row under [`LABEL`].
fn marked(name: &str) -> String {
    format!("{name} [{LABEL}]")
}

/// A quote whose mid follows the two-sided spread around `mid`.
fn quote(instrument_id: InstrumentId, mid: f64, ts: UnixNanos) -> Data {
    let bid = format!("{:.2}", mid - 0.05);
    let ask = format!("{:.2}", mid + 0.05);
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

/// A strategy that submits one market order per tick listed in its plan.
struct SubmitPlanStrategy {
    core: StrategyCore,
    instrument_id: InstrumentId,
    plan: Vec<(u64, OrderSide)>,
    tick_count: u64,
}

impl SubmitPlanStrategy {
    fn new(instrument_id: InstrumentId, plan: Vec<(u64, OrderSide)>) -> Self {
        let config = StrategyConfig {
            strategy_id: Some(StrategyId::from("UNIDENTIFIED-IMPACT-001")),
            order_id_tag: Some("001".to_string()),
            ..Default::default()
        };
        Self {
            core: StrategyCore::new(config),
            instrument_id,
            plan,
            tick_count: 0,
        }
    }
}

nautilus_strategy!(SubmitPlanStrategy);

impl std::fmt::Debug for SubmitPlanStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(SubmitPlanStrategy)).finish()
    }
}

impl DataActor for SubmitPlanStrategy {
    fn on_start(&mut self) -> anyhow::Result<()> {
        self.subscribe_quotes(self.instrument_id, None, None);
        Ok(())
    }

    fn on_quote(&mut self, _quote: &QuoteTick) -> anyhow::Result<()> {
        self.tick_count += 1;

        if let Some((_, side)) = self.plan.iter().find(|(tick, _)| *tick == self.tick_count) {
            let order = self.order().market(
                self.instrument_id,
                *side,
                Quantity::from("1.000"),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
            );
            self.submit_order(order, None, None, None)?;
        }

        Ok(())
    }
}

/// The impact model all three runs configure: a square-root model whose fills move by one
/// increment, so the model is exercised rather than left unused.
fn impact_model() -> MarketImpactModelHandle {
    let prefactor =
        PrefactorInterval::new(0.5, 1.5, ImpactCalibrationSource::Assumed).expect("valid bounds");
    let model = SquareRootMarketImpactModel::new(prefactor, Quantity::from("1.000"), 10)
        .expect("valid model");
    MarketImpactModelHandle::new(model)
}

/// Runs one deterministic scenario with the given declaration.
///
/// Day 0 opens a long at 1000 and day 2 closes it at 1200, so the run produces the cost rows. The
/// declaration changes no execution: only the label the report carries.
fn run(identification: Option<MarketImpactIdentification>) -> BacktestResult {
    let instrument = InstrumentAny::CryptoPerpetual(crypto_perpetual_ethusdt());
    let instrument_id = instrument.id();
    let venue = instrument_id.venue;

    let mut engine = BacktestEngine::new(BacktestEngineConfig {
        bypass_logging: true,
        ..Default::default()
    })
    .unwrap();
    engine
        .add_venue(
            SimulatedVenueConfig::builder()
                .venue(venue)
                .oms_type(OmsType::Netting)
                .account_type(AccountType::Margin)
                .book_type(BookType::L1_MBP)
                .starting_balances(vec![Money::from("1_000_000 USDT")])
                .fee_model(FeeModelAny::MakerTaker(MakerTakerFeeModel::zero()).into())
                .maybe_market_impact_model(Some(impact_model()))
                .maybe_market_impact_identification(identification)
                .build()
                .unwrap(),
        )
        .unwrap();
    engine.add_instrument(&instrument).unwrap();
    engine
        .add_strategy(SubmitPlanStrategy::new(
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

    engine.get_result()
}

fn returns_row(result: &BacktestResult, name: &str) -> Option<f64> {
    result.stats_returns.get(name).copied()
}

fn general_row(result: &BacktestResult, name: &str) -> Option<f64> {
    result.stats_general.get(name).copied()
}

#[test]
fn test_unidentified_impact_model_marks_the_cost_rows() {
    let result = run(Some(MarketImpactIdentification::unidentified("prefactor")));

    // Every cost row the model's fills contribute to is marked with the label naming the
    // parameter, and keeps its value: the row is labelled, not dropped.
    for name in COST_ROWS_RETURNS {
        assert!(
            returns_row(&result, &marked(name)).is_some(),
            "missing marked returns row `{}`",
            marked(name),
        );
        assert!(
            returns_row(&result, name).is_none(),
            "unmarked returns row `{name}` should be replaced by its marked form",
        );
    }
    for name in COST_ROWS_GENERAL {
        assert!(
            general_row(&result, &marked(name)).is_some(),
            "missing marked general row `{}`",
            marked(name),
        );
        assert!(
            general_row(&result, name).is_none(),
            "unmarked general row `{name}` should be replaced by its marked form",
        );
    }

    // The run summary says the model is unidentified and names the failing parameter.
    assert_eq!(
        result.summary.get("market_impact.BINANCE.identification"),
        Some(&ImpactModelVerdict::Unidentified.as_str().to_string()),
    );
    assert_eq!(
        result.summary.get("market_impact.BINANCE.failed_parameter"),
        Some(&"prefactor".to_string()),
    );

    // The model still runs: the marked rows carry the same numbers the identified run produces.
    let identified = run(Some(MarketImpactIdentification::identified()));
    for name in COST_ROWS_RETURNS {
        assert_eq!(
            returns_row(&result, &marked(name)),
            returns_row(&identified, name),
            "the model's number must not change with the label",
        );
    }
    for name in COST_ROWS_GENERAL {
        assert_eq!(
            general_row(&result, &marked(name)),
            general_row(&identified, name),
            "the model's number must not change with the label",
        );
    }
}

#[test]
fn test_identified_impact_model_prints_no_label() {
    let result = run(Some(MarketImpactIdentification::identified()));

    // An identified model prints no label: the marked row is absent rather than zero, and the
    // plain cost row is present.
    for name in COST_ROWS_RETURNS {
        assert!(
            returns_row(&result, &marked(name)).is_none(),
            "an identified model must not print `{}`",
            marked(name),
        );
        assert!(
            returns_row(&result, name).is_some(),
            "missing unmarked returns row `{name}`",
        );
    }
    for name in COST_ROWS_GENERAL {
        assert!(
            general_row(&result, &marked(name)).is_none(),
            "an identified model must not print `{}`",
            marked(name),
        );
        assert!(
            general_row(&result, name).is_some(),
            "missing unmarked general row `{name}`",
        );
    }

    // No label reaches the summary either.
    assert!(
        !result
            .summary
            .keys()
            .any(|key| key.starts_with("market_impact.")),
        "an identified model must add no summary entry",
    );
}

#[test]
fn test_no_declaration_behaves_as_before() {
    let result = run(None);

    for name in COST_ROWS_RETURNS {
        assert!(
            returns_row(&result, &marked(name)).is_none(),
            "no declaration must not print `{}`",
            marked(name),
        );
        assert!(
            returns_row(&result, name).is_some(),
            "missing unmarked returns row `{name}`",
        );
    }
    for name in COST_ROWS_GENERAL {
        assert!(
            general_row(&result, &marked(name)).is_none(),
            "no declaration must not print `{}`",
            marked(name),
        );
        assert!(
            general_row(&result, name).is_some(),
            "missing unmarked general row `{name}`",
        );
    }

    assert!(
        !result
            .summary
            .keys()
            .any(|key| key.starts_with("market_impact.")),
        "no declaration must add no summary entry",
    );
}
