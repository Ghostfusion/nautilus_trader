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

//! The opt-in corporate action adjustment stage.
//!
//! When a data configuration carries a [`DataAdjustment`] whose representations differ, the stage
//! converts the prices of the loaded market data between representations and appends the
//! instrument's [`CorporateAction`] records to the stream. The default path (no adjustment, or a
//! no-op combination) passes the catalog data through untouched.
//!
//! The conversion math lives in [`AdjustmentSeries`]; this module only locates the series for each
//! record and rebuilds the record through its own constructor so every other field is preserved.

use ahash::AHashMap;
use nautilus_core::UnixNanos;
use nautilus_model::{
    data::{AdjustmentSeries, Bar, CorporateAction, Data, HasTsInit, QuoteTick, TradeTick},
    identifiers::InstrumentId,
    types::Price,
};

use crate::config::DataAdjustment;

/// Groups a flat action list into one [`AdjustmentSeries`] per instrument.
///
/// # Errors
///
/// Returns an error if any action does not belong to its group's instrument or a split ratio is
/// not positive (see [`AdjustmentSeries::new`]).
pub fn build_series(
    actions: &[CorporateAction],
) -> anyhow::Result<AHashMap<InstrumentId, AdjustmentSeries>> {
    let mut grouped: AHashMap<InstrumentId, Vec<CorporateAction>> = AHashMap::new();
    for action in actions {
        grouped
            .entry(action.instrument_id)
            .or_default()
            .push(*action);
    }

    let mut series = AHashMap::with_capacity(grouped.len());
    for (instrument_id, actions) in grouped {
        series.insert(
            instrument_id,
            AdjustmentSeries::new(instrument_id, actions)?,
        );
    }
    Ok(series)
}

/// Returns the corporate action records to replay, ordered by their effective instant.
///
/// Each record is re-stamped so its replay key (`ts_init`) is its `effective_ns`, mirroring how
/// instrument-scoped auxiliary data such as `InstrumentStatus`/`InstrumentClose` is ordered. Every
/// other field, including `effective_ns` and `ts_event`, is preserved exactly.
#[must_use]
pub fn action_records(actions: &[CorporateAction]) -> Vec<Data> {
    let mut records: Vec<Data> = actions
        .iter()
        .map(|action| {
            Data::CorporateAction(CorporateAction {
                ts_init: action.effective_ns,
                ..*action
            })
        })
        .collect();
    records.sort_by_key(HasTsInit::ts_init);
    records
}

/// Converts one loaded record's prices between representations.
///
/// Only `Bar` open/high/low/close, `QuoteTick` bid/ask, and `TradeTick` price carry a price to
/// convert; any other record (and any record whose instrument has no action series) is returned
/// unchanged. Every other field is preserved exactly.
///
/// # Errors
///
/// Returns an error if the conversion is not exact for the record's price precision, or if the
/// rebuilt record fails its own constructor validation.
pub fn convert_data(
    data: Data,
    adjustment: DataAdjustment,
    series: &AHashMap<InstrumentId, AdjustmentSeries>,
) -> anyhow::Result<Data> {
    Ok(match data {
        Data::Bar(bar) => {
            let Some(series) = series.get(&bar.instrument_id()) else {
                return Ok(Data::Bar(bar));
            };

            let ts = bar.ts_event;
            let open = convert_price(series, bar.open, ts, adjustment)?;
            let high = convert_price(series, bar.high, ts, adjustment)?;
            let low = convert_price(series, bar.low, ts, adjustment)?;
            let close = convert_price(series, bar.close, ts, adjustment)?;
            Data::Bar(Bar::new_checked(
                bar.bar_type,
                open,
                high,
                low,
                close,
                bar.volume,
                bar.ts_event,
                bar.ts_init,
            )?)
        }
        Data::Quote(quote) => {
            let Some(series) = series.get(&quote.instrument_id) else {
                return Ok(Data::Quote(quote));
            };

            let ts = quote.ts_event;
            let bid_price = convert_price(series, quote.bid_price, ts, adjustment)?;
            let ask_price = convert_price(series, quote.ask_price, ts, adjustment)?;
            Data::Quote(QuoteTick::new_checked(
                quote.instrument_id,
                bid_price,
                ask_price,
                quote.bid_size,
                quote.ask_size,
                quote.ts_event,
                quote.ts_init,
            )?)
        }
        Data::Trade(trade) => {
            let Some(series) = series.get(&trade.instrument_id) else {
                return Ok(Data::Trade(trade));
            };

            let ts = trade.ts_event;
            let price = convert_price(series, trade.price, ts, adjustment)?;
            Data::Trade(TradeTick::new_checked(
                trade.instrument_id,
                price,
                trade.size,
                trade.aggressor_side,
                trade.trade_id,
                trade.ts_event,
                trade.ts_init,
            )?)
        }
        other => other,
    })
}

fn convert_price(
    series: &AdjustmentSeries,
    price: Price,
    ts: UnixNanos,
    adjustment: DataAdjustment,
) -> anyhow::Result<Price> {
    series.convert(price, ts, adjustment.input, adjustment.output)
}

/// Applies the adjustment stage to a fully loaded batch of data.
///
/// When the combination is a no-op the data is returned byte-identical, including its order. When
/// it is active, every price-bearing record is converted and the action records are appended.
///
/// # Errors
///
/// Returns an error if the series cannot be built or a record cannot be converted.
pub fn apply_to_vec(
    data: Vec<Data>,
    actions: &[CorporateAction],
    adjustment: DataAdjustment,
) -> anyhow::Result<Vec<Data>> {
    if adjustment.is_noop() {
        return Ok(data);
    }

    let series = build_series(actions)?;
    let mut adjusted = Vec::with_capacity(data.len() + actions.len());
    for record in data {
        adjusted.push(convert_data(record, adjustment, &series)?);
    }
    adjusted.extend(action_records(actions));
    Ok(adjusted)
}

/// Returns the configured representation pair, or `None` when the stage is disabled.
#[must_use]
pub const fn active_adjustment(adjustment: Option<DataAdjustment>) -> Option<DataAdjustment> {
    match adjustment {
        Some(adjustment) if !adjustment.is_noop() => Some(adjustment),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use nautilus_model::{
        data::{CorporateActionType, DataRef, PriceRepresentation},
        enums::{AggregationSource, BarAggregation, PriceType},
        identifiers::TradeId,
        types::{Quantity, fixed::FIXED_PRECISION},
    };
    use rstest::rstest;
    use rust_decimal::Decimal;

    use super::*;
    use crate::{config::DataAdjustment, data_iterator::BacktestDataIterator};

    const EFFECT_NS: u64 = 2_000_000_000_000_000_000;
    const EARLY_NS: u64 = 1_000_000_000_000_000_000;
    const LATE_NS: u64 = 3_000_000_000_000_000_000;

    fn instrument_id() -> InstrumentId {
        InstrumentId::from("AAPL.XNYS")
    }

    fn action(action: CorporateActionType, value: &str, effective_ns: u64) -> CorporateAction {
        CorporateAction::new(
            instrument_id(),
            action,
            value.parse::<Decimal>().unwrap(),
            None,
            effective_ns.into(),
            effective_ns.into(),
            effective_ns.into(),
        )
    }

    fn bar(ts: u64) -> Bar {
        Bar::new(
            nautilus_model::data::BarType::new(
                instrument_id(),
                nautilus_model::data::BarSpecification::new(
                    1,
                    BarAggregation::Minute,
                    PriceType::Last,
                ),
                AggregationSource::External,
            ),
            Price::from("100.00"),
            Price::from("102.00"),
            Price::from("99.00"),
            Price::from("101.00"),
            Quantity::from("10"),
            ts.into(),
            ts.into(),
        )
    }

    fn quote(ts: u64) -> QuoteTick {
        QuoteTick::new(
            instrument_id(),
            Price::from("100.00"),
            Price::from("100.50"),
            Quantity::from("10"),
            Quantity::from("10"),
            ts.into(),
            ts.into(),
        )
    }

    fn trade(ts: u64) -> TradeTick {
        TradeTick::new(
            instrument_id(),
            Price::from("100.25"),
            Quantity::from("5"),
            nautilus_model::enums::AggressorSide::Buy,
            TradeId::from("T-1"),
            ts.into(),
            ts.into(),
        )
    }

    fn adjustment(from: PriceRepresentation, to: PriceRepresentation) -> DataAdjustment {
        DataAdjustment::new(from, to)
    }

    #[rstest]
    fn test_active_adjustment_is_none_for_unset_and_noop() {
        assert_eq!(active_adjustment(None), None);
        assert_eq!(
            active_adjustment(Some(adjustment(
                PriceRepresentation::Raw,
                PriceRepresentation::Raw
            ))),
            None
        );
        assert_eq!(
            active_adjustment(Some(adjustment(
                PriceRepresentation::Raw,
                PriceRepresentation::Adjusted
            ))),
            Some(adjustment(
                PriceRepresentation::Raw,
                PriceRepresentation::Adjusted
            ))
        );
    }

    #[rstest]
    fn test_noop_raw_stage_passes_data_through_untouched() {
        let data = vec![
            Data::Bar(bar(EARLY_NS)),
            Data::Quote(quote(EFFECT_NS)),
            Data::Trade(trade(LATE_NS)),
        ];

        let result = apply_to_vec(
            data.clone(),
            &[],
            adjustment(PriceRepresentation::Raw, PriceRepresentation::Raw),
        )
        .unwrap();

        assert_eq!(result, data);
    }

    #[rstest]
    fn test_noop_stage_passes_data_through_untouched() {
        let data = vec![Data::Bar(bar(EARLY_NS)), Data::Quote(quote(LATE_NS))];
        let actions = vec![action(CorporateActionType::Split, "4", EFFECT_NS)];

        let result = apply_to_vec(
            data.clone(),
            &actions,
            adjustment(PriceRepresentation::Adjusted, PriceRepresentation::Adjusted),
        )
        .unwrap();

        // Even though actions exist, a no-op combination neither converts nor appends anything
        assert_eq!(result, data);
    }

    #[rstest]
    fn test_split_and_dividend_adjust_and_unadjust_a_pre_effect_bar() {
        let actions = vec![
            action(CorporateActionType::Split, "4", EFFECT_NS),
            action(CorporateActionType::Dividend, "0.10", EFFECT_NS),
        ];
        // A bar strictly before the effect instant
        let original = bar(EARLY_NS);

        let adjusted = apply_to_vec(
            vec![Data::Bar(original)],
            &actions,
            adjustment(PriceRepresentation::Raw, PriceRepresentation::Adjusted),
        )
        .unwrap();

        let Data::Bar(adjusted_bar) = &adjusted[0] else {
            panic!("expected a bar");
        };
        // adjusted = raw * (1 / 4) - 0.10
        assert_eq!(adjusted_bar.open, Price::from("24.90"));
        assert_eq!(adjusted_bar.high, Price::from("25.40"));
        assert_eq!(adjusted_bar.low, Price::from("24.65"));
        assert_eq!(adjusted_bar.close, Price::from("25.15"));
        // The volume and timestamps are preserved exactly
        assert_eq!(adjusted_bar.volume, original.volume);
        assert_eq!(adjusted_bar.ts_event, original.ts_event);
        assert_eq!(adjusted_bar.ts_init, original.ts_init);

        // Converting back recovers the original raw values exactly
        let raw = apply_to_vec(
            vec![adjusted[0].clone()],
            &actions,
            adjustment(PriceRepresentation::Adjusted, PriceRepresentation::Raw),
        )
        .unwrap();
        let Data::Bar(raw_bar) = &raw[0] else {
            panic!("expected a bar");
        };
        assert_eq!(raw_bar.open, original.open);
        assert_eq!(raw_bar.high, original.high);
        assert_eq!(raw_bar.low, original.low);
        assert_eq!(raw_bar.close, original.close);
    }

    #[rstest]
    fn test_post_effect_bar_is_unchanged() {
        let actions = vec![action(CorporateActionType::Split, "4", EFFECT_NS)];
        let original = bar(LATE_NS);

        let result = apply_to_vec(
            vec![Data::Bar(original)],
            &actions,
            adjustment(PriceRepresentation::Raw, PriceRepresentation::Adjusted),
        )
        .unwrap();

        let Data::Bar(result_bar) = &result[0] else {
            panic!("expected a bar");
        };
        assert_eq!(*result_bar, original);
    }

    #[rstest]
    fn test_actions_are_appended_at_their_effective_instants() {
        let actions = vec![
            action(CorporateActionType::Split, "4", EFFECT_NS),
            action(CorporateActionType::Dividend, "0.10", LATE_NS),
        ];

        let result = apply_to_vec(
            vec![Data::Quote(quote(EARLY_NS))],
            &actions,
            adjustment(PriceRepresentation::Raw, PriceRepresentation::Adjusted),
        )
        .unwrap();

        let delivered: Vec<&CorporateAction> = result
            .iter()
            .filter_map(|data| match data {
                Data::CorporateAction(action) => Some(action),
                _ => None,
            })
            .collect();

        assert_eq!(delivered.len(), 2);
        assert_eq!(delivered[0].ts_init, UnixNanos::from(EFFECT_NS));
        assert_eq!(delivered[1].ts_init, UnixNanos::from(LATE_NS));
        // Every field other than the replay key is preserved
        assert_eq!(delivered[0].effective_ns, UnixNanos::from(EFFECT_NS));
        assert_eq!(delivered[0].value, Decimal::from(4));
        assert_eq!(delivered[1].value, Decimal::from_str_exact("0.10").unwrap());
    }

    #[rstest]
    fn test_bar_precision_stays_uniform_after_conversion() {
        let actions = vec![action(CorporateActionType::Split, "4", EFFECT_NS)];
        let result = apply_to_vec(
            vec![Data::Bar(bar(EARLY_NS))],
            &actions,
            adjustment(PriceRepresentation::Raw, PriceRepresentation::Adjusted),
        )
        .unwrap();

        let Data::Bar(adjusted) = &result[0] else {
            panic!("expected a bar");
        };
        assert_eq!(adjusted.open.precision, adjusted.high.precision);
        assert_eq!(adjusted.open.precision, adjusted.low.precision);
        assert_eq!(adjusted.open.precision, adjusted.close.precision);
        assert!(adjusted.open.precision <= FIXED_PRECISION);
    }

    #[rstest]
    fn test_actions_replay_at_their_effective_instants_through_the_replay_key() {
        let actions = vec![action(CorporateActionType::Split, "4", EFFECT_NS)];
        let result = apply_to_vec(
            vec![Data::Quote(quote(EARLY_NS)), Data::Quote(quote(LATE_NS))],
            &actions,
            adjustment(PriceRepresentation::Raw, PriceRepresentation::Adjusted),
        )
        .unwrap();

        let mut iterator = BacktestDataIterator::new();
        iterator.add_data("adjusted", result, false);

        let mut order = Vec::new();
        while let Some(data) = iterator.peek() {
            let label = match data {
                DataRef::CorporateAction(_) => "action",
                DataRef::Quote(_) => "quote",
                _ => "other",
            };
            order.push((data.ts_init().as_u64(), label));
            iterator.advance();
        }

        assert_eq!(
            order,
            vec![
                (EARLY_NS, "quote"),
                (EFFECT_NS, "action"),
                (LATE_NS, "quote")
            ]
        );
    }
}
