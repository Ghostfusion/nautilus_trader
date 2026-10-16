# The pre-registered gate chain and the pre-campaign admission test

Date: 2026-10-16. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                                                                                               |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Nothing by itself: the chain acts on one candidate strategy's evidence after that candidate has been tested, and the admission assessment acts on a proposed search and the sample behind it                                                                                                                                                                                        |
| How often it trades       | Once per candidate, after its test run; and once per campaign, before the first test run                                                                                                                                                                                                                                                                                            |
| What you need             | Nothing but this page; the worked example needs only a small table of numbers                                                                                                                                                                                                                                                                                                       |
| Where the rules come from | The gate service [gates.py](../../../python/nautilus_trader/discovery/gates.py) and [the design](../../../docs/design/strategy_discovery_design.md) Section 11.1                                                                                                                                                                                                                    |
| The underlying research   | The minimum-backtest-length relation, cited by [the design](../../../docs/design/strategy_discovery_design.md) to Bailey, Borwein, Lopez de Prado and Zhu (2014), and the deflated Sharpe ratio of Bailey and Lopez de Prado (2014)                                                                                                                                                 |
| How well it held up       | Strong as a definition: the chain, its stopping rule, the verdict vocabulary and the admission arithmetic are unit-tested case by case against expected outcomes, and the calibration suite is a fixture set whose answers are known; whether these thresholds separate a realistic false discovery from a real but weak strategy is a property of a live campaign, not of the code |
| Also appears in           | [Scoring a strategy on its evidence, not its profit](../../project/scoring-evidence-not-profit/README.md) and [Attribution of results](../../project/attribution-of-results/README.md)                                                                                                                                                                                              |

## The idea in one paragraph

A search proposes candidate strategies, and before any is believed it must pass a fixed list of six
questions, asked in one order. The reading stops at the first no: a later question that would have
passed does not excuse an earlier failure, and it is written down as never asked rather than as passed.
The answers are a small fixed set of words - the candidate was not entitled to the data it used, it did
not survive its costs, the evidence was too thin to say, or it touched the data kept back for the final
check - and each is recorded with its reason. Before a campaign opens, one more test asks whether the
data can support the size of search proposed and answers admit, narrow or refuse, showing its
arithmetic. None of this makes money or predicts it: the layer exists to stop a search believing its
own luckiest result.

## Why anyone believed it

The pull to accept a good-looking test result is strong. A backtest is a test that applies a strategy's
rules to past prices to see what would have happened, and the best of many backtests looks good almost by
construction: try a thousand variations of a rule and the winner has the highest score whether or not the
data held anything real. The counterparty a candidate must convince is therefore the search itself, which
always offers a winner. The gates add a referee that asks every candidate the same questions, before
anyone sees the score. A fixed, pre-registered list does not find skill; it makes it harder to talk
yourself into a result you happened to get.

## An everyday comparison

Think of a fire inspection before a building may open. The inspector works a fixed list from top to
bottom: exits, alarms, extinguishers, wiring, and so on. The first item that fails stops the certificate,
and the inspector may not skip a failed item because a later one looks fine - a working alarm does not
make up for a locked fire door. The list is published before the inspection, so nobody can quietly
reorder it after seeing which items the building would pass. The pre-campaign test is the same habit
elsewhere: before plans are drawn, a structural engineer says how many floors this ground can carry, so
the building is not designed first and checked afterwards.

## The rules, step by step

1. **G0, data integrity: was the candidate entitled to what it used, and when?** The run must declare a
   price-series statement, use features only when they were available, and reconcile its entitlements. A
   failure is `INVALID`.
2. **G1, economic viability: does it survive its costs and beat a baseline?** The result pays the declared
   fees, spread, slippage and market impact, then faces a registered yardstick and a simple baseline in
   the same universe. A shortfall is `REJECTED`; a missing input is `INCONCLUSIVE`.
3. **G2, walk-forward: does it persist out of sample across time?** The frozen rule is checked on locked
   validation data - kept aside while the rule was built - in several stretches called folds. Too few
   positive folds is `REJECTED`; the wrong data or a missing split is `INCONCLUSIVE`.
4. **G3, statistical credibility: is it distinguishable from the best of a search this size?** The score
   is compared with a correction that counts how many configurations were tried. A failure is
   `INCONCLUSIVE` or `REJECTED`.
5. **G4, robustness: does it depend on a lucky name, period, or parameter value?** Parameters are nudged,
   costs stressed, and the result sliced by name, sector and period. A dependence is `REJECTED`.
6. **G5, holdout readiness: does it hold on unseen data, and is it frozen?** One recorded evaluation on
   the held-back data, a frozen specification and a complete evidence package. The outcome is
   `VALIDATION_PENDING`, or `QUARANTINED` if it touched data it should not have.
7. The six gates run in order and each is a conjunction: a list of conditions that must all hold. The
   loop stops at the first gate whose verdict is not a pass, so no later gate's strength compensates an
   earlier failure, and later gates are reported as absent rather than as passed.
8. Every threshold, cost, baseline and limit is declared once, in a pre-registered policy pinned by a
   digest before the campaign opens; the service reads that policy and nothing else, so a threshold is a
   decision made in advance, not a dial a caller can turn.
9. Every verdict is written to the campaign's record with its reason and the evidence digest. The
   vocabulary is fixed: `PASSED`, `INVALID`, `REJECTED`, `INCONCLUSIVE`, `VALIDATION_PENDING` and
   `QUARANTINED`, and only `PASSED` advances a candidate.
10. A candidate moves through named states - proposed, compiled, tested, validation-pending, validated
    and shadow - under named permissions; a transition not in the table, or performed by the wrong
    actor, is refused, and a terminal candidate is never reopened. A failing verdict is recorded against
    the candidate with its numbers and moves nothing.
11. Before a campaign opens, the admission assessment asks whether the data can support the proposed
    search, compares the sample the plan needs with the sample the data can spare, and returns `ADMIT`,
    `REDUCE` or `REJECT`, always with the arithmetic.
12. When a declared input is missing, a gate returns `INCONCLUSIVE` with the token `inputs_missing` and
    the input's name, and the assessment returns `REJECT`; neither substitutes a default, which would
    look precise while being invented.
13. One diagnostic sits beside the chain and rejects nothing: the probability of backtest overfitting is
    computed against its preconditions and reported with its distribution and its exclusions, but nothing
    in the gate chain reads it.
14. A limit on a post-run quantity - drawdown, turnover, cost ratio - is a gate condition, evaluated once
    the observations exist, rather than a control that acts during the run; a specification presenting one
    as a runtime control is refused. The candidate language itself belongs to a sibling page in this group.

## The maths, with every symbol named

The assessment works in numbers of observations, not money. First, how many it may count:

```text
U = max(0, E - (R + round(E * h)))
```

- `U` is the usable observations, the budget the ceiling may count.
- `E` is the effective number of independent observations, estimated from the return series rather than
  read off a bar count, because overlapping holdings make observations dependent.
- `R` is the observations reserved for the folds and the embargo; `h` is the share reserved for the
  final holdout; `round` rounds to a whole number and `max(0, ...)` floors at zero.
- The line means: keep back the reserved observations and a share for the final holdout, and spend what
  remains.

Then, how many trials the correction must account for:

```text
K = P + A            when the configurations are independent draws
K = X + A            when they are dependent
```

- `K` is the counted trials; `P` is the configurations the campaign plans; `X` is the effective trial
  count used when they are dependent; `A` is the trials the family already spent against the same data.
- The line means: a raw count of dependent tries overstates how many independent attempts happened, and
  earlier campaigns against this data are not forgotten.

The relation is the correction this repository already computes, turned around:

```text
T*(K) = the smallest sample length T at which the deflated Sharpe
        ratio of the target Sharpe reaches the declared significance s
```

- `T*` is the required observations, the ceiling for a trial count `K`; `T` is a candidate sample length
  in periods; `s` is the significance the correction must reach, `0.95` by default.
- The line means: ask how long the sample must be for the declared target advantage to stand out from
  what `K` tries would produce by chance; the code finds `T*` by searching.

Finally the comparison:

```text
if T*(K) <= U                      -> ADMIT
else if some c < P has T*(c) <= U  -> REDUCE to c configurations
else                               -> REJECT
```

- The line means: if the plan fits, the campaign may open; if only a smaller search fits, it narrows to
  the largest count that does; if no count fits, the answer is a refusal.

Three thresholds the policy declares with a default: the deflated Sharpe floor is `0.95` for G3, the
share of positive walk-forward folds required is `0.5` for G2, and the share of closed attempts that may
produce no usable return series before the count is `INCONCLUSIVE` is `0.25` for G3.

## A worked example

A campaign proposes a weekly search over a few years of daily bars. The declared inputs are these; the
numbers are made up but of the size these quantities take.

| Input                                           | Value |
| ----------------------------------------------- | ----- |
| Effective independent observations `E`          | 300   |
| Observations reserved for folds and embargo `R` | 0     |
| Share reserved for the final holdout `h`        | 0.20  |
| Configurations the campaign plans `P`           | 45    |
| Trials the research family already spent `A`    | 0     |
| Target Sharpe per period                        | 0.15  |
| Significance `s`                                | 0.95  |
| Assumed cross-trial dispersion                  | 0.02  |

The assessment computes the budget, following the formula exactly:

```text
U = max(0, 300 - (0 + round(300 * 0.20)))
  = max(0, 300 - (0 + 60))
  = 240
K = 45 + 0 = 45
```

The relation then gives the ceiling for these 45 tries, and the comparison settles it:

```text
T*(45) = 248 > 240   -> the plan does not fit
```

Because the plan exceeds its ceiling, the assessment looks for the largest count that would fit. From 44
downwards, the first count whose ceiling is at or below 240 is 36, so the outcome is `REDUCE` and the
report names 36 configurations rather than 45. Had the campaign proposed 30 configurations, the ceiling
would be `T*(30) = 234`, which fits, and the outcome would be `ADMIT`.

Now the missing-input rule, in the same example. If the configurations had been declared dependent and no
effective trial count had been supplied, `K` could not be worked out at all. The assessment then returns
`REJECT` with no ceiling and the reason "the configurations are dependent and no effective trial count is
estimated". It does not guess a count to keep the arithmetic going.

## What the research actually found

The relation the assessment inverts is not invented here. The design cites Bailey, Borwein, Lopez de
Prado and Zhu, "Pseudo-Mathematics and Financial Charlatanism" (2014): the expected maximum of a number
of skill-free Sharpe ratios grows roughly with the square root of twice the logarithm of the number
tried, so a large enough search finds a large Sharpe ratio even in data with nothing in it. That paper's
worked example states that with five years of data no more than 45 independent configurations should be
tried. The correction inverted is the deflated Sharpe ratio of Bailey and Lopez de Prado (2014), whose
threshold is the expected best of the tries.

What the design does with those numbers matters as much as the numbers. It states that the
45-configuration example is a conditional reference case for five years of weekly equity data at a
stated target, not a universal cap: every term is an assumption a different sample, horizon or target
changes. That is why the gate computes its own ceiling from declared inputs, and why the report carries a
sensitivity range rather than a single verdict.

The overfitting diagnostic is reported with the same caution. It comes from Bailey, Borwein, Lopez de
Prado and Zhu (2015), and the design keeps it a diagnostic: its preconditions are demanding, its authors
list seven limitations, and an independent study found it the weakest of three diagnostics. Promoting it
to a gate is not decided on a matrix that reports itself as not computable for realistic inventories.

## How this project relates to it

The rules live in [the gate service](../../../python/nautilus_trader/discovery/gates.py). `GateService`
holds one pre-registered `GatePolicy` and runs the six gates in order, writing each verdict to the
campaign's record and stopping at the first that blocks. `GateName` carries the six questions;
`GateVerdict` carries the vocabulary; `TRANSITIONS` is the lifecycle table as data, and `Lifecycle`
refuses a move the table does not name. The admission test is `assess_admission`, built on
`minimum_track_record_length`; the overfitting diagnostic is `evaluate_pbo_matrix`, with its four states
and its `diagnostic_only` flag.

The behaviour is exercised case by case in
[the gate tests](../../../python/tests/unit/discovery/test_gates.py): a candidate that meets every gate
passes, a chain stops at its first false term, G0 refuses a candidate that used what it was not entitled
to, G5 quarantines a leak, the transition table permits only what it names, a terminal candidate is
never reopened, and the admission relation is checked against expected outcomes. The campaign-level
behaviour is in [the governance fixture](../../../python/tests/integration/test_discovery_governance.py).
A reader who does read code can run the unit tests with:

```bash
python/.venv/Scripts/python.exe -m pytest python/tests/unit/discovery/test_gates.py
```

The record of what was tested is [the acceptance bundle](../../../python/tests/acceptance/discovery_acceptance_bundle.json),
one entry per acceptance criterion with the test that exercises it and the result it observed.

## Where it goes wrong

- The thresholds are declarations, not measurements. The code can show the chain asks the same question
  of every candidate, but whether `0.95` on the deflated Sharpe separates a realistic false discovery
  from a real but weak strategy is a property of a live campaign, not of the code.
- The relation is a significance relation, not a power calculation; the declared power (`0.8` by default)
  is an assumption the assessment states and cannot verify.
- The deflation's per-trial return series and Sharpe estimates are not read from the ledger at this
  milestone; the gate consumes them as declared evidence, so a mis-stated per-trial Sharpe is not yet
  caught by recomputation. Retaining the series is a later item.
- The overfitting diagnostic rejects nothing, so a high value is information rather than a verdict, and
  the decision to make it a gate has not been made.
- A clean pass is not an edge: the gates bound a search's self-deception, and whether anything survives
  costs is answered by the account-level measurements. A missing input produces a refusal rather than a
  number on purpose: a gate returns `INCONCLUSIVE` named `inputs_missing`, and the assessment `REJECT`.

## Try it yourself

You need a spreadsheet and nothing else. The exercise shows how the campaign budget is set by the data
and the plan's own history, not by how good a candidate looks.

1. Make three columns: quantity, value, source. Put in effective observations `E` of 300, reserved
   observations `R` of 0, and a holdout share `h` of 0.20.
2. Compute the usable observations: 300 times 0.20 is 60, add the reserved 0, subtract from 300 to get
   240.
3. Add planned configurations `P` of 45 and already-spent trials `A` of 0 to get the counted trials `K`
   of 45.
4. Given a ceiling of 248 for 45 trials: 248 against 240 leaves the plan 8 observations short, so the
   outcome is a narrowing; given a ceiling of 234 for 30 trials, it fits and the outcome is an admission.
5. Lower the holdout share to 0.10 and recompute step 2: the budget rises to 270, more configurations at
   the cost of less data held back for the final check.

What to notice: the same proposal can be admitted, narrowed or refused depending only on the sample and
on how many tries the search plans or has spent - not on whether the candidate is any good. The budget is
a property of the data and the plan, which is why a campaign cannot set its own ceiling.

## Where this came from

- [The gate service](../../../python/nautilus_trader/discovery/gates.py), the six gates and their
  questions, the conjunction and stopping rule, the verdict vocabulary, the transition table, the
  admission assessment and the overfitting matrix.
- [The design](../../../docs/design/strategy_discovery_design.md): Section 11.1 for the gates and their
  questions; Section 11.4 for the lifecycle; Section 7.4 for the admission test; SD8 for the diagnostic;
  SD10 for the budget as an output; and its bibliography for the papers named above.
- [The implementation plan](../../../docs/design/strategy_discovery_implementation.md), the M4
  governance milestone and the seven findings it carries rather than repairs.
- [The gate tests](../../../python/tests/unit/discovery/test_gates.py) and
  [the governance fixture](../../../python/tests/integration/test_discovery_governance.py).
- [The acceptance bundle](../../../python/tests/acceptance/discovery_acceptance_bundle.json), one entry
  per acceptance criterion.
- Bailey, Borwein, Lopez de Prado and Zhu, *Pseudo-Mathematics and Financial Charlatanism*, 2014,
  [author preprint](https://www.davidhbailey.com/dhbpapers/backtest-pseudo.pdf), the source of the
  minimum-backtest-length relation and the 45-configuration worked example; and Bailey and Lopez de
  Prado, *The Deflated Sharpe Ratio*, 2014,
  [author preprint](https://davidhbailey.com/dhbpapers/deflated-sharpe.pdf), the correction inverted.

## Words used in this tutorial

- admission assessment: the pre-campaign test comparing a proposed search's size with what its sample can
  support, answering admit, reduce or refuse with its arithmetic.
- gate: one of six fixed questions asked of a candidate after its test run, in a fixed order.
- quarantine: the terminal state a candidate enters when it touched the data kept for the final check, or
  when its evidence carries an accounting defect.
- verdict: one of the fixed answers a gate gives; only a pass advances a candidate.
- walk-forward fold: one stretch of locked data used once to check a frozen rule outside the data it was
  built on.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
