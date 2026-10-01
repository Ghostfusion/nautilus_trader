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

//! Notification sinks, their bounded queues, and burst coalescing.

use std::{
    fmt::Write as _,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::{RecvTimeoutError, SyncSender, sync_channel},
    },
    thread,
    time::{Duration, Instant},
};

use ustr::Ustr;

use super::event::{NotificationEvent, NotificationSeverity};

/// The result type returned by notification sinks and transports.
pub type NotificationResult<T> = anyhow::Result<T>;

/// A titled notification message with a severity.
///
/// This is the minimal payload a [`NotificationSink`] delivers; transports render it
/// into their own wire format.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotificationMessage {
    /// The message title.
    pub title: String,
    /// The message body.
    pub body: String,
    /// The message severity.
    pub severity: NotificationSeverity,
}

impl NotificationMessage {
    /// Creates a new [`NotificationMessage`].
    #[must_use]
    pub fn new(
        title: impl Into<String>,
        body: impl Into<String>,
        severity: NotificationSeverity,
    ) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
            severity,
        }
    }

    /// Renders a notification event into a message, appending any identities to the body.
    #[must_use]
    pub fn from_event(event: &NotificationEvent) -> Self {
        let title = format!("{}: {}", event.severity, event.class);
        let mut body = event.message.clone();

        if let Some(strategy_id) = event.strategy_id {
            let _ = write!(body, "\nstrategy_id={strategy_id}");
        }
        if let Some(instrument_id) = event.instrument_id {
            let _ = write!(body, "\ninstrument_id={instrument_id}");
        }
        if let Some(account_id) = event.account_id {
            let _ = write!(body, "\naccount_id={account_id}");
        }
        if let Some(run_id) = event.run_id {
            let _ = write!(body, "\nrun_id={run_id}");
        }

        Self {
            title,
            body,
            severity: event.severity,
        }
    }
}

/// A destination for titled notification messages.
///
/// Implementations perform their own I/O and are invoked on the sink's worker thread,
/// never on the publisher thread. A sink must not call back into the router.
pub trait NotificationSink: Send {
    /// Sends a notification message.
    ///
    /// # Errors
    ///
    /// Returns an error if the sink cannot deliver the message. The error is reported
    /// through the ordinary logging path and does not propagate to the publisher.
    fn send(&mut self, message: &NotificationMessage) -> NotificationResult<()>;
}

/// Configuration for a sink's bounded queue and coalescing behaviour.
#[derive(Clone, Copy, Debug)]
pub struct NotificationSinkConfig {
    /// The maximum number of messages held in the queue before the publisher drops new ones.
    ///
    /// A value of zero is treated as one so the queue never degenerates into a rendezvous
    /// channel that drops every message.
    pub queue_bound: usize,
    /// The interval over which messages delivered to the sink are coalesced into a single send.
    ///
    /// A zero interval disables coalescing, so every message is sent individually.
    pub coalesce_interval: Duration,
}

impl Default for NotificationSinkConfig {
    fn default() -> Self {
        Self {
            queue_bound: 1024,
            coalesce_interval: Duration::from_millis(250),
        }
    }
}

impl NotificationSinkConfig {
    /// Creates a new [`NotificationSinkConfig`].
    #[must_use]
    pub fn new(queue_bound: usize, coalesce_interval: Duration) -> Self {
        Self {
            queue_bound,
            coalesce_interval,
        }
    }
}

/// The publisher-side handle to a sink's bounded queue and its counters.
pub(crate) struct SinkChannel {
    pub sender: SyncSender<NotificationMessage>,
    pub dropped: Arc<AtomicU64>,
    pub sent: Arc<AtomicU64>,
}

/// Spawns a worker thread that drains `sink`'s bounded queue and delivers messages.
///
/// The returned [`SinkChannel`] is the non-blocking publisher endpoint. When the queue
/// is full the message is dropped and `dropped` is incremented; the caller is never blocked.
pub(crate) fn spawn_sink_worker(
    name: Ustr,
    config: NotificationSinkConfig,
    sink: Box<dyn NotificationSink>,
) -> SinkChannel {
    let bound = config.queue_bound.max(1);
    let (sender, receiver) = sync_channel::<NotificationMessage>(bound);
    let dropped = Arc::new(AtomicU64::new(0));
    let sent = Arc::new(AtomicU64::new(0));

    let worker_sent = Arc::clone(&sent);
    let worker_name = format!("nt-notify-{name}");
    let coalesce_interval = config.coalesce_interval;

    let result = thread::Builder::new().name(worker_name).spawn(move || {
        run_sink_worker(&receiver, coalesce_interval, sink, &worker_sent);
    });

    if let Err(e) = result {
        crate::log_error!("Failed to spawn notification sink '{name}' worker: {}", e);
    }

    SinkChannel {
        sender,
        dropped,
        sent,
    }
}

fn run_sink_worker(
    receiver: &std::sync::mpsc::Receiver<NotificationMessage>,
    coalesce_interval: Duration,
    mut sink: Box<dyn NotificationSink>,
    sent: &AtomicU64,
) {
    while let Ok(first) = receiver.recv() {
        let mut batch = vec![first];

        if !coalesce_interval.is_zero() {
            let deadline = Instant::now() + coalesce_interval;

            loop {
                let now = Instant::now();
                if now >= deadline {
                    break;
                }

                match receiver.recv_timeout(deadline - now) {
                    Ok(message) => batch.push(message),
                    Err(RecvTimeoutError::Timeout) => break,
                    Err(RecvTimeoutError::Disconnected) => {
                        deliver(&mut *sink, batch, sent);
                        return;
                    }
                }
            }
        }

        deliver(&mut *sink, batch, sent);
    }
}

fn deliver(sink: &mut dyn NotificationSink, batch: Vec<NotificationMessage>, sent: &AtomicU64) {
    let message = coalesce(batch);

    if let Err(e) = sink.send(&message) {
        crate::log_error!("Notification sink delivery failed: {}", e);
    } else {
        sent.fetch_add(1, Ordering::Relaxed);
    }
}

fn coalesce(mut batch: Vec<NotificationMessage>) -> NotificationMessage {
    if batch.len() == 1 {
        return batch
            .pop()
            .expect("batch is non-empty when its length is one");
    }

    let severity = batch
        .iter()
        .map(|message| message.severity)
        .max()
        .unwrap_or(NotificationSeverity::Info);
    let title = format!("{} ({} notifications)", batch[0].title, batch.len());
    let body = batch
        .iter()
        .map(|message| message.body.as_str())
        .collect::<Vec<_>>()
        .join("\n---\n");

    NotificationMessage {
        title,
        body,
        severity,
    }
}
