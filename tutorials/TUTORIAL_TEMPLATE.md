# How every tutorial in this collection is written

Date: 2026-10-07. Revision 1.

This file is the authoring contract. Every folder under [quantconnect](quantconnect),
[backtrader](backtrader), [freqtrade](freqtrade), [fastquant](fastquant),
[paperswithbacktest](paperswithbacktest) and [project](project) holds one `README.md` written to
the template below and obeying the rules below. If you are reading the collection rather than
writing it, [HOW_TO_READ_A_TUTORIAL.md](HOW_TO_READ_A_TUTORIAL.md) explains the sections in plain
words.

## Who the reader is

Assume the reader has never bought or sold anything in a market, has never heard any trading
vocabulary, and does not read code. They are intelligent and patient, and they will follow a
careful explanation. They know what a percentage is.

## The template

Every tutorial is one file, `README.md`, with exactly this shape and exactly these section
headings, in this order.

```text
# <Name of the strategy in plain words>

<Date: YYYY-MM-DD. Revision 1.>

<The provenance table described below>

## The idea in one paragraph

## Why anyone believed it

## An everyday comparison

## The rules, step by step

## The maths, with every symbol named

## A worked example

## What the research actually found

## How this project relates to it

## Where it goes wrong

## Try it yourself

## Where this came from

## Words used in this tutorial
```

### The provenance table

Seven rows, two columns, header row present. Add no other rows.

| Field                     | How to fill it                                                                                                                  |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | The kind of thing bought and sold, described concretely: "shares of large American companies", "gold versus silver", "Bitcoin". |
| How often it trades       | The rhythm in plain words: "about once a month", "a few times a day", "almost never after the first purchase".                  |
| What you need             | One of: "nothing but this page", "a spreadsheet", "Python and a data file".                                                     |
| Where the rules come from | The library page that states the rules, as a markdown link.                                                                     |
| The underlying research   | The paper, book or entry the rules were built from, as a markdown link, or "none, this is a practitioner's rule of thumb".      |
| How well it held up       | One of the four grades below, plus one clause saying what the grade is based on.                                                |
| Also appears in           | The other libraries in this collection that describe the same idea, as links, or "nothing else in this collection".             |

### Grades for "how well it held up"

| Grade    | Use it when                                                                                                            |
| -------- | ---------------------------------------------------------------------------------------------------------------------- |
| Strong   | Several independent samples, costs included, and the effect survives out of sample.                                    |
| Mixed    | The effect appears in some samples, markets or periods and not others, or it needs costs low enough to be unrealistic. |
| Weak     | Published once, on one sample, with no independent replication, or the replication reduced it to nothing.              |
| Disputed | Different credible sources reach opposite conclusions, and the disagreement is unresolved.                             |

### Section by section

**The idea in one paragraph.** What the strategy does and why in at most six sentences, with no
symbols and no vocabulary the reader has not met. If you cannot say it in six sentences you have
not understood it well enough to write the rest.

**Why anyone believed it.** The economic story: who is on the other side of the trade, what they
might be forced or willing to do, and why they would keep doing it. A strategy needs a counterparty
with a reason, not just a pattern.

**An everyday comparison.** One concrete non-financial situation that behaves the same way, such as
a supermarket discounting fruit that is about to spoil, or a bus that is late so the crowd at the
stop grows. Keep it to a paragraph.

**The rules, step by step.** A numbered list of exactly what the reader would look at and do: what
data, which frequencies, what the entry condition is, what the exit condition is, how much is
bought or sold, and how often positions are reviewed. Every rule must be checkable by the reader.
Say what "a rising price" or "the best performer" means precisely, in a way a person with a
spreadsheet could apply without guessing.

**The maths, with every symbol named.** The formulas as displayed blocks, each symbol defined in
words on the line after it, and every formula followed by a sentence saying what it means. A
formula with an undefined symbol is a defect. When a source gives numbers, put them in and show
what the formula produces.

**A worked example.** Five to ten periods of made-up but plausible numbers, laid out in a table,
each row showing the calculation. The arithmetic must be correct and consistent with the rules
section. Work the whole thing through to a final result, including the costs. Do not hand the
reader a formula and skip the example, and do not invent numbers that break the formula.

**What the research actually found.** What the source measured, on what data, over what period,
with what costs, and what the effect size was in plain words, for example "about four percent a
year before costs and two percent after". Distinguish what the paper proves from what it assumes.
If several sources disagree, report the disagreement rather than picking a side. Never write "this
works"; report what was measured.

**How this project relates to it.** The concrete link into this repository: the file or directory
that implements the same idea or the measurement that tests it, as a relative markdown link, and
one sentence on what the reader would see there. If this project implements nothing related, say
so plainly and point at the closest thing that exists.

**Where it goes wrong.** The honest failure modes: what makes the signal vanish (crowding, changes
in rules, a regime that ends), what breaks the measurement (survivorship, look-ahead, costs
ignored, a sample chosen after the fact), and what would have to be true for the whole idea to be
false. Three to six items, each one a sentence or two.

**Try it yourself.** One concrete exercise the reader can do without money and without code.
Either a spreadsheet task with explicit column headings, or a paper exercise such as scoring the
last twenty years of a published table by hand. Say what the reader should notice.

**Where this came from.** A bullet list of the links actually used: the library page, the paper or
Quantpedia entry, and the section of this repository's own research that covers it, such as a brief
in [strategies/books2](../strategies/books2). If you cite a paper by its arXiv identifier,
include the identifier here, for example `2107.06194v5`.

**Words used in this tutorial.** Three to eight glossary entries, in the form
`- term: one-sentence definition`. Use terms the tutorial introduced. Link to
[GLOSSARY.md](GLOSSARY.md) for the rest.

## Hard rules

1. Plain words first. The first time any trading or statistical term appears, define it in the same
   sentence, in parentheses or after a dash. No exceptions, including "return", "volatility",
   "position", "long", "short", "spread", "ETF" and "futures".
2. Never promise a profit and never write that a strategy works. Report what a source measured.
   Nothing in a tutorial may read as financial advice.
3. Attribute every number. A number without a source is a defect. Cite either a source link or an
   arXiv identifier with a page, for example `2107.06194v5` (p.4).
4. Plain ASCII punctuation only. No typographic dashes, curly quotes, ellipsis characters, or
   emoji. Use the hyphen-minus on the keyboard.
5. Recommend nothing about the reader's own money, taxes or legal situation.
6. Length: 150 to 300 lines for a strategy tutorial. Foundations primers may be 100 to 200 lines.
   Stop when the content stops, and do not pad to reach a length.
7. Do not copy a library page or a paper. Explain it, in your own sentences, aimed at someone
   who has never traded.
8. Keep the reader's arithmetic honest: one unit of currency, consistent rounding, and costs
   included in the worked example.

## Formatting rules

1. Sentence case for headings. One top-level `#` heading per file. Blank line above and below every
   heading, table and list. `-` for bullets, 1. 2. 3. for the numbered rules section.
2. Paragraphs wrapped at about 110 columns. Tables are not wrapped.
3. Tables use leading and trailing pipes, an aligned delimiter row, and padded cells. Run the table
   normaliser, which does the padding for you.
4. Fenced code blocks with a language tag, backticks only.
5. Links: relative links must point at files that exist in this repository. Never link to a
   heading in another file, because a broken fragment cannot be checked offline. Absolute links go
   to the library page, the paper or the data source, and are the only allowed external links.
6. End the file with exactly one newline and no trailing spaces.

## Check before you finish

From the repository root, all four must pass: the table normaliser run twice with the second run
reporting nothing, the documentation conventions hook, the Unicode typography hook, and the
tutorial checker, which verifies the links, the citations and the required sections.

```bash
python scripts/check-markdown-tables.py tutorials/**/README.md
bash .pre-commit-hooks/check_docs_conventions.sh
bash .pre-commit-hooks/check_unicode_typography.sh tutorials/**/README.md
python "E:/fin paper3/.state/check_tutorials.py" tutorials
```
