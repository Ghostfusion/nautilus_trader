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

//! Providers and clients whose answers are written down in advance, so that a chain can be exercised
//! without a network.
//!
//! What a chain does is a property of the chain and not of a venue, so it should be provable without
//! one. There are two levels of this, because a chain has two: a provider as the chain asks it, and a
//! client as the boundary carries it. Both are deterministic in the only way that matters: every
//! answer is decided before the demand is made, so a test states what it expects to happen rather
//! than waiting to find out.

use std::{
    any::Any,
    cell::{Cell, RefCell},
    collections::VecDeque,
    rc::Rc,
};

use anyhow::Context;
use async_trait::async_trait;
use nautilus_common::{
    cache::CacheView,
    clients::DataClient,
    clock::Clock,
    factories::{ClientConfig, DataClientFactory},
    live::{runner::try_get_data_event_sender, sender::EventSender},
    messages::{
        DataEvent,
        data::{
            BarsResponse, DataResponse, InstrumentResponse, InstrumentsResponse, RequestBars,
            RequestInstrument, RequestInstruments, SubscribeBars, SubscribeQuotes, SubscribeTrades,
            UnsubscribeBars, UnsubscribeQuotes, UnsubscribeTrades,
        },
    },
};
use nautilus_core::{UUID4, UnixNanos};
use nautilus_model::{
    data::{Bar, BarType},
    identifiers::{ClientId, InstrumentId, Venue},
    instruments::{Equity, InstrumentAny},
    types::{Currency, Price, Quantity},
};

use crate::{
    chain::Provider,
    failure::Failure,
    streaming::{Stream, Subscription, Unsubscription},
};

/// A provider whose answers are written down in advance.
///
/// The answers are read in the order the attempts are made, so a script of `[fail, ok]` describes one
/// failed attempt and one successful one, whether they are a retry or a hop. The demands it was
/// asked are kept, which is how a test states that a provider was *not* used: a chain that never
/// hops leaves the secondary with nothing to answer.
#[derive(Debug)]
pub struct ScriptedProvider<D, T> {
    id: ClientId,
    script: VecDeque<Result<T, Failure>>,
    demands: Vec<D>,
}

impl<D, T> ScriptedProvider<D, T> {
    /// Creates a provider that answers with `script`, in order.
    #[must_use]
    pub fn new(id: ClientId, script: Vec<Result<T, Failure>>) -> Self {
        Self {
            id,
            script: script.into(),
            demands: Vec::new(),
        }
    }

    /// Returns the demands this provider was asked, in the order it was asked them.
    #[must_use]
    pub fn demands(&self) -> &[D] {
        &self.demands
    }

    /// Returns how many answers the script has left.
    #[must_use]
    pub fn remaining(&self) -> usize {
        self.script.len()
    }
}

impl<D: Clone + 'static, T: 'static> Provider for ScriptedProvider<D, T> {
    type Demand = D;
    type Answer = T;

    fn id(&self) -> ClientId {
        self.id
    }

    /// # Errors
    ///
    /// Returns the failure the script holds for this attempt.
    ///
    /// # Panics
    ///
    /// Panics when the script has no answer left. A call that was not scripted is a test that
    /// expected something else to happen, and answering it anyway would hide that.
    fn serve(&mut self, demand: &Self::Demand) -> Result<Self::Answer, Failure> {
        self.demands.push(demand.clone());

        self.script
            .pop_front()
            .expect("the script has no answer left for this demand")
    }
}

/// What a scripted client was asked for.
#[derive(Debug, Clone)]
pub enum FakeDemand {
    /// A list of instruments for a venue.
    Instruments(RequestInstruments),
    /// One instrument.
    Instrument(RequestInstrument),
    /// Bars for one instrument and bar type.
    Bars(RequestBars),
    /// A stream the client was asked to take.
    Subscribed(Subscription),
    /// A stream the client was asked to give up.
    Unsubscribed(Stream),
}

impl FakeDemand {
    /// Returns the identifier the caller gave this demand, which is what its answer carries.
    #[must_use]
    pub fn request_id(&self) -> UUID4 {
        match self {
            Self::Instruments(request) => request.request_id,
            Self::Instrument(request) => request.request_id,
            Self::Bars(request) => request.request_id,
            Self::Subscribed(_) | Self::Unsubscribed(_) => UUID4::new(),
        }
    }
}

/// One call a scripted client took, in the order the chain made it.
///
/// A chain's rules about streams are about order - the old subscription is given up before the new one
/// is taken - and order across two clients is not visible from either of them. It is visible here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FakeCall {
    /// A stream was subscribed.
    Subscribed(ClientId, Stream),
    /// A stream was given up.
    Unsubscribed(ClientId, Stream),
}

/// The configuration a scripted client reads, which is empty: a scripted client is told what it needs
/// when it is made, so that a test does not have to write a configuration file to describe it.
#[derive(Debug, Clone, Copy, Default)]
pub struct FakeConfig;

impl ClientConfig for FakeConfig {
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A data client whose request calls are written down in advance.
///
/// A real client takes a request and answers it by writing a response, and the boundary is what
/// carries that response to the engine. This is a client in the respects a chain cares about: it
/// captures the data event sender of the thread it is built on, exactly as a real adapter does;
/// whether it takes a demand is decided before the demand is made; and what it answers with is
/// marked, so that a test can say which provider served.
#[derive(Debug)]
pub struct FakeDataClient {
    id: ClientId,
    sender: EventSender<DataEvent>,
    mark: Price,
    refuses: Rc<Cell<bool>>,
    silent: Rc<Cell<bool>>,
    connected: bool,
    asked: Rc<RefCell<Vec<FakeDemand>>>,
    journal: Option<Rc<RefCell<Vec<FakeCall>>>>,
}

impl FakeDataClient {
    /// Creates a client that answers under the identity it is given, marking its answers with `mark`
    /// and recording what it is asked in `asked`.
    ///
    /// # Errors
    ///
    /// Returns an error when no data event sender is installed on this thread, which is the same
    /// condition a real adapter fails on when it is built in the wrong place.
    pub fn new(
        id: ClientId,
        mark: Price,
        refuses: Rc<Cell<bool>>,
        silent: Rc<Cell<bool>>,
        asked: Rc<RefCell<Vec<FakeDemand>>>,
        journal: Option<Rc<RefCell<Vec<FakeCall>>>>,
    ) -> anyhow::Result<Self> {
        let sender = try_get_data_event_sender()
            .context("a scripted client must be built where a data event sender is installed")?;

        Ok(Self {
            id,
            sender,
            mark,
            refuses,
            silent,
            connected: true,
            asked,
            journal,
        })
    }

    /// Returns whether this client has a transport to send the request on.
    ///
    /// # Errors
    ///
    /// Returns an error when the client refuses, which is what a client with nowhere to send says.
    fn has_transport(&self) -> anyhow::Result<()> {
        if self.refuses.get() {
            anyhow::bail!("the client has no transport to send the request on");
        }

        Ok(())
    }

    /// Returns whether this client takes the request and never answers it.
    ///
    /// A client that is up with nothing behind it takes the demand and says nothing, which is the
    /// failure a chain can only see on the event channel.
    fn is_silent(&self) -> bool {
        self.silent.get()
    }

    /// Takes a stream, as a provider that serves it does.
    fn take_stream(&self, command: &Subscription) -> anyhow::Result<()> {
        self.asked
            .borrow_mut()
            .push(FakeDemand::Subscribed(command.clone()));
        self.witness(FakeCall::Subscribed(self.id, command.stream()));

        self.has_transport()
    }

    /// Gives up a stream, as a provider that was serving it does.
    fn give_up(&self, command: &Unsubscription) -> anyhow::Result<()> {
        self.asked
            .borrow_mut()
            .push(FakeDemand::Unsubscribed(command.stream()));
        self.witness(FakeCall::Unsubscribed(self.id, command.stream()));

        self.has_transport()
    }

    /// Notes a call in the shared journal, when the scripted clients are reporting to one.
    fn witness(&self, call: FakeCall) {
        if let Some(journal) = &self.journal {
            journal.borrow_mut().push(call);
        }
    }

    /// Writes an answer, as a client does when its provider has answered it.
    fn write(&self, response: DataResponse) -> anyhow::Result<()> {
        self.sender
            .send(DataEvent::Response(response))
            .map_err(|e| anyhow::anyhow!("cannot write the answer: {e}"))
    }

    /// Returns the one bar this client answers a bar request with.
    fn bar(&self, bar_type: BarType) -> Bar {
        Bar::new_checked(
            bar_type,
            self.mark,
            self.mark,
            self.mark,
            self.mark,
            Quantity::new(1.0, 0),
            UnixNanos::default(),
            UnixNanos::default(),
        )
        .expect("a bar whose prices are all one price is well formed")
    }
}

#[async_trait(?Send)]
impl DataClient for FakeDataClient {
    fn client_id(&self) -> ClientId {
        self.id
    }

    fn venue(&self) -> Option<Venue> {
        None
    }

    fn start(&mut self) -> anyhow::Result<()> {
        self.connected = true;

        Ok(())
    }

    fn stop(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn reset(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn dispose(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    fn is_disconnected(&self) -> bool {
        !self.connected
    }

    async fn connect(&mut self) -> anyhow::Result<()> {
        self.connected = true;

        Ok(())
    }

    async fn disconnect(&mut self) -> anyhow::Result<()> {
        self.connected = false;

        Ok(())
    }

    fn subscribe_bars(&mut self, cmd: SubscribeBars) -> anyhow::Result<()> {
        self.take_stream(&Subscription::Bars(cmd))
    }

    fn subscribe_quotes(&mut self, cmd: SubscribeQuotes) -> anyhow::Result<()> {
        self.take_stream(&Subscription::Quotes(cmd))
    }

    fn subscribe_trades(&mut self, cmd: SubscribeTrades) -> anyhow::Result<()> {
        self.take_stream(&Subscription::Trades(cmd))
    }

    fn unsubscribe_bars(&mut self, cmd: &UnsubscribeBars) -> anyhow::Result<()> {
        self.give_up(&Unsubscription::Bars(cmd.clone()))
    }

    fn unsubscribe_quotes(&mut self, cmd: &UnsubscribeQuotes) -> anyhow::Result<()> {
        self.give_up(&Unsubscription::Quotes(cmd.clone()))
    }

    fn unsubscribe_trades(&mut self, cmd: &UnsubscribeTrades) -> anyhow::Result<()> {
        self.give_up(&Unsubscription::Trades(cmd.clone()))
    }

    fn request_instruments(&self, request: RequestInstruments) -> anyhow::Result<()> {
        self.asked
            .borrow_mut()
            .push(FakeDemand::Instruments(request.clone()));

        self.has_transport()?;

        if self.is_silent() {
            return Ok(());
        }

        let instrument = fake_equity(InstrumentId::from("AAPL.US"));

        self.write(DataResponse::Instruments(InstrumentsResponse::new(
            request.request_id,
            request.client_id.unwrap_or(self.id),
            request.venue.unwrap_or_else(|| Venue::from("SIM")),
            vec![instrument],
            None,
            None,
            UnixNanos::default(),
            None,
        )))
    }

    fn request_instrument(&self, request: RequestInstrument) -> anyhow::Result<()> {
        self.asked
            .borrow_mut()
            .push(FakeDemand::Instrument(request.clone()));

        self.has_transport()?;

        if self.is_silent() {
            return Ok(());
        }

        self.write(DataResponse::Instrument(Box::new(InstrumentResponse::new(
            request.request_id,
            request.client_id.unwrap_or(self.id),
            request.instrument_id,
            fake_equity(request.instrument_id),
            None,
            None,
            UnixNanos::default(),
            None,
        ))))
    }

    fn request_bars(&self, request: RequestBars) -> anyhow::Result<()> {
        self.asked
            .borrow_mut()
            .push(FakeDemand::Bars(request.clone()));

        self.has_transport()?;

        if self.is_silent() {
            return Ok(());
        }

        self.write(DataResponse::Bars(BarsResponse::new(
            request.request_id,
            request.client_id.unwrap_or(self.id),
            request.bar_type,
            vec![self.bar(request.bar_type)],
            None,
            None,
            UnixNanos::default(),
            None,
        )))
    }
}

/// A factory that builds scripted clients, so that a chain can be assembled out of legs.
///
/// The factory keeps what its clients are asked, because a test that asserts which provider served a
/// demand has to be able to see which providers were asked for it.
#[derive(Debug, Clone)]
pub struct FakeDataClientFactory {
    name: String,
    mark: Price,
    refuses: Rc<Cell<bool>>,
    silent: Rc<Cell<bool>>,
    asked: Rc<RefCell<Vec<FakeDemand>>>,
    journal: Option<Rc<RefCell<Vec<FakeCall>>>>,
}

impl FakeDataClientFactory {
    /// Creates a factory whose clients answer with a mark of 100.00.
    #[must_use]
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            mark: Price::from("100.00"),
            refuses: Rc::new(Cell::new(false)),
            silent: Rc::new(Cell::new(false)),
            asked: Rc::new(RefCell::new(Vec::new())),
            journal: None,
        }
    }

    /// Returns the demands this factory's clients were asked, in the order they were asked them.
    #[must_use]
    pub fn asked(&self) -> Vec<FakeDemand> {
        self.asked.borrow().clone()
    }

    /// Returns the streams this factory's clients were asked to take, in the order they were asked.
    #[must_use]
    pub fn subscribed(&self) -> Vec<Stream> {
        self.asked
            .borrow()
            .iter()
            .filter_map(|demand| match demand {
                FakeDemand::Subscribed(subscription) => Some(subscription.stream()),
                _ => None,
            })
            .collect()
    }

    /// Returns this factory with its clients answering at `mark`.
    #[must_use]
    pub fn marked(mut self, mark: Price) -> Self {
        self.mark = mark;

        self
    }

    /// Returns this factory with its clients refusing every request, as one with no transport does.
    #[must_use]
    pub fn refusing(self) -> Self {
        self.refuses.set(true);

        self
    }

    /// Returns this factory with its clients reporting their streams to `journal`, in order.
    #[must_use]
    pub fn journaling(mut self, journal: &Rc<RefCell<Vec<FakeCall>>>) -> Self {
        self.journal = Some(Rc::clone(journal));

        self
    }

    /// Sets this factory's clients refusing from now on, for the failures that only happen later.
    pub fn refuse_now(&self) {
        self.refuses.set(true);
    }

    /// Sets this factory's clients taking demands and never answering them from now on.
    pub fn silent_now(&self) {
        self.silent.set(true);
    }

    /// Sets this factory's clients serving again, for the recovery that a cooldown is waited out for.
    pub fn recover(&self) {
        self.refuses.set(false);
        self.silent.set(false);
    }

    /// Returns this factory with its clients taking every request and answering none of them, as a
    /// provider that is up and not serving does.
    #[must_use]
    pub fn silent(self) -> Self {
        self.silent.set(true);

        self
    }
}

impl DataClientFactory for FakeDataClientFactory {
    fn create(
        &self,
        name: &str,
        _config: &dyn ClientConfig,
        _cache: CacheView,
        _clock: Rc<RefCell<dyn Clock>>,
    ) -> anyhow::Result<Box<dyn DataClient>> {
        let id = ClientId::from(name);
        let client = FakeDataClient::new(
            id,
            self.mark,
            Rc::clone(&self.refuses),
            Rc::clone(&self.silent),
            Rc::clone(&self.asked),
            self.journal.as_ref().map(Rc::clone),
        )?;

        Ok(Box::new(client))
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn config_type(&self) -> &'static str {
        "FakeConfig"
    }
}

/// Returns an instrument for `instrument_id`, built the way a provider builds one.
fn fake_equity(instrument_id: InstrumentId) -> InstrumentAny {
    InstrumentAny::Equity(
        Equity::builder()
            .instrument_id(instrument_id)
            .raw_symbol(instrument_id.symbol)
            .currency(Currency::from("USD"))
            .price_precision(2)
            .price_increment(Price::from("0.01"))
            .ts_event(UnixNanos::default())
            .ts_init(UnixNanos::default())
            .build()
            .expect("the instrument is well formed"),
    )
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    fn test_the_script_is_read_in_order() {
        let mut provider = ScriptedProvider::new(
            ClientId::from("PRIMARY"),
            vec![Err(Failure::Unanswered), Ok("second answer".to_string())],
        );

        assert_eq!(provider.remaining(), 2);
        assert_eq!(provider.serve(&7_u32), Err(Failure::Unanswered));
        assert_eq!(provider.serve(&8_u32), Ok("second answer".to_string()));
        assert_eq!(provider.remaining(), 0);
        assert_eq!(provider.demands(), &[7, 8]);
    }

    #[rstest]
    fn test_a_provider_that_was_never_asked_has_no_demands() {
        let provider: ScriptedProvider<u32, String> =
            ScriptedProvider::new(ClientId::from("SECONDARY"), vec![Ok("answer".to_string())]);

        assert!(provider.demands().is_empty());
    }
}
