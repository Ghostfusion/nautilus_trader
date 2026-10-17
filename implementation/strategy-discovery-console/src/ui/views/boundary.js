/**
 * The execution boundary: four assertions that make an absence observable rather than asserted.
 *
 * This view models `tutorials/strategy-discovery/the-boundary/README.md`, its four assertions in
 * `python/tests/integration/test_discovery_boundary.py`, and design Section 12 of
 * `docs/design/strategy_discovery_design.md`. The reader can put a live adapter into a simulated
 * import graph, add a credential to a simulated environment, or declare a field a candidate should not
 * have, and watch the boundary answer. Each of the four assertions is shown with what it cannot show,
 * because the page's own grade is Mixed and says why.
 *
 * The implementation is `../data/boundary.js` for the transcription and `../engine/boundary.js` for
 * the model; every number and code on screen comes from one of the two, and nothing here restates a
 * refusal code, a count or a message by hand.
 */

import { cite, sourceOf, valueOf } from '../../data/source.js';
import {
  ASSERTIONS,
  ASSERTIONS_SENTENCE,
  CAPABILITY,
  CREDENTIALS,
  DEPENDENCY,
  LEGAL_BLOCKS,
  REFUSALS,
  SANDBOX_SURFACE,
} from '../../data/boundary.js';
import {
  REFUSAL_CODES,
  authorityOf,
  capacities,
  checkEnvironment,
  executedSurface,
  refusalPairs,
  refuseSubmission,
  recordRefusals,
} from '../../engine/boundary.js';

export const meta = {
  id: 'boundary',
  title: 'The boundary: execution authority absent rather than switched off',
  order: 12,
  tutorial: 'tutorials/strategy-discovery/the-boundary/README.md',
};

const ROLES = ['strategy-worker', 'orchestrator', 'result-registration'];

/** One transcribed assert, with the file and line it was read from, so the reader sees the fixture. */
function assertionLine(node) {
  return `${cite(node)}  ${sourceOf(node).text.replace(/\s+/g, ' ').trim()}`;
}

export function mount(root, ctx) {
  const ui = ctx.ui;
  const layer = ctx.data.layer(meta.id);
  const names = ctx.data.credentials();
  const observed = executedSurface();
  const capability = capacities();
  const declared = valueOf(LEGAL_BLOCKS.declared);

  const state = {
    adapters: [],
    named: [],
    environment: {},
    pending: '',
    role: ROLES[0],
    refusals: [],
  };

  root.appendChild(
    ui.panel(
      { title: 'The idea', subtitle: `${layer.order}. ${layer.tutorial}`, tone: 'accent' },
      ui.quote(layer.idea, layer.tutorial),
      ui.verdict('How well it held up', 'Mixed', 'warn', layer.grade),
      ui.quote(valueOf(ASSERTIONS_SENTENCE), layer.tutorial),
      ui.note(
        'Nothing was switched off: the pieces were never put in. The four assertions below are what ' +
          'makes that absence observable, and each is followed by what it cannot show.',
      ),
    ),
  );

  // ---- 2. The model ---------------------------------------------------------------------------

  const assertionsTable = ui.table({
    dense: true,
    columns: ['#', 'The assertion', 'The fixture that runs it', 'What it cannot show'],
    rows: ASSERTIONS.map((assertion) => [
      valueOf(assertion.order),
      valueOf(assertion.design),
      valueOf(assertion.fixture),
      assertion.cannotShow.map(valueOf).join(' '),
    ]),
  });

  const surfaceLive = ui.stack();
  const authorityLive = ui.stack();
  const environmentLive = ui.stack();
  const refusalLive = ui.stack();

  const adapterInput = ui.textInput({
    id: 'boundary-adapter',
    value: '',
    placeholder: 'nautilus_trader.adapters.binance',
    onInput: (value) => {
      state.pending = value;
    },
  });
  const exportInput = ui.textInput({
    id: 'boundary-export',
    value: '',
    placeholder: 'submit_order',
    onInput: (value) => {
      state.pending = value;
    },
  });

  root.appendChild(
    ui.panel(
      { title: 'The model', subtitle: 'Put something in, and watch the assertion answer' },
      ui.block('The four assertions, and what each cannot show', assertionsTable),

      ui.block(
        'The dependency assertion',
        ui.note(
          'A clean interpreter loads the discovery package and reports every module whose name ' +
            'begins with the adapter prefix, the number of names the package offers, and whether any ' +
            'offered name is an execution surface of its own.',
        ),
        ui.list(
          [
            valueOf(DEPENDENCY.prefix),
            valueOf(DEPENDENCY.submissionExports).join(', '),
            assertionLine(DEPENDENCY.adaptersEmpty),
            assertionLine(DEPENDENCY.exportFloor),
            assertionLine(DEPENDENCY.forbiddenEmpty),
          ],
          { className: 'sources' },
        ),
        ui.grid(
          2,
          ui.field(
            'Reach a module from the graph',
            ui.row(adapterInput, ui.button('Add it', () => addAdapter(), { kind: 'primary' })),
            `The observed graph reaches none of these: ${valueOf(DEPENDENCY.cleanInterpreter)}`,
          ),
          ui.field(
            'Offer a name from the package',
            ui.row(exportInput, ui.button('Add it', () => addExport(), { kind: 'primary' })),
            'The observed reading offers none of the three forbidden substrings. The package itself ' +
              'is not read here: the count is the one the page records.',
          ),
        ),
        ui.row(ui.button('Start the graph again', () => resetGraph())),
        surfaceLive,
      ),

      ui.block(
        'The capability assertion',
        ui.note(
          'The asymmetry is the capability: a worker can turn an observation into a simulated fill ' +
            'inside the process, and no order leaves it.',
        ),
        ui.table({
          dense: true,
          columns: ['The sandbox package offers', 'The worker is given it'],
          rows: capability.exports.map((name) => [
            name,
            ui.badge(
              name.includes('DataClient') ? 'data client: not allowed' : 'execution client',
              name.includes('DataClient') ? 'bad' : 'ok',
            ),
          ]),
        }),
        ui.verdict(
          'The capability reading',
          capability.holds ? 'holds' : 'fails',
          capability.holds ? 'ok' : 'bad',
          'No offered name contains a data client, so the data path a worker is deliberately not ' +
            'given is absent.',
        ),
        ui.quote(valueOf(SANDBOX_SURFACE.exactly), layer.tutorial),
        ui.note(valueOf(SANDBOX_SURFACE.asymmetry)),
        ui.group(
          'The boundary the design asks for, as the environment rather than as an inspection of code',
          ui.list(
            [
              CAPABILITY.asked,
              CAPABILITY.installed,
              CAPABILITY.noCredentialFile,
              CAPABILITY.sandboxOnly,
              CAPABILITY.noExecutionCapability,
            ].map(valueOf),
          ),
        ),
        ui.note(
          `The stronger control is ${capability.allowlist.why}. Criterion 12's mechanism names ` +
            `${valueOf(CAPABILITY.mechanism)}, and the plan carries it as part of the M6.2 deliverable ` +
            `along with the assertions that did run: ${valueOf(CAPABILITY.deliverable)}. Nothing here ` +
            'checks a package set, because the repository has none to check.',
          'warn',
        ),
      ),

      ui.block(
        'The authority each role holds',
        ui.field(
          'The role',
          ui.selectInput({
            id: 'boundary-role',
            value: state.role,
            options: ROLES,
            onChange: (value) => {
              state.role = value;
              renderAuthority();
            },
          }),
          'The design names three roles and says what each may reach.',
        ),
        authorityLive,
      ),

      ui.block(
        'The environment assertion',
        ui.note(
          `The canonical list is read out of the script rather than copied, and it holds ` +
            `${names.length} names. Add one to the environment discovery runs in and see what the ` +
            'boundary does with it: a name on the list is refused into the environment the boundary ' +
            'defines, while the same name in the operator\u2019s shell is only reported.',
        ),
        environmentControls(),
        environmentLive,
        ui.list(
          [valueOf(CREDENTIALS.read), valueOf(CREDENTIALS.add), valueOf(CREDENTIALS.exclude)],
          { className: 'sources' },
        ),
      ),
    ),
  );

  // ---- 3. What it refuses ---------------------------------------------------------------------

  root.appendChild(
    ui.panel(
      { title: 'What it refuses', subtitle: 'Two field names, refused for two different reasons' },
      ui.table({
        dense: true,
        columns: ['Code', 'The condition that produces it', 'The message'],
        rows: [
          [
            ui.badge(REFUSAL_CODES.unknownField, 'bad'),
            'A payload carries a field the block never declares; the block reader refuses it last.',
            valueOf(REFUSALS.unknownFieldMessage).replace('{path}', 'data').replace('{key}', 'venue'),
          ],
          [
            ui.badge(REFUSAL_CODES.codeReference, 'bad'),
            'A field name asks for code, at any depth, or a value is shaped like code.',
            valueOf(REFUSALS.codeFieldMessage).replace('{key}', 'module_path'),
          ],
        ],
      }),
      ui.note(
        'The compiler scans the whole payload for code references before any block is read, so a ' +
          'payload that names both kinds is refused for the code reference first. A candidate whose ' +
          'fields are all declared data is accepted, so what is refused is the field names rather ' +
          'than the shape of the candidate.',
      ),
      ui.note(
        `The field sets the model is told about are the fixture's own: ${valueOf(LEGAL_BLOCKS.source)} ` +
          'is the payload the two negative cases mutate one field at a time, so the declared names ' +
          `are ${Object.entries(declared).map(([block, names]) => `${block} (${names.length} fields)`).join(', ')}.`,
      ),
      ui.quote(valueOf(REFUSALS.returned), layer.tutorial),
      ui.note(
        `The refusal is returned as ${valueOf(REFUSALS.shape).join(', ')}: the closed code for the ` +
          'reason, the path the reason was raised against, and the message. Two compilations of one ' +
          'payload produce the same refusal set, so a refusal is reproducible rather than incidental.',
      ),
      ui.row(
        ui.button('Declare data.venue', () => collect({ ...legalPayload(), data: { ...dataFields(), venue: 'BINANCE' } })),
        ui.button('Declare universe.module_path', () =>
          collect({
            ...legalPayload(),
            universe: { ...universeFields(), module_path: 'nautilus_trader.adapters.binance' },
          }),
        ),
        ui.button('Record what has been refused', () => renderRefusals()),
        ui.button('Start again', () => {
          state.refusals = [];
          renderRefusals();
        }),
      ),
      refusalLive,
    ),
  );

  // ---- 4. Where it goes wrong -----------------------------------------------------------------

  root.appendChild(
    ui.panel(
      { title: 'Where it goes wrong', subtitle: 'The page\u2019s own caveats, verbatim' },
      ui.list(layer.caveats),
    ),
  );

  // ---- 5. Where this came from ----------------------------------------------------------------

  root.appendChild(
    ui.panel(
      { title: 'Where this came from', subtitle: 'The files this layer reads' },
      ui.sources(layer.sources),
      ui.note(
        `The four assertions are the design's Section 12 items and the four fx_m6_1 tests in the ` +
          'boundary fixture; the canonical list is read from the script rather than restated, so the ' +
          `check can fail when the two drift.`,
      ),
    ),
  );

  function dataFields() {
    return Object.fromEntries(declared.data.map((name) => [name, 'declared']));
  }

  function universeFields() {
    return Object.fromEntries(declared.universe.map((name) => [name, 'declared']));
  }

  function legalPayload() {
    return { data: dataFields(), universe: universeFields() };
  }

  function addAdapter() {
    const name = state.pending.trim();
    if (name === '') return;
    state.adapters = [...state.adapters, name];
    state.pending = '';
    renderSurface();
  }

  function addExport() {
    const name = state.pending.trim();
    if (name === '') return;
    state.named = [...state.named, name];
    state.pending = '';
    renderSurface();
  }

  function resetGraph() {
    state.adapters = [];
    state.named = [];
    renderSurface();
  }

  function collect(refusal) {
    if (refusal.ok) return;
    state.refusals = [...state.refusals, refusal];
    renderRefusals();
  }

  function environmentControls() {
    const input = ui.textInput({
      id: 'boundary-credential',
      value: '',
      placeholder: 'EODHD_API_KEY',
      onInput: (value) => {
        state.pending = value;
      },
    });
    return ui.field(
      'A variable in the environment discovery runs in',
      ui.row(
        input,
        ui.button('Add it', () => {
          const name = state.pending.trim();
          if (name === '') return;
          state.environment = { ...state.environment, [name]: 'set' };
          state.pending = '';
          renderEnvironment();
        }, { kind: 'primary' }),
        ui.button('Clear the environment', () => {
          state.environment = {};
          renderEnvironment();
        }),
      ),
      `The fixture asserts the list holds more than ${valueOf(CREDENTIALS.floor)} names and that three ` +
        `of them are present by name. It holds ${names.length}.`,
    );
  }

  function renderSurface() {
    const surface = executedSurface({
      adapters: state.adapters,
      exports: { count: observed.count, named: state.named },
    });
    ui.clear(surfaceLive);
    surfaceLive.appendChild(
      ui.metrics(
        ui.metric('Adapters reached', surface.adapters.length, 'asserted to be empty', surface.adapters.length === 0 ? 'ok' : 'bad'),
        ui.metric('Names offered', surface.count, `asserted to be above ${surface.floor}`, surface.count > surface.floor ? 'ok' : 'bad'),
        ui.metric('Execution surface offered', surface.forbidden.length, 'asserted to be empty', surface.forbidden.length === 0 ? 'ok' : 'bad'),
      ),
    );
    surfaceLive.appendChild(
      ui.verdict(
        'The dependency assertion',
        surface.holds ? 'holds' : 'fails',
        surface.holds ? 'ok' : 'bad',
        surface.holds
          ? 'No adapter module is reachable, the export surface is above its floor and none of it is ' +
              'an execution surface of the package\x27s own.'
          : 'One of the three conjuncts fails: the graph reaches an adapter, the export count is at ' +
              'or below the floor, or an offered name is an execution surface.',
      ),
    );
    if (surface.adapters.length > 0) {
      surfaceLive.appendChild(
        ui.note(`Reached adapter modules: ${surface.adapters.join(', ')}`, 'bad'),
      );
    }
    if (surface.forbidden.length > 0) {
      surfaceLive.appendChild(
        ui.note(`Offered names that would mean an execution surface: ${surface.forbidden.join(', ')}`, 'bad'),
      );
    }
  }

  function renderAuthority() {
    const authority = authorityOf(state.role);
    ui.clear(authorityLive);
    authorityLive.appendChild(
      ui.table({
        dense: true,
        columns: ['Role', 'Execution capability', 'Data client', 'Credential'],
        rows: [
          [
            authority.role,
            authority.execution === null ? ui.badge('none at all', 'ok') : ui.badge(authority.execution, 'accent'),
            ui.badge(authority.dataClient ? 'given' : 'absent', authority.dataClient ? 'bad' : 'ok'),
            ui.badge(authority.credential ? 'set' : 'absent', authority.credential ? 'bad' : 'ok'),
          ],
        ],
      }),
    );
    authorityLive.appendChild(ui.quote(authority.note, 'docs/design/strategy_discovery_design.md'));
  }

  function renderEnvironment() {
    const check = checkEnvironment(state.environment, names);
    ui.clear(environmentLive);

    environmentLive.appendChild(
      ui.metrics(
        ui.metric('Names in the environment', check.declared.length, 'as you declared it'),
        ui.metric('On the canonical list', check.ambient.length, check.ambient.join(', ') || 'none', check.ambient.length === 0 ? 'ok' : 'bad'),
        ui.metric('In the environment the boundary defines', Object.keys(check.stripped).length, 'the declaration with the list removed'),
        ui.metric('The list itself', check.list.size, `floor ${check.list.floor}`, check.list.holds ? 'ok' : 'bad'),
      ),
    );

    if (check.refusal === null) {
      environmentLive.appendChild(
        ui.verdict(
          'The boundary',
          'defines this environment',
          'ok',
          'Nothing in it is on the canonical list, so a submission path that survived the import ' +
            'check would have nothing to authenticate with.',
        ),
      );
      return;
    }

    environmentLive.appendChild(
      ui.verdict(
        'The boundary',
        'refuses the declaration',
        'bad',
        `A name on the canonical list is in the environment the boundary defines, so this is not the ` +
          'environment the boundary is about: it is the ambient case, which the fixture reports rather ' +
          'than fails on.',
      ),
    );
    environmentLive.appendChild(
      ui.table({
        dense: true,
        columns: ['Code', 'Path', 'Message'],
        rows: [[ui.badge(check.refusal.code, 'bad'), check.refusal.path, check.refusal.reason]],
      }),
    );
    environmentLive.appendChild(
      ui.note(
        'The repository states no code for this case: the assertion removes the canonical names from ' +
          'the environment rather than refusing it, and the design\u2019s other half of item 3 - a loader ' +
          'that refuses a credential in a policy file - has no loader to test. So the refusal above is ' +
          'reported with the schema\u2019s unknown-field code for the declaration that carries the name, ' +
          'and the loader half is not claimed.',
        'warn',
      ),
    );
  }

  function renderRefusals() {
    const accepted = refuseSubmission(legalPayload(), declared);
    ui.clear(refusalLive);
    refusalLive.appendChild(
      ui.metrics(
        ui.metric(
          'The legal candidate',
          accepted.ok ? 'accepted' : 'refused',
          'the fixture compiles it unchanged, and accepted is True',
          accepted.ok ? 'ok' : 'bad',
        ),
      ),
    );
    if (state.refusals.length === 0) {
      refusalLive.appendChild(
        ui.note('No field has been declared yet, so nothing has been refused.'),
      );
      return;
    }

    refusalLive.appendChild(
      ui.table({
        dense: true,
        columns: ['Code', 'Path', 'Message'],
        rows: state.refusals.map((refusal) => [
          ui.badge(refusal.code, 'bad'),
          refusal.path,
          refusal.reason,
        ]),
      }),
    );

    const record = recordRefusals(refusalPairs(state.refusals));
    refusalLive.appendChild(
      ui.metrics(
        ui.metric('Outcome', record.outcome, 'read back from the record', 'bad'),
        ui.metric('Attempts', record.attempts.length, 'a refusal is never an attempt', 'ok'),
        ui.metric('Refusals', record.refusals.length, record.refusals.join(', ')),
        ui.metric('Proposal attempts', record.counts.proposal_attempts, 'counted', 'ok'),
        ui.metric('Executed experiments', record.counts.executed_experiments, 'counted', 'ok'),
      ),
    );
    refusalLive.appendChild(
      ui.note(
        'The refusal carries a code and the path it was raised against, not a message to be ' +
          `interpreted, and the audit reads them back for ${record.candidate}. A refusal counted as a ` +
          'proposal attempt or as an executed experiment would be the boundary failing quietly.',
      ),
    );
  }

  renderSurface();
  renderAuthority();
  renderEnvironment();
  renderRefusals();
}
