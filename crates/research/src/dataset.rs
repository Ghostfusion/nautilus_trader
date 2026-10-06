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

//! Dataset declarations and their stable identity.
//!
//! A dataset is a declaration, not a caller's convention: it names the catalog intervals it reads,
//! the membership source that decides which instruments are in scope, the digests of its feature
//! and label definitions, and its train/validation/test split. The declaration serializes
//! canonically and digests to a stable identifier, so two processes that build the same dataset
//! agree on its identity regardless of the order in which its parts were inserted.
//!
//! The feature and label expression tree is a separate layer. This module reserves the seams for
//! it: the [`Feature`] and [`Label`] traits describe a named definition with a digest, and
//! [`DigestInput`] carries a name and digest into the declaration without depending on the tree.

use std::fmt::Display;

use nautilus_core::UnixNanos;
use serde::{Serialize, Serializer};

use crate::label::LabelDefinition;

/// The length in bytes of a [`Digest`].
pub const DIGEST_LEN: usize = 32;

/// A stable 32-byte digest of a canonical byte sequence.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Digest([u8; DIGEST_LEN]);

impl Digest {
    /// Creates a digest from raw bytes.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; DIGEST_LEN]) -> Self {
        Self(bytes)
    }

    /// Returns the raw digest bytes.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; DIGEST_LEN] {
        &self.0
    }

    /// Returns the BLAKE3 digest of `bytes`.
    #[must_use]
    pub fn of(bytes: &[u8]) -> Self {
        Self(*blake3::hash(bytes).as_bytes())
    }

    /// Returns the BLAKE3 digest of the concatenated parts, independent of their order.
    ///
    /// The parts are sorted before hashing, so a set of partial digests reduced across processes
    /// always yields the same combined digest.
    #[must_use]
    pub fn combine(parts: &[Self]) -> Self {
        let mut sorted: Vec<[u8; DIGEST_LEN]> = parts.iter().map(|part| part.0).collect();
        sorted.sort_unstable();

        let mut hasher = blake3::Hasher::new();
        for part in sorted {
            hasher.update(&part);
        }

        Self(*hasher.finalize().as_bytes())
    }

    /// Returns the lowercase hexadecimal representation.
    #[must_use]
    pub fn to_hex(&self) -> String {
        let mut hex = String::with_capacity(DIGEST_LEN * 2);
        for byte in self.0 {
            hex.push_str(&format!("{byte:02x}"));
        }
        hex
    }
}

impl std::fmt::Debug for Digest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Digest({})", self.to_hex())
    }
}

impl Display for Digest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.to_hex())
    }
}

impl Serialize for Digest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_hex())
    }
}

/// A named digest input to a dataset declaration.
///
/// This is the seam a feature or label definition feeds once the expression tree exists; the
/// declaration only needs the name and the digest.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct DigestInput {
    /// The canonical name of the definition.
    pub name: String,
    /// The digest of the definition.
    pub digest: Digest,
}

impl DigestInput {
    /// Creates a new [`DigestInput`].
    #[must_use]
    pub fn new(name: impl Into<String>, digest: Digest) -> Self {
        Self {
            name: name.into(),
            digest,
        }
    }
}

/// Identifies a feature definition by name and digest.
///
/// The expression tree and its operators implement this trait; the dataset declaration consumes
/// only the name and digest.
pub trait Feature {
    /// Returns the canonical feature name.
    fn name(&self) -> &str;

    /// Returns the digest of the feature definition.
    fn digest(&self) -> Digest;
}

/// Identifies a label definition by name and digest.
///
/// The expression tree and its operators implement this trait; the dataset declaration consumes
/// only the name and digest.
pub trait Label {
    /// Returns the canonical label name.
    fn name(&self) -> &str;

    /// Returns the digest of the label definition.
    fn digest(&self) -> Digest;
}

/// A half-open time interval `[start, end)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct TimeInterval {
    /// The start instant, inclusive.
    pub start: UnixNanos,
    /// The end instant, exclusive.
    pub end: UnixNanos,
}

impl TimeInterval {
    /// Creates a new [`TimeInterval`].
    #[must_use]
    pub const fn new(start: UnixNanos, end: UnixNanos) -> Self {
        Self { start, end }
    }
}

/// A catalog interval a dataset reads from.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct SourceInterval {
    /// The catalog data type name.
    pub data_type: String,
    /// The optional catalog identifier (for example an instrument ID).
    pub identifier: Option<String>,
    /// The covered interval.
    pub interval: TimeInterval,
}

impl SourceInterval {
    /// Creates a new [`SourceInterval`].
    #[must_use]
    pub fn new(
        data_type: impl Into<String>,
        identifier: Option<String>,
        interval: TimeInterval,
    ) -> Self {
        Self {
            data_type: data_type.into(),
            identifier,
            interval,
        }
    }
}

/// The universe and source identities a dataset draws membership from.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct MembershipSource {
    /// The universe identity.
    pub universe: String,
    /// The source identity of the membership series.
    pub source: String,
}

impl MembershipSource {
    /// Creates a new [`MembershipSource`].
    #[must_use]
    pub fn new(universe: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            universe: universe.into(),
            source: source.into(),
        }
    }
}

/// The train/validation/test split of a dataset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct DatasetSplit {
    /// The training interval.
    pub train: TimeInterval,
    /// The optional validation interval.
    pub validation: Option<TimeInterval>,
    /// The test interval.
    pub test: TimeInterval,
}

impl DatasetSplit {
    /// Creates a new [`DatasetSplit`].
    #[must_use]
    pub const fn new(
        train: TimeInterval,
        validation: Option<TimeInterval>,
        test: TimeInterval,
    ) -> Self {
        Self {
            train,
            validation,
            test,
        }
    }
}

/// A dataset declaration.
///
/// The declaration carries everything that defines the dataset's identity: its catalog source
/// intervals, its membership source, the definition that produced its labels, the digests of its
/// feature and label definitions, and its split. It digests canonically, independent of the
/// insertion order of its parts.
///
/// The label definition is a required argument rather than an optional field, so a dataset cannot
/// be declared without saying how its labels were produced (`design 4 I13`). This costs every
/// caller a definition and changes the declaration's digest, because the definition is part of the
/// dataset's identity: two datasets whose labels were produced differently are different datasets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DatasetDeclaration {
    membership: MembershipSource,
    split: DatasetSplit,
    definition: LabelDefinition,
    sources: Vec<SourceInterval>,
    features: Vec<DigestInput>,
    labels: Vec<DigestInput>,
}

impl DatasetDeclaration {
    /// Creates a new [`DatasetDeclaration`] with a membership source, a split and the label
    /// definition that produced the dataset's labels.
    #[must_use]
    pub fn new(
        membership: MembershipSource,
        split: DatasetSplit,
        definition: LabelDefinition,
    ) -> Self {
        Self {
            membership,
            split,
            definition,
            sources: Vec::new(),
            features: Vec::new(),
            labels: Vec::new(),
        }
    }

    /// Returns the membership source.
    #[must_use]
    pub fn membership(&self) -> &MembershipSource {
        &self.membership
    }

    /// Returns the split.
    #[must_use]
    pub fn split(&self) -> &DatasetSplit {
        &self.split
    }

    /// Returns the label definition that produced the dataset's labels.
    #[must_use]
    pub fn definition(&self) -> &LabelDefinition {
        &self.definition
    }

    /// Returns the source intervals in insertion order.
    #[must_use]
    pub fn sources(&self) -> &[SourceInterval] {
        &self.sources
    }

    /// Returns the feature digest inputs in insertion order.
    #[must_use]
    pub fn features(&self) -> &[DigestInput] {
        &self.features
    }

    /// Returns the label digest inputs in insertion order.
    #[must_use]
    pub fn labels(&self) -> &[DigestInput] {
        &self.labels
    }

    /// Adds a catalog source interval, returning the declaration.
    #[must_use]
    pub fn with_source(mut self, source: SourceInterval) -> Self {
        self.sources.push(source);
        self
    }

    /// Adds a feature digest input, returning the declaration.
    #[must_use]
    pub fn with_feature(mut self, feature: DigestInput) -> Self {
        self.features.push(feature);
        self
    }

    /// Adds a label digest input, returning the declaration.
    #[must_use]
    pub fn with_label(mut self, label: DigestInput) -> Self {
        self.labels.push(label);
        self
    }

    /// Returns the canonical JSON serialization of the declaration.
    ///
    /// The serialization is deterministic: object keys are sorted, and every collection is sorted
    /// by its canonical fields, so the output does not depend on the insertion order of the
    /// declaration's parts.
    #[must_use]
    pub fn canonical_json(&self) -> String {
        let mut sources = self.sources.clone();
        sources.sort();

        let mut features = self.features.clone();
        features.sort();

        let mut labels = self.labels.clone();
        labels.sort();

        let value = serde_json::json!({
            "membership": self.membership,
            "split": self.split,
            "definition": self.definition,
            "sources": sources,
            "features": features,
            "labels": labels,
        });

        value.to_string()
    }

    /// Returns the stable digest of the declaration's canonical serialization.
    #[must_use]
    pub fn digest(&self) -> Digest {
        Digest::of(self.canonical_json().as_bytes())
    }

    /// Returns the hexadecimal identifier of the declaration's digest.
    #[must_use]
    pub fn id(&self) -> String {
        self.digest().to_hex()
    }
}
