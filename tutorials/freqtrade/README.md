# Freqtrade tutorials

Date: 2026-10-07. Revision 1.

This group explains strategies from the Freqtrade community strategy repository, a collection of
files written for a program that trades cryptocurrencies automatically. Each page here takes one
strategy, states its rules in plain words, shows the arithmetic, and reports what is actually known
about it, which is usually very little. Nothing here assumes that you have traded, opened an account
or read code.

## What the Freqtrade strategy repository is

[freqtrade-strategies](https://github.com/freqtrade/freqtrade-strategies) is a community repository
of free strategy files for Freqtrade, a free and open-source program, often called a bot, that trades
cryptocurrencies on an exchange. The repository holds around seventy strategy files. Many of them
come from a single contributor's series, kept in a folder under that author's name, and one folder,
named `lookahead_bias`, keeps deliberately broken strategies so that a beginner can see what it
looks like when a rule quietly uses information from the future.

The strategies come from trading practice rather than from the research literature: technical
indicators combined by hand, some adapted from MetaTrader systems, and some whose buy and sell
thresholds were chosen by a parameter search, called hyperopt, that the contributor ran.

## What is distinctive about these strategies

- They are practical and crypto-specific. Each file trades a pair such as Bitcoin against the
  dollar, using standard technical indicators, and is written to run inside the bot.
- Each file states a minimal return-on-investment target, the profit at which a position is closed
  automatically, a stop loss, the loss at which it is closed to limit damage, the buy and sell
  signals, the indicators those signals rest on, and any parameters chosen by hyperopt.

## The caveats to carry into them

- Almost none of it has published evidence. There is no paper, no measured effect and no independent
  replication behind the thresholds.
- Much of it was tuned on the pairs and the period it was written for, so the settings are fitted to
  that slice of history and need not carry to another pair or another year.
- The repository's own authors warn that the files are starting points for your own strategies, not
  strategies to trade, and that results depend heavily on the pairs, the timeframe and the period
  used for the backtest.
- The repository exists for educational purposes and ships without any warranty.

## How to read a tutorial from this group

Every tutorial in this group is one file with the same twelve sections, in the same order: the idea
in one paragraph; why anyone believed it; an everyday comparison; the rules, step by step; the maths
with every symbol named; a worked example; what the research actually found; how this project
relates to it; where it goes wrong; try it yourself; where this came from; and the words used in the
tutorial. A page written to a different shape would not belong here.

Just below the title comes the provenance table, a small table of seven rows that says at a glance
what the tutorial is about. It names what the strategy trades, how often it trades, and what you
need to follow it. It links to the source file or page that states the rules and to the paper, book
or article those rules came from, or says plainly that there is no research behind them. It gives
one of four grades for how well the idea held up, Strong, Mixed, Weak or Disputed, with a clause
saying what the grade rests on. And it lists the other libraries in this collection that describe
the same idea, or says that nothing else here does.

[GLOSSARY.md](../GLOSSARY.md) defines the words the tutorial uses, and
[HOW_TO_READ_A_TUTORIAL.md](../HOW_TO_READ_A_TUTORIAL.md) explains the sections one by one for a
reader who has never traded. The authoring rules this collection follows are in
[TUTORIAL_TEMPLATE.md](../TUTORIAL_TEMPLATE.md).

The full list of tutorials in this group, with one line on each, is in [MANIFEST.md](../MANIFEST.md).
