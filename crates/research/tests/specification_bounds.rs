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

//! The information coefficient's significance reported at both specification extremes
//! (`design 6.2`).
//!
//! A bound is only a bound once it is read at two extremes, so these tests exercise the pair and
//! the refusals: a single bound, a single declared specification, and a pair that is mislabelled
//! or unordered.

use nautilus_research::{
    InformationCoefficient, SpecificationBound, SpecificationBoundError, SpecificationBounds,
    SpecificationExtreme, information_coefficient_bounds,
};

/// An information coefficient with a mean and its jackknife standard error.
///
/// The significance is a function of the two, so the per-date series is not needed to exercise it.
fn coefficient(mean: f64, standard_error: f64) -> InformationCoefficient {
    InformationCoefficient {
        series: Vec::new(),
        mean: Some(mean),
        standard_error: Some(standard_error),
        interval: None,
        dates: 3,
        observations: 30,
        coverage_absent: 0,
        label_absent: 0,
    }
}

#[test]
fn the_bound_is_reported_at_both_specification_extremes() {
    let equal = coefficient(0.30, 0.10);
    let value_weighted = coefficient(0.05, 0.10);

    let bounds = information_coefficient_bounds(&[
        ("equal", &equal),
        ("value_four_factor", &value_weighted),
    ])
    .expect("two distinct specifications can be compared");

    // The higher significance is the most favourable reading, and each bound names its
    // specification and states which extreme it is.
    assert_eq!(bounds.most_favourable.specification, "equal");
    assert_eq!(
        bounds.most_favourable.extreme,
        SpecificationExtreme::MostFavourable
    );
    assert!((bounds.most_favourable.value - 3.0).abs() < 1e-12);

    assert_eq!(bounds.least_favourable.specification, "value_four_factor");
    assert_eq!(
        bounds.least_favourable.extreme,
        SpecificationExtreme::LeastFavourable
    );
    assert!((bounds.least_favourable.value - 0.5).abs() < 1e-12);

    assert_eq!(bounds.most_favourable.extreme.name(), "most_favourable");
    assert_eq!(bounds.least_favourable.extreme.name(), "least_favourable");
}

#[test]
fn a_single_bound_report_is_refused() {
    let bound = SpecificationBound {
        specification: "equal".to_string(),
        extreme: SpecificationExtreme::MostFavourable,
        value: 0.5,
    };

    assert_eq!(
        SpecificationBounds::from_bounds(std::slice::from_ref(&bound)),
        Err(SpecificationBoundError::SingleBound)
    );

    // Two bounds at one extreme are still one extreme, not both.
    assert_eq!(
        SpecificationBounds::from_bounds(&[bound.clone(), bound]),
        Err(SpecificationBoundError::SingleBound)
    );
}

#[test]
fn a_search_that_declared_one_specification_could_not_be_checked() {
    let equal = coefficient(0.30, 0.10);

    assert_eq!(
        information_coefficient_bounds(&[("equal", &equal)]),
        Err(SpecificationBoundError::NotCheckable { declared: 1 })
    );

    // Two measurements under one specification are still one declared specification.
    assert_eq!(
        information_coefficient_bounds(&[("equal", &equal), ("equal", &equal)]),
        Err(SpecificationBoundError::DuplicateSpecification {
            specification: "equal".to_string()
        })
    );

    // A bound whose specification is unnamed cannot be compared.
    assert_eq!(
        information_coefficient_bounds(&[("", &equal), ("value", &equal)]),
        Err(SpecificationBoundError::UnnamedSpecification)
    );
}

#[test]
fn coincident_extremes_read_as_two_equal_bounds_naming_one_specification() {
    let first = coefficient(0.20, 0.10);
    let second = coefficient(0.20, 0.10);

    let bounds = information_coefficient_bounds(&[("first", &first), ("second", &second)])
        .expect("two distinct specifications can be compared");

    // The search varied the specification but nothing that moves the bound, so the pair is two
    // equal bounds naming one specification rather than one bound.
    assert_eq!(bounds.most_favourable.value, bounds.least_favourable.value);
    assert_eq!(bounds.most_favourable.specification, "first");
    assert_eq!(bounds.least_favourable.specification, "first");
    assert_ne!(
        bounds.most_favourable.extreme,
        bounds.least_favourable.extreme
    );
}

#[test]
fn a_bound_pair_refuses_an_unordered_or_mislabelled_pair() {
    let low = SpecificationBound {
        specification: "equal".to_string(),
        extreme: SpecificationExtreme::MostFavourable,
        value: 0.2,
    };
    let high = SpecificationBound {
        specification: "value".to_string(),
        extreme: SpecificationExtreme::LeastFavourable,
        value: 0.8,
    };

    assert_eq!(
        SpecificationBounds::new(low, high),
        Err(SpecificationBoundError::Unordered {
            most: 0.2,
            least: 0.8
        })
    );

    let labelled_most = SpecificationBound {
        specification: "value".to_string(),
        extreme: SpecificationExtreme::MostFavourable,
        value: 0.8,
    };

    assert_eq!(
        SpecificationBounds::new(labelled_most.clone(), labelled_most),
        Err(SpecificationBoundError::MislabelledExtreme {
            expected: "least_favourable",
            found: "most_favourable",
        })
    );
}
