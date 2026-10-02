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

//! The connection to a local OpenD gateway.
//!
//! A connection is three tasks and a pending map. One task owns the write half of the socket and
//! takes encoded frames from a channel; one owns the read half, decodes frames, and dispatches
//! them; one sends the keep-alive on the interval the gateway negotiated. The pending map holds a
//! oneshot sender per outstanding request, keyed by protocol identifier and serial number, which is
//! what makes responses find the request that caused them regardless of the order they arrive in.
//!
//! # The handshake comes first
//!
//! The gateway expects one `InitConnect` request before anything else, and its response carries the
//! keep-alive interval, so the handshake must complete before the heartbeat can be started.
//!
//! # The heartbeat is a request
//!
//! The keep-alive is sent on four fifths of the negotiated interval, which is the reduction the
//! shipped client applies to leave margin against the gateway's own timeout. It is a full request
//! with a timeout rather than a bare write, because its failure is how a dead connection is
//! noticed: the gateway drops a socket that stays quiet beyond its window, and a keep-alive that
//! goes unanswered is the same condition seen earlier.
//!
//! # What closes a connection
//!
//! A frame that fails to decode in a way that loses the frame boundary is fatal, and so is the peer
//! closing the socket. A frame whose digest does not match is not fatal: its extent is known, so it
//! is reported and skipped. When the connection ends, every waiter is released through its oneshot
//! channel rather than left to its timeout.

use std::{
    collections::HashMap,
    sync::{
        Mutex, PoisonError,
        atomic::{AtomicU32, Ordering},
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use prost::Message as _;
use thiserror::Error;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpStream, tcp::OwnedReadHalf, tcp::OwnedWriteHalf},
    sync::{mpsc, oneshot, watch},
    time::{MissedTickBehavior, interval, timeout},
};

use crate::{
    codec::{CodecError, PROTO_FMT_PROTOBUF, decode_frame, encode_frame},
    common::CLIENT_ID,
    generated::{init_connect, keep_alive},
};

/// The client version the gateway expects, taken from the shipped client's own default.
pub const CLIENT_VERSION: i32 = 300;

/// The protocol identifier of the handshake.
pub const PROTO_ID_INIT_CONNECT: u32 = 1001;

/// The protocol identifier of the keep-alive.
pub const PROTO_ID_KEEP_ALIVE: u32 = 1004;

/// The protocol identifiers the gateway pushes without a request.
///
/// This is the set the protocol defines, restricted to the channels this adapter uses. `Notify`
/// belongs to it, because the gateway sends market state and connection events unprompted in the
/// same way it sends quotes, and the shipped client classifies it as a push for that reason.
pub const PUSH_PROTO_IDS: [u32; 5] = [
    1003, // Notify
    3005, // Qot_UpdateBasicQot
    3007, // Qot_UpdateKL
    3011, // Qot_UpdateTicker
    3013, // Qot_UpdateOrderBook
];

/// ``PacketEncAlgo_None``: the loopback path this adapter supports sends plaintext.
///
/// The default when the field is absent is ``PacketEncAlgo_FTAES_ECB``, so this is sent explicitly
/// rather than left out.
pub const PACKET_ENC_ALGO_NONE: i32 = -1;

/// The ``retType`` of a successful response.
pub const RET_OK: i32 = 0;

/// The request timeout used when the caller does not supply one.
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// The fraction of the negotiated keep-alive interval the heartbeat actually uses.
///
/// The shipped client applies the same reduction, leaving margin against the gateway's timeout.
const KEEP_ALIVE_NUMERATOR: u32 = 4;
const KEEP_ALIVE_DENOMINATOR: u32 = 5;

/// Returns whether the gateway pushes this protocol identifier without a request.
#[must_use]
pub fn is_push_proto_id(proto_id: u32) -> bool {
    PUSH_PROTO_IDS.contains(&proto_id)
}

/// How to reach a gateway.
#[derive(Debug, Clone)]
pub struct ConnectOptions {
    /// The gateway host.
    pub host: String,
    /// The gateway port.
    pub port: u16,
    /// The client identifier sent in the handshake.
    pub client_id: String,
    /// How long a request waits for its response.
    pub request_timeout: Duration,
}

impl ConnectOptions {
    /// Creates options for `host` and `port` with the default client identifier and timeout.
    #[must_use]
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
            client_id: CLIENT_ID.to_string(),
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
        }
    }

    /// Sets the client identifier sent in the handshake.
    #[must_use]
    pub fn with_client_id(mut self, client_id: impl Into<String>) -> Self {
        self.client_id = client_id.into();
        self
    }

    /// Sets how long a request waits for its response.
    #[must_use]
    pub fn with_request_timeout(mut self, request_timeout: Duration) -> Self {
        self.request_timeout = request_timeout;
        self
    }
}

/// What the gateway reported in its handshake response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Handshake {
    /// The gateway's version.
    pub server_ver: i32,
    /// The user the gateway is logged in as.
    pub login_user_id: u64,
    /// The gateway's identifier for this connection.
    pub conn_id: u64,
    /// The interval the gateway asked to be kept alive over.
    pub reported_keep_alive: Duration,
    /// The interval the heartbeat actually uses, which is four fifths of the reported one.
    pub keep_alive: Duration,
}

/// A message received from the gateway.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message {
    /// The protocol identifier, which selects the message type.
    pub proto_id: u32,
    /// The serial number, which is zero for an unsolicited push.
    pub serial_no: u32,
    /// The encoded body.
    pub body: Vec<u8>,
}

/// The ways a connection can fail.
#[derive(Debug, Error)]
pub enum ConnectionError {
    /// The socket failed, or could not be established.
    #[error("socket failure: {0}")]
    Io(#[from] std::io::Error),
    /// A frame could not be encoded or decoded.
    #[error("frame codec failure: {0}")]
    Codec(#[from] CodecError),
    /// The gateway refused the handshake, or answered it with something unusable.
    #[error("handshake failed: {0}")]
    Handshake(String),
    /// The connection ended while a request was outstanding.
    #[error("the connection is closed")]
    Closed,
    /// No response arrived within the request timeout.
    #[error("no response to protocol {proto_id} within {timeout:?}")]
    RequestTimeout {
        /// The protocol identifier that went unanswered.
        proto_id: u32,
        /// The timeout that elapsed.
        timeout: Duration,
    },
}

type Pending = Mutex<HashMap<(u32, u32), oneshot::Sender<Message>>>;

/// The request machinery, shared by the connection and the heartbeat task.
#[derive(Debug, Clone)]
struct Requester {
    frames: mpsc::UnboundedSender<Vec<u8>>,
    pending: std::sync::Arc<Pending>,
    next_serial: std::sync::Arc<AtomicU32>,
    timeout: Duration,
}

impl Requester {
    /// Sends a request and waits for the response that carries its serial number.
    async fn request(&self, proto_id: u32, body: &[u8]) -> Result<Message, ConnectionError> {
        let serial_no = self.next_serial.fetch_add(1, Ordering::Relaxed);
        let key = (proto_id, serial_no);
        let frame = encode_frame(proto_id, serial_no, body)?;

        let (tx, rx) = oneshot::channel();
        lock(&self.pending).insert(key, tx);

        if self.frames.send(frame).is_err() {
            lock(&self.pending).remove(&key);
            return Err(ConnectionError::Closed);
        }

        match timeout(self.timeout, rx).await {
            Ok(Ok(message)) => Ok(message),
            // The sender is dropped when the reader ends or when the entry is removed, so a
            // cancelled request and a dead connection are the same condition here.
            Ok(Err(_)) => Err(ConnectionError::Closed),
            Err(_) => {
                lock(&self.pending).remove(&key);
                Err(ConnectionError::RequestTimeout {
                    proto_id,
                    timeout: self.timeout,
                })
            }
        }
    }

    /// Performs the handshake and returns what the gateway reported.
    async fn handshake(&self, client_id: &str) -> Result<Handshake, ConnectionError> {
        let request = init_connect::Request {
            c2s: init_connect::C2s {
                client_ver: CLIENT_VERSION,
                client_id: client_id.to_string(),
                recv_notify: Some(true),
                packet_enc_algo: Some(PACKET_ENC_ALGO_NONE),
                push_proto_fmt: Some(i32::from(PROTO_FMT_PROTOBUF)),
                // Both are optional telemetry. Leaving them unset sends nothing rather than
                // claiming a language or an AI usage this adapter does not have.
                programming_language: None,
                ai_type: None,
            },
        };

        let message = self
            .request(
                PROTO_ID_INIT_CONNECT,
                &prost::Message::encode_to_vec(&request),
            )
            .await?;

        let response = init_connect::Response::decode(message.body.as_slice())
            .map_err(|e| ConnectionError::Handshake(format!("cannot decode the response: {e}")))?;

        if response.ret_type != RET_OK {
            return Err(ConnectionError::Handshake(format!(
                "the gateway returned retType {} with message {:?}",
                response.ret_type,
                response.ret_msg.unwrap_or_default()
            )));
        }

        let s2c = response
            .s2c
            .ok_or_else(|| ConnectionError::Handshake("the response carried no s2c".to_string()))?;

        let reported = if s2c.keep_alive_interval > 0 {
            Duration::from_secs(s2c.keep_alive_interval.unsigned_abs().into())
        } else {
            return Err(ConnectionError::Handshake(format!(
                "the gateway reported a keep-alive interval of {}",
                s2c.keep_alive_interval
            )));
        };

        Ok(Handshake {
            server_ver: s2c.server_ver,
            login_user_id: s2c.login_user_id,
            conn_id: s2c.conn_id,
            reported_keep_alive: reported,
            keep_alive: reduce_keep_alive(reported),
        })
    }
}

/// Locks the pending map, taking the contents even if a previous holder panicked.
///
/// Poisoning is not meaningful here: the map has no invariant that a panic could have broken, and
/// refusing to use it would turn a panic in one task into a dead connection.
fn lock(
    pending: &Pending,
) -> std::sync::MutexGuard<'_, HashMap<(u32, u32), oneshot::Sender<Message>>> {
    pending.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Applies the same reduction to the keep-alive interval that the shipped client applies.
fn reduce_keep_alive(reported: Duration) -> Duration {
    reported * KEEP_ALIVE_NUMERATOR / KEEP_ALIVE_DENOMINATOR
}

fn unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or_default()
}

/// An established, handshaken connection to a gateway.
#[derive(Debug)]
pub struct Connection {
    requester: Requester,
    handshake: Handshake,
    closed: watch::Receiver<bool>,
}

impl Connection {
    /// Returns what the gateway reported in the handshake.
    #[must_use]
    pub fn handshake(&self) -> &Handshake {
        &self.handshake
    }

    /// Returns a receiver that reports whether the connection has ended.
    #[must_use]
    pub fn closed(&self) -> watch::Receiver<bool> {
        self.closed.clone()
    }

    /// Sends a request and waits for its response.
    ///
    /// # Errors
    ///
    /// Returns [`ConnectionError::Codec`] if the request cannot be framed,
    /// [`ConnectionError::Closed`] if the connection ends while the request is outstanding, and
    /// [`ConnectionError::RequestTimeout`] if no response arrives within the configured timeout.
    pub async fn request(&self, proto_id: u32, body: &[u8]) -> Result<Message, ConnectionError> {
        self.requester.request(proto_id, body).await
    }
}

/// Connects to the gateway, performs the handshake, and starts the reader, writer, and heartbeat.
///
/// Push frames, and notifications, are delivered to `pushes` as they arrive.
///
/// # Errors
///
/// Returns [`ConnectionError::Io`] if the socket cannot be established, or
/// [`ConnectionError::Handshake`] if the gateway refuses the handshake or answers it with something
/// unusable.
pub async fn connect(
    options: &ConnectOptions,
    pushes: mpsc::UnboundedSender<Message>,
) -> Result<Connection, ConnectionError> {
    let stream = TcpStream::connect((options.host.as_str(), options.port)).await?;
    // The feed is latency sensitive and the frames are small, so coalescing would only add delay.
    stream.set_nodelay(true)?;

    let (read_half, write_half) = stream.into_split();

    let pending: std::sync::Arc<Pending> = std::sync::Arc::new(Mutex::new(HashMap::new()));
    let (frames, frame_rx) = mpsc::unbounded_channel();
    let (closed_tx, closed_rx) = watch::channel(false);

    tokio::spawn(write_task(write_half, frame_rx));
    tokio::spawn(read_task(
        read_half,
        std::sync::Arc::clone(&pending),
        pushes,
        closed_tx.clone(),
    ));

    let requester = Requester {
        frames,
        pending,
        next_serial: std::sync::Arc::new(AtomicU32::new(1)),
        timeout: options.request_timeout,
    };

    let handshake = requester.handshake(&options.client_id).await?;

    tokio::spawn(heartbeat_task(
        requester.clone(),
        handshake.keep_alive,
        closed_tx,
    ));

    Ok(Connection {
        requester,
        handshake,
        closed: closed_rx,
    })
}

/// Writes frames as they are queued, and shuts the socket down when the queue closes.
async fn write_task(mut socket: OwnedWriteHalf, mut frames: mpsc::UnboundedReceiver<Vec<u8>>) {
    while let Some(frame) = frames.recv().await {
        if let Err(e) = socket.write_all(&frame).await {
            log::debug!("write failed, closing the connection: {e}");
            break;
        }
    }

    let _ = socket.shutdown().await;
}

/// Decodes frames and dispatches them until the socket ends or a frame loses the boundary.
async fn read_task(
    mut socket: OwnedReadHalf,
    pending: std::sync::Arc<Pending>,
    pushes: mpsc::UnboundedSender<Message>,
    closed: watch::Sender<bool>,
) {
    let mut buffer: Vec<u8> = Vec::with_capacity(16 * 1024);

    'reading: loop {
        match socket.read_buf(&mut buffer).await {
            Ok(0) => {
                log::debug!("the gateway closed the connection");
                break;
            }
            Ok(_) => {}
            Err(e) => {
                log::debug!("read failed, closing the connection: {e}");
                break;
            }
        }

        let mut consumed = 0;
        loop {
            match decode_frame(&buffer[consumed..]) {
                Ok(Some(frame)) => {
                    let total_len = frame.total_len();
                    let message = Message {
                        proto_id: frame.header.proto_id,
                        serial_no: frame.header.serial_no,
                        body: frame.body.to_vec(),
                    };
                    consumed += total_len;
                    dispatch(message, &pending, &pushes);
                }
                Ok(None) => break,
                Err(CodecError::DigestMismatch {
                    proto_id,
                    total_len,
                    ..
                }) => {
                    // The extent is known, so this frame is skipped rather than fatal.
                    log::warn!(
                        "discarding a corrupt frame for protocol {proto_id} over {total_len} bytes"
                    );
                    consumed += total_len;
                }
                Err(e) => {
                    log::error!("closing the connection, the frame boundary is lost: {e}");
                    break 'reading;
                }
            }
        }

        if consumed > 0 {
            buffer.drain(..consumed);
        }
    }

    // Releasing the waiters here is what turns a dead connection into an immediate error for every
    // outstanding request, instead of each one waiting out its own timeout.
    lock(&pending).clear();
    let _ = closed.send(true);
}

/// Routes one decoded message: to its waiting request, to the push channel, or nowhere.
fn dispatch(message: Message, pending: &Pending, pushes: &mpsc::UnboundedSender<Message>) {
    let key = (message.proto_id, message.serial_no);

    if let Some(waiter) = lock(pending).remove(&key) {
        // A send failure means the caller stopped waiting, which is not an error here.
        let _ = waiter.send(message);
        return;
    }

    if is_push_proto_id(message.proto_id) {
        if pushes.send(message).is_err() {
            log::debug!("the data client is gone; dropping a push frame");
        }
        return;
    }

    log::warn!(
        "ignoring a response for protocol {} with serial {} that no request is waiting for",
        message.proto_id,
        message.serial_no
    );
}

/// Sends the keep-alive on the negotiated interval until it goes unanswered.
async fn heartbeat_task(requester: Requester, every: Duration, closed: watch::Sender<bool>) {
    let mut ticker = interval(every);
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    // The first tick of an interval completes immediately, and the handshake has just happened.
    ticker.tick().await;

    loop {
        ticker.tick().await;

        let request = keep_alive::Request {
            c2s: keep_alive::C2s {
                time: unix_seconds(),
            },
        };
        let body = prost::Message::encode_to_vec(&request);

        if let Err(e) = requester.request(PROTO_ID_KEEP_ALIVE, &body).await {
            log::warn!("keep-alive failed, closing the connection: {e}");
            let _ = closed.send(true);
            return;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::future::Future;

    use tokio::net::TcpListener;

    use super::*;

    const CLIENT_ID: &str = "test-client";
    const SERVER_VER: i32 = 1010;
    const CONN_ID: u64 = 99;
    const REPORTED_KEEP_ALIVE_SECS: i32 = 10;

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

    /// Starts a controlled gateway on an ephemeral port, answers the handshake, and then runs
    /// `script` over the rest of the connection.
    ///
    /// The handshake assertions live here rather than in each test because every test needs the
    /// exchange to succeed, and one place asserting the request shape keeps them honest.
    async fn spawn_gateway<F, Fut>(script: F) -> u16
    where
        F: FnOnce(OwnedWriteHalf, OwnedReadHalf) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send,
    {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let (mut read, mut write) = stream.into_split();
            let mut buffer = Vec::new();

            let request = recv_message(&mut read, &mut buffer).await;
            assert_eq!(request.proto_id, PROTO_ID_INIT_CONNECT);

            let decoded = init_connect::Request::decode(request.body.as_slice()).unwrap();
            assert_eq!(decoded.c2s.client_ver, CLIENT_VERSION);
            assert_eq!(decoded.c2s.client_id, CLIENT_ID);
            assert_eq!(decoded.c2s.recv_notify, Some(true));
            assert_eq!(decoded.c2s.packet_enc_algo, Some(PACKET_ENC_ALGO_NONE));
            assert_eq!(
                decoded.c2s.push_proto_fmt,
                Some(i32::from(PROTO_FMT_PROTOBUF))
            );
            assert!(decoded.c2s.programming_language.is_none());
            assert!(decoded.c2s.ai_type.is_none());

            let response = init_connect::Response {
                ret_type: RET_OK,
                ret_msg: None,
                err_code: None,
                s2c: Some(init_connect::S2c {
                    server_ver: SERVER_VER,
                    login_user_id: 7,
                    conn_id: CONN_ID,
                    conn_aes_key: "0123456789abcdef".to_string(),
                    keep_alive_interval: REPORTED_KEEP_ALIVE_SECS,
                    aes_cb_civ: None,
                    user_attribution: None,
                }),
            };

            send_message(
                &mut write,
                PROTO_ID_INIT_CONNECT,
                request.serial_no,
                &prost::Message::encode_to_vec(&response),
            )
            .await;

            script(write, read).await;
        });

        port
    }

    async fn connect_to(port: u16) -> (Connection, mpsc::UnboundedReceiver<Message>) {
        let (pushes, push_rx) = mpsc::unbounded_channel();
        let options = ConnectOptions::new("127.0.0.1", port).with_client_id(CLIENT_ID);
        let connection = connect(&options, pushes).await.unwrap();
        (connection, push_rx)
    }

    #[tokio::test]
    async fn test_handshake_reports_the_gateway_state() {
        let port = spawn_gateway(|_write, _read| async {}).await;
        let (connection, _pushes) = connect_to(port).await;

        let handshake = connection.handshake();

        assert_eq!(handshake.server_ver, SERVER_VER);
        assert_eq!(handshake.conn_id, CONN_ID);
        assert_eq!(handshake.login_user_id, 7);
        assert_eq!(
            handshake.reported_keep_alive,
            Duration::from_secs(u64::from(REPORTED_KEEP_ALIVE_SECS.unsigned_abs()))
        );
        assert_eq!(
            handshake.keep_alive,
            Duration::from_secs(8),
            "the heartbeat uses four fifths of what the gateway asked for"
        );
    }

    #[tokio::test]
    async fn test_request_and_response_round_trip() {
        let port = spawn_gateway(|mut write, mut read| async move {
            let mut buffer = Vec::new();
            let request = recv_message(&mut read, &mut buffer).await;

            assert_eq!(request.proto_id, 3202);
            assert_eq!(request.body, b"query");
            assert!(request.serial_no > 0, "a request carries a real serial");

            send_message(&mut write, request.proto_id, request.serial_no, b"answer").await;
        })
        .await;

        let (connection, mut pushes) = connect_to(port).await;
        let response = connection.request(3202, b"query").await.unwrap();

        assert_eq!(response.proto_id, 3202);
        assert_eq!(response.body, b"answer");
        assert!(
            pushes.try_recv().is_err(),
            "a response is not a push and must not reach the data client"
        );
    }

    /// Responses are matched by protocol identifier and serial number, so an answer that overtakes
    /// another still reaches the request that caused it.
    #[tokio::test]
    async fn test_responses_find_their_request_out_of_order() {
        let port = spawn_gateway(|mut write, mut read| async move {
            let mut buffer = Vec::new();
            let first = recv_message(&mut read, &mut buffer).await;
            let second = recv_message(&mut read, &mut buffer).await;

            assert_ne!(first.serial_no, second.serial_no);

            send_message(&mut write, second.proto_id, second.serial_no, b"second").await;
            send_message(&mut write, first.proto_id, first.serial_no, b"first").await;
        })
        .await;

        let (connection, _pushes) = connect_to(port).await;

        let (first, second) = tokio::join!(
            connection.request(3010, b"one"),
            connection.request(3010, b"two"),
        );

        let first = first.unwrap();
        let second = second.unwrap();

        assert_eq!(first.body, b"first");
        assert_eq!(second.body, b"second");
        assert!(first.serial_no < second.serial_no);
    }

    /// A push is anything the gateway sends unprompted, and a notification is one of those; a
    /// response nobody is waiting for is neither and must not be mistaken for one.
    #[tokio::test]
    async fn test_pushes_reach_the_data_client_and_stray_responses_do_not() {
        let port = spawn_gateway(|mut write, mut _read| async move {
            send_message(&mut write, 3011, 0, b"ticker").await;
            send_message(&mut write, 1003, 0, b"notify").await;
            // A response for a serial nobody asked for.
            send_message(&mut write, 3004, 9999, b"stray").await;
            tokio::time::sleep(Duration::from_millis(200)).await;
        })
        .await;

        let (connection, mut pushes) = connect_to(port).await;

        let first = tokio::time::timeout(Duration::from_secs(2), pushes.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(first.proto_id, 3011);
        assert_eq!(first.body, b"ticker");

        let second = tokio::time::timeout(Duration::from_secs(2), pushes.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(second.proto_id, 1003);
        assert_eq!(second.body, b"notify");

        assert!(
            pushes.try_recv().is_err(),
            "a stray response must not be delivered as a push"
        );

        drop(connection);
    }

    /// A gateway that closes the socket releases outstanding requests immediately rather than
    /// leaving them to their timeout, and the closure is observable.
    #[tokio::test]
    async fn test_a_closed_socket_releases_the_waiters() {
        let port = spawn_gateway(|mut write, _read| async move {
            // Accept the request, then drop the write half without answering.
            tokio::time::sleep(Duration::from_millis(50)).await;
            let _ = write.shutdown().await;
        })
        .await;

        let (connection, _pushes) = connect_to(port).await;
        let mut closed = connection.closed();

        let result = connection.request(3202, b"query").await;
        assert!(
            matches!(result, Err(ConnectionError::Closed)),
            "expected the request to be released, got {result:?}"
        );

        tokio::time::timeout(Duration::from_secs(2), closed.changed())
            .await
            .expect("the closure should be observable")
            .expect("the sender should not be dropped");
        assert!(*closed.borrow());
    }
}
