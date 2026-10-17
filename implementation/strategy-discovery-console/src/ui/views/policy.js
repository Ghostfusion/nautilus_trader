/**
 * The rules nobody may choose.
 *
 * This view models `python/nautilus_trader/discovery/policy.py` through `src/engine/policy.js`: the
 * fifteen declared sets, the nineteen first-release bounds, the three complexity caps, the frozen
 * references, the sector map and the freeze. The reader can edit a bound or a declared set and watch
 * the fingerprint move, and can resolve one value at a time through the field's own reader to see the
 * refusal the schema would return, with the declaration that decided it.
 *
 * The feature allowlist, the references and the sector map are campaign inputs the source's
 * `first_release` invents none of, so the reflectances this model resolves against are the console's
 * own and are labelled as such. A reference pins a fingerprint and never the artifact, so the digest
 * shown for one is the console's digest of its own name; the view says so rather than pretending an
 * artifact was hashed.
 */

import {
  BOUNDS,
  DECLARED_SETS,
  LIMITS,
  POLICY_RULES,
  REFUSAL_CODES as POLICY_REFUSALS,
  SECTORS,
} from '../../data/policy.js';
import { cite, valueOf } from '../../data/source.js';
import { fieldAt, resolveField } from '../../engine/candidate.js';
import {
  REFUSAL_CODES as POLICY_REFUSAL_NAMES,
  SET_NAMES,
  defaultPolicy,
  freeze,
  validatePolicy,
} from '../../engine/policy.js';

/** The layer this view is: its id, the tutorial page's own title, and its position in the pipeline. */
export const meta = {
  id: 'policy',
  title: 'The rules nobody may choose',
  order: 1,
  tutorial: 'tutorials/strategy-discovery/the-rules-nobody-may-choose/README.md',
};

/** The fields this view resolves by set membership, which is where a name is refused. */
const SET_FIELDS = [
  'identity.family',
  'universe.asset_classes',
  'data.signal_frequency',
  'data.execution_eligibility',
  'data.corporate_action_treatment',
  'selection.mode',
  'selection.tie_break',
  'entry.direction',
  'portfolio.weighting',
  'portfolio.cash_policy',
  'rebalance.frequency',
  'rebalance.timing',
];

/** The fields this view resolves against an inclusive bound. */
const BOUND_FIELDS = [
  'universe.min_price',
  'selection.percentile',
  'portfolio.position_count',
  'portfolio.max_position_weight',
  'risk.max_drawdown',
];

/** The nineteen bounds as this view shows them: the source's limits and the tutorial's words. */
const BOUND_ROWS = BOUNDS.map((row) => ({
  name: valueOf(row.name),
  unit: valueOf(row.unit),
  lower: valueOf(row.lower),
  upper: valueOf(row.upper),
  describe: `${valueOf(row.lower)} to ${valueOf(row.upper)} inclusive`,
  source: cite(row.name),
}));

/** The line each bound was read from, so an edited bound still shows where it came from. */
const BOUND_LINES = Object.fromEntries(BOUND_ROWS.map((row) => [row.name, row.source]));

/** The four refusals this view can produce, and the value that produces each. */
const DEMONSTRATIONS = [
  { code: 'value_not_declared', field: 'rebalance.frequency', value: 'hourly', note: 'a name no declared set holds' },
  { code: 'value_out_of_bound', field: 'universe.min_price', value: 2500, note: 'a number above the bound' },
  {
    code: 'bound_not_declared',
    field: 'universe.min_price',
    value: 5,
    dropBound: 'universe.min_price',
    note: 'the same number with the bound deleted from the policy',
  },
  { code: 'reference_unresolved', field: 'evaluation.benchmark', value: 'other', note: 'a name no reference holds' },
];

/** A reference the console supplies, with a digest of its own name rather than of an artifact. */
function demoReference(name, kind, digest) {
  return { name, kind, digest };
}

/** The campaign inputs this view supplies, which the source invents none of. */
function consoleInputs() {
  return {
    featureAllowlist: ['close', 'volume'],
    references: [
      demoReference('console_benchmark', 'benchmark', 'sha256:' + 'a'.repeat(64)),
      demoReference('console_cost_model', 'cost_model', 'sha256:' + 'b'.repeat(64)),
      demoReference('console_validation', 'validation_policy', 'sha256:' + 'c'.repeat(64)),
    ],
  };
}

/** The value a resolver should receive, given the field's own reader and what the reader typed. */
function coerce(entry, typed) {
  if (entry === null) return typed;
  if (entry.reads === 'bounded' || entry.reads === 'bounded_int') {
    const number = Number(typed);
    return Number.isFinite(number) ? number : typed;
  }
  if (entry.reads === 'declared_list') {
    return typed.split(',').map((item) => item.trim()).filter((item) => item !== '');
  }
  return typed;
}

/** Build the whole view into the empty root, and wire the view's own handlers. */
export function mount(root, ctx) {
  const { ui, data } = ctx;
  const layer = data.layer(meta.id);

  const state = {
    policy: defaultPolicy(consoleInputs()),
    published: freeze(defaultPolicy(consoleInputs())).digest,
    boundName: 'portfolio.max_position_weight',
    lower: 0.005,
    upper: 0.25,
    setName: 'rebalance_frequencies',
    member: 'daily',
    field: 'rebalance.frequency',
    typed: 'hourly',
  };

  const model = ui.stack();
  const refusals = ui.stack();
  const outcome = ui.stack();
  const demonstrated = ui.stack();

  /** Rebuild everything the local state decides. */
  function render() {
    ui.clear(model);
    model.appendChild(freezePanel());
    model.appendChild(boundEditor());
    model.appendChild(setEditor());
    model.appendChild(resolutionEditor());
    model.appendChild(transcription());

    ui.clear(refusals);
    refusals.appendChild(refusalPanel());
    refusals.appendChild(demonstrated);
  }

  /** The freeze: the declarations, their fingerprint, and what the fingerprint covers. */
  function freezePanel() {
    const result = freeze(state.policy);
    const check = validatePolicy(state.policy);
    const changed = result.digest !== state.published;
    return ui.panel(
      { title: 'The freeze', subtitle: 'digest(policy) = "sha256:" + SHA-256( canonical_text(policy) )' },
      ui.metrics(
        ui.metric('Declared sets', ui.format.int(Object.keys(state.policy.sets).length), 'fifteen lists of permitted names'),
        ui.metric('Bounds', ui.format.int(state.policy.bounds.length), 'each named after the field it governs'),
        ui.metric('Caps', ui.format.int(Object.keys(LIMITS).length), 'expression size, not a field'),
        ui.metric('References', ui.format.int(state.policy.references.length), 'caller inputs, none invented here'),
        ui.metric('Sectors', ui.format.int(state.policy.sectors.length), 'declared empty by the first release'),
      ),
      ui.field(
        'Fingerprint',
        ui.digest(result.digest, 16),
        changed ? 'changed since the published policy' : 'the published first-release policy',
      ),
      ui.table({
        columns: ['What the freeze covers', 'How this model holds it'],
        rows: [
          ['Policy identity', state.policy.policyId],
          ['Version', ui.format.int(state.policy.version)],
          ['The fifteen sets', 'sorted in the canonical form, so the order a set was written in cannot matter'],
          ['The nineteen bounds', 'in declared order, as _FIRST_RELEASE_BOUNDS declares them'],
          ['The feature allowlist', state.policy.featureAllowlist.join(', '), ],
          ['The references', state.policy.references.map((entry) => entry.name).join(', ')],
          ['The sector map', (state.policy.sectors ?? []).length === 0 ? 'none declared, so no sector cap may be stated' : 'declared'],
        ],
      }),
      ui.note(
        changed
          ? 'The digest changed while the declarations kept their meaning: the source calls that a new campaign rather than a tuning step.'
          : 'This is the fingerprint of the first-release declarations with the console\'s own campaign inputs.',
        changed ? 'warn' : 'muted',
      ),
      ui.note(
        'The console digests its own canonical text with SHA-256. The engine\'s canonical digest is blake3 over a canonical serialisation, and the two are never compared.',
        'muted',
      ),
      check.ok ? null : ui.note(`validatePolicy refuses this policy: ${check.code} — ${check.reason}`, 'bad'),
      ui.row(ui.button('Reset the declarations', reset, { id: 'policy-reset' })),
    );
  }

  /** Change one bound's two limits, which is where a value stops being admitted. */
  function boundEditor() {
    const bound = state.policy.bounds.find((entry) => entry.name === state.boundName);
    return ui.panel(
      { title: 'Change one bound' },
      ui.row(
        ui.field(
          'Bound',
          ui.selectInput({
            id: 'policy-bound',
            value: state.boundName,
            options: state.policy.bounds.map((entry) => entry.name),
            onChange: (value) => {
              const next = state.policy.bounds.find((entry) => entry.name === value);
              state.boundName = value;
              state.lower = next.lower;
              state.upper = next.upper;
              render();
            },
          }),
          BOUND_LINES[state.boundName] ?? '',
        ),
        ui.field('Lower', ui.numberInput({
          id: 'policy-lower',
          value: state.lower,
          step: 'any',
          onInput: (value) => {
            if (Number.isFinite(value)) {
              state.lower = value;
              applyBound();
            }
          },
        })),
        ui.field('Upper', ui.numberInput({
          id: 'policy-upper',
          value: state.upper,
          step: 'any',
          onInput: (value) => {
            if (Number.isFinite(value)) {
              state.upper = value;
              applyBound();
            }
          },
        })),
      ),
      ui.note(
        'Both ends are included: admitted(value) is lower <= value <= upper. An upper limit below the lower one is not an empty range but a policy the source refuses to build, which validatePolicy reports.',
        'muted',
      ),
      ui.note(
        `${state.boundName} is now ${bound.lower} to ${bound.upper} inclusive, and the refusal it would state reads "${state.boundName.split('.').pop()} must be ${bound.lower} to ${bound.upper} inclusive, was <value>".`,
        'muted',
      ),
    );
  }

  /** Add or remove one member of one declared set, which is what makes a name admissible. */
  function setEditor() {
    return ui.panel(
      { title: 'Change one declared set' },
      ui.row(
        ui.field(
          'Declared set',
          ui.selectInput({
            id: 'policy-set',
            value: state.setName,
            options: [...SET_NAMES],
            onChange: (value) => {
              state.setName = value;
              render();
            },
          }),
          'a value outside its set is refused rather than corrected',
        ),
        ui.field('Member', ui.textInput({
          id: 'policy-member',
          value: state.member,
          placeholder: 'a name',
          onInput: (value) => {
            state.member = value;
          },
        })),
        ui.button('Add the member', () => addMember(true), { id: 'policy-add' }),
        ui.button('Remove the member', () => addMember(false), { id: 'policy-remove' }),
      ),
      ui.table({
        columns: ['Set', 'Members', 'Sorted, as the canonical form writes them'],
        rows: SET_NAMES.map((name) => [
          name,
          (state.policy.sets[name] ?? []).length,
          [...(state.policy.sets[name] ?? [])].sort().join(', ') || '(empty, which the source refuses)',
        ]),
        dense: true,
      }),
      ui.note(
        `the members the first release declares for ${state.setName}: ${
          (DECLARED_SETS.find((set) => valueOf(set.name) === state.setName)?.members ?? [])
            .map((member) => valueOf(member))
            .join(', ') || '(none)'
        }`,
        'muted',
      ),
    );
  }

  /** Resolve one value through the field's own reader, and show what that reader returns. */
  function resolutionEditor() {
    const entry = fieldAt(state.field);
    return ui.panel(
      { title: 'Resolve one value' },
      ui.row(
        ui.field(
          'Field',
          ui.selectInput({
            id: 'policy-field',
            value: state.field,
            options: [...SET_FIELDS, ...BOUND_FIELDS],
            onChange: (value) => {
              state.field = value;
              state.typed = BOUND_FIELDS.includes(value) ? '2500' : 'hourly';
              render();
            },
          }),
          entry === null ? '' : `${entry.reads} → ${entry.declaration ?? 'no declaration'}`,
        ),
        ui.field('Value', ui.textInput({
          id: 'policy-value',
          value: String(state.typed),
          placeholder: 'a name or a number',
          onInput: (value) => {
            state.typed = value;
            showOutcome();
          },
        })),
      ),
      outcome,
      ui.note(
        'A value outside a declared set is never corrected or clipped, and a bound the policy does not declare refuses every value of its field rather than admitting all of them. The resolution is the schema\'s own path, which is why the value test lives in the candidate view\'s engine.',
        'muted',
      ),
    );
  }

  /** The transcription: every declared set and every bound, with the line each came from. */
  function transcription() {
    return ui.panel(
      { title: 'The declarations themselves' },
      ui.table({
        columns: ['Declared set', 'Members', 'Line'],
        rows: DECLARED_SETS.map((set) => [
          valueOf(set.name),
          set.members.map((member) => valueOf(member)).join(', '),
          cite(set.declared),
        ]),
        dense: true,
      }),
      ui.table({
        columns: ['Bound', 'What the field is', 'Admitted', 'Line'],
        rows: BOUND_ROWS.map((row) => [row.name, row.unit, row.describe, row.source]),
        dense: true,
      }),
      ui.note(
        'The code stores bare decimals and names no unit, so the words in the second column are the tutorial page\'s own, quoted from its bounds table.',
        'muted',
      ),
      ui.note(
        `the three caps: max_conditions ${valueOf(LIMITS.max_conditions)}, max_condition_depth ${valueOf(LIMITS.max_condition_depth)}, max_features ${valueOf(LIMITS.max_features)}. The policy declares no sector for any instrument, and ${valueOf(SECTORS).length} sectors is that empty map.`,
        'muted',
      ),
    );
  }

  /** The refusal vocabulary, and each code this model produces on demand. */
  function refusalPanel() {
    return ui.panel(
      { title: 'The vocabulary', subtitle: 'a refusal is a code, a path and a message, and a value rather than an exception' },
      ui.table({
        columns: ['Code', 'The message the reader records', 'Reader'],
        rows: POLICY_REFUSALS.map((row) => [
          valueOf(row.code.code),
          valueOf(row.message),
          `${valueOf(row.where.where)} at ${cite(row.where)}`,
        ]),
        dense: true,
      }),
      ui.table({
        columns: ['Code this model can produce', 'How', 'Show it'],
        rows: DEMONSTRATIONS.map((demo) => [
          demo.code,
          `${demo.note}: ${demo.field}`,
          ui.button('Show it', () => showDemonstration(demo.code), { id: `policy-demo-${demo.code}` }),
        ]),
      }),
      ui.note(
        'The three construction rules the freeze check reports come from policy.py rather than from a reader: ' +
          Object.values(POLICY_RULES).map((rule) => valueOf(rule)).join('; ') +
          '.',
        'muted',
      ),
      ui.note(
        'feature_not_allowed and feature_not_declared are demonstrated by the candidate view, where a feature declaration and an alias reference exist to refuse.',
        'muted',
      ),
    );
  }

  /** Apply the edited limits to the policy, then rebuild what depends on them. */
  function applyBound() {
    state.policy = {
      ...state.policy,
      bounds: state.policy.bounds.map((bound) =>
        bound.name === state.boundName
          ? { ...bound, lower: state.lower, upper: state.upper, describe: `${state.lower} to ${state.upper} inclusive` }
          : bound,
      ),
    };
    render();
  }

  /** Add or remove one member of one declared set. */
  function addMember(add) {
    const members = state.policy.sets[state.setName] ?? [];
    const next = add
      ? members.includes(state.member)
        ? members
        : [...members, state.member]
      : members.filter((member) => member !== state.member);
    state.policy = { ...state.policy, sets: { ...state.policy.sets, [state.setName]: next } };
    render();
  }

  /** Reset every edited declaration, keeping the console's own campaign inputs. */
  function reset() {
    state.policy = defaultPolicy({
      featureAllowlist: [...state.policy.featureAllowlist],
      references: state.policy.references.map((entry) => ({ ...entry })),
    });
    render();
  }

  /** The one resolution the reader asked for, beside the rule it was checked against. */
  function showOutcome() {
    ui.clear(outcome);
    const entry = fieldAt(state.field);
    if (entry === null) {
      outcome.appendChild(ui.note(`no block declares ${state.field}`, 'warn'));
      return;
    }
    const result = resolveField(state.policy, state.field, coerce(entry, state.typed), ['close', 'volume']);
    outcome.appendChild(
      result.ok
        ? ui.verdict('admitted', JSON.stringify(result.value), 'ok', 'a declaration covers it, which says nothing about the idea')
        : ui.verdict(result.code, result.reason, 'bad', `at ${result.path}`),
    );
  }

  /** One refusal produced on demand, with the declaration it names. */
  function showDemonstration(code) {
    ui.clear(demonstrated);
    const demo = DEMONSTRATIONS.find((entry) => entry.code === code);
    const policy = demo.dropBound
      ? { ...state.policy, bounds: state.policy.bounds.filter((bound) => bound.name !== demo.dropBound) }
      : state.policy;
    const result = resolveField(policy, demo.field, demo.value, ['close', 'volume']);
    demonstrated.appendChild(
      ui.panel(
        { title: `produced: ${code}`, tone: 'bad' },
        ui.note(
          demo.dropBound
            ? `the bound ${demo.dropBound} deleted from the policy, then ${demo.field} declaring ${demo.value}`
            : `${demo.field} declaring ${JSON.stringify(demo.value)}`,
        ),
        result.ok
          ? ui.verdict('admitted', JSON.stringify(result.value), 'ok')
          : ui.verdict(result.code, result.reason, 'bad', `at ${result.path}`),
      ),
    );
  }

  // The five parts, in the order the contract fixes them.
  root.appendChild(
    ui.panel(
      { title: layer.title, subtitle: layer.tutorial },
      ui.block(
        'The idea',
        ui.quote(layer.idea, layer.tutorial),
        ui.verdict('How well it held up', layer.grade, 'ok', null),
        ui.note(
          `Four of the twenty-two acceptance criteria do not pass: ${data
            .failingCriteria()
            .map((row) => row.id)
            .join(', ')}. None of them is about this layer, and this view does not present them as passing.`,
          'warn',
        ),
      ),
      ui.block('The model', model),
      ui.block('What it refuses', refusals),
      ui.block('Where it goes wrong', ui.list(layer.caveats)),
      ui.block(
        'Where this came from',
        ui.sources(layer.sources),
        ui.sources([
          { label: 'the policy transcription', path: 'implementation/strategy-discovery-console/src/data/policy.js' },
          { label: 'the policy engine', path: 'implementation/strategy-discovery-console/src/engine/policy.js' },
          { label: 'the readers a value resolves through', path: 'implementation/strategy-discovery-console/src/engine/candidate.js' },
        ]),
        ui.note(
          `the six codes a value resolved through a declaration can carry: ${POLICY_REFUSAL_NAMES.join(', ')}.`,
          'muted',
        ),
      ),
    ),
  );

  render();
  showOutcome();
}
