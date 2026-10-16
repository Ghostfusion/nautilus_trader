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
Tests for holdout custody: the confirmation state machine, its reservations and its refusals.

The five negative cases of the design's criterion 9 are asserted by refusal code rather than by
message: a second candidate, a mutation after a confession evaluation, a winner chosen from several
results, a branch expecting fresh access, and a reused evaluation presented without its
exploratory label.
"""

from __future__ import annotations

from pathlib import Path

import pytest

from nautilus_trader.discovery.custody import ConfirmationCustody
from nautilus_trader.discovery.custody import CustodyError
from nautilus_trader.discovery.custody import CustodyRefusalCode
from nautilus_trader.discovery.custody import CustodyState
from nautilus_trader.discovery.custody import inherited_reservations
from nautilus_trader.discovery.ledger import CampaignInventory
from nautilus_trader.discovery.ledger import EventKind
from nautilus_trader.discovery.ledger import research_family_id


CANDIDATE = "sha256:" + "a" * 64
REPLACEMENT = "sha256:" + "b" * 64
POLICY = "sha256:" + "c" * 64
DATASET = "sha256:" + "d" * 64
ROOT = "E:/holdout/confirmation"
FAMILY = research_family_id(snapshot="snapshot-1", selection_process="rank_first")


def inventory(tmp_path: Path, campaign: str = "campaign-1") -> CampaignInventory:
    """
    Return a campaign inventory of the fixture research family.
    """
    return CampaignInventory(
        tmp_path / campaign,
        writer="orchestrator",
        campaign_id=campaign,
        family_id=FAMILY,
    )


def custody(
    campaign: CampaignInventory,
    *,
    inherited: tuple[str, ...] = (),
) -> ConfirmationCustody:
    """
    Return a custody machine over the fixture campaign.
    """
    return ConfirmationCustody(
        campaign,
        dataset=DATASET,
        root=ROOT,
        inherited_reservations=inherited,
    )


def frozen(tmp_path: Path) -> tuple[CampaignInventory, ConfirmationCustody]:
    """
    Return a campaign whose fixture candidate is frozen for confirmation.
    """
    campaign = inventory(tmp_path)
    machine = custody(campaign)
    machine.freeze(
        CANDIDATE,
        specification_digest="sha256:specification",
        policy_digest=POLICY,
        evidence_digest="sha256:evidence",
    )

    return campaign, machine


def test_the_confirmation_set_cannot_be_read_while_the_search_is_running(tmp_path: Path) -> None:
    """
    A searching campaign has no reservation, so a read is refused and the refusal is recorded.
    """
    machine = custody(inventory(tmp_path))

    assert machine.state is CustodyState.SEARCHING
    with pytest.raises(CustodyError) as refused:
        machine.read_holdout(CANDIDATE, purpose="peek")
    assert refused.value.code is CustodyRefusalCode.NOT_FROZEN
    assert machine.access_records[0].refused_code == "not_frozen"
    assert machine.access_records[0].candidate_id == CANDIDATE


def test_a_frozen_candidate_may_not_be_mutated(tmp_path: Path) -> None:
    """
    The first negative case: a mutation after the campaign leaves searching is refused.
    """
    _, machine = frozen(tmp_path)

    with pytest.raises(CustodyError) as refused:
        machine.guard_mutation(CANDIDATE, specification_digest="sha256:mutated")
    assert refused.value.code is CustodyRefusalCode.FROZEN_CANDIDATE_MUTATED

    # A different candidate is unaffected: the freeze binds one candidate, not the search.
    machine.guard_mutation(REPLACEMENT, specification_digest="sha256:mutated")


def test_the_policy_under_a_frozen_candidate_may_not_change(tmp_path: Path) -> None:
    """
    A frozen candidate is judged under the policy it was frozen with.
    """
    _, machine = frozen(tmp_path)

    machine.guard_policy(POLICY)
    with pytest.raises(CustodyError) as refused:
        machine.guard_policy("sha256:other-policy")
    assert refused.value.code is CustodyRefusalCode.POLICY_CHANGED


def test_a_second_candidate_is_refused_and_one_reservation_runs_once(tmp_path: Path) -> None:
    """
    The second and third negative cases: one campaign produces one result it may cite.

    The reservation is consumed by the first read and nothing releases it, so a replacement
    candidate cannot read the same data to help choose a winner.
    """
    _, machine = frozen(tmp_path)

    with pytest.raises(CustodyError) as refused:
        machine.read_holdout(REPLACEMENT, purpose="second candidate")
    assert refused.value.code is CustodyRefusalCode.SECOND_CANDIDATE

    access = machine.read_holdout(CANDIDATE, purpose="the campaign's confirmation")
    assert access.label == "confirmation"
    assert access.exploratory is False
    assert machine.reservation is not None
    assert machine.reservation.consumed is True
    assert machine.state is CustodyState.CONFIRMING

    with pytest.raises(CustodyError) as spent:
        machine.read_holdout(CANDIDATE, purpose="again")
    assert spent.value.code is CustodyRefusalCode.RESERVATION_CONSUMED

    with pytest.raises(CustodyError) as reused:
        machine.read_holdout(REPLACEMENT, purpose="a replacement")
    assert reused.value.code is CustodyRefusalCode.SECOND_CANDIDATE


def test_a_reused_evaluation_carries_its_exploratory_label(tmp_path: Path) -> None:
    """
    The fifth negative case: a replacement's read is labelled, and the label travels with it.

    A replacement candidate may be evaluated on the same data only as an exploratory result, which
    it carries wherever it is shown; presenting it without the label is refused, and the labelled
    result may never be cited as the campaign's independent confirmation.
    """
    _, machine = frozen(tmp_path)

    with pytest.raises(CustodyError) as refused:
        machine.read_holdout(REPLACEMENT, purpose="choose a winner")
    assert refused.value.code is CustodyRefusalCode.SECOND_CANDIDATE

    access = machine.read_holdout(REPLACEMENT, purpose="reuse the data", exploratory=True)
    assert access.exploratory is True
    assert access.label == "exploratory"
    assert machine.state is CustodyState.EXPLORATORY_REUSE
    assert machine.reservation is not None
    assert machine.reservation.consumed is False, "an exploratory read consumes nothing"

    with pytest.raises(CustodyError) as cited:
        machine.citation(REPLACEMENT)
    assert cited.value.code is CustodyRefusalCode.NOT_CONFIRMED


def test_a_branch_inherits_its_family_spent_reservation(tmp_path: Path) -> None:
    """
    The fourth negative case: a new branch does not restore access to the family's data.

    A successor campaign reads its predecessor's history and declares the reservations the family
    spent, which refuses it the freeze and the read that its predecessor already used.
    """
    predecessor, machine = frozen(tmp_path)
    machine.read_holdout(CANDIDATE, purpose="confirmation")
    spent = inherited_reservations(machine.transitions)
    assert machine.reservation is not None
    assert spent == (machine.reservation.reservation_id,)

    successor_inventory = inventory(tmp_path, "campaign-2")
    successor = custody(successor_inventory, inherited=spent)
    assert successor.inherited_reservations == spent

    with pytest.raises(CustodyError) as refused:
        successor.freeze(
            REPLACEMENT,
            specification_digest="sha256:replacement",
            policy_digest=POLICY,
        )
    assert refused.value.code is CustodyRefusalCode.RESERVATION_CONSUMED

    # The family's history is read from the predecessor's own inventory, and it holds no artifacts.
    assert predecessor.family_id == successor_inventory.family_id


def test_a_verdict_is_bound_to_the_candidate_it_was_frozen_for(tmp_path: Path) -> None:
    """
    A verdict whose candidate identity does not match is void rather than merely suspicious.
    """
    _, machine = frozen(tmp_path)
    machine.read_holdout(CANDIDATE, purpose="confirmation")

    with pytest.raises(CustodyError) as mismatched:
        machine.confirm(REPLACEMENT, policy_digest=POLICY, evidence_digest="sha256:holdout")
    assert mismatched.value.code is CustodyRefusalCode.VERDICT_CANDIDATE_MISMATCH

    verdict = machine.confirm(
        CANDIDATE,
        policy_digest=POLICY,
        evidence_digest="sha256:holdout",
        summary="holds",
    )
    assert machine.state is CustodyState.CONFIRMED
    assert machine.citation(CANDIDATE) == verdict

    with pytest.raises(CustodyError) as cited:
        machine.citation(REPLACEMENT)
    assert cited.value.code is CustodyRefusalCode.VERDICT_CANDIDATE_MISMATCH


def test_carrying_the_confirmation_statistics_into_the_search_quarantines_it(
    tmp_path: Path,
) -> None:
    """
    A read that would carry the holdout's summary statistics into the search is refused.
    """
    _, machine = frozen(tmp_path)

    with pytest.raises(CustodyError) as refused:
        machine.read_holdout(
            CANDIDATE,
            purpose="tune the parameters",
            summary_statistics={"sharpe": 9.0},
        )
    assert refused.value.code is CustodyRefusalCode.SUMMARY_STATISTIC_IN_PARAMETERS
    assert machine.reservation is not None
    assert machine.reservation.consumed is False


def test_a_worker_environment_that_names_the_confirmation_root_is_reported(tmp_path: Path) -> None:
    """
    Discovery workers are not given the confirmation set's path, and the check says when one is.
    """
    _, machine = frozen(tmp_path)

    clean = machine.worker_environment("worker-1", {"NAUTILUS_ENV": "discovery"})
    assert clean.holds is True

    named = machine.worker_environment("worker-2", {"HOLDOUT_DIR": f"{ROOT}/2024"})
    assert named.holds is False
    assert "HOLDOUT_DIR names the confirmation root" in named.findings


def test_every_transition_is_written_with_its_reservation_and_access(tmp_path: Path) -> None:
    """
    The state, the reservation, the access and the candidate identity are one record.
    """
    campaign, machine = frozen(tmp_path)
    machine.read_holdout(CANDIDATE, purpose="confirmation")
    machine.confirm(CANDIDATE, policy_digest=POLICY, evidence_digest="sha256:holdout")

    events = [event for event in campaign.events() if event.kind is EventKind.CUSTODY_EVENT]
    transitions = [str(event.payload["transition"]) for event in events]
    assert transitions.count("frozen") == 1
    assert transitions.count("confirming") == 1
    assert transitions.count("confirmed") == 1
    assert transitions.count("access") == 1

    recorded = machine.to_dict()
    assert recorded["state"] == "confirmed"
    assert recorded["frozen_candidate"] == CANDIDATE
    assert recorded["verdict"]["candidate_id"] == CANDIDATE  # type: ignore[index]
    assert campaign.verify_chain(checkpoint=campaign.checkpoint()).ok


def test_freezing_twice_is_refused(tmp_path: Path) -> None:
    """
    One campaign freezes one candidate.
    """
    _, machine = frozen(tmp_path)

    with pytest.raises(CustodyError) as refused:
        machine.freeze(REPLACEMENT, specification_digest="sha256:replacement", policy_digest=POLICY)
    assert refused.value.code is CustodyRefusalCode.ALREADY_FROZEN


def test_a_confirmation_without_a_holdout_read_is_refused(tmp_path: Path) -> None:
    """
    A campaign with no holdout evaluation has no confirmation result to cite.
    """
    _, machine = frozen(tmp_path)

    with pytest.raises(CustodyError) as refused:
        machine.confirm(CANDIDATE, policy_digest=POLICY, evidence_digest="sha256:holdout")
    assert refused.value.code is CustodyRefusalCode.NOT_CONFIRMED


def test_a_withdrawn_campaign_holds_no_citable_verdict(tmp_path: Path) -> None:
    """
    Retirement keeps the spent reservation and voids the verdict.
    """
    _, machine = frozen(tmp_path)
    machine.read_holdout(CANDIDATE, purpose="confirmation")
    machine.confirm(CANDIDATE, policy_digest=POLICY, evidence_digest="sha256:holdout")
    machine.withdraw(CANDIDATE, reason="a superseding candidate")

    assert machine.state is CustodyState.WITHDRAWN
    assert machine.verdict is None
    assert machine.reservation is not None
    assert machine.reservation.consumed is True

    with pytest.raises(CustodyError) as refused:
        machine.citation(CANDIDATE)
    assert refused.value.code is CustodyRefusalCode.NOT_CONFIRMED
