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

use std::collections::BTreeMap;

use nautilus_core::UnixNanos;
use nautilus_model::identifiers::InstrumentId;
use nautilus_research::{
    AdmittedDecision, DatasetDeclaration, DatasetSplit, Digest, DigestInput, Disposition, FitModel,
    LabelDefinition, LabelError, MembershipInterval, MembershipSeries, MembershipSource,
    ParameterEstimate, ProducerIdentity, RecoveryCheck, RecoveryTolerance, RecoveryVerdict,
    ScoreObservation, SourceInterval, TimeInterval, confidence_calibration, parameter_recovery,
    signal_quality,
};

/// The digest of the declaration built by [`declaration`], pinned to detect any change to the
/// canonical serialization.
const PINNED_DIGEST: &str = "48d8731ebdf757e3efb0bdd1b43f00bc8c17d9498edfba26b5a068e17d8b70d5";

fn source(data_type: &str, identifier: &str) -> SourceInterval {
    SourceInterval::new(
        data_type,
        Some(identifier.to_string()),
        TimeInterval::new(UnixNanos::from(0), UnixNanos::from(300)),
    )
}

fn feature(name: &str, seed: &[u8]) -> DigestInput {
    DigestInput::new(name, Digest::of(seed))
}

/// A sixty-second poll over a fixed horizon, used by the declaration and record fixtures.
fn definition() -> LabelDefinition {
    LabelDefinition::new(
        "forward_return_poll",
        ProducerIdentity::Identified("collector".to_string()),
        Some(UnixNanos::from(60_000_000_000)),
        5,
    )
    .expect("the fixture definition is valid")
}

fn declaration(
    sources: &[SourceInterval],
    features: &[DigestInput],
    labels: &[DigestInput],
) -> DatasetDeclaration {
    let membership = MembershipSource::new("research", "rule/v1");
    let split = DatasetSplit::new(
        TimeInterval::new(UnixNanos::from(0), UnixNanos::from(100)),
        Some(TimeInterval::new(
            UnixNanos::from(100),
            UnixNanos::from(200),
        )),
        TimeInterval::new(UnixNanos::from(200), UnixNanos::from(300)),
    );

    let declaration = DatasetDeclaration::new(membership, split, definition());
    let declaration = sources
        .iter()
        .cloned()
        .fold(declaration, DatasetDeclaration::with_source);
    let declaration = features
        .iter()
        .cloned()
        .fold(declaration, DatasetDeclaration::with_feature);
    labels
        .iter()
        .cloned()
        .fold(declaration, DatasetDeclaration::with_label)
}

#[test]
fn identical_declarations_digest_identically() {
    let sources = [source("quotes", "A.X"), source("trades", "B.X")];
    let features = [
        feature("momentum", b"momentum-v1"),
        feature("volatility", b"volatility-v1"),
    ];
    let labels = [feature("return", b"return-v1")];

    let first = declaration(&sources, &features, &labels);
    let second = declaration(&sources, &features, &labels);

    assert_eq!(first.canonical_json(), second.canonical_json());
    assert_eq!(first.digest(), second.digest());
    assert_eq!(first.id(), PINNED_DIGEST);
}

#[test]
fn digest_is_independent_of_insertion_order() {
    let sources = [source("quotes", "A.X"), source("trades", "B.X")];
    let features = [
        feature("momentum", b"momentum-v1"),
        feature("volatility", b"volatility-v1"),
    ];
    let labels = [feature("return", b"return-v1")];

    let ordered = declaration(&sources, &features, &labels);

    let mut reversed_sources = sources.to_vec();
    reversed_sources.reverse();
    let mut reversed_features = features.to_vec();
    reversed_features.reverse();
    let mut reversed_labels = labels.to_vec();
    reversed_labels.reverse();

    let shuffled = declaration(&reversed_sources, &reversed_features, &reversed_labels);

    assert_eq!(shuffled.canonical_json(), ordered.canonical_json());
    assert_eq!(shuffled.digest(), ordered.digest());
}

#[test]
fn combined_digests_are_order_independent() {
    let a = Digest::of(b"a");
    let b = Digest::of(b"b");
    let c = Digest::of(b"c");

    assert_eq!(Digest::combine(&[a, b, c]), Digest::combine(&[c, a, b]));
    assert_ne!(Digest::combine(&[a, b]), Digest::combine(&[a, c]));
}

fn membership_series(instruments: &[&str]) -> MembershipSeries {
    let mut series = MembershipSeries::new("universe", "test");
    for instrument in instruments {
        series.push(MembershipInterval::entry(
            "universe".to_string(),
            "test".to_string(),
            InstrumentId::from(*instrument),
            UnixNanos::from(0),
            None,
        ));
    }
    series
}

fn decision(
    instrument: &str,
    label_definition: Option<LabelDefinition>,
    label: f64,
    score: f64,
) -> AdmittedDecision {
    let mut scores = BTreeMap::new();
    scores.insert(
        "alpha".to_string(),
        ScoreObservation::WithCoverage {
            value: score,
            coverage: 1.0,
        },
    );

    AdmittedDecision {
        producer: ProducerIdentity::Identified("research".to_string()),
        instrument_id: InstrumentId::from(instrument),
        ts_event: UnixNanos::from(100),
        available_at: UnixNanos::from(100),
        regime: None,
        rating: "BUY".to_string(),
        confidence: 0.5,
        horizon: 5,
        disposition: Disposition::Pass,
        scores,
        forward_return: Some(label),
        label_definition,
        eligible_signal: None,
        realization: None,
    }
}

#[test]
fn label_definition_refuses_degenerate_fields() {
    let identified = ProducerIdentity::Identified("collector".to_string());

    assert_eq!(
        LabelDefinition::new("", identified.clone(), None, 1),
        Err(LabelError::EmptyProcedure)
    );
    assert_eq!(
        LabelDefinition::new("poll", identified.clone(), None, 0),
        Err(LabelError::ZeroHorizon {
            name: "poll".to_string()
        })
    );
    assert_eq!(
        LabelDefinition::new("poll", identified, Some(UnixNanos::from(0)), 1),
        Err(LabelError::ZeroPollInterval {
            procedure: "poll".to_string()
        })
    );
    assert_eq!(
        LabelDefinition::new("poll", ProducerIdentity::Unknown, None, 1),
        Err(LabelError::UnidentifiedProducer {
            procedure: "poll".to_string()
        })
    );
}

#[test]
fn a_declaration_records_the_definition_that_produced_its_labels() {
    let sources = [source("quotes", "A.X")];
    let features = [feature("momentum", b"momentum-v1")];
    let labels = [feature("return", b"return-v1")];

    let declaration = declaration(&sources, &features, &labels);
    let definition = declaration.definition();

    assert_eq!(definition.procedure(), "forward_return_poll");
    assert_eq!(
        definition.poll_interval(),
        Some(UnixNanos::from(60_000_000_000))
    );
    assert_eq!(definition.horizon(), 5);
    assert!(declaration.canonical_json().contains("forward_return_poll"));
}

#[test]
fn labels_with_a_definition_report_the_interval_the_horizon_and_zero_undefined() {
    let series = membership_series(&["A.X", "B.X", "C.X", "D.X", "E.X"]);
    let definition = definition();

    let mut records = Vec::new();
    for (index, instrument) in ["A.X", "B.X", "C.X", "D.X", "E.X"].iter().enumerate() {
        let value = (index + 1) as f64;
        records.push(decision(
            instrument,
            Some(definition.clone()),
            value * 10.0,
            value,
        ));
    }

    let report = signal_quality(&series, &records).unwrap();

    assert_eq!(report.labels.defined, 5);
    assert_eq!(report.labels.undefined, 0);
    assert_eq!(report.labels.undefined_share(), Some(0.0));
    assert!(report.labels.is_fully_defined());

    let reported = report
        .labels
        .definition
        .as_ref()
        .expect("the defined records share one definition");
    assert_eq!(
        reported.poll_interval(),
        Some(UnixNanos::from(60_000_000_000))
    );
    assert_eq!(reported.horizon(), 5);
    assert!(report.scores.contains_key("alpha"));
}

#[test]
fn labels_without_a_definition_are_counted_and_not_scored() {
    let series = membership_series(&["A.X", "B.X", "C.X", "D.X", "E.X"]);

    let mut records = Vec::new();
    for (index, instrument) in ["A.X", "B.X", "C.X", "D.X", "E.X"].iter().enumerate() {
        let value = (index + 1) as f64;
        records.push(decision(instrument, None, value * 10.0, value));
    }

    let report = signal_quality(&series, &records).unwrap();

    // The scoring path carries the undefined count rather than a coefficient.
    assert_eq!(report.labels.defined, 0);
    assert_eq!(report.labels.undefined, 5);
    assert_eq!(report.labels.undefined_share(), Some(1.0));
    assert_eq!(report.labels.definition, None);
    assert!(report.scores.is_empty());
    assert_eq!(report.rows, 0);

    let calibration = confidence_calibration(&records);
    assert_eq!(calibration.labels.undefined, 5);
    assert!(calibration.groups.is_empty());
}

/// A model whose two parameters are recoverable from their own observations, used to prove the
/// recovery report is reproducible at a seed through the public surface.
struct ReproducibleModel;

impl FitModel for ReproducibleModel {
    type Dataset = [f64; 2];

    fn draw(&self, truth: &[f64], seed: u64) -> [f64; 2] {
        // A seed-derived perturbation, so the report is a function of the seed.
        let perturb = (seed % 7) as f64 * 0.001;
        [truth[0] + perturb, truth[1] - perturb]
    }

    fn fit(&self, dataset: &[f64; 2]) -> Vec<ParameterEstimate> {
        dataset
            .iter()
            .map(|value| ParameterEstimate {
                estimate: *value,
                interval: Some((*value - 0.1, *value + 0.1)),
            })
            .collect()
    }
}

#[test]
fn parameter_recovery_is_reproducible_at_a_seed() {
    let check = RecoveryCheck {
        truth: vec![1.0, 2.0],
        repetitions: 32,
        seed: 0x5EED,
        tolerance: RecoveryTolerance {
            max_absolute_bias: 0.05,
            max_rmse: 0.05,
            min_coverage: 0.9,
            unidentified_rmse: 0.25,
        },
    };

    let first = parameter_recovery(&ReproducibleModel, &check);
    let second = parameter_recovery(&ReproducibleModel, &check);

    assert_eq!(first, second);
    assert_eq!(first.repetitions, 32);
    assert_eq!(first.parameters.len(), 2);
    assert!(
        first
            .parameters
            .iter()
            .all(|parameter| parameter.verdict == RecoveryVerdict::Identified)
    );
}
