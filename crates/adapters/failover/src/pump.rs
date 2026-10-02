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
//! A provider answers a request by writing an event, and the engine matches that answer to the
//! request by the correlation identifier the caller gave it. A chain watches the same stream for a
//! different reason: a provider's own call reports only that the command was accepted, so whether a
//! provider actually served a demand is visible nowhere else. This is where a chain's events leave
//! for the engine, under the chain's identity, and where what answers a demand is decided.
//!
//! # Why the register is part of the boundary
//!
//! The answer to "did this provider serve the demand" is a response carrying the caller's
//! correlation identifier, and nothing else. A chain that hopped because it could not see an answer
//! would be guessing; one that hopped because it saw the provider refuse has a reason it can report.
//! The register is what turns the second case out of the first.

use std::{collections::HashMap, sync::Arc};

use nautilus_common::{live::sender::EventSender, messages::DataEvent};
use nautilus_core::UUID4;
use nautilus_model::identifiers::ClientId;
use parking_lot::Mutex;

use crate::tap::Tap;

/// The demands a chain is waiting on, by the identifier the caller gave them.
///
/// A forgotten demand is not woken. An answer that arrives after its deadline has passed is a late
/// answer, and it must not be counted as serving a demand made later, so the record of the demand is
/// removed when the chain stops waiting for it rather than when an answer does or does not arrive.
#[derive(Debug, Default)]
pub struct Answers {
    waiting: Mutex<HashMap<UUID4, tokio::sync::oneshot::Sender<()>>>,
}

impl Answers {
    /// Creates an empty register.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns a receiver that is woken when the demand identified by `correlation_id` is answered.
    ///
    /// A second demand for the same identifier replaces the first. Identifiers are the caller's to
    /// mint, and an engine matches one answer to one running demand, so two of them at once is a
    /// caller's error rather than a state the chain can serve both of.
    pub fn wait_for(&self, correlation_id: UUID4) -> tokio::sync::oneshot::Receiver<()> {
        let (sender, receiver) = tokio::sync::oneshot::channel();
        self.waiting.lock().insert(correlation_id, sender);

        receiver
    }

    /// Stops waiting for `correlation_id`, whether or not an answer has arrived.
    pub fn forget(&self, correlation_id: UUID4) {
        self.waiting.lock().remove(&correlation_id);
    }

    /// Notes that `event` answers a demand, and reports whether one was waiting for it.
    ///
    /// A demand stops waiting either by being forgotten or by giving up what it waits on, and
    /// neither of those is an answer.
    ///
    /// Market data answers nothing: it carries no identifier, and it is not what a request is
    /// matched to.
    #[must_use]
    pub fn resolve(&self, event: &DataEvent) -> bool {
        let DataEvent::Response(response) = event else {
            return false;
        };

        match self.waiting.lock().remove(response.correlation_id()) {
            Some(sender) => sender.send(()).is_ok(),
            None => false,
        }
    }
}

/// Carries a chain's events to the engine, under the chain's identity.
///
/// The pump does not write: the providers do, because each captures the sender it is given when it
/// is constructed. That gives the pump two halves, and they belong together - the sender, which a
/// chain installs only while it is building its providers, and the task, which runs for as long as
/// they can still write. A provider built while the engine's own sender is installed writes past the
/// chain entirely, so the two halves are created together and returned together.
#[derive(Debug)]
pub struct Pump {
    tap: Tap,
    answers: Arc<Answers>,
    events: tokio::sync::mpsc::UnboundedReceiver<DataEvent>,
}

impl Pump {
    /// Returns the pump, and the sender a chain's providers must be built with.
    #[must_use]
    pub fn new(
        client_id: ClientId,
        engine: EventSender<DataEvent>,
        answers: Arc<Answers>,
    ) -> (Self, EventSender<DataEvent>) {
        let (sender, events) = tokio::sync::mpsc::unbounded_channel();
        let tap = Tap::new(client_id, engine);

        (
            Self {
                tap,
                answers,
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
        while let Some(event) = self.events.recv().await {
            if self.answers.resolve(&event) {
                log::debug!("an answer reached the demand waiting for it");
            }

            if !self.tap.forward(event) {
                log::warn!("stopping the chain's boundary: the engine can no longer receive");

                return;
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

    const CHAIN: &str = "CHAIN";
    const PROVIDER: &str = "MOOMOO";

    fn chain_id() -> ClientId {
        ClientId::from(CHAIN)
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
            ClientId::from(PROVIDER),
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
    fn test_a_waiting_demand_is_woken_by_its_answer() {
        let answers = Answers::new();
        let correlation_id = UUID4::new();
        let mut waiting = answers.wait_for(correlation_id);

        assert!(answers.resolve(&answer(correlation_id)));
        assert!(waiting.try_recv().is_ok());
    }

    #[rstest]
    fn test_an_answer_nobody_is_waiting_for_wakes_nobody() {
        let answers = Answers::new();

        assert!(!answers.resolve(&answer(UUID4::new())));
    }

    /// The late answer to a demand that has already given up.
    #[rstest]
    fn test_a_forgotten_demand_is_not_woken() {
        let answers = Answers::new();
        let correlation_id = UUID4::new();
        let mut waiting = answers.wait_for(correlation_id);

        answers.forget(correlation_id);

        assert!(!answers.resolve(&answer(correlation_id)));
        assert!(waiting.try_recv().is_err());
    }

    #[rstest]
    fn test_a_demand_is_woken_once_even_if_an_answer_arrives_twice() {
        let answers = Answers::new();
        let correlation_id = UUID4::new();
        let _waiting = answers.wait_for(correlation_id);

        assert!(answers.resolve(&answer(correlation_id)));
        assert!(!answers.resolve(&answer(correlation_id)));
    }

    #[rstest]
    fn test_market_data_answers_nothing() {
        let answers = Answers::new();

        assert!(!answers.resolve(&market_data()));
    }

    #[tokio::test]
    async fn test_the_pump_carries_an_answer_to_the_engine_under_the_chains_identity() {
        let (engine, mut engine_events) = tokio::sync::mpsc::unbounded_channel();
        let (pump, providers) = Pump::new(
            chain_id(),
            EventSender::from(engine),
            Arc::new(Answers::new()),
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
    async fn test_the_pump_wakes_the_demand_its_answer_belongs_to() {
        let (engine, _engine_events) = tokio::sync::mpsc::unbounded_channel();
        let answers = Arc::new(Answers::new());
        let (pump, providers) = Pump::new(chain_id(), EventSender::from(engine), answers.clone());

        let correlation_id = UUID4::new();
        let mut waiting = answers.wait_for(correlation_id);

        providers.send(answer(correlation_id)).unwrap();
        drop(providers);

        pump.run().await;

        assert!(waiting.try_recv().is_ok());
    }

    /// A chain being torn down writes into a channel nobody reads, and must not keep writing.
    #[tokio::test]
    async fn test_the_pump_stops_once_the_engine_can_no_longer_receive() {
        let (engine, engine_events) = tokio::sync::mpsc::unbounded_channel();
        drop(engine_events);

        let (pump, providers) = Pump::new(
            chain_id(),
            EventSender::from(engine),
            Arc::new(Answers::new()),
        );

        providers.send(market_data()).unwrap();
        drop(providers);

        tokio::time::timeout(Duration::from_secs(5), pump.run())
            .await
            .expect("the pump should stop rather than write into a closed engine");
    }
}
