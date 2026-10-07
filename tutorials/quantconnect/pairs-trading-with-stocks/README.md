# Pairs trading: betting that two look-alike shares come back together

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                        |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| What it trades            | Two shares at once: one bought, one sold short, so the bet is on their prices relative to each other rather than on the market as a whole                                                                                                                    |
| How often it trades       | The pair list is rebuilt every six months, and a pair is traded when its gap stretches unusually far and closed again when it narrows                                                                                                                        |
| What you need             | A spreadsheet and one year of daily closing prices for the shares you want to compare                                                                                                                                                                        |
| Where the rules come from | [QuantConnect strategy library, pairs trading with stocks](https://www.quantconnect.com/tutorials/strategy-library/pairs-trading-with-stocks) and the [Quantpedia pairs trading entry](https://quantpedia.com/strategies/pairs-trading-with-stocks) it cites |
| The underlying research   | Gatev, Goetzmann and Rouwenhorst, [Pairs Trading: Performance of a Relative Value Arbitrage Rule](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=141615)                                                                                                |
| How well it held up       | Mixed: the classical 1962 to 2002 result is large and survives conservative costs, but later work finds the profits declining, most of the effect gone in ordinary markets, and the modern strategy failing to beat the market except during crises          |
| Also appears in           | [Dispersion and relative value](../../project/dispersion-and-relative-value/README.md) in this collection, which describes the same bet between two baskets rather than two shares                                                                           |

## The idea in one paragraph

Two companies in the same business, selling similar things to similar customers, often have share
prices that rise and fall together for years. Most of the time the gap between the two prices stays
within a familiar range. Every so often one of them jumps or falls for a reason that has nothing to do
with the other, and the gap stretches wider than usual. This strategy finds, for a chosen share, the
other share whose price has tracked it most closely over the past year, and watches the gap between
them. When the gap stretches about two of its usual swings away from normal, it buys the share that
fell behind and sells short the one that ran ahead, betting the gap closes. When the gap narrows back
to normal, both positions are closed.

## Why anyone believed it

When two shares have tracked each other for a year, the reason is usually shared: the same customers,
the same suppliers, the same regulation, the same economy. If one of them then moves and the other
does not, the move is more likely to be a temporary accident than a real change in the business. That
is the whole bet.

The accident has a source, and it is usually a person trading for a reason other than the outlook. A
fund facing withdrawals sells the share that is easiest to sell; an index rebuild forces money out of a
company that is leaving and into its replacement; a large investor moving a position pushes one price
away from the other. The counterparty is impatient or constrained, and the pairs trader supplies the
liquidity the impatient trader needs, buying cheaply on one side and selling expensively on the other.
The strategy is paid when the pressure passes and the relationship reasserts itself.

## An everyday comparison

Two neighbouring shops on the same street sell the same brand of paint. Week to week their prices are
almost identical, because if one charged much more nobody would shop there. One Saturday, the owner of
one shop is short of cash for a delivery and drops the price by a fifth to clear stock quickly. The
paint is identical, so the gap is plainly an accident of the seller's circumstances, not a sign that
one tin is better. Anyone who could buy the cheap tins and, at the same moment, sell tins at the other
shop would pocket the difference when the sale ended and the price returned to normal. Pairs trading
looks for that same accident in the stock market, and needs the same confidence the two things match.

## The rules, step by step

1. Choose a universe of shares, for example the large American companies with enough daily trading for
   costs to be low.
2. Take the last year of daily closing prices for every share. This is called the formation period.
   The QuantConnect page uses one year; the Quantpedia page uses twelve months and calls the following
   six months the trading period.
3. Rescale each share's price series so that its first day is 1.00. Divide every day's price by the
   price on the first day. Two shares can now be compared even if one trades at 50 dollars and the
   other at 500.
4. For each share, and each possible partner, add up the squared differences between the two rescaled
   series, day by day. The partner with the smallest total is the closest match.
5. Keep the best pairs. The QuantConnect page keeps the best 4 pairs. The Quantpedia page keeps the
   best 20. Rebuild the list every six months.
6. For a pair you are watching, work out the gap between the two shares: divide one price by the other.
   Record the gap's average and how widely it has swung, using the past year.
7. When the gap is about two of its usual swings above its average, the first share is expensive
   relative to the second: sell the first short and buy the second. When the gap is about two swings
   below its average, do the reverse. Put the same amount of money on each side.
8. Close the position when the gap returns to close to its average, or when the six-month trading
   period ends, whichever comes first.
9. Hold nothing in between. Do not add to the position because the gap stretched further.

## The maths, with every symbol named

The distance between two rescaled price series, which is what picks the partner:

```text
SSD = sum over t of ( X_t / X_1 - Y_t / Y_1 ) ^ 2
```

- `SSD` is the sum of squared differences, the match score for a candidate pair.
- `X_t` is share X's price on day `t`, and `X_1` is its price on the first day of the formation period.
- `Y_t` and `Y_1` are the same quantities for share Y.
- `sum over t` means add the squared gap for every day in the formation period.
- A small `SSD` means the two shares moved almost identically; the smallest `SSD` wins.

The gap between the two shares, and how unusual it is:

```text
G_t = P_A,t / P_B,t
z_t = ( G_t - G_bar ) / s_G
```

- `G_t` is the gap on day `t`, the price of share A divided by the price of share B.
- `G_bar` is the average of the gap over the look-back window, here the past year.
- `s_G` is the standard deviation of the gap over the same window, the size of a typical swing.
- `z_t` is how many typical swings the gap sits from its average.
- The rule opens when `z_t` is near plus or minus two and closes when `z_t` returns near zero.

The profit of a completed pair, with the same money `M` on each side:

```text
Profit = M * ( R_B - R_A )
```

- `Profit` is the gross profit in the money of the account.
- `M` is the money placed on each leg, so `2 * M` is committed in total.
- `R_A` is the return of share A over the holding period, as a decimal.
- `R_B` is the return of share B over the same period.
- The market's common move sits inside both returns and is cancelled by the subtraction; only the
  difference matters.

The cost is the same shape as for any strategy, but with more crossings than a simple share purchase:

```text
Cost = 2 * k * M * c + borrow
```

- `k` is the number of legs, two for a pair.
- `c` is the cost per trade as a fraction traded, covering the bid-ask gap. Modern large American
  shares cost about 0.002, that is 20 basis points; older studies used 26 to 35 basis points.
- The first term counts opening and closing each leg, four crossings in all.
- `borrow` is the fee paid to the lender for as long as the short leg is open.

## A worked example

Two paint-retailer shares, A and B. Six months of closing prices, which stand in for the formation
year. First the match score for the pair, using the first three days: A is 100, 103, 101 and B is 50,
51, 50.5, so the rescaled series are 1.00, 1.03, 1.01 and 1.00, 1.02, 1.01.

```text
day 1: ( 1.00 - 1.00 ) ^ 2 = 0.0000
day 2: ( 1.03 - 1.02 ) ^ 2 = 0.0001
day 3: ( 1.01 - 1.01 ) ^ 2 = 0.0000
SSD = 0.0001
```

A tiny `SSD` is what a close pair looks like. Now the gap series for the six months, using the raw
price ratio `G = P_A / P_B`:

| Month | Price of A | Price of B | Gap    |
| ----- | ---------- | ---------- | ------ |
| 1     | 100.00     | 50.00      | 2.0000 |
| 2     | 103.00     | 51.00      | 2.0196 |
| 3     | 101.00     | 50.50      | 2.0000 |
| 4     | 106.00     | 52.00      | 2.0385 |
| 5     | 109.00     | 53.00      | 2.0566 |
| 6     | 112.00     | 55.00      | 2.0364 |

```text
average gap G_bar = 12.1511 / 6 = 2.0252
squared gaps from the average: 0.000634, 0.000031, 0.000634, 0.000176, 0.000987, 0.000125
their sum = 0.002588; divided by 5 = 0.000518; s_G = sqrt( 0.000518 ) = 0.02275
two swings above the average = 2.0252 + 2 * 0.02275 = 2.0707
two swings below the average = 2.0252 - 2 * 0.02275 = 1.9797
```

Now the trading period. In month 7, A is at 118.00 and B is at 54.00, so the gap is 2.1852, well above
the 2.0707 line. The rule opens: sell A short and buy B, with 50,000 on each leg inside a 100,000
account. The pair is held through month 8 and closed in month 9, when A is at 116.00 and B is at 57.50,
so the gap is 2.0174, back through the average.

| Leg     | Entry price | Exit price | Return          |
| ------- | ----------- | ---------- | --------------- |
| Long B  | 54.00       | 57.50      | +6.4815 percent |
| Short A | 118.00      | 116.00     | +1.6949 percent |

The short leg gains when the price falls: A fell from 118.00 to 116.00, so the short position earned
1.6949 percent.

```text
Profit = 50,000 * ( 0.064815 + 0.016949 ) = 50,000 * 0.081764 = 4,088.20

Trading cost = 4 * 50,000 * 0.002 = 400.00
Borrow cost  = 50,000 * 0.005 * ( 2 / 12 ) = 41.67   (0.5 percent a year, held two months)
Net profit   = 4,088.20 - 400.00 - 41.67 = 3,646.53, which is 3.65 percent of the 100,000 account
```

Two things are worth noticing. Four crossings plus two months of borrowing took about ten percent of
the gross profit, which is the same order as the edge itself, so a slightly wider gap or a slightly
higher cost changes the answer a lot. And the short leg has no natural ceiling: if A had kept rising,
the loss on that leg would have grown without limit while the long leg could only gain what B could
rise by. The worked example shows how to apply the rules; it says nothing about whether the strategy
works.

## What the research actually found

| Source                                   | What it measured                                                       | Result                                                                                                                                                                                                                                     |
| ---------------------------------------- | ---------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Gatev, Goetzmann and Rouwenhorst         | Daily US data, 1962 to 2002, pairs matched by smallest distance        | Average annualised excess returns of up to 11 percent for portfolios of pairs; profits typically exceed conservative transaction costs, and the effect differs from ordinary reversal profits because the two shares share a common factor |
| Quantpedia, pairs trading entry          | The same rules, top 20 pairs, spread opened at two standard deviations | 11.16 percent a year, volatility 5.85 percent, worst fall 17 percent, reward-to-risk 1.22, over 1962 to 2002, with 40 instruments and confidence rated strong                                                                              |
| QuantConnect algorithm                   | Large US shares, best 4 pairs, rebuilt every six months                | Formation over one year by smallest distance, open at two standard deviations, close on reversion                                                                                                                                          |
| Do and Faff                              | The same strategy extended to June 2008                                | Profits continue to decline; the rise in hedge-fund activity is not the explanation; the pairs are simply less likely to be close substitutes in the trading period                                                                        |
| Chen, Chen and Li                        | The economic drivers of the profit                                     | Large and significant abnormal returns, but diminishing over time; most of the profit comes from correlations explained by ordinary factors; it is not pure short-term reversal                                                            |
| Rad, Low and Faff                        | US shares, 1962 to 2014, with time-varying costs                       | About 36 basis points a month for the distance method and 33 for the cointegration method after costs, against 88 and 83 before costs                                                                                                      |
| Fil, "Gold Standard Pairs Trading Rules" | NYSE shares, 1990 to 2020 including the Covid-19 crisis                | The baseline strategy fails to beat the market in both raw and risk-adjusted terms; about 0.11 percent a month against the market's 0.47 (`2010.01157v1`, pp.5-6), but it earns surpluses of about 2 percent a month in bear markets (p.5) |

Read together, the picture is a decay rather than a refutation. The original result was large and
survived conservative costs; later work found the pairs were less likely to stay similar; and the most
recent replication found that the naive version no longer beats the market except in a crisis, when the
forced selling the strategy feeds on is most common. The disagreement is mostly about what to call
that: a fading anomaly, or a hedge that only pays in bad markets. Nobody has settled it, and the
crisis result relies on knowing when a bear market has begun, which is itself hard.

## How this project relates to it

This repository's closest treatment is the sibling tutorial
[Dispersion and relative value](../../project/dispersion-and-relative-value/README.md), which runs the
same bet between two baskets rather than two shares and works the arithmetic with costs through.

The rules and evidence live in this repository's own study,
[Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md). Its Section 6
collects the dispersion and relative-value evidence with honesty labels, including the Gatev,
Goetzmann and Rouwenhorst figure of about 12 percent excess a year with 4 to 6 percent annualised
volatility, and a cointegration-based sector version whose reward-to-risk moved from 0.28 to 0.81 with
a change of specification. The design note
[Entry/Exit Price Engine: design](../../../strategies/entry_exit_engine_design.md) supplies one piece
of machinery a pairs book needs, the break-even identity for a short position, because the short leg
is the one most easily mis-costed.

[Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md)
reports the modern academic version: modelling the spread as switching between regimes, rather than
trading a fixed two-standard-deviation band, produced a crude-oil result of 15.18 percent a year with a
reward-to-risk of 1.18 and a worst fall of 5.73 percent, against minus 28.47 percent and minus 0.74 for
a crude-oil fund (`2309.00875v3`, reported at pp.21-25 of the brief). That paper trades three futures
contracts, not shares, and the brief labels the sample as narrow.

## Where it goes wrong

- The relationship breaks permanently. Two shares that tracked each other can stop, because one company
  is bought out, loses a contract, or changes what it does. The rule assumes the gap is temporary; if it
  is not, the losing leg keeps losing.
- The match is picked from history. Choosing a pair because it moved together in the past, then
  measuring the result on that same past, is the classic way a pairs backtest flatters itself. Out of
  sample, cointegration is not a persistent property: one study of more than 860,000 pairs found no
  support for it carrying over.
- Costs are heavy relative to the prize. Four crossings per round trip, plus a borrow fee for as long
  as the short leg is open, plus the fact that the profit is only a few percent of the money at risk.
- Crowding. The strategy is simple and public, and the shares with the cleanest historical matches are
  the first to attract money, which narrows the gap before a new entrant can capture it.
- The short leg has no floor. A short position loses without limit if the share keeps rising, and the
  long leg cannot make up for it. Pairs are described as market neutral, but they are not risk free.
- The threshold is a tuned number. Two standard deviations is a convention; studies that searched over
  thresholds found the best value moves with costs and market conditions, and that no single static
  rule reliably beat the market.

## Try it yourself

You need a spreadsheet and daily closing prices for two large shares in the same industry, from any
finance website.

1. Make one column per share and one row per trading day, for one year. Add a row for each day's ratio
   of share A's price to share B's price.
2. Add a row for the average of that ratio column, then a row for the difference between each day's
   ratio and the average, then a row for the square of that difference, then the sum of the squares
   divided by one less than the number of days, then the square root. That last number is the typical
   swing of the ratio.
3. Add a row that counts the swings: the ratio minus its average, divided by the typical swing.
4. Mark every day where that count is above 2 or below minus 2. Those are the days the rule would have
   opened a trade, and the direction is the opposite of whichever share ran ahead.
5. For each marked day, look forward and find the first later day when the count crosses back through
   zero. That pair of days is one completed trade. Record the return of each leg.
6. Do the same for a second pair chosen at random from the same industry, so you have something to
   compare.

What to notice: count how many marked days were followed by the gap closing, and how many were
followed by the gap carrying on or widening. On real pairs you will find both, and the mix is the whole
question. Also notice how sensitive the count is to the look-back window: a shorter window makes
swings look bigger and opens trades more often, which is exactly the knob a backtest can be tuned
with.

## Where this came from

- [QuantConnect strategy library: pairs trading with stocks](https://www.quantconnect.com/tutorials/strategy-library/pairs-trading-with-stocks),
  the rules as implemented: best 4 pairs, one-year formation, six-month trading, open at two standard
  deviations, close on reversion.
- [Quantpedia: pairs trading with stocks](https://quantpedia.com/strategies/pairs-trading-with-stocks),
  the restated rules, the top-20 construction, the 1962 to 2002 figures, and the source and other
  papers, including Do and Faff, Chen, Chen and Li, Rad, Low and Faff, Clegg, and Bowen and Hutchinson.
- Gatev, Goetzmann and Rouwenhorst, [Pairs Trading: Performance of a Relative Value Arbitrage Rule](https://papers.ssrn.com/sol3/papers.cfm?abstract_id=141615),
  the original 1962 to 2002 study.
- `2010.01157v1`, Gold Standard Pairs Trading Rules: Are They Valid? (pp.3-8), held in the local
  corpus and read directly for the modern replication and its result table.
- [Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md), Section 6, and
  [Dispersion and relative value](../../project/dispersion-and-relative-value/README.md), this
  repository's own study and its sibling tutorial.
- [Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  this repository's brief, Section 6, where the regime-switching result and its cost estimates live.

## Words used in this tutorial

- basis point: one hundredth of one percent, so twenty basis points is 0.20 percent.
- borrow fee: the rent paid to the lender of something you have sold short.
- cointegration: a statistical relationship in which two prices can wander but their gap stays stable.
- formation period: the stretch of past prices used to choose the pairs.
- market neutral: holding a book whose gains do not depend on the market rising or falling.
- short selling: borrowing something you do not own, selling it, and buying it back later.
- spread: here, the gap between the two shares' prices; elsewhere, the gap between buy and sell prices.
- standard deviation: a measure of how widely a set of numbers is scattered around its own average.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
