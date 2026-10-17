/**
 * The search slice's genotype, operators and declaration table, transcribed.
 *
 * The search treats a candidate specification as a genome, so this module transcribes two things.
 * From genome.py: the gene tables that fix which positions the genotype has, the three degrees of
 * freedom a change can carry, the operators' draw parameters and every refusal the operators
 * report. From evolve.py: the search policy's operator parameters, the seven objectives with the
 * sampling convention each name carries, the frozen objects, the equivalence rule's ignored paths
 * and the search's own refusal vocabulary.
 *
 * The declared sets and bounds come from policy.py, which is the document a gene's value resolves
 * through: without them the genotype cannot be drawn or mutated at all, so the console transcribes
 * the first-release table it needs rather than reaching into another slice's module. A policy
 * object of the same shape - `{ sets, bounds }`, as src/engine/policy.js returns - can be passed to
 * the engine instead.
 */

import { fact, valueOf } from './source.js';

const GENOME_PY = 'python/nautilus_trader/discovery/genome.py';
const EVOLVE_PY = 'python/nautilus_trader/discovery/evolve.py';
const POLICY_PY = 'python/nautilus_trader/discovery/policy.py';
const SCHEMA_PY = 'python/nautilus_trader/discovery/schema.py';
const AGENTS_TUTORIAL = 'tutorials/strategy-discovery/agents-that-propose/README.md';
const TUTORIAL = 'tutorials/strategy-discovery/search-by-evolution/README.md';

/** The genotype's fixed positions: what the search may move, and how each position is named. */
export const GENOME = {
  version: fact('discovery.genome/v1', GENOME_PY, 69, 'GENOME_VERSION = "discovery.genome/v1"'),
  geneValueKinds: fact(
    ['Decimal', 'int', 'str', 'bool'],
    GENOME_PY,
    75,
    'type GeneValue = Decimal | int | str | bool',
  ),
  mutationPlaces: fact(8, GENOME_PY, 80, '_MUTATION_PLACES = 8'),
  mutationAttempts: fact(8, GENOME_PY, 81, '_MUTATION_ATTEMPTS = 8'),
  conditionBound: fact('condition.value', GENOME_PY, 130, '_CONDITION_BOUND = "condition.value"'),
  statementFields: fact(
    ['family', 'mechanism', 'hypothesis', 'falsification'],
    GENOME_PY,
    132,
    '_STATEMENT_FIELDS: tuple[str, ...] = ("family", "mechanism", "hypothesis", "falsification")',
  ),
  mechanismPaths: fact(
    ['identity.family', 'identity.mechanism', 'identity.hypothesis', 'identity.falsification'],
    GENOME_PY,
    136,
    '_MECHANISM_PATHS: tuple[str, ...] = tuple(f"identity.{name}" for name in _STATEMENT_FIELDS)',
  ),
};

/**
 * Every structural gene, as (specification path, policy declared-set name), in the source's order.
 * A gene whose declared-set name is empty takes a boolean.
 */
export const STRUCTURAL_GENES = fact(
  [
    ['universe.membership_policy', 'membership_policies'],
    ['data.signal_frequency', 'signal_frequencies'],
    ['data.decision_time', 'decision_times'],
    ['data.execution_eligibility', 'execution_eligibilities'],
    ['data.corporate_action_treatment', 'corporate_action_treatments'],
    ['selection.mode', 'selection_modes'],
    ['selection.comparison', 'operators'],
    ['selection.tie_break', 'tie_breaks'],
    ['entry.direction', 'entry_directions'],
    ['exit.comparison', 'operators'],
    ['exit.flatten_ineligible', ''],
    ['portfolio.weighting', 'weighting_methods'],
    ['portfolio.cash_policy', 'cash_policies'],
    ['rebalance.frequency', 'rebalance_frequencies'],
    ['rebalance.timing', 'rebalance_timings'],
    ['risk.runtime.halt_new_entries', ''],
    ['risk.runtime.flatten_ineligible', ''],
  ],
  GENOME_PY,
  85,
  `_STRUCTURAL_GENES: tuple[tuple[str, str], ...] = (
    ("universe.membership_policy", "membership_policies"),
    ("data.signal_frequency", "signal_frequencies"),
    ("data.decision_time", "decision_times"),
    ("data.execution_eligibility", "execution_eligibilities"),
    ("data.corporate_action_treatment", "corporate_action_treatments"),
    ("selection.mode", "selection_modes"),
    ("selection.comparison", "operators"),
    ("selection.tie_break", "tie_breaks"),
    ("entry.direction", "entry_directions"),
    ("exit.comparison", "operators"),
    ("exit.flatten_ineligible", ""),
    ("portfolio.weighting", "weighting_methods"),
    ("portfolio.cash_policy", "cash_policies"),
    ("rebalance.frequency", "rebalance_frequencies"),
    ("rebalance.timing", "rebalance_timings"),
    ("risk.runtime.halt_new_entries", ""),
    ("risk.runtime.flatten_ineligible", ""),
)`,
);

/**
 * Every parameter gene, as (policy bound name, specification path), in the source's order. The
 * `{index}` placeholder expands once per element of the block its path names.
 */
export const PARAMETER_GENES = fact(
  [
    ['feature.lookback', 'features.declarations[{index}].lookback'],
    ['feature.skip', 'features.declarations[{index}].skip'],
    ['selection.percentile', 'selection.percentile'],
    ['selection.count', 'selection.count'],
    ['selection.threshold', 'selection.threshold'],
    ['exit.holding_period_cap', 'exit.holding_period_cap'],
    ['exit.rank_exit_count', 'exit.rank_exit_count'],
    ['exit.stop_fraction', 'exit.stop_fraction'],
    ['exit.target_fraction', 'exit.target_fraction'],
    ['portfolio.position_count', 'portfolio.position_count'],
    ['portfolio.max_position_weight', 'portfolio.max_position_weight'],
    ['portfolio.max_sector_weight', 'portfolio.max_sector_weight'],
    ['risk.max_gross_exposure', 'risk.pre_trade.max_gross_exposure'],
    ['risk.max_drawdown', 'risk.post_run.max_drawdown'],
    ['risk.max_turnover', 'risk.post_run.max_turnover'],
    ['risk.max_cost_ratio', 'risk.post_run.max_cost_ratio'],
    ['universe.min_price', 'universe.min_price'],
    ['universe.min_dollar_volume', 'universe.min_dollar_volume'],
  ],
  GENOME_PY,
  107,
  `_PARAMETER_GENES: tuple[tuple[str, str], ...] = (
    ("feature.lookback", "features.declarations[{index}].lookback"),
    ("feature.skip", "features.declarations[{index}].skip"),
    ("selection.percentile", "selection.percentile"),
    ("selection.count", "selection.count"),
    ("selection.threshold", "selection.threshold"),
    ("exit.holding_period_cap", "exit.holding_period_cap"),
    ("exit.rank_exit_count", "exit.rank_exit_count"),
    ("exit.stop_fraction", "exit.stop_fraction"),
    ("exit.target_fraction", "exit.target_fraction"),
    ("portfolio.position_count", "portfolio.position_count"),
    ("portfolio.max_position_weight", "portfolio.max_position_weight"),
    ("portfolio.max_sector_weight", "portfolio.max_sector_weight"),
    ("risk.max_gross_exposure", "risk.pre_trade.max_gross_exposure"),
    ("risk.max_drawdown", "risk.post_run.max_drawdown"),
    ("risk.max_turnover", "risk.post_run.max_turnover"),
    ("risk.max_cost_ratio", "risk.post_run.max_cost_ratio"),
    ("universe.min_price", "universe.min_price"),
    ("universe.min_dollar_volume", "universe.min_dollar_volume"),
)`,
);

/** The three degrees of freedom, in the source's order. */
export const DEGREES = [
  fact('parameter', GENOME_PY, 148, 'PARAMETER = "parameter"'),
  fact('structural', GENOME_PY, 149, 'STRUCTURAL = "structural"'),
  fact('family', GENOME_PY, 150, 'FAMILY = "family"'),
];

/** The property that makes the family degree the one that needs a replacement statement. */
export const REQUIRES_NEW_HYPOTHESIS = fact(
  'family',
  GENOME_PY,
  162,
  'return self is GeneticDegree.FAMILY',
);

/**
 * The bounded fields the schema reads as integers, which is what makes a gene draw an integer
 * rather than a decimal. Every other parameter gene keeps the decimal type its field declares.
 */
export const INTEGER_BOUNDS = [
  fact('feature.lookback', SCHEMA_PY, 1366, 'lookback = entry.bounded_int("lookback", "feature.lookback")'),
  fact(
    'feature.skip',
    SCHEMA_PY,
    1367,
    'skip = entry.bounded_int("skip", "feature.skip", required=False)',
  ),
  fact(
    'exit.holding_period_cap',
    SCHEMA_PY,
    1560,
    `holding_period_cap=reader.bounded_int(
            "holding_period_cap",`,
  ),
  fact(
    'exit.rank_exit_count',
    SCHEMA_PY,
    1565,
    `rank_exit_count=reader.bounded_int(`,
  ),
  fact(
    'portfolio.position_count',
    SCHEMA_PY,
    1581,
    'position_count=reader.bounded_int("position_count", "portfolio.position_count") or 0',
  ),
];

/** The five codes a genotype-level refusal is reported with, in the source's order. */
export const GENOME_REFUSAL_CODES = [
  fact('gene_unresolved', GENOME_PY, 174, 'GENE_UNRESOLVED = "gene_unresolved"'),
  fact('gene_not_mutable', GENOME_PY, 175, 'GENE_NOT_MUTABLE = "gene_not_mutable"'),
  fact('family_mismatch', GENOME_PY, 176, 'FAMILY_MISMATCH = "family_mismatch"'),
  fact('value_not_declared', GENOME_PY, 177, 'VALUE_NOT_DECLARED = "value_not_declared"'),
  fact('no_free_gene', GENOME_PY, 178, 'NO_FREE_GENE = "no_free_gene"'),
];

/** The eight codes a search-level refusal is reported with, in the source's order. */
export const SEARCH_REFUSAL_CODES = [
  fact(
    'locked_result_in_selection',
    EVOLVE_PY,
    201,
    'LOCKED_RESULT_IN_SELECTION = "locked_result_in_selection"',
  ),
  fact('role_mismatch', EVOLVE_PY, 202, 'ROLE_MISMATCH = "role_mismatch"'),
  fact('unknown_objective', EVOLVE_PY, 203, 'UNKNOWN_OBJECTIVE = "unknown_objective"'),
  fact('unknown_candidate', EVOLVE_PY, 204, 'UNKNOWN_CANDIDATE = "unknown_candidate"'),
  fact('missing_objective', EVOLVE_PY, 205, 'MISSING_OBJECTIVE = "missing_objective"'),
  fact('population_empty', EVOLVE_PY, 206, 'POPULATION_EMPTY = "population_empty"'),
  fact('edit_refused', EVOLVE_PY, 207, 'EDIT_REFUSED = "edit_refused"'),
  fact('frozen_field', EVOLVE_PY, 208, 'FROZEN_FIELD = "frozen_field"'),
];

/** Every genotype-level refusal, with the message the operator reports it with. */
export const GENOME_REFUSALS = {
  notAGene: fact(
    { code: 'gene_unresolved', message: '{path} is not a gene' },
    GENOME_PY,
    726,
    'return _refused(GenomeRefusal.GENE_UNRESOLVED, f"{path} is not a gene")',
  ),
  noGeneNamed: fact(
    { code: 'gene_unresolved', message: 'no gene was named' },
    GENOME_PY,
    737,
    'return _refused(GenomeRefusal.GENE_UNRESOLVED, "no gene was named")',
  ),
  statementPath: fact(
    {
      code: 'gene_not_mutable',
      message: '{path} replaces the mechanism, which needs a replacement statement',
    },
    GENOME_PY,
    724,
    'f"{path} replaces the mechanism, which needs a replacement statement",',
  ),
  noOtherValue: fact(
    { code: 'no_free_gene', message: '{path} has no other admissible value' },
    GENOME_PY,
    733,
    'return _refused(GenomeRefusal.NO_FREE_GENE, f"{path} has no other admissible value")',
  ),
  statementMissing: fact(
    { code: 'value_not_declared', message: 'the replacement statement is missing {names}' },
    GENOME_PY,
    786,
    'f"the replacement statement is missing {\', \'.join(missing)}",',
  ),
  familyNotDeclared: fact(
    { code: 'value_not_declared', message: '{family} is not a declared family' },
    GENOME_PY,
    791,
    'f"{mechanism.family} is not a declared family",',
  ),
  sameMechanism: fact(
    {
      code: 'no_free_gene',
      message: 'the replacement mechanism is the one the genome already carries',
    },
    GENOME_PY,
    809,
    '"the replacement mechanism is the one the genome already carries",',
  ),
  differentFamilies: fact(
    { code: 'family_mismatch', message: '{first} and {second} are different families' },
    GENOME_PY,
    859,
    'f"{first.family} and {second.family} are different families",',
  ),
  noSharedDifference: fact(
    { code: 'no_free_gene', message: 'the parents share no admissible difference' },
    GENOME_PY,
    872,
    'return _refused(GenomeRefusal.NO_FREE_GENE, "the parents share no admissible difference")',
  ),
  outOfBound: fact(
    { code: 'value_out_of_bound', message: '{path} is outside its bound {bound}' },
    GENOME_PY,
    444,
    'raise ValueError(f"{gene.path} is outside its bound {gene.bound.describe()}")',
  ),
  decimalDrawExhausted: fact(
    { code: 'draw_exhausted', message: 'no value inside {bound} was drawn' },
    GENOME_PY,
    957,
    'raise ValueError(f"no value inside {bound.describe()} was drawn")',
  ),
  noIntegerInBound: fact(
    { code: 'draw_exhausted', message: 'no integer sits inside {bound}' },
    GENOME_PY,
    965,
    'raise ValueError(f"no integer sits inside {bound.describe()}")',
  ),
  integerDrawExhausted: fact(
    { code: 'draw_exhausted', message: 'no integer inside {bound} was drawn' },
    GENOME_PY,
    972,
    'raise ValueError(f"no integer inside {bound.describe()} was drawn")',
  ),
};

/** The search policy's own constants: the stamp, the operator parameters and the frozen fields. */
export const SEARCH = {
  version: fact('discovery.evolve/v1', EVOLVE_PY, 77, 'EVOLUTION_VERSION = "discovery.evolve/v1"'),
  policyId: fact('discovery.search_policy', EVOLVE_PY, 78, 'SEARCH_POLICY_ID = "discovery.search_policy"'),
  policyVersion: fact(1, EVOLVE_PY, 79, 'SEARCH_POLICY_VERSION = 1'),
  equivalenceRuleId: fact(
    'discovery.equivalence/v1',
    EVOLVE_PY,
    80,
    'EQUIVALENCE_RULE_ID = "discovery.equivalence/v1"',
  ),
  generationStamp: fact('generation', EVOLVE_PY, 84, '_GENERATION_STAMP = "generation"'),
  familyAllocation: fact({}, EVOLVE_PY, 463, 'family_allocation: Mapping[str, float] = field(default_factory=dict)'),
  generation: fact(0, EVOLVE_PY, 464, 'generation: int = 0'),
  parameterGenesPerMutation: fact(2, EVOLVE_PY, 88, '_PARAMETER_GENES_PER_MUTATION = 2'),
  structuralGenesPerMutation: fact(1, EVOLVE_PY, 89, '_STRUCTURAL_GENES_PER_MUTATION = 1'),
  explorationShare: fact(0.25, EVOLVE_PY, 92, '_EXPLORATION_SHARE = 0.25'),
  editAttempts: fact(8, EVOLVE_PY, 96, '_EDIT_ATTEMPTS = 8'),
  mutationRate: fact(0.5, EVOLVE_PY, 461, 'mutation_rate: float = 0.5'),
  populationSize: fact(12, EVOLVE_PY, 1109, 'population_size: int = 12'),
  elite: fact(2, EVOLVE_PY, 1110, 'elite: int = 2'),
  familyFloor: fact(2, EVOLVE_PY, 1111, 'family_floor: int = 2'),
  mutableFields: fact(
    ['prompts', 'mutation_rate', 'exploration_share', 'family_allocation'],
    EVOLVE_PY,
    492,
    'return ("prompts", "mutation_rate", "exploration_share", "family_allocation")',
  ),
};

/** The seven objectives, in the source's order, each with its own sampling convention. */
export const OBJECTIVE_NAMES = [
  fact('risk_adjusted_return', EVOLVE_PY, 108, 'RISK_ADJUSTED_RETURN = "risk_adjusted_return"'),
  fact('return_dispersion', EVOLVE_PY, 109, 'RETURN_DISPERSION = "return_dispersion"'),
  fact('drawdown', EVOLVE_PY, 110, 'DRAWDOWN = "drawdown"'),
  fact('tail_loss', EVOLVE_PY, 111, 'TAIL_LOSS = "tail_loss"'),
  fact('relative_return', EVOLVE_PY, 112, 'RELATIVE_RETURN = "relative_return"'),
  fact('turnover', EVOLVE_PY, 113, 'TURNOVER = "turnover"'),
  fact('complexity', EVOLVE_PY, 114, 'COMPLEXITY = "complexity"'),
];

/** The convention each objective is measured under, quoted from the mapping as written. */
export const OBJECTIVE_CONVENTIONS = {
  risk_adjusted_return: fact(
    "the per-period Sharpe ratio of the development folds' return series at the declared signal frequency",
    EVOLVE_PY,
    142,
    `Objective.RISK_ADJUSTED_RETURN: (
        "the per-period Sharpe ratio of the development folds' return series at the declared "
        "signal frequency"
    ),`,
  ),
  return_dispersion: fact(
    'the standard deviation of the per-fold risk-adjusted return across the development folds',
    EVOLVE_PY,
    146,
    `Objective.RETURN_DISPERSION: (
        "the standard deviation of the per-fold risk-adjusted return across the development folds"
    ),`,
  ),
  drawdown: fact(
    'the maximum drawdown of the development window',
    EVOLVE_PY,
    149,
    'Objective.DRAWDOWN: "the maximum drawdown of the development window",',
  ),
  tail_loss: fact(
    'the fifth percentile of the development per-period returns',
    EVOLVE_PY,
    150,
    'Objective.TAIL_LOSS: "the fifth percentile of the development per-period returns",',
  ),
  relative_return: fact(
    "the development return less the registered benchmark's over the same window",
    EVOLVE_PY,
    151,
    `Objective.RELATIVE_RETURN: (
        "the development return less the registered benchmark's over the same window"
    ),`,
  ),
  turnover: fact(
    "turnover under the policy's declared cost model",
    EVOLVE_PY,
    154,
    'Objective.TURNOVER: "turnover under the policy\'s declared cost model",',
  ),
  complexity: fact(
    'the condition-leaf count plus the declared parameter count',
    EVOLVE_PY,
    155,
    'Objective.COMPLEXITY: "the condition-leaf count plus the declared parameter count",',
  ),
};

/** The three objectives where a larger value is preferable. */
export const PREFERS_HIGH = fact(
  ['risk_adjusted_return', 'tail_loss', 'relative_return'],
  EVOLVE_PY,
  158,
  `_PREFERS_HIGH: frozenset[Objective] = frozenset(
    {
        Objective.RISK_ADJUSTED_RETURN,
        Objective.TAIL_LOSS,
        Objective.RELATIVE_RETURN,
    },
)`,
);

/** The design's frozen objects, each with the reason it is frozen. */
export const FROZEN_FIELDS = fact(
  [
    ['feature_allowlist', 'the allowed feature set is frozen before a campaign opens'],
    ['bounds', 'a parameter bound is the space the search moves inside, not part of the search'],
    ['evaluation_window', 'the evaluation window is fixed and versioned before a campaign runs'],
    ['split_layout', 'the split layout and its purge and embargo values are frozen'],
    ['cost_model', 'the cost model and its parameters are frozen'],
    ['benchmark', 'the benchmark is frozen'],
    ['risk_policy', 'the risk policy and its limits are frozen'],
    ['confirmation_allocation', 'the allocation of the confirmation set is frozen'],
  ],
  EVOLVE_PY,
  168,
  `FROZEN_FIELDS: tuple[tuple[str, str], ...] = (
    ("feature_allowlist", "the allowed feature set is frozen before a campaign opens"),
    ("bounds", "a parameter bound is the space the search moves inside, not part of the search"),
    ("evaluation_window", "the evaluation window is fixed and versioned before a campaign runs"),
    ("split_layout", "the split layout and its purge and embargo values are frozen"),
    ("cost_model", "the cost model and its parameters are frozen"),
    ("benchmark", "the benchmark is frozen"),
    ("risk_policy", "the risk policy and its limits are frozen"),
    ("confirmation_allocation", "the allocation of the confirmation set is frozen"),
)`,
);

/** The five specification paths the equivalence rule removes before it digests a candidate. */
export const EQUIVALENCE_IGNORED_PATHS = fact(
  ['identity.hypothesis', 'identity.falsification', 'provenance.parents', 'provenance.generator', 'provenance.seed'],
  EVOLVE_PY,
  182,
  `EQUIVALENCE_IGNORED_PATHS: tuple[str, ...] = (
    "identity.hypothesis",
    "identity.falsification",
    "provenance.parents",
    "provenance.generator",
    "provenance.seed",
)`,
);

/** Every search-level refusal, with the reason the search reports it with. */
export const SEARCH_REFUSALS = {
  lockedResult: fact(
    {
      code: 'locked_result_in_selection',
      message: '{named} carries {role} data, and selection reads development results only',
    },
    EVOLVE_PY,
    850,
    `f"{named} carries {locked[0].role.value} data, and selection reads development "
                "results only"`,
  ),
  populationEmpty: fact(
    { code: 'population_empty', message: 'population_empty' },
    EVOLVE_PY,
    1173,
    'SearchRefusal.POPULATION_EMPTY,',
  ),
  unknownCandidate: fact(
    { code: 'unknown_candidate', message: 'the front names candidates the population does not hold' },
    EVOLVE_PY,
    1194,
    '"the front names candidates the population does not hold"',
  ),
  missingObjective: fact(
    {
      code: 'missing_objective',
      message: 'no candidate produced a complete set of development objectives',
    },
    EVOLVE_PY,
    1196,
    'else "no candidate produced a complete set of development objectives",',
  ),
  editRefused: fact(
    {
      code: 'edit_refused',
      message: 'every child the operators produced was already in the population',
    },
    EVOLVE_PY,
    1224,
    'else "every child the operators produced was already in the population"',
  ),
  frozenField: fact(
    { code: 'frozen_field', message: '{name} is frozen: {why}' },
    EVOLVE_PY,
    569,
    'reason=f"{name} is frozen: {frozen[name]}",',
  ),
  undeclaredField: fact(
    { code: 'frozen_field', message: "{name} is not part of the search policy's mutable set" },
    EVOLVE_PY,
    574,
    'reason=f"{name} is not part of the search policy\'s mutable set",',
  ),
  notAShare: fact(
    { code: 'not_a_share', message: '{name} must be a share, was {share}' },
    EVOLVE_PY,
    477,
    'raise ValueError(f"{name} must be a share, was {share}")',
  ),
  negativeAllocation: fact(
    { code: 'negative_allocation', message: 'the allocation for {family} must not be negative' },
    EVOLVE_PY,
    480,
    'raise ValueError(f"the allocation for {family} must not be negative")',
  ),
  countTooSmall: fact(
    { code: 'count_too_small', message: '{name} must be at least one, was {value}' },
    EVOLVE_PY,
    1123,
    'raise ValueError(f"{name} must be at least one, was {value}")',
  ),
};

/** The first-release declared sets a structural gene selects from, in the policy's own order. */
export const DECLARED_SETS = [
  fact(
    ['cross_sectional_rank', 'absolute_threshold', 'static_hold'],
    POLICY_PY,
    526,
    'families=frozenset({"cross_sectional_rank", "absolute_threshold", "static_hold"}),',
  ),
  fact(['EQUITY', 'ETF'], POLICY_PY, 527, 'asset_classes=frozenset({"EQUITY", "ETF"}),'),
  fact(['point_in_time'], POLICY_PY, 528, 'membership_policies=frozenset({"point_in_time"}),'),
  fact(['daily'], POLICY_PY, 529, 'signal_frequencies=frozenset({"daily"}),'),
  fact(
    ['session_close', 'session_open'],
    POLICY_PY,
    530,
    'decision_times=frozenset({"session_close", "session_open"}),',
  ),
  fact(
    ['same_session_close', 'next_session_open'],
    POLICY_PY,
    531,
    'execution_eligibilities=frozenset({"same_session_close", "next_session_open"}),',
  ),
  fact(
    ['adjusted', 'raw'],
    POLICY_PY,
    532,
    'corporate_action_treatments=frozenset({"adjusted", "raw"}),',
  ),
  fact(['long'], POLICY_PY, 533, 'entry_directions=frozenset({"long"}),'),
  fact(['gt', 'ge', 'lt', 'le'], POLICY_PY, 534, 'operators=frozenset({"gt", "ge", "lt", "le"}),'),
  fact(
    ['rank', 'threshold', 'none'],
    POLICY_PY,
    535,
    'selection_modes=frozenset({"rank", "threshold", "none"}),',
  ),
  fact(
    ['feature_desc', 'instrument_id_asc'],
    POLICY_PY,
    536,
    'tie_breaks=frozenset({"feature_desc", "instrument_id_asc"}),',
  ),
  fact(
    ['equal', 'rank_proportional'],
    POLICY_PY,
    537,
    'weighting_methods=frozenset({"equal", "rank_proportional"}),',
  ),
  fact(['leave_idle', 'reallocate'], POLICY_PY, 538, 'cash_policies=frozenset({"leave_idle", "reallocate"}),'),
  fact(
    ['daily', 'weekly', 'monthly'],
    POLICY_PY,
    539,
    'rebalance_frequencies=frozenset({"daily", "weekly", "monthly"}),',
  ),
  fact(
    ['next_session_open'],
    POLICY_PY,
    540,
    'rebalance_timings=frozenset({"next_session_open"}),',
  ),
];

/**
 * The first-release bounds, in the policy's own order, as { name, kind, lower, upper }. Every
 * declared first-release bound is a range bound, so no choice bound appears here.
 */
export const BOUNDS = [
  fact({ name: 'feature.lookback', kind: 'range', lower: 1, upper: 252 }, POLICY_PY, 552, 'Bound("feature.lookback", BoundKind.RANGE, lower=Decimal(1), upper=Decimal(252)),'),
  fact({ name: 'feature.skip', kind: 'range', lower: 0, upper: 5 }, POLICY_PY, 553, 'Bound("feature.skip", BoundKind.RANGE, lower=Decimal(0), upper=Decimal(5)),'),
  fact({ name: 'selection.percentile', kind: 'range', lower: 0.01, upper: 1 }, POLICY_PY, 554, 'Bound("selection.percentile", BoundKind.RANGE, lower=Decimal("0.01"), upper=Decimal(1)),'),
  fact({ name: 'selection.count', kind: 'range', lower: 1, upper: 50 }, POLICY_PY, 555, 'Bound("selection.count", BoundKind.RANGE, lower=Decimal(1), upper=Decimal(50)),'),
  fact(
    { name: 'selection.threshold', kind: 'range', lower: -1000000, upper: 1000000 },
    POLICY_PY,
    556,
    `Bound(
        "selection.threshold",
        BoundKind.RANGE,
        lower=Decimal(-1_000_000),
        upper=Decimal(1_000_000),
    ),`,
  ),
  fact({ name: 'condition.value', kind: 'range', lower: -1000000, upper: 1000000 }, POLICY_PY, 562, 'Bound("condition.value", BoundKind.RANGE, lower=Decimal(-1_000_000), upper=Decimal(1_000_000)),'),
  fact({ name: 'exit.holding_period_cap', kind: 'range', lower: 1, upper: 252 }, POLICY_PY, 563, 'Bound("exit.holding_period_cap", BoundKind.RANGE, lower=Decimal(1), upper=Decimal(252)),'),
  fact({ name: 'exit.rank_exit_count', kind: 'range', lower: 1, upper: 50 }, POLICY_PY, 564, 'Bound("exit.rank_exit_count", BoundKind.RANGE, lower=Decimal(1), upper=Decimal(50)),'),
  fact({ name: 'exit.stop_fraction', kind: 'range', lower: 0.001, upper: 0.5 }, POLICY_PY, 565, 'Bound("exit.stop_fraction", BoundKind.RANGE, lower=Decimal("0.001"), upper=Decimal("0.5")),'),
  fact({ name: 'exit.target_fraction', kind: 'range', lower: 0.001, upper: 5 }, POLICY_PY, 566, 'Bound("exit.target_fraction", BoundKind.RANGE, lower=Decimal("0.001"), upper=Decimal(5)),'),
  fact({ name: 'portfolio.position_count', kind: 'range', lower: 1, upper: 50 }, POLICY_PY, 567, 'Bound("portfolio.position_count", BoundKind.RANGE, lower=Decimal(1), upper=Decimal(50)),'),
  fact(
    { name: 'portfolio.max_position_weight', kind: 'range', lower: 0.005, upper: 0.25 },
    POLICY_PY,
    568,
    `Bound(
        "portfolio.max_position_weight",
        BoundKind.RANGE,
        lower=Decimal("0.005"),
        upper=Decimal("0.25"),
    ),`,
  ),
  fact({ name: 'portfolio.max_sector_weight', kind: 'range', lower: 0.1, upper: 1 }, POLICY_PY, 574, 'Bound("portfolio.max_sector_weight", BoundKind.RANGE, lower=Decimal("0.1"), upper=Decimal(1)),'),
  fact({ name: 'risk.max_gross_exposure', kind: 'range', lower: 0.1, upper: 2 }, POLICY_PY, 575, 'Bound("risk.max_gross_exposure", BoundKind.RANGE, lower=Decimal("0.1"), upper=Decimal(2)),'),
  fact({ name: 'risk.max_drawdown', kind: 'range', lower: 0.01, upper: 0.9 }, POLICY_PY, 576, 'Bound("risk.max_drawdown", BoundKind.RANGE, lower=Decimal("0.01"), upper=Decimal("0.9")),'),
  fact({ name: 'risk.max_turnover', kind: 'range', lower: 0.05, upper: 10 }, POLICY_PY, 577, 'Bound("risk.max_turnover", BoundKind.RANGE, lower=Decimal("0.05"), upper=Decimal(10)),'),
  fact({ name: 'risk.max_cost_ratio', kind: 'range', lower: 0.0001, upper: 0.1 }, POLICY_PY, 578, 'Bound("risk.max_cost_ratio", BoundKind.RANGE, lower=Decimal("0.0001"), upper=Decimal("0.1")),'),
  fact({ name: 'universe.min_price', kind: 'range', lower: 1, upper: 1000 }, POLICY_PY, 579, 'Bound("universe.min_price", BoundKind.RANGE, lower=Decimal(1), upper=Decimal(1000)),'),
  fact(
    { name: 'universe.min_dollar_volume', kind: 'range', lower: 0, upper: 10000000000 },
    POLICY_PY,
    580,
    `Bound(
        "universe.min_dollar_volume",
        BoundKind.RANGE,
        lower=Decimal(0),
        upper=Decimal(10_000_000_000),
    ),`,
  ),
];

/** The complexity caps a candidate's expression is held inside, which limit the demo's shape. */
export const COMPLEXITY_LIMITS = fact(
  { maxConditions: 4, maxConditionDepth: 2, maxFeatures: 8 },
  POLICY_PY,
  113,
  `max_conditions: int = 4
    max_condition_depth: int = 2
    max_features: int = 8`,
);

/**
 * The page's paper exercise: four columns and five made-up candidates, which is the whole of what
 * the exercise needs. The console reads the return column as the relative return and the other two
 * by their own names, and the reader may change every number, because the search measures nothing.
 */
export const PAPER_EXERCISE = {
  columns: fact(
    [
      { key: 'return', objective: 'relative_return', label: 'return (a percentage, higher is better)' },
      { key: 'turnover', objective: 'turnover', label: 'turnover (a percentage, lower is better)' },
      { key: 'drawdown', objective: 'drawdown', label: 'drawdown (a percentage, lower is better)' },
    ],
    TUTORIAL,
    259,
    '1. Rule a table with four columns: candidate, return (a percentage, higher is better), turnover (a\n   percentage, lower is better), and drawdown (a percentage, lower is better).',
  ),
  rows: fact(
    [
      { return: 4, turnover: 20, drawdown: 10 },
      { return: 6, turnover: 80, drawdown: 25 },
      { return: 5, turnover: 30, drawdown: 12 },
      { return: 3, turnover: 15, drawdown: 8 },
      { return: 7, turnover: 90, drawdown: 30 },
    ],
    TUTORIAL,
    261,
    '2. Fill five rows with plausible numbers, for example returns 4, 6, 5, 3, 7 and turnovers 20, 80, 30,\n   15, 90 and drawdowns 10, 25, 12, 8, 30.',
  ),
};

/** The paper exercise's columns and rows, unwrapped. */
export function paperExercise() {
  return { columns: valueOf(PAPER_EXERCISE.columns), rows: valueOf(PAPER_EXERCISE.rows) };
}

/** The one feature the campaign's own worked example declares, used as the demo allowlist. */
export const DEMO_FEATURE = fact(
  'ema_trend',
  AGENTS_TUTORIAL,
  131,
  'fields: families `cross_sectional_rank, absolute_threshold`; features `ema_trend`; bounds',
);

/** The genotype's fixed positions, unwrapped for the engine and the view. */
export function geneTables() {
  return {
    structural: valueOf(STRUCTURAL_GENES),
    parameters: valueOf(PARAMETER_GENES),
    conditionBound: valueOf(GENOME.conditionBound),
    statementFields: valueOf(GENOME.statementFields),
    mechanismPaths: valueOf(GENOME.mechanismPaths),
    mutationAttempts: valueOf(GENOME.mutationAttempts),
    mutationPlaces: valueOf(GENOME.mutationPlaces),
  };
}

/** The three degrees of freedom, unwrapped, with the property that separates the family degree. */
export function degrees() {
  const family = valueOf(REQUIRES_NEW_HYPOTHESIS);
  return DEGREES.map((row) => {
    const id = valueOf(row);
    return { id, requiresNewHypothesis: id === family };
  });
}

/** The seven objectives, each with the convention its name carries and its preferred direction. */
export function objectives() {
  const high = valueOf(PREFERS_HIGH);
  return OBJECTIVE_NAMES.map((row) => {
    const id = valueOf(row);
    return {
      id,
      convention: valueOf(OBJECTIVE_CONVENTIONS[id]),
      prefersHigh: high.includes(id),
    };
  });
}

/** The search policy's operator parameters and the identity it stamps a plan with. */
export function searchConstants() {
  const value = (key) => valueOf(SEARCH[key]);
  return {
    version: value('version'),
    policyId: value('policyId'),
    policyVersion: value('policyVersion'),
    equivalenceRuleId: value('equivalenceRuleId'),
    generationStamp: value('generationStamp'),
    parameterGenesPerMutation: value('parameterGenesPerMutation'),
    structuralGenesPerMutation: value('structuralGenesPerMutation'),
    explorationShare: value('explorationShare'),
    editAttempts: value('editAttempts'),
    mutationRate: value('mutationRate'),
    populationSize: value('populationSize'),
    elite: value('elite'),
    familyFloor: value('familyFloor'),
    mutableFields: value('mutableFields'),
  };
}

/**
 * The search policy in force, in the shape evolve.py's `SearchPolicy` holds: the two rates, the
 * family allocation, the generation stamp and the identity the plan is stamped with.
 */
export function searchPolicy(overrides = {}) {
  return {
    policy_id: valueOf(SEARCH.policyId),
    version: valueOf(SEARCH.policyVersion),
    prompts: {},
    mutation_rate: valueOf(SEARCH.mutationRate),
    exploration_share: valueOf(SEARCH.explorationShare),
    family_allocation: valueOf(SEARCH.familyAllocation),
    generation: valueOf(SEARCH.generation),
    ...overrides,
  };
}

/** The frozen objects, each with the reason a change to it is refused. */
export function frozenFields() {
  return valueOf(FROZEN_FIELDS).map(([field, why]) => ({ field, why }));
}

/** The five paths the equivalence rule ignores. */
export function ignoredPaths() {
  return valueOf(EQUIVALENCE_IGNORED_PATHS);
}

/** The bounded fields the schema reads as integers. */
export function integerBounds() {
  return INTEGER_BOUNDS.map(valueOf);
}

/** The complexity caps the demo's candidate shape is held inside. */
export function complexityLimits() {
  return valueOf(COMPLEXITY_LIMITS);
}

/** The genotype-level and search-level refusal codes, in the source's order. */
export function refusalCodes() {
  return {
    genome: GENOME_REFUSAL_CODES.map(valueOf),
    search: SEARCH_REFUSAL_CODES.map(valueOf),
  };
}

/** The genotype-level refusals, as { key: { code, message } }, the messages still templated. */
export function genomeRefusals() {
  return Object.fromEntries(
    Object.entries(GENOME_REFUSALS).map(([key, node]) => [key, valueOf(node)]),
  );
}

/** The search-level refusals, as { key: { code, message } }, the messages still templated. */
export function searchRefusals() {
  return Object.fromEntries(
    Object.entries(SEARCH_REFUSALS).map(([key, node]) => [key, valueOf(node)]),
  );
}

/**
 * The declaration table the genes resolve through, in the shape src/engine/policy.js returns: the
 * fifteen declared sets and the nineteen bounds, plus the allowlist and the limits the demo needs.
 */
export function demoPolicy() {
  return {
    policyId: 'discovery.policy',
    version: 1,
    sets: declaredSetsByName(),
    bounds: BOUNDS.map(valueOf),
    limits: complexityLimits(),
    featureAllowlist: [valueOf(DEMO_FEATURE)],
  };
}

/** The declared sets keyed by their policy name, which is what a gene's table names. */
export function declaredSetsByName() {
  const names = [
    'families',
    'asset_classes',
    'membership_policies',
    'signal_frequencies',
    'decision_times',
    'execution_eligibilities',
    'corporate_action_treatments',
    'entry_directions',
    'operators',
    'selection_modes',
    'tie_breaks',
    'weighting_methods',
    'cash_policies',
    'rebalance_frequencies',
    'rebalance_timings',
  ];
  if (names.length !== DECLARED_SETS.length) {
    throw new Error('the declared sets and their names are not in step');
  }
  return Object.fromEntries(names.map((name, index) => [name, valueOf(DECLARED_SETS[index])]));
}
