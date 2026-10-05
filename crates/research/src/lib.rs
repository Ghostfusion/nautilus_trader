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

//! Point-in-time research datasets for [NautilusTrader](https://nautilustrader.io).
//!
//! The `nautilus-research` crate defines the dataset contract that keeps a research pipeline
//! honest about time. It is deliberately independent of the trading engine: it depends only on
//! the domain model, the catalog, and serialization, so a research process can build and version
//! datasets without instantiating a node.
//!
//! - [`membership`]: universe membership history stored as data beside the data it describes,
//!   with point-in-time resolution and catalog persistence.
//! - [`dataset`]: the dataset declaration, its canonical serialization, and its stable digest.
//! - [`panel`]: a point-in-time panel whose membership and feature apertures are checked by
//!   construction rather than left to a caller's convention.
//! - [`operators`]: the time-series and cross-sectional operator set, with the convention that
//!   defines each operator stated explicitly.
//! - [`feature`]: a feature compiled once from a plain-text definition into a typed expression
//!   tree, with a canonical serialization that digests to a stable identifier, a declared side,
//!   and provenance for every value it emits.
//! - [`label`]: forward labels, each with a horizon and a stated terminal convention.
//! - [`measurement`]: the measurement of an admitted decision stream, with the three experiments
//!   of the research-to-execution bridge kept apart, coverage-aware signal quality, confidence
//!   calibration, the reduction factor's effect, redundancy and regime conditioning.
//!
//! The compiled feature and label definitions implement the [`dataset::Feature`] and
//! [`dataset::Label`] seams, so a dataset declaration consumes only their names and digests. The
//! factor pipeline is a general research primitive; it is authoritative in the research pipeline
//! and nowhere else, and execution stays on the normal strategy and execution path.

#![warn(rustc::all)]
#![deny(unsafe_code)]
#![deny(unsafe_op_in_unsafe_fn)]
#![deny(nonstandard_style)]
#![deny(missing_debug_implementations)]
#![deny(clippy::missing_errors_doc)]
#![deny(rustdoc::broken_intra_doc_links)]

pub mod dataset;
pub mod feature;
pub mod label;
pub mod measurement;
pub mod membership;
pub mod operators;
pub mod panel;

pub use dataset::{
    DatasetDeclaration, DatasetSplit, Digest, DigestInput, MembershipSource, SourceInterval,
    TimeInterval,
};
pub use feature::{
    AttributedValue, Expr, ExprSource, Feature, FeatureError, ParseError, Provenance,
};
pub use label::{Label, LabelError, LabelKind};
pub use measurement::{
    AdmittedDecision, CONFIDENCE_BANDS, CVAR_TAIL_FRACTION, CalibrationGroup, CalibrationReport,
    ConfidenceBand, Disposition, DispositionOutcome, EligibleSignal, Exclusions,
    ExecutionRealizationReport, InformationCoefficient, InformationCoefficientPoint, Metric,
    MetricComparison, MetricEstimate, PolicyEffectReport, ProducerIdentity,
    REDUNDANCY_CORRELATION_THRESHOLD, Realization, ReductionEffectReport, RedundancyReport,
    RiskDecision, ScoreCluster, ScoreCorrelation, ScoreObservation, SignalQualityReport,
    confidence_calibration, execution_realization, policy_effect, reduction_effect, redundancy,
    restrict_to_regime, signal_quality,
};
pub use membership::{MembershipInterval, MembershipRule, MembershipSeries, MembershipSpell};
pub use operators::{
    Operator, TransformSide, cross_sectional_rank, cross_sectional_scale, cross_sectional_sum,
    neutralize, rolling_correlation, rolling_mean, rolling_rank, rolling_regression_residual,
    rolling_std,
};
pub use panel::{FeatureValue, Panel, PanelError, PanelRow};
