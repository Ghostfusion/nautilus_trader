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

//! The data client the engine sees, behind which a chain of providers serves it.
//!
//! The engine knows one client under one identity, and a chain adds exactly one thing to that: what
//! to do when the provider that should serve cannot. Every demand is passed to the first provider in
//! priority order that will take it, and everything the providers emit comes out under the chain's
//! identity, so that the engine is shown one client's worth of events.
//!
//! # What routing can decide, and what it cannot
//!
//! Routing is decided from what a provider says when it is asked, because the engine calls a client's
//! request method from its own loop and that call has to return rather than wait. A chain therefore
//! moves on when a provider will not take the demand, and it is the *answers* that say whether a
//! provider served what it took. A provider that takes a demand and then says nothing leaves that
//! demand unanswered, and the chain learns it for the next demand rather than for the one already
//! gone.

use std::{cell::RefCell, fmt::Debug, rc::Rc, sync::Arc};

use anyhow::Context;
use async_trait::async_trait;
use nautilus_common::{
    cache::CacheView,
    clients::DataClient,
    clock::Clock,
    factories::{ClientConfig, DataClientFactory},
    live::{
        get_runtime,
        runner::{replace_data_event_sender, try_get_data_event_sender},
        sender::EventSender,
    },
    messages::{
        DataEvent,
        data::{RequestBars, RequestInstrument, RequestInstruments},
    },
};
use nautilus_core::UUID4;
use nautilus_model::identifiers::{ClientId, Venue};

use crate::{
    chain::{Provider, Served, Trace, run},
    failure::Failure,
    pump::{Answers, Pump},
};

/// One request, as a chain hands it from provider to provider.
#[derive(Debug, Clone)]
pub enum Request {
    /// A list of instruments for a venue.
    Instruments(RequestInstruments),
    /// One instrument.
    Instrument(RequestInstrument),
    /// Bars for one instrument and bar type.
    Bars(RequestBars),
}

impl Request {
    /// Returns the identifier the caller gave this request.
    ///
    /// This is what the engine matches an answer to a request by, and it is the caller's, so a chain
    /// carries it rather than minting one.
    #[must_use]
    pub fn request_id(&self) -> UUID4 {
        match self {
            Self::Instruments(request) => request.request_id,
            Self::Instrument(request) => request.request_id,
            Self::Bars(request) => request.request_id,
        }
    }

    /// Returns this request addressed to `client_id`, which is the chain.
    ///
    /// A provider builds the identity of its answer from the request when the request names one, so
    /// addressing the request to the chain is what makes even the provider's own answer about the
    /// chain rather than about whoever happened to serve it. The boundary rewrites the identity
    /// regardless, so this is not what the engine's view rests on.
    #[must_use]
    pub fn addressed_to(&self, client_id: ClientId) -> Self {
        match self {
            Self::Instruments(request) => {
                let mut request = request.clone();
                request.client_id = Some(client_id);

                Self::Instruments(request)
            }
            Self::Instrument(request) => {
                let mut request = request.clone();
                request.client_id = Some(client_id);

                Self::Instrument(request)
            }
            Self::Bars(request) => {
                let mut request = request.clone();
                request.client_id = Some(client_id);

                Self::Bars(request)
            }
        }
    }
}

/// One provider in a chain, as a chain is assembled from it.
///
/// A leg carries what would have built its client on its own, because a chain is assembled rather
/// than configured: a trait object is not a configuration file, so the chain is put together where
/// the factories are rather than deserialized somewhere else.
#[derive(Debug)]
pub struct Leg {
    name: ClientId,
    factory: Box<dyn DataClientFactory>,
    config: Box<dyn ClientConfig>,
}

impl Leg {
    /// Creates a leg that builds its client through `factory`.
    #[must_use]
    pub fn new(
        name: ClientId,
        factory: Box<dyn DataClientFactory>,
        config: Box<dyn ClientConfig>,
    ) -> Self {
        Self {
            name,
            factory,
            config,
        }
    }

    /// Returns what this provider is called, which is the name a trace reports.
    #[must_use]
    pub fn name(&self) -> ClientId {
        self.name
    }
}

/// A leg whose client has been built, which is what the chain asks.
struct Built {
    name: ClientId,
    client: Box<dyn DataClient>,
}

impl Provider for Built {
    type Demand = Request;
    type Answer = ClientId;

    fn id(&self) -> ClientId {
        self.name
    }

    fn serve(&mut self, demand: &Request) -> Result<Self::Answer, Failure> {
        let taken = match demand {
            Request::Instruments(request) => self.client.request_instruments(request.clone()),
            Request::Instrument(request) => self.client.request_instrument(request.clone()),
            Request::Bars(request) => self.client.request_bars(request.clone()),
        };

        match taken {
            Ok(()) => Ok(self.name),
            // A client whose request call fails has not taken the demand: nothing was handed over, so
            // there is nothing on the other end for this provider. That is a gap rather than a
            // transient condition, and a chain moves on rather than asking the same provider again.
            Err(e) => Err(Failure::Unreachable(e.to_string())),
        }
    }
}

/// Installs a chain's boundary on this thread for as long as it lives.
///
/// A provider takes the data event sender it finds when it is constructed, so the boundary has to be
/// the sender in place for exactly as long as the legs are being built - and no longer, because
/// anything else built on this thread afterwards must still be heard by the engine directly.
#[derive(Debug)]
struct Boundary {
    previous: EventSender<DataEvent>,
}

impl Boundary {
    fn install(boundary: EventSender<DataEvent>, previous: EventSender<DataEvent>) -> Self {
        replace_data_event_sender(boundary);

        Self { previous }
    }
}

impl Drop for Boundary {
    fn drop(&mut self) {
        replace_data_event_sender(self.previous.clone());
    }
}

/// A data client that is a chain of providers.
pub struct CompositeDataClient {
    client_id: ClientId,
    legs: RefCell<Vec<Built>>,
    pump: Option<Pump>,
}

impl CompositeDataClient {
    /// Builds a chain of `legs`, in priority order, answering as `client_id`.
    ///
    /// The legs are built here, through their own factories, and they are built with the boundary in
    /// place: a provider built while the engine's own sender is installed would write past the chain,
    /// and nothing about it would be observed.
    ///
    /// # Errors
    ///
    /// Returns an error when no data event sender is installed on this thread, or when a leg cannot
    /// be built.
    pub fn new(
        client_id: ClientId,
        legs: Vec<Leg>,
        cache: &CacheView,
        clock: &Rc<RefCell<dyn Clock>>,
    ) -> anyhow::Result<Self> {
        let engine = try_get_data_event_sender()
            .context("a chain must be built where the engine's data event sender is installed")?;
        let (pump, boundary) = Pump::new(client_id, engine.clone(), Arc::new(Answers::new()));

        let mut built = Vec::with_capacity(legs.len());

        {
            let _boundary = Boundary::install(boundary, engine);

            for leg in legs {
                let client = leg.factory.create(
                    leg.name.as_str(),
                    leg.config.as_ref(),
                    cache.clone(),
                    Rc::clone(clock),
                )?;

                built.push(Built {
                    name: leg.name,
                    client,
                });
            }
        }

        Ok(Self {
            client_id,
            legs: RefCell::new(built),
            pump: Some(pump),
        })
    }

    /// Passes one demand to the first provider that will take it.
    fn route(&self, request: &Request) -> anyhow::Result<()> {
        let addressed = request.addressed_to(self.client_id);
        let mut trace = Trace::default();

        match run(
            self.legs.borrow_mut().as_mut_slice(),
            &addressed,
            &mut trace,
        ) {
            Served::Answered(provider) => {
                log::debug!("{provider} took the demand");

                Ok(())
            }
            Served::Failed(failure) => {
                for attempt in trace.attempts() {
                    log::warn!("{} was asked and could not serve", attempt.provider);
                }

                anyhow::bail!("no provider in the chain took the demand: {failure}")
            }
            Served::Unconfigured => anyhow::bail!("the chain has no providers"),
        }
    }
}

impl Debug for CompositeDataClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // A client is not printable, so the legs are named rather than printed.
        let providers = self
            .legs
            .borrow()
            .iter()
            .map(|leg| leg.name)
            .collect::<Vec<_>>();

        f.debug_struct(stringify!(CompositeDataClient))
            .field("client_id", &self.client_id)
            .field("providers", &providers)
            .finish_non_exhaustive()
    }
}

#[async_trait(?Send)]
impl DataClient for CompositeDataClient {
    fn client_id(&self) -> ClientId {
        self.client_id
    }

    /// A chain is not a venue.
    ///
    /// Its providers may serve different venues, and a caller reaches them by naming the instrument
    /// rather than the venue.
    fn venue(&self) -> Option<Venue> {
        None
    }

    fn start(&mut self) -> anyhow::Result<()> {
        if let Some(pump) = self.pump.take() {
            get_runtime().spawn(pump.run());
        }

        for leg in self.legs.get_mut() {
            leg.client.start()?;
        }

        Ok(())
    }

    fn stop(&mut self) -> anyhow::Result<()> {
        for leg in self.legs.get_mut() {
            leg.client.stop()?;
        }

        Ok(())
    }

    fn reset(&mut self) -> anyhow::Result<()> {
        for leg in self.legs.get_mut() {
            leg.client.reset()?;
        }

        Ok(())
    }

    fn dispose(&mut self) -> anyhow::Result<()> {
        for leg in self.legs.get_mut() {
            leg.client.dispose()?;
        }

        Ok(())
    }

    /// A chain is connected while any of its providers is.
    fn is_connected(&self) -> bool {
        self.legs
            .borrow()
            .iter()
            .any(|leg| leg.client.is_connected())
    }

    fn is_disconnected(&self) -> bool {
        !self.is_connected()
    }

    /// Connects every leg, and fails only when no leg can serve.
    ///
    /// A provider that will not connect is the reason a chain exists, so one leg failing is not the
    /// chain failing; a chain with nothing connected is.
    async fn connect(&mut self) -> anyhow::Result<()> {
        let mut last = None;

        for leg in self.legs.get_mut() {
            if let Err(e) = leg.client.connect().await {
                log::warn!("{} will not connect: {e}", leg.name);

                last = Some(e);
            }
        }

        match (self.is_connected(), last) {
            (true, _) => Ok(()),
            (false, Some(e)) => Err(e),
            (false, None) => anyhow::bail!("the chain has no providers"),
        }
    }

    async fn disconnect(&mut self) -> anyhow::Result<()> {
        for leg in self.legs.get_mut() {
            leg.client.disconnect().await?;
        }

        Ok(())
    }

    fn request_instruments(&self, request: RequestInstruments) -> anyhow::Result<()> {
        self.route(&Request::Instruments(request))
    }

    fn request_instrument(&self, request: RequestInstrument) -> anyhow::Result<()> {
        self.route(&Request::Instrument(request))
    }

    fn request_bars(&self, request: RequestBars) -> anyhow::Result<()> {
        self.route(&Request::Bars(request))
    }
}

/// Builds a chain out of the legs it was assembled from.
///
/// A chain has no configuration file of its own: each leg carries the configuration that would have
/// built its client directly, so the legs are consumed by the one client this builds.
#[derive(Debug)]
pub struct CompositeDataClientFactory {
    name: String,
    legs: RefCell<Vec<Leg>>,
}

impl CompositeDataClientFactory {
    /// Creates a factory that builds a chain called `name` from `legs`, in priority order.
    #[must_use]
    pub fn new(name: &str, legs: Vec<Leg>) -> Self {
        Self {
            name: name.to_string(),
            legs: RefCell::new(legs),
        }
    }
}

impl DataClientFactory for CompositeDataClientFactory {
    /// Builds the chain the legs describe.
    ///
    /// The configuration given here is not read: a chain is assembled from legs rather than
    /// deserialized, so there is nothing about it that a configuration could say.
    ///
    /// # Errors
    ///
    /// Returns an error when the legs have already been consumed, or when the chain cannot be built.
    fn create(
        &self,
        name: &str,
        _config: &dyn ClientConfig,
        cache: CacheView,
        clock: Rc<RefCell<dyn Clock>>,
    ) -> anyhow::Result<Box<dyn DataClient>> {
        let legs = std::mem::take(&mut *self.legs.borrow_mut());

        anyhow::ensure!(!legs.is_empty(), "the chain has already been built");

        let client = CompositeDataClient::new(ClientId::from(name), legs, &cache, &clock)?;

        Ok(Box::new(client))
    }

    fn name(&self) -> &str {
        &self.name
    }

    /// A chain reads no configuration of its own.
    fn config_type(&self) -> &'static str {
        "None"
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use nautilus_common::{
        cache::Cache,
        clock::VirtualClock,
        live::runner::{get_data_event_sender, replace_data_event_sender},
        messages::data::{BarsResponse, DataResponse},
    };
    use nautilus_core::UnixNanos;
    use nautilus_model::{
        data::{BarSpecification, BarType},
        enums::{AggregationSource, BarAggregation, PriceType},
        identifiers::InstrumentId,
        types::Price,
    };
    use rstest::rstest;

    use super::*;
    use crate::testing::{FakeConfig, FakeDataClientFactory};

    const CHAIN: &str = "CHAIN";

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

    fn bars_request() -> RequestBars {
        RequestBars::new(
            bar_type(),
            None,
            None,
            None,
            Some(chain_id()),
            UUID4::new(),
            UnixNanos::default(),
            None,
        )
    }

    fn instrument_request() -> RequestInstrument {
        RequestInstrument::new(
            InstrumentId::from("AAPL.US"),
            None,
            None,
            Some(chain_id()),
            UUID4::new(),
            UnixNanos::default(),
            None,
        )
    }

    fn instruments_request() -> RequestInstruments {
        RequestInstruments::new(
            None,
            None,
            Some(chain_id()),
            Some(Venue::from("US")),
            UUID4::new(),
            UnixNanos::default(),
            None,
        )
    }

    fn leg(factory: &FakeDataClientFactory) -> Leg {
        Leg::new(
            ClientId::from(factory.name()),
            Box::new(factory.clone()),
            Box::new(FakeConfig),
        )
    }

    fn provider(name: &str, mark: &str) -> FakeDataClientFactory {
        FakeDataClientFactory::new(name).marked(Price::from(mark))
    }

    /// Returns the two legs of a chain whose engine events go to `events`.
    fn chain(legs: Vec<Leg>) -> CompositeDataClient {
        let cache = CacheView::new(Rc::new(RefCell::new(Cache::default())));
        let clock: Rc<RefCell<dyn Clock>> = Rc::new(RefCell::new(VirtualClock::new()));

        CompositeDataClient::new(chain_id(), legs, &cache, &clock)
            .expect("the chain should be built where the test installed a data event sender")
    }

    /// Waits for the engine's next event, or fails the test rather than hanging the suite.
    fn next_event(events: &mut tokio::sync::mpsc::UnboundedReceiver<DataEvent>) -> DataEvent {
        let deadline = Instant::now() + Duration::from_secs(5);

        loop {
            if let Ok(event) = events.try_recv() {
                return event;
            }

            assert!(
                Instant::now() < deadline,
                "the engine was not given an answer"
            );

            std::thread::sleep(Duration::from_millis(1));
        }
    }

    /// The engine's data event sender on this thread, replaced with a test's channel.
    fn install_engine() -> tokio::sync::mpsc::UnboundedReceiver<DataEvent> {
        let (sender, events) = tokio::sync::mpsc::unbounded_channel();
        replace_data_event_sender(EventSender::from(sender));

        events
    }

    #[rstest]
    fn test_the_primary_serves_the_demand_and_the_secondary_is_not_needed() {
        let mut events = install_engine();
        let primary = provider("PRIMARY", "100.00");
        let secondary = provider("SECONDARY", "200.00");
        let mut chain = chain(vec![leg(&primary), leg(&secondary)]);

        chain.start().unwrap();

        let request = bars_request();
        chain.request_bars(request.clone()).unwrap();

        let DataEvent::Response(DataResponse::Bars(answer)) = next_event(&mut events) else {
            panic!("expected a bar response");
        };

        assert_eq!(answer.correlation_id, request.request_id);
        assert_eq!(answer.client_id, chain_id());
        assert_eq!(answer.data[0].close, Price::from("100.00"));
        assert_eq!(primary.asked().len(), 1, "the primary was asked once");
        assert!(secondary.asked().is_empty(), "the secondary was not needed");
    }

    /// A provider that will not take the demand is what the next provider is for.
    #[rstest]
    fn test_the_secondary_serves_when_the_primary_will_not_take_the_demand() {
        let mut events = install_engine();
        let primary = FakeDataClientFactory::new("PRIMARY").refusing();
        let secondary = provider("SECONDARY", "200.00");
        let mut chain = chain(vec![leg(&primary), leg(&secondary)]);

        chain.start().unwrap();

        let request = bars_request();
        chain.request_bars(request.clone()).unwrap();

        let DataEvent::Response(DataResponse::Bars(answer)) = next_event(&mut events) else {
            panic!("expected a bar response");
        };

        assert_eq!(answer.correlation_id, request.request_id);
        assert_eq!(answer.client_id, chain_id());
        assert_eq!(
            answer.data[0].close,
            Price::from("200.00"),
            "the answer is the secondary's"
        );
        assert_eq!(
            primary.asked().len(),
            1,
            "a provider that would not take the demand is not asked again"
        );
    }

    #[rstest]
    fn test_the_universe_reaches_the_engine_under_the_chains_identity() {
        let mut events = install_engine();
        let primary = provider("PRIMARY", "100.00");
        let mut chain = chain(vec![leg(&primary)]);

        chain.start().unwrap();

        let request = instruments_request();
        chain.request_instruments(request.clone()).unwrap();

        let DataEvent::Response(DataResponse::Instruments(answer)) = next_event(&mut events) else {
            panic!("expected an instruments response");
        };

        assert_eq!(answer.correlation_id, request.request_id);
        assert_eq!(answer.client_id, chain_id());
        assert_eq!(answer.data.len(), 1);
    }

    /// Every request kind takes the same road, and this is the third of them.
    #[rstest]
    fn test_the_secondary_serves_an_instrument_when_the_primary_will_not() {
        let mut events = install_engine();
        let primary = FakeDataClientFactory::new("PRIMARY").refusing();
        let secondary = provider("SECONDARY", "200.00");
        let mut chain = chain(vec![leg(&primary), leg(&secondary)]);

        chain.start().unwrap();

        let request = instrument_request();
        chain.request_instrument(request.clone()).unwrap();

        let DataEvent::Response(DataResponse::Instrument(answer)) = next_event(&mut events) else {
            panic!("expected an instrument response");
        };

        assert_eq!(answer.correlation_id, request.request_id);
        assert_eq!(answer.client_id, chain_id());
        assert_eq!(answer.instrument_id, request.instrument_id);
    }

    #[rstest]
    fn test_a_demand_that_nobody_will_take_is_reported_rather_than_lost() {
        let _events = install_engine();
        let primary = FakeDataClientFactory::new("PRIMARY").refusing();
        let secondary = FakeDataClientFactory::new("SECONDARY").refusing();
        let mut chain = chain(vec![leg(&primary), leg(&secondary)]);

        chain.start().unwrap();

        let error = chain
            .request_bars(bars_request())
            .expect_err("no provider took the demand");

        assert!(error.to_string().contains("no provider"));
        assert_eq!(primary.asked().len(), 1);
        assert_eq!(secondary.asked().len(), 1, "the chain tried both, in order");
    }

    #[rstest]
    fn test_a_chain_with_no_providers_can_serve_nothing() {
        let _events = install_engine();
        let chain = chain(Vec::new());

        assert!(chain.request_bars(bars_request()).is_err());
        assert!(chain.is_disconnected());
    }

    /// A chain is connected while any of its providers is, and not once they are all disconnected.
    #[tokio::test]
    async fn test_a_chain_is_connected_while_any_provider_is() {
        let _events = install_engine();
        let primary = provider("PRIMARY", "100.00");
        let secondary = provider("SECONDARY", "200.00");
        let mut chain = chain(vec![leg(&primary), leg(&secondary)]);

        assert!(chain.is_connected());

        chain.disconnect().await.unwrap();

        assert!(chain.is_disconnected());
    }

    /// A chain takes the boundary down again once its legs are built, so that anything else built on
    /// the same thread is still heard by the engine directly.
    #[rstest]
    fn test_the_boundary_is_not_left_installed_after_the_legs_are_built() {
        let mut events = install_engine();
        let primary = provider("PRIMARY", "100.00");

        let _chain = chain(vec![leg(&primary)]);

        // Whatever is written to this thread's sender now goes to the engine as it is: an identity
        // the boundary would have rewritten arrives untouched.
        let response = DataResponse::Bars(BarsResponse::new(
            UUID4::new(),
            ClientId::from("PRIMARY"),
            bar_type(),
            Vec::new(),
            None,
            None,
            UnixNanos::default(),
            None,
        ));

        get_data_event_sender()
            .send(DataEvent::Response(response))
            .unwrap();

        let DataEvent::Response(DataResponse::Bars(delivered)) = next_event(&mut events) else {
            panic!("expected a bar response");
        };

        assert_eq!(delivered.client_id, ClientId::from("PRIMARY"));
    }
}
