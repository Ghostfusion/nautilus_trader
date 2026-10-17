/**
 * The agent layer's behaviour: the roles, the prompt identity, and what a response becomes.
 *
 * If one of these broke, a role would be handed a field its prompt does not declare, a response
 * would be read in the wrong order, or a replay would invent an answer no model gave.
 */

import { describe, expect, test } from 'bun:test';

import {
  CONTAMINATION,
  OUTCOMES,
  PROMPT_FIELDS,
  REFUSING_OUTCOMES,
  ROLES,
  classifyReply,
  contamination,
  inputDigest,
  isRefusal,
  promptDigest,
  refusals,
  renderPrompt,
  replay,
} from '../src/engine/agents.js';
import { brief, context, recordedReplies, roleById, roles } from '../src/data/agents.js';

/**
 * The language's own validator, stood in for here by the one rule the worked example needs: a
 * proposal is accepted when its family is one the policy declares, and refused by name otherwise.
 */
const language = {
  validate(payload) {
    const family = payload?.identity?.family;
    if (family === 'cross_sectional_rank') return { accepted: true, errors: [] };
    return {
      accepted: false,
      errors: [
        {
          code: 'value_not_declared',
          path: 'identity.family',
          message: `family must be one of absolute_threshold, cross_sectional_rank, static_hold, was ${String(family)}`,
        },
      ],
    };
  },
};

const ENGINEER = 'strategy_engineer';

describe('the five roles', () => {
  test('each role declares a prompt, a field set, a product and a denied authority', () => {
    // A role without its denied authority is half a definition, and that half is what the design
    // says does the work.
    const rows = ROLES();
    expect(rows.map((role) => role.id)).toEqual([
      'research_scientist',
      'strategy_engineer',
      'experiment_analyst',
      'adversarial_reviewer',
      'search_strategist',
    ]);
    expect(new Set(rows.map((role) => role.produces)).size).toBe(5);
    expect(new Set(rows.map((role) => role.mayNot)).size).toBe(5);
    for (const role of rows) {
      expect(role.promptId).toBe(role.id);
      expect(role.version).toBe('1');
      expect(role.fields.length).toBeGreaterThan(1);
      expect(role.fields).toContain('information_set');
      expect(role.template).toContain('{information_set}');
    }
    expect(PROMPT_FIELDS()[ENGINEER]).toEqual([
      'families',
      'features',
      'bounds',
      'declared_sets',
      'information_set',
    ]);
  });

  test('a role prompt renders its declared fields and refuses any other', () => {
    // A brief that leaked a field would defeat the separation the roles exist to keep, so the render
    // refusal is load-bearing rather than tidy.
    const rendered = renderPrompt(ENGINEER, brief());
    expect(rendered.ok).toBe(true);
    expect(rendered.instructions).not.toContain('{');
    expect(rendered.instructions).toContain(brief().bounds);
    expect(rendered.informationSet).toBe(brief().information_set);
    expect(renderPrompt(ENGINEER, { ...brief(), leaked: 'results' })).toEqual({
      ok: false,
      code: 'field_undeclared',
      reason: 'strategy_engineer 1 was given leaked',
    });
    expect(renderPrompt('experiment_analyst', { information_set: 'x' })).toEqual({
      ok: false,
      code: 'field_missing',
      reason: 'experiment_analyst 1 is missing ledger_record',
    });
    expect(() => roleById('nope')).toThrow('unknown role');
  });

  test('a prompt digest follows its template and its version, and names the brief', () => {
    // The digest is what lets an invocation record say which instructions produced an answer.
    const role = roles().find((row) => row.id === ENGINEER);
    const digest = promptDigest(role);
    expect(digest.startsWith('sha256:')).toBe(true);
    expect(promptDigest({ ...role })).toBe(digest);
    expect(promptDigest({ ...role, version: '2' })).not.toBe(digest);
    expect(promptDigest({ ...role, template: `${role.template} one more sentence` })).not.toBe(digest);
  });

  test('an invocation digest is the replay key and moves with the instructions', () => {
    // Two briefs differing by one sentence are two invocations, so a replay cannot answer for both.
    const first = renderPrompt(ENGINEER, brief());
    const shorter = renderPrompt(ENGINEER, { ...brief(), information_set: 'nothing' });
    expect(first.inputDigest).toBe(renderPrompt(ENGINEER, brief()).inputDigest);
    expect(shorter.inputDigest).not.toBe(first.inputDigest);
    expect(
      inputDigest({
        role: first.role,
        promptId: first.promptId,
        promptVersion: first.promptVersion,
        promptDigest: first.promptDigest,
        instructions: first.instructions,
      }),
    ).toBe(first.inputDigest);
  });
});

describe('reading a response', () => {
  test('the four outcomes come back in the documented order', () => {
    // The order is the contract: a response that is not an object is unstructured before anything
    // else is read, and an insufficient-data declaration is a result rather than a malformed reply.
    expect(OUTCOMES).toEqual(['accepted', 'unstructured', 'schema_violation', 'insufficient_data']);
    expect(REFUSING_OUTCOMES).toEqual(['unstructured', 'schema_violation']);
    expect(isRefusal('unstructured')).toBe(true);
    expect(isRefusal('insufficient_data')).toBe(false);

    expect(classifyReply('The trend should continue, so buy the strongest names.').outcome).toBe(
      'unstructured',
    );
    expect(classifyReply('[{"specification": {}}]').outcome).toBe('unstructured');
    expect(classifyReply('{"outcome": "insufficient_data"').outcome).toBe('unstructured');
    // The declaration is read before the role is, so an insufficient-data reply is a result for a
    // role that could not produce a candidate at all.
    expect(classifyReply('{"outcome": "insufficient_data"}').outcome).toBe('insufficient_data');
    expect(classifyReply('{"explanation": "x"}', { role: 'experiment_analyst' }).outcome).toBe(
      'accepted',
    );
    expect(classifyReply('{}', { role: ENGINEER, language })).toEqual({
      outcome: 'schema_violation',
      reason: 'a candidate proposal carries its specification under specification',
      payload: {},
      refusals: [
        {
          code: 'block_missing',
          path: 'specification',
          message: 'a candidate proposal carries its specification under specification',
        },
      ],
    });
  });

  test('an insufficient-data reply is complete and carries its own reason', () => {
    // The reason is the reply's, not the layer's, because the model is what says what is missing.
    const read = classifyReply('{"outcome": "insufficient_data", "reason": "no bound fits"}', {
      role: ENGINEER,
      language,
    });
    expect(read.outcome).toBe('insufficient_data');
    expect(read.reason).toBe('no bound fits');
    expect(read.refusals).toEqual([]);
  });

  test('the four recorded responses read as the page records them', () => {
    // The page's own worked example: one brief sent four times, one outcome each.
    const recorded = recordedReplies();
    expect(recorded.map((row) => row.outcome)).toEqual([
      'unstructured',
      'schema_violation',
      'insufficient_data',
      'accepted',
    ]);
    for (const row of recorded) {
      const read = classifyReply(row.response, { role: ENGINEER, language });
      expect(read.outcome).toBe(row.outcome);
    }
  });

  test('a proposal the language refuses carries the language own refusals', () => {
    // The layer does not know what a candidate is: the reasons belong to the validator that read it.
    const read = classifyReply('{"specification": {"identity": {"family": "invented_family"}}}', {
      role: ENGINEER,
      language,
    });
    expect(read.outcome).toBe('schema_violation');
    expect(read.refusals[0].code).toBe('value_not_declared');
    expect(read.refusals[0].path).toBe('identity.family');
    expect(read.reason).toBe(
      'family must be one of absolute_threshold, cross_sectional_rank, static_hold, was invented_family',
    );
  });

  test('a candidate proposal without a language is refused rather than accepted', () => {
    // Accepting an unvalidated proposal is the thing the contract exists to prevent.
    expect(() =>
      classifyReply('{"specification": {"identity": {"family": "cross_sectional_rank"}}}', {
        role: ENGINEER,
      }),
    ).toThrow('a candidate proposal needs the language to be validated against');
  });

  test('the layer reports the messages it refuses by, and they are the repository own words', () => {
    // A caller groups by code, so the code has to travel with the message rather than be inferred.
    expect(refusals().unstructured).toEqual({
      code: 'unstructured',
      message: 'the response is not a JSON object',
    });
    expect(refusals().unrecorded.code).toBe('unrecorded_invocation');
    expect(refusals().languageMissing.message).toBe(
      'a candidate proposal needs the language to be validated against',
    );
  });
});

describe('the replay', () => {
  test('a replay returns the recorded response and refuses an invocation never recorded', () => {
    // A replay that invented a response would not be a replay, so an unrecorded invocation is
    // refused by name rather than answered.
    const request = renderPrompt(ENGINEER, brief());
    const record = {
      inputDigest: request.inputDigest,
      role: ENGINEER,
      response: '{"specification": {"identity": {"family": "cross_sectional_rank"}}}',
    };
    const replayed = replay(record, { language });
    expect(replayed.ok).toBe(true);
    expect(replayed.outcome).toBe('accepted');
    expect(replayed.responseDigest.startsWith('sha256:')).toBe(true);
    expect(replay({ inputDigest: request.inputDigest, role: ENGINEER })).toEqual({
      ok: false,
      code: 'unrecorded_invocation',
      reason: `no recorded response for invocation ${request.inputDigest}`,
    });
  });
});

describe('the contamination label', () => {
  test('a period is labelled against the cutoff, and an unshaped date is unknown', () => {
    // The comparison is only sound because both dates are checked to be zero-padded and the same
    // length, and the label is standing rather than a fix.
    expect(CONTAMINATION).toEqual(['post_cutoff', 'vulnerable', 'unknown']);
    expect(contamination('2023-06-01', '2024-03-01')).toBe('post_cutoff');
    expect(contamination('2023-06-01', '2023-06-01')).toBe('vulnerable');
    expect(contamination('2023-6-1', '2024-03-01')).toBe('unknown');
    expect(contamination('', '2024-03-01')).toBe('unknown');
  });

  test('the worked example record is labelled post-cutoff as the page prints it', () => {
    // The record the page shows is the evidence the campaign keeps, label included.
    expect(context().contamination).toBe(
      contamination(context().knowledgeCutoff, context().periodStart),
    );
    expect(context().contamination).toBe('post_cutoff');
  });
});
