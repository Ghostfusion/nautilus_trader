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

//! The notification router: bus subscriptions, classification, and sink dispatch.

use std::{
    cell::RefCell,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::{SyncSender, TrySendError},
    },
};

use ahash::AHashSet;
use nautilus_model::{
    enums::TradingState,
    events::{OrderEventAny, PositionEvent},
};
use ustr::Ustr;

use super::{
    event::{NotificationClass, NotificationEvent, NotificationSeverity},
    sink::{
        NotificationMessage, NotificationSink, NotificationSinkConfig, SinkChannel,
        spawn_sink_worker,
    },
};
use crate::{
    messages::system::TradingStateChanged,
    msgbus::{self, MStr, Pattern, TypedHandler, switchboard::MessagingSwitchboard},
};

/// The message bus topic on which a [`NotificationEvent`] can be published directly.
///
/// Subsystems that detect an event with no existing bus representation (for example a
/// data feed disconnect) publish here; the router picks it up when subscribed.
pub const NOTIFICATION_TOPIC: &str = "events.notification";

struct SinkRegistration {
    name: Ustr,
    classes: AHashSet<NotificationClass>,
    sender: SyncSender<NotificationMessage>,
    dropped: Arc<AtomicU64>,
    sent: Arc<AtomicU64>,
}

impl std::fmt::Debug for SinkRegistration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(SinkRegistration))
            .field("name", &self.name)
            .field("classes", &self.classes)
            .field("dropped", &self.dropped.load(Ordering::Relaxed))
            .field("sent", &self.sent.load(Ordering::Relaxed))
            .finish()
    }
}

/// Routes classified notification events to the sinks registered for their class.
#[derive(Debug, Default)]
pub struct NotificationRouter {
    sinks: RefCell<Vec<SinkRegistration>>,
    drawdown_threshold: Option<f64>,
}

impl NotificationRouter {
    /// Creates a new empty [`NotificationRouter`] with no sinks and no drawdown threshold.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the realized-return magnitude at or above which a position change raises a
    /// [`NotificationClass::DrawdownThresholdBreach`].
    ///
    /// A position change raises the breach when its `realized_return` is less than or
    /// equal to the negated threshold. Position events are ignored while the threshold
    /// is unset.
    pub fn set_drawdown_threshold(&mut self, threshold: f64) {
        self.drawdown_threshold = Some(threshold);
    }

    /// Registers a sink for one or more classes and spawns its worker thread.
    ///
    /// The sink receives events whose class is in `classes`. The queue is bounded by
    /// `config`; when it is saturated the publisher drops the message and increments the
    /// sink's drop counter.
    pub fn register_sink<S: NotificationSink + 'static>(
        &self,
        name: impl Into<Ustr>,
        sink: S,
        classes: impl IntoIterator<Item = NotificationClass>,
        config: NotificationSinkConfig,
    ) {
        let name = name.into();
        let SinkChannel {
            sender,
            dropped,
            sent,
        } = spawn_sink_worker(name, config, Box::new(sink));

        self.sinks.borrow_mut().push(SinkRegistration {
            name,
            classes: classes.into_iter().collect(),
            sender,
            dropped,
            sent,
        });
    }

    /// Dispatches a notification event to every sink registered for its class.
    ///
    /// This is a no-op when no sink is registered, so a subsystem never depends on a
    /// sink being configured. The call never blocks: a saturated sink queue drops the
    /// message.
    pub fn publish(&self, event: &NotificationEvent) {
        let message = NotificationMessage::from_event(event);

        for sink in self.sinks.borrow().iter() {
            if !sink.classes.contains(&event.class) {
                continue;
            }

            match sink.sender.try_send(message.clone()) {
                Ok(()) => {}
                Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) => {
                    sink.dropped.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }

    /// Subscribes the router to the existing bus subscription points and the
    /// [`NOTIFICATION_TOPIC`] direct-publish point.
    ///
    /// Handlers translate order, position, and risk events into notification classes and
    /// then dispatch them through [`Self::publish`].
    pub fn subscribe(self: &Rc<Self>) {
        let order_handler = TypedHandler::from({
            let router = Rc::clone(self);
            move |event: &OrderEventAny| router.on_order_event(event)
        });
        msgbus::subscribe_order_events(
            MStr::<Pattern>::from("events.order.*"),
            order_handler,
            None,
        );

        let position_handler = TypedHandler::from({
            let router = Rc::clone(self);
            move |event: &PositionEvent| router.on_position_event(event)
        });
        msgbus::subscribe_position_events(
            MStr::<Pattern>::from("events.position.*"),
            position_handler,
            None,
        );

        let risk_handler = TypedHandler::from_typed({
            let router = Rc::clone(self);
            move |event: &TradingStateChanged| router.on_trading_state_changed(event)
        });
        msgbus::subscribe_any(
            MessagingSwitchboard::risk_events_topic().into(),
            risk_handler,
            None,
        );

        let notification_handler = TypedHandler::from_typed({
            let router = Rc::clone(self);
            move |event: &NotificationEvent| router.publish(event)
        });
        msgbus::subscribe_any(
            MStr::<Pattern>::from(NOTIFICATION_TOPIC),
            notification_handler,
            None,
        );
    }

    /// Returns the number of messages dropped by the named sink's bounded queue.
    ///
    /// Returns zero when no sink with that name is registered.
    #[must_use]
    pub fn drop_count(&self, name: &str) -> u64 {
        self.sinks
            .borrow()
            .iter()
            .find(|sink| sink.name.as_str() == name)
            .map_or(0, |sink| sink.dropped.load(Ordering::Relaxed))
    }

    /// Returns the number of messages delivered by the named sink.
    ///
    /// Returns zero when no sink with that name is registered.
    #[must_use]
    pub fn sent_count(&self, name: &str) -> u64 {
        self.sinks
            .borrow()
            .iter()
            .find(|sink| sink.name.as_str() == name)
            .map_or(0, |sink| sink.sent.load(Ordering::Relaxed))
    }

    fn on_order_event(&self, event: &OrderEventAny) {
        match event {
            OrderEventAny::Rejected(rejected) => {
                let notification = NotificationEvent::new(
                    NotificationClass::OrderRejection,
                    NotificationSeverity::Warning,
                    format!("Order rejected: {}", rejected.reason),
                )
                .with_strategy_id(rejected.strategy_id)
                .with_instrument_id(rejected.instrument_id)
                .with_account_id(rejected.account_id)
                .with_ts_event(rejected.ts_event);

                self.publish(&notification);
            }
            OrderEventAny::Filled(filled) => {
                let notification = NotificationEvent::new(
                    NotificationClass::ExecutionCompletion,
                    NotificationSeverity::Info,
                    format!(
                        "Order filled: {} {} @ {}",
                        filled.client_order_id, filled.last_qty, filled.last_px
                    ),
                )
                .with_strategy_id(filled.strategy_id)
                .with_instrument_id(filled.instrument_id)
                .with_account_id(filled.account_id)
                .with_ts_event(filled.ts_event);

                self.publish(&notification);
            }
            _ => {}
        }
    }

    fn on_position_event(&self, event: &PositionEvent) {
        let Some(threshold) = self.drawdown_threshold else {
            return;
        };

        let PositionEvent::PositionChanged(changed) = event else {
            return;
        };

        if changed.realized_return > -threshold {
            return;
        }

        let notification = NotificationEvent::new(
            NotificationClass::DrawdownThresholdBreach,
            NotificationSeverity::Critical,
            format!(
                "Drawdown threshold breached: realized_return={:.4}",
                changed.realized_return
            ),
        )
        .with_strategy_id(changed.strategy_id)
        .with_instrument_id(changed.instrument_id)
        .with_account_id(changed.account_id)
        .with_ts_event(changed.ts_event);

        self.publish(&notification);
    }

    fn on_trading_state_changed(&self, event: &TradingStateChanged) {
        if event.state != TradingState::Halted {
            return;
        }

        let notification = NotificationEvent::new(
            NotificationClass::RiskLimitBreach,
            NotificationSeverity::Critical,
            "Risk engine halted trading",
        )
        .with_ts_event(event.ts_event);

        self.publish(&notification);
    }
}
