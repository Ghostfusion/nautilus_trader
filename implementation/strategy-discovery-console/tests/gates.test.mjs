import { test, expect } from 'bun:test';

import { LADDER, walkGates, admissionTest, VERDICTS } from '../src/engine/gates.js';
import { GATE_QUESTIONS, ADMISSION_WORKED_EXAMPLE } from '../src/data/gates.js';
import { valueOf } from '../src/data/source.js';

/** A candidate that meets every gate, so a test can break exactly one condition against it. */
function passingCandidate(overrides = {}) {
  const returns = Object.fromEntries(
    Array.from({ length: 60 }, (_, i) => [i + 1, 0.004 + 0.002 * Math.sin(i)]),
  );
  const benchmarkReturns = Array.from({ length: 60 }, (_, i) => 0.0005 * Math.cos(i));
  const trialSharpes = Array.from({ length: 12 }, (_, i) => -0.18 + i * 0.03);
  return {
    candidateId: 'candidate-1',
    specificationDigest: 'sha256:spec',
    priceSeriesStatement: 'SPY one-minute bars, 2018-2023',
    dataRole: 'locked_validation',
    returns,
    benchmarkReturns,
    costConfiguration: 'fees + half-spread + impact',
    split: { folds: 3 },
    foldReturns: [
      [0.01, 0.02, 0.005],
      [0.004, 0.008],
      [0.002, 0.006],
    ],
    trialSharpes,
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

test('the ladder names the six gates in order, each with its question', () => {
  // If this breaks, the console presents a chain in the wrong order or with a missing question.
  const ladder = LADDER();
  expect(ladder.map((gate) => gate.gate)).toEqual(['G0', 'G1', 'G2', 'G3', 'G4', 'G5']);
  expect(ladder[0].question).toBe(valueOf(GATE_QUESTIONS[0]));
  expect(ladder[4].question).toContain('lucky name');
  expect(ladder[3].thresholds.map((entry) => entry.name)).toContain('minimum_deflated_sharpe');
});

test('the verdict vocabulary is the fixed six the page names', () => {
  // If this breaks, a verdict outside the closed set could be presented as a gate's answer.
  expect(VERDICTS).toEqual(['passed', 'invalid', 'rejected', 'inconclusive', 'validation_pending', 'quarantined']);
});

test('a candidate that meets every gate is reached on all six and stops nowhere', () => {
  // If this breaks, a clean candidate fails a gate it should pass, or the walk reports a stop it
  // did not make.
  const report = walkGates(passingCandidate());
  expect(report.stoppedAt).toBeNull();
  expect(report.outcome).toBe('passed');
  expect(report.records.map((record) => record.gate)).toEqual(['G0', 'G1', 'G2', 'G3', 'G4', 'G5']);
  expect(report.neverAsked).toEqual([]);
});

test('the chain stops at a failing middle gate and the later gates are never asked', () => {
  // If this breaks, a later gate that would have passed could excuse an earlier failure, which is
  // exactly what the stopping rule forbids.
  const report = walkGates(passingCandidate({ dataRole: 'development' }));
  expect(report.stoppedAt).toBe('G2');
  expect(report.outcome).toBe('inconclusive');
  expect(report.records.map((record) => record.gate)).toEqual(['G0', 'G1', 'G2']);
  expect(report.neverAsked).toEqual(['G3', 'G4', 'G5']);
  const later = report.states.filter((state) => state.gate !== 'G0' && state.gate !== 'G1' && state.gate !== 'G2');
  expect(later.every((state) => state.asked === false && state.verdict === null)).toBe(true);
});

test('G0 refuses a candidate that cannot say what it consumed', () => {
  // If this breaks, a run whose price-series statement is missing could still advance.
  const report = walkGates(passingCandidate({ priceSeriesStatement: '' }));
  expect(report.stoppedAt).toBe('G0');
  expect(report.outcome).toBe('invalid');
  expect(report.records[0].reason).toContain('no price-series statement');
});

test('G1 refuses a candidate below its floors and cannot compare a missing cost configuration', () => {
  // If this breaks, a candidate that does not survive its costs could pass, or a missing input
  // could be met with a default instead of a refusal.
  const missingCost = walkGates(passingCandidate({ costConfiguration: '' }));
  expect(missingCost.stoppedAt).toBe('G1');
  expect(missingCost.outcome).toBe('inconclusive');
  expect(missingCost.records.at(-1).evidence.inputs_missing).toContain('cost_configuration');

  const losing = walkGates(passingCandidate({ benchmarkReturns: Array.from({ length: 60 }, (_, i) => 0.01 + 0.001 * Math.cos(i)) }));
  expect(losing.stoppedAt).toBe('G1');
  expect(losing.outcome).toBe('rejected');
});

test('G2 refuses losing walk-forward folds over locked validation data', () => {
  // If this breaks, a candidate that does not persist out of sample could advance.
  const report = walkGates(passingCandidate({ foldReturns: [[-0.01], [-0.02], [-0.005]] }));
  expect(report.stoppedAt).toBe('G2');
  expect(report.outcome).toBe('rejected');
  expect(report.records[2].reason).toContain('folds are positive');
});

test('G3 goes inconclusive on an unusable share above the declared ceiling', () => {
  // If this breaks, a campaign that mostly produced nothing could still report a correction.
  const report = walkGates(passingCandidate({ executedExperiments: 20, unusableAttempts: 15 }));
  expect(report.stoppedAt).toBe('G3');
  expect(report.outcome).toBe('inconclusive');
  expect(report.records.at(-1).reason).toContain('produced no usable return series');
});

test('G3 goes inconclusive when the inventory declares no multiplicity', () => {
  // If this breaks, the correction could be computed against a trial count nobody recorded.
  const report = walkGates(passingCandidate({ statisticalTrials: 0, trialSharpes: [] }));
  expect(report.stoppedAt).toBe('G3');
  expect(report.outcome).toBe('inconclusive');
  expect(report.records.at(-1).evidence.inputs_missing).toContain('trial_sharpes');
});

test('G4 refuses concentration, a non-positive stress and a deep drawdown', () => {
  // If this breaks, a result that depends on one name or a lucky value could pass.
  const concentrated = walkGates(passingCandidate({ nameShares: { a: 0.9, b: 0.1 } }));
  expect(concentrated.stoppedAt).toBe('G4');
  expect(concentrated.outcome).toBe('rejected');

  const stressed = walkGates(passingCandidate({ stressReturns: { rates: 0.01, volatility: -0.02 } }));
  expect(stressed.stoppedAt).toBe('G4');
  expect(stressed.outcome).toBe('rejected');

  const drawn = walkGates(passingCandidate({ drawdown: 0.9 }));
  expect(drawn.stoppedAt).toBe('G4');
  expect(drawn.outcome).toBe('rejected');
});

test('G5 quarantines a leak and holds a candidate without a human decision pending', () => {
  // If this breaks, a candidate that touched the confirmation data could pass, or an incomplete
  // package could advance.
  const leaked = walkGates(passingCandidate({ quarantineReasons: ['read the confirmation set'] }));
  expect(leaked.stoppedAt).toBe('G5');
  expect(leaked.outcome).toBe('quarantined');

  const moved = walkGates(passingCandidate({ frozenSpecificationDigest: 'sha256:other' }));
  expect(moved.outcome).toBe('quarantined');

  const pending = walkGates(passingCandidate({ humanDecision: '' }));
  expect(pending.outcome).toBe('validation_pending');
});

test('the admission assessment reproduces the tutorial worked example exactly', () => {
  // If this breaks, the console's worked example no longer matches the page: 240 usable, 45 counted
  // trials, a 248-observation ceiling and a narrowing to 36 configurations.
  const inputs = {
    effectiveObservations: 300,
    reservedObservations: 0,
    holdoutFraction: 0.2,
    plannedConfigurations: 45,
    trialsAlreadyConsumed: 0,
    targetSharpe: 0.15,
    significance: 0.95,
    crossTrialDispersion: 0.02,
  };
  const assessment = admissionTest(inputs);
  expect(assessment.usableObservations).toBe(240);
  expect(assessment.countedTrials).toBe(45);
  expect(assessment.requiredObservations).toBe(248);
  expect(assessment.outcome).toBe('reduce');
  expect(assessment.reducedConfigurations).toBe(36);
  expect(assessment.reason).toContain('narrows to 36');
  expect(valueOf(ADMISSION_WORKED_EXAMPLE[0])).toBe(240);
});

test('the same plan of thirty configurations is admitted at a 234-observation ceiling', () => {
  // If this breaks, a smaller plan that fits its sample could be narrowed or refused.
  const assessment = admissionTest({
    effectiveObservations: 300,
    reservedObservations: 0,
    holdoutFraction: 0.2,
    plannedConfigurations: 30,
    trialsAlreadyConsumed: 0,
    targetSharpe: 0.15,
    significance: 0.95,
    crossTrialDispersion: 0.02,
  });
  expect(assessment.requiredObservations).toBe(234);
  expect(assessment.outcome).toBe('admit');
  expect(assessment.countedTrials).toBe(30);
});

test('the admission assessment refuses an unestimable input rather than defaulting it', () => {
  // If this breaks, a dependent campaign with no effective count would be given one.
  const dependent = admissionTest({
    effectiveObservations: 300,
    holdoutFraction: 0.2,
    plannedConfigurations: 45,
    dependence: 'dependent',
    targetSharpe: 0.15,
    crossTrialDispersion: 0.02,
  });
  expect(dependent.outcome).toBe('reject');
  expect(dependent.reason).toBe('the configurations are dependent and no effective trial count is estimated');

  const noDispersion = admissionTest({ effectiveObservations: 300, plannedConfigurations: 10 });
  expect(noDispersion.outcome).toBe('reject');
  expect(noDispersion.reason).toContain('cross-trial dispersion');
  expect(noDispersion.requiredObservations).toBeNull();
});

test('the admission assessment is deterministic across two runs', () => {
  // If this breaks, two readers of the same plan would see different ceilings.
  const inputs = {
    effectiveObservations: 300,
    holdoutFraction: 0.2,
    plannedConfigurations: 45,
    targetSharpe: 0.15,
    crossTrialDispersion: 0.02,
  };
  expect(admissionTest(inputs).requiredObservations).toBe(admissionTest(inputs).requiredObservations);
});

test('the admission assessment reports the relation as a limitation, not a power calculation', () => {
  // If this breaks, the declared power would be presented as a verified quantity.
  const assessment = admissionTest({
    effectiveObservations: 300,
    holdoutFraction: 0.2,
    plannedConfigurations: 45,
    targetSharpe: 0.15,
    crossTrialDispersion: 0.02,
    power: 0.8,
  });
  expect(assessment.limitations.join(' ')).toContain('not a power calculation');
});