/**
 * The transcribed holdout roles, reservation states, transitions and refusals.
 *
 * Two kinds of data are held back: locked validation, which a frozen candidate is judged on, and
 * the final confirmation set, which one campaign spends once and never releases. The rules live in
 * `python/nautilus_trader/discovery/custody.py` and `ledger.py`, with the per-lineage downgrade in
 * `evolve.py` and the gate that reads the role in `gates.py`. Every constant, state, transition,
 * refusal code and message is wrapped in `fact(...)` with the file, line and verbatim text it was
 * read from, so tools/check-facts.mjs can fail when a source moves on.
 */

import { fact, valueOf } from './source.js';

const CUSTODY = 'python/nautilus_trader/discovery/custody.py';
const LEDGER = 'python/nautilus_trader/discovery/ledger.py';
const EVOLVE = 'python/nautilus_trader/discovery/evolve.py';
const GATES = 'python/nautilus_trader/discovery/gates.py';
const DESIGN = 'docs/design/strategy_discovery_design.md';

/** The three data roles, each with the surface that may read it. Only `locked_validation` is
 *  admissible as validation evidence, which is what makes the classification mean something. */
export const ROLES = [
  fact(
    {
      name: 'development',
      label: 'Development',
      admissibleAsValidation: false,
      readBy: fact('The search, the agents, and the gate service', DESIGN, 1154, '| Development        | The search, the agents, and the gate service'),
    },
    LEDGER, 494, 'DEVELOPMENT = "development"',
  ),
  fact(
    {
      name: 'locked_validation',
      label: 'Locked validation',
      admissibleAsValidation: true,
      readBy: fact('The validation service, after the candidate or candidate set is frozen', DESIGN, 1155, '| Locked validation  | The validation service, after the candidate or candidate set is frozen'),
    },
    LEDGER, 495, 'LOCKED_VALIDATION = "locked_validation"',
  ),
  fact(
    {
      name: 'final_confirmation',
      label: 'Final confirmation',
      admissibleAsValidation: false,
      readBy: fact('The validator entry point, once per campaign, for one frozen candidate', DESIGN, 1156, '| Final confirmation | The validator entry point, once per campaign, for one frozen candidate'),
    },
    LEDGER, 496, 'FINAL_CONFIRMATION = "final_confirmation"',
  ),
];

/** The order the three roles are declared in. */
export const ROLE_ORDER = ['development', 'locked_validation', 'final_confirmation'];

/** Which role may be read as validation evidence. */
export const ADMISSIBLE_AS_VALIDATION = fact(
  { development: false, locked_validation: true, final_confirmation: false },
  LEDGER, 503,
  'return self is DataRole.LOCKED_VALIDATION',
);

/** The six states of the confirmation state machine of design Section 11.3. */
export const RESERVATION_STATES = [
  fact({ name: 'searching', label: 'Searching' }, CUSTODY, 61, 'SEARCHING = "searching"'),
  fact({ name: 'frozen', label: 'Frozen' }, CUSTODY, 62, 'FROZEN = "frozen"'),
  fact({ name: 'confirming', label: 'Confirming' }, CUSTODY, 63, 'CONFIRMING = "confirming"'),
  fact({ name: 'confirmed', label: 'Confirmed' }, CUSTODY, 64, 'CONFIRMED = "confirmed"'),
  fact({ name: 'withdrawn', label: 'Withdrawn' }, CUSTODY, 65, 'WITHDRAWN = "withdrawn"'),
  fact({ name: 'exploratory-reuse', label: 'Exploratory reuse' }, CUSTODY, 66, 'EXPLORATORY_REUSE = "exploratory-reuse"'),
];

/** The states in declared order. */
export const STATE_ORDER = ['searching', 'frozen', 'confirming', 'confirmed', 'withdrawn', 'exploratory-reuse'];

/** The transition table. The reservation is the one quantity that never moves back: it is taken at
 *  a freeze, spent on the one read that names the frozen candidate, and untouched forever after. */
export const TRANSITIONS = [
  fact({ id: 'T1', from: 'searching', operation: 'freeze', guard: 'no inherited reservation and the reservation is not consumed', to: 'frozen', reservation: 'taken' }, CUSTODY, 532, 'state=CustodyState.FROZEN,'),
  fact({ id: 'T2', from: 'frozen', operation: 'read_holdout for the frozen candidate, unlabelled', guard: 'the reservation is unconsumed', to: 'confirming', reservation: 'spent' }, CUSTODY, 663, 'if candidate_id == self._frozen_candidate:'),
  fact({ id: 'T3', from: 'frozen, confirming, exploratory-reuse or confirmed', operation: 'read_holdout for another candidate, labelled exploratory', guard: 'a labelled read consumes nothing', to: 'exploratory-reuse', reservation: 'unchanged (spent)' }, CUSTODY, 881, 'return CustodyRefusalCode.SECOND_CANDIDATE'),
  fact({ id: 'T4', from: 'any non-searching, non-withdrawn state', operation: 'read_holdout for another candidate, unlabelled', guard: 'refused', to: 'unchanged', reservation: 'unchanged' }, CUSTODY, 881, 'return CustodyRefusalCode.SECOND_CANDIDATE'),
  fact({ id: 'T5', from: 'any', operation: 'read_holdout with summary_statistics', guard: 'refused before the reservation is examined', to: 'unchanged', reservation: 'unchanged' }, CUSTODY, 629, 'if summary_statistics:'),
  fact({ id: 'T6', from: 'confirming or exploratory-reuse', operation: 'confirm with the correct candidate, matching policy and a spent reservation', guard: 'permitted', to: 'confirmed', reservation: 'spent' }, CUSTODY, 754, 'state=CustodyState.CONFIRMED,'),
  fact({ id: 'T7', from: 'any', operation: 'withdraw', guard: 'permitted; the verdict is cleared', to: 'withdrawn', reservation: 'spent' }, CUSTODY, 785, 'state=CustodyState.WITHDRAWN,'),
  fact({ id: 'T8', from: 'searching', operation: 'freeze when the state is not searching, a reservation is inherited or the reservation is spent', guard: 'refused already_frozen or reservation_consumed', to: 'searching (unchanged)', reservation: 'unchanged' }, CUSTODY, 489, 'if self._state is not CustodyState.SEARCHING:'),
  fact({ id: 'T9', from: 'withdrawn', operation: 'read_holdout', guard: 'refused not_frozen', to: 'unchanged', reservation: 'spent' }, CUSTODY, 878, 'return CustodyRefusalCode.NOT_FROZEN'),
  fact({ id: 'T10', from: 'confirmed', operation: 'citation', guard: 'permitted only while confirmed', to: 'unchanged', reservation: 'spent' }, CUSTODY, 828, 'if self._state is not CustodyState.CONFIRMED:'),
];

/** The closed custody refusal vocabulary, each code with the message it renders and the condition
 *  that produces it. Two codes have no raising path today; the model shows them as vocabulary
 *  members rather than hiding them. */
export const REFUSAL_CODES = [
  {
    code: fact('not_frozen', CUSTODY, 74, 'NOT_FROZEN = "not_frozen"'),
    message: fact('the campaign is {state} and no candidate is frozen for confirmation, so the confirmation set may not be read', CUSTODY, 898, 'f"the campaign is {self._state.value} and no candidate is frozen for " f"confirmation, so the confirmation set may not be read"'),
    condition: 'read_holdout when no candidate is frozen, or the state is searching or withdrawn',
  },
  {
    code: fact('already_frozen', CUSTODY, 75, 'ALREADY_FROZEN = "already_frozen"'),
    message: fact('the campaign is {state} and freezes one candidate once', CUSTODY, 492, 'f"the campaign is {self._state.value} and freezes one candidate once"'),
    condition: 'freeze when the state is not searching',
  },
  {
    code: fact('frozen_candidate_mutated', CUSTODY, 76, 'FROZEN_CANDIDATE_MUTATED = "frozen_candidate_mutated"'),
    message: fact('the candidate {candidate_id} is frozen under {frozen_policy} and may not be mutated (mutation to {specification_digest})', CUSTODY, 563, 'f"the candidate {candidate_id} is frozen under {self._frozen_policy} and may not be " f"mutated (mutation to {specification_digest})"'),
    condition: 'guard_mutation for the frozen candidate after the campaign has left searching',
  },
  {
    code: fact('policy_changed', CUSTODY, 77, 'POLICY_CHANGED = "policy_changed"'),
    message: fact('the frozen candidate is judged under {frozen_policy} and the policy would become {policy_digest}', CUSTODY, 585, 'f"the frozen candidate is judged under {self._frozen_policy} and the policy would " f"become {policy_digest}"'),
    condition: 'guard_policy with a different policy under a frozen candidate',
  },
  {
    code: fact('reservation_consumed', CUSTODY, 78, 'RESERVATION_CONSUMED = "reservation_consumed"'),
    message: fact("the family's confirmation reservation is spent and nothing releases it", CUSTODY, 503, '"the family\'s confirmation reservation is spent and nothing releases it"'),
    condition: 'freeze with a spent or inherited reservation; a read for the frozen candidate after its one reservation is spent',
  },
  {
    code: fact('reservation_unreserved', CUSTODY, 79, 'RESERVATION_UNRESERVED = "reservation_unreserved"'),
    message: fact("the family's reservation for {dataset} is spent and nothing releases it: a replacement needs newly reserved, unseen data or a separately pre-registered protocol", CUSTODY, 909, 'f"the family\'s reservation for {self._dataset} is spent and nothing releases " f"it: a replacement needs newly reserved, unseen data or a separately " f"pre-registered protocol"'),
    condition: 'read_holdout when the reservation state is unreserved (no reservation held)',
  },
  {
    code: fact('second_candidate', CUSTODY, 80, 'SECOND_CANDIDATE = "second_candidate"'),
    message: fact('the read is for {candidate_id} and the campaign confirms {frozen_candidate}: one campaign produces one confirmation result it may cite as independent', CUSTODY, 903, 'f"the read is for {candidate_id} and the campaign confirms " f"{self._frozen_candidate}: one campaign produces one confirmation result it " f"may cite as independent"'),
    condition: 'read_holdout for a non-frozen candidate without the exploratory label',
  },
  {
    code: fact('verdict_candidate_mismatch', CUSTODY, 81, 'VERDICT_CANDIDATE_MISMATCH = "verdict_candidate_mismatch"'),
    message: fact('the verdict names {candidate_id} and the frozen candidate is {frozen_candidate_or_none}: a verdict whose identity does not match is void rather than suspicious', CUSTODY, 730, 'f"the verdict names {candidate_id} and the frozen candidate is " f"{self._frozen_candidate or \'none\'}: a verdict whose identity does not match is " f"void rather than suspicious"'),
    condition: 'confirm for a candidate that is not the frozen one, or citation for another candidate',
  },
  {
    code: fact('exploratory_label_required', CUSTODY, 82, 'EXPLORATORY_LABEL_REQUIRED = "exploratory_label_required"'),
    message: null,
    condition: 'declared only: no path raises it today; the condition it names is reported under second_candidate',
  },
  {
    code: fact('confirmation_access_without_reservation', CUSTODY, 83, 'CONFIRMATION_ACCESS_WITHOUT_RESERVATION = "confirmation_access_without_reservation"'),
    message: null,
    condition: 'declared only: no path raises it today; the condition it names is reported under reservation_consumed or reservation_unreserved',
  },
  {
    code: fact('summary_statistic_in_parameters', CUSTODY, 84, 'SUMMARY_STATISTIC_IN_PARAMETERS = "summary_statistic_in_parameters"'),
    message: fact("the read would carry {summary_statistics} into the candidate's search-phase inputs, which quarantines it rather than confirming it", CUSTODY, 641, 'f"the read would carry {sorted(summary_statistics)} into the candidate\'s " f"search-phase inputs, which quarantines it rather than confirming it"'),
    condition: 'read_holdout with truthy summary_statistics, checked before the reservation is examined',
  },
  {
    code: fact('not_confirmed', CUSTODY, 85, 'NOT_CONFIRMED = "not_confirmed"'),
    message: fact('the campaign has read no holdout data for its frozen candidate, so it has no confirmation result to cite', CUSTODY, 740, '"the campaign has read no holdout data for its frozen candidate, so it has no " "confirmation result to cite"'),
    condition: 'confirm with no consumed reservation, or citation with no verdict or outside confirmed',
  },
];

/** The per-lineage downgrade rule: a partition that informed a lineage is development for it and
 *  stays locked for one that never saw it. One direction records the downgrade; the other is the
 *  check that refuses a consumed partition presented as fresh validation. */
export const DOWNGRADE_RULE = {
  informed: fact(
    { role: 'development', reason: '{by} was evaluated on {partition} and its result informed the search' },
    EVOLVE, 1038,
    'reason=f"{by} was evaluated on {partition} and its result informed the search"',
  ),
  asValidation: fact(
    { role: 'locked_validation', reason: 'the lineage has never consumed a result on this partition' },
    EVOLVE, 1063,
    'reason="the lineage has never consumed a result on this partition"',
  ),
  refusal: fact('role_mismatch', EVOLVE, 1065, 'return SearchRefusal.ROLE_MISMATCH'),
};

/** The walk-forward gate's reading of a role: only locked validation data is admissible. */
export const GATE_ROLE_CHECK = fact(
  'the data role is {role} and only locked validation data is admissible',
  GATES, 784,
  'reason = f"the data role is {role} and only locked validation data is admissible"',
);

/** The three roles, with their reader unwrapped for display. */
export function roles() {
  return ROLES.map((node) => {
    const role = valueOf(node);
    return { ...role, readBy: valueOf(role.readBy) };
  });
}

/** The six reservation states, with their labels unwrapped for display. */
export function reservationStates() {
  return RESERVATION_STATES.map(valueOf);
}

/** The transition table, with each row unwrapped for display. */
export function transitions() {
  return TRANSITIONS.map(valueOf);
}

/** The refusal vocabulary, with each code and message unwrapped for display. */
export function refusalCodes() {
  return REFUSAL_CODES.map((entry) => ({
    code: valueOf(entry.code),
    message: entry.message === null ? null : valueOf(entry.message),
    condition: entry.condition,
  }));
}

/** Whether a role may be read as validation evidence. */
export function admissibleAsValidation(role) {
  return valueOf(ADMISSIBLE_AS_VALIDATION)[role] === true;
}
