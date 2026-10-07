# The accrual anomaly: buying the companies whose profit is backed by cash

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                    |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of American companies, split into a long basket and a short basket                                                                                                                                                                                |
| How often it trades       | Once a year, when the portfolio is rebuilt in May                                                                                                                                                                                                        |
| What you need             | A spreadsheet and two years of company accounts                                                                                                                                                                                                          |
| Where the rules come from | [QuantConnect strategy library, accrual anomaly](https://www.quantconnect.com/tutorials/strategy-library/accrual-anomaly) and the [Quantpedia entry](https://quantpedia.com/strategies/accrual-anomaly) it cites                                         |
| The underlying research   | Sloan, [Do Stock Prices Fully Reflect Information in Accruals and Cash Flows About Future Earnings?](https://www.jstor.org/stable/248290)                                                                                                                |
| How well it held up       | Weak: the effect was strong over a long sample but has weakened or vanished out of sample, the direction of the decline is itself disputed, and the entry that summarises the work grades its confidence as weak and notes a negative out-of-sample test |
| Also appears in           | The [asset growth effect](../asset-growth-effect/README.md) tutorial in this collection, the other rule built from a line in the accounts                                                                                                                |

## The idea in one paragraph

Companies report a profit at the end of the year, but that profit is not the same as cash. Some of
it is cash that actually arrived; the rest is a set of bookkeeping entries that move profit forward
before the money shows up. The part of reported profit that is not yet backed by cash is called
accruals. This strategy buys the companies whose profit is mostly cash and bets against the ones
whose profit is mostly accruals. Both baskets are held for a year, then the whole thing is done
again after the annual reports are published. The reason offered is that investors look at the
headline profit and ignore how much of it arrived as cash, so they overpay for the companies whose
profit is largely a promise.

## Why anyone believed it

Cash is hard to fake and accruals are easier to stretch. A company that has booked a sale but has not
been paid yet counts the sale as profit immediately, and if it is slow to collect, or if the buyer
never pays, the profit has to be written back later. A company whose profit arrives as cash has
already proved that the money is real.

The story is that investors read the headline profit and do not split it into its cash part and its
accrual part. They grow too optimistic about the high-accrual companies and too gloomy about the
low-accrual ones. When the accruals fail to turn into cash, the price of the over-valued group falls.
The person on the other side is the inattentive investor who reads only the top line.

## An everyday comparison

Imagine a shop that sells furniture and lets customers pay in thirty days. In December it delivers
ten sofas and records ten sales, even though nobody has paid yet. The owner's books show a great
month, and a neighbour with a cash-only shop shows a smaller one, but the cash-only owner already
has the money in the till. If two of the ten customers never pay, the first shop's December looks
much weaker once the truth arrives. The strategy buys the cash-only shop and bets against the one
whose sales are still promises.

## The rules, step by step

1. Start with every company listed on the New York, American or Nasdaq exchanges whose accounts we
   can read. Several lines must be present and positive: total current assets, cash, total current
   liabilities, short-term debt, income tax payable, and the year's depreciation and amortisation.
   Companies missing any of these are left out.
2. Current assets are the things a company expects to turn into cash within a year, such as stock and
   money owed by customers. Current liabilities are the amounts it expects to pay within a year.
3. For each company, take the most recent accounts and the accounts one year earlier, and subtract
   the old value from the new for each of those lines.
4. Compute the accrual for the year with the formula below.
5. Sort every company from the smallest accrual to the largest.
6. Cut the sorted list into ten equal groups. The first decile has the lowest accruals, the tenth the
   highest.
7. Buy the first decile, spreading the money equally. Sell short the tenth decile, spreading equally.
   Short selling means borrowing shares you do not own, selling them now and buying them back later;
   if the price falls you keep the difference, and if it rises you lose.
8. Hold for a year. Rebuild both baskets in May, after the annual reports have been published, when
   the previous year's accounts are available for almost every company.

## The maths, with every symbol named

The accrual is built from the balance sheet, the page of the accounts that lists what a company owns
and owes on a chosen day, and from one line of the income statement, the page that records the year's
trading.

```text
BS_ACC = [ (dCA - dCash) - (dCL - dSTD - dITP) - Dep ] / AvgAssets
```

- `BS_ACC` is the accrual of the company for the year, a decimal. A positive number means reported
  profit was larger than the cash it produced; a negative number means the opposite.
- `dCA` is the change in current assets: this year's total minus last year's.
- `dCash` is the change in cash and near-cash holdings over the year.
- `dCL` is the change in current liabilities.
- `dSTD` is the change in the debt due within a year.
- `dITP` is the change in income tax payable.
- `Dep` is the year's depreciation and amortisation, the writing-down of long-lived assets such as
  machines and buildings as they wear out.
- `AvgAssets` is the average of total assets at the start and the end of the year, which scales the
  result so that a large company and a small one can be compared.

The bracket `(dCA - dCash)` is the rise in everything the company is owed or holds short-term, apart
from cash. The bracket `(dCL - dSTD - dITP)` is the rise in what it owes, apart from short-term debt
and tax. Subtracting the second from the first leaves the part of the increase that is neither cash
nor borrowing, and taking off depreciation leaves the accrual. Dividing by average assets turns it
into a percentage of the company's size.

Rank every company by `BS_ACC`. Each company inside a basket gets the same weight:

```text
w_i = 1 / 10
```

- `w_i` is the fraction of one side of the trade placed in company `i`.

The two sides are held in equal size, one unit long and one unit short, and the profit over the next
year is the low-accrual basket's return minus the high-accrual basket's return:

```text
S = R_low - R_high
```

- `S` is the spread return of the strategy for the year, a decimal.
- `R_low` is the average return of the ten companies with the lowest accruals.
- `R_high` is the average return of the ten with the highest accruals.

The cost of rebuilding both baskets each year:

```text
Cost = T * c
```

- `T` is the two-way turnover in units of one side. Replacing 40 percent of each basket is 0.4 sold
  plus 0.4 bought on the long side and the same on the short side, so `T = 1.6`.
- `c` is the cost of one trade as a fraction of the amount traded. High-accrual companies are often
  small and awkward to borrow, so a realistic working figure is 0.002, or 20 basis points, where one
  basis point is one hundredth of one percent.

The return the account keeps is `S - Cost`.

## A worked example

First, one company's accrual, from the changes in its accounts over a year, in millions of one
currency.

| Line                          | Last year | This year | Change      |
| ----------------------------- | --------- | --------- | ----------- |
| Current assets                | 500       | 600       | +100        |
| Cash and near-cash            | 100       | 130       | +30         |
| Current liabilities           | 200       | 250       | +50         |
| Short-term debt               | 50        | 60        | +10         |
| Income tax payable            | 20        | 26        | +6          |
| Depreciation and amortisation | -         | -         | 40          |
| Total assets                  | 900       | 1000      | average 950 |

The first bracket is 100 minus 30, which is 70. The second is 50 minus 10 minus 6, which is 34. Then
70 minus 34 minus 40 is minus 4. Divide by 950 to get minus 0.0042, so `BS_ACC` is about minus 0.42
percent, a low-accrual company, because its profit was more than fully backed by cash.

Now ten invented companies sorted by `BS_ACC`, with the following year's returns. The low decile is
A and the high decile is J, as before.

| Company | Accrual | Basket | Next-year return |
| ------- | ------- | ------ | ---------------- |
| A       | -0.05   | Low    | +10%             |
| B       | -0.03   | Low    | +6%              |
| C       | -0.01   | Low    | +9%              |
| D       | 0.00    | Low    | +4%              |
| E       | +0.01   | Low    | +8%              |
| F       | +0.02   | Low    | +7%              |
| G       | +0.03   | High   | -1%              |
| H       | +0.05   | High   | +3%              |
| I       | +0.07   | High   | +4%              |
| J       | +0.12   | High   | +2%              |

In a real study each decile holds hundreds of companies. Here we show the year's spread for six
consecutive years, using invented returns of realistic size.

| Year | Low side return | High side return | Spread | Replaced each side | Two-way turnover | Cost  | Net     |
| ---- | --------------- | ---------------- | ------ | ------------------ | ---------------- | ----- | ------- |
| 1    | +10.0%          | -2.0%            | +12.0% | 40%                | 1.6              | 0.32% | +11.68% |
| 2    | +6.0%           | +5.0%            | +1.0%  | 50%                | 2.0              | 0.40% | +0.60%  |
| 3    | +9.0%           | +2.0%            | +7.0%  | 30%                | 1.2              | 0.24% | +6.76%  |
| 4    | +4.0%           | -1.0%            | +5.0%  | 60%                | 2.4              | 0.48% | +4.52%  |
| 5    | +8.0%           | +3.0%            | +5.0%  | 40%                | 1.6              | 0.32% | +4.68%  |
| 6    | +7.0%           | +4.0%            | +3.0%  | 50%                | 2.0              | 0.40% | +2.60%  |

The arithmetic for year 1: the spread is 10.0 minus (-2.0), which is 12.0 percent. Turnover of 1.6
units at 20 basis points is 1.6 times 0.0020, which is 0.0032, or 0.32 percent. Net is 12.0 minus
0.32, which is 11.68 percent. Every other row is worked the same way.

The net values add to 30.84, an average of 5.14 percent a year. Compounding the six yearly factors
(1.1168 times 1.0060 times 1.0676 times 1.0452 times 1.0468 times 1.0260) gives 1.3534, so the money
grew by 35.34 percent over six years, which is 5.17 percent a year compounded. The gap between the
5.14 percent average and the 5.17 percent compounded figure is the drag from the losing years.

## What the research actually found

| Source                                    | What it measured                                                   | Result                                                                                                                                                                                                                                               |
| ----------------------------------------- | ------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Sloan, the original study                 | The relation between accruals and later returns on American shares | Documented the negative relation: companies with high accruals went on to earn less than companies with low accruals, which he read as evidence that investors fixate on the headline profit                                                         |
| Quantpedia, from the source paper's table | The low-minus-high portfolio, 1966 to 2003                         | 7.5 percent a year, volatility 10.26 percent, worst fall 35.58 percent, reward-to-risk 0.34, with a confidence grade of Weak and a note that the out-of-sample backtest was significantly negative and the in-sample result may have been data-mined |
| LaFond                                    | The same test in 17 countries, 1989 to 2003                        | The anomaly was present in many countries, and the mispricing was largest for the working-capital part of accruals                                                                                                                                   |
| Lev and Nissim                            | Whether the effect had been traded away                            | They found it still persisted and had not shrunk, and argued that the high-accrual companies were unattractive to most large investors and expensive for small ones to trade                                                                         |
| Mohanram                                  | Whether the effect weakened                                        | It generated strong returns for over four decades until 2002 but appeared to weaken afterwards, which he attributed in part to analysts publishing cash-flow forecasts that made the accrual part visible                                            |
| Bender and Nielsen                        | The same question, different window                                | The earnings-quality signal stopped working in the mid-2000s and then rebounded strongly after 2008, which they read as evidence it is a genuine signal rather than a fixed risk factor                                                              |

Read together: the accrual anomaly was real and large in its original sample, and even that sample's
reported figure is modest once the reward is set against the risk. Since then it has weakened, and
the sources disagree about whether that weakening is permanent, caused by other disclosure, or
already reversed. The disagreement is unresolved, and the summary entry grades its confidence as
weak.

## How this project relates to it

This repository's reading of the anomaly literature is in
[Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md).
Its conclusion, from a broad panel of studies, is that cross-sectional predictors are mostly
genuine, so the binding problems are decay, crowding and the integrity of the test rather than the
existence of the pattern.

The second brief is
[Overfitting, reproducibility and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md).
It reports that sorting shares on simple functions of hundreds of accounting variables produces tens
of thousands of apparent strategies, a substantial fraction of which clear a naive significance
bar. The accrual rule was found in exactly that kind of search, so the brief's advice to fix the
number of starts in advance applies directly to it.

## Where it goes wrong

- The formula is a choice, not a law. There are several accepted ways to compute accruals, and the
  different versions do not always give the same ranking, so a result can depend on which one was
  used.
- Short borrowing is the hardest part. High-accrual companies are often small, thinly traded and
  costly to borrow, and without the short side the strategy is only a long bet on steady companies.
- Costs ignored. A yearly rebuild looks cheap, but the small companies involved carry wide gaps
  between buying and selling prices, and the short side pays a borrow fee on top.
- Data snooping. The rule was found in a wide search over accounting variables, and the summary
  entry itself suspects the in-sample result was data-mined.
- The signal has genuinely weakened. Whatever the cause, the out-of-sample experience is worse than
  the original sample, and reporting the original figure alone would be misleading.
- What would have to be true for the idea to be false: that investors already price the split between
  cash and accruals correctly, so there is nothing to trade, and that the original finding was a
  property of the 1966 to 2003 sample rather than of markets in general.

## Try it yourself

You need a spreadsheet and two years of accounts for about twenty companies in one industry.

1. Build a table with one row per company and these columns: current assets last year, current assets
   this year, cash last year, cash this year, current liabilities last year, current liabilities this
   year, short-term debt last year, short-term debt this year, tax payable last year, tax payable
   this year, depreciation, total assets last year, total assets this year.
2. Add a column for each of the changes: current assets, cash, current liabilities, short-term debt,
   tax payable.
3. Add a column for average total assets: total assets last year plus this year, divided by two.
4. Add a column for the accrual: (change in current assets minus change in cash) minus (change in
   current liabilities minus change in short-term debt minus change in tax payable) minus
   depreciation, all divided by average total assets.
5. Sort by that column and split the twenty into the lowest four and the highest four.
6. Average the past year's return of each group and subtract the high group's average from the low
   group's average.

What to notice: in most industries the two groups differ by only a few percentage points, and a
single company with an unusual year can dominate both group averages. Add a column for each
company's industry and check whether the result survives when you keep only one industry. If it does
not, the apparent signal may be industry membership rather than accruals.

## Where this came from

- [QuantConnect strategy library: accrual anomaly](https://www.quantconnect.com/tutorials/strategy-library/accrual-anomaly),
  the implementation: the balance-sheet accrual formula, the top and bottom deciles, rebuilt in May.
- [Quantpedia: accrual anomaly](https://quantpedia.com/strategies/accrual-anomaly), the performance
  figures, the confidence grade and the underlying papers.
- Sloan, [Do Stock Prices Fully Reflect Information in Accruals and Cash Flows About Future Earnings?](https://www.jstor.org/stable/248290),
  the paper that first documented the anomaly.
- [Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md)
  and [Overfitting, reproducibility and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md),
  this repository's own reading of the predictability and data-mining evidence.

## Words used in this tutorial

- accruals: the part of reported profit that is not yet backed by cash, from entries recorded before
  the money arrives.
- asset: something a company owns that has value, such as a building, a machine or cash owed to it.
- balance sheet: the page of a company's accounts that lists everything it owns and owes on a chosen
  day.
- cash flow: the money that actually moved in or out of a company over a period, as opposed to the
  profit the accounts show.
- current assets: the things a company expects to turn into cash within a year.
- current liabilities: the amounts a company expects to pay within a year.
- decile: one of ten equal groups after sorting, so the first decile is the lowest tenth.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
