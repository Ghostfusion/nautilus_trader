/**
 * The candidate engine.
 *
 * This module models `python/nautilus_trader/discovery/schema.py`, the four stages of
 * `python/nautilus_trader/discovery/validate.py` and the compilation stage of
 * `python/nautilus_trader/discovery/compile.py`. `validateSpecification` runs the five stages in the
 * source's order - schema validation, feature resolution, semantic validation, temporal validation,
 * trusted compilation - and stops at the first stage that refuses, so a refused candidate is an
 * outcome carrying its refusals rather than an exception. Inside the schema stage the order is the
 * source's too: the code scan over every key and value at every depth, then the version and its
 * migrations, then the twelve blocks with the features block read first, then the specification.
 *
 * Two inputs are the caller's rather than the console's, because the source's `first_release` invents
 * neither: the feature allowlist on the policy, and the registry of feature definitions handed to
 * stage 2 as `options.registry`. A registry that holds nothing therefore refuses every declared
 * feature with `feature_unresolved`, which is the honest result rather than a missing definition.
 *
 * The digests stage 5 computes are the console's SHA-256 over canonically serialised JSON. The
 * repository's canonical digest is blake3, and two of a plan's inputs have no counterpart here - the
 * assumptions digest of the bar-ambiguity policy, and the engine version - so they are reported as
 * not modelled rather than replaced by a plausible value.
 */

import {
  BLOCK_NAMES,
  BLOCKS as BLOCK_ROWS,
  CAPS as CAP_ROWS,
  CODE_FIELD_NAMES,
  CODE_VALUE_PATTERNS,
  COMPILE_DIGEST_KEYS,
  ENGINE_VERSION_UNKNOWN,
  EXECUTION_INSTANT_ORDER,
  FREQUENCY_SESSIONS,
  FULLY_INVESTED,
  MAX_MIGRATIONS,
  MIGRATIONS,
  NOT_MODELLED_INPUTS,
  REFUSAL_CODES as REFUSAL_ROWS,
  REFUSAL_PATHS,
  RESOLUTION_PLACES as RESOLUTION_PLACE_FACTS,
  RULE_SUBJECTS,
  SCHEMA_VERSION,
  SCHEMA_VERSIONS,
  SELECTION_MODES_CHECKED,
  SESSION_NS,
  STAGES as STAGE_ROWS,
  WARNING_CODES as WARNING_ROWS,
  WARMUP_DOMINANCE_MULTIPLE,
} from '../data/candidate.js';
import { valueOf } from '../data/source.js';

import { canonicalJson, digestOf } from './digest.js';
import { admits, boundFor, describeBound, freeze, pythonType, SET_NAMES } from './policy.js';

/** The closed refusal vocabulary of thirty-two names, as the source declares it. */
export const REFUSAL_CODES = Object.freeze(REFUSAL_ROWS.map((row) => valueOf(row.code)));

/** The four warnings, which are a separate channel and never soften a refusal. */
export const WARNING_CODES = Object.freeze(WARNING_ROWS.map((row) => valueOf(row.code)));

/** The five stages, in the one order the pipeline runs them. */
export const STAGES = Object.freeze(
  STAGE_ROWS.map((stage) => ({
    order: stage.order,
    id: stage.id,
    name: valueOf(stage.name),
    implementation: valueOf(stage.implementation),
  })),
);

/** The three complexity caps, which are the policy's rather than a candidate's to state. */
const CAP = Object.freeze({
  max_conditions: valueOf(CAP_ROWS.max_conditions),
  max_condition_depth: valueOf(CAP_ROWS.max_condition_depth),
  max_features: valueOf(CAP_ROWS.max_features),
});

/** The three complexity caps, keyed by the name the policy declares them under. */
export function CAPS() {
  return { ...CAP };
}

/** The three places a value may resolve from, in the order the language admits them. */
export const RESOLUTION_PLACES = Object.freeze([...valueOf(RESOLUTION_PLACE_FACTS)]);

/** The engine version a plan carries when the installed package exposes none. */
export const ENGINE_VERSION = valueOf(ENGINE_VERSION_UNKNOWN);

/** The two plan inputs this console cannot reproduce, which the view reports rather than fills in. */
export const NOT_MODELLED = Object.freeze(NOT_MODELLED_INPUTS.map((entry) => valueOf(entry)));

/** The refusal vocabulary, in the source's order, with each message its source states. */
const MESSAGES = Object.fromEntries(
  REFUSAL_ROWS.map((row) => [valueOf(row.code), row.messages.map((message) => valueOf(message))]),
);

/** The warning vocabulary with each message its source states. */
const WARNING_MESSAGES = Object.fromEntries(
  WARNING_ROWS.map((row) => [valueOf(row.code), row.messages.map((message) => valueOf(message))]),
);

/**
 * The message each rule states, named by the rule rather than by an index into the list above.
 *
 * A code that is recorded with several messages lists them in the source's own order, so
 * `field_type[0]` is the string reading and `field_type[5]` the feature alias, and every name here
 * says which reading it belongs to.
 */
const MSG = {
  not_a_mapping: MESSAGES.not_a_mapping[0],
  version_missing: MESSAGES.version_missing[0],
  version_not_a_string: MESSAGES.version_not_a_string[0],
  version_unsupported: MESSAGES.version_unsupported[0],
  version_migration_cycle: MESSAGES.version_migration_cycle[0],
  block_missing: MESSAGES.block_missing[0],
  block_not_a_mapping: MESSAGES.block_not_a_mapping[0],
  block_unknown_field: MESSAGES.block_unknown_field[0],
  field_missing: MESSAGES.field_missing[0],
  field_type_string: MESSAGES.field_type[0],
  field_type_bool: MESSAGES.field_type[1],
  field_type_number: MESSAGES.field_type[2],
  field_type_int: MESSAGES.field_type[3],
  field_type_int_value: MESSAGES.field_type[4],
  field_type_sequence: MESSAGES.field_type[5],
  field_type_entries: MESSAGES.field_type[6],
  field_type_mapping: MESSAGES.field_type[7],
  field_type_alias: MESSAGES.field_type[8],
  field_empty: MESSAGES.field_empty[0],
  field_repeated: MESSAGES.field_repeated[0],
  value_not_declared: MESSAGES.value_not_declared[0],
  value_not_declared_entries: MESSAGES.value_not_declared[1],
  bound_not_declared: MESSAGES.bound_not_declared[0],
  value_out_of_bound: MESSAGES.value_out_of_bound[0],
  feature_not_allowed: MESSAGES.feature_not_allowed[0],
  feature_not_declared: MESSAGES.feature_not_declared[0],
  feature_count_exceeded: MESSAGES.feature_count_exceeded[0],
  condition_count_exceeded: MESSAGES.condition_count_exceeded[0],
  condition_depth_exceeded: MESSAGES.condition_depth_exceeded[0],
  condition_not_resolved: MESSAGES.condition_not_resolved[0],
  condition_ambiguous: MESSAGES.condition_not_resolved[1],
  condition_empty_group: MESSAGES.condition_not_resolved[2],
  reference_unresolved: MESSAGES.reference_unresolved[0],
  risk_field_misplaced: MESSAGES.risk_field_misplaced[0],
  code_declare: MESSAGES.code_reference_refused[0],
  code_carry: MESSAGES.code_reference_refused[1],
  code_value: MESSAGES.code_reference_refused[2],
  feature_unresolved: MESSAGES.feature_unresolved[0],
  selection_uncheckable: MESSAGES.selection_incomplete[0],
  selection_exactly_one: MESSAGES.selection_incomplete[1],
  selection_requires: MESSAGES.selection_incomplete[2],
  selection_forbids: MESSAGES.selection_incomplete[3],
  exit_incomplete: MESSAGES.exit_incomplete[0],
  exit_rank: MESSAGES.exit_incomplete[1],
  rebalance_timing: MESSAGES.rebalance_timing_mismatch[0],
  rebalance_unorderable: MESSAGES.rebalance_frequency_mismatch[0],
  rebalance_finer: MESSAGES.rebalance_frequency_mismatch[1],
  sector_input: MESSAGES.sector_input_undeclared[0],
  feature_unavailable: MESSAGES.feature_unavailable_at_decision[0],
  execution_unordered: MESSAGES.undeferred_execution[0],
  execution_same_instant: MESSAGES.undeferred_execution[1],
};

/** The key names the code scan refuses by, which is a set of lowercase spellings. */
const CODE_KEYS = new Set(CODE_FIELD_NAMES.map((name) => valueOf(name)));

/** The shapes the code scan refuses by, which are the source's four patterns. */
const CODE_PATTERNS = CODE_VALUE_PATTERNS.map((pattern) => valueOf(pattern));

/** The twelve block names, in the order the source requires them. */
const BLOCK_NAMES_PLAIN = valueOf(BLOCK_NAMES);

/** The three selection modes the semantic stage can check. */
const CHECKED_MODES = valueOf(SELECTION_MODES_CHECKED);

/** The declared-set names, so `declared` can refuse a name no set carries. */
const DECLARED_SET_NAMES = SET_NAMES;

/** One session in nanoseconds, which both the signal and the rebalance interval read. */
const SESSION = valueOf(SESSION_NS);

/** The multiple a lookback is warned against, which the source declares rather than stating. */
const WARMUP_MULTIPLE = valueOf(WARMUP_DOMINANCE_MULTIPLE);

/** The four warnings a legal specification can carry, in the order they are collected. */
const WARNING_TEXT = {
  warmup: WARNING_MESSAGES.feature_warmup_dominates[0],
  unused: WARNING_MESSAGES.unused_declared_feature[0],
  positionCap: WARNING_MESSAGES.position_cap_below_fully_invested[0],
  postRun: WARNING_MESSAGES.post_run_limits_are_gate_conditions[0],
};

/** The twelve blocks, each with the question it answers and every field it declares. */
export function BLOCKS() {
  return BLOCK_ROWS.map((block) => ({
    name: valueOf(block.name),
    question: valueOf(block.question),
    reader: valueOf(block.reader),
    fields: block.fields.map(plainField),
  }));
}

/** One declared field or one risk category, as plain data. */
function plainField(entry) {
  if (entry.fields) {
    return {
      category: valueOf(entry.name),
      fields: entry.fields.map(plainField),
    };
  }
  return {
    name: valueOf(entry.name),
    optional: Boolean(entry.optional),
    reads: entry.reads,
    declaration: entry.declaration,
  };
}

/** The risk categories as plain data: the category's name and the fields it may state. */
const RISK_CATEGORIES = BLOCK_ROWS.filter((row) => valueOf(row.name) === 'risk')
  .flatMap((row) => row.fields)
  .map((category) => ({
    name: valueOf(category.name),
    fields: category.fields.map(plainField),
  }));

/** Every declared field with its dotted path, so a path can be resolved to the rule that reads it. */
function flatFields() {
  const flat = [];
  for (const block of BLOCKS()) {
    for (const entry of block.fields) {
      if (entry.fields) {
        for (const inner of entry.fields) {
          flat.push({ ...inner, block: block.name, path: `${block.name}.${entry.category}.${inner.name}` });
        }
      } else {
        flat.push({ ...entry, block: block.name, path: `${block.name}.${entry.name}` });
      }
    }
  }
  return flat;
}

/** The declared field a dotted path names, or null when no block declares it. */
export function fieldAt(path) {
  if (typeof path !== 'string') throw new TypeError(`a field path must be a string, was ${pythonType(path)}`);
  return flatFields().find((entry) => entry.path === path) ?? null;
}

/**
 * One value the schema admits for a field: the first member of the declared set the field names, or
 * the lower end of the bound that governs it, or the bound's first choice.
 *
 * A control that resolves a value should open on a value the policy already covers, so a refusal on
 * screen is one the reader caused rather than the state the screen starts in. The value follows the
 * policy: a set the caller reorders or empties moves the value with it, and a bound whose lower end
 * is outside its own range is the bound's defect, not this function's to correct.
 */
export function admittedValue(policy, path) {
  const entry = fieldAt(path);
  if (entry === null) throw new TypeError(`no block declares the field ${path}`);
  const name = valueOf(entry.declaration ?? '');
  const members = policy?.sets?.[name];
  if (Array.isArray(members) && members.length > 0 && entry.reads.startsWith('declared')) {
    return entry.reads === 'declared_list' ? [members[0]] : members[0];
  }
  // A bounded field names its bound in the declaration, and that name is not always the field's own
  // path: `features.lookback` is governed by `feature.lookback`, `exit.threshold` by `condition.value`.
  const bound = boundFor(policy, name) ?? boundFor(policy, path);
  if (bound === null) return entry.reads === 'declared_list' ? [] : '';
  if (bound.kind === 'choice') return bound.choices[0];
  return bound.lower;
}

/** An empty candidate: the twelve blocks with nothing declared, ready to be built block by block. */
export function emptySpecification() {
  return {
    version: valueOf(SCHEMA_VERSION),
    identity: { family: '', hypothesis: '', mechanism: '', falsification: '' },
    universe: { asset_classes: [], membership_policy: '', source: '' },
    data: {
      signal_frequency: '',
      decision_time: '',
      execution_eligibility: '',
      corporate_action_treatment: '',
    },
    features: { declared: {} },
    selection: { mode: '' },
    entry: { direction: '' },
    exit: {},
    portfolio: { weighting: '', cash_policy: '' },
    rebalance: { frequency: '', timing: '' },
    risk: { pre_trade: {}, runtime: {}, post_run: {} },
    evaluation: {},
    provenance: { generator: '', seed: 0 },
  };
}

/* -------------------------------------------------------------------------------------------------
 * The schema stage: the code scan, the version, the blocks and the readers.
 * ---------------------------------------------------------------------------------------------- */

/** A refusal, as the console reports one: a value rather than an exception. */
function refusal(code, path, message, values = {}) {
  const filled = { ...values };
  if (values.type !== undefined) {
    // The source's messages print a Python type name through four spellings, so one `type` here
    // fills whichever of them the message states.
    for (const spelling of [
      'type(payload).__name__',
      'type(value).__name__',
      'type(item).__name__',
      'type(alias).__name__',
      'type(key).__name__',
    ]) {
      filled[spelling] = values.type;
    }
  }
  return { ok: false, code, path, reason: fill(message, filled) };
}

/** Fill a message's `{placeholder}`s with the values a rule states, leaving unknown ones in place. */
function fill(template, values) {
  return String(template).replace(/\{([^}]+)\}/g, (match, key) =>
    key in values ? String(values[key]) : match,
  );
}

/** True when a value is a plain mapping, which is what every block and every payload must be. */
function isPlainMapping(value) {
  return (
    value !== null &&
    typeof value === 'object' &&
    !Array.isArray(value) &&
    !(value instanceof Date) &&
    Object.getPrototypeOf(value) === Object.prototype
  );
}

/** `decimal_of`: an int, a float or a numeric string, and never a bool. */
function decimalOf(value) {
  if (typeof value === 'boolean' || value === null || value === undefined) return null;
  if (typeof value === 'number') return Number.isFinite(value) ? value : null;
  if (typeof value === 'string') {
    if (value.trim() === '') return null;
    const number = Number(value);
    return Number.isFinite(number) ? number : null;
  }
  return null;
}

/** One block's reader: the payload, the policy, the path, the refusals and the keys it has read. */
function makeReader(policy, payload, path, refusals) {
  const reader = { policy, payload: {}, path, refusals, read: new Set() };
  if (!isPlainMapping(payload)) {
    refusals.push(refusal('block_not_a_mapping', path, MSG.block_not_a_mapping, { type: pythonType(payload) }));
    return reader;
  }
  reader.payload = payload;
  return reader;
}

/** Read a field, marking it read so `finish` does not refuse it as undeclared. */
function take(reader, name) {
  reader.read.add(name);
  const present = Object.prototype.hasOwnProperty.call(reader.payload, name);
  return [present, reader.payload[name]];
}

/** The path a field is reported at, which is the block's own path when it has one. */
function fieldPath(reader, name) {
  return reader.path ? `${reader.path}.${name}` : name;
}

/** Refuse every key the block did not read, so the allowed set cannot drift from the reading. */
function finish(reader) {
  for (const key of Object.keys(reader.payload)) {
    if (!reader.read.has(key)) {
      reader.refusals.push(
        refusal('block_unknown_field', fieldPath(reader, key), MSG.block_unknown_field, {
          path: reader.path,
          key,
        }),
      );
    }
  }
}

/** Read a non-empty text field. */
function text(reader, name, required = true) {
  const [present, value] = take(reader, name);
  if (!present || value === null || value === undefined) {
    if (required) {
      reader.refusals.push(refusal('field_missing', fieldPath(reader, name), MSG.field_missing, { name }));
    }
    return null;
  }
  if (typeof value !== 'string') {
    reader.refusals.push(
      refusal('field_type', fieldPath(reader, name), MSG.field_type_string, { name, type: pythonType(value) }),
    );
    return null;
  }
  if (value.trim() === '') {
    reader.refusals.push(refusal('field_empty', fieldPath(reader, name), MSG.field_empty, { name }));
    return null;
  }
  return value;
}

/** Read a boolean field, returning the default when it is absent. */
function boolean(reader, name, fallback = null) {
  const [present, value] = take(reader, name);
  if (!present || value === null || value === undefined) {
    if (!present && fallback === null) {
      reader.refusals.push(refusal('field_missing', fieldPath(reader, name), MSG.field_missing, { name }));
    }
    return fallback;
  }
  if (typeof value !== 'boolean') {
    reader.refusals.push(
      refusal('field_type', fieldPath(reader, name), MSG.field_type_bool, { name, type: pythonType(value) }),
    );
    return fallback;
  }
  return value;
}

/** Read an integer field, which a bool is not. */
function integer(reader, name, required = true) {
  const [present, value] = take(reader, name);
  if (!present || value === null || value === undefined) {
    if (required) {
      reader.refusals.push(refusal('field_missing', fieldPath(reader, name), MSG.field_missing, { name }));
    }
    return null;
  }
  if (typeof value !== 'number' || !Number.isInteger(value)) {
    reader.refusals.push(
      refusal('field_type', fieldPath(reader, name), MSG.field_type_int, { name, type: pythonType(value) }),
    );
    return null;
  }
  return value;
}

/** The bound the policy declares under a name, refusing the field when it declares none. */
function bindingOf(reader, boundName) {
  const bound = boundFor(reader.policy, boundName);
  if (bound === null) {
    reader.refusals.push(
      refusal('bound_not_declared', reader.path, MSG.bound_not_declared, {
        bound_name: boundName,
        path: reader.path,
      }),
    );
    return null;
  }
  return bound;
}

/** Read a decimal field and check it against the policy's bound of that name. */
function bounded(reader, name, boundName, required = true) {
  const [present, value] = take(reader, name);
  if (!present || value === null || value === undefined) {
    if (required) {
      reader.refusals.push(refusal('field_missing', fieldPath(reader, name), MSG.field_missing, { name }));
    }
    return null;
  }
  const number = decimalOf(value);
  if (number === null) {
    reader.refusals.push(
      refusal('field_type', fieldPath(reader, name), MSG.field_type_number, { name, type: pythonType(value) }),
    );
    return null;
  }
  const bound = bindingOf(reader, boundName);
  if (bound === null) return null;
  if (!admits(bound, number)) {
    reader.refusals.push(
      refusal('value_out_of_bound', fieldPath(reader, name), MSG.value_out_of_bound, {
        name,
        number,
        'bound.describe()': describeBound(bound),
        bound: describeBound(bound),
      }),
    );
    return null;
  }
  return number;
}

/** Read an integer field and check it against the policy's bound of that name. */
function boundedInt(reader, name, boundName, required = true) {
  const number = bounded(reader, name, boundName, required);
  if (number === null) return null;
  if (!Number.isInteger(number)) {
    reader.refusals.push(
      refusal('field_type', fieldPath(reader, name), MSG.field_type_int_value, {
        name,
        number,
        type: pythonType(number),
      }),
    );
    return null;
  }
  return number;
}

/** The members of a declared set, which every name a field states must belong to. */
function declaredSetOf(reader, setName) {
  if (!DECLARED_SET_NAMES.includes(setName)) {
    throw new TypeError(`${setName} is not a declared set of this policy`);
  }
  return reader.policy.sets?.[setName] ?? [];
}

/** Read a field whose value is one of the policy's declared names. */
function declared(reader, name, setName, required = true) {
  const value = text(reader, name, required);
  if (value === null) return null;
  const members = declaredSetOf(reader, setName);
  if (!members.includes(value)) {
    reader.refusals.push(
      refusal('value_not_declared', fieldPath(reader, name), MSG.value_not_declared, {
        name,
        value,
        'sorted set': [...members].sort().join(', '),
      }),
    );
    return null;
  }
  return value;
}

/** Read a sequence whose every element is one of the policy's declared names. */
function declaredList(reader, name, setName, required = true) {
  const values = identifiers(reader, name, required);
  const members = declaredSetOf(reader, setName);
  const admitted = [];
  for (const value of values) {
    if (!members.includes(value)) {
      reader.refusals.push(
        refusal('value_not_declared', fieldPath(reader, name), MSG.value_not_declared_entries, {
          name,
          value,
          'sorted set': [...members].sort().join(', '),
        }),
      );
      continue;
    }
    admitted.push(value);
  }
  return admitted;
}

/** Read a sequence of non-empty, distinct identifiers. */
function identifiers(reader, name, required = false) {
  const [present, value] = take(reader, name);
  if (!present || value === null || value === undefined) {
    if (required) {
      reader.refusals.push(refusal('field_missing', fieldPath(reader, name), MSG.field_missing, { name }));
    }
    return [];
  }
  if (typeof value === 'string' || !Array.isArray(value)) {
    reader.refusals.push(
      refusal('field_type', fieldPath(reader, name), MSG.field_type_sequence, {
        name,
        type: pythonType(value),
      }),
    );
    return [];
  }
  const seen = [];
  value.forEach((item, index) => {
    const itemPath = `${fieldPath(reader, name)}.${index}`;
    if (typeof item !== 'string' || item.trim() === '') {
      reader.refusals.push(
        refusal('field_type', itemPath, MSG.field_type_entries, { name, type: pythonType(item) }),
      );
      return;
    }
    if (seen.includes(item)) {
      reader.refusals.push(refusal('field_repeated', itemPath, MSG.field_repeated, { name, item }));
      return;
    }
    seen.push(item);
  });
  return seen;
}

/** Read a field that must name one of the features the specification declares. */
function featureRef(reader, name, aliases, required = true) {
  const value = text(reader, name, required);
  if (value === null) return null;
  if (!aliases.includes(value)) {
    reader.refusals.push(
      refusal('feature_not_declared', fieldPath(reader, name), MSG.feature_not_declared, { name, value }),
    );
    return null;
  }
  return value;
}

/** Read a field that must name one of the policy's frozen references. */
function reference(reader, name, required = true) {
  const value = text(reader, name, required);
  if (value === null) return null;
  const held = reader.policy.references ?? [];
  if (!held.some((entry) => entry.name === value)) {
    const declared = held.map((entry) => entry.name).join(', ');
    reader.refusals.push(
      refusal('reference_unresolved', fieldPath(reader, name), MSG.reference_unresolved, { name, value }),
    );
    return null;
  }
  return value;
}

/** Refuse every place in the payload that names code rather than data, at every depth. */
export function scanForCode(payload, path = '') {
  const refusals = [];
  if (isPlainMapping(payload)) {
    for (const [key, value] of Object.entries(payload)) {
      if (CODE_KEYS.has(key.toLowerCase())) {
        refusals.push(
          refusal('code_reference_refused', path ? `${path}.${key}` : key, MSG.code_declare, { key }),
        );
        continue;
      }
      refusals.push(...scanForCode(value, path ? `${path}.${key}` : key));
    }
    return refusals;
  }
  if (Array.isArray(payload)) {
    payload.forEach((item, index) => refusals.push(...scanForCode(item, `${path}.${index}`)));
    return refusals;
  }
  if (typeof payload === 'function' || (typeof payload === 'object' && payload !== null)) {
    refusals.push(refusal('code_reference_refused', path, MSG.code_carry, { type: pythonType(payload) }));
    return refusals;
  }
  if (typeof payload === 'string') {
    for (const pattern of CODE_PATTERNS) {
      if (new RegExp(pattern).test(payload)) {
        refusals.push(
          refusal('code_reference_refused', path, MSG.code_value, { 'payload!r': JSON.stringify(payload) }),
        );
        break;
      }
    }
  }
  return refusals;
}

/** Apply the registered migrations until the version is one this build accepts. */
export function migrate(payload, migrations = null) {
  const table = migrations === null ? valueOf(MIGRATIONS) : migrations;
  const applied = [];
  for (let hop = 0; hop < valueOf(MAX_MIGRATIONS); hop++) {
    const version = payload.version;
    if (valueOf(SCHEMA_VERSIONS).includes(version)) return { payload, applied, refusal: null };
    if (typeof version !== 'string' || version === '') {
      const code = version === null || version === undefined ? 'version_missing' : 'version_not_a_string';
      return {
        payload,
        applied,
        refusal: refusal(code, 'version', MESSAGES[code][0], {
          versions: valueOf(SCHEMA_VERSIONS).join(', '),
        }),
      };
    }
    const migration = table[version];
    if (typeof migration !== 'function') {
      return {
        payload,
        applied,
        refusal: refusal('version_unsupported', 'version', MSG.version_unsupported, {
          version,
          versions: valueOf(SCHEMA_VERSIONS).join(', '),
        }),
      };
    }
    payload = migration(payload);
    applied.push(version);
  }
  return {
    payload,
    applied,
    refusal: refusal('version_migration_cycle', 'version', MSG.version_migration_cycle, {
      version: payload.version,
    }),
  };
}

/** The features block, read first because the later blocks refer to the aliases it declares. */
function readFeatures(reader) {
  const [present, payload] = take(reader, 'declared');
  if (!present || payload === null || payload === undefined) {
    reader.refusals.push(
      refusal('field_missing', fieldPath(reader, 'declared'), MSG.field_missing, { name: 'declared' }),
    );
    return { declarations: {} };
  }
  if (!isPlainMapping(payload)) {
    reader.refusals.push(
      refusal('field_type', fieldPath(reader, 'declared'), MSG.field_type_mapping, { type: pythonType(payload) }),
    );
    return { declarations: {} };
  }

  const limit = CAP.max_features;
  const count = Object.keys(payload).length;
  if (count > limit) {
    reader.refusals.push(
      refusal('feature_count_exceeded', fieldPath(reader, 'declared'), MSG.feature_count_exceeded, {
        n: count,
        limit,
      }),
    );
  }

  const declarations = {};
  for (const [alias, body] of Object.entries(payload)) {
    const aliasPath = `${fieldPath(reader, 'declared')}.${alias}`;
    if (typeof alias !== 'string' || alias.trim() === '') {
      reader.refusals.push(refusal('field_type', aliasPath, MSG.field_type_alias, { type: pythonType(alias) }));
      continue;
    }
    const entry = makeReader(reader.policy, isPlainMapping(body) ? body : {}, aliasPath, reader.refusals);
    let featureId = text(entry, 'feature');
    if (featureId !== null && !(reader.policy.featureAllowlist ?? []).includes(featureId)) {
      reader.refusals.push(
        refusal('feature_not_allowed', `${aliasPath}.feature`, MSG.feature_not_allowed, {
          feature_id: featureId,
        }),
      );
      featureId = null;
    }
    const lookback = boundedInt(entry, 'lookback', 'feature.lookback');
    const skip = boundedInt(entry, 'skip', 'feature.skip', false);
    finish(entry);
    declarations[alias] = { feature: featureId ?? '', lookback: lookback ?? 0, skip: skip ?? 0 };
  }
  return { declarations };
}

/** The identity block: what the candidate claims. */
function readIdentity(reader) {
  return {
    family: declared(reader, 'family', 'families') ?? '',
    hypothesis: text(reader, 'hypothesis') ?? '',
    mechanism: text(reader, 'mechanism') ?? '',
    falsification: text(reader, 'falsification') ?? '',
  };
}

/** The universe block: which instruments may be held, and the floors that decide membership. */
function readUniverse(reader) {
  return {
    asset_classes: declaredList(reader, 'asset_classes', 'asset_classes'),
    membership_policy: declared(reader, 'membership_policy', 'membership_policies') ?? '',
    source: text(reader, 'source') ?? '',
    min_price: bounded(reader, 'min_price', 'universe.min_price', false),
    min_dollar_volume: bounded(reader, 'min_dollar_volume', 'universe.min_dollar_volume', false),
    exclusions: identifiers(reader, 'exclusions'),
  };
}

/** The data block: when it decides, when its intent may fill, how actions are treated. */
function readData(reader) {
  return {
    signal_frequency: declared(reader, 'signal_frequency', 'signal_frequencies') ?? '',
    decision_time: declared(reader, 'decision_time', 'decision_times') ?? '',
    execution_eligibility: declared(reader, 'execution_eligibility', 'execution_eligibilities') ?? '',
    corporate_action_treatment:
      declared(reader, 'corporate_action_treatment', 'corporate_action_treatments') ?? '',
  };
}

/** The selection block: how the eligible set is chosen, and how a tie is broken. */
function readSelection(reader, aliases) {
  return {
    mode: declared(reader, 'mode', 'selection_modes') ?? '',
    feature: featureRef(reader, 'feature', aliases, false),
    percentile: bounded(reader, 'percentile', 'selection.percentile', false),
    count: bounded(reader, 'count', 'selection.count', false),
    threshold: bounded(reader, 'threshold', 'selection.threshold', false),
    comparison: declared(reader, 'comparison', 'operators', false),
    tie_break: declared(reader, 'tie_break', 'tie_breaks', false),
  };
}

/** The depth of a condition tree, counted from 1 at the top. */
function descend(walk, path) {
  return { ...walk, path, depth: walk.depth + 1 };
}

/** The number of leaf comparisons in a condition tree. */
export function countConditionLeaves(node) {
  if (node === null || node === undefined) return 0;
  if (Array.isArray(node.all)) return node.all.reduce((total, child) => total + countConditionLeaves(child), 0);
  if (Array.isArray(node.any)) return node.any.reduce((total, child) => total + countConditionLeaves(child), 0);
  return 1;
}

/**
 * The depth a condition tree reaches, counted from 1 at the top.
 *
 * `_ConditionWalk.descend` is the source of the rule: the top of a tree is level 1 and a child of a
 * group is one deeper, so this reports the level the walk would reach rather than the height of any
 * particular branch.
 */
export function conditionDepth(node) {
  if (node === null || node === undefined) return 0;
  const children = Array.isArray(node.all) ? node.all : Array.isArray(node.any) ? node.any : [];
  if (children.length === 0) return 1;
  return 1 + Math.max(...children.map(conditionDepth));
}

/** Read one condition node, refusing a tree that nests deeper than the declared limit. */
function readCondition(payload, walk) {
  if (walk.depth > CAP.max_condition_depth) {
    walk.reader.refusals.push(
      refusal('condition_depth_exceeded', walk.path, MSG.condition_depth_exceeded, {
        depth: walk.depth,
      }),
    );
    return null;
  }
  if (!isPlainMapping(payload)) {
    walk.reader.refusals.push(
      refusal('condition_not_resolved', walk.path, MSG.condition_not_resolved, {
        type: pythonType(payload),
      }),
    );
    return null;
  }
  const groups = ['all', 'any'].filter((group) => group in payload);
  if (groups.length > 0) return readConditionGroup(payload, groups, walk);
  return readConditionLeaf(payload, walk);
}

/** Read a condition group, which states `all` or `any` and holds at least one condition. */
function readConditionGroup(payload, groups, walk) {
  for (const key of Object.keys(payload)) {
    if (key !== 'all' && key !== 'any') {
      walk.reader.refusals.push(
        refusal('block_unknown_field', `${walk.path}.${key}`, MSG.block_unknown_field, {
          path: walk.path,
          key,
        }),
      );
    }
  }
  if (groups.length > 1) {
    walk.reader.refusals.push(
      refusal('condition_not_resolved', walk.path, MSG.condition_ambiguous),
    );
    return null;
  }
  const group = groups[0];
  const body = payload[group];
  if (!Array.isArray(body) || body.length === 0) {
    walk.reader.refusals.push(
      refusal('condition_not_resolved', `${walk.path}.${group}`, MSG.condition_empty_group, { group }),
    );
    return null;
  }
  const children = [];
  body.forEach((item, index) => {
    const child = readCondition(item, descend(walk, `${walk.path}.${group}.${index}`));
    if (child !== null) children.push(child);
  });
  return { [group]: children };
}

/** Read one condition leaf, which is exactly `{feature, comparison, value}`. */
function readConditionLeaf(payload, walk) {
  const leaf = makeReader(walk.reader.policy, payload, walk.path, walk.reader.refusals);
  const feature = featureRef(leaf, 'feature', walk.aliases);
  const comparison = declared(leaf, 'comparison', 'operators');
  const value = bounded(leaf, 'value', 'condition.value');
  finish(leaf);
  if (feature === null || comparison === null || value === null) return null;
  return { feature, comparison, value };
}

/** The entry block: whether and when it enters, capped in leaves and in depth. */
function readEntry(reader, aliases) {
  const direction = declared(reader, 'direction', 'entry_directions') ?? '';
  const [present, payload] = take(reader, 'conditions');
  if (!present || payload === null || payload === undefined) return { direction, conditions: null };

  const path = fieldPath(reader, 'conditions');
  const conditions = readCondition(payload, { reader, aliases, path, depth: 1 });
  if (conditions === null) return { direction, conditions: null };

  const leaves = countConditionLeaves(conditions);
  if (leaves > CAP.max_conditions) {
    reader.refusals.push(
      refusal('condition_count_exceeded', path, MSG.condition_count_exceeded, {
        leaves,
        limit: CAP.max_conditions,
      }),
    );
    return { direction, conditions: null };
  }
  return { direction, conditions };
}

/** The exit block: the declared ways a position leaves the portfolio. */
function readExit(reader, aliases) {
  return {
    holding_period_cap: boundedInt(reader, 'holding_period_cap', 'exit.holding_period_cap', false),
    rank_exit_count: boundedInt(reader, 'rank_exit_count', 'exit.rank_exit_count', false),
    feature: featureRef(reader, 'feature', aliases, false),
    comparison: declared(reader, 'comparison', 'operators', false),
    threshold: bounded(reader, 'threshold', 'condition.value', false),
    stop_fraction: bounded(reader, 'stop_fraction', 'exit.stop_fraction', false),
    target_fraction: bounded(reader, 'target_fraction', 'exit.target_fraction', false),
    flatten_ineligible: boolean(reader, 'flatten_ineligible', true) ?? false,
  };
}

/** The portfolio block: how much it holds, in how many positions, under which caps. */
function readPortfolio(reader) {
  return {
    position_count: boundedInt(reader, 'position_count', 'portfolio.position_count') ?? 0,
    weighting: declared(reader, 'weighting', 'weighting_methods') ?? '',
    max_position_weight: bounded(reader, 'max_position_weight', 'portfolio.max_position_weight') ?? 0,
    max_sector_weight: bounded(reader, 'max_sector_weight', 'portfolio.max_sector_weight', false),
    cash_policy: declared(reader, 'cash_policy', 'cash_policies') ?? '',
  };
}

/** The rebalance block: how often and when. */
function readRebalance(reader) {
  return {
    frequency: declared(reader, 'frequency', 'rebalance_frequencies') ?? '',
    timing: declared(reader, 'timing', 'rebalance_timings') ?? '',
  };
}

/** Refuse every risk field placed under a category that does not control it. */
function refuseMisplacedRiskFields(reader, categories) {
  for (const category of categories) {
    const payload = take(reader, category.name)[1];
    if (!isPlainMapping(payload)) continue;
    for (const name of Object.keys(payload)) {
      if (category.fields.some((entry) => entry.name === name)) continue;
      for (const other of categories) {
        if (other.name === category.name) continue;
        if (!other.fields.some((entry) => entry.name === name)) continue;
        reader.refusals.push(
          refusal('risk_field_misplaced', `${fieldPath(reader, category.name)}.${name}`, MSG.risk_field_misplaced, {
            name,
            other: other.name,
            'other-dashed': other.name.replace('_', '-'),
          }),
        );
      }
    }
  }
}

/** The risk block: its three categories, each declaring exactly its own fields. */
function readRisk(reader, categories) {
  refuseMisplacedRiskFields(reader, categories);

  const blocks = {};
  for (const category of categories) {
    const payload = take(reader, category.name)[1];
    const nested = makeReader(
      reader.policy,
      isPlainMapping(payload) ? payload : {},
      fieldPath(reader, category.name),
      reader.refusals,
    );
    const parsed = {};
    for (const entry of category.fields) {
      parsed[entry.name] = readRiskField(nested, entry);
    }
    finish(nested);
    blocks[category.name] = parsed;
  }
  return { pre_trade: blocks.pre_trade, runtime: blocks.runtime, post_run: blocks.post_run };
}

/** One risk field, read the way the field's own category reads it. */
function readRiskField(reader, entry) {
  if (entry.reads === 'boolean') return boolean(reader, entry.name, false) ?? false;
  if (entry.reads === 'bounded_int') return boundedInt(reader, entry.name, entry.declaration, false);
  return bounded(reader, entry.name, entry.declaration, false);
}

/** The evaluation block: which frozen objects the candidate is judged against. */
function readEvaluation(reader) {
  return {
    benchmark: reference(reader, 'benchmark') ?? '',
    cost_model: reference(reader, 'cost_model') ?? '',
    validation_policy: reference(reader, 'validation_policy') ?? '',
  };
}

/** The provenance block: where the candidate came from. */
function readProvenance(reader) {
  return {
    generator: text(reader, 'generator') ?? '',
    seed: integer(reader, 'seed') ?? 0,
    parents: identifiers(reader, 'parents'),
  };
}

/** Read one block through its own reader, which finishes it so an undeclared key is refused. */
function readBlock(payload, name, policy, refusals, read) {
  const reader = makeReader(policy, payload[name], name, refusals);
  const parsed = read(reader);
  finish(reader);
  return parsed;
}

/** The schema stage: the scan, the version, the twelve blocks, and the specification they build. */
export function parseSpecification(payload, policy, options = {}) {
  if (!isPlainMapping(payload)) {
    return {
      specification: null,
      refusals: [refusal('not_a_mapping', '', MSG.not_a_mapping, { type: pythonType(payload) })],
      migrations: [],
    };
  }

  const scanned = scanForCode(payload);
  if (scanned.length > 0) return { specification: null, refusals: scanned, migrations: [] };

  const migrated = migrate(payload, options.migrations ?? null);
  if (migrated.refusal !== null) {
    return { specification: null, refusals: [migrated.refusal], migrations: migrated.applied };
  }

  const refusals = [];
  const reader = makeReader(policy, migrated.payload, '', refusals);
  for (const name of BLOCK_NAMES_PLAIN) {
    const [present, value] = take(reader, name);
    if (!present || value === null || value === undefined) {
      refusals.push(refusal('block_missing', name, MSG.block_missing, { name }));
    }
  }
  const version = text(reader, 'version');
  finish(reader);
  if (refusals.length > 0) {
    return { specification: null, refusals, migrations: migrated.applied };
  }

  const source = migrated.payload;
  const features = readBlock(source, 'features', policy, refusals, readFeatures);
  const aliases = Object.keys(features.declarations);
  const identity = readBlock(source, 'identity', policy, refusals, readIdentity);
  const universe = readBlock(source, 'universe', policy, refusals, readUniverse);
  const data = readBlock(source, 'data', policy, refusals, readData);
  const selection = readBlock(source, 'selection', policy, refusals, (block) => readSelection(block, aliases));
  const entry = readBlock(source, 'entry', policy, refusals, (block) => readEntry(block, aliases));
  const exit = readBlock(source, 'exit', policy, refusals, (block) => readExit(block, aliases));
  const portfolio = readBlock(source, 'portfolio', policy, refusals, readPortfolio);
  const rebalance = readBlock(source, 'rebalance', policy, refusals, readRebalance);
  const risk = readBlock(source, 'risk', policy, refusals, (block) => readRisk(block, RISK_CATEGORIES));
  const evaluation = readBlock(source, 'evaluation', policy, refusals, readEvaluation);
  const provenance = readBlock(source, 'provenance', policy, refusals, readProvenance);

  if (refusals.length > 0) {
    return { specification: null, refusals, migrations: migrated.applied };
  }

  return {
    specification: {
      version: version ?? valueOf(SCHEMA_VERSION),
      identity,
      universe,
      data,
      features,
      selection,
      entry,
      exit,
      portfolio,
      rebalance,
      risk,
      evaluation,
      provenance,
    },
    refusals: [],
    migrations: migrated.applied,
  };
}

/* -------------------------------------------------------------------------------------------------
 * Stages two to five: resolution, semantics, times and compilation.
 * ---------------------------------------------------------------------------------------------- */

/** The one instance order the temporal stage reads, unwrapped once. */
const INSTANT_ORDER = valueOf(EXECUTION_INSTANT_ORDER);

/** The interval of each declared frequency in sessions, unwrapped once. */
const FREQUENCIES = valueOf(FREQUENCY_SESSIONS);

/** Stage 2: resolve every declared alias against the registry the caller holds. */
export function resolveFeatures(specification, registry = {}) {
  const refusals = [];
  const definitions = [];
  for (const [alias, declaration] of Object.entries(specification.features.declarations)) {
    const definition = registry === null || registry === undefined ? null : registry[declaration.feature];
    if (definition === null || definition === undefined) {
      refusals.push(
        refusal('feature_unresolved', `features.declared.${alias}.feature`, MSG.feature_unresolved, {
          feature_id: declaration.feature,
          alias,
        }),
      );
      continue;
    }
    definitions.push({ alias, feature: declaration.feature, ...definition });
  }
  return { definitions, refusals };
}

/** Stage 3: refuse every interaction between blocks that is resolved by declaration. */
export function semanticRefusals(specification, policy) {
  return [
    ...selectionRefusals(specification.selection),
    ...exitRefusals(specification.exit, specification.selection.mode),
    ...portfolioRefusals(specification.portfolio, policy),
    ...rebalanceRefusals(specification.data, specification.rebalance),
  ];
}

/** The selection rules of the mode the block declares, which is checked rather than skipped. */
export function selectionRefusals(selection) {
  const path = valueOf(REFUSAL_PATHS.selection);
  if (selection.mode === CHECKED_MODES[0]) return rankSelectionRefusals(selection, path);
  if (selection.mode === CHECKED_MODES[1]) return thresholdSelectionRefusals(selection, path);
  if (selection.mode === CHECKED_MODES[2]) return noneSelectionRefusals(selection, path);
  return [
    refusal('selection_incomplete', path, MSG.selection_uncheckable, {
      mode: selection.mode,
      modes: CHECKED_MODES.join(', '),
    }),
  ];
}

/** A field the subject requires and does not state. */
function requiredRefusals(subject, fields, path) {
  return Object.entries(fields)
    .filter(([, value]) => value === null || value === undefined)
    .map(([name]) => refusal('selection_incomplete', path, MSG.selection_requires, { subject, name }));
}

/** A field the subject states and must not. */
function forbiddenRefusals(subject, fields, path) {
  return Object.entries(fields)
    .filter(([, value]) => value !== null && value !== undefined)
    .map(([name]) => refusal('selection_incomplete', path, MSG.selection_forbids, { subject, name }));
}

/** Rank selection: exactly one of percentile and count, plus its feature and its tie-break. */
function rankSelectionRefusals(selection, path) {
  const refusals = [];
  if ((selection.percentile === null) === (selection.count === null)) {
    refusals.push(refusal('selection_incomplete', path, MSG.selection_exactly_one));
  }
  refusals.push(
    ...requiredRefusals(valueOf(RULE_SUBJECTS.rank), { feature: selection.feature, tie_break: selection.tie_break }, path),
  );
  return refusals;
}

/** Threshold selection: its comparison and its threshold, and no ranking field at all. */
function thresholdSelectionRefusals(selection, path) {
  const subject = valueOf(RULE_SUBJECTS.threshold);
  return [
    ...requiredRefusals(
      subject,
      { feature: selection.feature, comparison: selection.comparison, threshold: selection.threshold },
      path,
    ),
    ...forbiddenRefusals(
      subject,
      { percentile: selection.percentile, count: selection.count, tie_break: selection.tie_break },
      path,
    ),
  ];
}

/** None selection: every ranking field it could state is refused. */
function noneSelectionRefusals(selection, path) {
  return forbiddenRefusals(
    valueOf(RULE_SUBJECTS.none),
    {
      feature: selection.feature,
      percentile: selection.percentile,
      count: selection.count,
      threshold: selection.threshold,
      comparison: selection.comparison,
      tie_break: selection.tie_break,
    },
    path,
  );
}

/** The exit block: a feature with both comparison and threshold, or with neither. */
export function exitRefusals(exit, selectionMode) {
  const refusals = [];
  const complete = exit.comparison !== null && exit.threshold !== null;
  if ((exit.feature !== null) !== complete) {
    refusals.push(refusal('exit_incomplete', valueOf(REFUSAL_PATHS.exit), MSG.exit_incomplete));
  }
  if (exit.rank_exit_count !== null && selectionMode !== CHECKED_MODES[0]) {
    refusals.push(
      refusal('exit_incomplete', valueOf(REFUSAL_PATHS.rankExit), MSG.exit_rank, {
        mode: selectionMode,
      }),
    );
  }
  return refusals;
}

/** The portfolio block: a sector cap needs a sector map to be applied through. */
export function portfolioRefusals(portfolio, policy) {
  const hasSectors = (policy?.sectors ?? []).length > 0;
  if (portfolio.max_sector_weight !== null && !hasSectors) {
    return [refusal('sector_input_undeclared', valueOf(REFUSAL_PATHS.sectorWeight), MSG.sector_input)];
  }
  return [];
}

/** The rebalance block: its timing, and then its frequency, against the declared signal. */
export function rebalanceRefusals(data, rebalance) {
  const refusals = [];
  if (rebalance.timing !== data.execution_eligibility) {
    refusals.push(
      refusal('rebalance_timing_mismatch', valueOf(REFUSAL_PATHS.timing), MSG.rebalance_timing, {
        timing: rebalance.timing,
        eligibility: data.execution_eligibility,
      }),
    );
  }
  const signalSessions = FREQUENCIES[data.signal_frequency];
  const rebalanceSessions = FREQUENCIES[rebalance.frequency];
  if (signalSessions === undefined || rebalanceSessions === undefined) {
    refusals.push(
      refusal('rebalance_frequency_mismatch', valueOf(REFUSAL_PATHS.frequency), MSG.rebalance_unorderable, {
        freq: rebalance.frequency,
        signal: data.signal_frequency,
        frequencies: Object.keys(FREQUENCIES).join(', '),
      }),
    );
  } else if (rebalanceSessions < signalSessions) {
    refusals.push(
      refusal('rebalance_frequency_mismatch', valueOf(REFUSAL_PATHS.frequency), MSG.rebalance_finer, {
        freq: rebalance.frequency,
        signal: data.signal_frequency,
      }),
    );
  }
  return refusals;
}

/** The fill instant must be strictly later than the decision instant, and both must be ordered. */
export function undeferredExecutionRefusal(data) {
  const ordered = Object.keys(INSTANT_ORDER).join(', ');
  const decision = INSTANT_ORDER[data.decision_time];
  const eligibility = INSTANT_ORDER[data.execution_eligibility];
  if (decision === undefined) {
    return refusal('undeferred_execution', valueOf(REFUSAL_PATHS.decisionTime), MSG.execution_unordered, {
      order: ordered,
      instant: data.decision_time,
    });
  }
  if (eligibility === undefined) {
    return refusal('undeferred_execution', valueOf(REFUSAL_PATHS.eligibility), MSG.execution_unordered, {
      order: ordered,
      instant: data.execution_eligibility,
    });
  }
  if (eligibility <= decision) {
    return refusal('undeferred_execution', valueOf(REFUSAL_PATHS.eligibility), MSG.execution_same_instant, {
      eligibility: data.execution_eligibility,
      decision: data.decision_time,
    });
  }
  return null;
}

/** Every declared feature must be knowable at the decision instant of the declared signal. */
export function unavailableFeatureRefusals(specification, definitions) {
  const frequency = specification.data.signal_frequency;
  const intervalNs = FREQUENCIES[frequency] * SESSION;
  const refusals = [];
  for (const definition of definitions) {
    const lag = definition.availability_lag_ns ?? 0;
    if (lag < intervalNs) continue;
    refusals.push(
      refusal('feature_unavailable_at_decision', `features.declared.${definition.alias}.feature`, MSG.feature_unavailable, {
        feature_id: definition.feature,
        lag,
        frequency,
        interval: intervalNs,
        interval_ns: intervalNs,
      }),
    );
  }
  return refusals;
}

/** Stage 4: the times, in the source's order - the fill instant first, then the readings. */
export function temporalRefusals(specification, definitions) {
  const refusals = [];
  const undeferred = undeferredExecutionRefusal(specification.data);
  if (undeferred !== null) refusals.push(undeferred);
  refusals.push(...unavailableFeatureRefusals(specification, definitions));
  return refusals;
}

/** One warning, which is reported beside an accepted specification and never blocks it. */
function warning(code, path, message, values) {
  return { code, path, reason: fill(message, values) };
}

/** Every alias the selection, the entry tree and the exit block read. */
export function referencedAliases(specification) {
  const referenced = new Set();
  if (specification.selection.feature) referenced.add(specification.selection.feature);
  if (specification.exit.feature) referenced.add(specification.exit.feature);
  const visit = (node) => {
    if (node === null || node === undefined) return;
    if (Array.isArray(node.all)) node.all.forEach(visit);
    else if (Array.isArray(node.any)) node.any.forEach(visit);
    else if (node.feature) referenced.add(node.feature);
  };
  visit(specification.entry.conditions);
  return referenced;
}

/** The four warning rules, in the fixed order the source collects them. */
export function collectWarnings(specification) {
  return [
    ...warmupWarnings(specification),
    ...unusedFeatureWarnings(specification),
    ...positionCapWarnings(specification.portfolio),
    ...postRunWarnings(specification.risk.post_run),
  ];
}

/** A lookback that dominates the rebalance interval warns, because its warm-up consumes the window. */
export function warmupWarnings(specification) {
  const ceiling = (FREQUENCIES[specification.rebalance.frequency] ?? 0) * WARMUP_MULTIPLE;
  return Object.entries(specification.features.declarations)
    .filter(([, declaration]) => declaration.lookback > ceiling)
    .map(([alias, declaration]) =>
      warning('feature_warmup_dominates', `features.declared.${alias}.lookback`, WARNING_TEXT.warmup, {
        lookback: declaration.lookback,
        frequency: specification.rebalance.frequency,
        ceiling,
      }),
    );
}

/** A declared alias nothing reads warns, because its value is never consulted. */
export function unusedFeatureWarnings(specification) {
  const referenced = referencedAliases(specification);
  return Object.entries(specification.features.declarations)
    .filter(([alias]) => !referenced.has(alias))
    .map(([alias]) =>
      warning('unused_declared_feature', `features.declared.${alias}`, WARNING_TEXT.unused, { alias }),
    );
}

/** A position cap that cannot invest the whole portfolio warns. */
export function positionCapWarnings(portfolio) {
  const allocated = portfolio.position_count * portfolio.max_position_weight;
  if (!(allocated < valueOf(FULLY_INVESTED))) return [];
  return [
    warning('position_cap_below_fully_invested', valueOf(REFUSAL_PATHS.portfolio), WARNING_TEXT.positionCap, {
      count: portfolio.position_count,
      weight: portfolio.max_position_weight,
      allocated,
    }),
  ];
}

/** Every post-run limit warns, because the run it is recorded beside cannot obey it. */
export function postRunWarnings(postRun) {
  return Object.entries(postRun)
    .filter(([, value]) => value !== null && value !== undefined)
    .map(([name]) =>
      warning('post_run_limits_are_gate_conditions', `${valueOf(REFUSAL_PATHS.postRun)}.${name}`, WARNING_TEXT.postRun, {
        name,
      }),
    );
}

/** Stage 5: compose the plan of an accepted specification, with the identities it carries. */
export function composePlan(specification, policy, definitions, options = {}) {
  const registry = options.registry ?? {};
  const policyDigest = freeze(policy).digest;
  const specificationDigest = digestOf(specification);
  const registryDigest = digestOf(registry);
  const features = definitions.map((definition) => digestOf(definition)).join('|');
  const behaviouralDigest = digestOf({
    schema: valueOf(SCHEMA_VERSION),
    specification: specificationDigest,
    policy: policyDigest,
    registry: registryDigest,
    features,
    assumptions: null,
    engine: ENGINE_VERSION,
  });
  return {
    specificationDigest,
    behaviouralDigest,
    policyDigest,
    registryDigest,
    assumptionsDigest: null,
    engineVersion: ENGINE_VERSION,
    migrations: options.migrations ? Object.keys(options.migrations) : [],
    digests: {
      keys: valueOf(COMPILE_DIGEST_KEYS),
      canonical: canonicalJson({
        schema: valueOf(SCHEMA_VERSION),
        specification: specificationDigest,
        policy: policyDigest,
        registry: registryDigest,
        features,
        assumptions: null,
        engine: ENGINE_VERSION,
      }),
    },
    notModelled: NOT_MODELLED,
  };
}

/** Assemble the outcome of the stage pipeline, in one shape for a refusal and for an acceptance. */
function outcome(stage, refusals, warnings, specification, plan, migrations) {
  const first = refusals.length > 0 ? refusals[0] : null;
  return {
    ok: refusals.length === 0,
    stage,
    code: first === null ? null : first.code,
    reason: first === null ? null : first.reason,
    refusals,
    warnings,
    specification,
    plan,
    migrations,
  };
}

/**
 * Run the five stages over a payload and return the structured outcome.
 *
 * A stage runs only when the stage before it refused nothing, and the refusals of the stage that
 * refuses are returned together, so a candidate refused for four reasons is one outcome carrying
 * four reasons. A refusal is never softened by a warning: warnings are computed only for a
 * specification that passed every stage, and they are a separate channel.
 */
export function validateSpecification(specification, policy, options = {}) {
  const parsed = parseSpecification(specification, policy, options);
  if (parsed.specification === null) {
    return outcome('schema_validation', parsed.refusals, [], null, null, parsed.migrations);
  }

  const resolved = resolveFeatures(parsed.specification, options.registry ?? {});
  if (resolved.refusals.length > 0) {
    return outcome('feature_resolution', resolved.refusals, [], parsed.specification, null, parsed.migrations);
  }

  const semantic = semanticRefusals(parsed.specification, policy);
  if (semantic.length > 0) {
    return outcome('semantic_validation', semantic, [], parsed.specification, null, parsed.migrations);
  }

  const temporal = temporalRefusals(parsed.specification, resolved.definitions);
  if (temporal.length > 0) {
    return outcome('temporal_validation', temporal, [], parsed.specification, null, parsed.migrations);
  }

  return outcome(
    'trusted_compilation',
    [],
    collectWarnings(parsed.specification),
    parsed.specification,
    composePlan(parsed.specification, policy, resolved.definitions, options),
    parsed.migrations,
  );
}

/**
 * Resolve one value through the reader the field's block declares, without a whole candidate.
 *
 * This is the resolution path a policy view needs: it answers what a value a reader would accept,
 * and with which code and message it would be refused, for one named field at a time.
 */
export function resolveField(policy, path, value, aliases = []) {
  const entry = fieldAt(path);
  if (entry === null) throw new TypeError(`no block declares the field ${path}`);
  const parent = path.slice(0, path.lastIndexOf('.'));
  const refusals = [];
  const reader = makeReader(policy, { [entry.name]: value }, parent, refusals);
  let accepted = null;

  switch (entry.reads) {
    case 'declared':
      accepted = declared(reader, entry.name, entry.declaration);
      break;
    case 'declared_list':
      accepted = declaredList(reader, entry.name, entry.declaration);
      break;
    case 'bounded':
      accepted = bounded(reader, entry.name, entry.declaration, !entry.optional);
      break;
    case 'bounded_int':
      accepted = boundedInt(reader, entry.name, entry.declaration, !entry.optional);
      break;
    case 'integer':
      accepted = integer(reader, entry.name, !entry.optional);
      break;
    case 'boolean':
      accepted = boolean(reader, entry.name, true);
      break;
    case 'identifiers':
      accepted = identifiers(reader, entry.name, !entry.optional);
      break;
    case 'feature_ref':
      accepted = featureRef(reader, entry.name, aliases, !entry.optional);
      break;
    case 'reference':
      accepted = reference(reader, entry.name, !entry.optional);
      break;
    case 'text':
      accepted = text(reader, entry.name, !entry.optional);
      break;
    default:
      throw new TypeError(`the field ${path} is read by ${entry.reads}, which resolves no single value`);
  }

  if (refusals.length > 0) return { ...refusals[0], field: path };
  return { ok: true, field: path, value: accepted };
}

