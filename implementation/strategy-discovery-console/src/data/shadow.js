/**
 * The shadow period's contract, record and counts, transcribed.
 *
 * The shadow half asks one question of a frozen candidate: does a live feed behave as the backtest
 * assumed? So this module transcribes the four declarations the two halves must share, the parts
 * each side of the comparison records, the seven-field observation record, the three refusals the
 * monitor reports instead of a report, every count the report states, and the page's worked example
 * - five observations, their costs and the totals the report computes from them.
 */

import { fact, valueOf } from './source.js';

const SHADOW_PY = 'python/nautilus_trader/discovery/shadow.py';
const TUTORIAL = 'tutorials/strategy-discovery/the-shadow-period/README.md';

/** The schema and the label every shadow result carries. */
export const SHADOW = {
  schema: fact('discovery.shadow/v1', SHADOW_PY, 62, 'SHADOW_SCHEMA = "discovery.shadow/v1"'),
  label: fact('simulated', SHADOW_PY, 65, 'SIMULATED_LABEL = "simulated"'),
};

/** Why the monitor refused to produce a report rather than reporting a divergence. */
export const REFUSAL_CODES = [
  fact('minimum_not_declared', SHADOW_PY, 74, 'MINIMUM_NOT_DECLARED = "minimum_not_declared"'),
  fact('eligible_before_signal', SHADOW_PY, 75, 'ELIGIBLE_BEFORE_SIGNAL = "eligible_before_signal"'),
  fact(
    'non_fill_without_reason',
    SHADOW_PY,
    76,
    'NON_FILL_WITHOUT_REASON = "non_fill_without_reason"',
  ),
];

/** The four declarations replay and shadow must share, in the code's order. */
export const CONTRACT_FIELDS = [
  fact('signal_frequency', SHADOW_PY, 123, 'signal_frequency: str'),
  fact('decision_time', SHADOW_PY, 124, 'decision_time: str'),
  fact('execution_eligibility', SHADOW_PY, 125, 'execution_eligibility: str'),
  fact('corporate_action_treatment', SHADOW_PY, 126, 'corporate_action_treatment: str'),
];

/** One decision's pair of instants, under the contract it was taken under. */
export const READING_FIELDS = [
  fact('contract', SHADOW_PY, 208, 'contract: TemporalContract'),
  fact('decision_at', SHADOW_PY, 209, 'decision_at: int'),
  fact('eligible_fill_at', SHADOW_PY, 210, 'eligible_fill_at: int'),
];

/** What a shadow period was declared to be, before it started. */
export const PLAN_FIELDS = [
  fact('candidate_id', SHADOW_PY, 392, 'candidate_id: str'),
  fact('specification_digest', SHADOW_PY, 393, 'specification_digest: str'),
  fact('period', SHADOW_PY, 394, 'period: str'),
  fact('minimum_observations', SHADOW_PY, 395, 'minimum_observations: int'),
];

/**
 * The seven fields of one observation, in the code's order. The page's rule 6 lists six and omits
 * the reason; the design's section 11.5 lists six and omits the observed cost; the record carries
 * seven.
 */
export const OBSERVATION_FIELDS = [
  fact('signal_at', SHADOW_PY, 455, 'signal_at: int'),
  fact('intended_entry', SHADOW_PY, 456, 'intended_entry: bool'),
  fact('eligible_fill_at', SHADOW_PY, 457, 'eligible_fill_at: int'),
  fact('fill_price', SHADOW_PY, 458, 'fill_price: Decimal | None = None'),
  fact('modelled_cost', SHADOW_PY, 459, 'modelled_cost: Decimal | None = None'),
  fact('observed_cost', SHADOW_PY, 460, 'observed_cost: Decimal | None = None'),
  fact('non_fill_reason', SHADOW_PY, 461, 'non_fill_reason: str = ""'),
];

/** One observation whose signal time is not an instant the replay decided at. */
export const DISCREPANCY_FIELDS = [
  fact('expected_at', SHADOW_PY, 508, 'expected_at: int'),
  fact('observed_at', SHADOW_PY, 509, 'observed_at: int'),
  fact('magnitude', SHADOW_PY, 510, 'magnitude: int'),
];

/** One simulated non-fill, with the reason rather than only the fact. */
export const NON_FILL_FIELDS = [
  fact('signal_at', SHADOW_PY, 544, 'signal_at: int'),
  fact('intended_entry', SHADOW_PY, 545, 'intended_entry: bool'),
  fact('reason', SHADOW_PY, 546, 'reason: str'),
];

/** What a shadow period observed, as divergence rather than as a summary. */
export const REPORT_FIELDS = [
  fact('plan', SHADOW_PY, 590, 'plan: ShadowPlan'),
  fact('contract', SHADOW_PY, 591, 'contract: TemporalContract'),
  fact('observations', SHADOW_PY, 592, 'observations: tuple[ShadowObservation, ...] = ()'),
  fact('expected_signal_times', SHADOW_PY, 593, 'expected_signal_times: tuple[int, ...] = ()'),
  fact('label', SHADOW_PY, 594, 'label: str = SIMULATED_LABEL'),
];

/** The parity rule, as the page states it, and as the code evaluates the two halves. */
export const PARITY = {
  page: fact(
    'holds = same_fields and same_pair',
    TUTORIAL,
    93,
    `holds = same_fields and same_pair
same_fields = the ordered list of (name, value) pairs is equal on both sides
same_pair   = (decision_at, eligible_fill_at) is equal on both sides`,
  ),
  sameFields: fact(
    'the ordered declaration lists of the two readings are equal, value by value',
    SHADOW_PY,
    271,
    'return self.replay.contract.declared_fields() == self.shadow.contract.declared_fields()',
  ),
  samePair: fact(
    'the decision instant and the eligible fill instant are equal on both sides',
    SHADOW_PY,
    278,
    'return self.replay.pair() == self.shadow.pair()',
  ),
  holds: fact(
    'the contract is shared only when both halves hold',
    SHADOW_PY,
    285,
    'return self.same_fields and self.same_pair',
  ),
};

/** Every count the report states, with the expression that computes it. */
export const COUNTS = {
  page: fact(
    'reached = observed_decisions = the number of observations delivered',
    TUTORIAL,
    109,
    `reached              = observed_decisions = the number of observations delivered
readable             = reached >= minimum_observations
missing_observations = how many expected instants the feed never delivered a signal for
cost_difference      = observed_cost - modelled_cost
diverged             = any discrepancy, or any non-fill, or missing_observations > 0`,
  ),
  eligibleDecisions: fact('the number of instants the replay was expected to decide at', SHADOW_PY, 608, 'return len(self.expected_signal_times)'),
  observedDecisions: fact('the number of observations the feed delivered', SHADOW_PY, 615, 'return len(self.observations)'),
  reached: fact('the count the report states, which is the observed decision count', SHADOW_PY, 622, 'return self.observed_decisions'),
  readable: fact('true only when the period reached the count declared before it started', SHADOW_PY, 629, 'return self.reached >= self.plan.minimum_observations'),
  unreadableReason: fact(
    `f"the period reached {self.reached} observations against the "
            f"{self.plan.minimum_observations} declared before it started"`,
    SHADOW_PY,
    640,
    `f"the period reached {self.reached} observations against the "
            f"{self.plan.minimum_observations} declared before it started"`,
  ),
  missingObservations: fact(
    'how many expected instants the feed never delivered a signal for',
    SHADOW_PY,
    651,
    'return sum(1 for instant in self.expected_signal_times if instant not in delivered)',
  ),
  costDifference: fact(
    'the observed cost less the modelled cost',
    SHADOW_PY,
    727,
    'return self.observed_cost - self.modelled_cost',
  ),
  diverged: fact(
    'any discrepancy, any non-fill, or any missing observation',
    SHADOW_PY,
    735,
    'bool(self.signal_discrepancies) or bool(self.non_fills) or self.missing_observations > 0',
  ),
};

/** The refusals and the findings lines, in the code's own words. */
export const MESSAGES = {
  errorPrefix: fact(
    '{code}: {message}',
    SHADOW_PY,
    96,
    'super().__init__(f"{code.value}: {message}")',
  ),
  minimumNotDeclared: fact(
    { code: 'minimum_not_declared', message: 'the plan declares {n} as its minimum observation count' },
    SHADOW_PY,
    863,
    'f"the plan declares {plan.minimum_observations} as its minimum observation count",',
  ),
  eligibleBeforeSignal: fact(
    { code: 'eligible_before_signal', message: 'an observation at {signal} is eligible at {fill}' },
    SHADOW_PY,
    870,
    `f"an observation at {observation.signal_at} is eligible at "
                f"{observation.eligible_fill_at}",`,
  ),
  nonFillWithoutReason: fact(
    {
      code: 'non_fill_without_reason',
      message: 'the intended entry at {signal} did not fill and gives no reason',
    },
    SHADOW_PY,
    877,
    'f"the intended entry at {observation.signal_at} did not fill and gives no reason",',
  ),
  missingFinding: fact(
    '{count} observations were never delivered',
    SHADOW_PY,
    749,
    'found.append(f"{self.missing_observations} observations were never delivered")',
  ),
  discrepancyFinding: fact(
    'a signal arrived {magnitude} ns from the expected {expected}',
    SHADOW_PY,
    751,
    `            f"a signal arrived {discrepancy.magnitude} ns from the expected "
            f"{discrepancy.expected_at}"`,
  ),
  nonFillFinding: fact(
    'the signal at {signal} did not fill: {reason}',
    SHADOW_PY,
    756,
    'f"the signal at {non_fill.signal_at} did not fill: {non_fill.reason}"',
  ),
  costFinding: fact(
    'the observed cost differed from the modelled one by {difference}',
    SHADOW_PY,
    761,
    'f"the observed cost differed from the modelled one by {self.cost_difference}"',
  ),
  unreadableReason: fact(
    'the period reached {reached} observations against the {minimum} declared before it started',
    SHADOW_PY,
    640,
    `f"the period reached {self.reached} observations against the "
            f"{self.plan.minimum_observations} declared before it started"`,
  ),
  parityHolds: fact(
    'replay and shadow share one temporal contract',
    SHADOW_PY,
    297,
    'return "replay and shadow share one temporal contract"',
  ),
  parityFields: fact(
    'the declared fields differ: {fields}',
    SHADOW_PY,
    301,
    'parts.append(f"the declared fields differ: {\', \'.join(self.differing_fields)}")',
  ),
  parityPair: fact(
    'the timestamps differ: {replay} against {shadow}',
    SHADOW_PY,
    304,
    'f"the timestamps differ: {self.replay.pair()} against {self.shadow.pair()}"',
  ),
};

/**
 * The page's worked example: five weekly observations of one candidate, with the times in seconds
 * from the start of the period as the page writes them. The code stores instants as nanoseconds, so
 * the view multiplies before it runs the monitor.
 */
export const WORKED_EXAMPLE = [
  fact(
    {
      intendedEntry: true,
      eligibleFillSeconds: 86400,
      fillPrice: 25.14,
      modelledCost: 0.5,
      observedCost: 0.62,
      nonFillReason: '',
    },
    TUTORIAL,
    134,
    '| 1   | 0               | yes            | 86400                  | 25.14      | 0.50          | 0.62          |                                                 |',
  ),
  fact(
    {
      intendedEntry: true,
      eligibleFillSeconds: 691200,
      fillPrice: 25.4,
      modelledCost: 0.5,
      observedCost: 0.48,
      nonFillReason: '',
    },
    TUTORIAL,
    135,
    '| 2   | 604800          | yes            | 691200                 | 25.40      | 0.50          | 0.48          |                                                 |',
  ),
  fact(
    {
      intendedEntry: true,
      eligibleFillSeconds: 1274400,
      fillPrice: null,
      modelledCost: null,
      observedCost: null,
      nonFillReason: 'the feed delivered no quote at the session open',
    },
    TUTORIAL,
    136,
    '| 3   | 1209600         | yes            | 1274400                |            |               |               | the feed delivered no quote at the session open |',
  ),
  fact(
    {
      intendedEntry: false,
      eligibleFillSeconds: 1900800,
      fillPrice: null,
      modelledCost: null,
      observedCost: null,
      nonFillReason: '',
    },
    TUTORIAL,
    137,
    '| 4   | 1814400         | no             | 1900800                |            |               |               |                                                 |',
  ),
  fact(
    {
      intendedEntry: true,
      eligibleFillSeconds: 2505600,
      fillPrice: 25.02,
      modelledCost: 0.5,
      observedCost: 0.55,
      nonFillReason: '',
    },
    TUTORIAL,
    138,
    '| 5   | 2419200         | yes            | 2505600                | 25.02      | 0.50          | 0.55          |                                                 |',
  ),
];

/** The signal times the replay of the worked example was expected to decide at, in seconds. */
export const WORKED_SIGNAL_SECONDS = fact(
  [0, 604800, 1209600, 1814400, 2419200],
  TUTORIAL,
  146,
  'Now the whole period. The replay was expected to decide at five instants, seconds 0, 604800,\n1209600, 1814400 and 2419200, and the feed delivered an observation for each, so nothing is missing\nand no signal time is unexpected.',
);

/** The totals the page computes from those five rows, which are what the report must state. */
export const WORKED_TOTALS = {
  eligibleDecisions: fact(5, TUTORIAL, 153, 'eligible_decisions    = 5'),
  observedDecisions: fact(5, TUTORIAL, 154, 'observed_decisions    = 5'),
  reached: fact(5, TUTORIAL, 155, 'reached               = 5'),
  missingObservations: fact(0, TUTORIAL, 156, 'missing_observations  = 0'),
  signalDiscrepancies: fact(0, TUTORIAL, 157, 'signal_discrepancies  = 0'),
  nonFills: fact(1, TUTORIAL, 158, 'non_fills             = 1'),
  modelledCost: fact(1.5, TUTORIAL, 159, 'modelled_cost         = 0.50 + 0.50 + 0.50            = 1.50'),
  observedCost: fact(1.65, TUTORIAL, 160, 'observed_cost         = 0.62 + 0.48 + 0.55            = 1.65'),
  costDifference: fact(0.15, TUTORIAL, 161, 'cost_difference       = 1.65 - 1.50                   = +0.15'),
  diverged: fact(true, TUTORIAL, 162, 'diverged              = True, because one non-fill was found'),
  minimumObservations: fact(4, TUTORIAL, 165, 'With a declared minimum of four observations, `readable` is true, because five reached. Had the plan'),
};

/** The four declarations, in the code's order. */
export function contractFields() {
  return CONTRACT_FIELDS.map(valueOf);
}

/** The seven fields of one observation, in the code's order. */
export function observationFields() {
  return OBSERVATION_FIELDS.map(valueOf);
}

/** What a shadow period was declared to be, unwrapped. */
export function planFields() {
  return PLAN_FIELDS.map(valueOf);
}

/** What a period's report carries, unwrapped. */
export function reportFields() {
  return REPORT_FIELDS.map(valueOf);
}

/** One decision's pair of instants, unwrapped. */
export function readingFields() {
  return READING_FIELDS.map(valueOf);
}

/** One discrepancy's fields, unwrapped. */
export function discrepancyFields() {
  return DISCREPANCY_FIELDS.map(valueOf);
}

/** One non-fill's fields, unwrapped. */
export function nonFillFields() {
  return NON_FILL_FIELDS.map(valueOf);
}

/** The three refusals the monitor reports instead of a report. */
export function refusalCodes() {
  return REFUSAL_CODES.map(valueOf);
}

/** The messages the monitor reports, as { key: { code, message } }, still templated. */
export function refusalMessages() {
  return Object.fromEntries(
    Object.entries(MESSAGES).map(([key, node]) => [key, valueOf(node)]),
  );
}

/** The page's worked example, with each row's signal time carried in from the expected list. */
export function workedExample() {
  const signalSeconds = valueOf(WORKED_SIGNAL_SECONDS);
  return WORKED_EXAMPLE.map((node, index) => ({ signalSeconds: signalSeconds[index], ...valueOf(node) }));
}

/** The totals the page computes from the worked example. */
export function workedTotals() {
  return Object.fromEntries(Object.entries(WORKED_TOTALS).map(([key, node]) => [key, valueOf(node)]));
}
