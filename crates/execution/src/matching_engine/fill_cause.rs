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

//! The cause of an order event the matching engine produced.
//!
//! A fill caused by a venue rule is not a market outcome. The matching engine makes each decision
//! at a known place - an ordinary book match, a halt's cancel-on-halt, a price-band rejection, a
//! circuit breaker's halt window, a margin liquidation, or a corporate action - and attributes the
//! event it produces to that decision. Counting the per-run totals by cause lets a study separate
//! the rule-driven prints from the market ones, rather than averaging a category error into its
//! result.

use std::{collections::BTreeMap, fmt::Display, str::FromStr};

/// The venue decision that produced an order event.
///
/// The set is closed and each variant has a stable string, so a cause is checkable rather than a
/// spelling competition. The variants name the decisions in `OrderMatchingEngine`; `book_match`
/// is the ordinary order-book match that every fill is attributed to when no rule intervened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FillCause {
    /// An ordinary order-book match.
    BookMatch,
    /// An operator halt's cancel-on-halt.
    Halt,
    /// A submission refused by the venue price band.
    PriceBand,
    /// A circuit breaker's halt window and the cancel-on-halt it triggered.
    CircuitBreaker,
    /// A margin liquidation closing an open position.
    Liquidation,
    /// A corporate action closing a position at expiration or delisting.
    CorporateAction,
}

impl FillCause {
    /// All variants of the closed vocabulary.
    pub const ALL: &'static [Self] = &[
        Self::BookMatch,
        Self::Halt,
        Self::PriceBand,
        Self::CircuitBreaker,
        Self::Liquidation,
        Self::CorporateAction,
    ];

    /// Returns the stable string for this cause.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BookMatch => "book_match",
            Self::Halt => "halt",
            Self::PriceBand => "price_band",
            Self::CircuitBreaker => "circuit_breaker",
            Self::Liquidation => "liquidation",
            Self::CorporateAction => "corporate_action",
        }
    }
}

impl Display for FillCause {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for FillCause {
    type Err = FillCauseParseError;

    /// Parses a cause from its stable string.
    ///
    /// # Errors
    ///
    /// Returns an error if `s` is not one of the closed vocabulary's stable strings.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "book_match" => Ok(Self::BookMatch),
            "halt" => Ok(Self::Halt),
            "price_band" => Ok(Self::PriceBand),
            "circuit_breaker" => Ok(Self::CircuitBreaker),
            "liquidation" => Ok(Self::Liquidation),
            "corporate_action" => Ok(Self::CorporateAction),
            _ => Err(FillCauseParseError {
                value: s.to_string(),
            }),
        }
    }
}

/// The error returned when a string is not a [`FillCause`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FillCauseParseError {
    value: String,
}

impl Display for FillCauseParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "unknown fill cause `{}`, expected one of book_match, halt, price_band, circuit_breaker, liquidation, corporate_action",
            self.value
        )
    }
}

impl std::error::Error for FillCauseParseError {}

/// The per-run totals of order events by cause.
///
/// Every cause in [`FillCause::ALL`] is present from construction, so a cause that did not occur
/// reads as a zero rather than an absent key: a zero is a fact and an omitted row is not. The
/// counter records an event at the decision that produced it, so no event is attributed twice and
/// no event the engine produced is left uncounted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FillCauseCounts {
    counts: BTreeMap<FillCause, u64>,
}

impl FillCauseCounts {
    /// Creates a new counter with every cause at zero.
    #[must_use]
    pub fn new() -> Self {
        let counts = FillCause::ALL.iter().map(|cause| (*cause, 0)).collect();
        Self { counts }
    }

    /// Records one event attributed to `cause`.
    pub fn record(&mut self, cause: FillCause) {
        *self.counts.entry(cause).or_insert(0) += 1;
    }

    /// Returns the total recorded for `cause`.
    #[must_use]
    pub fn get(&self, cause: FillCause) -> u64 {
        self.counts.get(&cause).copied().unwrap_or(0)
    }

    /// Returns the total recorded across every cause.
    #[must_use]
    pub fn total(&self) -> u64 {
        self.counts.values().sum()
    }

    /// Iterates the totals in the vocabulary's order, including the causes at zero.
    pub fn iter(&self) -> impl Iterator<Item = (FillCause, u64)> + '_ {
        FillCause::ALL
            .iter()
            .map(|cause| (*cause, self.get(*cause)))
    }

    /// Adds every total in `other` into this counter.
    pub fn merge(&mut self, other: &Self) {
        for cause in FillCause::ALL {
            *self.counts.entry(*cause).or_insert(0) += other.get(*cause);
        }
    }

    /// Returns the totals keyed by each cause's stable string, including the causes at zero.
    #[must_use]
    pub fn to_string_map(&self) -> BTreeMap<String, u64> {
        self.iter()
            .map(|(cause, count)| (cause.as_str().to_string(), count))
            .collect()
    }
}

impl Default for FillCauseCounts {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for FillCauseCounts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FillCauseCounts(")?;
        for (index, (cause, count)) in self.iter().enumerate() {
            if index > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{cause}={count}")?;
        }
        write!(f, ")")
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[test]
    fn test_all_is_the_closed_vocabulary() {
        assert_eq!(FillCause::ALL.len(), 6);
        for cause in FillCause::ALL {
            assert_eq!(FillCause::from_str(cause.as_str()), Ok(*cause));
        }
    }

    #[rstest]
    #[case(FillCause::BookMatch, "book_match")]
    #[case(FillCause::Halt, "halt")]
    #[case(FillCause::PriceBand, "price_band")]
    #[case(FillCause::CircuitBreaker, "circuit_breaker")]
    #[case(FillCause::Liquidation, "liquidation")]
    #[case(FillCause::CorporateAction, "corporate_action")]
    fn test_as_str_is_stable(#[case] cause: FillCause, #[case] expected: &str) {
        assert_eq!(cause.as_str(), expected);
    }

    #[test]
    fn test_unknown_string_is_refused() {
        assert_eq!(
            FillCause::from_str("weather"),
            Err(FillCauseParseError {
                value: "weather".to_string(),
            })
        );
    }

    #[test]
    fn test_new_counter_holds_every_cause_at_zero() {
        let counts = FillCauseCounts::new();
        assert_eq!(counts.total(), 0);
        for cause in FillCause::ALL {
            assert_eq!(counts.get(*cause), 0);
        }
        assert_eq!(counts.to_string_map().len(), 6);
    }

    #[test]
    fn test_record_and_merge_accumulate_by_cause() {
        let mut counts = FillCauseCounts::new();
        counts.record(FillCause::BookMatch);
        counts.record(FillCause::BookMatch);
        counts.record(FillCause::CircuitBreaker);

        let mut other = FillCauseCounts::new();
        other.record(FillCause::CircuitBreaker);
        counts.merge(&other);

        assert_eq!(counts.get(FillCause::BookMatch), 2);
        assert_eq!(counts.get(FillCause::CircuitBreaker), 2);
        assert_eq!(counts.get(FillCause::Liquidation), 0);
        assert_eq!(counts.total(), 4);
    }
}
