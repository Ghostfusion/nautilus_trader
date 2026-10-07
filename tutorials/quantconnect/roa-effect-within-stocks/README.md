# The ROA effect: profit per unit of assets, compared within each size group

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                       |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of American companies, bought and sold short in two baskets formed separately inside each size group                                                                                                                                 |
| How often it trades       | Once a month, when both baskets are rebuilt                                                                                                                                                                                                 |
| What you need             | A spreadsheet and a source of company quarterly accounts                                                                                                                                                                                    |
| Where the rules come from | [QuantConnect strategy library, ROA effect within stocks](https://www.quantconnect.com/tutorials/strategy-library/roa-effect-within-stocks) and the [Quantpedia entry](https://quantpedia.com/strategies/roa-effect-within-stocks) it cites |
| The underlying research   | Chen and Zhang, [A Better Three-Factor Model That Explains More Anomalies](https://static1.squarespace.com/static/5e6033a4ea02d801f37e15bb/t/5f61583e88f43b7d5b7196b5/1600215105801/Chen_Zhang_JF.pdf)                                      |
| How well it held up       | Mixed: a long American sample from 1972 to 2006 and a profitability factor that reappears across several countries, on one side, and published tests that are before costs with sources disagreeing on how much to hold, on the other       |
| Also appears in           | [Earnings quality factor](../earnings-quality-factor/README.md) in this collection, its close relative                                                                                                                                      |

## The idea in one paragraph

Return on assets, shortened to ROA, is the profit a company makes in a period divided by the value of
everything it owns and uses to make that profit: buildings, machines, stock, cash, everything. It is
profit per unit of assets, the way rent per room measures a hotel. Two hotels can both make a million
a year while one has twice the rooms; the smaller one is working its rooms harder. This strategy
computes that ratio for every company, sorts them into two groups by size, and inside each group buys
the tenth of companies with the highest ROA and sells short the tenth with the lowest. It does this
every month. The bet is that the market pays more for companies that turn their assets into profit
efficiently, and less for those that do not, and that it keeps doing so.

## Why anyone believed it

The first thing to say is that a company producing a lot of profit per unit of assets ought to be
worth more, and in a perfectly efficient market it already would be, so there would be no extra return
left for anyone. The claim in the research is that the market does not price this fully, at least not
quickly. Profit tends to persist: a company that earns well on its assets this quarter usually earns
well next quarter, because the reasons for it, a strong brand, a good factory, a loyal customer base,
do not vanish overnight.

The counterparty is the investor who looks at the profit number alone and not at the assets behind
it, or who undervalues a company because its profit is unremarkable in absolute terms even though the
assets it took to earn that profit were small. The rule also compares companies only against others
of similar size, because a small company and a giant rarely have comparable ratios, and mixing them
would let size, rather than efficiency, decide the ranking.

## An everyday comparison

Two coffee shops take the same amount of money each day. The first rents a large corner site with
four machines and six staff. The second runs from a kiosk with one machine and two staff. The takings
are identical, but the kiosk is doing far more with far less, and its owner could open another kiosk
tomorrow, while the first shop could not. Anyone choosing between the two businesses would want to
know the takings, but would want to know them per unit of what was spent to get them. ROA is exactly
that per-unit figure, and the rule prefers the kiosks over the large sites, within each size of
business.

## The rules, step by step

1. Start with every share listed on the New York Stock Exchange, the American Stock Exchange and
   Nasdaq. Drop anything without company accounts, which removes most exchange-traded funds.
2. Keep only companies whose sales are greater than ten million dollars, so that very small firms with
   unstable accounts are left out of the comparison.
3. Keep only companies that have a reported share count and a positive price and profit, so that the
   size and ratio measures make sense.
4. Compute ROA for each company: the profit reported for one quarter divided by the value of the
   company's total assets at the start of that quarter, that is one quarter earlier. Using the assets
   from before the profit was earned avoids a mismatch between the two numbers.
5. Split the companies into two halves by market value, the share price multiplied by the number of
   shares. Call them the big group and the small group.
6. Inside each half, sort the companies by ROA and divide them into ten equal groups, the tenths.
7. Buy the best tenth of the big group and the best tenth of the small group, equally weighted, using
   half the money in total. Sell short the worst tenth of each group, equally weighted, using the
   other half.
8. Hold for one month, then rebuild from step 4. Repeat.

One disagreement to record. Quantpedia says the investor holds the best and worst three tenths of
each group, while the QuantConnect implementation holds only the best and worst tenth of each. This
tutorial describes the implementation, because that is the page the rules come from, and notes that
the published result depends on the size of the slice.

## The maths, with every symbol named

Return on assets, the number the whole rule is built on:

```text
ROA = Earnings_quarter / Total_assets_lagged
```

- `Earnings_quarter` is the profit reported for one quarter, before any extraordinary items are
  counted.
- `Total_assets_lagged` is the value of everything the company owns, taken from the accounts one
  quarter earlier, so that the profit is compared with the assets that existed while it was earned.
- The result is a decimal, so 0.08 means the company earned 8 percent of the value of its assets in a
  quarter.

The size of a company:

```text
Market_value = P * N
```

- `P` is the share price.
- `N` is the number of shares the company has issued.

The weights assigned to the chosen shares:

```text
w_best = 0.5 / (number of shares in the long basket)
w_worst = 0.5 / (number of shares in the short basket)
```

- `w_best` is the fraction of the account put into each of the best-ROA shares, so the whole long side
  adds up to one half.
- `w_worst` is the fraction sold short in each of the worst-ROA shares, so the whole short side also
  adds up to one half.

The return of the book over a month:

```text
R = 0.5 * R_long - 0.5 * R_short
```

- `R_long` is the equal-weighted return of the shares that were bought.
- `R_short` is the equal-weighted return of the shares that were sold short, so subtracting it is the
  same as subtracting their price move.

The cost of rebuilding every month:

```text
Cost = t * c
```

- `t` is the fraction of the account traded in the month, counting both the sale and the purchase of
  every position that changes.
- `c` is the cost per trade as a fraction of the amount traded, covering the gap between the buying
  and selling price plus commission. A realistic figure for American shares is 0.001, that is ten
  basis points, where one basis point is one hundredth of one percent.

## A worked example

Two groups of ten invented companies. With ten in each group, the best tenth and the worst tenth are
one company each. Earnings are quarterly and in millions of dollars; assets are the value one quarter
earlier, also in millions.

| Big company | Earnings | Assets | ROA    | Rank |
| ----------- | -------- | ------ | ------ | ---- |
| B1          | 40       | 500    | 8.00%  | 1    |
| B2          | 25       | 400    | 6.25%  | 2    |
| B3          | 60       | 1000   | 6.00%  | 3    |
| B4          | 30       | 600    | 5.00%  | 4    |
| B5          | 18       | 450    | 4.00%  | 5    |
| B6          | 45       | 1500   | 3.00%  | 6    |
| B7          | 20       | 800    | 2.50%  | 7    |
| B8          | 12       | 600    | 2.00%  | 8    |
| B9          | 10       | 1000   | 1.00%  | 9    |
| B10         | -5       | 500    | -1.00% | 10   |

| Small company | Earnings | Assets | ROA    | Rank |
| ------------- | -------- | ------ | ------ | ---- |
| S1            | 8        | 100    | 8.00%  | 1    |
| S2            | 6        | 90     | 6.67%  | 2    |
| S3            | 5        | 100    | 5.00%  | 3    |
| S4            | 3        | 80     | 3.75%  | 4    |
| S5            | 4        | 120    | 3.33%  | 5    |
| S6            | 2        | 100    | 2.00%  | 6    |
| S7            | 2        | 150    | 1.33%  | 7    |
| S8            | 1        | 100    | 1.00%  | 8    |
| S9            | 1        | 200    | 0.50%  | 9    |
| S10           | -2       | 80     | -2.50% | 10   |

Work one line: B1 earned 40 on assets of 500, and 40 divided by 500 is 0.08, or 8.00 percent. The
long basket is B1 and S1, one quarter of the money in each, and the short basket is B10 and S10, also
one quarter of the money in each.

Now six invented months. The long basket return is the average of B1 and S1 for that month, and the
short basket return is the average of B10 and S10. The gross figure is `0.5 * long - 0.5 * short`, and
the cost assumes the whole book is replaced each month, which is the worst case, so the traded
fraction is 2.0 and the cost is 0.20 percent.

| Month | Long basket | Short basket | Gross  | Cost  | Net    |
| ----- | ----------- | ------------ | ------ | ----- | ------ |
| 1     | +3.0%       | +1.0%        | +1.00% | 0.20% | +0.80% |
| 2     | +1.0%       | -2.0%        | +1.50% | 0.20% | +1.30% |
| 3     | -2.0%       | -4.0%        | +1.00% | 0.20% | +0.80% |
| 4     | +4.0%       | +3.0%        | +0.50% | 0.20% | +0.30% |
| 5     | +2.0%       | -1.0%        | +1.50% | 0.20% | +1.30% |
| 6     | -1.0%       | -3.0%        | +1.00% | 0.20% | +0.80% |

Work one row: in month 1 the long basket rose 3.0 percent and the short basket rose 1.0 percent, so
the gross is (0.5 times 3.0) minus (0.5 times 1.0), which is 1.5 minus 0.5, or 1.00 percent. After the
0.20 percent cost the net is 0.80 percent.

Chaining the six monthly net returns gives 1.008 times 1.013 times 1.008 times 1.003 times 1.013 times
1.008, which is 1.0541, or about 5.41 percent over six months. Two such half-years give 1.0541 times
itself, which is 1.1112, so about 11.1 percent a year after costs. The monthly cost is the number to
watch: at 0.20 percent a month the rule pays about 2.4 percent a year in costs, and if the whole book
does not change every month the real figure is lower. The example is invented and shows only how the
arithmetic works.

## What the research actually found

| Source                                                                   | What it measured                                                         | Result                                                                                                                                                                                                                                                                                  |
| ------------------------------------------------------------------------ | ------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Quantpedia, summarising Chen and Zhang                                   | American shares, 1972 to 2006, sorted by ROA inside two size halves      | 12.15 percent a year from a 0.96 percent monthly figure, volatility 13.36 percent, worst fall 47.43 percent, reward-to-risk 0.61                                                                                                                                                        |
| Chen and Zhang, A Better Three-Factor Model That Explains More Anomalies | The cross-section of American share returns                              | A model built from the market, an investment factor and a return-on-assets factor described returns better than the standard models and explained anomalies such as prior short-term returns, financial distress, share issuance, asset growth, earnings surprises and valuation ratios |
| Chen, Novy-Marx and Zhang, An Alternative Three-Factor Model             | The same construction, presented separately                              | The return-on-assets factor carried a return premium and improved on the standard models                                                                                                                                                                                                |
| Lu, Stambaugh and Yuan, Anomalies Abroad                                 | Nine well-known American patterns tested in five other developed markets | All nine, including the profitability-based ones, remained significant across Canada, France, Germany, Japan and the United Kingdom                                                                                                                                                     |
| Bouchaud, Stefano, Landier, Simon and Thesmar                            | The cause of the profitability premium                                   | The returns were abnormally high after adjusting for risk and were not prone to crashes, which the authors read as a behavioural rather than a risk explanation                                                                                                                         |

The pattern is real in the samples that were tested, and the profitability idea behind it has been
found in more than one country. Two cautions keep the grade below the best. The published returns are
computed before trading costs, and a monthly rebuild of two baskets is not cheap. And the sources
disagree about how large the slices should be, the implementation using the top and bottom tenth while
Quantpedia describes the top and bottom three tenths, which tells you the measured size of the premium
depends on choices like that.

## How this project relates to it

This repository does not implement ROA, but its study of portfolio construction is the right place to
test a factor like this one:
[strategies/books2/10_portfolio_and_allocation.md](../../../strategies/books2/10_portfolio_and_allocation.md).
That brief reports two findings that decide how much a result like this is worth. First,
publication-bias corrections shrink published cross-sectional mean returns by only 10 to 15 percent
and the false-discovery rate is under 10 percent, so the premium is unlikely to be pure luck. Second,
a factor model's ranking is not invariant to how the portfolio is built: the same three-factor model
won in six of eight daily constant-weight cells while five- and six-factor models won most buy-and-hold
cells. That is the same warning the disagreement about tenths and three-tenths gives, stated as a
measurement.

## Where it goes wrong

- The published numbers are before costs. Rebuilding two baskets every month at ten basis points a
  trade gives up roughly 2.4 percent a year, which is a fifth of the measured premium before anything
  goes wrong.
- The size of the slice decides the answer. Holding the top and bottom tenth is not the same trade as
  holding the top and bottom three tenths, and both appear in the sources.
- Small companies do a lot of the work. The effect is strongest where accounts are noisiest and
  trading is most expensive, and a backtest that ignores the companies that later disappeared looks
  better than the real experience.
- The lag matters. Using this quarter's assets with this quarter's earnings, rather than the previous
  quarter's assets, is a subtle form of using information before it existed, and it flatters the
  result.
- Profitability overlaps with other factors. A company that earns well on its assets also tends to
  look like a quality or value company, so some of the measured premium may belong to ideas already
  counted elsewhere.
- It may be compensation for risk. If unprofitable companies are riskier, their shares should offer
  higher returns, which would reverse the whole rule; the research argues against this for quality
  shares, but the argument is not settled.

## Try it yourself

You need a spreadsheet and a public source of quarterly company accounts, such as the investor
relations pages of any listed company.

1. Make a sheet with one row per company and these columns: Earnings this quarter, Total assets last
   quarter, Share price, Number of shares, and Sales this quarter.
2. Add a column for ROA: earnings divided by the lagged assets, and a column for market value: price
   times shares.
3. Keep only companies with sales above ten million, then sort by market value and cut the list in
   half.
4. Inside each half, sort by ROA and put the top tenth and the bottom tenth in two columns.
5. Write down the average next-quarter share return of the top tenth and of the bottom tenth of each
   half, and subtract the second from the first.

What to notice: the two halves give different answers, which is the reason the rule compares like with
like instead of pooling every company. Also notice how many companies have a ROA above 10 percent and
how many sit below zero, because the spread between the two ends is what the strategy is trying to
capture.

## Where this came from

- [QuantConnect strategy library: ROA effect within stocks](https://www.quantconnect.com/tutorials/strategy-library/roa-effect-within-stocks),
  the rules as implemented: the ten-million sales filter, the size split, the ROA deciles and the
  monthly rebuild. The page is titled "ROE effect within stocks" even though its method and its
  Quantpedia link both use return on assets; this tutorial follows the method.
- [Quantpedia: ROA effect within stocks](https://quantpedia.com/strategies/roa-effect-within-stocks),
  the performance figures, the sample and the underlying papers.
- Chen and Zhang, [A Better Three-Factor Model That Explains More Anomalies](https://static1.squarespace.com/static/5e6033a4ea02d801f37e15bb/t/5f61583e88f43b7d5b7196b5/1600215105801/Chen_Zhang_JF.pdf),
  the source paper.
- Chen, Novy-Marx and Zhang, [An Alternative Three-Factor Model](http://papers.ssrn.com/sol3/papers.cfm?abstract_id=1418117),
  the same construction presented separately.
- Lu, Stambaugh and Yuan, [Anomalies Abroad: Beyond Data Mining](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=3012923),
  which tests these patterns outside the United States.
- [strategies/books2/10_portfolio_and_allocation.md](../../../strategies/books2/10_portfolio_and_allocation.md),
  this repository's survey of how portfolio construction and publication bias change a measured
  factor.

## Words used in this tutorial

- decile: one of ten equal groups formed by sorting, so the top decile is the best tenth.
- factor: a shared characteristic used to sort assets into groups, such as size or profitability.
- market capitalisation: the total value of a company's shares, the share price times the number of
  shares.
- profitability factor: the idea that companies earning more from their assets earn higher returns.
- return on assets: quarterly profit divided by the value of the company's assets one quarter earlier.
- rebalance: adjusting a portfolio back to its intended holdings by buying and selling.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- transaction cost: everything paid to trade, including commission and the spread.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
