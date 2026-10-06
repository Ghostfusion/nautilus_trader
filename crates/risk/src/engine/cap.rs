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

//! Risk caps: counts over a scope, a metric and a rolling window.
//!
//! A cap is a predicate, not a counter: the configuration declares which scope and metric it
//! covers and how many occurrences are allowed, and the state that makes the predicate decidable
//! lives beside it. A counter belongs to one rule over one concrete scope, so two caps that count
//! the same metric over the same scope with different windows are two rules and neither redefines
//! the other's window.
//!
//! A refusal is recorded as a [`RiskCapDecision`] and rendered to an
//! [`OrderDeniedReason`], so the string a log line shows is a rendering of a record rather than the
//! only form the refusal takes. Caps only constrain the actions that increase exposure: a
//! cancellation is never refused, because a cap that blocks reducing exposure is wrong.

use std::collections::VecDeque;

use ahash::{AHashMap, AHashSet};
use nautilus_common::cache::Cache;
use nautilus_core::{DurationNanos, UnixNanos};
use nautilus_model::{
    events::{OrderDeniedReason, OrderEventAny},
    identifiers::{AccountId, InstrumentId, StrategyId, Venue},
    orders::{Order, OrderAny},
    risk::{RiskCapMetric, RiskCapScope, RiskRequestKey},
    types::Currency,
};
use nautilus_portfolio::Portfolio;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// The number of most recent cap refusals an engine retains for observation.
pub const CAP_DECISION_HISTORY: usize = 64;

/// A configured cap: what it counts, over what scope, and how many are allowed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RiskCap {
    /// The metric counted.
    pub metric: RiskCapMetric,
    /// The kind of scope counted over.
    pub scope: RiskCapScope,
    /// The number of occurrences allowed before the cap refuses an action.
    ///
    /// Zero for a cap measured in quantity, which carries [`Self::quantity_limit`] instead.
    pub limit: u32,
    /// The quantity allowed before the cap refuses an action, in the instrument's units.
    ///
    /// Set for [`RiskCapMetric::Participation`] and [`RiskCapMetric::Inventory`], which measure a
    /// size rather than a count, so a configuration cannot read "no more than 1.5 BTC" as a number
    /// of events.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quantity_limit: Option<Decimal>,
    /// The money allowed before the cap refuses an action, in [`Self::money_currency`].
    ///
    /// Set for [`RiskCapMetric::NetExposure`], which measures money rather than occurrences or a
    /// size, so a configuration cannot read "no more than 1,000,000 USD" as a number of events and
    /// a limit without a currency cannot be compared against an exposure at all.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub money_limit: Option<Decimal>,
    /// The currency [`Self::money_limit`] is expressed in.
    ///
    /// Set for [`RiskCapMetric::NetExposure`] and `None` for every other metric, which measures in
    /// occurrences or in the instrument's units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub money_currency: Option<Currency>,
    /// The rolling window the count is taken over, in nanoseconds.
    ///
    /// `None` for [`RiskCapMetric::Active`], which counts the open order set as it stands rather
    /// than occurrences over a window, and for [`RiskCapMetric::Inventory`] and
    /// [`RiskCapMetric::NetExposure`], which read the standing position and the portfolio.
    pub window: Option<DurationNanos>,
}

impl RiskCap {
    /// Creates a new [`RiskCap`] instance.
    #[must_use]
    pub const fn new(
        metric: RiskCapMetric,
        scope: RiskCapScope,
        limit: u32,
        window: Option<DurationNanos>,
    ) -> Self {
        Self {
            metric,
            scope,
            limit,
            quantity_limit: None,
            money_limit: None,
            money_currency: None,
            window,
        }
    }

    /// Creates a new [`RiskCap`] instance measured in quantity.
    #[must_use]
    pub const fn new_quantity(
        metric: RiskCapMetric,
        scope: RiskCapScope,
        quantity_limit: Decimal,
        window: Option<DurationNanos>,
    ) -> Self {
        Self {
            metric,
            scope,
            limit: 0,
            quantity_limit: Some(quantity_limit),
            money_limit: None,
            money_currency: None,
            window,
        }
    }

    /// Creates a new [`RiskCap`] instance measured in money.
    #[must_use]
    pub const fn new_money(
        metric: RiskCapMetric,
        scope: RiskCapScope,
        money_limit: Decimal,
        money_currency: Currency,
    ) -> Self {
        Self {
            metric,
            scope,
            limit: 0,
            quantity_limit: None,
            money_limit: Some(money_limit),
            money_currency: Some(money_currency),
            window: None,
        }
    }

    /// Returns whether this cap is measured in quantity rather than in occurrences.
    #[must_use]
    pub const fn measures_quantity(&self) -> bool {
        self.quantity_limit.is_some()
    }

    /// Returns whether this cap is measured in money rather than in occurrences or quantity.
    #[must_use]
    pub const fn measures_money(&self) -> bool {
        self.money_limit.is_some()
    }

    /// Returns whether this cap counts occurrences of `action`.
    ///
    /// A cap that gates an action does not necessarily count it: a cancellation or fill cap
    /// gates the submissions that increase exposure but is fed by the cancellation and fill events
    /// themselves, so what gets recorded is determined here rather than by [`Self::gates`]. A
    /// repeated request cap counts submissions, so an admitted submission feeds it.
    #[must_use]
    pub const fn counts(&self, action: RiskCapMetric) -> bool {
        match self.metric {
            RiskCapMetric::Active | RiskCapMetric::Inventory | RiskCapMetric::NetExposure => false,
            RiskCapMetric::Submit | RiskCapMetric::RepeatedRequest => {
                matches!(action, RiskCapMetric::Submit)
            }
            RiskCapMetric::Modify => matches!(action, RiskCapMetric::Modify),
            RiskCapMetric::Cancel => matches!(action, RiskCapMetric::Cancel),
            RiskCapMetric::Fill | RiskCapMetric::Participation => {
                matches!(action, RiskCapMetric::Fill)
            }
        }
    }

    /// Returns whether this cap constrains `action`.
    ///
    /// A cap on a metric that cannot itself be refused - a cancellation or a fill - constrains the
    /// submissions that increase exposure instead, and no cap ever constrains a cancellation.
    #[must_use]
    pub const fn gates(&self, action: RiskCapMetric) -> bool {
        match action {
            RiskCapMetric::Submit => matches!(
                self.metric,
                RiskCapMetric::Submit
                    | RiskCapMetric::Cancel
                    | RiskCapMetric::Fill
                    | RiskCapMetric::Active
                    | RiskCapMetric::RepeatedRequest
                    | RiskCapMetric::Participation
                    | RiskCapMetric::Inventory
                    | RiskCapMetric::NetExposure
            ),
            RiskCapMetric::Modify => matches!(
                self.metric,
                RiskCapMetric::Modify
                    | RiskCapMetric::Active
                    | RiskCapMetric::Inventory
                    | RiskCapMetric::NetExposure
            ),
            RiskCapMetric::Cancel
            | RiskCapMetric::Fill
            | RiskCapMetric::Active
            | RiskCapMetric::Inventory
            | RiskCapMetric::Participation
            | RiskCapMetric::RepeatedRequest
            | RiskCapMetric::NetExposure => false,
        }
    }
}

/// The concrete entity a scoped counter counts.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RiskScopeKey {
    /// Every order.
    Global,
    /// One strategy.
    Strategy(StrategyId),
    /// One account.
    Account(AccountId),
    /// One instrument.
    Instrument(InstrumentId),
    /// One venue.
    Venue(Venue),
    /// One strategy in one instrument.
    StrategyInstrument(StrategyId, InstrumentId),
}

impl RiskScopeKey {
    /// Returns the scope kind of this key.
    #[must_use]
    pub const fn scope(&self) -> RiskCapScope {
        match self {
            Self::Global => RiskCapScope::Global,
            Self::Strategy(_) => RiskCapScope::Strategy,
            Self::Account(_) => RiskCapScope::Account,
            Self::Instrument(_) => RiskCapScope::Instrument,
            Self::Venue(_) => RiskCapScope::Venue,
            Self::StrategyInstrument(_, _) => RiskCapScope::StrategyInstrument,
        }
    }
}

impl std::fmt::Display for RiskScopeKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Global => write!(f, "GLOBAL"),
            Self::Strategy(strategy_id) => write!(f, "STRATEGY({strategy_id})"),
            Self::Account(account_id) => write!(f, "ACCOUNT({account_id})"),
            Self::Instrument(instrument_id) => write!(f, "INSTRUMENT({instrument_id})"),
            Self::Venue(venue) => write!(f, "VENUE({venue})"),
            Self::StrategyInstrument(strategy_id, instrument_id) => {
                write!(f, "STRATEGY_INSTRUMENT({strategy_id}, {instrument_id})")
            }
        }
    }
}

/// The scope values of one order or order event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RiskSubject {
    /// The strategy the order belongs to.
    pub strategy_id: StrategyId,
    /// The instrument the order trades.
    pub instrument_id: InstrumentId,
    /// The account the order is routed to, when it declares one.
    pub account_id: Option<AccountId>,
}

impl RiskSubject {
    /// Creates a new [`RiskSubject`] instance.
    #[must_use]
    pub const fn new(
        strategy_id: StrategyId,
        instrument_id: InstrumentId,
        account_id: Option<AccountId>,
    ) -> Self {
        Self {
            strategy_id,
            instrument_id,
            account_id,
        }
    }

    /// Returns the scope key of this subject for `scope`.
    ///
    /// Returns `None` when a cap scoped to an account does not apply, because the subject declares
    /// no account, so such a cap neither counts nor refuses for it.
    #[must_use]
    pub fn key(&self, scope: RiskCapScope) -> Option<RiskScopeKey> {
        match scope {
            RiskCapScope::Global => Some(RiskScopeKey::Global),
            RiskCapScope::Strategy => Some(RiskScopeKey::Strategy(self.strategy_id)),
            RiskCapScope::Account => self.account_id.map(RiskScopeKey::Account),
            RiskCapScope::Instrument => Some(RiskScopeKey::Instrument(self.instrument_id)),
            RiskCapScope::Venue => Some(RiskScopeKey::Venue(self.instrument_id.venue)),
            RiskCapScope::StrategyInstrument => Some(RiskScopeKey::StrategyInstrument(
                self.strategy_id,
                self.instrument_id,
            )),
        }
    }
}

impl From<&OrderAny> for RiskSubject {
    fn from(order: &OrderAny) -> Self {
        Self::new(
            order.strategy_id(),
            order.instrument_id(),
            order.account_id(),
        )
    }
}

impl From<&OrderEventAny> for RiskSubject {
    fn from(event: &OrderEventAny) -> Self {
        Self::new(
            event.strategy_id(),
            event.instrument_id(),
            event.account_id(),
        )
    }
}

/// The identity of one counter: a cap's dimensions over one concrete scope.
///
/// A repeated request cap adds the canonical request identity, so each request shape holds its own
/// counter rather than sharing one cumulative count.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RiskCounterKey {
    /// The metric counted.
    pub metric: RiskCapMetric,
    /// The kind of scope counted over.
    pub scope: RiskCapScope,
    /// The rolling window the count is taken over.
    pub window: Option<DurationNanos>,
    /// The concrete scope counted.
    pub subject: RiskScopeKey,
    /// The canonical request identity, for a cap that counts repeated requests.
    pub request: Option<RiskRequestKey>,
}

impl RiskCounterKey {
    /// Creates the counter key of `cap` over `subject`.
    ///
    /// The request identity is carried only by a repeated request cap; every other metric counts
    /// occurrences of the scope alone, so the key cannot separate two windows over one scope.
    #[must_use]
    pub fn new(cap: &RiskCap, subject: RiskScopeKey, request: Option<&RiskRequestKey>) -> Self {
        let request = match cap.metric {
            RiskCapMetric::RepeatedRequest => request.cloned(),
            _ => None,
        };

        Self {
            metric: cap.metric,
            scope: cap.scope,
            window: cap.window,
            subject,
            request,
        }
    }
}

/// Rolling occurrence counters, one per counter key.
#[derive(Debug, Default)]
pub struct RiskCounters {
    occurrences: AHashMap<RiskCounterKey, VecDeque<UnixNanos>>,
    volumes: AHashMap<RiskCounterKey, VecDeque<(UnixNanos, Decimal)>>,
}

impl RiskCounters {
    /// Records one occurrence of `key` at `ts_event`.
    ///
    /// Occurrences outside the window are dropped as they are seen, so a counter holds only the
    /// occurrences that can still count.
    pub fn record(&mut self, key: RiskCounterKey, ts_event: UnixNanos) {
        let window = key.window;
        let occurrences = self.occurrences.entry(key).or_default();
        occurrences.push_back(ts_event);
        Self::expire(occurrences, ts_event, window);
    }

    /// Removes the most recent occurrence of `key`, for a fill that was voided.
    pub fn void_last(&mut self, key: &RiskCounterKey, ts_event: UnixNanos) {
        let Some(occurrences) = self.occurrences.get_mut(key) else {
            return;
        };

        occurrences.pop_back();
        Self::expire(occurrences, ts_event, key.window);
    }

    /// Returns the number of occurrences of `key` within the window ending at `ts_event`.
    pub fn count(&mut self, key: &RiskCounterKey, ts_event: UnixNanos) -> u32 {
        let Some(occurrences) = self.occurrences.get_mut(key) else {
            return 0;
        };

        Self::expire(occurrences, ts_event, key.window);
        u32::try_from(occurrences.len()).unwrap_or(u32::MAX)
    }

    /// Clears every counter.
    pub fn clear(&mut self) {
        self.occurrences.clear();
        self.volumes.clear();
    }

    /// Records one quantity of `key` at `ts_event`.
    ///
    /// Quantities outside the window are dropped as they are seen, in the same way occurrences
    /// are, so a counter holds only what can still count.
    pub fn record_volume(&mut self, key: RiskCounterKey, ts_event: UnixNanos, quantity: Decimal) {
        let window = key.window;
        let volumes = self.volumes.entry(key).or_default();
        volumes.push_back((ts_event, quantity));
        Self::expire_volumes(volumes, ts_event, window);
    }

    /// Removes the most recent quantity of `key`, for a fill that was voided.
    pub fn void_last_volume(&mut self, key: &RiskCounterKey, ts_event: UnixNanos) {
        let Some(volumes) = self.volumes.get_mut(key) else {
            return;
        };

        volumes.pop_back();
        Self::expire_volumes(volumes, ts_event, key.window);
    }

    /// Returns the quantity of `key` within the window ending at `ts_event`.
    pub fn volume(&mut self, key: &RiskCounterKey, ts_event: UnixNanos) -> Decimal {
        let Some(volumes) = self.volumes.get_mut(key) else {
            return Decimal::ZERO;
        };

        Self::expire_volumes(volumes, ts_event, key.window);

        // A sum that cannot be represented saturates, so an observed quantity can overstate a cap
        // and never understate one.
        volumes.iter().fold(Decimal::ZERO, |total, (_, quantity)| {
            total.checked_add(*quantity).unwrap_or(Decimal::MAX)
        })
    }

    /// Drops the quantities that no longer fall inside the window ending at `ts_event`.
    fn expire_volumes(
        volumes: &mut VecDeque<(UnixNanos, Decimal)>,
        ts_event: UnixNanos,
        window: Option<DurationNanos>,
    ) {
        let Some(window) = window else {
            return;
        };

        volumes.retain(|(ts, _)| ts_event.saturating_duration_since(*ts) < window);
    }

    /// Drops the occurrences that no longer fall inside the window ending at `ts_event`.
    ///
    /// The window is half open: an occurrence exactly one window old no longer counts. An
    /// occurrence is dropped as it is seen rather than recomputed, so a query at an earlier
    /// `ts_event` than a previous query sees only what that query kept.
    fn expire(
        occurrences: &mut VecDeque<UnixNanos>,
        ts_event: UnixNanos,
        window: Option<DurationNanos>,
    ) {
        let Some(window) = window else {
            return;
        };

        occurrences.retain(|ts| ts_event.saturating_duration_since(*ts) < window);
    }
}

/// The immutable record of one cap refusal.
///
/// The record is the source of the refusal: [`Self::reason`] renders it to the typed denial, and
/// an engine retains its most recent records so a refusal is observable as data and not only as
/// the string a log line shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RiskCapDecision {
    /// The metric the reached cap counts.
    pub metric: RiskCapMetric,
    /// The scope kind the reached cap counts over.
    pub scope: RiskCapScope,
    /// The concrete scope the reached cap counts.
    pub subject: RiskScopeKey,
    /// The canonical request identity, for a cap that counts repeated requests.
    pub request: Option<RiskRequestKey>,
    /// The count observed.
    ///
    /// Zero for a decision measured in quantity, which carries [`Self::observed_quantity`].
    pub observed: u32,
    /// The configured limit.
    ///
    /// Zero for a decision measured in quantity, which carries [`Self::limit_quantity`].
    pub limit: u32,
    /// The quantity observed, for a metric measured in quantity.
    pub observed_quantity: Option<Decimal>,
    /// The configured quantity limit, for a metric measured in quantity.
    pub limit_quantity: Option<Decimal>,
    /// The money observed, for a metric measured in money.
    ///
    /// `None` on a money metric means the portfolio could not value the aggregate, so the refusal
    /// records an unanswerable limit rather than a limit that was reached.
    pub observed_money: Option<Decimal>,
    /// The configured money limit, for a metric measured in money.
    pub limit_money: Option<Decimal>,
    /// The currency the money measurements are expressed in, for a metric measured in money.
    pub money_currency: Option<Currency>,
    /// The rolling window the count was taken over.
    pub window: Option<DurationNanos>,
    /// The timestamp the refusal was evaluated at.
    pub ts_event: UnixNanos,
}

impl RiskCapDecision {
    /// Returns the typed denial this record renders to.
    #[must_use]
    pub fn reason(&self) -> OrderDeniedReason {
        match self.metric {
            RiskCapMetric::Active => OrderDeniedReason::ActiveOrderLimitReached {
                scope: self.scope,
                observed: self.observed,
                limit: self.limit,
            },
            RiskCapMetric::RepeatedRequest => match &self.request {
                Some(request) => OrderDeniedReason::RepeatedRequestLimitReached {
                    scope: self.scope,
                    request: request.clone(),
                    observed: self.observed,
                    limit: self.limit,
                    window_ns: self.window.map_or(0, |window| window.as_u64()),
                },
                // A repeated request refusal is only produced with an identity; the fallback keeps
                // the rendering total rather than panicking on the engine's send path.
                None => OrderDeniedReason::OrderCountLimitReached {
                    metric: self.metric,
                    scope: self.scope,
                    observed: self.observed,
                    limit: self.limit,
                    window_ns: self.window.map_or(0, |window| window.as_u64()),
                },
            },
            RiskCapMetric::Participation => OrderDeniedReason::ParticipationLimitReached {
                scope: self.scope,
                // A decision measured in quantity always carries both, so the fallback keeps the
                // rendering total rather than panicking on the engine's send path.
                observed: self.observed_quantity.unwrap_or_default(),
                limit: self.limit_quantity.unwrap_or_default(),
                window_ns: self.window.map_or(0, |window| window.as_u64()),
            },
            RiskCapMetric::Inventory => OrderDeniedReason::InventoryLimitReached {
                scope: self.scope,
                observed: self.observed_quantity.unwrap_or_default(),
                limit: self.limit_quantity.unwrap_or_default(),
            },
            RiskCapMetric::NetExposure => match self.observed_money {
                Some(observed) => OrderDeniedReason::ExposureLimitReached {
                    scope: self.scope,
                    // A decision measured in money always carries the currency, so the fallback
                    // keeps the rendering total rather than panicking on the engine's send path.
                    currency: self.money_currency.unwrap_or_else(Currency::USD),
                    observed,
                    limit: self.limit_money.unwrap_or_default(),
                },
                None => OrderDeniedReason::ExposureLimitUnknown {
                    scope: self.scope,
                    currency: self.money_currency.unwrap_or_else(Currency::USD),
                },
            },
            metric => OrderDeniedReason::OrderCountLimitReached {
                metric,
                scope: self.scope,
                observed: self.observed,
                limit: self.limit,
                window_ns: self.window.map_or(0, |window| window.as_u64()),
            },
        }
    }
}

/// Evaluates every cap that gates `action` for `subject`, returning the first refusal.
///
/// Caps are evaluated in configuration order, so the reported refusal is deterministic. The first
/// refusal is returned rather than every refusal, because the caller refuses once and the record
/// names the rule that stopped it.
///
/// A cap that counts repeated requests is counted against the identity of the request being
/// evaluated, so a caller that has no identity for the action skips that cap rather than counting
/// every request together.
#[expect(
    clippy::too_many_arguments,
    reason = "a cap is decidable against the counters, the cache and the portfolio"
)]
pub fn evaluate(
    caps: &[RiskCap],
    counters: &mut RiskCounters,
    cache: &Cache,
    portfolio: &Portfolio,
    action: RiskCapMetric,
    subject: &RiskSubject,
    request: Option<&RiskRequestKey>,
    ts_event: UnixNanos,
) -> Option<RiskCapDecision> {
    for cap in caps {
        if !cap.gates(action) {
            continue;
        }

        if cap.metric == RiskCapMetric::RepeatedRequest && request.is_none() {
            continue;
        }

        let subject_key = subject.key(cap.scope)?;

        if let Some(money_limit) = cap.money_limit {
            let currency = cap.money_currency.unwrap_or_else(Currency::USD);
            let observed = portfolio_exposure(portfolio, cache, cap.scope, subject, currency);

            // An unanswerable limit is refused rather than read as zero, because a limit that
            // cannot be evaluated must not be treated as one that was not reached.
            if observed.is_none_or(|observed| observed >= money_limit) {
                return Some(RiskCapDecision {
                    metric: cap.metric,
                    scope: cap.scope,
                    subject: subject_key,
                    request: None,
                    observed: 0,
                    limit: 0,
                    observed_quantity: None,
                    limit_quantity: None,
                    observed_money: observed,
                    limit_money: Some(money_limit),
                    money_currency: Some(currency),
                    window: cap.window,
                    ts_event,
                });
            }

            continue;
        }

        if let Some(quantity_limit) = cap.quantity_limit {
            let observed = if cap.metric == RiskCapMetric::Inventory {
                inventory_quantity(cache, cap.scope, subject)
            } else {
                counters.volume(
                    &RiskCounterKey::new(cap, subject_key.clone(), request),
                    ts_event,
                )
            };

            if observed >= quantity_limit {
                return Some(RiskCapDecision {
                    metric: cap.metric,
                    scope: cap.scope,
                    subject: subject_key,
                    request: None,
                    observed: 0,
                    limit: 0,
                    observed_quantity: Some(observed),
                    limit_quantity: Some(quantity_limit),
                    observed_money: None,
                    limit_money: None,
                    money_currency: None,
                    window: cap.window,
                    ts_event,
                });
            }

            continue;
        }

        let observed = if cap.metric == RiskCapMetric::Active {
            active_order_count(cache, cap.scope, subject)
        } else {
            counters.count(
                &RiskCounterKey::new(cap, subject_key.clone(), request),
                ts_event,
            )
        };

        if observed >= cap.limit {
            let request = match cap.metric {
                RiskCapMetric::RepeatedRequest => request.cloned(),
                _ => None,
            };

            return Some(RiskCapDecision {
                metric: cap.metric,
                scope: cap.scope,
                subject: subject_key,
                request,
                observed,
                limit: cap.limit,
                observed_quantity: None,
                limit_quantity: None,
                observed_money: None,
                limit_money: None,
                money_currency: None,
                window: cap.window,
                ts_event,
            });
        }
    }

    None
}

/// Returns the absolute position size `subject` holds in the instrument it is trading.
///
/// Every selector reads the same instrument, because a size summed across instruments is not a
/// size; what the selector changes is whose position is read. A closed or absent position observes
/// zero, which is the honest reading of a scope that holds nothing.
fn inventory_quantity(cache: &Cache, selector: RiskCapScope, subject: &RiskSubject) -> Decimal {
    let strategy_id = &subject.strategy_id;
    let instrument_id = &subject.instrument_id;
    let account_id = subject.account_id.as_ref();
    let venue = &subject.instrument_id.venue;

    let positions = match selector {
        RiskCapScope::Global | RiskCapScope::Instrument => {
            cache.positions(None, Some(instrument_id), None, None, None)
        }
        RiskCapScope::Strategy | RiskCapScope::StrategyInstrument => {
            cache.positions(None, Some(instrument_id), Some(strategy_id), None, None)
        }
        RiskCapScope::Account => match account_id {
            Some(account_id) => {
                cache.positions(None, Some(instrument_id), None, Some(account_id), None)
            }
            None => return Decimal::ZERO,
        },
        RiskCapScope::Venue => cache.positions(Some(venue), Some(instrument_id), None, None, None),
    };

    positions
        .iter()
        .map(|position| position.quantity.as_decimal().abs())
        .sum()
}

/// Returns the portfolio's net exposure in `currency`, aggregated over the scope `subject` names.
///
/// Every venue that holds an open position is asked, so the aggregate spans venues as well as
/// strategies: the same aggregation the portfolio publishes is the one enforced here, rather than a
/// second reading of the positions that could disagree with it. A venue whose totals do not name
/// `currency` contributes nothing, because that is a venue holding nothing in it, but a venue the
/// portfolio cannot value at all leaves the aggregate unknown rather than partial.
fn portfolio_exposure(
    portfolio: &Portfolio,
    cache: &Cache,
    selector: RiskCapScope,
    subject: &RiskSubject,
    currency: Currency,
) -> Option<Decimal> {
    let account_id = match selector {
        RiskCapScope::Account => subject.account_id.as_ref(),
        _ => None,
    };

    let venues: AHashSet<Venue> = cache
        .positions(None, None, None, None, None)
        .iter()
        .map(|position| position.instrument_id.venue)
        .collect();

    let mut total = Decimal::ZERO;

    for venue in venues {
        let exposures = portfolio.net_exposures(&venue, account_id, None)?;

        match exposures.get(&currency) {
            Some(exposure) => total = total.checked_add(exposure.as_decimal())?,
            // A venue with no exposure in any currency contributes nothing. A venue with exposure
            // in another currency leaves the aggregate unknown rather than dropping it: a total
            // read from part of the portfolio is not the portfolio's exposure.
            None if exposures.is_empty() => {}
            None => return None,
        }
    }

    Some(total)
}

/// Returns the number of open orders `subject` has for a cap scoped to `selector`.
///
/// The count is the open order set, so an order that is partially filled still counts once and an
/// order that is cancelled or filled releases the capacity it held.
fn active_order_count(cache: &Cache, selector: RiskCapScope, subject: &RiskSubject) -> u32 {
    let strategy_id = &subject.strategy_id;
    let instrument_id = &subject.instrument_id;
    let account_id = subject.account_id.as_ref();
    let venue = &subject.instrument_id.venue;

    let count = match selector {
        RiskCapScope::Global => cache.orders_open_count(None, None, None, None, None),
        RiskCapScope::Strategy => {
            cache.orders_open_count(None, None, Some(strategy_id), None, None)
        }
        RiskCapScope::Account => match account_id {
            Some(account_id) => cache.orders_open_count(None, None, None, Some(account_id), None),
            None => return 0,
        },
        RiskCapScope::Instrument => {
            cache.orders_open_count(None, Some(instrument_id), None, None, None)
        }
        RiskCapScope::Venue => cache.orders_open_count(Some(venue), None, None, None, None),
        RiskCapScope::StrategyInstrument => {
            cache.orders_open_count(None, Some(instrument_id), Some(strategy_id), None, None)
        }
    };

    u32::try_from(count).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use nautilus_common::clock::{Clock, VirtualClock};
    use nautilus_core::{UUID4, UnixNanos};
    use nautilus_model::{
        enums::{OrderSide, OrderType},
        identifiers::ClientOrderId,
        orders::OrderTestBuilder,
        types::{Price, Quantity},
    };
    use rstest::rstest;
    use rust_decimal_macros::dec;

    use super::*;

    /// Evaluates caps against a portfolio that holds nothing.
    ///
    /// A test whose cap is measured in occurrences, quantity or standing inventory does not read
    /// the portfolio, so it is given one over an empty cache rather than restating the parameter.
    fn evaluate(
        caps: &[RiskCap],
        counters: &mut RiskCounters,
        cache: &Cache,
        action: RiskCapMetric,
        subject: &RiskSubject,
        request: Option<&RiskRequestKey>,
        ts_event: UnixNanos,
    ) -> Option<RiskCapDecision> {
        let portfolio_cache = Rc::new(RefCell::new(Cache::default()));
        let clock = Rc::new(RefCell::new(VirtualClock::new()));
        let portfolio = Portfolio::new(
            Rc::clone(&clock) as Rc<RefCell<dyn Clock>>,
            portfolio_cache,
            None,
        );

        super::evaluate(
            caps, counters, cache, &portfolio, action, subject, request, ts_event,
        )
    }

    #[rstest]
    fn test_participation_cap_sums_fills_over_the_window_and_denies() {
        use nautilus_common::cache::Cache;
        use rust_decimal_macros::dec;

        let scope = RiskCapScope::StrategyInstrument;
        let cap = RiskCap::new_quantity(
            RiskCapMetric::Participation,
            scope,
            dec!(1.5),
            Some(window_secs(60)),
        );
        let subject = subject(true);
        let mut counters = RiskCounters::default();
        let cache = Cache::default();
        let key = RiskCounterKey::new(&cap, subject.key(scope).unwrap(), None);

        counters.record_volume(key.clone(), UnixNanos::from(1_000_000_000), dec!(1.0));
        counters.record_volume(key.clone(), UnixNanos::from(2_000_000_000), dec!(0.25));

        // 1.25 is inside the 1.5 budget, so nothing is refused.
        assert_eq!(
            counters.volume(&key, UnixNanos::from(2_000_000_000)),
            dec!(1.25)
        );
        assert!(
            evaluate(
                std::slice::from_ref(&cap),
                &mut counters,
                &cache,
                RiskCapMetric::Submit,
                &subject,
                None,
                UnixNanos::from(2_000_000_000),
            )
            .is_none()
        );

        // A third fill takes the window to 1.75, past the budget, and the refusal names it.
        counters.record_volume(key.clone(), UnixNanos::from(3_000_000_000), dec!(0.5));
        let decision = evaluate(
            std::slice::from_ref(&cap),
            &mut counters,
            &cache,
            RiskCapMetric::Submit,
            &subject,
            None,
            UnixNanos::from(3_000_000_000),
        )
        .expect("the participation budget should be reached");

        assert_eq!(decision.observed_quantity, Some(dec!(1.75)));
        assert_eq!(decision.limit_quantity, Some(dec!(1.5)));

        let rendered = decision.reason().to_string();
        assert!(
            rendered.starts_with("PARTICIPATION_LIMIT_REACHED:")
                && rendered.contains("observed=1.75")
                && rendered.contains("limit=1.5"),
            "{rendered}"
        );

        // The window expires the oldest fills: at 62 s only the last 0.5 remains.
        assert!(
            evaluate(
                std::slice::from_ref(&cap),
                &mut counters,
                &cache,
                RiskCapMetric::Submit,
                &subject,
                None,
                UnixNanos::from(62_000_000_000),
            )
            .is_none()
        );
        assert_eq!(
            counters.volume(&key, UnixNanos::from(62_000_000_000)),
            dec!(0.5)
        );

        // A voided fill releases the quantity it recorded.
        counters.void_last_volume(&key, UnixNanos::from(62_000_000_000));
        assert_eq!(
            counters.volume(&key, UnixNanos::from(62_000_000_000)),
            dec!(0.0)
        );
    }

    #[rstest]
    fn test_inventory_cap_reads_the_open_position_and_denies() {
        use nautilus_common::cache::Cache;
        use nautilus_model::{
            enums::OmsType,
            events::OrderEventAny,
            instruments::{Instrument, InstrumentAny, stubs::audusd_sim},
            orders::stubs::TestOrderEventStubs,
            position::Position,
        };
        use rust_decimal_macros::dec;

        let instrument = InstrumentAny::CurrencyPair(audusd_sim());
        let entry = OrderTestBuilder::new(OrderType::Market)
            .instrument_id(instrument.id())
            .side(OrderSide::Buy)
            .quantity(Quantity::from("5000"))
            .build();
        let filled = TestOrderEventStubs::filled(
            &entry,
            &instrument,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let OrderEventAny::Filled(filled) = filled else {
            panic!("expected a filled order event")
        };
        let position = Position::new(&instrument, filled);
        assert_eq!(position.quantity, Quantity::from("5000"));

        let mut cache = Cache::default();
        cache.add_instrument(instrument).unwrap();
        cache.add_position(&position, OmsType::Netting).unwrap();

        let scope = RiskCapScope::Instrument;
        let subject = subject(true);
        let mut counters = RiskCounters::default();

        let cap = RiskCap::new_quantity(RiskCapMetric::Inventory, scope, dec!(1000), None);
        let decision = evaluate(
            std::slice::from_ref(&cap),
            &mut counters,
            &cache,
            RiskCapMetric::Submit,
            &subject,
            None,
            UnixNanos::from(1),
        )
        .expect("a position above the inventory limit should be refused");

        assert_eq!(decision.observed_quantity, Some(dec!(5000)));
        assert_eq!(decision.limit_quantity, Some(dec!(1000)));

        let rendered = decision.reason().to_string();
        assert!(
            rendered.starts_with("INVENTORY_LIMIT_REACHED:") && rendered.contains("observed=5000"),
            "{rendered}"
        );

        // The same inventory passes a limit it does not exceed, so the cap reads the size rather
        // than refusing every order. A limit exactly equal to the inventory is refused, because
        // every cap in this vocabulary treats its limit as inclusive.
        let cap = RiskCap::new_quantity(RiskCapMetric::Inventory, scope, dec!(6000), None);
        assert!(
            evaluate(
                std::slice::from_ref(&cap),
                &mut counters,
                &cache,
                RiskCapMetric::Submit,
                &subject,
                None,
                UnixNanos::from(1),
            )
            .is_none()
        );
    }

    fn subject(account: bool) -> RiskSubject {
        RiskSubject::new(
            StrategyId::from("S-1"),
            InstrumentId::from("AUD/USD.SIM"),
            account.then(|| AccountId::from("ACC-1")),
        )
    }

    fn window_secs(secs: u64) -> DurationNanos {
        DurationNanos::from_secs(secs)
    }

    #[rstest]
    #[case(RiskCapMetric::Submit, RiskCapMetric::Submit, true)]
    #[case(RiskCapMetric::Cancel, RiskCapMetric::Submit, true)]
    #[case(RiskCapMetric::Fill, RiskCapMetric::Submit, true)]
    #[case(RiskCapMetric::Active, RiskCapMetric::Submit, true)]
    #[case(RiskCapMetric::RepeatedRequest, RiskCapMetric::Submit, true)]
    #[case(RiskCapMetric::RepeatedRequest, RiskCapMetric::Modify, false)]
    #[case(RiskCapMetric::RepeatedRequest, RiskCapMetric::RepeatedRequest, false)]
    #[case(RiskCapMetric::Submit, RiskCapMetric::RepeatedRequest, false)]
    #[case(RiskCapMetric::Modify, RiskCapMetric::Submit, false)]
    #[case(RiskCapMetric::Modify, RiskCapMetric::Modify, true)]
    #[case(RiskCapMetric::Active, RiskCapMetric::Modify, true)]
    #[case(RiskCapMetric::Submit, RiskCapMetric::Modify, false)]
    #[case(RiskCapMetric::Cancel, RiskCapMetric::Cancel, false)]
    #[case(RiskCapMetric::Fill, RiskCapMetric::Fill, false)]
    #[case(RiskCapMetric::Active, RiskCapMetric::Active, false)]
    fn test_only_increasing_actions_are_gated(
        #[case] metric: RiskCapMetric,
        #[case] action: RiskCapMetric,
        #[case] expected: bool,
    ) {
        let cap = RiskCap::new(metric, RiskCapScope::Global, 10, Some(window_secs(60)));

        assert_eq!(cap.gates(action), expected);
    }

    #[rstest]
    #[case(RiskCapMetric::Submit, RiskCapMetric::Submit, true)]
    #[case(RiskCapMetric::RepeatedRequest, RiskCapMetric::Submit, true)]
    #[case(RiskCapMetric::Cancel, RiskCapMetric::Submit, false)]
    #[case(RiskCapMetric::Fill, RiskCapMetric::Submit, false)]
    #[case(RiskCapMetric::Active, RiskCapMetric::Submit, false)]
    #[case(RiskCapMetric::Modify, RiskCapMetric::Modify, true)]
    #[case(RiskCapMetric::Submit, RiskCapMetric::Modify, false)]
    #[case(RiskCapMetric::Cancel, RiskCapMetric::Cancel, true)]
    #[case(RiskCapMetric::Fill, RiskCapMetric::Fill, true)]
    #[case(RiskCapMetric::RepeatedRequest, RiskCapMetric::Fill, false)]
    fn test_only_the_counted_action_is_recorded(
        #[case] metric: RiskCapMetric,
        #[case] action: RiskCapMetric,
        #[case] expected: bool,
    ) {
        let cap = RiskCap::new(metric, RiskCapScope::Global, 10, Some(window_secs(60)));

        assert_eq!(cap.counts(action), expected);
    }

    #[rstest]
    fn test_subject_keys_cover_every_scope() {
        let subject = subject(true);

        assert_eq!(
            subject.key(RiskCapScope::Global),
            Some(RiskScopeKey::Global)
        );
        assert_eq!(
            subject.key(RiskCapScope::Strategy),
            Some(RiskScopeKey::Strategy(StrategyId::from("S-1")))
        );
        assert_eq!(
            subject.key(RiskCapScope::Account),
            Some(RiskScopeKey::Account(AccountId::from("ACC-1")))
        );
        assert_eq!(
            subject.key(RiskCapScope::Instrument),
            Some(RiskScopeKey::Instrument(InstrumentId::from("AUD/USD.SIM")))
        );
        assert_eq!(
            subject.key(RiskCapScope::Venue),
            Some(RiskScopeKey::Venue(Venue::from("SIM")))
        );
        assert_eq!(
            subject.key(RiskCapScope::StrategyInstrument),
            Some(RiskScopeKey::StrategyInstrument(
                StrategyId::from("S-1"),
                InstrumentId::from("AUD/USD.SIM")
            ))
        );
    }

    #[rstest]
    fn test_an_account_cap_does_not_apply_without_an_account() {
        let subject = subject(false);

        assert_eq!(subject.key(RiskCapScope::Account), None);
        assert!(subject.key(RiskCapScope::Global).is_some());
    }

    #[rstest]
    fn test_scope_key_round_trips_its_scope() {
        let subject = subject(true);

        for scope in [
            RiskCapScope::Global,
            RiskCapScope::Strategy,
            RiskCapScope::Account,
            RiskCapScope::Instrument,
            RiskCapScope::Venue,
            RiskCapScope::StrategyInstrument,
        ] {
            let key = subject.key(scope).unwrap();

            assert_eq!(key.scope(), scope);
        }
    }

    #[rstest]
    fn test_occurrences_expire_at_the_window_boundary() {
        let subject = subject(true);
        let cap = RiskCap::new(
            RiskCapMetric::Submit,
            RiskCapScope::Global,
            2,
            Some(window_secs(60)),
        );
        let key = RiskCounterKey::new(&cap, subject.key(RiskCapScope::Global).unwrap(), None);
        let mut counters = RiskCounters::default();

        counters.record(key.clone(), UnixNanos::from_seconds(0));
        counters.record(key.clone(), UnixNanos::from_seconds(30));

        // Half open: an occurrence exactly one window old no longer counts. The queries ascend,
        // because an occurrence outside the window is dropped as it is seen and never returns.
        assert_eq!(counters.count(&key, UnixNanos::from_seconds(59)), 2);
        assert_eq!(counters.count(&key, UnixNanos::from_seconds(60)), 1);
        assert_eq!(counters.count(&key, UnixNanos::from_seconds(89)), 1);
        assert_eq!(counters.count(&key, UnixNanos::from_seconds(90)), 0);
    }

    #[rstest]
    fn test_a_voided_fill_releases_its_occurrence() {
        let subject = subject(true);
        let cap = RiskCap::new(
            RiskCapMetric::Fill,
            RiskCapScope::Global,
            2,
            Some(window_secs(60)),
        );
        let key = RiskCounterKey::new(&cap, subject.key(RiskCapScope::Global).unwrap(), None);
        let mut counters = RiskCounters::default();

        counters.record(key.clone(), UnixNanos::from_seconds(1));
        counters.record(key.clone(), UnixNanos::from_seconds(2));
        assert_eq!(counters.count(&key, UnixNanos::from_seconds(3)), 2);

        counters.void_last(&key, UnixNanos::from_seconds(3));
        assert_eq!(counters.count(&key, UnixNanos::from_seconds(3)), 1);
    }

    #[rstest]
    fn test_counters_are_per_rule_and_per_scope() {
        let cap = RiskCap::new(
            RiskCapMetric::Submit,
            RiskCapScope::Strategy,
            1,
            Some(window_secs(60)),
        );
        let global = RiskCounterKey::new(&cap, RiskScopeKey::Global, None);
        let strategy =
            RiskCounterKey::new(&cap, RiskScopeKey::Strategy(StrategyId::from("S-1")), None);
        let mut counters = RiskCounters::default();

        counters.record(global.clone(), UnixNanos::from_seconds(1));

        assert_eq!(counters.count(&global, UnixNanos::from_seconds(2)), 1);
        assert_eq!(counters.count(&strategy, UnixNanos::from_seconds(2)), 0);
    }

    #[rstest]
    fn test_evaluate_refuses_at_the_limit_and_records_the_rule() {
        let cache = Cache::default();
        let subject = subject(true);
        let cap = RiskCap::new(
            RiskCapMetric::Submit,
            RiskCapScope::Global,
            2,
            Some(window_secs(60)),
        );
        let caps = vec![cap.clone()];
        let key = RiskCounterKey::new(&cap, RiskScopeKey::Global, None);
        let mut counters = RiskCounters::default();
        let ts = UnixNanos::from_seconds(60);

        assert!(
            evaluate(
                &caps,
                &mut counters,
                &cache,
                RiskCapMetric::Submit,
                &subject,
                None,
                ts
            )
            .is_none()
        );

        counters.record(key.clone(), UnixNanos::from_seconds(1));

        assert!(
            evaluate(
                &caps,
                &mut counters,
                &cache,
                RiskCapMetric::Submit,
                &subject,
                None,
                ts
            )
            .is_none()
        );

        counters.record(key, UnixNanos::from_seconds(2));
        let decision = evaluate(
            &caps,
            &mut counters,
            &cache,
            RiskCapMetric::Submit,
            &subject,
            None,
            ts,
        )
        .expect("the cap is reached");

        assert_eq!(decision.metric, RiskCapMetric::Submit);
        assert_eq!(decision.scope, RiskCapScope::Global);
        assert_eq!(decision.subject, RiskScopeKey::Global);
        assert_eq!(decision.observed, 2);
        assert_eq!(decision.limit, 2);
        assert_eq!(decision.window, cap.window);
        assert_eq!(decision.ts_event, ts);

        let OrderDeniedReason::OrderCountLimitReached {
            metric,
            scope,
            observed,
            limit,
            window_ns,
        } = decision.reason()
        else {
            panic!("expected an order count refusal");
        };

        assert_eq!(metric, RiskCapMetric::Submit);
        assert_eq!(scope, RiskCapScope::Global);
        assert_eq!(observed, 2);
        assert_eq!(limit, 2);
        assert_eq!(window_ns, 60_000_000_000);
    }

    #[rstest]
    fn test_evaluate_ignores_a_cap_that_does_not_gate_the_action() {
        let cache = Cache::default();
        let subject = subject(true);
        let caps = vec![RiskCap::new(
            RiskCapMetric::Modify,
            RiskCapScope::Global,
            0,
            Some(window_secs(60)),
        )];
        let mut counters = RiskCounters::default();

        assert!(
            evaluate(
                &caps,
                &mut counters,
                &cache,
                RiskCapMetric::Submit,
                &subject,
                None,
                UnixNanos::from_seconds(1)
            )
            .is_none()
        );
    }

    #[rstest]
    fn test_evaluate_skips_an_account_cap_without_an_account() {
        let cache = Cache::default();
        let subject = subject(false);
        let caps = vec![RiskCap::new(
            RiskCapMetric::Submit,
            RiskCapScope::Account,
            0,
            Some(window_secs(60)),
        )];
        let mut counters = RiskCounters::default();

        assert!(
            evaluate(
                &caps,
                &mut counters,
                &cache,
                RiskCapMetric::Submit,
                &subject,
                None,
                UnixNanos::from_seconds(1)
            )
            .is_none()
        );
    }

    fn limit_order(client_order_id: &str, quantity: &str) -> OrderAny {
        OrderTestBuilder::new(OrderType::Limit)
            .instrument_id(InstrumentId::from("AUD/USD.SIM"))
            .client_order_id(ClientOrderId::from(client_order_id))
            .side(OrderSide::Buy)
            .quantity(Quantity::from(quantity))
            .price(Price::from("1.00000"))
            .build()
    }

    #[rstest]
    fn test_request_key_excludes_the_client_order_id() {
        let first = limit_order("O-001", "100000");
        let repeat = limit_order("O-002", "100000");
        let other_size = limit_order("O-003", "200000");

        assert_eq!(RiskRequestKey::from(&first), RiskRequestKey::from(&repeat));
        assert_ne!(
            RiskRequestKey::from(&first),
            RiskRequestKey::from(&other_size)
        );
    }

    #[rstest]
    fn test_a_repeated_request_cap_counts_per_request_identity() {
        let cache = Cache::default();
        let subject = subject(true);
        let cap = RiskCap::new(
            RiskCapMetric::RepeatedRequest,
            RiskCapScope::Global,
            1,
            Some(window_secs(60)),
        );
        let caps = vec![cap.clone()];
        let mut counters = RiskCounters::default();
        let ts = UnixNanos::from_seconds(1);

        let first = RiskRequestKey::new(
            InstrumentId::from("AUD/USD.SIM"),
            OrderType::Limit,
            OrderSide::Buy,
            Quantity::from("100000"),
            Some(Price::from("1.00000")),
        );
        let other = RiskRequestKey::new(
            InstrumentId::from("AUD/USD.SIM"),
            OrderType::Limit,
            OrderSide::Buy,
            Quantity::from("200000"),
            Some(Price::from("1.00000")),
        );

        assert!(
            evaluate(
                &caps,
                &mut counters,
                &cache,
                RiskCapMetric::Submit,
                &subject,
                Some(&first),
                ts
            )
            .is_none()
        );

        counters.record(
            RiskCounterKey::new(&cap, RiskScopeKey::Global, Some(&first)),
            ts,
        );

        let decision = evaluate(
            &caps,
            &mut counters,
            &cache,
            RiskCapMetric::Submit,
            &subject,
            Some(&first),
            ts,
        )
        .expect("the repeat is refused");

        assert_eq!(decision.metric, RiskCapMetric::RepeatedRequest);
        assert_eq!(decision.scope, RiskCapScope::Global);
        assert_eq!(decision.subject, RiskScopeKey::Global);
        assert_eq!(decision.request.as_ref(), Some(&first));
        assert_eq!(decision.observed, 1);
        assert_eq!(decision.limit, 1);
        assert_eq!(decision.window, cap.window);
        assert_eq!(decision.ts_event, ts);

        let OrderDeniedReason::RepeatedRequestLimitReached {
            scope,
            request,
            observed,
            limit,
            window_ns,
        } = decision.reason()
        else {
            panic!("expected a repeated request refusal");
        };

        assert_eq!(scope, RiskCapScope::Global);
        assert_eq!(request, first);
        assert_eq!(observed, 1);
        assert_eq!(limit, 1);
        assert_eq!(window_ns, 60_000_000_000);

        // A different request shape holds its own counter.
        assert!(
            evaluate(
                &caps,
                &mut counters,
                &cache,
                RiskCapMetric::Submit,
                &subject,
                Some(&other),
                ts
            )
            .is_none()
        );

        // Without an identity the cap cannot be counted, so it is skipped.
        assert!(
            evaluate(
                &caps,
                &mut counters,
                &cache,
                RiskCapMetric::Submit,
                &subject,
                None,
                ts
            )
            .is_none()
        );
    }

    #[rstest]
    fn test_exposure_cap_aggregates_across_strategies_and_venues_and_denies() {
        let (cache, portfolio) = funded_portfolio(true);
        let subject = subject(true);
        let mut counters = RiskCounters::default();
        let ts = UnixNanos::from(1);

        let cap = RiskCap::new_money(
            RiskCapMetric::NetExposure,
            RiskCapScope::Global,
            dec!(200_000),
            Currency::USD(),
        );
        let decision = super::evaluate(
            std::slice::from_ref(&cap),
            &mut counters,
            &cache.borrow(),
            &portfolio,
            RiskCapMetric::Submit,
            &subject,
            None,
            ts,
        )
        .expect("a portfolio above the exposure limit should be refused");

        // 100,000 AUD/USD on one venue and 1,000 AAPL on another, both owned by a different
        // strategy than the one submitting: neither is above the limit alone.
        assert_eq!(decision.observed_money, Some(dec!(250_000)));
        assert_eq!(decision.limit_money, Some(dec!(200_000)));
        assert_eq!(decision.money_currency, Some(Currency::USD()));

        let rendered = decision.reason().to_string();
        assert!(
            rendered.starts_with("EXPOSURE_LIMIT_REACHED:")
                && rendered.contains("currency=USD")
                && rendered.contains("observed=250000"),
            "{rendered}"
        );

        // The same portfolio passes a limit it does not reach.
        let cap = RiskCap::new_money(
            RiskCapMetric::NetExposure,
            RiskCapScope::Global,
            dec!(300_000),
            Currency::USD(),
        );
        assert!(
            super::evaluate(
                std::slice::from_ref(&cap),
                &mut counters,
                &cache.borrow(),
                &portfolio,
                RiskCapMetric::Submit,
                &subject,
                None,
                ts
            )
            .is_none()
        );

        // An account scope aggregates one account's portfolio rather than every account's, so the
        // second venue's positions are not counted and the same limit is not reached.
        let cap = RiskCap::new_money(
            RiskCapMetric::NetExposure,
            RiskCapScope::Account,
            dec!(200_000),
            Currency::USD(),
        );
        let sim_subject = RiskSubject::new(
            StrategyId::from("S-1"),
            InstrumentId::from("AUD/USD.SIM"),
            Some(AccountId::from("SIM-001")),
        );
        assert!(
            super::evaluate(
                std::slice::from_ref(&cap),
                &mut counters,
                &cache.borrow(),
                &portfolio,
                RiskCapMetric::Submit,
                &sim_subject,
                None,
                ts
            )
            .is_none()
        );
    }

    #[rstest]
    fn test_exposure_cap_refuses_when_the_portfolio_cannot_value_it() {
        let (cache, portfolio) = funded_portfolio(false);
        let mut counters = RiskCounters::default();

        let cap = RiskCap::new_money(
            RiskCapMetric::NetExposure,
            RiskCapScope::Global,
            dec!(1_000_000_000),
            Currency::USD(),
        );
        let decision = super::evaluate(
            std::slice::from_ref(&cap),
            &mut counters,
            &cache.borrow(),
            &portfolio,
            RiskCapMetric::Submit,
            &subject(true),
            None,
            UnixNanos::from(1),
        )
        .expect("an unanswerable exposure limit should be refused");

        assert_eq!(decision.observed_money, None);
        assert_eq!(decision.limit_money, Some(dec!(1_000_000_000)));

        let rendered = decision.reason().to_string();
        assert!(
            rendered.starts_with("EXPOSURE_LIMIT_UNKNOWN:") && rendered.contains("currency=USD"),
            "{rendered}"
        );
    }

    /// Returns a cache holding two positions in two venues under two strategies, and the portfolio
    /// over it.
    ///
    /// `priced` adds the quotes the portfolio values the positions with, so a test can ask for the
    /// same holdings with no price to read.
    fn funded_portfolio(priced: bool) -> (Rc<RefCell<Cache>>, Portfolio) {
        use nautilus_model::{
            accounts::{AccountAny, CashAccount},
            data::QuoteTick,
            enums::{AccountType, LiquiditySide, OmsType},
            events::{AccountState, OrderEventAny},
            identifiers::{PositionId, TradeId},
            instruments::{
                Instrument, InstrumentAny,
                stubs::{audusd_sim, equity_aapl},
            },
            orders::stubs::TestOrderEventStubs,
            position::Position,
            types::{AccountBalance, Money},
        };

        let audusd = InstrumentAny::CurrencyPair(audusd_sim());
        let aapl = InstrumentAny::Equity(equity_aapl());

        let cache = Rc::new(RefCell::new(Cache::default()));

        for instrument in [&audusd, &aapl] {
            cache
                .borrow_mut()
                .add_instrument(instrument.clone())
                .unwrap();
        }

        for account_id in [AccountId::from("SIM-001"), AccountId::from("XNAS-001")] {
            let state = AccountState::new(
                account_id,
                AccountType::Cash,
                vec![AccountBalance::new(
                    Money::from("1000000 USD"),
                    Money::zero(Currency::USD()),
                    Money::from("1000000 USD"),
                )],
                vec![],
                true,
                UUID4::new(),
                UnixNanos::default(),
                UnixNanos::default(),
                Some(Currency::USD()),
            );
            cache
                .borrow_mut()
                .add_account(AccountAny::Cash(CashAccount::new(state, true, false)))
                .unwrap();
        }

        if priced {
            for (instrument, bid) in [(&audusd, "1.00000"), (&aapl, "150.00")] {
                cache
                    .borrow_mut()
                    .add_quote(QuoteTick {
                        instrument_id: instrument.id(),
                        bid_price: Price::from(bid),
                        ask_price: Price::from(bid),
                        bid_size: Quantity::from("1"),
                        ask_size: Quantity::from("1"),
                        ts_event: UnixNanos::default(),
                        ts_init: UnixNanos::from(1),
                    })
                    .unwrap();
            }
        }

        for (instrument, strategy, quantity, price, position_id, account_id) in [
            (&audusd, "S-1", "100000", "1.00000", "P-SIM-1", "SIM-001"),
            (&aapl, "S-2", "1000", "150.00", "P-XNAS-1", "XNAS-001"),
        ] {
            let order = OrderTestBuilder::new(OrderType::Limit)
                .instrument_id(instrument.id())
                .strategy_id(StrategyId::from(strategy))
                .side(OrderSide::Buy)
                .quantity(Quantity::from(quantity))
                .price(Price::from(price))
                .build();
            let filled = TestOrderEventStubs::filled(
                &order,
                instrument,
                Some(TradeId::new(format!("T-{position_id}"))),
                Some(PositionId::new(position_id)),
                Some(Price::from(price)),
                Some(Quantity::from(quantity)),
                Some(LiquiditySide::Taker),
                None,
                None,
                Some(AccountId::from(account_id)),
            );
            let OrderEventAny::Filled(filled) = filled else {
                panic!("expected a filled order event")
            };

            cache
                .borrow_mut()
                .add_position(&Position::new(instrument, filled), OmsType::Netting)
                .unwrap();
        }

        let clock = Rc::new(RefCell::new(VirtualClock::new()));
        let portfolio = Portfolio::new(
            Rc::clone(&clock) as Rc<RefCell<dyn Clock>>,
            Rc::clone(&cache),
            None,
        );

        (cache, portfolio)
    }
}
