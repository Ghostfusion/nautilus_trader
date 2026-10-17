/**
 * The search view: the genotype, the three degrees of freedom and one generation of breeding.
 *
 * Models `python/nautilus_trader/discovery/genome.py` and the breeding half of `evolve.py`. The
 * genotype is every addressable value of a candidate, so the view shows the genes a mutation may
 * move and the bound or declared set each one resolves through; the generation is a front over
 * declared measurements and the children one generation breeds from it. Nothing here runs a
 * backtest and nothing here measures anything: the search never sees a result, and the numbers the
 * front ranks are the reader's, which is why the paper exercise is editable.
 */

import {
  DEGREES_OF_FREEDOM,
  GENES,
  amend,
  crossover,
  digestOfSpecification,
  dominates,
  equivalenceKey,
  genesOf,
  makeGenome,
  mechanismOf,
  mutate,
  nextGeneration,
  paretoFront,
  replaceMechanism,
  rngFor,
} from '../../engine/genome.js';
import { shortDigest } from '../../engine/digest.js';
import {
  complexityLimits,
  demoPolicy,
  frozenFields,
  ignoredPaths,
  objectives as objectiveRows,
  paperExercise,
  refusalCodes,
  searchConstants,
  searchPolicy,
} from '../../data/genome.js';

/** What this view models: a candidate's genome, the operators that move it, and one generation. */
export const meta = {
  id: 'genome',
  title: 'Searching for strategies by breeding candidates',
  order: 8,
  tutorial: 'tutorials/strategy-discovery/search-by-evolution/README.md',
};

/** The population size the console draws, so the front has candidates to rank. */
const DEMO_POPULATION = 5;

/** Grade a layer's own paragraph into the tone the verdict is shown with. */
function gradeTone(grade) {
  if (grade.startsWith('Strong')) return 'ok';
  if (grade.startsWith('Mixed')) return 'warn';
  return 'bad';
}

/** Build the view into the empty root, and wire every control the model needs. */
export function mount(root, ctx) {
  const { ui, data } = ctx;
  const layer = data.layer(meta.id);
  const policy = demoPolicy();
  const limits = complexityLimits();
  const constants = searchConstants();
  const catalogue = objectiveRows();
  const exercise = paperExercise();
  const codes = refusalCodes();

  const state = {
    seed: 7,
    generation: 3,
    features: 1,
    leaves: 2,
    kept: exercise.columns.map((column) => column.objective),
    numbers: exercise.rows.map((row) => ({ ...row })),
    demonstration: null,
  };

  /** The population a seed and a generation draw, which is what the console's search starts from. */
  const population = () => {
    const rng = rngFor(state.seed, state.generation);
    return Array.from({ length: DEMO_POPULATION }, () =>
      makeGenome(rng, policy, {
        seed: state.seed,
        features: state.features,
        conditions: state.leaves,
      }),
    );
  };

  /** The declared measurements of one candidate, which the search reads rather than computes. */
  const declaredObjectives = (index) =>
    Object.fromEntries(
      exercise.columns.map((column) => [column.objective, state.numbers[index][column.key]]),
    );

  /** The declared results of a population, keyed by candidate identity. */
  const declaredResults = (genomes) =>
    genomes.map((genome, index) => ({
      candidateId: `candidate-${index + 1}`,
      digest: digestOfSpecification(genome),
      family: genome.identity.family,
      partition: 'development',
      role: 'development',
      objectives: declaredObjectives(index),
    }));

  // --- the demonstrations --------------------------------------------------
  //
  // Every demonstration runs the operator the code runs and renders what it returned, so a refusal
  // is shown rather than claimed.

  const demonstration = ui.h('div', null);

  const showDemonstration = () => {
    ui.clear(demonstration);
    demonstration.appendChild(
      state.demonstration
        ? ui.verdict(
            state.demonstration.label,
            state.demonstration.outcome.ok
              ? 'not refused'
              : String(state.demonstration.outcome.code).replaceAll('_', ' '),
            state.demonstration.outcome.ok ? 'warn' : 'bad',
            state.demonstration.outcome.ok
              ? 'The operator produced an edit where this demonstration expected a refusal.'
              : state.demonstration.outcome.reason,
          )
        : ui.note('Run a demonstration to see the refusal it reports.'),
    );
  };

  /** A generation reports its refusal inside the plan, so both shapes are read as one here. */
  const asRefusal = (outcome) =>
    outcome.ok === false
      ? outcome
      : outcome.refusal
        ? { ok: false, code: outcome.refusal.code, reason: outcome.reason }
        : outcome;

  const refused = (label, outcome) => {
    state.demonstration = { label, outcome: asRefusal(outcome) };
    showDemonstration();
  };

  // --- block two: the model ------------------------------------------------

  const results = ui.h('div', null);

  const seedInput = ui.numberInput({
    id: 'genome-seed',
    value: state.seed,
    min: 0,
    step: 1,
    onInput: (value) => {
      if (!Number.isFinite(value)) return;
      state.seed = value;
      render();
    },
  });
  const generationInput = ui.numberInput({
    id: 'genome-generation',
    value: state.generation,
    min: 0,
    step: 1,
    onInput: (value) => {
      if (!Number.isFinite(value)) return;
      state.generation = value;
      render();
    },
  });
  const featureInput = ui.numberInput({
    id: 'genome-features',
    value: state.features,
    min: 1,
    max: limits.maxFeatures,
    step: 1,
    onInput: (value) => {
      if (!Number.isFinite(value)) return;
      state.features = Math.max(
        1,
        Math.min(value, limits.maxFeatures, policy.featureAllowlist.length),
      );
      render();
    },
  });
  const leafInput = ui.numberInput({
    id: 'genome-leaves',
    value: state.leaves,
    min: 0,
    max: limits.maxConditions,
    step: 1,
    onInput: (value) => {
      if (!Number.isFinite(value)) return;
      state.leaves = Math.max(0, Math.min(value, limits.maxConditions));
      render();
    },
  });

  const keptToggles = ui.h(
    'div',
    { class: 'row' },
    ...catalogue.map((objective) =>
      ui.toggle({
        id: `genome-kept-${objective.id}`,
        checked: state.kept.includes(objective.id),
        label: `${objective.id} (${objective.prefersHigh ? 'higher' : 'lower'} is better)`,
        onChange: (checked) => {
          state.kept = checked
            ? [...state.kept, objective.id]
            : state.kept.filter((name) => name !== objective.id);
          render();
        },
      }),
    ),
  );

  const exerciseTable = () => {
    const front = paretoFront(declaredResults(population()), { objectives: state.kept });
    const keptIds = new Set(front.map((result) => result.candidateId));
    const columns = ['candidate', ...exercise.columns.map((column) => column.label), 'kept'];
    const rows = state.numbers.map((row, index) => [
      `candidate-${index + 1}`,
      ...exercise.columns.map((column) =>
        ui.numberInput({
          id: `genome-number-${index}-${column.key}`,
          value: row[column.key],
          step: 1,
          onInput: (value, event) => {
            if (!Number.isFinite(value)) return;
            state.numbers[index][column.key] = value;
            state.focus = event?.target?.id ?? null;
            render();
          },
        }),
      ),
      keptIds.has(`candidate-${index + 1}`)
        ? ui.badge('on the front', 'ok')
        : ui.badge('left out', 'muted'),
    ]);
    return ui.table({
      columns,
      rows,
      dense: true,
      caption:
        'The five made-up candidates of the page\u2019s paper exercise. Every number is the reader\u2019s: the search ranks declared measurements and never measures one itself.',
    });
  };

  const frontTable = () => {
    const genomes = population();
    const declared = declaredResults(genomes);
    const front = paretoFront(declared, { objectives: state.kept });
    const onFront = new Set(front.map((result) => result.candidateId));
    const beaten = (result) =>
      declared.some(
        (other) => other.candidateId !== result.candidateId && dominates(other, result, state.kept),
      );
    const rows = declared.map((result) => [
      result.candidateId,
      result.family,
      ...exercise.columns.map((column) =>
        column.objective in result.objectives ? result.objectives[column.objective] : 'not measured',
      ),
      onFront.has(result.candidateId) ? ui.badge('kept', 'ok') : ui.badge('left out', 'muted'),
      onFront.has(result.candidateId)
        ? 'no other candidate beats it everywhere'
        : beaten(result)
          ? 'beaten on every kept measurement'
          : 'not comparable: a kept objective was never measured',
    ]);
    return ui.table({
      columns: [
        'candidate',
        'family',
        ...exercise.columns.map((column) => column.objective),
        'front',
        'why',
      ],
      rows,
      dense: true,
      caption: `Compared on ${state.kept.length} of ${catalogue.length} objectives.`,
    });
  };

  const genotypeTable = (genome) => {
    const genes = genesOf(genome, policy);
    const rows = genes.map((gene) => [
      gene.path,
      ui.badge(gene.degree, gene.degree === 'parameter' ? 'accent' : 'violet'),
      String(gene.value),
      gene.bound
        ? `range ${gene.bound.lower} to ${gene.bound.upper}`
        : gene.declared.length > 0
          ? `declared: ${gene.declared.join(', ')}`
          : 'boolean',
      gene.choices.length > 0 ? gene.choices.map(String).join(', ') : 'drawn inside the range',
    ]);
    return ui.table({
      columns: ['path', 'degree', 'value', 'resolves through', 'other admissible values'],
      rows,
      dense: true,
      caption: `${genes.length} genes, ordered by path: ${genes.filter((gene) => gene.degree === 'structural').length} structural and ${genes.filter((gene) => gene.degree === 'parameter').length} parameter.`,
    });
  };

  const generationBlock = () => {
    const genomes = population();
    const declared = declaredResults(genomes);
    const plan = nextGeneration({
      population: genomes,
      results: declared,
      generation: state.generation,
      policy,
      searchPolicy: searchPolicy(),
      seed: state.seed,
      objectives: state.kept,
    });
    const children = plan.population.map((child, index) => {
      const edit = plan.edits.find((item) => item.ok && item.genome === child);
      return [
        index + 1,
        ui.digest(digestOfSpecification(child)),
        child.identity.family,
        ...['parameter', 'structural', 'family'].map((degree) => {
          const changes = (edit?.changes ?? []).filter((change) => change.degree === degree);
          return changes.length > 0 ? ui.badge(`${degree} ${changes.length}`, 'accent') : '';
        }),
        ui.digest(equivalenceKey(child)),
      ];
    });
    const refusedEdits = plan.edits.filter((edit) => !edit.ok);
    return ui.stack(
      ui.metrics(
        ui.metric('population', genomes.length, 'what the console draws; the repository starts from compiled candidates'),
        ui.metric('on the front', plan.front.length, `kept from ${declared.length} declared results`),
        ui.metric('survivors', plan.survivors.length, 'elite, then each family floor, then the rest'),
        ui.metric('children', plan.population.length, `retried up to ${constants.editAttempts} times per place`),
        ui.metric('merges', plan.merges.length, `under ${constants.equivalenceRuleId}`, plan.merges.length ? 'warn' : null),
      ),
      ui.note(
        `The generator is the console's own, seeded with "${state.seed}:${state.generation}" and not CPython's. The page's rule is that the same pair draws the same sequence; it does not promise that this console draws the repository's sequence.`,
      ),
      plan.refusal
        ? ui.verdict('the generation refused', plan.refusal.code, 'bad', plan.reason)
        : ui.note('The generation produced its plan.', 'ok'),
      refusedEdits.length > 0
        ? ui.group(
            'edits the operators refused',
            ui.table({
              columns: ['code', 'reason'],
              rows: refusedEdits.map((edit) => [ui.badge(edit.code, 'warn'), edit.reason]),
              dense: true,
            }),
          )
        : null,
      ui.group(
        'the children',
        ui.table({
          columns: [
            '#',
            'specification',
            'family',
            ...['parameter', 'structural', 'family'].map((degree) => `${degree} changes`),
            'equivalence key',
          ],
          rows: children,
          dense: true,
          caption:
            'Each child carries no measurement: the search compiles candidates and never runs a backtest, so a child is a specification until a later stage measures it.',
        }),
      ),
    );
  };

  // --- block three: what it refuses ---------------------------------------

  const demonstrations = () => {
    /** A generation over the population in force, with the results replaced as the demo needs. */
    const generation = (populationNow, replace) =>
      nextGeneration({
        population: populationNow,
        results: declaredResults(populationNow).map(replace ?? ((result) => result)),
        generation: state.generation,
        policy,
        searchPolicy: searchPolicy(),
        seed: state.seed,
        objectives: state.kept,
      });

    /** One candidate moved to another declared family, which is the third degree of freedom. */
    const movedFamily = (genome) => {
      const held = mechanismOf(genome);
      const family = policy.sets.families.find((name) => name !== held.family);
      return replaceMechanism(
        genome,
        {
          family,
          mechanism: 'a mechanism',
          hypothesis: 'a hypothesis',
          falsification: 'a falsification',
        },
        policy,
      ).genome;
    };

    const rows = [
      {
        code: 'gene_unresolved',
        condition: 'a named path is not a gene, or the selection named nothing',
        run: () =>
          mutate(population()[0], policy, {
            paths: ['entry.conditions.nope'],
            rng: rngFor(state.seed, 1),
          }),
      },
      {
        code: 'gene_not_mutable',
        condition: 'a named path is a statement, which needs a replacement statement',
        run: () =>
          mutate(population()[0], policy, {
            paths: ['identity.hypothesis'],
            rng: rngFor(state.seed, 1),
          }),
      },
      {
        code: 'no_free_gene',
        condition: 'a gene has no other admissible value, or two parents share none',
        run: () =>
          mutate(population()[0], policy, {
            paths: ['data.signal_frequency'],
            rng: rngFor(state.seed, 1),
          }),
      },
      {
        code: 'family_mismatch',
        condition: 'recombination was asked for between two families',
        run: () => {
          const [first] = population();
          return crossover(first, movedFamily(first), policy, { rng: rngFor(state.seed, 1) });
        },
      },
      {
        code: 'value_not_declared',
        condition: 'a replacement statement is blank, or a family is not declared',
        run: () => {
          const genome = population()[0];
          return replaceMechanism(genome, { ...mechanismOf(genome), hypothesis: '  ' }, policy);
        },
      },
      {
        code: 'population_empty',
        condition: 'a generation was planned from an empty population',
        run: () => generation([]),
      },
      {
        code: 'locked_result_in_selection',
        condition: 'a result offered to selection is not development data',
        run: () =>
          generation(population(), (result, index) =>
            index === 0 ? { ...result, role: 'locked_validation' } : result,
          ),
      },
      {
        code: 'missing_objective',
        condition: 'no result carried a complete set of development objectives',
        run: () => generation(population(), (result) => ({ ...result, objectives: {} })),
      },
      {
        code: 'frozen_field',
        condition: 'the search policy was amended outside its mutable set',
        run: () => amend(searchPolicy(), { bounds: 1 }),
      },
    ];
    return ui.stack(
      ui.note(
        'Each row runs the operator the code runs. Two codes of the search vocabulary are declared and have no raising site here: `unknown_objective` has none in evolve.py at all, and `role_mismatch` belongs to the partition-role rule another layer owns. `edit_refused` needs a population whose children all fold back into it, which this demo does not reach.',
        'muted',
      ),
      ui.table({
        columns: ['code', 'what produces it', ''],
        rows: rows.map((row) => [
          ui.badge(row.code, 'warn'),
          row.condition,
          ui.button('demonstrate', () => refused(row.code, row.run())),
        ]),
        dense: true,
      }),
      demonstration,
      ui.group(
        'the codes, in the order the source declares them',
        ui.table({
          columns: ['genotype-level', 'search-level'],
          rows: Array.from({ length: Math.max(codes.genome.length, codes.search.length) }, (_, index) => [
            codes.genome[index] ?? '',
            codes.search[index] ?? '',
          ]),
          dense: true,
        }),
      ),
    );
  };

  // --- the view ------------------------------------------------------------

  function render() {
    ui.clear(results);
    const genomes = population();
    const first = genomes[0];
    const declared = declaredResults(genomes);
    const genes = genesOf(first, policy);
    const front = paretoFront(declared, { objectives: state.kept });
    const comparables = declared.filter((result) =>
      state.kept.every((name) => name in result.objectives),
    );
    results.appendChild(
      ui.stack(
        ui.metrics(
          ui.metric('genes', genes.length, `${genes.filter((gene) => gene.degree === 'structural').length} structural, ${genes.filter((gene) => gene.degree === 'parameter').length} parameter`),
          ui.metric('kept objectives', state.kept.length, `of ${catalogue.length} named measurements`),
          ui.metric('comparables', comparables.length, 'a result missing a kept objective is not comparable'),
          ui.metric('on the front', front.length, 'a front is a set of options, not a finding', front.length === 0 ? 'warn' : 'accent'),
          ui.metric('specification at seed ' + state.seed, shortDigest(digestOfSpecification(first)), 'the console\u2019s SHA-256 over its own canonical text'),
        ),
        ui.note(
          `The population below is drawn by the console, because the repository has no initial-population operator: the search starts from candidates the compiler produced. Every value here comes from a declared set or a declared bound.`,
          'muted',
        ),
        ui.group(
          'the seven objectives',
          ui.table({
            columns: ['objective', 'the convention its name carries', 'prefers'],
            rows: catalogue.map((objective) => [
              objective.id,
              objective.convention,
              objective.prefersHigh ? ui.badge('high', 'ok') : ui.badge('low', 'accent'),
            ]),
            dense: true,
            caption:
              'An objective is only as good as its stated convention: the same name at two frequencies is two different quantities, so a comparison across conventions is meaningless.',
          }),
        ),
        ui.group('which measurements are kept', keptToggles, ui.note('Erasing a column is the page\u2019s own exercise. A candidate is comparable only if every kept objective was declared for it.')),
        ui.group('the declared measurements', exerciseTable()),
        ui.group('the front', frontTable()),
        ui.group('one generation', generationBlock()),
        ui.group(
          'one genotype',
          ui.note(
            `The first candidate of the population. The three degrees of freedom are ${DEGREES_OF_FREEDOM.map((degree) => degree.id).join(', ')}; only the family degree replaces the statements, and no operator draws one.`,
          ),
          genotypeTable(first),
          ui.note(
            `The statements are the console\u2019s placeholder: a statement is written by an agent and replaced only through mechanism replacement. An equivalence key ignores ${ignoredPaths().join(', ')}.`,
            'muted',
          ),
        ),
      ),
    );
    if (state.focus) {
      const field = root.querySelector(`#${state.focus}`);
      if (field) field.focus();
    }
  }

  root.appendChild(
    ui.panel(
      { title: layer.title, subtitle: layer.tutorial, id: 'genome' },
      ui.block(
        'The idea',
        ui.quote(layer.idea, layer.tutorial),
        ui.verdict('grade', layer.grade.split(':')[0], gradeTone(layer.grade), layer.grade),
      ),
      ui.block(
        'The model',
        ui.row(
          ui.field('seed', seedInput, 'the campaign seed'),
          ui.field('generation', generationInput, 'the generation number'),
          ui.field('feature declarations', featureInput, `bounded by the policy's ${limits.maxFeatures}`),
          ui.field('condition leaves', leafInput, `bounded by the policy's ${limits.maxConditions}; above ${limits.maxConditionDepth} deep is out of scope`),
        ),
        results,
      ),
      ui.block('What it refuses', demonstrations()),
      ui.block('Where it goes wrong', ui.list(layer.caveats)),
      ui.block(
        'Where this came from',
        ui.group(
          'the acceptance criteria this layer is named by',
          ui.table({
            columns: ['#', 'requirement', 'status', 'fixture'],
            rows: data
              .criteria()
              .filter((row) => layer.criteria.includes(row.id))
              .map((row) => [
                row.id,
                row.requirement,
                row.status === 'passed' ? ui.badge('passed', 'ok') : ui.badge(row.status, 'bad'),
                row.fixture,
              ]),
            dense: true,
            caption:
              'Criteria 4, 6 and 10 fall in this layer and do not pass; the console reports them rather than presenting them as passing.',
          }),
        ),
        ui.sources(layer.sources),
        ui.sources([
          { label: 'the gene tables and the refusals', path: 'src/data/genome.js' },
          { label: 'the operators and the front', path: 'src/engine/genome.js' },
        ]),
        ui.note(
          `${GENES().structural.length} structural genes and ${GENES().parameters.length} parameter genes, plus a gene per condition leaf; the frozen objects are ${frozenFields().length}, and ${new Set(policy.sets.families).size} families are declared.`,
          'muted',
        ),
      ),
    ),
  );
  showDemonstration();
  render();
}
