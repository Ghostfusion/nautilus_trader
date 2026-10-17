/**
 * The transcription check, inside the suite.
 *
 * The console's claim is that it implements rules that live in this repository, and that its
 * quotations are the repository's own words. This test is what makes the claim falsifiable: it
 * re-runs the same verification as `bun run verify` and fails when a quoted paragraph, a bound, a
 * refusal code or an acceptance status has moved on without the console.
 */

import { describe, expect, test } from 'bun:test';

import { verifyFacts } from '../tools/check-facts.mjs';
import { CRITERIA, LAYERS, credentials, layer, layers } from '../src/data/layers.js';
import { isFact, valueOf } from '../src/data/source.js';

describe('transcriptions', () => {
  test('every transcription is still supported by the file it names', async () => {
    const { checked, errors } = await verifyFacts();
    if (errors.length > 0) console.error(errors);
    expect(errors).toEqual([]);
    expect(checked).toBeGreaterThan(300);
  });

  test('every transcribed value is wrapped with its source', () => {
    // A value that lost its wrapper is a value nothing checks, so this is a hard failure.
    for (const row of LAYERS) {
      expect(isFact(row.idea)).toBe(true);
      expect(isFact(row.grade)).toBe(true);
      expect(row.caveats.length).toBeGreaterThan(0);
      for (const caveat of row.caveats) expect(isFact(caveat)).toBe(true);
    }
    for (const row of CRITERIA) {
      expect(isFact(row.status)).toBe(true);
      expect(isFact(row.observed)).toBe(true);
    }
    expect(credentials().length).toBeGreaterThan(50);
  });

  test('the twelve layers are present in pipeline order', () => {
    const rows = layers();
    expect(rows).toHaveLength(12);
    expect(rows.map((row) => row.order)).toEqual([...rows.map((row) => row.order)].sort((a, b) => a - b));
    expect(rows[0].id).toBe('policy');
    expect(rows[11].id).toBe('boundary');
  });

  test('an unknown layer is a wiring mistake, not a condition to render', () => {
    expect(() => layer('nope')).toThrow('unknown layer');
  });

  test('the quoted idea is the tutorial own opening sentence, unwrapped for display', () => {
    const row = layer('candidate');
    expect(row.idea.startsWith('A candidate strategy in this repository')).toBe(true);
    expect(row.idea.includes('](')).toBe(false);
    expect(valueOf(LAYERS[1].idea)).toBe(row.idea);
  });
});
