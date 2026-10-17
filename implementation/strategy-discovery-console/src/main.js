/**
 * Boot: the navigation, the active view, and the one campaign every view shares.
 *
 * Views are mounted one at a time into #view and own their own DOM afterwards; the only thing this
 * module does on a change is to mount the active view again. The campaign store is the single shared
 * object: the campaign view writes it, the views downstream of a campaign read it.
 */

import { LAYER_ORDER, layer } from './data/layers.js';
import { store } from './store.js';
import * as data from './data/layers.js';
import * as ui from './ui/shell.js';

import * as agents from './ui/views/agents.js';
import * as acceptance from './ui/views/acceptance.js';
import * as audit from './ui/views/audit.js';
import * as boundary from './ui/views/boundary.js';
import * as campaign from './ui/views/campaign.js';
import * as candidate from './ui/views/candidate.js';
import * as counts from './ui/views/counts.js';
import * as executor from './ui/views/executor.js';
import * as gates from './ui/views/gates.js';
import * as genome from './ui/views/genome.js';
import * as holdout from './ui/views/holdout.js';
import * as overview from './ui/views/overview.js';
import * as policy from './ui/views/policy.js';
import * as record from './ui/views/record.js';
import * as shadow from './ui/views/shadow.js';

const MODULES = [
  agents,
  acceptance,
  audit,
  boundary,
  campaign,
  candidate,
  counts,
  executor,
  gates,
  genome,
  holdout,
  overview,
  policy,
  record,
  shadow,
];

const BY_ID = new Map(MODULES.map((module) => [module.meta.id, module]));

/** Every layer view, in pipeline order, taken from the generated layer rows. */
const LAYER_VIEWS = LAYER_ORDER.map((id) => {
  const module = BY_ID.get(id);
  if (!module) throw new Error(`no view is mounted for the layer "${id}"`);
  return module;
});

/** The navigation: the layers in pipeline order, the whole campaign, and how to read any of it. */
const GROUPS = [
  { title: 'One campaign, end to end', views: [campaign, acceptance] },
  { title: 'The layers', views: LAYER_VIEWS },
  { title: 'About', views: [overview] },
];

const viewRoot = document.getElementById('view');
const navRoot = document.getElementById('nav');
const footerRoot = document.getElementById('footer');
let active = null;

function context() {
  return {
    ui,
    data,
    store,
    rerender: () => mountActive(),
  };
}

/** Mount one view. The view builds its own subtree and wires its own handlers. */
function mountView(module) {
  active = module;
  ui.clear(viewRoot);
  viewRoot.appendChild(ui.h('h2', { text: module.meta.title }));
  viewRoot.appendChild(
    ui.h('p', { class: 'note' }, 'Tutorial page: ', ui.repoLink(module.meta.tutorial, module.meta.tutorial)),
  );
  module.mount(viewRoot, context());
  document.title = `${module.meta.title} - Strategy Discovery Console`;
  location.hash = `#/${module.meta.id}`;
  highlight();
}

function mountActive() {
  if (active) mountView(active);
}

function highlight() {
  for (const button of navRoot.querySelectorAll('button')) {
    button.classList.toggle('active', button.dataset.view === active?.meta.id);
  }
}

function buildNav() {
  ui.clear(navRoot);
  navRoot.appendChild(
    ui.h('p', { class: 'group', text: 'Strategy discovery' }),
  );

  for (const group of GROUPS) {
    navRoot.appendChild(ui.h('p', { class: 'group', text: group.title }));
    for (const module of group.views) {
      const isLayer = LAYER_ORDER.includes(module.meta.id);
      const step = isLayer ? String(layer(module.meta.id).order) : '';
      const button = ui.h(
        'button',
        {
          type: 'button',
          dataset: { view: module.meta.id },
          onclick: () => mountView(module),
        },
        isLayer ? ui.h('span', { class: 'step', text: step }) : null,
        module.meta.title,
      );
      navRoot.appendChild(button);
    }
  }
}

function buildFooter() {
  footerRoot.appendChild(
    ui.h(
      'p',
      null,
      'A browser model of the strategy discovery layer of the NautilusTrader repository. The ',
      'artifact of record is ',
      ui.h('code', { text: 'python/nautilus_trader/discovery/' }),
      ' and the Rust engine it calls; this console computes its own numbers from rules transcribed ',
      'out of that code and the documents beside it, and every transcription is verified against ',
      'the file it came from by ',
      ui.h('code', { text: 'bun run verify' }),
      ' and by the test suite that runs the same check.',
    ),
  );
  footerRoot.appendChild(
    ui.h(
      'p',
      { class: 'note' },
      'Its digests are SHA-256, labelled ',
      ui.h('code', { text: 'sha256:' }),
      ', and are not comparable with the engine\'s blake3 identities. It places no order, holds no ',
      'credential and calls no model. All twenty-two acceptance criteria pass, and the four that ',
      'used to fail are reported with the bundle\'s own record of what closed each.',
    ),
  );
}

function boot() {
  buildNav();
  buildFooter();

  const requested = location.hash.replace(/^#\/?/, '');
  const initial = BY_ID.get(requested) ?? LAYER_VIEWS[0];
  mountView(initial);

  window.addEventListener('hashchange', () => {
    const id = location.hash.replace(/^#\/?/, '');
    const module = BY_ID.get(id);
    if (module && module !== active) mountView(module);
  });

  // A campaign change invalidates every view downstream of it, so the active view is rebuilt.
  store.subscribe(() => mountActive());
}

boot();
