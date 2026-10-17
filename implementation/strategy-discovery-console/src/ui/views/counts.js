/**
 * What this view models: the four counts a campaign keeps, folded from one append-only event record,
 * and the correction that deflates a result against the trial count. It models
 * `counts_from_events` and `deflated_sharpe_ratio` as the page
 * `tutorials/strategy-discovery/counting-the-search/README.md` describes them and the file
 * `python/nautilus_trader/discovery/ledger.py` implements them.
 */

import { countsFromEvents, effectiveCount, correction } from '../../engine/counts.js';
import { valueOf } from '../../data/source.js';
import {
  COUNT_NAMES,
  FOLD_RULES,
  FOLD_FORMULA,
  CONSISTENCY_CHECK,
  DEPENDENCE_REFUSALS,
  EFFECTIVE_COUNT_RELATION,
  DEFLATED_SHARPE_FORMULA,
  DEFLATED_SHARPE_SYMBOLS,
  CORRECTION_REFUSALS,
  SIGNIFICANCE_STATUSES,
  PBO_FORMULA,
  PBO_PRECONDITIONS,
  PBO_STATES,
  PBO_DIAGNOSTIC_ONLY,
  COUNTING_LIMITS,
} from '../../data/counts.js';

export const meta = {
  id: 'counts',
  title: 'Counting the search: the four counts a campaign keeps',
  order: 5,
  tutorial: 'tutorials/strategy-discovery/counting-the-search/README.md',
};

/** The kind each of the four counted conditions reads, and what it contributes. */
const COUNT_CONTRIBUTIONS = [
  ['proposal_recorded', '+1 proposal attempt'],
  ['specification_refused', 'nothing'],
  ['compilation_recorded', '+1 configuration only when the identity is new'],
  ['equivalence_merge', 'nothing'],
  ['job_submitted', 'nothing'],
  ['execution_attempt_closed', '+1 execution, whatever the outcome'],
  ['registration_verified', 'nothing'],
  ['trial_admitted', '+1 statistical trial, once per idempotency key'],
  ['registration_incomplete', 'nothing'],
  ['duplicate_registration', 'nothing'],
  ['gate_verdict', 'nothing'],
  ['custody_event', 'nothing'],
  ['search_policy_recorded', 'nothing'],
];

/** Build the four-count model, its event builder and its correction chart. */
export function mount(root, ctx) {
  const { ui, data } = ctx;
  const layer = data.layer(meta.id);

  const state = {
    events: [],
    nextKey: 1,
    lastKey: null,
    dependence: 'independent',
    effectiveTrials: null,
    dispersion: 0.02,
    targetSharpe: 0.15,
  };

  const idea = ui.panel(
    { title: layer.title, subtitle: layer.tutorial, tone: 'accent' },
    ui.block('The idea', ui.quote(layer.idea, layer.tutorial)),
    ui.block(
      'The grade',
      ui.note(`Grade: ${layer.grade}`),
      layer.gradeRestOn ? ui.note(layer.gradeRestOn, 'muted') : null,
    ),
  );

  root.appendChild(idea);

  const output = ui.stack();
  const refusalOutput = ui.stack();
  const chartOutput = ui.stack();

  const addEvent = (kind, payload, key) => {
    state.events.push({
      kind,
      key: key ?? '',
      candidateId: 'campaign-1',
      payload: payload ?? {},
    });
    render();
  };

  const controls = ui.block(
    'Add events to the campaign',
    ui.row(
      ui.button('+ proposal', () => addEvent('proposal_recorded')),
      ui.button('+ new configuration', () => addEvent('compilation_recorded', { new_configuration: true })),
      ui.button('+ equivalent configuration', () => addEvent('compilation_recorded', { new_configuration: false })),
      ui.button('+ execution', () => addEvent('execution_attempt_closed', { outcome: 'completed' })),
    ),
    ui.row(
      ui.button('+ trial (new key)', () => {
        const key = `job-${state.nextKey}`;
        state.nextKey += 1;
        state.lastKey = key;
        addEvent('trial_admitted', { receipt_digest: key }, key);
      }),
      ui.button('+ trial (replay the last key)', () => {
        if (state.lastKey === null) return;
        addEvent('trial_admitted', { receipt_digest: state.lastKey }, state.lastKey);
      }),
      ui.button('reset', () => {
        state.events = [];
        state.nextKey = 1;
        state.lastKey = null;
        render();
      }),
    ),
    ui.note(
      'The four counts move independently: a refusal moves only proposals, an equivalent configuration moves nothing, an execution moves executions alone, and a replayed admission moves trials once.',
      'muted',
    ),
  );

  const model = ui.panel(
    { title: 'The model', subtitle: 'one versioned fold over the campaign record' },
    controls,
    output,
    ui.block('The effective count', refusalOutput),
    ui.block('The correction against the trial count', chartOutput),
    ui.block(
      'The overfitting diagnostic, which rejects nothing',
      ui.quote(valueOf(PBO_FORMULA)),
      ui.list(PBO_STATES.map((state) => ui.h('li', null, ui.h('code', { text: valueOf(state) })))),
      ui.note(
        `Preconditions: at least ${valueOf(PBO_PRECONDITIONS[0])} survivors, an even block count of at least ${valueOf(PBO_PRECONDITIONS[1])}, one common index, and a default of ${valueOf(PBO_PRECONDITIONS[2])} blocks. Its flag is always ${valueOf(PBO_DIAGNOSTIC_ONLY)}, and nothing in the gate chain reads it.`,
        'muted',
      ),
    ),
  );
  root.appendChild(model);

  const refusals = ui.panel(
    { title: 'What it refuses' },
    ui.block(
      'The fold refuses an impossible record',
      ui.list([
        ui.h('li', null, 'a trial count above the executions that produced it: ', ui.h('code', { text: valueOf(CONSISTENCY_CHECK) })),
      ]),
    ),
    ui.block(
      'The dependence declaration is refused in both directions',
      ui.list([
        ui.h('li', null, ui.h('strong', { text: 'dependent' }), ' without an effective count: ', ui.h('code', { text: valueOf(DEPENDENCE_REFUSALS[0]) })),
        ui.h('li', null, ui.h('strong', { text: 'independent' }), ' with an effective count: ', ui.h('code', { text: valueOf(DEPENDENCE_REFUSALS[1]) })),
      ]),
    ),
    ui.block(
      'The correction refuses what it cannot compute from',
      ui.list(CORRECTION_REFUSALS.map((entry) => ui.h('li', null, ui.h('code', { text: valueOf(entry) })))),
      ui.note(
        `Below ${valueOf(SIGNIFICANCE_STATUSES[1])} / ${valueOf(SIGNIFICANCE_STATUSES[3])} from insufficient data, and ${valueOf(SIGNIFICANCE_STATUSES[2])} / ${valueOf(SIGNIFICANCE_STATUSES[4])} from an undefined result, the correction reports a status rather than a number.`,
        'muted',
      ),
    ),
  );
  root.appendChild(refusals);

  const caveats = ui.panel(
    { title: 'Where it goes wrong' },
    ui.list(layer.caveats),
    ui.note(valueOf(COUNTING_LIMITS[0]), 'warn'),
    ui.note(valueOf(COUNTING_LIMITS[1]), 'warn'),
  );
  root.appendChild(caveats);

  const sources = ui.panel(
    { title: 'Where this came from' },
    ui.sources(layer.sources),
    ui.block('The arithmetic, as stated', ui.quote(valueOf(FOLD_FORMULA))),
    ui.block(
      'The fold, one condition per count',
      ui.list(FOLD_RULES.map((rule) => ui.h('li', null, ui.h('code', { text: valueOf(rule) })))),
    ),
    ui.quote(valueOf(EFFECTIVE_COUNT_RELATION)),
    ui.quote(valueOf(DEFLATED_SHARPE_FORMULA)),
    ui.note(valueOf(DEFLATED_SHARPE_SYMBOLS), 'muted'),
  );
  root.appendChild(sources);

  /** Render the model's own numbers, refusals and chart into their containers. */
  function render() {
    const fold = countsFromEvents(state.events);

    ui.clear(output);
    if (!fold.ok) {
      output.appendChild(ui.note(fold.reason, 'bad'));
      output.appendChild(ui.badge(fold.code, 'bad'));
      ui.clear(refusalOutput);
      ui.clear(chartOutput);
      return;
    }
    output.appendChild(ui.badge(`mapping ${fold.mapping}`, 'violet'));
    output.appendChild(
      ui.metrics(
        ui.metric('proposal attempts', ui.format.int(fold.proposalAttempts), valueOf(COUNT_NAMES[0])),
        ui.metric('unique configurations', ui.format.int(fold.uniqueConfigurations), valueOf(COUNT_NAMES[1])),
        ui.metric('executed experiments', ui.format.int(fold.executedExperiments), valueOf(COUNT_NAMES[2])),
        ui.metric('statistical trials', ui.format.int(fold.statisticalTrials), valueOf(COUNT_NAMES[3]), 'accent'),
      ),
    );
    output.appendChild(
      ui.table({
        columns: ['event kind', 'contributes to the counts'],
        rows: COUNT_CONTRIBUTIONS,
        dense: true,
        caption: 'the closed event vocabulary and what each kind moves',
      }),
    );
    output.appendChild(
      ui.table({
        columns: ['kind', 'count effect'],
        rows: state.events.map((event, index) => [`${index + 1}. ${event.kind}`, event.payload?.new_configuration === false ? 'equivalent identity' : '']),
        dense: true,
        caption: 'the record so far, in written order',
      }),
    );

    renderEffective(fold);
    renderChart(fold);
  }

  /** Show the effective-count declaration and its refusal, with the control that produces it. */
  function renderEffective(fold) {
    ui.clear(refusalOutput);
    const declared = {
      dependence: state.dependence,
      effectiveTrials: state.dependence === 'dependent' ? state.effectiveTrials : null,
    };
    const effect = effectiveCount(fold, declared);

    const dependenceSelect = ui.selectInput({
      id: 'counts-dependence',
      value: state.dependence,
      options: [
        { value: 'independent', label: 'independent' },
        { value: 'dependent', label: 'dependent' },
      ],
      onChange: (value) => {
        state.dependence = value;
        render();
      },
    });
    const effectiveInput = ui.numberInput({
      id: 'counts-effective-trials',
      value: state.effectiveTrials ?? '',
      min: 1,
      step: 1,
      onInput: (value) => {
        state.effectiveTrials = Number.isFinite(value) ? value : null;
        render();
      },
    });

    refusalOutput.appendChild(
      ui.row(
        ui.field('dependence', dependenceSelect, valueOf(EFFECTIVE_COUNT_RELATION).split('\n')[0]),
        ui.field('declared effective count', effectiveInput, 'required when dependent, refused when independent'),
      ),
    );
    if (effect.ok) {
      refusalOutput.appendChild(
        ui.metric('counted trials N', ui.format.int(effect.countedTrials), `raw statistical count ${effect.raw}, ${effect.dependence}`, 'ok'),
      );
    } else {
      refusalOutput.appendChild(ui.badge(effect.code, 'bad'));
      refusalOutput.appendChild(ui.note(effect.reason, 'bad'));
    }
  }

  /** Chart the correction as the trial count rises, which is the whole point of the correction. */
  function renderChart(fold) {
    ui.clear(chartOutput);
    const points = [];
    for (let n = 8; n <= 40; n += 1) {
      const spread = state.dispersion * Math.sqrt((n - 1) / n);
      const positives = Math.floor(n / 2);
      const trialSharpes = [
        ...Array(positives).fill(spread),
        ...Array(n - positives).fill(-spread),
      ];
      const result = correction({
        sharpe: state.targetSharpe,
        trialSharpes,
        observations: 250,
        skew: 0,
        kurtosis: 3,
      });
      if (result.ok && result.status === 'computed') points.push([n, result.value]);
    }

    const marks = [];
    if (fold.ok && fold.statisticalTrials >= 8 && fold.statisticalTrials <= 40) {
      marks.push({ x: fold.statisticalTrials, label: 'this campaign', color: ui.COLORS.accent });
    }

    chartOutput.appendChild(
      ui.lineChart({
        series: [{ label: 'deflated Sharpe ratio', color: ui.COLORS.accent, points, kind: 'both' }],
        xLabel: 'counted trials N',
        yLabel: 'deflated Sharpe ratio',
        xDomain: [8, 40],
        yDomain: [0, 1],
        marks,
        title: 'the correction falls as the trial count rises',
      }),
    );
    chartOutput.appendChild(
      ui.note(
        `A declared sample of ${state.targetSharpe} per-period Sharpe over 250 periods, with trial estimates spread by ${state.dispersion}. A larger N lowers the corrected value.`,
        'muted',
      ),
    );
  }

  render();
}
