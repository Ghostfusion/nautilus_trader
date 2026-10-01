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

//! Sniper execution algorithm.
//!
//! A sniper sweeps displayed liquidity at a fixed limit price, one slice at a time, and only when
//! the touch is reachable: for a buy when the best ask is at or below the sweep price, for a sell
//! when the best bid is at or above it. At most one child is working at a time. A terminal child
//! (cancelled, filled, denied, rejected or expired) permits the next one while the primary has
//! quantity remaining and fewer than `max_children` children have been sent.
//!
//! Each child is a limit order at the sweep price for `min(remaining, displayed size at the touch)`,
//! where the displayed size is the quote's ask size for a buy and bid size for a sell, floored to
//! the instrument's size precision and never below its minimum quantity. A touch whose displayed
//! size cannot form a valid child sends nothing and is re-checked by the next quote. A denied or
//! rejected child is not retried: the sequence completes and the primary carries the child's refusal.
//!
//! A caller that needs an upper bound on the sequence declares the policy horizon; without one a
//! sniper waits for the touch to become reachable rather than quoting at a price the market is not
//! showing, which is the behaviour it is for but leaves the sequence open indefinitely.
//!
//! # Parameters
//!
//! Orders submitted to this algorithm must include `exec_algorithm_params` with:
//! - `limit_price`: The sweep price each child is quoted at.
//! - `max_children`: The hard bound on the number of children the sequence may send.
//!
//! The horizon of the execution policy is honoured as the sequence deadline, and an aggressive
//! preference is what the algorithm does already. A passive preference is refused because the
//! algorithm crosses deliberately, and a declared price limit is refused by name because the sweep
//! price is this algorithm's own `limit_price` parameter. The other policy parts are refused with a
//! denial naming the field, rather than executed as though they had not been declared.

use std::time::Duration;

use ahash::AHashMap;
use indexmap::IndexMap;
use nautilus_common::{
    actor::{DataActor, DataActorNative},
    timer::TimeEvent,
};
use nautilus_core::UnixNanos;
use nautilus_model::{
    data::QuoteTick,
    enums::{OrderSide, OrderType},
    events::{
        OrderCanceled, OrderDenied, OrderDeniedReason, OrderExpired, OrderFilled, OrderRejected,
    },
    identifiers::{ClientOrderId, InstrumentId},
    instruments::{Instrument, InstrumentAny},
    orders::{Order, OrderAny},
    types::{Price, Quantity},
};
use rust_decimal::{Decimal, RoundingStrategy};
use ustr::Ustr;

use super::{
    ExecutionAlgorithm, ExecutionAlgorithmConfig, ExecutionAlgorithmCore, ExecutionAlgorithmNative,
    ExecutionIntent, ExecutionPolicy, ExecutionPreference, PolicyError, PolicyPart,
};
use crate::nautilus_execution_algorithm;

/// Configuration for [`SniperAlgorithm`].
pub type SniperAlgorithmConfig = ExecutionAlgorithmConfig;

/// The policy parts sniper can honour.
///
/// The horizon becomes the sequence deadline and an aggressive preference is what the algorithm
/// does already. A passive preference and a price limit are refused explicitly before this list is
/// consulted; a participation rate, a slippage cap and an urgency would each have to change how the
/// children are placed, which this algorithm does not do.
static SNIPER_SUPPORTED_POLICY_PARTS: [PolicyPart; 2] =
    [PolicyPart::Horizon, PolicyPart::Preference];

/// The parameter key declaring the sweep price each child is quoted at.
const KEY_LIMIT_PRICE: &str = "limit_price";
/// The parameter key declaring the hard bound on the children the sequence may send.
const KEY_MAX_CHILDREN: &str = "max_children";

/// Sniper execution algorithm.
///
/// Sweeps displayed liquidity at a fixed limit price, keeping at most one child working and
/// sending the next when the previous one is terminal and the touch is reachable.
#[derive(Debug)]
pub struct SniperAlgorithm {
    /// The algorithm core.
    pub core: ExecutionAlgorithmCore,
    /// The scheduling state of each primary order.
    schedules: AHashMap<ClientOrderId, SniperSchedule>,
}

impl SniperAlgorithm {
    /// Creates a new [`SniperAlgorithm`] instance.
    #[must_use]
    pub fn new(config: SniperAlgorithmConfig) -> Self {
        Self {
            core: ExecutionAlgorithmCore::new(config),
            schedules: AHashMap::new(),
        }
    }

    /// Returns the children submitted and cancelled for a primary order.
    ///
    /// This is the churn evidence the algorithm owes: a caller can compare it against the risk
    /// engine's own count limits rather than assuming the algorithm is within them.
    #[must_use]
    pub fn counts(&self, primary_id: &ClientOrderId) -> Option<(u32, u32)> {
        self.schedules
            .get(primary_id)
            .map(|schedule| (schedule.submitted, schedule.cancelled))
    }

    /// Returns the working child of a primary order, if one is working.
    #[must_use]
    pub fn working_child(&self, primary_id: &ClientOrderId) -> Option<ClientOrderId> {
        self.schedules.get(primary_id).and_then(|s| s.child)
    }

    /// Completes the execution sequence for a primary order.
    fn complete_sequence(&mut self, primary_id: ClientOrderId) {
        let timer_name = primary_id.as_str();
        let core = ExecutionAlgorithmNative::exec_algorithm_core_mut(self);
        if core.clock_mut().timer_names().contains(&timer_name) {
            core.clock_mut().cancel_timer(timer_name);
        }
        core.remove_submit_params(&primary_id);

        if let Some(schedule) = self.schedules.remove(&primary_id) {
            DataActor::unsubscribe_quotes(self, schedule.instrument_id, None, None);
        }

        log::info!("Completed sniper execution for {primary_id}");
    }

    /// Returns the primary order of a child order, when this algorithm owns the child.
    fn primary_of_child(&self, child_id: ClientOrderId) -> Option<ClientOrderId> {
        let cache = ExecutionAlgorithmNative::exec_algorithm_core(self).cache_ref();

        cache
            .order(&child_id)
            .and_then(|order| order.exec_spawn_id())
    }

    /// Returns the primary order from the cache, if it is still there.
    fn cached_order(&self, client_order_id: ClientOrderId) -> Option<OrderAny> {
        let cache = ExecutionAlgorithmNative::exec_algorithm_core(self).cache_ref();

        cache.order(&client_order_id).map(|order| order.clone())
    }

    /// Returns the instrument from the cache, if it is still there.
    fn cached_instrument(&self, instrument_id: &InstrumentId) -> Option<InstrumentAny> {
        let cache = ExecutionAlgorithmNative::exec_algorithm_core(self).cache_ref();

        cache.instrument(instrument_id).cloned()
    }

    /// Sends the next child for a primary order, when the touch permits it.
    ///
    /// The sequence completes when the primary has no quantity remaining or the `max_children`
    /// bound has been reached with quantity remaining; otherwise a child is sent only when the
    /// stored touch is reachable and its displayed size can form a valid child. A touch that
    /// cannot is left to the next quote.
    fn send_next_child(&mut self, primary_id: ClientOrderId) {
        let (touch, max_children, submitted, side, price, instrument_id) =
            match self.schedules.get(&primary_id) {
                Some(schedule) => (
                    schedule.touch,
                    schedule.max_children,
                    schedule.submitted,
                    schedule.side,
                    schedule.price,
                    schedule.instrument_id,
                ),
                None => return,
            };

        let Some(primary) = self.cached_order(primary_id) else {
            log::warn!("Cannot find primary order {primary_id} to send the next child");
            self.complete_sequence(primary_id);
            return;
        };

        let remaining = primary.quantity();
        if remaining.is_zero() {
            self.complete_sequence(primary_id);
            return;
        }

        if submitted >= max_children {
            log::info!("Sniper sequence {primary_id} reached its {max_children} child bound");
            self.complete_sequence(primary_id);
            return;
        }

        let Some(touch) = touch else {
            // No quote has arrived yet, so the touch is unknown and the next quote re-checks it.
            return;
        };

        if !touch_reachable(side, touch.best, price) {
            return;
        }

        let Some(instrument) = self.cached_instrument(&instrument_id) else {
            let reason = OrderDeniedReason::InstrumentNotFound { instrument_id }.to_string();
            let _ = self.deny_order(&primary, Ustr::from(&reason));
            self.complete_sequence(primary_id);
            return;
        };

        let Some(sized) = size_at_touch(&instrument, touch.displayed) else {
            // The displayed size cannot form a valid child, so this touch sends nothing.
            return;
        };

        let quantity = if remaining < sized { remaining } else { sized };
        let time_in_force = primary.time_in_force();
        let post_only = primary.is_post_only();
        let reduce_only = primary.is_reduce_only();
        let tags = primary.tags().map(<[Ustr]>::to_vec);

        let mut primary = primary;
        let spawned = self.spawn_limit(
            &mut primary,
            quantity,
            price,
            time_in_force,
            None,
            post_only,
            reduce_only,
            None,
            None,
            tags,
            true,
        );
        let child_id = spawned.client_order_id();

        if let Err(e) = self.submit_order(spawned.into(), None, None) {
            log::error!("Failed to submit the sniper child {child_id}: {e}");
            self.complete_sequence(primary_id);
            return;
        }

        if let Some(schedule) = self.schedules.get_mut(&primary_id) {
            schedule.child = Some(child_id);
            schedule.submitted += 1;
        }

        log::info!("Sniper child {child_id} submitted for {quantity} at {price}");
    }

    /// Handles a child order that has reached a terminal state.
    ///
    /// The unfilled quantity of a cancelled, expired or denied child is restored to the primary
    /// order by the core, so the primary's own quantity is the quantity still to execute.
    fn handle_child_terminal(&mut self, primary_id: ClientOrderId, cancelled: bool) {
        let deadline = {
            let Some(schedule) = self.schedules.get_mut(&primary_id) else {
                return;
            };

            if cancelled {
                schedule.cancelled += 1;
            }
            schedule.child = None;
            schedule.deadline
        };

        if let Some(deadline) = deadline {
            let now = ExecutionAlgorithmNative::exec_algorithm_core_mut(self)
                .clock_mut()
                .timestamp_ns();

            if now.as_u64() >= deadline.as_u64() {
                self.complete_sequence(primary_id);
                return;
            }
        }

        self.send_next_child(primary_id);
    }

    /// Stores the touch from a quote and sends the next child when none is working.
    ///
    /// The touch is recorded on every quote, including while a child is working, so a terminal
    /// child re-checks the most recent one. A quote never itself sends a second child while one is
    /// already working.
    fn handle_quote(&mut self, quote: &QuoteTick) {
        let primary_ids: Vec<ClientOrderId> = self.schedules.keys().copied().collect();

        for primary_id in primary_ids {
            let send = {
                let Some(schedule) = self.schedules.get_mut(&primary_id) else {
                    continue;
                };

                if schedule.instrument_id != quote.instrument_id {
                    continue;
                }

                let (best, displayed) = match schedule.side {
                    OrderSide::Buy => (quote.ask_price, quote.ask_size),
                    _ => (quote.bid_price, quote.bid_size),
                };
                schedule.touch = Some(Touch { best, displayed });
                schedule.child.is_none()
            };

            if send {
                self.send_next_child(primary_id);
            }
        }
    }
}

// The clock and component lifecycle dispatch through the `DataActor` hooks,
// so forward them to the `ExecutionAlgorithm` implementations.
impl DataActor for SniperAlgorithm {
    fn on_quote(&mut self, quote: &QuoteTick) -> anyhow::Result<()> {
        self.handle_quote(quote);

        Ok(())
    }

    fn on_time_event(&mut self, event: &TimeEvent) -> anyhow::Result<()> {
        ExecutionAlgorithm::on_time_event(self, event)
    }

    fn on_stop(&mut self) -> anyhow::Result<()> {
        ExecutionAlgorithm::on_stop(self)
    }

    fn on_resume(&mut self) -> anyhow::Result<()> {
        ExecutionAlgorithm::on_resume(self)
    }

    fn on_reset(&mut self) -> anyhow::Result<()> {
        ExecutionAlgorithm::on_reset(self)
    }
}

nautilus_execution_algorithm!(SniperAlgorithm, {
    fn supported_policy_parts(&self) -> &'static [PolicyPart] {
        &SNIPER_SUPPORTED_POLICY_PARTS
    }

    fn on_order(&mut self, order: OrderAny) -> anyhow::Result<()> {
        let primary_id = order.client_order_id();

        if self.schedules.contains_key(&primary_id) {
            anyhow::bail!("Order {primary_id} already being executed");
        }

        log::info!("Received order for sniper execution: {order:?}");

        // A sniper is placed by a limit order at the sweep price.
        if order.order_type() != OrderType::Limit {
            let reason = OrderDeniedReason::UnsupportedOrderType {
                order_type: order.order_type(),
            }
            .to_string();
            return self.deny_order(&order, Ustr::from(&reason));
        }

        let instrument = {
            let cache = ExecutionAlgorithmNative::exec_algorithm_core(self).cache_ref();
            cache.instrument(&order.instrument_id()).cloned()
        };

        if instrument.is_none() {
            let reason = OrderDeniedReason::InstrumentNotFound {
                instrument_id: order.instrument_id(),
            }
            .to_string();
            return self.deny_order(&order, Ustr::from(&reason));
        }

        let Some(exec_params) = order.exec_algorithm_params() else {
            return self.deny_order(&order, validation_failed("exec_algorithm_params not found"));
        };

        let policy = match ExecutionPolicy::from_params(exec_params) {
            Ok(policy) => policy,
            Err(err) => return self.deny_order(&order, validation_failed(err.message())),
        };

        // The sweep price is this algorithm's own parameter, so a declared price limit has no
        // independent effect and is refused by name rather than listed as supported.
        if policy.price_limit.is_some() {
            return self.deny_order(
                &order,
                validation_failed(
                    "price_limit is not supported: the sweep price is this algorithm's own \
                     limit_price parameter",
                ),
            );
        }

        if let Some(part) = policy.unsupported(self.supported_policy_parts()) {
            return self.deny_order(
                &order,
                validation_failed(PolicyError::unsupported(part).message()),
            );
        }

        if let Err(err) = policy.validate(&ExecutionIntent::from(&order)) {
            return self.deny_order(&order, validation_failed(err.message()));
        }

        if policy.preference == Some(ExecutionPreference::Passive) {
            return self.deny_order(
                &order,
                validation_failed(
                    "preference=passive must not be declared: this algorithm crosses the touch \
                     deliberately",
                ),
            );
        }

        let price = match parse_limit_price(exec_params) {
            Ok(price) => price,
            Err(reason) => return self.deny_order(&order, reason),
        };

        let max_children = match parse_max_children(exec_params) {
            Ok(max_children) => max_children,
            Err(reason) => return self.deny_order(&order, reason),
        };

        let deadline = match policy.horizon {
            Some(horizon) => {
                let now = ExecutionAlgorithmNative::exec_algorithm_core_mut(self)
                    .clock_mut()
                    .timestamp_ns();

                match now.as_u64().checked_add(horizon.as_u64()) {
                    Some(deadline) => Some(UnixNanos::new(deadline)),
                    None => {
                        return self.deny_order(
                            &order,
                            validation_failed("the horizon exceeds the clock timestamp headroom"),
                        );
                    }
                }
            }
            None => None,
        };

        // Add the primary to the cache so the schedule can retrieve it, it is already present
        // when the order is routed through the engine's submit path.
        {
            let cache_rc = ExecutionAlgorithmNative::exec_algorithm_core(self).cache_rc();
            let mut cache = cache_rc.borrow_mut();
            if !cache.order_exists(&primary_id) {
                cache.add_order(order.clone(), None, None, false)?;
            }
        }

        let instrument_id = order.instrument_id();
        let side = order.order_side();
        self.schedules.insert(
            primary_id,
            SniperSchedule {
                instrument_id,
                side,
                price,
                max_children,
                deadline,
                child: None,
                touch: None,
                submitted: 0,
                cancelled: 0,
            },
        );

        // The deadline is the sequence's own timer: when it fires the working child is cancelled
        // and the sequence completes with whatever quantity is left.
        if let Some(deadline) = deadline {
            let now = ExecutionAlgorithmNative::exec_algorithm_core_mut(self)
                .clock_mut()
                .timestamp_ns();
            let duration = Duration::from_nanos(deadline.as_u64().saturating_sub(now.as_u64()));
            ExecutionAlgorithmNative::exec_algorithm_core_mut(self)
                .clock_mut()
                .set_timer(primary_id.as_str(), duration, None, None, None, None, None)?;
        }

        DataActor::subscribe_quotes(self, instrument_id, None, None);

        // The first child waits for a quote, because the touch is unknown until one arrives.
        log::info!("Started sniper execution for {primary_id}");

        Ok(())
    }

    fn on_time_event(&mut self, event: &TimeEvent) -> anyhow::Result<()> {
        let primary_id = ClientOrderId::new(event.name);

        let Some(child_id) = self.working_child(&primary_id) else {
            if self.schedules.contains_key(&primary_id) {
                log::info!("Sniper deadline reached for {primary_id}");
                self.complete_sequence(primary_id);
            }
            return Ok(());
        };

        log::info!("Sniper deadline reached for {primary_id}, cancelling child {child_id}");

        if let Some(mut child) = self.cached_order(child_id) {
            let _ = self.cancel_order(&mut child, None);
        }

        self.complete_sequence(primary_id);

        Ok(())
    }

    fn on_algo_order_canceled(&mut self, event: OrderCanceled) {
        let child_id = event.client_order_id;

        if let Some(primary_id) = self.primary_of_child(child_id) {
            self.handle_child_terminal(primary_id, true);
        }
    }

    fn on_algo_order_filled(&mut self, event: OrderFilled) {
        let child_id = event.client_order_id;

        if let Some(primary_id) = self.primary_of_child(child_id) {
            self.handle_child_terminal(primary_id, false);
        }
    }

    fn on_order_expired(&mut self, event: OrderExpired) {
        let child_id = event.client_order_id;

        if let Some(primary_id) = self.primary_of_child(child_id) {
            self.handle_child_terminal(primary_id, true);
        }
    }

    fn on_order_denied(&mut self, event: OrderDenied) {
        self.abort_on_child_refusal(event.client_order_id, event.reason);
    }

    fn on_order_rejected(&mut self, event: OrderRejected) {
        let reason = OrderDeniedReason::SubmitFailed {
            detail: format!(
                "the sniper child {} was rejected by the venue: {}",
                event.client_order_id, event.reason
            ),
        }
        .to_string();

        self.abort_on_child_refusal(event.client_order_id, Ustr::from(&reason));
    }

    fn on_stop(&mut self) -> anyhow::Result<()> {
        ExecutionAlgorithmNative::exec_algorithm_core_mut(self)
            .clock_mut()
            .cancel_timers();

        Ok(())
    }

    fn on_resume(&mut self) -> anyhow::Result<()> {
        let primary_ids: Vec<ClientOrderId> = self.schedules.keys().copied().collect();

        for primary_id in primary_ids {
            let Some(child_id) = self.working_child(&primary_id) else {
                self.send_next_child(primary_id);
                continue;
            };

            match self.cached_order(child_id) {
                Some(child) if !child.is_closed() => {}
                _ => self.handle_child_terminal(primary_id, false),
            }
        }

        Ok(())
    }

    fn on_reset(&mut self) -> anyhow::Result<()> {
        self.unsubscribe_all_strategy_events();
        ExecutionAlgorithmNative::exec_algorithm_core_mut(self).reset();
        self.schedules.clear();

        Ok(())
    }
});

impl SniperAlgorithm {
    /// Refuses the primary order when one of its children cannot be executed.
    ///
    /// A refused child is not retried: the sequence completes and the primary carries the refusal,
    /// so the algorithm is never left neither quoting nor completing.
    fn abort_on_child_refusal(&mut self, child_id: ClientOrderId, reason: Ustr) {
        let Some(primary_id) = self.primary_of_child(child_id) else {
            return;
        };

        if let Some(primary) = self.cached_order(primary_id) {
            let _ = self.deny_order(&primary, reason);
        }

        self.complete_sequence(primary_id);
    }
}

/// The scheduling state of one primary order.
#[derive(Debug)]
struct SniperSchedule {
    /// The instrument the primary order executes in.
    instrument_id: InstrumentId,
    /// The side of the primary order.
    side: OrderSide,
    /// The sweep price each child is quoted at.
    price: Price,
    /// The hard bound on the number of children the sequence may send.
    max_children: u32,
    /// When the sequence must be complete, from the policy horizon.
    deadline: Option<UnixNanos>,
    /// The working child, if one is working.
    child: Option<ClientOrderId>,
    /// The most recent touch, whether or not it was reachable.
    touch: Option<Touch>,
    /// The children submitted, for the churn evidence.
    submitted: u32,
    /// The children cancelled, for the churn evidence.
    cancelled: u32,
}

/// The touch of the book as seen by the quote stream.
#[derive(Clone, Copy, Debug)]
struct Touch {
    /// The best opposite price: the ask for a buy, the bid for a sell.
    best: Price,
    /// The displayed size at that price: the ask size for a buy, the bid size for a sell.
    displayed: Quantity,
}

/// Returns whether the best opposite price makes the sweep price reachable.
fn touch_reachable(order_side: OrderSide, best: Price, limit: Price) -> bool {
    match order_side {
        OrderSide::Sell => best >= limit,
        _ => best <= limit,
    }
}

/// Returns the displayed size as a child quantity, or `None` when it cannot form a valid child.
///
/// The size is floored to the instrument's size precision and never below its minimum quantity.
fn size_at_touch(instrument: &InstrumentAny, displayed: Quantity) -> Option<Quantity> {
    if displayed.is_zero() || displayed.is_undefined() {
        return None;
    }

    let floored = displayed.as_decimal().round_dp_with_strategy(
        u32::from(instrument.size_precision()),
        RoundingStrategy::ToZero,
    );

    if floored <= Decimal::ZERO {
        return None;
    }

    let sized = instrument
        .try_make_qty_from_decimal(floored, Some(true))
        .ok()?;

    if let Some(min_qty) = instrument.min_quantity()
        && sized < min_qty
    {
        return Some(min_qty);
    }

    Some(sized)
}

fn validation_failed(detail: impl Into<String>) -> Ustr {
    let reason = OrderDeniedReason::ValidationFailed {
        detail: detail.into(),
    };

    Ustr::from(&reason.to_string())
}

/// Returns the sweep price declared in `params`.
fn parse_limit_price(params: &IndexMap<Ustr, Ustr>) -> Result<Price, Ustr> {
    let Some(raw) = params.get(&Ustr::from(KEY_LIMIT_PRICE)) else {
        return Err(validation_failed(
            "limit_price not found in exec_algorithm_params",
        ));
    };

    match raw.as_str().parse::<Price>() {
        Ok(price) => Ok(price),
        Err(_) => Err(validation_failed(format!(
            "limit_price={raw} is not a valid price"
        ))),
    }
}

/// Returns the child bound declared in `params`.
fn parse_max_children(params: &IndexMap<Ustr, Ustr>) -> Result<u32, Ustr> {
    let Some(raw) = params.get(&Ustr::from(KEY_MAX_CHILDREN)) else {
        return Err(validation_failed(
            "max_children not found in exec_algorithm_params",
        ));
    };

    let Ok(max_children) = raw.as_str().parse::<u32>() else {
        return Err(validation_failed(format!(
            "max_children={raw} is not a valid positive integer"
        )));
    };

    if max_children == 0 {
        return Err(validation_failed("max_children=0 must be positive"));
    }

    Ok(max_children)
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use indexmap::IndexMap;
    use nautilus_common::{
        cache::Cache,
        clock::{Clock, VirtualClock},
        component::Component,
        enums::ComponentTrigger,
        msgbus::{self, TypedHandler},
    };
    use nautilus_core::{UUID4, UnixNanos};
    use nautilus_model::{
        data::QuoteTick,
        enums::{OrderSide, OrderStatus, OrderType, TimeInForce},
        events::{
            OrderDenied, OrderDeniedReason, OrderEventAny,
            order::spec::{OrderAcceptedSpec, OrderCanceledSpec, OrderFilledSpec},
        },
        identifiers::{
            AccountId, ExecAlgorithmId, InstrumentId, StrategyId, TraderId, VenueOrderId,
        },
        orders::{LimitOrder, OrderAny},
        types::{Price, Quantity},
    };
    use rstest::rstest;
    use ustr::Ustr;

    use super::*;

    fn create_sniper_algorithm() -> SniperAlgorithm {
        // Use unique ID to avoid thread-local registry/msgbus conflicts in parallel tests
        let unique_id = format!("SNIPER-{}", UUID4::new());
        let config = SniperAlgorithmConfig {
            exec_algorithm_id: Some(ExecAlgorithmId::new(&unique_id)),
            ..Default::default()
        };
        SniperAlgorithm::new(config)
    }

    fn register_algorithm_with_clock(algo: &mut SniperAlgorithm) -> Rc<RefCell<VirtualClock>> {
        use nautilus_common::timer::TimeEventCallback;

        let trader_id = TraderId::from("TRADER-001");
        let clock = Rc::new(RefCell::new(VirtualClock::new()));
        let cache = Rc::new(RefCell::new(Cache::default()));

        // Register a no-op default handler for timer callbacks
        clock
            .borrow_mut()
            .register_default_handler(TimeEventCallback::Rust(std::sync::Arc::new(|_| {})));

        algo.core.register(trader_id, clock.clone(), cache).unwrap();

        // Transition to Running state for tests
        algo.transition_state(ComponentTrigger::Initialize).unwrap();
        algo.transition_state(ComponentTrigger::Start).unwrap();
        algo.transition_state(ComponentTrigger::StartCompleted)
            .unwrap();

        clock
    }

    fn register_algorithm(algo: &mut SniperAlgorithm) {
        let _ = register_algorithm_with_clock(algo);
    }

    fn add_instrument_to_cache(algo: &SniperAlgorithm) {
        use nautilus_model::instruments::{InstrumentAny, stubs::crypto_perpetual_ethusdt};

        let instrument = crypto_perpetual_ethusdt();
        let cache_rc = algo.core.cache_rc();
        let mut cache = cache_rc.borrow_mut();
        cache
            .add_instrument(InstrumentAny::CryptoPerpetual(instrument))
            .unwrap();
    }

    fn params(entries: &[(&str, &str)]) -> IndexMap<Ustr, Ustr> {
        entries
            .iter()
            .map(|(key, value)| (Ustr::from(key), Ustr::from(value)))
            .collect()
    }

    fn sweep_params() -> IndexMap<Ustr, Ustr> {
        params(&[("limit_price", "100.00"), ("max_children", "3")])
    }

    fn sweep_params_with(entries: &[(&str, &str)]) -> IndexMap<Ustr, Ustr> {
        let mut merged = sweep_params();

        for (key, value) in entries {
            merged.insert(Ustr::from(key), Ustr::from(value));
        }

        merged
    }

    fn create_order(params: IndexMap<Ustr, Ustr>, quantity: Quantity, side: OrderSide) -> OrderAny {
        let client_order_id = ClientOrderId::from("O-001");
        OrderAny::Limit(LimitOrder::new(
            TraderId::from("TRADER-001"),
            StrategyId::from("STRAT-001"),
            InstrumentId::from("ETHUSDT-PERP.BINANCE"),
            client_order_id,
            side,
            quantity,
            Price::from("100.00"),
            TimeInForce::Gtc,
            None,  // expire_time
            false, // post_only
            false, // reduce_only
            false, // quote_quantity
            None,  // display_qty
            None,  // emulation_trigger
            None,  // trigger_instrument_id
            None,  // contingency_type
            None,  // order_list_id
            None,  // linked_order_ids
            None,  // parent_order_id
            Some(ExecAlgorithmId::new("SNIPER")),
            Some(params),
            Some(client_order_id),
            None, // tags
            UUID4::new(),
            0.into(),
        ))
    }

    fn create_buy_order(params: IndexMap<Ustr, Ustr>) -> OrderAny {
        create_order(params, Quantity::from("1.000"), OrderSide::Buy)
    }

    fn quote(bid_price: &str, ask_price: &str, bid_size: &str, ask_size: &str) -> QuoteTick {
        QuoteTick::new(
            InstrumentId::from("ETHUSDT-PERP.BINANCE"),
            Price::from(bid_price),
            Price::from(ask_price),
            Quantity::from(bid_size),
            Quantity::from(ask_size),
            UnixNanos::default(),
            UnixNanos::default(),
        )
    }

    fn assert_sniper_denied(algo: &mut SniperAlgorithm, order: &OrderAny, expected_reason: &str) {
        let strategy_id = order.strategy_id();
        {
            let cache_rc = algo.core.cache_rc();
            cache_rc
                .borrow_mut()
                .add_order(order.clone(), None, None, false)
                .unwrap();
        }
        let events = Rc::new(RefCell::new(Vec::new()));
        let handler = TypedHandler::from({
            let events = events.clone();
            move |event: &OrderEventAny| events.borrow_mut().push(event.clone())
        });
        let topic = format!("events.order.{strategy_id}");
        msgbus::subscribe_order_events(topic.clone().into(), handler.clone(), None);

        algo.on_order(order.clone()).unwrap();
        algo.on_order(order.clone()).unwrap();

        msgbus::unsubscribe_order_events(topic.into(), &handler);
        let cached_order = algo.cache().order(&order.client_order_id()).unwrap();
        let events = events.borrow();

        assert_eq!(cached_order.status(), OrderStatus::Denied);
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            OrderEventAny::Denied(event)
                if event.reason == expected_reason
                    && event.strategy_id == strategy_id
                    && event.client_order_id == order.client_order_id()
        ));
        assert!(algo.schedules.is_empty());
        assert!(algo.clock().timer_names().is_empty());
    }

    fn child_id(sequence: u32) -> ClientOrderId {
        ClientOrderId::from(format!("O-001-E{sequence}"))
    }

    fn cached_child(algo: &SniperAlgorithm, sequence: u32) -> OrderAny {
        algo.cache().order(&child_id(sequence)).unwrap()
    }

    fn venue_order_id(order: &OrderAny) -> VenueOrderId {
        VenueOrderId::from(format!("V-{}", order.client_order_id()).as_str())
    }

    /// Applies an order event to the cache and dispatches it, as the engine does.
    fn dispatch(algo: &mut SniperAlgorithm, event: OrderEventAny) {
        {
            let cache_rc = algo.core.cache_rc();
            cache_rc.borrow_mut().update_order(&event).unwrap();
        }

        algo.handle_order_event(event);
    }

    fn accepted_event(order: &OrderAny) -> OrderEventAny {
        OrderEventAny::Accepted(
            OrderAcceptedSpec::builder()
                .trader_id(order.trader_id())
                .strategy_id(order.strategy_id())
                .instrument_id(order.instrument_id())
                .client_order_id(order.client_order_id())
                .venue_order_id(venue_order_id(order))
                .account_id(AccountId::from("SIM-001"))
                .build(),
        )
    }

    fn filled_event(order: &OrderAny, last_qty: Quantity) -> OrderEventAny {
        OrderEventAny::Filled(
            OrderFilledSpec::builder()
                .trader_id(order.trader_id())
                .strategy_id(order.strategy_id())
                .instrument_id(order.instrument_id())
                .client_order_id(order.client_order_id())
                .venue_order_id(venue_order_id(order))
                .account_id(AccountId::from("SIM-001"))
                .order_side(order.order_side())
                .order_type(order.order_type())
                .last_qty(last_qty)
                .last_px(order.price().unwrap_or(Price::from("1000.00")))
                .build(),
        )
    }

    /// Dispatches a child cancellation through the order event path, so the core restores the
    /// primary quantity before the algorithm handles the terminal child.
    fn cancel_child(algo: &mut SniperAlgorithm, sequence: u32) {
        let child = cached_child(algo, sequence);
        let event = OrderEventAny::Canceled(
            OrderCanceledSpec::builder()
                .trader_id(child.trader_id())
                .strategy_id(child.strategy_id())
                .instrument_id(child.instrument_id())
                .client_order_id(child.client_order_id())
                .build(),
        );

        dispatch(algo, event);
    }

    fn deny_child(algo: &mut SniperAlgorithm, sequence: u32, reason: Ustr) {
        let child = cached_child(algo, sequence);
        let event = OrderDenied::new(
            child.trader_id(),
            child.strategy_id(),
            child.instrument_id(),
            child.client_order_id(),
            reason,
            UUID4::new(),
            0.into(),
            0.into(),
        );

        algo.on_order_denied(event);
    }

    fn primary_id() -> ClientOrderId {
        ClientOrderId::from("O-001")
    }

    #[rstest]
    fn test_sniper_creation() {
        let algo = create_sniper_algorithm();

        assert!(algo.id().inner().starts_with("SNIPER"));
        assert!(algo.schedules.is_empty());
        assert_eq!(algo.counts(&primary_id()), None);
    }

    #[rstest]
    fn test_sniper_registration() {
        let mut algo = create_sniper_algorithm();
        register_algorithm(&mut algo);

        assert_eq!(algo.trader_id(), Some(TraderId::from("TRADER-001")));
    }

    #[rstest]
    fn test_sniper_rejects_non_limit_orders() {
        let mut algo = create_sniper_algorithm();
        register_algorithm(&mut algo);
        add_instrument_to_cache(&algo);

        let client_order_id = ClientOrderId::from("O-001");
        let order = OrderAny::Market(nautilus_model::orders::MarketOrder::new(
            TraderId::from("TRADER-001"),
            StrategyId::from("STRAT-001"),
            InstrumentId::from("ETHUSDT-PERP.BINANCE"),
            client_order_id,
            OrderSide::Buy,
            Quantity::from("1.000"),
            TimeInForce::Gtc,
            UUID4::new(),
            0.into(),
            false,
            false,
            None,
            None,
            None,
            None,
            Some(ExecAlgorithmId::new("SNIPER")),
            Some(sweep_params()),
            Some(client_order_id),
            None,
        ));

        let reason = OrderDeniedReason::UnsupportedOrderType {
            order_type: OrderType::Market,
        }
        .to_string();

        assert_sniper_denied(&mut algo, &order, &reason);
    }

    #[rstest]
    fn test_sniper_denies_missing_instrument() {
        let mut algo = create_sniper_algorithm();
        register_algorithm(&mut algo);

        let order = create_buy_order(sweep_params());
        let reason = OrderDeniedReason::InstrumentNotFound {
            instrument_id: order.instrument_id(),
        }
        .to_string();

        assert_sniper_denied(&mut algo, &order, &reason);
    }

    #[rstest]
    fn test_sniper_rejects_missing_params() {
        let mut algo = create_sniper_algorithm();
        register_algorithm(&mut algo);
        add_instrument_to_cache(&algo);

        let client_order_id = ClientOrderId::from("O-001");
        let order = OrderAny::Limit(LimitOrder::new(
            TraderId::from("TRADER-001"),
            StrategyId::from("STRAT-001"),
            InstrumentId::from("ETHUSDT-PERP.BINANCE"),
            client_order_id,
            OrderSide::Buy,
            Quantity::from("1.000"),
            Price::from("100.00"),
            TimeInForce::Gtc,
            None,
            false,
            false,
            false,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            Some(ExecAlgorithmId::new("SNIPER")),
            None,
            Some(client_order_id),
            None,
            UUID4::new(),
            0.into(),
        ));

        assert_sniper_denied(
            &mut algo,
            &order,
            "VALIDATION_FAILED: exec_algorithm_params not found",
        );
    }

    #[rstest]
    #[case::missing_limit_price(
        params(&[("max_children", "3")]),
        "VALIDATION_FAILED: limit_price not found in exec_algorithm_params"
    )]
    #[case::malformed_limit_price(
        params(&[("limit_price", "free"), ("max_children", "3")]),
        "VALIDATION_FAILED: limit_price=free is not a valid price"
    )]
    #[case::missing_max_children(
        params(&[("limit_price", "100.00")]),
        "VALIDATION_FAILED: max_children not found in exec_algorithm_params"
    )]
    #[case::zero_max_children(
        params(&[("limit_price", "100.00"), ("max_children", "0")]),
        "VALIDATION_FAILED: max_children=0 must be positive"
    )]
    #[case::negative_max_children(
        params(&[("limit_price", "100.00"), ("max_children", "-1")]),
        "VALIDATION_FAILED: max_children=-1 is not a valid positive integer"
    )]
    #[case::malformed_max_children(
        params(&[("limit_price", "100.00"), ("max_children", "many")]),
        "VALIDATION_FAILED: max_children=many is not a valid positive integer"
    )]
    fn test_sniper_refuses_missing_or_malformed_parameters(
        #[case] exec_params: IndexMap<Ustr, Ustr>,
        #[case] expected_reason: &str,
    ) {
        let mut algo = create_sniper_algorithm();
        register_algorithm(&mut algo);
        add_instrument_to_cache(&algo);

        let order = create_buy_order(exec_params);

        assert_sniper_denied(&mut algo, &order, expected_reason);
    }

    #[rstest]
    #[case(
        "participation_rate",
        "0.15",
        "VALIDATION_FAILED: participation_rate is not supported by this execution algorithm"
    )]
    #[case(
        "max_slippage_bps",
        "15",
        "VALIDATION_FAILED: max_slippage_bps is not supported by this execution algorithm"
    )]
    #[case(
        "urgency",
        "high",
        "VALIDATION_FAILED: urgency is not supported by this execution algorithm"
    )]
    fn test_sniper_refuses_a_policy_part_it_cannot_honour(
        #[case] key: &str,
        #[case] value: &str,
        #[case] expected_reason: &str,
    ) {
        let mut algo = create_sniper_algorithm();
        register_algorithm(&mut algo);
        add_instrument_to_cache(&algo);

        let order = create_buy_order(sweep_params_with(&[(key, value)]));

        assert_sniper_denied(&mut algo, &order, expected_reason);
    }

    #[rstest]
    fn test_sniper_refuses_a_price_limit_by_name() {
        let mut algo = create_sniper_algorithm();
        register_algorithm(&mut algo);
        add_instrument_to_cache(&algo);

        let order = create_buy_order(sweep_params_with(&[("price_limit", "100.50")]));

        assert_sniper_denied(
            &mut algo,
            &order,
            "VALIDATION_FAILED: price_limit is not supported: the sweep price is this algorithm's \
             own limit_price parameter",
        );
    }

    #[rstest]
    fn test_sniper_refuses_a_passive_preference_by_name() {
        let mut algo = create_sniper_algorithm();
        register_algorithm(&mut algo);
        add_instrument_to_cache(&algo);

        let order = create_buy_order(sweep_params_with(&[("preference", "passive")]));

        assert_sniper_denied(
            &mut algo,
            &order,
            "VALIDATION_FAILED: preference=passive must not be declared: this algorithm crosses \
             the touch deliberately",
        );
    }

    #[rstest]
    fn test_sniper_refuses_a_malformed_policy_field_before_the_part_it_cannot_honour() {
        let mut algo = create_sniper_algorithm();
        register_algorithm(&mut algo);
        add_instrument_to_cache(&algo);

        let order = create_buy_order(sweep_params_with(&[("max_slippage_bps", "-1")]));

        assert_sniper_denied(
            &mut algo,
            &order,
            "VALIDATION_FAILED: max_slippage_bps=-1 must not be negative",
        );
    }

    #[rstest]
    fn test_sniper_accepts_an_aggressive_preference_and_horizon() {
        let mut algo = create_sniper_algorithm();
        register_algorithm(&mut algo);
        add_instrument_to_cache(&algo);

        let order = create_buy_order(sweep_params_with(&[
            ("preference", "aggressive"),
            ("horizon_secs", "60"),
        ]));
        let id = primary_id();

        algo.on_order(order).unwrap();

        let schedule = algo.schedules.get(&id).unwrap();
        assert_eq!(schedule.max_children, 3);
        assert_eq!(schedule.price, Price::from("100.00"));
        assert!(schedule.deadline.is_some());
        assert!(algo.clock().timer_names().contains(&id.to_string()));
    }

    #[rstest]
    fn test_sniper_sends_no_child_while_the_touch_is_unreachable() {
        let mut algo = create_sniper_algorithm();
        register_algorithm(&mut algo);
        add_instrument_to_cache(&algo);

        algo.on_order(create_buy_order(sweep_params())).unwrap();

        // The best ask is above the sweep price, so the touch is not reachable for a buy.
        DataActor::on_quote(&mut algo, &quote("100.60", "100.70", "1.000", "1.000")).unwrap();

        let schedule = algo.schedules.get(&primary_id()).unwrap();
        assert!(schedule.child.is_none());
        assert_eq!(algo.counts(&primary_id()), Some((0, 0)));
        assert!(algo.cache().order(&child_id(1)).is_none());
    }

    #[rstest]
    fn test_sniper_sends_one_child_of_the_displayed_size_when_reachable() {
        let mut algo = create_sniper_algorithm();
        register_algorithm(&mut algo);
        add_instrument_to_cache(&algo);

        algo.on_order(create_buy_order(sweep_params())).unwrap();

        // The best ask is at the sweep price, so the buy is reachable and the ask size is taken.
        DataActor::on_quote(&mut algo, &quote("99.90", "100.00", "0.500", "0.300")).unwrap();

        let child = cached_child(&algo, 1);
        assert_eq!(child.quantity(), Quantity::from("0.300"));
        assert_eq!(child.price(), Some(Price::from("100.00")));
        assert_eq!(child.exec_spawn_id(), Some(primary_id()));
        assert_eq!(algo.working_child(&primary_id()), Some(child_id(1)));
        assert_eq!(algo.counts(&primary_id()), Some((1, 0)));

        let primary = algo.cache().order(&primary_id()).unwrap();
        assert_eq!(primary.quantity(), Quantity::from("0.700"));
    }

    #[rstest]
    fn test_sniper_sweeps_the_bid_for_a_sell() {
        let mut algo = create_sniper_algorithm();
        register_algorithm(&mut algo);
        add_instrument_to_cache(&algo);

        let order = create_order(sweep_params(), Quantity::from("1.000"), OrderSide::Sell);
        algo.on_order(order).unwrap();

        // The best bid is below the sweep price, so the sell is not reachable.
        DataActor::on_quote(&mut algo, &quote("99.50", "99.60", "0.400", "0.400")).unwrap();
        assert!(algo.working_child(&primary_id()).is_none());

        // The best bid rises to the sweep price, so the sell is reachable and the bid size is taken.
        DataActor::on_quote(&mut algo, &quote("100.50", "100.60", "0.400", "0.400")).unwrap();

        let child = cached_child(&algo, 1);
        assert_eq!(child.order_side(), OrderSide::Sell);
        assert_eq!(child.quantity(), Quantity::from("0.400"));
        assert_eq!(child.price(), Some(Price::from("100.00")));
    }

    #[rstest]
    fn test_sniper_floors_the_displayed_size_to_the_instrument_precision() {
        let mut algo = create_sniper_algorithm();
        register_algorithm(&mut algo);
        add_instrument_to_cache(&algo);

        algo.on_order(create_buy_order(sweep_params())).unwrap();

        DataActor::on_quote(&mut algo, &quote("99.90", "100.00", "0.5000", "0.3004")).unwrap();

        let child = cached_child(&algo, 1);
        let instrument = nautilus_model::instruments::stubs::crypto_perpetual_ethusdt();
        assert_eq!(child.quantity(), Quantity::from("0.300"));
        assert_eq!(child.quantity().precision, instrument.size_precision());
    }

    #[rstest]
    fn test_sniper_sends_nothing_when_the_displayed_size_cannot_form_a_child() {
        let mut algo = create_sniper_algorithm();
        register_algorithm(&mut algo);
        add_instrument_to_cache(&algo);

        algo.on_order(create_buy_order(sweep_params())).unwrap();

        // The reachable ask size floors to zero at the instrument precision, so no child is formed.
        DataActor::on_quote(&mut algo, &quote("99.90", "100.00", "0.5000", "0.0004")).unwrap();

        assert!(algo.working_child(&primary_id()).is_none());
        assert_eq!(algo.counts(&primary_id()), Some((0, 0)));
        assert!(algo.cache().order(&child_id(1)).is_none());
    }

    #[rstest]
    fn test_sniper_caps_the_child_by_the_remaining_primary_quantity() {
        let mut algo = create_sniper_algorithm();
        register_algorithm(&mut algo);
        add_instrument_to_cache(&algo);

        algo.on_order(create_buy_order(sweep_params())).unwrap();

        // The displayed size exceeds the primary, so the child is capped by the remaining quantity.
        DataActor::on_quote(&mut algo, &quote("99.90", "100.00", "5.000", "5.000")).unwrap();

        let child = cached_child(&algo, 1);
        assert_eq!(child.quantity(), Quantity::from("1.000"));
        assert_eq!(
            algo.cache().order(&primary_id()).unwrap().quantity(),
            Quantity::from("0.000")
        );
        assert_eq!(algo.counts(&primary_id()), Some((1, 0)));
    }

    #[rstest]
    fn test_sniper_sends_no_more_than_max_children_and_completes() {
        let mut algo = create_sniper_algorithm();
        register_algorithm(&mut algo);
        add_instrument_to_cache(&algo);

        let order = create_buy_order(sweep_params_with(&[("max_children", "2")]));
        algo.on_order(order).unwrap();

        DataActor::on_quote(&mut algo, &quote("99.90", "100.00", "0.100", "0.100")).unwrap();
        assert_eq!(algo.counts(&primary_id()), Some((1, 0)));

        cancel_child(&mut algo, 1);
        assert_eq!(algo.counts(&primary_id()), Some((2, 1)));
        assert_eq!(algo.working_child(&primary_id()), Some(child_id(2)));

        // The second child is terminal at the bound with quantity remaining, so the sequence
        // completes rather than sending a third child.
        cancel_child(&mut algo, 2);

        assert!(algo.schedules.get(&primary_id()).is_none());
        assert_eq!(algo.counts(&primary_id()), None);
        assert!(algo.clock().timer_names().is_empty());
        assert!(algo.cache().order(&child_id(2)).is_some());
        assert!(algo.cache().order(&child_id(3)).is_none());
    }

    #[rstest]
    fn test_sniper_a_terminal_child_permits_exactly_one_next_child() {
        let mut algo = create_sniper_algorithm();
        register_algorithm(&mut algo);
        add_instrument_to_cache(&algo);

        let order = create_buy_order(sweep_params_with(&[("max_children", "5")]));
        algo.on_order(order).unwrap();

        DataActor::on_quote(&mut algo, &quote("99.90", "100.00", "0.100", "0.100")).unwrap();
        assert_eq!(algo.counts(&primary_id()), Some((1, 0)));

        cancel_child(&mut algo, 1);
        assert_eq!(algo.counts(&primary_id()), Some((2, 1)));
        assert_eq!(algo.working_child(&primary_id()), Some(child_id(2)));

        // Further reachable quotes while the second child works send nothing more.
        for _ in 0..3 {
            DataActor::on_quote(&mut algo, &quote("99.80", "99.90", "0.200", "0.200")).unwrap();
        }

        assert_eq!(algo.counts(&primary_id()), Some((2, 1)));
        assert_eq!(algo.working_child(&primary_id()), Some(child_id(2)));
        assert!(algo.cache().order(&child_id(3)).is_none());
    }

    #[rstest]
    fn test_sniper_a_denied_child_completes_and_denies_the_primary() {
        let mut algo = create_sniper_algorithm();
        register_algorithm(&mut algo);
        add_instrument_to_cache(&algo);

        let order = create_buy_order(sweep_params_with(&[("max_children", "5")]));
        algo.on_order(order).unwrap();

        DataActor::on_quote(&mut algo, &quote("99.90", "100.00", "0.100", "0.100")).unwrap();
        assert_eq!(algo.counts(&primary_id()), Some((1, 0)));

        let events = Rc::new(RefCell::new(Vec::new()));
        let handler = TypedHandler::from({
            let events = events.clone();
            move |event: &OrderEventAny| events.borrow_mut().push(event.clone())
        });
        let topic = "events.order.STRAT-001".to_string();
        msgbus::subscribe_order_events(topic.clone().into(), handler.clone(), None);

        let reason = Ustr::from("INSUFFICIENT_MARGIN");
        deny_child(&mut algo, 1, reason);

        msgbus::unsubscribe_order_events(topic.into(), &handler);
        let events = events.borrow();

        // A refused child is not retried: the primary carries the child's reason and the sequence
        // completes.
        assert!(events.iter().any(|event| matches!(
            event,
            OrderEventAny::Denied(denied)
                if denied.client_order_id == primary_id() && denied.reason == reason
        )));
        assert!(algo.schedules.get(&primary_id()).is_none());
        assert_eq!(algo.counts(&primary_id()), None);
        assert!(algo.clock().timer_names().is_empty());
        assert!(algo.cache().order(&child_id(2)).is_none());
    }

    #[rstest]
    fn test_sniper_a_touch_that_moves_every_tick_sends_one_child_per_terminal_event() {
        let mut algo = create_sniper_algorithm();
        register_algorithm(&mut algo);
        add_instrument_to_cache(&algo);

        let order = create_buy_order(sweep_params_with(&[("max_children", "5")]));
        algo.on_order(order).unwrap();

        DataActor::on_quote(&mut algo, &quote("99.90", "100.00", "0.100", "0.100")).unwrap();
        assert_eq!(algo.counts(&primary_id()), Some((1, 0)));

        // A touch that moves on every tick must not add a child while one is already working.
        for step in 1..=4 {
            let ask = format!("99.{step:02}");
            DataActor::on_quote(&mut algo, &quote("99.00", &ask, "0.100", "0.100")).unwrap();
        }
        assert_eq!(algo.counts(&primary_id()), Some((1, 0)));

        // The terminal child permits exactly one successor, and the moving touch adds nothing.
        cancel_child(&mut algo, 1);
        assert_eq!(algo.counts(&primary_id()), Some((2, 1)));

        for step in 1..=4 {
            let ask = format!("99.{step:02}");
            DataActor::on_quote(&mut algo, &quote("99.00", &ask, "0.100", "0.100")).unwrap();
        }

        assert_eq!(algo.counts(&primary_id()), Some((2, 1)));
        assert_eq!(algo.working_child(&primary_id()), Some(child_id(2)));
        assert!(algo.cache().order(&child_id(3)).is_none());
    }

    #[rstest]
    fn test_sniper_deadline_cancels_the_working_child_and_completes() {
        let mut algo = create_sniper_algorithm();
        register_algorithm(&mut algo);
        add_instrument_to_cache(&algo);

        let order = create_buy_order(sweep_params_with(&[("horizon_secs", "60")]));
        let id = primary_id();
        algo.on_order(order).unwrap();
        assert!(algo.clock().timer_names().contains(&id.to_string()));

        DataActor::on_quote(&mut algo, &quote("99.90", "100.00", "0.100", "0.100")).unwrap();
        assert_eq!(algo.working_child(&id), Some(child_id(1)));

        let event = TimeEvent::new(id.inner(), UUID4::new(), 0.into(), 0.into());
        ExecutionAlgorithm::on_time_event(&mut algo, &event).unwrap();

        assert!(algo.schedules.get(&id).is_none());
        assert!(algo.clock().timer_names().is_empty());

        // A quote arriving after completion sends nothing.
        DataActor::on_quote(&mut algo, &quote("99.90", "100.00", "0.100", "0.100")).unwrap();
        assert!(algo.cache().order(&child_id(2)).is_none());
    }

    #[rstest]
    fn test_sniper_completes_when_the_remaining_quantity_is_zero() {
        let mut algo = create_sniper_algorithm();
        register_algorithm(&mut algo);
        add_instrument_to_cache(&algo);

        let order = create_order(sweep_params(), Quantity::from("0.300"), OrderSide::Buy);
        algo.on_order(order).unwrap();

        DataActor::on_quote(&mut algo, &quote("99.90", "100.00", "0.500", "0.500")).unwrap();
        assert_eq!(algo.counts(&primary_id()), Some((1, 0)));
        assert_eq!(
            algo.cache().order(&primary_id()).unwrap().quantity(),
            Quantity::from("0.000")
        );

        // The child fills the whole primary, so the sequence completes on the terminal fill.
        let child = cached_child(&algo, 1);
        dispatch(&mut algo, accepted_event(&child));
        dispatch(&mut algo, filled_event(&child, Quantity::from("0.300")));

        assert!(algo.schedules.get(&primary_id()).is_none());
        assert_eq!(algo.counts(&primary_id()), None);
        assert!(algo.clock().timer_names().is_empty());
    }

    #[rstest]
    fn test_sniper_counts_reports_submitted_and_cancelled() {
        let mut algo = create_sniper_algorithm();
        register_algorithm(&mut algo);
        add_instrument_to_cache(&algo);

        let order = create_buy_order(sweep_params_with(&[("max_children", "5")]));
        algo.on_order(order).unwrap();

        DataActor::on_quote(&mut algo, &quote("99.90", "100.00", "0.100", "0.100")).unwrap();
        assert_eq!(algo.counts(&primary_id()), Some((1, 0)));

        // The touch moves away, so the cancelled child is not immediately replaced.
        DataActor::on_quote(&mut algo, &quote("100.60", "100.70", "0.100", "0.100")).unwrap();
        cancel_child(&mut algo, 1);

        assert_eq!(algo.counts(&primary_id()), Some((1, 1)));
        assert!(algo.schedules.contains_key(&primary_id()));

        // The touch returns, so the next child is sent.
        DataActor::on_quote(&mut algo, &quote("99.90", "100.00", "0.100", "0.100")).unwrap();

        assert_eq!(algo.counts(&primary_id()), Some((2, 1)));
    }

    #[rstest]
    fn test_sniper_reset_clears_schedules() {
        let mut algo = create_sniper_algorithm();
        algo.schedules.insert(
            primary_id(),
            SniperSchedule {
                instrument_id: InstrumentId::from("ETHUSDT-PERP.BINANCE"),
                side: OrderSide::Buy,
                price: Price::from("100.00"),
                max_children: 3,
                deadline: None,
                child: None,
                touch: None,
                submitted: 0,
                cancelled: 0,
            },
        );

        assert!(!algo.schedules.is_empty());

        // Dispatch through the DataActor entry point the component lifecycle uses
        DataActor::on_reset(&mut algo).unwrap();

        assert!(algo.schedules.is_empty());
    }
}
