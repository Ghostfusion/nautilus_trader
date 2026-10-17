# Two kinds of held-back data: the roles, the reservation and the refusals

Date: 2026-10-16. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                                                                                                                                                                                                |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| What it trades            | Nothing by itself: it acts on the data a campaign may read, labelling every result with a role and recording every read of the confirmation set                                                                                                                                                                                                                                                                                                                                      |
| How often it trades       | A role label is written once per result; the confirmation reservation is taken once per campaign; an access record is written on every read, permitted or refused                                                                                                                                                                                                                                                                                                                    |
| What you need             | Nothing but this page; the worked example needs only a table of reads                                                                                                                                                                                                                                                                                                                                                                                                                |
| Where the rules come from | [The custody module](../../../python/nautilus_trader/discovery/custody.py) and [the design](../../../docs/design/strategy_discovery_design.md) Sections 8.6 and 11.3                                                                                                                                                                                                                                                                                                                 |
| The underlying research   | None of the published kind: the design records the rule as its own, written after two internal reviews of an earlier draft, and states that a walk-forward split does not by itself make a result independent of the search that fits it                                                                                                                                                                                                                                             |
| How well it held up       | Mixed: the locked-validation surface is enforced by a role label that both the walk-forward gate and the per-lineage rule read, and the confirmation reservation is spent once, never released, and recorded with every access including a refused one; but only the locked-validation surface has an enforcement path outside the custody module, and a branch inherits its family's spent reservation through a declared input rather than through a store the two campaigns share |
| Also appears in           | [The pre-registered gate chain and the pre-campaign admission test](../the-admission-gate/README.md) and [Search by evolution](../search-by-evolution/README.md)                                                                                                                                                                                                                                                                                                                     |

## The idea in one paragraph

A search tries many candidate rules on the same historical prices, so the data it reads while searching
cannot also be the evidence that it found something rather than a coincidence. This part of the project
gives every result a role: development data may be read while searching, locked validation data only
once a candidate is frozen and only by the validation service, and the final confirmation set once per
campaign for one frozen candidate. A role belongs to a result together with the line of descent that
consumed it, so a stretch of data that shaped one line stops counting as validation for that line,
however it was labelled on paper. The final set is rationed by a small state machine that takes a
reservation, spends it on one read, never releases it, and writes down every read it permits or refuses;
none of this predicts a price, it decides what a campaign may look at and records what it looked at.

## Why anyone believed it

The party on the other side of this rule is not a buyer or a seller. It is the person or program that
wants the candidate to be good: the operator who proposed it, the agent that mutated it, or the gate
that has to decide whether to believe it. That party has both the motive and the means to look, because
looking makes the rule fit better and a rule that fits better looks better. Nothing dishonest has to
happen for the data to be used up; the search has only to read the result and try the next variation.

The belief behind custody is that the accounting can be made mechanical. The design states that retiring
an evaluated candidate does not undo the information released by evaluating several: a reader who has
seen three confirmation results and picked the best has used the confirmation set to select, whatever
the ledger says about each retirement. If the data cannot be kept secret on one workstation, the next
best thing is to make its use visible and rationed.

## An everyday comparison

A school sets one end-of-year paper and keeps it in a sealed envelope. Teachers may use old papers,
homework and practice tests as much as they like while teaching; that is the material the year is built
on. The sealed paper may be opened only after the class list is fixed and only for the one sitting it was
set for. If a teacher opens it in March and teaches to it, the paper stops measuring anything, whether or
not the class passes. A supply teacher who joins later does not get a fresh envelope, because the school
remembers that the class already sat it, and the school logs every request for it.

## The rules, step by step

1. Every result carries one of three roles. **Development** data is read by the search, the agents and the
   gate service, and is what candidate generation, mutation, recombination and selection work over.
   **Locked validation** data is read by the validation service once the candidate or candidate set is
   frozen, and is what the walk-forward gate and the statistical assessment use. **Final confirmation**
   data is read by the validator entry point, once per campaign, for one frozen candidate.
2. The rule making the roles meaningful is about feedback, not an access list: a result whose value
   influenced the next mutation is development data, whatever split it came from, and a fold the search
   optimised against cannot afterwards be presented as independent evidence for the winner.
3. A role attaches to a result together with the line of descent that consumed it, not to a dataset on
   its own: a partition whose result influenced a line is development data for that line and stays locked
   validation for a line that never saw it, and moving a candidate to a new branch does not shed the
   label. Two readers enforce it. The walk-forward gate refuses a result whose role is not locked
   validation, saying that the data role is development or final confirmation and that only locked
   validation data is admissible. The per-lineage rule records the downgrade beside the candidate whose
   result did the influencing, and refuses a frozen candidate presenting a result its own line consumed.
4. The confirmation set is rationed by a campaign-level state machine with six states: searching, frozen,
   confirming, confirmed, withdrawn and exploratory-reuse. It is campaign-level, not per-candidate,
   because evaluating several candidates and keeping the best has already used the data to select.
5. While **searching**, generation, mutation, recombination and selection over development results are
   permitted and any read of the confirmation set is refused. **Freezing** designates one candidate and
   takes a reservation over the dataset for it, recording its specification, policy and evidence digests;
   it is refused unless the campaign is searching, and refused when the family has already spent a
   reservation against that dataset. Once frozen, that candidate may not be mutated and the policy may
   not change; another candidate is unaffected, because the freeze binds one candidate and not the search.
6. The reservation is spent once and never released. No operation gives it back, and the refusal says so:
   the family's confirmation reservation is spent and nothing releases it.
7. A read is permitted for the frozen candidate under the reservation and consumes it. A read for any
   other candidate is refused unless labelled **exploratory**; a labelled read is permitted every time,
   consumes nothing, and carries the label `exploratory` wherever the result is shown. A read declaring
   it will carry the set's summary statistics into the candidate's search-phase inputs is refused, which
   the design lists as detectable: such a candidate is quarantined rather than confirmed.
8. A refused read is recorded before it is raised, as an access record with its code. The codes are a
   closed vocabulary: `not_frozen`, `already_frozen`, `frozen_candidate_mutated`, `policy_changed`,
   `reservation_consumed`, `reservation_unreserved`, `second_candidate`, `verdict_candidate_mismatch`,
   `exploratory_label_required`, `confirmation_access_without_reservation`,
   `summary_statistic_in_parameters` and `not_confirmed`.
9. The verdict is bound to the frozen candidate's identity. It is recorded only after the holdout has
   been read for that candidate under a matching policy, and citable only while the campaign is
   confirmed; withdrawing clears the verdict and keeps the spent reservation. Every state change and
   access is written through the inventory as one record rather than four claims that could drift apart.

## The maths, with every symbol named

There is no probability here, only identities and counts. The reservation's identity is a digest of the
five things that define it:

```text
reservation_id = digest_of(family_id, campaign_id, candidate_id, dataset, policy_digest)
```

- `reservation_id` is a short fixed-length string computed from those five fields, so two reservations
  differing in any one of them get different strings.
- `family_id` names the research family, the campaigns reading one snapshot and selecting through one
  process; `campaign_id` names the campaign holding it; `candidate_id` names the frozen candidate bound
  to it; `dataset` is the dataset's definition digest rather than a file path; `policy_digest` is the
  identity of the policy it will be judged under. The line means the reservation names its family,
  campaign, candidate, data and rules, so it cannot be quietly pointed at another of any of them.

What a successor campaign inherits is the set of reservations its predecessor took:

```text
inherited = the distinct reservation identities in a predecessor's transitions, in the order taken
```

- `inherited` is the list the successor campaign is constructed with, and a non-empty list is what
  refuses it a freeze and the access its predecessor already used.

What one campaign may do with the set, counted: one read for the frozen candidate, which consumes the
reservation; any number of labelled exploratory reads, which consume nothing; and a written record of
every other read, with its refusal code. The set therefore yields one read that can be cited and any
number that cannot, and the only quantity that never moves back is the reservation.

## A worked example

One campaign, one frozen candidate called B1, one replacement called B2. The events are invented, but
each answer and code is the one the code produces for that request in that state.

| Step | Who asks for what                      | State before      | Label | Answer  | Code recorded                     | State after       | Reservation |
| ---- | -------------------------------------- | ----------------- | ----- | ------- | --------------------------------- | ----------------- | ----------- |
| 1    | Freeze B1 as the campaign's candidate  | searching         | no    | allowed | none                              | frozen            | taken       |
| 2    | Read the set for B1                    | frozen            | no    | allowed | none                              | confirming        | spent       |
| 3    | Read the set for B1 again              | confirming        | no    | refused | `reservation_consumed`            | confirming        | spent       |
| 4    | Read the set for B2                    | confirming        | no    | refused | `second_candidate`                | confirming        | spent       |
| 5    | Read the set for B2                    | confirming        | yes   | allowed | none                              | exploratory-reuse | spent       |
| 6    | Read for B1 carrying `{"sharpe": 9.0}` | exploratory-reuse | no    | refused | `summary_statistic_in_parameters` | exploratory-reuse | spent       |
| 7    | Record the verdict for B1              | exploratory-reuse | no    | allowed | none                              | confirmed         | spent       |
| 8    | Cite the verdict for B2                | confirmed         | no    | refused | `verdict_candidate_mismatch`      | confirmed         | spent       |
| 9    | Withdraw B1                            | confirmed         | no    | allowed | none                              | withdrawn         | spent       |
| 10   | Cite the verdict for B1                | withdrawn         | no    | refused | `not_confirmed`                   | withdrawn         | spent       |

The counts, read straight off the table:

```text
reads asked           = steps 2, 3, 4, 5, 6 = 5
reads permitted       = steps 2, 5         = 2
reads refused         = steps 3, 4, 6      = 3
access records        = 2 + 3              = 5
reservations consumed = step 2             = 1
verdicts citable      = step 7, withdrawn at step 9 = 0
```

Three details are rules rather than accidents. Step 3 is refused with `reservation_consumed` because B1 is
the frozen candidate and the question is no longer whose read it is but whether anything is left to
spend; step 4 is refused with `second_candidate` even though the reservation is spent, because the read
is for the wrong candidate and that fault is the more specific one; and step 6 is refused before the
reservation is examined at all, so its access record carries an empty reservation identity while the
candidate would be quarantined rather than confirmed.

Step 5 consumes nothing: it is permitted, recorded under the label `exploratory`, and the reservation
stays `spent` from step 2 rather than being newly spent. Step 7 succeeds because recording a verdict only
asks that the reservation is spent and the candidate identity matches, and step 9 keeps the history: the
verdict is gone and the reservation is still spent. A successor campaign built with the list its
predecessor's history yields, which after step 2 is one entry, has its first freeze refused with
`reservation_consumed`, so its state never leaves searching and a read for its own candidate is refused
with `not_frozen`; the honest options for a replacement are newly reserved, unseen data, or a separately
pre-registered protocol.

## What the research actually found

This part of the project was measured rather than argued. The design's twenty-two acceptance criteria
include one exactly for this ground, criterion 9, which asks that the confirmation set cannot influence
selection at campaign level: no mutation, parameter bound, threshold or selection decision derived from
confirmation data, every access recorded against the experiment that made it, and the state machine
enforced rather than described. Its stated mechanism is five negative cases over the confirmation set - a
second candidate from one campaign, a mutation after a confirmation evaluation, a winner selected from
several confirmation results, a new branch expecting fresh access to the same data, and a reused
evaluation presented without its exploratory label - plus the set's path absent from worker environments
and a quarantine fixture whose parameters carry a confirmation statistic. The plan records the criterion
as passed on those cases, the spent reservation, the bound verdict and the worker-environment check, with
thirteen test functions behind it.

What those tests establish is narrow: that in this code, for these inputs, the machine answers with the
code and the record the design names. They do not show that a search is unreachable from the data, that a
determined operator cannot read the file, or that a confirmation result says anything about future
prices.

## How this project relates to it

The rules live in [the custody module](../../../python/nautilus_trader/discovery/custody.py).
`ConfirmationCustody` is the campaign-level machine; `CustodyState` is the six-state vocabulary;
`CustodyRefusalCode` is the refusal vocabulary and `CustodyError` carries the code with the message;
`Reservation`, `AccessRecord`, `ConfirmationVerdict` and `CustodyTransition` are the four records; and
`inherited_reservations` reads what a family has spent.

The roles live in [the ledger](../../../python/nautilus_trader/discovery/ledger.py) as `DataRole`, whose
`admissible_as_validation` property is true only for locked validation. The walk-forward gate in
[the gate service](../../../python/nautilus_trader/discovery/gates.py) returns inconclusive when the role
is not locked validation, and the holdout-readiness gate returns quarantined when the evidence carries a
quarantine reason or the frozen specification identity does not match the one that ran. The per-lineage
half is in [the evolution module](../../../python/nautilus_trader/discovery/evolve.py) as
`PartitionRoles`, whose `informed` method downgrades a partition to development for one line and whose
`as_validation` returns a role mismatch once that line has consumed it. A transition, its reservation
and its access are written through the inventory as a custody event, so the record is
[the campaign ledger](../../../python/nautilus_trader/discovery/ledger.py) rather than a second store.

The behaviour is exercised in
[the custody tests](../../../python/tests/unit/discovery/test_custody.py) and at campaign level in
[the governance fixture](../../../python/tests/integration/test_discovery_governance.py), and the record
of what was tested is [the acceptance bundle](../../../python/tests/acceptance/discovery_acceptance_bundle.json),
one entry per acceptance criterion with the test that exercises it and the result it observed. A reader
who does read code can run the unit tests with:

```bash
python/.venv/Scripts/python.exe -m pytest python/tests/unit/discovery/test_custody.py
```

The record of what was tested is
[the acceptance bundle](../../../python/tests/acceptance/discovery_acceptance_bundle.json), one entry per
acceptance criterion with the test that exercises it and the result it observed.

## Where it goes wrong

- Custody here is not a security boundary, and the module says so in its own first paragraph: what it
  enforces is the record. An operator holding the path and the credentials can read the file outside the
  code, and the design states plainly that no boundary test can make such a machine safe; the operator
  not passing the path to a search process is the procedural half of the rule. A role label describes
  provenance, not intent either, so a search that reads locked validation data without writing a row
  leaves nothing for the label to catch.
- A branch learns what its family spent from a declared input, not from a store the two campaigns share.
  The plan records this as a narrowing, and it means a successor that declares nothing inherits nothing.
- Two codes in the refusal vocabulary have no path that raises them today, `exploratory_label_required`
  and `confirmation_access_without_reservation`; the refusals they would name are reported under other
  codes, so a reader counting codes finds a vocabulary wider than the behaviour.
- The worker check inspects an environment mapping a caller hands it, and the design's worker allowlist is
  declared and checked in `config.py` rather than provisioned: the module refuses an entry naming a live
  adapter and measures a worker's packages against the allowlist, and no module builds the worker
  environment that allowlist describes. The design also asks a configuration loader to refuse a credential
  in a policy file, which the configuration document now does, so the half that had no loader is carried
  by tests of its own.
- Withdrawal keeps the spent reservation, which is the point of it, but it means an error found after a
  read permanently costs the campaign its one confirmation result; nothing restores it, and the design's
  honest alternatives are new data or a new pre-registered protocol.

## Try it yourself

You need a table and nothing else; the exercise is to see how much of this machinery is bookkeeping you
can do by hand.

1. Draw a table with these column headings: step, request, state before, label, answer, code, state after,
   reservation spent.
2. Write the ten rows from the worked example into it, copying each answer and code.
3. Count the rows answered refused and allowed. You should get five and five, because five of the rows are
   not reads at all: one freeze, one verdict, one withdrawal and two citations.
4. Now make step 5 unlabelled. Its answer becomes refused with code `second_candidate`, its state after
   stays confirming, and the campaign has one permitted read instead of two.
5. Add an eleventh row, a successor campaign asking to freeze the same candidate against the same dataset:
   put `reservation_consumed` in its code cell and leave its state cell as searching, because a refused
   freeze never moves the campaign.

What to notice: every question in the table is answered by four things - whether a candidate is frozen,
who the request is for, whether it is labelled exploratory, and whether the reservation is still unspent -
and none of them is a statement about how good the candidate is. The one cell that never changes back is
the reservation, which makes the successor campaign's position different from the first's even when its
candidate is better.

## Where this came from

- [The custody module](../../../python/nautilus_trader/discovery/custody.py): the six states, the refusal
  vocabulary, the reservation and its digest, `read_holdout` and its refusal order, the verdict bound to a
  candidate, the worker-environment check, and `inherited_reservations`.
- [The design](../../../docs/design/strategy_discovery_design.md): Section 8.6 for the three data roles
  and the feedback rule, and Section 11.3 for the state table, the boundary's enforceable and procedural
  halves, the detectable quarantine case, and what boundary tests cannot prove.
- [The plan](../../../docs/design/strategy_discovery_implementation.md): the M4 finding that a successor
  inherits its family's spent reservation through a declared input read from its predecessor's inventory
  rather than from a shared file, the worker allowlist limit, which `config.py` now revises by declaring
  and checking the allowlist, and the criterion 9 entry.
- [The ledger](../../../python/nautilus_trader/discovery/ledger.py), [the gate
  service](../../../python/nautilus_trader/discovery/gates.py) and [the evolution
  module](../../../python/nautilus_trader/discovery/evolve.py): the `DataRole` vocabulary and its
  `admissible_as_validation` property, the walk-forward gate's use of it, the holdout-readiness gate's
  quarantine outcomes, and the `informed` and `as_validation` methods.
- [The custody tests](../../../python/tests/unit/discovery/test_custody.py) and
  [the acceptance bundle](../../../python/tests/acceptance/discovery_acceptance_bundle.json), the
  criterion 9 entry.

## Words used in this tutorial

- access record: the row written for each read of the confirmation set, holding its label or its refusal code.
- confirmation set: the data held back for the one held-back evaluation a promotion decision may cite;
  the code names its result locked validation or final confirmation by the role it carries.
- custody: the bookkeeping that rations the confirmation set, binds its one read to one candidate and keeps
  the record of every attempt, without being a lock on the file.
- data role: the label development, locked validation or final confirmation that travels with a result and
  the line of descent that consumed it.
- exploratory label: the mark on a read that is not the campaign's one confirmation read, which lets the
  result be seen and stops it ever being cited as independent.
- quarantine: the gate outcome for a candidate whose inputs carry a confirmation statistic, reported instead
  of a pass or a fail.
- reservation: the one-shot claim a campaign takes on the confirmation set for its frozen candidate,
  spent by the first read for that candidate and never released.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
