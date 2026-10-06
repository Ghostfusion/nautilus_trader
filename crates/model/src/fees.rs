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

//! Account-owned fee schedules decoupled from instrument definitions.
//!
//! Fee policy belongs to the execution runtime serving an account, not to the
//! instrument. Two accounts can trade the same instrument with different fees
//! without constructing different instrument definitions.

use ahash::AHashMap;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::{
    enums::LiquiditySide,
    identifiers::InstrumentId,
    instruments::{Instrument, InstrumentAny},
    types::{Money, Price, Quantity},
};

/// Explicit maker/taker fee rates as a fraction of notional value.
///
/// Rates are expressed as decimals (for example `0.001` is 0.1%). Negative
/// maker rates represent rebates where the venue supports them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct MakerTakerFeeRates {
    /// The maker fee rate.
    pub maker: Decimal,
    /// The taker fee rate.
    pub taker: Decimal,
}

impl MakerTakerFeeRates {
    /// Creates new [`MakerTakerFeeRates`] with explicit rates.
    #[must_use]
    pub const fn new(maker: Decimal, taker: Decimal) -> Self {
        Self { maker, taker }
    }

    /// Returns an explicit zero-fee rate pair.
    #[must_use]
    pub const fn zero() -> Self {
        Self {
            maker: Decimal::ZERO,
            taker: Decimal::ZERO,
        }
    }

    /// Returns the rate for the given liquidity side.
    ///
    /// # Errors
    ///
    /// Returns an error if `liquidity_side` is [`LiquiditySide::NoLiquiditySide`].
    pub fn rate_for(&self, liquidity_side: LiquiditySide) -> anyhow::Result<Decimal> {
        match liquidity_side {
            LiquiditySide::Maker => Ok(self.maker),
            LiquiditySide::Taker => Ok(self.taker),
            LiquiditySide::NoLiquiditySide => {
                anyhow::bail!("Invalid `LiquiditySide`: {liquidity_side}")
            }
        }
    }
}

/// Account-owned maker/taker fee schedule with exact instrument overrides.
///
/// Resolution is deterministic: for a given cumulative volume the highest volume tier at or below
/// it wins, then an exact [`InstrumentId`] override, then the default. Absent configuration is
/// represented by the absence of a schedule (`None` at the owner), which is distinct from an
/// explicit zero rate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MakerTakerFeeSchedule {
    /// The default rates applied when no override matches.
    pub default: MakerTakerFeeRates,
    /// Exact per-instrument rate overrides.
    pub overrides: AHashMap<InstrumentId, MakerTakerFeeRates>,
    /// Per-instrument volume tiers, strictly ascending by `min_volume`.
    #[serde(default)]
    pub volume_tiers: AHashMap<InstrumentId, Vec<VolumeTier>>,
}

/// A volume tier: the maker/taker rates that apply from a cumulative volume upward.
///
/// Tiers are per instrument and resolved against a cumulative volume, so a schedule can express
/// the rebate ladders venues publish instead of one flat rate per instrument.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct VolumeTier {
    /// The cumulative volume at which the tier's rates begin, inclusive.
    pub min_volume: Quantity,
    /// The rates that apply at or above `min_volume`.
    pub rates: MakerTakerFeeRates,
}

impl VolumeTier {
    /// Creates a new [`VolumeTier`].
    #[must_use]
    pub const fn new(min_volume: Quantity, maker: Decimal, taker: Decimal) -> Self {
        Self {
            min_volume,
            rates: MakerTakerFeeRates::new(maker, taker),
        }
    }
}

impl MakerTakerFeeSchedule {
    /// Creates a new schedule with explicit default rates and no overrides.
    #[must_use]
    pub fn new(maker: Decimal, taker: Decimal) -> Self {
        Self {
            default: MakerTakerFeeRates::new(maker, taker),
            overrides: AHashMap::new(),
            volume_tiers: AHashMap::new(),
        }
    }

    /// Creates a new explicit zero-fee schedule.
    #[must_use]
    pub fn zero() -> Self {
        Self {
            default: MakerTakerFeeRates::zero(),
            overrides: AHashMap::new(),
            volume_tiers: AHashMap::new(),
        }
    }

    /// Adds or replaces an exact instrument override.
    pub fn set_override(&mut self, instrument_id: InstrumentId, rates: MakerTakerFeeRates) {
        self.overrides.insert(instrument_id, rates);
    }

    /// Sets the volume tiers for an instrument.
    ///
    /// # Errors
    ///
    /// Returns an error if `tiers` is empty, if any tier begins at a zero volume, or if the tiers
    /// are not strictly ascending by `min_volume`. Resolution picks the highest tier at or below
    /// the volume, so an unordered list would resolve by position rather than by volume.
    pub fn set_volume_tiers(
        &mut self,
        instrument_id: InstrumentId,
        tiers: Vec<VolumeTier>,
    ) -> anyhow::Result<()> {
        if tiers.is_empty() {
            anyhow::bail!("Volume tiers must not be empty");
        }

        let mut previous: Option<Quantity> = None;

        for tier in &tiers {
            if tier.min_volume.is_zero() {
                anyhow::bail!("Volume tier min_volume must be greater than zero");
            }

            if previous.is_some_and(|previous| tier.min_volume <= previous) {
                anyhow::bail!("Volume tiers must be strictly ascending by min_volume");
            }

            previous = Some(tier.min_volume);
        }

        self.volume_tiers.insert(instrument_id, tiers);
        Ok(())
    }

    /// Returns the volume tiers configured for an instrument, if any.
    #[must_use]
    pub fn volume_tiers_for(&self, instrument_id: InstrumentId) -> Option<&[VolumeTier]> {
        self.volume_tiers.get(&instrument_id).map(Vec::as_slice)
    }

    /// Returns the resolved rates for the given instrument and cumulative volume.
    ///
    /// The highest tier whose `min_volume` is at or below `volume` wins. An instrument without
    /// tiers, or a volume below the first tier, resolves through the exact instrument override and
    /// then the default, so a ladder that starts above zero never leaves a trade unpriced.
    #[must_use]
    pub fn rates_for_volume(
        &self,
        instrument_id: InstrumentId,
        volume: Quantity,
    ) -> MakerTakerFeeRates {
        if let Some(tier) = self
            .volume_tiers
            .get(&instrument_id)
            .and_then(|tiers| tiers.iter().rev().find(|tier| volume >= tier.min_volume))
        {
            return tier.rates;
        }

        self.rates_for(instrument_id)
    }

    /// Returns the resolved rate for the given instrument, cumulative volume and liquidity side.
    ///
    /// # Errors
    ///
    /// Returns an error if `liquidity_side` is [`LiquiditySide::NoLiquiditySide`].
    pub fn rate_for_volume(
        &self,
        instrument_id: InstrumentId,
        volume: Quantity,
        liquidity_side: LiquiditySide,
    ) -> anyhow::Result<Decimal> {
        self.rates_for_volume(instrument_id, volume)
            .rate_for(liquidity_side)
    }

    /// Returns the resolved rates for the given instrument.
    #[must_use]
    pub fn rates_for(&self, instrument_id: InstrumentId) -> MakerTakerFeeRates {
        self.overrides
            .get(&instrument_id)
            .copied()
            .unwrap_or(self.default)
    }

    /// Returns the resolved rate for the given instrument and liquidity side.
    ///
    /// # Errors
    ///
    /// Returns an error if `liquidity_side` is [`LiquiditySide::NoLiquiditySide`].
    pub fn rate_for(
        &self,
        instrument_id: InstrumentId,
        liquidity_side: LiquiditySide,
    ) -> anyhow::Result<Decimal> {
        self.rates_for(instrument_id).rate_for(liquidity_side)
    }
}

/// Calculates maker/taker commission from an explicitly resolved fee rate.
///
/// This is the single shared arithmetic for notional-based maker/taker fees used
/// by both account calculations and execution fee models. Contract terms (notional
/// and currency) come from the instrument; fee policy (the rate) comes from the
/// account-owned schedule and must already be resolved by the caller.
///
/// # Errors
///
/// Returns an error if the notional value cannot be calculated, arithmetic
/// overflows, or the commission cannot be represented in the notional currency.
pub fn calculate_maker_taker_commission(
    instrument: &InstrumentAny,
    last_qty: Quantity,
    last_px: Price,
    fee_rate: Decimal,
    use_quote_for_inverse: Option<bool>,
) -> anyhow::Result<Money> {
    let notional =
        instrument.try_calculate_notional_value(last_qty, last_px, use_quote_for_inverse)?;
    let commission = notional
        .as_decimal()
        .checked_mul(fee_rate)
        .ok_or_else(|| anyhow::anyhow!("commission calculation overflow"))?;
    Money::from_decimal(commission, notional.currency).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use rstest::rstest;
    use rust_decimal_macros::dec;

    use super::*;
    use crate::enums::LiquiditySide;

    #[rstest]
    fn test_rates_for_liquidity_side() {
        let rates = MakerTakerFeeRates::new(dec!(0.0002), dec!(0.0005));
        assert_eq!(rates.rate_for(LiquiditySide::Maker).unwrap(), dec!(0.0002));
        assert_eq!(rates.rate_for(LiquiditySide::Taker).unwrap(), dec!(0.0005));
        assert!(rates.rate_for(LiquiditySide::NoLiquiditySide).is_err());
    }

    #[rstest]
    fn test_schedule_override_wins_over_default() {
        let mut schedule = MakerTakerFeeSchedule::new(dec!(0.001), dec!(0.002));
        let instrument_id = InstrumentId::from("BTCUSDT.BINANCE");
        let other_id = InstrumentId::from("ETHUSDT.BINANCE");
        schedule.set_override(
            instrument_id,
            MakerTakerFeeRates::new(dec!(0.0001), dec!(0.0002)),
        );

        assert_eq!(
            schedule.rates_for(instrument_id),
            MakerTakerFeeRates::new(dec!(0.0001), dec!(0.0002))
        );
        assert_eq!(
            schedule.rates_for(other_id),
            MakerTakerFeeRates::new(dec!(0.001), dec!(0.002))
        );
    }

    #[rstest]
    fn test_explicit_zero_distinguishable_from_default() {
        let mut schedule = MakerTakerFeeSchedule::new(dec!(0.001), dec!(0.002));
        let instrument_id = InstrumentId::from("BTCUSDT.BINANCE");
        schedule.set_override(instrument_id, MakerTakerFeeRates::zero());

        assert_eq!(
            schedule
                .rate_for(instrument_id, LiquiditySide::Taker)
                .unwrap(),
            Decimal::ZERO
        );
        assert_eq!(
            schedule
                .rate_for(InstrumentId::from("ETHUSDT.BINANCE"), LiquiditySide::Taker)
                .unwrap(),
            dec!(0.002)
        );
    }

    #[rstest]
    fn test_commission_matches_notional_arithmetic() {
        use crate::{
            instruments::{Instrument, stubs::audusd_sim},
            types::{Price, Quantity},
        };

        let instrument = InstrumentAny::CurrencyPair(audusd_sim());
        let commission = calculate_maker_taker_commission(
            &instrument,
            Quantity::from(100_000),
            Price::from("1.0"),
            dec!(0.0002),
            None,
        )
        .unwrap();
        let notional = instrument
            .try_calculate_notional_value(Quantity::from(100_000), Price::from("1.0"), None)
            .unwrap();
        assert_eq!(
            commission.as_decimal(),
            notional.as_decimal() * dec!(0.0002)
        );
        assert_eq!(commission.currency, notional.currency);
    }

    #[rstest]
    fn test_volume_tiers_resolve_the_highest_tier_at_or_below_the_volume() {
        use crate::{identifiers::InstrumentId, types::Quantity};

        let instrument_id = InstrumentId::from("AUD/USD.SIM");
        let mut schedule = MakerTakerFeeSchedule::new(dec!(0.005), dec!(0.006));
        schedule
            .set_volume_tiers(
                instrument_id,
                vec![
                    VolumeTier::new(Quantity::from(1_000), dec!(0.001), dec!(0.002)),
                    VolumeTier::new(Quantity::from(10_000), dec!(0.0005), dec!(0.001)),
                    VolumeTier::new(Quantity::from(100_000), dec!(-0.0001), dec!(0.0005)),
                ],
            )
            .unwrap();

        // Below the first tier the default still prices the trade.
        assert_eq!(
            schedule.rates_for_volume(instrument_id, Quantity::from(999)),
            MakerTakerFeeRates::new(dec!(0.005), dec!(0.006)),
        );
        // The tier boundary is inclusive.
        assert_eq!(
            schedule.rates_for_volume(instrument_id, Quantity::from(1_000)),
            MakerTakerFeeRates::new(dec!(0.001), dec!(0.002)),
        );
        assert_eq!(
            schedule
                .rates_for_volume(instrument_id, Quantity::from(99_999))
                .taker,
            dec!(0.001),
        );
        // The top tier carries a maker rebate.
        assert_eq!(
            schedule.rates_for_volume(instrument_id, Quantity::from(100_000)),
            MakerTakerFeeRates::new(dec!(-0.0001), dec!(0.0005)),
        );
        assert_eq!(
            schedule
                .rate_for_volume(instrument_id, Quantity::from(100_000), LiquiditySide::Maker)
                .unwrap(),
            dec!(-0.0001),
        );
        assert!(
            schedule
                .rate_for_volume(
                    instrument_id,
                    Quantity::from(0),
                    LiquiditySide::NoLiquiditySide
                )
                .is_err()
        );
    }

    #[rstest]
    fn test_volume_tiers_must_be_ascending_and_non_zero() {
        use crate::{identifiers::InstrumentId, types::Quantity};

        let instrument_id = InstrumentId::from("AUD/USD.SIM");
        let mut schedule = MakerTakerFeeSchedule::zero();

        assert!(schedule.set_volume_tiers(instrument_id, vec![]).is_err());
        assert!(
            schedule
                .set_volume_tiers(
                    instrument_id,
                    vec![VolumeTier::new(Quantity::zero(0), dec!(0.001), dec!(0.002))],
                )
                .is_err()
        );
        assert!(
            schedule
                .set_volume_tiers(
                    instrument_id,
                    vec![
                        VolumeTier::new(Quantity::from(100), dec!(0.001), dec!(0.002)),
                        VolumeTier::new(Quantity::from(100), dec!(0.0005), dec!(0.001)),
                    ],
                )
                .is_err()
        );
        assert!(
            schedule
                .set_volume_tiers(
                    instrument_id,
                    vec![
                        VolumeTier::new(Quantity::from(200), dec!(0.001), dec!(0.002)),
                        VolumeTier::new(Quantity::from(100), dec!(0.0005), dec!(0.001)),
                    ],
                )
                .is_err()
        );
        // A refused ladder is not stored.
        assert!(schedule.volume_tiers_for(instrument_id).is_none());
    }
}
