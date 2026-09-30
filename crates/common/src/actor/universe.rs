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

//! Delivery of universe membership changes to actors and strategies.
//!
//! A universe publishes each membership change on the topic
//! `events.universe.{universe}`. A subscriber registers a handler on that topic through the same
//! topic subscription machinery as any other message, so the subscription is released with the
//! component's other subscriptions when it is disposed or unsubscribed.
//!
//! The payload is a typed [`UniverseChange`], dispatched to
//! [`DataActor::on_universe_changed`](crate::actor::DataActor::on_universe_changed): a membership
//! change is not data, so it is not routed through the data pipeline.

use log::error;
use nautilus_model::{identifiers::ActorId, universe::UniverseChange};
use ustr::Ustr;

use super::{Actor, DataActor, DataActorCore, registry::try_get_actor_unchecked};
use crate::msgbus::{ShareableMessageHandler, switchboard::get_universe_membership_topic};

/// Subscribes the actor registered as `actor_id` to the membership changes of `universe`.
///
/// The handler dispatches [`DataActor::on_universe_changed`] on the actor with the concrete type
/// `T`. Subscribing twice to the same universe is a no-op, apart from a warning.
pub fn subscribe_universe_changes<T>(core: &mut DataActorCore, actor_id: Ustr, universe: Ustr)
where
    T: Actor + DataActor + Sized + 'static,
{
    let topic = get_universe_membership_topic(universe);

    let handler = ShareableMessageHandler::from_typed(move |change: &UniverseChange| {
        if let Some(mut actor) = try_get_actor_unchecked::<T>(&actor_id) {
            if let Err(e) = DataActor::on_universe_changed(&mut *actor, change) {
                error!("{e}");
            }
        } else {
            error!("Actor {actor_id} not found for universe change handling");
        }
    });

    core.add_subscription_any(topic, handler, None, None);
}

/// Unsubscribes the actor from the membership changes of `universe`.
pub fn unsubscribe_universe_changes(core: &mut DataActorCore, universe: Ustr) {
    let topic = get_universe_membership_topic(universe);
    core.remove_subscription_any(topic);
}

/// Returns the actor ID used to register a universe component of `universe`.
#[must_use]
pub fn universe_actor_id(universe: Ustr) -> ActorId {
    ActorId::new(universe)
}
