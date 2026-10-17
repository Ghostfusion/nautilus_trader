/**
 * Tests for the candidate engine.
 *
 * These cases are the ones a reader would notice breaking: the twelve blocks or the closed
 * vocabulary drifting from `src/data/candidate.js`, a refusal that stops at the wrong stage, a
 * refusal that carries a code the source does not record for that rule, and the two condition caps
 * being counted as one thing when the source counts them as two.
 */

import { describe, expect, test } from 'bun:test';

import {
  BLOCKS as BLOCK_FACTS,
  BLOCK_NAMES,
  CAPS as CAP_FACTS,
  REFUSAL_CODES as REFUSAL_FACTS,
  RISK_CATEGORIES,
  STAGES as STAGE_FACTS,
} from '../src/data/candidate.js';
import { LIMITS } from '../src/data/policy.js';
import { valueOf } from '../src/data/source.js';
import { isDigest } from '../src/engine/digest.js';
import {
  BLOCKS,
  CAPS,
  ENGINE_VERSION,
  NOT_MODELLED,
  REFUSAL_CODES,
  STAGES,
  conditionDepth,
  countConditionLeaves,
  emptySpecification,
  fieldAt,
  resolveField,
  validateSpecification,
} from '../src/engine/candidate.js';
import { defaultPolicy, freeze } from '../src/engine/policy.js';

/** The campaign inputs the source invents none of: an allowlist and three references. */
function campaignInputs() {
  return {
    featureAllowlist: ['close', 'volume'],
    references: [
      { name: 'console_benchmark', kind: 'benchmark', digest: `sha256:${'a'.repeat(64)}` },
      { name: 'console_cost_model', kind: 'cost_model', digest: `sha256:${'b'.repeat(64)}` },
      { name: 'console_validation', kind: 'validation_policy', digest: `sha256:${'c'.repeat(64)}` },
    ],
  };
}

/** The policy a specification is checked against, which is the first release plus its campaign. */
function policy() {
  return defaultPolicy(campaignInputs());
}

/** The registry stage two resolves against, which is another campaign input. */
function registry(lagNs = 0) {
  return { close: { availability_lag_ns: lagNs }, volume: { availability_lag_ns: 0 } };
}

/** A hand-written legal specification: one comparison, a rank selection, a weekly rebalance. */
function legalSpecification() {
  return {
    version: 'discovery.specification/v1',
    identity: {
      family: 'cross_sectional_rank',
      hypothesis: 'the rank carries information',
      mechanism: 'a cross-sectional rank of the declared feature',
      falsification: 'the rank carries no information',
    },
    universe: {
      asset_classes: ['EQUITY', 'ETF'],
      membership_policy: 'point_in_time',
      source: 'the declared snapshot',
      min_price: 5,
    },
    data: {
      signal_frequency: 'daily',
      decision_time: 'session_close',
      execution_eligibility: 'next_session_open',
      corporate_action_treatment: 'adjusted',
    },
    features: { declared: { trend: { feature: 'close', lookback: 20 } } },
    selection: { mode: 'rank', feature: 'trend', percentile: 0.1, tie_break: 'feature_desc' },
    entry: { direction: 'long', conditions: { all: [{ feature: 'trend', comparison: 'gt', value: 0 }] } },
    exit: {},
    portfolio: {
      position_count: 20,
      weighting: 'equal',
      max_position_weight: 0.2,
      cash_policy: 'leave_idle',
    },
    rebalance: { frequency: 'weekly', timing: 'next_session_open' },
    risk: {},
    evaluation: {
      benchmark: 'console_benchmark',
      cost_model: 'console_cost_model',
      validation_policy: 'console_validation',
    },
    provenance: { generator: 'hand-written', seed: 1 },
  };
}

/** The same payload with one field replaced, so a test changes exactly one thing. */
function withBlock(name, value) {
  const specification = legalSpecification();
  specification[name] = value;
  return specification;
}

describe('the language', () => {
  test('the twelve blocks are the ones BLOCK_NAMES fixes, in its order', () => {
    // If this fails a candidate is read in an order the source does not read it in.
    expect(BLOCKS().map((block) => block.name)).toEqual(valueOf(BLOCK_NAMES));
    expect(BLOCKS().length).toBe(12);
  });

  test('every block declares the fields its own reader reads', () => {
    // A field missing here is a field the console cannot show or set, and one added here is a field
    // the source would refuse as undeclared.
    const blocks = Object.fromEntries(BLOCKS().map((block) => [block.name, block]));
    const declared = Object.fromEntries(
      BLOCKS().map((block) => [block.name, block.fields.map((entry) => entry.category ?? entry.name)]),
    );
    expect(declared.identity).toEqual(['family', 'hypothesis', 'mechanism', 'falsification']);
    expect(declared.universe).toEqual([
      'asset_classes',
      'membership_policy',
      'source',
      'min_price',
      'min_dollar_volume',
      'exclusions',
    ]);
    expect(declared.data).toEqual([
      'signal_frequency',
      'decision_time',
      'execution_eligibility',
      'corporate_action_treatment',
    ]);
    expect(declared.features).toEqual(['declared', 'feature', 'lookback', 'skip']);
    expect(declared.selection).toEqual([
      'mode',
      'feature',
      'percentile',
      'count',
      'threshold',
      'comparison',
      'tie_break',
    ]);
    expect(declared.exit).toEqual([
      'holding_period_cap',
      'rank_exit_count',
      'feature',
      'comparison',
      'threshold',
      'stop_fraction',
      'target_fraction',
      'flatten_ineligible',
    ]);
    expect(declared.portfolio).toEqual([
      'position_count',
      'weighting',
      'max_position_weight',
      'max_sector_weight',
      'cash_policy',
    ]);
    expect(declared.rebalance).toEqual(['frequency', 'timing']);
    expect(blocks.risk.fields.map((category) => category.category)).toEqual(['pre_trade', 'runtime', 'post_run']);
    expect(declared.evaluation).toEqual(['benchmark', 'cost_model', 'validation_policy']);
    expect(declared.provenance).toEqual(['generator', 'seed', 'parents']);

    // And every block agrees with the transcription, block by block.
    for (const row of BLOCK_FACTS) {
      expect(declared[valueOf(row.name)]).toEqual(row.fields.map((entry) => valueOf(entry.name)));
    }
  });

  test('the three risk categories hold exactly their own fields', () => {
    // The categories are what makes a post-run limit presented as a runtime control refusable.
    expect(RISK_CATEGORIES.map((category) => valueOf(category.name))).toEqual([
      'pre_trade',
      'runtime',
      'post_run',
    ]);
    const runtime = BLOCKS().find((block) => block.name === 'risk').fields[1];
    expect(runtime.fields.map((entry) => entry.name)).toEqual(['halt_new_entries', 'flatten_ineligible']);
    const postRun = BLOCKS().find((block) => block.name === 'risk').fields[2];
    expect(postRun.fields.map((entry) => entry.name)).toEqual(['max_drawdown', 'max_turnover', 'max_cost_ratio']);
  });

  test('the caps agree with the policy transcription and with the source', () => {
    // If this fails the console counts a cap the policy does not declare, or misses one it does.
    expect(CAPS()).toEqual({ max_conditions: 4, max_condition_depth: 2, max_features: 8 });
    expect(CAPS().max_conditions).toBe(valueOf(LIMITS.max_conditions));
    expect(CAPS().max_condition_depth).toBe(valueOf(LIMITS.max_condition_depth));
    expect(CAPS().max_features).toBe(valueOf(LIMITS.max_features));
    expect(CAPS()).toEqual(
      Object.fromEntries(Object.entries(CAP_FACTS).map(([name, value]) => [name, valueOf(value)])),
    );
  });

  test('the five stages are named, ordered and implemented as the source says', () => {
    // The order is the rule: a stage runs only when the stage before it accepted.
    expect(STAGES.map((stage) => stage.id)).toEqual([
      'schema_validation',
      'feature_resolution',
      'semantic_validation',
      'temporal_validation',
      'trusted_compilation',
    ]);
    expect(STAGES.map((stage) => stage.order)).toEqual([1, 2, 3, 4, 5]);
    expect(STAGES.map((stage) => stage.implementation)).toEqual([
      'parse_specification',
      '_resolve_features',
      '_validate_semantics',
      '_validate_temporal',
      'compile_specification',
    ]);
    expect(STAGES.map((stage) => stage.name)).toEqual(valueOf(STAGE_FACTS).map((stage) => valueOf(stage.name)));
  });

  test('the closed refusal vocabulary is the thirty-two names the source declares', () => {
    // A refusal carrying a code the repository never records is a refusal a reader cannot look up.
    expect(REFUSAL_CODES.length).toBe(32);
    expect(REFUSAL_CODES[0]).toBe('not_a_mapping');
    expect(REFUSAL_CODES[31]).toBe('undeferred_execution');
    expect(REFUSAL_CODES).toEqual(REFUSAL_FACTS.map((row) => valueOf(row.code)));
  });

  test('an empty candidate is the twelve blocks with nothing declared', () => {
    // The builder starts here, so a block missing would leave a control writing nowhere.
    const empty = emptySpecification();
    expect(Object.keys(empty).length).toBe(13);
    expect(empty.features).toEqual({ declared: {} });
    expect(empty.risk).toEqual({ pre_trade: {}, runtime: {}, post_run: {} });
    expect(valueOf(BLOCK_NAMES).every((name) => name in empty)).toBe(true);
  });

  test('a field is looked up by the dotted path its block reports it at', () => {
    // The refusal paths are dotted paths, so a lookup that missed would misname the refused field.
    expect(fieldAt('rebalance.frequency').declaration).toBe('rebalance_frequencies');
    expect(fieldAt('risk.post_run.max_drawdown').declaration).toBe('risk.max_drawdown');
    expect(fieldAt('portfolio.position_count').reads).toBe('bounded_int');
    expect(fieldAt('no.such.field')).toBe(null);
  });
});

describe('a legal candidate', () => {
  test('a hand-written candidate compiles, and the plan carries its identities', () => {
    // If this fails nothing legal compiles, and every refusal below would be unreadable.
    const result = validateSpecification(legalSpecification(), policy(), { registry: registry() });
    expect(result.ok).toBe(true);
    expect(result.stage).toBe('trusted_compilation');
    expect(result.refusals).toEqual([]);
    expect(result.plan.engineVersion).toBe(ENGINE_VERSION);
    expect(result.plan.specificationDigest).not.toBe(result.plan.behaviouralDigest);
    expect(isDigest(result.plan.specificationDigest)).toBe(true);
    expect(result.plan.policyDigest).toBe(freeze(policy()).digest);
    expect(result.plan.assumptionsDigest).toBe(null);
    expect(NOT_MODELLED.length).toBe(2);
  });

  test('two runs over one payload agree, and the payload is never mutated', () => {
    // Determinism is the whole claim: a candidate compiles to the same plan twice, and validating it
    // does not edit what was submitted.
    const specification = legalSpecification();
    const before = JSON.stringify(specification);
    const first = validateSpecification(specification, policy(), { registry: registry() });
    const second = validateSpecification(specification, policy(), { registry: registry() });
    expect(JSON.stringify(first)).toBe(JSON.stringify(second));
    expect(JSON.stringify(specification)).toBe(before);
  });

  test('a refused payload returns its refusals rather than raising', () => {
    // A refusal is an outcome, and the payload is never compiled.
    const result = validateSpecification(
      withBlock('universe', {
        asset_classes: ['EQUITY', 'ETF'],
        membership_policy: 'point_in_time',
        source: 'nowhere',
        min_price: 2500,
      }),
      policy(),
      { registry: registry() },
    );
    expect(result.ok).toBe(false);
    expect(result.plan).toBe(null);
    expect(result.refusals.length).toBeGreaterThan(0);
    expect(result.refusals[0].ok).toBe(false);
    expect(result.refusals[0].path).toBe('universe.min_price');
  });
});

describe('the schema stage', () => {
  test('a payload that is not a mapping is refused before anything is read', () => {
    // The first rule of the parser, and the one every other refusal depends on not being reached.
    const result = validateSpecification('not a mapping', policy(), { registry: registry() });
    expect(result.stage).toBe('schema_validation');
    expect(result.code).toBe('not_a_mapping');
    expect(result.reason).toBe('a specification must be a mapping, was str');
    expect(result.refusals[0].path).toBe('');
  });

  test('a field name that asks for code is refused by name, wherever it is put', () => {
    // Criterion 3: a specification naming a module is refused at that field, not compiled.
    const specification = legalSpecification();
    specification.provenance.module_path = 'harmless text';
    const result = validateSpecification(specification, policy(), { registry: registry() });
    expect(result.stage).toBe('schema_validation');
    expect(result.code).toBe('code_reference_refused');
    expect(result.refusals[0].path).toBe('provenance.module_path');
    expect(result.reason).toBe('a specification is data and may not declare module_path');
  });

  test('a value shaped like code is refused by shape, and a bare dotted string is admitted', () => {
    // The boundary the page states: nothing resolves a dotted string with no colon, so it is data.
    const shaped = legalSpecification();
    shaped.provenance.generator = 'strategies.ema_cross:EMACross';
    const refused = validateSpecification(shaped, policy(), { registry: registry() });
    expect(refused.code).toBe('code_reference_refused');
    expect(refused.refusals[0].path).toBe('provenance.generator');

    const inert = legalSpecification();
    inert.provenance.generator = 'nautilus_trader.adapters.binance';
    const admitted = validateSpecification(inert, policy(), { registry: registry() });
    expect(admitted.ok).toBe(true);
    expect(admitted.specification.provenance.generator).toBe('nautilus_trader.adapters.binance');
  });

  test('an accepted version is required, and an unaccepted one is refused rather than migrated', () => {
    // A stored candidate is only readable against the schema it was written for.
    const unversioned = legalSpecification();
    delete unversioned.version;
    expect(validateSpecification(unversioned, policy(), { registry: registry() }).code).toBe('version_missing');

    const older = validateSpecification(
      { ...legalSpecification(), version: 'discovery.specification/v0' },
      policy(),
      { registry: registry() },
    );
    expect(older.code).toBe('version_unsupported');
    expect(older.refusals[0].path).toBe('version');
  });

  test('a migration cycle is refused rather than looped', () => {
    // Eight hops without reaching an accepted version is a refusal, not a hang.
    const migrations = {
      'discovery.specification/a': (payload) => ({ ...payload, version: 'discovery.specification/b' }),
      'discovery.specification/b': (payload) => ({ ...payload, version: 'discovery.specification/a' }),
    };
    const result = validateSpecification(
      { ...legalSpecification(), version: 'discovery.specification/a' },
      policy(),
      { registry: registry(), migrations },
    );
    expect(result.code).toBe('version_migration_cycle');
    expect(result.reason).toContain('do not reach an accepted version');
  });

  test('all twelve blocks are required by name, and nothing else is allowed beside them', () => {
    // A missing block is refused by name; an extra top-level key is refused because the specification
    // is closed.
    const missing = legalSpecification();
    delete missing.portfolio;
    const missingResult = validateSpecification(missing, policy(), { registry: registry() });
    expect(missingResult.code).toBe('block_missing');
    expect(missingResult.refusals[0].path).toBe('portfolio');

    const extra = { ...legalSpecification(), notes: 'a comment' };
    const extraResult = validateSpecification(extra, policy(), { registry: registry() });
    expect(extraResult.code).toBe('block_unknown_field');
    expect(extraResult.refusals[0].path).toBe('notes');
  });

  test('a key a block did not read is refused at its own path', () => {
    // The allowed set is the reading, so it cannot drift from the reader.
    const result = validateSpecification(
      withBlock('portfolio', { ...legalSpecification().portfolio, leverage: 2 }),
      policy(),
      { registry: registry() },
    );
    expect(result.code).toBe('block_unknown_field');
    expect(result.refusals[0].path).toBe('portfolio.leverage');
    expect(result.reason).toBe('the portfolio block does not declare leverage');
  });

  test('a value outside a declared set is refused with the set in sorted order', () => {
    // The message restates the declaration rather than the candidate's opinion of it.
    const result = validateSpecification(
      withBlock('rebalance', { frequency: 'hourly', timing: 'next_session_open' }),
      policy(),
      { registry: registry() },
    );
    expect(result.code).toBe('value_not_declared');
    expect(result.refusals[0].path).toBe('rebalance.frequency');
    expect(result.reason).toBe('frequency must be one of daily, monthly, weekly, was hourly');
  });

  test('a value outside its bound is refused with the bound stated', () => {
    // The tutorial's own refusal, character for character.
    const specification = legalSpecification();
    specification.universe.min_price = 2500;
    const result = validateSpecification(specification, policy(), { registry: registry() });
    expect(result.code).toBe('value_out_of_bound');
    expect(result.refusals[0].path).toBe('universe.min_price');
    expect(result.reason).toBe('min_price must be 1 to 1000 inclusive, was 2500');
  });

  test('a bound the policy does not declare refuses every value of its field', () => {
    // "A missing bound is not a default: the policy does not cover the field." So the value that was
    // admitted a moment ago is refused once the bound is gone.
    const narrowed = policy();
    narrowed.bounds = narrowed.bounds.filter((bound) => bound.name !== 'portfolio.position_count');
    const result = validateSpecification(legalSpecification(), narrowed, { registry: registry() });
    expect(result.code).toBe('bound_not_declared');
    expect(result.refusals[0].path).toBe('portfolio');
    expect(result.reason).toBe(
      'the policy declares no bound named portfolio.position_count, so the portfolio block cannot admit a value',
    );
  });

  test('a feature the campaign does not allow is refused, and one the registry does not hold too', () => {
    // Stage one checks the allowlist and stage two checks the registry: two different refusals.
    const notAllowed = legalSpecification();
    notAllowed.features.declared.trend.feature = 'book_value';
    const allowed = validateSpecification(notAllowed, policy(), { registry: registry() });
    expect(allowed.code).toBe('feature_not_allowed');
    expect(allowed.refusals[0].path).toBe('features.declared.trend.feature');

    const unresolved = validateSpecification(legalSpecification(), policy(), { registry: {} });
    expect(unresolved.stage).toBe('feature_resolution');
    expect(unresolved.code).toBe('feature_unresolved');
    expect(unresolved.reason).toContain('the alias trend selects a feature the run cannot serve');
  });

  test('more than eight declared features is refused before any declaration is read', () => {
    // The cap is on the count, and it does not stop the rest of the block being read.
    const specification = legalSpecification();
    specification.features.declared = Object.fromEntries(
      Array.from({ length: 9 }, (unused, index) => [`alias_${index}`, { feature: 'close', lookback: 20 }]),
    );
    const result = validateSpecification(specification, policy(), { registry: registry() });
    expect(result.code).toBe('feature_count_exceeded');
    expect(result.reason).toBe('a specification declares at most 8 features, was 9');
  });

  test('a reference the policy does not hold is refused with the pinned names stated', () => {
    // A reference pins a fingerprint, so a name the policy does not hold resolves to nothing.
    const result = validateSpecification(
      withBlock('evaluation', { benchmark: 'other', cost_model: 'console_cost_model', validation_policy: 'console_validation' }),
      policy(),
      { registry: registry() },
    );
    expect(result.code).toBe('reference_unresolved');
    expect(result.refusals[0].path).toBe('evaluation.benchmark');
    expect(result.reason).toContain('benchmark must name a frozen reference, other is not declared');
  });

  test('a risk field placed under the wrong category is refused by category', () => {
    // A maximum drawdown is measured after the run, so presenting it as a live control is refused.
    const result = validateSpecification(
      withBlock('risk', { runtime: { max_drawdown: 0.2 } }),
      policy(),
      { registry: registry() },
    );
    expect(result.code).toBe('risk_field_misplaced');
    expect(result.refusals[0].path).toBe('risk.runtime.max_drawdown');
    expect(result.reason).toBe(
      'max_drawdown is a post-run field and does not control behaviour, so it belongs under post_run',
    );
  });

  test('a repeated identifier in a sequence is refused at its index', () => {
    // A repeated identifier in a universe is a defect a reader wants named, not deduplicated.
    const result = validateSpecification(
      withBlock('universe', { asset_classes: ['EQUITY', 'EQUITY'], membership_policy: 'point_in_time', source: 'x' }),
      policy(),
      { registry: registry() },
    );
    expect(result.code).toBe('field_repeated');
    expect(result.refusals[0].path).toBe('universe.asset_classes.1');
    expect(result.reason).toBe('asset_classes states EQUITY more than once');
  });
});

describe('the condition tree and its two caps', () => {
  test('the caps count different things, and both are refusals', () => {
    // The page's own arithmetic: five leaves three levels deep breaches the depth cap first, and a
    // five-leaf tree two levels deep breaches the leaf cap instead.
    const deep = legalSpecification();
    deep.entry.conditions = {
      all: [
        { feature: 'trend', comparison: 'gt', value: 0 },
        { feature: 'trend', comparison: 'gt', value: 0 },
        { any: [
          { feature: 'trend', comparison: 'gt', value: 0 },
          { feature: 'trend', comparison: 'gt', value: 0 },
          { feature: 'trend', comparison: 'gt', value: 0 },
        ] },
      ],
    };
    const depthRefusal = validateSpecification(deep, policy(), { registry: registry() });
    expect(depthRefusal.code).toBe('condition_depth_exceeded');
    // The refusal is reported at the first node that reached level three, which is that node's own
    // path rather than the group that holds it.
    expect(depthRefusal.refusals[0].path).toBe('entry.conditions.all.2.any.0');
    expect(depthRefusal.reason).toBe('a condition tree nests at most 2 levels, was 3');

    const wide = legalSpecification();
    wide.entry.conditions = {
      all: Array.from({ length: 5 }, () => ({ feature: 'trend', comparison: 'gt', value: 0 })),
    };
    const leafRefusal = validateSpecification(wide, policy(), { registry: registry() });
    expect(leafRefusal.code).toBe('condition_count_exceeded');
    expect(leafRefusal.refusals[0].path).toBe('entry.conditions');
    expect(leafRefusal.reason).toBe('an entry block states at most 4 conditions, was 5');
  });

  test('leaves are comparisons and depth is counted from one', () => {
    // The two counts the page tells the reader to make by hand.
    const tree = {
      all: [
        { feature: 'trend', comparison: 'gt', value: 0 },
        { feature: 'trend', comparison: 'gt', value: 0 },
        { any: [
          { feature: 'trend', comparison: 'gt', value: 0 },
          { feature: 'trend', comparison: 'gt', value: 0 },
          { feature: 'trend', comparison: 'gt', value: 0 },
        ] },
      ],
    };
    expect(countConditionLeaves(tree)).toBe(5);
    expect(conditionDepth(tree)).toBe(3);
    expect(countConditionLeaves({ feature: 'trend', comparison: 'gt', value: 0 })).toBe(1);
    expect(conditionDepth({ feature: 'trend', comparison: 'gt', value: 0 })).toBe(1);
    expect(countConditionLeaves(null)).toBe(0);
  });

  test('a group states all or any, not both, and holds at least one condition', () => {
    // Two ways one group node is refused, both at the group's own path.
    const ambiguous = legalSpecification();
    ambiguous.entry.conditions = {
      all: [{ feature: 'trend', comparison: 'gt', value: 0 }],
      any: [{ feature: 'trend', comparison: 'gt', value: 0 }],
    };
    const both = validateSpecification(ambiguous, policy(), { registry: registry() });
    expect(both.code).toBe('condition_not_resolved');
    expect(both.reason).toBe('a condition states all or any, not both');
    expect(both.refusals[0].path).toBe('entry.conditions');

    const empty = legalSpecification();
    empty.entry.conditions = { any: [] };
    const none = validateSpecification(empty, policy(), { registry: registry() });
    expect(none.code).toBe('condition_not_resolved');
    expect(none.refusals[0].path).toBe('entry.conditions.any');
    expect(none.reason).toBe('the any group must hold at least one condition');
  });

  test('a comparison names a declared alias and an operator from the closed set', () => {
    // The leaf is exactly {feature, comparison, value}, refused field by field.
    const spec = legalSpecification();
    spec.entry.conditions = { all: [{ feature: 'missing', comparison: 'gt', value: 0 }] };
    const undeclared = validateSpecification(spec, policy(), { registry: registry() });
    expect(undeclared.code).toBe('feature_not_declared');
    expect(undeclared.refusals[0].path).toBe('entry.conditions.all.0.feature');

    const operator = legalSpecification();
    operator.entry.conditions = { all: [{ feature: 'trend', comparison: 'eq', value: 0 }] };
    const refused = validateSpecification(operator, policy(), { registry: registry() });
    expect(refused.code).toBe('value_not_declared');
    expect(refused.refusals[0].path).toBe('entry.conditions.all.0.comparison');
    expect(refused.reason).toBe('comparison must be one of ge, gt, le, lt, was eq');
  });
});

describe('the semantic and temporal stages', () => {
  test('a rank selection states exactly one bound and names its feature and tie-break', () => {
    // The mode dispatches the rules, so a rank selection without its feature is refused by name.
    const withoutFeature = legalSpecification();
    delete withoutFeature.selection.feature;
    const result = validateSpecification(withoutFeature, policy(), { registry: registry() });
    expect(result.stage).toBe('semantic_validation');
    expect(result.code).toBe('selection_incomplete');
    expect(result.refusals[0].path).toBe('selection');
    expect(result.reason).toBe('a rank selection requires feature');

    const both = legalSpecification();
    both.selection.count = 20;
    const bothResult = validateSpecification(both, policy(), { registry: registry() });
    expect(bothResult.refusals[0].reason).toBe('a rank selection states exactly one of percentile and count');
  });

  test('a selection that ranks nothing states no ranking field', () => {
    // The none mode forbids all six ranking fields rather than ignoring them.
    const result = validateSpecification(
      withBlock('selection', { mode: 'none', feature: 'trend' }),
      policy(),
      { registry: registry() },
    );
    expect(result.code).toBe('selection_incomplete');
    expect(result.reason).toBe('a selection that ranks nothing states no feature');
  });

  test('an exit states a feature with both comparison and threshold, or with neither', () => {
    // A stop or target without a feature cannot be read, and a feature without a level cannot either.
    const result = validateSpecification(withBlock('exit', { feature: 'trend' }), policy(), { registry: registry() });
    expect(result.code).toBe('exit_incomplete');
    expect(result.refusals[0].path).toBe('exit');
    expect(result.reason).toBe('an exit states a feature with both comparison and threshold, or states no feature');
  });

  test('a rank exit count requires a rank selection', () => {
    // rank_exit_count is resolved against a ranking, so beside a none selection it is refused.
    const result = validateSpecification(
      { ...legalSpecification(), selection: { mode: 'none' }, exit: { rank_exit_count: 20 } },
      policy(),
      { registry: registry() },
    );
    expect(result.code).toBe('exit_incomplete');
    expect(result.refusals[0].path).toBe('exit.rank_exit_count');
    expect(result.reason).toBe('rank_exit_count is resolved against a ranking, and the selection mode is none');
  });

  test('a sector cap without a sector map is refused, and the policy declares none', () => {
    // The sector map is a campaign input, and none is invented for it.
    const specification = legalSpecification();
    specification.portfolio.max_sector_weight = 0.4;
    const result = validateSpecification(specification, policy(), { registry: registry() });
    expect(result.code).toBe('sector_input_undeclared');
    expect(result.refusals[0].path).toBe('portfolio.max_sector_weight');
  });

  test('the rebalance timing must equal the execution eligibility', () => {
    // Both are declared, so a disagreement is a refusal rather than a default.
    const result = validateSpecification(
      withBlock('rebalance', { frequency: 'weekly', timing: 'daily' }),
      policy(),
      { registry: registry() },
    );
    expect(result.code).toBe('value_not_declared');
    expect(result.refusals[0].path).toBe('rebalance.timing');
  });

  test('a frequency the compiler cannot order is refused at the rebalance block', () => {
    // The ordering table is closed, so an unordered frequency is refused rather than assumed.
    const widened = policy();
    widened.sets = { ...widened.sets, rebalance_frequencies: ['daily', 'weekly', 'monthly', 'hourly'] };
    const result = validateSpecification(
      withBlock('rebalance', { frequency: 'hourly', timing: 'next_session_open' }),
      widened,
      { registry: registry() },
    );
    expect(result.stage).toBe('semantic_validation');
    expect(result.code).toBe('rebalance_frequency_mismatch');
    expect(result.refusals[0].path).toBe('rebalance.frequency');
    expect(result.reason).toContain('cannot order hourly against daily');
  });

  test('the fill instant must be strictly later than the decision instant', () => {
    // Deciding at the close and filling at it is the undeterred case the stage refuses.
    const widened = policy();
    widened.sets = { ...widened.sets, rebalance_timings: ['next_session_open', 'same_session_close'] };
    const specification = legalSpecification();
    specification.data.execution_eligibility = 'same_session_close';
    specification.rebalance.timing = 'same_session_close';
    const result = validateSpecification(specification, widened, { registry: registry() });
    expect(result.stage).toBe('temporal_validation');
    expect(result.code).toBe('undeferred_execution');
    expect(result.refusals[0].path).toBe('data.execution_eligibility');
    expect(result.reason).toContain('must be strictly later than the decision time session_close');
  });

  test('a feature whose definition is available at or beyond the signal interval is refused', () => {
    // The reading would not exist at the decision instant, so the value is not knowable.
    const atInterval = validateSpecification(legalSpecification(), policy(), {
      registry: registry(86400000000000),
    });
    expect(atInterval.code).toBe('feature_unavailable_at_decision');
    expect(atInterval.refusals[0].path).toBe('features.declared.trend.feature');
    expect(atInterval.reason).toContain('not knowable at the decision instant');

    const before = validateSpecification(legalSpecification(), policy(), {
      registry: registry(86399999999999),
    });
    expect(before.ok).toBe(true);
  });
});

describe('the warnings', () => {
  test('warnings are a separate channel and never block a candidate', () => {
    // A warning names a legal specification an operator should see; a refusal is never softened by
    // the warning beside it.
    const specification = legalSpecification();
    specification.features.declared = {
      trend: { feature: 'close', lookback: 20 },
      spare: { feature: 'close', lookback: 20 },
    };
    const unused = validateSpecification(specification, policy(), { registry: registry() });
    expect(unused.ok).toBe(true);
    expect(unused.warnings.map((warning) => warning.code)).toContain('unused_declared_feature');
    expect(unused.warnings.find((warning) => warning.code === 'unused_declared_feature').path).toBe(
      'features.declared.spare',
    );

    const warmup = legalSpecification();
    warmup.features.declared.trend.lookback = 21;
    const warmed = validateSpecification(warmup, policy(), { registry: registry() });
    expect(warmed.ok).toBe(true);
    expect(warmed.warnings[0].code).toBe('feature_warmup_dominates');
    expect(warmed.warnings[0].reason).toContain('exceeds 4 weekly intervals, which is 20 sessions');
  });

  test('a position cap below fully invested and a post-run limit both warn', () => {
    // Twenty positions at a weight of 0.05 allocate 1.0, which is not below fully invested, so the
    // warning needs a cap the allocation cannot reach.
    const small = legalSpecification();
    small.portfolio.max_position_weight = 0.04;
    const warned = validateSpecification(small, policy(), { registry: registry() });
    expect(warned.ok).toBe(true);
    expect(warned.warnings.map((warning) => warning.code)).toContain('position_cap_below_fully_invested');

    const gated = validateSpecification(withBlock('risk', { post_run: { max_drawdown: 0.2 } }), policy(), {
      registry: registry(),
    });
    expect(gated.ok).toBe(true);
    expect(gated.warnings.map((warning) => warning.code)).toContain('post_run_limits_are_gate_conditions');
    expect(gated.warnings.find((warning) => warning.path === 'risk.post_run.max_drawdown')).not.toBe(undefined);

    const fullyInvested = validateSpecification(legalSpecification(), policy(), { registry: registry() });
    expect(fullyInvested.warnings).toEqual([]);
  });
});

describe('one field at a time', () => {
  test('resolveField returns the tutorial\u2019s own refusals, character for character', () => {
    // The policy view resolves values through this path, so a message that drifted would put a
    // sentence on screen that the repository does not print.
    expect(resolveField(policy(), 'rebalance.frequency', 'hourly')).toEqual({
      ok: false,
      code: 'value_not_declared',
      path: 'rebalance.frequency',
      reason: 'frequency must be one of daily, monthly, weekly, was hourly',
      field: 'rebalance.frequency',
    });
    expect(resolveField(policy(), 'universe.min_price', 2500).reason).toBe(
      'min_price must be 1 to 1000 inclusive, was 2500',
    );
    expect(resolveField(policy(), 'universe.min_price', 5)).toEqual({
      ok: true,
      field: 'universe.min_price',
      value: 5,
    });
  });

  test('resolveField refuses a bound the policy does not declare, for every value', () => {
    // The policy view's "delete one row and re-mark the number" exercise, as a value.
    const narrowed = policy();
    narrowed.bounds = narrowed.bounds.filter((bound) => bound.name !== 'universe.min_price');
    const result = resolveField(narrowed, 'universe.min_price', 5);
    expect(result.code).toBe('bound_not_declared');
    expect(result.reason).toBe(
      'the policy declares no bound named universe.min_price, so the universe block cannot admit a value',
    );
  });

  test('resolveField refuses a field no block declares, as the programming error it is', () => {
    // A path the twelve blocks do not hold is a wiring mistake in the console, not a refusal.
    expect(() => resolveField(policy(), 'portfolio.something_else', 1)).toThrow();
  });
});
