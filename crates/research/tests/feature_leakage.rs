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

//! A deliberately leaky feature - one that reads its own future - is detected, and the sides that
//! keep it out of an inference value are enforced.

use nautilus_model::identifiers::InstrumentId;
use nautilus_research::{ExprSource, Feature, FeatureError, ParseError, TransformSide};

fn instrument(value: &str) -> InstrumentId {
    InstrumentId::from(value)
}

struct Fixture {
    universe: Vec<InstrumentId>,
    close: Vec<f64>,
}

impl ExprSource for Fixture {
    fn column(&self, instrument_id: InstrumentId, column: &str) -> Option<&[f64]> {
        if column == "close" && instrument_id == self.universe[0] {
            Some(&self.close)
        } else {
            None
        }
    }

    fn universe(&self) -> &[InstrumentId] {
        &self.universe
    }
}

#[test]
fn forward_reading_feature_is_rejected_on_the_inference_side() {
    // `lead` reads the next observation, which at evaluation time is the feature's own future.
    // On the inference side the compiler refuses it, so a leaky feature cannot exist.
    let error = Feature::parse("leak", TransformSide::Inference, "lead(close, 1)").unwrap_err();
    assert_eq!(
        error,
        ParseError::FutureRead {
            function: "lead".to_string(),
        }
    );
}

#[test]
fn forward_reading_feature_is_allowed_on_the_learning_side() {
    let feature = Feature::parse("target", TransformSide::Learning, "lead(close, 1)").unwrap();
    let fixture = Fixture {
        universe: vec![instrument("AAA.X")],
        close: vec![1.0, 2.0, 4.0],
    };

    // The lead reads the observation after the row, which is exactly the future it must not read
    // as a feature; on the learning side it is permitted and observable.
    assert_eq!(
        feature.evaluate(&fixture, fixture.universe[0], 1),
        Some(4.0)
    );
}

#[test]
fn a_definition_cannot_be_used_on_the_other_side() {
    let feature = Feature::parse("mom", TransformSide::Inference, "lag(close, 1)").unwrap();

    assert!(feature.assert_side(TransformSide::Inference).is_ok());
    assert_eq!(
        feature.assert_side(TransformSide::Learning),
        Err(FeatureError::SideMismatch {
            name: "mom".to_string(),
            declared: TransformSide::Inference,
            requested: TransformSide::Learning,
        })
    );
}

#[test]
fn malformed_definitions_are_typed_errors_not_reinterpretations() {
    assert_eq!(
        Feature::parse("bad", TransformSide::Inference, "eval(close)").unwrap_err(),
        ParseError::UnknownFunction {
            name: "eval".to_string(),
        }
    );
    assert_eq!(
        Feature::parse("bad", TransformSide::Inference, "rolling_mean(close, 0)").unwrap_err(),
        ParseError::InvalidArgument {
            name: "rolling_mean".to_string(),
            expected: "a positive integer window or lag",
        }
    );
    assert_eq!(
        Feature::parse(
            "bad",
            TransformSide::Inference,
            "neutralize(close, rank(close))"
        )
        .unwrap_err(),
        ParseError::InvalidArgument {
            name: "neutralize".to_string(),
            expected: "a group column as its second argument",
        }
    );
    assert_eq!(
        Feature::parse("bad", TransformSide::Inference, "lag(close)").unwrap_err(),
        ParseError::Arity {
            name: "lag".to_string(),
            expected: 2,
            actual: 1,
        }
    );
}
