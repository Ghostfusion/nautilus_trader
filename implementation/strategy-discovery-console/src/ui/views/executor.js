/**
 * The one executor: handing a compiled candidate to the engine.
 *
 * This view models `python/nautilus_trader/discovery/strategy.py` and the page
 * `tutorials/strategy-discovery/one-executor/README.md`. A reader sets a ranking length, a portfolio
 * size and a per-position cap, and sees the allocation table and the instruction strengths the nine
 * declared decisions produce - the worked example number by number. It also shows the interpreter's
 * decision vocabulary, the record kinds a run writes, the difference between a decision bar and a
 * rebalance bar, the declared signal-frequency scope, and the bridge's timing caveat.
 */

import { allocate, DECISIONS, recordKinds, REFUSAL_CODES, TIMING_CAVEAT } from '../../engine/executor.js';
import {
  FIRST_RELEASE_SCOPE,
  SIGNAL_SCOPE,
  closedSets,
  rebalancePeriods,
} from '../../data/executor.js';

export const meta = {
  id: 'executor',
  title: 'The one executor: handing a compiled candidate to the engine',
  order: 4,
  tutorial: 'tutorials/strategy-discovery/one-executor/README.md',
};

const LETTERS = 'ABCDEFGHIJKLMNOPQRSTUVWXYZ';

/** The refusal messages are long in the source; the view shows the declared set each names. */
const REFUSAL_TEXTS = {
  cash_policy_not_declared: 'cash_policy must be one of leave_idle, reallocate, was the declared value',
  selection_mode_not_declared: 'selection mode must be one of rank, threshold, none, was the declared value',
  rebalance_frequency_not_declared: 'rebalance frequency must be one of daily, weekly, monthly, was the declared value',
  weighting_not_declared: 'weighting must be one of equal, rank_proportional, was the declared value',
  signal_frequency_not_declared: 'signal frequency must be one of the declared table, was the declared value',
};

function instrumentName(index) {
  return LETTERS[index] ?? `N${index + 1}`;
}

function fixed(value) {
  return value === null || value === undefined ? 'n/a' : value.toFixed(4);
}

export function mount(root, ctx) {
  const { ui, data } = ctx;
  const layer = data.layer(meta.id);

  const state = {
    length: 3,
    positionCount: 3,
    cap: 0.25,
    weighting: 'rank_proportional',
    cashPolicy: 'leave_idle',
  };

  root.appendChild(ui.block('The idea',
    ui.quote(layer.idea, layer.tutorial),
    ui.metrics(
      ui.metric('Grade', layer.grade),
      ui.metric('Tutorial', layer.tutorial),
    ),
  ));

  const modelHost = ui.block('The model');
  const refusalHost = ui.block('What it refuses');
  root.appendChild(modelHost);
  root.appendChild(refusalHost);

  root.appendChild(ui.block('Where it goes wrong', ui.list(layer.caveats)));
  root.appendChild(ui.block('Where this came from', ui.sources(layer.sources)));

  function render() {
    ui.clear(modelHost);
    const selection = Array.from({ length: state.length }, (_, index) => instrumentName(index));
    const result = allocate(selection, {
      positionCount: state.positionCount,
      maxPositionWeight: state.cap,
      weighting: state.weighting,
      cashPolicy: state.cashPolicy,
    });

    modelHost.appendChild(ui.field('Ranking length', ui.numberInput({
      id: 'executor-length', value: state.length, min: 1, max: 12, step: 1,
      onInput: (value) => { state.length = Math.min(Math.max(Math.round(value), 1), 12); render(); },
    }), 'How many instruments the ranking produces, in rank order.'));

    modelHost.appendChild(ui.field('Portfolio size', ui.numberInput({
      id: 'executor-position-count', value: state.positionCount, min: 0, max: 12, step: 1,
      onInput: (value) => { state.positionCount = Math.min(Math.max(Math.round(value), 0), 12); render(); },
    }), 'The pre-trade cap: the first names the allocation weights; a shorter selection is not padded.'));

    modelHost.appendChild(ui.field('Position cap', ui.numberInput({
      id: 'executor-cap', value: state.cap, min: 0, max: 1, step: 0.05,
      onInput: (value) => { state.cap = Math.min(Math.max(value, 0), 1); render(); },
    }), 'The largest share of the account one holding may take; the cap clamps and redistributes nothing.'));

    modelHost.appendChild(ui.field('Weighting', ui.selectInput({
      id: 'executor-weighting', value: state.weighting, options: ['equal', 'rank_proportional'],
      onChange: (value) => { state.weighting = value; render(); },
    }), 'equal gives one share each; rank_proportional gives the i-th name count - i.'));

    modelHost.appendChild(ui.field('Cash policy', ui.selectInput({
      id: 'executor-cash-policy', value: state.cashPolicy, options: ['leave_idle', 'reallocate'],
      onChange: (value) => { state.cashPolicy = value; render(); },
    }), 'leave_idle leaves the residual uninvested; reallocate offers it within the caps.'));

    if (result.ok) {
      modelHost.appendChild(ui.table({
        columns: ['Instrument', 'Rank position', 'Share', 'Raw weight',
          `Weight after the ${state.cap} cap`, 'Instruction strength'],
        rows: result.rows.map((row) => [
          row.instrument,
          String(row.rank + 1),
          String(row.share),
          fixed(row.rawWeight),
          fixed(row.cappedWeight),
          fixed(row.instructionStrength),
        ]),
        caption: 'The allocation, one row per ranked instrument.',
      }));

      modelHost.appendChild(ui.metrics(
        ui.metric('Total invested', fixed(result.invested), 'The capped weights added together.', 'accent'),
        ui.metric('Left idle', fixed(result.idle), 'What the caps leave as cash under the idle policy.', 'muted'),
        ui.metric('Ceiling', fixed(result.ceiling), 'One unit, or the declared gross cap when smaller.'),
      ));

      modelHost.appendChild(ui.barChart({
        bars: result.rows.map((row) => ({ label: row.instrument, value: row.instructionStrength })),
        max: 1,
        yLabel: 'Instruction strength',
      }));
    } else {
      modelHost.appendChild(ui.verdict('Refused', result.code, 'bad', result.reason));
    }

    modelHost.appendChild(ui.group('The nine allocation decisions, in their fixed order',
      ui.list(DECISIONS().map((decision) => `${decision.n}. ${decision.title} - ${decision.rule}`), { ordered: true }),
    ));
    modelHost.appendChild(ui.group('The record kinds one run writes',
      ui.table({
        columns: ['Record', 'Fields, in order', 'What it is'],
        rows: recordKinds().map((record) => [record.name, record.fields.join(', '), record.note]),
        dense: true,
      }),
    ));
    modelHost.appendChild(ui.group('The closed vocabularies',
      ui.table({
        columns: ['Vocabulary', 'Members, in declared order'],
        rows: closedSets().map((set) => [set.name, set.members.join(', ')]),
        dense: true,
      }),
    ));
    modelHost.appendChild(ui.group('A decision bar against a rebalance bar',
      ui.table({
        columns: ['Frequency', 'Period key', 'When it rebalances'],
        rows: rebalancePeriods().map((period) => [period.frequency, period.key ?? 'none', period.rule]),
        dense: true,
      }),
      ui.note('A holding-period cap is counted in decision periods, so it counts sessions rather than bars.'),
    ));
    modelHost.appendChild(ui.group('The declared signal-frequency scope',
      ui.table({
        columns: ['Signal frequency', 'Bar specification', 'Declared entries', 'Unknown frequency'],
        rows: [[SIGNAL_SCOPE.signalFrequency, SIGNAL_SCOPE.barSpecification,
          String(SIGNAL_SCOPE.entries), SIGNAL_SCOPE.unknown]],
        dense: true,
      }),
      ui.quote(FIRST_RELEASE_SCOPE, meta.tutorial),
    ));
    modelHost.appendChild(ui.note(TIMING_CAVEAT, 'warn'));

    ui.clear(refusalHost);
    refusalHost.appendChild(ui.table({
      columns: ['Code', 'Message the interpreter raises'],
      rows: REFUSAL_CODES.map((code) => [code, REFUSAL_TEXTS[code] ?? code]),
      dense: true,
      caption: 'The interpreter raises a bare ValueError with no code enum; the code is the model\u2019s handle.',
    }));
  }

  // The sections were built as blocks; they become panels here so this view carries the same cards
  // as every other view. The section node itself is kept, because two of them are the hosts the
  // model renders into.
  const sections = [...root.children];
  ui.clear(root);
  for (const section of sections) {
    const heading = section.querySelector('h3');
    if (heading) heading.remove();
    root.appendChild(ui.panel({ title: heading ? heading.textContent : '' }, section));
  }

  render();
}
