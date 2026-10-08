# Tutorials: trading strategies in plain words

Date: 2026-10-07. Revision 1.

This directory explains trading strategies to someone who has never traded, never opened a
brokerage account, and does not read code. Every idea is built up from scratch: what the thing is,
why anyone thought it would make money, the exact rules, the mathematics with each symbol named in
words, a worked example with real arithmetic, and what happened when researchers tested it.

You do not need money, an account, or a computer to use these tutorials. Most of the
"try it yourself" sections can be done on paper or in a spreadsheet.

## Read this first

| If you want to                      | Go to                                                  |
| ----------------------------------- | ------------------------------------------------------ |
| Understand the words                | [GLOSSARY.md](GLOSSARY.md)                             |
| Know how each tutorial is organised | [HOW_TO_READ_A_TUTORIAL.md](HOW_TO_READ_A_TUTORIAL.md) |
| The rules this collection follows   | [TUTORIAL_TEMPLATE.md](TUTORIAL_TEMPLATE.md)           |
| Learn the background ideas first    | [foundations](foundations)                             |
| See the whole catalogue             | [MANIFEST.md](MANIFEST.md)                             |

## What is in here

One folder per strategy. A strategy that appears in several libraries is still one tutorial, and its
source table names every library it comes from.

| Group                          | What it holds                                                                                                                                                                                                                         | Folder                                   |
| ------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------- |
| Foundations                    | Short primers for someone with no background: what a market is, what an order does, what risk means, why strategies stop working                                                                                                      | [foundations](foundations)               |
| QuantConnect strategy library  | The published, academically documented ideas in the QuantConnect/LEAN library, each one linked to the paper or the Quantpedia entry behind it                                                                                         | [quantconnect](quantconnect)             |
| Backtrader strategy compendium | The thirty strategy categories of the backtrader compendium, which covers 1,152 runnable backtests from Donchian breakouts to HMM regime switching                                                                                    | [backtrader](backtrader)                 |
| Freqtrade strategies           | The practical, mostly crypto, mostly technical-indicator strategies of the Freqtrade community repository                                                                                                                             | [freqtrade](freqtrade)                   |
| fastquant strategies           | The small strategy library shipped with the fastquant backtesting package                                                                                                                                                             | [fastquant](fastquant)                   |
| Papers with backtest           | The published strategies in the awesome-systematic-trading list that do not already appear above, with the replication statistics that list reports                                                                                   | [paperswithbacktest](paperswithbacktest) |
| Quant-trading rules            | The teaching strategies of the quant-trading repository: session breakouts, an oscillator benchmarked against its own control group, a volatility index built from option prices, a price-simulation study, and an estimation problem | [quant-trading](quant-trading)           |
| This project                   | The ideas this repository implements, including the sector rotation engine and the entry and exit price engine                                                                                                                        | [project](project)                       |

## The honest part, before you read anything else

Trading strategies are not recipes that make money. They are hypotheses about why prices might
move in a particular way. Most published ones do not survive contact with real trading costs, and
the ones that do usually produce a small edge that decays as more people use it.

Every tutorial here therefore has a section called "Where it goes wrong" and a section called "What
the research actually found". Those sections are not disclaimers appended at the end. They are the
point. A strategy you understand but do not use is a successful outcome of reading this collection.

Nothing in this directory is financial advice, and nothing here should be used to decide what to do
with your own money.

## What every tutorial assumes

Unless a page says otherwise, the numbers here are the ones a source reported on that source's own
assumptions: a fill happens at the quoted price, there is no slippage and no market impact, and the
only cost is the fee the page names. Real trading adds all three, which is why an effect smaller
than the round-trip cost is reported as such. Where a source assumed something more generous than
that, the page says so. Where this repository models one of the missing pieces, the "How this
project relates to it" section links to the code that does it.

## Where the numbers come from

Three kinds of source appear in these tutorials, and each one is linked in the tutorial that uses
it.

- Library pages: the QuantConnect strategy library, the Freqtrade strategies repository, and the
  fastquant documentation, which describe each strategy's rules.
- Research papers: the primary literature behind a strategy, cited by arXiv identifier and page,
  for example `2107.06194v5` (p.4). The local harvest of those papers, and the twenty-eight
  general-finance and sixteen microstructure briefs written from it, live in
  [strategies/books2](../strategies/books2) and [strategies/books](../strategies/books).
- This repository: the code that implements a strategy, which is what the "How this project
  relates to it" section points at.

## How the tutorials were checked

Every tutorial in this collection passes the same checks the rest of this repository's documents
pass: markdown table normalisation, the documentation conventions hook, the Unicode typography
rules (plain ASCII punctuation only), and a link check that every relative link resolves to a file
that exists. If a number or a formula appears in a tutorial, the source it came from is named in
that tutorial.
