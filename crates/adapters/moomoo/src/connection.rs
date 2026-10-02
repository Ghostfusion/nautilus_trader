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
    net::{
        TcpStream,
        tcp::{OwnedReadHalf, OwnedWriteHalf},
    },
    sync::{mpsc, oneshot, watch},
    time::{Instant, MissedTickBehavior, interval, sleep, timeout},
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
    /// How many consecutive reconnect attempts are made before the supervisor gives up.
    ///
    /// `None`, the default, retries indefinitely: a gateway that is down for an hour is one that
    /// comes back, and an adapter that stopped trying while the market was open would be worse than
    /// one that kept trying. A limit is for a caller with somewhere else to go, such as a failover
    /// chain, and for a test that must not leave a task retrying behind it.
    pub reconnect_attempts: Option<u32>,
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
            reconnect_attempts: None,
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

    /// Sets how many consecutive reconnect attempts are made before the supervisor gives up.
    #[must_use]
    pub fn with_reconnect_attempts(mut self, attempts: u32) -> Self {
        self.reconnect_attempts = Some(attempts);
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

/// What the connection is currently doing.
///
/// A session is one socket with one handshake behind it, and a connection outlives its sessions: a
/// dropped socket is replaced, and the requests and pushes carried by the connection do not care
/// which session they travelled on. A caller holding anything derived from a session, such as an
/// entitlement read or a book built from a stream, watches this to learn that what it holds came
/// from a socket that has gone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionState {
    /// The socket is up and the gateway has answered its handshake.
    Ready(Handshake),
    /// The socket is down, and this many consecutive attempts to restore it have failed.
    ///
    /// The first state published after a drop carries zero, because no attempt has failed yet, and
    /// a session that is established and then drops again publishes zero afresh: the count measures
    /// the current outage, not the connection's lifetime.
    Restoring {
        /// The number of consecutive attempts that have failed.
        attempts: u32,
    },
}

impl SessionState {
    /// Returns the handshake, when there is a session behind it.
    #[must_use]
    pub fn handshake(&self) -> Option<&Handshake> {
        match self {
            Self::Ready(handshake) => Some(handshake),
            Self::Restoring { .. } => None,
        }
    }
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

/// A connection to a gateway, which outlives the sockets that carry it.
///
/// A dropped socket is replaced rather than ending the connection, so a caller keeps one handle for
/// the life of the adapter and watches [`Connection::state`] to learn when the socket behind it
/// changed.
#[derive(Debug)]
pub struct Connection {
    requester: Requester,
    session: watch::Receiver<SessionState>,
    closed: watch::Receiver<bool>,
    stop: watch::Sender<bool>,
}

impl Connection {
    /// Returns a receiver reporting what the connection is doing.
    ///
    /// A caller holding anything derived from a session watches this. Every [`SessionState::Ready`]
    /// after the first is a session the caller has to bring its own state back onto, by replaying
    /// what it wants and by reseeding anything it built from a stream rather than from an answer.
    #[must_use]
    pub fn state(&self) -> watch::Receiver<SessionState> {
        self.session.clone()
    }

    /// Returns what the gateway reported in the current session's handshake.
    ///
    /// `None` while there is no session, which lasts the whole of an outage.
    #[must_use]
    pub fn handshake(&self) -> Option<Handshake> {
        self.session.borrow().handshake().cloned()
    }

    /// Returns a receiver reporting whether the supervisor has stopped for good.
    ///
    /// This is not the same as being mid-outage. A dropped socket is replaced, and this becomes true
    /// only once the supervisor stops, whether it was asked to or it ran out of attempts.
    #[must_use]
    pub fn closed(&self) -> watch::Receiver<bool> {
        self.closed.clone()
    }

    /// Asks the supervisor to stop, which closes the socket and ends the connection.
    ///
    /// It does not wait for the teardown, because the caller has nothing to gain from watching a
    /// socket close and the supervisor may be asleep in a backoff.
    pub fn shutdown(&self) {
        let _ = self.stop.send(true);
    }

    /// Sends a request and waits for its response.
    ///
    /// The request travels on whichever session is current when it is written. A request outstanding
    /// when a socket drops is released at once with [`ConnectionError::Closed`] rather than waiting
    /// out its timeout, and it is not repeated here: whether a request may be sent twice is a
    /// question about the request, which this layer does not know, so the decision stays with the
    /// caller that made it.
    ///
    /// # Errors
    ///
    /// Returns [`ConnectionError::Codec`] if the request cannot be framed,
    /// [`ConnectionError::Closed`] if there is no session or the current one ends while the request
    /// is outstanding, and [`ConnectionError::RequestTimeout`] if no response arrives within the
    /// configured timeout.
    pub async fn request(&self, proto_id: u32, body: &[u8]) -> Result<Message, ConnectionError> {
        self.requester.request(proto_id, body).await
    }
}

/// Connects to the gateway, performs the first handshake, and starts the supervisor.
///
/// Push frames, and notifications, are delivered to `pushes` as they arrive.
///
/// A gateway that cannot be reached, or that refuses the handshake, is an error from this call: the
/// supervisor's job is to replace a socket that was working, and a caller that cannot connect at all
/// has somewhere else to go, such as the next provider in a failover chain.
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

    let pending: std::sync::Arc<Pending> = std::sync::Arc::new(Mutex::new(HashMap::new()));
    let (frames, frame_rx) = mpsc::unbounded_channel();
    let (session_tx, session_rx) = watch::channel(SessionState::Restoring { attempts: 0 });
    let (closed_tx, closed_rx) = watch::channel(false);
    let (stop_tx, stop_rx) = watch::channel(false);
    let (first_tx, first_rx) = oneshot::channel();

    let requester = Requester {
        frames,
        pending: std::sync::Arc::clone(&pending),
        next_serial: std::sync::Arc::new(AtomicU32::new(1)),
        timeout: options.request_timeout,
    };

    let supervisor = Supervisor {
        shared: Shared {
            options: options.clone(),
            requester: requester.clone(),
            pending,
            pushes,
            session: session_tx,
            closed: closed_tx,
        },
        frames: frame_rx,
        stopped: stop_rx,
    };

    tokio::spawn(supervisor.run(stream, first_tx));

    match first_rx.await {
        Ok(Ok(())) => Ok(Connection {
            requester,
            session: session_rx,
            closed: closed_rx,
            stop: stop_tx,
        }),
        Ok(Err(message)) => {
            let _ = stop_tx.send(true);
            Err(ConnectionError::Handshake(message))
        }
        // The supervisor sends its first outcome before anything else, so a missing one means it
        // ended before it could.
        Err(_) => Err(ConnectionError::Closed),
    }
}

/// How long a session has to last before the outage that ended it is treated as over.
const STABLE_SESSION: Duration = Duration::from_secs(30);

/// The pause before the first attempt to replace a session.
const RECONNECT_DELAY: Duration = Duration::from_secs(1);

/// The longest pause between attempts to replace a session.
const RECONNECT_DELAY_MAX: Duration = Duration::from_secs(30);

/// Returns how long to wait before the next attempt.
///
/// The wait doubles with the attempts that have failed in a row and stops at the maximum, so a
/// gateway down for an hour is asked about a hundred and twenty times rather than three thousand
/// six hundred.
fn backoff(attempts: u32) -> Duration {
    let doublings = attempts.min(5);
    (RECONNECT_DELAY * (1 << doublings)).min(RECONNECT_DELAY_MAX)
}

/// The pieces of a connection that outlive any one socket.
#[derive(Debug, Clone)]
struct Shared {
    options: ConnectOptions,
    requester: Requester,
    pending: std::sync::Arc<Pending>,
    pushes: mpsc::UnboundedSender<Message>,
    session: watch::Sender<SessionState>,
    closed: watch::Sender<bool>,
}

/// Owns the socket, and replaces it when it drops.
#[derive(Debug)]
struct Supervisor {
    shared: Shared,
    frames: mpsc::UnboundedReceiver<Vec<u8>>,
    stopped: watch::Receiver<bool>,
}

impl Supervisor {
    /// Runs sessions, replacing the socket after each one ends, until stopping or giving up.
    ///
    /// The first socket arrives already open, because failing to open one is the caller's error to
    /// report rather than something to retry behind it.
    async fn run(mut self, first: TcpStream, first_outcome: oneshot::Sender<Result<(), String>>) {
        let mut attempts = 0;
        let mut stream = Some(first);
        let mut outcome = Some(first_outcome);

        loop {
            let socket = match stream.take() {
                Some(socket) => socket,
                None => {
                    // No reconnect happens without a pause first, so a gateway that is refusing
                    // connections is asked again at the backoff and not at the speed of this loop.
                    let wait = backoff(attempts);
                    log::warn!(
                        "the connection is down; trying again in {wait:?} (attempt {})",
                        attempts.saturating_add(1)
                    );

                    tokio::select! {
                        () = sleep(wait) => {}
                        _ = self.stopped.changed() => break,
                    }

                    let address = (self.shared.options.host.as_str(), self.shared.options.port);

                    match timeout(
                        self.shared.options.request_timeout,
                        TcpStream::connect(address),
                    )
                    .await
                    {
                        Ok(Ok(socket)) => {
                            if let Err(e) = socket.set_nodelay(true) {
                                log::warn!("cannot disable Nagle on the new socket: {e}");
                            }

                            socket
                        }
                        Ok(Err(e)) => {
                            attempts = attempts.saturating_add(1);

                            if !self.may_retry(attempts) {
                                break;
                            }

                            log::warn!("cannot reach the gateway: {e}");
                            continue;
                        }
                        Err(_) => {
                            attempts = attempts.saturating_add(1);

                            if !self.may_retry(attempts) {
                                break;
                            }

                            log::warn!("the gateway did not accept a connection in time");
                            continue;
                        }
                    }
                }
            };

            let started = Instant::now();
            let result = self.session(socket, outcome.take()).await;
            let lived = started.elapsed();

            // A session that was established and then lasted is a connection that recovered, so the
            // outage it ended is a new one and the wait starts over. Anything else is the same
            // outage continuing, however many sockets it has taken.
            match result {
                Ok(_) if lived >= STABLE_SESSION => attempts = 0,
                _ => attempts = attempts.saturating_add(1),
            }

            if *self.stopped.borrow() {
                break;
            }

            // Saying the socket is gone as soon as it is gone is what lets a caller stop trusting
            // what it holds without waiting for the next attempt to fail.
            let _ = self
                .shared
                .session
                .send(SessionState::Restoring { attempts });

            if !self.may_retry(attempts) {
                break;
            }
        }

        let _ = self.shared.closed.send(true);
    }

    /// Returns whether another attempt is allowed, reporting the one that is not.
    fn may_retry(&self, attempts: u32) -> bool {
        if let Some(limit) = self.shared.options.reconnect_attempts
            && attempts >= limit
        {
            log::error!("giving up after {attempts} attempts to restore the connection");
            return false;
        }

        true
    }

    /// Runs one session until it ends, returning the handshake of the session it established.
    ///
    /// An error means no session was established on this socket, which is what tells the caller
    /// whether the outage is a new one.
    ///
    /// `first` carries the first session's outcome to whoever is waiting to hear whether the
    /// connection came up, and it is fired as soon as there is an answer either way rather than when
    /// the session ends: a caller waiting for a connection must not be made to wait for its loss.
    async fn session(
        &mut self,
        stream: TcpStream,
        first: Option<oneshot::Sender<Result<(), String>>>,
    ) -> Result<Handshake, ConnectionError> {
        let (mut read_half, mut write_half) = stream.into_split();
        let mut buffer: Vec<u8> = Vec::with_capacity(16 * 1024);

        let outcome = self
            .handshake(&mut read_half, &mut write_half, &mut buffer)
            .await;

        match outcome {
            Ok(reported) => {
                if let Some(sender) = first {
                    let _ = sender.send(Ok(()));
                }

                let _ = self
                    .shared
                    .session
                    .send(SessionState::Ready(reported.clone()));

                let ending = self
                    .steady(
                        &mut read_half,
                        &mut write_half,
                        &mut buffer,
                        reported.keep_alive,
                    )
                    .await;

                let _ = write_half.shutdown().await;
                // Releasing the waiters is what turns a dead socket into an immediate error for
                // every outstanding request, instead of each one waiting out its own timeout.
                lock(&self.shared.pending).clear();
                log::debug!("{ending}");

                Ok(reported)
            }
            Err(e) => {
                if let Some(sender) = first {
                    let _ = sender.send(Err(e.to_string()));
                }

                log::warn!("the session could not be established: {e}");
                let _ = write_half.shutdown().await;
                lock(&self.shared.pending).clear();

                Err(e)
            }
        }
    }

    /// Performs the handshake, driving the read and write paths while it is outstanding.
    ///
    /// Nothing may be written before the handshake, so the write path is here for one frame: the
    /// handshake request itself. The read path has to run all the same, because the answer is a
    /// response like any other and only this loop can deliver it.
    async fn handshake(
        &mut self,
        read_half: &mut OwnedReadHalf,
        write_half: &mut OwnedWriteHalf,
        buffer: &mut Vec<u8>,
    ) -> Result<Handshake, ConnectionError> {
        let handshake = self
            .shared
            .requester
            .handshake(&self.shared.options.client_id);
        tokio::pin!(handshake);

        loop {
            tokio::select! {
                result = &mut handshake => return result,

                frame = self.frames.recv() => {
                    let Some(frame) = frame else {
                        return Err(ConnectionError::Closed);
                    };

                    if write_half.write_all(&frame).await.is_err() {
                        return Err(ConnectionError::Closed);
                    }
                }

                read = read_half.read_buf(buffer) => {
                    match read {
                        Ok(0) => {
                            // The answer can arrive in the same read that ends the socket, and it is
                            // this path that delivered it, so the handshake is polled once more
                            // before the session is reported as never having been established. A
                            // gateway that answers and then closes immediately is a gateway that
                            // spoke the protocol, which is what the caller is waiting to hear.
                            return match timeout(Duration::ZERO, &mut handshake).await {
                                Ok(result) => result,
                                Err(_) => Err(ConnectionError::Closed),
                            };
                        }
                        Ok(_) => {}
                        Err(e) => return Err(ConnectionError::Io(e)),
                    }

                    drain_frames(buffer, &self.shared.pending, &self.shared.pushes)?;
                }

                _ = self.stopped.changed() => return Err(ConnectionError::Closed),
            }
        }
    }

    /// Runs an established session until one of its three paths ends it, returning why.
    async fn steady(
        &mut self,
        read_half: &mut OwnedReadHalf,
        write_half: &mut OwnedWriteHalf,
        buffer: &mut Vec<u8>,
        keep_alive: Duration,
    ) -> &'static str {
        let mut ticker = interval(keep_alive);
        ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
        // The first tick of an interval completes immediately, and the handshake has just happened.
        ticker.tick().await;

        loop {
            tokio::select! {
                frame = self.frames.recv() => {
                    let Some(frame) = frame else {
                        return "the request channel closed";
                    };

                    if write_half.write_all(&frame).await.is_err() {
                        return "the gateway stopped accepting writes";
                    }
                }

                read = read_half.read_buf(buffer) => {
                    match read {
                        Ok(0) => return "the gateway closed the socket",
                        Ok(_) => {}
                        Err(_) => return "the socket failed while reading",
                    }

                    if let Err(e) = drain_frames(buffer, &self.shared.pending, &self.shared.pushes) {
                        log::error!("the frame boundary is lost: {e}");
                        return "a frame lost its boundary";
                    }
                }

                _ = ticker.tick() => {
                    let request = keep_alive::Request {
                        c2s: keep_alive::C2s { time: unix_seconds() },
                    };
                    let body = prost::Message::encode_to_vec(&request);
                    let requester = self.shared.requester.clone();

                    // The answer arrives on the read path like any other response, so this must not
                    // be awaited here: waiting for it would stop the read path that delivers it.
                    tokio::spawn(async move {
                        if let Err(e) = requester.request(PROTO_ID_KEEP_ALIVE, &body).await {
                            log::debug!("the keep-alive went unanswered: {e}");
                        }
                    });
                }

                _ = self.stopped.changed() => return "the connection was asked to stop",
            }
        }
    }
}

/// Decodes and dispatches every whole frame in `buffer`, leaving a partial frame in place.
///
/// A corrupt frame is skipped using the extent its header states, because that extent is what makes
/// skipping possible at all. A frame whose boundary cannot be found is the one failure a caller
/// cannot recover from, since every byte after it is unreadable.
fn drain_frames(
    buffer: &mut Vec<u8>,
    pending: &Pending,
    pushes: &mpsc::UnboundedSender<Message>,
) -> Result<(), CodecError> {
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
                dispatch(message, pending, pushes);
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
                if consumed > 0 {
                    buffer.drain(..consumed);
                }

                return Err(e);
            }
        }
    }

    if consumed > 0 {
        buffer.drain(..consumed);
    }

    Ok(())
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

#[cfg(test)]
mod tests {
    use std::future::Future;

    use rstest::rstest;
    use tokio::{
        net::TcpListener,
        net::tcp::{OwnedReadHalf, OwnedWriteHalf},
    };

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

            send_message(
                &mut write,
                PROTO_ID_INIT_CONNECT,
                request.serial_no,
                &prost::Message::encode_to_vec(&handshake_response(CONN_ID)),
            )
            .await;

            script(write, read).await;
        });

        port
    }

    /// The handshake answer a controlled gateway sends, reporting `conn_id` for the session.
    fn handshake_response(conn_id: u64) -> init_connect::Response {
        init_connect::Response {
            ret_type: RET_OK,
            ret_msg: None,
            err_code: None,
            s2c: Some(init_connect::S2c {
                server_ver: SERVER_VER,
                login_user_id: 7,
                conn_id,
                conn_aes_key: "0123456789abcdef".to_string(),
                keep_alive_interval: REPORTED_KEEP_ALIVE_SECS,
                aes_cb_civ: None,
                user_attribution: None,
            }),
        }
    }

    /// Starts a controlled gateway that accepts `sessions` connections, handshaking each and
    /// reporting a distinct connection identifier for it.
    ///
    /// The first session is dropped as soon as it is up, which is the outage under test, and each
    /// later one answers a quote request, which is what proves a replacement carries traffic rather
    /// than merely existing.
    async fn spawn_reconnecting_gateway(sessions: u64) -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();

        tokio::spawn(async move {
            for index in 0..sessions {
                let (stream, _) = listener.accept().await.unwrap();
                let (mut read, mut write) = stream.into_split();
                let mut buffer = Vec::new();

                let request = recv_message(&mut read, &mut buffer).await;
                assert_eq!(request.proto_id, PROTO_ID_INIT_CONNECT);

                send_message(
                    &mut write,
                    PROTO_ID_INIT_CONNECT,
                    request.serial_no,
                    &prost::Message::encode_to_vec(&handshake_response(CONN_ID + index)),
                )
                .await;

                if index == 0 {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    let _ = write.shutdown().await;
                    continue;
                }

                loop {
                    let message = recv_message(&mut read, &mut buffer).await;

                    if message.proto_id == 3202 {
                        send_message(&mut write, message.proto_id, message.serial_no, b"restored")
                            .await;
                        break;
                    }
                }
            }
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
        // The session is held open, because the handshake belongs to the session that is current:
        // a gateway that has already gone leaves none to report. The halves have to be captured by
        // the future, or they would drop as soon as the script returns them.
        let port = spawn_gateway(|write, read| async move {
            let _held = (write, read);
            tokio::time::sleep(Duration::from_secs(30)).await;
        })
        .await;
        let (connection, _pushes) = connect_to(port).await;

        let handshake = connection.handshake().unwrap_or_else(|| {
            panic!(
                "expected the first session to be established, found {:?}",
                *connection.state().borrow(),
            )
        });

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

    /// A dropped socket releases outstanding requests immediately rather than leaving them to their
    /// timeout, and it is reported as an outage rather than as the end of the connection.
    #[tokio::test]
    async fn test_a_dropped_socket_releases_the_waiters_and_reports_the_outage() {
        let port = spawn_gateway(|mut write, _read| async move {
            // Accept the request, then drop the write half without answering.
            tokio::time::sleep(Duration::from_millis(50)).await;
            let _ = write.shutdown().await;
        })
        .await;

        let options = ConnectOptions::new("127.0.0.1", port)
            .with_client_id(CLIENT_ID)
            // The gateway's listener goes with the first connection, so a replacement cannot be
            // made and the supervisor is told to stop rather than retry behind the test.
            .with_reconnect_attempts(1);
        let (pushes, _push_rx) = mpsc::unbounded_channel();
        let connection = connect(&options, pushes).await.unwrap();

        let mut state = connection.state();
        let mut closed = connection.closed();

        let result = connection.request(3202, b"query").await;
        assert!(
            matches!(result, Err(ConnectionError::Closed)),
            "expected the request to be released, got {result:?}"
        );

        tokio::time::timeout(Duration::from_secs(5), state.changed())
            .await
            .expect("the outage should be observable")
            .expect("the sender should not be dropped");
        assert_eq!(*state.borrow(), SessionState::Restoring { attempts: 1 });

        tokio::time::timeout(Duration::from_secs(5), closed.changed())
            .await
            .expect("the supervisor should stop once its attempts run out")
            .expect("the sender should not be dropped");
        assert!(*closed.borrow());
    }

    /// The socket is replaced, handshaken again, and carries traffic, which is what makes a drop an
    /// outage rather than the end.
    #[tokio::test]
    async fn test_a_dropped_socket_is_replaced_and_rehandshaken() {
        let port = spawn_reconnecting_gateway(2).await;

        let options = ConnectOptions::new("127.0.0.1", port)
            .with_client_id(CLIENT_ID)
            .with_reconnect_attempts(3);
        let (pushes, _push_rx) = mpsc::unbounded_channel();
        let connection = connect(&options, pushes).await.unwrap();

        let first = connection
            .handshake()
            .expect("the first session is established");
        assert_eq!(first.conn_id, CONN_ID);

        let mut state = connection.state();

        let replaced = tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                if let SessionState::Ready(handshake) = state.borrow().clone()
                    && handshake.conn_id != first.conn_id
                {
                    return handshake;
                }

                state.changed().await.unwrap();
            }
        })
        .await
        .expect("the session should be replaced");

        assert_eq!(
            replaced.conn_id,
            CONN_ID + 1,
            "the replacement is a new session, not the old one reported again",
        );
        assert_eq!(
            connection.handshake().expect("there is a session").conn_id,
            CONN_ID + 1,
        );

        let response = connection.request(3202, b"query").await.unwrap();
        assert_eq!(
            response.body, b"restored",
            "the replacement carries requests, not just a handshake",
        );
    }

    /// Being asked to stop is not an outage, and it is the only thing besides running out of
    /// attempts that ends the supervisor.
    #[tokio::test]
    async fn test_shutdown_stops_the_supervisor() {
        let port = spawn_gateway(|write, read| async move {
            // Hold the session open, so that only the shutdown can end it. The halves have to be
            // captured by the future, or they would drop as soon as the script returns them.
            let _held = (write, read);
            tokio::time::sleep(Duration::from_secs(30)).await;
        })
        .await;

        let (connection, _pushes) = connect_to(port).await;
        let mut closed = connection.closed();

        assert!(
            !*closed.borrow(),
            "a live session is not a closed connection"
        );
        connection.shutdown();

        tokio::time::timeout(Duration::from_secs(5), closed.changed())
            .await
            .expect("the closure should be observable")
            .expect("the sender should not be dropped");
        assert!(*closed.borrow());
    }

    /// The wait between attempts grows and then stops growing, so a gateway down for an hour is
    /// asked roughly a hundred times rather than thousands.
    #[rstest]
    #[case(0, 1)]
    #[case(1, 2)]
    #[case(2, 4)]
    #[case(4, 16)]
    #[case(5, 30)]
    #[case(9, 30)]
    #[case(u32::MAX, 30)]
    fn test_the_reconnect_wait_grows_and_is_capped(#[case] attempts: u32, #[case] seconds: u64) {
        assert_eq!(backoff(attempts), Duration::from_secs(seconds));
    }
}
