/**
 * The execution boundary as a model, from `python/tests/integration/test_discovery_boundary.py` and
 * design Section 12 of `docs/design/strategy_discovery_design.md`.
 *
 * Four assertions make the absence of execution authority observable: the import graph reaches no
 * adapter module, the only execution client a worker is given is the sandbox one, no credential from
 * the repository's canonical list is in the environment the boundary defines, and a candidate that
 * names an execution venue is refused and the refusal is recorded. This module models the four as
 * pure functions over the transcribed values in `../data/boundary.js`, so a reader can put a live
 * adapter into a simulated graph, put a credential into a simulated environment, or declare a
 * venue-shaped field, and watch the assertion answer rather than take the page's word for it.
 *
 * Two things the model does not claim, because the repository does not state them. The import-graph
 * check is evidence rather than proof: a dynamic import, an indirect callback or a shared execution
 * stack evades a check made over module names, which is why the module is a regression check. And the
 * stronger control, the capability allowlist as the shape of a worker environment, has no
 * implementation to model: `config.py` is unbuilt, so `capacities()` reports the sandbox surface and
 * names the allowlist as procedural, without pretending to check it.
 *
 * Every returned value is plain JSON data. A refusal is a value: `{ ok: false, code, path, reason }`.
 */

import { valueOf } from '../data/source.js';
import {
  AUTHORITY,
  CAPABILITY,
  CODE_FIELD_NAMES,
  CODE_VALUE_PATTERNS,
  CREDENTIALS,
  DEPENDENCY,
  RECORDED,
  REFUSALS,
  SANDBOX_SURFACE,
} from '../data/boundary.js';

const ADAPTER_PREFIX = valueOf(DEPENDENCY.prefix);
const SUBMISSION_EXPORTS = valueOf(DEPENDENCY.submissionExports);
const EXPORT_FLOOR = valueOf(DEPENDENCY.exportFloor);
const OBSERVED_EXPORTS = valueOf(DEPENDENCY.observedExports);

const SANDBOX_CONFIG = valueOf(SANDBOX_SURFACE.config);
const SANDBOX_FACTORY = valueOf(SANDBOX_SURFACE.factory);
const SANDBOX_EXPORTS = [SANDBOX_CONFIG, SANDBOX_FACTORY];

const CREDENTIAL_FLOOR = valueOf(CREDENTIALS.floor);
const CREDENTIAL_ANCHORS = CREDENTIALS.anchors.map(valueOf);

const CODE_FIELD_SET = valueOf(CODE_FIELD_NAMES);
const CODE_PATTERNS = CODE_VALUE_PATTERNS.map((item) => new RegExp(valueOf(item), 'u'));

const UNKNOWN_FIELD_MESSAGE = valueOf(REFUSALS.unknownFieldMessage);
const CODE_FIELD_MESSAGE = valueOf(REFUSALS.codeFieldMessage);
const NON_PLAIN_MESSAGE = valueOf(REFUSALS.nonPlainMessage);
const EXECUTABLE_MESSAGE = valueOf(REFUSALS.executableMessage);

/** The two refusal codes this layer can produce, transcribed from the schema's closed vocabulary. */
export const REFUSAL_CODES = Object.freeze({
  unknownField: valueOf(REFUSALS.unknownField),
  codeReference: valueOf(REFUSALS.codeReference),
});

const DATA_CLIENT = 'DataClient';

function join(path, key) {
  return path === '' ? String(key) : `${path}.${key}`;
}

function fill(template, values) {
  return template.replace(/\{(\w+)\}/g, (whole, name) => (name in values ? values[name] : whole));
}

function unknownFieldRefusal(path, key) {
  return {
    ok: false,
    code: REFUSAL_CODES.unknownField,
    path: join(path, key),
    reason: fill(UNKNOWN_FIELD_MESSAGE, { path, key }),
  };
}

function codeFieldRefusal(path, key) {
  return {
    ok: false,
    code: REFUSAL_CODES.codeReference,
    path,
    reason: fill(CODE_FIELD_MESSAGE, { key }),
  };
}

function nonPlainRefusal(path, pythonType) {
  return {
    ok: false,
    code: REFUSAL_CODES.codeReference,
    path,
    reason: fill(NON_PLAIN_MESSAGE, { type: pythonType }),
  };
}

function executableRefusal(path, repr) {
  return {
    ok: false,
    code: REFUSAL_CODES.codeReference,
    path,
    reason: fill(EXECUTABLE_MESSAGE, { value: repr }),
  };
}

function isMapping(node) {
  return node !== null && typeof node === 'object' && !Array.isArray(node);
}

function isPlainValue(node) {
  return (
    typeof node === 'string' ||
    typeof node === 'number' ||
    typeof node === 'boolean' ||
    node === null
  );
}

/** The Python type name the schema's message carries, so the message reads as the source writes it. */
function pythonTypeName(node) {
  if (node === undefined || node === null) return 'NoneType';
  if (Array.isArray(node)) return 'list';
  if (typeof node === 'bigint') return 'int';
  return typeof node;
}

/** Python's `repr` for a string value, which is the form the executable-code message quotes. */
function pythonStringRepr(text) {
  return `'${text.replace(/\\/g, '\\\\').replace(/'/g, "\\'")}'`;
}

/**
 * The surface the dependency assertion reads: which modules the graph reached, which of them are
 * venue adapters, how many names the package offers, and which offered names would mean the package
 * reaches an execution surface of its own.
 *
 * `graph` is `{ adapters: [...], exports: { count, named } }`: the modules the clean interpreter
 * loaded, how many names the package offers, and the offered names that matter to the filter. With no
 * argument the observed reading is used - no adapter module, the count the page records, and no
 * offered name that would mean an execution surface. Handing in an adapter, a shortened count or an
 * execution-shaped name is how the view shows the assertion has teeth, because the three conjuncts
 * are asserted in a fixed order and any one of them fails the boundary.
 */
export function executedSurface(graph) {
  const modules = [...(graph?.adapters ?? [])].map(String);
  const adapters = modules.filter((name) => name.startsWith(ADAPTER_PREFIX)).sort();

  const offered = graph?.exports ?? {};
  const count = offered.count ?? OBSERVED_EXPORTS;
  const named = [...(offered.named ?? [])].map(String).sort();
  const forbidden = named.filter((name) => SUBMISSION_EXPORTS.some((part) => name.includes(part)));

  return {
    prefix: ADAPTER_PREFIX,
    reached: modules.sort(),
    adapters,
    count,
    floor: EXPORT_FLOOR,
    named,
    forbidden,
    holds: adapters.length === 0 && count > EXPORT_FLOOR && forbidden.length === 0,
  };
}

/**
 * What a worker is given: the sandbox execution client and its config, and no data client. The
 * asymmetry is the capability, so the same list decides both halves.
 *
 * The allowlist the design asks for has no implementation in the repository - the layout's
 * configuration module is unbuilt - so the returned `allowlist.built` is false and `allowlist.why`
 * carries the plan's own sentence. Nothing here checks a package set, because there is none to check.
 */
export function capacities(exports) {
  const offered = [...(exports ?? SANDBOX_EXPORTS)].map(String).sort();
  const dataClients = offered.filter((name) => name.includes(DATA_CLIENT));

  return {
    exports: offered,
    executionClient: offered.includes(SANDBOX_FACTORY),
    executionConfig: offered.includes(SANDBOX_CONFIG),
    dataClients,
    holds:
      offered.includes(SANDBOX_FACTORY) &&
      offered.includes(SANDBOX_CONFIG) &&
      dataClients.length === 0,
    allowlist: {
      built: false,
      why: valueOf(CAPABILITY.unbuilt),
      asked: valueOf(CAPABILITY.asked),
    },
  };
}

/**
 * The execution authority one role holds, from the two sentences the design writes about roles: only
 * the sandbox venue is exposed to a strategy worker, and the orchestrator and the result-registration
 * process carry no execution capability at all.
 *
 * An unknown role is a wiring mistake rather than a condition to render, so it throws with the three
 * roles the design names.
 */
export function authorityOf(role) {
  const row = AUTHORITY.find((item) => item.id === role);
  if (!row) {
    throw new Error(
      `unknown role: ${role}. The boundary names a strategy worker, an orchestrator and a result-registration process.`,
    );
  }
  return {
    role: row.id,
    execution: row.execution,
    dataClient: row.dataClient,
    credential: row.credential,
    note: valueOf(row.note),
  };
}

/**
 * Check an environment against the canonical credential list.
 *
 * The list is read rather than restated, so the first half of the answer is about the list itself:
 * its size against the floor the fixture asserts and the three names it asserts by name. The second
 * half is about the environment: the names the list claims (`ambient`), the environment the boundary
 * defines (the same environment with those names removed), and whether what was handed in is clean.
 *
 * An ambient credential is reported and never hidden - the fixture reports the operator's shell
 * rather than failing on it - and the boundary refuses only to define an environment that carries
 * one, because a submission path that survived would then have something to authenticate with. The
 * repository states no code for that case, so the refusal is reported with the schema's unknown-field
 * code for the declaration that carries the name, and the view says so beside it.
 */
export function checkEnvironment(env, credentials) {
  const named = [...new Set((credentials ?? []).map(valueOf).map(String))].sort();
  const entries = Object.entries(env ?? {})
    .map(([name, value]) => [String(name), value])
    .sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0));

  const ambient = entries.filter(([name]) => named.includes(name)).map(([name]) => name);
  const stripped = {};
  for (const [name, value] of entries) {
    if (!named.includes(name)) stripped[name] = value;
  }

  const clean = ambient.length === 0;

  return {
    ok: clean,
    clean,
    ambient,
    stripped,
    declared: entries.map(([name]) => name),
    list: {
      size: named.length,
      floor: CREDENTIAL_FLOOR,
      holds: named.length > CREDENTIAL_FLOOR,
      anchors: CREDENTIAL_ANCHORS.map((name) => ({ name, named: named.includes(name) })),
    },
    refusal: clean ? null : unknownFieldRefusal('environment', ambient[0]),
  };
}

function scanForCode(payload, path) {
  if (isMapping(payload)) {
    for (const [key, value] of Object.entries(payload)) {
      if (CODE_FIELD_SET.includes(key.toLowerCase())) return codeFieldRefusal(join(path, key), key);
      const found = scanForCode(value, join(path, key));
      if (found) return found;
    }
    return null;
  }

  if (Array.isArray(payload)) {
    for (let index = 0; index < payload.length; index += 1) {
      const found = scanForCode(payload[index], join(path, index));
      if (found) return found;
    }
    return null;
  }

  if (!isPlainValue(payload)) return nonPlainRefusal(path, pythonTypeName(payload));

  if (typeof payload === 'string') {
    for (const pattern of CODE_PATTERNS) {
      if (pattern.test(payload)) return executableRefusal(path, pythonStringRepr(payload));
    }
  }

  return null;
}

function scanForUnknown(payload, declared, path) {
  if (!isMapping(payload)) return null;

  const names = declared[path];
  if (Array.isArray(names)) {
    for (const key of Object.keys(payload)) {
      if (!names.includes(key)) return unknownFieldRefusal(path, key);
    }
  }

  for (const [key, value] of Object.entries(payload)) {
    const found = scanForUnknown(value, declared, join(path, key));
    if (found) return found;
  }

  return null;
}

/**
 * Refuse a candidate's field, if it is one the language may not carry.
 *
 * The order is the compiler's, which is the part a reader cannot guess: the code-reference scan runs
 * over the whole payload before any block is read, and a field the block does not declare is refused
 * last, by the block reader's own `finish()`. So a payload that names both a code field and an
 * undeclared field is refused for the code field, whichever comes first in the payload.
 *
 * `payload` is plain specification data: `{ block: { field: value } }`. `declared` is the field names
 * each block declares, keyed by the block's dotted path, because the block's field set belongs to the
 * candidate language and is transcribed with it.
 */
export function refuseSubmission(payload, declared = {}) {
  const code = scanForCode(payload, '');
  if (code) return code;
  const unknown = scanForUnknown(payload, declared, '');
  if (unknown) return unknown;
  return { ok: true, code: null, path: null, reason: null };
}

/**
 * The fixture's own reading of a compilation: each refusal as its code and the path it was raised
 * against, one pair per field the compilation turned down.
 */
export function refusalPairs(refusals) {
  return refusals.map((refusal) => [refusal.code, refusal.path]);
}

/**
 * What the audit reads back from the record: the candidate is refused, has no attempts, carries the
 * refusal codes in the order they were recorded, and counts no proposal attempt and no executed
 * experiment. Refusals are recorded as refusals, never as work.
 */
export function recordRefusals(pairs) {
  return {
    candidate: valueOf(RECORDED.candidate),
    outcome: valueOf(RECORDED.outcome),
    attempts: valueOf(RECORDED.attempts),
    refusals: pairs.map(([code]) => code),
    counts: {
      proposal_attempts: valueOf(RECORDED.proposalAttempts),
      executed_experiments: valueOf(RECORDED.executedExperiments),
    },
  };
}
