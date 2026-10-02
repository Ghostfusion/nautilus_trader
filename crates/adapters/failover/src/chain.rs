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

//! Running one demand across the providers, in priority order.
//!
//! # The budget
//!
//! A demand is bounded by the number of providers rather than by an attempt count that could grow:
//!
//! - **Each provider is retried at most once per demand**, and only for a transient failure. A
//!   retry is safe here because this is a read-only client, where a repeated request is idempotent.
//! - **Each provider is entered at most once per demand**, so the chain never returns to a provider
//!   it has left, and a hop always moves to the next provider in the order.
//! - **The chain stops** when the providers are exhausted, and reports the last failure rather than
//!   looping.
//!
//! On a chain of two providers that bounds a demand at three attempts in the worst case: the
//! primary, its one retry, and the secondary.
//!
//! # What the chain is not
//!
//! It is not a load balancer and not a health tracker. It is handed an order and a demand, and it
//! either produces an answer or reports why it could not. Whether a demand should *start* at the
//! primary is a policy above it, which is what lets a caller hold a demand on the provider that is
//! currently serving it rather than reconsidering on every request.

use async_trait::async_trait;
use nautilus_model::identifiers::ClientId;

use crate::failure::{Action, Failure};

/// One provider in the chain.
///
/// The chain knows nothing about what a provider is made of: that a provider can be asked for a
/// demand, that it answers or says why it could not, and what it is called. Everything else about
/// talking to a venue belongs to the provider.
#[async_trait(?Send)]
pub trait Provider {
    /// What this provider is asked for.
    type Demand;
    /// What it answers with.
    type Answer;

    /// Returns the provider's identity, which is what the chain reports in its trace.
    fn id(&self) -> ClientId;

    /// Serves one demand, or returns why it could not.
    async fn serve(&mut self, demand: &Self::Demand) -> Result<Self::Answer, Failure>;
}

/// One attempt at one provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attempt {
    /// The provider that was asked.
    pub provider: ClientId,
    /// Why the attempt failed, or `None` when it answered.
    pub failure: Option<Failure>,
}

/// What the chain did with a demand.
///
/// The trace is what makes a hop explainable rather than merely visible in the data: it names every
/// provider that was asked, in order, and why each one could not answer.
#[derive(Debug, Default)]
pub struct Trace {
    attempts: Vec<Attempt>,
}

impl Trace {
    /// Returns one entry per attempt, in the order they were made.
    #[must_use]
    pub fn attempts(&self) -> &[Attempt] {
        &self.attempts
    }

    /// Returns whether every attempt so far answered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.attempts.is_empty()
    }

    fn record(&mut self, provider: ClientId, failure: Option<Failure>) {
        self.attempts.push(Attempt { provider, failure });
    }
}

/// What the chain produced for one demand.
#[derive(Debug)]
pub enum Served<T> {
    /// A provider answered.
    Answered(T),
    /// Every provider that could be tried failed, and this is the last failure.
    Failed(Failure),
    /// There was no provider to ask, which is a configuration defect rather than a provider's.
    Unconfigured,
}

impl<T> Served<T> {
    /// Returns the answer, when a provider gave one.
    #[must_use]
    pub fn answer(self) -> Option<T> {
        match self {
            Self::Answered(answer) => Some(answer),
            Self::Failed(_) | Self::Unconfigured => None,
        }
    }
}

/// Runs one demand across `providers`, in the order they are given.
///
/// Every attempt is appended to `trace`, so a caller that wants one trace per demand passes a fresh
/// one. A hop is logged with the failure that caused it, because a hop is a change of dataset and
/// an operator has to be able to see when one happened.
pub async fn run<P>(providers: &mut [P], demand: &P::Demand, trace: &mut Trace) -> Served<P::Answer>
where
    P: Provider,
{
    let last = providers.len().saturating_sub(1);

    if providers.is_empty() {
        log::warn!("the chain has no providers to ask");

        return Served::Unconfigured;
    }

    for (index, provider) in providers.iter_mut().enumerate() {
        let id = provider.id();
        let mut retried = false;

        loop {
            match provider.serve(demand).await {
                Ok(answer) => {
                    trace.record(id, None);

                    return Served::Answered(answer);
                }
                Err(failure) => {
                    trace.record(id, Some(failure.clone()));

                    match Action::decide(&failure, retried) {
                        Action::Retry => {
                            log::debug!("{id} did not answer ({failure}); asking it once more");
                            retried = true;
                        }
                        Action::Hop if index < last => {
                            log::warn!(
                                "{id} could not serve the demand ({failure}); asking the next provider"
                            );

                            break;
                        }
                        // A hop with nowhere to hop to is the end of the chain, and so is a stop.
                        Action::Hop | Action::Stop => {
                            log::error!("{id} could not serve the demand: {failure}");

                            return Served::Failed(failure);
                        }
                    }
                }
            }
        }
    }

    unreachable!("the loop returns on the last provider")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::ScriptedProvider;

    fn provider(id: &str, script: Vec<Result<&str, Failure>>) -> ScriptedProvider<(), String> {
        ScriptedProvider::new(
            ClientId::from(id),
            script
                .into_iter()
                .map(|outcome| outcome.map(str::to_string))
                .collect(),
        )
    }

    fn asked(provider: &ScriptedProvider<(), String>) -> usize {
        provider.demands().len()
    }

    #[tokio::test]
    async fn test_a_provider_that_answers_is_asked_once_and_the_chain_stops() {
        let mut providers = vec![
            provider("PRIMARY", vec![Ok("bars")]),
            provider("SECONDARY", vec![Ok("other bars")]),
        ];
        let mut trace = Trace::default();

        let served = run(&mut providers, &(), &mut trace).await;

        assert!(matches!(served.answer().as_deref(), Some("bars")));
        assert_eq!(asked(&providers[0]), 1);
        assert_eq!(asked(&providers[1]), 0, "the secondary was not needed");
        assert_eq!(trace.attempts().len(), 1);
        assert_eq!(trace.attempts()[0].provider, ClientId::from("PRIMARY"));
        assert!(trace.attempts()[0].failure.is_none());
    }

    #[tokio::test]
    async fn test_a_transient_failure_is_retried_once_and_then_hopped() {
        let mut providers = vec![
            provider("PRIMARY", vec![Err(Failure::Unanswered), Ok("bars")]),
            provider("SECONDARY", vec![Ok("other bars")]),
        ];
        let mut trace = Trace::default();

        let served = run(&mut providers, &(), &mut trace).await;

        assert!(matches!(served.answer().as_deref(), Some("bars")));
        assert_eq!(asked(&providers[0]), 2, "one attempt and one retry");
        assert_eq!(asked(&providers[1]), 0);
        assert_eq!(trace.attempts().len(), 2);
        assert_eq!(trace.attempts()[1].failure, None);
    }

    #[tokio::test]
    async fn test_a_transient_failure_that_recurs_moves_to_the_next_provider() {
        let mut providers = vec![
            provider(
                "PRIMARY",
                vec![Err(Failure::Unanswered), Err(Failure::Unanswered)],
            ),
            provider("SECONDARY", vec![Ok("other bars")]),
        ];
        let mut trace = Trace::default();

        let served = run(&mut providers, &(), &mut trace).await;

        assert!(matches!(served.answer().as_deref(), Some("other bars")));
        assert_eq!(
            asked(&providers[0]),
            2,
            "retried once, and not a third time"
        );
        assert_eq!(asked(&providers[1]), 1);
        assert_eq!(trace.attempts().len(), 3);
    }

    #[tokio::test]
    async fn test_a_provider_that_cannot_be_reached_is_not_retried() {
        let mut providers = vec![
            provider(
                "PRIMARY",
                vec![Err(Failure::Unreachable("refused".to_string()))],
            ),
            provider("SECONDARY", vec![Ok("other bars")]),
        ];
        let mut trace = Trace::default();

        let served = run(&mut providers, &(), &mut trace).await;

        assert!(matches!(served.answer().as_deref(), Some("other bars")));
        assert_eq!(
            asked(&providers[0]),
            1,
            "a refused connection is not retried"
        );
        assert_eq!(asked(&providers[1]), 1);
    }

    #[tokio::test]
    async fn test_a_rate_limited_provider_is_hopped_and_recorded() {
        let mut providers = vec![
            provider("PRIMARY", vec![Err(Failure::RateLimited)]),
            provider("SECONDARY", vec![Ok("other bars")]),
        ];
        let mut trace = Trace::default();

        let served = run(&mut providers, &(), &mut trace).await;

        assert!(matches!(served.answer().as_deref(), Some("other bars")));
        assert_eq!(asked(&providers[0]), 1, "a rate limit is never retried");
        assert_eq!(trace.attempts()[0].failure, Some(Failure::RateLimited));
    }

    #[tokio::test]
    async fn test_an_entitlement_refusal_is_hopped_and_recorded() {
        let mut providers = vec![
            provider(
                "PRIMARY",
                vec![Err(Failure::NotEntitled("US options".to_string()))],
            ),
            provider("SECONDARY", vec![Ok("other bars")]),
        ];
        let mut trace = Trace::default();

        let served = run(&mut providers, &(), &mut trace).await;

        assert!(matches!(served.answer().as_deref(), Some("other bars")));
        assert_eq!(asked(&providers[0]), 1);
        assert_eq!(
            trace.attempts()[0].failure,
            Some(Failure::NotEntitled("US options".to_string()))
        );
    }

    #[tokio::test]
    async fn test_a_defect_stops_the_chain_and_never_reaches_the_next_provider() {
        let mut providers = vec![
            provider(
                "PRIMARY",
                vec![Err(Failure::Invalid("negative quantity".to_string()))],
            ),
            provider("SECONDARY", vec![Ok("other bars")]),
        ];
        let mut trace = Trace::default();

        let served = run(&mut providers, &(), &mut trace).await;

        assert!(matches!(served, Served::Failed(Failure::Invalid(_))));
        assert_eq!(asked(&providers[0]), 1);
        assert_eq!(
            asked(&providers[1]),
            0,
            "a caller defect is not another provider's to answer"
        );
    }

    #[tokio::test]
    async fn test_the_chain_never_returns_to_a_provider_it_has_left() {
        let mut providers = vec![
            provider(
                "PRIMARY",
                vec![Err(Failure::Unreachable("refused".to_string()))],
            ),
            provider("SECONDARY", vec![Err(Failure::RateLimited)]),
            provider("TERTIARY", vec![Ok("third time")]),
        ];
        let mut trace = Trace::default();

        let served = run(&mut providers, &(), &mut trace).await;

        assert!(matches!(served.answer().as_deref(), Some("third time")));
        assert_eq!(asked(&providers[0]), 1);
        assert_eq!(asked(&providers[1]), 1);
        assert_eq!(asked(&providers[2]), 1);

        let asked_order: Vec<_> = trace
            .attempts()
            .iter()
            .map(|attempt| attempt.provider)
            .collect();

        assert_eq!(
            asked_order,
            vec![
                ClientId::from("PRIMARY"),
                ClientId::from("SECONDARY"),
                ClientId::from("TERTIARY"),
            ]
        );
    }

    #[tokio::test]
    async fn test_an_exhausted_chain_reports_the_last_failure() {
        let mut providers = vec![
            provider(
                "PRIMARY",
                vec![Err(Failure::Unanswered), Err(Failure::Unanswered)],
            ),
            provider("SECONDARY", vec![Err(Failure::NotFound)]),
        ];
        let mut trace = Trace::default();

        let served = run(&mut providers, &(), &mut trace).await;

        assert!(matches!(served, Served::Failed(Failure::NotFound)));
        assert_eq!(
            trace.attempts().len(),
            3,
            "one retry on the primary, one attempt each on the secondary"
        );
    }

    #[tokio::test]
    async fn test_an_empty_chain_is_unconfigured_rather_than_failed() {
        let mut providers: Vec<ScriptedProvider<(), String>> = Vec::new();
        let mut trace = Trace::default();

        let served = run(&mut providers, &(), &mut trace).await;

        assert!(matches!(served, Served::Unconfigured));
        assert!(trace.is_empty());
    }

    /// An empty answer is an answer, and the chain must stop on it rather than ask anyone else: a
    /// symbol that legitimately has no data on the primary is not a reason to serve it from a
    /// different source with different semantics.
    #[tokio::test]
    async fn test_an_empty_answer_does_not_hop() {
        let mut providers = vec![
            provider("PRIMARY", vec![Ok("")]),
            provider("SECONDARY", vec![Ok("other bars")]),
        ];
        let mut trace = Trace::default();

        let served = run(&mut providers, &(), &mut trace).await;

        assert!(matches!(served.answer().as_deref(), Some("")));
        assert_eq!(
            asked(&providers[1]),
            0,
            "an empty answer is a provider answering, not a gap"
        );
    }

    /// Every gap, whatever its shape, costs one attempt and no retry.
    #[tokio::test]
    async fn test_a_gap_moves_on_without_retrying() {
        for failure in [
            Failure::NotFound,
            Failure::Malformed("bad json".to_string()),
            Failure::Refused { status: 403 },
            Failure::ServerUnavailable { status: 503 },
        ] {
            let script = if matches!(failure, Failure::ServerUnavailable { .. }) {
                // A 5xx is transient, so it costs a retry before the hop; the point here is that
                // the hop happens without a third attempt.
                vec![Err(failure.clone()), Err(failure.clone())]
            } else {
                vec![Err(failure.clone())]
            };

            let mut providers = vec![
                ScriptedProvider::new(ClientId::from("PRIMARY"), script),
                ScriptedProvider::new(
                    ClientId::from("SECONDARY"),
                    vec![Ok("other bars".to_string())],
                ),
            ];
            let mut trace = Trace::default();

            let served = run(&mut providers, &(), &mut trace).await;

            assert!(
                matches!(served.answer().as_deref(), Some("other bars")),
                "{failure} should have moved the demand on"
            );
            assert_eq!(asked(&providers[1]), 1, "{failure} reached the secondary");
        }
    }
}
