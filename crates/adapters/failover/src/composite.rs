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
//! demand unanswered, and the chain learns it from the boundary rather than from the call - which is
//! what the health record is for, and why it is read here rather than reported.
//!
//! # Stickiness
//!
//! A demand starts where the chain last answered, not at the highest-priority provider that is
//! available. A provider that has just recovered is not immediately trusted with the next demand, and
//! a chain that moved to a fallback stays there until the fallback fails in turn or until it is
//! deliberately brought back: moving back the moment the primary answers again would spend a request
//! on every oscillation and, for a metered provider, quota with it.

use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    fmt::Debug,
    rc::Rc,
    sync::Arc,
    time::Instant,
};

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
        data::{
            RequestBars, RequestInstrument, RequestInstruments, SubscribeBars, SubscribeQuotes,
            SubscribeTrades, UnsubscribeBars, UnsubscribeQuotes, UnsubscribeTrades,
        },
    },
};
use nautilus_core::UUID4;
use nautilus_model::{
    data::{CustomData, Data, DataType},
    identifiers::{ClientId, Venue},
};

use crate::{
    chain::{Provider, Served, Trace, run},
    failure::Failure,
    health::{Health, Policy, Snapshot},
    pump::{Pending, Pump},
    streaming::{
        CONTINUITY_DATA_TYPE, Continuity, Stream, Streaming, Subscription, Unsubscription,
    },
};

/// One demand, as a chain hands it from provider to provider.
///
/// A request is answered and over; a subscription continues. Both are demands, and both are passed
/// along the same chain, but only a request has an answer to wait for.
#[derive(Debug, Clone)]
pub enum Demand {
    /// A list of instruments for a venue.
    Instruments(RequestInstruments),
    /// One instrument.
    Instrument(RequestInstrument),
    /// Bars for one instrument and bar type.
    Bars(RequestBars),
    /// A stream for the chain to hold.
    Subscribe(Subscription),
}

impl Demand {
    /// Returns the identifier the caller gave this demand, when it is one that is answered.
    ///
    /// This is what the engine matches an answer to a request by, and it is the caller's, so a chain
    /// carries it rather than minting one. A subscription is not answered by an identifier: it is
    /// held, and what it delivers arrives as data.
    #[must_use]
    pub fn request_id(&self) -> Option<UUID4> {
        match self {
            Self::Instruments(request) => Some(request.request_id),
            Self::Instrument(request) => Some(request.request_id),
            Self::Bars(request) => Some(request.request_id),
            Self::Subscribe(_) => None,
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
            Self::Subscribe(subscription) => Self::Subscribe(subscription.addressed_to(client_id)),
        }
    }
}

/// Asks a client for a stream, whichever kind it is.
fn subscribe(client: &mut dyn DataClient, command: &Subscription) -> anyhow::Result<()> {
    match command {
        Subscription::Bars(command) => client.subscribe_bars(command.clone()),
        Subscription::Quotes(command) => client.subscribe_quotes(command.clone()),
        Subscription::Trades(command) => client.subscribe_trades(command.clone()),
    }
}

/// Takes a stream away from a client, whichever kind it is.
fn unsubscribe(client: &mut dyn DataClient, command: &Unsubscription) -> anyhow::Result<()> {
    match command {
        Unsubscription::Bars(command) => client.unsubscribe_bars(command),
        Unsubscription::Quotes(command) => client.unsubscribe_quotes(command),
        Unsubscription::Trades(command) => client.unsubscribe_trades(command),
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
    /// Where this provider stands in the chain's priority order.
    priority: usize,
    name: ClientId,
    client: Box<dyn DataClient>,
    health: Arc<Health>,
    pending: Arc<Pending>,
}

impl Provider for Built {
    type Demand = Demand;
    type Answer = ClientId;

    fn id(&self) -> ClientId {
        self.name
    }

    fn serve(&mut self, demand: &Demand) -> Result<Self::Answer, Failure> {
        self.health.asked(&self.name);

        let taken = match demand {
            Demand::Instruments(request) => self.client.request_instruments(request.clone()),
            Demand::Instrument(request) => self.client.request_instrument(request.clone()),
            Demand::Bars(request) => self.client.request_bars(request.clone()),
            Demand::Subscribe(subscription) => subscribe(self.client.as_mut(), subscription),
        };

        match taken {
            Ok(()) => {
                // A provider that has taken a request owes an answer for it, and the boundary is the
                // only thing that can notice that it never arrives. A subscription is not answered
                // that way: what it delivers arrives as data, and a stream that stops arriving is not
                // an unanswered request.
                if let Some(request_id) = demand.request_id() {
                    let due = Instant::now() + self.health.policy().answer_deadline;
                    self.pending.expect(request_id, self.name, due);
                }

                Ok(self.name)
            }
            // A client whose request call fails has not taken the demand: nothing was handed over, so
            // there is nothing on the other end for this provider. That is a gap rather than a
            // transient condition, and a chain moves on rather than asking the same provider again.
            Err(e) => {
                let failure = Failure::Unreachable(e.to_string());
                self.health.failed(&self.name, &failure, Instant::now());

                Err(failure)
            }
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
    /// The provider the chain is currently being served by, when one has served.
    current: Cell<Option<ClientId>>,
    health: Arc<Health>,
    streaming: Streaming,
    /// Where the chain writes what is its own rather than a provider's, such as a move of source.
    engine: EventSender<DataEvent>,
    clock: Rc<RefCell<dyn Clock>>,
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
        policy: Policy,
        cache: &CacheView,
        clock: &Rc<RefCell<dyn Clock>>,
    ) -> anyhow::Result<Self> {
        let engine = try_get_data_event_sender()
            .context("a chain must be built where the engine's data event sender is installed")?;
        let health = Arc::new(Health::new(policy));
        let pending = Arc::new(Pending::new());
        let (pump, boundary) = Pump::new(
            client_id,
            engine.clone(),
            Arc::clone(&pending),
            Arc::clone(&health),
        );

        let mut built = Vec::with_capacity(legs.len());

        {
            let _boundary = Boundary::install(boundary, engine.clone());

            for (priority, leg) in legs.into_iter().enumerate() {
                let client = leg.factory.create(
                    leg.name.as_str(),
                    leg.config.as_ref(),
                    cache.clone(),
                    Rc::clone(clock),
                )?;

                built.push(Built {
                    priority,
                    name: leg.name,
                    client,
                    health: Arc::clone(&health),
                    pending: Arc::clone(&pending),
                });
            }
        }

        Ok(Self {
            client_id,
            legs: RefCell::new(built),
            current: Cell::new(None),
            health,
            streaming: Streaming::new(),
            engine,
            clock: Rc::clone(clock),
            pump: Some(pump),
        })
    }

    /// Returns what the chain knows about every provider it has asked.
    ///
    /// This is the chain's account of itself: which providers it asked, how often, why they failed,
    /// how many hops each one caused, and whether it has been set aside. A chain that keeps this to
    /// itself keeps its outages to itself.
    #[must_use]
    pub fn health(&self) -> Vec<Snapshot> {
        self.health.snapshots()
    }

    /// Returns what the chain is streaming, and which provider serves each stream.
    #[must_use]
    pub fn streamed(&self) -> Vec<(Stream, ClientId)> {
        self.streaming.held()
    }

    /// Passes one demand to the first provider that will take it.
    fn route(&self, request: &Demand) -> anyhow::Result<()> {
        let now = Instant::now();
        let current = self.current.get();
        let addressed = request.addressed_to(self.client_id);
        let mut legs = self.legs.borrow_mut();

        // A provider that has been set aside is not asked at all. A chain that asked a provider that
        // has just failed on every demand would not be failing over, it would be waiting.
        let admissible: HashMap<ClientId, bool> = legs
            .iter()
            .map(|leg| (leg.name, self.health.admit(&leg.name, now)))
            .collect();
        let asking = admissible.values().filter(|admitted| **admitted).count();

        // The demand starts where the chain last answered, and the rest keep their priority order, so
        // that a fallback is not displaced by a provider that has merely stopped failing.
        legs.sort_by_key(|leg| {
            (
                !admissible[&leg.name],
                Some(leg.name) != current,
                leg.priority,
            )
        });

        let mut trace = Trace::default();
        let served = run(&mut legs[..asking], &addressed, &mut trace);

        // A hop is counted against the provider that was left, once per demand. Two things are a hop:
        // the chain being served by a different provider than the one that served it last, which is
        // what a provider set aside for failing to answer produces, and a provider it gave up on
        // during the demand itself. An attempt repeated at the same provider is not a hop.
        let mut left: Vec<ClientId> = Vec::new();

        if let (Some(previous), Served::Answered(provider)) = (current, &served)
            && *provider != previous
        {
            left.push(previous);
        }

        for pair in trace.attempts().windows(2) {
            if pair[0].provider != pair[1].provider && !left.contains(&pair[0].provider) {
                left.push(pair[0].provider);
            }
        }

        for provider in left {
            self.health.hopped_from(&provider);
        }

        match served {
            Served::Answered(provider) => {
                self.current.set(Some(provider));
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

impl CompositeDataClient {
    /// Returns the providers the chain may ask, in the order it prefers them.
    ///
    /// The provider already being served from comes first, the priority order fills in behind it, and
    /// what has been set aside is left out - so that the chain has one answer to which provider serves
    /// it rather than one answer per kind of demand.
    fn preferred_providers(&self, now: Instant) -> Vec<ClientId> {
        let legs = self.legs.borrow();
        let mut providers: Vec<ClientId> = legs
            .iter()
            .filter(|leg| self.health.admit(&leg.name, now))
            .map(|leg| leg.name)
            .collect();

        if let Some(index) = self
            .current
            .get()
            .and_then(|current| providers.iter().position(|provider| *provider == current))
        {
            let current = providers.remove(index);
            providers.insert(0, current);
        }

        providers
    }

    /// Returns the provider the chain streams from, which is determined once for all of them.
    fn streaming_provider(&mut self) -> anyhow::Result<ClientId> {
        let now = Instant::now();

        let Some(serving) = self.streaming.provider() else {
            return self
                .preferred_providers(now)
                .into_iter()
                .next()
                .context("the chain has no provider that can stream");
        };

        if self.health.admit(&serving, now) {
            return Ok(serving);
        }

        // The provider that was streaming has failed, and everything it held moves with it. A stream
        // is never moved on its own: moving one instrument at a time would leave the chain streaming
        // from two providers at once, which is indistinguishable from a stream that is simply wrong.
        let to = self
            .preferred_providers(now)
            .into_iter()
            .find(|provider| *provider != serving)
            .context("the chain has no provider left to stream from")?;

        self.move_streams(&serving, to)?;

        Ok(to)
    }

    /// Moves every stream `from` holds to `to`, in the order they were taken.
    ///
    /// The old subscription is given up before the new one is taken, so that a stream is never served
    /// from two providers at once, and every move is written as a continuity event, so that a change
    /// of source is visible rather than silent.
    fn move_streams(&mut self, from: &ClientId, to: ClientId) -> anyhow::Result<()> {
        let reason = self.health.snapshot(from).last_failure.map_or_else(
            || "the provider stopped serving".to_string(),
            |failure| failure.to_string(),
        );

        for (stream, command) in self.streaming.held_by(from) {
            self.streaming.release(&stream);

            if let Err(e) = self.release_at(from, &command.released()) {
                // A provider that is being left may not take the unsubscription either. The stream
                // still moves, and what could not be given up is recorded rather than hidden.
                log::warn!("{from} would not give up {stream}: {e}");
            }

            let provider = match self.subscribe_at(to, &command) {
                Ok(()) => to,
                Err(e) => {
                    log::error!("{to} would not take {stream}: {e}");

                    // The stream is left where it was, which is worse than where it was going, so it
                    // goes back into the record as the failed provider's and is moved again later.
                    self.streaming.hold(stream, *from, command);

                    return Err(e);
                }
            };

            log::warn!("{stream} is now streamed from {provider}, and was served by {from}");
            self.announce(&stream, *from, provider, &reason);
        }

        self.current.set(Some(to));

        Ok(())
    }

    /// Asks one provider to take one stream.
    fn subscribe_at(&mut self, provider: ClientId, command: &Subscription) -> anyhow::Result<()> {
        let addressed = command.addressed_to(self.client_id);
        let leg = self.leg_mut(&provider)?;

        leg.serve(&Demand::Subscribe(addressed.clone()))
            .map_err(|failure| {
                anyhow::anyhow!("{provider} would not take the stream: {failure}")
            })?;

        self.streaming.hold(addressed.stream(), provider, addressed);

        Ok(())
    }

    /// Asks one provider to give one stream up.
    fn release_at(&mut self, provider: &ClientId, command: &Unsubscription) -> anyhow::Result<()> {
        let leg = self.leg_mut(provider)?;

        unsubscribe(leg.client.as_mut(), command)
    }

    /// Returns one provider, as the chain holds it.
    fn leg_mut(&mut self, provider: &ClientId) -> anyhow::Result<&mut Built> {
        self.legs
            .get_mut()
            .iter_mut()
            .find(|leg| leg.name == *provider)
            .with_context(|| format!("the chain has no provider called {provider}"))
    }

    /// Takes a stream, from the provider the chain is streaming from.
    fn subscribe(&mut self, subscription: &Subscription) -> anyhow::Result<()> {
        let stream = subscription.stream();

        // A stream the chain already holds is already being streamed: asking twice does not move it,
        // and it does not give one stream two sources.
        if let Some(provider) = self.streaming.provider_of(&stream) {
            log::debug!("{stream} is already streamed from {provider}");

            return Ok(());
        }

        // A provider that will not take the stream at all is a provider the chain leaves, and every
        // stream it holds leaves with it, so the attempt is repeated at the next provider rather than
        // repeated at that one.
        let attempts = self.legs.borrow().len().max(1);
        let mut last = None;

        for _ in 0..attempts {
            let provider = self.streaming_provider()?;

            match self.subscribe_at(provider, subscription) {
                Ok(()) => return Ok(()),
                Err(e) => {
                    log::warn!("{provider} would not take {stream}: {e}");
                    last = Some(e);
                }
            }
        }

        Err(last.unwrap_or_else(|| anyhow::anyhow!("no provider would take {stream}")))
    }

    /// Gives up a stream, at the provider that was serving it.
    fn unsubscribe(&mut self, command: &Unsubscription) -> anyhow::Result<()> {
        let stream = command.stream();

        let Some(provider) = self.streaming.release(&stream) else {
            log::debug!("{stream} is not streamed by the chain");

            return Ok(());
        };

        self.release_at(&provider, command)
    }

    /// Brings the chain back to the provider it prefers, deliberately.
    ///
    /// Fail-back is not a side effect of a provider recovering. It means giving up every stream a
    /// fallback holds and taking them again at the preferred provider, which is a fresh subscription
    /// at a provider that may itself refuse to release the old one for a while - so it is asked for at
    /// a checkpoint rather than done the moment it becomes possible.
    ///
    /// # Errors
    ///
    /// Returns an error when no provider is available, or when a stream cannot be moved.
    pub fn checkpoint(&mut self) -> anyhow::Result<()> {
        let preferred = self
            .legs
            .borrow()
            .iter()
            .find(|leg| self.health.admit(&leg.name, Instant::now()))
            .map(|leg| leg.name)
            .context("the chain has no provider available")?;

        self.current.set(Some(preferred));

        let Some(serving) = self.streaming.provider() else {
            return Ok(());
        };

        if serving == preferred {
            return Ok(());
        }

        self.move_streams(&serving, preferred)
    }

    /// Writes the event that says a stream changed source.
    ///
    /// This is written by the chain about itself rather than by a provider, so it is written to the
    /// engine directly: there is no provider's identity to rewrite, and the event is the chain's.
    fn announce(&self, stream: &Stream, from: ClientId, to: ClientId, reason: &str) {
        let now = self.clock.borrow().timestamp_ns();
        let continuity = Continuity {
            stream: stream.to_string(),
            from,
            to,
            reason: reason.to_string(),
            ts_event: now,
            ts_init: now,
        };
        let data = Data::Custom(CustomData::new(
            Arc::new(continuity),
            DataType::new(CONTINUITY_DATA_TYPE, None, None),
        ));

        if let Err(e) = self.engine.send(DataEvent::Data(data)) {
            log::error!("cannot write the continuity event for {stream}: {e}");
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
            .field("current", &self.current)
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
        self.route(&Demand::Instruments(request))
    }

    fn request_instrument(&self, request: RequestInstrument) -> anyhow::Result<()> {
        self.route(&Demand::Instrument(request))
    }

    fn request_bars(&self, request: RequestBars) -> anyhow::Result<()> {
        self.route(&Demand::Bars(request))
    }

    fn subscribe_bars(&mut self, cmd: SubscribeBars) -> anyhow::Result<()> {
        self.subscribe(&Subscription::Bars(cmd))
    }

    fn subscribe_quotes(&mut self, cmd: SubscribeQuotes) -> anyhow::Result<()> {
        self.subscribe(&Subscription::Quotes(cmd))
    }

    fn subscribe_trades(&mut self, cmd: SubscribeTrades) -> anyhow::Result<()> {
        self.subscribe(&Subscription::Trades(cmd))
    }

    fn unsubscribe_bars(&mut self, cmd: &UnsubscribeBars) -> anyhow::Result<()> {
        self.unsubscribe(&Unsubscription::Bars(cmd.clone()))
    }

    fn unsubscribe_quotes(&mut self, cmd: &UnsubscribeQuotes) -> anyhow::Result<()> {
        self.unsubscribe(&Unsubscription::Quotes(cmd.clone()))
    }

    fn unsubscribe_trades(&mut self, cmd: &UnsubscribeTrades) -> anyhow::Result<()> {
        self.unsubscribe(&Unsubscription::Trades(cmd.clone()))
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
    policy: Policy,
}

impl CompositeDataClientFactory {
    /// Creates a factory that builds a chain called `name` from `legs`, in priority order.
    #[must_use]
    pub fn new(name: &str, legs: Vec<Leg>) -> Self {
        Self {
            name: name.to_string(),
            legs: RefCell::new(legs),
            policy: Policy::default(),
        }
    }

    /// Returns this factory building a chain that sets its providers aside by `policy`.
    #[must_use]
    pub fn with_policy(mut self, policy: Policy) -> Self {
        self.policy = policy;

        self
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

        let client =
            CompositeDataClient::new(ClientId::from(name), legs, self.policy, &cache, &clock)?;

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
    use std::time::{Duration, Instant as StdInstant};

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
    use crate::testing::{FakeCall, FakeConfig, FakeDataClientFactory};

    const CHAIN: &str = "CHAIN";

    fn chain_id() -> ClientId {
        ClientId::from(CHAIN)
    }

    fn id(name: &str) -> ClientId {
        ClientId::from(name)
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

    fn subscribe_bars_request(symbol: &str) -> SubscribeBars {
        SubscribeBars::new(
            BarType::new(
                InstrumentId::from(symbol),
                BarSpecification::new(1, BarAggregation::Day, PriceType::Last),
                AggregationSource::External,
            ),
            Some(chain_id()),
            None,
            UUID4::new(),
            UnixNanos::default(),
            None,
            None,
        )
    }

    fn unsubscribe_bars_request(symbol: &str) -> UnsubscribeBars {
        UnsubscribeBars::new(
            BarType::new(
                InstrumentId::from(symbol),
                BarSpecification::new(1, BarAggregation::Day, PriceType::Last),
                AggregationSource::External,
            ),
            Some(chain_id()),
            None,
            UUID4::new(),
            UnixNanos::default(),
            None,
            None,
        )
    }

    /// Reads the next continuity event the engine was given, ignoring anything else it was given.
    fn continuity_of(events: &mut tokio::sync::mpsc::UnboundedReceiver<DataEvent>) -> Continuity {
        loop {
            let DataEvent::Data(Data::Custom(custom)) = next_event(events) else {
                continue;
            };

            return custom
                .data
                .as_any()
                .downcast_ref::<Continuity>()
                .expect("the chain writes its continuity as a continuity event")
                .clone();
        }
    }

    fn journal() -> Rc<RefCell<Vec<FakeCall>>> {
        Rc::new(RefCell::new(Vec::new()))
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

    fn chain_with(policy: Policy, legs: Vec<Leg>) -> CompositeDataClient {
        let cache = CacheView::new(Rc::new(RefCell::new(Cache::default())));
        let clock: Rc<RefCell<dyn Clock>> = Rc::new(RefCell::new(VirtualClock::new()));

        CompositeDataClient::new(chain_id(), legs, policy, &cache, &clock)
            .expect("the chain should be built where the test installed a data event sender")
    }

    fn chain(legs: Vec<Leg>) -> CompositeDataClient {
        chain_with(Policy::default(), legs)
    }

    /// Returns the engine's data event sender installed on this thread, replaced with a test channel.
    fn install_engine() -> tokio::sync::mpsc::UnboundedReceiver<DataEvent> {
        let (sender, events) = tokio::sync::mpsc::unbounded_channel();
        replace_data_event_sender(EventSender::from(sender));

        events
    }

    /// Waits for the engine's next event, or fails the test rather than hanging the suite.
    fn next_event(events: &mut tokio::sync::mpsc::UnboundedReceiver<DataEvent>) -> DataEvent {
        let deadline = StdInstant::now() + Duration::from_secs(5);

        loop {
            if let Ok(event) = events.try_recv() {
                return event;
            }

            assert!(
                StdInstant::now() < deadline,
                "the engine was not given an answer"
            );

            std::thread::sleep(Duration::from_millis(1));
        }
    }

    /// Returns one provider's account of itself.
    fn snapshot(client: &CompositeDataClient, name: &str) -> Snapshot {
        client
            .health()
            .into_iter()
            .find(|snapshot| snapshot.provider == id(name))
            .expect("the provider should have been asked")
    }

    /// Waits for a provider to be set aside, or fails the test rather than hanging the suite.
    fn await_set_aside(client: &CompositeDataClient, name: &str) -> Snapshot {
        let deadline = StdInstant::now() + Duration::from_secs(5);

        loop {
            let snapshot = snapshot(client, name);

            if snapshot.is_open {
                return snapshot;
            }

            assert!(
                StdInstant::now() < deadline,
                "the provider was not held to account for an answer that never came"
            );

            std::thread::sleep(Duration::from_millis(2));
        }
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

    /// The exit of this step: an outage costs one hop and one request, not one per demand, and the
    /// chain can say what happened and why.
    #[rstest]
    fn test_a_provider_that_will_not_take_a_demand_is_not_asked_again() {
        let mut events = install_engine();
        let primary = FakeDataClientFactory::new("PRIMARY").refusing();
        let secondary = provider("SECONDARY", "200.00");
        let mut chain = chain(vec![leg(&primary), leg(&secondary)]);

        chain.start().unwrap();

        let first = bars_request();
        chain.request_bars(first.clone()).unwrap();

        let DataEvent::Response(DataResponse::Bars(answer)) = next_event(&mut events) else {
            panic!("expected a bar response");
        };

        assert_eq!(answer.correlation_id, first.request_id);
        assert_eq!(answer.data[0].close, Price::from("200.00"));

        let second = bars_request();
        chain.request_bars(second.clone()).unwrap();

        let DataEvent::Response(DataResponse::Bars(answer)) = next_event(&mut events) else {
            panic!("expected a bar response");
        };

        assert_eq!(answer.correlation_id, second.request_id);
        assert_eq!(answer.data[0].close, Price::from("200.00"));
        assert_eq!(
            primary.asked().len(),
            1,
            "the provider that failed was asked once, not once per demand"
        );
        assert_eq!(secondary.asked().len(), 2);

        let snapshot = snapshot(&chain, "PRIMARY");

        assert!(snapshot.is_open);
        assert_eq!(snapshot.attempts, 1);
        assert_eq!(snapshot.failures, 1);
        assert_eq!(snapshot.hops, 1, "the chain moved away from it once");
        assert!(
            matches!(snapshot.last_failure, Some(Failure::Unreachable(_))),
            "the reason is kept, was {:?}",
            snapshot.last_failure
        );
    }

    /// A demand stays where it was served. The cooldown here is nothing at all, so the primary is
    /// available for the second demand and would be asked by a chain that only followed priority.
    #[rstest]
    fn test_a_demand_stays_with_the_provider_that_served_it() {
        let mut events = install_engine();
        let policy = Policy {
            cooldown: Duration::ZERO,
            ..Policy::default()
        };
        let primary = FakeDataClientFactory::new("PRIMARY").refusing();
        let secondary = provider("SECONDARY", "200.00");
        let mut chain = chain_with(policy, vec![leg(&primary), leg(&secondary)]);

        chain.start().unwrap();

        chain.request_bars(bars_request()).unwrap();
        let DataEvent::Response(DataResponse::Bars(answer)) = next_event(&mut events) else {
            panic!("expected a bar response");
        };
        assert_eq!(answer.data[0].close, Price::from("200.00"));

        let second = bars_request();
        chain.request_bars(second.clone()).unwrap();

        let DataEvent::Response(DataResponse::Bars(answer)) = next_event(&mut events) else {
            panic!("expected a bar response");
        };

        assert_eq!(answer.correlation_id, second.request_id);
        assert_eq!(answer.data[0].close, Price::from("200.00"));
        assert_eq!(
            primary.asked().len(),
            1,
            "the chain did not go back to the primary the moment it could be asked"
        );
    }

    /// The failure a chain cannot see when it asks: a provider that takes the demand and says nothing.
    /// The boundary holds it to account, and the next demand does not go there.
    #[rstest]
    fn test_a_provider_that_never_answers_is_held_to_it() {
        let mut events = install_engine();
        let policy = Policy {
            answer_deadline: Duration::from_millis(50),
            ..Policy::default()
        };
        let silent = FakeDataClientFactory::new("PRIMARY").silent();
        let secondary = provider("SECONDARY", "200.00");
        let mut chain = chain_with(policy, vec![leg(&silent), leg(&secondary)]);

        chain.start().unwrap();

        chain.request_bars(bars_request()).unwrap();
        assert!(
            silent.asked().len() == 1,
            "the silent provider took the demand"
        );

        let snapshot = await_set_aside(&chain, "PRIMARY");

        assert_eq!(snapshot.last_failure, Some(Failure::Unanswered));
        assert_eq!(snapshot.answers, 0);

        let second = bars_request();
        chain.request_bars(second.clone()).unwrap();

        let DataEvent::Response(DataResponse::Bars(answer)) = next_event(&mut events) else {
            panic!("expected a bar response");
        };

        assert_eq!(answer.correlation_id, second.request_id);
        assert_eq!(answer.data[0].close, Price::from("200.00"));
        assert_eq!(
            silent.asked().len(),
            1,
            "a provider that did not answer is not asked again for the next demand"
        );
    }

    /// A provider that stops answering is left even though no attempt of it failed: the demand it
    /// took was never answered, and the deadline is what makes that a hop rather than a mystery.
    #[rstest]
    fn test_a_provider_that_stops_answering_is_counted_as_hopped_from() {
        let mut events = install_engine();
        let policy = Policy {
            answer_deadline: Duration::from_millis(50),
            ..Policy::default()
        };
        let primary = provider("PRIMARY", "100.00");
        let secondary = provider("SECONDARY", "200.00");
        let mut chain = chain_with(policy, vec![leg(&primary), leg(&secondary)]);

        chain.start().unwrap();

        // The primary serves a demand, so the chain is being served by it.
        chain.request_bars(bars_request()).unwrap();
        let DataEvent::Response(DataResponse::Bars(_)) = next_event(&mut events) else {
            panic!("expected a bar response");
        };

        // It stops answering, and the deadline sets it aside.
        primary.silent_now();
        chain.request_bars(bars_request()).unwrap();
        await_set_aside(&chain, "PRIMARY");

        // The next demand is served elsewhere, and the chain says it moved away.
        chain.request_bars(bars_request()).unwrap();

        let DataEvent::Response(DataResponse::Bars(answer)) = next_event(&mut events) else {
            panic!("expected a bar response");
        };

        assert_eq!(answer.data[0].close, Price::from("200.00"));

        let snapshot = snapshot(&chain, "PRIMARY");

        assert_eq!(
            snapshot.hops, 1,
            "the chain moved away from the primary once"
        );
        assert_eq!(snapshot.answers, 1);
    }

    /// The counts are the chain's account of itself, so a provider nobody asked is not in them.
    #[rstest]
    fn test_the_health_reports_only_the_providers_that_were_asked() {
        let mut events = install_engine();
        let primary = provider("PRIMARY", "100.00");
        let secondary = provider("SECONDARY", "200.00");
        let mut chain = chain(vec![leg(&primary), leg(&secondary)]);

        chain.start().unwrap();

        assert!(chain.health().is_empty());

        chain.request_bars(bars_request()).unwrap();
        next_event(&mut events);

        let health = chain.health();

        assert_eq!(health.len(), 1, "the secondary was never asked");
        assert_eq!(health[0].provider, id("PRIMARY"));
        assert_eq!(health[0].attempts, 1);
        assert_eq!(health[0].answers, 1);
    }

    /// The chain takes a stream from the provider it prefers, and says nothing about it: a stream
    /// that was not moved is not a change of source.
    #[rstest]
    fn test_the_chain_takes_a_stream_from_the_provider_it_prefers() {
        let _events = install_engine();
        let journal = journal();
        let primary = provider("PRIMARY", "100.00").journaling(&journal);
        let secondary = provider("SECONDARY", "200.00").journaling(&journal);
        let mut chain = chain(vec![leg(&primary), leg(&secondary)]);

        chain.start().unwrap();

        let bars = subscribe_bars_request("AAPL.US");
        let stream = Stream::Bars(bars.bar_type);
        chain.subscribe_bars(bars).unwrap();

        assert_eq!(chain.streamed(), vec![(stream.clone(), id("PRIMARY"))]);
        assert!(
            journal
                .borrow()
                .contains(&FakeCall::Subscribed(id("PRIMARY"), stream))
        );
        assert!(secondary.subscribed().is_empty());
    }

    /// The exit of this step: the provider streaming is the one that fails, and the whole stream moves
    /// with it, once, giving the old subscription up before taking the new one.
    #[rstest]
    fn test_a_stream_moves_once_when_the_provider_serving_it_fails() {
        let mut events = install_engine();
        let journal = journal();
        let primary = provider("PRIMARY", "100.00").journaling(&journal);
        let secondary = provider("SECONDARY", "200.00").journaling(&journal);
        let mut chain = chain(vec![leg(&primary), leg(&secondary)]);

        chain.start().unwrap();

        let bars = subscribe_bars_request("AAPL.US");
        let stream = Stream::Bars(bars.bar_type);
        chain.subscribe_bars(bars).unwrap();

        // The provider that is streaming stops serving, which the chain sees when it next asks it.
        primary.refuse_now();
        let _ = chain.request_bars(bars_request());

        // A second demand arrives, and the chain moves as a provider rather than per instrument.
        chain
            .subscribe_bars(subscribe_bars_request("MSFT.US"))
            .unwrap();

        let calls = journal.borrow().clone();
        let given_up = FakeCall::Unsubscribed(id("PRIMARY"), stream.clone());
        let moved = FakeCall::Subscribed(id("SECONDARY"), stream);

        assert_eq!(
            calls.iter().filter(|call| **call == moved).count(),
            1,
            "the stream moved once"
        );
        assert!(
            calls.iter().position(|call| *call == given_up)
                < calls.iter().position(|call| *call == moved),
            "the old subscription was given up before the new one was taken"
        );
        assert_eq!(chain.streamed()[0].1, id("SECONDARY"));

        let continuity = continuity_of(&mut events);

        assert_eq!(continuity.from, id("PRIMARY"));
        assert_eq!(continuity.to, id("SECONDARY"));
        assert!(continuity.stream.contains("AAPL.US"));
        assert_eq!(continuity.ts_init, continuity.ts_event);
    }

    /// Fail-back is asked for rather than done: the chain stays on the fallback while the fallback
    /// serves, and comes back only at a checkpoint.
    #[rstest]
    fn test_a_checkpoint_brings_the_streams_back_to_the_provider_it_prefers() {
        let _events = install_engine();
        let journal = journal();
        let policy = Policy {
            cooldown: Duration::from_millis(5),
            ..Policy::default()
        };
        let primary = provider("PRIMARY", "100.00").journaling(&journal);
        let secondary = provider("SECONDARY", "200.00").journaling(&journal);
        let mut chain = chain_with(policy, vec![leg(&primary), leg(&secondary)]);

        chain.start().unwrap();

        let bars = subscribe_bars_request("AAPL.US");
        let stream = Stream::Bars(bars.bar_type);
        chain.subscribe_bars(bars).unwrap();

        primary.refuse_now();
        let _ = chain.request_bars(bars_request());
        chain
            .subscribe_bars(subscribe_bars_request("MSFT.US"))
            .unwrap();

        assert_eq!(chain.streamed()[0].1, id("SECONDARY"));
        assert_eq!(
            chain.streamed()[0].1,
            id("SECONDARY"),
            "the chain did not go back the moment the primary could be asked again"
        );

        // The provider recovers, and the chain is brought back on purpose.
        primary.recover();
        std::thread::sleep(Duration::from_millis(50));
        chain.checkpoint().unwrap();

        assert_eq!(chain.streamed()[0].1, id("PRIMARY"));
        let calls = journal.borrow().clone();

        assert!(calls.contains(&FakeCall::Unsubscribed(id("SECONDARY"), stream.clone())));
        assert_eq!(
            calls
                .iter()
                .filter(|call| **call == FakeCall::Subscribed(id("PRIMARY"), stream.clone()))
                .count(),
            2,
            "the stream was taken at the primary, moved away, and taken again"
        );
    }

    /// A stream the chain holds is not subscribed again: asking twice does not give it two sources.
    #[rstest]
    fn test_a_stream_already_held_is_not_subscribed_again() {
        let _events = install_engine();
        let journal = journal();
        let primary = provider("PRIMARY", "100.00").journaling(&journal);
        let mut chain = chain(vec![leg(&primary)]);

        chain.start().unwrap();

        chain
            .subscribe_bars(subscribe_bars_request("AAPL.US"))
            .unwrap();
        chain
            .subscribe_bars(subscribe_bars_request("AAPL.US"))
            .unwrap();

        assert_eq!(
            primary.subscribed().len(),
            1,
            "the provider was asked for the stream once"
        );
        assert_eq!(chain.streamed().len(), 1);
    }

    #[rstest]
    fn test_a_stream_is_given_up_at_the_provider_serving_it() {
        let _events = install_engine();
        let journal = journal();
        let primary = provider("PRIMARY", "100.00").journaling(&journal);
        let mut chain = chain(vec![leg(&primary)]);

        chain.start().unwrap();

        let bars = subscribe_bars_request("AAPL.US");
        let stream = Stream::Bars(bars.bar_type);
        chain.subscribe_bars(bars).unwrap();

        chain
            .unsubscribe_bars(&unsubscribe_bars_request("AAPL.US"))
            .unwrap();

        assert!(chain.streamed().is_empty());
        assert!(
            journal
                .borrow()
                .contains(&FakeCall::Unsubscribed(id("PRIMARY"), stream))
        );
    }
}
