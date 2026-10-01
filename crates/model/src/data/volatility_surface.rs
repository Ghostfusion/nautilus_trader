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

//! Implied volatility surface calibration and querying.
//!
//! The surface is built from *option chain observations* - one contract each with a bid, an ask,
//! an optional volume, and an observation timestamp. Observations pass through a set of **named
//! filters**; every rejected observation is counted by its rejection reason rather than dropped
//! silently. Surviving observations are mapped to the standard no-arbitrage coordinates: total
//! implied variance `w = sigma^2 * T` against log moneyness `k = ln(K / F)`, with time to expiry
//! `T` as the second axis. The forward `F` is either supplied or derived from put-call parity
//! through [`implied_forward_from_parity`], and the implied volatility comes from the repository's
//! implied-vol solver [`imply_vol`].
//!
//! # Interpolation
//!
//! The default slice representation is **piecewise-linear total variance against log moneyness**.
//! It reproduces every node exactly and preserves the data's segment ordering and convexity, so it
//! cannot introduce a butterfly arbitrage that the observations did not already contain: the
//! density is a sum of (non-negative, by convexity) point masses at the nodes plus a continuous
//! part that is checked directly on each segment.
//!
//! A slice whose nodes cannot satisfy the no-arbitrage conditions under that family is refitted
//! with the **SVI** (stochastic-volatility-inspired) raw total-variance family
//! [`SviParams::total_variance`], fitted by a bounded deterministic grid search that solves the
//! two linear parameters in closed form and then checks the family's own arbitrage conditions.
//! Failing that, the surface is refused with a typed error - it is never repaired silently.
//!
//! # Extrapolation
//!
//! Total variance is held flat beyond the quoted moneyness range and beyond the quoted expiry
//! range. A continued spline is *not* equivalent: a continuation would extrapolate the local
//! shape and can introduce arbitrage, while a flat total variance keeps the boundary value and so
//! cannot create a crossing. Results outside the quoted region carry `extrapolated: true`.
//!
//! # Calendar consistency
//!
//! For each log moneyness the total variance must be non-decreasing in time to expiry across
//! slices. A surface that violates this is refused, never repaired.

use std::collections::BTreeMap;

use nautilus_core::UnixNanos;
use serde::{Deserialize, Serialize};

use super::{greeks::imply_vol, pricing::implied_forward_from_parity};
use crate::enums::OptionKind;

/// The minimum number of usable observations required per expiry slice.
///
/// Three points are the minimum that can carry a convexity (no-butterfly) statement.
pub const MIN_OBSERVATIONS_PER_SLICE: usize = 3;

/// The minimum number of expiry slices required to carry a calendar statement.
pub const MIN_SLICES_FOR_CALENDAR: usize = 2;

/// Tolerance used when comparing total variances and implied densities against zero.
const ARBITRAGE_TOLERANCE: f64 = 1e-10;

/// Log-moneyness values closer than this are treated as the same node and averaged.
const LOG_MONEYNESS_TOLERANCE: f64 = 1e-9;

/// Number of sample points taken across each linear segment when checking Durrleman's density.
const SEGMENT_SAMPLES: usize = 8;

/// Number of sample points used across a slice range when checking the SVI density and calendar.
const RANGE_SAMPLES: usize = 64;

const NANOSECONDS_PER_SECOND: f64 = 1_000_000_000.0;
const SECONDS_PER_YEAR: f64 = 365.25 * 24.0 * 60.0 * 60.0;
const NANOSECONDS_PER_YEAR: f64 = SECONDS_PER_YEAR * NANOSECONDS_PER_SECOND;

/// A single option chain observation used to calibrate a volatility surface.
///
/// Prices are `f64` because the implied-vol solver in [`super::greeks`] is defined on `f64`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SurfaceObservation {
    /// The instrument the observation belongs to.
    pub instrument_id: crate::identifiers::InstrumentId,
    /// The option strike price.
    pub strike: f64,
    /// The contract expiration, as a UNIX timestamp (nanoseconds).
    pub expiry_ns: UnixNanos,
    /// The option kind (call or put).
    pub option_kind: OptionKind,
    /// The top-of-book bid price.
    pub bid: f64,
    /// The top-of-book ask price.
    pub ask: f64,
    /// The traded volume, when available.
    pub volume: Option<f64>,
    /// The observation timestamp, as a UNIX timestamp (nanoseconds).
    pub ts_event: UnixNanos,
}

/// A named reason an observation was rejected by the surface filters.
///
/// Rejected observations are counted by reason; the enumeration is closed so an unexpected
/// rejection cannot be recorded as an unnamed bucket.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceRejection {
    /// The bid, ask, or strike was not finite, or the strike was not positive.
    InvalidQuote,
    /// The bid exceeded the ask.
    CrossedMarket,
    /// The bid fell below the configured minimum bid.
    BidBelowMinimum,
    /// The spread exceeded the configured maximum spread.
    SpreadAboveMaximum,
    /// The time to expiry fell below the configured minimum (or was non-positive).
    TimeToExpiryBelowMinimum,
    /// The volume fell below the configured minimum, where volume was available.
    VolumeBelowMinimum,
    /// The observation was older than the configured maximum age.
    StaleObservation,
    /// The implied-vol solver could not produce a positive, finite volatility.
    InvalidImpliedVolatility,
}

/// Per-reason rejection counts for a calibration, plus the accepted total.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RejectionCounts {
    counts: BTreeMap<SurfaceRejection, usize>,
    accepted: usize,
}

impl RejectionCounts {
    /// Returns the number of observations rejected for `reason`.
    #[must_use]
    pub fn count(&self, reason: SurfaceRejection) -> usize {
        self.counts.get(&reason).copied().unwrap_or(0)
    }

    /// Returns the number of accepted observations.
    #[must_use]
    pub fn accepted(&self) -> usize {
        self.accepted
    }

    /// Returns the total number of rejected observations.
    #[must_use]
    pub fn rejected(&self) -> usize {
        self.counts.values().sum()
    }

    /// Returns the total number of observations seen (accepted and rejected).
    #[must_use]
    pub fn total(&self) -> usize {
        self.accepted + self.rejected()
    }

    /// Iterates over the non-zero rejection counts by reason.
    pub fn iter(&self) -> impl Iterator<Item = (SurfaceRejection, usize)> + '_ {
        self.counts.iter().map(|(k, v)| (*k, *v))
    }

    fn record(&mut self, reason: SurfaceRejection) {
        *self.counts.entry(reason).or_insert(0) += 1;
    }
}

/// The price fed to the implied-vol solver when forming a node.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfacePriceSource {
    /// Use the mid price `(bid + ask) / 2`.
    #[default]
    Mid,
    /// Solve from the bid and the ask separately and take the mid of the two total variances.
    ///
    /// Both legs must solve; an observation where either leg fails is rejected.
    BidAsk,
}

/// How the per-slice forward `F` is obtained.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ForwardSource {
    /// A single forward supplied by the caller, applied to every slice.
    Supplied(f64),
    /// Derive the forward per slice from put-call parity, taking the median over strikes where
    /// both a call and a put quote exist. Requires European exercise.
    PutCallParity,
}

/// The fallback family selected when the interpolation family cannot make a slice satisfy the
/// no-arbitrage conditions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SliceFallback {
    /// Refuse the surface.
    Refuse,
    /// Refit the slice with the arbitrage-free SVI total-variance family.
    #[default]
    Svi,
}

/// Declares when a surface should be recalibrated.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CalibrationTrigger {
    /// Recalibrate only when explicitly requested (the default).
    #[default]
    OnDemand,
    /// Recalibrate once at least `seconds` have elapsed since the last calibration.
    Interval {
        /// The minimum elapsed time, in seconds.
        seconds: f64,
    },
    /// Recalibrate once at least `count` new observations have arrived.
    OnNewObservations {
        /// The minimum number of new observations.
        count: usize,
    },
}

impl CalibrationTrigger {
    /// Returns whether a recalibration is due given the elapsed time and new observation count.
    #[must_use]
    pub fn is_due(&self, elapsed_secs: f64, new_observations: usize) -> bool {
        match self {
            Self::OnDemand => false,
            Self::Interval { seconds } => elapsed_secs >= *seconds,
            Self::OnNewObservations { count } => new_observations >= *count,
        }
    }
}

/// The declarative limits applied to observed quotes before calibration.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SurfaceFilters {
    /// The minimum acceptable bid price.
    pub min_bid: f64,
    /// The maximum acceptable spread `ask - bid`.
    pub max_spread: f64,
    /// The minimum acceptable time to expiry, in seconds.
    pub min_time_to_expiry_secs: f64,
    /// The minimum acceptable volume, applied only where volume is available.
    pub min_volume: f64,
    /// The maximum acceptable observation age, in seconds, measured against the reference time.
    pub max_age_secs: f64,
}

impl Default for SurfaceFilters {
    fn default() -> Self {
        Self {
            min_bid: 0.0,
            max_spread: f64::INFINITY,
            min_time_to_expiry_secs: 0.0,
            min_volume: 0.0,
            max_age_secs: f64::INFINITY,
        }
    }
}

/// The declared inputs and limits for calibrating a [`VolatilitySurface`].
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SurfaceConfig {
    /// The observation filters.
    pub filters: SurfaceFilters,
    /// Which price the total variance is computed from.
    pub price_source: SurfacePriceSource,
    /// How the per-slice forward is obtained.
    pub forward_source: ForwardSource,
    /// The continuously compounded risk-free rate used by the implied-vol solver and by parity.
    pub risk_free_rate: f64,
    /// The reference timestamp used for time-to-expiry and staleness checks.
    pub reference_ns: UnixNanos,
    /// The fallback family selected for a slice the interpolation family cannot make satisfy the
    /// no-arbitrage conditions.
    pub slice_fallback: SliceFallback,
    /// The recalibration trigger.
    pub trigger: CalibrationTrigger,
}

impl Default for SurfaceConfig {
    fn default() -> Self {
        Self {
            filters: SurfaceFilters::default(),
            price_source: SurfacePriceSource::Mid,
            forward_source: ForwardSource::PutCallParity,
            risk_free_rate: 0.0,
            reference_ns: UnixNanos::default(),
            slice_fallback: SliceFallback::Svi,
            trigger: CalibrationTrigger::OnDemand,
        }
    }
}

/// A specific way an arbitrage check failed.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArbitrageViolation {
    /// A total variance was negative.
    NegativeTotalVariance {
        /// The log moneyness where the violation was found.
        log_moneyness: f64,
        /// The offending total variance.
        total_variance: f64,
    },
    /// The implied density (Durrleman's `g`) was negative - a butterfly arbitrage.
    Butterfly {
        /// The log moneyness where the violation was found.
        log_moneyness: f64,
        /// The offending implied density value.
        density: f64,
    },
    /// Total variance decreased with time to expiry - a calendar arbitrage.
    Calendar {
        /// The log moneyness where the violation was found.
        log_moneyness: f64,
        /// The earlier total variance.
        earlier_variance: f64,
        /// The later total variance that is smaller.
        later_variance: f64,
    },
}

/// An error returned while calibrating a volatility surface.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum SurfaceError {
    /// No observation survived the filters.
    #[error("no usable observations survived the filters (rejected {rejected})")]
    NoUsableObservations {
        /// The number of rejected observations.
        rejected: usize,
    },
    /// Fewer expiry slices than required to make a calendar statement.
    #[error("requires at least {required} expiry slices, found {found}")]
    TooFewSlices {
        /// The number of slices found.
        found: usize,
        /// The number of slices required.
        required: usize,
    },
    /// Fewer usable observations than required in an expiry slice.
    #[error("expiry {expiry_ns} requires at least {required} usable observations, found {found}")]
    TooFewObservations {
        /// The expiry of the slice.
        expiry_ns: UnixNanos,
        /// The number of usable observations found.
        found: usize,
        /// The number of usable observations required.
        required: usize,
    },
    /// A forward could not be derived for an expiry slice.
    #[error("no forward could be derived for expiry {expiry_ns}")]
    ForwardUnavailable {
        /// The expiry of the slice.
        expiry_ns: UnixNanos,
    },
    /// A configuration parameter was invalid.
    #[error("invalid surface parameter `{name}`: {value}")]
    InvalidParameter {
        /// The name of the invalid parameter.
        name: &'static str,
        /// The invalid value.
        value: f64,
    },
    /// The surface failed a no-arbitrage condition and the fallback was refused or not applied.
    #[error("surface violates a no-arbitrage condition: {violation:?}")]
    ArbitrageViolation {
        /// The violation that was found.
        violation: ArbitrageViolation,
    },
    /// The SVI fallback was selected for a slice but could not be calibrated to satisfy its own
    /// no-arbitrage conditions.
    #[error("SVI fallback could not be calibrated for expiry {expiry_ns}")]
    SviCalibrationFailed {
        /// The expiry of the slice.
        expiry_ns: UnixNanos,
    },
}

/// The representation selected for an expiry slice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceScheme {
    /// Piecewise-linear total variance against log moneyness (the interpolation family).
    LinearTotalVariance,
    /// Raw SVI total variance (the arbitrage-free fallback family).
    Svi,
}

/// A single node of a slice: total variance at a log moneyness.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct VarianceNode {
    /// The log moneyness `ln(K / F)`.
    pub log_moneyness: f64,
    /// The total implied variance `sigma^2 * T`.
    pub total_variance: f64,
}

/// Raw SVI total variance parameters `w(k) = a + b * (rho * (k - m) + sqrt((k - m)^2 + sigma^2))`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SviParams {
    /// The level parameter.
    pub a: f64,
    /// The slope parameter (must be non-negative for convexity).
    pub b: f64,
    /// The correlation parameter (in `(-1, 1)`).
    pub rho: f64,
    /// The shift parameter.
    pub m: f64,
    /// The curvature parameter (strictly positive).
    pub sigma: f64,
}

impl SviParams {
    /// Returns the total variance at the given log moneyness.
    #[must_use]
    pub fn total_variance(&self, log_moneyness: f64) -> f64 {
        let d = log_moneyness - self.m;
        self.a + self.b * (self.rho * d + (d * d + self.sigma * self.sigma).sqrt())
    }

    /// Returns the first derivative of the total variance with respect to log moneyness.
    #[must_use]
    pub fn total_variance_prime(&self, log_moneyness: f64) -> f64 {
        let d = log_moneyness - self.m;
        let s = (d * d + self.sigma * self.sigma).sqrt();
        self.b * (self.rho + d / s)
    }

    /// Returns the second derivative of the total variance with respect to log moneyness.
    #[must_use]
    pub fn total_variance_double_prime(&self, log_moneyness: f64) -> f64 {
        let d = log_moneyness - self.m;
        let s = (d * d + self.sigma * self.sigma).sqrt();
        self.b * self.sigma * self.sigma / (s * s * s)
    }

    /// Returns whether the parameters satisfy the family's parameter constraints.
    ///
    /// `b >= 0` gives `w'' >= 0` (convexity), and `a + b * sigma * sqrt(1 - rho^2) >= 0` is exactly
    /// the condition `min_k w(k) >= 0`, since the SVI minimum over all `k` is
    /// `a + b * sigma * sqrt(1 - rho^2)`.
    #[must_use]
    pub fn satisfies_constraints(&self) -> bool {
        self.a.is_finite()
            && self.b.is_finite()
            && self.rho.is_finite()
            && self.m.is_finite()
            && self.sigma.is_finite()
            && self.b >= 0.0
            && self.sigma > 0.0
            && self.rho.abs() < 1.0
            && self.a + self.b * self.sigma * (1.0 - self.rho * self.rho).sqrt()
                >= -ARBITRAGE_TOLERANCE
    }

    /// Returns whether the slice is free of butterfly arbitrage on `[k_min, k_max]`.
    ///
    /// The parameter constraints already imply `w >= 0` and `w'' >= 0`; this additionally samples
    /// Durrleman's implied-density condition `g(k) >= 0`.
    #[must_use]
    pub fn is_arbitrage_free(&self, k_min: f64, k_max: f64) -> bool {
        if !self.satisfies_constraints() {
            return false;
        }
        if k_max <= k_min {
            return self.total_variance(k_min) >= -ARBITRAGE_TOLERANCE;
        }
        for i in 0..=RANGE_SAMPLES {
            let frac = i as f64 / RANGE_SAMPLES as f64;
            let k = k_min + frac * (k_max - k_min);
            let w = self.total_variance(k);
            let g = durrleman_g(
                k,
                w,
                self.total_variance_prime(k),
                self.total_variance_double_prime(k),
            );
            if w < -ARBITRAGE_TOLERANCE || g < -ARBITRAGE_TOLERANCE {
                return false;
            }
        }
        true
    }

    /// Fits the family to `nodes` by a bounded deterministic search.
    ///
    /// For each `(rho, m, sigma)` on a fixed grid the two linear parameters `(a, b)` are solved in
    /// closed form by constrained least squares; the candidate with the smallest residual sum of
    /// squares that also satisfies [`SviParams::is_arbitrage_free`] over the node range is returned
    /// together with the root-mean-square residual. Returns `None` when no grid point yields an
    /// arbitrage-free fit.
    #[must_use]
    pub fn fit(nodes: &[VarianceNode]) -> Option<(Self, f64)> {
        const RHO_STEPS: usize = 15;
        const M_STEPS: usize = 9;
        const SIGMAS: [f64; 7] = [0.01, 0.03, 0.06, 0.12, 0.25, 0.5, 1.0];

        let first = nodes.first()?;
        let last = nodes.last()?;
        let k_min = first.log_moneyness;
        let k_max = last.log_moneyness;
        let range = (k_max - k_min).max(0.05);

        let mut best: Option<(Self, f64)> = None;
        for ri in 0..RHO_STEPS {
            let rho = -0.98 + 1.96 * ri as f64 / (RHO_STEPS as f64 - 1.0);
            for mi in 0..M_STEPS {
                let m = k_min - 0.5 * range + 2.0 * range * mi as f64 / (M_STEPS as f64 - 1.0);
                for sigma in SIGMAS {
                    let (a, b) = solve_svi_ab(nodes, rho, m, sigma);
                    let candidate = Self {
                        a,
                        b,
                        rho,
                        m,
                        sigma,
                    };
                    if !candidate.is_arbitrage_free(k_min, k_max) {
                        continue;
                    }
                    let sse = svi_sse(nodes, &candidate);
                    if best.is_some_and(|(_, best_sse)| sse >= best_sse) {
                        continue;
                    }
                    best = Some((candidate, sse));
                }
            }
        }

        let (params, sse) = best?;
        let rmse = (sse / nodes.len() as f64).sqrt();
        Some((params, rmse))
    }
}

/// Solves the two linear SVI parameters `(a, b)` for fixed `(rho, m, sigma)` by least squares,
/// then projects onto the family's non-negativity constraint.
fn solve_svi_ab(nodes: &[VarianceNode], rho: f64, m: f64, sigma: f64) -> (f64, f64) {
    let n = nodes.len() as f64;
    let (mut sx, mut sy, mut sxx, mut sxy) = (0.0, 0.0, 0.0, 0.0);
    for node in nodes {
        let d = node.log_moneyness - m;
        let x = rho * d + (d * d + sigma * sigma).sqrt();
        sx += x;
        sy += node.total_variance;
        sxx += x * x;
        sxy += x * node.total_variance;
    }

    let denom = n * sxx - sx * sx;
    let mut b = if denom.abs() > 1e-12 {
        (n * sxy - sx * sy) / denom
    } else {
        0.0
    };
    if b < 0.0 {
        b = 0.0;
    }
    let mut a = if b > 0.0 { (sy - b * sx) / n } else { sy / n };

    // Enforce a + b * sigma * sqrt(1 - rho^2) >= 0 (the global minimum of w).
    let floor = -b * sigma * (1.0 - rho * rho).sqrt();
    if a < floor {
        a = floor;
    }
    (a, b)
}

fn svi_sse(nodes: &[VarianceNode], params: &SviParams) -> f64 {
    nodes
        .iter()
        .map(|node| {
            let residual = params.total_variance(node.log_moneyness) - node.total_variance;
            residual * residual
        })
        .sum()
}

/// Durrleman's implied-density condition.
///
/// The risk-neutral density is proportional to `g(k) / sqrt(2 * pi * w(k))`; `g(k) >= 0` is the
/// no-butterfly condition. `wp` and `wpp` are the first and second derivatives of `w` in `k`.
fn durrleman_g(k: f64, w: f64, wp: f64, wpp: f64) -> f64 {
    if w <= 0.0 {
        return f64::NEG_INFINITY;
    }
    let term = 1.0 - k * wp / (2.0 * w);
    term * term - wp * wp / 4.0 * (1.0 / w + 0.25) + wpp / 2.0
}

/// A calibrated slice of the surface at a single expiry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VolatilitySlice {
    /// The time to expiry in years.
    pub time_to_expiry: f64,
    /// The forward used for the slice.
    pub forward: f64,
    /// The representation selected for the slice.
    pub scheme: SurfaceScheme,
    /// The observed nodes, sorted by log moneyness.
    pub nodes: Vec<VarianceNode>,
    /// The fitted SVI parameters, when the SVI fallback was selected.
    pub svi: Option<SviParams>,
    /// The root-mean-square residual of the fit at the observed nodes.
    pub fit_error: f64,
    /// The number of observations contributing to the slice.
    pub observation_count: usize,
}

impl VolatilitySlice {
    /// Returns the quoted log-moneyness range as `(min, max)`.
    #[must_use]
    pub fn log_moneyness_range(&self) -> (f64, f64) {
        match (self.nodes.first(), self.nodes.last()) {
            (Some(first), Some(last)) => (first.log_moneyness, last.log_moneyness),
            _ => (0.0, 0.0),
        }
    }

    /// Returns the total variance at `log_moneyness`, held flat beyond the quoted range.
    ///
    /// Returns `(total_variance, extrapolated)`.
    #[must_use]
    pub fn total_variance(&self, log_moneyness: f64) -> (f64, bool) {
        let (Some(first), Some(last)) = (self.nodes.first(), self.nodes.last()) else {
            return (0.0, false);
        };
        if self.nodes.len() == 1 {
            return (first.total_variance, log_moneyness != first.log_moneyness);
        }
        if log_moneyness <= first.log_moneyness {
            return (first.total_variance, log_moneyness < first.log_moneyness);
        }
        if log_moneyness >= last.log_moneyness {
            return (last.total_variance, log_moneyness > last.log_moneyness);
        }
        let value = match self.scheme {
            SurfaceScheme::LinearTotalVariance => linear_total_variance(&self.nodes, log_moneyness),
            SurfaceScheme::Svi => match self.svi {
                Some(params) => params.total_variance(log_moneyness),
                None => linear_total_variance(&self.nodes, log_moneyness),
            },
        };
        (value, false)
    }
}

/// Piecewise-linear total variance against log moneyness, with the node bounds assumed to contain
/// `log_moneyness`.
fn linear_total_variance(nodes: &[VarianceNode], log_moneyness: f64) -> f64 {
    let Some(idx) = nodes
        .windows(2)
        .position(|pair| log_moneyness <= pair[1].log_moneyness)
    else {
        return nodes.last().map_or(0.0, |n| n.total_variance);
    };
    let left = nodes[idx];
    let right = nodes[idx + 1];
    let dk = right.log_moneyness - left.log_moneyness;
    if dk <= 0.0 {
        return left.total_variance;
    }
    let frac = (log_moneyness - left.log_moneyness) / dk;
    left.total_variance + frac * (right.total_variance - left.total_variance)
}

/// The result of querying a volatility surface.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SurfaceQuery {
    /// The queried log moneyness.
    pub log_moneyness: f64,
    /// The queried time to expiry, in years.
    pub time_to_expiry: f64,
    /// The total implied variance.
    pub total_variance: f64,
    /// The implied volatility `sqrt(w / T)`.
    pub implied_volatility: f64,
    /// The number of observations contributing to the query.
    pub observation_count: usize,
    /// The root-mean-square fit residual of the contributing slices.
    pub fit_error: f64,
    /// Whether the query fell outside the quoted moneyness or expiry region.
    pub extrapolated: bool,
}

/// A calibrated implied volatility surface.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VolatilitySurface {
    slices: Vec<VolatilitySlice>,
    rejection_counts: RejectionCounts,
    fit_error: f64,
    config: SurfaceConfig,
}

impl VolatilitySurface {
    /// Calibrates a surface from `observations` under `config`.
    ///
    /// Observations pass through the named filters, are grouped by expiry, and each slice is
    /// fitted. The surface is refused - never repaired - if a slice cannot be made arbitrage-free
    /// or if the calendar condition fails.
    ///
    /// # Errors
    ///
    /// Returns a [`SurfaceError`] if the configuration is invalid, if there are too few slices or
    /// usable observations, if a forward cannot be derived, or if a no-arbitrage condition fails.
    pub fn calibrate(
        observations: &[SurfaceObservation],
        config: SurfaceConfig,
    ) -> Result<Self, SurfaceError> {
        validate_config(&config)?;

        let mut rejection_counts = RejectionCounts::default();
        let mut groups: BTreeMap<u64, Vec<&SurfaceObservation>> = BTreeMap::new();
        for observation in observations {
            if let Err(reason) = classify(observation, &config) {
                rejection_counts.record(reason);
                continue;
            }
            rejection_counts.accepted += 1;
            groups
                .entry(observation.expiry_ns.as_u64())
                .or_default()
                .push(observation);
        }

        if rejection_counts.accepted == 0 {
            return Err(SurfaceError::NoUsableObservations {
                rejected: rejection_counts.rejected(),
            });
        }

        if groups.len() < MIN_SLICES_FOR_CALENDAR {
            return Err(SurfaceError::TooFewSlices {
                found: groups.len(),
                required: MIN_SLICES_FOR_CALENDAR,
            });
        }

        let mut slices = Vec::with_capacity(groups.len());
        for (expiry_raw, group) in &groups {
            let (slice, additional) = build_slice(*expiry_raw, group, &config)?;
            for reason in additional {
                rejection_counts.record(reason);
            }
            slices.push(slice);
        }

        check_calendar(&slices)?;

        let fit_error = pooled_fit_error(&slices);
        Ok(Self {
            slices,
            rejection_counts,
            fit_error,
            config,
        })
    }

    /// Returns the declared inputs and limits used to calibrate the surface.
    #[must_use]
    pub fn config(&self) -> &SurfaceConfig {
        &self.config
    }

    /// Returns the calibrated slices, ordered by time to expiry.
    #[must_use]
    pub fn slices(&self) -> &[VolatilitySlice] {
        &self.slices
    }

    /// Returns the per-reason rejection counts for the calibration.
    #[must_use]
    pub fn rejection_counts(&self) -> &RejectionCounts {
        &self.rejection_counts
    }

    /// Returns the pooled root-mean-square fit residual of the surface.
    #[must_use]
    pub fn fit_error(&self) -> f64 {
        self.fit_error
    }

    /// Returns the total number of observations contributing to the surface.
    #[must_use]
    pub fn observation_count(&self) -> usize {
        self.slices.iter().map(|s| s.observation_count).sum()
    }

    /// Returns the quoted time-to-expiry range as `(min, max)`.
    #[must_use]
    pub fn time_to_expiry_range(&self) -> (f64, f64) {
        match (self.slices.first(), self.slices.last()) {
            (Some(first), Some(last)) => (first.time_to_expiry, last.time_to_expiry),
            _ => (0.0, 0.0),
        }
    }

    /// Queries the surface for total variance and implied volatility.
    ///
    /// Total variance is interpolated between bracketing slices and held flat beyond the quoted
    /// moneyness or expiry ranges; `extrapolated` is set when either occurs.
    #[must_use]
    pub fn query(&self, log_moneyness: f64, time_to_expiry: f64) -> SurfaceQuery {
        let Some(first) = self.slices.first() else {
            return SurfaceQuery {
                log_moneyness,
                time_to_expiry,
                total_variance: 0.0,
                implied_volatility: 0.0,
                observation_count: 0,
                fit_error: 0.0,
                extrapolated: false,
            };
        };
        let last = match self.slices.last() {
            Some(last) => last,
            None => first,
        };

        let mut extrapolated = false;
        let t = if time_to_expiry < first.time_to_expiry {
            extrapolated = true;
            first.time_to_expiry
        } else if time_to_expiry > last.time_to_expiry {
            extrapolated = true;
            last.time_to_expiry
        } else {
            time_to_expiry
        };

        let idx = self
            .slices
            .partition_point(|slice| slice.time_to_expiry <= t);
        let (lo, hi, weight) = if idx == 0 {
            (0, 0, 0.0)
        } else if idx >= self.slices.len() {
            let last_idx = self.slices.len() - 1;
            (last_idx, last_idx, 0.0)
        } else {
            let span = self.slices[idx].time_to_expiry - self.slices[idx - 1].time_to_expiry;
            let weight = if span > 0.0 {
                (t - self.slices[idx - 1].time_to_expiry) / span
            } else {
                0.0
            };
            (idx - 1, idx, weight)
        };

        let (w_lo, extrapolated_lo) = self.slices[lo].total_variance(log_moneyness);
        extrapolated |= extrapolated_lo;
        // A zero weight means the later slice contributes nothing, so it is neither counted nor
        // used to raise the extrapolation flag.
        let (total_variance, observation_count, fit_error) = if lo == hi || weight <= 0.0 {
            (
                w_lo,
                self.slices[lo].observation_count,
                self.slices[lo].fit_error,
            )
        } else {
            let (w_hi, extrapolated_hi) = self.slices[hi].total_variance(log_moneyness);
            extrapolated |= extrapolated_hi;
            let lo_obs = self.slices[lo].observation_count;
            let hi_obs = self.slices[hi].observation_count;
            let count = lo_obs + hi_obs;
            let fit_error = if count > 0 {
                (self.slices[lo].fit_error * lo_obs as f64
                    + self.slices[hi].fit_error * hi_obs as f64)
                    / count as f64
            } else {
                0.0
            };
            (w_lo + weight * (w_hi - w_lo), count, fit_error)
        };

        let implied_volatility = if t > 0.0 {
            (total_variance / t).sqrt()
        } else {
            0.0
        };

        SurfaceQuery {
            log_moneyness,
            time_to_expiry: t,
            total_variance,
            implied_volatility,
            observation_count,
            fit_error,
            extrapolated,
        }
    }
}

fn validate_config(config: &SurfaceConfig) -> Result<(), SurfaceError> {
    for (name, value) in [
        ("risk_free_rate", config.risk_free_rate),
        ("min_bid", config.filters.min_bid),
        ("max_spread", config.filters.max_spread),
        (
            "min_time_to_expiry_secs",
            config.filters.min_time_to_expiry_secs,
        ),
        ("min_volume", config.filters.min_volume),
        ("max_age_secs", config.filters.max_age_secs),
    ] {
        if value.is_nan() {
            return Err(SurfaceError::InvalidParameter { name, value });
        }
        if value < 0.0 {
            return Err(SurfaceError::InvalidParameter { name, value });
        }
    }
    if let ForwardSource::Supplied(forward) = config.forward_source
        && (!forward.is_finite() || forward <= 0.0)
    {
        return Err(SurfaceError::InvalidParameter {
            name: "forward",
            value: forward,
        });
    }
    Ok(())
}

/// Returns `(later - earlier)` in nanoseconds as an `f64`.
///
/// The subtraction is done in `i128` before the cast so that large absolute timestamps (which
/// exceed the `f64` integer range) cannot lose the sub-second difference to rounding.
fn nanos_diff(later: UnixNanos, earlier: UnixNanos) -> f64 {
    (i128::from(later.as_u64()) - i128::from(earlier.as_u64())) as f64
}

fn time_to_expiry_years(expiry_ns: UnixNanos, reference_ns: UnixNanos) -> f64 {
    nanos_diff(expiry_ns, reference_ns) / NANOSECONDS_PER_YEAR
}

fn classify(
    observation: &SurfaceObservation,
    config: &SurfaceConfig,
) -> Result<(), SurfaceRejection> {
    if !observation.bid.is_finite()
        || !observation.ask.is_finite()
        || !observation.strike.is_finite()
        || observation.strike <= 0.0
    {
        return Err(SurfaceRejection::InvalidQuote);
    }
    if observation.bid > observation.ask {
        return Err(SurfaceRejection::CrossedMarket);
    }
    if observation.bid < config.filters.min_bid {
        return Err(SurfaceRejection::BidBelowMinimum);
    }
    if observation.ask - observation.bid > config.filters.max_spread {
        return Err(SurfaceRejection::SpreadAboveMaximum);
    }
    let t_secs =
        time_to_expiry_years(observation.expiry_ns, config.reference_ns) * SECONDS_PER_YEAR;
    if t_secs <= 0.0 || t_secs < config.filters.min_time_to_expiry_secs {
        return Err(SurfaceRejection::TimeToExpiryBelowMinimum);
    }
    if let Some(volume) = observation.volume
        && volume < config.filters.min_volume
    {
        return Err(SurfaceRejection::VolumeBelowMinimum);
    }
    let age_secs = nanos_diff(config.reference_ns, observation.ts_event) / NANOSECONDS_PER_SECOND;
    if age_secs > config.filters.max_age_secs {
        return Err(SurfaceRejection::StaleObservation);
    }
    Ok(())
}

/// Builds a slice from the accepted observations at one expiry.
///
/// Returns the slice plus any observations that passed the quote filters but could not be turned
/// into a node because the implied-vol solver failed.
fn build_slice(
    expiry_raw: u64,
    group: &[&SurfaceObservation],
    config: &SurfaceConfig,
) -> Result<(VolatilitySlice, Vec<SurfaceRejection>), SurfaceError> {
    let expiry_ns = UnixNanos::from(expiry_raw);
    let time_to_expiry = time_to_expiry_years(expiry_ns, config.reference_ns);
    let forward = resolve_forward(group, config, time_to_expiry)
        .ok_or(SurfaceError::ForwardUnavailable { expiry_ns })?;
    if !forward.is_finite() || forward <= 0.0 {
        return Err(SurfaceError::InvalidParameter {
            name: "forward",
            value: forward,
        });
    }

    let mut rejections = Vec::new();
    let mut points: Vec<(f64, f64)> = Vec::with_capacity(group.len());
    for observation in group {
        if let Some(variance) = observation_variance(observation, forward, config, time_to_expiry) {
            let log_moneyness = (observation.strike / forward).ln();
            points.push((log_moneyness, variance));
        } else {
            rejections.push(SurfaceRejection::InvalidImpliedVolatility);
        }
    }

    points.sort_by(|a, b| a.0.total_cmp(&b.0));

    let mut merged: Vec<(f64, f64, usize)> = Vec::with_capacity(points.len());
    for (log_moneyness, variance) in points {
        match merged.last_mut() {
            Some(last) if (log_moneyness - last.0).abs() <= LOG_MONEYNESS_TOLERANCE => {
                last.1 += variance;
                last.2 += 1;
            }
            _ => merged.push((log_moneyness, variance, 1)),
        }
    }

    let nodes: Vec<VarianceNode> = merged
        .iter()
        .map(|(log_moneyness, sum, count)| VarianceNode {
            log_moneyness: *log_moneyness,
            total_variance: sum / *count as f64,
        })
        .collect();

    if nodes.len() < MIN_OBSERVATIONS_PER_SLICE {
        return Err(SurfaceError::TooFewObservations {
            expiry_ns,
            found: nodes.len(),
            required: MIN_OBSERVATIONS_PER_SLICE,
        });
    }

    let observation_count = merged.iter().map(|(_, _, count)| count).sum();

    let (scheme, svi, fit_error) = match linear_slice_violation(&nodes) {
        None => (SurfaceScheme::LinearTotalVariance, None, 0.0),
        Some(violation) => match config.slice_fallback {
            SliceFallback::Refuse => {
                return Err(SurfaceError::ArbitrageViolation { violation });
            }
            SliceFallback::Svi => match SviParams::fit(&nodes) {
                Some((params, rmse)) => (SurfaceScheme::Svi, Some(params), rmse),
                None => return Err(SurfaceError::SviCalibrationFailed { expiry_ns }),
            },
        },
    };

    Ok((
        VolatilitySlice {
            time_to_expiry,
            forward,
            scheme,
            nodes,
            svi,
            fit_error,
            observation_count,
        },
        rejections,
    ))
}

/// Checks a piecewise-linear slice for no-arbitrage violations.
///
/// The segment slopes must be non-decreasing (convexity), which makes the point masses at the
/// nodes non-negative, and Durrleman's condition must hold on every segment interior.
fn linear_slice_violation(nodes: &[VarianceNode]) -> Option<ArbitrageViolation> {
    for node in nodes {
        if node.total_variance < -ARBITRAGE_TOLERANCE {
            return Some(ArbitrageViolation::NegativeTotalVariance {
                log_moneyness: node.log_moneyness,
                total_variance: node.total_variance,
            });
        }
    }

    let mut previous_slope = f64::NEG_INFINITY;
    for pair in nodes.windows(2) {
        let left = pair[0];
        let right = pair[1];
        let dk = right.log_moneyness - left.log_moneyness;
        if dk <= 0.0 {
            continue;
        }
        let slope = (right.total_variance - left.total_variance) / dk;
        if slope < previous_slope - ARBITRAGE_TOLERANCE {
            return Some(ArbitrageViolation::Butterfly {
                log_moneyness: right.log_moneyness,
                density: slope - previous_slope,
            });
        }
        previous_slope = slope;
        for i in 0..=SEGMENT_SAMPLES {
            let frac = i as f64 / SEGMENT_SAMPLES as f64;
            let k = left.log_moneyness + frac * dk;
            let w = left.total_variance + frac * (right.total_variance - left.total_variance);
            let g = durrleman_g(k, w, slope, 0.0);
            if g < -ARBITRAGE_TOLERANCE {
                return Some(ArbitrageViolation::Butterfly {
                    log_moneyness: k,
                    density: g,
                });
            }
        }
    }
    None
}

fn resolve_forward(
    group: &[&SurfaceObservation],
    config: &SurfaceConfig,
    time_to_expiry: f64,
) -> Option<f64> {
    match config.forward_source {
        ForwardSource::Supplied(forward) => Some(forward),
        ForwardSource::PutCallParity => {
            let mut forwards = Vec::new();
            for call in group
                .iter()
                .filter(|o| matches!(o.option_kind, OptionKind::Call))
            {
                let Some(put) = group.iter().find(|o| {
                    matches!(o.option_kind, OptionKind::Put)
                        && (o.strike - call.strike).abs() <= LOG_MONEYNESS_TOLERANCE
                }) else {
                    continue;
                };
                let call_mid = f64::midpoint(call.bid, call.ask);
                let put_mid = f64::midpoint(put.bid, put.ask);
                let forward = implied_forward_from_parity(
                    call_mid,
                    put_mid,
                    call.strike,
                    config.risk_free_rate,
                    time_to_expiry,
                );
                if forward.is_finite() && forward > 0.0 {
                    forwards.push(forward);
                }
            }
            if forwards.is_empty() {
                return None;
            }
            forwards.sort_by(f64::total_cmp);
            Some(forwards[forwards.len() / 2])
        }
    }
}

fn observation_variance(
    observation: &SurfaceObservation,
    forward: f64,
    config: &SurfaceConfig,
    time_to_expiry: f64,
) -> Option<f64> {
    let is_call = matches!(observation.option_kind, OptionKind::Call);
    let total_variance = |price: f64| {
        let vol = imply_vol(
            forward,
            config.risk_free_rate,
            0.0,
            is_call,
            observation.strike,
            time_to_expiry,
            price,
        );
        if vol.is_finite() && vol > 0.0 {
            Some(vol * vol * time_to_expiry)
        } else {
            None
        }
    };

    match config.price_source {
        SurfacePriceSource::Mid => total_variance(f64::midpoint(observation.bid, observation.ask)),
        SurfacePriceSource::BidAsk => {
            let from_bid = total_variance(observation.bid)?;
            let from_ask = total_variance(observation.ask)?;
            Some(f64::midpoint(from_bid, from_ask))
        }
    }
}

fn check_calendar(slices: &[VolatilitySlice]) -> Result<(), SurfaceError> {
    for pair in slices.windows(2) {
        let earlier = &pair[0];
        let later = &pair[1];
        let (earlier_min, earlier_max) = earlier.log_moneyness_range();
        let (later_min, later_max) = later.log_moneyness_range();
        let k_min = earlier_min.min(later_min);
        let k_max = earlier_max.max(later_max);
        if k_max <= k_min {
            let (we, _) = earlier.total_variance(k_min);
            let (wl, _) = later.total_variance(k_min);
            if wl < we - ARBITRAGE_TOLERANCE {
                return Err(SurfaceError::ArbitrageViolation {
                    violation: ArbitrageViolation::Calendar {
                        log_moneyness: k_min,
                        earlier_variance: we,
                        later_variance: wl,
                    },
                });
            }
            continue;
        }
        for i in 0..=RANGE_SAMPLES {
            let frac = i as f64 / RANGE_SAMPLES as f64;
            let k = k_min + frac * (k_max - k_min);
            let (we, _) = earlier.total_variance(k);
            let (wl, _) = later.total_variance(k);
            if wl < we - ARBITRAGE_TOLERANCE {
                return Err(SurfaceError::ArbitrageViolation {
                    violation: ArbitrageViolation::Calendar {
                        log_moneyness: k,
                        earlier_variance: we,
                        later_variance: wl,
                    },
                });
            }
        }
    }
    Ok(())
}

fn pooled_fit_error(slices: &[VolatilitySlice]) -> f64 {
    let total_obs: usize = slices.iter().map(|s| s.observation_count).sum();
    if total_obs == 0 {
        return 0.0;
    }
    slices
        .iter()
        .map(|s| s.fit_error * s.observation_count as f64)
        .sum::<f64>()
        / total_obs as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calibration_trigger_default_is_on_demand() {
        assert_eq!(CalibrationTrigger::default(), CalibrationTrigger::OnDemand);
        assert!(!CalibrationTrigger::OnDemand.is_due(1e9, 1_000_000));
    }

    #[test]
    fn test_calibration_trigger_interval_and_observations() {
        let interval = CalibrationTrigger::Interval { seconds: 60.0 };
        assert!(!interval.is_due(59.9, 0));
        assert!(interval.is_due(60.0, 0));
        let observations = CalibrationTrigger::OnNewObservations { count: 5 };
        assert!(!observations.is_due(0.0, 4));
        assert!(observations.is_due(0.0, 5));
    }

    #[test]
    fn test_rejection_counts_accessors() {
        let mut counts = RejectionCounts {
            accepted: 2,
            ..Default::default()
        };
        counts.record(SurfaceRejection::CrossedMarket);
        counts.record(SurfaceRejection::CrossedMarket);
        counts.record(SurfaceRejection::StaleObservation);
        assert_eq!(counts.count(SurfaceRejection::CrossedMarket), 2);
        assert_eq!(counts.count(SurfaceRejection::BidBelowMinimum), 0);
        assert_eq!(counts.rejected(), 3);
        assert_eq!(counts.total(), 5);
    }

    #[test]
    fn test_linear_total_variance_interpolates_nodes() {
        let nodes = vec![
            VarianceNode {
                log_moneyness: -0.2,
                total_variance: 0.04,
            },
            VarianceNode {
                log_moneyness: 0.0,
                total_variance: 0.06,
            },
            VarianceNode {
                log_moneyness: 0.2,
                total_variance: 0.08,
            },
        ];
        assert_eq!(linear_total_variance(&nodes, -0.2), 0.04);
        assert_eq!(linear_total_variance(&nodes, 0.2), 0.08);
        assert!((linear_total_variance(&nodes, -0.1) - 0.05).abs() < 1e-12);
    }
}
