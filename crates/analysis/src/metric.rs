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

//! Metric identity, metadata and status.
//!
//! A [`PortfolioStatistic`](crate::statistic::PortfolioStatistic) declares, beside its
//! calculation, a [`MetricDefinition`]: a stable machine-facing [`id`](MetricDefinition::id), a
//! rendered [`title`](MetricDefinition::title) built from named parameters, and the declarative
//! metadata a report consumer needs - [`units`](MetricDefinition::units),
//! [`tags`](MetricDefinition::tags), [`direction`](MetricDefinition::direction) and the
//! [`inputs`](MetricDefinition::inputs) the definition requires.
//!
//! A calculation reports a [`MetricResult`], not an optional number: a status from the closed
//! [`MetricStatus`] vocabulary, plus a [`MetricReason`] code whenever the status is not
//! `Computed`. Three states are distinguished that a name-keyed `Option` map cannot distinguish:
//!
//! - `Unavailable` - an input the definition requires was absent.
//! - `Invalid` - the inputs were present and no meaningful value could be produced, for example
//!   a non-finite input or a zero denominator.
//! - `NotRegistered` - the metric is not in the analyzer's metric set at all.
//!
//! Collapsing `Invalid` into `Unavailable` hides a data defect behind an applicability rule, and
//! collapsing either into a dropped row is what a name-keyed map does by construction.
//!
//! Every vocabulary in this module is closed: it is an enum with an exhaustive string mapping,
//! so a reported value is checkable rather than a spelling competition.

use nautilus_core::capability::Capability;
use std::{collections::BTreeMap, fmt::Display};

/// The unit a metric value is expressed in.
///
/// The set is closed and deliberately small: a dimensionless metric is either bounded to the
/// unit interval (`Fraction`) or not (`Ratio`), and a metric with a denomination is money
/// (`Currency`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(
        eq,
        eq_int,
        frozen,
        hash,
        module = "nautilus_trader.analysis",
        from_py_object,
        rename_all = "SCREAMING_SNAKE_CASE",
    )
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass_enum(module = "nautilus_trader.analysis")
)]
pub enum MetricUnits {
    /// A dimensionless value that is not bounded to a unit interval, e.g. a Sharpe ratio.
    Ratio,
    /// A dimensionless value bounded to the unit interval, e.g. a drawdown or a win rate.
    Fraction,
    /// A money amount, e.g. an expectancy per trade.
    Currency,
}

impl MetricUnits {
    /// All variants of the closed vocabulary.
    pub const ALL: &'static [Self] = &[Self::Ratio, Self::Fraction, Self::Currency];

    /// Returns the stable string for this unit.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ratio => "ratio",
            Self::Fraction => "fraction",
            Self::Currency => "currency",
        }
    }
}

impl Display for MetricUnits {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A cross-cutting facet of a metric, used to select or group metrics in a report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(
        eq,
        eq_int,
        frozen,
        hash,
        module = "nautilus_trader.analysis",
        from_py_object,
        rename_all = "SCREAMING_SNAKE_CASE",
    )
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass_enum(module = "nautilus_trader.analysis")
)]
pub enum MetricTag {
    /// Describes the return series itself.
    Returns,
    /// A magnitude of risk, e.g. a volatility.
    Risk,
    /// A return per unit of risk.
    RiskAdjusted,
    /// A drawdown measure.
    Drawdown,
    /// A trade or PnL statistic.
    Trade,
    /// A position exposure or participation statistic.
    Exposure,
    /// Computed against a benchmark series.
    BenchmarkRelative,
    /// A distribution shape statistic, e.g. a skewness.
    Distribution,
    /// A tail-risk statistic.
    Tail,
    /// Depends on an annualisation period.
    Annualised,
}

impl MetricTag {
    /// All variants of the closed vocabulary.
    pub const ALL: &'static [Self] = &[
        Self::Returns,
        Self::Risk,
        Self::RiskAdjusted,
        Self::Drawdown,
        Self::Trade,
        Self::Exposure,
        Self::BenchmarkRelative,
        Self::Distribution,
        Self::Tail,
        Self::Annualised,
    ];

    /// Returns the stable string for this tag.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Returns => "returns",
            Self::Risk => "risk",
            Self::RiskAdjusted => "risk_adjusted",
            Self::Drawdown => "drawdown",
            Self::Trade => "trade",
            Self::Exposure => "exposure",
            Self::BenchmarkRelative => "benchmark_relative",
            Self::Distribution => "distribution",
            Self::Tail => "tail",
            Self::Annualised => "annualised",
        }
    }
}

impl Display for MetricTag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The direction in which a consumer rewards a metric value.
///
/// An action-oriented set rather than a sign: `Informational` states that no direction is
/// claimed, and a metric with a target value carries the target in its definition, so a
/// distance to a target is expressible without inventing a non-monotonic direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(
        eq,
        eq_int,
        frozen,
        hash,
        module = "nautilus_trader.analysis",
        from_py_object,
        rename_all = "SCREAMING_SNAKE_CASE",
    )
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass_enum(module = "nautilus_trader.analysis")
)]
pub enum MetricDirection {
    /// A larger reported value is preferred.
    Maximize,
    /// A smaller reported value is preferred.
    Minimize,
    /// The preferred value is the definition's target.
    Target,
    /// No direction is claimed.
    Informational,
}

impl MetricDirection {
    /// All variants of the closed vocabulary.
    pub const ALL: &'static [Self] = &[
        Self::Maximize,
        Self::Minimize,
        Self::Target,
        Self::Informational,
    ];

    /// Returns the stable string for this direction.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Maximize => "maximize",
            Self::Minimize => "minimize",
            Self::Target => "target",
            Self::Informational => "informational",
        }
    }
}

impl Display for MetricDirection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// An input a metric definition requires.
///
/// This is applicability, not a calculation path: it states which of the analyzer's data
/// sources the definition is defined over, so a report can report a metric as unavailable
/// rather than omitting it when the source it needs was not supplied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(
        eq,
        eq_int,
        frozen,
        hash,
        module = "nautilus_trader.analysis",
        from_py_object,
        rename_all = "SCREAMING_SNAKE_CASE",
    )
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass_enum(module = "nautilus_trader.analysis")
)]
pub enum MetricInput {
    /// A time-indexed returns series.
    Returns,
    /// A time-indexed benchmark returns series.
    Benchmark,
    /// A sequence of realized PnL values.
    RealizedPnls,
    /// A sequence of closed positions.
    Positions,
    /// A frame of performance periods.
    PerformancePeriods,
}

impl MetricInput {
    /// All variants of the closed vocabulary.
    pub const ALL: &'static [Self] = &[
        Self::Returns,
        Self::Benchmark,
        Self::RealizedPnls,
        Self::Positions,
        Self::PerformancePeriods,
    ];

    /// Returns the stable string for this input.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Returns => "returns",
            Self::Benchmark => "benchmark",
            Self::RealizedPnls => "realized_pnls",
            Self::Positions => "positions",
            Self::PerformancePeriods => "performance_periods",
        }
    }
}

impl Display for MetricInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The status of a metric in a report.
///
/// Four states, not three: `Unavailable` is an applicability judgement about an absent input,
/// and `Invalid` is a defect finding about a present one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(
        eq,
        eq_int,
        frozen,
        hash,
        module = "nautilus_trader.analysis",
        from_py_object,
        rename_all = "SCREAMING_SNAKE_CASE",
    )
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass_enum(module = "nautilus_trader.analysis")
)]
pub enum MetricStatus {
    /// A value was produced.
    Computed,
    /// An input the definition requires was absent.
    Unavailable,
    /// The inputs were present and no meaningful value could be produced.
    Invalid,
    /// The metric is not in the analyzer's metric set.
    NotRegistered,
}

impl MetricStatus {
    /// All variants of the closed vocabulary.
    pub const ALL: &'static [Self] = &[
        Self::Computed,
        Self::Unavailable,
        Self::Invalid,
        Self::NotRegistered,
    ];

    /// Returns the stable string for this status.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Computed => "computed",
            Self::Unavailable => "unavailable",
            Self::Invalid => "invalid",
            Self::NotRegistered => "not_registered",
        }
    }
}

impl Display for MetricStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The reason a metric is not `Computed`.
///
/// The set is closed and owned by this domain, so a report consumer can distinguish "not
/// applicable here" from "the data is wrong" without parsing free text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(
        eq,
        eq_int,
        frozen,
        hash,
        module = "nautilus_trader.analysis",
        from_py_object,
        rename_all = "SCREAMING_SNAKE_CASE",
    )
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass_enum(module = "nautilus_trader.analysis")
)]
pub enum MetricReason {
    /// The input series the definition requires was empty.
    InsufficientData,
    /// The definition is not defined over the supplied input source.
    UnsupportedInput,
    /// The definition requires a benchmark series and none was supplied.
    MissingBenchmark,
    /// An input value was not finite.
    NonFiniteInput,
    /// The inputs were finite and the computation had no defined value.
    UndefinedResult,
    /// The realized PnL currency could not be resolved.
    UnresolvedCurrency,
    /// The metric is not in the analyzer's metric set.
    NotInMetricSet,
}

impl MetricReason {
    /// All variants of the closed vocabulary.
    pub const ALL: &'static [Self] = &[
        Self::InsufficientData,
        Self::UnsupportedInput,
        Self::MissingBenchmark,
        Self::NonFiniteInput,
        Self::UndefinedResult,
        Self::UnresolvedCurrency,
        Self::NotInMetricSet,
    ];

    /// Returns the stable string for this reason.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InsufficientData => "insufficient_data",
            Self::UnsupportedInput => "unsupported_input",
            Self::MissingBenchmark => "missing_benchmark",
            Self::NonFiniteInput => "non_finite_input",
            Self::UndefinedResult => "undefined_result",
            Self::UnresolvedCurrency => "unresolved_currency",
            Self::NotInMetricSet => "not_in_metric_set",
        }
    }

    /// Returns the status this reason belongs to.
    #[must_use]
    pub const fn status(self) -> MetricStatus {
        match self {
            Self::InsufficientData | Self::UnsupportedInput | Self::MissingBenchmark => {
                MetricStatus::Unavailable
            }
            Self::UnresolvedCurrency => MetricStatus::Unavailable,
            Self::NonFiniteInput | Self::UndefinedResult => MetricStatus::Invalid,
            Self::NotInMetricSet => MetricStatus::NotRegistered,
        }
    }
}

impl Display for MetricReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The declarative definition of a metric.
///
/// `id` is stable and machine-facing and does not carry a parameter; `title_template` is
/// presentation and is rendered from `parameters` by [`Self::title`]. Renaming or translating a
/// title therefore does not touch a calculation, and changing a parameter does not change the
/// identity a result records.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.analysis", from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.analysis")
)]
pub struct MetricDefinition {
    id: String,
    title_template: String,
    parameters: BTreeMap<String, String>,
    units: MetricUnits,
    tags: Vec<MetricTag>,
    direction: MetricDirection,
    target: Option<f64>,
    inputs: Vec<MetricInput>,
    derived: bool,
}

impl MetricDefinition {
    /// Creates a new [`MetricDefinition`] instance.
    ///
    /// The title template may reference any declared parameter as `{name}`; a placeholder with
    /// no matching parameter is left verbatim so that a defect is visible in the rendered title
    /// rather than silently dropped.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        title_template: impl Into<String>,
        units: MetricUnits,
        direction: MetricDirection,
        inputs: impl AsRef<[MetricInput]>,
    ) -> Self {
        Self {
            id: id.into(),
            title_template: title_template.into(),
            parameters: BTreeMap::new(),
            units,
            tags: Vec::new(),
            direction,
            target: None,
            inputs: inputs.as_ref().to_vec(),
            derived: false,
        }
    }

    /// Marks the definition as derived rather than declared.
    ///
    /// A definition is derived when a wrapper could not read a declaration and supplied a value
    /// instead, which is the case for a statistic registered from Python that declares no metadata.
    /// The distinction is carried rather than hidden, so a report consumer can tell a declared
    /// assumption from an inferred one.
    #[must_use]
    pub fn as_derived(mut self) -> Self {
        self.derived = true;
        self
    }

    /// Sets the tags, replacing any already declared.
    #[must_use]
    pub fn with_tags(mut self, tags: impl AsRef<[MetricTag]>) -> Self {
        self.tags = tags.as_ref().to_vec();
        self
    }

    /// Sets the target value of a [`MetricDirection::Target`] definition.
    #[must_use]
    pub fn with_target(mut self, target: f64) -> Self {
        self.target = Some(target);
        self
    }

    /// Declares a named parameter, which the title template may render.
    #[must_use]
    pub fn with_parameter(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.parameters.insert(name.into(), value.into());
        self
    }

    /// Declares an integer parameter from a count.
    #[must_use]
    pub fn with_count(self, name: impl Into<String>, value: usize) -> Self {
        self.with_parameter(name, value.to_string())
    }

    /// Declares a floating point parameter rendered in its shortest round-trip form.
    #[must_use]
    pub fn with_number(self, name: impl Into<String>, value: f64) -> Self {
        self.with_parameter(name, format_number(value))
    }

    /// Returns the stable machine-facing identity of the metric.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the title template, with its parameters unrendered.
    #[must_use]
    pub fn title_template(&self) -> &str {
        &self.title_template
    }

    /// Returns the declared parameters.
    #[must_use]
    pub fn parameters(&self) -> &BTreeMap<String, String> {
        &self.parameters
    }

    /// Returns the units the value is expressed in.
    #[must_use]
    pub const fn units(&self) -> MetricUnits {
        self.units
    }

    /// Returns the tags declared for the metric.
    #[must_use]
    pub fn tags(&self) -> &[MetricTag] {
        &self.tags
    }

    /// Returns the direction a consumer rewards the metric in.
    #[must_use]
    pub const fn direction(&self) -> MetricDirection {
        self.direction
    }

    /// Returns the target value of a [`MetricDirection::Target`] definition, if any.
    #[must_use]
    pub const fn target(&self) -> Option<f64> {
        self.target
    }

    /// Returns the inputs the definition requires.
    #[must_use]
    pub fn inputs(&self) -> &[MetricInput] {
        &self.inputs
    }

    /// Returns whether the definition is declared or was derived by a wrapper.
    #[must_use]
    pub const fn is_derived(&self) -> bool {
        self.derived
    }

    /// Returns whether the definition is declared over `input`.
    #[must_use]
    pub fn is_defined_over(&self, input: MetricInput) -> bool {
        self.inputs.contains(&input)
    }

    /// Renders the title from the declared parameters.
    #[must_use]
    pub fn title(&self) -> String {
        render_title(&self.title_template, &self.parameters)
    }
}

/// A metric's outcome in a report.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.analysis", from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.analysis")
)]
pub struct MetricResult {
    id: String,
    title: String,
    status: MetricStatus,
    value: Option<f64>,
    reason: Option<MetricReason>,
}

impl MetricResult {
    /// Creates a result for a computed value.
    #[must_use]
    pub fn computed(id: String, title: String, value: f64) -> Self {
        Self {
            id,
            title,
            status: MetricStatus::Computed,
            value: Some(value),
            reason: None,
        }
    }

    /// Creates a result for a state other than `Computed`.
    ///
    /// # Panics
    ///
    /// Panics if `reason` is a [`MetricReason::NotInMetricSet`] with a status other than
    /// `NotRegistered`, or if any other reason is paired with `Computed` or `NotRegistered`:
    /// a reason and a status that disagree would make the four states ambiguous.
    #[must_use]
    pub fn not_computed(
        id: String,
        title: String,
        reason: MetricReason,
        status: MetricStatus,
    ) -> Self {
        assert_eq!(
            reason.status(),
            status,
            "reason {reason} does not belong to status {status}",
        );
        Self {
            id,
            title,
            status,
            value: None,
            reason: Some(reason),
        }
    }

    /// Returns the stable metric identity.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Returns the rendered title.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Returns the status.
    #[must_use]
    pub const fn status(&self) -> MetricStatus {
        self.status
    }

    /// Returns the value, present only when the status is `Computed`.
    #[must_use]
    pub const fn value(&self) -> Option<f64> {
        self.value
    }

    /// Returns the reason, present whenever the status is not `Computed`.
    #[must_use]
    pub const fn reason(&self) -> Option<MetricReason> {
        self.reason
    }
}

impl MetricResult {
    /// Returns the capability answer for this result.
    ///
    /// The answer is available when a value was computed, and otherwise carries the result's
    /// reason, which is this domain's closed set: the seven reason codes are what a caller may
    /// branch on. The detail names the metric and its status for a human and is never canonical,
    /// and the requirements are empty because a statistic is computed from what it was given rather
    /// than from a precondition that could be supplied.
    #[must_use]
    pub fn capability(&self) -> Capability {
        match self.reason() {
            None => Capability::available(),
            Some(reason) => Capability::unavailable(
                reason.as_str(),
                format!("{} is {}", self.title(), self.status().as_str()),
            ),
        }
    }
}

/// The outcome of every requested metric in a report.
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.analysis", from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.analysis")
)]
pub struct MetricReport {
    results: Vec<MetricResult>,
}

impl MetricReport {
    /// Creates a new [`MetricReport`] instance.
    #[must_use]
    pub fn new(results: Vec<MetricResult>) -> Self {
        Self { results }
    }

    /// Returns every result, in request order.
    #[must_use]
    pub fn results(&self) -> &[MetricResult] {
        &self.results
    }

    /// Returns the number of results.
    #[must_use]
    pub fn len(&self) -> usize {
        self.results.len()
    }

    /// Returns whether the report has no results.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.results.is_empty()
    }

    /// Returns the result for a metric identity, if requested.
    #[must_use]
    pub fn get(&self, id: &str) -> Option<&MetricResult> {
        self.results.iter().find(|result| result.id == id)
    }

    /// Returns every result with the given status.
    #[must_use]
    pub fn with_status(&self, status: MetricStatus) -> Vec<&MetricResult> {
        self.results
            .iter()
            .filter(|result| result.status == status)
            .collect()
    }
}

/// Formats a parameter in its shortest round-trip form, without a trailing `.0`.
#[must_use]
pub fn format_number(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        format!("{value}")
    }
}

/// Renders `template`, substituting `{name}` placeholders from `parameters`.
///
/// A placeholder with no matching parameter is left verbatim.
#[must_use]
fn render_title(template: &str, parameters: &BTreeMap<String, String>) -> String {
    let mut rendered = String::with_capacity(template.len());
    let mut rest = template;

    while let Some(start) = rest.find('{') {
        rendered.push_str(&rest[..start]);
        let after = &rest[start + 1..];

        let Some(end) = after.find('}') else {
            rendered.push_str(&rest[start..]);
            return rendered;
        };

        let name = &after[..end];

        if let Some(value) = parameters.get(name) {
            rendered.push_str(value);
        } else {
            rendered.push('{');
            rendered.push_str(name);
            rendered.push('}');
        }

        rest = &after[end + 1..];
    }

    rendered.push_str(rest);
    rendered
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    fn test_vocabularies_are_closed_and_stably_named() {
        // Exhaustive matches above are the closure guarantee; these assertions pin the strings
        // a report consumer sees, and that every variant is in ALL.
        assert_eq!(MetricUnits::ALL.len(), 3);
        for unit in MetricUnits::ALL {
            assert!(!unit.as_str().is_empty());
            assert_eq!(unit.to_string(), unit.as_str());
        }

        assert_eq!(MetricTag::ALL.len(), 10);
        for tag in MetricTag::ALL {
            assert!(!tag.as_str().is_empty());
            assert_eq!(tag.to_string(), tag.as_str());
        }

        assert_eq!(MetricDirection::ALL.len(), 4);
        for direction in MetricDirection::ALL {
            assert!(!direction.as_str().is_empty());
            assert_eq!(direction.to_string(), direction.as_str());
        }

        assert_eq!(MetricInput::ALL.len(), 5);
        for input in MetricInput::ALL {
            assert!(!input.as_str().is_empty());
            assert_eq!(input.to_string(), input.as_str());
        }

        assert_eq!(MetricStatus::ALL.len(), 4);
        for status in MetricStatus::ALL {
            assert!(!status.as_str().is_empty());
            assert_eq!(status.to_string(), status.as_str());
        }

        // Every reason belongs to exactly one status, and the mapping is total.
        assert_eq!(MetricReason::ALL.len(), 7);
        for reason in MetricReason::ALL {
            assert!(MetricStatus::ALL.contains(&reason.status()));
            assert!(!reason.as_str().is_empty());
        }
        assert_eq!(
            MetricReason::ALL
                .iter()
                .filter(|reason| reason.status() == MetricStatus::Computed)
                .count(),
            0,
        );
    }

    #[rstest]
    fn test_statuses_are_distinct_strings() {
        let mut names: Vec<&str> = MetricStatus::ALL.iter().map(|s| s.as_str()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), MetricStatus::ALL.len());

        let invalid = MetricStatus::Invalid.as_str();
        let unavailable = MetricStatus::Unavailable.as_str();
        assert_ne!(invalid, unavailable);
    }

    #[rstest]
    fn test_title_renders_from_parameters() {
        let definition = MetricDefinition::new(
            "sharpe_ratio",
            "Sharpe Ratio ({annualisation} days)",
            MetricUnits::Ratio,
            MetricDirection::Maximize,
            [MetricInput::Returns],
        )
        .with_count("annualisation", 252)
        .with_tags([MetricTag::RiskAdjusted, MetricTag::Annualised]);

        assert_eq!(definition.id(), "sharpe_ratio");
        assert_eq!(definition.title(), "Sharpe Ratio (252 days)");
        assert_eq!(
            definition.title_template(),
            "Sharpe Ratio ({annualisation} days)",
        );

        let other = MetricDefinition::new(
            "sharpe_ratio",
            definition.title_template(),
            MetricUnits::Ratio,
            MetricDirection::Maximize,
            [MetricInput::Returns],
        )
        .with_count("annualisation", 30);

        // The identity is stable across a parameter change; the title is not.
        assert_eq!(definition.id(), other.id());
        assert_eq!(other.title(), "Sharpe Ratio (30 days)");
        assert_ne!(definition.title(), other.title());
    }

    #[rstest]
    fn test_title_leaves_an_undeclared_placeholder_verbatim() {
        let definition = MetricDefinition::new(
            "custom",
            "Custom ({missing}) metric",
            MetricUnits::Ratio,
            MetricDirection::Informational,
            [MetricInput::Returns],
        );

        assert_eq!(definition.title(), "Custom ({missing}) metric");
    }

    #[rstest]
    fn test_format_number_avoids_a_trailing_zero_fraction() {
        assert_eq!(format_number(252.0), "252");
        assert_eq!(format_number(0.95), "0.95");
        assert_eq!(format_number(0.0), "0");
        assert_eq!(format_number(-0.0025), "-0.0025");
    }

    #[rstest]
    fn test_target_direction_carries_its_target() {
        let definition = MetricDefinition::new(
            "tracking_error",
            "Tracking Error ({annualisation} days)",
            MetricUnits::Fraction,
            MetricDirection::Target,
            [MetricInput::Returns, MetricInput::Benchmark],
        )
        .with_count("annualisation", 252)
        .with_target(0.0);

        assert_eq!(definition.direction(), MetricDirection::Target);
        assert_eq!(definition.target(), Some(0.0));
        assert!(definition.is_defined_over(MetricInput::Benchmark));
        assert!(!definition.is_defined_over(MetricInput::Positions));
    }

    #[rstest]
    fn test_report_helpers_address_results_by_identity() {
        let report = MetricReport::new(vec![
            MetricResult::computed("sharpe_ratio".into(), "Sharpe Ratio (252 days)".into(), 1.5),
            MetricResult::not_computed(
                "long_ratio".into(),
                "Long Ratio".into(),
                MetricReason::UnsupportedInput,
                MetricStatus::Unavailable,
            ),
            MetricResult::not_computed(
                "unknown".into(),
                "unknown".into(),
                MetricReason::NotInMetricSet,
                MetricStatus::NotRegistered,
            ),
        ]);

        assert_eq!(report.len(), 3);
        assert!(!report.is_empty());
        assert_eq!(report.get("sharpe_ratio").unwrap().value(), Some(1.5));
        assert_eq!(report.get("sharpe_ratio").unwrap().reason(), None);
        assert_eq!(report.get("long_ratio").unwrap().value(), None);
        assert_eq!(
            report.get("long_ratio").unwrap().reason(),
            Some(MetricReason::UnsupportedInput),
        );
        assert_eq!(report.with_status(MetricStatus::Unavailable).len(), 1);
        assert_eq!(report.with_status(MetricStatus::Invalid).len(), 0);
        assert!(report.get("absent").is_none());
    }

    #[rstest]
    fn test_a_definition_is_declared_unless_marked_derived() {
        let declared = MetricDefinition::new(
            "custom",
            "Custom",
            MetricUnits::Ratio,
            MetricDirection::Informational,
            [MetricInput::Returns],
        );

        assert!(!declared.is_derived());
        assert!(declared.as_derived().is_derived());
    }

    #[rstest]
    #[should_panic(expected = "does not belong to status")]
    fn test_a_reason_must_belong_to_its_status() {
        let _ = MetricResult::not_computed(
            "sharpe_ratio".into(),
            "Sharpe Ratio (252 days)".into(),
            MetricReason::InsufficientData,
            MetricStatus::Invalid,
        );
    }
}

#[cfg(test)]
mod capability_tests {
    use nautilus_core::capability::is_canonical_code;
    use rstest::rstest;

    use super::*;

    #[test]
    fn test_a_computed_result_is_available_and_carries_no_code() {
        let result = MetricResult::computed(
            "sharpe_ratio".to_string(),
            "Sharpe Ratio (252 days)".to_string(),
            1.5,
        );

        let capability = result.capability();

        assert!(capability.is_available());
        assert_eq!(capability.code(), None);
        assert!(capability.detail().is_empty());
    }

    #[rstest]
    #[case(
        MetricReason::UnsupportedInput,
        MetricStatus::Unavailable,
        "unsupported_input"
    )]
    #[case(
        MetricReason::NonFiniteInput,
        MetricStatus::Invalid,
        "non_finite_input"
    )]
    #[case(
        MetricReason::NotInMetricSet,
        MetricStatus::NotRegistered,
        "not_in_metric_set"
    )]
    fn test_a_result_that_was_not_computed_reports_its_reason_code(
        #[case] reason: MetricReason,
        #[case] status: MetricStatus,
        #[case] expected: &str,
    ) {
        let result = MetricResult::not_computed(
            "long_ratio".to_string(),
            "Long Ratio".to_string(),
            reason,
            status,
        );

        let capability = result.capability();

        assert!(!capability.is_available());
        assert_eq!(capability.code(), Some(expected));
        assert!(
            capability
                .to_string()
                .starts_with(&format!("{expected}: Long Ratio is "))
        );
        assert!(capability.requirements().is_empty());
    }

    #[test]
    fn test_every_metric_reason_in_the_closed_set_is_canonical() {
        for reason in MetricReason::ALL {
            let token = reason.as_str();

            assert!(is_canonical_code(token), "{token} is not canonical");
        }
    }
}
