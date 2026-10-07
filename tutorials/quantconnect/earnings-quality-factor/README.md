# Earnings quality: buying companies whose reported profit is backed by cash

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                    |
| ------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of American companies that are not banks or insurers, bought and sold short in two large baskets                                                                                                                                  |
| How often it trades       | Once a year, when both baskets are rebuilt                                                                                                                                                                                               |
| What you need             | A spreadsheet and a source of company annual accounts                                                                                                                                                                                    |
| Where the rules come from | [QuantConnect strategy library, earnings quality factor](https://www.quantconnect.com/tutorials/strategy-library/earnings-quality-factor) and the [Quantpedia entry](https://quantpedia.com/strategies/earnings-quality-factor) it cites |
| The underlying research   | Kozlov and Petajisto, [Global Return Premiums on Earnings Quality, Value, and Size](http://papers.ssrn.com/sol3/papers.cfm?abstract_id=2179247)                                                                                          |
| How well it held up       | Mixed: a long global sample and an independent quality-factor literature on one side, and on the other evidence that only the profitability parts of the score carry a premium while the debt and stability parts do not                 |
| Also appears in           | [ROA effect within stocks](../roa-effect-within-stocks/README.md) in this collection, its close relative                                                                                                                                 |

## The idea in one paragraph

Every company reports a profit each year, but the reported number can be solid or flimsy. A solid
profit is one the company actually collected in cash: customers paid, bills were paid, and the money
is in the bank. A flimsy profit is one that exists mostly on paper, because sales were recorded before
the cash arrived or because costs were spread over many years so that this year looks better than it
is. This strategy scores every company on four measures of how solid its profit is, buys the best
thirty percent, and sells short the worst thirty percent. It rebuilds the list once a year. The bet is
that investors fixate on the headline profit number and ignore how it was made, so the solid companies
are underpriced and the flimsy ones are overpriced.

## Why anyone believed it

Company accounts are long and dull, and most people who buy shares read one number from them: the
profit, usually called earnings. The profit can be moved around by perfectly legal choices, such as
when to count a sale, how fast to write down a machine, or how much to set aside for a future cost.
The people who run a company know this and have a reason to report a smooth, rising profit, because
their pay and their share price often depend on it.

The counterparty is therefore the investor who reads the headline profit and stops there. That
investor buys the flimsy company at a price that assumes the profit is real, and avoids or sells the
solid company whose profit looks smaller because it was not dressed up. If enough investors do this,
the flimsy company is too expensive and the solid one is too cheap, and a strategy that buys the
solid and sells the flimsy collects the difference as the truth comes out in later years.

## An everyday comparison

Imagine a cafe owner deciding whether a neighbouring shop is doing well. One way is to read the sign
on the door saying "record year". A better way is to look at the till and at the slips from customers
who said they would pay next week. A shop with a busy till and few slips is really earning money; a
shop whose "record year" is mostly slips, or whose fridges were bought and then quietly spread over
ten years so this year looks bright, is not. Both can print the same profit figure, but only one has
the cash.

## The rules, step by step

1. Take the shares of every company listed on the New York Stock Exchange, the American Stock
   Exchange and Nasdaq, and set aside the banks and insurers, whose accounts are built differently.
   That leaves the firms whose profit is mostly made by selling goods and services.
2. Once a year, gather each company's last two annual reports. From them you need current assets,
   cash, current liabilities, the part of the debt due within a year, income taxes payable,
   depreciation, total assets, the cash the business generated, the reported profit, shareholders'
   equity and total debt.
3. Compute four measures for every company. In plain words: how much of the reported profit is not
   cash (accruals, lower is better), how much cash the business generates relative to its size (cash
   flow, higher is better), how much profit it makes per unit of the owners' money (return on equity,
   higher is better), and how much of its funding is borrowed (debt, lower is better). The exact
   formulas are in the next section.
4. Rank all the companies on each measure, worst to best. Give the worst a score of 0 and the best a
   score of 100, with everyone else spread in between in proportion to their position. For accruals
   and debt, the lowest number is the best and gets 100; for cash flow and return on equity, the
   highest is the best and gets 100.
5. Add the four scores for each company. The total runs from 0 (bad on all four) to 400 (good on all
   four).
6. Buy the shares of the top thirty percent of scores, in equal amounts. Sell short the shares of the
   bottom thirty percent, also in equal amounts.
7. Hold for a year, then rebuild. The list is formed at the end of June, so that the accounts from the
   previous December are already public and there is no use of information before it existed.

The QuantConnect implementation gives sixty percent of the money to the long basket and sixty percent
to the short basket rather than fifty and fifty, which puts more of the account at work and more of it
at risk. This tutorial uses half and half for the worked example, because it is easier to follow and
closer to the description in Quantpedia.

## The maths, with every symbol named

The first measure is accruals, the part of reported profit that did not arrive as cash:

```text
A = ((CA_now - CA_before) - (Cash_now - Cash_before)) - ((CL_now - CL_before) - (STD_now - STD_before) - (ITP_now - ITP_before)) - Dep
Accruals_ratio = A / ((Total_assets_now + Total_assets_before) / 2)
```

- `A` is the accruals of the company for the year, in the same units as the accounts.
- `CA_now` and `CA_before` are current assets at the end of this year and the year before.
- `Cash_now` and `Cash_before` are cash and near-cash at the two dates.
- `CL_now` and `CL_before` are current liabilities, the bills due within a year.
- `STD_now` and `STD_before` are the part of the debt due within a year.
- `ITP_now` and `ITP_before` are income taxes payable.
- `Dep` is the year's depreciation and amortisation charge, the cost the accounts record for wear on
  buildings, machines and patents.
- `Accruals_ratio` divides `A` by the average of total assets across the two years, so that companies
  of different sizes can be compared.

A small positive ratio is good and a large one is a warning; a negative number means the company
collected more cash than it reported as profit, which the rule treats as the best case.

The second measure, cash flow to assets:

```text
CFA = Operating_cash_flow / ((Total_assets_now + Total_assets_before) / 2)
```

`Operating_cash_flow` is the cash the business generated from its ordinary operations. The page's own
worked code instead divides operating cash flow by `EPS * average_shares`, which is the profit rather
than the assets, while its prose and Quantpedia both say assets. This tutorial uses assets, as the
prose does, and flags the disagreement rather than hiding it. Higher is better.

The third measure, return on equity, and the fourth, debt to assets:

```text
ROE = Net_income / Shareholders_equity
DA  = Total_debt / Total_assets
```

`Net_income` is the reported profit, and `Shareholders_equity` is what is left of the assets after all
debts, often called book value; higher ROE is better. `Total_debt` is all borrowing, short and long
term; lower DA is better, because a company carrying less debt depends less on lenders and on interest
rates staying friendly.

The score for one company is the sum of four percentile ranks, and the return of the strategy in a
year, with half the money in the long basket and half in the short:

```text
Score = r_accruals + r_cashflow + r_roe + r_debt
R = 0.5 * R_long - 0.5 * R_short
```

- `r_accruals`, `r_cashflow`, `r_roe` and `r_debt` are each between 0 and 100, with 100 meaning best.
- `R_long` is the return of the equal-weighted basket of top-scoring shares.
- `R_short` is the return of the equal-weighted basket of bottom-scoring shares, so subtracting it is
  the same as subtracting the price move of the shares that were sold short.

Finally the cost of rebuilding, once a year:

```text
Cost = t * c
```

- `t` is the fraction of the account traded in the year, counting both the sale and the purchase of
  every position that changes.
- `c` is the cost of one trade as a fraction of the amount traded, covering the gap between the buying
  and selling price plus commission. A realistic figure for large American shares is 0.001, that is
  ten basis points, where one basis point is one hundredth of one percent.

## A worked example

Ten companies, with the four raw measures. The numbers are invented but of a size that real accounts
produce.

| Company | Accruals | Cash flow / assets | ROE  | Debt / assets |
| ------- | -------- | ------------------ | ---- | ------------- |
| A       | +0.01    | 0.11               | 0.20 | 0.30          |
| B       | +0.05    | 0.07               | 0.15 | 0.55          |
| C       | -0.02    | 0.14               | 0.25 | 0.20          |
| D       | +0.03    | 0.09               | 0.18 | 0.45          |
| E       | +0.08    | 0.05               | 0.10 | 0.65          |
| F       | 0.00     | 0.12               | 0.22 | 0.35          |
| G       | +0.06    | 0.06               | 0.12 | 0.60          |
| H       | +0.02    | 0.10               | 0.16 | 0.40          |
| I       | +0.09    | 0.04               | 0.08 | 0.70          |
| J       | -0.01    | 0.13               | 0.24 | 0.25          |

Sorting each column and giving the best value 100 and the worst 0, with the rest spread evenly (each
step is 100 divided by 9, that is 11.1 points), gives these scores:

| Company | Accruals score | Cash flow score | ROE score | Debt score | Composite (0-400) |
| ------- | -------------- | --------------- | --------- | ---------- | ----------------- |
| C       | 100            | 100             | 100       | 100        | 400               |
| J       | 88.9           | 88.9            | 88.9      | 88.9       | 355.6             |
| F       | 77.8           | 77.8            | 77.8      | 66.7       | 300.1             |
| A       | 66.7           | 66.7            | 66.7      | 77.8       | 277.9             |
| H       | 55.6           | 55.6            | 44.4      | 55.6       | 211.2             |
| D       | 44.4           | 44.4            | 55.6      | 44.4       | 188.8             |
| B       | 33.3           | 33.3            | 33.3      | 33.3       | 133.2             |
| G       | 22.2           | 22.2            | 22.2      | 22.2       | 88.8              |
| E       | 11.1           | 11.1            | 11.1      | 11.1       | 44.4              |
| I       | 0              | 0               | 0         | 0          | 0                 |

Thirty percent of ten companies is three, so the long basket is C, J and F, and the short basket is I,
E and G. One company, A, sits just outside the long basket despite scoring well, which shows how much
the cutoff matters when the list is short.

Now five years of returns for those baskets. The long basket return is the equal-weighted average of
its three shares, the short basket return is the same for the three sold-short shares, and the gross
figure is `0.5 * long - 0.5 * short`. The cost assumes that about seventy percent of each basket
changes each year, so the traded fraction is 0.7 of the long side plus 0.7 of the short side, that is
1.4 of the account, and at ten basis points the cost is 0.14 percent a year.

| Year | Long basket | Short basket | Gross long-short | Cost  | Net    |
| ---- | ----------- | ------------ | ---------------- | ----- | ------ |
| 1    | +12.0%      | +2.0%        | +5.00%           | 0.14% | +4.86% |
| 2    | +8.0%       | -6.0%        | +7.00%           | 0.14% | +6.86% |
| 3    | -4.0%       | -10.0%       | +3.00%           | 0.14% | +2.86% |
| 4    | +15.0%      | +9.0%        | +3.00%           | 0.14% | +2.86% |
| 5    | +6.0%       | -2.0%        | +4.00%           | 0.14% | +3.86% |

Cumulative, the five net returns chain to 1.0486 times 1.0686 times 1.0286 times 1.0286 times 1.0386,
which is 1.2313, so about 23.1 percent over five years, or roughly 4.2 percent a year after costs. In
year 4 the long basket rose 15 percent and still the strategy made only 3 percent, because the
shorted shares rose almost as much; a long-short strategy only profits from the gap between the two
baskets, not from the market rising. This example shows the arithmetic, nothing more. It cannot say
whether the strategy will pay in future.

## What the research actually found

| Source                                                                            | What it measured                                                                                      | Result                                                                                                                                                                                                             |
| --------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Quantpedia, summarising Kozlov and Petajisto                                      | The composite quality score, long top 30 percent against short bottom 30 percent, United States       | 7.95 percent a year from a 0.6399 percent monthly figure, volatility 5.91 percent, worst fall 18.67 percent, reward-to-risk 0.67, over July 1988 to June 2012                                                      |
| Kozlov and Petajisto, Global Return Premiums on Earnings Quality, Value, and Size | Seven developed markets, July 1988 to June 2012                                                       | The long-short earnings-quality portfolio had a higher reward-to-risk than the market or than value and small-stock portfolios, held in both the full sample and the years after 2005, and moved opposite to value |
| Asness, Frazzini and Pedersen, Quality Minus Junk                                 | Quality, defined as safe, profitable, growing and well-managed, in the United States and 24 countries | High-quality shares earned higher risk-adjusted returns than low-quality shares, and the price paid for quality varied over time                                                                                   |
| Hsu, Kalesnik and Kose, What Is Quality?                                          | A wide set of quality definitions                                                                     | Profitability, accounting quality, payouts and investment carried a return premium, while capital structure and earnings stability showed little evidence of one                                                   |
| Blitz, Baltussen and van Vliet                                                    | Splitting factors into their long and short legs                                                      | Most of the premium came from the long leg; the short leg added less and was largely contained in the long                                                                                                         |
| Bouchaud, Stefano, Landier, Simon and Thesmar                                     | The cause of the quality premium                                                                      | Quality returns were abnormally high after adjusting for risk and were not prone to crashes, which supports a behavioural explanation over a risk explanation                                                      |

Read together, the record shows a real premium attached to profitable, cash-generating companies,
confirmed in more than one market and period. The weaker part is the score itself. Hsu, Kalesnik and
Kose find that the profitability and investment ideas carry the premium, while the debt and earnings
stability ideas do not, so two of the four components here may be adding little. A second caution is
that most published numbers are before costs, and a yearly rebuild of two baskets of several hundred
shares is not free.

## How this project relates to it

This repository contains no earnings-quality or accruals implementation; the closest thing is its
survey of cross-sectional predictors in
[strategies/books2/08_predictability_and_trading_strategies.md](../../../strategies/books2/08_predictability_and_trading_strategies.md).
That brief reports that the cross-sectional factor zoo is probably mostly genuine, with a
false-discovery bound of 8.5 to 25 percent, but that value-weighting the same signals and adjusting
for known factors raises the bound to 41.7 percent. That number is the honest backdrop for a rule
like this one, which is built on rankings that look strong when every share counts equally and weaker
when large companies count for more. The brief also reports that signals decay as more money runs
them, which is what would happen to a published quality score.

## Where it goes wrong

- The score mixes strong and weak ideas. Profitability and cash generation carry a premium in the
  literature; low debt and stable earnings show little evidence of one, so the debt component may be
  adding noise rather than signal.
- The short basket is the hard half. Selling shares short means borrowing them, paying a fee, and
  being exposed to a loss that has no ceiling, and the published evidence says the short leg of a
  quality factor adds less than the long leg.
- Accounting definitions differ. Accruals can be computed in several ways, cash flow can be measured
  over a year or a quarter, and a company can change its own accounting choices. A reader who follows
  a different recipe gets a different ranking.
- It is slow and concentrated in time. Once a year, with a June formation, means one bad ranking
  lasts twelve months, and a single fraud that year can dominate the short basket.
- Value-weighting cuts the measured edge. When large companies count for more, the apparent premium
  shrinks sharply, which suggests much of it lived in small, illiquid shares where trading costs are
  highest.

## Try it yourself

You need only a spreadsheet and a public source of company accounts, such as the annual report pages
of any large listed company.

1. Make a sheet with one row per company and these columns: Current assets, Cash, Current
   liabilities, Short-term debt, Income taxes payable, Depreciation, Total assets last year, Total
   assets this year, Operating cash flow, Net income, Shareholders' equity, Total debt.
2. Add a column for the accruals ratio using the formula above, and three more for cash flow to
   assets, return on equity and debt to assets.
3. Add four rank columns, each giving every company a score from 0 to 100 on one measure, remembering
   that lower accruals and lower debt are better.
4. Add a total column that sums the four scores, then sort by it.
5. Compute the average next-year share return of the top three and of the bottom three, and subtract
   the second from the first.

What to notice: the same company can score well on profit and badly on cash, and those two measures
alone will disagree on several rows. That disagreement is the whole point of using four measures
instead of one, and it is also a warning that the choice of measures decides the answer.

## Where this came from

- [QuantConnect strategy library: earnings quality factor](https://www.quantconnect.com/tutorials/strategy-library/earnings-quality-factor),
  the rules as implemented: four measures, a summed rank score, long the top thirty percent and short
  the bottom thirty percent, rebuilt yearly.
- [Quantpedia: earnings quality factor](https://quantpedia.com/strategies/earnings-quality-factor),
  the performance figures, the sample, the instrument count and the underlying papers.
- Kozlov and Petajisto, [Global Return Premiums on Earnings Quality, Value, and Size](http://papers.ssrn.com/sol3/papers.cfm?abstract_id=2179247),
  the source paper and its global test.
- Asness, Frazzini and Pedersen, [Quality Minus Junk](http://papers.ssrn.com/sol3/papers.cfm?abstract_id=2312432),
  the independent quality-factor study.
- Hsu, Kalesnik and Kose, [What Is Quality?](https://www.tandfonline.com/doi/full/10.1080/0015198X.2019.1567194),
  which separates the quality ideas that carry a premium from those that do not.
- [strategies/books2/08_predictability_and_trading_strategies.md](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  this repository's survey of the cross-sectional evidence and its decay.

## Words used in this tutorial

- accruals: the part of reported profit that is not cash, such as sales recorded before payment
  arrives.
- earnings: the reported profit of a company over a period.
- quality factor: the idea that companies with solid, profitable, safe businesses earn higher returns.
- rebalance: adjusting a portfolio back to its intended holdings by buying and selling.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
