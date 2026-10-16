# Auditing a campaign from the ledger alone

Date: 2026-10-16. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                                                                                                     |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Nothing by itself: it acts on one campaign's ledger, reading its chain of recorded events back into the list of candidates the search contained and what became of each                                                                                                                                                                                                                   |
| How often it trades       | On demand, after a search has run; each reading takes the same ledger and the same declared inputs, and is meant to be re-run whenever a reader wants to check it                                                                                                                                                                                                                         |
| What you need             | Nothing but this page; the worked example is a paper exercise over the small table of events below                                                                                                                                                                                                                                                                                        |
| Where the rules come from | The audit surface of [the ledger module](../../../python/nautilus_trader/discovery/ledger.py) and acceptance criterion 16 of [the design](../../../docs/design/strategy_discovery_design.md)                                                                                                                                                                                              |
| The underlying research   | None from the literature: the audit is a bookkeeping rule, stated as the design's own requirement that a whole search be reconstructable after the fact                                                                                                                                                                                                                                   |
| How well it held up       | Strong as a definition: the audit reconstructs the campaign from the chain alone, names candidates the compiled lineage cannot hold, agrees with the gate report's reasons, and is deterministic under the same inputs, and criterion 16 passes; it deliberately does not claim the seed, build and configuration join, which is the open item that leaves two acceptance criteria failed |
| Also appears in           | [What ran, and what was written down](../what-ran-versus-what-was-written/README.md) and [The pre-registered gate chain and the pre-campaign admission test](../the-admission-gate/README.md)                                                                                                                                                                                             |

## The idea in one paragraph

After a search finishes, the winner is easy to read and the rest of the search is easy to lose. The
audit is the reading that puts the whole search back: it takes the campaign's ledger, the record of
everything the campaign wrote down, and rebuilds the list of candidates that existed, which ones ran,
which were judged, under which policy, and why each was turned away. It writes nothing new. It is a
derived view, meaning it reads the chain of events, the lineage, the attempts and the gate verdicts,
and states what they already show. It folds the events itself rather than trusting any counter the
ledger keeps. And because it is built only from what was recorded, it can be run again over the same
ledger with the same declared inputs and come out the same.

## Why anyone believed it

The pull is the one a shopkeeper feels at the year end. The sales report lists what sold. It does not
list the stock that came in and went out unsold, the returns, or the deliveries refused at the door. A
reader who trusts only the sales report will misread the year, because a busy shop and a lucky one look
identical on it.

A search has exactly this shape. It proposes many candidate strategies, tests some of them, and keeps
one. The best of many tries looks good almost by construction, so the candidates that were turned away
are the most informative part of the record and also the part most easily lost. A list of the candidates
that compiled loses every candidate that never compiled, which is exactly the set that was refused. The
audit exists so that the record of a search is the whole search rather than the survivors.

The second reason is about trust. A summary counter is one more thing that can be wrong, and nobody can
check it without going back to the events anyway. A reading that rebuilds the list from the events can
be checked against the chain, and giving the whole reading its own code lets two readers compare their
readings instead of their recollections.

## An everyday comparison

A year-end stocktake. The manager could read the till summary and be done, but an auditor instead walks
the shelves and reads the year's delivery notes, adding every item that entered the building and
subtracting every item that left, including the ones sent back to the supplier. The auditor's sheet is
not a new document about the shop. It is built entirely from documents the shop already had, and two
auditors given the same notes produce the same sheet. Its value is not that the auditor is clever but
that the sheet can be checked line by line against the notes.

## The rules, step by step

1. Start from the ledger's chain of events, in the order they were appended.
2. Fold those events yourself rather than trusting any counter. Collect three kinds in particular: the
   refusals the validator recorded, each a code against a candidate; the gate verdicts, each a gate, a
   status and a reason; and the search-policy digests the generations recorded.
3. Collect the attempts from the inventory, ordered by their key. Each attempt names the candidate it
   ran for, how it closed, and whether it entered the statistical inventory.
4. Build the candidate set: the union of the candidates the compiled lineage holds with every candidate
   named by a refusal, a verdict or an attempt, in the order each candidate is first seen. This set is
   wider than the lineage on purpose, and the reason is given below.
5. For each candidate, record its history as separate facts rather than one summary: the refusal codes
   against it, its attempts and how each closed, how many of those attempts were admitted, and its gate
   verdicts with their reasons.
6. Derive one outcome word for each candidate by a fixed rule, stated in the next section.
7. Compare the policy digest the reader is holding with the one the inventory was opened with.
8. Report the build identities the reconciled receipts carry, as they are recorded, beside the engine
   version the reader holds.
9. Give the whole reading a digest, so that a second reader can compare readings rather than memories.

## The maths, with every symbol named

The candidate set is a union taken in a fixed order:

```text
candidates = lineage keys, then refusal keys, then verdict keys, then attempt keys
```

- The lineage keys are the candidates the compiled lineage holds, in the lineage's own order.
- A refusal key, a verdict key or an attempt key is a candidate that one of those records names; each is
  added after the lineage and only when it is not already present.
- The order is therefore first-seen order, and a refused candidate that never compiled appears last.

The outcome rule is a short decision, taken from the top down:

```text
outcome = the last verdict's status   if any gate judged the candidate
        = "admitted"                  otherwise, if it ran and was admitted
        = "ran"                       otherwise, if it ran and was not admitted
        = "refused"                   otherwise, if it was refused
        = "proposed"                  otherwise
```

- The last verdict's status is the status of the most recent gate verdict recorded against the
  candidate, so a verdict outranks any number of attempts.
- Ran and was admitted counts the candidate's attempts that entered the statistical inventory, so one
  admitted attempt is enough.
- Was refused means at least one refusal code stands against it and it never ran.
- Proposed is the leftover: the candidate was written down and nothing further happened to it. A
  candidate that ran and was admitted is not proposed, because the earlier tests catch it first.

The policy comparison is honest about a missing input:

```text
policy_matches = whether the declared digest equals the recorded one   when a digest was declared
               = nothing at all                                        when none was declared
```

- The declared digest is the one the reader passes in; the recorded digest is the one the inventory was
  opened with.
- When the reader declared none, the answer is not a failed match but no comparison at all. The honest
  answer is that nothing was declared, because a policy digest is an input to the audit rather than
  something the ledger can invent.

The reading itself is given one code:

```text
audit digest = a sha256 code over the audit's own plain data
```

- The plain data is the audit written out as a mapping. A different declared policy digest is a
  different statement about the campaign, so it has a different code even over an identical chain.

## A worked example

A campaign recorded ten events. The numbers are made up but of the shape these records take. Read the
chain in order first:

| #   | Event                  | Candidate | Detail                                                                                         |
| --- | ---------------------- | --------- | ---------------------------------------------------------------------------------------------- |
| 1   | search policy recorded | -         | digest sha256:sp1                                                                              |
| 2   | candidate proposed     | alpha     | generation 0                                                                                   |
| 3   | candidate proposed     | beta      | generation 1, parent alpha                                                                     |
| 4   | candidate proposed     | epsilon   | generation 0                                                                                   |
| 5   | candidate proposed     | delta     | generation 0                                                                                   |
| 6   | specification refused  | gamma     | code VALUE_NOT_DECLARED                                                                        |
| 7   | attempt closed         | alpha     | key k1, completed, admitted                                                                    |
| 8   | attempt closed         | beta      | key k2, crashed, not admitted                                                                  |
| 9   | attempt closed         | epsilon   | key k3, completed, admitted                                                                    |
| 10  | gate verdict           | alpha     | gate G3, status INCONCLUSIVE, reason "not distinguishable from the best of a search this size" |

The lineage holds alpha, beta, epsilon and delta, in that order. Gamma is added from the refusal, so it
appears last. The audit reads the chain back into this table:

| Candidate | Generation | Parent | Attempts     | Admitted | Verdict         | Outcome      |
| --------- | ---------- | ------ | ------------ | -------- | --------------- | ------------ |
| alpha     | 0          | none   | k1 completed | 1        | G3 INCONCLUSIVE | INCONCLUSIVE |
| beta      | 1          | alpha  | k2 crashed   | 0        | none            | ran          |
| epsilon   | 0          | none   | k3 completed | 1        | none            | admitted     |
| delta     | 0          | none   | none         | 0        | none            | proposed     |
| gamma     | 0          | none   | none         | 0        | none            | refused      |

Applying the rule to the five rows:

- Alpha was judged, so its outcome is the last verdict's status, INCONCLUSIVE, even though it ran and
  was admitted. The verdict is tested first.
- Beta ran and was not admitted, so its outcome is "ran".
- Epsilon ran and was admitted and was not judged, so its outcome is "admitted".
- Delta is in the lineage and nothing else happened to it, so its outcome is "proposed".
- Gamma was refused and never ran, so its outcome is "refused". It has a row only because the refusal
  names it: gamma never compiled, so the lineage cannot hold it, and a reader listing compiled
  candidates would lose exactly the candidate that was turned away.

The declared policy digest sha256:pol1 equals the recorded one, so the comparison is true. Had the
reader passed no digest, the answer would be nothing at all rather than false. The build identities are
reported as the receipts carry them, say one identity, printed beside the engine version the reader
holds; the audit does not check that the two agree. Finally the whole reading is given one code, so a
second reader running the same call over the same ledger gets the same code.

## What the research actually found

This is machinery, so the evidence here is the repository's own acceptance record rather than a
published study. The requirement is design criterion 16: a reader with the ledger, the policy digest and
the engine version can reconstruct which candidates existed, which ran, which were judged, under which
policy, and why each was rejected. One integration fixture exercises it, and the bundle entry for the
criterion is recorded as passing, with its identity recomputed.

The fixture builds a small campaign and checks the reconstruction question by question. It finds three
candidates in the reading - the compiled one, its descendant, and the refused one that the lineage does
not hold - and the refused candidate carries no attempts and the outcome "refused". One attempt closed
"completed" and was admitted, while the descendant's attempt closed "crashed" and was not, so one
admitted trial stands against one incomplete attempt. The gate reasons read back from the ledger equal,
word for word, the reasons the gate service returned at the time, so the audit agrees with the report
rather than paraphrasing it. The policy digest is corroborated, the search-policy digest is reported,
and the chain is verified against the audit's own head. A second case in the same fixture checks the
property that makes the reading usable as evidence: the same ledger and the same declared inputs give
the same digest and the same plain data, while a different declared policy gives a different digest.

Two things are worth stating plainly. First, the reconstruction reports what the ledger retained. It
does not recompute the engine's own canonical digest, and it does not perform the seed, build and
configuration join of decision M1.3, which the plan records as an open item; that missing join is the
same piece of work behind two of the four acceptance criteria the bundle records as not passing, so the
audit is complete over the chain it reads and silent over the join it cannot make. Second, the four
counts the reading carries are the subject of another page in this group; this page reports that they
travel with the reading and does not explain them.

## How this project relates to it

The audit lives in [the ledger module](../../../python/nautilus_trader/discovery/ledger.py).
`audit_campaign` takes a campaign's inventory and two declared inputs, a policy digest and an engine
version, and returns a `CampaignAudit`. That object holds the campaign and family identifiers, the event
count, the chain's head, the counts, the declared and recorded policy digests, the search-policy digests,
the engine version, the build identities, and one judgement per candidate. `CandidateJudgement` holds one
candidate's separate facts and its derived outcome, and the private helper `_judgement_outcome` states
the outcome rule in one place. `policy_matches` and `judged` are properties, and `digest()` gives the
whole reading its code.

The behaviour is exercised in [the audit fixture](../../../python/tests/integration/test_discovery_audit.py)
and [the ledger unit tests](../../../python/tests/unit/discovery/test_ledger.py). A reader who does read
code can run the fixture with:

```bash
python/.venv/Scripts/python.exe -m pytest python/tests/integration/test_discovery_audit.py
```

The record of what was tested is [the acceptance bundle](../../../python/tests/acceptance/discovery_acceptance_bundle.json),
one entry per criterion; the entry for criterion 16 is `FX-M6-4`. The plan places the audit as a
derived view rather than a new module, on the ground that the layout already owns a derived artifact in
the bundle.

## Where it goes wrong

- The audit can only report what the ledger retained. It adds no record and can invent none, so an
  event that was never written down leaves no trace, and a refusal that never reached the ledger is a
  candidate the audit cannot name.
- It does not verify the chain. The fixture verifies the chain separately, against a checkpoint held
  outside it; the audit simply reads the events, so a reader who wants a verified reading has to verify
  as well.
- It does not compare the engine version with the receipts' build identities. It reports the identities
  as recorded and the version as declared, side by side, and leaves the comparison to the reader,
  because the join that would make the comparison meaningful is the open item named above.
- A policy answer of "nothing declared" is honest and easy to misread. The comparison returns nothing
  at all when the reader declared no digest, which is neither a pass nor a failure; a report that
  printed it as false would be wrong.
- Determinism is only over the same inputs. A different declared policy digest produces a different
  reading and a different code even over an identical chain, which is deliberate, because the declared
  policy is part of the statement the reading makes.
- It summarises nothing away but it also explains nothing. The outcome word is a one-line answer derived
  from the fields; every fact behind it stays in the record, so a reader who wants to know why a
  candidate was refused must read the reasons, not the outcome word.

## Try it yourself

You need a pen and the ten-event chain from the worked example, or one you invent in the same shape.
The exercise is to build the audit's table by hand and watch which facts decide each outcome word.

1. Write the events in order in a list, with one column for the candidate each event names.
2. Take the candidates that were proposed as your first rows, in the order proposed. Then append any
   candidate named by a refusal, a verdict or an attempt that is not already a row.
3. For each row, fill four columns: attempts and how they closed; how many were admitted; whether a gate
   judged it; whether it was refused.
4. Apply the rule from the top: a verdict first, then admitted, then ran, then refused, then proposed.
   Write the outcome word.
5. Now change one event: give beta a gate verdict whose status is REJECTED, and re-derive its row. The
   outcome word changes from "ran" to REJECTED even though the attempt still crashed.

What to notice: the outcome word is decided by which facts are present, not by how many there are. One
verdict overrides any number of attempts, and a candidate can appear in the reading with no attempt at
all. Notice also that the refused candidate has a row only because a refusal named it: delete that event
and the candidate vanishes from the reading, even though nothing else about the campaign changed.

## Where this came from

- [The ledger module](../../../python/nautilus_trader/discovery/ledger.py), the audit surface:
  `audit_campaign`, `CampaignAudit`, `CandidateJudgement` and the outcome rule they share.
- [The audit fixture](../../../python/tests/integration/test_discovery_audit.py), the reconstruction of
  a completed search and the determinism of the reading, with its candidate order and its refused case.
- [The ledger unit tests](../../../python/tests/unit/discovery/test_ledger.py), the verdict reader and
  the unit case for the whole-campaign reconstruction.
- [The design](../../../docs/design/strategy_discovery_design.md): acceptance criterion 16, which states
  the requirement; Section 8.4, for the ledger being append-only files with a derived, disposable index;
  and Section 15.1, for the bundle being a derived artifact of the ledger.
- [The implementation plan](../../../docs/design/strategy_discovery_implementation.md), the M6
  validation milestone, where the audit is placed as a derived view and where the criteria left failed
  by the same missing run identity are recorded.
- [The acceptance bundle](../../../python/tests/acceptance/discovery_acceptance_bundle.json), the entry
  for criterion 16, fixture `FX-M6-4`, recorded as passing.

## Words used in this tutorial

- audit: the reading that rebuilds a whole search from the ledger after the fact, adding no record of
  its own.
- candidate set: every candidate the ledger names, whether it compiled or was turned away.
- derived view: a reading built entirely from existing records, which is not itself a new record.
- digest: a short fixed-length code computed from a piece of content, so two contents can be compared
  by their codes.
- judgement outcome: a candidate's one-word fate, derived from its verdicts, attempts and refusals by a
  fixed rule.
- ledger: the append-only record of everything a campaign wrote down, keyed by digest.
- policy digest: the identity of the frozen policy a campaign ran under, declared by the reader rather
  than stored in the ledger.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
