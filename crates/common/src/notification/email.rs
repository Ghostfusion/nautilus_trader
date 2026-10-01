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

//! The SMTP email sink and its plaintext transport.
//!
//! [`SmtpTransport`] speaks a minimal plaintext SMTP dialogue (`EHLO`, `MAIL FROM`,
//! `RCPT TO`, `DATA`, `QUIT`) over a blocking `std::net::TcpStream`. It does **not**
//! negotiate TLS and does not authenticate, so it is suitable for a trusted local
//! relay or tunnel only.

use std::{
    io::{BufRead, BufReader, Write},
    net::TcpStream,
    time::Duration,
};

use anyhow::{Context, bail};

use super::{
    sink::{NotificationMessage, NotificationSink},
    transport::NotificationTransport,
};

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// A plaintext SMTP transport.
#[derive(Clone, Debug)]
pub struct SmtpTransport {
    host: String,
    port: u16,
    from: String,
    recipients: Vec<String>,
    timeout: Duration,
}

impl SmtpTransport {
    /// Creates a new [`SmtpTransport`].
    #[must_use]
    pub fn new(
        host: impl Into<String>,
        port: u16,
        from: impl Into<String>,
        recipients: Vec<String>,
    ) -> Self {
        Self {
            host: host.into(),
            port,
            from: from.into(),
            recipients,
            timeout: DEFAULT_TIMEOUT,
        }
    }

    /// Sets the read and write timeout applied to the socket.
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    fn render(&self, message: &NotificationMessage) -> String {
        format!(
            "From: {from}\r\nTo: {to}\r\nSubject: {subject}\r\nX-Severity: {severity}\r\n\
             MIME-Version: 1.0\r\nContent-Type: text/plain; charset=utf-8\r\n\r\n{body}\r\n",
            from = self.from,
            to = self.recipients.join(", "),
            subject = message.title,
            severity = message.severity,
            body = message.body,
        )
    }
}

impl NotificationTransport for SmtpTransport {
    fn deliver(&mut self, message: &NotificationMessage) -> anyhow::Result<()> {
        let stream = TcpStream::connect((self.host.as_str(), self.port)).with_context(|| {
            format!(
                "failed to connect to SMTP server {}:{}",
                self.host, self.port
            )
        })?;
        stream
            .set_read_timeout(Some(self.timeout))
            .context("failed to set SMTP read timeout")?;
        stream
            .set_write_timeout(Some(self.timeout))
            .context("failed to set SMTP write timeout")?;

        let mut conn = BufReader::new(stream);

        expect_code(&mut conn, 220, "greeting")?;

        send_line(conn.get_mut(), "EHLO localhost")?;
        expect_code(&mut conn, 250, "EHLO")?;

        send_line(conn.get_mut(), &format!("MAIL FROM:<{}>", self.from))?;
        expect_code(&mut conn, 250, "MAIL FROM")?;

        for recipient in &self.recipients {
            send_line(conn.get_mut(), &format!("RCPT TO:<{recipient}>"))?;
            let code = read_code(&mut conn)?;
            if code != 250 && code != 251 {
                bail!("SMTP RCPT TO rejected with code {code}");
            }
        }

        send_line(conn.get_mut(), "DATA")?;
        expect_code(&mut conn, 354, "DATA")?;

        let email = self.render(message);
        conn.get_mut()
            .write_all(email.as_bytes())
            .context("failed to write SMTP message body")?;
        conn.get_mut()
            .write_all(b"\r\n.\r\n")
            .context("failed to terminate SMTP message body")?;
        expect_code(&mut conn, 250, "message body")?;

        send_line(conn.get_mut(), "QUIT")?;

        Ok(())
    }
}

fn send_line(stream: &mut TcpStream, line: &str) -> anyhow::Result<()> {
    stream
        .write_all(format!("{line}\r\n").as_bytes())
        .with_context(|| format!("failed to write SMTP command: {line}"))
}

fn expect_code(
    reader: &mut BufReader<TcpStream>,
    expected: u16,
    stage: &str,
) -> anyhow::Result<()> {
    let code = read_code(reader)?;
    if code != expected {
        bail!("SMTP {stage} expected code {expected}, was {code}");
    }
    Ok(())
}

fn read_code(reader: &mut BufReader<TcpStream>) -> anyhow::Result<u16> {
    loop {
        let mut line = String::new();
        let read = reader
            .read_line(&mut line)
            .context("failed to read SMTP response")?;
        if read == 0 {
            bail!("SMTP server closed the connection");
        }

        let bytes = line.as_bytes();
        if bytes.len() < 3 {
            bail!("malformed SMTP response: {line:?}");
        }

        let code: u16 = line[..3]
            .parse()
            .with_context(|| format!("malformed SMTP status code: {line:?}"))?;

        // A hyphen at position four begins a multi-line response; keep reading.
        if bytes.get(3) == Some(&b'-') {
            continue;
        }

        return Ok(code);
    }
}

/// An email notification sink over an injectable [`NotificationTransport`].
#[derive(Clone, Debug)]
pub struct EmailSink<T: NotificationTransport> {
    transport: T,
}

impl<T: NotificationTransport> EmailSink<T> {
    /// Creates a new [`EmailSink`] over the given transport.
    #[must_use]
    pub fn new(transport: T) -> Self {
        Self { transport }
    }
}

impl<T: NotificationTransport> NotificationSink for EmailSink<T> {
    fn send(&mut self, message: &NotificationMessage) -> anyhow::Result<()> {
        self.transport.deliver(message)
    }
}
