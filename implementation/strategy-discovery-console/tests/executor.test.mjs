import { describe, expect, test } from 'bun:test';

import {
  DECISIONS,
  REFUSAL_CODES,
  SUBMISSION_DEFERS,
  TIMING_CAVEAT,
  allocate,
  recordKinds,
} from '../src/engine/executor.js';
import {
  allocationDecisions,
  closedSets,
  recordKinds as declaredRecordKinds,
} from '../src/data/executor.js';

const EXAMPLE = {
  positionCount: 3,
  maxPositionWeight: 0.25,
  weighting: 'rank_proportional',
  cashPolicy: 'leave_idle',
};

describe('the worked example', () => {
  test('the rows are 0.5000, 0.3333 and 0.1667 with the 0.25 cap and strengths 1.0000, 1.0000, 0.6667', () => {
    // Breaks the page's four-row table if the shares, normalisation or cap arithmetic move.
    const result = allocate(['A', 'B', 'C'], EXAMPLE);
    expect(result.ok).toBe(true);
    expect(result.rows.map((row) => row.instrument)).toEqual(['A', 'B', 'C']);
    expect(result.rows.map((row) => row.share)).toEqual([3, 2, 1]);
    expect(result.rows[0].rawWeight).toBeCloseTo(0.5, 6);
    expect(result.rows[1].rawWeight).toBeCloseTo(1 / 3, 6);
    expect(result.rows[2].rawWeight).toBeCloseTo(1 / 6, 6);
    expect(result.rows[0].cappedWeight).toBeCloseTo(0.25, 6);
    expect(result.rows[1].cappedWeight).toBeCloseTo(0.25, 6);
    expect(result.rows[2].cappedWeight).toBeCloseTo(1 / 6, 6);
    expect(result.rows[0].instructionStrength).toBeCloseTo(1, 6);
    expect(result.rows[1].instructionStrength).toBeCloseTo(1, 6);
    expect(result.rows[2].instructionStrength).toBeCloseTo(0.6667, 4);
  });

  test('leave_idle leaves 0.6667 invested and 0.3333 idle when the cap binds', () => {
    // Breaks the residual figure if the position cap redistributes.
    const result = allocate(['A', 'B', 'C'], EXAMPLE);
    expect(result.invested).toBeCloseTo(2 / 3, 6);
    expect(result.idle).toBeCloseTo(1 / 3, 6);
  });

  test('reallocate moves C only as far as its headroom, so 0.7500 is invested and 0.2500 stays idle', () => {
    // Breaks the reallocation example if the residual is offered uncapped.
    const result = allocate(['A', 'B', 'C'], { ...EXAMPLE, cashPolicy: 'reallocate' });
    expect(result.rows[2].cappedWeight).toBeCloseTo(0.25, 6);
    expect(result.invested).toBeCloseTo(0.75, 6);
    expect(result.idle).toBeCloseTo(0.25, 6);
  });
});

describe('the declared decisions', () => {
  test('equal weighting gives every selected instrument the same raw weight', () => {
    const result = allocate(['A', 'B', 'C'], { ...EXAMPLE, weighting: 'equal' });
    for (const row of result.rows) expect(row.rawWeight).toBeCloseTo(1 / 3, 6);
  });

  test('the portfolio size keeps only the ranked leaders and does not pad a short selection', () => {
    const truncated = allocate(['A', 'B', 'C', 'D'], { ...EXAMPLE, positionCount: 2 });
    expect(truncated.rows.map((row) => row.instrument)).toEqual(['A', 'B']);
    const short = allocate(['A'], { ...EXAMPLE, positionCount: 3 });
    expect(short.rows.map((row) => row.instrument)).toEqual(['A']);
  });

  test('a repeated instrument is weighted once, at its first position', () => {
    const result = allocate(['A', 'B', 'A'], EXAMPLE);
    expect(result.rows.map((row) => row.instrument)).toEqual(['A', 'B']);
    expect(result.rows[0].share).toBe(2);
  });

  test('the position cap clamps each weight and redistributes nothing', () => {
    const result = allocate(['A', 'B', 'C'], EXAMPLE);
    const total = result.rows.reduce((sum, row) => sum + row.cappedWeight, 0);
    expect(total).toBeCloseTo(2 / 3, 6);
    expect(result.rows[2].cappedWeight).toBeLessThan(0.25);
  });

  test('the gross exposure cap scales every weight down proportionally', () => {
    const result = allocate(['A', 'B'], { positionCount: 2, maxPositionWeight: 1, weighting: 'equal', cashPolicy: 'leave_idle', maxGrossExposure: 0.5 });
    for (const row of result.rows) expect(row.cappedWeight).toBeCloseTo(0.25, 6);
    expect(result.invested).toBeCloseTo(0.5, 6);
  });

  test('the sector cap scales a two-name sector down to the cap', () => {
    const result = allocate(['A', 'B'], {
      positionCount: 2, maxPositionWeight: 1, weighting: 'equal', cashPolicy: 'leave_idle',
      maxSectorWeight: 0.4, sectorOf: { A: 'tech', B: 'tech' },
    });
    expect(result.rows[0].cappedWeight).toBeCloseTo(0.2, 6);
    expect(result.rows[1].cappedWeight).toBeCloseTo(0.2, 6);
  });

  test('an ineligible holding is flattened only when the rule declares it', () => {
    const flatten = allocate(['A', 'B'], {
      ...EXAMPLE, held: ['B'], ineligible: ['B'], flattenIneligible: true,
    });
    expect(flatten.targets.find((target) => target.instrument === 'B').weight).toBe(0);
    const leave = allocate(['A', 'B'], { ...EXAMPLE, held: ['B'], ineligible: ['B'], flattenIneligible: false });
    expect(leave.targets.some((target) => target.instrument === 'B')).toBe(false);
  });

  test('a held instrument the selection no longer names is flattened at zero weight', () => {
    const result = allocate(['A'], { ...EXAMPLE, held: ['Z'] });
    expect(result.targets.find((target) => target.instrument === 'Z').weight).toBe(0);
  });

  test('an unknown weighting or cash policy is refused rather than defaulted', () => {
    // Breaks the closed-vocabulary claim if a bad value silently picks a default.
    const badWeighting = allocate(['A'], { ...EXAMPLE, weighting: 'momentum' });
    expect(badWeighting.ok).toBe(false);
    expect(badWeighting.code).toBe('weighting_not_declared');
    expect(badWeighting.reason).toContain('equal, rank_proportional');
    const badCash = allocate(['A'], { ...EXAMPLE, cashPolicy: 'spend' });
    expect(badCash.code).toBe('cash_policy_not_declared');
  });

  test('two allocations of the same input are identical', () => {
    // Breaks the allocation-determinism claim (criterion 21) if any state leaks between runs.
    expect(allocate(['A', 'B', 'C'], EXAMPLE)).toEqual(allocate(['A', 'B', 'C'], EXAMPLE));
  });
});

describe('the transcription and the model agree', () => {
  test('there are exactly nine allocation decisions, in data order', () => {
    expect(DECISIONS().map((decision) => decision.n)).toEqual([1, 2, 3, 4, 5, 6, 7, 8, 9]);
    expect(DECISIONS()).toEqual(allocationDecisions());
  });

  test('the record kinds match the data module and CrossSectionEntry keeps its field order', () => {
    expect(recordKinds()).toEqual(declaredRecordKinds());
    const crossSection = recordKinds().find((record) => record.name === 'CrossSectionEntry');
    expect(crossSection.fields).toEqual(['instrument_id', 'alias', 'value', 'observed_at', 'available_at', 'member']);
  });

  test('the refusal vocabulary is the five declared interpreter refusals', () => {
    expect(REFUSAL_CODES).toEqual([
      'cash_policy_not_declared',
      'selection_mode_not_declared',
      'rebalance_frequency_not_declared',
      'weighting_not_declared',
      'signal_frequency_not_declared',
    ]);
  });

  test('the closed vocabularies keep their declared members and order', () => {
    const sets = closedSets().map((set) => [set.name, set.members]);
    expect(sets).toContainEqual(['weightings', ['equal', 'rank_proportional']]);
    expect(sets).toContainEqual(['cash policies', ['leave_idle', 'reallocate']]);
    expect(sets).toContainEqual(['rebalance frequencies', ['daily', 'weekly', 'monthly']]);
  });

  test('the bridge does not defer, and the timing caveat is kept visible', () => {
    // Breaks the honesty rule if the failing timing criterion (4) is presented as enforced.
    expect(SUBMISSION_DEFERS).toBe(false);
    expect(TIMING_CAVEAT).toContain('submits it without waiting');
  });
});
