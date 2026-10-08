# quant-trading tutorials

Date: 2026-10-08. Revision 1.

This group explains a set of teaching strategies published in the
[quant-trading](https://github.com/je-suis-tm/quant-trading) repository. Each page here takes one
idea, states its rules in plain words, shows the arithmetic, and reports what is and is not known
about it. Nothing here assumes that you have traded, opened an account or read code.

## What the quant-trading repository is

It is one author's collection of standalone Python scripts, published under the Apache licence, each
one a complete little backtest of a single idea: an indicator rule, an intraday session breakout, a
pairs trade, an option position, or a study of whether a simulation method predicts anything. Alongside
the scripts sit four longer projects that ask a question rather than test a rule, such as whether a
currency moves with the commodity its country exports, and whether a farmer's planting decision can
be modelled.

Its author states one assumption for the whole repository: every trade is frictionless, with no
slippage, no surcharge and no illiquidity. Most of the scripts plot a chart and print nothing else,
and the repository publishes no performance table. The author's habit of reporting dead ends plainly,
and of saying what a result depends on, is the most useful thing in it.

## What is distinctive about these strategies

- Several are rule sets that other libraries state loosely. Here they are written out as conditions a
  person could check with a spreadsheet: eight conditions for one candle shape, a five-node search for
  one band pattern, a rejection rule that skips a whole day.
- Two of them are not strategies at all but measured questions: whether a simulated price path can
  predict a direction, and how a volatility reading is built from option prices rather than quoted.
- The author compares one rule against another on purpose, and reports that the faster signal is not
  the better one.

## The caveats to carry into them

- No script charges a cost. Anything a page here reports has to be read as a number before costs.
- Several scripts state no instrument, no dates and no result: they are demonstrations of a rule, not
  tests of it. Where that is so, the page says so and grades the rule accordingly.
- The projects contain arithmetic defects that the author's own write-up contradicts, including
  position sizes chosen with hindsight and a regression fitted on the part of the sample it calls
  out of sample. The pages name them instead of repeating them.
- These are teaching files. Nothing in them was written to be traded, and the author says as much.

## How to read a tutorial from this group

Every tutorial in this group is one file with the same twelve sections, in the same order: the idea
in one paragraph; why anyone believed it; an everyday comparison; the rules, step by step; the maths
with every symbol named; a worked example; what the research actually found; how this project
relates to it; where it goes wrong; try it yourself; where this came from; and the words used in the
tutorial. A page written to a different shape would not belong here.

Just below the title comes the provenance table, a small table of seven rows that says at a glance
what the tutorial is about. It names what the strategy trades, how often it trades, and what you
need to follow it. It links to the source file that states the rules and to the paper, book or entry
those rules came from, or says plainly that there is no research behind them. It gives one of five
grades for how well the idea held up: Strong, Mixed, Weak, Disputed or Open question, with a clause
saying what the grade rests on. And it lists the other libraries in this collection that describe
the same idea, or says that nothing else here does.

[GLOSSARY.md](../GLOSSARY.md) defines the words the tutorial uses, and
[HOW_TO_READ_A_TUTORIAL.md](../HOW_TO_READ_A_TUTORIAL.md) explains the sections one by one for a
reader who has never traded. The authoring rules this collection follows are in
[TUTORIAL_TEMPLATE.md](../TUTORIAL_TEMPLATE.md).

The full list of tutorials in this group, with one line on each, is in [MANIFEST.md](../MANIFEST.md).
