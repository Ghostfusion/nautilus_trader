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

//! Why a provider could not answer, and what the chain does about it.
//!
//! "Fails for any reason" is not a policy, so a failure is classified before anything is decided
//! about it, and the two decisions a chain makes are kept apart:
//!
//! - **A retry** is for a transient condition on a provider that is otherwise the right one.
//! - **A hop** is a deliberate move to a different dataset, which is why it is never taken on a
//!   failure that a second provider would answer just as badly, and never taken on an empty answer.
//!
//! Three rules produce the classification, and everything else follows from them.
//!
//! - **A provider that cannot be reached is a gap, not a transient condition.** Nothing about a
//!   refused connection changes between two attempts, so the chain moves on rather than repeating
//!   the wait.
//! - **No client error is retried.** A 4xx is the provider stating that the request, the
//!   credential, or the caller's entitlement is wrong, or that the caller is being throttled. A
//!   second identical request cannot change that answer, and for a rate limit it makes the
//!   condition worse.
//! - **An unreadable answer, a refusal, and a resource the provider does not carry are all gaps.**
//!   Each is the provider saying it cannot serve this demand, which is what the next provider is
//!   for, and each is recorded rather than swallowed.

use thiserror::Error;

/// Why a provider could not answer a demand.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum Failure {
    /// The provider could not be reached at all.
    ///
    /// A refused connection, a name that does not resolve, and a gateway that is not running are
    /// the same failure to a caller: there is nothing on the other end.
    #[error("the provider could not be reached: {0}")]
    Unreachable(String),
    /// The request was sent, and no answer arrived.
    ///
    /// This covers both a request that timed out and one whose outcome is unknown because the
    /// transport ended without one. The two are the same condition for a read-only data client,
    /// where a repeated request is idempotent.
    #[error("no answer arrived for the request")]
    Unanswered,
    /// The provider failed while serving the request.
    #[error("the provider failed the request with status {status}")]
    ServerUnavailable {
        /// The status the provider returned.
        status: u16,
    },
    /// The provider is throttling the caller.
    #[error("the provider is rate limiting the caller")]
    RateLimited,
    /// The provider refused the request, which for a data provider means a credential or a plan.
    #[error("the provider refused the request with status {status}")]
    Refused {
        /// The status the provider returned.
        status: u16,
    },
    /// The provider does not carry the resource.
    #[error("the provider does not carry the requested resource")]
    NotFound,
    /// The provider answered with something that could not be read.
    #[error("the provider's answer could not be read: {0}")]
    Malformed(String),
    /// The provider's own entitlement refuses this demand.
    #[error("the provider is not entitled to serve this demand: {0}")]
    NotEntitled(String),
    /// The demand itself cannot be served by anyone.
    #[error("the demand is not valid: {0}")]
    Invalid(String),
}

/// How a failure is classified, before the attempt budget is applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureClass {
    /// Transient: the same provider may answer on a second attempt.
    Transient,
    /// A gap in what this provider serves: another provider may be able to answer.
    ProviderGap,
    /// A defect in the demand: no provider can answer it.
    Defect,
}

impl Failure {
    /// Returns how this failure is classified.
    #[must_use]
    pub fn class(&self) -> FailureClass {
        match self {
            Self::Unanswered | Self::ServerUnavailable { .. } => FailureClass::Transient,
            Self::Unreachable(_)
            | Self::RateLimited
            | Self::Refused { .. }
            | Self::NotFound
            | Self::Malformed(_)
            | Self::NotEntitled(_) => FailureClass::ProviderGap,
            Self::Invalid(_) => FailureClass::Defect,
        }
    }

    /// Returns the failure a response status describes.
    ///
    /// A status is a statement about the request or about the provider, and the four answers a
    /// provider can give are told apart here rather than at each call site:
    ///
    /// - a rate limit and a refusal are the provider declining, which moves the demand on;
    /// - a missing resource is the provider not having it, which moves the demand on;
    /// - a server error is the provider's own failure, which is worth one more attempt;
    /// - and every other client error is the caller's, which no provider and no attempt can fix.
    #[must_use]
    pub fn from_http_status(status: u16) -> Self {
        match status {
            429 => Self::RateLimited,
            403 => Self::Refused { status },
            404 => Self::NotFound,
            500..=599 => Self::ServerUnavailable { status },
            _ => Self::Invalid(format!(
                "the provider rejected the request with status {status}"
            )),
        }
    }
}

/// What the chain does about a failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Ask the same provider again.
    Retry,
    /// Ask the next provider.
    Hop,
    /// Stop, and report the failure rather than asking anyone else.
    Stop,
}

impl Action {
    /// Returns what to do about `failure`, given whether this provider has already been retried.
    ///
    /// The retry is the caller's to account for rather than an internal counter, because the budget
    /// is per provider per demand and the chain that spends it is the one that knows when a demand
    /// begins.
    #[must_use]
    pub fn decide(failure: &Failure, retried: bool) -> Self {
        match failure.class() {
            FailureClass::Transient if !retried => Self::Retry,
            FailureClass::Transient | FailureClass::ProviderGap => Self::Hop,
            FailureClass::Defect => Self::Stop,
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case::unreachable(Failure::Unreachable("refused".to_string()), FailureClass::ProviderGap)]
    #[case::unanswered(Failure::Unanswered, FailureClass::Transient)]
    #[case::server_error(Failure::ServerUnavailable { status: 503 }, FailureClass::Transient)]
    #[case::rate_limited(Failure::RateLimited, FailureClass::ProviderGap)]
    #[case::refused(Failure::Refused { status: 403 }, FailureClass::ProviderGap)]
    #[case::not_found(Failure::NotFound, FailureClass::ProviderGap)]
    #[case::malformed(Failure::Malformed("bad json".to_string()), FailureClass::ProviderGap)]
    #[case::not_entitled(Failure::NotEntitled("US options".to_string()), FailureClass::ProviderGap)]
    #[case::invalid(Failure::Invalid("negative quantity".to_string()), FailureClass::Defect)]
    fn test_each_failure_is_classified(#[case] failure: Failure, #[case] expected: FailureClass) {
        assert_eq!(failure.class(), expected);
    }

    /// A status is read once, here, so that no call site has to decide whether a 403 is worth
    /// another attempt.
    #[rstest]
    #[case::rate_limit(429, Failure::RateLimited)]
    #[case::forbidden(403, Failure::Refused { status: 403 })]
    #[case::not_found(404, Failure::NotFound)]
    #[case::server_error(500, Failure::ServerUnavailable { status: 500 })]
    #[case::bad_gateway(502, Failure::ServerUnavailable { status: 502 })]
    fn test_a_status_classifies_a_retryable_or_avoidable_condition(
        #[case] status: u16,
        #[case] expected: Failure,
    ) {
        assert_eq!(Failure::from_http_status(status), expected);
    }

    #[rstest]
    #[case::bad_request(400)]
    #[case::unauthorized(401)]
    #[case::unprocessable(422)]
    fn test_any_other_client_error_is_the_callers(#[case] status: u16) {
        let failure = Failure::from_http_status(status);

        assert_eq!(failure.class(), FailureClass::Defect);
    }

    #[rstest]
    fn test_a_transient_failure_is_retried_once_and_then_hopped() {
        let failure = Failure::Unanswered;

        assert_eq!(Action::decide(&failure, false), Action::Retry);
        assert_eq!(Action::decide(&failure, true), Action::Hop);
    }

    #[rstest]
    fn test_a_gap_is_hopped_without_an_attempt_at_the_same_provider() {
        for failure in [
            Failure::Unreachable("refused".to_string()),
            Failure::RateLimited,
            Failure::Refused { status: 403 },
            Failure::NotFound,
            Failure::Malformed("bad json".to_string()),
            Failure::NotEntitled("US equities".to_string()),
        ] {
            assert_eq!(Action::decide(&failure, false), Action::Hop, "{failure}");
        }
    }

    #[rstest]
    fn test_a_defect_stops_the_chain_rather_than_moving_it() {
        let failure = Failure::Invalid("negative quantity".to_string());

        assert_eq!(Action::decide(&failure, false), Action::Stop);
        assert_eq!(Action::decide(&failure, true), Action::Stop);
    }
}
