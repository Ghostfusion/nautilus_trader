/**
 * Tests for the record layer: the closed vocabulary, the append-only chain, the digest of a record,
 * and the verification that reports a finding by name.
 *
 * What breaks in the app if these fail: the chain table would show digests that do not identify their
 * records, an append would be able to rewrite an entry the reader already wrote, and the tamper
 * demonstration would either miss a damaged entry or claim a damage that is not there.
 */

import { describe, expect, test } from 'bun:test';

import { GENESIS_CHAIN, chainValue, digestOf } from '../src/engine/digest.js';
import {
  ENTRY_KINDS,
  LIMITS,
  REFUSAL_CODES,
  append,
  entry,
  requiredFields,
  verifyChain,
} from '../src/engine/ledger.js';
import {
  CHAIN_RULES,
  ENTRY_KIND_FACTS,
  REFUSAL_MESSAGES,
  REQUIRED_FIELDS,
} from '../src/data/ledger.js';
import { valueOf } from '../src/data/source.js';

/** A chain built through the write path, so no test writes a record the ledger would refuse. */
function chain(script) {
  let entries = [];
  for (const [kind, payload] of script) {
    const result = append(entries, kind, payload);
    if (!result.ok) throw new Error(`${kind} was refused: ${result.code}`);
    entries = result.entries;
  }
  return entries;
}

const CAMPAIGN = [
  ['proposal_recorded', { candidateId: 'alpha', generation: 0 }],
  [
    'compilation_recorded',
    { candidateId: 'alpha', specification_digest: 'sha256:spec-alpha', new_configuration: true },
  ],
  ['job_submitted', { candidateId: 'alpha', key: 'k1' }],
  ['execution_attempt_closed', { candidateId: 'alpha', key: 'k1', outcome: 'completed' }],
];

/** A copy of the chain with one entry's payload replaced, its digest and chain value left as they were. */
function tamper(entries, position, patch) {
  return entries.map((item, index) =>
    index === position ? { ...item, payload: { ...item.payload, ...patch } } : item,
  );
}

describe('the vocabulary', () => {
  test('is the thirteen kinds the module declares, in its order', () => {
    expect(ENTRY_KINDS()).toEqual(ENTRY_KIND_FACTS.map(valueOf));
    expect(ENTRY_KINDS()).toHaveLength(13);
  });

  test('lists exactly the fields each kind must record', () => {
    for (const kind of ENTRY_KINDS()) {
      expect(requiredFields(kind)).toEqual(valueOf(REQUIRED_FIELDS[kind]));
    }
  });
});

describe('a record', () => {
  test('numbers the chain from zero and chains the first entry to the genesis value', () => {
    const [first] = chain([['proposal_recorded', { candidateId: 'alpha', generation: 0 }]]);

    expect(first.sequence).toBe(0);
    expect(first.previous).toBe('');
    expect(first.digest).toBe(
      digestOf({
        sequence: 0,
        kind: 'proposal_recorded',
        key: '',
        candidateId: 'alpha',
        payload: { generation: 0 },
        previous: '',
      }),
    );
    expect(first.chain).toBe(chainValue(GENESIS_CHAIN, first.digest));
  });

  test('quotes the digest of the record before it and covers every entry before it', () => {
    const entries = chain(CAMPAIGN);

    entries.slice(1).forEach((item, index) => {
      expect(item.sequence).toBe(index + 1);
      expect(item.previous).toBe(entries[index].digest);
      expect(item.chain).toBe(chainValue(entries[index].chain, item.digest));
    });
  });

  test('digests a set of values rather than the order they were written in', () => {
    const forward = entry('proposal_recorded', { candidateId: 'alpha', generation: 0 }, null);
    const reversed = entry('proposal_recorded', { generation: 0, candidateId: 'alpha' }, null);

    expect(reversed.digest).toBe(forward.digest);
  });

  test('gets a different digest at a different position, so a record cannot be moved', () => {
    const alone = chain([['proposal_recorded', { candidateId: 'alpha', generation: 0 }]])[0];
    const second = chain([
      ['search_policy_recorded', { search_policy_digest: 'sha256:sp1' }],
      ['proposal_recorded', { candidateId: 'alpha', generation: 0 }],
    ])[1];

    expect(second.digest).not.toBe(alone.digest);
  });
});

describe('the write path', () => {
  test('refuses a kind outside the closed vocabulary', () => {
    const refusal = append([], '', {});

    expect(refusal).toEqual({
      ok: false,
      code: 'KIND_NOT_IN_VOCABULARY',
      reason: valueOf(REFUSAL_MESSAGES.KIND_NOT_IN_VOCABULARY),
    });
    expect(REFUSAL_CODES.KIND_NOT_IN_VOCABULARY.message).toBe(
      valueOf(REFUSAL_MESSAGES.KIND_NOT_IN_VOCABULARY),
    );
    expect(Object.keys(REFUSAL_CODES)).toEqual([
      'REWRITE_REFUSED',
      'MISSING_REQUIRED_FIELD',
      'KIND_NOT_IN_VOCABULARY',
    ]);
    expect(() => entry('candidate_proposed', {}, null)).toThrow(/not in the vocabulary/);
  });

  test('refuses a payload that is missing a field its kind requires, and names them', () => {
    const refusal = append([], 'gate_verdict', { candidateId: 'alpha', gate: 'G0' });

    expect(refusal.ok).toBe(false);
    expect(refusal.code).toBe('MISSING_REQUIRED_FIELD');
    expect(refusal.reason).toBe("a gate_verdict event is missing ['status', 'evidence_digest']");
  });

  test('does not append a record when the payload is missing a field', () => {
    const entries = chain([['proposal_recorded', { candidateId: 'alpha', generation: 0 }]]);
    const result = append(entries, 'specification_refused', { candidateId: 'gamma' });

    expect(result.ok).toBe(false);
    expect(entries).toHaveLength(1);
  });

  test('refuses a second ledger entry for one candidate identity', () => {
    const entries = chain(CAMPAIGN);
    const result = append(entries, 'compilation_recorded', {
      candidateId: 'alpha',
      specification_digest: 'sha256:spec-again',
      new_configuration: false,
    });

    expect(result.ok).toBe(false);
    expect(result.code).toBe('REWRITE_REFUSED');
    expect(result.reason).toBe('an entry for candidate alpha already exists at entries/alpha.json');

    // A different candidate is not a rewrite: the rule is per candidate identity, not per kind.
    const other = append(entries, 'compilation_recorded', {
      candidateId: 'beta',
      specification_digest: 'sha256:spec-beta',
      new_configuration: true,
    });
    expect(other.ok).toBe(true);
  });

  test('never rewrites an entry it already holds', () => {
    const entries = chain(CAMPAIGN);
    const before = JSON.parse(JSON.stringify(entries));
    const result = append(entries, 'proposal_recorded', { candidateId: 'beta', generation: 1 });

    expect(result.ok).toBe(true);
    expect(result.entries).toHaveLength(entries.length + 1);
    expect(result.entries.slice(0, entries.length)).toEqual(entries);
    result.entries.slice(0, entries.length).forEach((item, index) => {
      expect(item).toBe(entries[index]);
    });
    expect(JSON.parse(JSON.stringify(entries))).toEqual(before);
    expect(Object.isFrozen(entries[0])).toBe(true);
  });
});

describe('verification', () => {
  test('finds nothing in an intact chain and ends at the last entry it wrote', () => {
    const entries = chain(CAMPAIGN);
    const check = verifyChain(entries);

    expect(check.ok).toBe(true);
    expect(check.findings).toEqual([]);
    expect(check.brokenAt).toBe(null);
    expect(check.events).toBe(entries.length);
    expect(check.head).toBe(entries[entries.length - 1].chain);
    expect(check.links.every((link) => link.ok)).toBe(true);
  });

  test('breaks from the tampered entry onward, and finds the record that follows it', () => {
    const entries = chain(CAMPAIGN);
    const check = verifyChain(tamper(entries, 1, { new_configuration: false }));

    expect(check.ok).toBe(false);
    expect(check.brokenAt).toBe(1);
    expect(check.links[0].ok).toBe(true);
    expect(check.links.slice(1).every((link) => !link.ok)).toBe(true);

    // The module's own check reports the link at the record after the tampered one.
    expect(check.findings).toHaveLength(1);
    expect(check.findings[0]).toMatch(/^event 2 chains to '.*' instead of '.*'$/);
    expect(check.findings[0]).toContain(entries[1].digest);
  });

  test('catches a tampered final record, which the module\u2019s own link check cannot see', () => {
    const entries = chain(CAMPAIGN);
    const check = verifyChain(tamper(entries, entries.length - 1, { outcome: 'crashed' }));

    expect(check.findings).toEqual([]);
    expect(check.ok).toBe(false);
    expect(check.brokenAt).toBe(entries.length - 1);
  });

  test('reports a record removed from the chain through its sequence number', () => {
    const entries = chain(CAMPAIGN);
    const removed = entries.filter((_item, index) => index !== 1);
    const check = verifyChain(removed);

    expect(check.ok).toBe(false);
    expect(check.findings).toContain('event 1 records sequence 2: a record was removed or inserted');
  });

  test('compares the chain\u2019s end with a checkpoint held outside the writer', () => {
    const entries = chain(CAMPAIGN);
    const head = verifyChain(entries).head;

    expect(verifyChain(entries, head).findings).toEqual([]);
    expect(verifyChain(entries, 'sha256:0000').findings).toEqual([
      `the inventory ends at '${head}' and the checkpoint holds 'sha256:0000'`,
    ]);
    // No checkpoint given is no checkpoint compared, not a failure.
    expect(verifyChain(entries, undefined).ok).toBe(true);
  });

  test('reports findings as data rather than raising, and leaves the chain alone', () => {
    const entries = chain(CAMPAIGN);
    const before = JSON.parse(JSON.stringify(entries));
    // A payload member the kind does not require is still part of the digest, so the next record's
    // link no longer matches; the damaged chain is reported and the entries are left as they were.
    const check = verifyChain(tamper(entries, 2, { data_role: 'development' }));

    expect(typeof check.findings[0]).toBe('string');
    expect(JSON.parse(JSON.stringify(entries))).toEqual(before);
  });
});

describe('the documented limits', () => {
  test('names the receipt join and the engine digest the module and the page have since closed', () => {
    expect(LIMITS.map((limit) => limit.id)).toEqual(['receipt-join', 'engine-digest']);
    for (const limit of LIMITS) {
      expect(limit.resolved.length).toBeGreaterThan(0);
      expect(limit.page.length).toBeGreaterThan(0);
    }
    expect(LIMITS[0].page).toContain('was missing, and now exists');
    expect(LIMITS[1].page).toContain('could not be recomputed, and now can');
  });

  test('carries the append-only rule as the module states it', () => {
    expect(valueOf(CHAIN_RULES.appendOnly)).toContain('does not rewrite what it already learned');
  });
});
