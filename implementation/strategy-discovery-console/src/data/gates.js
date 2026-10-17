/**
 * Transcription: the pre-registered gate chain, its verdict vocabulary and the pre-campaign
 * admission arithmetic.
 *
 * WHY THIS MODULE EXISTS. The chain asks every candidate the same six questions in one order and
 * stops at the first no; the admission assessment compares the sample a proposed search needs with
 * the sample the data can spare, before any backtest runs. Every threshold, gate name, question,
 * verdict, default and refusal message below is transcribed verbatim from `gates.py` (or the page
 * that states it), wrapped in `fact(...)` so tools/check-facts.mjs can prove it is still there.
 *
 * Nothing here computes. The walk, the admission test and the inversion live in
 * `src/engine/gates.js`.
 */

import { fact } from './source.js';

const GATES = 'python/nautilus_trader/discovery/gates.py';
const TUTORIAL = 'tutorials/strategy-discovery/the-admission-gate/README.md';

/** The identity and version of the pre-registered threshold policy every trial records. */
export const GATE_POLICY_IDENTITY = [
  fact('discovery.gates', GATES, 71, 'GATE_POLICY_ID = "discovery.gates"'),
  fact(1, GATES, 72, 'GATE_POLICY_VERSION = 1'),
];

/** The identity of the inverted relation the admission assessment computes. */
export const ADMISSION_VERSION = fact(
  'discovery.admission/v1',
  GATES,
  75,
  'ADMISSION_VERSION = "discovery.admission/v1"',
);

/** The token a gate reports when a declared input is missing, and the search bounds. */
export const GATE_TOKENS = [
  fact('inputs_missing', GATES, 79, '_MISSING = "inputs_missing"'),
  fact(4, GATES, 83, '_MINIMUM_SAMPLE = 4'),
  fact(10000000, GATES, 84, '_MAXIMUM_SAMPLE = 10_000_000'),
  fact(2, GATES, 91, '_MINIMUM_NEIGHBOURS = 2'),
];

/** One threshold per line, in the order the policy declares them, with its default. */
export const GATE_POLICY_THRESHOLDS = [
  fact(0.95, GATES, 197, 'minimum_deflated_sharpe: float = 0.95'),
  fact(0.0, GATES, 198, 'minimum_net_sharpe: float = 0.0'),
  fact(0.0, GATES, 199, 'minimum_baseline_margin: float = 0.0'),
  fact(0.5, GATES, 200, 'maximum_single_name_share: float = 0.5'),
  fact(0.75, GATES, 201, 'maximum_single_sector_share: float = 0.75'),
  fact(0.5, GATES, 202, 'maximum_single_period_share: float = 0.5'),
  fact(0.25, GATES, 203, 'neighbourhood_tolerance: float = 0.25'),
  fact(0.5, GATES, 204, 'minimum_neighbourhood_agreement: float = 0.5'),
  fact(0.5, GATES, 205, 'maximum_drawdown: float = 0.5'),
  fact(0.5, GATES, 206, 'minimum_positive_fold_share: float = 0.5'),
  fact(0.25, GATES, 207, 'maximum_unusable_share: float = 0.25'),
];

/** The gates, in the order they run, each with the question it asks. */
export const GATE_NAMES = [
  fact('G0', GATES, 99, 'G0_DATA_INTEGRITY = "G0"'),
  fact('G1', GATES, 100, 'G1_ECONOMIC_VIABILITY = "G1"'),
  fact('G2', GATES, 101, 'G2_WALK_FORWARD = "G2"'),
  fact('G3', GATES, 102, 'G3_STATISTICAL_CREDIBILITY = "G3"'),
  fact('G4', GATES, 103, 'G4_ROBUSTNESS = "G4"'),
  fact('G5', GATES, 104, 'G5_HOLDOUT_READINESS = "G5"'),
];

/** The question each gate asks, as the design states it. */
export const GATE_QUESTIONS = [
  fact(
    'Was the candidate entitled to what it used, and when?',
    GATES,
    112,
    'GateName.G0_DATA_INTEGRITY: "Was the candidate entitled to what it used, and when?",',
  ),
  fact(
    'Does it survive its costs and beat a baseline?',
    GATES,
    113,
    'GateName.G1_ECONOMIC_VIABILITY: "Does it survive its costs and beat a baseline?",',
  ),
  fact(
    'Does it persist out of sample across time?',
    GATES,
    114,
    'GateName.G2_WALK_FORWARD: "Does it persist out of sample across time?",',
  ),
  fact(
    "Is it distinguishable from this search's best?",
    GATES,
    115,
    'GateName.G3_STATISTICAL_CREDIBILITY: "Is it distinguishable from this search\'s best?",',
  ),
  fact(
    'Does it depend on a lucky name, period, or value?',
    GATES,
    116,
    'GateName.G4_ROBUSTNESS: "Does it depend on a lucky name, period, or value?",',
  ),
  fact(
    'Does it hold on unseen data, and is it frozen?',
    GATES,
    117,
    'GateName.G5_HOLDOUT_READINESS: "Does it hold on unseen data, and is it frozen?",',
  ),
];

/** The fixed verdict vocabulary. Only `passed` advances a candidate. */
export const GATE_VERDICTS = [
  fact('passed', GATES, 133, 'PASSED = "passed"'),
  fact('invalid', GATES, 134, 'INVALID = "invalid"'),
  fact('rejected', GATES, 135, 'REJECTED = "rejected"'),
  fact('inconclusive', GATES, 136, 'INCONCLUSIVE = "inconclusive"'),
  fact('validation_pending', GATES, 137, 'VALIDATION_PENDING = "validation_pending"'),
  fact('quarantined', GATES, 138, 'QUARANTINED = "quarantined"'),
];

/** What the admission assessment returns. */
export const ADMISSION_OUTCOMES = [
  fact('admit', GATES, 1507, 'ADMIT = "admit"'),
  fact('reduce', GATES, 1508, 'REDUCE = "reduce"'),
  fact('reject', GATES, 1509, 'REJECT = "reject"'),
];

/** The declared defaults the admission assessment computes from. */
export const ADMISSION_INPUT_DEFAULTS = [
  fact(0, GATES, 1554, 'sample_observations: int = 0'),
  fact(0, GATES, 1555, 'effective_observations: int = 0'),
  fact(1, GATES, 1556, 'label_horizon: int = 1'),
  fact(0, GATES, 1557, 'planned_configurations: int = 0'),
  fact('independent', GATES, 1558, 'dependence: TrialDependence = TrialDependence.INDEPENDENT'),
  fact(null, GATES, 1559, 'effective_trials: int | None = None'),
  fact(0, GATES, 1560, 'trials_already_consumed: int = 0'),
  fact(0, GATES, 1561, 'reserved_observations: int = 0'),
  fact(0.0, GATES, 1562, 'holdout_fraction: float = 0.0'),
  fact(0.0, GATES, 1563, 'target_sharpe: float = 0.0'),
  fact(0.95, GATES, 1564, 'significance: float = 0.95'),
  fact(0.8, GATES, 1565, 'power: float = 0.8'),
  fact(null, GATES, 1566, 'cross_trial_dispersion: float | None = None'),
  fact(0, GATES, 1567, 'complexity_parameters: int = 0'),
];

/** The assumptions the inverted relation is computed under. */
export const TRACKING_ASSUMPTIONS_DEFAULTS = [
  fact(0.0, GATES, 1737, 'target_sharpe: float = 0.0'),
  fact(0.0, GATES, 1738, 'cross_trial_dispersion: float = 0.0'),
  fact(0.95, GATES, 1739, 'significance: float = 0.95'),
  fact(0.0, GATES, 1740, 'skew: float = 0.0'),
  fact(3.0, GATES, 1741, 'kurtosis: float = 3.0'),
];

/** The threshold values a policy must state, and the refusals when it does not. */
export const POLICY_REFUSALS = [
  fact(
    'a name or sector share must be in (0, 1]',
    GATES,
    219,
    'raise ValueError(f"{name} must be a share in (0, 1], was {value}")',
  ),
  fact(
    'a period share, agreement share, fold share or unusable share must be in [0, 1]',
    GATES,
    228,
    'raise ValueError(f"{name} must be a share in [0, 1], was {value}")',
  ),
  fact(
    'the deflated Sharpe floor must be in (0, 1)',
    GATES,
    231,
    'f"minimum_deflated_sharpe must be in (0, 1), was {self.minimum_deflated_sharpe}"',
  ),
  fact(
    'the drawdown ceiling must be positive',
    GATES,
    234,
    'raise ValueError(f"maximum_drawdown must be positive, was {self.maximum_drawdown}")',
  ),
];

/** Why the assessment refuses to estimate rather than defaulting. */
export const ADMISSION_REFUSALS = [
  fact(
    'the configurations are dependent and no effective trial count is estimated',
    GATES,
    2035,
    'return "the configurations are dependent and no effective trial count is estimated"',
  ),
  fact(
    'the effective number of independent observations is not estimated',
    GATES,
    2037,
    'return "the effective number of independent observations is not estimated"',
  ),
  fact(
    'the cross-trial dispersion of the Sharpe estimates is not estimated',
    GATES,
    2039,
    'return "the cross-trial dispersion of the Sharpe estimates is not estimated"',
  ),
  fact(
    'the campaign proposes no configuration, so there is nothing to assess',
    GATES,
    2043,
    'return "the campaign proposes no configuration, so there is nothing to assess"',
  ),
  fact(
    'no sample distinguishes the declared target Sharpe from what the trials would produce by chance',
    GATES,
    1964,
    'f"no sample distinguishes the declared target Sharpe {inputs.target_sharpe} from what "',
  ),
  fact(
    'no allocation fits: one configuration needs more than the usable observations',
    GATES,
    2014,
    'reason = f"no allocation fits: one configuration needs {ceiling:.0f} against {usable} usable"',
  ),
];

/** The admission arithmetic, written out with every symbol named. */
export const ADMISSION_FORMULAE = [
  fact('U = max(0, E - (R + round(E * h)))', TUTORIAL, 97, 'U = max(0, E - (R + round(E * h)))'),
  fact(
    'K = P + A when the configurations are independent draws\nK = X + A when they are dependent',
    TUTORIAL,
    111,
    'K = P + A            when the configurations are independent draws\nK = X + A            when they are dependent',
  ),
  fact(
    'T*(K) = the smallest sample length T at which the deflated Sharpe ratio of the target Sharpe reaches the declared significance s',
    TUTORIAL,
    123,
    'T*(K) = the smallest sample length T at which the deflated Sharpe\n        ratio of the target Sharpe reaches the declared significance s',
  ),
  fact(
    'if T*(K) <= U -> ADMIT else if some c < P has T*(c) <= U -> REDUCE to c configurations else -> REJECT',
    TUTORIAL,
    135,
    'if T*(K) <= U                      -> ADMIT\nelse if some c < P has T*(c) <= U  -> REDUCE to c configurations\nelse                               -> REJECT',
  ),
];

/** The one worked example the tutorial states, with its numbers, asserted by the tests. */
export const ADMISSION_WORKED_EXAMPLE = [
  fact(240, TUTORIAL, 168, '= 240'),
  fact(45, TUTORIAL, 169, 'K = 45 + 0 = 45'),
  fact(248, TUTORIAL, 175, 'T*(45) = 248 > 240   -> the plan does not fit'),
  fact(36, TUTORIAL, 179, 'the first count whose ceiling is at or below 240 is 36'),
  fact(234, TUTORIAL, 181, 'would be `T*(30) = 234`, which fits, and the outcome would be `ADMIT`.'),
];

/** The limits the page states on the admission relation. */
export const ADMISSION_LIMITS = [
  fact(
    'the relation is a significance relation, not a power calculation',
    TUTORIAL,
    239,
    'The relation is a significance relation, not a power calculation; the declared power (`0.8` by default)',
  ),
  fact(
    'the 45-configuration example is a conditional reference case, not a universal cap',
    TUTORIAL,
    194,
    'worked example states that with five years of data no more than 45 independent configurations should be',
  ),
];

/** The convergence threshold the inversion is computed to, and the minimum sample it starts at. */
export const ADMISSION_INVERSION_BOUNDS = [
  fact(
    'the search refuses a trial count below one or a non-positive dispersion',
    GATES,
    1795,
    'if counts_trials < 1 or sharpes.cross_trial_dispersion <= 0.0:',
  ),
  fact(
    'the search refuses a significance outside (0, 1)',
    GATES,
    1797,
    'if not 0.0 < sharpes.significance < 1.0:',
  ),
];
