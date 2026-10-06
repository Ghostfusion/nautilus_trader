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

//! Provides a configuration for `RiskEngine` instances.

use ahash::{AHashMap, AHashSet};
use nautilus_common::{
    config::{ConfigError, ConfigErrorCollector, ConfigResult},
    throttler::RateLimit,
};
use nautilus_core::DurationNanos;
use nautilus_model::{
    identifiers::{InstrumentId, Venue},
    risk::{RiskCapMetric, RiskCapScope},
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use super::cap::RiskCap;

/// Configuration for `RiskEngineConfig` instances.
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.risk", from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.risk")
)]
#[cfg_attr(
    feature = "python",
    expect(
        clippy::unsafe_derive_deserialize,
        reason = "config deserializes plain fields; unsafe methods come from generated PyO3 integration"
    )
)]
#[derive(Debug, Clone, Deserialize, Serialize, bon::Builder)]
#[builder(finish_fn(name = build_inner, vis = ""))]
#[serde(default, deny_unknown_fields)]
pub struct RiskEngineConfig {
    /// Whether to bypass risk checks and order rate limits.
    #[builder(default)]
    pub bypass: bool,
    /// Rate limit for order submission commands.
    #[builder(default = RateLimit::new(100, DurationNanos::from_secs(1)))]
    pub max_order_submit: RateLimit,
    /// Rate limit for order modifications, counting each batch child separately.
    #[builder(default = RateLimit::new(100, DurationNanos::from_secs(1)))]
    pub max_order_modify: RateLimit,
    /// Maximum notional per order by instrument, in each instrument's quote currency.
    #[builder(default)]
    pub max_notional_per_order: AHashMap<InstrumentId, Decimal>,
    /// Count caps, each a predicate over a scope, a metric and a window.
    ///
    /// A cap refuses the actions that increase exposure when the count it observes reaches its
    /// limit, and it never refuses a cancellation. Caps are evaluated in configuration order, so
    /// the first reached cap names the refusal.
    #[builder(default)]
    pub count_caps: Vec<RiskCap>,
    /// Venues whose execution clients enforce whole-position conditional exits.
    ///
    /// Validated exits skip bounds that apply only to their placeholder quantity and notional.
    #[builder(default)]
    pub full_position_exit_venues: AHashSet<Venue>,
    /// Venues that refuse a submission whose price is not a whole multiple of the instrument's
    /// price increment.
    ///
    /// A venue that aligns a price instead of refusing it cannot be expressed here. An order's
    /// price is fixed when it is built and no event can change it, so a venue that rounded would
    /// either book a price its submitter never sees or fill outside the submitted limit. A caller
    /// trading on such a venue rounds the price at construction, where the order is built.
    #[builder(default)]
    pub tick_alignment_venues: AHashSet<Venue>,
    /// Whether to emit additional debug logs.
    #[builder(default)]
    pub debug: bool,
}

impl<S: risk_engine_config_builder::IsComplete> RiskEngineConfigBuilder<S> {
    /// Validates and builds the [`RiskEngineConfig`].
    ///
    /// # Errors
    ///
    /// Returns a [`ConfigError`] if any field fails validation
    /// (see [`RiskEngineConfig::validate`]).
    pub fn build(self) -> ConfigResult<RiskEngineConfig> {
        let config = self.build_inner();
        config.validate()?;
        Ok(config)
    }
}

impl RiskEngineConfig {
    /// Validates the risk engine configuration, collecting every field violation.
    ///
    /// # Errors
    ///
    /// Returns a [`ConfigError`] (a [`ConfigError::Multiple`] when more than one field is
    /// invalid) if any field fails validation.
    pub fn validate(&self) -> ConfigResult<()> {
        let mut errors = ConfigErrorCollector::new();

        for (instrument_id, notional) in &self.max_notional_per_order {
            errors.check(
                *notional > Decimal::ZERO,
                ConfigError::range(
                    "max_notional_per_order",
                    format!("notional for {instrument_id} must be positive, was {notional}"),
                ),
            );
        }

        let mut seen_caps = AHashSet::new();

        for cap in &self.count_caps {
            validate_quantity_cap(cap, &mut errors);
            validate_money_cap(cap, &mut errors);

            errors.check(
                cap.limit > 0 || cap.measures_quantity() || cap.measures_money(),
                ConfigError::range(
                    "count_caps",
                    format!(
                        "the {} limit for {} must be positive, was {}",
                        cap.metric, cap.scope, cap.limit
                    ),
                ),
            );

            match (cap.metric, cap.window) {
                (
                    RiskCapMetric::Active | RiskCapMetric::Inventory | RiskCapMetric::NetExposure,
                    None,
                ) => {}
                (RiskCapMetric::Active, Some(window)) => errors.check(
                    false,
                    ConfigError::invalid_value(
                        "count_caps",
                        format!(
                            "an ACTIVE cap counts the open order set and takes no window, was {} ns",
                            window.as_u64()
                        ),
                    ),
                ),
                (RiskCapMetric::Inventory, Some(window)) => errors.check(
                    false,
                    ConfigError::invalid_value(
                        "count_caps",
                        format!(
                            "an INVENTORY cap reads the standing position and takes no window, was {} ns",
                            window.as_u64()
                        ),
                    ),
                ),
                (RiskCapMetric::NetExposure, Some(window)) => errors.check(
                    false,
                    ConfigError::invalid_value(
                        "count_caps",
                        format!(
                            "a NET_EXPOSURE cap reads the portfolio and takes no window, was {} ns",
                            window.as_u64()
                        ),
                    ),
                ),
                (metric, None) => errors.check(
                    false,
                    ConfigError::invalid_value(
                        "count_caps",
                        format!("a {metric} cap requires a window"),
                    ),
                ),
                (metric, Some(window)) => errors.check(
                    !window.is_zero(),
                    ConfigError::range(
                        "count_caps",
                        format!("the {metric} window for {} must be positive", cap.scope),
                    ),
                ),
            }

            errors.check(
                seen_caps.insert(cap),
                ConfigError::duplicate("count_caps", Some(format!("{}/{}", cap.scope, cap.metric))),
            );
        }

        errors.into_result()
    }
}

/// Validates the quantity-limit half of a cap, which is a pair with the metric it measures.
///
/// A metric measured in occurrences must not carry one and a metric measured in quantity must, so
/// a configuration cannot read a size as a number of events or leave a size cap without a size.
fn validate_quantity_cap(cap: &RiskCap, errors: &mut ConfigErrorCollector) {
    match cap.quantity_limit {
        Some(quantity_limit) => {
            errors.check(
                matches!(
                    cap.metric,
                    RiskCapMetric::Participation | RiskCapMetric::Inventory
                ),
                ConfigError::invalid_value(
                    "count_caps",
                    format!(
                        "the {} metric is measured in occurrences, not quantity",
                        cap.metric
                    ),
                ),
            );
            errors.check(
                quantity_limit > Decimal::ZERO,
                ConfigError::range(
                    "count_caps",
                    format!(
                        "the {} quantity limit for {} must be positive, was {quantity_limit}",
                        cap.metric, cap.scope
                    ),
                ),
            );
            errors.check(
                cap.limit == 0,
                ConfigError::invalid_value(
                    "count_caps",
                    format!(
                        "a {} cap measured in quantity carries no occurrence limit, was {}",
                        cap.metric, cap.limit
                    ),
                ),
            );
        }
        None => errors.check(
            !matches!(
                cap.metric,
                RiskCapMetric::Participation | RiskCapMetric::Inventory
            ),
            ConfigError::invalid_value(
                "count_caps",
                format!("the {} metric requires a quantity limit", cap.metric),
            ),
        ),
    }
}

/// Validates the money-limit half of a cap, which is a pair with the metric it measures.
///
/// A cap measured in money names the currency it is limited in, because an amount without a
/// currency cannot be compared against an exposure, and it is scoped to an aggregation the
/// portfolio can resolve, because the portfolio is what a money cap reads.
fn validate_money_cap(cap: &RiskCap, errors: &mut ConfigErrorCollector) {
    if let Some(money_limit) = cap.money_limit {
        errors.check(
            cap.metric == RiskCapMetric::NetExposure,
            ConfigError::invalid_value(
                "count_caps",
                format!(
                    "the {} metric is measured in occurrences, not money",
                    cap.metric
                ),
            ),
        );
        errors.check(
            cap.money_currency.is_some(),
            ConfigError::invalid_value(
                "count_caps",
                format!("a {} cap measured in money requires a currency", cap.metric),
            ),
        );
        errors.check(
            money_limit > Decimal::ZERO,
            ConfigError::range(
                "count_caps",
                format!(
                    "the {} money limit for {} must be positive, was {money_limit}",
                    cap.metric, cap.scope
                ),
            ),
        );
        errors.check(
            cap.limit == 0,
            ConfigError::invalid_value(
                "count_caps",
                format!(
                    "a {} cap measured in money carries no occurrence limit, was {}",
                    cap.metric, cap.limit
                ),
            ),
        );
        errors.check(
            cap.quantity_limit.is_none(),
            ConfigError::invalid_value(
                "count_caps",
                format!(
                    "a {} cap measured in money carries no quantity limit",
                    cap.metric
                ),
            ),
        );
    } else {
        errors.check(
            cap.metric != RiskCapMetric::NetExposure,
            ConfigError::invalid_value(
                "count_caps",
                format!("the {} metric requires a money limit", cap.metric),
            ),
        );
        errors.check(
            cap.money_currency.is_none(),
            ConfigError::invalid_value(
                "count_caps",
                format!("the {} metric does not take a currency", cap.metric),
            ),
        );
    }

    errors.check(
        cap.metric != RiskCapMetric::NetExposure
            || matches!(cap.scope, RiskCapScope::Global | RiskCapScope::Account),
        ConfigError::invalid_value(
            "count_caps",
            format!(
                "a {} cap aggregates the portfolio, so it is scoped to GLOBAL or ACCOUNT, was {}",
                cap.metric, cap.scope
            ),
        ),
    );
}

impl Default for RiskEngineConfig {
    fn default() -> Self {
        Self::builder()
            .build()
            .expect("default `RiskEngineConfig` should be valid")
    }
}

#[cfg(test)]
mod tests {
    use nautilus_model::{risk::RiskCapScope, types::Currency};
    use rstest::rstest;
    use rust_decimal_macros::dec;

    use super::*;

    #[rstest]
    fn test_money_caps_are_validated_as_a_pair_with_their_metric() {
        // A money limit below or at zero is refused.
        let zero = RiskEngineConfig::builder()
            .count_caps(vec![RiskCap::new_money(
                RiskCapMetric::NetExposure,
                RiskCapScope::Global,
                Decimal::ZERO,
                Currency::USD(),
            )])
            .build();
        assert!(matches!(zero, Err(ConfigError::Range { field, .. }) if field == "count_caps"));

        // An occurrence metric cannot carry one.
        let wrong_metric = RiskEngineConfig::builder()
            .count_caps(vec![RiskCap {
                money_limit: Some(dec!(1.0)),
                money_currency: Some(Currency::USD()),
                ..RiskCap::new(
                    RiskCapMetric::Submit,
                    RiskCapScope::Global,
                    0,
                    Some(DurationNanos::from_secs(60)),
                )
            }])
            .build();
        assert!(matches!(
            wrong_metric,
            Err(ConfigError::InvalidValue { .. })
        ));

        // A money metric must carry one.
        let missing = RiskEngineConfig::builder()
            .count_caps(vec![RiskCap::new(
                RiskCapMetric::NetExposure,
                RiskCapScope::Global,
                10,
                None,
            )])
            .build();
        assert!(matches!(missing, Err(ConfigError::InvalidValue { .. })));

        // A money limit without a currency cannot be compared against an exposure.
        let without_currency = RiskEngineConfig::builder()
            .count_caps(vec![RiskCap {
                money_limit: Some(dec!(1.0)),
                ..RiskCap::new(RiskCapMetric::NetExposure, RiskCapScope::Global, 0, None)
            }])
            .build();
        assert!(matches!(
            without_currency,
            Err(ConfigError::InvalidValue { .. })
        ));

        // A net exposure cap aggregates the portfolio, so it is scoped to an aggregation the
        // portfolio can resolve rather than to one strategy or instrument.
        let wrong_scope = RiskEngineConfig::builder()
            .count_caps(vec![RiskCap::new_money(
                RiskCapMetric::NetExposure,
                RiskCapScope::Instrument,
                dec!(1.0),
                Currency::USD(),
            )])
            .build();
        assert!(matches!(wrong_scope, Err(ConfigError::InvalidValue { .. })));

        // A net exposure cap reads the portfolio and takes no window.
        let windowed = RiskEngineConfig::builder()
            .count_caps(vec![RiskCap {
                window: Some(DurationNanos::from_secs(60)),
                ..RiskCap::new_money(
                    RiskCapMetric::NetExposure,
                    RiskCapScope::Global,
                    dec!(1.0),
                    Currency::USD(),
                )
            }])
            .build();
        assert!(matches!(windowed, Err(ConfigError::InvalidValue { .. })));

        // Both aggregations the portfolio resolves are accepted in the shape the measurement needs.
        for scope in [RiskCapScope::Global, RiskCapScope::Account] {
            let accepted = RiskEngineConfig::builder()
                .count_caps(vec![RiskCap::new_money(
                    RiskCapMetric::NetExposure,
                    scope,
                    dec!(1000.0),
                    Currency::USD(),
                )])
                .build();
            assert!(accepted.is_ok(), "{scope} should be accepted");
        }
    }

    #[rstest]
    fn test_quantity_caps_are_validated_as_a_pair_with_their_metric() {
        // A quantity limit below or at zero is refused.
        let zero = RiskEngineConfig::builder()
            .count_caps(vec![RiskCap::new_quantity(
                RiskCapMetric::Participation,
                RiskCapScope::Instrument,
                Decimal::ZERO,
                Some(DurationNanos::from_secs(60)),
            )])
            .build();
        assert!(matches!(zero, Err(ConfigError::Range { field, .. }) if field == "count_caps"));

        // An occurrence metric cannot carry one.
        let wrong_metric = RiskEngineConfig::builder()
            .count_caps(vec![RiskCap {
                quantity_limit: Some(dec!(1.0)),
                ..RiskCap::new(
                    RiskCapMetric::Submit,
                    RiskCapScope::Instrument,
                    0,
                    Some(DurationNanos::from_secs(60)),
                )
            }])
            .build();
        assert!(matches!(
            wrong_metric,
            Err(ConfigError::InvalidValue { .. })
        ));

        // A quantity metric must carry one.
        let missing = RiskEngineConfig::builder()
            .count_caps(vec![RiskCap::new(
                RiskCapMetric::Inventory,
                RiskCapScope::Instrument,
                10,
                None,
            )])
            .build();
        assert!(matches!(missing, Err(ConfigError::InvalidValue { .. })));

        // An inventory cap reads the standing position and takes no window.
        let windowed = RiskEngineConfig::builder()
            .count_caps(vec![RiskCap::new_quantity(
                RiskCapMetric::Inventory,
                RiskCapScope::Instrument,
                dec!(1.0),
                Some(DurationNanos::from_secs(60)),
            )])
            .build();
        assert!(matches!(windowed, Err(ConfigError::InvalidValue { .. })));

        // Both quantity metrics are accepted in the shape their measurement needs.
        let participation = RiskEngineConfig::builder()
            .count_caps(vec![RiskCap::new_quantity(
                RiskCapMetric::Participation,
                RiskCapScope::Instrument,
                dec!(1.0),
                Some(DurationNanos::from_secs(60)),
            )])
            .build();
        assert!(participation.is_ok());

        let inventory = RiskEngineConfig::builder()
            .count_caps(vec![RiskCap::new_quantity(
                RiskCapMetric::Inventory,
                RiskCapScope::Instrument,
                dec!(1.0),
                None,
            )])
            .build();
        assert!(inventory.is_ok());
    }

    #[rstest]
    fn test_default_config_is_valid() {
        let config = RiskEngineConfig::builder().build().unwrap();

        assert!(config.full_position_exit_venues.is_empty());
    }

    #[rstest]
    #[case(Decimal::ZERO)]
    #[case(Decimal::from(-1))]
    fn test_non_positive_notional_rejected(#[case] notional: Decimal) {
        let mut notionals = AHashMap::new();
        notionals.insert(InstrumentId::from("ESZ21.GLBX"), notional);
        let result = RiskEngineConfig::builder()
            .max_notional_per_order(notionals)
            .build();
        assert!(
            matches!(result, Err(ConfigError::Range { field, .. }) if field == "max_notional_per_order")
        );
    }

    #[rstest]
    fn test_positive_notional_accepted() {
        let mut notionals = AHashMap::new();
        notionals.insert(InstrumentId::from("ESZ21.GLBX"), Decimal::from(1_000_000));
        let result = RiskEngineConfig::builder()
            .max_notional_per_order(notionals)
            .build();
        assert!(result.is_ok());
    }

    #[rstest]
    fn test_multiple_violations_collected() {
        let mut notionals = AHashMap::new();
        notionals.insert(InstrumentId::from("ESZ21.GLBX"), Decimal::ZERO);
        notionals.insert(InstrumentId::from("CLZ21.NYMEX"), Decimal::from(-1));
        let result = RiskEngineConfig::builder()
            .max_notional_per_order(notionals)
            .build();
        let ConfigError::Multiple { errors } = result.unwrap_err() else {
            panic!("expected ConfigError::Multiple");
        };
        assert_eq!(errors.len(), 2);
        assert!(errors.iter().all(
            |e| matches!(e, ConfigError::Range { field, .. } if field == "max_notional_per_order")
        ));
    }

    #[rstest]
    fn test_caps_default_to_empty() {
        assert!(RiskEngineConfig::default().count_caps.is_empty());
    }

    #[rstest]
    fn test_a_count_cap_is_accepted() {
        let result = RiskEngineConfig::builder()
            .count_caps(vec![RiskCap::new(
                RiskCapMetric::Submit,
                RiskCapScope::Instrument,
                10,
                Some(DurationNanos::from_secs(60)),
            )])
            .build();

        assert!(result.is_ok());
    }

    #[rstest]
    fn test_a_zero_limit_is_rejected() {
        let result = RiskEngineConfig::builder()
            .count_caps(vec![RiskCap::new(
                RiskCapMetric::Submit,
                RiskCapScope::Global,
                0,
                Some(DurationNanos::from_secs(60)),
            )])
            .build();

        assert!(matches!(result, Err(ConfigError::Range { field, .. }) if field == "count_caps"));
    }

    #[rstest]
    fn test_a_repeated_request_cap_is_accepted() {
        let result = RiskEngineConfig::builder()
            .count_caps(vec![RiskCap::new(
                RiskCapMetric::RepeatedRequest,
                RiskCapScope::Global,
                10,
                Some(DurationNanos::from_secs(60)),
            )])
            .build();

        assert!(result.is_ok());
    }

    #[rstest]
    fn test_a_repeated_request_cap_requires_a_window() {
        let result = RiskEngineConfig::builder()
            .count_caps(vec![RiskCap::new(
                RiskCapMetric::RepeatedRequest,
                RiskCapScope::Global,
                10,
                None,
            )])
            .build();

        assert!(
            matches!(result, Err(ConfigError::InvalidValue { field, .. }) if field == "count_caps")
        );
    }

    #[rstest]
    fn test_an_active_cap_takes_no_window() {
        let result = RiskEngineConfig::builder()
            .count_caps(vec![RiskCap::new(
                RiskCapMetric::Active,
                RiskCapScope::Global,
                50,
                Some(DurationNanos::from_secs(60)),
            )])
            .build();

        assert!(
            matches!(result, Err(ConfigError::InvalidValue { field, .. }) if field == "count_caps")
        );
    }

    #[rstest]
    fn test_a_count_cap_requires_a_window() {
        let result = RiskEngineConfig::builder()
            .count_caps(vec![RiskCap::new(
                RiskCapMetric::Submit,
                RiskCapScope::Global,
                10,
                None,
            )])
            .build();

        assert!(
            matches!(result, Err(ConfigError::InvalidValue { field, .. }) if field == "count_caps")
        );
    }

    #[rstest]
    fn test_a_zero_window_is_rejected() {
        let result = RiskEngineConfig::builder()
            .count_caps(vec![RiskCap::new(
                RiskCapMetric::Fill,
                RiskCapScope::Instrument,
                10,
                Some(DurationNanos::ZERO),
            )])
            .build();

        assert!(matches!(result, Err(ConfigError::Range { field, .. }) if field == "count_caps"));
    }

    #[rstest]
    fn test_duplicate_caps_are_rejected() {
        let cap = RiskCap::new(
            RiskCapMetric::Submit,
            RiskCapScope::Global,
            10,
            Some(DurationNanos::from_secs(60)),
        );
        let result = RiskEngineConfig::builder()
            .count_caps(vec![cap.clone(), cap])
            .build();

        assert!(
            matches!(result, Err(ConfigError::Duplicate { field, .. }) if field == "count_caps")
        );
    }
}
