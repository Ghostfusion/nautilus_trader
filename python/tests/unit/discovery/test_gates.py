# -------------------------------------------------------------------------------------------------
#  Copyright (C) 2015-2026 Nautech Systems Pty Ltd. All rights reserved.
#  https://nautechsystems.io
#
#  Licensed under the GNU Lesser General Public License Version 3.0 (the "License");
#  You may not use this file except in compliance with the License.
#  You may obtain a copy of the License at https://www.gnu.org/licenses/lgpl-3.0.en.html
#
#  Unless required by applicable law or agreed to in writing, software
#  distributed under the License is distributed on an "AS IS" BASIS,
#  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#  See the License for the specific language governing permissions and
#  limitations under the License.
# -------------------------------------------------------------------------------------------------
"""
Tests for the gate service, the candidate lifecycle, the admission assessment and the calibration.
"""

from __future__ import annotations

import pytest

from nautilus_trader.discovery.gates import TRANSITIONS
from nautilus_trader.discovery.gates import Actor
from nautilus_trader.discovery.gates import AdmissionAssessment
from nautilus_trader.discovery.gates import AdmissionInputs
from nautilus_trader.discovery.gates import AdmissionOutcome
from nautilus_trader.discovery.gates import CalibrationCase
from nautilus_trader.discovery.gates import CandidateState
from nautilus_trader.discovery.gates import ControlClass
from nautilus_trader.discovery.gates import GateEvidence
from nautilus_trader.discovery.gates import GateName
from nautilus_trader.discovery.gates import GatePolicy
from nautilus_trader.discovery.gates import GateService
from nautilus_trader.discovery.gates import GateVerdict
from nautilus_trader.discovery.gates import Lifecycle
from nautilus_trader.discovery.gates import PboRow
from nautilus_trader.discovery.gates import PboState
from nautilus_trader.discovery.gates import TrackingAssumptions
from nautilus_trader.discovery.gates import assess_admission
from nautilus_trader.discovery.gates import evaluate_pbo_matrix
from nautilus_trader.discovery.gates import minimum_track_record_length
from nautilus_trader.discovery.gates import run_calibration
from nautilus_trader.discovery.gates import transition_rule
from nautilus_trader.discovery.ledger import DataRole
from nautilus_trader.optimization.significance import TrialDependence
from nautilus_trader.optimization.splits import LeakagePolicy
from nautilus_trader.optimization.splits import SplitContract


SECOND = 1_000_000_000
CANDIDATE = "sha256:" + "a" * 64
SPECIFICATION = "sha256:specification"
POLICY = "sha256:policy"
COSTS = "sha256:costs"
HOLDOUT = "sha256:holdout"

SPLIT = SplitContract(
    sets=("train", "test"),
    lengths={"train": 6 * SECOND, "test": 4 * SECOND},
    leakage=LeakagePolicy(purge_before=SECOND, purge_after=SECOND, embargo_after=SECOND),
)

# A trial inventory whose cross-trial dispersion is small enough for a strong candidate to be
# distinguishable from the best of the search, which is what G3 asks.
TRIALS = tuple([0.05, -0.05] * 5)


def series(mean: float, spread: float, count: int = 250) -> dict[int, float]:
    """Return an alternating return series with a known mean and dispersion."""
    return {index: mean + (spread if index % 2 else -spread) for index in range(count)}


def evidence(**overrides: object) -> GateEvidence:
    """Return evidence for a candidate that passes every gate, with the overrides applied."""
    base: dict[str, object] = {
        "candidate_id": CANDIDATE,
        "specification_digest": SPECIFICATION,
        "policy_digest": POLICY,
        "price_series_statement": "adjusted closes, no distributions",
        "returns": series(0.01, 0.02),
        "benchmark_returns": tuple(series(0.0, 0.004).values()),
        "cost_configuration": COSTS,
        "data_role": DataRole.LOCKED_VALIDATION,
        "split": SPLIT,
        "fold_returns": tuple([tuple(series(0.01, 0.02, 50).values())] * 4),
        "trial_sharpes": TRIALS,
        "statistical_trials": 10,
        "executed_experiments": 10,
        "name_shares": {"A": 0.5, "B": 0.5},
        "sector_shares": {"tech": 0.6, "energy": 0.4},
        "period_shares": {"2024": 0.5, "2025": 0.5},
        "neighbourhood_objectives": (1.0, 0.95, 0.9),
        "stress_returns": {"doubled costs": 0.004, "overnight gaps": 0.005},
        "drawdown": 0.2,
        "confirmation_evaluation": HOLDOUT,
        "frozen_specification_digest": SPECIFICATION,
        "evidence_package_complete": True,
        "human_decision": "promoted 2026-01-01T00:00:00Z",
    }
    base.update(overrides)

    return GateEvidence(**base)  # type: ignore[arg-type]


def verdict_for(service: GateService, item: GateEvidence) -> GateVerdict:
    """Return the verdict the gate chain reached on one piece of evidence."""
    return service.evaluate(item).outcome


def test_the_chain_passes_a_candidate_that_meets_every_gate() -> None:
    """A conjunction is a pass only when every gate passed, and it reaches them all."""
    report = GateService(GatePolicy()).evaluate(evidence())

    assert report.passed is True
    assert report.outcome is GateVerdict.PASSED
    assert report.stopped_at is None
    assert [record.gate for record in report.verdicts] == list(GateName)
    statistical = report.verdict_for(GateName.G3_STATISTICAL_CREDIBILITY)
    assert statistical is not None
    assert statistical.verdict is GateVerdict.PASSED
    assert report.policy_digest == GatePolicy().digest()
    assert report.evidence_digest == evidence().digest()


def test_a_chain_stops_at_its_first_false_term() -> None:
    """A verdict after the gate that stopped the chain is absent rather than passed."""
    report = GateService(GatePolicy()).evaluate(evidence(timing_violations=("case_a",)))

    assert report.passed is False
    assert report.stopped_at is GateName.G0_DATA_INTEGRITY
    assert report.verdict_for(GateName.G1_ECONOMIC_VIABILITY) is None
    assert [record.gate for record in report.verdicts] == [GateName.G0_DATA_INTEGRITY]


def test_g0_refuses_a_candidate_that_used_what_it_was_not_entitled_to() -> None:
    """A timing violation, an unavailable feature, an entitlement and a silence are each invalid."""
    service = GateService(GatePolicy())

    assert verdict_for(service, evidence(timing_violations=("case_a",))) is GateVerdict.INVALID
    assert (
        verdict_for(service, evidence(feature_availability_defects=("ema",))) is GateVerdict.INVALID
    )
    assert verdict_for(service, evidence(entitlement_defects=("dividend",))) is GateVerdict.INVALID
    assert verdict_for(service, evidence(price_series_statement="")) is GateVerdict.INVALID


def test_g1_holds_the_candidate_to_its_costs_and_its_baseline() -> None:
    """A cost-blind candidate and a baseline beater are judged, and distributions are withheld."""
    service = GateService(GatePolicy())

    assert verdict_for(service, evidence(returns=series(-0.002, 0.02))) is GateVerdict.REJECTED
    assert verdict_for(service, evidence(returns=series(0.002, 0.02))) is GateVerdict.REJECTED
    assert verdict_for(service, evidence(cost_configuration="")) is GateVerdict.INCONCLUSIVE
    assert verdict_for(service, evidence(benchmark_returns=())) is GateVerdict.INCONCLUSIVE
    assert verdict_for(service, evidence(includes_distributions=True)) is GateVerdict.INCONCLUSIVE
    demonstrated = evidence(includes_distributions=True, cash_path_demonstrated=True)
    assert verdict_for(service, demonstrated) is GateVerdict.PASSED


def test_g2_reads_only_locked_validation_data_and_needs_positive_folds() -> None:
    """A search-visible result is inadmissible, and a candidate that does not persist is rejected."""
    service = GateService(GatePolicy())

    assert (
        verdict_for(service, evidence(data_role=DataRole.DEVELOPMENT)) is GateVerdict.INCONCLUSIVE
    )
    assert verdict_for(service, evidence(split=None)) is GateVerdict.INCONCLUSIVE
    assert verdict_for(service, evidence(fold_returns=())) is GateVerdict.INCONCLUSIVE
    losing = tuple([tuple(series(-0.01, 0.02, 50).values())] * 4)
    assert verdict_for(service, evidence(fold_returns=losing)) is GateVerdict.REJECTED


def test_g3_reads_its_multiplicity_from_the_mapping() -> None:
    """A candidate indistinguishable from the best of its search is rejected, not accepted."""
    service = GateService(GatePolicy())

    null = evidence(
        returns=series(0.0005, 0.01),
        trial_sharpes=tuple([0.08, -0.08] * 100),
        statistical_trials=200,
        executed_experiments=200,
    )
    assert verdict_for(service, null) is GateVerdict.REJECTED
    assert verdict_for(service, evidence(statistical_trials=0)) is GateVerdict.INCONCLUSIVE
    assert verdict_for(service, evidence(trial_sharpes=())) is GateVerdict.INCONCLUSIVE
    uncertain = evidence(executed_experiments=20, unusable_attempts=15, statistical_trials=5)
    assert verdict_for(service, uncertain) is GateVerdict.INCONCLUSIVE


def test_g4_refuses_concentration_a_parameter_spike_and_a_cost_stress() -> None:
    """One name, one period, one parameter value and a failed stress are each rejections."""
    service = GateService(GatePolicy())

    assert verdict_for(service, evidence(name_shares={"A": 0.9})) is GateVerdict.REJECTED
    assert verdict_for(service, evidence(period_shares={"2024": 0.8})) is GateVerdict.REJECTED
    spike = evidence(neighbourhood_objectives=(1.0, 0.1, 0.2))
    assert verdict_for(service, spike) is GateVerdict.REJECTED
    stressed = evidence(stress_returns={"doubled costs": -0.001})
    assert verdict_for(service, stressed) is GateVerdict.REJECTED
    assert verdict_for(service, evidence(drawdown=0.9)) is GateVerdict.REJECTED
    assert verdict_for(service, evidence(name_shares={})) is GateVerdict.REJECTED


def test_g5_quarantines_a_leak_and_voids_a_verdict_whose_identity_moved() -> None:
    """Leakage and an accounting defect quarantine, and another specification voids the holdout."""
    service = GateService(GatePolicy())

    quarantine = evidence(quarantine_reasons=("the parameters carry a holdout statistic",))
    assert verdict_for(service, quarantine) is GateVerdict.QUARANTINED

    moved = evidence(frozen_specification_digest="sha256:other")
    assert verdict_for(service, moved) is GateVerdict.QUARANTINED

    assert (
        verdict_for(service, evidence(confirmation_evaluation="")) is GateVerdict.VALIDATION_PENDING
    )
    assert verdict_for(service, evidence(human_decision="")) is GateVerdict.VALIDATION_PENDING
    incomplete = evidence(evidence_package_complete=False)
    assert verdict_for(service, incomplete) is GateVerdict.VALIDATION_PENDING


def test_the_transition_table_permits_only_what_it_names() -> None:
    """Every declared transition has a rule, and a transition the table does not hold is refused."""
    assert len(TRANSITIONS) == 7
    assert transition_rule(CandidateState.PROPOSED, CandidateState.COMPILED) is not None
    assert transition_rule(CandidateState.BACKTESTED, CandidateState.VALIDATED) is None

    lifecycle = Lifecycle(CANDIDATE)
    lifecycle.advance(CandidateState.COMPILED, actor=Actor.COMPILER, evidence={"digest": "x"})
    assert lifecycle.state is CandidateState.COMPILED
    assert lifecycle.history[-1].detail["evidence_required"]

    with pytest.raises(ValueError, match="performed by"):
        lifecycle.advance(CandidateState.BACKTESTED, actor=Actor.OPERATOR)
    with pytest.raises(ValueError, match="not in the table"):
        lifecycle.advance(CandidateState.VALIDATED, actor=Actor.GATE_SERVICE)


def test_a_terminal_candidate_is_never_reopened() -> None:
    """Quarantine and retirement are terminal, and a successor is a new candidate."""
    lifecycle = Lifecycle(CANDIDATE)
    lifecycle.advance(
        CandidateState.QUARANTINED, actor=Actor.GATE_SERVICE, evidence={"why": "leak"}
    )

    assert lifecycle.state.terminal is True
    assert lifecycle.permits(CandidateState.COMPILED, actor=Actor.COMPILER) is False
    with pytest.raises(ValueError, match="terminal"):
        lifecycle.advance(CandidateState.COMPILED, actor=Actor.COMPILER)


def test_a_gate_report_records_an_outcome_without_moving_a_failed_candidate() -> None:
    """A passing report earns the transition, and a stopped one leaves the candidate where it was."""
    service = GateService(GatePolicy())
    failing = Lifecycle(CANDIDATE)
    failing.advance(CandidateState.COMPILED, actor=Actor.COMPILER)
    failing.advance(CandidateState.BACKTESTED, actor=Actor.RUN_ADAPTER)
    failing.apply(service.evaluate(evidence(timing_violations=("case",))))

    assert failing.state is CandidateState.BACKTESTED
    assert failing.outcomes[0].verdict is GateVerdict.INVALID
    assert failing.outcomes[0].gate is GateName.G0_DATA_INTEGRITY
    assert failing.outcomes[0].subject == "backtested"

    passing = Lifecycle(CANDIDATE)
    passing.advance(CandidateState.COMPILED, actor=Actor.COMPILER)
    passing.advance(CandidateState.BACKTESTED, actor=Actor.RUN_ADAPTER)
    passing.apply(service.evaluate(evidence()))

    assert passing.state is CandidateState.VALIDATION_PENDING
    assert passing.history[-1].actor is Actor.GATE_SERVICE


def test_the_minimum_track_record_length_inverts_the_correction() -> None:
    """A stronger claim needs less sample, and an unachievable one has no length at all."""
    assumptions = TrackingAssumptions(target_sharpe=0.3, cross_trial_dispersion=0.05)
    required = minimum_track_record_length(200, assumptions)
    assert required is not None
    assert required > 20

    weaker = minimum_track_record_length(20, assumptions)
    assert weaker is not None
    assert weaker < required

    unachievable = TrackingAssumptions(target_sharpe=0.1, cross_trial_dispersion=0.05)
    assert minimum_track_record_length(200, unachievable) is None
    # The correction itself needs at least ten counted trials, so one has no length.
    assert minimum_track_record_length(1, assumptions) is None


def assessment(**overrides: object) -> AdmissionAssessment:
    """Return an admission assessment over the fixture campaign, with the overrides applied."""
    base: dict[str, object] = {
        "sample_observations": 1500,
        "effective_observations": 1200,
        "label_horizon": 5,
        "planned_configurations": 200,
        "reserved_observations": 120,
        "holdout_fraction": 0.1,
        "target_sharpe": 0.3,
        "cross_trial_dispersion": 0.05,
    }
    base.update(overrides)

    return assess_admission(AdmissionInputs(**base))  # type: ignore[arg-type]


def test_an_admitted_plan_reports_its_arithmetic_and_its_provenance() -> None:
    """An admitted plan carries the relation, its inputs' sources, and the claims it withholds."""
    result = assessment()

    assert result.outcome is AdmissionOutcome.ADMIT
    assert result.required_observations is not None
    assert result.required_observations <= result.usable_observations
    assert result.counted_trials == 200
    assert result.inputs["effective_observations"]["source"].startswith("estimated from")
    assert result.assumptions["cross_trial_dispersion"]["sensitivity"]["factor"] == 0.1
    assert any("profitability" in claim for claim in result.withholds)
    assert "discovery.admission/v1" in result.formula
    assert result.to_dict()["usable_observations"] == result.usable_observations


def test_an_over_budget_plan_narrows_and_a_hopeless_one_is_refused() -> None:
    """A plan beyond its ceiling narrows to the count that fits, and hopeless data is refused."""
    reduced = assessment(target_sharpe=0.16)
    assert reduced.outcome is AdmissionOutcome.REDUCE
    assert reduced.reduced_configurations is not None
    assert 1 <= reduced.reduced_configurations < 200
    assert reduced.required_observations is not None
    assert reduced.required_observations > reduced.usable_observations

    tiny = assessment(effective_observations=20, reserved_observations=0, holdout_fraction=0.0)
    assert tiny.outcome is AdmissionOutcome.REJECT
    assert tiny.reduced_configurations is None
    assert "no allocation fits" in tiny.reason


def test_an_unestimable_input_is_refused_rather_than_defaulted() -> None:
    """A dispersion or an effective count that cannot be estimated is a refusal, never a guess."""
    dispersion = assessment(cross_trial_dispersion=None)
    assert dispersion.outcome is AdmissionOutcome.REJECT
    assert dispersion.required_observations is None
    assert "dispersion" in dispersion.reason

    assert assessment(effective_observations=0).outcome is AdmissionOutcome.REJECT

    dependent = assessment(dependence=TrialDependence.DEPENDENT, effective_trials=None)
    assert dependent.outcome is AdmissionOutcome.REJECT

    declared = assessment(dependence=TrialDependence.DEPENDENT, effective_trials=40)
    assert declared.outcome in (AdmissionOutcome.ADMIT, AdmissionOutcome.REDUCE)
    assert declared.effective_trials == 40


def test_the_probability_of_backtest_overfitting_distinguishes_its_four_states() -> None:
    """A valid matrix reports its statistic with its exclusions, and an unusable one produces none."""
    overfit = {index: (0.05 if index < 40 else -0.01) for index in range(80)}
    steady = dict.fromkeys(range(80), 0.005)
    rows = [
        PboRow("overfit", returns=overfit),
        PboRow("steady", returns=steady),
        PboRow("noise", returns=series(0.0, 0.02, 80)),
        PboRow("ineligible", eligible=False, ineligible_rule="failed the data-integrity gate"),
        PboRow("timed_out", failure_class="timed_out"),
        PboRow("incomplete", returns={0: 0.1, 1: None}),
    ]
    evaluation = evaluate_pbo_matrix(rows)

    assert evaluation.state is PboState.VALID_WITH_EXCLUSIONS
    assert evaluation.computable is True
    assert evaluation.probability is not None
    assert evaluation.probability > 0.0, "a planted overfit row must be visible in the statistic"
    assert evaluation.logit_quantiles["minimum"] < 0.0
    states = {exclusion.state for exclusion in evaluation.exclusions}
    assert states == {PboState.INELIGIBLE, PboState.ELIGIBLE_BUT_FAILED}
    assert evaluation.exclusions[0].reason == "failed the data-integrity gate"
    assert evaluation.to_dict()["diagnostic_only"] is True


def test_an_unusable_matrix_produces_no_statistic() -> None:
    """Too few rows, an incomparable index or too few blocks is not computable before computing."""
    short = evaluate_pbo_matrix([PboRow("a", returns=series(0.01, 0.02, 6))])
    assert short.state is PboState.NOT_COMPUTABLE
    assert short.probability is None

    incomparable = evaluate_pbo_matrix(
        [
            PboRow("a", returns={0: 0.1, 1: 0.1, 2: 0.1, 3: 0.1, 4: 0.1, 5: 0.1, 6: 0.1, 7: 0.1}),
            PboRow("b", returns={1: 0.1, 2: 0.1, 3: 0.1, 4: 0.1, 5: 0.1, 6: 0.1, 7: 0.1, 8: 0.1}),
        ],
    )
    assert incomparable.state is PboState.NOT_COMPUTABLE

    odd = evaluate_pbo_matrix(
        [PboRow("a", returns=series(0.01, 0.02, 80)), PboRow("b", returns=series(0.0, 0.02, 80))],
        blocks=5,
    )
    assert odd.state is PboState.NOT_COMPUTABLE

    all_failed = evaluate_pbo_matrix(
        [PboRow("a", failure_class="crashed"), PboRow("b", failure_class="timed_out")],
    )
    assert all_failed.state is PboState.NOT_COMPUTABLE
    assert len(all_failed.exclusions) == 2


def calibration_cases() -> list[CalibrationCase]:
    """Return one case per control class, each with the gate that should detect it."""
    null = evidence(
        returns=series(0.0005, 0.01),
        trial_sharpes=tuple([0.08, -0.08] * 100),
        statistical_trials=200,
        executed_experiments=200,
    )

    return [
        CalibrationCase(
            name="null",
            control_class=ControlClass.NULL,
            evidence=null,
            expectation="detected",
            expected_gate=GateName.G3_STATISTICAL_CREDIBILITY,
        ),
        CalibrationCase(
            name="positive",
            control_class=ControlClass.POSITIVE,
            evidence=evidence(),
            expectation="accepted",
        ),
        CalibrationCase(
            name="leakage",
            control_class=ControlClass.LEAKAGE,
            evidence=evidence(timing_violations=("case_a",)),
            expectation="detected",
            expected_gate=GateName.G0_DATA_INTEGRITY,
        ),
        CalibrationCase(
            name="cost",
            control_class=ControlClass.COST,
            evidence=evidence(returns=series(-0.002, 0.02)),
            expectation="detected",
            expected_gate=GateName.G1_ECONOMIC_VIABILITY,
        ),
        CalibrationCase(
            name="concentration",
            control_class=ControlClass.CONCENTRATION,
            evidence=evidence(name_shares={"A": 0.9}),
            expectation="detected",
            expected_gate=GateName.G4_ROBUSTNESS,
        ),
        CalibrationCase(
            name="parameter_spike",
            control_class=ControlClass.PARAMETER_SPIKE,
            evidence=evidence(neighbourhood_objectives=(1.0, 0.1, 0.2)),
            expectation="detected",
            expected_gate=GateName.G4_ROBUSTNESS,
        ),
        CalibrationCase(
            name="accounting",
            control_class=ControlClass.ACCOUNTING,
            evidence=evidence(entitlement_defects=("dividend",)),
            expectation="detected",
            expected_gate=GateName.G0_DATA_INTEGRITY,
        ),
    ]


def test_the_calibration_suite_detects_every_control_class_and_accepts_the_positive() -> None:
    """The seven classes are detected by the gate that carries them, with no false accept."""
    report = run_calibration(calibration_cases(), policy=GatePolicy())

    assert report.calibrated is True
    assert report.missed == ()
    assert report.false_accepts == ()
    assert report.false_rejects == ()
    assert set(report.detected_classes) == {
        "null",
        "leakage",
        "cost",
        "concentration",
        "parameter_spike",
        "accounting",
    }
    assert report.policy_digest == GatePolicy().digest()
    assert len(report.outcomes) == 7
    assert report.to_dict()["calibrated"] is True


def test_a_control_the_chain_passes_is_reported_as_missed() -> None:
    """A calibration case whose expectation the chain does not meet is reported, not raised."""
    cases = calibration_cases()
    cases.append(
        CalibrationCase(
            name="should_have_been_detected",
            control_class=ControlClass.NULL,
            evidence=evidence(),
            expectation="detected",
            expected_gate=GateName.G0_DATA_INTEGRITY,
        ),
    )
    report = run_calibration(cases, policy=GatePolicy())

    assert report.calibrated is False
    assert report.missed == ("should_have_been_detected",)
    assert report.false_accepts == ("should_have_been_detected",)
    outcome = next(
        item for item in report.outcomes if item.case.name == "should_have_been_detected"
    )
    assert outcome.agrees is False
    assert outcome.detected_by_expected_gate is False
    assert outcome.observed_gate is None


def test_a_rejected_positive_control_is_a_false_reject() -> None:
    """The other direction of a miscalibrated gate is reported as well."""
    cases = calibration_cases()
    cases[1] = CalibrationCase(
        name="positive",
        control_class=ControlClass.POSITIVE,
        evidence=evidence(returns=series(-0.002, 0.02)),
        expectation="accepted",
    )
    report = run_calibration(cases, policy=GatePolicy())

    assert report.false_rejects == ("positive",)
    assert report.calibrated is False


def test_a_calibration_case_must_name_the_gate_that_detects_it() -> None:
    """An expectation without a gate is not falsifiable, so the case refuses to exist."""
    with pytest.raises(ValueError, match="must name the gate"):
        CalibrationCase(
            name="unfalsifiable",
            control_class=ControlClass.NULL,
            evidence=evidence(),
            expectation="detected",
        )
    with pytest.raises(ValueError, match="detected or accepted"):
        CalibrationCase(
            name="unclear",
            control_class=ControlClass.NULL,
            evidence=evidence(),
            expectation="maybe",
        )


def test_a_policy_states_what_its_thresholds_mean() -> None:
    """A threshold outside its own range is refused rather than silently accepted."""
    with pytest.raises(ValueError, match="minimum_deflated_sharpe"):
        GatePolicy(minimum_deflated_sharpe=1.5)
    with pytest.raises(ValueError, match="maximum_single_name_share"):
        GatePolicy(maximum_single_name_share=0.0)
    with pytest.raises(ValueError, match="maximum_drawdown"):
        GatePolicy(maximum_drawdown=0.0)

    assert GatePolicy().statistical_contract.digest
    assert GatePolicy(version=2).digest() != GatePolicy().digest()


def test_every_gate_asks_the_question_the_design_states() -> None:
    """The gate vocabulary carries its own question, so a report reads as the design reads."""
    for gate in GateName:
        assert gate.question
        assert gate.value.startswith("G")

    assert GateVerdict.PASSED.blocks is False
    assert GateVerdict.INCONCLUSIVE.blocks is True
    assert GateVerdict.QUARANTINED.passed is False
