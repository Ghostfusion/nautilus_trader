/**
 * The campaign view: one campaign, end to end.
 *
 * This is the only view that is not a layer page. It drives the composed pipeline
 * (`engine/campaign.js`) over the declared inputs below and shows what each layer decided, in order,
 * with the numbers the layers produced. It is where the honest limits of the console are most
 * visible: the campaign is one deterministic path, its development results are declared rather than
 * measured, its feed is declared rather than live, and when the gate ladder stops the stages that
 * depend on a judged candidate are reported as not reached.
 */

import { DEFECTS, DEFAULT_OPTIONS } from '../../engine/campaign.js';

export const meta = {
  id: 'campaign',
  title: 'One campaign, end to end',
  order: 13,
  tutorial: 'tutorials/strategy-discovery/README.md',
};

const TONES = { ok: 'ok', warn: 'warn', bad: 'bad', muted: 'muted' };

export function mount(root, ctx) {
  const { ui, data, store } = ctx;
  const campaign = store.getCampaign();
  const options = store.getOptions();

  root.appendChild(
    ui.panel(
      { title: 'What this is' },
      ui.h(
        'p',
        null,
        'Every other view shows one layer. This one runs the whole thing: it freezes a policy, reads ',
        'the declared candidate as data, opens the ledger, allocates the ranking, folds the four ',
        'counts, takes the campaign\u2019s one confirmation reservation, walks the gate ladder, ',
        'assesses what the sample can support, breeds children from the candidate, classifies a ',
        'recorded proposal, watches the frozen candidate, reads the ledger back and checks the ',
        'boundary. Each stage calls that layer\u2019s own engine.',
      ),
      ui.h(
        'p',
        null,
        'It is one deterministic path through the rules, not a measurement of anything. The declared ',
        'evidence the gates read is the gate page\u2019s worked candidate with this campaign\u2019s own ',
        'candidate identity in it, and the development results the search reads are declared by the ',
        'campaign rather than computed. Change an input below and the whole trace is recomputed.',
      ),
    ),
  );

  const controls = ui.panel(
    { title: 'The declared inputs' },
    ui.grid(
      2,
      ui.field(
        'campaign label',
        ui.textInput({
          id: 'campaign-label',
          value: options.label,
          onInput: (value) => store.setOptions({ label: value || DEFAULT_OPTIONS.label }),
        }),
        'the name the audit reading is made under',
      ),
      ui.field(
        'seed',
        ui.numberInput({
          id: 'campaign-seed',
          value: options.seed,
          step: 1,
          onInput: (value) => store.setOptions({ seed: Math.round(value) }),
        }),
        'the seed and the campaign label together fix every draw the search makes',
      ),
      ui.field(
        'planted defect',
        ui.selectInput({
          id: 'campaign-defect',
          value: options.defect,
          options: Object.entries(DEFECTS).map(([value, defect]) => ({ value, label: defect.label })),
          onChange: (value) => store.setOptions({ defect: value }),
        }),
        'the one condition the gate ladder is meant to catch',
      ),
      ui.field(
        'planned configurations',
        ui.numberInput({
          id: 'campaign-configurations',
          value: options.plannedConfigurations,
          min: 1,
          step: 1,
          onInput: (value) => store.setOptions({ plannedConfigurations: Math.max(1, Math.round(value)) }),
        }),
        'the size of search the admission test is asked about',
      ),
      ui.field(
        'target Sharpe',
        ui.numberInput({
          id: 'campaign-target',
          value: options.targetSharpe,
          step: 0.01,
          onInput: (value) => store.setOptions({ targetSharpe: value }),
        }),
        'the effect the sample would have to distinguish from chance',
      ),
      ui.field(
        'holdout fraction',
        ui.numberInput({
          id: 'campaign-holdout',
          value: options.holdoutFraction,
          min: 0,
          max: 0.9,
          step: 0.05,
          onInput: (value) => store.setOptions({ holdoutFraction: value }),
        }),
        'the share of observations the ceiling may not count',
      ),
      ui.field(
        'position cap',
        ui.numberInput({
          id: 'campaign-cap',
          value: options.maxPositionWeight,
          min: 0.01,
          max: 1,
          step: 0.01,
          onInput: (value) => store.setOptions({ maxPositionWeight: value }),
        }),
        'the per-position cap the executor clamps to',
      ),
      ui.field(
        'declared Sharpe',
        ui.numberInput({
          id: 'campaign-sharpe',
          value: options.declaredSharpe,
          step: 0.01,
          onInput: (value) => store.setOptions({ declaredSharpe: value }),
        }),
        'the candidate\u2019s declared Sharpe, which the correction is computed from',
      ),
    ),
    ui.row(
      ui.button('restore the defaults', () => store.reset(), { kind: 'primary' }),
      ui.note('Every change re-runs the whole campaign, so the trace below is never stale.'),
    ),
  );
  root.appendChild(controls);

  const ran = campaign.stages.filter((stage) => stage.ran).length;
  const reachedLater = campaign.stages.filter((stage) => !stage.ran).map((stage) => stage.id);
  // A ladder that stopped is not the same as a stage that refused: the campaign carried on, and what
  // stops is the reading. The verdict names which of the two happened rather than leaving a reader to
  // reconcile "no stage refused" with "11 of 13".
  const ladderStopped = campaign.stoppedAt !== null;
  const verdict = !campaign.ok
    ? { value: 'a stage refused', note: 'see the refusals below', tone: 'bad' }
    : ladderStopped
      ? {
          value: `the ladder stopped at ${campaign.stoppedAt}`,
          note: 'no stage refused, but the reading stopped and the stages that depend on a judged candidate did not run',
          tone: 'warn',
        }
      : { value: 'every stage ran', note: 'nothing refused and no gate stopped the reading', tone: 'ok' };

  root.appendChild(
    ui.panel(
      { title: 'The campaign', tone: campaign.ok && !ladderStopped ? 'ok' : 'warn' },
      ui.metrics(
        ui.metric('verdict', verdict.value, verdict.note, verdict.tone),
        ui.metric('stages run', `${ran} of ${campaign.stages.length}`, reachedLater.length > 0 ? `not reached: ${reachedLater.join(', ')}` : 'every stage ran', reachedLater.length > 0 ? 'warn' : 'ok'),
        ui.metric('gate ladder', campaign.stoppedAt === null ? 'no gate stopped it' : `stopped at ${campaign.stoppedAt}`, campaign.stoppedAt === null ? 'all six gates passed' : 'the reading stops at the first no', campaign.stoppedAt === null ? 'ok' : 'warn'),
        ui.metric('ledger', `${campaign.ledger.entries} entries`, campaign.ledger.verified ? 'the chain verifies' : 'the chain does not verify', campaign.ledger.verified ? 'ok' : 'bad'),
      ),
      ui.table({
        columns: ['Step', 'Layer', 'What it decided', 'Identity', 'Detail'],
        rows: campaign.stages.map((stage) => [
          stage.step,
          stage.title,
          ui.h('span', null, ui.badge(stage.ran ? stage.verdict : 'not reached', TONES[stage.tone] ?? 'muted')),
          stage.digest ? ui.digest(stage.digest) : '',
          ui.h('span', { class: 'note', text: stage.detail }),
        ]),
        dense: true,
      }),
    ),
  );

  const withFacts = campaign.stages.filter((stage) => stage.facts.length > 0);
  if (withFacts.length > 0) {
    root.appendChild(
      ui.panel(
        { title: 'The numbers behind each stage' },
        ...withFacts.map((stage) =>
          ui.block(
            `${stage.step}. ${stage.title}`,
            ui.table({
              columns: ['what', 'value'],
              rows: stage.facts.map(([label, value]) => [label, String(value)]),
              dense: true,
              caption: stage.ran ? stage.verdict : 'not reached',
            }),
          ),
        ),
      ),
    );
  }

  const withRows = campaign.stages.filter((stage) => Array.isArray(stage.rows) && stage.rows.length > 0);
  if (withRows.length > 0) {
    root.appendChild(
      ui.panel(
        { title: 'What each stage actually produced' },
        ...withRows.map((stage) =>
          ui.block(
            `${stage.step}. ${stage.title}`,
            ui.h('div', { class: 'scroll-x' }, ui.table({ columns: ROW_COLUMNS[stage.id], rows: stage.rows, dense: true })),
          ),
        ),
      ),
    );
  }

  const stageRefusals = campaign.stages.filter((stage) => stage.refusals.length > 0);
  root.appendChild(
    ui.panel(
      { title: 'What the campaign refused, and where' },
      stageRefusals.length === 0
        ? ui.note('No stage refused anything under these inputs.')
        : ui.stack(
            ...stageRefusals.map((stage) =>
              ui.block(
                `${stage.step}. ${stage.title}`,
                ui.table({
                  columns: ['code', 'the condition the layer states'],
                  rows: stage.refusals.map((item) => [ui.h('code', { text: item.code }), item.condition]),
                  dense: true,
                }),
              ),
            ),
          ),
      ui.note(
        'A refusal here is the layer working. The second holdout reservation and the ambient credential are refused on purpose, to show the rule rather than to report a fault.',
        'muted',
      ),
    ),
  );

  root.appendChild(
    ui.panel(
      { title: 'The reading, and what it will not assert' },
      ui.table({
        columns: ['candidate', 'what the record shows'],
        rows: campaign.audit.readOutcomes.map((item) => [item.candidateId || 'the campaign', item.outcome]),
        dense: true,
      }),
      ui.note(
        `Read under ${campaign.digests.policy.slice(0, 19)}... over ${campaign.ledger.entries} entries, whose chain verifies against its own head.`,
      ),
      ui.note(
        'The audit deliberately does not claim the seed, build and configuration join: two of the twenty-two acceptance criteria need that identity and it does not exist at the engine boundary yet.',
        'warn',
      ),
    ),
  );

  const failing = data.failingCriteria();
  root.appendChild(
    ui.panel(
      { title: 'What this campaign does not claim' },
      ui.list([
        'It is one path, not a distribution. Nothing here is an estimate of anything.',
        `The development results the search reads are declared by the campaign. The four acceptance criteria that do not pass (${failing.map((row) => row.id).join(', ')}) all bear on parts this console cannot exercise: the engine-side run manifest, the settlement bridge, the corporate-action entitlement stage and the engine digest recomputation.`,
        'The shadow feed is declared rather than bound to a live adapter, which the plan records as a platform decision rather than a property of the code.',
        'The digests here are SHA-256 over a canonical serialisation. The engine\u2019s canonical digest is blake3, so no digest on this page can be compared with one recorded in the repository.',
        'The policy file loader exists and refuses a credential placed in a policy file, and the command front end reports that refusal by code. What no module does is provision the worker environment the capability allowlist describes, so that shape is still procedural.',
      ]),
    ),
  );

  root.appendChild(
    ui.panel(
      { title: 'Where this came from' },
      ui.sources([
        { label: 'the design', path: 'docs/design/strategy_discovery_design.md' },
        { label: 'the implementation plan', path: 'docs/design/strategy_discovery_implementation.md' },
        { label: 'the tutorial group', path: 'tutorials/strategy-discovery/README.md' },
        { label: 'the package facade', path: 'python/nautilus_trader/discovery/__init__.py' },
        { label: 'the acceptance record', path: 'python/tests/acceptance/discovery_acceptance_bundle.json' },
      ]),
      ui.note('The run identity this view ends on is the campaign\u2019s own; no stage of it is the repository\u2019s engine.', 'muted'),
    ),
  );
}

/** The column headings for the rows a stage produced, which only the view knows how to name. */
const ROW_COLUMNS = {
  executor: ['instrument', 'rank', 'raw weight', 'capped weight', 'instruction strength'],
  gates: ['gate', 'verdict'],
  search: ['child', 'the gene the operator moved', 'return', 'turnover', 'drawdown', 'front'],
  agents: ['recorded response', 'how it was read', 'recorded outcome'],
  audit: ['candidate', 'what the record shows'],
};
