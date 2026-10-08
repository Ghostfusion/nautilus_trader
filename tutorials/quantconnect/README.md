# QuantConnect tutorials

Date: 2026-10-07. Revision 1.

This group explains the strategies published in the QuantConnect strategy library, the catalogue of
tutorials that sits inside the QuantConnect/LEAN documentation. Each page here takes one of those
tutorials, states its rules in plain words, shows the arithmetic, and reports what the research
behind it measured. Nothing here assumes that you have traded, opened an account or read code.

## What the QuantConnect strategy library is

QuantConnect is the company behind LEAN, a free and open-source engine for writing and testing
trading algorithms. Its [strategy library](https://www.quantconnect.com/docs/v2/writing-algorithms/strategy-library)
lists roughly eighty-five tutorials. QuantConnect team members and community members wrote them, and
each tutorial documents the engine, the market data and the code needed to run the strategy.

What matters most is where the ideas come from. Every tutorial names the source of its rules: an
academic paper, an entry in the Quantpedia database of published strategies, or a book. A tutorial
on momentum, for instance, links to the paper that first reported the effect, so the rule can be
traced back to the study that motivated it.

## What is distinctive about these strategies

Three things separate this library from the others in this collection.

- The rules are stated. Each tutorial writes down the exact entry and exit conditions, the kind of
  thing bought and sold, and how often the portfolio is rebuilt, so a reader can follow it by hand.
- The provenance is explicit. Each tutorial names the paper, the Quantpedia entry or the book its
  rules were built from, and links to it.
- The sources are academic. Most of these ideas were first reported in the research literature
  rather than invented by a practitioner or posted on a forum.

## The caveats to carry into them

- A library tutorial demonstrates an implementation, not profitability. It shows that the rule can
  be written and run; it does not show that the rule earns money.
- The published evidence is usually measured on a decades-old sample, often running from the 1920s
  or 1970s up to a date well before the tutorial was written.
- Those published tests often assume low or zero trading costs. The gap between the buying and
  selling price, and any commission, is what erases many of the smaller reported edges.
- The sample was chosen by the researcher, and the rule was fitted to it. A rule that looks good on
  the years it was built from is not the same as a rule that holds on years it has never seen.

## How to read a tutorial from this group

Every tutorial in this group is one file with the same twelve sections, in the same order: the idea
in one paragraph; why anyone believed it; an everyday comparison; the rules, step by step; the maths
with every symbol named; a worked example; what the research actually found; how this project
relates to it; where it goes wrong; try it yourself; where this came from; and the words used in the
tutorial. A page written to a different shape would not belong here.

Just below the title comes the provenance table, a small table of seven rows that says at a glance
what the tutorial is about. It names what the strategy trades, how often it trades, and what you
need to follow it. It links to the library page that states the rules and to the paper, book or
entry those rules came from, or says plainly that there is no research behind them. It gives one of
five grades for how well the idea held up: Strong, Mixed, Weak, Disputed or Open question, with a
clause saying what the grade rests on. And it lists the other libraries in this collection that describe the same
idea, or says that nothing else here does.

[GLOSSARY.md](../GLOSSARY.md) defines the words the tutorial uses, and
[HOW_TO_READ_A_TUTORIAL.md](../HOW_TO_READ_A_TUTORIAL.md) explains the sections one by one for a
reader who has never traded. The authoring rules this collection follows are in
[TUTORIAL_TEMPLATE.md](../TUTORIAL_TEMPLATE.md).

The full list of tutorials in this group, with one line on each, is in [MANIFEST.md](../MANIFEST.md).
