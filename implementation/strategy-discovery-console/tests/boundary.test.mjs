/**
 * Tests for the execution boundary. The cases mirror the repository's own four `fx_m6_1` assertions
 * and the two refusal pairs the boundary fixture asserts, so a reader who breaks one of the four sees
 * it here rather than as a boundary that quietly passes.
 */

import { describe, expect, test } from 'bun:test';

import { valueOf } from '../src/data/source.js';
import { credentials } from '../src/data/layers.js';
import {
  CODE_FIELD_NAMES,
  CREDENTIALS,
  DEPENDENCY,
  LEGAL_BLOCKS,
  REFUSALS,
  SANDBOX_SURFACE,
} from '../src/data/boundary.js';
import {
  authorityOf,
  capacities,
  checkEnvironment,
  executedSurface,
  recordRefusals,
  refusalPairs,
  refuseSubmission,
} from '../src/engine/boundary.js';

const NAMES = credentials();

describe('the canonical credential list', () => {
  test('holds the 91 names the page records, first and last as transcribed', () => {
    // If this drifted, the console's list table and the drift check would disagree with the script.
    expect(NAMES.length).toBe(valueOf(CREDENTIALS.count));
    expect(NAMES).toBeArrayOfSize(91);
    expect(NAMES[0]).toBe(valueOf(CREDENTIALS.first));
    expect(NAMES[NAMES.length - 1]).toBe(valueOf(CREDENTIALS.last));
  });

  test('carries the three names the fixture asserts by name', () => {
    // A list that lost an anchored name would pass the raw count and fail the fixture's own check.
    for (const anchor of CREDENTIALS.anchors) expect(NAMES).toContain(valueOf(anchor));
  });

  test('reads the same list the repository reads, from the same declaration', () => {
    // The list is not restated in the console: it is the transcription of ADAPTER_ENV_VARS.
    expect(valueOf(CREDENTIALS.list)).toBe('ADAPTER_ENV_VARS=(');
    expect(valueOf(CREDENTIALS.floor)).toBe(80);
    expect(NAMES.length).toBeGreaterThan(valueOf(CREDENTIALS.floor));
  });
});

describe('the environment assertion', () => {
  test('finds a planted credential and names it', () => {
    // The control the boundary view offers: a credential in the environment is reported, never hidden.
    const check = checkEnvironment({ PATH: '/usr/bin', EODHD_API_KEY: 'set' }, NAMES);
    expect(check.clean).toBe(false);
    expect(check.ok).toBe(false);
    expect(check.ambient).toEqual(['EODHD_API_KEY']);
    expect(check.refusal.code).toBe('block_unknown_field');
    expect(check.refusal.path).toBe('environment.EODHD_API_KEY');
    expect(check.refusal.reason).toBe('the environment block does not declare EODHD_API_KEY');
  });

  test('passes a clean environment and defines it unchanged', () => {
    // The half that is checkable: the environment the boundary defines holds no credential at all.
    const check = checkEnvironment({ PATH: '/usr/bin', HOME: '/home/reader' }, NAMES);
    expect(check.ok).toBe(true);
    expect(check.clean).toBe(true);
    expect(check.ambient).toEqual([]);
    expect(check.refusal).toBe(null);
    expect(check.stripped).toEqual({ HOME: '/home/reader', PATH: '/usr/bin' });
  });

  test('strips a credential out of the environment the boundary defines', () => {
    // This is the fixture's own arithmetic: the boundary's environment is the caller's minus the list.
    const check = checkEnvironment({ EODHD_API_KEY: 'set', PATH: '/usr/bin' }, NAMES);
    expect(check.stripped).toEqual({ PATH: '/usr/bin' });
    expect(check.declared).toEqual(['EODHD_API_KEY', 'PATH']);
  });

  test('reports the list size against the floor and the three anchors', () => {
    // The size half of the assertion, so a shortened list is caught here rather than by the count table.
    const check = checkEnvironment({}, NAMES);
    expect(check.list.size).toBe(91);
    expect(check.list.floor).toBe(80);
    expect(check.list.holds).toBe(true);
    expect(check.list.anchors.every((anchor) => anchor.named)).toBe(true);
  });
});

describe('the dependency assertion', () => {
  test('holds on the observed graph and fails when an adapter is reached', () => {
    // A live adapter in the graph is the regression the assertion exists to catch.
    expect(executedSurface().holds).toBe(true);
    expect(executedSurface().count).toBe(valueOf(DEPENDENCY.observedExports));

    const live = executedSurface({
      adapters: ['nautilus_trader.adapters.binance', 'nautilus_trader.discovery'],
    });
    expect(live.adapters).toEqual(['nautilus_trader.adapters.binance']);
    expect(live.reached).toEqual([
      'nautilus_trader.adapters.binance',
      'nautilus_trader.discovery',
    ]);
    expect(live.holds).toBe(false);
  });

  test('fails when an offered name is an execution surface of its own', () => {
    // The export half has teeth independently of the module half: reaching no adapter is not enough.
    const surface = executedSurface({
      adapters: [],
      exports: { count: 249, named: ['submit_order'] },
    });
    expect(surface.forbidden).toEqual(['submit_order']);
    expect(surface.holds).toBe(false);
  });

  test('fails a short export list even with nothing forbidden in it', () => {
    // The floor is the conjunct that catches a package that stopped offering what it used to offer.
    const short = executedSurface({ adapters: [], exports: { count: 3 } });
    expect(short.forbidden).toEqual([]);
    expect(short.count).toBe(3);
    expect(short.holds).toBe(false);
  });

  test('reads the forbidden names from the transcription rather than from the model', () => {
    // The three substrings are the transcription's, and the observed reading offers none of them.
    expect(executedSurface().named).toEqual([]);
    expect(valueOf(DEPENDENCY.submissionExports)).toEqual([
      'ExecutionClient',
      'OrderFactory',
      'submit_order',
    ]);
  });
});

describe('the capability assertion', () => {
  test('gives a worker the sandbox execution client and no data client', () => {
    // Both halves of the asymmetry are asserted on the same list.
    const capability = capacities();
    expect(capability.holds).toBe(true);
    expect(capability.executionClient).toBe(true);
    expect(capability.executionConfig).toBe(true);
    expect(capability.dataClients).toEqual([]);
    expect(capability.exports).toEqual([
      valueOf(SANDBOX_SURFACE.config),
      valueOf(SANDBOX_SURFACE.factory),
    ]);
  });

  test('fails when a data-client factory appears in the sandbox package', () => {
    // The data path a worker is deliberately not given; a worker with it could fetch its own data.
    const capability = capacities([
      'SandboxExecutionClientConfig',
      'SandboxExecutionClientFactory',
      'SandboxDataClientFactory',
    ]);
    expect(capability.dataClients).toEqual(['SandboxDataClientFactory']);
    expect(capability.holds).toBe(false);
  });

  test('reports the capability allowlist as unbuilt rather than checking it', () => {
    // The design asks for an allowlist and no configuration module exists, so nothing is claimed.
    const capability = capacities();
    expect(capability.allowlist.built).toBe(false);
    expect(capability.allowlist.why).toContain('procedural today');
  });
});

describe('execution authority by role', () => {
  test('gives the strategy worker the sandbox venue and the other roles nothing', () => {
    // The separation the design states: only the worker is exposed to a venue, sandbox or otherwise.
    expect(authorityOf('strategy-worker')).toEqual({
      role: 'strategy-worker',
      execution: 'sandbox',
      dataClient: false,
      credential: false,
      note: 'Only the sandbox venue is exposed to a strategy worker.',
    });
    for (const role of ['orchestrator', 'result-registration']) {
      expect(authorityOf(role).execution).toBe(null);
    }
  });

  test('refuses a role the boundary does not name', () => {
    // An unknown role is a wiring mistake, not a condition to render: it has no capability to report.
    expect(() => authorityOf('broker')).toThrow('unknown role: broker');
  });
});

describe('the negative submission test', () => {
  // The fixture's own field sets: mutating the legal candidate is what makes `venue` undeclared.
  const declared = valueOf(LEGAL_BLOCKS.declared);
  const legal = {
    data: { signal_frequency: 'daily', decision_time: 'session_close' },
    universe: { asset_classes: ['EQUITY'], min_price: '5' },
  };

  test('refuses data.venue as a field the block does not declare', () => {
    // The first refusal pair the fixture asserts, message included so the reason is not re-worded.
    const refusal = refuseSubmission(
      { ...legal, data: { ...legal.data, venue: 'BINANCE' } },
      declared,
    );
    expect(refusal.ok).toBe(false);
    expect(refusal.code).toBe('block_unknown_field');
    expect(refusal.path).toBe('data.venue');
    expect(refusal.reason).toBe('the data block does not declare venue');
  });

  test('refuses universe.module_path as a code reference', () => {
    // The second pair: a field name that asks for code is refused by name, not by value.
    const refusal = refuseSubmission(
      { ...legal, universe: { ...legal.universe, module_path: 'nautilus_trader.adapters.binance' } },
      declared,
    );
    expect(refusal.code).toBe('code_reference_refused');
    expect(refusal.path).toBe('universe.module_path');
    expect(refusal.reason).toBe('a specification is data and may not declare module_path');
  });

  test('runs the code scan before the block readers', () => {
    // The compiler's fixed order: an undeclared field in the first block loses to a code field later.
    const refusal = refuseSubmission(
      {
        ...legal,
        data: { ...legal.data, venue: 'BINANCE' },
        universe: { ...legal.universe, module_path: 'venue.module' },
      },
      declared,
    );
    expect(refusal.code).toBe('code_reference_refused');
    expect(refusal.path).toBe('universe.module_path');
  });

  test('refuses a value shaped like an import, quoting the value', () => {
    // A value can carry code wherever it is put, including in a place the schema declares.
    const imported = refuseSubmission(
      { ...legal, data: { ...legal.data, signal_frequency: '__import__("os")' } },
      declared,
    );
    expect(imported.code).toBe('code_reference_refused');
    expect(imported.path).toBe('data.signal_frequency');
    expect(imported.reason).toBe(
      "a specification value may not be executable code, '__import__(\"os\")' is",
    );

    const colon = refuseSubmission(
      { ...legal, universe: { ...legal.universe, source: 'pkg.mod:Attribute' } },
      declared,
    );
    expect(colon.code).toBe('code_reference_refused');
    expect(colon.path).toBe('universe.source');
  });

  test('refuses a value that is not plain data, by its type', () => {
    // A callable is the value the schema's non-plain branch exists for.
    const refusal = refuseSubmission(
      { ...legal, data: { ...legal.data, decision_time: () => '1d' } },
      declared,
    );
    expect(refusal.code).toBe('code_reference_refused');
    expect(refusal.reason).toBe('a specification is data and may not carry a function');
  });

  test('accepts a candidate whose fields are all declared data', () => {
    // The legal case: what is refused is the field names, not the shape of the candidate.
    const accepted = refuseSubmission(legal, declared);
    expect(accepted.ok).toBe(valueOf(REFUSALS.legal));
    expect(accepted.code).toBe(null);
  });

  test('reads the two refusals back from the record in recording order', () => {
    // The audit's reading: a refusal is never a proposal attempt and never an executed experiment.
    const venue = refuseSubmission({ ...legal, data: { venue: 'BINANCE' } }, declared);
    const code = refuseSubmission(
      { ...legal, universe: { module_path: 'venue.module' } },
      declared,
    );

    // Each set holds exactly the one pair the fixture asserts; the order is the recording order.
    expect(refusalPairs([venue])).toEqual(valueOf(REFUSALS.venue));
    expect(refusalPairs([code])).toEqual(valueOf(REFUSALS.modulePath));
    expect(refusalPairs([venue, code])).toEqual([
      ...valueOf(REFUSALS.venue),
      ...valueOf(REFUSALS.modulePath),
    ]);

    expect(recordRefusals(refusalPairs([venue, code]))).toEqual({
      candidate: 'sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
      outcome: 'refused',
      attempts: [],
      refusals: ['block_unknown_field', 'code_reference_refused'],
      counts: { proposal_attempts: 0, executed_experiments: 0 },
    });
  });

  test('knows the eighteen field names that ask for code', () => {
    // A field the list does not name is a field the scan will not refuse by name.
    expect(valueOf(CODE_FIELD_NAMES)).toBeArrayOfSize(18);
    expect(valueOf(CODE_FIELD_NAMES)).toContain('module_path');
    expect(valueOf(CODE_FIELD_NAMES)).toContain('strategy_path');
  });
});

describe('determinism', () => {
  test('returns identical values for identical inputs', () => {
    // The boundary is a reading of facts, so two calls may not differ in any field or ordering.
    expect(executedSurface()).toEqual(executedSurface());
    expect(capacities()).toEqual(capacities());
    expect(checkEnvironment({ EODHD_API_KEY: 'set' }, NAMES)).toEqual(
      checkEnvironment({ EODHD_API_KEY: 'set' }, NAMES),
    );
    expect(refuseSubmission({ data: { venue: 'BINANCE' } }, { data: ['timeframe'] })).toEqual(
      refuseSubmission({ data: { venue: 'BINANCE' } }, { data: ['timeframe'] }),
    );
  });
});
