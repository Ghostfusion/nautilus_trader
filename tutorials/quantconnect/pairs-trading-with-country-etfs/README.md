# Pairs trading with country funds: two countries that normally move together, and the moments they do not

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                                                       |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Funds that each hold the shares of one foreign country, bought and sold short in pairs at the same time                                                                                                                                                                                                     |
| How often it trades       | Checked each day; a pair is opened when two funds have drifted apart and closed when they come back together, with a new list of pairs chosen each month                                                                                                                                                    |
| What you need             | A spreadsheet and 120 daily closing prices for about twenty country funds                                                                                                                                                                                                                                   |
| Where the rules come from | [QuantConnect strategy library, pairs trading with country ETFs](https://www.quantconnect.com/tutorials/strategy-library/pairs-trading-with-country-etfs) and the [Quantpedia entry](https://quantpedia.com/strategies/pairs-trading-with-country-etfs) it cites                                            |
| The underlying research   | Thomakos, Wang and Schizas, [Pairs Trading on International ETFs](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1958546)                                                                                                                                                                              |
| How well it held up       | Weak: the headline result comes from one sample over 1996 to 2009, and the entry that states the rules records that its own later out-of-sample test was significantly negative and suggests the original sample may have been searched rather than found, so the replication reduced the effect to nothing |
| Also appears in           | [Pairs trading with stocks](../pairs-trading-with-stocks/README.md) in this collection, the same idea applied to individual shares instead of country funds                                                                                                                                                 |

## The idea in one paragraph

Two countries that trade with each other, share a currency or sell the same commodities often have
share markets that rise and fall together. When one country's fund suddenly jumps away from the
other, the gap is usually the result of a temporary event rather than a permanent change, and it
tends to close. This strategy watches a list of country funds, finds the pairs that have moved
together most closely over the past few months, and each day checks whether the two have drifted
apart. When they have, it sells short the one that has risen more and buys the one that has lagged,
putting the same amount of money on each side, so the overall direction of world markets matters
little. It waits for the two to come back together and then closes both sides.

## Why anyone believed it

When two share markets move together, there is usually a reason: the same customers buy their
exports, the same commodities drive their industries, the same currency is in use. A temporary event
in one country, a political scare, a currency wobble, a change in an index's membership, can move one
fund without touching the shared reasons the two move together. The price of the fund that moved has
gone beyond what the shared reasons justify.

The counterparty is the person who sold in a hurry: a fund facing withdrawals, an investor reacting
to a headline, an index that had to be rebalanced on a fixed date. Providing the other side of those
forced trades is a service, and in a market where some people must trade at any price, the person who
is willing to wait is paid. The strategy is the waiter: it steps in when the gap is wide and is paid
when the panic passes and the prices rejoin.

## An everyday comparison

Two supermarkets on the same street sell the same branded milk, and for years the price is within a
cent in both shops. One day a delivery problem makes one shop's milk forty cents dearer. Nobody
believes milk has become more valuable in that one shop; the gap is a temporary accident, and within
a week the prices rejoin. A shopper who could buy at the cheap shop and sell at the expensive one
would have a safe profit, as long as the gap really was temporary. That is what this strategy does
with two funds, and its whole risk is that sometimes the delivery problem is permanent.

## The rules, step by step

1. Choose a list of about twenty-five funds, each holding the shares of one foreign country, such as
   Australia, Canada, Japan, Germany and so on. Quantpedia's own version of the same rules uses
   twenty-two.
2. For each fund build a cumulative index over the past 120 trading days. Start each fund's index at
   1.000 and multiply it by one plus each day's percentage change, so all the funds are on a scale
   where they can be compared.
3. For every possible pair of funds, compute the distance: the average size of the gap between their
   two indices over those 120 days, counting a gap either way as a positive amount.
4. Rank the pairs by that distance, smallest first, and keep the five pairs that have moved together
   most closely. Roughly three hundred pairs are examined at this step to keep five.
5. Then, for the next twenty trading days, look at the five kept pairs each day. For one pair, if the
   first fund's index is above the second's by more than half the pair's distance, sell short the
   first fund and buy the second, putting the same amount of money in each. If the second fund's
   index is above the first's by more than half the distance, do the reverse.
6. Close both sides of a pair when the gap between the two indices falls back inside the band, that
   is, less than half the distance either way. Also close if the pair has been open for twenty trading
   days without converging.
7. At the start of each month, throw away the old list, recompute the distances from the latest 120
   days, and choose five new pairs.

Market neutral means the two sides are the same size, half bought and half sold, so a common move in
both countries' markets cancels and only the difference between them remains.

## The maths, with every symbol named

First, each fund's daily return and its cumulative index:

```text
r_t^n = P_t^n / P_(t-1)^n - 1
R_t^n = (1 + r_1^n) * (1 + r_2^n) * ... * (1 + r_t^n)
```

- `P_t^n` is the closing price of fund `n` on day `t`.
- `r_t^n` is fund `n`'s return on day `t`, as a decimal.
- `R_t^n` is fund `n`'s cumulative index, which starts at 1 and multiplies up the daily returns. It is
  a single number that says how far the fund has travelled since the start of the formation period,
  so two funds that began at very different prices can still be compared.

Next, the distance between two funds, `a` and `b`, over the 120-day formation period:

```text
D^(a,b) = (1 / 120) * the sum over the 120 days of | R_t^a - R_t^b |
```

- `D^(a,b)` is the pair's distance: the average size of the gap between the two indices, whether the
  first is ahead or behind. The two vertical bars mean "take the size and ignore the sign".
- The Quantpedia entry describes the same quantity as the sum of squared deviations rather than the
  sum of absolute ones. The two pages therefore state the rule slightly differently, which is worth
  knowing: a squared version punishes a single large gap more than many small ones, and the two can
  rank pairs differently.

Next, the trading rule. With a threshold set at half the distance, the trigger level is:

```text
Trigger = 0.5 * D^(a,b)
```

- `Trigger` is how far apart the two indices must be before the pair is traded. A distance of 0.030
  gives a trigger of 0.015.
- If `R_t^a - R_t^b` is greater than `Trigger`, fund `a` is ahead, so the strategy sells `a` short and
  buys `b`. If the difference is below minus the trigger, it does the reverse.
- The pair is closed when the gap falls back inside the band from minus `Trigger` to plus `Trigger`.

Finally the profit, and the meaning of market neutral. If the strategy buys `b` and sells `a`, each
with half the account, the return over the holding period is:

```text
R_pair = 0.5 * ( R_end^b / R_start^b - 1 ) - 0.5 * ( R_end^a / R_start^a - 1 )
```

- `R_start` and `R_end` are the two funds' cumulative indices at the open and the close of the trade.
- The first half is the gain on the bought fund; the second half is the gain on the sold fund, which
  is a gain when the implied price falls, hence the minus sign.
- If both funds had moved by exactly the same percentage, the two terms would cancel and the pair
  would return nothing. That is what "market neutral" means in this precise sense: the return depends
  on the difference between the two funds, not on the level of the market.

The cost of a pair is charged on both legs, once when the pair is opened and once when it is closed:

```text
Cost_pair = 2 * c
```

- `c` is the cost of one trade as a fraction of the money traded, covering the gap between the buying
  and selling price plus commission. One basis point is one hundredth of one percent.
- The factor of 2 counts the opening and the closing of the pair, each of which trades 1.0 of the
  account in total across the two legs.
- A borrow fee is charged on the sold leg for as long as it is open, and it is not included in this
  expression.

## A worked example

The formation period gave one pair a distance `D` of 0.030, so the trigger is `0.5 * 0.030 = 0.015`.
Both indices start at 1.000. The table below is eight days of invented but plausible index values.

| Day | Index A | Index B | Gap (A minus B) | What the rule does                |
| --- | ------- | ------- | --------------- | --------------------------------- |
| 1   | 1.000   | 1.000   | 0.000           | nothing                           |
| 2   | 1.010   | 1.005   | +0.005          | nothing                           |
| 3   | 1.030   | 1.010   | +0.020          | gap above trigger: sell A, buy B  |
| 4   | 1.035   | 1.015   | +0.020          | hold                              |
| 5   | 1.028   | 1.014   | +0.014          | hold, gap back inside the band    |
| 6   | 1.018   | 1.020   | -0.002          | gap inside band: close both sides |
| 7   | 1.012   | 1.022   | -0.010          | nothing                           |
| 8   | 1.005   | 1.025   | -0.020          | gap below the band: buy A, sell B |

Work the trade that opens on day 3 and closes on day 6. Fund A went from 1.030 to 1.018, a fall of
`1.018 / 1.030 - 1 = -1.165` percent, and because A was sold short that is a gain of 1.165 percent on
half the account. Fund B went from 1.010 to 1.020, a rise of `1.020 / 1.010 - 1 = +0.990` percent on
the other half. The pair's gross return is:

```text
0.5 * 0.990 + 0.5 * 1.165 = 0.495 + 0.583 = 1.078 percent
```

The round trip trades 2.0 of the account, so at ten basis points per trade the cost is
`2 * 0.001 = 0.2` percent, and three days of borrow cost about 0.003 percent. The net result is
`1.078 - 0.2 - 0.003 = +0.875` percent of the account for that pair. Now suppose the strategy had
completed six such trades in a season, with the gross returns and the same 0.21 percent of costs
each time.

| Trade | Gross return  | Net after costs |
| ----- | ------------- | --------------- |
| 1     | +1.08 percent | +0.87 percent   |
| 2     | -0.90 percent | -1.11 percent   |
| 3     | +0.60 percent | +0.39 percent   |
| 4     | +1.40 percent | +1.19 percent   |
| 5     | -0.30 percent | -0.51 percent   |
| 6     | +0.75 percent | +0.54 percent   |

Compounding the six net returns gives
`1.0087 * 0.9889 * 1.0039 * 1.0119 * 0.9949 * 1.0054 = 1.0136`, about +1.36 percent over six round
trips. Two of the six trades lost money, which is what happens when a pair that was supposed to
converge instead keeps drifting, and the invented season ends only slightly ahead. The example is
made up, but the shape is not: a few convergences pay the costs, one or two non-convergences eat
several of them.

## What the research actually found

The published record is one strong in-sample number and a set of later studies that do not confirm it.

| Source                                                                             | What it measured                                                             | Result                                                                                                                                                                                                                                                                           |
| ---------------------------------------------------------------------------------- | ---------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Thomakos, Wang and Schizas, the source paper                                       | International exchange-traded funds, 1996 to 2009                            | Reported returns from pairs trading on international funds, and found the returns could be partially explained by economic factors between the countries                                                                                                                         |
| Quantpedia, the entry the rules come from                                          | Same rules, five closest pairs, a one-day delay before trading               | Indicative performance 20.6 percent a year, volatility 10 percent, worst fall 26.67 percent, reward to risk 1.66, over 1996 to 2009                                                                                                                                              |
| Quantpedia's own confidence note, on the same page                                 | The out-of-sample backtest of the rule                                       | Marked the confidence in the effect as weak, recorded that the out-of-sample backtest showed significantly negative performance, and suggested the in-sample result may have been the product of searching                                                                       |
| Fil, Gold Standard Pairs Trading Rules: Are They Valid?                            | The common distance and cointegration methods, American shares, 1990 to 2020 | The strategy failed to beat the market overall even after tuning, returning less than 0.2 percent a month against the market's 0.47 percent; it did perform strongly in falling markets, around 2 percent a month of excess return, but that needs knowing when a fall is coming |
| Doering, Does Pairs Trading with ETFs Work?                                        | Exchange-traded fund pairs, 2001 to 2016                                     | Fund pairs earned up to 27 basis points a month of excess return, but were usually less profitable than share pairs, and lower arbitrage risk was more than outweighed by greater market efficiency                                                                              |
| Tokat and Hayrullahoglu, Pairs trading: is it applicable to exchange-traded funds? | A cointegration-based version, 45 pairs, 2007 to 2021                        | An average annual return of 15 percent with a reward to risk of 1.43 after costs, performing better in falling markets than in rising ones                                                                                                                                       |

Read together: the in-sample result is large, the later tests are not. The distinction the sources do
agree on is that pairs trading tends to do better in falling markets, when more investors are being
forced to sell, and worse in quiet rising markets, when there is no forced selling to be paid for.
That is a coherent story and it is also a hard one to trade, because it requires knowing in advance
which kind of market is coming.

## How this project relates to it

This repository contains its own design for exactly this kind of screen, and it is the most useful
thing on this page to read after the rules. The document is
[Relative-value screening](../../../docs/design/relative_value_screening.md), and its second section
works out what happens when an investor tests many pairs for a relationship at a five percent
threshold. With eleven funds there are 55 unique pairs, and at that threshold the expected number of
false matches is 2.75; the chance of at least one false match is 94 percent, and of at least three is
52 percent. A screen that keeps two or three pairs has demonstrated nothing. The strategy on this
page keeps five pairs out of roughly three hundred, which is the same problem several times larger.

The building blocks are in the repository too. The screening logic in
[python/nautilus_trader/optimization/relative_value.py](../../../python/nautilus_trader/optimization/relative_value.py)
and its companion capability list refuse a pair whose spread does not revert, or whose two series
cannot be told apart, before any trade is considered; that is the check the original rule does not
make. The study in
[Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md) reports its own
cointegration-based pairs test on sectors, which returned 1.01 percent in total with a reward to risk
of 0.28, and which the study describes as extremely sensitive to how the test is specified. No file
in this repository runs the country-fund strategy itself.

## Where it goes wrong

- The relationship can break for good. A country can change its currency regime or its index
  membership, after which the two funds are no longer the same pair, and nothing protects against it.
- The screen is a search, and the search is not counted. Five survivors are the best of roughly three
  hundred pairs, so a pair that passed is not the same as a pair that passed an independent test.
- The headline result did not survive. The entry that states the rules marks its confidence weak and
  records a significantly negative out-of-sample test, so the 20.6 percent belongs to one sample.
- Costs are charged on two legs, opened and closed. A round trip trades twice the account, and less
  liquid country funds have wider spreads, so a small convergence can be eaten before it is captured.
- Market neutral is not the same as safe. Equal and opposite positions cancel the common market move
  but not a shock to one country, and in a crisis the pair often widens when the book is largest.
- The two source pages do not agree on the formula. One uses the average absolute gap, the other the
  sum of squared gaps, and one uses twenty-five funds while the other uses twenty-two, so the rules
  are not pinned down and the results depend on which version was tested.

## Try it yourself

You need a spreadsheet and 120 daily closing prices for two funds that track two countries you
suspect move together, for example two commodity exporters.

1. Put the dates down one column and the two price series in the next two columns.
2. Add two columns for the cumulative index: the first row is 1.000, and each later row is the
   previous row times `today's price / yesterday's price`.
3. Add a column for the gap, `index of fund one minus index of fund two`, and a column for its size
   with the sign removed.
4. Compute the average of the size column; that is the pair's distance `D`. Mark the band as plus and
   minus `0.5 * D`.
5. Add a column that shows "open, sell the first fund" when the gap is above the band, "open, sell
   the second fund" when it is below, and "close" when the gap is inside the band and a trade is open.
6. For each completed trade, compute the pair's return from the cumulative indices at the open and
   the close, using the formula above, then subtract 0.2 percent for costs.

What to notice: how rarely the gap crosses, and how many crossings come back inside the band within
a few days. Then recompute the distance using 100 days instead of 120: a pair that looks like a safe
convergence on one setting can look like a drift on another, which is the whole difficulty.

## Where this came from

- [QuantConnect strategy library: pairs trading with country ETFs](https://www.quantconnect.com/tutorials/strategy-library/pairs-trading-with-country-etfs),
  the rules as implemented: twenty-five funds, a 120-day formation period, the five closest pairs,
  a half-distance trigger, dollar-neutral legs and a twenty-day limit.
- [Quantpedia: pairs trading with country ETFs](https://quantpedia.com/strategies/pairs-trading-with-country-etfs),
  the performance figures, the instrument count, the sample period, the weak-confidence note and the
  list of related papers.
- Thomakos, Wang and Schizas, [Pairs Trading on International ETFs](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=1958546),
  the source paper.
- Fil, [Gold Standard Pairs Trading Rules: Are They Valid?](https://arxiv.org/abs/2010.01157),
  a recent test of the same methods on American shares that did not find the effect (`2010.01157v1`).
- Doering, [Does Pairs Trading with ETFs Work?](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=2816849),
  the exchange-traded fund comparison.
- Tokat and Hayrullahoglu, [Pairs trading: is it applicable to exchange-traded funds?](https://doi.org/10.1016/j.bir.2021.08.001),
  the cointegration-based version over a full market cycle.
- [Relative-value screening](../../../docs/design/relative_value_screening.md), this repository's own
  design for the screen and its counting of false matches, and
  [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md), its
  cointegration-based sector pairs test.

## Words used in this tutorial

- cointegration: a lasting relationship in which two prices move together even though neither settles
  down on its own.
- distance: the average size of the gap between two funds' cumulative indices over a formation period.
- formation period: the stretch of past prices used to choose which pairs to trade.
- long: owning something, so that a rise in its price is a gain.
- market neutral: holding equal amounts bought and sold, so that the overall market's direction
  matters little.
- short selling: borrowing something you do not own, selling it, and buying it back later, so that a
  fall in its price is a gain.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
