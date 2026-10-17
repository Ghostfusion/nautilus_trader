/**
 * Transcription: the four counts, their versioned fold, the effective-count declaration and the two
 * corrections that consume them.
 *
 * WHY THIS MODULE EXISTS. The counting layer is a definition rather than a measurement: four sums
 * over one append-only event list, a declaration that separates the raw count from the effective
 * count, and two pieces of arithmetic - the deflated Sharpe ratio and the probability of backtest
 * overfitting - that read the counts rather than a number a caller hands them. Every constant, name,
 * refusal message and formula below is transcribed verbatim from the file that states it, wrapped in
 * `fact(...)` so tools/check-facts.mjs can prove the text is still there.
 *
 * Nothing here computes. The fold, the correction and the diagnostic live in `src/engine/counts.js`;
 * this module only states what the repository says, with the line it said it at.
 */

import { fact } from './source.js';

const LEDGER = 'python/nautilus_trader/discovery/ledger.py';
const SIGNIFICANCE = 'python/nautilus_trader/optimization/significance.py';
const GATES = 'python/nautilus_trader/discovery/gates.py';
const TUTORIAL = 'tutorials/strategy-discovery/counting-the-search/README.md';

/** The versioned identity of the mapping that turns events into the four counts. */
export const COUNTS_MAPPING = fact(
  'discovery.counts/v1',
  LEDGER,
  83,
  'COUNTS_MAPPING = "discovery.counts/v1"',
);

/** The four exact names a report uses for the counts, in the order the enum declares them. */
export const COUNT_NAMES = [
  fact('proposal_attempt_count', LEDGER, 477, 'PROPOSAL_ATTEMPT = "proposal_attempt_count"'),
  fact(
    'unique_configuration_count',
    LEDGER,
    478,
    'UNIQUE_CONFIGURATION = "unique_configuration_count"',
  ),
  fact('executed_experiment_count', LEDGER, 479, 'EXECUTED_EXPERIMENT = "executed_experiment_count"'),
  fact('statistical_trial_count', LEDGER, 480, 'STATISTICAL_TRIAL = "statistical_trial_count"'),
];

/** The closed vocabulary of inventory events. An event outside it cannot change a count. */
export const EVENT_KINDS = [
  fact('proposal_recorded', LEDGER, 457, 'PROPOSAL_RECORDED = "proposal_recorded"'),
  fact('specification_refused', LEDGER, 458, 'SPECIFICATION_REFUSED = "specification_refused"'),
  fact('compilation_recorded', LEDGER, 459, 'COMPILATION_RECORDED = "compilation_recorded"'),
  fact('equivalence_merge', LEDGER, 460, 'EQUIVALENCE_MERGE = "equivalence_merge"'),
  fact('job_submitted', LEDGER, 461, 'JOB_SUBMITTED = "job_submitted"'),
  fact(
    'execution_attempt_closed',
    LEDGER,
    462,
    'EXECUTION_ATTEMPT_CLOSED = "execution_attempt_closed"',
  ),
  fact('registration_verified', LEDGER, 463, 'REGISTRATION_VERIFIED = "registration_verified"'),
  fact('trial_admitted', LEDGER, 464, 'TRIAL_ADMITTED = "trial_admitted"'),
  fact('registration_incomplete', LEDGER, 465, 'REGISTRATION_INCOMPLETE = "registration_incomplete"'),
  fact('duplicate_registration', LEDGER, 466, 'DUPLICATE_REGISTRATION = "duplicate_registration"'),
  fact('gate_verdict', LEDGER, 467, 'GATE_VERDICT = "gate_verdict"'),
  fact('custody_event', LEDGER, 468, 'CUSTODY_EVENT = "custody_event"'),
  fact('search_policy_recorded', LEDGER, 469, 'SEARCH_POLICY_RECORDED = "search_policy_recorded"'),
];

/**
 * The four conditions the single fold tests, transcribed from the one pass that takes the counts.
 * Each is quoted as contiguous source; a kind absent from the chain (such as `equivalence_merge`)
 * changes no count.
 */
export const FOLD_RULES = [
  fact(
    'proposal_recorded adds one to proposals',
    LEDGER,
    695,
    'if event.kind is EventKind.PROPOSAL_RECORDED: proposals += 1',
  ),
  fact(
    'compilation_recorded adds one to configurations only when the identity is new',
    LEDGER,
    697,
    'elif event.kind is EventKind.COMPILATION_RECORDED: configurations += 1 if event.payload.get("new_configuration") else 0',
  ),
  fact(
    'execution_attempt_closed adds one to executions whatever the outcome',
    LEDGER,
    699,
    'elif event.kind is EventKind.EXECUTION_ATTEMPT_CLOSED: executions += 1',
  ),
  fact(
    'trial_admitted counts once per distinct idempotency key',
    LEDGER,
    701,
    'elif event.kind is EventKind.TRIAL_ADMITTED: admitted.add(event.key)',
  ),
];

/** The four sums, written out with every symbol named, as the tutorial states them. */
export const FOLD_FORMULA = fact(
  [
    'proposal_attempt_count     = the number of `proposal_recorded` events',
    'unique_configuration_count = the sum over `compilation_recorded` of 1 when the identity is new, else 0',
    'executed_experiment_count  = the number of `execution_attempt_closed` events',
    'statistical_trial_count    = the number of distinct idempotency keys among `trial_admitted` events',
  ].join('\n'),
  TUTORIAL,
  81,
  'proposal_attempt_count     = the number of `proposal_recorded` events\nunique_configuration_count = the sum over `compilation_recorded` of 1 when the identity is new, else 0\nexecuted_experiment_count  = the number of `execution_attempt_closed` events\nstatistical_trial_count    = the number of distinct idempotency keys among `trial_admitted` events',
);

/** The consistency check the fold's object refuses: a trial is a verified execution. */
export const CONSISTENCY_CHECK = fact(
  'statistical_trials <= executed_experiments',
  TUTORIAL,
  96,
  'In the code the check is `statistical_trials <= executed_experiments`.',
);

/** Both dependence refusals, in the ledger's own words. */
export const DEPENDENCE_REFUSALS = [
  fact(
    'a dependent campaign that declares no effective count is refused',
    LEDGER,
    2628,
    'raise ValueError("a campaign with dependent trials must declare its effective trial count")',
  ),
  fact(
    'an independent campaign that declares an effective count is refused',
    LEDGER,
    2630,
    'raise ValueError("a campaign with independent trials must not declare an effective count")',
  ),
];

/** The effective-count relation: raw and effective are separate numbers joined by a declaration. */
export const EFFECTIVE_COUNT_RELATION = fact(
  'N = statistical_trial_count   when the attempts are independent\nN = effective_trials          when they are dependent and one is declared',
  TUTORIAL,
  103,
  'N = statistical_trial_count   when the attempts are independent\nN = effective_trials          when they are dependent and one is declared',
);

/** Only a completed or economically-failed attempt produced a usable return series. */
export const RETURN_SERIES_RULE = fact(
  'an attempt produced a usable return series only when completed or economically failed',
  LEDGER,
  526,
  'return self in (AttemptOutcome.COMPLETED, AttemptOutcome.ECONOMICALLY_FAILED)',
);

/** Only the locked-validation role may be read as validation evidence. */
export const VALIDATION_ROLE_RULE = fact(
  'only the locked-validation role is admissible as validation',
  LEDGER,
  503,
  'return self is DataRole.LOCKED_VALIDATION',
);

/** The deflated Sharpe arithmetic, transcribed from the docstring of the function that computes it. */
export const DEFLATED_SHARPE_FORMULA = fact(
  'SR0 = sqrt(V[SR]) * ((1 - gamma) * Phi^-1(1 - 1/N) + gamma * Phi^-1(1 - 1/(N e)))\nDSR = Phi((SR - SR0) * sqrt(T - 1) / sqrt(1 - skew * SR + ((kurtosis - 1) / 4) * SR^2))',
  SIGNIFICANCE,
  1066,
  'SR0 = sqrt(V[SR]) * ((1 - gamma) * Phi^-1(1 - 1/N) + gamma * Phi^-1(1 - 1/(N e)))\nDSR = Phi((SR - SR0) * sqrt(T - 1) / sqrt(1 - skew * SR + ((kurtosis - 1) / 4) * SR^2))',
);

/** The symbols of the correction, named one by one. */
export const DEFLATED_SHARPE_SYMBOLS = fact(
  'V[SR] the cross-trial variance of the Sharpe estimates, N the trial count the sample declares, T the contributing periods, gamma the Euler-Mascheroni constant',
  SIGNIFICANCE,
  1070,
  'where `V[SR]` is the cross-trial variance of the Sharpe estimates, `N` the trial count the sample declares, `T` the contributing periods, and `gamma` the Euler-Mascheroni constant.',
);

/** The correction is reported, never a gate. */
export const DEFLATED_SHARPE_NEVER_A_GATE = fact(
  'the value is reported, never a gate',
  SIGNIFICANCE,
  1075,
  'The value is reported, never a gate.',
);

/** The statistical contract and the constants its guards read. */
export const SIGNIFICANCE_CONTRACT = [
  fact('deflated_sharpe_ratio', SIGNIFICANCE, 105, 'SIGNIFICANCE_CONTRACT_ID = "deflated_sharpe_ratio"'),
  fact(1, SIGNIFICANCE, 106, 'SIGNIFICANCE_CONTRACT_VERSION = 1'),
  fact(20, SIGNIFICANCE, 111, 'DEFAULT_MINIMUM_OBSERVATIONS = 20'),
  fact(10, SIGNIFICANCE, 112, 'DEFAULT_MINIMUM_TRIALS = 10'),
  fact(0.5772156649015329, SIGNIFICANCE, 125, 'EULER_MASCHERONI = 0.5772156649015329'),
  fact(2, SIGNIFICANCE, 133, '_MINIMUM_MOMENT_OBSERVATIONS = 2'),
  fact(
    'the cross-trial variance of the Sharpe estimates uses the sample divisor',
    SIGNIFICANCE,
    360,
    'trial_variance_divisor: DivisorConvention = DivisorConvention.SAMPLE',
  ),
  fact(
    'the per-period Sharpe ratio uses the sample divisor',
    SIGNIFICANCE,
    359,
    'sharpe_divisor: DivisorConvention = DivisorConvention.SAMPLE',
  ),
];

/** The status and reason vocabulary a correction's result carries. */
export const SIGNIFICANCE_STATUSES = [
  fact('computed', SIGNIFICANCE, 143, 'MetricStatus.COMPUTED: "computed",'),
  fact('unavailable', SIGNIFICANCE, 144, 'MetricStatus.UNAVAILABLE: "unavailable",'),
  fact('invalid', SIGNIFICANCE, 145, 'MetricStatus.INVALID: "invalid",'),
  fact('insufficient_data', SIGNIFICANCE, 150, 'MetricReason.INSUFFICIENT_DATA: "insufficient_data",'),
  fact('undefined_result', SIGNIFICANCE, 154, 'MetricReason.UNDEFINED_RESULT: "undefined_result",'),
];

/** The declarations the correction refuses, in the sample's own words. */
export const CORRECTION_REFUSALS = [
  fact(
    'an annualised input is refused rather than rescaled',
    SIGNIFICANCE,
    701,
    'f"the correction is per-period: an annualised input "',
  ),
  fact(
    'a study with independent trials must not declare an effective trial count',
    SIGNIFICANCE,
    735,
    '"a study with independent trials must not declare an effective trial count: "',
  ),
  fact(
    'a study with dependent trials must declare its effective trial count',
    SIGNIFICANCE,
    741,
    'raise ValueError("a study with dependent trials must declare its effective trial count")',
  ),
  fact(
    'an effective trial count above the nominal count is refused',
    SIGNIFICANCE,
    746,
    'f"effective_trials {self.effective_trials} exceeds the nominal count "',
  ),
];

/** The overfitting diagnostic's formula, as the tutorial states it. */
export const PBO_FORMULA = fact(
  'combinations = the number of ways to choose S/2 blocks out of S\nPBO          = the share of those combinations whose in-sample best\n               ranks below the median out of sample',
  TUTORIAL,
  117,
  'combinations = the number of ways to choose S/2 blocks out of S\nPBO          = the share of those combinations whose in-sample best\n               ranks below the median out of sample',
);

/** The preconditions and the default block count the diagnostic reads before it computes. */
export const PBO_PRECONDITIONS = [
  fact(2, GATES, 87, '_PBO_MINIMUM_SURVIVORS = 2'),
  fact(4, GATES, 88, '_PBO_MINIMUM_BLOCKS = 4'),
  fact(8, GATES, 2263, 'blocks: int = 8,'),
  fact(
    'a row with a missing observation is eligible but failed',
    GATES,
    2366,
    'return PboExclusion(row.configuration, PboState.ELIGIBLE_BUT_FAILED, "missing_observations")',
  ),
];

/** The four states the diagnostic distinguishes. */
export const PBO_STATES = [
  fact('ineligible', GATES, 2101, 'INELIGIBLE = "ineligible"'),
  fact('eligible_but_failed', GATES, 2102, 'ELIGIBLE_BUT_FAILED = "eligible_but_failed"'),
  fact(
    'valid_with_documented_exclusions',
    GATES,
    2103,
    'VALID_WITH_EXCLUSIONS = "valid_with_documented_exclusions"',
  ),
  fact('not_computable', GATES, 2104, 'NOT_COMPUTABLE = "not_computable"'),
];

/** The diagnostic rejects nothing: the flag is emitted with every evaluation. */
export const PBO_DIAGNOSTIC_ONLY = fact(true, GATES, 2256, '"diagnostic_only": True,');

/** The counting rule is a definition, not a measurement, and the calibration is not established. */
export const COUNTING_LIMITS = [
  fact(
    'the counting rules are a definition, not a measured effect',
    TUTORIAL,
    179,
    'The counting rules themselves are not a measured effect; they are a definition, and the design states',
  ),
  fact(
    'whether the resulting correction is well calibrated is a property of a live campaign',
    TUTORIAL,
    12,
    'whether the resulting correction is well calibrated is a property of a live campaign, which is exactly why the overfitting probability is reported as a diagnostic and rejects nothing',
  ),
];
