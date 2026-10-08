# fastquant tutorials

Date: 2026-10-07. Revision 1.

This group explains the strategies that ship with the fastquant backtesting package, a small set of
worked examples meant to show a beginner how a backtest is run. Each page here takes one strategy,
states its rules in plain words, shows the arithmetic, and reports what is known about it, which is
usually only that it is a teaching example. Nothing here assumes that you have traded, opened an
account or read code.

## What the fastquant strategy library is

[fastquant](https://github.com/enzoampil/fastquant) is a small open-source Python package built to
make backtesting accessible to beginners; its README promises to backtest an investment strategy in
as few as three lines of code. Its author, who publishes on GitHub as enzoampil, ships it with nine
built-in strategies, so there is nothing to write: you name the strategy by a short alias and supply
the data.

The nine are a relative strength index (RSI) rule, a simple moving-average crossover, an exponential
moving-average crossover, a moving-average convergence divergence (MACD) rule, a Bollinger-bands
rule, buy and hold, a news-sentiment rule, and two custom variants, one driven by a prediction
column in the data and one that reads a buy or sell number from a column you supply. The examples in
the package use Philippine and American shares, and crypto prices from the exchange Binance.

## What is distinctive about these strategies

- They are tiny and readable, and were written to teach backtesting rather than to be traded.
- Each one has a one-line alias and a handful of parameters, so a reader can run a different rule
  without editing any code, and can see how the parameters change the result.

## The caveats to carry into them

- They are teaching examples. The package README demonstrates several of them on a single stock, the
  Philippine food company Jollibee, with default parameters.
- There is no published evidence behind any of them: no paper, no measured effect, no independent
  test.
- A different stock or a different period changes the result, and the default parameters were not
  chosen to be robust across either.

## How to read a tutorial from this group

Every tutorial in this group is one file with the same twelve sections, in the same order: the idea
in one paragraph; why anyone believed it; an everyday comparison; the rules, step by step; the maths
with every symbol named; a worked example; what the research actually found; how this project
relates to it; where it goes wrong; try it yourself; where this came from; and the words used in the
tutorial. A page written to a different shape would not belong here.

Just below the title comes the provenance table, a small table of seven rows that says at a glance
what the tutorial is about. It names what the strategy trades, how often it trades, and what you
need to follow it. It links to the package documentation and to the paper, book or article the rule
came from, or says plainly that there is no research behind it. It gives one of five grades for how
well the idea held up: Strong, Mixed, Weak, Disputed or Open question, with a clause saying what the
grade rests on. And it lists the other libraries in this collection that describe the same idea, or says that
nothing else here does.

[GLOSSARY.md](../GLOSSARY.md) defines the words the tutorial uses, and
[HOW_TO_READ_A_TUTORIAL.md](../HOW_TO_READ_A_TUTORIAL.md) explains the sections one by one for a
reader who has never traded. The authoring rules this collection follows are in
[TUTORIAL_TEMPLATE.md](../TUTORIAL_TEMPLATE.md).

The full list of tutorials in this group, with one line on each, is in [MANIFEST.md](../MANIFEST.md).
