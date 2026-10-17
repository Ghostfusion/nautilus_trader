/**
 * What ran against what was written down: the fingerprint discipline and the append-only chain.
 *
 * Models `tutorials/strategy-discovery/what-ran-versus-what-was-written/README.md` and the ledger it
 * describes in `python/nautilus_trader/discovery/ledger.py`. The reader appends events to a chain,
 * sees each entry's digest and the chain value that covers the history before it, verifies the chain
 * against a checkpoint, and then tampers with an entry it already wrote: the chain value stops
 * recomputing at that entry and at every entry after it. The two acceptance criteria this layer
 * carries both pass now, and each is shown with the bundle's own record of what closed it.
 */

import {
  ENTRY_KINDS,
  LIMITS,
  REFUSAL_CODES,
  append,
  requiredFields,
  verifyChain,
} from '../../engine/ledger.js';
import {
  CHAIN_RULES,
  CRITERION_DEFECTS,
  LIMIT_FACTS,
  REFUSAL_MESSAGES,
} from '../../data/ledger.js';
import { cite, sourceOf, valueOf } from '../../data/source.js';

export const meta = {
  id: 'record',
  title: 'What ran against what was written down',
  order: 3,
  tutorial: 'tutorials/strategy-discovery/what-ran-versus-what-was-written/README.md',
};

/** The campaign the view opens with: two candidates, one of them compiled and judged. */
const SEED = [
  ['proposal_recorded', { candidateId: 'alpha', generation: 0 }],
  [
    'compilation_recorded',
    {
      candidateId: 'alpha',
      specification_digest: 'sha256:spec-alpha',
      new_configuration: true,
    },
  ],
  ['proposal_recorded', { candidateId: 'beta', generation: 1, parentId: 'alpha' }],
  [
    'gate_verdict',
    {
      candidateId: 'alpha',
      gate: 'G3',
      status: 'INCONCLUSIVE',
      reason: 'not distinguishable from the best of a search this size',
      evidence_digest: 'sha256:evidence-1',
    },
  ],
];

/** What the two limits were called on screen, keyed by the identifier the engine gives each one. */
const LIMIT_LABELS = {
  'receipt-join': 'The receipt join',
  'engine-digest': 'The engine digest',
};

/**
 * The form's starting values for one kind: every field the kind requires is present, because the
 * module refuses a payload that omits one, and `new_configuration` is the one field it reads as a
 * boolean rather than as text.
 */
function defaultFields(kind) {
  const fields = {};
  for (const name of requiredFields(kind)) fields[name] = name === 'new_configuration' ? true : '';
  return fields;
}

/** Build the opening chain through the write path, so the seed obeys the same rules as the reader. */
function seedChain() {
  let entries = [];
  for (const [kind, payload] of SEED) {
    const result = append(entries, kind, payload);
    if (result.ok) entries = result.entries;
  }
  return entries;
}

function payloadText(payload) {
  const parts = Object.entries(payload).map(([name, value]) => `${name}=${JSON.stringify(value)}`);
  return parts.length > 0 ? parts.join(' ') : '-';
}

export function mount(root, ctx) {
  const { ui, data } = ctx;
  const layer = data.layer(meta.id);
  const criteria = data.criteria().filter((row) => layer.criteria.includes(row.id));

  const state = {
    entries: seedChain(),
    kind: 'proposal_recorded',
    candidateId: 'gamma',
    key: 'k3',
    fields: defaultFields('proposal_recorded'),
    refusal: null,
    tamperAt: 1,
    tamperValue: 'tampered',
    useCheckpoint: true,
    checkpoint: '',
  };
  state.checkpoint = verifyChain(state.entries).head;

  const model = ui.h('div', { class: 'stack' });
  const refusals = ui.h('div', { class: 'stack' });

  const verification = () => verifyChain(state.entries, state.useCheckpoint ? state.checkpoint : undefined);

  function appendFromForm() {
    const payload = { candidateId: state.candidateId, key: state.key, ...state.fields };
    const result = append(state.entries, state.kind, payload);
    state.refusal = result.ok ? null : result;
    if (result.ok) state.entries = result.entries;
    render();
  }

  function demonstration(kind, payload) {
    state.refusal = append(state.entries, kind, payload);
    render();
  }

  function tamper(position) {
    state.entries = state.entries.map((item, index) => {
      if (index !== position) return item;
      const [first] = Object.keys(item.payload);
      const payload =
        first === undefined
          ? { tampered: state.tamperValue }
          : { ...item.payload, [first]: state.tamperValue };
      return { ...item, payload };
    });
    render();
  }

  function renderModel() {
    ui.clear(model);
    const check = verification();
    const head = state.entries.length > 0 ? state.entries[state.entries.length - 1] : null;
    const broken = check.links.filter((link) => !link.ok).length;

    const fieldControls = requiredFields(state.kind).map((name) =>
      ui.field(
        name,
        name === 'new_configuration'
          ? ui.selectInput({
              id: `record-field-${name}`,
              value: String(state.fields[name] ?? 'true'),
              options: ['true', 'false'],
              onChange: (value) => {
                state.fields[name] = value === 'true';
              },
            })
          : ui.textInput({
              id: `record-field-${name}`,
              value: String(state.fields[name] ?? ''),
              placeholder: name,
              onInput: (value) => {
                state.fields[name] = value;
              },
            }),
      ),
    );

    model.appendChild(
      ui.group(
        'Append one event to the chain',
        ui.grid(
          2,
          ui.field(
            'Kind',
            ui.selectInput({
              id: 'record-kind',
              value: state.kind,
              options: ENTRY_KINDS(),
              onChange: (value) => {
                state.kind = value;
                state.fields = defaultFields(value);
                render();
              },
            }),
            `${ENTRY_KINDS().length} kinds, the module's closed vocabulary`,
          ),
          ui.field(
            'Candidate',
            ui.textInput({
              id: 'record-candidate',
              value: state.candidateId,
              onInput: (value) => {
                state.candidateId = value;
              },
            }),
          ),
          ui.field(
            'Job key',
            ui.textInput({
              id: 'record-key',
              value: state.key,
              onInput: (value) => {
                state.key = value;
              },
            }),
            'the idempotency key of the job the event is about, empty when it is about none',
          ),
          ...fieldControls,
        ),
        ui.row(
          ui.button('Append', appendFromForm, { kind: 'primary' }),
          ui.button('Restore the opening chain', () => {
            state.entries = seedChain();
            state.refusal = null;
            state.checkpoint = verifyChain(state.entries).head;
            render();
          }),
        ),
        state.refusal === null
          ? null
          : ui.note(
              `${state.refusal.code}: ${state.refusal.reason}`,
              state.refusal.code === 'REWRITE_REFUSED' ? 'bad' : 'warn',
            ),
      ),
    );

    model.appendChild(
      ui.group(
        'The chain',
        ui.metrics(
          ui.metric('Entries', check.events, 'one record per event, in append order'),
          ui.metric(
            'Chain head',
            `${check.head.slice(0, 19)}...`,
            'the end of the chain, which a checkpoint holds',
          ),
          ui.metric('Links that hold', `${check.events - broken} of ${check.events}`),
          ui.metric(
            'First link that fails',
            check.brokenAt === null ? 'none' : check.brokenAt,
            'the tamper shows here and at every entry after it',
            check.brokenAt === null ? 'ok' : 'bad',
          ),
        ),
        ui.table({
          dense: true,
          columns: [
            '#',
            'kind',
            'candidate',
            'key',
            'payload',
            'digest',
            'chain',
            { label: 'link', align: 'text' },
          ],
          rows: state.entries.map((item, position) => [
            item.sequence,
            item.kind,
            item.candidateId,
            item.key === '' ? '-' : item.key,
            payloadText(item.payload),
            ui.digest(item.digest),
            ui.digest(item.chain),
            check.links[position]?.ok
              ? ui.badge('holds', 'ok')
              : ui.badge('does not recompute', 'bad'),
          ]),
        }),
        head === null
          ? ui.note('The chain is empty: the first entry chains to the genesis value.', 'muted')
          : ui.note(
              `Head ${check.head.slice(0, 19)}... · last record's digest ${head.digest.slice(0, 19)}...`,
              'muted',
            ),
        ui.note(
          'Two digests here are computed: each record\'s own digest, and the chain value that covers the history before it. The specification, behavioural and experiment fingerprints are payload values in this chain, because their pre-images - the twelve blocks and the resolved feature definitions - live in files this slice does not model.',
        ),
      ),
    );

    model.appendChild(
      ui.group(
        'Verify, and tamper with what was already written',
        ui.grid(
          2,
          ui.field(
            'Checkpoint, held outside the writer',
            ui.textInput({
              id: 'record-checkpoint',
              value: state.checkpoint,
              onInput: (value) => {
                state.checkpoint = value;
                render();
              },
            }),
            'the chain end the reader recorded separately',
          ),
          ui.toggle({
            id: 'record-use-checkpoint',
            checked: state.useCheckpoint,
            label: 'compare the chain end with the checkpoint',
            onChange: (checked) => {
              state.useCheckpoint = checked;
              render();
            },
          }),
          ui.field(
            'Tamper with entry number',
            ui.numberInput({
              id: 'record-tamper-at',
              value: state.tamperAt,
              min: 0,
              max: Math.max(0, state.entries.length - 1),
              onInput: (value) => {
                state.tamperAt = value;
              },
            }),
            'the entry is rewritten, its digest and chain value left as they were',
          ),
          ui.field(
            'its first payload field becomes',
            ui.textInput({
              id: 'record-tamper-value',
              value: state.tamperValue,
              placeholder: 'tampered',
              onInput: (value) => {
                state.tamperValue = value;
              },
            }),
          ),
        ),
        ui.row(
          ui.button('Tamper', () => tamper(Number(state.tamperAt)), { kind: 'primary' }),
          ui.button('Verify', () => render()),
        ),
        ui.verdict(
          'The chain',
          check.ok ? 'holds' : 'does not hold',
          check.ok ? 'ok' : 'bad',
          check.ok
            ? 'every sequence number, every link and the checkpoint agree with the records'
            : 'a finding is reported by name rather than raised, and the reading continues',
        ),
        check.findings.length > 0
          ? ui.list(check.findings, { className: 'reasons' })
          : ui.note('No finding: the chain recomputes end to end.', 'ok'),
      ),
    );

    model.appendChild(
      ui.group(
        'What this layer used to be unable to show, and what closed it',
        ...LIMIT_FACTS.map((limit) => {
          const documented = LIMITS.find((item) => item.id === limit.id);
          const source = sourceOf(limit.resolved);
          return ui.stack(
            ui.metric(LIMIT_LABELS[limit.id] ?? limit.id, 'closed', documented.resolved, 'ok'),
            ui.note(`the page records it: ${valueOf(limit.page)}  [${cite(limit.page)}]`),
            ui.note(`${source.path}:${source.line}`, 'muted'),
          );
        }),
      ),
    );
  }

  function renderRefusals() {
    ui.clear(refusals);
    const demonstrations = {
      REWRITE_REFUSED: () =>
        demonstration('compilation_recorded', {
          candidateId: state.entries.find((item) => item.kind === 'compilation_recorded')?.candidateId ?? 'alpha',
          specification_digest: 'sha256:spec-again',
          new_configuration: true,
        }),
      MISSING_REQUIRED_FIELD: () =>
        demonstration('gate_verdict', { candidateId: 'alpha', gate: 'G0' }),
      KIND_NOT_IN_VOCABULARY: () => demonstration('', {}),
    };

    for (const [code, rule] of Object.entries(REFUSAL_CODES)) {
      const message = valueOf(REFUSAL_MESSAGES[code]);
      refusals.appendChild(
        ui.stack(
          ui.row(ui.badge(code, 'bad'), ui.badge(cite(REFUSAL_MESSAGES[code]), 'muted')),
          ui.note(rule.condition),
          ui.note(`the module's own text: ${message}`, 'muted'),
          ui.button(`Demonstrate ${code}`, demonstrations[code]),
        ),
      );
    }
  }

  function render() {
    // A control's own input event repaints this view, so the control the reader is in is put back
    // where it was: without this the caret is lost on the first keystroke of every field.
    const active = typeof document === 'undefined' ? null : document.activeElement;
    const focus =
      active !== null && active.id !== '' && root.contains(active)
        ? { id: active.id, caret: active.selectionStart, type: active.type }
        : null;

    renderModel();
    renderRefusals();

    if (focus !== null) {
      const next = root.querySelector(`#${focus.id}`);
      if (next !== null) {
        next.focus();
        if (focus.type === 'text' && typeof next.setSelectionRange === 'function') {
          next.setSelectionRange(focus.caret, focus.caret);
        }
      }
    }
  }

  root.appendChild(
    ui.panel(
      { title: meta.title, subtitle: layer.tutorial },
      ui.block(
        'The idea',
        ui.quote(layer.idea, layer.tutorial),
        ui.metric('Grade', 'mixed', layer.grade, 'warn'),
        ...criteria.map((row) => {
          const defect = CRITERION_DEFECTS[row.id];
          return ui.stack(
            ui.verdict(
              `Criterion ${row.id}: ${row.requirement}`,
              row.status,
              row.status === 'passed' ? 'ok' : 'bad',
              `fixture ${row.fixture} · milestone ${row.milestone}`,
            ),
            row.status === 'passed' ? null : ui.note(row.observed),
            defect === undefined
              ? null
              : row.status === 'passed'
                ? ui.note(
                    `the bundle records as what closed it: ${valueOf(defect)}  [${cite(defect)}]`,
                    'ok',
                  )
                : ui.note(`the bundle's defect: ${valueOf(defect)}  [${cite(defect)}]`, 'bad'),
          );
        }),
      ),
      ui.block('The model', model),
      ui.block(
        'What it refuses',
        ui.note(
          'The write path refuses rather than raises, so each rule can be shown instead of caught. The message is the module\'s own text.',
        ),
        refusals,
        ui.quote(valueOf(CHAIN_RULES.appendOnly), cite(CHAIN_RULES.appendOnly)),
      ),
      ui.block('Where it goes wrong', ui.list(layer.caveats)),
      ui.block(
        'Where this came from',
        ui.sources(layer.sources),
        ui.sources([
          {
            label: 'space.py (the canonical serialisation and the digest recipe)',
            path: 'python/nautilus_trader/optimization/space.py',
          },
          {
            label: 'test_discovery_round_trip.py',
            path: 'python/tests/integration/test_discovery_round_trip.py',
          },
        ]),
        ui.note(
          'This console digests with SHA-256 and labels every value sha256:. The engine\'s own canonical digest is blake3 over a canonical serialisation, and no digest printed here is a digest recorded in the repository.',
        ),
      ),
    ),
  );

  render();
}
