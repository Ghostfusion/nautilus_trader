/**
 * The audit: the whole search read back from the chain alone.
 *
 * Models `audit_campaign`, `CandidateJudgement` and `_judgement_outcome` in
 * `python/nautilus_trader/discovery/ledger.py`. It folds the events itself rather than trusting a
 * counter the ledger keeps: the candidate set is the compiled lineage first and then whatever a
 * refusal, a verdict or an attempt names, in first-seen order, which is a set wider than the lineage
 * on purpose, so a candidate that was refused before it ever compiled still has a row. Each row
 * carries separate facts - its refusals, its attempts with their outcomes, how many were admitted and
 * its gate verdicts - and one derived outcome word.
 *
 * It adds no record and invents none: a candidate the chain never names cannot appear, a reason the
 * gate never wrote down cannot be quoted, and a chain that does not hold is read rather than
 * verified. The declared inputs are part of the statement the reading makes, so a different declared
 * policy digest is a different reading of an identical chain.
 *
 * Nothing here touches the DOM, the network or a clock: the same chain and the same declared inputs
 * give the same plain data and the same digest.
 */

import { digestOf } from './digest.js';
import { valueOf } from '../data/source.js';
import { CANDIDATE_SET, OUTCOME_RULE, REFUSED_ASSERTION_FACTS } from '../data/ledger.js';

/** The union's sources, in the order the tutorial states them. */
const SET_ORDER = Object.freeze(valueOf(CANDIDATE_SET.order));

/** The four words the rule can return, transcribed from the tutorial. */
const WORDS = Object.freeze({
  admitted: valueOf(OUTCOME_RULE.words.admitted),
  ran: valueOf(OUTCOME_RULE.words.ran),
  refused: valueOf(OUTCOME_RULE.words.refused),
  proposed: valueOf(OUTCOME_RULE.words.proposed),
});

/** The declared inputs, so a caller passes only what it holds and nothing is defaulted silently. */
const DECLARED_DEFAULTS = Object.freeze({
  campaignId: '',
  familyId: '',
  policyDigest: '',
  recordedPolicyDigest: '',
  engineVersion: '',
  gateReport: null,
});

const byKey = (left, right) => (left.key < right.key ? -1 : left.key > right.key ? 1 : 0);

/** One candidate's attempts, folded by idempotency key and ordered by that key. */
function attemptRecords(attempts, candidateId) {
  return [...attempts.values()]
    .filter((record) => record.candidateId === candidateId)
    .sort(byKey);
}

/**
 * The outcome word, evaluated top-down: a verdict first, then admitted, then ran, then refused, then
 * proposed. A verdict outranks any number of attempts and one admitted attempt is enough.
 */
function outcomeWord(row) {
  if (row.verdicts.length > 0) return row.verdicts[row.verdicts.length - 1].status;
  if (row.attempts.length > 0) return row.admitted > 0 ? WORDS.admitted : WORDS.ran;
  return row.refusals.length > 0 ? WORDS.refused : WORDS.proposed;
}

/** Fold the chain into the facts the audit reads, each kept separate rather than summarised. */
function fold(entries) {
  const proposals = new Map();
  const lineage = [];
  const refusals = new Map();
  const verdicts = new Map();
  const verdictsInOrder = [];
  const attempts = new Map();
  const searchPolicyDigests = [];
  const buildIdentities = new Set();

  const push = (map, key, value) => {
    if (!map.has(key)) map.set(key, []);
    map.get(key).push(value);
  };
  const attemptFor = (key, candidateId) => {
    if (!attempts.has(key)) {
      attempts.set(key, { key, candidateId, outcome: null, admitted: false });
    }
    return attempts.get(key);
  };

  for (const item of entries) {
    const candidateId = item.candidateId;
    switch (item.kind) {
      case 'proposal_recorded':
        proposals.set(candidateId, {
          generation: Number(item.payload.generation ?? 0),
          parentId: String(item.payload.parentId ?? ''),
        });
        break;
      case 'compilation_recorded':
        if (!lineage.includes(candidateId)) lineage.push(candidateId);
        break;
      case 'specification_refused':
        push(refusals, candidateId, String(item.payload.code ?? ''));
        break;
      case 'job_submitted':
        attemptFor(item.key, candidateId);
        break;
      case 'execution_attempt_closed':
        attemptFor(item.key, candidateId).outcome = String(item.payload.outcome ?? '');
        break;
      case 'registration_incomplete':
        // The reason stays in the record; the audit reads only that the attempt exists and closed
        // without entering the inventory, which is what its outcome and admitted count already show.
        attemptFor(item.key, candidateId);
        break;
      case 'registration_verified': {
        attemptFor(item.key, candidateId);
        const receipt = item.payload.receipt;
        const identity =
          receipt !== null && typeof receipt === 'object'
            ? String(receipt.build_identity ?? '')
            : '';
        if (identity !== '') buildIdentities.add(identity);
        break;
      }
      case 'trial_admitted':
        attemptFor(item.key, candidateId).admitted = true;
        break;
      case 'gate_verdict': {
        const row = {
          candidateId,
          gate: String(item.payload.gate ?? ''),
          status: String(item.payload.status ?? ''),
          reason: String(item.payload.reason ?? ''),
          evidenceDigest: String(item.payload.evidence_digest ?? ''),
          sequence: item.sequence,
        };
        push(verdicts, candidateId, row);
        verdictsInOrder.push(row);
        break;
      }
      case 'search_policy_recorded':
        searchPolicyDigests.push(String(item.payload.search_policy_digest ?? ''));
        break;
      default:
        // equivalence_merge, duplicate_registration and custody_event change none of the facts the
        // audit reads: a merge is a configuration decision, a duplicate changes no count, and custody
        // is the held-back data's own record.
        break;
    }
  }

  return {
    proposals,
    lineage,
    refusals,
    verdicts,
    verdictsInOrder,
    attempts,
    searchPolicyDigests,
    buildIdentities,
  };
}

/** The candidate set: the lineage first, then the first candidate each later source names. */
function candidateSet(folded) {
  const sources = {
    lineage: () => folded.lineage,
    refusals: () => [...folded.refusals.keys()],
    verdicts: () => [...folded.verdicts.keys()],
    attempts: () => [...folded.attempts.values()].map((record) => record.candidateId),
  };

  const named = new Map();
  for (const source of SET_ORDER) {
    for (const candidateId of sources[source]()) named.set(candidateId, null);
  }
  return [...named.keys()];
}

/** One candidate's row: the separate facts and the derived word. */
function rowFor(candidateId, folded) {
  const proposal = folded.proposals.get(candidateId);
  const records = attemptRecords(folded.attempts, candidateId);
  const row = {
    candidateId,
    generation: proposal ? proposal.generation : 0,
    parentId: proposal ? proposal.parentId : '',
    refusals: folded.refusals.get(candidateId) ?? [],
    attempts: records.map((record) => record.key),
    outcomes: records.filter((record) => record.outcome !== null).map((record) => record.outcome),
    admitted: records.filter((record) => record.admitted).length,
    verdicts: folded.verdicts.get(candidateId) ?? [],
    outcome: '',
  };
  row.outcome = outcomeWord(row);
  return row;
}

/**
 * Compare the verdicts the chain holds with a gate report, when the reader holds one. The audit
 * agrees with the report's reasons word for word or it names each disagreement; it never paraphrases
 * one, and it does not check a report against anything but its own reading.
 */
function agreeWithGateReport(folded, report) {
  if (!Array.isArray(report)) return null;

  const disagreements = [];
  const length = Math.max(folded.length, report.length);
  for (let index = 0; index < length; index++) {
    const mine = folded[index];
    const theirs = report[index];
    if (!mine || !theirs) {
      disagreements.push({
        index,
        candidateId: String((mine ?? theirs)?.candidateId ?? ''),
        field: 'verdict',
        audit: mine ? mine.status : '',
        report: theirs ? String(theirs.status ?? '') : '',
      });
      continue;
    }
    for (const field of ['candidateId', 'gate', 'status', 'reason']) {
      if (String(mine[field] ?? '') !== String(theirs[field] ?? '')) {
        disagreements.push({
          index,
          candidateId: mine.candidateId,
          field,
          audit: mine[field],
          report: theirs[field],
        });
      }
    }
  }

  return { ok: disagreements.length === 0, disagreements };
}

/**
 * The reading: the campaign, the chain's head, the declared and recorded policy digests, the
 * search-policy digests and build identities as recorded, and one row per candidate.
 *
 * `declared` carries what the reader holds rather than what the ledger knows: the policy digest, the
 * engine version, the campaign and family identifiers, and optionally a gate report to check against.
 * `policyMatches` is `null` when no policy digest was declared, which is no comparison at all rather
 * than a failed one. `judged`, `outsideLineage` and `gateAgreement` are derived properties rather than
 * fields of the plain data, so `digest` is a code over the reading and not over the check.
 */
export function derivedView(entries, declared = {}) {
  if (!Array.isArray(entries)) {
    throw new Error('the audit reads the chain it is given');
  }

  const inputs = { ...DECLARED_DEFAULTS, ...declared };
  const folded = fold(entries);
  const candidates = candidateSet(folded).map((candidateId) => rowFor(candidateId, folded));
  const lineage = new Set(folded.lineage);
  const plain = {
    campaignId: inputs.campaignId,
    familyId: inputs.familyId,
    events: entries.length,
    head: entries.length > 0 ? entries[entries.length - 1].digest : '',
    policyDigest: inputs.policyDigest,
    recordedPolicyDigest: inputs.recordedPolicyDigest,
    policyMatches:
      inputs.policyDigest === '' ? null : inputs.policyDigest === inputs.recordedPolicyDigest,
    searchPolicyDigests: folded.searchPolicyDigests,
    engineVersion: inputs.engineVersion,
    buildIdentities: [...folded.buildIdentities].sort(),
    candidates,
  };

  return {
    ...plain,
    judged: candidates.filter((row) => row.verdicts.length > 0).map((row) => row.candidateId),
    outsideLineage: candidates
      .filter((row) => !lineage.has(row.candidateId))
      .map((row) => row.candidateId),
    gateAgreement: agreeWithGateReport(folded.verdictsInOrder, inputs.gateReport),
    digest: digestOf(plain),
  };
}

/**
 * The outcome word for every candidate in a reading, in audit order, derived from the row's fields.
 *
 * The word is recomputed here rather than read from the row so that a reader can see the rule applied
 * to what the row holds; the reading's own digest covers the same words, because they are part of its
 * plain data.
 */
export function outcome(view) {
  if (view === null || typeof view !== 'object' || !Array.isArray(view.candidates)) {
    throw new Error('an outcome is derived from a reading that holds candidate rows');
  }
  return view.candidates.map((row) => ({
    candidateId: row.candidateId,
    outcome: outcomeWord(row),
  }));
}

/**
 * The assertions the audit refuses to make, in the tutorial's own words: it verifies nothing, it adds
 * no record, it joins no build identity to the declared version, it does not read "nothing declared"
 * as a failure, its determinism holds only over the same declared inputs, and it explains nothing
 * beyond the one-word outcome.
 */
export const REFUSED_ASSERTIONS = Object.freeze(
  REFUSED_ASSERTION_FACTS.map((item) =>
    Object.freeze({ id: item.id, assertion: valueOf(item.claim) }),
  ),
);
