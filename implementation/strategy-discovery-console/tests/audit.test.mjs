/**
 * Tests for the audit layer: the candidate set, the outcome rule, the policy comparison, agreement
 * with a gate report, and the limits of a reading that adds nothing of its own.
 *
 * What breaks in the app if these fail: the reconstruction table would show the wrong order, a
 * candidate that never compiled would vanish, an outcome word would disagree with the rule the page
 * prints, and the reading would appear to verify or join things it does not.
 */

import { describe, expect, test } from 'bun:test';

import { append } from '../src/engine/ledger.js';
import { REFUSED_ASSERTIONS, derivedView, outcome } from '../src/engine/audit.js';
import { OUTCOME_RULE } from '../src/data/ledger.js';
import { valueOf } from '../src/data/source.js';

/** A chain built through the write path. */
function chain(script) {
  let entries = [];
  for (const [kind, payload] of script) {
    const result = append(entries, kind, payload);
    if (!result.ok) throw new Error(`${kind} was refused: ${result.code}`);
    entries = result.entries;
  }
  return entries;
}

const COMPILED = (candidateId) => [
  'compilation_recorded',
  {
    candidateId,
    specification_digest: `sha256:spec-${candidateId}`,
    new_configuration: true,
  },
];

const RECEIPT = (candidateId) => ({
  build_identity: 'nautilus_trader 1.226.0',
  receipt_digest: `sha256:receipt-${candidateId}`,
});

/**
 * The manifest the engine produces for a run and the registration retains beside the receipt, which
 * `_REQUIRED_FIELDS` now demands of every `registration_verified` event.
 */
const MANIFEST = (candidateId) => ({
  run_id: `run-${candidateId}`,
  build_identity: 'nautilus_trader 1.226.0',
  configuration_digest: `sha256:config-${candidateId}`,
  seed: 1,
  manifest_digest: `sha256:manifest-${candidateId}`,
});

/** The campaign the tutorial works through: four compiled candidates, a refusal, and one verdict. */
const CAMPAIGN = [
  ['search_policy_recorded', { search_policy_digest: 'sha256:sp1' }],
  ['proposal_recorded', { candidateId: 'alpha', generation: 0 }],
  COMPILED('alpha'),
  ['proposal_recorded', { candidateId: 'beta', generation: 1, parentId: 'alpha' }],
  COMPILED('beta'),
  ['proposal_recorded', { candidateId: 'epsilon', generation: 0 }],
  COMPILED('epsilon'),
  ['proposal_recorded', { candidateId: 'delta', generation: 0 }],
  COMPILED('delta'),
  ['specification_refused', { candidateId: 'gamma', code: 'VALUE_NOT_DECLARED' }],
  ['job_submitted', { candidateId: 'alpha', key: 'k1' }],
  ['execution_attempt_closed', { candidateId: 'alpha', key: 'k1', outcome: 'completed' }],
  [
    'registration_verified',
    {
      candidateId: 'alpha',
      key: 'k1',
      receipt: RECEIPT('alpha'),
      receipt_digest: 'sha256:receipt-alpha',
      manifest: MANIFEST('alpha'),
    },
  ],
  ['trial_admitted', { candidateId: 'alpha', key: 'k1', receipt_digest: 'sha256:receipt-alpha' }],
  ['job_submitted', { candidateId: 'beta', key: 'k2' }],
  ['execution_attempt_closed', { candidateId: 'beta', key: 'k2', outcome: 'crashed' }],
  ['registration_incomplete', { candidateId: 'beta', key: 'k2', reason: 'no usable return series' }],
  ['job_submitted', { candidateId: 'epsilon', key: 'k3' }],
  ['execution_attempt_closed', { candidateId: 'epsilon', key: 'k3', outcome: 'completed' }],
  [
    'registration_verified',
    {
      candidateId: 'epsilon',
      key: 'k3',
      receipt: RECEIPT('epsilon'),
      receipt_digest: 'sha256:receipt-epsilon',
      manifest: MANIFEST('epsilon'),
    },
  ],
  [
    'trial_admitted',
    { candidateId: 'epsilon', key: 'k3', receipt_digest: 'sha256:receipt-epsilon' },
  ],
  [
    'gate_verdict',
    {
      candidateId: 'alpha',
      gate: 'G3',
      status: 'INCONCLUSIVE',
      reason: 'not distinguishable from the best of a search this size',
      evidence_digest: 'sha256:evidence-1',
    },
  ],
];

const DECLARED = {
  campaignId: 'campaign-1',
  familyId: 'family-1',
  policyDigest: 'sha256:policy-1',
  recordedPolicyDigest: 'sha256:policy-1',
  engineVersion: '1.226.0',
};

const GATE_REPORT = [
  {
    candidateId: 'alpha',
    gate: 'G3',
    status: 'INCONCLUSIVE',
    reason: 'not distinguishable from the best of a search this size',
  },
];

const campaign = () => chain(CAMPAIGN);
const read = (declared = {}) => derivedView(campaign(), { ...DECLARED, ...declared });
const rowOf = (view, candidateId) => view.candidates.find((row) => row.candidateId === candidateId);

describe('the candidate set', () => {
  test('is the lineage first and then whatever a later record names, in the tutorial\u2019s order', () => {
    const view = read();

    expect(view.candidates.map((row) => row.candidateId)).toEqual([
      'alpha',
      'beta',
      'epsilon',
      'delta',
      'gamma',
    ]);
    expect(rowOf(view, 'beta').generation).toBe(1);
    expect(rowOf(view, 'beta').parentId).toBe('alpha');
    expect(rowOf(view, 'delta').generation).toBe(0);
    expect(rowOf(view, 'delta').parentId).toBe('');
  });

  test('names a candidate the compiled lineage cannot hold, from the refusal that names it', () => {
    const view = read();

    expect(view.outsideLineage).toEqual(['gamma']);
    expect(rowOf(view, 'gamma')).toMatchObject({
      refusals: ['VALUE_NOT_DECLARED'],
      attempts: [],
      admitted: 0,
      verdicts: [],
      outcome: 'refused',
    });
    expect(rowOf(view, 'delta').outcome).toBe('proposed');
  });

  test('adds no record and cannot name a candidate the chain never mentions', () => {
    const entries = campaign();
    const before = JSON.parse(JSON.stringify(entries));
    const view = derivedView(entries, DECLARED);

    expect(JSON.parse(JSON.stringify(entries))).toEqual(before);
    expect(rowOf(view, 'zeta')).toBe(undefined);
  });
});

describe('the outcome rule', () => {
  test('gives a judged candidate the last verdict\u2019s status, over any number of attempts', () => {
    const view = read();

    expect(rowOf(view, 'alpha').admitted).toBe(1);
    expect(rowOf(view, 'alpha').outcome).toBe('INCONCLUSIVE');
    expect(view.judged).toEqual(['alpha']);
  });

  test('reads a closed attempt that was admitted as "admitted" and one that was not as "ran"', () => {
    const view = read();

    expect(rowOf(view, 'epsilon')).toMatchObject({
      attempts: ['k3'],
      outcomes: ['completed'],
      admitted: 1,
      outcome: 'admitted',
    });
    expect(rowOf(view, 'beta')).toMatchObject({
      attempts: ['k2'],
      outcomes: ['crashed'],
      admitted: 0,
      outcome: 'ran',
    });
  });

  test('orders a candidate\u2019s attempts and outcomes by idempotency key, not by chain order', () => {
    const scattered = chain([
      ['proposal_recorded', { candidateId: 'x', generation: 0 }],
      COMPILED('x'),
      ['job_submitted', { candidateId: 'x', key: 'k2' }],
      ['job_submitted', { candidateId: 'x', key: 'k1' }],
      ['execution_attempt_closed', { candidateId: 'x', key: 'k2', outcome: 'completed' }],
      ['execution_attempt_closed', { candidateId: 'x', key: 'k1', outcome: 'crashed' }],
      ['trial_admitted', { candidateId: 'x', key: 'k2', receipt_digest: 'sha256:receipt-x' }],
    ]);
    const row = derivedView(scattered).candidates[0];

    expect(row.attempts).toEqual(['k1', 'k2']);
    expect(row.outcomes).toEqual(['crashed', 'completed']);
    expect(row.admitted).toBe(1);
    expect(row.outcome).toBe('admitted');
  });

  test('derives the words from the rows, in audit order, using the transcribed vocabulary', () => {
    const view = read();
    const words = outcome(view);

    expect(words).toEqual(
      view.candidates.map((row) => ({ candidateId: row.candidateId, outcome: row.outcome })),
    );
    expect(words.map((item) => item.outcome)).toEqual([
      'INCONCLUSIVE',
      valueOf(OUTCOME_RULE.words.ran),
      valueOf(OUTCOME_RULE.words.admitted),
      valueOf(OUTCOME_RULE.words.proposed),
      valueOf(OUTCOME_RULE.words.refused),
    ]);
  });
});

describe('the declared inputs', () => {
  test('compare the declared policy digest, and report nothing declared as no comparison at all', () => {
    expect(read().policyMatches).toBe(true);
    expect(read({ policyDigest: 'sha256:other' }).policyMatches).toBe(false);
    expect(read({ policyDigest: '' }).policyMatches).toBe(null);
    // A different recorded digest the inventory was opened with is a mismatch, not a missing input.
    expect(read({ recordedPolicyDigest: 'sha256:other' }).policyMatches).toBe(false);
  });

  test('give the same plain data and the same code for the same chain and the same inputs', () => {
    const first = read();
    const second = read();

    expect(second.digest).toBe(first.digest);
    expect(second).toEqual(first);
    expect(JSON.parse(JSON.stringify(first))).toEqual(first);
  });

  test('give a different code to a different declared policy digest over an identical chain', () => {
    const under = read();
    const other = read({ policyDigest: 'sha256:other' });

    expect(other.digest).not.toBe(under.digest);
    expect(other.candidates).toEqual(under.candidates);
    expect(other.head).toBe(under.head);
  });

  test('report the build identities and the engine version side by side, without joining them', () => {
    const view = read({ engineVersion: '9.9.9' });

    expect(view.engineVersion).toBe('9.9.9');
    expect(view.buildIdentities).toEqual(['nautilus_trader 1.226.0']);
    expect(view.searchPolicyDigests).toEqual(['sha256:sp1']);
  });
});

describe('agreement with a gate report', () => {
  test('agrees word for word when the report is the one the reasons came from', () => {
    const view = read({ gateReport: GATE_REPORT });

    expect(view.gateAgreement).toEqual({ ok: true, disagreements: [] });
  });

  test('names the disagreement when a report reason differs', () => {
    const view = read({
      gateReport: [{ ...GATE_REPORT[0], reason: 'the report paraphrased the reason' }],
    });

    expect(view.gateAgreement.ok).toBe(false);
    expect(view.gateAgreement.disagreements).toEqual([
      {
        index: 0,
        candidateId: 'alpha',
        field: 'reason',
        audit: 'not distinguishable from the best of a search this size',
        report: 'the report paraphrased the reason',
      },
    ]);
  });

  test('does not check a report it was not given', () => {
    expect(read().gateAgreement).toBe(null);
  });
});

describe('what the reading does not do', () => {
  test('reads rather than verifies: a tampered field changes the reading, the head stays as recorded', () => {
    const entries = campaign();
    const before = read().candidates[0].generation;
    const tampered = entries.map((item, index) =>
      index === 1 ? { ...item, payload: { ...item.payload, generation: 7 } } : item,
    );
    const view = derivedView(tampered, DECLARED);

    expect(view.candidates[0].generation).toBe(7);
    expect(before).toBe(0);
    expect(view.head).toBe(tampered[tampered.length - 1].digest);
  });

  test('reads an empty chain as no candidates and no head', () => {
    const view = derivedView([]);

    expect(view.candidates).toEqual([]);
    expect(view.judged).toEqual([]);
    expect(view.events).toBe(0);
    expect(view.head).toBe('');
    expect(view.digest).toMatch(/^sha256:[0-9a-f]{64}$/);
  });

  test('carries the assertions it refuses to make', () => {
    const ids = REFUSED_ASSERTIONS.map((item) => item.id);

    expect(ids).toEqual([
      'names-only-what-was-retained',
      'does-not-verify-the-chain',
      'does-not-join-the-build-identities',
      'nothing-declared-is-not-a-failure',
      'deterministic-only-over-declared-inputs',
      'explains-nothing',
    ]);
    expect(REFUSED_ASSERTIONS[1].assertion).toContain('does not verify the chain');
    expect(REFUSED_ASSERTIONS[2].assertion).toContain('build identities');
  });
});
