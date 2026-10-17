/**
 * The policy engine.
 *
 * This module models the pre-registered declarations of
 * `python/nautilus_trader/discovery/policy.py` and the two rules
 * `python/nautilus_trader/discovery/schema.py` reads them by: `Bound.admits` (the inclusive range or
 * membership of the choice list) and `Bound.describe` (how a refusal restates the bound). It hands a
 * caller the fifteen declared sets, the nineteen bounds, the policy of the declared first-release
 * scope, a check of a policy against the source's own construction rules, and the freeze.
 *
 * The freeze is the part the policy layer exists for: the policy is fingerprinted before the campaign
 * opens, and one changed bound or one changed set moves the fingerprint. `policy.py` digests
 * `canonical_mapping(self.to_dict())`, whose sets and allowlist are sorted and whose bounds and
 * references keep their declared order, so this module builds that same plain-data form and digests
 * it. The digest here is the console's SHA-256 over canonically serialised JSON; the repository's
 * canonical digest is blake3, so the two are never compared.
 *
 * A malformed policy makes `policy.py` raise at construction rather than return a refusal. The
 * console's rule is that a refusal is a value, so `validatePolicy` reports one, using the code in the
 * closed vocabulary whose own rule the failure breaks and the message the source raises with.
 */

import {
  BOUND_KINDS,
  BOUND_RULES,
  BOUNDS,
  DECLARED_SETS,
  DECLARED_SET_NAMES,
  LIMITS,
  POLICY_ID,
  POLICY_RULES,
  POLICY_VERSION,
  REFERENCE_DIGEST_REFUSAL,
  REFERENCE_KINDS,
  REFUSAL_CODES as REFUSAL_ROWS,
  SECTORS,
  SHA256_DIGEST_LENGTH,
} from '../data/policy.js';
import { valueOf } from '../data/source.js';

import { canonicalJson, digestOf } from './digest.js';

/** The six refusal codes a value resolved through the policy's declarations can carry. */
export const REFUSAL_CODES = Object.freeze(REFUSAL_ROWS.map((row) => valueOf(row.code)));

/** The fifteen declared-set names, in the order the source fixes them. */
export const SET_NAMES = Object.freeze([...valueOf(DECLARED_SET_NAMES)]);

/** The three reference kinds a policy may pin. */
export const REFERENCE_KIND_NAMES = Object.freeze(REFERENCE_KINDS.map((kind) => valueOf(kind)));

/** The two bound kinds, so an unknown kind is reported as the programming error it is. */
export const BOUND_KIND_NAMES = Object.freeze(BOUND_KINDS.map((kind) => valueOf(kind)));

/** The three complexity caps, keyed by the name the source declares them under. */
export function limits() {
  const caps = {};
  for (const [name, value] of Object.entries(LIMITS)) caps[name] = valueOf(value);
  return caps;
}

/**
 * The fifteen declared sets: name to members, in `DECLARED_SET_NAMES` order.
 *
 * Members keep the order of the source's `frozenset` literal here, which is the transcription's
 * order; only the canonical form sorts them, because only the canonical form is digested.
 */
export function declaredSets() {
  const sets = {};
  for (const set of DECLARED_SETS) sets[valueOf(set.name)] = set.members.map((member) => valueOf(member));
  return sets;
}

/** The nineteen bounds, in declared order, each carrying the message a refusal would state. */
export function bounds() {
  return BOUNDS.map((row) => {
    const lower = valueOf(row.lower);
    const upper = valueOf(row.upper);
    const bound = { name: valueOf(row.name), kind: valueOf(row.kind), lower, upper, choices: [] };
    return { ...bound, unit: valueOf(row.unit), describe: describeBound(bound) };
  });
}

/** `Bound.describe()`: the wording a refusal states its bound in. */
export function describeBound(bound) {
  if (bound.kind === 'choice') return `one of ${bound.choices.join(', ')}`;
  return `${bound.lower} to ${bound.upper} inclusive`;
}

/**
 * `Bound.admits()`: both ends of a range are included, a choice bound admits membership only.
 *
 * A value that is not a number is not admitted, because `decimal_of` refuses it before the bound is
 * consulted (`schema.py`); `false` here therefore never stands in for a type refusal.
 */
export function admits(bound, value) {
  if (bound.kind === 'choice') return bound.choices.includes(value);
  if (typeof value !== 'number' || Number.isNaN(value)) return false;
  return bound.lower <= value && value <= bound.upper;
}

/** The bound the policy declares under a name, or null when it declares none. */
export function boundFor(policy, name) {
  return (policy?.bounds ?? []).find((bound) => bound.name === name) ?? null;
}

/**
 * The policy of the declared first-release scope, with the caller's own inputs.
 *
 * The sets, bounds and caps are transcribed; the feature allowlist, the frozen references and the
 * sector map are campaign inputs the source invents none of (`policy.py:507-509`), so they default
 * to empty and a caller must supply them. `overrides` replaces a whole collection, or a single
 * declared set or cap by name.
 */
export function defaultPolicy(overrides = {}) {
  return {
    policyId: overrides.policyId ?? valueOf(POLICY_ID),
    version: overrides.version ?? valueOf(POLICY_VERSION),
    sets: { ...declaredSets(), ...(overrides.sets ?? {}) },
    limits: { ...limits(), ...(overrides.limits ?? {}) },
    bounds: (overrides.bounds ?? bounds()).map((bound) => ({ ...bound })),
    references: (overrides.references ?? []).map((reference) => ({ ...reference })),
    sectors: (overrides.sectors ?? valueOf(SECTORS)).map(([instrumentId, sector]) => [
      instrumentId,
      sector,
    ]),
    featureAllowlist: [...(overrides.featureAllowlist ?? [])],
  };
}

/**
 * The `to_dict` form of a policy: sets and allowlist sorted, everything else in declared order.
 *
 * This is the shape `DiscoveryPolicy.digest` serialises, so it is the shape the two-provider rule
 * compares: two policies declaring the same things in a different order are the same text.
 */
export function policyPayload(policy) {
  const sorted = {};
  for (const name of SET_NAMES) sorted[name] = [...(policy.sets?.[name] ?? [])].sort();
  return {
    policy_id: policy.policyId,
    version: policy.version,
    ...sorted,
    feature_allowlist: [...(policy.featureAllowlist ?? [])].sort(),
    bounds: (policy.bounds ?? []).map((bound) => ({
      name: bound.name,
      kind: bound.kind,
      lower: bound.lower,
      upper: bound.upper,
      choices: [...(bound.choices ?? [])],
    })),
    limits: { ...policy.limits },
    references: (policy.references ?? []).map((reference) => ({
      name: reference.name,
      kind: reference.kind,
      digest: reference.digest,
    })),
    sectors: (policy.sectors ?? []).map(([instrumentId, sector]) => [instrumentId, sector]),
  };
}

/**
 * Freeze a policy: its canonical plain-data form, that text, and the console's digest of it.
 *
 * The freeze is a record rather than a wall. A later policy does not stop an earlier result; it makes
 * that result read as judged under a different fingerprint, which is what comparing two digests
 * finds out.
 */
export function freeze(policy) {
  const payload = policyPayload(policy);
  return { policy: payload, canonical: canonicalJson(payload), digest: digestOf(payload) };
}

/** A refusal, as the console reports one: a value rather than an exception. */
function refuse(code, reason) {
  return { ok: false, code, reason };
}

/** The Python type name a message would state for a plain JavaScript value. */
export function pythonType(value) {
  if (value === null || value === undefined) return 'NoneType';
  if (Array.isArray(value)) return 'list';
  if (typeof value === 'string') return 'str';
  if (typeof value === 'boolean') return 'bool';
  if (typeof value === 'number') return Number.isInteger(value) ? 'int' : 'float';
  if (typeof value === 'object') return 'dict';
  return typeof value;
}

/** Fill a message template's `{name}` placeholders with the values a rule states. */
function fill(template, values) {
  return template.replace(/\{(\w+)\}/g, (match, key) => (key in values ? String(values[key]) : match));
}

/** One bound against the rules `Bound.__post_init__` enforces, in the order it enforces them. */
function boundFailure(bound) {
  if (bound === null || typeof bound !== 'object' || Array.isArray(bound)) {
    return refuse('field_type', valueOf(POLICY_RULES.bounds));
  }
  const name = bound.name;
  if (typeof name !== 'string' || name === '') return refuse('field_empty', valueOf(BOUND_RULES.name));

  if (bound.kind === 'range') {
    if (typeof bound.lower !== 'number' || typeof bound.upper !== 'number') {
      return refuse(
        'field_missing',
        fill(valueOf(BOUND_RULES.limits), { name, lower: bound.lower, upper: bound.upper }),
      );
    }
    if ((bound.choices ?? []).length > 0) {
      return refuse('field_type', fill(valueOf(BOUND_RULES.rangeChoices), { name }));
    }
    if (bound.lower > bound.upper) {
      return refuse(
        'value_out_of_bound',
        fill(valueOf(BOUND_RULES.order), { name, lower: bound.lower, upper: bound.upper }),
      );
    }
    return null;
  }

  if (bound.kind === 'choice') {
    if (bound.lower !== undefined || bound.upper !== undefined) {
      return refuse('field_type', fill(valueOf(BOUND_RULES.choiceLimits), { name }));
    }
    const choices = bound.choices ?? [];
    if (choices.length === 0) {
      return refuse('field_missing', fill(valueOf(BOUND_RULES.choiceRequired), { name }));
    }
    if (new Set(choices).size !== choices.length) {
      return refuse('field_repeated', fill(valueOf(BOUND_RULES.choiceDistinct), { name }));
    }
    return null;
  }

  throw new TypeError(
    `a bound kind must be one of ${BOUND_KIND_NAMES.join(', ')}, was ${String(bound.kind)}`,
  );
}

/** One reference against the rules `Reference.__post_init__` enforces, in the order it enforces them. */
function referenceFailure(reference) {
  if (reference === null || typeof reference !== 'object' || Array.isArray(reference)) {
    return refuse('field_type', valueOf(POLICY_RULES.references));
  }
  if (typeof reference.name !== 'string') {
    return refuse(
      'field_type',
      fill(valueOf(POLICY_RULES.referenceNameType), { type: pythonType(reference.name) }),
    );
  }
  if (reference.name === '') return refuse('field_empty', valueOf(POLICY_RULES.referenceName));
  if (typeof reference.kind !== 'string' || !REFERENCE_KIND_NAMES.includes(reference.kind)) {
    return refuse(
      'field_type',
      fill(valueOf(POLICY_RULES.referenceKindType), { type: pythonType(reference.kind) }),
    );
  }
  if (typeof reference.digest !== 'string') {
    return refuse(
      'field_type',
      fill(valueOf(POLICY_RULES.referenceDigestType), { type: pythonType(reference.digest) }),
    );
  }
  if (
    !reference.digest.startsWith('sha256:') ||
    reference.digest.length !== valueOf(SHA256_DIGEST_LENGTH)
  ) {
    return refuse(
      'reference_unresolved',
      fill(valueOf(REFERENCE_DIGEST_REFUSAL), { digest: reference.digest }),
    );
  }
  return null;
}

/**
 * Check a policy against the rules `policy.py` enforces at construction, in its own order.
 *
 * The source raises at the first rule it finds broken, so this returns the first failure rather than
 * collecting them, and the reason is the message the source raises with. A policy that is not an
 * object at all is a programming error in the console and throws, because no caller can reach it:
 * `defaultPolicy` is the only builder.
 */
export function validatePolicy(policy) {
  if (policy === null || typeof policy !== 'object' || Array.isArray(policy)) {
    throw new TypeError(`a policy must be a plain object, was ${pythonType(policy)}`);
  }

  for (const name of SET_NAMES) {
    const members = policy.sets?.[name];
    if (!Array.isArray(members)) {
      return refuse(
        'field_type',
        fill(valueOf(POLICY_RULES.declaredSetType), { name, type: pythonType(members) }),
      );
    }
    if (members.length === 0) {
      return refuse('field_empty', fill(valueOf(POLICY_RULES.declaredSetEmpty), { name }));
    }
    if (!members.every((member) => typeof member === 'string' && member !== '')) {
      return refuse('field_type', fill(valueOf(POLICY_RULES.declaredSetNames), { name }));
    }
  }

  if (!Array.isArray(policy.featureAllowlist)) {
    return refuse(
      'field_type',
      fill(valueOf(POLICY_RULES.allowlistType), { type: pythonType(policy.featureAllowlist) }),
    );
  }
  if (!policy.featureAllowlist.every((id) => typeof id === 'string' && id !== '')) {
    return refuse('field_type', valueOf(POLICY_RULES.allowlistNames));
  }
  if (!Array.isArray(policy.bounds)) return refuse('field_type', valueOf(POLICY_RULES.bounds));
  if (!Array.isArray(policy.references)) {
    return refuse('field_type', valueOf(POLICY_RULES.references));
  }
  if (policy.limits === null || typeof policy.limits !== 'object' || Array.isArray(policy.limits)) {
    return refuse('field_type', fill(valueOf(POLICY_RULES.limits), { type: pythonType(policy.limits) }));
  }
  if (!Array.isArray(policy.sectors)) {
    return refuse('field_type', fill(valueOf(POLICY_RULES.sectors), { type: pythonType(policy.sectors) }));
  }

  for (const name of Object.keys(LIMITS)) {
    const cap = policy.limits[name];
    if (typeof cap !== 'number' || !Number.isInteger(cap)) {
      return refuse(
        'field_type',
        fill(valueOf(POLICY_RULES.capType), { field: name, type: pythonType(cap) }),
      );
    }
    if (cap < 1) {
      return refuse('value_out_of_bound', fill(valueOf(POLICY_RULES.capPositive), { field: name, value: cap }));
    }
  }

  for (const bound of policy.bounds) {
    const failure = boundFailure(bound);
    if (failure) return failure;
  }

  if (typeof policy.policyId !== 'string' || policy.policyId === '') {
    return refuse('field_empty', valueOf(POLICY_RULES.policyId));
  }
  if (typeof policy.version !== 'number' || !Number.isInteger(policy.version)) {
    return refuse(
      'field_type',
      fill(valueOf(POLICY_RULES.versionType), { type: pythonType(policy.version) }),
    );
  }
  if (policy.version < 1) {
    return refuse('value_out_of_bound', fill(valueOf(POLICY_RULES.versionPositive), { version: policy.version }));
  }

  for (const reference of policy.references) {
    const failure = referenceFailure(reference);
    if (failure) return failure;
  }

  if (new Set(policy.bounds.map((bound) => bound.name)).size !== policy.bounds.length) {
    return refuse('field_repeated', valueOf(POLICY_RULES.boundNames));
  }
  if (new Set(policy.references.map((reference) => reference.name)).size !== policy.references.length) {
    return refuse('field_repeated', valueOf(POLICY_RULES.referenceNames));
  }
  if (new Set(policy.sectors.map(([instrumentId]) => instrumentId)).size !== policy.sectors.length) {
    return refuse('field_repeated', valueOf(POLICY_RULES.sectorIds));
  }

  return { ok: true, policy };
}
