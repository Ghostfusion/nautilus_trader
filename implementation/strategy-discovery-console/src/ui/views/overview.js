/**
 * The overview: what this console is, what it models, and what it refuses to claim.
 *
 * Every other view answers a question about one layer. This one answers the question a reader has
 * first - what am I looking at - and it answers it with the repository's own facts: the twelve
 * layers in pipeline order with the grade each tutorial page carries, the four acceptance criteria
 * that used to fail and what closed each, and the parts of the design that are recorded as unbuilt
 * or procedural.
 */

import { FORMER_FAILURES } from '../../data/acceptance.js';
import { criteria, failingCriteria, layers } from '../../data/layers.js';
import { valueOf } from '../../data/source.js';

export const meta = {
  id: 'overview',
  title: 'What this console is',
  order: 0,
  tutorial: 'tutorials/strategy-discovery/README.md',
};

export function mount(root, ctx) {
  const { ui, data } = ctx;
  const rows = data.layers();
  const all = data.criteria();
  const failing = data.failingCriteria();
  const passed = all.filter((row) => row.status === 'passed').length;
  const blocked = all.filter((row) => row.status === 'blocked').length;

  root.appendChild(
    ui.panel(
      { title: 'What this console is' },
      ui.h(
        'p',
        null,
        'This is a browser model of one piece of work in the NautilusTrader repository: the layer ',
        'that proposes candidate strategies, tests them through the simulation the repository already ',
        'has, and keeps a record of the whole search rather than of the winner. Nothing is installed, ',
        'nothing is downloaded and no Python runs. Every number on every page is computed in this ',
        'page from rules transcribed out of the repository, and every transcription is checked ',
        'against the file it came from by ',
        ui.h('code', { text: 'tools/check-facts.mjs' }),
        ', which also runs inside the test suite.',
      ),
      ui.h(
        'p',
        null,
        'The subject is machinery rather than a trade. The console does not decide whether any ',
        'strategy is good; it shows what the layer accepts, what it refuses, what it counts and what ',
        'it records, at every layer the tutorial group covers.',
      ),
      ui.metrics(
        ui.metric('Layers modelled', String(rows.length), 'one per tutorial page', 'accent'),
        ui.metric('Acceptance criteria', String(all.length), `${passed} pass, ${failing.length} failing`, failing.length === 0 ? 'ok' : 'warn'),
        ui.metric('Credentials refused', String(data.credentials().length), 'the canonical list', 'violet'),
        ui.metric('Blocked criteria', String(blocked), 'not attempted rather than failed', 'muted'),
      ),
    ),
  );

  root.appendChild(
    ui.panel(
      { title: 'The layers, in the order the work happens' },
      ui.table({
        columns: ['Step', 'Layer', 'The tutorial page', 'How well it held up', 'Criteria'],
        rows: rows.map((row) => [
          row.order,
          ui.repoLink(row.tutorial, row.title),
          ui.h('span', { class: 'mono', text: row.tutorial.replace('tutorials/strategy-discovery/', '') }),
          ui.h('span', { class: 'note', text: row.grade.split(':')[0] }),
          row.criteria.join(', ') || '-',
        ]),
      }),
      ui.note(
        'A grade is the tutorial collection\'s own verdict on the layer, and each is stated with what it rests on. The four grades that read "Mixed" all rest on something recorded as missing or procedural, and the pages that carry them say what.',
      ),
    ),
  );

  const byId = new Map(all.map((row) => [row.id, row]));
  const former = FORMER_FAILURES.map((failure) => {
    const row = byId.get(valueOf(failure.id));
    return [
      row.id,
      row.requirement,
      ui.badge(row.status, 'ok'),
      ui.h('span', { class: 'note', text: valueOf(failure.closed) }),
    ];
  });
  root.appendChild(
    ui.panel(
      { title: 'The four acceptance criteria that used to fail' },
      ui.h(
        'p',
        null,
        'The repository keeps one entry per acceptance criterion in the acceptance bundle, with the ',
        'test that exercises it and what was observed. All twenty-two pass now: none is failed and ',
        'none is blocked. These four used to fail, and each carries the bundle\'s own text recording ',
        'what closed it rather than what was once absent.',
      ),
      ui.table({
        columns: ['#', 'What it requires', 'Status', 'What closed it'],
        rows: former,
        dense: true,
      }),
      ui.note(
        'Two of the four shared one missing piece: a run identity that joins the seed, the build and the configuration at the engine\'s own boundary, and the engine now produces that manifest itself. One needed the settlement bridge to stop settling a deferred intent at the last known price. One needed the corporate-action stage to credit the entitlement a held position was owed. None of them was discovery-layer work, which is why the console reported the failure rather than a fix until the engine closed each.',
        'ok',
      ),
    ),
  );

  root.appendChild(
    ui.panel(
      { title: 'What this console does not do' },
      ui.list([
        'It places no orders, holds no credential and has no broker connection. The boundary view is about that absence, which is a property of the design rather than a switch.',
        'It is not the artifact of record. That is python/nautilus_trader/discovery/ and the Rust engine it calls. Where the two could be confused, the console says so on the page.',
        'Its digests are SHA-256 over a canonical serialisation and are labelled sha256:. The engine\'s canonical digest is blake3, and no digest computed here can be compared with one recorded there.',
        'It does not call a language model. The agents view replays recorded responses through the classifier the code applies to them.',
        'It makes no claim about profitability. The design states the deliverable is evidence, and it stops at a shadow stage.',
      ]),
      ui.note(
        'Every module the plan names in its layout exists, including the command-line entry point, `discovery/cli.py`: it reports the configuration document the loader admits rather than running a campaign, so the console says which half of a control a module enforces and which half remains procedural.',
      ),
    ),
  );

  root.appendChild(
    ui.panel(
      { title: 'How to read a layer' },
      ui.h(
        'p',
        null,
        'Every layer is a page in the same five parts: the tutorial\'s own paragraph, quoted, with ',
        'the grade it carries; the model of that layer, which you can drive; what the model refuses, ',
        'with the code it refuses under; where it goes wrong, again in the tutorial\'s words; and where ',
        'the rules came from, as paths into this repository.',
      ),
      ui.h(
        'p',
        null,
        'The models are small on purpose. Each one shows the decision its layer makes and the ',
        'refusals that surround it, not a reimplementation of the package. Where a model simplifies, ',
        'the page says which part it left out.',
      ),
    ),
  );

  root.appendChild(
    ui.panel(
      { title: 'Where this came from' },
      ui.sources([
        { label: 'the design', path: 'docs/design/strategy_discovery_design.md' },
        { label: 'the implementation plan', path: 'docs/design/strategy_discovery_implementation.md' },
        { label: 'the tutorial group', path: 'tutorials/strategy-discovery/README.md' },
        { label: 'the package', path: 'python/nautilus_trader/discovery/__init__.py' },
        { label: 'the acceptance record', path: 'python/tests/acceptance/discovery_acceptance_bundle.json' },
      ]),
    ),
  );

  void criteria;
  void layers;
  void failingCriteria;
}
