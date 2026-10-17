/**
 * A pure model of the holdout roles and the confirmation reservation.
 *
 * It models `python/nautilus_trader/discovery/custody.py` and `ledger.py`: the three data roles and
 * which of them may be read as validation, the confirmation reservation that a campaign takes once
 * and never releases, and the per-lineage downgrade rule from `evolve.py` - a partition that
 * informed a lineage is development for that lineage however it was labelled on paper. Deterministic:
 * identical inputs produce identical outputs, and a refusal is a value rather than an exception.
 */

import { valueOf } from '../data/source.js';
import {
  DOWNGRADE_RULE,
  REFUSAL_CODES as REFUSAL_CODE_FACTS,
  ROLES as ROLE_FACTS,
  admissibleAsValidation,
  roles,
} from '../data/holdout.js';

/** The three data roles, in declared order, with their reader unwrapped for display. */
export function ROLES() {
  return roles();
}

/** The closed custody refusal vocabulary, as the declared code strings. */
export const REFUSAL_CODES = REFUSAL_CODE_FACTS.map((entry) => valueOf(entry.code));

function refusal(code, reason) {
  return { ok: false, code, reason };
}

function informedReason(partition, by) {
  return valueOf(DOWNGRADE_RULE.informed).reason
    .replace('{by}', by)
    .replace('{partition}', partition);
}

/** A campaign that holds no confirmation reservation yet. */
export function initialReservation() {
  return { reserved: false, lineage: null, role: null, consumed: false };
}

/**
 * Take the campaign's one confirmation reservation for a lineage and a role.
 *
 * A campaign holds at most one reservation. A repeat for the same lineage is refused
 * `reservation_consumed`: the family's reservation is spent and nothing releases it. A call that
 * names a different lineage while one is held is refused `second_candidate`: one campaign produces
 * one confirmation result it may cite as independent. A role outside the declared three is refused
 * `role_mismatch`.
 */
export function reserve(state, lineage, role) {
  if (!ROLE_FACTS.map(valueOf).some((entry) => entry.name === role)) {
    return refusal('role_mismatch', `the role ${role} is not one of the declared data roles`);
  }
  if (state === null || state === undefined || state.reserved !== true) {
    return { ok: true, state: { reserved: true, lineage, role, consumed: false } };
  }
  if (state.lineage === lineage) {
    return refusal('reservation_consumed', "the family's confirmation reservation is spent and nothing releases it");
  }
  return refusal(
    'second_candidate',
    `the reservation is for ${state.lineage} and the campaign confirms one candidate: one campaign produces one confirmation result it may cite as independent`,
  );
}

/**
 * Apply the per-lineage downgrade: every partition this lineage consumed becomes development for it.
 *
 * `lineages` is an array of assignment rows `{partition, lineage, role, by}`. A partition that
 * informed a lineage is development for that lineage with the reason the source records. A lineage
 * that consumed nothing has nothing to downgrade, and that is refused `role_mismatch` with the
 * source's own reason for the case.
 */
export function downgrade(lineages, lineage) {
  const matching = lineages.filter((row) => row.lineage === lineage);
  if (matching.length === 0) {
    return refusal('role_mismatch', valueOf(DOWNGRADE_RULE.asValidation).reason);
  }
  return {
    ok: true,
    downgraded: matching.map((row) => row.partition),
    lineages: lineages.map((row) =>
      row.lineage === lineage
        ? { ...row, role: 'development', reason: informedReason(row.partition, row.by ?? lineage) }
        : row,
    ),
  };
}

/** Whether a role may be read as validation evidence, which only locked validation may. */
export function ADMISSIBLE_AS_VALIDATION(role) {
  return admissibleAsValidation(role);
}
