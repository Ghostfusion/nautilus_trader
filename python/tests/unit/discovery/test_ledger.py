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
Tests for the campaign inventory: the append protocol, the four counts, lineage and reconciliation.

Each case is a declared campaign with one thing changed - a duplicate, a crash, a refusal, an
altered receipt - and the assertion is on the counts, the states and the refusals, because those are
the contract rather than the wording of a message.
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest

from nautilus_trader.discovery.ledger import BUNDLE_SCHEMA
from nautilus_trader.discovery.ledger import AttemptOutcome
from nautilus_trader.discovery.ledger import CampaignInventory
from nautilus_trader.discovery.ledger import DataRole
from nautilus_trader.discovery.ledger import DiscoveryLedger
from nautilus_trader.discovery.ledger import EnvelopeKind
from nautilus_trader.discovery.ledger import EnvelopeReference
from nautilus_trader.discovery.ledger import EvidencePackage
from nautilus_trader.discovery.ledger import IdempotencyKey
from nautilus_trader.discovery.ledger import RegistrationOutcome
from nautilus_trader.discovery.ledger import RegistrationState
from nautilus_trader.discovery.ledger import ReproducibilityEnvelope
from nautilus_trader.discovery.ledger import RunReceipt
from nautilus_trader.discovery.ledger import bundle_identity
from nautilus_trader.discovery.ledger import counts_from_events
from nautilus_trader.discovery.ledger import non_deterministic_digest
from nautilus_trader.discovery.ledger import recompute
from nautilus_trader.discovery.ledger import reconcile_receipt
from nautilus_trader.discovery.ledger import research_family_id
from nautilus_trader.discovery.ledger import trial_mapping
from nautilus_trader.discovery.ledger import verify_bundle
from nautilus_trader.optimization.significance import TrialDependence


CANDIDATE = "sha256:" + "a" * 64
OTHER = "sha256:" + "b" * 64
CANDIDATE_DIGEST = "sha256:" + "c" * 64
MANIFEST = {
    "run_id": "run-1",
    "manifest_digest": "sha256:" + "e" * 64,
    "build_identity": "nautilus_trader 2.0.0rc8",
    "configuration_digest": "sha256:" + "f" * 64,
    "seed": 1,
}
ARTIFACT = "canonical/blake3_" + "0" * 8


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
        artifact_digest="blake3:" + "1" * 8,
    )


def key(attempt: str = "") -> IdempotencyKey:
    """
    Return the idempotency key of the fixture candidate's job.
    """
    return IdempotencyKey.of(CANDIDATE, snapshot="snapshot-1", seed=1, attempt=attempt)


def inventory(directory: Path, *, writer: str = "orchestrator") -> CampaignInventory:
    """
    Return an inventory over a campaign of the fixture research family.
    """
    return CampaignInventory(
        directory,
        writer=writer,
        campaign_id="campaign-1",
        family_id=research_family_id(snapshot="snapshot-1", selection_process="rank_first"),
    )


def register(
    campaign: CampaignInventory,
    *,
    attempt: str = "",
    outcome: AttemptOutcome = AttemptOutcome.COMPLETED,
    artifact_digest: str | None = None,
) -> RegistrationOutcome:
    """
    Run the registration protocol for the fixture job.
    """
    return campaign.register_attempt(
        key(attempt),
        candidate_id=CANDIDATE,
        outcome=outcome,
        receipt=receipt(),
        manifest=MANIFEST,
        artifact_digest=receipt().artifact_digest if artifact_digest is None else artifact_digest,
    )


def test_the_four_counts_are_separate_and_follow_the_versioned_mapping(tmp_path: Path) -> None:
    """
    A campaign with a duplicate, a crash and a refusal reports four different numbers.

    This is criterion 7's arithmetic: the proposal attempt, the unique configuration, the executed
    experiment and the statistical trial are distinct quantities, and a refusal is the first and
    none of the others.
    """
    campaign = inventory(tmp_path / "inventory")
    campaign.record_proposal(CANDIDATE, generation=0, agent_invocation="invocation-1")
    campaign.record_proposal(OTHER, generation=0)
    campaign.record_refusal(OTHER, code="VALUE_NOT_DECLARED", path="entry.threshold")
    campaign.record_compilation(
        CANDIDATE,
        specification_digest=CANDIDATE_DIGEST,
        new_configuration=True,
        generation=0,
    )
    campaign.record_compilation(
        CANDIDATE,
        specification_digest=CANDIDATE_DIGEST,
        new_configuration=False,
        generation=0,
    )

    registered = register(campaign)
    assert registered.counted is True

    register(campaign)
    register(campaign, attempt="retry-1", outcome=AttemptOutcome.CRASHED)

    counts = campaign.counts()
    assert counts.proposal_attempts == 2
    assert counts.unique_configurations == 1
    assert counts.executed_experiments == 2
    assert counts.statistical_trials == 1
    assert counts.mapping == "discovery.counts/v1"
    assert counts == counts_from_events(campaign.events())


def test_an_interrupted_registration_replays_to_one_statistical_trial(tmp_path: Path) -> None:
    """
    A registration that fails before its artifact verifies is retryable and counted once.

    The first call leaves an incomplete record, which is counted nowhere; the replay under the same
    key admits exactly one trial, and a third call is idempotent rather than additive.
    """
    campaign = inventory(tmp_path / "inventory")
    incomplete = register(campaign, artifact_digest="blake3:tampered")
    assert incomplete.state is RegistrationState.INCOMPLETE
    assert incomplete.counted is False
    assert incomplete.retryable is True
    assert campaign.counts().statistical_trials == 0
    assert campaign.counts().executed_experiments == 1

    replayed = register(campaign)
    assert replayed.state is RegistrationState.REGISTRATION_RECONCILED
    assert replayed.counted is True

    duplicate = register(campaign)
    assert duplicate.idempotent is True
    assert duplicate.counted is False
    assert campaign.counts().statistical_trials == 1
    assert campaign.counts().executed_experiments == 1
    assert [event.kind.value for event in campaign.events()].count("duplicate_registration") == 1


def test_an_attempt_that_produced_no_return_series_is_never_admitted(tmp_path: Path) -> None:
    """
    A crash stays in the execution count and is never given an imputed return.

    This is the conservative rule of the plan's mapping: the attempt closed, so it is an execution,
    and it produced no usable series, so it is not a statistical trial.
    """
    campaign = inventory(tmp_path / "inventory")
    crashed = register(campaign, outcome=AttemptOutcome.TIMED_OUT)

    assert crashed.state is RegistrationState.INCOMPLETE
    assert "no usable return series" in crashed.reason
    assert campaign.counts().executed_experiments == 1
    assert campaign.counts().statistical_trials == 0


def test_an_incomplete_record_is_never_completed_by_inference(tmp_path: Path) -> None:
    """
    A registration marked incomplete by a caller stays incomplete and uncounted.
    """
    campaign = inventory(tmp_path / "inventory")
    campaign.submit_job(key(), candidate_id=CANDIDATE)
    campaign.close_attempt(key(), outcome=AttemptOutcome.COMPLETED)
    campaign.mark_incomplete(key(), reason="the worker stopped before its artifact was written")

    record = campaign.attempts()[key().value]
    assert record.state is RegistrationState.INCOMPLETE
    assert record.admitted is False
    assert campaign.counts().statistical_trials == 0


def test_an_illegal_registration_transition_is_refused(tmp_path: Path) -> None:
    """
    A reconciled registration may not move again, because nothing resets a verified result.
    """
    campaign = inventory(tmp_path / "inventory")
    register(campaign)

    with pytest.raises(ValueError, match="may not move to"):
        campaign.mark_incomplete(key(), reason="late")


def test_one_authoritative_writer_owns_every_append(tmp_path: Path) -> None:
    """
    A second writer on one inventory is refused rather than serialised.
    """
    directory = tmp_path / "inventory"
    inventory(directory)

    with pytest.raises(RuntimeError, match="one authoritative writer"):
        inventory(directory, writer="worker-2")


def test_two_concurrent_registrations_of_one_experiment_admit_one_trial(tmp_path: Path) -> None:
    """
    Two workers registering the same experiment contend for one key, so the second is idempotent.

    The protocol has one writer, and the concurrency case is about two registrations reaching it,
    which is what the ledger's own lock serialises.
    """
    import threading

    campaign = inventory(tmp_path / "inventory")
    outcomes = []
    barrier = threading.Barrier(2)

    def worker() -> None:
        barrier.wait()
        outcomes.append(register(campaign))

    threads = [threading.Thread(target=worker) for _ in range(2)]
    for thread in threads:
        thread.start()
    for thread in threads:
        thread.join()

    assert campaign.counts().statistical_trials == 1
    assert sum(1 for outcome in outcomes if outcome.counted) == 1
    assert sum(1 for outcome in outcomes if outcome.idempotent) == 1
    assert campaign.verify_chain(checkpoint=campaign.checkpoint()).ok


def test_a_duplicate_execution_is_a_distinct_execution_event(tmp_path: Path) -> None:
    """
    A deliberate rerun of one configuration is another execution and not another configuration.
    """
    campaign = inventory(tmp_path / "inventory")
    register(campaign)
    register(campaign, attempt="rerun", artifact_digest=receipt().artifact_digest)

    counts = campaign.counts()
    assert counts.executed_experiments == 2
    assert counts.unique_configurations == 0
    assert counts.statistical_trials == 2


def test_the_chain_verifies_and_a_reordered_record_is_detectable(tmp_path: Path) -> None:
    """
    The chain's head matches a checkpoint, and swapping two records is found against it.
    """
    directory = tmp_path / "inventory"
    campaign = inventory(directory)
    register(campaign)
    register(campaign, attempt="rerun")
    checkpoint = campaign.checkpoint()

    assert campaign.verify_chain(checkpoint=checkpoint).ok is True

    path = directory / "inventory.jsonl"
    records = path.read_text(encoding="utf-8").splitlines()
    records[1], records[2] = records[2], records[1]
    path.write_text("\n".join(records) + "\n", encoding="utf-8")

    verification = campaign.verify_chain(checkpoint=checkpoint)
    assert verification.ok is False
    assert any("chains to" in finding for finding in verification.findings)


def test_a_receipt_reconciles_and_an_altered_field_fails_closed() -> None:
    """
    Reconciliation compares every joined field, and an absent or differing field is a mismatch.
    """
    emitted = receipt()
    assert reconcile_receipt(emitted, manifest=MANIFEST).ok is True

    for field in ("run_id", "manifest_digest", "build_identity", "configuration_digest", "seed"):
        altered = dict(MANIFEST, **{field: "changed" if field != "seed" else 2})
        reconciliation = reconcile_receipt(emitted, manifest=altered)
        assert reconciliation.ok is False
        assert field in reconciliation.mismatched

    missing = {name: value for name, value in MANIFEST.items() if name != "seed"}
    assert reconcile_receipt(emitted, manifest=missing).mismatched == ("seed",)


def test_the_ledger_records_lineage_over_a_campaign(tmp_path: Path) -> None:
    """
    Parentage, campaign, family and the recorded invocation are all part of the lineage.
    """
    campaign = inventory(tmp_path / "inventory")
    campaign.record_proposal(CANDIDATE, generation=2, agent_invocation="invocation-7")
    campaign.record_compilation(
        CANDIDATE,
        specification_digest=CANDIDATE_DIGEST,
        new_configuration=True,
        parent_id=OTHER,
        generation=2,
    )

    record = campaign.lineage()[0]
    assert record.candidate_id == CANDIDATE
    assert record.parent_id == OTHER
    assert record.campaign_id == "campaign-1"
    assert record.family_id == campaign.family_id
    assert record.generation == 2
    assert record.agent_invocation == "invocation-7"


def test_a_family_identity_names_one_search_over_one_snapshot() -> None:
    """
    Two campaigns over one snapshot and one selection process share a family, and a third does not.
    """
    first = research_family_id(snapshot="snapshot-1", selection_process="rank_first")
    second = research_family_id(snapshot="snapshot-1", selection_process="rank_first")
    other = research_family_id(snapshot="snapshot-2", selection_process="rank_first")

    assert first == second
    assert first != other


def test_a_campaign_with_dependent_trials_must_declare_its_effective_count(
    tmp_path: Path,
) -> None:
    """
    The mapping refuses a dependence declaration the correction cannot be computed from.
    """
    campaign = inventory(tmp_path / "inventory")
    register(campaign)

    with pytest.raises(ValueError, match="must declare its effective trial count"):
        trial_mapping(campaign, dependence=TrialDependence.DEPENDENT)
    with pytest.raises(ValueError, match="must not declare an effective count"):
        trial_mapping(campaign, dependence=TrialDependence.INDEPENDENT, effective_trials=3)

    mapping = trial_mapping(campaign, dependence=TrialDependence.DEPENDENT, effective_trials=1)
    assert mapping.statistical_trials == 1
    assert mapping.executed_experiments == 1
    assert mapping.data_roles[key().value] is DataRole.DEVELOPMENT


def test_the_mapping_carries_the_attempts_that_produced_no_return_series(tmp_path: Path) -> None:
    """
    A crashed attempt is named in the mapping, which is how a gate reports its uncertainty.
    """
    campaign = inventory(tmp_path / "inventory")
    register(campaign, outcome=AttemptOutcome.ERRORED)

    mapping = trial_mapping(campaign)
    assert mapping.unusable_attempts == 1
    assert mapping.statistical_trials == 0
    assert key().value in mapping.unusable


def envelope(
    *, omit: EnvelopeKind | None = None, unresolved: bool = False
) -> ReproducibilityEnvelope:
    """
    Return a complete envelope, optionally missing a kind or holding an unresolved reference.
    """
    references = []
    for kind in EnvelopeKind:
        if kind is EnvelopeKind.EXTERNAL_ARTIFACT or kind is omit:
            continue
        if unresolved and kind is EnvelopeKind.SEED:
            references.append(
                EnvelopeReference(name=kind.value, kind=kind, digest="sha256:" + "9" * 64),
            )
            continue
        references.append(EnvelopeReference.of(kind.value, kind, {"declared": kind.value}))

    return ReproducibilityEnvelope(references=tuple(references))


def test_an_envelope_declares_every_required_kind_and_resolves_it() -> None:
    """
    Every kind the design lists is required, and a reference resolves only with its content.
    """
    complete = envelope()
    assert complete.unresolved() == ()
    assert complete.reference(EnvelopeKind.SEED).resolves is True

    with pytest.raises(ValueError, match="does not declare"):
        envelope(omit=EnvelopeKind.SEED)

    incomplete = envelope(unresolved=True)
    assert incomplete.unresolved() == ("seed",)
    assert incomplete.reference(EnvelopeKind.SEED).recomputed_digest() == ""


def test_a_package_is_keyed_by_its_behavioural_identity_and_never_rewritten(
    tmp_path: Path,
) -> None:
    """
    The ledger refuses a second entry for one candidate, which is what append-only means here.
    """
    ledger = DiscoveryLedger(tmp_path / "ledger")
    package = EvidencePackage(
        candidate_id=CANDIDATE,
        specification_digest=CANDIDATE_DIGEST,
        behavioural_digest=CANDIDATE,
        policy_digest="sha256:" + "1" * 64,
        registry_digest="sha256:" + "2" * 64,
        experiment_digest="sha256:" + "3" * 64,
        canonical_digest="blake3:" + "4" * 8,
        canonical_bytes=b"{}",
        metric_values={"Sharpe Ratio": 1.5},
        engine_version="nautilus_trader 2.0.0rc8",
        snapshot="snapshot-1",
        seed=1,
        status="completed",
    )
    ledger.append(package)

    assert ledger.candidate_ids() == (CANDIDATE,)
    assert len(ledger) == 1
    with pytest.raises(FileExistsError):
        ledger.append(package)


def test_the_recomputation_agrees_across_the_two_stores(tmp_path: Path) -> None:
    """
    The counts, the identities and the envelope recompute from the ledger and the inventory alone.

    The two stores are checked against each other rather than one against itself: every entry on the
    research side must be explained by an admitted trial on the campaign's, and each entry's
    canonical digest must equal the digest of the receipt that admitted it.
    """
    campaign = inventory(tmp_path / "inventory")
    register(campaign)

    ledger = DiscoveryLedger(tmp_path / "ledger")
    package = EvidencePackage(
        candidate_id=CANDIDATE,
        specification_digest=CANDIDATE_DIGEST,
        behavioural_digest=CANDIDATE,
        policy_digest="sha256:" + "1" * 64,
        registry_digest="sha256:" + "2" * 64,
        experiment_digest="sha256:" + "3" * 64,
        canonical_digest=receipt().artifact_digest,
        canonical_bytes=b"{}",
        metric_values={"Sharpe Ratio": 1.5},
        engine_version="nautilus_trader 2.0.0rc8",
        snapshot="snapshot-1",
        seed=1,
        status="completed",
    )
    ledger.append(package)

    recomputed = recompute(ledger, inventory=campaign, envelope=envelope())
    assert recomputed.disagreements == ()
    assert recomputed.unresolved == ()
    assert recomputed.non_deterministic == non_deterministic_digest()
    assert recomputed.counts.statistical_trials == 1

    # Three inputs cannot be recomputed here, and each names the dependency that blocks it: the
    # engine's canonical digest, the manifest of decision M1.3, and the retained artifact, which
    # needs the store that holds it.
    assert len(recomputed.unavailable) == 3
    assert any("M1.3" in entry for entry in recomputed.unavailable)
    assert any("retained canonical artifact" in entry for entry in recomputed.unavailable)
    with pytest.raises(ValueError, match="not complete"):
        recomputed.require()


def test_the_recomputation_finds_a_research_entry_no_trial_explains(tmp_path: Path) -> None:
    """
    A disagreement between the two stores is reported rather than resolved in one store's favour.
    """
    campaign = inventory(tmp_path / "inventory")
    ledger = DiscoveryLedger(tmp_path / "ledger")
    package = EvidencePackage(
        candidate_id=CANDIDATE,
        specification_digest=CANDIDATE_DIGEST,
        behavioural_digest=CANDIDATE,
        policy_digest="sha256:" + "1" * 64,
        registry_digest="sha256:" + "2" * 64,
        experiment_digest="sha256:" + "3" * 64,
        canonical_digest="blake3:" + "5" * 8,
        canonical_bytes=b"{}",
        metric_values={"Sharpe Ratio": 1.5},
        engine_version="nautilus_trader 2.0.0rc8",
        snapshot="snapshot-1",
        seed=1,
        status="completed",
    )
    ledger.append(package)

    recomputed = recompute(ledger, inventory=campaign)
    assert "research_entries" in recomputed.disagreements
    assert f"canonical_digest:{CANDIDATE}" in recomputed.disagreements


def _bundle_entry(*, digest: str | None = None) -> dict[str, object]:
    """
    Return a bundle entry that holds, or one whose identity digest is altered.
    """
    components = {"fixture": "FX-M4-8", "test": "tests/x.py", "engine": "build", "data": "none"}
    recorded = receipt()
    return {
        "criterion": 18,
        "requirement": "The ledger protocol holds.",
        "requirement_source": "design Section 15 criterion 18",
        "fixture": "FX-M4-8",
        "milestone": "M4",
        "status": "passed",
        "identity": {
            "digest": digest if digest is not None else bundle_identity(components),
            "components": components,
        },
        "test_or_fixture": "tests/x.py",
        "procedure": "pytest tests/x.py",
        "expected": "the protocol holds",
        "observed": "it holds",
        "receipts": {
            "run": {**recorded.to_dict(), "digest": recorded.digest()},
            "artifacts": [
                {"reference": ARTIFACT, "digest": recorded.artifact_digest},
            ],
            "engine_build": recorded.build_identity,
            "policy_digest": "sha256:" + "7" * 64,
        },
    }


def bundle(**overrides: object) -> dict[str, object]:
    """
    Return a bundle document holding one entry, with the given overrides applied.
    """
    entry = _bundle_entry()
    entry.update(overrides)

    return {"schema": BUNDLE_SCHEMA, "entries": [entry]}


def findings(document: dict[str, object], resolve: dict[str, str] | None = None) -> list[str]:
    """
    Return the described findings of a bundle verification.
    """
    return [finding.describe() for finding in verify_bundle(document, resolve=resolve).findings]


def test_a_bundle_verifies_by_recomputation_and_receipt_resolution() -> None:
    """
    A holding entry's identity digest recomputes and its artifact resolves against the ledger.
    """
    verification = verify_bundle(
        bundle(),
        resolve={ARTIFACT: receipt().artifact_digest},
    )
    assert verification.ok is True
    assert verification.resolved == (ARTIFACT,)


def test_a_bundle_entry_that_claims_an_identity_it_does_not_hold_is_found() -> None:
    """
    An altered identity digest and an unresolved artifact are both findings.
    """
    identity: dict[str, object] = {
        "digest": "sha256:" + "0" * 64,
        "components": {"fixture": "FX-M4-8"},
    }
    tampered = findings(bundle(identity=identity))
    assert any("identity.digest" in finding for finding in tampered)

    unresolved = verify_bundle(bundle(), resolve={})
    assert unresolved.ok is False
    assert any("retained content" in finding.describe() for finding in unresolved.findings)


def test_a_bundle_refuses_a_status_outside_the_closed_vocabulary() -> None:
    """
    A status the bundle vocabulary does not hold is a finding, and a block must name a dependency.
    """
    assert any("is not a status" in finding for finding in findings(bundle(status="probably")))

    blocked = bundle(status="blocked")
    assert any("dependency" in finding for finding in findings(blocked))

    named = bundle(status="blocked")
    entry = named["entries"][0]  # type: ignore[index]
    entry["dependency"] = "the run receipt of SD16"
    assert findings(named) == []


def test_a_bundle_document_with_the_wrong_schema_is_not_evidence() -> None:
    """
    The document's schema is checked before any entry is read.
    """
    document = bundle()

    assert any("expected" in finding for finding in findings(document | {"schema": "wrong"}))
    assert any("no entry sequence" in finding for finding in findings({"schema": BUNDLE_SCHEMA}))


def test_the_inventory_records_every_declared_field_of_a_job(tmp_path: Path) -> None:
    """
    A submitted job names its candidate, its data role and the non-determinism it ran under.
    """
    campaign = inventory(tmp_path / "inventory")
    campaign.submit_job(key(), candidate_id=CANDIDATE, data_role=DataRole.LOCKED_VALIDATION)
    campaign.close_attempt(key(), outcome=AttemptOutcome.COMPLETED)

    record = campaign.attempts()[key().value]
    assert record.data_role is DataRole.LOCKED_VALIDATION
    assert record.outcome is AttemptOutcome.COMPLETED
    assert record.state is RegistrationState.COMPLETED

    first = (tmp_path / "inventory" / "inventory.jsonl").read_text(encoding="utf-8").splitlines()[0]
    recorded = json.loads(first)
    assert recorded["payload"]["non_deterministic"] == non_deterministic_digest()
    assert recorded["payload"]["data_role"] == "locked_validation"


def test_an_equivalence_merge_records_both_identities(tmp_path: Path) -> None:
    """
    A merge is recorded rather than performed silently, so a distinct configuration cannot vanish.
    """
    campaign = inventory(tmp_path / "inventory")
    event = campaign.record_equivalence_merge(
        CANDIDATE, merged_into=OTHER, rule="ordered_conditions"
    )

    assert event.payload["merged_into"] == OTHER
    assert event.payload["rule"] == "ordered_conditions"
    assert campaign.counts().unique_configurations == 0
