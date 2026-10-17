#!/usr/bin/env bun
/**
 * Fact checker: every transcription in src/data/ must still be supported by the file it names.
 *
 * The console claims to implement rules that live in this repository. That claim is only worth
 * something if it is checked, so this tool walks every data module, collects every `fact(...)`
 * wrapper, and asserts that the verbatim text recorded with it is still present in the file at the
 * recorded path. A line number that no longer holds its text is reported as a move (the fact still
 * holds); a missing file, or a text that has gone, is an error.
 *
 * The same check runs inside the test suite via tests/facts.test.mjs, so `bun test` fails when a
 * document the console quotes has moved on without it.
 *
 * Usage:  bun run verify        (exit 0 when every fact is supported, 1 otherwise)
 */

import { readdirSync, readFileSync } from 'node:fs';
import { join, normalize } from 'node:path';
import { pathToFileURL } from 'node:url';

import { isFact, normalise } from '../src/data/source.js';

export const APP_DIR = normalize(join(import.meta.dir, '..'));
export const REPO_ROOT = normalize(join(APP_DIR, '..', '..'));
const DATA_DIR = join(APP_DIR, 'src', 'data');

function collect(node, trail, out, seen) {
  if (node === null || typeof node !== 'object') return;
  if (seen.has(node)) return;
  seen.add(node);

  if (isFact(node)) {
    out.push({ trail, fact: node });
    return;
  }
  if (Array.isArray(node)) {
    node.forEach((item, index) => collect(item, `${trail}[${index}]`, out, seen));
    return;
  }
  for (const [key, value] of Object.entries(node)) collect(value, `${trail}.${key}`, out, seen);
}

/** Every fact wrapper exported by every data module, with the trail that found it. */
export async function loadFacts() {
  const found = [];
  const files = readdirSync(DATA_DIR)
    .filter((name) => name.endsWith('.js') && name !== 'source.js')
    .sort();

  for (const file of files) {
    const module = await import(pathToFileURL(join(DATA_DIR, file)).href);
    for (const [name, value] of Object.entries(module)) {
      collect(value, `${file}:${name}`, found, new Set());
    }
  }
  return found;
}

/**
 * Verify every fact. Returns { checked, errors, moved, facts }:
 *   errors - the supporting text is missing, or the file is
 *   moved  - the text is present but no longer at the recorded line
 */
export async function verifyFacts() {
  const facts = await loadFacts();
  const rawCache = new Map();
  const errors = [];
  const moved = [];
  let checked = 0;

  const readRaw = (path) => {
    if (!rawCache.has(path)) {
      try {
        rawCache.set(path, readFileSync(join(REPO_ROOT, path), 'utf8'));
      } catch {
        rawCache.set(path, null);
      }
    }
    return rawCache.get(path);
  };

  for (const { trail, fact } of facts) {
    const { path, line, text } = fact.source;
    const raw = readRaw(path);
    if (raw === null) {
      errors.push({ trail, path, line, why: 'file not found' });
      continue;
    }

    const needle = normalise(text);
    if (needle.length === 0) {
      errors.push({ trail, path, line, why: 'no supporting text recorded' });
      continue;
    }
    if (!normalise(raw).includes(needle)) {
      errors.push({ trail, path, line, why: 'the supporting text is no longer in the file' });
      continue;
    }

    // The fact holds. Check only whether the citation still points at it, so a moved line shows up
    // as a stale citation to fix rather than as a broken claim.
    const probe = needle.split(' ').slice(0, 8).join(' ');
    const atLine = normalise((raw.split('\n')[line - 1] ?? '').trimEnd());
    if (!atLine.includes(probe) && !atLine.startsWith(probe.split(' ')[0] ?? '')) {
      moved.push({ trail, path, line });
    }
    checked++;
  }

  return { checked, errors, moved, facts };
}

if (import.meta.main) {
  const { checked, errors, moved } = await verifyFacts();
  console.log(`checked ${checked} transcribed facts against the repository`);

  if (moved.length > 0) {
    console.log(`\nnote: ${moved.length} citation(s) point at a line that has moved:`);
    for (const item of moved.slice(0, 15)) console.log(`  ${item.path}:${item.line}  ${item.trail}`);
    if (moved.length > 15) console.log(`  ... and ${moved.length - 15} more`);
  }

  if (errors.length > 0) {
    console.log(`\nFinding: ${errors.length} transcription(s) are no longer supported:`);
    for (const item of errors) {
      console.log(`  ${item.path}:${item.line}  ${item.why}`);
      console.log(`    ${item.trail}`);
    }
    process.exit(1);
  }

  console.log('every transcription is supported by the file it names');
}
