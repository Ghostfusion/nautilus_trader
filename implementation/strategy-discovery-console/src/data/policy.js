/**
 * The pre-registered policy, transcribed.
 *
 * This module models `python/nautilus_trader/discovery/policy.py` and the readers in
 * `python/nautilus_trader/discovery/schema.py` that resolve a value through it: the fifteen declared
 * sets with every member, the nineteen first-release bounds with their limits, the three reference
 * kinds, the sector map, the complexity caps, and the refusal codes a resolution failure carries.
 *
 * Three things the source does not state are marked here rather than invented:
 *
 * - the code stores bare `Decimal`s and names no unit, so `unit` is the phrase the tutorial's bound
 *   table uses for the field, quoted from the tutorial page;
 * - the sector map, the feature allowlist and the references are caller inputs, and `first_release`
 *   declares no sector for any instrument, so `SECTORS` transcribes the empty default rather than a
 *   made-up instrument;
 * - a malformed policy makes `policy.py` raise at construction rather than return a refusal, so the
 *   construction rules are transcribed as rules and the engine reports them with the closed
 *   vocabulary's code whose rule the failure breaks (see `src/engine/policy.js`).
 *
 * Every value below is wrapped in `fact(...)`; `tools/check-facts.mjs` fails when the supporting
 * text has left the file it names.
 */

import { fact } from './source.js';

const POLICY = 'python/nautilus_trader/discovery/policy.py';
const SCHEMA = 'python/nautilus_trader/discovery/schema.py';
const VALIDATE = 'python/nautilus_trader/discovery/validate.py';
const TUTORIAL = 'tutorials/strategy-discovery/the-rules-nobody-may-choose/README.md';

/** The stable identity of the pre-registered policy. */
export const POLICY_ID = fact('discovery.policy', POLICY, 53, 'POLICY_ID = "discovery.policy"');

/** The policy version, which changes when any declaration above it changes. */
export const POLICY_VERSION = fact(1, POLICY, 54, 'POLICY_VERSION = 1');

/** The length of a `sha256:` digest: the scheme, the separator and the 64 hex digits. */
export const SHA256_DIGEST_LENGTH = fact(71, POLICY, 57, '_SHA256_DIGEST_LENGTH = 71');

/** The declared-set names, which are also the names `DiscoveryPolicy.declared` accepts. */
export const DECLARED_SET_NAMES = fact(
  [
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
  ],
  POLICY,
  61,
  'DECLARED_SET_NAMES: tuple[str, ...] = (\n    "families",\n    "asset_classes",\n' +
    '    "membership_policies",\n    "signal_frequencies",\n    "decision_times",\n' +
    '    "execution_eligibilities",\n    "corporate_action_treatments",\n    "entry_directions",\n' +
    '    "operators",\n    "selection_modes",\n    "tie_breaks",\n    "weighting_methods",\n' +
    '    "cash_policies",\n    "rebalance_frequencies",\n    "rebalance_timings",\n)',
);

/**
 * One declared set: the name it is declared under, and every member the first release declares.
 *
 * The member order is the order of the `frozenset` literal, which is the transcription's order; the
 * engine sorts members where the source sorts them, which is in the canonical form only.
 */
function declaredSet(name, nameLine, setLine, literal, members) {
  return {
    name: fact(name, POLICY, nameLine, `"${name}",`),
    declared: fact(literal, POLICY, setLine, literal),
    members: members.map((member) => fact(member, POLICY, setLine, literal)),
  };
}

/** The fifteen declared sets of the first-release scope, in `DECLARED_SET_NAMES` order. */
export const DECLARED_SETS = [
  declaredSet('families', 62, 526, 'families=frozenset({"cross_sectional_rank", "absolute_threshold", "static_hold"}),', [
    'cross_sectional_rank',
    'absolute_threshold',
    'static_hold',
  ]),
  declaredSet('asset_classes', 63, 527, 'asset_classes=frozenset({"EQUITY", "ETF"}),', ['EQUITY', 'ETF']),
  declaredSet('membership_policies', 64, 528, 'membership_policies=frozenset({"point_in_time"}),', [
    'point_in_time',
  ]),
  declaredSet('signal_frequencies', 65, 529, 'signal_frequencies=frozenset({"daily"}),', ['daily']),
  declaredSet('decision_times', 66, 530, 'decision_times=frozenset({"session_close", "session_open"}),', [
    'session_close',
    'session_open',
  ]),
  declaredSet(
    'execution_eligibilities',
    67,
    531,
    'execution_eligibilities=frozenset({"same_session_close", "next_session_open"}),',
    ['same_session_close', 'next_session_open'],
  ),
  declaredSet('corporate_action_treatments', 68, 532, 'corporate_action_treatments=frozenset({"adjusted", "raw"}),', [
    'adjusted',
    'raw',
  ]),
  declaredSet('entry_directions', 69, 533, 'entry_directions=frozenset({"long"}),', ['long']),
  declaredSet('operators', 70, 534, 'operators=frozenset({"gt", "ge", "lt", "le"}),', ['gt', 'ge', 'lt', 'le']),
  declaredSet('selection_modes', 71, 535, 'selection_modes=frozenset({"rank", "threshold", "none"}),', [
    'rank',
    'threshold',
    'none',
  ]),
  declaredSet('tie_breaks', 72, 536, 'tie_breaks=frozenset({"feature_desc", "instrument_id_asc"}),', [
    'feature_desc',
    'instrument_id_asc',
  ]),
  declaredSet('weighting_methods', 73, 537, 'weighting_methods=frozenset({"equal", "rank_proportional"}),', [
    'equal',
    'rank_proportional',
  ]),
  declaredSet('cash_policies', 74, 538, 'cash_policies=frozenset({"leave_idle", "reallocate"}),', [
    'leave_idle',
    'reallocate',
  ]),
  declaredSet('rebalance_frequencies', 75, 539, 'rebalance_frequencies=frozenset({"daily", "weekly", "monthly"}),', [
    'daily',
    'weekly',
    'monthly',
  ]),
  declaredSet('rebalance_timings', 76, 540, 'rebalance_timings=frozenset({"next_session_open"}),', [
    'next_session_open',
  ]),
];

/** The two forms a declared bound takes: a contiguous range, or an enumerated choice set. */
export const BOUND_KINDS = [
  fact('range', SCHEMA, 111, 'RANGE = "range"'),
  fact('choice', SCHEMA, 112, 'CHOICE = "choice"'),
];

/** How a bound is restated in the message of a refusal, one form per bound kind. */
export const BOUND_DESCRIBE = [
  fact(
    'one of ',
    SCHEMA,
    230,
    'return "one of " + ", ".join(str(choice) for choice in self.choices)',
  ),
  fact(
    'to upper inclusive',
    SCHEMA,
    232,
    'return f"{self.lower} to {self.upper} inclusive"',
  ),
];

/**
 * One bound: the field it is named after, its two inclusive limits, and the tutorial's description
 * of that field. `unit` is a quotation, not a unit the code declares.
 */
function bound(name, line, literal, lower, upper, unitLine, unit) {
  return {
    name: fact(name, POLICY, line, literal),
    kind: fact('range', SCHEMA, 111, 'RANGE = "range"'),
    lower: fact(lower, POLICY, line, literal),
    upper: fact(upper, POLICY, line, literal),
    unit: fact(unit, TUTORIAL, unitLine, unit),
  };
}

/** The nineteen first-release bounds, in the order `_FIRST_RELEASE_BOUNDS` declares them. */
export const BOUNDS = [
  bound(
    'feature.lookback',
    552,
    'Bound("feature.lookback", BoundKind.RANGE, lower=Decimal(1), upper=Decimal(252)),',
    1,
    252,
    115,
    'how many periods a feature looks back',
  ),
  bound(
    'feature.skip',
    553,
    'Bound("feature.skip", BoundKind.RANGE, lower=Decimal(0), upper=Decimal(5)),',
    0,
    5,
    116,
    'periods skipped before the lookback',
  ),
  bound(
    'selection.percentile',
    554,
    'Bound("selection.percentile", BoundKind.RANGE, lower=Decimal("0.01"), upper=Decimal(1)),',
    0.01,
    1,
    117,
    'the share cut off by a rank selection',
  ),
  bound(
    'selection.count',
    555,
    'Bound("selection.count", BoundKind.RANGE, lower=Decimal(1), upper=Decimal(50)),',
    1,
    50,
    118,
    'how many instruments are selected',
  ),
  bound(
    'selection.threshold',
    556,
    'Bound( "selection.threshold", BoundKind.RANGE, lower=Decimal(-1_000_000), upper=Decimal(1_000_000), ),',
    -1000000,
    1000000,
    119,
    'the level a numeric selection compares',
  ),
  bound(
    'condition.value',
    562,
    'Bound("condition.value", BoundKind.RANGE, lower=Decimal(-1_000_000), upper=Decimal(1_000_000)),',
    -1000000,
    1000000,
    120,
    'the level a condition compares against',
  ),
  bound(
    'exit.holding_period_cap',
    563,
    'Bound("exit.holding_period_cap", BoundKind.RANGE, lower=Decimal(1), upper=Decimal(252)),',
    1,
    252,
    121,
    'the longest a position may be kept',
  ),
  bound(
    'exit.rank_exit_count',
    564,
    'Bound("exit.rank_exit_count", BoundKind.RANGE, lower=Decimal(1), upper=Decimal(50)),',
    1,
    50,
    122,
    'how far a rank may fall before exit',
  ),
  bound(
    'exit.stop_fraction',
    565,
    'Bound("exit.stop_fraction", BoundKind.RANGE, lower=Decimal("0.001"), upper=Decimal("0.5")),',
    0.001,
    0.5,
    123,
    'the fall that triggers an exit',
  ),
  bound(
    'exit.target_fraction',
    566,
    'Bound("exit.target_fraction", BoundKind.RANGE, lower=Decimal("0.001"), upper=Decimal(5)),',
    0.001,
    5,
    124,
    'the rise that triggers an exit',
  ),
  bound(
    'portfolio.position_count',
    567,
    'Bound("portfolio.position_count", BoundKind.RANGE, lower=Decimal(1), upper=Decimal(50)),',
    1,
    50,
    125,
    'how many positions are held at once',
  ),
  bound(
    'portfolio.max_position_weight',
    568,
    'Bound( "portfolio.max_position_weight", BoundKind.RANGE, lower=Decimal("0.005"), upper=Decimal("0.25"), ),',
    0.005,
    0.25,
    126,
    'the largest share in one position',
  ),
  bound(
    'portfolio.max_sector_weight',
    574,
    'Bound("portfolio.max_sector_weight", BoundKind.RANGE, lower=Decimal("0.1"), upper=Decimal(1)),',
    0.1,
    1,
    127,
    'the largest share in one sector',
  ),
  bound(
    'risk.max_gross_exposure',
    575,
    'Bound("risk.max_gross_exposure", BoundKind.RANGE, lower=Decimal("0.1"), upper=Decimal(2)),',
    0.1,
    2,
    128,
    'total exposure across all positions',
  ),
  bound(
    'risk.max_drawdown',
    576,
    'Bound("risk.max_drawdown", BoundKind.RANGE, lower=Decimal("0.01"), upper=Decimal("0.9")),',
    0.01,
    0.9,
    129,
    'the fall from a peak that is permitted',
  ),
  bound(
    'risk.max_turnover',
    577,
    'Bound("risk.max_turnover", BoundKind.RANGE, lower=Decimal("0.05"), upper=Decimal(10)),',
    0.05,
    10,
    130,
    'how much of the portfolio is replaced',
  ),
  bound(
    'risk.max_cost_ratio',
    578,
    'Bound("risk.max_cost_ratio", BoundKind.RANGE, lower=Decimal("0.0001"), upper=Decimal("0.1")),',
    0.0001,
    0.1,
    131,
    'costs as a share of the money traded',
  ),
  bound(
    'universe.min_price',
    579,
    'Bound("universe.min_price", BoundKind.RANGE, lower=Decimal(1), upper=Decimal(1000)),',
    1,
    1000,
    132,
    'the lowest price an instrument may have',
  ),
  bound(
    'universe.min_dollar_volume',
    580,
    'Bound( "universe.min_dollar_volume", BoundKind.RANGE, lower=Decimal(0), upper=Decimal(10_000_000_000), ),',
    0,
    10000000000,
    133,
    'the least daily money traded',
  ),
];

/** The three kinds a frozen reference may carry. */
export const REFERENCE_KINDS = [
  fact('benchmark', POLICY, 85, 'BENCHMARK = "benchmark"'),
  fact('cost_model', POLICY, 86, 'COST_MODEL = "cost_model"'),
  fact('validation_policy', POLICY, 87, 'VALIDATION_POLICY = "validation_policy"'),
];

/** What a reference's digest must look like, which is the one rule a reference is checked by. */
export const REFERENCE_DIGEST_RULE = fact(
  'sha256: and 71 characters',
  POLICY,
  190,
  'if not self.digest.startswith("sha256:") or len(self.digest) != _SHA256_DIGEST_LENGTH:',
);

/** The refusal a reference whose digest is not a `sha256:` digest carries. */
export const REFERENCE_DIGEST_REFUSAL = fact(
  'digest must be a sha256 digest, was {digest}',
  POLICY,
  191,
  'raise ValueError(f"digest must be a sha256 digest, was {self.digest}")',
);

/**
 * The sector map as the first release declares it: empty.
 *
 * `sectors` is a caller input (`policy.py:517-518`) and none is invented here, so this transcribes
 * the default an empty map has, which is what makes `has_sectors` false and a sector cap refused.
 */
export const SECTORS = fact([], POLICY, 501, 'sectors: Iterable[tuple[str, str]] = (),');

/** The rule that decides whether a sector cap may be stated at all. */
export const SECTOR_RULE = [
  fact(
    'The instrument-to-sector mapping the campaign declares.',
    POLICY,
    518,
    'The instrument-to-sector mapping the campaign declares.',
  ),
  fact(
    'Whether the policy declares a sector for any instrument.',
    POLICY,
    445,
    'Whether the policy declares a sector for any instrument.',
  ),
];

/** The three complexity caps, which limit expression size rather than any single field. */
export const LIMITS = {
  max_conditions: fact(4, POLICY, 113, 'max_conditions: int = 4'),
  max_condition_depth: fact(2, POLICY, 114, 'max_condition_depth: int = 2'),
  max_features: fact(8, POLICY, 115, 'max_features: int = 8'),
};

/**
 * The policy's own construction rules, which `policy.py` raises on rather than reporting as data.
 *
 * Each value is the message the source raises with, so the engine can report the rule a malformed
 * policy breaks without inventing a message, and the key is the rule rather than a line number.
 */
export const POLICY_RULES = {
  declaredSetType: fact(
    '{name} must be a frozenset, was {type}',
    POLICY,
    308,
    'raise TypeError(f"{name} must be a frozenset, was {type(declared).__name__}")',
  ),
  declaredSetEmpty: fact('{name} must not be empty', POLICY, 310, 'raise ValueError(f"{name} must not be empty")'),
  declaredSetNames: fact(
    '{name} must hold non-empty names',
    POLICY,
    312,
    'raise ValueError(f"{name} must hold non-empty names")',
  ),
  allowlistType: fact(
    'feature_allowlist must be a frozenset, was {type}',
    POLICY,
    315,
    '"feature_allowlist must be a frozenset, "',
  ),
  allowlistNames: fact(
    'feature_allowlist must hold non-empty identifiers',
    POLICY,
    319,
    'raise ValueError("feature_allowlist must hold non-empty identifiers")',
  ),
  bounds: fact('bounds must be a tuple of Bound', POLICY, 328, 'raise TypeError("bounds must be a tuple of Bound")'),
  references: fact(
    'references must be a tuple of Reference',
    POLICY,
    332,
    'raise TypeError("references must be a tuple of Reference")',
  ),
  limits: fact(
    'limits must be a ComplexityLimits, was {type}',
    POLICY,
    334,
    'raise TypeError(f"limits must be a ComplexityLimits, was {type(self.limits).__name__}")',
  ),
  sectors: fact(
    'sectors must be a tuple, was {type}',
    POLICY,
    336,
    'raise TypeError(f"sectors must be a tuple, was {type(self.sectors).__name__}")',
  ),
  policyId: fact('policy_id must not be empty', POLICY, 343, 'raise ValueError("policy_id must not be empty")'),
  versionType: fact(
    'version must be an int, was {type}',
    POLICY,
    345,
    'raise TypeError(f"version must be an int, was {type(self.version).__name__}")',
  ),
  versionPositive: fact(
    'version must be positive, was {version}',
    POLICY,
    347,
    'raise ValueError(f"version must be positive, was {self.version}")',
  ),
  boundNames: fact('bounds must have distinct names', POLICY, 355, 'raise ValueError("bounds must have distinct names")'),
  referenceNames: fact(
    'references must have distinct names',
    POLICY,
    358,
    'raise ValueError("references must have distinct names")',
  ),
  sectorIds: fact('sectors must name each instrument once', POLICY, 361, 'raise ValueError("sectors must name each instrument once")'),
  capType: fact(
    '{field} must be an int, was {type}',
    POLICY,
    127,
    'raise TypeError(f"{field} must be an int, was {type(value).__name__}")',
  ),
  capPositive: fact('{field} must be positive, was {value}', POLICY, 129, 'raise ValueError(f"{field} must be positive, was {value}")'),
  referenceNameType: fact(
    'name must be a str, was {type}',
    POLICY,
    183,
    'raise TypeError(f"name must be a str, was {type(self.name).__name__}")',
  ),
  referenceKindType: fact(
    'kind must be a ReferenceKind, was {type}',
    POLICY,
    185,
    'raise TypeError(f"kind must be a ReferenceKind, was {type(self.kind).__name__}")',
  ),
  referenceDigestType: fact(
    'digest must be a str, was {type}',
    POLICY,
    187,
    'raise TypeError(f"digest must be a str, was {type(self.digest).__name__}")',
  ),
  referenceName: fact('name must not be empty', POLICY, 189, 'raise ValueError("name must not be empty")'),
};

/** The bounds' own construction rules, which `Bound` raises on rather than reporting as data. */
export const BOUND_RULES = {
  limits: fact(
    'a range bound requires both limits, {name} states {lower} and {upper}',
    SCHEMA,
    175,
    'f"a range bound requires both limits, {self.name} states "',
  ),
  rangeChoices: fact(
    'a range bound states no choices, {name} states choices',
    SCHEMA,
    179,
    'raise ValueError(f"a range bound states no choices, {self.name} states choices")',
  ),
  order: fact(
    'a range bound requires lower <= upper, {name} states {lower} > {upper}',
    SCHEMA,
    182,
    'f"a range bound requires lower <= upper, {self.name} states "',
  ),
  choiceLimits: fact(
    'a choice bound states no limits, {name} states limits',
    SCHEMA,
    191,
    'raise ValueError(f"a choice bound states no limits, {self.name} states limits")',
  ),
  choiceRequired: fact(
    'a choice bound requires choices, {name} states none',
    SCHEMA,
    193,
    'raise ValueError(f"a choice bound requires choices, {self.name} states none")',
  ),
  choiceDistinct: fact(
    'a choice bound requires distinct choices, {name} repeats one',
    SCHEMA,
    195,
    'raise ValueError(f"a choice bound requires distinct choices, {self.name} repeats one")',
  ),
  name: fact('name must not be empty', SCHEMA, 162, 'raise ValueError("name must not be empty")'),
};

/** The message a bound the policy does not declare carries, quoted from the reader that records it. */
export const BOUND_ABSENT_MESSAGE = fact(
  'the policy declares no bound named {bound_name}, so the {path} block cannot admit a value',
  SCHEMA,
  1269,
  'f"the policy declares no bound named {bound_name}, so the {self.path} block "',
);

/** The canonical form's sorted sets, which is why the order a set was written in cannot matter. */
export const CANONICAL_SORTING = [
  fact('sets sorted', POLICY, 461, '"families": sorted(self.families),'),
  fact('allowlist sorted', POLICY, 476, '"feature_allowlist": sorted(self.feature_allowlist),'),
  fact('digest of the canonical form', POLICY, 493, 'return digest_of(canonical_mapping(self.to_dict()))'),
];

/**
 * One refusal code: the code, the message the source states it with, and the reader that records it.
 *
 * `message` is the rule as the refusal phrases it and `messageText` is the line it was read from, so
 * the two can be compared rather than trusted.
 */
function refusalCode(row) {
  return {
    code: fact(row.name, SCHEMA, row.codeLine, `${row.name.toUpperCase()} = "${row.name}"`),
    message: fact(row.message, SCHEMA, row.messageLine, row.messageText),
    where: fact(row.where, SCHEMA, row.whereLine, row.whereText),
  };
}

/** The six refusals a value resolved through the policy's declarations can carry. */
export const REFUSAL_CODES = [
  refusalCode({
    name: 'value_not_declared',
    codeLine: 301,
    message: "{name} must be one of {', '.join(sorted(declared))}, was {value}",
    messageLine: 1153,
    messageText: `f"{name} must be one of {', '.join(sorted(declared))}, was {value}",`,
    where: '_Reader.declared',
    whereLine: 1140,
    whereText: 'def declared(self, name: str, set_name: str, *, required: bool = True) -> str | None:',
  }),
  refusalCode({
    name: 'bound_not_declared',
    codeLine: 302,
    message: 'the policy declares no bound named {bound_name}, so the {path} block cannot admit a value',
    messageLine: 1269,
    messageText: 'f"the policy declares no bound named {bound_name}, so the {self.path} block "',
    where: '_Reader.bound',
    whereLine: 1262,
    whereText: 'def bound(self, bound_name: str) -> Bound | None:',
  }),
  refusalCode({
    name: 'value_out_of_bound',
    codeLine: 303,
    message: '{name} must be {bound.describe()}, was {number}',
    messageLine: 1117,
    messageText: 'f"{name} must be {bound.describe()}, was {number}",',
    where: '_Reader.bounded',
    whereLine: 1087,
    whereText: 'def bounded(self, name: str, bound_name: str, *, required: bool = True) -> Decimal | None:',
  }),
  refusalCode({
    name: 'feature_not_allowed',
    codeLine: 304,
    message: "feature must be one of the policy's registered features, {feature_id} is not",
    messageLine: 1362,
    messageText: `f"feature must be one of the policy's registered features, {feature_id} is not",`,
    where: '_read_features',
    whereLine: 1318,
    whereText: 'def _read_features(reader: _Reader) -> FeaturesBlock:',
  }),
  refusalCode({
    name: 'feature_not_declared',
    codeLine: 305,
    message: '{name} must name a declared feature, {value} is not declared',
    messageLine: 1237,
    messageText: 'f"{name} must name a declared feature, {value} is not declared",',
    where: '_Reader.feature_ref',
    whereLine: 1220,
    whereText: 'def feature_ref(',
  }),
  refusalCode({
    name: 'reference_unresolved',
    codeLine: 310,
    message: '{name} must name a frozen reference, {value} is not declared',
    messageLine: 1255,
    messageText: 'f"{name} must name a frozen reference, {value} is not declared"',
    where: '_Reader.reference',
    whereLine: 1243,
    whereText: 'def reference(self, name: str, *, required: bool = True) -> str | None:',
  }),
];

/** The stage-5 side of the freeze: the policy digest enters the behavioural digest of every plan. */
export const POLICY_DIGEST_USES = [
  fact('the policy digest of a compiled plan', 'python/nautilus_trader/discovery/compile.py', 287, 'policy_digest = policy.digest()'),
  fact(
    'the policy digest inside the behavioural digest',
    'python/nautilus_trader/discovery/compile.py',
    294,
    '"policy": policy_digest,',
  ),
];

/** The scope sentence the first-release sets are also the statement of. */
export const FIRST_RELEASE_SCOPE = fact(
  'long only, daily signals, weekly or slower rebalance, equities and ETFs, and no shorting, options, intraday logic or fundamental input',
  POLICY,
  506,
  'The scope is decision SD12: long only, daily signals, weekly or slower rebalance, equities',
);

/** The stage-3 rule that refuses a sector cap when the policy declares no sector. */
export const SECTOR_REFUSAL = fact(
  'max_sector_weight is applied through the instrument-to-sector mapping, and the policy declares no sector for any instrument',
  VALIDATE,
  547,
  '"max_sector_weight is applied through the instrument-to-sector mapping, and the "',
);
