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

//! Competitor-set arrival ranking for backtest passive fills.
//!
//! A backtest can only answer "would this order have arrived first?" by comparing its arrival to a
//! declared cohort of rival latencies. The read is ordinal: our order and every competitor share
//! the same decision instant, so that instant cancels out of the ordering and only the latency
//! differences matter. The venue's closest trader earned while the second closest lost, so the
//! question a cohort answers is the order's rank at the decision, not how long its round trip took.

use std::{cmp::Ordering, fmt::Debug, rc::Rc};

use nautilus_core::{DurationNanos, UnixNanos};

/// The ordinal arrival rank of our order against a [`CompetitorSet`] at a shared decision instant.
///
/// `ahead` counts rivals that arrive strictly earlier than us, `tied` counts those that arrive at
/// the same instant, and `behind` counts those that arrive strictly later. `rank` is `1 + ahead`,
/// so a rank of one means the order arrived first and a rival tied with us is not ahead.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArrivalRank {
    /// Our order's arrival timestamp, `ts_decision + our_latency`.
    pub our_arrival: UnixNanos,
    /// The ordinal rank at the level, `1 + ahead`.
    pub rank: u32,
    /// The number of rivals arriving strictly earlier than our order.
    pub ahead: usize,
    /// The number of rivals arriving at the same instant as our order.
    pub tied: usize,
    /// The number of rivals arriving strictly later than our order.
    pub behind: usize,
    /// Whether no rival arrived strictly earlier, i.e. `ahead == 0`.
    pub arrived_first: bool,
    /// The gap from our arrival to the closest rival arriving later, `None` when none does.
    pub gap_to_closest_behind: Option<DurationNanos>,
}

/// A declared cohort of rival latencies, in nanoseconds.
///
/// The set carries only the rivals' latencies, not their identities or order sizes: the read is
/// ordinal and the shared decision instant cancels out, so all that matters is how many rivals
/// arrive before us. An empty set is the default and reads as arriving first.
#[derive(Debug, Clone, Default)]
pub struct CompetitorSet {
    competitors: Vec<DurationNanos>,
}

impl CompetitorSet {
    /// Creates a new [`CompetitorSet`] from the declared rival latencies.
    #[must_use]
    pub fn new(competitors: Vec<DurationNanos>) -> Self {
        Self { competitors }
    }

    /// Creates a [`CompetitorSet`] of `count` rivals that all share `latency`.
    #[must_use]
    pub fn uniform(count: usize, latency: DurationNanos) -> Self {
        Self {
            competitors: vec![latency; count],
        }
    }

    /// Returns `true` when no rivals are declared.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.competitors.is_empty()
    }

    /// Returns the number of declared rivals.
    #[must_use]
    pub fn len(&self) -> usize {
        self.competitors.len()
    }

    /// Returns the declared rival latencies.
    #[must_use]
    pub fn competitors(&self) -> &[DurationNanos] {
        &self.competitors
    }

    /// Returns the ordinal rank of an order with `our_latency`, `1 + count(rival < our_latency)`.
    ///
    /// The comparison is strict, so a rival tied with us is not ahead. An empty set gives rank 1.
    #[must_use]
    pub fn rank(&self, our_latency: DurationNanos) -> u32 {
        1 + self
            .competitors
            .iter()
            .filter(|&&rival| rival < our_latency)
            .count() as u32
    }

    /// Returns the full arrival rank of an order at `ts_decision` with `our_latency`.
    ///
    /// Our arrival is `ts_decision + our_latency`; every rival arrives at `ts_decision` plus its
    /// own latency. Because the decision instant is shared it cancels out of the ordering, which is
    /// why the read is ordinal and [`Self::rank`] is consistent with this one.
    #[must_use]
    pub fn rank_at_decision_time(
        &self,
        our_latency: DurationNanos,
        ts_decision: UnixNanos,
    ) -> ArrivalRank {
        let our_arrival = ts_decision + our_latency;
        let mut ahead = 0usize;
        let mut tied = 0usize;
        let mut behind = 0usize;
        let mut gap_to_closest_behind: Option<DurationNanos> = None;

        for &rival_latency in &self.competitors {
            let rival_arrival = ts_decision + rival_latency;
            match rival_arrival.cmp(&our_arrival) {
                Ordering::Less => ahead += 1,
                Ordering::Greater => {
                    behind += 1;
                    let gap = rival_arrival - our_arrival;
                    gap_to_closest_behind =
                        Some(gap_to_closest_behind.map_or(gap, |closest| closest.min(gap)));
                }
                Ordering::Equal => tied += 1,
            }
        }

        ArrivalRank {
            our_arrival,
            rank: 1 + ahead as u32,
            ahead,
            tied,
            behind,
            arrived_first: ahead == 0,
            gap_to_closest_behind,
        }
    }
}

/// Shared runtime handle for a [`CompetitorSet`].
///
/// The set is plain data with a single representation, so the handle wraps it directly rather than
/// a trait object. It is cloneable and cheap to share with the exchange.
#[derive(Clone)]
pub struct CompetitorSetHandle(Rc<CompetitorSet>);

impl CompetitorSetHandle {
    /// Creates a new [`CompetitorSetHandle`] from a competitor set.
    #[must_use]
    pub fn new(set: CompetitorSet) -> Self {
        Self(Rc::new(set))
    }

    /// Returns the ordinal rank of an order with `our_latency`.
    #[must_use]
    pub fn rank(&self, our_latency: DurationNanos) -> u32 {
        self.0.rank(our_latency)
    }

    /// Returns the full arrival rank of an order at `ts_decision` with `our_latency`.
    #[must_use]
    pub fn rank_at_decision_time(
        &self,
        our_latency: DurationNanos,
        ts_decision: UnixNanos,
    ) -> ArrivalRank {
        self.0.rank_at_decision_time(our_latency, ts_decision)
    }

    /// Returns `true` when no rivals are declared.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the number of declared rivals.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }
}

impl Debug for CompetitorSetHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple(stringify!(CompetitorSetHandle))
            .field(&"<CompetitorSet>")
            .finish()
    }
}

impl From<CompetitorSet> for CompetitorSetHandle {
    fn from(set: CompetitorSet) -> Self {
        Self::new(set)
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    fn latency(nanos: u64) -> DurationNanos {
        DurationNanos::new(nanos)
    }

    #[rstest]
    fn test_empty_set_reads_rank_one_and_arrives_first() {
        let set = CompetitorSet::default();

        assert!(set.is_empty());
        assert_eq!(set.len(), 0);
        assert_eq!(set.rank(latency(100)), 1);

        let rank = set.rank_at_decision_time(latency(100), UnixNanos::from(1_000));
        assert_eq!(rank.our_arrival, UnixNanos::from(1_100));
        assert_eq!(rank.rank, 1);
        assert_eq!(rank.ahead, 0);
        assert_eq!(rank.tied, 0);
        assert_eq!(rank.behind, 0);
        assert!(rank.arrived_first);
        assert_eq!(rank.gap_to_closest_behind, None);
    }

    #[rstest]
    fn test_strictly_faster_rival_gives_rank_two() {
        let set = CompetitorSet::new(vec![latency(50)]);

        assert_eq!(set.rank(latency(100)), 2);

        let rank = set.rank_at_decision_time(latency(100), UnixNanos::from(1_000));
        assert_eq!(rank.rank, 2);
        assert_eq!(rank.ahead, 1);
        assert_eq!(rank.tied, 0);
        assert_eq!(rank.behind, 0);
        assert!(!rank.arrived_first);
        assert_eq!(rank.gap_to_closest_behind, None);
    }

    #[rstest]
    fn test_tied_rival_is_counted_as_tied_and_leaves_arrived_first() {
        let set = CompetitorSet::new(vec![latency(100)]);

        assert_eq!(set.rank(latency(100)), 1);

        let rank = set.rank_at_decision_time(latency(100), UnixNanos::from(1_000));
        assert_eq!(rank.rank, 1);
        assert_eq!(rank.ahead, 0);
        assert_eq!(rank.tied, 1);
        assert_eq!(rank.behind, 0);
        assert!(rank.arrived_first);
        assert_eq!(rank.gap_to_closest_behind, None);
    }

    #[rstest]
    fn test_mixed_cohort_counts_and_gap_are_correct() {
        let set = CompetitorSet::new(vec![latency(50), latency(100), latency(120), latency(150)]);

        let rank = set.rank_at_decision_time(latency(100), UnixNanos::from(1_000));
        assert_eq!(rank.our_arrival, UnixNanos::from(1_100));
        assert_eq!(rank.rank, 2);
        assert_eq!(rank.ahead, 1);
        assert_eq!(rank.tied, 1);
        assert_eq!(rank.behind, 2);
        assert!(!rank.arrived_first);
        // Closest later arrival is 120ns, i.e. 20ns after ours.
        assert_eq!(rank.gap_to_closest_behind, Some(latency(20)));
    }

    #[rstest]
    fn test_rank_is_ordinal_under_shared_offset_and_scaling() {
        let set = CompetitorSet::new(vec![latency(40), latency(80), latency(120)]);
        let shifted = CompetitorSet::new(vec![latency(140), latency(180), latency(220)]);
        let scaled = CompetitorSet::new(vec![latency(80), latency(160), latency(240)]);

        assert_eq!(set.rank(latency(100)), 3);
        // Adding the same offset to our latency and every rival's leaves the rank unchanged.
        assert_eq!(shifted.rank(latency(200)), set.rank(latency(100)));
        // Scaling every latency by a constant leaves the rank unchanged.
        assert_eq!(scaled.rank(latency(200)), set.rank(latency(100)));
    }

    #[rstest]
    fn test_uniform_builds_the_declared_count() {
        let set = CompetitorSet::uniform(3, latency(75));

        assert_eq!(set.len(), 3);
        assert_eq!(set.competitors(), &[latency(75); 3]);
        assert_eq!(set.rank(latency(75)), 1);
        assert_eq!(set.rank(latency(76)), 4);
    }

    #[rstest]
    fn test_handle_forwards_the_read() {
        let handle = CompetitorSetHandle::new(CompetitorSet::new(vec![latency(50)]));
        let cloned = handle.clone();
        drop(handle);

        assert!(!cloned.is_empty());
        assert_eq!(cloned.len(), 1);
        assert_eq!(cloned.rank(latency(100)), 2);
        assert!(
            !cloned
                .rank_at_decision_time(latency(100), UnixNanos::from(1))
                .arrived_first
        );
    }
}
