/**
 * One campaign, composed through every layer.
 *
 * Models the discovery layer as a whole rather than layer by layer: it freezes a policy, validates
 * the campaign's declared candidate as data, opens the ledger and appends the campaign's events,
 * allocates the ranking, folds the four counts, takes the campaign's one confirmation reservation,
 * walks the pre-registered gate ladder, assesses the search the sample can support, breeds children
 * from the declared candidate with the search's own operators, classifies a recorded proposal,
 * watches the frozen candidate against a feed, reads the ledger back as an audit, and checks the
 * execution boundary.
 *
 * THE HONESTY OF THIS COMPOSITION
 * - It is one deterministic path, not a measurement. Every stage calls the layer's own engine with
 *   the campaign's declared inputs; where a stage would need something the repository does not have
 *   (a live feed, a model call, an unbuilt configuration module) the stage says so in its `detail`
 *   rather than substituting a value.
 * - When the gate ladder stops, the stage that depends on a judged candidate - the shadow period -
 *   is reported as NOT REACHED rather than run, because the design's rule is that the reading stops
 *   at the first no. The stages that fed the ladder have already happened, so they stay.
 * - The declared evidence the gates read is the gate page's worked candidate
 *   (`tutorials/strategy-discovery/the-admission-gate/README.md`), with the campaign's own candidate
 *   identity and specification digest substituted for the fixture's. Nothing about it is presented as
 *   a result: it is what the campaign declares.
 */

import { RECORDED_REPLIES } from '../data/agents.js';
import { declaredCandidate } from '../data/campaign-template.js';
import { credentials } from '../data/layers.js';
import { valueOf } from '../data/source.js';

import { classifyReply } from './agents.js';
import { derivedView, outcome } from './audit.js';
import { checkEnvironment } from './boundary.js';
import { validateSpecification } from './candidate.js';
import { countsFromEvents, correction } from './counts.js';
import { digestOf } from './digest.js';
import { TIMING_CAVEAT, allocate } from './executor.js';
import { admissionTest, walkGates } from './gates.js';
import { digestOfSpecification, equivalenceKey, genesOf, mutate, paretoFront } from './genome.js';
import { ROLES, initialReservation, reserve } from './holdout.js';
import { append, verifyChain } from './ledger.js';
import { defaultPolicy, freeze, validatePolicy } from './policy.js';
import { makeRng } from './rng.js';
import { TEMPORAL_CONTRACT, contractDigest, observe, shadowReport, watch } from './shadow.js';

/** The campaign's defaults. A reader changes these and the whole trace is recomputed. */
export const DEFAULT_OPTIONS = Object.freeze({
  label: 'campaign-1',
  seed: 20261017,
  rankingLength: 6,
  positionCount: 4,
  maxPositionWeight: 0.25,
  population: 6,
  objectives: ['return', 'turnover', 'drawdown'],
  observations: 250,
  declaredSharpe: 0.25,
  targetSharpe: 0.15,
  crossTrialDispersion: 0.02,
  effectiveObservations: 300,
  holdoutFraction: 0.2,
  plannedConfigurations: 45,
  defect: 'none',
});

/** The feature registry the campaign validates against: the one feature its candidate declares. */
const REGISTRY = { close: { availability_lag_ns: 0 }, volume: { availability_lag_ns: 0 } };

/** The declared evidence the gates read, with the campaign's own candidate identity substituted. */
function declaredEvidence(candidateId, specificationDigest, defect) {
  const returns = Object.fromEntries(
    Array.from({ length: 60 }, (_, i) => [i + 1, 0.004 + 0.002 * Math.sin(i)]),
  );
  const base = {
    candidateId,
    specificationDigest,
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
  };
  return { ...base, ...DEFECTS[defect]?.overrides ?? {} };
}

/** The defects a reader may plant: each breaks exactly one gate's condition. */
export const DEFECTS = Object.freeze({
  none: { label: 'a clean candidate', overrides: {} },
  g0: { label: 'G0: no price-series statement', overrides: { priceSeriesStatement: '' } },
  g1: { label: 'G1: no cost configuration', overrides: { costConfiguration: '' } },
  g2: { label: 'G2: built on development data', overrides: { dataRole: 'development' } },
  g3: {
    label: 'G3: fifteen unusable of twenty attempts',
    overrides: { executedExperiments: 20, unusableAttempts: 15 },
  },
  g4: { label: 'G4: one name contributes 0.9', overrides: { nameShares: { a: 0.9, b: 0.1 } } },
  g5: {
    label: 'G5: the holdout was leaked',
    overrides: { quarantineReasons: ['read the confirmation set'] },
  },
});

/** One stage of the trace. `ran: false` is how a stage the campaign never reached is reported. */
function stage({
  id,
  step,
  title,
  ran = true,
  verdict = '',
  tone = 'muted',
  detail = '',
  facts = [],
  refusals = [],
  rows = null,
  digest = null,
}) {
  return { id, step, title, ran, verdict, tone, detail, facts, refusals, rows, digest };
}

const notReached = (id, step, title, reason) =>
  stage({ id, step, title, ran: false, verdict: 'not reached', tone: 'muted', detail: reason });

/** A refusal as the campaign reports one: the layer's code and the condition the layer states. */
const refusalOf = (result) => ({ code: result.code, condition: result.reason });

/**
 * Run one campaign and return its whole trace.
 *
 * @param {object} [options] the campaign's declared inputs, merged over `DEFAULT_OPTIONS`.
 * @returns {object} `{ ok, options, stages, stoppedAt, refusals, ledger, audit, digests, timing }`.
 */
export function runCampaign(options = {}) {
  const settings = { ...DEFAULT_OPTIONS, ...options };
  const rng = makeRng(`${settings.seed}:${settings.label}`);
  const stages = [];
  // Only a refusal that stops a stage is a campaign refusal; a refusal the campaign demonstrates on
  // purpose (a second reservation, an ambient credential) is reported by its own stage.
  const refusals = [];

  /* 1. The rules: the thresholds are declared and frozen before anything is searched for. */
  const policy = defaultPolicy({
    featureAllowlist: ['close', 'volume'],
    // The references are campaign inputs the policy cannot invent, so the campaign declares one of
    // each kind, over the console's own digest of the object it names.
    references: ['benchmark', 'cost_model', 'validation_policy'].map((kind) => ({
      name: `campaign_${kind}`,
      kind,
      digest: digestOf({ kind, campaign: settings.label }),
    })),
  });
  const frozen = freeze(policy);
  const policyCheck = validatePolicy(policy);
  const policyRefused = policyCheck.ok === false;
  if (policyRefused) refusals.push(...(policyCheck.refusals ?? []).map(refusalOf));
  stages.push(
    stage({
      id: 'policy',
      step: 1,
      title: 'The rules nobody may choose',
      verdict: policyRefused ? 'refused' : 'frozen',
      tone: policyRefused ? 'bad' : 'ok',
      detail:
        'the search reads this policy and cannot change it: the sets, the bounds and the references are inputs to the campaign',
      digest: frozen.digest,
      facts: [
        ['declared sets', String(Object.keys(policy.sets).length)],
        ['bounds', String(policy.bounds.length)],
        ['references', String(policy.references.length)],
      ],
      refusals: policyRefused ? (policyCheck.refusals ?? []).map(refusalOf) : [],
    }),
  );

  /* 2. The candidate: the campaign's declared candidate, read as data before anything runs. */
  const specification = declaredCandidate({ seed: settings.seed });
  const validation = validateSpecification(specification, policy, { registry: REGISTRY });
  const candidateRefusals = (validation.refusals ?? []).map((item) => ({
    code: item.code,
    condition: item.path ? `${item.path}: ${item.reason ?? item.message ?? ''}` : item.reason ?? item.message ?? '',
  }));
  const specificationDigest = digestOfSpecification(specification);
  const candidateId = `candidate-${specificationDigest.slice(7, 15)}`;
  if (!validation.ok) refusals.push(...candidateRefusals);
  stages.push(
    stage({
      id: 'candidate',
      step: 2,
      title: 'What a candidate is',
      verdict: validation.ok ? 'compiled as data' : 'refused',
      tone: validation.ok ? 'ok' : 'bad',
      detail:
        'nothing was run to read this: the language walks the twelve blocks, resolves every feature and refuses what the policy does not cover',
      digest: specificationDigest,
      facts: [
        ['stage reached', String(validation.stage ?? 'schema_validation')],
        ['refusals', String(candidateRefusals.length)],
      ],
      refusals: candidateRefusals,
    }),
  );

  /* 3. The record: the campaign's own ledger, opened and appended to as the campaign proceeds. */
  let entries = [];
  const appendOne = (kind, payload) => {
    const result = append(entries, kind, payload);
    if (result.ok) {
      entries = result.entries;
      return true;
    }
    refusals.push(refusalOf(result));
    return false;
  };
  appendOne('proposal_recorded', { candidateId, generation: 0 });
  appendOne('compilation_recorded', {
    candidateId,
    specification_digest: specificationDigest,
    new_configuration: true,
  });
  appendOne('job_submitted', { candidateId, job_id: 'job-1' });
  stages.push(
    stage({
      id: 'record',
      step: 3,
      title: 'What ran against what was written down',
      verdict: `${entries.length} entries appended`,
      tone: entries.length > 0 ? 'ok' : 'bad',
      detail:
        'the ledger is append-only, and the chain value of each entry covers the one before it, so a removed or reordered record is visible against a checkpoint held elsewhere',
      digest: entries.length > 0 ? entries.at(-1).chain : null,
      facts: [
        ['entries', String(entries.length)],
        ['head', entries.length > 0 ? entries.at(-1).chain.slice(0, 19) : 'none'],
      ],
    }),
  );

  /* 4. The executor: one compiled candidate becomes a ranking, weights and instructions. */
  const ranking = Array.from({ length: settings.rankingLength }, (_, index) => `INST.${index + 1}`);
  const allocation = allocate(ranking, {
    positionCount: settings.positionCount,
    maxPositionWeight: settings.maxPositionWeight,
    weighting: 'rank_proportional',
    cashPolicy: 'leave_idle',
  });
  const allocationRefused = allocation.ok === false;
  if (allocationRefused) refusals.push(refusalOf(allocation));
  stages.push(
    stage({
      id: 'executor',
      step: 4,
      title: 'The one executor',
      verdict: allocationRefused ? 'refused' : 'allocated on a rebalance bar',
      tone: allocationRefused ? 'bad' : 'ok',
      detail: allocationRefused
        ? allocation.reason
        : `${settings.positionCount} of ${settings.rankingLength} ranked names, each capped at ${settings.maxPositionWeight}; the remainder is left idle rather than reallocated`,
      rows: allocationRefused
        ? null
        : allocation.rows.map((row) => [
            row.instrument,
            row.rank,
            row.rawWeight,
            row.cappedWeight,
            row.instructionStrength,
          ]),
      refusals: allocationRefused ? [refusalOf(allocation)] : [],
    }),
  );

  /* 5. The counting: the campaign's events folded once into four counts, then the correction. */
  const events = [
    { kind: 'proposal_recorded', key: '', candidateId, payload: {} },
    { kind: 'proposal_recorded', key: '', candidateId: `${candidateId}-b`, payload: {} },
    {
      kind: 'specification_refused',
      key: '',
      candidateId: `${candidateId}-b`,
      payload: { code: 'CAP_EXCEEDED' },
    },
    { kind: 'compilation_recorded', key: '', candidateId, payload: { new_configuration: true } },
    { kind: 'job_submitted', key: '', candidateId, payload: {} },
    { kind: 'execution_attempt_closed', key: '', candidateId, payload: { outcome: 'completed' } },
    { kind: 'registration_verified', key: '', candidateId, payload: {} },
    { kind: 'trial_admitted', key: 'job-1', candidateId, payload: {} },
  ];
  const fold = countsFromEvents(events);
  const foldRefused = fold.ok === false;
  if (foldRefused) refusals.push(refusalOf(fold));
  const diagnostic = foldRefused
    ? null
    : correction({
        sharpe: settings.declaredSharpe,
        trialSharpes: declaredEvidence(candidateId, specificationDigest, 'none').trialSharpes,
        observations: settings.observations,
        skew: 0,
        kurtosis: 3,
      });
  stages.push(
    stage({
      id: 'counts',
      step: 5,
      title: 'Counting the search',
      verdict: foldRefused ? 'refused' : 'folded once',
      tone: foldRefused ? 'bad' : 'ok',
      detail: foldRefused
        ? fold.reason
        : `${fold.mapping} over ${events.length} events: a proposal is not a configuration, a configuration is not an execution, and an execution is not a trial`,
      facts: foldRefused
        ? []
        : [
            ['proposals', String(fold.proposalAttempts)],
            ['configurations', String(fold.uniqueConfigurations)],
            ['executions', String(fold.executedExperiments)],
            ['trials', String(fold.statisticalTrials)],
            [
              'deflated Sharpe',
              diagnostic && diagnostic.status === 'computed'
                ? diagnostic.value.toFixed(4)
                : `not computed (${diagnostic ? diagnostic.status : 'no sample'})`,
            ],
          ],
      refusals: foldRefused ? [refusalOf(fold)] : [],
    }),
  );

  /* 6. The held-back data: the campaign's one confirmation reservation, then a second ask refused. */
  const roles = ROLES();
  const role = roles.find((item) => item.admissibleAsValidation)?.name ?? roles[0].name;
  let reservation = initialReservation();
  const firstReserve = reserve(reservation, candidateId, role);
  if (firstReserve.ok) reservation = firstReserve.state;
  else refusals.push(refusalOf(firstReserve));
  const secondReserve = reserve(reservation, candidateId, role);
  stages.push(
    stage({
      id: 'holdout',
      step: 6,
      title: 'Two kinds of held-back data',
      verdict: firstReserve.ok ? 'reserved once' : 'refused',
      tone: firstReserve.ok ? 'ok' : 'bad',
      detail:
        'the reservation is taken by a lineage for a role and nothing releases it, so the campaign has one confirmation result it may cite as independent',
      facts: [
        ['role reserved', role],
        ['second ask', secondReserve.ok ? 'granted' : String(secondReserve.code).replaceAll('_', ' ')],
        ['release path', 'none exists in the layer'],
      ],
      refusals: secondReserve.ok ? [] : [refusalOf(secondReserve)],
    }),
  );

  /* 7. The gates: six questions in one order, stopping at the first no. */
  const evidence = declaredEvidence(candidateId, specificationDigest, settings.defect);
  const walk = walkGates(evidence);
  const stopped = walk.outcome !== 'passed';
  const stopReason = `the ladder stopped at ${walk.stoppedAt}, so this stage of the campaign never ran`;
  appendOne('execution_attempt_closed', { candidateId, outcome: walk.outcome });
  stages.push(
    stage({
      id: 'gates',
      step: 7,
      title: 'The pre-registered gate chain',
      verdict: walk.outcome,
      tone: walk.outcome === 'passed' ? 'ok' : 'warn',
      detail: stopped
        ? `stopped at ${walk.stoppedAt}; never asked: ${walk.neverAsked.join(', ') || 'none'}`
        : 'every gate passed, in order, on the declared evidence',
      facts: [
        ['gates asked', `${walk.records.length} of 6`],
        ['stopped at', walk.stoppedAt ?? 'nothing'],
        ['verdict recorded', `entry ${entries.length} of the ledger`],
      ],
      rows: walk.states.map((state) => [
        state.gate,
        state.asked ? state.verdict ?? '' : 'never asked',
      ]),
      refusals: walk.records
        .filter((record) => record.verdict !== 'passed')
        .map((record) => ({ code: record.verdict, condition: record.reason })),
    }),
  );

  /* 8. The admission test: what the sample can support, answered before the first run. */
  const admission = admissionTest({
    effectiveObservations: settings.effectiveObservations,
    reservedObservations: 0,
    holdoutFraction: settings.holdoutFraction,
    plannedConfigurations: settings.plannedConfigurations,
    trialsAlreadyConsumed: 0,
    targetSharpe: settings.targetSharpe,
    crossTrialDispersion: settings.crossTrialDispersion,
    significance: 0.95,
    dependence: 'independent',
  });
  stages.push(
    stage({
      id: 'admission',
      step: 8,
      title: 'The pre-campaign admission test',
      verdict: admission.outcome,
      tone: admission.outcome === 'admit' ? 'ok' : admission.outcome === 'reduce' ? 'warn' : 'bad',
      detail:
        admission.reason ??
        'the ceiling the relation implies fits the configuration count the campaign plans',
      facts: [
        ['usable observations', String(admission.usableObservations)],
        ['counted trials', String(admission.countedTrials)],
        [
          'ceiling at that count',
          admission.requiredObservations === null ? 'not estimable' : String(admission.requiredObservations),
        ],
        [
          'narrowed to',
          admission.reducedConfigurations === null ? 'nothing' : String(admission.reducedConfigurations),
        ],
      ],
    }),
  );

  /* 9. The search: children bred from the declared candidate, kept on a Pareto front. */
  const genes = genesOf(specification, policy);
  const paths = genes.map((gene) => gene.path);
  const edits = [];
  const operatorRefusals = [];
  for (const path of paths) {
    if (edits.length >= settings.population) break;
    const edit = mutate(specification, policy, { paths: [path], rng });
    if (edit.ok) edits.push(edit);
    else operatorRefusals.push(edit);
  }
  const bred = edits.map((edit, index) => ({
    candidateId: `child-${index + 1}`,
    family: edit.genome.identity?.family ?? 'unknown',
    key: equivalenceKey(edit.genome),
    changes: edit.changes,
    // The development results are DECLARED by the campaign: the search reads what a development run
    // declared rather than computing a performance figure itself.
    objectives: {
      return: Number((0.04 + 0.01 * Math.sin(index * 1.7)).toFixed(6)),
      turnover: Number((0.2 + 0.05 * Math.cos(index * 1.3)).toFixed(6)),
      drawdown: Number((0.1 + 0.02 * Math.sin(index * 0.9 + 1)).toFixed(6)),
    },
  }));
  const front = paretoFront(bred, { objectives: settings.objectives });
  const onFront = new Set(front.map((result) => result.candidateId));
  if (stopped) {
    stages.push(notReached('search', 9, 'Searching for strategies by breeding candidates', stopReason));
  } else {
    stages.push(
      stage({
        id: 'search',
        step: 9,
        title: 'Searching for strategies by breeding candidates',
        verdict: `${front.length} of ${bred.length} kept`,
        tone: front.length > 0 ? 'ok' : 'warn',
        detail:
          'a front is a set of options: it corrects for nothing, and the seed fixes which value was drawn for which gene',
        digest: digestOf(bred.map((child) => child.key)),
        facts: [
          ['genes the candidate has', String(paths.length)],
          ['children bred', String(bred.length)],
          ['genes the operators refused', String(operatorRefusals.length)],
          ['objectives kept', settings.objectives.join(', ')],
        ],
        rows: bred.map((child) => [
          child.candidateId,
          child.changes.map((change) => `${change.path}: ${change.before} -> ${change.after}`).join('; '),
          child.objectives.return,
          child.objectives.turnover,
          child.objectives.drawdown,
          onFront.has(child.candidateId) ? 'kept' : 'beaten',
        ]),
        refusals: operatorRefusals.map(refusalOf),
      }),
    );
  }

  /* 10. The agents: four recorded responses, classified by the layer's own order of checks. */
  const language = {
    validate: (payload) => {
      const result = validateSpecification(payload, policy, { registry: REGISTRY });
      return {
        accepted: result.ok === true,
        errors: (result.refusals ?? []).map((item) => ({
          code: item.code,
          path: item.path,
          message: item.reason ?? item.message ?? item.code,
        })),
      };
    },
  };
  const replies = RECORDED_REPLIES.map((node) => {
    const reply = valueOf(node);
    return {
      record: reply,
      result: classifyReply(reply.response, { role: 'strategy_engineer', language }),
    };
  });
  stages.push(
    stage({
      id: 'agents',
      step: 10,
      title: 'Agents that propose candidates',
      verdict: `${replies.length} recorded responses classified`,
      tone: 'ok',
      detail:
        'no model is called: a live call cannot be reproduced, which is why a campaign acts on a recorded response and why the classification is the part that can be checked',
      rows: replies.map((item) => [
        String(item.record.response).slice(0, 60),
        item.record.howRead,
        item.result.outcome ?? item.result.code ?? '',
      ]),
    }),
  );

  /* 11. The shadow: the frozen candidate watched against a feed. Not reached when the ladder stopped,
   *     because a candidate the gates turned away is never promoted to a shadow period. */
  if (stopped) {
    stages.push(notReached('shadow', 11, 'The shadow period', stopReason));
  } else {
    const NS = 1000000000;
    const plan = {
      candidate_id: candidateId,
      specification_digest: specificationDigest,
      period: '2024-01-01 to 2024-02-01',
      minimum_observations: 3,
    };
    const signals = [1, 2, 3, 4, 5].map((day) => day * 86400 * NS);
    const feed = signals.map((signalAt, index) => ({
      signal_at: signalAt,
      intended_entry: 'INST.1',
      eligible_fill_at: signalAt + NS,
      fill_price: 100 + index,
      modelled_cost: 0.01,
      observed_cost: 0.011,
      non_fill_reason: '',
    }));
    let shadowState = watch({ plan, expectedSignalTimes: signals });
    let shadowRefusal = null;
    for (const observation of feed) {
      const step = observe(shadowState, observation);
      if (step.ok === false) {
        shadowRefusal = step;
        break;
      }
      shadowState = step.state;
    }
    const report = shadowRefusal ? null : shadowReport({ ...shadowState, plan });
    const reportRefused = report !== null && report.ok === false;
    if (reportRefused) refusals.push(refusalOf(report));
    stages.push(
      stage({
        id: 'shadow',
        step: 11,
        title: 'The shadow period',
        verdict: shadowRefusal || reportRefused ? 'refused' : 'reported',
        tone: shadowRefusal || reportRefused ? 'bad' : 'ok',
        detail: shadowRefusal
          ? shadowRefusal.reason
          : reportRefused
            ? report.reason
            : 'the run and the shadow period declare the same contract and the same pair of instants, compared by value rather than by name; the feed here is declared by the campaign, not bound to a live adapter',
        facts: [
          ['observations', String(feed.length)],
          ['declared minimum', String(plan.minimum_observations)],
          ['contract digest', contractDigest(TEMPORAL_CONTRACT).slice(0, 19)],
        ],
        refusals: shadowRefusal
          ? [refusalOf(shadowRefusal)]
          : reportRefused
            ? [refusalOf(report)]
            : [],
      }),
    );
  }

  /* 12. The audit: the ledger read back, and what the reading refuses to assert. */
  const reading = derivedView(entries, {
    campaignId: settings.label,
    familyId: specification.identity.family,
    policyDigest: frozen.digest,
    recordedPolicyDigest: frozen.digest,
    engineVersion: 'nautilus_trader 2.0.0rc6, built with uncommitted crate sources present',
    gateReport: walk.records.map((record) => ({ gate: record.gate, reason: record.reason })),
  });
  const readOutcomes = outcome(reading);
  stages.push(
    stage({
      id: 'audit',
      step: 12,
      title: 'Auditing a campaign from the ledger alone',
      verdict: readOutcomes.map((item) => item.outcome).join('; '),
      tone: 'ok',
      detail:
        'the reading folds the chain itself rather than trusting a counter, and it asserts only what the chain holds',
      digest: digestOf(reading),
      facts: [
        ['candidates in the record', String(readOutcomes.length)],
        [
          'policy compared',
          reading.policyMatches === null
            ? 'nothing declared'
            : reading.policyMatches
              ? 'the declared digest is the recorded one'
              : 'the declared digest differs',
        ],
        ['entries read', String(entries.length)],
      ],
      rows: readOutcomes.map((item) => [item.candidateId || 'the campaign', item.outcome]),
    }),
  );

  /* 13. The boundary: what a worker is given, and what is absent rather than switched off. */
  const clean = checkEnvironment({ PATH: '/usr/bin', HOME: '/home/worker' }, []);
  const planted = checkEnvironment(
    { PATH: '/usr/bin', HOME: '/home/worker', EODHD_API_KEY: 'set-on-this-machine' },
    ['EODHD_API_KEY'],
  );
  const chain = verifyChain(entries, null);
  stages.push(
    stage({
      id: 'boundary',
      step: 13,
      title: 'The boundary',
      verdict: planted.clean ? 'no credential found' : 'a credential was found',
      tone: planted.clean ? 'ok' : 'warn',
      detail:
        'the layer holds no live execution adapter and no credential, so nothing in it can authenticate an order; a market-data adapter is a different kind of thing, which the shadow period takes as a declared feed rather than binding to one, and the allowlist that would make the worker capabilities explicit is procedural, because the layout configuration module is unbuilt',
      facts: [
        ['credential names in the canonical list', String(credentials().length)],
        ['clean environment', clean.clean ? 'clean' : 'not clean'],
        ['planted name reported', planted.ambient.join(', ') || 'none'],
        ['chain verified', chain.ok ? 'yes' : `no, first break at ${chain.brokenAt}`],
      ],
      refusals: planted.ambient.map((name) => ({
        code: 'ambient_credential',
        condition: `${name} is set in the environment the boundary defines, so the boundary reports it rather than running`,
      })),
    }),
  );

  return {
    ok: refusals.length === 0,
    options: settings,
    stages,
    stoppedAt: walk.stoppedAt,
    refusals,
    ledger: {
      entries: entries.length,
      head: entries.length > 0 ? entries.at(-1).chain : null,
      verified: chain.ok,
    },
    audit: { reading, readOutcomes },
    digests: { policy: frozen.digest, specification: specificationDigest, candidate: candidateId },
    timing: TIMING_CAVEAT,
  };
}
