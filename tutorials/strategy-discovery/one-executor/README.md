# The one executor: handing a compiled candidate to the engine

Date: 2026-10-16. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                                                                                                                                                                                             |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Nothing on its own: it acts on the instruments one compiled candidate selects, and it submits one batch of buy and flatten instructions for them instead of placing orders itself                                                                                                                                                                                                                                                                                                 |
| How often it trades       | It decides once per price bar it receives, and it selects, allocates and submits only on a rebalance bar                                                                                                                                                                                                                                                                                                                                                                          |
| What you need             | Nothing but this page; the worked example needs only a table of four rows                                                                                                                                                                                                                                                                                                                                                                                                         |
| Where the rules come from | The interpreter and its allocation contract in [strategy.py](../../../python/nautilus_trader/discovery/strategy.py), and the design's Sections 9.3 and 9.7 in [the design](../../../docs/design/strategy_discovery_design.md)                                                                                                                                                                                                                                                     |
| The underlying research   | None, this is the repository's own machinery: the allocation contract is a specification in the design rather than a published paper, and the nine decisions it fixes are the design's, not an author's                                                                                                                                                                                                                                                                           |
| How well it held up       | Mixed: the interpreter matches independently specified strategies fixture by fixture, and the interpreter-parity, feature-parity and allocation-determinism criteria pass, but the timing criterion fails here because the decision bridge resolves an instrument's eligibility and defers nothing, so the timing contract is carried by the engine's latency model rather than by the bridge; the fixture's pre-enforcement half is kept as evidence that the test discriminates |
| Also appears in           | [The admission gate](../the-admission-gate/README.md), which reads what a run produced, and [Scoring a strategy on its evidence, not its profit](../../project/scoring-evidence-not-profit/README.md), on judging a result by its evidence                                                                                                                                                                                                                                        |

## The idea in one paragraph

A search proposes a candidate strategy, and a candidate is written down as data rather than code: a
list of blocks with fixed shapes that can be read, hashed and refused without being run. Something
then has to turn that data into an actual test. The one executor does that. It is a single standing
object that the engine drives, and that object watches one price bar per instrument, decides on every
bar, and only on the bars that open a new rebalancing period chooses which instruments to hold and
how much of each. It does not compute prices, size orders or place them; it reads feature values the
research layer already keeps, turns its chosen weights into instructions, and hands one batch of those
to the platform's own machinery, which reconciles them and applies its own caps. Everything the run
produced is then packaged with the identities of what ran, so a later reader compares an artifact
rather than a claim; nothing here predicts a price or claims a profit.

## Why anyone believed it

The alternative is to compile a candidate into code: generate a program from the specification, import
it, and let it run. The repository rejects that. Generated code means side effects on import, a second
execution path that the ordinary tests never touch, and a hashed specification that no longer fully
describes the thing that ran. The argument for one shared executor is that the artifact which was
hashed and the artifact which ran become the same object, so two runs of the same specification differ
only by data, seed and engine build, and nothing hidden in a generated file can drift between them.

That argument has a price, and the design states it plainly. One executor is a single point of failure
for every candidate, so its own testing has to be strong enough that a candidate's disappointing
result can never be blamed on the shared machinery.

## An everyday comparison

Think of a player piano and a stack of punched paper rolls. Each roll is only a pattern of holes - data,
not music - and one piano reads any roll: the roll says which notes, and when, and the piano does the
sounding. If you want to test two rolls, you do not build a second piano, you change the roll. That is
the executor. The engineering habit it buys is that the record of the performance comes from the piano
rather than from a claim written on the roll. The cost is the obvious one: a mistuned piano makes every
roll sound wrong.

## The rules, step by step

1. **Read the specification, not code.** The compiled candidate describes one object's settings: the
   plan, the universe and its members, the venue, the commission, the stop distance and any sectors.
2. **Refuse a wrong type early.** The venue is not read until the first decision and the stop distance
   not until the target pipeline is switched on, so both are type-checked when the object is built;
   otherwise a run built from a wrong type would decide nothing and still report success.
3. **Subscribe to one bar per instrument.** For a declared daily signal frequency the object watches
   the daily bar series of each instrument in the run, and keeps one running store of feature values
   per instrument.
4. **On every bar, in order:** update that instrument's store; advance the decision period if the bar
   belongs to a new one; evaluate the exits; and if the bar opens a rebalance period, select and
   allocate. It then submits one batch and appends a decision record, on every bar, so the exits are
   looked at more often than the holdings are changed.
5. **Select the cross-section.** The selection ranks the eligible instruments by the declared feature
   value, breaks ties by its declared rule, and takes either a count or a percentile of them. An
   instrument with no available feature value is not in the selection at all and can never be weighted.
6. **Allocate the nine decisions, in this order, so the same selection and caps always produce the same
   targets.** 1: the declared weighting method turns the ordered selection into weights. 2: ties are
   already broken by that order, which the weights keep. 3: a name with no available feature value is
   not in the selection and is never weighted. 4: a held name that has become ineligible is flattened
   when the candidate declares it and left untouched when it does not. 5: a held name that has left the
   selection but is still eligible is flattened too. 6: the per-position cap clamps each weight and
   redistributes nothing. 7: the per-sector cap scales a sector's weights down proportionally. 8: the
   per-gross-exposure cap scales every weight down proportionally. 9: the cash policy leaves the weight
   those caps left unused idle, or offers it back to the names with room in a loop bounded by the number
   of instruments.
7. **Submit exactly one batch.** The exits come first, then the chosen holdings, one instruction per
   instrument; a holdings instruction is a buy, and an instrument the allocation weighted at zero is a
   flatten. A flatten is never suppressed, even when new buying is halted.
8. **Let the platform finish the job.** The batch is handed to the target pipeline, which turns an
   instruction into a quantity, reconciles it against the position and resting orders, emits the
   smallest set of orders that realises it, and applies the engine's own risk caps. The executor never
   places an order itself.
9. **Attribute the fills.** Each fill the engine reports becomes a fill record, matched back to the
   decision that produced its order, so a reader can see which decision asked for it; a fill from an
   order this executor did not submit records no decision at all.
10. **Package the run.** The runner keeps the engine's own canonical result beside the identities of
    what ran and returns them together as one evidence package, which the ledger then stores once and
    never rewrites.

## The maths, with every symbol named

The allocation turns a rank into a weight by the declared method. For the proportional method, an
instrument at position `i` of a selection of `count` instruments gets the share

```text
share_i = count - i
weight_i = share_i / (share_0 + share_1 + ... + share_(count-1))
```

- `count` is how many instruments the selection produced, at most the declared portfolio size.
- `i` is the instrument's position in the selection, counting from zero, as the selection ordered it.
- `share_i` is the raw share before normalising, so the first name gets the most.
- `weight_i` is that share divided by the sum of all the shares, so the weights add to one before any
  cap is applied. The equal method instead gives every selected name the same `weight_i`.

The per-position cap then clamps each weight, and the declared weight is carried into the run as an
instruction strength rather than as a quantity:

```text
capped_i = min(weight_i, max_position_weight)
strength_i = clamp(capped_i / max_position_weight, 0, 1)
```

- `max_position_weight` is the declared largest share of the account one holding may take.
- `capped_i` is the weight after the cap, which the cap never redistributes: clamping one name does
  not push its excess onto another.
- `strength_i` is the number the instruction carries, between zero and one. The pipeline sizes a
  position from risk, and this strength tells it what fraction of the allowed size the candidate wants.

Two smaller relations fix when a value may be read and how much may be invested:

```text
available_at = observed_at + availability_lag_ns
ceiling = min(1, max_gross_exposure)
```

- `observed_at` is the event time of the bar the feature value was read from.
- `availability_lag_ns` is the delay the feature's own definition declares before that value may be
  used, so a value published later is never treated as known earlier.
- `available_at` is the first instant the value may be read; the store keeps a value only when its
  availability has passed at the decision instant, and drops one that is already past its declared
  skip. Because the lag cannot be negative, the availability instant is never earlier than the
  observation instant.
- `max_gross_exposure` is the declared total-weight cap. The ceiling the caps are measured against is
  one unit of the account, or that cap when it is smaller, so a larger declared cap admits no more
  than the whole account.

## A worked example

Four instruments are in the universe and the candidate declares a rank proportional weighting, a
portfolio size of three, a per-position cap of 0.25, no sector cap and no gross cap, and the policy of
leaving leftover cash idle. At one decision the ranking produces the selection in this order: A, B, C.
The fourth instrument, D, is ranked below the cut and is not selected.

| Instrument | Selection position `i` | Share `3 - i` | Raw weight | After the 0.25 cap | Instruction strength |
| ---------- | ---------------------- | ------------- | ---------- | ------------------ | -------------------- |
| A          | 0                      | 3             | 0.5000     | 0.2500             | 1.0000               |
| B          | 1                      | 2             | 0.3333     | 0.2500             | 1.0000               |
| C          | 2                      | 1             | 0.1667     | 0.1667             | 0.6667               |

The arithmetic, step by step.

```text
sum of shares = 3 + 2 + 1 = 6
raw A = 3 / 6 = 0.5000    raw B = 2 / 6 = 0.3333    raw C = 1 / 6 = 0.1667
sum of raw weights = 1.0000
capped A = min(0.5000, 0.2500) = 0.2500
capped B = min(0.3333, 0.2500) = 0.2500
capped C = min(0.1667, 0.2500) = 0.1667
total invested = 0.2500 + 0.2500 + 0.1667 = 0.6667, residual left idle = 0.3333
strength A = 0.2500 / 0.2500 = 1.0000
strength B = 0.2500 / 0.2500 = 1.0000
strength C = 0.1667 / 0.2500 = 0.6667
```

Read plainly: A and B each ask for the largest holding the cap allows, C asks for a smaller one, and a
third of the account stays in cash because the cap on A and B stopped the raw weights from being
realised. Now suppose the same decision declared the reallocate policy instead. The residual, 0.3333,
is offered to the names with room left, in their declared share proportions; only C has room, so it can
take at most 0.2500 - 0.1667 = 0.0833. C moves to 0.2500, the total invested rises to 0.7500, and the
remaining 0.2500 stays idle because every name now sits at the cap, and no name was pushed above it.

## What the research actually found

There is no paper behind this page, because the executor is machinery rather than a trading idea. The
answer is what the repository's own tests measured about it, and that answer is mixed. The seven
reference strategies of the design - a ranking strategy, a threshold strategy, a holding
period exit, a rebalance schedule, a target allocation, a strategy with no eligible instruments, and one
that meets an opening gap - are each specified by hand as a fixture and compared against what the
interpreter produced. The acceptance record states that all seven agree with the hand-written
expectation, that the first servable decision is the third period, which is the declared lookback, so
the warm-up is pinned rather than assumed, and that the ranking case selects the same two names in the
same order on all forty-five decisions whose cross-section holds all four instruments.

Feature values read at decision time agree with the research path on the value, both timestamps, the
unit label and the definition identity. Four behaviours beyond agreement were measured too: a warm-up
reading that a research indicator reports as zero is served as nothing rather than as zero, a value
with a one-day publication lag is served one bar late, membership is read at the decision instant, and
a series shorter than the lookback yields no value.

Allocation has thirty-two cases, twenty-one for the declared rules, written against the numbers rather
than the implementation, so each rule has a case that fails if it is reversed; determinism and
immutability are among them, and the same request always produces the same targets.

One criterion does not hold, and it is about timing rather than about this page's arithmetic. A signal
must not fill before its declared eligibility instant, and the test must be able to fail. The
pre-enforcement half does fail as intended, which is what makes the test discriminating: with no
deferral the fill carries the decision bar's own timestamp and that bar's own close. The enforced half
holds for the engine's latency model, but not for the decision bridge: the bridge resolves an
instrument's eligibility and stamps the instruction with it, and its submission path holds no clock, so
the order set is submitted at delivery and the fill can precede the stamped instant by the whole gap to
the next session. The missing piece is therefore a deferral in that submission path, not a check on the
fill; until it exists the timing contract is carried by the latency model alone.

## How this project relates to it

The executor lives in [strategy.py](../../../python/nautilus_trader/discovery/strategy.py), which holds
the object, the nine allocation decisions and the selection rules. The runner that injects the object
and packages the run is [run.py](../../../python/nautilus_trader/discovery/run.py), and the package it
returns is assembled in [ledger.py](../../../python/nautilus_trader/discovery/ledger.py).

The one test that walks the whole path is
[test_discovery_round_trip.py](../../../python/tests/integration/test_discovery_round_trip.py). It
compiles one hand-written specification, runs it once, checks that the interpreter's decisions,
instructions, targets and fills line up, then stores the package in the ledger and refuses to rewrite
it. A reader who does read code can run it with

```bash
cd python && python/.venv/Scripts/python.exe -m pytest tests/integration/test_discovery_round_trip.py -q
```

## Where it goes wrong

- One executor is one point of failure. A defect here affects every candidate and presents itself as a
  strategy result, which is why the design demands fixture-by-fixture coverage rather than a completion
  check. The runner's own unit test file is named in the plan but absent, so this page's path is covered
  only by the integration round trip below.
- The timing contract is not enforced by the bridge. The bridge stamps an instruction with the instant
  it becomes eligible and submits it without waiting, so the deferred fill can land earlier than the
  stamp. The missing piece is a submission path that can hold a clock and release the instruction at
  its stamped instant; until then the engine's latency model carries the contract. One settlement
  point ignores it: the shutdown drain fills an instruction the object is never told about, and a fill
  the object cannot see is a holding it cannot reason from.
- The first release is narrow. It runs long only, on daily signals, with weekly or slower rebalancing,
  over equities and exchange-traded funds, and it has no stop or target that needs the inside of an
  intraday bar. A frequency outside the declared one is refused rather than approximated by a bar size
  the candidate never declared.
- A failed run raises rather than returning a failure. The protocol that records failures in the ledger
  belongs to a later milestone, so at this stage a run has two outcomes - completed or an exception -
  and nothing in between is recorded.

## Try it yourself

This is a paper exercise with a table of four rows and no code, using the numbers from the worked
example. Set up columns headed Instrument, Rank position, Share, Raw weight, Weight after the 0.25 cap
and Instruction strength, and fill in rows for A, B and C.

1. Give the three names shares of 3, 2 and 1 by their rank position, and add them: the total is 6.
2. Divide each share by 6 to get the raw weights, 0.5000, 0.3333 and 0.1667.
3. Apply the cap of 0.25 to each, clamping any weight above it down to 0.25.
4. Add the capped weights: 0.6667, so 0.3333 is left as cash under the idle policy.
5. Divide each capped weight by 0.25 to get the instruction strengths, 1.0000, 1.0000 and 0.6667.
6. Now change one thing: add a fourth instrument D at the bottom of the ranking and raise the portfolio
   size to four, keeping the cap at 0.25. The shares become 4, 3, 2 and 1, and the raw weights 0.4000,
   0.3000, 0.2000 and 0.1000.

What to notice: in the second run the cap bites on A and B, and the total invested falls even though
the candidate asked to hold more names - a cap does not send the excess anywhere unless the reallocate
policy tells it to, which is why the allocation is stated as an ordered list of decisions and caps.

## Where this came from

- [The interpreter and allocation contract](../../../python/nautilus_trader/discovery/strategy.py), the
  object, its inputs, the nine allocation decisions and their order, the selection rules and the
  rebalance keying.
- [The runner](../../../python/nautilus_trader/discovery/run.py) takes the request, raises on a
  failed run, and returns the [evidence package](../../../python/nautilus_trader/discovery/ledger.py),
  thirteen fields appended once and never rewritten.
- [The feature values](../../../python/nautilus_trader/discovery/features.py), the observation and
  availability instants and the snapshot rule that decides what is readable at a decision.
- [The round-trip test](../../../python/tests/integration/test_discovery_round_trip.py), the whole path
  from a specification to a ledger entry, and every assertion it makes along the way.
- [The design](../../../docs/design/strategy_discovery_design.md) fixes the contract: Section 9.3 on
  the interpreter being the only executor and its reference fixtures, Section 9.7 on the nine
  allocation decisions, and Section 15 criterion 4 on timing; the
  [implementation plan](../../../docs/design/strategy_discovery_implementation.md) records the M2
  findings on which mechanism enforces that timing contract.
- [The acceptance bundle](../../../python/tests/acceptance/discovery_acceptance_bundle.json), the entries
  for the interpreter parity, feature parity and allocation determinism criteria, and the failed entry
  for the timing criterion with its named defect.

## Words used in this tutorial

- executor: the one standing object the engine drives to run any compiled candidate, also called the
  interpreter.
- cross-section: every instrument's feature values at one decision instant, taken together.
- decision bar: any price bar the executor receives, on which it evaluates exits and records a
  decision.
- rebalance bar: a decision bar that opens a new rebalancing period, on which the executor also selects
  and allocates; for a daily frequency every decision bar is one.
- target weight: the share of the account one selected instrument is asked to take, after the caps are
  applied.
- instruction strength: the number between zero and one an instruction carries, which tells the
  platform's pipeline what fraction of the allowed position size the candidate wants.
- target pipeline: the platform's own machinery that turns an instruction into an order set and applies
  the engine's risk caps.
- evidence package: the record one run returns, holding the engine's canonical result beside the
  identities of what ran.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
