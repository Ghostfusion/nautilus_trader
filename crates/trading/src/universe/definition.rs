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

//! Universe definitions: what is eligible, and what a member holds.
//!
//! A definition is immutable input to a run. It carries the rule, the venue the universe is
//! defined over, the subscriptions each member holds, the optional selection interval, and the
//! removal policy. Nothing here is stateful: membership lives in the component.

use std::fmt::Debug;

use anyhow::{Result, ensure};
use nautilus_core::DurationNanos;
use nautilus_model::{
    data::{BarSpecification, BarType},
    enums::{AggregationSource, BarAggregation, PriceType},
    identifiers::{InstrumentId, Venue},
};
use ustr::Ustr;

use super::{membership::validate_universe_name, rule::SharedUniverseRule};

/// What a member subscribes to.
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, strum::Display, strum::AsRefStr,
)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(
        frozen,
        eq,
        eq_int,
        module = "nautilus_trader.trading",
        from_py_object,
        rename_all = "SCREAMING_SNAKE_CASE",
    )
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass_enum(module = "nautilus_trader.trading")
)]
pub enum UniverseSubscription {
    /// The instrument definition for the venue.
    Instrument,
    /// Instrument status updates.
    InstrumentStatus,
    /// Quoted prices.
    Quotes,
    /// Trade ticks.
    Trades,
    /// Bars of the configured specification.
    Bars,
}

impl UniverseSubscription {
    /// Returns the canonical string representation.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.as_ref()
    }
}

/// The bar specification a universe subscribes to for each member.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UniverseBarSpec {
    /// The bar specification.
    pub spec: BarSpecification,
    /// The source the bars are aggregated by.
    pub aggregation_source: AggregationSource,
}

impl UniverseBarSpec {
    /// Creates a new [`UniverseBarSpec`] with external aggregation.
    ///
    /// External bars are the ones a venue or a data catalog publishes, which is what a universe
    /// subscribes to by default.
    ///
    /// # Errors
    ///
    /// Returns an error if the specification is invalid.
    pub fn new(step: usize, aggregation: BarAggregation, price_type: PriceType) -> Result<Self> {
        Ok(Self {
            spec: BarSpecification::new_checked(step, aggregation, price_type)?,
            aggregation_source: AggregationSource::External,
        })
    }

    /// Returns a copy with the given aggregation source.
    #[must_use]
    pub const fn with_aggregation_source(mut self, aggregation_source: AggregationSource) -> Self {
        self.aggregation_source = aggregation_source;
        self
    }

    /// Returns a copy with internal aggregation.
    #[must_use]
    pub const fn internal(mut self) -> Self {
        self.aggregation_source = AggregationSource::Internal;
        self
    }

    /// Returns the bar type for an `instrument_id`.
    #[must_use]
    pub fn bar_type(&self, instrument_id: InstrumentId) -> BarType {
        BarType::new(instrument_id, self.spec, self.aggregation_source)
    }
}

/// The condition a member must satisfy before its removal completes.
///
/// The universe never submits or cancels orders. Before releasing the subscriptions of a departing
/// instrument it reports the open orders and positions it can see, and the component that owns the
/// orders decides what to do about them.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, strum::Display, strum::AsRefStr, strum::EnumString,
)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(
        frozen,
        eq,
        eq_int,
        module = "nautilus_trader.trading",
        from_py_object,
        rename_all = "SCREAMING_SNAKE_CASE",
    )
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass_enum(module = "nautilus_trader.trading")
)]
pub enum UniverseRemovalPolicy {
    /// Removal completes once the member has no open orders and no open position.
    #[default]
    RequireFlat,
    /// Removal completes regardless of open orders and open position.
    ReleaseRegardless,
}

impl UniverseRemovalPolicy {
    /// Returns the canonical string representation.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.as_ref()
    }
}

/// The rule and settings that describe which instruments are eligible.
///
/// Cloning a definition copies its settings and shares the selection rule.
#[derive(Clone)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.trading", unsendable, from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.trading")
)]
pub struct UniverseDefinition {
    name: Ustr,
    venue: Venue,
    rule: SharedUniverseRule,
    subscriptions: Vec<UniverseSubscription>,
    bar_spec: UniverseBarSpec,
    selection_interval_ns: Option<DurationNanos>,
    removal_policy: UniverseRemovalPolicy,
}

impl UniverseDefinition {
    /// Creates a new [`UniverseDefinition`].
    ///
    /// The default member subscriptions are the instrument definition, quotes, and trades, with no
    /// selection interval and [`UniverseRemovalPolicy::RequireFlat`].
    ///
    /// # Errors
    ///
    /// Returns an error if the name is not a valid universe name.
    pub fn new(name: &str, venue: Venue, rule: SharedUniverseRule) -> Result<Self> {
        Ok(Self {
            name: validate_universe_name(name)?,
            venue,
            rule,
            subscriptions: vec![
                UniverseSubscription::Instrument,
                UniverseSubscription::Quotes,
                UniverseSubscription::Trades,
            ],
            bar_spec: UniverseBarSpec::new(1, BarAggregation::Minute, PriceType::Last)?,
            selection_interval_ns: None,
            removal_policy: UniverseRemovalPolicy::default(),
        })
    }

    /// Returns the universe name.
    ///
    /// The name identifies the component, its membership topic, and its selection timer.
    #[must_use]
    pub fn name(&self) -> Ustr {
        self.name
    }

    /// Returns the venue the universe is defined over.
    #[must_use]
    pub fn venue(&self) -> Venue {
        self.venue
    }

    /// Returns the selection rule.
    #[must_use]
    pub fn rule(&self) -> &SharedUniverseRule {
        &self.rule
    }

    /// Returns the rule name.
    #[must_use]
    pub fn rule_name(&self) -> String {
        self.rule.borrow().name().to_string()
    }

    /// Returns the subscriptions a member holds.
    #[must_use]
    pub fn subscriptions(&self) -> &[UniverseSubscription] {
        &self.subscriptions
    }

    /// Returns the bar specification used by [`UniverseSubscription::Bars`].
    #[must_use]
    pub fn bar_spec(&self) -> UniverseBarSpec {
        self.bar_spec
    }

    /// Returns the selection interval, if selection is periodic.
    #[must_use]
    pub fn selection_interval_ns(&self) -> Option<DurationNanos> {
        self.selection_interval_ns
    }

    /// Returns the removal policy.
    #[must_use]
    pub fn removal_policy(&self) -> UniverseRemovalPolicy {
        self.removal_policy
    }

    /// Returns a definition with the given member subscriptions.
    ///
    /// Duplicates are removed, so a repeated subscription is requested once.
    #[must_use]
    pub fn with_subscriptions(mut self, subscriptions: Vec<UniverseSubscription>) -> Self {
        self.subscriptions = subscriptions;
        self.subscriptions.sort_unstable();
        self.subscriptions.dedup();
        self
    }

    /// Returns a definition with `subscription` added to the member subscriptions.
    #[must_use]
    pub fn with_subscription(mut self, subscription: UniverseSubscription) -> Self {
        if !self.subscriptions.contains(&subscription) {
            self.subscriptions.push(subscription);
        }
        self
    }

    /// Returns a definition with the given bar specification.
    #[must_use]
    pub fn with_bar_spec(mut self, bar_spec: UniverseBarSpec) -> Self {
        self.bar_spec = bar_spec;
        self
    }

    /// Returns a definition that selects on `interval_ns`.
    ///
    /// The interval is a clock timer, so no wall clock is read and the cadence is deterministic in
    /// a backtest.
    ///
    /// # Errors
    ///
    /// Returns an error if the interval is zero.
    pub fn with_selection_interval(mut self, interval_ns: DurationNanos) -> Result<Self> {
        ensure!(
            !interval_ns.is_zero(),
            "Universe selection interval must be greater than zero"
        );
        self.selection_interval_ns = Some(interval_ns);
        Ok(self)
    }

    /// Returns a definition with the given removal policy.
    #[must_use]
    pub fn with_removal_policy(mut self, removal_policy: UniverseRemovalPolicy) -> Self {
        self.removal_policy = removal_policy;
        self
    }
}

impl Debug for UniverseDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(UniverseDefinition))
            .field("name", &self.name)
            .field("venue", &self.venue)
            .field("rule", &self.rule_name())
            .field("subscriptions", &self.subscriptions)
            .field("bar_spec", &self.bar_spec)
            .field("selection_interval_ns", &self.selection_interval_ns)
            .field("removal_policy", &self.removal_policy)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use rstest::rstest;

    use super::*;
    use crate::universe::StaticUniverseRule;

    fn rule() -> SharedUniverseRule {
        Rc::new(RefCell::new(
            StaticUniverseRule::new("static", Vec::new()).unwrap(),
        ))
    }

    fn definition() -> UniverseDefinition {
        UniverseDefinition::new("equities", Venue::from("XNYS"), rule()).unwrap()
    }

    #[rstest]
    fn test_definition_defaults() {
        let definition = definition();

        assert_eq!(definition.name(), Ustr::from("equities"));
        assert_eq!(definition.venue(), Venue::from("XNYS"));
        assert_eq!(definition.rule_name(), "static");
        assert_eq!(
            definition.subscriptions(),
            [
                UniverseSubscription::Instrument,
                UniverseSubscription::Quotes,
                UniverseSubscription::Trades,
            ]
        );
        assert_eq!(definition.selection_interval_ns(), None);
        assert_eq!(
            definition.removal_policy(),
            UniverseRemovalPolicy::RequireFlat
        );
        assert_eq!(
            definition
                .bar_spec()
                .bar_type(InstrumentId::from("AAPL.XNYS")),
            BarType::new(
                InstrumentId::from("AAPL.XNYS"),
                BarSpecification::new(1, BarAggregation::Minute, PriceType::Last),
                AggregationSource::External,
            )
        );
    }

    #[rstest]
    fn test_definition_rejects_invalid_name() {
        assert!(UniverseDefinition::new("equities.xnys", Venue::from("XNYS"), rule()).is_err());
    }

    #[rstest]
    fn test_definition_with_subscriptions_deduplicates() {
        let definition = definition().with_subscriptions(vec![
            UniverseSubscription::Trades,
            UniverseSubscription::Trades,
            UniverseSubscription::Bars,
        ]);

        assert_eq!(
            definition.subscriptions(),
            [UniverseSubscription::Trades, UniverseSubscription::Bars]
        );

        let definition = definition.with_subscription(UniverseSubscription::Bars);
        assert_eq!(
            definition.subscriptions(),
            [UniverseSubscription::Trades, UniverseSubscription::Bars]
        );
    }

    #[rstest]
    fn test_definition_rejects_zero_selection_interval() {
        assert!(
            definition()
                .with_selection_interval(DurationNanos::new(0))
                .is_err()
        );

        let definition = definition()
            .with_selection_interval(DurationNanos::from_mins(1))
            .unwrap();
        assert_eq!(
            definition.selection_interval_ns(),
            Some(DurationNanos::from_mins(1))
        );
    }

    #[rstest]
    fn test_definition_bar_spec_rejects_invalid_specification() {
        assert!(UniverseBarSpec::new(0, BarAggregation::Minute, PriceType::Last).is_err());

        let bar_spec = UniverseBarSpec::new(5, BarAggregation::Minute, PriceType::Bid)
            .unwrap()
            .internal();
        assert_eq!(bar_spec.aggregation_source, AggregationSource::Internal);
        assert_eq!(bar_spec.spec.step.get(), 5);
    }

    #[rstest]
    fn test_definition_debug_lists_the_rule_name() {
        let debug = format!("{:?}", definition());

        assert!(debug.contains("UniverseDefinition"));
        assert!(debug.contains("static"));
    }
}
