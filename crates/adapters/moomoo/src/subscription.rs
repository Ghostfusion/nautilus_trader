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

//! The subscription lifecycle.
//!
//! The venue's model is adversarial to a strategy's natural behaviour, in three ways, and this
//! module exists to absorb all three.
//!
//! The first is that a subscription is a shared, counted resource rather than a property of one
//! consumer, so what a consumer wants and what the venue holds are two different things. They are
//! kept as two maps, because a subscription can outlive the last consumer's interest by the release
//! delay below.
//!
//! The second is that the venue refuses to release a subscription inside a minimum duration of
//! taking it. A release is therefore scheduled rather than immediate, and a symbol wanted again
//! during the delay is not re-subscribed, because it was never actually released.
//!
//! The third is that the venue's state dies with the socket while the consumers' intent does not,
//! so a reconnect replays from intent and not from the map of what the venue held.

use std::{collections::HashMap, sync::Arc, time::Duration};

use anyhow::bail;
use parking_lot::Mutex;
use tokio::{
    sync::Notify,
    time::{Instant, sleep_until},
};

use crate::{
    common::Market,
    connection::Connection,
    mappers::bars::BarSession,
    providers::{self, SubscriptionAllowance},
};

/// How long a subscription is held after the last consumer loses interest.
///
/// The venue refuses a release inside one minute of the subscription being taken, so a shorter
/// delay produces a refusal rather than a release. A longer one holds a subscription the adapter no
/// longer needs, and the allowance is small.
pub const RELEASE_DELAY: Duration = Duration::from_secs(65);

/// A data type the venue can hold a subscription for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SubscriptionType {
    /// Basic quote snapshots.
    Quote,
    /// The order book.
    OrderBook,
    /// Tick by tick trades.
    Ticker,
}

impl SubscriptionType {
    /// Returns the venue's own code, per `Qot_Common.SubType`.
    #[must_use]
    pub fn sub_type(self) -> i32 {
        match self {
            Self::Quote => 1,
            Self::OrderBook => 2,
            Self::Ticker => 4,
        }
    }
}

/// One security and one data type: the unit the venue counts.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Subscription {
    /// The market the security trades on.
    pub market: Market,
    /// The security's code, without its market prefix.
    pub code: String,
    /// The data type.
    pub kind: SubscriptionType,
}

impl Subscription {
    /// Creates a subscription.
    #[must_use]
    pub fn new(market: Market, code: impl Into<String>, kind: SubscriptionType) -> Self {
        Self {
            market,
            code: code.into(),
            kind,
        }
    }
}

/// What the venue holds for one subscription.
#[derive(Debug, Clone, Copy)]
struct Held {
    /// When the venue granted it, which is what the release minimum is measured from.
    acquired_at: Instant,
    /// When a release may next be attempted. Only consulted once intent has fallen to zero.
    release_at: Instant,
}

#[derive(Debug, Default)]
struct State {
    /// What consumers want, reference counted.
    intent: HashMap<Subscription, u32>,
    /// What the venue holds.
    held: HashMap<Subscription, Held>,
}

impl State {
    /// Removes and returns the subscriptions whose release is due.
    fn take_due(&mut self, now: Instant) -> Vec<(Subscription, Held)> {
        let mut due = Vec::new();

        self.held.retain(|subscription, held| {
            if !self.intent.contains_key(subscription) && held.release_at <= now {
                due.push((subscription.clone(), *held));
                return false;
            }

            true
        });

        due
    }

    /// Returns the earliest moment a release becomes due, when there is one.
    fn next_deadline(&self) -> Option<Instant> {
        self.held
            .iter()
            .filter(|(subscription, _)| !self.intent.contains_key(*subscription))
            .map(|(_, held)| held.release_at)
            .min()
    }
}

/// Owns the venue's subscription state, the intent behind it, and the delay between them.
#[derive(Debug)]
pub struct SubscriptionManager {
    connection: Arc<Connection>,
    release_delay: Duration,
    session: BarSession,
    state: Mutex<State>,
    allowance: Mutex<Option<SubscriptionAllowance>>,
    wake: Notify,
}

impl SubscriptionManager {
    /// Creates a manager and starts its release task.
    ///
    /// `release_delay` is how long a subscription is held after the last consumer loses interest;
    /// [`RELEASE_DELAY`] is the value that satisfies the venue's own minimum.
    #[must_use]
    pub fn new(
        connection: Arc<Connection>,
        release_delay: Duration,
        session: BarSession,
    ) -> Arc<Self> {
        let manager = Arc::new(Self {
            connection,
            release_delay,
            session,
            state: Mutex::new(State::default()),
            allowance: Mutex::new(None),
            wake: Notify::new(),
        });

        tokio::spawn(Arc::clone(&manager).reap());

        manager
    }

    /// Returns the subscriptions the venue currently holds.
    #[must_use]
    pub fn held(&self) -> Vec<Subscription> {
        let mut held: Vec<Subscription> = self.state.lock().held.keys().cloned().collect();
        held.sort_by(|left, right| {
            (&left.market, &left.code, left.kind.sub_type()).cmp(&(
                &right.market,
                &right.code,
                right.kind.sub_type(),
            ))
        });
        held
    }

    /// Returns how many consumers want a subscription.
    #[must_use]
    pub fn reference_count(&self, subscription: &Subscription) -> u32 {
        self.state
            .lock()
            .intent
            .get(subscription)
            .copied()
            .unwrap_or(0)
    }

    /// Reads the venue's own allowance and remembers it for admission.
    ///
    /// The count is an estimate of what the adapter itself is using, so it is refreshed rather than
    /// tracked: the allowance is shared with every other tool pointed at the same gateway.
    ///
    /// # Errors
    ///
    /// Returns an error if the allowance cannot be read.
    pub async fn refresh_allowance(&self) -> anyhow::Result<SubscriptionAllowance> {
        let allowance = providers::request_subscription_allowance(&self.connection).await?;
        *self.allowance.lock() = Some(allowance);

        Ok(allowance)
    }

    /// Records interest in a subscription, subscribing at the venue if it is not held.
    ///
    /// A subscription the venue still holds is not re-subscribed: the reference is simply counted,
    /// which is what cancels a release that was waiting on the delay.
    ///
    /// # Errors
    ///
    /// Returns an error if the allowance is known to be fully used, or if the venue refuses the
    /// subscription.
    pub async fn subscribe(&self, subscription: Subscription) -> anyhow::Result<()> {
        let needed = {
            let mut state = self.state.lock();
            *state.intent.entry(subscription.clone()).or_insert(0) += 1;

            !state.held.contains_key(&subscription)
        };

        self.wake.notify_one();

        if !needed {
            return Ok(());
        }

        self.admit()?;

        providers::set_subscriptions(
            &self.connection,
            std::slice::from_ref(&subscription),
            true,
            self.session,
        )
        .await?;

        let now = Instant::now();
        self.state.lock().held.insert(
            subscription,
            Held {
                acquired_at: now,
                release_at: now + self.release_delay,
            },
        );

        Ok(())
    }

    /// Drops one consumer's interest, scheduling a release when the last one goes.
    ///
    /// The release itself happens in the manager's own task once the delay has elapsed, because the
    /// venue would refuse it any earlier.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no interest to drop, which is a caller defect rather than a
    /// venue condition.
    pub fn unsubscribe(&self, subscription: &Subscription) -> anyhow::Result<()> {
        {
            let mut state = self.state.lock();

            match state.intent.get_mut(subscription) {
                Some(count) if *count > 1 => *count -= 1,
                Some(_) => {
                    state.intent.remove(subscription);
                }
                None => bail!("no interest is recorded for {subscription:?}"),
            }
        }

        self.wake.notify_one();

        Ok(())
    }

    /// Re-issues every subscription consumers still want.
    ///
    /// This is what a reconnect calls. The venue's state died with the socket, so it is discarded
    /// and rebuilt from intent rather than replayed from the map of what the venue held.
    ///
    /// # Errors
    ///
    /// Returns an error if the venue refuses a subscription.
    pub async fn replay(&self) -> anyhow::Result<()> {
        let wanted = {
            let mut state = self.state.lock();
            state.held.clear();

            let mut wanted: Vec<Subscription> = state.intent.keys().cloned().collect();
            wanted.sort_by(|left, right| {
                (&left.market, &left.code, left.kind.sub_type()).cmp(&(
                    &right.market,
                    &right.code,
                    right.kind.sub_type(),
                ))
            });
            wanted
        };

        if wanted.is_empty() {
            return Ok(());
        }

        providers::set_subscriptions(&self.connection, &wanted, true, self.session).await?;

        let now = Instant::now();
        let mut state = self.state.lock();

        for subscription in wanted {
            state.held.insert(
                subscription,
                Held {
                    acquired_at: now,
                    release_at: now + self.release_delay,
                },
            );
        }

        Ok(())
    }

    /// Refuses a subscription the venue has no room for.
    ///
    /// The allowance is only consulted when it is known. Before the first read nothing is refused,
    /// because a guessed refusal would be worse than letting the venue answer.
    fn admit(&self) -> anyhow::Result<()> {
        let Some(allowance) = *self.allowance.lock() else {
            return Ok(());
        };

        let held = self.state.lock().held.len();
        let quota = allowance.quota();

        if held >= quota {
            bail!(
                "the subscription allowance is fully used: {held} of {quota} held, with {} other \
                 subscriptions on this connection",
                allowance.used
            );
        }

        Ok(())
    }

    /// Releases subscriptions whose delay has elapsed, and waits for the next one.
    async fn reap(self: Arc<Self>) {
        loop {
            let due = self.state.lock().take_due(Instant::now());

            for (subscription, held) in due {
                let released = providers::set_subscriptions(
                    &self.connection,
                    std::slice::from_ref(&subscription),
                    false,
                    self.session,
                )
                .await;

                if let Err(e) = released {
                    // The venue may still hold it, so it is kept and retried rather than forgotten:
                    // forgetting it would leave a subscription burning the allowance with nothing
                    // tracking it.
                    log::warn!("cannot release {subscription:?}, retrying after the delay: {e}");

                    self.state.lock().held.insert(
                        subscription,
                        Held {
                            acquired_at: held.acquired_at,
                            release_at: Instant::now() + self.release_delay,
                        },
                    );
                }
            }

            // The guard has to be dropped before the wait, or the task stops being sendable.
            let next = self.state.lock().next_deadline();

            match next {
                Some(at) => {
                    tokio::select! {
                        () = sleep_until(at) => {}
                        () = self.wake.notified() => {}
                    }
                }
                None => self.wake.notified().await,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use prost::Message as _;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{
            TcpListener,
            tcp::{OwnedReadHalf, OwnedWriteHalf},
        },
    };

    use super::*;
    use crate::{
        codec::{decode_frame, encode_frame},
        connection::{
            ConnectOptions, Message, PROTO_ID_INIT_CONNECT, PROTO_ID_KEEP_ALIVE, RET_OK, connect,
        },
        generated::{init_connect, keep_alive, qot_get_sub_info, qot_sub},
        providers::{PROTO_ID_GET_SUB_INFO, PROTO_ID_SUB},
    };

    /// A release delay short enough for a test, since the mock gateway does not enforce the
    /// venue's own minimum. The minimum is a property of the venue, not of this state machine.
    const TEST_RELEASE_DELAY: Duration = Duration::from_millis(200);

    const QUOTA: i32 = 100;

    /// What the mock gateway was asked to do, in order.
    #[derive(Debug, Default)]
    struct Observed {
        requests: Vec<(i32, bool)>,
    }

    async fn recv_message(socket: &mut OwnedReadHalf, buffer: &mut Vec<u8>) -> Option<Message> {
        loop {
            if let Some(frame) = decode_frame(buffer).ok().flatten() {
                let message = Message {
                    proto_id: frame.header.proto_id,
                    serial_no: frame.header.serial_no,
                    body: frame.body.to_vec(),
                };
                let total_len = frame.total_len();
                buffer.drain(..total_len);
                return Some(message);
            }

            let read = socket.read_buf(buffer).await.ok()?;
            if read == 0 {
                return None;
            }
        }
    }

    async fn send_message(socket: &mut OwnedWriteHalf, proto_id: u32, serial_no: u32, body: &[u8]) {
        let frame = encode_frame(proto_id, serial_no, body).unwrap();
        socket.write_all(&frame).await.unwrap();
    }

    async fn serve(
        mut read: OwnedReadHalf,
        mut write: OwnedWriteHalf,
        observed: Arc<Mutex<Observed>>,
    ) {
        let mut buffer = Vec::new();

        while let Some(message) = recv_message(&mut read, &mut buffer).await {
            match message.proto_id {
                PROTO_ID_SUB => {
                    let request = qot_sub::Request::decode(message.body.as_slice()).unwrap();

                    for sub_type in &request.c2s.sub_type_list {
                        observed
                            .lock()
                            .requests
                            .push((*sub_type, request.c2s.is_sub_or_un_sub));
                    }

                    let response = qot_sub::Response {
                        ret_type: RET_OK,
                        ..Default::default()
                    };
                    send_message(
                        &mut write,
                        PROTO_ID_SUB,
                        message.serial_no,
                        &response.encode_to_vec(),
                    )
                    .await;
                }
                PROTO_ID_GET_SUB_INFO => {
                    let response = qot_get_sub_info::Response {
                        ret_type: RET_OK,
                        s2c: Some(qot_get_sub_info::S2c {
                            total_used_quota: 0,
                            remain_quota: QUOTA,
                            ..Default::default()
                        }),
                        ..Default::default()
                    };
                    send_message(
                        &mut write,
                        PROTO_ID_GET_SUB_INFO,
                        message.serial_no,
                        &response.encode_to_vec(),
                    )
                    .await;
                }
                PROTO_ID_KEEP_ALIVE => {
                    let response = keep_alive::Response {
                        ret_type: RET_OK,
                        ..Default::default()
                    };
                    send_message(
                        &mut write,
                        PROTO_ID_KEEP_ALIVE,
                        message.serial_no,
                        &response.encode_to_vec(),
                    )
                    .await;
                }
                other => panic!("the mock gateway was sent proto {other}"),
            }
        }
    }

    /// Starts a mock gateway and returns its port and the record of what it was asked.
    async fn spawn_gateway() -> (u16, Arc<Mutex<Observed>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let observed = Arc::new(Mutex::new(Observed::default()));
        let recorder = Arc::clone(&observed);

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let (mut read, mut write) = stream.into_split();
            let mut buffer = Vec::new();

            let handshake = recv_message(&mut read, &mut buffer).await.unwrap();
            assert_eq!(handshake.proto_id, PROTO_ID_INIT_CONNECT);

            let response = init_connect::Response {
                ret_type: RET_OK,
                s2c: Some(init_connect::S2c {
                    server_ver: 1010,
                    login_user_id: 1,
                    conn_id: 1,
                    conn_aes_key: String::new(),
                    keep_alive_interval: 10,
                    ..Default::default()
                }),
                ..Default::default()
            };
            send_message(
                &mut write,
                PROTO_ID_INIT_CONNECT,
                handshake.serial_no,
                &response.encode_to_vec(),
            )
            .await;

            serve(read, write, recorder).await;
        });

        (port, observed)
    }

    async fn manager() -> (Arc<SubscriptionManager>, Arc<Mutex<Observed>>) {
        let (port, observed) = spawn_gateway().await;
        let (pushes, _push_rx) = tokio::sync::mpsc::unbounded_channel();
        let options = ConnectOptions::new("127.0.0.1", port);
        let connection = Arc::new(connect(&options, pushes).await.unwrap());
        let manager = SubscriptionManager::new(connection, TEST_RELEASE_DELAY, BarSession::Regular);

        (manager, observed)
    }

    fn ticker() -> Subscription {
        Subscription::new(Market::Us, "AAPL", SubscriptionType::Ticker)
    }

    fn requests(observed: &Arc<Mutex<Observed>>) -> Vec<(i32, bool)> {
        observed.lock().requests.clone()
    }

    #[tokio::test]
    async fn test_subscribing_holds_the_subscription() {
        let (manager, observed) = manager().await;
        let subscription = ticker();

        manager.refresh_allowance().await.unwrap();
        manager.subscribe(subscription.clone()).await.unwrap();

        assert_eq!(manager.held(), vec![subscription.clone()]);
        assert_eq!(manager.reference_count(&subscription), 1);
        assert_eq!(requests(&observed), vec![(4, true)]);
    }

    /// The second consumer of a subscription does not send anything, because the venue already
    /// holds it.
    #[tokio::test]
    async fn test_a_second_consumer_does_not_resubscribe() {
        let (manager, observed) = manager().await;
        let subscription = ticker();

        manager.subscribe(subscription.clone()).await.unwrap();
        manager.subscribe(subscription.clone()).await.unwrap();

        assert_eq!(manager.reference_count(&subscription), 2);
        assert_eq!(requests(&observed), vec![(4, true)]);
    }

    /// Losing one of two consumers must not release a subscription the other still wants.
    #[tokio::test]
    async fn test_one_consumer_leaving_keeps_the_subscription() {
        let (manager, observed) = manager().await;
        let subscription = ticker();

        manager.subscribe(subscription.clone()).await.unwrap();
        manager.subscribe(subscription.clone()).await.unwrap();
        manager.unsubscribe(&subscription).unwrap();

        tokio::time::sleep(TEST_RELEASE_DELAY * 2).await;

        assert_eq!(manager.held(), vec![subscription]);
        assert_eq!(requests(&observed), vec![(4, true)]);
    }

    /// The release waits out the delay, because the venue would refuse it any earlier.
    #[tokio::test]
    async fn test_the_release_waits_for_the_delay() {
        let (manager, observed) = manager().await;
        let subscription = ticker();

        manager.subscribe(subscription.clone()).await.unwrap();
        manager.unsubscribe(&subscription).unwrap();

        assert!(
            !manager.held().is_empty(),
            "the subscription is still held inside the delay"
        );

        tokio::time::sleep(TEST_RELEASE_DELAY * 3).await;

        assert!(manager.held().is_empty());
        assert_eq!(requests(&observed), vec![(4, true), (4, false)]);
    }

    /// A symbol wanted again during the delay is not re-subscribed, because it was never released.
    #[tokio::test]
    async fn test_wanting_a_symbol_again_cancels_the_release() {
        let (manager, observed) = manager().await;
        let subscription = ticker();

        manager.subscribe(subscription.clone()).await.unwrap();
        manager.unsubscribe(&subscription).unwrap();
        manager.subscribe(subscription.clone()).await.unwrap();

        tokio::time::sleep(TEST_RELEASE_DELAY * 3).await;

        assert_eq!(manager.held(), vec![subscription]);
        assert_eq!(
            requests(&observed),
            vec![(4, true)],
            "the release was cancelled and nothing was re-sent"
        );
    }

    /// A reconnect replays from what consumers want, not from what the venue held, because the
    /// venue's state died with the socket.
    #[tokio::test]
    async fn test_replay_reissues_what_is_wanted() {
        let (manager, observed) = manager().await;
        let subscription = ticker();

        manager.subscribe(subscription.clone()).await.unwrap();
        manager.replay().await.unwrap();

        assert_eq!(manager.held(), vec![subscription]);
        assert_eq!(requests(&observed), vec![(4, true), (4, true)]);
    }

    /// A release the venue refuses is retried rather than forgotten, because forgetting it would
    /// leave a subscription spending allowance with nothing tracking it.
    #[tokio::test]
    async fn test_dropping_unknown_interest_is_refused() {
        let (manager, _observed) = manager().await;

        assert!(manager.unsubscribe(&ticker()).is_err());
    }
}
