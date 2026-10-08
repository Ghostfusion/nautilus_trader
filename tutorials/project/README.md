# Project tutorials

Date: 2026-10-07. Revision 1.

This group covers the ideas this repository itself implements and measures, rather than ideas taken
from an outside library. Each page here takes one piece of that work, states its rules or its
questions in plain words, shows the arithmetic, and reports what the repository's own measurement
found. Nothing here assumes that you have traded, opened an account or read code.

## What this project is

This project is the repository you are reading, a research and trading platform whose own studies,
designs and programs feed this group. Four pieces of work supply it.

- The sector rotation study, [Profiting from sector rotation](../../strategies/sector_rotation_strategies.md),
  asks what is left of a sector-selection strategy once leadership is assumed to have no memory.
- The entry and exit price engine design, [Entry/Exit Price Engine: design](../../strategies/entry_exit_engine_design.md),
  adds execution realism, meaning the gap between the buying and selling price, the cost of moving a
  large order, and risk boundaries, without forecasting a return.
- The sector regime engine, [Sector Regime Engine](../../implementation/sector-regime-engine/README.md),
  measures whether sector leadership contains exploitable temporal dependence and refuses to suggest
  a directional strategy when it does not.
- The measurement work in [the analysis crate](../../crates/analysis/README.md) scores a candidate on
  named evidence metrics rather than on profit alone.

The ideas come from the repository's own research briefs, which read the general-finance and
market-microstructure literature and state what it changes about how a strategy is built, simulated
and evaluated. Those briefs are indexed in [strategies/books2](../../strategies/books2/README.md).

## What is distinctive about this work

- Every claim is tied to executable code and to the repository's own research briefs, so a reader can
  check the source rather than take a number on trust.
- The measurement is built to refuse. The regime engine reports one of four verdicts and says that
  there is nothing to exploit when the data does not support a pattern.

## The caveats to carry into it

- The code is research infrastructure, not a trading system. It contains no prices feed, no broker
  connection and no recommendation of any kind.
- The user manual, [Sector Regime Engine: user manual](../../implementation/sector-regime-engine/USER_MANUAL.md),
  states plainly what the engine does not do: it does not predict the future, place trades, produce
  a complete strategy or give advice.
- The entry and exit price engine is a design for review, not implemented, and its own document says
  so on its first page.

## How to read a tutorial from this group

Every tutorial in this group is one file with the same twelve sections, in the same order: the idea
in one paragraph; why anyone believed it; an everyday comparison; the rules, step by step; the maths
with every symbol named; a worked example; what the research actually found; how this project
relates to it; where it goes wrong; try it yourself; where this came from; and the words used in the
tutorial. A page written to a different shape would not belong here.

Just below the title comes the provenance table, a small table of seven rows that says at a glance
what the tutorial is about. It names what the strategy trades, how often it trades, and what you
need to follow it. It links to the file in this repository that states the rules and to the research
brief or paper those rules came from, or says plainly that there is no external research behind
them. It gives one of five grades for how well the idea held up: Strong, Mixed, Weak, Disputed or
Open question, with a clause saying what the grade rests on. And it lists the other libraries in this collection
that describe the same idea, or says that nothing else here does.

[GLOSSARY.md](../GLOSSARY.md) defines the words the tutorial uses, and
[HOW_TO_READ_A_TUTORIAL.md](../HOW_TO_READ_A_TUTORIAL.md) explains the sections one by one for a
reader who has never traded. The authoring rules this collection follows are in
[TUTORIAL_TEMPLATE.md](../TUTORIAL_TEMPLATE.md).

The full list of tutorials in this group, with one line on each, is in [MANIFEST.md](../MANIFEST.md).
