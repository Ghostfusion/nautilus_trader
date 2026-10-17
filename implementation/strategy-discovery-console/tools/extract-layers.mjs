#!/usr/bin/env bun
/**
 * Generates src/data/layers.js from the repository itself.
 *
 * The console quotes twelve tutorial pages, reports twenty-two acceptance criteria and lists the
 * venue-credential names one by one. All of that is transcription, and transcription by hand drifts,
 * so this script reads the sources and writes the data module with a `fact(...)` wrapper around
 * every extracted value: the file, the line it was read at, and the verbatim text. The verbatim text
 * is what tools/check-facts.mjs verifies, so a regenerated module and a checked module are the same
 * claim.
 *
 * Sources read:
 *   tutorials/strategy-discovery/<page>/README.md          the idea, the grade, the caveats, citations
 *   python/tests/acceptance/discovery_acceptance_bundle.json  the 22 acceptance criteria
 *   scripts/strip-adapter-env.bash                         the canonical credential names
 *
 * Usage:  bun run extract      (writes src/data/layers.js; then run bun test and bun run verify)
 */

import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, normalize, relative, resolve } from 'node:path';

const APP_DIR = normalize(join(import.meta.dir, '..'));
const REPO_ROOT = normalize(join(APP_DIR, '..', '..'));
const GROUP_DIR = join(REPO_ROOT, 'tutorials', 'strategy-discovery');
const OUT = join(APP_DIR, 'src', 'data', 'layers.js');

const BUNDLE = 'python/tests/acceptance/discovery_acceptance_bundle.json';
const CREDENTIALS = 'scripts/strip-adapter-env.bash';
const TUTORIAL_ROOT = 'tutorials/strategy-discovery';

/** The pipeline, in the order the console presents it. The folder is the tutorial that covers it. */
const PIPELINE = [
  { id: 'policy', folder: 'the-rules-nobody-may-choose', step: 1 },
  { id: 'candidate', folder: 'what-a-candidate-is', step: 2 },
  { id: 'record', folder: 'what-ran-versus-what-was-written', step: 3 },
  { id: 'executor', folder: 'one-executor', step: 4 },
  { id: 'counts', folder: 'counting-the-search', step: 5 },
  { id: 'holdout', folder: 'two-kinds-of-held-back-data', step: 6 },
  { id: 'gates', folder: 'the-admission-gate', step: 7 },
  { id: 'genome', folder: 'search-by-evolution', step: 8 },
  { id: 'agents', folder: 'agents-that-propose', step: 9 },
  { id: 'shadow', folder: 'the-shadow-period', step: 10 },
  { id: 'audit', folder: 'auditing-a-campaign', step: 11 },
  { id: 'boundary', folder: 'the-boundary', step: 12 },
];

/* ------------------------------------------------------------------ *
 * Markdown helpers
 * ------------------------------------------------------------------ */

const linesOf = (text) => text.split('\n');

/** The half-open line range of each `## ` section. */
function sectionsOf(lines) {
  const sections = new Map();
  let current = null;
  lines.forEach((line, index) => {
    const match = /^## (.+?)\s*$/.exec(line);
    if (!match) return;
    if (current) sections.get(current).end = index;
    current = match[1];
    sections.set(current, { start: index + 1, end: lines.length });
  });
  return sections;
}

/** Raw slice of a section, and the line its first non-blank line sits on. */
function rawSection(lines, sections, name) {
  const range = sections.get(name);
  if (!range) throw new Error(`no section named "${name}"`);
  let start = range.start;
  while (start < range.end && lines[start].trim() === '') start++;
  return { text: lines.slice(start, range.end).join('\n'), line: start + 1 };
}

/** The bullets of a section: a `- ` line and its indented continuation lines. */
function bullets(lines, sections, name) {
  const range = sections.get(name);
  if (!range) throw new Error(`no section named "${name}"`);
  const out = [];
  let open = null;
  for (let index = range.start; index < range.end; index++) {
    const line = lines[index];
    if (/^- /.test(line)) {
      if (open) out.push(open);
      open = { start: index, end: index + 1 };
      continue;
    }
    if (open && /^\s+\S/.test(line)) {
      open.end = index + 1;
      continue;
    }
    if (open && line.trim() === '') {
      out.push(open);
      open = null;
    }
  }
  if (open) out.push(open);
  return out.map((item) => ({
    text: lines.slice(item.start, item.end).join('\n'),
    line: item.start + 1,
  }));
}

/** One cell of the provenance table, by the row's label. */
function tableRow(lines, label) {
  const index = lines.findIndex((line) => /^\|/.test(line) && line.includes(`| ${label}`));
  if (index === -1) throw new Error(`no provenance row labelled "${label}"`);
  const cells = lines[index].split('|');
  return { text: cells[2] ?? '', line: index + 1 };
}

/** Markdown to display text: links become their labels, code spans lose their backticks, and a list
 *  item loses its bullet, because the value is the sentence and the verbatim text keeps the bullet. */
function clean(text) {
  return String(text)
    .replace(/\[([^\]]+)\]\([^)\s]+\)/g, '$1')
    .replace(/`([^`]*)`/g, '$1')
    .replace(/\*\*([^*]+)\*\*/g, '$1')
    .replace(/\*([^*]+)\*/g, '$1')
    .replace(/\s+/g, ' ')
    .trim()
    .replace(/^[-*]\s+/, '');
}

/** Repository-relative paths of every link inside a chunk of markdown, in order, deduplicated. */
function links(markdown, fromPath) {
  const found = [];
  for (const match of markdown.matchAll(/\[([^\]]+)\]\(([^)\s]+)\)/g)) {
    const target = match[2].split('#')[0];
    if (target.length === 0 || /^[a-z]+:/.test(target)) continue;
    const absolute = resolve(dirname(join(REPO_ROOT, fromPath)), target);
    if (!absolute.startsWith(REPO_ROOT)) continue;
    const path = relative(REPO_ROOT, absolute).split('\\').join('/');
    if (!found.some((item) => item.path === path)) found.push({ label: clean(match[1]), path });
  }
  return found;
}

/** Criterion numbers mentioned beside the word criterion or criteria. */
function criteriaMentioned(text) {
  const numbers = new Set();
  for (const sentence of text.split(/(?<=[.;])\s+/)) {
    if (!/criteri/i.test(sentence)) continue;
    for (const match of sentence.matchAll(/\b(\d{1,2})\b/g)) {
      const value = Number(match[1]);
      if (value >= 1 && value <= 22) numbers.add(value);
    }
  }
  return [...numbers].sort((a, b) => a - b);
}

/* ------------------------------------------------------------------ *
 * Emitting
 * ------------------------------------------------------------------ */

const q = (value) => JSON.stringify(value);
const factLiteral = (value, path, line, text) =>
  `fact(${q(value)}, ${q(path)}, ${line}, ${q(text)})`;

function emitLayers() {
  const blocks = [];
  const order = [];

  for (const entry of PIPELINE) {
    const path = `${TUTORIAL_ROOT}/${entry.folder}/README.md`;
    const text = readFileSync(join(REPO_ROOT, path), 'utf8');
    const lines = linesOf(text);
    const sections = sectionsOf(lines);
    const titleMatch = /^# (.+?)\s*$/.exec(lines[0]);
    if (!titleMatch) throw new Error(`${path}: no top-level heading`);

    const idea = rawSection(lines, sections, 'The idea in one paragraph');
    const grade = tableRow(lines, 'How well it held up');
    const rules = tableRow(lines, 'Where the rules come from');
    const caveats = bullets(lines, sections, 'Where it goes wrong');
    const provenance = rawSection(lines, sections, 'Where this came from');

    const sources = [];
    for (const item of [...links(rules.text, path), ...links(provenance.text, path)]) {
      if (sources.some((found) => found.path === item.path)) continue;
      sources.push({ label: item.path.split('/').pop(), path: item.path });
    }
    const criteria = criteriaMentioned(text);
    order.push(entry.id);

    blocks.push(`  {
    id: ${q(entry.id)},
    order: ${entry.step},
    step: ${entry.step},
    tutorial: ${q(path)},
    title: ${factLiteral(clean(titleMatch[1]), path, 1, lines[0])},
    grade: ${factLiteral(clean(grade.text), path, grade.line, grade.text)},
    idea: ${factLiteral(clean(idea.text), path, idea.line, idea.text)},
    caveats: [
${caveats.map((item) => `      ${factLiteral(clean(item.text), path, item.line, item.text)},`).join('\n')}
    ],
    sources: [
${sources.map((item) => `      { label: ${q(item.label)}, path: ${q(item.path)} },`).join('\n')}
    ],
    criteria: [${criteria.join(', ')}],
  },`);
  }

  return { blocks: blocks.join('\n'), order };
}

function emitCriteria() {
  const text = readFileSync(join(REPO_ROOT, BUNDLE), 'utf8');
  const lines = linesOf(text);
  const entries = JSON.parse(text).entries;
  const fields = ['requirement', 'requirement_source', 'fixture', 'milestone', 'status', 'observed'];

  // The bundle is one object per entry with one field per line, so an entry's range is found from
  // its own criterion line and each field's line is then the provenance for that field's value.
  const starts = [];
  lines.forEach((line, index) => {
    const match = /^\s*"criterion":\s*(\d+),\s*$/.exec(line);
    if (match) starts.push({ criterion: Number(match[1]), line: index });
  });

  const blocks = entries.map((entry) => {
    const position = starts.findIndex((item) => item.criterion === entry.criterion);
    if (position === -1) throw new Error(`criterion ${entry.criterion} is not in the bundle`);
    const from = starts[position].line;
    const to = position + 1 < starts.length ? starts[position + 1].line : lines.length;
    const located = { criterion: { line: from + 1, text: lines[from].trim() } };

    for (const field of fields) {
      const pattern = new RegExp(`^\\s*"${field}":`);
      const index = lines.findIndex(
        (line, at) => at >= from && at < to && pattern.test(line),
      );
      if (index === -1) throw new Error(`criterion ${entry.criterion}: no "${field}" field`);
      located[field] = { line: index + 1, text: lines[index].trim() };
    }

    return `  {
    id: ${factLiteral(entry.criterion, BUNDLE, located.criterion.line, located.criterion.text)},
    requirement: ${factLiteral(clean(entry.requirement), BUNDLE, located.requirement.line, located.requirement.text)},
    requirementSource: ${factLiteral(entry.requirement_source, BUNDLE, located.requirement_source.line, located.requirement_source.text)},
    fixture: ${factLiteral(entry.fixture, BUNDLE, located.fixture.line, located.fixture.text)},
    milestone: ${factLiteral(entry.milestone, BUNDLE, located.milestone.line, located.milestone.text)},
    status: ${factLiteral(entry.status, BUNDLE, located.status.line, located.status.text)},
    observed: ${factLiteral(clean(entry.observed), BUNDLE, located.observed.line, located.observed.text)},
  },`;
  });

  return { blocks: blocks.join('\n'), count: entries.length };
}

function emitCredentials() {
  const text = readFileSync(join(REPO_ROOT, CREDENTIALS), 'utf8');
  const lines = linesOf(text);
  const start = lines.findIndex((line) => /^ADAPTER_ENV_VARS=\(/.test(line));
  const end = lines.findIndex((line, index) => index > start && /^\)/.test(line));
  if (start === -1 || end === -1) throw new Error(`${CREDENTIALS}: ADAPTER_ENV_VARS not found`);

  const found = [];
  for (let index = start + 1; index < end; index++) {
    const name = lines[index].trim();
    if (name.length === 0 || name.startsWith('#')) continue;
    found.push(`  ${factLiteral(name, CREDENTIALS, index + 1, lines[index])},`);
  }
  return { blocks: found.join('\n'), count: found.length };
}

/* ------------------------------------------------------------------ *
 * Write
 * ------------------------------------------------------------------ */

const layers = emitLayers();
const criteria = emitCriteria();
const credentials = emitCredentials();

const header = `/**
 * GENERATED by tools/extract-layers.mjs - do not edit by hand.
 *
 * Every value here is transcribed from the repository inside a \`fact(...)\` wrapper that records the
 * file, the line and the verbatim text it came from. tools/check-facts.mjs verifies those texts, in
 * the test suite as well as by hand, so this module cannot quietly drift from the tutorials, the
 * acceptance bundle or the credential list.
 *
 * Regenerate after the sources change:  bun run extract
 */

import { fact, valueOf } from './source.js';

/** The layers in pipeline order, which is the order the console presents them in. */
export const LAYER_ORDER = [${layers.order.map(q).join(', ')}];
`;

const body = `
export const LAYERS = [
${layers.blocks}
];

export const CRITERIA = [
${criteria.blocks}
];

/** Every adapter environment variable the repository treats as a credential (${credentials.count} of them). */
export const CREDENTIALS = [
${credentials.blocks}
];

const unwrap = (row) => ({
  ...row,
  title: valueOf(row.title),
  grade: valueOf(row.grade),
  // The grade cell reads "Mixed: the clause it rests on", so the clause is also exposed on its own
  // rather than making every view split the string the same way.
  gradeRestOn: valueOf(row.grade).includes(': ')
    ? valueOf(row.grade).slice(valueOf(row.grade).indexOf(': ') + 2)
    : '',
  idea: valueOf(row.idea),
  caveats: row.caveats.map(valueOf),
});

/** Every layer row, in pipeline order, with the transcribed values unwrapped for display. */
export function layers() {
  return LAYERS.map(unwrap);
}

/** One layer row by id. Throws on an unknown id, because a view asking for a layer that is not
 *  there is a wiring mistake rather than a condition the interface should render. */
export function layer(id) {
  const row = LAYERS.find((item) => item.id === id);
  if (!row) throw new Error(\`unknown layer: \${id}\`);
  return unwrap(row);
}

/** The twenty-two acceptance criteria, with the transcribed values unwrapped for display. */
export function criteria() {
  return CRITERIA.map((row) => ({
    id: valueOf(row.id),
    requirement: valueOf(row.requirement),
    requirementSource: valueOf(row.requirementSource),
    fixture: valueOf(row.fixture),
    milestone: valueOf(row.milestone),
    status: valueOf(row.status),
    observed: valueOf(row.observed),
  }));
}

/** The criteria that do not pass. The console reports these rather than hiding them. */
export function failingCriteria() {
  return criteria().filter((row) => row.status !== 'passed');
}

/** Every credential name the boundary must refuse to hand a worker. */
export function credentials() {
  return CREDENTIALS.map(valueOf);
}
`;

writeFileSync(OUT, `${header}${body}`, 'utf8');

console.log(`wrote ${relative(REPO_ROOT, OUT).split('\\').join('/')}`);
console.log(`  ${layers.order.length} layers, ${criteria.count} acceptance criteria, ${credentials.count} credentials`);
