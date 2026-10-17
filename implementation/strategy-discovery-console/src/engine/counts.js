/**
 * The counting engine: one versioned fold over a campaign's event list, the declaration that
 * separates the raw count from the effective count, and the two pieces of arithmetic that consume
 * them - the deflated Sharpe ratio and the probability of backtest overfitting.
 *
 * WHY THIS MODULE EXISTS. The console refuses to let a method define its own counting rule. Every
 * count a reader sees is taken by one fold over the events, so the numbers are recomputable from
 * the record alone and a replayed event cannot move them twice. `countsFromEvents` models
 * `counts_from_events` in `python/nautilus_trader/discovery/ledger.py`; `effectiveCount` models the
 * both-ways dependence refusal of `trial_mapping`; `correction` models `deflated_sharpe_ratio` in
 * `python/nautilus_trader/optimization/significance.py`; `pbo` models `evaluate_pbo_matrix` in
 * `python/nautilus_trader/discovery/gates.py`.
 *
 * Pure: no DOM, no I/O, no randomness. Identical inputs give identical outputs.
 */

import { valueOf } from '../data/source.js';
import { COUNTS_MAPPING, PBO_PRECONDITIONS, SIGNIFICANCE_CONTRACT } from '../data/counts.js';

const SQRT_TWO = Math.sqrt(2.0);
const SQRT_TWO_PI = Math.sqrt(2.0 * Math.PI);

// The Euler-Mascheroni constant and the contract defaults are read from the transcription, not
// written here twice.
const EULER = valueOf(SIGNIFICANCE_CONTRACT[4]);
const DEFAULT_MINIMUM_OBSERVATIONS = valueOf(SIGNIFICANCE_CONTRACT[2]);
const DEFAULT_MINIMUM_TRIALS = valueOf(SIGNIFICANCE_CONTRACT[3]);
const MINIMUM_MOMENT_OBSERVATIONS = valueOf(SIGNIFICANCE_CONTRACT[5]);

const PBO_MINIMUM_SURVIVORS = valueOf(PBO_PRECONDITIONS[0]);
const PBO_MINIMUM_BLOCKS = valueOf(PBO_PRECONDITIONS[1]);
const PBO_DEFAULT_BLOCKS = valueOf(PBO_PRECONDITIONS[2]);

/** A compensated sum, so a long series of returns does not lose the precision a fold needs. */
function fsum(values) {
  let sum = 0.0;
  let compensation = 0.0;
  for (const value of values) {
    const add = value - compensation;
    const next = sum + add;
    compensation = next - sum - add;
    sum = next;
  }
  return sum;
}

/** The sample or population variance of the given values, two-pass for precision. */
function variance(values, divisor) {
  const mean = fsum(values) / values.length;
  const squared = fsum(values.map((value) => (value - mean) ** 2));
  const denominator = divisor === 'population' ? values.length : values.length - 1;
  return squared / denominator;
}

/** The standard normal probability density function. */
function normalPdf(value) {
  return Math.exp(-0.5 * value * value) / SQRT_TWO_PI;
}

/** The Taylor series of erf, used below the continued fraction's crossover. */
function erfSeries(x) {
  const x2 = x * x;
  let term = x;
  let sum = x;
  let compensation = 0.0;
  for (let i = 1; i < 200; i += 1) {
    term = (-term * x2) / i;
    const add = term / (2 * i + 1);
    const y = add - compensation;
    const next = sum + y;
    compensation = next - sum - y;
    sum = next;
    if (Math.abs(add) < 1e-18) break;
  }
  return (2 / Math.sqrt(Math.PI)) * sum;
}

/** erfc by its continued fraction, accurate in the tail where the correction's quantiles live. */
function erfcContinuedFraction(x) {
  const tiny = 1e-300;
  let f = tiny;
  let c = tiny;
  let d = 0.0;
  const x2 = x * x;
  for (let i = 1; i <= 300; i += 1) {
    const a = i === 1 ? 1 : (i - 1) / 2;
    d = x + a * d;
    if (d === 0) d = tiny;
    d = 1 / d;
    c = x + a / c;
    if (c === 0) c = tiny;
    const delta = c * d;
    f *= delta;
    if (Math.abs(delta - 1) < 1e-17) break;
  }
  return (Math.exp(-x2) / Math.sqrt(Math.PI)) * f;
}

/** The error function, matching the accuracy of the C library's `erf`. */
function erf(x) {
  if (x < 0) return -erf(-x);
  if (x <= 1.5) return erfSeries(x);
  return 1 - erfcContinuedFraction(x);
}

/** The standard normal cumulative distribution function, as `0.5 erfc(-x / sqrt(2))`. */
function normalCdf(value) {
  return 0.5 * (1 - erf(-value / SQRT_TWO));
}

/** The standard normal quantile function, by Newton's method on a maintained bracket. */
function normalPpf(probability) {
  if (!(probability > 0 && probability < 1)) {
    throw new Error(`probability must be in (0, 1), was ${probability}`);
  }
  let lower = -40.0;
  let upper = 40.0;
  let value = 0.0;
  for (let i = 0; i < 300; i += 1) {
    const error = normalCdf(value) - probability;
    if (error < 0) lower = value;
    else upper = value;
    const density = normalPdf(value);
    let candidate = density > 0 ? value - error / density : 0.5 * (lower + upper);
    if (!(lower < candidate && candidate < upper)) candidate = 0.5 * (lower + upper);
    if (candidate === value || upper - lower < 1e-16) return candidate;
    value = candidate;
  }
  return value;
}

/** The per-period Sharpe ratio of a return series and the periods that contributed. */
export function perPeriodSharpe(returns) {
  const values = Array.isArray(returns) ? returns : Object.values(returns);
  const contributing = [];
  for (const value of values) {
    if (value === null || value === undefined || Number.isNaN(value)) continue;
    if (!Number.isFinite(value)) throw new Error('a return must be finite: an infinity is a defect, not a gap');
    contributing.push(value);
  }
  if (contributing.length < MINIMUM_MOMENT_OBSERVATIONS) return { value: null, observations: contributing.length };
  const dispersion = Math.sqrt(variance(contributing, 'sample'));
  if (dispersion <= 0) return { value: null, observations: contributing.length };
  return { value: fsum(contributing) / contributing.length / dispersion, observations: contributing.length };
}

/** The skewness and the non-excess kurtosis of a return series, or null when undefined. */
export function returnMoments(returns) {
  const values = Array.isArray(returns) ? returns : Object.values(returns);
  const contributing = values.filter((value) => value !== null && value !== undefined && !Number.isNaN(value));
  if (contributing.length < MINIMUM_MOMENT_OBSERVATIONS) return null;
  const mean = fsum(contributing) / contributing.length;
  const deviations = contributing.map((value) => value - mean);
  const second = fsum(deviations.map((d) => d ** 2)) / deviations.length;
  if (second <= 0) return null;
  const third = fsum(deviations.map((d) => d ** 3)) / deviations.length;
  const fourth = fsum(deviations.map((d) => d ** 4)) / deviations.length;
  return { skew: third / second ** 1.5, kurtosis: fourth / second ** 2, observations: contributing.length };
}

/**
 * Fold an event list into the four counts, under the versioned mapping.
 *
 * The fold is a single pass over the events in written order and never an increment: the same
 * record replayed gives the same counts. A refusal (`specification_refused`) moves only the
 * proposal count; a compilation moves the configuration count only when its identity is new; an
 * attempt that closed moves the execution count whatever its outcome; and a trial admission moves
 * the statistical count once per distinct idempotency key. The four counts are refused when the
 * record cannot hold - a negative count, an empty mapping, or a trial count above the executions
 * that could have produced it.
 *
 * @returns {object} `{ ok, proposalAttempts, uniqueConfigurations, executedExperiments,
 *   statisticalTrials, mapping }`, or `{ ok: false, code, reason }` when the record cannot hold.
 */
export function countsFromEvents(events) {
  if (!Array.isArray(events)) {
    throw new Error('countsFromEvents requires an array of events');
  }
  let proposals = 0;
  let configurations = 0;
  let executions = 0;
  const admitted = new Set();

  for (const event of events) {
    const kind = event.kind;
    if (kind === 'proposal_recorded') proposals += 1;
    else if (kind === 'compilation_recorded') {
      configurations += event.payload && event.payload.new_configuration ? 1 : 0;
    } else if (kind === 'execution_attempt_closed') executions += 1;
    else if (kind === 'trial_admitted') admitted.add(event.key);
  }

  const statisticalTrials = admitted.size;
  if (statisticalTrials > executions) {
    return {
      ok: false,
      code: 'trial_exceeds_execution',
      reason: `statistical_trials ${statisticalTrials} exceeds executed_experiments ${executions}`,
    };
  }

  const mapping = valueOf(COUNTS_MAPPING);
  if (!mapping) {
    return { ok: false, code: 'mapping_empty', reason: 'mapping must not be empty' };
  }

  return {
    ok: true,
    proposalAttempts: proposals,
    uniqueConfigurations: configurations,
    executedExperiments: executions,
    statisticalTrials,
    mapping,
  };
}

/**
 * Return the trial count a correction deflates against, or refuse the declaration.
 *
 * Raw and effective counts are separate numbers joined by a declaration, not a formula. When the
 * attempts are independent the effective count is the raw statistical count; when they are
 * dependent the campaign must declare an effective count, and a campaign that declares one while
 * claiming independence is refused too. Both directions are refused rather than defaulted.
 *
 * @returns {object} `{ ok, dependence, raw, countedTrials }`, or `{ ok: false, code, reason }`.
 */
export function effectiveCount(counts, declared = {}) {
  const dependence = declared.dependence === 'dependent' ? 'dependent' : 'independent';
  const effectiveTrials =
    declared.effectiveTrials === undefined || declared.effectiveTrials === null
      ? null
      : declared.effectiveTrials;

  if (dependence === 'dependent' && effectiveTrials === null) {
    return {
      ok: false,
      code: 'dependent_without_effective_trials',
      reason: 'a campaign with dependent trials must declare its effective trial count',
    };
  }
  if (dependence === 'independent' && effectiveTrials !== null) {
    return {
      ok: false,
      code: 'independent_with_effective_trials',
      reason: 'a campaign with independent trials must not declare an effective count',
    };
  }

  return {
    ok: true,
    dependence,
    raw: counts.statisticalTrials,
    countedTrials: dependence === 'dependent' ? effectiveTrials : counts.statisticalTrials,
  };
}

/**
 * Compute the deflated Sharpe ratio of a declared sample, the correction the gates consume.
 *
 * The arithmetic is `SR0 = sqrt(V[SR]) * ((1 - gamma) * Phi^-1(1 - 1/N) + gamma * Phi^-1(1 - 1/(N
 * e)))` and `DSR = Phi((SR - SR0) * sqrt(T - 1) / sqrt(1 - skew * SR + ((kurtosis - 1) / 4) *
 * SR^2))`, where `V[SR]` is the cross-trial variance of the trial Sharpe estimates, `N` the counted
 * trials, `T` the contributing periods and `gamma` the Euler-Mascheroni constant. The result is a
 * value and a status, never a gate: below the contract's minimums it is `unavailable` with
 * `insufficient_data`, and a non-positive variance factor or a non-finite value is `invalid` with
 * `undefined_result`. The declaration itself is refused when the dependence and the effective count
 * disagree, when an annualised frequency is declared, or when a trial estimate is not finite.
 *
 * @param {object} sample `{ sharpe, trialSharpes, observations, skew, kurtosis, dependence,
 *   effectiveTrials, frequency }`.
 * @param {object} [contract] `{ minimumObservations, minimumTrials, trialVarianceDivisor }`.
 * @returns {object} `{ ok, status, reason, value, ... }`, or `{ ok: false, code, reason }`.
 */
export function correction(sample, contract = {}) {
  const frequency = sample.frequency ?? 'per_period';
  if (frequency !== 'per_period') {
    return {
      ok: false,
      code: 'annualised_input',
      reason: `the correction is per-period: an annualised input (${frequency}) must be rescaled by its caller, not divided here`,
    };
  }

  const sharpe = sample.sharpe;
  const skew = sample.skew;
  const kurtosis = sample.kurtosis;
  const observations = sample.observations;
  const trialSharpes = sample.trialSharpes ?? [];
  const dependence = sample.dependence === 'dependent' ? 'dependent' : 'independent';
  const effectiveTrials =
    sample.effectiveTrials === undefined || sample.effectiveTrials === null
      ? null
      : sample.effectiveTrials;

  for (const [name, value] of [
    ['sharpe', sharpe],
    ['skew', skew],
    ['kurtosis', kurtosis],
  ]) {
    if (!Number.isFinite(value)) {
      return { ok: false, code: 'non_finite_input', reason: `${name} must be finite, was ${value}` };
    }
  }
  if (kurtosis < 1.0) {
    return {
      ok: false,
      code: 'kurtosis_convention',
      reason: `kurtosis must be the non-excess convention and therefore at least 1, was ${kurtosis}`,
    };
  }
  if (observations < MINIMUM_MOMENT_OBSERVATIONS) {
    return { ok: false, code: 'too_few_observations', reason: `observations must be at least ${MINIMUM_MOMENT_OBSERVATIONS}, was ${observations}` };
  }
  for (const value of trialSharpes) {
    if (!Number.isFinite(value)) {
      return { ok: false, code: 'non_finite_input', reason: `every trial Sharpe must be finite, found ${value}` };
    }
  }

  if (dependence !== 'dependent' && effectiveTrials !== null) {
    return {
      ok: false,
      code: 'independent_with_effective_trials',
      reason: 'a study with independent trials must not declare an effective trial count: the nominal count is the effective count',
    };
  }
  if (dependence === 'dependent' && effectiveTrials === null) {
    return {
      ok: false,
      code: 'dependent_without_effective_trials',
      reason: 'a study with dependent trials must declare its effective trial count',
    };
  }
  if (dependence === 'dependent' && effectiveTrials < 1) {
    return { ok: false, code: 'non_positive_effective_trials', reason: `effective_trials must be positive, was ${effectiveTrials}` };
  }
  if (dependence === 'dependent' && effectiveTrials > trialSharpes.length) {
    return {
      ok: false,
      code: 'effective_exceeds_nominal',
      reason: `effective_trials ${effectiveTrials} exceeds the nominal count ${trialSharpes.length}`,
    };
  }

  const minimumObservations = contract.minimumObservations ?? DEFAULT_MINIMUM_OBSERVATIONS;
  const minimumTrials = contract.minimumTrials ?? DEFAULT_MINIMUM_TRIALS;
  const divisor = contract.trialVarianceDivisor ?? 'sample';

  const nominalTrials = trialSharpes.length;
  const countedTrials = dependence === 'dependent' ? effectiveTrials : nominalTrials;
  const trialVariance = nominalTrials >= MINIMUM_MOMENT_OBSERVATIONS ? variance(trialSharpes, divisor) : null;

  const base = {
    ok: true,
    countedTrials,
    nominalTrials,
    observations,
    trialVariance,
    dependence,
    effectiveTrials,
  };

  if (observations < minimumObservations || countedTrials < minimumTrials || trialVariance === null) {
    return { ...base, status: 'unavailable', reason: 'insufficient_data', value: null };
  }

  const expectedMaximum =
    Math.sqrt(trialVariance) *
    ((1 - EULER) * normalPpf(1 - 1 / countedTrials) + EULER * normalPpf(1 - 1 / (countedTrials * Math.E)));

  const varianceFactor = 1 - skew * sharpe + ((kurtosis - 1) / 4) * sharpe * sharpe;
  if (varianceFactor <= 0) {
    return { ...base, status: 'invalid', reason: 'undefined_result', value: null, expectedMaximum, varianceFactor };
  }

  const argument = ((sharpe - expectedMaximum) * Math.sqrt(observations - 1)) / Math.sqrt(varianceFactor);
  const value = normalCdf(argument);
  if (!Number.isFinite(value)) {
    return { ...base, status: 'invalid', reason: 'undefined_result', value: null, expectedMaximum, varianceFactor };
  }

  return { ...base, status: 'computed', reason: null, value, expectedMaximum, varianceFactor };
}

/** Every index combination of `choose` from `count`, in lexicographic order. */
function combinations(count, choose) {
  const result = [];
  const current = [];
  const walk = (start) => {
    if (current.length === choose) {
      result.push(current.slice());
      return;
    }
    for (let i = start; i < count; i += 1) {
      current.push(i);
      walk(i + 1);
      current.pop();
    }
  };
  walk(0);
  return result;
}

/** The mean of a block's values, treating a missing value as no observation. */
function blockMean(values) {
  const contributing = values.filter((value) => value !== null && value !== undefined);
  return contributing.length === 0 ? 0.0 : fsum(contributing) / contributing.length;
}

/** The ascending numeric stamps of a row's return mapping. */
function sortedStamps(returns) {
  return Object.keys(returns ?? {})
    .map(Number)
    .sort((a, b) => a - b);
}

/**
 * Evaluate the probability of backtest overfitting against its preconditions, then compute it.
 *
 * The preconditions are checked before the statistic, because a value computed on a conveniently
 * selected subset carries the authority of a number and is worse than no value. A row is excluded
 * when it was ineligible, when it ran and failed, or when it left a missing observation; the
 * survivors must share one common index and there must be enough of them for an even block count to
 * mean anything. The result is a diagnostic and rejects nothing, so `diagnosticOnly` is always
 * true.
 *
 * @param {object} diagnostic `{ rows, blocks }`, where a row is `{ configuration, eligible,
 *   ineligibleRule, failureClass, returns }` and `returns` maps a stamp to a value or null.
 * @returns {object} the four-state evaluation as plain data.
 */
export function pbo(diagnostic = {}) {
  const rows = diagnostic.rows ?? [];
  const blocks = diagnostic.blocks ?? PBO_DEFAULT_BLOCKS;

  const exclusions = [];
  const survivors = [];
  for (const row of rows) {
    if (!row.eligible) {
      exclusions.push({ configuration: row.configuration, state: 'ineligible', reason: row.ineligibleRule ?? '' });
    } else if (row.failureClass) {
      exclusions.push({ configuration: row.configuration, state: 'eligible_but_failed', reason: row.failureClass });
    } else if (!row.returns || Object.keys(row.returns).length === 0 || Object.values(row.returns).some((value) => value === null)) {
      exclusions.push({ configuration: row.configuration, state: 'eligible_but_failed', reason: 'missing_observations' });
    } else {
      survivors.push(row);
    }
  }

  const notComputable = (reason) => ({
    ok: true,
    state: 'not_computable',
    probability: null,
    blocks,
    combinations: 0,
    configurations: survivors.length,
    exclusions,
    logitMean: null,
    logitQuantiles: {},
    reason,
    diagnosticOnly: true,
  });

  const precondition = preconditionFailure(survivors, blocks);
  if (precondition) return notComputable(precondition);

  const index = sortedStamps(survivors[0].returns);
  const size = Math.floor(index.length / blocks);
  const chunks = [];
  for (let position = 0; position < blocks; position += 1) {
    chunks.push(index.slice(position * size, (position + 1) * size));
  }
  const half = Math.floor(blocks / 2);
  const configurations = survivors.length;

  const logits = [];
  for (const combination of combinations(blocks, half)) {
    const chosen = new Set(combination);
    const inSample = [];
    const outSample = [];
    chunks.forEach((chunk, number) => {
      (chosen.has(number) ? inSample : outSample).push(...chunk);
    });

    const inMeans = survivors.map((row) => blockMean(inSample.map((stamp) => row.returns[stamp])));
    let best = 0;
    for (let i = 1; i < configurations; i += 1) if (inMeans[i] > inMeans[best]) best = i;
    const outMeans = survivors.map((row) => blockMean(outSample.map((stamp) => row.returns[stamp])));

    const rank = outMeans.filter((value) => value < outMeans[best]).length;
    const relative = (rank + 1) / (configurations + 1);
    logits.push(Math.log(relative / (1 - relative)));
  }

  const ordered = [...logits].sort((a, b) => a - b);
  const below = logits.filter((logit) => logit < 0.0).length;

  return {
    ok: true,
    state: 'valid_with_documented_exclusions',
    probability: below / logits.length,
    blocks,
    combinations: logits.length,
    configurations,
    exclusions,
    logitMean: fsum(logits) / logits.length,
    logitQuantiles: {
      minimum: ordered[0],
      median: ordered[Math.floor(ordered.length / 2)],
      maximum: ordered[ordered.length - 1],
    },
    reason: `computed over ${logits.length} combinations of ${blocks} blocks and ${configurations} comparable configurations, with ${exclusions.length} exclusions accounted for`,
    diagnosticOnly: true,
  };
}

/** Why the matrix's preconditions do not hold, or the empty string when they do. */
function preconditionFailure(survivors, blocks) {
  if (survivors.length < PBO_MINIMUM_SURVIVORS) {
    return `${survivors.length} rows survived, and a comparison needs ${PBO_MINIMUM_SURVIVORS}`;
  }
  if (blocks < PBO_MINIMUM_BLOCKS || blocks % 2 !== 0) {
    return `the block count ${blocks} is not an even count of at least ${PBO_MINIMUM_BLOCKS}`;
  }
  const index = sortedStamps(survivors[0].returns);
  for (const row of survivors.slice(1)) {
    const stamps = sortedStamps(row.returns);
    if (stamps.length !== index.length || stamps.some((stamp, i) => stamp !== index[i])) {
      return `${row.configuration} does not share the common index of the first row`;
    }
  }
  if (index.length < blocks * 2) {
    return `the common index holds ${index.length} observations, fewer than the ${blocks * 2} needed`;
  }
  return '';
}
