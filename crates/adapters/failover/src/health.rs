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

//! What a chain knows about its providers, and when it stops asking one of them.
//!
//! A chain that asks a dead provider on every demand is not a chain, it is a load on a dead provider:
//! the point of having a second provider is that the first one's failure stops costing anything. So a
//! provider that has failed is set aside, and it is brought back deliberately.
//!
//! # The breaker
//!
//! The conventional name for this is a circuit breaker, and what it does here is simpler than the
//! name suggests: a provider that has failed is set aside, and brought back deliberately.
//!
//! - **A provider that has failed is not asked again** until its cooldown has passed.
//! - **After the cooldown it is probed once**, and only once: while a probe is outstanding the
//!   provider is not admitted, so a burst of demands does not become a burst of probes.
//! - **An answer closes the breaker** and resets the count of consecutive failures. A failure during
//!   a probe reopens it for a further cooldown.
//!
//! # Why the time is the caller's
//!
//! Every method that depends on the passage of time takes it, rather than reading a clock. A policy
//! about time is exactly the kind of thing that is only ever tested by waiting, and a policy that can
//! be given a time is tested by stating it.

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use nautilus_model::identifiers::ClientId;
use parking_lot::Mutex;

use crate::failure::Failure;

/// How a chain decides that a provider is not worth asking.
#[derive(Debug, Clone, Copy)]
pub struct Policy {
    /// How many consecutive failures set a provider aside.
    pub failure_threshold: u32,
    /// How long a provider is set aside before it is probed once.
    pub cooldown: Duration,
    /// How long a demand may go unanswered before the provider that took it is held to account.
    ///
    /// A provider that takes a demand and then says nothing is the one failure a chain cannot see
    /// when it is asked, because a provider answers on the event channel rather than to the call.
    pub answer_deadline: Duration,
}

impl Default for Policy {
    /// One failure is enough, because one failure is already a provider that did not serve.
    ///
    /// A chain moves on when a provider will not serve, and there is nothing to wait for: the
    /// alternative is asking a provider that has just said it cannot answer, on every demand, which
    /// is what the cooldown is for.
    fn default() -> Self {
        Self {
            failure_threshold: 1,
            cooldown: Duration::from_secs(60),
            answer_deadline: Duration::from_secs(30),
        }
    }
}

/// What a chain knows about one provider.
#[derive(Debug, Clone)]
pub struct Snapshot {
    /// The provider this is about.
    pub provider: ClientId,
    /// Whether the provider has been set aside.
    pub is_open: bool,
    /// How many times the provider has failed in a row.
    pub consecutive_failures: u32,
    /// Why the provider last failed, which is what its hops are reported with.
    pub last_failure: Option<Failure>,
    /// How many times the provider was asked for a demand.
    pub attempts: u64,
    /// How many of those attempts failed.
    pub failures: u64,
    /// How many answers the provider gave.
    pub answers: u64,
    /// How many times the chain moved away from this provider.
    pub hops: u64,
}

/// What one provider is doing as far as the chain is concerned.
#[derive(Debug, Default)]
struct Provider {
    consecutive_failures: u32,
    set_aside_at: Option<Instant>,
    probing: bool,
    last_failure: Option<Failure>,
    attempts: u64,
    failures: u64,
    answers: u64,
    hops: u64,
}

/// The state of every provider a chain has asked.
#[derive(Debug)]
pub struct Health {
    policy: Policy,
    providers: Mutex<HashMap<ClientId, Provider>>,
}

impl Health {
    /// Creates a record that sets a provider aside by `policy`.
    #[must_use]
    pub fn new(policy: Policy) -> Self {
        Self {
            policy,
            providers: Mutex::new(HashMap::new()),
        }
    }

    /// Returns the policy this record applies.
    #[must_use]
    pub fn policy(&self) -> Policy {
        self.policy
    }

    /// Returns whether `provider` may be asked at `now`.
    ///
    /// A provider that has never failed may be asked. One that has failed may be asked again once its
    /// cooldown has passed, and that ask is the probe: while the probe is outstanding, which is from
    /// the moment the provider is asked until it answers or fails, it is not admitted again, so a
    /// demand arriving every tick does not become a probe every tick.
    ///
    /// Reading this creates nothing: a provider the chain has never asked is not one it knows
    /// anything about, and what it does not know is not a failure.
    pub fn admit(&self, provider: &ClientId, now: Instant) -> bool {
        let providers = self.providers.lock();
        let Some(state) = providers.get(provider) else {
            // A provider nothing is known about is a provider that has not failed.
            return true;
        };

        let Some(set_aside_at) = state.set_aside_at else {
            return true;
        };

        if state.probing {
            return false;
        }

        if now.duration_since(set_aside_at) < self.policy.cooldown {
            return false;
        }

        true
    }

    /// Notes that `provider` was asked for a demand.
    ///
    /// Being asked is what makes a probe outstanding, so that a provider that has been set aside is
    /// given one chance to answer before it is set aside again.
    pub fn asked(&self, provider: &ClientId) {
        let mut providers = self.providers.lock();
        let state = providers.entry(*provider).or_default();

        state.attempts += 1;

        if state.set_aside_at.is_some() {
            state.probing = true;
        }
    }

    /// Notes that `provider` answered, which is what closes a breaker.
    pub fn answered(&self, provider: &ClientId) {
        let mut providers = self.providers.lock();
        let state = providers.entry(*provider).or_default();

        state.consecutive_failures = 0;
        state.set_aside_at = None;
        state.probing = false;
        state.answers += 1;
    }

    /// Notes that `provider` failed, which sets it aside once it has failed often enough.
    pub fn failed(&self, provider: &ClientId, failure: &Failure, now: Instant) {
        let threshold = self.policy.failure_threshold;
        let mut providers = self.providers.lock();
        let state = providers.entry(*provider).or_default();

        state.consecutive_failures += 1;
        state.failures += 1;
        state.last_failure = Some(failure.clone());

        if state.consecutive_failures >= threshold {
            state.set_aside_at = Some(now);
            state.probing = false;
        }
    }

    /// Notes that the chain moved away from `provider`.
    pub fn hopped_from(&self, provider: &ClientId) {
        self.providers.lock().entry(*provider).or_default().hops += 1;
    }

    /// Returns what is known about `provider`, whether or not it has ever been asked.
    #[must_use]
    pub fn snapshot(&self, provider: &ClientId) -> Snapshot {
        let providers = self.providers.lock();

        match providers.get(provider) {
            Some(state) => snapshot_of(*provider, state),
            None => snapshot_of(*provider, &Provider::default()),
        }
    }

    /// Returns what is known about every provider that has been asked.
    #[must_use]
    pub fn snapshots(&self) -> Vec<Snapshot> {
        self.providers
            .lock()
            .iter()
            .map(|(provider, state)| snapshot_of(*provider, state))
            .collect()
    }
}

/// Returns what one provider's state says about it.
///
/// A provider that has never been asked is reported the same way as one that has been asked and
/// answered everything, because nobody has asked it a question it failed: the two are one state.
fn snapshot_of(provider: ClientId, state: &Provider) -> Snapshot {
    Snapshot {
        provider,
        is_open: state.set_aside_at.is_some(),
        consecutive_failures: state.consecutive_failures,
        last_failure: state.last_failure.clone(),
        attempts: state.attempts,
        failures: state.failures,
        answers: state.answers,
        hops: state.hops,
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    fn provider() -> ClientId {
        ClientId::from("PRIMARY")
    }

    fn health() -> Health {
        Health::new(Policy {
            failure_threshold: 1,
            cooldown: Duration::from_secs(60),
            answer_deadline: Duration::from_secs(30),
        })
    }

    fn now() -> Instant {
        Instant::now()
    }

    #[rstest]
    fn test_a_provider_that_has_never_failed_may_always_be_asked() {
        let health = health();
        let now = now();

        assert!(health.admit(&provider(), now));
        assert!(health.admit(&provider(), now));
        assert!(!health.snapshot(&provider()).is_open);
    }

    #[rstest]
    fn test_a_failure_sets_the_provider_aside() {
        let health = health();
        let now = now();

        health.failed(
            &provider(),
            &Failure::Unreachable("refused".to_string()),
            now,
        );

        assert!(!health.admit(&provider(), now + Duration::from_secs(59)));
        assert!(health.snapshot(&provider()).is_open);
    }

    /// The point of the cooldown is that a dead provider stops costing anything, and the point of the
    /// probe is that it is not abandoned forever. One probe, not one per demand.
    #[rstest]
    fn test_the_provider_is_probed_once_after_the_cooldown() {
        let health = health();
        let now = now();

        health.failed(&provider(), &Failure::Unanswered, now);

        let after = now + Duration::from_secs(61);

        assert!(health.admit(&provider(), after), "the probe is available");
        assert!(
            health.admit(&provider(), after),
            "reading admission twice before the provider is asked does not consume the probe"
        );

        health.asked(&provider());

        assert!(
            !health.admit(&provider(), after + Duration::from_secs(1)),
            "while a probe is outstanding the provider is not asked again"
        );
    }

    #[rstest]
    fn test_an_answer_closes_the_breaker_and_clears_the_failures() {
        let health = health();
        let now = now();

        health.failed(&provider(), &Failure::Unanswered, now);
        assert!(health.admit(&provider(), now + Duration::from_secs(61)));

        health.asked(&provider());
        health.answered(&provider());

        let snapshot = health.snapshot(&provider());

        assert!(!snapshot.is_open);
        assert_eq!(snapshot.consecutive_failures, 0);
        assert_eq!(snapshot.answers, 1);
        assert!(
            health.admit(&provider(), now),
            "a provider that answered is asked again"
        );
    }

    #[rstest]
    fn test_a_failed_probe_sets_the_provider_aside_for_a_further_cooldown() {
        let health = health();
        let now = now();

        health.failed(&provider(), &Failure::Unanswered, now);

        let after = now + Duration::from_secs(61);
        assert!(health.admit(&provider(), after));

        health.failed(
            &provider(),
            &Failure::Unreachable("refused".to_string()),
            after,
        );

        assert!(!health.admit(&provider(), after + Duration::from_secs(59)));
        assert!(health.admit(&provider(), after + Duration::from_secs(61)));
    }

    #[rstest]
    fn test_failures_are_counted_in_a_row_and_reset_by_an_answer() {
        let health = Health::new(Policy {
            failure_threshold: 2,
            cooldown: Duration::from_secs(60),
            answer_deadline: Duration::from_secs(30),
        });
        let now = now();

        health.failed(&provider(), &Failure::Unanswered, now);
        assert!(
            health.admit(&provider(), now),
            "one failure is not enough to set a provider aside"
        );

        health.asked(&provider());
        health.answered(&provider());
        health.failed(&provider(), &Failure::Unanswered, now);

        assert_eq!(health.snapshot(&provider()).consecutive_failures, 1);
        assert!(!health.snapshot(&provider()).is_open);
    }

    #[rstest]
    fn test_the_counts_and_the_reason_are_kept_for_each_provider() {
        let health = health();
        let now = now();

        health.asked(&provider());
        health.failed(
            &provider(),
            &Failure::Unreachable("refused".to_string()),
            now,
        );
        health.hopped_from(&provider());

        health.asked(&ClientId::from("SECONDARY"));
        health.answered(&ClientId::from("SECONDARY"));

        let snapshot = health.snapshot(&provider());

        assert_eq!(snapshot.attempts, 1);
        assert_eq!(snapshot.failures, 1);
        assert_eq!(snapshot.answers, 0);
        assert_eq!(snapshot.hops, 1);
        assert_eq!(
            snapshot.last_failure,
            Some(Failure::Unreachable("refused".to_string()))
        );

        let secondary = health.snapshot(&ClientId::from("SECONDARY"));

        assert_eq!(secondary.attempts, 1);
        assert_eq!(secondary.answers, 1);
        assert!(!secondary.is_open);
        assert_eq!(health.snapshots().len(), 2);
    }

    #[rstest]
    fn test_a_provider_nobody_has_asked_is_reported_as_untouched() {
        let health = health();
        let snapshot = health.snapshot(&ClientId::from("SECONDARY"));

        assert!(!snapshot.is_open);
        assert_eq!(snapshot.attempts, 0);
        assert!(snapshot.last_failure.is_none());
        assert!(health.snapshots().is_empty());
    }
}
