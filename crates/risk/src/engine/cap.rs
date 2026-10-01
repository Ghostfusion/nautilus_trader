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

use ahash::AHashMap;
use nautilus_common::cache::Cache;
use nautilus_core::{DurationNanos, UnixNanos};
use nautilus_model::{
    events::{OrderDeniedReason, OrderEventAny},
    identifiers::{AccountId, InstrumentId, StrategyId, Venue},
    orders::{Order, OrderAny},
    risk::{RiskCapMetric, RiskCapScope, RiskRequestKey},
};
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
    pub limit: u32,
    /// The rolling window the count is taken over, in nanoseconds.
    ///
    /// `None` for [`RiskCapMetric::Active`], which counts the open order set as it stands rather
    /// than occurrences over a window.
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
            window,
        }
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
            RiskCapMetric::Active => false,
            RiskCapMetric::Submit | RiskCapMetric::RepeatedRequest => {
                matches!(action, RiskCapMetric::Submit)
            }
            RiskCapMetric::Modify => matches!(action, RiskCapMetric::Modify),
            RiskCapMetric::Cancel => matches!(action, RiskCapMetric::Cancel),
            RiskCapMetric::Fill => matches!(action, RiskCapMetric::Fill),
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
            ),
            RiskCapMetric::Modify => {
                matches!(self.metric, RiskCapMetric::Modify | RiskCapMetric::Active)
            }
            RiskCapMetric::Cancel
            | RiskCapMetric::Fill
            | RiskCapMetric::Active
            | RiskCapMetric::RepeatedRequest => false,
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
    pub observed: u32,
    /// The configured limit.
    pub limit: u32,
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
pub fn evaluate(
    caps: &[RiskCap],
    counters: &mut RiskCounters,
    cache: &Cache,
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

        let (observed, subject_key) = if cap.metric == RiskCapMetric::Active {
            let observed = active_order_count(cache, cap.scope, subject);
            (observed, subject.key(cap.scope)?)
        } else {
            let subject_key = subject.key(cap.scope)?;
            let key = RiskCounterKey::new(cap, subject_key.clone(), request);
            (counters.count(&key, ts_event), subject_key)
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
                window: cap.window,
                ts_event,
            });
        }
    }

    None
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
    use nautilus_core::UnixNanos;
    use nautilus_model::{
        enums::{OrderSide, OrderType},
        identifiers::ClientOrderId,
        orders::OrderTestBuilder,
        types::{Price, Quantity},
    };
    use rstest::rstest;

    use super::*;

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
}
