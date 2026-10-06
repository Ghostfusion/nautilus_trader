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

//! Cross-venue lead-lag with the bound it has to carry.
//!
//! A lead-lag number between two venues means nothing without the offset uncertainty of the vantage
//! that produced it: one vantage cannot separate a clock offset from the propagation delay, so the
//! same measurement can show an edge that is really the offset being wrong. The corpus records an
//! apparent 16 ms edge that a +/-99 ms single-vantage ambiguity reduces to a null, with drift
//! bounded to 6 ms over the window.
//!
//! There is therefore no way to ask this module for a number without a bound. [`CrossVenueOffset`]
//! is the only evidence [`cross_venue_lead_lag`] accepts, and its only constructor takes a
//! [`ClockOffsetEstimate`] from `nautilus_common::clock::ClockOffsetEstimator` plus the pairing
//! ambiguity the caller owns, so an unestimated read cannot be requested at all. A read whose peak
//! sits inside the ambiguity is reported as [`LeadLagVerdict::Unresolved`] with the bound that
//! swallowed it rather than as an edge, and the drift bound travels on the report beside the
//! ambiguity.
//!
//! The estimator is Hayashi-Yoshida style: each series is reduced to the returns over the intervals
//! between its own events, the two interval grids are paired within a declared window at each lag
//! on a grid, and the lag whose paired returns correlate highest is the read. A positive lag means
//! the first venue leads the second. The read is repeated at every pairing window the caller
//! supplies, because the window is a choice that can move the answer, and a ladder whose windows
//! disagree is visible through [`LeadLagReport::is_stable`] rather than averaged away.
//!
//! The grid is walked in full at every window, with one nearest-interval search per interval, so
//! the cost is the grid size times the product of the two interval counts. That is a research
//! cost, not an engine one: nothing here runs on the replay path.

use std::fmt::{Display, Formatter};

use anyhow::{Result, ensure};
use nautilus_common::clock::ClockOffsetEstimate;
use nautilus_core::UnixNanos;
use nautilus_model::types::Price;

/// The offset a cross-venue read is paired under, with the bounds it cannot escape.
///
/// The ambiguity is the caller's declaration of what the vantage cannot resolve: the offset is
/// estimated from paired timestamps, and a single vantage cannot tell the offset from the
/// propagation delay, so a lag smaller than the ambiguity is not evidence of anything. The drift
/// bound is not declared, it is carried from the [`ClockOffsetEstimate`] the offset came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CrossVenueOffset {
    offset_ns: i64,
    ambiguity_ns: u64,
    drift_bound_ns: u64,
    samples: usize,
}

impl CrossVenueOffset {
    /// Declares the pairing ambiguity against an estimated clock offset.
    ///
    /// This is the only way to obtain the evidence a read requires, so a caller that has not
    /// estimated the offset cannot request a read at all.
    #[must_use]
    pub fn new(estimate: ClockOffsetEstimate, ambiguity_ns: u64) -> Self {
        Self {
            offset_ns: estimate.offset_ns,
            ambiguity_ns,
            drift_bound_ns: estimate.drift_bound_ns,
            samples: estimate.samples,
        }
    }

    /// Returns the offset applied to the second venue's timestamps (local minus venue).
    #[must_use]
    pub fn offset_ns(&self) -> i64 {
        self.offset_ns
    }

    /// Returns the ambiguity a single vantage cannot resolve, symmetric about the offset.
    #[must_use]
    pub fn ambiguity_ns(&self) -> u64 {
        self.ambiguity_ns
    }

    /// Returns the drift bound the offset estimate showed over its window.
    #[must_use]
    pub fn drift_bound_ns(&self) -> u64 {
        self.drift_bound_ns
    }

    /// Returns the number of samples the offset estimate was drawn from.
    #[must_use]
    pub fn samples(&self) -> usize {
        self.samples
    }

    /// Returns whether a lag of the given magnitude is inside the ambiguity.
    #[must_use]
    pub fn is_within_ambiguity(&self, lag_ns: i64) -> bool {
        lag_ns.unsigned_abs() <= self.ambiguity_ns
    }

    /// Returns whether a lag of the given magnitude is inside the drift the offset showed.
    #[must_use]
    pub fn is_within_drift(&self, lag_ns: i64) -> bool {
        lag_ns.unsigned_abs() <= self.drift_bound_ns
    }
}

/// One venue's observation of the same data class for the same instrument.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrossVenueEvent {
    /// The instant the venue stamped the event, on the venue's own clock.
    pub ts_event: UnixNanos,
    /// The price the event carries.
    pub price: Price,
}

/// The lag grid and the minimum evidence a read must have.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LeadLagConfig {
    /// The largest lag the grid tests, in nanoseconds; the grid is symmetric about zero.
    pub max_lag_ns: u64,
    /// The grid's spacing, in nanoseconds.
    pub lag_step_ns: u64,
    /// The smallest number of paired returns a read may be reported from.
    pub min_pairs: usize,
}

impl LeadLagConfig {
    /// Creates a config, refusing a grid that cannot be walked.
    ///
    /// # Errors
    ///
    /// Returns an error if `lag_step_ns` is zero, if `max_lag_ns` is smaller than the step, or if
    /// `min_pairs` is zero.
    pub fn new(max_lag_ns: u64, lag_step_ns: u64, min_pairs: usize) -> Result<Self> {
        ensure!(lag_step_ns > 0, "lag_step_ns must be positive");
        ensure!(
            max_lag_ns >= lag_step_ns,
            "max_lag_ns must be at least lag_step_ns"
        );
        ensure!(min_pairs > 0, "min_pairs must be positive");

        Ok(Self {
            max_lag_ns,
            lag_step_ns,
            min_pairs,
        })
    }
}

/// What a read can be said to be, given the bound it carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeadLagVerdict {
    /// The peak lag is outside the pairing ambiguity, so it can be read as a lead-lag.
    Resolved,
    /// The peak lag is inside the pairing ambiguity, so the read is a null with the bound that
    /// would have to narrow before the peak would mean anything.
    Unresolved,
    /// Too few returns paired at this window for a peak to be read at all.
    Insufficient,
}

impl Display for LeadLagVerdict {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Resolved => f.write_str("resolved"),
            Self::Unresolved => f.write_str("unresolved"),
            Self::Insufficient => f.write_str("insufficient"),
        }
    }
}

/// One read of the lead-lag at one pairing window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LeadLagRead {
    /// The pairing window this read used, in nanoseconds.
    pub pairing_window_ns: u64,
    /// The lag of the highest correlation on the grid, when one was readable.
    pub peak_lag_ns: Option<i64>,
    /// The correlation at the peak, when one was readable.
    pub peak_correlation: Option<f64>,
    /// The number of returns paired behind the peak.
    pub paired_returns: usize,
    /// Whether the peak sits inside the drift bound the offset showed.
    pub within_drift: bool,
    /// What the read can be said to be.
    pub verdict: LeadLagVerdict,
}

/// A cross-venue lead-lag read with the evidence it was paired under.
#[derive(Clone, Debug, PartialEq)]
pub struct LeadLagReport {
    /// The offset and the bounds every read in the report carries.
    pub offset: CrossVenueOffset,
    /// The grid the reads were taken over.
    pub config: LeadLagConfig,
    /// One read per pairing window, in the order the windows were supplied.
    pub reads: Vec<LeadLagRead>,
}

impl LeadLagReport {
    /// Returns whether every read reached the same verdict.
    ///
    /// The pairing window is a choice, so a ladder whose windows disagree is a read that is not
    /// stable under that choice; this reports the disagreement rather than hiding it in an average.
    #[must_use]
    pub fn is_stable(&self) -> bool {
        let mut verdicts = self.reads.iter().map(|read| read.verdict);
        match verdicts.next() {
            None => true,
            Some(first) => verdicts.all(|verdict| verdict == first),
        }
    }

    /// Returns the lag of the first read that resolved outside the ambiguity.
    ///
    /// This is the only accessor that hands out a point estimate, and it hands one out only from a
    /// read the bound did not swallow, so the report's [`CrossVenueOffset`] is always available
    /// beside it.
    #[must_use]
    pub fn resolved_lag_ns(&self) -> Option<i64> {
        self.reads
            .iter()
            .find(|read| read.verdict == LeadLagVerdict::Resolved)
            .and_then(|read| read.peak_lag_ns)
    }
}

/// Measures the lead-lag of `venue_a` against `venue_b` under a declared offset.
///
/// A positive lag means `venue_a` leads `venue_b`: a return on `venue_a` at `t` pairs with a return
/// on `venue_b` at `t + lag`. Every timestamp of `venue_b` is first moved onto the local clock by
/// the offset, so the read is of the lag and not of the offset error.
///
/// Both slices must hold the same data class for the same instrument, ordered by `ts_event`. The
/// pairing window is the largest separation at which an interval of one series is treated as the
/// same interval as one of the other; at each grid lag every return of `venue_a` is paired with the
/// single nearest return of `venue_b` within that window, and only the highest correlation on the
/// grid is read. A negative peak is not a lead-lag and is not separated out here, so a caller that
/// needs the sign should read [`LeadLagRead::peak_correlation`] beside the lag.
///
/// Returns one read per pairing window, in the order supplied, with the evidence on the report.
#[must_use]
pub fn cross_venue_lead_lag(
    venue_a: &[CrossVenueEvent],
    venue_b: &[CrossVenueEvent],
    pairing_windows_ns: &[u64],
    config: &LeadLagConfig,
    offset: &CrossVenueOffset,
) -> LeadLagReport {
    let a = intervals(venue_a, 0);
    let b = intervals(venue_b, offset.offset_ns());

    let reads = pairing_windows_ns
        .iter()
        .map(|window_ns| read_at_window(&a, &b, *window_ns, config, offset))
        .collect();

    LeadLagReport {
        offset: *offset,
        config: *config,
        reads,
    }
}

/// A return over the interval between two consecutive events, on the local clock.
#[derive(Clone, Copy, Debug)]
struct Interval {
    /// The instant the interval starts.
    start_ns: i64,
    /// The price return the interval carries.
    ret: f64,
}

/// Reduces a series to its interval returns, moving each timestamp by `offset_ns`.
///
/// A series of `n` events gives `n - 1` intervals. An interval whose opening price is zero carries
/// no return and is left out rather than divided by.
fn intervals(events: &[CrossVenueEvent], offset_ns: i64) -> Vec<Interval> {
    let mut out = Vec::with_capacity(events.len().saturating_sub(1));

    for pair in events.windows(2) {
        let (previous, next) = (pair[0], pair[1]);
        let previous_px = previous.price.as_f64();
        if previous_px == 0.0 {
            continue;
        }

        out.push(Interval {
            start_ns: to_local(previous.ts_event, offset_ns),
            ret: (next.price.as_f64() - previous_px) / previous_px,
        });
    }

    out
}

/// Moves a timestamp onto the local clock, saturating rather than wrapping.
fn to_local(ts_event: UnixNanos, offset_ns: i64) -> i64 {
    let raw = i128::from(ts_event.as_u64()) + i128::from(offset_ns);

    match i64::try_from(raw) {
        Ok(value) => value,
        Err(_) if raw < 0 => i64::MIN,
        Err(_) => i64::MAX,
    }
}

/// Returns the correlation of the paired returns at `lag` and the number of pairs behind it.
fn paired_at(a: &[Interval], b: &[Interval], lag_ns: i64, window_ns: u64) -> (Option<f64>, usize) {
    let window = i128::from(window_ns);
    let mut xs = Vec::with_capacity(a.len());
    let mut ys = Vec::with_capacity(a.len());

    for interval_a in a {
        let target = i128::from(interval_a.start_ns) + i128::from(lag_ns);
        let mut closest: Option<(&Interval, i128)> = None;

        for interval_b in b {
            let separation = (i128::from(interval_b.start_ns) - target).abs();
            let better = match closest {
                None => true,
                Some((_, best)) => separation < best,
            };
            if separation <= window && better {
                closest = Some((interval_b, separation));
            }
        }

        if let Some((interval_b, _)) = closest {
            xs.push(interval_a.ret);
            ys.push(interval_b.ret);
        }
    }

    let count = xs.len();
    if count < 2 {
        return (None, count);
    }

    let divisor = count as f64;
    let mean_x = xs.iter().sum::<f64>() / divisor;
    let mean_y = ys.iter().sum::<f64>() / divisor;

    let mut covariance = 0.0;
    let mut variance_x = 0.0;
    let mut variance_y = 0.0;
    for (x, y) in xs.iter().zip(ys.iter()) {
        covariance += (x - mean_x) * (y - mean_y);
        variance_x += (x - mean_x).powi(2);
        variance_y += (y - mean_y).powi(2);
    }

    // A flat run of returns at either side has no correlation to report.
    if variance_x == 0.0 || variance_y == 0.0 {
        return (None, count);
    }

    (
        Some(covariance / (variance_x.sqrt() * variance_y.sqrt())),
        count,
    )
}

/// Reads the highest correlation on the grid at one pairing window.
fn read_at_window(
    a: &[Interval],
    b: &[Interval],
    window_ns: u64,
    config: &LeadLagConfig,
    offset: &CrossVenueOffset,
) -> LeadLagRead {
    let step = i64::try_from(config.lag_step_ns).unwrap_or(i64::MAX);
    let max = i64::try_from(config.max_lag_ns).unwrap_or(i64::MAX);
    let mut best: Option<(i64, f64, usize)> = None;
    let mut lag = -max;

    loop {
        let (correlation, pairs) = paired_at(a, b, lag, window_ns);

        if let Some(correlation) = correlation
            && pairs >= config.min_pairs
            && best.is_none_or(|(_, best_correlation, _)| correlation > best_correlation)
        {
            best = Some((lag, correlation, pairs));
        }

        if lag >= max {
            break;
        }
        lag = lag.saturating_add(step).min(max);
    }

    let Some((lag_ns, correlation, paired_returns)) = best else {
        return LeadLagRead {
            pairing_window_ns: window_ns,
            peak_lag_ns: None,
            peak_correlation: None,
            paired_returns: 0,
            within_drift: false,
            verdict: LeadLagVerdict::Insufficient,
        };
    };

    let verdict = if offset.is_within_ambiguity(lag_ns) {
        LeadLagVerdict::Unresolved
    } else {
        LeadLagVerdict::Resolved
    };

    LeadLagRead {
        pairing_window_ns: window_ns,
        peak_lag_ns: Some(lag_ns),
        peak_correlation: Some(correlation),
        paired_returns,
        within_drift: offset.is_within_drift(lag_ns),
        verdict,
    }
}

#[cfg(test)]
mod tests {
    use nautilus_common::clock::ClockOffsetEstimate;

    use super::*;

    const MILLIS: u64 = 1_000_000;

    /// Builds the evidence a read requires around a stated estimate.
    fn evidence(offset_ns: i64, ambiguity_ns: u64, drift_bound_ns: u64) -> CrossVenueOffset {
        CrossVenueOffset::new(
            ClockOffsetEstimate {
                offset_ns,
                drift_bound_ns,
                samples: 240,
            },
            ambiguity_ns,
        )
    }

    fn config() -> LeadLagConfig {
        LeadLagConfig::new(20 * MILLIS, MILLIS, 10).expect("a walked grid")
    }

    /// Builds a deterministic price path, so a test needs no random source.
    fn events(start_ns: u64, shift_ns: i64, count: usize, seed: u64) -> Vec<CrossVenueEvent> {
        let mut state = seed;
        let mut price = 100.00;
        let mut out = Vec::with_capacity(count);

        for index in 0..count {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);

            // A varied step on a cent scale: a smooth path correlates at many lags, so the peak
            // would not be unique and the test would prove nothing about the lag it reads.
            let draw = f64::from(u32::try_from(state >> 33).unwrap()) / 2_147_483_648.0 - 0.5;
            price += draw;

            let ts =
                i128::from(start_ns) + index as i128 * i128::from(MILLIS) + i128::from(shift_ns);
            out.push(CrossVenueEvent {
                ts_event: UnixNanos::from(u64::try_from(ts).unwrap()),
                price: Price::new(price, 2),
            });
        }

        out
    }

    #[test]
    fn resolved_when_the_lag_is_outside_the_ambiguity() {
        let a = events(1_000_000_000, 0, 60, 7);
        let b = events(1_000_000_000, 5 * MILLIS as i64, 60, 7);
        let report = cross_venue_lead_lag(&a, &b, &[100_000], &config(), &evidence(0, MILLIS, 0));

        let read = report.reads[0];
        assert_eq!(read.verdict, LeadLagVerdict::Resolved);
        assert_eq!(read.peak_lag_ns, Some(5 * MILLIS as i64));
        assert!((read.peak_correlation.unwrap() - 1.0).abs() < 1e-9);
        assert_eq!(read.paired_returns, 59);
        assert!(!read.within_drift);
        assert_eq!(report.resolved_lag_ns(), Some(5 * MILLIS as i64));
        assert!(report.is_stable());
        assert_eq!(report.offset.samples(), 240);
    }

    #[test]
    fn the_corpus_null_is_unresolved_rather_than_an_edge() {
        let a = events(1_000_000_000, 0, 60, 11);
        let b = events(1_000_000_000, 16 * MILLIS as i64, 60, 11);

        // The 16 ms shift is a real peak in the data, but a +/-99 ms single-vantage ambiguity
        // swallows it, and the 6 ms drift bound does not explain it either.
        let report = cross_venue_lead_lag(
            &a,
            &b,
            &[100_000],
            &config(),
            &evidence(0, 99 * MILLIS, 6 * MILLIS),
        );

        let read = report.reads[0];
        assert_eq!(read.peak_lag_ns, Some(16 * MILLIS as i64));
        assert!((read.peak_correlation.unwrap() - 1.0).abs() < 1e-9);
        assert_eq!(read.verdict, LeadLagVerdict::Unresolved);
        assert!(!read.within_drift);

        // No point estimate is published for a read the bound swallowed.
        assert_eq!(report.resolved_lag_ns(), None);
        assert_eq!(report.offset.ambiguity_ns(), 99 * MILLIS);
        assert_eq!(report.offset.drift_bound_ns(), 6 * MILLIS);
    }

    #[test]
    fn a_lag_inside_the_drift_is_flagged_beside_the_ambiguity() {
        let a = events(1_000_000_000, 0, 60, 13);
        let b = events(1_000_000_000, 2 * MILLIS as i64, 60, 13);
        let report = cross_venue_lead_lag(
            &a,
            &b,
            &[100_000],
            &config(),
            &evidence(0, MILLIS, 3 * MILLIS),
        );

        let read = report.reads[0];
        assert_eq!(read.verdict, LeadLagVerdict::Resolved);
        assert!(read.within_drift);
        assert_eq!(read.peak_lag_ns, Some(2 * MILLIS as i64));
    }

    #[test]
    fn the_declared_offset_is_removed_before_the_lag_is_measured() {
        // The second venue's clock runs 37 ms ahead, so the same 5 ms lag arrives as 42 ms and the
        // offset (local minus venue) is negative.
        let a = events(1_000_000_000, 0, 60, 17);
        let b = events(1_000_000_000, 42 * MILLIS as i64, 60, 17);
        let report = cross_venue_lead_lag(
            &a,
            &b,
            &[100_000],
            &config(),
            &evidence(-37 * MILLIS as i64, MILLIS, 0),
        );

        assert_eq!(report.reads[0].peak_lag_ns, Some(5 * MILLIS as i64));
        assert_eq!(report.reads[0].verdict, LeadLagVerdict::Resolved);
    }

    #[test]
    fn a_window_that_pairs_nothing_is_insufficient_rather_than_zero() {
        // The grid steps a whole millisecond, so a half-millisecond offset only pairs within a
        // window wider than that.
        let a = events(1_000_000_000, 0, 60, 19);
        let b = events(1_000_000_000, 5 * MILLIS as i64 + 500, 60, 19);
        let report = cross_venue_lead_lag(&a, &b, &[1, 1_000], &config(), &evidence(0, MILLIS, 0));

        assert_eq!(report.reads[0].verdict, LeadLagVerdict::Insufficient);
        assert_eq!(report.reads[0].peak_lag_ns, None);
        assert_eq!(report.reads[1].verdict, LeadLagVerdict::Resolved);
        assert_eq!(report.reads[1].peak_lag_ns, Some(5 * MILLIS as i64));

        // Two windows that disagree report the disagreement rather than a number.
        assert!(!report.is_stable());
    }

    #[test]
    fn a_config_that_cannot_walk_its_grid_is_refused() {
        assert!(LeadLagConfig::new(10 * MILLIS, 0, 1).is_err());
        assert!(LeadLagConfig::new(MILLIS - 1, MILLIS, 1).is_err());
        assert!(LeadLagConfig::new(10 * MILLIS, MILLIS, 0).is_err());
        assert!(LeadLagConfig::new(10 * MILLIS, MILLIS, 1).is_ok());
    }
}
