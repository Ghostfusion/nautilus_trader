# How to read a tutorial

Date: 2026-10-07. Revision 1.

Every tutorial in this collection takes one trading idea and builds it up from nothing, for a
reader who has never bought or sold anything and does not read code. All of them have the same
sections in the same order, so once you have finished one you know where to look in the rest. This
page explains what each section is for and what a good one contains. The authoring rules are in
[TUTORIAL_TEMPLATE.md](TUTORIAL_TEMPLATE.md), and any word you do not know is defined in
[GLOSSARY.md](GLOSSARY.md).

## The provenance table

Each tutorial opens with a seven-row table that tells you, before you read a word of the rest,
what the idea is about. The rows are: what it trades, how often it trades, what you need in order
to follow along, where the rules come from, the research underneath them, how well the idea held
up, and any other library in this collection that describes the same thing.

Read this table first. It answers the questions that decide whether the tutorial is worth your
time. If it trades something you do not care about, or needs a data file you do not have, or the
evidence grade is "Disputed", you know that before investing effort. A good table is concrete: not
"financial assets" but "shares of large American companies", not "sometimes" but "about once a
month". The "where the rules come from" row always links to the library page that states the
rules, so you can check that the tutorial has not invented them.

## What the five grades mean

The row called "How well it held up" carries one of five grades. The grade summarises the evidence
behind the idea. It is not a recommendation, and a high grade is not a promise that the idea makes
money.

| Grade         | What it means                                                                                                                           |
| ------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| Strong        | Several independent samples, trading costs included, and the effect still shows up on data that was not used to build it.               |
| Mixed         | The effect appears in some samples, markets or periods and not others, or it only works if costs are lower than they realistically are. |
| Weak          | Published once on one sample with no independent replication, or the replication reduced it to nothing.                                 |
| Disputed      | Different credible sources reach opposite conclusions and nobody has settled the disagreement.                                          |
| Open question | A published claim exists for the idea, but this collection found no measurement of the exact rule.                                      |

A good tutorial names what the grade is based on in one clause, so you can see whether it rests on
a single paper or on many tests. "Strong" describes the evidence, not the strategy's future.

## The idea in one paragraph

This opens the tutorial and says what the strategy does in at most six sentences, with no symbols
and no words you have not met yet. A good one can be read out loud to someone who has never traded
and understood immediately. If the idea cannot be said that briefly, it is not yet understood well
enough to explain.

## Why anyone believed it

Every trade has a person on the other side of it, and this section says who that person is and why
they would take the opposite bet. The reason might be a rule they must follow, a need for cash, or
a habit they keep repeating. A good version names a real counterparty and a real reason, rather
than pointing at a shape on a chart and calling it a pattern. A strategy without a reason for the
other side to trade is a coincidence waiting to disappear.

## An everyday comparison

One concrete, non-financial situation that behaves the same way, such as a supermarket discounting
fruit that is about to spoil, or a bus stop where a delayed bus makes the crowd grow. A good
comparison is short and specific, and it makes the mechanism obvious without using a single piece
of trading vocabulary.

## The rules, step by step

This is the part you could actually follow. It is a numbered list of exactly what you would look
at and do: what data, how often, what makes you buy, what makes you sell, how much you buy or
sell, and how often you review. A good version is checkable: it says precisely what "rising" or
"the best performer" means, so a person with a spreadsheet could apply it without guessing. If a
rule cannot be checked after the fact, it is not a rule.

## The maths, with every symbol named

The formulas appear as displayed blocks, and every symbol is defined in words on the line after
it, followed by a sentence saying what the formula means. A symbol with no definition is a defect,
not a challenge. A good section also puts the source's own numbers into the formula and shows what
comes out, so the maths connects to the research rather than floating above it.

## A worked example

Five to ten periods of made-up but plausible numbers, laid out in a table, with each row showing
its calculation. A good example uses one currency, rounds consistently, follows the rules section
exactly, includes the costs, and arrives at a final result. You should be able to redo it by hand
and get the same answers. If the example and the rules disagree, trust neither and say so.

## What the research actually found

This section reports what the source measured: on what data, over what period, with what costs,
and how large the effect was in plain words, for example "about four percent a year before costs
and two percent after". It distinguishes what a paper proves from what it assumes, attributes
every number to a source, and reports disagreement between sources instead of picking a side. It
never says the strategy works; it says what was measured.

## How this project relates to it

The concrete link back into this repository: a relative link to the file or folder that implements
the same idea or measures it, plus one sentence about what you would see there. A good version
does not overstate the connection. If this project implements nothing related, the section says so
plainly and points at the closest thing that does exist.

## Where it goes wrong

The honest failure modes, in three to six short items: what makes the signal vanish, such as too
many people trading it or a rule change; what breaks the measurement, such as survivorship,
look-ahead, or ignored costs; and what would have to be true for the whole idea to be false. A
good version is specific about the mechanism, not a vague warning that markets are risky.

## Why every tutorial has a section about failure

Published strategies are hypotheses, not recipes, and the ones that look best in a first test
often fall apart once costs, fresh data and real competition are added. A collection that only
described the ideas would train you to believe them. The failure section is therefore part of the
explanation, not a disclaimer bolted onto the end. Reading it first is a perfectly good way to use
a tutorial: if the failure modes are fatal for your purposes, you have saved yourself the rest.
Understanding a strategy well enough to decide not to use it is a successful outcome of reading.

## Try it yourself

One exercise you can do without money and without code: either a spreadsheet task with named
column headings, or a paper task such as scoring a published table by hand. A good exercise ends
by saying what you should notice, so you know whether you did it right. Doing this once is worth
more than reading three more tutorials.

## Where this came from

A bullet list of the links actually used: the library page that states the rules, the paper or
entry behind them, and the matching brief in this repository. Papers are cited by arXiv identifier
and page, for example `2107.06194v5` (p.4). If a claim has no source here, it should not be in the
tutorial.

## Words used in this tutorial

Three to eight glossary entries for the terms this particular tutorial introduced, each defined in
one sentence. The remaining terms are in [GLOSSARY.md](GLOSSARY.md). This section exists so that
you never have to leave a tutorial to understand it.

## About the numbers in the worked examples

The worked examples use invented numbers. They are chosen to be plausible for the thing being
traded, but they are not a forecast, not a backtest, and not a result from the research. What they
are for is the arithmetic: every step is shown, so you can follow the multiplication and the
percentages yourself and check that the rules produce the answer printed at the bottom. The real
measured numbers belong to the "What the research actually found" section, where they are
attributed to a source.

## How to use one tutorial from start to finish

1. Read the provenance table and the idea paragraph. Decide whether the thing being traded and the
   evidence grade are worth your time at all.
2. Read "Why anyone believed it" and "An everyday comparison". If nobody would plausibly be on the
   other side of the trade, the rest does not matter.
3. Check the rules. Ask yourself whether you could apply each step to next month's prices without
   guessing. A rule you cannot write down is not a rule.
4. Verify the maths on the small table in "A worked example". Redo one row by hand, including the
   costs, and confirm you get the same number.
5. Read "What the research actually found" and note the effect size and the period. Small, old or
   unreplicated effects deserve suspicion.
6. Read "Where it goes wrong" last, with the rest in mind. Ask which of the listed failures has
   already happened.
7. Do the "Try it yourself" exercise. It is the part that turns reading into understanding.

Nothing here is financial advice, and nothing in the collection should be used to decide what to
do with your own money.
