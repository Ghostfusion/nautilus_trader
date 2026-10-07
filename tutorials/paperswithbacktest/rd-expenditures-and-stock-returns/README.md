# Research and development spending as a signal: buying the companies that invest in the future

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                   |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of American companies, bought where research spending is large relative to the company's value and sold short where it is small                                                                                  |
| How often it trades       | Once a year, at the end of April, after the annual reports are out                                                                                                                                                      |
| What you need             | A spreadsheet, five years of research spending figures, and each company's market value                                                                                                                                 |
| Where the rules come from | [Quantpedia, R&D expenditures and stock returns](https://quantpedia.com/strategies/rd-expenditures-and-stock-returns/), the page the list's implementation is written from                                              |
| The underlying research   | Chan, Lakonishok and Sougiannis, [The Stock Market Valuation of Research and Development Expenditures](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=227564)                                                      |
| How well it held up       | Mixed: the effect is documented over decades but is small and concentrated in particular kinds of company, and one recent test of the same idea finds no statistically reliable spread                                  |
| Also appears in           | Nothing else in this collection describes research spending as a signal; [book-to-market value anomaly](../../quantconnect/book-to-market-value-anomaly/README.md) is the closest, and it shares the accounting problem |

## The idea in one paragraph

Companies that spend on research and development are building something that does not appear in
their accounts. The rule that governs how they record it makes the spending look like a cost, so it
reduces the profit they report, even though the benefit lasts for years. This strategy measures how
much a company has spent on research over the past five years relative to its market value, buys the
companies where that measure is highest, and sells short the companies where it is lowest. It
rebuilds the list once a year, in April, after the annual reports have been published. The bet is
that the market is slow to credit research spending, so the cheap-looking companies catch up.

## Why anyone believed it

The accounts treat two kinds of spending differently. Money spent on a factory is an investment: it
is recorded as an asset, and its cost is spread over the years the factory will be useful. Money
spent on research is an expense: the accounting rules require it to be subtracted from profit in the
year it is spent, even though the products it produces may earn money for a decade. A company that
invests heavily in research therefore looks less profitable today than a company with no research at
all, even when the two are otherwise alike.

The counterparty is the investor who reads reported profit and compares companies on it. That
investor underrates the research-heavy company and sells it, or simply never buys it, because the
research does not show up as an asset. A company with high research relative to its value is one the
market has doubts about, and those doubts are what the strategy buys. The paper behind the rules
argues the market fails to give enough credit for the spending, and that the failure is largest
where the doubt is greatest.

## An everyday comparison

Think of two shops on the same street. One repaints and refits every few years; the money it spends
is visible as a better shop, and a buyer looking at last year's takings would count the refit as a
cost. The other shop never spends anything and its takings look better on paper. The first shop is
cheaper than it should be, because the thing that makes it valuable is not in the numbers the buyer
is reading. This strategy buys the shops that have been quietly refitting.

## The rules, step by step

1. Start with all shares listed on the New York, American and Nasdaq exchanges whose price is above
   5 dollars and whose research spending is recorded. A company that reports no research spending at
   all cannot be ranked and is left out.
2. For each company, collect its research spending for each of the past five years, from the annual
   reports. One year's figure is a flow: it is how much was spent in that year.
3. Combine the five years into a single number with weights that count recent years more heavily.
   The implementation uses weights of 1, then 0.8, 0.6, 0.4 and 0.2, so the most recent year is
   worth five times the oldest.
4. Divide that combined figure by the company's market value, which is its share price times its
   number of shares. The result is the company's research intensity. Dividing by market value turns
   the research spending, which is a sum of money, into something comparable across companies of
   different sizes.
5. Rank all companies by research intensity and cut them into five equal groups.
6. Buy the top group, the companies with the most research relative to their value, and sell short
   the bottom group, the ones with the least. Weight every holding in a group equally.
7. Do this at the end of April and hold for a year, then repeat. April is chosen because annual
   reports for the previous year have been published by then, so the research figures are known.

## The maths, with every symbol named

The research intensity, one number per company:

```text
RD = (1.0 * R1 + 0.8 * R2 + 0.6 * R3 + 0.4 * R4 + 0.2 * R5) / MarketValue
```

- `R1` is research spending in the most recent year, `R2` the year before that, and so on to `R5`,
  five years back, all in the same currency.
- The numbers 1.0, 0.8, 0.6, 0.4 and 0.2 are the weights; they make recent spending count more.
- `MarketValue` is the company's share price multiplied by its number of shares.

A higher `RD` means the company spends more on research per unit of its market value. The strategy
buys the highest fifth and shorts the lowest fifth.

The account's return for the year is the long group's average return minus the short group's average
return:

```text
R = (R_1 + ... + R_n) / n over the long group  -  (R_1 + ... + R_m) / m over the short group
```

- `R_1` to `R_n` are the yearly returns of the long group's holdings, and `R_1` to `R_m` the short
  group's.
- Dividing by the count of holdings gives each one equal weight.

The cost, paid once a year rather than every month:

```text
Cost = t * c + b
```

- `t` is the traded fraction, 2.0 when the whole book is replaced at the annual rebuild.
- `c` is the cost of one side, about 0.001, that is 0.10 percent, for large American shares.
- `b` is the borrow fee on the shorted shares, charged as a yearly rate; for the low-research group,
  which tends to hold older, unfashionable companies, it is usually small, a fraction of a percent.

## A worked example

Six companies at one April rebuild, with five years of research spending behind them. The figures are
invented but of the size real companies report.

| Company | R5 (oldest) | R4   | R3   | R2   | R1 (latest) | Weighted research | Market value | Intensity | Side  |
| ------- | ----------- | ---- | ---- | ---- | ----------- | ----------------- | ------------ | --------- | ----- |
| A       | 10.0        | 10.0 | 10.0 | 10.0 | 10.0        | 30.0              | 200.0        | 0.150     | long  |
| B       | 5.0         | 5.0  | 5.0  | 5.0  | 5.0         | 15.0              | 200.0        | 0.075     | none  |
| C       | 20.0        | 20.0 | 20.0 | 20.0 | 20.0        | 60.0              | 400.0        | 0.150     | long  |
| D       | 2.0         | 2.0  | 2.0  | 2.0  | 2.0         | 6.0               | 300.0        | 0.020     | short |
| E       | 30.0        | 30.0 | 30.0 | 30.0 | 30.0        | 90.0              | 500.0        | 0.180     | long  |
| F       | 1.0         | 1.0  | 1.0  | 1.0  | 1.0         | 3.0               | 400.0        | 0.008     | short |

Worked through for company E, the weighted research is:

```text
Weighted = 1.0*30 + 0.8*30 + 0.6*30 + 0.4*30 + 0.2*30 = 30 + 24 + 18 + 12 + 6 = 90
Intensity = 90 / 500 = 0.18, that is 18 percent
```

The highest fifth of six companies is roughly the top one, and the lowest fifth the bottom one, so
the two long names shown are the highest intensities, E, A and C, and the two short names are the
lowest, D and F. Suppose the next year's returns are:

| Company | Side  | Return next year | Weight | Contribution |
| ------- | ----- | ---------------- | ------ | ------------ |
| E       | long  | +3.0 percent     | 1 / 3  | +1.000       |
| A       | long  | +4.0 percent     | 1 / 3  | +1.333       |
| C       | long  | +3.5 percent     | 1 / 3  | +1.167       |
| D       | short | -1.5 percent     | 1 / 2  | +0.750       |
| F       | short | -1.0 percent     | 1 / 2  | +0.500       |

The long leg's average is `(3.0 + 4.0 + 3.5) / 3 = 3.5 percent` and the short leg's average is
`(-1.5 - 1.0) / 2 = -1.25 percent`. Because the account is short, the short leg adds `+1.25 percent`.
Holding one dollar long and one dollar short, the gross return is:

```text
R = 3.5 percent + 1.25 percent = 4.75 percent
```

Now the annual costs:

```text
Trading cost = 4 * 0.001 = 0.004, that is 0.40 percent
Borrow cost  = 0.002, that is 0.20 percent for the year on the short leg
Net          = 4.75 - 0.40 - 0.20 = 4.15 percent
```

The gross return of 4.75 percent becomes about 4.15 percent after costs, a loss of a little over half
a percentage point to frictions. That is a small cost next to a monthly-rebalanced rule, which is
the appeal of an annual strategy, but the return itself is small too, and it comes with the risk of
holding a short position for a whole year.

## What the research actually found

| Source                                                      | What it measured                                                       | Result                                                                                                                                                                                |
| ----------------------------------------------------------- | ---------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Chan, Lakonishok and Sougiannis, the paper behind the rules | American shares, several decades to the late 1990s                     | Companies with high research relative to their market value showed signs of being mispriced, and among growth companies the research-heavy ones beat those with little or no research |
| Quantpedia, the page the rules come from                    | The strategy run over 2003 to 2020, a later sample than the paper      | A return of 4.67 percent a year, volatility of 8.2 percent, a reward-to-risk ratio of 0.51 and a worst fall of 41.09 percent, and it rates the anomaly's reliability as strong        |
| Sehgal, a more recent study of the same idea                | Large American companies, annual sorts by research relative to revenue | The high-minus-low spread averaged 3.73 percent a year, but with a statistical measure of 1.10, which is below the 1.96 usually needed to call a result reliable                      |
| The same later study                                        | A simpler long-only version holding the top 20 research companies      | 7.52 percent a year more than the market index after trading costs, keeping almost all of the gross return                                                                            |

The honest reading is that the effect is real but small, and that its shape matters. The original
paper found it strongest inside the group of growth companies and among firms whose research was
large relative to their value, not spread evenly across the market. The later study's long-short
spread of 3.73 percent a year comes with a statistical measure of 1.10, which cannot rule out chance,
while its long-only version did better after costs. The vendor's own recent run gives a
reward-to-risk ratio of 0.51, which is above the list's whole-catalogue median of 0.37 but far from a
strong result. The list's headline table of its 61 strongest replications does not include this
strategy.

## How this project relates to it

The completed tutorial on the value anomaly,
[book-to-market value anomaly](../../quantconnect/book-to-market-value-anomaly/README.md), faces the
same accounting problem from the other side: it notes that book value depends on the rules for
intangible assets, goodwill and write-downs, so two firms with the same business can report very
different numbers. Research spending is the clearest example of an intangible that the rules push
out of the accounts, and reading that tutorial explains why a company that invests in research looks
worse on paper than it is.

The repository's brief on overfitting and research integrity,
[strategies/books2/28_overfitting_and_research_integrity.md](../../../strategies/books2/28_overfitting_and_research_integrity.md),
is the second link. It is about how easily a real-looking result can come from choosing one
measurement among many, which is the warning this strategy most needs: research intensity can be
scaled by market value, by revenue or by assets, weighted in several ways, and sorted on any past
window, and the published effect is small enough that the choice matters.

## Where it goes wrong

- The effect is small and contingent. The original paper found it mainly inside growth companies and
  firms with high research relative to value, not across the whole market, and the recent long-short
  spread of 3.73 percent a year is not statistically reliable on its own.
- Accounting rules are not stable. Research spending is defined by the accounting standards, which
  change, and the same company can report a different figure after a rule change. The signal is only
  as good as the definition behind it.
- Company size and industry are tangled up with research. Research is concentrated in technology and
  health care, so a portfolio sorted on research is also a portfolio that bets on those industries,
  and part of the measured return is that bet rather than the signal.
- A short leg held for a year is exposed. Selling short for twelve months means paying the borrow fee
  for twelve months and carrying the risk that the company is bought by someone else at a premium, in
  which case the short loses sharply.
- Survivorship and look-ahead. Research figures are published with a delay, and a backtest that uses
  the figure for a year before it was published is reading the future. April is chosen to avoid part
  of this problem, not all of it.
- One number, one year. The portfolio is rebuilt once a year, so a single bad pick stays in the book
  for twelve months, and a strategy with a small average return is fragile to a few large losers.

## Try it yourself

You need a spreadsheet and the annual reports of a handful of large technology and health care
companies, which state their research spending every year.

1. Build one row per company, with five columns for research spending in each of the last five years
   and one column for the company's current market value.
2. Add a column for the weighted research figure: the latest year times 1.0, plus the year before
   times 0.8, and so on down to 0.2 for the oldest.
3. Add a column for research intensity: weighted research divided by market value.
4. Sort by intensity and mark the top fifth and the bottom fifth.
5. Collect each company's share price change over the next year and average it within each marked
   group.
6. Subtract the bottom group's average from the top group's average, and then subtract about 0.60
   percent for the round trip and the borrow fee.

What to notice: the companies that rank highest on research intensity are almost always young and in
a single industry, while the lowest are almost always old and in another. If your top group and
bottom group differ by industry that strongly, part of whatever gap you find is the industry's
return, not the research signal, and that is the ambiguity the studies have to work around.

## Where this came from

- [Quantpedia, R&D expenditures and stock returns](https://quantpedia.com/strategies/rd-expenditures-and-stock-returns/),
  the page the list's implementation is written from, and the source of the 4.67 percent, the 8.2
  percent volatility, the 0.51 ratio and the 41.09 percent fall.
- Chan, Lakonishok and Sougiannis, [The Stock Market Valuation of Research and Development Expenditures](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=227564),
  the source of the original finding and the accounting explanation.
- Sehgal, [R&D Alpha: Investment Intensity and Long-Term Stock Returns](https://ssrn.com/abstract=6002295),
  the source of the 3.73 percent spread, the 1.10 statistical measure and the 7.52 percent long-only
  result.
- The implementation the list carries:
  [rd-expenditures-and-stock-returns.py](https://github.com/paperswithbacktest/awesome-systematic-trading/blob/main/static/strategies/rd-expenditures-and-stock-returns.py),
  which states the five-year weights, the quintile sort and the April rebuild.
- [book-to-market value anomaly](../../quantconnect/book-to-market-value-anomaly/README.md), a
  completed tutorial facing the same intangible-asset accounting problem.
- [Overfitting and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md),
  this repository's brief on how a result can come from choosing one measurement among many.

## Words used in this tutorial

- expense: a cost subtracted from profit in the year it is incurred, as opposed to a cost recorded as
  an asset and spread over several years.
- investment: spending that is recorded as an asset and written down over the years it is useful.
- intangible asset: something valuable a company owns that is not physical, such as a brand, a
  patent or the skill of its workforce.
- market value: a company's share price multiplied by its number of shares.
- long: owning something, so that a rise in its price makes money.
- short selling: borrowing something you do not own, selling it now, and buying it back later, which
  makes money if the price falls.
- quintile: one fifth of a sorted list.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
