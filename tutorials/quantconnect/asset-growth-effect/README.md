# The asset growth effect: betting against the companies that spent the most

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                             |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of American companies that are not banks or insurers, split into two baskets                                                                                                                                                                                                                               |
| How often it trades       | Once a year, when the portfolio is rebuilt at the end of June                                                                                                                                                                                                                                                     |
| What you need             | A spreadsheet and two years of company balance sheets                                                                                                                                                                                                                                                             |
| Where the rules come from | [QuantConnect strategy library, asset growth effect](https://www.quantconnect.com/tutorials/strategy-library/asset-growth-effect) and the [Quantpedia entry](https://quantpedia.com/strategies/asset-growth-effect) it cites                                                                                      |
| The underlying research   | Cooper, Gulen and Schill, [The Asset Growth Effect in Stock Returns](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1335524)                                                                                                                                                                                 |
| How well it held up       | Mixed: the negative relation between asset growth and later returns repeats across many samples and dozens of countries, but its size depends on how the baskets are weighted and how the returns are counted, and one careful study attributes much of it to a flaw in the data rather than to the effect itself |
| Also appears in           | The [accrual anomaly](../accrual-anomaly/README.md) tutorial in this collection, the other rule built from a line in the company accounts                                                                                                                                                                         |

## The idea in one paragraph

Every company publishes a set of accounts once a year, and the first page lists everything it owns:
buildings, machines, stock, cash owed to it. That list is called the balance sheet, and the total of
everything the company owns is called its total assets. Work out how much that total grew from one
year to the next, as a percentage. This strategy buys the companies whose assets grew the slowest
and bets against the ones whose assets grew the fastest. The bet is held for a year and then the
whole thing is done again. The reason offered is that a company which expands fast is often
spending on things that do not pay off, while investors pay too much for it because they assume the
growth will continue.

## Why anyone believed it

Two stories are given. The first is about risk. A young company holds many plans for things it might
build one day, which researchers call growth options. As it actually builds them, those plans become
ordinary, already-paid-for assets, which are safer. A safer company does not have to promise as much
to attract money, so its shares earn less. The second story is about behaviour: when a company has
grown quickly, investors take the past growth and assume the future will look the same, so they pay
a high price. When the growth does not continue, the price falls.

The person on the other side of this trade is that over-optimistic buyer of the fast-growing
company, who keeps paying up after the news is already in the price. The seller of the slow-growing
company is often an investor who finds it dull, or a fund that has to trim a position for reasons
unrelated to the company's prospects.

## An everyday comparison

Think of two neighbouring restaurants. One keeps its menu, raises prices gently and adds a single
table each year. The other borrows money, opens six new branches in eighteen months and hires fast.
Diners crowd the new places because something is clearly happening, and the owner is celebrated.
Then the sixth branch has to be paid for, the novelty fades, and the borrowed money has to be repaid
out of thinner earnings. The steady restaurant is not exciting, but it is rarely the one that closes.
The strategy buys the steady restaurant's owner and bets against the expanding one.

## The rules, step by step

1. Start with every company listed on the New York, American or Nasdaq exchanges that is not a bank,
   an insurer or another financial firm, and whose balance sheet we can read. Total assets must be a
   positive number.
2. For each company read total assets for the most recent year and for the year before, from the
   same line of the balance sheet in the annual report.
3. Work out asset growth: assets this year minus assets last year, divided by assets last year. A
   company that went from 100 million to 110 million grew by 10 percent.
4. Sort all the companies from the smallest growth to the largest.
5. Cut the sorted list into ten equal groups, called deciles. The first decile is the slowest-growing
   tenth, the tenth decile is the fastest-growing tenth.
6. Buy the first decile, spreading the money equally across its companies.
7. Sell short the tenth decile, spreading equally. Short selling means borrowing shares you do not
   own, selling them now and buying them back later; if the price falls you keep the difference, and
   if it rises you lose.
8. Hold for one year. At the end of the next June, after the annual reports have arrived, recompute
   from step 2 and rebuild both baskets.
9. If short selling is not available to you, the long-only version simply owns the slowest-growing
   tenth. This tutorial works through the two-sided version because that is the one both sources
   state.

## The maths, with every symbol named

The ranking is one division per company.

The asset growth of one company:

```text
G = (A_now - A_last_year) / A_last_year
```

- `G` is the asset growth of the company, written as a decimal: 0.10 means 10 percent.
- `A_now` is total assets on the most recent balance sheet, in a single currency.
- `A_last_year` is total assets on the balance sheet one year earlier.

Rank every company by `G`. The slowest-growing tenth is the long side; the fastest-growing tenth is
the short side. Each company inside a basket gets the same weight:

```text
w_i = 1 / 10
```

- `w_i` is the fraction of one side of the trade placed in company `i`.
- A decile holds ten companies, so each gets one tenth of that side.

The two sides are held in equal size, one unit long and one unit short. The profit of that pairing
over the following year is the long basket's return minus the short basket's return:

```text
S = R_low - R_high
```

- `S` is the spread return of the strategy for the year, written as a decimal.
- `R_low` is the average return of the ten slowest-growing companies.
- `R_high` is the average return of the ten fastest-growing companies.
- Because the short basket is sold first and bought back later, a fall in `R_high` helps the
  strategy, which is why it is subtracted.

Finally the cost of rebuilding both baskets each year:

```text
Cost = T * c
```

- `T` is the two-way turnover, counted in units of one side of the trade. Replacing 40 percent of
  the long basket means selling 0.4 and buying back 0.4, and the same again on the short side, so
  `T = 1.6`. `T` is 0 when nothing changes.
- `c` is the cost of one trade as a fraction of the amount traded, covering the gap between the
  buying and the selling price plus commission. For the large, liquid companies in this universe
  0.0015, or 15 basis points, is a reasonable working figure; one basis point is one hundredth of
  one percent.

The return the account keeps is `S - Cost`.

## A worked example

Ten invented companies, chosen so the arithmetic is easy to follow. Total assets are in millions of
one currency.

| Company | Assets last year | Assets this year | Growth | Rank |
| ------- | ---------------- | ---------------- | ------ | ---- |
| A       | 100              | 101              | 0.010  | 1    |
| B       | 200              | 206              | 0.030  | 2    |
| C       | 150              | 160              | 0.067  | 3    |
| D       | 300              | 330              | 0.100  | 4    |
| E       | 250              | 280              | 0.120  | 5    |
| F       | 400              | 460              | 0.150  | 6    |
| G       | 120              | 140              | 0.167  | 7    |
| H       | 500              | 600              | 0.200  | 8    |
| I       | 180              | 225              | 0.250  | 9    |
| J       | 90               | 120              | 0.333  | 10   |

With ten companies each decile is a single company, so the long basket is A and the short basket is
J. Now suppose the next six years bring the spread returns below, with the ranking rebuilt each
June.

| Year | Low side return | High side return | Spread | Replaced each side | Two-way turnover | Cost  | Net     |
| ---- | --------------- | ---------------- | ------ | ------------------ | ---------------- | ----- | ------- |
| 1    | +14.0%          | -6.0%            | +20.0% | 40%                | 1.6              | 0.24% | +19.76% |
| 2    | -3.0%           | +4.0%            | -7.0%  | 50%                | 2.0              | 0.30% | -7.30%  |
| 3    | +9.0%           | +1.0%            | +8.0%  | 30%                | 1.2              | 0.18% | +7.82%  |
| 4    | +5.0%           | -9.0%            | +14.0% | 60%                | 2.4              | 0.36% | +13.64% |
| 5    | +11.0%          | -2.0%            | +13.0% | 40%                | 1.6              | 0.24% | +12.76% |
| 6    | -1.0%           | +6.0%            | -7.0%  | 50%                | 2.0              | 0.30% | -7.30%  |

The arithmetic for year 1: the spread is 14.0 minus (-6.0), which is 20.0 percent. Turnover of 1.6
units at 15 basis points is 1.6 times 0.0015, which is 0.0024, or 0.24 percent. Net is 20.0 minus
0.24, which is 19.76 percent. Every other row is worked the same way.

The net values add to 39.38, and 39.38 divided by 6 is an average of 6.56 percent a year. Compounding
the six yearly factors (1.1976 times 0.9270 times 1.0782 times 1.1364 times 1.1276 times 0.9270)
gives 1.4218, so the money grew by 42.18 percent over six years, which is 6.04 percent a year
compounded. The gap between the average and the compounded figure is the drag from the losing years,
and reporting both is the honest way to state it.

Two cautions about this example. First, the 20 percent spread in year 1 is close to the headline
figure the papers report, but those papers rank thousands of companies, and the extreme tenth of
thousands is a more extreme group than the extreme tenth of ten. Second, the example says nothing
about whether the rule is real; it only shows how to apply it and how the arithmetic behaves.

## What the research actually found

| Source                                             | What it measured                                                                              | Result                                                                                                                                                                                                                                                                                                                                                    |
| -------------------------------------------------- | --------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Cooper, Gulen and Schill, drawing on American data | All non-financial listed companies, sorted yearly on the change in total assets, 1968 to 2006 | Low-growth companies beat high-growth companies by about 20 percent a year across the sample; the premium starts in the January after the measurement year and lasts up to five years; it appears in large companies as well as small; the authors wrote that risk-based explanations have difficulty accounting for something so large and so consistent |
| Quantpedia, summarising that work                  | The same rule, monthly returns                                                                | 20.84 percent a year, volatility 14.07 percent, worst fall 31.37 percent, reward-to-risk 1.2, 1968 to 2006, and a confidence grade of Strong                                                                                                                                                                                                              |
| Watanabe, Xu, Yao and Yu                           | The same sort in 40 international markets                                                     | Higher asset growth went with lower later returns in all of them, and the effect was stronger in developed markets and in markets where shares are more efficiently priced                                                                                                                                                                                |
| Fu                                                 | The same sort, correcting two data problems                                                   | The low-growth basket's advantage shrank once companies that left the exchange were counted properly, and the high-growth basket's weakness came mostly from firms that had just raised large amounts of debt or sold new shares; with both corrected, no separate asset-growth effect remained                                                           |
| He, Miao, Kapadia and Tice                         | A measure of how much investors associate fast growth with a "next big thing"                 | Returns to the asset-growth factor were 17.5 percent over the three years after months when that association was strong, and 5.4 percent after months when it was weak                                                                                                                                                                                    |

Read together: the negative relation is one of the better replicated results in accounting-based
investing, and it shows up in many countries. But the measured size moves a lot with the choices
made, and the strongest single critique says a good part of it is a data artefact rather than a
tradable edge. The disagreement is not about the sign; it is about how much of the prize is real.

## How this project relates to it

This repository studies whether cross-sectional predictors of the kind used here survive, in
[Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md).
Its summary of the evidence is that most of the factor zoo is genuine, so the binding problem is not
discovery but decay and crowding, and it notes that large, liquid shares show weaker predictability
than small ones. The asset-growth rule is a small-share effect as much as a large-share one, which
is exactly where that warning bites.

The second relevant brief is
[Overfitting, reproducibility and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md).
It reports that sorting shares on simple functions of hundreds of accounting variables produces tens
of thousands of apparent strategies, a large fraction of which clear a naive significance bar, and
that the honest response is to fix the number of starts in advance rather than to pick the best
finished rule. The asset-growth rule was found in that kind of search.

## Where it goes wrong

- Data errors in the raw returns. The Fu paper's result is the warning: if companies that leave the
  exchange are dropped instead of counted, the low-growth basket looks better than it was.
- The weighting choice. Equal-weighted and value-weighted versions of the same sort give different
  numbers, and the effect is much larger among small companies, which are the expensive and awkward
  ones to trade.
- Shorting costs and availability. The short side needs shares to borrow, and precisely the
  fast-growing, heavily promoted companies can be hard or expensive to borrow. Part of a measured
  spread can be unavailable in practice.
- Crowding and decay. Once the rule is widely known the buying arrives earlier and the spread
  narrows, and the effect is at its largest in the small, illiquid part of the market where most
  money cannot go.
- Annual accounts arrive late and can be restated. The numbers used at the end of June are the ones
  published by then, and a later correction to a balance sheet would change yesterday's ranking.
- What would have to be true for the idea to be false: that the relation between growth and later
  returns is fully explained by the data corrections and the risk of the growing firms, leaving
  nothing extra to trade. The Fu paper is close to that position.

## Try it yourself

You need a spreadsheet and a public source of company accounts, such as the filings a company
publishes on its investor-relations page.

1. Pick 20 companies from one industry, so the comparison is fair.
2. Build a table with one row per company and these columns: name, total assets two years ago, total
   assets one year ago, asset growth, rank, return over the last year.
3. Fill the growth column with assets one year ago divided by assets two years ago, minus one, in
   percent.
4. Fill the rank column by sorting growth from smallest to largest, so rank 1 is the slowest.
5. Take the four smallest (the bottom fifth) and the four largest (the top fifth). Average the past
   year's returns of the two groups separately.
6. Subtract the top group's average from the bottom group's average. That number is the spread this
   rule would have captured over that one year.

What to notice: with only twenty companies, one surprising year in a single company can flip the
whole result, which is why the papers use thousands. Do it for five different years and watch how
often the sign of the spread changes. The honest finding is that the sign is not stable from year to
year even when the full-sample average is not zero.

## Where this came from

- [QuantConnect strategy library: asset growth effect](https://www.quantconnect.com/tutorials/strategy-library/asset-growth-effect),
  the rules as implemented: non-financial listed companies, the year-on-year change in total assets,
  the top and bottom deciles, rebuilt each June.
- [Quantpedia: asset growth effect](https://quantpedia.com/strategies/asset-growth-effect), the
  performance figures, the instrument count, the confidence grade and the underlying papers.
- Cooper, Gulen and Schill, [The Asset Growth Effect in Stock Returns](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1335524),
  the original large-sample study and the source of the 20 percent figure.
- Fu, [What Is behind the Asset Growth and Investment Growth Anomalies?](http://ink.library.smu.edu.sg/cgi/viewcontent.cgi?article=4158&context=lkcsb_research),
  the critique that the effect is largely a data artefact.
- [Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md)
  and [Overfitting, reproducibility and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md),
  this repository's own reading of the predictability and data-mining evidence.

## Words used in this tutorial

- asset: something a company owns that has value, such as a building, a machine or cash owed to it.
- balance sheet: the page of a company's accounts that lists everything it owns and everything it
  owes on a chosen day.
- decile: one of ten equal groups after sorting, so the first decile is the lowest tenth.
- growth option: a plan a company holds to invest in the future, not yet built or paid for.
- risk: the chance that a price moves against you; a riskier holding has to offer more to attract
  buyers.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- spread return: the profit on a paired long and short holding, being the long return minus the
  short return.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
