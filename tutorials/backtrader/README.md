# Backtrader tutorials

Date: 2026-10-07. Revision 1.

This group explains strategies from the backtrader strategy compendium, a series of articles that
describe 1,152 runnable backtests. Each page here takes one of those backtests, states its rules in
plain words, shows the arithmetic, and reports what is known about the idea behind it. Nothing here
assumes that you have traded, opened an account or read code.

## What the backtrader strategy compendium is

The compendium is a series of thirty category articles in the documentation of
[cloudQuant/backtrader](https://backtrader.readthedocs.io/en/latest/strategies-series/en/00-overview.html),
a performance-focused fork of the backtrader engine. It describes 1,152 runnable backtests that live
in the project's `tests/functional/strategies` folder. The series was written by the cloudQuant
project, and its strategies come from three places: classic published systems such as the Turtle
Traders and Dual Thrust breakouts; ports of MetaTrader expert advisors and other trading-platform
systems; and research methods such as hidden-Markov regime switching and Kalman-filtered pairs
trading.

Each backtest runs on real market data: gold (XAUUSD) fifteen-minute and daily bars, rebar and glass
futures minute data, and Oracle (ORCL) daily prices. Each one asserts its result rather than printing
it: the final portfolio value, the Sharpe ratio and the maximum drawdown are compared against
baselines, and every strategy must produce identical results in the vectorised and event-driven
engine modes.

## What is distinctive about these strategies

- The breadth is enormous. Trend following alone accounts for 340 strategies, mean reversion 331,
  momentum 45, price patterns 44, and the remaining categories, from asset allocation to
  forecasting, run down to single digits.
- Every category article carries an inventory of all its strategies at a glance, then deep dives
  into two or three representative ones with runnable code.
- Every entry is a complete backtest with asserted numbers, not pseudocode and not a toy example.

## The caveats to carry into them

- Many entries are variants of the same idea. A moving-average crossover with a different pair of
  look-back periods is a new backtest but not a new strategy.
- Passing an assertion proves the engine is correct, not that the strategy earns anything. The
  assertion says the backtest produced the same number in both engine modes; it says nothing about
  whether that number is good or repeatable.
- The assertions compare against baselines, and the baselines live inside the same narrow data set,
  not a realistic account of spreads, slippage and commission.
- The default data is narrow: one gold series, one futures complex and one stock. A rule that holds
  on Oracle daily bars is not evidence that it holds anywhere else.

## How to read a tutorial from this group

Every tutorial in this group is one file with the same twelve sections, in the same order: the idea
in one paragraph; why anyone believed it; an everyday comparison; the rules, step by step; the maths
with every symbol named; a worked example; what the research actually found; how this project
relates to it; where it goes wrong; try it yourself; where this came from; and the words used in the
tutorial. A page written to a different shape would not belong here.

Just below the title comes the provenance table, a small table of seven rows that says at a glance
what the tutorial is about. It names what the strategy trades, how often it trades, and what you
need to follow it. It links to the source page that states the rules and to the paper, book or
article those rules came from, or says plainly that there is no research behind them. It gives one
of five grades for how well the idea held up: Strong, Mixed, Weak, Disputed or Open question, with a
clause saying what the grade rests on. And it lists the other libraries in this collection that
describe the same idea, or says that nothing else here does.

[GLOSSARY.md](../GLOSSARY.md) defines the words the tutorial uses, and
[HOW_TO_READ_A_TUTORIAL.md](../HOW_TO_READ_A_TUTORIAL.md) explains the sections one by one for a
reader who has never traded. The authoring rules this collection follows are in
[TUTORIAL_TEMPLATE.md](../TUTORIAL_TEMPLATE.md).

The full list of tutorials in this group, with one line on each, is in [MANIFEST.md](../MANIFEST.md).
