/**
 * Auditing a campaign from the ledger alone.
 *
 * Models `tutorials/strategy-discovery/auditing-a-campaign/README.md` and `audit_campaign` in
 * `python/nautilus_trader/discovery/ledger.py`. The view builds the tutorial's worked campaign on the
 * chain, reads it back with `derivedView`, and shows the reconstruction question by question: the
 * candidates that existed, which ran, which were judged, why each was turned away, and the one
 * outcome word the rule gives each of them. The declared inputs are controls, so a reader can watch
 * the reading's own code change over an identical chain, and the assertions the audit refuses to make
 * are listed beside the reading they limit.
 */

import { append } from '../../engine/ledger.js';
import { REFUSED_ASSERTIONS, derivedView, outcome } from '../../engine/audit.js';
import {
  AUDIT_DIGEST,
  CANDIDATE_SET,
  OUTCOME_RULE,
  POLICY_COMPARISON,
} from '../../data/ledger.js';
import { cite, normalise, sourceOf } from '../../data/source.js';

export const meta = {
  id: 'audit',
  title: 'Auditing a campaign from the ledger alone',
  order: 11,
  tutorial: 'tutorials/strategy-discovery/auditing-a-campaign/README.md',
};

/**
 * The campaign the tutorial works through, as events. It carries the four compilation events the
 * lineage is folded from, which the tutorial's ten-row table does not list; without them the reading
 * would hold no lineage at all. Everything else is the tutorial's own example.
 */
const CAMPAIGN = [
  ['search_policy_recorded', { search_policy_digest: 'sha256:sp1' }],
  ['proposal_recorded', { candidateId: 'alpha', generation: 0 }],
  [
    'compilation_recorded',
    { candidateId: 'alpha', specification_digest: 'sha256:spec-alpha', new_configuration: true },
  ],
  ['proposal_recorded', { candidateId: 'beta', generation: 1, parentId: 'alpha' }],
  [
    'compilation_recorded',
    { candidateId: 'beta', specification_digest: 'sha256:spec-beta', new_configuration: true },
  ],
  ['proposal_recorded', { candidateId: 'epsilon', generation: 0 }],
  [
    'compilation_recorded',
    {
      candidateId: 'epsilon',
      specification_digest: 'sha256:spec-epsilon',
      new_configuration: true,
    },
  ],
  ['proposal_recorded', { candidateId: 'delta', generation: 0 }],
  [
    'compilation_recorded',
    { candidateId: 'delta', specification_digest: 'sha256:spec-delta', new_configuration: false },
  ],
  ['specification_refused', { candidateId: 'gamma', code: 'VALUE_NOT_DECLARED' }],
  ['job_submitted', { candidateId: 'alpha', key: 'k1' }],
  ['execution_attempt_closed', { candidateId: 'alpha', key: 'k1', outcome: 'completed' }],
  [
    'registration_verified',
    {
      candidateId: 'alpha',
      key: 'k1',
      receipt: { build_identity: 'nautilus_trader 1.226.0' },
      receipt_digest: 'sha256:receipt-alpha',
    },
  ],
  ['trial_admitted', { candidateId: 'alpha', key: 'k1', receipt_digest: 'sha256:receipt-alpha' }],
  ['job_submitted', { candidateId: 'beta', key: 'k2' }],
  ['execution_attempt_closed', { candidateId: 'beta', key: 'k2', outcome: 'crashed' }],
  [
    'registration_incomplete',
    { candidateId: 'beta', key: 'k2', reason: 'crashed with no usable return series' },
  ],
  ['job_submitted', { candidateId: 'epsilon', key: 'k3' }],
  ['execution_attempt_closed', { candidateId: 'epsilon', key: 'k3', outcome: 'completed' }],
  [
    'registration_verified',
    {
      candidateId: 'epsilon',
      key: 'k3',
      receipt: { build_identity: 'nautilus_trader 1.226.0' },
      receipt_digest: 'sha256:receipt-epsilon',
    },
  ],
  [
    'trial_admitted',
    { candidateId: 'epsilon', key: 'k3', receipt_digest: 'sha256:receipt-epsilon' },
  ],
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

/** The verdict the gate service reported, which the reading must agree with word for word. */
const GATE_REPORT = [
  {
    candidateId: 'alpha',
    gate: 'G3',
    status: 'INCONCLUSIVE',
    reason: 'not distinguishable from the best of a search this size',
  },
];

const RECORDED_POLICY = 'sha256:policy-1';
const ENGINE_VERSION = '1.226.0';

/** Build the campaign through the write path, so the demo chain obeys the ledger's own rules. */
function campaignChain() {
  let entries = [];
  for (const [kind, payload] of CAMPAIGN) {
    const result = append(entries, kind, payload);
    if (result.ok) entries = result.entries;
  }
  return entries;
}

/** The transcribed rule lines, whitespace collapsed, each with the file it was read from. */
function ruleRows(facts) {
  return facts.map((fact) => [normalise(sourceOf(fact).text), cite(fact)]);
}

/** The order the outcome rule is applied in, and the four words it can return. */
const OUTCOME_RULES = [
  OUTCOME_RULE.verdictFirst,
  OUTCOME_RULE.words.admitted,
  OUTCOME_RULE.words.ran,
  OUTCOME_RULE.words.refused,
  OUTCOME_RULE.words.proposed,
];

export function mount(root, ctx) {
  const { ui, data } = ctx;
  const layer = data.layer(meta.id);

  const state = {
    entries: campaignChain(),
    declarePolicy: true,
    policyDigest: RECORDED_POLICY,
    recordedPolicyDigest: RECORDED_POLICY,
    engineVersion: ENGINE_VERSION,
    perturbReport: false,
    asked: 'zeta',
  };

  const model = ui.h('div', { class: 'stack' });
  const refusals = ui.h('div', { class: 'stack' });

  function readIt() {
    const report = state.perturbReport
      ? GATE_REPORT.map((row) => ({ ...row, reason: `${row.reason} (as the report wrote it)` }))
      : GATE_REPORT;
    return derivedView(state.entries, {
      campaignId: 'campaign-1',
      familyId: 'family-1',
      policyDigest: state.declarePolicy ? state.policyDigest : '',
      recordedPolicyDigest: state.recordedPolicyDigest,
      engineVersion: state.engineVersion,
      gateReport: report,
    });
  }

  function renderModel() {
    ui.clear(model);
    const reading = readIt();
    const again = readIt();
    const words = outcome(reading);
    const policy =
      reading.policyMatches === null
        ? { value: 'nothing declared', tone: 'muted', detail: 'no comparison at all, which is not a failed match' }
        : reading.policyMatches
          ? { value: 'the declared digest is the recorded one', tone: 'ok', detail: 'the reading is made under that policy' }
          : { value: 'the declared digest differs', tone: 'bad', detail: 'the reading describes a campaign under a policy the inventory was not opened with' };

    model.appendChild(
      ui.group(
        'The reading',
        ui.grid(
          2,
          ui.toggle({
            id: 'audit-declare-policy',
            checked: state.declarePolicy,
            label: 'declare a policy digest',
            onChange: (checked) => {
              state.declarePolicy = checked;
              render();
            },
          }),
          ui.field(
            'the declared policy digest',
            ui.textInput({
              id: 'audit-policy',
              value: state.policyDigest,
              onInput: (value) => {
                state.policyDigest = value;
                render();
              },
            }),
            'an input to the audit, not something the ledger can invent',
          ),
          ui.field(
            'the digest the inventory was opened with',
            ui.textInput({
              id: 'audit-recorded-policy',
              value: state.recordedPolicyDigest,
              onInput: (value) => {
                state.recordedPolicyDigest = value;
                render();
              },
            }),
          ),
          ui.field(
            'the engine version the reader holds',
            ui.textInput({
              id: 'audit-engine',
              value: state.engineVersion,
              onInput: (value) => {
                state.engineVersion = value;
                render();
              },
            }),
            'reported beside the build identities as recorded; the audit does not compare them',
          ),
        ),
        ui.metrics(
          ui.metric('Events read', reading.events, 'the chain, in append order'),
          ui.metric('Candidates', reading.candidates.length, 'lineage first, then any later name'),
          ui.metric('Judged', reading.judged.length === 0 ? 'none' : reading.judged.join(', ')),
          ui.metric('Outside the lineage', reading.outsideLineage.join(', ') || 'none', 'named by a refusal, a verdict or an attempt'),
        ),
        ui.verdict('The policy comparison', policy.value, policy.tone, policy.detail),
        ui.verdict(
          'Two readings of the same chain',
          again.digest === reading.digest ? 'the same code' : 'different codes',
          again.digest === reading.digest ? 'ok' : 'bad',
          'the same chain and the same declared inputs give the same plain data',
        ),
        ui.row(
          ui.badge('the reading\'s own code', 'accent'),
          ui.digest(reading.digest, 24),
        ),
      ),
    );

    model.appendChild(
      ui.group(
        'Every candidate, reconstructed from the chain alone',
        ui.table({
          columns: [
            'candidate',
            'generation',
            'parent',
            'attempts',
            'outcomes',
            'admitted',
            'verdict',
            'outcome',
          ],
          rows: reading.candidates.map((row) => [
            row.candidateId,
            row.generation,
            row.parentId === '' ? 'none' : row.parentId,
            row.attempts.length === 0 ? 'none' : row.attempts.join(', '),
            row.outcomes.length === 0 ? 'none' : row.outcomes.join(', '),
            row.admitted,
            row.verdicts.length === 0
              ? 'none'
              : row.verdicts.map((verdict) => `${verdict.gate} ${verdict.status}`).join(', '),
            ui.badge(row.outcome, row.verdicts.length > 0 ? 'violet' : 'accent'),
          ]),
        }),
        ui.list(
          reading.candidates.map(
            (row, index) =>
              `${row.candidateId}: ${words[index].outcome}${
                row.verdicts.length > 0
                  ? ` - the last verdict's status is tested first, so ${row.admitted} admitted attempt(s) do not make it "admitted"`
                  : row.attempts.length > 0
                    ? ` - it ran, and ${row.admitted} of its attempts entered the statistical inventory`
                    : row.refusals.length > 0
                      ? ` - refused (${row.refusals.join(', ')}) and it never ran`
                      : ' - written down and nothing further happened to it'
              }`,
          ),
        ),
        reading.outsideLineage.length === 0
          ? null
          : ui.note(
              `The compiled lineage cannot hold ${reading.outsideLineage.join(', ')}: the reading names ${
                reading.outsideLineage.length === 1 ? 'it' : 'them'
              } from the event that does, and puts a candidate that never compiled last.`,
            ),
      ),
    );

    model.appendChild(
      ui.group(
        'The gate report the reading is checked against',
        ui.toggle({
          id: 'audit-perturb-report',
          checked: state.perturbReport,
          label: "rewrite the report's reason, to see the check fire",
          onChange: (checked) => {
            state.perturbReport = checked;
            render();
          },
        }),
        ui.table({
          columns: ['candidate', 'gate', 'status', 'reason the gate wrote'],
          rows: (state.perturbReport
            ? GATE_REPORT.map((row) => ({ ...row, reason: `${row.reason} (as the report wrote it)` }))
            : GATE_REPORT
          ).map((row) => [row.candidateId, row.gate, row.status, row.reason]),
        }),
        ui.verdict(
          'The reading against the report',
          reading.gateAgreement.ok ? 'agrees word for word' : 'disagrees',
          reading.gateAgreement.ok ? 'ok' : 'warn',
          'the reasons read back from the chain equal the reasons the gate returned, or the difference is named',
        ),
        reading.gateAgreement.ok
          ? null
          : ui.list(
              reading.gateAgreement.disagreements.map(
                (item) =>
                  `${item.candidateId} · ${item.field}: the chain holds ${JSON.stringify(item.audit)} and the report holds ${JSON.stringify(item.report)}`,
              ),
              { className: 'reasons' },
            ),
      ),
    );

    model.appendChild(
      ui.group(
        'The chain it read',
        ui.table({
          dense: true,
          columns: ['#', 'kind', 'candidate', 'key', 'digest', 'chain'],
          rows: state.entries.map((item) => [
            item.sequence,
            item.kind,
            item.candidateId === '' ? '-' : item.candidateId,
            item.key === '' ? '-' : item.key,
            ui.digest(item.digest),
            ui.digest(item.chain),
          ]),
        }),
        ui.note(
          `The audit reads these records and verifies none of them: verification is the record layer's job, and the reading is the same whether or not the chain holds. The values are the tutorial's worked example, which says its numbers are made up.`,
          'muted',
        ),
      ),
    );

    model.appendChild(
      ui.group(
        'The rules the reading is built from',
        ui.table({
          caption: 'outcome, taken from the top down',
          columns: ['the rule, as the tutorial writes it', 'source'],
          rows: ruleRows(OUTCOME_RULES),
        }),
        ui.table({
          caption: 'the candidate set, a union in a fixed order',
          columns: ['the rule, as the tutorial writes it', 'source'],
          rows: ruleRows([CANDIDATE_SET.order, CANDIDATE_SET.firstSeen]),
        }),
        ui.table({
          caption: 'the policy comparison',
          columns: ['the rule, as the tutorial writes it', 'source'],
          rows: ruleRows([POLICY_COMPARISON.tutorial, POLICY_COMPARISON.nothingDeclaredTutorial]),
        }),
        ui.table({
          caption: 'the reading itself is given one code',
          columns: ['the rule, as the tutorial writes it', 'source'],
          rows: ruleRows([AUDIT_DIGEST.rule]),
        }),
      ),
    );
  }

  function renderRefusals() {
    ui.clear(refusals);
    const reading = readIt();
    const named = reading.candidates.some((row) => row.candidateId === state.asked);

    refusals.appendChild(
      ui.list(
        REFUSED_ASSERTIONS.map((item) => `${item.assertion}  [${item.id}]`),
        { className: 'reasons' },
      ),
    );
    refusals.appendChild(
      ui.group(
        'A candidate the ledger never wrote down',
        ui.field(
          'ask the reading to name a candidate',
          ui.textInput({
            id: 'audit-asked',
            value: state.asked,
            onInput: (value) => {
              state.asked = value;
              render();
            },
          }),
        ),
        named
          ? ui.note(`${state.asked} is in the reading: the chain names it, so a record exists for it.`, 'ok')
          : ui.note(
              `${state.asked} is not in the reading and cannot be put there: the audit adds no record and can invent none, so a candidate the chain never names leaves no trace.`,
              'warn',
            ),
      ),
    );
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
        ui.metric('Grade', 'strong as a definition', layer.grade, 'ok'),
        ...data
          .criteria()
          .filter((row) => layer.criteria.includes(row.id))
          .map((row) =>
            ui.stack(
              ui.verdict(
                `Criterion ${row.id}: ${row.requirement}`,
                row.status,
                row.status === 'passed' ? 'ok' : 'bad',
                `fixture ${row.fixture} · milestone ${row.milestone}`,
              ),
              row.status === 'passed' ? null : ui.note(row.observed),
            ),
          ),
      ),
      ui.block('The model', model),
      ui.block(
        'What it refuses',
        ui.note(
          'The audit is a reading, so what it refuses is a set of assertions rather than a set of refusal codes. Each is a statement the reading deliberately does not make.',
        ),
        refusals,
      ),
      ui.block('Where it goes wrong', ui.list(layer.caveats)),
      ui.block(
        'Where this came from',
        ui.sources(layer.sources),
        ui.note(
          'This console digests with SHA-256 and labels every value sha256:. The reading\'s code here covers this console\'s plain data, not the repository\'s, and no digest printed here is compared with a digest recorded in the repository.',
        ),
      ),
    ),
  );

  render();
}
