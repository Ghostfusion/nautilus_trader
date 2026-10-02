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

//! What a chain is streaming, and where each stream is served from.
//!
//! A subscription is not a request. A request is answered and over; a stream continues, and whoever
//! is watching it cannot tell a change of source from a venue outage followed by a new venue - the
//! timestamps, the granularity, the latency, and sometimes the meaning of a field all change with the
//! provider. The rules that follow are therefore about the *whole* stream rather than about one
//! demand:
//!
//! - **A stream moves as part of a provider, never on its own.** One instrument must not come from
//!   two sources at once, so what moves is everything one provider was serving.
//! - **The old subscription is given up before the new one is taken.** A stream that is unsubscribed
//!   from one provider and subscribed at another is briefly absent, which is visible; a stream served
//!   from both is not, which is worse.
//! - **A move is announced.** The change of source is written as an event, because a chain that
//!   substitutes its datasets silently is the risk the whole design is arranged against.
//!
//! This module holds the bookkeeping for those rules and nothing else: which streams are held, by
//! whom, and the commands that would hold them somewhere else.

use std::{any::Any, fmt::Display, sync::Arc};

use nautilus_common::messages::data::{
    SubscribeBars, SubscribeQuotes, SubscribeTrades, UnsubscribeBars, UnsubscribeQuotes,
    UnsubscribeTrades,
};
use nautilus_core::{UUID4, UnixNanos};
use nautilus_model::{
    data::{BarType, CustomDataTrait, HasTsInit},
    identifiers::{ClientId, InstrumentId},
};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

/// The data type a chain's continuity event is written under.
pub const CONTINUITY_DATA_TYPE: &str = "FailoverContinuity";

/// What identifies one stream: the thing being streamed.
///
/// A subscription and the unsubscription that ends it name the same thing, which is how a chain knows
/// which stream a caller is giving up.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Stream {
    /// Bars for one instrument and bar type.
    Bars(BarType),
    /// Quotes for one instrument.
    Quotes(InstrumentId),
    /// Trades for one instrument.
    Trades(InstrumentId),
}

impl Display for Stream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bars(bar_type) => write!(f, "bars {bar_type}"),
            Self::Quotes(instrument_id) => write!(f, "quotes {instrument_id}"),
            Self::Trades(instrument_id) => write!(f, "trades {instrument_id}"),
        }
    }
}

/// A subscription a chain can hold.
#[derive(Debug, Clone)]
pub enum Subscription {
    /// Bars for one instrument and bar type.
    Bars(SubscribeBars),
    /// Quotes for one instrument.
    Quotes(SubscribeQuotes),
    /// Trades for one instrument.
    Trades(SubscribeTrades),
}

impl Subscription {
    /// Returns what this subscription streams.
    #[must_use]
    pub fn stream(&self) -> Stream {
        match self {
            Self::Bars(command) => Stream::Bars(command.bar_type),
            Self::Quotes(command) => Stream::Quotes(command.instrument_id),
            Self::Trades(command) => Stream::Trades(command.instrument_id),
        }
    }

    /// Returns this subscription addressed to `client_id`.
    #[must_use]
    pub fn addressed_to(&self, client_id: ClientId) -> Self {
        match self {
            Self::Bars(command) => {
                let mut command = command.clone();
                command.client_id = Some(client_id);

                Self::Bars(command)
            }
            Self::Quotes(command) => {
                let mut command = command.clone();
                command.client_id = Some(client_id);

                Self::Quotes(command)
            }
            Self::Trades(command) => {
                let mut command = command.clone();
                command.client_id = Some(client_id);

                Self::Trades(command)
            }
        }
    }

    /// Returns the command that gives this stream up.
    ///
    /// The identifiers are the chain's own rather than a caller's: this is a command the chain makes
    /// when it moves a stream, and it is made about the same thing the subscription was.
    #[must_use]
    pub fn released(&self) -> Unsubscription {
        match self {
            Self::Bars(command) => Unsubscription::Bars(UnsubscribeBars {
                bar_type: command.bar_type,
                client_id: command.client_id,
                venue: command.venue,
                command_id: UUID4::new(),
                ts_init: UnixNanos::default(),
                correlation_id: command.correlation_id,
                params: command.params.clone(),
            }),
            Self::Quotes(command) => Unsubscription::Quotes(UnsubscribeQuotes {
                instrument_id: command.instrument_id,
                client_id: command.client_id,
                venue: command.venue,
                command_id: UUID4::new(),
                ts_init: UnixNanos::default(),
                correlation_id: command.correlation_id,
                params: command.params.clone(),
            }),
            Self::Trades(command) => Unsubscription::Trades(UnsubscribeTrades {
                instrument_id: command.instrument_id,
                client_id: command.client_id,
                venue: command.venue,
                command_id: UUID4::new(),
                ts_init: UnixNanos::default(),
                correlation_id: command.correlation_id,
                params: command.params.clone(),
            }),
        }
    }
}

/// An unsubscription a chain asks a provider for.
#[derive(Debug, Clone)]
pub enum Unsubscription {
    /// Bars for one instrument and bar type.
    Bars(UnsubscribeBars),
    /// Quotes for one instrument.
    Quotes(UnsubscribeQuotes),
    /// Trades for one instrument.
    Trades(UnsubscribeTrades),
}

impl Unsubscription {
    /// Returns what this unsubscription gives up.
    #[must_use]
    pub fn stream(&self) -> Stream {
        match self {
            Self::Bars(command) => Stream::Bars(command.bar_type),
            Self::Quotes(command) => Stream::Quotes(command.instrument_id),
            Self::Trades(command) => Stream::Trades(command.instrument_id),
        }
    }
}

/// One stream a chain holds, and where it is served from.
#[derive(Debug, Clone)]
struct Held {
    stream: Stream,
    provider: ClientId,
    command: Subscription,
}

/// What a chain is streaming, and where from.
///
/// The streams are kept in the order they were taken, because a move gives them up in that order and
/// takes them again in the same order: a stream that changes source should look to whoever is
/// watching it like the same stream, apart from where it comes from.
#[derive(Debug, Default)]
pub struct Streaming {
    held: Mutex<Vec<Held>>,
}

impl Streaming {
    /// Creates a record of nothing being streamed.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the provider serving `stream`, when the chain holds it.
    #[must_use]
    pub fn provider_of(&self, stream: &Stream) -> Option<ClientId> {
        self.held
            .lock()
            .iter()
            .find(|held| held.stream == *stream)
            .map(|held| held.provider)
    }

    /// Returns the provider the chain streams from, when it streams anything.
    #[must_use]
    pub fn provider(&self) -> Option<ClientId> {
        self.held.lock().first().map(|held| held.provider)
    }

    /// Returns what `provider` is streaming, in the order it was taken.
    #[must_use]
    pub fn held_by(&self, provider: &ClientId) -> Vec<(Stream, Subscription)> {
        self.held
            .lock()
            .iter()
            .filter(|held| held.provider == *provider)
            .map(|held| (held.stream.clone(), held.command.clone()))
            .collect()
    }

    /// Notes that `provider` is now serving `stream`.
    pub fn hold(&self, stream: Stream, provider: ClientId, command: Subscription) {
        let mut held = self.held.lock();

        if let Some(stream) = held.iter_mut().find(|held| held.stream == stream) {
            stream.provider = provider;
            stream.command = command;

            return;
        }

        held.push(Held {
            stream,
            provider,
            command,
        });
    }

    /// Stops holding `stream`, returning the provider that was serving it.
    pub fn release(&self, stream: &Stream) -> Option<ClientId> {
        let mut held = self.held.lock();
        let index = held.iter().position(|held| held.stream == *stream)?;

        Some(held.remove(index).provider)
    }

    /// Returns every stream the chain holds, with the provider serving it.
    #[must_use]
    pub fn held(&self) -> Vec<(Stream, ClientId)> {
        self.held
            .lock()
            .iter()
            .map(|held| (held.stream.clone(), held.provider))
            .collect()
    }

    /// Returns how many streams the chain holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.held.lock().len()
    }

    /// Returns whether the chain holds no stream at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// The event a chain writes when a stream changes source.
///
/// A move is not silent. Whoever is watching the stream is told what moved, from where, to where, and
/// why, and the same record is what an operator reads afterwards when a chain hopped.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Continuity {
    /// What is being streamed.
    pub stream: String,
    /// The provider that was serving it.
    pub from: ClientId,
    /// The provider that serves it now.
    pub to: ClientId,
    /// Why it moved.
    pub reason: String,
    /// The time the move took effect.
    pub ts_event: UnixNanos,
    /// The time the move was recorded.
    pub ts_init: UnixNanos,
}

impl HasTsInit for Continuity {
    fn ts_init(&self) -> UnixNanos {
        self.ts_init
    }
}

impl CustomDataTrait for Continuity {
    fn type_name(&self) -> &'static str {
        CONTINUITY_DATA_TYPE
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn ts_event(&self) -> UnixNanos {
        self.ts_event
    }

    fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string(self)?)
    }

    fn clone_arc(&self) -> Arc<dyn CustomDataTrait> {
        Arc::new(self.clone())
    }

    fn eq_arc(&self, other: &dyn CustomDataTrait) -> bool {
        other
            .as_any()
            .downcast_ref::<Self>()
            .is_some_and(|other| other == self)
    }
}

#[cfg(test)]
mod tests {
    use nautilus_common::messages::data::SubscribeBars;
    use nautilus_model::{
        data::BarSpecification,
        enums::{AggregationSource, BarAggregation, PriceType},
    };
    use rstest::rstest;

    use super::*;

    fn bar_type(symbol: &str) -> BarType {
        BarType::new(
            InstrumentId::from(symbol),
            BarSpecification::new(1, BarAggregation::Day, PriceType::Last),
            AggregationSource::External,
        )
    }

    fn bars(symbol: &str) -> Subscription {
        Subscription::Bars(SubscribeBars::new(
            bar_type(symbol),
            Some(ClientId::from("CHAIN")),
            None,
            UUID4::new(),
            UnixNanos::default(),
            None,
            None,
        ))
    }

    #[rstest]
    fn test_a_stream_is_held_with_the_provider_serving_it() {
        let streaming = Streaming::new();
        let command = bars("AAPL.US");
        let stream = command.stream();

        assert!(streaming.is_empty());
        assert_eq!(streaming.provider_of(&stream), None);

        streaming.hold(stream.clone(), ClientId::from("PRIMARY"), command);

        assert_eq!(
            streaming.provider_of(&stream),
            Some(ClientId::from("PRIMARY"))
        );
        assert_eq!(streaming.provider(), Some(ClientId::from("PRIMARY")));
        assert_eq!(streaming.len(), 1);
    }

    #[rstest]
    fn test_holding_a_stream_again_moves_it_rather_than_duplicating_it() {
        let streaming = Streaming::new();
        let command = bars("AAPL.US");
        let stream = command.stream();

        streaming.hold(stream.clone(), ClientId::from("PRIMARY"), command.clone());
        streaming.hold(stream.clone(), ClientId::from("SECONDARY"), command);

        assert_eq!(
            streaming.provider_of(&stream),
            Some(ClientId::from("SECONDARY"))
        );
        assert_eq!(
            streaming.len(),
            1,
            "one stream is one stream, whichever provider serves it"
        );
    }

    #[rstest]
    fn test_a_released_stream_is_no_longer_held() {
        let streaming = Streaming::new();
        let command = bars("AAPL.US");
        let stream = command.stream();

        streaming.hold(stream.clone(), ClientId::from("PRIMARY"), command);

        assert_eq!(streaming.release(&stream), Some(ClientId::from("PRIMARY")));
        assert!(
            streaming.is_empty(),
            "a stream cannot be in two providers' hands"
        );
        assert_eq!(streaming.release(&stream), None);
    }

    /// What moves is a provider's streams, so the chain has to be able to name them, in the order
    /// they were taken.
    #[rstest]
    fn test_only_the_streams_of_one_provider_are_returned_for_it() {
        let streaming = Streaming::new();
        let first = bars("AAPL.US");
        let second = bars("MSFT.US");

        streaming.hold(first.stream(), ClientId::from("PRIMARY"), first.clone());
        streaming.hold(second.stream(), ClientId::from("SECONDARY"), second);

        let held = streaming.held_by(&ClientId::from("PRIMARY"));

        assert_eq!(held.len(), 1);
        assert_eq!(held[0].0, first.stream());
        assert_eq!(streaming.held_by(&ClientId::from("SECONDARY")).len(), 1);
        assert!(streaming.held_by(&ClientId::from("TERTIARY")).is_empty());
    }

    #[rstest]
    fn test_the_streams_are_kept_in_the_order_they_were_taken() {
        let streaming = Streaming::new();
        let first = bars("AAPL.US");
        let second = bars("MSFT.US");

        streaming.hold(first.stream(), ClientId::from("PRIMARY"), first.clone());
        streaming.hold(second.stream(), ClientId::from("PRIMARY"), second.clone());

        let held = streaming.held();

        assert_eq!(held[0].0, first.stream());
        assert_eq!(held[1].0, second.stream());
    }

    #[rstest]
    fn test_a_subscription_and_its_unsubscription_are_the_same_stream() {
        let command = bars("AAPL.US");
        let released = command.released();

        assert_eq!(command.stream(), released.stream());
    }

    #[rstest]
    fn test_a_command_is_addressed_to_the_provider_that_receives_it() {
        let command = bars("AAPL.US");
        let addressed = command.addressed_to(ClientId::from("SECONDARY"));

        assert_eq!(addressed.stream(), command.stream());
        assert!(matches!(
            addressed,
            Subscription::Bars(SubscribeBars {
                client_id: Some(client_id),
                ..
            }) if client_id == ClientId::from("SECONDARY")
        ));
    }

    #[rstest]
    fn test_the_continuity_event_carries_what_moved_and_why() {
        let continuity = Continuity {
            stream: "bars AAPL.US".to_string(),
            from: ClientId::from("PRIMARY"),
            to: ClientId::from("SECONDARY"),
            reason: "the primary stopped serving".to_string(),
            ts_event: UnixNanos::default(),
            ts_init: UnixNanos::default(),
        };

        assert_eq!(continuity.type_name(), CONTINUITY_DATA_TYPE);
        assert!(continuity.to_json().unwrap().contains("SECONDARY"));
        assert!(
            continuity.eq_arc(Arc::new(continuity.clone()).as_ref()),
            "the same event is equal to itself, which is what a data type is compared by"
        );
        assert_eq!(continuity.ts_init(), UnixNanos::default());
    }
}
