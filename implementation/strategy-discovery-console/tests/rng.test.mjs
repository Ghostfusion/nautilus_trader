/**
 * Tests for the seeded generator and the summary statistics. The point of the generator is
 * reproducibility, and the point of the statistics is agreement with the definitions, so both are
 * checked directly rather than through the interface.
 */

import { describe, expect, test } from 'bun:test';

import {
  hashSeed,
  kurtosis,
  makeRng,
  mean,
  mulberry32,
  quantile,
  skewness,
  stdev,
} from '../src/engine/rng.js';

describe('the seeded generator', () => {
  test('the same seed produces the same sequence', () => {
    const a = Array.from({ length: 5 }, mulberry32(42));
    const b = Array.from({ length: 5 }, mulberry32(42));
    expect(a).toEqual(b);
  });

  test('a different seed produces a different sequence', () => {
    const a = Array.from({ length: 5 }, mulberry32(42));
    const b = Array.from({ length: 5 }, mulberry32(43));
    expect(a).not.toEqual(b);
  });

  test('a name works as a seed, so a run identity can be one', () => {
    expect(hashSeed('campaign-1')).toBe(hashSeed('campaign-1'));
    expect(hashSeed('campaign-1')).not.toBe(hashSeed('campaign-2'));
    const rng = makeRng('campaign-1');
    expect(rng.int(1, 6)).toBe(makeRng('campaign-1').int(1, 6));
  });

  test('draws stay inside their bounds and cover them', () => {
    const rng = makeRng(7);
    const draws = Array.from({ length: 2000 }, () => rng.real(0, 1));
    expect(Math.min(...draws)).toBeGreaterThanOrEqual(0);
    expect(Math.max(...draws)).toBeLessThan(1);
    for (let i = 0; i < 200; i++) {
      const value = rng.int(3, 9);
      expect(value).toBeGreaterThanOrEqual(3);
      expect(value).toBeLessThanOrEqual(9);
    }
  });

  test('shuffle returns a permutation and never mutates its argument', () => {
    const source = [1, 2, 3, 4, 5, 6, 7, 8];
    const shuffled = makeRng(3).shuffle(source);
    expect(source).toEqual([1, 2, 3, 4, 5, 6, 7, 8]);
    expect(shuffled.slice().sort((a, b) => a - b)).toEqual(source);
    expect(makeRng(3).shuffle(source)).toEqual(shuffled);
  });

  test('pick refuses an empty list instead of returning undefined', () => {
    expect(() => makeRng(1).pick([])).toThrow('pick from an empty list');
  });

  test('the normal draws have roughly the right first two moments', () => {
    const rng = makeRng(11);
    const draws = Array.from({ length: 20000 }, () => rng.normal());
    expect(Math.abs(mean(draws))).toBeLessThan(0.05);
    expect(Math.abs(stdev(draws) - 1)).toBeLessThan(0.05);
  });
});

describe('summary statistics', () => {
  test('mean and standard deviation agree with hand arithmetic', () => {
    expect(mean([1, 2, 3, 4])).toBe(2.5);
    // Sample standard deviation of 1..4 is sqrt(5/3).
    expect(stdev([1, 2, 3, 4])).toBeCloseTo(Math.sqrt(5 / 3), 12);
  });

  test('refuse a sample too small to define them', () => {
    expect(() => mean([])).toThrow('mean of an empty list');
    expect(() => stdev([1])).toThrow('stdev needs at least two observations');
    expect(() => skewness([1, 2])).toThrow('skewness needs at least three observations');
    expect(() => kurtosis([1, 2, 3])).toThrow('kurtosis needs at least four observations');
  });

  test('a symmetric sample has zero skewness and a uniform one has negative excess kurtosis', () => {
    expect(skewness([1, 2, 3, 4, 5])).toBeCloseTo(0, 12);
    expect(kurtosis([1, 2, 3, 4, 5])).toBeLessThan(0);
  });

  test('a constant sample has no shape rather than a division by zero', () => {
    expect(skewness([2, 2, 2, 2])).toBe(0);
    expect(kurtosis([2, 2, 2, 2, 2])).toBe(0);
  });

  test('quantiles interpolate between order statistics', () => {
    expect(quantile([1, 2, 3, 4], 0)).toBe(1);
    expect(quantile([1, 2, 3, 4], 1)).toBe(4);
    expect(quantile([1, 2, 3, 4], 0.5)).toBe(2.5);
    expect(quantile([4, 1, 3, 2], 0.25)).toBeCloseTo(1.75, 12);
  });
});
