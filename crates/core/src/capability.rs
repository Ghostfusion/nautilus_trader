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

//! Domain-scoped capability answers.
//!
//! A capability answer states whether a request can be served and, when it cannot, what stops it: a
//! canonical code owned by the domain that answers, a human-readable detail, and the requirements
//! that were not met. The shape is shared and the code sets are not. Each domain declares its own
//! closed set, so no universal enum accumulates every refusal reason in the system and stops being
//! checkable.
//!
//! The code is canonical and the detail is not. An order denial's codes are the leading tokens of
//! its messages and are upper case; the analysis crate's metric reasons are the lower-case
//! vocabulary it already publishes. Only the code may be compared, matched or used for control
//! flow; the detail is for a human, and the requirements are the list a caller can act on. An
//! answer is either available, which carries no code, or unavailable, which always carries one, so
//! the two cannot disagree.

use std::fmt::{Display, Formatter};

/// Returns whether the given token is a canonical capability code.
///
/// A canonical code is non-empty, ASCII, contains at least one letter, and is made of letters,
/// digits and underscores only. The case convention belongs to the domain that owns the set.
#[must_use]
pub fn is_canonical_code(token: &str) -> bool {
    !token.is_empty()
        && token.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && token.chars().any(|c| c.is_ascii_alphabetic())
}

/// A capability answer: whether a request can be served, and what stops it when it cannot.
///
/// Parameters are deliberately absent: an answer describes a question that was asked, and the
/// caller already holds what it asked about. A probe built on this type is a pure function of its
/// inputs, so it is cheap to call before doing the work it predicts.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "python",
    pyo3::pyclass(module = "nautilus_trader.core", from_py_object)
)]
#[cfg_attr(
    feature = "python",
    pyo3_stub_gen::derive::gen_stub_pyclass(module = "nautilus_trader.core")
)]
pub struct Capability {
    available: bool,
    code: Option<String>,
    detail: String,
    requirements: Vec<String>,
}

impl Capability {
    /// Returns an available answer, which carries no code and no requirements.
    #[must_use]
    pub fn available() -> Self {
        Self {
            available: true,
            code: None,
            detail: String::new(),
            requirements: Vec::new(),
        }
    }

    /// Returns an unavailable answer carrying the given code and detail.
    ///
    /// The code must come from a closed set the domain declares, because a code that is prose
    /// defeats the checkable half of the answer. Canonical form is asserted in debug builds and
    /// every domain asserts the form of its whole set in its own tests, so the closed sets are
    /// what enforce it and this only reports a set that was declared badly.
    ///
    /// # Panics
    ///
    /// Panics in debug builds if the code is not canonical.
    #[must_use]
    pub fn unavailable(code: impl Into<String>, detail: impl Into<String>) -> Self {
        let code = code.into();
        debug_assert!(
            is_canonical_code(&code),
            "capability code `{code}` is not canonical"
        );
        Self {
            available: false,
            code: Some(code),
            detail: detail.into(),
            requirements: Vec::new(),
        }
    }

    /// Returns this answer with a requirement added, in the order the requirements are recorded.
    #[must_use]
    pub fn requiring(mut self, requirement: impl Into<String>) -> Self {
        self.requirements.push(requirement.into());
        self
    }

    /// Returns whether the request can be served.
    #[must_use]
    pub const fn is_available(&self) -> bool {
        self.available
    }

    /// Returns the canonical code, or `None` when the request can be served.
    #[must_use]
    pub fn code(&self) -> Option<&str> {
        self.code.as_deref()
    }

    /// Returns the human-readable detail, which is empty when the request can be served.
    #[must_use]
    pub fn detail(&self) -> &str {
        &self.detail
    }

    /// Returns the requirements that were not met.
    #[must_use]
    pub fn requirements(&self) -> &[String] {
        &self.requirements
    }
}

impl Display for Capability {
    /// Writes the code first when the request cannot be served, so a log line is classifiable.
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match (self.code.as_deref(), self.detail.is_empty()) {
            (None, _) => f.write_str("available"),
            (Some(code), true) => f.write_str(code),
            (Some(code), false) => write!(f, "{code}: {}", self.detail),
        }
    }
}

#[cfg(test)]
mod tests {
    use rstest::rstest;

    use super::*;

    #[rstest]
    #[case("PRICE_NOT_POSITIVE", true)]
    #[case("unsupported_input", true)]
    #[case("RANGE2", true)]
    #[case("", false)]
    #[case("TWO WORDS", false)]
    #[case("_", false)]
    #[case("42", false)]
    #[case("UPPER_CASE", true)]
    #[case("acentuado_\u{e9}", false)]
    fn test_canonical_codes_are_tokens_rather_than_prose(
        #[case] token: &str,
        #[case] expected: bool,
    ) {
        assert_eq!(is_canonical_code(token), expected);
    }

    #[test]
    fn test_an_available_answer_carries_no_code_and_no_requirements() {
        let capability = Capability::available();

        assert!(capability.is_available());
        assert_eq!(capability.code(), None);
        assert!(capability.detail().is_empty());
        assert!(capability.requirements().is_empty());
        assert_eq!(capability.to_string(), "available");
    }

    #[test]
    fn test_an_unavailable_answer_carries_its_code_and_renders_it_first() {
        let capability = Capability::unavailable("PRICE_NOT_POSITIVE", "the price was -1");

        assert!(!capability.is_available());
        assert_eq!(capability.code(), Some("PRICE_NOT_POSITIVE"));
        assert_eq!(
            capability.to_string(),
            "PRICE_NOT_POSITIVE: the price was -1"
        );
    }

    #[test]
    fn test_an_unavailable_answer_with_no_detail_renders_only_the_code() {
        let capability = Capability::unavailable("RANGE_NOT_COVERED", "");

        assert_eq!(capability.to_string(), "RANGE_NOT_COVERED");
    }

    #[test]
    fn test_requirements_are_recorded_in_order() {
        let capability = Capability::unavailable("RANGE_GAPS", "covered in part")
            .requiring("missing 100..200")
            .requiring("missing 400..500");

        assert_eq!(
            capability.requirements(),
            ["missing 100..200", "missing 400..500"]
        );
    }

    #[test]
    #[should_panic(expected = "is not canonical")]
    fn test_a_code_that_is_prose_is_refused() {
        let _ = Capability::unavailable("two words", "detail");
    }
}
