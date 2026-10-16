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
Holdout custody: the confirmation state machine, its reservations and its access records.

Custody on one workstation is not a security boundary and this module does not pretend it is. What
it enforces is the record. The confirmation set is reserved once per campaign for one frozen
candidate; the reservation is bound to that candidate's identity, the access is written together
with it, no operation releases a reservation, and a verdict whose candidate identity no longer
matches the frozen one is void rather than merely suspicious.

The state machine is campaign-level rather than per-candidate, because retiring a candidate that has
been evaluated does not undo the information released by evaluating several of them: a reader who
has seen three confirmation results and picked the best has used the confirmation set to select,
whatever the ledger says about each individual retirement. A replacement candidate may therefore be
evaluated on the same data only as an *exploratory* result, and the label travels with the result
wherever it is shown.

The dataset is not independent because the system calls it confirmation. It is independent because
the record shows that nothing which influenced the candidate ever read it, which is a property of
the pair of identities rather than of the dataset on its own: a partition whose results influenced
an evolutionary lineage is development data *for that lineage*, and it stays locked for a lineage
that has never seen it.
"""

from __future__ import annotations

from dataclasses import dataclass
from dataclasses import field
from enum import Enum
from typing import TYPE_CHECKING

from nautilus_trader.discovery.schema import canonical_mapping
from nautilus_trader.optimization.space import digest_of


if TYPE_CHECKING:
    from collections.abc import Mapping
    from collections.abc import Sequence

    from nautilus_trader.discovery.ledger import CampaignInventory


class CustodyState(Enum):
    """
    The states of the confirmation state machine of design Section 11.3.
    """

    SEARCHING = "searching"
    FROZEN = "frozen"
    CONFIRMING = "confirming"
    CONFIRMED = "confirmed"
    WITHDRAWN = "withdrawn"
    EXPLORATORY_REUSE = "exploratory-reuse"


class CustodyRefusalCode(Enum):
    """
    The closed vocabulary a custody refusal is reported with.
    """

    NOT_FROZEN = "not_frozen"
    ALREADY_FROZEN = "already_frozen"
    FROZEN_CANDIDATE_MUTATED = "frozen_candidate_mutated"
    POLICY_CHANGED = "policy_changed"
    RESERVATION_CONSUMED = "reservation_consumed"
    RESERVATION_UNRESERVED = "reservation_unreserved"
    SECOND_CANDIDATE = "second_candidate"
    VERDICT_CANDIDATE_MISMATCH = "verdict_candidate_mismatch"
    EXPLORATORY_LABEL_REQUIRED = "exploratory_label_required"
    CONFIRMATION_ACCESS_WITHOUT_RESERVATION = "confirmation_access_without_reservation"
    SUMMARY_STATISTIC_IN_PARAMETERS = "summary_statistic_in_parameters"
    NOT_CONFIRMED = "not_confirmed"


class CustodyError(RuntimeError):
    """
    A refused custody operation, carrying the code from the closed vocabulary.

    Parameters
    ----------
    code : CustodyRefusalCode
        The refusal's code.
    message : str
        What was refused, in the vocabulary the coding standards require.

    """

    def __init__(self, code: CustodyRefusalCode, message: str) -> None:
        """
        Initialize the refusal.
        """
        super().__init__(f"{code.value}: {message}")
        self.code = code


@dataclass(frozen=True)
class Reservation:
    """
    A holdout reservation, which is spent once and never released.

    Parameters
    ----------
    reservation_id : str
        The digest of the reservation's declared fields.
    family_id : str
        The research family the reservation is held by. A successor campaign in the same family
        inherits the reservation rather than a fresh copy of the data.
    campaign_id : str
        The campaign that holds it.
    candidate_id : str
        The frozen candidate the reservation is bound to.
    dataset : str
        The identity of the confirmation dataset, which is its definition digest rather than a path.
    policy_digest : str
        The digest of the policy the candidate will be judged under.
    consumed : bool
        Whether the reservation has been spent. Nothing sets this back.

    """

    reservation_id: str
    family_id: str
    campaign_id: str
    candidate_id: str
    dataset: str
    policy_digest: str
    consumed: bool = False

    def to_dict(self) -> dict[str, object]:
        """
        Return the reservation as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "reservation_id": self.reservation_id,
            "family_id": self.family_id,
            "campaign_id": self.campaign_id,
            "candidate_id": self.candidate_id,
            "dataset": self.dataset,
            "policy_digest": self.policy_digest,
            "consumed": self.consumed,
        }


@dataclass(frozen=True)
class AccessRecord:
    """
    One read of the confirmation set, recorded with the experiment that requested it.

    Parameters
    ----------
    reservation_id : str
        The reservation the read consumed, or the empty string for a refused read.
    candidate_id : str
        The candidate the read is about.
    purpose : str
        What the read was for.
    exploratory : bool
        Whether the result is labelled exploratory. A replacement candidate's result carries the
        label, and the label travels with the result rather than living in a note beside it.
    refused_code : str
        The code a refused read was refused with, or the empty string when it was permitted.

    """

    reservation_id: str
    candidate_id: str
    purpose: str
    exploratory: bool = False
    refused_code: str = ""

    @property
    def label(self) -> str:
        """
        Return the label the result is shown under.
        """
        return "exploratory" if self.exploratory else "confirmation"

    def to_dict(self) -> dict[str, object]:
        """
        Return the record as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "reservation_id": self.reservation_id,
            "candidate_id": self.candidate_id,
            "purpose": self.purpose,
            "exploratory": self.exploratory,
            "refused_code": self.refused_code,
            "label": self.label,
        }


@dataclass(frozen=True)
class ConfirmationVerdict:
    """
    The one evaluation a campaign may cite as independent confirmation.

    Parameters
    ----------
    candidate_id : str
        The frozen candidate the verdict is bound to.
    reservation_id : str
        The reservation it consumed.
    policy_digest : str
        The policy the evaluation was made under.
    evidence_digest : str
        The digest of the evidence the evaluation produced.
    summary : str
        What the evaluation found, in the vocabulary the caller reports.

    """

    candidate_id: str
    reservation_id: str
    policy_digest: str
    evidence_digest: str
    summary: str = ""

    def to_dict(self) -> dict[str, object]:
        """
        Return the verdict as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "candidate_id": self.candidate_id,
            "reservation_id": self.reservation_id,
            "policy_digest": self.policy_digest,
            "evidence_digest": self.evidence_digest,
            "summary": self.summary,
        }


@dataclass(frozen=True)
class CustodyTransition:
    """
    One recorded transition, written with the reservation and the access it consumed.

    Parameters
    ----------
    state : CustodyState
        The state the campaign moved to.
    candidate_id : str
        The candidate the transition is about.
    policy_digest : str
        The digest of the policy in force at the transition.
    specification_digest : str
        The digest of the candidate's specification at the transition, which is what a later
        mutation is detected against.
    reservation_id : str
        The reservation the transition is bound to, or the empty string when it consumed none.
    access : AccessRecord | None
        The access written with it, or None when it read no holdout data.
    detail : Mapping[str, object]
        Anything else the transition records.

    """

    state: CustodyState
    candidate_id: str
    policy_digest: str = ""
    specification_digest: str = ""
    reservation_id: str = ""
    access: AccessRecord | None = None
    detail: Mapping[str, object] = field(default_factory=dict)

    def to_dict(self) -> dict[str, object]:
        """
        Return the transition as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "state": self.state.value,
            "candidate_id": self.candidate_id,
            "policy_digest": self.policy_digest,
            "specification_digest": self.specification_digest,
            "reservation_id": self.reservation_id,
            "access": None if self.access is None else self.access.to_dict(),
            "detail": dict(self.detail),
        }


@dataclass(frozen=True)
class WorkerEnvironmentCheck:
    """
    Whether a worker environment is allowed to see the confirmation set.

    Parameters
    ----------
    worker : str
        The worker the environment was checked for.
    findings : tuple[str, ...]
        What the environment names that it must not, empty when it names nothing.

    """

    worker: str
    findings: tuple[str, ...] = ()

    @property
    def holds(self) -> bool:
        """
        Whether the worker's environment is clean.
        """
        return not self.findings


class ConfirmationCustody:
    """
    The campaign-level confirmation state machine, with its reservations and access records.

    Every transition is written through the campaign inventory, so the state, the reservation, the
    access and the candidate identity are one record rather than four assertions that could drift
    apart. The machine refuses rather than warns: a second confirmation candidate, a mutation after
    a confirmation evaluation, a policy change under a frozen candidate, a read without a
    reservation and an unlabelled reuse are all `CustodyError`s with a code from the closed
    vocabulary.

    Parameters
    ----------
    inventory : CampaignInventory
        The inventory every transition is written to.
    dataset : str
        The identity of the confirmation dataset, which is its definition digest rather than a
        path.
    root : str, default ""
        The declared location of the confirmation set, recorded so that a worker environment can be
        checked against it. A location is not evidence of anything, which is why the checks below
        are about reservations and identities rather than about this string.
    inherited_reservations : Sequence[str], default ()
        The reservations the candidate's research family has already spent, read from a predecessor
        campaign's inventory. A branch inherits its family's confirmation history rather than a
        fresh copy of the data, so a non-empty sequence here is what refuses a successor campaign
        the access its family has spent.

    """

    def __init__(
        self,
        inventory: CampaignInventory,
        *,
        dataset: str,
        root: str = "",
        inherited_reservations: Sequence[str] = (),
    ) -> None:
        """
        Initialize the custody machine over a campaign's inventory.
        """
        if not dataset:
            raise ValueError("dataset must not be empty")

        self._inventory = inventory
        self._dataset = dataset
        self._root = root
        self._inherited = tuple(inherited_reservations)
        self._state = CustodyState.SEARCHING
        self._reservation: Reservation | None = None
        self._frozen_candidate = ""
        self._frozen_specification = ""
        self._frozen_policy = ""
        self._verdict: ConfirmationVerdict | None = None
        self._transitions: list[CustodyTransition] = []
        self._access: list[AccessRecord] = []

    @property
    def state(self) -> CustodyState:
        """
        The campaign's custody state.
        """
        return self._state

    @property
    def dataset(self) -> str:
        """
        The identity of the confirmation dataset.
        """
        return self._dataset

    @property
    def inherited_reservations(self) -> tuple[str, ...]:
        """
        The reservations the campaign's research family has already spent.
        """
        return self._inherited

    @property
    def root(self) -> str:
        """
        The declared location of the confirmation set.
        """
        return self._root

    @property
    def frozen_candidate(self) -> str:
        """
        The frozen confirmation candidate, or the empty string when none is frozen.
        """
        return self._frozen_candidate

    @property
    def reservation(self) -> Reservation | None:
        """
        The reservation the campaign holds, or None when it holds none.
        """
        return self._reservation

    @property
    def verdict(self) -> ConfirmationVerdict | None:
        """
        The one confirmation verdict the campaign may cite, or None.
        """
        return self._verdict

    @property
    def transitions(self) -> tuple[CustodyTransition, ...]:
        """
        Every recorded transition, in order.
        """
        return tuple(self._transitions)

    @property
    def access_records(self) -> tuple[AccessRecord, ...]:
        """
        Every read of the confirmation set, permitted or refused.
        """
        return tuple(self._access)

    def freeze(
        self,
        candidate_id: str,
        *,
        specification_digest: str,
        policy_digest: str,
        evidence_digest: str = "",
    ) -> CustodyTransition:
        """
        Designate one candidate the campaign's confirmation candidate and take its reservation.

        Parameters
        ----------
        candidate_id : str
            The candidate to freeze.
        specification_digest : str
            The digest of its specification, which a later mutation is detected against.
        policy_digest : str
            The digest of the policy it will be judged under.
        evidence_digest : str, default ""
            The digest of the evidence the freeze rests on.

        Returns
        -------
        CustodyTransition

        Raises
        ------
        CustodyError
            If the campaign has already frozen a candidate, or its reservation is spent.

        """
        if self._state is not CustodyState.SEARCHING:
            raise CustodyError(
                CustodyRefusalCode.ALREADY_FROZEN,
                f"the campaign is {self._state.value} and freezes one candidate once",
            )
        if self._inherited:
            raise CustodyError(
                CustodyRefusalCode.RESERVATION_CONSUMED,
                f"the research family has already spent {list(self._inherited)} against "
                f"{self._dataset}: a branch inherits its family's history rather than fresh data",
            )
        if self._reservation is not None and self._reservation.consumed:
            raise CustodyError(
                CustodyRefusalCode.RESERVATION_CONSUMED,
                "the family's confirmation reservation is spent and nothing releases it",
            )

        reservation = Reservation(
            reservation_id=digest_of(
                canonical_mapping(
                    {
                        "family_id": self._inventory.family_id,
                        "campaign_id": self._inventory.campaign_id,
                        "candidate_id": candidate_id,
                        "dataset": self._dataset,
                        "policy_digest": policy_digest,
                    },
                ),
            ),
            family_id=self._inventory.family_id,
            campaign_id=self._inventory.campaign_id,
            candidate_id=candidate_id,
            dataset=self._dataset,
            policy_digest=policy_digest,
        )

        self._reservation = reservation
        self._frozen_candidate = candidate_id
        self._frozen_specification = specification_digest
        self._frozen_policy = policy_digest

        return self._record(
            CustodyTransition(
                state=CustodyState.FROZEN,
                candidate_id=candidate_id,
                policy_digest=policy_digest,
                specification_digest=specification_digest,
                reservation_id=reservation.reservation_id,
                detail={"evidence_digest": evidence_digest, "dataset": self._dataset},
            ),
        )

    def guard_mutation(self, candidate_id: str, *, specification_digest: str) -> None:
        """
        Refuse a mutation of the frozen candidate, which is the guard the search calls.

        Parameters
        ----------
        candidate_id : str
            The candidate the search intends to mutate.
        specification_digest : str
            The digest the mutation would produce or the candidate's current one.

        Raises
        ------
        CustodyError
            If the candidate is the frozen one and the campaign has left `SEARCHING`.

        """
        if self._state is CustodyState.SEARCHING or candidate_id != self._frozen_candidate:
            return

        raise CustodyError(
            CustodyRefusalCode.FROZEN_CANDIDATE_MUTATED,
            f"the candidate {candidate_id} is frozen under {self._frozen_policy} and may not be "
            f"mutated (mutation to {specification_digest})",
        )

    def guard_policy(self, policy_digest: str) -> None:
        """
        Refuse a policy change under a frozen candidate.

        Parameters
        ----------
        policy_digest : str
            The digest of the policy that would be used.

        Raises
        ------
        CustodyError
            If the candidate is frozen and the policy is not the one it was frozen under.

        """
        if self._frozen_policy and policy_digest != self._frozen_policy:
            raise CustodyError(
                CustodyRefusalCode.POLICY_CHANGED,
                f"the frozen candidate is judged under {self._frozen_policy} and the policy would "
                f"become {policy_digest}",
            )

    def read_holdout(
        self,
        candidate_id: str,
        *,
        purpose: str,
        exploratory: bool = False,
        summary_statistics: Mapping[str, float] | None = None,
    ) -> AccessRecord:
        """
        Read the confirmation set for the frozen candidate, once per campaign.

        A read is permitted for exactly one candidate under exactly one reservation, and it consumes
        that reservation. A read for any other candidate is refused with the code that names why,
        and the refusal is recorded as an access record rather than thrown away: a refused read is
        evidence about the workflow.

        Parameters
        ----------
        candidate_id : str
            The candidate the read is for.
        purpose : str
            What the read is for.
        exploratory : bool, default False
            Whether the result is labelled exploratory. A read that is not the campaign's single
            confirmation read must be labelled, or it is refused.
        summary_statistics : Mapping[str, float] | None, default None
            Statistics the caller declares it will carry into the candidate's search-phase inputs,
            or None when it carries none. Carrying the confirmation set's summary statistics into a
            candidate's parameters is what quarantines a candidate rather than confirming it.

        Returns
        -------
        AccessRecord

        Raises
        ------
        CustodyError
            If the read is not permitted, after the refused access has been recorded.

        """
        if summary_statistics:
            self._record_access(
                AccessRecord(
                    reservation_id="",
                    candidate_id=candidate_id,
                    purpose=purpose,
                    exploratory=exploratory,
                    refused_code=CustodyRefusalCode.SUMMARY_STATISTIC_IN_PARAMETERS.value,
                ),
            )
            raise CustodyError(
                CustodyRefusalCode.SUMMARY_STATISTIC_IN_PARAMETERS,
                f"the read would carry {sorted(summary_statistics)} into the candidate's "
                f"search-phase inputs, which quarantines it rather than confirming it",
            )

        reservation = self._reservation
        refusal = self._refuse_read(candidate_id, reservation, exploratory=exploratory)
        if refusal is not None or reservation is None:
            code = CustodyRefusalCode.RESERVATION_UNRESERVED if refusal is None else refusal
            self._record_access(
                AccessRecord(
                    reservation_id="" if reservation is None else reservation.reservation_id,
                    candidate_id=candidate_id,
                    purpose=purpose,
                    exploratory=exploratory,
                    refused_code=code.value,
                ),
            )
            raise CustodyError(code, self._refusal_message(candidate_id, code))

        # A read for the frozen candidate consumes the campaign's one reservation. A labelled read
        # for another candidate is exploratory every time it happens, so it consumes nothing: the
        # result may be read, and it may never be cited as independent.
        if candidate_id == self._frozen_candidate:
            self._reservation = Reservation(
                reservation_id=reservation.reservation_id,
                family_id=reservation.family_id,
                campaign_id=reservation.campaign_id,
                candidate_id=reservation.candidate_id,
                dataset=reservation.dataset,
                policy_digest=reservation.policy_digest,
                consumed=True,
            )
        access = AccessRecord(
            reservation_id=reservation.reservation_id,
            candidate_id=candidate_id,
            purpose=purpose,
            exploratory=exploratory,
        )
        self._record_access(access)
        self._record(
            CustodyTransition(
                state=(CustodyState.EXPLORATORY_REUSE if exploratory else CustodyState.CONFIRMING),
                candidate_id=candidate_id,
                policy_digest=reservation.policy_digest,
                specification_digest=self._frozen_specification,
                reservation_id=reservation.reservation_id,
                access=access,
                detail={"purpose": purpose, "label": access.label},
            ),
        )

        return access

    def confirm(
        self,
        candidate_id: str,
        *,
        policy_digest: str,
        evidence_digest: str,
        summary: str = "",
    ) -> ConfirmationVerdict:
        """
        Record the campaign's one confirmation verdict, bound to the frozen candidate's identity.

        Parameters
        ----------
        candidate_id : str
            The candidate the verdict is about, which must be the frozen one.
        policy_digest : str
            The digest of the policy the evaluation was made under.
        evidence_digest : str
            The digest of the evidence the evaluation produced.
        summary : str, default ""
            What the evaluation found.

        Returns
        -------
        ConfirmationVerdict

        Raises
        ------
        CustodyError
            If the candidate is not the frozen one, the campaign read the holdout for no candidate,
            or the policy is not the one the candidate was frozen under.

        """
        if candidate_id != self._frozen_candidate:
            raise CustodyError(
                CustodyRefusalCode.VERDICT_CANDIDATE_MISMATCH,
                f"the verdict names {candidate_id} and the frozen candidate is "
                f"{self._frozen_candidate or 'none'}: a verdict whose identity does not match is "
                f"void rather than suspicious",
            )
        self.guard_policy(policy_digest)

        reservation = self._reservation
        if reservation is None or not reservation.consumed:
            raise CustodyError(
                CustodyRefusalCode.NOT_CONFIRMED,
                "the campaign has read no holdout data for its frozen candidate, so it has no "
                "confirmation result to cite",
            )

        verdict = ConfirmationVerdict(
            candidate_id=candidate_id,
            reservation_id=reservation.reservation_id,
            policy_digest=policy_digest,
            evidence_digest=evidence_digest,
            summary=summary,
        )
        self._verdict = verdict
        self._record(
            CustodyTransition(
                state=CustodyState.CONFIRMED,
                candidate_id=candidate_id,
                policy_digest=policy_digest,
                specification_digest=self._frozen_specification,
                reservation_id=reservation.reservation_id,
                detail={"evidence_digest": evidence_digest, "summary": summary},
            ),
        )

        return verdict

    def withdraw(self, candidate_id: str, *, reason: str) -> CustodyTransition:
        """
        Retire the frozen candidate, keeping its spent reservation.

        Parameters
        ----------
        candidate_id : str
            The candidate to retire.
        reason : str
            Why it was retired.

        Returns
        -------
        CustodyTransition

        """
        self._verdict = None

        return self._record(
            CustodyTransition(
                state=CustodyState.WITHDRAWN,
                candidate_id=candidate_id,
                policy_digest=self._frozen_policy,
                specification_digest=self._frozen_specification,
                detail={"reason": reason},
            ),
        )

    def citation(self, candidate_id: str) -> ConfirmationVerdict:
        """
        Return the verdict a promotion decision may cite for a candidate.

        A verdict is citable for exactly the candidate it was bound to. A replacement candidate,
        or a successor campaign in the same family, has no verdict to cite: the honest options are
        newly reserved, genuinely unseen data or a separately pre-registered protocol, and a new
        search branch does not restore access to data the family has spent.

        Parameters
        ----------
        candidate_id : str
            The candidate a decision wants to cite.

        Returns
        -------
        ConfirmationVerdict

        Raises
        ------
        CustodyError
            If there is no verdict, or the verdict belongs to another candidate.

        """
        if self._verdict is None:
            raise CustodyError(
                CustodyRefusalCode.NOT_CONFIRMED,
                f"the campaign holds no confirmation verdict for {candidate_id}",
            )
        if self._verdict.candidate_id != candidate_id:
            raise CustodyError(
                CustodyRefusalCode.VERDICT_CANDIDATE_MISMATCH,
                f"the verdict is bound to {self._verdict.candidate_id} and cannot be cited for "
                f"{candidate_id}",
            )
        if self._state is not CustodyState.CONFIRMED:
            raise CustodyError(
                CustodyRefusalCode.NOT_CONFIRMED,
                f"the campaign is {self._state.value} and its verdict is void",
            )

        return self._verdict

    def worker_environment(
        self,
        worker: str,
        environment: Mapping[str, str],
    ) -> WorkerEnvironmentCheck:
        """
        Check that a worker's environment does not name the confirmation set.

        Parameters
        ----------
        worker : str
            The worker the environment belongs to.
        environment : Mapping[str, str]
            The environment the worker would be started with.

        Returns
        -------
        WorkerEnvironmentCheck

        """
        findings: list[str] = []
        if self._root:
            for name, value in sorted(environment.items()):
                if self._root in value:
                    findings.append(f"{name} names the confirmation root")

        return WorkerEnvironmentCheck(worker=worker, findings=tuple(findings))

    def _refuse_read(
        self,
        candidate_id: str,
        reservation: Reservation | None,
        *,
        exploratory: bool,
    ) -> CustodyRefusalCode | None:
        """
        Return the code a read is refused with, or None when it is permitted.
        """
        if not self._frozen_candidate or self._state in (
            CustodyState.SEARCHING,
            CustodyState.WITHDRAWN,
        ):
            return CustodyRefusalCode.NOT_FROZEN
        if candidate_id != self._frozen_candidate:
            if not exploratory:
                return CustodyRefusalCode.SECOND_CANDIDATE

            return None
        if self._inherited and reservation is None:
            return CustodyRefusalCode.RESERVATION_CONSUMED
        if reservation is None or reservation.consumed:
            return CustodyRefusalCode.RESERVATION_CONSUMED

        return None

    def _refusal_message(self, candidate_id: str, code: CustodyRefusalCode) -> str:
        """
        Return what a refused read is refused with, in the vocabulary the standards require.
        """
        match code:
            case CustodyRefusalCode.NOT_FROZEN:
                return (
                    f"the campaign is {self._state.value} and no candidate is frozen for "
                    f"confirmation, so the confirmation set may not be read"
                )
            case CustodyRefusalCode.SECOND_CANDIDATE:
                return (
                    f"the read is for {candidate_id} and the campaign confirms "
                    f"{self._frozen_candidate}: one campaign produces one confirmation result it "
                    f"may cite as independent"
                )
            case _:
                return (
                    f"the family's reservation for {self._dataset} is spent and nothing releases "
                    f"it: a replacement needs newly reserved, unseen data or a separately "
                    f"pre-registered protocol"
                )

    def _record_access(self, access: AccessRecord) -> None:
        """
        Keep the access record and write it through the inventory.
        """
        self._access.append(access)
        self._inventory.record_custody_event(
            access.candidate_id,
            transition="access",
            detail=access.to_dict(),
        )

    def _record(self, transition: CustodyTransition) -> CustodyTransition:
        """
        Record a transition and write it, with its reservation and access, through the inventory.
        """
        self._state = transition.state
        self._transitions.append(transition)
        self._inventory.record_custody_event(
            transition.candidate_id,
            transition=transition.state.value,
            detail={
                **transition.to_dict(),
                "dataset": self._dataset,
                "frozen_candidate": self._frozen_candidate,
            },
        )

        return transition

    def to_dict(self) -> dict[str, object]:
        """
        Return the machine's state as plain data.

        Returns
        -------
        dict[str, object]

        """
        return {
            "dataset": self._dataset,
            "root": self._root,
            "state": self._state.value,
            "frozen_candidate": self._frozen_candidate,
            "frozen_policy": self._frozen_policy,
            "inherited_reservations": list(self._inherited),
            "reservation": None if self._reservation is None else self._reservation.to_dict(),
            "verdict": None if self._verdict is None else self._verdict.to_dict(),
            "transitions": [transition.to_dict() for transition in self._transitions],
            "access": [record.to_dict() for record in self._access],
        }


def inherited_reservations(
    transitions: Sequence[CustodyTransition],
) -> tuple[str, ...]:
    """
    Return the reservations a custody history spent, which a successor campaign inherits.

    A branch inherits its family's confirmation history rather than a fresh copy of the data, so a
    successor campaign that reads this history knows what its family has already spent.

    Parameters
    ----------
    transitions : Sequence[CustodyTransition]
        The recorded transitions.

    Returns
    -------
    tuple[str, ...]
        The distinct reservation identities, in the order they were taken.

    """
    taken = [transition.reservation_id for transition in transitions if transition.reservation_id]

    return tuple(dict.fromkeys(taken))
