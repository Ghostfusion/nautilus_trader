# What a candidate is

Date: 2026-10-16. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                                                                                                                               |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Nothing is bought or sold: the part acts on a candidate strategy held as plain data, reading and checking it before anything runs                                                                                                                                                                                                                                                                                   |
| How often it trades       | Once per candidate, when it is compiled: the twelve blocks are read, resolved, checked and composed into one plan, and a refused candidate runs nothing                                                                                                                                                                                                                                                             |
| What you need             | Nothing but this page; the worked example needs only a table of numbers and two stated ranges                                                                                                                                                                                                                                                                                                                       |
| Where the rules come from | [The schema and its reader](../../../python/nautilus_trader/discovery/schema.py), [the four validation stages](../../../python/nautilus_trader/discovery/validate.py) and [the compiler that composes the plan](../../../python/nautilus_trader/discovery/compile.py)                                                                                                                                               |
| The underlying research   | No paper behind it: the rules are this repository's own design, [strategy_discovery_design.md](../../../docs/design/strategy_discovery_design.md), Section 9.1 to Section 9.5                                                                                                                                                                                                                                       |
| How well it held up       | Strong as a definition: the closed schema, the resolution paths and the refusal vocabulary are unit- and integration-tested, including the tests that every dynamic import is made unavailable while a legal specification still compiles; whether a constrained language discovers better strategies than free code generation was never measured, and the design records that as unproven rather than established |
| Also appears in           | [Searching for strategies by breeding candidates](../search-by-evolution/README.md), which treats this data as a genome, and the [group index](../README.md)                                                                                                                                                                                                                                                        |

## The idea in one paragraph

A candidate strategy in this repository is a description rather than a program. It is one lump of
plain data, grouped into twelve named blocks of names, numbers, words and short statements, and each
block answers one question about the proposed strategy. Because it is only data, the whole candidate
can be read, copied, fingerprinted and turned down without any code being run. A reader, or a search
that proposes candidates, can therefore see exactly what was suggested before anything is tested.

## Why anyone believed it

Three pieces of evidence decided it, and the design states them plainly. Unrestricted generated code
in a research environment is an operational hazard with a network surface, a credentials surface and
a reproducibility problem. The closest published system to this design calls unconstrained generation
operationally unsafe and sandboxes it for the same reason. And the engine already offers a way to name
a strategy by module path, which runs arbitrary code out of a file that would then itself have to be
hashed, stored and trusted.

The counterweight is recorded: no head-to-head benchmark was found showing that a constrained language
discovers better strategies than free code generation, so "constrained beats free" is an argument
about safety and auditability and not a measured claim about discovery quality. The design records
that as unproven rather than established. What it does claim is structural: with no generated code
there is no stub to generate, no import side effect, no second execution path, and one fingerprint
that fully identifies the behaviour that was tested.

## An everyday comparison

A building-permit form behaves this way. You do not hand the clerk a model of the building; you fill
in a fixed set of boxes, and the clerk checks each box against the rulebook. If a box asks for a
number that must sit between two limits and yours sits outside them, the form comes back with that
one box named and the permitted range restated, not with a shrug. A form is something a clerk who
will never enter the building can read, file, compare with last month's forms, and check again next
year.

## The rules, step by step

A candidate is one mapping of plain data with a version line, which for this build is exactly
`discovery.specification/v1`, and twelve blocks. `schema.py` calls the twelve names `_BLOCK_NAMES`,
and each block answers one question:

| Block        | The question it answers                                                                        |
| ------------ | ---------------------------------------------------------------------------------------------- |
| `identity`   | What does the candidate claim, in the words it will be audited by?                             |
| `universe`   | Which instruments may it hold, and what floors decide membership?                              |
| `data`       | When does it decide, when may its intent fill, and how are company actions treated?            |
| `features`   | Which registered measurements does it select, and what names does the rest of it use for them? |
| `selection`  | How does it choose its eligible set, and how is a tie broken?                                  |
| `entry`      | Whether and when does it enter an eligible instrument?                                         |
| `exit`       | Which declared ways does a position leave the portfolio?                                       |
| `portfolio`  | How much does it hold, in how many positions, under which caps?                                |
| `rebalance`  | How often does it adjust the holdings, and when?                                               |
| `risk`       | Which constraints act before the trade, while it is live, and after the run?                   |
| `evaluation` | Which frozen objects is it judged against?                                                     |
| `provenance` | Where did it come from, recorded rather than trusted?                                          |

1. Read the candidate as one mapping. Nothing in it is callable: a callable, a type or a module
   carried as a value is refused at the path that carried it.
2. Run the code scan first. Before any block is read, every key and value is checked, so a field name
   that means code is refused by name wherever it is put, and a value shaped like code is refused by
   shape wherever it is put.
3. Require all twelve blocks and the version line, and nothing else. A missing block is
   `BLOCK_MISSING`; a field a block does not declare is `BLOCK_UNKNOWN_FIELD`, because the block
   refuses every key it did not read, so the allowed set can never drift from the reading.
4. Resolve every value from one of three places only. The first is the pre-registered policy's
   declared sets, bounds and frozen references. The second is the candidate's own declared feature
   aliases. The third is the run's feature stream, and only at run time, when a reading is served
   because the lookback is satisfied and its availability time is at or before the decision time.
5. The features block is read first, because selection, entry and exit refer to the aliases it
   declares. Each declared feature must be inside the policy's allowlist and must exist in the
   registry; a miss is `FEATURE_NOT_ALLOWED` or `FEATURE_UNRESOLVED`.
6. Enforce the caps. At most 8 features, at most 4 condition leaves in the entry tree, and at most 2
   levels of nesting, counted from level 1. A feature beyond the allowlist is refused whatever the
   count.
7. Check the interactions between blocks, then the times. Entry against exit, holding-period cap
   against rebalance frequency, position caps against portfolio caps, missing data against ranking
   eligibility, risk caps against target weights, rebalance against existing positions, and stop or
   target rules against what a single day's price bar can establish. Then every input must be
   knowable at the declared decision instant and every simulated fill must be eligible strictly after
   it.
8. Return an outcome rather than raise: an accepted candidate yields a plan, a refused one yields its
   refusals, each with a code, a path and a message, and the payload is never compiled. Warnings are a
   separate channel and a refusal is never softened by a warning beside it.

## The maths, with every symbol named

A field checked against a bound is admitted only by an inclusive comparison:

```text
admitted(value) = lower <= value <= upper        for a range bound
admitted(value) = value in choices               for a choice bound
```

- `value` is the number the candidate wrote, kept as an exact decimal rather than a rounded one.
- `lower` and `upper` are the two limits the policy declares, and both ends are included, so a value
  equal to either limit is admitted.
- `choices` is the declared list of allowed values, used instead of a range where the field names one
  of a fixed set.
- A value that fails the test is refused with its own path, so the refusal says which box was wrong,
  and the bound is restated in words rather than silently clipped to fit.

The condition tree is counted two ways, and both have a cap:

```text
leaves(a leaf)                = 1
leaves(an all or any group)   = the sum of leaves over its children
depth(the top of the tree)    = 1
depth(a child of a group)     = depth(that group) + 1
```

- `leaves` is the number of comparisons in the tree, so a group holding three comparisons has three
  leaves, and a group inside a group adds the inner leaves to the outer ones.
- `depth` counts levels of nesting from the top, which is level 1.
- The caps are `max_conditions` = 4, `max_condition_depth` = 2 and `max_features` = 8, and they are
  the default of the policy rather than numbers a candidate may state.

## A worked example

Six fields are submitted, one at a time, against the bounds the first-release policy declares. Every
number and range below is the real one; the candidate is the fixture's own legal payload with one
field changed.

| Field it sits at             | Value submitted                    | The rule it is checked against                   | Result                            |
| ---------------------------- | ---------------------------------- | ------------------------------------------------ | --------------------------------- |
| `universe.min_price`         | 2500                               | the range 1 to 1000 inclusive                    | refused, `value_out_of_bound`     |
| `universe.min_price`         | 5                                  | the range 1 to 1000 inclusive                    | admitted                          |
| `data.execution_eligibility` | `session_open`                     | one of `next_session_open`, `same_session_close` | refused, `value_not_declared`     |
| `provenance.module_path`     | `strategies.ema_cross:EMACross`    | no field may name code                           | refused by name                   |
| `provenance.generator`       | `strategies.ema_cross:EMACross`    | no value may take the `module:attribute` shape   | refused, `code_reference_refused` |
| `provenance.generator`       | `nautilus_trader.adapters.binance` | nothing resolves it                              | admitted as text                  |

The arithmetic behind the first two rows, worked in full:

```text
the bound universe.min_price is the range 1 to 1000 inclusive
admitted(v) is true when 1 <= v <= 1000
v = 2500: 1 <= 2500 is true, but 2500 <= 1000 is false, so the pair is false
v = 5:    1 <= 5 is true and 5 <= 1000 is true, so the pair is true

the refusal for 2500 reads, exactly:
min_price must be 1 to 1000 inclusive, was 2500
```

The third row is a set membership test rather than a comparison, and its refusal restates the
declared set in sorted order: `execution_eligibility must be one of next_session_open,
same_session_close, was session_open`. The last three rows are the two ways code is refused and the
one case that is admitted. A key named `module_path` is a request to import, so it is refused by its
name no matter what it holds. The value `strategies.ema_cross:EMACross` is the shape this repository
uses to name an importable strategy, `module:attribute`, so it is refused by its shape no matter
which field holds it. But `nautilus_trader.adapters.binance` is a bare dotted string with no colon,
so it matches no code pattern, nothing resolves it, and it is admitted inert and carried into the
plan as the text it was.

The caps are counted the same way. A tree written as one `all` group holding two comparisons and one
`any` group of three:

```text
leaves = 1 + 1 + (1 + 1 + 1) = 5, above the cap of 4, refused
depth  = the all group is level 1, its children are level 2, the any group's children are level 3
         level 3 is above the cap of 2, refused before the count is even reached
```

So a candidate may say "this feature is above this number, and that feature is above that number"
inside one group, and may nest a group inside another group only if the inner one holds no further
group. A refused payload comes back as a result whose plan is empty and whose error list holds the
refusals, each written as `code`, `path` and `message`, which is what lets a refusal survive a trip
through plain data and be recorded rather than argued about.

## What the research actually found

This page is about machinery, so what is measured is the repository's own tests rather than a market.
Criterion 3 of the design's acceptance list reads "No candidate can execute generated code", and its
bundle entry records eight tests passing in about a third of a second: a field naming a module, the
`module:attribute` value form, three expression shapes, three code artefacts, the refusal returned as
an outcome rather than raised, one compilation with every dynamic import recorded and the recording
required to be empty, the bare-dotted-string boundary, and a check that the plan names only plain
values and never an executor. Criterion 1, the round trip from a hand-written specification to a
ledger entry, also passes, and criteria 19, 20 and 21, which check the interpreter, feature parity
and allocation, pass as well.

The unit tests pin the rest of the vocabulary by name: a value outside the policy's bound is refused
with the bound stated, a bound the policy does not declare refuses every value of that field, a value
outside a declared set is refused, a feature outside the allowlist is refused, a risk field in the
wrong category is refused by category, a migration cycle is refused rather than looped, and the order
of fields does not change the canonical form. The compiler tests assert that a refused payload
returns its errors rather than raising and that a legal specification compiles with both of its
fingerprints.

What the tests do not measure is the important half. No measurement in this repository shows that a
constrained language finds better strategies than free code generation; the design states the
concession and records it as unproven rather than established. Of the twenty-two acceptance criteria,
four do not pass, none of them about this page's subject, and the group's other pages carry them
rather than bury them.

## How this project relates to it

The language lives in [schema.py](../../../python/nautilus_trader/discovery/schema.py):
`parse_specification` runs the code scan, migrates an older version or refuses it, reads the twelve
blocks, and collects every refusal, and `RefusalCode` is the closed vocabulary of 32 names. The five
stages are schema validation, feature resolution, semantic validation and temporal validation in
[validate.py](../../../python/nautilus_trader/discovery/validate.py), and trusted compilation in
[compile.py](../../../python/nautilus_trader/discovery/compile.py), where an accepted specification
becomes a `CompiledPlan`. A field's value comes from
[policy.py](../../../python/nautilus_trader/discovery/policy.py), through
[features.py](../../../python/nautilus_trader/discovery/features.py).

A reader who does read code can run the eight tests that carry criterion 3:

```bash
python -m pytest python/tests/integration/test_discovery_generated_code_refusals.py
```

The group's [index](../README.md) states the four artifacts all of these belong to.

## Where it goes wrong

- The language is closed, so a hypothesis it cannot express is refused rather than approximated. The
  design lists what it cannot say: no arbitrary code, no imports, no file or network access, no clock
  reading, no data source other than the declared snapshot, no shorting in this release, no options,
  no within-the-day price-path logic, no company accounts or news, no distribution capture and no
  discretionary rule.
- A refusal is a fact about the form, not a verdict on the idea. The bounds come from the policy, so a
  candidate refused for a value outside a range has learned something about the hypothesis only
  because the range was chosen in advance and published.
- The code scan is a pattern match over names and shapes, and the safety claim rests on resolution
  rather than on the scan alone. A bare dotted string with no colon is admitted, because nothing
  imports, evaluates or calls it; a field name that asks for a module is refused even if it holds
  harmless text.
- A risk field's name does not control behaviour. A maximum drawdown is measured after the run, so it
  is a gate condition rather than a live control, and a specification that presents it as a runtime
  control is refused; a stop is not a guaranteed loss ceiling either, because a price can gap past it
  and the fill that follows is the fill model's.
- A candidate cannot define a feature, and cannot state a significance threshold, a benchmark return
  or a cost assumption, because those live in the policy where the search cannot reach them.

## Try it yourself

You need a pen and a page, and no money and no code. The exercise is to be the clerk.

1. Draw twelve boxes and label them `identity`, `universe`, `data`, `features`, `selection`, `entry`,
   `exit`, `portfolio`, `rebalance`, `risk`, `evaluation`, `provenance`.
2. In `universe`, write a minimum share price of 2500, and in `entry` write two comparisons: one
   feature above 0, and a second feature above 0.
3. Take the bounds from the worked example: a minimum price must sit between 1 and 1000 inclusive,
   and an entry tree may hold at most 4 comparisons at no more than 2 levels of nesting.
4. Mark the first box refused and write the sentence the refusal would carry.
5. Put one `any` group of three comparisons into `entry`, beside the two you already wrote, and count
   the leaves and the levels again.

What to notice: the count of comparisons is not the count of levels, and a candidate can break one
cap while staying inside the other, because the two caps count different things.

## Where this came from

- [The design, Section 9.1 to Section 9.5](../../../docs/design/strategy_discovery_design.md): the reason
  a candidate is data, the twelve-block surface, the five-stage pipeline, the bounds and the caps.
- [The implementation plan](../../../docs/design/strategy_discovery_implementation.md), whose
  compiler milestone fixes these rules and whose exit note carries the criterion-3 finding.
- [The schema and its reader](../../../python/nautilus_trader/discovery/schema.py):
  `parse_specification`, the block readers, the condition walk, `scan_for_code_references`,
  `RefusalCode`, `Bound` and `canonical_mapping`.
- [The validation stages](../../../python/nautilus_trader/discovery/validate.py) and
  [the compiler](../../../python/nautilus_trader/discovery/compile.py): the stage order, the closed
  ordering tables, the warning channel, `compile_specification`, `compose_plan` and `CompiledPlan`.
- [The pre-registered policy](../../../python/nautilus_trader/discovery/policy.py) and
  [the feature registry](../../../python/nautilus_trader/discovery/features.py): the fifteen declared
  sets, the nineteen inclusive bounds `_FIRST_RELEASE_BOUNDS`, `ComplexityLimits`, and the resolution
  of a declared alias to a definition.
- [The code-refusal integration fixture](../../../python/tests/integration/test_discovery_generated_code_refusals.py),
  whose eight tests carry criterion 3, and the [acceptance bundle record](../../../python/tests/acceptance/discovery_acceptance_bundle.json)
  that reports their observed result.
- [The schema unit tests](../../../python/tests/unit/discovery/test_schema.py),
  [the compiler unit tests](../../../python/tests/unit/discovery/test_compile.py) and
  [the legal payload fixture](../../../python/tests/unit/discovery/fixtures.py), the specification
  every negative case mutates.

## Words used in this tutorial

- block: one named section of a candidate, holding the fields that answer one question about it.
- bound: the inclusive range or the list of allowed choices the policy states for one field.
- candidate: one proposed strategy, written as data and read without anything being run.
- compiled plan: the accepted specification together with everything its behaviour depends on,
  carrying two fingerprints.
- condition leaf: one comparison inside an entry rule, such as a feature being above a number.
- policy: the pre-registered document that decides which values a field may hold, published before
  any candidate is proposed.
- refusal: one recorded reason a payload was not accepted, written as a code, a path and a message.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
