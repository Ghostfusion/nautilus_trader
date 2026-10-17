/**
 * A pure model of the one executor's declaration and allocation.
 *
 * It models `python/nautilus_trader/discovery/strategy.py`: the interpreter's decision vocabulary,
 * the record kinds a run writes, and the nine allocation decisions in the fixed order the design
 * gives them - the weighting, the position cap, the sector cap, the gross exposure cap and the cash
 * policy's treatment of the residual. It computes an allocation from a ranking and a set of declared
 * caps, so a reader can check the worked example number by number. It does not compute prices, size
 * orders or place them: the allocation's weights are the candidate's own record, and the platform
 * resolves quantities from the strengths. Deterministic: identical inputs produce identical outputs.
 */

import { valueOf } from '../data/source.js';
import {
  ALLOCATION_DECISIONS,
  CLOSED_SETS,
  RECORD_KINDS,
  REFUSALS,
  TIMING_CAVEAT as TIMING_CAVEAT_FACT,
} from '../data/executor.js';

/** The bridge stamps an instruction with the instant it becomes eligible and submits it without
 *  waiting, so a deferred fill can land earlier than the stamp; criterion 4 fails for this reason. */
export const SUBMISSION_DEFERS = false;

/** The timing caveat, verbatim, kept beside the model so a reader sees the limit of the bridge. */
export const TIMING_CAVEAT = valueOf(TIMING_CAVEAT_FACT);

const UNIT = 1;

/** The nine allocation decisions, in their fixed order. A view shows them as the model's steps. */
export function DECISIONS() {
  return ALLOCATION_DECISIONS.map(valueOf);
}

/** The record kinds one run writes, each with its fields in the exact order the source declares. */
export function recordKinds() {
  return RECORD_KINDS.map(valueOf);
}

/** The refusal codes the interpreter's own errors map to, as the model's short handles. */
export const REFUSAL_CODES = REFUSALS.map((node) => valueOf(node).code);

function members(name) {
  const set = CLOSED_SETS.map(valueOf).find((entry) => entry.name === name);
  return set ? set.members : [];
}

function refusal(code, reason) {
  return { ok: false, code, reason };
}

function refusalReason(code, params) {
  const entry = REFUSALS.map(valueOf).find((item) => item.code === code);
  const template = entry ? entry.message : code;
  return template.replace(/\{([a-z_ ]+)\}/g, (match, key) => {
    const value = params[key];
    return value === undefined ? match : String(value);
  });
}

/** The investable ceiling: one unit, or the declared gross exposure cap when it is smaller. */
function investableCeiling(grossCap) {
  if (grossCap === null || grossCap === undefined) return UNIT;
  return Math.min(UNIT, grossCap);
}

/** Truncate the selection to the portfolio size, keeping the order and dropping a repeat. */
function selectedOrder(selected, positionCount) {
  const order = [];
  for (const instrument of selected) {
    if (order.length >= Math.max(positionCount, 0)) break;
    if (!order.includes(instrument)) order.push(instrument);
  }
  return order;
}

/** The declared weighting's shares, and the weights they normalise to. */
function initialWeights(order, weighting) {
  if (order.length === 0) return { weights: {}, shares: {} };
  const shares = {};
  if (weighting === 'equal') {
    for (const instrument of order) shares[instrument] = UNIT;
  } else {
    order.forEach((instrument, index) => {
      shares[instrument] = order.length - index;
    });
  }
  const total = Object.values(shares).reduce((sum, share) => sum + share, 0);
  const weights = {};
  for (const instrument of order) weights[instrument] = shares[instrument] / total;
  return { weights, shares };
}

/** Clamp each weight to the per-position cap, redistributing nothing. */
function applyPositionCap(weights, position) {
  const out = {};
  for (const [instrument, weight] of Object.entries(weights)) out[instrument] = Math.min(weight, position);
  return out;
}

/** Scale a sector down proportionally when its total exceeds the sector cap. */
function applySectorCap(weights, sectorCap, sectorOf) {
  if (sectorCap === null || sectorCap === undefined) return weights;
  const sectorOfName = (instrument) => sectorOf[instrument] ?? instrument;
  const totals = {};
  for (const [instrument, weight] of Object.entries(weights)) {
    const sector = sectorOfName(instrument);
    totals[sector] = (totals[sector] ?? 0) + weight;
  }
  const out = {};
  for (const [instrument, weight] of Object.entries(weights)) {
    const total = totals[sectorOfName(instrument)];
    out[instrument] = total <= sectorCap ? weight : (weight * sectorCap) / total;
  }
  return out;
}

/** Scale every weight down proportionally when the total exceeds the investable ceiling. */
function applyGrossCap(weights, ceiling) {
  const total = Object.values(weights).reduce((sum, weight) => sum + weight, 0);
  if (total <= ceiling || total <= 0) return weights;
  const scale = ceiling / total;
  const out = {};
  for (const [instrument, weight] of Object.entries(weights)) out[instrument] = weight * scale;
  return out;
}

/** Each weight's remaining headroom under the position cap and its sector's remaining room. */
function headroom(weights, position, sectorCap, sectorOf) {
  const room = {};
  for (const [instrument, weight] of Object.entries(weights)) room[instrument] = Math.max(position - weight, 0);
  if (sectorCap === null || sectorCap === undefined) return room;
  const sectorOfName = (instrument) => sectorOf[instrument] ?? instrument;
  const totals = {};
  for (const [instrument, weight] of Object.entries(weights)) {
    const sector = sectorOfName(instrument);
    totals[sector] = (totals[sector] ?? 0) + weight;
  }
  for (const instrument of Object.keys(room)) {
    const remaining = Math.max(sectorCap - totals[sectorOfName(instrument)], 0);
    room[instrument] = Math.min(room[instrument], remaining);
  }
  return room;
}

/** Offer the residual in the declared share proportions, capped by each name's headroom. The loop
 *  is bounded by the number of instruments, so no instrument is pushed above a cap. */
function reallocate(weights, shares, ceiling, position, sectorCap, sectorOf) {
  for (let pass = 0; pass < Object.keys(weights).length; pass += 1) {
    const invested = Object.values(weights).reduce((sum, weight) => sum + weight, 0);
    const residual = ceiling - invested;
    if (residual <= 0) break;
    const room = headroom(weights, position, sectorCap, sectorOf);
    const candidates = Object.keys(room).filter((instrument) => room[instrument] > 0);
    const offered = candidates.reduce((sum, instrument) => sum + (shares[instrument] ?? 0), 0);
    if (candidates.length === 0 || offered <= 0) break;
    let added = 0;
    for (const instrument of candidates) {
      const share = (residual * (shares[instrument] ?? 0)) / offered;
      const gain = Math.min(share, room[instrument]);
      weights[instrument] += gain;
      added += gain;
    }
    if (added <= 0) break;
  }
  return weights;
}

/** The candidate's weight as the signal strength the pipeline scales risk by. */
function strength(weight, cap) {
  if (cap <= 0) return 0;
  return Math.min(Math.max(weight / cap, 0), 1);
}

/**
 * Compute one allocation from a ranked selection and the declared caps.
 *
 * `selection` is the ranked instrument identifiers. `options` carries the declared caps and policy:
 * `weighting` (equal or rank_proportional), `positionCount`, `maxPositionWeight`, `cashPolicy`
 * (leave_idle or reallocate), `maxGrossExposure`, `maxSectorWeight`, `sectorOf`, `held`,
 * `ineligible` and `flattenIneligible`. Returns the per-instrument rows, the emitted target weights
 * and the invested and idle totals. A value outside a closed vocabulary is a refusal value, not an
 * exception.
 */
export function allocate(selection, options = {}) {
  const weighting = options.weighting ?? 'rank_proportional';
  const cashPolicy = options.cashPolicy ?? 'leave_idle';
  const positionCount = options.positionCount ?? selection.length;
  const position = options.maxPositionWeight ?? UNIT;
  const grossCap = options.maxGrossExposure ?? null;
  const sectorCap = options.maxSectorWeight ?? null;
  const sectorOf = options.sectorOf ?? {};
  const held = options.held ?? [];
  const ineligible = options.ineligible ?? [];
  const flattenIneligible = options.flattenIneligible === true;

  if (!members('weightings').includes(weighting)) {
    return refusal('weighting_not_declared', refusalReason('weighting_not_declared', {
      weightings: members('weightings').join(', '),
      weighting,
    }));
  }
  if (!members('cash policies').includes(cashPolicy)) {
    return refusal('cash_policy_not_declared', refusalReason('cash_policy_not_declared', {
      'cash policies': members('cash policies').join(', '),
      cash_policy: cashPolicy,
    }));
  }

  const ceiling = investableCeiling(grossCap);
  const order = selectedOrder(selection, positionCount);
  const { weights: raw, shares } = initialWeights(order, weighting);

  let weights = applyPositionCap(raw, position);
  if (sectorCap !== null) weights = applySectorCap(weights, sectorCap, sectorOf);
  weights = applyGrossCap(weights, ceiling);
  if (cashPolicy === 'reallocate') weights = reallocate(weights, shares, ceiling, position, sectorCap, sectorOf);

  const rows = order.map((instrument, index) => ({
    instrument,
    rank: index,
    share: shares[instrument],
    rawWeight: raw[instrument],
    cappedWeight: weights[instrument],
    instructionStrength: strength(weights[instrument], position),
  }));

  const invested = Object.values(weights).reduce((sum, weight) => sum + weight, 0);
  const idle = ceiling - invested;

  const flat = new Set();
  if (flattenIneligible) for (const instrument of ineligible) if (held.includes(instrument)) flat.add(instrument);
  for (const instrument of held) {
    if (!(instrument in weights) && !ineligible.includes(instrument)) flat.add(instrument);
  }

  const targets = [];
  for (const [instrument, weight] of Object.entries(weights)) {
    if (weight > 0 && !ineligible.includes(instrument)) targets.push({ instrument, weight });
  }
  for (const instrument of flat) if (!targets.some((target) => target.instrument === instrument)) targets.push({ instrument, weight: 0 });
  targets.sort((a, b) => (a.instrument < b.instrument ? -1 : a.instrument > b.instrument ? 1 : 0));

  return {
    ok: true,
    order,
    rows,
    targets,
    invested,
    idle,
    ceiling,
    position,
    weighting,
    cashPolicy,
  };
}
