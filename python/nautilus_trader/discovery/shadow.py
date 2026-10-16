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
The shadow monitor: one temporal contract for replay and shadow, and the divergence report.

Shadow mode of design Section 11.5 uses the mechanisms that already exist: a live data adapter feeds
a frozen specification in real time, and the sandbox venue stands in for execution, so fills are
simulated against observed prices and quotes with the recorded cost configuration. No order leaves
the process, and nothing here holds a client: an observation arrives from the feed and the simulated
fill is computed from it, which is why the module is pure and takes its observations as an argument
rather than opening a connection.

Criterion 13 is the reason the temporal contract is an explicit object rather than a convention. A
shadow run records the same declared field set and the same pair of timestamps as the backtest it
shadows, so a simulated fill can be compared with the replay's expectation directly instead of being
reconciled by hand. `TemporalContract` carries the four declarations of the specification's data
block, `TemporalReading` carries one decision's pair of instants, and `temporal_parity` reports
whether two readings share both. Declarations are part of identity: a contract whose
`execution_eligibility` differs is a different contract, whatever the timestamps do.

Criterion 15 is the reason the report is a report rather than a summary. It carries the count of
eligible decisions and of missing observations, each signal discrepancy with its magnitude, each
simulated non-fill with its reason, and the modelled against the observed cost, so the discrepancy
between the backtest's assumption and the market's behaviour is diagnosable. The observation count
is declared by the strategy's trade frequency before the period starts and is reported beside the
count reached: meeting it is a precondition for reading a report, and neither a passing period nor a
met count is evidence of an edge. Every report is labelled simulated wherever it is shown.
"""

from __future__ import annotations

from collections import Counter
from dataclasses import dataclass
from decimal import Decimal
from enum import Enum
from enum import unique
from typing import TYPE_CHECKING

from nautilus_trader.discovery.schema import canonical_mapping
from nautilus_trader.optimization.space import digest_of


if TYPE_CHECKING:
    from collections.abc import Sequence

    from nautilus_trader.discovery.schema import DataBlock


# The schema of a shadow report, so a retained report says what produced it.
SHADOW_SCHEMA = "discovery.shadow/v1"

# The label every shadow result carries, because a simulated fill is never an observed one.
SIMULATED_LABEL = "simulated"


@unique
class ShadowRefusalCode(Enum):
    """
    Why the monitor refused to produce a report, as opposed to reporting a divergence.
    """

    MINIMUM_NOT_DECLARED = "minimum_not_declared"
    ELIGIBLE_BEFORE_SIGNAL = "eligible_before_signal"
    NON_FILL_WITHOUT_REASON = "non_fill_without_reason"


class ShadowError(RuntimeError):
    """
    Raised when a shadow period cannot be run as declared.

    Parameters
    ----------
    code : ShadowRefusalCode
        The refusal.
    message : str
        The explanation, which names the value that produced the refusal.

    """

    def __init__(self, code: ShadowRefusalCode, message: str) -> None:
        """
        Initialise the refusal with its code and explanation.
        """
        super().__init__(f"{code.value}: {message}")
        self.code = code


@dataclass(frozen=True)
class TemporalContract:
    """
    The declared temporal fields replay and shadow must share.

    The four declarations are the specification's own, read from its data block: they are what the
    run promised about when it decides, when it may execute, how often it looks and how it treats a
    corporate action. Two runs that disagree about any of them are not comparable, so a contract is
    compared by value rather than by shape.

    Parameters
    ----------
    signal_frequency : str
        How often a signal is produced.
    decision_time : str
        The instant within a period a decision is taken.
    execution_eligibility : str
        The earliest instant a decision may be executed at.
    corporate_action_treatment : str
        How a corporate action is treated.

    """

    signal_frequency: str
    decision_time: str
    execution_eligibility: str
    corporate_action_treatment: str

    @classmethod
    def of(cls, data: DataBlock) -> TemporalContract:
        """
        Return the contract a specification's data block declares.

        Parameters
        ----------
        data : DataBlock
            The specification's data block.

        Returns
        -------
        TemporalContract

        """
        return cls(
            signal_frequency=data.signal_frequency,
            decision_time=data.decision_time,
            execution_eligibility=data.execution_eligibility,
            corporate_action_treatment=data.corporate_action_treatment,
        )

    def declared_fields(self) -> tuple[tuple[str, str], ...]:
        """
        Return the declared field set as named pairs, in a stable order.

        Returns
        -------
        tuple[tuple[str, str], ...]

        """
        return (
            ("signal_frequency", self.signal_frequency),
            ("decision_time", self.decision_time),
            ("execution_eligibility", self.execution_eligibility),
            ("corporate_action_treatment", self.corporate_action_treatment),
        )

    def to_dict(self) -> dict[str, object]:
        """
        Return the contract as plain data.

        Returns
        -------
        dict[str, object]

        """
        return dict(self.declared_fields())

    def digest(self) -> str:
        """
        Return the digest of the contract.

        Returns
        -------
        str
            The `sha256:<hex>` digest.

        """
        return digest_of(canonical_mapping(self.to_dict()))


@dataclass(frozen=True)
class TemporalReading:
    """
    One decision's pair of instants, under the contract it was taken under.

    Parameters
    ----------
    contract : TemporalContract
        The declared fields the reading was taken under.
    decision_at : int
        The instant the decision was taken, in nanoseconds.
    eligible_fill_at : int
        The instant the decision was eligible to execute at, in nanoseconds. In a replay this is the
        instant the fill was recorded at for a filled decision, and in shadow it is the instant the
        declared eligibility rule selects for the observed signal.

    """

    contract: TemporalContract
    decision_at: int
    eligible_fill_at: int

    def pair(self) -> tuple[int, int]:
        """
        Return the pair criterion 13 compares.

        Returns
        -------
        tuple[int, int]

        """
        return (self.decision_at, self.eligible_fill_at)

    def to_dict(self) -> dict[str, object]:
        """
        Return the reading as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "contract": self.contract.to_dict(),
            "contract_digest": self.contract.digest(),
            "decision_at": self.decision_at,
            "eligible_fill_at": self.eligible_fill_at,
        }


@dataclass(frozen=True)
class TemporalParity:
    """
    Whether a replay and a shadow run share one temporal contract.

    Parameters
    ----------
    replay : TemporalReading
        The replay's reading.
    shadow : TemporalReading
        The shadow's reading.
    differing_fields : tuple[str, ...], default ()
        The declared fields whose values differ, named.
    detail : str, default ""
        What the comparison found, in one sentence.

    """

    replay: TemporalReading
    shadow: TemporalReading
    differing_fields: tuple[str, ...] = ()
    detail: str = ""

    @property
    def same_fields(self) -> bool:
        """
        Whether both readings declare the same fields with the same values.

        A declaration is part of identity, so a contract whose `execution_eligibility` differs is a
        different contract even though it names the same four fields.
        """
        return self.replay.contract.declared_fields() == self.shadow.contract.declared_fields()

    @property
    def same_pair(self) -> bool:
        """
        Whether both readings record the same pair of instants.
        """
        return self.replay.pair() == self.shadow.pair()

    @property
    def holds(self) -> bool:
        """
        Whether the contract is shared, which is both halves.
        """
        return self.same_fields and self.same_pair

    def describe(self) -> str:
        """
        Return the comparison in one sentence.

        Returns
        -------
        str

        """
        if self.holds:
            return "replay and shadow share one temporal contract"

        parts: list[str] = []
        if not self.same_fields:
            parts.append(f"the declared fields differ: {', '.join(self.differing_fields)}")
        if not self.same_pair:
            parts.append(
                f"the timestamps differ: {self.replay.pair()} against {self.shadow.pair()}"
            )

        return "; ".join(parts)

    def to_dict(self) -> dict[str, object]:
        """
        Return the comparison as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "same_fields": self.same_fields,
            "same_pair": self.same_pair,
            "holds": self.holds,
            "differing_fields": list(self.differing_fields),
            "replay": self.replay.to_dict(),
            "shadow": self.shadow.to_dict(),
            "detail": self.detail,
        }


def temporal_parity(replay: TemporalReading, shadow: TemporalReading) -> TemporalParity:
    """
    Compare a replay's reading with a shadow run's, field by field and instant by instant.

    The two halves are reported separately because they fail differently: a differing declaration
    means the shadow run is not shadowing the backtest at all, while a differing instant means the
    feed delivered the decision later or earlier than the replay assumed. Naming which declared
    fields differ is what makes the first diagnosable.

    Parameters
    ----------
    replay : TemporalReading
        The replay's reading.
    shadow : TemporalReading
        The shadow's reading.

    Returns
    -------
    TemporalParity

    """
    replay_fields = dict(replay.contract.declared_fields())
    shadow_fields = dict(shadow.contract.declared_fields())
    differing = tuple(
        name
        for name, value in replay_fields.items()
        if name not in shadow_fields or shadow_fields[name] != value
    )
    differing += tuple(name for name in shadow_fields if name not in replay_fields)

    parity = TemporalParity(replay=replay, shadow=shadow, differing_fields=differing)

    return TemporalParity(
        replay=replay,
        shadow=shadow,
        differing_fields=differing,
        detail=parity.describe(),
    )


@dataclass(frozen=True)
class ShadowPlan:
    """
    What a shadow period was declared to be, before it started.

    The declared observation count is a property of the strategy's trade frequency and the period's
    length, which is why it is a field of the plan rather than of the report: a report that chose
    its own threshold after seeing the period could always be made to pass one. The plan's digest
    is carried by the report it produced, so what a report was declared against is checkable.

    Parameters
    ----------
    candidate_id : str
        The frozen candidate the period observes.
    specification_digest : str
        The specification the candidate was frozen at.
    period : str
        The period, as declared.
    minimum_observations : int
        The observation count declared before the period started.

    """

    candidate_id: str
    specification_digest: str
    period: str
    minimum_observations: int

    def to_dict(self) -> dict[str, object]:
        """
        Return the plan as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "candidate_id": self.candidate_id,
            "specification_digest": self.specification_digest,
            "period": self.period,
            "minimum_observations": self.minimum_observations,
        }

    def digest(self) -> str:
        """
        Return the digest of the plan.

        Returns
        -------
        str
            The `sha256:<hex>` digest.

        """
        return digest_of(canonical_mapping(self.to_dict()))


@dataclass(frozen=True)
class ShadowObservation:
    """
    One observation of a shadow period, which is what makes a divergence visible.

    The six fields of design Section 11.5 are all here, and the observed cost is carried beside the
    modelled one so the report's cost difference is computed rather than asserted. A non-fill is an
    observation with no fill and a reason, not a missing record: the reason is what separates a
    security that did not trade from one the feed never delivered.

    Parameters
    ----------
    signal_at : int
        The signal time the feed reported, in nanoseconds.
    intended_entry : bool
        Whether the candidate's signal intended an entry.
    eligible_fill_at : int
        The eligible fill time under the declared eligibility rule, in nanoseconds.
    fill_price : Decimal | None, default None
        The simulated fill's price, or None when the observation did not fill.
    modelled_cost : Decimal | None, default None
        The cost the recorded cost configuration models for the observation.
    observed_cost : Decimal | None, default None
        The cost implied by the observed prices and quotes.
    non_fill_reason : str, default ""
        Why the observation did not fill, present exactly when it did not.

    """

    signal_at: int
    intended_entry: bool
    eligible_fill_at: int
    fill_price: Decimal | None = None
    modelled_cost: Decimal | None = None
    observed_cost: Decimal | None = None
    non_fill_reason: str = ""

    @property
    def filled(self) -> bool:
        """
        Whether the observation simulated a fill.
        """
        return self.fill_price is not None

    def to_dict(self) -> dict[str, object]:
        """
        Return the observation as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "signal_at": self.signal_at,
            "intended_entry": self.intended_entry,
            "eligible_fill_at": self.eligible_fill_at,
            "fill_price": None if self.fill_price is None else str(self.fill_price),
            "modelled_cost": None if self.modelled_cost is None else str(self.modelled_cost),
            "observed_cost": None if self.observed_cost is None else str(self.observed_cost),
            "non_fill_reason": self.non_fill_reason,
            "filled": self.filled,
        }


@dataclass(frozen=True)
class SignalDiscrepancy:
    """
    One observation whose signal time is not an instant the replay decided at.

    Parameters
    ----------
    expected_at : int
        The nearest instant the replay was expected to decide at, in nanoseconds.
    observed_at : int
        The instant the feed reported, in nanoseconds.
    magnitude : int
        How far the observation is from that instant, in nanoseconds. A duplicate of an instant the
        replay already decided at is reported with a magnitude of zero rather than dropped.

    """

    expected_at: int
    observed_at: int
    magnitude: int

    def to_dict(self) -> dict[str, object]:
        """
        Return the discrepancy as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "expected_at": self.expected_at,
            "observed_at": self.observed_at,
            "magnitude": self.magnitude,
        }


@dataclass(frozen=True)
class NonFill:
    """
    One simulated non-fill, with the reason rather than only the fact.

    Parameters
    ----------
    signal_at : int
        The signal time of the observation that did not fill, in nanoseconds.
    intended_entry : bool
        Whether that observation intended an entry.
    reason : str
        Why it did not fill.

    """

    signal_at: int
    intended_entry: bool
    reason: str

    def to_dict(self) -> dict[str, object]:
        """
        Return the non-fill as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "signal_at": self.signal_at,
            "intended_entry": self.intended_entry,
            "reason": self.reason,
        }


@dataclass(frozen=True)
class ShadowReport:
    """
    What a shadow period observed, as divergence rather than as a summary.

    The report is a report about integration and operational behaviour: it says whether signals,
    timing, data delivery and simulated execution behaved as the replay expected them to on live
    inputs. It is not evidence of an edge, and no field here is a performance statistic. The
    observation count is reported beside the count the plan declared, so a period that did not reach
    its pre-registered count says so instead of being read as a result.

    Parameters
    ----------
    plan : ShadowPlan
        The declaration the period ran under.
    contract : TemporalContract
        The declared fields the shadow run took its decisions under.
    observations : tuple[ShadowObservation, ...], default ()
        The observations the feed delivered, in the order it delivered them.
    expected_signal_times : tuple[int, ...], default ()
        The instants the replay was expected to decide at, in the order it decided.
    label : str, default "simulated"
        The label every shadow result carries.

    """

    plan: ShadowPlan
    contract: TemporalContract
    observations: tuple[ShadowObservation, ...] = ()
    expected_signal_times: tuple[int, ...] = ()
    label: str = SIMULATED_LABEL

    @property
    def schema(self) -> str:
        """
        The schema this report was produced under.
        """
        return SHADOW_SCHEMA

    @property
    def eligible_decisions(self) -> int:
        """
        How many decisions the replay was expected to take in the period.
        """
        return len(self.expected_signal_times)

    @property
    def observed_decisions(self) -> int:
        """
        How many observations the feed delivered.
        """
        return len(self.observations)

    @property
    def reached(self) -> int:
        """
        The count the report states, which is the observed decision count.
        """
        return self.observed_decisions

    @property
    def readable(self) -> bool:
        """
        Whether the period reached the count declared before it started.
        """
        return self.reached >= self.plan.minimum_observations

    @property
    def unreadable_reason(self) -> str:
        """
        Why the report may not be read yet, or the empty string when it may.
        """
        if self.readable:
            return ""

        return (
            f"the period reached {self.reached} observations against the "
            f"{self.plan.minimum_observations} declared before it started"
        )

    @property
    def missing_observations(self) -> int:
        """
        How many of the eligible decisions the feed never delivered an observation for.
        """
        delivered = {observation.signal_at for observation in self.observations}

        return sum(1 for instant in self.expected_signal_times if instant not in delivered)

    @property
    def signal_discrepancies(self) -> tuple[SignalDiscrepancy, ...]:
        """
        Each observation whose signal time is not an instant the replay decided at.
        """
        expected = set(self.expected_signal_times)
        counts = Counter(observation.signal_at for observation in self.observations)
        found: list[SignalDiscrepancy] = []

        for observation in self.observations:
            if observation.signal_at in expected and counts[observation.signal_at] == 1:
                continue
            nearest = _nearest(observation.signal_at, self.expected_signal_times)
            found.append(
                SignalDiscrepancy(
                    expected_at=nearest,
                    observed_at=observation.signal_at,
                    magnitude=abs(observation.signal_at - nearest),
                ),
            )

        return tuple(found)

    @property
    def non_fills(self) -> tuple[NonFill, ...]:
        """
        Each simulated non-fill, with the reason for it.

        A non-fill is an intended entry that produced no fill. An observation that intended nothing
        is neither a fill nor a non-fill, so a quiet period is not reported as divergence.
        """
        return tuple(
            NonFill(
                signal_at=observation.signal_at,
                intended_entry=observation.intended_entry,
                reason=observation.non_fill_reason,
            )
            for observation in self.observations
            if observation.intended_entry and not observation.filled
        )

    @property
    def modelled_cost(self) -> Decimal:
        """
        The total cost the recorded cost configuration modelled.
        """
        return sum(
            (
                observation.modelled_cost
                for observation in self.observations
                if observation.modelled_cost is not None
            ),
            Decimal(0),
        )

    @property
    def observed_cost(self) -> Decimal:
        """
        The total cost the observed prices and quotes imply.
        """
        return sum(
            (
                observation.observed_cost
                for observation in self.observations
                if observation.observed_cost is not None
            ),
            Decimal(0),
        )

    @property
    def cost_difference(self) -> Decimal:
        """
        The difference between the observed and the modelled cost.
        """
        return self.observed_cost - self.modelled_cost

    @property
    def diverged(self) -> bool:
        """
        Whether anything in the period diverged from the replay's expectation.
        """
        return (
            bool(self.signal_discrepancies) or bool(self.non_fills) or self.missing_observations > 0
        )

    def findings(self) -> tuple[str, ...]:
        """
        Return each divergence in one sentence, which is what a reader reads first.

        Returns
        -------
        tuple[str, ...]

        """
        found: list[str] = []
        if self.missing_observations:
            found.append(f"{self.missing_observations} observations were never delivered")
        found.extend(
            f"a signal arrived {discrepancy.magnitude} ns from the expected "
            f"{discrepancy.expected_at}"
            for discrepancy in self.signal_discrepancies
        )
        found.extend(
            f"the signal at {non_fill.signal_at} did not fill: {non_fill.reason}"
            for non_fill in self.non_fills
        )
        if self.cost_difference:
            found.append(
                f"the observed cost differed from the modelled one by {self.cost_difference}"
            )

        return tuple(found)

    def to_dict(self) -> dict[str, object]:
        """
        Return the report as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "schema": SHADOW_SCHEMA,
            "label": self.label,
            "plan": self.plan.to_dict(),
            "plan_digest": self.plan.digest(),
            "contract": self.contract.to_dict(),
            "contract_digest": self.contract.digest(),
            "eligible_decisions": self.eligible_decisions,
            "observed_decisions": self.observed_decisions,
            "minimum_observations": self.plan.minimum_observations,
            "reached": self.reached,
            "readable": self.readable,
            "unreadable_reason": self.unreadable_reason,
            "missing_observations": self.missing_observations,
            "signal_discrepancies": [
                discrepancy.to_dict() for discrepancy in self.signal_discrepancies
            ],
            "non_fills": [non_fill.to_dict() for non_fill in self.non_fills],
            "modelled_cost": str(self.modelled_cost),
            "observed_cost": str(self.observed_cost),
            "cost_difference": str(self.cost_difference),
            "diverged": self.diverged,
            "findings": list(self.findings()),
            "observations": [observation.to_dict() for observation in self.observations],
        }

    def digest(self) -> str:
        """
        Return the digest of the report.

        Returns
        -------
        str
            The `sha256:<hex>` digest.

        """
        return digest_of(canonical_mapping(self.to_dict()))


def _nearest(instant: int, candidates: Sequence[int]) -> int:
    """
    Return the candidate nearest an instant, or the instant itself when there are none.
    """
    if not candidates:
        return instant

    return min(candidates, key=lambda candidate: (abs(candidate - instant), candidate))


def shadow_period(
    plan: ShadowPlan,
    contract: TemporalContract,
    *,
    expected_signal_times: Sequence[int],
    observations: Sequence[ShadowObservation],
) -> ShadowReport:
    """
    Report what a shadow period observed against what the replay expected.

    Nothing here decides whether the candidate is good: the report states the divergence between a
    backtest's assumptions and a live feed's behaviour, which is what a shadow period can establish.
    An observation whose eligible fill time precedes its signal is refused rather than reported,
    because a fill that precedes its own signal is not a divergence but a broken record.

    Parameters
    ----------
    plan : ShadowPlan
        The declaration the period runs under, which fixes the observation count before it starts.
    contract : TemporalContract
        The declared fields the shadow run takes its decisions under.
    expected_signal_times : Sequence[int]
        The instants the replay was expected to decide at.
    observations : Sequence[ShadowObservation]
        The observations the feed delivered.

    Returns
    -------
    ShadowReport

    Raises
    ------
    ShadowError
        If the plan declared no observation count, or an observation is malformed.

    """
    if plan.minimum_observations < 1:
        raise ShadowError(
            ShadowRefusalCode.MINIMUM_NOT_DECLARED,
            f"the plan declares {plan.minimum_observations} as its minimum observation count",
        )

    for observation in observations:
        if observation.eligible_fill_at < observation.signal_at:
            raise ShadowError(
                ShadowRefusalCode.ELIGIBLE_BEFORE_SIGNAL,
                f"an observation at {observation.signal_at} is eligible at "
                f"{observation.eligible_fill_at}",
            )
        unexplained = observation.intended_entry and not observation.filled
        if unexplained and not observation.non_fill_reason:
            raise ShadowError(
                ShadowRefusalCode.NON_FILL_WITHOUT_REASON,
                f"the intended entry at {observation.signal_at} did not fill and gives no reason",
            )

    return ShadowReport(
        plan=plan,
        contract=contract,
        observations=tuple(observations),
        expected_signal_times=tuple(expected_signal_times),
    )
