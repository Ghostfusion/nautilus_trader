# Residual momentum: ranking shares by the part of their rise the market does not explain

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                      |
| ------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Shares of large American companies, bought in the strongest group and sold short in the weakest                                                                                                                                                                            |
| How often it trades       | Once a month, when the portfolio is rebuilt                                                                                                                                                                                                                                |
| What you need             | A spreadsheet and three years of monthly prices for many shares, plus the three factor numbers each month                                                                                                                                                                  |
| Where the rules come from | [QuantConnect strategy library, residual momentum](https://www.quantconnect.com/tutorials/strategy-library/residual-momentum) and the matching [Quantpedia entry](https://quantpedia.com/strategies/residual-momentum-factor)                                              |
| The underlying research   | Blitz, Huij and Martens, [Residual Momentum](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2319861) (2011), and the out-of-sample revisit by [Huij and Lansdorp](https://ssrn.com/abstract=2929306) (2017)                                                           |
| How well it held up       | Mixed: a long American sample back to 1926 and a later global replication both show a steadier reward than ordinary momentum, but the effect lives in a crowded long-and-short strategy whose profits have narrowed and whose size depends on choices the researcher makes |
| Also appears in           | nothing else in this collection                                                                                                                                                                                                                                            |

## The idea in one paragraph

Ordinary momentum buys the shares that have risen most and sells the shares that have fallen most.
The trouble is that the shares which rose most usually did so because the whole market rose, and
when the market turns those same shares fall back. This version first subtracts the part of each
share's move that the market as a whole, and a few broad styles, can explain. What is left over is
called the residual: the part of the month's move that is the share's own news rather than the
weather. The strategy ranks shares by how steadily and how strongly their residuals have been
positive, buys the leaders, sells the laggards, and repeats monthly.

## Why anyone believed it

A share moves for two reasons. One is common to the whole market: interest rates, investor mood, a
war, a recession. The other is specific to the company: a new product, a lost contract, a good
quarter. The first kind is easy to see and easy to copy, so it is not much of an edge. The second
kind spreads slowly. Analysts take weeks to revise their numbers, and ordinary investors notice the
news late, so a company's own good news keeps nudging its price up for a while. If you could strip
out the common part, the leftover should be the part that keeps drifting.

The counterparty is the investor who buys a share because the market is rising, without checking
whether the company itself is doing anything different. Ordinary momentum rewards that behaviour and
then punishes it when the market rolls over. By removing the common part first, this version tries to
keep only the company-specific drift and to avoid the market's reversals.

## An everyday comparison

Imagine a class of thirty students sitting the same exam. Some score high because the whole class is
strong and the paper was easy; that is the market. To judge a student properly, you compare their
mark with the mark a student of similar background would be expected to get. The difference, above
or below that expectation, is what the student brought themselves. A student who beats their own
expected mark term after term is the one to watch; a student who scores well only because the class
did well tells you little. This strategy watches the students who beat their own expectation, not
the ones on top of the raw table.

## The rules, step by step

1. Choose a large set of shares. The QuantConnect page starts from the 400 shares with the highest
   dollar trading volume and keeps the top 10 percent by market value. The Quantpedia page starts
   from all shares on the main American markets with a price above one dollar and keeps the top 10
   percent by market value. Both exclude a share priced below one dollar.
2. For each share, collect the last 36 months of monthly returns.
3. Each month, work out how the share's return has moved with three broad factors: the market, and
   the size and value styles described in the maths section. Fit this relationship on the past 36
   months.
4. For every one of those months, keep the leftover part of the return, the part the three factors
   did not explain. That leftover is the residual.
5. Take the residuals of the past 12 months, but skip the most recent month. The reason is that over
   days and weeks prices tend to bounce back, which works against the signal.
6. Add the twelve residuals together and divide the sum by their own standard deviation, a measure
   of how much they jump around. The result is the share's score.
7. Rank every share by its score. Buy the top 10 percent, in equal amounts, and sell short the
   bottom 10 percent, also in equal amounts.
8. Hold for one month, then rebuild from step 2. A share that falls out of the top or bottom group
   is closed and replaced.

Selling short means borrowing a share you do not own, selling it, and buying it back later; if the
price falls you keep the difference, and if it rises you lose.

## The maths, with every symbol named

For each share, one month's return is split into a part the broad factors explain and a part left
over:

```text
r_t = alpha + b1 * Mkt_t + b2 * SMB_t + b3 * HML_t + e_t
```

- `r_t` is the share's return in month `t`, as a decimal: 0.03 means 3 percent.
- `Mkt_t` is the market factor in month `t`: the return of the broad market minus the return on a
  safe short-term deposit.
- `SMB_t` is the size factor, "small minus big": the extra return that month of small companies over
  large ones.
- `HML_t` is the value factor, "high minus low": the extra return that month of cheap shares over
  expensive ones, where cheap means a low price relative to the company's book value.
- `alpha` is the average part of the share's return the three factors did not explain.
- `b1`, `b2` and `b3` are the share's sensitivities to the three factors, its betas.
- `e_t` is the residual: the part of the month's return left over after the explained part is
  removed.

The residual is what the company did that the weather did not. It can be positive or negative. To
turn a run of residuals into one number, add them and divide by how much they vary:

```text
score = (e_1 + e_2 + ... + e_12) / sigma_e
```

- `e_1` to `e_12` are the residuals of the past 12 months, skipping the most recent one.
- `sigma_e` is the standard deviation of those same residuals, a measure of their typical jump size.
- The sum is in the same units as the residuals, so the score has no unit. A larger score means
  larger and steadier positive residuals.

The division matters. One share whose residuals are all about +1 percent scores as highly as another
whose residuals swing between +5 and -3 percent, even though the second had the same total. The rule
prefers the calm, consistent one.

## A worked example

Suppose a line has been fitted through 36 months of one share's returns, and it produced these
leftover residuals for the last five months (a stand-in for the twelve). The percentages are
invented but of a realistic size.

| Month   | Residual e   |
| ------- | ------------ |
| Month 1 | +2.0 percent |
| Month 2 | -1.0 percent |
| Month 3 | +3.0 percent |
| Month 4 |  0.0 percent |
| Month 5 | +1.0 percent |

Add them: 2.0 - 1.0 + 3.0 + 0.0 + 1.0 = 5.0 percent. Their average is 1.0 percent. The differences
from the average are +1.0, -2.0, +2.0, -1.0 and 0.0; squared, those are 1.0, 4.0, 4.0, 1.0 and 0.0,
adding to 10.0. Dividing by five gives a variance of 2.0, so the standard deviation is the square
root of 2.0, which is 1.4142 percent.

```text
score = 5.0 / 1.4142 = 3.54
```

Now a second share with the same total but steadier residuals:

| Month   | Residual e   |
| ------- | ------------ |
| Month 1 | +1.0 percent |
| Month 2 | +2.0 percent |
| Month 3 |  0.0 percent |
| Month 4 | +1.0 percent |
| Month 5 | +1.0 percent |

The sum is again 5.0 percent and the average 1.0 percent, but the differences from the average are
0.0, +1.0, -1.0, 0.0 and 0.0, whose squares add to 2.0. The variance is 0.4 and the standard
deviation is the square root of 0.4, which is 0.6325 percent.

```text
score = 5.0 / 0.6325 = 7.91
```

Share two ranks first, even though the two shares earned the same total residual, because its
residuals are the steadier of the two.

Now the portfolio. Suppose the shares with the highest scores gained 2.5 percent next month and the
shares with the lowest scores lost 1.0 percent. The rule holds one dollar bought in the winners and
one dollar sold short in the losers. The long side gains 2.5 cents; the short side, sold at the
start and bought back 1.0 percent cheaper, gains another 1.0 cent. That is 3.5 cents on two dollars
of positions, or 1.75 percent before costs. If the whole book is replaced at the rebuild, trade
counts both sides and the traded fraction is 2.0:

```text
Cost = t * c = 2.0 * 0.0010 = 0.0020, that is 0.20 percent
Net return for the month = 1.75 - 0.20 = 1.55 percent
```

The cost figure uses ten basis points per side, which covers the gap between buying and selling
prices plus the fee to borrow the shares that are sold short. The worked example says nothing about
whether the strategy works; it only shows how to apply the rules and how the arithmetic behaves.

## What the research actually found

| Source                                                                        | What it measured                                                            | Result                                                                                                                                                                   |
| ----------------------------------------------------------------------------- | --------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Blitz, Huij and Martens, Residual Momentum (2011)                             | American shares, 1926-2009, monthly residual momentum versus total momentum | Residual momentum's risk-adjusted profit was about twice total momentum's, mainly because its return varied less; and it was less concentrated in the smallest companies |
| The same paper, on the recent period                                          | Total momentum and residual momentum, January 2000 to December 2009         | Total momentum lost 8.5 percent a year; residual momentum gained 4.7 percent a year over the same decade                                                                 |
| The same paper, on the business cycle                                         | 1930-2009, recession months                                                 | Total momentum lost 8.7 percent a year in recessions; residual momentum gained 5.6 percent a year in the same months                                                     |
| Quantpedia, summarising the same work                                         | Top-minus-bottom decile, 12-1 month residual momentum, 1926-2009            | 9.18 percent a year, volatility 15.27 percent, worst fall 59.74 percent, reward-to-risk 0.34, over roughly 200 shares                                                    |
| Huij and Lansdorp, Residual Momentum and Reversal Strategies Revisited (2017) | Re-tests the original findings on global share universes                    | The lower exposure to the broad factors, and the higher reward for the risk, held up out of sample after the original publication                                        |

The honest split is this. The original study and its replication agree that residual momentum is
steadier than ordinary momentum and earns more per unit of risk, and the reason is easy to state:
ordinary momentum carries hidden market and style exposure that turns against it in a downturn, and
residual momentum carries much less of it. But the reward-to-risk of 0.34 in the Quantpedia table is
modest, the worst fall in the same table is almost 60 percent, and a long-and-short strategy that
has been published for years is exactly the kind that gets crowded. The two sources also use
different rules, so the numbers are not directly comparable.

## How this project relates to it

This repository's harvest of the predictability literature,
[return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
covers momentum as a design problem rather than a forecast problem. It reports that the cross-section
of share returns is mostly real rather than an artefact of searching, so the interesting question is
not whether momentum exists but how crowded and how fragile a particular version has become.

The second related piece is
[overfitting, reproducibility and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md).
It reports that 207 published cross-sectional predictors mostly replicate, that the false-discovery
rate is under 10 percent, and that the hurdle a researcher should use is stricter than the textbook
one precisely because so many variants get tried. That is the danger for this strategy: it is one of
many ways to define a residual, and the best-looking window or factor list is easy to pick after the
fact.

The third link is
[portfolio construction and allocation](../../../strategies/books2/10_portfolio_and_allocation.md),
which shows that a factor model's measured performance is a property of how its test portfolios are
built, not only of the model. The residual depends on the factor model, so changing the model changes
the ranking.

## Where it goes wrong

- The factor list is a choice. Blitz, Huij and Martens used three factors. Using five factors, or a
  different definition of value, produces different residuals and a different ranking, and nothing in
  the rule says which is right.
- It is still a long-and-short share strategy. Removing market exposure reduces the crashes but does
  not remove the crowded-trade risk, and a sudden unwind still hurts both legs.
- Shorting has extra costs. The shares that fall to the bottom of the ranking are often hard or
  expensive to borrow, and the ten basis points used above can be optimistic for them.
- Dividing by variability can mislead. Because the score is the residual sum divided by its own
  standard deviation, a quiet share with small but very steady residuals can outrank a share with a
  genuine, larger piece of news.
- The estimation window is a choice. Three years of months is a convention, not a law, and the score
  changes if the window changes.
- Survivorship and data errors. A share that was removed from the market may be missing from a
  spreadsheet, which flatters any rule that buys past winners. This repository's overfitting brief
  records a published result that turned from a large gain into a loss because of a single bad data
  row.

## Try it yourself

This is a paper exercise; no prices are needed, because the residuals are given.

1. On a sheet, write five rows, one per month, and two columns of residuals, one for share A and one
   for share B. Use: A = +2, -1, +3, 0, +1; B = +1, +2, 0, +1, +1, all in percent.
2. Add each column and write the total beside it. Both columns total 5.
3. Subtract the column's average from every entry to get the differences, square each difference, add
   the squares, divide by five, and take the square root. That is the standard deviation.
4. Divide each total by its standard deviation to get the two scores. Share A scores about 3.54 and
   share B about 7.91.
5. Now change one number in column B, making its +2 into a -1, and redo step 3 and step 4.

What to notice: the two shares have the same total, but share B scores higher, because B's residuals
are steadier. Then, when you change one number in B, its total falls to 2 and its score drops to
about 2.5, below A's. A single large residual, good or bad, moves both the sum and the standard
deviation, which is why this kind of score is sensitive to outliers and to data errors.

## Where this came from

- [QuantConnect strategy library: residual momentum](https://www.quantconnect.com/tutorials/strategy-library/residual-momentum),
  the rules as implemented: the three-factor regression, the residual score, the monthly long and
  short deciles.
- [Quantpedia: residual momentum factor](https://quantpedia.com/strategies/residual-momentum-factor),
  the performance figures, the instrument count and the description of the universe.
- Blitz, Huij and Martens, [Residual Momentum](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2319861),
  the paper behind the rules. The abstract and the introduction were read from a freely available
  accepted-manuscript copy at Erasmus University; the published version is in the Journal of
  Empirical Finance. The paper is not in this repository's local corpus.
- Huij and Lansdorp, [Residual Momentum and Reversal Strategies Revisited](https://ssrn.com/abstract=2929306),
  the out-of-sample revisit, read as an abstract.
- [Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md)
  and [overfitting, reproducibility and research integrity](../../../strategies/books2/28_overfitting_and_research_integrity.md),
  this repository's own studies of how momentum is designed and how researchers deceive themselves.

## Words used in this tutorial

- beta: how much a share moves when the thing it is measured against moves by one unit.
- decile: one of ten equal groups, so the top decile is the best tenth.
- factor: a broad influence on many prices at once, such as the market or the value style.
- momentum: the tendency of something that has been rising to keep rising for a while.
- regression: fitting a line through data to describe how one number moves with others.
- residual: the part of a change that the things you measured cannot explain, the leftover.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- standard deviation: a measure of how much a set of numbers typically differs from its average.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
