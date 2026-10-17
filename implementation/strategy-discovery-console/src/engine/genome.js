/**
 * The search's genotype and operators: what a candidate's genome is, and how a generation breeds.
 *
 * Models `python/nautilus_trader/discovery/genome.py` (the gene tables, the three degrees of
 * freedom, mutation, recombination and mechanism replacement) and the parts of `evolve.py` that
 * decide how a generation moves: the seed a generation draws from, the family weights, the mix of
 * mutation and recombination, the retry cap, the equivalence rule and the Pareto front.
 *
 * Two things are the console's own rather than the repository's, and both are visible in the view.
 * The repository's search starts from candidates the compiler produced, so nothing in it draws an
 * initial population; `makeGenome` draws one from the declaration table, which is what makes the
 * operators watchable. And the seeded generator is the console's own (`mulberry32` over the
 * `seed:generation` text), not CPython's Mersenne Twister, so a seed fixes a run here without
 * reproducing a run there.
 *
 * A genome is the candidate specification as plain nested data: the blocks the genotype addresses,
 * with collections as arrays and statements under `identity`. Nothing here reads a document at
 * runtime; every name, bound and message comes from `../data/genome.js`.
 */

import { canonicalJson, digestOf } from './digest.js';
import { makeRng } from './rng.js';
import {
  degrees,
  geneTables,
  genomeRefusals,
  ignoredPaths,
  integerBounds,
  objectives,
  searchConstants,
  searchRefusals,
  frozenFields,
  demoPolicy,
} from '../data/genome.js';

/** The three degrees of freedom, in the order the source declares them. */
export const DEGREES_OF_FREEDOM = degrees();

/** The ids of those degrees, so the operators name a degree the transcription declares. */
const [PARAMETER_DEGREE, STRUCTURAL_DEGREE, FAMILY_DEGREE] = DEGREES_OF_FREEDOM.map(
  (degree) => degree.id,
);

/**
 * The console's own placeholder for the three statements a genome carries.
 *
 * A statement is not a gene: the search draws values and never writes prose, so a drawn genome can
 * only carry the statement the candidate was compiled with. `replaceMechanism` is the one operation
 * that changes them, which is exactly why the third degree exists.
 */
export const NO_STATEMENT = 'no statement: the search draws values, not statements';

/** The generator name a drawn genome records in its provenance. */
const CONSOLE_GENERATOR = 'discovery.console/genome';

/** The genotype's declared positions, from the transcribed gene tables. */
export function GENES() {
  return geneTables();
}

/** Wrap a refusal from the transcription table, substituting the message's named values. */
function refuse(table, key, values = {}) {
  const entry = table[key];
  return {
    ok: false,
    code: entry.code,
    reason: entry.message.replace(/\{(\w+)\}/g, (match, name) =>
      name in values ? String(values[name]) : match,
    ),
  };
}

/** The refusal table for the genotype-level operators. */
function genomeRefusal(key, values) {
  return refuse(genomeRefusals(), key, values);
}

/** The refusal table for the search-level rules. */
function searchRefusal(key, values) {
  return refuse(searchRefusals(), key, values);
}

/** The bound a policy declares under a name, or undefined when it declares none. */
function boundOf(policy, name) {
  return (policy.bounds ?? []).find((bound) => bound.name === name);
}

/** The declared set a policy holds under a name, in the order the policy writes it. */
function setOf(policy, name) {
  return policy.sets?.[name] ?? [];
}

/** Whether a value is a numeric gene value: a boolean is not one, whatever JavaScript thinks. */
function isNumeric(value) {
  return typeof value === 'number' && Number.isFinite(value);
}

/** Whether a bound admits a value. A range is inclusive; a choice set is enumerated. */
function admits(bound, value) {
  if (bound.kind === 'choice') return (bound.choices ?? []).includes(value);
  const lower = bound.lower ?? 0;
  const upper = bound.upper ?? lower;
  return value >= lower && value <= upper;
}

/** A bound in the form a message states it, as the repository's own `Bound.describe` writes it. */
function describeBound(bound) {
  if (bound.kind === 'choice') {
    return `one of ${(bound.choices ?? []).map(String).join(', ')}`;
  }
  return `${bound.lower ?? 0} to ${bound.upper ?? bound.lower ?? 0} inclusive`;
}

/** The digits a value carries after its point, which is the precision a draw quantises to. */
function decimalsOf(value) {
  const text = String(value);
  const point = text.indexOf('.');
  return point === -1 ? 0 : text.length - point - 1;
}

/** A value quantised to a number of places, which is what a drawn decimal is written as. */
function quantise(value, places) {
  return Number(value.toFixed(Math.min(places, 20)));
}

// --- specification paths ---------------------------------------------------

/** Split a specification path into its named and indexed steps. */
function steps(path) {
  return path.split('.').map((part) => {
    const bracket = part.indexOf('[');
    if (bracket === -1) return { name: part, index: null };
    return { name: part.slice(0, bracket), index: Number(part.slice(bracket + 1, -1)) };
  });
}

/** Read the value at a path, or undefined when the specification does not carry one. */
export function readPath(specification, path) {
  let current = specification;
  for (const { name, index } of steps(path)) {
    if (current === null || typeof current !== 'object') return undefined;
    current = current[name];
    if (index === null) continue;
    if (!Array.isArray(current) || index >= current.length) return undefined;
    current = current[index];
  }
  return current;
}

/** Return a copy of the specification with the value at a path replaced. */
function writePath(specification, path, value) {
  const parts = steps(path);
  const rebuild = (node, index) => {
    const { name, index: item } = parts[index];
    const last = index === parts.length - 1;
    if (item === null) {
      return { ...node, [name]: last ? value : rebuild(node[name], index + 1) };
    }
    const list = node[name].slice();
    list[item] = last ? value : rebuild(list[item], index + 1);
    return { ...node, [name]: list };
  };
  return rebuild(specification, 0);
}

/** Expand a parameter path's `{index}` once per element of the block the path names. */
function expandPath(pattern, specification) {
  if (!pattern.includes('{index}')) return [pattern];
  const [head, tail] = pattern.split('[{index}]');
  const collection = readPath(specification, head);
  if (!Array.isArray(collection)) return [];
  return collection.map((_, index) => `${head}[${index}]${tail}`);
}

/** Every condition leaf's value path, in the order a walk of the condition tree finds them. */
function conditionValuePaths(specification) {
  const paths = [];
  const node = specification.entry?.conditions;
  if (node !== undefined) walkConditions(node, 'entry.conditions', paths);
  return paths;
}

/** Walk one node of the condition tree, mirroring the repository's own walk. */
function walkConditions(node, path, paths) {
  if (node === null || typeof node !== 'object') return;
  if (!Array.isArray(node.conditions)) {
    paths.push(`${path}.value`);
    return;
  }
  node.conditions.forEach((child, index) => walkConditions(child, `${path}.conditions[${index}]`, paths));
}

// --- genes -----------------------------------------------------------------

/** The values a gene may take other than the one it holds, in the repository's own order. */
function choicesOf(gene) {
  if (gene.bound && gene.bound.kind === 'range') return [];
  if (gene.bound) return (gene.bound.choices ?? []).filter((choice) => choice !== gene.value);
  if (gene.declared.length > 0) {
    return gene.declared.filter((choice) => choice !== gene.value).sort();
  }
  if (typeof gene.value === 'boolean') return [!gene.value];
  return [];
}

/** Whether a gene has any other admissible value, which is what a mutation needs. */
function hasFreeValue(gene) {
  if (gene.bound) {
    if (gene.bound.kind === 'choice') return choicesOf(gene).length > 0;
    return (gene.bound.lower ?? 0) !== (gene.bound.upper ?? gene.bound.lower ?? 0);
  }
  return choicesOf(gene).length > 0;
}

/** Whether a value is admissible at a gene's own position, which is what a donor value needs. */
function admitsValue(gene, value) {
  if (gene.bound) {
    if (!isNumeric(value) || !isNumeric(gene.value)) return false;
    if (Number.isInteger(value) !== Number.isInteger(gene.value)) return false;
    return admits(gene.bound, value);
  }
  if (gene.declared.length > 0) return typeof value === 'string' && gene.declared.includes(value);
  return typeof value === 'boolean';
}

/**
 * Every gene of a genome, ordered by path: a parameter gene per bounded field with its bound, a
 * parameter gene per condition leaf, and a structural gene per switched field with the declared set
 * it selects from.
 */
export function genesOf(specification, policy) {
  const tables = geneTables();
  const genes = [];
  for (const [name, pattern] of tables.parameters) {
    const bound = boundOf(policy, name);
    if (!bound) continue;
    for (const path of expandPath(pattern, specification)) {
      const value = readPath(specification, path);
      if (!isNumeric(value)) continue;
      genes.push({
        name,
        path,
        degree: PARAMETER_DEGREE,
        value,
        bound,
        declared: [],
        choices: [],
      });
    }
  }
  const conditionBound = boundOf(policy, tables.conditionBound);
  if (conditionBound) {
    for (const path of conditionValuePaths(specification)) {
      const value = readPath(specification, path);
      if (!isNumeric(value)) continue;
      genes.push({
        name: tables.conditionBound,
        path,
        degree: PARAMETER_DEGREE,
        value,
        bound: conditionBound,
        declared: [],
        choices: [],
      });
    }
  }
  for (const [path, setName] of tables.structural) {
    const value = readPath(specification, path);
    if (!(typeof value === 'string' || typeof value === 'boolean')) continue;
    genes.push({
      name: path,
      path,
      degree: STRUCTURAL_DEGREE,
      value,
      bound: null,
      declared: setName ? setOf(policy, setName) : [],
      choices: [],
    });
  }
  return genes
    .map((gene) => ({ ...gene, choices: choicesOf(gene) }))
    .sort((left, right) => (left.path < right.path ? -1 : left.path > right.path ? 1 : 0));
}

/** The paths of the parameter genes, in the order the genotype holds them. */
function parameterPaths(specification, policy) {
  return genesOf(specification, policy)
    .filter((gene) => gene.degree === PARAMETER_DEGREE)
    .map((gene) => gene.path);
}

/** The paths of the structural genes, in the order the genotype holds them. */
function structuralPaths(specification, policy) {
  return genesOf(specification, policy)
    .filter((gene) => gene.degree === STRUCTURAL_DEGREE)
    .map((gene) => gene.path);
}

// --- draws -----------------------------------------------------------------

/** Draw an integer inside a range bound, as the repository's integer draw does. */
function drawInteger(bound, rng, attempts) {
  const lower = bound.lower ?? 0;
  const upper = bound.upper ?? lower;
  const floor = Math.ceil(lower);
  const ceiling = Math.floor(upper);
  if (floor > ceiling) {
    throw new Error(genomeRefusal('noIntegerInBound', { bound: describeBound(bound) }).reason);
  }
  for (let attempt = 0; attempt < attempts; attempt++) {
    const drawn = rng.int(floor, ceiling);
    if (admits(bound, drawn)) return drawn;
  }
  throw new Error(genomeRefusal('integerDrawExhausted', { bound: describeBound(bound) }).reason);
}

/** Draw a decimal inside a range bound, quantised to the held value's precision. */
function drawDecimal(bound, current, rng, attempts) {
  const lower = bound.lower ?? 0;
  const upper = bound.upper ?? lower;
  const span = upper - lower;
  const places = Math.max(8, decimalsOf(current));
  for (let attempt = 0; attempt < attempts; attempt++) {
    const offset = span === 0 ? 0 : rng.next() * span;
    const drawn = quantise(lower + offset, places);
    if (admits(bound, drawn)) return drawn;
  }
  throw new Error(genomeRefusal('decimalDrawExhausted', { bound: describeBound(bound) }).reason);
}

/** Draw a fresh value for a gene, which is how a population the console draws is filled. */
function drawFresh(name, bound, rng, attempts) {
  if (bound.kind === 'choice') {
    if (!(bound.choices ?? []).length) {
      throw new Error(genomeRefusal('noOtherValue', { path: name }).reason);
    }
    return rng.pick(bound.choices);
  }
  if (integerBounds().includes(name)) return drawInteger(bound, rng, attempts);
  return drawDecimal(bound, bound.lower ?? 0, rng, attempts);
}

/** Draw the value a mutation writes at a gene, which never repeats the value it holds. */
function draw(gene, rng, attempts) {
  if (gene.bound && gene.bound.kind === 'choice') return rng.pick(gene.choices);
  if (gene.bound && integerBounds().includes(gene.name)) return drawInteger(gene.bound, rng, attempts);
  if (gene.bound) return drawDecimal(gene.bound, gene.value, rng, attempts);
  if (gene.choices.length === 0) {
    throw new Error(genomeRefusal('noOtherValue', { path: gene.path }).reason);
  }
  return rng.pick(gene.choices);
}

// --- the genome ------------------------------------------------------------

/** The statement fields, which are paths rather than genes. */
function statementFields() {
  return geneTables().statementFields;
}

/** The mechanism a genome carries, as the four statements. */
export function mechanismOf(specification) {
  return Object.fromEntries(statementFields().map((name) => [name, specification.identity[name]]));
}

/** The declared family a genome belongs to. */
export function familyOf(specification) {
  return specification.identity.family;
}

/** The specification's own digest, which is the candidate's identity. */
export function digestOfSpecification(specification) {
  return digestOf(flattenSpecification(specification));
}

/** Flatten nested specification data into the dotted, indexed keys the canonical serializer takes. */
export function flattenSpecification(specification, prefix = '') {
  const flat = {};
  const into = (value, path) => {
    if (value === null || typeof value !== 'object') {
      if (path) flat[path] = value;
      return;
    }
    if (Array.isArray(value)) {
      value.forEach((item, index) => into(item, `${path}.${index}`));
      return;
    }
    for (const [key, item] of Object.entries(value)) into(item, path ? `${path}.${key}` : key);
  };
  into(specification, prefix);
  return flat;
}

/** Drop a dotted path and anything keyed beneath it, as the equivalence rule does. */
function dropPath(flat, path) {
  delete flat[path];
  const prefix = `${path}.`;
  for (const key of Object.keys(flat)) {
    if (key.startsWith(prefix)) delete flat[key];
  }
}

/**
 * The equivalence key the pre-registered rule gives a candidate: the digest of the canonical text
 * with the five ignored paths removed.
 *
 * The repository holds a decimal-valued gene as text, so its canonical text quotes such a value
 * where the console writes a number; the key here is therefore the console's own digest, and what
 * it reproduces is the rule's decision - which candidates merge - rather than the repository's hex.
 */
export function equivalenceKey(specification) {
  const flat = flattenSpecification(specification);
  for (const path of ignoredPaths()) dropPath(flat, path);
  return digestOf(flat);
}

/** The canonical text of a specification, in the sorted compact form the digest is taken over. */
export function canonicalText(specification) {
  return canonicalJson(flattenSpecification(specification));
}

/**
 * Draw one genome from a policy's declaration table.
 *
 * The repository's search never draws its first population: it starts from candidates the compiler
 * produced, and every later candidate comes from an operator. This is the console's stand-in for
 * that population, so the operators have something to move; every value it writes comes from a
 * declared set or a declared bound, the family included, and every draw comes from the generator it
 * is handed. The three statements are the console's placeholder rather than a draw, because a
 * statement is written by an agent and replaced only through `replaceMechanism`.
 */
export function makeGenome(rng, policy, options = {}) {
  const tables = geneTables();
  const limits = policy.limits ?? { maxConditions: 4, maxConditionDepth: 2, maxFeatures: 8 };
  const attempts = tables.mutationAttempts;
  const allowlist = policy.featureAllowlist ?? [];
  if (allowlist.length === 0) {
    throw new Error('the policy declares no feature to name in a feature declaration');
  }
  const declared = Math.max(1, Math.min(options.features ?? 1, limits.maxFeatures, allowlist.length));
  const leaves = Math.max(0, Math.min(options.conditions ?? 2, limits.maxConditions));
  const leaf = () => ({ value: 0 });
  const conditions = leaves === 0 ? leaf() : { kind: 'all', conditions: Array.from({ length: leaves }, leaf) };

  let specification = {
    identity: {
      family: rng.pick(setOf(policy, 'families').slice().sort()),
      mechanism: NO_STATEMENT,
      hypothesis: NO_STATEMENT,
      falsification: NO_STATEMENT,
    },
    provenance: {
      generator: CONSOLE_GENERATOR,
      parents: [],
      seed: options.seed ?? 0,
    },
    universe: { membership_policy: '', min_price: 0, min_dollar_volume: 0 },
    data: {
      signal_frequency: '',
      decision_time: '',
      execution_eligibility: '',
      corporate_action_treatment: '',
    },
    features: {
      declarations: allowlist
        .slice(0, declared)
        .map((feature) => ({ feature, lookback: 1, skip: 0 })),
    },
    selection: { mode: '', comparison: '', tie_break: '', percentile: 0, count: 0, threshold: 0 },
    entry: { direction: '', conditions },
    exit: {
      comparison: '',
      flatten_ineligible: false,
      holding_period_cap: 1,
      rank_exit_count: 1,
      stop_fraction: 0,
      target_fraction: 0,
    },
    portfolio: {
      weighting: '',
      cash_policy: '',
      position_count: 1,
      max_position_weight: 0,
      max_sector_weight: 0,
    },
    rebalance: { frequency: '', timing: '' },
    risk: {
      runtime: { halt_new_entries: false, flatten_ineligible: false },
      pre_trade: { max_gross_exposure: 0 },
      post_run: { max_drawdown: 0, max_turnover: 0, max_cost_ratio: 0 },
    },
  };

  for (const [path, setName] of tables.structural) {
    const value = setName ? rng.pick(setOf(policy, setName).slice().sort()) : rng.next() < 0.5;
    specification = writePath(specification, path, value);
  }
  for (const [name, pattern] of tables.parameters) {
    const bound = boundOf(policy, name);
    if (!bound) continue;
    for (const path of expandPath(pattern, specification)) {
      specification = writePath(specification, path, drawFresh(name, bound, rng, attempts));
    }
  }
  const conditionBound = boundOf(policy, tables.conditionBound);
  if (conditionBound) {
    for (const path of conditionValuePaths(specification)) {
      specification = writePath(
        specification,
        path,
        drawFresh(tables.conditionBound, conditionBound, rng, attempts),
      );
    }
  }
  for (const gene of genesOf(specification, policy)) {
    if (gene.bound && !admits(gene.bound, gene.value)) {
      throw new Error(genomeRefusal('outOfBound', { path: gene.path, bound: describeBound(gene.bound) }).reason);
    }
  }
  return specification;
}

/** The generator a generation draws from: the same `seed:generation` text always draws it again. */
export function rngFor(seed, generation) {
  return makeRng(`${seed}:${generation}`);
}

// --- operators -------------------------------------------------------------

/**
 * Change the named genes to values drawn from their bounds, or refuse the whole edit.
 *
 * Every path is resolved before anything is drawn, so a caller that asked for three genes is told
 * which one stopped the edit rather than left with two of them changed. A statement path is refused
 * by name, because replacing a mechanism needs the replacement statements.
 */
export function mutate(specification, policy, { paths, rng }) {
  const tables = geneTables();
  const genes = genesOf(specification, policy);
  const seen = new Set();
  const resolved = [];
  for (const path of paths ?? []) {
    if (seen.has(path)) continue;
    seen.add(path);
    const gene = genes.find((item) => item.path === path);
    if (!gene) {
      if (tables.mechanismPaths.includes(path)) return genomeRefusal('statementPath', { path });
      return genomeRefusal('notAGene', { path });
    }
    if (gene.degree === FAMILY_DEGREE) return genomeRefusal('statementPath', { path });
    if (!hasFreeValue(gene)) return genomeRefusal('noOtherValue', { path });
    resolved.push(gene);
  }
  if (resolved.length === 0) return genomeRefusal('noGeneNamed');

  let next = specification;
  const changes = [];
  for (const gene of resolved) {
    const drawn = draw(gene, rng, tables.mutationAttempts);
    next = writePath(next, gene.path, drawn);
    changes.push({
      path: gene.path,
      name: gene.name,
      degree: gene.degree,
      before: gene.value,
      after: drawn,
    });
  }
  return { ok: true, genome: next, changes };
}

/**
 * Exchange a block of compatible genes between two parents of the same family.
 *
 * A parent pair of different families is refused: crossing one family's entry with another's exit
 * produces a specification with no mechanism behind it. The split is drawn from the positions where
 * the parents differ, so an exchange that could miss the only difference is not drawn at all.
 */
export function crossover(first, second, policy, { rng }) {
  if (familyOf(first) !== familyOf(second)) {
    return genomeRefusal('differentFamilies', {
      first: familyOf(first),
      second: familyOf(second),
    });
  }
  const donors = new Map(genesOf(second, policy).map((gene) => [gene.path, gene]));
  const ordered = genesOf(first, policy);
  const differences = ordered
    .map((gene, index) => ({ gene, index }))
    .filter(({ gene }) => {
      const donor = donors.get(gene.path);
      return donor !== undefined && donor.value !== gene.value && admitsValue(gene, donor.value);
    })
    .map(({ index }) => index);
  if (differences.length === 0) return genomeRefusal('noSharedDifference');

  const split = rng.pick(differences);
  let next = first;
  const changes = [];
  for (const gene of ordered.slice(split)) {
    const donor = donors.get(gene.path);
    if (donor === undefined || donor.value === gene.value || !admitsValue(gene, donor.value)) continue;
    next = writePath(next, gene.path, donor.value);
    changes.push({
      path: gene.path,
      name: gene.name,
      degree: gene.degree,
      before: gene.value,
      after: donor.value,
    });
  }
  return { ok: true, genome: next, changes };
}

/**
 * Replace the mechanism, and with it the hypothesis and the falsification statement.
 *
 * This is the third degree of freedom and the only operation that touches a statement: the previous
 * pair describes something that is no longer being tested, so the replacement comes in as an
 * argument rather than being drawn. The checks run in the repository's order - the statements, then
 * the declared family, then whether anything actually changed.
 */
export function replaceMechanism(specification, mechanism, policy, { parents = [] } = {}) {
  const fields = statementFields();
  const missing = fields.filter((name) => !String(mechanism[name] ?? '').trim());
  if (missing.length > 0) {
    return genomeRefusal('statementMissing', { names: missing.join(', ') });
  }
  if (!setOf(policy, 'families').includes(mechanism.family)) {
    return genomeRefusal('familyNotDeclared', { family: mechanism.family });
  }
  const current = mechanismOf(specification);
  const changes = fields
    .filter((name) => current[name] !== mechanism[name])
    .map((name) => ({
      path: `identity.${name}`,
      name: `identity.${name}`,
      degree: FAMILY_DEGREE,
      before: current[name],
      after: mechanism[name],
    }));
  if (changes.length === 0) return genomeRefusal('sameMechanism');

  const next = {
    ...specification,
    identity: {
      ...specification.identity,
      family: mechanism.family,
      mechanism: mechanism.mechanism,
      hypothesis: mechanism.hypothesis,
      falsification: mechanism.falsification,
    },
    provenance: {
      ...specification.provenance,
      parents: parents.length > 0 ? parents : [digestOfSpecification(specification)],
    },
  };
  return { ok: true, genome: next, changes };
}

// --- selection -------------------------------------------------------------

/** The objective names, in the source's order, with their directions and conventions. */
export function OBJECTIVES() {
  return objectives();
}

/** Whether every kept objective is present and finite, which is what selection requires. */
export function complete(result, kept) {
  return kept.every((name) => {
    const value = result.objectives?.[name];
    return typeof value === 'number' && Number.isFinite(value);
  });
}

/** The names of the objectives a comparison keeps, whether they are given as rows or as ids. */
function objectiveNames(kept) {
  return (kept ?? OBJECTIVES()).map((objective) =>
    typeof objective === 'string' ? objective : objective.id,
  );
}

/** Whether the first result is no worse on every kept objective and better on at least one. */
export function dominates(first, second, kept) {
  const names = objectiveNames(kept);
  const directions = new Map(
    OBJECTIVES().map((objective) => [objective.id, objective.prefersHigh]),
  );
  let better = false;
  for (const name of names) {
    const left = first.objectives?.[name];
    const right = second.objectives?.[name];
    if (left === undefined || right === undefined) return false;
    if (directions.get(name)) {
      if (left < right) return false;
      better = better || left > right;
    } else {
      if (left > right) return false;
      better = better || left < right;
    }
  }
  return better;
}

/**
 * The non-dominated results, ordered by candidate identity.
 *
 * `objectives` narrows the comparison to the measurements the reader keeps, which is how the page's
 * own exercise shows that a front depends on which columns are kept; by default all seven are
 * compared and a result missing any of them is not comparable and stays out of the front.
 */
export function paretoFront(results, { objectives: kept } = {}) {
  const names = objectiveNames(kept);
  const comparable = results.filter((result) => complete(result, names));
  const front = comparable.filter(
    (candidate) =>
      !comparable.some(
        (other) => other.candidateId !== candidate.candidateId && dominates(other, candidate, names),
      ),
  );
  return front.slice().sort((left, right) => (left.candidateId < right.candidateId ? -1 : 1));
}

/** The results selection may read, refusing when one of them is not development data. */
export function selectable(results) {
  const locked = results.filter((result) => result.role !== 'development');
  if (locked.length === 0) return { ok: true, results };
  const named = locked.map((result) => result.candidateId).sort().join(', ');
  return searchRefusal('lockedResult', { named, role: locked[0].role });
}

// --- a generation ----------------------------------------------------------

/** The effort weights over the families present, defaulting to an even allocation. */
export function familyWeights(families, searchPolicy) {
  const allocation = searchPolicy.family_allocation ?? {};
  const declared = [...new Set(families)]
    .sort()
    .map((family) => [family, allocation[family] ?? 0]);
  const total = declared.reduce((sum, [, weight]) => sum + weight, 0);
  if (total <= 0) {
    const share = declared.length > 0 ? 1 / declared.length : 0;
    return declared.map(([family]) => [family, share]);
  }
  return declared.map(([family, weight]) => [family, weight / total]);
}

/**
 * Amend the search policy, or refuse the change by name.
 *
 * The generation a policy applies to is a stamp rather than an instruction, so it advances without
 * being refused. Every object the design freezes is refused with the reason it is frozen, and a name
 * the policy's own mutable set does not carry is refused as undeclared, so a change to the search's
 * rules is either inside its declared set or named as the thing it tried to move.
 */
export function amend(searchPolicy, changes) {
  const constants = searchConstants();
  const frozen = new Map(frozenFields().map((row) => [row.field, row.why]));
  const next = { ...searchPolicy };
  for (const [name, value] of Object.entries(changes)) {
    if (name === constants.generationStamp) {
      next.generation = value;
      continue;
    }
    if (frozen.has(name)) return searchRefusal('frozenField', { name, why: frozen.get(name) });
    if (!constants.mutableFields.includes(name)) {
      return searchRefusal('undeclaredField', { name });
    }
    next[name] = value;
  }
  return { ok: true, policy: next, changed: Object.keys(changes) };
}

/** Draw a name from weighted pairs, by one uniform draw against the running total. */
function weightedPick(weights, rng) {
  if (weights.length === 0) return '';
  const drawValue = rng.next();
  let cumulative = 0;
  for (const [name, weight] of weights) {
    cumulative += weight;
    if (drawValue < cumulative) return name;
  }
  return weights[weights.length - 1][0];
}

/** The paths one mutation names: the degree's own paths, up to the cap the operator allows. */
function geneSelection(specification, policy, rng, explore, searchPolicy) {
  const constants = searchConstants();
  let paths = explore
    ? structuralPaths(specification, policy)
    : parameterPaths(specification, policy);
  if (paths.length === 0) {
    paths = genesOf(specification, policy).map((gene) => gene.path);
  }
  if (paths.length === 0) return [];
  const cap = explore
    ? constants.structuralGenesPerMutation
    : constants.parameterGenesPerMutation;
  const count = Math.min(cap, paths.length);
  return rng.shuffle(paths).slice(0, count).sort();
}

/** One edit attempt: recombination when the parents number more than one and the rate allows it. */
function attempt(parents, policy, searchPolicy, rng, explore) {
  if (parents.length > 1 && rng.next() >= (searchPolicy.mutation_rate ?? 0.5)) {
    const first = parents[rng.int(0, parents.length - 1)];
    const second = parents[rng.int(0, parents.length - 1)];
    return crossover(first, second, policy, { rng });
  }
  const parent = parents[rng.int(0, parents.length - 1)];
  return mutate(parent, policy, {
    paths: geneSelection(parent, policy, rng, explore, searchPolicy),
    rng,
  });
}

/** Breed one child from the survivors, retrying a refused edit up to the operator's cap. */
function breed(survivors, policy, searchPolicy, rng) {
  const weights = familyWeights(survivors.map(familyOf), searchPolicy);
  const family = weightedPick(weights, rng);
  let parents = survivors.filter((genome) => familyOf(genome) === family);
  if (parents.length === 0) parents = survivors;
  const explore = rng.next() < (searchPolicy.exploration_share ?? 0.25);
  let edit = attempt(parents, policy, searchPolicy, rng, explore);
  const attempts = searchConstants().editAttempts;
  for (let index = 1; index < attempts; index++) {
    if (edit.ok) return edit;
    edit = attempt(parents, policy, searchPolicy, rng, explore);
  }
  return edit;
}

/** The survivors the front names, in the repository's order: elite, then family floors, then rest. */
function survivorGenomes(front, byDigest, elite, familyFloor) {
  const selected = [];
  const seen = new Set();
  const take = (result) => {
    const genome = byDigest.get(result.digest);
    if (genome === undefined || seen.has(result.digest)) return;
    seen.add(result.digest);
    selected.push(genome);
  };
  front.slice(0, elite).forEach(take);
  for (const family of [...new Set(front.map((result) => result.family))].sort()) {
    front
      .filter((result) => result.family === family)
      .slice(0, familyFloor)
      .forEach(take);
  }
  front.forEach(take);
  return selected;
}

/** Keep the first candidate per equivalence key, recording every later one as a merge. */
export function deduplicate(genomes) {
  const survivors = [];
  const merges = [];
  const firstOf = new Map();
  for (const genome of genomes) {
    const key = equivalenceKey(genome);
    const candidateId = digestOfSpecification(genome);
    if (firstOf.has(key)) {
      merges.push({
        candidateId,
        mergedInto: firstOf.get(key),
        rule: searchConstants().equivalenceRuleId,
        digest: key,
      });
      continue;
    }
    firstOf.set(key, candidateId);
    survivors.push(genome);
  }
  return { survivors, merges };
}

/**
 * Breed one generation from a population and the results measured for it.
 *
 * The order is the repository's: refuse an empty population, refuse a result that is not development
 * data, rank by the front, take the survivors, breed one child per place in the population size,
 * fold candidates that are the same configuration under the equivalence rule, and refuse when no
 * child survived the fold. The plan reports the front, the survivors, every edit attempted
 * including the refused ones, and the merges.
 */
export function nextGeneration({
  population,
  results,
  generation,
  policy,
  searchPolicy,
  seed,
  populationSize,
  elite,
  familyFloor,
  objectives: kept,
}) {
  const constants = searchConstants();
  const active = searchPolicy ?? searchConstants();
  const size = populationSize ?? constants.populationSize;
  const eliteCount = elite ?? constants.elite;
  const floor = familyFloor ?? constants.familyFloor;
  for (const [name, value] of [
    ['population_size', size],
    ['elite', eliteCount],
    ['family_floor', floor],
  ]) {
    if (value < 1) throw new Error(searchRefusal('countTooSmall', { name, value }).reason);
  }

  const plan = (front = [], extra = {}) => ({
    generation,
    policyDigest: digestOf({ policy_id: active.policy_id, version: active.version }),
    population: [],
    front: front.map((result) => result.candidateId),
    survivors: [],
    edits: [],
    merges: [],
    refusal: null,
    reason: '',
    ...extra,
  });
  const refuseWith = (front, entry, extra = {}) => ({
    ...plan(front, extra),
    refusal: { code: entry.code, reason: entry.reason },
    reason: entry.reason,
  });

  if (population.length === 0) return refuseWith([], searchRefusal('populationEmpty'));

  const selection = selectable(results);
  if (!selection.ok) return refuseWith([], selection);

  const byDigest = new Map(population.map((genome) => [digestOfSpecification(genome), genome]));
  const front = paretoFront(selection.results, kept ? { objectives: kept } : {});
  const survivors = survivorGenomes(front, byDigest, eliteCount, floor);
  if (survivors.length === 0) {
    return refuseWith(
      front,
      front.length > 0 ? searchRefusal('unknownCandidate') : searchRefusal('missingObjective'),
    );
  }

  const rng = rngFor(seed, generation);
  const edits = [];
  const children = [];
  for (let place = 0; place < size; place++) {
    const edit = breed(survivors, policy, active, rng);
    edits.push(edit);
    if (edit.ok) children.push(edit.genome);
  }

  const folded = deduplicate([...survivors, ...children]);
  const held = new Set(survivors.map(digestOfSpecification));
  const produced = folded.survivors.filter((genome) => !held.has(digestOfSpecification(genome)));
  const merges = folded.merges.filter((merge) => !held.has(merge.mergedInto));
  const refusedEdits = edits.filter((edit) => !edit.ok);
  if (produced.length === 0) {
    const reason =
      refusedEdits.length > 0 && refusedEdits.length === edits.length
        ? refusedEdits[0].reason
        : searchRefusal('editRefused').reason;
    return refuseWith(front, { code: 'edit_refused', reason }, { edits, merges, survivors });
  }

  return plan(front, { population: produced, edits, merges, survivors });
}
