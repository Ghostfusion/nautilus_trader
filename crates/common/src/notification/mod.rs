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

//! Notification routing and delivery.
//!
//! The notification subsystem turns a small, named set of operational events into
//! outbound messages. It is intentionally closed: a [`NotificationEvent`] carries one
//! of the nine [`NotificationClass`] variants, so a sink cannot be registered for an
//! open-ended event type it has never heard of.
//!
//! # Classes
//!
//! - [`NotificationClass::RiskLimitBreach`]
//! - [`NotificationClass::OrderRejection`]
//! - [`NotificationClass::StrategyStop`]
//! - [`NotificationClass::ExecutionCompletion`]
//! - [`NotificationClass::DrawdownThresholdBreach`]
//! - [`NotificationClass::DataFeedDisconnect`]
//! - [`NotificationClass::BrokerDisconnect`]
//! - [`NotificationClass::BacktestCompletion`]
//! - [`NotificationClass::OptimizationCompletion`]
//!
//! [`NotificationRouter::subscribe`] translates the events already flowing on the
//! message bus (order, position, and risk events) into the classes they can express.
//! Classes with no existing bus event (strategy stop, data feed disconnect, broker
//! disconnect, backtest completion, optimization completion) are raised by publishing
//! a [`NotificationEvent`] to [`NOTIFICATION_TOPIC`] or by calling
//! [`NotificationRouter::publish`] directly.
//!
//! # Delivery model
//!
//! Each registered sink owns a worker thread fed by a bounded queue. Publishing
//! never blocks and never runs transport I/O on the publisher thread: a full queue
//! drops the message and increments a counter, while the worker coalesces a burst of
//! messages that arrive within the configured interval into a single send. A send
//! failure is reported through the ordinary logging path (`log_error!`) and never
//! re-enters the router.
//!
//! # Transport security
//!
//! The default transports ([`SmtpTransport`] and [`HttpTransport`]) speak plaintext
//! over a blocking `std::net::TcpStream` and do not negotiate TLS. They can be used
//! over a private network, a local relay, or an authenticated tunnel, but credentials
//! and notification bodies are sent unencrypted on the wire. Provide a custom
//! [`NotificationTransport`] implementation to add TLS.

#![allow(
    clippy::module_name_repetitions,
    reason = "notification types are named for their public role when imported outside this module"
)]

mod email;
mod event;
mod router;
mod sink;
mod transport;
mod webhook;

pub use self::{
    email::{EmailSink, SmtpTransport},
    event::{NotificationClass, NotificationEvent, NotificationSeverity},
    router::{NOTIFICATION_TOPIC, NotificationRouter},
    sink::{NotificationMessage, NotificationResult, NotificationSink, NotificationSinkConfig},
    transport::NotificationTransport,
    webhook::{HttpTransport, WebhookSink},
};
