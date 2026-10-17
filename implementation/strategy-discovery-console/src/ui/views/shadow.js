/**
 * The shadow view: one frozen candidate watched against a synthetic feed.
 *
 * Models `python/nautilus_trader/discovery/shadow.py`. The view compares a replay's declaration with
 * the shadow run's by value, feeds the page's worked example in one observation at a time, and
 * reports the counts the monitor reports - including the refusal of an observation whose fill
 * precedes its own signal, which is the look-ahead the stage exists to catch. Nothing here sends an
 * order, holds a client, opens a connection, or measures the candidate's edge: the report is about
 * integration and operational behaviour.
 */

import {
  LABEL,
  REFUSAL_CODES,
  TEMPORAL_CONTRACT,
  contractDigest,
  declaredFields,
  observe,
  parityCheck,
  shadowReport,
  watch,
} from '../../engine/shadow.js';
import { declaredSetsByName } from '../../data/genome.js';
import { MESSAGES, observationFields, workedExample, workedTotals } from '../../data/shadow.js';

/** What this view models: the shared contract, the record, and the counts a watch reports. */
export const meta = {
  id: 'shadow',
  title: 'The shadow period: watching a frozen candidate against a live feed',
  order: 10,
  tutorial: 'tutorials/strategy-discovery/the-shadow-period/README.md',
};

/** The unit the page writes its worked example in, and the unit the code stores instants in. */
const NS = 1e9;

/** Grade a layer's own paragraph into the tone the verdict is shown with. */
function gradeTone(grade) {
  if (grade.startsWith('Strong')) return 'ok';
  if (grade.startsWith('Mixed')) return 'warn';
  return 'bad';
}

/** Build the view into the empty root, and wire every control the model needs. */
export function mount(root, ctx) {
  const { ui, data } = ctx;
  const layer = data.layer(meta.id);
  const sets = declaredSetsByName();
  const example = workedExample();
  const totals = workedTotals();
  const fields = observationFields();

  const state = {
    minimum: totals.minimumObservations,
    late: false,
    lookAhead: false,
    eligibility: TEMPORAL_CONTRACT.execution_eligibility,
    demonstration: null,
  };

  /** The feed the view runs: the page's five rows, with the reader's own two changes applied. */
  const feed = () =>
    example.map((row, index) => ({
      signal_at: row.signalSeconds * NS + (state.late && index === 1 ? NS : 0),
      intended_entry: row.intendedEntry,
      eligible_fill_at: row.eligibleFillSeconds * NS,
      fill_price: row.fillPrice,
      modelled_cost: row.modelledCost,
      observed_cost: row.observedCost,
      non_fill_reason: row.nonFillReason,
    }));

  const plan = () => ({
    candidate_id: 'frozen-candidate',
    specification_digest: 'sha256:the-frozen-specification',
    period: '2024-01-01 to 2024-02-01',
    minimum_observations: state.minimum,
  });

  const expected = () => example.map((row) => row.signalSeconds * NS);

  // --- block two: the model ------------------------------------------------

  const results = ui.h('div', null);
  const demonstration = ui.h('div', null);

  /** Show what the last demonstration's call returned. */
  const showDemonstration = () => {
    ui.clear(demonstration);
    demonstration.appendChild(
      state.demonstration
        ? ui.verdict(
            state.demonstration.label.replaceAll('_', ' '),
            state.demonstration.outcome.ok === false
              ? String(state.demonstration.outcome.code).replaceAll('_', ' ')
              : 'not refused',
            state.demonstration.outcome.ok === false ? 'bad' : 'warn',
            state.demonstration.outcome.reason ??
              'The monitor produced a report where this demonstration expected a refusal.',
          )
        : ui.note('Run a demonstration to see the refusal it reports.'),
    );
  };

  const minimumInput = ui.numberInput({
    id: 'shadow-minimum',
    value: state.minimum,
    min: 0,
    step: 1,
    onInput: (value) => {
      if (!Number.isFinite(value)) return;
      state.minimum = value;
      render();
    },
  });
  const lateToggle = ui.toggle({
    id: 'shadow-late',
    checked: state.late,
    label: 'deliver the second signal one second late',
    onChange: (checked) => {
      state.late = checked;
      render();
    },
  });
  const lookAheadToggle = ui.toggle({
    id: 'shadow-lookahead',
    checked: state.lookAhead,
    label: 'record a fill one second before its own signal',
    onChange: (checked) => {
      state.lookAhead = checked;
      render();
    },
  });
  const eligibilitySelect = ui.selectInput({
    id: 'shadow-eligibility',
    value: state.eligibility,
    options: sets.execution_eligibilities,
    onChange: (value) => {
      state.eligibility = value;
      render();
    },
  });

  const contractBlock = () =>
    ui.stack(
      ui.table({
        columns: ['declaration', 'the run declares'],
        rows: declaredFields(TEMPORAL_CONTRACT).map(([name, value]) => [name, value]),
        dense: true,
        caption: `The contract\u2019s digest is ${contractDigest(TEMPORAL_CONTRACT).slice(0, 19)}\u2026 over its four declared fields.`,
      }),
      ui.note(
        'In the repository the four declarations are read off the frozen candidate\u2019s own data block; here they are taken from the policy\u2019s declared sets, which is the same path a candidate\u2019s values resolve through.',
        'muted',
      ),
    );

  const parityBlock = () => {
    const instants = [0, 86400 * NS];
    const replayReading = {
      contract: TEMPORAL_CONTRACT,
      decision_at: instants[0],
      eligible_fill_at: instants[1],
    };
    const shadowReading = {
      contract: { ...TEMPORAL_CONTRACT, execution_eligibility: state.eligibility },
      decision_at: instants[0],
      eligible_fill_at: instants[1],
    };
    const check = parityCheck(replayReading, shadowReading);
    return ui.stack(
      ui.row(
        ui.field(
          'the shadow run declares',
          eligibilitySelect,
          'the replay declares ' + TEMPORAL_CONTRACT.execution_eligibility,
        ),
      ),
      ui.metrics(
        ui.metric('same declarations', check.same_fields ? 'yes' : 'no', 'compared by value, not by name', check.same_fields ? 'ok' : 'bad'),
        ui.metric('same pair of instants', check.same_pair ? 'yes' : 'no', `${instants[0]} against ${instants[0]}, ${instants[1]} against ${instants[1]}`, check.same_pair ? 'ok' : 'bad'),
        ui.metric('holds', check.holds ? 'yes' : 'no', 'true only when both halves hold', check.holds ? 'ok' : 'bad'),
      ),
      ui.verdict('the comparison', check.holds ? 'shared contract' : 'not comparable', check.holds ? 'ok' : 'warn', check.detail),
      ui.note(
        'A run declaring a different execution eligibility is a different contract however similar the timestamps look; only the differing-field list says so.',
        'muted',
      ),
    );
  };

  const feedBlock = () => {
    const observations = feed();
    if (state.lookAhead) {
      observations[2] = {
        ...observations[2],
        eligible_fill_at: observations[2].signal_at - NS,
        fill_price: 25,
        non_fill_reason: '',
      };
    }
    let state_ = watch({ plan: plan(), expectedSignalTimes: expected() });
    let refused = null;
    for (const observation of observations) {
      const step = observe(state_, observation);
      if (!step.ok) {
        refused = step;
        break;
      }
      state_ = step.state;
    }
    if (refused) {
      return ui.stack(
        ui.verdict('the feed was refused', refused.code.replaceAll('_', ' '), 'bad', refused.reason),
        ui.note(
          'A fill that precedes its own signal is not a divergence but a broken record: the run used a price it could not have had at the instant it decided, so the monitor refuses it rather than counting it.',
        ),
        observationTable(observations, false),
      );
    }
    const report = shadowReport({ ...state_, plan: plan() });
    if (report.ok === false) {
      return ui.stack(
        ui.verdict(
          'the period was refused',
          String(report.code).replaceAll('_', ' '),
          'bad',
          report.reason,
        ),
        ui.note(
          'The monitor refuses rather than reports when the period cannot be run as declared; a divergence inside a run period it reports instead.',
        ),
        observationTable(observations, false),
      );
    }
    return ui.stack(
      ui.metrics(
        ui.metric('eligible decisions', report.eligible_decisions, 'instants the replay was expected to decide at'),
        ui.metric('observed decisions', report.observed_decisions, 'observations the feed delivered'),
        ui.metric('reached', report.reached, `declared minimum ${report.minimum_observations}`, report.readable ? 'ok' : 'bad'),
        ui.metric('missing observations', report.missing_observations, 'expected instants never delivered', report.missing_observations ? 'warn' : null),
        ui.metric('discrepancies', report.signal_discrepancies.length, 'a signal that is not an expected instant, or a duplicate'),
        ui.metric('non-fills', report.non_fills.length, 'an intended entry that did not fill'),
        ui.metric('modelled cost', report.modelled_cost, 'the recorded cost configuration'),
        ui.metric('observed cost', report.observed_cost, 'what the observed prices and quotes imply'),
        ui.metric('cost difference', report.cost_difference, 'outside the diverged test', report.cost_difference ? 'warn' : null),
      ),
      ui.verdict(
        'the period',
        report.diverged ? 'diverged' : 'not diverged',
        report.diverged ? 'warn' : 'ok',
        report.readable ? 'reached the count declared before it started' : report.unreadable_reason,
      ),
      ui.note(
        `Labelled ${report.label}: a simulated fill is never an observed one. No divergence is not a pass, and no field here is a performance statistic.`,
        report.diverged ? 'warn' : 'muted',
      ),
      ui.group('what the report found', report.findings.length > 0 ? ui.list(report.findings) : ui.note('Nothing diverged from the replay\u2019s expectation.')),
      ui.group('the record', observationTable(report.observations, true)),
    );
  };

  /** The seven-field record, as the report carries it. */
  function observationTable(observations, withFilled) {
    const columns = ['#', ...fields, ...(withFilled ? ['filled'] : [])];
    return ui.table({
      columns,
      rows: observations.map((observation, index) => [
        index + 1,
        ...fields.map((name) => {
          const value = observation[name];
          if (value === null || value === undefined) return '';
          if (name === 'non_fill_reason') return value;
          if (name === 'intended_entry') return value ? 'yes' : 'no';
          if (name === 'signal_at' || name === 'eligible_fill_at') return String(value);
          return String(value);
        }),
        ...(withFilled ? [observation.filled ? ui.badge('filled', 'ok') : ui.badge('no fill', 'muted')] : []),
      ]),
      dense: true,
      caption:
        'Instants are whole nanoseconds, as the code stores them; the page writes its example in seconds. The seven fields include both the reason a fill did not happen and the observed cost.',
    });
  }

  // --- block three: what it refuses ---------------------------------------

  const demonstrations = () => {
    const demoRows = [
      {
        code: 'minimum_not_declared',
        condition: 'the plan\u2019s declared observation count is below one',
        run: () =>
          shadowReport({
            plan: { ...plan(), minimum_observations: 0 },
            contract: TEMPORAL_CONTRACT,
            expected_signal_times: expected(),
            observations: feed(),
          }),
      },
      {
        code: 'eligible_before_signal',
        condition: 'an observation is eligible to fill before its own signal',
        run: () => {
          const state_ = watch({ plan: plan(), expectedSignalTimes: expected() });
          return observe(state_, { ...feed()[0], eligible_fill_at: feed()[0].signal_at - NS });
        },
      },
      {
        code: 'non_fill_without_reason',
        condition: 'an intended entry did not fill and gives no reason',
        run: () =>
          shadowReport({
            plan: plan(),
            contract: TEMPORAL_CONTRACT,
            expected_signal_times: expected(),
            observations: [{ ...feed()[2], non_fill_reason: '' }],
          }),
      },
    ];
    return ui.stack(
      ui.note(
        'The monitor refuses rather than reports when a record is malformed; a divergence is reported instead. Both are shown here.',
        'muted',
      ),
      ui.table({
        columns: ['code', 'what produces it', ''],
        rows: demoRows.map((row) => [
          ui.badge(row.code, 'warn'),
          row.condition,
          ui.button('demonstrate', () => {
            state.demonstration = { label: row.code, outcome: row.run() };
            showDemonstration();
          }),
        ]),
        dense: true,
      }),
      demonstration,
      ui.group(
        'the refusal codes, in the order the source declares them',
        ui.table({
          columns: ['#', 'code', 'the message it carries'],
          rows: REFUSAL_CODES.map((code, index) => [
            index + 1,
            ui.badge(code, 'warn'),
            MESSAGES[{
              minimum_not_declared: 'minimumNotDeclared',
              eligible_before_signal: 'eligibleBeforeSignal',
              non_fill_without_reason: 'nonFillWithoutReason',
            }[code]].value.message,
          ]),
          dense: true,
          caption: `Reported as "${MESSAGES.errorPrefix.value}".`,
        }),
      ),
      ui.group(
        'the lines the report writes',
        ui.list(
          Object.entries(MESSAGES)
            .filter(([key]) => key.endsWith('Finding'))
            .map(([, node]) => node.value),
        ),
      ),
    );
  };

  // --- the view ------------------------------------------------------------

  function render() {
    ui.clear(results);
    const report = state.lookAhead
      ? null
      : shadowReport({
          plan: plan(),
          contract: TEMPORAL_CONTRACT,
          expected_signal_times: expected(),
          observations: feed(),
        });
    const readableNote =
      report && report.ok
        ? `Readable: ${report.readable ? 'the period reached its declared count, which is a precondition and evidence of nothing' : report.unreadable_reason}.`
        : null;
    results.appendChild(
      ui.stack(
        ui.metrics(
          ui.metric('the candidate', plan().candidate_id, 'frozen: its rules may not change'),
          ui.metric('the label', LABEL, 'every shadow result carries it'),
          ui.metric('declared minimum', state.minimum, 'set before the period started', state.minimum >= 1 ? null : 'bad'),
          ui.metric('the page\u2019s example', `${example.length} rows`, `modelled ${totals.modelledCost}, observed ${totals.observedCost}, difference ${totals.costDifference}`),
        ),
        ui.group('the declared contract', contractBlock()),
        ui.group('parity, by value rather than by name', parityBlock()),
        ui.group(
          'the feed',
          ui.row(
            ui.field('declared minimum observations', minimumInput, 'below one the report is refused, not run'),
            lateToggle,
            lookAheadToggle,
          ),
          readableNote ? ui.note(readableNote) : null,
          feedBlock(),
        ),
        ui.group('what the page computes from the same five rows', ui.table({
          columns: ['count', 'the page\u2019s total'],
          rows: [
            ['eligible_decisions', totals.eligibleDecisions],
            ['observed_decisions', totals.observedDecisions],
            ['reached', totals.reached],
            ['missing_observations', totals.missingObservations],
            ['signal_discrepancies', totals.signalDiscrepancies],
            ['non_fills', totals.nonFills],
            ['modelled_cost', totals.modelledCost],
            ['observed_cost', totals.observedCost],
            ['cost_difference', totals.costDifference],
            ['diverged', String(totals.diverged)],
          ],
          dense: true,
          aligns: ['text', 'num'],
        })),
      ),
    );
  }

  root.appendChild(
    ui.panel(
      { title: layer.title, subtitle: layer.tutorial, id: 'shadow' },
      ui.block(
        'The idea',
        ui.quote(layer.idea, layer.tutorial),
        ui.verdict('grade', layer.grade.split(':')[0], gradeTone(layer.grade), layer.grade),
      ),
      ui.block('The model', results),
      ui.block('What it refuses', demonstrations()),
      ui.block('Where it goes wrong', ui.list(layer.caveats)),
      ui.block(
        'Where this came from',
        ui.group(
          'the acceptance criteria this layer is named by',
          ui.table({
            columns: ['#', 'requirement', 'status', 'fixture'],
            rows: data
              .criteria()
              .filter((row) => layer.criteria.includes(row.id))
              .map((row) => [
                row.id,
                row.requirement,
                row.status === 'passed' ? ui.badge('passed', 'ok') : ui.badge(row.status, 'bad'),
                row.fixture,
              ]),
            dense: true,
            caption:
              'Both criteria this layer rests on pass: the shared temporal contract and reporting divergence rather than summarising it.',
          }),
        ),
        ui.sources(layer.sources),
        ui.sources([
          { label: 'the contract, the record and the counts', path: 'src/data/shadow.js' },
          { label: 'the monitor', path: 'src/engine/shadow.js' },
        ]),
        ui.note(
          `The shadow half is fed observed data rather than bound to a live adapter, so the report cannot show the feed\u2019s own latencies, drops or re-connections.`,
          'muted',
        ),
      ),
    ),
  );
  showDemonstration();
  render();
}
