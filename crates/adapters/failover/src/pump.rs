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

//! The engine side of a chain's boundary.
//!
//! The pump has two halves and they belong together: the sender a chain's providers are built with,
//! and the task that drains what they write. What it drains it forwards to the engine under the
//! chain's identity, and it reads on the way past.
//!
//! # What it reads, and why that is the only place it can be read
//!
//! A response answers a request, and the only thing that ties the two together is the correlation
//! identifier the caller gave. A chain keeps the demands it has issued until they are answered,
//! because the one failure a provider cannot report *when it is asked* is that it took the demand and
//! then said nothing - and a provider that is down is exactly the provider that does that. So an
//! answer says which provider served, and a deadline passing with nothing in a demand's place says
//! which provider did not, and neither of those is visible from the demand itself.

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use nautilus_common::{live::sender::EventSender, messages::DataEvent};
use nautilus_core::UUID4;
use nautilus_model::identifiers::ClientId;
use parking_lot::Mutex;

use crate::{failure::Failure, health::Health, tap::Tap};

/// A demand that has been issued and not yet answered.
#[derive(Debug, Clone)]
struct Expected {
    provider: ClientId,
    due: Instant,
}

/// The demands a chain has issued and not yet seen answered.
///
/// This is keyed by the caller's correlation identifier, because that is what the answer carries. A
/// demand is removed when it is answered or when its deadline passes, never by being replaced: a late
/// answer to a demand that has already been given up on must not be counted as serving a later one.
#[derive(Debug, Default)]
pub struct Pending {
    demands: Mutex<std::collections::HashMap<UUID4, Expected>>,
}

impl Pending {
    /// Creates an empty register.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Notes that `provider` has taken a demand, and has until `due` to answer it.
    pub fn expect(&self, request_id: UUID4, provider: ClientId, due: Instant) {
        self.demands
            .lock()
            .insert(request_id, Expected { provider, due });
    }

    /// Stops expecting anything for `request_id`.
    pub fn forget(&self, request_id: UUID4) {
        self.demands.lock().remove(&request_id);
    }

    /// Notes that `event` answers a demand, and returns the provider that was expected to answer it.
    ///
    /// Market data answers nothing: it carries no identifier, and it is not what a request is
    /// matched to.
    #[must_use]
    pub fn resolve(&self, event: &DataEvent) -> Option<ClientId> {
        let DataEvent::Response(response) = event else {
            return None;
        };

        self.demands
            .lock()
            .remove(response.correlation_id())
            .map(|expected| expected.provider)
    }

    /// Returns the demands whose deadline has passed, and forgets them.
    #[must_use]
    pub fn expire(&self, now: Instant) -> Vec<(UUID4, ClientId)> {
        let mut demands = self.demands.lock();

        demands
            .extract_if(|_, expected| now >= expected.due)
            .map(|(request_id, expected)| (request_id, expected.provider))
            .collect()
    }

    /// Returns how many demands are outstanding.
    #[must_use]
    pub fn len(&self) -> usize {
        self.demands.lock().len()
    }

    /// Returns whether no demand is outstanding.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Carries a chain's events to the engine, under the chain's identity, and holds its providers to
/// account for what they were asked.
///
/// The pump does not write: the providers do, because each captures the sender it is given when it is
/// constructed. That gives the pump two halves, and they belong together - the sender, which a chain
/// installs only while it is building its providers, and the task, which runs for as long as they can
/// still write and reads what they answer for the whole of that time.
#[derive(Debug)]
pub struct Pump {
    tap: Tap,
    pending: Arc<Pending>,
    health: Arc<Health>,
    events: tokio::sync::mpsc::UnboundedReceiver<DataEvent>,
}

impl Pump {
    /// Returns the pump, and the sender a chain's providers must be built with.
    #[must_use]
    pub fn new(
        client_id: ClientId,
        engine: EventSender<DataEvent>,
        pending: Arc<Pending>,
        health: Arc<Health>,
    ) -> (Self, EventSender<DataEvent>) {
        let (sender, events) = tokio::sync::mpsc::unbounded_channel();
        let tap = Tap::new(client_id, engine);

        (
            Self {
                tap,
                pending,
                health,
                events,
            },
            EventSender::from(sender),
        )
    }

    /// Forwards events for as long as any provider can still write one.
    ///
    /// Stops rather than spinning once the engine can no longer receive: there is nothing left that
    /// the events could be for, and a chain that is being torn down should not keep writing into a
    /// channel nobody reads.
    pub async fn run(mut self) {
        // The deadline is checked several times within itself, so that a demand is given what it was
        // promised within a fraction of it rather than up to twice it.
        let tick = (self.health.policy().answer_deadline / 4).max(Duration::from_millis(10));
        let mut ticker = tokio::time::interval(tick);

        loop {
            tokio::select! {
                event = self.events.recv() => {
                    let Some(event) = event else {
                        return;
                    };

                    if let Some(provider) = self.pending.resolve(&event) {
                        log::debug!("{provider} answered a demand");
                        self.health.answered(&provider);
                    }

                    if !self.tap.forward(event) {
                        log::warn!("stopping the chain's boundary: the engine can no longer receive");

                        return;
                    }
                }
                _ = ticker.tick() => {
                    let now = Instant::now();

                    for (request_id, provider) in self.pending.expire(now) {
                        log::warn!(
                            "{provider} took a demand and did not answer it: {request_id} is \
                             unanswered"
                        );

                        self.health.failed(&provider, &Failure::Unanswered, now);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use nautilus_common::messages::data::{BarsResponse, DataResponse};
    use nautilus_core::UnixNanos;
    use nautilus_model::{
        data::{BarSpecification, BarType},
        enums::{AggregationSource, BarAggregation, PriceType},
        identifiers::InstrumentId,
        instruments::{InstrumentAny, stubs},
    };
    use rstest::rstest;

    use super::*;
    use crate::health::Policy;

    const CHAIN: &str = "CHAIN";
    const PROVIDER: &str = "MOOMOO";

    fn chain_id() -> ClientId {
        ClientId::from(CHAIN)
    }

    fn provider() -> ClientId {
        ClientId::from(PROVIDER)
    }

    fn health() -> Arc<Health> {
        Arc::new(Health::new(Policy {
            failure_threshold: 1,
            cooldown: Duration::from_secs(60),
            answer_deadline: Duration::from_millis(500),
        }))
    }

    fn bar_type() -> BarType {
        BarType::new(
            InstrumentId::from("AAPL.US"),
            BarSpecification::new(1, BarAggregation::Day, PriceType::Last),
            AggregationSource::External,
        )
    }

    /// An answer as a provider writes it: about itself, carrying the caller's identifier.
    fn answer(correlation_id: UUID4) -> DataEvent {
        DataEvent::Response(DataResponse::Bars(BarsResponse::new(
            correlation_id,
            provider(),
            bar_type(),
            Vec::new(),
            None,
            None,
            UnixNanos::default(),
            None,
        )))
    }

    fn market_data() -> DataEvent {
        DataEvent::Instrument(InstrumentAny::Equity(stubs::equity_aapl()))
    }

    #[rstest]
    fn test_an_answer_names_the_provider_the_demand_was_given_to() {
        let pending = Pending::new();
        let request_id = UUID4::new();
        let due = Instant::now() + Duration::from_secs(30);

        pending.expect(request_id, provider(), due);

        assert_eq!(pending.resolve(&answer(request_id)), Some(provider()));
        assert!(pending.is_empty());
    }

    #[rstest]
    fn test_an_answer_to_nothing_is_not_an_answer() {
        let pending = Pending::new();

        assert_eq!(pending.resolve(&answer(UUID4::new())), None);
        assert_eq!(pending.resolve(&market_data()), None);
    }

    /// The late answer to a demand that has already been given up on.
    #[rstest]
    fn test_a_forgotten_demand_is_not_answered() {
        let pending = Pending::new();
        let request_id = UUID4::new();

        pending.expect(
            request_id,
            provider(),
            Instant::now() + Duration::from_secs(30),
        );
        pending.forget(request_id);

        assert_eq!(pending.resolve(&answer(request_id)), None);
    }

    #[rstest]
    fn test_a_demand_is_expired_only_once_its_deadline_has_passed() {
        let pending = Pending::new();
        let request_id = UUID4::new();
        let now = Instant::now();
        let due = now + Duration::from_secs(30);

        pending.expect(request_id, provider(), due);

        assert!(pending.expire(now).is_empty());
        assert_eq!(pending.len(), 1);
        assert_eq!(pending.expire(due), vec![(request_id, provider())]);
        assert!(pending.is_empty(), "an expired demand is given up on");
    }

    /// A demand is answered or given up on, never both.
    #[rstest]
    fn test_an_answered_demand_cannot_also_expire() {
        let pending = Pending::new();
        let request_id = UUID4::new();
        let due = Instant::now();

        pending.expect(request_id, provider(), due);

        assert_eq!(pending.resolve(&answer(request_id)), Some(provider()));
        assert!(pending.expire(due).is_empty());
    }

    #[tokio::test]
    async fn test_the_pump_carries_an_answer_to_the_engine_under_the_chains_identity() {
        let (engine, mut engine_events) = tokio::sync::mpsc::unbounded_channel();
        let (pump, providers) = Pump::new(
            chain_id(),
            EventSender::from(engine),
            Arc::new(Pending::new()),
            health(),
        );

        providers.send(answer(UUID4::new())).unwrap();
        drop(providers);

        pump.run().await;

        let DataEvent::Response(DataResponse::Bars(delivered)) = engine_events
            .try_recv()
            .expect("the engine should have been given the answer")
        else {
            panic!("expected a bar response");
        };

        assert_eq!(delivered.client_id, chain_id());
    }

    #[tokio::test]
    async fn test_the_pump_holds_a_provider_to_account_for_an_answer_it_gave() {
        let (engine, _engine_events) = tokio::sync::mpsc::unbounded_channel();
        let health = health();
        let pending = Arc::new(Pending::new());
        let (pump, providers) = Pump::new(
            chain_id(),
            EventSender::from(engine),
            Arc::clone(&pending),
            Arc::clone(&health),
        );

        health.failed(&provider(), &Failure::Unanswered, Instant::now());
        assert!(health.snapshot(&provider()).is_open);

        let request_id = UUID4::new();
        health.asked(&provider());
        pending.expect(
            request_id,
            provider(),
            Instant::now() + Duration::from_secs(30),
        );

        providers.send(answer(request_id)).unwrap();
        drop(providers);

        pump.run().await;

        assert!(
            !health.snapshot(&provider()).is_open,
            "an answer clears the failures"
        );
        assert_eq!(health.snapshot(&provider()).answers, 1);
    }

    /// The failure a chain cannot see when it asks: a provider that took the demand and said nothing.
    #[tokio::test]
    async fn test_the_pump_holds_a_provider_to_account_for_a_demand_it_never_answered() {
        let (engine, _engine_events) = tokio::sync::mpsc::unbounded_channel();
        let health = Arc::new(Health::new(Policy {
            failure_threshold: 1,
            cooldown: Duration::from_secs(60),
            answer_deadline: Duration::from_millis(20),
        }));
        let pending = Arc::new(Pending::new());
        let (pump, _providers) = Pump::new(
            chain_id(),
            EventSender::from(engine),
            Arc::clone(&pending),
            Arc::clone(&health),
        );
        let pump = tokio::spawn(pump.run());

        pending.expect(
            UUID4::new(),
            provider(),
            Instant::now() + Duration::from_millis(20),
        );

        tokio::time::sleep(Duration::from_millis(200)).await;

        let snapshot = health.snapshot(&provider());

        assert!(snapshot.is_open, "the demand went unanswered");
        assert_eq!(snapshot.last_failure, Some(Failure::Unanswered));

        pump.abort();
    }

    /// A chain being torn down writes into a channel nobody reads, and must not keep writing.
    #[tokio::test]
    async fn test_the_pump_stops_once_the_engine_can_no_longer_receive() {
        let (engine, engine_events) = tokio::sync::mpsc::unbounded_channel();
        drop(engine_events);

        let (pump, providers) = Pump::new(
            chain_id(),
            EventSender::from(engine),
            Arc::new(Pending::new()),
            health(),
        );

        providers.send(market_data()).unwrap();
        drop(providers);

        tokio::time::timeout(Duration::from_secs(5), pump.run())
            .await
            .expect("the pump should stop rather than write into a closed engine");
    }
}
