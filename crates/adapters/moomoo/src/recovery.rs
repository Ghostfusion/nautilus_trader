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

//! Bringing the adapter's own state back onto a session that replaced a lost one.
//!
//! A dropped socket is expected rather than exceptional, and what makes recovery correct rather
//! than merely automatic is what happens once the replacement is up. Three things have to be
//! brought back, and they are not the same kind of thing.
//!
//! The entitlement is read again, because it describes the login rather than the socket: a gateway
//! reconfigured between sessions would otherwise be served under a record that no longer describes
//! it.
//!
//! The subscriptions are replayed from what the consumers want, never from what the venue held.
//! The venue's state died with the socket and the consumers' intentions did not, so intent is the
//! only one of the two that still means anything.
//!
//! Every book is reseeded from a fresh snapshot. Once the subscription is replayed the deltas start
//! arriving again, but the book the consumer holds was built from the previous session, and the
//! deltas that arrived while it was down are gone. Resuming from the next delta leaves a book that
//! is wrong in a way nothing downstream can detect, which is what makes the snapshot load-bearing
//! rather than tidy.
//!
//! The replay has to precede the reseed, because the gateway refuses an order book request for a
//! security whose book is not subscribed, and it is the replay that takes the subscription out
//! again.

use std::{fmt::Debug, sync::Arc};

use nautilus_core::time::get_atomic_clock_realtime;
use nautilus_model::data::OrderBookDeltas;

use crate::{
    common::Market,
    connection::{Connection, SessionState},
    entitlement::Capabilities,
    mappers::book::{BOARD_NORMAL, BookContext, book_deltas_from, side_time},
    providers,
    subscription::{Subscription, SubscriptionManager, SubscriptionType},
};

/// What the layer that owns the instruments and the books has to give recovery.
///
/// `Debug` is required because the recovery that holds one is a public type of a crate that requires
/// its public types to be printable.
pub trait BookRecovery: Debug + Send + Sync {
    /// Returns the context `subscription`'s book is rebuilt against.
    ///
    /// `None` means this layer holds no book for that subscription, so there is nothing to rebuild.
    fn context(&self, subscription: &Subscription) -> Option<BookContext>;

    /// Takes a book that replaces everything held for `subscription` before the gap.
    fn seeded(&self, subscription: Subscription, deltas: OrderBookDeltas);
}

/// Restores the adapter's state on every session after the one the connection was made on.
#[derive(Debug)]
pub struct Recovery {
    connection: Arc<Connection>,
    manager: Arc<SubscriptionManager>,
    books: Arc<dyn BookRecovery>,
}

impl Recovery {
    /// Creates a recovery over an established connection.
    #[must_use]
    pub fn new(
        connection: Arc<Connection>,
        manager: Arc<SubscriptionManager>,
        books: Arc<dyn BookRecovery>,
    ) -> Self {
        Self {
            connection,
            manager,
            books,
        }
    }

    /// Runs until the connection closes, restoring the adapter's state after every gap.
    ///
    /// It restores nothing on the session the connection was made on: that session is up before
    /// this is called, and everything derived from it was derived from it already.
    pub async fn run(self) {
        let mut state = self.connection.state();
        let mut closed = self.connection.closed();

        // The session the connection was made on is not restored: whatever was derived from it was
        // derived from it already, before this was called.
        //
        // Sessions are told apart by the identifier the gateway gave them rather than by counting
        // the states that pass, because the state is a watch: an outage can begin and end between
        // two polls of it, and a transition that was never observed cannot be counted. An
        // identifier survives being missed, and starting mid-outage needs no special case, because
        // there is then no session to skip.
        let mut current = self
            .connection
            .handshake()
            .map(|handshake| handshake.conn_id);

        loop {
            tokio::select! {
                changed = state.changed() => {
                    if changed.is_err() {
                        break;
                    }
                }
                changed = closed.changed() => {
                    if changed.is_err() || *closed.borrow() {
                        break;
                    }

                    continue;
                }
            }

            let ready = match &*state.borrow() {
                SessionState::Ready(handshake) => handshake.conn_id,
                SessionState::Restoring { .. } => continue,
            };

            if current == Some(ready) {
                continue;
            }

            current = Some(ready);
            log::info!("a session replaced a lost one; restoring the adapter's state");
            self.restore().await;
        }
    }

    /// Brings the entitlement, the subscriptions, and the books back onto the current session.
    async fn restore(&self) {
        match Capabilities::read(&self.connection).await {
            Ok(capabilities) => log::info!(
                "the replacement session reports US equity quotes {} and HK equity quotes {}",
                capabilities.equity_quote_right(Market::Us).as_str(),
                capabilities.equity_quote_right(Market::Hk).as_str(),
            ),
            Err(e) => log::warn!("cannot re-read the entitlement after a gap: {e}"),
        }

        if let Err(e) = self.manager.replay().await {
            log::error!("cannot replay the subscriptions after a gap: {e}");
            return;
        }

        for subscription in self.manager.held() {
            if subscription.kind != SubscriptionType::OrderBook {
                continue;
            }

            if let Err(e) = self.reseed(&subscription).await {
                log::error!("cannot reseed the book for {subscription:?} after a gap: {e}");
            }
        }
    }

    /// Rebuilds one book from a fresh snapshot of the current session.
    async fn reseed(&self, subscription: &Subscription) -> anyhow::Result<()> {
        let Some(mut context) = self.books.context(subscription) else {
            return Ok(());
        };

        // The book is being built now, whatever instant the context was last used at.
        context.ts_init = get_atomic_clock_realtime().get_time_ns();

        let security = providers::security(subscription.market, &subscription.code);
        let depth = i32::try_from(context.depth).unwrap_or(providers::MAX_BOOK_DEPTH);
        let book =
            providers::request_order_book(&self.connection, security, depth, BOARD_NORMAL).await?;

        let ts_bid = side_time(book.svr_recv_time_bid_timestamp, context.ts_init);
        let ts_ask = side_time(book.svr_recv_time_ask_timestamp, context.ts_init);

        let deltas = book_deltas_from(
            &book.order_book_bid_list,
            &book.order_book_ask_list,
            &context,
            book.order_book_type,
            ts_bid,
            ts_ask,
        )?;

        log::info!(
            "reseeded {} for {subscription:?} with {} records",
            context.instrument_id,
            deltas.deltas.len(),
        );

        self.books.seeded(subscription.clone(), deltas);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Mutex, time::Duration};

    use nautilus_core::UnixNanos;
    use prost::Message as _;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{
            TcpListener,
            tcp::{OwnedReadHalf, OwnedWriteHalf},
        },
        sync::mpsc,
    };

    use super::*;
    use crate::{
        codec::{decode_frame, encode_frame},
        connection::{ConnectOptions, Message, PROTO_ID_KEEP_ALIVE, RET_OK, connect},
        entitlement::PROTO_ID_GET_USER_INFO,
        generated::{get_user_info, init_connect, qot_common, qot_get_order_book, qot_sub},
        mappers::bars::{Adjustment, BarSession},
        providers::{PROTO_ID_GET_ORDER_BOOK, PROTO_ID_SUB},
    };

    const SERVER_VER: i32 = 1010;
    const REPORTED_KEEP_ALIVE_SECS: i32 = 10;

    /// What each session of the controlled gateway was asked, by protocol identifier.
    type Record = Arc<Mutex<Vec<Vec<u32>>>>;

    /// Reads one message, however many socket reads it takes.
    async fn recv_message(socket: &mut OwnedReadHalf, buffer: &mut Vec<u8>) -> Message {
        loop {
            if let Some(frame) = decode_frame(buffer).unwrap() {
                let message = Message {
                    proto_id: frame.header.proto_id,
                    serial_no: frame.header.serial_no,
                    body: frame.body.to_vec(),
                };
                let total_len = frame.total_len();
                buffer.drain(..total_len);
                return message;
            }

            let read = socket.read_buf(buffer).await.unwrap();
            assert!(read > 0, "the client closed the connection");
        }
    }

    async fn send_message(socket: &mut OwnedWriteHalf, proto_id: u32, serial_no: u32, body: &[u8]) {
        let frame = encode_frame(proto_id, serial_no, body).unwrap();
        socket.write_all(&frame).await.unwrap();
    }

    /// One book level as the gateway states it.
    fn level(price: f64, volume: i64) -> qot_common::OrderBook {
        qot_common::OrderBook {
            price,
            volume,
            oreder_count: 0,
            detail_list: Vec::new(),
            hp_volume: None,
        }
    }

    /// The answer a controlled gateway gives a request the adapter makes.
    fn answer(proto_id: u32) -> Vec<u8> {
        match proto_id {
            PROTO_ID_GET_USER_INFO => get_user_info::Response {
                ret_type: RET_OK,
                ret_msg: None,
                err_code: None,
                s2c: Some(get_user_info::S2c {
                    us_qot_right: Some(2),
                    hk_qot_right: Some(1),
                    sub_quota: Some(100),
                    history_kl_quota: Some(100),
                    ..Default::default()
                }),
            }
            .encode_to_vec(),
            PROTO_ID_SUB => qot_sub::Response {
                ret_type: RET_OK,
                ret_msg: None,
                err_code: None,
                s2c: Some(qot_sub::S2c::default()),
            }
            .encode_to_vec(),
            PROTO_ID_GET_ORDER_BOOK => qot_get_order_book::Response {
                ret_type: RET_OK,
                ret_msg: None,
                err_code: None,
                s2c: Some(qot_get_order_book::S2c {
                    security: qot_common::Security {
                        market: 11,
                        code: "AAPL".to_string(),
                    },
                    order_book_bid_list: vec![level(332.8, 24)],
                    order_book_ask_list: vec![level(332.83, 119)],
                    order_book_type: Some(BOARD_NORMAL),
                    ..Default::default()
                }),
            }
            .encode_to_vec(),
            other => panic!("the controlled gateway was asked for protocol {other}"),
        }
    }

    /// Starts a controlled gateway that handshakes `sessions` connections and records what each was
    /// asked for.
    ///
    /// The first session is dropped once it has answered `drop_after` requests, and a value of zero
    /// drops it as soon as it is up: either way the drop is the forced outage under test, and it is
    /// the gateway's doing rather than a socket the adapter was allowed to notice was idle.
    async fn spawn_gateway(sessions: usize, drop_after: usize) -> (u16, Record) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let record: Record = Arc::new(Mutex::new(vec![Vec::new(); sessions]));
        let shared = Arc::clone(&record);

        tokio::spawn(async move {
            for index in 0..sessions {
                let (stream, _) = listener.accept().await.unwrap();
                let (mut read, mut write) = stream.into_split();
                let mut buffer = Vec::new();

                let request = recv_message(&mut read, &mut buffer).await;
                assert_eq!(request.proto_id, 1001);

                let handshake = init_connect::Response {
                    ret_type: RET_OK,
                    ret_msg: None,
                    err_code: None,
                    s2c: Some(init_connect::S2c {
                        server_ver: SERVER_VER,
                        login_user_id: 7,
                        conn_id: index as u64 + 1,
                        conn_aes_key: "0123456789abcdef".to_string(),
                        keep_alive_interval: REPORTED_KEEP_ALIVE_SECS,
                        aes_cb_civ: None,
                        user_attribution: None,
                    }),
                };

                send_message(
                    &mut write,
                    request.proto_id,
                    request.serial_no,
                    &handshake.encode_to_vec(),
                )
                .await;

                if index == 0 && drop_after == 0 {
                    let _ = write.shutdown().await;
                    continue;
                }

                loop {
                    let request = recv_message(&mut read, &mut buffer).await;

                    // The keep-alive is never decoded by the adapter, so it is not answered.
                    if request.proto_id == PROTO_ID_KEEP_ALIVE {
                        continue;
                    }

                    let answered = {
                        let mut seen = shared.lock().unwrap();
                        seen[index].push(request.proto_id);
                        seen[index].len()
                    };

                    send_message(
                        &mut write,
                        request.proto_id,
                        request.serial_no,
                        &answer(request.proto_id),
                    )
                    .await;

                    if index == 0 && answered >= drop_after {
                        let _ = write.shutdown().await;
                        break;
                    }
                }
            }
        });

        (port, record)
    }

    /// Waits until a session has been asked for `proto_id`, and returns the whole record.
    async fn wait_for(record: &Record, session: usize, proto_id: u32) -> Vec<Vec<u32>> {
        tokio::time::timeout(Duration::from_secs(20), async {
            loop {
                {
                    let seen = record.lock().unwrap();

                    if seen[session].contains(&proto_id) {
                        return seen.clone();
                    }
                }

                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("the gateway should have been asked by now")
    }

    /// The book layer, holding the books it is told about and recording what it is given.
    #[derive(Debug)]
    struct Books {
        holds: Vec<Subscription>,
        seeded: Mutex<Vec<OrderBookDeltas>>,
    }

    impl Books {
        fn new(holds: Vec<Subscription>) -> Arc<Self> {
            Arc::new(Self {
                holds,
                seeded: Mutex::new(Vec::new()),
            })
        }
    }

    impl BookRecovery for Books {
        fn context(&self, subscription: &Subscription) -> Option<BookContext> {
            self.holds.contains(subscription).then(|| BookContext {
                instrument_id: format!("{}.{}", subscription.code, subscription.market).into(),
                price_precision: 2,
                size_precision: 0,
                depth: 10,
                ts_init: UnixNanos::default(),
            })
        }

        fn seeded(&self, _subscription: Subscription, deltas: OrderBookDeltas) {
            self.seeded.lock().unwrap().push(deltas);
        }
    }

    /// Connects, holds `subscriptions`, and runs a recovery over the connection.
    async fn start(
        sessions: usize,
        drop_after: usize,
        subscriptions: &[Subscription],
        books: &Arc<Books>,
    ) -> (
        Arc<Connection>,
        Arc<SubscriptionManager>,
        Record,
        tokio::task::JoinHandle<()>,
    ) {
        let (port, record) = spawn_gateway(sessions, drop_after).await;

        let options = ConnectOptions::new("127.0.0.1", port).with_reconnect_attempts(3);
        let (pushes, _push_rx) = mpsc::unbounded_channel();
        let connection = Arc::new(connect(&options, pushes).await.unwrap());

        let manager = SubscriptionManager::new(
            Arc::clone(&connection),
            Duration::from_secs(65),
            BarSession::default(),
            Adjustment::default(),
        );

        for subscription in subscriptions {
            manager.subscribe(subscription.clone()).await.unwrap();
        }

        let recovery = Recovery::new(
            Arc::clone(&connection),
            Arc::clone(&manager),
            Arc::clone(books) as Arc<dyn BookRecovery>,
        );

        let handle = tokio::spawn(recovery.run());

        (connection, manager, record, handle)
    }

    /// A replaced session re-reads the entitlement, replays what the consumers want, and reseeds
    /// the book, in that order, because the gateway refuses the book until the subscription is back.
    #[tokio::test]
    async fn test_a_replacement_session_replays_the_subscriptions_and_reseeds_the_book() {
        let book = Subscription::new(Market::Us, "AAPL", SubscriptionType::OrderBook);
        let books = Books::new(vec![book.clone()]);

        let (connection, manager, record, handle) =
            start(2, 1, std::slice::from_ref(&book), &books).await;

        // The first session answers the subscribe and is then dropped, so the book request below can
        // only have come from the replacement.
        let seen = wait_for(&record, 1, PROTO_ID_GET_ORDER_BOOK).await;

        assert_eq!(
            seen[0],
            vec![PROTO_ID_SUB],
            "the first session was asked for the subscription and nothing else",
        );
        assert_eq!(
            seen[1],
            vec![
                PROTO_ID_GET_USER_INFO,
                PROTO_ID_SUB,
                PROTO_ID_GET_ORDER_BOOK
            ],
            "the replacement re-read the entitlement, replayed the subscription, then reseeded",
        );
        assert!(
            manager.held().contains(&book),
            "the subscription is held again on the replacement",
        );

        {
            let seeded = books.seeded.lock().unwrap();
            assert_eq!(seeded.len(), 1, "one book was reseeded");
            assert_eq!(seeded[0].deltas.len(), 3, "a clear and one record a side");
        }

        connection.shutdown();
        handle.await.unwrap();
    }

    /// A replacement with nothing held restores nothing but the entitlement it re-reads.
    #[tokio::test]
    async fn test_a_replacement_session_with_nothing_held_only_re_reads_the_entitlement() {
        let books = Books::new(Vec::new());
        let (connection, _manager, record, handle) = start(2, 0, &[], &books).await;

        wait_for(&record, 1, PROTO_ID_GET_USER_INFO).await;

        // A replay and a reseed would follow the entitlement read immediately, so a pause with
        // neither is what shows there was nothing to restore.
        tokio::time::sleep(Duration::from_millis(500)).await;

        assert_eq!(record.lock().unwrap()[1], vec![PROTO_ID_GET_USER_INFO]);
        assert!(books.seeded.lock().unwrap().is_empty());

        connection.shutdown();
        handle.await.unwrap();
    }

    /// A subscription that is not a book is replayed and nothing is reseeded for it: a book that
    /// was never held is not a book to rebuild.
    #[tokio::test]
    async fn test_a_replacement_session_reseeds_nothing_for_a_subscription_that_is_not_a_book() {
        let ticker = Subscription::new(Market::Us, "AAPL", SubscriptionType::Ticker);
        let books = Books::new(vec![ticker.clone()]);

        let (connection, manager, record, handle) =
            start(2, 1, std::slice::from_ref(&ticker), &books).await;

        wait_for(&record, 1, PROTO_ID_SUB).await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        assert_eq!(
            record.lock().unwrap()[1],
            vec![PROTO_ID_GET_USER_INFO, PROTO_ID_SUB],
            "the subscription is replayed and no book is asked for",
        );
        assert!(manager.held().contains(&ticker));
        assert!(books.seeded.lock().unwrap().is_empty());

        connection.shutdown();
        handle.await.unwrap();
    }
}
