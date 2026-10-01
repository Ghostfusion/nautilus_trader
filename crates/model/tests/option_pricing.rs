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

use nautilus_model::{
    data::{
        binomial::{CoxRossRubinstein, DEFAULT_STEPS},
        greeks::black_scholes_greeks_exact,
        pricing::{
            OptionPricingModel, OptionPricingParams, implied_forward_from_parity, price_option,
        },
    },
    enums::{ExerciseStyle, OptionKind},
};

const SPOT: f64 = 100.0;
const STRIKE: f64 = 100.0;
const RATE: f64 = 0.05;
const VOL: f64 = 0.2;
const EXPIRY: f64 = 1.0;

#[expect(clippy::too_many_arguments)]
fn params(
    spot: f64,
    strike: f64,
    risk_free_rate: f64,
    cost_of_carry: f64,
    volatility: f64,
    time_to_expiry: f64,
    option_kind: OptionKind,
    exercise_style: ExerciseStyle,
) -> OptionPricingParams {
    OptionPricingParams {
        spot,
        strike,
        risk_free_rate,
        cost_of_carry,
        volatility,
        time_to_expiry,
        option_kind,
        exercise_style,
    }
}

/// The repository's own f64 European closed form, used as the independent reference.
fn european_reference(p: &OptionPricingParams) -> f64 {
    black_scholes_greeks_exact(
        p.spot,
        p.risk_free_rate,
        p.cost_of_carry,
        p.volatility,
        matches!(p.option_kind, OptionKind::Call),
        p.strike,
        p.time_to_expiry,
    )
    .price
}

/// Analytic intrinsic value, `max(S - K, 0)` for a call and `max(K - S, 0)` for a put.
fn intrinsic(spot: f64, strike: f64, kind: OptionKind) -> f64 {
    match kind {
        OptionKind::Call => (spot - strike).max(0.0),
        OptionKind::Put => (strike - spot).max(0.0),
    }
}

/// Discounted intrinsic value of the forward, `exp(-r T) * max(F - K, 0)` (call).
fn discounted_forward_intrinsic(p: &OptionPricingParams) -> f64 {
    let forward = p.spot * (p.cost_of_carry * p.time_to_expiry).exp();
    intrinsic(forward, p.strike, p.option_kind) * (-p.risk_free_rate * p.time_to_expiry).exp()
}

// 6a. The CRR tree converges to the repository's Black-Scholes price as the step count rises.
#[test]
fn crr_converges_to_black_scholes() {
    let p = params(
        SPOT,
        STRIKE,
        RATE,
        RATE,
        VOL,
        EXPIRY,
        OptionKind::Call,
        ExerciseStyle::European,
    );
    let exact = european_reference(&p);

    let error_at = |steps| (CoxRossRubinstein { steps }.price(&p).unwrap() - exact).abs();

    let e32 = error_at(32);
    let e128 = error_at(128);
    let e512 = error_at(512);

    // Error decreases strictly across the named step counts (CRR error oscillates but the
    // 1/N envelope dominates here).
    assert!(e128 < e32, "e128={e128} !< e32={e32}");
    assert!(e512 < e128, "e512={e512} !< e128={e128}");

    // Absolute-error bounds observed for this case on this host (CRR is O(1/N)):
    //   N=32  -> 0.06224, N=128 -> 0.01561, N=512 -> 0.00390 (exact = 10.450584...).
    assert!(e32 < 1e-1, "e32={e32}");
    assert!(e128 < 2e-2, "e128={e128}");
    assert!(e512 < 5e-3, "e512={e512}");

    // The default step count targets the same converged value.
    let default_price = CoxRossRubinstein::default().price(&p).unwrap();
    assert!((default_price - exact).abs() < 5e-3);
    assert_eq!(CoxRossRubinstein::default().steps, DEFAULT_STEPS);
}

// 6b. An American put dominates both its European counterpart and its intrinsic value.
#[test]
fn american_put_dominates_european_and_intrinsic() {
    for spot in [80.0, 100.0, 120.0] {
        let european = params(
            spot,
            STRIKE,
            RATE,
            RATE,
            VOL,
            EXPIRY,
            OptionKind::Put,
            ExerciseStyle::European,
        );
        let american = OptionPricingParams {
            exercise_style: ExerciseStyle::American,
            ..european
        };

        let euro_price = price_option(&european).unwrap();
        let amer_price = price_option(&american).unwrap();
        let intrinsic_value = intrinsic(spot, STRIKE, OptionKind::Put);

        assert!(
            amer_price >= euro_price - 1e-12,
            "spot={spot}: american={amer_price} < european={euro_price}",
        );
        assert!(
            amer_price >= intrinsic_value - 1e-12,
            "spot={spot}: american={amer_price} < intrinsic={intrinsic_value}",
        );
    }
}

// 6c. With no dividend (b = r) an American call has no early-exercise premium.
#[test]
fn american_call_without_dividend_equals_european() {
    for spot in [80.0, 100.0, 120.0] {
        let european = params(
            spot,
            STRIKE,
            RATE,
            RATE,
            VOL,
            EXPIRY,
            OptionKind::Call,
            ExerciseStyle::European,
        );
        let american = OptionPricingParams {
            exercise_style: ExerciseStyle::American,
            ..european
        };

        let euro_price = price_option(&european).unwrap();
        let amer_price = price_option(&american).unwrap();

        // The identity is exact in continuous time; the residual is the CRR discretization
        // error at DEFAULT_STEPS (observed < 4e-3 for this case).
        assert!(
            (amer_price - euro_price).abs() < 5e-3,
            "spot={spot}: american={amer_price}, european={euro_price}",
        );
    }
}

// 6d. At expiry the price equals the intrinsic value.
#[test]
fn at_expiry_price_equals_intrinsic() {
    let strikes = [80.0, 100.0, 120.0];
    let carries = [0.0, 0.05];

    for strike in strikes {
        for cost_of_carry in carries {
            for option_kind in [OptionKind::Call, OptionKind::Put] {
                for exercise_style in [ExerciseStyle::European, ExerciseStyle::American] {
                    let p = params(
                        SPOT,
                        strike,
                        RATE,
                        cost_of_carry,
                        VOL,
                        0.0,
                        option_kind,
                        exercise_style,
                    );
                    let expected = intrinsic(SPOT, strike, option_kind);

                    assert!(
                        (price_option(&p).unwrap() - expected).abs() < 1e-12,
                        "strike={strike}, b={cost_of_carry}, kind={option_kind:?}, style={exercise_style:?}",
                    );
                }
            }
        }
    }
}

// 6e. Zero volatility gives the discounted forward intrinsic.
#[test]
fn zero_volatility_gives_discounted_forward_intrinsic() {
    let carries = [0.0, -0.03, 0.05];

    for cost_of_carry in carries {
        for strike in [80.0, 100.0, 120.0] {
            for option_kind in [OptionKind::Call, OptionKind::Put] {
                for exercise_style in [ExerciseStyle::European, ExerciseStyle::American] {
                    let p = params(
                        SPOT,
                        strike,
                        RATE,
                        cost_of_carry,
                        0.0,
                        EXPIRY,
                        option_kind,
                        exercise_style,
                    );
                    let expected = discounted_forward_intrinsic(&p);

                    assert!(
                        (price_option(&p).unwrap() - expected).abs() < 1e-12,
                        "strike={strike}, b={cost_of_carry}, kind={option_kind:?}, style={exercise_style:?}",
                    );
                }
            }
        }
    }
}

// 6f. Ignoring the declared style changes the price materially for a deep ITM put.
#[test]
fn price_option_routes_on_declared_style() {
    // Deep in-the-money American put: early exercise is valuable.
    let european = params(
        60.0,
        STRIKE,
        RATE,
        RATE,
        VOL,
        EXPIRY,
        OptionKind::Put,
        ExerciseStyle::European,
    );
    let american = OptionPricingParams {
        exercise_style: ExerciseStyle::American,
        ..european
    };

    let euro_price = price_option(&european).unwrap();
    let amer_price = price_option(&american).unwrap();

    assert!(
        amer_price > euro_price + 1.0,
        "american={amer_price}, european={euro_price}",
    );

    // `price_option` must route European to the closed form, not to the tree.
    assert!((euro_price - european_reference(&european)).abs() < 1e-12);

    // And route American to the tree rather than the closed form.
    let tree = CoxRossRubinstein::default().price(&american).unwrap();
    assert!((amer_price - tree).abs() < 1e-12);
}

// 6g. Reference value table.
//
// Provenance for every entry:
// - European entries: the repository's own closed form `black_scholes_greeks_exact` (source ii).
//   The expected value recorded is that closed form; it is asserted against `price_option`.
// - American entries: no independent published value is claimed. Each is checked against the
//   analytic identities it must satisfy (source i): intrinsic bound, dominance of the European
//   price for a put, and equality with the European price for a call when there is no dividend
//   (b = r). The comment next to each entry states which identity is used.
#[test]
fn reference_value_table() {
    struct Case {
        name: &'static str,
        spot: f64,
        strike: f64,
        risk_free_rate: f64,
        cost_of_carry: f64,
        volatility: f64,
        time_to_expiry: f64,
        option_kind: OptionKind,
        exercise_style: ExerciseStyle,
        provenance: &'static str,
    }

    let cases = [
        // European, deep ITM call, zero carry (option on a future, Black-76).
        Case {
            name: "euro_itm_call_b0",
            spot: 120.0,
            strike: 100.0,
            risk_free_rate: 0.05,
            cost_of_carry: 0.0,
            volatility: 0.2,
            time_to_expiry: 1.0,
            option_kind: OptionKind::Call,
            exercise_style: ExerciseStyle::European,
            provenance: "repository closed form",
        },
        // European, deep OTM put, zero carry.
        Case {
            name: "euro_otm_put_b0",
            spot: 120.0,
            strike: 100.0,
            risk_free_rate: 0.05,
            cost_of_carry: 0.0,
            volatility: 0.2,
            time_to_expiry: 1.0,
            option_kind: OptionKind::Put,
            exercise_style: ExerciseStyle::European,
            provenance: "repository closed form",
        },
        // European, high dividend yield (b = r - q = 0.05 - 0.10 = -0.05).
        Case {
            name: "euro_call_high_yield",
            spot: 100.0,
            strike: 100.0,
            risk_free_rate: 0.05,
            cost_of_carry: -0.05,
            volatility: 0.2,
            time_to_expiry: 1.0,
            option_kind: OptionKind::Call,
            exercise_style: ExerciseStyle::European,
            provenance: "repository closed form",
        },
        // European, zero rate, zero carry.
        Case {
            name: "euro_put_zero_rate",
            spot: 100.0,
            strike: 100.0,
            risk_free_rate: 0.0,
            cost_of_carry: 0.0,
            volatility: 0.2,
            time_to_expiry: 1.0,
            option_kind: OptionKind::Put,
            exercise_style: ExerciseStyle::European,
            provenance: "repository closed form",
        },
        // European, high rate.
        Case {
            name: "euro_call_high_rate",
            spot: 100.0,
            strike: 100.0,
            risk_free_rate: 0.20,
            cost_of_carry: 0.20,
            volatility: 0.2,
            time_to_expiry: 1.0,
            option_kind: OptionKind::Call,
            exercise_style: ExerciseStyle::European,
            provenance: "repository closed form",
        },
        // American, deep ITM put: must exceed intrinsic and the European price.
        Case {
            name: "amer_itm_put",
            spot: 60.0,
            strike: 100.0,
            risk_free_rate: 0.05,
            cost_of_carry: 0.05,
            volatility: 0.2,
            time_to_expiry: 1.0,
            option_kind: OptionKind::Put,
            exercise_style: ExerciseStyle::American,
            provenance: "identity: >= intrinsic and >= European",
        },
        // American, deep OTM put: no early exercise premium, must equal European.
        Case {
            name: "amer_otm_put",
            spot: 200.0,
            strike: 100.0,
            risk_free_rate: 0.05,
            cost_of_carry: 0.05,
            volatility: 0.2,
            time_to_expiry: 1.0,
            option_kind: OptionKind::Put,
            exercise_style: ExerciseStyle::American,
            provenance: "identity: >= intrinsic (equals European here)",
        },
        // American call with no dividend: equals European.
        Case {
            name: "amer_call_b_eq_r",
            spot: 100.0,
            strike: 100.0,
            risk_free_rate: 0.05,
            cost_of_carry: 0.05,
            volatility: 0.2,
            time_to_expiry: 1.0,
            option_kind: OptionKind::Call,
            exercise_style: ExerciseStyle::American,
            provenance: "identity: equals European when b = r",
        },
    ];

    for case in cases {
        let p = params(
            case.spot,
            case.strike,
            case.risk_free_rate,
            case.cost_of_carry,
            case.volatility,
            case.time_to_expiry,
            case.option_kind,
            case.exercise_style,
        );
        let price = price_option(&p).unwrap();
        let european = european_reference(&p);
        let intrinsic_value = intrinsic(case.spot, case.strike, case.option_kind);

        match case.exercise_style {
            ExerciseStyle::European => {
                assert!(
                    (price - european).abs() < 1e-12,
                    "{}: {} (provenance: {})",
                    case.name,
                    price,
                    case.provenance,
                );
            }
            ExerciseStyle::American => {
                assert!(
                    price >= intrinsic_value - 1e-9,
                    "{}: {} below intrinsic {} (provenance: {})",
                    case.name,
                    price,
                    intrinsic_value,
                    case.provenance,
                );

                // The identities are exact in continuous time; the residual is the CRR
                // discretization error at DEFAULT_STEPS, tolerated here.
                if case.option_kind == OptionKind::Call && case.cost_of_carry == case.risk_free_rate
                {
                    assert!(
                        (price - european).abs() < 5e-3,
                        "{}: {} != European {} (provenance: {})",
                        case.name,
                        price,
                        european,
                        case.provenance,
                    );
                } else if case.option_kind == OptionKind::Put {
                    assert!(
                        price >= european - 5e-3,
                        "{}: {} below European {} (provenance: {})",
                        case.name,
                        price,
                        european,
                        case.provenance,
                    );
                }
            }
        }
    }
}

#[test]
fn implied_forward_from_parity_recovers_forward() {
    // An option on a future (b = 0): the forward equals the spot, so parity returns S.
    let p = params(
        SPOT,
        STRIKE,
        RATE,
        0.0,
        VOL,
        EXPIRY,
        OptionKind::Call,
        ExerciseStyle::European,
    );
    let call = european_reference(&p);
    let put = european_reference(&OptionPricingParams {
        option_kind: OptionKind::Put,
        ..p
    });
    let forward = implied_forward_from_parity(call, put, STRIKE, RATE, EXPIRY);

    assert!((forward - SPOT).abs() < 1e-9, "forward={forward}");
}
