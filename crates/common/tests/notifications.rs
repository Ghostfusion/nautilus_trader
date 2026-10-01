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

//! Integration tests for the notification subsystem.
//!
//! The sink tests drive each default transport against a local `TcpListener` stub. The
//! router tests use a recording or gate-blocking sink to bound the send rate under a
//! burst, to prove the bounded queue drops and counts rather than blocks, and to prove a
//! publish with no sink configured is a no-op. Timing assertions wait on observable
//! counters with generous deadlines rather than relying on a fixed sleep.

use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpListener,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant},
};

use nautilus_common::{
    msgbus::{self, MStr, Topic},
    notification::{
        EmailSink, HttpTransport, NOTIFICATION_TOPIC, NotificationClass, NotificationEvent,
        NotificationMessage, NotificationResult, NotificationRouter, NotificationSeverity,
        NotificationSink, NotificationSinkConfig, SmtpTransport, WebhookSink,
    },
};
use nautilus_model::identifiers::InstrumentId;

#[test]
fn email_sink_delivers_over_smtp() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = mpsc::channel::<String>();

    let server = thread::spawn(move || run_smtp_stub(&listener, &tx));

    let transport = SmtpTransport::new(
        "127.0.0.1",
        port,
        "alerts@example.com",
        vec!["ops@example.com".to_string()],
    );
    let mut sink = EmailSink::new(transport);
    let message = NotificationMessage::new(
        "Risk halt",
        "Risk engine halted trading",
        NotificationSeverity::Critical,
    );

    sink.send(&message).unwrap();
    server.join().unwrap();

    let transcript = rx.try_iter().collect::<String>();
    assert!(
        transcript.contains("MAIL FROM:<alerts@example.com>"),
        "{transcript}"
    );
    assert!(
        transcript.contains("RCPT TO:<ops@example.com>"),
        "{transcript}"
    );
    assert!(transcript.contains("Subject: Risk halt"), "{transcript}");
    assert!(
        transcript.contains("Risk engine halted trading"),
        "{transcript}"
    );
}

#[test]
fn webhook_sink_posts_json_over_http() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let (tx, rx) = mpsc::channel::<String>();

    let server = thread::spawn(move || run_http_stub(&listener, &tx));

    let url = format!("http://127.0.0.1:{port}/hooks/alerts");
    let transport = HttpTransport::new(&url).unwrap();
    let mut sink = WebhookSink::new(transport);
    let message = NotificationMessage::new(
        "Drawdown",
        "Drawdown threshold breached",
        NotificationSeverity::Warning,
    );

    sink.send(&message).unwrap();
    server.join().unwrap();

    let request = rx.recv().unwrap();
    assert!(
        request.starts_with("POST /hooks/alerts HTTP/1.1"),
        "{request}"
    );
    assert!(request.contains("\"title\":\"Drawdown\""), "{request}");
    assert!(request.contains("\"severity\":\"WARNING\""), "{request}");
    assert!(request.contains("Drawdown threshold breached"), "{request}");
}

#[test]
fn router_coalesces_burst_into_single_send() {
    let sends = Arc::new(AtomicU64::new(0));
    let router = Rc::new(NotificationRouter::new());

    router.register_sink(
        "recording",
        RecordingSink {
            sends: Arc::clone(&sends),
        },
        [NotificationClass::OrderRejection],
        NotificationSinkConfig::new(4096, Duration::from_millis(500)),
    );

    for _ in 0..100 {
        router.publish(&NotificationEvent::new(
            NotificationClass::OrderRejection,
            NotificationSeverity::Warning,
            "order rejected",
        ));
    }

    let deadline = Instant::now() + Duration::from_secs(5);
    while router.sent_count("recording") == 0 && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(10));
    }

    // Allow a further coalescing window to elapse so any stray second batch would appear.
    thread::sleep(Duration::from_millis(700));

    assert_eq!(
        sends.load(Ordering::Relaxed),
        1,
        "a burst within one interval must produce a single send"
    );
    assert_eq!(router.sent_count("recording"), 1);
    assert_eq!(router.drop_count("recording"), 0);
}

#[test]
fn router_queue_drops_and_counts_under_saturation() {
    let (release, gate) = mpsc::channel::<()>();
    let router = Rc::new(NotificationRouter::new());

    router.register_sink(
        "blocked",
        BlockingSink { gate },
        [NotificationClass::StrategyStop],
        NotificationSinkConfig::new(1, Duration::from_millis(0)),
    );

    let start = Instant::now();
    for _ in 0..500 {
        router.publish(&NotificationEvent::new(
            NotificationClass::StrategyStop,
            NotificationSeverity::Info,
            "strategy stopped",
        ));
    }
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_secs(1),
        "publishing must not block, took {elapsed:?}"
    );

    let drops = router.drop_count("blocked");
    assert!(
        drops >= 400,
        "expected the bounded queue to drop, was {drops}"
    );
    assert_eq!(router.sent_count("blocked"), 0);

    // Release the blocked worker so it can drain and exit cleanly.
    drop(release);

    let deadline = Instant::now() + Duration::from_secs(5);
    while router.sent_count("blocked") == 0 && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(10));
    }
    assert!(router.sent_count("blocked") >= 1);
}

#[test]
fn publishing_without_sinks_is_a_no_op() {
    let router = Rc::new(NotificationRouter::new());
    router.subscribe();

    let event = NotificationEvent::new(
        NotificationClass::DataFeedDisconnect,
        NotificationSeverity::Warning,
        "data feed disconnected",
    )
    .with_instrument_id(InstrumentId::from("EURUSD.SIM"));

    // A subsystem publishes to the bus; the router translates and finds no sink.
    msgbus::publish_any(MStr::<Topic>::from(NOTIFICATION_TOPIC), &event);
    // A direct publish must equally be a no-op.
    router.publish(&event);

    assert_eq!(router.drop_count("none"), 0);
    assert_eq!(router.sent_count("none"), 0);
}

struct RecordingSink {
    sends: Arc<AtomicU64>,
}

impl NotificationSink for RecordingSink {
    fn send(&mut self, _message: &NotificationMessage) -> NotificationResult<()> {
        self.sends.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

struct BlockingSink {
    gate: mpsc::Receiver<()>,
}

impl NotificationSink for BlockingSink {
    fn send(&mut self, _message: &NotificationMessage) -> NotificationResult<()> {
        // Blocks until the test releases the gate by dropping its sender, after which
        // every subsequent receive returns immediately.
        let _ = self.gate.recv();
        Ok(())
    }
}

fn run_smtp_stub(listener: &TcpListener, tx: &mpsc::Sender<String>) {
    let (stream, _) = listener.accept().unwrap();
    let mut writer = stream.try_clone().unwrap();
    writer.write_all(b"220 stub ESMTP ready\r\n").unwrap();

    let mut reader = BufReader::new(stream);
    let mut in_data = false;

    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap() == 0 {
            break;
        }
        let _ = tx.send(line.clone());

        let trimmed = line.trim_end_matches(['\r', '\n']);

        if in_data {
            if trimmed == "." {
                in_data = false;
                writer.write_all(b"250 OK message accepted\r\n").unwrap();
            }
            continue;
        }

        let upper = trimmed.to_uppercase();

        if upper.starts_with("EHLO") {
            writer.write_all(b"250-stub\r\n250 OK\r\n").unwrap();
        } else if upper.starts_with("MAIL FROM") || upper.starts_with("RCPT TO") {
            writer.write_all(b"250 OK\r\n").unwrap();
        } else if upper == "DATA" {
            writer
                .write_all(b"354 End data with <CR><LF>.<CR><LF>\r\n")
                .unwrap();
            in_data = true;
        } else if upper.starts_with("QUIT") {
            writer.write_all(b"221 Bye\r\n").unwrap();
            break;
        } else {
            writer.write_all(b"250 OK\r\n").unwrap();
        }
    }
}

fn run_http_stub(listener: &TcpListener, tx: &mpsc::Sender<String>) {
    let (mut stream, _) = listener.accept().unwrap();
    let mut reader = BufReader::new(stream.try_clone().unwrap());

    let mut head = String::new();
    let mut content_length = 0usize;

    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap() == 0 {
            break;
        }
        if let Some(value) = line.strip_prefix("Content-Length:") {
            content_length = value.trim().parse().unwrap();
        }
        head.push_str(&line);
        if line == "\r\n" {
            break;
        }
    }

    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body).unwrap();
    head.push_str(&String::from_utf8_lossy(&body));

    let _ = tx.send(head);

    stream
        .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
        .unwrap();
    stream.flush().unwrap();
}
