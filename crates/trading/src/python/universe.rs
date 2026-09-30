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

//! Python bindings for instrument universes.
//!
//! A [`PyUniverse`] wraps the native [`Universe`] component behind shared ownership, so the
//! component registered with a run and the object a Python caller holds are the same instance. The
//! wrapper exposes the definition, the membership state, and the selection step; the component's
//! lifecycle callbacks (`on_start`, `on_stop`, `on_instrument`) are driven by the trader that
//! registers it.
//!
//! A rule is any Python object implementing `select(ts_ns)`, so a universe may be defined with the
//! declarative rule classes or with a rule written by the caller.

use std::{
    cell::{RefCell, UnsafeCell},
    fmt::Debug,
    rc::Rc,
};

use nautilus_common::{
    actor::{Actor, registry::with_actor_registry},
    cache::Cache,
    clock::Clock,
    component::{Component, with_component_registry},
};
use nautilus_core::{DurationNanos, UnixNanos, python::to_pyruntime_err};
use nautilus_model::{
    identifiers::{ActorId, ComponentId, InstrumentId, TraderId, Venue},
    universe::UniverseMembershipState,
};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

use crate::{
    SharedUniverseRule,
    universe::{
        ScheduledUniverseRule, StaticUniverseRule, Universe, UniverseDefinition,
        UniverseRemovalPolicy, UniverseRule, UniverseSubscription,
    },
};

/// Python-facing wrapper for a universe component.
#[pyo3::pyclass(module = "nautilus_trader.trading", name = "Universe", unsendable)]
#[gen_stub_pyclass(module = "nautilus_trader.trading")]
pub struct PyUniverse {
    inner: Rc<UnsafeCell<Universe>>,
}

impl Debug for PyUniverse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(PyUniverse))
            .field("universe", self.inner())
            .finish()
    }
}

impl PyUniverse {
    /// Creates a new [`PyUniverse`] from `definition`.
    #[must_use]
    pub fn new(definition: UniverseDefinition) -> Self {
        Self {
            inner: Rc::new(UnsafeCell::new(Universe::new(definition))),
        }
    }

    /// Returns the universe actor ID.
    #[must_use]
    pub fn actor_id(&self) -> ActorId {
        self.inner().actor_id()
    }

    /// Returns a shared reference to the universe.
    #[allow(unsafe_code)]
    fn inner(&self) -> &Universe {
        // Safety: single-threaded kernel with shared ownership, as for the other Python components.
        unsafe { &*self.inner.get() }
    }

    /// Returns the universe for native host integration.
    ///
    /// This is the access a Python component registration path needs; it is not part of the
    /// Python-facing API, and callers must not hold another borrow across the call.
    #[must_use]
    #[allow(unsafe_code, clippy::mut_from_ref)]
    pub fn universe_mut(&self) -> &mut Universe {
        // Safety: single-threaded kernel with shared ownership, as for the other Python components.
        unsafe { &mut *self.inner.get() }
    }

    /// Registers the universe with a trader's clock and cache.
    ///
    /// # Errors
    ///
    /// Returns an error if the universe is already registered with a trader.
    pub fn register_component(
        &self,
        trader_id: TraderId,
        clock: Rc<RefCell<dyn Clock>>,
        cache: Rc<RefCell<Cache>>,
    ) -> anyhow::Result<()> {
        Component::register(self.universe_mut(), trader_id, clock, cache)
    }

    /// Registers the universe in the thread-local actor and component registries.
    pub fn register_in_global_registries(&self) {
        let inner_ref: Rc<UnsafeCell<Universe>> = self.inner.clone();

        let component_id = ComponentId::from(self.actor_id());
        let actor_id = Actor::id(self.inner());

        let component_trait_ref: Rc<UnsafeCell<dyn Component>> = inner_ref.clone();
        with_component_registry(|registry| {
            registry.insert(component_id.inner(), component_trait_ref);
        });

        let actor_trait_ref: Rc<UnsafeCell<dyn Actor>> = inner_ref;
        with_actor_registry(|registry| registry.insert(actor_id, actor_trait_ref));
    }
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl PyUniverse {
    /// Creates a new universe from `definition`.
    #[new]
    fn py_new(definition: &UniverseDefinition) -> Self {
        Self::new(definition.clone())
    }

    /// The name that identifies the universe, its membership topic, and its selection timer.
    #[getter]
    fn name(&self) -> String {
        self.inner().name().to_string()
    }

    /// The number of instruments that hold membership, including those being removed.
    #[getter]
    fn member_count(&self) -> usize {
        self.inner().member_count()
    }

    /// The timer name used when the definition configures periodic selection.
    #[getter]
    fn selection_timer_name(&self) -> String {
        self.inner().selection_timer_name()
    }

    /// Returns the member instrument IDs in membership order.
    fn members(&self) -> Vec<InstrumentId> {
        self.inner().members()
    }

    /// Returns the active member instrument IDs in membership order.
    fn active_members(&self) -> Vec<InstrumentId> {
        self.inner().active_members()
    }

    /// Returns the membership state of `instrument_id`.
    fn state(&self, instrument_id: &InstrumentId) -> Option<UniverseMembershipState> {
        self.inner().state(instrument_id)
    }

    /// Returns whether `instrument_id` holds membership.
    fn is_member(&self, instrument_id: &InstrumentId) -> bool {
        self.inner().is_member(instrument_id)
    }

    /// Returns whether `instrument_id` is an active member.
    fn is_active(&self, instrument_id: &InstrumentId) -> bool {
        self.inner().is_active(instrument_id)
    }

    /// Returns whether the removal of `instrument_id` is held by the removal policy.
    fn removal_blocked(&self, instrument_id: &InstrumentId) -> bool {
        self.inner().removal_blocked(instrument_id)
    }

    /// Runs one selection step at `ts_ns`, returning the number of membership changes.
    ///
    /// # Errors
    ///
    /// Returns an error if the rule cannot evaluate eligibility.
    fn select(&self, ts_ns: u64) -> PyResult<usize> {
        self.universe_mut()
            .select(UnixNanos::from(ts_ns))
            .map_err(to_pyruntime_err)
    }

    /// Adds `instrument_id` to the universe at `ts_ns`, independently of the rule.
    ///
    /// # Errors
    ///
    /// Returns an error if the membership transition is not valid.
    fn add(&self, instrument_id: &InstrumentId, ts_ns: u64) -> PyResult<()> {
        self.universe_mut()
            .add(*instrument_id, UnixNanos::from(ts_ns))
            .map_err(to_pyruntime_err)
    }

    /// Begins removal of `instrument_id` at `ts_ns`.
    ///
    /// # Errors
    ///
    /// Returns an error if the membership transition is not valid.
    fn remove(&self, instrument_id: &InstrumentId, ts_ns: u64) -> PyResult<()> {
        self.universe_mut()
            .remove(*instrument_id, UnixNanos::from(ts_ns))
            .map_err(to_pyruntime_err)
    }

    /// Re-evaluates every held removal at `ts_ns`, returning the number completed.
    ///
    /// # Errors
    ///
    /// Returns an error if a membership transition is not valid.
    fn check_removals(&self, ts_ns: u64) -> PyResult<usize> {
        self.universe_mut()
            .check_removals(UnixNanos::from(ts_ns))
            .map_err(to_pyruntime_err)
    }
}

/// Wraps a Python rule object as a native [`UniverseRule`].
///
/// The rule is called through `select(ts_ns)`, so a rule may be a declarative rule class or any
/// Python object that implements the same method.
pub struct PyUniverseRule {
    rule: Py<PyAny>,
}

impl Debug for PyUniverseRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(PyUniverseRule)).finish()
    }
}

impl PyUniverseRule {
    /// Creates a new [`PyUniverseRule`] wrapping `rule`.
    #[must_use]
    pub fn new(rule: Py<PyAny>) -> Self {
        Self { rule }
    }
}

impl UniverseRule for PyUniverseRule {
    fn select(&mut self, timestamp_ns: UnixNanos) -> anyhow::Result<Vec<InstrumentId>> {
        Python::attach(|py| -> anyhow::Result<Vec<InstrumentId>> {
            let selected = self
                .rule
                .bind(py)
                .call_method1("select", (timestamp_ns.as_u64(),))?;

            selected.extract::<Vec<InstrumentId>>().map_err(Into::into)
        })
    }

    fn name(&self) -> &'static str {
        "PythonUniverseRule"
    }
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl UniverseDefinition {
    /// The rule and settings that describe which instruments are eligible.
    ///
    /// Cloning a definition copies its settings and shares the selection rule.
    #[new]
    fn py_new(name: &str, venue: Venue, rule: &Bound<'_, PyAny>) -> PyResult<Self> {
        let rule: SharedUniverseRule =
            Rc::new(RefCell::new(PyUniverseRule::new(rule.clone().unbind())));

        Self::new(name, venue, rule).map_err(to_pyruntime_err)
    }

    /// Returns the universe name.
    ///
    /// The name identifies the component, its membership topic, and its selection timer.
    #[getter]
    #[pyo3(name = "name")]
    fn py_name(&self) -> String {
        Self::name(self).to_string()
    }

    /// Returns the venue the universe is defined over.
    #[getter]
    #[pyo3(name = "venue")]
    fn py_venue(&self) -> Venue {
        Self::venue(self)
    }

    /// Returns the rule name.
    #[getter]
    #[pyo3(name = "rule_name")]
    fn py_rule_name(&self) -> String {
        Self::rule_name(self)
    }

    /// Returns the subscriptions a member holds.
    #[getter]
    #[pyo3(name = "subscriptions")]
    fn py_subscriptions(&self) -> Vec<UniverseSubscription> {
        Self::subscriptions(self).to_vec()
    }

    /// Returns the selection interval, if selection is periodic.
    #[getter]
    #[pyo3(name = "selection_interval_ns")]
    fn py_selection_interval_ns(&self) -> Option<u64> {
        Self::selection_interval_ns(self).map(|interval| interval.as_u64())
    }

    /// Returns the removal policy.
    #[getter]
    #[pyo3(name = "removal_policy")]
    fn py_removal_policy(&self) -> UniverseRemovalPolicy {
        Self::removal_policy(self)
    }

    /// Sets the subscriptions a member holds.
    #[pyo3(name = "set_subscriptions")]
    fn py_set_subscriptions(&mut self, subscriptions: Vec<UniverseSubscription>) {
        *self = self.clone().with_subscriptions(subscriptions);
    }

    /// Sets the selection interval in nanoseconds.
    ///
    /// # Errors
    ///
    /// Returns an error if the interval is zero.
    #[pyo3(name = "set_selection_interval_ns")]
    fn py_set_selection_interval_ns(&mut self, interval_ns: u64) -> PyResult<()> {
        *self = self
            .clone()
            .with_selection_interval(DurationNanos::new(interval_ns))
            .map_err(to_pyruntime_err)?;
        Ok(())
    }

    /// Sets the removal policy.
    #[pyo3(name = "set_removal_policy")]
    fn py_set_removal_policy(&mut self, removal_policy: UniverseRemovalPolicy) {
        *self = self.clone().with_removal_policy(removal_policy);
    }
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl StaticUniverseRule {
    /// A rule with the same membership set at every instant.
    #[new]
    fn py_new(name: &str, instruments: Vec<InstrumentId>) -> PyResult<Self> {
        Self::new(name, instruments).map_err(to_pyruntime_err)
    }

    /// Returns the instruments eligible at `ts_ns`.
    ///
    /// # Errors
    ///
    /// Returns an error if the rule cannot evaluate eligibility.
    #[pyo3(name = "select")]
    fn py_select(&mut self, ts_ns: u64) -> PyResult<Vec<InstrumentId>> {
        UniverseRule::select(self, UnixNanos::from(ts_ns)).map_err(to_pyruntime_err)
    }
}

#[gen_stub_pymethods]
#[pyo3::pymethods]
impl ScheduledUniverseRule {
    /// A rule whose membership is a declared schedule of sets.
    ///
    /// Selection returns the set with the greatest effective instant at or before the instant it is
    /// asked about, and nothing before the first set. A schedule is input data to a run, so the same
    /// schedule always produces the same membership at the same instant.
    #[new]
    fn py_new(name: &str, sets: Vec<(u64, Vec<InstrumentId>)>) -> PyResult<Self> {
        Self::from_schedule(
            name,
            sets.into_iter()
                .map(|(effective_ns, instruments)| (UnixNanos::from(effective_ns), instruments)),
        )
        .map_err(to_pyruntime_err)
    }

    /// Returns the instruments effective at `ts_ns`.
    ///
    /// # Errors
    ///
    /// Returns an error if the rule cannot evaluate eligibility.
    #[pyo3(name = "select")]
    fn py_select(&mut self, ts_ns: u64) -> PyResult<Vec<InstrumentId>> {
        UniverseRule::select(self, UnixNanos::from(ts_ns)).map_err(to_pyruntime_err)
    }
}
