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

use nautilus_core::UnixNanos;
use nautilus_research::{
    DatasetDeclaration, DatasetSplit, Digest, DigestInput, MembershipSource, SourceInterval,
    TimeInterval,
};

/// The digest of the declaration built by [`declaration`], pinned to detect any change to the
/// canonical serialization.
const PINNED_DIGEST: &str = "42422e67d97e5de484f90cdc5abc306e72f56147c326c8a3fa7e16b8cc7b1499";

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

    let declaration = DatasetDeclaration::new(membership, split);
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
