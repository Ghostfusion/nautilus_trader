/**
 * The transcribed executor vocabulary, records and rules.
 *
 * The one executor is `python/nautilus_trader/discovery/strategy.py`: a single interpreter the
 * engine drives, which decides on every bar and allocates only on a bar that opens a new rebalancing
 * period. This module transcribes the closed vocabularies it resolves, the record kinds it writes,
 * the nine declared allocation decisions, the difference between a decision bar and a rebalance bar,
 * the declared signal-frequency scope, and the caveat that the bridge stamps an instruction and
 * submits it without deferring. Every value is wrapped in `fact(...)` with the file, line and
 * verbatim text it was read from, so tools/check-facts.mjs can fail when a source moves on.
 */

import { fact, valueOf } from './source.js';

const STRATEGY = 'python/nautilus_trader/discovery/strategy.py';
const RUN = 'python/nautilus_trader/discovery/run.py';
const TUTORIAL = 'tutorials/strategy-discovery/one-executor/README.md';

/** The declared constants the interpreter resolves, each a member of a closed set it refuses
 *  outside of. Transcribed one by one so a rename in the source is caught member by member. */
export const CONSTANTS = {
  SELECTION_MODE_RANK: fact('rank', STRATEGY, 124, 'SELECTION_MODE_RANK = "rank"'),
  SELECTION_MODE_THRESHOLD: fact('threshold', STRATEGY, 125, 'SELECTION_MODE_THRESHOLD = "threshold"'),
  SELECTION_MODE_NONE: fact('none', STRATEGY, 126, 'SELECTION_MODE_NONE = "none"'),
  TIE_BREAK_FEATURE_DESC: fact('feature_desc', STRATEGY, 128, 'TIE_BREAK_FEATURE_DESC = "feature_desc"'),
  TIE_BREAK_INSTRUMENT_ID_ASC: fact('instrument_id_asc', STRATEGY, 129, 'TIE_BREAK_INSTRUMENT_ID_ASC = "instrument_id_asc"'),
  WEIGHTING_EQUAL: fact('equal', STRATEGY, 131, 'WEIGHTING_EQUAL = "equal"'),
  WEIGHTING_RANK_PROPORTIONAL: fact('rank_proportional', STRATEGY, 132, 'WEIGHTING_RANK_PROPORTIONAL = "rank_proportional"'),
  CASH_POLICY_LEAVE_IDLE: fact('leave_idle', STRATEGY, 134, 'CASH_POLICY_LEAVE_IDLE = "leave_idle"'),
  CASH_POLICY_REALLOCATE: fact('reallocate', STRATEGY, 135, 'CASH_POLICY_REALLOCATE = "reallocate"'),
  REBALANCE_DAILY: fact('daily', STRATEGY, 137, 'REBALANCE_DAILY = "daily"'),
  REBALANCE_WEEKLY: fact('weekly', STRATEGY, 138, 'REBALANCE_WEEKLY = "weekly"'),
  REBALANCE_MONTHLY: fact('monthly', STRATEGY, 139, 'REBALANCE_MONTHLY = "monthly"'),
  COMPARISON_GT: fact('gt', STRATEGY, 141, 'COMPARISON_GT = "gt"'),
  COMPARISON_GE: fact('ge', STRATEGY, 142, 'COMPARISON_GE = "ge"'),
  COMPARISON_LT: fact('lt', STRATEGY, 143, 'COMPARISON_LT = "lt"'),
  COMPARISON_LE: fact('le', STRATEGY, 144, 'COMPARISON_LE = "le"'),
  SIGNAL_SOURCE: fact('discovery', STRATEGY, 148, 'SIGNAL_SOURCE = "discovery"'),
  UNIT: fact('1', STRATEGY, 159, '_UNIT = Decimal(1)'),
  NS_PER_SECOND: fact(1000000000, STRATEGY, 161, '_NS_PER_SECOND = 1_000_000_000'),
};

/** The closed sets, in the order the source declares them. A value outside one of these is refused
 *  rather than defaulted, which is why the set itself is the transcription. */
export const CLOSED_SETS = [
  fact(
    { name: 'selection modes', members: ['rank', 'threshold', 'none'] },
    STRATEGY, 163,
    '_SELECTION_MODES = (SELECTION_MODE_RANK, SELECTION_MODE_THRESHOLD, SELECTION_MODE_NONE)',
  ),
  fact(
    { name: 'tie breaks', members: ['feature_desc', 'instrument_id_asc'] },
    STRATEGY, 164,
    '_TIE_BREAKS = (TIE_BREAK_FEATURE_DESC, TIE_BREAK_INSTRUMENT_ID_ASC)',
  ),
  fact(
    { name: 'weightings', members: ['equal', 'rank_proportional'] },
    STRATEGY, 165,
    '_WEIGHTINGS = (WEIGHTING_EQUAL, WEIGHTING_RANK_PROPORTIONAL)',
  ),
  fact(
    { name: 'cash policies', members: ['leave_idle', 'reallocate'] },
    STRATEGY, 166,
    '_CASH_POLICIES = (CASH_POLICY_LEAVE_IDLE, CASH_POLICY_REALLOCATE)',
  ),
  fact(
    { name: 'rebalance frequencies', members: ['daily', 'weekly', 'monthly'] },
    STRATEGY, 167,
    '_REBALANCE_FREQUENCIES = (REBALANCE_DAILY, REBALANCE_WEEKLY, REBALANCE_MONTHLY)',
  ),
  fact(
    { name: 'comparisons', members: ['gt', 'ge', 'lt', 'le'] },
    STRATEGY, 168,
    '_COMPARISONS = (COMPARISON_GT, COMPARISON_GE, COMPARISON_LT, COMPARISON_LE)',
  ),
];

/** The record kinds one run writes, each with its fields in the exact order the source declares
 *  them. The order is the transcription: a record whose fields reordered is a different record. */
export const RECORD_KINDS = [
  fact(
    { name: 'CrossSectionEntry', note: 'one feature value a decision read', fields: ['instrument_id', 'alias', 'value', 'observed_at', 'available_at', 'member'] },
    STRATEGY, 510, 'class CrossSectionEntry:',
  ),
  fact(
    { name: 'SignalRecord', note: 'one signal a decision submitted', fields: ['instrument_id', 'direction', 'strength', 'ts_event', 'ts_init'] },
    STRATEGY, 540, 'class SignalRecord:',
  ),
  fact(
    { name: 'FillRecord', note: 'one fill the engine reported back to the interpreter', fields: ['client_order_id', 'instrument_id', 'decision_at', 'ts_event', 'ts_init', 'last_px'] },
    STRATEGY, 567, 'class FillRecord:',
  ),
  fact(
    { name: 'DecisionRecord', note: 'everything one decision read, decided and submitted', fields: ['decision_at', 'ts_init', 'rebalance', 'exits', 'cross_section', 'selection', 'targets', 'signals', 'client_order_ids'] },
    STRATEGY, 598, 'class DecisionRecord:',
  ),
  fact(
    { name: 'InterpreterRunInputs', note: 'the run inputs one interpreter is configured from', fields: ['plan', 'memberships', 'instrument_ids', 'venue', 'commission_rate', 'stop_loss_bps', 'sectors'] },
    STRATEGY, 637, 'class InterpreterRunInputs:',
  ),
  fact(
    { name: 'AllocationRequest', note: 'everything one allocation is computed from', fields: ['ts_event', 'ts_init', 'selected', 'held', 'held_weights', 'ineligible', 'sector_of', 'weighting', 'position_count', 'max_position_weight', 'max_sector_weight', 'max_gross_exposure', 'cash_policy', 'flatten_ineligible'] },
    STRATEGY, 172, 'class AllocationRequest:',
  ),
  fact(
    { name: 'RunRequest', note: 'everything one experiment needs', fields: ['plan', 'strategy', 'venues', 'data', 'engine', 'metrics', 'seed', 'snapshot', 'run_id'] },
    RUN, 61, 'class RunRequest:',
  ),
];

/** A decision bar against a rebalance bar: the decision period is a UTC calendar day, so a
 *  holding-period cap declared in periods counts sessions rather than bars. */
export const DECISION_PERIOD = fact(
  { key: '(year, month, day)', countedIn: 'UTC calendar days, not bars' },
  STRATEGY, 491,
  'The first release declares daily signal frequencies, so a decision period is a calendar day and',
);

/** How the declared rebalance frequency turns a decision instant into a period key, and why a bar
 *  rebalances: `daily` has no period, and a periodic frequency rebalances on the first bar whose
 *  key differs from the previous decision's. */
export const REBALANCE_PERIODS = [
  fact({ frequency: 'daily', key: null, rule: 'every decision bar rebalances' }, STRATEGY, 442, 'def rebalance_period_key(ts_ns: int, frequency: str) -> tuple[int, int] | None:'),
  fact({ frequency: 'weekly', key: '(isocalendar year, isocalendar week)', rule: 'first bar whose key differs' }, STRATEGY, 474, 'if frequency == REBALANCE_WEEKLY:'),
  fact({ frequency: 'monthly', key: '(year, month)', rule: 'first bar whose key differs' }, STRATEGY, 479, 'if frequency == REBALANCE_MONTHLY:'),
];

/** The rebalance bar is the decision bar whose period key differs from the previous decision's; the
 *  instant is the decision's own, never the wall clock's. */
export const REBALANCE_BAR = fact(
  { reads: 'the decision instant', updates: 'the last period key when the bar rebalances' },
  STRATEGY, 1177,
  'def _is_rebalance_bar(self, decision_at: int) -> bool:',
);

/** The nine allocation decisions, in the order the design fixes them. The order is load-bearing:
 *  the weighting comes first, then the position cap, the sector cap and the gross exposure cap, and
 *  the cash policy disposes of whatever weight those caps leave unused. */
export const ALLOCATION_DECISIONS = [
  fact({ n: 1, title: 'The declared weighting method', rule: 'equal gives every selected instrument one share; rank_proportional gives the i-th name count - i, so the first name gets the most' }, STRATEGY, 1746, 'def _initial_weights('),
  fact({ n: 2, title: 'The selection order is preserved and a repeated instrument is weighted once', rule: 'a repeated identifier is weighted once, at its first position' }, STRATEGY, 1716, 'def _selected_order(selected: tuple[str, ...], position_count: int) -> tuple[str, ...]:'),
  fact({ n: 3, title: 'The portfolio size is a pre-trade cap on the selection', rule: 'the first position_count names are kept, the rest are dropped, and a shorter selection is not padded' }, STRATEGY, 1716, 'def _selected_order(selected: tuple[str, ...], position_count: int) -> tuple[str, ...]:'),
  fact({ n: 4, title: 'A held ineligible instrument is flattened only when a rule declares it', rule: 'the declared flatten rule is the exit block or the runtime block, and either one flattens' }, STRATEGY, 1913, 'def _emitted_weights('),
  fact({ n: 5, title: 'A holding the selection no longer names is flattened, and an untouched name gets no target', rule: 'held - weighted is flattened; an instrument neither selected nor held is not emitted at all' }, STRATEGY, 1913, 'def _emitted_weights('),
  fact({ n: 6, title: 'The position cap clamps each weight and redistributes nothing', rule: 'capped_i = min(weight_i, max_position_weight)' }, STRATEGY, 1788, 'def _apply_position_cap(weights: dict[str, Decimal], caps: _Caps) -> dict[str, Decimal]:'),
  fact({ n: 7, title: 'The sector cap scales a sector down proportionally', rule: 'weight * cap / total when the sector total exceeds the cap; an unnamed instrument is its own sector' }, STRATEGY, 1809, 'def _apply_sector_cap(weights: dict[str, Decimal], caps: _Caps) -> dict[str, Decimal]:'),
  fact({ n: 8, title: 'The gross exposure cap scales every weight down proportionally', rule: 'weight * ceiling / total when the total exceeds the investable ceiling' }, STRATEGY, 1838, 'def _apply_gross_cap(weights: dict[str, Decimal], caps: _Caps) -> dict[str, Decimal]:'),
  fact({ n: 9, title: 'The cash policy disposes of the residual the caps left', rule: 'leave_idle leaves it uninvested; reallocate offers it in the declared share proportions, capped by each name\u2019s headroom, so no cap is exceeded' }, STRATEGY, 1863, 'def _reallocate('),
];

/** The declared signal-frequency scope: the first release reads daily signals at the one bar
 *  specification the table holds, and a frequency the table does not hold is refused rather than
 *  approximated. */
export const SIGNAL_SCOPE = fact(
  { signalFrequency: 'daily', barSpecification: '1-DAY-LAST-EXTERNAL', entries: 1, unknown: 'refused, not approximated' },
  STRATEGY, 154,
  '"daily": "1-DAY-LAST-EXTERNAL",',
);

/** The first-release narrowness, verbatim: long only, daily signals, weekly or slower rebalancing,
 *  equities and exchange-traded funds, and no stop or target that needs the inside of an intraday
 *  bar. */
export const FIRST_RELEASE_SCOPE = fact(
  'The first release is narrow. It runs long only, on daily signals, with weekly or slower rebalancing, over equities and exchange-traded funds, and it has no stop or target that needs the inside of an intraday bar. A frequency outside the declared one is refused rather than approximated by a bar size the candidate never declared.',
  TUTORIAL, 235,
  '- The first release is narrow. It runs long only, on daily signals, with weekly or slower rebalancing,',
);

/** The timing caveat, verbatim: the bridge stamps an instruction with the instant it becomes
 *  eligible and submits it without waiting, so the deferred fill can land earlier than the stamp. */
export const TIMING_CAVEAT = fact(
  'The timing contract is not enforced by the bridge. The bridge stamps an instruction with the instant it becomes eligible and submits it without waiting, so the deferred fill can land earlier than the stamp. The missing piece is a submission path that can hold a clock and release the instruction at its stamped instant; until then the engine\u2019s latency model carries the contract. One settlement point ignores it: the shutdown drain fills an instruction the object is never told about, and a fill the object cannot see is a holding it cannot reason from.',
  TUTORIAL, 229,
  '- The timing contract is not enforced by the bridge. The bridge stamps an instruction with the instant',
);

/** The interpreter's refusals. The source raises a bare `ValueError`/`TypeError` with no code enum,
 *  so the `code` here is the model's own short handle and the `message` is the interpreter's
 *  verbatim message template, with `{...}` standing for the field it interpolates. */
export const REFUSALS = [
  fact(
    { code: 'cash_policy_not_declared', message: 'cash_policy must be one of {cash policies}, was {cash_policy}' },
    STRATEGY, 357,
    'f"cash_policy must be one of {\', \'.join(_CASH_POLICIES)}, was {request.cash_policy}"',
  ),
  fact(
    { code: 'selection_mode_not_declared', message: 'selection mode must be one of {selection modes}, was {selection.mode}' },
    STRATEGY, 434,
    'f"selection mode must be one of {\', \'.join(_SELECTION_MODES)}, was {selection.mode}"',
  ),
  fact(
    { code: 'rebalance_frequency_not_declared', message: 'rebalance frequency must be one of {rebalance frequencies}, was {frequency}' },
    STRATEGY, 483,
    'f"rebalance frequency must be one of {\', \'.join(_REBALANCE_FREQUENCIES)}, was {frequency}"',
  ),
  fact(
    { code: 'weighting_not_declared', message: 'weighting must be one of {weightings}, was {weighting}' },
    STRATEGY, 1780,
    'raise ValueError(f"weighting must be one of {\', \'.join(_WEIGHTINGS)}, was {weighting}")',
  ),
  fact(
    { code: 'signal_frequency_not_declared', message: 'signal frequency must be one of {declared}, was {signal_frequency}' },
    STRATEGY, 1653,
    'f"signal frequency must be one of {declared}, was {signal_frequency}"',
  ),
];

/** Everything in the closed vocabularies, in display order. */
export function constants() {
  return Object.fromEntries(Object.entries(CONSTANTS).map(([key, node]) => [key, valueOf(node)]));
}

/** The closed sets, with their members unwrapped for display. */
export function closedSets() {
  return CLOSED_SETS.map(valueOf);
}

/** The record kinds, with their field orderings unwrapped for display. */
export function recordKinds() {
  return RECORD_KINDS.map(valueOf);
}

/** The nine allocation decisions, in their fixed order. */
export function allocationDecisions() {
  return ALLOCATION_DECISIONS.map(valueOf);
}

/** The rebalance periods, keyed by the declared frequency. */
export function rebalancePeriods() {
  return REBALANCE_PERIODS.map(valueOf);
}
