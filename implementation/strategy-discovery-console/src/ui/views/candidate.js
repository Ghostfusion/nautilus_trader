/**
 * What a candidate is.
 *
 * This view models `python/nautilus_trader/discovery/schema.py`, the four stages of
 * `python/nautilus_trader/discovery/validate.py` and `python/nautilus_trader/discovery/compile.py`
 * through `src/engine/candidate.js`. The reader builds a specification out of the twelve blocks, one
 * control per block, then breaks one thing at a time and watches the pipeline refuse it with the
 * field named, the stage recorded and the code the source uses.
 *
 * The feature allowlist, the registry of definitions, the references and the sector map are campaign
 * inputs the source invents none of, so the ones this model resolves against are the console's own
 * and say so. Two of a compiled plan's inputs have no counterpart in a browser - the assumptions
 * digest of the bar-ambiguity policy, and the engine version - and are reported as not modelled.
 */

import { FORMER_FAILURES } from '../../data/acceptance.js';
import { REFUSAL_CODES as REFUSAL_FACTS } from '../../data/candidate.js';
import { cite, valueOf } from '../../data/source.js';
import {
  BLOCKS,
  CAPS,
  NOT_MODELLED,
  STAGES,
  WARNING_CODES,
  conditionDepth,
  countConditionLeaves,
  validateSpecification,
} from '../../engine/candidate.js';
import { defaultPolicy } from '../../engine/policy.js';

/** The layer this view is: its id, the tutorial page's own title, and its position in the pipeline. */
export const meta = {
  id: 'candidate',
  title: 'What a candidate is',
  order: 2,
  tutorial: 'tutorials/strategy-discovery/what-a-candidate-is/README.md',
};

/** The values the builder starts from: one legal candidate, written field by field. */
const DEFAULTS = {
  version: 'discovery.specification/v1',
  family: 'cross_sectional_rank',
  minPrice: 5,
  eligibility: 'next_session_open',
  alias: 'trend',
  featureId: 'close',
  lookback: 20,
  selectionMode: 'rank',
  selectionFeature: 'trend',
  percentile: 0.1,
  tieBreak: 'feature_desc',
  entryShape: 'two_comparisons',
  rankExitCount: 'none',
  positionCount: 20,
  maxPositionWeight: 0.2,
  frequency: 'weekly',
  benchmark: 'console_benchmark',
  generator: 'hand-written',
};

/** The condition trees the entry control writes, including the one the tutorial counts by hand. */
const ENTRY_SHAPES = {
  none: () => null,
  one_comparison: (alias) => ({ all: [leaf(alias)] }),
  two_comparisons: (alias) => ({ all: [leaf(alias), leaf(alias)] }),
  cap_breach: (alias) => ({ all: [leaf(alias), leaf(alias), { any: [leaf(alias), leaf(alias), leaf(alias)] }] }),
  cap_leaves: (alias) => ({ all: [leaf(alias), leaf(alias), leaf(alias), leaf(alias), leaf(alias)] }),
  both_groups: (alias) => ({ all: [leaf(alias)], any: [leaf(alias)] }),
  empty_group: () => ({ any: [] }),
};

/** One comparison inside an entry tree. */
function leaf(alias) {
  return { feature: alias, comparison: 'gt', value: 0 };
}

/**
 * The one thing the reader breaks, applied to the built specification or to the policy.
 *
 * Each entry says which field it writes and which rule the source has for it, so a refusal shown in
 * the model can be read beside the defect that caused it.
 */
const DEFECTS = [
  { id: 'none', label: 'nothing: the candidate is legal', field: null, rule: null },
  { id: 'min_price_above', label: 'universe.min_price declares 2500', field: 'universe.min_price', rule: 'the bound is 1 to 1000 inclusive' },
  { id: 'family_undeclared', label: 'identity.family declares a name no set holds', field: 'identity.family', rule: 'the declared set decides, not the candidate' },
  { id: 'eligibility_at_decision', label: 'data.execution_eligibility fills at the decision instant', field: 'data.execution_eligibility', rule: 'the fill must be strictly later than the decision' },
  { id: 'eligibility_unspecified', label: 'data.execution_eligibility declares an instant no declared set holds', field: 'data.execution_eligibility', rule: 'the declared set decides' },
  { id: 'eligibility_unordered', label: 'data.execution_eligibility declares an instant the order table does not hold, with the policy declaring it too', field: 'data.execution_eligibility', rule: 'the compiler orders the instants, and an unordered one is refused' },
  { id: 'frequency_undeclared', label: 'rebalance.frequency declares hourly', field: 'rebalance.frequency', rule: 'the declared set decides' },
  { id: 'frequency_unorderable', label: 'rebalance.frequency declares hourly, with the policy declaring it too', field: 'rebalance.frequency', rule: 'the compiler orders daily, weekly and monthly' },
  { id: 'lookback_above', label: 'features.declared.trend.lookback declares 300', field: 'features.declared.trend.lookback', rule: 'the bound is 1 to 252 inclusive' },
  { id: 'feature_not_allowed', label: 'the declared feature is not on the campaign allowlist', field: 'features.declared.trend.feature', rule: 'the feature must be one the policy registers' },
  { id: 'feature_unresolved', label: 'the registry holds no definition for the declared feature', field: 'features.declared.trend.feature', rule: 'stage two resolves the alias against the registry' },
  { id: 'feature_late', label: 'the definition is available a session after it is observed', field: 'features.declared.trend.feature', rule: 'a value must be knowable at the decision instant' },
  { id: 'nine_features', label: 'nine features are declared', field: 'features.declared', rule: 'at most 8 features' },
  { id: 'condition_cap', label: 'the entry tree nests three levels deep', field: 'entry.conditions.all.2.any', rule: 'at most 2 levels of nesting' },
  { id: 'condition_leaves', label: 'the entry tree holds five comparisons two levels deep', field: 'entry.conditions', rule: 'at most 4 leaf conditions' },
  { id: 'condition_both_groups', label: 'a condition group states all and any', field: 'entry.conditions.all', rule: 'a condition states all or any, not both' },
  { id: 'condition_empty', label: 'an any group holds no condition', field: 'entry.conditions.any', rule: 'a group must hold at least one condition' },
  { id: 'unused_alias', label: 'the declared alias nothing references', field: 'features.declared.trend', rule: 'a warning, not a refusal' },
  { id: 'warmup', label: 'the lookback dominates the rebalance interval', field: 'features.declared.trend.lookback', rule: 'a warning, not a refusal' },
  { id: 'selection_no_feature', label: 'a rank selection states no feature', field: 'selection.feature', rule: 'a rank selection requires its feature and its tie-break' },
  { id: 'selection_both_bounds', label: 'a rank selection states both percentile and count', field: 'selection.percentile', rule: 'a rank selection states exactly one of the two' },
  { id: 'selection_none_with_feature', label: 'a selection that ranks nothing states a feature', field: 'selection.feature', rule: 'a none selection states no ranking field' },
  { id: 'rank_exit_without_rank', label: 'an exit states a rank exit beside a threshold selection', field: 'exit.rank_exit_count', rule: 'rank_exit_count is resolved against a ranking' },
  { id: 'exit_feature_incomplete', label: 'an exit states a feature without its comparison', field: 'exit.feature', rule: 'a feature comes with both comparison and threshold, or with neither' },
  { id: 'rebalance_timing', label: 'rebalance.timing disagrees with the execution eligibility, with the policy declaring the timing too', field: 'rebalance.timing', rule: 'the two must be equal' },
  { id: 'sector_cap', label: 'portfolio.max_sector_weight is stated with no sector map', field: 'portfolio.max_sector_weight', rule: 'the policy declares no sector for any instrument' },
  { id: 'position_cap', label: 'twenty positions at a weight of 0.05', field: 'portfolio.max_position_weight', rule: 'a warning: below fully invested' },
  { id: 'benchmark_undeclared', label: 'evaluation.benchmark names a reference the policy does not hold', field: 'evaluation.benchmark', rule: 'a reference pins a fingerprint' },
  { id: 'code_field', label: 'provenance declares a module_path field', field: 'provenance.module_path', rule: 'a field naming code is refused by name' },
  { id: 'code_value', label: 'provenance.generator carries a module:attribute value', field: 'provenance.generator', rule: 'a value shaped like code is refused by shape' },
  { id: 'block_missing', label: 'the portfolio block is deleted', field: 'portfolio', rule: 'all twelve blocks are required' },
  { id: 'unknown_field', label: 'the portfolio block declares a field no block declares', field: 'portfolio.something_else', rule: 'a block refuses every key it did not read' },
  { id: 'repeated_asset_class', label: 'universe.asset_classes states EQUITY twice', field: 'universe.asset_classes', rule: 'a repeated identifier is refused' },
  { id: 'version_unsupported', label: 'the version is one this build does not accept', field: 'version', rule: 'no migration is registered for it' },
  { id: 'version_missing', label: 'there is no version line', field: 'version', rule: 'a specification must state its schema version' },
  { id: 'bound_dropped', label: 'the policy declares no bound named portfolio.position_count', field: 'portfolio.position_count', rule: 'an undeclared bound refuses every value of the field' },
  { id: 'bound_inverted', label: 'the policy declares the min_price bound upside down', field: 'universe.min_price', rule: 'a range bound requires lower <= upper' },
];

/** The console's own campaign inputs, which the source invents none of. */
function consoleInputs() {
  return {
    featureAllowlist: ['close', 'volume'],
    references: [
      { name: 'console_benchmark', kind: 'benchmark', digest: 'sha256:' + 'a'.repeat(64) },
      { name: 'console_cost_model', kind: 'cost_model', digest: 'sha256:' + 'b'.repeat(64) },
      { name: 'console_validation', kind: 'validation_policy', digest: 'sha256:' + 'c'.repeat(64) },
    ],
  };
}

/** The candidate the builder's controls describe, before the chosen defect is applied. */
function buildSpecification(state) {
  const alias = state.alias;
  const entry = {
    direction: 'long',
    conditions: ENTRY_SHAPES[state.entryShape](alias),
  };
  const selection =
    state.selectionMode === 'rank'
      ? {
          mode: 'rank',
          feature: state.selectionFeature,
          percentile: state.percentile,
          tie_break: state.tieBreak,
        }
      : { mode: state.selectionMode };

  return {
    version: state.version,
    identity: {
      family: state.family,
      hypothesis: 'rank the eligible set and hold the leaders',
      mechanism: 'a cross-sectional rank of the declared feature',
      falsification: 'the rank carries no information about forward returns',
    },
    universe: {
      asset_classes: ['EQUITY', 'ETF'],
      membership_policy: 'point_in_time',
      source: 'the declared snapshot',
      min_price: state.minPrice,
    },
    data: {
      signal_frequency: 'daily',
      decision_time: 'session_close',
      execution_eligibility: state.eligibility,
      corporate_action_treatment: 'adjusted',
    },
    features: { declared: { [alias]: { feature: state.featureId, lookback: state.lookback } } },
    selection,
    entry,
    exit: state.rankExitCount === 'none' ? {} : { rank_exit_count: 20 },
    portfolio: {
      position_count: state.positionCount,
      weighting: 'equal',
      max_position_weight: state.maxPositionWeight,
      cash_policy: 'leave_idle',
    },
    rebalance: { frequency: state.frequency, timing: 'next_session_open' },
    risk: {},
    evaluation: {
      benchmark: state.benchmark,
      cost_model: 'console_cost_model',
      validation_policy: 'console_validation',
    },
    provenance: { generator: state.generator, seed: 1 },
  };
}

/** Apply the chosen defect to the specification, the policy and the stage-two inputs. */
function applyDefect(state) {
  const specification = buildSpecification(state);
  const policy = defaultPolicy(consoleInputs());
  const options = { registry: { close: { availability_lag_ns: 0 }, volume: { availability_lag_ns: 0 } } };

  switch (state.defect) {
    case 'min_price_above':
      specification.universe.min_price = 2500;
      break;
    case 'family_undeclared':
      specification.identity.family = 'mean_reversion';
      break;
    case 'eligibility_at_decision':
      specification.data.execution_eligibility = 'same_session_close';
      specification.rebalance.timing = 'same_session_close';
      policy.sets = { ...policy.sets, rebalance_timings: [...policy.sets.rebalance_timings, 'same_session_close'] };
      break;
    case 'eligibility_unspecified':
      specification.data.execution_eligibility = 'next_session_auction';
      break;
    case 'eligibility_unordered':
      specification.data.execution_eligibility = 'next_session_auction';
      specification.rebalance.timing = 'next_session_auction';
      policy.sets = {
        ...policy.sets,
        execution_eligibilities: [...policy.sets.execution_eligibilities, 'next_session_auction'],
        rebalance_timings: [...policy.sets.rebalance_timings, 'next_session_auction'],
      };
      break;
    case 'frequency_undeclared':
      specification.rebalance.frequency = 'hourly';
      break;
    case 'frequency_unorderable':
      specification.rebalance.frequency = 'hourly';
      policy.sets = { ...policy.sets, rebalance_frequencies: [...policy.sets.rebalance_frequencies, 'hourly'] };
      break;
    case 'lookback_above':
      specification.features.declared[state.alias].lookback = 300;
      break;
    case 'feature_not_allowed':
      specification.features.declared[state.alias].feature = 'book_value';
      break;
    case 'feature_unresolved':
      options.registry = {};
      break;
    case 'feature_late':
      options.registry = { close: { availability_lag_ns: 86400000000000 } };
      break;
    case 'nine_features':
      specification.features.declared = Object.fromEntries(
        Array.from({ length: 9 }, (unused, index) => [`alias_${index}`, { feature: 'close', lookback: 20 }]),
      );
      break;
    case 'condition_cap':
      specification.entry.conditions = ENTRY_SHAPES.cap_breach(state.alias);
      break;
    case 'condition_leaves':
      specification.entry.conditions = ENTRY_SHAPES.cap_leaves(state.alias);
      break;
    case 'condition_both_groups':
      specification.entry.conditions = ENTRY_SHAPES.both_groups(state.alias);
      break;
    case 'condition_empty':
      specification.entry.conditions = ENTRY_SHAPES.empty_group();
      break;
    case 'unused_alias':
      specification.entry.conditions = ENTRY_SHAPES.none();
      break;
    case 'warmup':
      specification.features.declared[state.alias].lookback = 21;
      break;
    case 'selection_no_feature':
      delete specification.selection.feature;
      break;
    case 'selection_both_bounds':
      specification.selection.count = 20;
      break;
    case 'selection_none_with_feature':
      specification.selection = { mode: 'none', feature: state.alias };
      break;
    case 'rank_exit_without_rank':
      specification.selection = { mode: 'none' };
      specification.exit = { rank_exit_count: 20 };
      break;
    case 'exit_feature_incomplete':
      specification.exit = { feature: state.alias };
      break;
    case 'rebalance_timing':
      specification.rebalance.timing = 'same_session_close';
      policy.sets = { ...policy.sets, rebalance_timings: [...policy.sets.rebalance_timings, 'same_session_close'] };
      break;
    case 'sector_cap':
      specification.portfolio.max_sector_weight = 0.4;
      break;
    case 'position_cap':
      specification.portfolio.max_position_weight = 0.05;
      break;
    case 'benchmark_undeclared':
      specification.evaluation.benchmark = 'sp_500_total_return';
      break;
    case 'code_field':
      specification.provenance.module_path = 'harmless text';
      break;
    case 'code_value':
      specification.provenance.generator = 'strategies.ema_cross:EMACross';
      break;
    case 'block_missing':
      delete specification.portfolio;
      break;
    case 'unknown_field':
      specification.portfolio.something_else = 1;
      break;
    case 'repeated_asset_class':
      specification.universe.asset_classes = ['EQUITY', 'EQUITY'];
      break;
    case 'version_unsupported':
      specification.version = 'discovery.specification/v0';
      break;
    case 'version_missing':
      delete specification.version;
      break;
    case 'bound_dropped':
      policy.bounds = policy.bounds.filter((bound) => bound.name !== 'portfolio.position_count');
      break;
    case 'bound_inverted':
      policy.bounds = policy.bounds.map((bound) =>
        bound.name === 'universe.min_price' ? { ...bound, lower: 1000, upper: 1 } : bound,
      );
      break;
    default:
      break;
  }

  return { specification, policy, options };
}

/** Build the whole view into the empty root, and wire the view's own handlers. */
export function mount(root, ctx) {
  const { ui, data } = ctx;
  const layer = data.layer(meta.id);

  const state = { ...DEFAULTS, defect: 'none' };
  const results = ui.stack();

  /** Run the five stages over what the controls describe and show the outcome stage by stage. */
  function update() {
    ui.clear(results);
    const { specification, policy, options } = applyDefect(state);
    const validation = validateSpecification(specification, policy, options);
    const index = STAGES.findIndex((stage) => stage.id === validation.stage);
    const reached = validation.ok ? STAGES.length : index + 1;
    const parsed = validation.specification;

    results.appendChild(
      ui.panel(
        { title: 'The pipeline', tone: validation.ok ? 'ok' : 'bad' },
        ui.metrics(
          ...STAGES.map((stage, position) =>
            ui.metric(
              `${stage.order}. ${stage.name}`,
              position + 1 < reached ? 'passed' : position + 1 === reached ? (validation.ok ? 'accepted' : 'refused') : 'not reached',
              stage.implementation,
              position + 1 < reached ? 'ok' : position + 1 === reached ? (validation.ok ? 'ok' : 'bad') : 'muted',
            ),
          ),
        ),
        ui.note(
          `the refusal belongs to ${validation.stage}` +
            (validation.code === null ? '' : `, and the first code is ${validation.code}: ${validation.reason}`),
          validation.ok ? 'ok' : 'bad',
        ),
        ui.table({
          columns: ['Code', 'Path', 'What the stage records'],
          rows: validation.refusals.map((entry) => [entry.code, entry.path, entry.reason]),
          dense: true,
          caption: `the refusals of ${validation.stage}`,
        }),
        ui.table({
          columns: ['Warning', 'Path', 'What it says'],
          rows: validation.warnings.map((entry) => [entry.code, entry.path, entry.reason]),
          dense: true,
          caption: 'warnings are a separate channel and soften no refusal',
        }),
        ui.note(
          `the caps the model counts: ${countConditionLeaves(parsed === null ? null : parsed.entry.conditions)} leaves, depth ${conditionDepth(parsed === null ? null : parsed.entry.conditions)}, against max_conditions ${CAPS().max_conditions} and max_condition_depth ${CAPS().max_condition_depth}; features against max_features ${CAPS().max_features}.`,
          'muted',
        ),
        validation.plan === null ? null : planPanel(validation.plan),
      ),
    );

    results.appendChild(
      ui.note(
        reached < STAGES.length
          ? `the pipeline stopped after ${STAGES[reached - 1].name}: ${STAGES
              .slice(reached)
              .map((stage) => stage.name)
              .join(', ')} never ran, and the payload was never compiled.`
          : 'every stage ran, so the plan above is the compiled candidate.',
        validation.ok ? 'ok' : 'muted',
      ),
    );
  }

  /** The plan an accepted specification compiles into, with the identities it carries. */
  function planPanel(plan) {
    return ui.panel(
      { title: 'The compiled plan', tone: 'ok' },
      ui.table({
        columns: ['Identity', 'Digest'],
        rows: [
          ['specification', ui.digest(plan.specificationDigest, 16)],
          ['behavioural', ui.digest(plan.behaviouralDigest, 16)],
          ['policy', ui.digest(plan.policyDigest, 16)],
          ['registry', ui.digest(plan.registryDigest, 16)],
          ['assumptions', 'not modelled'],
          ['engine', plan.engineVersion],
        ],
      }),
      ui.note(
        `the behavioural digest is composed from the source's own keys: ${plan.digests.keys.join(', ')}. The assumptions digest would be ${NOT_MODELLED[0]} and the engine version ${NOT_MODELLED[1]}, neither of which exists in a browser, so both are reported as not modelled rather than filled in.`,
        'muted',
      ),
      ui.note(
        'these are the console\'s SHA-256 digests over canonically serialised JSON; the repository\'s canonical digest is blake3, and the two are never compared.',
        'muted',
      ),
    );
  }

  /** One labelled control, with the block and field it writes. */
  function control(label, node, help) {
    return ui.field(label, node, help);
  }

  const controls = ui.stack(
    ui.row(
      control('version', ui.selectInput({
        id: 'candidate-version',
        value: state.version,
        options: [
          { value: 'discovery.specification/v1', label: 'discovery.specification/v1' },
          { value: 'discovery.specification/v0', label: 'discovery.specification/v0' },
          { value: '', label: '(no version line)' },
        ],
        onChange: (value) => {
          state.version = value === '' ? undefined : value;
          update();
        },
      }), 'the version line the schema requires'),
      control('the one thing broken', ui.selectInput({
        id: 'candidate-defect',
        value: state.defect,
        options: DEFECTS.map((defect) => ({ value: defect.id, label: defect.label })),
        onChange: (value) => {
          state.defect = value;
          update();
        },
      }), 'each defect names the field it writes and the rule that decides it'),
    ),
    ui.group('identity',
      ui.row(
        control('family', ui.selectInput({
          id: 'candidate-family',
          value: state.family,
          options: ['cross_sectional_rank', 'absolute_threshold', 'static_hold'],
          onChange: (value) => {
            state.family = value;
            update();
          },
        }), 'resolved from the declared set families'),
      ),
    ),
    ui.group('universe',
      ui.row(
        control('min_price', ui.numberInput({
          id: 'candidate-min-price',
          value: state.minPrice,
          step: 'any',
          onInput: (value) => {
            if (Number.isFinite(value)) {
              state.minPrice = value;
              update();
            }
          },
        }), 'bounded by universe.min_price'),
        control('asset_classes', ui.note('EQUITY, ETF', 'muted'), 'each entry resolved from asset_classes'),
      ),
    ),
    ui.group('data',
      ui.row(
        control('signal_frequency', ui.note('daily', 'muted'), 'the only declared frequency'),
        control('decision_time', ui.note('session_close', 'muted'), 'resolved from decision_times'),
        control('execution_eligibility', ui.selectInput({
          id: 'candidate-eligibility',
          value: state.eligibility,
          options: ['next_session_open', 'same_session_close', 'session_open'],
          onChange: (value) => {
            state.eligibility = value;
            update();
          },
        }), 'strictly later than the decision instant'),
      ),
    ),
    ui.group('features',
      ui.row(
        control('alias', ui.textInput({
          id: 'candidate-alias',
          value: state.alias,
          placeholder: 'the name the rest of it uses',
          onInput: (value) => {
            state.alias = value;
            update();
          },
        }), 'read first, because the later blocks refer to it'),
        control('feature', ui.selectInput({
          id: 'candidate-feature',
          value: state.featureId,
          options: ['close', 'volume', 'book_value'],
          onChange: (value) => {
            state.featureId = value;
            update();
          },
        }), 'must be one the campaign allowlist registers'),
        control('lookback', ui.numberInput({
          id: 'candidate-lookback',
          value: state.lookback,
          min: 1,
          max: 252,
          onInput: (value) => {
            if (Number.isFinite(value)) {
              state.lookback = value;
              update();
            }
          },
        }), 'bounded by feature.lookback, 1 to 252 inclusive'),
      ),
    ),
    ui.group('selection',
      ui.row(
        control('mode', ui.selectInput({
          id: 'candidate-mode',
          value: state.selectionMode,
          options: ['rank', 'threshold', 'none'],
          onChange: (value) => {
            state.selectionMode = value;
            update();
          },
        }), 'each mode states its own fields and no others'),
        control('feature', ui.selectInput({
          id: 'candidate-selection-feature',
          value: state.selectionFeature,
          options: ['trend', 'missing'],
          onChange: (value) => {
            state.selectionFeature = value;
            update();
          },
        }), 'must name a declared alias'),
        control('percentile', ui.numberInput({
          id: 'candidate-percentile',
          value: state.percentile,
          step: 'any',
          min: 0.01,
          max: 1,
          onInput: (value) => {
            if (Number.isFinite(value)) {
              state.percentile = value;
              update();
            }
          },
        }), 'bounded by selection.percentile'),
        control('tie_break', ui.selectInput({
          id: 'candidate-tie-break',
          value: state.tieBreak,
          options: ['feature_desc', 'instrument_id_asc'],
          onChange: (value) => {
            state.tieBreak = value;
            update();
          },
        }), 'resolved from tie_breaks'),
      ),
    ),
    ui.group('entry',
      ui.row(
        control('conditions', ui.selectInput({
          id: 'candidate-entry',
          value: state.entryShape,
          options: Object.keys(ENTRY_SHAPES).map((shape) => ({ value: shape, label: shape.replace(/_/g, ' ') })),
          onChange: (value) => {
            state.entryShape = value;
            update();
          },
        }), 'at most 4 leaf comparisons and 2 levels of nesting'),
        control('rank_exit_count', ui.selectInput({
          id: 'candidate-rank-exit',
          value: state.rankExitCount,
          options: [{ value: 'none', label: '(not stated)' }, { value: '20', label: '20' }],
          onChange: (value) => {
            state.rankExitCount = value;
            update();
          },
        }), 'resolved against a ranking, so a rank selection must be declared'),
      ),
    ),
    ui.group('portfolio',
      ui.row(
        control('position_count', ui.numberInput({
          id: 'candidate-position-count',
          value: state.positionCount,
          min: 1,
          max: 50,
          onInput: (value) => {
            if (Number.isFinite(value)) {
              state.positionCount = value;
              update();
            }
          },
        }), 'bounded by portfolio.position_count'),
        control('max_position_weight', ui.numberInput({
          id: 'candidate-position-weight',
          value: state.maxPositionWeight,
          step: 'any',
          min: 0.005,
          max: 0.25,
          onInput: (value) => {
            if (Number.isFinite(value)) {
              state.maxPositionWeight = value;
              update();
            }
          },
        }), 'bounded by portfolio.max_position_weight'),
      ),
    ),
    ui.group('rebalance',
      ui.row(
        control('frequency', ui.selectInput({
          id: 'candidate-frequency',
          value: state.frequency,
          options: ['daily', 'weekly', 'monthly'],
          onChange: (value) => {
            state.frequency = value;
            update();
          },
        }), 'no finer than the signal, and ordered by the compiler'),
        control('timing', ui.note('next_session_open', 'muted'), 'must equal the execution eligibility'),
      ),
    ),
    ui.group('evaluation and provenance',
      ui.row(
        control('benchmark', ui.selectInput({
          id: 'candidate-benchmark',
          value: state.benchmark,
          options: ['console_benchmark', 'sp_500_total_return'],
          onChange: (value) => {
            state.benchmark = value;
            update();
          },
        }), 'must name a reference the policy holds'),
        control('generator', ui.textInput({
          id: 'candidate-generator',
          value: state.generator,
          placeholder: 'where it came from',
          onInput: (value) => {
            state.generator = value;
            update();
          },
        }), 'a bare dotted string is data because nothing resolves it'),
      ),
    ),
  );

  // The five parts, in the order the contract fixes them.
  root.appendChild(
    ui.panel(
      { title: layer.title, subtitle: layer.tutorial },
      ui.block(
        'The idea',
        ui.quote(layer.idea, layer.tutorial),
        ui.verdict('How well it held up', layer.grade, 'ok', null),
        ui.note(
          `the layer cites acceptance criteria ${layer.criteria.join(', ')}; all twenty-two of the record's criteria pass, and the four that used to fail (${FORMER_FAILURES.map((failure) => valueOf(failure.id)).join(', ')}) include none of criterion 1, 3, 19, 20 or 21.`,
          'ok',
        ),
      ),
      ui.block(
        'The model',
        ui.note(
          `${BLOCKS().length} blocks, each answering one question, and one control per block. The five stages run in the source's order and stop at the first that refuses.`,
          'muted',
        ),
        controls,
        results,
        ui.note(
          'The allowlist, the registry, the references and the sector map are campaign inputs the source invents none of, so the ones used here are the console\'s own: an allowlist of close and volume, three references, no sectors.',
          'muted',
        ),
      ),
      ui.block('What it refuses', refusalVocabulary(ui)),
      ui.block('Where it goes wrong', ui.list(layer.caveats)),
      ui.block(
        'Where this came from',
        ui.sources(layer.sources),
        ui.sources([
          { label: 'the candidate transcription', path: 'implementation/strategy-discovery-console/src/data/candidate.js' },
          { label: 'the candidate engine', path: 'implementation/strategy-discovery-console/src/engine/candidate.js' },
          { label: 'the policy it resolves against', path: 'implementation/strategy-discovery-console/src/engine/policy.js' },
        ]),
        ui.note(
          `the five stages this model walks: ${STAGES.map((stage) => `${stage.order}. ${stage.name} (${stage.implementation})`).join(', ')}.`,
          'muted',
        ),
        ui.note(`the four warnings, which never block a candidate: ${WARNING_CODES.join(', ')}.`, 'muted'),
      ),
    ),
  );

  update();
}

/** The closed vocabulary, with the message each code is recorded with and the reader that records it. */
function refusalVocabulary(ui) {
  const rows = [];
  for (const row of REFUSAL_FACTS) {
    const code = valueOf(row.code);
    const where = `${valueOf(row.where)} (${cite(row.where)})`;
    for (const message of row.messages) rows.push([code, valueOf(message), where]);
  }
  return ui.stack(
    ui.table({
      columns: ['Code', 'The message the source records', 'Recorded by'],
      rows,
      dense: true,
      caption: `${REFUSAL_FACTS.length} names, and every message each is recorded with`,
    }),
    ui.note(
      'The model above produces the codes its controls can reach and shows them with the field they name. The ones it cannot reach are reachable in the repository only through inputs a browser does not have: a payload that is not a mapping, a registered migration table (so version_migration_cycle), and a policy that declares an unordered frequency alongside an ordered one is reachable here only through the frequency_unorderable defect.',
      'muted',
    ),
  );
}
