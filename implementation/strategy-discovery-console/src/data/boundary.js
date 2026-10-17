/**
 * The execution boundary, transcribed.
 *
 * The console's boundary layer models four assertions the repository makes about a discovery process
 * that cannot send an order: the import graph reaches no adapter module, the only execution client a
 * worker is given is the sandbox one, no credential from the canonical list is in the environment the
 * boundary defines, and a candidate that names an execution venue is refused and recorded. This
 * module holds the repository's own words for those assertions, for the credential list and its rule
 * for adding a name, for the sandbox surface, for the capability allowlist that is asked for and not
 * yet built, and for the two refusals a candidate collects.
 *
 * Every value is wrapped in `fact(value, path, line, text)` with the verbatim supporting text, so
 * tools/check-facts.mjs fails when the file moves away from what the console claims. Nothing here
 * computes: the engine modules read these values, the views print them, and neither may restate one
 * from memory.
 */

import { fact } from './source.js';

const TEST = 'python/tests/integration/test_discovery_boundary.py';
const DESIGN = 'docs/design/strategy_discovery_design.md';
const PLAN = 'docs/design/strategy_discovery_implementation.md';
const README = 'tutorials/strategy-discovery/the-boundary/README.md';
const SCRIPT = 'scripts/strip-adapter-env.bash';
const SANDBOX = 'python/nautilus_trader/adapters/sandbox/__init__.py';
const SCHEMA = 'python/nautilus_trader/discovery/schema.py';

/** The canonical credential list's file, as a path rather than a claim about its contents. */
export const CREDENTIALS_PATH = SCRIPT;

/** The four assertions in the page's own sentence, which is the list the whole layer is built on. */
export const ASSERTIONS_SENTENCE = fact(
  'Four assertions make that absence observable rather than asserted: the import graph reaches no ' +
    'adapter module, the only execution client a worker is given is the sandbox one, no credential ' +
    "from the repository's canonical list is in the environment the boundary defines, and a " +
    'candidate that tries to name an execution venue is refused and the refusal is recorded.',
  README,
  22,
  "that absence observable rather than asserted: the import graph reaches no adapter module, the only\n" +
    "execution client a worker is given is the sandbox one, no credential from the repository's canonical\n" +
    'list is in the environment the boundary defines, and a candidate that tries to name an execution\n' +
    'venue is refused and the refusal is recorded.',
);

/**
 * The four assertions, one row each: what the design asks for, the fixture that runs it, and what the
 * page says the assertion cannot show. The limits are the reason the layer's grade is Mixed rather
 * than Strong, so they are data here rather than a footnote in the view.
 */
export const ASSERTIONS = [
  {
    id: 'dependency',
    order: 1,
    design: fact(
      "A dependency assertion that the discovery subpackage's import graph contains no live adapter " +
        'module and no order-submission call, run over the resolved module set rather than over source ' +
        'text.',
      DESIGN,
      1800,
      "1. A dependency assertion that the discovery subpackage's import graph contains no live adapter\n" +
        '   module and no order-submission call, run over the resolved module set rather than over source\n' +
        '   text.',
    ),
    fixture: fact(
      'test_fx_m6_1_the_discovery_import_graph_reaches_no_adapter_module',
      TEST,
      154,
      'def test_fx_m6_1_the_discovery_import_graph_reaches_no_adapter_module() -> None:',
    ),
    cannotShow: [
      fact(
        'an import-graph check is evidence rather than proof: dynamic imports, indirect callbacks and a ' +
          'shared execution stack can all evade a check made over module names',
        DESIGN,
        1810,
        'Those four tests are necessary and not sufficient, and the review is right that an import-graph check\n' +
          'is evidence rather than proof: dynamic imports, indirect callbacks and a shared execution stack can\n' +
          'all evade a check made over module names. The stronger control is a capability boundary expressed as',
      ),
    ],
  },
  {
    id: 'capability',
    order: 2,
    design: fact(
      'A capability assertion that the only execution client reachable from the discovery entry point is ' +
        'the sandbox one.',
      DESIGN,
      1803,
      '2. A capability assertion that the only execution client reachable from the discovery entry point is\n' +
        '   the sandbox one.',
    ),
    fixture: fact(
      'test_fx_m6_1_the_sandbox_venue_is_the_only_execution_client_a_worker_is_given',
      TEST,
      172,
      'def test_fx_m6_1_the_sandbox_venue_is_the_only_execution_client_a_worker_is_given() -> None:',
    ),
    cannotShow: [
      fact(
        "the stronger control is the shape of the worker environment, which no milestone builds yet " +
          "because the layout's `config.py` is unbuilt, so the worker allowlist is procedural today",
        PLAN,
        798,
        'over module names, and the stronger control is the shape of the worker environment, which no\n' +
          "milestone builds yet because the layout's `config.py` is unbuilt, so the worker allowlist is\n" +
          'procedural today. The design\'s environment assertion also asks that a configuration loader refuse a',
      ),
    ],
  },
  {
    id: 'environment',
    order: 3,
    design: fact(
      'An environment assertion that no adapter credential variable is set in the discovery process, and ' +
        'that the configuration loader refuses a credential in a policy file.',
      DESIGN,
      1805,
      '3. An environment assertion that no adapter credential variable is set in the discovery process, and\n' +
        '   that the configuration loader refuses a credential in a policy file.',
    ),
    fixture: fact(
      'test_fx_m6_1_no_credential_reaches_the_environment_discovery_is_run_in',
      TEST,
      189,
      'def test_fx_m6_1_no_credential_reaches_the_environment_discovery_is_run_in() -> None:',
    ),
    cannotShow: [
      fact(
        'the design also asks that a configuration loader refuse a credential in a policy file, and no ' +
          'policy-file loader exists in the package: `policy.py` owns the policy object, its digests and ' +
          'its freeze rules, so that half is not testable and is not claimed',
        PLAN,
        800,
        "procedural today. The design's environment assertion also asks that a configuration loader refuse a\n" +
          'credential in a policy file, and no policy-file loader exists in the package: `policy.py` owns the\n' +
          'policy object, its digests and its freeze rules, so that half is not testable and is not claimed,',
      ),
      fact(
        'the tests cannot make a machine with the operator\'s own credentials safe: the assertion is over ' +
          'the environment the boundary defines rather than over the shell the operator sits at, and the ' +
          'fixture reports the ambient case rather than failing on it',
        README,
        235,
        '- The tests cannot stop an operator from installing an adapter and running the discovery code with\n' +
          "  it, and they cannot make a machine with the operator's own credentials safe. If the layer is run on\n" +
          '  such a machine the protection is procedural, which is why the environment assertion is over the\n' +
          '  environment the boundary defines rather than over the shell the operator sits at; `EODHD_API_KEY` is\n' +
          '  set on the machine this was written on, and the fixture reports that ambient case rather than\n' +
          '  failing on it.',
      ),
    ],
  },
  {
    id: 'submission',
    order: 4,
    design: fact(
      'A negative test that a candidate attempting to name an execution venue is refused by the validator, ' +
        'with the refusal recorded.',
      DESIGN,
      1807,
      '4. A negative test that a candidate attempting to name an execution venue is refused by the\n' +
        '   validator, with the refusal recorded.',
    ),
    fixture: fact(
      'test_fx_m6_1_a_candidate_naming_an_execution_path_is_refused_and_recorded',
      TEST,
      214,
      'def test_fx_m6_1_a_candidate_naming_an_execution_path_is_refused_and_recorded(',
    ),
    cannotShow: [
      fact(
        'a refusal names the box and not the idea, so a candidate turned away for a field it should not ' +
          'have named has learned nothing about whether its hypothesis was any good',
        README,
        268,
        'The second thing to notice is that a refusal names the box and not the idea, so a candidate turned away\n' +
          'for a field it should not have named has learned nothing about whether its hypothesis was any good.',
      ),
      fact(
        'a passing boundary says nothing about whether any candidate is any good',
        README,
        245,
        '- A passing boundary says nothing about whether any candidate is any good. It shows that the layer\n' +
          '  cannot send an order and has nothing to authenticate one with; whether a tested idea earns anything\n' +
          '  after costs is a question the account-stage measurements answer, not this one.',
      ),
    ],
  },
];

/** The dependency assertion: the module prefix it forbids, the exports that would mean a surface, the
 *  three conjuncts in their fixed order, and the count observed when the assertion last ran. */
export const DEPENDENCY = {
  prefix: fact(
    'nautilus_trader.adapters',
    TEST,
    65,
    'ADAPTER_PREFIX = "nautilus_trader.adapters"',
  ),
  submissionExports: fact(
    ['ExecutionClient', 'OrderFactory', 'submit_order'],
    TEST,
    68,
    'SUBMISSION_EXPORTS = ("ExecutionClient", "OrderFactory", "submit_order")',
  ),
  adaptersEmpty: fact([], TEST, 165, 'assert graph["adapters"] == [], graph["adapters"]'),
  exportFloor: fact(200, TEST, 168, 'assert len(exports) > 200'),
  forbiddenEmpty: fact(
    [],
    TEST,
    169,
    'assert [name for name in exports if any(part in name for part in SUBMISSION_EXPORTS)] == []',
  ),
  observedExports: fact(
    249,
    README,
    103,
    '  the count observed is 249, grown from 231 when this work was added.',
  ),
  cleanInterpreter: fact(
    'A separate interpreter is started, the discovery package is imported in it, and it prints every ' +
      'module it loaded whose name begins with `nautilus_trader.adapters`, together with the names the ' +
      'package exports.',
    README,
    54,
    '1. Resolve the import graph in a clean interpreter. A separate interpreter is started, the discovery\n' +
      '   package is imported in it, and it prints every module it loaded whose name begins with\n' +
      '   `nautilus_trader.adapters`, together with the names the package exports. A clean interpreter is',
  ),
};

/** The canonical credential list: where it lives, how the fixture reads it rather than restating it,
 *  the bound and the three names asserted by name, and the script's own rule for adding a name. */
export const CREDENTIALS = {
  list: fact('ADAPTER_ENV_VARS=(', SCRIPT, 16, 'ADAPTER_ENV_VARS=('),
  count: fact(
    91,
    README,
    116,
    '  names, the first `AX_API_KEY` and the last `TWS_USERNAME`.',
  ),
  countSentence: fact(
    'the canonical list, read out of the script rather than copied into the test; it holds 91 names, ' +
      'the first `AX_API_KEY` and the last `TWS_USERNAME`',
    README,
    115,
    '- `named` is the canonical list, read out of the script rather than copied into the test; it holds 91\n' +
      '  names, the first `AX_API_KEY` and the last `TWS_USERNAME`.',
  ),
  floor: fact(80, TEST, 202, 'assert len(named) > 80'),
  first: fact('AX_API_KEY', SCRIPT, 17, '  AX_API_KEY'),
  last: fact('TWS_USERNAME', SCRIPT, 107, '  TWS_USERNAME'),
  anchors: [
    fact('BINANCE_API_KEY', TEST, 203, 'assert "BINANCE_API_KEY" in named'),
    fact('DATABENTO_API_KEY', TEST, 204, 'assert "DATABENTO_API_KEY" in named'),
    fact('EODHD_API_KEY', TEST, 205, 'assert "EODHD_API_KEY" in named'),
  ],
  read: fact(
    'It is read rather than restated, so this fixture cannot drift from the list `make pre-flight` ' +
      'unsets.',
    TEST,
    109,
    '    The list is `ADAPTER_ENV_VARS` in `scripts/strip-adapter-env.bash`, which the design names as the\n' +
      '    one place the repository declares them. It is read rather than restated, so this fixture cannot\n' +
      '    drift from the list `make pre-flight` unsets.',
  ),
  add: fact(
    'When a new adapter environment variable is introduced (API key, secret, passphrase, private key, ' +
      'vault, account address or index, URL override, or token read from the process environment), add ' +
      'it to ADAPTER_ENV_VARS below.',
    SCRIPT,
    6,
    '# When a new adapter environment variable is introduced (API key, secret,\n' +
      '# passphrase, private key, vault, account address or index, URL override, or\n' +
      '# token read from the process environment), add it to ADAPTER_ENV_VARS below.',
  ),
  exclude: fact(
    'Do not add core variables such as NAUTILUS_PATH, internal process markers such as ' +
      'BLOCKCHAIN_PROXY_CHILD, names that tests set for themselves with std::env::set_var, or names ' +
      'that are not environment variables',
    SCRIPT,
    9,
    '# Do not add core variables such as NAUTILUS_PATH, internal process markers\n' +
      '# such as BLOCKCHAIN_PROXY_CHILD, names that tests set for themselves with\n' +
      '# std::env::set_var, or names that are not environment variables (for example\n' +
      '# POLY_API_KEY and POLY_PASSPHRASE, which are Polymarket HTTP header names).',
  ),
};

/** The sandbox surface: the two names a worker is given, and the data-client name it is not given. */
export const SANDBOX_SURFACE = {
  config: fact('SandboxExecutionClientConfig', SANDBOX, 26, '    "SandboxExecutionClientConfig",'),
  factory: fact('SandboxExecutionClientFactory', SANDBOX, 27, '    "SandboxExecutionClientFactory",'),
  exactly: fact(
    '`sandbox` holds exactly `SandboxExecutionClientConfig` and `SandboxExecutionClientFactory`.',
    README,
    130,
    '- `sandbox` holds exactly `SandboxExecutionClientConfig` and `SandboxExecutionClientFactory`.',
  ),
  asymmetry: fact(
    '`data_path` is the data-client factory that a worker is deliberately not given.',
    README,
    131,
    '- `data_path` is the data-client factory that a worker is deliberately not given.',
  ),
};

/** The capability allowlist as the design asks for it, as the plan records its state, and the two
 *  roled capabilities the design names. The allowlist itself does not exist: the model says so. */
export const CAPABILITY = {
  asked: fact(
    'Discovery workers run in an environment with an explicit capability allowlist, not in the ' +
      "operator's working environment.",
    DESIGN,
    1815,
    '- Discovery workers run in an environment with an explicit capability allowlist, not in the\n' +
      "  operator's working environment.",
  ),
  installed: fact(
    'No broker or live execution adapter is installed there, so an import naming one fails for want of ' +
      'the package.',
    DESIGN,
    1817,
    '- No broker or live execution adapter is installed there, so an import naming one fails for want of\n' +
      '  the package.',
  ),
  noCredentialFile: fact(
    'No credential-bearing configuration file is mounted and no credential variable is set, so a ' +
      'submission path that survived has nothing to authenticate with.',
    DESIGN,
    1819,
    '- No credential-bearing configuration file is mounted and no credential variable is set, so a\n' +
      '  submission path that survived has nothing to authenticate with.',
  ),
  sandboxOnly: fact(
    'Only the sandbox venue is exposed to a strategy worker.',
    DESIGN,
    1821,
    '- Only the sandbox venue is exposed to a strategy worker.',
  ),
  noExecutionCapability: fact(
    'The orchestrator and the result-registration process carry no execution capability at all',
    DESIGN,
    1822,
    '- The orchestrator and the result-registration process carry no execution capability at all, which is\n' +
      '  the same separation Section 8.4 states for the append protocol.',
  ),
  mechanism: fact(
    'a capability check that a worker environment contains only allowlisted packages and that a ' +
      'submission attempt would have nothing to authenticate with',
    DESIGN,
    1985,
    '12. **The execution boundary is tested.** Adding a live adapter to the discovery import graph fails\n' +
      '    the dependency assertion, a candidate naming an execution venue is refused, and no broker\n' +
      '    credential, live adapter or order-submission path is present in the discovery environment or the\n' +
      '    shadow environment. Mechanism: the four boundary tests of Section 12, an environment assertion\n' +
      '    run over both entry points, and a capability check that a worker environment contains only\n' +
      '    allowlisted packages and that a submission attempt would have nothing to authenticate with.',
  ),
  deliverable: fact(
    '`FX-M6-1`: dependency assertion, capability allowlist, no credential, negative submission test',
    PLAN,
    749,
    '| M6.2       | Assert the execution boundary                            | `FX-M6-1`: dependency assertion, capability allowlist, no credential, negative submission test     |',
  ),
  unbuilt: fact(
    "the stronger control is the shape of the worker environment, which no milestone builds yet because " +
      "the layout's `config.py` is unbuilt, so the worker allowlist is procedural today",
    PLAN,
    799,
    "milestone builds yet because the layout's `config.py` is unbuilt, so the worker allowlist is\n" +
      "procedural today. The design's environment assertion also asks that a configuration loader refuse a",
  ),
};

/** The two roles the design names as carrying no execution capability, and the one that carries the
 *  sandbox venue, with the sentence each reading comes from. */
export const AUTHORITY = [
  {
    id: 'strategy-worker',
    execution: 'sandbox',
    dataClient: false,
    credential: false,
    note: fact(
      'Only the sandbox venue is exposed to a strategy worker.',
      DESIGN,
      1821,
      '- Only the sandbox venue is exposed to a strategy worker.',
    ),
  },
  {
    id: 'orchestrator',
    execution: null,
    dataClient: false,
    credential: false,
    note: fact(
      'The orchestrator and the result-registration process carry no execution capability at all',
      DESIGN,
      1822,
      '- The orchestrator and the result-registration process carry no execution capability at all, which is\n' +
        '  the same separation Section 8.4 states for the append protocol.',
    ),
  },
  {
    id: 'result-registration',
    execution: null,
    dataClient: false,
    credential: false,
    note: fact(
      'The orchestrator and the result-registration process carry no execution capability at all',
      DESIGN,
      1822,
      '- The orchestrator and the result-registration process carry no execution capability at all, which is\n' +
        '  the same separation Section 8.4 states for the append protocol.',
    ),
  },
];

/** The refusal vocabulary this slice produces: the two codes, their message templates, and the two
 *  paths the fixture asserts. The schema returns the refusal rather than raising it. */
export const REFUSALS = {
  unknownField: fact('block_unknown_field', SCHEMA, 296, 'BLOCK_UNKNOWN_FIELD = "block_unknown_field"'),
  codeReference: fact(
    'code_reference_refused',
    SCHEMA,
    312,
    'CODE_REFERENCE_REFUSED = "code_reference_refused"',
  ),
  unknownFieldMessage: fact(
    'the {path} block does not declare {key}',
    SCHEMA,
    1012,
    '                    f"the {self.path} block does not declare {key}",',
  ),
  codeFieldMessage: fact(
    'a specification is data and may not declare {key}',
    SCHEMA,
    515,
    '                        f"a specification is data and may not declare {key}",',
  ),
  nonPlainMessage: fact(
    'a specification is data and may not carry a {type}',
    SCHEMA,
    533,
    '                f"a specification is data and may not carry a {type(payload).__name__}",',
  ),
  executableMessage: fact(
    'a specification value may not be executable code, {value} is',
    SCHEMA,
    544,
    '                        f"a specification value may not be executable code, {payload!r} is",',
  ),
  returned: fact(
    'The refusal carries the code and the path it was raised against, not a message to be interpreted.',
    README,
    73,
    '5. Refuse a candidate that names an execution path. A candidate specification is data, so it has no\n' +
      '   field for a venue or an adapter; a field named like one is refused as an unknown field, and a value\n' +
      '   shaped like an import is refused as a code reference. The refusal carries the code and the path it\n' +
      '   was raised against, not a message to be interpreted.',
  ),
  shape: fact(
    ['code', 'path', 'message'],
    SCHEMA,
    352,
    '        return {"code": self.code.value, "path": self.path, "message": self.message}',
  ),
  venue: fact(
    [['block_unknown_field', 'data.venue']],
    TEST,
    235,
    '    assert refused(submission) == ((RefusalCode.BLOCK_UNKNOWN_FIELD.value, "data.venue"),)',
  ),
  modulePath: fact(
    [['code_reference_refused', 'universe.module_path']],
    TEST,
    244,
    '    assert refused(code) == ((RefusalCode.CODE_REFERENCE_REFUSED.value, "universe.module_path"),)',
  ),
  legal: fact(
    true,
    TEST,
    247,
    '    assert compile_specification(legal_payload(), policy, registry).accepted is True',
  ),
};

/**
 * The legal candidate's own field names, which are what make a venue field an undeclared one. The
 * fixture mutates this payload one field at a time, so the refusals are about the declared field set
 * rather than about a payload shape invented for the console.
 */
export const LEGAL_BLOCKS = {
  source: fact(
    'from tests.unit.discovery.fixtures import legal_payload',
    TEST,
    52,
    'from tests.unit.discovery.fixtures import legal_payload',
  ),
  declared: fact(
    {
      universe: [
        'asset_classes',
        'membership_policy',
        'source',
        'min_price',
        'min_dollar_volume',
        'exclusions',
      ],
      data: [
        'signal_frequency',
        'decision_time',
        'execution_eligibility',
        'corporate_action_treatment',
      ],
    },
    'python/tests/unit/discovery/fixtures.py',
    168,
    '        "universe": {\n' +
      '            "asset_classes": ["EQUITY"],\n' +
      '            "membership_policy": "point_in_time",\n' +
      '            "source": "catalog.us_equities",\n' +
      '            "min_price": "5",\n' +
      '            "min_dollar_volume": "1000000",\n' +
      '            "exclusions": [],\n' +
      '        },\n' +
      '        "data": {\n' +
      '            "signal_frequency": "daily",\n' +
      '            "decision_time": "session_close",\n' +
      '            "execution_eligibility": "next_session_open",\n' +
      '            "corporate_action_treatment": "adjusted",',
  ),
};

/**
 * The field names that ask for code rather than data, at any depth of a payload. A specification
 * naming one of these is refused by name, so the refusal says what was asked for.
 */
export const CODE_FIELD_NAMES = fact(
  [
    'callable',
    'class',
    'class_path',
    'code',
    'code_path',
    'entry_point',
    'eval',
    'exec',
    'expression',
    'hook',
    'import',
    'imports',
    'lambda',
    'module',
    'module_path',
    'script',
    'source_code',
    'strategy_path',
  ],
  SCHEMA,
  69,
  '_CODE_FIELD_NAMES: frozenset[str] = frozenset(',
);

/**
 * The value shapes that are code rather than data: an import, an evaluation, a shell call, or the
 * `module:attribute` form the repository uses to name an importable strategy. The first match wins.
 */
export const CODE_VALUE_PATTERNS = [
  fact('\\b__import__\\b', SCHEMA, 95, '    re.compile(r"\\b__import__\\b"),'),
  fact('\\b(?:eval|exec)\\s*\\(', SCHEMA, 96, '    re.compile(r"\\b(?:eval|exec)\\s*\\("),'),
  fact(
    '\\b(?:os\\.system|subprocess|importlib)\\b',
    SCHEMA,
    97,
    '    re.compile(r"\\b(?:os\\.system|subprocess|importlib)\\b"),',
  ),
  fact(
    '^[A-Za-z_][\\w]*(?:\\.[A-Za-z_][\\w]*)*:[A-Za-z_][\\w]*$',
    SCHEMA,
    98,
    '    re.compile(r"^[A-Za-z_][\\w]*(?:\\.[A-Za-z_][\\w]*)*:[A-Za-z_][\\w]*$"),',
  ),
];

/** What the fixture records, and what the audit reads back: a refusal is never a proposal attempt and
 *  never an executed experiment. */
export const RECORDED = {
  candidate: fact(
    'sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
    TEST,
    62,
    'CANDIDATE = "sha256:" + "a" * 64',
  ),
  record: fact(
    'inventory.record_refusal(CANDIDATE, code=error.code.value, path=error.path)',
    TEST,
    253,
    '        inventory.record_refusal(CANDIDATE, code=error.code.value, path=error.path)',
  ),
  outcome: fact('refused', TEST, 259, '    assert judgement.outcome == "refused"'),
  attempts: fact([], TEST, 260, '    assert judgement.attempts == ()'),
  refusals: fact(
    ['block_unknown_field', 'code_reference_refused'],
    TEST,
    261,
    '    assert judgement.refusals == (\n' +
      '        RefusalCode.BLOCK_UNKNOWN_FIELD.value,\n' +
      '        RefusalCode.CODE_REFERENCE_REFUSED.value,\n' +
      '    )',
  ),
  proposalAttempts: fact(0, TEST, 265, '    assert inventory.counts().proposal_attempts == 0'),
  executedExperiments: fact(0, TEST, 266, '    assert inventory.counts().executed_experiments == 0'),
};
