/**
 * The acceptance record as a model, from `python/tests/acceptance/discovery_acceptance_bundle.json`
 * and design Section 15 of `docs/design/strategy_discovery_design.md`.
 *
 * The twenty-two criteria are a record, not an opinion: each is passed, failed or blocked with the
 * fixture that produced the entry, so the dashboard's only job is to read them without flattering
 * them. This module counts them, groups them, describes them one line at a time, and joins the four
 * failures to the missing piece each is carried by - and it derives the counts from the rows rather
 * than carrying a total, so the tally cannot drift from the table above it.
 *
 * A failure is never softened into a pass and a missing value is never filled with a default: the
 * four failing criteria are returned as they are, with the bundle's own `observed` and `defect` text
 * and the cause the page names.
 */

import { valueOf } from '../data/source.js';
import { FAILURES, STATUSES as STATUS_VOCABULARY } from '../data/acceptance.js';

/** The closed set of statuses a criterion may carry, read from the transcription of the schema. */
const STATUSES = valueOf(STATUS_VOCABULARY);

function countOf(rows, status) {
  return rows.filter((row) => row.status === status).length;
}

/**
 * The tally over whatever rows are handed in: how many the record holds, how many pass, fail and are
 * blocked, and whether every row carries a status from the closed set. `holds` is false when a row
 * carries a status the vocabulary does not name, which is the one way this dashboard could lie.
 */
export function summary(rows) {
  const passed = countOf(rows, 'passed');
  const failed = countOf(rows, 'failed');
  const blocked = countOf(rows, 'blocked');

  return {
    total: rows.length,
    passed,
    failed,
    blocked,
    holds: rows.every((row) => STATUSES.includes(row.status)),
    accounted: passed + failed + blocked,
  };
}

/**
 * The criteria that do not pass, in the record's own order. Statuses other than `failed` are included
 * deliberately: `blocked` is a legitimate answer whose dependency is named, so the dashboard shows
 * every row whose entry is not a pass rather than only the word `failed`.
 */
export function failing(rows) {
  return rows.filter((row) => row.status !== 'passed');
}

function milestoneOrder(label) {
  const match = /M(\d+)/.exec(String(label));
  return match ? Number(match[1]) : Number.POSITIVE_INFINITY;
}

/**
 * The criteria grouped by the milestone that owes them, in milestone order, with the tally per group.
 * A criterion owed by two milestones (`M1, M6`) is grouped under its label as the bundle writes it,
 * ordered by the first milestone it names.
 */
export function byMilestone(rows) {
  const groups = new Map();
  for (const row of rows) {
    if (!groups.has(row.milestone)) groups.set(row.milestone, []);
    groups.get(row.milestone).push(row);
  }

  return [...groups.entries()]
    .map(([milestone, items]) => ({
      milestone,
      order: milestoneOrder(milestone),
      ids: items.map((row) => row.id).sort((a, b) => a - b),
      total: items.length,
      passed: countOf(items, 'passed'),
      failed: countOf(items, 'failed'),
      blocked: countOf(items, 'blocked'),
    }))
    .sort((a, b) => (a.order === b.order ? (a.milestone < b.milestone ? -1 : 1) : a.order - b.order));
}

/**
 * One criterion as one line: its number, its requirement, its milestone and its status, all read from
 * the row. The dashboard prints this for a reader who wants the record as a list rather than a table,
 * and it never writes a status itself.
 */
export function describe(row) {
  return `Criterion ${row.id}. ${row.requirement} (${row.milestone}): ${row.status}`;
}

/**
 * The four failures with the missing piece behind each, joined to the criterion rows.
 *
 * The row supplies what the bundle records; the failure supplies the bundle's own `observed` text,
 * its `defect`, and the cause the page names. The join is on the criterion number, so a failure whose
 * row is missing from the record is reported as unjoined rather than silently dropped.
 */
export function explain(rows) {
  return FAILURES.map((failure) => {
    const id = valueOf(failure.id);
    const row = rows.find((item) => item.id === id);
    return {
      id,
      joined: row !== undefined,
      requirement: row ? row.requirement : null,
      milestone: row ? row.milestone : null,
      status: row ? row.status : null,
      fixture: row ? row.fixture : null,
      observed: valueOf(failure.observed),
      defect: valueOf(failure.defect),
      missing: valueOf(failure.missing),
    };
  });
}
