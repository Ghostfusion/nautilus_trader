# Strategy discovery tutorials

Date: 2026-10-16. Revision 1.

This group covers one piece of work in this repository: a layer that proposes candidate strategies,
tests them through the simulation the repository already has, and keeps a record of the whole search
rather than only of the winner. Each page here takes one part of that layer, states its rules in
plain words, shows the arithmetic where there is arithmetic to show, and reports what the
repository's own tests and records found. Nothing here assumes that you have traded, opened an
account or read code.

## What this work is

The work is four artifacts of one subject, and the pages in this group cite all four.

- The design, [Autonomous strategy discovery: design considerations](../../docs/design/strategy_discovery_design.md),
  states the contract, the candidate language, the search's authorities, the gates and the shadow
  stage, and ends with twenty-two acceptance criteria.
- The plan, [Strategy discovery: implementation plan](../../docs/design/strategy_discovery_implementation.md),
  turns that into six milestones, a package layout, a statistical-input decision, an
  acceptance-evidence bundle and the commands that run it.
- The package, [python/nautilus_trader/discovery](../../python/nautilus_trader/discovery/),
  is the code: the policy, the schema, the validator, the compiler, the feature registry, the
  interpreter, the run, the ledger, custody, the gates, the genome, evolution, the agents and the
  shadow.
- The evidence, [discovery_acceptance_bundle.json](../../python/tests/acceptance/discovery_acceptance_bundle.json),
  is one entry per acceptance criterion with the test that exercises it, the procedure that runs it,
  the expected and observed result, the receipts it was produced under and its status.

## What is distinctive about this work

- A candidate is data, not code. A specification is a mapping of blocks with a fixed shape, and no
  part of it is callable, so what a search proposes can be read, hashed and refused without being
  executed.
- The search may propose anything and decide nothing. The thresholds, the holdout and the risk
  bounds are inputs to the campaign rather than choices inside it, and the layer has no order
  submission path at all.
- Every claim is tied to a digest. What was written down, what ran, what the run produced and what
  the ledger recorded each have their own identity, so a reader compares artifacts rather than
  taking a number on trust.
- The measurement is built to refuse. A candidate that does not compile is a recorded refusal
  rather than an experiment, and a page of evidence is reported as a bundle of receipts rather than
  as an assertion.

## The caveats to carry into it

- Nothing here is a profitability claim. The design states that the deliverable is evidence and not
  a return, and it stops at a shadow stage: no order submission, no broker credential, no live
  authority.
- Four of the twenty-two acceptance criteria do not pass, and the pages that touch them say so. Two
  of the four need the same missing piece, a run identity that joins the seed, the build and the
  configuration at the engine's own boundary. One needs the settlement bridge to defer rather than
  to settle at the last known price. One needs the corporate-action stage to credit the entitlement
  a held position was owed, and until it does, the plan withholds its equity-readiness statement
  rather than misstating a distribution.
- Two modules named in the plan's layout are unbuilt: the configuration module and the command line
  entry point. The pages that would use them describe the behaviour as procedural rather than
  configured, and say why.
- The shadow half is fed observed data rather than bound to a live adapter, which the plan records
  as a platform decision rather than as a property of the code.

## How to read a tutorial from this group

Every tutorial in this group is one file with the same twelve sections, in the same order: the idea
in one paragraph; why anyone believed it; an everyday comparison; the rules, step by step; the maths
with every symbol named; a worked example; what the research actually found; how this project
relates to it; where it goes wrong; try it yourself; where this came from; and the words used in the
tutorial. A page written to a different shape would not belong here.

Just below the title comes the provenance table, a small table of seven rows that says at a glance
what the tutorial is about. Because most of these pages are about a piece of machinery rather than
about a trade, the rows that would name an instrument and a trading rhythm instead say which part of
the system the page covers and how often it runs. The table links to the file in this repository that
states the rules, gives one of five grades for how well the idea held up - Strong, Mixed, Weak,
Disputed or Open question - with a clause saying what the grade rests on, and lists the other pages
in this collection that describe the same ground.

[GLOSSARY.md](../GLOSSARY.md) defines the words the tutorial uses, and
[HOW_TO_READ_A_TUTORIAL.md](../HOW_TO_READ_A_TUTORIAL.md) explains the sections one by one for a
reader who has never traded. The authoring rules this collection follows are in
[TUTORIAL_TEMPLATE.md](../TUTORIAL_TEMPLATE.md).

The full list of tutorials in this group, with one line on each, is in [MANIFEST.md](../MANIFEST.md).
