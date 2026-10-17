/**
 * The gate engine: the six-question chain that stops at its first no, and the pre-campaign
 * admission assessment that compares a proposed search's size with what its sample can support.
 *
 * WHY THIS MODULE EXISTS. A candidate must pass a fixed list of six questions, asked in one order,
 * and the reading stops at the first no: a later question that would have passed does not excuse an
 * earlier failure, so it is recorded as never asked rather than as passed. Before a campaign opens,
 * the admission assessment computes the budget, inverts the deflated Sharpe relation for the ceiling
 * a trial count requires, and returns admit, narrow or refuse with its arithmetic. `walkGates`
 * models `GateService.evaluate`, the per-gate functions and the refused thresholds of
 * `python/nautilus_trader/discovery/gates.py`; `admissionTest` models `assess_admission` and
 * `minimum_track_record_length` in the same file.
 *
 * Pure: no DOM, no I/O, no randomness. Identical inputs give identical outputs.
 */

import { valueOf } from '../data/source.js';
import {
  GATE_POLICY_IDENTITY,
  GATE_POLICY_THRESHOLDS,
  GATE_NAMES,
  GATE_QUESTIONS,
  GATE_VERDICTS,
  ADMISSION_OUTCOMES,
  ADMISSION_VERSION,
  GATE_TOKENS,
} from '../data/gates.js';
import { correction, perPeriodSharpe, returnMoments } from './counts.js';

/** The pre-registered policy thresholds, read from the transcription so the numbers live once. */
export const POLICY_DEFAULTS = {
  minimumDeflatedSharpe: valueOf(GATE_POLICY_THRESHOLDS[0]),
  minimumNetSharpe: valueOf(GATE_POLICY_THRESHOLDS[1]),
  minimumBaselineMargin: valueOf(GATE_POLICY_THRESHOLDS[2]),
  maximumSingleNameShare: valueOf(GATE_POLICY_THRESHOLDS[3]),
  maximumSingleSectorShare: valueOf(GATE_POLICY_THRESHOLDS[4]),
  maximumSinglePeriodShare: valueOf(GATE_POLICY_THRESHOLDS[5]),
  neighbourhoodTolerance: valueOf(GATE_POLICY_THRESHOLDS[6]),
  minimumNeighbourhoodAgreement: valueOf(GATE_POLICY_THRESHOLDS[7]),
  maximumDrawdown: valueOf(GATE_POLICY_THRESHOLDS[8]),
  minimumPositiveFoldShare: valueOf(GATE_POLICY_THRESHOLDS[9]),
  maximumUnusableShare: valueOf(GATE_POLICY_THRESHOLDS[10]),
};

const MISSING = valueOf(GATE_TOKENS[0]);
const MINIMUM_SAMPLE = valueOf(GATE_TOKENS[1]);
const MAXIMUM_SAMPLE = valueOf(GATE_TOKENS[2]);
const MINIMUM_NEIGHBOURS = valueOf(GATE_TOKENS[3]);

/** The fixed verdict vocabulary, in the order the enum declares it. */
export const VERDICTS = GATE_VERDICTS.map(valueOf);

/** The admission outcomes, in the order the enum declares them. */
export const ADMISSION = ADMISSION_OUTCOMES.map(valueOf);

/** Round half to even, matching Python's `round` at the exact halves the budget formula hits. */
function pythonRound(value) {
  const floor = Math.floor(value);
  const fraction = value - floor;
  if (fraction > 0.5) return floor + 1;
  if (fraction < 0.5) return floor;
  return floor % 2 === 0 ? floor : floor + 1;
}

/** The values of a return series whether it is an array or a stamp-keyed mapping. */
function valuesOf(returns) {
  if (returns === undefined || returns === null) return [];
  return Array.isArray(returns) ? returns : Object.values(returns);
}

/**
 * The gate ladder: the six gates in the order they run, each with the question it asks, the policy
 * thresholds it reads and the verdicts that stop the walk.
 *
 * @returns {Array<object>}
 */
export function LADDER() {
  const q = GATE_QUESTIONS.map(valueOf);
  const names = GATE_NAMES.map(valueOf);
  return [
    { gate: names[0], question: q[0], thresholds: [], stops: ['invalid'] },
    {
      gate: names[1],
      question: q[1],
      thresholds: [
        { name: 'minimum_net_sharpe', value: POLICY_DEFAULTS.minimumNetSharpe },
        { name: 'minimum_baseline_margin', value: POLICY_DEFAULTS.minimumBaselineMargin },
      ],
      stops: ['rejected', 'inconclusive'],
    },
    {
      gate: names[2],
      question: q[2],
      thresholds: [{ name: 'minimum_positive_fold_share', value: POLICY_DEFAULTS.minimumPositiveFoldShare }],
      stops: ['rejected', 'inconclusive'],
    },
    {
      gate: names[3],
      question: q[3],
      thresholds: [
        { name: 'maximum_unusable_share', value: POLICY_DEFAULTS.maximumUnusableShare },
        { name: 'minimum_deflated_sharpe', value: POLICY_DEFAULTS.minimumDeflatedSharpe },
      ],
      stops: ['rejected', 'inconclusive'],
    },
    {
      gate: names[4],
      question: q[4],
      thresholds: [
        { name: 'maximum_single_name_share', value: POLICY_DEFAULTS.maximumSingleNameShare },
        { name: 'maximum_single_sector_share', value: POLICY_DEFAULTS.maximumSingleSectorShare },
        { name: 'maximum_single_period_share', value: POLICY_DEFAULTS.maximumSinglePeriodShare },
        { name: 'neighbourhood_tolerance', value: POLICY_DEFAULTS.neighbourhoodTolerance },
        { name: 'minimum_neighbourhood_agreement', value: POLICY_DEFAULTS.minimumNeighbourhoodAgreement },
        { name: 'maximum_drawdown', value: POLICY_DEFAULTS.maximumDrawdown },
      ],
      stops: ['rejected'],
    },
    { gate: names[5], question: q[5], thresholds: [], stops: ['quarantined', 'validation_pending'] },
  ];
}

/** G0: entitlement to the information used, at the time it was used. */
function gateDataIntegrity(evidence) {
  const defects = [];
  const timing = [...(evidence.timingViolations ?? [])].sort();
  const features = [...(evidence.featureAvailabilityDefects ?? [])].sort();
  const entitlements = [...(evidence.entitlementDefects ?? [])].sort();
  if (timing.length) defects.push(`timing cases violated: ${JSON.stringify(timing)}`);
  if (features.length) defects.push(`features unavailable: ${JSON.stringify(features)}`);
  if (entitlements.length) defects.push(`entitlements unreconciled: ${JSON.stringify(entitlements)}`);
  if (!evidence.priceSeriesStatement) defects.push('no price-series statement: what the run consumed is unknown');

  const details = {
    timing_violations: timing.length,
    feature_availability_defects: features.length,
    entitlement_defects: entitlements.length,
    price_series_statement: evidence.priceSeriesStatement ?? '',
  };
  if (defects.length) return ['invalid', defects.join('; '), details];
  return ['passed', 'the run was entitled to what it consumed', details];
}

/** G1: survival of its own costs, against a registered baseline. */
function gateEconomicViability(evidence, policy) {
  const returns = valuesOf(evidence.returns);
  const benchmark = valuesOf(evidence.benchmarkReturns);
  const missing = [];
  if (!returns.length) missing.push('returns');
  if (!evidence.costConfiguration) missing.push('cost_configuration');
  if (!benchmark.length) missing.push('benchmark_returns');
  if (missing.length) {
    return ['inconclusive', `the candidate declares no ${missing}, so nothing can be compared`, { [MISSING]: missing }];
  }

  const estimate = perPeriodSharpe(returns);
  const baseline = perPeriodSharpe(benchmark);
  const details = {
    net_sharpe_per_period: estimate.value,
    baseline_sharpe_per_period: baseline.value,
    observations: estimate.observations,
  };

  if (evidence.includesDistributions && !evidence.cashPathDemonstrated) {
    return ['inconclusive', 'the return includes distributions and the cash path is not demonstrated', details];
  }
  if (estimate.value === null || baseline.value === null) {
    return ['inconclusive', 'the candidate or the baseline produced no estimate, so neither is comparable', details];
  }

  const margin = estimate.value - baseline.value;
  details.margin = margin;
  if (estimate.value <= policy.minimumNetSharpe || margin < policy.minimumBaselineMargin) {
    return ['rejected', `the net Sharpe ${estimate.value} and margin ${margin} fail the declared floors`, details];
  }
  return ['passed', 'the candidate survives its costs and beats its baseline', details];
}

/** G2: persistence out of sample, over locked validation data. */
function gateWalkForward(evidence, policy) {
  const role = evidence.dataRole ?? 'development';
  const folds = evidence.foldReturns ?? [];
  const details = { data_role: role, folds: folds.length, split_declared: evidence.split !== undefined && evidence.split !== null };

  if (role !== 'locked_validation') {
    return ['inconclusive', `the data role is ${role} and only locked validation data is admissible`, details];
  }
  const absent = [];
  if (evidence.split === undefined || evidence.split === null) absent.push('split');
  if (!folds.length) absent.push('fold_returns');
  if (absent.length) {
    return ['inconclusive', `the walk-forward evaluation does not declare ${absent}`, { [MISSING]: absent }];
  }

  const means = [];
  let positive = 0;
  for (const fold of folds) {
    const values = fold.filter((value) => value !== null && value !== undefined);
    const mean = values.length ? values.reduce((a, b) => a + b, 0) / values.length : 0.0;
    means.push(mean);
    if (mean > 0) positive += 1;
  }
  const share = positive / folds.length;
  const full = { ...details, folds_positive: positive, positive_share: share, means };
  if (share < policy.minimumPositiveFoldShare) {
    return ['rejected', `${positive} of ${means.length} folds are positive, below the declared share`, full];
  }
  return ['passed', 'the candidate persists out of sample across the folds', full];
}

/** G3: distinguishability from the best of a search this size. */
function gateStatisticalCredibility(evidence, policy) {
  const executed = evidence.executedExperiments ?? 0;
  const unusable = evidence.unusableAttempts ?? 0;
  const trialSharpes = evidence.trialSharpes ?? [];
  const details = {
    statistical_trials: evidence.statisticalTrials ?? 0,
    executed_experiments: executed,
    unusable_attempts: unusable,
    trial_estimates: trialSharpes.length,
  };

  if (executed > 0) {
    const unusableShare = unusable / executed;
    details.unusable_share = unusableShare;
    if (unusableShare > policy.maximumUnusableShare) {
      return ['inconclusive', `${unusable} of ${executed} attempts produced no usable return series`, details];
    }
  }

  const absent = [];
  if ((evidence.statisticalTrials ?? 0) < 1) absent.push('statistical_trials');
  if (!trialSharpes.length) absent.push('trial_sharpes');
  if (absent.length) {
    return ['inconclusive', `the inventory declares no ${absent}, so there is no multiplicity to read`, { [MISSING]: absent }];
  }

  const returns = valuesOf(evidence.returns);
  const estimate = perPeriodSharpe(returns);
  const moments = returnMoments(returns);
  if (estimate.value === null || moments === null) {
    return ['inconclusive', 'the selected estimate or its moments are undefined for this series', { ...details, candidate_sharpe: estimate.value, moments: moments !== null }];
  }

  const result = correction({
    sharpe: estimate.value,
    trialSharpes,
    observations: estimate.observations,
    skew: moments.skew,
    kurtosis: moments.kurtosis,
    dependence: evidence.dependence ?? 'independent',
    effectiveTrials: evidence.effectiveTrials ?? null,
  });
  if (!result.ok) {
    return ['inconclusive', `the trial declaration is refused: ${result.reason}`, details];
  }

  const full = {
    ...details,
    candidate_sharpe: estimate.value,
    nominal_trials: result.nominalTrials,
    counted_trials: result.countedTrials,
    deflated_sharpe: result.value,
    status: result.status,
    reason: result.reason,
  };
  if (result.status !== 'computed') {
    return ['inconclusive', `the deflated Sharpe ratio was not computable (${result.reason})`, full];
  }
  if (result.value < policy.minimumDeflatedSharpe) {
    return ['rejected', `the deflated Sharpe ratio ${result.value} is below the declared threshold`, full];
  }
  return ['passed', `the result is distinguishable from the best of ${result.countedTrials} trials`, full];
}

/** The concentration defects one dimension contributes, recording the worst share. */
function concentrationDefects(label, shares, ceiling, details) {
  if (!shares || Object.keys(shares).length === 0) return [`no ${label} concentration is declared`];
  let worstKey = null;
  let worstValue = -Infinity;
  for (const [key, value] of Object.entries(shares)) {
    if (value > worstValue) {
      worstValue = value;
      worstKey = key;
    }
  }
  details[`${label}_share`] = { [worstKey]: worstValue };
  if (worstValue > ceiling) return [`${worstKey} contributes ${worstValue} of the ${label} ceiling ${ceiling}`];
  return [];
}

/** The share of neighbours that reproduce, or why the check cannot be made. */
function neighbourhoodDefect(objectives, tolerance, minimumAgreement) {
  if (!objectives || objectives.length < MINIMUM_NEIGHBOURS) return 'no parameter neighbourhood is declared';
  const selected = objectives[0];
  if (selected <= 0) return "the selected parameter's objective is not positive, so a neighbourhood is undefined";
  const agreeing = objectives.slice(1).filter((value) => value >= selected * (1 - tolerance));
  const share = agreeing.length / (objectives.length - 1);
  if (share < minimumAgreement) return `only ${share} of the declared neighbours reproduce within ${tolerance}`;
  return share;
}

/** G4: independence from a lucky name, period, or parameter value. */
function gateRobustness(evidence, policy) {
  const details = {};
  const defects = [];

  for (const [label, shares, ceiling] of [
    ['instrument', evidence.nameShares, policy.maximumSingleNameShare],
    ['sector', evidence.sectorShares, policy.maximumSingleSectorShare],
    ['period', evidence.periodShares, policy.maximumSinglePeriodShare],
  ]) {
    defects.push(...concentrationDefects(label, shares, ceiling, details));
  }

  const neighbourhood = neighbourhoodDefect(
    evidence.neighbourhoodObjectives ?? [],
    policy.neighbourhoodTolerance,
    policy.minimumNeighbourhoodAgreement,
  );
  if (typeof neighbourhood === 'string') defects.push(neighbourhood);
  else details.neighbourhood_agreement = neighbourhood;

  const stress = evidence.stressReturns ?? {};
  if (Object.keys(stress).length === 0) defects.push('no stress scenario is declared');
  for (const scenario of Object.keys(stress).sort()) {
    details[`stress:${scenario}`] = stress[scenario];
    if (stress[scenario] <= 0) defects.push(`the ${scenario} stress turns the mean return negative (${stress[scenario]})`);
  }

  details.drawdown = evidence.drawdown ?? 0;
  if (details.drawdown >= policy.maximumDrawdown) {
    defects.push(`the drawdown ${details.drawdown} reaches ${policy.maximumDrawdown}`);
  }

  if (defects.length) return ['rejected', defects.join('; '), details];
  return ['passed', 'the result does not depend on one name, period or value', details];
}

/** G5: the one holdout evaluation, the frozen artifact and the human decision. */
function gateHoldoutReadiness(evidence) {
  const details = {
    confirmation_evaluation: evidence.confirmationEvaluation ?? '',
    frozen_specification_digest: evidence.frozenSpecificationDigest ?? '',
    evidence_package_complete: Boolean(evidence.evidencePackageComplete),
    human_decision: evidence.humanDecision ?? '',
  };

  const quarantine = [...(evidence.quarantineReasons ?? [])].sort();
  if (quarantine.length) {
    return ['quarantined', `quarantine follows evidence of a defect: ${JSON.stringify(quarantine)}`, details];
  }
  const frozen = evidence.frozenSpecificationDigest ?? '';
  if (frozen && frozen !== (evidence.specificationDigest ?? '')) {
    return ['quarantined', `the holdout was made under ${frozen}: the identity does not match`, details];
  }

  const pending = [];
  if (evidence.custody && evidence.custody.verdict === null) pending.push('the campaign holds no confirmation verdict');
  if (!details.confirmation_evaluation) pending.push('no recorded holdout evaluation');
  if (!details.evidence_package_complete) pending.push('the evidence package is not complete');
  if (!details.human_decision) pending.push('no human decision is recorded');
  if (pending.length) return ['validation_pending', pending.join('; '), details];
  return ['passed', 'the frozen artifact is complete and a human decided', details];
}

/**
 * Walk the gate ladder over one candidate's evidence, stopping at the first non-pass.
 *
 * The gates run G0 to G5 and each is a conjunction. The loop stops at the first verdict that blocks,
 * so no later gate's strength compensates an earlier failure; the gates the walk never reached are
 * reported as never asked (`asked: false`) rather than as passed, and `stoppedAt` names the gate
 * that stopped it. A gate reports a missing declared input as `inconclusive` with the token
 * `inputs_missing` and the input's name, rather than substituting a default.
 *
 * @param {object} evidence the candidate's declared evidence.
 * @param {object} [policy] a policy overriding `POLICY_DEFAULTS`.
 * @returns {object} `{ ok, candidateId, records, states, neverAsked, stoppedAt, outcome, policy }`.
 */
export function walkGates(evidence, policy = {}) {
  const settings = { ...POLICY_DEFAULTS, ...policy };
  const names = GATE_NAMES.map(valueOf);
  const evaluators = [
    (e) => gateDataIntegrity(e),
    (e) => gateEconomicViability(e, settings),
    (e) => gateWalkForward(e, settings),
    (e) => gateStatisticalCredibility(e, settings),
    (e) => gateRobustness(e, settings),
    (e) => gateHoldoutReadiness(e),
  ];

  const records = [];
  const states = [];
  let stoppedAt = null;
  let outcome = 'passed';

  for (let i = 0; i < names.length; i += 1) {
    if (stoppedAt !== null) {
      states.push({ gate: names[i], asked: false, verdict: null });
      continue;
    }
    const [verdict, reason, details] = evaluators[i](evidence);
    records.push({ gate: names[i], verdict, reason, evidence: details });
    states.push({ gate: names[i], asked: true, verdict });
    if (verdict !== 'passed') {
      stoppedAt = names[i];
      outcome = verdict;
    }
  }

  return {
    ok: true,
    candidateId: evidence.candidateId,
    records,
    states,
    neverAsked: states.filter((state) => !state.asked).map((state) => state.gate),
    stoppedAt,
    outcome,
    policy: { id: valueOf(GATE_POLICY_IDENTITY[0]), version: valueOf(GATE_POLICY_IDENTITY[1]) },
  };
}

/** The observations the ceiling may count, after the folds, the embargo and the holdout. */
export function usableObservations(inputs) {
  const effective = inputs.effectiveObservations ?? 0;
  const reserved = (inputs.reservedObservations ?? 0) + pythonRound(effective * (inputs.holdoutFraction ?? 0));
  return Math.max(0, effective - reserved);
}

/** The trial count the correction counts, or null when it cannot be estimated. */
export function countedTrials(inputs) {
  const spent = inputs.trialsAlreadyConsumed ?? 0;
  if ((inputs.dependence ?? 'independent') === 'dependent') {
    if (inputs.effectiveTrials === undefined || inputs.effectiveTrials === null) return null;
    return inputs.effectiveTrials + spent;
  }
  return (inputs.plannedConfigurations ?? 0) + spent;
}

/** Invert the deflated Sharpe relation for the smallest sample that reaches a significance. */
export function minimumTrackRecordLength(countsTrials, assumptions, contract = {}) {
  const dispersion = assumptions.crossTrialDispersion ?? 0;
  const significance = assumptions.significance ?? 0.95;
  if (countsTrials < 1 || dispersion <= 0) return null;
  if (!(significance > 0 && significance < 1)) return null;

  const spread = dispersion * Math.sqrt((countsTrials - 1) / countsTrials);
  const positives = Math.floor(countsTrials / 2);
  const estimates = [
    ...Array(positives).fill(spread),
    ...Array(countsTrials - positives).fill(-spread),
  ];
  const minimumObservations = contract.minimumObservations ?? 20;
  const lowest = Math.max(MINIMUM_SAMPLE, minimumObservations);
  const highest = MAXIMUM_SAMPLE;

  const reached = (length) => {
    const result = correction({
      sharpe: assumptions.targetSharpe ?? 0,
      trialSharpes: estimates,
      observations: length,
      skew: assumptions.skew ?? 0,
      kurtosis: assumptions.kurtosis ?? 3,
      dependence: 'independent',
    }, contract);
    return result.ok && result.status === 'computed' && result.value >= significance;
  };

  if (reached(lowest)) return lowest;
  if (!reached(highest)) return null;

  let low = lowest;
  let high = highest;
  while (low + 1 < high) {
    const middle = Math.floor((low + high) / 2);
    if (reached(middle)) high = middle;
    else low = middle;
  }
  return high;
}

/** The sample the declared inputs need for a configuration count. */
export function requiredObservations(inputs, configurations, contract) {
  if (inputs.crossTrialDispersion === undefined || inputs.crossTrialDispersion === null) return null;
  let counted = configurations;
  if ((inputs.dependence ?? 'independent') === 'dependent') {
    if (inputs.effectiveTrials === undefined || inputs.effectiveTrials === null) return null;
    counted = inputs.effectiveTrials;
  }
  const assumptions = {
    targetSharpe: inputs.targetSharpe ?? 0,
    crossTrialDispersion: inputs.crossTrialDispersion,
    significance: inputs.significance ?? 0.95,
  };
  return minimumTrackRecordLength(counted + (inputs.trialsAlreadyConsumed ?? 0), assumptions, contract);
}

/** Why the assessment cannot estimate what it needs, or the empty string when it can. */
function unestimable(inputs) {
  if ((inputs.dependence ?? 'independent') === 'dependent' && (inputs.effectiveTrials === undefined || inputs.effectiveTrials === null)) {
    return 'the configurations are dependent and no effective trial count is estimated';
  }
  if ((inputs.effectiveObservations ?? 0) <= 0) {
    return 'the effective number of independent observations is not estimated';
  }
  if (inputs.crossTrialDispersion === undefined || inputs.crossTrialDispersion === null) {
    return 'the cross-trial dispersion of the Sharpe estimates is not estimated';
  }
  const counted = countedTrials(inputs);
  if (counted === null || counted < 1) {
    return 'the campaign proposes no configuration, so there is nothing to assess';
  }
  return '';
}

/** The largest configuration count whose ceiling fits, or null when none does. */
function largestThatFits(inputs, required, usable) {
  for (let candidate = (inputs.plannedConfigurations ?? 0) - 1; candidate >= 1; candidate -= 1) {
    const ceiling = required(candidate);
    if (ceiling !== null && ceiling <= usable) return candidate;
  }
  return null;
}

/** How the ceiling moves when one input is perturbed by a declared factor. */
function sensitivity(inputs, required, fieldName, factor = 0.1) {
  const base = inputs[fieldName];
  if (typeof base !== 'number' || base === 0) {
    return { factor, measurements: {}, note: 'the input cannot be perturbed' };
  }
  const measurements = {};
  for (const [label, multiplier] of [['lower', 1 - factor], ['upper', 1 + factor]]) {
    measurements[label] = required(inputs.plannedConfigurations ?? 0, { [fieldName]: base * multiplier });
  }
  return { factor, measurements };
}

/**
 * Assess whether a proposed search fits what its sample can support, showing its arithmetic.
 *
 * The assessment answers before the first backtest runs: it computes the usable observations `U`,
 * the counted trials `K`, inverts the deflated Sharpe relation for the ceiling `T*(K)`, and settles
 * it - `admit` when the ceiling fits, `reduce` to the largest configuration count that does, and
 * `reject` otherwise. An input it cannot estimate is a `reject` rather than a guess, and the
 * declared power is reported as an assumption it cannot verify from the relation.
 *
 * @param {object} inputs the declared admission inputs.
 * @param {object} [policy] the gate policy, which supplies the statistical contract.
 * @returns {object} the assessment as plain data, with its arithmetic.
 */
export function admissionTest(inputs, policy = {}) {
  const contract = policy.contract ?? {};
  const usable = usableObservations(inputs);
  const counted = countedTrials(inputs);
  const limitations = [
    `the relation is a significance relation, not a power calculation: the declared power ${inputs.power ?? 0.8} cannot be verified from it`,
  ];

  const reject = (reason, assumptions = {}) => ({
    ok: true,
    outcome: 'reject',
    requiredObservations: null,
    usableObservations: usable,
    countedTrials: counted,
    effectiveTrials: inputs.effectiveTrials ?? null,
    reducedConfigurations: null,
    inputs,
    assumptions,
    limitations,
    supports: ['nothing: the assessment could not estimate what it needs'],
    withholds: ['any claim about the campaign, because an input cannot be estimated'],
    reason,
    version: valueOf(ADMISSION_VERSION),
    arithmetic: { usable, counted },
  });

  const failure = unestimable(inputs);
  if (failure) return reject(failure);

  const required = (configurations, overrides = {}) =>
    requiredObservations({ ...inputs, ...overrides }, configurations, contract);

  const ceiling = required(inputs.plannedConfigurations ?? 0);
  const assumptions = {
    cross_trial_dispersion: {
      value: inputs.crossTrialDispersion,
      sensitivity: sensitivity(inputs, required, 'crossTrialDispersion'),
    },
    target_sharpe: {
      value: inputs.targetSharpe ?? 0,
      sensitivity: sensitivity(inputs, required, 'targetSharpe'),
    },
    dependence: {
      value: inputs.dependence ?? 'independent',
      note: 'the dependence declaration decides whether the raw or effective count is used',
    },
  };

  if (ceiling === null) {
    return reject(
      `no sample distinguishes the declared target Sharpe ${inputs.targetSharpe ?? 0} from what ${counted} trials would produce by chance, so no ceiling is claimed`,
      assumptions,
    );
  }

  const supports = [
    `a claim that the search fits the sample at a per-period Sharpe of ${inputs.targetSharpe ?? 0} and ${counted} counted trials`,
  ];
  const withholds = [
    'any claim of future profitability, which no admission assessment can make',
    'any claim beyond the declared target effect size, because the ceiling moves with it',
  ];

  if (ceiling <= usable) {
    return {
      ok: true,
      outcome: 'admit',
      requiredObservations: ceiling,
      usableObservations: usable,
      countedTrials: counted,
      effectiveTrials: inputs.effectiveTrials ?? null,
      reducedConfigurations: null,
      inputs,
      assumptions,
      limitations,
      supports,
      withholds,
      reason: `the plan fits: ${Math.round(ceiling)} required observations against ${usable} usable`,
      version: valueOf(ADMISSION_VERSION),
      arithmetic: { usable, counted },
    };
  }

  const reduced = largestThatFits(inputs, required, usable);
  if (reduced !== null && reduced >= 1) {
    return {
      ok: true,
      outcome: 'reduce',
      requiredObservations: ceiling,
      usableObservations: usable,
      countedTrials: counted,
      effectiveTrials: inputs.effectiveTrials ?? null,
      reducedConfigurations: reduced,
      inputs,
      assumptions,
      limitations,
      supports: [`a claim that a search of ${reduced} configurations fits the sample`],
      withholds: [`the proposed ${inputs.plannedConfigurations ?? 0}-configuration plan`],
      reason: `the plan exceeds its ceiling and narrows to ${reduced} configurations`,
      version: valueOf(ADMISSION_VERSION),
      arithmetic: { usable, counted },
    };
  }

  return {
    ok: true,
    outcome: 'reject',
    requiredObservations: ceiling,
    usableObservations: usable,
    countedTrials: counted,
    effectiveTrials: inputs.effectiveTrials ?? null,
    reducedConfigurations: null,
    inputs,
    assumptions,
    limitations,
    supports: ['nothing: the data cannot support the proposed claims at any allocation'],
    withholds: ['every claim in the plan, including a narrowed one'],
    reason: `no allocation fits: one configuration needs ${Math.round(ceiling)} against ${usable} usable`,
    version: valueOf(ADMISSION_VERSION),
    arithmetic: { usable, counted },
  };
}
