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

//! Iceberg execution algorithm.
//!
//! An iceberg order hides its size by keeping one child order working at a time: the next child is
//! sent when the previous one is terminal, for `min(remaining, display_size)` floored to the
//! instrument's size precision and never below its minimum quantity. When the market crosses the
//! working child's price the child is assumed to have been missed, so it is cancelled and
//! re-quoted, but only once `requote_secs` has elapsed since the last send.
//!
//! # Parameters
//!
//! Orders submitted to this algorithm must include `exec_algorithm_params` with:
//! - `display_size`: The quantity each child order is quoted for.
//! - `requote_secs`: The minimum interval between a cancel and its re-quote, in seconds.
//!
//! The horizon of the execution policy is honoured as the sequence deadline. A declared price
//! limit is honoured by refusing an order whose price already breaches it, and a passive
//! preference is what the algorithm does already. The other policy parts are refused with a denial
//! naming the field, as is an aggressive preference, which an order that rests cannot satisfy.

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
    events::{OrderCanceled, OrderDenied, OrderDeniedReason, OrderFilled, OrderRejected},
    identifiers::{ClientOrderId, InstrumentId},
    instruments::{Instrument, InstrumentAny},
    orders::{Order, OrderAny},
    types::{Price, Quantity},
};
use rust_decimal::RoundingStrategy;
use ustr::Ustr;

use super::{
    ExecutionAlgorithm, ExecutionAlgorithmConfig, ExecutionAlgorithmCore, ExecutionAlgorithmNative,
    ExecutionIntent, ExecutionPolicy, ExecutionPreference, PolicyError, PolicyPart,
};
use crate::nautilus_execution_algorithm;

/// Configuration for [`IcebergAlgorithm`].
pub type IcebergAlgorithmConfig = ExecutionAlgorithmConfig;

/// The policy parts iceberg can honour.
///
/// The horizon becomes the sequence deadline, a price limit is checked against the order's own
/// price, and a passive preference is what the algorithm does already. A participation rate, a
/// slippage cap and an urgency would each have to change how the children are placed, which this
/// algorithm does not do.
static ICEBERG_SUPPORTED_POLICY_PARTS: [PolicyPart; 3] = [
    PolicyPart::Horizon,
    PolicyPart::PriceLimit,
    PolicyPart::Preference,
];

/// The parameter key declaring the quantity each child is quoted for.
const KEY_DISPLAY_SIZE: &str = "display_size";
/// The parameter key declaring the minimum interval between a cancel and its re-quote.
const KEY_REQUOTE_SECS: &str = "requote_secs";

/// Iceberg execution algorithm.
///
/// Keeps at most one child order working per primary order, sending the next child when the
/// previous one is terminal and re-quoting a child the market has crossed once the re-quote
/// interval has elapsed.
#[derive(Debug)]
pub struct IcebergAlgorithm {
    /// The algorithm core.
    pub core: ExecutionAlgorithmCore,
    /// The scheduling state of each primary order.
    schedules: AHashMap<ClientOrderId, IcebergSchedule>,
}

impl IcebergAlgorithm {
    /// Creates a new [`IcebergAlgorithm`] instance.
    #[must_use]
    pub fn new(config: IcebergAlgorithmConfig) -> Self {
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

        log::info!("Completed iceberg execution for {primary_id}");
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

    /// Sends the next child for a primary order, when it has quantity remaining.
    fn send_next_child(&mut self, primary_id: ClientOrderId) {
        let (display_size, price) = {
            let Some(schedule) = self.schedules.get(&primary_id) else {
                return;
            };

            (schedule.display_size, schedule.price)
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

        let quantity = if remaining < display_size {
            remaining
        } else {
            display_size
        };
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
            log::error!("Failed to submit the iceberg child {child_id}: {e}");
            self.complete_sequence(primary_id);
            return;
        }

        let now = ExecutionAlgorithmNative::exec_algorithm_core_mut(self)
            .clock_mut()
            .timestamp_ns();

        if let Some(schedule) = self.schedules.get_mut(&primary_id) {
            schedule.child = Some(child_id);
            schedule.last_send = Some(now);
            schedule.submitted += 1;
        }

        log::info!("Iceberg child {child_id} submitted for {quantity} at {price}");
    }

    /// Handles a child order that has reached a terminal state.
    ///
    /// The unfilled quantity of a cancelled or denied child is restored to the primary order by
    /// the core, so the primary's own quantity is the quantity still to execute.
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

    /// Cancels the working child when the market has crossed its price and the re-quote interval
    /// has elapsed since it was sent.
    fn handle_quote(&mut self, quote: &QuoteTick) {
        let primary_ids: Vec<ClientOrderId> = self.schedules.keys().copied().collect();

        for primary_id in primary_ids {
            let (instrument_id, child_id, price, requote_nanos, last_send) = {
                let Some(schedule) = self.schedules.get(&primary_id) else {
                    continue;
                };
                let Some(child_id) = schedule.child else {
                    continue;
                };

                (
                    schedule.instrument_id,
                    child_id,
                    schedule.price,
                    schedule.requote_nanos,
                    schedule.last_send,
                )
            };

            if quote.instrument_id != instrument_id {
                continue;
            }

            let Some(child) = self.cached_order(child_id) else {
                continue;
            };

            let crossed = match child.order_side() {
                OrderSide::Buy => quote.ask_price <= price,
                OrderSide::Sell => quote.bid_price >= price,
            };
            if !crossed {
                continue;
            }

            let now = ExecutionAlgorithmNative::exec_algorithm_core_mut(self)
                .clock_mut()
                .timestamp_ns();
            let elapsed = now
                .as_u64()
                .saturating_sub(last_send.map_or(0, |last_send| last_send.as_u64()));

            if elapsed < requote_nanos {
                // The re-quote interval has not elapsed, so the child is left working and the next
                // quote re-checks it.
                continue;
            }

            let mut child = child;
            match self.cancel_order(&mut child, None) {
                Ok(()) => log::info!("Cancelled iceberg child {child_id} on a crossed market"),
                Err(e) => log::error!("Failed to cancel iceberg child {child_id}: {e}"),
            }
        }
    }
}

// The clock and component lifecycle dispatch through the `DataActor` hooks,
// so forward them to the `ExecutionAlgorithm` implementations.
impl DataActor for IcebergAlgorithm {
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

nautilus_execution_algorithm!(IcebergAlgorithm, {
    fn supported_policy_parts(&self) -> &'static [PolicyPart] {
        &ICEBERG_SUPPORTED_POLICY_PARTS
    }

    fn on_order(&mut self, order: OrderAny) -> anyhow::Result<()> {
        let primary_id = order.client_order_id();

        if self.schedules.contains_key(&primary_id) {
            anyhow::bail!("Order {primary_id} already being executed");
        }

        log::info!("Received order for iceberg execution: {order:?}");

        // An iceberg rests, so it is placed by a limit order.
        if order.order_type() != OrderType::Limit {
            let reason = OrderDeniedReason::UnsupportedOrderType {
                order_type: order.order_type(),
            }
            .to_string();
            return self.deny_order(&order, Ustr::from(&reason));
        }

        let Some(price) = order.price() else {
            return self.deny_order(&order, validation_failed("limit price not found"));
        };

        let instrument = {
            let cache = ExecutionAlgorithmNative::exec_algorithm_core(self).cache_ref();
            cache.instrument(&order.instrument_id()).cloned()
        };

        let Some(instrument) = instrument else {
            let reason = OrderDeniedReason::InstrumentNotFound {
                instrument_id: order.instrument_id(),
            }
            .to_string();
            return self.deny_order(&order, Ustr::from(&reason));
        };

        let Some(exec_params) = order.exec_algorithm_params() else {
            return self.deny_order(&order, validation_failed("exec_algorithm_params not found"));
        };

        let policy = match ExecutionPolicy::from_params(exec_params) {
            Ok(policy) => policy,
            Err(err) => return self.deny_order(&order, validation_failed(err.message())),
        };

        if let Some(part) = policy.unsupported(self.supported_policy_parts()) {
            return self.deny_order(
                &order,
                validation_failed(PolicyError::unsupported(part).message()),
            );
        }

        if let Err(err) = policy.validate(&ExecutionIntent::from(&order)) {
            return self.deny_order(&order, validation_failed(err.message()));
        }

        if policy.preference == Some(ExecutionPreference::Aggressive) {
            return self.deny_order(
                &order,
                validation_failed("preference=aggressive is not possible for an order that rests"),
            );
        }

        if let Some(limit) = policy.price_limit
            && !within_price_limit(order.order_side(), price, limit)
        {
            return self.deny_order(
                &order,
                validation_failed(format!(
                    "price_limit={limit} is breached by the order price {price}"
                )),
            );
        }

        let display_size = match parse_display_size(exec_params, &instrument) {
            Ok(display_size) => display_size,
            Err(reason) => return self.deny_order(&order, reason),
        };

        let requote_nanos = match parse_requote_secs(exec_params) {
            Ok(requote_nanos) => requote_nanos,
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
        self.schedules.insert(
            primary_id,
            IcebergSchedule {
                instrument_id,
                price,
                display_size,
                requote_nanos,
                deadline,
                child: None,
                last_send: None,
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

        self.send_next_child(primary_id);

        log::info!("Started iceberg execution for {primary_id}");

        Ok(())
    }

    fn on_time_event(&mut self, event: &TimeEvent) -> anyhow::Result<()> {
        let primary_id = ClientOrderId::new(event.name);

        let Some(child_id) = self.working_child(&primary_id) else {
            if self.schedules.contains_key(&primary_id) {
                log::info!("Iceberg deadline reached for {primary_id}");
                self.complete_sequence(primary_id);
            }
            return Ok(());
        };

        log::info!("Iceberg deadline reached for {primary_id}, cancelling child {child_id}");

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

    fn on_order_denied(&mut self, event: OrderDenied) {
        self.abort_on_child_refusal(event.client_order_id, event.reason);
    }

    fn on_order_rejected(&mut self, event: OrderRejected) {
        let reason = OrderDeniedReason::SubmitFailed {
            detail: format!(
                "the iceberg child {} was rejected by the venue: {}",
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

impl IcebergAlgorithm {
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
struct IcebergSchedule {
    /// The instrument the primary order executes in.
    instrument_id: InstrumentId,
    /// The price each child is quoted at.
    price: Price,
    /// The quantity each child is quoted for, already floored to the instrument precision.
    display_size: Quantity,
    /// The minimum interval between a cancel and its re-quote, in nanoseconds.
    requote_nanos: u64,
    /// When the sequence must be complete, from the policy horizon.
    deadline: Option<UnixNanos>,
    /// The working child, if one is working.
    child: Option<ClientOrderId>,
    /// When the last child was sent.
    last_send: Option<UnixNanos>,
    /// The children submitted, for the churn evidence.
    submitted: u32,
    /// The children cancelled, for the churn evidence.
    cancelled: u32,
}

/// Returns whether `price` respects the execution price limit for `order_side`.
fn within_price_limit(order_side: OrderSide, price: Price, limit: Price) -> bool {
    match order_side {
        OrderSide::Sell => price >= limit,
        _ => price <= limit,
    }
}

fn validation_failed(detail: impl Into<String>) -> Ustr {
    let reason = OrderDeniedReason::ValidationFailed {
        detail: detail.into(),
    };

    Ustr::from(&reason.to_string())
}

/// Returns the display size declared in `params`, floored to the instrument precision.
fn parse_display_size(
    params: &IndexMap<Ustr, Ustr>,
    instrument: &InstrumentAny,
) -> Result<Quantity, Ustr> {
    let Some(raw) = params.get(&Ustr::from(KEY_DISPLAY_SIZE)) else {
        return Err(validation_failed(
            "display_size not found in exec_algorithm_params",
        ));
    };

    let Ok(display_size) = raw.as_str().parse::<Quantity>() else {
        return Err(validation_failed(format!(
            "display_size={raw} is not a valid quantity"
        )));
    };

    if display_size.is_zero() {
        return Err(validation_failed("display_size=0 must be positive"));
    }

    let floored = display_size.as_decimal().round_dp_with_strategy(
        u32::from(instrument.size_precision()),
        RoundingStrategy::ToZero,
    );

    let floored = match instrument.try_make_qty_from_decimal(floored, None) {
        Ok(quantity) => quantity,
        Err(e) => {
            return Err(validation_failed(format!(
                "invalid display_size={display_size}: {e}"
            )));
        }
    };

    if floored < instrument.size_increment() {
        return Err(validation_failed(format!(
            "display_size={display_size} is below the instrument size increment {}",
            instrument.size_increment()
        )));
    }

    if let Some(min_qty) = instrument.min_quantity()
        && floored < min_qty
    {
        return Err(validation_failed(format!(
            "display_size={display_size} is below the instrument minimum quantity {min_qty}"
        )));
    }

    Ok(floored)
}

/// Returns the re-quote interval declared in `params`, in nanoseconds.
fn parse_requote_secs(params: &IndexMap<Ustr, Ustr>) -> Result<u64, Ustr> {
    let Some(raw) = params.get(&Ustr::from(KEY_REQUOTE_SECS)) else {
        return Err(validation_failed(
            "requote_secs not found in exec_algorithm_params",
        ));
    };

    let Ok(requote_secs) = raw.as_str().parse::<f64>() else {
        return Err(validation_failed(format!(
            "requote_secs={raw} is not a valid number"
        )));
    };

    if !requote_secs.is_finite() || requote_secs <= 0.0 {
        return Err(validation_failed(format!(
            "requote_secs={requote_secs} must be finite and positive"
        )));
    }

    let Ok(duration) = Duration::try_from_secs_f64(requote_secs) else {
        return Err(validation_failed(format!(
            "requote_secs={requote_secs} is not a valid duration"
        )));
    };

    if duration == Duration::ZERO {
        return Err(validation_failed(format!(
            "requote_secs={requote_secs} rounds to a zero duration"
        )));
    }

    Ok(duration.as_nanos() as u64)
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
        messages::execution::{CancelOrder, TradingCommand},
        msgbus::{self, MessagingSwitchboard, TypedHandler},
        timer::TimeEvent,
    };
    use nautilus_core::UUID4;
    use nautilus_model::{
        data::QuoteTick,
        enums::{OrderStatus, TimeInForce},
        events::{
            OrderDeniedReason, OrderEventAny,
            order::spec::{OrderAcceptedSpec, OrderCanceledSpec, OrderDeniedSpec, OrderFilledSpec},
        },
        identifiers::{
            AccountId, ClientOrderId, ExecAlgorithmId, InstrumentId, StrategyId, TraderId,
            VenueOrderId,
        },
        orders::{LimitOrder, MarketOrder},
    };
    use rstest::rstest;

    use super::*;

    fn create_iceberg_algorithm() -> IcebergAlgorithm {
        // Use a unique ID to avoid thread-local registry conflicts in parallel tests.
        let unique_id = format!("ICEBERG-{}", UUID4::new());
        let config = IcebergAlgorithmConfig {
            exec_algorithm_id: Some(ExecAlgorithmId::new(&unique_id)),
            ..Default::default()
        };
        IcebergAlgorithm::new(config)
    }

    fn register_algorithm_with_clock(algo: &mut IcebergAlgorithm) -> Rc<RefCell<VirtualClock>> {
        use nautilus_common::timer::TimeEventCallback;

        let trader_id = TraderId::from("TRADER-001");
        let clock = Rc::new(RefCell::new(VirtualClock::new()));
        let cache = Rc::new(RefCell::new(Cache::default()));

        clock
            .borrow_mut()
            .register_default_handler(TimeEventCallback::Rust(std::sync::Arc::new(|_| {})));

        algo.core.register(trader_id, clock.clone(), cache).unwrap();

        algo.transition_state(ComponentTrigger::Initialize).unwrap();
        algo.transition_state(ComponentTrigger::Start).unwrap();
        algo.transition_state(ComponentTrigger::StartCompleted)
            .unwrap();

        clock
    }

    fn register_algorithm(algo: &mut IcebergAlgorithm) {
        let _ = register_algorithm_with_clock(algo);
    }

    fn add_instrument_to_cache(algo: &IcebergAlgorithm) {
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

    fn schedule_params(display_size: &str) -> IndexMap<Ustr, Ustr> {
        params(&[(KEY_DISPLAY_SIZE, display_size), (KEY_REQUOTE_SECS, "5")])
    }

    fn create_limit_order_with_params(
        algo: &IcebergAlgorithm,
        exec_params: IndexMap<Ustr, Ustr>,
        quantity: Quantity,
        price: Price,
    ) -> OrderAny {
        let client_order_id = ClientOrderId::from("O-001");

        OrderAny::Limit(LimitOrder::new(
            TraderId::from("TRADER-001"),
            StrategyId::from("STRAT-001"),
            InstrumentId::from("ETHUSDT-PERP.BINANCE"),
            client_order_id,
            OrderSide::Buy,
            quantity,
            price,
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
            Some(algo.id()),
            Some(exec_params),
            Some(client_order_id),
            None,
            UUID4::new(),
            0.into(),
        ))
    }

    fn create_market_order_with_params(
        algo: &IcebergAlgorithm,
        exec_params: IndexMap<Ustr, Ustr>,
    ) -> OrderAny {
        let client_order_id = ClientOrderId::from("O-001");

        OrderAny::Market(MarketOrder::new(
            TraderId::from("TRADER-001"),
            StrategyId::from("STRAT-001"),
            InstrumentId::from("ETHUSDT-PERP.BINANCE"),
            client_order_id,
            OrderSide::Buy,
            Quantity::from("10"),
            TimeInForce::Gtc,
            UUID4::new(),
            0.into(),
            false,
            false,
            None,
            None,
            None,
            None,
            Some(algo.id()),
            Some(exec_params),
            Some(client_order_id),
            None,
        ))
    }

    fn assert_iceberg_denied(algo: &mut IcebergAlgorithm, order: &OrderAny, expected_reason: &str) {
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

    fn venue_order_id(order: &OrderAny) -> VenueOrderId {
        VenueOrderId::from(format!("V-{}", order.client_order_id()).as_str())
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
                .maybe_position_id(None)
                .maybe_commission(None)
                .maybe_info(None)
                .build(),
        )
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

    fn canceled_event(order: &OrderAny) -> OrderEventAny {
        OrderEventAny::Canceled(
            OrderCanceledSpec::builder()
                .trader_id(order.trader_id())
                .strategy_id(order.strategy_id())
                .instrument_id(order.instrument_id())
                .client_order_id(order.client_order_id())
                .venue_order_id(venue_order_id(order))
                .account_id(AccountId::from("SIM-001"))
                .build(),
        )
    }

    fn child(algo: &IcebergAlgorithm, client_order_id: &str) -> OrderAny {
        algo.cache()
            .order(&ClientOrderId::from(client_order_id))
            .unwrap()
    }

    /// Applies an order event to the cache and dispatches it, as the engine does.
    fn dispatch(algo: &mut IcebergAlgorithm, event: OrderEventAny) {
        {
            let cache_rc = algo.core.cache_rc();
            cache_rc.borrow_mut().update_order(&event).unwrap();
        }

        algo.handle_order_event(event);
    }

    #[rstest]
    fn test_iceberg_rejects_non_limit_orders() {
        let mut algo = create_iceberg_algorithm();
        register_algorithm(&mut algo);

        add_instrument_to_cache(&algo);

        let order = create_market_order_with_params(&algo, schedule_params("4"));

        assert_iceberg_denied(
            &mut algo,
            &order,
            &format!(
                "{}",
                OrderDeniedReason::UnsupportedOrderType {
                    order_type: OrderType::Market
                }
            ),
        );
    }

    #[rstest]
    #[case(&[], "VALIDATION_FAILED: display_size not found in exec_algorithm_params")]
    #[case(
        &[(KEY_DISPLAY_SIZE, "abc")],
        "VALIDATION_FAILED: display_size=abc is not a valid quantity"
    )]
    #[case(
        &[(KEY_DISPLAY_SIZE, "0")],
        "VALIDATION_FAILED: display_size=0 must be positive"
    )]
    fn test_iceberg_denies_a_missing_or_malformed_display_size(
        #[case] entries: &[(&str, &str)],
        #[case] expected_reason: &str,
    ) {
        let mut algo = create_iceberg_algorithm();
        register_algorithm(&mut algo);

        add_instrument_to_cache(&algo);

        let mut exec_params = params(&[(KEY_REQUOTE_SECS, "5")]);
        for (key, value) in entries {
            exec_params.insert(Ustr::from(key), Ustr::from(value));
        }

        let order = create_limit_order_with_params(
            &algo,
            exec_params,
            Quantity::from("10"),
            Price::from("1000.00"),
        );

        assert_iceberg_denied(&mut algo, &order, expected_reason);
    }

    #[rstest]
    #[case(
        &[],
        "VALIDATION_FAILED: requote_secs not found in exec_algorithm_params"
    )]
    #[case(
        &[(KEY_REQUOTE_SECS, "soon")],
        "VALIDATION_FAILED: requote_secs=soon is not a valid number"
    )]
    #[case(
        &[(KEY_REQUOTE_SECS, "0")],
        "VALIDATION_FAILED: requote_secs=0 must be finite and positive"
    )]
    fn test_iceberg_denies_a_missing_or_malformed_requote_secs(
        #[case] entries: &[(&str, &str)],
        #[case] expected_reason: &str,
    ) {
        let mut algo = create_iceberg_algorithm();
        register_algorithm(&mut algo);

        add_instrument_to_cache(&algo);

        let mut exec_params = params(&[(KEY_DISPLAY_SIZE, "4")]);
        for (key, value) in entries {
            exec_params.insert(Ustr::from(key), Ustr::from(value));
        }

        let order = create_limit_order_with_params(
            &algo,
            exec_params,
            Quantity::from("10"),
            Price::from("1000.00"),
        );

        assert_iceberg_denied(&mut algo, &order, expected_reason);
    }

    #[rstest]
    #[case("participation_rate", "0.5")]
    #[case("max_slippage_bps", "15")]
    #[case("urgency", "high")]
    fn test_iceberg_refuses_a_policy_part_it_cannot_honour(#[case] key: &str, #[case] value: &str) {
        let mut algo = create_iceberg_algorithm();
        register_algorithm(&mut algo);

        add_instrument_to_cache(&algo);

        let mut exec_params = schedule_params("4");
        exec_params.insert(Ustr::from(key), Ustr::from(value));

        let order = create_limit_order_with_params(
            &algo,
            exec_params,
            Quantity::from("10"),
            Price::from("1000.00"),
        );

        assert_iceberg_denied(
            &mut algo,
            &order,
            &format!("VALIDATION_FAILED: {key} is not supported by this execution algorithm"),
        );
    }

    #[rstest]
    fn test_iceberg_refuses_an_aggressive_preference() {
        let mut algo = create_iceberg_algorithm();
        register_algorithm(&mut algo);

        add_instrument_to_cache(&algo);

        let mut exec_params = schedule_params("4");
        exec_params.insert(Ustr::from("preference"), Ustr::from("aggressive"));

        let order = create_limit_order_with_params(
            &algo,
            exec_params,
            Quantity::from("10"),
            Price::from("1000.00"),
        );

        assert_iceberg_denied(
            &mut algo,
            &order,
            "VALIDATION_FAILED: preference=aggressive is not possible for an order that rests",
        );
    }

    #[rstest]
    fn test_iceberg_refuses_an_order_price_that_breaches_the_price_limit() {
        let mut algo = create_iceberg_algorithm();
        register_algorithm(&mut algo);

        add_instrument_to_cache(&algo);

        let mut exec_params = schedule_params("4");
        exec_params.insert(Ustr::from("price_limit"), Ustr::from("999"));

        let price = Price::from("1000.00");
        let limit = Price::from("999");
        let order = create_limit_order_with_params(&algo, exec_params, Quantity::from("10"), price);

        assert_iceberg_denied(
            &mut algo,
            &order,
            &format!(
                "VALIDATION_FAILED: price_limit={limit} is breached by the order price {price}"
            ),
        );
    }

    #[rstest]
    fn test_iceberg_accepts_a_passive_preference_and_a_price_limit() {
        let mut algo = create_iceberg_algorithm();
        register_algorithm(&mut algo);

        add_instrument_to_cache(&algo);

        let mut exec_params = schedule_params("4");
        exec_params.insert(Ustr::from("preference"), Ustr::from("passive"));
        exec_params.insert(Ustr::from("price_limit"), Ustr::from("1000.00"));
        exec_params.insert(Ustr::from("horizon_secs"), Ustr::from("60"));

        let order = create_limit_order_with_params(
            &algo,
            exec_params,
            Quantity::from("10"),
            Price::from("1000.00"),
        );
        let primary_id = order.client_order_id();

        algo.on_order(order).unwrap();

        assert!(algo.schedules.contains_key(&primary_id));
        assert_eq!(algo.counts(&primary_id), Some((1, 0)));
        assert!(algo.clock().timer_names().contains(&primary_id.to_string()));
    }

    #[rstest]
    fn test_iceberg_sends_one_child_for_the_display_size() {
        let mut algo = create_iceberg_algorithm();
        register_algorithm(&mut algo);

        add_instrument_to_cache(&algo);

        let order = create_limit_order_with_params(
            &algo,
            schedule_params("4"),
            Quantity::from("10"),
            Price::from("1000.00"),
        );
        let primary_id = order.client_order_id();

        algo.on_order(order).unwrap();

        let child = child(&algo, "O-001-E1");

        assert_eq!(child.order_type(), OrderType::Limit);
        assert_eq!(child.quantity(), Quantity::from("4"));
        assert_eq!(child.price(), Some(Price::from("1000.00")));
        assert_eq!(child.exec_spawn_id(), Some(primary_id));
        assert_eq!(
            algo.cache().order(&primary_id).unwrap().quantity(),
            Quantity::from("6")
        );
        assert_eq!(
            algo.working_child(&primary_id),
            Some(child.client_order_id())
        );
        assert_eq!(algo.counts(&primary_id), Some((1, 0)));
    }

    #[rstest]
    fn test_iceberg_sends_the_next_child_when_one_is_terminal_and_conserves_quantity() {
        let mut algo = create_iceberg_algorithm();
        register_algorithm(&mut algo);

        add_instrument_to_cache(&algo);

        let order = create_limit_order_with_params(
            &algo,
            schedule_params("4"),
            Quantity::from("10"),
            Price::from("1000.00"),
        );
        let primary_id = order.client_order_id();

        algo.on_order(order).unwrap();

        // The first child fills in full: the primary keeps the reduction and the next child is sent.
        let first = child(&algo, "O-001-E1");
        dispatch(&mut algo, accepted_event(&first));
        dispatch(&mut algo, filled_event(&first, Quantity::from("4")));

        let second = child(&algo, "O-001-E2");
        assert_eq!(second.quantity(), Quantity::from("4"));
        assert_eq!(algo.counts(&primary_id), Some((2, 0)));

        let second_id = second.client_order_id();
        dispatch(&mut algo, accepted_event(&second));
        dispatch(&mut algo, filled_event(&second, Quantity::from("4")));

        // Two of the ten remain, so the last child carries the remainder rather than a display size.
        let third = child(&algo, "O-001-E3");
        assert_eq!(third.quantity(), Quantity::from("2"));
        assert_eq!(algo.counts(&primary_id), Some((3, 0)));

        // Filling the remainder completes the sequence with the scheduled children summing to the
        // parent quantity: 4 + 4 + 2 = 10.
        let third_id = third.client_order_id();
        dispatch(&mut algo, accepted_event(&third));
        dispatch(&mut algo, filled_event(&third, Quantity::from("2")));

        assert!(!algo.schedules.contains_key(&primary_id));
        assert!(algo.working_child(&primary_id).is_none());
        assert!(algo.clock().timer_names().is_empty());
        assert_eq!(second_id, ClientOrderId::from("O-001-E2"));
        assert_eq!(third_id, ClientOrderId::from("O-001-E3"));
    }

    #[rstest]
    fn test_iceberg_cancelled_child_restores_the_primary_and_counts_the_churn() {
        let mut algo = create_iceberg_algorithm();
        register_algorithm(&mut algo);

        add_instrument_to_cache(&algo);

        let order = create_limit_order_with_params(
            &algo,
            schedule_params("4"),
            Quantity::from("10"),
            Price::from("1000.00"),
        );
        let primary_id = order.client_order_id();

        algo.on_order(order).unwrap();

        let first = child(&algo, "O-001-E1");
        dispatch(&mut algo, canceled_event(&first));

        // The unfilled quantity of a cancelled child is restored to the primary before the next
        // child is sent, so the next child is the display size rather than a leftover and the
        // primary holds the ten less the working child.
        let second = child(&algo, "O-001-E2");
        assert_eq!(second.quantity(), Quantity::from("4"));
        assert_eq!(
            algo.cache().order(&primary_id).unwrap().quantity(),
            Quantity::from("6")
        );
        assert_eq!(algo.counts(&primary_id), Some((2, 1)));

        dispatch(&mut algo, canceled_event(&second));

        let third = child(&algo, "O-001-E3");
        assert_eq!(third.quantity(), Quantity::from("4"));
        assert_eq!(algo.counts(&primary_id), Some((3, 2)));
        assert_eq!(
            algo.cache().order(&primary_id).unwrap().quantity(),
            Quantity::from("6")
        );
    }

    #[rstest]
    fn test_iceberg_denies_the_primary_when_a_child_is_denied() {
        let mut algo = create_iceberg_algorithm();
        register_algorithm(&mut algo);

        add_instrument_to_cache(&algo);

        let order = create_limit_order_with_params(
            &algo,
            schedule_params("4"),
            Quantity::from("10"),
            Price::from("1000.00"),
        );
        let primary_id = order.client_order_id();

        algo.on_order(order).unwrap();
        assert!(algo.schedules.contains_key(&primary_id));

        let first = child(&algo, "O-001-E1");
        algo.on_order_denied(
            OrderDeniedSpec::builder()
                .trader_id(first.trader_id())
                .strategy_id(first.strategy_id())
                .instrument_id(first.instrument_id())
                .client_order_id(first.client_order_id())
                .reason(Ustr::from("VALIDATION_FAILED: no"))
                .build(),
        );

        assert!(!algo.schedules.contains_key(&primary_id));
    }

    #[rstest]
    fn test_iceberg_crossed_child_is_requoted_only_after_the_interval() {
        let mut algo = create_iceberg_algorithm();
        let clock = register_algorithm_with_clock(&mut algo);

        add_instrument_to_cache(&algo);

        let order = create_limit_order_with_params(
            &algo,
            schedule_params("4"),
            Quantity::from("10"),
            Price::from("1000.00"),
        );
        let primary_id = order.client_order_id();

        algo.on_order(order).unwrap();

        // The child is accepted at the venue, so a cancellation is a command rather than a local
        // state change.
        let first = child(&algo, "O-001-E1");
        dispatch(&mut algo, accepted_event(&first));

        let cancel = Rc::new(RefCell::new(None::<CancelOrder>));
        let handler = msgbus::TypedIntoHandler::from({
            let captured = cancel.clone();
            move |command: TradingCommand| {
                if let TradingCommand::CancelOrder(command) = command {
                    *captured.borrow_mut() = Some(command);
                }
            }
        });
        msgbus::register_trading_command_endpoint(
            MessagingSwitchboard::exec_engine_queue_execute(),
            handler,
        );

        let quote = QuoteTick::new(
            InstrumentId::from("ETHUSDT-PERP.BINANCE"),
            Price::from("999.00"),
            Price::from("1000.00"),
            Quantity::from("1"),
            Quantity::from("1"),
            0.into(),
            0.into(),
        );

        // The market crosses the child's price immediately, which is inside the re-quote interval,
        // so the child is left working and no cancellation is sent.
        algo.on_quote(&quote).unwrap();
        assert!(algo.working_child(&primary_id).is_some());
        assert!(cancel.borrow().is_none());

        // Once the interval has elapsed the crossed child is cancelled.
        clock.borrow_mut().advance_time(6_000_000_000.into(), true);
        algo.on_quote(&quote).unwrap();

        assert_eq!(
            cancel
                .borrow()
                .as_ref()
                .map(|command| command.client_order_id),
            Some(ClientOrderId::from("O-001-E1"))
        );
    }

    #[rstest]
    fn test_iceberg_deadline_cancels_the_working_child_and_completes() {
        let mut algo = create_iceberg_algorithm();
        register_algorithm(&mut algo);

        add_instrument_to_cache(&algo);

        let mut exec_params = schedule_params("4");
        exec_params.insert(Ustr::from("horizon_secs"), Ustr::from("60"));

        let order = create_limit_order_with_params(
            &algo,
            exec_params,
            Quantity::from("10"),
            Price::from("1000.00"),
        );
        let primary_id = order.client_order_id();

        algo.on_order(order).unwrap();
        assert!(algo.working_child(&primary_id).is_some());
        assert!(algo.clock().timer_names().contains(&primary_id.to_string()));

        let event = TimeEvent::new(primary_id.inner(), UUID4::new(), 0.into(), 0.into());
        ExecutionAlgorithm::on_time_event(&mut algo, &event).unwrap();

        assert!(!algo.schedules.contains_key(&primary_id));
        assert!(algo.working_child(&primary_id).is_none());
        assert!(algo.clock().timer_names().is_empty());
        assert!(algo.cache().order(&primary_id).is_some());
    }

    #[rstest]
    fn test_iceberg_reset_clears_the_schedules() {
        let mut algo = create_iceberg_algorithm();
        register_algorithm(&mut algo);

        add_instrument_to_cache(&algo);

        let order = create_limit_order_with_params(
            &algo,
            schedule_params("4"),
            Quantity::from("10"),
            Price::from("1000.00"),
        );

        algo.on_order(order).unwrap();
        assert!(!algo.schedules.is_empty());

        ExecutionAlgorithm::on_reset(&mut algo).unwrap();

        assert!(algo.schedules.is_empty());
    }

    #[rstest]
    fn test_iceberg_display_size_is_floored_to_the_instrument_precision() {
        let mut algo = create_iceberg_algorithm();
        register_algorithm(&mut algo);

        add_instrument_to_cache(&algo);

        let order = create_limit_order_with_params(
            &algo,
            schedule_params("4.5678"),
            Quantity::from("10"),
            Price::from("1000.00"),
        );

        algo.on_order(order).unwrap();

        assert_eq!(child(&algo, "O-001-E1").quantity(), Quantity::from("4.567"));
    }

    #[rstest]
    fn test_iceberg_denies_a_display_size_below_the_instrument_size_increment() {
        let mut algo = create_iceberg_algorithm();
        register_algorithm(&mut algo);

        add_instrument_to_cache(&algo);

        let instrument = {
            let cache_rc = algo.core.cache_rc();
            let cache = cache_rc.borrow();
            cache
                .instrument(&InstrumentId::from("ETHUSDT-PERP.BINANCE"))
                .cloned()
        }
        .unwrap();

        // One tenth of the increment floors to zero at the instrument precision.
        let display_size = format!(
            "{}",
            instrument.size_increment().as_decimal() / rust_decimal::Decimal::from(10)
        );
        let order = create_limit_order_with_params(
            &algo,
            schedule_params(&display_size),
            Quantity::from("10"),
            Price::from("1000.00"),
        );

        assert_iceberg_denied(
            &mut algo,
            &order,
            &format!(
                "VALIDATION_FAILED: display_size={display_size} is below the instrument size increment {}",
                instrument.size_increment()
            ),
        );
    }
}
