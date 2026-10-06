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

//! Measurement of the research-to-execution bridge (`design 6.1`-`6.5`).
//!
//! The engine half of the bridge is worth having for two things: execution, and knowing what the
//! research decisions were worth. This module is the second: it measures an admitted stream of
//! decisions so that a poor result is attributed to the right cause rather than to the nearest
//! number.
//!
//! **Three experiments, kept apart (`design 6.1`).** Poor performance has several different
//! causes, and the three experiments measure each where it happens:
//!
//! - A. research signal quality: the research decision against a forward return, with no bridge.
//!   [`signal_quality`] returns a [`SignalQualityReport`].
//! - B. bridge policy effect: the research decision against the eligible signals the policy yields.
//!   [`policy_effect`] returns a [`PolicyEffectReport`].
//! - C. execution realization: an eligible signal against its target, its risk decision and its
//!   fill. [`execution_realization`] returns an [`ExecutionRealizationReport`].
//!
//! The three reports are three distinct types produced by three distinct functions. There is
//! deliberately no type, function or trait in this module that consumes two of them, so no API can
//! pool the three into one number: a strong information coefficient with weak realised profit stays
//! a finding about B or C rather than about A.
//!
//! Beyond the three experiments: [`confidence_calibration`] measures the artifact's `confidence`
//! against realised hit rates by bucket and never as one pooled number (`design 6.3`), and
//! [`reduction_effect`] measures the `UNCERTAIN` reduction factor across a five-metric set rather
//! than on one number, with "no meaningful difference" an admissible result (`design 6.3`).
//! [`redundancy`] reports the correlation structure of the score set and how many independent
//! hypotheses it really contains (`design 6.4`), and [`restrict_to_regime`] conditions every
//! measurement on a regime verdict supplied by the caller, with no regime engine here
//! (`design 6.4`).
//!
//! A fitted model is measured for parameter recovery by [`parameter_recovery`] over a [`FitModel`]
//! seam and a declared [`RecoveryTolerance`] set (`design D4`): it draws R seeded datasets at a
//! known parameter vector, refits each, and reports per-parameter bias, RMSE, interval coverage
//! and a verdict of [`RecoveryVerdict::Identified`], [`RecoveryVerdict::WeaklyIdentified`] or
//! [`RecoveryVerdict::Unidentified`]. The verdict is a required field of every
//! [`ParameterRecovery`], so a parameter that cannot be recovered is labelled rather than
//! described with a point estimate alone.
//!
//! **Two rules travel with every aggregate.** A score observation that arrived without its
//! coverage is an absence and is reported as such, never measured as a neutral value
//! (`design 4 I11`); and a record whose producer identity is unknown is excluded from every
//! aggregate and counted in [`Exclusions::unknown_producer`] rather than pooled (`design 4 I12`).
//! Producer identity and authorization are separate properties: authorization is established
//! upstream by the admission gate, which refuses a producer that is identified but not authorised,
//! so this aggregate-side module distinguishes only the case it must act on.
//!
//! Every aggregate reads no clock, holds no global state and is a pure function of its inputs, so
//! equal inputs produce equal outputs.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use nautilus_core::UnixNanos;
use nautilus_model::{identifiers::InstrumentId, signal::SignalDirection};
use serde::{Serialize, Serializer};
use thiserror::Error;

use crate::label::LabelDefinition;
use crate::membership::MembershipSeries;
use crate::operators::{cross_sectional_rank, rolling_correlation, rolling_std};
use crate::panel::{Panel, PanelError, PanelRow};

/// The number of equal confidence bands a calibration reports (`design 6.3`).
pub const CONFIDENCE_BANDS: usize = 10;

/// The tail fraction a conditional value at risk averages over (`design 6.3`).
pub const CVAR_TAIL_FRACTION: f64 = 0.05;

/// The absolute correlation at or above which two scores are treated as one hypothesis
/// (`design 6.4`).
pub const REDUNDANCY_CORRELATION_THRESHOLD: f64 = 0.7;

/// The number of distinct declared specifications a bound pair needs before it can be compared
/// (`design 6.2`).
///
/// One specification is one reading of a search, not a range: the pair's two extremes coincide
/// with it, so a single declared specification is reported as uncheckable rather than as one
/// figure.
pub const MINIMUM_BOUND_SPECIFICATIONS: usize = 2;

/// The producer identity carried by an admitted decision (`design 4 I12`).
///
/// Identity and authorization are separate properties. Identity says *who spoke*; authorization
/// says *whether they may be trusted*. Authorization is established by the admission gate, which
/// refuses a producer that is identified but not authorised, so an aggregate never sees that case.
/// This type therefore distinguishes only the two cases the aggregate side must act on: an
/// identified producer, and an unknown one, which is excluded from every aggregate and reported in
/// [`Exclusions::unknown_producer`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProducerIdentity {
    /// The producer is identified. Its authorization was established upstream at admission.
    Identified(String),
    /// The producer identity is unknown. The record is admitted but pooled with nothing.
    Unknown,
}

impl ProducerIdentity {
    /// Returns whether the producer identity is unknown.
    #[must_use]
    pub const fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown)
    }
}

impl Serialize for ProducerIdentity {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Identified(name) => serializer.serialize_str(&format!("identified:{name}")),
            Self::Unknown => serializer.serialize_str("unknown"),
        }
    }
}

/// A research score observation together with its coverage (`design 4 I11`).
///
/// The producer withholds a composite below a coverage floor and records coverage beside the
/// score, so a score without its coverage is a different object from the same score with it. The
/// two cases are modelled here rather than collapsed into an `Option<f64>`, so an absent score can
/// never read as a neutral value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScoreObservation {
    /// The score value and the coverage it arrived with.
    WithCoverage {
        /// The score value.
        value: f64,
        /// The coverage the score was produced with.
        coverage: f64,
    },
    /// The score arrived without its coverage. It is an absence and is reported as one.
    CoverageAbsent,
}

/// The bridge's resolved disposition for a decision (`design 3.2`, `6.3`).
///
/// `Pass` and `Uncertain` are the populations the reduction factor is measured over; `Restrict`
/// yields no eligible signal and is reported for completeness.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Disposition {
    /// The research claim raises no objection.
    Pass,
    /// The research claim permits the action at reduced risk.
    Uncertain,
    /// The research claim refuses the action.
    Restrict,
}

impl Disposition {
    /// Returns the canonical uppercase name of the disposition.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Uncertain => "UNCERTAIN",
            Self::Restrict => "RESTRICT",
        }
    }
}

/// The eligible signal the policy yielded for a decision (`design 6.1`, experiment B).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EligibleSignal {
    /// The direction the policy resolved the decision to.
    pub direction: SignalDirection,
    /// The policy magnitude carried by the signal, never the model's confidence.
    pub strength: f64,
}

/// The risk engine's decision on an eligible signal (`design 6.1`, experiment C).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RiskDecision {
    /// The risk engine approved the signal.
    Approved,
    /// The risk engine denied the signal with a typed denial.
    Denied,
}

/// The execution realization of an eligible signal (`design 6.1`, experiment C).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Realization {
    /// The engine-constructed target allocation, if a target was constructed.
    pub target: Option<f64>,
    /// The risk engine's decision on the signal.
    pub risk: RiskDecision,
    /// The allocation actually filled, if a fill occurred.
    pub fill: Option<f64>,
    /// The realised return of the filled exposure, if a fill occurred.
    pub realized_return: Option<f64>,
}

/// One admitted decision on the measurement stream (`design 6.1`).
///
/// The stream is shared by the three experiments and carries, per record, each experiment's own
/// inputs: the research scores and their coverage and the forward label (experiment A), the
/// eligible signal the policy yields (experiment B), and the target, risk decision and fill
/// (experiment C). The struct being shared is what makes the three **separately reportable over
/// one stream**; the reports themselves stay distinct types.
///
/// `ts_event` is the row timestamp the record is measured at, and `available_at` is the instant
/// the record became available. A panel built from the stream uses `available_at` as each
/// feature's aperture, so a record timestamped at its reference date rather than its availability
/// cannot be constructed into a panel (`design 4 I4`).
#[derive(Clone, Debug, PartialEq)]
pub struct AdmittedDecision {
    /// The producer identity (`design 4 I12`).
    pub producer: ProducerIdentity,
    /// The instrument the decision is about.
    pub instrument_id: InstrumentId,
    /// The row timestamp the decision is measured at.
    pub ts_event: UnixNanos,
    /// The instant the decision became available; the feature aperture of the panel.
    pub available_at: UnixNanos,
    /// The regime verdict the record was admitted under, if any (`design 6.4`).
    pub regime: Option<String>,
    /// The research rating.
    pub rating: String,
    /// The artifact's self-reported confidence, measured by bucket and never used as a size.
    pub confidence: f64,
    /// The horizon of the decision, in observations.
    pub horizon: usize,
    /// The resolved bridge disposition.
    pub disposition: Disposition,
    /// The research scores with their coverage (`design 4 I11`).
    pub scores: BTreeMap<String, ScoreObservation>,
    /// The forward return of the label, absent at the end of the data (`design 6.2`).
    pub forward_return: Option<f64>,
    /// The definition that produced the label, absent when the label was added without one
    /// (`design 4 I13`). A record whose definition is absent is counted by
    /// [`LabelProvenance::undefined`] and is not scored.
    pub label_definition: Option<LabelDefinition>,
    /// The eligible signal the policy yields, if any (`design 6.1`, experiment B).
    pub eligible_signal: Option<EligibleSignal>,
    /// The realization of an eligible signal, if one exists (`design 6.1`, experiment C).
    pub realization: Option<Realization>,
}

/// The count of records excluded from an aggregate, reported rather than silently dropped.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Exclusions {
    /// Admitted records excluded from every aggregate because the producer identity is unknown
    /// (`design 4 I12`).
    pub unknown_producer: usize,
}

/// The label provenance of a measured stream (`design 4 I13`).
///
/// Every record either carries the [`LabelDefinition`] that produced its label or does not. Both
/// counts are reported as first-class numbers, so an undefined-label share is visible rather than
/// inferred from a log line. A record whose definition is absent is never scored: a score over
/// labels of unknown provenance would not be interpretable, so it is refused and the count is
/// carried into the report instead.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LabelProvenance {
    /// The number of records whose label carried a definition.
    pub defined: usize,
    /// The number of records whose label was added without a definition.
    pub undefined: usize,
    /// The definition shared by the defined records, present when there is exactly one.
    pub definition: Option<LabelDefinition>,
}

impl LabelProvenance {
    /// Returns the share of records whose label carried no definition, absent for an empty stream.
    #[must_use]
    pub fn undefined_share(&self) -> Option<f64> {
        let total = self.defined + self.undefined;
        if total == 0 {
            None
        } else {
            Some(self.undefined as f64 / total as f64)
        }
    }

    /// Returns whether every record carried the definition that produced its label.
    #[must_use]
    pub const fn is_fully_defined(&self) -> bool {
        self.undefined == 0
    }
}

/// Partitions the scored records into those whose label carried a definition and counts the rest,
/// reporting the single shared definition when the defined records agree on one.
fn label_provenance(defined: &[&AdmittedDecision], undefined: usize) -> LabelProvenance {
    let definition = defined
        .first()
        .and_then(|record| record.label_definition.as_ref())
        .filter(|first| {
            defined
                .iter()
                .all(|record| record.label_definition.as_ref() == Some(*first))
        })
        .cloned();

    LabelProvenance {
        defined: defined.len(),
        undefined,
        definition,
    }
}

/// An information coefficient at one date, with its cross-sectional uncertainty (`design 6.2`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InformationCoefficientPoint {
    /// The date the cross-section was measured at.
    pub ts_event: UnixNanos,
    /// The rank information coefficient, absent if the cross-section was too small or degenerate.
    pub ic: Option<f64>,
    /// The number of instrument-label pairs that contributed.
    pub n: usize,
    /// The jackknife standard error of `ic` over the cross-section, absent if undefined.
    pub standard_error: Option<f64>,
}

/// The information coefficient series of one score with its uncertainty (`design 6.2`).
#[derive(Clone, Debug, PartialEq)]
pub struct InformationCoefficient {
    /// The per-date series, one point per date in the panel.
    pub series: Vec<InformationCoefficientPoint>,
    /// The mean of the defined per-date coefficients.
    pub mean: Option<f64>,
    /// The jackknife standard error of the mean across dates.
    pub standard_error: Option<f64>,
    /// The 95% interval of the mean, absent if no standard error is defined.
    pub interval: Option<(f64, f64)>,
    /// The number of dates with a defined coefficient.
    pub dates: usize,
    /// The number of instrument-label pairs that contributed across the series.
    pub observations: usize,
    /// The number of observations withheld for want of coverage (`design 4 I11`).
    pub coverage_absent: usize,
    /// The number of observations whose forward label was absent.
    pub label_absent: usize,
}

impl InformationCoefficient {
    /// Returns the coefficient's significance: the mean over its jackknife standard error.
    ///
    /// The significance is the distance of the coefficient's mean from zero in units of its own
    /// uncertainty, so a mean of one standard error above zero has significance one. An absent mean
    /// or standard error, or a non-positive standard error, is an explicit absence rather than a
    /// zero.
    #[must_use]
    pub fn significance(&self) -> Option<f64> {
        let mean = self.mean?;
        let standard_error = self.standard_error?;
        if standard_error > 0.0 {
            Some(mean.abs() / standard_error)
        } else {
            None
        }
    }
}

/// Which extreme of a declared specification pair a reported bound names (`design 6.2`).
///
/// A bound is only comparable once it names the specification it was measured under and which end
/// of the range it is, so the extreme travels with the value rather than being implied by a pair's
/// order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SpecificationExtreme {
    /// The higher bound: the reading under which the finding looks strongest.
    MostFavourable,
    /// The lower bound: the reading under which the finding looks weakest.
    LeastFavourable,
}

impl SpecificationExtreme {
    /// Returns the canonical lowercase name of the extreme.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::MostFavourable => "most_favourable",
            Self::LeastFavourable => "least_favourable",
        }
    }
}

/// A significance bound together with the specification that produced it.
///
/// A bound whose specification is unnamed cannot be compared, so the name travels with the value.
/// The extreme is carried too, so a caller cannot confuse which end of the range a bound is from
/// the order a pair happens to be in.
#[derive(Clone, Debug, PartialEq)]
pub struct SpecificationBound {
    /// The declared specification the bound was measured under.
    pub specification: String,
    /// Which extreme of the declared pair this bound is.
    pub extreme: SpecificationExtreme,
    /// The bound's value.
    pub value: f64,
}

/// The reasons a specification bound pair cannot be formed (`design 6.2`).
#[derive(Clone, Debug, PartialEq, Error)]
pub enum SpecificationBoundError {
    /// A report that carries one bound rather than both extremes.
    #[error(
        "a significance bound report must carry both specification extremes: a single bound \
         cannot be compared and is refused"
    )]
    SingleBound,
    /// A bound whose specification is unnamed cannot be compared.
    #[error("a significance bound must name the specification that produced it")]
    UnnamedSpecification,
    /// Fewer than two distinct declared specifications: the bound cannot be checked.
    #[error(
        "the significance bound could not be checked: the search declared {declared} distinct \
         specification(s), and a bound is only a bound once it is read at two extremes"
    )]
    NotCheckable {
        /// The number of distinct declared specifications.
        declared: usize,
    },
    /// Two measurements record one specification, so the pair is ambiguous.
    #[error("two measurements record the specification `{specification}`")]
    DuplicateSpecification {
        /// The duplicated specification.
        specification: String,
    },
    /// The bound is not defined under a declared specification.
    #[error("the significance bound is not defined under the specification `{specification}`")]
    Undefined {
        /// The specification that produced no bound.
        specification: String,
    },
    /// A bound is labelled the wrong extreme for its position in the pair.
    #[error("the {expected} bound is labelled {found}")]
    MislabelledExtreme {
        /// The extreme the position requires.
        expected: &'static str,
        /// The extreme the bound carries.
        found: &'static str,
    },
    /// The pair is not ordered: the most favourable bound is below the least favourable one.
    #[error("the most favourable bound {most} is below the least favourable bound {least}")]
    Unordered {
        /// The most favourable value.
        most: f64,
        /// The least favourable value.
        least: f64,
    },
}

/// A significance bound reported at both declared specification extremes (`design 6.2`).
///
/// The bound depends on the specification the finding was measured under, so a single figure is
/// one reading of a search rather than a bound on the finding. This report carries the most and the
/// least favourable declared specifications, each naming itself in its own bound, and refuses to
/// exist with one.
#[derive(Clone, Debug, PartialEq)]
pub struct SpecificationBounds {
    /// The bound at the most favourable declared specification, which is the higher one.
    pub most_favourable: SpecificationBound,
    /// The bound at the least favourable declared specification, which is the lower one.
    pub least_favourable: SpecificationBound,
}

impl SpecificationBounds {
    /// Builds the pair from the bounds a report carries, refusing a single-bound report.
    ///
    /// # Errors
    ///
    /// Returns [`SpecificationBoundError::SingleBound`] unless exactly two bounds are given, one at
    /// each extreme, and whatever [`Self::new`] refuses.
    pub fn from_bounds(bounds: &[SpecificationBound]) -> Result<Self, SpecificationBoundError> {
        if bounds.len() != MINIMUM_BOUND_SPECIFICATIONS {
            return Err(SpecificationBoundError::SingleBound);
        }

        let most_favourable = bounds
            .iter()
            .find(|bound| bound.extreme == SpecificationExtreme::MostFavourable)
            .ok_or(SpecificationBoundError::SingleBound)?
            .clone();
        let least_favourable = bounds
            .iter()
            .find(|bound| bound.extreme == SpecificationExtreme::LeastFavourable)
            .ok_or(SpecificationBoundError::SingleBound)?
            .clone();

        Self::new(most_favourable, least_favourable)
    }

    /// Builds the pair from its two labelled bounds.
    ///
    /// # Errors
    ///
    /// Returns [`SpecificationBoundError::MislabelledExtreme`] if a bound is not labelled for its
    /// position, [`SpecificationBoundError::UnnamedSpecification`] if a specification is empty, and
    /// [`SpecificationBoundError::Unordered`] if the most favourable bound is below the least
    /// favourable one.
    pub fn new(
        most_favourable: SpecificationBound,
        least_favourable: SpecificationBound,
    ) -> Result<Self, SpecificationBoundError> {
        if most_favourable.extreme != SpecificationExtreme::MostFavourable {
            return Err(SpecificationBoundError::MislabelledExtreme {
                expected: SpecificationExtreme::MostFavourable.name(),
                found: most_favourable.extreme.name(),
            });
        }
        if least_favourable.extreme != SpecificationExtreme::LeastFavourable {
            return Err(SpecificationBoundError::MislabelledExtreme {
                expected: SpecificationExtreme::LeastFavourable.name(),
                found: least_favourable.extreme.name(),
            });
        }
        if most_favourable.specification.is_empty() || least_favourable.specification.is_empty() {
            return Err(SpecificationBoundError::UnnamedSpecification);
        }
        if most_favourable.value < least_favourable.value {
            return Err(SpecificationBoundError::Unordered {
                most: most_favourable.value,
                least: least_favourable.value,
            });
        }

        Ok(Self {
            most_favourable,
            least_favourable,
        })
    }
}

/// Returns the information coefficient's significance at both declared specification extremes.
///
/// Each entry pairs a declared specification name with the coefficient measured under it. The
/// significance of each coefficient is [`InformationCoefficient::significance`], and the pair is
/// the highest and the lowest of them, each naming its specification. Fewer than two distinct
/// declared specifications is not a comparison: the bound could not be checked, and this returns an
/// error rather than one figure. Ties break towards the first declared specification for both
/// extremes, so a search that varied nothing that moves the bound reports two equal bounds naming
/// one specification rather than one bound.
///
/// # Errors
///
/// Returns [`SpecificationBoundError::UnnamedSpecification`] if a specification name is empty,
/// [`SpecificationBoundError::DuplicateSpecification`] if a specification is declared twice,
/// [`SpecificationBoundError::NotCheckable`] if fewer than two distinct specifications are
/// declared, [`SpecificationBoundError::Undefined`] if a coefficient has no significance under its
/// specification, and whatever [`SpecificationBounds::new`] refuses.
pub fn information_coefficient_bounds(
    declared: &[(&str, &InformationCoefficient)],
) -> Result<SpecificationBounds, SpecificationBoundError> {
    let mut seen: Vec<&str> = Vec::with_capacity(declared.len());
    for (specification, _) in declared.iter().copied() {
        if specification.is_empty() {
            return Err(SpecificationBoundError::UnnamedSpecification);
        }
        if seen.contains(&specification) {
            return Err(SpecificationBoundError::DuplicateSpecification {
                specification: specification.to_string(),
            });
        }
        seen.push(specification);
    }
    if seen.len() < MINIMUM_BOUND_SPECIFICATIONS {
        return Err(SpecificationBoundError::NotCheckable {
            declared: seen.len(),
        });
    }

    let mut most: Option<(usize, &str, f64)> = None;
    let mut least: Option<(usize, &str, f64)> = None;
    for (index, (specification, coefficient)) in declared.iter().copied().enumerate() {
        let Some(value) = coefficient.significance() else {
            return Err(SpecificationBoundError::Undefined {
                specification: specification.to_string(),
            });
        };
        if most.is_none_or(|(_, _, best)| value > best) {
            most = Some((index, specification, value));
        }
        if least.is_none_or(|(_, _, best)| value < best) {
            least = Some((index, specification, value));
        }
    }

    let (_, most_specification, most_value) =
        most.ok_or(SpecificationBoundError::NotCheckable {
            declared: seen.len(),
        })?;
    let (_, least_specification, least_value) =
        least.ok_or(SpecificationBoundError::NotCheckable {
            declared: seen.len(),
        })?;

    SpecificationBounds::new(
        SpecificationBound {
            specification: most_specification.to_string(),
            extreme: SpecificationExtreme::MostFavourable,
            value: most_value,
        },
        SpecificationBound {
            specification: least_specification.to_string(),
            extreme: SpecificationExtreme::LeastFavourable,
            value: least_value,
        },
    )
}

/// Experiment A: research signal quality, the research decision against a forward return
/// (`design 6.1`).
#[derive(Clone, Debug, PartialEq)]
pub struct SignalQualityReport {
    /// One information coefficient series per score, keyed by score name.
    pub scores: BTreeMap<String, InformationCoefficient>,
    /// The number of point-in-time panel rows the measurement was built over.
    pub rows: usize,
    /// The provenance of the labels the measurement was built over (`design 4 I13`).
    pub labels: LabelProvenance,
    /// The records excluded from the aggregate.
    pub exclusions: Exclusions,
}

/// The outcome of one disposition in experiment B (`design 6.1`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DispositionOutcome {
    /// The resolved disposition.
    pub disposition: Disposition,
    /// The number of decisions with this disposition.
    pub decisions: usize,
    /// The number of those decisions that yielded an eligible signal.
    pub eligible: usize,
}

/// Experiment B: bridge policy effect, the research decision against the eligible signals the
/// policy yields (`design 6.1`).
#[derive(Clone, Debug, PartialEq)]
pub struct PolicyEffectReport {
    /// The number of admitted decisions measured.
    pub decisions: usize,
    /// The decisions and eligible yields, one row per disposition present.
    pub by_disposition: Vec<DispositionOutcome>,
    /// The number of decisions that yielded an eligible signal.
    pub eligible: usize,
    /// The number of decisions the policy declined.
    pub declined: usize,
    /// The eligible fraction of the decisions, absent when there are no decisions.
    pub eligibility_rate: Option<f64>,
    /// The records excluded from the aggregate.
    pub exclusions: Exclusions,
}

/// Experiment C: execution realization, an eligible signal against its target, its risk decision
/// and its fill (`design 6.1`).
#[derive(Clone, Debug, PartialEq)]
pub struct ExecutionRealizationReport {
    /// The number of eligible signals measured.
    pub eligible: usize,
    /// The eligible signals for which the engine constructed a target.
    pub targeted: usize,
    /// The eligible signals the risk engine approved.
    pub risk_approved: usize,
    /// The eligible signals the risk engine denied.
    pub risk_denied: usize,
    /// The eligible signals that realized a fill.
    pub filled: usize,
    /// The filled fraction of the eligible signals, absent when there are none.
    pub fill_rate: Option<f64>,
    /// The mean filled-to-constructed allocation ratio, over signals with both.
    pub mean_fill_to_target: Option<f64>,
    /// The mean realised return over filled signals.
    pub mean_realized_return: Option<f64>,
    /// The records excluded from the aggregate.
    pub exclusions: Exclusions,
}

/// One metric of the reduction factor's metric set (`design 6.3`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Metric {
    /// The arithmetic mean of the per-decision realised returns.
    Expectancy,
    /// The mean return divided by its population standard deviation, with no annualisation
    /// because no clock is read.
    Sharpe,
    /// The fraction of realised returns above zero.
    HitRate,
    /// The deepest peak-to-trough decline of the compounded equity path, as a non-positive
    /// fraction.
    MaxDrawdown,
    /// The conditional value at risk: the mean of the worst [`CVAR_TAIL_FRACTION`] of returns, in
    /// return space, where a negative value is a loss.
    Cvar,
}

impl Metric {
    /// Every metric of the set, in report order.
    pub const ALL: [Self; 5] = [
        Self::Expectancy,
        Self::Sharpe,
        Self::HitRate,
        Self::MaxDrawdown,
        Self::Cvar,
    ];

    /// Returns the canonical lowercase name of the metric.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Expectancy => "expectancy",
            Self::Sharpe => "sharpe",
            Self::HitRate => "hit_rate",
            Self::MaxDrawdown => "max_drawdown",
            Self::Cvar => "cvar",
        }
    }
}

/// One metric estimated over one population, with its uncertainty (`design 6.3`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MetricEstimate {
    /// The metric.
    pub metric: Metric,
    /// The number of realised returns the estimate was computed over.
    pub sample: usize,
    /// The estimate, absent if it is undefined for the sample.
    pub estimate: Option<f64>,
    /// The jackknife standard error of the estimate, absent if undefined.
    pub standard_error: Option<f64>,
}

/// The reduction factor's measured effect on one metric (`design 6.3`).
///
/// The difference is `pass - uncertain`, reported without any direction claim: a reduction is
/// justified only if the difference is favourable in risk-adjusted terms, and three outcomes are
/// legitimate findings, including "no meaningful difference".
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MetricComparison {
    /// The metric.
    pub metric: Metric,
    /// The estimate over the admitted population under `PASS`.
    pub pass: MetricEstimate,
    /// The estimate over the admitted population under `UNCERTAIN`.
    pub uncertain: MetricEstimate,
    /// `pass.estimate - uncertain.estimate`, absent when either estimate is absent.
    pub difference: Option<f64>,
    /// The standard error of the difference, combining the two independent jackknife estimates.
    pub difference_standard_error: Option<f64>,
}

/// The reduction factor measured across the metric set (`design 6.3`).
#[derive(Clone, Debug, PartialEq)]
pub struct ReductionEffectReport {
    /// One comparison per metric, always the full set, so no single number stands for the effect.
    pub comparisons: Vec<MetricComparison>,
    /// The number of realised returns in the `PASS` population.
    pub pass_sample: usize,
    /// The number of realised returns in the `UNCERTAIN` population.
    pub uncertain_sample: usize,
    /// The records excluded from the aggregate.
    pub exclusions: Exclusions,
}

/// One confidence band of a calibration (`design 6.3`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ConfidenceBand {
    /// The inclusive lower boundary.
    pub lower: f64,
    /// The exclusive upper boundary, except for the final band which includes `1.0`.
    pub upper: f64,
    /// The number of decisions that fell in the band.
    pub count: usize,
    /// The number of those decisions whose realised return was above zero.
    pub hits: usize,
    /// The realised hit rate of the band, absent for an empty band.
    pub hit_rate: Option<f64>,
}

/// The calibration of one (rating, horizon) group (`design 6.3`).
#[derive(Clone, Debug, PartialEq)]
pub struct CalibrationGroup {
    /// The research rating.
    pub rating: String,
    /// The horizon of the decisions.
    pub horizon: usize,
    /// The number of decisions in the group whose label was absent.
    pub unlabelled: usize,
    /// The number of decisions whose confidence fell outside the unit interval.
    pub out_of_range: usize,
    /// Every confidence band, with its boundaries, count and realised hit rate.
    pub bands: Vec<ConfidenceBand>,
}

/// The calibration of `confidence` against realised hit rates, by rating and horizon bucket
/// (`design 6.3`).
///
/// The output is the bucket boundaries, the realised hit rate in each bucket and its count. There
/// is no pooled hit rate: a single number would hide the shape the calibration exists to show.
#[derive(Clone, Debug, PartialEq)]
pub struct CalibrationReport {
    /// One group per (rating, horizon) present, each with its own bands.
    pub groups: Vec<CalibrationGroup>,
    /// The provenance of the labels the calibration was built over (`design 4 I13`).
    pub labels: LabelProvenance,
    /// The records excluded from the aggregate.
    pub exclusions: Exclusions,
}

/// The correlation of two scores over their common observations (`design 6.4`).
#[derive(Clone, Debug, PartialEq)]
pub struct ScoreCorrelation {
    /// The first score name, lexicographically smaller.
    pub left: String,
    /// The second score name.
    pub right: String,
    /// The Pearson correlation over the common observations, absent if undefined.
    pub correlation: Option<f64>,
    /// The number of common observations.
    pub observations: usize,
}

/// A cluster of scores at or above the redundancy threshold (`design 6.4`).
#[derive(Clone, Debug, PartialEq)]
pub struct ScoreCluster {
    /// The member score names, sorted.
    pub members: Vec<String>,
}

/// The correlation structure of the score set (`design 6.4`).
#[derive(Clone, Debug, PartialEq)]
pub struct RedundancyReport {
    /// The score names, sorted.
    pub scores: Vec<String>,
    /// Every pairwise correlation, in lexicographic pair order.
    pub correlations: Vec<ScoreCorrelation>,
    /// The clusters at or above [`REDUNDANCY_CORRELATION_THRESHOLD`].
    pub clusters: Vec<ScoreCluster>,
    /// The number of independent hypotheses: the cluster count.
    pub independent_hypotheses: usize,
    /// The records excluded from the aggregate.
    pub exclusions: Exclusions,
}

/// The minimum a model must provide to be measured for parameter recovery (`design D4`).
///
/// A fitted model is only worth configuring if its parameters can be recovered from data drawn at
/// a known parameter vector. This is the seam a caller's own model plugs into: it draws a dataset
/// from itself at a known `truth` under a `seed`, and refits a dataset to parameter estimates.
/// Both uses the platform needs are covered by these two methods, and nothing else is required of
/// a model, so a generator or a calibration can implement it without knowing about this report.
///
/// A model MUST return one [`ParameterEstimate`] per parameter of the truth vector it was drawn
/// at, in the same order. The report indexes the estimates positionally, so a model that returns a
/// different count violates the contract.
pub trait FitModel {
    /// The dataset the model draws and refits.
    type Dataset;

    /// Draws a dataset from the model at the known parameter vector `truth` under `seed`.
    ///
    /// The same `(truth, seed)` MUST produce the same dataset, so a report over a seed is
    /// reproducible.
    fn draw(&self, truth: &[f64], seed: u64) -> Self::Dataset;

    /// Refits `dataset`, returning one estimate per parameter of the drawn truth.
    ///
    /// An estimate may declare the interval the fit is prepared to stand behind; the report
    /// measures the fraction of those intervals that covered the truth. A parameter the fit cannot
    /// bound is returned with no interval rather than a fabricated one.
    fn fit(&self, dataset: &Self::Dataset) -> Vec<ParameterEstimate>;
}

/// One fitted parameter: the point estimate and the interval the fit declares for it
/// (`design D4`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParameterEstimate {
    /// The fitted point estimate.
    pub estimate: f64,
    /// The interval the fit declares for the parameter, absent when the fit declares none.
    pub interval: Option<(f64, f64)>,
}

/// The tolerance set a recovery check decides its verdicts against (`design D4`).
///
/// Every threshold is a declared convention rather than a hidden constant, so it is printed with
/// the run and stored in the fixture and a reader can disagree with the number. A parameter is
/// `identified` when it meets all three identification thresholds, `unidentified` when its RMSE is
/// beyond [`RecoveryTolerance::unidentified_rmse`], and `weakly_identified` otherwise.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RecoveryTolerance {
    /// The largest absolute bias at which a parameter is called identified.
    pub max_absolute_bias: f64,
    /// The largest RMSE at which a parameter is called identified.
    pub max_rmse: f64,
    /// The smallest interval coverage at which a parameter is called identified.
    pub min_coverage: f64,
    /// The RMSE beyond which a parameter is called unidentified rather than weakly identified.
    pub unidentified_rmse: f64,
}

/// The recovery verdict of one parameter (`design D4`).
///
/// The vocabulary is closed and the names are stable, so a verdict is a value a caller can match
/// on rather than a string. There is no verdict that means "not measured": a parameter that could
/// not be measured is `unidentified`, never omitted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RecoveryVerdict {
    /// The parameter is recoverable: its bias, RMSE and coverage all meet tolerance.
    Identified,
    /// The parameter is only partly recoverable: it misses at least one identification threshold
    /// but is not far enough outside to be called unidentified.
    WeaklyIdentified,
    /// The parameter is not recoverable: its RMSE is beyond the unidentified tolerance, or no
    /// repetition produced an estimate.
    Unidentified,
}

impl RecoveryVerdict {
    /// Every verdict of the vocabulary, in order.
    pub const ALL: [Self; 3] = [Self::Identified, Self::WeaklyIdentified, Self::Unidentified];

    /// Returns the stable lowercase name of the verdict.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Identified => "identified",
            Self::WeaklyIdentified => "weakly_identified",
            Self::Unidentified => "unidentified",
        }
    }
}

/// The recovery measurement of one parameter (`design D4`).
///
/// The truth and the error statistics are reported together with the verdict, and the verdict is a
/// required field rather than an `Option`, so no reader can take the numbers without the label
/// that says whether they are recoverable. The struct carries no fitted point estimate of its own:
/// bias is `mean(estimate) - truth` and RMSE is the root-mean-square of the same errors, so a
/// point estimate is never handed out on its own.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParameterRecovery {
    /// The truth the model was drawn at.
    pub truth: f64,
    /// The mean estimate minus the truth over the repetitions, absent when none produced an
    /// estimate.
    pub bias: Option<f64>,
    /// The root-mean-square error over the repetitions, absent when none produced an estimate.
    pub rmse: Option<f64>,
    /// The fraction of repetitions whose declared interval covered the truth, absent when there
    /// were no repetitions. A parameter whose intervals never covered the truth reports `0.0`,
    /// never an omission.
    pub coverage: Option<f64>,
    /// The verdict against the declared tolerance. Always present.
    pub verdict: RecoveryVerdict,
}

/// The declared check a recovery measurement is run against (`design D4`).
#[derive(Clone, Debug, PartialEq)]
pub struct RecoveryCheck {
    /// The known parameter vector the model is drawn at.
    pub truth: Vec<f64>,
    /// The number of seeded datasets drawn and refitted.
    pub repetitions: usize,
    /// The base seed; the seed of repetition `r` is derived from it deterministically.
    pub seed: u64,
    /// The tolerance set the verdicts are decided against.
    pub tolerance: RecoveryTolerance,
}

/// The result of a parameter-recovery check (`design D4`).
///
/// The report draws `repetitions` seeded datasets from the model at `truth`, refits each and
/// reports per parameter the bias, RMSE, interval coverage and verdict, together with the
/// tolerance they were decided against. A model whose parameters are not recoverable is labelled
/// `unidentified` rather than described with a point estimate alone.
#[derive(Clone, Debug, PartialEq)]
pub struct ParameterRecoveryReport {
    /// The number of seeded datasets drawn and refitted.
    pub repetitions: usize,
    /// The known parameter vector the model was drawn at.
    pub truth: Vec<f64>,
    /// The tolerance the verdicts were decided against, printed with the run.
    pub tolerance: RecoveryTolerance,
    /// One measurement per declared parameter, in the truth vector's order.
    pub parameters: Vec<ParameterRecovery>,
}

/// Selects the identifiable records an aggregate measures, counting the rest (`design 4 I12`).
///
/// When `regime` is supplied, a record whose regime label differs is not part of the conditioned
/// population at all and is not counted as an exclusion. A matching record whose producer identity
/// is unknown is excluded from the aggregate and counted, so no aggregate can pool it.
fn select<'a>(
    records: &'a [AdmittedDecision],
    regime: Option<&str>,
) -> (Vec<&'a AdmittedDecision>, usize) {
    let mut selected = Vec::new();
    let mut excluded = 0usize;
    for record in records {
        if let Some(regime) = regime
            && record.regime.as_deref() != Some(regime)
        {
            continue;
        }
        if record.producer.is_unknown() {
            excluded += 1;
            continue;
        }
        selected.push(record);
    }
    (selected, excluded)
}

/// Restricts the stream to the records admitted under `regime` (`design 6.4`).
///
/// The regime verdict is an input supplied by the caller; no regime engine is implemented here.
/// The returned stream composes with every measurement in this module, so the same measurements
/// can be conditioned on a verdict. A record with no regime label, or a different one, is not part
/// of the conditioned stream; the order of the input is preserved.
#[must_use]
pub fn restrict_to_regime(records: &[AdmittedDecision], regime: &str) -> Vec<AdmittedDecision> {
    records
        .iter()
        .filter(|record| record.regime.as_deref() == Some(regime))
        .cloned()
        .collect()
}

/// Measures experiment A: research signal quality (`design 6.1`, `6.2`).
///
/// Each record becomes a point-in-time panel row whose score features carry the record's
/// availability as their aperture and whose label is its forward return. [`Panel::new`] then runs
/// the existing structural check, so a record whose feature reads past its row timestamp cannot be
/// measured at all (`design 4 I4`), and the existing membership rule applies unchanged. The
/// information coefficient of each score is the cross-sectional rank correlation of its values
/// against the label per date, computed with the crate's own
/// [`cross_sectional_rank`](crate::operators::cross_sectional_rank) and
/// [`rolling_correlation`](crate::operators::rolling_correlation) rather than re-derived, with a
/// jackknife standard error per date and across dates.
///
/// A score observation that arrived without coverage is an absence: it is excluded from the
/// coefficient and reported in [`InformationCoefficient::coverage_absent`], never measured as a
/// neutral value (`design 4 I11`).
///
/// A record whose label carries no [`LabelDefinition`] is not scored: a coefficient over labels of
/// unknown provenance would not be interpretable. Such records are excluded from the panel and
/// their count is reported in [`SignalQualityReport::labels`], so a model trained on undefined
/// labels is reported as such rather than scored (`design 4 I13`).
///
/// # Errors
///
/// Returns a [`PanelError`] if a feature reads past its row timestamp or a row's membership
/// disagrees with the stored series.
pub fn signal_quality(
    membership: &MembershipSeries,
    records: &[AdmittedDecision],
) -> Result<SignalQualityReport, PanelError> {
    let (selected, excluded) = select(records, None);
    let (defined, undefined): (Vec<&AdmittedDecision>, Vec<&AdmittedDecision>) = selected
        .into_iter()
        .partition(|record| record.label_definition.is_some());
    let labels = label_provenance(&defined, undefined.len());

    let mut names: BTreeSet<String> = BTreeSet::new();
    let mut coverage_absent: BTreeMap<String, usize> = BTreeMap::new();
    let mut rows: Vec<PanelRow> = Vec::with_capacity(defined.len());

    for record in &defined {
        let mut row = PanelRow::new(record.instrument_id, record.ts_event)
            .with_member(membership.is_member_at(record.instrument_id, record.ts_event));

        for (name, observation) in &record.scores {
            names.insert(name.clone());
            match observation {
                ScoreObservation::WithCoverage { value, .. } => {
                    row = row.with_feature(name.clone(), *value, record.available_at);
                }
                ScoreObservation::CoverageAbsent => {
                    *coverage_absent.entry(name.clone()).or_default() += 1;
                }
            }
        }

        if let Some(label) = record.forward_return {
            row = row.with_label(label);
        }

        rows.push(row);
    }

    let panel = Panel::new(membership, rows)?;

    let mut by_date: BTreeMap<UnixNanos, Vec<&PanelRow>> = BTreeMap::new();
    for row in panel.rows() {
        by_date.entry(row.ts_event).or_default().push(row);
    }

    let mut scores: BTreeMap<String, InformationCoefficient> = names
        .iter()
        .map(|name| {
            (
                name.clone(),
                InformationCoefficient {
                    series: Vec::new(),
                    mean: None,
                    standard_error: None,
                    interval: None,
                    dates: 0,
                    observations: 0,
                    coverage_absent: coverage_absent.get(name).copied().unwrap_or_default(),
                    label_absent: 0,
                },
            )
        })
        .collect();

    for (ts_event, date_rows) in &by_date {
        for name in &names {
            let mut pairs: Vec<(f64, f64)> = Vec::new();
            let mut label_absent = 0usize;

            for row in date_rows {
                if let Some(feature) = row.features.get(name) {
                    match row.label {
                        Some(label) => pairs.push((feature.value, label)),
                        None => label_absent += 1,
                    }
                }
            }

            let ic = rank_correlation(&pairs);
            let standard_error = jackknife_standard_error(&pairs, rank_correlation);
            let coefficient = scores
                .get_mut(name)
                .expect("the score name was inserted above");

            coefficient.label_absent += label_absent;
            if ic.is_some() {
                coefficient.dates += 1;
            }
            coefficient.observations += pairs.len();
            coefficient.series.push(InformationCoefficientPoint {
                ts_event: *ts_event,
                ic,
                n: pairs.len(),
                standard_error,
            });
        }
    }

    for coefficient in scores.values_mut() {
        let ics: Vec<f64> = coefficient
            .series
            .iter()
            .filter_map(|point| point.ic)
            .collect();
        if ics.is_empty() {
            continue;
        }
        let mean = mean_of(&ics).expect("the series is not empty");
        let standard_error = jackknife_standard_error(&ics, |values| mean_of(values));
        coefficient.mean = Some(mean);
        coefficient.standard_error = standard_error;
        coefficient.interval = standard_error.map(|se| (mean - 1.96 * se, mean + 1.96 * se));
    }

    Ok(SignalQualityReport {
        scores,
        rows: panel.len(),
        labels,
        exclusions: Exclusions {
            unknown_producer: excluded,
        },
    })
}

/// Measures experiment B: bridge policy effect (`design 6.1`).
///
/// Reports, per resolved disposition, how many research decisions yielded an eligible signal the
/// policy could pass on. It is deliberately not a profit number: it measures the policy's own
/// yield, so a strong signal quality with a weak yield is a finding here rather than there.
#[must_use]
pub fn policy_effect(records: &[AdmittedDecision]) -> PolicyEffectReport {
    let (selected, excluded) = select(records, None);

    let mut buckets: BTreeMap<Disposition, (usize, usize)> = BTreeMap::new();
    let mut eligible = 0usize;
    for record in &selected {
        let entry = buckets.entry(record.disposition).or_default();
        entry.0 += 1;
        if record.eligible_signal.is_some() {
            entry.1 += 1;
            eligible += 1;
        }
    }

    let decisions = selected.len();
    let by_disposition = buckets
        .into_iter()
        .map(|(disposition, (decisions, eligible))| DispositionOutcome {
            disposition,
            decisions,
            eligible,
        })
        .collect();

    PolicyEffectReport {
        decisions,
        by_disposition,
        eligible,
        declined: decisions - eligible,
        eligibility_rate: ratio(eligible, decisions),
        exclusions: Exclusions {
            unknown_producer: excluded,
        },
    }
}

/// Measures experiment C: execution realization (`design 6.1`).
///
/// Reports an eligible signal against its target, its risk decision and its fill, so a fill
/// shortfall localises to construction, to the risk engine or to execution rather than to the
/// research decision.
#[must_use]
pub fn execution_realization(records: &[AdmittedDecision]) -> ExecutionRealizationReport {
    let (selected, excluded) = select(records, None);

    let mut eligible = 0usize;
    let mut targeted = 0usize;
    let mut risk_approved = 0usize;
    let mut risk_denied = 0usize;
    let mut filled = 0usize;
    let mut ratios: Vec<f64> = Vec::new();
    let mut returns: Vec<f64> = Vec::new();

    for record in &selected {
        if record.eligible_signal.is_none() {
            continue;
        }
        eligible += 1;

        let Some(realization) = &record.realization else {
            continue;
        };
        if realization.target.is_some() {
            targeted += 1;
        }
        match realization.risk {
            RiskDecision::Approved => risk_approved += 1,
            RiskDecision::Denied => risk_denied += 1,
        }
        if let Some(fill) = realization.fill {
            filled += 1;
            if let Some(target) = realization.target
                && target > 0.0
            {
                ratios.push(fill / target);
            }
        }
        if let Some(realized_return) = realization.realized_return {
            returns.push(realized_return);
        }
    }

    ExecutionRealizationReport {
        eligible,
        targeted,
        risk_approved,
        risk_denied,
        filled,
        fill_rate: ratio(filled, eligible),
        mean_fill_to_target: mean_of(&ratios),
        mean_realized_return: mean_of(&returns),
        exclusions: Exclusions {
            unknown_producer: excluded,
        },
    }
}

/// Measures the `UNCERTAIN` reduction factor across the metric set (`design 6.3`).
///
/// Expectancy, Sharpe, hit rate, maximum drawdown and conditional value at risk are reported for
/// the admitted population under `PASS` and under `UNCERTAIN`, with the difference between them,
/// and each estimate carries a nonparametric jackknife standard error. The report presumes no
/// direction: "no meaningful difference" is an admissible result and is visible as a difference of
/// zero.
///
/// The two populations are the decisions with the matching disposition that realized a return,
/// ordered by `(ts_event, instrument_id)` so the drawdown path and the CVaR tail are deterministic.
#[must_use]
pub fn reduction_effect(records: &[AdmittedDecision]) -> ReductionEffectReport {
    let (selected, excluded) = select(records, None);
    let pass = realized_returns(&selected, Disposition::Pass);
    let uncertain = realized_returns(&selected, Disposition::Uncertain);

    let comparisons = Metric::ALL
        .iter()
        .map(|&metric| {
            let pass = estimate(metric, &pass);
            let uncertain = estimate(metric, &uncertain);
            let difference = match (pass.estimate, uncertain.estimate) {
                (Some(pass), Some(uncertain)) => Some(pass - uncertain),
                _ => None,
            };
            let difference_standard_error = match (pass.standard_error, uncertain.standard_error) {
                (Some(pass), Some(uncertain)) => Some((pass * pass + uncertain * uncertain).sqrt()),
                _ => None,
            };
            MetricComparison {
                metric,
                pass,
                uncertain,
                difference,
                difference_standard_error,
            }
        })
        .collect();

    ReductionEffectReport {
        comparisons,
        pass_sample: pass.len(),
        uncertain_sample: uncertain.len(),
        exclusions: Exclusions {
            unknown_producer: excluded,
        },
    }
}

/// Calibrates `confidence` against realised hit rates by bucket (`design 6.3`).
///
/// The output is one group per (rating, horizon) pair, and within each group the band boundaries,
/// the realised hit rate in each band and its count. There is no pooled hit rate anywhere in the
/// report. A decision whose confidence falls outside the unit interval is counted as out of range
/// rather than clamped into a band, and a decision whose label is absent is counted as unlabelled.
///
/// A decision whose label carries no [`LabelDefinition`] is not calibrated: a realised hit rate
/// over labels of unknown provenance would not be interpretable. Such records are excluded from
/// the groups and their count is reported in [`CalibrationReport::labels`] (`design 4 I13`).
#[must_use]
pub fn confidence_calibration(records: &[AdmittedDecision]) -> CalibrationReport {
    let (selected, excluded) = select(records, None);
    let (defined, undefined): (Vec<&AdmittedDecision>, Vec<&AdmittedDecision>) = selected
        .into_iter()
        .partition(|record| record.label_definition.is_some());
    let labels = label_provenance(&defined, undefined.len());

    let mut groups: BTreeMap<(String, usize), CalibrationAccumulator> = BTreeMap::new();
    for record in &defined {
        let group = groups
            .entry((record.rating.clone(), record.horizon))
            .or_default();

        let Some(label) = record.forward_return else {
            group.unlabelled += 1;
            continue;
        };
        if !(0.0..=1.0).contains(&record.confidence) {
            group.out_of_range += 1;
            continue;
        }

        let index = confidence_band(record.confidence);
        group.counts[index] += 1;
        if label > 0.0 {
            group.hits[index] += 1;
        }
    }

    let groups = groups
        .into_iter()
        .map(|((rating, horizon), accumulator)| CalibrationGroup {
            rating,
            horizon,
            unlabelled: accumulator.unlabelled,
            out_of_range: accumulator.out_of_range,
            bands: (0..CONFIDENCE_BANDS)
                .map(|index| ConfidenceBand {
                    lower: index as f64 / CONFIDENCE_BANDS as f64,
                    upper: (index + 1) as f64 / CONFIDENCE_BANDS as f64,
                    count: accumulator.counts[index],
                    hits: accumulator.hits[index],
                    hit_rate: ratio(accumulator.hits[index], accumulator.counts[index]),
                })
                .collect(),
        })
        .collect();

    CalibrationReport {
        groups,
        labels,
        exclusions: Exclusions {
            unknown_producer: excluded,
        },
    }
}

/// Reports the correlation structure of the score set (`design 6.4`).
///
/// Nine scores invite nine tests, and a panel multiplies them by symbols and days. This reports
/// the pairwise correlation matrix over the common observations and clusters the scores at or
/// above [`REDUNDANCY_CORRELATION_THRESHOLD`], with the number of clusters as the count of
/// independent hypotheses really being tested.
#[must_use]
pub fn redundancy(records: &[AdmittedDecision]) -> RedundancyReport {
    let (selected, excluded) = select(records, None);

    let mut names: BTreeSet<String> = BTreeSet::new();
    for record in &selected {
        for name in record.scores.keys() {
            names.insert(name.clone());
        }
    }
    let names: Vec<String> = names.into_iter().collect();

    let mut ordered: Vec<&AdmittedDecision> = selected;
    ordered.sort_by(|a, b| {
        a.ts_event
            .cmp(&b.ts_event)
            .then(a.instrument_id.cmp(&b.instrument_id))
    });

    let columns: Vec<Vec<Option<f64>>> = names
        .iter()
        .map(|name| {
            ordered
                .iter()
                .map(|record| match record.scores.get(name) {
                    Some(ScoreObservation::WithCoverage { value, .. }) => Some(*value),
                    _ => None,
                })
                .collect()
        })
        .collect();

    let mut correlations: Vec<ScoreCorrelation> = Vec::new();
    let mut parents: Vec<usize> = (0..names.len()).collect();

    for left in 0..names.len() {
        for right in (left + 1)..names.len() {
            let mut lhs: Vec<f64> = Vec::new();
            let mut rhs: Vec<f64> = Vec::new();
            for (a, b) in columns[left].iter().zip(&columns[right]) {
                if let (Some(a), Some(b)) = (a, b) {
                    lhs.push(*a);
                    rhs.push(*b);
                }
            }

            let correlation = rolling_correlation(&lhs, &rhs);
            if correlation.is_some_and(|value| value.abs() >= REDUNDANCY_CORRELATION_THRESHOLD) {
                union(&mut parents, left, right);
            }
            correlations.push(ScoreCorrelation {
                left: names[left].clone(),
                right: names[right].clone(),
                correlation,
                observations: lhs.len(),
            });
        }
    }

    let mut clusters: BTreeMap<usize, Vec<String>> = BTreeMap::new();
    for (index, name) in names.iter().enumerate() {
        let root = find(&mut parents, index);
        clusters.entry(root).or_default().push(name.clone());
    }
    let mut clusters: Vec<ScoreCluster> = clusters
        .into_values()
        .map(|members| ScoreCluster { members })
        .collect();
    clusters.sort_by(|a, b| a.members.first().cmp(&b.members.first()));

    RedundancyReport {
        scores: names,
        correlations,
        independent_hypotheses: clusters.len(),
        clusters,
        exclusions: Exclusions {
            unknown_producer: excluded,
        },
    }
}

/// Measures whether a model's own parameters can be recovered (`design D4`).
///
/// Draws `check.repetitions` datasets from `model` at `check.truth`, each under a seed derived
/// from `check.seed`, refits each and reports per parameter the bias, RMSE, the fraction of
/// declared intervals that covered the truth, and a [`RecoveryVerdict`] against
/// `check.tolerance`. The measurement is a pure function of the model, the truth, the tolerance
/// and the seed: it reads no clock and holds no state, so the same seed produces the same report.
///
/// A parameter whose verdict is not [`RecoveryVerdict::Identified`] is still reported with its
/// numbers; the verdict is a required field and is never omitted, so a point estimate cannot be
/// read from the report without the label that says whether it is recoverable.
///
/// # Panics
///
/// Panics if a refit returns a number of estimates different from the length of `check.truth`.
/// That is a violation of the [`FitModel`] contract, not a data condition.
#[must_use]
pub fn parameter_recovery<M: FitModel>(
    model: &M,
    check: &RecoveryCheck,
) -> ParameterRecoveryReport {
    let count = check.truth.len();
    let mut estimates: Vec<Vec<f64>> = vec![Vec::with_capacity(check.repetitions); count];
    let mut covered = vec![0usize; count];

    for repetition in 0..check.repetitions {
        let seed = repetition_seed(check.seed, repetition);
        let dataset = model.draw(&check.truth, seed);
        let fitted = model.fit(&dataset);
        assert_eq!(
            fitted.len(),
            count,
            "the model returned {} estimates for {} declared parameters",
            fitted.len(),
            count,
        );

        for (index, estimate) in fitted.iter().enumerate() {
            estimates[index].push(estimate.estimate);
            if let Some((lower, upper)) = estimate.interval
                && lower <= check.truth[index]
                && check.truth[index] <= upper
            {
                covered[index] += 1;
            }
        }
    }

    let parameters = check
        .truth
        .iter()
        .zip(estimates.iter())
        .zip(covered.iter())
        .map(|((truth, values), covered)| {
            let errors: Vec<f64> = values.iter().map(|value| value - truth).collect();
            let bias = mean_of(&errors);
            let rmse = if errors.is_empty() {
                None
            } else {
                Some(
                    (errors.iter().map(|error| error * error).sum::<f64>() / errors.len() as f64)
                        .sqrt(),
                )
            };
            let coverage = ratio(*covered, check.repetitions);
            ParameterRecovery {
                truth: *truth,
                bias,
                rmse,
                coverage,
                verdict: recovery_verdict(bias, rmse, coverage, &check.tolerance),
            }
        })
        .collect();

    ParameterRecoveryReport {
        repetitions: check.repetitions,
        truth: check.truth.clone(),
        tolerance: check.tolerance,
        parameters,
    }
}

/// The accumulator of one (rating, horizon) calibration group.
#[derive(Default)]
struct CalibrationAccumulator {
    unlabelled: usize,
    out_of_range: usize,
    counts: [usize; CONFIDENCE_BANDS],
    hits: [usize; CONFIDENCE_BANDS],
}

/// Returns the index of the confidence band a value in the unit interval falls in.
fn confidence_band(confidence: f64) -> usize {
    let index = (confidence * CONFIDENCE_BANDS as f64) as usize;
    index.min(CONFIDENCE_BANDS - 1)
}

/// Returns the rank information coefficient of score-label pairs within one cross-section.
///
/// The scores and labels are ranked with the crate's own cross-sectional rank and correlated with
/// its own Pearson correlation, so the coefficient is the crate's definition rather than a second
/// one. Fewer than two pairs, or a zero denominator, is an explicit absence.
fn rank_correlation(pairs: &[(f64, f64)]) -> Option<f64> {
    if pairs.len() < 2 {
        return None;
    }
    let scores: Vec<f64> = pairs.iter().map(|pair| pair.0).collect();
    let labels: Vec<f64> = pairs.iter().map(|pair| pair.1).collect();
    let ranked_scores = cross_sectional_rank(&scores);
    let ranked_labels = cross_sectional_rank(&labels);
    rolling_correlation(&ranked_scores, &ranked_labels)
}

/// Returns the jackknife standard error of a statistic over a sample.
///
/// The estimate is the leave-one-out jackknife, so no distributional shape is assumed and the
/// result is deterministic. Fewer than two observations, or a leave-one-out statistic that is
/// undefined, is an explicit absence.
fn jackknife_standard_error<T: Clone>(
    values: &[T],
    statistic: impl Fn(&[T]) -> Option<f64>,
) -> Option<f64> {
    let n = values.len();
    if n < 2 {
        return None;
    }

    let mut estimates = Vec::with_capacity(n);
    for index in 0..n {
        let subset: Vec<T> = values
            .iter()
            .enumerate()
            .filter(|(other, _)| *other != index)
            .map(|(_, value)| value.clone())
            .collect();
        estimates.push(statistic(&subset)?);
    }

    let mean = mean_of(&estimates)?;
    let sum = estimates
        .iter()
        .map(|estimate| (estimate - mean).powi(2))
        .sum::<f64>();
    Some((((n - 1) as f64 / n as f64) * sum).sqrt())
}

/// Returns the realised returns of one disposition, ordered deterministically.
fn realized_returns(records: &[&AdmittedDecision], disposition: Disposition) -> Vec<f64> {
    let mut selected: Vec<&AdmittedDecision> = records
        .iter()
        .copied()
        .filter(|record| record.disposition == disposition)
        .collect();
    selected.sort_by(|a, b| {
        a.ts_event
            .cmp(&b.ts_event)
            .then(a.instrument_id.cmp(&b.instrument_id))
    });
    selected
        .into_iter()
        .filter_map(|record| record.realization.as_ref()?.realized_return)
        .collect()
}

/// Estimates one metric over a return series, with its jackknife standard error.
fn estimate(metric: Metric, returns: &[f64]) -> MetricEstimate {
    MetricEstimate {
        metric,
        sample: returns.len(),
        estimate: metric_value(metric, returns),
        standard_error: jackknife_standard_error(returns, |values| metric_value(metric, values)),
    }
}

/// Returns the value of one metric over a return series.
fn metric_value(metric: Metric, returns: &[f64]) -> Option<f64> {
    match metric {
        Metric::Expectancy => mean_of(returns),
        Metric::Sharpe => {
            let mean = mean_of(returns)?;
            let standard_deviation = rolling_std(returns)?;
            if standard_deviation > 0.0 {
                Some(mean / standard_deviation)
            } else {
                None
            }
        }
        Metric::HitRate => {
            if returns.is_empty() {
                None
            } else {
                let hits = returns.iter().filter(|value| **value > 0.0).count();
                Some(hits as f64 / returns.len() as f64)
            }
        }
        Metric::MaxDrawdown => max_drawdown(returns),
        Metric::Cvar => cvar(returns),
    }
}

/// Returns the deepest peak-to-trough decline of a compounded equity path.
fn max_drawdown(returns: &[f64]) -> Option<f64> {
    if returns.is_empty() {
        return None;
    }

    let mut equity = 1.0f64;
    let mut peak = 1.0f64;
    let mut worst = 0.0f64;
    for realized_return in returns {
        equity *= 1.0 + realized_return;
        if equity > peak {
            peak = equity;
        }
        let drawdown = equity / peak - 1.0;
        if drawdown < worst {
            worst = drawdown;
        }
    }
    Some(worst)
}

/// Returns the conditional value at risk: the mean of the worst tail of a return series.
fn cvar(returns: &[f64]) -> Option<f64> {
    if returns.is_empty() {
        return None;
    }

    let mut sorted = returns.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let count = ((returns.len() as f64) * CVAR_TAIL_FRACTION)
        .ceil()
        .max(1.0) as usize;
    let tail = &sorted[..count.min(sorted.len())];
    mean_of(tail)
}

/// Returns the arithmetic mean of a slice, or an explicit absence for an empty slice.
fn mean_of(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    Some(values.iter().sum::<f64>() / values.len() as f64)
}

/// Returns `numerator / denominator`, or an explicit absence when the denominator is zero.
fn ratio(numerator: usize, denominator: usize) -> Option<f64> {
    if denominator == 0 {
        None
    } else {
        Some(numerator as f64 / denominator as f64)
    }
}

/// Returns the verdict of one parameter against a tolerance set (`design D4`).
///
/// A parameter with no formable numbers is unidentified rather than weakly identified: nothing was
/// recovered, so nothing can be trusted. The identification test requires all three thresholds,
/// and the unidentified test is checked first so a parameter whose RMSE is beyond the unidentified
/// tolerance can never be labelled identified by a wide interval.
fn recovery_verdict(
    bias: Option<f64>,
    rmse: Option<f64>,
    coverage: Option<f64>,
    tolerance: &RecoveryTolerance,
) -> RecoveryVerdict {
    let (Some(bias), Some(rmse), Some(coverage)) = (bias, rmse, coverage) else {
        return RecoveryVerdict::Unidentified;
    };
    let unidentified = !rmse.is_finite() || rmse > tolerance.unidentified_rmse;
    if unidentified {
        RecoveryVerdict::Unidentified
    } else if bias.abs() <= tolerance.max_absolute_bias
        && rmse <= tolerance.max_rmse
        && coverage >= tolerance.min_coverage
    {
        RecoveryVerdict::Identified
    } else {
        RecoveryVerdict::WeaklyIdentified
    }
}

/// Derives the seed of one repetition from the check's base seed (`design D4`).
///
/// The derivation is a SplitMix64 mix, so consecutive repetitions receive well-separated seeds even
/// when a caller's own generator consumes them linearly. It is arithmetic on the seed alone: no
/// dependency is added and no clock is read.
fn repetition_seed(base: u64, repetition: usize) -> u64 {
    splitmix64(base ^ (repetition as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15))
}

/// Returns the SplitMix64 mix of a 64-bit value.
fn splitmix64(value: u64) -> u64 {
    let mut mixed = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    mixed ^ (mixed >> 31)
}

/// Returns the representative root of a disjoint-set element, with path halving.
fn find(parents: &mut [usize], mut index: usize) -> usize {
    while parents[index] != index {
        parents[index] = parents[parents[index]];
        index = parents[index];
    }
    index
}

/// Unions two disjoint-set elements.
fn union(parents: &mut [usize], left: usize, right: usize) {
    let left = find(parents, left);
    let right = find(parents, right);
    if left != right {
        parents[right] = left;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instrument(value: &str) -> InstrumentId {
        InstrumentId::from(value)
    }

    fn membership(instruments: &[&str]) -> MembershipSeries {
        let mut series = MembershipSeries::new("universe", "test");
        for value in instruments {
            series.push(crate::membership::MembershipInterval::entry(
                "universe".to_string(),
                "test".to_string(),
                instrument(value),
                UnixNanos::from(0),
                None,
            ));
        }
        series
    }

    fn label_definition() -> LabelDefinition {
        LabelDefinition::new(
            "forward_return_poll",
            ProducerIdentity::Identified("research".to_string()),
            Some(UnixNanos::from(60_000_000_000)),
            5,
        )
        .expect("the test definition is valid")
    }

    fn decision(instrument_id: &str, ts_event: u64, available_at: u64) -> AdmittedDecision {
        AdmittedDecision {
            producer: ProducerIdentity::Identified("research".to_string()),
            instrument_id: instrument(instrument_id),
            ts_event: UnixNanos::from(ts_event),
            available_at: UnixNanos::from(available_at),
            regime: None,
            rating: "BUY".to_string(),
            confidence: 0.5,
            horizon: 5,
            disposition: Disposition::Pass,
            scores: BTreeMap::new(),
            forward_return: None,
            label_definition: Some(label_definition()),
            eligible_signal: None,
            realization: None,
        }
    }

    fn scored(name: &str, value: f64) -> (String, ScoreObservation) {
        (
            name.to_string(),
            ScoreObservation::WithCoverage {
                value,
                coverage: 1.0,
            },
        )
    }

    fn missing(name: &str) -> (String, ScoreObservation) {
        (name.to_string(), ScoreObservation::CoverageAbsent)
    }

    trait InsertScore {
        fn insert_score(&mut self, pair: (String, ScoreObservation));
    }

    impl InsertScore for AdmittedDecision {
        fn insert_score(&mut self, (key, value): (String, ScoreObservation)) {
            self.scores.insert(key, value);
        }
    }

    fn approx(left: f64, right: f64) -> bool {
        (left - right).abs() < 1e-12
    }

    #[test]
    fn future_dated_feature_is_refused_at_panel_construction() {
        // The row is timestamped at its reference date, but the decision only became available
        // later; the existing Panel::check refuses the aperture rather than a convention here.
        let series = membership(&["A.X"]);
        let mut record = decision("A.X", 100, 200);
        record.insert_score(scored("alpha", 1.0));

        let result = signal_quality(&series, &[record]);

        assert!(matches!(
            result,
            Err(PanelError::Lookahead { ref feature, .. }) if feature.as_str() == "alpha"
        ));
    }

    #[test]
    fn information_coefficient_recovers_a_known_relationship() {
        let series = membership(&["A.X", "B.X", "C.X", "D.X", "E.X"]);
        let scores = [1.0, 2.0, 3.0, 4.0, 5.0];

        // Ranks 1..5 against label ranks [1, 2, 3, 5, 4]: Spearman = 1 - 6*2/(5*24) = 0.9.
        let known = [10.0, 20.0, 30.0, 50.0, 40.0];
        let mut records = Vec::new();
        for (index, instrument) in ["A.X", "B.X", "C.X", "D.X", "E.X"].iter().enumerate() {
            let mut record = decision(instrument, 100, 100);
            record.insert_score(scored("alpha", scores[index]));
            record.forward_return = Some(known[index]);
            records.push(record);
        }

        let report = signal_quality(&series, &records).unwrap();
        let coefficient = report.scores.get("alpha").unwrap();
        let point = coefficient.series[0];

        assert!(approx(point.ic.unwrap(), 0.9));
        assert_eq!(point.n, 5);
        assert!(point.standard_error.is_some());

        // The known relationship is recovered inside the coefficient's stated uncertainty.
        let standard_error = point.standard_error.unwrap();
        assert!((point.ic.unwrap() - 0.9).abs() <= 2.0 * standard_error);
        assert!(approx(coefficient.mean.unwrap(), 0.9));
    }

    #[test]
    fn information_coefficient_of_a_null_signal_is_zero() {
        let series = membership(&["A.X", "B.X", "C.X", "D.X", "E.X"]);
        // Rank vector [5, 1, 2, 3, 4] against [1, 2, 3, 4, 5]: Spearman = 0.0 exactly.
        let scores = [1.0, 2.0, 3.0, 4.0, 5.0];
        let labels = [50.0, 10.0, 20.0, 30.0, 40.0];

        let mut records = Vec::new();
        for (index, instrument) in ["A.X", "B.X", "C.X", "D.X", "E.X"].iter().enumerate() {
            let mut record = decision(instrument, 100, 100);
            record.insert_score(scored("alpha", scores[index]));
            record.forward_return = Some(labels[index]);
            records.push(record);
        }

        let report = signal_quality(&series, &records).unwrap();
        assert!(approx(report.scores["alpha"].series[0].ic.unwrap(), 0.0));
    }

    #[test]
    fn absent_coverage_is_reported_as_absent_not_neutral() {
        let series = membership(&["A.X", "B.X", "C.X"]);
        let mut records = Vec::new();
        for (index, instrument) in ["A.X", "B.X", "C.X"].iter().enumerate() {
            let mut record = decision(instrument, 100, 100);
            record.forward_return = Some(if index == 0 { 0.1 } else { -0.1 });
            if index == 0 {
                record.insert_score(scored("composite", 1.0));
            } else {
                record.insert_score(missing("composite"));
            }
            records.push(record);
        }

        let report = signal_quality(&series, &records).unwrap();
        let coefficient = report.scores.get("composite").unwrap();

        assert_eq!(coefficient.coverage_absent, 2);
        assert_eq!(coefficient.observations, 1);
        assert_eq!(coefficient.dates, 0);
        assert_eq!(coefficient.mean, None);
        assert_eq!(coefficient.series[0].ic, None);
        assert!(!report.scores.is_empty());
    }

    #[test]
    fn the_three_experiments_report_separately_on_one_stream() {
        let series = membership(&["A.X", "B.X", "C.X", "D.X", "E.X"]);
        let scores = [1.0, 2.0, 3.0, 4.0, 5.0];
        let labels = [10.0, 20.0, 30.0, 40.0, 50.0];

        let mut records = Vec::new();
        for (index, instrument) in ["A.X", "B.X", "C.X", "D.X", "E.X"].iter().enumerate() {
            let mut record = decision(instrument, 100, 100);
            record.insert_score(scored("alpha", scores[index]));
            record.forward_return = Some(labels[index]);
            record.eligible_signal = Some(EligibleSignal {
                direction: SignalDirection::Long,
                strength: 0.5,
            });
            record.realization = Some(Realization {
                target: Some(0.05),
                risk: if index == 4 {
                    RiskDecision::Denied
                } else {
                    RiskDecision::Approved
                },
                fill: if index == 4 { None } else { Some(0.04) },
                realized_return: if index == 4 { None } else { Some(0.01) },
            });
            records.push(record);
        }

        let quality = signal_quality(&series, &records).unwrap();
        let policy = policy_effect(&records);
        let execution = execution_realization(&records);

        // Each experiment reports its own population, not a pooled one.
        assert_eq!(quality.rows, 5);
        assert_eq!(quality.scores["alpha"].observations, 5);
        assert_eq!(quality.scores["alpha"].coverage_absent, 0);
        assert_eq!(policy.decisions, 5);
        assert_eq!(policy.eligible, 5);
        assert_eq!(policy.declined, 0);
        assert_eq!(execution.eligible, 5);
        assert_eq!(execution.targeted, 5);
        assert_eq!(execution.risk_approved, 4);
        assert_eq!(execution.risk_denied, 1);
        assert_eq!(execution.filled, 4);
    }

    #[test]
    fn reduction_effect_reports_the_metric_set_with_uncertainty() {
        let mut records = Vec::new();
        for (index, value) in [0.02, 0.01, -0.005, 0.03].iter().enumerate() {
            let mut record = decision("A.X", 100 + index as u64, 100 + index as u64);
            record.disposition = Disposition::Pass;
            record.eligible_signal = Some(EligibleSignal {
                direction: SignalDirection::Long,
                strength: 0.5,
            });
            record.realization = Some(Realization {
                target: Some(0.05),
                risk: RiskDecision::Approved,
                fill: Some(0.05),
                realized_return: Some(*value),
            });
            records.push(record);
        }
        for (index, value) in [0.005, -0.004, 0.002, -0.001].iter().enumerate() {
            let mut record = decision("A.X", 100 + index as u64, 100 + index as u64);
            record.disposition = Disposition::Uncertain;
            record.eligible_signal = Some(EligibleSignal {
                direction: SignalDirection::Long,
                strength: 0.25,
            });
            record.realization = Some(Realization {
                target: Some(0.025),
                risk: RiskDecision::Approved,
                fill: Some(0.025),
                realized_return: Some(*value),
            });
            records.push(record);
        }

        let report = reduction_effect(&records);

        assert_eq!(report.pass_sample, 4);
        assert_eq!(report.uncertain_sample, 4);
        assert_eq!(report.comparisons.len(), Metric::ALL.len());

        for comparison in &report.comparisons {
            assert!(
                comparison.pass.estimate.is_some(),
                "{:?}",
                comparison.metric
            );
            assert!(comparison.uncertain.estimate.is_some());
            assert!(comparison.difference.is_some());
            assert!(comparison.difference_standard_error.is_some());
        }

        let expectancy = report
            .comparisons
            .iter()
            .find(|comparison| comparison.metric == Metric::Expectancy)
            .unwrap();
        let expected = 0.01375 - 0.0005;
        assert!(approx(expectancy.difference.unwrap(), expected));

        let hit_rate = report
            .comparisons
            .iter()
            .find(|comparison| comparison.metric == Metric::HitRate)
            .unwrap();
        assert!(approx(hit_rate.pass.estimate.unwrap(), 0.75));
        assert!(approx(hit_rate.uncertain.estimate.unwrap(), 0.5));
    }

    #[test]
    fn reduction_effect_allows_no_meaningful_difference() {
        let returns = [0.02, -0.01, 0.03, -0.02];
        let mut records = Vec::new();
        for disposition in [Disposition::Pass, Disposition::Uncertain] {
            for (index, value) in returns.iter().enumerate() {
                let mut record = decision("A.X", 100 + index as u64, 100 + index as u64);
                record.disposition = disposition;
                record.eligible_signal = Some(EligibleSignal {
                    direction: SignalDirection::Long,
                    strength: 0.5,
                });
                record.realization = Some(Realization {
                    target: Some(0.05),
                    risk: RiskDecision::Approved,
                    fill: Some(0.05),
                    realized_return: Some(*value),
                });
                records.push(record);
            }
        }

        let report = reduction_effect(&records);

        for comparison in &report.comparisons {
            let difference = comparison.difference.unwrap();
            assert!(approx(difference, 0.0), "{:?}", comparison.metric);
        }
    }

    #[test]
    fn calibration_is_per_rating_and_horizon_bucket_never_pooled() {
        let mut records = Vec::new();
        for (index, confidence) in [0.05, 0.15, 0.95].iter().enumerate() {
            let mut record = decision("A.X", 100, 100);
            record.rating = "BUY".to_string();
            record.horizon = 5;
            record.confidence = *confidence;
            record.forward_return = Some(if index == 2 { -0.1 } else { 0.1 });
            records.push(record);
        }
        let mut other = decision("A.X", 100, 100);
        other.rating = "SELL".to_string();
        other.horizon = 10;
        other.confidence = 0.55;
        other.forward_return = Some(-0.1);
        records.push(other);

        let report = confidence_calibration(&records);

        assert_eq!(report.groups.len(), 2);

        let buy = report
            .groups
            .iter()
            .find(|group| group.rating == "BUY" && group.horizon == 5)
            .unwrap();
        assert_eq!(buy.bands.len(), CONFIDENCE_BANDS);
        assert!(approx(buy.bands[0].lower, 0.0));
        assert!(approx(buy.bands[0].upper, 0.1));
        assert!(approx(buy.bands[9].lower, 0.9));
        assert!(approx(buy.bands[9].upper, 1.0));
        assert_eq!(buy.bands[0].count, 1);
        assert_eq!(buy.bands[0].hit_rate, Some(1.0));
        assert_eq!(buy.bands[1].count, 1);
        assert_eq!(buy.bands[2].count, 0);
        assert_eq!(buy.bands[2].hit_rate, None);
        assert_eq!(buy.bands[9].count, 1);
        assert_eq!(buy.bands[9].hit_rate, Some(0.0));

        let sell = report
            .groups
            .iter()
            .find(|group| group.rating == "SELL" && group.horizon == 10)
            .unwrap();
        assert_eq!(sell.bands[5].count, 1);
        assert_eq!(sell.bands[5].hit_rate, Some(0.0));
    }

    #[test]
    fn redundancy_clusters_correlated_scores() {
        let alpha = [1.0, 2.0, 3.0, 4.0];
        let beta = [1.0, 2.0, 3.0, 4.0];
        let gamma = [1.0, -1.0, 1.0, -1.0];

        let mut records = Vec::new();
        for index in 0..4 {
            let mut record = decision("A.X", 100 + index as u64, 100 + index as u64);
            record.insert_score(scored("a", alpha[index]));
            record.insert_score(scored("b", beta[index]));
            record.insert_score(scored("c", gamma[index]));
            records.push(record);
        }

        let report = redundancy(&records);

        assert_eq!(report.scores, vec!["a", "b", "c"]);
        assert_eq!(report.correlations.len(), 3);

        let ab = report
            .correlations
            .iter()
            .find(|correlation| correlation.left == "a" && correlation.right == "b")
            .unwrap();
        assert!(approx(ab.correlation.unwrap(), 1.0));
        assert_eq!(ab.observations, 4);

        let ac = report
            .correlations
            .iter()
            .find(|correlation| correlation.left == "a" && correlation.right == "c")
            .unwrap();
        assert!(ac.correlation.unwrap().abs() < REDUNDANCY_CORRELATION_THRESHOLD);

        assert_eq!(report.independent_hypotheses, 2);
        assert_eq!(report.clusters[0].members, vec!["a", "b"]);
        assert_eq!(report.clusters[1].members, vec!["c"]);
    }

    #[test]
    fn regime_conditioning_restricts_the_stream() {
        let series = membership(&["A.X", "B.X", "C.X", "D.X", "E.X"]);
        let scores = [1.0, 2.0, 3.0, 4.0, 5.0];
        let trend_labels = [10.0, 20.0, 30.0, 40.0, 50.0];
        let range_labels = [50.0, 40.0, 30.0, 20.0, 10.0];

        let mut records = Vec::new();
        for (index, instrument) in ["A.X", "B.X", "C.X", "D.X", "E.X"].iter().enumerate() {
            let mut trend = decision(instrument, 100, 100);
            trend.regime = Some("trend".to_string());
            trend.insert_score(scored("alpha", scores[index]));
            trend.forward_return = Some(trend_labels[index]);
            records.push(trend);

            let mut range = decision(instrument, 100, 100);
            range.regime = Some("range".to_string());
            range.insert_score(scored("alpha", scores[index]));
            range.forward_return = Some(range_labels[index]);
            records.push(range);
        }

        let trend = signal_quality(&series, &restrict_to_regime(&records, "trend")).unwrap();
        let range = signal_quality(&series, &restrict_to_regime(&records, "range")).unwrap();

        assert!(approx(trend.scores["alpha"].mean.unwrap(), 1.0));
        assert!(approx(range.scores["alpha"].mean.unwrap(), -1.0));
        assert_eq!(trend.rows, 5);
        assert_eq!(range.rows, 5);
    }

    #[test]
    fn unknown_producer_record_appears_in_no_aggregate() {
        let series = membership(&["A.X", "B.X", "C.X", "D.X", "E.X"]);
        let scores = [1.0, 2.0, 3.0, 4.0, 5.0];

        let mut records = Vec::new();
        for (index, instrument) in ["A.X", "B.X", "C.X", "D.X", "E.X"].iter().enumerate() {
            let mut record = decision(instrument, 100, 100);
            record.insert_score(scored("alpha", scores[index]));
            record.forward_return = Some(0.01);
            record.eligible_signal = Some(EligibleSignal {
                direction: SignalDirection::Long,
                strength: 0.5,
            });
            record.realization = Some(Realization {
                target: Some(0.05),
                risk: RiskDecision::Approved,
                fill: Some(0.05),
                realized_return: Some(0.01),
            });
            records.push(record);
        }

        // An admitted record with an unknown producer and an extreme value.
        let mut unknown = decision("A.X", 100, 100);
        unknown.producer = ProducerIdentity::Unknown;
        unknown.insert_score(scored("alpha", 1_000.0));
        unknown.forward_return = Some(100.0);
        unknown.eligible_signal = Some(EligibleSignal {
            direction: SignalDirection::Long,
            strength: 1.0,
        });
        unknown.realization = Some(Realization {
            target: Some(1.0),
            risk: RiskDecision::Approved,
            fill: Some(1.0),
            realized_return: Some(100.0),
        });
        records.push(unknown);

        let quality = signal_quality(&series, &records).unwrap();
        let policy = policy_effect(&records);
        let execution = execution_realization(&records);
        let reduction = reduction_effect(&records);
        let calibration = confidence_calibration(&records);

        assert_eq!(quality.exclusions.unknown_producer, 1);
        assert_eq!(quality.rows, 5);
        assert_eq!(quality.scores["alpha"].observations, 5);
        assert_eq!(quality.scores["alpha"].coverage_absent, 0);

        assert_eq!(policy.exclusions.unknown_producer, 1);
        assert_eq!(policy.decisions, 5);
        assert_eq!(policy.eligible, 5);

        assert_eq!(execution.exclusions.unknown_producer, 1);
        assert_eq!(execution.eligible, 5);
        assert_eq!(execution.filled, 5);
        assert!(approx(execution.mean_realized_return.unwrap(), 0.01));

        assert_eq!(reduction.exclusions.unknown_producer, 1);
        assert_eq!(reduction.pass_sample, 5);
        let expectancy = reduction
            .comparisons
            .iter()
            .find(|comparison| comparison.metric == Metric::Expectancy)
            .unwrap();
        assert!(approx(expectancy.pass.estimate.unwrap(), 0.01));

        assert_eq!(calibration.exclusions.unknown_producer, 1);
        let grouped: usize = calibration
            .groups
            .iter()
            .map(|group| group.bands.iter().map(|band| band.count).sum::<usize>())
            .sum();
        assert_eq!(grouped, 5);
    }

    #[test]
    fn aggregates_are_deterministic() {
        let series = membership(&["A.X", "B.X", "C.X", "D.X", "E.X"]);
        let mut records = Vec::new();
        for (index, instrument) in ["A.X", "B.X", "C.X", "D.X", "E.X"].iter().enumerate() {
            let mut record = decision(instrument, 100, 100);
            record.insert_score(scored("alpha", index as f64));
            record.forward_return = Some(index as f64);
            record.eligible_signal = Some(EligibleSignal {
                direction: SignalDirection::Long,
                strength: 0.5,
            });
            record.realization = Some(Realization {
                target: Some(0.05),
                risk: RiskDecision::Approved,
                fill: Some(0.05),
                realized_return: Some(0.01 * index as f64),
            });
            records.push(record);
        }

        assert_eq!(
            signal_quality(&series, &records).unwrap(),
            signal_quality(&series, &records).unwrap()
        );
        assert_eq!(policy_effect(&records), policy_effect(&records));
        assert_eq!(
            execution_realization(&records),
            execution_realization(&records)
        );
        assert_eq!(reduction_effect(&records), reduction_effect(&records));
        assert_eq!(
            confidence_calibration(&records),
            confidence_calibration(&records)
        );
        assert_eq!(redundancy(&records), redundancy(&records));
    }

    #[test]
    fn labels_with_an_unknown_definition_are_counted_rather_than_scored() {
        let series = membership(&["A.X", "B.X", "C.X", "D.X"]);
        let mut records = Vec::new();
        for (index, instrument) in ["A.X", "B.X", "C.X", "D.X"].iter().enumerate() {
            let mut record = decision(instrument, 100, 100);
            record.insert_score(scored("alpha", index as f64));
            record.forward_return = Some(index as f64);
            record.label_definition = None;
            records.push(record);
        }

        let report = signal_quality(&series, &records).unwrap();

        assert_eq!(report.labels.defined, 0);
        assert_eq!(report.labels.undefined, 4);
        assert_eq!(report.labels.undefined_share(), Some(1.0));
        assert_eq!(report.labels.definition, None);
        assert!(report.scores.is_empty());
        assert_eq!(report.rows, 0);

        let calibration = confidence_calibration(&records);
        assert_eq!(calibration.labels.undefined, 4);
        assert!(calibration.groups.is_empty());
    }

    /// A deterministic SplitMix64 generator for the fixture models, so a fixture's noise is a
    /// function of the seed alone.
    struct FixtureRng(u64);

    impl FixtureRng {
        fn new(seed: u64) -> Self {
            Self(seed)
        }

        fn next_u64(&mut self) -> u64 {
            self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut mixed = self.0;
            mixed = (mixed ^ (mixed >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            mixed = (mixed ^ (mixed >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            mixed ^ (mixed >> 31)
        }

        fn uniform(&mut self) -> f64 {
            (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
        }
    }

    /// The tolerance the fixtures declare. A parameter is identified when its bias is within 0.05,
    /// its RMSE within 0.05 and its coverage at or above 0.9; it is unidentified when its RMSE is
    /// beyond 0.25.
    fn recovery_tolerance() -> RecoveryTolerance {
        RecoveryTolerance {
            max_absolute_bias: 0.05,
            max_rmse: 0.05,
            min_coverage: 0.9,
            unidentified_rmse: 0.25,
        }
    }

    fn recovery_check(truth: &[f64], repetitions: usize, seed: u64) -> RecoveryCheck {
        RecoveryCheck {
            truth: truth.to_vec(),
            repetitions,
            seed,
            tolerance: recovery_tolerance(),
        }
    }

    /// Two parameters that enter separate terms, so each is recoverable from its own observation.
    struct Separable;

    impl FitModel for Separable {
        type Dataset = [f64; 2];

        fn draw(&self, truth: &[f64], seed: u64) -> [f64; 2] {
            let mut rng = FixtureRng::new(seed);
            [
                truth[0] + 0.02 * (rng.uniform() - 0.5),
                truth[1] + 0.02 * (rng.uniform() - 0.5),
            ]
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

    /// Two parameters that enter one term, so only their sum is observable and neither can be
    /// recovered on its own.
    struct Coupled;

    impl FitModel for Coupled {
        type Dataset = f64;

        fn draw(&self, truth: &[f64], seed: u64) -> f64 {
            let mut rng = FixtureRng::new(seed);
            truth.iter().sum::<f64>() + 0.02 * (rng.uniform() - 0.5)
        }

        fn fit(&self, dataset: &f64) -> Vec<ParameterEstimate> {
            // The sum is recoverable but the split between the two parameters is not: the fit
            // returns the symmetric split, with an interval far wider than the truth's separation.
            let half = *dataset / 2.0;
            vec![
                ParameterEstimate {
                    estimate: half,
                    interval: Some((half - 50.0, half + 50.0)),
                },
                ParameterEstimate {
                    estimate: half,
                    interval: Some((half - 50.0, half + 50.0)),
                },
            ]
        }
    }

    /// One parameter whose declared interval sits above the truth, so it never covers it.
    struct Shifted;

    impl FitModel for Shifted {
        type Dataset = f64;

        fn draw(&self, truth: &[f64], seed: u64) -> f64 {
            let mut rng = FixtureRng::new(seed);
            truth[0] + 0.02 * (rng.uniform() - 0.5)
        }

        fn fit(&self, dataset: &f64) -> Vec<ParameterEstimate> {
            vec![ParameterEstimate {
                estimate: *dataset,
                interval: Some((*dataset + 0.5, *dataset + 1.5)),
            }]
        }
    }

    #[test]
    fn separable_parameters_are_identified() {
        let report = parameter_recovery(&Separable, &recovery_check(&[1.0, 2.0], 64, 0x5EED));

        assert_eq!(report.repetitions, 64);
        assert_eq!(report.truth, vec![1.0, 2.0]);
        assert_eq!(report.parameters.len(), 2);

        for parameter in &report.parameters {
            let bias = parameter.bias.expect("an estimate was produced");
            let rmse = parameter.rmse.expect("an estimate was produced");
            assert!(bias.abs() <= recovery_tolerance().max_absolute_bias);
            assert!(rmse <= recovery_tolerance().max_rmse);
            assert_eq!(parameter.coverage, Some(1.0));
            assert_eq!(parameter.verdict, RecoveryVerdict::Identified);
        }
    }

    #[test]
    fn coupled_parameters_are_unidentified_with_intervals_wider_than_their_separation() {
        let report = parameter_recovery(&Coupled, &recovery_check(&[1.0, 2.0], 64, 0x5EED));
        let separation = (report.truth[1] - report.truth[0]).abs();

        // The interval the fit declares is far wider than the truth's separation: coverage of 1.0
        // here says the interval is uninformative, not that the parameter is recoverable.
        let fitted = Coupled.fit(&3.0);
        let (lower, upper) = fitted[0]
            .interval
            .expect("the fixture declares an interval");
        assert!(upper - lower > 10.0 * separation);

        for parameter in &report.parameters {
            let rmse = parameter.rmse.expect("an estimate was produced");
            assert!(rmse > recovery_tolerance().unidentified_rmse);
            assert_eq!(parameter.coverage, Some(1.0));
            assert_eq!(parameter.verdict, RecoveryVerdict::Unidentified);
        }
    }

    #[test]
    fn a_parameter_with_no_covering_interval_reports_zero_coverage() {
        let report = parameter_recovery(&Shifted, &recovery_check(&[1.0], 16, 0x5EED));
        let parameter = report.parameters[0];

        assert_eq!(parameter.coverage, Some(0.0));
        assert_eq!(parameter.verdict, RecoveryVerdict::WeaklyIdentified);
        assert!(parameter.bias.is_some());
        assert!(parameter.rmse.is_some());
    }

    #[test]
    fn parameter_recovery_is_deterministic_at_a_seed() {
        let first = parameter_recovery(&Separable, &recovery_check(&[1.0, 2.0], 32, 0x5EED));
        let second = parameter_recovery(&Separable, &recovery_check(&[1.0, 2.0], 32, 0x5EED));
        assert_eq!(first, second);

        // The seed reaches the model: a different seed draws different datasets.
        let other = parameter_recovery(&Separable, &recovery_check(&[1.0, 2.0], 32, 0x5EEE));
        assert_ne!(first, other);
    }

    #[test]
    fn a_parameter_that_was_not_measured_is_labelled_not_omitted() {
        let report = parameter_recovery(&Separable, &recovery_check(&[1.0, 2.0], 0, 0x5EED));

        assert_eq!(report.parameters.len(), 2);
        for parameter in &report.parameters {
            assert_eq!(parameter.bias, None);
            assert_eq!(parameter.rmse, None);
            assert_eq!(parameter.coverage, None);
            assert_eq!(parameter.verdict, RecoveryVerdict::Unidentified);
        }
    }

    #[test]
    fn recovery_verdict_names_are_stable() {
        assert_eq!(RecoveryVerdict::Identified.name(), "identified");
        assert_eq!(
            RecoveryVerdict::WeaklyIdentified.name(),
            "weakly_identified"
        );
        assert_eq!(RecoveryVerdict::Unidentified.name(), "unidentified");
        assert_eq!(RecoveryVerdict::ALL.len(), 3);
    }
}
