# Searching for strategies by breeding candidates

Date: 2026-10-16. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| What it trades            | Nothing is bought or sold: the part acts on candidate strategies held as data, turning each into a set of genes and breeding new candidates from the ones that survive                                                                                                               |
| How often it trades       | Once per generation: a generation reads the current population and its development results and returns the next population, rather than running on any fixed clock                                                                                                                   |
| What you need             | Nothing but this page                                                                                                                                                                                                                                                                |
| Where the rules come from | [The genotype and its operators](../../../python/nautilus_trader/discovery/genome.py) and [the search rules](../../../python/nautilus_trader/discovery/evolve.py)                                                                                                                    |
| The underlying research   | No paper behind it: the rules are this repository's own design, [strategy_discovery_design.md](../../../docs/design/strategy_discovery_design.md), Section 10.3                                                                                                                      |
| How well it held up       | Mixed: the three degrees of freedom, the seeded determinism, the refusal paths and the equivalence rule are unit-tested, but the design states plainly that Pareto selection makes trade-offs visible and corrects for nothing, so a front is a set of options rather than a finding |
| Also appears in           | [Sweeping settings](../../research-process/sweeping-settings/README.md), on searching a grid of settings, and [Scoring what survived](../../research-process/scoring-what-survived/README.md), on judging what a search keeps                                                        |

## The idea in one paragraph

A candidate strategy here is held as data rather than as code: a fixed set of blocks whose values can
be read, copied and checked without running anything. The search treats that data as a genome, a set
of named positions each holding one value, with the range or the list of alternatives the policy
allows. It breeds new candidates two ways: a mutation changes values in place, and a recombination
exchanges a block of values between two parents of the same family, the same economic idea. Every
draw comes from a generator seeded with the campaign seed and the generation number, so the same pair
reproduces the same children. A generation ranks candidates on several named measurements at once and
keeps the ones no other candidate beats on every measurement, rather than blending them into one
number. The children are compiled into plans, each carrying a fingerprint, and both accepted plans
and refused proposals are recorded, though the search never runs a backtest and never sees a result.

## Why anyone believed it

Searching a large space of settings will always turn up one that looks good on the data it was
searched on, because the best of many tries is by definition high. That is data snooping, and the
only defence is an honest account of how wide the search was. Breeding is attractive because it
searches widely while keeping every candidate readable: each child is a copy of a parent with a
handful of named changes, so a reader sees what was tried without reading code. Named measurements
keep a search from collapsing into one blended number whose weights nobody can inspect, and the
equivalence rule stops one setup being counted as many trials because it was renamed or rerun. Seeded
draws make the search itself replayable. There is no counterparty story here, because nothing is
bought or sold; what could mislead a reader is the record of what was tried.

## An everyday comparison

A farm's plant-breeding programme behaves this way. Each variety is a card of traits: leaf size,
ripening week, stalk height. Rather than inventing varieties from nothing, breeders cross two parents
of the same kind and keep the cards that survive a trial field. A value is nudged inside a stated
range, one whole trait is swapped for another value the catalogue offers, or the kind of plant itself
is replaced, which needs a new written description of what is being bred. The varieties no worse than
the rest on every measured trait at once are kept, so one that ripens late but resists disease is not
thrown away for being slow. The trial field judges the variety; it does not create the harvest.

## The rules, step by step

1. Read a candidate as data and list its positions. Every addressable value is a gene, named by the
   path it occupies; a gene carries its value, its kind, and the range or list of alternatives the
   policy allows. A feature cannot be defined, only selected, and a selected feature's lookback and
   skip are genes.
2. Separate the genes by kind, the degree of freedom. A parameter gene holds a number inside one
   hypothesis; a structural gene picks one alternative from a declared list, changing the logic while
   the mechanism stands; a family gene replaces the mechanism and no draw can reach it.
3. Replace a mechanism only with a new statement. `with_mechanism` requires the replacement
   mechanism, hypothesis and falsification statement as arguments, and refuses a replacement missing
   any of them or naming an undeclared family.
4. Mutate. A parameter mutation draws up to two parameter paths and a structural mutation up to one
   structural path; a drawn value must sit inside its bound or come from the declared list. The
   operation applies every change or refuses the whole edit, naming the path that stopped it.
5. Recombine. With more than one parent of the same family and the draw permitting, recombination
   takes two parents and one split position, drawn only from positions where they differ and where
   the receiving gene admits the donor value, and every shared gene from there on takes the donor's
   value. Two families, or two parents with no admissible difference, are refused.
6. Rank the results. One candidate dominates another when it is no worse on every named measurement
   and strictly better on at least one. The undominated candidates form the Pareto front.
7. Read only development data. Selection refuses a result whose partition was not development data
   for that lineage, so a fold that has already informed the search cannot be used again as if new.
8. Fold duplicates. Two candidates with the same equivalence key are one configuration: the first
   survives and the later is recorded as a merge.
9. Compile the survivors. Each surviving child becomes a compiled plan reporting the specification's
   fingerprint and a second, behavioural fingerprint over the specification, the policy, the feature
   registry, the resolved features, the assumptions and the engine version.
10. Change only the search's own instructions. A generation may change its prompts, its mutation and
    recombination operators, its exploration balance and its allocation of effort across families.
    It may not change the eight frozen fields: the allowed feature set, any parameter bound, the
    evaluation window, the split layout, the cost model, the benchmark, the risk policy, and the
    allocation of the confirmation set; an amendment to one is refused by name, and every generation
    stores the digest of the policy it ran under.

## The maths, with every symbol named

The selection rule is dominance, stated for one candidate against another:

```text
A dominates B when, for every objective o, A's value is at least as good as B's,
and for at least one objective o, A's value is strictly better than B's.
```

- `A` and `B` are candidates carrying a complete set of objectives, and `o` ranges over the seven
  named objectives below; "at least as good" means larger when the objective prefers high and
  smaller when it prefers low.
- A candidate missing any objective is not comparable and is left out of the front, though it stays
  in the inventory, where the multiplicity it contributes is counted rather than dropped.

The seven objectives, each with the sampling convention its name carries:

| Objective            | Convention (what is measured)                                                               | Prefers |
| -------------------- | ------------------------------------------------------------------------------------------- | ------- |
| risk-adjusted return | the per-period Sharpe ratio of the development folds' return series at the signal frequency | high    |
| return dispersion    | the standard deviation of the per-fold risk-adjusted return across the folds                | low     |
| drawdown             | the maximum drawdown of the development window                                              | low     |
| tail loss            | the fifth percentile of the development per-period returns                                  | high    |
| relative return      | the development return less the registered benchmark's over the same window                 | high    |
| turnover             | turnover under the policy's declared cost model                                             | low     |
| complexity           | the condition-leaf count plus the declared parameter count                                  | low     |

- A Sharpe ratio is a return above the safe rate divided by how much the return swings around its
  average; a drawdown is a peak-to-low fall as a share of the peak; turnover is how often holdings
  change.

The seeded generator and the breed mix, which together say how a child is produced:

```text
rng = Random(seed + ":" + generation)
explore = rng.random() < exploration_share
parameter mutation draws up to 2 parameter paths
structural mutation draws up to 1 structural path
recombination is taken when the parents number more than one and rng.random() >= mutation_rate
```

- `seed` is the campaign's declared whole number and `generation` the generation number as text, fed
  to the standard library's seeded generator so the same pair draws the same sequence.
- `exploration_share` is the share of children bred structurally and `mutation_rate` the share bred
  by mutation rather than recombination; the defaults are 0.25 and 0.5, and an edit that could not
  produce a valid genome is retried up to 8 times before the plan reports the refusal.

The equivalence rule, which is what stops one setup becoming many trials:

```text
equivalence_key = SHA-256 of the candidate's canonical text with these five paths removed:
                  identity.hypothesis, identity.falsification, provenance.parents,
                  provenance.generator, provenance.seed
```

- The canonical text is the candidate written as sorted, compact text; removing those five paths
  means a renamed hypothesis, a rewritten statement or different provenance does not by itself make a
  new configuration, while any other change does.

## A worked example

One parent candidate is carried through one generation into a compiled plan. Every number is made up
but of the size these values take.

Parent P, family `cross-sectional-momentum`, read against a policy whose bounds are shown:

| Gene path                         | Value | Degree     | Bound        |
| --------------------------------- | ----- | ---------- | ------------ |
| features.declarations[0].lookback | 20    | parameter  | 5 to 60      |
| selection.percentile              | 10    | parameter  | 1 to 50      |
| portfolio.max_position_weight     | 0.10  | parameter  | 0.01 to 0.25 |
| entry.comparison                  | gt    | structural | declared     |
| data.signal_frequency             | daily | structural | declared     |
| portfolio.weighting               | equal | structural | declared     |

The generation is number 3 and the seed is 7, so the generator is `Random("7:3")`. The first draw is
0.62, not below the exploration share of 0.25, so the child exploits: a parameter mutation. The two
parameter paths drawn are `features.declarations[0].lookback` and `portfolio.max_position_weight`;
the lookback is drawn as 25, inside 5 to 60, and the weight as 0.13, inside 0.01 to 0.25.

| Step | What happens        | Value                                                                |
| ---- | ------------------- | -------------------------------------------------------------------- |
| 1    | parent fingerprint  | the digest of P's canonical text                                     |
| 2    | first gene changed  | features.declarations[0].lookback: 20 becomes 25                     |
| 3    | second gene changed | portfolio.max_position_weight: 0.10 becomes 0.13                     |
| 4    | child fingerprint   | different from the parent's, because a gene changed                  |
| 5    | child is compiled   | accepted, with its specification fingerprint equal to the child's    |
| 6    | equivalence key     | different from the parent's, because a parameter path is not ignored |

The digest is the fingerprint of the canonical text, so the same text always gives the same string of
hexadecimal digits and any change gives a different one. The digits below are the true SHA-256 of the
two-gene stand-in beside them; a real digest covers the whole candidate.

```text
text A    {"features.declarations[0].lookback":20,"portfolio.max_position_weight":"0.10"}
digest A  sha256:47a25dd4946806baa16a53f670ca94cd992d6315a81cb86c61394dfd494f3ba6
text B    {"features.declarations[0].lookback":25,"portfolio.max_position_weight":"0.13"}
digest B  sha256:101643d4e6dbe396724a96a3b45c1010cd3b2c8332500b3756d38301123904b3
```

So the child's `specification_digest` is digest B rather than digest A, and the compiled plan reports
that value beside a `behavioural_digest` built from six inputs: the specification fingerprint, the
policy fingerprint, the registry fingerprint, the resolved features, the timing assumptions and the
engine version. Because `features.declarations[0].lookback` is not one of the five ignored paths, the
child is counted as a genuinely different configuration; a candidate differing from P only in
`identity.hypothesis` and `provenance.parents` would carry P's key and be merged into P instead.

## What the research actually found

This page is about machinery, so what is measured is the repository's own tests, not a market. The
genotype unit tests assert that a mutation is deterministic under its seed, that a mutation refuses a
path it cannot resolve, a mechanism path, and a gene with no other admissible value, that
`with_mechanism` refuses the mechanism the genome already carries, that recombination refuses two
families and two identical parents, and that a mutated genome still compiles with its digest
reported. The search unit tests assert that selection reads development results only, that a rewritten
statement and different provenance are the same configuration while a parameter change is a different
configuration, that the first of two equivalent candidates survives and the merge is recorded, that an
amendment refuses a frozen field and an undeclared one by name, and that a generation breeds children
under its policy and reports its front and the degrees it exercised. The integration fixture shows a
partition that informed one lineage becoming development data for it, while a second lineage keeps it.

The design is explicit about what this machinery does not do. Section 7.5 states that Pareto selection
makes trade-offs visible and preserves diversity; it does not correct for multiple testing, protect a
confirmation set, or establish an edge. The implementation plan carries that finding rather than
repairing it, and names two further limits: the three degrees are enforced by construction rather
than by review, and the per-period Sharpe ratios an attempt produced are declared when the attempt
closes and read back from the inventory, while the raw return series stays with the run's own
document, which this Python layer cannot read back. Nothing here measures what any strategy earns on
any market; it measures whether the breeding rules do what they say.

## How this project relates to it

The genotype lives in [genome.py](../../../python/nautilus_trader/discovery/genome.py): the genes, the
three degrees of freedom, `mutate`, `with_mechanism` and `recombine`, and the refusal each returns.
The search lives in [evolve.py](../../../python/nautilus_trader/discovery/evolve.py): the seven
objectives with their conventions, the Pareto front, the equivalence key and deduplication, and the
search policy's named mutable set. An accepted child becomes a compiled plan in
[compile.py](../../../python/nautilus_trader/discovery/compile.py), where the two fingerprints are
built. A reader who does read code can run the two suites:

```bash
python -m pytest python/tests/unit/discovery/test_genome.py python/tests/unit/discovery/test_evolve.py
```

The fixture behind the partition rule is
[test_discovery_search.py](../../../python/tests/integration/test_discovery_search.py). The group's
[index](../README.md) states the four artifacts these files belong to. The frozen field set is stated
here; what a policy is, and which thresholds it carries, belongs to the group's page on the admission
gate, stated in the design [Section 11.2](../../../docs/design/strategy_discovery_design.md). The
trial counts belong to the group's page on counting the search, the identities and digests to
[what ran against what was written](../what-ran-versus-what-was-written/README.md), and the agents
that propose candidates to [agents that propose](../agents-that-propose/README.md).

## Where it goes wrong

- A front is not a finding. The design says Pareto selection corrects for multiple testing, for a
  confirmation set and for an edge, none of which it does; a kept set is a set of options, and the
  decision to keep looking is a human one.
- An objective is only as good as its stated convention. A risk-adjusted return at a daily frequency
  and the same name at a monthly one are different quantities, so a comparison across conventions is
  meaningless.
- The count is only as good as the equivalence rule. If the rule's ignored paths are wrong, a
  genuinely distinct configuration can merge into another, or one setup can be counted as several;
  the rule is pre-registered for exactly that reason.
- Diversity has to be defended, or a search collapses into near-identical variants of one idea; the
  plan's survivor rule keeps each family on the front represented.
- The search reads declared numbers, so the correction that consumes the per-period Sharpe ratios
  depends on the attempt's own record rather than on a recomputation from the raw series.

## Try it yourself

You need a table of five made-up candidates and no code. The exercise shows why a kept set is a set
of options rather than a winner.

1. Rule a table with four columns: candidate, return (a percentage, higher is better), turnover (a
   percentage, lower is better), and drawdown (a percentage, lower is better).
2. Fill five rows with plausible numbers, for example returns 4, 6, 5, 3, 7 and turnovers 20, 80, 30,
   15, 90 and drawdowns 10, 25, 12, 8, 30.
3. Mark every candidate that no other candidate beats on all three columns at once, then erase the
   turnover column and mark again, writing down which candidates enter the kept set and which leave.

What to notice: no candidate is "the best". The survivors trade one good column against another, and
a single blended score would have hidden exactly the candidates strong on one column and weak on
another. Adding or removing one column changes who survives, which is why the objectives have to be
named before the search runs.

## Where this came from

- [The genotype module](../../../python/nautilus_trader/discovery/genome.py), the genes, the three
  degrees of freedom, `mutate`, `with_mechanism` and `recombine`, and the refusals of each.
- [The search module](../../../python/nautilus_trader/discovery/evolve.py), the seven objectives with
  their conventions, the Pareto front, the equivalence key, deduplication and the mutable set.
- [The compiler](../../../python/nautilus_trader/discovery/compile.py), where an accepted
  specification becomes a compiled plan with its specification and behavioural digests.
- [The genotype unit tests](../../../python/tests/unit/discovery/test_genome.py) and [the search unit
  tests](../../../python/tests/unit/discovery/test_evolve.py), the tests named above.
- [The search integration fixture](../../../python/tests/integration/test_discovery_search.py), which
  exercises the partition role and the refusals.
- [The design document](../../../docs/design/strategy_discovery_design.md), Sections 10.3, 10.4, 10.6
  and 7.5, and criterion 17 in Section 15.
- [The implementation plan](../../../docs/design/strategy_discovery_implementation.md), Section 11
  (M5) for the degree enforcement, the equivalence rule and the carried limits.

## Words used in this tutorial

- genome: a candidate strategy read as a set of named positions each holding a value.
- gene: one addressable position in a genome, with its path, its value and its allowed range or list.
- degree of freedom: the kind of change a gene carries, being a parameter value, a structural switch
  or a mechanism replacement.
- mutation: an operation that draws new values for named genes from a seeded generator, or refuses.
- recombination: an operation that swaps a block of genes between two parents of the same family.
- Pareto front: the set of candidates no other candidate beats on every named objective at once.
- equivalence key: the fingerprint that decides whether two candidates are one configuration or two.
- digest: the short hexadecimal fingerprint of a candidate's canonical text.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
