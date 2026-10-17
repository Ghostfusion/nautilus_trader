/**
 * Tests for the acceptance dashboard. The record has twenty-two passes and no failure, and the four
 * criteria that used to fail are 2, 4, 6 and 10; if the dashboard ever reported them as still failing
 * it would be reading a stale record, which is the one thing this view may not do.
 */

import { describe, expect, test } from 'bun:test';

import { valueOf } from '../src/data/source.js';
import { criteria } from '../src/data/layers.js';
import { COUNTS, FORMER_FAILURES } from '../src/data/acceptance.js';
import {
  byMilestone,
  describe as describeRow,
  explain,
  failing,
  summary,
} from '../src/engine/acceptance.js';

const ROWS = criteria();

describe('the criteria the dashboard reads', () => {
  test('are the twenty-two the record holds, numbered in order', () => {
    // A twenty-third criterion, or one dropped, would change the tally the view prints.
    expect(ROWS).toBeArrayOfSize(22);
    expect(ROWS.map((row) => row.id)).toEqual([
      1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
    ]);
  });

  test('carry a status from the closed vocabulary', () => {
    // A status outside passed/failed/blocked would make the summary unaccountable.
    for (const row of ROWS) expect(['passed', 'failed', 'blocked']).toContain(row.status);
  });
});

describe('the tally', () => {
  test('is twenty-two passed, none failed and none blocked', () => {
    // The acceptance criterion for this slice: the console reports the record, not a hopeful reading.
    expect(summary(ROWS)).toEqual({
      total: 22,
      passed: 22,
      failed: 0,
      blocked: 0,
      holds: true,
      accounted: 22,
    });
  });

  test('agrees with the count the bundle itself reports', () => {
    // The transcription and the computation would otherwise be two different numbers on one screen.
    const tally = summary(ROWS);
    expect([tally.total, tally.passed, tally.failed, tally.blocked]).toEqual([
      valueOf(COUNTS.total),
      valueOf(COUNTS.passed),
      valueOf(COUNTS.failed),
      valueOf(COUNTS.blocked),
    ]);
  });

  test('counts the rows it is handed rather than a stored total', () => {
    // The view filters, so the summary has to be over its input, not over the whole record.
    expect(summary(ROWS.slice(0, 5))).toEqual({
      total: 5,
      passed: 5,
      failed: 0,
      blocked: 0,
      holds: true,
      accounted: 5,
    });
  });

  test('does not hold when a row carries a status the vocabulary does not name', () => {
    // `holds` is the dashboard's own honesty flag, and this is the only way it can be false.
    const rogue = [...ROWS, { id: 23, requirement: 'Invented.', milestone: 'M7', status: 'maybe' }];
    expect(summary(rogue).holds).toBe(false);
  });
});

describe('the criteria that used to fail', () => {
  test('leave the failed list empty and are the four the record names', () => {
    // The record holds no failure now; a fifth former failure would be a different record.
    expect(failing(ROWS)).toEqual([]);
    expect(FORMER_FAILURES.map((failure) => valueOf(failure.id))).toEqual([2, 4, 6, 10]);
    for (const row of ROWS.filter((item) => [2, 4, 6, 10].includes(item.id))) {
      expect(row.status).toBe('passed');
    }
  });

  test('join to the rows the record holds, field for field', () => {
    // The closure table in the view is built from the transcription and the rows, so their fields
    // may not disagree: a stale requirement or milestone would be a page about a different record.
    const byId = new Map(ROWS.map((row) => [row.id, row]));
    for (const failure of FORMER_FAILURES) {
      const row = byId.get(valueOf(failure.id));
      expect(row.status).toBe('passed');
      expect(valueOf(failure.requirement)).toBe(row.requirement);
      expect(valueOf(failure.milestone)).toBe(row.milestone);
      expect(valueOf(failure.status)).toBe(row.status);
    }
  });

  test('each carry the bundle observed text and what closed them', () => {
    // A former failure with no closure recorded would tell the reader nothing about what changed.
    for (const row of explain(ROWS)) {
      expect(row.joined).toBe(true);
      expect(row.status).toBe('passed');
      expect(row.observed.length).toBeGreaterThan(0);
      expect(row.closed.length).toBeGreaterThan(0);
    }
  });

  test('name what closed each in the bundle own words', () => {
    // Each `closed` is the bundle's own defect text, which reads as the change rather than the symptom.
    const byId = new Map(explain(ROWS).map((row) => [row.id, row]));
    expect(byId.get(2).closed).toContain('closed by this change');
    expect(byId.get(4).closed).toContain('except for the bridge case it names');
    expect(byId.get(6).closed).toContain('the cash leg reconciles');
    expect(byId.get(10).closed).toContain('the recomputation agrees with nothing unavailable');
  });

  test('report a former failure whose criterion row is absent rather than dropping it', () => {
    // A join that silently drops a row would make the closure table look complete when it is not.
    const partial = explain(ROWS.filter((row) => row.id === 2));
    expect(partial).toBeArrayOfSize(4);
    expect(partial.map((row) => row.joined)).toEqual([true, false, false, false]);
    expect(partial[1].status).toBe(null);
  });

  test('are listed one line at a time from the row itself', () => {
    // `describe` reads the status from the row, so the view cannot print a status of its own.
    const two = ROWS.find((row) => row.id === 2);
    expect(describeRow(two)).toBe(
      'Criterion 2. A rerun is identical, and what is not deterministic is named. (M4): passed',
    );
  });
});

describe('the milestones', () => {
  test('group in milestone order with each group tallied', () => {
    // The dashboard's second reading: which milestone owes which criteria, and which owes a failure.
    const groups = byMilestone(ROWS);
    expect(groups.map((group) => group.milestone)).toEqual(['M1, M6', 'M2', 'M3', 'M4', 'M5', 'M6']);
    expect(groups.find((group) => group.milestone === 'M4')).toEqual({
      milestone: 'M4',
      order: 4,
      ids: [2, 5, 7, 8, 9, 10, 11, 18],
      total: 8,
      passed: 8,
      failed: 0,
      blocked: 0,
    });
  });

  test('account for every criterion exactly once', () => {
    // A criterion in no group, or in two, would mean the grouping lost part of the record.
    const groups = byMilestone(ROWS);
    const grouped = groups.flatMap((group) => group.ids).sort((a, b) => a - b);
    expect(grouped).toEqual(ROWS.map((row) => row.id));
    expect(groups.reduce((total, group) => total + group.total, 0)).toBe(22);
  });

  test('place both M2 criteria in their own group, both passing', () => {
    // M2 owes criteria 4 and 6, and both used to fail; the group now carries no failure.
    const m2 = byMilestone(ROWS).find((group) => group.milestone === 'M2');
    expect(m2).toEqual({
      milestone: 'M2',
      order: 2,
      ids: [4, 6],
      total: 2,
      passed: 2,
      failed: 0,
      blocked: 0,
    });
  });
});

describe('determinism', () => {
  test('returns the same readings for the same rows', () => {
    // The dashboard reads a fixed record, so two renders may not differ.
    expect(summary(ROWS)).toEqual(summary(criteria()));
    expect(byMilestone(ROWS)).toEqual(byMilestone(criteria()));
    expect(explain(ROWS)).toEqual(explain(criteria()));
  });
});
