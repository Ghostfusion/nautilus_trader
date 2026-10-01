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

//! The HTTP webhook sink and its plaintext transport.
//!
//! [`HttpTransport`] issues a minimal HTTP/1.1 `POST` with a JSON body over a blocking
//! `std::net::TcpStream`. It uses the `http` scheme only and does **not** negotiate
//! TLS; an `https` URL is rejected. Use it against a trusted endpoint or tunnel only.

use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpStream,
    time::Duration,
};

use anyhow::{Context, anyhow, bail};

use super::{
    sink::{NotificationMessage, NotificationSink},
    transport::NotificationTransport,
};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_HTTP_PORT: u16 = 80;

/// A plaintext HTTP webhook transport.
#[derive(Clone, Debug)]
pub struct HttpTransport {
    host: String,
    port: u16,
    path: String,
    timeout: Duration,
}

impl HttpTransport {
    /// Parses a plaintext `http://` URL into an [`HttpTransport`].
    ///
    /// # Errors
    ///
    /// Returns an error if the URL does not use the `http` scheme, or if its host or
    /// port cannot be parsed.
    pub fn new(url: impl AsRef<str>) -> anyhow::Result<Self> {
        let url = url.as_ref();
        let rest = url.strip_prefix("http://").ok_or_else(|| {
            anyhow!(
                "only plaintext http:// webhook URLs are supported (TLS is unavailable), was {url:?}"
            )
        })?;

        let (authority, path) = match rest.find('/') {
            Some(index) => (&rest[..index], &rest[index..]),
            None => (rest, "/"),
        };

        if authority.is_empty() {
            bail!("webhook URL has no host: {url:?}");
        }

        let (host, port) = match authority.rsplit_once(':') {
            Some((host, port)) => (host, port.parse::<u16>().context("invalid webhook port")?),
            None => (authority, DEFAULT_HTTP_PORT),
        };

        if host.is_empty() {
            bail!("webhook URL has no host: {url:?}");
        }

        Ok(Self {
            host: host.to_string(),
            port,
            path: path.to_string(),
            timeout: DEFAULT_TIMEOUT,
        })
    }

    /// Sets the read and write timeout applied to the socket.
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

impl NotificationTransport for HttpTransport {
    fn deliver(&mut self, message: &NotificationMessage) -> anyhow::Result<()> {
        let body = serde_json::to_string(&serde_json::json!({
            "title": message.title,
            "body": message.body,
            "severity": message.severity.to_string(),
        }))
        .context("failed to serialize webhook payload")?;

        let stream = TcpStream::connect((self.host.as_str(), self.port))
            .with_context(|| format!("failed to connect to webhook {}:{}", self.host, self.port))?;
        stream
            .set_read_timeout(Some(self.timeout))
            .context("failed to set webhook read timeout")?;
        stream
            .set_write_timeout(Some(self.timeout))
            .context("failed to set webhook write timeout")?;

        let host_header = if self.port == DEFAULT_HTTP_PORT {
            self.host.clone()
        } else {
            format!("{}:{}", self.host, self.port)
        };

        let request = format!(
            "POST {path} HTTP/1.1\r\nHost: {host}\r\nContent-Type: application/json\r\n\
             Content-Length: {length}\r\nConnection: close\r\n\r\n{body}",
            path = self.path,
            host = host_header,
            length = body.len(),
            body = body,
        );

        let mut reader = BufReader::new(stream);
        reader
            .get_mut()
            .write_all(request.as_bytes())
            .context("failed to write webhook request")?;

        let mut status = String::new();
        reader
            .read_line(&mut status)
            .context("failed to read webhook response status line")?;

        if !status.starts_with("HTTP/") {
            bail!("malformed webhook response status line: {status:?}");
        }

        let code: u16 = status
            .split_whitespace()
            .nth(1)
            .and_then(|token| token.parse().ok())
            .ok_or_else(|| anyhow!("malformed webhook response status line: {status:?}"))?;

        if !(200..300).contains(&code) {
            bail!("webhook returned HTTP {code}");
        }

        // Drain the response so the server observes a clean read before we close.
        let mut sink = Vec::new();
        let _ = reader.read_to_end(&mut sink);

        Ok(())
    }
}

/// A webhook notification sink over an injectable [`NotificationTransport`].
#[derive(Clone, Debug)]
pub struct WebhookSink<T: NotificationTransport> {
    transport: T,
}

impl<T: NotificationTransport> WebhookSink<T> {
    /// Creates a new [`WebhookSink`] over the given transport.
    #[must_use]
    pub fn new(transport: T) -> Self {
        Self { transport }
    }
}

impl<T: NotificationTransport> NotificationSink for WebhookSink<T> {
    fn send(&mut self, message: &NotificationMessage) -> anyhow::Result<()> {
        self.transport.deliver(message)
    }
}
