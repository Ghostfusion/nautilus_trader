/**
 * The agent view: the five roles, one brief, and what the campaign does with an answer.
 *
 * Models `python/nautilus_trader/discovery/agents.py` and `.../prompts/__init__.py`. The view renders
 * the strategy engineer's brief from the page's own worked example, sends the four recorded
 * responses through the classifier the code uses, and replays a recorded answer. There is no model
 * call here and no network: a live call cannot be reproduced, so the only thing that can be shown
 * twice is the recorded response.
 */

import {
  CONTAMINATION,
  OUTCOMES,
  PROMPT_FIELDS,
  REFUSING_OUTCOMES,
  ROLES,
  classifyReply,
  contamination,
  isRefusal,
  promptDigest,
  refusals,
  renderPrompt,
  replay,
} from '../../engine/agents.js';
import { brief, context, recordedReplies, validatorRefusal } from '../../data/agents.js';

/** What this view models: the five roles, one brief, and what an answer becomes. */
export const meta = {
  id: 'agents',
  title: 'Agents that propose candidates',
  order: 9,
  tutorial: 'tutorials/strategy-discovery/agents-that-propose/README.md',
};

/** Grade a layer's own paragraph into the tone the verdict is shown with. */
function gradeTone(grade) {
  if (grade.startsWith('Strong')) return 'ok';
  if (grade.startsWith('Mixed')) return 'warn';
  return 'bad';
}

/**
 * The language's validator, stood in for here.
 *
 * In the repository a candidate proposal is read by the candidate layer's own validator, which the
 * policy and the feature registry travel with. This view does not own that module, so it stands in a
 * validator that applies the one rule the page's worked example needs - the family has to be one the
 * policy declares - and reports it in the schema's own words. Nothing else about a candidate is
 * checked here, and that is a limit of the demonstration rather than a claim about the validator.
 */
function demoLanguage() {
  const admitted = ['absolute_threshold', 'cross_sectional_rank', 'static_hold'];
  const refusal = validatorRefusal();
  return {
    validate(payload) {
      const family = payload?.identity?.family;
      if (admitted.includes(family)) return { accepted: true, errors: [] };
      return {
        accepted: false,
        errors: [
          {
            code: refusal.code,
            path: 'identity.family',
            message: refusal.message
              .replace('{name}', 'family')
              .replace('{admitted}', admitted.join(', '))
              .replace('{value}', String(family)),
          },
        ],
      };
    },
  };
}

/** Build the view into the empty root, and wire every control the model needs. */
export function mount(root, ctx) {
  const { ui, data } = ctx;
  const layer = data.layer(meta.id);
  const language = demoLanguage();
  const rows = ROLES();
  const reply = recordedReplies();
  const example = context();

  const state = {
    leaked: false,
    cutoff: example.knowledgeCutoff,
    periodStart: example.periodStart,
    demonstration: null,
    focus: null,
  };

  const briefValues = () =>
    state.leaked ? { ...brief(), results: 'the campaign\u2019s results table' } : brief();

  const read = (response) => classifyReply(response, { role: 'strategy_engineer', language });

  const demonstration = ui.h('div', null);

  /** Show what the last demonstration's call returned, or raised. */
  const showDemonstration = () => {
    ui.clear(demonstration);
    const shown = state.demonstration;
    if (!shown) {
      demonstration.appendChild(ui.note('Run a demonstration to see the refusal it reports.'));
      return;
    }
    if (shown.raised !== null) {
      demonstration.appendChild(
        ui.verdict(shown.label, 'raised rather than refused', 'bad', shown.raised),
      );
      return;
    }
    demonstration.appendChild(
      ui.verdict(
        shown.label,
        shown.value.ok === false ? shown.value.code : (shown.value.outcome ?? 'recorded'),
        shown.value.code === 'accepted' || shown.value.outcome === 'accepted' ? 'ok' : 'warn',
        shown.value.reason ?? '',
      ),
    );
  };

  /** Run one demonstration and render what the layer returned or raised. */
  const run = (label, action) => {
    try {
      state.demonstration = { label, value: action(), raised: null };
    } catch (error) {
      state.demonstration = { label, value: null, raised: String(error.message) };
    }
    showDemonstration();
  };

  // --- block two: the model ------------------------------------------------

  const results = ui.h('div', null);

  const leakedToggle = ui.toggle({
    id: 'agents-leaked',
    checked: state.leaked,
    label: 'leak an undeclared field into the brief',
    onChange: (checked) => {
      state.leaked = checked;
      render();
    },
  });

  const cutoffInput = ui.textInput({
    id: 'agents-cutoff',
    value: state.cutoff,
    placeholder: '2023-06-01',
    onInput: (value, event) => {
      state.cutoff = value;
      state.focus = event?.target?.id ?? null;
      render();
    },
  });
  const periodInput = ui.textInput({
    id: 'agents-period',
    value: state.periodStart,
    placeholder: '2024-03-01',
    onInput: (value, event) => {
      state.periodStart = value;
      state.focus = event?.target?.id ?? null;
      render();
    },
  });

  const rolesTable = () =>
    ui.table({
      columns: ['role', 'fields it is given', 'produces', 'may not'],
      rows: rows.map((role) => [
        role.id,
        role.fields.join(', '),
        role.produces,
        role.mayNot,
      ]),
      dense: true,
      caption:
        'The field set is the whole of what a role is handed, which is why the authority column is described by what the prompt never receives.',
    });

  const briefBlock = () => {
    const rendered = renderPrompt('strategy_engineer', briefValues());
    const values = Object.entries(brief()).map(([name, value]) => [name, value]);
    return ui.stack(
      ui.note(
        'One brief is sent to the strategy engineer four times in the page\u2019s worked example, so its input digest is the same for all four and a replay keys on it.',
      ),
      ui.table({
        columns: ['field', 'value'],
        rows: state.leaked ? [...values, ['results', 'the campaign\u2019s results table']] : values,
        dense: true,
      }),
      rendered.ok
        ? ui.frag(
            ui.metrics(
              ui.metric('prompt digest', rendered.promptDigest.slice(0, 19), 'the prompt\u2019s identity: id, version, fields, template'),
              ui.metric('input digest', rendered.inputDigest.slice(0, 19), 'the role, the prompt identity and the instructions'),
            ),
            ui.group('the rendered brief', ui.quote(rendered.instructions, 'the strategy engineer\u2019s template, rendered')),
          )
        : ui.verdict('the brief refused', rendered.code, 'bad', rendered.reason),
      ui.note(
        'Editing the template to accept one more field would change the role\u2019s authority, not just its wording, which is why the render refusal is load-bearing.',
        'muted',
      ),
    );
  };

  const outcomesTable = () => {
    const readings = reply.map((row) => ({ row, result: read(row.response) }));
    return ui.stack(
      ui.metrics(
        ui.metric('outcomes', OUTCOMES.length, 'in the order the code tests them'),
        ui.metric('refusals', REFUSING_OUTCOMES.length, REFUSING_OUTCOMES.join(', ')),
        ui.metric(
          'the page\u2019s four rows',
          readings.every((item) => item.result.outcome === item.row.outcome) ? 'read back as recorded' : 'differ',
          'each response sent through the classifier',
          readings.every((item) => item.result.outcome === item.row.outcome) ? 'ok' : 'bad',
        ),
      ),
      ui.table({
        columns: ['the response sent', 'how the page read it', 'recorded outcome', 'read again here', 'the reason'],
        rows: readings.map(({ row, result }) => [
          row.response,
          row.howRead,
          row.outcome,
          isRefusal(result.outcome) ? ui.badge(result.outcome, 'bad') : ui.badge(result.outcome, 'ok'),
          result.reason === '' ? '' : result.reason,
        ]),
        dense: true,
        caption:
          'The page shows two of the four responses with the specification elided as {...}; the console sends the smallest object with the property the page states, and shows what it sent.',
      }),
      ui.note(
        'Four of the five roles are checked only for producing a JSON object: their limit is the field set they are given, not the content of what they write.',
        'muted',
      ),
    );
  };

  const replayBlock = () => {
    const recorded = read(reply[3].response);
    const request = renderPrompt('strategy_engineer', briefValues());
    return ui.stack(
      ui.note(
        'A live call cannot be reproduced: sampling, version and provider all move, so the recorded response is the only reproducible object and the invocation record is the evidence.',
        'muted',
      ),
      ui.row(
        ui.button(
          'replay the accepted response',
          () =>
            run('replay', () =>
              replay(
                { inputDigest: request.inputDigest, role: 'strategy_engineer', response: reply[3].response },
                { language },
              ),
            ),
          { kind: 'primary' },
        ),
        ui.button('replay an invocation that was never recorded', () =>
          run('replay', () =>
            replay({ inputDigest: request.inputDigest, role: 'strategy_engineer' }, { language }),
          ),
        ),
      ),
      ui.note(
        `The recorded answer read back gives ${recorded.outcome}, and its response digest names the bytes rather than the answer.`,
      ),
    );
  };

  const contaminationBlock = () => {
    const label = contamination(state.cutoff, state.periodStart);
    return ui.stack(
      ui.row(
        ui.field('knowledge cutoff', cutoffInput, 'the model\u2019s declared cutoff, as an ISO date'),
        ui.field('period start', periodInput, 'the earliest date the result covers'),
      ),
      ui.metrics(
        ui.metric('contamination', label, `one of ${CONTAMINATION.join(', ')}`, label === 'post_cutoff' ? 'ok' : 'warn'),
        ui.metric('the record\u2019s model', example.modelIdentifier, 'the model and version the record names'),
      ),
      ui.note(
        'The label is standing rather than a fix: a vulnerable period stays vulnerable, and no gate converts the first into the second. A date that is not zero-padded and ten characters long is labelled unknown rather than compared.',
      ),
    );
  };

  // --- block three: what it refuses ---------------------------------------

  const demonstrations = () => {
    const rows = [
      {
        code: 'field_missing',
        condition: 'a declared field of the brief was not supplied',
        run: () => renderPrompt('experiment_analyst', { information_set: 'nothing' }),
      },
      {
        code: 'field_undeclared',
        condition: 'a supplied field is not one the template declares',
        run: () => renderPrompt('strategy_engineer', { ...brief(), results: 'x' }),
      },
      {
        code: 'unstructured',
        condition: 'the response is not a JSON object',
        run: () => read('The trend should continue, so buy the strongest names.'),
      },
      {
        code: 'block_missing',
        condition: 'a candidate proposal carries no specification block',
        run: () => read('{"family": "cross_sectional_rank"}'),
      },
      {
        code: 'language_missing',
        condition: 'a candidate proposal was offered without the language to validate it',
        run: () =>
          classifyReply('{"specification": {"identity": {"family": "cross_sectional_rank"}}}', {
            role: 'strategy_engineer',
          }),
      },
      {
        code: 'unrecorded_invocation',
        condition: 'a replay was asked for an invocation with nothing recorded against it',
        run: () => replay({ inputDigest: 'sha256:not-recorded', role: 'strategy_engineer' }, { language }),
      },
      {
        code: 'model_identifier_empty',
        condition: 'an invocation context names no model, which the record could not attribute',
        run: () => {
          throw new Error(refusals().emptyModelIdentifier.message);
        },
      },
    ];
    return ui.stack(
      ui.table({
        columns: ['code', 'what produces it', ''],
        rows: rows.map((row) => [
          ui.badge(row.code, 'warn'),
          row.condition,
          ui.button('demonstrate', () => run(row.code, row.run)),
        ]),
        dense: true,
      }),
      demonstration,
      ui.group(
        'the outcome vocabulary',
        ui.table({
          columns: ['outcome', 'a refusal?', 'the condition that produces it'],
          rows: OUTCOMES.map((outcome) => [
            outcome,
            REFUSING_OUTCOMES.includes(outcome) ? 'yes' : 'no',
            {
              unstructured: 'the response is not a JSON object',
              insufficient_data: 'the response declares that its information was not enough',
              accepted: 'a JSON object, and for a candidate proposal the language accepted it',
              schema_violation: 'no specification block, or the language refused the proposal',
            }[outcome],
          ]),
          dense: true,
        }),
      ),
    );
  };

  // --- the view ------------------------------------------------------------

  function render() {
    ui.clear(results);
    const rendered = renderPrompt('strategy_engineer', briefValues());
    const readings = reply.map((row) => read(row.response));
    results.appendChild(
      ui.stack(
        ui.metrics(
          ui.metric('roles', rows.length, rows.map((role) => role.id).join(', ')),
          ui.metric('candidate producers', 1, 'the other four are checked for a JSON object alone'),
          ui.metric('prompt', `${rendered.promptId ?? 'strategy_engineer'} v1`, rendered.ok ? `${rendered.promptDigest.slice(0, 12)}…` : 'refused'),
          ui.metric('the brief', state.leaked ? 'refused' : 'rendered', state.leaked ? 'an undeclared field was supplied' : `${PROMPT_FIELDS().strategy_engineer.length} declared fields`, state.leaked ? 'bad' : 'ok'),
          ui.metric('the four rows', readings.filter((item, index) => item.outcome === reply[index].outcome).length, 'of four read back as the page records them'),
        ),
        ui.group('the five roles', rolesTable()),
        ui.group('one brief', leakedToggle, briefBlock()),
        ui.group('what came back', outcomesTable()),
        ui.group('the replay', replayBlock()),
        ui.group('the contamination label', contaminationBlock()),
      ),
    );
    if (state.focus) {
      const field = root.querySelector(`#${state.focus}`);
      if (field) field.focus();
    }
  }

  root.appendChild(
    ui.panel(
      { title: layer.title, subtitle: layer.tutorial, id: 'agents' },
      ui.block(
        'The idea',
        ui.quote(layer.idea, layer.tutorial),
        ui.verdict('grade', layer.grade.split(':')[0], gradeTone(layer.grade), layer.grade),
      ),
      ui.block('The model', results),
      ui.block('What it refuses', demonstrations()),
      ui.block('Where it goes wrong', ui.list(layer.caveats)),
      ui.block(
        'Where this came from',
        ui.sources(layer.sources),
        ui.sources([
          { label: 'the roles, the prompts and the words', path: 'src/data/agents.js' },
          { label: 'the classifier and the replay', path: 'src/engine/agents.js' },
        ]),
        ui.note(
          `The prompt digest is taken over the template as written, so editing one sentence of a role\u2019s brief changes every invocation digest below it. ${promptDigest(rows[1]).slice(0, 19)}… is the strategy engineer\u2019s.`,
          'muted',
        ),
      ),
    ),
  );
  showDemonstration();
  render();
}
