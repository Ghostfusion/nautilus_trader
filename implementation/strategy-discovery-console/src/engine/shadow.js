/**
 * The shadow period: one frozen candidate watched against a feed, and what the watch reports.
 *
 * Models `python/nautilus_trader/discovery/shadow.py`. The first job is the temporal contract: both
 * halves write down the same four declarations, and they are compared by value rather than by name,
 * so a run declaring a different `execution_eligibility` is a different contract however alike the
 * timestamps look. The second is the record: each watched signal becomes one observation, seven
 * fields wide, and each observation the feed reports that the replay did not expect becomes a
 * discrepancy with a magnitude.
 *
 * Nothing here decides whether the candidate is good. A report says whether signals, timing,
 * delivery and simulated execution behaved as the replay expected on live inputs; meeting the
 * declared observation count is a precondition for reading it and evidence of nothing. No order is
 * sent, no client is held, and no field here is a performance statistic.
 *
 * Costs are held as numbers rather than as decimals, so a total is rounded to ten places: the point
 * is that the report's own arithmetic reads as the page's worked example does.
 */

import { digestOf } from './digest.js';
import { declaredSetsByName } from '../data/genome.js';
import {
  MESSAGES,
  SHADOW,
  contractFields,
  refusalCodes,
} from '../data/shadow.js';

/** The three refusals the monitor reports instead of a report. */
export const REFUSAL_CODES = refusalCodes();

/**
 * The console's declared contract: one value from each of the policy's own declared sets.
 *
 * In the repository the contract is read off the candidate's data block, so its four values come
 * from the compiled specification; here they come from the declaration table the specification is
 * read against, which is the same path a candidate's values resolve through.
 */
export const TEMPORAL_CONTRACT = (() => {
  const sets = declaredSetsByName();
  return {
    signal_frequency: sets.signal_frequencies[0],
    decision_time: sets.decision_times[0],
    execution_eligibility: sets.execution_eligibilities[0],
    corporate_action_treatment: sets.corporate_action_treatments[0],
  };
})();

/** The label every shadow result carries, because a simulated fill is never an observed one. */
export const LABEL = SHADOW.label.value;

/** Compose a transcribed message, substituting the names it carries. */
function message(key, values = {}) {
  const entry = MESSAGES[key].value;
  const text = typeof entry === 'string' ? entry : entry.message;
  return text.replace(/\{(\w+)\}/g, (match, name) =>
    name in values ? String(values[name]) : match,
  );
}

/** Refuse with the code and message the monitor reports. */
function refuse(key, values = {}) {
  const entry = MESSAGES[key].value;
  return { ok: false, code: entry.code, reason: message(key, values) };
}

/** The declared fields as named pairs, in the order the contract states them. */
export function declaredFields(contract) {
  return contractFields().map((name) => [name, contract[name]]);
}

/** The digest of a contract: its declared fields, canonicalised, which is what names it. */
export function contractDigest(contract) {
  return digestOf(Object.fromEntries(declaredFields(contract)));
}

/** The pair the parity rule compares: the decision instant and the eligible fill instant. */
export function readingPair(reading) {
  return [reading.decision_at, reading.eligible_fill_at];
}

/**
 * Compare a replay's reading with a shadow run's, field by field and instant by instant.
 *
 * The two halves are reported separately because they fail differently: a differing declaration
 * means the shadow run is not shadowing the backtest at all, while a differing instant means the
 * feed delivered the decision later or earlier than the replay assumed. Naming the fields that
 * differ is what makes the first diagnosable, and the two go on to make `holds` true only together.
 */
export function parityCheck(replayReading, shadowReading) {
  const replay = Object.fromEntries(declaredFields(replayReading.contract));
  const shadow = Object.fromEntries(declaredFields(shadowReading.contract));
  const differing = declaredFields(replayReading.contract)
    .map(([name]) => name)
    .filter((name) => !(name in shadow) || shadow[name] !== replay[name]);
  differing.push(...Object.keys(shadow).filter((name) => !(name in replay)));

  const sameFields = differing.length === 0;
  const samePair = readingPair(replayReading).join(',') === readingPair(shadowReading).join(',');
  const holds = sameFields && samePair;

  const parts = [];
  if (!sameFields) parts.push(message('parityFields', { fields: differing.join(', ') }));
  if (!samePair) {
    parts.push(
      message('parityPair', {
        replay: `(${readingPair(replayReading).join(', ')})`,
        shadow: `(${readingPair(shadowReading).join(', ')})`,
      }),
    );
  }
  return {
    same_fields: sameFields,
    same_pair: samePair,
    holds,
    differing_fields: differing,
    detail: holds ? message('parityHolds') : parts.join('; '),
  };
}

/** Whether an observation simulated a fill, which is the presence of a fill price. */
export function filled(observation) {
  return observation.fill_price !== null && observation.fill_price !== undefined;
}

/** The two rules an observation must satisfy before it can be recorded at all. */
function checkObservation(observation) {
  if (observation.eligible_fill_at < observation.signal_at) {
    return refuse('eligibleBeforeSignal', {
      signal: observation.signal_at,
      fill: observation.eligible_fill_at,
    });
  }
  if (observation.intended_entry && !filled(observation) && !observation.non_fill_reason) {
    return refuse('nonFillWithoutReason', { signal: observation.signal_at });
  }
  return null;
}

/** One observation record, carrying all seven fields the record is made of. */
export function observe(previous, observation) {
  if (previous === undefined || previous === null) {
    throw new Error('a shadow watch starts from a watch state, and none was given');
  }
  const malformed = checkObservation(observation);
  if (malformed) return malformed;
  return {
    ok: true,
    state: {
      ...previous,
      observations: [...previous.observations, { ...observation }],
    },
  };
}

/** The state a watch runs over: the declaration, the replay's instants, and the records so far. */
export function watch({ plan, contract = TEMPORAL_CONTRACT, expectedSignalTimes = [] }) {
  return {
    plan,
    contract,
    expected_signal_times: [...expectedSignalTimes],
    observations: [],
  };
}

/** The expected instant nearest a delivered one, by distance and then by value. */
function nearest(instant, candidates) {
  if (candidates.length === 0) return instant;
  return candidates.reduce((best, candidate) => {
    const distance = Math.abs(candidate - instant);
    const bestDistance = Math.abs(best - instant);
    if (distance < bestDistance) return candidate;
    if (distance === bestDistance && candidate < best) return candidate;
    return best;
  });
}

/** A total of the cost values present, rounded because the console holds costs as numbers. */
function totalCost(values) {
  const sum = values.reduce((running, value) => running + value, 0);
  return Number(sum.toFixed(10));
}

/**
 * Report what a shadow period observed against what the replay expected.
 *
 * The refusals come before the report, in the repository's order: a plan that declared no
 * observation count, an observation eligible before its own signal, and an intended entry that did
 * not fill without a reason are each refused by name rather than reported, because a report that
 * cannot say why is not diagnosable.
 */
export function shadowReport(input) {
  const { plan, contract } = input;
  const expected = input.expected_signal_times ?? input.expectedSignalTimes ?? [];
  const observations = input.observations ?? [];

  if (plan.minimum_observations < 1) {
    return refuse('minimumNotDeclared', { n: plan.minimum_observations });
  }
  for (const observation of observations) {
    const malformed = checkObservation(observation);
    if (malformed) return malformed;
  }

  const delivered = new Set(observations.map((observation) => observation.signal_at));
  const counts = new Map();
  for (const observation of observations) {
    counts.set(observation.signal_at, (counts.get(observation.signal_at) ?? 0) + 1);
  }
  const expectedSet = new Set(expected);

  const signal_discrepancies = observations
    .filter(
      (observation) =>
        !expectedSet.has(observation.signal_at) || counts.get(observation.signal_at) !== 1,
    )
    .map((observation) => {
      const closest = nearest(observation.signal_at, expected);
      return {
        expected_at: closest,
        observed_at: observation.signal_at,
        magnitude: Math.abs(observation.signal_at - closest),
      };
    });

  const non_fills = observations
    .filter((observation) => observation.intended_entry && !filled(observation))
    .map((observation) => ({
      signal_at: observation.signal_at,
      intended_entry: observation.intended_entry,
      reason: observation.non_fill_reason,
    }));

  const modelled = totalCost(
    observations
      .map((observation) => observation.modelled_cost)
      .filter((value) => value !== null && value !== undefined),
  );
  const observed = totalCost(
    observations
      .map((observation) => observation.observed_cost)
      .filter((value) => value !== null && value !== undefined),
  );

  const reached = observations.length;
  const missing_observations = expected.filter((instant) => !delivered.has(instant)).length;
  const cost_difference = Number((observed - modelled).toFixed(10));
  const readable = reached >= plan.minimum_observations;
  const diverged =
    signal_discrepancies.length > 0 || non_fills.length > 0 || missing_observations > 0;

  const findings = [];
  if (missing_observations) {
    findings.push(message('missingFinding', { count: missing_observations }));
  }
  for (const discrepancy of signal_discrepancies) {
    findings.push(
      message('discrepancyFinding', {
        magnitude: discrepancy.magnitude,
        expected: discrepancy.expected_at,
      }),
    );
  }
  for (const non_fill of non_fills) {
    findings.push(
      message('nonFillFinding', { signal: non_fill.signal_at, reason: non_fill.reason }),
    );
  }
  if (cost_difference) {
    findings.push(message('costFinding', { difference: cost_difference }));
  }

  return {
    ok: true,
    schema: SHADOW.schema.value,
    label: LABEL,
    plan,
    plan_digest: digestOf(plan),
    contract,
    contract_digest: contractDigest(contract),
    eligible_decisions: expected.length,
    observed_decisions: observations.length,
    minimum_observations: plan.minimum_observations,
    reached,
    readable,
    unreadable_reason: readable
      ? ''
      : message('unreadableReason', {
          reached,
          minimum: plan.minimum_observations,
        }),
    missing_observations,
    signal_discrepancies,
    non_fills,
    modelled_cost: modelled,
    observed_cost: observed,
    cost_difference,
    diverged,
    findings,
    observations: observations.map((observation) => ({ ...observation, filled: filled(observation) })),
  };
}
