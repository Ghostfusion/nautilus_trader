# research-process tutorials

Date: 2026-10-08. Revision 1.

This group does not explain a strategy. It explains the process that decides whether a strategy is
worth believing: how a search over settings is run, which data is held back and how, how to ask
whether a result depended on the order of the past, and how to score what survives. Every other
group in this collection uses this process; this group is where it is written out. Nothing here
assumes that you have traded, opened an account or read code.

## What is in this group

Four pages, each one a procedure you could follow by hand:

- Searching settings, and why the winner looks too good. How many candidates were tried decides how
  high the best one is, even when none of them has an edge.
- Held-back data comes in two flavours. The two independent ways to keep data away from the search,
  and what each one does and does not prove.
- Shuffling the past to ask whether the order mattered. The test that rebuilds the same prices in a
  different order and asks how often the rearranged world does as well.
- Scoring what survived the held-back test. How to turn two runs, an original and a held-back one,
  into a single statement about how much of the result survived.

## Why a group about method, in a collection of strategies

A rule and a number are not the same thing. The rule can be stated in a paragraph and checked by
anyone. The number is produced by a process: which data was used, which period, how many settings
were tried before one was kept, and what the trades cost. Two people applying the same rule can
report very different numbers and both be honest, because the processes differ. Reading a strategy
without knowing the process is like reading a drug's description without knowing whether it was
tested against a dummy pill or only given to people who were already recovering.

The pages here are the process half. They are deliberately arithmetic rather than philosophical:
each one ends in a small sum you can redo, not in a warning that markets are risky.

## The caveats to carry into them

- No procedure removes luck. Counting the tries, holding data back and rearranging the past all
  shrink a number towards what it really is; none of them turns a losing rule into a winning one.
- Every quantity here is an estimate of an estimate. The papers behind these pages disagree about
  how large the correction should be, and the pages report the disagreement rather than a single
  figure.
- The tests are stated in terms of returns, which are the percentage gains and losses a strategy
  produces. A page that says "the p-value is 0.04" is making a statement about arithmetic, not about
  anyone's money.
- Nothing here is a recipe for finding a strategy. It is a recipe for distrusting one properly.

## How to read a tutorial from this group

Every tutorial in this group is one file with the same twelve sections, in the same order: the idea
in one paragraph; why anyone believed it; an everyday comparison; the rules, step by step; the maths
with every symbol named; a worked example; what the research actually found; how this project
relates to it; where it goes wrong; try it yourself; where this came from; and the words used in the
tutorial. A page written to a different shape would not belong here.

Just below the title comes the provenance table, a small table of seven rows that says at a glance
what the tutorial is about. Because these pages are procedures rather than trades, the first two rows
say what the procedure acts on and how often it runs, which for a test is once per study rather than
once per day. The table links to the source that states the rules - for these pages, usually code in
this repository - and to the research behind them, or says plainly that there is none. It gives one
of five grades for how well the procedure held up: Strong, Mixed, Weak, Disputed or Open question,
with a clause saying what the grade rests on. And it lists the other pages in this collection that
describe the same idea, or says that nothing else here does.

[GLOSSARY.md](../GLOSSARY.md) defines the words the tutorials use, and
[HOW_TO_READ_A_TUTORIAL.md](../HOW_TO_READ_A_TUTORIAL.md) explains the sections one by one for a
reader who has never traded. The authoring rules this collection follows are in
[TUTORIAL_TEMPLATE.md](../TUTORIAL_TEMPLATE.md).

The full list of tutorials in this group, with one line on each, is in [MANIFEST.md](../MANIFEST.md).
