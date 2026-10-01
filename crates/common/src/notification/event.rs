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

//! Notification events and their classification.

use std::fmt::Display;

use nautilus_core::UnixNanos;
use nautilus_model::identifiers::{AccountId, InstrumentId, StrategyId};
use ustr::Ustr;

/// The fixed set of notification classes a sink can subscribe to.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum NotificationClass {
    /// A risk limit was breached and the risk engine halted trading.
    RiskLimitBreach,
    /// An order was rejected by a venue or by the risk engine.
    OrderRejection,
    /// A strategy stopped.
    StrategyStop,
    /// An order execution completed.
    ExecutionCompletion,
    /// A drawdown threshold was breached.
    DrawdownThresholdBreach,
    /// A data feed disconnected.
    DataFeedDisconnect,
    /// A broker or execution connection disconnected.
    BrokerDisconnect,
    /// A backtest run completed.
    BacktestCompletion,
    /// An optimization run completed.
    OptimizationCompletion,
}

impl NotificationClass {
    /// Returns the stable `SCREAMING_SNAKE_CASE` name of the class.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Self::RiskLimitBreach => "RISK_LIMIT_BREACH",
            Self::OrderRejection => "ORDER_REJECTION",
            Self::StrategyStop => "STRATEGY_STOP",
            Self::ExecutionCompletion => "EXECUTION_COMPLETION",
            Self::DrawdownThresholdBreach => "DRAWDOWN_THRESHOLD_BREACH",
            Self::DataFeedDisconnect => "DATA_FEED_DISCONNECT",
            Self::BrokerDisconnect => "BROKER_DISCONNECT",
            Self::BacktestCompletion => "BACKTEST_COMPLETION",
            Self::OptimizationCompletion => "OPTIMIZATION_COMPLETION",
        }
    }
}

impl Display for NotificationClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// The severity of a notification.
///
/// This is intentionally a separate enum rather than a reuse of [`crate::enums::LogLevel`],
/// because a notification severity is a delivery contract (how loudly a sink should alert)
/// while `LogLevel` governs console and file logging. The variants map to the `log` levels
/// `info`, `warn`, and `error` respectively when a notification is also logged.
#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum NotificationSeverity {
    /// Informational, no action required.
    Info,
    /// A warning that should be investigated.
    Warning,
    /// A critical condition requiring immediate attention.
    Critical,
}

impl Display for NotificationSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Info => "INFO",
            Self::Warning => "WARNING",
            Self::Critical => "CRITICAL",
        })
    }
}

/// A classified notification with the identities a sink needs to route it.
///
/// All identity fields are optional because not every class has every identity; for
/// example a data feed disconnect has no strategy, while a backtest completion has a
/// run but no live account.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotificationEvent {
    /// The notification class.
    pub class: NotificationClass,
    /// The notification severity.
    pub severity: NotificationSeverity,
    /// The strategy associated with the event, if any.
    pub strategy_id: Option<StrategyId>,
    /// The instrument associated with the event, if any.
    pub instrument_id: Option<InstrumentId>,
    /// The account associated with the event, if any.
    pub account_id: Option<AccountId>,
    /// The backtest or optimization run associated with the event, if any.
    pub run_id: Option<Ustr>,
    /// The human-readable detail for the event.
    pub message: String,
    /// UNIX timestamp (nanoseconds) when the event occurred.
    pub ts_event: UnixNanos,
}

impl NotificationEvent {
    /// Creates a new [`NotificationEvent`] with no identities attached.
    #[must_use]
    pub fn new(
        class: NotificationClass,
        severity: NotificationSeverity,
        message: impl Into<String>,
    ) -> Self {
        Self {
            class,
            severity,
            strategy_id: None,
            instrument_id: None,
            account_id: None,
            run_id: None,
            message: message.into(),
            ts_event: UnixNanos::default(),
        }
    }

    /// Sets the strategy identity.
    #[must_use]
    pub fn with_strategy_id(mut self, strategy_id: StrategyId) -> Self {
        self.strategy_id = Some(strategy_id);
        self
    }

    /// Sets the instrument identity.
    #[must_use]
    pub fn with_instrument_id(mut self, instrument_id: InstrumentId) -> Self {
        self.instrument_id = Some(instrument_id);
        self
    }

    /// Sets the account identity.
    #[must_use]
    pub fn with_account_id(mut self, account_id: AccountId) -> Self {
        self.account_id = Some(account_id);
        self
    }

    /// Sets the run identity.
    #[must_use]
    pub fn with_run_id(mut self, run_id: impl Into<Ustr>) -> Self {
        self.run_id = Some(run_id.into());
        self
    }

    /// Sets the event timestamp.
    #[must_use]
    pub fn with_ts_event(mut self, ts_event: UnixNanos) -> Self {
        self.ts_event = ts_event;
        self
    }
}
