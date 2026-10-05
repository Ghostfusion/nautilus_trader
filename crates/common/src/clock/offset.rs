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

//! Cross-venue clock offset estimation.
//!
//! A pair of timestamps taken on two clocks cannot separate the offset between them from the delay
//! standing between them, so the estimator reports the offset together with the bound the evidence
//! supports, and refuses to estimate until it has evidence. The estimate is what a cross-venue
//! comparison needs before it can claim anything about which venue moved first.

use std::collections::VecDeque;

/// The number of samples in one bucket.
///
/// A bucket is the unit of the drift window: its minimum is one reading of the offset.
pub const DEFAULT_BUCKET_SAMPLES: usize = 8;

/// The smallest number of completed buckets an estimate may be drawn from.
///
/// One reading is not evidence of a bound, so the estimator refuses to estimate below this.
pub const MIN_BUCKETS: usize = 3;

/// The number of buckets retained, i.e. the drift window.
const WINDOW_BUCKETS: usize = 12;

/// A cross-venue clock offset estimate with the bound its samples support.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClockOffsetEstimate {
    /// The venue clock's offset from the local clock (local minus venue).
    pub offset_ns: i64,
    /// The change between the per-bucket offsets over the window, i.e. the drift it shows.
    pub drift_bound_ns: u64,
    /// The number of samples the estimate is drawn from.
    pub samples: usize,
}

/// Estimates the offset between a venue's clock and the local clock from paired timestamps.
///
/// A sample delayed in flight can only make the venue's timestamp look older than it is, so the
/// smallest offset seen in a bucket is the least contaminated reading of it; this is the minimum
/// filter the estimator applies, and the spread of the per-bucket minima is what it reports as the
/// drift bound. The bound is therefore what the samples show about the offset moving, and it is
/// never narrower than the pairing's own noise.
///
/// The estimator refuses to estimate from fewer than [`MIN_BUCKETS`] completed buckets, because a
/// bound drawn from less is a bound the data cannot support.
#[derive(Clone, Debug)]
pub struct ClockOffsetEstimator {
    bucket_samples: usize,
    current_min: Option<i64>,
    current_count: usize,
    buckets: VecDeque<i64>,
    window_samples: usize,
}

impl Default for ClockOffsetEstimator {
    fn default() -> Self {
        Self::new(DEFAULT_BUCKET_SAMPLES)
    }
}

impl ClockOffsetEstimator {
    /// Creates a new [`ClockOffsetEstimator`] with the given number of samples per bucket.
    ///
    /// # Panics
    ///
    /// Panics if `bucket_samples` is zero.
    #[must_use]
    pub fn new(bucket_samples: usize) -> Self {
        assert!(bucket_samples > 0, "bucket_samples must be positive");

        Self {
            bucket_samples,
            current_min: None,
            current_count: 0,
            buckets: VecDeque::with_capacity(WINDOW_BUCKETS),
            window_samples: 0,
        }
    }

    /// Observes a pair of timestamps taken from the same event on the venue clock and the local
    /// clock.
    ///
    /// A pair whose difference does not fit the offset representation (about 292 years of
    /// nanoseconds) is not a clock offset, so it is discarded as a bad timestamp rather than
    /// allowed to widen every bound that follows it.
    pub fn observe(&mut self, venue_ns: u64, local_ns: u64) {
        let Ok(raw) = i64::try_from(local_ns as i128 - venue_ns as i128) else {
            return;
        };

        self.current_min = Some(match self.current_min {
            Some(current) => current.min(raw),
            None => raw,
        });
        self.current_count += 1;

        if self.current_count == self.bucket_samples
            && let Some(minimum) = self.current_min.take()
        {
            self.buckets.push_back(minimum);
            self.window_samples += self.bucket_samples;

            while self.buckets.len() > WINDOW_BUCKETS {
                self.buckets.pop_front();
                self.window_samples -= self.bucket_samples;
            }

            self.current_count = 0;
        }
    }

    /// Returns the estimate, or `None` until [`MIN_BUCKETS`] buckets have completed.
    #[must_use]
    pub fn estimate(&self) -> Option<ClockOffsetEstimate> {
        if self.buckets.len() < MIN_BUCKETS {
            return None;
        }

        let mut offset_ns = i64::MAX;
        let mut highest = i64::MIN;

        for bucket in &self.buckets {
            offset_ns = offset_ns.min(*bucket);
            highest = highest.max(*bucket);
        }

        let drift_bound_ns =
            u64::try_from((highest as i128 - offset_ns as i128).unsigned_abs()).unwrap_or(u64::MAX);

        Some(ClockOffsetEstimate {
            offset_ns,
            drift_bound_ns,
            samples: self.window_samples,
        })
    }

    /// Clears every sample, so the estimator starts again.
    pub fn reset(&mut self) {
        self.current_min = None;
        self.current_count = 0;
        self.buckets.clear();
        self.window_samples = 0;
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    fn test_estimate_is_absent_until_three_buckets_complete() {
        let mut estimator = ClockOffsetEstimator::new(2);

        for local in [1_000u64, 1_001, 1_002, 1_003] {
            estimator.observe(0, local);
        }
        assert_eq!(estimator.estimate(), None);

        for local in [1_004u64, 1_005] {
            estimator.observe(0, local);
        }
        assert_eq!(
            estimator.estimate(),
            Some(ClockOffsetEstimate {
                offset_ns: 1_000,
                drift_bound_ns: 4,
                samples: 6,
            })
        );
    }

    #[rstest]
    fn test_minimum_filter_selects_the_least_contaminated_sample() {
        let mut estimator = ClockOffsetEstimator::new(1);

        // A delay in flight inflates the observed offset; only the smallest reading is uncontaminated.
        for local in [100u64, 1_100, 1_050, 900] {
            estimator.observe(0, local);
        }

        let estimate = estimator.estimate().unwrap();
        assert_eq!(estimate.offset_ns, 100);
        assert_eq!(estimate.drift_bound_ns, 1_000);
        assert_eq!(estimate.samples, 4);
    }

    #[rstest]
    fn test_bucket_minimum_suppresses_a_single_delayed_sample() {
        let mut estimator = ClockOffsetEstimator::new(2);

        // One delayed sample cannot raise its bucket's reading above the other sample in it.
        for local in [10u64, 60, 12, 500, 14, 15] {
            estimator.observe(0, local);
        }

        let estimate = estimator.estimate().unwrap();
        assert_eq!(estimate.offset_ns, 10);
        assert_eq!(estimate.drift_bound_ns, 4);
    }

    #[rstest]
    fn test_offset_is_signed_when_the_venue_clock_is_ahead() {
        let mut estimator = ClockOffsetEstimator::new(1);

        // The venue reports 1_000 ns later than the local clock, so local minus venue is negative.
        for local in [100u64, 101, 102] {
            estimator.observe(local + 1_000, local);
        }

        let estimate = estimator.estimate().unwrap();
        assert_eq!(estimate.offset_ns, -1_000);
        assert_eq!(estimate.drift_bound_ns, 0);
    }

    #[rstest]
    fn test_a_difference_that_is_not_a_clock_offset_is_discarded() {
        let mut estimator = ClockOffsetEstimator::new(1);

        // A difference beyond the offset representation is a bad timestamp, not a clock offset.
        estimator.observe(u64::MAX, 0);
        estimator.observe(0, u64::MAX);

        for local in [1_000u64, 1_000, 1_000] {
            estimator.observe(0, local);
        }

        assert_eq!(
            estimator.estimate(),
            Some(ClockOffsetEstimate {
                offset_ns: 1_000,
                drift_bound_ns: 0,
                samples: 3,
            })
        );
    }

    #[rstest]
    fn test_window_drops_the_oldest_bucket() {
        let mut estimator = ClockOffsetEstimator::new(1);

        estimator.observe(0, 1_000_000);
        for local in [10u64, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21] {
            estimator.observe(0, local);
        }

        let estimate = estimator.estimate().unwrap();
        assert_eq!(estimate.offset_ns, 10);
        assert_eq!(estimate.drift_bound_ns, 11);
        assert_eq!(estimate.samples, WINDOW_BUCKETS);
    }

    #[rstest]
    fn test_reset_clears_the_estimate() {
        let mut estimator = ClockOffsetEstimator::default();

        for _ in 0..(DEFAULT_BUCKET_SAMPLES * 2) {
            estimator.observe(0, 1_000);
        }
        assert!(estimator.estimate().is_none());

        for _ in 0..(DEFAULT_BUCKET_SAMPLES * 2) {
            estimator.observe(0, 1_001);
        }
        assert!(estimator.estimate().is_some());

        estimator.reset();
        assert_eq!(estimator.estimate(), None);
    }
}
