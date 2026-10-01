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
    risk::RiskCapMetric,
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
            errors.check(
                cap.limit > 0,
                ConfigError::range(
                    "count_caps",
                    format!(
                        "the {} limit for {} must be positive, was {}",
                        cap.metric, cap.scope, cap.limit
                    ),
                ),
            );

            match (cap.metric, cap.window) {
                (RiskCapMetric::Active, None) => {}
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

impl Default for RiskEngineConfig {
    fn default() -> Self {
        Self::builder()
            .build()
            .expect("default `RiskEngineConfig` should be valid")
    }
}

#[cfg(test)]
mod tests {
    use nautilus_model::risk::RiskCapScope;
    use rstest::rstest;

    use super::*;

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
