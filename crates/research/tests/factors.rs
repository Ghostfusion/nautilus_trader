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

//! Numeric pinning of a small factor set over a hand-computed fixture, and the stability of a
//! factor definition's digest.
//!
//! Every expected value below is derived by hand; the arithmetic is in the comment above each
//! assertion. The fixture is deliberately exact where a binary fraction exists and carries its
//! exact fraction otherwise.

use nautilus_model::identifiers::InstrumentId;
use nautilus_research::{Digest, ExprSource, Feature, TransformSide};

/// The pinned digest of the "momentum" definition compiled by [`pinned`]. It detects any change to
/// the canonical serialization.
const PINNED_DIGEST: &str = "efbcf90a9df8567da04229361a3e5733d829b89effcde1896e2a370c768c3efe";

fn instrument(value: &str) -> InstrumentId {
    InstrumentId::from(value)
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-12,
        "expected {expected}, was {actual}"
    );
}

struct Fixture {
    universe: Vec<InstrumentId>,
    aaa_close: Vec<f64>,
    bbb_close: Vec<f64>,
    sector: Vec<f64>,
}

impl Fixture {
    fn new() -> Self {
        Self {
            universe: vec![instrument("AAA.X"), instrument("BBB.X")],
            aaa_close: vec![1.0, 2.0, 4.0, 8.0, 16.0],
            bbb_close: vec![10.0, 5.0, 20.0, 10.0, 40.0],
            sector: vec![1.0, 1.0, 1.0, 1.0, 1.0],
        }
    }

    fn aaa(&self) -> InstrumentId {
        self.universe[0]
    }

    fn bbb(&self) -> InstrumentId {
        self.universe[1]
    }
}

impl ExprSource for Fixture {
    fn column(&self, instrument_id: InstrumentId, column: &str) -> Option<&[f64]> {
        match column {
            "close" => {
                if instrument_id == self.aaa() {
                    Some(&self.aaa_close)
                } else if instrument_id == self.bbb() {
                    Some(&self.bbb_close)
                } else {
                    None
                }
            }
            // A constant sector: both fixtures share one group at every timestamp.
            "sector" => Some(&self.sector),
            _ => None,
        }
    }

    fn universe(&self) -> &[InstrumentId] {
        &self.universe
    }
}

fn pinned() -> Feature {
    Feature::parse(
        "momentum",
        TransformSide::Inference,
        "rolling_mean(close, 20) / lag(close, 1)",
    )
    .unwrap()
}

#[test]
fn lag_reads_the_previous_observation() {
    // close(AAA) = [1, 2, 4, 8, 16]; lag(close, 1) at index 3 = close[2] = 4.
    let fixture = Fixture::new();
    let feature = Feature::parse("lag1", TransformSide::Inference, "lag(close, 1)").unwrap();
    assert_eq!(feature.evaluate(&fixture, fixture.aaa(), 3), Some(4.0));
}

#[test]
fn rolling_mean_averages_the_trailing_window() {
    // window [2, 4, 8]; mean = (2 + 4 + 8) / 3 = 14 / 3.
    let fixture = Fixture::new();
    let feature = Feature::parse("ma", TransformSide::Inference, "rolling_mean(close, 3)").unwrap();
    assert_close(
        feature.evaluate(&fixture, fixture.aaa(), 3).unwrap(),
        14.0 / 3.0,
    );
}

#[test]
fn rolling_std_is_the_population_stddev() {
    // window [2, 4, 8]; mean 14 / 3; variance = ((2 - 14/3)^2 + (4 - 14/3)^2 + (8 - 14/3)^2) / 3
    //          = (64/9 + 4/9 + 100/9) / 3 = (56/3) / 3 = 56/9; std = sqrt(56) / 3.
    let fixture = Fixture::new();
    let feature = Feature::parse("vol", TransformSide::Inference, "rolling_std(close, 3)").unwrap();
    assert_close(
        feature.evaluate(&fixture, fixture.aaa(), 3).unwrap(),
        (56.0f64).sqrt() / 3.0,
    );
}

#[test]
fn return_factor_is_the_simple_return() {
    // close[3] / close[2] - 1 = 8 / 4 - 1 = 1.
    let fixture = Fixture::new();
    let feature =
        Feature::parse("ret", TransformSide::Inference, "close / lag(close, 1) - 1").unwrap();
    assert_eq!(feature.evaluate(&fixture, fixture.aaa(), 3), Some(1.0));
}

#[test]
fn cross_sectional_rank_orders_the_universe() {
    // At index 0, close(AAA) = 1 and close(BBB) = 10, so AAA ranks 1 and BBB ranks 2.
    let fixture = Fixture::new();
    let feature = Feature::parse("rank", TransformSide::Inference, "rank(close)").unwrap();
    assert_eq!(feature.evaluate(&fixture, fixture.aaa(), 0), Some(1.0));
    assert_eq!(feature.evaluate(&fixture, fixture.bbb(), 0), Some(2.0));
}

#[test]
fn cross_sectional_scale_divides_by_the_absolute_sum() {
    // At index 0, sum |close| = 1 + 10 = 11, so AAA scales to 1/11 and BBB to 10/11.
    let fixture = Fixture::new();
    let feature = Feature::parse("scale", TransformSide::Inference, "scale(close)").unwrap();
    assert_close(
        feature.evaluate(&fixture, fixture.aaa(), 0).unwrap(),
        1.0 / 11.0,
    );
    assert_close(
        feature.evaluate(&fixture, fixture.bbb(), 0).unwrap(),
        10.0 / 11.0,
    );
}

#[test]
fn neutralisation_subtracts_the_group_mean() {
    // At index 0, close(AAA) = 1 and close(BBB) = 10 share one sector; mean = 5.5, so the
    // neutralised values are 1 - 5.5 = -4.5 and 10 - 5.5 = 4.5.
    let fixture = Fixture::new();
    let feature = Feature::parse(
        "neutral",
        TransformSide::Inference,
        "neutralize(close, sector)",
    )
    .unwrap();
    assert_eq!(feature.evaluate(&fixture, fixture.aaa(), 0), Some(-4.5));
    assert_eq!(feature.evaluate(&fixture, fixture.bbb(), 0), Some(4.5));
}

#[test]
fn missing_history_is_an_absence_not_a_value() {
    // At index 0 there are fewer than three trailing observations, so the window is not full.
    let fixture = Fixture::new();
    let feature = Feature::parse("ma", TransformSide::Inference, "rolling_mean(close, 3)").unwrap();
    assert_eq!(feature.evaluate(&fixture, fixture.aaa(), 0), None);
}

#[test]
fn definition_digest_is_stable_and_pinned() {
    let feature = pinned();
    let whitespace_variant = Feature::parse(
        "momentum",
        TransformSide::Inference,
        "  rolling_mean( close , 20 )   /   lag( close , 1 )  ",
    )
    .unwrap();

    assert_eq!(feature.digest(), whitespace_variant.digest());
    assert_eq!(
        feature.to_text(),
        "(rolling_mean(close, 20) / lag(close, 1))"
    );
    assert_eq!(feature.digest().to_hex(), PINNED_DIGEST);
}

#[test]
fn definition_digest_separates_sides() {
    // The same tree on a different side is a different definition.
    let inference = pinned();
    let learning = Feature::parse(
        "momentum",
        TransformSide::Learning,
        "rolling_mean(close, 20) / lag(close, 1)",
    )
    .unwrap();
    assert_ne!(inference.digest(), learning.digest());
}

#[test]
fn combined_digest_is_independent_of_construction_order() {
    let first = Feature::parse("first", TransformSide::Inference, "close + 1").unwrap();
    let second =
        Feature::parse("second", TransformSide::Inference, "rolling_mean(close, 3)").unwrap();

    let forward = Digest::combine(&[first.digest(), second.digest()]);
    let reverse = Digest::combine(&[second.digest(), first.digest()]);

    assert_eq!(forward, reverse);
}
