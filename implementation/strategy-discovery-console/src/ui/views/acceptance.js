/**
 * The acceptance record: twenty-two criteria, all twenty-two passing, none failed and none blocked.
 *
 * This view models `python/tests/acceptance/discovery_acceptance_bundle.json` and design Section 15 of
 * `docs/design/strategy_discovery_design.md`. It is the one page in the console that is not about a
 * mechanism: it shows the whole record, names the four criteria that used to fail together with what
 * closed each, and prints the canonical credential list the boundary is read against.
 *
 * Every status on screen comes from `ctx.data.criteria()` through `../engine/acceptance.js`, and every
 * record of a closure comes from `../data/acceptance.js`. Nothing here writes a status by hand: a view
 * that could decide a status could also flatter one.
 */

import { valueOf } from '../../data/source.js';
import { COUNTS, RULES, SHARED_CAUSE } from '../../data/acceptance.js';
import { CREDENTIALS, CREDENTIALS_PATH } from '../../data/boundary.js';
import { byMilestone, describe as describeRow, explain, failing, summary } from '../../engine/acceptance.js';

export const meta = {
  id: 'acceptance',
  title: 'The acceptance record',
  order: 90,
  tutorial: 'python/tests/acceptance/discovery_acceptance_bundle.json',
};

function toneOf(status) {
  if (status === 'passed') return 'ok';
  if (status === 'failed') return 'bad';
  if (status === 'blocked') return 'warn';
  return 'muted';
}

function statusCell(ui, status) {
  return ui.badge(status, toneOf(status));
}

export function mount(root, ctx) {
  const ui = ctx.ui;
  const rows = ctx.data.criteria();
  const credentials = ctx.data.credentials();
  const tally = summary(rows);
  const failed = failing(rows);
  const former = explain(rows);

  // ---- 1. The idea ----------------------------------------------------------------------------

  root.appendChild(
    ui.panel(
      {
        title: 'The idea',
        subtitle: 'An entry is produced by running the test, never by reading the document',
        tone: 'accent',
      },
      ui.quote(valueOf(RULES.reporting), 'docs/design/strategy_discovery_design.md section 15'),
      ui.quote(valueOf(RULES.entry), 'docs/design/strategy_discovery_design.md section 15.1'),
      ui.quote(valueOf(COUNTS.sentence), 'tutorials/strategy-discovery/the-boundary/README.md'),
      ui.note(
        `The bundle validates ${tally.total} entries against its schema and records each one's ` +
          'identity, so a criterion cannot be satisfied by the existence of a design section. The ' +
          `entry this page reads is ${meta.tutorial}: all ${tally.passed} pass, and the four that ` +
          'used to fail are recorded there as closed.',
      ),
    ),
  );

  // ---- 2. The model: the tally and the twenty-two rows -----------------------------------------

  const groups = byMilestone(rows);

  root.appendChild(
    ui.panel(
      { title: 'The model', subtitle: 'The record, counted and read back' },
      ui.metrics(
        ui.metric('Entries', tally.total, 'the whole list', 'accent'),
        ui.metric('Passed', tally.passed, 'with their identities', 'ok'),
        ui.metric('Failed', tally.failed, 'none remains', 'ok'),
        ui.metric('Blocked', tally.blocked, 'a legitimate status, used by none', 'muted'),
      ),
      ui.note(
        `${valueOf(COUNTS.bundleObserved)}. The tally above is computed over the rows below, not ` +
          'stored beside them, so a status changed in the transcription changes the count.',
      ),

      ui.block(
        'The twenty-two criteria',
        ui.table({
          dense: true,
          columns: ['#', 'Requirement', 'Milestone', 'Status', 'Fixture', 'The observed line'],
          rows: rows.map((row) => [
            row.id,
            row.requirement,
            row.milestone,
            statusCell(ui, row.status),
            row.fixture,
            row.observed,
          ]),
        }),
      ),

      ui.block(
        'The same twenty-two, by milestone',
        ui.table({
          dense: true,
          columns: ['Milestone', 'Criteria', 'Passed', 'Failed', 'Blocked'],
          rows: groups.map((group) => [
            group.milestone,
            group.ids.join(', '),
            group.passed,
            group.failed,
            group.blocked,
          ]),
        }),
      ),

      ui.block(
        'One line each, read from the row',
        ui.list(rows.map(describeRow), { ordered: true, className: 'reasons' }),
      ),
    ),
  );

  // ---- 3. The four criteria that used to fail -------------------------------------------------

  root.appendChild(
    ui.panel(
      {
        title: 'The four criteria that used to fail, and what closed each',
        subtitle: 'The list of failed criteria is empty; these four are recorded as closed',
        tone: 'ok',
      },
      ui.note(
        `The list of failed criteria is empty: ${failed.length} of ${tally.total} remain. The four ` +
          'below are the criteria that used to fail, each with the bundle\'s own observed line and ' +
          'the defect it records against the criterion, which now reads as what closed the failure ' +
          'rather than as what is absent. The console has no default value to fall back on and ' +
          'makes none.',
      ),
      ...former.map((failure) =>
        ui.group(
          failure.joined
            ? `Criterion ${failure.id}. ${failure.requirement}`
            : `Criterion ${failure.id}`,
          ui.row(
            statusCell(ui, failure.status),
            ui.badge(failure.milestone ?? 'milestone unknown', 'muted'),
            failure.fixture ? ui.badge(failure.fixture, 'muted') : null,
          ),
          ui.note(failure.observed),
          ui.note(`The bundle records as what closed it: ${failure.closed}`, 'ok'),
        ),
      ),
      ui.note(valueOf(SHARED_CAUSE), 'ok'),
    ),
  );

  // ---- 4. The credential list the boundary is read against -------------------------------------

  root.appendChild(
    ui.panel(
      {
        title: 'The canonical credential list',
        subtitle: 'What the environment assertion is read against, counted',
      },
      ui.metrics(
        ui.metric(
          'Names',
          credentials.length,
          `transcribed as ${valueOf(CREDENTIALS.count)}`,
          'accent',
        ),
        ui.metric(
          'The floor the fixture asserts',
          valueOf(CREDENTIALS.floor),
          'the list holds more than this many',
          'muted',
        ),
        ui.metric('First', valueOf(CREDENTIALS.first), 'in the script\u2019s own order', 'muted'),
        ui.metric('Last', valueOf(CREDENTIALS.last), 'in the script\u2019s own order', 'muted'),
      ),
      ui.note(
        `${valueOf(CREDENTIALS.countSentence)}. The list is read out of ${CREDENTIALS_PATH}, so the ` +
          'fixture cannot drift from what the repository unsets; the console prints the same names ' +
          'in the same order.',
      ),
      ui.table({
        dense: true,
        columns: ['#', 'Name'],
        rows: credentials.map((name, index) => [index + 1, name]),
      }),
      ui.note(
        'Three of these are asserted by name, and the assertion also reads the list rather than ' +
          'restating it: EODHD_API_KEY is set on the machine this work was written on, and the ' +
          'fixture reports that ambient case rather than failing on it.',
      ),
    ),
  );

  // ---- 5. Where this came from -----------------------------------------------------------------

  root.appendChild(
    ui.panel(
      { title: 'Where this came from', subtitle: 'The files this view reads' },
      ui.sources([
        { label: 'discovery_acceptance_bundle.json', path: meta.tutorial },
        { label: 'strategy_discovery_design.md', path: 'docs/design/strategy_discovery_design.md' },
        { label: 'strip-adapter-env.bash', path: CREDENTIALS_PATH },
        { label: 'the-boundary/README.md', path: 'tutorials/strategy-discovery/the-boundary/README.md' },
      ]),
      ui.note(
        'The criteria themselves are transcribed in src/data/layers.js and read here through ' +
          'data.criteria(); the four criteria that used to fail, and what closed each, are ' +
          'transcribed in src/data/acceptance.js. This view computes the tally and the grouping ' +
          'over those rows and restates neither.',
      ),
    ),
  );
}
