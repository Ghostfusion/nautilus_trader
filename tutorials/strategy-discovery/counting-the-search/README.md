# Counting the search: the four counts a campaign keeps

Date: 2026-10-16. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Nothing by itself: it acts on a campaign's own record of what was proposed, compiled, run and admitted, and turns that record into four separate numbers                                                                                                                                                                                                                                                                                                                   |
| How often it trades       | Every time the record is read: the counts are folded from the whole event file in one pass rather than incremented in place, so a replayed event cannot move them twice                                                                                                                                                                                                                                                                                                    |
| What you need             | Nothing but this page; the worked example needs only a table of thirteen labelled events                                                                                                                                                                                                                                                                                                                                                                                   |
| Where the rules come from | The one counting fold in [the campaign ledger](../../../python/nautilus_trader/discovery/ledger.py) and the versioned rules in [the implementation plan](../../../docs/design/strategy_discovery_implementation.md) Section 5.3                                                                                                                                                                                                                                            |
| The underlying research   | The multiplicity setting the design reads, gathered in [Overfitting, reproducibility and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md), and the overfitting diagnostic of Bailey, Borwein, Lopez de Prado and Zhu (2015)                                                                                                                                                                                                        |
| How well it held up       | Strong as a definition: one versioned fold turns the campaign's own events into four counts, no method may define its own counting rule, the refusals in both directions are tested, and the correction's inputs are read back from the inventory rather than supplied by a caller; whether the resulting correction is well calibrated is a property of a live campaign, which is exactly why the overfitting probability is reported as a diagnostic and rejects nothing |
| Also appears in           | [The pre-registered gate chain and the pre-campaign admission test](../the-admission-gate/README.md), [Search by evolution](../search-by-evolution/README.md), [Agents that propose](../agents-that-propose/README.md), [What ran versus what was written](../what-ran-versus-what-was-written/README.md) and [Scoring a strategy on its evidence, not its profit](../../project/scoring-evidence-not-profit/README.md)                                                    |

## The idea in one paragraph

A search proposes many candidate strategies, and the same search runs them, so the number that a later
correction needs is not the winner's score but how many attempts were made. This layer keeps one
append-only record of everything a campaign does - every proposal, every compilation, every run that
closes, every run admitted to the selection - and a single versioned fold turns that record into four
separate counts. The counts are kept apart because a proposal is not a configuration, a configuration is
not an execution, and an execution is not a statistical trial, and a method that needs a count reads it
from this fold rather than from a number a caller hands it. Raw and effective counts are separate
numbers: a campaign with dependent attempts must declare an effective count, and one with independent
attempts must not. A diagnostic, the probability of backtest overfitting, is reported beside the counts
but rejects nothing.

## Why anyone believed it

The pressure to report one number is real. The best of many tries is high by construction: if you try a
thousand variations of a rule, the winner of that contest has the highest score whether or not the data
held anything real. If nobody counts the tries, the number looks like evidence of skill. The belief
behind the counting is that a single counter cannot carry all the meanings at once. The number of ideas
proposed, the number of distinct configurations, the number of times a machine ran, and the number of
attempts that entered a statistical comparison are four different questions. The design's sharpest
statistical point is that separate counters are bookkeeping rather than statistics: a thousand
correlated configurations are not a thousand independent experiments, and a crashed execution is not a
completed observation. So the design keeps four counts, derives the statistical one, and pins the rules
that map between them to a version, so that a result records which counting rule produced it.

## An everyday comparison

Think of a kitchen testing recipes. The chef writes down every recipe idea before deciding whether it is
even possible; some are crossed out at once, and those still count as ideas tried. Of the workable ones,
only genuinely different recipes count as distinct, even when one is a renamed copy of another. Every
batch actually cooked counts as a batch, including the ones that burn or never finish. But only the
batches that were tasted and entered the comparison count as tasters' trials, and a batch whose
paperwork was lost does not count at all. Four numbers answer four questions, and mixing them up would
flatter whichever recipe happened to be tasted last.

## The rules, step by step

1. Write down every proposal before checking it, whether it came from a person or a program. Each one
   adds one to the proposal count.
2. Record the refusal too. A proposal that does not compile is still a proposal attempt, and it is
   counted there and nowhere else; it is not a statistical trial.
3. When a proposal compiles, compute its behavioural identity and compare it with the identities already
   seen under the pre-registered equivalence rule. Only an identity that is new under that rule adds one
   to the distinct-configuration count; an equivalent identity adds nothing, and the merge records both
   identities beside it.
4. When a job is submitted, change no count. A queued job is not yet an execution.
5. When an attempt closes - completed, failed economically, errored, timed out or crashed - add one to
   the execution count, whatever the outcome. A deliberate rerun is another execution even though it is
   not a new configuration.
6. Verify the result: the artifact exists, its digest checks out, and the run identity reconciles against
   its receipt. Nothing is counted for any statistical use before it verifies.
7. Admit a verified execution to the selection process. That admission adds one to the statistical-trial
   count, which counts distinct admissions, so a replayed registration still counts once.
8. Never complete an incomplete record by inference. A registration that failed part-way counts nothing,
   now or later.
9. Keep raw and effective apart. If the campaign's attempts are dependent, declare an effective trial
   count; if they are independent, declare none. Both directions are refused rather than defaulted.
10. Report the overfitting diagnostic with its state and its exclusions, and let nothing in the gate
    chain reject a candidate on it.

## The maths, with every symbol named

The fold is four sums over one list of events:

```text
proposal_attempt_count     = the number of `proposal_recorded` events
unique_configuration_count = the sum over `compilation_recorded` of 1 when the identity is new, else 0
executed_experiment_count  = the number of `execution_attempt_closed` events
statistical_trial_count    = the number of distinct idempotency keys among `trial_admitted` events
```

- An event is one written line in the campaign's record; its kind comes from a closed list, so no event
  outside the list can change a count.
- An idempotency key is a name derived from a job's declared inputs, so a resubmission of the same job is
  recognised as the same job rather than as a new one.
- The line means: read the whole record once and add up what each kind contributes, so the four numbers
  are recomputable from the file alone.

One consistency check is built in: a campaign whose trials exceed its executions is refused rather than
reported, because a trial is a verified execution and cannot be more numerous than the executions that
produced it. In the code the check is `statistical_trials <= executed_experiments`.

The effective count and the raw count are related by a declaration, not a formula. When the attempts are
independent, the effective count is the raw count; when they are dependent, it is a number the campaign
declares and it is used wherever a correction needs `N`:

```text
N = statistical_trial_count   when the attempts are independent
N = effective_trials          when they are dependent and one is declared
```

- `N` is the trial count a correction such as the deflated Sharpe ratio deflates against; that derivation
  is on [the evidence-scoring page](../../project/scoring-evidence-not-profit/README.md) and is not
  repeated here.
- The line means: dependent attempts raise the effective count by less than the raw count, so a campaign
  must say how much less, in one direction or the other.

The overfitting diagnostic is a share over combinations. The common index is split into an even number
`S` of blocks, and every half-sized set of blocks is treated once as the in-sample half:

```text
combinations = the number of ways to choose S/2 blocks out of S
PBO          = the share of those combinations whose in-sample best
               ranks below the median out of sample
```

- `S` is the block count; with the design's worked setting `S = 16` the combinations number 12,780, and
  the implementation's default of eight blocks gives 70.
- The line means: if the setting that looked best in one half of the data keeps winning in the other
  half, the share is low; if it usually falls back, the share is high, and a high share is the signature
  of a search that fitted noise.

The matrix is reported in one of four states, and two of them are per-row rather than matrix-wide:

- `ineligible`: the configuration fails a rule fixed before the matrix was built, such as a failed
  data-integrity check, so it is excluded and the rule is named.
- `eligible_but_failed`: it was admissible, and its run failed economically, timed out, or left a missing
  observation; it is excluded from the statistic and kept in the trial inventory with its failure class.
- `valid_with_documented_exclusions`: the surviving matrix still meets the procedure's preconditions, so
  the statistic is computed with its exclusions reported beside it.
- `not_computable`: too much of the matrix is missing, or the surviving rows do not share a common index,
  so no statistic is produced at all.

## A worked example

Thirteen events were written to a campaign's record, in this order. The kinds are the ones the ledger
recognises, and the four right-hand columns are the running counts.

| #   | Event kind                 | Why it was written                        | Proposals | Configurations | Executions | Trials |
| --- | -------------------------- | ----------------------------------------- | --------- | -------------- | ---------- | ------ |
| 1   | `proposal_recorded`        | a first proposal arrives                  | 1         | 0              | 0          | 0      |
| 2   | `specification_refused`    | it does not compile                       | 1         | 0              | 0          | 0      |
| 3   | `proposal_recorded`        | a second proposal arrives                 | 2         | 0              | 0          | 0      |
| 4   | `compilation_recorded`     | it compiles to a new identity             | 2         | 1              | 0          | 0      |
| 5   | `compilation_recorded`     | an equivalent identity merges into it     | 2         | 1              | 0          | 0      |
| 6   | `job_submitted`            | the configuration is queued               | 2         | 1              | 0          | 0      |
| 7   | `execution_attempt_closed` | the attempt closes, completed             | 2         | 1              | 1          | 0      |
| 8   | `registration_verified`    | the artifact verifies against its receipt | 2         | 1              | 1          | 0      |
| 9   | `trial_admitted`           | it enters the selection process           | 2         | 1              | 1          | 1      |
| 10  | `execution_attempt_closed` | a second attempt closes, as a crash       | 2         | 1              | 2          | 1      |
| 11  | `registration_incomplete`  | its registration failed part-way          | 2         | 1              | 2          | 1      |
| 12  | `duplicate_registration`   | a resubmission under an existing key      | 2         | 1              | 2          | 1      |
| 13  | `trial_admitted`           | the same admission arrives again          | 2         | 1              | 2          | 1      |

Read the last row: two proposals, one distinct configuration, two executions, one statistical trial. The
refusal at row 2 raised only the proposal count. The second compilation at row 5 added nothing, because
its identity was not new. The crash at row 10 added to executions, but its incomplete registration at row
11 added nothing and it never becomes a trial. The replay at row 13 is the same admission under the same
key, so the trial count stays at one.

## What the research actually found

The overfitting diagnostic is not invented here. The design cites Bailey, Borwein, Lopez de Prado and
Zhu (2015), who set out the procedure as a way to estimate how often the best of a search is the best
only in the past. The design keeps it a diagnostic rather than a gate for three stated reasons: its own
preconditions are demanding, because it needs a true matrix of observations shared across every
configuration and a complete trial file rather than only the converged runs; its authors list seven
limitations, including that a high value can coexist with many skilful strategies and that it must not be
used as a selection objective; and an independent controlled study, which the design labels unrefereed
and therefore weaker evidence, found it the weakest of three diagnostics at telling strategies with no
edge from strategies with one, with areas under the curve of 0.669 and 0.727 against 0.808 and 0.759 for
the probabilistic Sharpe ratio.

The counting rules themselves are not a measured effect; they are a definition, and the design states
the definitional choice openly. A proposal that never compiled is not a statistical trial because it
produced no return series and cannot inflate the maximum that the correction corrects for. The design
says a reader could reasonably ask for refusals to count, which is why the choice is written down rather
than buried.

The design is also explicit about what it could not verify: the published versions of the
deflated-Sharpe and backtest-overfitting papers are paywalled, so the authors' own preprints were read,
and the page ranges are taken from third-party bibliographies and may be wrong.

## How this project relates to it

The counting rule is one function in [the campaign ledger](../../../python/nautilus_trader/discovery/ledger.py).
`counts_from_events` is the fold: it walks the record once and returns a `Counts` object, whose four
fields are `proposal_attempts`, `unique_configurations`, `executed_experiments` and `statistical_trials`,
tagged with the mapping version `discovery.counts/v1`. `CountName` carries the four exact names a report
uses. `Counts.__post_init__` refuses a negative count and refuses a campaign whose trials exceed its
executions.

Everything that needs a count reads it through one mapping object, `trial_mapping`, which also carries
the admitted trials by key, the attempts that produced no usable return series, the data role each job
ran under, and the declared effective count. The dependence declaration is enforced in both directions:
a dependent campaign that declares no effective count is refused, and an independent campaign that
declares one is refused too. The research family is `research_family_id`, which names one search over one
snapshot under one counting mapping, so a successor campaign inherits what its predecessor spent rather
than starting clean.

The overfitting diagnostic is `evaluate_pbo_matrix` in [the gate service](../../../python/nautilus_trader/discovery/gates.py),
with its four states and its `diagnostic_only` flag; the admission test that consumes the counts is
`assess_admission`. A reader who does read code can run the count tests with:

```bash
python/.venv/Scripts/python.exe -m pytest python/tests/unit/discovery/test_ledger.py
```

## Where it goes wrong

- A single counter would carry several incompatible meanings at once. The number of ideas tried, the
  number of distinct configurations, the number of machine runs and the number of attempts that entered
  a comparison answer different questions, and a report that collapsed them could not be corrected.
- The trial count can be forgotten or mis-declared. The correction needs the number of attempts, and a
  dependent campaign that declares no effective count is refused rather than given a default; a campaign
  that declares the wrong dependence gets a correction built on the wrong number.
- The refusal rule is a definitional choice, not a measurement. Counting refusals as trials would change
  every number, and the design states its choice rather than proving it.
- Digest identity catches the syntactic case and not the semantic one. A recompiled plan with identical
  behaviour, or a renamed family, has a different digest and could look like a new configuration; the
  policy states which differences count, and every merge is recorded rather than performed silently.
- The overfitting diagnostic is biased by an incomplete trial file. A guided search that reports only its
  converged outcomes lowers the value, and a value computed on a conveniently selected subset is worse
  than no value, because it carries the authority of a number. That is why the matrix is evaluated
  against its preconditions before it is computed.
- The per-trial return series and Sharpe estimates are not read from the ledger at this milestone. An
  entry retains the engine's own document, which the Python side cannot deserialise, so the gate consumes
  those inputs as declared evidence; retaining a per-trial series is a later item rather than a change to
  the entries' identity.

## Try it yourself

You need a piece of paper and nothing else. The exercise is to fold a small sequence of events by hand
and watch the four counts separate.

1. Write the numbers 1 to 13 down the left of a sheet. In a second column write these kinds in this
   order: proposal, refusal, proposal, compile, compile, submit, close, verify, admit, close, incomplete,
   duplicate, admit.
2. Keep four running totals, one for each count, and start each at zero.
3. Walk the list once, in order. Add one to proposals on every proposal, and to executions on every
   close. Add one to configurations only on a compile you mark as new, so mark the second compile as not
   new. Add one to trials only on an admit you have not seen before, so the second admit under the same
   name adds nothing.
4. Stop at the end and read your four totals: two, one, two, one.
5. Now change one thing: delete the refusal at step 2 and the incomplete at step 11, and fold again.
   Notice which counts move and which do not.

What to notice: the four numbers are not versions of one number. A refusal moves only the proposal count,
a crash moves only the execution count, and a duplicate moves nothing at all. This is why a report states
all four rather than a single trial count, and why a correction must be told which one it is deflating
against.

## Where this came from

- [The campaign ledger](../../../python/nautilus_trader/discovery/ledger.py), the closed event vocabulary,
  the four count names, the `Counts` dataclass and its consistency check, the `counts_from_events` fold,
  the `trial_mapping` object, the dependence refusals, and the research-family identity.
- [The gate service](../../../python/nautilus_trader/discovery/gates.py), the `PboState` vocabulary,
  `evaluate_pbo_matrix` and its preconditions, the admission assessment, and the `diagnostic_only` flag.
- [The design](../../../docs/design/strategy_discovery_design.md), Section 10.5 for the four quantities
  and the rules between them, Section 7.4 for the rule that only the statistical count reaches the
  correction, Section 7.2 and SD8 for the overfitting diagnostic, SD6 for multiplicity coming from the
  ledger, and Section 7.7 for the versioned mapping from events to each method's inputs.
- [The implementation plan](../../../docs/design/strategy_discovery_implementation.md), Section 5.3 for
  the versioned `discovery.counts/v1` rules, the event table and the five binding rules, Section 5.1 for
  the four states of an unusable matrix, and Section 10 for the M4 findings, including the carried limit
  on the per-trial series.
- [The ledger tests](../../../python/tests/unit/discovery/test_ledger.py), the campaign with a duplicate,
  a crash and a refusal, the replayed registration that counts once, the incomplete record that is never
  completed, and the dependence declaration refused in both directions.
- [The gate tests](../../../python/tests/unit/discovery/test_gates.py), the case that distinguishes the
  four states of the overfitting matrix.
- [The overfitting brief](../../../strategies/books2/28_overfitting_and_research_integrity.md), this
  repository's account of what replication of published findings shows, which is the source of the
  multiplicity setting this page sits in, and `design Section 7.2`, which cites Bailey, Borwein, Lopez de
  Prado and Zhu (2015) for the diagnostic and its limitations.

## Words used in this tutorial

- campaign: one search over one set of observations, kept as one append-only record of everything it does.
- count: one of the four numbers the record is folded into, kept separate so that a proposal, a
  configuration, an execution and a statistical trial are never confused.
- effective count: the number of independent attempts a set of dependent attempts is worth, declared by
  the campaign rather than computed.
- event: one written line in a campaign's record; its kind comes from a closed list, and only some kinds
  change a count.
- fold: reading the whole record once and adding up what each event contributes, so the counts are
  recomputable from the file alone.
- idempotency key: a name derived from a job's declared inputs, so a resubmission is recognised as the
  same job and not a new one.
- probability of backtest overfitting: a diagnostic share saying how often the setting that looked best
  in one half of the data falls back in the other half; it rejects nothing.
- statistical trial: a verified run that entered the selection process; only these are counted for a
  correction.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
