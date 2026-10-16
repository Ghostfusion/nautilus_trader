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
`FX-M6-4`: the whole search is auditable after the fact, which is design criterion 16.

The criterion names five questions a reader must be able to answer from the ledger, the policy digest
and the engine version: which candidates existed, which ran, which were judged, under which policy,
and why each was rejected. This fixture runs a campaign that has an answer to every one of them - a
compiled candidate whose execution verified, a mutation of it that crashed and was left incomplete, a
proposal the validator refused, a generation that recorded its search-policy digest, and a gate
service that judged the compiled candidate - and then reconstructs the campaign from the ledger alone.

The audit is a derived view rather than a new record, so the fixture also checks the two things that
make it usable as evidence: the chain it reads verifies against its own head, and the same chain with
the same declared inputs produces the same digest. A gate's reason is read back from the ledger and
compared with the report the service returned, because the fact that a gate stopped a candidate and
the reason it did are different statements and only the second explains a rejection.
"""

from __future__ import annotations

import json
from pathlib import Path

from nautilus_trader.discovery.gates import GateEvidence
from nautilus_trader.discovery.gates import GatePolicy
from nautilus_trader.discovery.gates import GateReport
from nautilus_trader.discovery.gates import GateService
from nautilus_trader.discovery.ledger import AttemptOutcome
from nautilus_trader.discovery.ledger import CampaignInventory
from nautilus_trader.discovery.ledger import IdempotencyKey
from nautilus_trader.discovery.ledger import RunReceipt
from nautilus_trader.discovery.ledger import audit_campaign
from nautilus_trader.discovery.ledger import research_family_id


CANDIDATE = "sha256:" + "a" * 64
DESCENDANT = "sha256:" + "b" * 64
REFUSED = "sha256:" + "c" * 64
SPECIFICATION = "sha256:" + "d" * 64
MUTATED = "sha256:" + "e" * 64
SEARCH_POLICY = "sha256:" + "7" * 64
POLICY = "sha256:" + "9" * 64
ENGINE_VERSION = "nautilus_trader 2.0.0rc8"
ARTIFACT = "canonical/blake3_" + "0" * 8

MANIFEST: dict[str, object] = {
    "run_id": "run-1",
    "manifest_digest": "sha256:" + "1" * 64,
    "build_identity": ENGINE_VERSION,
    "configuration_digest": "sha256:" + "2" * 64,
    "seed": 1,
}


def receipt() -> RunReceipt:
    """
    Return the receipt a run of the fixture candidate emits.
    """
    return RunReceipt(
        run_id="run-1",
        manifest_digest=MANIFEST["manifest_digest"],
        build_identity=MANIFEST["build_identity"],
        configuration_digest=MANIFEST["configuration_digest"],
        seed=1,
        artifact_reference=ARTIFACT,
        artifact_digest="blake3:" + "3" * 8,
    )


def key(candidate_id: str, *, attempt: str = "") -> IdempotencyKey:
    """
    Return one candidate's job key.
    """
    return IdempotencyKey.of(candidate_id, snapshot="snapshot-1", seed=1, attempt=attempt)


def campaign(directory: Path) -> CampaignInventory:
    """
    Return an inventory over the fixture campaign, opened under the declared policy digest.
    """
    return CampaignInventory(
        directory / "campaign-1",
        writer="orchestrator",
        campaign_id="campaign-1",
        family_id=research_family_id(snapshot="snapshot-1", selection_process="rank_first"),
        policy_digest=POLICY,
    )


def search(tmp_path: Path) -> tuple[CampaignInventory, GateReport]:
    """
    Return the fixture search and the report its gate service returned.

    The campaign is deliberately mixed: one candidate ran and verified, one crashed and was left
    incomplete, one was refused by the validator, and one mutates the first with the parent recorded.
    """
    inventory = campaign(tmp_path)
    inventory.record_proposal(CANDIDATE, generation=0, agent_invocation="invocation-1")
    inventory.record_compilation(
        CANDIDATE,
        specification_digest=SPECIFICATION,
        new_configuration=True,
        generation=0,
    )
    inventory.record_proposal(DESCENDANT, generation=1)
    inventory.record_compilation(
        DESCENDANT,
        specification_digest=MUTATED,
        new_configuration=True,
        parent_id=CANDIDATE,
        generation=1,
    )
    inventory.record_proposal(REFUSED, generation=1)
    inventory.record_refusal(REFUSED, code="VALUE_NOT_DECLARED", path="entry.threshold")
    inventory.record_search_policy(search_policy_digest=SEARCH_POLICY, generation=1)

    verified = inventory.register_attempt(
        key(CANDIDATE),
        candidate_id=CANDIDATE,
        outcome=AttemptOutcome.COMPLETED,
        receipt=receipt(),
        manifest=MANIFEST,
        artifact_digest=receipt().artifact_digest,
    )
    assert verified.counted is True

    crashed = inventory.register_attempt(
        key(DESCENDANT, attempt="crash"),
        candidate_id=DESCENDANT,
        outcome=AttemptOutcome.CRASHED,
        receipt=receipt(),
        manifest=MANIFEST,
    )
    assert crashed.counted is False

    service = GateService(GatePolicy(), inventory=inventory)
    report = service.evaluate(
        GateEvidence(candidate_id=CANDIDATE, specification_digest=SPECIFICATION),
    )

    return inventory, report


def test_fx_m6_4_a_completed_search_is_reconstructable_from_the_ledger(tmp_path: Path) -> None:
    """
    `FX-M6-4`: criterion 16, over a search whose every candidate has an answer.

    Each question is asked separately, because they are separate facts and a summary that answered
    them together would hide the case this fixture is built around: the refused candidate never
    reaches the lineage, and a reader listing only compiled candidates would lose exactly the
    candidates that were turned away.
    """
    inventory, report = search(tmp_path)
    audit = audit_campaign(inventory, policy_digest=POLICY, engine_version=ENGINE_VERSION)

    # Which candidates existed: the compiled ones and the refused one, each named once.
    assert [candidate.candidate_id for candidate in audit.candidates] == [
        CANDIDATE,
        DESCENDANT,
        REFUSED,
    ]
    assert audit.candidates[1].parent_id == CANDIDATE
    assert audit.candidates[1].generation == 1

    # Which ran, and how each closed.
    assert audit.candidates[0].outcomes == ("completed",)
    assert audit.candidates[0].admitted == 1
    assert audit.candidates[1].outcomes == ("crashed",)
    assert audit.candidates[1].admitted == 0
    assert audit.candidates[2].attempts == ()
    assert audit.candidates[2].outcome == "refused"
    assert audit.candidates[2].refusals == ("VALUE_NOT_DECLARED",)

    # Which were judged, and why each was rejected, quoted from the report the service returned.
    assert [candidate.candidate_id for candidate in audit.judged] == [CANDIDATE]
    assert audit.candidates[0].reasons == tuple(verdict.reason for verdict in report.verdicts)
    assert audit.candidates[0].outcome == report.verdicts[-1].verdict.value
    assert [verdict.gate for verdict in inventory.gate_verdicts()] == [
        verdict.gate.value for verdict in report.verdicts
    ]

    # Under which policy, and under which search policy the generation ran.
    assert audit.policy_matches is True
    assert audit.recorded_policy_digest == POLICY
    assert audit.search_policy_digests == (SEARCH_POLICY,)

    # The four counts, which are separate numbers under the versioned mapping.
    assert audit.counts.proposal_attempts == 3
    assert audit.counts.unique_configurations == 2
    assert audit.counts.executed_experiments == 2
    assert audit.counts.statistical_trials == 1

    # And the engine identity is reported as the receipts carry it, not assumed.
    assert audit.engine_version == ENGINE_VERSION
    assert audit.build_identities == (ENGINE_VERSION,)


def test_fx_m6_4_the_chain_an_audit_reads_verifies_and_the_audit_is_deterministic(
    tmp_path: Path,
) -> None:
    """
    `FX-M6-4`: the two things that make the reconstruction usable as evidence.

    An audit over a chain that does not verify is a reconstruction of something that did not happen,
    so the chain is verified against its own head as part of the fixture. And the reconstruction is
    deterministic: the same chain and the same declared inputs produce the same digest, so a reader
    who re-runs it can compare rather than take the entry's word for it.
    """
    inventory, _ = search(tmp_path)
    audit = audit_campaign(inventory, policy_digest=POLICY, engine_version=ENGINE_VERSION)

    chain = inventory.verify_chain(checkpoint=audit.head)

    assert chain.ok is True, chain.findings
    assert chain.events == audit.events == len(inventory.events())
    assert audit.head == inventory.checkpoint()

    again = audit_campaign(inventory, policy_digest=POLICY, engine_version=ENGINE_VERSION)

    assert again.digest() == audit.digest()
    assert again.to_dict() == audit.to_dict()

    # A different declared policy is a different statement, and an undeclared one is not a match.
    assert audit_campaign(inventory, policy_digest=CANDIDATE).policy_matches is False
    assert audit_campaign(inventory, policy_digest=POLICY).policy_matches is True
    assert audit_campaign(inventory).policy_matches is None
    assert audit_campaign(inventory).digest() != again.digest()

    # The reconstruction is plain data, so a reader can retain and compare it.
    assert json.loads(json.dumps(audit.to_dict())) == audit.to_dict()
