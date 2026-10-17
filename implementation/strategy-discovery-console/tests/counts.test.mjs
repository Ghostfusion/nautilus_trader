import { test, expect } from 'bun:test';

import { countsFromEvents, effectiveCount, correction, pbo, perPeriodSharpe, returnMoments } from '../src/engine/counts.js';
import { COUNTS_MAPPING, FOLD_FORMULA, CORRECTION_REFUSALS } from '../src/data/counts.js';
import { valueOf } from '../src/data/source.js';

/** One written inventory event, with the fields the fold reads. */
const event = (kind, extra = {}) => ({ kind, key: '', candidateId: '', payload: {}, ...extra });

/**
 * The thirteen events of the tutorial's worked example, in written order. Row 1..13 as the page
 * lists them.
 */
const WORKED_EXAMPLE = [
  event('proposal_recorded'),
  event('specification_refused'),
  event('proposal_recorded'),
  event('compilation_recorded', { payload: { new_configuration: true } }),
  event('compilation_recorded', { payload: { new_configuration: false } }),
  event('job_submitted'),
  event('execution_attempt_closed', { payload: { outcome: 'completed' } }),
  event('registration_verified'),
  event('trial_admitted', { key: 'job-a' }),
  event('execution_attempt_closed', { payload: { outcome: 'crashed' } }),
  event('registration_incomplete', { payload: { reason: 'lost paperwork' } }),
  event('duplicate_registration', { payload: { reason: 'same key' } }),
  event('trial_admitted', { key: 'job-a' }),
];

test('the four counts over the tutorial worked example are exactly 2, 1, 2, 1', () => {
  // If this breaks, the one fold no longer reproduces the page's thirteen-event example, and every
  // number the console shows downstream is wrong.
  const counts = countsFromEvents(WORKED_EXAMPLE);
  expect(counts.ok).toBe(true);
  expect(counts.proposalAttempts).toBe(2);
  expect(counts.uniqueConfigurations).toBe(1);
  expect(counts.executedExperiments).toBe(2);
  expect(counts.statisticalTrials).toBe(1);
});

test('the counts carry the versioned mapping the transcription names', () => {
  // If this breaks, a report cannot name which counting rule produced it.
  expect(countsFromEvents(WORKED_EXAMPLE).mapping).toBe(valueOf(COUNTS_MAPPING));
  expect(valueOf(COUNTS_MAPPING)).toBe('discovery.counts/v1');
});

test('the fold is order-independent over a replayed event list', () => {
  // If this breaks, replaying the record in another order could move a count, which the fold exists
  // to prevent.
  const forward = countsFromEvents(WORKED_EXAMPLE);
  const reversed = countsFromEvents([...WORKED_EXAMPLE].reverse());
  expect(reversed.proposalAttempts).toBe(forward.proposalAttempts);
  expect(reversed.uniqueConfigurations).toBe(forward.uniqueConfigurations);
  expect(reversed.executedExperiments).toBe(forward.executedExperiments);
  expect(reversed.statisticalTrials).toBe(forward.statisticalTrials);
});

test('a refusal moves only the proposal count and a merge moves nothing', () => {
  // If this breaks, the page's definitional choice - refusals are proposals, never trials - is lost.
  const events = [
    event('proposal_recorded'),
    event('specification_refused'),
    event('equivalence_merge', { payload: { merged_into: 'x', rule: 'ignored_paths' } }),
  ];
  const counts = countsFromEvents(events);
  expect(counts.proposalAttempts).toBe(1);
  expect(counts.uniqueConfigurations).toBe(0);
  expect(counts.executedExperiments).toBe(0);
  expect(counts.statisticalTrials).toBe(0);
});

test('a timed-out attempt stays an execution and is never a trial', () => {
  // If this breaks, an attempt that produced no return series could inflate the trial count.
  const counts = countsFromEvents([
    event('execution_attempt_closed', { payload: { outcome: 'timed_out' } }),
  ]);
  expect(counts.executedExperiments).toBe(1);
  expect(counts.statisticalTrials).toBe(0);
});

test('a replayed admission under the same idempotency key counts once', () => {
  // If this breaks, a resubmitted job would move the statistical count twice.
  const counts = countsFromEvents([
    event('execution_attempt_closed', { payload: { outcome: 'completed' } }),
    event('trial_admitted', { key: 'job-a' }),
    event('trial_admitted', { key: 'job-a' }),
  ]);
  expect(counts.statisticalTrials).toBe(1);
});

test('the fold refuses a trial count above the executions that produced it', () => {
  // If this breaks, the consistency check the page states - statistical_trials <= executions - is
  // not enforced.
  const counts = countsFromEvents([event('trial_admitted', { key: 'job-a' })]);
  expect(counts.ok).toBe(false);
  expect(counts.reason).toBe('statistical_trials 1 exceeds executed_experiments 0');
});

test('an independent campaign reads the raw count and a dependent one uses the declared count', () => {
  // If this breaks, the correction deflates against the wrong trial count.
  const counts = countsFromEvents(WORKED_EXAMPLE);
  const independent = effectiveCount(counts, { dependence: 'independent' });
  expect(independent.countedTrials).toBe(1);
  const dependent = effectiveCount(counts, { dependence: 'dependent', effectiveTrials: 8 });
  expect(dependent.countedTrials).toBe(8);
});

test('the dependence declaration is refused in both directions with the ledger messages', () => {
  // If this breaks, a dependent campaign with no effective count silently gets a default.
  const counts = countsFromEvents(WORKED_EXAMPLE);
  const missing = effectiveCount(counts, { dependence: 'dependent' });
  expect(missing.ok).toBe(false);
  expect(missing.reason).toBe('a campaign with dependent trials must declare its effective trial count');
  const surplus = effectiveCount(counts, { dependence: 'independent', effectiveTrials: 3 });
  expect(surplus.ok).toBe(false);
  expect(surplus.reason).toBe('a campaign with independent trials must not declare an effective count');
});

test('the correction is unavailable below the contract minimums', () => {
  // If this breaks, a correction is reported where the sample cannot support one.
  const short = correction({
    sharpe: 0.2,
    trialSharpes: Array.from({ length: 12 }, (_, i) => 0.1 + i * 0.01),
    observations: 19,
    skew: 0,
    kurtosis: 3,
  });
  expect(short.status).toBe('unavailable');
  expect(short.reason).toBe('insufficient_data');
  expect(short.value).toBeNull();

  const fewTrials = correction({
    sharpe: 0.2,
    trialSharpes: Array.from({ length: 9 }, (_, i) => 0.1 + i * 0.01),
    observations: 250,
    skew: 0,
    kurtosis: 3,
  });
  expect(fewTrials.status).toBe('unavailable');
});

test('the correction computes a deflated Sharpe ratio for a declared sample', () => {
  // If this breaks, G3 cannot compare a result with the best of a search its size.
  const trialSharpes = Array.from({ length: 30 }, (_, i) => -0.3 + i * 0.02);
  const result = correction({
    sharpe: 0.35,
    trialSharpes,
    observations: 260,
    skew: -0.2,
    kurtosis: 3.4,
  });
  expect(result.ok).toBe(true);
  expect(result.status).toBe('computed');
  expect(result.countedTrials).toBe(30);
  expect(result.value).toBeGreaterThan(0.3);
  expect(result.value).toBeLessThan(1);
});

test('the correction is invalid when the variance factor is not positive', () => {
  // If this breaks, an undefined denominator is reported as a number rather than a refusal.
  const result = correction({
    sharpe: 1.0,
    trialSharpes: Array.from({ length: 20 }, (_, i) => -0.2 + i * 0.02),
    observations: 200,
    skew: 5.0,
    kurtosis: 3,
  });
  expect(result.status).toBe('invalid');
  expect(result.reason).toBe('undefined_result');
  expect(result.value).toBeNull();
});

test('the correction refuses a dependent sample that declares no effective trial count', () => {
  // If this breaks, the correction is computed against a count nobody declared.
  const sample = {
    sharpe: 0.2,
    trialSharpes: Array.from({ length: 20 }, (_, i) => -0.2 + i * 0.02),
    observations: 200,
    skew: 0,
    kurtosis: 3,
    dependence: 'dependent',
  };
  const result = correction(sample);
  expect(result.ok).toBe(false);
  expect(result.reason).toBe('a study with dependent trials must declare its effective trial count');
  expect(valueOf(CORRECTION_REFUSALS[2])).toContain('dependent trials must declare');
});

test('the four sums are stated once with every symbol named', () => {
  // If this breaks, the console's displayed formula drifts from the page it quotes.
  expect(valueOf(FOLD_FORMULA)).toContain('statistical_trial_count    = the number of distinct idempotency keys');
});

/** A matrix row with a clean common index, valued by an offset so the rows differ. */
const matrixRow = (configuration, offset, length = 16) => ({
  configuration,
  eligible: true,
  returns: Object.fromEntries(Array.from({ length }, (_, i) => [i + 1, offset + Math.sin(i + offset)])),
});

test('the overfitting diagnostic computes over the default eight blocks', () => {
  // If this breaks, the documented default of eight blocks no longer gives the page's 70
  // combinations, and the diagnostic is silently a different statistic.
  const result = pbo({ rows: [matrixRow('a', 0), matrixRow('b', 0.3), matrixRow('c', -0.2)] });
  expect(result.state).toBe('valid_with_documented_exclusions');
  expect(result.combinations).toBe(70);
  expect(result.configurations).toBe(3);
  expect(result.probability).toBeGreaterThanOrEqual(0);
  expect(result.probability).toBeLessThanOrEqual(1);
  expect(result.diagnosticOnly).toBe(true);
  expect(result.logitQuantiles).toHaveProperty('median');
});

test('the diagnostic classifies every excluded row and produces no number when preconditions fail', () => {
  // If this breaks, a row that failed or was ineligible could enter the statistic, or a matrix that
  // cannot support a comparison could still report one.
  const rows = [
    matrixRow('survivor', 0),
    { configuration: 'ineligible', eligible: false, ineligibleRule: 'data_integrity', returns: {} },
    { configuration: 'failed', eligible: true, failureClass: 'economically_failed', returns: {} },
    { configuration: 'missing', eligible: true, returns: { 1: 0.1, 2: null } },
  ];
  const result = pbo({ rows });
  const states = result.exclusions.map((exclusion) => exclusion.state);
  expect(states).toContain('ineligible');
  expect(states).toContain('eligible_but_failed');
  expect(result.state).toBe('not_computable');
  expect(result.probability).toBeNull();
  expect(result.reason).toContain('rows survived');

  const oddBlocks = pbo({ rows: [matrixRow('a', 0), matrixRow('b', 0.2)], blocks: 5 });
  expect(oddBlocks.state).toBe('not_computable');
  expect(oddBlocks.reason).toContain('even count');
});

test('the diagnostic is deterministic across two runs', () => {
  // If this breaks, two readers of the same matrix would see different numbers.
  const rows = [matrixRow('a', 0), matrixRow('b', 0.3), matrixRow('c', -0.2)];
  const first = pbo({ rows });
  const second = pbo({ rows });
  expect(second.probability).toBe(first.probability);
  expect(second.logitMean).toBe(first.logitMean);
});

test('the Sharpe ratio and the moments exclude missing observations', () => {
  // If this breaks, a gap in a return series would be read as a zero observation.
  const returns = [0.01, null, 0.02, 0.03, null, -0.01, 0.04, 0.02];
  const sharpes = perPeriodSharpe(returns);
  expect(sharpes.observations).toBe(6);
  const moments = returnMoments(returns);
  expect(moments.observations).toBe(6);
});