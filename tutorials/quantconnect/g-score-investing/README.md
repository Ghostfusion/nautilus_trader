# The G-score: picking technology shares by scoring the quality of their accounts

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                        |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of American technology companies, chosen by a score built from the items in their published annual accounts                                                                                                                           |
| How often it trades       | Once a year, when new annual accounts are published and the scores are recomputed                                                                                                                                                            |
| What you need             | Python and a data file of company accounts                                                                                                                                                                                                   |
| Where the rules come from | [QuantConnect strategy library, G-score investing](https://www.quantconnect.com/tutorials/strategy-library/g-score-investing)                                                                                                                |
| The underlying research   | Mohanram, [Separating Winners from Losers among Low Book-to-Market Stocks using Financial Statement Analysis](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=403180), Review of Accounting Studies, 2005                                |
| How well it held up       | Mixed: a large original sample, robust across company size, industry and year, but the library's own long-only technology version underperformed its index over 2016 to 2020 and the paper's own tests show two of its eight items were weak |
| Also appears in           | [Earnings quality](../earnings-quality-factor/README.md) in this same library, its closest relative: another score that ranks shares by the quality of their reported profit                                                                 |

## The idea in one paragraph

Some companies are priced at many times the value of the assets on their books. That happens for two
reasons: because investors expect them to grow quickly, or because the accounts do not show what the
company really owns. On average this expensive-looking group does worse than the market, because the
market tends to expect too much of it. But the group is not uniform, so this strategy gives every
company in it a score from a checklist about its profit, its cash, the steadiness of both, and how
much it spends on research, new equipment and advertising, and buys the shares with high scores. The
bet is that the accounts say which of these companies are real and which are only enthusiasm.

## Why anyone believed it

A company that has grown fast and is priced expensively attracts attention: analysts follow it,
newspapers write about it, and its shares keep rising, so people expect the growth to continue and the
price reflects that. When the growth slows even slightly the price falls hard, which is the
well-documented average behaviour of this group and the reason it is a discouraging place to invest.

The counter-argument is that the average hides a wide spread: two companies can be priced expensively
for completely different reasons, one with a genuine business that keeps improving and one that has
been talked up. The accounts distinguish them, because profit backed by cash, earned steadily,
alongside heavy spending on research and equipment, is hard to fake, while one good year of reported
profit is easy to produce. The counterparty is the investor watching the share price and the
headlines rather than the accounts. The paper adds that some of these companies look expensive because
accounting rules wrote off spending that had actually created something valuable, so their book value
understates what they own.

## An everyday comparison

Two second-hand cars are parked side by side, both with the same sticker price and both with polished
paint. One comes with a folder of service records going back years, showing what was replaced and
when; the other comes with a promise that it has always been looked after. The folder is real evidence
and the promise is not, so this strategy is the buyer who reads the folder, choosing which car to keep
rather than which to look at.

## The rules, step by step

First, a word about the name. In investing, governance means the way a company is run and supervised:
who sits on its board, who owns it, how its auditors are chosen, what rights a small shareholder has,
and what it must tell the public. A score built from those things is a governance score, and the letter
G is used for it. The score here is not that: its G stands for growth, because the companies being
scored are the ones the market values at many times their book value.

1. Take every company with accounts data and compute its book-to-market ratio: the net tangible
   assets, meaning what would be left for shareholders if the physical and financial assets were
   counted and the intangible ones set aside, divided by the market value of all its shares. A low
   ratio means a lot is being paid for each unit of book value.
2. Keep the bottom quarter of companies by that ratio, the ones with the highest market value relative
   to their assets. The study keeps the bottom fifth rather than the bottom quarter, and the library
   then keeps only technology companies, while the study used the whole group and reported technology
   separately.
3. Award one point for each of the seven conditions below, using the latest annual accounts. Every
   median mentioned is the median for other companies in the same industry at the same moment.
   1. Return on assets, meaning profit divided by total assets, is above the industry median.
   2. Cash flow return on assets, meaning the cash the business generated divided by total assets, is
      above the industry median.
   3. Cash flow return on assets is above return on assets, the check that reported profit is backed
      by money rather than by accounting entries.
   4. The variance of the return on assets over the last twelve quarters is below the industry median,
      the check that profit arrives steadily rather than in one lucky year.
   5. Spending on research and development, relative to total assets, is above the industry median.
   6. Spending on buildings and equipment, relative to total assets, is above the industry median.
   7. Spending on advertising and selling, relative to total assets, is above the industry median.
4. Add the points. The score runs from 0 to 7. The study uses eight items, adding a check on how
   steady the growth in sales has been, which the library leaves out.
5. Buy the shares of every company scoring 5 or more, in equal amounts, and rebuild when the next set
   of annual accounts is published. The study forms its lists four months after each company's
   financial year ends, so the accounts are already public and nobody is using information early.

One difference to keep in view: the study also sells short the weakest group, so its result does not
depend on the market rising, while the library's version is long only. The weak group is where most of
the measured difference lives, so the long-only version is a weaker proposition.

## The maths, with every symbol named

The ratio that decides which companies are looked at:

```text
BM = net_tangible_assets / market_capitalisation
```

- `BM` is the book-to-market ratio, a decimal: 0.10 means the assets on the books are worth a tenth of
  the market value of the shares.
- `net_tangible_assets` is what would be left for shareholders if the physical and financial assets
  were counted and the intangible ones set aside, and `market_capitalisation` is the share price times
  the number of shares. A small `BM` selects the companies this strategy studies, the opposite of the
  value style, which looks for large ones.

The measurements of profit and cash, the steadiness check and the three spending checks:

```text
ROA    = net_income / total_assets
CFROA  = operating_cash_flow / total_assets
VARROA = variance of ROA over the last twelve quarters
RD     = research_and_development / total_assets
CAPEX  = capital_expenditure / total_assets
AD     = advertising_and_selling / total_assets
```

- `ROA` is the return on assets, how much profit the company produced for each unit of assets it owns.
  `net_income` is the reported profit and `total_assets` everything the company owns; the study
  divides by the assets at the start of the year, so profit is compared with the resources that
  produced it.
- `CFROA` is the cash flow return on assets, and `operating_cash_flow` the cash that arrived from
  trading after paying the bills. `CFROA - ROA` is positive when the business collected more cash than
  it reported as profit, the sign of solid earnings, and negative when profit depends on entries that
  are not cash.
- `VARROA` is how widely the return on assets has swung from quarter to quarter; a small number means
  reliable profit. The study computes it over the last five years.
- `RD`, `CAPEX` and `AD` are the three kinds of spending, each divided by total assets so that small
  and large companies can be compared: money spent trying to invent things, money spent on buildings,
  machines and equipment, and money spent winning customers.

The score, and the rule that turns it into a portfolio:

```text
Score = G1 + G2 + G3 + G4 + G5 + G6 + G7
each Gj is 1 when its condition holds and 0 when it does not
buy every company with Score >= 5
```

- `G1` to `G7` are the seven conditions in the order of the rules, and `Score` is a whole number from
  0 to 7; the study's version goes from 0 to 8. Because each condition compares a company with its
  industry median, a company can score well in a weak industry by being the best of a poor group.

## A worked example

Eight technology companies, all in the bottom quarter by book-to-market. Each column is one of the
seven conditions; `yes` earns a point. The answers are invented.

| Company | 1. Profit above median | 2. Cash flow above median | 3. Cash above profit | 4. Profit steady | 5. Research heavy | 6. Equipment heavy | 7. Advertising heavy | Score | Bought |
| ------- | ---------------------- | ------------------------- | -------------------- | ---------------- | ----------------- | ------------------ | -------------------- | ----- | ------ |
| A       | yes                    | yes                       | yes                  | yes              | yes               | no                 | no                   | 5     | yes    |
| B       | yes                    | yes                       | no                   | no               | yes               | yes                | yes                  | 5     | yes    |
| C       | no                     | no                        | no                   | yes              | no                | yes                | no                   | 2     | no     |
| D       | yes                    | no                        | yes                  | yes              | yes               | yes                | yes                  | 6     | yes    |
| E       | no                     | yes                       | yes                  | no               | no                | no                 | no                   | 2     | no     |
| F       | yes                    | yes                       | yes                  | yes              | no                | yes                | yes                  | 6     | yes    |
| G       | no                     | no                        | no                   | no               | yes               | no                 | yes                  | 2     | no     |
| H       | yes                    | yes                       | yes                  | no               | yes               | yes                | no                   | 5     | yes    |

Five of the eight score 5 or more, so the basket is A, B, D, F and H in equal amounts. Now five years
of returns for that basket against a technology index fund, with a yearly rebuild. The returns are
invented, and the cost assumes that about half the basket changes each year, so one unit of the
account is traded counting both the sale of a leaver and the purchase of a joiner, at ten basis points.

| Year | Basket | Index fund | Basket minus index | Cost  | Basket after cost |
| ---- | ------ | ---------- | ------------------ | ----- | ----------------- |
| 1    | +22.0% | +20.0%     | +2.0%              | 0.10% | +21.9%            |
| 2    | +11.0% | +14.0%     | -3.0%              | 0.10% | +10.9%            |
| 3    | +30.0% | +24.0%     | +6.0%              | 0.10% | +29.9%            |
| 4    | -9.0%  | -7.0%      | -2.0%              | 0.10% | -9.1%             |
| 5    | +17.0% | +13.0%     | +4.0%              | 0.10% | +16.9%            |

```text
Basket after cost, five years = 1.219 * 1.109 * 1.299 * 0.909 * 1.169 = 1.8660, so +86.6 percent
Index fund, five years        = 1.200 * 1.140 * 1.240 * 0.930 * 1.130 = 1.7827, so +78.3 percent
Difference                    = 1.8660 / 1.7827 = 1.047, so about +4.7 percent over five years
```

Two things are worth noticing. The basket wins in three years out of five and still ends only about
4.7 percent ahead after five years, under one percent a year, because the winning years are not much
better and a bad year costs a lot; the yearly rebuild cost is about the size of that advantage. The
example shows arithmetic and cannot say what the score will do.

## What the research actually found

| Source                                                                             | What it measured                                                                                                                             | Result                                                                                                                                                                                                                                                                                             |
| ---------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Mohanram, Separating Winners from Losers                                           | 20,866 company-years of American companies in the lowest fifth by book-to-market, 1979 to 1999                                               | The whole group earned size-adjusted returns of -6.0 percent and -4.2 percent in the first two years after selection. Companies scoring 6 to 8 earned 17.4 percent raw against -4.0 percent for those scoring 0 or 1, a 21.4 percent difference; size-adjusted, +3.3 percent against -17.9 percent |
| The same study, on steadiness of the result                                        | The same sample split by year, size, industry, listing venue and analyst coverage                                                            | The difference was positive in all 21 years and significant in 16 of them, and it held for large companies, for companies with many analysts, and for technology companies alone, where the strong group earned 5.2 percent                                                                        |
| The same study, on the individual items                                            | Each of the eight items on its own                                                                                                           | All eight separated winners from losers in at least one of the two years, but two were weak: spending on equipment was significant only in the second year and advertising spending only in the first                                                                                              |
| Hsu, Kalesnik and Kose, as reported in this collection's earnings quality tutorial | A wide set of definitions of what makes a company high quality                                                                               | Profitability, accounting quality, payouts and investment carried a return premium, while capital structure and earnings stability showed little evidence of one                                                                                                                                   |
| QuantConnect implementation, April 2016 to September 2020                          | Technology companies in the bottom quarter by book-to-market, seven items, buying scores of 5 or more, against the technology index fund QQQ | Reward-to-risk of 0.609 against 1.002 for simply holding QQQ                                                                                                                                                                                                                                       |

Read together, the picture is this. The study found a large and unusually consistent difference
between the strong and weak companies in its group, and it survived checks against company size,
industry, listing venue and analyst coverage. Three things cut the result down for anyone using it
today. The difference came mostly from the weak companies: the strong group beat its peers by only 3.3
percent a year while the weak group fell 17.9 percent behind, so the library's long-only version gives
up the whole second half. Two items were weak even in the original study, and the library's own run
over four and a half years did worse than holding the index fund.

The study is explicit about what it assumes rather than proves: its items rest on the belief that the
low book-to-market group is mispriced rather than fundamentally riskier, and in its own regression one
extra point on the score was worth about 3.7 percent more return the next year, after other effects.

## How this project relates to it

The score is a cross-sectional factor: it ranks companies against each other rather than forecasting
the market. This repository has a manual for that style of work,
[Factor research and portfolio construction](../../../docs/usermanauls/factor-portfolio/README.md). It
turns a ranking into weights, splits the period so the part used to build the rule is never the part
used to judge it, and corrects the reported reward-to-risk figure for the number of things that were
tried. Its worked example uses a five-day momentum factor rather than accounting data, so it supplies
the machinery and not the score.

The repository's brief on predictability,
[Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
reports that the large collection of published cross-sectional signals is probably mostly genuine,
with a false-discovery rate bounded at 8.5 to 25 percent, but that the same signals weaken sharply
once large companies count for more and known factors are adjusted for, with the bound rising to 41.7
percent. It also reports a model in which a published signal's half-life shortens as more money runs
it.

The governance brief,
[Manipulation, fraud and governance](../../../strategies/books2/19_manipulation_fraud_and_governance.md),
covers the other meaning of governance: manipulation of prices and settlement, the detectors that try
to catch it, and the governance of trading venues. It says nothing about boards, auditors or
shareholder rights, so it is not the source of this score.

## Where it goes wrong

- Two of the seven items were weak from the start. Spending on equipment and spending on advertising
  each separated winners from losers in only one of the two measured years, and the author says so.
- The long-only version loses most of the effect: size-adjusted, the strong companies beat their peers
  by only 3.3 percent a year while the weak companies fell 17.9 percent behind, and removing the
  ability to sell short removes most of the difference.
- The cut-off is coarse. A company failing one item drops out of the basket entirely, and the
  difference between a score of 4 and a score of 5 is enormous, a sign the score is being used as a
  label rather than as a measurement.
- Accounting can be chosen, and must be read late. The same item can be measured several ways,
  companies can change their own accounting, and comparing with an industry median lets a company
  score well by being the best of a weak industry. Using an annual report before the date it appeared
  would also make any test of this rule look better than it could ever have been.
- It is a bet on why the shares are cheap. The study assumes the low book-to-market group is mispriced
  rather than fundamentally riskier, and if the opposite is true then buying cheap-looking assets is
  being paid for risk rather than gaining an advantage.

## Try it yourself

You need a spreadsheet and the annual accounts of a handful of listed technology companies, which any
company's investor relations page will give you.

1. Make one row per company with these columns: Net tangible assets, Market value of all shares,
   Book-to-market, Total assets, Net income, Operating cash flow, Return on assets, Cash flow return
   on assets, and the difference between the last two.
2. Book-to-market is net tangible assets divided by market value. Sort the rows by it and keep the
   bottom quarter.
3. Add columns for the return on assets over each of the last twelve quarters, then one holding the
   variance of those twelve numbers.
4. Add columns for research and development, spending on buildings and equipment, and advertising and
   selling, each divided by total assets, then seven yes or no columns, one for each condition in the
   rules, comparing each company with the median of the companies in your sheet, and a final column
   totalling the yes answers.

What to notice: how few companies reach 5 out of 7, and how much the answer changes when you swap one
item or change the comparison from the median to a number the company reports itself. That fragility
is why the study checks its result against size, industry and year. Then compare the high scorers with
the low ones over the following year; with a handful of companies the difference will be mostly luck,
which is the point of doing it with a handful.

## Where this came from

- [QuantConnect strategy library: G-score investing](https://www.quantconnect.com/tutorials/strategy-library/g-score-investing),
  the rules as implemented: the bottom quarter by book-to-market, technology only, seven conditions, a
  total of 5 or more, and the comparison over April 2016 to September 2020 against QQQ.
- Mohanram,
  [Separating Winners from Losers among Low Book-to-Market Stocks using Financial Statement Analysis](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=403180),
  Review of Accounting Studies, 2005, pages 133 to 170. The figures above are from the
  [accepted manuscript](https://utoronto.scholaris.ca/bitstreams/534ba0fe-3d8c-4d73-87df-6360fc3be80c/download),
  where Sections 3.2 to 3.4 define the eight items, Table 1 describes the 20,866 company-years, Table 4
  gives the returns by score, and Tables 5 to 7 give the checks by size, industry and year.
- [Factor research and portfolio construction](../../../docs/usermanauls/factor-portfolio/README.md),
  this repository's manual on ranking instruments honestly, splitting the sample and correcting for how
  many things were tried.
- [Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  which reports the false-discovery bounds of 8.5 to 25 percent rising to 41.7 percent under value
  weighting (`2206.15365v10`), at [2206.15365](https://arxiv.org/abs/2206.15365).
- [Manipulation, fraud and governance](../../../strategies/books2/19_manipulation_fraud_and_governance.md),
  the brief covering the other meaning of governance, which this score does not use.

## Words used in this tutorial

- book-to-market: the value of a company's assets on its books divided by the market value of its
  shares; a low number means the market pays a lot for those assets.
- corporate governance: the rules and structures by which a company is directed and supervised, such
  as its board, its auditors and the rights of its shareholders; this score does not use them.
- growth company: one the market values at many times its book value because it is expected to grow
  quickly; the group this score starts from.
- return on assets: profit divided by the assets that produced it, a measure of how hard the assets
  work.
- cash flow: the money that actually moved in and out of the business, as opposed to the profit the
  accounts report.
- factor investing: choosing shares by a measurable characteristic rather than by a view on the market.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
