# Lexical density of company filings: buying companies that describe themselves in more detail

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of the largest American companies, held in two baskets: the detailed writers bought, the vague writers sold short                                                                                                                                                                             |
| How often it trades       | About once a month, when the baskets are rebuilt                                                                                                                                                                                                                                                     |
| What you need             | A spreadsheet and a table of document word counts                                                                                                                                                                                                                                                    |
| Where the rules come from | [The list's implementation file](https://github.com/paperswithbacktest/awesome-systematic-trading/blob/main/static/strategies/how-to-use-lexical-density-of-company-filings.py) and the [Quantpedia entry](https://quantpedia.com/strategies/how-to-use-lexical-density-of-company-filings) it cites |
| The underlying research   | Hanicova, Kalus and Vojtko, [How to Use Lexical Density of Company Filings](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=3921091)                                                                                                                                                             |
| How well it held up       | Mixed: the effect is measured on one provider's text dataset over 2010 to 2021 with a positive result, but no independent replication uses a different language-measurement provider, and the same repository briefs show how much the provider changes the number                                   |
| Also appears in           | [Using news sentiment to predict the price direction of drug manufacturers](../../../tutorials/quantconnect/using-news-sentiment-to-predict-price-direction-of-drug-manufacturers/README.md), a close relative that scores text instead of prices                                                    |

## The idea in one paragraph

Every public company in the United States must file a detailed report with the regulator several
times a year. This strategy counts the vocabulary in those reports. It buys the companies whose
reports use many information-carrying words, and it sells short the companies whose reports use few.
It rebuilds the two baskets once a month. The bet is that how much a company explains, and how
plainly it uses financial language, carries information that the numbers on the front page do not,
and that the market takes time to read it.

## Why anyone believed it

The counterparty is anyone who does not read the filings. Most investors, human and machine, work
from the numbers: revenue, profit, debt. The words around those numbers are long, repetitive and
unstructured, which is exactly why they are easy to skip and hard to process at scale. A company that
hedges, obscures or writes vaguely may be doing so because the truth is uncomfortable; a company that
documents its problems in concrete financial language may be more transparent about what is coming.

There is also a mechanical channel. Regulators require the reports, so every company writes one on
the same schedule, which means the signal arrives at known dates for thousands of companies at once.
If one corner of the market reads them carefully and the rest do not, the careful reader can trade
before the crowd. The provider of the data, a company called Brain, computes the counts and publishes
them as a dataset, which turns reading into a number a spreadsheet can use.

## An everyday comparison

Think of a school report. One teacher writes "doing fine, could focus more", and another writes a
page naming each topic the child has mastered and each one they have not. Both reports may be
accurate, but the second carries far more information for the same event, and a parent skimming
thirty reports learns more from the detailed one. The strategy treats the density of a company's
report as a sign of how much it is telling you: dense, specific writing is the detailed report, and
vague writing is the one-line summary.

## The rules, step by step

1. Choose the universe: the 500 American companies with the largest dollar value of shares traded,
   listed on the New York, Nasdaq or American exchanges.
2. For each company, take its most recent annual report, called a 10-K, or its most recent quarterly
   report, called a 10-Q. Read the text.
3. Compute two numbers from the text. Lexical density is the count of information-carrying words
   divided by the total number of words. Specific density is the count of finance-related words
   divided by the total number of words.
4. Rank the companies separately by each number, best first.
5. Buy the top tenth and sell short the bottom tenth according to lexical density, and do the same
   according to specific density. Split the money equally between the two rankings, so a company that
   is in the top tenth of both receives twice as much as one in the top tenth of only one.
6. Within each basket, give each company an equal share.
7. Hold for one month, then use the newest filing and rebuild. The universe of 500 companies is
   refreshed every three months.

A note on what a filing is. A 10-K is the once-a-year report; a 10-Q is the three-times-a-year
quarterly report. Both are filed with the Securities and Exchange Commission, the American market
regulator, and both are public in full on the regulator's EDGAR website. They contain the financial
statements, a description of the business, and a discussion by management of how the year or quarter
went and what risks lie ahead.

## The maths, with every symbol named

The two text measures used by the strategy:

```text
LD = count of information-carrying words / total words
SD = count of finance-related words / total words
```

- `LD` is lexical density. Information-carrying words are the nouns, main verbs, adjectives and
  adverbs; the words left out are the small connecting words such as "the", "of" and "and". A high
  value means a text packed with content; a value around 0.55 is typical for a long report.
- `SD` is specific density. Finance-related words are the terms a financial dictionary recognises,
  such as "liability", "hedge" and "cash flow". A high value means the report discusses money and
  risk in money terms rather than in generalities.

Then rank, and split the money between the two rankings:

```text
w_i = 0.5 * (1/N_LD if i is in the top tenth on LD) + 0.5 * (1/N_SD if i is in the top tenth on SD)
```

- `w_i` is the fraction of the account placed in company `i` on the bought side.
- `N_LD` and `N_SD` are the numbers of companies in the top tenth on each measure. With 500
  companies and equal weighting, that is 50 companies each, so each contributes 0.5 / 50 = 0.01.

The return of the whole bet over the following month:

```text
R_strategy = sum(w_i * r_i for i on the bought side) - sum(w_i * r_i for i on the sold side)
```

- `r_i` is the return of company `i` over the month ahead, as a decimal.
- The sold side's return is kept by the strategy, because a short position gains when the price falls.

The cost of trading and of borrowing, exactly as in the previous tutorial:

```text
Cost = t * c + b * s
```

- `t` is the fraction of the account traded counting both sides.
- `c` is the cost per trade as a fraction of the amount traded, about 0.001, or ten basis points,
  where one basis point is one hundredth of one percent.
- `b` is the fraction held short.
- `s` is the borrow fee for one month, the rent on the shares that were sold, about 0.0004 to 0.0025.

## A worked example

Ten companies, with the two text measures read from their latest filings. The numbers are invented but
of a plausible size. Both measures are ranked separately, best first.

| Company | Lexical density | Specific density | Rank on LD | Rank on SD |
| ------- | --------------- | ---------------- | ---------- | ---------- |
| A       | 0.62            | 0.051            | 1          | 7          |
| B       | 0.60            | 0.081            | 2          | 1          |
| C       | 0.58            | 0.038            | 3          | 9          |
| D       | 0.55            | 0.077            | 4          | 2          |
| E       | 0.53            | 0.055            | 5          | 6          |
| F       | 0.51            | 0.070            | 6          | 3          |
| G       | 0.49            | 0.046            | 7          | 8          |
| H       | 0.47            | 0.066            | 8          | 4          |
| I       | 0.45            | 0.034            | 9          | 10         |
| J       | 0.43            | 0.060            | 10         | 5          |

With ten companies a tenth is one company on each measure, so the example buys A (best on lexical
density) and B (best on specific density), half the money each, and sells short I (worst on lexical
density) and J (worst on specific density), half the money each. Real implementations use 500
companies and hold about 50 names per basket; a single name here is far too concentrated.

Now suppose six months pass. The monthly cost is `t * c + b * s = 2.0 * 0.001 + 1.0 * 0.0025 =
0.0045`, that is 0.45 percent, because every position is replaced and the whole short side is
borrowed.

| Month | Bought side | Sold side | Gross (bought minus sold) | Cost  | Net    |
| ----- | ----------- | --------- | ------------------------- | ----- | ------ |
| 1     | +0.9%       | -0.4%     | +1.30%                    | 0.45% | +0.85% |
| 2     | +0.3%       | +0.2%     | +0.10%                    | 0.45% | -0.35% |
| 3     | -0.5%       | -0.7%     | +0.20%                    | 0.45% | -0.25% |
| 4     | +0.7%       | -0.1%     | +0.80%                    | 0.45% | +0.35% |
| 5     | +0.2%       | -0.3%     | +0.50%                    | 0.45% | +0.05% |
| 6     | -0.1%       | +0.4%     | -0.50%                    | 0.45% | -0.95% |
| Total |             |           | +2.40%                    | 2.70% | -0.30% |

The gross gain is 2.40 percent over six months, and the trading and borrow cost is 2.70 percent, so
the example ends slightly negative. Notice that the cost per rebuild is fixed by the rule, while the
gross return each month is not, so a month that trades a lot but earns little is a month the strategy
pays to stand still. The worked example says nothing about whether the strategy works; it shows only
how to apply the rules and how the arithmetic behaves.

## What the research actually found

| Source                                                                                                            | What it measured                                                      | Result                                                                                                                                                                                                                                                                                                          |
| ----------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Hanicova, Kalus and Vojtko, How to Use Lexical Density of Company Filings                                         | Language metrics on the Brain dataset, about 6,000 American companies | Reading density from the filings and going long the densest tenth and short the vaguest tenth produced a positive return; lexical richness, a third measure, was the weakest of the group and improved when the universe was widened to 500 stocks                                                              |
| Quantpedia, summarising that paper                                                                                | The strategy's compounding annual return, 2010 to 2021                | About 8.16 percent a year, volatility 10.4 percent, reward-to-risk 0.69, taken from the paper's table on page 6, with a market exposure of -0.029, meaning it moved slightly against the market                                                                                                                 |
| This repository, [language models, news and text](../../../strategies/books2/11_language_models_news_and_text.md) | Filing text as a firm-level signal, across several studies            | Word counts carry a signal: bankrupt firms use debt words 3.04 percent of the time against 2.58 percent for healthy firms, four years before the filing (`2101.00719v1`, p.15). Seven language models scoring the same transcripts agree only loosely, at a mean rank correlation of 0.52 (`2609.31013v1`, p.3) |
| The list's replication record                                                                                     | 4,843 coded papers, each over its own full history                    | The median replication has a reward-to-risk of 0.37, 48 percent clear a t-statistic of 1.96, the median test window is 34 years, and the median carries a market exposure of +0.17                                                                                                                              |

Read together, the picture is this. The words in a filing do carry information that the numbers do
not, and different studies measure it in different ways: a count of finance words here, a dictionary
of bankruptcy words there, the mere mention of artificial intelligence somewhere else. The specific
strategy in this page has one positive measured window and depends on a single commercial dataset for
its scores.

## How this project relates to it

Two briefs carry the same idea. The first is
[language models, news and text](../../../strategies/books2/11_language_models_news_and_text.md),
which is where the filing-text numbers above come from and which states plainly that a language-model
or dictionary score is a measurement whose value depends on the provider and the model chosen. The
second is
[market design, regulation and fees](../../../strategies/books2/20_market_design_regulation_and_fees.md),
whose filing study counts the rulebook itself: across more than 165,000 annual reports, references to
named laws rose from 8.4 per filing in 1996 to 31.7 in 2016, a 237 percent increase
(`1612.09244v2`, p.5). That is a direct measure of how the documents this strategy reads have grown,
and it supports the idea that there is now far more text to score than when the method was designed.
The closest finished tutorial is
[using news sentiment to predict the price direction of drug manufacturers](../../../tutorials/quantconnect/using-news-sentiment-to-predict-price-direction-of-drug-manufacturers/README.md),
which turns news text into a buy-or-sell signal for one industry.

## Where it goes wrong

- The score depends on the provider. Different language models and dictionaries score the same text
  differently, and the repository brief records a mean rank correlation of only 0.52 between seven
  models reading the same transcripts. A strategy built on one provider's numbers may not reproduce
  on another's.
- The documents are long and the effect is small. About 8 percent a year before costs on one dataset
  leaves little room, and reading errors or a change in how the provider computes a metric can move
  the result by more than the effect itself.
- The signal arrives in a crowd. All companies file on the same schedule, so whoever trades the
  signal does so in the same window as everyone else who reads the same dataset, which is exactly the
  crowding that erodes published effects.
- The rule pays a fixed toll. The worked example shows 2.70 percent of cost over six months against
  2.40 percent of gross return, so the strategy has to earn more than the cost of rebuilding a
  long-short book every month.
- The measure can be gamed. A company that learns the score is watched can pad a report with finance
  words and detail, which raises the measure without changing anything real.
- The whole idea would be false if density simply tracked firm size, industry or legal boilerplate,
  so that the ranking was sorting by accident rather than by disclosure. Controlling for those
  characteristics, and re-measuring on a second provider's data, is what would settle it.

## Try it yourself

You need a spreadsheet, a company's annual report from the regulator's public filing website, and
about half an hour. You do not need any money.

1. Download one company's latest 10-K and paste the text into a document.
2. Split it into words on spaces, and count the total number of words.
3. Build a short list of finance words, say thirty of them, and count how many times each appears.
4. Compute specific density as the finance-word count divided by the total word count.
5. Repeat for a second company in the same industry, so that the comparison is fair.
6. Sort the two companies by specific density and ask which one a reader learns more from by skimming
   the first page.

What to notice: the boilerplate at the front and back of every report is nearly identical across
companies, so most of the measure comes from a few pages in the middle. Notice too how much a single
long risk-factor section changes the count. If one company's density looks far higher, read the
sentences behind the count before believing the difference is disclosure rather than structure.

## Where this came from

- [The list's implementation file](https://github.com/paperswithbacktest/awesome-systematic-trading/blob/main/static/strategies/how-to-use-lexical-density-of-company-filings.py),
  the rules as coded: the top 500 American stocks, the two text metrics, the top and bottom tenths,
  monthly rebuilding.
- [Quantpedia: how to use lexical density of company filings](https://quantpedia.com/strategies/how-to-use-lexical-density-of-company-filings),
  the indicative performance figures and the description of the two metrics.
- Hanicova, Kalus and Vojtko, [How to Use Lexical Density of Company Filings](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=3921091),
  the original study of the three lexical measures.
- [Language models, news and text](../../../strategies/books2/11_language_models_news_and_text.md)
  and [market design, regulation and fees](../../../strategies/books2/20_market_design_regulation_and_fees.md),
  this repository's briefs, which supply `2101.00719v1`, `2609.31013v1` and `1612.09244v2`.

## Words used in this tutorial

- 10-K: a company's once-a-year report to the American market regulator.
- 10-Q: a company's quarterly report to the same regulator.
- basis point: one hundredth of one percent, so ten basis points is 0.10 percent.
- borrow fee: the rent paid to borrow shares that have been sold short.
- decile: a tenth of a ranked list, so the top decile is the best tenth.
- fundamental data: the numbers and statements a company reports about itself.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
