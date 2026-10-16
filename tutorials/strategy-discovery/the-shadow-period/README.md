# The shadow period: watching a frozen candidate against a live feed

Date: 2026-10-16. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                   |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Nothing by itself: it watches one already-finished candidate as a feed delivers fresh prices, and computes a fill from each observation instead of sending an order                                                                                                                                                                                                                                                                                                                                     |
| How often it trades       | Once per shadow period, and then once for each signal the feed delivers during that period                                                                                                                                                                                                                                                                                                                                                                                                              |
| What you need             | Nothing but this page; the worked example needs only a small table of numbers                                                                                                                                                                                                                                                                                                                                                                                                                           |
| Where the rules come from | The monitor [shadow.py](../../../python/nautilus_trader/discovery/shadow.py) and [the design](../../../docs/design/strategy_discovery_design.md) Section 11.5                                                                                                                                                                                                                                                                                                                                           |
| The underlying research   | None directly: the rules are this repository's own design, Section 11.5 and acceptance criteria 13 and 15, rather than a published study                                                                                                                                                                                                                                                                                                                                                                |
| How well it held up       | Strong as a definition: the contract, the parity rule over declared values, the observation record and every count in the report are unit- and integration-tested, and the two criteria for the shared contract and for reporting divergence both pass; but meeting the declared observation count is a precondition for reading a report rather than evidence of anything, and the shadow half is fed observed data rather than bound to a live adapter, which the plan records as a platform decision |
| Also appears in           | [One executor](../one-executor/README.md) and [The pre-registered gate chain and the pre-campaign admission test](../the-admission-gate/README.md)                                                                                                                                                                                                                                                                                                                                                      |

## The idea in one paragraph

A backtest applies a candidate's rules to prices that have already happened. The shadow period does
something narrower and stranger: it takes one candidate that is already frozen, so its rules may not
change, and watches it run while a feed delivers fresh observed prices, computing what would have
filled rather than sending any order. Both halves write down the same four declarations about when
they decide and when they may execute, and the first job is to compare those declarations by value,
not by name. Then each watched signal becomes one record: when it arrived, whether it intended a
trade, when it was allowed to fill, the price it would have filled at, the two cost figures, and why
it did not fill if it did not. The report counts what happened instead of summarising it. Meeting the
observation count declared before the period began is a precondition for reading the report, and it
is not evidence of anything.

## Why anyone believed it

A backtest assumes that the rule sees a price and fills at a price. When the same rule meets a live
feed, the two can separate in small ways that a history file hides: a signal can arrive a moment
later than the replay decided, a session can open without a quote, and the price at which an eligible
decision would have filled can be worse than the one written down. The history file is a tidied
record; it holds the bars that arrived and not the ones that did not.

The counterparty here is not another trader but the market's plumbing: the feed that carries prices
and the venue that quotes them. Nobody is obliged to trade against the candidate, and the feed has no
reason to be considerate - it delivers what it delivers, in the order it delivers it. The belief
behind the shadow period is that this plumbing is worth measuring directly, before a run's
expectations are filed away as a fact about the world.

## An everyday comparison

A commuter checks a printed timetable and then stands on the platform. The timetable is the history:
it records what usually happened. The platform is the live feed: same route, same declared stops, but
the train can arrive two minutes late, a stop can be passed without a call, and a connection the
timetable implies can be missed. Standing there for a week records exactly those small discrepancies
that the timetable cannot show. It records nothing about whether the journey was worth making, and
that is the point of separating the two.

## The rules, step by step

1. Declare four things before the period runs, read from the candidate's own data block rather than
   chosen at run time: how often a signal is produced, the instant within a period a decision is
   taken, the earliest instant a decision may execute at, and how a corporate action is treated.
2. Freeze the candidate and record the identity of the specification it was frozen at. That identity
   and the declared observation count belong to the plan, not to the report, because a report that
   chose its own threshold after seeing the period could always be made to pass one.
3. Declare the minimum observation count before the period starts, from the strategy's trade
   frequency. A plan that declares no count, that is, a count below one, is refused by name rather
   than run.
4. Feed observed prices in, compute each fill from the observation, and open nothing: the monitor
   takes the plan, the contract, the replay's expected signal times and the feed's observations as
   arguments. It holds no connection and no client, so a fill is computed rather than obtained.
5. Label every result simulated, wherever it is shown, because a simulated fill is never an observed
   one.
6. For each signal the feed delivers, record six things: the signal time, whether it intended an
   entry, the eligible fill time, the simulated fill price, the modelled cost, and the cost the
   observed prices and quotes imply.
7. Compare the replay's reading with the shadow run's reading in two separate halves: the declared
   fields must match, name and value together, and the pair of instants must match, the decision
   instant and the eligible fill instant.
8. Treat a signal that is not an instant the replay decided at, or a duplicate of one, as a
   discrepancy, and record how far it sits from the nearest expected instant. A duplicate is
   reported with a magnitude of zero rather than dropped.
9. A non-fill is an intended entry that produced no fill, and it must carry a reason. A non-fill
   that gives no reason is refused rather than reported, because a report that cannot say why is not
   diagnosable. An observation that intended nothing is neither a fill nor a non-fill, so a quiet
   period is not reported as divergence.
10. Refuse an observation whose eligible fill time is earlier than its own signal time. A fill that
    precedes its own signal is not a divergence but a broken record.
11. Count what happened and report the counts: the eligible decisions, the observations delivered,
    the missing observations, each discrepancy, each non-fill with its reason, the modelled cost, the
    observed cost and their difference.
12. State whether the period is readable, which means only that it reached the declared count, and
    when it did not, state the count it reached against the count declared.

## The maths, with every symbol named

The comparison of two readings is deliberately two conditions joined by "and":

```text
holds = same_fields and same_pair
same_fields = the ordered list of (name, value) pairs is equal on both sides
same_pair   = (decision_at, eligible_fill_at) is equal on both sides
```

- `holds` is true only when both halves agree, which is what "the shadow shares the backtest's
  contract" means.
- `same_fields` compares the four declarations by value, not by name: a contract whose
  `execution_eligibility` differs is a different contract, however the timestamps look.
- `same_pair` compares the decision instant with the eligible fill instant, both whole nanoseconds.
- The line means: a run declaring different things is not shadowing this candidate, and a run
  declaring the same things but recording a different instant saw a late or early delivery.

The counts are small and exact:

```text
reached              = observed_decisions = the number of observations delivered
readable             = reached >= minimum_observations
missing_observations = how many expected instants the feed never delivered a signal for
cost_difference      = observed_cost - modelled_cost
diverged             = any discrepancy, or any non-fill, or missing_observations > 0
```

- `reached` is the count the report states, which is simply how many observations arrived.
- `minimum_observations` is the count the plan declared before the period started.
- `readable` is a precondition for reading the report, never a measure of quality.
- `missing_observations` counts expected instants absent from the set of delivered signal times.
- `modelled_cost` and `observed_cost` each add the values present, starting from zero.
- `diverged` is true when any discrepancy, any non-fill or any missing observation was found. The
  cost difference is not part of it: a period can be not diverged while the observed cost differs.

## A worked example

One candidate trades once a week. Its declared eligibility rule is that a decision taken at a
session close may fill at the next observed session open, so a signal at second 0 is eligible at
second 86400, which is one day later (86400 seconds in a day). The numbers below are made up but of
the size these quantities take, and the times are seconds from the start of the period, though the
code stores them as nanoseconds.

| #   | Signal time (s) | Intended entry | Eligible fill time (s) | Fill price | Modelled cost | Observed cost | Reason if no fill                               |
| --- | --------------- | -------------- | ---------------------- | ---------- | ------------- | ------------- | ----------------------------------------------- |
| 1   | 0               | yes            | 86400                  | 25.14      | 0.50          | 0.62          |                                                 |
| 2   | 604800          | yes            | 691200                 | 25.40      | 0.50          | 0.48          |                                                 |
| 3   | 1209600         | yes            | 1274400                |            |               |               | the feed delivered no quote at the session open |
| 4   | 1814400         | no             | 1900800                |            |               |               |                                                 |
| 5   | 2419200         | yes            | 2505600                | 25.02      | 0.50          | 0.55          |                                                 |

Take the first row from signal to fill. The signal arrives at second 0 and intends an entry. The
declared rule selects the next observed session open, second 86400, as the eligible fill time. The
simulated fill is computed at 25.14. The recorded cost configuration models 0.50 for that fill, and
the prices and quotes that were actually observed imply 0.62. The difference for this one
observation is 0.62 minus 0.50, or 0.12.

Now the whole period. The replay was expected to decide at five instants, seconds 0, 604800,
1209600, 1814400 and 2419200, and the feed delivered an observation for each, so nothing is missing
and no signal time is unexpected. Observation 3 intended an entry and did not fill, and it carries
its reason, so it is a non-fill rather than a quiet row. Observation 4 intended nothing, so it is
neither a fill nor a non-fill.

```text
eligible_decisions    = 5
observed_decisions    = 5
reached               = 5
missing_observations  = 0
signal_discrepancies  = 0
non_fills             = 1
modelled_cost         = 0.50 + 0.50 + 0.50            = 1.50
observed_cost         = 0.62 + 0.48 + 0.55            = 1.65
cost_difference       = 1.65 - 1.50                   = +0.15
diverged              = True, because one non-fill was found
```

With a declared minimum of four observations, `readable` is true, because five reached. Had the plan
declared six, the report would say the period is not readable and give the reason "the period reached
5 observations against the 6 declared before it started". The cost difference of +0.15 appears in the
findings as a line saying the observed cost differed from the modelled one, and it does not by itself
make the period diverged; the non-fill does that.

Finally, the parity half. Suppose the replay declared `next_session_open` as its execution
eligibility and the shadow run declared `same_session_close`, while recording the same pair of
instants. The timestamps agree, but `holds` is false, because `execution_eligibility` is named among
the differences. The two runs would not be comparable even though they look alike.

## What the research actually found

There is no external paper behind this stage. Its evidence is the repository's own tests and records,
and this page reports those rather than a measured edge.

The unit tests in [test_shadow.py](../../../python/tests/unit/discovery/test_shadow.py) check the
declared field set, the parity rule, the divergence report, a readable period and a refused one. The
integration fixture in [test_discovery_shadow.py](../../../python/tests/integration/test_discovery_shadow.py)
records the field set and timestamps of the backtest a shadow run shadows, where a changed
declaration reports `execution_eligibility` as the differing field and a shifted reading reports that
the timestamps differ. It measures a late signal with a magnitude, one non-fill with its reason, a
modelled cost of 2.00 against an observed cost of 2.25, a difference of 0.25, a period marked
diverged, and a period readable against three but not against one more than the expected count.

The acceptance record is [the acceptance bundle](../../../python/tests/acceptance/discovery_acceptance_bundle.json).
The criterion that a replay and a shadow run share one temporal contract passes, and so does the
criterion that shadow divergence is measurable and reported as divergence rather than as a verdict.

The limits are recorded rather than implied. [The implementation plan](../../../docs/design/strategy_discovery_implementation.md)
carries the finding that the shadow half is fed observed bars rather than bound to a live adapter,
because the design names no adapter and its matrix records the choice as a platform decision, so the
report is evidence about integration and operational behaviour and not about profitability. The same
plan states that the declaration that the count was set before the period is procedural, and that the
checkable part is that the plan's identity travels with the report. The design adds that meeting a
pre-registered count is a precondition for reading a report rather than evidence of an edge, and that
a few weeks over a weekly strategy yields a handful of observations, so the purpose is operational
divergence.

## How this project relates to it

The monitor is [shadow.py](../../../python/nautilus_trader/discovery/shadow.py). `TemporalContract`
holds the four declared fields and can be read off a specification's data block; `TemporalReading`
holds one decision's pair of instants; `temporal_parity` compares two readings and names the fields
that differ. `ShadowPlan` holds the frozen candidate's identity and the declared observation count.
`ShadowObservation` is the six-field record, and `ShadowReport` carries every count above plus one
`findings()` line for each divergence. The entry point, `shadow_period`, is a pure function: it takes
the plan, the contract, the replay's expected signal times and the observations, and returns a
report, opening no connection and holding no client.

A reader who does read code can run the two test files, which is where the behaviour above is
demonstrated:

```bash
python/.venv/Scripts/python.exe -m pytest python/tests/unit/discovery/test_shadow.py python/tests/integration/test_discovery_shadow.py
```

The environment this stage would run in, and the absence of any order-submission path, belong to the
boundary page of this group. What the candidate does with its signals belongs to
[One executor](../one-executor/README.md), and the state before a shadow may run to
[the admission gate](../the-admission-gate/README.md).

## Where it goes wrong

- The shadow half is fed observed data rather than bound to a live adapter, so the report cannot show
  the feed's own latencies, drops or re-connections, and the plan records that as a platform decision.
- A handful of observations proves very little: a few weeks over a weekly strategy yields only a
  handful of independent observations, which is why the purpose is operational divergence.
- Readable is not the same as informative. The count can be reached by a period in which nothing
  intended a trade at all, and such a period reports no divergence.
- No divergence is not a pass. The report says signals, timing, delivery and simulated execution
  matched the replay's expectations; whether the rule has any advantage is a different question this
  stage cannot answer, and no field here is a performance statistic.
- The cost difference is outside the diverged test, so a period can be reported as not diverged while
  the observed cost differs from the modelled one, and the difference then appears only as a finding
  line.
- Parity compares declarations by value, so a run with a different `execution_eligibility` is a
  different contract however similar the timestamps look; only the differing-field list says so.

## Try it yourself

You need a spreadsheet and nothing else. The exercise is to see how a report is counted, and how
little a reached count decides.

1. Make a table with these column headings: signal time, intended entry, eligible fill time, fill
   price, modelled cost, observed cost, reason if no fill. Copy the five rows from the worked
   example, keeping the times in seconds.
2. Add a second small table with the replay's expected signal times: 0, 604800, 1209600, 1814400,
   2419200. Count how many of them have a matching signal time in the first table. That is the
   missing-observation count, whose answer here is zero.
3. Add up the modelled costs, skipping blank cells. It should come to 1.50. Add up the observed costs
   the same way, for 1.65. Subtract to get the cost difference of +0.15.
4. Count the rows where intended entry is yes and the fill price is blank. That is the non-fill
   count, and here it is one, so the period is diverged.
5. Change one expected signal time to a value no row matches, for example 3024000, and recompute
   steps 2 and 3: now one observation is missing, and the period is diverged for a different reason.
6. Write a declared minimum of four observations, then of six, and check the reached count of five.

What to notice: the report's numbers are counts and differences, not judgements. A period can be
readable or not, and diverged or not, depending on declarations made before it ran and on what the
feed delivered, not on whether the candidate is good: a period in which nothing traded can still read
as perfectly readable while saying nothing at all.

## Where this came from

- [shadow.py](../../../python/nautilus_trader/discovery/shadow.py), the module contract, the schema
  and the simulated label, the three refusals, the contract, the reading, the parity comparison, the
  plan, the observation, the discrepancy record and the report with its counts.
- [The design](../../../docs/design/strategy_discovery_design.md), Section 11.5 for the shadow stage
  and its observation record, acceptance criteria 13 and 15, and SD13 for the sandbox venue.
- [The implementation plan](../../../docs/design/strategy_discovery_implementation.md), milestone M6
  and its carried findings on the fed observed bars and the pre-registered count.
- [The unit tests](../../../python/tests/unit/discovery/test_shadow.py) and
  [the integration fixture](../../../python/tests/integration/test_discovery_shadow.py).
- [The acceptance bundle](../../../python/tests/acceptance/discovery_acceptance_bundle.json), the
  criterion entries for the shared temporal contract and for reported divergence.

## Words used in this tutorial

- contract: the four declarations a run makes about how often it looks, when it decides, when it may
  execute and how it treats a corporate action.
- diverged: a period is diverged when a signal discrepancy, a non-fill or a missing observation was
  found; the cost difference alone does not make it so.
- non-fill: an intended entry that produced no fill, which must carry a reason or be refused.
- observation: one record of a signal the feed delivered, with its intended entry, its eligible fill
  time, its simulated fill and its two costs.
- parity: whether a replay and a shadow run share both the declared fields and the pair of instants.
- reading: one decision's pair of instants, taken under the contract that was declared.
- simulated fill: a fill computed from an observation rather than obtained from a venue.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
