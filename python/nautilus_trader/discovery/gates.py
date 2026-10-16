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
The gate service, the candidate lifecycle, the admission assessment and the calibration suite.

The gates of design Section 11.1 run in order and each is a conjunction: none is compensable by a
later gate's strength, and nothing here is a composite score whose weights a caller could move.
Every threshold lives in the pre-registered `GatePolicy`, which is versioned and digest-pinned, and
the service reads that policy and nothing else. A gate that cannot reach a verdict because an input
is missing reports `INCONCLUSIVE` or `REJECTED` with the input named, never a defaulted number.

Three things make the vocabulary operational rather than decorative, and they are the three the
design asks for by name. The candidate lifecycle of Section 11.4 is a transition table rather than
a status field, so a transition the table does not permit is refused. The admission assessment of
Section 7.4 inverts the published minimum-backtest-length relation over the declared inputs and
returns `ADMIT`, `REDUCE` or `REJECT` with its arithmetic, its provenance and each input's
sensitivity. And the probability of backtest overfitting of Section 7.2 is evaluated against its
preconditions before it is computed, with its four states reported.

The calibration suite is what makes "the gate is calibrated, not assumed" checkable: it runs the
whole chain over cases with known answers and reports which controls each gate detected and which it
missed, with the false-accept and false-reject behaviour at the pre-registered thresholds.
"""

from __future__ import annotations

import itertools
import math
from dataclasses import dataclass
from dataclasses import field
from dataclasses import replace
from enum import Enum
from typing import TYPE_CHECKING

from nautilus_trader.discovery.ledger import DataRole
from nautilus_trader.discovery.schema import canonical_mapping
from nautilus_trader.optimization.identity import split_contract_digest
from nautilus_trader.optimization.significance import SharpeFrequency
from nautilus_trader.optimization.significance import SharpeSample
from nautilus_trader.optimization.significance import StatisticalContract
from nautilus_trader.optimization.significance import TrialDependence
from nautilus_trader.optimization.significance import deflated_sharpe_ratio
from nautilus_trader.optimization.significance import per_period_sharpe
from nautilus_trader.optimization.significance import return_moments
from nautilus_trader.optimization.space import digest_of


if TYPE_CHECKING:
    from collections.abc import Callable
    from collections.abc import Mapping
    from collections.abc import Sequence

    from nautilus_trader.discovery.custody import ConfirmationCustody
    from nautilus_trader.discovery.ledger import CampaignInventory
    from nautilus_trader.optimization.splits import SplitContract


# The identity of the pre-registered gate thresholds, which every trial records.
GATE_POLICY_ID = "discovery.gates"
GATE_POLICY_VERSION = 1

# The identity of the inverted relation the admission assessment computes.
ADMISSION_VERSION = "discovery.admission/v1"

# What every declared undeflated input means, and the gates that report an outcome rather than a
# rejection when one is missing.
_MISSING = "inputs_missing"

# The sample-length bounds the inverted relation searches between: below four periods nothing is
# estimable, and above ten million periods the search is not describing a research question.
_MINIMUM_SAMPLE = 4
_MAXIMUM_SAMPLE = 10_000_000

# The fewest comparable rows and the fewest blocks a matrix needs before a statistic means anything.
_PBO_MINIMUM_SURVIVORS = 2
_PBO_MINIMUM_BLOCKS = 4

# The fewest objectives a parameter neighbourhood needs: the selected value and one neighbour.
_MINIMUM_NEIGHBOURS = 2


class GateName(Enum):
    """
    The gates, in the order they run, each with the question it asks.
    """

    G0_DATA_INTEGRITY = "G0"
    G1_ECONOMIC_VIABILITY = "G1"
    G2_WALK_FORWARD = "G2"
    G3_STATISTICAL_CREDIBILITY = "G3"
    G4_ROBUSTNESS = "G4"
    G5_HOLDOUT_READINESS = "G5"

    @property
    def question(self) -> str:
        """
        The question the gate asks, as the design states it.
        """
        questions = {
            GateName.G0_DATA_INTEGRITY: "Was the candidate entitled to what it used, and when?",
            GateName.G1_ECONOMIC_VIABILITY: "Does it survive its costs and beat a baseline?",
            GateName.G2_WALK_FORWARD: "Does it persist out of sample across time?",
            GateName.G3_STATISTICAL_CREDIBILITY: "Is it distinguishable from this search's best?",
            GateName.G4_ROBUSTNESS: "Does it depend on a lucky name, period, or value?",
            GateName.G5_HOLDOUT_READINESS: "Does it hold on unseen data, and is it frozen?",
        }

        return questions[self]


class GateVerdict(Enum):
    """
    The verdict vocabulary the gates and the lifecycle produce.

    `PASSED` is a gate's own pass. `VALIDATION_PENDING` and `QUARANTINED` are G5's outcomes, and
    `INVALID`, `REJECTED` and `INCONCLUSIVE` are the resting outcomes the design keeps separate: a
    candidate that failed economically is not the same as one the search simply lacked the
    observations to say anything about.
    """

    PASSED = "passed"
    INVALID = "invalid"
    REJECTED = "rejected"
    INCONCLUSIVE = "inconclusive"
    VALIDATION_PENDING = "validation_pending"
    QUARANTINED = "quarantined"

    @property
    def passed(self) -> bool:
        """
        Whether the verdict advances a candidate.
        """
        return self is GateVerdict.PASSED

    @property
    def blocks(self) -> bool:
        """
        Whether the verdict stops the chain, which every verdict but a pass does.
        """
        return not self.passed


@dataclass(frozen=True)
class GatePolicy:
    """
    The pre-registered thresholds, costs, baseline and limits the gate service reads.

    The policy is versioned and published with its digest before a campaign opens, and a campaign
    whose results are reported against a policy edited afterwards is invalid.

    Parameters
    ----------
    minimum_deflated_sharpe : float, default 0.95
        The deflated Sharpe ratio a candidate must reach to pass G3.
    minimum_net_sharpe : float, default 0.0
        The per-period net Sharpe ratio a candidate must exceed to pass G1.
    minimum_baseline_margin : float, default 0.0
        How far the candidate must exceed the registered baseline in the same universe.
    maximum_single_name_share : float, default 0.5
        The largest share of the result a single instrument may contribute.
    maximum_single_sector_share : float, default 0.75
        The largest share a single sector may contribute.
    maximum_single_period_share : float, default 0.5
        The largest share a single period may contribute.
    neighbourhood_tolerance : float, default 0.25
        How far a neighbouring parameter's objective may fall below the selected one.
    minimum_neighbourhood_agreement : float, default 0.5
        The share of neighbours that must reproduce within the tolerance.
    maximum_drawdown : float, default 0.5
        The largest drawdown magnitude a candidate may carry.
    minimum_positive_fold_share : float, default 0.5
        The share of walk-forward folds that must be positive for G2.
    maximum_unusable_share : float, default 0.25
        The share of closed attempts that may produce no usable return series before the
        statistical count is `INCONCLUSIVE` on that ground alone.
    contract : StatisticalContract | None, default None
        The statistical contract the correction is computed under, or None for the declared default.
    policy_id : str, default `GATE_POLICY_ID`
        The stable identity of the policy.
    version : int, default `GATE_POLICY_VERSION`
        The policy version, which changes when any threshold above changes.

    """

    minimum_deflated_sharpe: float = 0.95
    minimum_net_sharpe: float = 0.0
    minimum_baseline_margin: float = 0.0
    maximum_single_name_share: float = 0.5
    maximum_single_sector_share: float = 0.75
    maximum_single_period_share: float = 0.5
    neighbourhood_tolerance: float = 0.25
    minimum_neighbourhood_agreement: float = 0.5
    maximum_drawdown: float = 0.5
    minimum_positive_fold_share: float = 0.5
    maximum_unusable_share: float = 0.25
    contract: StatisticalContract | None = None
    policy_id: str = GATE_POLICY_ID
    version: int = GATE_POLICY_VERSION

    def __post_init__(self) -> None:
        """
        Validate the declared thresholds, so a policy states what it means.
        """
        for name in ("maximum_single_name_share", "maximum_single_sector_share"):
            value = float(getattr(self, name))
            if not 0.0 < value <= 1.0:
                raise ValueError(f"{name} must be a share in (0, 1], was {value}")
        for name in (
            "maximum_single_period_share",
            "minimum_neighbourhood_agreement",
            "minimum_positive_fold_share",
            "maximum_unusable_share",
        ):
            value = float(getattr(self, name))
            if not 0.0 <= value <= 1.0:
                raise ValueError(f"{name} must be a share in [0, 1], was {value}")
        if not 0.0 < self.minimum_deflated_sharpe < 1.0:
            raise ValueError(
                f"minimum_deflated_sharpe must be in (0, 1), was {self.minimum_deflated_sharpe}"
            )
        if self.maximum_drawdown <= 0.0:
            raise ValueError(f"maximum_drawdown must be positive, was {self.maximum_drawdown}")

    @property
    def statistical_contract(self) -> StatisticalContract:
        """
        The statistical contract in force, which defaults to the declared one.
        """
        if self.contract is None:
            return StatisticalContract.declared_default()

        return self.contract

    def to_dict(self) -> dict[str, object]:
        """
        Return the policy as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "policy_id": self.policy_id,
            "version": self.version,
            "minimum_deflated_sharpe": self.minimum_deflated_sharpe,
            "minimum_net_sharpe": self.minimum_net_sharpe,
            "minimum_baseline_margin": self.minimum_baseline_margin,
            "maximum_single_name_share": self.maximum_single_name_share,
            "maximum_single_sector_share": self.maximum_single_sector_share,
            "maximum_single_period_share": self.maximum_single_period_share,
            "neighbourhood_tolerance": self.neighbourhood_tolerance,
            "minimum_neighbourhood_agreement": self.minimum_neighbourhood_agreement,
            "maximum_drawdown": self.maximum_drawdown,
            "minimum_positive_fold_share": self.minimum_positive_fold_share,
            "maximum_unusable_share": self.maximum_unusable_share,
            "contract_digest": self.statistical_contract.digest,
        }

    def digest(self) -> str:
        """
        Return the digest of the policy, which every trial records.

        Returns
        -------
        str
            The `sha256:<hex>` digest.

        """
        return digest_of(canonical_mapping(self.to_dict()))


@dataclass(frozen=True)
class GateEvidence:
    """
    What the gates are evaluated on, declared rather than inferred.

    Every field is a declaration the caller makes about the candidate. A field a gate needs and the
    caller does not declare is reported as a missing input, which is why most of them have no
    default that would let an unestimable quantity look estimated.

    Parameters
    ----------
    candidate_id : str
        The candidate the evidence is about, which is its behavioural identity.
    specification_digest : str
        The digest of the specification the evidence was produced under.
    policy_digest : str, default ""
        The digest of the policy in force when the evidence was produced.
    data_role : DataRole, default DEVELOPMENT
        The role the data behind the result plays.
    timing_violations : tuple[str, ...], default ()
        The planted timing cases the candidate violated, from the case table.
    feature_availability_defects : tuple[str, ...], default ()
        Features whose availability or warm-up the run did not honour.
    entitlement_defects : tuple[str, ...], default ()
        Corporate-action entitlements the accounting did not reconcile.
    price_series_statement : str, default ""
        The statement the run makes about the price series it consumed.
    returns : Mapping[int, float | None], default {}
        The candidate's net per-period returns, keyed by timestamp.
    benchmark_returns : tuple[float | None, ...], default ()
        The registered baseline's returns over the same universe.
    cost_configuration : str, default ""
        The digest of the cost configuration the returns are net of.
    includes_distributions : bool, default False
        Whether the candidate's return depends on distributions.
    cash_path_demonstrated : bool, default False
        Whether the corporate-action cash path is demonstrated for the instruments involved.
    split : SplitContract | None, default None
        The split contract the walk-forward evaluation used.
    fold_returns : tuple[tuple[float | None, ...], ...], default ()
        One net return series per walk-forward fold.
    trial_sharpes : tuple[float, ...], default ()
        The per-period Sharpe ratios of every trial that produced an estimate.
    dependence : TrialDependence, default INDEPENDENT
        Whether the trials are independent draws.
    effective_trials : int | None, default None
        The effective trial count, required when the trials are dependent.
    statistical_trials : int, default 0
        The statistical trial count the mapping produced.
    executed_experiments : int, default 0
        Every attempt that closed for this campaign, whatever its outcome.
    unusable_attempts : int, default 0
        How many of them produced no usable return series.
    permutation_p_value : float | None, default None
        The permutation test's p-value, reported as a supplementary diagnostic.
    name_shares : Mapping[str, float], default {}
        The share of the result each instrument contributed.
    sector_shares : Mapping[str, float], default {}
        The share each sector contributed.
    period_shares : Mapping[str, float], default {}
        The share each period contributed.
    neighbourhood_objectives : tuple[float, ...], default ()
        The objective at the selected parameter value and at its declared neighbours.
    stress_returns : Mapping[str, float], default {}
        The mean net return under each declared stress scenario.
    drawdown : float, default 0.0
        The drawdown magnitude the run reported.
    confirmation_evaluation : str, default ""
        The recorded holdout evaluation's identity.
    frozen_specification_digest : str, default ""
        The digest of the frozen specification the holdout evaluation was made under.
    evidence_package_complete : bool, default False
        Whether the candidate's evidence package is complete.
    human_decision : str, default ""
        The human promotion decision and its timestamp.
    quarantine_reasons : tuple[str, ...], default ()
        Evidence of confirmation-set leakage or of an accounting defect.

    """

    candidate_id: str
    specification_digest: str
    policy_digest: str = ""
    data_role: DataRole = DataRole.DEVELOPMENT
    timing_violations: tuple[str, ...] = ()
    feature_availability_defects: tuple[str, ...] = ()
    entitlement_defects: tuple[str, ...] = ()
    price_series_statement: str = ""
    returns: Mapping[int, float | None] = field(default_factory=dict)
    benchmark_returns: tuple[float | None, ...] = ()
    cost_configuration: str = ""
    includes_distributions: bool = False
    cash_path_demonstrated: bool = False
    split: SplitContract | None = None
    fold_returns: tuple[tuple[float | None, ...], ...] = ()
    trial_sharpes: tuple[float, ...] = ()
    dependence: TrialDependence = TrialDependence.INDEPENDENT
    effective_trials: int | None = None
    statistical_trials: int = 0
    executed_experiments: int = 0
    unusable_attempts: int = 0
    permutation_p_value: float | None = None
    name_shares: Mapping[str, float] = field(default_factory=dict)
    sector_shares: Mapping[str, float] = field(default_factory=dict)
    period_shares: Mapping[str, float] = field(default_factory=dict)
    neighbourhood_objectives: tuple[float, ...] = ()
    stress_returns: Mapping[str, float] = field(default_factory=dict)
    drawdown: float = 0.0
    confirmation_evaluation: str = ""
    frozen_specification_digest: str = ""
    evidence_package_complete: bool = False
    human_decision: str = ""
    quarantine_reasons: tuple[str, ...] = ()

    def __post_init__(self) -> None:
        """
        Validate that the evidence names the candidate it is about.
        """
        if not self.candidate_id:
            raise ValueError("candidate_id must not be empty")

    def to_dict(self) -> dict[str, object]:
        """
        Return the evidence as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "candidate_id": self.candidate_id,
            "specification_digest": self.specification_digest,
            "policy_digest": self.policy_digest,
            "data_role": self.data_role.value,
            "timing_violations": list(self.timing_violations),
            "feature_availability_defects": list(self.feature_availability_defects),
            "entitlement_defects": list(self.entitlement_defects),
            "price_series_statement": self.price_series_statement,
            "returns": {str(key): value for key, value in self.returns.items()},
            "benchmark_returns": list(self.benchmark_returns),
            "cost_configuration": self.cost_configuration,
            "includes_distributions": self.includes_distributions,
            "cash_path_demonstrated": self.cash_path_demonstrated,
            "split_digest": None if self.split is None else split_contract_digest(self.split),
            "folds": len(self.fold_returns),
            "trial_sharpes": list(self.trial_sharpes),
            "dependence": self.dependence.value,
            "effective_trials": self.effective_trials,
            "statistical_trials": self.statistical_trials,
            "executed_experiments": self.executed_experiments,
            "unusable_attempts": self.unusable_attempts,
            "permutation_p_value": self.permutation_p_value,
            "name_shares": dict(self.name_shares),
            "sector_shares": dict(self.sector_shares),
            "period_shares": dict(self.period_shares),
            "neighbourhood_objectives": list(self.neighbourhood_objectives),
            "stress_returns": dict(self.stress_returns),
            "drawdown": self.drawdown,
            "confirmation_evaluation": self.confirmation_evaluation,
            "frozen_specification_digest": self.frozen_specification_digest,
            "evidence_package_complete": self.evidence_package_complete,
            "human_decision": self.human_decision,
            "quarantine_reasons": list(self.quarantine_reasons),
        }

    def digest(self) -> str:
        """
        Return the digest of the declared evidence.

        Returns
        -------
        str
            The `sha256:<hex>` digest.

        """
        return digest_of(canonical_mapping(self.to_dict()))


@dataclass(frozen=True)
class GateVerdictRecord:
    """
    One gate's verdict on one candidate.

    Parameters
    ----------
    gate : GateName
        The gate that reached it.
    verdict : GateVerdict
        The verdict.
    reason : str
        Why, in terms of the numbers rather than in prose.
    evidence : Mapping[str, object]
        The quantities the verdict rests on.

    """

    gate: GateName
    verdict: GateVerdict
    reason: str
    evidence: Mapping[str, object] = field(default_factory=dict)

    def to_dict(self) -> dict[str, object]:
        """
        Return the record as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "gate": self.gate.value,
            "verdict": self.verdict.value,
            "reason": self.reason,
            "evidence": dict(self.evidence),
        }


@dataclass(frozen=True)
class GateReport:
    """
    The conjunction the gates make, in the order they were asked.

    The verdicts after the first non-pass are absent rather than passed, because a conjunction stops
    at its first false term and reporting a later gate as passed would present a result the run
    never reached.

    Parameters
    ----------
    candidate_id : str
        The candidate the report is about.
    verdicts : tuple[GateVerdictRecord, ...]
        Every verdict reached, in gate order.
    policy_digest : str
        The digest of the policy the verdicts were reached under.
    evidence_digest : str
        The digest of the evidence they were reached on.
    outcome : GateVerdict
        The conjunction's outcome: a pass only when every gate passed.

    """

    candidate_id: str
    verdicts: tuple[GateVerdictRecord, ...]
    policy_digest: str
    evidence_digest: str
    outcome: GateVerdict

    @property
    def passed(self) -> bool:
        """
        Whether every gate passed.
        """
        return self.outcome.passed

    @property
    def stopped_at(self) -> GateName | None:
        """
        The gate the chain stopped at, or None when every gate passed.
        """
        for record in self.verdicts:
            if not record.verdict.passed:
                return record.gate

        return None

    def verdict_for(self, gate: GateName) -> GateVerdictRecord | None:
        """
        Return a gate's verdict, or None when the chain never reached it.

        Parameters
        ----------
        gate : GateName
            The gate to read.

        Returns
        -------
        GateVerdictRecord | None

        """
        for record in self.verdicts:
            if record.gate is gate:
                return record

        return None

    def to_dict(self) -> dict[str, object]:
        """
        Return the report as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "candidate_id": self.candidate_id,
            "outcome": self.outcome.value,
            "stopped_at": None if self.stopped_at is None else self.stopped_at.value,
            "policy_digest": self.policy_digest,
            "evidence_digest": self.evidence_digest,
            "verdicts": [record.to_dict() for record in self.verdicts],
        }


class GateService:
    """
    The gate service: a conjunction of six gates, evaluated against one pre-registered policy.

    Every verdict is written to the campaign inventory, so a verdict is evidence the ledger holds
    rather than a claim a report makes. The service reads its policy and nothing else, which is what
    makes a threshold a pre-registered decision rather than a parameter a caller tunes.

    Parameters
    ----------
    policy : GatePolicy
        The pre-registered thresholds.
    inventory : CampaignInventory | None, default None
        The campaign inventory every verdict is written to, or None when the caller keeps its own
        record and the service writes nothing.
    custody : ConfirmationCustody | None, default None
        The campaign's custody machine, or None when the campaign confirms nothing.

    """

    def __init__(
        self,
        policy: GatePolicy,
        *,
        inventory: CampaignInventory | None = None,
        custody: ConfirmationCustody | None = None,
    ) -> None:
        """
        Initialize the service over a policy and a campaign's records.
        """
        self._policy = policy
        self._inventory = inventory
        self._custody = custody

    @property
    def policy(self) -> GatePolicy:
        """
        The pre-registered policy the service reads.
        """
        return self._policy

    def evaluate(self, evidence: GateEvidence) -> GateReport:
        """
        Evaluate the gates in order, stopping at the first one that does not pass.

        Parameters
        ----------
        evidence : GateEvidence
            The declared evidence.

        Returns
        -------
        GateReport

        """
        evaluated = (
            (GateName.G0_DATA_INTEGRITY, self._gate_data_integrity),
            (GateName.G1_ECONOMIC_VIABILITY, self._gate_economic_viability),
            (GateName.G2_WALK_FORWARD, self._gate_walk_forward),
            (GateName.G3_STATISTICAL_CREDIBILITY, self._gate_statistical_credibility),
            (GateName.G4_ROBUSTNESS, self._gate_robustness),
            (GateName.G5_HOLDOUT_READINESS, self._gate_holdout_readiness),
        )

        reached: list[GateVerdictRecord] = []
        outcome = GateVerdict.PASSED
        for gate, evaluate in evaluated:
            verdict, reason, details = evaluate(evidence)
            record = GateVerdictRecord(gate=gate, verdict=verdict, reason=reason, evidence=details)
            reached.append(record)
            self._write(evidence, record)
            if record.verdict.blocks:
                outcome = record.verdict
                break

        return GateReport(
            candidate_id=evidence.candidate_id,
            verdicts=tuple(reached),
            policy_digest=self._policy.digest(),
            evidence_digest=evidence.digest(),
            outcome=outcome,
        )

    def _write(self, evidence: GateEvidence, record: GateVerdictRecord) -> None:
        """
        Write a verdict to the inventory, when the service owns one.
        """
        if self._inventory is None:
            return

        self._inventory.record_gate_verdict(
            evidence.candidate_id,
            gate=record.gate.value,
            status=record.verdict.value,
            evidence_digest=evidence.digest(),
            reason=record.reason,
        )

    def _gate_data_integrity(
        self,
        evidence: GateEvidence,
    ) -> tuple[GateVerdict, str, Mapping[str, object]]:
        """
        G0: entitlement to the information used, at the time it was used.
        """
        defects: list[str] = []
        if evidence.timing_violations:
            defects.append(f"timing cases violated: {sorted(evidence.timing_violations)}")
        if evidence.feature_availability_defects:
            defects.append(f"features unavailable: {sorted(evidence.feature_availability_defects)}")
        if evidence.entitlement_defects:
            defects.append(f"entitlements unreconciled: {sorted(evidence.entitlement_defects)}")
        if not evidence.price_series_statement:
            defects.append("no price-series statement: what the run consumed is unknown")

        details: dict[str, object] = {
            "timing_violations": len(evidence.timing_violations),
            "feature_availability_defects": len(evidence.feature_availability_defects),
            "entitlement_defects": len(evidence.entitlement_defects),
            "price_series_statement": evidence.price_series_statement,
        }

        if defects:
            return GateVerdict.INVALID, "; ".join(defects), details

        return GateVerdict.PASSED, "the run was entitled to what it consumed", details

    def _gate_economic_viability(
        self,
        evidence: GateEvidence,
    ) -> tuple[GateVerdict, str, Mapping[str, object]]:
        """
        G1: survival of its own costs, against a registered baseline.
        """
        missing = [
            name
            for name, present in (
                ("returns", bool(evidence.returns)),
                ("cost_configuration", bool(evidence.cost_configuration)),
                ("benchmark_returns", bool(evidence.benchmark_returns)),
            )
            if not present
        ]
        if missing:
            reason = f"the candidate declares no {missing}, so nothing can be compared"
            return GateVerdict.INCONCLUSIVE, reason, {_MISSING: missing}

        estimate = per_period_sharpe(dict(evidence.returns))
        baseline = per_period_sharpe(list(evidence.benchmark_returns))
        details: dict[str, object] = {
            "net_sharpe_per_period": estimate.value,
            "baseline_sharpe_per_period": baseline.value,
            "observations": estimate.observations,
        }

        if evidence.includes_distributions and not evidence.cash_path_demonstrated:
            reason = "the return includes distributions and the cash path is not demonstrated"
            return GateVerdict.INCONCLUSIVE, reason, details

        if estimate.value is None or baseline.value is None:
            reason = "the candidate or the baseline produced no estimate, so neither is comparable"
            return GateVerdict.INCONCLUSIVE, reason, details

        margin = estimate.value - baseline.value
        details["margin"] = margin
        if (
            estimate.value <= self._policy.minimum_net_sharpe
            or margin < self._policy.minimum_baseline_margin
        ):
            reason = f"the net Sharpe {estimate.value} and margin {margin} fail the declared floors"
            return GateVerdict.REJECTED, reason, details

        return (
            GateVerdict.PASSED,
            "the candidate survives its costs and beats its baseline",
            details,
        )

    def _gate_walk_forward(
        self,
        evidence: GateEvidence,
    ) -> tuple[GateVerdict, str, Mapping[str, object]]:
        """
        G2: persistence out of sample, over locked validation data.
        """
        details: dict[str, object] = {
            "data_role": evidence.data_role.value,
            "folds": len(evidence.fold_returns),
            "split_declared": evidence.split is not None,
        }

        if not evidence.data_role.admissible_as_validation:
            role = evidence.data_role.value
            reason = f"the data role is {role} and only locked validation data is admissible"
            return GateVerdict.INCONCLUSIVE, reason, details
        absent = [
            name
            for name, present in (
                ("split", evidence.split is not None),
                ("fold_returns", bool(evidence.fold_returns)),
            )
            if not present
        ]
        if absent:
            reason = f"the walk-forward evaluation does not declare {absent}"
            return GateVerdict.INCONCLUSIVE, reason, {_MISSING: absent}

        means: list[float] = []
        positive = 0
        for fold in evidence.fold_returns:
            values = [value for value in fold if value is not None]
            mean = math.fsum(values) / len(values) if values else 0.0
            means.append(mean)
            positive += 1 if mean > 0.0 else 0

        share = positive / len(evidence.fold_returns)
        details = {**details, "folds_positive": positive, "positive_share": share, "means": means}
        if share < self._policy.minimum_positive_fold_share:
            reason = f"{positive} of {len(means)} folds are positive, below the declared share"
            return GateVerdict.REJECTED, reason, details

        return GateVerdict.PASSED, "the candidate persists out of sample across the folds", details

    def _gate_statistical_credibility(
        self,
        evidence: GateEvidence,
    ) -> tuple[GateVerdict, str, Mapping[str, object]]:
        """
        G3: distinguishability from the best of a search this size.
        """
        details: dict[str, object] = {
            "statistical_trials": evidence.statistical_trials,
            "executed_experiments": evidence.executed_experiments,
            "unusable_attempts": evidence.unusable_attempts,
            "trial_estimates": len(evidence.trial_sharpes),
        }

        if evidence.executed_experiments > 0:
            unusable_share = evidence.unusable_attempts / evidence.executed_experiments
            details["unusable_share"] = unusable_share
            if unusable_share > self._policy.maximum_unusable_share:
                unusable = evidence.unusable_attempts
                executed = evidence.executed_experiments
                reason = f"{unusable} of {executed} attempts produced no usable return series"
                return GateVerdict.INCONCLUSIVE, reason, details

        absent = [
            name
            for name, present in (
                ("statistical_trials", evidence.statistical_trials >= 1),
                ("trial_sharpes", bool(evidence.trial_sharpes)),
            )
            if not present
        ]
        if absent:
            reason = f"the inventory declares no {absent}, so there is no multiplicity to read"
            return GateVerdict.INCONCLUSIVE, reason, {_MISSING: absent}

        estimate = per_period_sharpe(dict(evidence.returns))
        moments = return_moments(dict(evidence.returns))
        if estimate.value is None or moments is None:
            reason = "the selected estimate or its moments are undefined for this series"
            details["candidate_sharpe"] = estimate.value
            details["moments"] = moments is not None
            return GateVerdict.INCONCLUSIVE, reason, details

        sample = SharpeSample(
            sharpe=estimate.value,
            trial_sharpes=list(evidence.trial_sharpes),
            observations=estimate.observations,
            skew=moments.skew,
            kurtosis=moments.kurtosis,
            dependence=evidence.dependence,
            effective_trials=evidence.effective_trials,
            frequency=SharpeFrequency.PER_PERIOD,
        )
        result = deflated_sharpe_ratio(sample, contract=self._policy.statistical_contract)
        recorded = result.to_dict()
        details = {
            **details,
            "candidate_sharpe": estimate.value,
            "nominal_trials": sample.nominal_trials,
            "counted_trials": sample.counted_trials,
            "deflated_sharpe": result.value,
            "status": recorded["status"],
            "reason": recorded["reason"],
            "permutation_p_value": evidence.permutation_p_value,
        }

        if not result.is_computed:
            reason = f"the deflated Sharpe ratio was not computable ({details['reason']})"
            return GateVerdict.INCONCLUSIVE, reason, details
        if result.value is None or result.value < self._policy.minimum_deflated_sharpe:
            reason = f"the deflated Sharpe ratio {result.value} is below the declared threshold"
            return GateVerdict.REJECTED, reason, details

        reason = f"the result is distinguishable from the best of {sample.counted_trials} trials"
        return GateVerdict.PASSED, reason, details

    def _gate_robustness(
        self,
        evidence: GateEvidence,
    ) -> tuple[GateVerdict, str, Mapping[str, object]]:
        """
        G4: independence from a lucky name, period, or parameter value.
        """
        details: dict[str, object] = {}
        defects: list[str] = []

        for label, shares, ceiling in (
            ("instrument", evidence.name_shares, self._policy.maximum_single_name_share),
            ("sector", evidence.sector_shares, self._policy.maximum_single_sector_share),
            ("period", evidence.period_shares, self._policy.maximum_single_period_share),
        ):
            defects.extend(_concentration_defects(label, shares, ceiling, details))

        neighbourhood = _neighbourhood_defect(
            evidence.neighbourhood_objectives,
            self._policy.neighbourhood_tolerance,
            self._policy.minimum_neighbourhood_agreement,
        )
        if isinstance(neighbourhood, str):
            defects.append(neighbourhood)
        else:
            details["neighbourhood_agreement"] = neighbourhood

        if not evidence.stress_returns:
            defects.append("no stress scenario is declared")
        for scenario, mean in sorted(evidence.stress_returns.items()):
            details[f"stress:{scenario}"] = mean
            if mean <= 0.0:
                defects.append(f"the {scenario} stress turns the mean return negative ({mean})")

        details["drawdown"] = evidence.drawdown
        if evidence.drawdown >= self._policy.maximum_drawdown:
            defects.append(
                f"the drawdown {evidence.drawdown} reaches {self._policy.maximum_drawdown}"
            )

        if defects:
            return GateVerdict.REJECTED, "; ".join(defects), details

        return (
            GateVerdict.PASSED,
            "the result does not depend on one name, period or value",
            details,
        )

    def _gate_holdout_readiness(
        self,
        evidence: GateEvidence,
    ) -> tuple[GateVerdict, str, Mapping[str, object]]:
        """
        G5: the one holdout evaluation, the frozen artifact and the human decision.
        """
        details: dict[str, object] = {
            "confirmation_evaluation": evidence.confirmation_evaluation,
            "frozen_specification_digest": evidence.frozen_specification_digest,
            "evidence_package_complete": evidence.evidence_package_complete,
            "human_decision": evidence.human_decision,
        }

        if evidence.quarantine_reasons:
            reasons = sorted(evidence.quarantine_reasons)
            return (
                GateVerdict.QUARANTINED,
                f"quarantine follows evidence of a defect: {reasons}",
                details,
            )

        if (
            evidence.frozen_specification_digest
            and evidence.frozen_specification_digest != evidence.specification_digest
        ):
            frozen = evidence.frozen_specification_digest
            return (
                GateVerdict.QUARANTINED,
                f"the holdout was made under {frozen}: the identity does not match",
                details,
            )

        pending: list[str] = []
        if self._custody is not None and self._custody.verdict is None:
            pending.append("the campaign holds no confirmation verdict")
        if not evidence.confirmation_evaluation:
            pending.append("no recorded holdout evaluation")
        if not evidence.evidence_package_complete:
            pending.append("the evidence package is not complete")
        if not evidence.human_decision:
            pending.append("no human decision is recorded")
        if pending:
            return GateVerdict.VALIDATION_PENDING, "; ".join(pending), details

        return GateVerdict.PASSED, "the frozen artifact is complete and a human decided", details


def _concentration_defects(
    label: str,
    shares: Mapping[str, float],
    ceiling: float,
    details: dict[str, object],
) -> list[str]:
    """
    Return the concentration defects one dimension contributes, recording the worst share.
    """
    if not shares:
        return [f"no {label} concentration is declared"]

    worst = max(shares.items(), key=lambda item: item[1])
    details[f"{label}_share"] = {worst[0]: worst[1]}
    if worst[1] > ceiling:
        return [f"{worst[0]} contributes {worst[1]} of the {label} ceiling {ceiling}"]

    return []


def _neighbourhood_defect(
    objectives: Sequence[float],
    tolerance: float,
    minimum_agreement: float,
) -> float | str:
    """
    Return the share of neighbours that reproduce, or why the check cannot be made.
    """
    if len(objectives) < _MINIMUM_NEIGHBOURS:
        return "no parameter neighbourhood is declared"

    selected = objectives[0]
    if selected <= 0.0:
        return "the selected parameter's objective is not positive, so a neighbourhood is undefined"

    agreeing = [value for value in objectives[1:] if value >= selected * (1.0 - tolerance)]
    share = len(agreeing) / (len(objectives) - 1)
    if share < minimum_agreement:
        return f"only {share} of the declared neighbours reproduce within {tolerance}"

    return share


class CandidateState(Enum):
    """
    The candidate lifecycle of design Section 11.4.
    """

    PROPOSED = "proposed"
    COMPILED = "compiled"
    BACKTESTED = "backtested"
    VALIDATION_PENDING = "validation_pending"
    VALIDATED = "validated"
    SHADOW = "shadow"
    QUARANTINED = "quarantined"
    RETIRED = "retired"

    @property
    def terminal(self) -> bool:
        """
        Whether the state ends the candidate's life for its specification.
        """
        return self in (CandidateState.QUARANTINED, CandidateState.RETIRED)


class Actor(Enum):
    """
    Who may perform a transition.
    """

    COMPILER = "compiler"
    RUN_ADAPTER = "run_adapter"
    GATE_SERVICE = "gate_service"
    HUMAN = "human"
    OPERATOR = "operator"


@dataclass(frozen=True)
class TransitionRule:
    """
    One row of the lifecycle transition table.

    Parameters
    ----------
    source : CandidateState | None
        The state the transition starts in, or None for a transition from any state.
    target : CandidateState
        The state it moves to.
    performers : frozenset[Actor]
        Who may perform it.
    evidence_required : str
        What must exist beforehand.
    retry : str
        What a retry creates, or why it is not available.

    """

    source: CandidateState | None
    target: CandidateState
    performers: frozenset[Actor]
    evidence_required: str
    retry: str

    def to_dict(self) -> dict[str, object]:
        """
        Return the rule as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "source": None if self.source is None else self.source.value,
            "target": self.target.value,
            "performers": sorted(actor.value for actor in self.performers),
            "evidence_required": self.evidence_required,
            "retry": self.retry,
        }


# The transition table of design Section 11.4, as data rather than as a status field.
TRANSITIONS: tuple[TransitionRule, ...] = (
    TransitionRule(
        source=CandidateState.PROPOSED,
        target=CandidateState.COMPILED,
        performers=frozenset({Actor.COMPILER}),
        evidence_required="a specification passing the compiler's five stages, with both digests",
        retry="a refusal leaves it proposed, and is a proposal attempt rather than a trial",
    ),
    TransitionRule(
        source=CandidateState.COMPILED,
        target=CandidateState.BACKTESTED,
        performers=frozenset({Actor.RUN_ADAPTER}),
        evidence_required="a registered execution with a verified receipt",
        retry="a retry after a crash is a new attempt against the same configuration",
    ),
    TransitionRule(
        source=CandidateState.BACKTESTED,
        target=CandidateState.VALIDATION_PENDING,
        performers=frozenset({Actor.GATE_SERVICE}),
        evidence_required="the evidence package, with its data role declared",
        retry="not retryable without new evidence",
    ),
    TransitionRule(
        source=CandidateState.VALIDATION_PENDING,
        target=CandidateState.VALIDATED,
        performers=frozenset({Actor.GATE_SERVICE, Actor.HUMAN}),
        evidence_required="every gate passed, one confirmation evaluation, a frozen artifact",
        retry="a policy or data change invalidates it: it is re-judged under the new policy",
    ),
    TransitionRule(
        source=CandidateState.VALIDATED,
        target=CandidateState.SHADOW,
        performers=frozenset({Actor.OPERATOR}),
        evidence_required="a human decision recorded with its timestamp",
        retry="not automatic and not retryable as a right",
    ),
    TransitionRule(
        source=None,
        target=CandidateState.QUARANTINED,
        performers=frozenset({Actor.GATE_SERVICE}),
        evidence_required="evidence of confirmation-set leakage or of an accounting defect",
        retry="terminal for that specification",
    ),
    TransitionRule(
        source=None,
        target=CandidateState.RETIRED,
        performers=frozenset({Actor.OPERATOR, Actor.GATE_SERVICE}),
        evidence_required="a confirmation evaluation, a superseding candidate, or a branch",
        retry="terminal; a successor is a new candidate",
    ),
)


def transition_rule(source: CandidateState, target: CandidateState) -> TransitionRule | None:
    """
    Return the rule that permits a transition, or None when the table permits none.

    Parameters
    ----------
    source : CandidateState
        The state the transition starts in.
    target : CandidateState
        The state it would move to.

    Returns
    -------
    TransitionRule | None

    """
    for rule in TRANSITIONS:
        if rule.target is target and rule.source in (None, source):
            return rule

    return None


@dataclass(frozen=True)
class OutcomeRecord:
    """
    A gate's outcome recorded against a candidate without moving it.

    `INVALID`, `FAILED`, `REJECTED` and `INCONCLUSIVE` are outcomes rather than resting states: each
    records why, and each leaves the candidate where it was for a purpose the transition table
    names.

    Parameters
    ----------
    verdict : GateVerdict
        The outcome.
    gate : GateName | None
        The gate that produced it, or None for a runtime failure.
    reason : str
        Why, in terms of the numbers.
    subject : str
        The state the candidate is left in.

    """

    verdict: GateVerdict
    gate: GateName | None
    reason: str
    subject: str = ""

    def to_dict(self) -> dict[str, object]:
        """
        Return the outcome as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "verdict": self.verdict.value,
            "gate": None if self.gate is None else self.gate.value,
            "reason": self.reason,
            "subject": self.subject,
        }


@dataclass(frozen=True)
class TransitionRecord:
    """
    One recorded transition.

    Parameters
    ----------
    source : CandidateState
        Where the candidate was.
    target : CandidateState
        Where it moved to.
    actor : Actor
        Who performed it.
    evidence : Mapping[str, object]
        What existed beforehand.
    detail : Mapping[str, object]
        Anything else the record carries.

    """

    source: CandidateState
    target: CandidateState
    actor: Actor
    evidence: Mapping[str, object] = field(default_factory=dict)
    detail: Mapping[str, object] = field(default_factory=dict)

    def to_dict(self) -> dict[str, object]:
        """
        Return the record as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "source": self.source.value,
            "target": self.target.value,
            "actor": self.actor.value,
            "evidence": dict(self.evidence),
            "detail": dict(self.detail),
        }


class Lifecycle:
    """
    One candidate's lifecycle: transitions are recorded, not mutated.

    A transition the table does not permit is refused, and so is a transition out of a terminal
    state, because a revised candidate is a new child that inherits nothing: not its parent's
    evidence, not its status, and not its place in a queue.

    Parameters
    ----------
    candidate_id : str
        The candidate's behavioural identity.
    inventory : CampaignInventory | None, default None
        The campaign inventory the outcome is written to, or None when the caller keeps its own.
    initial : CandidateState, default PROPOSED
        The state the candidate starts in.

    """

    def __init__(
        self,
        candidate_id: str,
        *,
        inventory: CampaignInventory | None = None,
        initial: CandidateState = CandidateState.PROPOSED,
    ) -> None:
        """
        Initialize the lifecycle over one candidate.
        """
        if not candidate_id:
            raise ValueError("candidate_id must not be empty")

        self._candidate_id = candidate_id
        self._inventory = inventory
        self._state = initial
        self._history: list[TransitionRecord] = []
        self._outcomes: list[OutcomeRecord] = []

    @property
    def candidate_id(self) -> str:
        """
        The candidate's behavioural identity.
        """
        return self._candidate_id

    @property
    def state(self) -> CandidateState:
        """
        The candidate's current state.
        """
        return self._state

    @property
    def history(self) -> tuple[TransitionRecord, ...]:
        """
        Every recorded transition, in order.
        """
        return tuple(self._history)

    @property
    def outcomes(self) -> tuple[OutcomeRecord, ...]:
        """
        Every outcome recorded against the candidate, in order.
        """
        return tuple(self._outcomes)

    def permits(self, target: CandidateState, *, actor: Actor) -> bool:
        """
        Whether the table permits a transition by an actor.

        Parameters
        ----------
        target : CandidateState
            The state the candidate would move to.
        actor : Actor
            Who would perform it.

        Returns
        -------
        bool

        """
        if self._state.terminal:
            return False

        rule = transition_rule(self._state, target)

        return rule is not None and actor in rule.performers

    def advance(
        self,
        target: CandidateState,
        *,
        actor: Actor,
        evidence: Mapping[str, object] | None = None,
    ) -> TransitionRecord:
        """
        Record a transition, or refuse it.

        Parameters
        ----------
        target : CandidateState
            The state to move to.
        actor : Actor
            Who is performing it.
        evidence : Mapping[str, object] | None, default None
            What exists beforehand.

        Returns
        -------
        TransitionRecord

        Raises
        ------
        ValueError
            If the table permits no transition from the current state to the target by that actor,
            or the candidate is already terminal.

        """
        if self._state.terminal:
            raise ValueError(f"{self._state.value} is terminal for this specification")

        rule = transition_rule(self._state, target)
        if rule is None:
            raise ValueError(
                f"the transition {self._state.value} to {target.value} is not in the table"
            )
        if actor not in rule.performers:
            performers = sorted(performer.value for performer in rule.performers)
            raise ValueError(
                f"{target.value} is performed by {performers} and not by {actor.value}"
            )

        record = TransitionRecord(
            source=self._state,
            target=target,
            actor=actor,
            evidence={} if evidence is None else dict(evidence),
            detail={"evidence_required": rule.evidence_required, "retry": rule.retry},
        )
        self._state = target
        self._history.append(record)

        return record

    def record_outcome(
        self,
        verdict: GateVerdict,
        *,
        reason: str,
        gate: GateName | None = None,
    ) -> OutcomeRecord:
        """
        Record a gate's outcome without moving the candidate.

        Parameters
        ----------
        verdict : GateVerdict
            The outcome.
        reason : str
            Why, in terms of the numbers.
        gate : GateName | None, default None
            The gate that produced it, or None for a runtime failure.

        Returns
        -------
        OutcomeRecord

        Raises
        ------
        ValueError
            If the outcome is a pass, which is a transition rather than an outcome.

        """
        if verdict.passed:
            raise ValueError(f"a {verdict.value} verdict is a transition rather than an outcome")

        record = OutcomeRecord(verdict=verdict, gate=gate, reason=reason, subject=self._state.value)
        self._outcomes.append(record)

        return record

    def apply(self, report: GateReport) -> None:
        """
        Record a gate report's outcome and make the transition it earns.

        A report that passed every gate moves the candidate to `VALIDATION_PENDING`; a report that
        stopped records the outcome and leaves the candidate where it was.

        Parameters
        ----------
        report : GateReport
            The gate service's report.

        """
        if report.passed:
            self.advance(
                CandidateState.VALIDATION_PENDING,
                actor=Actor.GATE_SERVICE,
                evidence={"evidence_digest": report.evidence_digest, "gates": len(report.verdicts)},
            )
            return

        stopped = report.stopped_at
        record = report.verdict_for(stopped) if stopped is not None else None
        self.record_outcome(
            report.outcome,
            reason="" if record is None else record.reason,
            gate=stopped,
        )

    def to_dict(self) -> dict[str, object]:
        """
        Return the lifecycle as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "candidate_id": self._candidate_id,
            "state": self._state.value,
            "terminal": self._state.terminal,
            "history": [record.to_dict() for record in self._history],
            "outcomes": [record.to_dict() for record in self._outcomes],
        }


class AdmissionOutcome(Enum):
    """
    What the pre-campaign admission assessment returns.
    """

    ADMIT = "admit"
    REDUCE = "reduce"
    REJECT = "reject"


@dataclass(frozen=True)
class AdmissionInputs:
    """
    The declared inputs the admission assessment computes its ceiling from.

    Parameters
    ----------
    sample_observations : int, default 0
        The number of periods the data snapshot holds.
    effective_observations : int, default 0
        The effective number of independent observations, estimated from the series rather than
        read off a bar count.
    label_horizon : int, default 1
        The label horizon in periods, which makes consecutive observations dependent.
    planned_configurations : int, default 0
        The configuration count the campaign proposes.
    dependence : TrialDependence, default INDEPENDENT
        Whether the configurations are independent draws.
    effective_trials : int | None, default None
        The effective trial count, required when the configurations are dependent.
    trials_already_consumed : int, default 0
        How many trials the research family has already spent against this data.
    reserved_observations : int, default 0
        The observations consumed by validation folds and the embargo.
    holdout_fraction : float, default 0.0
        The share of the sample reserved for the final holdout.
    target_sharpe : float, default 0.0
        The per-period Sharpe ratio the campaign claims.
    significance : float, default 0.95
        The probability the deflated Sharpe ratio must reach.
    power : float, default 0.8
        The power the campaign assumes. The relation is not a power calculation, so the assessment
        reports this as an assumption it cannot verify from the relation itself.
    cross_trial_dispersion : float | None, default None
        The assumed cross-trial dispersion of the Sharpe estimates, or None when it cannot be
        estimated. A None here is a `REJECT` rather than a default.
    complexity_parameters : int, default 0
        The free parameters per configuration, which decides how many configurations cover the same
        behaviour.

    """

    sample_observations: int = 0
    effective_observations: int = 0
    label_horizon: int = 1
    planned_configurations: int = 0
    dependence: TrialDependence = TrialDependence.INDEPENDENT
    effective_trials: int | None = None
    trials_already_consumed: int = 0
    reserved_observations: int = 0
    holdout_fraction: float = 0.0
    target_sharpe: float = 0.0
    significance: float = 0.95
    power: float = 0.8
    cross_trial_dispersion: float | None = None
    complexity_parameters: int = 0

    @property
    def usable_observations(self) -> int:
        """
        The observations the ceiling may count, after the folds, the embargo and the holdout.
        """
        reserved = self.reserved_observations + round(
            self.effective_observations * self.holdout_fraction,
        )

        return max(0, self.effective_observations - reserved)

    @property
    def counted_trials(self) -> int | None:
        """
        The trial count the correction counts, or None when it cannot be estimated.
        """
        if self.dependence is TrialDependence.DEPENDENT:
            if self.effective_trials is None:
                return None

            return self.effective_trials + self.trials_already_consumed

        return self.planned_configurations + self.trials_already_consumed

    def provenance(self) -> dict[str, object]:
        """
        Return every input with where it came from.

        Returns
        -------
        dict[str, object]

        """
        declared = {
            "sample_observations": "data snapshot",
            "effective_observations": "estimated from the return series' dependence",
            "label_horizon": "policy declaration",
            "planned_configurations": "campaign plan",
            "dependence": "policy declaration",
            "effective_trials": "search plan",
            "trials_already_consumed": "research family's ledger",
            "reserved_observations": "split layout",
            "holdout_fraction": "policy declaration",
            "target_sharpe": "policy declaration",
            "significance": "policy declaration",
            "power": "policy declaration",
            "cross_trial_dispersion": "estimated from the trial inventory",
            "complexity_parameters": "policy declaration",
        }

        return {
            name: {"value": getattr(self, name), "source": source}
            for name, source in declared.items()
        }

    def as_mapping(self) -> dict[str, object]:
        """
        Return the input values as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {name: entry["value"] for name, entry in self.provenance().items()}


@dataclass(frozen=True)
class AdmissionAssessment:
    """
    The admission assessment's report: an outcome, its arithmetic and what it cannot support.

    Parameters
    ----------
    outcome : AdmissionOutcome
        `ADMIT`, `REDUCE` or `REJECT`.
    required_observations : float | None
        The ceiling the relation returns for the counted trials, or None when it was not computed.
    usable_observations : int
        The observations available after the folds, the embargo and the holdout.
    counted_trials : int | None
        The trial count the ceiling was computed at.
    effective_trials : int | None
        The estimated effective trial count, or None when the method cannot estimate one.
    reduced_configurations : int | None
        The configuration count that would fit, present exactly when the outcome is `REDUCE`.
    inputs : Mapping[str, object]
        Every input with its provenance.
    assumptions : Mapping[str, object]
        The assumptions, each with the sensitivity of the outcome to it.
    limitations : tuple[str, ...]
        The sample-size and power limitations the assessment ran into.
    supports : tuple[str, ...]
        The statistical claims the assessment can support.
    withholds : tuple[str, ...]
        The claims it cannot.
    reason : str
        Why the outcome, in terms of the numbers.

    """

    outcome: AdmissionOutcome
    required_observations: float | None
    usable_observations: int
    counted_trials: int | None
    effective_trials: int | None = None
    reduced_configurations: int | None = None
    inputs: Mapping[str, object] = field(default_factory=dict)
    assumptions: Mapping[str, object] = field(default_factory=dict)
    limitations: tuple[str, ...] = ()
    supports: tuple[str, ...] = ()
    withholds: tuple[str, ...] = ()
    reason: str = ""

    @property
    def formula(self) -> str:
        """
        The relation, with its version.
        """
        return f"minimum track record length, inverted by search ({ADMISSION_VERSION})"

    def to_dict(self) -> dict[str, object]:
        """
        Return the assessment as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "outcome": self.outcome.value,
            "formula": self.formula,
            "version": ADMISSION_VERSION,
            "required_observations": self.required_observations,
            "usable_observations": self.usable_observations,
            "counted_trials": self.counted_trials,
            "effective_trials": self.effective_trials,
            "reduced_configurations": self.reduced_configurations,
            "inputs": dict(self.inputs),
            "assumptions": dict(self.assumptions),
            "limitations": list(self.limitations),
            "supports": list(self.supports),
            "withholds": list(self.withholds),
            "reason": self.reason,
        }


@dataclass(frozen=True)
class TrackingAssumptions:
    """
    The assumptions the inverted relation is computed under.

    Parameters
    ----------
    target_sharpe : float, default 0.0
        The per-period Sharpe ratio the campaign claims.
    cross_trial_dispersion : float, default 0.0
        The assumed standard deviation of the trial Sharpe estimates.
    significance : float, default 0.95
        The probability the correction must reach.
    skew : float, default 0.0
        The assumed skewness of the selected series.
    kurtosis : float, default 3.0
        The assumed non-excess kurtosis, which is 3 for normal returns.

    """

    target_sharpe: float = 0.0
    cross_trial_dispersion: float = 0.0
    significance: float = 0.95
    skew: float = 0.0
    kurtosis: float = 3.0

    def to_dict(self) -> dict[str, object]:
        """
        Return the assumptions as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "target_sharpe": self.target_sharpe,
            "cross_trial_dispersion": self.cross_trial_dispersion,
            "significance": self.significance,
            "skew": self.skew,
            "kurtosis": self.kurtosis,
            "kurtosis_convention": "non_excess",
        }


def minimum_track_record_length(
    counts_trials: int,
    assumptions: TrackingAssumptions,
    *,
    contract: StatisticalContract | None = None,
) -> float | None:
    """
    Invert the deflated Sharpe relation for the smallest sample that reaches a significance.

    The relation is the correction the repository already computes, and it is inverted by search
    rather than by a second implementation of its quantiles: the correction is evaluated at
    increasing sample lengths until it reaches the declared significance. The declared cross-trial
    dispersion is realised by a symmetric set of trial estimates whose sample variance is exactly
    that dispersion squared, because the correction consumes the dispersion rather than the
    estimates themselves, and the assumption is reported beside the result rather than buried here.

    Parameters
    ----------
    counts_trials : int
        The trial count the correction counts.
    assumptions : TrackingAssumptions
        The declared assumptions the relation is computed under.
    contract : StatisticalContract | None, default None
        The contract to compute under, or None for the declared default.

    Returns
    -------
    float | None
        The smallest sample length, or None when the relation cannot be satisfied at any length the
        search reaches or when the correction is unavailable.

    """
    sharpes = assumptions
    if counts_trials < 1 or sharpes.cross_trial_dispersion <= 0.0:
        return None
    if not 0.0 < sharpes.significance < 1.0:
        return None

    spread = sharpes.cross_trial_dispersion * math.sqrt((counts_trials - 1) / counts_trials)
    estimates = [spread] * (counts_trials // 2) + [-spread] * (counts_trials - counts_trials // 2)
    declared = StatisticalContract.declared_default() if contract is None else contract
    lowest = max(_MINIMUM_SAMPLE, declared.minimum_observations)
    highest = _MAXIMUM_SAMPLE

    def reached(length: int) -> bool:
        sample = SharpeSample(
            sharpe=sharpes.target_sharpe,
            trial_sharpes=estimates,
            observations=length,
            skew=sharpes.skew,
            kurtosis=sharpes.kurtosis,
            dependence=TrialDependence.INDEPENDENT,
            frequency=SharpeFrequency.PER_PERIOD,
        )
        result = deflated_sharpe_ratio(sample, contract=contract)

        return (
            result.is_computed and result.value is not None and result.value >= sharpes.significance
        )

    if reached(lowest):
        return float(lowest)
    if not reached(highest):
        return None

    while lowest + 1 < highest:
        middle = (lowest + highest) // 2
        if reached(middle):
            highest = middle
        else:
            lowest = middle

    return float(highest)


def required_observations(
    inputs: AdmissionInputs,
    configurations: int,
    contract: StatisticalContract,
) -> float | None:
    """
    Return the sample the declared inputs need for a configuration count.

    Parameters
    ----------
    inputs : AdmissionInputs
        The declared inputs.
    configurations : int
        The configuration count to assess.
    contract : StatisticalContract
        The contract the correction is computed under.

    Returns
    -------
    float | None

    """
    if inputs.cross_trial_dispersion is None:
        return None

    counted = configurations
    if inputs.dependence is TrialDependence.DEPENDENT:
        if inputs.effective_trials is None:
            return None
        counted = inputs.effective_trials

    assumptions = TrackingAssumptions(
        target_sharpe=inputs.target_sharpe,
        cross_trial_dispersion=inputs.cross_trial_dispersion,
        significance=inputs.significance,
    )

    return minimum_track_record_length(
        counted + inputs.trials_already_consumed,
        assumptions,
        contract=contract,
    )


def assess_admission(
    inputs: AdmissionInputs,
    *,
    policy: GatePolicy | None = None,
) -> AdmissionAssessment:
    """
    Assess whether a campaign's proposed search fits what its sample can support.

    The assessment answers before the first backtest runs, because the question is whether the
    sample can support the claim, and that question does not need a result. An input it cannot
    estimate is a `REJECT` rather than a guess, and a plan that exceeds its ceiling is a `REDUCE`
    when narrowing the configuration count would fit, and a `REJECT` when no allocation would.

    Parameters
    ----------
    inputs : AdmissionInputs
        The declared inputs.
    policy : GatePolicy | None, default None
        The gate policy, which supplies the statistical contract.

    Returns
    -------
    AdmissionAssessment

    """
    declared = GatePolicy() if policy is None else policy
    contract = declared.statistical_contract
    provenance = inputs.provenance()
    limitations = [
        (
            f"the relation is a significance relation, not a power calculation: the declared "
            f"power {inputs.power} cannot be verified from it"
        ),
    ]

    def reject(reason: str, assumptions: Mapping[str, object] | None = None) -> AdmissionAssessment:
        return AdmissionAssessment(
            outcome=AdmissionOutcome.REJECT,
            required_observations=None,
            usable_observations=inputs.usable_observations,
            counted_trials=inputs.counted_trials,
            effective_trials=inputs.effective_trials,
            inputs=provenance,
            assumptions={} if assumptions is None else dict(assumptions),
            limitations=tuple(limitations),
            supports=("nothing: the assessment could not estimate what it needs",),
            withholds=("any claim about the campaign, because an input cannot be estimated",),
            reason=reason,
        )

    unestimable = _unestimable(inputs)
    if unestimable:
        return reject(unestimable)

    counted_trials = inputs.counted_trials
    effective_trials = inputs.effective_trials
    if inputs.dependence is TrialDependence.DEPENDENT and effective_trials is not None:
        counted_trials = effective_trials + inputs.trials_already_consumed

    def required(configurations: int, **overrides: object) -> float | None:
        declared_inputs = replace(inputs, **overrides) if overrides else inputs

        return required_observations(declared_inputs, configurations, contract)

    ceiling = required(inputs.planned_configurations)
    usable = inputs.usable_observations
    assumptions: dict[str, object] = {
        "cross_trial_dispersion": {
            "value": inputs.cross_trial_dispersion,
            "sensitivity": _sensitivity(inputs, required, field_name="cross_trial_dispersion"),
        },
        "target_sharpe": {
            "value": inputs.target_sharpe,
            "sensitivity": _sensitivity(inputs, required, field_name="target_sharpe"),
        },
        "dependence": {
            "value": inputs.dependence.value,
            "note": "the dependence declaration decides whether the raw or effective count is used",
        },
    }

    if ceiling is None:
        reason = (
            f"no sample distinguishes the declared target Sharpe {inputs.target_sharpe} from what "
            f"{counted_trials} trials would produce by chance, so no ceiling is claimed"
        )
        return reject(reason, assumptions)

    supports = (
        (
            f"a claim that the search fits the sample at a per-period Sharpe of "
            f"{inputs.target_sharpe} and {counted_trials} counted trials"
        ),
    )
    withholds = (
        "any claim of future profitability, which no admission assessment can make",
        "any claim beyond the declared target effect size, because the ceiling moves with it",
    )

    if ceiling <= usable:
        reason = f"the plan fits: {ceiling:.0f} required observations against {usable} usable"
        return AdmissionAssessment(
            outcome=AdmissionOutcome.ADMIT,
            required_observations=ceiling,
            usable_observations=usable,
            counted_trials=counted_trials,
            effective_trials=effective_trials,
            inputs=provenance,
            assumptions=assumptions,
            limitations=tuple(limitations),
            supports=supports,
            withholds=withholds,
            reason=reason,
        )

    reduced = _largest_that_fits(inputs, required, usable)
    if reduced is not None and reduced >= 1:
        reason = f"the plan exceeds its ceiling and narrows to {reduced} configurations"
        return AdmissionAssessment(
            outcome=AdmissionOutcome.REDUCE,
            required_observations=ceiling,
            usable_observations=usable,
            counted_trials=counted_trials,
            effective_trials=effective_trials,
            reduced_configurations=reduced,
            inputs=provenance,
            assumptions=assumptions,
            limitations=tuple(limitations),
            supports=(f"a claim that a search of {reduced} configurations fits the sample",),
            withholds=(f"the proposed {inputs.planned_configurations}-configuration plan",),
            reason=reason,
        )

    reason = f"no allocation fits: one configuration needs {ceiling:.0f} against {usable} usable"
    return AdmissionAssessment(
        outcome=AdmissionOutcome.REJECT,
        required_observations=ceiling,
        usable_observations=usable,
        counted_trials=counted_trials,
        effective_trials=effective_trials,
        inputs=provenance,
        assumptions=assumptions,
        limitations=tuple(limitations),
        supports=("nothing: the data cannot support the proposed claims at any allocation",),
        withholds=("every claim in the plan, including a narrowed one",),
        reason=reason,
    )


def _unestimable(inputs: AdmissionInputs) -> str:
    """
    Return why the assessment cannot estimate what it needs, or the empty string when it can.
    """
    if inputs.dependence is TrialDependence.DEPENDENT and inputs.effective_trials is None:
        return "the configurations are dependent and no effective trial count is estimated"
    if inputs.effective_observations <= 0:
        return "the effective number of independent observations is not estimated"
    if inputs.cross_trial_dispersion is None:
        return "the cross-trial dispersion of the Sharpe estimates is not estimated"

    counted = inputs.counted_trials
    if counted is None or counted < 1:
        return "the campaign proposes no configuration, so there is nothing to assess"

    return ""


def _sensitivity(
    inputs: AdmissionInputs,
    required: Callable[..., float | None],
    *,
    field_name: str,
    factor: float = 0.1,
) -> Mapping[str, object]:
    """
    Return how the ceiling moves when one input is perturbed by a declared factor.
    """
    base = getattr(inputs, field_name)
    if not isinstance(base, int | float) or base == 0:
        return {"factor": factor, "measurements": {}, "note": "the input cannot be perturbed"}

    measurements: dict[str, object] = {}
    for label, multiplier in (("lower", 1.0 - factor), ("upper", 1.0 + factor)):
        measurements[label] = required(
            inputs.planned_configurations,
            **{field_name: base * multiplier},
        )

    return {"factor": factor, "measurements": measurements}


def _largest_that_fits(
    inputs: AdmissionInputs,
    required: Callable[..., float | None],
    usable: int,
) -> int | None:
    """
    Return the largest configuration count whose ceiling fits, or None when none does.

    The search is a descent from the proposed count rather than a second inversion, so the number it
    returns is one this relation produced rather than one an estimate produced.
    """
    for candidate in range(inputs.planned_configurations - 1, 0, -1):
        ceiling = required(candidate)
        if ceiling is not None and ceiling <= usable:
            return candidate

    return None


class PboState(Enum):
    """
    The four states the PBO matrix evaluation distinguishes.

    Two are per-row states - ineligible, and eligible-but-failed - and two are matrix states. A
    matrix that survives its preconditions is reported *with documented exclusions*, whether or not
    it has any, because the exclusions and their classes are part of the result rather than a note
    beside it.
    """

    INELIGIBLE = "ineligible"
    ELIGIBLE_BUT_FAILED = "eligible_but_failed"
    VALID_WITH_EXCLUSIONS = "valid_with_documented_exclusions"
    NOT_COMPUTABLE = "not_computable"


@dataclass(frozen=True)
class PboRow:
    """
    One configuration's row of the overfitting matrix.

    Parameters
    ----------
    configuration : str
        The configuration's identity.
    eligible : bool, default True
        Whether it passed the rules fixed before the matrix was built.
    ineligible_rule : str, default ""
        The rule that excluded it, present exactly when it is ineligible.
    failure_class : str, default ""
        The class of its failure, present exactly when it ran and failed.
    returns : Mapping[int, float | None], default {}
        Its per-period returns on the matrix's common index.

    """

    configuration: str
    eligible: bool = True
    ineligible_rule: str = ""
    failure_class: str = ""
    returns: Mapping[int, float | None] = field(default_factory=dict)

    def to_dict(self) -> dict[str, object]:
        """
        Return the row as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "configuration": self.configuration,
            "eligible": self.eligible,
            "ineligible_rule": self.ineligible_rule,
            "failure_class": self.failure_class,
            "observations": len(self.returns),
        }


@dataclass(frozen=True)
class PboExclusion:
    """
    One exclusion and the state that explains it.

    Parameters
    ----------
    configuration : str
        The excluded configuration.
    state : PboState
        Why it was excluded.
    reason : str
        The rule or the failure class that excluded it.

    """

    configuration: str
    state: PboState
    reason: str

    def to_dict(self) -> dict[str, object]:
        """
        Return the exclusion as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "configuration": self.configuration,
            "state": self.state.value,
            "reason": self.reason,
        }


@dataclass(frozen=True)
class PboEvaluation:
    """
    The probability of backtest overfitting, evaluated against its preconditions before computing.

    The evaluation distinguishes four states and reports them, because the failure mode to prevent
    is a value that looks valid but was computed on a conveniently selected subset: that carries the
    authority of a number, which is worse than no value at all. The statistic is a diagnostic under
    decision SD8: nothing rejects a candidate on it, and its exclusions are visible here.

    Parameters
    ----------
    state : PboState
        The matrix's state.
    probability : float | None
        The probability of backtest overfitting, or None when no statistic was produced.
    blocks : int
        How many blocks the common index was split into.
    combinations : int
        How many in-sample combinations were evaluated.
    configurations : int
        How many rows survived to be compared.
    exclusions : tuple[PboExclusion, ...]
        Every excluded row with the state that explains it.
    logit_mean : float | None
        The mean of the logits of the out-of-sample relative ranks.
    logit_quantiles : Mapping[str, float]
        The minimum, median and maximum of those logits.
    reason : str
        Why the state, in terms of the rows rather than in prose.

    """

    state: PboState
    probability: float | None = None
    blocks: int = 0
    combinations: int = 0
    configurations: int = 0
    exclusions: tuple[PboExclusion, ...] = ()
    logit_mean: float | None = None
    logit_quantiles: Mapping[str, float] = field(default_factory=dict)
    reason: str = ""

    @property
    def computable(self) -> bool:
        """
        Whether a statistic was produced.
        """
        return self.state is PboState.VALID_WITH_EXCLUSIONS and self.probability is not None

    def to_dict(self) -> dict[str, object]:
        """
        Return the evaluation as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "state": self.state.value,
            "probability": self.probability,
            "blocks": self.blocks,
            "combinations": self.combinations,
            "configurations": self.configurations,
            "exclusions": [exclusion.to_dict() for exclusion in self.exclusions],
            "logit_mean": self.logit_mean,
            "logit_quantiles": dict(self.logit_quantiles),
            "reason": self.reason,
            "diagnostic_only": True,
        }


def evaluate_pbo_matrix(
    rows: Sequence[PboRow],
    *,
    blocks: int = 8,
) -> PboEvaluation:
    """
    Evaluate the overfitting matrix against its preconditions, then compute the statistic.

    The preconditions are checked first, as the design requires: rows are excluded with the rule or
    the failure class that excluded them, the survivors must share one common index, and there must
    be enough blocks and enough survivors for the comparison to mean anything. A matrix that fails
    any of those reports `NOT_COMPUTABLE` and produces no number.

    Parameters
    ----------
    rows : Sequence[PboRow]
        One row per configuration, including the ones that failed or were ineligible.
    blocks : int, default 8
        How many blocks to split the common index into. Must be even, so half can be in-sample.

    Returns
    -------
    PboEvaluation

    """
    exclusions: list[PboExclusion] = []
    survivors: list[PboRow] = []

    for row in rows:
        excluded = _classify(row)
        if excluded is None:
            survivors.append(row)
        else:
            exclusions.append(excluded)

    def not_computable(reason: str) -> PboEvaluation:
        return PboEvaluation(
            state=PboState.NOT_COMPUTABLE,
            exclusions=tuple(exclusions),
            configurations=len(survivors),
            reason=reason,
        )

    precondition = _precondition_failure(survivors, blocks)
    if precondition:
        return not_computable(precondition)

    index = sorted(survivors[0].returns)
    size = len(index) // blocks
    chunks = [index[position * size : (position + 1) * size] for position in range(blocks)]
    half = blocks // 2
    logits: list[float] = []
    configurations = len(survivors)

    for combination in itertools.combinations(range(blocks), half):
        chosen = set(combination)
        in_sample = [
            stamp for number, chunk in enumerate(chunks) if number in chosen for stamp in chunk
        ]
        out_sample = [
            stamp for number, chunk in enumerate(chunks) if number not in chosen for stamp in chunk
        ]

        in_means = [_mean([row.returns[stamp] for stamp in in_sample]) for row in survivors]
        best = max(range(configurations), key=lambda position: in_means[position])
        out_means = [_mean([row.returns[stamp] for stamp in out_sample]) for row in survivors]

        rank = sum(1 for value in out_means if value < out_means[best])
        relative = (rank + 1) / (configurations + 1)
        logits.append(math.log(relative / (1.0 - relative)))

    ordered = sorted(logits)
    below = sum(1 for logit in logits if logit < 0.0)
    reason = (
        f"computed over {len(logits)} combinations of {blocks} blocks and {configurations} "
        f"comparable configurations, with {len(exclusions)} exclusions accounted for"
    )

    return PboEvaluation(
        state=PboState.VALID_WITH_EXCLUSIONS,
        probability=below / len(logits),
        blocks=blocks,
        combinations=len(logits),
        configurations=configurations,
        exclusions=tuple(exclusions),
        logit_mean=math.fsum(logits) / len(logits),
        logit_quantiles={
            "minimum": ordered[0],
            "median": ordered[len(ordered) // 2],
            "maximum": ordered[-1],
        },
        reason=reason,
    )


def _classify(row: PboRow) -> PboExclusion | None:
    """
    Return the exclusion a row earns, or None when it survives to be compared.
    """
    if not row.eligible:
        return PboExclusion(row.configuration, PboState.INELIGIBLE, row.ineligible_rule)

    if row.failure_class:
        return PboExclusion(row.configuration, PboState.ELIGIBLE_BUT_FAILED, row.failure_class)

    if not row.returns or any(value is None for value in row.returns.values()):
        return PboExclusion(row.configuration, PboState.ELIGIBLE_BUT_FAILED, "missing_observations")

    return None


def _precondition_failure(survivors: Sequence[PboRow], blocks: int) -> str:
    """
    Return why the matrix's preconditions do not hold, or the empty string when they do.
    """
    if len(survivors) < _PBO_MINIMUM_SURVIVORS:
        return f"{len(survivors)} rows survived, and a comparison needs {_PBO_MINIMUM_SURVIVORS}"

    if blocks < _PBO_MINIMUM_BLOCKS or blocks % 2 != 0:
        return f"the block count {blocks} is not an even count of at least {_PBO_MINIMUM_BLOCKS}"

    index = sorted(survivors[0].returns)
    for row in survivors[1:]:
        if sorted(row.returns) != index:
            return f"{row.configuration} does not share the common index of the first row"

    if len(index) < blocks * 2:
        return (
            f"the common index holds {len(index)} observations, fewer than the {blocks * 2} needed"
        )

    return ""


def _mean(values: Sequence[float | None]) -> float:
    """
    Return the mean of a block's values, treating a missing value as no observation.
    """
    contributing = [float(value) for value in values if value is not None]

    return 0.0 if not contributing else math.fsum(contributing) / len(contributing)


class ControlClass(Enum):
    """
    The seven classes of control the calibration suite carries.
    """

    NULL = "null"
    POSITIVE = "positive"
    LEAKAGE = "leakage"
    COST = "cost"
    CONCENTRATION = "concentration"
    PARAMETER_SPIKE = "parameter_spike"
    ACCOUNTING = "accounting"


@dataclass(frozen=True)
class CalibrationCase:
    """
    One control case with a known answer.

    Parameters
    ----------
    name : str
        The case's name.
    control_class : ControlClass
        Which class of control it is.
    evidence : GateEvidence
        The evidence the gate chain is evaluated on.
    expectation : str
        `detected` when the chain must not pass it, `accepted` when it must pass.
    expected_gate : GateName | None, default None
        The gate that should detect it, or None for an accepted control.

    """

    name: str
    control_class: ControlClass
    evidence: GateEvidence
    expectation: str
    expected_gate: GateName | None = None

    def __post_init__(self) -> None:
        """
        Validate the expectation, which is what makes the case falsifiable.
        """
        if self.expectation not in ("detected", "accepted"):
            raise ValueError(f"expectation must be detected or accepted, was {self.expectation!r}")
        if self.expectation == "detected" and self.expected_gate is None:
            raise ValueError("a case expected to be detected must name the gate that detects it")

    def to_dict(self) -> dict[str, object]:
        """
        Return the case as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "name": self.name,
            "control_class": self.control_class.value,
            "expectation": self.expectation,
            "expected_gate": None if self.expected_gate is None else self.expected_gate.value,
            "candidate_id": self.evidence.candidate_id,
        }


@dataclass(frozen=True)
class CalibrationOutcome:
    """
    What one control case produced.

    Parameters
    ----------
    case : CalibrationCase
        The case.
    observed : GateVerdict
        The chain's outcome.
    observed_gate : GateName | None, default None
        The gate the chain stopped at, or None when it passed everything.

    """

    case: CalibrationCase
    observed: GateVerdict
    observed_gate: GateName | None = None

    @property
    def agrees(self) -> bool:
        """
        Whether the observed behaviour met the expectation.
        """
        if self.case.expectation == "accepted":
            return self.observed.passed

        return not self.observed.passed

    @property
    def detected_by_expected_gate(self) -> bool:
        """
        Whether the expected gate is the one that detected it.
        """
        if self.case.expectation == "accepted":
            return self.observed.passed

        return self.observed_gate is self.case.expected_gate

    def to_dict(self) -> dict[str, object]:
        """
        Return the outcome as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            **self.case.to_dict(),
            "observed": self.observed.value,
            "observed_gate": None if self.observed_gate is None else self.observed_gate.value,
            "agrees": self.agrees,
            "detected_by_expected_gate": self.detected_by_expected_gate,
        }


@dataclass(frozen=True)
class CalibrationReport:
    """
    What the calibration suite found, with the detected and missed sets named.

    Parameters
    ----------
    outcomes : tuple[CalibrationOutcome, ...]
        One outcome per case.
    policy_digest : str
        The digest of the policy the suite ran under.

    """

    outcomes: tuple[CalibrationOutcome, ...]
    policy_digest: str

    @property
    def detected(self) -> tuple[str, ...]:
        """
        The controls the chain did not pass.
        """
        return tuple(
            outcome.case.name
            for outcome in self.outcomes
            if outcome.case.expectation == "detected" and not outcome.observed.passed
        )

    @property
    def missed(self) -> tuple[str, ...]:
        """
        The controls the chain passed, which is the calibration's failure mode.
        """
        return tuple(
            outcome.case.name
            for outcome in self.outcomes
            if outcome.case.expectation == "detected" and outcome.observed.passed
        )

    @property
    def false_accepts(self) -> tuple[str, ...]:
        """
        The controls accepted that should have been detected, which is `missed` by another name.
        """
        return self.missed

    @property
    def false_rejects(self) -> tuple[str, ...]:
        """
        The positive controls rejected, which is the other direction of a miscalibrated gate.
        """
        return tuple(
            outcome.case.name
            for outcome in self.outcomes
            if outcome.case.expectation == "accepted" and not outcome.observed.passed
        )

    @property
    def detected_classes(self) -> tuple[str, ...]:
        """
        The control classes the chain detected, at least once.
        """
        return tuple(
            sorted(
                {
                    outcome.case.control_class.value
                    for outcome in self.outcomes
                    if outcome.case.expectation == "detected" and not outcome.observed.passed
                },
            ),
        )

    @property
    def missed_classes(self) -> tuple[str, ...]:
        """
        The control classes the chain missed, all of whose cases it passed.
        """
        classes = [outcome.case.control_class.value for outcome in self.outcomes]
        detected = set(self.detected_classes)

        return tuple(sorted({name for name in classes if name not in detected}))

    @property
    def calibrated(self) -> bool:
        """
        Whether every case behaved as declared.
        """
        return all(outcome.agrees for outcome in self.outcomes)

    def to_dict(self) -> dict[str, object]:
        """
        Return the report as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "policy_digest": self.policy_digest,
            "calibrated": self.calibrated,
            "detected": list(self.detected),
            "missed": list(self.missed),
            "detected_classes": list(self.detected_classes),
            "missed_classes": list(self.missed_classes),
            "false_accepts": list(self.false_accepts),
            "false_rejects": list(self.false_rejects),
            "outcomes": [outcome.to_dict() for outcome in self.outcomes],
        }


def run_calibration(
    cases: Sequence[CalibrationCase],
    *,
    policy: GatePolicy,
    custody: ConfirmationCustody | None = None,
) -> CalibrationReport:
    """
    Run the gate chain over the control cases and report what it detected and what it missed.

    The suite is broader than a planted noise set on purpose: rejecting random specifications does
    not show that a diagnostic can separate a realistic false discovery from a real but weak
    strategy. The report names the controls each gate detected and those it failed to detect, and
    the false-accept and false-reject behaviour at the pre-registered thresholds, which is what the
    calibration is for.

    Parameters
    ----------
    cases : Sequence[CalibrationCase]
        The control cases, each with its declared expectation.
    policy : GatePolicy
        The pre-registered thresholds.
    custody : ConfirmationCustody | None, default None
        The campaign's custody machine, or None when the suite confirms nothing.

    Returns
    -------
    CalibrationReport

    """
    service = GateService(policy, custody=custody)
    outcomes: list[CalibrationOutcome] = []

    for case in cases:
        report = service.evaluate(case.evidence)
        outcomes.append(
            CalibrationOutcome(case=case, observed=report.outcome, observed_gate=report.stopped_at),
        )

    return CalibrationReport(outcomes=tuple(outcomes), policy_digest=policy.digest())
