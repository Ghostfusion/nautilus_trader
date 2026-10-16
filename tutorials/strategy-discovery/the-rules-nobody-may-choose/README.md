# The rules nobody may choose

Date: 2026-10-16. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Nothing is bought or sold: the part acts on field values, and it is the frozen table every candidate's names and numbers are resolved from, so a value it does not declare cannot be compiled at all                                                                                                                                                                                                                                                                        |
| How often it trades       | Once per campaign, then on every compilation: the campaign publishes it with its fingerprint before it opens, and the validator reads it on each candidate it checks                                                                                                                                                                                                                                                                                                        |
| What you need             | Nothing but this page; the worked example needs only a table of field values and the declarations they are checked against                                                                                                                                                                                                                                                                                                                                                  |
| Where the rules come from | [The pre-registered policy](../../../python/nautilus_trader/discovery/policy.py) and the resolution it drives in [the schema's readers](../../../python/nautilus_trader/discovery/schema.py)                                                                                                                                                                                                                                                                                |
| The underlying research   | None that measures it: no study was found showing that a constrained language discovers better strategies than free code generation, which [the design](../../../docs/design/strategy_discovery_design.md) records as unproven rather than established                                                                                                                                                                                                                      |
| How well it held up       | Strong as a definition: the declared sets, the bounds, the references and the sector map are the only path a field value resolves through, so a value the policy does not declare cannot be compiled at all, and each refusal is tested by name; the campaign's own inputs, which are the feature allowlist, the references and the sector map, are supplied by the caller rather than invented here, and no policy-file loader exists to refuse a credential placed in one |
| Also appears in           | [What a candidate is](../what-a-candidate-is/README.md), on the language this policy closes; [The admission gate](../the-admission-gate/README.md), on the thresholds a policy carries; [Searching by breeding](../search-by-evolution/README.md), on the search that may change nothing here; and [What ran versus what was written](../what-ran-versus-what-was-written/README.md), on the digest this policy is recorded by                                              |

## The idea in one paragraph

A campaign in this repository searches for strategies, but it does not get to decide what counts as
acceptable. Those decisions are written down first, in one document called the policy: fifteen lists
of permitted names, nineteen permitted ranges for numbers, three limits on how large an expression
may be, a handful of named references pinned to a fingerprint, and a map from each instrument to its
business sector. A candidate reads its values out of that document and from nowhere else, so a name
or a number the policy does not declare cannot be compiled at all. The policy is fingerprinted
before the campaign opens, and that fingerprint is folded into every compiled plan and recorded
against every trial. The search may read the policy but not write it, and a run may only be read
against the policy it was judged under.

## Why anyone believed it

A search that picks its own limits always finds something that looks good, because choosing the
limits is itself one of the choices being made. If the party proposing candidates could also set the
ceiling on how much risk is permitted, or choose which comparison counts as the benchmark, then the
number it reports would describe its own freedom rather than the market. The belief behind a frozen
policy is that the rules can be fixed before anything is looked at, and that fixing them is what
makes the later number mean anything. The design names the worry itself: a system might improve how
it searches while quietly relaxing what it demands. So the rules are taken away from the search, and
the only thing left to improve is the search.

## An everyday comparison

A school examination works this way. The board publishes the mark scheme and the pass mark before
the examination is sat: which questions are worth what, what counts as a correct method, and the
score at which a paper passes. The examiner may not decide afterwards that 45 marks is a pass, and a
paper answering a question the scheme does not contain cannot be marked at all. The scheme carries a
version, so a result from last year can only be read against the scheme it was marked under, and
changing the scheme is a new year rather than a re-mark of the old one.

## The rules, step by step

1. Write one policy per campaign: a single versioned document, published with its fingerprint before
   the campaign opens.
2. Declare the fifteen sets of permitted names; each set names the field whose values come from it,
   and a value outside its set is refused rather than corrected.
3. Declare the permitted numbers as bounds, each named after the field it governs. The first release
   declares nineteen.
4. Declare the three complexity caps, which limit how much expression one candidate may spend.
5. Pin the references: each carries a kind and the fingerprint of a fixed artifact, so a candidate
   cites a name and the compiler resolves it to that fingerprint.
6. Declare the sector map, one instrument to one sector, or declare none and accept that no sector
   cap may be stated.
7. Supply the campaign's own inputs. The feature allowlist, the references and the sector map are
   arguments to the policy rather than contents of it, and it invents none of them.
8. Resolve every value through one path: a name from a declared set, a number against a declared
   bound, a feature against the allowlist and the registry, a reference against the pinned list.
9. Refuse an undeclared bound. Where the policy declares no bound for a field, every value of that
   field is refused, because an unstated bound is not an unbounded one.
10. Freeze. Changing any frozen declaration is not a tuning step; it begins a new campaign with a new
    trial inventory, and the previous campaign's record stands.

## The maths, with every symbol named

The fifteen declared sets, with their members as the first release declares them:

| Declared set                  | Its members                                           |
| ----------------------------- | ----------------------------------------------------- |
| `families`                    | cross_sectional_rank, absolute_threshold, static_hold |
| `asset_classes`               | EQUITY, ETF                                           |
| `membership_policies`         | point_in_time                                         |
| `signal_frequencies`          | daily                                                 |
| `decision_times`              | session_close, session_open                           |
| `execution_eligibilities`     | same_session_close, next_session_open                 |
| `corporate_action_treatments` | adjusted, raw                                         |
| `entry_directions`            | long                                                  |
| `operators`                   | gt, ge, lt, le                                        |
| `selection_modes`             | rank, threshold, none                                 |
| `tie_breaks`                  | feature_desc, instrument_id_asc                       |
| `weighting_methods`           | equal, rank_proportional                              |
| `cash_policies`               | leave_idle, reallocate                                |
| `rebalance_frequencies`       | daily, weekly, monthly                                |
| `rebalance_timings`           | next_session_open                                     |

A value is admitted only where a declaration covers it; the two forms are membership of a set and
inclusion in an inclusive range. The sets are also the scope: long only, daily signals, equities and
ETFs, with no value declared for shorting, options, intraday logic or fundamental input, and no stop
or target that needs a daily bar's own path.

```text
declared(field) = S, the policy's set for that field;   admitted(value) = value is in S
admitted(value) = lower(B) <= value <= upper(B)
describe(B) = "lower to upper inclusive"   when B is a range
describe(B) = "one of a, b, c"             when B is a choice
```

- `field` is the position in a candidate, such as `rebalance.frequency`.
- `S` is the declared set the policy holds under that field's set name.
- `B` is the bound the policy declares under the field's own name, such as `portfolio.position_count`.
- `lower` and `upper` are its two end values, and both ends are themselves admitted, which is what
  "inclusive" means.
- A value that fails either test is refused with the set's members or with `describe(B)` in the
  message, so the refusal states the declaration rather than the candidate's opinion of it.

The nineteen bounds of the first release, each named after the field it governs:

| Bound                           | What the field is                       | Admitted range             |
| ------------------------------- | --------------------------------------- | -------------------------- |
| `feature.lookback`              | how many periods a feature looks back   | 1 to 252 inclusive         |
| `feature.skip`                  | periods skipped before the lookback     | 0 to 5 inclusive           |
| `selection.percentile`          | the share cut off by a rank selection   | 0.01 to 1 inclusive        |
| `selection.count`               | how many instruments are selected       | 1 to 50 inclusive          |
| `selection.threshold`           | the level a numeric selection compares  | -1000000 to 1000000        |
| `condition.value`               | the level a condition compares against  | -1000000 to 1000000        |
| `exit.holding_period_cap`       | the longest a position may be kept      | 1 to 252 inclusive         |
| `exit.rank_exit_count`          | how far a rank may fall before exit     | 1 to 50 inclusive          |
| `exit.stop_fraction`            | the fall that triggers an exit          | 0.001 to 0.5 inclusive     |
| `exit.target_fraction`          | the rise that triggers an exit          | 0.001 to 5 inclusive       |
| `portfolio.position_count`      | how many positions are held at once     | 1 to 50 inclusive          |
| `portfolio.max_position_weight` | the largest share in one position       | 0.005 to 0.25 inclusive    |
| `portfolio.max_sector_weight`   | the largest share in one sector         | 0.1 to 1 inclusive         |
| `risk.max_gross_exposure`       | total exposure across all positions     | 0.1 to 2 inclusive         |
| `risk.max_drawdown`             | the fall from a peak that is permitted  | 0.01 to 0.9 inclusive      |
| `risk.max_turnover`             | how much of the portfolio is replaced   | 0.05 to 10 inclusive       |
| `risk.max_cost_ratio`           | costs as a share of the money traded    | 0.0001 to 0.1 inclusive    |
| `universe.min_price`            | the lowest price an instrument may have | 1 to 1000 inclusive        |
| `universe.min_dollar_volume`    | the least daily money traded            | 0 to 10000000000 inclusive |

The three complexity caps are limits rather than bounds on a single field:

```text
max_conditions = 4        the most leaf conditions one entry block may state
max_condition_depth = 2   how deep a condition tree may nest, counted from 1
max_features = 8          the most features one candidate may declare
```

- A leaf condition is one comparison, such as a feature being greater than a level.
- Depth is counted from 1, so a top-level comparison is depth 1 and one nesting level inside it is
  depth 2, the last admitted; the leaf count is checked after the whole tree is read.

The policy's fingerprint covers every declaration, sorted so the order it was written in cannot
matter:

```text
digest(policy) = "sha256:" + SHA-256( canonical_text(policy) )
```

- `SHA-256` is a fixed recipe turning text into 64 hexadecimal digits, and the same text always gives
  the same digits.
- `canonical_text` is the policy written with its keys sorted and its sets sorted, so two policies
  declaring the same things are the same text and carry the same fingerprint.
- A declared reference carries a digest of the same shape: `"sha256:"` is seven characters and the
  hexadecimal part is 64, so a reference digest is 71 characters and a shorter one is refused.

## A worked example

One candidate's field values are resolved against the first-release policy, one at a time. The
values are made up, but of the size these fields take.

| Field in the candidate             | Value  | The policy declares                | Outcome  | What the validator returns                                            |
| ---------------------------------- | ------ | ---------------------------------- | -------- | --------------------------------------------------------------------- |
| `features.declared.trend.lookback` | 20     | bound 1 to 252 inclusive           | admitted | nothing                                                               |
| `features.declared.trend.feature`  | close  | a name on the campaign's allowlist | admitted | nothing                                                               |
| `selection.percentile`             | 0.01   | bound 0.01 to 1 inclusive          | admitted | nothing                                                               |
| `portfolio.position_count`         | 20     | bound 1 to 50 inclusive            | admitted | nothing                                                               |
| `portfolio.max_position_weight`    | 0.20   | bound 0.005 to 0.25 inclusive      | admitted | nothing                                                               |
| `portfolio.max_position_weight`    | 0.30   | bound 0.005 to 0.25 inclusive      | refused  | max_position_weight must be 0.005 to 0.25 inclusive, was 0.30         |
| `selection.count`                  | 51     | bound 1 to 50 inclusive            | refused  | count must be 1 to 50 inclusive, was 51                               |
| `rebalance.frequency`              | hourly | set daily, monthly, weekly         | refused  | frequency must be one of daily, monthly, weekly, was hourly           |
| `features.declared.trend.feature`  | other  | an allowlist the campaign declared | refused  | feature must be one of the policy's registered features, other is not |
| `evaluation.benchmark`             | other  | the pinned reference names         | refused  | benchmark must name a frozen reference, other is not declared         |

Read the two weight rows together: 0.20 was admitted for one field and 0.30 refused for the same
field, and nothing in the candidate's own text decided that, the bound did. The last row is the same
act one level up: the candidate cites a name, the policy resolves it to a fixed fingerprint, and a
name the policy does not hold resolves to nothing.

Now the case the schema is often misread on. Suppose the policy is loaded with one bound left out:
`portfolio.position_count` is removed, nothing else changes, and the candidate still declares 20.
The lookup for a bound named `portfolio.position_count` returns nothing, because the policy declares
no bound under that name, so the portfolio block refuses with the message that the policy declares no
bound named portfolio.position_count, so the portfolio block cannot admit a value. Every other value
of that field is refused too, including the 20 admitted a moment ago, so a missing bound is not a
default: the policy does not cover the field, and no value of it can be reached.

## What the research actually found

This page describes machinery, so what exists to check is the repository's own tests rather than a
market. Each refusal above is exercised by name in
[the schema's unit tests](../../../python/tests/unit/discovery/test_schema.py): a value outside a
declared bound is refused with the bound stated, a bound the policy does not declare refuses every
value of the field, a value outside a declared set is refused, a reference the policy does not
declare is refused, and a feature outside the campaign's allowlist is refused. Two properties are
asserted as pairs: a payload legal under the shipped bound becomes illegal when the policy is
rebuilt with a narrower bound and nothing in the payload changes, which is what makes the bound the
policy's rather than the schema's; and a changed policy changes only the behavioural fingerprint of
a compiled plan. The sector map is tested the same way, accepted with a map and refused without one,
and the campaign audit compares the fingerprint a reader holds with the one the record carries.

The design is explicit that none of this measures discovery quality. Section 9.1 concedes that no
head-to-head benchmark was found showing a constrained language discovers better strategies than
free code generation, so constrained generation is justified on safety and auditability grounds
rather than as a measured claim. The implementation plan carries one further limit rather than
repairing it: the design asks that a configuration loader refuse a credential placed in a policy
file, and no policy-file loader exists in the package, so that half is not testable and is not
claimed; the checkable half is that the policy's fields declare no credential at all. Nothing here
is a measurement of what any strategy earns on any market.

## How this project relates to it

The policy lives in [policy.py](../../../python/nautilus_trader/discovery/policy.py): the fifteen
declared sets, the nineteen bounds, the three complexity caps, the typed references with their kinds
and digests, the sector map, and the digest covering all of them. Values resolve in
[schema.py](../../../python/nautilus_trader/discovery/schema.py), whose readers take a name from a
declared set, a number from a declared bound, a feature from the campaign's allowlist and a
reference from the pinned list, collecting a refusal rather than stopping at the first one. The
fingerprint is folded into the compiled plan in
[compile.py](../../../python/nautilus_trader/discovery/compile.py) and read back in
[ledger.py](../../../python/nautilus_trader/discovery/ledger.py), which is how a campaign says which
rules it ran under.

A reader who does read code can run the suite that carries the refusals above:

```bash
python -m pytest python/tests/unit/discovery/test_schema.py
```

The declarations are also asserted against the design's own sections: Section 9.4 for bounds as
inclusive ranges or enumerated choices, Sections 10.4 and 11.2 for what is frozen and published, and
decisions SD7 and SD12 for the freeze rule and the first-release scope.

## Where it goes wrong

- The policy's reach stops at its own declarations. The allowlist, the references and the sector map
  are inputs the caller supplies, so a campaign can declare a thin allowlist and every test passes.
- The freeze rule is a record, not a wall. A policy edited afterwards does not stop anything
  retroactively; it makes the old results read as invalid, and only comparing the two fingerprints
  finds that out.
- No policy-file loader exists, so nothing refuses a credential placed in a policy file; the
  checkable half is narrower, that the policy's fields declare no credential at all.
- The first-release scope states weekly or slower rebalance, while the declared set the compiler
  reads also contains daily; where the two differ, the declared set decides, being the only path a
  value resolves through.
- Admitted is not good. A value inside every bound and every set says only that the policy covers
  it; nothing about a policy makes a strategy, and the design's deliverable is evidence, not a
  return.
- A reference pins a fingerprint, not the artifact. The policy holds a frozen object's digest and
  never the object, so it can say two results cited one benchmark and cannot say the benchmark
  existed.

## Try it yourself

You need a table and nothing else; the exercise is to see how much of a candidate's freedom is the
policy's.

1. Write three column headings: `Field`, `Value`, `Outcome`. Copy the ten rows of the worked example,
   leaving the outcome blank, with the bounds and sets beside them.
2. For each row, decide admitted or refused, and write the sentence you think the validator would
   print, naming the declaration that allowed what it allowed. Compare with the worked example.
3. Now delete the bound row named `portfolio.position_count` and re-mark the row declaring 20.

What to notice: the same number is admitted or refused depending only on a table no candidate can
write, and one missing row of that table does not widen the field. It closes it.

## Where this came from

- [policy.py](../../../python/nautilus_trader/discovery/policy.py), the fifteen declared sets, the
  three complexity caps, the reference kinds and digests, the sector map, the nineteen first-release
  bounds and the digest covering all of them.
- [schema.py](../../../python/nautilus_trader/discovery/schema.py), the readers resolving a name, a
  number, a feature and a reference, and the refusals they return with the declaration stated.
- [compile.py](../../../python/nautilus_trader/discovery/compile.py) and
  [ledger.py](../../../python/nautilus_trader/discovery/ledger.py), where the policy digest enters a
  compiled plan and where a campaign audit compares it with the digest a reader holds.
- [evolve.py](../../../python/nautilus_trader/discovery/evolve.py), the frozen field list and the
  search policy that may amend only its own named mutable fields.
- [The schema's unit tests](../../../python/tests/unit/discovery/test_schema.py) and
  [the validator's unit tests](../../../python/tests/unit/discovery/test_validate.py), the refusals
  tested by name and the fixture policy every discovery test builds from.
- [strategy_discovery_design.md](../../../docs/design/strategy_discovery_design.md), design Sections
  9.1, 9.4, 10.4 and 11.2 with decisions SD7 and SD12, and
  [strategy_discovery_implementation.md](../../../docs/design/strategy_discovery_implementation.md),
  Section 6 for the package layout and Section 12.1 for the carried limit on the policy-file loader.

## Words used in this tutorial

- declared set: one of the fifteen named lists of permitted names a field value must belong to.
- bound: a range or list attached to a named field, so that field admits only what was declared.
- inclusive range: a range whose two end values are themselves admitted, so 1 to 252 contains 1.
- complexity cap: one of three limits on expression size: four leaf conditions, two levels, eight features.
- frozen reference: a name pinned by the policy to a fingerprint, with a kind saying what it names.
- sector map: the list pairing each instrument with its sector, without which a sector cap is refused.
- freeze rule: the policy is fixed and fingerprinted before a campaign opens, so an edit starts a new one.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
