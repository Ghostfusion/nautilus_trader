/**
 * What this view models: the six-question gate chain that stops at its first no, and the
 * pre-campaign admission test that compares a proposed search's size with what its sample can
 * support. It models `GateService.evaluate` and `assess_admission` as the page
 * `tutorials/strategy-discovery/the-admission-gate/README.md` describes them and the file
 * `python/nautilus_trader/discovery/gates.py` implements them.
 */

import { LADDER, walkGates, admissionTest } from '../../engine/gates.js';
import { valueOf } from '../../data/source.js';
import {
  GATE_QUESTIONS,
  GATE_VERDICTS,
  GATE_TOKENS,
  ADMISSION_OUTCOMES,
  ADMISSION_FORMULAE,
  ADMISSION_REFUSALS,
  ADMISSION_WORKED_EXAMPLE,
  ADMISSION_LIMITS,
  POLICY_REFUSALS,
  ADMISSION_VERSION,
} from '../../data/gates.js';

export const meta = {
  id: 'gates',
  title: 'The pre-registered gate chain and the pre-campaign admission test',
  order: 7,
  tutorial: 'tutorials/strategy-discovery/the-admission-gate/README.md',
};

/** The verdict vocabulary with the tone each verdict carries on screen. */
const VERDICT_TONES = {
  passed: 'ok',
  invalid: 'bad',
  rejected: 'bad',
  quarantined: 'bad',
  inconclusive: 'warn',
  validation_pending: 'warn',
};

/** A candidate that meets every gate, so choosing one defect breaks exactly one condition. */
function workedCandidate(overrides) {
  const returns = Object.fromEntries(
    Array.from({ length: 60 }, (_, i) => [i + 1, 0.004 + 0.002 * Math.sin(i)]),
  );
  return {
    candidateId: 'candidate-1',
    specificationDigest: 'sha256:spec',
    priceSeriesStatement: 'SPY one-minute bars, 2018-2023',
    dataRole: 'locked_validation',
    returns,
    benchmarkReturns: Array.from({ length: 60 }, (_, i) => 0.0005 * Math.cos(i)),
    costConfiguration: 'fees + half-spread + impact',
    split: { folds: 3 },
    foldReturns: [
      [0.01, 0.02, 0.005],
      [0.004, 0.008],
      [0.002, 0.006],
    ],
    trialSharpes: Array.from({ length: 12 }, (_, i) => -0.18 + i * 0.03),
    dependence: 'independent',
    statisticalTrials: 30,
    executedExperiments: 40,
    unusableAttempts: 2,
    nameShares: { a: 0.4, b: 0.35, c: 0.25 },
    sectorShares: { s1: 0.5, s2: 0.5 },
    periodShares: { p1: 0.3, p2: 0.4, p3: 0.3 },
    neighbourhoodObjectives: [1.0, 0.9, 0.85],
    stressReturns: { rates: 0.01, volatility: 0.02 },
    drawdown: 0.2,
    confirmationEvaluation: 'holdout-1',
    evidencePackageComplete: true,
    humanDecision: 'approved 2026-01-01',
    custody: { verdict: 'confirmed' },
    ...overrides,
  };
}

/** The defects a reader may plant, and the one condition each breaks. */
const DEFECTS = [
  { value: 'none', label: 'a clean candidate', overrides: {} },
  { value: 'g0', label: 'G0: no price-series statement', overrides: { priceSeriesStatement: '' } },
  { value: 'g1', label: 'G1: no cost configuration', overrides: { costConfiguration: '' } },
  { value: 'g2', label: 'G2: built on development data', overrides: { dataRole: 'development' } },
  { value: 'g3', label: 'G3: 15 unusable of 20 attempts', overrides: { executedExperiments: 20, unusableAttempts: 15 } },
  { value: 'g4', label: 'G4: one name contributes 0.9', overrides: { nameShares: { a: 0.9, b: 0.1 } } },
  { value: 'g5', label: 'G5: the holdout was leaked', overrides: { quarantineReasons: ['read the confirmation set'] } },
];

/** Build the gate walk, its refusals and the admission assessment. */
export function mount(root, ctx) {
  const { ui, data } = ctx;
  const layer = data.layer(meta.id);

  const state = {
    defect: 'none',
    admission: {
      effectiveObservations: 300,
      reservedObservations: 0,
      holdoutFraction: 0.2,
      plannedConfigurations: 45,
      trialsAlreadyConsumed: 0,
      targetSharpe: 0.15,
      significance: 0.95,
      crossTrialDispersion: 0.02,
    },
  };

  root.appendChild(
    ui.panel(
      { title: layer.title, subtitle: layer.tutorial, tone: 'accent' },
      ui.block('The idea', ui.quote(layer.idea, layer.tutorial)),
      ui.block(
        'The grade',
        ui.note(`Grade: ${layer.grade}`),
        layer.gradeRestOn ? ui.note(layer.gradeRestOn, 'muted') : null,
      ),
    ),
  );

  const walkOutput = ui.stack();
  const admissionOutput = ui.stack();

  const defectSelect = ui.selectInput({
    id: 'gates-defect',
    value: state.defect,
    options: DEFECTS.map((defect) => ({ value: defect.value, label: defect.label })),
    onChange: (value) => {
      state.defect = value;
      renderWalk();
    },
  });

  const model = ui.panel(
    { title: 'The model', subtitle: 'six questions, in one order, stopping at the first no' },
    ui.block(
      'Walk the ladder over a worked candidate',
      ui.field('planted defect', defectSelect, 'the one condition the candidate fails'),
      walkOutput,
    ),
    ui.block('The pre-campaign admission test', admissionOutput),
  );
  root.appendChild(model);

  root.appendChild(
    ui.panel(
      { title: 'What it refuses' },
      ui.block(
        'A policy that does not say what its thresholds mean',
        ui.list(POLICY_REFUSALS.map((entry) => ui.h('li', null, ui.h('code', { text: valueOf(entry) })))),
      ),
      ui.block(
        'A missing input is a refusal, not a default',
        ui.list([
          ui.h('li', null, 'a gate returns ', ui.h('code', { text: 'INCONCLUSIVE' }), ' named ', ui.h('code', { text: valueOf(GATE_TOKENS[0]) }), ' with the input it could not read'),
        ]),
      ),
      ui.block(
        'The verdict vocabulary',
        ui.list(GATE_VERDICTS.map((entry) => ui.h('li', null, ui.badge(valueOf(entry), VERDICT_TONES[valueOf(entry)])))),
        ui.note(`Admission answers one of ${ADMISSION_OUTCOMES.map(valueOf).join(', ')}.`, 'muted'),
      ),
      ui.block(
        'The admission assessment refuses what it cannot estimate',
        ui.list(ADMISSION_REFUSALS.map((entry) => ui.h('li', null, ui.h('code', { text: valueOf(entry) })))),
        ui.note('Demonstrated above: choose the G1 defect to see the missing input reported as inputs_missing rather than filled in.', 'muted'),
      ),
    ),
  );

  root.appendChild(
    ui.panel(
      { title: 'Where it goes wrong' },
      ui.list(layer.caveats),
      ui.note(valueOf(ADMISSION_LIMITS[0]), 'warn'),
      ui.note(valueOf(ADMISSION_LIMITS[1]), 'warn'),
    ),
  );

  root.appendChild(
    ui.panel(
      { title: 'Where this came from' },
      ui.sources(layer.sources),
      ui.block('The admission arithmetic, as stated', ui.quote(ADMISSION_FORMULAE.map(valueOf).join('\n'))),
      ui.block(
        'The worked example, as stated',
        ui.note(
          `usable ${valueOf(ADMISSION_WORKED_EXAMPLE[0])}, counted trials ${valueOf(ADMISSION_WORKED_EXAMPLE[1])}, a ceiling of ${valueOf(ADMISSION_WORKED_EXAMPLE[2])}, a narrowing to ${valueOf(ADMISSION_WORKED_EXAMPLE[3])}, and ${valueOf(ADMISSION_WORKED_EXAMPLE[4])}`,
        ),
      ),
      ui.note(`${valueOf(ADMISSION_VERSION)}`, 'muted'),
    ),
  );

  /** Walk the ladder and render the verdicts, the stop and the gates never asked. */
  function renderWalk() {
    ui.clear(walkOutput);
    const defect = DEFECTS.find((entry) => entry.value === state.defect) ?? DEFECTS[0];
    const report = walkGates(workedCandidate(defect.overrides));

    walkOutput.appendChild(
      ui.metrics(
        ui.metric('outcome', report.outcome, report.stoppedAt ? `stopped at ${report.stoppedAt}` : 'reached every gate', VERDICT_TONES[report.outcome]),
        ui.metric('gates asked', ui.format.int(report.records.length), 'of six'),
      ),
    );

    walkOutput.appendChild(
      ui.table({
        columns: ['gate', 'question', 'verdict', 'asked'],
        rows: report.states.map((state_, index) => [
          state_.gate,
          ui.h('span', { text: valueOf(GATE_QUESTIONS[index]) }),
          state_.asked ? ui.badge(state_.verdict, VERDICT_TONES[state_.verdict]) : ui.badge('never asked', 'muted'),
          state_.asked ? 'yes' : 'no',
        ]),
        caption: 'the ladder, gate by gate',
      }),
    );

    if (report.stoppedAt) {
      walkOutput.appendChild(ui.note(report.records.at(-1).reason, VERDICT_TONES[report.outcome]));
    }
    walkOutput.appendChild(
      ui.note(
        report.neverAsked.length
          ? `Never asked, not passed: ${report.neverAsked.join(', ')}`
          : 'Every gate was asked and passed.',
        report.neverAsked.length ? 'warn' : 'ok',
      ),
    );
    ui.clear(admissionOutput);
    renderAdmission();
  }

  /** Compute and render the admission assessment from the editable inputs. */
  function renderAdmission() {
    ui.clear(admissionOutput);
    const assessment = admissionTest(state.admission);

    const bind = (key, min, step, help) =>
      ui.field(
        key,
        ui.numberInput({
          id: `adm-${key}`,
          value: state.admission[key],
          min,
          step,
          onInput: (value) => {
            state.admission[key] = Number.isFinite(value) ? value : 0;
            renderAdmission();
          },
        }),
        help,
      );

    admissionOutput.appendChild(
      ui.row(
        bind('effectiveObservations', 0, 1, 'E, effective independent observations'),
        bind('reservedObservations', 0, 1, 'R, reserved for folds and embargo'),
        bind('holdoutFraction', 0, 0.05, 'h, share for the final holdout'),
        bind('plannedConfigurations', 0, 1, 'P, planned configurations'),
      ),
    );
    admissionOutput.appendChild(
      ui.row(
        bind('targetSharpe', 0, 0.01, 'per-period target Sharpe'),
        bind('crossTrialDispersion', 0, 0.01, 'assumed cross-trial dispersion'),
        bind('significance', 0, 0.01, 's, significance the correction must reach'),
      ),
    );

    admissionOutput.appendChild(
      ui.badge(assessment.outcome, assessment.outcome === 'admit' ? 'ok' : assessment.outcome === 'reduce' ? 'warn' : 'bad'),
    );
    admissionOutput.appendChild(
      ui.metrics(
        ui.metric('usable observations U', ui.format.int(assessment.usableObservations), 'the budget the ceiling may count'),
        ui.metric('counted trials K', ui.format.int(assessment.countedTrials), 'from the plan and its history'),
        ui.metric('ceiling T*(K)', assessment.requiredObservations === null ? 'none' : ui.format.int(assessment.requiredObservations), 'required observations'),
        ui.metric('reduced to', assessment.reducedConfigurations === null ? 'n/a' : ui.format.int(assessment.reducedConfigurations), 'configurations'),
      ),
    );
    admissionOutput.appendChild(ui.note(assessment.reason, assessment.outcome === 'reject' ? 'bad' : 'ok'));
    admissionOutput.appendChild(ui.note(assessment.limitations.join(' '), 'muted'));
  }

  renderWalk();
}
