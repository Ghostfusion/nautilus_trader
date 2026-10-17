/**
 * Tests for the composed campaign. What matters here is not that the stages run but that the
 * composition tells the truth: the ladder stops where the layer stops, the stages that depend on a
 * judged candidate are reported as not reached rather than run, the arithmetic matches the pages
 * that state it, and the same seed gives the same campaign twice.
 */

import { describe, expect, test } from 'bun:test';

import { DEFAULT_OPTIONS, DEFECTS, runCampaign } from '../src/engine/campaign.js';
import { credentials } from '../src/data/layers.js';

const clean = runCampaign();
const halted = runCampaign({ defect: 'g2' });

const stageOf = (campaign, id) => campaign.stages.find((stage) => stage.id === id);

describe('the shape of the trace', () => {
  test('every layer is a stage, in pipeline order', () => {
    // If this breaks, a layer is missing from the end-to-end view and the console no longer shows the
    // whole campaign.
    expect(clean.stages.map((stage) => stage.id)).toEqual([
      'policy',
      'candidate',
      'record',
      'executor',
      'counts',
      'holdout',
      'gates',
      'admission',
      'search',
      'agents',
      'shadow',
      'audit',
      'boundary',
    ]);
    expect(clean.stages.map((stage) => stage.step)).toEqual(
      clean.stages.map((_, index) => index + 1),
    );
  });

  test('a clean candidate leaves the campaign without a refusal that stopped anything', () => {
    // If this breaks, the default campaign refuses somewhere, and every reader sees a broken trace.
    expect(clean.refusals).toEqual([]);
    expect(clean.ok).toBe(true);
    expect(clean.stoppedAt).toBe(null);
  });
});

describe('the composed numbers', () => {
  test('the gate ladder passes every gate on the declared evidence', () => {
    // If this breaks, the campaign's declared evidence no longer clears the pre-registered chain.
    const gates = stageOf(clean, 'gates');
    expect(gates.verdict).toBe('passed');
    expect(gates.rows).toHaveLength(6);
    expect(gates.rows.every((row) => row[1] === 'passed')).toBe(true);
  });

  test('the admission test narrows the plan exactly as the page states', () => {
    // If this breaks, the pre-campaign arithmetic no longer reproduces usable 240, counted 45, a
    // ceiling of 248 and a narrowing to 36.
    const admission = stageOf(clean, 'admission');
    const facts = Object.fromEntries(admission.facts);
    expect(admission.verdict).toBe('reduce');
    expect(facts['usable observations']).toBe('240');
    expect(facts['counted trials']).toBe('45');
    expect(facts['ceiling at that count']).toBe('248');
    expect(facts['narrowed to']).toBe('36');
  });

  test('the four counts come from one fold over the campaign events', () => {
    // If this breaks, a proposal, a configuration, an execution or a trial is being counted twice.
    const facts = Object.fromEntries(stageOf(clean, 'counts').facts);
    expect(facts.proposals).toBe('2');
    expect(facts.configurations).toBe('1');
    expect(facts.executions).toBe('1');
    expect(facts.trials).toBe('1');
  });

  test('the ranking is capped and the remainder is left idle', () => {
    // If this breaks, the executor's cap or its idle policy has changed under the campaign.
    const executor = stageOf(clean, 'executor');
    expect(executor.rows.map((row) => row[3])).toEqual([0.25, 0.25, 0.2, 0.1]);
  });

  test('the campaign takes one reservation and the second ask is refused', () => {
    // If this breaks, a campaign can hold two confirmation reservations, which the layer forbids.
    const holdout = stageOf(clean, 'holdout');
    expect(holdout.verdict).toBe('reserved once');
    expect(holdout.refusals.map((item) => item.code)).toEqual(['reservation_consumed']);
  });
});

describe('where the campaign stops', () => {
  test('a defect at a middle gate stops the ladder and the earlier gates stay asked', () => {
    // If this breaks, a failed gate no longer stops the walk, and a later gate could excuse it.
    const gates = stageOf(halted, 'gates');
    expect(halted.stoppedAt).toBe('G2');
    expect(gates.verdict).toBe('inconclusive');
    expect(gates.rows.map((row) => row[1])).toEqual([
      'passed',
      'passed',
      'inconclusive',
      'never asked',
      'never asked',
      'never asked',
    ]);
  });

  test('the stages that depend on a judged candidate are reported as not reached', () => {
    // If this breaks, a candidate the gates turned away is still promoted to a shadow period.
    expect(stageOf(halted, 'search').ran).toBe(false);
    expect(stageOf(halted, 'shadow').ran).toBe(false);
    expect(stageOf(halted, 'shadow').verdict).toBe('not reached');
    expect(stageOf(halted, 'shadow').detail).toContain('G2');
  });

  test('the stages that fed the ladder still happened', () => {
    // If this breaks, stopping the ladder erases the record of what was already done.
    for (const id of ['policy', 'candidate', 'record', 'executor', 'counts', 'holdout', 'admission']) {
      expect(stageOf(halted, id).ran).toBe(true);
    }
  });

  test('every plantable defect is one the campaign can run', () => {
    // If this breaks, a defect in the list breaks no gate and the control demonstrates nothing.
    for (const key of Object.keys(DEFECTS)) {
      const campaign = runCampaign({ defect: key });
      const gates = stageOf(campaign, 'gates');
      if (key === 'none') expect(gates.verdict).toBe('passed');
      else expect(gates.verdict === 'passed').toBe(false);
    }
  });
});

describe('the record and the reading', () => {
  test('the ledger verifies and the gate verdict is one of its entries', () => {
    // If this breaks, the campaign's own record no longer holds what the gates decided.
    expect(clean.ledger.verified).toBe(true);
    expect(clean.ledger.entries).toBe(4);
    expect(stageOf(clean, 'gates').facts).toContainEqual(['verdict recorded', 'entry 4 of the ledger']);
  });

  test('the audit reads the campaign back without being told what it did', () => {
    // If this breaks, the audit is reading something other than the chain.
    const audit = stageOf(clean, 'audit');
    expect(audit.ran).toBe(true);
    expect(clean.audit.readOutcomes.length).toBeGreaterThan(0);
    expect(audit.facts).toContainEqual(['policy compared', 'the declared digest is the recorded one']);
  });

  test('the boundary reports the credential list and a planted credential', () => {
    // If this breaks, the environment assertion stops reporting what it found.
    const boundary = stageOf(clean, 'boundary');
    expect(boundary.facts).toContainEqual([
      'credential names in the canonical list',
      String(credentials().length),
    ]);
    expect(boundary.refusals.map((item) => item.code)).toEqual(['ambient_credential']);
  });
});

describe('determinism', () => {
  test('the same options give the same campaign twice', () => {
    // If this breaks, a campaign cannot be replayed from its declared inputs, which is the whole
    // point of a seeded search.
    const first = runCampaign({ seed: 7 });
    const second = runCampaign({ seed: 7 });
    expect(first.digests).toEqual(second.digests);
    expect(stageOf(first, 'search').digest).toBe(stageOf(second, 'search').digest);
    expect(stageOf(first, 'search').rows).toEqual(stageOf(second, 'search').rows);
  });

  test('a different seed breeds different children', () => {
    // If this breaks, the seed does not reach the operators and the search is not reproducible per
    // seed.
    const other = runCampaign({ seed: 8 });
    expect(stageOf(other, 'search').digest).not.toBe(stageOf(clean, 'search').digest);
  });

  test('the defaults are frozen, so the view cannot drift from the engine', () => {
    expect(Object.isFrozen(DEFAULT_OPTIONS)).toBe(true);
    expect(DEFAULT_OPTIONS.defect).toBe('none');
  });
});
