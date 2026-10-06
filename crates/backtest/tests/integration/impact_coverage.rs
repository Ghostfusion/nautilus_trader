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

//! Acceptance for the coverage of the impact interval (`design D4`, work item I2.3).
//!
//! The synthetic flow generates paths at a known prefactor, the calibration fits its interval on
//! each, and the coverage check reports the fraction of repetitions whose fitted interval contained
//! that prefactor. The acceptance pins three observable facts: a well-calibrated interval reads at
//! or above its nominal level, a deliberately over-tight interval reads below it, and the same seed
//! produces the same coverage.

use anyhow::Result;
use nautilus_backtest::synthetic::FlowImpactCoverage;
use nautilus_execution::models::market_impact::PrefactorInterval;
use nautilus_execution::models::market_impact_calibration::{
    ImpactObservation, PREFACTOR_COVERAGE_NOMINAL, PrefactorCoverageCheck, PrefactorCoverageModel,
    prefactor_coverage,
};

/// The known prefactor the paths are generated at.
const KNOWN_PREFACTOR: f64 = 2.0;

/// The nominal coverage level both intervals are checked against.
const NOMINAL: f64 = PREFACTOR_COVERAGE_NOMINAL;

/// The number of seeded repetitions.
const REPETITIONS: usize = 32;

/// The base seed; the same seed must give the same coverage.
const SEED: u64 = 0x5EED;

/// Returns the coverage model the acceptance measures.
fn model() -> FlowImpactCoverage {
    FlowImpactCoverage::new(0.7, 0.5, 1_024, 1).expect("the model parameters are valid")
}

/// Returns the check the acceptance runs against.
fn check() -> PrefactorCoverageCheck {
    PrefactorCoverageCheck::new(KNOWN_PREFACTOR, NOMINAL, REPETITIONS, SEED)
        .expect("the check parameters are valid")
}

/// The calibration's own span, narrowed to a point at its midpoint: a deliberately over-tight
/// interval that the data does not support.
struct OverTightFlow {
    inner: FlowImpactCoverage,
}

impl PrefactorCoverageModel for OverTightFlow {
    fn draw(&self, truth: f64, seed: u64) -> Result<Vec<ImpactObservation>> {
        self.inner.draw(truth, seed)
    }

    fn fit(&self, observations: &[ImpactObservation]) -> Result<PrefactorInterval> {
        let span = self.inner.fit(observations)?;
        let midpoint = f64::midpoint(span.lower(), span.upper());
        PrefactorInterval::new(midpoint, midpoint, span.source())
    }
}

#[test]
fn test_a_well_calibrated_interval_reads_at_or_above_its_nominal_level() {
    let report = prefactor_coverage(&model(), &check()).expect("coverage measured");

    assert!(
        report.holds(),
        "a well-calibrated interval must read at or above its nominal level: {report}"
    );
    assert!(report.coverage >= NOMINAL, "{report}");
    assert_eq!(report.repetitions, REPETITIONS);
    assert_eq!(report.nominal, NOMINAL);
}

#[test]
fn test_a_deliberately_over_tight_interval_reads_below_its_nominal_level() {
    let over_tight = OverTightFlow { inner: model() };
    let report = prefactor_coverage(&over_tight, &check()).expect("coverage measured");

    assert!(
        !report.holds(),
        "an over-tight interval must read below its nominal level rather than being excused: {report}"
    );
    assert!(report.coverage < NOMINAL, "{report}");
}

#[test]
fn test_the_same_seed_produces_the_same_coverage() {
    let first = prefactor_coverage(&model(), &check()).expect("coverage measured");
    let second = prefactor_coverage(&model(), &check()).expect("coverage measured");

    assert_eq!(first, second);
    assert_eq!(first.to_string(), second.to_string());
}
