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

//! The boundary a chain presents to the engine.
//!
//! The engine knows one client, and that client is the chain. Its providers, however, build their
//! events about themselves: a response carries the identity of the provider that answered it. The
//! chain therefore reads what its providers emit and writes what the engine is shown, and this is
//! where the identity is changed.
//!
//! # What is rewritten, and what is not
//!
//! Only a response carries an identity at all. It is also the only event the engine matches to a
//! request, and it is matched by the correlation identifier the caller *sent*, which a provider
//! copies from the request rather than minting. That identifier therefore passes through untouched,
//! and rewriting it would break every request the chain serves.
//!
//! Market data events - bars, ticks, book deltas, instruments - carry no identity, so a rewrite has
//! nothing to do to them. They are forwarded as they arrive.

use nautilus_common::{
    live::sender::EventSender,
    messages::{DataEvent, data::DataResponse},
};
use nautilus_model::identifiers::ClientId;

/// Reads what a chain's providers emit and writes what the engine is shown.
///
/// The tap is a value rather than a task: what the boundary does is a function from one event to
/// one event, and whether that function is applied inline or from a task is the caller's business.
#[derive(Debug, Clone)]
pub struct Tap {
    /// The one identity the engine knows the chain by.
    client_id: ClientId,
    /// Where the rewritten events are written.
    engine: EventSender<DataEvent>,
}

impl Tap {
    /// Creates a tap that presents `client_id` to the engine.
    #[must_use]
    pub const fn new(client_id: ClientId, engine: EventSender<DataEvent>) -> Self {
        Self { client_id, engine }
    }

    /// Returns the identity the engine knows the chain by.
    #[must_use]
    pub const fn client_id(&self) -> ClientId {
        self.client_id
    }

    /// Returns what the engine is to be shown for one event a provider emitted.
    ///
    /// A response takes the chain's identity, and everything else is unchanged. The correlation
    /// identifier is deliberately left alone: it is the caller's, and it is what the engine matches
    /// the answer to the request by.
    #[must_use]
    pub fn rewrite(&self, event: DataEvent) -> DataEvent {
        match event {
            DataEvent::Response(response) => {
                DataEvent::Response(identify(response, self.client_id))
            }
            other => other,
        }
    }

    /// Writes one provider event to the engine, rewritten.
    ///
    /// Returns whether it was delivered, because a caller that has lost the engine has nothing left
    /// to do with the event and a caller that has not should not be told so by silence.
    pub fn forward(&self, event: DataEvent) -> bool {
        let event = self.rewrite(event);

        match self.engine.send(event) {
            Ok(()) => true,
            Err(e) => {
                log::error!("cannot write a provider's event to the engine: {e}");

                false
            }
        }
    }
}

/// Returns `response` with `client_id` as the identity it carries.
fn identify(response: DataResponse, client_id: ClientId) -> DataResponse {
    match response {
        DataResponse::Data(mut r) => {
            r.client_id = client_id;

            DataResponse::Data(r)
        }
        DataResponse::Instrument(mut r) => {
            r.client_id = client_id;

            DataResponse::Instrument(r)
        }
        DataResponse::Instruments(mut r) => {
            r.client_id = client_id;

            DataResponse::Instruments(r)
        }
        DataResponse::Book(mut r) => {
            r.client_id = client_id;

            DataResponse::Book(r)
        }
        DataResponse::BookDeltas(mut r) => {
            r.client_id = client_id;

            DataResponse::BookDeltas(r)
        }
        DataResponse::BookDepth(mut r) => {
            r.client_id = client_id;

            DataResponse::BookDepth(r)
        }
        DataResponse::Quotes(mut r) => {
            r.client_id = client_id;

            DataResponse::Quotes(r)
        }
        DataResponse::Trades(mut r) => {
            r.client_id = client_id;

            DataResponse::Trades(r)
        }
        DataResponse::FundingRates(mut r) => {
            r.client_id = client_id;

            DataResponse::FundingRates(r)
        }
        DataResponse::OptionChainReferencePrice(mut r) => {
            r.client_id = client_id;

            DataResponse::OptionChainReferencePrice(r)
        }
        DataResponse::Bars(mut r) => {
            r.client_id = client_id;

            DataResponse::Bars(r)
        }
    }
}

#[cfg(test)]
mod tests {
    use nautilus_common::{
        live::sender::EventSender,
        messages::data::{BarsResponse, InstrumentResponse, InstrumentsResponse},
    };
    use nautilus_core::{UUID4, UnixNanos};
    use nautilus_model::{
        data::{Bar, BarSpecification, BarType, Data},
        enums::{AggregationSource, BarAggregation, PriceType},
        identifiers::{InstrumentId, Venue},
        instruments::{InstrumentAny, stubs},
        types::{Price, Quantity},
    };
    use rstest::rstest;

    use super::*;

    const CHAIN: &str = "CHAIN";
    const PROVIDER: &str = "MOOMOO";

    /// A tap whose engine has gone away, for the tests that only exercise the rewrite.
    fn tap() -> Tap {
        let (sender, _receiver) = tokio::sync::mpsc::unbounded_channel();

        Tap::new(ClientId::from(CHAIN), EventSender::new(sender))
    }

    fn bar_type() -> BarType {
        BarType::new(
            InstrumentId::from("AAPL.US"),
            BarSpecification::new(1, BarAggregation::Day, PriceType::Last),
            AggregationSource::External,
        )
    }

    fn equity() -> InstrumentAny {
        InstrumentAny::Equity(stubs::equity_aapl())
    }

    fn bar() -> Bar {
        Bar::new_checked(
            bar_type(),
            Price::new(100.0, 2),
            Price::new(102.0, 2),
            Price::new(99.0, 2),
            Price::new(101.0, 2),
            Quantity::new(10.0, 0),
            UnixNanos::default(),
            UnixNanos::default(),
        )
        .unwrap()
    }

    fn bar_response(correlation_id: UUID4) -> DataResponse {
        DataResponse::Bars(BarsResponse::new(
            correlation_id,
            ClientId::from(PROVIDER),
            bar_type(),
            Vec::new(),
            None,
            None,
            UnixNanos::default(),
            None,
        ))
    }

    /// A response carries two identifiers, and only one of them is the chain's to change. The
    /// correlation identifier is the caller's, and it is what the engine matches the answer to the
    /// request by: a chain that rewrote it would leave every request it serves unanswered.
    #[rstest]
    fn test_a_bar_response_takes_the_chains_identity_and_keeps_the_callers_identifier() {
        let correlation_id = UUID4::new();

        let DataEvent::Response(DataResponse::Bars(rewritten)) =
            tap().rewrite(DataEvent::Response(bar_response(correlation_id)))
        else {
            panic!("expected a bar response");
        };

        assert_eq!(rewritten.client_id, ClientId::from(CHAIN));
        assert_eq!(rewritten.correlation_id, correlation_id);
    }

    #[rstest]
    fn test_an_instrument_response_takes_the_chains_identity() {
        let response = DataResponse::Instrument(Box::new(InstrumentResponse::new(
            UUID4::new(),
            ClientId::from(PROVIDER),
            InstrumentId::from("AAPL.US"),
            equity(),
            None,
            None,
            UnixNanos::default(),
            None,
        )));

        let DataEvent::Response(DataResponse::Instrument(rewritten)) =
            tap().rewrite(DataEvent::Response(response))
        else {
            panic!("expected an instrument response");
        };

        assert_eq!(rewritten.client_id, ClientId::from(CHAIN));
    }

    #[rstest]
    fn test_a_universe_response_takes_the_chains_identity() {
        let response = DataResponse::Instruments(InstrumentsResponse::new(
            UUID4::new(),
            ClientId::from(PROVIDER),
            Venue::from("US"),
            vec![equity()],
            None,
            None,
            UnixNanos::default(),
            None,
        ));

        let DataEvent::Response(DataResponse::Instruments(rewritten)) =
            tap().rewrite(DataEvent::Response(response))
        else {
            panic!("expected an instruments response");
        };

        assert_eq!(rewritten.client_id, ClientId::from(CHAIN));
    }

    /// Market data carries no identity, so the boundary has nothing to change and must not change
    /// anything: a bar that arrived from a provider is the bar the engine is given.
    #[rstest]
    fn test_market_data_is_forwarded_unchanged() {
        let DataEvent::Data(Data::Bar(forwarded)) =
            tap().rewrite(DataEvent::Data(Data::Bar(bar())))
        else {
            panic!("expected the bar back");
        };

        assert_eq!(forwarded, bar());
    }

    #[rstest]
    fn test_an_instrument_event_is_forwarded_unchanged() {
        let rewritten = tap().rewrite(DataEvent::Instrument(equity()));

        assert!(matches!(rewritten, DataEvent::Instrument(_)));
    }

    /// What a caller sent for a request travels with the answer, and the boundary has no business
    /// with it.
    #[rstest]
    fn test_the_window_a_caller_asked_for_survives_the_boundary() {
        let response = DataResponse::Bars(BarsResponse::new(
            UUID4::new(),
            ClientId::from(PROVIDER),
            bar_type(),
            Vec::new(),
            Some(UnixNanos::from(1_u64)),
            Some(UnixNanos::from(2_u64)),
            UnixNanos::default(),
            None,
        ));

        let DataEvent::Response(DataResponse::Bars(rewritten)) =
            tap().rewrite(DataEvent::Response(response))
        else {
            panic!("expected a bar response");
        };

        assert_eq!(rewritten.start, Some(UnixNanos::from(1_u64)));
        assert_eq!(rewritten.end, Some(UnixNanos::from(2_u64)));
    }

    #[rstest]
    fn test_an_event_reaches_the_engine_with_the_chains_identity() {
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<DataEvent>();
        let tap = Tap::new(ClientId::from(CHAIN), EventSender::from(sender));

        assert!(tap.forward(DataEvent::Response(bar_response(UUID4::new()))));

        let DataEvent::Response(DataResponse::Bars(delivered)) = receiver
            .try_recv()
            .expect("the event should have been written")
        else {
            panic!("expected a bar response");
        };

        assert_eq!(delivered.client_id, ClientId::from(CHAIN));
    }

    #[rstest]
    fn test_an_event_the_engine_cannot_receive_is_reported_rather_than_failing_silently() {
        let (sender, receiver) = tokio::sync::mpsc::unbounded_channel::<DataEvent>();
        drop(receiver);

        let tap = Tap::new(ClientId::from(CHAIN), EventSender::from(sender));

        assert!(!tap.forward(DataEvent::Data(Data::Bar(bar()))));
    }
}
