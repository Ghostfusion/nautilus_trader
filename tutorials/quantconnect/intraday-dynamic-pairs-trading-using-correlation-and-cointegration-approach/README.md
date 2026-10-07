# Pairs trading: buying the one that fell behind and selling short the one that ran ahead

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                     |
| ------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Two shares from the same industry at the same time, one bought and one sold short, so the bet is on their prices relative to each other                                                                                                                                                                   |
| How often it trades       | Many times a day; this version uses ten-minute price bars and re-opens whenever the gap is stretched                                                                                                                                                                                                      |
| What you need             | A spreadsheet and a long file of repeated intraday prices for a group of similar shares                                                                                                                                                                                                                   |
| Where the rules come from | [QuantConnect strategy library, intraday dynamic pairs trading](https://www.quantconnect.com/tutorials/strategy-library/intraday-dynamic-pairs-trading-using-correlation-and-cointegration-approach)                                                                                                      |
| The underlying research   | George J. Miao, [High Frequency and Dynamic Pairs Trading Based on Statistical Arbitrage](https://ccsenet.org/journal/index.php/ijef/article/view/33007), and the classic distance study of Gatev, Goetzmann and Rouwenhorst, [Pairs Trading](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=141615) |
| How well it held up       | Mixed: the broad pairs-trading family has decades of replication showing a modest net reward, but this specific intraday version rests on a single 2012 to 2013 sample of American bank shares, and independent work finds that the relationship it relies on does not persist                            |
| Also appears in           | [Dispersion and relative value](../../project/dispersion-and-relative-value/README.md) in the project tutorials, which trades the same gap between two similar baskets over months rather than minutes                                                                                                    |

## The idea in one paragraph

Two companies that do almost the same thing should have shares that move together. If one of them
jumps or sags while the other stays put, that gap is often temporary: a seller needed cash, an order
was too large, or the news mattered to one and not the other. This strategy watches many such pairs and,
when the gap stretches unusually far, buys the cheaper one and sells short the dearer one, with the
same money on each side. It does not care whether the market rises or falls, because one side gains
whatever the other loses, and it is paid when the gap closes.

## Why anyone believed it

Two shares that are close substitutes should be worth about the same relative to each other. In
practice they drift apart for reasons unrelated to value: a fund facing withdrawals sells whatever is
easiest to sell, and an index rebuild forces money out of one company and into its replacement.

The counterparty is often trading for a reason other than the outlook: they need cash, they must match
a benchmark, or they are unwinding a position that has grown too large. If those pressures reverse, the
gap closes, and the belief is that it was an accident of supply, not a permanent change in value.

## An everyday comparison

Picture a market with two stalls selling the same crate of apples. Most mornings they charge about the
same. One day one stall takes delivery of far too many crates and cuts its price by a fifth to clear
them, so the gap is clearly an accident of supply. If you could buy cheaply there and at the same
moment sell crates at the full price to the other stall, you would keep the difference once the cheap
stall sold out and its price came back. You would not care whether apple prices in general rose or
fell, because you hold one crate and owe one crate, and the two cancel.

## The rules, step by step

1. Choose a group of companies in the same line of business, because firms that do similar things are
   the most likely to be close substitutes. The library page uses twenty American bank shares.
2. List every possible pairing of two companies from the group. With twenty names there are
   `20 * 19 / 2 = 190` pairs.
3. Over a training window of about three months of ten-minute prices, compute how closely each pair's
   prices move together, the correlation. Keep only the pairs whose correlation is at least 0.90.
4. For each surviving pair, fit a straight line so that one share's price is explained by the other's,
   and look at the leftovers, the residuals. Test whether the leftovers keep returning to an average
   or wander off like a random walk. Keep only the pairs whose test returns a p-value at or below
   0.05, meaning the leftovers behave as if pulled back to a level.
5. Rank the survivors by how strongly they pass the test, best first, and trade only the top few, ten
   in the library page.
6. Every ten minutes, for each selected pair, work out the current gap between the two prices after
   allowing for their usual relationship, and compare it with the gap's own average and typical swing.
7. If the gap is more than 2.33 typical swings above its average, the first share is expensive
   relative to the second: sell the first short and buy the second. If it is more than 2.33 swings
   below, do the reverse. Put the same amount of money on each side.
8. Close the position when the gap comes back to within 0.5 of a swing of its average, and give up and
   close if it instead reaches 4 swings, the stop-loss.
9. Reselect the pairs every three months on a rolling window, dropping the ones that no longer behave
   together.

## The maths, with every symbol named

The first filter is the correlation between the two prices over the window:

```text
rho = sum( (A_i - A_bar) * (B_i - B_bar) ) / sqrt( sum( (A_i - A_bar)^2 ) * sum( (B_i - B_bar)^2 ) )
```

- `rho` is the correlation, a number between -1 and +1. Values near +1 mean the two prices move
  together almost exactly.
- `A_i` and `B_i` are the two prices at the same moment `i`.
- `A_bar` and `B_bar` are the averages over the window, and `sum(...)` adds up every observation in it.

The second filter is a test that the two prices are tied together, which is called cointegration. The
relationship is written as a straight line:

```text
A_t = alpha + beta * B_t + e_t
```

- `A_t` and `B_t` are the two prices at time `t`.
- `alpha` is the intercept of the fitted line.
- `beta` is the slope: how many units of `A` go with one unit of `B`.
- `e_t` is the leftover, the residual, the part of `A` that the relationship with `B` does not explain.

The residual is what the strategy trades. For each pair, the fit gives `alpha` and `beta`, and the
current gap is:

```text
gap_t = A_t - alpha - beta * B_t
```

The gap is then turned into a count of typical swings, a standard score:

```text
z_t = ( gap_t - Gap_mean ) / Gap_sd
```

- `z_t` is how many typical swings the gap sits from its average.
- `Gap_mean` is the average of the gaps over the training window.
- `Gap_sd` is the standard deviation of the gaps, the size of a typical swing.

The rule "act when `z_t` is above 2.33 or below -2.33" says: act only when the gap is unusually wide by
the standards of its own recent history. When the two legs hold the same amount of money `M`, the gross
profit of a long position in `B` against a short position in `A` is:

```text
profit = M * ( R_B - R_A )
```

- `profit` is the money gained or lost on the round trip.
- `M` is the money placed on each leg.
- `R_B` and `R_A` are the returns of the bought and the short-sold share over the holding period.

The market's move sits inside both returns, so it cancels, and the profit depends only on the difference.

## A worked example

Eight successive ten-minute observations of two bank shares, A and B. The regression pairs A with B.

| Period | Price A | Price B |
| ------ | ------- | ------- |
| 1      | 30.00   | 20.00   |
| 2      | 30.30   | 20.12   |
| 3      | 30.10   | 20.00   |
| 4      | 30.50   | 20.20   |
| 5      | 30.75   | 20.30   |
| 6      | 30.40   | 20.15   |
| 7      | 30.90   | 20.35   |
| 8      | 31.60   | 20.40   |

Fitting the line by least squares gives `alpha = -33.75` and `beta = 3.19`. The leftovers are then:

| Period | Price A | Expected A = -33.75 + 3.19 * B | Residual |
| ------ | ------- | ------------------------------ | -------- |
| 1      | 30.00   | 29.97                          | +0.03    |
| 2      | 30.30   | 30.36                          | -0.06    |
| 3      | 30.10   | 29.97                          | +0.13    |
| 4      | 30.50   | 30.61                          | -0.11    |
| 5      | 30.75   | 30.93                          | -0.18    |
| 6      | 30.40   | 30.45                          | -0.05    |
| 7      | 30.90   | 31.09                          | -0.19    |
| 8      | 31.60   | 31.25                          | +0.35    |

The average residual is 0.00, which the fit guarantees, and the size of a typical swing is
`Gap_sd = 0.168`. The three thresholds follow from that one number:

```text
open when |z| > 2.33:  |gap| > 2.33 * 0.168 = 0.392
close at |z| = 0.5:    |gap| = 0.5 * 0.168  = 0.084
stop at  |z| = 4.0:    |gap| = 4.0 * 0.168  = 0.672
```

Now suppose at period 9 share A rises to 31.66 and share B stays at 20.40. The gap is
`31.66 - (-33.75 + 3.19 * 20.40) = +0.41`, more than 0.392, so the rule acts. A positive gap means A is
expensive relative to B, so the book sells A short and buys B, with 50,000 on each leg:

```text
buy B at 20.40: 50,000 / 20.40 = 2,450 shares
short A at 31.66: 50,000 / 31.66 = 1,579 shares
```

At period 10 the gap closes to about +0.05, which is within 0.084 of the average, so the book closes.
Suppose A is then at 31.36 and B at 20.42.

| Leg     | Entry | Exit  | Move  | Shares | Profit  |
| ------- | ----- | ----- | ----- | ------ | ------- |
| Buy B   | 20.40 | 20.42 | +0.02 | 2,450  | +49.00  |
| Short A | 31.66 | 31.36 | +0.30 | 1,579  | +473.70 |
| Total   |       |       |       |        | +522.70 |

Now the costs. There are four crossings, buying B, selling short A, then closing each leg. At five
basis points, that is 0.0005, of the amount traded:

```text
turnover = 2,450 * 20.40 + 1,579 * 31.66 + 2,450 * 20.42 + 1,579 * 31.36
         = 49,980.00 + 49,991.14 + 50,029.00 + 49,517.44 = 199,517.58
cost     = 199,517.58 * 0.0005 = 99.76
net      = 522.70 - 99.76 = 422.94, which is 0.42 percent of the 100,000 account
```

Two things to notice. Nearly all the profit came from the short leg falling, so the trade was in
practice a bet that A would drop. And the four crossings cost about 100 on a gross profit of about
523, a fifth of the prize, before any borrow fee.

## What the research actually found

The evidence divides into the broad pairs-trading family, studied for decades, and this intraday version.

| Source                                                   | What it measured                                                                               | Result                                                                                                                                                                                                              |
| -------------------------------------------------------- | ---------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Miao, the paper behind the library page                  | Bank shares, ten-minute data, a three-month rolling window, out-of-sample 2012 to 2013         | A cumulative return up to 56.58 percent, beating the S&P 500 index by 34.35 percent over twelve months, with a monthly Sharpe ratio of 2.67 and an annual figure of 9.25                                            |
| The library page                                         | The same rules applied to American bank shares with ten-minute data, September 2013            | A compounded annual return up to 26.924 percent and a Sharpe ratio of 3.011, profitable especially when the market was falling                                                                                      |
| Quantpedia, summarising Gatev, Goetzmann and Rouwenhorst | The classic distance version, American shares, daily data, 1962 to 2002                        | 11.16 percent a year after estimated transaction costs, volatility 5.85 percent, a worst fall of 17 percent and a Sharpe ratio of 1.22                                                                              |
| Quantpedia, summarising Do and Faff                      | The same idea extended to 2008                                                                 | Profits declined; the drop was not explained by hedge funds crowding in, but by the convergence itself becoming less reliable, so a pair that was close in one period was less likely to still be close in the next |
| Quantpedia, summarising Rad, Faff and others             | Distance and cointegration methods, American shares, 1962 to 2014                              | After costs, the two methods returned about 36 and 33 basis points a month; before costs, about 88 and 83                                                                                                           |
| Quantpedia, summarising Clegg                            | Whether a pair that is tied together in one period stays tied in the next, over 860,000 pairs  | The evidence did not support persistence: being cointegrated in one period did not make it likely in the next                                                                                                       |
| Fil, reported by Quantpedia                              | Common distance and cointegration rules, American shares, 1990 to 2020, including the pandemic | The strategy failed to beat the market even after tuning, but performed very strongly during falling markets                                                                                                        |

The last row is the tension. The strategy is often called market-neutral, yet several studies find it
does well precisely when the market falls, which makes it look like protection rather than a source of
returns. The Sharpe ratios in the two intraday rows, 9.25 and 3.011, are far above the 1 to 1.5 usual
in the daily studies, and a figure that far above the literature normally reflects a sample or a cost
assumption rather than a better strategy.

## How this project relates to it

This repository studies the same family of ideas with its own honesty labels, in
[Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md).
Its Section 6 reports a regime-switching cointegration study on crude oil futures, and its findings
table concludes that the state of a spread should be modelled explicitly rather than traded with a
fixed band, which is the direct warning against step 6 above. The related brief
[Regimes, breakpoints and state switching](../../../strategies/books2/24_regimes_and_change_points.md)
explains why a filter that updates as data arrives differs from one that uses the whole history at once.

This repository also studies the sector-level version of the idea,
[Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md), whose Section 6
collects the relative-value evidence and whose Section 9 places pairs trading in a table of strategies
allowed even in a market with no memory, with the note that the evidence is thin. The project tutorial
[Dispersion and relative value](../../project/dispersion-and-relative-value/README.md) works the same
gap at a monthly rhythm and does the same cost arithmetic.

## Where it goes wrong

- The relationship does not last. A pair tied together in one three-month window is often not tied in
  the next, the finding of the 860,000-pair study, so a rule that reselects rarely trades stale pairs.
- The gap can widen forever. If the two companies were never really equivalent, or one is bought or
  broken, the gap is a permanent change rather than an accident, and the losing leg keeps losing.
- Costs are heavy relative to the prize. Four crossings per round trip plus the borrow fee eat a large
  share of a gap that may only be a fraction of a percent wide, especially at ten-minute frequency.
- Selling short has its own risks. To sell short you must borrow the share, pay rent while the
  position is open, and accept that the lender can ask for it back and force you to close early. The
  short leg also has no natural ceiling: its loss grows without bound if the share keeps rising.
- The measurement can flatter itself. Selecting the pairs whose relationship held best in the past,
  then measuring on that same past, is the standard way a pairs backtest looks far better than the
  trade ever was. The intraday papers use a rolling out-of-sample window, but a three-month window on
  one sector over one year is a small amount of evidence, and the rule is one of the most widely known
  in the industry, so the easy cases are competed away first.
- A correlation spike turns the book directional. In a crisis the shares that normally move apart fall
  together, the short leg stops delivering the gains the book counted on, and the hedge fails at the
  worst moment.

## Try it yourself

You need a spreadsheet and two shares you can look up daily prices for, ideally two banks.

1. Make a column for the date, a column for the price of share A and a column for the price of
   share B, one row per day for two years.
2. Add a column for the gap, price A minus a fitted line through price B, using your spreadsheet's
   slope function for `beta` and `alpha = average(A) - beta * average(B)`.
3. Add a column for the gap's average so far, and a column for its typical swing, the standard
   deviation.
4. Add a column for the count of swings: the gap minus its average, divided by the typical swing.
5. Mark every row where that count is above 2.33 or below -2.33. Those are the rows the rule would
   have opened a position.
6. For each marked row, look at where the count is a week later: has it moved back toward zero, or
   carried on? Count both outcomes.

What to notice: count how many marks were followed by the gap closing and how many by it widening, and
how much the answer changes when you shorten the window used to fit the line. Also count crossings:
every round trip needs four, so at five basis points each it costs about 0.2 percent of the notional.

## Where this came from

- [QuantConnect strategy library: intraday dynamic pairs trading](https://www.quantconnect.com/tutorials/strategy-library/intraday-dynamic-pairs-trading-using-correlation-and-cointegration-approach),
  the two-stage selection, the thresholds, the rolling window and the September 2013 result.
- George J. Miao, [High Frequency and Dynamic Pairs Trading Based on Statistical Arbitrage](https://ccsenet.org/journal/index.php/ijef/article/view/33007),
  International Journal of Economics and Finance, Volume 6, Number 3, 2014, for the cumulative
  56.58 percent and the Sharpe ratios.
- [Quantpedia: pairs trading with stocks](https://quantpedia.com/strategies/pairs-trading-with-stocks),
  the restatement of the rules and the abstracts and figures from Gatev, Goetzmann and Rouwenhorst, Do
  and Faff, Rad and Faff, Clegg, and Fil on the same page.
- [Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  Section 6, this repository's brief on regime-switching cointegration and why a fixed band is the
  wrong shape for a spread.
- [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md), Sections 6 and
  9, this repository's collection of relative-value evidence and its place in the eligibility table.
- [Dispersion and relative value](../../project/dispersion-and-relative-value/README.md), the project
  tutorial that builds the same trade at a monthly rhythm, including its short-side break-even
  identity.

## Words used in this tutorial

- cointegration: a tie between two prices that wander on their own, such that a particular
  combination of them returns to a level.
- correlation: a number between -1 and +1 describing how closely two series move together.
- market neutral: a book whose result does not depend on whether the market rises or falls.
- p-value: the chance of seeing a test result at least this extreme if there were really no
  relationship.
- residual: the part of one price that its usual relationship with another does not explain.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- z-score: how many standard deviations a value sits from its average.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
