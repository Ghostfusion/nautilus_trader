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

//! A client for one EODHD streaming channel.
//!
//! The task spawned by [`EodhdWebSocketClient::connect`] owns the underlying
//! [`WebSocketClient`], which reconnects on its own. Because the task owns it, the task can also
//! answer the protocol pings the server sends. A reconnect arrives as a new connection epoch, and
//! the task replays the symbols it has been asked to stream on the replacement connection.
//!
//! No adapter message carries a subscription acknowledgment: the server answers a subscribe
//! request with a control frame carrying a status code, which is forwarded as an
//! [`EodhdWsMessage::Status`] for the data client to log.

use std::{collections::BTreeSet, fmt::Debug, sync::Arc};

use nautilus_common::live::get_runtime;
use nautilus_core::string::secret::SecretString;
use nautilus_network::{
    http::create_standard_nautilus_headers,
    websocket::{
        PingHandler, TransportBackend, WebSocketClient, WebSocketConfig,
        channel_epoch_message_handler,
    },
};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender}; // tokio-import-ok
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;

use super::messages::{EodhdWsMessage, parse_message};

/// The longest wait for the initial connection, in milliseconds.
const CONNECT_TIMEOUT_MS: u64 = 5_000;

/// A command handled by a streaming task.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StreamCommand {
    /// Subscribe to `symbols`, which carry no exchange suffix.
    Subscribe(Vec<String>),
    /// Unsubscribe from `symbols`.
    Unsubscribe(Vec<String>),
}

/// A client for one EODHD streaming channel.
///
/// The handle sends subscription commands to the channel's task and does not read frames; the
/// parsed frames arrive on the receiver returned by [`EodhdWebSocketClient::connect`]. Dropping
/// the handle stops the task.
pub struct EodhdWebSocketClient {
    channel: String,
    url: SecretString,
    cmd_tx: UnboundedSender<StreamCommand>,
    token: CancellationToken,
}

impl Debug for EodhdWebSocketClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(EodhdWebSocketClient))
            .field("channel", &self.channel)
            .field("url", &self.url)
            .finish()
    }
}

impl Drop for EodhdWebSocketClient {
    fn drop(&mut self) {
        self.token.cancel();
    }
}

impl EodhdWebSocketClient {
    /// Connects `channel` under `base_url` and returns the client with its parsed frames.
    ///
    /// The API token travels as the `api_token` query parameter, as it does on the REST API.
    ///
    /// # Errors
    ///
    /// Returns an error if the connection cannot be established, or if the streaming task stops
    /// before it connects. A connection the server refuses is established at the transport level
    /// and reported as an [`EodhdWsMessage::Status`] frame instead.
    pub async fn connect(
        base_url: &str,
        channel: &str,
        api_key: &str,
        proxy_url: Option<String>,
    ) -> anyhow::Result<(Self, UnboundedReceiver<EodhdWsMessage>)> {
        let url = format!("{base_url}/{channel}?api_token={api_key}");
        let (cmd_tx, cmd_rx) = tokio::sync::mpsc::unbounded_channel();
        let (out_tx, out_rx) = tokio::sync::mpsc::unbounded_channel();
        let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
        let token = CancellationToken::new();

        let channel_name = channel.to_string();
        let task_token = token.clone();

        get_runtime().spawn(run_channel(
            url.clone(),
            channel_name,
            proxy_url,
            cmd_rx,
            out_tx,
            task_token,
            ready_tx,
        ));

        let outcome = ready_rx.await.map_err(|_| {
            anyhow::anyhow!("The EODHD {channel} streaming task stopped before it connected")
        })?;
        outcome?;

        Ok((
            Self {
                channel: channel.to_string(),
                url: SecretString::from(url),
                cmd_tx,
                token,
            },
            out_rx,
        ))
    }

    /// Returns the channel this client streams.
    #[must_use]
    pub fn channel(&self) -> &str {
        &self.channel
    }

    /// Sends a subscription command to the channel's task.
    ///
    /// # Errors
    ///
    /// Returns an error if the streaming task has stopped.
    pub fn send(&self, command: StreamCommand) -> anyhow::Result<()> {
        self.cmd_tx
            .send(command)
            .map_err(|_| anyhow::anyhow!("The EODHD {} stream has stopped", self.channel))
    }
}

/// Runs one streaming channel until the token is cancelled or the transport closes.
async fn run_channel(
    url: String,
    channel: String,
    proxy_url: Option<String>,
    mut cmd_rx: UnboundedReceiver<StreamCommand>,
    out_tx: UnboundedSender<EodhdWsMessage>,
    token: CancellationToken,
    ready_tx: tokio::sync::oneshot::Sender<anyhow::Result<()>>,
) {
    let (epoch_handler, mut raw_rx) = channel_epoch_message_handler();

    // No-op: the WebSocketClient answers protocol pings, and this task answers any that reach it.
    let ping_handler: PingHandler = Arc::new(|_payload: Vec<u8>| {});

    let config = WebSocketConfig {
        url: url.clone(),
        headers: create_standard_nautilus_headers(),
        heartbeat_interval_secs: None,
        heartbeat_payload: None,
        connect_timeout_ms: Some(CONNECT_TIMEOUT_MS),
        reconnect_delay_initial_ms: Some(500),
        reconnect_delay_max_ms: Some(5_000),
        reconnect_backoff_factor: Some(1.5),
        reconnect_jitter_ms: Some(250),
        reconnect_max_attempts: None,
        heartbeat_timeout_secs: None,
        idle_timeout_ms: None,
        writer_capacity: None,
        backend: TransportBackend::default(),
        proxy_url,
        max_message_size_bytes: None,
        max_frame_size_bytes: None,
    };

    let connect = WebSocketClient::epoch_builder()
        .config(config)
        .epoch_handler(epoch_handler)
        .ping_handler(ping_handler)
        .cancellation_token(token.clone())
        .connect()
        .await;

    let client = match connect {
        Ok(client) => client,
        Err(e) => {
            let _ = ready_tx.send(Err(anyhow::anyhow!(
                "Failed to connect to the EODHD {channel} stream: {e}"
            )));

            return;
        }
    };

    log::debug!("Connected to the EODHD {channel} stream");

    if ready_tx.send(Ok(())).is_err() {
        return;
    }

    let mut subscriptions: BTreeSet<String> = BTreeSet::new();
    // The initial connection carries epoch 0, so only a replacement connection replays.
    let mut epoch: Option<u64> = Some(0);

    loop {
        tokio::select! {
            biased;
            () = token.cancelled() => break,
            Some(command) = cmd_rx.recv() => {
                match command {
                    StreamCommand::Subscribe(symbols) => {
                        subscriptions.extend(symbols.iter().cloned());
                        send_subscription(&client, "subscribe", &symbols).await;
                    }
                    StreamCommand::Unsubscribe(symbols) => {
                        for symbol in &symbols {
                            subscriptions.remove(symbol);
                        }

                        send_subscription(&client, "unsubscribe", &symbols).await;
                    }
                }
            }
            Some((frame_epoch, message)) = raw_rx.recv() => {
                if epoch != Some(frame_epoch) {
                    epoch = Some(frame_epoch);

                    if subscriptions.is_empty() {
                        log::debug!("The EODHD {channel} stream reconnected with no subscriptions");
                    } else {
                        let symbols: Vec<String> = subscriptions.iter().cloned().collect();

                        log::debug!(
                            "Replaying {} EODHD {channel} subscriptions after a reconnect",
                            symbols.len()
                        );
                        send_subscription(&client, "subscribe", &symbols).await;
                    }
                }

                if let Some(message) = handle_frame(&client, &channel, message).await
                    && out_tx.send(message).is_err()
                {
                    break;
                }
            }
            else => break,
        }
    }

    log::debug!("The EODHD {channel} stream has stopped");
}

/// Returns the message a frame carries, answering a protocol ping as a side effect.
async fn handle_frame(
    client: &WebSocketClient,
    channel: &str,
    message: Message,
) -> Option<EodhdWsMessage> {
    match message {
        Message::Text(text) => match parse_message(&text) {
            Ok(message) => Some(message),
            Err(e) => {
                log::warn!("Failed to parse an EODHD {channel} frame: {e}");

                None
            }
        },
        Message::Ping(data) => {
            if let Err(e) = client.send_pong(data.to_vec()).await {
                log::warn!("Failed to answer an EODHD {channel} ping: {e}");
            }

            None
        }
        Message::Close(frame) => {
            log::debug!("The EODHD {channel} stream closed: {frame:?}");

            None
        }
        Message::Binary(_) | Message::Pong(_) | Message::Frame(_) => None,
    }
}

/// Sends a subscription command for `symbols`.
///
/// The channel takes `symbols` as one comma separated string rather than as an array.
async fn send_subscription(client: &WebSocketClient, action: &str, symbols: &[String]) {
    if symbols.is_empty() {
        return;
    }

    let message = serde_json::json!({ "action": action, "symbols": symbols.join(",") });
    log::debug!(
        "Sending the EODHD '{action}' command for {}",
        symbols.join(",")
    );

    if let Err(e) = client.send_text(message.to_string(), None).await {
        log::error!("Failed to send the EODHD '{action}' command: {e}");
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    fn test_debug_redacts_the_api_token() {
        let (cmd_tx, _cmd_rx) = tokio::sync::mpsc::unbounded_channel();
        let client = EodhdWebSocketClient {
            channel: "us".to_string(),
            url: SecretString::from("wss://example/ws/us?api_token=super-secret-token"),
            cmd_tx,
            token: CancellationToken::new(),
        };

        let debug = format!("{client:?}");

        assert!(!debug.contains("super-secret-token"), "{debug}");
        assert!(debug.contains("us"), "{debug}");
    }

    #[rstest]
    fn test_send_reports_a_stopped_stream() {
        let (cmd_tx, cmd_rx) = tokio::sync::mpsc::unbounded_channel();
        drop(cmd_rx);

        let client = EodhdWebSocketClient {
            channel: "us".to_string(),
            url: SecretString::from("wss://example/ws/us"),
            cmd_tx,
            token: CancellationToken::new(),
        };

        assert!(
            client
                .send(StreamCommand::Subscribe(vec!["AAPL".to_string()]))
                .is_err()
        );
    }

    #[rstest]
    fn test_send_delivers_the_command() {
        let (cmd_tx, mut cmd_rx) = tokio::sync::mpsc::unbounded_channel();
        let client = EodhdWebSocketClient {
            channel: "us".to_string(),
            url: SecretString::from("wss://example/ws/us"),
            cmd_tx,
            token: CancellationToken::new(),
        };

        client
            .send(StreamCommand::Subscribe(vec!["AAPL".to_string()]))
            .unwrap();

        assert_eq!(
            cmd_rx.try_recv().unwrap(),
            StreamCommand::Subscribe(vec!["AAPL".to_string()])
        );
    }

    #[rstest]
    fn test_drop_cancels_the_streaming_task() {
        let (cmd_tx, _cmd_rx) = tokio::sync::mpsc::unbounded_channel();
        let token = CancellationToken::new();
        let client = EodhdWebSocketClient {
            channel: "us".to_string(),
            url: SecretString::from("wss://example/ws/us"),
            cmd_tx,
            token: token.clone(),
        };

        drop(client);

        assert!(token.is_cancelled());
    }
}
