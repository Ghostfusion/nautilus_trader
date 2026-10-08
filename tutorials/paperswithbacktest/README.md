# Paperswithbacktest tutorials

Date: 2026-10-07. Revision 1.

This group explains the published strategies in the awesome-systematic-trading list that are covered
nowhere else in this collection, from its tables for equities, bonds, commodities, currencies,
crypto, derivatives and mixed portfolios. Each page here takes one strategy, states its rules in
plain words, shows the arithmetic, and reports the result the list measured for it. Nothing here
assumes that you have traded, opened an account or read code.

## What the awesome-systematic-trading list is

[paperswithbacktest/awesome-systematic-trading](https://github.com/paperswithbacktest/awesome-systematic-trading)
is a curated list on GitHub, maintained by the paperswithbacktest project, that gathers libraries,
strategies taken from published papers, books and videos for systematic trading. Each strategy in
its tables is a short Python file plus a measured result. The list is written and maintained by the
paperswithbacktest project, and its strategy rows are drawn from academic papers, one per row.

## What is distinctive about these strategies

The list is unusual because it reports what happened when each strategy it has coded was run, rather
than only repeating the paper's claim.

- It gives a measured Sharpe ratio for each strategy it has coded and run over that strategy's own
  full history.
- It publishes a replication record across its whole set. Of 4,843 papers coded and run, the median
  replication returned a Sharpe ratio of 0.37, and 48 percent cleared a t-statistic of 1.96; the
  list notes that half the published record cannot be distinguished from zero on its own sample.
- The median test window is 34 years, and the median strategy carries a beta of +0.17 to the S&P 500;
  removing that market exposure takes the median information ratio down to 0.21.
- Across 2,838 papers with a record on both sides of their publication date, the project reports no
  measurable decay after publication, once the market period is controlled for, to within a fifth of
  a percentage point a year.

## The caveats to carry into them

- The numbers are one project's own measurements. They are not peer reviewed, and each comes from
  running one strategy over one chosen history.
- The list mixes quality. It gathers strategies of very different reliability under one heading, so
  finding a strategy in it is not a statement that the strategy is trustworthy.
- A single-sample Sharpe ratio is not evidence of a durable edge. One number from one history can be
  produced by luck, and the project's own median of 0.37 across thousands of papers is the clearest
  sign of that.

## How to read a tutorial from this group

Every tutorial in this group is one file with the same twelve sections, in the same order: the idea
in one paragraph; why anyone believed it; an everyday comparison; the rules, step by step; the maths
with every symbol named; a worked example; what the research actually found; how this project
relates to it; where it goes wrong; try it yourself; where this came from; and the words used in the
tutorial. A page written to a different shape would not belong here.

Just below the title comes the provenance table, a small table of seven rows that says at a glance
what the tutorial is about. It names what the strategy trades, how often it trades, and what you
need to follow it. It links to the list row that states the rules and to the paper those rules came
from, or says plainly that there is no research behind them. It gives one of five grades for how
well the idea held up: Strong, Mixed, Weak, Disputed or Open question, with a clause saying what the
grade rests on. And it lists the other libraries in this collection that describe the same idea, or says that
nothing else here does.

[GLOSSARY.md](../GLOSSARY.md) defines the words the tutorial uses, and
[HOW_TO_READ_A_TUTORIAL.md](../HOW_TO_READ_A_TUTORIAL.md) explains the sections one by one for a
reader who has never traded. The authoring rules this collection follows are in
[TUTORIAL_TEMPLATE.md](../TUTORIAL_TEMPLATE.md).

The full list of tutorials in this group, with one line on each, is in [MANIFEST.md](../MANIFEST.md).
