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
Identity contracts for a study, a trial, a dataset and a result.

Provenance is part of the meaning of a result: a number that cannot name the study that produced
it, the data it read, the code that computed it and the assumptions it made is an observation, not
a research result. The contracts here make that naming mechanical.

Three levels are distinguished, because collapsing them loses exactly the information a
multiple-testing correction needs: a study contains trials, and knowing that five hundred trials
occurred is not knowing which five hundred.

- `StudyIdentity` is what the study *declares*: the dataset it reads, the split and leakage
  contracts it validates with, the parameter space, the objective, the selection rule, the metric
  set and the seed. It is stable across a re-run.
- `TrialIdentity` is one run of that study: its parameter values and digest, its seed, its
  execution status, its objective value and the digest of the canonical result document.
- `ResearchResult` is the record: a study, a trial, the computation identity and the declared
  assumption policy, plus the metric results.

Two identities are referenced by the study and recorded in their own right:

- `DatasetIdentity` identifies the *information state* available to the study, not merely the bytes
  consumed, because byte-identical files can differ in universe membership, calendar or adjustment
  policy.
- `UniverseIdentity` carries the membership policy and the membership as-of time, because a rule
  evaluated today cannot recover an instrument that has since left the universe.

`TrialProvenance` carries the counts that a correction needs. The counts are deliberately *not* part
of a study's identity: a transient failure would otherwise change the identity of the study that
suffered it, and a set of trials that is "the same study" would stop being comparable.

Nothing here is computed by a statistic or consulted by an order: an identity has no decision
authority. The digests compose from values that already exist where they can: the parameter digest
is the experiment digest, the result digest is the canonical backtest document digest, and the
policy digests are the leakage and assumption policies' own.
"""

from __future__ import annotations

from dataclasses import dataclass
from dataclasses import field
from enum import Enum
from typing import TYPE_CHECKING
from typing import cast

from nautilus_trader.analysis import ObjectiveDirection
from nautilus_trader.optimization.assumptions import BarAmbiguityPolicy
from nautilus_trader.optimization.runner import CanonicalRun
from nautilus_trader.optimization.runner import FailedExperiment
from nautilus_trader.optimization.space import digest_of


if TYPE_CHECKING:
    from collections.abc import Mapping
    from collections.abc import Sequence

    from nautilus_trader.analysis import ObjectiveTerm
    from nautilus_trader.optimization.space import JsonValue
    from nautilus_trader.optimization.splits import LeakagePolicy
    from nautilus_trader.optimization.splits import SplitContract


# The schema of the computation identity, so a change to its shape is visible in a digest.
COMPUTATION_SCHEMA = "nautilus.research.computation/v1"

# The schema of the record this module's identities describe.
RESULT_SCHEMA = "nautilus.research.result/v1"


def _digest(payload: Mapping[str, object]) -> str:
    """
    Return the digest of a JSON-serialisable payload.

    `digest_of` is typed for scalar values, and the payloads here are nested mappings and
    sequences, which `canonical_json` serializes exactly as JSON. The cast states that expectation
    rather than widening the shared parameter type.
    """
    return digest_of(cast("Mapping[str, JsonValue]", payload))


def split_contract_digest(contract: SplitContract) -> str:
    """
    Return the digest of a split contract's declared layout.

    The contract itself carries no digest, so this composes one from the values that define it and
    the digest of the leakage policy it applies.

    Parameters
    ----------
    contract : SplitContract
        The split contract to digest.

    Returns
    -------
    str

    """
    return _digest(
        {
            "sets": list(contract.sets),
            "lengths": dict(contract.lengths),
            "evaluation": contract.evaluation,
            "window": contract.window,
            "stride": contract.stride,
            "min_length": contract.min_length,
            "n_splits": contract.n_splits,
            "direction": contract.direction.value,
            "leakage_digest": contract.leakage.digest,
        },
    )


class SelectionRule(Enum):
    """
    How a study selects its winner from a search.
    """

    RANK_FIRST = "rank_first"
    RANK_FIRST_FEASIBLE = "rank_first_feasible"


class ExecutionStatus(Enum):
    """
    How a trial ended.
    """

    COMPLETED = "completed"
    FAILED = "failed"


@dataclass(frozen=True)
class UniverseIdentity:
    """
    The instrument universe a dataset was read under.

    Parameters
    ----------
    universe_digest : str
        The digest of the membership itself.
    membership_policy_id : str
        The identity of the membership rule that produced it, such as a rule's name.
    membership_as_of : int | None
        The Unix nanosecond time the membership applied at, or None when the membership is
        declared timeless.

    """

    universe_digest: str
    membership_policy_id: str
    membership_as_of: int | None = None

    def __post_init__(self) -> None:
        """
        Validate the declared universe identity.
        """
        if not self.universe_digest:
            raise ValueError("universe_digest must not be empty")
        if not self.membership_policy_id:
            raise ValueError("membership_policy_id must not be empty")

    def to_dict(self) -> dict[str, object]:
        """
        Return the identity as a canonical mapping.

        Returns
        -------
        dict[str, object]

        """
        return {
            "universe_digest": self.universe_digest,
            "membership_policy_id": self.membership_policy_id,
            "membership_as_of": self.membership_as_of,
        }

    @property
    def digest(self) -> str:
        """
        Return the digest of the identity.

        Returns
        -------
        str

        """
        return _digest(self.to_dict())


@dataclass(frozen=True)
class DatasetIdentity:
    """
    The information state a study read.

    Parameters
    ----------
    dataset_digest : str
        The digest of the data the study read, however the caller derives it.
    universe : UniverseIdentity
        The universe the data was read under.
    source_version : str | None
        The source or commit version of the data, or None when undeclared.
    as_of : int | None
        The Unix nanosecond as-of time of the read, or None when the latest state was read.
    calendar_identity : str | None
        The trading calendar the data was read against, or None when undeclared.
    adjustment_policy : str | None
        The corporate-action adjustment policy applied, or None when raw prices were read.
    missing_data_policy : str | None
        The declared treatment of missing data, or None when undeclared.

    """

    dataset_digest: str
    universe: UniverseIdentity
    source_version: str | None = None
    as_of: int | None = None
    calendar_identity: str | None = None
    adjustment_policy: str | None = None
    missing_data_policy: str | None = None

    def __post_init__(self) -> None:
        """
        Validate the declared dataset identity.
        """
        if not self.dataset_digest:
            raise ValueError("dataset_digest must not be empty")
        if not isinstance(self.universe, UniverseIdentity):
            raise TypeError("universe must be a UniverseIdentity")

    def to_dict(self) -> dict[str, object]:
        """
        Return the identity as a canonical mapping.

        Returns
        -------
        dict[str, object]

        """
        return {
            "dataset_digest": self.dataset_digest,
            "source_version": self.source_version,
            "as_of": self.as_of,
            "calendar_identity": self.calendar_identity,
            "instrument_universe_identity": self.universe.to_dict(),
            "adjustment_policy": self.adjustment_policy,
            "missing_data_policy": self.missing_data_policy,
        }

    @property
    def digest(self) -> str:
        """
        Return the digest of the identity.

        Returns
        -------
        str

        """
        return _digest(self.to_dict())


@dataclass(frozen=True)
class ComputationIdentity:
    """
    The code and numerics that computed a result.

    Parameters
    ----------
    code_version : str
        The version of the code that ran, such as the installed package version.
    schema_version : str
        The schema of the computation record itself.
    numeric_kernel_version : str
        The version of the numerical kernel the metrics were computed with. The name is
        distinguished deliberately: `kernel_version` is already the operating system's, and a
        result computed before and after a numerical change is not the same result.
    numerical_backend : str
        The backend that computed the metrics, such as `rust`.
    parameter_digest : str
        The digest of the parameter values the computation ran with.

    """

    code_version: str
    numeric_kernel_version: str
    numerical_backend: str
    parameter_digest: str
    schema_version: str = COMPUTATION_SCHEMA

    def __post_init__(self) -> None:
        """
        Validate the declared computation identity.
        """
        for name in (
            "code_version",
            "schema_version",
            "numeric_kernel_version",
            "numerical_backend",
            "parameter_digest",
        ):
            if not getattr(self, name):
                raise ValueError(f"{name} must not be empty")

    def to_dict(self) -> dict[str, object]:
        """
        Return the identity as a canonical mapping.

        Returns
        -------
        dict[str, object]

        """
        return {
            "code_version": self.code_version,
            "schema_version": self.schema_version,
            "numeric_kernel_version": self.numeric_kernel_version,
            "numerical_backend": self.numerical_backend,
            "parameter_digest": self.parameter_digest,
        }

    @property
    def digest(self) -> str:
        """
        Return the digest of the identity.

        Returns
        -------
        str

        """
        return _digest(self.to_dict())


@dataclass(frozen=True)
class StudyIdentity:
    """
    What a study declares, and nothing about how a single run of it ended.

    Parameters
    ----------
    dataset : DatasetIdentity
        The information state the study reads.
    parameter_space_digest : str
        The digest of the declared parameter space.
    objective_definition : Mapping[str, object]
        The objective, in the canonical form the caller declares: at least the metric, the weight
        and the direction of every term.
    selection_rule : SelectionRule
        How the winner is selected from the search.
    metric_set : Sequence[str]
        The metrics the study reports, by identity.
    split_contract : SplitContract | None
        The validation split, or None when the study runs a single search.
    leakage_policy : LeakagePolicy | None
        The leakage exclusion relation, or None when the study declares none.
    study_seed : int | None
        The study seed, or None when the study declares none.

    """

    dataset: DatasetIdentity
    parameter_space_digest: str
    objective_definition: Mapping[str, object]
    selection_rule: SelectionRule
    metric_set: Sequence[str]
    split_contract: SplitContract | None = None
    leakage_policy: LeakagePolicy | None = None
    study_seed: int | None = None

    def __post_init__(self) -> None:
        """
        Validate the declared study identity.
        """
        if not isinstance(self.dataset, DatasetIdentity):
            raise TypeError("dataset must be a DatasetIdentity")
        if not isinstance(self.selection_rule, SelectionRule):
            raise TypeError("selection_rule must be a SelectionRule")
        if not self.parameter_space_digest:
            raise ValueError("parameter_space_digest must not be empty")
        if not dict(self.objective_definition):
            raise ValueError("objective_definition must not be empty")
        if not list(self.metric_set):
            raise ValueError("metric_set must not be empty")

    def to_dict(self) -> dict[str, object]:
        """
        Return the identity as a canonical mapping.

        Returns
        -------
        dict[str, object]

        """
        return {
            "dataset_identity": self.dataset.to_dict(),
            "parameter_space_digest": self.parameter_space_digest,
            "objective_definition": dict(self.objective_definition),
            "selection_rule": self.selection_rule.value,
            "metric_set": sorted(self.metric_set),
            "split_contract_digest": (
                None if self.split_contract is None else split_contract_digest(self.split_contract)
            ),
            "leakage_policy_digest": (
                self.leakage_policy.digest if self.leakage_policy is not None else None
            ),
            "study_seed": self.study_seed,
        }

    @property
    def study_id(self) -> str:
        """
        Return the digest of the declared study.

        Returns
        -------
        str

        """
        return _digest(self.to_dict())


@dataclass(frozen=True)
class TrialProvenance:
    """
    The counts a correction needs, kept out of the study's identity.

    Parameters
    ----------
    trial_count : int
        The number of trials the study ran.
    failed_trial_count : int
        How many of them failed.
    nominal_and_effective : bool
        Whether the study can distinguish a nominal trial count from an effective one. A sweep over
        twenty adjacent moving-average windows is not twenty independent opportunities, and a
        correction that assumes independence while the study did not provide it is wrong in a way a
        reader cannot see from the result.

    """

    trial_count: int
    failed_trial_count: int = 0
    nominal_and_effective: bool = False

    def __post_init__(self) -> None:
        """
        Validate the counts.
        """
        if self.trial_count < 0:
            raise ValueError("trial_count must not be negative")
        if self.failed_trial_count < 0:
            raise ValueError("failed_trial_count must not be negative")
        if self.failed_trial_count > self.trial_count:
            raise ValueError(
                f"failed_trial_count {self.failed_trial_count} exceeds "
                f"trial_count {self.trial_count}",
            )

    def to_dict(self) -> dict[str, object]:
        """
        Return the record as a canonical mapping.

        Returns
        -------
        dict[str, object]

        """
        return {
            "trial_count": self.trial_count,
            "failed_trial_count": self.failed_trial_count,
            "nominal_and_effective": self.nominal_and_effective,
        }


@dataclass(frozen=True)
class TrialIdentity:
    """
    One trial of a study, enough to re-run it.

    Parameters
    ----------
    study_id : str
        The study the trial belongs to.
    parameter_digest : str
        The digest of the trial's parameter values, which is the experiment digest.
    parameter_values : Mapping[str, object]
        The parameter values themselves, so the trial can be reconstructed rather than compared.
    execution_status : ExecutionStatus
        Whether the trial completed or failed.
    seed : int | None
        The trial seed, or None when the run declared none.
    objective_value : float | None
        The objective value, or None when the trial failed before one was produced.
    result_digest : str | None
        The digest of the canonical result document, or None when there is no result.

    """

    study_id: str
    parameter_digest: str
    parameter_values: Mapping[str, object]
    execution_status: ExecutionStatus
    seed: int | None = None
    objective_value: float | None = None
    result_digest: str | None = None

    def __post_init__(self) -> None:
        """
        Validate the trial identity, including that its digest matches its values.
        """
        if not self.study_id:
            raise ValueError("study_id must not be empty")
        if not self.parameter_digest:
            raise ValueError("parameter_digest must not be empty")
        if not isinstance(self.execution_status, ExecutionStatus):
            raise TypeError("execution_status must be an ExecutionStatus")
        if self.parameter_values and _digest(dict(self.parameter_values)) != self.parameter_digest:
            raise ValueError(
                f"parameter_digest {self.parameter_digest} does not digest the parameter values",
            )
        if self.execution_status is ExecutionStatus.COMPLETED and self.result_digest is None:
            raise ValueError("a completed trial must record its result digest")

    def to_dict(self) -> dict[str, object]:
        """
        Return the identity as a canonical mapping.

        Returns
        -------
        dict[str, object]

        """
        return {
            "study_id": self.study_id,
            "parameter_digest": self.parameter_digest,
            "parameter_values": dict(self.parameter_values),
            "seed": self.seed,
            "execution_status": self.execution_status.value,
            "objective_value": self.objective_value,
            "result_digest": self.result_digest,
        }

    @property
    def trial_id(self) -> str:
        """
        Return the digest of the trial within its study.

        Returns
        -------
        str

        """
        return _digest(
            {
                "study_id": self.study_id,
                "parameter_digest": self.parameter_digest,
                "seed": self.seed,
            },
        )


@dataclass(frozen=True)
class ResearchResult:
    """
    A result record: a trial of a study, the computation behind it and what it assumed.

    Parameters
    ----------
    study : StudyIdentity
        The declared study.
    trial : TrialIdentity
        The trial of that study.
    computation : ComputationIdentity
        The code and numerics that computed the result.
    assumption_policy : BarAmbiguityPolicy
        The bar-derived execution assumptions the run was produced under.
    metric_results : Sequence[str]
        The metric identities the result reports.

    """

    study: StudyIdentity
    trial: TrialIdentity
    computation: ComputationIdentity
    assumption_policy: BarAmbiguityPolicy
    metric_results: Sequence[str] = field(default_factory=tuple)

    def __post_init__(self) -> None:
        """
        Validate the record, including that the trial belongs to the study.
        """
        # Construction-level rather than conventional: a result whose trial does not belong to its
        # study is not a result, and a result without a study cannot be built at all.
        if not isinstance(self.study, StudyIdentity):
            raise TypeError("study must be a StudyIdentity")
        if not isinstance(self.trial, TrialIdentity):
            raise TypeError("trial must be a TrialIdentity")
        if not isinstance(self.computation, ComputationIdentity):
            raise TypeError("computation must be a ComputationIdentity")
        if not isinstance(self.assumption_policy, BarAmbiguityPolicy):
            raise TypeError("assumption_policy must be a BarAmbiguityPolicy")
        if self.trial.study_id != self.study.study_id:
            raise ValueError(
                f"trial belongs to study {self.trial.study_id}, not {self.study.study_id}",
            )

    def to_dict(self) -> dict[str, object]:
        """
        Return the record as a canonical mapping.

        Returns
        -------
        dict[str, object]

        """
        return {
            "schema": RESULT_SCHEMA,
            "study_id": self.study.study_id,
            "study": self.study.to_dict(),
            "trial_id": self.trial.trial_id,
            "trial": self.trial.to_dict(),
            "dataset_identity": self.study.dataset.to_dict(),
            "computation_identity": self.computation.to_dict(),
            "assumption_policy": self.assumption_policy.to_dict(),
            "metric_results": sorted(self.metric_results),
        }

    @property
    def result_digest(self) -> str:
        """
        Return the digest of the record.

        Returns
        -------
        str

        """
        return _digest(self.to_dict())


def objective_definition_from_terms(
    terms: Sequence[ObjectiveTerm],
) -> dict[str, object]:
    """
    Return the canonical objective definition for an objective's terms.

    The objective model is a Rust type over directed metric terms; this converts it to the declared
    form the study identity carries, so the identity does not depend on a Python string form of the
    direction enum.

    Parameters
    ----------
    terms : Sequence[object]
        The objective terms, each exposing `metric`, `weight` and `direction`.

    Returns
    -------
    dict[str, object]

    """
    directions = {
        ObjectiveDirection.MAXIMIZE: "maximize",
        ObjectiveDirection.MINIMIZE: "minimize",
    }

    declared: list[object] = []
    for term in terms:
        direction = directions.get(term.direction)
        if direction is None:
            raise ValueError(f"unsupported objective direction {term.direction!r}")

        declared.append(
            {
                "metric": term.metric,
                "weight": term.weight,
                "direction": direction,
            },
        )

    return {"terms": declared}


def trial_identity(
    study_id: str,
    run: CanonicalRun | FailedExperiment,
    *,
    seed: int | None = None,
    objective_value: float | None = None,
) -> TrialIdentity:
    """
    Return the trial identity of a run of a study.

    Parameters
    ----------
    study_id : str
        The study the run belongs to.
    run : CanonicalRun | FailedExperiment
        The run, as the optimizer records it.
    seed : int | None, default None
        The seed the run declared, or None.
    objective_value : float | None, default None
        The objective value of the run, or None when it produced none.

    Returns
    -------
    TrialIdentity

    """
    experiment = run.experiment
    completed = not isinstance(run, FailedExperiment)

    return TrialIdentity(
        study_id=study_id,
        parameter_digest=experiment.digest,
        parameter_values=dict(experiment.parameters),
        execution_status=(ExecutionStatus.COMPLETED if completed else ExecutionStatus.FAILED),
        seed=seed,
        objective_value=objective_value,
        result_digest=run.canonical_digest if completed else None,
    )
