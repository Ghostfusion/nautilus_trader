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

//! Feature definitions: a compiled expression tree over typed operators.
//!
//! A feature is a deterministic transformation of market or reference data, declared once and
//! reusable. It is written once in a plain-text surface and compiled once into a typed tree:
//!
//! ```text
//! rolling_mean(close, 20) / lag(close, 1)
//! ```
//!
//! The tree is the authority. There is no `eval` and no string is interpreted at runtime: the
//! parser rejects an unknown function, a wrong argument count, a non-integer window and a group
//! key that is not a column as typed errors, so a malformed definition cannot silently become
//! another definition. The compiled tree serializes canonically - independent of whitespace and of
//! the order in which a set of definitions is combined - and digests to a stable identifier
//! through [`Digest`], usable as a regression pin and inside an experiment digest.
//!
//! # Sides
//!
//! Every transform declares the side it is computed on. An [`Inference`](TransformSide::Inference)
//! transform may read only the current observation and earlier ones; a
//! [`Learning`](TransformSide::Learning) transform may be fit over the whole learning window. The
//! side is part of the definition's digest, and [`Feature::assert_side`] refuses to use a
//! definition on a side other than the one it declared, so an inference value cannot silently be
//! produced by a learning-side definition. A forward-reading operator such as
//! [`Lead`](crate::operators::Operator::Lead) is permitted only on the learning side and is
//! rejected at parse time on the inference side.
//!
//! # Boundaries
//!
//! There is one execution model. A signal strategy rebalances through the normal strategy and
//! execution path; a factor value is authoritative in the research pipeline and nowhere else.
//! This pipeline is a general research primitive: it does not absorb, and is not absorbed by, a
//! domain-specific score engine. A deterministic score and a learned factor may read the same data
//! infrastructure, but they are not one subsystem.
//!
//! # Point-in-time discipline
//!
//! Evaluation of a feature reads only the current and earlier observations of the columns it names
//! and the point-in-time universe its source reports. A panel that carries the values enforces the
//! same rule structurally: a feature value whose aperture reaches past its row timestamp is
//! rejected by [`PanelError::Lookahead`](crate::panel::PanelError::Lookahead), and a row whose
//! membership disagrees with the stored membership series is rejected by
//! [`PanelError::MembershipMismatch`](crate::panel::PanelError::MembershipMismatch). Every value
//! may be attributed through [`Provenance`].

use std::fmt;

use nautilus_core::UnixNanos;
use nautilus_model::identifiers::InstrumentId;
use serde_json::{Value, json};
use thiserror::Error;

use crate::dataset::{Digest, DigestInput, Feature as FeatureTrait};
use crate::operators::{self, Operator, TransformSide};

/// A compiled node of a feature expression.
#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    /// A constant.
    Literal(f64),
    /// A named data column, resolved per instrument by the source.
    Column(String),
    /// An operator applied to its arguments.
    Call {
        /// The operator.
        op: Operator,
        /// The arguments, in the order the operator expects them.
        args: Vec<Self>,
    },
}

impl Expr {
    /// Evaluates the node at `index` for one instrument.
    ///
    /// Returns an explicit absence ([`None`]) for a missing, out-of-range or non-finite input and
    /// for an undefined statistic; an undefined statistic is never reported as zero.
    #[must_use]
    pub fn evaluate(
        &self,
        source: &dyn ExprSource,
        instrument_id: InstrumentId,
        index: usize,
    ) -> Option<f64> {
        let value = match self {
            Self::Literal(value) => Some(*value),
            Self::Column(name) => source
                .column(instrument_id, name)
                .and_then(|series| series.get(index).copied()),
            Self::Call { op, args } => evaluate_call(op, args, source, instrument_id, index),
        };
        value.filter(|value| value.is_finite())
    }

    fn to_json(&self) -> Value {
        match self {
            Self::Literal(value) => json!({ "kind": "literal", "value": value }),
            Self::Column(name) => json!({ "kind": "column", "name": name }),
            Self::Call { op, args } => json!({
                "kind": "call",
                "operator": op.name(),
                "parameter": op.parameter(),
                "arguments": args.iter().map(Self::to_json).collect::<Vec<_>>(),
            }),
        }
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Literal(value) => write!(f, "{value}"),
            Self::Column(name) => f.write_str(name),
            Self::Call { op, args } => write_call(op, args, f),
        }
    }
}

fn write_call(op: &Operator, args: &[Expr], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    if let Some(symbol) = op.infix_symbol() {
        return write!(f, "({} {} {})", args[0], symbol, args[1]);
    }
    if *op == Operator::Neg {
        return write!(f, "(-{})", args[0]);
    }

    write!(f, "{}(", op.name())?;
    let mut first = true;
    for arg in args {
        if !first {
            f.write_str(", ")?;
        }
        write!(f, "{arg}")?;
        first = false;
    }
    if let Some(parameter) = op.parameter() {
        if !first {
            f.write_str(", ")?;
        }
        write!(f, "{parameter}")?;
    }
    f.write_str(")")
}

fn evaluate_call(
    op: &Operator,
    args: &[Expr],
    source: &dyn ExprSource,
    instrument_id: InstrumentId,
    index: usize,
) -> Option<f64> {
    match op {
        Operator::Neg => Some(-eval_arg(args, 0, source, instrument_id, index)?),
        Operator::Add => Some(
            eval_arg(args, 0, source, instrument_id, index)?
                + eval_arg(args, 1, source, instrument_id, index)?,
        ),
        Operator::Sub => Some(
            eval_arg(args, 0, source, instrument_id, index)?
                - eval_arg(args, 1, source, instrument_id, index)?,
        ),
        Operator::Mul => Some(
            eval_arg(args, 0, source, instrument_id, index)?
                * eval_arg(args, 1, source, instrument_id, index)?,
        ),
        Operator::Div => {
            // A zero divisor produces a non-finite value, which the caller filters to an absence.
            Some(
                eval_arg(args, 0, source, instrument_id, index)?
                    / eval_arg(args, 1, source, instrument_id, index)?,
            )
        }
        Operator::Less => Some(flag(
            eval_arg(args, 0, source, instrument_id, index)?
                < eval_arg(args, 1, source, instrument_id, index)?,
        )),
        Operator::LessOrEqual => Some(flag(
            eval_arg(args, 0, source, instrument_id, index)?
                <= eval_arg(args, 1, source, instrument_id, index)?,
        )),
        Operator::Greater => Some(flag(
            eval_arg(args, 0, source, instrument_id, index)?
                > eval_arg(args, 1, source, instrument_id, index)?,
        )),
        Operator::GreaterOrEqual => Some(flag(
            eval_arg(args, 0, source, instrument_id, index)?
                >= eval_arg(args, 1, source, instrument_id, index)?,
        )),
        Operator::Equal => Some(flag(
            eval_arg(args, 0, source, instrument_id, index)?
                == eval_arg(args, 1, source, instrument_id, index)?,
        )),
        Operator::NotEqual => Some(flag(
            eval_arg(args, 0, source, instrument_id, index)?
                != eval_arg(args, 1, source, instrument_id, index)?,
        )),
        Operator::Lag(periods) => {
            let target = index.checked_sub(*periods)?;
            eval_arg(args, 0, source, instrument_id, target)
        }
        Operator::Lead(periods) => {
            let target = index.checked_add(*periods)?;
            eval_arg(args, 0, source, instrument_id, target)
        }
        Operator::RollingMean(window) => operators::rolling_mean(&window_values(
            args,
            0,
            *window,
            source,
            instrument_id,
            index,
        )?),
        Operator::RollingStd(window) => operators::rolling_std(&window_values(
            args,
            0,
            *window,
            source,
            instrument_id,
            index,
        )?),
        Operator::RollingRank(window) => operators::rolling_rank(&window_values(
            args,
            0,
            *window,
            source,
            instrument_id,
            index,
        )?),
        Operator::RollingCorrelation(window) => {
            let lhs = window_values(args, 0, *window, source, instrument_id, index)?;
            let rhs = window_values(args, 1, *window, source, instrument_id, index)?;
            operators::rolling_correlation(&lhs, &rhs)
        }
        Operator::RollingRegressionResidual(window) => {
            let target = window_values(args, 0, *window, source, instrument_id, index)?;
            let factor = window_values(args, 1, *window, source, instrument_id, index)?;
            operators::rolling_regression_residual(&target, &factor)
        }
        Operator::CrossSectionalRank => {
            cross_sectional(args, source, instrument_id, index, |values, position| {
                Some(operators::cross_sectional_rank(values)[position])
            })
        }
        Operator::CrossSectionalScale => {
            cross_sectional(args, source, instrument_id, index, |values, position| {
                operators::cross_sectional_scale(values).map(|scaled| scaled[position])
            })
        }
        Operator::CrossSectionalSum => {
            cross_sectional(args, source, instrument_id, index, |values, _position| {
                Some(operators::cross_sectional_sum(values))
            })
        }
        Operator::Neutralize => neutralize(args, source, instrument_id, index),
    }
}

fn eval_arg(
    args: &[Expr],
    position: usize,
    source: &dyn ExprSource,
    instrument_id: InstrumentId,
    index: usize,
) -> Option<f64> {
    args.get(position)?.evaluate(source, instrument_id, index)
}

fn window_values(
    args: &[Expr],
    position: usize,
    window: usize,
    source: &dyn ExprSource,
    instrument_id: InstrumentId,
    index: usize,
) -> Option<Vec<f64>> {
    let start = index.checked_add(1)?.checked_sub(window)?;
    let arg = args.get(position)?;
    (start..=index)
        .map(|cursor| arg.evaluate(source, instrument_id, cursor))
        .collect()
}

fn cross_sectional(
    args: &[Expr],
    source: &dyn ExprSource,
    instrument_id: InstrumentId,
    index: usize,
    reduce: impl Fn(&[f64], usize) -> Option<f64>,
) -> Option<f64> {
    let arg = args.first()?;
    let mut values = Vec::new();
    let mut target = None;
    for member in source.universe() {
        if let Some(value) = arg.evaluate(source, *member, index) {
            if *member == instrument_id {
                target = Some(values.len());
            }
            values.push(value);
        }
    }
    reduce(&values, target?)
}

fn neutralize(
    args: &[Expr],
    source: &dyn ExprSource,
    instrument_id: InstrumentId,
    index: usize,
) -> Option<f64> {
    let value_arg = args.first()?;
    let group_arg = args.get(1)?;
    let mut values = Vec::new();
    let mut groups = Vec::new();
    let mut target = None;
    for member in source.universe() {
        if let (Some(value), Some(group)) = (
            value_arg.evaluate(source, *member, index),
            group_arg.evaluate(source, *member, index),
        ) {
            if *member == instrument_id {
                target = Some(values.len());
            }
            values.push(value);
            groups.push(group);
        }
    }
    let position = target?;
    Some(operators::neutralize(&values, &groups)[position])
}

fn flag(condition: bool) -> f64 {
    f64::from(u8::from(condition))
}

/// A source of column series and a point-in-time universe for feature evaluation.
///
/// The universe must be the member set that applies at the row being evaluated, as resolved from
/// the stored membership series; it is the cross-section a cross-sectional operator reduces over,
/// so an instrument that joined later or left earlier cannot enter the cross section.
pub trait ExprSource {
    /// Returns the index-aligned column series for an instrument, if the instrument has the column.
    fn column(&self, instrument_id: InstrumentId, column: &str) -> Option<&[f64]>;

    /// Returns the point-in-time member set for the row being evaluated.
    fn universe(&self) -> &[InstrumentId];
}

/// A failure to compile or use a feature definition.
#[derive(Clone, Debug, Error, PartialEq)]
pub enum ParseError {
    /// The definition ended while an expression was still expected.
    #[error("unexpected end of input")]
    UnexpectedEnd,
    /// A token appeared where it was not expected.
    #[error("unexpected token `{token}` at position {position}")]
    UnexpectedToken {
        /// The offending token.
        token: String,
        /// The token position, zero-based.
        position: usize,
    },
    /// A character is not part of the surface syntax.
    #[error("unexpected character `{character}` at position {position}")]
    UnexpectedCharacter {
        /// The offending character.
        character: char,
        /// The character position, zero-based.
        position: usize,
    },
    /// A number literal could not be parsed as a finite value.
    #[error("invalid number literal at position {position}")]
    InvalidNumber {
        /// The literal position, zero-based.
        position: usize,
    },
    /// A function name is not part of the operator set.
    #[error("unknown function `{name}`")]
    UnknownFunction {
        /// The unknown function name.
        name: String,
    },
    /// A function was called with the wrong number of arguments.
    #[error("`{name}` expects {expected} arguments, was {actual}")]
    Arity {
        /// The function name.
        name: String,
        /// The number of arguments the function expects.
        expected: usize,
        /// The number of arguments the function received.
        actual: usize,
    },
    /// An argument is ill-typed for the function.
    #[error("`{name}` expects {expected}")]
    InvalidArgument {
        /// The function name.
        name: String,
        /// A description of the argument the function expects.
        expected: &'static str,
    },
    /// A forward-reading operator was used on the inference side.
    #[error("`{function}` reads future data and is not permitted on the inference side")]
    FutureRead {
        /// The offending function name.
        function: String,
    },
}

/// A failure to use a feature definition on a side.
#[derive(Clone, Debug, Error, PartialEq)]
pub enum FeatureError {
    /// A definition was used on a side other than the one it declared.
    #[error("feature `{name}` is declared for the {declared} side, not the {requested} side")]
    SideMismatch {
        /// The feature name.
        name: String,
        /// The side the feature declared.
        declared: TransformSide,
        /// The side it was used on.
        requested: TransformSide,
    },
}

/// A compiled feature definition.
///
/// The definition is compiled once from its text form and is then immutable: its side, its tree
/// and its digest cannot drift from one another. It implements the dataset crate's
/// [`Feature`](crate::dataset::Feature) seam, so a declaration consumes only its name and digest.
#[derive(Clone, Debug, PartialEq)]
pub struct Feature {
    name: String,
    side: TransformSide,
    expr: Expr,
}

impl Feature {
    /// Compiles a feature definition from its text form on a declared side.
    ///
    /// # Errors
    ///
    /// Returns a [`ParseError`] if the definition is not parseable, names an unknown function,
    /// passes the wrong number of arguments, is ill-typed, or uses a forward-reading operator on
    /// the inference side.
    pub fn parse(
        name: impl Into<String>,
        side: TransformSide,
        definition: &str,
    ) -> Result<Self, ParseError> {
        let mut parser = Parser::new(definition, side)?;
        let expr = parser.parse()?;
        Ok(Self {
            name: name.into(),
            side,
            expr,
        })
    }

    /// Returns the canonical feature name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the side the feature is declared for.
    #[must_use]
    pub const fn side(&self) -> TransformSide {
        self.side
    }

    /// Returns the compiled expression tree.
    #[must_use]
    pub const fn expression(&self) -> &Expr {
        &self.expr
    }

    /// Returns the feature in its plain-text surface form.
    #[must_use]
    pub fn to_text(&self) -> String {
        self.expr.to_string()
    }

    /// Returns the canonical JSON serialization of the definition.
    ///
    /// The serialization is deterministic: object keys are sorted and the tree structure fixes
    /// the order of the arguments, so two equal definitions serialize identically regardless of
    /// the whitespace of their source text.
    #[must_use]
    pub fn canonical_json(&self) -> String {
        json!({
            "name": self.name,
            "side": self.side.as_str(),
            "definition": self.expr.to_json(),
        })
        .to_string()
    }

    /// Returns the stable digest of the definition's canonical serialization.
    #[must_use]
    pub fn digest(&self) -> Digest {
        Digest::of(self.canonical_json().as_bytes())
    }

    /// Returns the definition's name and digest as a dataset digest input.
    #[must_use]
    pub fn digest_input(&self) -> DigestInput {
        DigestInput::new(self.name.clone(), self.digest())
    }

    /// Returns an error if the feature was not declared for `side`.
    ///
    /// # Errors
    ///
    /// Returns [`FeatureError::SideMismatch`] when the requested side differs from the declared
    /// side, so a definition cannot be silently used where it was not declared.
    pub fn assert_side(&self, side: TransformSide) -> Result<(), FeatureError> {
        if self.side == side {
            Ok(())
        } else {
            Err(FeatureError::SideMismatch {
                name: self.name.clone(),
                declared: self.side,
                requested: side,
            })
        }
    }

    /// Evaluates the feature at `index` for one instrument.
    #[must_use]
    pub fn evaluate(
        &self,
        source: &dyn ExprSource,
        instrument_id: InstrumentId,
        index: usize,
    ) -> Option<f64> {
        self.expr.evaluate(source, instrument_id, index)
    }

    /// Attributes a value to this feature definition and the digests of the run that produced it.
    #[must_use]
    pub fn attribute(
        &self,
        value: f64,
        as_of: UnixNanos,
        dataset: Digest,
        membership: Digest,
        experiment: Digest,
        source_version: impl Into<String>,
    ) -> AttributedValue {
        AttributedValue::new(
            value,
            Provenance::new(
                dataset,
                self.digest(),
                membership,
                experiment,
                as_of,
                source_version,
            ),
        )
    }
}

impl FeatureTrait for Feature {
    fn name(&self) -> &str {
        &self.name
    }

    fn digest(&self) -> Digest {
        Digest::of(self.canonical_json().as_bytes())
    }
}

/// The digests, timestamp and source version that attribute a research value.
///
/// Every emitted value resolves to its dataset, feature, membership and experiment digests, its
/// as-of timestamp and the source version of its data. A value that cannot be attributed is not a
/// result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Provenance {
    /// The digest of the dataset declaration.
    pub dataset: Digest,
    /// The digest of the feature definition.
    pub feature: Digest,
    /// The digest of the membership series.
    pub membership: Digest,
    /// The digest of the experiment run.
    pub experiment: Digest,
    /// The instant of the latest input the value reads.
    pub as_of: UnixNanos,
    /// The source version of the underlying data.
    pub source_version: String,
}

impl Provenance {
    /// Creates a new [`Provenance`]; every field is required.
    #[must_use]
    pub fn new(
        dataset: Digest,
        feature: Digest,
        membership: Digest,
        experiment: Digest,
        as_of: UnixNanos,
        source_version: impl Into<String>,
    ) -> Self {
        Self {
            dataset,
            feature,
            membership,
            experiment,
            as_of,
            source_version: source_version.into(),
        }
    }
}

/// A value together with the provenance that attributes it.
#[derive(Clone, Debug, PartialEq)]
pub struct AttributedValue {
    /// The value.
    pub value: f64,
    /// The provenance of the value.
    pub provenance: Provenance,
}

impl AttributedValue {
    /// Creates a new [`AttributedValue`].
    #[must_use]
    pub const fn new(value: f64, provenance: Provenance) -> Self {
        Self { value, provenance }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Number(f64),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Lt,
    Le,
    Gt,
    Ge,
    Equal,
    NotEqual,
    LeftParen,
    RightParen,
    Comma,
}

impl Token {
    fn describe(&self) -> String {
        match self {
            Self::Number(value) => value.to_string(),
            Self::Ident(name) => name.clone(),
            Self::Plus => "+".to_string(),
            Self::Minus => "-".to_string(),
            Self::Star => "*".to_string(),
            Self::Slash => "/".to_string(),
            Self::Lt => "<".to_string(),
            Self::Le => "<=".to_string(),
            Self::Gt => ">".to_string(),
            Self::Ge => ">=".to_string(),
            Self::Equal => "==".to_string(),
            Self::NotEqual => "!=".to_string(),
            Self::LeftParen => "(".to_string(),
            Self::RightParen => ")".to_string(),
            Self::Comma => ",".to_string(),
        }
    }
}

fn tokenize(input: &str) -> Result<Vec<Token>, ParseError> {
    let chars: Vec<char> = input.chars().collect();
    let mut tokens = Vec::new();
    let mut position = 0;

    while position < chars.len() {
        let character = chars[position];
        if character.is_whitespace() {
            position += 1;
            continue;
        }

        let next = chars.get(position + 1).copied();
        match character {
            '+' => {
                tokens.push(Token::Plus);
                position += 1;
            }
            '-' => {
                tokens.push(Token::Minus);
                position += 1;
            }
            '*' => {
                tokens.push(Token::Star);
                position += 1;
            }
            '/' => {
                tokens.push(Token::Slash);
                position += 1;
            }
            '<' => {
                if next == Some('=') {
                    tokens.push(Token::Le);
                    position += 2;
                } else {
                    tokens.push(Token::Lt);
                    position += 1;
                }
            }
            '>' => {
                if next == Some('=') {
                    tokens.push(Token::Ge);
                    position += 2;
                } else {
                    tokens.push(Token::Gt);
                    position += 1;
                }
            }
            '=' => {
                if next == Some('=') {
                    tokens.push(Token::Equal);
                    position += 2;
                } else {
                    return Err(ParseError::UnexpectedCharacter {
                        character,
                        position,
                    });
                }
            }
            '!' => {
                if next == Some('=') {
                    tokens.push(Token::NotEqual);
                    position += 2;
                } else {
                    return Err(ParseError::UnexpectedCharacter {
                        character,
                        position,
                    });
                }
            }
            '(' => {
                tokens.push(Token::LeftParen);
                position += 1;
            }
            ')' => {
                tokens.push(Token::RightParen);
                position += 1;
            }
            ',' => {
                tokens.push(Token::Comma);
                position += 1;
            }
            character if character.is_ascii_digit() => {
                let start = position;
                while position < chars.len()
                    && (chars[position].is_ascii_digit() || chars[position] == '.')
                {
                    position += 1;
                }
                let literal: String = chars[start..position].iter().collect();
                let value: f64 = literal
                    .parse()
                    .map_err(|_| ParseError::InvalidNumber { position: start })?;
                if !value.is_finite() {
                    return Err(ParseError::InvalidNumber { position: start });
                }
                tokens.push(Token::Number(value));
            }
            character if character.is_ascii_alphabetic() || character == '_' => {
                let start = position;
                while position < chars.len()
                    && (chars[position].is_ascii_alphanumeric() || chars[position] == '_')
                {
                    position += 1;
                }
                tokens.push(Token::Ident(chars[start..position].iter().collect()));
            }
            _ => {
                return Err(ParseError::UnexpectedCharacter {
                    character,
                    position,
                });
            }
        }
    }

    Ok(tokens)
}

struct Parser {
    tokens: Vec<Token>,
    position: usize,
    side: TransformSide,
}

impl Parser {
    fn new(input: &str, side: TransformSide) -> Result<Self, ParseError> {
        Ok(Self {
            tokens: tokenize(input)?,
            position: 0,
            side,
        })
    }

    fn parse(&mut self) -> Result<Expr, ParseError> {
        let expr = self.parse_comparison()?;
        if let Some(token) = self.peek() {
            return Err(ParseError::UnexpectedToken {
                token: token.describe(),
                position: self.position,
            });
        }
        Ok(expr)
    }

    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let lhs = self.parse_additive()?;
        let op = match self.peek() {
            Some(Token::Lt) => Operator::Less,
            Some(Token::Le) => Operator::LessOrEqual,
            Some(Token::Gt) => Operator::Greater,
            Some(Token::Ge) => Operator::GreaterOrEqual,
            Some(Token::Equal) => Operator::Equal,
            Some(Token::NotEqual) => Operator::NotEqual,
            _ => return Ok(lhs),
        };
        self.position += 1;
        let rhs = self.parse_additive()?;
        Ok(Expr::Call {
            op,
            args: vec![lhs, rhs],
        })
    }

    fn parse_additive(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_multiplicative()?;
        loop {
            let op = match self.peek() {
                Some(Token::Plus) => Operator::Add,
                Some(Token::Minus) => Operator::Sub,
                _ => break,
            };
            self.position += 1;
            let rhs = self.parse_multiplicative()?;
            lhs = Expr::Call {
                op,
                args: vec![lhs, rhs],
            };
        }
        Ok(lhs)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_unary()?;
        loop {
            let op = match self.peek() {
                Some(Token::Star) => Operator::Mul,
                Some(Token::Slash) => Operator::Div,
                _ => break,
            };
            self.position += 1;
            let rhs = self.parse_unary()?;
            lhs = Expr::Call {
                op,
                args: vec![lhs, rhs],
            };
        }
        Ok(lhs)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        if matches!(self.peek(), Some(Token::Minus)) {
            self.position += 1;
            let arg = self.parse_unary()?;
            return Ok(Expr::Call {
                op: Operator::Neg,
                args: vec![arg],
            });
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let Some(token) = self.advance() else {
            return Err(ParseError::UnexpectedEnd);
        };
        match token {
            Token::Number(value) => Ok(Expr::Literal(value)),
            Token::Ident(name) => {
                if matches!(self.peek(), Some(Token::LeftParen)) {
                    self.parse_call(&name)
                } else {
                    Ok(Expr::Column(name))
                }
            }
            Token::LeftParen => {
                let expr = self.parse_comparison()?;
                self.expect(&Token::RightParen)?;
                Ok(expr)
            }
            other => Err(ParseError::UnexpectedToken {
                token: other.describe(),
                position: self.position.saturating_sub(1),
            }),
        }
    }

    fn parse_call(&mut self, name: &str) -> Result<Expr, ParseError> {
        self.expect(&Token::LeftParen)?;
        let mut args = Vec::new();
        if matches!(self.peek(), Some(Token::RightParen)) {
            self.position += 1;
        } else {
            loop {
                args.push(self.parse_comparison()?);
                match self.advance() {
                    Some(Token::Comma) => {}
                    Some(Token::RightParen) => break,
                    Some(other) => {
                        return Err(ParseError::UnexpectedToken {
                            token: other.describe(),
                            position: self.position.saturating_sub(1),
                        });
                    }
                    None => return Err(ParseError::UnexpectedEnd),
                }
            }
        }
        build_call(name, args, self.side)
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }

    fn advance(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.position).cloned();
        if token.is_some() {
            self.position += 1;
        }
        token
    }

    fn expect(&mut self, expected: &Token) -> Result<(), ParseError> {
        match self.advance() {
            Some(token) if token == *expected => Ok(()),
            Some(token) => Err(ParseError::UnexpectedToken {
                token: token.describe(),
                position: self.position.saturating_sub(1),
            }),
            None => Err(ParseError::UnexpectedEnd),
        }
    }
}

fn check_arity(name: &str, actual: usize, expected: usize) -> Result<(), ParseError> {
    if actual == expected {
        Ok(())
    } else {
        Err(ParseError::Arity {
            name: name.to_string(),
            expected,
            actual,
        })
    }
}

fn window_parameter(name: &str, expr: &Expr) -> Result<usize, ParseError> {
    if let Expr::Literal(value) = expr
        && value.is_finite()
        && (1.0..=usize::MAX as f64).contains(value)
        && value.fract().abs() < f64::EPSILON
    {
        return Ok(*value as usize);
    }
    Err(ParseError::InvalidArgument {
        name: name.to_string(),
        expected: "a positive integer window or lag",
    })
}

fn build_call(name: &str, args: Vec<Expr>, side: TransformSide) -> Result<Expr, ParseError> {
    let call = match name {
        "lag" | "lead" | "rolling_mean" | "rolling_std" | "rolling_rank" => {
            check_arity(name, args.len(), 2)?;
            let mut args = args;
            let input = args.remove(0);
            let window = window_parameter(name, &args.remove(0))?;
            let op = match name {
                "lag" => Operator::Lag(window),
                "lead" => Operator::Lead(window),
                "rolling_mean" => Operator::RollingMean(window),
                "rolling_std" => Operator::RollingStd(window),
                _ => Operator::RollingRank(window),
            };
            if matches!(op, Operator::Lead(_)) && side == TransformSide::Inference {
                return Err(ParseError::FutureRead {
                    function: name.to_string(),
                });
            }
            Expr::Call {
                op,
                args: vec![input],
            }
        }
        "rolling_correlation" | "rolling_regression_residual" => {
            check_arity(name, args.len(), 3)?;
            let mut args = args;
            let first = args.remove(0);
            let second = args.remove(0);
            let window = window_parameter(name, &args.remove(0))?;
            let op = if name == "rolling_correlation" {
                Operator::RollingCorrelation(window)
            } else {
                Operator::RollingRegressionResidual(window)
            };
            Expr::Call {
                op,
                args: vec![first, second],
            }
        }
        "rank" | "scale" | "sum" => {
            check_arity(name, args.len(), 1)?;
            let mut args = args;
            let input = args.remove(0);
            let op = match name {
                "rank" => Operator::CrossSectionalRank,
                "scale" => Operator::CrossSectionalScale,
                _ => Operator::CrossSectionalSum,
            };
            Expr::Call {
                op,
                args: vec![input],
            }
        }
        "neutralize" => {
            check_arity(name, args.len(), 2)?;
            let mut args = args;
            let input = args.remove(0);
            let group = args.remove(0);
            if !matches!(group, Expr::Column(_)) {
                return Err(ParseError::InvalidArgument {
                    name: name.to_string(),
                    expected: "a group column as its second argument",
                });
            }
            Expr::Call {
                op: Operator::Neutralize,
                args: vec![input, group],
            }
        }
        _ => {
            return Err(ParseError::UnknownFunction {
                name: name.to_string(),
            });
        }
    };
    Ok(call)
}
