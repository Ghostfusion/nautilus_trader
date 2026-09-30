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

//! Delivery of corporate actions to actors and strategies.
//!
//! A corporate action is published on the topic `data.corporate_actions.{venue}.{symbol}`, built
//! by [`crate::msgbus::switchboard::get_corporate_action_topic`]. A subscriber registers a handler
//! on that topic through the same topic subscription machinery as any other message, so the
//! subscription is released with the component's other subscriptions when it is disposed or
//! unsubscribed.
//!
//! The payload is a typed [`CorporateAction`], dispatched to
//! [`DataActor::on_corporate_action`](crate::actor::DataActor::on_corporate_action).
//!
//! In a backtest the action records reach a subscriber only when the data is loaded with an
//! adjustment (or another configuration that replays them): an unconfigured run never opens the
//! action stream.

use log::error;
use nautilus_model::{data::CorporateAction, identifiers::InstrumentId};
use ustr::Ustr;

use super::{Actor, DataActor, DataActorCore, registry::try_get_actor_unchecked};
use crate::msgbus::{ShareableMessageHandler, switchboard::get_corporate_action_topic};

/// Subscribes the actor registered as `actor_id` to the corporate actions of `instrument_id`.
///
/// The handler dispatches [`DataActor::on_corporate_action`] on the actor with the concrete type
/// `T`. Subscribing twice to the same instrument is a no-op, apart from a warning.
pub fn subscribe_corporate_actions<T>(
    core: &mut DataActorCore,
    actor_id: Ustr,
    instrument_id: InstrumentId,
) where
    T: Actor + DataActor + Sized + 'static,
{
    let topic = get_corporate_action_topic(instrument_id);

    let handler = ShareableMessageHandler::from_typed(move |action: &CorporateAction| {
        if let Some(mut actor) = try_get_actor_unchecked::<T>(&actor_id) {
            if let Err(e) = DataActor::on_corporate_action(&mut *actor, action) {
                error!("{e}");
            }
        } else {
            error!("Actor {actor_id} not found for corporate action handling");
        }
    });

    core.add_subscription_any(topic, handler, None, None);
}

/// Unsubscribes the actor from the corporate actions of `instrument_id`.
pub fn unsubscribe_corporate_actions(core: &mut DataActorCore, instrument_id: InstrumentId) {
    let topic = get_corporate_action_topic(instrument_id);
    core.remove_subscription_any(topic);
}
