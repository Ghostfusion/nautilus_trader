/**
 * Tests for the policy engine.
 *
 * These cases are the ones a reader of the console would notice breaking: the declared sets and the
 * bounds drifting from `src/data/policy.js`, a freeze that does not move when a declaration moves,
 * and a refusal that carries a code or a message the source does not state.
 */

import { describe, expect, test } from 'bun:test';

import {
  BOUNDS,
  DECLARED_SETS,
  DECLARED_SET_NAMES,
  LIMITS,
  REFUSAL_CODES as POLICY_REFUSALS,
} from '../src/data/policy.js';
import { valueOf } from '../src/data/source.js';
import { canonicalJson, digestOf, isDigest } from '../src/engine/digest.js';
import {
  REFUSAL_CODES,
  SET_NAMES,
  admits,
  boundFor,
  bounds,
  declaredSets,
  defaultPolicy,
  describeBound,
  freeze,
  limits,
  validatePolicy,
} from '../src/engine/policy.js';

/** A reference the source declares no artifact for, so the tests supply their own. */
function reference(name, kind, letter) {
  return { name, kind, digest: `sha256:${letter.repeat(64)}` };
}

/** The campaign inputs the source invents none of: an allowlist, three references, no sectors. */
function campaignInputs() {
  return {
    featureAllowlist: ['close', 'volume'],
    references: [
      reference('console_benchmark', 'benchmark', 'a'),
      reference('console_cost_model', 'cost_model', 'b'),
      reference('console_validation', 'validation_policy', 'c'),
    ],
  };
}

describe('the declarations', () => {
  test('fifteen declared sets, in the order DECLARED_SET_NAMES fixes', () => {
    // If this fails the console shows names under a field whose set the source does not read.
    expect(SET_NAMES).toEqual(valueOf(DECLARED_SET_NAMES));
    expect(SET_NAMES.length).toBe(15);
    expect(Object.keys(declaredSets())).toEqual(valueOf(DECLARED_SET_NAMES));
  });

  test('every set agrees with the transcription, member by member', () => {
    // If this fails a permitted value became forbidden, or the other way round, without the source
    // changing.
    const sets = declaredSets();
    for (const set of DECLARED_SETS) {
      expect(sets[valueOf(set.name)]).toEqual(set.members.map((member) => valueOf(member)));
    }
  });

  test('the nineteen bounds agree with the transcription, lower and upper', () => {
    // If this fails a value the repository admits is refused in the console, or vice versa.
    const declared = bounds();
    expect(declared.length).toBe(19);
    declared.forEach((bound, index) => {
      expect(bound.name).toBe(valueOf(BOUNDS[index].name));
      expect(bound.kind).toBe(valueOf(BOUNDS[index].kind));
      expect(bound.lower).toBe(valueOf(BOUNDS[index].lower));
      expect(bound.upper).toBe(valueOf(BOUNDS[index].upper));
      expect(bound.unit).toBe(valueOf(BOUNDS[index].unit));
    });
  });

  test('the three caps are the ones ComplexityLimits declares', () => {
    // If this fails the console would refuse a candidate the source accepts, or admit one it refuses.
    expect(limits()).toEqual({
      max_conditions: valueOf(LIMITS.max_conditions),
      max_condition_depth: valueOf(LIMITS.max_condition_depth),
      max_features: valueOf(LIMITS.max_features),
    });
  });

  test('a bound restates itself the way a refusal states it', () => {
    // If this fails every out-of-bound message in the console misquotes the bound.
    const minPrice = bounds().find((bound) => bound.name === 'universe.min_price');
    expect(minPrice.describe).toBe('1 to 1000 inclusive');
    expect(describeBound({ kind: 'choice', choices: ['a', 'b'] })).toBe('one of a, b');
  });

  test('both ends of a range are admitted, and a choice bound admits only its choices', () => {
    // This is the tutorial's own arithmetic: 1 <= 5 <= 1000 is true, and 2500 is refused by the
    // second comparison alone.
    const minPrice = bounds().find((bound) => bound.name === 'universe.min_price');
    expect(admits(minPrice, 1)).toBe(true);
    expect(admits(minPrice, 5)).toBe(true);
    expect(admits(minPrice, 1000)).toBe(true);
    expect(admits(minPrice, 1001)).toBe(false);
    expect(admits(minPrice, 2500)).toBe(false);
    expect(admits({ kind: 'choice', choices: [1, 2] }, 2)).toBe(true);
    expect(admits({ kind: 'choice', choices: [1, 2] }, 3)).toBe(false);
  });

  test('declaredSets hands back a fresh object rather than the table', () => {
    // If this fails one view mutating a set would change what every other view resolves against.
    const first = declaredSets();
    first.families.push('invented');
    expect(declaredSets().families).not.toContain('invented');
  });

  test('the first-release policy declares the transcriptions and invents no campaign input', () => {
    // The allowlist, the references and the sector map are the caller's, so a default policy that
    // filled them in would be claiming a campaign the source never declared.
    const policy = defaultPolicy();
    expect(Object.keys(policy.sets).length).toBe(15);
    expect(policy.bounds.length).toBe(19);
    expect(policy.references).toEqual([]);
    expect(policy.sectors).toEqual([]);
    expect(policy.featureAllowlist).toEqual([]);
    expect(policy.policyId).toBe('discovery.policy');
    expect(policy.version).toBe(1);
  });

  test('an override replaces one declared set or one cap without disturbing the rest', () => {
    // If this fails the reader cannot widen one set and see only that set change.
    const policy = defaultPolicy({ sets: { rebalance_frequencies: ['daily'] }, limits: { max_features: 2 } });
    expect(policy.sets.rebalance_frequencies).toEqual(['daily']);
    expect(policy.sets.families).toEqual(declaredSets().families);
    expect(policy.limits.max_features).toBe(2);
    expect(policy.limits.max_conditions).toBe(4);
  });
});

describe('the freeze', () => {
  test('the digest is the console digest of the canonical payload', () => {
    // If this fails the view shows a fingerprint that does not belong to the declarations beside it.
    const policy = defaultPolicy(campaignInputs());
    const frozen = freeze(policy);
    expect(isDigest(frozen.digest)).toBe(true);
    expect(frozen.digest).toBe(digestOf(frozen.policy));
    expect(frozen.canonical).toBe(canonicalJson(frozen.policy));
  });

  test('a changed bound moves the digest', () => {
    // The layer exists for this: one changed bound is a different campaign, not a tuning step.
    const policy = defaultPolicy(campaignInputs());
    const narrowed = {
      ...policy,
      bounds: policy.bounds.map((bound) =>
        bound.name === 'portfolio.max_position_weight' ? { ...bound, upper: 0.3 } : bound,
      ),
    };
    expect(freeze(narrowed).digest).not.toBe(freeze(policy).digest);
  });

  test('a changed set moves the digest, and the order a set was written in does not', () => {
    // The source sorts every set in its canonical form, so two policies declaring the same names in
    // a different order are one policy, and adding a name is a different one.
    const policy = defaultPolicy(campaignInputs());
    const reordered = {
      ...policy,
      sets: { ...policy.sets, operators: ['le', 'lt', 'ge', 'gt'] },
      featureAllowlist: ['volume', 'close'],
    };
    expect(freeze(reordered).digest).toBe(freeze(policy).digest);

    const widened = { ...policy, sets: { ...policy.sets, rebalance_frequencies: ['daily', 'weekly', 'monthly', 'hourly'] } };
    expect(freeze(widened).digest).not.toBe(freeze(policy).digest);
  });

  test('the order of the policy object keys does not move the digest', () => {
    // If this fails a rebuilt policy would look like a changed one because of how it was assembled.
    const policy = defaultPolicy(campaignInputs());
    const shuffled = {
      featureAllowlist: policy.featureAllowlist,
      version: policy.version,
      sectors: policy.sectors,
      bounds: policy.bounds,
      references: policy.references,
      limits: policy.limits,
      sets: policy.sets,
      policyId: policy.policyId,
    };
    expect(freeze(shuffled).digest).toBe(freeze(policy).digest);
  });

  test('the frozen payload carries the declarations and no more', () => {
    // A payload that dropped a set or a bound would fingerprint less than the policy declares.
    const frozen = freeze(defaultPolicy(campaignInputs()));
    expect(frozen.policy.policy_id).toBe('discovery.policy');
    expect(Object.keys(frozen.policy).filter((key) => SET_NAMES.includes(key)).length).toBe(15);
    expect(frozen.policy.bounds.length).toBe(19);
    expect(frozen.policy.limits).toEqual(limits());
    expect(frozen.policy.feature_allowlist).toEqual(['close', 'volume']);
  });
});

describe('validatePolicy', () => {
  test('the first-release policy with campaign inputs is accepted', () => {
    // If this fails every other layer's default policy is refused before it can be used.
    const policy = defaultPolicy(campaignInputs());
    const result = validatePolicy(policy);
    expect(result.ok).toBe(true);
    expect(result.policy).toBe(policy);
  });

  test('an empty declared set is refused with the source message', () => {
    // The source raises "{name} must not be empty"; an empty set declares nothing, so every value of
    // its field would be refused as undeclared.
    const policy = defaultPolicy({ ...campaignInputs(), sets: { operators: [] } });
    const result = validatePolicy(policy);
    expect(result.ok).toBe(false);
    expect(result.code).toBe('field_empty');
    expect(result.reason).toBe('operators must not be empty');
  });

  test('a set holding an empty name is refused', () => {
    // If this fails a blank name would be treated as a declared value.
    const policy = defaultPolicy({ ...campaignInputs(), sets: { operators: ['gt', ''] } });
    const result = validatePolicy(policy);
    expect(result.ok).toBe(false);
    expect(result.code).toBe('field_type');
    expect(result.reason).toBe('operators must hold non-empty names');
  });

  test('an inverted range bound is refused as the bound rule states it', () => {
    // A range whose lower limit is above its upper one admits nothing, and the source refuses to
    // build it rather than shipping a bound that refuses every value of its field.
    const policy = defaultPolicy(campaignInputs());
    const inverted = {
      ...policy,
      bounds: policy.bounds.map((bound) =>
        bound.name === 'universe.min_price' ? { ...bound, lower: 1000, upper: 1 } : bound,
      ),
    };
    const result = validatePolicy(inverted);
    expect(result.ok).toBe(false);
    expect(result.code).toBe('value_out_of_bound');
    expect(result.reason).toContain('a range bound requires lower <= upper');
  });

  test('a bound without both of its limits is refused', () => {
    // "an unstated bound is not an unbounded one": a range missing an end cannot admit a value.
    const policy = defaultPolicy(campaignInputs());
    const broken = {
      ...policy,
      bounds: policy.bounds.map((bound) =>
        bound.name === 'feature.skip' ? { name: 'feature.skip', kind: 'range', lower: null, upper: 5, choices: [] } : bound,
      ),
    };
    const result = validatePolicy(broken);
    expect(result.ok).toBe(false);
    expect(result.code).toBe('field_missing');
    expect(result.reason).toContain('a range bound requires both limits, feature.skip states');
  });

  test('two bounds with one name are refused', () => {
    // A repeated name makes a lookup ambiguous, which is what the source refuses it for.
    const policy = defaultPolicy(campaignInputs());
    const repeated = { ...policy, bounds: [...policy.bounds, { ...boundFor(policy, 'universe.min_price') }] };
    const result = validatePolicy(repeated);
    expect(result.ok).toBe(false);
    expect(result.code).toBe('field_repeated');
    expect(result.reason).toBe('bounds must have distinct names');
  });

  test('a reference whose digest is not a sha256 digest is refused', () => {
    // A reference pins a fingerprint, so one that pins nothing resolves to nothing.
    const policy = defaultPolicy({
      ...campaignInputs(),
      references: [reference('console_benchmark', 'benchmark', 'a'), { name: 'short', kind: 'benchmark', digest: 'sha256:abc' }],
    });
    const result = validatePolicy(policy);
    expect(result.ok).toBe(false);
    expect(result.code).toBe('reference_unresolved');
    expect(result.reason).toBe('digest must be a sha256 digest, was sha256:abc');
  });

  test('a cap below one is refused, and an allowlist holding an empty identifier too', () => {
    // Both are construction rules of the source rather than reader rules, and the console reports
    // them instead of raising.
    const capped = defaultPolicy({ ...campaignInputs(), limits: { max_features: 0 } });
    expect(validatePolicy(capped).code).toBe('value_out_of_bound');
    expect(validatePolicy(capped).reason).toBe('max_features must be positive, was 0');

    const allowlist = defaultPolicy({ ...campaignInputs(), featureAllowlist: ['close', ''] });
    expect(validatePolicy(allowlist).code).toBe('field_type');
    expect(validatePolicy(allowlist).reason).toBe('feature_allowlist must hold non-empty identifiers');
  });
});

describe('the refusal vocabulary', () => {
  test('the six codes are the ones the source names', () => {
    // If this fails a refusal in the console carries a code the repository never records.
    expect(REFUSAL_CODES).toEqual(POLICY_REFUSALS.map((row) => valueOf(row.code)));
    expect(REFUSAL_CODES).toEqual([
      'value_not_declared',
      'bound_not_declared',
      'value_out_of_bound',
      'feature_not_allowed',
      'feature_not_declared',
      'reference_unresolved',
    ]);
  });
});
