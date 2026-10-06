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
Tests for the study, trial, dataset and result identity contracts.
"""

import pytest

from nautilus_trader.analysis import Objective
from nautilus_trader.analysis import ObjectiveDirection
from nautilus_trader.analysis import ObjectiveTerm
from nautilus_trader.optimization import BarAmbiguityPolicy
from nautilus_trader.optimization import ComputationIdentity
from nautilus_trader.optimization import DatasetIdentity
from nautilus_trader.optimization import ExecutionStatus
from nautilus_trader.optimization import Experiment
from nautilus_trader.optimization import LeakagePolicy
from nautilus_trader.optimization import ResearchResult
from nautilus_trader.optimization import SelectionRule
from nautilus_trader.optimization import SplitContract
from nautilus_trader.optimization import SplitDirection
from nautilus_trader.optimization import StudyIdentity
from nautilus_trader.optimization import TrialIdentity
from nautilus_trader.optimization import TrialProvenance
from nautilus_trader.optimization import UniverseIdentity
from nautilus_trader.optimization import objective_definition_from_terms
from nautilus_trader.optimization import split_contract_digest


def _universe(**overrides: object) -> UniverseIdentity:
    fields: dict[str, object] = {
        "universe_digest": "sha256:00000000000000000000000000000000000000000000000000000000000000aa",
        "membership_policy_id": "top_500_by_capitalisation",
        "membership_as_of": 1_600_000_000_000_000_000,
    }
    fields.update(overrides)

    return UniverseIdentity(**fields)  # type: ignore[arg-type]


def _dataset(**overrides: object) -> DatasetIdentity:
    fields: dict[str, object] = {
        "dataset_digest": "sha256:00000000000000000000000000000000000000000000000000000000000000bb",
        "universe": _universe(),
        "source_version": "2026-01-01",
        "as_of": 1_600_000_000_000_000_000,
        "calendar_identity": "nautilus-trading-calendar/v1",
        "adjustment_policy": "raw",
        "missing_data_policy": "reject",
    }
    fields.update(overrides)

    return DatasetIdentity(**fields)  # type: ignore[arg-type]


def _objective_definition() -> dict[str, object]:
    terms = [
        ObjectiveTerm("Sharpe Ratio (simple, sample, 252 days)", 1.0, ObjectiveDirection.MAXIMIZE),
        ObjectiveTerm("Max Drawdown (simple)", 1.0, ObjectiveDirection.MAXIMIZE),
    ]

    return objective_definition_from_terms(terms)


def _study(**overrides: object) -> StudyIdentity:
    definition = _objective_definition()
    fields: dict[str, object] = {
        "dataset": _dataset(),
        "parameter_space_digest": "sha256:00000000000000000000000000000000000000000000000000000000000000cc",
        "objective_definition": definition,
        "selection_rule": SelectionRule.RANK_FIRST,
        "metric_set": ("Sharpe Ratio (simple, sample, 252 days)", "Max Drawdown (simple)"),
        "study_seed": 7,
    }
    fields.update(overrides)

    return StudyIdentity(**fields)  # type: ignore[arg-type]


def _trial(study_id: str, *, parameters: object = None) -> TrialIdentity:
    values = {"fast_ema_period": 5} if parameters is None else parameters
    return TrialIdentity(
        study_id=study_id,
        parameter_digest=Experiment(values).digest,  # type: ignore[arg-type]
        parameter_values=values,  # type: ignore[arg-type]
        execution_status=ExecutionStatus.COMPLETED,
        seed=7,
        objective_value=1.5,
        result_digest="blake3:199f",
    )


def _computation(**overrides: object) -> ComputationIdentity:
    fields: dict[str, object] = {
        "code_version": "2.0.0rc6",
        "numeric_kernel_version": "nautilus-analysis-0.65.0",
        "numerical_backend": "rust",
        "parameter_digest": Experiment({"fast_ema_period": 5}).digest,
    }
    fields.update(overrides)

    return ComputationIdentity(**fields)  # type: ignore[arg-type]


def test_a_result_requires_a_study_and_a_trial_of_that_study() -> None:
    """
    Test the requirement is structural rather than conventional.
    """
    study = _study()
    trial = _trial(study.study_id)

    result = ResearchResult(
        study=study,
        trial=trial,
        computation=_computation(),
        assumption_policy=BarAmbiguityPolicy.declared_default(),
    )

    assert result.study.study_id == study.study_id
    assert result.result_digest.startswith("sha256:")

    # A result cannot be built from a string where an identity belongs.
    with pytest.raises(TypeError, match="study must be a StudyIdentity"):
        ResearchResult(
            study=study.study_id,  # type: ignore[arg-type]
            trial=trial,
            computation=_computation(),
            assumption_policy=BarAmbiguityPolicy.declared_default(),
        )

    # A trial of a different study is not a result of this study.
    other = _study(study_seed=99)
    with pytest.raises(ValueError, match="trial belongs to study"):
        ResearchResult(
            study=study,
            trial=_trial(other.study_id),
            computation=_computation(),
            assumption_policy=BarAmbiguityPolicy.declared_default(),
        )


def test_dataset_identity_distinguishes_the_information_state() -> None:
    """
    Test the identity identifies the information state, not only the bytes.
    """
    base = _dataset()
    same = _dataset()
    assert base.digest == same.digest

    # Byte-identical data, a different membership as-of time.
    later = _dataset(universe=_universe(membership_as_of=1_700_000_000_000_000_000))
    assert later.digest != base.digest

    # The same data under a different membership policy, calendar or adjustment policy.
    assert _dataset(universe=_universe(membership_policy_id="all_listed")).digest != base.digest
    assert _dataset(calendar_identity="other-calendar/v1").digest != base.digest
    assert _dataset(adjustment_policy="adjusted").digest != base.digest

    # And a changed dataset identity changes the study's identity.
    assert _study(dataset=later).study_id != _study().study_id


def test_a_study_is_stable_across_a_re_run_and_a_kernel_change_is_not() -> None:
    """
    Test a re-run keeps the study identity while the computation identity moves.
    """
    study = _study()
    assert _study().study_id == study.study_id
    assert _study(objective_definition=_objective_definition()).study_id == study.study_id

    before = _computation()
    after = _computation(numeric_kernel_version="nautilus-analysis-0.66.0")

    assert after.digest != before.digest
    assert _study().study_id == study.study_id

    result_before = ResearchResult(
        study=study,
        trial=_trial(study.study_id),
        computation=before,
        assumption_policy=BarAmbiguityPolicy.declared_default(),
    )
    result_after = ResearchResult(
        study=study,
        trial=_trial(study.study_id),
        computation=after,
        assumption_policy=BarAmbiguityPolicy.declared_default(),
    )

    assert result_before.study.study_id == result_after.study.study_id
    assert result_before.computation.digest != result_after.computation.digest
    assert result_before.result_digest != result_after.result_digest


def test_a_trial_identity_is_sufficient_to_re_run_the_trial() -> None:
    """
    Test the recorded parameters reconstruct the experiment that produced the trial.
    """
    study = _study()
    trial = _trial(study.study_id, parameters={"fast_ema_period": 10, "slow_ema_period": 30})

    reconstructed = Experiment(dict(trial.parameter_values))

    assert reconstructed.digest == trial.parameter_digest
    assert dict(reconstructed.parameters) == dict(trial.parameter_values)
    assert trial.seed == 7
    assert trial.trial_id.startswith("sha256:")

    # A digest that does not describe the recorded values is refused rather than trusted.
    with pytest.raises(ValueError, match="does not digest the parameter values"):
        TrialIdentity(
            study_id=study.study_id,
            parameter_digest="sha256:00000000000000000000000000000000000000000000000000000000000000dd",
            parameter_values={"fast_ema_period": 5},
            execution_status=ExecutionStatus.COMPLETED,
            result_digest="blake3:199f",
        )

    # A completed trial that records no result digest is not a completed trial.
    with pytest.raises(ValueError, match="must record its result digest"):
        TrialIdentity(
            study_id=study.study_id,
            parameter_digest=Experiment({"fast_ema_period": 5}).digest,
            parameter_values={"fast_ema_period": 5},
            execution_status=ExecutionStatus.COMPLETED,
        )


def test_trial_provenance_validates_the_counts() -> None:
    """
    Test the provenance counts are coherent and stay out of the study identity.
    """
    provenance = TrialProvenance(trial_count=500, failed_trial_count=3)
    assert provenance.to_dict()["trial_count"] == 500

    with pytest.raises(ValueError, match="exceeds"):
        TrialProvenance(trial_count=2, failed_trial_count=3)

    with pytest.raises(ValueError, match="must not be negative"):
        TrialProvenance(trial_count=-1)

    # The counts describe the outcome of a study, not the study itself.
    study_id = _study().study_id
    TrialProvenance(trial_count=1, failed_trial_count=1)
    assert _study().study_id == study_id


def test_the_objective_definition_is_canonical_and_sensitive() -> None:
    """
    Test the objective is carried in a declared form the identity depends on.
    """
    definition = _objective_definition()
    terms = definition["terms"]
    assert isinstance(terms, list)
    assert terms[0] == {
        "metric": "Sharpe Ratio (simple, sample, 252 days)",
        "weight": 1.0,
        "direction": "maximize",
    }

    # A different objective is a different study.
    changed = objective_definition_from_terms(
        [
            ObjectiveTerm(
                "Sharpe Ratio (simple, sample, 252 days)", 0.5, ObjectiveDirection.MAXIMIZE
            )
        ],
    )
    assert _study(objective_definition=changed).study_id != _study().study_id

    # And a different selection rule likewise.
    assert _study(selection_rule=SelectionRule.RANK_FIRST_FEASIBLE).study_id != _study().study_id

    # The real objective type converts without depending on a Python string form of its direction.
    objective = Objective(
        [ObjectiveTerm("Max Drawdown (simple)", 1.0, ObjectiveDirection.MAXIMIZE)]
    )
    assert objective_definition_from_terms(objective.terms) == {
        "terms": [{"metric": "Max Drawdown (simple)", "weight": 1.0, "direction": "maximize"}],
    }


def test_a_split_contract_changes_the_study_identity() -> None:
    """
    Test the declared validation contract is part of the study identity.
    """
    leakage = LeakagePolicy(zero_interval_justification="No label horizon is declared.")
    contract = SplitContract(
        sets=("in_sample", "out_of_sample"),
        lengths={"in_sample": 100, "out_of_sample": 50},
        leakage=leakage,
        direction=SplitDirection.EXACT,
    )
    same = SplitContract(
        sets=("in_sample", "out_of_sample"),
        lengths={"in_sample": 100, "out_of_sample": 50},
        leakage=LeakagePolicy(zero_interval_justification="No label horizon is declared."),
        direction=SplitDirection.EXACT,
    )
    other = SplitContract(
        sets=("in_sample", "out_of_sample"),
        lengths={"in_sample": 100, "out_of_sample": 50},
        leakage=leakage,
        direction=SplitDirection.FORWARD,
    )

    assert split_contract_digest(contract) == split_contract_digest(same)
    assert split_contract_digest(contract) != split_contract_digest(other)

    assert _study(split_contract=contract, leakage_policy=leakage).study_id != _study().study_id
    assert (
        _study(split_contract=contract, leakage_policy=leakage).study_id
        != _study(split_contract=other, leakage_policy=leakage).study_id
    )
