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

//! The universe component.
//!
//! A [`Universe`] is an actor with an explicit membership lifecycle. It evaluates its rule at
//! instants the clock drives, subscribes to what its definition declares for each member, and
//! holds or releases those claims as membership changes.
//!
//! # Determinism
//!
//! Selection takes the instant it is evaluated at. Members are processed in instrument ID order
//! and membership is stored in insertion order, so the same rule, definition, and instant produce
//! the same changes in the same order. The component reads no wall clock and requests nothing from
//! a venue that a backtest cannot answer.
//!
//! # Removal
//!
//! Leaving the universe is a process. A member that selection no longer includes moves to
//! `REMOVING` and keeps its subscriptions. The universe then reports the open orders and positions
//! it can see for that instrument and completes the removal only when
//! [`UniverseRemovalPolicy`](super::UniverseRemovalPolicy) allows it. The universe never submits or
//! cancels orders, so a held removal is reported once and re-evaluated on every subsequent
//! selection step until the owning component has closed what it needs to close.

use std::fmt::Debug;

use ahash::AHashSet;
use anyhow::Result;
use indexmap::IndexMap;
use log::{debug, error, info, warn};
use nautilus_common::{
    actor::{
        DataActor, DataActorConfig, DataActorCore, DataActorNative,
        registry::try_get_actor_unchecked, universe::universe_actor_id,
    },
    clock::ClockApi,
    msgbus::{self, switchboard::get_universe_membership_topic},
    timer::{TimeEvent, TimeEventCallback},
};
use nautilus_core::UnixNanos;
use nautilus_model::{
    identifiers::{ActorId, InstrumentId},
    instruments::{Instrument, InstrumentAny},
    universe::{UniverseChange, UniverseChangeReason, UniverseMembershipState},
};
use ustr::Ustr;

use super::{
    definition::{UniverseDefinition, UniverseRemovalPolicy, UniverseSubscription},
    membership::UniverseMember,
    rule::SharedUniverseRule,
};

/// The prefix of the timer that drives periodic selection.
const SELECTION_TIMER_PREFIX: &str = "UNIVERSE-SELECT";

/// Whether a removal can complete.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RemovalEvaluation {
    Ready,
    Blocked {
        open_orders: usize,
        open_positions: usize,
    },
}

/// An instrument universe with a definition and a membership lifecycle.
pub struct Universe {
    core: DataActorCore,
    definition: UniverseDefinition,
    members: IndexMap<InstrumentId, UniverseMember>,
}

impl Debug for Universe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(Universe))
            .field("actor_id", &self.core.actor_id)
            .field("definition", &self.definition)
            .field("members", &self.members)
            .finish()
    }
}

impl Universe {
    /// Creates a new [`Universe`] from `definition`.
    ///
    /// The universe name identifies the component, so the definition's name must be unique among
    /// the actors registered with a trader.
    ///
    /// # Panics
    ///
    /// Panics if the definition name is not a valid identifier. The name is validated when the
    /// definition is constructed.
    #[must_use]
    pub fn new(definition: UniverseDefinition) -> Self {
        let core = DataActorCore::new(DataActorConfig {
            actor_id: Some(universe_actor_id(definition.name())),
            ..Default::default()
        });

        Self {
            core,
            definition,
            members: IndexMap::new(),
        }
    }

    /// Returns the universe name.
    #[must_use]
    pub fn name(&self) -> Ustr {
        self.definition.name()
    }

    /// Returns the universe actor ID.
    #[must_use]
    pub fn actor_id(&self) -> ActorId {
        self.core.actor_id()
    }

    /// Returns the universe definition.
    #[must_use]
    pub fn definition(&self) -> &UniverseDefinition {
        &self.definition
    }

    /// Returns the selection rule.
    #[must_use]
    pub fn rule(&self) -> &SharedUniverseRule {
        self.definition.rule()
    }

    /// Returns the member instrument IDs in membership order.
    #[must_use]
    pub fn members(&self) -> Vec<InstrumentId> {
        self.members.keys().copied().collect()
    }

    /// Returns the active member instrument IDs in membership order.
    #[must_use]
    pub fn active_members(&self) -> Vec<InstrumentId> {
        self.members
            .values()
            .filter(|member| member.state.is_active())
            .map(|member| member.instrument_id)
            .collect()
    }

    /// Returns the number of members, including those being removed.
    #[must_use]
    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    /// Returns the member record for `instrument_id`.
    #[must_use]
    pub fn membership(&self, instrument_id: &InstrumentId) -> Option<&UniverseMember> {
        self.members.get(instrument_id)
    }

    /// Returns the membership state of `instrument_id`.
    #[must_use]
    pub fn state(&self, instrument_id: &InstrumentId) -> Option<UniverseMembershipState> {
        self.members.get(instrument_id).map(|member| member.state)
    }

    /// Returns whether `instrument_id` holds membership.
    #[must_use]
    pub fn is_member(&self, instrument_id: &InstrumentId) -> bool {
        self.members
            .get(instrument_id)
            .is_some_and(UniverseMember::is_member)
    }

    /// Returns whether `instrument_id` is an active member.
    #[must_use]
    pub fn is_active(&self, instrument_id: &InstrumentId) -> bool {
        self.members
            .get(instrument_id)
            .is_some_and(|member| member.state.is_active())
    }

    /// Returns whether the removal of `instrument_id` is held by the removal policy.
    #[must_use]
    pub fn removal_blocked(&self, instrument_id: &InstrumentId) -> bool {
        self.members
            .get(instrument_id)
            .is_some_and(|member| member.removal_blocked)
    }

    /// Runs one selection step at `now`, returning the number of membership changes.
    ///
    /// The rule is evaluated once. Instruments it includes that hold no membership are added,
    /// instruments whose removal is pending are restored to active, and members it no longer
    /// includes begin removal. Every state change is applied before any change is reported, so a
    /// subscriber never observes a half-applied step.
    ///
    /// # Errors
    ///
    /// Returns an error if the rule cannot evaluate eligibility.
    pub fn select(&mut self, now: UnixNanos) -> Result<usize> {
        let desired = self.evaluate_rule(now)?;
        let desired_set: AHashSet<InstrumentId> = desired.iter().copied().collect();
        let mut updates = Vec::new();

        for instrument_id in &desired {
            match self.state(instrument_id) {
                None | Some(UniverseMembershipState::Removed) => {
                    updates.push((
                        *instrument_id,
                        UniverseMembershipState::Added,
                        UniverseChangeReason::Selected,
                    ));
                }
                Some(UniverseMembershipState::Removing) => {
                    updates.push((
                        *instrument_id,
                        UniverseMembershipState::Active,
                        UniverseChangeReason::Selected,
                    ));
                }
                Some(UniverseMembershipState::Added) => {
                    if self.instrument_known(instrument_id) {
                        updates.push((
                            *instrument_id,
                            UniverseMembershipState::Active,
                            UniverseChangeReason::Selected,
                        ));
                    }
                }
                Some(UniverseMembershipState::Active) => {}
            }
        }

        let mut departing: Vec<InstrumentId> = self
            .members
            .values()
            .filter(|member| member.is_member() && !desired_set.contains(&member.instrument_id))
            .map(|member| member.instrument_id)
            .collect();
        departing.sort_unstable();

        for instrument_id in departing {
            updates.push((
                instrument_id,
                UniverseMembershipState::Removing,
                UniverseChangeReason::Deselected,
            ));
        }

        let mut changes = 0;
        for (instrument_id, state, reason) in updates {
            self.apply(instrument_id, state, reason, now)?;
            changes += 1;
        }

        changes += self.complete_ready_removals(now)?;
        Ok(changes)
    }

    /// Adds `instrument_id` to the universe at `now`, independently of the rule.
    ///
    /// A later selection step that does not include the instrument begins its removal, so an
    /// explicit addition is only stable for a rule that includes it.
    ///
    /// # Errors
    ///
    /// Returns an error if the membership transition is not valid.
    pub fn add(&mut self, instrument_id: InstrumentId, now: UnixNanos) -> Result<()> {
        match self.state(&instrument_id) {
            None | Some(UniverseMembershipState::Removed) => {
                self.apply(
                    instrument_id,
                    UniverseMembershipState::Added,
                    UniverseChangeReason::Explicit,
                    now,
                )?;
            }
            Some(UniverseMembershipState::Added) => {
                debug!(
                    "Universe {} already holds {} as ADDED",
                    self.name(),
                    instrument_id
                );
            }
            Some(UniverseMembershipState::Active) => {
                debug!(
                    "Universe {} already holds {} as ACTIVE",
                    self.name(),
                    instrument_id
                );
            }
            Some(UniverseMembershipState::Removing) => {
                self.apply(
                    instrument_id,
                    UniverseMembershipState::Active,
                    UniverseChangeReason::Explicit,
                    now,
                )?;
            }
        }

        Ok(())
    }

    /// Begins removal of `instrument_id` at `now`.
    ///
    /// # Errors
    ///
    /// Returns an error if the membership transition is not valid.
    pub fn remove(&mut self, instrument_id: InstrumentId, now: UnixNanos) -> Result<()> {
        match self.state(&instrument_id) {
            None => {
                debug!("Universe {} does not hold {}", self.name(), instrument_id);
            }
            Some(UniverseMembershipState::Removed) => {
                debug!("Universe {} already removed {}", self.name(), instrument_id);
            }
            Some(UniverseMembershipState::Removing) => {
                self.complete_ready_removals(now)?;
            }
            Some(UniverseMembershipState::Added | UniverseMembershipState::Active) => {
                self.apply(
                    instrument_id,
                    UniverseMembershipState::Removing,
                    UniverseChangeReason::Explicit,
                    now,
                )?;
                self.complete_ready_removals(now)?;
            }
        }

        Ok(())
    }

    /// Re-evaluates every held removal at `now`, returning the number completed.
    ///
    /// A component that has just closed the orders and position of a departing instrument calls
    /// this to complete the removal without waiting for the next selection step.
    ///
    /// # Errors
    ///
    /// Returns an error if a membership transition is not valid.
    pub fn check_removals(&mut self, now: UnixNanos) -> Result<usize> {
        self.complete_ready_removals(now)
    }

    /// Returns the rule's desired membership at `now`, sorted and de-duplicated.
    fn evaluate_rule(&self, now: UnixNanos) -> Result<Vec<InstrumentId>> {
        let mut desired = {
            let mut rule = self.definition.rule().borrow_mut();
            rule.select(now)?
        };

        desired.sort_unstable();
        desired.dedup();
        Ok(desired)
    }

    /// Returns whether the run knows the definition of `instrument_id`.
    fn instrument_known(&self, instrument_id: &InstrumentId) -> bool {
        self.cache().instrument(instrument_id).is_some()
    }

    /// Applies a single membership transition and reports the change.
    fn apply(
        &mut self,
        instrument_id: InstrumentId,
        state: UniverseMembershipState,
        reason: UniverseChangeReason,
        now: UnixNanos,
    ) -> Result<()> {
        let started_removal = state == UniverseMembershipState::Removing;

        if let Some(member) = self.members.get_mut(&instrument_id) {
            member.transition(state, now)?;

            if started_removal {
                member.removal_reason = Some(reason);
            } else {
                member.removal_reason = None;
                member.removal_blocked = false;
            }
        } else {
            let mut member = UniverseMember::new(instrument_id, state, now, now);
            member.removal_reason = started_removal.then_some(reason);
            self.members.insert(instrument_id, member);
        }

        match state {
            UniverseMembershipState::Added => {
                self.subscribe_member(&instrument_id);
                self.request_member_metadata(&instrument_id);
            }
            UniverseMembershipState::Active | UniverseMembershipState::Removing => {}
            UniverseMembershipState::Removed => {
                self.release_member(&instrument_id);
            }
        }

        self.publish_change(instrument_id, state, reason, now);
        Ok(())
    }

    /// Completes the removals whose members satisfy the removal policy.
    fn complete_ready_removals(&mut self, now: UnixNanos) -> Result<usize> {
        let mut candidates: Vec<InstrumentId> = self
            .members
            .values()
            .filter(|member| member.state == UniverseMembershipState::Removing)
            .map(|member| member.instrument_id)
            .collect();
        candidates.sort_unstable();

        let mut completed = 0;
        for instrument_id in candidates {
            let evaluation = self.evaluate_removal(&instrument_id);

            match evaluation {
                RemovalEvaluation::Ready => {
                    let reason = self
                        .members
                        .get(&instrument_id)
                        .and_then(|member| member.removal_reason)
                        .unwrap_or(UniverseChangeReason::Explicit);

                    self.apply(instrument_id, UniverseMembershipState::Removed, reason, now)?;
                    completed += 1;
                }
                RemovalEvaluation::Blocked {
                    open_orders,
                    open_positions,
                } => self.report_blocked_removal(&instrument_id, open_orders, open_positions),
            }
        }

        Ok(completed)
    }

    /// Returns whether the removal of `instrument_id` can complete under the removal policy.
    fn evaluate_removal(&self, instrument_id: &InstrumentId) -> RemovalEvaluation {
        if self.definition.removal_policy() == UniverseRemovalPolicy::ReleaseRegardless {
            return RemovalEvaluation::Ready;
        }

        let cache = self.cache();
        let open_orders = cache.orders_open_count(None, Some(instrument_id), None, None, None);
        let open_positions =
            cache.positions_open_count(None, Some(instrument_id), None, None, None);

        if open_orders == 0 && open_positions == 0 {
            RemovalEvaluation::Ready
        } else {
            RemovalEvaluation::Blocked {
                open_orders,
                open_positions,
            }
        }
    }

    /// Reports a held removal once per blocking condition.
    fn report_blocked_removal(
        &mut self,
        instrument_id: &InstrumentId,
        open_orders: usize,
        open_positions: usize,
    ) {
        let already_reported = self
            .members
            .get(instrument_id)
            .is_some_and(|member| member.removal_blocked);

        if already_reported {
            return;
        }

        if let Some(member) = self.members.get_mut(instrument_id) {
            member.removal_blocked = true;
        }

        error!(
            "Universe {} cannot remove {}: it still has {open_orders} open order(s) and \
             {open_positions} open position(s). The subscriptions are held until the owning \
             component closes them",
            self.name(),
            instrument_id,
        );
    }

    /// Subscribes to everything the definition declares for `instrument_id`.
    fn subscribe_member(&mut self, instrument_id: &InstrumentId) {
        for subscription in self.definition.subscriptions().to_vec() {
            match subscription {
                UniverseSubscription::Instrument => {
                    DataActor::subscribe_instrument(self, *instrument_id, None, None);
                }
                UniverseSubscription::InstrumentStatus => {
                    DataActor::subscribe_instrument_status(self, *instrument_id, None, None);
                }
                UniverseSubscription::Quotes => {
                    DataActor::subscribe_quotes(self, *instrument_id, None, None);
                }
                UniverseSubscription::Trades => {
                    DataActor::subscribe_trades(self, *instrument_id, None, None);
                }
                UniverseSubscription::Bars => {
                    let bar_type = self.definition.bar_spec().bar_type(*instrument_id);
                    DataActor::subscribe_bars(self, bar_type, None, None);
                }
            }
        }

        debug!(
            "Universe {} subscribed {} ({})",
            self.name(),
            instrument_id,
            self.subscription_names(),
        );
    }

    /// Releases everything the definition declares for `instrument_id`.
    fn release_member(&mut self, instrument_id: &InstrumentId) {
        for subscription in self.definition.subscriptions().to_vec() {
            match subscription {
                UniverseSubscription::Instrument => {
                    DataActor::unsubscribe_instrument(self, *instrument_id, None, None);
                }
                UniverseSubscription::InstrumentStatus => {
                    DataActor::unsubscribe_instrument_status(self, *instrument_id, None, None);
                }
                UniverseSubscription::Quotes => {
                    DataActor::unsubscribe_quotes(self, *instrument_id, None, None);
                }
                UniverseSubscription::Trades => {
                    DataActor::unsubscribe_trades(self, *instrument_id, None, None);
                }
                UniverseSubscription::Bars => {
                    let bar_type = self.definition.bar_spec().bar_type(*instrument_id);
                    DataActor::unsubscribe_bars(self, bar_type, None, None);
                }
            }
        }

        debug!(
            "Universe {} released {} ({})",
            self.name(),
            instrument_id,
            self.subscription_names(),
        );
    }

    /// Requests the definition of `instrument_id` through the data command path.
    fn request_member_metadata(&mut self, instrument_id: &InstrumentId) {
        if let Err(e) = self.request_instrument(*instrument_id, None, None, None, None) {
            warn!(
                "Universe {} could not request the definition of {}: {e}",
                self.name(),
                instrument_id,
            );
        }
    }

    /// Returns the declared subscription names, for logging.
    fn subscription_names(&self) -> String {
        self.definition
            .subscriptions()
            .iter()
            .map(UniverseSubscription::as_str)
            .collect::<Vec<_>>()
            .join(",")
    }

    /// Reports a membership change.
    fn publish_change(
        &self,
        instrument_id: InstrumentId,
        state: UniverseMembershipState,
        reason: UniverseChangeReason,
        now: UnixNanos,
    ) {
        let change = UniverseChange::new(self.name(), instrument_id, state, reason, now, now);

        if self.core.config.log_events {
            info!("{change}");
        } else {
            debug!("{change}");
        }

        let topic = get_universe_membership_topic(self.name());
        msgbus::publish_any(topic, &change);
    }

    /// Returns the name of the selection timer.
    #[must_use]
    pub fn selection_timer_name(&self) -> String {
        format!("{SELECTION_TIMER_PREFIX}:{}", self.name())
    }

    /// Arms the periodic selection timer, if the definition configures an interval.
    fn schedule_selection(&self) {
        let Some(interval_ns) = self.definition.selection_interval_ns() else {
            return;
        };

        let name = self.selection_timer_name();
        let clock = self.clock_api();

        if clock.timer_exists(&name) {
            return;
        }

        let actor_id = self.core.actor_id().inner();
        let callback = TimeEventCallback::from(move |event: TimeEvent| {
            if let Some(mut universe) = try_get_actor_unchecked::<Self>(&actor_id) {
                if let Err(e) = universe.select(event.ts_event) {
                    error!("{e}");
                }
            } else {
                error!("Universe {actor_id} not found for selection");
            }
        });

        if let Err(e) =
            clock.set_timer_ns(&name, interval_ns, None, None, Some(callback), None, None)
        {
            error!("Universe {} could not schedule selection: {e}", self.name());
        }
    }

    /// Cancels the periodic selection timer.
    fn cancel_selection(&self) {
        let name = self.selection_timer_name();

        if self.clock_api().timer_exists(&name) {
            self.clock_api().cancel_timer(&name);
        }
    }

    /// Releases the subscriptions of every member.
    ///
    /// Stopping a universe releases every claim it holds, including the claims of members whose
    /// removal was held by the removal policy: a stopped component must not leave a subscription
    /// behind.
    ///
    /// # Errors
    ///
    /// Returns an error if a membership transition is not valid.
    fn release_all(&mut self) -> Result<()> {
        let mut members: Vec<InstrumentId> = self
            .members
            .values()
            .filter(|member| member.is_member())
            .map(|member| member.instrument_id)
            .collect();
        members.sort_unstable();

        let now = self.clock_api().timestamp_ns();

        for instrument_id in members {
            if self.state(&instrument_id) == Some(UniverseMembershipState::Added) {
                self.apply(
                    instrument_id,
                    UniverseMembershipState::Removing,
                    UniverseChangeReason::Explicit,
                    now,
                )?;
            }

            self.apply(
                instrument_id,
                UniverseMembershipState::Removed,
                UniverseChangeReason::Explicit,
                now,
            )?;
        }

        Ok(())
    }

    /// Returns a clock handle for the component.
    fn clock_api(&self) -> ClockApi<'_> {
        DataActor::clock(self)
    }
}

impl DataActorNative for Universe {
    fn core(&self) -> &DataActorCore {
        &self.core
    }

    fn core_mut(&mut self) -> &mut DataActorCore {
        &mut self.core
    }
}

impl DataActor for Universe {
    fn on_start(&mut self) -> Result<()> {
        let venue = self.definition.venue();

        if let Err(e) = self.request_instruments(Some(venue), None, None, None, None) {
            warn!(
                "Universe {} could not request the instruments of {venue}: {e}",
                self.name(),
            );
        }

        self.schedule_selection();

        let now = self.clock_api().timestamp_ns();
        self.select(now)?;
        Ok(())
    }

    fn on_stop(&mut self) -> Result<()> {
        self.cancel_selection();
        self.release_all()?;
        Ok(())
    }

    fn on_instrument(&mut self, instrument: &InstrumentAny) -> Result<()> {
        let instrument_id = instrument.id();

        if self.state(&instrument_id) != Some(UniverseMembershipState::Added) {
            return Ok(());
        }

        let now = self.clock_api().timestamp_ns();
        self.apply(
            instrument_id,
            UniverseMembershipState::Active,
            UniverseChangeReason::Selected,
            now,
        )?;

        Ok(())
    }
}
