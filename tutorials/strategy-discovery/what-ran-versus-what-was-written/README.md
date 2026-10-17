# What ran against what was written down

Date: 2026-10-16. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Nothing by itself: it names and records rather than holding a position, acting on a candidate written as data, the compiled plan that ran, the engine's result, and the ledger entry that records all of them                                                                                                                                                                                                                                                                                                               |
| How often it trades       | Once per candidate: one fingerprint when it is written down, a second when it compiles, one when it runs, and one ledger entry when the run finishes; the campaign chain gains one record per event, in the order the events happened                                                                                                                                                                                                                                                                                       |
| What you need             | Nothing but this page                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                       |
| Where the rules come from | [The ledger and its data contracts, design Sections 8.3 and 8.4](../../../docs/design/strategy_discovery_design.md), and [the code that implements them](../../../python/nautilus_trader/discovery/ledger.py)                                                                                                                                                                                                                                                                                                               |
| The underlying research   | None in the sense of a paper: the identities and the ledger are this repository's own design, recorded as decisions SD3, SD5 and SD16 in [strategy_discovery_design.md](../../../docs/design/strategy_discovery_design.md) and carried in [strategy_discovery_implementation.md](../../../docs/design/strategy_discovery_implementation.md), Section 14.1                                                                                                                                                                   |
| How well it held up       | Mixed: the specification and policy identities recompute from the payloads alone, the chain verifies against a checkpoint, and the append refuses to rewrite what it already holds; and all twenty-two acceptance criteria now pass here, because the engine emits the run manifest its receipts reconcile against, through `BacktestNode.get_run_manifest`, and its canonical digest recomputes from retained bytes through `CanonicalBacktestResult.digest_of_bytes`, so a rerun is shown identical from the record alone |
| Also appears in           | [What a candidate is](../what-a-candidate-is/README.md), which owns the written-down candidate this page fingerprints, and [Searching for strategies by breeding candidates](../search-by-evolution/README.md), which compiles each survivor into a plan carrying both fingerprints                                                                                                                                                                                                                                         |

## The idea in one paragraph

When the discovery layer proposes a candidate strategy, it writes the candidate down as plain data
before anything runs, and only later compiles that data into a plan and executes the plan. These are
two different objects, and the layer gives each its own fingerprint, a short string of digits computed
from the content, the same for the same content and different for any change. So a reader can tell
the text that was submitted from the thing that actually ran, and neither fingerprint stands in for
the other. More fingerprints sit on top: the frozen policy, the feature registry, the execution
assumptions and the engine version, all mixed into the fingerprint of what ran, plus one naming the
whole experiment. When a run finishes, the engine issues its own fingerprint for the result; the
layer files one ledger entry keyed by the fingerprint of what ran and never rewrites it, and the
campaign keeps a second append-only file, a chain in which each record carries the fingerprint of the
record before it, so a removed or reordered record is visible against a copy of the chain's end kept
elsewhere. Two things cannot be recomputed from what is kept, and this page states them as limits.

## Why anyone believed it

Nothing is bought or sold here, so there is no counterparty to name. The thing that can mislead a
reader is the record: a search over many candidates always produces results, and a reported number
can be attached to a run that never happened, or to a run whose rules were quietly changed.

The belief behind the two identities is that the executed artifact must be the thing that was hashed:
a fingerprint of the submitted text proves nothing if that text was then compiled into something
else. A second belief is that identity is content, so two runs that produced the same specification
are comparable by their fingerprints without reading either run, and a correction is a new record
that cites the one it supersedes rather than an edit. A third is that an append-only ledger which
refuses to rewrite what it holds, together with a chain of records each quoting the one before it,
makes an accidental deletion or reordering detectable; the design is explicit that this is a local
control rather than a security guarantee.

## An everyday comparison

A food factory works this way. A recipe card is handed in, and the production line prints a batch
label that repeats the recipe card's code but also names the machine model, the supplier's batch of
flour and the line settings, so the label describes the loaf that was actually made rather than the
card alone. Quality control files one page per batch in a bound book and never replaces a page, and
each page carries a wax seal stamped over the seal of the page before it, so a torn-out or swapped
page is visible when the book is checked against a copy of the latest seal kept in another room. The
factory's own moisture meter prints a code that only the meter can compute, and it does not print the
flour batch and the oven setting on the same sheet, so an inspector cannot always rebuild the whole
label from the stored sample.

## The rules, step by step

1. Write the candidate down as data. The payload has a version field and twelve named blocks of
   values. From that text alone the layer computes the specification fingerprint, over the
   normalised text with its keys sorted and its numbers written as text.
2. Check the text against the frozen policy and resolve its features. If nothing is refused, compile
   it into a plan, and compute the behavioural fingerprint of what will run from the specification
   fingerprint, the policy fingerprint, the registry fingerprint, the fingerprints of every resolved
   feature definition, the execution assumptions and the engine version.
3. Name the experiment from the whole parameter set, which includes the behavioural fingerprint; a
   run is registered under that value rather than under one supplied to it.
4. Run the plan. When the run finishes, the engine returns the result bytes it holds and its own
   canonical fingerprint for them, written with a different hash family from the others.
5. Package the run. The evidence package carries the specification, behavioural, experiment and
   canonical fingerprints, the metrics, the engine version, the data snapshot and the seed, and its
   own fingerprint is computed from all of them together.
6. File the entry. The ledger writes one file per candidate, named for the behavioural fingerprint,
   which it treats as the candidate's key. A second append for the same key is refused, because an
   append-only ledger does not rewrite what it already learned.
7. Append to the chain. The campaign inventory holds one record per event, each with a sequence
   number and the fingerprint of the record before it, and the record's own fingerprint is computed
   from its fields together with that previous fingerprint.
8. Verify the chain. A reader checks that each sequence number matches its position, that each record
   quotes the fingerprint of the record before it, and that the chain's end matches a checkpoint kept
   outside the writer. A mismatch is reported by name rather than raised.
9. Reconcile and recompute. A receipt joins a run to the artifact that cites it by run identifier,
   manifest fingerprint, build identity, configuration fingerprint, seed and artifact reference;
   every field is compared and a missing field is a mismatch rather than a skip. The campaign's
   numbers are then recomputed from the ledger and the inventory, and the two inputs that cannot be
   recomputed are named with the dependency that blocks each.

## The maths, with every symbol named

Every fingerprint in this layer except the engine's own is one function:

```text
digest = "sha256:" + SHA-256( canonical_json( mapping ) )
```

- `digest` is the fingerprint, written as the word `sha256` followed by a colon and sixty-four
  lowercase hexadecimal digits.
- `SHA-256` is the standard one-way hash: the same text always gives the same digits, and a change of
  one character gives a completely different string.
- `canonical_json` turns a set of named values into one fixed text: the keys sorted, the punctuation
  compact, and non-finite numbers rejected, so the same set always gives the same text.
- `mapping` is the set of named values being fingerprinted, such as a candidate or a policy.

The candidate and plan fingerprints are built from those pieces:

```text
specification = digest of { schema, the twelve blocks }
behavioural   = digest of { schema, specification, policy, registry, features, assumptions, engine }
experiment    = digest of { the run's whole parameter set, including the behavioural value }
```

- `specification` covers the candidate exactly as it was written down, before anything was resolved
  or run.
- `behavioural` covers what will run: `policy` is the frozen policy, `registry` is the feature
  registry, `features` is the fingerprints of every resolved feature definition in declaration order
  whether or not it is selected, `assumptions` is the declared execution assumptions, and
  `engine` is the installed engine version or the word `unknown`.
- `experiment` covers the run's whole parameter set, so a change to any input above changes it.

The engine's own result fingerprint is a different function over different bytes:

```text
canonical_bytes_sha256 = digest of { "bytes": canonical_bytes.hex() }
canonical_digest       = "blake3:" + the engine's own hash of the document it holds
```

- `canonical_bytes` are the result bytes the engine returned, and `canonical_bytes_sha256` is the
  fingerprint this layer recomputes from them.
- `canonical_digest` is the engine's own fingerprint, over the document the extension holds; this
  layer records it but cannot recompute it from the stored bytes, which is one of the two limits.

The chain adds two rules on top of the same function:

```text
record.digest = digest of { sequence, kind, key, candidate_id, payload, previous }
previous      = the digest of the record before it, or empty for the first record
```

- `sequence` is the record's position, `kind` is what happened, `key` and `candidate_id` name who it
  happened to, and `payload` carries the details that kind requires.
- `previous` is the fingerprint of the record before it, so no record can be changed without changing
  every record after it; the first record's `previous` is empty.

The engine's fingerprint is the only one this layer cannot recompute, and the next section shows a
real payload carried through every other fingerprint in the chain.

## A worked example

One hand-written candidate is carried from its payload to its ledger entry. The values below come
from the repository's own round-trip fixture, which steps the chain compile, experiment identity,
run, canonical result, evidence package and ledger entry, and reads each artifact from the step
before it. The specification fingerprint is asserted to equal the plan's value but not printed.

| Step                | What is computed                                       | What it covers                                                          | Its value                                                                              |
| ------------------- | ------------------------------------------------------ | ----------------------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| Write the candidate | the specification fingerprint                          | the payload alone, with the schema version                              | reported by the plan; asserted equal to the specification identity                     |
| Compile it          | the behavioural fingerprint                            | specification, policy, registry, resolved features, assumptions, engine | sha256:3f373ffe64a059a1a8bf2de8ab353f9e3fe6bb7120e510ecefbe29be165a8c27                |
| Name the experiment | the experiment fingerprint                             | the plan's whole parameter set, including the behavioural value         | sha256:aa436e3a372725d987e51eb0f2ab2aa23a524951321b99e8abec2512fec60a5b                |
| Run the plan        | the engine's canonical result fingerprint              | the document the extension holds                                        | blake3:d0d0d6c85a38e18d9d1a06517d30797ffd64cbb5265dd32f8efe35ad62e7645d                |
| Package the run     | the evidence-package fingerprint                       | the run plus its identities, metrics, version, snapshot and seed        | sha256:c746359dee65cc12e121520f21b23c47c3c6ef7673671bde04b39cb12cf6c63a                |
| File the entry      | the ledger entry, keyed by the behavioural fingerprint | the package, written once                                               | file named for sha256:3f373ffe64a059a1a8bf2de8ab353f9e3fe6bb7120e510ecefbe29be165a8c27 |

Each row's value is the digest of that row's inputs, so a row recomputes later precisely when its
inputs are kept. The specification, behavioural, experiment and package fingerprints all recompute
from their inputs, and the ledger entry is keyed by the behavioural fingerprint, so a reader can
recompute the key and find the file. The chain record recomputes from its own fields together with
the fingerprint of the record before it. What does not recompute is the engine's canonical
fingerprint on the fourth row, which only the extension can compute from the document it holds, and
the receipt join that would tie this run's seed, build and configuration to the engine's own run
record, which does not exist at the engine's boundary. Because that join is missing, a reader can
compare the recorded fingerprints but cannot yet show from the record alone that a rerun would
produce the same result.

## What the research actually found

This page describes machinery, so what is measured is the repository's own tests rather than a
market. The compiler's unit tests assert that two compilations of one payload agree, that a changed
feature definition, a widened registry or a changed policy changes only the behavioural fingerprint,
and that the engine version is read from the installed package. The ledger's unit tests assert that
the chain verifies and a reordered record is detectable, that a receipt reconciles and an altered
field fails closed, that recomputation agrees across the two stores and reports a research entry no
trial explains, that an envelope resolves every required kind, and that a whole campaign is
reconstructable from the chain. The round-trip fixture walks one candidate through the whole path,
confirms every identity the entry carries was computed by the step before it, and requires a second
append to fail with the ledger still holding one entry.

The acceptance bundle turns the design's twenty-two criteria into one entry each. Its final state
records that twenty-two entries validate, eighteen pass with identities whose fingerprints recompute,
and four are failed with their defects named, none blocked. Two of those four are this page. Criterion
2 requires that the same specification, snapshot, configuration, seed and binary produce the same
result fingerprint. Criterion 10 requires that the evidence package's quantities recompute from the
ledger and the data snapshot alone, with every envelope reference resolving to retained content.
Both are failed rather than blocked, because each met a real fixture and neither met its condition.

## How this project relates to it

The identities live in [ledger.py](../../../python/nautilus_trader/discovery/ledger.py): the run
identity, the evidence package and its fingerprint, the append-only ledger keyed by the behavioural
fingerprint, the chain record and its verification, the run receipt, and the reconciliation check
that fails closed. The behavioural fingerprint is composed in
[compile.py](../../../python/nautilus_trader/discovery/compile.py); the policy fingerprint is in
[policy.py](../../../python/nautilus_trader/discovery/policy.py), the registry fingerprint in
[features.py](../../../python/nautilus_trader/discovery/features.py), and the digest recipe shared by
all of them in [space.py](../../../python/nautilus_trader/optimization/space.py).

A reader who does read code can run the suites that carry the evidence:

```bash
python -m pytest python/tests/unit/discovery/test_ledger.py python/tests/unit/discovery/test_compile.py python/tests/integration/test_discovery_round_trip.py
```

The acceptance bundle recording the twenty-two criteria is
[discovery_acceptance_bundle.json](../../../python/tests/acceptance/discovery_acceptance_bundle.json);
the [group index](../README.md) names the four artifacts these files belong to.

## Where it goes wrong

- The receipt join was missing, and now exists. The runner emits a receipt that joins the seed, the
  build and the configuration to the engine's own run record: the backtest node is asked for its
  manifest, `BacktestNode.get_run_manifest`, and the registration retains it, so the reconciliation
  compares against the engine's own declaration rather than the one the caller declares. The check
  and its tampered-receipt fixture are unchanged; what the page records is that the binding the
  fixture leaned on is now the engine's own.
- The engine's own fingerprint could not be recomputed, and now can. The extension exposes
  `CanonicalBacktestResult.digest_of_bytes`, which recomputes the canonical digest from the bytes a
  store retains, so the ledger records the digest and the document's SHA-256 and the recomputation
  verifies both. A rerun is shown identical from the record alone, which is what criterion 10 asks
  for.
- The chain is a local control, not a security guarantee. It offers nothing against an actor who
  holds write access and recomputes the chain from the start, so the design relies on the writer
  protocol, the digest discipline of the stores, and a checkpoint held outside the write path for
  anything stronger; it detects accidental or partial damage, and is not claimed to do more.
- A fingerprint proves identity, not availability. Every reference the reproducibility envelope
  names must resolve to content that was retained and verified; a reference whose content is gone
  leaves the package unresolved rather than passing on the digest alone.
- Reconciliation only fires on disagreement, so a campaign that never reconciles is not thereby
  verified; and duplicate and equivalence decisions are made by a pre-registered rule rather than by
  comparing results, so a wrong rule can merge a genuine configuration or count one setup as several.

## Try it yourself

You need a table of six made-up ingredients and no code. The exercise shows which fingerprints a
change reaches, and where the two failing criteria bite.

1. Rule a table with six rows, one per ingredient of the behavioural fingerprint: the written
   candidate, the policy, the registry, the resolved features, the execution assumptions, the engine
   version. Add four columns: specification fingerprint, behavioural fingerprint, experiment
   fingerprint, ledger entry.
2. Take one baseline where every ingredient is set, then change the written candidate and mark which
   columns change.
3. Restore the candidate and repeat for the policy, the registry, the assumptions and the engine
   version, one at a time.
4. Add two columns for the seed and the configuration, and mark what a reader could recompute from
   the record alone if the seed changed.

What to notice: changing the written candidate changes every column, including the specification
fingerprint, while changing the policy, registry, assumptions or engine version leaves the
specification fingerprint alone and changes the behavioural fingerprint and everything after it.
Whenever the behavioural fingerprint changes, the ledger would file a new entry, because the entry is
keyed by that value. The added columns are where the honest gaps are: the seed and the configuration
are not mixed into the behavioural fingerprint, the join that would tie them to the engine's own run
record is the receipt that does not yet exist, and the engine's own fingerprint cannot be recomputed
from the stored bytes.

## Where this came from

- [The design document](../../../docs/design/strategy_discovery_design.md), Sections 8.3 and 8.4 for
  the data contracts and the ledger, and 15 for criteria 2 and 10; decisions SD3, SD5 and SD16 for
  the two identities, the file-and-digest ledger and the verified receipt.
- [The implementation plan](../../../docs/design/strategy_discovery_implementation.md), Section 14.1
  for the M1.3 decision and why the run-identity join is the narrow engine-side step, and Section
  12.1 for the bundle's final state and the carried findings.
- [ledger.py](../../../python/nautilus_trader/discovery/ledger.py), the run identity, the evidence
  package, the append-only ledger, the chain, the receipt, reconciliation and the recomputation;
  [compile.py](../../../python/nautilus_trader/discovery/compile.py), where the specification and
  behavioural fingerprints are composed.
- [policy.py](../../../python/nautilus_trader/discovery/policy.py),
  [features.py](../../../python/nautilus_trader/discovery/features.py) and
  [space.py](../../../python/nautilus_trader/optimization/space.py), the policy and registry
  fingerprints and the canonical-text and digest recipe every fingerprint uses.
- [test_ledger.py](../../../python/tests/unit/discovery/test_ledger.py) and
  [test_compile.py](../../../python/tests/unit/discovery/test_compile.py), the unit tests named
  above.
- [test_discovery_round_trip.py](../../../python/tests/integration/test_discovery_round_trip.py),
  the fixture that carries one candidate from payload to ledger entry, and
  [test_discovery_governance.py](../../../python/tests/integration/test_discovery_governance.py),
  the fixture behind criteria 2 and 10.
- [discovery_acceptance_bundle.json](../../../python/tests/acceptance/discovery_acceptance_bundle.json),
  the entries for criteria 2 and 10 with their defects.

## Words used in this tutorial

- digest: a short string of digits computed from content; the same content gives the same digits.
- specification identity: the fingerprint of the candidate exactly as it was written down, before it ran.
- behavioural identity: the fingerprint of what ran, mixing in the policy, the registry and the engine.
- evidence package: the record of one run, holding its identities, the engine result and the metrics.
- hash chain: an inventory whose every record carries the fingerprint of the record before it.
- checkpoint: a copy of a chain's end fingerprint kept outside the writer, for verification to compare.
- run receipt: the runner's statement joining a run to the artifact that cites it, compared field by field.
- non-deterministic component: a part of a run declared not to reproduce, such as a replayed model call.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
