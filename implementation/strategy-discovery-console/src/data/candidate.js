/**
 * The candidate language, transcribed.
 *
 * This module models `python/nautilus_trader/discovery/schema.py`, the four stages of
 * `python/nautilus_trader/discovery/validate.py` and the compilation stage of
 * `python/nautilus_trader/discovery/compile.py`: the twelve blocks and every field they declare, the
 * closed vocabularies the language refuses by name and by shape, the eleven resolver rules a value
 * has to come through, the three complexity caps, the five stages in their fixed order, and every
 * refusal code with the message the source records it with.
 *
 * Two things are marked rather than invented:
 *
 * - the feature allowlist and the feature registry are campaign inputs the source's `first_release`
 *   invents none of, so this module transcribes the rule that a feature must be inside the allowlist
 *   and nothing about a particular feature;
 * - the bound unit words the tutorial supplies are transcribed in `src/data/policy.js`, because the
 *   code stores bare `Decimal`s.
 *
 * Every value below is wrapped in `fact(...)`; `tools/check-facts.mjs` fails when the supporting text
 * has left the file it names.
 */

import { fact } from './source.js';

const SCHEMA = 'python/nautilus_trader/discovery/schema.py';
const VALIDATE = 'python/nautilus_trader/discovery/validate.py';
const POLICY = 'python/nautilus_trader/discovery/policy.py';
const COMPILE = 'python/nautilus_trader/discovery/compile.py';
const TUTORIAL = 'tutorials/strategy-discovery/what-a-candidate-is/README.md';

/** The identity and the accepted versions of the specification schema. */
export const SCHEMA_ID = fact('discovery.specification', SCHEMA, 63, 'SPECIFICATION_SCHEMA_ID = "discovery.specification"');

/** The one version this build accepts, which every specification must state. */
export const SCHEMA_VERSION = fact(
  'discovery.specification/v1',
  SCHEMA,
  64,
  'SPECIFICATION_SCHEMA_VERSION = "discovery.specification/v1"',
);

/** The accepted versions as a set, which is what `migrate` tests membership against. */
export const SCHEMA_VERSIONS = fact(
  ['discovery.specification/v1'],
  SCHEMA,
  65,
  'SPECIFICATION_SCHEMA_VERSIONS: tuple[str, ...] = (SPECIFICATION_SCHEMA_VERSION,)',
);

/** The most migrations one payload may pass through, so a cycle is a refusal rather than a loop. */
export const MAX_MIGRATIONS = fact(8, SCHEMA, 103, '_MAX_MIGRATIONS = 8');

/** The migration table as this build ships it: empty, so an older version is refused. */
export const MIGRATIONS = fact({}, SCHEMA, 414, 'MIGRATIONS: dict[str, Migration] = {}');

/** The engine version a plan carries when the installed package exposes none. */
export const ENGINE_VERSION_UNKNOWN = fact('unknown', COMPILE, 63, 'ENGINE_VERSION_UNKNOWN = "unknown"');

/** One field of a block: how it is read, what it resolves against, and whether it is required. */
function field({ name, line, text, reads = 'text', declaration = null, optional = false }) {
  return { name: fact(name, SCHEMA, line, text), reads, declaration, optional };
}

/** The twelve block names, in the order `parse_specification` requires them. */
export const BLOCK_NAMES = fact(
  [
    'identity',
    'universe',
    'data',
    'features',
    'selection',
    'entry',
    'exit',
    'portfolio',
    'rebalance',
    'risk',
    'evaluation',
    'provenance',
  ],
  SCHEMA,
  1686,
  '_BLOCK_NAMES: tuple[str, ...] = (\n    "identity",\n    "universe",\n    "data",\n' +
    '    "features",\n    "selection",\n    "entry",\n    "exit",\n    "portfolio",\n' +
    '    "rebalance",\n    "risk",\n    "evaluation",\n    "provenance",\n)',
);

/** The fields a feature declaration states, under the alias the rest of the specification uses. */
export const DECLARATION_FIELDS = [
  field({ name: 'feature', line: 1357, text: 'feature_id = entry.text("feature")' }),
  field({
    name: 'lookback',
    line: 1366,
    text: 'lookback = entry.bounded_int("lookback", "feature.lookback")',
    reads: 'bounded_int',
    declaration: 'feature.lookback',
  }),
  field({
    name: 'skip',
    line: 1367,
    text: 'skip = entry.bounded_int("skip", "feature.skip", required=False)',
    reads: 'bounded_int',
    declaration: 'feature.skip',
    optional: true,
  }),
];

/** The only two keys a condition group may state, and they are alternatives. */
export const CONDITION_GROUP_KEYS = fact(
  ['all', 'any'],
  SCHEMA,
  1467,
  'groups = [group for group in ("all", "any") if group in payload]',
);

/** The three fields one condition leaf is fixed to. */
export const CONDITION_LEAF_FIELDS = [
  field({ name: 'feature', line: 1520, text: 'feature = leaf.feature_ref("feature", walk.declared)', reads: 'feature_ref' }),
  field({
    name: 'comparison',
    line: 1521,
    text: 'comparison = leaf.declared("comparison", "operators")',
    reads: 'declared',
    declaration: 'operators',
  }),
  field({
    name: 'value',
    line: 1522,
    text: 'value = leaf.bounded("value", "condition.value")',
    reads: 'bounded',
    declaration: 'condition.value',
  }),
];

/** The three risk categories, each with the fields it is allowed to state and no others. */
export const RISK_CATEGORIES = [
  {
    name: fact('pre_trade', SCHEMA, 1681, '"pre_trade": ("max_gross_exposure",),'),
    meaning: fact(
      'The constraint the allocation function enforces before targets are emitted.',
      SCHEMA,
      722,
      'The constraint the allocation function enforces before targets are emitted.',
    ),
    fields: [
      field({
        name: 'max_gross_exposure',
        line: 1606,
        text: 'max_gross_exposure=pre_trade_reader.bounded(',
        reads: 'bounded',
        declaration: 'risk.max_gross_exposure',
        optional: true,
      }),
    ],
  },
  {
    name: fact('runtime', SCHEMA, 1682, '"runtime": ("halt_new_entries", "flatten_ineligible"),'),
    meaning: fact('The controls the interpreter applies while the run is live.', SCHEMA, 731, 'The controls the interpreter applies while the run is live.'),
    fields: [
      field({
        name: 'halt_new_entries',
        line: 1616,
        text: 'halt_new_entries=runtime_reader.boolean("halt_new_entries", default=False) or False,',
        reads: 'boolean',
        optional: true,
      }),
      field({
        name: 'flatten_ineligible',
        line: 1617,
        text: 'flatten_ineligible=runtime_reader.boolean("flatten_ineligible", default=False) or False,',
        reads: 'boolean',
        optional: true,
      }),
    ],
  },
  {
    name: fact('post_run', SCHEMA, 1683, '"post_run": ("max_drawdown", "max_turnover", "max_cost_ratio"),'),
    meaning: fact(
      'The acceptance limits the gate evaluates once the observations exist.',
      SCHEMA,
      741,
      'The acceptance limits the gate evaluates once the observations exist.',
    ),
    fields: [
      field({
        name: 'max_drawdown',
        line: 1623,
        text: 'max_drawdown=post_run_reader.bounded("max_drawdown", "risk.max_drawdown", required=False),',
        reads: 'bounded',
        declaration: 'risk.max_drawdown',
        optional: true,
      }),
      field({
        name: 'max_turnover',
        line: 1624,
        text: 'max_turnover=post_run_reader.bounded("max_turnover", "risk.max_turnover", required=False),',
        reads: 'bounded',
        declaration: 'risk.max_turnover',
        optional: true,
      }),
      field({
        name: 'max_cost_ratio',
        line: 1625,
        text: 'max_cost_ratio=post_run_reader.bounded(',
        reads: 'bounded',
        declaration: 'risk.max_cost_ratio',
        optional: true,
      }),
    ],
  },
];

/** The twelve blocks, each with the question it answers and every field it declares. */
export const BLOCKS = [
  {
    name: fact('identity', SCHEMA, 1687, '"identity",'),
    question: fact(
      'What does the candidate claim, in the words it will be audited by?',
      TUTORIAL,
      56,
      '| `identity`   | What does the candidate claim, in the words it will be audited by?',
    ),
    reader: fact('_read_identity', SCHEMA, 1277, 'def _read_identity(reader: _Reader) -> IdentityBlock:'),
    fields: [
      field({
        name: 'family',
        line: 1279,
        text: 'family=reader.declared("family", "families") or "",',
        reads: 'declared',
        declaration: 'families',
      }),
      field({ name: 'hypothesis', line: 1280, text: 'hypothesis=reader.text("hypothesis") or "",' }),
      field({ name: 'mechanism', line: 1281, text: 'mechanism=reader.text("mechanism") or "",' }),
      field({ name: 'falsification', line: 1282, text: 'falsification=reader.text("falsification") or "",' }),
    ],
  },
  {
    name: fact('universe', SCHEMA, 1688, '"universe",'),
    question: fact(
      'Which instruments may it hold, and what floors decide membership?',
      TUTORIAL,
      57,
      '| `universe`   | Which instruments may it hold, and what floors decide membership?',
    ),
    reader: fact('_read_universe', SCHEMA, 1286, 'def _read_universe(reader: _Reader) -> UniverseBlock:'),
    fields: [
      field({
        name: 'asset_classes',
        line: 1288,
        text: 'asset_classes=reader.declared_list("asset_classes", "asset_classes"),',
        reads: 'declared_list',
        declaration: 'asset_classes',
      }),
      field({
        name: 'membership_policy',
        line: 1289,
        text: 'membership_policy=reader.declared("membership_policy", "membership_policies") or "",',
        reads: 'declared',
        declaration: 'membership_policies',
      }),
      field({ name: 'source', line: 1290, text: 'source=reader.text("source") or "",' }),
      field({
        name: 'min_price',
        line: 1291,
        text: 'min_price=reader.bounded("min_price", "universe.min_price", required=False),',
        reads: 'bounded',
        declaration: 'universe.min_price',
        optional: true,
      }),
      field({
        name: 'min_dollar_volume',
        line: 1292,
        text: 'min_dollar_volume=reader.bounded(',
        reads: 'bounded',
        declaration: 'universe.min_dollar_volume',
        optional: true,
      }),
      field({ name: 'exclusions', line: 1297, text: 'exclusions=reader.identifiers("exclusions"),', reads: 'identifiers', optional: true }),
    ],
  },
  {
    name: fact('data', SCHEMA, 1689, '"data",'),
    question: fact(
      'When does it decide, when may its intent fill, and how are company actions treated?',
      TUTORIAL,
      58,
      '| `data`       | When does it decide, when may its intent fill, and how are company actions treated?',
    ),
    reader: fact('_read_data', SCHEMA, 1301, 'def _read_data(reader: _Reader) -> DataBlock:'),
    fields: [
      field({
        name: 'signal_frequency',
        line: 1303,
        text: 'signal_frequency=reader.declared("signal_frequency", "signal_frequencies") or "",',
        reads: 'declared',
        declaration: 'signal_frequencies',
      }),
      field({
        name: 'decision_time',
        line: 1304,
        text: 'decision_time=reader.declared("decision_time", "decision_times") or "",',
        reads: 'declared',
        declaration: 'decision_times',
      }),
      field({
        name: 'execution_eligibility',
        line: 1305,
        text: 'execution_eligibility=reader.declared(',
        reads: 'declared',
        declaration: 'execution_eligibilities',
      }),
      field({
        name: 'corporate_action_treatment',
        line: 1310,
        text: 'corporate_action_treatment=reader.declared(',
        reads: 'declared',
        declaration: 'corporate_action_treatments',
      }),
    ],
  },
  {
    name: fact('features', SCHEMA, 1690, '"features",'),
    question: fact(
      'Which registered measurements does it select, and what names does the rest of it use for them?',
      TUTORIAL,
      59,
      '| `features`   | Which registered measurements does it select, and what names does the rest of it use for them?',
    ),
    reader: fact('_read_features', SCHEMA, 1318, 'def _read_features(reader: _Reader) -> FeaturesBlock:'),
    fields: [
      field({ name: 'declared', line: 1319, text: 'present, payload = reader.take("declared")', reads: 'mapping', declaration: 'declaration' }),
      ...DECLARATION_FIELDS,
    ],
  },
  {
    name: fact('selection', SCHEMA, 1691, '"selection",'),
    question: fact(
      'How does it choose its eligible set, and how is a tie broken?',
      TUTORIAL,
      60,
      '| `selection`  | How does it choose its eligible set, and how is a tie broken?',
    ),
    reader: fact('_read_selection', SCHEMA, 1382, 'def _read_selection(reader: _Reader, declared: frozenset[str]) -> SelectionBlock:'),
    fields: [
      field({
        name: 'mode',
        line: 1384,
        text: 'mode=reader.declared("mode", "selection_modes") or "",',
        reads: 'declared',
        declaration: 'selection_modes',
      }),
      field({ name: 'feature', line: 1385, text: 'feature=reader.feature_ref("feature", declared, required=False),', reads: 'feature_ref', optional: true }),
      field({ name: 'percentile', line: 1386, text: 'percentile=reader.bounded("percentile", "selection.percentile", required=False),', reads: 'bounded', declaration: 'selection.percentile', optional: true }),
      field({ name: 'count', line: 1387, text: 'count=reader.bounded("count", "selection.count", required=False),', reads: 'bounded', declaration: 'selection.count', optional: true }),
      field({ name: 'threshold', line: 1388, text: 'threshold=reader.bounded("threshold", "selection.threshold", required=False),', reads: 'bounded', declaration: 'selection.threshold', optional: true }),
      field({
        name: 'comparison',
        line: 1389,
        text: 'comparison=reader.declared("comparison", "operators", required=False),',
        reads: 'declared',
        declaration: 'operators',
        optional: true,
      }),
      field({
        name: 'tie_break',
        line: 1390,
        text: 'tie_break=reader.declared("tie_break", "tie_breaks", required=False),',
        reads: 'declared',
        declaration: 'tie_breaks',
        optional: true,
      }),
    ],
  },
  {
    name: fact('entry', SCHEMA, 1692, '"entry",'),
    question: fact(
      'Whether and when does it enter an eligible instrument?',
      TUTORIAL,
      61,
      '| `entry`      | Whether and when does it enter an eligible instrument?',
    ),
    reader: fact('_read_entry', SCHEMA, 1531, 'def _read_entry(reader: _Reader, declared: frozenset[str]) -> EntryBlock:'),
    fields: [
      field({
        name: 'direction',
        line: 1532,
        text: 'direction = reader.declared("direction", "entry_directions") or ""',
        reads: 'declared',
        declaration: 'entry_directions',
      }),
      field({ name: 'conditions', line: 1533, text: 'present, payload = reader.take("conditions")', reads: 'condition_tree', optional: true }),
      ...CONDITION_LEAF_FIELDS.map((leaf) => ({ ...leaf })),
    ],
  },
  {
    name: fact('exit', SCHEMA, 1693, '"exit",'),
    question: fact('Which declared ways does a position leave the portfolio?', TUTORIAL, 62, '| `exit`       | Which declared ways does a position leave the portfolio?'),
    reader: fact('_read_exit', SCHEMA, 1558, 'def _read_exit(reader: _Reader, declared: frozenset[str]) -> ExitBlock:'),
    fields: [
      field({ name: 'holding_period_cap', line: 1560, text: 'holding_period_cap=reader.bounded_int(', reads: 'bounded_int', declaration: 'exit.holding_period_cap', optional: true }),
      field({ name: 'rank_exit_count', line: 1565, text: 'rank_exit_count=reader.bounded_int(', reads: 'bounded_int', declaration: 'exit.rank_exit_count', optional: true }),
      field({ name: 'feature', line: 1570, text: 'feature=reader.feature_ref("feature", declared, required=False),', reads: 'feature_ref', optional: true }),
      field({
        name: 'comparison',
        line: 1571,
        text: 'comparison=reader.declared("comparison", "operators", required=False),',
        reads: 'declared',
        declaration: 'operators',
        optional: true,
      }),
      field({ name: 'threshold', line: 1572, text: 'threshold=reader.bounded("threshold", "condition.value", required=False),', reads: 'bounded', declaration: 'condition.value', optional: true }),
      field({ name: 'stop_fraction', line: 1573, text: 'stop_fraction=reader.bounded("stop_fraction", "exit.stop_fraction", required=False),', reads: 'bounded', declaration: 'exit.stop_fraction', optional: true }),
      field({ name: 'target_fraction', line: 1574, text: 'target_fraction=reader.bounded("target_fraction", "exit.target_fraction", required=False),', reads: 'bounded', declaration: 'exit.target_fraction', optional: true }),
      field({ name: 'flatten_ineligible', line: 1575, text: 'flatten_ineligible=reader.boolean("flatten_ineligible", default=True) or False,', reads: 'boolean', optional: true }),
    ],
  },
  {
    name: fact('portfolio', SCHEMA, 1694, '"portfolio",'),
    question: fact('How much does it hold, in how many positions, under which caps?', TUTORIAL, 63, '| `portfolio`  | How much does it hold, in how many positions, under which caps?'),
    reader: fact('_read_portfolio', SCHEMA, 1579, 'def _read_portfolio(reader: _Reader) -> PortfolioBlock:'),
    fields: [
      field({ name: 'position_count', line: 1581, text: 'position_count=reader.bounded_int("position_count", "portfolio.position_count") or 0,', reads: 'bounded_int', declaration: 'portfolio.position_count' }),
      field({
        name: 'weighting',
        line: 1582,
        text: 'weighting=reader.declared("weighting", "weighting_methods") or "",',
        reads: 'declared',
        declaration: 'weighting_methods',
      }),
      field({ name: 'max_position_weight', line: 1583, text: 'max_position_weight=reader.bounded("max_position_weight", "portfolio.max_position_weight")', reads: 'bounded', declaration: 'portfolio.max_position_weight' }),
      field({ name: 'max_sector_weight', line: 1585, text: 'max_sector_weight=reader.bounded(', reads: 'bounded', declaration: 'portfolio.max_sector_weight', optional: true }),
      field({
        name: 'cash_policy',
        line: 1590,
        text: 'cash_policy=reader.declared("cash_policy", "cash_policies") or "",',
        reads: 'declared',
        declaration: 'cash_policies',
      }),
    ],
  },
  {
    name: fact('rebalance', SCHEMA, 1695, '"rebalance",'),
    question: fact('How often does it adjust the holdings, and when?', TUTORIAL, 64, '| `rebalance`  | How often does it adjust the holdings, and when?'),
    reader: fact('_read_rebalance', SCHEMA, 1594, 'def _read_rebalance(reader: _Reader) -> RebalanceBlock:'),
    fields: [
      field({
        name: 'frequency',
        line: 1596,
        text: 'frequency=reader.declared("frequency", "rebalance_frequencies") or "",',
        reads: 'declared',
        declaration: 'rebalance_frequencies',
      }),
      field({
        name: 'timing',
        line: 1597,
        text: 'timing=reader.declared("timing", "rebalance_timings") or "",',
        reads: 'declared',
        declaration: 'rebalance_timings',
      }),
    ],
  },
  {
    name: fact('risk', SCHEMA, 1696, '"risk",'),
    question: fact(
      'Which constraints act before the trade, while it is live, and after the run?',
      TUTORIAL,
      65,
      '| `risk`       | Which constraints act before the trade, while it is live, and after the run?',
    ),
    reader: fact('_read_risk', SCHEMA, 1601, 'def _read_risk(reader: _Reader) -> RiskBlock:'),
    fields: RISK_CATEGORIES,
  },
  {
    name: fact('evaluation', SCHEMA, 1697, '"evaluation",'),
    question: fact('Which frozen objects is it judged against?', TUTORIAL, 66, '| `evaluation` | Which frozen objects is it judged against?'),
    reader: fact('_read_evaluation', SCHEMA, 1662, 'def _read_evaluation(reader: _Reader) -> EvaluationBlock:'),
    fields: [
      field({ name: 'benchmark', line: 1664, text: 'benchmark=reader.reference("benchmark") or "",', reads: 'reference' }),
      field({ name: 'cost_model', line: 1665, text: 'cost_model=reader.reference("cost_model") or "",', reads: 'reference' }),
      field({ name: 'validation_policy', line: 1666, text: 'validation_policy=reader.reference("validation_policy") or "",', reads: 'reference' }),
    ],
  },
  {
    name: fact('provenance', SCHEMA, 1698, '"provenance",'),
    question: fact('Where did it come from, recorded rather than trusted?', TUTORIAL, 67, '| `provenance` | Where did it come from, recorded rather than trusted?'),
    reader: fact('_read_provenance', SCHEMA, 1670, 'def _read_provenance(reader: _Reader) -> ProvenanceBlock:'),
    fields: [
      field({ name: 'generator', line: 1672, text: 'generator=reader.text("generator") or "",' }),
      field({ name: 'seed', line: 1673, text: 'seed=reader.integer("seed") or 0,', reads: 'integer' }),
      field({ name: 'parents', line: 1674, text: 'parents=reader.identifiers("parents"),', reads: 'identifiers', optional: true }),
    ],
  },
];

/**
 * The field names that ask for code rather than data, matched on the key's lowercase spelling.
 *
 * A specification naming one of these is refused by name, wherever in the payload it is put.
 */
export const CODE_FIELD_NAMES = [
  fact('callable', SCHEMA, 71, '"callable",'),
  fact('class', SCHEMA, 72, '"class",'),
  fact('class_path', SCHEMA, 73, '"class_path",'),
  fact('code', SCHEMA, 74, '"code",'),
  fact('code_path', SCHEMA, 75, '"code_path",'),
  fact('entry_point', SCHEMA, 76, '"entry_point",'),
  fact('eval', SCHEMA, 77, '"eval",'),
  fact('exec', SCHEMA, 78, '"exec",'),
  fact('expression', SCHEMA, 79, '"expression",'),
  fact('hook', SCHEMA, 80, '"hook",'),
  fact('import', SCHEMA, 81, '"import",'),
  fact('imports', SCHEMA, 82, '"imports",'),
  fact('lambda', SCHEMA, 83, '"lambda",'),
  fact('module', SCHEMA, 84, '"module",'),
  fact('module_path', SCHEMA, 85, '"module_path",'),
  fact('script', SCHEMA, 86, '"script",'),
  fact('source_code', SCHEMA, 87, '"source_code",'),
  fact('strategy_path', SCHEMA, 88, '"strategy_path",'),
];

/** The value shapes that are code rather than data: an import, an evaluation, a call, a `module:name`. */
export const CODE_VALUE_PATTERNS = [
  fact('\\b__import__\\b', SCHEMA, 95, 're.compile(r"\\b__import__\\b"),'),
  fact('\\b(?:eval|exec)\\s*\\(', SCHEMA, 96, 're.compile(r"\\b(?:eval|exec)\\s*\\("),'),
  fact('\\b(?:os\\.system|subprocess|importlib)\\b', SCHEMA, 97, 're.compile(r"\\b(?:os\\.system|subprocess|importlib)\\b"),'),
  fact('^[A-Za-z_][\\w]*(?:\\.[A-Za-z_][\\w]*)*:[A-Za-z_][\\w]*$', SCHEMA, 98, 're.compile(r"^[A-Za-z_][\\w]*(?:\\.[A-Za-z_][\\w]*)*:[A-Za-z_][\\w]*$"),'),
];

/** One read path: the reader that takes it, and the rule the reader states in its own words. */
function readerRule(name, line, text, ruleLine, rule) {
  return { reader: fact(name, SCHEMA, line, text), rule: fact(rule, SCHEMA, ruleLine, rule) };
}

/**
 * The eleven read paths a field value may come through.
 *
 * A field's `reads` names one of these, so the refused field can be shown beside the rule the reader
 * applied to it rather than beside a rule this console invented.
 */
export const READER_RULES = {
  text: readerRule('text', 1015, 'def text(self, name: str, *, required: bool = True) -> str | None:', 1017, 'Read a non-empty text field.'),
  boolean: readerRule('boolean', 1041, 'def boolean(self, name: str, *, default: bool | None = None) -> bool | None:', 1043, 'Read a boolean field.'),
  integer: readerRule('integer', 1064, 'def integer(self, name: str, *, required: bool = True) -> int | None:', 1066, 'Read an integer field.'),
  bounded: readerRule('bounded', 1087, 'def bounded(self, name: str, bound_name: str, *, required: bool = True) -> Decimal | None:', 1089, 'Read a decimal field and check it against the policy\'s bound of that name.'),
  bounded_int: readerRule('bounded_int', 1123, 'def bounded_int(self, name: str, bound_name: str, *, required: bool = True) -> int | None:', 1125, 'Read an integer field and check it against the policy\'s bound of that name.'),
  declared: readerRule('declared', 1140, 'def declared(self, name: str, set_name: str, *, required: bool = True) -> str | None:', 1142, 'Read a field whose value is one of the policy\'s declared names.'),
  declared_list: readerRule('declared_list', 1159, 'def declared_list(self, name: str, set_name: str, *, required: bool = True) -> tuple[str, ...]:', 1161, 'Read a sequence whose every element is one of the policy\'s declared names.'),
  identifiers: readerRule('identifiers', 1179, 'def identifiers(self, name: str, *, required: bool = False) -> tuple[str, ...]:', 1181, 'Read a sequence of non-empty, distinct identifiers.'),
  feature_ref: readerRule('feature_ref', 1220, 'def feature_ref(', 1220, 'def feature_ref('),
  reference: readerRule('reference', 1243, 'def reference(self, name: str, *, required: bool = True) -> str | None:', 1245, 'Read a field that must name one of the policy\'s frozen references.'),
  mapping: readerRule('mapping', 1319, 'present, payload = reader.take("declared")', 1329, 'if not isinstance(payload, Mapping):'),
  condition_tree: readerRule('condition_tree', 1533, 'present, payload = reader.take("conditions")', 1447, 'def _read_condition(payload: object, walk: _ConditionWalk) -> ConditionNode | None:'),
};

/** The three complexity caps, which limit how much expression one candidate may spend. */
export const CAPS = {
  max_conditions: fact(4, POLICY, 113, 'max_conditions: int = 4'),
  max_condition_depth: fact(2, POLICY, 114, 'max_condition_depth: int = 2'),
  max_features: fact(8, POLICY, 115, 'max_features: int = 8'),
};

/** The five stages, in the one order the pipeline runs them and the file that implements each. */
export const STAGES = [
  {
    order: 1,
    id: 'schema_validation',
    name: fact('schema validation', TUTORIAL, 212, 'stages are schema validation, feature resolution, semantic validation and temporal validation in'),
    implementation: fact('parse_specification', SCHEMA, 1702, 'def parse_specification('),
  },
  {
    order: 2,
    id: 'feature_resolution',
    name: fact('feature resolution', TUTORIAL, 212, 'stages are schema validation, feature resolution, semantic validation and temporal validation in'),
    implementation: fact('_resolve_features', VALIDATE, 280, 'def _resolve_features('),
  },
  {
    order: 3,
    id: 'semantic_validation',
    name: fact('semantic validation', TUTORIAL, 212, 'stages are schema validation, feature resolution, semantic validation and temporal validation in'),
    implementation: fact('_validate_semantics', VALIDATE, 320, 'def _validate_semantics('),
  },
  {
    order: 4,
    id: 'temporal_validation',
    name: fact('temporal validation', TUTORIAL, 212, 'stages are schema validation, feature resolution, semantic validation and temporal validation in'),
    implementation: fact('_validate_temporal', VALIDATE, 609, 'def _validate_temporal('),
  },
  {
    order: 5,
    id: 'trusted_compilation',
    name: fact('trusted compilation', TUTORIAL, 213, 'and trusted compilation in'),
    implementation: fact('compile_specification', COMPILE, 203, 'def compile_specification('),
  },
];

/** One trading session in nanoseconds, which both the signal and the rebalance interval read. */
export const SESSION_NS = fact(86400000000000, VALIDATE, 81, 'SESSION_NS = 86_400_000_000_000');

/** The interval of each declared frequency in sessions, which is the ordering the compiler uses. */
export const FREQUENCY_SESSIONS = fact(
  { daily: 1, weekly: 5, monthly: 21 },
  VALIDATE,
  82,
  'FREQUENCY_SESSIONS: dict[str, int] = {"daily": 1, "weekly": 5, "monthly": 21}',
);

/** The instants the compiler can order, from a decision at the open to the latest eligible fill. */
export const EXECUTION_INSTANT_ORDER = fact(
  { session_open: 0, session_close: 1, same_session_close: 1, next_session_open: 2 },
  VALIDATE,
  87,
  'EXECUTION_INSTANT_ORDER: dict[str, int] = {\n    "session_open": 0,\n    "session_close": 1,\n' +
    '    "same_session_close": 1,\n    "next_session_open": 2,\n}',
);

/** The three selection modes the semantic stage knows how to check, in the order it tries them. */
export const SELECTION_MODES_CHECKED = fact(
  ['rank', 'threshold', 'none'],
  VALIDATE,
  96,
  'RANK_MODE = "rank"\nTHRESHOLD_MODE = "threshold"\nNONE_MODE = "none"',
);

/** The multiple a lookback is warned against, and the wholly-invested level a cap is warned below. */
export const WARMUP_DOMINANCE_MULTIPLE = fact(4, VALIDATE, 102, 'WARMUP_DOMINANCE_MULTIPLE = 4');

/** `Decimal(1)`: the allocation a position cap is compared against when it warns. */
export const FULLY_INVESTED = fact(1, VALIDATE, 103, 'FULLY_INVESTED = Decimal(1)');

/** The paths the refusals and warnings of the validation stages are reported at. */
export const REFUSAL_PATHS = {
  selection: fact('selection', VALIDATE, 107, '_SELECTION_PATH = "selection"'),
  exit: fact('exit', VALIDATE, 108, '_EXIT_PATH = "exit"'),
  rankExit: fact('exit.rank_exit_count', VALIDATE, 109, '_RANK_EXIT_PATH = "exit.rank_exit_count"'),
  portfolio: fact('portfolio', VALIDATE, 110, '_PORTFOLIO_PATH = "portfolio"'),
  sectorWeight: fact('portfolio.max_sector_weight', VALIDATE, 111, '_SECTOR_WEIGHT_PATH = "portfolio.max_sector_weight"'),
  timing: fact('rebalance.timing', VALIDATE, 112, '_TIMING_PATH = "rebalance.timing"'),
  frequency: fact('rebalance.frequency', VALIDATE, 113, '_FREQUENCY_PATH = "rebalance.frequency"'),
  eligibility: fact('data.execution_eligibility', VALIDATE, 114, '_ELIGIBILITY_PATH = "data.execution_eligibility"'),
  decisionTime: fact('data.decision_time', VALIDATE, 115, '_DECISION_TIME_PATH = "data.decision_time"'),
  postRun: fact('risk.post_run', VALIDATE, 116, '_POST_RUN_PATH = "risk.post_run"'),
};

/** The subjects the field-level selection rules are phrased by. */
export const RULE_SUBJECTS = {
  rank: fact('rank selection', VALIDATE, 119, '_RANK_SUBJECT = "rank selection"'),
  threshold: fact('threshold selection', VALIDATE, 120, '_THRESHOLD_SUBJECT = "threshold selection"'),
  none: fact('selection that ranks nothing', VALIDATE, 121, '_NONE_SUBJECT = "selection that ranks nothing"'),
};

/** The four warnings, which are a separate channel from the refusals. */
export const WARNING_CODES = [
  {
    code: fact('feature_warmup_dominates', VALIDATE, 133, 'FEATURE_WARMUP_DOMINATES = "feature_warmup_dominates"'),
    messages: [fact('a lookback of {lookback} exceeds 4 {frequency} intervals, which is {ceiling} sessions, so the feature\'s warm-up consumes most of the window it is read in', VALIDATE, 779, 'f"a lookback of {declaration.lookback} exceeds {WARMUP_DOMINANCE_MULTIPLE} "')],
  },
  {
    code: fact('unused_declared_feature', VALIDATE, 134, 'UNUSED_DECLARED_FEATURE = "unused_declared_feature"'),
    messages: [fact('the alias {alias} is referenced by neither the selection, the entry tree nor the exit block, so its value is never read', VALIDATE, 808, 'f"the alias {declaration.alias} is referenced by neither the selection, the entry tree "')],
  },
  {
    code: fact('position_cap_below_fully_invested', VALIDATE, 135, 'POSITION_CAP_BELOW_FULLY_INVESTED = "position_cap_below_fully_invested"'),
    messages: [fact('{count} positions at max_position_weight {weight} allocate {allocated}, below fully invested, so the remainder is decided by the cash policy rather than by the selection', VALIDATE, 886, 'f"{portfolio.position_count} positions at max_position_weight "')],
  },
  {
    code: fact('post_run_limits_are_gate_conditions', VALIDATE, 136, 'POST_RUN_LIMITS_ARE_GATE_CONDITIONS = "post_run_limits_are_gate_conditions"'),
    messages: [fact('{name} is measured once the observations exist, so it cannot control the run it is recorded beside and is a candidate acceptance criterion rather than a runtime promise', VALIDATE, 917, 'f"{name} is measured once the observations exist, so it cannot control the run it is "')],
  },
];

/** One refusal code: the code, every message it is recorded with, and the function that records it. */
function refusalRow([path, name, codeLine, where, whereLine, whereText, messages]) {
  return {
    code: fact(name, SCHEMA, codeLine, `${name.toUpperCase()} = "${name}"`),
    where: fact(where, path, whereLine, whereText),
    messages: messages.map(([line, text, message]) => fact(message, path, line, text)),
  };
}

/**
 * The closed vocabulary of thirty-two refusal codes, each with the message the source records.
 *
 * `message` is the f-string's body, so a placeholder is the source's own and the console can show a
 * candidate what it would have been told.
 */
export const REFUSAL_CODES = [
  [SCHEMA, 'not_a_mapping', 289, 'parse_specification', 1702, 'def parse_specification(', [
    [1735, 'f"a specification must be a mapping, was {type(payload).__name__}",', 'a specification must be a mapping, was {type(payload).__name__}'],
  ]],
  [SCHEMA, 'version_missing', 290, 'migrate', 417, 'def migrate(', [
    [455, 'f"a specification must declare its schema version, one of "', 'a specification must declare its schema version, one of discovery.specification/v1'],
  ]],
  [SCHEMA, 'version_not_a_string', 291, 'migrate', 417, 'def migrate(', [
    [455, 'f"a specification must declare its schema version, one of "', 'a specification must declare its schema version, one of discovery.specification/v1'],
  ]],
  [SCHEMA, 'version_unsupported', 292, 'migrate', 417, 'def migrate(', [
    [467, 'f"schema version {version} is not accepted and no migration from it is "', 'schema version {version} is not accepted and no migration from it is registered, this build accepts discovery.specification/v1'],
  ]],
  [SCHEMA, 'version_migration_cycle', 293, 'migrate', 417, 'def migrate(', [
    [480, `f"the migrations applied to schema version {payload.get('version')} do not reach an "`, 'the migrations applied to schema version {version} do not reach an accepted version'],
  ]],
  [SCHEMA, 'block_missing', 294, 'parse_specification', 1702, 'def parse_specification(', [
    [1753, 'reader.refuse(RefusalCode.BLOCK_MISSING, name, f"the {name} block is required")', 'the {name} block is required'],
  ]],
  [SCHEMA, 'block_not_a_mapping', 295, '_read_block', 1822, 'def _read_block[T](', [
    [975, 'f"a block must be a mapping, was {type(payload).__name__}",', 'a block must be a mapping, was {type(payload).__name__}'],
  ]],
  [SCHEMA, 'block_unknown_field', 296, 'finish', 1003, 'def finish(self) -> None:', [
    [1012, 'f"the {self.path} block does not declare {key}",', 'the {path} block does not declare {key}'],
  ]],
  [SCHEMA, 'field_missing', 297, 'text', 1015, 'def text(self, name: str, *, required: bool = True) -> str | None:', [
    [1025, 'f"{name} is required",', '{name} is required'],
  ]],
  [SCHEMA, 'field_type', 298, 'text', 1015, 'def text(self, name: str, *, required: bool = True) -> str | None:', [
    [1032, 'f"{name} must be a string, was {type(value).__name__}",', '{name} must be a string, was {type}'],
    [1058, 'f"{name} must be a bool, was {type(value).__name__}",', '{name} must be a bool, was {type}'],
    [1106, 'f"{name} must be a number, was {type(value).__name__}",', '{name} must be a number, was {type}'],
    [1081, 'f"{name} must be an int, was {type(value).__name__}",', '{name} must be an int, was {type}'],
    [1134, 'f"{name} must be an int, was {number}",', '{name} must be an int, was {number}'],
    [1196, 'f"{name} must be a sequence, was {type(value).__name__}",', '{name} must be a sequence, was {type}'],
    [1206, 'f"{name} entries must be non-empty strings, was {type(item).__name__}",', '{name} entries must be non-empty strings, was {type}'],
    [1333, 'f"declared must be a mapping of aliases, was {type(payload).__name__}",', 'declared must be a mapping of aliases, was {type}'],
    [1352, 'f"a feature alias must be a non-empty string, was {type(alias).__name__}",', 'a feature alias must be a non-empty string, was {type}'],
  ]],
  [SCHEMA, 'field_empty', 299, 'text', 1015, 'def text(self, name: str, *, required: bool = True) -> str | None:', [
    [1036, 'f"{name} must not be empty"', '{name} must not be empty'],
  ]],
  [SCHEMA, 'field_repeated', 300, 'identifiers', 1179, 'def identifiers(self, name: str, *, required: bool = False) -> tuple[str, ...]:', [
    [1213, 'f"{name} states {item} more than once",', '{name} states {item} more than once'],
  ]],
  [SCHEMA, 'value_not_declared', 301, 'declared', 1140, 'def declared(self, name: str, set_name: str, *, required: bool = True) -> str | None:', [
    [1153, `f"{name} must be one of {', '.join(sorted(declared))}, was {value}",`, "{name} must be one of {sorted set}, was {value}"],
    [1172, `f"{name} entries must be one of {', '.join(sorted(declared))}, was {value}",`, '{name} entries must be one of {sorted set}, was {value}'],
  ]],
  [SCHEMA, 'bound_not_declared', 302, 'bound', 1262, 'def bound(self, bound_name: str) -> Bound | None:', [
    [1269, 'f"the policy declares no bound named {bound_name}, so the {self.path} block "', 'the policy declares no bound named {bound_name}, so the {path} block cannot admit a value'],
  ]],
  [SCHEMA, 'value_out_of_bound', 303, 'bounded', 1087, 'def bounded(self, name: str, bound_name: str, *, required: bool = True) -> Decimal | None:', [
    [1117, 'f"{name} must be {bound.describe()}, was {number}",', '{name} must be {bound.describe()}, was {number}'],
  ]],
  [SCHEMA, 'feature_not_allowed', 304, '_read_features', 1318, 'def _read_features(reader: _Reader) -> FeaturesBlock:', [
    [1362, `f"feature must be one of the policy's registered features, {feature_id} is not",`, "feature must be one of the policy's registered features, {feature_id} is not"],
  ]],
  [SCHEMA, 'feature_not_declared', 305, 'feature_ref', 1220, 'def feature_ref(', [
    [1237, 'f"{name} must name a declared feature, {value} is not declared",', '{name} must name a declared feature, {value} is not declared'],
  ]],
  [SCHEMA, 'feature_count_exceeded', 306, '_read_features', 1318, 'def _read_features(reader: _Reader) -> FeaturesBlock:', [
    [1342, 'f"a specification declares at most {limit} features, was {len(payload)}",', 'a specification declares at most 8 features, was {n}'],
  ]],
  [SCHEMA, 'condition_count_exceeded', 307, '_read_entry', 1531, 'def _read_entry(reader: _Reader, declared: frozenset[str]) -> EntryBlock:', [
    [1551, 'f"an entry block states at most {limit} conditions, was {leaves}",', 'an entry block states at most 4 conditions, was {leaves}'],
  ]],
  [SCHEMA, 'condition_depth_exceeded', 308, '_read_condition', 1447, 'def _read_condition(payload: object, walk: _ConditionWalk) -> ConditionNode | None:', [
    [1454, 'f"a condition tree nests at most {limit.max_condition_depth} levels, was {walk.depth}",', 'a condition tree nests at most 2 levels, was {depth}'],
  ]],
  [SCHEMA, 'condition_not_resolved', 309, '_read_condition', 1447, 'def _read_condition(payload: object, walk: _ConditionWalk) -> ConditionNode | None:', [
    [1462, 'f"a condition must be a comparison, an all group or an any group, was "', 'a condition must be a comparison, an all group or an any group, was {type}'],
    [1490, '"a condition states all or any, not both",', 'a condition states all or any, not both'],
    [1500, 'f"the {group} group must hold at least one condition",', 'the {group} group must hold at least one condition'],
  ]],
  [SCHEMA, 'reference_unresolved', 310, 'reference', 1243, 'def reference(self, name: str, *, required: bool = True) -> str | None:', [
    [1255, 'f"{name} must name a frozen reference, {value} is not declared"', '{name} must name a frozen reference, {value} is not declared'],
  ]],
  [SCHEMA, 'risk_field_misplaced', 311, '_refuse_misplaced_risk_fields', 1636, 'def _refuse_misplaced_risk_fields(reader: _Reader) -> None:', [
    [1649, `f"{name} is a {other.replace('_', '-')} field and does not control "`, '{name} is a {other-dashed} field and does not control behaviour, so it belongs under {other}'],
  ]],
  [SCHEMA, 'code_reference_refused', 312, 'scan_for_code_references', 486, 'def scan_for_code_references(payload: object, path: str = "") -> list[Refusal]:', [
    [515, 'f"a specification is data and may not declare {key}",', 'a specification is data and may not declare {key}'],
    [533, 'f"a specification is data and may not carry a {type(payload).__name__}",', 'a specification is data and may not carry a {type}'],
    [544, 'f"a specification value may not be executable code, {payload!r} is",', 'a specification value may not be executable code, {payload!r} is'],
  ]],
  [VALIDATE, 'feature_unresolved', 313, '_resolve_features', 280, 'def _resolve_features(', [
    [310, 'f"the registry holds no definition for {declaration.feature_id}, so the alias "', 'the registry holds no definition for {feature_id}, so the alias {alias} selects a feature the run cannot serve'],
  ]],
  [VALIDATE, 'selection_incomplete', 314, '_selection_refusals', 348, 'def _selection_refusals(selection: SelectionBlock) -> list[Refusal]:', [
    [373, 'f"the compiler checks the selection modes {RANK_MODE}, {THRESHOLD_MODE} and "', 'the compiler checks the selection modes rank, threshold and none, and cannot check {mode}'],
    [400, '"a rank selection states exactly one of percentile and count",', 'a rank selection states exactly one of percentile and count'],
    [951, 'Refusal(code, path, f"a {subject} requires {name}")', 'a {subject} requires {name}'],
    [983, 'Refusal(code, path, f"a {subject} states no {name}")', 'a {subject} states no {name}'],
  ]],
  [VALIDATE, 'exit_incomplete', 315, '_exit_refusals', 485, 'def _exit_refusals(exit_block: ExitBlock, selection_mode: str) -> list[Refusal]:', [
    [509, '"an exit states a feature with both comparison and threshold, or states no feature",', 'an exit states a feature with both comparison and threshold, or states no feature'],
    [518, 'f"rank_exit_count is resolved against a ranking, and the selection mode is "', 'rank_exit_count is resolved against a ranking, and the selection mode is {mode}'],
  ]],
  [VALIDATE, 'rebalance_timing_mismatch', 316, '_rebalance_refusals', 555, 'def _rebalance_refusals(data: DataBlock, rebalance: RebalanceBlock) -> list[Refusal]:', [
    [578, 'f"the rebalance timing {rebalance.timing} must equal the execution eligibility "', 'the rebalance timing {timing} must equal the execution eligibility {eligibility}, so a rebalance is eligible when the data block says its intent is'],
  ]],
  [VALIDATE, 'rebalance_frequency_mismatch', 317, '_rebalance_refusals', 555, 'def _rebalance_refusals(data: DataBlock, rebalance: RebalanceBlock) -> list[Refusal]:', [
    [592, `f"the compiler orders the frequencies {', '.join(FREQUENCY_SESSIONS)}, and cannot "`, 'the compiler orders the frequencies daily, weekly, monthly, and cannot order {freq} against {signal}'],
    [601, 'f"a rebalance at {rebalance.frequency} is finer than the signal at "', 'a rebalance at {freq} is finer than the signal at {signal}, so the candidate would rebalance between decisions'],
  ]],
  [VALIDATE, 'sector_input_undeclared', 318, '_portfolio_refusals', 526, 'def _portfolio_refusals(portfolio: PortfolioBlock, policy: DiscoveryPolicy) -> list[Refusal]:', [
    [547, '"max_sector_weight is applied through the instrument-to-sector mapping, and the "', 'max_sector_weight is applied through the instrument-to-sector mapping, and the policy declares no sector for any instrument'],
  ]],
  [VALIDATE, 'feature_unavailable_at_decision', 319, '_validate_temporal', 609, 'def _validate_temporal(', [
    [723, 'f"the {frequency} signal interval of {interval_ns} nanoseconds, so the value is "', 'the definition of {feature_id} is available {lag} nanoseconds after it is observed, at or beyond the {frequency} signal interval of {interval} nanoseconds, so the value is not knowable at the decision instant'],
  ]],
  [VALIDATE, 'undeferred_execution', 320, '_validate_temporal', 609, 'def _validate_temporal(', [
    [661, 'f"the compiler orders the instants {ordered}, which do not hold {data.decision_time}",', 'the compiler orders the instants {order}, which do not hold {instant}'],
    [676, 'f"the execution eligibility {data.execution_eligibility} must be strictly later than "', 'the execution eligibility {eligibility} must be strictly later than the decision time {decision}, or the candidate decides at one instant and fills at it'],
  ]],
].map(refusalRow);

/** The three places a value may resolve from, in the order the language admits them. */
export const RESOLUTION_PLACES = fact(
  [
    "the pre-registered policy's declared sets, bounds and frozen references",
    "the candidate's own declared feature aliases",
    "the run's feature stream, and only at run time",
  ],
  TUTORIAL,
  77,
  "4. Resolve every value from one of three places only. The first is the pre-registered policy's",
);

/** The keys the behavioural digest of a compiled plan is composed from. */
export const COMPILE_DIGEST_KEYS = fact(
  ['schema', 'specification', 'policy', 'registry', 'features', 'assumptions', 'engine'],
  COMPILE,
  291,
  '{\n            "schema": SPECIFICATION_SCHEMA_VERSION,\n            "specification": specification_digest,\n' +
    '            "policy": policy_digest,\n            "registry": registry_digest,\n' +
    '            "features": "|".join(definition.digest() for definition in definitions),\n' +
    '            "assumptions": assumptions_digest,\n            "engine": engine,\n        }',
);

/**
 * The two inputs of a compiled plan this console cannot reproduce.
 *
 * The assumptions digest names a bar-ambiguity policy the discovery package declares, and the engine
 * version is read from the installed package. Neither exists in the browser, so the plan reports them
 * as not modelled rather than substituting a value.
 */
export const NOT_MODELLED_INPUTS = [
  fact(
    'the assumptions digest of the declared bar-ambiguity policy',
    COMPILE,
    284,
    'assumptions_digest = BarAmbiguityPolicy.declared_default().digest',
  ),
  fact(
    'the engine version read from the installed package',
    COMPILE,
    285,
    'engine = engine_version()',
  ),
];
