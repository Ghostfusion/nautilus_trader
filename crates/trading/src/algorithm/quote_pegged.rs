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

//! Quote pegged execution algorithm.
//!
//! A quote pegged order keeps one child order resting at the touch, and re-quotes it as the touch
//! moves. The child is a limit order for the primary's whole remaining quantity, so the algorithm
//! never holds a second copy of the order state: the primary in the cache owns the quantity, and
//! the algorithm holds only the schedule that decides where and when to quote.
//!
//! # Parameters
//!
//! Orders submitted to this algorithm must include `exec_algorithm_params` with:
//! - `pegging`: Where the child is quoted, one of `passive` or `join`. A passive quote joins the
//!   near touch (the same side's best price: the best bid for a buy, the best ask for a sell). A
//!   join quote is placed one price increment inside the opposite touch (the best ask less one
//!   increment for a buy, the best bid plus one increment for a sell), which cannot cross the book.
//! - `requote_secs`: The minimum interval between sending a child and re-quoting it, in seconds.
//!
//! A quote price is taken from the cached quote for the instrument, and it must be a valid price on
//! the instrument's tick grid and must never cross the book. An order whose quote price cannot be
//! formed that way is refused.
//!
//! The horizon of the execution policy is honoured as the sequence deadline, a declared price limit
//! is honoured by refusing an order whose quote would breach it, and a passive preference is what
//! the algorithm does already. The other policy parts are refused with a denial naming the field,
//! as is an aggressive preference, which a quote that rests cannot satisfy.

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
    types::Price,
};
use rust_decimal::{Decimal, RoundingStrategy};
use ustr::Ustr;

use super::{
    ExecutionAlgorithm, ExecutionAlgorithmConfig, ExecutionAlgorithmCore, ExecutionAlgorithmNative,
    ExecutionIntent, ExecutionPolicy, ExecutionPreference, PolicyError, PolicyPart,
};
use crate::nautilus_execution_algorithm;

/// Configuration for [`QuotePeggedAlgorithm`].
pub type QuotePeggedAlgorithmConfig = ExecutionAlgorithmConfig;

/// The policy parts quote pegging can honour.
///
/// The horizon becomes the sequence deadline, a price limit is checked against each quote, and a
/// passive preference is what the algorithm does already. A participation rate, a slippage cap and
/// an urgency would each have to change how the children are placed, which this algorithm does not
/// do.
static QUOTE_PEGGED_SUPPORTED_POLICY_PARTS: [PolicyPart; 3] = [
    PolicyPart::Horizon,
    PolicyPart::PriceLimit,
    PolicyPart::Preference,
];

/// The parameter key declaring where the child is quoted.
const KEY_PEGGING: &str = "pegging";
/// The parameter key declaring the minimum interval between a child and its re-quote.
const KEY_REQUOTE_SECS: &str = "requote_secs";

/// Where a quote pegged child is placed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Pegging {
    /// Join the near touch: the same side's best price.
    Passive,
    /// Join the opposite touch, one price increment inside it.
    Join,
}

/// Quote pegged execution algorithm.
///
/// Keeps at most one child order working per primary order, quoting it at the touch declared by
/// `pegging`, and re-quoting it only when the touch has moved by at least one price increment and
/// the re-quote interval has elapsed since the child was sent.
#[derive(Debug)]
pub struct QuotePeggedAlgorithm {
    /// The algorithm core.
    pub core: ExecutionAlgorithmCore,
    /// The scheduling state of each primary order.
    schedules: AHashMap<ClientOrderId, QuotePeggedSchedule>,
}

impl QuotePeggedAlgorithm {
    /// Creates a new [`QuotePeggedAlgorithm`] instance.
    #[must_use]
    pub fn new(config: QuotePeggedAlgorithmConfig) -> Self {
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

        log::info!("Completed quote pegged execution for {primary_id}");
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

    /// Returns the latest quote for `instrument_id` from the cache, if there is one.
    fn cached_quote(&self, instrument_id: &InstrumentId) -> Option<QuoteTick> {
        let cache = ExecutionAlgorithmNative::exec_algorithm_core(self).cache_ref();

        cache.quote(instrument_id).copied()
    }

    /// Sends the next child for a primary order, when it has quantity remaining.
    fn send_next_child(&mut self, primary_id: ClientOrderId) {
        let Some(price) = self
            .schedules
            .get(&primary_id)
            .map(|schedule| schedule.price)
        else {
            return;
        };

        let Some(primary) = self.cached_order(primary_id) else {
            log::warn!("Cannot find primary order {primary_id} to send the next child");
            self.complete_sequence(primary_id);
            return;
        };

        // The child carries the primary's whole remaining quantity, so zero remaining is the only
        // completion condition the spawned quantity itself can reach.
        let quantity = primary.quantity();
        if quantity.is_zero() {
            self.complete_sequence(primary_id);
            return;
        }

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
            log::error!("Failed to submit the quote pegged child {child_id}: {e}");
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

        log::info!("Quote pegged child {child_id} submitted for {quantity} at {price}");
    }

    /// Handles a child order that has reached a terminal state.
    ///
    /// The unfilled quantity of a cancelled or expired child is restored to the primary order by
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

    /// Re-quotes the working child when the touch has moved by at least one price increment and
    /// the re-quote interval has elapsed since the child was sent.
    ///
    /// A quote whose price would cross the book, or that cannot be formed on the instrument's tick
    /// grid, is left alone: the child keeps working and the next quote re-checks it.
    fn handle_quote(&mut self, quote: &QuoteTick) {
        let primary_ids: Vec<ClientOrderId> = self.schedules.keys().copied().collect();

        for primary_id in primary_ids {
            let (
                instrument_id,
                child_id,
                pegging,
                order_side,
                price,
                touch,
                requote_nanos,
                last_send,
            ) = {
                let Some(schedule) = self.schedules.get(&primary_id) else {
                    continue;
                };
                let Some(child_id) = schedule.child else {
                    continue;
                };

                (
                    schedule.instrument_id,
                    child_id,
                    schedule.pegging,
                    schedule.order_side,
                    schedule.price,
                    schedule.touch,
                    schedule.requote_nanos,
                    schedule.last_send,
                )
            };

            if quote.instrument_id != instrument_id {
                continue;
            }

            let Some(instrument) = self.cached_instrument(&instrument_id) else {
                continue;
            };

            let Ok((candidate, candidate_touch)) =
                compute_quote(&instrument, pegging, order_side, quote)
            else {
                continue;
            };

            if candidate == price {
                continue;
            }

            let increment = instrument.price_increment().as_decimal();
            let moved = (candidate_touch.as_decimal() - touch.as_decimal()).abs();
            if moved < increment {
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

            let Some(child) = self.cached_order(child_id) else {
                continue;
            };

            // The next child is placed at the moved touch when the cancelled child's terminal
            // event arrives.
            if let Some(schedule) = self.schedules.get_mut(&primary_id) {
                schedule.price = candidate;
                schedule.touch = candidate_touch;
            }

            let mut child = child;
            match self.cancel_order(&mut child, None) {
                Ok(()) => log::info!("Re-quoting quote pegged child {child_id} at {candidate}"),
                Err(e) => log::error!("Failed to cancel quote pegged child {child_id}: {e}"),
            }
        }
    }
}

// The clock and component lifecycle dispatch through the `DataActor` hooks,
// so forward them to the `ExecutionAlgorithm` implementations.
impl DataActor for QuotePeggedAlgorithm {
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

nautilus_execution_algorithm!(QuotePeggedAlgorithm, {
    fn supported_policy_parts(&self) -> &'static [PolicyPart] {
        &QUOTE_PEGGED_SUPPORTED_POLICY_PARTS
    }

    fn on_order(&mut self, order: OrderAny) -> anyhow::Result<()> {
        let primary_id = order.client_order_id();

        if self.schedules.contains_key(&primary_id) {
            anyhow::bail!("Order {primary_id} already being executed");
        }

        log::info!("Received order for quote pegged execution: {order:?}");

        // A quote pegged order rests on the book, so it is placed by a limit order.
        if order.order_type() != OrderType::Limit {
            let reason = OrderDeniedReason::UnsupportedOrderType {
                order_type: order.order_type(),
            }
            .to_string();
            return self.deny_order(&order, Ustr::from(&reason));
        }

        let instrument = self.cached_instrument(&order.instrument_id());

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

        let pegging = match parse_pegging(exec_params) {
            Ok(pegging) => pegging,
            Err(reason) => return self.deny_order(&order, reason),
        };

        let requote_nanos = match parse_requote_secs(exec_params) {
            Ok(requote_nanos) => requote_nanos,
            Err(reason) => return self.deny_order(&order, reason),
        };

        let instrument_id = order.instrument_id();
        let order_side = order.order_side();

        // The quote price is pegged to the book, so the order cannot start without one.
        let Some(quote) = self.cached_quote(&instrument_id) else {
            return self.deny_order(
                &order,
                validation_failed(format!(
                    "no quote available for {instrument_id} to form a quote price"
                )),
            );
        };

        let (price, touch) = match compute_quote(&instrument, pegging, order_side, &quote) {
            Ok(quote_price) => quote_price,
            Err(reason) => return self.deny_order(&order, reason),
        };

        if let Some(limit) = policy.price_limit
            && !within_price_limit(order_side, price, limit)
        {
            return self.deny_order(
                &order,
                validation_failed(format!(
                    "price_limit={limit} is breached by the quote price {price}"
                )),
            );
        }

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

        self.schedules.insert(
            primary_id,
            QuotePeggedSchedule {
                instrument_id,
                order_side,
                pegging,
                price,
                touch,
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

        log::info!("Started quote pegged execution for {primary_id}");

        Ok(())
    }

    fn on_time_event(&mut self, event: &TimeEvent) -> anyhow::Result<()> {
        let primary_id = ClientOrderId::new(event.name);

        let Some(child_id) = self.working_child(&primary_id) else {
            if self.schedules.contains_key(&primary_id) {
                log::info!("Quote pegged deadline reached for {primary_id}");
                self.complete_sequence(primary_id);
            }
            return Ok(());
        };

        log::info!("Quote pegged deadline reached for {primary_id}, cancelling child {child_id}");

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
            self.handle_child_terminal(primary_id, false);
        }
    }

    fn on_order_denied(&mut self, event: OrderDenied) {
        self.abort_on_child_refusal(event.client_order_id, event.reason);
    }

    fn on_order_rejected(&mut self, event: OrderRejected) {
        let reason = OrderDeniedReason::SubmitFailed {
            detail: format!(
                "the quote pegged child {} was rejected by the venue: {}",
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

impl QuotePeggedAlgorithm {
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
struct QuotePeggedSchedule {
    /// The instrument the primary order executes in.
    instrument_id: InstrumentId,
    /// The side of the primary order.
    order_side: OrderSide,
    /// Where the child is quoted.
    pegging: Pegging,
    /// The price the working child is quoted at.
    price: Price,
    /// The touch the quote price was derived from.
    touch: Price,
    /// The minimum interval between sending a child and re-quoting it, in nanoseconds.
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

/// Returns the pegging mode declared in `params`.
fn parse_pegging(params: &IndexMap<Ustr, Ustr>) -> Result<Pegging, Ustr> {
    let Some(raw) = params.get(&Ustr::from(KEY_PEGGING)) else {
        return Err(validation_failed(
            "pegging not found in exec_algorithm_params",
        ));
    };

    match raw.as_str() {
        "passive" => Ok(Pegging::Passive),
        "join" => Ok(Pegging::Join),
        other => Err(validation_failed(format!(
            "pegging={other} must be one of passive or join"
        ))),
    }
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

/// Returns the quote price for `order_side` and the touch it was derived from.
///
/// The quote price must be a valid price on the instrument's tick grid and must never cross the
/// book, otherwise the quote cannot be formed and the pair is refused.
fn compute_quote(
    instrument: &InstrumentAny,
    pegging: Pegging,
    order_side: OrderSide,
    quote: &QuoteTick,
) -> Result<(Price, Price), Ustr> {
    let increment = instrument.price_increment().as_decimal();
    let bid = quote.bid_price.as_decimal();
    let ask = quote.ask_price.as_decimal();

    let (touch, raw) = match (pegging, order_side) {
        (Pegging::Passive, OrderSide::Buy) => (bid, bid),
        (Pegging::Passive, OrderSide::Sell) => (ask, ask),
        // One price increment inside the opposite touch, clamped so it stays inside the book.
        (Pegging::Join, OrderSide::Buy) => (
            ask,
            clamp_inside(ask - increment, order_side, bid, ask, increment),
        ),
        (Pegging::Join, OrderSide::Sell) => (
            bid,
            clamp_inside(bid + increment, order_side, bid, ask, increment),
        ),
    };

    // A quote is never placed on the far side of the opposite touch: it would cross the book.
    let crosses = match order_side {
        OrderSide::Buy => raw >= ask,
        OrderSide::Sell => raw <= bid,
    };
    if crosses {
        return Err(unusable_quote(instrument, quote));
    }

    let Some(price) = make_grid_price(instrument, raw) else {
        return Err(unusable_quote(instrument, quote));
    };
    let Some(touch_price) = make_grid_price(instrument, touch) else {
        return Err(unusable_quote(instrument, quote));
    };

    Ok((price, touch_price))
}

/// Clamps `candidate` so a quote stays on the non-crossing side of the book.
fn clamp_inside(
    candidate: Decimal,
    order_side: OrderSide,
    bid: Decimal,
    ask: Decimal,
    increment: Decimal,
) -> Decimal {
    match order_side {
        // A buy never bids up to or through the ask.
        OrderSide::Buy => candidate.min(ask - increment),
        // A sell never offers down to or through the bid.
        OrderSide::Sell => candidate.max(bid + increment),
    }
}

/// Returns `value` as a `Price` when it is positive and a valid price on the instrument's tick
/// grid, otherwise `None`.
fn make_grid_price(instrument: &InstrumentAny, value: Decimal) -> Option<Price> {
    let increment = instrument.price_increment().as_decimal();
    if value <= Decimal::ZERO || increment.is_zero() {
        return None;
    }

    let precision = u32::from(instrument.price_precision());
    if value.round_dp_with_strategy(precision, RoundingStrategy::MidpointNearestEven) != value {
        return None;
    }

    if !(value % increment).is_zero() {
        return None;
    }

    instrument.try_make_price_from_decimal(value).ok()
}

/// Returns the refusal of a quote price that cannot be formed from `quote`.
fn unusable_quote(instrument: &InstrumentAny, quote: &QuoteTick) -> Ustr {
    validation_failed(format!(
        "quote_price cannot be formed for {} from bid={} ask={}",
        instrument.id(),
        quote.bid_price,
        quote.ask_price
    ))
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
        messages::execution::TradingCommand,
        msgbus::{self, MessagingSwitchboard, TypedHandler},
        timer::TimeEvent,
    };
    use nautilus_core::UUID4;
    use nautilus_model::{
        enums::{OrderStatus, TimeInForce},
        events::{
            OrderDeniedReason, OrderEventAny,
            order::spec::{OrderAcceptedSpec, OrderCanceledSpec, OrderDeniedSpec, OrderFilledSpec},
        },
        identifiers::{AccountId, ExecAlgorithmId, StrategyId, TraderId, VenueOrderId},
        orders::{LimitOrder, MarketOrder},
        types::Quantity,
    };
    use rstest::rstest;

    use super::*;

    fn create_quote_pegged_algorithm() -> QuotePeggedAlgorithm {
        // Use a unique ID to avoid thread-local registry conflicts in parallel tests.
        let unique_id = format!("QUOTE-PEGGED-{}", UUID4::new());
        let config = QuotePeggedAlgorithmConfig {
            exec_algorithm_id: Some(ExecAlgorithmId::new(&unique_id)),
            ..Default::default()
        };
        QuotePeggedAlgorithm::new(config)
    }

    fn register_algorithm_with_clock(algo: &mut QuotePeggedAlgorithm) -> Rc<RefCell<VirtualClock>> {
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

    fn register_algorithm(algo: &mut QuotePeggedAlgorithm) {
        let _ = register_algorithm_with_clock(algo);
    }

    fn add_instrument_to_cache(algo: &QuotePeggedAlgorithm) {
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

    fn schedule_params(pegging: &str) -> IndexMap<Ustr, Ustr> {
        params(&[(KEY_PEGGING, pegging), (KEY_REQUOTE_SECS, "5")])
    }

    fn quote(bid: &str, ask: &str) -> QuoteTick {
        QuoteTick::new(
            InstrumentId::from("ETHUSDT-PERP.BINANCE"),
            Price::from(bid),
            Price::from(ask),
            Quantity::from("1"),
            Quantity::from("1"),
            0.into(),
            0.into(),
        )
    }

    fn seed_quote(algo: &QuotePeggedAlgorithm, bid: &str, ask: &str) {
        let cache_rc = algo.core.cache_rc();
        cache_rc.borrow_mut().add_quote(quote(bid, ask)).unwrap();
    }

    fn create_limit_order(
        algo: &QuotePeggedAlgorithm,
        exec_params: IndexMap<Ustr, Ustr>,
        order_side: OrderSide,
        quantity: Quantity,
        price: Price,
    ) -> OrderAny {
        let client_order_id = ClientOrderId::from("O-001");

        OrderAny::Limit(LimitOrder::new(
            TraderId::from("TRADER-001"),
            StrategyId::from("STRAT-001"),
            InstrumentId::from("ETHUSDT-PERP.BINANCE"),
            client_order_id,
            order_side,
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
        algo: &QuotePeggedAlgorithm,
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

    /// Drives `on_order` for `order` and returns the single denial reason it published.
    fn denial_reason(algo: &mut QuotePeggedAlgorithm, order: &OrderAny) -> String {
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

        assert_eq!(
            algo.cache()
                .order(&order.client_order_id())
                .unwrap()
                .status(),
            OrderStatus::Denied
        );
        assert!(algo.schedules.is_empty());
        assert!(algo.clock().timer_names().is_empty());

        let events = events.borrow();
        assert_eq!(events.len(), 1);

        match &events[0] {
            OrderEventAny::Denied(event) => event.reason.to_string(),
            other => panic!("expected a denial, was {other:?}"),
        }
    }

    fn assert_quote_pegged_denied(
        algo: &mut QuotePeggedAlgorithm,
        order: &OrderAny,
        expected_reason: &str,
    ) {
        assert_eq!(denial_reason(algo, order), expected_reason);
    }

    fn accepted_event(order: &OrderAny) -> OrderEventAny {
        OrderEventAny::Accepted(
            OrderAcceptedSpec::builder()
                .trader_id(order.trader_id())
                .strategy_id(order.strategy_id())
                .instrument_id(order.instrument_id())
                .client_order_id(order.client_order_id())
                .venue_order_id(VenueOrderId::from("V-001"))
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
                .venue_order_id(VenueOrderId::from("V-001"))
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
                .venue_order_id(VenueOrderId::from("V-001"))
                .account_id(AccountId::from("SIM-001"))
                .order_side(order.order_side())
                .order_type(order.order_type())
                .last_qty(last_qty)
                .last_px(order.price().unwrap_or(Price::from("100.00")))
                .maybe_position_id(None)
                .maybe_commission(None)
                .maybe_info(None)
                .build(),
        )
    }

    fn child(algo: &QuotePeggedAlgorithm, client_order_id: &str) -> OrderAny {
        algo.cache()
            .order(&ClientOrderId::from(client_order_id))
            .unwrap()
    }

    /// Applies an order event to the cache and dispatches it, as the engine does.
    fn dispatch(algo: &mut QuotePeggedAlgorithm, event: OrderEventAny) {
        {
            let cache_rc = algo.core.cache_rc();
            cache_rc.borrow_mut().update_order(&event).unwrap();
        }

        algo.handle_order_event(event);
    }

    /// Registers a handler that counts the cancel commands the algorithm sends.
    fn count_cancels() -> Rc<RefCell<usize>> {
        let cancels = Rc::new(RefCell::new(0usize));
        let handler = msgbus::TypedIntoHandler::from({
            let cancels = cancels.clone();
            move |cmd: TradingCommand| {
                if let TradingCommand::CancelOrder(_) = cmd {
                    *cancels.borrow_mut() += 1;
                }
            }
        });
        msgbus::register_trading_command_endpoint(
            MessagingSwitchboard::exec_engine_queue_execute(),
            handler,
        );

        cancels
    }

    #[rstest]
    fn test_quote_pegged_rejects_non_limit_orders() {
        let mut algo = create_quote_pegged_algorithm();
        register_algorithm(&mut algo);

        add_instrument_to_cache(&algo);

        let order = create_market_order_with_params(&algo, schedule_params("passive"));

        assert_quote_pegged_denied(
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
    #[case(&[], "VALIDATION_FAILED: pegging not found in exec_algorithm_params")]
    #[case(
        &[(KEY_PEGGING, "sideways")],
        "VALIDATION_FAILED: pegging=sideways must be one of passive or join"
    )]
    fn test_quote_pegged_denies_a_missing_or_malformed_pegging(
        #[case] entries: &[(&str, &str)],
        #[case] expected_reason: &str,
    ) {
        let mut algo = create_quote_pegged_algorithm();
        register_algorithm(&mut algo);

        add_instrument_to_cache(&algo);

        let mut exec_params = params(&[(KEY_REQUOTE_SECS, "5")]);
        for (key, value) in entries {
            exec_params.insert(Ustr::from(key), Ustr::from(value));
        }

        let order = create_limit_order(
            &algo,
            exec_params,
            OrderSide::Buy,
            Quantity::from("10"),
            Price::from("1000.00"),
        );

        assert_quote_pegged_denied(&mut algo, &order, expected_reason);
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
    fn test_quote_pegged_denies_a_missing_or_malformed_requote_secs(
        #[case] entries: &[(&str, &str)],
        #[case] expected_reason: &str,
    ) {
        let mut algo = create_quote_pegged_algorithm();
        register_algorithm(&mut algo);

        add_instrument_to_cache(&algo);

        let mut exec_params = params(&[(KEY_PEGGING, "passive")]);
        for (key, value) in entries {
            exec_params.insert(Ustr::from(key), Ustr::from(value));
        }

        let order = create_limit_order(
            &algo,
            exec_params,
            OrderSide::Buy,
            Quantity::from("10"),
            Price::from("1000.00"),
        );

        assert_quote_pegged_denied(&mut algo, &order, expected_reason);
    }

    #[rstest]
    #[case("participation_rate", "0.5")]
    #[case("max_slippage_bps", "15")]
    #[case("urgency", "high")]
    fn test_quote_pegged_refuses_a_policy_part_it_cannot_honour(
        #[case] key: &str,
        #[case] value: &str,
    ) {
        let mut algo = create_quote_pegged_algorithm();
        register_algorithm(&mut algo);

        add_instrument_to_cache(&algo);

        let mut exec_params = schedule_params("passive");
        exec_params.insert(Ustr::from(key), Ustr::from(value));

        let order = create_limit_order(
            &algo,
            exec_params,
            OrderSide::Buy,
            Quantity::from("10"),
            Price::from("1000.00"),
        );

        assert_quote_pegged_denied(
            &mut algo,
            &order,
            &format!("VALIDATION_FAILED: {key} is not supported by this execution algorithm"),
        );
    }

    #[rstest]
    fn test_quote_pegged_refuses_an_aggressive_preference() {
        let mut algo = create_quote_pegged_algorithm();
        register_algorithm(&mut algo);

        add_instrument_to_cache(&algo);

        let mut exec_params = schedule_params("passive");
        exec_params.insert(Ustr::from("preference"), Ustr::from("aggressive"));

        let order = create_limit_order(
            &algo,
            exec_params,
            OrderSide::Buy,
            Quantity::from("10"),
            Price::from("1000.00"),
        );

        assert_quote_pegged_denied(
            &mut algo,
            &order,
            "VALIDATION_FAILED: preference=aggressive is not possible for an order that rests",
        );
    }

    #[rstest]
    fn test_quote_pegged_refuses_a_quote_price_that_breaches_the_price_limit() {
        let mut algo = create_quote_pegged_algorithm();
        register_algorithm(&mut algo);

        add_instrument_to_cache(&algo);
        seed_quote(&algo, "100.00", "100.02");

        let mut exec_params = schedule_params("passive");
        exec_params.insert(Ustr::from("price_limit"), Ustr::from("99.99"));

        let order = create_limit_order(
            &algo,
            exec_params,
            OrderSide::Buy,
            Quantity::from("10"),
            Price::from("1000.00"),
        );

        let limit = Price::from("99.99");
        let price = Price::from("100.00");

        assert_quote_pegged_denied(
            &mut algo,
            &order,
            &format!(
                "VALIDATION_FAILED: price_limit={limit} is breached by the quote price {price}"
            ),
        );
    }

    #[rstest]
    fn test_quote_pegged_denies_an_order_whose_quote_crosses_the_book() {
        let mut algo = create_quote_pegged_algorithm();
        register_algorithm(&mut algo);

        add_instrument_to_cache(&algo);
        // A locked book: the near touch for a buy is at the ask, which would cross.
        seed_quote(&algo, "100.02", "100.02");

        let order = create_limit_order(
            &algo,
            schedule_params("passive"),
            OrderSide::Buy,
            Quantity::from("10"),
            Price::from("1000.00"),
        );

        let instrument_id = InstrumentId::from("ETHUSDT-PERP.BINANCE");
        let bid = Price::from("100.02");
        let ask = Price::from("100.02");

        assert_quote_pegged_denied(
            &mut algo,
            &order,
            &format!(
                "VALIDATION_FAILED: quote_price cannot be formed for {instrument_id} from bid={bid} ask={ask}"
            ),
        );
    }

    #[rstest]
    fn test_quote_pegged_denies_an_order_without_a_quote() {
        let mut algo = create_quote_pegged_algorithm();
        register_algorithm(&mut algo);

        add_instrument_to_cache(&algo);

        let order = create_limit_order(
            &algo,
            schedule_params("passive"),
            OrderSide::Buy,
            Quantity::from("10"),
            Price::from("1000.00"),
        );

        let instrument_id = InstrumentId::from("ETHUSDT-PERP.BINANCE");

        assert_quote_pegged_denied(
            &mut algo,
            &order,
            &format!(
                "VALIDATION_FAILED: no quote available for {instrument_id} to form a quote price"
            ),
        );
    }

    #[rstest]
    #[case("passive", "100.00")]
    #[case("join", "100.01")]
    fn test_quote_pegged_places_one_child_at_the_expected_price(
        #[case] pegging: &str,
        #[case] expected_price: &str,
    ) {
        let mut algo = create_quote_pegged_algorithm();
        register_algorithm(&mut algo);

        add_instrument_to_cache(&algo);
        seed_quote(&algo, "100.00", "100.02");

        let order = create_limit_order(
            &algo,
            schedule_params(pegging),
            OrderSide::Buy,
            Quantity::from("10"),
            Price::from("1000.00"),
        );
        let primary_id = order.client_order_id();

        algo.on_order(order).unwrap();

        let child = child(&algo, "O-001-E1");

        assert_eq!(child.order_type(), OrderType::Limit);
        assert_eq!(child.quantity(), Quantity::from("10"));
        assert_eq!(child.price(), Some(Price::from(expected_price)));
        assert_eq!(child.exec_spawn_id(), Some(primary_id));
        assert!(
            algo.cache()
                .order(&primary_id)
                .unwrap()
                .quantity()
                .is_zero()
        );
        assert_eq!(
            algo.working_child(&primary_id),
            Some(child.client_order_id())
        );
        assert_eq!(algo.counts(&primary_id), Some((1, 0)));
    }

    #[rstest]
    #[case("passive", "100.02")]
    #[case("join", "100.01")]
    fn test_quote_pegged_places_a_sell_child_at_the_expected_price(
        #[case] pegging: &str,
        #[case] expected_price: &str,
    ) {
        let mut algo = create_quote_pegged_algorithm();
        register_algorithm(&mut algo);

        add_instrument_to_cache(&algo);
        seed_quote(&algo, "100.00", "100.02");

        let order = create_limit_order(
            &algo,
            schedule_params(pegging),
            OrderSide::Sell,
            Quantity::from("10"),
            Price::from("1000.00"),
        );
        let primary_id = order.client_order_id();

        algo.on_order(order).unwrap();

        let child = child(&algo, "O-001-E1");

        assert_eq!(child.order_side(), OrderSide::Sell);
        assert_eq!(child.price(), Some(Price::from(expected_price)));
        assert_eq!(algo.counts(&primary_id), Some((1, 0)));
    }

    #[rstest]
    fn test_quote_pegged_requotes_only_after_the_touch_moves_and_the_interval_elapses() {
        let mut algo = create_quote_pegged_algorithm();
        let clock = register_algorithm_with_clock(&mut algo);

        add_instrument_to_cache(&algo);
        seed_quote(&algo, "100.00", "100.02");

        let order = create_limit_order(
            &algo,
            schedule_params("passive"),
            OrderSide::Buy,
            Quantity::from("10"),
            Price::from("1000.00"),
        );
        let primary_id = order.client_order_id();

        algo.on_order(order).unwrap();
        assert_eq!(algo.schedules[&primary_id].price, Price::from("100.00"));

        let cancels = count_cancels();

        // The touch moves by one increment, but the re-quote interval has not elapsed, so the
        // child is left working at its original price.
        let moved = quote("100.01", "100.03");
        algo.on_quote(&moved).unwrap();

        assert_eq!(*cancels.borrow(), 0);
        assert_eq!(algo.schedules[&primary_id].price, Price::from("100.00"));
        assert_eq!(
            algo.working_child(&primary_id),
            Some(ClientOrderId::from("O-001-E1"))
        );
        assert!(!child(&algo, "O-001-E1").is_closed());

        // Once the interval has elapsed the moved touch is re-quoted.
        clock.borrow_mut().advance_time(6_000_000_000.into(), true);
        algo.on_quote(&moved).unwrap();

        assert_eq!(*cancels.borrow(), 1);
        assert_eq!(algo.schedules[&primary_id].price, Price::from("100.01"));

        // The venue confirms the cancel and the next child is quoted at the moved touch.
        let first = child(&algo, "O-001-E1");
        dispatch(&mut algo, canceled_event(&first));

        let second = child(&algo, "O-001-E2");
        assert_eq!(second.price(), Some(Price::from("100.01")));
        assert_eq!(second.quantity(), Quantity::from("10"));
        assert_eq!(algo.counts(&primary_id), Some((2, 1)));
    }

    #[rstest]
    fn test_quote_pegged_does_not_requote_at_a_crossing_price() {
        let mut algo = create_quote_pegged_algorithm();
        let clock = register_algorithm_with_clock(&mut algo);

        add_instrument_to_cache(&algo);
        seed_quote(&algo, "100.00", "100.02");

        let order = create_limit_order(
            &algo,
            schedule_params("passive"),
            OrderSide::Buy,
            Quantity::from("10"),
            Price::from("1000.00"),
        );
        let primary_id = order.client_order_id();

        algo.on_order(order).unwrap();

        let cancels = count_cancels();
        clock.borrow_mut().advance_time(6_000_000_000.into(), true);

        // A crossed book: the computed near touch would cross, so the child is left working.
        let crossed = quote("100.02", "100.00");
        algo.on_quote(&crossed).unwrap();

        assert_eq!(*cancels.borrow(), 0);
        assert_eq!(algo.schedules[&primary_id].price, Price::from("100.00"));
        assert_eq!(
            algo.working_child(&primary_id),
            Some(ClientOrderId::from("O-001-E1"))
        );
    }

    #[rstest]
    fn test_quote_pegged_terminal_child_sends_one_next_child_and_restores_the_primary() {
        let mut algo = create_quote_pegged_algorithm();
        register_algorithm(&mut algo);

        add_instrument_to_cache(&algo);
        seed_quote(&algo, "100.00", "100.02");

        let order = create_limit_order(
            &algo,
            schedule_params("passive"),
            OrderSide::Buy,
            Quantity::from("10"),
            Price::from("1000.00"),
        );
        let primary_id = order.client_order_id();

        algo.on_order(order).unwrap();

        let first = child(&algo, "O-001-E1");
        assert!(
            algo.cache()
                .order(&primary_id)
                .unwrap()
                .quantity()
                .is_zero()
        );

        dispatch(&mut algo, canceled_event(&first));

        // The cancelled child's unfilled quantity is restored and immediately re-quoted, so the
        // primary holds nothing while the next child carries the whole quantity again.
        let second = child(&algo, "O-001-E2");
        assert_eq!(second.quantity(), Quantity::from("10"));
        assert_eq!(
            algo.working_child(&primary_id),
            Some(second.client_order_id())
        );
        assert_eq!(algo.counts(&primary_id), Some((2, 1)));
        assert!(
            algo.cache()
                .order(&ClientOrderId::from("O-001-E3"))
                .is_none()
        );
    }

    #[rstest]
    fn test_quote_pegged_filled_child_completes_the_sequence() {
        let mut algo = create_quote_pegged_algorithm();
        register_algorithm(&mut algo);

        add_instrument_to_cache(&algo);
        seed_quote(&algo, "100.00", "100.02");

        let order = create_limit_order(
            &algo,
            schedule_params("passive"),
            OrderSide::Buy,
            Quantity::from("10"),
            Price::from("1000.00"),
        );
        let primary_id = order.client_order_id();

        algo.on_order(order).unwrap();

        let first = child(&algo, "O-001-E1");
        dispatch(&mut algo, accepted_event(&first));
        dispatch(&mut algo, filled_event(&first, Quantity::from("10")));

        assert!(!algo.schedules.contains_key(&primary_id));
        assert!(algo.working_child(&primary_id).is_none());
        assert!(algo.clock().timer_names().is_empty());
    }

    #[rstest]
    fn test_quote_pegged_denies_the_primary_when_a_child_is_denied() {
        let mut algo = create_quote_pegged_algorithm();
        register_algorithm(&mut algo);

        add_instrument_to_cache(&algo);
        seed_quote(&algo, "100.00", "100.02");

        let order = create_limit_order(
            &algo,
            schedule_params("passive"),
            OrderSide::Buy,
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
        assert_eq!(
            algo.cache().order(&primary_id).unwrap().status(),
            OrderStatus::Denied
        );
    }

    #[rstest]
    fn test_quote_pegged_deadline_cancels_the_working_child_and_completes() {
        let mut algo = create_quote_pegged_algorithm();
        register_algorithm(&mut algo);

        add_instrument_to_cache(&algo);
        seed_quote(&algo, "100.00", "100.02");

        let mut exec_params = schedule_params("passive");
        exec_params.insert(Ustr::from("horizon_secs"), Ustr::from("60"));

        let order = create_limit_order(
            &algo,
            exec_params,
            OrderSide::Buy,
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
    fn test_quote_pegged_counts_report_submitted_and_cancelled() {
        let mut algo = create_quote_pegged_algorithm();
        register_algorithm(&mut algo);

        add_instrument_to_cache(&algo);
        seed_quote(&algo, "100.00", "100.02");

        let order = create_limit_order(
            &algo,
            schedule_params("passive"),
            OrderSide::Buy,
            Quantity::from("10"),
            Price::from("1000.00"),
        );
        let primary_id = order.client_order_id();

        algo.on_order(order).unwrap();
        assert_eq!(algo.counts(&primary_id), Some((1, 0)));

        let first = child(&algo, "O-001-E1");
        dispatch(&mut algo, canceled_event(&first));

        assert_eq!(algo.counts(&primary_id), Some((2, 1)));
    }

    #[rstest]
    fn test_quote_pegged_accepts_a_passive_preference_and_a_price_limit() {
        let mut algo = create_quote_pegged_algorithm();
        register_algorithm(&mut algo);

        add_instrument_to_cache(&algo);
        seed_quote(&algo, "100.00", "100.02");

        let mut exec_params = schedule_params("join");
        exec_params.insert(Ustr::from("preference"), Ustr::from("passive"));
        exec_params.insert(Ustr::from("price_limit"), Ustr::from("100.01"));
        exec_params.insert(Ustr::from("horizon_secs"), Ustr::from("60"));

        let order = create_limit_order(
            &algo,
            exec_params,
            OrderSide::Buy,
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
    fn test_quote_pegged_reset_clears_the_schedules() {
        let mut algo = create_quote_pegged_algorithm();
        register_algorithm(&mut algo);

        add_instrument_to_cache(&algo);
        seed_quote(&algo, "100.00", "100.02");

        let order = create_limit_order(
            &algo,
            schedule_params("passive"),
            OrderSide::Buy,
            Quantity::from("10"),
            Price::from("1000.00"),
        );

        algo.on_order(order).unwrap();
        assert!(!algo.schedules.is_empty());

        ExecutionAlgorithm::on_reset(&mut algo).unwrap();

        assert!(algo.schedules.is_empty());
    }
}
