/**
 * The search slice's behaviour: the genotype, the operators, the equivalence rule and the front.
 *
 * Every assertion here is a rule the design states and the repository tests, expressed against the
 * console's own model. If one broke, the search would move a position it does not have, count two
 * trials as one, or keep a candidate the front drops - which is what these tests are for.
 */

import { describe, expect, test } from 'bun:test';

import {
  DEGREES_OF_FREEDOM,
  GENES,
  amend,
  crossover,
  deduplicate,
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
  selectable,
} from '../src/engine/genome.js';
import {
  demoPolicy,
  frozenFields,
  ignoredPaths,
  integerBounds,
  objectives,
  searchConstants,
  searchPolicy,
} from '../src/data/genome.js';

/** A population the same seed always reproduces, which is what the search's determinism means. */
function drawPopulation(seed, count = 5) {
  const rng = rngFor(seed, 3);
  return Array.from({ length: count }, () => makeGenome(rng, demoPolicy(), { seed }));
}

/** Declared development results over a population, with distinct values on every objective. */
function resultsFor(population, offset = 0) {
  return population.map((genome, index) => ({
    candidateId: `candidate-${index}`,
    digest: digestOfSpecification(genome),
    family: genome.identity.family,
    partition: 'development',
    role: 'development',
    objectives: Object.fromEntries(
      objectives().map((objective, position) => [
        objective.id,
        position + index + offset * (position + 1),
      ]),
    ),
  }));
}

describe('the genotype', () => {
  test('the genotype carries every declared position, ordered by path', () => {
    // If a position went missing, a mutation would have nothing to move where the policy allows one.
    const tables = GENES();
    const genes = genesOf(drawPopulation(11)[0], demoPolicy());
    const paths = genes.map((gene) => gene.path);
    expect(paths).toEqual([...paths].sort());
    expect(genes.filter((gene) => gene.degree === 'structural')).toHaveLength(17);
    expect(genes.filter((gene) => gene.degree === 'parameter')).toHaveLength(
      tables.parameters.length + 2,
    );
    expect(new Set(paths).size).toBe(paths.length);
    expect(genes.map((gene) => gene.degree).every((degree) => ['parameter', 'structural'].includes(degree))).toBe(
      true,
    );
  });

  test('a range bound offers no discrete alternative, and a declared set excludes the value held', () => {
    // If a range bound enumerated alternatives, a mutation would step through a list rather than
    // draw inside the interval the policy declared.
    const genome = drawPopulation(11)[0];
    const genes = genesOf(genome, demoPolicy());
    const ranged = genes.find((gene) => gene.degree === 'parameter');
    expect(ranged.choices).toEqual([]);
    const switched = genes.find((gene) => gene.path === 'data.decision_time');
    expect(switched.declared).toEqual(['session_close', 'session_open']);
    expect(switched.choices).toEqual(switched.declared.filter((value) => value !== switched.value));
  });

  test('a drawn genome keeps every value inside its bound and inside its declared set', () => {
    // A value outside its bound would mean the search bred a candidate the compiler could not admit.
    const policy = demoPolicy();
    const genome = drawPopulation(4)[0];
    for (const gene of genesOf(genome, policy)) {
      if (gene.bound) {
        const lower = gene.bound.lower ?? 0;
        const upper = gene.bound.upper ?? lower;
        expect(gene.value).toBeGreaterThanOrEqual(lower);
        expect(gene.value).toBeLessThanOrEqual(upper);
        if (integerBounds().includes(gene.name)) expect(Number.isInteger(gene.value)).toBe(true);
      } else if (gene.declared.length > 0) {
        expect(gene.declared).toContain(gene.value);
      } else {
        expect(typeof gene.value).toBe('boolean');
      }
    }
  });

  test('the three degrees of freedom are recorded, and only the family degree needs new statements', () => {
    // A parameter or structural change that demanded a new hypothesis would make every edit a
    // mechanism change, which is the thing the third degree exists to separate.
    expect(DEGREES_OF_FREEDOM.map((degree) => degree.id)).toEqual([
      'parameter',
      'structural',
      'family',
    ]);
    expect(DEGREES_OF_FREEDOM.filter((degree) => degree.requiresNewHypothesis)).toEqual([
      { id: 'family', requiresNewHypothesis: true },
    ]);
  });
});

describe('the seeded run', () => {
  test('a seed and a generation fix the whole run, twice over', () => {
    // If two runs with one seed differed, a campaign could not be replayed and no record of it
    // would be worth keeping.
    const first = drawPopulation(7).map(digestOfSpecification);
    const second = drawPopulation(7).map(digestOfSpecification);
    expect(first).toEqual(second);

    const results = resultsFor(drawPopulation(7));
    const breed = () =>
      nextGeneration({
        population: drawPopulation(7),
        results,
        generation: 3,
        policy: demoPolicy(),
        searchPolicy: searchPolicy(),
        seed: 7,
      }).population.map(digestOfSpecification);
    expect(breed()).toEqual(breed());
  });

  test('a different seed or a different generation moves the run', () => {
    // If a second seed drew the same values, the search would be a single population repeated.
    const other = drawPopulation(8).map(digestOfSpecification);
    expect(other).not.toEqual(drawPopulation(7).map(digestOfSpecification));
    const plan = nextGeneration({
      population: drawPopulation(7),
      results: resultsFor(drawPopulation(7)),
      generation: 3,
      policy: demoPolicy(),
      searchPolicy: searchPolicy(),
      seed: 7,
    });
    const later = nextGeneration({
      population: drawPopulation(7),
      results: resultsFor(drawPopulation(7)),
      generation: 4,
      policy: demoPolicy(),
      searchPolicy: searchPolicy(),
      seed: 7,
    });
    expect(plan.population.map(digestOfSpecification)).not.toEqual(
      later.population.map(digestOfSpecification),
    );
  });
});

describe('the mutation operator', () => {
  test('a mutation writes inside the bound and keeps the declared type', () => {
    // A lookback drawn as a decimal would be a candidate the schema refuses; the draw has to keep
    // the type the field declares.
    const genome = drawPopulation(5)[0];
    const policy = demoPolicy();
    const lookback = `features.declarations[0].lookback`;
    const edit = mutate(genome, policy, { paths: [lookback], rng: rngFor(5, 9) });
    expect(edit.ok).toBe(true);
    expect(edit.changes).toHaveLength(1);
    expect(Number.isInteger(edit.changes[0].after)).toBe(true);
    const bound = policy.bounds.find((item) => item.name === 'feature.lookback');
    expect(edit.changes[0].after).toBeGreaterThanOrEqual(bound.lower);
    expect(edit.changes[0].after).toBeLessThanOrEqual(bound.upper);
    expect(edit.changes[0].before).toBe(genesOf(genome, policy).find((gene) => gene.path === lookback).value);

    const weight = mutate(genome, policy, {
      paths: ['portfolio.max_position_weight'],
      rng: rngFor(5, 10),
    });
    expect(Number.isInteger(weight.changes[0].after)).toBe(false);
  });

  test('a mutation refuses a path that is not a gene, a statement path, and an empty selection', () => {
    // The three refusals are what tell a caller which of its arguments stopped the edit.
    const genome = drawPopulation(5)[0];
    const policy = demoPolicy();
    const rng = rngFor(5, 2);
    expect(mutate(genome, policy, { paths: ['entry.conditions.nope'], rng })).toEqual({
      ok: false,
      code: 'gene_unresolved',
      reason: 'entry.conditions.nope is not a gene',
    });
    expect(mutate(genome, policy, { paths: ['identity.hypothesis'], rng })).toEqual({
      ok: false,
      code: 'gene_not_mutable',
      reason: 'identity.hypothesis replaces the mechanism, which needs a replacement statement',
    });
    expect(mutate(genome, policy, { paths: [], rng })).toEqual({
      ok: false,
      code: 'gene_unresolved',
      reason: 'no gene was named',
    });
  });

  test('a mutation refuses a gene whose declared set holds nothing else', () => {
    // The first-release policy declares one signal frequency, so that gene is pinned; a search that
    // overwrote it anyway would move a value the policy fixed.
    const genome = drawPopulation(5)[0];
    const policy = demoPolicy();
    expect(policy.sets.signal_frequencies).toEqual(['daily']);
    expect(mutate(genome, policy, { paths: ['data.signal_frequency'], rng: rngFor(5, 3) })).toEqual({
      ok: false,
      code: 'no_free_gene',
      reason: 'data.signal_frequency has no other admissible value',
    });
    const degenerate = {
      ...policy,
      bounds: policy.bounds.map((bound) =>
        bound.name === 'selection.percentile' ? { ...bound, lower: 0.5, upper: 0.5 } : bound,
      ),
    };
    expect(
      mutate(genome, degenerate, { paths: ['selection.percentile'], rng: rngFor(5, 4) }).code,
    ).toBe('no_free_gene');
  });

  test('a choice bound draws one of its own alternatives rather than repeating the value held', () => {
    // A gene with a choice bound has no interval to draw inside, so the draw comes from its list.
    const policy = demoPolicy();
    const withChoice = {
      ...policy,
      bounds: policy.bounds.map((bound) =>
        bound.name === 'selection.count'
          ? { kind: 'choice', name: bound.name, lower: null, upper: null, choices: [3, 7, 11] }
          : bound,
      ),
    };
    const genome = drawPopulation(31)[0];
    const edit = mutate(genome, withChoice, { paths: ['selection.count'], rng: rngFor(31, 1) });
    expect(edit.ok).toBe(true);
    expect([3, 7, 11]).toContain(edit.changes[0].after);
    expect(edit.changes[0].after).not.toBe(edit.changes[0].before);
  });

  test('every named path is resolved before anything is drawn', () => {
    // A half-applied edit would leave a genome carrying a change nobody asked for.
    const genome = drawPopulation(5)[0];
    const policy = demoPolicy();
    const before = JSON.stringify(genome);
    const edit = mutate(genome, policy, {
      paths: ['selection.count', 'identity.mechanism'],
      rng: rngFor(5, 6),
    });
    expect(edit.ok).toBe(false);
    expect(JSON.stringify(genome)).toBe(before);
  });
});

describe('the recombination and mechanism operators', () => {
  test('recombination exchanges a block of compatible values between two parents of one family', () => {
    // The exchange is a block rather than a coin flip per gene, so a child carries the donor's
    // values from one position onward and nothing else.
    const [first, second] = drawPopulation(9, 2);
    const policy = demoPolicy();
    const edit = crossover(first, second, policy, { rng: rngFor(9, 7) });
    expect(edit.ok).toBe(true);
    expect(edit.changes.length).toBeGreaterThan(0);
    const donors = new Map(genesOf(second, policy).map((gene) => [gene.path, gene.value]));
    for (const change of edit.changes) {
      expect(change.after).toBe(donors.get(change.path));
      expect(change.before).not.toBe(change.after);
    }
    const split = genesOf(first, policy).findIndex(
      (gene) => gene.path === edit.changes[0].path,
    );
    for (const gene of genesOf(first, policy).slice(0, split)) {
      expect(edit.changes.some((change) => change.path === gene.path)).toBe(false);
    }
  });

  test('recombination refuses two families and two parents that share no admissible difference', () => {
    // Crossing one family's entry with another's exit produces a specification with no mechanism
    // behind it, and identical parents have nothing to exchange.
    const [first, second] = drawPopulation(9, 2);
    const policy = demoPolicy();
    const moved = replaceMechanism(
      first,
      { family: 'static_hold', mechanism: 'm', hypothesis: 'h', falsification: 'f' },
      policy,
    ).genome;
    expect(crossover(first, moved, policy, { rng: rngFor(9, 2) })).toEqual({
      ok: false,
      code: 'family_mismatch',
      reason: `${first.identity.family} and static_hold are different families`,
    });
    expect(crossover(first, first, policy, { rng: rngFor(9, 2) })).toEqual({
      ok: false,
      code: 'no_free_gene',
      reason: 'the parents share no admissible difference',
    });
  });

  test('replacing the mechanism carries the statements with it and records the candidate replaced', () => {
    // This is the only operation that may change a mechanism, and it takes the replacement
    // statements by requirement rather than keeping the old ones.
    const genome = drawPopulation(3)[0];
    const policy = demoPolicy();
    const replacement = {
      family: 'static_hold',
      mechanism: 'a mechanism',
      hypothesis: 'a hypothesis',
      falsification: 'a falsification',
    };
    const edit = replaceMechanism(genome, replacement, policy);
    expect(edit.ok).toBe(true);
    expect(edit.changes.every((change) => change.degree === 'family')).toBe(true);
    expect(edit.changes.map((change) => change.path)).toEqual([
      'identity.family',
      'identity.mechanism',
      'identity.hypothesis',
      'identity.falsification',
    ]);
    expect(mechanismOf(edit.genome)).toEqual(replacement);
    expect(edit.genome.provenance.parents).toEqual([digestOfSpecification(genome)]);
  });

  test('replacing the mechanism refuses a blank statement, an undeclared family, then no change', () => {
    // The order matters: a blank statement is a missing argument, an undeclared family is a value
    // outside the policy, and only a complete replacement can be the one already carried.
    const genome = drawPopulation(3)[0];
    const policy = demoPolicy();
    const held = mechanismOf(genome);
    expect(
      replaceMechanism(genome, { ...held, hypothesis: '  ' }, policy),
    ).toEqual({
      ok: false,
      code: 'value_not_declared',
      reason: 'the replacement statement is missing hypothesis',
    });
    expect(replaceMechanism(genome, { ...held, family: 'invented_family' }, policy)).toEqual({
      ok: false,
      code: 'value_not_declared',
      reason: 'invented_family is not a declared family',
    });
    expect(replaceMechanism(genome, held, policy)).toEqual({
      ok: false,
      code: 'no_free_gene',
      reason: 'the replacement mechanism is the one the genome already carries',
    });
  });
});

describe('the equivalence rule', () => {
  test('a rewritten statement and different provenance are the same configuration', () => {
    // The rule is pre-registered so that a rerun or a recompilation is one trial rather than
    // several; if these merged differently the trial count would be wrong.
    const genome = drawPopulation(21)[0];
    const rewritten = {
      ...genome,
      identity: { ...genome.identity, hypothesis: 'rewritten', falsification: 'rewritten too' },
      provenance: { ...genome.provenance, parents: ['someone'], seed: 99 },
    };
    expect(equivalenceKey(rewritten)).toBe(equivalenceKey(genome));
    expect(digestOfSpecification(rewritten)).not.toBe(digestOfSpecification(genome));
    expect(ignoredPaths()).toHaveLength(5);
  });

  test('a counted change is a different configuration, and the first of two survives', () => {
    // A parameter move is not an ignored path, so a setup cannot become several trials by editing
    // wording, and two candidates with one key keep the first.
    const genome = drawPopulation(21)[1];
    const counted = { ...genome, selection: { ...genome.selection, count: genome.selection.count + 1 } };
    expect(equivalenceKey(counted)).not.toBe(equivalenceKey(genome));
    const folded = deduplicate([genome, counted, { ...genome, provenance: { ...genome.provenance, seed: 5 } }]);
    expect(folded.survivors.map((item) => item.selection.count)).toEqual([
      genome.selection.count,
      counted.selection.count,
    ]);
    expect(folded.merges).toHaveLength(1);
    expect(folded.merges[0].mergedInto).toBe(digestOfSpecification(genome));
    expect(folded.merges[0].rule).toBe(searchConstants().equivalenceRuleId);
  });
});

describe('selection and the front', () => {
  test('a front keeps a tie and drops a candidate another beats everywhere', () => {
    // A front that kept a dominated candidate would report an option the measurements exclude.
    const results = [
      { candidateId: 'a', objectives: { risk_adjusted_return: 1, drawdown: 1 } },
      { candidateId: 'b', objectives: { risk_adjusted_return: 1, drawdown: 1 } },
      { candidateId: 'c', objectives: { risk_adjusted_return: 0, drawdown: 2 } },
    ];
    const kept = ['risk_adjusted_return', 'drawdown'];
    expect(paretoFront(results, { objectives: kept }).map((row) => row.candidateId)).toEqual([
      'a',
      'b',
    ]);
    expect(dominates(results[0], results[2], kept)).toBe(true);
    expect(dominates(results[0], results[1], kept)).toBe(false);
  });

  test('a result missing a kept objective is not comparable in either direction', () => {
    // An incomplete result stays in the inventory rather than being selected against or for.
    const kept = ['risk_adjusted_return', 'drawdown'];
    const partial = { candidateId: 'd', objectives: { drawdown: 1 } };
    const complete = { candidateId: 'e', objectives: { risk_adjusted_return: 0, drawdown: 1 } };
    expect(dominates(partial, complete, kept)).toBe(false);
    expect(dominates(complete, partial, kept)).toBe(false);
    expect(paretoFront([partial, complete], { objectives: kept }).map((row) => row.candidateId)).toEqual([
      'e',
    ]);
  });

  test('selection reads development results only, and names what it refused', () => {
    // A locked result reaching selection would let the search read the data it is held back from.
    const locked = [
      { candidateId: 'locked-one', role: 'locked_validation', objectives: {} },
      { candidateId: 'planned', role: 'development', objectives: {} },
    ];
    expect(selectable(locked)).toEqual({
      ok: false,
      code: 'locked_result_in_selection',
      reason:
        'locked-one carries locked_validation data, and selection reads development results only',
    });
    expect(selectable([locked[1]]).ok).toBe(true);
  });
});

describe('a generation', () => {
  test('an empty population is refused before any breeding happens', () => {
    // Breeding from nothing would report children nobody asked for.
    const plan = nextGeneration({
      population: [],
      results: [],
      generation: 3,
      policy: demoPolicy(),
      searchPolicy: searchPolicy(),
      seed: 7,
    });
    expect(plan.refusal.code).toBe('population_empty');
    expect(plan.population).toEqual([]);
  });

  test('a population whose results are all incomplete is refused as missing an objective', () => {
    // A front cannot rank candidates nobody measured, and the plan says which failure it was.
    const population = drawPopulation(13);
    const results = resultsFor(population).map((result) => ({ ...result, objectives: {} }));
    const plan = nextGeneration({
      population,
      results,
      generation: 3,
      policy: demoPolicy(),
      searchPolicy: searchPolicy(),
      seed: 13,
    });
    expect(plan.refusal).toEqual({
      code: 'missing_objective',
      reason: 'no candidate produced a complete set of development objectives',
    });
  });

  test('a generation breeds one child per place and reports the degrees it exercised', () => {
    // The plan is what tells a reader whether the generation exploited, explored, or fell over.
    const population = drawPopulation(17);
    const results = resultsFor(population);
    const plan = nextGeneration({
      population,
      results,
      generation: 3,
      policy: demoPolicy(),
      searchPolicy: searchPolicy(),
      seed: 17,
    });
    expect(plan.refusal).toBe(null);
    expect(plan.population).toHaveLength(searchConstants().populationSize);
    expect(plan.front.length).toBeGreaterThan(0);
    expect(plan.edits).toHaveLength(searchConstants().populationSize);
    const degrees = new Set(plan.edits.flatMap((edit) => edit.changes.map((change) => change.degree)));
    expect([...degrees].sort()).toEqual(['parameter', 'structural']);
    const parents = new Set(population.map(digestOfSpecification));
    for (const child of plan.population) {
      expect(parents.has(digestOfSpecification(child))).toBe(false);
    }
  });

  test('a generation refuses a plan whose counts are not at least one', () => {
    // A population size of zero would silently breed nothing and report success.
    expect(() =>
      nextGeneration({
        population: drawPopulation(17),
        results: resultsFor(drawPopulation(17)),
        generation: 3,
        policy: demoPolicy(),
        searchPolicy: searchPolicy(),
        seed: 17,
        elite: 0,
      }),
    ).toThrow('elite must be at least one, was 0');
  });

  test('an amendment refuses a frozen field and an undeclared one, and advances the stamp', () => {
    // The frozen set is what stops the search moving the objects the design fixed; the stamp is not
    // an instruction and advances without being refused.
    const frozen = searchPolicy();
    expect(frozenFields().map((row) => row.field)).toHaveLength(8);
    const refused = amend(frozen, { bounds: 1 });
    expect(refused.code).toBe('frozen_field');
    expect(refused.reason).toBe(
      'bounds is frozen: a parameter bound is the space the search moves inside, not part of the search',
    );
    expect(amend(frozen, { nonsense: 1 }).reason).toBe(
      "nonsense is not part of the search policy's mutable set",
    );
    const amended = amend(frozen, { generation: 4, mutation_rate: 0.9 });
    expect(amended.ok).toBe(true);
    expect(amended.policy.generation).toBe(4);
    expect(amended.policy.mutation_rate).toBe(0.9);
  });
});
