# Commodity momentum: buying the raw materials that have already been rising

Date: 2026-10-07. Revision 1.

| Field                     | Value                                                                                                                                                                                                                                                                         |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| What it trades            | Agreements to buy raw materials such as oil, metals and farm products at a fixed price on a future date, called futures                                                                                                                                                       |
| How often it trades       | Once a month, when the ranking is recomputed and the portfolio rebuilt                                                                                                                                                                                                        |
| What you need             | A spreadsheet and a table of monthly prices for a set of raw materials                                                                                                                                                                                                        |
| Where the rules come from | [QuantConnect strategy library, momentum effect in commodities futures](https://www.quantconnect.com/tutorials/strategy-library/momentum-effect-in-commodities-futures) and the [Quantpedia entry](https://quantpedia.com/strategies/momentum-effect-in-commodities) it cites |
| The underlying research   | Miffre and Rallis, [Momentum in Commodity Futures Markets](http://papers.ssrn.com/sol3/papers.cfm?abstract_id=702281)                                                                                                                                                         |
| How well it held up       | Mixed: the effect appears in several independent samples, including a hand-collected history going back to 1877, but the recent decades are weaker and the long-short version suffered a maximum fall near eighty percent                                                     |
| Also appears in           | The [sector momentum tutorial](../sector-momentum/README.md) in this collection, which applies the same ranking idea to industries rather than raw materials                                                                                                                  |

## The idea in one paragraph

Raw materials such as oil, copper, wheat and coffee are traded through agreements to buy them at a
fixed price on a future date. Some of these materials have risen a lot over the past year and some
have fallen. This strategy ranks them all by how much their price has changed over the past twelve
months, buys the group that rose the most, and at the same time sells the group that fell the most,
on the bet that the two groups keep going the same way for a while longer. Once a month it re-ranks
everything and rebuilds the portfolio. Because it buys one group and sells another, the overall
exposure to rising or falling prices is small; the bet is purely on the strong staying stronger than
the weak.

## Why anyone believed it

When the price of a raw material rises, the reason usually lasts for months. A poor harvest, a
pipeline closure or a mine strike does not fix itself in a week, so the shortage pushes the price up
gradually and the news arrives piece by piece rather than all at once. Traders who hear the news
late buy after the first move, which pushes the price further in the same direction.

The counterparty, then, is the trader who is slow to react, or who is forced to trade for reasons
unrelated to the outlook. A producer who has to sell next season's crop, a company that must buy
fuel regardless of the price, or a fund facing withdrawals all trade at fixed dates, not at the
moment that suits the price. There is also a second, older story. A raw material that is easy to
store and hard to find today tends to trade above its future price, a situation called
backwardation; one that is easy to supply and costly to store trades below it, called contango. The
research this strategy rests on found that it tends to buy the backwardated materials and sell the
contangoed ones, which gives the pattern an economic reason rather than being a coincidence.

## An everyday comparison

Think of the league table of a chain of shops. At the end of each quarter the head office ranks the
branches by sales. The branches at the top of the table tend to be at the top again the next quarter,
not because the rankings are fixed, but because the reasons a branch sells well, such as a good
location and a loyal set of customers, do not change quickly. The branches at the bottom stay at the
bottom for the same reason in reverse. The strategy here is to back the top of the league table
against the bottom, and to update the bet at the end of every quarter rather than holding forever.

## The rules, step by step

1. Choose a fixed list of raw materials whose prices you can follow, for example oil, copper, gold,
   wheat, corn, soybeans, coffee, sugar, natural gas and live cattle. Both sources use the futures
   that trade on the large American exchanges.
2. For each material, work out its return over the past twelve months: take today's price, divide by
   the price twelve months ago, and subtract one. The QuantConnect version calls this the rate of
   change over twelve months.
3. Rank the materials by that return, best first.
4. Split the ranking into equal fifths, so with ten materials the top two form the highest fifth and
   the bottom two form the lowest fifth.
5. Buy the highest fifth and sell the lowest fifth. Selling something you do not own, so that you
   gain when its price falls, is called going short. The two sides take equal amounts of exposure:
   half of the position is long and half is short, which is written as plus 0.5 divided by the number
   in each group on the long side and minus 0.5 divided by the same number on the short side.
6. Hold for one month. Do not look at the prices in between.
7. At the start of the next month, recompute step 2 for every material and repeat from step 3. Close
   anything that has left its group and open whatever has joined it.

The QuantConnect page uses dairy, meat and forestry futures from the large American exchange; the
Quantpedia version uses a broad basket of about thirty raw materials. The rules are the same; only
the list differs.

## The maths, with every symbol named

The entire strategy is one calculation per material, one sort, and one weighted sum.

The twelve-month return of a material:

```text
M = P_today / P_twelve_months_ago - 1
```

- `M` is the momentum score of the material, written as a decimal: 0.42 means 42 percent.
- `P_today` is the price today, taken from the agreement that is easiest to trade, which is usually
  the one closest to delivery.
- `P_twelve_months_ago` is the price on the same day one year earlier.

Then rank the materials by `M` from largest to smallest and split the ranking into fifths. If there
are `N` materials, each group holds `N / 5` of them. Give each material in the highest fifth this
weight, and each in the lowest fifth the opposite:

```text
w = +0.5 / (N / 5) for the highest fifth
w = -0.5 / (N / 5) for the lowest fifth
w = 0 for everything in between
```

- `w` is the fraction of the account's notional value placed in one material; a positive number means
  bought and a negative number means sold short.
- The highs add up to plus half and the lows add up to minus half, so the bought side and the sold
  side have the same size and the net exposure to the whole market is zero.

Because the two sides are equal in size, the portfolio's return each month is half the gap between
the average return of the strong group and the average return of the weak group:

```text
R_portfolio = 0.5 * (R_strong - R_weak)
```

- `R_strong` is the average return of the materials that were in the highest fifth.
- `R_weak` is the average return of the materials that were in the lowest fifth.
- When the strong group rises more than the weak group, the result is positive; when it lags, the
  result is negative, whichever way the whole market moved.

Finally the cost of rebuilding the portfolio. If a fraction `t` of the position is traded and each
trade costs a fraction `c` of the amount traded:

```text
Cost = t * c
```

- `t` is the traded fraction: 2.0 if the whole book is closed and reopened, because the sale and the
  purchase each count, and less when some positions are kept.
- `c` is the cost per trade as a fraction of the amount traded, covering commission and the gap
  between buying and selling prices. Futures on liquid raw materials are cheap to trade; a figure of
  0.0005, that is five hundredths of a percent, is realistic for the largest ones.

## A worked example

Ten invented materials, ranked by their return over the past year. The numbers are made up but are of
the size these prices actually move.

| Material    | Price a year ago | Price today | M     | Rank |
| ----------- | ---------------- | ----------- | ----- | ---- |
| Coffee      | 100.00           | 142.00      | +0.42 | 1    |
| Crude oil   | 80.00            | 108.00      | +0.35 | 2    |
| Copper      | 90.00            | 117.00      | +0.30 | 3    |
| Soybeans    | 120.00           | 144.00      | +0.20 | 4    |
| Corn        | 60.00            | 69.00       | +0.15 | 5    |
| Cattle      | 110.00           | 121.00      | +0.10 | 6    |
| Wheat       | 70.00            | 75.60       | +0.08 | 7    |
| Gold        | 200.00           | 208.00      | +0.04 | 8    |
| Sugar       | 50.00            | 49.00       | -0.02 | 9    |
| Natural gas | 40.00            | 32.00       | -0.20 | 10   |

With ten materials each fifth holds two. The highest fifth is coffee and crude oil; the lowest fifth
is sugar and natural gas. Each of the four gets a weight of 0.25, positive for the first two and
negative for the last two. Now follow the portfolio for five months. `R_strong` is the average return
of the two bought materials and `R_weak` the average of the two sold materials, in percent.

| Month | Strong group return | Weak group return | Gross result | Cost | Net result |
| ----- | ------------------- | ----------------- | ------------ | ---- | ---------- |
| 1     | +2.5                | -2.5              | +2.50        | 0.10 | +2.40      |
| 2     | -1.0                | +2.0              | -1.50        | 0.10 | -1.60      |
| 3     | +4.0                | -3.0              | +3.50        | 0.10 | +3.40      |
| 4     | +1.0                | +3.0              | -1.00        | 0.10 | -1.10      |
| 5     | -2.0                | -5.0              | +1.50        | 0.10 | +1.40      |
| Total |                     |                   | +5.00        | 0.50 | +4.50      |

Read the fourth row carefully, because it is the lesson of the whole strategy. Both groups rose that
month, the bought one by 1 percent and the sold one by 3 percent, so the portfolio lost 1 percent
even though prices went up. The strategy does not bet on prices rising; it bets on the strong group
beating the weak one, and it can lose while everything rises. The cost is 0.10 percent each month
under the assumption that the whole book turns over and each trade costs five hundredths of a
percent. Five months together returned 4.50 percent, about 0.9 percent a month, which is in the
range the research reports but on a made-up sample that proves nothing about the future.

## What the research actually found

| Source                                                | What it measured                                                 | Result                                                                                                                                                                                                                     |
| ----------------------------------------------------- | ---------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Quantpedia, summarising the commodity momentum papers | Twelve-month ranking, monthly rebalancing, 1979 to 2004          | 14.6 percent a year for the best-studied version, volatility 25.6 percent, worst fall 79.75 percent, reward-to-risk 0.57                                                                                                   |
| Miffre and Rallis, the source paper                   | Thirteen ranking rules on a basket of commodity futures          | 9.38 percent average return a year; the winners tended to be backwardated and volatile, the losers contangoed                                                                                                              |
| Geczy and Samonov                                     | Hand-collected commodity futures going back to 1877              | Momentum, value and the storage-based basis all stayed positive in the extra eighty years before the usual sample, and a basket of the three roughly doubled its reward-to-risk ratio against holding commodities outright |
| Urquhart and Zhang                                    | Twenty-nine commodity futures, 1979 to 2017                      | Momentum was statistically and economically significant, but the strongest version's edge declined sharply after 1998                                                                                                      |
| Zaremba                                               | Commodity markets split by how much outside investors trade them | Momentum and storage-based strategies earned less in the markets where outside investors were most active, which the paper links to the arrival of index money                                                             |

Read together, the pattern is a real, replicated tendency that has become harder to collect. The
effect is not a single study: Miffre and Rallis found it in one sample, Quantpedia added a second,
and the hand-collected history extends the evidence back more than a century. That is why the
direction is not seriously disputed. What is disputed is the size. The long-short portfolio's worst
fall of nearly 80 percent means an account that ran it without a break could be almost erased even
while the average over the whole sample was positive. The later papers point to outside investment
money as one reason the edge has thinned, and the QuantConnect version narrows the list to dairy,
meat and forestry contracts, which are less crowded but also less liquid.

Note also what the source paper does not claim. It reports that the winners were backwardated, which
is a description of what the strategy happened to hold, not a proof that the pattern will persist.
That the momentum profit is compensation for risk, rather than a lasting mispricing, is an
assumption the papers debate rather than settle.

## How this project relates to it

This repository's own brief on the commodity evidence is
[Energy and commodity markets](../../../strategies/books2/18_energy_and_commodities.md). It reports
that the level, slope and curvature of a commodity futures curve explain almost all of the curve's
movement, that the nearest four contracts hold most of the liquidity, and, on the paper `2308.00383v1`,
that the net edge from trading those curves is small and has weakened over time. That is the same
caution the momentum sources reach: the structure is real and the tradable part shrinks with time.

The momentum design question is covered in
[Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
which records that a momentum rule's returns can be positively skewed even when the underlying
returns are not, so a strategy may win less than half its months and still be worth studying. The
repository's own study of ranking systems is
[Profiting from sector rotation](../../../strategies/sector_rotation_strategies.md), which tested
much the same ranking idea on industries and found that most of the apparent outperformance came
from trying many rules rather than from a durable edge. The one implemented tool,
[the sector regime engine](../../../implementation/sector-regime-engine/README.md), measures whether
a market is in a state where a ranking rule is worth switching on at all.

## Where it goes wrong

- Momentum crashes. After a broad fall in prices, the materials that fell furthest are often the ones
  that led the previous ranking, and they bounce hardest. The published worst fall of nearly 80
  percent comes from exactly this.
- The two sides can both lose. The portfolio is not protected against a rise in everything or a fall
  in everything; it is protected only against the strong group underperforming the weak one, and the
  worked example shows a month where both sides rose and the strategy still lost.
- Crowding and outside money. Once the rule is known and easy to trade through index products, more
  money runs it, the buying happens earlier, and the return each participant collects shrinks.
- The shape of the futures matters. The price series used is a stitched-together chain of contracts,
  and how that stitching is done changes the measured return. Two data providers can disagree on the
  same raw material.
- Costs on a monthly rebuild. A rule that changes its mind often pays the gap between buying and
  selling prices every time. On the smaller contracts in the QuantConnect list those gaps are wider
  than on oil or copper.
- The number of rules tried. There are many ways to define the ranking: how many months, how many
  groups, how often to rebuild, whether to demand a trend filter. Choosing the best of these after
  seeing the results is how an apparent edge can be manufactured.

## Try it yourself

You need a spreadsheet and a public source of monthly commodity prices, such as the ones on any large
market-data website.

1. Build a sheet with one column per material and one row per month for the last five years.
2. Add a column that computes the twelve-month return: today's price divided by the price twelve rows
   up, minus one.
3. For each month, add a column naming the two materials with the highest value in that row and the
   two with the lowest. These are what the rules would have bought and sold.
4. In the next row, average the next month's returns of the high pair and of the low pair, take half
   the difference, and that is the strategy's return for the month.
5. Build a second column that is the average return of all the materials, which is what holding the
   whole basket would have done.
6. Subtract a cost of 0.10 percent from any month in which the names changed.

What to notice: in many months the high pair is almost the same as the previous month, so the cost is
near zero, while in others every name changes and the cost bites. Over five years the two columns
will often be close, with the strategy ahead in trending periods and behind in sharp reversals. If it
wins by a wide margin, the likely cause is that you picked the materials after seeing which ones
worked.

## Where this came from

- [QuantConnect strategy library: momentum effect in commodities futures](https://www.quantconnect.com/tutorials/strategy-library/momentum-effect-in-commodities-futures),
  the rules as implemented: twelve-month rate of change, ranking into fifths, the highest fifth bought
  and the lowest fifth sold, monthly rebalance.
- [Quantpedia: momentum effect in commodities](https://quantpedia.com/strategies/momentum-effect-in-commodities),
  the performance figures, the instrument count, the sample period 1979 to 2004 and the bundle of
  underlying papers.
- Miffre and Rallis, [Momentum in Commodity Futures Markets](http://papers.ssrn.com/sol3/papers.cfm?abstract_id=702281),
  the source paper, which found thirteen profitable ranking rules and that the winners were
  backwardated.
- [Energy and commodity markets](../../../strategies/books2/18_energy_and_commodities.md), this
  repository's brief on the commodity-curve evidence and the finding from `2308.00383v1` that the
  tradable curve edge has weakened.
- [Return predictability and trading strategies](../../../strategies/books2/08_predictability_and_trading_strategies.md),
  which covers the design and skewness of momentum rules, citing `2101.01006v2`.

## Words used in this tutorial

- backwardation: a market where a material costs more to buy now than to buy for future delivery,
  usually because it is scarce today.
- contango: the opposite, where the future price is above the current price, usually because storage
  costs money.
- futures: an agreement to buy or sell something at a fixed price on a fixed future date.
- going short: selling something you do not own, so that you gain when its price falls.
- momentum: the tendency of something that has been rising to keep rising for a while.
- quintile: a group formed by dividing a ranked list into five equal parts.
- rate of change: the percentage change in a price between two dates.
- weight: the share of your money placed in one holding.
- See [GLOSSARY.md](../../GLOSSARY.md) for the rest.
